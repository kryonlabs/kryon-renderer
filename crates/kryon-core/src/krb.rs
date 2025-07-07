// crates/kryon-core/src/krb.rs
use crate::{Element, ElementId, ElementType, PropertyValue, Result, KryonError, TextAlignment, Style, CursorType, InteractionState, EventType}; 
use std::collections::HashMap;
use glam::{Vec2, Vec4};

#[derive(Debug)]
pub struct KRBFile {
    pub header: KRBHeader,
    pub strings: Vec<String>,
    pub elements: HashMap<u32, Element>,
    pub styles: HashMap<u8, Style>, 
    pub root_element_id: Option<u32>,
    pub resources: Vec<String>,
    pub scripts: Vec<ScriptEntry>,
}

#[derive(Debug)]
pub struct KRBHeader {
    pub magic: [u8; 4],
    pub version: u16,
    pub flags: u16,
    pub element_count: u16,
    pub style_count: u16,
    pub component_count: u16,
    pub script_count: u16,
    pub string_count: u16,
    pub resource_count: u16,
}

#[derive(Debug, Clone)]
pub struct ScriptEntry {
    pub language: String,
    pub name: String,
    pub code: String,
    pub entry_points: Vec<String>,
}

pub struct KRBParser {
    data: Vec<u8>,
    position: usize,
}

impl KRBParser {
    pub fn new(data: Vec<u8>) -> Self {
        Self { data, position: 0 }
    }
    
    pub fn parse(&mut self) -> Result<KRBFile> {
        let header = self.parse_header()?;
        
        if &header.magic != b"KRB1" {
            return Err(KryonError::InvalidKRB("Invalid magic number".to_string()));
        }
        
        if header.version > 0x0500 {
            return Err(KryonError::UnsupportedVersion(header.version));
        }
        
        let strings = self.parse_string_table(&header)?;
        let styles = self.parse_style_table(&header, &strings)?;
        let mut elements = self.parse_element_tree(&header, &strings)?;
        let resources = self.parse_resource_table(&header)?;
        let scripts = self.parse_script_table(&header, &strings)?;
        
        // Apply style-based layout flags to elements
        self.apply_style_layout_flags(&mut elements, &styles)?;
        
        // Find root element (App type) or create default App wrapper
        let root_element_id = if let Some((id, _)) = elements.iter()
            .find(|(_, element)| element.element_type == ElementType::App) {
            Some(*id)
        } else {
            // No App element found, create a default App wrapper
            Self::create_default_app_wrapper(&mut elements)
        };
        
        Ok(KRBFile {
            header,
            strings,
            elements,
            styles,
            root_element_id,
            resources,
            scripts,
        })
    }
    
    fn parse_style_table(&mut self, header: &KRBHeader, strings: &[String]) -> Result<HashMap<u8, Style>> {
        let style_offset = self.read_u32_at(26) as usize;
        let mut styles = HashMap::new();
        
        eprintln!("[STYLE] Parsing {} styles from offset 0x{:X}", header.style_count, style_offset);
        
        self.position = style_offset;
        
        for i in 0..header.style_count {
            let style_id = self.read_u8(); // Read the actual style ID from file
            let name_index = self.read_u8() as usize;
            let property_count = self.read_u8();

            let name = if name_index < strings.len() {
                strings[name_index].clone()
            } else {
                format!("style_{}", style_id)
            };

            eprintln!("[STYLE] Style {}: name='{}', name_index={}, props={}", style_id, name, name_index, property_count);

            let mut properties = HashMap::new();
            for j in 0..property_count {
                let prop_id = self.read_u8();
                let _value_type = self.read_u8(); // We can use this for more robust parsing later
                let size = self.read_u8();
                
                eprintln!("[STYLE]   Property {}: id=0x{:02X}, size={}", j, prop_id, size);
                
                let value = match prop_id {
                    0x01 | 0x02 | 0x03 => PropertyValue::Color(self.read_color()),
                    0x04 | 0x05 => PropertyValue::Float(self.read_u8() as f32), // BorderWidth, BorderRadius
                    0x06 => {
                        if size == 1 {
                            PropertyValue::Int(self.read_u8() as i32) // Layout flags
                        } else {
                            // Skip if wrong size
                            for _ in 0..size { self.read_u8(); }
                            continue;
                        }
                    }
                    0x19 => {
                        if size == 2 {
                            PropertyValue::Float(self.read_u16() as f32) // Width
                        } else {
                            // Property 0x19 with size != 2 is unexpected, skip it
                            for _ in 0..size { self.read_u8(); }
                            continue;
                        }
                    }
                    0x1A => {
                        if size == 1 {
                            PropertyValue::Int(self.read_u8() as i32) // Layout flags
                        } else {
                            // Skip if wrong size
                            for _ in 0..size { self.read_u8(); }
                            continue;
                        }
                    }
                    0x1B => {
                        if size == 2 {
                            PropertyValue::Float(self.read_u16() as f32) // Height
                        } else {
                            // Property 0x1B with size != 2 is unexpected, skip it
                            for _ in 0..size { self.read_u8(); }
                            continue;
                        }
                    }
                    0x0B => {
                        if size == 1 {
                            PropertyValue::Int(self.read_u8() as i32) // TextAlignment
                        } else {
                            // Skip if wrong size
                            for _ in 0..size { self.read_u8(); }
                            continue;
                        }
                    }
                    0x1C => {
                        // Custom data - just skip it for now since it's string data
                        for _ in 0..size { self.read_u8(); }
                        continue;
                    }
                    // Modern Taffy layout properties (0x40-0x4F range)
                    0x40 => { // Display
                        if size == 1 {
                            let string_index = self.read_u8() as usize;
                            if string_index < strings.len() {
                                PropertyValue::String(strings[string_index].clone())
                            } else {
                                eprintln!("[STYLE]     Display: invalid string index {}", string_index);
                                continue;
                            }
                        } else {
                            for _ in 0..size { self.read_u8(); }
                            continue;
                        }
                    }
                    0x41 => { // FlexDirection
                        if size == 1 {
                            let string_index = self.read_u8() as usize;
                            if string_index < strings.len() {
                                PropertyValue::String(strings[string_index].clone())
                            } else {
                                eprintln!("[STYLE]     FlexDirection: invalid string index {}", string_index);
                                continue;
                            }
                        } else {
                            for _ in 0..size { self.read_u8(); }
                            continue;
                        }
                    }
                    0x42 => { // FlexWrap
                        if size == 1 {
                            let string_index = self.read_u8() as usize;
                            if string_index < strings.len() {
                                PropertyValue::String(strings[string_index].clone())
                            } else {
                                eprintln!("[STYLE]     FlexWrap: invalid string index {}", string_index);
                                continue;
                            }
                        } else {
                            for _ in 0..size { self.read_u8(); }
                            continue;
                        }
                    }
                    0x43 => { // FlexGrow
                        if size == 4 {
                            let flex_grow_bytes = [self.read_u8(), self.read_u8(), self.read_u8(), self.read_u8()];
                            PropertyValue::Float(f32::from_le_bytes(flex_grow_bytes))
                        } else {
                            for _ in 0..size { self.read_u8(); }
                            continue;
                        }
                    }
                    0x44 => { // FlexShrink
                        if size == 4 {
                            let flex_shrink_bytes = [self.read_u8(), self.read_u8(), self.read_u8(), self.read_u8()];
                            PropertyValue::Float(f32::from_le_bytes(flex_shrink_bytes))
                        } else {
                            for _ in 0..size { self.read_u8(); }
                            continue;
                        }
                    }
                    0x45 => { // FlexBasis
                        if size == 1 {
                            let string_index = self.read_u8() as usize;
                            if string_index < strings.len() {
                                PropertyValue::String(strings[string_index].clone())
                            } else {
                                eprintln!("[STYLE]     FlexBasis: invalid string index {}", string_index);
                                continue;
                            }
                        } else {
                            for _ in 0..size { self.read_u8(); }
                            continue;
                        }
                    }
                    0x46 => { // AlignItems
                        if size == 1 {
                            let string_index = self.read_u8() as usize;
                            if string_index < strings.len() {
                                PropertyValue::String(strings[string_index].clone())
                            } else {
                                eprintln!("[STYLE]     AlignItems: invalid string index {}", string_index);
                                continue;
                            }
                        } else {
                            for _ in 0..size { self.read_u8(); }
                            continue;
                        }
                    }
                    0x47 => { // AlignSelf
                        if size == 1 {
                            let string_index = self.read_u8() as usize;
                            if string_index < strings.len() {
                                PropertyValue::String(strings[string_index].clone())
                            } else {
                                eprintln!("[STYLE]     AlignSelf: invalid string index {}", string_index);
                                continue;
                            }
                        } else {
                            for _ in 0..size { self.read_u8(); }
                            continue;
                        }
                    }
                    0x48 => { // AlignContent
                        if size == 1 {
                            let string_index = self.read_u8() as usize;
                            if string_index < strings.len() {
                                PropertyValue::String(strings[string_index].clone())
                            } else {
                                eprintln!("[STYLE]     AlignContent: invalid string index {}", string_index);
                                continue;
                            }
                        } else {
                            for _ in 0..size { self.read_u8(); }
                            continue;
                        }
                    }
                    0x49 => { // JustifyContent
                        if size == 1 {
                            let string_index = self.read_u8() as usize;
                            if string_index < strings.len() {
                                PropertyValue::String(strings[string_index].clone())
                            } else {
                                eprintln!("[STYLE]     JustifyContent: invalid string index {}", string_index);
                                continue;
                            }
                        } else {
                            for _ in 0..size { self.read_u8(); }
                            continue;
                        }
                    }
                    0x4A => { // JustifyItems
                        if size == 1 {
                            let string_index = self.read_u8() as usize;
                            if string_index < strings.len() {
                                PropertyValue::String(strings[string_index].clone())
                            } else {
                                eprintln!("[STYLE]     JustifyItems: invalid string index {}", string_index);
                                continue;
                            }
                        } else {
                            for _ in 0..size { self.read_u8(); }
                            continue;
                        }
                    }
                    0x4B => { // JustifySelf
                        if size == 1 {
                            let string_index = self.read_u8() as usize;
                            if string_index < strings.len() {
                                PropertyValue::String(strings[string_index].clone())
                            } else {
                                eprintln!("[STYLE]     JustifySelf: invalid string index {}", string_index);
                                continue;
                            }
                        } else {
                            for _ in 0..size { self.read_u8(); }
                            continue;
                        }
                    }
                    0x50 => { // Position
                        if size == 1 {
                            let string_index = self.read_u8() as usize;
                            if string_index < strings.len() {
                                PropertyValue::String(strings[string_index].clone())
                            } else {
                                eprintln!("[STYLE]     Position: invalid string index {}", string_index);
                                continue;
                            }
                        } else {
                            for _ in 0..size { self.read_u8(); }
                            continue;
                        }
                    }
                    // Add other property types here
                    _ => {
                        // For unknown properties, read the raw bytes and display them
                        eprintln!("[STYLE]     Unknown property 0x{:02X}, size={}, reading raw bytes...", prop_id, size);
                        let mut raw_bytes = Vec::new();
                        for i in 0..size {
                            let byte = self.read_u8();
                            raw_bytes.push(byte);
                            eprintln!("[STYLE]       Byte {}: 0x{:02X} ({})", i, byte, byte);
                        }
                        
                        // Try to interpret as different types
                        if size == 1 {
                            eprintln!("[STYLE]     Could be layout flags: 0x{:02X}", raw_bytes[0]);
                        } else if size == 4 {
                            let color = Vec4::new(
                                raw_bytes[0] as f32 / 255.0,
                                raw_bytes[1] as f32 / 255.0,
                                raw_bytes[2] as f32 / 255.0,
                                raw_bytes[3] as f32 / 255.0
                            );
                            eprintln!("[STYLE]     Could be color: {:?}", color);
                        }
                        continue;
                    }
                };
                properties.insert(prop_id, value);
            }
            
            eprintln!("[STYLE] Loaded style {}: '{}' with {} properties", style_id, name, properties.len());
            // Ensure we don't overwrite existing styles with the same ID
            if !styles.contains_key(&style_id) {
                styles.insert(style_id, Style { name, properties });
            } else {
                eprintln!("[STYLE] WARNING: Duplicate style ID {} - skipping", style_id);
            }
        }

        eprintln!("[STYLE] Parsed {} styles total", styles.len());
        Ok(styles)
    }

    fn parse_header(&mut self) -> Result<KRBHeader> {
        if self.data.len() < 54 {
            return Err(KryonError::InvalidKRB("File too small".to_string()));
        }
        
        let mut magic = [0u8; 4];
        magic.copy_from_slice(&self.data[0..4]);
        
        Ok(KRBHeader {
            magic,
            version: self.read_u16_at(4),
            flags: self.read_u16_at(6),
            element_count: self.read_u16_at(8),
            style_count: self.read_u16_at(10),
            component_count: self.read_u16_at(12),
            script_count: self.read_u16_at(16),
            string_count: self.read_u16_at(18),
            resource_count: self.read_u16_at(20),
        })
    }
    
    fn parse_string_table(&mut self, header: &KRBHeader) -> Result<Vec<String>> {
        let string_offset = self.read_u32_at(42) as usize;
        let mut strings = Vec::new();
        
        self.position = string_offset;
        
        for _ in 0..header.string_count {
            let length = self.read_u8() as usize;
            let string_data = &self.data[self.position..self.position + length];
            let string = String::from_utf8_lossy(string_data).to_string();
            strings.push(string);
            self.position += length;
        }
        
        Ok(strings)
    }
    
    fn parse_element_tree(&mut self, header: &KRBHeader, strings: &[String]) -> Result<HashMap<u32, Element>> {
        let element_offset = self.read_u32_at(22) as usize;
        let mut elements = HashMap::new();
        
        self.position = element_offset;
        
        for element_id in 0..header.element_count {
            let element = self.parse_element(element_id as u32, strings)?;
            elements.insert(element_id as u32, element);
        }
        
        // Build parent-child relationships from tree structure
        self.build_element_hierarchy(&mut elements, header.element_count as u32);
        
        Ok(elements)
    }
    fn parse_element(&mut self, element_id: u32, strings: &[String]) -> Result<Element> {
        let mut element = Element::default();
        
        // Parse element header (19 bytes)
        let element_type = ElementType::from(self.read_u8());
        let id_index = self.read_u8();
        let pos_x = self.read_u16() as f32;
        let pos_y = self.read_u16() as f32;
        let width = self.read_u16() as f32;
        let height = self.read_u16() as f32;
        let layout_flags = self.read_u8();
        let style_id = self.read_u8();
        let checked = self.read_u8() != 0;
        let property_count = self.read_u8();
        let child_count = self.read_u8();
        let _event_count = self.read_u8();
        let _animation_count = self.read_u8();
        let custom_prop_count = self.read_u8();
        let _state_prop_count = self.read_u8();
        
        element.element_type = element_type;
        
        // Set default cursor type for interactive elements
        if element_type == ElementType::Button {
            element.cursor = CursorType::Pointer;
            eprintln!("[PARSE] Auto-set cursor to Pointer for Button element");
        }
        
        element.id = if id_index > 0 && (id_index as usize) < strings.len() {
            strings[id_index as usize].clone()
        } else {
            format!("element_{}", element_id)
        };
        
        element.style_id = style_id; 
        element.position = Vec2::new(pos_x, pos_y);
        element.size = Vec2::new(width, height);
        element.layout_flags = layout_flags;
        
        // Set initial interaction state based on checked field
        element.current_state = if checked {
            InteractionState::Checked
        } else {
            InteractionState::Normal
        };
        
        // Store original layout_flags for later style merging
        let _original_layout_flags = layout_flags;
        
        eprintln!("[PARSE] Element {}: type={:?}, style_id={}, layout_flags={:08b}, props={}, children={}, custom_props={}", 
            element_id, element_type, style_id, layout_flags, property_count, child_count, custom_prop_count);
        
        // Parse standard properties
        for i in 0..property_count {
            eprintln!("[PARSE] Parsing standard property {} for element {}", i, element_id);
            self.parse_standard_property(&mut element, strings)?;
        }
        
        // Parse custom properties  
        for i in 0..custom_prop_count {
            eprintln!("[PARSE] Parsing custom property {} for element {}", i, element_id);
            self.parse_custom_property(&mut element, strings)?;
        }
        
        // Skip state property sets (TODO: implement properly)
        for _ in 0.._state_prop_count {
            let state_flags = self.read_u8();
            let state_property_count = self.read_u8();
            for _ in 0..state_property_count {
                // Skip state properties (same format as standard properties)
                let _property_id = self.read_u8();
                let _value_type = self.read_u8();
                let size = self.read_u8();
                for _ in 0..size {
                    self.read_u8();
                }
            }
        }
        
        // Parse events
        for _ in 0.._event_count {
            let event_type_id = self.read_u8();
            let callback_string_index = self.read_u8() as usize;
            
            if let Some(event_type) = self.event_type_from_id(event_type_id) {
                if callback_string_index < strings.len() {
                    let callback_name = strings[callback_string_index].clone();
                    element.event_handlers.insert(event_type, callback_name);
                    eprintln!("[EVENT] Added {} event handler: {}", self.event_type_name(event_type), strings[callback_string_index]);
                }
            }
        }
        
        // Skip child element offsets (TODO: implement properly)
        for _ in 0..child_count {
            self.read_u16(); // child offset
        }
        
        // Initialize children vector based on child_count
        element.children = Vec::with_capacity(child_count as usize);
        
        Ok(element)
    }
    
    fn build_element_hierarchy(&self, elements: &mut HashMap<u32, Element>, element_count: u32) {
        // Build parent-child relationships based on tree structure
        // Elements are written in depth-first traversal order
        
        let mut parent_stack = vec![0u32]; // Start with root element
        let mut remaining_children: Vec<u32> = Vec::new();
        
        // Initialize remaining children counts
        for i in 0..element_count {
            if let Some(element) = elements.get(&i) {
                remaining_children.push(element.children.capacity() as u32);
            } else {
                remaining_children.push(0);
            }
        }
        
        // Process elements in order, assigning parents
        for element_id in 1..element_count { // Skip root element (0)
            // Find the current parent (top of stack with remaining children)
            while let Some(&current_parent) = parent_stack.last() {
                if remaining_children[current_parent as usize] > 0 {
                    // This element is a child of current_parent
                    if let Some(parent) = elements.get_mut(&current_parent) {
                        parent.children.push(element_id);
                        eprintln!("[KRB] Element {}: added child {}", current_parent, element_id);
                    }
                    
                    if let Some(child) = elements.get_mut(&element_id) {
                        child.parent = Some(current_parent);
                        eprintln!("[KRB] Element {}: set parent [{}]", element_id, current_parent);
                    }
                    
                    remaining_children[current_parent as usize] -= 1;
                    
                    // If this element has children, add it to the stack
                    if remaining_children[element_id as usize] > 0 {
                        parent_stack.push(element_id);
                    }
                    break;
                } else {
                    // Current parent has no more children, pop it
                    parent_stack.pop();
                }
            }
        }
    }
    
    fn parse_standard_property(&mut self, element: &mut Element, strings: &[String]) -> Result<()> {
        let property_id = self.read_u8();
        let value_type = self.read_u8();
        let size = self.read_u8();
        
        eprintln!("[PROP] Property ID: 0x{:02X}, value_type: 0x{:02X}, size: {}", property_id, value_type, size);
        
        match property_id {
            0x01 => { // BackgroundColor
                element.background_color = self.read_color();
                eprintln!("[PROP] BackgroundColor: {:?}", element.background_color);
            }
            0x02 => { // ForegroundColor/TextColor
                element.text_color = self.read_color();
                eprintln!("[PROP] TextColor: {:?}", element.text_color);
            }
            0x03 => { // BorderColor
                element.border_color = self.read_color();
                eprintln!("[PROP] BorderColor: {:?}", element.border_color);
            }
            0x04 => { // BorderWidth
                element.border_width = self.read_u8() as f32;
                eprintln!("[PROP] BorderWidth: {}", element.border_width);
            }
            0x05 => { // BorderRadius
                element.border_radius = self.read_u8() as f32;
                eprintln!("[PROP] BorderRadius: {}", element.border_radius);
            }
            0x06 => { // Layout flags
                let layout_value = self.read_u8();
                element.layout_flags = layout_value;
                eprintln!("[PROP] Layout: flags=0x{:02X} (binary: {:08b})", layout_value, layout_value);
            }
            0x08 => { // TextContent
                let string_index = self.read_u8() as usize;
                if string_index < strings.len() {
                    element.text = strings[string_index].clone();
                    eprintln!("[PROP] TextContent: '{}'", element.text);
                }
            }
            0x09 => { // FontSize
                element.font_size = self.read_u16() as f32;
                eprintln!("[PROP] FontSize: {}", element.font_size);
            }
            0x0A => { // FontWeight
                let weight = self.read_u16();
                element.font_weight = match weight {
                    300 => crate::elements::FontWeight::Light,
                    400 => crate::elements::FontWeight::Normal,
                    700 => crate::elements::FontWeight::Bold,
                    900 => crate::elements::FontWeight::Heavy,
                    _ => crate::elements::FontWeight::Normal,
                };
                eprintln!("[PROP] FontWeight: {}", weight);
            }
            0x0D => { // Opacity
                element.opacity = self.read_u16() as f32 / 256.0; // 8.8 fixed point
                eprintln!("[PROP] Opacity: {}", element.opacity);
            }
            0x0E => { // ZIndex
                let z_index = self.read_u16() as i32;
                element.custom_properties.insert("z_index".to_string(), PropertyValue::Int(z_index));
                eprintln!("[PROP] ZIndex: {}", z_index);
            }
            0x0B => { // TextAlignment
                let alignment = self.read_u8();
                eprintln!("[PROP] TextAlignment: {}", alignment);
                // Apply text alignment to element
                element.text_alignment = match alignment {
                    0 => TextAlignment::Start,
                    1 => TextAlignment::Center,
                    2 => TextAlignment::End,
                    3 => TextAlignment::Justify,
                    _ => TextAlignment::Start,
                };
            }
            0x0C => { // ImageSource
                let string_index = self.read_u8() as usize;
                if string_index < strings.len() {
                    let image_src = strings[string_index].clone();
                    element.custom_properties.insert("src".to_string(), PropertyValue::String(image_src.clone()));
                    eprintln!("[PROP] ImageSource: '{}'", image_src);
                }
            }
            0x0F => { // Visibility
                element.visible = self.read_u8() != 0;
                eprintln!("[PROP] Visibility: {}", element.visible);
            }
            0x10 => { // Gap
                let gap = self.read_u8() as f32;
                element.custom_properties.insert("gap".to_string(), PropertyValue::Float(gap));
                eprintln!("[PROP] Gap: {}", gap);
            }
            0x11 => { // MinWidth
                let min_width = self.read_u16() as f32;
                element.custom_properties.insert("min_width".to_string(), PropertyValue::Float(min_width));
                eprintln!("[PROP] MinWidth: {}", min_width);
            }
            0x12 => { // MinHeight
                let min_height = self.read_u16() as f32;
                element.custom_properties.insert("min_height".to_string(), PropertyValue::Float(min_height));
                eprintln!("[PROP] MinHeight: {}", min_height);
            }
            0x13 => { // MaxWidth
                let max_width = self.read_u16() as f32;
                element.custom_properties.insert("max_width".to_string(), PropertyValue::Float(max_width));
                eprintln!("[PROP] MaxWidth: {}", max_width);
            }
            0x14 => { // MaxHeight
                let max_height = self.read_u16() as f32;
                element.custom_properties.insert("max_height".to_string(), PropertyValue::Float(max_height));
                eprintln!("[PROP] MaxHeight: {}", max_height);
            }
            // App-specific properties
            0x20 => { // WindowWidth
                let width = self.read_u16();
                eprintln!("[PROP] WindowWidth: {}", width);
                // App elements use this for initial size
                if element.element_type == ElementType::App {
                    element.size.x = width as f32;
                }
            }
            0x21 => { // WindowHeight  
                let height = self.read_u16();
                eprintln!("[PROP] WindowHeight: {}", height);
                if element.element_type == ElementType::App {
                    element.size.y = height as f32;
                }
            }
            0x22 => { // WindowTitle
                let string_index = self.read_u8() as usize;
                if string_index < strings.len() {
                    eprintln!("[PROP] WindowTitle: '{}'", strings[string_index]);
                    // Could store in custom properties if needed
                }
            }
            0x23 => { // Resizable
                let resizable = self.read_u8() != 0;
                eprintln!("[PROP] Resizable: {}", resizable);
            }
            0x24 => { // KeepAspectRatio
                let keep_aspect = self.read_u8() != 0;
                eprintln!("[PROP] KeepAspectRatio: {}", keep_aspect);
            }
            0x25 => { // ScaleFactor
                let scale = self.read_u16() as f32 / 256.0; // 8.8 fixed point
                eprintln!("[PROP] ScaleFactor: {}", scale);
            }
            0x26 => { // Icon
                let string_index = self.read_u8() as usize;
                if string_index < strings.len() {
                    eprintln!("[PROP] Icon: '{}'", strings[string_index]);
                }
            }
            0x27 => { // Version
                let string_index = self.read_u8() as usize;
                if string_index < strings.len() {
                    eprintln!("[PROP] Version: '{}'", strings[string_index]);
                }
            }
            0x28 => { // Author
                let string_index = self.read_u8() as usize;
                if string_index < strings.len() {
                    eprintln!("[PROP] Author: '{}'", strings[string_index]);
                }
            }
            0x29 => { // Cursor
                let cursor_value = self.read_u8();
                element.cursor = match cursor_value {
                    0 => CursorType::Default,
                    1 => CursorType::Pointer,
                    2 => CursorType::Text,
                    3 => CursorType::Move,
                    4 => CursorType::NotAllowed,
                    _ => CursorType::Default,
                };
                eprintln!("[PROP] Cursor: {} ({})", cursor_value, match element.cursor {
                    CursorType::Default => "Default",
                    CursorType::Pointer => "Pointer",
                    CursorType::Text => "Text",
                    CursorType::Move => "Move",
                    CursorType::NotAllowed => "NotAllowed",
                });
            }
            0x1A => { // LayoutFlags (layout property)
                let layout_value = self.read_u8();
                element.layout_flags = layout_value;
                eprintln!("[PROP] LayoutFlags: flags=0x{:02X} (binary: {:08b})", layout_value, layout_value);
            }
            0x19 => { // Width
                let width = self.read_u16() as f32;
                element.size.x = width;
                eprintln!("[PROP] Width: {}", width);
            }
            0x1B => { // Height
                let height = self.read_u16() as f32;
                element.size.y = height;
                eprintln!("[PROP] Height: {}", height);
            }
            // Modern Taffy layout properties (0x40-0x4F range)
            0x40 => { // Display
                let string_index = self.read_u8() as usize;
                if string_index < strings.len() {
                    let display_value = strings[string_index].clone();
                    element.custom_properties.insert("display".to_string(), PropertyValue::String(display_value.clone()));
                    eprintln!("[PROP] Display: '{}'", display_value);
                }
            }
            0x41 => { // FlexDirection
                let string_index = self.read_u8() as usize;
                if string_index < strings.len() {
                    let flex_direction = strings[string_index].clone();
                    element.custom_properties.insert("flex_direction".to_string(), PropertyValue::String(flex_direction.clone()));
                    eprintln!("[PROP] FlexDirection: '{}'", flex_direction);
                }
            }
            0x42 => { // FlexWrap
                let string_index = self.read_u8() as usize;
                if string_index < strings.len() {
                    let flex_wrap = strings[string_index].clone();
                    element.custom_properties.insert("flex_wrap".to_string(), PropertyValue::String(flex_wrap.clone()));
                    eprintln!("[PROP] FlexWrap: '{}'", flex_wrap);
                }
            }
            0x43 => { // FlexGrow
                let flex_grow_bytes = [self.read_u8(), self.read_u8(), self.read_u8(), self.read_u8()];
                let flex_grow = f32::from_le_bytes(flex_grow_bytes);
                element.custom_properties.insert("flex_grow".to_string(), PropertyValue::Float(flex_grow));
                eprintln!("[PROP] FlexGrow: {}", flex_grow);
            }
            0x44 => { // FlexShrink
                let flex_shrink_bytes = [self.read_u8(), self.read_u8(), self.read_u8(), self.read_u8()];
                let flex_shrink = f32::from_le_bytes(flex_shrink_bytes);
                element.custom_properties.insert("flex_shrink".to_string(), PropertyValue::Float(flex_shrink));
                eprintln!("[PROP] FlexShrink: {}", flex_shrink);
            }
            0x45 => { // FlexBasis
                let string_index = self.read_u8() as usize;
                if string_index < strings.len() {
                    let flex_basis = strings[string_index].clone();
                    element.custom_properties.insert("flex_basis".to_string(), PropertyValue::String(flex_basis.clone()));
                    eprintln!("[PROP] FlexBasis: '{}'", flex_basis);
                }
            }
            0x46 => { // AlignItems
                let string_index = self.read_u8() as usize;
                if string_index < strings.len() {
                    let align_items = strings[string_index].clone();
                    element.custom_properties.insert("align_items".to_string(), PropertyValue::String(align_items.clone()));
                    eprintln!("[PROP] AlignItems: '{}'", align_items);
                }
            }
            0x47 => { // AlignSelf
                let string_index = self.read_u8() as usize;
                if string_index < strings.len() {
                    let align_self = strings[string_index].clone();
                    element.custom_properties.insert("align_self".to_string(), PropertyValue::String(align_self.clone()));
                    eprintln!("[PROP] AlignSelf: '{}'", align_self);
                }
            }
            0x48 => { // AlignContent
                let string_index = self.read_u8() as usize;
                if string_index < strings.len() {
                    let align_content = strings[string_index].clone();
                    element.custom_properties.insert("align_content".to_string(), PropertyValue::String(align_content.clone()));
                    eprintln!("[PROP] AlignContent: '{}'", align_content);
                }
            }
            0x49 => { // JustifyContent
                let string_index = self.read_u8() as usize;
                if string_index < strings.len() {
                    let justify_content = strings[string_index].clone();
                    element.custom_properties.insert("justify_content".to_string(), PropertyValue::String(justify_content.clone()));
                    eprintln!("[PROP] JustifyContent: '{}'", justify_content);
                }
            }
            0x4A => { // JustifyItems
                let string_index = self.read_u8() as usize;
                if string_index < strings.len() {
                    let justify_items = strings[string_index].clone();
                    element.custom_properties.insert("justify_items".to_string(), PropertyValue::String(justify_items.clone()));
                    eprintln!("[PROP] JustifyItems: '{}'", justify_items);
                }
            }
            0x4B => { // JustifySelf
                let string_index = self.read_u8() as usize;
                if string_index < strings.len() {
                    let justify_self = strings[string_index].clone();
                    element.custom_properties.insert("justify_self".to_string(), PropertyValue::String(justify_self.clone()));
                    eprintln!("[PROP] JustifySelf: '{}'", justify_self);
                }
            }
            0x50 => { // Position
                let string_index = self.read_u8() as usize;
                if string_index < strings.len() {
                    let position = strings[string_index].clone();
                    element.custom_properties.insert("position".to_string(), PropertyValue::String(position.clone()));
                    eprintln!("[PROP] Position: '{}'", position);
                }
            }
            _ => {
                eprintln!("[PROP] Unknown property 0x{:02X}, skipping {} bytes...", property_id, size);
                // Skip unknown property using size field
                for _ in 0..size {
                    self.read_u8();
                }
            }
        }
        
        Ok(())
    }
    
    fn parse_custom_property(&mut self, element: &mut Element, strings: &[String]) -> Result<()> {
        let key_index = self.read_u8() as usize;
        let value_type = self.read_u8();
        let size = self.read_u8();
        
        let key = if key_index < strings.len() {
            strings[key_index].clone()
        } else {
            return Ok(()); // Skip invalid key
        };
        
        let value = match value_type {
            0x01 => PropertyValue::Int(self.read_u8() as i32),
            0x02 => PropertyValue::Int(self.read_u16() as i32),
            0x03 => PropertyValue::Color(self.read_color()),
            0x04 => {
                let string_index = self.read_u8() as usize;
                if string_index < strings.len() {
                    PropertyValue::String(strings[string_index].clone())
                } else {
                    PropertyValue::String(String::new())
                }
            }
            _ => PropertyValue::String(String::new()),
        };
        
        element.custom_properties.insert(key, value);
        Ok(())
    }
    
    fn parse_resource_table(&mut self, header: &KRBHeader) -> Result<Vec<String>> {
        let resource_offset = self.read_u32_at(46) as usize;
        let mut resources = Vec::new();
        
        self.position = resource_offset;
        
        for _ in 0..header.resource_count {
            let length = self.read_u8() as usize;
            let resource_data = &self.data[self.position..self.position + length];
            let resource = String::from_utf8_lossy(resource_data).to_string();
            resources.push(resource);
            self.position += length;
        }
        
        Ok(resources)
    }
    
    fn parse_script_table(&mut self, header: &KRBHeader, strings: &[String]) -> Result<Vec<ScriptEntry>> {
        let script_offset = self.read_u32_at(38) as usize;
        let mut scripts = Vec::new();
        
        self.position = script_offset;
        
        for _ in 0..header.script_count {
            let language_id = self.read_u8();
            let name_index = self.read_u8();
            let storage_format = self.read_u8();
            let entry_point_count = self.read_u8();
            let data_size = self.read_u16() as usize;
            
            let language = match language_id {
                0x01 => "lua".to_string(),
                0x02 => "javascript".to_string(),
                0x03 => "python".to_string(),
                0x04 => "wren".to_string(),
                _ => "unknown".to_string(),
            };
            
            let name = if name_index > 0 && (name_index as usize) < strings.len() {
                strings[name_index as usize].clone()
            } else {
                format!("script_{}", scripts.len())
            };
            
            // Parse entry points
            let mut entry_points = Vec::new();
            for _ in 0..entry_point_count {
                let func_name_index = self.read_u8() as usize;
                if func_name_index < strings.len() {
                    entry_points.push(strings[func_name_index].clone());
                }
            }
            
            // Parse script code
            let code = if storage_format == 0 { // Inline
                let code_data = &self.data[self.position..self.position + data_size];
                String::from_utf8_lossy(code_data).to_string()
            } else { // External
                format!("external:{}", data_size) // Resource index
            };
            
            if storage_format == 0 {
                self.position += data_size;
            }
            
            scripts.push(ScriptEntry {
                language,
                name,
                code,
                entry_points,
            });
        }
        
        Ok(scripts)
    }
    
    fn create_default_app_wrapper(elements: &mut HashMap<ElementId, Element>) -> Option<ElementId> {
        if elements.is_empty() {
            return None;
        }
        
        // Find the next available element ID
        let app_id = elements.keys().max().unwrap_or(&0) + 1;
        
        // Create a default App element with sensible defaults
        let mut app_element = Element {
            id: "auto_generated_app".to_string(),
            element_type: ElementType::App,
            parent: None,
            children: Vec::new(),
            style_id: 0,
            position: Vec2::ZERO,
            size: Vec2::new(800.0, 600.0), // Default window size
            layout_flags: 0,
            background_color: Vec4::new(0.1, 0.1, 0.1, 1.0), // Dark gray background
            text_color: Vec4::new(1.0, 1.0, 1.0, 1.0), // White text
            border_color: Vec4::new(0.0, 0.0, 0.0, 0.0), // Transparent border
            border_width: 0.0,
            border_radius: 0.0,
            opacity: 1.0,
            visible: true,
            text: "Auto-generated App".to_string(),
            font_size: 14.0,
            font_weight: crate::elements::FontWeight::Normal,
            text_alignment: crate::elements::TextAlignment::Start,
            cursor: crate::elements::CursorType::Default,
            disabled: false,
            current_state: crate::elements::InteractionState::Normal,
            custom_properties: HashMap::new(),
            state_properties: HashMap::new(),
            event_handlers: HashMap::new(),
            component_name: None,
            is_component_instance: false,
        };
        
        // Collect all current root elements (elements with no parent)
        let mut root_elements = Vec::new();
        for (id, element) in elements.iter() {
            if element.parent.is_none() {
                root_elements.push(*id);
            }
        }
        
        // Make all current root elements children of the new App
        app_element.children = root_elements.clone();
        
        // Update parent references for all root elements
        for root_id in &root_elements {
            if let Some(element) = elements.get_mut(root_id) {
                element.parent = Some(app_id);
            }
        }
        
        // Insert the App element
        elements.insert(app_id, app_element);
        
        eprintln!("[AUTO_APP] Created default App wrapper with ID {} containing {} child elements", 
                 app_id, root_elements.len());
        
        Some(app_id)
    }
    
    fn apply_style_layout_flags(&self, elements: &mut HashMap<ElementId, Element>, styles: &HashMap<u8, Style>) -> Result<()> {
        for (_element_id, element) in elements.iter_mut() {
            if element.style_id > 0 {
                if let Some(style_block) = styles.get(&element.style_id) {
                    // Apply layout flags - Check property ID 0x06 and 0x1A for layout flags
                    let layout_prop = style_block.properties.get(&0x06)
                        .or_else(|| style_block.properties.get(&0x1A));
                    
                    if let Some(layout_prop) = layout_prop {
                        if let Some(layout_flags) = layout_prop.as_int() {
                            let new_flags = layout_flags as u8;
                            eprintln!("[STYLE_LAYOUT] Applying layout flags 0x{:02X} from style '{}' to element", 
                                new_flags, style_block.name);
                            element.layout_flags = new_flags;
                        }
                    }
                    
                    // Apply width property (0x19)
                    if let Some(width_prop) = style_block.properties.get(&0x19) {
                        if let Some(width) = width_prop.as_float() {
                            eprintln!("[STYLE_LAYOUT] Applying width {} from style '{}' to element", 
                                width, style_block.name);
                            element.size.x = width;
                        }
                    }
                    
                    // Apply height property (0x1B)
                    if let Some(height_prop) = style_block.properties.get(&0x1B) {
                        if let Some(height) = height_prop.as_float() {
                            eprintln!("[STYLE_LAYOUT] Applying height {} from style '{}' to element", 
                                height, style_block.name);
                            element.size.y = height;
                        }
                    }
                    
                    // Apply text alignment property (0x0B) to Button and Text elements
                    if element.element_type == ElementType::Button || element.element_type == ElementType::Text {
                        eprintln!("[STYLE_LAYOUT] Checking text alignment for {} element with style_id={}, style_name='{}'", 
                            if element.element_type == ElementType::Button { "Button" } else { "Text" },
                            element.style_id, style_block.name);
                        eprintln!("[STYLE_LAYOUT] Style '{}' has {} properties: {:?}", 
                            style_block.name, style_block.properties.len(), style_block.properties.keys().collect::<Vec<_>>());
                        
                        if let Some(alignment_prop) = style_block.properties.get(&0x0B) {
                            if let Some(alignment) = alignment_prop.as_int() {
                                eprintln!("[STYLE_LAYOUT] Applying text_alignment {} from style '{}' to element", 
                                    alignment, style_block.name);
                                element.text_alignment = match alignment {
                                    0 => TextAlignment::Start,
                                    1 => TextAlignment::Center,
                                    2 => TextAlignment::End,
                                    3 => TextAlignment::Justify,
                                    _ => TextAlignment::Start,
                                };
                            } else {
                                eprintln!("[STYLE_LAYOUT] Found text_alignment property but failed to get as_int()");
                            }
                        } else {
                            eprintln!("[STYLE_LAYOUT] No text_alignment property (0x0B) found in style '{}'", style_block.name);
                        }
                    }
                    
                    // Apply Taffy layout properties to custom_properties
                    let taffy_properties = [
                        (0x40, "display"),
                        (0x41, "flex_direction"),
                        (0x42, "flex_wrap"),
                        (0x43, "flex_grow"),
                        (0x44, "flex_shrink"),
                        (0x45, "flex_basis"),
                        (0x46, "align_items"),
                        (0x47, "align_self"),
                        (0x48, "align_content"),
                        (0x49, "justify_content"),
                        (0x4A, "justify_items"),
                        (0x4B, "justify_self"),
                        (0x50, "position"),
                    ];
                    
                    for (prop_id, prop_name) in taffy_properties {
                        if let Some(taffy_prop) = style_block.properties.get(&prop_id) {
                            element.custom_properties.insert(prop_name.to_string(), taffy_prop.clone());
                            eprintln!("[STYLE_LAYOUT] Applied Taffy property {} ({}) from style '{}' to element", 
                                prop_name, prop_id, style_block.name);
                        }
                    }
                    
                    // Legacy fallback for hardcoded styles
                    if style_block.name == "containerstyle" && element.layout_flags == 0 {
                        element.layout_flags = 0x05;
                        eprintln!("[STYLE_LAYOUT] Applied layout: center (0x05) to containerstyle element");
                    }
                    
                }
            }
        }
        Ok(())
    }
    
    // Helper methods for reading binary data
    fn read_u8(&mut self) -> u8 {
        let value = self.data[self.position];
        self.position += 1;
        value
    }
    
    fn read_u16(&mut self) -> u16 {
        let value = u16::from_le_bytes([self.data[self.position], self.data[self.position + 1]]);
        self.position += 2;
        value
    }
    
    fn read_u16_at(&self, offset: usize) -> u16 {
        u16::from_le_bytes([self.data[offset], self.data[offset + 1]])
    }
    
    fn read_u32(&mut self) -> u32 {
        let value = u32::from_le_bytes([
            self.data[self.position],
            self.data[self.position + 1],
            self.data[self.position + 2],
            self.data[self.position + 3],
        ]);
        self.position += 4;
        value
    }
    
    fn read_u32_at(&self, offset: usize) -> u32 {
        u32::from_le_bytes([
            self.data[offset],
            self.data[offset + 1],
            self.data[offset + 2],
            self.data[offset + 3],
        ])
    }
    
    fn event_type_from_id(&self, id: u8) -> Option<EventType> {
        match id {
            // --- THIS IS THE CORRECTED MAPPING ---
            0x01 => Some(EventType::Click),
            0x02 => Some(EventType::Press),   // Assuming you have this event type
            0x03 => Some(EventType::Release), // Assuming you have this event type
            0x04 => Some(EventType::Hover),
            0x05 => Some(EventType::Focus),
            0x06 => Some(EventType::Blur),
            0x07 => Some(EventType::Change),
            0x08 => Some(EventType::Submit),
            _ => None, // Safely ignore unknown event types
        }
    }
    fn event_type_name(&self, event_type: EventType) -> &'static str {
        match event_type {
            EventType::Click => "Click",
            EventType::Press => "Press",
            EventType::Release => "Release",
            EventType::Hover => "Hover",
            EventType::Focus => "Focus",
            EventType::Blur => "Blur",
            EventType::Change => "Change",
            EventType::Submit => "Submit",
        }
    }
    
    fn read_color(&mut self) -> Vec4 {
        let r = self.read_u8() as f32 / 255.0;
        let g = self.read_u8() as f32 / 255.0;
        let b = self.read_u8() as f32 / 255.0;
        let a = self.read_u8() as f32 / 255.0;
        Vec4::new(r, g, b, a)
    }
}

pub fn load_krb_file(path: &str) -> Result<KRBFile> {
    let data = std::fs::read(path)?;
    let mut parser = KRBParser::new(data);
    let krb_file = parser.parse()?;
    
    // DEBUG: Print everything we parsed
    eprintln!("=== KRB FILE DEBUG ===");
    eprintln!("Header: element_count={}, style_count={}, string_count={}", 
        krb_file.header.element_count, krb_file.header.style_count, krb_file.header.string_count);
    
    // Add explicit style debugging
    if krb_file.header.style_count == 0 {
        eprintln!("WARNING: No styles found in KRB file! This means:");
        eprintln!("  - Styles were not compiled into the KRB");
        eprintln!("  - Elements will use default colors (black text, transparent backgrounds)");
        eprintln!("  - The original .kry file styles were lost during compilation");
    }
    
    eprintln!("Strings ({}):", krb_file.strings.len());
    for (i, s) in krb_file.strings.iter().enumerate() {
        eprintln!("  [{}]: '{}'", i, s);
    }
    
    eprintln!("Elements ({}):", krb_file.elements.len());
    for (id, element) in &krb_file.elements {
        eprintln!("  [{}]: type={:?}, id='{}', pos=({:.1},{:.1}), size=({:.1},{:.1}), children={}, text='{}'",
            id, element.element_type, element.id, 
            element.position.x, element.position.y,
            element.size.x, element.size.y,
            element.children.len(), element.text);
    }
    
    eprintln!("Root element ID: {:?}", krb_file.root_element_id);
    if let Some(root_id) = krb_file.root_element_id {
        if let Some(root_element) = krb_file.elements.get(&root_id) {
            if root_element.id == "auto_generated_app" {
                eprintln!("NOTE: Auto-generated App wrapper created for standalone rendering");
            }
        }
    }
    eprintln!("=== END DEBUG ===");
    
    Ok(krb_file)
}
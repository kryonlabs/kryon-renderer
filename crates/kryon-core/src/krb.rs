// crates/kryon-core/src/krb.rs
use crate::{Element, ElementId, ElementType, PropertyValue, Result, KryonError, TextAlignment, Style, CursorType, InteractionState, EventType, TransformData, TransformType, TransformProperty, TransformPropertyType, CSSUnitValue, CSSUnit, LayoutSize, LayoutPosition, LayoutDimension}; 
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
    pub template_variables: Vec<TemplateVariable>,
    pub template_bindings: Vec<TemplateBinding>,
    pub transforms: Vec<TransformData>,
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
    pub template_variable_count: u16,
    pub template_binding_count: u16,
    pub transform_count: u16,
}

#[derive(Debug, Clone)]
pub struct ScriptEntry {
    pub language: String,
    pub name: String,
    pub code: String,
    pub entry_points: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TemplateVariable {
    pub name: String,
    pub value_type: u8,
    pub default_value: String,
}

#[derive(Debug, Clone)]
pub struct TemplateBinding {
    pub element_index: u16,
    pub property_id: u8,
    pub template_expression: String,
    pub variable_indices: Vec<u8>,
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
        let template_variables = self.parse_template_variables(&header, &strings)?;
        let template_bindings = self.parse_template_bindings(&header, &strings)?;
        let transforms = self.parse_transforms(&header)?;
        
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
            template_variables,
            template_bindings,
            transforms,
        })
    }
    
    fn parse_style_table(&mut self, header: &KRBHeader, strings: &[String]) -> Result<HashMap<u8, Style>> {
        let style_offset = self.read_u32_at(32) as usize;
        let mut styles = HashMap::new();
        
        eprintln!("[STYLE] Parsing {} styles from offset 0x{:X}", header.style_count, style_offset);
        
        self.position = style_offset;
        
        for _i in 0..header.style_count {
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
                    0x1A => { // Width
                        if size == 2 {
                            PropertyValue::Float(self.read_u16() as f32)
                        } else {
                            for _ in 0..size { self.read_u8(); }
                            continue;
                        }
                    }
                    0x1B => { // LayoutFlags
                        if size == 1 {
                            PropertyValue::Int(self.read_u8() as i32)
                        } else {
                            for _ in 0..size { self.read_u8(); }
                            continue;
                        }
                    }
                    0x09 => { // FontSize
                        if size == 2 {
                            PropertyValue::Float(self.read_u16() as f32)
                        } else {
                            for _ in 0..size { self.read_u8(); }
                            continue;
                        }
                    }
                    0x0A => { // FontWeight
                        if size == 2 {
                            PropertyValue::Int(self.read_u16() as i32)
                        } else {
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
                    0x0C => { // FontFamily
                        if size == 1 {
                            let string_index = self.read_u8() as usize;
                            if string_index < strings.len() {
                                PropertyValue::String(strings[string_index].clone())
                            } else {
                                eprintln!("[STYLE]     FontFamily: invalid string index {}", string_index);
                                continue;
                            }
                        } else {
                            for _ in 0..size { self.read_u8(); }
                            continue;
                        }
                    }
                    0x07 => { // Margin
                        if size == 1 {
                            PropertyValue::Float(self.read_u8() as f32)
                        } else {
                            for _ in 0..size { self.read_u8(); }
                            continue;
                        }
                    }
                    0x1C => { // Height
                        if size == 2 {
                            PropertyValue::Float(self.read_u16() as f32)
                        } else {
                            for _ in 0..size { self.read_u8(); }
                            continue;
                        }
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
        if self.data.len() < 68 {
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
            template_variable_count: self.read_u16_at(22),
            template_binding_count: self.read_u16_at(24),
            transform_count: self.read_u16_at(26),
        })
    }
    
    fn parse_string_table(&mut self, header: &KRBHeader) -> Result<Vec<String>> {
        let string_offset = self.read_u32_at(48) as usize;
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
        let element_offset = self.read_u32_at(28) as usize;
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
        // Initialize layout fields with pixel values for now
        element.layout_position = LayoutPosition::pixels(pos_x, pos_y);
        element.layout_size = LayoutSize::pixels(width, height);
        
        // TODO: Parse percentage values from custom properties when compiler supports it
        // For now we'll use a simple enhancement for testing
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
            let _state_flags = self.read_u8();
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
        
        // Convert modern CSS properties to layout_flags if present
        self.convert_css_properties_to_layout_flags(&mut element);
        
        // Check for percentage values in custom properties
        self.parse_percentage_properties(&mut element);
        
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
                if size == 4 {
                    element.background_color = self.read_color();
                    eprintln!("[PROP] BackgroundColor: {:?}", element.background_color);
                } else {
                    eprintln!("[PROP] BackgroundColor: size mismatch, expected 4, got {}, skipping", size);
                    for _ in 0..size { self.read_u8(); }
                }
            }
            0x02 => { // ForegroundColor/TextColor
                if size == 4 {
                    element.text_color = self.read_color();
                    eprintln!("[PROP] TextColor: {:?}", element.text_color);
                } else {
                    eprintln!("[PROP] TextColor: size mismatch, expected 4, got {}, skipping", size);
                    for _ in 0..size { self.read_u8(); }
                }
            }
            0x03 => { // BorderColor
                if size == 4 {
                    element.border_color = self.read_color();
                    eprintln!("[PROP] BorderColor: {:?}", element.border_color);
                } else {
                    eprintln!("[PROP] BorderColor: size mismatch, expected 4, got {}, skipping", size);
                    for _ in 0..size { self.read_u8(); }
                }
            }
            0x04 => { // BorderWidth
                if size == 1 {
                    element.border_width = self.read_u8() as f32;
                    eprintln!("[PROP] BorderWidth: {}", element.border_width);
                } else {
                    eprintln!("[PROP] BorderWidth: size mismatch, expected 1, got {}, skipping", size);
                    for _ in 0..size { self.read_u8(); }
                }
            }
            0x05 => { // BorderRadius
                if size == 1 {
                    element.border_radius = self.read_u8() as f32;
                    eprintln!("[PROP] BorderRadius: {}", element.border_radius);
                } else {
                    eprintln!("[PROP] BorderRadius: size mismatch, expected 1, got {}, skipping", size);
                    for _ in 0..size { self.read_u8(); }
                }
            }
            0x06 => { // Layout flags  
                if size == 1 {
                    let layout_value = self.read_u8();
                    element.layout_flags = layout_value;
                    eprintln!("[PROP] Layout: flags=0x{:02X} (binary: {:08b})", layout_value, layout_value);
                } else {
                    eprintln!("[PROP] Layout: size mismatch, expected 1, got {}, skipping", size);
                    for _ in 0..size { self.read_u8(); }
                }
            }
            0x08 => { // TextContent
                if size == 1 {
                    let string_index = self.read_u8() as usize;
                    if string_index < strings.len() {
                        element.text = strings[string_index].clone();
                        eprintln!("[PROP] TextContent: '{}'", element.text);
                    }
                } else {
                    eprintln!("[PROP] TextContent: size mismatch, expected 1, got {}, skipping", size);
                    for _ in 0..size { self.read_u8(); }
                }
            }
            0x09 => { // FontSize
                if size == 2 {
                    element.font_size = self.read_u16() as f32;
                    eprintln!("[PROP] FontSize: {}", element.font_size);
                } else {
                    eprintln!("[PROP] FontSize: size mismatch, expected 2, got {}, skipping", size);
                    for _ in 0..size { self.read_u8(); }
                }
            }
            0x0A => { // FontWeight
                if size == 2 {
                    let weight = self.read_u16();
                    element.font_weight = match weight {
                        300 => crate::elements::FontWeight::Light,
                        400 => crate::elements::FontWeight::Normal,
                        700 => crate::elements::FontWeight::Bold,
                        900 => crate::elements::FontWeight::Heavy,
                        _ => crate::elements::FontWeight::Normal,
                    };
                    eprintln!("[PROP] FontWeight: {}", weight);
                } else {
                    eprintln!("[PROP] FontWeight: size mismatch, expected 2, got {}, skipping", size);
                    for _ in 0..size { self.read_u8(); }
                }
            }
            0x0C => { // FontFamily
                if size == 1 {
                    let string_index = self.read_u8() as usize;
                    if string_index < strings.len() {
                        element.font_family = strings[string_index].clone();
                        eprintln!("[PROP] FontFamily: '{}'", element.font_family);
                    }
                } else {
                    eprintln!("[PROP] FontFamily: size mismatch, expected 1, got {}, skipping", size);
                    for _ in 0..size { self.read_u8(); }
                }
            }
            0x0E => { // Opacity
                if size == 2 {
                    element.opacity = self.read_u16() as f32 / 256.0; // 8.8 fixed point
                    eprintln!("[PROP] Opacity: {}", element.opacity);
                } else {
                    eprintln!("[PROP] Opacity: size mismatch, expected 2, got {}, skipping", size);
                    for _ in 0..size { self.read_u8(); }
                }
            }
            0x0F => { // ZIndex
                if size == 2 {
                    let z_index = self.read_u16() as i32;
                    element.custom_properties.insert("z_index".to_string(), PropertyValue::Int(z_index));
                    eprintln!("[PROP] ZIndex: {}", z_index);
                } else {
                    eprintln!("[PROP] ZIndex: size mismatch, expected 2, got {}, skipping", size);
                    for _ in 0..size { self.read_u8(); }
                }
            }
            0x0B => { // TextAlignment
                if size == 1 {
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
                } else {
                    eprintln!("[PROP] TextAlignment: size mismatch, expected 1, got {}, skipping", size);
                    for _ in 0..size { self.read_u8(); }
                }
            }
            0x0D => { // ImageSource
                if size == 1 {
                    let string_index = self.read_u8() as usize;
                    if string_index < strings.len() {
                        let image_src = strings[string_index].clone();
                        element.custom_properties.insert("src".to_string(), PropertyValue::String(image_src.clone()));
                        eprintln!("[PROP] ImageSource: '{}'", image_src);
                    }
                } else {
                    eprintln!("[PROP] ImageSource: size mismatch, expected 1, got {}, skipping", size);
                    for _ in 0..size { self.read_u8(); }
                }
            }
            0x10 => { // Visibility
                if size == 1 {
                    element.visible = self.read_u8() != 0;
                    eprintln!("[PROP] Visibility: {}", element.visible);
                } else {
                    eprintln!("[PROP] Visibility: size mismatch, expected 1, got {}, skipping", size);
                    for _ in 0..size { self.read_u8(); }
                }
            }
            0x11 => { // Gap
                if size == 1 {
                    let gap = self.read_u8() as f32;
                    element.custom_properties.insert("gap".to_string(), PropertyValue::Float(gap));
                    eprintln!("[PROP] Gap: {}", gap);
                } else {
                    eprintln!("[PROP] Gap: size mismatch, expected 1, got {}, skipping", size);
                    for _ in 0..size { self.read_u8(); }
                }
            }
            0x12 => { // MinWidth
                if size == 2 {
                    let min_width = self.read_u16() as f32;
                    element.custom_properties.insert("min_width".to_string(), PropertyValue::Float(min_width));
                    eprintln!("[PROP] MinWidth: {}", min_width);
                } else {
                    eprintln!("[PROP] MinWidth: size mismatch, expected 2, got {}, skipping", size);
                    for _ in 0..size { self.read_u8(); }
                }
            }
            0x13 => { // MinHeight
                if size == 2 {
                    let min_height = self.read_u16() as f32;
                    element.custom_properties.insert("min_height".to_string(), PropertyValue::Float(min_height));
                    eprintln!("[PROP] MinHeight: {}", min_height);
                } else {
                    eprintln!("[PROP] MinHeight: size mismatch, expected 2, got {}, skipping", size);
                    for _ in 0..size { self.read_u8(); }
                }
            }
            0x14 => { // MaxWidth
                if size == 2 {
                    let max_width = self.read_u16() as f32;
                    element.custom_properties.insert("max_width".to_string(), PropertyValue::Float(max_width));
                    eprintln!("[PROP] MaxWidth: {}", max_width);
                } else {
                    eprintln!("[PROP] MaxWidth: size mismatch, expected 2, got {}, skipping", size);
                    for _ in 0..size { self.read_u8(); }
                }
            }
            0x15 => { // MaxHeight
                let max_height = self.read_u16() as f32;
                element.custom_properties.insert("max_height".to_string(), PropertyValue::Float(max_height));
                eprintln!("[PROP] MaxHeight: {}", max_height);
            }
            // App-specific properties
            0x20 => { // WindowWidth
                if size == 2 {
                    let width = self.read_u16();
                    eprintln!("[PROP] WindowWidth: {}", width);
                    // App elements use this for initial size
                    if element.element_type == ElementType::App {
                        element.size.x = width as f32;
                    }
                } else {
                    eprintln!("[PROP] WindowWidth: size mismatch, expected 2, got {}, skipping", size);
                    for _ in 0..size { self.read_u8(); }
                }
            }
            0x21 => { // WindowHeight  
                if size == 2 {
                    let height = self.read_u16();
                    eprintln!("[PROP] WindowHeight: {}", height);
                    if element.element_type == ElementType::App {
                        element.size.y = height as f32;
                    }
                } else {
                    eprintln!("[PROP] WindowHeight: size mismatch, expected 2, got {}, skipping", size);
                    for _ in 0..size { self.read_u8(); }
                }
            }
            0x22 => { // WindowTitle
                if size == 1 {
                    let string_index = self.read_u8() as usize;
                    if string_index < strings.len() {
                        eprintln!("[PROP] WindowTitle: '{}'", strings[string_index]);
                        // Could store in custom properties if needed
                    }
                } else {
                    eprintln!("[PROP] WindowTitle: size mismatch, expected 1, got {}, skipping", size);
                    for _ in 0..size { self.read_u8(); }
                }
            }
            0x23 => { // Resizable
                if size == 1 {
                    let resizable = self.read_u8() != 0;
                    eprintln!("[PROP] Resizable: {}", resizable);
                } else {
                    eprintln!("[PROP] Resizable: size mismatch, expected 1, got {}, skipping", size);
                    for _ in 0..size { self.read_u8(); }
                }
            }
            0x24 => { // KeepAspectRatio
                if size == 1 {
                    let keep_aspect = self.read_u8() != 0;
                    eprintln!("[PROP] KeepAspectRatio: {}", keep_aspect);
                } else {
                    eprintln!("[PROP] KeepAspectRatio: size mismatch, expected 1, got {}, skipping", size);
                    for _ in 0..size { self.read_u8(); }
                }
            }
            0x25 => { // ScaleFactor
                if size == 2 {
                    let scale = self.read_u16() as f32 / 256.0; // 8.8 fixed point
                    eprintln!("[PROP] ScaleFactor: {}", scale);
                } else {
                    eprintln!("[PROP] ScaleFactor: size mismatch, expected 2, got {}, skipping", size);
                    for _ in 0..size { self.read_u8(); }
                }
            }
            0x26 => { // Icon
                if size == 1 {
                    let string_index = self.read_u8() as usize;
                    if string_index < strings.len() {
                        eprintln!("[PROP] Icon: '{}'", strings[string_index]);
                    }
                } else {
                    eprintln!("[PROP] Icon: size mismatch, expected 1, got {}, skipping", size);
                    for _ in 0..size { self.read_u8(); }
                }
            }
            0x27 => { // Version
                if size == 1 {
                    let string_index = self.read_u8() as usize;
                    if string_index < strings.len() {
                        eprintln!("[PROP] Version: '{}'", strings[string_index]);
                    }
                } else {
                    eprintln!("[PROP] Version: size mismatch, expected 1, got {}, skipping", size);
                    for _ in 0..size { self.read_u8(); }
                }
            }
            0x28 => { // Author
                if size == 1 {
                    let string_index = self.read_u8() as usize;
                    if string_index < strings.len() {
                        eprintln!("[PROP] Author: '{}'", strings[string_index]);
                    }
                } else {
                    eprintln!("[PROP] Author: size mismatch, expected 1, got {}, skipping", size);
                    for _ in 0..size { self.read_u8(); }
                }
            }
            0x29 => { // Cursor
                if size == 1 {
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
                } else {
                    eprintln!("[PROP] Cursor: size mismatch, expected 1, got {}, skipping", size);
                    for _ in 0..size { self.read_u8(); }
                }
            }
            0x1A => { // Width
                if size == 2 {
                    let width = self.read_u16() as f32;
                    element.size.x = width;
                    eprintln!("[PROP] Width: {}", width);
                } else {
                    eprintln!("[PROP] Width: size mismatch, expected 2, got {}, skipping", size);
                    for _ in 0..size { self.read_u8(); }
                }
            }
            0x1B => { // LayoutFlags (layout property)
                if size == 1 {
                    let layout_value = self.read_u8();
                    element.layout_flags = layout_value;
                    eprintln!("[PROP] LayoutFlags: flags=0x{:02X} (binary: {:08b})", layout_value, layout_value);
                } else {
                    eprintln!("[PROP] LayoutFlags: size mismatch, expected 1, got {}, skipping", size);
                    for _ in 0..size { self.read_u8(); }
                }
            }
            0x1C => { // Height
                if size == 2 {
                    let height = self.read_u16() as f32;
                    element.size.y = height;
                    eprintln!("[PROP] Height: {}", height);
                } else {
                    eprintln!("[PROP] Height: size mismatch, expected 2, got {}, skipping", size);
                    for _ in 0..size { self.read_u8(); }
                }
            }
            0x2B => { // InputType
                if size == 1 && value_type == 0x09 { // Enum type
                    let input_type_value = self.read_u8();
                    // Map the enum value to a string representation
                    let input_type_name = match input_type_value {
                        0x00 => "text",
                        0x01 => "password",
                        0x02 => "email",
                        0x03 => "number",
                        0x04 => "tel",
                        0x05 => "url",
                        0x06 => "search",
                        0x10 => "checkbox",
                        0x11 => "radio",
                        0x20 => "range",
                        0x30 => "date",
                        0x31 => "datetime-local",
                        0x32 => "month",
                        0x33 => "time",
                        0x34 => "week",
                        0x40 => "color",
                        0x41 => "file",
                        0x42 => "hidden",
                        0x50 => "submit",
                        0x51 => "reset",
                        0x52 => "button",
                        0x53 => "image",
                        _ => "text", // Default to text for unknown types
                    };
                    element.custom_properties.insert("input_type".to_string(), PropertyValue::String(input_type_name.to_string()));
                    eprintln!("[PROP] InputType: '{}' (0x{:02X})", input_type_name, input_type_value);
                } else {
                    eprintln!("[PROP] InputType: size mismatch or wrong type, expected size=1 type=0x09, got size={} type=0x{:02X}, skipping", size, value_type);
                    for _ in 0..size { self.read_u8(); }
                }
            }
            // Modern Taffy layout properties (0x40-0x4F range)
            0x40 => { // Display
                if size == 1 {
                    let string_index = self.read_u8() as usize;
                    if string_index < strings.len() {
                        let display_value = strings[string_index].clone();
                        element.custom_properties.insert("display".to_string(), PropertyValue::String(display_value.clone()));
                        eprintln!("[PROP] Display: '{}'", display_value);
                    }
                } else {
                    eprintln!("[PROP] Display: size mismatch, expected 1, got {}, skipping", size);
                    for _ in 0..size { self.read_u8(); }
                }
            }
            0x41 => { // FlexDirection
                if size == 1 {
                    let string_index = self.read_u8() as usize;
                    if string_index < strings.len() {
                        let flex_direction = strings[string_index].clone();
                        element.custom_properties.insert("flex_direction".to_string(), PropertyValue::String(flex_direction.clone()));
                        eprintln!("[PROP] FlexDirection: '{}'", flex_direction);
                    }
                } else {
                    eprintln!("[PROP] FlexDirection: size mismatch, expected 1, got {}, skipping", size);
                    for _ in 0..size { self.read_u8(); }
                }
            }
            0x42 => { // FlexWrap
                if size == 1 {
                    let string_index = self.read_u8() as usize;
                    if string_index < strings.len() {
                        let flex_wrap = strings[string_index].clone();
                        element.custom_properties.insert("flex_wrap".to_string(), PropertyValue::String(flex_wrap.clone()));
                        eprintln!("[PROP] FlexWrap: '{}'", flex_wrap);
                    }
                } else {
                    eprintln!("[PROP] FlexWrap: size mismatch, expected 1, got {}, skipping", size);
                    for _ in 0..size { self.read_u8(); }
                }
            }
            0x43 => { // FlexGrow
                if size == 4 {
                    let flex_grow_bytes = [self.read_u8(), self.read_u8(), self.read_u8(), self.read_u8()];
                    let flex_grow = f32::from_le_bytes(flex_grow_bytes);
                    element.custom_properties.insert("flex_grow".to_string(), PropertyValue::Float(flex_grow));
                    eprintln!("[PROP] FlexGrow: {}", flex_grow);
                } else {
                    eprintln!("[PROP] FlexGrow: size mismatch, expected 4, got {}, skipping", size);
                    for _ in 0..size { self.read_u8(); }
                }
            }
            0x44 => { // FlexShrink
                if size == 4 {
                    let flex_shrink_bytes = [self.read_u8(), self.read_u8(), self.read_u8(), self.read_u8()];
                    let flex_shrink = f32::from_le_bytes(flex_shrink_bytes);
                    element.custom_properties.insert("flex_shrink".to_string(), PropertyValue::Float(flex_shrink));
                    eprintln!("[PROP] FlexShrink: {}", flex_shrink);
                } else {
                    eprintln!("[PROP] FlexShrink: size mismatch, expected 4, got {}, skipping", size);
                    for _ in 0..size { self.read_u8(); }
                }
            }
            0x45 => { // FlexBasis
                if size == 1 {
                    let string_index = self.read_u8() as usize;
                    if string_index < strings.len() {
                        let flex_basis = strings[string_index].clone();
                        element.custom_properties.insert("flex_basis".to_string(), PropertyValue::String(flex_basis.clone()));
                        eprintln!("[PROP] FlexBasis: '{}'", flex_basis);
                    }
                } else {
                    eprintln!("[PROP] FlexBasis: size mismatch, expected 1, got {}, skipping", size);
                    for _ in 0..size { self.read_u8(); }
                }
            }
            0x46 => { // AlignItems
                if size == 1 {
                    let string_index = self.read_u8() as usize;
                    if string_index < strings.len() {
                        let align_items = strings[string_index].clone();
                        element.custom_properties.insert("align_items".to_string(), PropertyValue::String(align_items.clone()));
                        eprintln!("[PROP] AlignItems: '{}'", align_items);
                    }
                } else {
                    eprintln!("[PROP] AlignItems: size mismatch, expected 1, got {}, skipping", size);
                    for _ in 0..size { self.read_u8(); }
                }
            }
            0x47 => { // AlignSelf
                if size == 1 {
                    let string_index = self.read_u8() as usize;
                    if string_index < strings.len() {
                        let align_self = strings[string_index].clone();
                        element.custom_properties.insert("align_self".to_string(), PropertyValue::String(align_self.clone()));
                        eprintln!("[PROP] AlignSelf: '{}'", align_self);
                    }
                } else {
                    eprintln!("[PROP] AlignSelf: size mismatch, expected 1, got {}, skipping", size);
                    for _ in 0..size { self.read_u8(); }
                }
            }
            0x48 => { // AlignContent
                if size == 1 {
                    let string_index = self.read_u8() as usize;
                    if string_index < strings.len() {
                        let align_content = strings[string_index].clone();
                        element.custom_properties.insert("align_content".to_string(), PropertyValue::String(align_content.clone()));
                        eprintln!("[PROP] AlignContent: '{}'", align_content);
                    }
                } else {
                    eprintln!("[PROP] AlignContent: size mismatch, expected 1, got {}, skipping", size);
                    for _ in 0..size { self.read_u8(); }
                }
            }
            0x49 => { // JustifyContent
                if size == 1 {
                    let string_index = self.read_u8() as usize;
                    if string_index < strings.len() {
                        let justify_content = strings[string_index].clone();
                        element.custom_properties.insert("justify_content".to_string(), PropertyValue::String(justify_content.clone()));
                        eprintln!("[PROP] JustifyContent: '{}'", justify_content);
                    }
                } else {
                    eprintln!("[PROP] JustifyContent: size mismatch, expected 1, got {}, skipping", size);
                    for _ in 0..size { self.read_u8(); }
                }
            }
            0x4A => { // JustifyItems
                if size == 1 {
                    let string_index = self.read_u8() as usize;
                    if string_index < strings.len() {
                        let justify_items = strings[string_index].clone();
                        element.custom_properties.insert("justify_items".to_string(), PropertyValue::String(justify_items.clone()));
                        eprintln!("[PROP] JustifyItems: '{}'", justify_items);
                    }
                } else {
                    eprintln!("[PROP] JustifyItems: size mismatch, expected 1, got {}, skipping", size);
                    for _ in 0..size { self.read_u8(); }
                }
            }
            0x4B => { // JustifySelf
                if size == 1 {
                    let string_index = self.read_u8() as usize;
                    if string_index < strings.len() {
                        let justify_self = strings[string_index].clone();
                        element.custom_properties.insert("justify_self".to_string(), PropertyValue::String(justify_self.clone()));
                        eprintln!("[PROP] JustifySelf: '{}'", justify_self);
                    }
                } else {
                    eprintln!("[PROP] JustifySelf: size mismatch, expected 1, got {}, skipping", size);
                    for _ in 0..size { self.read_u8(); }
                }
            }
            0x50 => { // Position
                if size == 1 {
                    let string_index = self.read_u8() as usize;
                    if string_index < strings.len() {
                        let position = strings[string_index].clone();
                        element.custom_properties.insert("position".to_string(), PropertyValue::String(position.clone()));
                        eprintln!("[PROP] Position: '{}'", position);
                    }
                } else {
                    eprintln!("[PROP] Position: size mismatch, expected 1, got {}, skipping", size);
                    for _ in 0..size { self.read_u8(); }
                }
            }
            0x16 => { // Transform
                // For now, we'll parse transform data as a simple index into the transforms array
                // In a full implementation, this would reference the transform data parsed earlier
                let transform_index = self.read_u8() as usize;
                element.custom_properties.insert("transform_index".to_string(), PropertyValue::Int(transform_index as i32));
                eprintln!("[PROP] Transform: index={}", transform_index);
                
                // Skip remaining bytes if any
                for _ in 1..size {
                    self.read_u8();
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
            // Invalid key - must still consume the value bytes to stay in sync
            for _ in 0..size {
                self.read_u8();
            }
            return Ok(());
        };
        
        let value = match value_type {
            0x01 if size == 1 => PropertyValue::Int(self.read_u8() as i32),
            0x02 if size == 2 => PropertyValue::Int(self.read_u16() as i32),
            0x03 if size == 4 => PropertyValue::Color(self.read_color()),
            0x04 if size == 1 => {
                let string_index = self.read_u8() as usize;
                if string_index < strings.len() {
                    PropertyValue::String(strings[string_index].clone())
                } else {
                    PropertyValue::String(String::new())
                }
            }
            _ => {
                // Unknown value type or size mismatch - consume bytes and skip
                for _ in 0..size {
                    self.read_u8();
                }
                return Ok(());
            }
        };
        
        element.custom_properties.insert(key, value);
        Ok(())
    }
    
    /// Convert modern CSS properties to legacy layout_flags
    fn convert_css_properties_to_layout_flags(&self, element: &mut Element) {
        let mut layout_flags = element.layout_flags; // Start with existing flags
        
        // Extract CSS properties
        let display = element.custom_properties.get("display")
            .and_then(|v| if let PropertyValue::String(s) = v { Some(s) } else { None });
        let flex_direction = element.custom_properties.get("flex_direction")
            .and_then(|v| if let PropertyValue::String(s) = v { Some(s) } else { None });
        let align_items = element.custom_properties.get("align_items")
            .and_then(|v| if let PropertyValue::String(s) = v { Some(s) } else { None });
        let justify_content = element.custom_properties.get("justify_content")
            .and_then(|v| if let PropertyValue::String(s) = v { Some(s) } else { None });
        let flex_grow = element.custom_properties.get("flex_grow")
            .and_then(|v| if let PropertyValue::Float(f) = v { Some(*f) } else { None });
        
        // Process flex container properties if we have display: flex
        if let Some(display_val) = display {
            if display_val == "flex" {
                // Set direction
                if let Some(direction) = flex_direction {
                    match direction.as_str() {
                        "row" => {
                            layout_flags = (layout_flags & !0x03) | 0x00; // LAYOUT_DIRECTION_ROW
                        }
                        "column" => {
                            layout_flags = (layout_flags & !0x03) | 0x01; // LAYOUT_DIRECTION_COLUMN
                        }
                        _ => {}
                    }
                }
                
                // Set alignment based on justify_content and align_items
                // For flexbox, justify_content controls main axis, align_items controls cross axis
                // Our layout engine uses alignment for both cross axis and main axis centering
                let mut should_center = false;
                
                if let Some(align) = align_items {
                    match align.as_str() {
                        "center" | "centre" => {
                            should_center = true;
                        }
                        "flex-start" | "start" => {
                            layout_flags = (layout_flags & !0x0C) | 0x00; // LAYOUT_ALIGNMENT_START  
                        }
                        "flex-end" | "end" => {
                            layout_flags = (layout_flags & !0x0C) | 0x08; // LAYOUT_ALIGNMENT_END
                        }
                        _ => {}
                    }
                }
                
                // For justify_content=center, also enable centering
                if let Some(justify) = justify_content {
                    match justify.as_str() {
                        "center" | "centre" => {
                            should_center = true;
                        }
                        _ => {}
                    }
                }
                
                if should_center {
                    layout_flags = (layout_flags & !0x0C) | 0x04; // LAYOUT_ALIGNMENT_CENTER
                }
                
                eprintln!("[CSS_CONVERT] Element '{}': display={}, flex_direction={:?}, align_items={:?}, justify_content={:?} -> layout_flags=0x{:02X}", 
                    element.id, display_val, flex_direction, align_items, justify_content, layout_flags);
                
                element.layout_flags = layout_flags;
            }
        }
        
        // Process flex item properties (like flex_grow) regardless of display property
        // These apply to children of flex containers
        if let Some(grow) = flex_grow {
            if grow > 0.0 {
                layout_flags |= 0x20; // LAYOUT_GROW_BIT
                eprintln!("[CSS_CONVERT_FLEX_ITEM] Element '{}': flex_grow={} -> layout_flags=0x{:02X}", 
                    element.id, grow, layout_flags);
                element.layout_flags = layout_flags;
            }
        }
    }
    
    fn parse_resource_table(&mut self, header: &KRBHeader) -> Result<Vec<String>> {
        let resource_offset = self.read_u32_at(52) as usize;
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
        let script_offset = self.read_u32_at(44) as usize;
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
    
    fn parse_template_variables(&mut self, header: &KRBHeader, strings: &[String]) -> Result<Vec<TemplateVariable>> {
        let template_var_offset = self.read_u32_at(56) as usize;
        let mut template_variables = Vec::new();
        
        println!("PARSE: template_variable_count = {}, offset = 0x{:X}", header.template_variable_count, template_var_offset);
        
        self.position = template_var_offset;
        
        for i in 0..header.template_variable_count {
            let name_index = self.read_u8() as usize;
            let value_type = self.read_u8();
            let default_value_index = self.read_u8() as usize;
            
            let name = if name_index < strings.len() {
                strings[name_index].clone()
            } else {
                format!("template_var_{}", template_variables.len())
            };
            
            let default_value = if default_value_index < strings.len() {
                strings[default_value_index].clone()
            } else {
                String::new()
            };
            
            println!("PARSE: template_variable[{}]: name='{}' (idx={}), type={}, default='{}' (idx={})", 
                i, name, name_index, value_type, default_value, default_value_index);
            
            template_variables.push(TemplateVariable {
                name,
                value_type,
                default_value,
            });
        }
        
        Ok(template_variables)
    }
    
    fn parse_template_bindings(&mut self, header: &KRBHeader, strings: &[String]) -> Result<Vec<TemplateBinding>> {
        let template_binding_offset = self.read_u32_at(60) as usize;
        let mut template_bindings = Vec::new();
        
        println!("PARSE: template_binding_count = {}, offset = 0x{:X}", header.template_binding_count, template_binding_offset);
        
        self.position = template_binding_offset;
        
        for i in 0..header.template_binding_count {
            let element_index = self.read_u16();
            let property_id = self.read_u8();
            let template_expression_index = self.read_u8() as usize;
            let variable_count = self.read_u8();
            
            let template_expression = if template_expression_index < strings.len() {
                strings[template_expression_index].clone()
            } else {
                String::new()
            };
            
            let mut variable_indices = Vec::new();
            for _ in 0..variable_count {
                variable_indices.push(self.read_u8());
            }
            
            println!("PARSE: template_binding[{}]: element={}, property=0x{:02X}, expr='{}' (idx={}), vars={:?}", 
                i, element_index, property_id, template_expression, template_expression_index, variable_indices);
            
            template_bindings.push(TemplateBinding {
                element_index,
                property_id,
                template_expression,
                variable_indices,
            });
        }
        
        Ok(template_bindings)
    }
    
    fn parse_transforms(&mut self, header: &KRBHeader) -> Result<Vec<TransformData>> {
        let transform_offset = self.read_u32_at(64) as usize;
        let mut transforms = Vec::new();
        
        println!("PARSE: transform_count = {}, offset = 0x{:X}", header.transform_count, transform_offset);
        
        self.position = transform_offset;
        
        for i in 0..header.transform_count {
            let transform_type = self.read_u8();
            let property_count = self.read_u8();
            
            let transform_type_enum = match transform_type {
                0x01 => TransformType::Transform2D,
                0x02 => TransformType::Transform3D,
                0x03 => TransformType::Matrix2D,
                0x04 => TransformType::Matrix3D,
                _ => TransformType::Transform2D, // Default fallback
            };
            
            let mut properties = Vec::new();
            for j in 0..property_count {
                let property_type = self.read_u8();
                let value_type = self.read_u8();
                let size = self.read_u8();
                
                let property_type_enum = match property_type {
                    0x01 => TransformPropertyType::Scale,
                    0x02 => TransformPropertyType::ScaleX,
                    0x03 => TransformPropertyType::ScaleY,
                    0x04 => TransformPropertyType::TranslateX,
                    0x05 => TransformPropertyType::TranslateY,
                    0x06 => TransformPropertyType::Rotate,
                    0x07 => TransformPropertyType::SkewX,
                    0x08 => TransformPropertyType::SkewY,
                    0x09 => TransformPropertyType::ScaleZ,
                    0x0A => TransformPropertyType::TranslateZ,
                    0x0B => TransformPropertyType::RotateX,
                    0x0C => TransformPropertyType::RotateY,
                    0x0D => TransformPropertyType::RotateZ,
                    0x0E => TransformPropertyType::Perspective,
                    0x0F => TransformPropertyType::Matrix,
                    _ => TransformPropertyType::Scale, // Default fallback
                };
                
                // Parse the value based on value_type
                let css_unit_value = match value_type {
                    0x19 => { // CSSUnit
                        if size >= 9 { // 8 bytes for f64 + 1 byte for unit
                            let value_bytes = [self.read_u8(), self.read_u8(), self.read_u8(), self.read_u8(),
                                             self.read_u8(), self.read_u8(), self.read_u8(), self.read_u8()];
                            let value = f64::from_le_bytes(value_bytes);
                            let unit_byte = self.read_u8();
                            
                            let unit = match unit_byte {
                                0x01 => CSSUnit::Pixels,
                                0x02 => CSSUnit::Em,
                                0x03 => CSSUnit::Rem,
                                0x04 => CSSUnit::ViewportWidth,
                                0x05 => CSSUnit::ViewportHeight,
                                0x06 => CSSUnit::Percentage,
                                0x07 => CSSUnit::Degrees,
                                0x08 => CSSUnit::Radians,
                                0x09 => CSSUnit::Turns,
                                0x0A => CSSUnit::Number,
                                _ => CSSUnit::Number, // Default fallback
                            };
                            
                            CSSUnitValue { value, unit }
                        } else {
                            // Skip malformed data
                            for _ in 0..size {
                                self.read_u8();
                            }
                            continue;
                        }
                    }
                    _ => {
                        // Skip unknown value types
                        for _ in 0..size {
                            self.read_u8();
                        }
                        continue;
                    }
                };
                
                properties.push(TransformProperty {
                    property_type: property_type_enum,
                    value: css_unit_value,
                });
                
                println!("PARSE: transform[{}].property[{}]: type={:?}, value={:?}", 
                    i, j, property_type_enum, properties.last().unwrap().value);
            }
            
            transforms.push(TransformData {
                transform_type: transform_type_enum,
                properties,
            });
            
            println!("PARSE: transform[{}]: type={:?}, properties={}", 
                i, transform_type_enum, transforms.last().unwrap().properties.len());
        }
        
        Ok(transforms)
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
            layout_position: LayoutPosition::pixels(0.0, 0.0),
            layout_size: LayoutSize::pixels(800.0, 600.0),
            layout_flags: 0,
            gap: 0.0,
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
            font_family: "default".to_string(),
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
                    
                    // Apply width property (0x1A)
                    if let Some(width_prop) = style_block.properties.get(&0x1A) {
                        if let Some(width) = width_prop.as_float() {
                            eprintln!("[STYLE_LAYOUT] Applying width {} from style '{}' to element", 
                                width, style_block.name);
                            element.size.x = width;
                        }
                    }
                    
                    // Apply height property (0x1C)
                    if let Some(height_prop) = style_block.properties.get(&0x1C) {
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
                        
                        // Apply font properties
                        if let Some(font_size_prop) = style_block.properties.get(&0x09) {
                            if let Some(font_size) = font_size_prop.as_float() {
                                eprintln!("[STYLE_LAYOUT] Applying font_size {} from style '{}' to element", 
                                    font_size, style_block.name);
                                element.font_size = font_size;
                            }
                        }
                        
                        if let Some(font_weight_prop) = style_block.properties.get(&0x0A) {
                            if let Some(weight) = font_weight_prop.as_int() {
                                eprintln!("[STYLE_LAYOUT] Applying font_weight {} from style '{}' to element", 
                                    weight, style_block.name);
                                element.font_weight = match weight {
                                    300 => crate::elements::FontWeight::Light,
                                    700 => crate::elements::FontWeight::Bold,
                                    900 => crate::elements::FontWeight::Heavy,
                                    _ => crate::elements::FontWeight::Normal,
                                };
                            }
                        }
                        
                        if let Some(font_family_prop) = style_block.properties.get(&0x0C) {
                            if let Some(font_family) = font_family_prop.as_string() {
                                eprintln!("[STYLE_LAYOUT] Applying font_family '{}' from style '{}' to element", 
                                    font_family, style_block.name);
                                element.font_family = font_family.to_string();
                            }
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
        if self.position >= self.data.len() {
            eprintln!("DEBUG: Attempted to read u8 at position {} but file is only {} bytes", self.position, self.data.len());
            eprintln!("DEBUG: This suggests the KRB file is truncated or the parser is reading more than expected");
            panic!("KRB parsing error: trying to read u8 at position {} but data length is {}", self.position, self.data.len());
        }
        let value = self.data[self.position];
        self.position += 1;
        value
    }
    
    fn read_u16(&mut self) -> u16 {
        if self.position + 1 >= self.data.len() {
            panic!("KRB parsing error: trying to read u16 at position {} but data length is {}", self.position, self.data.len());
        }
        let value = u16::from_le_bytes([self.data[self.position], self.data[self.position + 1]]);
        self.position += 2;
        value
    }
    
    fn read_u16_at(&self, offset: usize) -> u16 {
        u16::from_le_bytes([self.data[offset], self.data[offset + 1]])
    }
    
    fn _read_u32(&mut self) -> u32 {
        if self.position + 3 >= self.data.len() {
            panic!("KRB parsing error: trying to read u32 at position {} but data length is {}", self.position, self.data.len());
        }
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
    
    /// Parse percentage values from custom properties and update layout fields
    fn parse_percentage_properties(&self, element: &mut Element) {
        // Check for width percentage
        if let Some(PropertyValue::String(width_str)) = element.custom_properties.get("width") {
            if let Ok(dimension) = self.parse_dimension_string(width_str) {
                element.layout_size.width = dimension;
            }
        }
        
        // Check for height percentage
        if let Some(PropertyValue::String(height_str)) = element.custom_properties.get("height") {
            if let Ok(dimension) = self.parse_dimension_string(height_str) {
                element.layout_size.height = dimension;
            }
        }
        
        // Check for position percentages
        if let Some(PropertyValue::String(x_str)) = element.custom_properties.get("pos_x") {
            if let Ok(dimension) = self.parse_dimension_string(x_str) {
                element.layout_position.x = dimension;
            }
        }
        
        if let Some(PropertyValue::String(y_str)) = element.custom_properties.get("pos_y") {
            if let Ok(dimension) = self.parse_dimension_string(y_str) {
                element.layout_position.y = dimension;
            }
        }
    }
    
    /// Parse a dimension string (e.g., "50%", "100px", "auto") into a LayoutDimension
    fn parse_dimension_string(&self, value: &str) -> Result<LayoutDimension> {
        Ok(LayoutDimension::from_string(value))
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
    eprintln!("Header: element_count={}, style_count={}, string_count={}, transform_count={}", 
        krb_file.header.element_count, krb_file.header.style_count, krb_file.header.string_count, krb_file.header.transform_count);
    
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
    
    eprintln!("Transforms ({}):", krb_file.transforms.len());
    for (i, transform) in krb_file.transforms.iter().enumerate() {
        eprintln!("  [{}]: type={:?}, properties={}", i, transform.transform_type, transform.properties.len());
        for (j, prop) in transform.properties.iter().enumerate() {
            eprintln!("    [{}]: {:?} = {:?}", j, prop.property_type, prop.value);
        }
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
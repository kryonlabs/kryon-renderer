// crates/kryon-core/src/krb.rs
use crate::{Element, ElementType, PropertyValue, Result, KryonError};
use std::collections::HashMap;
use glam::{Vec2, Vec4};

#[derive(Debug)]
pub struct KRBFile {
    pub header: KRBHeader,
    pub strings: Vec<String>,
    pub elements: HashMap<u32, Element>,
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
        
        if header.version > 0x0005 {
            return Err(KryonError::UnsupportedVersion(header.version));
        }
        
        let strings = self.parse_string_table(&header)?;
        let elements = self.parse_element_tree(&header, &strings)?;
        let resources = self.parse_resource_table(&header)?;
        let scripts = self.parse_script_table(&header, &strings)?;
        
        // Find root element (App type)
        let root_element_id = elements.iter()
            .find(|(_, element)| element.element_type == ElementType::App)
            .map(|(id, _)| *id);
        
        Ok(KRBFile {
            header,
            strings,
            elements,
            root_element_id,
            resources,
            scripts,
        })
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
        
        Ok(elements)
    }
    
    fn parse_element(&mut self, element_id: u32, strings: &[String]) -> Result<Element> {
        let mut element = Element::default();
        
        // Parse element header (18 bytes)
        let element_type = ElementType::from(self.read_u8());
        let id_index = self.read_u8();
        let pos_x = self.read_u16() as f32;
        let pos_y = self.read_u16() as f32;
        let width = self.read_u16() as f32;
        let height = self.read_u16() as f32;
        let layout_flags = self.read_u8();
        let _style_id = self.read_u8();
        let property_count = self.read_u8();
        let child_count = self.read_u8();
        let _event_count = self.read_u8();
        let _animation_count = self.read_u8();
        let custom_prop_count = self.read_u8();
        let _state_prop_count = self.read_u8();
        
        element.element_type = element_type;
        element.id = if id_index > 0 && (id_index as usize) < strings.len() {
            strings[id_index as usize].clone()
        } else {
            format!("element_{}", element_id)
        };
        element.position = Vec2::new(pos_x, pos_y);
        element.size = Vec2::new(width, height);
        element.layout_flags = layout_flags;
        
        // Parse standard properties
        for _ in 0..property_count {
            self.parse_standard_property(&mut element, strings)?;
        }
        
        // Parse custom properties
        for _ in 0..custom_prop_count {
            self.parse_custom_property(&mut element, strings)?;
        }
        
        // Store child count for later linking
        element.children = vec![0; child_count as usize]; // Placeholder
        
        Ok(element)
    }
    
    fn parse_standard_property(&mut self, element: &mut Element, strings: &[String]) -> Result<()> {
        let property_id = self.read_u8();
        let value_type = self.read_u8();
        
        match property_id {
            0x01 => { // BackgroundColor
                element.background_color = self.read_color();
            }
            0x02 => { // ForegroundColor/TextColor
                element.text_color = self.read_color();
            }
            0x03 => { // BorderColor
                element.border_color = self.read_color();
            }
            0x04 => { // BorderWidth
                element.border_width = self.read_u8() as f32;
            }
            0x05 => { // BorderRadius
                element.border_radius = self.read_u8() as f32;
            }
            0x08 => { // TextContent
                let string_index = self.read_u8() as usize;
                if string_index < strings.len() {
                    element.text = strings[string_index].clone();
                }
            }
            0x09 => { // FontSize
                element.font_size = self.read_u16() as f32;
            }
            0x0D => { // Opacity
                element.opacity = self.read_u16() as f32 / 256.0; // 8.8 fixed point
            }
            0x0F => { // Visibility
                element.visible = self.read_u8() != 0;
            }
            _ => {
                // Skip unknown property
                match value_type {
                    0x01 => { self.read_u8(); } // Byte
                    0x02 => { self.read_u16(); } // Short
                    0x03 => { self.read_color(); } // Color
                    0x04 => { self.read_u8(); } // String index
                    _ => {}
                }
            }
        }
        
        Ok(())
    }
    
    fn parse_custom_property(&mut self, element: &mut Element, strings: &[String]) -> Result<()> {
        let key_index = self.read_u8() as usize;
        let value_type = self.read_u8();
        
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
    parser.parse()
}
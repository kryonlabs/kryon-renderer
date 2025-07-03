// crates/kryon-core/src/style.rs

use crate::{Element, ElementId, PropertyValue};
use glam::Vec4;
use std::collections::HashMap;
use std::cell::RefCell;

/// Represents a single style block from the .krb file, like "appstyle".
#[derive(Debug, Clone)]
pub struct Style {
    pub name: String,
    // A map of Property ID -> Value. e.g., 0x01 -> PropertyValue::Color(...)
    pub properties: HashMap<u8, PropertyValue>,
}

/// Holds the final, calculated style values for a single element after inheritance.
/// This is the "single source of truth" for the renderer.
#[derive(Debug, Clone, Copy)]
pub struct ComputedStyle {
    pub background_color: Vec4,
    pub text_color: Vec4,
    pub border_color: Vec4,
    pub border_width: f32,
    pub border_radius: f32,
    // Add other inheritable properties here as needed (font_size, etc.)
}

impl Default for ComputedStyle {
    fn default() -> Self {
        Self {
            background_color: Vec4::ZERO, // Transparent
            text_color: Vec4::new(0.0, 0.0, 0.0, 1.0), // Black
            border_color: Vec4::ZERO, // Transparent
            border_width: 0.0,
            border_radius: 0.0,
        }
    }
}

#[derive(Clone)] // Add Clone here for easier use later
pub struct StyleComputer {
    elements: HashMap<ElementId, Element>,
    styles: HashMap<u8, Style>,
    cache: RefCell<HashMap<ElementId, ComputedStyle>>,
}

impl StyleComputer {
    pub fn new(elements: &HashMap<ElementId, Element>, styles: &HashMap<u8, Style>) -> Self {
        Self {
            elements: elements.clone(),
            styles: styles.clone(),
            cache: RefCell::new(HashMap::new()),
        }
    }
    /// Computes the final style for a given element, using caching for performance.
    pub fn compute(&self, element_id: ElementId) -> ComputedStyle {
        // If style is already computed, return it from cache.
        if let Some(cached_style) = self.cache.borrow().get(&element_id) {
            return *cached_style;
        }

        let element = self.elements.get(&element_id)
            .expect("Element ID must exist");

        eprintln!("[StyleComputer] Computing style for element {}: style_id={}", element_id, element.style_id);

        // STEP 1: Get Parent's Computed Style (Inheritance)
        // If the element has a parent, compute its style first and use it as the base.
        // Only inherit text color, not borders (which are component-specific)
        let mut computed_style = if let Some(parent_id) = element.parent {
            let parent_style = self.compute(parent_id);
            ComputedStyle {
                text_color: parent_style.text_color, // Inherit text color
                background_color: Vec4::ZERO, // Don't inherit background
                border_color: Vec4::ZERO, // Don't inherit border
                border_width: 0.0, // Don't inherit border
                border_radius: 0.0, // Don't inherit border
            }
        } else {
            ComputedStyle::default()
        };

        eprintln!("[StyleComputer]   Base style: bg={:?}, text={:?}, border={:?}", 
            computed_style.background_color, computed_style.text_color, computed_style.border_color);

        // STEP 2: Apply Its Own Style Block
        if element.style_id > 0 {
            if let Some(style_block) = self.styles.get(&element.style_id) {
                eprintln!("[StyleComputer]   Found style block '{}' with {} properties", style_block.name, style_block.properties.len());
                // Apply all properties from the referenced style block
                for (prop_id, prop_value) in &style_block.properties {
                    match *prop_id {
                        0x01 => if let Some(c) = prop_value.as_color() { 
                            computed_style.background_color = c; 
                            eprintln!("[StyleComputer]     Applied background_color: {:?}", c);
                        }
                        0x02 => if let Some(c) = prop_value.as_color() { 
                            computed_style.text_color = c; 
                            eprintln!("[StyleComputer]     Applied text_color: {:?}", c);
                        }
                        0x03 => if let Some(c) = prop_value.as_color() { 
                            computed_style.border_color = c; 
                            eprintln!("[StyleComputer]     Applied border_color: {:?}", c);
                        }
                        0x04 => if let Some(f) = prop_value.as_float() { 
                            computed_style.border_width = f; 
                            eprintln!("[StyleComputer]     Applied border_width: {}", f);
                        }
                        0x05 => if let Some(f) = prop_value.as_float() { 
                            computed_style.border_radius = f; 
                            eprintln!("[StyleComputer]     Applied border_radius: {}", f);
                        }
                        _ => { 
                            eprintln!("[StyleComputer]     Ignored property 0x{:02X}", prop_id);
                        }
                    }
                }
            } else {
                eprintln!("[StyleComputer]   Style block {} not found!", element.style_id);
            }
        }
        
        // STEP 3: Apply Inline Properties (These are already on the Element struct from parsing)
        // This is for properties defined directly on the element, not in a style block.
        // The parser places these values directly on the Element struct. We check if they are
        // non-default, which signifies an inline override.
        if element.background_color != Vec4::ZERO { computed_style.background_color = element.background_color; }
        if element.text_color != Vec4::new(0.0, 0.0, 0.0, 1.0) { computed_style.text_color = element.text_color; }
        if element.border_color != Vec4::ZERO { computed_style.border_color = element.border_color; }
        if element.border_width != 0.0 { computed_style.border_width = element.border_width; }
        if element.border_radius != 0.0 { computed_style.border_radius = element.border_radius; }

        // Store the final computed style in the cache and return it.
        self.cache.borrow_mut().insert(element_id, computed_style);
        computed_style
    }
    
    /// Get an element by ID
    pub fn get_element(&self, element_id: ElementId) -> Option<&Element> {
        self.elements.get(&element_id)
    }
}
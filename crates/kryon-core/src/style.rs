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
    cache: RefCell<HashMap<(ElementId, crate::InteractionState), ComputedStyle>>,
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
        self.compute_with_state(element_id, crate::InteractionState::Normal)
    }
    
    /// Computes the final style for a given element in a specific interaction state.
    pub fn compute_with_state(&self, element_id: ElementId, state: crate::InteractionState) -> ComputedStyle {
        // If style is already computed for this element+state combination, return it from cache.
        let cache_key = (element_id, state);
        if let Some(cached_style) = self.cache.borrow().get(&cache_key) {
            return *cached_style;
        }

        let element = self.elements.get(&element_id)
            .expect("Element ID must exist");

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


        // STEP 2: Apply Its Own Style Block
        if element.style_id > 0 {
            if let Some(style_block) = self.styles.get(&element.style_id) {
                // Apply all properties from the referenced style block
                for (prop_id, prop_value) in &style_block.properties {
                    match *prop_id {
                        0x01 => if let Some(c) = prop_value.as_color() { 
                            computed_style.background_color = c; 
                        }
                        0x02 => if let Some(c) = prop_value.as_color() { 
                            computed_style.text_color = c; 
                        }
                        0x03 => if let Some(c) = prop_value.as_color() { 
                            computed_style.border_color = c; 
                        }
                        0x04 => if let Some(f) = prop_value.as_float() { 
                            computed_style.border_width = f; 
                        }
                        0x05 => if let Some(f) = prop_value.as_float() { 
                            computed_style.border_radius = f; 
                        }
                        _ => {}
                    }
                }
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

        // STEP 4: Auto-apply border width when border color is set but width is not
        if computed_style.border_color.w > 0.0 && computed_style.border_width == 0.0 {
            computed_style.border_width = 1.0;
        }

        // STEP 5: Apply intelligent default interaction effects for buttons
        if element.element_type == crate::ElementType::Button {
            computed_style = Self::apply_button_interaction_defaults(computed_style, state);
        }

        // Store the final computed style in the cache and return it.
        self.cache.borrow_mut().insert(cache_key, computed_style);
        computed_style
    }
    
    /// Get an element by ID
    pub fn get_element(&self, element_id: ElementId) -> Option<&Element> {
        self.elements.get(&element_id)
    }
    
    /// Lightens a color by a given factor (0.0 = no change, 1.0 = white)
    fn lighten_color(color: Vec4, factor: f32) -> Vec4 {
        Vec4::new(
            (color.x + (1.0 - color.x) * factor).min(1.0),
            (color.y + (1.0 - color.y) * factor).min(1.0),
            (color.z + (1.0 - color.z) * factor).min(1.0),
            color.w, // Keep alpha unchanged
        )
    }
    
    /// Darkens a color by a given factor (0.0 = no change, 1.0 = black)
    fn darken_color(color: Vec4, factor: f32) -> Vec4 {
        Vec4::new(
            (color.x * (1.0 - factor)).max(0.0),
            (color.y * (1.0 - factor)).max(0.0),
            (color.z * (1.0 - factor)).max(0.0),
            color.w, // Keep alpha unchanged
        )
    }
    
    /// Intelligently applies default interaction states for buttons based on their base color
    /// Only applies if explicit state styles are not defined (future-proof)
    fn apply_button_interaction_defaults(mut style: ComputedStyle, state: crate::InteractionState) -> ComputedStyle {
        // Only apply defaults if the button has a visible background
        if style.background_color.w <= 0.0 {
            return style;
        }
        
        match state {
            crate::InteractionState::Normal => {
                // Apply intelligent defaults for normal state
                Self::apply_button_normal_defaults(&mut style);
            }
            crate::InteractionState::Hover => {
                // Apply intelligent hover effects based on button color
                Self::apply_button_hover_defaults(&mut style);
            }
            crate::InteractionState::Active => {
                // Apply intelligent pressed/active effects
                Self::apply_button_active_defaults(&mut style);
            }
            crate::InteractionState::Focus => {
                // Apply intelligent focus effects
                Self::apply_button_focus_defaults(&mut style);
            }
            _ => {} // Disabled, Checked, etc. - no defaults for now
        }
        
        style
    }
    
    /// Apply intelligent defaults for normal button state
    fn apply_button_normal_defaults(style: &mut ComputedStyle) {
        // For normal state, ensure button has a good default appearance
        
        // If no border is set, add a subtle border that complements the background
        if style.border_color.w <= 0.0 && style.border_width <= 0.0 {
            // Create a border color that's slightly darker than the background
            style.border_color = Self::darken_color(style.background_color, 0.2);
            style.border_width = 1.0;
        }
        
        // Ensure text contrast - if text color is too similar to background, adjust it
        if Self::color_contrast(style.text_color, style.background_color) < 3.0 {
            // Choose black or white text based on background brightness
            let brightness = Self::color_brightness(style.background_color);
            style.text_color = if brightness > 0.5 {
                Vec4::new(0.0, 0.0, 0.0, 1.0) // Black text for light backgrounds
            } else {
                Vec4::new(1.0, 1.0, 1.0, 1.0) // White text for dark backgrounds
            };
        }
    }
    
    /// Apply intelligent defaults for button hover state
    fn apply_button_hover_defaults(style: &mut ComputedStyle) {
        // Lighten the background color for hover effect
        style.background_color = Self::lighten_color(style.background_color, 0.15);
        
        // Make border slightly more prominent
        if style.border_color.w > 0.0 {
            style.border_color = Self::lighten_color(style.border_color, 0.1);
            style.border_width = (style.border_width * 1.2).min(3.0); // Slightly thicker, max 3px
        }
    }
    
    /// Apply intelligent defaults for button active/pressed state  
    fn apply_button_active_defaults(style: &mut ComputedStyle) {
        // Darken the background color for pressed effect
        style.background_color = Self::darken_color(style.background_color, 0.2);
        
        // Make border darker and thicker for pressed feeling
        if style.border_color.w > 0.0 {
            style.border_color = Self::darken_color(style.border_color, 0.3);
            style.border_width = (style.border_width * 1.5).min(4.0); // Thicker border
        }
    }
    
    /// Apply intelligent defaults for button focus state
    fn apply_button_focus_defaults(style: &mut ComputedStyle) {
        // Add a subtle glow effect by adding a blue-tinted border
        if style.border_color.w <= 0.0 {
            // Add a blue focus border if no border exists
            style.border_color = Vec4::new(0.2, 0.6, 1.0, 1.0); // Light blue
            style.border_width = 2.0;
        } else {
            // Enhance existing border with blue tint
            let blue_tint = Vec4::new(0.2, 0.6, 1.0, 1.0);
            style.border_color = Self::blend_colors(style.border_color, blue_tint, 0.5);
            style.border_width = (style.border_width * 1.3).min(3.0);
        }
    }
    
    /// Calculate the brightness of a color (0.0 = black, 1.0 = white)
    fn color_brightness(color: Vec4) -> f32 {
        // Using perceived brightness formula
        0.299 * color.x + 0.587 * color.y + 0.114 * color.z
    }
    
    /// Calculate contrast ratio between two colors (simplified)
    fn color_contrast(color1: Vec4, color2: Vec4) -> f32 {
        let brightness1 = Self::color_brightness(color1);
        let brightness2 = Self::color_brightness(color2);
        (brightness1 - brightness2).abs() * 10.0 // Simplified contrast ratio
    }
    
    /// Blend two colors with a given factor (0.0 = first color, 1.0 = second color)
    fn blend_colors(color1: Vec4, color2: Vec4, factor: f32) -> Vec4 {
        Vec4::new(
            color1.x * (1.0 - factor) + color2.x * factor,
            color1.y * (1.0 - factor) + color2.y * factor,
            color1.z * (1.0 - factor) + color2.z * factor,
            color1.w * (1.0 - factor) + color2.w * factor,
        )
    }
}
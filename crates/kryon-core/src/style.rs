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
    // Non-inheritable visual properties
    pub background_color: Vec4,
    pub border_color: Vec4,
    pub border_width: f32,
    pub border_radius: f32,
    
    // Inheritable text properties
    pub text_color: Vec4,
    pub font_size: f32,
    pub font_weight: crate::FontWeight,
    pub text_alignment: crate::TextAlignment,
    
    // Inheritable display properties  
    pub opacity: f32,
    pub visible: bool,
    pub cursor: crate::CursorType,
}

impl Default for ComputedStyle {
    fn default() -> Self {
        Self {
            // Non-inheritable visual properties
            background_color: Vec4::ZERO, // Transparent
            border_color: Vec4::ZERO, // Transparent
            border_width: 0.0,
            border_radius: 0.0,
            
            // Inheritable text properties
            text_color: Vec4::new(0.0, 0.0, 0.0, 1.0), // Black
            font_size: 14.0, // Default font size
            font_weight: crate::FontWeight::Normal,
            text_alignment: crate::TextAlignment::Start,
            
            // Inheritable display properties
            opacity: 1.0, // Fully opaque
            visible: true, // Visible by default
            cursor: crate::CursorType::Default,
        }
    }
}

#[derive(Clone)] // Add Clone here for easier use later
pub struct StyleComputer {
    elements: HashMap<ElementId, Element>,
    styles: HashMap<u8, Style>,
    cache: RefCell<HashMap<(ElementId, crate::InteractionState), ComputedStyle>>,
    property_registry: crate::PropertyRegistry,
}

impl StyleComputer {
    pub fn new(elements: &HashMap<ElementId, Element>, styles: &HashMap<u8, Style>) -> Self {
        Self {
            elements: elements.clone(),
            styles: styles.clone(),
            cache: RefCell::new(HashMap::new()),
            property_registry: crate::PropertyRegistry::new(),
        }
    }
    
    /// Determines if a property should be inherited from parent to child
    /// Now uses the unified PropertyRegistry instead of hardcoded match
    fn is_property_inheritable(&self, property_id: u8) -> bool {
        let property_enum = crate::PropertyId::from(property_id);
        self.property_registry.is_inheritable(property_enum)
    }
    
    /// Apply a property value to computed style using the PropertyRegistry
    fn apply_property_to_computed_style(
        &self, 
        computed_style: &mut ComputedStyle, 
        property_id: u8, 
        prop_value: &PropertyValue,
        state: crate::InteractionState
    ) {
        let property_enum = crate::PropertyId::from(property_id);
        
        match property_enum {
            crate::PropertyId::BackgroundColor => {
                if state != crate::InteractionState::Checked {
                    if let Some(c) = prop_value.as_color() { 
                        computed_style.background_color = c; 
                    }
                }
            }
            crate::PropertyId::TextColor => {
                if state != crate::InteractionState::Checked {
                    if let Some(c) = prop_value.as_color() { 
                        computed_style.text_color = c; 
                    }
                }
            }
            crate::PropertyId::BorderColor => {
                if let Some(c) = prop_value.as_color() { 
                    computed_style.border_color = c; 
                }
            }
            crate::PropertyId::BorderWidth => {
                if let Some(f) = prop_value.as_float() { 
                    computed_style.border_width = f; 
                }
            }
            crate::PropertyId::BorderRadius => {
                if let Some(f) = prop_value.as_float() { 
                    computed_style.border_radius = f; 
                }
            }
            crate::PropertyId::FontSize => {
                if let Some(f) = prop_value.as_float() { 
                    computed_style.font_size = f; 
                }
            }
            crate::PropertyId::FontWeight => {
                if let Some(i) = prop_value.as_int() {
                    computed_style.font_weight = match i {
                        300 => crate::FontWeight::Light,
                        400 => crate::FontWeight::Normal,
                        700 => crate::FontWeight::Bold,
                        900 => crate::FontWeight::Heavy,
                        _ => crate::FontWeight::Normal,
                    };
                }
            }
            crate::PropertyId::TextAlignment => {
                if let Some(s) = prop_value.as_string() {
                    computed_style.text_alignment = match s {
                        "start" => crate::TextAlignment::Start,
                        "center" => crate::TextAlignment::Center,
                        "end" => crate::TextAlignment::End,
                        "justify" => crate::TextAlignment::Justify,
                        _ => crate::TextAlignment::Start,
                    };
                }
            }
            crate::PropertyId::Opacity => {
                if let Some(f) = prop_value.as_float() {
                    computed_style.opacity = f.clamp(0.0, 1.0); 
                }
            }
            crate::PropertyId::Visibility => {
                if let Some(b) = prop_value.as_bool() {
                    computed_style.visible = b;
                }
            }
            crate::PropertyId::Cursor => {
                if let Some(s) = prop_value.as_string() {
                    computed_style.cursor = match s {
                        "default" => crate::CursorType::Default,
                        "pointer" => crate::CursorType::Pointer,
                        "text" => crate::CursorType::Text,
                        "move" => crate::CursorType::Move,
                        "not-allowed" => crate::CursorType::NotAllowed,
                        _ => crate::CursorType::Default,
                    };
                }
            }
            // Properties that need different handling (layout properties)
            crate::PropertyId::Width | crate::PropertyId::Height | crate::PropertyId::OldLayoutFlags => {
                // These properties need to be applied to the element directly, not computed style
                // For now, skip them - they'll be handled by the element update system
            }
            _ => {
                // Unknown or unsupported property - could log a warning here
            }
        }
    }
    /// Computes the final style for a given element, using caching for performance.
    pub fn compute(&self, element_id: ElementId) -> ComputedStyle {
        self.compute_with_state(element_id, crate::InteractionState::Normal)
    }
    
    /// Computes the final style for a given element in a specific interaction state.
    pub fn compute_with_state(&self, element_id: ElementId, state: crate::InteractionState) -> ComputedStyle {
        // Temporarily disable cache to debug state changes
        let cache_key = (element_id, state);
        // if let Some(cached_style) = self.cache.borrow().get(&cache_key) {
        //     return *cached_style;
        // }

        let element = self.elements.get(&element_id)
            .expect("Element ID must exist");

        // STEP 1: Get Parent's Computed Style (Inheritance)
        // If the element has a parent, compute its style first and inherit inheritable properties
        let mut computed_style = if let Some(parent_id) = element.parent {
            let parent_style = self.compute(parent_id);
            ComputedStyle {
                // Non-inheritable properties - always reset to defaults
                background_color: Vec4::ZERO,
                border_color: Vec4::ZERO,
                border_width: 0.0,
                border_radius: 0.0,
                
                // Inheritable properties - inherit from parent
                text_color: parent_style.text_color,
                font_size: parent_style.font_size,
                font_weight: parent_style.font_weight,
                text_alignment: parent_style.text_alignment,
                opacity: parent_style.opacity,
                visible: parent_style.visible,
                cursor: parent_style.cursor,
            }
        } else {
            ComputedStyle::default()
        };


        // STEP 2: Apply Its Own Style Block (but only for non-interactive states)
        if element.style_id > 0 {
            if let Some(style_block) = self.styles.get(&element.style_id) {
                // Apply all properties from the referenced style block using PropertyRegistry
                for (prop_id, prop_value) in &style_block.properties {
                    self.apply_property_to_computed_style(&mut computed_style, *prop_id, prop_value, state);
                }
            }
        }
        
        // STEP 3: Apply Inline Properties (These are already on the Element struct from parsing)
        // This is for properties defined directly on the element, not in a style block.
        // The parser places these values directly on the Element struct. We check if they are
        // non-default, which signifies an inline override.
        
        // Non-inheritable visual properties
        if element.background_color != Vec4::ZERO { computed_style.background_color = element.background_color; }
        if element.border_color != Vec4::ZERO { computed_style.border_color = element.border_color; }
        if element.border_width != 0.0 { computed_style.border_width = element.border_width; }
        if element.border_radius != 0.0 { computed_style.border_radius = element.border_radius; }
        
        // Inheritable text properties
        if element.text_color != Vec4::new(0.0, 0.0, 0.0, 1.0) { computed_style.text_color = element.text_color; }
        if element.font_size != 14.0 { computed_style.font_size = element.font_size; }
        if element.font_weight != crate::FontWeight::Normal { computed_style.font_weight = element.font_weight; }
        if element.text_alignment != crate::TextAlignment::Start { computed_style.text_alignment = element.text_alignment; }
        
        // Inheritable display properties
        if element.opacity != 1.0 { computed_style.opacity = element.opacity; }
        if !element.visible { computed_style.visible = element.visible; }
        if element.cursor != crate::CursorType::Default { computed_style.cursor = element.cursor; }

        // STEP 4: Auto-apply border width when border color is set but width is not
        if computed_style.border_color.w > 0.0 && computed_style.border_width == 0.0 {
            computed_style.border_width = 1.0;
        }

        // STEP 5: Apply intelligent default interaction effects for buttons
        if element.element_type == crate::ElementType::Button {
            eprintln!("[STYLE_DEBUG] Button element {}: state={:?}, bg_before={:?}", 
                     element_id, state, computed_style.background_color);
            computed_style = Self::apply_button_interaction_defaults(computed_style, state);
            eprintln!("[STYLE_DEBUG] Button element {}: bg_after={:?}", 
                     element_id, computed_style.background_color);
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
        // For checked state, always apply styling even if background is transparent
        // For other states, only apply defaults if the button has a visible background
        if style.background_color.w <= 0.0 && state != crate::InteractionState::Checked {
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
            crate::InteractionState::Checked => {
                // Apply intelligent checked/selected effects
                Self::apply_button_checked_defaults(&mut style);
            }
            _ => {} // Disabled, etc. - no defaults for now
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
    
    /// Apply intelligent defaults for button checked state (e.g., for tab buttons)
    fn apply_button_checked_defaults(style: &mut ComputedStyle) {
        eprintln!("[CHECKED_STYLE] Applying checked defaults: bg_before={:?}", style.background_color);
        
        // For checked/selected buttons (like active tabs), use a distinctive color
        // This provides visual feedback for the "selected" state
        
        // Apply a distinct checked background - typically a more saturated/bright color
        // Using a blue tone to indicate "selected/active" state (like browser tabs)
        style.background_color = Vec4::new(0.0, 0.4, 0.8, 1.0); // Blue (#0066CC)
        
        // Ensure good text contrast for checked state
        style.text_color = Vec4::new(1.0, 1.0, 1.0, 1.0); // White text for good contrast
        
        // Add a subtle border to emphasize the selected state
        if style.border_color.w <= 0.0 {
            style.border_color = Vec4::new(0.0, 0.3, 0.6, 1.0); // Darker blue border
            style.border_width = 1.0;
        }
        
        eprintln!("[CHECKED_STYLE] Applied checked defaults: bg_after={:?}", style.background_color);
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
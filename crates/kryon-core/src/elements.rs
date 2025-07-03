// crates/kryon-core/src/elements.rs
use glam::{Vec2, Vec4};
use std::collections::HashMap;
use crate::PropertyValue;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ElementType {
    App = 0x00,
    Container = 0x01,
    Text = 0x02,
    Image = 0x03,
    Button = 0x10,
    Input = 0x11,
    Custom(u8),
}

impl From<u8> for ElementType {
    fn from(value: u8) -> Self {
        match value {
            0x00 => ElementType::App,
            0x01 => ElementType::Container,
            0x02 => ElementType::Text,
            0x03 => ElementType::Image,
            0x10 => ElementType::Button,
            0x11 => ElementType::Input,
            other => ElementType::Custom(other),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InteractionState {
    Normal = 0,
    Hover = 1,
    Active = 2,
    Focus = 4,
    Disabled = 8,
    Checked = 16,
}

#[derive(Debug, Clone)]
pub struct Element {
    pub id: String,
    pub element_type: ElementType,
    pub parent: Option<ElementId>,
    pub children: Vec<ElementId>,
    
    pub style_id: u8,

    // Layout properties
    pub position: Vec2,
    pub size: Vec2,
    pub layout_flags: u8,
    
    // Visual properties
    pub background_color: Vec4,
    pub text_color: Vec4,
    pub border_color: Vec4,
    pub border_width: f32,
    pub border_radius: f32,
    pub opacity: f32,
    pub visible: bool,
    
    // Text properties
    pub text: String,
    pub font_size: f32,
    pub font_weight: FontWeight,
    pub text_alignment: TextAlignment,
    
    // Interactive properties
    pub cursor: CursorType,
    pub disabled: bool,
    pub current_state: InteractionState,
    
    // Custom properties (for components)
    pub custom_properties: HashMap<String, PropertyValue>,
    
    // State-based properties
    pub state_properties: HashMap<InteractionState, HashMap<String, PropertyValue>>,
    
    // Event handlers
    pub event_handlers: HashMap<EventType, String>,
    
    // Component-specific
    pub component_name: Option<String>,
    pub is_component_instance: bool,
}

pub type ElementId = u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontWeight {
    Normal = 400,
    Bold = 700,
    Light = 300,
    Heavy = 900,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAlignment {
    Start,
    Center,
    End,
    Justify,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorType {
    Default,
    Pointer,
    Text,
    Move,
    NotAllowed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EventType {
    Click,
    Hover,
    Focus,
    Blur,
    Change,
    Submit,
}

impl Default for Element {
    fn default() -> Self {
        Self {
            id: String::new(),
            element_type: ElementType::Container,
            parent: None,
            children: Vec::new(),
            style_id: 0, 
            position: Vec2::ZERO,
            size: Vec2::ZERO,
            layout_flags: 0,
            background_color: Vec4::new(0.0, 0.0, 0.0, 0.0), // Transparent
            text_color: Vec4::new(0.0, 0.0, 0.0, 1.0), // Black
            border_color: Vec4::new(0.0, 0.0, 0.0, 0.0), // Transparent
            border_width: 0.0,
            border_radius: 0.0,
            opacity: 1.0,
            visible: true,
            text: String::new(),
            font_size: 14.0,
            font_weight: FontWeight::Normal,
            text_alignment: TextAlignment::Start,
            cursor: CursorType::Default,
            disabled: false,
            current_state: InteractionState::Normal,
            custom_properties: HashMap::new(),
            state_properties: HashMap::new(),
            event_handlers: HashMap::new(),
            component_name: None,
            is_component_instance: false,
        }
    }
}
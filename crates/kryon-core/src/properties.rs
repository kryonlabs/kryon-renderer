// crates/kryon-core/src/properties.rs
use glam::Vec4;

#[derive(Debug, Clone)]
pub enum PropertyValue {
    String(String),
    Int(i32),
    Float(f32),
    Bool(bool),
    Color(Vec4),
    Resource(String),
}

impl PropertyValue {
    pub fn as_string(&self) -> Option<&str> {
        match self {
            PropertyValue::String(s) => Some(s),
            _ => None,
        }
    }
    
    pub fn as_int(&self) -> Option<i32> {
        match self {
            PropertyValue::Int(i) => Some(*i),
            _ => None,
        }
    }
    
    pub fn as_float(&self) -> Option<f32> {
        match self {
            PropertyValue::Float(f) => Some(*f),
            PropertyValue::Int(i) => Some(*i as f32),
            _ => None,
        }
    }
    
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            PropertyValue::Bool(b) => Some(*b),
            _ => None,
        }
    }
    
    pub fn as_color(&self) -> Option<Vec4> {
        match self {
            PropertyValue::Color(c) => Some(*c),
            _ => None,
        }
    }
}
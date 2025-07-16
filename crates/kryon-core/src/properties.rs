// crates/kryon-core/src/properties.rs
use glam::Vec4;

#[derive(Debug, Clone)]
pub enum PropertyValue {
    String(String),
    Int(i32),
    Float(f32),
    Percentage(f32), // 0-100 range, e.g., 50.0 for 50%
    Bool(bool),
    Color(Vec4),
    Resource(String),
    Transform(TransformData),
    CSSUnit(CSSUnitValue),
    RichText(crate::text::RichText),
}

#[derive(Debug, Clone)]
pub struct TransformData {
    pub transform_type: TransformType,
    pub properties: Vec<TransformProperty>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransformType {
    Transform2D = 0x01,
    Transform3D = 0x02,
    Matrix2D = 0x03,
    Matrix3D = 0x04,
}

#[derive(Debug, Clone)]
pub struct TransformProperty {
    pub property_type: TransformPropertyType,
    pub value: CSSUnitValue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransformPropertyType {
    // 2D Transform properties
    Scale = 0x01,
    ScaleX = 0x02,
    ScaleY = 0x03,
    TranslateX = 0x04,
    TranslateY = 0x05,
    Rotate = 0x06,
    SkewX = 0x07,
    SkewY = 0x08,
    
    // 3D Transform properties
    ScaleZ = 0x09,
    TranslateZ = 0x0A,
    RotateX = 0x0B,
    RotateY = 0x0C,
    RotateZ = 0x0D,
    Perspective = 0x0E,
    
    // Matrix properties
    Matrix = 0x0F,
}

#[derive(Debug, Clone)]
pub struct CSSUnitValue {
    pub value: f64,
    pub unit: CSSUnit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CSSUnit {
    // Size units
    Pixels = 0x01,
    Em = 0x02,
    Rem = 0x03,
    ViewportWidth = 0x04,
    ViewportHeight = 0x05,
    Percentage = 0x06,
    
    // Angle units
    Degrees = 0x07,
    Radians = 0x08,
    Turns = 0x09,
    
    // Unitless (for scale, matrix values)
    Number = 0x0A,
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
    
    pub fn as_percentage(&self) -> Option<f32> {
        match self {
            PropertyValue::Percentage(p) => Some(*p),
            _ => None,
        }
    }
    
    /// Convert percentage to pixel value relative to a parent size
    pub fn as_pixels(&self, parent_size: f32) -> Option<f32> {
        match self {
            PropertyValue::Float(f) => Some(*f),
            PropertyValue::Int(i) => Some(*i as f32),
            PropertyValue::Percentage(p) => Some((p / 100.0) * parent_size),
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
    
    pub fn as_transform(&self) -> Option<&TransformData> {
        match self {
            PropertyValue::Transform(t) => Some(t),
            _ => None,
        }
    }
    
    pub fn as_css_unit(&self) -> Option<&CSSUnitValue> {
        match self {
            PropertyValue::CSSUnit(u) => Some(u),
            _ => None,
        }
    }
    
    pub fn as_rich_text(&self) -> Option<&crate::text::RichText> {
        match self {
            PropertyValue::RichText(rt) => Some(rt),
            _ => None,
        }
    }
}
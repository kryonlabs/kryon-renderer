// crates/kryon-layout/src/flexbox.rs
use kryon_core::ElementId;

#[derive(Debug, Clone)]
pub struct FlexItem {
    pub element_id: ElementId,
    pub flex_basis: f32,
    pub flex_grow: f32,
    pub flex_shrink: f32,
    pub main_axis_size: f32,
    pub cross_axis_size: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct LayoutFlags {
    pub direction: LayoutDirection,
    pub alignment: LayoutAlignment,
    pub wrap: bool,
    pub grow: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutDirection {
    Row,
    Column,
    Absolute,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutAlignment {
    Start,
    Center,
    End,
    SpaceBetween,
}

impl LayoutFlags {
    pub fn from_bits(bits: u8) -> Self {
        let direction = match bits & 0x03 {
            0x00 => LayoutDirection::Row,
            0x01 => LayoutDirection::Column,
            0x02 => LayoutDirection::Absolute,
            _ => LayoutDirection::Column,
        };
        
        let alignment = match (bits >> 2) & 0x03 {
            0x00 => LayoutAlignment::Start,
            0x01 => LayoutAlignment::Center,
            0x02 => LayoutAlignment::End,
            0x03 => LayoutAlignment::SpaceBetween,
            _ => LayoutAlignment::Start,
        };
        
        let wrap = (bits & 0x10) != 0;
        let grow = (bits & 0x20) != 0;
        
        Self {
            direction,
            alignment,
            wrap,
            grow,
        }
    }
}
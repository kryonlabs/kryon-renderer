// crates/kryon-layout/src/flexbox.rs
// FlexItem struct removed - was only used by legacy FlexboxLayoutEngine

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
// crates/kryon-core/src/property_registry.rs

use glam::Vec4;
use crate::PropertyValue;

/// Unified property registry that consolidates all property handling
/// Replaces the 4 separate property mapping systems:
/// 1. KRB Parser Mapping (krb.rs:669-1212)
/// 2. Style Computer Mapping (style.rs:152-228)
/// 3. Style Inheritance Mapping (style.rs:78-104)
/// 4. Legacy Layout Flags (flexbox.rs:38-64)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum PropertyId {
    // Visual Properties (0x01-0x0F)
    BackgroundColor = 0x01,
    TextColor = 0x02,
    BorderColor = 0x03,
    BorderWidth = 0x04,
    BorderRadius = 0x05,
    LayoutFlags = 0x06,
    
    // Text Properties (0x08-0x0F)
    TextContent = 0x08,
    FontSize = 0x09,
    FontWeight = 0x0A,
    TextAlignment = 0x0B,
    FontFamily = 0x0C,
    ImageSource = 0x0D,
    Opacity = 0x0E,
    ZIndex = 0x0F,
    ListStyleType = 0x1E,
    WhiteSpace = 0x1F,
    
    // Display Properties (0x10-0x1F)
    Visibility = 0x10,
    Gap = 0x11,
    MinWidth = 0x12,
    MinHeight = 0x13,
    MaxWidth = 0x14,
    MaxHeight = 0x15,
    Transform = 0x16,
    // Reserved for future use = 0x17,
    Shadow = 0x18,
    
    // Layout Properties (0x19-0x1F)
    Width = 0x19,
    Height = 0x1A,
    OldLayoutFlags = 0x1B, // Legacy layout flags
    
    // Window Properties (0x20-0x2F)
    WindowWidth = 0x20,
    WindowHeight = 0x21,
    WindowTitle = 0x22,
    WindowResizable = 0x23,
    WindowFullscreen = 0x24,
    WindowVsync = 0x25,
    WindowTargetFps = 0x26,
    WindowAntialiasing = 0x27,
    WindowIcon = 0x28,
    Cursor = 0x29,
    
    // Flexbox Properties (0x40-0x4F)
    Display = 0x40,
    FlexDirection = 0x41,
    FlexWrap = 0x42,
    FlexGrow = 0x43,
    FlexShrink = 0x44,
    FlexBasis = 0x45,
    AlignItems = 0x46,
    AlignContent = 0x47,
    AlignSelf = 0x48,
    JustifyContent = 0x49,
    JustifyItems = 0x4A,
    JustifySelf = 0x4B,
    
    // Position Properties (0x50-0x5F)
    Position = 0x50,
    Left = 0x51,
    Top = 0x52,
    Right = 0x53,
    Bottom = 0x54,
    
    // CSS Grid Properties (0x60-0x6F)
    GridTemplateColumns = 0x60,
    GridTemplateRows = 0x61,
    GridTemplateAreas = 0x62,
    GridAutoColumns = 0x63,
    GridAutoRows = 0x64,
    GridAutoFlow = 0x65,
    GridArea = 0x66,
    GridColumn = 0x67,
    GridRow = 0x68,
    GridColumnStart = 0x69,
    GridColumnEnd = 0x6A,
    GridRowStart = 0x6B,
    GridRowEnd = 0x6C,
    GridGap = 0x6D,
    GridColumnGap = 0x6E,
    GridRowGap = 0x6F,
    
    // Box Model Properties (0x70-0x8F)
    // Padding properties
    Padding = 0x70,
    PaddingTop = 0x71,
    PaddingRight = 0x72,
    PaddingBottom = 0x73,
    PaddingLeft = 0x74,
    
    // Margin properties
    Margin = 0x75,
    MarginTop = 0x76,
    MarginRight = 0x77,
    MarginBottom = 0x78,
    MarginLeft = 0x79,
    
    // Border width properties (individual sides)
    BorderTopWidth = 0x7A,
    BorderRightWidth = 0x7B,
    BorderBottomWidth = 0x7C,
    BorderLeftWidth = 0x7D,
    
    // Border color properties (individual sides)
    BorderTopColor = 0x7E,
    BorderRightColor = 0x7F,
    BorderBottomColor = 0x80,
    BorderLeftColor = 0x81,
    
    // Border radius properties (individual corners)
    BorderTopLeftRadius = 0x82,
    BorderTopRightRadius = 0x83,
    BorderBottomRightRadius = 0x84,
    BorderBottomLeftRadius = 0x85,
    
    // Box sizing and outline
    BoxSizing = 0x86,
    Outline = 0x87,
    OutlineColor = 0x88,
    OutlineWidth = 0x89,
    OutlineOffset = 0x8A,
    
    // Overflow properties
    Overflow = 0x8B,
    OverflowX = 0x8C,
    OverflowY = 0x8D,
    
    // Reserved for custom properties (0x90-0xFF)
    Custom(u8),
}

impl From<u8> for PropertyId {
    fn from(value: u8) -> Self {
        match value {
            0x01 => PropertyId::BackgroundColor,
            0x02 => PropertyId::TextColor,
            0x03 => PropertyId::BorderColor,
            0x04 => PropertyId::BorderWidth,
            0x05 => PropertyId::BorderRadius,
            0x06 => PropertyId::LayoutFlags,
            0x08 => PropertyId::TextContent,
            0x09 => PropertyId::FontSize,
            0x0A => PropertyId::FontWeight,
            0x0B => PropertyId::TextAlignment,
            0x0C => PropertyId::FontFamily,
            0x0D => PropertyId::ImageSource,
            0x0E => PropertyId::Opacity,
            0x0F => PropertyId::ZIndex,
            0x1E => PropertyId::ListStyleType,
            0x1F => PropertyId::WhiteSpace,
            0x10 => PropertyId::Visibility,
            0x11 => PropertyId::Gap,
            0x12 => PropertyId::MinWidth,
            0x13 => PropertyId::MinHeight,
            0x14 => PropertyId::MaxWidth,
            0x15 => PropertyId::MaxHeight,
            0x16 => PropertyId::Transform,
            0x18 => PropertyId::Shadow,
            0x19 => PropertyId::Width,
            0x1A => PropertyId::Height,
            0x1B => PropertyId::OldLayoutFlags,
            0x20 => PropertyId::WindowWidth,
            0x21 => PropertyId::WindowHeight,
            0x22 => PropertyId::WindowTitle,
            0x23 => PropertyId::WindowResizable,
            0x24 => PropertyId::WindowFullscreen,
            0x25 => PropertyId::WindowVsync,
            0x26 => PropertyId::WindowTargetFps,
            0x27 => PropertyId::WindowAntialiasing,
            0x28 => PropertyId::WindowIcon,
            0x29 => PropertyId::Cursor,
            0x40 => PropertyId::Display,
            0x41 => PropertyId::FlexDirection,
            0x42 => PropertyId::FlexWrap,
            0x43 => PropertyId::FlexGrow,
            0x44 => PropertyId::FlexShrink,
            0x45 => PropertyId::FlexBasis,
            0x46 => PropertyId::AlignItems,
            0x47 => PropertyId::AlignContent,
            0x48 => PropertyId::AlignSelf,
            0x49 => PropertyId::JustifyContent,
            0x4A => PropertyId::JustifyItems,
            0x4B => PropertyId::JustifySelf,
            0x50 => PropertyId::Position,
            0x51 => PropertyId::Left,
            0x52 => PropertyId::Top,
            0x53 => PropertyId::Right,
            0x54 => PropertyId::Bottom,
            0x60 => PropertyId::GridTemplateColumns,
            0x61 => PropertyId::GridTemplateRows,
            0x62 => PropertyId::GridTemplateAreas,
            0x63 => PropertyId::GridAutoColumns,
            0x64 => PropertyId::GridAutoRows,
            0x65 => PropertyId::GridAutoFlow,
            0x66 => PropertyId::GridArea,
            0x67 => PropertyId::GridColumn,
            0x68 => PropertyId::GridRow,
            0x69 => PropertyId::GridColumnStart,
            0x6A => PropertyId::GridColumnEnd,
            0x6B => PropertyId::GridRowStart,
            0x6C => PropertyId::GridRowEnd,
            0x6D => PropertyId::GridGap,
            0x6E => PropertyId::GridColumnGap,
            0x6F => PropertyId::GridRowGap,
            0x70 => PropertyId::Padding,
            0x71 => PropertyId::PaddingTop,
            0x72 => PropertyId::PaddingRight,
            0x73 => PropertyId::PaddingBottom,
            0x74 => PropertyId::PaddingLeft,
            0x75 => PropertyId::Margin,
            0x76 => PropertyId::MarginTop,
            0x77 => PropertyId::MarginRight,
            0x78 => PropertyId::MarginBottom,
            0x79 => PropertyId::MarginLeft,
            0x7A => PropertyId::BorderTopWidth,
            0x7B => PropertyId::BorderRightWidth,
            0x7C => PropertyId::BorderBottomWidth,
            0x7D => PropertyId::BorderLeftWidth,
            0x7E => PropertyId::BorderTopColor,
            0x7F => PropertyId::BorderRightColor,
            0x80 => PropertyId::BorderBottomColor,
            0x81 => PropertyId::BorderLeftColor,
            0x82 => PropertyId::BorderTopLeftRadius,
            0x83 => PropertyId::BorderTopRightRadius,
            0x84 => PropertyId::BorderBottomRightRadius,
            0x85 => PropertyId::BorderBottomLeftRadius,
            0x86 => PropertyId::BoxSizing,
            0x87 => PropertyId::Outline,
            0x88 => PropertyId::OutlineColor,
            0x89 => PropertyId::OutlineWidth,
            0x8A => PropertyId::OutlineOffset,
            0x8B => PropertyId::Overflow,
            0x8C => PropertyId::OverflowX,
            0x8D => PropertyId::OverflowY,
            other => PropertyId::Custom(other),
        }
    }
}

impl PropertyId {
    /// Get the u8 value for this property ID
    pub fn as_u8(self) -> u8 {
        match self {
            PropertyId::BackgroundColor => 0x01,
            PropertyId::TextColor => 0x02,
            PropertyId::BorderColor => 0x03,
            PropertyId::BorderWidth => 0x04,
            PropertyId::BorderRadius => 0x05,
            PropertyId::LayoutFlags => 0x06,
            PropertyId::TextContent => 0x08,
            PropertyId::FontSize => 0x09,
            PropertyId::FontWeight => 0x0A,
            PropertyId::TextAlignment => 0x0B,
            PropertyId::FontFamily => 0x0C,
            PropertyId::ImageSource => 0x0D,
            PropertyId::Opacity => 0x0E,
            PropertyId::ZIndex => 0x0F,
            PropertyId::ListStyleType => 0x1E,
            PropertyId::WhiteSpace => 0x1F,
            PropertyId::Visibility => 0x10,
            PropertyId::Gap => 0x11,
            PropertyId::MinWidth => 0x12,
            PropertyId::MinHeight => 0x13,
            PropertyId::MaxWidth => 0x14,
            PropertyId::MaxHeight => 0x15,
            PropertyId::Transform => 0x16,
            PropertyId::Shadow => 0x18,
            PropertyId::Width => 0x19,
            PropertyId::Height => 0x1A,
            PropertyId::OldLayoutFlags => 0x1B,
            PropertyId::WindowWidth => 0x20,
            PropertyId::WindowHeight => 0x21,
            PropertyId::WindowTitle => 0x22,
            PropertyId::WindowResizable => 0x23,
            PropertyId::WindowFullscreen => 0x24,
            PropertyId::WindowVsync => 0x25,
            PropertyId::WindowTargetFps => 0x26,
            PropertyId::WindowAntialiasing => 0x27,
            PropertyId::WindowIcon => 0x28,
            PropertyId::Cursor => 0x29,
            PropertyId::Display => 0x40,
            PropertyId::FlexDirection => 0x41,
            PropertyId::FlexWrap => 0x42,
            PropertyId::FlexGrow => 0x43,
            PropertyId::FlexShrink => 0x44,
            PropertyId::FlexBasis => 0x45,
            PropertyId::AlignItems => 0x46,
            PropertyId::AlignContent => 0x47,
            PropertyId::AlignSelf => 0x48,
            PropertyId::JustifyContent => 0x49,
            PropertyId::JustifyItems => 0x4A,
            PropertyId::JustifySelf => 0x4B,
            PropertyId::Position => 0x50,
            PropertyId::Left => 0x51,
            PropertyId::Top => 0x52,
            PropertyId::Right => 0x53,
            PropertyId::Bottom => 0x54,
            PropertyId::GridTemplateColumns => 0x60,
            PropertyId::GridTemplateRows => 0x61,
            PropertyId::GridTemplateAreas => 0x62,
            PropertyId::GridAutoColumns => 0x63,
            PropertyId::GridAutoRows => 0x64,
            PropertyId::GridAutoFlow => 0x65,
            PropertyId::GridArea => 0x66,
            PropertyId::GridColumn => 0x67,
            PropertyId::GridRow => 0x68,
            PropertyId::GridColumnStart => 0x69,
            PropertyId::GridColumnEnd => 0x6A,
            PropertyId::GridRowStart => 0x6B,
            PropertyId::GridRowEnd => 0x6C,
            PropertyId::GridGap => 0x6D,
            PropertyId::GridColumnGap => 0x6E,
            PropertyId::GridRowGap => 0x6F,
            PropertyId::Padding => 0x70,
            PropertyId::PaddingTop => 0x71,
            PropertyId::PaddingRight => 0x72,
            PropertyId::PaddingBottom => 0x73,
            PropertyId::PaddingLeft => 0x74,
            PropertyId::Margin => 0x75,
            PropertyId::MarginTop => 0x76,
            PropertyId::MarginRight => 0x77,
            PropertyId::MarginBottom => 0x78,
            PropertyId::MarginLeft => 0x79,
            PropertyId::BorderTopWidth => 0x7A,
            PropertyId::BorderRightWidth => 0x7B,
            PropertyId::BorderBottomWidth => 0x7C,
            PropertyId::BorderLeftWidth => 0x7D,
            PropertyId::BorderTopColor => 0x7E,
            PropertyId::BorderRightColor => 0x7F,
            PropertyId::BorderBottomColor => 0x80,
            PropertyId::BorderLeftColor => 0x81,
            PropertyId::BorderTopLeftRadius => 0x82,
            PropertyId::BorderTopRightRadius => 0x83,
            PropertyId::BorderBottomRightRadius => 0x84,
            PropertyId::BorderBottomLeftRadius => 0x85,
            PropertyId::BoxSizing => 0x86,
            PropertyId::Outline => 0x87,
            PropertyId::OutlineColor => 0x88,
            PropertyId::OutlineWidth => 0x89,
            PropertyId::OutlineOffset => 0x8A,
            PropertyId::Overflow => 0x8B,
            PropertyId::OverflowX => 0x8C,
            PropertyId::OverflowY => 0x8D,
            PropertyId::Custom(value) => value,
        }
    }
}

/// Metadata about a property including inheritance rules, default values, and validation
#[derive(Debug, Clone)]
pub struct PropertyMetadata {
    pub id: PropertyId,
    pub name: &'static str,
    pub inheritable: bool,
    pub default_value: PropertyValue,
    pub value_type: PropertyValueType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropertyValueType {
    Color,
    Float,
    Int,
    String,
    Bool,
    Percentage,
    Transform,
    CSSUnit,
}

/// Unified property registry that provides single source of truth for all property handling
#[derive(Clone)]
pub struct PropertyRegistry {
    properties: Vec<PropertyMetadata>,
    id_to_index: [Option<usize>; 256],
}

impl PropertyRegistry {
    pub fn new() -> Self {
        let mut registry = PropertyRegistry {
            properties: Vec::new(),
            id_to_index: [None; 256],
        };
        
        registry.initialize_properties();
        registry
    }
    
    fn initialize_properties(&mut self) {
        // Visual Properties
        self.register_property(PropertyMetadata {
            id: PropertyId::BackgroundColor,
            name: "background-color",
            inheritable: false,
            default_value: PropertyValue::Color(Vec4::ZERO),
            value_type: PropertyValueType::Color,
        });
        
        self.register_property(PropertyMetadata {
            id: PropertyId::TextColor,
            name: "color",
            inheritable: true,
            default_value: PropertyValue::Color(Vec4::new(0.0, 0.0, 0.0, 1.0)),
            value_type: PropertyValueType::Color,
        });
        
        self.register_property(PropertyMetadata {
            id: PropertyId::BorderColor,
            name: "border-color",
            inheritable: false,
            default_value: PropertyValue::Color(Vec4::ZERO),
            value_type: PropertyValueType::Color,
        });
        
        self.register_property(PropertyMetadata {
            id: PropertyId::BorderWidth,
            name: "border-width",
            inheritable: false,
            default_value: PropertyValue::Float(0.0),
            value_type: PropertyValueType::Float,
        });
        
        self.register_property(PropertyMetadata {
            id: PropertyId::BorderRadius,
            name: "border-radius",
            inheritable: false,
            default_value: PropertyValue::Float(0.0),
            value_type: PropertyValueType::Float,
        });
        
        // Text Properties
        self.register_property(PropertyMetadata {
            id: PropertyId::FontSize,
            name: "font-size",
            inheritable: true,
            default_value: PropertyValue::Float(14.0),
            value_type: PropertyValueType::Float,
        });
        
        self.register_property(PropertyMetadata {
            id: PropertyId::FontWeight,
            name: "font-weight",
            inheritable: true,
            default_value: PropertyValue::Int(400),
            value_type: PropertyValueType::Int,
        });
        
        self.register_property(PropertyMetadata {
            id: PropertyId::TextAlignment,
            name: "text-align",
            inheritable: true,
            default_value: PropertyValue::String("start".to_string()),
            value_type: PropertyValueType::String,
        });
        
        self.register_property(PropertyMetadata {
            id: PropertyId::ListStyleType,
            name: "list-style-type",
            inheritable: true,
            default_value: PropertyValue::Int(0), // 0 = none, 1 = bullet, 2 = number
            value_type: PropertyValueType::Int,
        });
        
        self.register_property(PropertyMetadata {
            id: PropertyId::WhiteSpace,
            name: "white-space",
            inheritable: true,
            default_value: PropertyValue::Int(0), // 0 = normal, 1 = nowrap, 2 = pre
            value_type: PropertyValueType::Int,
        });
        
        self.register_property(PropertyMetadata {
            id: PropertyId::FontFamily,
            name: "font-family",
            inheritable: true,
            default_value: PropertyValue::String("default".to_string()),
            value_type: PropertyValueType::String,
        });
        
        // Display Properties
        self.register_property(PropertyMetadata {
            id: PropertyId::Opacity,
            name: "opacity",
            inheritable: true,
            default_value: PropertyValue::Float(1.0),
            value_type: PropertyValueType::Float,
        });
        
        self.register_property(PropertyMetadata {
            id: PropertyId::ZIndex,
            name: "z-index",
            inheritable: false,
            default_value: PropertyValue::Int(0),
            value_type: PropertyValueType::Int,
        });
        
        // Alias for z-index
        self.register_property(PropertyMetadata {
            id: PropertyId::ZIndex,
            name: "z_index",
            inheritable: false,
            default_value: PropertyValue::Int(0),
            value_type: PropertyValueType::Int,
        });
        
        self.register_property(PropertyMetadata {
            id: PropertyId::Visibility,
            name: "visibility",
            inheritable: true,
            default_value: PropertyValue::Bool(true),
            value_type: PropertyValueType::Bool,
        });
        
        self.register_property(PropertyMetadata {
            id: PropertyId::Shadow,
            name: "box-shadow",
            inheritable: false,
            default_value: PropertyValue::String("none".to_string()),
            value_type: PropertyValueType::String,
        });
        
        // Alias for box-shadow
        self.register_property(PropertyMetadata {
            id: PropertyId::Shadow,
            name: "shadow",
            inheritable: false,
            default_value: PropertyValue::String("none".to_string()),
            value_type: PropertyValueType::String,
        });
        
        self.register_property(PropertyMetadata {
            id: PropertyId::Cursor,
            name: "cursor",
            inheritable: true,
            default_value: PropertyValue::String("default".to_string()),
            value_type: PropertyValueType::String,
        });
        
        // Layout Properties
        self.register_property(PropertyMetadata {
            id: PropertyId::Width,
            name: "width",
            inheritable: false,
            default_value: PropertyValue::Float(0.0),
            value_type: PropertyValueType::Float,
        });
        
        self.register_property(PropertyMetadata {
            id: PropertyId::Height,
            name: "height",
            inheritable: false,
            default_value: PropertyValue::Float(0.0),
            value_type: PropertyValueType::Float,
        });
        
        self.register_property(PropertyMetadata {
            id: PropertyId::Gap,
            name: "gap",
            inheritable: false,
            default_value: PropertyValue::Float(0.0),
            value_type: PropertyValueType::Float,
        });
        
        // CSS Grid Properties
        self.register_property(PropertyMetadata {
            id: PropertyId::GridTemplateColumns,
            name: "grid-template-columns",
            inheritable: false,
            default_value: PropertyValue::String("none".to_string()),
            value_type: PropertyValueType::String,
        });
        
        self.register_property(PropertyMetadata {
            id: PropertyId::GridTemplateRows,
            name: "grid-template-rows",
            inheritable: false,
            default_value: PropertyValue::String("none".to_string()),
            value_type: PropertyValueType::String,
        });
        
        self.register_property(PropertyMetadata {
            id: PropertyId::GridTemplateAreas,
            name: "grid-template-areas",
            inheritable: false,
            default_value: PropertyValue::String("none".to_string()),
            value_type: PropertyValueType::String,
        });
        
        self.register_property(PropertyMetadata {
            id: PropertyId::GridAutoColumns,
            name: "grid-auto-columns",
            inheritable: false,
            default_value: PropertyValue::String("auto".to_string()),
            value_type: PropertyValueType::String,
        });
        
        self.register_property(PropertyMetadata {
            id: PropertyId::GridAutoRows,
            name: "grid-auto-rows",
            inheritable: false,
            default_value: PropertyValue::String("auto".to_string()),
            value_type: PropertyValueType::String,
        });
        
        self.register_property(PropertyMetadata {
            id: PropertyId::GridAutoFlow,
            name: "grid-auto-flow",
            inheritable: false,
            default_value: PropertyValue::String("row".to_string()),
            value_type: PropertyValueType::String,
        });
        
        self.register_property(PropertyMetadata {
            id: PropertyId::GridArea,
            name: "grid-area",
            inheritable: false,
            default_value: PropertyValue::String("auto".to_string()),
            value_type: PropertyValueType::String,
        });
        
        self.register_property(PropertyMetadata {
            id: PropertyId::GridColumn,
            name: "grid-column",
            inheritable: false,
            default_value: PropertyValue::String("auto".to_string()),
            value_type: PropertyValueType::String,
        });
        
        self.register_property(PropertyMetadata {
            id: PropertyId::GridRow,
            name: "grid-row",
            inheritable: false,
            default_value: PropertyValue::String("auto".to_string()),
            value_type: PropertyValueType::String,
        });
        
        self.register_property(PropertyMetadata {
            id: PropertyId::GridColumnStart,
            name: "grid-column-start",
            inheritable: false,
            default_value: PropertyValue::String("auto".to_string()),
            value_type: PropertyValueType::String,
        });
        
        self.register_property(PropertyMetadata {
            id: PropertyId::GridColumnEnd,
            name: "grid-column-end",
            inheritable: false,
            default_value: PropertyValue::String("auto".to_string()),
            value_type: PropertyValueType::String,
        });
        
        self.register_property(PropertyMetadata {
            id: PropertyId::GridRowStart,
            name: "grid-row-start",
            inheritable: false,
            default_value: PropertyValue::String("auto".to_string()),
            value_type: PropertyValueType::String,
        });
        
        self.register_property(PropertyMetadata {
            id: PropertyId::GridRowEnd,
            name: "grid-row-end",
            inheritable: false,
            default_value: PropertyValue::String("auto".to_string()),
            value_type: PropertyValueType::String,
        });
        
        self.register_property(PropertyMetadata {
            id: PropertyId::GridGap,
            name: "grid-gap",
            inheritable: false,
            default_value: PropertyValue::Float(0.0),
            value_type: PropertyValueType::Float,
        });
        
        self.register_property(PropertyMetadata {
            id: PropertyId::GridColumnGap,
            name: "grid-column-gap",
            inheritable: false,
            default_value: PropertyValue::Float(0.0),
            value_type: PropertyValueType::Float,
        });
        
        self.register_property(PropertyMetadata {
            id: PropertyId::GridRowGap,
            name: "grid-row-gap",
            inheritable: false,
            default_value: PropertyValue::Float(0.0),
            value_type: PropertyValueType::Float,
        });
        
        // Box Model Properties
        self.register_property(PropertyMetadata {
            id: PropertyId::Padding,
            name: "padding",
            inheritable: false,
            default_value: PropertyValue::Float(0.0),
            value_type: PropertyValueType::Float,
        });
        
        self.register_property(PropertyMetadata {
            id: PropertyId::PaddingTop,
            name: "padding-top",
            inheritable: false,
            default_value: PropertyValue::Float(0.0),
            value_type: PropertyValueType::Float,
        });
        
        self.register_property(PropertyMetadata {
            id: PropertyId::PaddingRight,
            name: "padding-right",
            inheritable: false,
            default_value: PropertyValue::Float(0.0),
            value_type: PropertyValueType::Float,
        });
        
        self.register_property(PropertyMetadata {
            id: PropertyId::PaddingBottom,
            name: "padding-bottom",
            inheritable: false,
            default_value: PropertyValue::Float(0.0),
            value_type: PropertyValueType::Float,
        });
        
        self.register_property(PropertyMetadata {
            id: PropertyId::PaddingLeft,
            name: "padding-left",
            inheritable: false,
            default_value: PropertyValue::Float(0.0),
            value_type: PropertyValueType::Float,
        });
        
        self.register_property(PropertyMetadata {
            id: PropertyId::Margin,
            name: "margin",
            inheritable: false,
            default_value: PropertyValue::Float(0.0),
            value_type: PropertyValueType::Float,
        });
        
        self.register_property(PropertyMetadata {
            id: PropertyId::MarginTop,
            name: "margin-top",
            inheritable: false,
            default_value: PropertyValue::Float(0.0),
            value_type: PropertyValueType::Float,
        });
        
        self.register_property(PropertyMetadata {
            id: PropertyId::MarginRight,
            name: "margin-right",
            inheritable: false,
            default_value: PropertyValue::Float(0.0),
            value_type: PropertyValueType::Float,
        });
        
        self.register_property(PropertyMetadata {
            id: PropertyId::MarginBottom,
            name: "margin-bottom",
            inheritable: false,
            default_value: PropertyValue::Float(0.0),
            value_type: PropertyValueType::Float,
        });
        
        self.register_property(PropertyMetadata {
            id: PropertyId::MarginLeft,
            name: "margin-left",
            inheritable: false,
            default_value: PropertyValue::Float(0.0),
            value_type: PropertyValueType::Float,
        });
        
        self.register_property(PropertyMetadata {
            id: PropertyId::BorderTopWidth,
            name: "border-top-width",
            inheritable: false,
            default_value: PropertyValue::Float(0.0),
            value_type: PropertyValueType::Float,
        });
        
        self.register_property(PropertyMetadata {
            id: PropertyId::BorderRightWidth,
            name: "border-right-width",
            inheritable: false,
            default_value: PropertyValue::Float(0.0),
            value_type: PropertyValueType::Float,
        });
        
        self.register_property(PropertyMetadata {
            id: PropertyId::BorderBottomWidth,
            name: "border-bottom-width",
            inheritable: false,
            default_value: PropertyValue::Float(0.0),
            value_type: PropertyValueType::Float,
        });
        
        self.register_property(PropertyMetadata {
            id: PropertyId::BorderLeftWidth,
            name: "border-left-width",
            inheritable: false,
            default_value: PropertyValue::Float(0.0),
            value_type: PropertyValueType::Float,
        });
        
        self.register_property(PropertyMetadata {
            id: PropertyId::BorderTopColor,
            name: "border-top-color",
            inheritable: false,
            default_value: PropertyValue::Color(Vec4::ZERO),
            value_type: PropertyValueType::Color,
        });
        
        self.register_property(PropertyMetadata {
            id: PropertyId::BorderRightColor,
            name: "border-right-color",
            inheritable: false,
            default_value: PropertyValue::Color(Vec4::ZERO),
            value_type: PropertyValueType::Color,
        });
        
        self.register_property(PropertyMetadata {
            id: PropertyId::BorderBottomColor,
            name: "border-bottom-color",
            inheritable: false,
            default_value: PropertyValue::Color(Vec4::ZERO),
            value_type: PropertyValueType::Color,
        });
        
        self.register_property(PropertyMetadata {
            id: PropertyId::BorderLeftColor,
            name: "border-left-color",
            inheritable: false,
            default_value: PropertyValue::Color(Vec4::ZERO),
            value_type: PropertyValueType::Color,
        });
        
        self.register_property(PropertyMetadata {
            id: PropertyId::BorderTopLeftRadius,
            name: "border-top-left-radius",
            inheritable: false,
            default_value: PropertyValue::Float(0.0),
            value_type: PropertyValueType::Float,
        });
        
        self.register_property(PropertyMetadata {
            id: PropertyId::BorderTopRightRadius,
            name: "border-top-right-radius",
            inheritable: false,
            default_value: PropertyValue::Float(0.0),
            value_type: PropertyValueType::Float,
        });
        
        self.register_property(PropertyMetadata {
            id: PropertyId::BorderBottomRightRadius,
            name: "border-bottom-right-radius",
            inheritable: false,
            default_value: PropertyValue::Float(0.0),
            value_type: PropertyValueType::Float,
        });
        
        self.register_property(PropertyMetadata {
            id: PropertyId::BorderBottomLeftRadius,
            name: "border-bottom-left-radius",
            inheritable: false,
            default_value: PropertyValue::Float(0.0),
            value_type: PropertyValueType::Float,
        });
        
        self.register_property(PropertyMetadata {
            id: PropertyId::BoxSizing,
            name: "box-sizing",
            inheritable: false,
            default_value: PropertyValue::String("content-box".to_string()),
            value_type: PropertyValueType::String,
        });
        
        self.register_property(PropertyMetadata {
            id: PropertyId::Outline,
            name: "outline",
            inheritable: false,
            default_value: PropertyValue::Float(0.0),
            value_type: PropertyValueType::Float,
        });
        
        self.register_property(PropertyMetadata {
            id: PropertyId::OutlineColor,
            name: "outline-color",
            inheritable: false,
            default_value: PropertyValue::Color(Vec4::ZERO),
            value_type: PropertyValueType::Color,
        });
        
        self.register_property(PropertyMetadata {
            id: PropertyId::OutlineWidth,
            name: "outline-width",
            inheritable: false,
            default_value: PropertyValue::Float(0.0),
            value_type: PropertyValueType::Float,
        });
        
        self.register_property(PropertyMetadata {
            id: PropertyId::OutlineOffset,
            name: "outline-offset",
            inheritable: false,
            default_value: PropertyValue::Float(0.0),
            value_type: PropertyValueType::Float,
        });
        
        // Overflow properties
        self.register_property(PropertyMetadata {
            id: PropertyId::Overflow,
            name: "overflow",
            inheritable: false,
            default_value: PropertyValue::String("visible".to_string()),
            value_type: PropertyValueType::String,
        });
        
        self.register_property(PropertyMetadata {
            id: PropertyId::OverflowX,
            name: "overflow-x",
            inheritable: false,
            default_value: PropertyValue::String("visible".to_string()),
            value_type: PropertyValueType::String,
        });
        
        self.register_property(PropertyMetadata {
            id: PropertyId::OverflowY,
            name: "overflow-y",
            inheritable: false,
            default_value: PropertyValue::String("visible".to_string()),
            value_type: PropertyValueType::String,
        });
        
        // Add more properties as needed...
    }
    
    fn register_property(&mut self, metadata: PropertyMetadata) {
        let index = self.properties.len();
        let id_value = metadata.id.as_u8();
        
        self.properties.push(metadata);
        self.id_to_index[id_value as usize] = Some(index);
    }
    
    pub fn get_property_metadata(&self, id: PropertyId) -> Option<&PropertyMetadata> {
        let id_value = id.as_u8();
        self.id_to_index[id_value as usize]
            .and_then(|index| self.properties.get(index))
    }
    
    pub fn get_property_by_u8(&self, id: u8) -> Option<&PropertyMetadata> {
        self.id_to_index[id as usize]
            .and_then(|index| self.properties.get(index))
    }
    
    pub fn is_inheritable(&self, id: PropertyId) -> bool {
        self.get_property_metadata(id)
            .map(|meta| meta.inheritable)
            .unwrap_or(false)
    }
    
    pub fn get_default_value(&self, id: PropertyId) -> Option<&PropertyValue> {
        self.get_property_metadata(id)
            .map(|meta| &meta.default_value)
    }
    
    pub fn get_property_name(&self, id: PropertyId) -> Option<&str> {
        self.get_property_metadata(id)
            .map(|meta| meta.name)
    }
    
    pub fn all_properties(&self) -> &[PropertyMetadata] {
        &self.properties
    }
}

impl Default for PropertyRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_property_registry_lookup() {
        let registry = PropertyRegistry::new();
        
        // Test property lookup
        let bg_color = registry.get_property_metadata(PropertyId::BackgroundColor);
        assert!(bg_color.is_some());
        assert_eq!(bg_color.unwrap().name, "background-color");
        assert!(!bg_color.unwrap().inheritable);
        
        // Test inheritance
        assert!(registry.is_inheritable(PropertyId::TextColor));
        assert!(!registry.is_inheritable(PropertyId::BackgroundColor));
        
        // Test u8 conversion
        let prop_from_u8 = registry.get_property_by_u8(0x01);
        assert!(prop_from_u8.is_some());
        assert_eq!(prop_from_u8.unwrap().id, PropertyId::BackgroundColor);
    }
    
    #[test]
    fn test_property_id_conversion() {
        assert_eq!(PropertyId::from(0x01), PropertyId::BackgroundColor);
        assert_eq!(PropertyId::from(0x02), PropertyId::TextColor);
        assert_eq!(PropertyId::from(0xFF), PropertyId::Custom(0xFF));
    }
}
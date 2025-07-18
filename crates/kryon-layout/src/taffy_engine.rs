//! Taffy-based layout engine for Kryon
//! 
//! This module provides modern Grid and Flexbox layout capabilities using Taffy,
//! implementing Kryon's own styling system while maintaining KRB binary compatibility.

use kryon_core::{Element, ElementId};
use glam::Vec2;
use std::collections::HashMap;
use taffy::prelude::*;
use taffy::ResolveOrZero;
use tracing::debug;

/// Taffy-based layout engine that replaces the legacy flex layout system
pub struct TaffyLayoutEngine {
    /// Taffy layout tree
    taffy: TaffyTree<ElementId>,
    /// Map from element ID to Taffy node
    element_to_node: HashMap<ElementId, taffy::NodeId>,
    /// Map from Taffy node back to element ID
    node_to_element: HashMap<taffy::NodeId, ElementId>,
    /// Cached final layout results
    layout_cache: HashMap<ElementId, Layout>,
}

impl TaffyLayoutEngine {
    /// Create a new Taffy layout engine
    pub fn new() -> Self {
        Self {
            taffy: TaffyTree::new(),
            element_to_node: HashMap::new(),
            node_to_element: HashMap::new(),
            layout_cache: HashMap::new(),
        }
    }

    /// Convert KRB elements to Taffy layout tree and compute layout
    pub fn compute_taffy_layout(
        &mut self,
        elements: &HashMap<ElementId, Element>,
        root_element_id: ElementId,
        available_space: Size<f32>,
    ) -> Result<(), taffy::TaffyError> {
        // Clear previous state
        self.clear();

        // Build Taffy tree from KRB elements in deterministic order
        let root_node = self.build_taffy_tree_deterministic(elements, root_element_id)?;
        
        // Compute layout with Taffy
        let available_space = Size {
            width: AvailableSpace::Definite(available_space.width),
            height: AvailableSpace::Definite(available_space.height),
        };
        
        self.taffy.compute_layout(root_node, available_space)?;

        // Cache layout results
        self.cache_layouts(elements)?;
        
        // Debug: Print computed layouts  
        eprintln!("[TAFFY_CACHE] Layout cache has {} entries", self.layout_cache.len());
        for (&element_id, layout) in &self.layout_cache {
            eprintln!("[TAFFY_COMPUTED] Element {}: pos=({}, {}), size=({}, {})", 
                element_id, layout.location.x, layout.location.y, layout.size.width, layout.size.height);
        }

        debug!("Taffy layout computation completed for {} elements", elements.len());
        Ok(())
    }

    /// Get the computed layout for an element
    pub fn get_layout(&self, element_id: ElementId) -> Option<&Layout> {
        self.layout_cache.get(&element_id)
    }

    /// Clear all cached data and create fresh Taffy instance
    fn clear(&mut self) {
        // Create completely fresh Taffy instance to avoid node ID reuse bugs
        self.taffy = TaffyTree::new();
        self.element_to_node.clear();
        self.node_to_element.clear();
        self.layout_cache.clear();
    }

    /// Build Taffy tree in deterministic order to avoid node ID confusion
    fn build_taffy_tree_deterministic(
        &mut self,
        elements: &HashMap<ElementId, Element>,
        root_element_id: ElementId,
    ) -> Result<taffy::NodeId, taffy::TaffyError> {
        // First pass: Create all nodes in sorted order by element ID
        let mut sorted_elements: Vec<_> = elements.iter().collect();
        sorted_elements.sort_by_key(|(id, _)| *id);
        
        for (&element_id, element) in sorted_elements {
            let style = self.krb_to_taffy_style(element);
            let node = self.taffy.new_leaf(style)?;
            
            self.element_to_node.insert(element_id, node);
            self.node_to_element.insert(node, element_id);
            
            eprintln!("[TAFFY_NODE] Element {} -> Taffy Node {:?}", element_id, node);
        }
        
        // Second pass: Set up parent-child relationships
        for (&element_id, element) in elements {
            if let Some(&parent_node) = self.element_to_node.get(&element_id) {
                let mut child_nodes = Vec::new();
                for &child_id in &element.children {
                    if let Some(&child_node) = self.element_to_node.get(&child_id) {
                        child_nodes.push(child_node);
                    }
                }
                
                if !child_nodes.is_empty() {
                    eprintln!("[TAFFY_TREE] Element {} (Node {:?}) has children: {:?}", 
                        element_id, parent_node, child_nodes);
                    self.taffy.set_children(parent_node, &child_nodes)?;
                } else {
                    eprintln!("[TAFFY_TREE] Element {} (Node {:?}) is a leaf node", 
                        element_id, parent_node);
                }
            }
        }
        
        // Return root node
        self.element_to_node.get(&root_element_id)
            .copied()
            .ok_or_else(|| taffy::TaffyError::InvalidChildNode(taffy::NodeId::new(0)))
    }


    /// Convert kryon-core Element to Taffy Style
    fn krb_to_taffy_style(&self, element: &Element) -> Style {
        let mut style = Style::default();

        // Apply default flex layout for containers first (provides defaults)
        self.apply_default_container_layout(&mut style, element);
        
        // Apply modern CSS properties (these override defaults)
        self.apply_custom_properties(&mut style, element);

        // Apply size constraints from element - check both size and layout_size for width
        let explicit_width = if let kryon_core::LayoutDimension::Pixels(width) = element.layout_size.width {
            if width > 0.0 { Some(width) } else { None }
        } else { 
            if element.size.x > 0.0 { Some(element.size.x) } else { None }
        };
        
        // Apply size constraints from element - check both size and layout_size for height  
        let explicit_height = if let kryon_core::LayoutDimension::Pixels(height) = element.layout_size.height {
            if height > 0.0 { Some(height) } else { None }
        } else { 
            if element.size.y > 0.0 { Some(element.size.y) } else { None }
        };
        
        if let Some(width) = explicit_width {
            style.size.width = Dimension::Length(width);
            // Only prevent flex grow/shrink if both dimensions are explicitly set
            // This allows buttons with explicit height to still grow horizontally
            if explicit_height.is_some() {
                style.flex_grow = 0.0;
                style.flex_shrink = 0.0;
                eprintln!("[TAFFY_SIZE] Element '{}': Using width {}px from layout_size (both dimensions set, flex-shrink: 0)", element.id, width);
            } else {
                eprintln!("[TAFFY_SIZE] Element '{}': Using width {}px from layout_size (height flexible)", element.id, width);
            }
        } else if element.element_type == kryon_core::ElementType::Container {
            eprintln!("[TAFFY_CONTAINER] Container '{}': no explicit width, using intrinsic sizing", element.id);
        }
        
        if let Some(height) = explicit_height {
            style.size.height = Dimension::Length(height);
            // Only prevent flex grow/shrink if both dimensions are explicitly set
            // This allows buttons with explicit height to still grow horizontally
            if explicit_width.is_some() {
                style.flex_grow = 0.0;
                style.flex_shrink = 0.0;
                eprintln!("[TAFFY_SIZE] Element '{}': Using height {}px from layout_size (both dimensions set, flex-shrink: 0)", element.id, height);
            } else {
                eprintln!("[TAFFY_SIZE] Element '{}': Using height {}px from layout_size (width flexible)", element.id, height);
            }
        }

        // Apply position from element ONLY if it's explicitly set as absolute positioning
        // Don't treat computed layout positions as absolute positioning
        let has_position_absolute = element.custom_properties.get("position")
            .map(|v| if let kryon_core::PropertyValue::String(s) = v { s == "absolute" } else { false })
            .unwrap_or(false);
        
        // Check if element has centering layout that should override absolute positioning
        let has_centering_layout = element.custom_properties.contains_key("justify_content") || 
                                  element.custom_properties.contains_key("align_items");
        
        if has_position_absolute && (element.position.x != 0.0 || element.position.y != 0.0) && !has_centering_layout {
            style.position = Position::Absolute;
            style.inset.left = LengthPercentage::Length(element.position.x).into();
            style.inset.top = LengthPercentage::Length(element.position.y).into();
            eprintln!("[TAFFY_ABSOLUTE] Element '{}': applying absolute position ({}, {})", 
                element.id, element.position.x, element.position.y);
        } else if has_position_absolute && has_centering_layout {
            eprintln!("[TAFFY_CENTER_OVERRIDE] Element '{}': ignoring absolute position ({}, {}) in favor of flex centering", 
                element.id, element.position.x, element.position.y);
        }

        // Handle text element intrinsic sizing
        if element.element_type == kryon_core::ElementType::Text || element.element_type == kryon_core::ElementType::Link {
            style.display = Display::Block;
            
            // Text elements should fit within their parent container
            // Only calculate width if no explicit width was set
            if style.size.width == Dimension::Auto {
                // Leave width as Auto - text will fill the parent container
                let element_name = if element.element_type == kryon_core::ElementType::Text { "Text" } else { "Link" };
                eprintln!("[TAFFY_TEXT_SIZE] {} Element '{}': using parent container width", element_name, element.id);
            } else {
                eprintln!("[TAFFY_TEXT_SIZE] Element '{}': respecting explicit width {:?}", element.id, style.size.width);
            }
            
            // Calculate intrinsic text height if not explicitly set
            if element.size.y == 0.0 {
                let text_height = element.font_size.max(16.0);
                style.size.height = Dimension::Length(text_height);
            }
        } else {
            // Set default display based on element type
            style.display = match element.element_type {
                kryon_core::ElementType::Container => Display::Flex,
                kryon_core::ElementType::App => Display::Flex, // App should be flex container by default
                kryon_core::ElementType::Button => Display::Block, // Buttons should be block-level for proper sizing
                _ => Display::Block,
            };
        }
        
        // Override display for specific element types (must be at the end)
        match element.element_type {
            kryon_core::ElementType::Button => {
                // Buttons should behave as block elements that respect their explicit dimensions
                eprintln!("[TAFFY_BUTTON_OVERRIDE] Setting button '{}' as block element with fixed dimensions", element.id);
                style.display = Display::Block;
                
                // Check if flex_grow was explicitly set in custom properties
                let has_explicit_flex_grow = element.custom_properties.contains_key("flex_grow");
                
                // Force buttons to preserve their size by making them non-flexible
                if !has_explicit_flex_grow {
                    style.flex_grow = 0.0;
                    eprintln!("[TAFFY_BUTTON_FLEX] Button '{}': no explicit flex_grow, setting to 0.0", element.id);
                } else {
                    eprintln!("[TAFFY_BUTTON_FLEX] Button '{}': respecting explicit flex_grow = {}", element.id, style.flex_grow);
                }
                style.flex_shrink = 0.0;
                style.flex_basis = taffy::Dimension::Auto; // Don't use flex-basis
                // For buttons with explicit size, make them absolutely positioned within their flex container
                if element.size.x > 0.0 || element.size.y > 0.0 {
                    eprintln!("[TAFFY_BUTTON_SIZE_LOCK] Button '{}' has explicit size, preventing flex stretching", element.id);
                    // Don't change to absolute position, but ensure min/max size constraints
                    if element.size.x > 0.0 {
                        style.min_size.width = taffy::Dimension::Length(element.size.x);
                        style.max_size.width = taffy::Dimension::Length(element.size.x);
                    }
                    if element.size.y > 0.0 {
                        style.min_size.height = taffy::Dimension::Length(element.size.y);
                        style.max_size.height = taffy::Dimension::Length(element.size.y);
                    }
                }
                // Reset flex container properties that might have been set by layout_flags
                style.align_items = None;
                style.justify_content = None;
            }
            kryon_core::ElementType::App => {
                // App elements should always be flex containers that center their content by default
                // Only set defaults if not explicitly overridden by custom properties
                if !element.custom_properties.contains_key("display") {
                    style.display = Display::Flex;
                }
                if !element.custom_properties.contains_key("flex_direction") {
                    style.flex_direction = FlexDirection::Column;
                }
                if !element.custom_properties.contains_key("align_items") {
                    style.align_items = Some(AlignItems::Center);
                }
                if !element.custom_properties.contains_key("justify_content") {
                    style.justify_content = Some(JustifyContent::Center);
                }
                eprintln!("[TAFFY_APP_DEFAULTS] App '{}': setting default flex centering (respecting custom properties)", element.id);
            }
            kryon_core::ElementType::Container => {
                // Ensure Container elements stay as flex containers if they have ANY flex properties
                let has_flex_direction = element.custom_properties.contains_key("flex_direction");
                let has_justify_content = style.justify_content.is_some();
                let has_align_items = style.align_items.is_some();
                let has_display_flex = element.custom_properties.get("display").map_or(false, |v| {
                    if let kryon_core::PropertyValue::String(s) = v { s == "flex" } else { false }
                });
                let has_layout_flags = element.layout_flags != 0;
                
                if has_flex_direction || has_justify_content || has_align_items || has_display_flex || has_layout_flags {
                    eprintln!("[TAFFY_CONTAINER_OVERRIDE] Ensuring '{}' stays Display::Flex (flex_dir={}, justify={}, align={}, display_flex={}, flags={})", 
                        element.id, has_flex_direction, has_justify_content, has_align_items, has_display_flex, has_layout_flags);
                    style.display = Display::Flex;
                }
            }
            _ => {}
        }
        
        eprintln!("[TAFFY_STYLE] Element '{}': layout_flags=0x{:02X}, display={:?}, flex_direction={:?}, align_items={:?}, justify_content={:?}", 
            element.id, element.layout_flags, style.display, style.flex_direction, style.align_items, style.justify_content);

        style
    }

    /// Apply custom properties from element to Taffy style
    fn apply_custom_properties(&self, style: &mut Style, element: &Element) {
        use kryon_core::PropertyValue;
        
        // Parse Kryon Grid properties
        if let Some(PropertyValue::String(value)) = element.custom_properties.get("display") {
            match value.as_str() {
                "grid" => style.display = Display::Grid,
                "flex" => style.display = Display::Flex,
                "block" => style.display = Display::Block,
                "none" => style.display = Display::None,
                _ => {}
            }
        }

        // Grid template columns/rows
        if let Some(PropertyValue::String(value)) = element.custom_properties.get("grid_template_columns") {
            style.grid_template_columns = self.parse_grid_track_list(value);
        }
        if let Some(PropertyValue::String(value)) = element.custom_properties.get("grid_template_rows") {
            style.grid_template_rows = self.parse_grid_track_list(value);
        }
        
        // Grid template areas (Note: Taffy 0.5 may not have this field yet)
        // TODO: Implement when Taffy adds grid-template-areas support
        
        // Grid auto columns/rows - TODO: Implement proper type conversion
        // For now, skipping these as they require different types than TrackSizingFunction
        if let Some(PropertyValue::String(_value)) = element.custom_properties.get("grid_auto_columns") {
            // TODO: Implement grid_auto_columns when proper type conversion is available
            // style.grid_auto_columns = ...;
        }
        if let Some(PropertyValue::String(_value)) = element.custom_properties.get("grid_auto_rows") {
            // TODO: Implement grid_auto_rows when proper type conversion is available
            // style.grid_auto_rows = ...;
        }
        
        // Grid auto flow
        if let Some(PropertyValue::String(value)) = element.custom_properties.get("grid_auto_flow") {
            style.grid_auto_flow = match value.as_str() {
                "row" => taffy::GridAutoFlow::Row,
                "column" => taffy::GridAutoFlow::Column,
                "row dense" => taffy::GridAutoFlow::RowDense,
                "column dense" => taffy::GridAutoFlow::ColumnDense,
                _ => taffy::GridAutoFlow::Row,
            };
        }
        
        // Grid area (shorthand for grid-row-start/end and grid-column-start/end)
        if let Some(PropertyValue::String(value)) = element.custom_properties.get("grid_area") {
            let (row_start, column_start, row_end, column_end) = self.parse_grid_area(value);
            style.grid_row = taffy::Line { start: row_start, end: row_end };
            style.grid_column = taffy::Line { start: column_start, end: column_end };
        } else {
            // Individual grid placement properties
            if let Some(PropertyValue::String(value)) = element.custom_properties.get("grid_column_start") {
                style.grid_column.start = self.parse_grid_line(value);
            }
            if let Some(PropertyValue::String(value)) = element.custom_properties.get("grid_column_end") {
                style.grid_column.end = self.parse_grid_line(value);
            }
            if let Some(PropertyValue::String(value)) = element.custom_properties.get("grid_row_start") {
                style.grid_row.start = self.parse_grid_line(value);
            }
            if let Some(PropertyValue::String(value)) = element.custom_properties.get("grid_row_end") {
                style.grid_row.end = self.parse_grid_line(value);
            }
            
            // Grid column/row shorthand
            if let Some(PropertyValue::String(value)) = element.custom_properties.get("grid_column") {
                let (start, end) = self.parse_grid_line_range(value);
                style.grid_column = taffy::Line { start, end };
            }
            if let Some(PropertyValue::String(value)) = element.custom_properties.get("grid_row") {
                let (start, end) = self.parse_grid_line_range(value);
                style.grid_row = taffy::Line { start, end };
            }
        }

        // Flexbox properties
        if let Some(PropertyValue::String(value)) = element.custom_properties.get("flex_direction") {
            style.flex_direction = match value.as_str() {
                "row" => FlexDirection::Row,
                "column" => FlexDirection::Column,
                "row-reverse" => FlexDirection::RowReverse,
                "column-reverse" => FlexDirection::ColumnReverse,
                _ => FlexDirection::Row,
            };
        }
        
        // Flex wrap
        if let Some(PropertyValue::String(value)) = element.custom_properties.get("flex_wrap") {
            style.flex_wrap = match value.as_str() {
                "nowrap" => FlexWrap::NoWrap,
                "wrap" => FlexWrap::Wrap,
                "wrap-reverse" => FlexWrap::WrapReverse,
                _ => FlexWrap::NoWrap,
            };
        }
        
        // Flex basis
        if let Some(value) = element.custom_properties.get("flex_basis") {
            style.flex_basis = match value {
                PropertyValue::String(s) => match s.as_str() {
                    "auto" => Dimension::Auto,
                    "content" => Dimension::Auto, // Taffy doesn't have content, use auto
                    _ => {
                        if s.ends_with("px") {
                            if let Ok(px_value) = s.trim_end_matches("px").parse::<f32>() {
                                Dimension::Length(px_value)
                            } else {
                                Dimension::Auto
                            }
                        } else if s.ends_with("%") {
                            if let Ok(percent_value) = s.trim_end_matches("%").parse::<f32>() {
                                Dimension::Percent(percent_value / 100.0)
                            } else {
                                Dimension::Auto
                            }
                        } else {
                            Dimension::Auto
                        }
                    }
                }
                PropertyValue::Float(f) => Dimension::Length(*f),
                PropertyValue::Int(i) => Dimension::Length(*i as f32),
                _ => Dimension::Auto,
            };
        }

        if let Some(PropertyValue::String(value)) = element.custom_properties.get("align_items") {
            style.align_items = Some(match value.as_str() {
                "start" | "flex-start" | "flex_start" => AlignItems::Start,
                "center" => AlignItems::Center,
                "end" | "flex-end" | "flex_end" => AlignItems::End,
                "stretch" => AlignItems::Stretch,
                "baseline" => AlignItems::Baseline,
                _ => AlignItems::Start,
            });
        }
        
        // Align self (for individual flex items)
        if let Some(PropertyValue::String(value)) = element.custom_properties.get("align_self") {
            style.align_self = Some(match value.as_str() {
                "start" | "flex-start" => AlignSelf::Start,
                "center" => AlignSelf::Center,
                "end" | "flex-end" => AlignSelf::End,
                "stretch" => AlignSelf::Stretch,
                "baseline" => AlignSelf::Baseline,
                _ => AlignSelf::Start, // Default to start instead of auto
            });
        }
        
        // Align content (for wrapped flex lines)
        if let Some(PropertyValue::String(value)) = element.custom_properties.get("align_content") {
            style.align_content = Some(match value.as_str() {
                "start" | "flex-start" => AlignContent::Start,
                "center" => AlignContent::Center,
                "end" | "flex-end" => AlignContent::End,
                "stretch" => AlignContent::Stretch,
                "space-between" => AlignContent::SpaceBetween,
                "space-around" => AlignContent::SpaceAround,
                "space-evenly" => AlignContent::SpaceEvenly,
                _ => AlignContent::Start,
            });
        }

        if let Some(PropertyValue::String(value)) = element.custom_properties.get("justify_content") {
            style.justify_content = Some(match value.as_str() {
                "start" | "flex-start" | "flex_start" => JustifyContent::Start,
                "center" => JustifyContent::Center,
                "end" | "flex-end" | "flex_end" => JustifyContent::End,
                "space-between" | "space_between" => JustifyContent::SpaceBetween,
                "space-around" | "space_around" => JustifyContent::SpaceAround,
                "space-evenly" | "space_evenly" => JustifyContent::SpaceEvenly,
                _ => JustifyContent::Start,
            });
        }

        // Flex grow/shrink/basis
        if let Some(value) = element.custom_properties.get("flex_grow") {
            let grow_value = match value {
                PropertyValue::String(s) => s.parse::<f32>().unwrap_or(0.0),
                PropertyValue::Float(f) => *f,
                PropertyValue::Int(i) => *i as f32,
                _ => 0.0,
            };
            // eprintln!("[TAFFY_FLEX] Element '{}' has flex_grow: {} (value: {:?})", element.id, grow_value, value);
            if grow_value > 0.0 {
                style.flex_grow = grow_value;
                // eprintln!("[TAFFY_FLEX] Applied flex_grow: {} to element '{}'", grow_value, element.id);
            }
        }
        if let Some(value) = element.custom_properties.get("flex_shrink") {
            let shrink_value = match value {
                PropertyValue::String(s) => s.parse::<f32>().unwrap_or(1.0),
                PropertyValue::Float(f) => *f,
                PropertyValue::Int(i) => *i as f32,
                _ => 1.0,
            };
            style.flex_shrink = shrink_value;
        }
        
        // Flex order - TODO: Check if this is available in current Taffy version
        if let Some(value) = element.custom_properties.get("order") {
            let _order_value = match value {
                PropertyValue::String(s) => s.parse::<i32>().unwrap_or(0),
                PropertyValue::Int(i) => *i,
                PropertyValue::Float(f) => *f as i32,
                _ => 0,
            };
            // style.order = order_value; // TODO: Uncomment when Taffy supports this
            eprintln!("[TAFFY_ORDER] Order property not yet supported in this Taffy version");
        }

        // Position properties
        if let Some(PropertyValue::String(value)) = element.custom_properties.get("position") {
            style.position = match value.as_str() {
                "absolute" => Position::Absolute,
                "relative" => Position::Relative,
                _ => Position::Relative,
            };
        }

        // Gap property
        if let Some(value) = element.custom_properties.get("gap") {
            if let Some(gap_value) = value.as_float() {
                style.gap = Size {
                    width: LengthPercentage::Length(gap_value),
                    height: LengthPercentage::Length(gap_value),
                };
            }
        }

        // Box Model Properties

        // Padding properties
        if let Some(value) = element.custom_properties.get("padding") {
            if let Some(padding_value) = value.as_float() {
                let padding = LengthPercentage::Length(padding_value);
                style.padding = Rect {
                    left: padding,
                    right: padding,
                    top: padding,
                    bottom: padding,
                };
            }
        }

        // Individual padding sides
        if let Some(value) = element.custom_properties.get("padding_top") {
            if let Some(val) = value.as_float() {
                style.padding.top = LengthPercentage::Length(val);
            }
        }
        if let Some(value) = element.custom_properties.get("padding_right") {
            if let Some(val) = value.as_float() {
                style.padding.right = LengthPercentage::Length(val);
            }
        }
        if let Some(value) = element.custom_properties.get("padding_bottom") {
            if let Some(val) = value.as_float() {
                style.padding.bottom = LengthPercentage::Length(val);
            }
        }
        if let Some(value) = element.custom_properties.get("padding_left") {
            if let Some(val) = value.as_float() {
                style.padding.left = LengthPercentage::Length(val);
            }
        }

        // Margin properties
        if let Some(value) = element.custom_properties.get("margin") {
            if let Some(margin_value) = value.as_float() {
                let margin = LengthPercentage::Length(margin_value).into();
                style.margin = Rect {
                    left: margin,
                    right: margin,
                    top: margin,
                    bottom: margin,
                };
            }
        }

        // Individual margin sides
        if let Some(value) = element.custom_properties.get("margin_top") {
            if let Some(val) = value.as_float() {
                style.margin.top = LengthPercentage::Length(val).into();
            }
        }
        if let Some(value) = element.custom_properties.get("margin_right") {
            if let Some(val) = value.as_float() {
                style.margin.right = LengthPercentage::Length(val).into();
            }
        }
        if let Some(value) = element.custom_properties.get("margin_bottom") {
            if let Some(val) = value.as_float() {
                style.margin.bottom = LengthPercentage::Length(val).into();
            }
        }
        if let Some(value) = element.custom_properties.get("margin_left") {
            if let Some(val) = value.as_float() {
                style.margin.left = LengthPercentage::Length(val).into();
            }
        }

        // Border properties
        if let Some(value) = element.custom_properties.get("border_width") {
            if let Some(border_value) = value.as_float() {
                let border = LengthPercentage::Length(border_value);
                style.border = Rect {
                    left: border,
                    right: border,
                    top: border,
                    bottom: border,
                };
            }
        }

        // Individual border width sides
        if let Some(value) = element.custom_properties.get("border_top_width") {
            if let Some(val) = value.as_float() {
                style.border.top = LengthPercentage::Length(val);
            }
        }
        if let Some(value) = element.custom_properties.get("border_right_width") {
            if let Some(val) = value.as_float() {
                style.border.right = LengthPercentage::Length(val);
            }
        }
        if let Some(value) = element.custom_properties.get("border_bottom_width") {
            if let Some(val) = value.as_float() {
                style.border.bottom = LengthPercentage::Length(val);
            }
        }
        if let Some(value) = element.custom_properties.get("border_left_width") {
            if let Some(val) = value.as_float() {
                style.border.left = LengthPercentage::Length(val);
            }
        }

        // Box sizing property
        if let Some(PropertyValue::String(value)) = element.custom_properties.get("box_sizing") {
            match value.as_str() {
                "border-box" => {
                    // Implement border-box sizing by adjusting size constraints
                    // In border-box, width/height include padding and border
                    if let Some(width_value) = element.custom_properties.get("width") {
                        if let Some(width) = width_value.as_float() {
                            // Calculate content width by subtracting padding and border
                            let padding_left = style.padding.left.resolve_or_zero(Some(width));
                            let padding_right = style.padding.right.resolve_or_zero(Some(width));
                            let border_left = style.border.left.resolve_or_zero(Some(width));
                            let border_right = style.border.right.resolve_or_zero(Some(width));
                            
                            let content_width = width - padding_left - padding_right - border_left - border_right;
                            style.size.width = Dimension::Length(content_width.max(0.0));
                        }
                    }
                    
                    if let Some(height_value) = element.custom_properties.get("height") {
                        if let Some(height) = height_value.as_float() {
                            // Calculate content height by subtracting padding and border
                            let padding_top = style.padding.top.resolve_or_zero(Some(height));
                            let padding_bottom = style.padding.bottom.resolve_or_zero(Some(height));
                            let border_top = style.border.top.resolve_or_zero(Some(height));
                            let border_bottom = style.border.bottom.resolve_or_zero(Some(height));
                            
                            let content_height = height - padding_top - padding_bottom - border_top - border_bottom;
                            style.size.height = Dimension::Length(content_height.max(0.0));
                        }
                    }
                    
                    eprintln!("[TAFFY_BOX_MODEL] Applied border-box sizing calculations");
                }
                "content-box" => {
                    // This is the default behavior in Taffy
                    eprintln!("[TAFFY_BOX_MODEL] Using content-box sizing (default)");
                }
                _ => {}
            }
        }
    }

    /// Apply default container layout behavior (replaces legacy layout flags)
    fn apply_default_container_layout(&self, style: &mut Style, element: &Element) {
        // Set default display behavior based on element type and CSS properties
        match element.element_type {
            kryon_core::ElementType::App | kryon_core::ElementType::Container => {
                // Containers are flex by default unless overridden by CSS display property
                if !element.custom_properties.contains_key("display") {
                    style.display = Display::Flex;
                    
                    // Default flex direction if not specified by CSS
                    if !element.custom_properties.contains_key("flex_direction") {
                        style.flex_direction = FlexDirection::Row; // Default to row layout
                    }
                }
            }
            kryon_core::ElementType::Button => {
                style.display = Display::Block;
                // Only apply minimum sizing if no explicit size was set
                if style.size.width == Dimension::Auto {
                    style.min_size.width = Dimension::Length(80.0);
                }
                if style.size.height == Dimension::Auto {
                    style.min_size.height = Dimension::Length(40.0);
                }
                eprintln!("[TAFFY_BUTTON] Button '{}': explicit width={:?}, height={:?}, min_width={:?}, min_height={:?}", 
                    element.id, style.size.width, style.size.height, style.min_size.width, style.min_size.height);
            }
            kryon_core::ElementType::Text => {
                style.display = Display::Block;
            }
            kryon_core::ElementType::Link => {
                style.display = Display::Block;
                // Links are inline by default but can be styled as block
                // For now, treat them similar to Text elements
            }
            kryon_core::ElementType::Canvas => {
                style.display = Display::Block;
                // Canvas elements are block-level containers for custom drawing
                // They should maintain their specified width and height
            }
            kryon_core::ElementType::WasmView => {
                style.display = Display::Block;
                // WasmView elements are block-level containers for WASM module output
                // They should maintain their specified width and height for the WASM viewport
            }
            _ => {
                style.display = Display::Block;
            }
        }
        
        eprintln!("[TAFFY_DEFAULT] Element '{}' type={:?}: set display={:?}, flex_direction={:?}", 
            element.id, element.element_type, style.display, style.flex_direction);
    }

    /// Cache computed layouts for all elements
    fn cache_layouts(&mut self, elements: &HashMap<ElementId, Element>) -> Result<(), taffy::TaffyError> {
        for (&element_id, element) in elements {
            if let Some(&node) = self.element_to_node.get(&element_id) {
                let layout = self.taffy.layout(node)?;
                eprintln!("[TAFFY_CACHE] Element {} '{}' (Node {:?}): size=({}, {}) type={:?}", 
                    element_id, element.id, node, layout.size.width, layout.size.height, element.element_type);
                
                // Extra debugging: Also log the reverse mapping
                if let Some(&mapped_element_id) = self.node_to_element.get(&node) {
                    if mapped_element_id != element_id {
                        eprintln!("[TAFFY_CACHE_ERROR] Node {:?} maps back to element {} instead of {}!", 
                            node, mapped_element_id, element_id);
                    }
                } else {
                    eprintln!("[TAFFY_CACHE_ERROR] Node {:?} has no reverse mapping!", node);
                }
                
                self.layout_cache.insert(element_id, *layout);
            }
        }
        Ok(())
    }

    /// Parse Kryon Grid track list (e.g., "1fr 2fr 100px")
    fn parse_grid_track_list(&self, value: &str) -> Vec<TrackSizingFunction> {
        let mut tracks = Vec::new();
        
        // Parser for Kryon grid track syntax
        for token in value.split_whitespace() {
            if token.ends_with("fr") {
                if let Ok(fr_value) = token.trim_end_matches("fr").parse::<f32>() {
                    tracks.push(fr(fr_value));
                }
            } else if token.ends_with("px") {
                if let Ok(px_value) = token.trim_end_matches("px").parse::<f32>() {
                    tracks.push(length(px_value));
                }
            } else if token.ends_with("%") {
                if let Ok(percent_value) = token.trim_end_matches("%").parse::<f32>() {
                    tracks.push(percent(percent_value / 100.0));
                }
            } else if token == "auto" {
                tracks.push(auto());
            } else if token == "min-content" {
                tracks.push(min_content());
            } else if token == "max-content" {
                tracks.push(max_content());
            }
        }
        
        // Default to 1fr if no valid tracks found
        if tracks.is_empty() {
            tracks.push(fr(1.0));
        }
        
        tracks
    }
    
    /// Parse grid template areas
    #[allow(dead_code)]
    fn parse_grid_template_areas(&self, value: &str) -> Vec<Vec<String>> {
        let mut areas = Vec::new();
        
        // Split by lines and parse each row
        for line in value.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            
            // Remove quotes if present
            let line_content = if trimmed.starts_with('"') && trimmed.ends_with('"') {
                &trimmed[1..trimmed.len()-1]
            } else {
                trimmed
            };
            
            // Split by whitespace to get area names
            let row_areas: Vec<String> = line_content
                .split_whitespace()
                .map(|s| s.to_string())
                .collect();
            
            if !row_areas.is_empty() {
                areas.push(row_areas);
            }
        }
        
        areas
    }
    
    /// Parse grid area shorthand (e.g., "2 / 1 / 4 / 3" or "header")
    fn parse_grid_area(&self, value: &str) -> (taffy::GridPlacement, taffy::GridPlacement, taffy::GridPlacement, taffy::GridPlacement) {
        let trimmed = value.trim();
        
        // Check if it's a named area
        if !trimmed.contains('/') {
            // Named area reference - for now, treat as auto since Taffy may not support named areas yet
            let placement = taffy::GridPlacement::Auto;
            return (placement, placement, placement, placement);
        }
        
        // Parse positional values (row-start / column-start / row-end / column-end)
        let parts: Vec<&str> = trimmed.split('/').map(|s| s.trim()).collect();
        
        match parts.len() {
            1 => {
                // Single value applies to all
                let placement = self.parse_grid_line(parts[0]);
                (placement, placement, placement, placement)
            }
            2 => {
                // Two values: row-start/end, column-start/end
                let row_placement = self.parse_grid_line(parts[0]);
                let column_placement = self.parse_grid_line(parts[1]);
                (row_placement, column_placement, row_placement, column_placement)
            }
            3 => {
                // Three values: row-start, column-start/end, row-end
                let row_start = self.parse_grid_line(parts[0]);
                let column_placement = self.parse_grid_line(parts[1]);
                let row_end = self.parse_grid_line(parts[2]);
                (row_start, column_placement, row_end, column_placement)
            }
            4 => {
                // Four values: row-start, column-start, row-end, column-end
                let row_start = self.parse_grid_line(parts[0]);
                let column_start = self.parse_grid_line(parts[1]);
                let row_end = self.parse_grid_line(parts[2]);
                let column_end = self.parse_grid_line(parts[3]);
                (row_start, column_start, row_end, column_end)
            }
            _ => {
                // Invalid format, return auto
                let auto_placement = taffy::GridPlacement::Auto;
                (auto_placement, auto_placement, auto_placement, auto_placement)
            }
        }
    }
    
    /// Parse a single grid line (e.g., "2", "span 3", "auto", "header")
    fn parse_grid_line(&self, value: &str) -> taffy::GridPlacement {
        let trimmed = value.trim();
        
        if trimmed == "auto" {
            return taffy::GridPlacement::Auto;
        }
        
        if trimmed.starts_with("span ") {
            if let Ok(span_count) = trimmed.strip_prefix("span ").unwrap().parse::<u16>() {
                return taffy::GridPlacement::Span(span_count);
            }
        }
        
        if let Ok(line_number) = trimmed.parse::<i16>() {
            return taffy::GridPlacement::Line(line_number.into());
        }
        
        // Try parsing as named line - for now, treat as auto
        taffy::GridPlacement::Auto
    }
    
    /// Parse grid line range (e.g., "1 / 3", "span 2", "header / footer")
    fn parse_grid_line_range(&self, value: &str) -> (taffy::GridPlacement, taffy::GridPlacement) {
        let trimmed = value.trim();
        
        if trimmed.contains('/') {
            let parts: Vec<&str> = trimmed.split('/').map(|s| s.trim()).collect();
            if parts.len() == 2 {
                let start = self.parse_grid_line(parts[0]);
                let end = self.parse_grid_line(parts[1]);
                return (start, end);
            }
        }
        
        // Single value - treat as start, end is auto
        let start = self.parse_grid_line(trimmed);
        (start, taffy::GridPlacement::Auto)
    }

    /// Recursively compute absolute positions by accumulating parent offsets
    fn compute_absolute_positions(
        &self,
        elements: &HashMap<ElementId, Element>,
        element_id: ElementId,
        parent_offset: Vec2,
        computed_positions: &mut HashMap<ElementId, Vec2>,
        computed_sizes: &mut HashMap<ElementId, Vec2>,
    ) {
        if let Some(element) = elements.get(&element_id) {
            if let Some(layout) = self.get_layout(element_id) {
                // Store the computed size
                computed_sizes.insert(element_id, Vec2::new(layout.size.width, layout.size.height));

                // Determine positioning behavior - check both custom_properties and element.position
                let has_position_absolute = element.custom_properties.get("position")
                    .map(|v| if let kryon_core::PropertyValue::String(s) = v { s == "absolute" } else { false })
                    .unwrap_or(false);
                let has_explicit_position = element.position.x != 0.0 || element.position.y != 0.0;

                // Compute absolute position - ALWAYS use layout position for all elements
                let taffy_offset = Vec2::new(layout.location.x, layout.location.y);
                let absolute_position = parent_offset + taffy_offset;
                eprintln!("[TAFFY_LAYOUT] Element {}: layout position parent_offset({}, {}) + taffy_offset({}, {}) = final({}, {})", 
                    element_id, parent_offset.x, parent_offset.y, taffy_offset.x, taffy_offset.y, absolute_position.x, absolute_position.y);

                // Check if element has centering layout that should override absolute positioning
                let has_centering_layout = element.custom_properties.contains_key("justify_content") || 
                                          element.custom_properties.contains_key("align_items");
                
                // If element has explicit positioning, add that to the layout position (unless overridden by centering)
                let final_position = if has_explicit_position && has_position_absolute && !has_centering_layout {
                    let pos_with_offset = absolute_position + Vec2::new(element.position.x, element.position.y);
                    eprintln!("[TAFFY_LAYOUT] Element {}: absolute element, adding offset ({}, {}) = final({}, {})", 
                        element_id, element.position.x, element.position.y, pos_with_offset.x, pos_with_offset.y);
                    pos_with_offset
                } else {
                    if has_explicit_position && has_position_absolute && has_centering_layout {
                        eprintln!("[TAFFY_LAYOUT] Element {}: skipping absolute offset ({}, {}) due to centering layout", 
                            element_id, element.position.x, element.position.y);
                    }
                    absolute_position
                };

                computed_positions.insert(element_id, final_position);

                // For absolute positioned elements, they become the new positioning context for their children
                // For relative/static positioned elements, pass through the current absolute position
                let child_parent_offset = if has_position_absolute {
                    final_position  // Children of absolute elements are positioned relative to the absolute element
                } else {
                    final_position  // Children continue using accumulated absolute position
                };

                // Recursively process children
                for &child_id in &element.children {
                    self.compute_absolute_positions(elements, child_id, child_parent_offset, computed_positions, computed_sizes);
                }
            }
        }
    }
}

impl Default for TaffyLayoutEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl crate::LayoutEngine for TaffyLayoutEngine {
    fn compute_layout(
        &mut self,
        elements: &HashMap<ElementId, Element>,
        root_id: ElementId,
        viewport_size: Vec2,
    ) -> crate::LayoutResult {
        let size = Size {
            width: viewport_size.x,
            height: viewport_size.y,
        };
        
        if let Err(e) = self.compute_taffy_layout(elements, root_id, size) {
            tracing::error!("Taffy layout computation failed: {:?}", e);
            return crate::LayoutResult {
                computed_positions: HashMap::new(),
                computed_sizes: HashMap::new(),
            };
        }

        // Convert Taffy layouts to LayoutResult with proper absolute positioning
        let mut computed_positions = HashMap::new();
        let mut computed_sizes = HashMap::new();

        // Compute absolute positions by traversing hierarchy
        eprintln!("[LAYOUT_DEBUG] Starting compute_absolute_positions for root {}", root_id);
        self.compute_absolute_positions(elements, root_id, Vec2::ZERO, &mut computed_positions, &mut computed_sizes);
        eprintln!("[LAYOUT_DEBUG] compute_absolute_positions completed. Positions: {}, Sizes: {}", computed_positions.len(), computed_sizes.len());

        crate::LayoutResult {
            computed_positions,
            computed_sizes,
        }
    }
}

impl TaffyLayoutEngine {
    /// Determines if an explicit width should be converted to percentage-based width
    /// to ensure proper justify_content behavior in flex containers.
    /// 
    /// This method identifies patterns where explicit width is likely meant to fill
    /// available space (like width matching content area after padding).
    fn should_use_percentage_width(&self, element: &kryon_core::Element, explicit_width: f32) -> bool {
        // Check if this element has justify_content property - if so, it's a flex container
        // where width behavior affects item positioning
        let has_justify_content = element.custom_properties.contains_key("justify_content");
        
        if !has_justify_content {
            return false; // Only apply this fix to flex containers with justify_content
        }
        
        // Check for common patterns where explicit width should be percentage:
        // 1. Round numbers that are likely design widths (600, 700, 800, etc.)
        // 2. Widths that are suspiciously close to common content areas
        let is_round_design_width = explicit_width % 50.0 == 0.0 && explicit_width >= 400.0 && explicit_width <= 1000.0;
        
        // 3. Check if width appears to account for parent padding
        // Common pattern: parent has width X with padding Y, child has width X-2Y
        let looks_like_content_width = matches!(explicit_width as i32, 
            500 | 550 | 600 | 650 | 700 | 750 | 800 | 850 | 900 | 950 | 1000
        );
        
        if is_round_design_width && looks_like_content_width {
            eprintln!("[TAFFY_WIDTH_ANALYSIS] Element '{}': width {}px detected as likely content-filling width", element.id, explicit_width);
            return true;
        }
        
        false
    }
}

// TODO: Future extension for CSS Grid and modern Flexbox properties
// When kryon-compiler supports generating these properties in KRB,
// we can parse them from element.custom_properties and apply to Taffy styles
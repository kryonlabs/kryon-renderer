//! Taffy-based layout engine for Kryon
//! 
//! This module provides modern Grid and Flexbox layout capabilities using Taffy,
//! implementing Kryon's own styling system while maintaining KRB binary compatibility.

use kryon_core::{Element, ElementId};
use glam::Vec2;
use std::collections::HashMap;
use taffy::prelude::*;
use tracing::{debug, trace};

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

    /// Build Taffy tree recursively from KRB elements (OLD METHOD)
    fn build_taffy_tree(
        &mut self,
        elements: &HashMap<ElementId, Element>,
        element_id: ElementId,
    ) -> Result<taffy::NodeId, taffy::TaffyError> {
        let element = elements.get(&element_id)
            .ok_or_else(|| taffy::TaffyError::InvalidChildNode(taffy::NodeId::new(0)))?;

        // Convert KRB properties to Taffy style
        let style = self.krb_to_taffy_style(element);
        
        // eprintln!("[TAFFY_BUILD] Element {}: style={:?}", element_id, style);

        // Create Taffy node
        let node = self.taffy.new_leaf(style)?;
        
        // Store bidirectional mapping
        self.element_to_node.insert(element_id, node);
        self.node_to_element.insert(node, element_id);
        
        eprintln!("[TAFFY_NODE] Element {} -> Taffy Node {:?}", element_id, node);

        // Process children recursively
        let mut child_nodes = Vec::new();
        for &child_id in &element.children {
            let child_node = self.build_taffy_tree(elements, child_id)?;
            child_nodes.push(child_node);
        }

        // Add children to this node
        if !child_nodes.is_empty() {
            self.taffy.set_children(node, &child_nodes)?;
        }

        trace!("Created Taffy node for element {}: {:?}", element_id, element.element_type);
        Ok(node)
    }

    /// Convert kryon-core Element to Taffy Style
    fn krb_to_taffy_style(&self, element: &Element) -> Style {
        let mut style = Style::default();

        // Apply layout flags (legacy system)
        self.apply_legacy_layout_flags(&mut style, element.layout_flags, element);

        // Apply custom properties for modern CSS features
        self.apply_custom_properties(&mut style, element);

        // Apply size constraints from element
        if element.size.x > 0.0 {
            style.size.width = Dimension::Length(element.size.x);
        } else if element.element_type == kryon_core::ElementType::Container {
            // Containers without explicit width should fill available space
            style.size.width = Dimension::Percent(1.0);
            // eprintln!("[TAFFY_CONTAINER] Set container width to 100%");
        }
        if element.size.y > 0.0 {
            style.size.height = Dimension::Length(element.size.y);
        }

        // Apply position from element (for absolute positioning)
        if element.position.x != 0.0 || element.position.y != 0.0 {
            style.position = Position::Absolute;
            style.inset.left = LengthPercentage::Length(element.position.x).into();
            style.inset.top = LengthPercentage::Length(element.position.y).into();
        }

        // Handle text element intrinsic sizing
        if element.element_type == kryon_core::ElementType::Text {
            style.display = Display::Block;
            
            // Always use full width for text elements to ensure consistent alignment
            // This prevents text alignment issues when containers have different sizes
            style.size.width = Dimension::Percent(1.0); // Fill 100% of parent container
            eprintln!("[TAFFY_TEXT_SIZE] Element '{}': using 100% width for consistent alignment", element.id);
            
            // Calculate intrinsic text height if not explicitly set
            if element.size.y == 0.0 {
                let text_height = element.font_size.max(16.0);
                style.size.height = Dimension::Length(text_height);
            }
        } else {
            // Set default display based on element type
            style.display = match element.element_type {
                kryon_core::ElementType::Container => Display::Flex,
                kryon_core::ElementType::Button => Display::Block, // Buttons should be block-level for proper sizing
                _ => Display::Block,
            };
        }
        
        // Override display for specific element types (must be at the end)
        match element.element_type {
            kryon_core::ElementType::Button => {
                eprintln!("[TAFFY_BUTTON_OVERRIDE] Setting button '{}' to Display::Block", element.id);
                style.display = Display::Block;
                // Reset flex properties that might have been set by layout_flags
                style.align_items = None;
                style.justify_content = None;
            }
            kryon_core::ElementType::App | kryon_core::ElementType::Container => {
                // Ensure App and Container elements stay as flex containers if they have flex properties
                if style.align_items.is_some() || style.justify_content.is_some() || element.layout_flags != 0 {
                    eprintln!("[TAFFY_CONTAINER_OVERRIDE] Ensuring '{}' stays Display::Flex", element.id);
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

        if let Some(PropertyValue::String(value)) = element.custom_properties.get("align_items") {
            style.align_items = Some(match value.as_str() {
                "start" | "flex-start" => AlignItems::Start,
                "center" => AlignItems::Center,
                "end" | "flex-end" => AlignItems::End,
                "stretch" => AlignItems::Stretch,
                _ => AlignItems::Start,
            });
        }

        if let Some(PropertyValue::String(value)) = element.custom_properties.get("justify_content") {
            style.justify_content = Some(match value.as_str() {
                "start" | "flex-start" => JustifyContent::Start,
                "center" => JustifyContent::Center,
                "end" | "flex-end" => JustifyContent::End,
                "space-between" => JustifyContent::SpaceBetween,
                "space-around" => JustifyContent::SpaceAround,
                "space-evenly" => JustifyContent::SpaceEvenly,
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
            if grow_value > 0.0 {
                style.flex_grow = grow_value;
                // eprintln!("[TAFFY_FLEX] Applied flex_grow: {} to element", grow_value);
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
    }

    /// Apply legacy layout flags for backward compatibility
    fn apply_legacy_layout_flags(&self, style: &mut Style, layout_flags: u8, element: &Element) {
        // Layout direction constants (matching kryon-compiler types.rs)
        const LAYOUT_DIRECTION_MASK: u8 = 0x03;
        const LAYOUT_DIRECTION_ROW: u8 = 0;
        const LAYOUT_DIRECTION_COLUMN: u8 = 1;
        const LAYOUT_DIRECTION_ABSOLUTE: u8 = 2;

        let direction = layout_flags & LAYOUT_DIRECTION_MASK;
        
        // Only apply flex layout to container elements or elements with explicit layout flags
        let should_apply_flex = layout_flags != 0 || element.element_type == kryon_core::ElementType::Container || element.element_type == kryon_core::ElementType::App;
        
        if should_apply_flex {
            match direction {
                LAYOUT_DIRECTION_ROW => {
                    style.display = Display::Flex;
                    style.flex_direction = FlexDirection::Row;
                }
                LAYOUT_DIRECTION_COLUMN => {
                    style.display = Display::Flex;
                    style.flex_direction = FlexDirection::Column;
                }
                LAYOUT_DIRECTION_ABSOLUTE => {
                    style.position = Position::Absolute;
                    style.display = Display::Block; // Absolute elements need block display
                }
                _ => {
                    // Only set flex for containers when layout_flags is 0
                    if element.element_type == kryon_core::ElementType::Container || element.element_type == kryon_core::ElementType::App {
                        style.display = Display::Flex;
                        style.flex_direction = FlexDirection::Row;
                    }
                }
            }
        }

        // Apply alignment flags - for centering, set both align_items and justify_content
        let alignment = (layout_flags >> 2) & 0x03;
        match alignment {
            1 => {
                style.align_items = Some(AlignItems::Center);
                style.justify_content = Some(JustifyContent::Center);
                eprintln!("[TAFFY_LEGACY] Applied CENTER alignment: layout_flags=0x{:02X}, alignment_bits={}", layout_flags, alignment);
            },
            2 => {
                style.align_items = Some(AlignItems::FlexEnd);
                style.justify_content = Some(JustifyContent::FlexEnd);
            },
            _ => {
                style.align_items = Some(AlignItems::FlexStart);
                style.justify_content = Some(JustifyContent::FlexStart);
            },
        }

        // Apply grow flag - this is crucial for TabBar buttons!
        if layout_flags & 0x20 != 0 {
            style.flex_grow = 1.0;
        }
        
        // Handle button-specific sizing for flex containers  
        if style.flex_grow > 0.0 {
            // Elements with grow should have at least some minimum size
            style.min_size.width = Dimension::Length(80.0); // Minimum button width for flex_grow elements
            style.min_size.height = Dimension::Length(40.0); // Minimum button height
            // eprintln!("[TAFFY_BUTTON] Applied minimum size to flex_grow element: min_width=80, min_height=40");
        }
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

                // Compute absolute position
                let absolute_position = if element.position.x != 0.0 || element.position.y != 0.0 {
                    // Element has explicit position - use it as absolute position
                    let abs_pos = Vec2::new(element.position.x, element.position.y);
                    eprintln!("[TAFFY_LAYOUT] Element {}: explicit position ({}, {})", element_id, abs_pos.x, abs_pos.y);
                    abs_pos
                } else {
                    // Element uses layout positioning - add to parent offset
                    let taffy_offset = Vec2::new(layout.location.x, layout.location.y);
                    let final_pos = parent_offset + taffy_offset;
                    eprintln!("[TAFFY_LAYOUT] Element {}: layout position parent_offset({}, {}) + taffy_offset({}, {}) = final({}, {})", 
                        element_id, parent_offset.x, parent_offset.y, taffy_offset.x, taffy_offset.y, final_pos.x, final_pos.y);
                    final_pos
                };

                computed_positions.insert(element_id, absolute_position);

                // Recursively process children with the new absolute position as their parent offset
                for &child_id in &element.children {
                    self.compute_absolute_positions(elements, child_id, absolute_position, computed_positions, computed_sizes);
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
        self.compute_absolute_positions(elements, root_id, Vec2::ZERO, &mut computed_positions, &mut computed_sizes);

        crate::LayoutResult {
            computed_positions,
            computed_sizes,
        }
    }
}

// TODO: Future extension for CSS Grid and modern Flexbox properties
// When kryon-compiler supports generating these properties in KRB,
// we can parse them from element.custom_properties and apply to Taffy styles
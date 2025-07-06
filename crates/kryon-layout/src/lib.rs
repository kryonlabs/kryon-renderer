// crates/kryon-layout/src/lib.rs

use kryon_core::{Element, ElementId};
use glam::Vec2;
use std::collections::HashMap;
use tracing::debug;

pub mod flexbox;
pub mod constraints;

pub use flexbox::*;
pub use constraints::*;

#[derive(Debug, Clone)]
pub struct LayoutResult {
    pub computed_positions: HashMap<ElementId, Vec2>,
    pub computed_sizes: HashMap<ElementId, Vec2>,
}

pub trait LayoutEngine {
    fn compute_layout(
        &mut self,
        elements: &HashMap<ElementId, Element>,
        root_id: ElementId,
        viewport_size: Vec2,
    ) -> LayoutResult;
}

#[derive(Debug)]
pub struct FlexboxLayoutEngine {
    debug: bool,
}

impl FlexboxLayoutEngine {
    pub fn new() -> Self {
        Self { debug: false }
    }
    
    pub fn with_debug(mut self, debug: bool) -> Self {
        self.debug = debug;
        self
    }
}

impl Default for FlexboxLayoutEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl LayoutEngine for FlexboxLayoutEngine {
    fn compute_layout(
        &mut self,
        elements: &HashMap<ElementId, Element>,
        root_id: ElementId,
        viewport_size: Vec2,
    ) -> LayoutResult {
        let mut result = LayoutResult {
            computed_positions: HashMap::new(),
            computed_sizes: HashMap::new(),
        };
        
        if let Some(root_element) = elements.get(&root_id) {
            // Calculate scaling factor based on designed size vs actual viewport
            // Use the root element's size as the design size (from .kry file)
            let design_size = root_element.size;
            let scale_x = viewport_size.x / design_size.x;
            let scale_y = viewport_size.y / design_size.y;
            let scale_factor = scale_x.min(scale_y); // Use uniform scaling to maintain aspect ratio
            
            if self.debug {
                debug!("Viewport: {:?}, Design: {:?}, Scale: {:.3}", viewport_size, design_size, scale_factor);
            }
            
            let constraints = ConstraintBox {
                min_width: 0.0,
                max_width: viewport_size.x,
                min_height: 0.0,
                max_height: viewport_size.y,
                definite_width: true,
                definite_height: true,
            };
            
            self.layout_element_with_scale(elements, root_id, root_element, constraints, Vec2::ZERO, scale_factor, &mut result);
        }
        
        result
    }
}

impl FlexboxLayoutEngine {
    fn layout_element(
        &self,
        elements: &HashMap<ElementId, Element>,
        element_id: ElementId,
        element: &Element,
        constraints: ConstraintBox,
        offset: Vec2,
        result: &mut LayoutResult,
    ) {
        // Default to no scaling for backward compatibility
        self.layout_element_with_scale(elements, element_id, element, constraints, offset, 1.0, result);
    }
    
    fn layout_element_with_scale(
        &self,
        elements: &HashMap<ElementId, Element>,
        element_id: ElementId,
        element: &Element,
        constraints: ConstraintBox,
        offset: Vec2,
        scale_factor: f32,
        result: &mut LayoutResult,
    ) {
        // Apply scaling to element size and position
        let scaled_size = Vec2::new(
            element.size.x * scale_factor,
            element.size.y * scale_factor
        );
        let scaled_position = Vec2::new(
            element.position.x * scale_factor,
            element.position.y * scale_factor
        );
        
        // Calculate element size with scaling
        let computed_size = self.compute_element_size_scaled(scaled_size, constraints);
        
        // Store computed size and scaled position
        let final_position = scaled_position + offset;
        result.computed_sizes.insert(element_id, computed_size);
        result.computed_positions.insert(element_id, final_position);
        
        if self.debug {
            debug!(
                "Layout element {} (scale={:.3}): size={:?}, pos={:?}",
                element.id,
                scale_factor,
                computed_size,
                final_position
            );
        }
        
        // Layout children if this is a container
        eprintln!("[LAYOUT_ELEMENT] Element {} has {} children", element.id, element.children.len());
        if !element.children.is_empty() {
            eprintln!("[LAYOUT_ELEMENT] Laying out children for element {} at offset {:?}", element.id, final_position);
            self.layout_children_with_scale(elements, element_id, element, computed_size, final_position, scale_factor, result);
        } else {
            eprintln!("[LAYOUT_ELEMENT] No children to layout for element {}", element.id);
        }
    }
    
    fn compute_element_size(&self, element: &Element, constraints: ConstraintBox) -> Vec2 {
        self.compute_element_size_scaled(element.size, constraints)
    }
    
    fn compute_element_size_scaled(&self, size: Vec2, constraints: ConstraintBox) -> Vec2 {
        let mut width = size.x;
        let mut height = size.y;
        
        // Handle auto sizing
        if width == 0.0 {
            width = constraints.max_width;
        }
        if height == 0.0 {
            height = constraints.max_height;
        }
        
        // Apply constraints
        width = width.clamp(constraints.min_width, constraints.max_width);
        height = height.clamp(constraints.min_height, constraints.max_height);
        
        Vec2::new(width, height)
    }
    
    fn layout_children(
        &self,
        elements: &HashMap<ElementId, Element>,
        _parent_id: ElementId,
        parent: &Element,
        parent_size: Vec2,
        parent_offset: Vec2,
        result: &mut LayoutResult,
    ) {
        self.layout_children_with_scale(elements, _parent_id, parent, parent_size, parent_offset, 1.0, result);
    }
    
    fn layout_children_with_scale(
        &self,
        elements: &HashMap<ElementId, Element>,
        _parent_id: ElementId,
        parent: &Element,
        parent_size: Vec2,
        parent_offset: Vec2,
        scale_factor: f32,
        result: &mut LayoutResult,
    ) {
        let layout = LayoutFlags::from_bits(parent.layout_flags);
        eprintln!("[LAYOUT_ENGINE] Parent {} has layout_flags=0x{:02X}, parsed as direction={:?}, alignment={:?}", 
            parent.id, parent.layout_flags, layout.direction, layout.alignment);
        
        eprintln!("[LAYOUT_CHILDREN] Parent {} direction={:?}, alignment={:?}", 
            parent.id, layout.direction, layout.alignment);
            
        match layout.direction {
            LayoutDirection::Row => {
                eprintln!("[LAYOUT_CHILDREN] Using ROW layout for {}", parent.id);
                self.layout_flex_children_with_scale(
                    elements, parent, parent_size, parent_offset, 
                    true, layout, scale_factor, result
                );
            }
            LayoutDirection::Column => {
                eprintln!("[LAYOUT_CHILDREN] Using COLUMN layout for {}", parent.id);
                self.layout_flex_children_with_scale(
                    elements, parent, parent_size, parent_offset,
                    false, layout, scale_factor, result
                );
            }
            LayoutDirection::Absolute => {
                eprintln!("[LAYOUT_CHILDREN] Using ABSOLUTE layout for {}", parent.id);
                self.layout_absolute_children_with_scale(elements, parent, parent_offset, scale_factor, result);
            }
        }
    }
    
    fn layout_flex_children(
        &self,
        elements: &HashMap<ElementId, Element>,
        parent: &Element,
        parent_size: Vec2,
        parent_offset: Vec2,
        is_row: bool,
        layout: LayoutFlags,
        result: &mut LayoutResult,
    ) {
        self.layout_flex_children_with_scale(elements, parent, parent_size, parent_offset, is_row, layout, 1.0, result);
    }
    
fn layout_flex_children_with_scale(
    &self,
    elements: &HashMap<ElementId, Element>,
    parent: &Element,
    parent_size: Vec2,
    parent_offset: Vec2,
    is_row: bool,
    layout: LayoutFlags,
    scale_factor: f32,
    result: &mut LayoutResult,
) {
    eprintln!("[LAYOUT_FLEX_START] Parent={}, is_row={}, children_count={}, parent_offset={:?}",
        parent.id, is_row, parent.children.len(), parent_offset);
    let mut flex_items = Vec::new();
    let mut total_flex_grow = 0.0;
    let mut used_space = 0.0;
    
    // Collect flex items and measure their initial sizes
    for &child_id in &parent.children {
        if let Some(child) = elements.get(&child_id) {
            // NEW: If a child has an explicit position, treat it like an absolute element
            // and do not include it in the flexbox flow calculation.
            if child.position != Vec2::ZERO {
                eprintln!("[LAYOUT_FLEX] Child {} at {:?} has absolute position, skipping flex flow.", child.id, child.position);
                let constraints = ConstraintBox {
                    min_width: 0.0, max_width: f32::INFINITY,
                    min_height: 0.0, max_height: f32::INFINITY,
                    definite_width: false, definite_height: false,
                };
                // Lay it out directly, relative to the parent's final offset.
                self.layout_element_with_scale(elements, child_id, child, constraints, parent_offset, scale_factor, result);
                continue; // Skip to the next child
            }

            // For text elements, compute intrinsic size based on text content
            let is_text_element = child.element_type == kryon_core::ElementType::Text;
            let is_button_element = child.element_type == kryon_core::ElementType::Button;
            
            let intrinsic_size = if is_text_element {
                // Estimate text size: ~8 pixels per character width, font_size height
                let text_width = if !child.text.is_empty() {
                    (child.text.len() as f32 * 8.0).min(parent_size.x * 0.8)
                } else {
                    50.0 // Default width for empty text
                };
                let text_height = child.font_size.max(16.0);
                Vec2::new(text_width, text_height)
            } else if is_button_element {
                // For buttons, first check if explicit size is set from styles
                if child.size.x > 0.0 && child.size.y > 0.0 {
                    // Use explicit size from styles
                    child.size
                } else {
                    // Fall back to text-based sizing for buttons without explicit sizes
                    let child_layout = LayoutFlags::from_bits(child.layout_flags);
                    let button_width = if child.size.x > 0.0 {
                        // Use explicit width from styles
                        child.size.x
                    } else if child_layout.grow {
                        // Minimal width for buttons that need to grow to fill space
                        if !child.text.is_empty() {
                            // Just enough for text without padding
                            (child.text.len() as f32 * 8.0).min(120.0)
                        } else {
                            40.0 // Minimal width for empty button
                        }
                    } else {
                        // Fixed-size buttons: text + padding
                        if !child.text.is_empty() {
                            (child.text.len() as f32 * 8.0) + 32.0
                        } else {
                            80.0 // Default width for button with no text
                        }
                    };
                    let button_height = if child.size.y > 0.0 {
                        // Use explicit height from styles
                        child.size.y
                    } else {
                        32.0 // Default height for buttons without explicit height
                    };
                    Vec2::new(button_width, button_height)
                }
            } else {
                // For other elements (containers, etc.)
                let is_container = child.element_type == kryon_core::ElementType::Container;
                
                if child.size.x > 0.0 || child.size.y > 0.0 {
                    // Use explicit sizes if set
                    Vec2::new(
                        if child.size.x > 0.0 { child.size.x } else { 
                            if is_container { parent_size.x } else { 100.0 }
                        },
                        if child.size.y > 0.0 { child.size.y } else { 100.0 }
                    )
                } else {
                    // Default intrinsic sizes
                    if is_container {
                        // Containers take full parent width by default
                        Vec2::new(parent_size.x, 100.0)
                    } else {
                        // Other elements use fixed defaults
                        Vec2::new(100.0, 100.0)
                    }
                }
            };
            
            eprintln!("[FLEX_SIZE] Child {}: is_text={}, intrinsic_size={:?}", 
                child.id, is_text_element, intrinsic_size);
            let flex_basis = if is_row { intrinsic_size.x } else { intrinsic_size.y };
            let child_layout = LayoutFlags::from_bits(child.layout_flags);
            let flex_grow = if child_layout.grow { 1.0 } else { 0.0 };
            eprintln!("[FLEX_GROW] Child {}: child_layout.grow={}, flex_grow={}, layout_flags=0x{:02X}", 
                child.id, child_layout.grow, flex_grow, child.layout_flags);
            
            flex_items.push(FlexItem {
                element_id: child_id,
                flex_basis,
                flex_grow,
                flex_shrink: 1.0,
                main_axis_size: flex_basis,
                cross_axis_size: if is_row { intrinsic_size.y } else { intrinsic_size.x },
            });
            
            used_space += flex_basis;
            total_flex_grow += flex_grow;
        }
    }
    
    // Distribute remaining space
    let container_main_size = if is_row { parent_size.x } else { parent_size.y };
    let remaining_space = (container_main_size - used_space).max(0.0);
    
    eprintln!("[FLEX_GROW_TOTAL] total_flex_grow={}, remaining_space={}, container_main_size={}, used_space={}", 
        total_flex_grow, remaining_space, container_main_size, used_space);
    
    if remaining_space > 0.0 && total_flex_grow > 0.0 {
        for item in &mut flex_items {
            if item.flex_grow > 0.0 {
                let grow_amount = (item.flex_grow / total_flex_grow) * remaining_space;
                item.main_axis_size += grow_amount;
            }
        }
    }
    
    // Position elements
    // For center alignment with single child, center on main axis too
    let mut current_position = if layout.alignment == LayoutAlignment::Center && flex_items.len() == 1 {
        // Center the single item on main axis
        (container_main_size - flex_items[0].main_axis_size) / 2.0
    } else {
        0.0
    };
    let gap = 0.0; // TODO: Add gap support
    
    eprintln!("[LAYOUT_FLEX] Positioning {} flex items, remaining_space={}, is_row={}, start_position={}", 
        flex_items.len(), remaining_space, is_row, current_position);
    
    for (i, item) in flex_items.iter().enumerate() {
        if let Some(child) = elements.get(&item.element_id) {
            let cross_axis_pos = if is_row {
                self.compute_cross_axis_position(item.cross_axis_size, parent_size.y, layout.alignment)
            } else {
                self.compute_cross_axis_position(item.cross_axis_size, parent_size.x, layout.alignment)
            };
            
            let child_position = if is_row {
                Vec2::new(current_position, cross_axis_pos)
            } else {
                Vec2::new(cross_axis_pos, current_position)
            };
            
            eprintln!("[LAYOUT_FLEX] Child {}: main_axis={}, cross_axis={}, cross_axis_pos={}, relative_pos={:?}", 
                child.id, current_position, item.cross_axis_size, cross_axis_pos, child_position);
            
            let child_size = if is_row {
                Vec2::new(item.main_axis_size, item.cross_axis_size)
            } else {
                Vec2::new(item.cross_axis_size, item.main_axis_size)
            };
            
            let child_constraints = ConstraintBox {
                min_width: child_size.x,
                max_width: child_size.x,
                min_height: child_size.y,
                max_height: child_size.y,
                definite_width: true,
                definite_height: true,
            };
            
            // Update child position to be relative to parent
            let mut updated_child = child.clone();
            updated_child.position = child_position;
            
            self.layout_element_with_scale(
                elements,
                item.element_id,
                &updated_child,
                child_constraints,
                parent_offset,
                scale_factor,
                result,
            );
            
            current_position += item.main_axis_size;
            if i < flex_items.len() - 1 {
                current_position += gap;
            }
        }
    }
}
    
    fn layout_absolute_children(
        &self,
        elements: &HashMap<ElementId, Element>,
        parent: &Element,
        parent_offset: Vec2,
        result: &mut LayoutResult,
    ) {
        self.layout_absolute_children_with_scale(elements, parent, parent_offset, 1.0, result);
    }
    
    fn layout_absolute_children_with_scale(
        &self,
        elements: &HashMap<ElementId, Element>,
        parent: &Element,
        parent_offset: Vec2,
        scale_factor: f32,
        result: &mut LayoutResult,
    ) {
        for &child_id in &parent.children {
            if let Some(child) = elements.get(&child_id) {
                let constraints = ConstraintBox {
                    min_width: 0.0,
                    max_width: f32::INFINITY,
                    min_height: 0.0,
                    max_height: f32::INFINITY,
                    definite_width: false,
                    definite_height: false,
                };
                
                self.layout_element_with_scale(
                    elements,
                    child_id,
                    child,
                    constraints,
                    parent_offset,
                    scale_factor,
                    result,
                );
            }
        }
    }
    
    fn compute_cross_axis_position(&self, child_size: f32, parent_size: f32, alignment: LayoutAlignment) -> f32 {
        let result = match alignment {
            LayoutAlignment::Start => 0.0,
            LayoutAlignment::Center => (parent_size - child_size) / 2.0,
            LayoutAlignment::End => parent_size - child_size,
            LayoutAlignment::SpaceBetween => 0.0, // Not applicable for cross axis
        };
        eprintln!("[CROSS_AXIS] child_size={}, parent_size={}, alignment={:?} -> position={}", 
            child_size, parent_size, alignment, result);
        result
    }
}
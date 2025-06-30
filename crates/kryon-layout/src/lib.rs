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
            let constraints = ConstraintBox {
                min_width: 0.0,
                max_width: viewport_size.x,
                min_height: 0.0,
                max_height: viewport_size.y,
                definite_width: true,
                definite_height: true,
            };
            
            self.layout_element(elements, root_id, root_element, constraints, Vec2::ZERO, &mut result);
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
        // Calculate element size
        let computed_size = self.compute_element_size(element, constraints);
        
        // Store computed size and position
        result.computed_sizes.insert(element_id, computed_size);
        result.computed_positions.insert(element_id, element.position + offset);
        
        if self.debug {
            debug!(
                "Layout element {}: size={:?}, pos={:?}",
                element.id,
                computed_size,
                element.position + offset
            );
        }
        
        // Layout children if this is a container
        if !element.children.is_empty() {
            self.layout_children(elements, element_id, element, computed_size, offset, result);
        }
    }
    
    fn compute_element_size(&self, element: &Element, constraints: ConstraintBox) -> Vec2 {
        let mut width = element.size.x;
        let mut height = element.size.y;
        
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
        let layout = LayoutFlags::from_bits(parent.layout_flags);
        
        match layout.direction {
            LayoutDirection::Row => {
                self.layout_flex_children(
                    elements, parent, parent_size, parent_offset, 
                    true, layout, result
                );
            }
            LayoutDirection::Column => {
                self.layout_flex_children(
                    elements, parent, parent_size, parent_offset,
                    false, layout, result
                );
            }
            LayoutDirection::Absolute => {
                self.layout_absolute_children(elements, parent, parent_offset, result);
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
        let mut flex_items = Vec::new();
        let mut total_flex_grow = 0.0;
        let mut used_space = 0.0;
        
        // Collect flex items and measure their initial sizes
        for &child_id in &parent.children {
            if let Some(child) = elements.get(&child_id) {
                let constraints = ConstraintBox {
                    min_width: 0.0,
                    max_width: if is_row { f32::INFINITY } else { parent_size.x },
                    min_height: 0.0,
                    max_height: if is_row { parent_size.y } else { f32::INFINITY },
                    definite_width: !is_row,
                    definite_height: is_row,
                };
                
                let intrinsic_size = self.compute_element_size(child, constraints);
                let flex_basis = if is_row { intrinsic_size.x } else { intrinsic_size.y };
                let flex_grow = if layout.grow { 1.0 } else { 0.0 };
                
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
        
        if remaining_space > 0.0 && total_flex_grow > 0.0 {
            for item in &mut flex_items {
                if item.flex_grow > 0.0 {
                    let grow_amount = (item.flex_grow / total_flex_grow) * remaining_space;
                    item.main_axis_size += grow_amount;
                }
            }
        }
        
        // Position elements
        let mut current_position = 0.0;
        let gap = 0.0; // TODO: Add gap support
        
        for (i, item) in flex_items.iter().enumerate() {
            if let Some(child) = elements.get(&item.element_id) {
                let child_position = if is_row {
                    Vec2::new(current_position, self.compute_cross_axis_position(
                        item.cross_axis_size, parent_size.y, layout.alignment
                    ))
                } else {
                    Vec2::new(self.compute_cross_axis_position(
                        item.cross_axis_size, parent_size.x, layout.alignment
                    ), current_position)
                };
                
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
                
                self.layout_element(
                    elements,
                    item.element_id,
                    &updated_child,
                    child_constraints,
                    parent_offset + parent.position,
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
                
                self.layout_element(
                    elements,
                    child_id,
                    child,
                    constraints,
                    parent_offset + parent.position,
                    result,
                );
            }
        }
    }
    
    fn compute_cross_axis_position(&self, child_size: f32, parent_size: f32, alignment: LayoutAlignment) -> f32 {
        match alignment {
            LayoutAlignment::Start => 0.0,
            LayoutAlignment::Center => (parent_size - child_size) / 2.0,
            LayoutAlignment::End => parent_size - child_size,
            LayoutAlignment::SpaceBetween => 0.0, // Not applicable for cross axis
        }
    }
}
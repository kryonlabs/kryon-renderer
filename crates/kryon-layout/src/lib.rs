// crates/kryon-layout/src/lib.rs

use kryon_core::{Element, ElementId};
use glam::Vec2;
use std::collections::HashMap;

pub mod flexbox;
pub mod constraints;
pub mod taffy_engine;

pub use flexbox::{LayoutFlags, LayoutDirection, LayoutAlignment};
pub use constraints::*;
pub use taffy_engine::TaffyLayoutEngine;

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

// Legacy FlexboxLayoutEngine removed - use TaffyLayoutEngine instead
// FlexboxLayoutEngine struct and implementation (~500 lines) removed in Phase 1 Week 1-2

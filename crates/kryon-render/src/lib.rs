// crates/kryon-render/src/lib.rs

use anyhow::Result;
use kryon_core::{
    Element, ElementId, ElementType, TextAlignment,
    StyleComputer, ComputedStyle
};
use kryon_layout::LayoutResult;
use glam::{Vec2, Vec4};
use std::collections::HashMap;

pub mod events;
pub use events::*;

#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("Renderer initialization failed: {0}")]
    InitializationFailed(String),
    #[error("Render operation failed: {0}")]
    RenderFailed(String),
    #[error("Resource not found: {0}")]
    ResourceNotFound(String),
    #[error("Unsupported operation: {0}")]
    UnsupportedOperation(String),
}

pub type RenderResult<T> = std::result::Result<T, RenderError>;

/// Core rendering trait that all backends must implement
pub trait Renderer {
    type Surface;
    type Context;

    /// Initialize the renderer with the given surface
    fn initialize(surface: Self::Surface) -> RenderResult<Self> where Self: Sized;

    /// Begin a new frame
    fn begin_frame(&mut self, clear_color: Vec4) -> RenderResult<Self::Context>;

    /// End the current frame and present it
    fn end_frame(&mut self, context: Self::Context) -> RenderResult<()>;

    /// Render a single element
    fn render_element(
        &mut self,
        context: &mut Self::Context,
        element: &Element,
        layout: &LayoutResult,
        element_id: ElementId,
    ) -> RenderResult<()>;

    /// Handle window resize
    fn resize(&mut self, new_size: Vec2) -> RenderResult<()>;

    /// Get current viewport size
    fn viewport_size(&self) -> Vec2;
}

/// High-level rendering commands
#[derive(Debug, Clone)]
pub enum RenderCommand {
    DrawRect {
        position: Vec2,
        size: Vec2,
        color: Vec4,
        border_radius: f32,
        border_width: f32,
        border_color: Vec4,
    },
    DrawText {
        position: Vec2,
        text: String,
        font_size: f32,
        color: Vec4,
        alignment: TextAlignment,
        max_width: Option<f32>,
    },
    DrawImage {
        position: Vec2,
        size: Vec2,
        source: String,
        opacity: f32,
    },
    SetClip {
        position: Vec2,
        size: Vec2,
    },
    ClearClip,
}

/// Trait for backends that use command-based rendering
pub trait CommandRenderer: Renderer {
    /// Execute a batch of render commands
    fn execute_commands(
        &mut self,
        context: &mut Self::Context,
        commands: &[RenderCommand],
    ) -> RenderResult<()>;
}


pub struct ElementRenderer<R: CommandRenderer> {
    backend: R,
    style_computer: StyleComputer, 
    viewport_size: Vec2,
}



impl<R: CommandRenderer> ElementRenderer<R> {
    pub fn new(backend: R, style_computer: StyleComputer) -> Self {
        let viewport_size = backend.viewport_size();
        Self {
            backend,
            style_computer,
            viewport_size,
        }
    }

    pub fn render_frame(
        &mut self,
        elements: &HashMap<ElementId, Element>,
        layout: &LayoutResult,
        root_id: ElementId,
        clear_color: Vec4,
    ) -> RenderResult<()> {
        let mut context = self.backend.begin_frame(clear_color)?;
        if let Some(root_element) = elements.get(&root_id) {
            self.render_element_tree(&mut context, elements, layout, root_id, root_element)?;
        }
        self.backend.end_frame(context)?;
        Ok(())
    }

    fn render_element_tree(
        &mut self,
        context: &mut R::Context,
        elements: &HashMap<ElementId, Element>,
        layout: &LayoutResult,
        element_id: ElementId,
        element: &Element,
    ) -> RenderResult<()> {
        if !element.visible {
            // If the element is not visible, it would stop here.
            return Ok(());
        }

        let commands = self.element_to_commands(element, layout, element_id)?;

        println!("[ElementRenderer] For element ID {}, generated {} commands.", element_id, commands.len());

        self.backend.execute_commands(context, &commands)?;

        for &child_id in &element.children {
            if let Some(child_element) = elements.get(&child_id) {
                self.render_element_tree(context, elements, layout, child_id, child_element)?;
            }
        }
        Ok(())
    }

    fn element_to_commands(
        &self,
        element: &Element, // The fix is to USE this parameter...
        layout: &LayoutResult,
        element_id: ElementId,
    ) -> RenderResult<Vec<RenderCommand>> {
        let mut commands = Vec::new();

        // Compute the final style based on inheritance
        let style = self.style_computer.compute(element_id);
        
        // ... instead of calling a non-existent method.
        // let element = self.style_computer.get_element(element_id).unwrap(); // <-- REMOVE THIS LINE

        // For elements with explicit positions in KRB (non-zero), use those instead of layout
        let position = if element.position != Vec2::ZERO {
            // Element has explicit position in KRB - use it (fixed positioning)
            element.position
        } else {
            // Element has no explicit position - use layout computed position
            layout.computed_positions.get(&element_id).copied().unwrap_or(element.position)
        };
        
        let size = layout.computed_sizes.get(&element_id).copied().unwrap_or(element.size);
        
        println!("[POSITION] Element {}: KRB pos={:?}, Layout pos={:?}, Final pos={:?}", 
            element_id, element.position, layout.computed_positions.get(&element_id), position);

        // Now, use the perfectly computed style values!
        let mut bg_color = style.background_color;
        bg_color.w *= element.opacity;

        let mut border_width = style.border_width;
        let mut border_color = style.border_color;
        border_color.w *= element.opacity;
        
        println!("[element_to_commands] Element {}: bg={:?}, border={:?}, border_width={}, type={:?}",
            element.id, bg_color, border_color, border_width, element.element_type);

        if border_width == 0.0 && border_color.w > 0.0 {
            border_width = 1.0;
        }

        if bg_color.w > 0.0 || border_width > 0.0 {
            commands.push(RenderCommand::DrawRect {
                position, size, color: bg_color,
                border_radius: style.border_radius,
                border_width,
                border_color,
            });
        }

        if !element.text.is_empty() {
            let mut text_color = style.text_color;
            text_color.w *= element.opacity;

            if text_color.w > 0.0 {
                // Calculate text position based on alignment
                let text_position = match element.text_alignment {
                    TextAlignment::Center => {
                        // Center the text in the element
                        Vec2::new(
                            position.x + size.x / 2.0,
                            position.y + size.y / 2.0 - element.font_size.max(16.0) / 2.0
                        )
                    }
                    TextAlignment::Start => position + Vec2::new(5.0, 5.0),
                    TextAlignment::End => Vec2::new(
                        position.x + size.x - 5.0,
                        position.y + 5.0
                    ),
                    TextAlignment::Justify => position + Vec2::new(5.0, 5.0),
                };
                
                commands.push(RenderCommand::DrawText {
                    position: text_position,
                    text: element.text.clone(),
                    font_size: element.font_size.max(16.0),
                    color: text_color,
                    alignment: element.text_alignment,
                    max_width: Some(size.x - 10.0),
                });
            }
        }

        Ok(commands)
    }
    pub fn resize(&mut self, new_size: Vec2) -> RenderResult<()> {
        self.viewport_size = new_size;
        self.backend.resize(new_size)
    }
    
    // --- THIS IS THE NEW FUNCTION THAT FIXES THE ERROR ---
    pub fn viewport_size(&self) -> Vec2 {
        self.viewport_size
    }
    
    // It's also good practice to provide access to the underlying backend
    pub fn backend(&self) -> &R {
        &self.backend
    }

    pub fn backend_mut(&mut self) -> &mut R {
        &mut self.backend
    }
}


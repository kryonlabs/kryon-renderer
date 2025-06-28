// crates/kryon-render/src/lib.rs
use kryon_core::{Element, ElementId, ElementType, InteractionState};
use kryon_layout::LayoutResult;
use glam::{Vec2, Vec4};
use std::collections::HashMap;

pub mod primitives;
pub mod text;
pub mod events;

pub use primitives::*;
pub use text::*;
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

pub type RenderResult<T> = Result<T, RenderError>;

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

/// High-level renderer that converts elements to commands
pub struct ElementRenderer<R: CommandRenderer> {
    backend: R,
    viewport_size: Vec2,
}

impl<R: CommandRenderer> ElementRenderer<R> {
    pub fn new(backend: R) -> Self {
        let viewport_size = backend.viewport_size();
        Self {
            backend,
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
            self.render_element_tree(
                &mut context,
                elements,
                layout,
                root_id,
                root_element,
            )?;
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
            return Ok(());
        }
        
        // Generate render commands for this element
        let commands = self.element_to_commands(element, layout, element_id)?;
        
        // Execute commands
        self.backend.execute_commands(context, &commands)?;
        
        // Render children
        for &child_id in &element.children {
            if let Some(child_element) = elements.get(&child_id) {
                self.render_element_tree(
                    context,
                    elements,
                    layout,
                    child_id,
                    child_element,
                )?;
            }
        }
        
        Ok(())
    }
    
    fn element_to_commands(
        &self,
        element: &Element,
        layout: &LayoutResult,
        element_id: ElementId,
    ) -> RenderResult<Vec<RenderCommand>> {
        let mut commands = Vec::new();
        
        let position = layout.computed_positions.get(&element_id)
            .copied()
            .unwrap_or(element.position);
        let size = layout.computed_sizes.get(&element_id)
            .copied()
            .unwrap_or(element.size);
        
        // Apply opacity
        let mut bg_color = element.background_color;
        let mut text_color = element.text_color;
        let mut border_color = element.border_color;
        
        bg_color.w *= element.opacity;
        text_color.w *= element.opacity;
        border_color.w *= element.opacity;
        
        // Background rectangle
        if bg_color.w > 0.0 || element.border_width > 0.0 {
            commands.push(RenderCommand::DrawRect {
                position,
                size,
                color: bg_color,
                border_radius: element.border_radius,
                border_width: element.border_width,
                border_color,
            });
        }
        
        // Element-specific content
        match element.element_type {
            ElementType::Text | ElementType::Button => {
                if !element.text.is_empty() && text_color.w > 0.0 {
                    commands.push(RenderCommand::DrawText {
                        position: position + Vec2::new(4.0, 4.0), // Basic padding
                        text: element.text.clone(),
                        font_size: element.font_size,
                        color: text_color,
                        alignment: element.text_alignment.into(),
                        max_width: Some(size.x - 8.0),
                    });
                }
            }
            ElementType::Image => {
                if let Some(src) = element.custom_properties.get("src") {
                    if let Some(source) = src.as_string() {
                        commands.push(RenderCommand::DrawImage {
                            position,
                            size,
                            source: source.to_string(),
                            opacity: element.opacity,
                        });
                    }
                }
            }
            ElementType::Input => {
                // Draw input background and text
                let input_bg = Vec4::new(1.0, 1.0, 1.0, 1.0); // White background
                commands.push(RenderCommand::DrawRect {
                    position,
                    size,
                    color: input_bg,
                    border_radius: element.border_radius,
                    border_width: element.border_width.max(1.0),
                    border_color: if border_color.w > 0.0 { border_color } else { Vec4::new(0.8, 0.8, 0.8, 1.0) },
                });
                
                let display_text = if element.text.is_empty() {
                    element.custom_properties.get("placeholder")
                        .and_then(|p| p.as_string())
                        .unwrap_or("")
                } else {
                    &element.text
                };
                
                if !display_text.is_empty() {
                    let text_color = if element.text.is_empty() {
                        Vec4::new(0.6, 0.6, 0.6, 1.0) // Gray for placeholder
                    } else {
                        text_color
                    };
                    
                    commands.push(RenderCommand::DrawText {
                        position: position + Vec2::new(8.0, 4.0),
                        text: display_text.to_string(),
                        font_size: element.font_size,
                        color: text_color,
                        alignment: TextAlignment::Start,
                        max_width: Some(size.x - 16.0),
                    });
                }
            }
            _ => {
                // Other element types don't have specific rendering
            }
        }
        
        Ok(commands)
    }
    
    pub fn resize(&mut self, new_size: Vec2) -> RenderResult<()> {
        self.viewport_size = new_size;
        self.backend.resize(new_size)
    }
    
    pub fn viewport_size(&self) -> Vec2 {
        self.viewport_size
    }
}
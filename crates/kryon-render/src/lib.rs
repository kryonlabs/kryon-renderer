use glam::{Vec2, Vec4};
use std::collections::HashMap;
// use tracing::info; // No longer needed

use kryon_core::{Element, ElementId, ElementType, PropertyValue, StyleComputer, TextAlignment, TransformData};
use kryon_layout::LayoutResult;

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

/// Core rendering trait that all backends must implement.
pub trait Renderer {
    type Surface;
    type Context;

    fn initialize(surface: Self::Surface) -> RenderResult<Self>
    where
        Self: Sized;
    fn begin_frame(&mut self, clear_color: Vec4) -> RenderResult<Self::Context>;
    fn end_frame(&mut self, context: Self::Context) -> RenderResult<()>;
    fn render_element(
        &mut self,
        context: &mut Self::Context,
        element: &Element,
        layout: &LayoutResult,
        element_id: ElementId,
    ) -> RenderResult<()>;
    fn resize(&mut self, new_size: Vec2) -> RenderResult<()>;
    fn viewport_size(&self) -> Vec2;
}

/// High-level rendering commands for backends that use them.
#[derive(Debug, Clone)]
pub enum RenderCommand {
    DrawRect {
        position: Vec2,
        size: Vec2,
        color: Vec4,
        border_radius: f32,
        border_width: f32,
        border_color: Vec4,
        transform: Option<TransformData>,
    },
    DrawText {
        position: Vec2,
        text: String,
        font_size: f32,
        color: Vec4,
        alignment: TextAlignment,
        max_width: Option<f32>,
        max_height: Option<f32>,
        transform: Option<TransformData>,
    },
    DrawImage {
        position: Vec2,
        size: Vec2,
        source: String,
        opacity: f32,
        transform: Option<TransformData>,
    },
    SetClip {
        position: Vec2,
        size: Vec2,
    },
    ClearClip,
    /// Informs the renderer of the application's intended canvas size.
    SetCanvasSize(Vec2),
}

/// Trait for backends that use command-based rendering.
pub trait CommandRenderer: Renderer {
    fn execute_commands(
        &mut self,
        context: &mut Self::Context,
        commands: &[RenderCommand],
    ) -> RenderResult<()>;
    
    /// Set the mouse cursor type (optional - some backends may not support this)
    fn set_cursor(&mut self, _cursor_type: kryon_core::CursorType) {
        // Default implementation does nothing
    }
}

/// The bridge between the scene graph and the rendering backend.
/// It translates elements and layout into a stream of `RenderCommand`s.
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

    /// Renders a complete frame by generating and executing a single batch of commands.
    pub fn render_frame(
        &mut self,
        elements: &HashMap<ElementId, Element>,
        layout: &LayoutResult,
        root_id: ElementId,
        clear_color: Vec4,
    ) -> RenderResult<()> {
        let mut context = self.backend.begin_frame(clear_color)?;

        if let Some(root_element) = elements.get(&root_id) {
            let mut all_commands = Vec::new();

            // Use the root element's size as defined in the KRB file for the canvas.
            let canvas_size = root_element.size;
            if canvas_size.x > 0.0 && canvas_size.y > 0.0 {
                all_commands.push(RenderCommand::SetCanvasSize(canvas_size));
            }

            // Recursively fill the command list from the element tree.
            self.collect_render_commands(&mut all_commands, elements, layout, root_id, root_element)?;

            self.backend.execute_commands(&mut context, &all_commands)?;
        }

        self.backend.end_frame(context)?;
        Ok(())
    }

    /// Recursively traverses the element tree and appends drawing commands to a list.
    fn collect_render_commands(
        &self,
        all_commands: &mut Vec<RenderCommand>,
        elements: &HashMap<ElementId, Element>,
        layout: &LayoutResult,
        element_id: ElementId,
        element: &Element,
    ) -> RenderResult<()> {
        if !element.visible {
            return Ok(());
        }

        // Generate commands for the current element and append them.
        let mut element_commands = self.element_to_commands(element, layout, element_id)?;
        all_commands.append(&mut element_commands);

        // Recurse for children.
        for &child_id in &element.children {
            if let Some(child_element) = elements.get(&child_id) {
                self.collect_render_commands(all_commands, elements, layout, child_id, child_element)?;
            }
        }
        Ok(())
    }

    /// Translates a single element into one or more `RenderCommand`s.
    /// This function is the heart of the renderer logic.
    fn element_to_commands(
        &self,
        element: &Element,
        layout: &LayoutResult,
        element_id: ElementId,
    ) -> RenderResult<Vec<RenderCommand>> {
        let mut commands = Vec::new();

        // Get the final computed style for the element using its current interaction state.
        let style = self.style_computer.compute_with_state(element_id, element.current_state);

        // Get the position and size FROM THE LAYOUT ENGINE. This is the single source of truth.
        let Some(position) = layout.computed_positions.get(&element_id).copied() else {
            return Ok(commands); // Element not positioned by layout, so it can't be drawn.
        };
        let Some(size) = layout.computed_sizes.get(&element_id).copied() else {
            return Ok(commands); // Element has no size, so it can't be drawn.
        };
        
        // Draw the background/border rectangle.
        let mut bg_color = style.background_color;
        bg_color.w *= element.opacity;

        let border_width = style.border_width;
        let mut border_color = style.border_color;
        border_color.w *= element.opacity;

        // Check if element has transform data
        let transform = element.custom_properties.get("transform_index")
            .and_then(|v| v.as_int())
            .and_then(|index| {
                // TODO: Get transform data from KRB file transforms array
                // For now, return None until we have access to the transforms
                None
            });
        
        if bg_color.w > 0.0 || border_width > 0.0 {
            commands.push(RenderCommand::DrawRect {
                position,
                size,
                color: bg_color,
                border_radius: style.border_radius,
                border_width,
                border_color,
                transform: transform.clone(),
            });
        }

        // Draw the text, if any.
        if !element.text.is_empty() {
            let mut text_color = style.text_color;
            text_color.w *= element.opacity;

            if text_color.w > 0.0 {
                // The position for the text block is the same as the element's bounding box.
                // The renderer backend (e.g., Ratatui) will handle alignment within that box.
                eprintln!("[RENDER_TEXT] Element {}: text='{}', alignment={:?}, size={:?}", 
                    element.id, element.text, element.text_alignment, size);
                commands.push(RenderCommand::DrawText {
                    position, // Use the element's top-left corner.
                    text: element.text.clone(),
                    font_size: element.font_size,
                    color: text_color,
                    alignment: element.text_alignment,
                    max_width: Some(size.x), // The max width is the element's full width.
                    max_height: Some(size.y), // The max height is the element's full height.
                    transform: transform.clone(),
                });
            }
        }
        
        // Draw images for Image elements
        if element.element_type == ElementType::Image {
            if let Some(src_property) = element.custom_properties.get("src") {
                if let PropertyValue::String(image_source) = src_property {
                    commands.push(RenderCommand::DrawImage {
                        position,
                        size,
                        source: image_source.clone(),
                        opacity: element.opacity,
                        transform: transform.clone(),
                    });
                }
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

    pub fn backend(&self) -> &R {
        &self.backend
    }

    pub fn backend_mut(&mut self) -> &mut R {
        &mut self.backend
    }
}
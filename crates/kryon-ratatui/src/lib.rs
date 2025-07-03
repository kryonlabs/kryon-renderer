// crates/kryon-ratatui/src/lib.rs

use kryon_core::{Element, ElementId, TextAlignment};
use kryon_layout::LayoutResult;
use kryon_render::{
    CommandRenderer, RenderCommand, RenderError, RenderResult, Renderer,
};

use ratatui::{
    backend::Backend,
    layout::{Alignment, Rect},
    style::{Color, Style},
    widgets::{Block, Clear, Paragraph},
    Frame, Terminal,
};

use glam::{Vec2, Vec4};

pub struct RatatuiRenderer<B: Backend> {
    pub terminal: Terminal<B>,
    // --- FIX: Add a field to store the source canvas size ---
    // This will hold the intended size of the app (e.g., 800x600)
    source_size: Vec2,
}

pub struct RatatuiContext;

// --- FIX: Add a helper function to translate coordinates ---
fn translate_rect(source_pos: Vec2, source_size: Vec2, app_canvas_size: Vec2, terminal_area: Rect) -> Option<Rect> {
    // Prevent division by zero if the app canvas size is invalid.
    if app_canvas_size.x == 0.0 || app_canvas_size.y == 0.0 {
        return None;
    }

    // 1. Calculate relative position and size.
    let rel_x = source_pos.x / app_canvas_size.x;
    let rel_y = source_pos.y / app_canvas_size.y;
    let rel_w = source_size.x / app_canvas_size.x;
    let rel_h = source_size.y / app_canvas_size.y;

    // 2. Apply relative dimensions to the terminal's character grid.
    let target_w_f32 = terminal_area.width as f32;
    let target_h_f32 = terminal_area.height as f32;

    let term_x = (rel_x * target_w_f32).floor() as u16;
    let term_y = (rel_y * target_h_f32).floor() as u16;
    let term_w = (rel_w * target_w_f32).ceil() as u16;
    let term_h = (rel_h * target_h_f32).ceil() as u16;

    // 3. Create the final Ratatui Rect and clamp it to the terminal bounds.
    let final_x = term_x.min(terminal_area.right());
    let final_y = term_y.min(terminal_area.bottom());
    let final_w = term_w.min(terminal_area.width.saturating_sub(final_x));
    let final_h = term_h.min(terminal_area.height.saturating_sub(final_y));

    let final_rect = Rect::new(final_x, final_y, final_w, final_h);
    
    // Only return a rect if it has a drawable area.
    if final_rect.width > 0 && final_rect.height > 0 {
        Some(final_rect)
    } else {
        None
    }
}


impl<B: Backend> Renderer for RatatuiRenderer<B> {
    type Surface = B;
    type Context = RatatuiContext;

    fn initialize(surface: Self::Surface) -> RenderResult<Self>
    where
        Self: Sized,
    {
        let terminal = Terminal::new(surface)
            .map_err(|e| RenderError::InitializationFailed(e.to_string()))?;
        Ok(Self {
            terminal,
            // --- FIX: Initialize source_size with a default ---
            source_size: Vec2::new(800.0, 600.0), // Default canvas size
        })
    }

    fn begin_frame(&mut self, _clear_color: Vec4) -> RenderResult<Self::Context> {
        Ok(RatatuiContext)
    }

    fn end_frame(&mut self, _context: Self::Context) -> RenderResult<()> {
        // This seems to be for flushing the buffer, which `draw` does automatically.
        // It's okay for it to be minimal.
        self.terminal.draw(|_frame| {}).map_err(|e| RenderError::RenderFailed(e.to_string()))?;
        Ok(())
    }

    fn render_element( &mut self, _c: &mut Self::Context, _: &Element, _: &LayoutResult, _: ElementId) -> RenderResult<()> { Ok(()) }

    fn resize(&mut self, new_size: Vec2) -> RenderResult<()> {
        self.terminal.resize(Rect::new(0, 0, new_size.x as u16, new_size.y as u16))
            .map_err(|e| RenderError::RenderFailed(format!("Terminal resize failed: {}", e)))
    }

    fn viewport_size(&self) -> Vec2 {
        let size = self.terminal.size().unwrap_or_default();
        Vec2::new(size.width as f32, size.height as f32)
    }
}

impl<B: Backend> CommandRenderer for RatatuiRenderer<B> {
    fn execute_commands(&mut self, _: &mut Self::Context, commands: &[RenderCommand]) -> RenderResult<()> {
        // --- FIX: The draw closure needs access to `self` to get the source_size ---
        let source_size = &mut self.source_size;

        self.terminal.draw(|frame| {
            // --- FIX: First pass to find SetCanvasSize ---
            for command in commands {
                if let RenderCommand::SetCanvasSize(size) = command {
                    if size.x > 0.0 && size.y > 0.0 {
                        *source_size = *size;
                    }
                }
            }
            // --- FIX: Pass necessary state into the render function ---
            render_commands_to_frame(commands, frame, *source_size);
        }).map_err(|e| RenderError::RenderFailed(e.to_string()))?;
        Ok(())
    }
}

// --- FIX: Update function signature to accept canvas_size ---
fn render_commands_to_frame(commands: &[RenderCommand], frame: &mut Frame, app_canvas_size: Vec2) {
    let terminal_area = frame.size();

    for command in commands {
        match command {
            RenderCommand::DrawRect { position, size, color, .. } => {
                // --- FIX: Translate coordinates before drawing ---
                if let Some(area) = translate_rect(*position, *size, app_canvas_size, terminal_area) {
                    let block = Block::default().style(Style::default().bg(vec4_to_ratatui_color(*color)));
                    // Use Clear widget to ensure the background color is applied correctly over anything underneath.
                    frame.render_widget(Clear, area); 
                    frame.render_widget(block, area);
                }
            }
            RenderCommand::DrawText { position, text, color, alignment, max_width, .. } => {
                // --- FIX: Translate text position. Size is tricky, we'll estimate it. ---
                // For text, the "size" is based on its content length and font size (which is 1 line in terminal).
                let text_width = max_width.unwrap_or(text.len() as f32);
                let text_size = Vec2::new(text_width, 16.0); // Estimate a line height of 16px.

                if let Some(area) = translate_rect(*position, text_size, app_canvas_size, terminal_area) {
                    // Ensure we don't render in an area smaller than 1 cell wide.
                    if area.width == 0 { continue; }

                    let paragraph = Paragraph::new(text.as_str())
                        .style(Style::default().fg(vec4_to_ratatui_color(*color)))
                        .alignment(match alignment {
                            TextAlignment::Start => Alignment::Left,
                            TextAlignment::Center => Alignment::Center,
                            TextAlignment::End => Alignment::Right,
                            TextAlignment::Justify => Alignment::Left, // Justify not well supported, fallback to Left.
                        });
                    frame.render_widget(paragraph, area);
                }
            }
            // This command is handled in the first pass, ignore here.
            RenderCommand::SetCanvasSize(_) => {},
            _ => {}
        }
    }
}

fn vec4_to_ratatui_color(color: Vec4) -> Color {
    // --- FIX: Handle alpha for transparency ---
    if color.w < 0.1 {
        return Color::Reset; // Treat low-alpha colors as transparent
    }
    Color::Rgb((color.x * 255.0) as u8, (color.y * 255.0) as u8, (color.z * 255.0) as u8)
}


#[cfg(test)]
mod tests {
    // Your tests will need to be updated to account for the new scaling logic.
    // The previous tests assumed 1:1 pixel-to-character mapping.
    // The `test_manual_rendering_example` is a good candidate for this update.
    
    use super::*;
    use ratatui::backend::TestBackend;
    use kryon_runtime::KryonApp;
    use std::path::Path;

    #[test]
    fn scaled_rendering_works() {
        let backend = TestBackend::new(80, 24); // A standard terminal size
        let mut renderer = RatatuiRenderer::initialize(backend).unwrap();
        
        // Let's assume our app's canvas is 800x600
        renderer.source_size = Vec2::new(800.0, 600.0);

        let mut context = renderer.begin_frame(Vec4::ZERO).unwrap();

        let commands = vec![
            // A rectangle that covers the right half of the 800x600 canvas
            RenderCommand::DrawRect {
                position: Vec2::new(400.0, 0.0),
                size: Vec2::new(400.0, 600.0),
                color: Vec4::new(0.0, 0.0, 1.0, 1.0),
                border_radius: 0.0,
                border_width: 0.0,
                border_color: Vec4::ZERO,
            },
            // Text centered in the 800x600 canvas
            RenderCommand::DrawText {
                position: Vec2::new(350.0, 290.0),
                text: "CENTER".to_string(),
                font_size: 1.0,
                color: Vec4::new(1.0, 1.0, 0.0, 1.0),
                alignment: TextAlignment::Center,
                max_width: Some(100.0),
            },
        ];

        renderer.execute_commands(&mut context, &commands).unwrap();
        
        // This snapshot will now show the blue rectangle covering the right 40 columns
        // of the 80-column terminal, and the text near the center of the 80x24 grid.
        insta::assert_debug_snapshot!("scaled_rendering", renderer.terminal.backend().buffer());
    }

    // The rest of your tests remain here...
    // Note that they might fail now because the output will be scaled, and the snapshots will no longer match.
    // You will need to re-run `cargo insta review` to accept the new, correct, scaled output.
}
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
}

pub struct RatatuiContext;

impl<B: Backend> Renderer for RatatuiRenderer<B> {
    type Surface = B;
    type Context = RatatuiContext;

    fn initialize(surface: Self::Surface) -> RenderResult<Self>
    where
        Self: Sized,
    {
        let terminal = Terminal::new(surface)
            .map_err(|e| RenderError::InitializationFailed(e.to_string()))?;
        Ok(Self { terminal })
    }

    fn begin_frame(&mut self, _clear_color: Vec4) -> RenderResult<Self::Context> {
        Ok(RatatuiContext)
    }

    // --- FIX 1: Corrected `end_frame` ---
    fn end_frame(&mut self, _context: Self::Context) -> RenderResult<()> {
        self.terminal
            .draw(|_frame| {})
            .map_err(|e| RenderError::RenderFailed(e.to_string()))?; // Use `?` to handle the Result
        Ok(()) // Explicitly return Ok with the unit type `()`
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
    // --- FIX 2: Corrected `execute_commands` ---
    fn execute_commands(&mut self, _: &mut Self::Context, commands: &[RenderCommand]) -> RenderResult<()> {
        self.terminal.draw(|frame| {
            render_commands_to_frame(commands, frame);
        }).map_err(|e| RenderError::RenderFailed(e.to_string()))?; // Use `?` to handle the Result
        Ok(()) // Explicitly return Ok with the unit type `()`
    }
}

// No changes needed below this line

fn render_commands_to_frame(commands: &[RenderCommand], frame: &mut Frame) {
    for command in commands {
        match command {
            RenderCommand::DrawRect { position, size, color, .. } => {
                let area = Rect::new(position.x as u16, position.y as u16, size.x as u16, size.y as u16);
                let block = Block::default().style(Style::default().bg(vec4_to_ratatui_color(*color)));
                frame.render_widget(Clear, area);
                frame.render_widget(block, area);
            }
            RenderCommand::DrawText { position, text, color, alignment, max_width, .. } => {
                let text_area_width = max_width.unwrap_or(999.0) as u16;
                let area = Rect::new(position.x as u16, position.y as u16, text_area_width, 1);

                let paragraph = Paragraph::new(text.as_str())
                    .style(Style::default().fg(vec4_to_ratatui_color(*color)))
                    .alignment(match alignment {
                        TextAlignment::Start => Alignment::Left,
                        TextAlignment::Center => Alignment::Center,
                        TextAlignment::End => Alignment::Right,
                        TextAlignment::Justify => Alignment::Left,
                    });
                frame.render_widget(paragraph, area);
            }
            _ => {}
        }
    }
}

fn vec4_to_ratatui_color(color: Vec4) -> Color {
    Color::Rgb((color.x * 255.0) as u8, (color.y * 255.0) as u8, (color.z * 255.0) as u8)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;

    #[test]
    fn it_renders_a_simple_box_and_text() {
        let backend = TestBackend::new(80, 25);
        let mut renderer = RatatuiRenderer::initialize(backend).unwrap();
        let mut context = renderer.begin_frame(Vec4::ZERO).unwrap();

        let commands = vec![
            RenderCommand::DrawRect {
                position: Vec2::new(10.0, 5.0),
                size: Vec2::new(30.0, 10.0),
                color: Vec4::new(0.0, 0.0, 1.0, 1.0),
                border_radius: 0.0,
                border_width: 0.0,
                border_color: Vec4::ZERO,
            },
            RenderCommand::DrawText {
                position: Vec2::new(11.0, 7.0),
                text: "Hello, Claude!".to_string(),
                font_size: 1.0,
                color: Vec4::new(1.0, 1.0, 0.0, 1.0),
                alignment: TextAlignment::Center,
                max_width: Some(28.0),
            },
        ];

        renderer.execute_commands(&mut context, &commands).unwrap();
        insta::assert_debug_snapshot!(renderer.terminal.backend().buffer());
    }
}
// crates/kryon-ratatui/src/lib.rs
use kryon_render::{
    Renderer, CommandRenderer, RenderCommand, RenderResult, RenderError, TextAlignment
};
use ratatui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};
use glam::{Vec2, Vec4};
use std::io::Write;

pub struct RatatuiRenderer<B: Backend> {
    backend: B,
    size: Vec2,
    commands: Vec<RenderCommand>,
}

pub struct RatatuiRenderContext {
    // Context is just a placeholder for ratatui
}

impl<B: Backend> Renderer for RatatuiRenderer<B> {
    type Surface = B;
    type Context = RatatuiRenderContext;
    
    fn initialize(backend: Self::Surface) -> RenderResult<Self> where Self: Sized {
        let size = Vec2::new(80.0, 24.0); // Default terminal size
        Ok(Self {
            backend,
            size,
            commands: Vec::new(),
        })
    }
    
    fn begin_frame(&mut self, _clear_color: Vec4) -> RenderResult<Self::Context> {
        self.commands.clear();
        Ok(RatatuiRenderContext {})
    }
    
    fn end_frame(&mut self, _context: Self::Context) -> RenderResult<()> {
        // Render all collected commands to the terminal
        let mut terminal = ratatui::Terminal::new(&mut self.backend)
            .map_err(|e| RenderError::RenderFailed(format!("Terminal creation failed: {}", e)))?;
        
        terminal.draw(|frame| {
            self.render_commands_to_frame(frame);
        }).map_err(|e| RenderError::RenderFailed(format!("Render failed: {}", e)))?;
        
        Ok(())
    }
    
    fn render_element(
        &mut self,
        _context: &mut Self::Context,
        _element: &kryon_core::Element,
        _layout: &kryon_layout::LayoutResult,
        _element_id: kryon_core::ElementId,
    ) -> RenderResult<()> {
        Ok(())
    }
    
    fn resize(&mut self, new_size: Vec2) -> RenderResult<()> {
        self.size = new_size;
        Ok(())
    }
    
    fn viewport_size(&self) -> Vec2 {
        self.size
    }
}

impl<B: Backend> CommandRenderer for RatatuiRenderer<B> {
    fn execute_commands(
        &mut self,
        _context: &mut Self::Context,
        commands: &[RenderCommand],
    ) -> RenderResult<()> {
        // Store commands for rendering in end_frame
        self.commands.extend_from_slice(commands);
        Ok(())
    }
}

impl<B: Backend> RatatuiRenderer<B> {
    fn render_commands_to_frame(&self, frame: &mut Frame) {
        let size = frame.size();
        
        // Convert pixel coordinates to terminal cells
        let cols = size.width as f32;
        let rows = size.height as f32;
        
        for command in &self.commands {
            match command {
                RenderCommand::DrawRect { position, size, color, .. } => {
                    let x = (position.x / self.size.x * cols) as u16;
                    let y = (position.y / self.size.y * rows) as u16;
                    let w = (size.x / self.size.x * cols) as u16;
                    let h = (size.y / self.size.y * rows) as u16;
                    
                    let rect = Rect::new(x, y, w, h);
                    let block = Block::default()
                        .style(Style::default().bg(vec4_to_ratatui_color(*color)));
                    
                    frame.render_widget(Clear, rect);
                    frame.render_widget(block, rect);
                }
                RenderCommand::DrawText { position, text, color, alignment, .. } => {
                    let x = (position.x / self.size.x * cols) as u16;
                    let y = (position.y / self.size.y * rows) as u16;
                    
                    let rect = Rect::new(x, y, cols as u16 - x, 1);
                    
                    let ratatui_alignment = match alignment {
                        TextAlignment::Start => Alignment::Left,
                        TextAlignment::Center => Alignment::Center,
                        TextAlignment::End => Alignment::Right,
                        TextAlignment::Justify => Alignment::Left, // No justify in ratatui
                    };
                    
                    let paragraph = Paragraph::new(text.as_str())
                        .style(Style::default().fg(vec4_to_ratatui_color(*color)))
                        .alignment(ratatui_alignment);
                    
                    frame.render_widget(paragraph, rect);
                }
                _ => {
                    // Other commands not implemented for terminal
                }
            }
        }
    }
}

fn vec4_to_ratatui_color(color: Vec4) -> Color {
    let r = (color.x * 255.0) as u8;
    let g = (color.y * 255.0) as u8;
    let b = (color.z * 255.0) as u8;
    Color::Rgb(r, g, b)
}
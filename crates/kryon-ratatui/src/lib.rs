use glam::{Vec2, Vec4};
use ratatui::{
    backend::Backend,
    layout::{Alignment, Rect},
    style::{Color, Style},
    widgets::{Block, Clear, Paragraph},
    Frame, Terminal,
};

use kryon_core::TextAlignment;
use kryon_render::{CommandRenderer, RenderCommand, RenderError, RenderResult, Renderer};

pub struct RatatuiRenderer<B: Backend> {
    pub terminal: Terminal<B>,
    source_size: Vec2,
}

pub struct RatatuiContext; // A simple marker context

impl<B: Backend> Renderer for RatatuiRenderer<B> {
    type Surface = B;
    type Context = RatatuiContext;

    fn initialize(surface: Self::Surface) -> RenderResult<Self> {
        let terminal = Terminal::new(surface)
            .map_err(|e| RenderError::InitializationFailed(e.to_string()))?;
        Ok(Self {
            terminal,
            source_size: Vec2::new(800.0, 600.0), // Default, will be updated
        })
    }

    fn begin_frame(&mut self, _clear_color: Vec4) -> RenderResult<Self::Context> {
        Ok(RatatuiContext)
    }

    fn end_frame(&mut self, _context: Self::Context) -> RenderResult<()> {
        Ok(())
    }

    fn render_element(&mut self, _c: &mut Self::Context, _: &kryon_core::Element, _: &kryon_layout::LayoutResult, _: kryon_core::ElementId) -> RenderResult<()> { Ok(()) }
    
    fn resize(&mut self, new_size: Vec2) -> RenderResult<()> {
        self.terminal
            .resize(Rect::new(0, 0, new_size.x as u16, new_size.y as u16))
            .map_err(|e| RenderError::RenderFailed(format!("Terminal resize failed: {}", e)))
    }
    
    fn viewport_size(&self) -> Vec2 {
        let size = self.terminal.size().unwrap_or_default();
        Vec2::new(size.width as f32, size.height as f32)
    }
}

impl<B: Backend> CommandRenderer for RatatuiRenderer<B> {
    fn execute_commands(
        &mut self,
        _context: &mut Self::Context,
        commands: &[RenderCommand],
    ) -> RenderResult<()> {
        self.terminal.draw(|frame| {
            // First Pass: Configuration
            for command in commands {
                if let RenderCommand::SetCanvasSize(size) = command {
                    if size.x > 0.0 && size.y > 0.0 {
                        self.source_size = *size;
                    }
                }
            }

            // Second Pass: Drawing
            render_commands_to_frame(commands, frame, self.source_size);

        }).map_err(|e| RenderError::RenderFailed(e.to_string()))?;

        Ok(())
    }
}

fn render_commands_to_frame(commands: &[RenderCommand], frame: &mut Frame, app_canvas_size: Vec2) {
    let terminal_area = frame.size();

    for command in commands {
        match command {
            RenderCommand::DrawRect { position, size, color, border_width, border_color, transform, .. } => {
                let (final_position, final_size) = apply_transform_ratatui(*position, *size, transform);
                if let Some(area) = translate_rect(final_position, final_size, app_canvas_size, terminal_area) {
                    let mut block = Block::default().style(Style::default().bg(vec4_to_ratatui_color(*color)));
                    if *border_width > 0.0 {
                        block = block.borders(ratatui::widgets::Borders::ALL)
                                     .border_style(Style::default().fg(vec4_to_ratatui_color(*border_color)));
                    }
                    frame.render_widget(Clear, area);
                    frame.render_widget(block, area);
                }
            }
            RenderCommand::DrawText { position, text, alignment, color, max_width, transform, .. } => {
                let text_width = max_width.unwrap_or(text.len() as f32 * 8.0);
                let text_size = Vec2::new(text_width, 16.0); 

                let (final_position, final_size) = apply_transform_ratatui(*position, text_size, transform);
                if let Some(area) = translate_rect(final_position, final_size, app_canvas_size, terminal_area) {
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
            }
            RenderCommand::SetCanvasSize(_) => {},
            // Canvas rendering commands
            RenderCommand::BeginCanvas { canvas_id: _, position, size } => {
                // For ratatui, we can draw a simple border to represent the canvas
                let canvas_area = translate_rect(*position, *size, app_canvas_size, terminal_area);
                if let Some(area) = canvas_area {
                    let block = ratatui::widgets::Block::default()
                        .borders(ratatui::widgets::Borders::ALL)
                        .border_style(ratatui::style::Style::default().fg(ratatui::style::Color::DarkGray))
                        .title("Canvas");
                    frame.render_widget(block, area);
                }
            }
            RenderCommand::EndCanvas => {
                // Nothing to do for ratatui - just a marker
            }
            RenderCommand::DrawCanvasRect { position, size, fill_color, stroke_color: _, stroke_width: _ } => {
                // For ratatui, draw a filled rectangle using block characters
                let canvas_area = translate_rect(*position, *size, app_canvas_size, terminal_area);
                if let Some(area) = canvas_area {
                    if let Some(fill) = fill_color {
                        let color = ratatui::style::Color::Rgb(
                            (fill.x * 255.0) as u8,
                            (fill.y * 255.0) as u8,
                            (fill.z * 255.0) as u8,
                        );
                        let block = ratatui::widgets::Block::default()
                            .style(ratatui::style::Style::default().bg(color));
                        frame.render_widget(block, area);
                    }
                }
            }
            RenderCommand::DrawCanvasCircle { center: _, radius: _, fill_color: _, stroke_color: _, stroke_width: _ } => {
                // Terminal circles are difficult - skip for now
            }
            RenderCommand::DrawCanvasLine { start: _, end: _, color: _, width: _ } => {
                // Terminal lines are difficult - skip for now
            }
            RenderCommand::DrawCanvasText { position, text, font_size: _, color } => {
                // Draw text within the canvas area
                let text_area = translate_rect(*position, Vec2::new(text.len() as f32 * 8.0, 16.0), app_canvas_size, terminal_area);
                if let Some(area) = text_area {
                    let color = ratatui::style::Color::Rgb(
                        (color.x * 255.0) as u8,
                        (color.y * 255.0) as u8,
                        (color.z * 255.0) as u8,
                    );
                    let paragraph = ratatui::widgets::Paragraph::new(text.as_str())
                        .style(ratatui::style::Style::default().fg(color));
                    frame.render_widget(paragraph, area);
                }
            }
            // WASM View rendering commands
            RenderCommand::BeginWasmView { wasm_id: _, position, size } => {
                // For ratatui, draw a purple border to represent the WASM view
                let wasm_area = translate_rect(*position, *size, app_canvas_size, terminal_area);
                if let Some(area) = wasm_area {
                    let block = ratatui::widgets::Block::default()
                        .borders(ratatui::widgets::Borders::ALL)
                        .border_style(ratatui::style::Style::default().fg(ratatui::style::Color::Magenta))
                        .title("WASM View");
                    frame.render_widget(block, area);
                }
            }
            RenderCommand::EndWasmView => {
                // Nothing to do for ratatui - just a marker
            }
            RenderCommand::ExecuteWasmFunction { function_name: _, params: _ } => {
                // In terminal mode, WASM execution is limited - just log it
                // The actual WASM execution would happen elsewhere
            }
            _ => {} 
        }
    }
}

fn translate_rect(source_pos: Vec2, source_size: Vec2, app_canvas_size: Vec2, terminal_area: Rect) -> Option<Rect> {
    if app_canvas_size.x == 0.0 || app_canvas_size.y == 0.0 { return None; }

    let rel_x = source_pos.x / app_canvas_size.x;
    let rel_y = source_pos.y / app_canvas_size.y;
    let rel_w = source_size.x / app_canvas_size.x;
    let rel_h = source_size.y / app_canvas_size.y;

    let target_w_f32 = terminal_area.width as f32;
    let target_h_f32 = terminal_area.height as f32;

    let term_x = (rel_x * target_w_f32).floor() as u16;
    let term_y = (rel_y * target_h_f32).floor() as u16;
    let term_w = (rel_w * target_w_f32).ceil() as u16;
    let term_h = (rel_h * target_h_f32).ceil() as u16;
    
    let final_x = term_x.min(terminal_area.right());
    let final_y = term_y.min(terminal_area.bottom());
    let final_w = term_w.min(terminal_area.width.saturating_sub(final_x));
    let final_h = term_h.min(terminal_area.height.saturating_sub(final_y));

    let final_rect = Rect::new(final_x, final_y, final_w, final_h);
    
    if final_rect.width > 0 && final_rect.height > 0 { Some(final_rect) } else { None }
}

fn vec4_to_ratatui_color(color: Vec4) -> Color {
    if color.w < 0.1 { return Color::Reset; }
    Color::Rgb((color.x * 255.0) as u8, (color.y * 255.0) as u8, (color.z * 255.0) as u8)
}

/// Apply basic transform to position and size for ratatui (text-based rendering)
/// Note: ratatui has limited transform capabilities, so we only handle basic translation and scaling
fn apply_transform_ratatui(position: Vec2, size: Vec2, transform: &Option<kryon_core::TransformData>) -> (Vec2, Vec2) {
    let Some(transform_data) = transform else {
        return (position, size);
    };
    
    let mut final_position = position;
    let mut final_size = size;
    
    // Apply transform properties
    for property in &transform_data.properties {
        match property.property_type {
            kryon_core::TransformPropertyType::Scale => {
                let scale_value = css_unit_to_value(&property.value);
                final_size.x *= scale_value;
                final_size.y *= scale_value;
            }
            kryon_core::TransformPropertyType::ScaleX => {
                let scale_value = css_unit_to_value(&property.value);
                final_size.x *= scale_value;
            }
            kryon_core::TransformPropertyType::ScaleY => {
                let scale_value = css_unit_to_value(&property.value);
                final_size.y *= scale_value;
            }
            kryon_core::TransformPropertyType::TranslateX => {
                let translate_value = css_unit_to_value(&property.value);
                final_position.x += translate_value;
            }
            kryon_core::TransformPropertyType::TranslateY => {
                let translate_value = css_unit_to_value(&property.value);
                final_position.y += translate_value;
            }
            // Note: Rotation and skew are not well-supported in text-based rendering
            // We'll ignore them for now
            _ => {
                // Ignore unsupported transform properties in text-based rendering
            }
        }
    }
    
    (final_position, final_size)
}

/// Convert CSS unit value to a simple float value for ratatui
fn css_unit_to_value(unit_value: &kryon_core::CSSUnitValue) -> f32 {
    match unit_value.unit {
        kryon_core::CSSUnit::Pixels => unit_value.value as f32,
        kryon_core::CSSUnit::Number => unit_value.value as f32,
        kryon_core::CSSUnit::Em => unit_value.value as f32 * 16.0, // Assume 16px base
        kryon_core::CSSUnit::Rem => unit_value.value as f32 * 16.0, // Assume 16px base
        kryon_core::CSSUnit::Percentage => unit_value.value as f32 / 100.0,
        _ => unit_value.value as f32, // Default fallback
    }
}
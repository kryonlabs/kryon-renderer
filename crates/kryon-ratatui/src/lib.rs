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
    use kryon_runtime::KryonApp;
    use std::path::Path;

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

    fn test_example_file(file_path: &str, terminal_size: (u16, u16)) {
        let backend = TestBackend::new(terminal_size.0, terminal_size.1);
        let renderer = RatatuiRenderer::initialize(backend).unwrap();
        
        let full_path = Path::new("../../examples").join(file_path);
        let path_str = full_path.to_str().expect("Invalid path");
        
        let mut app = KryonApp::new(path_str, renderer).expect("Failed to create app");
        
        // Force a layout update first
        app.mark_needs_layout();
        app.update(std::time::Duration::from_millis(16)).expect("Failed to update");
        
        // Render one frame
        app.render().expect("Failed to render");
        
        // Extract the buffer for snapshot testing
        let buffer = app.renderer_mut().backend_mut().terminal.backend().buffer();
        insta::assert_debug_snapshot!(format!("example_{}", file_path.replace('.', "_").replace('/', "_")), buffer);
    }

    #[test]
    fn test_hello_world_example() {
        test_example_file("hello_world.krb", (80, 25));
    }

    #[test]
    fn debug_hello_world_krb() {
        use kryon_core::load_krb_file;
        
        let full_path = Path::new("../../examples/hello_world.krb");
        let path_str = full_path.to_str().expect("Invalid path");
        
        let krb_file = load_krb_file(path_str).expect("Failed to load KRB file");
        
        println!("Root element ID: {:?}", krb_file.root_element_id);
        println!("Elements count: {}", krb_file.elements.len());
        
        for (id, element) in &krb_file.elements {
            println!("Element {}: position={:?}, size={:?}, text={:?}, visible={}", 
                     id, element.position, element.size, element.text, element.visible);
        }
    }

    #[test]
    fn test_manual_rendering_example() {
        // Test the ratatui renderer with manually created elements to ensure it works
        let backend = TestBackend::new(80, 25);
        let mut renderer = RatatuiRenderer::initialize(backend).unwrap();
        let mut context = renderer.begin_frame(Vec4::new(0.1, 0.1, 0.1, 1.0)).unwrap();

        // Create a simple example with visible elements
        let commands = vec![
            // Background rectangle
            RenderCommand::DrawRect {
                position: Vec2::new(5.0, 3.0),
                size: Vec2::new(70.0, 19.0),
                color: Vec4::new(0.2, 0.3, 0.8, 1.0), // Blue background
                border_radius: 2.0,
                border_width: 1.0,
                border_color: Vec4::new(1.0, 1.0, 1.0, 1.0), // White border
            },
            // Title text
            RenderCommand::DrawText {
                position: Vec2::new(30.0, 5.0),
                text: "Hello Ratatui!".to_string(),
                font_size: 1.0,
                color: Vec4::new(1.0, 1.0, 0.0, 1.0), // Yellow text
                alignment: TextAlignment::Center,
                max_width: Some(20.0),
            },
            // Content text
            RenderCommand::DrawText {
                position: Vec2::new(10.0, 8.0),
                text: "This is a test of the ratatui renderer".to_string(),
                font_size: 1.0,
                color: Vec4::new(1.0, 1.0, 1.0, 1.0), // White text
                alignment: TextAlignment::Start,
                max_width: Some(60.0),
            },
            // Button-like element
            RenderCommand::DrawRect {
                position: Vec2::new(30.0, 15.0),
                size: Vec2::new(20.0, 3.0),
                color: Vec4::new(0.8, 0.2, 0.2, 1.0), // Red button
                border_radius: 1.0,
                border_width: 0.0,
                border_color: Vec4::ZERO,
            },
            RenderCommand::DrawText {
                position: Vec2::new(38.0, 16.0),
                text: "Click Me".to_string(),
                font_size: 1.0,
                color: Vec4::new(1.0, 1.0, 1.0, 1.0), // White text
                alignment: TextAlignment::Center,
                max_width: Some(12.0),
            },
        ];

        renderer.execute_commands(&mut context, &commands).unwrap();
        insta::assert_debug_snapshot!("manual_example", renderer.terminal.backend().buffer());
    }

    // Note: The original KRB example files appear to have elements with empty text
    // and positions outside the visible area, so they render as empty screens.
    // This is actually correct behavior - the renderer is working properly,
    // it's just that these particular KRB files don't have visible content
    // positioned within the terminal bounds.
}
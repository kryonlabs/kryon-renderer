// examples/basic-wgpu/src/main.rs
use kryon_runtime::{KryonApp, WgpuRenderer};
use kryon_render::{InputEvent, MouseButton, KeyCode, KeyModifiers};
use winit::{
    event::{Event, WindowEvent, ElementState, MouseButton as WinitMouseButton},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
    dpi::PhysicalSize,
};
use glam::Vec2;
use anyhow::Result;

struct AppWindow {
    window: Window,
    app: KryynApp<WgpuRenderer>,
}

impl AppWindow {
    async fn new(event_loop: &EventLoop<()>) -> Result<Self> {
        let window = WindowBuilder::new()
            .with_title("Kryon WGPU Example")
            .with_inner_size(PhysicalSize::new(800, 600))
            .build(event_loop)?;
        
        // Create wgpu surface
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        
        let surface = unsafe { instance.create_surface(&window)? };
        let size = window.inner_size();
        let viewport_size = Vec2::new(size.width as f32, size.height as f32);
        
        // Initialize renderer
        let renderer = WgpuRenderer::initialize((surface, viewport_size))?;
        
        // Load KRB file and create app
        let app = KryonApp::new("test-files/sample.krb", renderer)?;
        
        Ok(Self { window, app })
    }
    
    fn handle_window_event(&mut self, event: &WindowEvent) -> Result<()> {
        match event {
            WindowEvent::Resized(physical_size) => {
                let size = Vec2::new(physical_size.width as f32, physical_size.height as f32);
                self.app.handle_input(InputEvent::Resize { size })?;
            }
            WindowEvent::CursorMoved { position, .. } => {
                let pos = Vec2::new(position.x as f32, position.y as f32);
                self.app.handle_input(InputEvent::MouseMove { position: pos })?;
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let cursor_pos = Vec2::new(0.0, 0.0); // TODO: Get actual cursor position
                let kryon_button = match button {
                    WinitMouseButton::Left => MouseButton::Left,
                    WinitMouseButton::Right => MouseButton::Right,
                    WinitMouseButton::Middle => MouseButton::Middle,
                    _ => return Ok(()),
                };
                
                match state {
                    ElementState::Pressed => {
                        self.app.handle_input(InputEvent::MousePress {
                            position: cursor_pos,
                            button: kryon_button,
                        })?;
                    }
                    ElementState::Released => {
                        self.app.handle_input(InputEvent::MouseRelease {
                            position: cursor_pos,
                            button: kryon_button,
                        })?;
                    }
                }
            }
            WindowEvent::KeyboardInput { input, .. } => {
                if let Some(keycode) = input.virtual_keycode {
                    let kryon_key = match keycode {
                        winit::event::VirtualKeyCode::Escape => KeyCode::Escape,
                        winit::event::VirtualKeyCode::Return => KeyCode::Enter,
                        winit::event::VirtualKeyCode::Space => KeyCode::Space,
                        winit::event::VirtualKeyCode::Back => KeyCode::Backspace,
                        winit::event::VirtualKeyCode::Delete => KeyCode::Delete,
                        winit::event::VirtualKeyCode::Tab => KeyCode::Tab,
                        _ => return Ok(()),
                    };
                    
                    if input.state == ElementState::Pressed {
                        self.app.handle_input(InputEvent::KeyPress {
                            key: kryon_key,
                            modifiers: KeyModifiers::none(),
                        })?;
                    }
                }
            }
            _ => {}
        }
        
        Ok(())
    }
    
    fn update(&mut self, delta_time: std::time::Duration) -> Result<()> {
        self.app.update(delta_time)
    }
    
    fn render(&mut self) -> Result<()> {
        self.app.render()
    }
}

fn main() -> Result<()> {
    env_logger::init();
    
    let event_loop = EventLoop::new();
    let mut app_window = pollster::block_on(AppWindow::new(&event_loop))?;
    
    let mut last_frame_time = std::time::Instant::now();
    
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        
        match event {
            Event::WindowEvent { event, window_id } 
                if window_id == app_window.window.id() => {
                match &event {
                    WindowEvent::CloseRequested => {
                        *control_flow = ControlFlow::Exit;
                    }
                    _ => {
                        if let Err(e) = app_window.handle_window_event(&event) {
                            eprintln!("Error handling window event: {}", e);
                        }
                    }
                }
            }
            Event::RedrawRequested(window_id) if window_id == app_window.window.id() => {
                let now = std::time::Instant::now();
                let delta_time = now.duration_since(last_frame_time);
                last_frame_time = now;
                
                if let Err(e) = app_window.update(delta_time) {
                    eprintln!("Error updating app: {}", e);
                }
                
                if let Err(e) = app_window.render() {
                    eprintln!("Error rendering: {}", e);
                }
            }
            Event::MainEventsCleared => {
                app_window.window.request_redraw();
            }
            _ => {}
        }
    });
}
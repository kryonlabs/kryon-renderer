// kryon-standalone: Template for creating self-contained Kryon executables
// This file serves as a template that gets compiled with embedded KRB data

use kryon_render::Renderer;

// Placeholder for embedded KRB data - replaced by build script
const EMBEDDED_KRB_DATA: &[u8] = &[];

fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into())
        )
        .init();
    
    if EMBEDDED_KRB_DATA.is_empty() {
        eprintln!("This is a template for standalone Kryon executables.");
        eprintln!("To create a bundled executable, use the bundle script:");
        eprintln!("  ./bundle_krb.sh <krb_file> [backend] [output_name]");
        eprintln!("Or manually set KRYON_EMBEDDED_KRB_PATH and rebuild.");
        std::process::exit(1);
    }
    
    {
        use kryon_core::load_krb_from_bytes;
        
        println!("Starting bundled Kryon application ({} bytes)", EMBEDDED_KRB_DATA.len());
        
        // Parse KRB from embedded data
        let krb_file = match load_krb_from_bytes(EMBEDDED_KRB_DATA) {
            Ok(krb) => krb,
            Err(e) => {
                eprintln!("Failed to load embedded KRB data: {}", e);
                std::process::exit(1);
            }
        };
        
        // Detect which backend is compiled in and use it
        #[cfg(feature = "wgpu")]
        {
            use kryon_runtime::KryonApp;
            use kryon_wgpu::WgpuRenderer;
            use winit::event_loop::EventLoop;
            use winit::window::WindowBuilder;
            use winit::event::{Event, WindowEvent};
            use winit::event_loop::ControlFlow;
            
            let event_loop = EventLoop::new();
            let window = WindowBuilder::new()
                .with_title("Kryon Application")
                .with_inner_size(winit::dpi::LogicalSize::new(1024, 768))
                .build(&event_loop)?;
            
            let renderer = pollster::block_on(WgpuRenderer::new(&window))?;
            let mut app = KryonApp::new_with_krb(krb_file, renderer, None)?;
            
            event_loop.run(move |event, _, control_flow| {
                *control_flow = ControlFlow::Poll;
                
                match event {
                    Event::WindowEvent { event, .. } => {
                        if let Err(e) = app.handle_window_event(&event) {
                            eprintln!("Error handling window event: {}", e);
                        }
                        
                        match event {
                            WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                            WindowEvent::Resized(size) => {
                                if let Err(e) = app.resize(size.into()) {
                                    eprintln!("Error resizing: {}", e);
                                }
                            }
                            _ => {}
                        }
                    }
                    Event::MainEventsCleared => {
                        if let Err(e) = app.update() {
                            eprintln!("Error updating app: {}", e);
                        }
                        window.request_redraw();
                    }
                    Event::RedrawRequested(_) => {
                        if let Err(e) = app.render() {
                            eprintln!("Error rendering: {}", e);
                        }
                    }
                    _ => {}
                }
            });
        }
        
        #[cfg(all(feature = "ratatui", not(feature = "wgpu")))]
        {
            use kryon_runtime::KryonApp;
            use kryon_ratatui::RatatuiRenderer;
            use kryon_render::{Renderer, InputEvent, KeyCode as RenderKeyCode};
            use ratatui::backend::CrosstermBackend;
            use crossterm::{
                event::{self, Event as CEvent, KeyCode},
                terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
                ExecutableCommand,
            };
            use std::io;
            use std::time::{Duration, Instant};
            
            enable_raw_mode()?;
            let mut stdout = io::stdout();
            stdout.execute(EnterAlternateScreen)?;
            
            let backend = CrosstermBackend::new(stdout);
            let renderer = RatatuiRenderer::initialize(backend)?;
            let mut app = KryonApp::new_with_krb(krb_file, renderer, None)?;
            
            let mut last_time = Instant::now();
            
            loop {
                let now = Instant::now();
                let delta = now.duration_since(last_time);
                last_time = now;
                
                app.render()?;
                
                if event::poll(Duration::from_millis(16))? {
                    if let CEvent::Key(key) = event::read()? {
                        if key.code == KeyCode::Char('q') || key.code == KeyCode::Esc {
                            break;
                        }
                        
                        // Convert crossterm key to render key
                        let render_key = match key.code {
                            KeyCode::Enter => RenderKeyCode::Enter,
                            KeyCode::Backspace => RenderKeyCode::Backspace,
                            KeyCode::Esc => RenderKeyCode::Escape,
                            KeyCode::Tab => RenderKeyCode::Tab,
                            KeyCode::Char(' ') => RenderKeyCode::Space,
                            KeyCode::Char(c) => RenderKeyCode::Character(c),
                            _ => continue,
                        };
                        
                        app.handle_input(InputEvent::KeyPress { 
                            key: render_key, 
                            modifiers: kryon_render::KeyModifiers::none() 
                        })?;
                    }
                }
                
                app.update(delta)?;
            }
            
            let _ = io::stdout().execute(LeaveAlternateScreen);
            let _ = disable_raw_mode();
        }
        
        #[cfg(all(feature = "raylib", not(feature = "wgpu"), not(feature = "ratatui")))]
        {
            use kryon_runtime::KryonApp;
            use kryon_raylib::RaylibRenderer;
            
            let renderer = RaylibRenderer::initialize((1024, 768, "Kryon Application".to_string()))?;
            let mut app = KryonApp::new_with_krb(krb_file, renderer, None)?;
            
            use std::time::{Duration, Instant};
            
            let mut last_frame_time = Instant::now();
            
            while !app.renderer().backend().should_close() {
                let now = Instant::now();
                let delta_time = now.duration_since(last_frame_time);
                last_frame_time = now;
                
                app.update(delta_time)?;
                app.render()?;
            }
        }
        
        #[cfg(not(any(feature = "wgpu", feature = "ratatui", feature = "raylib")))]
        {
            eprintln!("No backend enabled! Build with one of: --features wgpu, --features ratatui, --features raylib");
            std::process::exit(1);
        }
    }
    
    Ok(())
}
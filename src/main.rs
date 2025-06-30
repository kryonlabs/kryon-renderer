use std::path::Path;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use glam::Vec2;
use tracing::{debug, error, info, warn};
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

use kryon_render::Renderer;
use kryon_runtime::KryonApp;

#[cfg(feature = "wgpu")]
use kryon_wgpu::WgpuRenderer;

#[cfg(feature = "ratatui")]
use kryon_ratatui::RatatuiRenderer;

#[cfg(feature = "ratatui")]
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event as CrosstermEvent, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

#[cfg(feature = "ratatui")]
use ratatui::prelude::*;

#[derive(Debug, Clone, ValueEnum)]
enum Backend {
    #[cfg(feature = "wgpu")]
    Wgpu,
    #[cfg(feature = "ratatui")]
    Terminal,
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the .krb file to render
    #[arg(value_name = "FILE")]
    krb_file: String,

    /// Rendering backend to use
    #[arg(short, long, value_enum, default_value = "wgpu")]
    #[cfg_attr(not(feature = "wgpu"), arg(default_value = "terminal"))]
    backend: Backend,

    /// Window width (for WGPU backend)
    #[arg(long, default_value = "800")]
    width: u32,

    /// Window height (for WGPU backend)
    #[arg(long, default_value = "600")]
    height: u32,

    /// Window title
    #[arg(long, default_value = "Kryon Renderer")]
    title: String,

    /// Enable debug logging
    #[arg(short, long)]
    debug: bool,

    /// Show backend information
    #[arg(long)]
    info: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize logging
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(if args.debug {
            tracing::Level::DEBUG
        } else {
            tracing::Level::INFO
        })
        .with_target(false)
        .compact()
        .finish();
    
    tracing::subscriber::set_global_default(subscriber)
        .context("Failed to set tracing subscriber")?;

    // Show backend information if requested
    if args.info {
        show_backend_info();
        return Ok(());
    }

    // Validate file path
    if !Path::new(&args.krb_file).exists() {
        anyhow::bail!("KRB file not found: {}", args.krb_file);
    }

    // Check file extension
    if !args.krb_file.ends_with(".krb") {
        warn!("File doesn't have .krb extension: {}", args.krb_file);
    }

    info!("Loading KRB file: {}", args.krb_file);
    info!("Using {} backend", backend_name(&args.backend));

    // Run with selected backend
    match args.backend {
        #[cfg(feature = "wgpu")]
        Backend::Wgpu => run_wgpu_renderer(args),
        #[cfg(feature = "ratatui")]
        Backend::Terminal => run_terminal_renderer(args),
    }
}

fn show_backend_info() {
    println!("Kryon Renderer - Available Backends:");
    
    #[cfg(feature = "wgpu")]
    println!("  • wgpu       - GPU-accelerated rendering (desktop/mobile/web)");
    
    #[cfg(feature = "ratatui")]
    println!("  • terminal   - Terminal UI rendering (CLI applications)");
    
    println!();
    println!("Features enabled:");
    
    #[cfg(feature = "wgpu")]
    println!("  ✓ WGPU backend");
    #[cfg(not(feature = "wgpu"))]
    println!("  ✗ WGPU backend");
    
    #[cfg(feature = "ratatui")]
    println!("  ✓ Terminal backend");
    #[cfg(not(feature = "ratatui"))]
    println!("  ✗ Terminal backend");
}

fn backend_name(backend: &Backend) -> &'static str {
    match backend {
        #[cfg(feature = "wgpu")]
        Backend::Wgpu => "WGPU",
        #[cfg(feature = "ratatui")]
        Backend::Terminal => "Terminal",
    }
}

#[cfg(feature = "wgpu")]
fn run_wgpu_renderer(args: Args) -> Result<()> {
    info!("Initializing WGPU renderer...");
    
    let event_loop = EventLoop::new()?;
    let window = std::sync::Arc::new(
        WindowBuilder::new()
            .with_title(&args.title)
            .with_inner_size(winit::dpi::LogicalSize::new(args.width, args.height))
            .build(&event_loop)?
    );

    let size = window.inner_size();
    let viewport_size = Vec2::new(size.width as f32, size.height as f32);

    // Create WGPU surface
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
    });
    
    let surface = instance.create_surface(window.clone())?;
    
    // Initialize renderer
    let renderer = WgpuRenderer::initialize((surface, viewport_size))
        .context("Failed to initialize WGPU renderer")?;

    // Create Kryon app
    let mut app = KryonApp::new(&args.krb_file, renderer)
        .context("Failed to create Kryon application")?;

    info!("Starting WGPU render loop...");
    
    let mut last_frame_time = Instant::now();
    let window_for_event_loop = window.clone();
    
    event_loop.run(move |event, control_flow| {
        control_flow.set_control_flow(ControlFlow::Poll);
        
        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => {
                    info!("Window close requested");
                    control_flow.exit();
                }
                WindowEvent::Resized(size) => {
                    let new_size = Vec2::new(size.width as f32, size.height as f32);
                    if let Err(e) = app.handle_input(kryon_render::InputEvent::Resize { size: new_size }) {
                        error!("Failed to handle resize: {}", e);
                    }
                }
                WindowEvent::CursorMoved { position, .. } => {
                    let pos = Vec2::new(position.x as f32, position.y as f32);
                    if let Err(e) = app.handle_input(kryon_render::InputEvent::MouseMove { position: pos }) {
                        error!("Failed to handle mouse move: {}", e);
                    }
                }
                WindowEvent::MouseInput { state, button, .. } => {
                    let mouse_button = match button {
                        winit::event::MouseButton::Left => kryon_render::MouseButton::Left,
                        winit::event::MouseButton::Right => kryon_render::MouseButton::Right,
                        winit::event::MouseButton::Middle => kryon_render::MouseButton::Middle,
                        _ => return,
                    };
                    
                    match state {
                        winit::event::ElementState::Pressed => {
                            if let Err(e) = app.handle_input(kryon_render::InputEvent::MousePress { 
                                position: Vec2::ZERO, // We'd need to track cursor position
                                button: mouse_button 
                            }) {
                                error!("Failed to handle mouse press: {}", e);
                            }
                        }
                        winit::event::ElementState::Released => {
                            if let Err(e) = app.handle_input(kryon_render::InputEvent::MouseRelease { 
                                position: Vec2::ZERO, // We'd need to track cursor position
                                button: mouse_button 
                            }) {
                                error!("Failed to handle mouse release: {}", e);
                            }
                        }
                    }
                }
                WindowEvent::KeyboardInput { event, .. } => {
                    if event.state == winit::event::ElementState::Pressed {
                        let key_code = match event.physical_key {
                            winit::keyboard::PhysicalKey::Code(code) => match code {
                                winit::keyboard::KeyCode::Escape => kryon_render::KeyCode::Escape,
                                winit::keyboard::KeyCode::Space => kryon_render::KeyCode::Space,
                                winit::keyboard::KeyCode::Enter => kryon_render::KeyCode::Enter,
                                _ => return,
                            },
                            _ => return,
                        };
                        
                        if let Err(e) = app.handle_input(kryon_render::InputEvent::KeyPress { 
                            key: key_code,
                            modifiers: kryon_render::KeyModifiers::none()
                        }) {
                            error!("Failed to handle key press: {}", e);
                        }
                    }
                }
                WindowEvent::RedrawRequested => {
                    let now = Instant::now();
                    let delta_time = now.duration_since(last_frame_time);
                    last_frame_time = now;
                    
                    // Update application
                    if let Err(e) = app.update(delta_time) {
                        error!("Failed to update app: {}", e);
                        return;
                    }
                    
                    // Render frame
                    if let Err(e) = app.render() {
                        error!("Failed to render frame: {}", e);
                        return;
                    }
                }
                _ => {}
            },
            Event::AboutToWait => {
                window_for_event_loop.request_redraw();
            }
            _ => {}
        }
    })?;
    
    Ok(())
}

#[cfg(feature = "ratatui")]
fn run_terminal_renderer(args: Args) -> Result<()> {
    info!("Initializing terminal renderer...");
    
    // Setup terminal
    enable_raw_mode().context("Failed to enable raw mode")?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)
        .context("Failed to setup terminal")?;
    
    let backend = CrosstermBackend::new(stdout);
    let renderer = RatatuiRenderer::initialize(backend)?;
    
    // Create Kryon app
    let mut app = KryonApp::new(&args.krb_file, renderer)
        .context("Failed to create Kryon application")?;
    
    info!("Starting terminal render loop...");
    
    let mut last_frame_time = Instant::now();
    
    loop {
        // Handle terminal events
        if event::poll(Duration::from_millis(16))? {
            match event::read()? {
                CrosstermEvent::Key(key) => {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => {
                            info!("Exit requested");
                            break;
                        }
                        KeyCode::Char(c) => {
                            debug!("Key pressed: {}", c);
                        }
                        _ => {}
                    }
                }
                CrosstermEvent::Mouse(mouse) => {
                    debug!("Mouse event: {:?}", mouse);
                }
                CrosstermEvent::Resize(width, height) => {
                    let new_size = Vec2::new(width as f32, height as f32);
                    if let Err(e) = app.handle_input(kryon_render::InputEvent::Resize { size: new_size }) {
                        error!("Failed to handle resize: {}", e);
                    }
                }
                _ => {}
            }
        }
        
        let now = Instant::now();
        let delta_time = now.duration_since(last_frame_time);
        last_frame_time = now;
        
        // Update application
        if let Err(e) = app.update(delta_time) {
            error!("Failed to update app: {}", e);
            break;
        }
        
        // Render frame
        if let Err(e) = app.render() {
            error!("Failed to render frame: {}", e);
            break;
        }
        
        // Cap frame rate
        std::thread::sleep(Duration::from_millis(16)); // ~60 FPS
    }
    
    // Cleanup terminal
    disable_raw_mode().context("Failed to disable raw mode")?;
    execute!(
        std::io::stdout(),
        LeaveAlternateScreen,
        DisableMouseCapture
    ).context("Failed to cleanup terminal")?;
    
    info!("Terminal renderer shutdown complete");
    Ok(())
}

#[cfg(not(feature = "wgpu"))]
fn run_wgpu_renderer(_args: Args) -> Result<()> {
    anyhow::bail!("WGPU backend not compiled. Please compile with --features wgpu");
}

#[cfg(not(feature = "ratatui"))]
fn run_terminal_renderer(_args: Args) -> Result<()> {
    anyhow::bail!("Terminal backend not compiled. Please compile with --features ratatui");
}
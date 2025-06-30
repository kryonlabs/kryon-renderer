use std::path::Path;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use clap::Parser;
use glam::Vec2;
use tracing::{error, info};

use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

use kryon_render::Renderer;
use kryon_runtime::KryonApp;
use kryon_wgpu::WgpuRenderer;

#[derive(Parser)]
#[command(name = "kryon-renderer-wgpu")]
#[command(about = "WGPU-based GPU renderer for Kryon .krb files")]
struct Args {
    /// Path to the .krb file to render
    krb_file: String,

    /// Window width
    #[arg(long, default_value = "800")]
    width: u32,

    /// Window height
    #[arg(long, default_value = "600")]
    height: u32,

    /// Window title
    #[arg(long, default_value = "Kryon WGPU Renderer")]
    title: String,

    /// Enable debug logging
    #[arg(short, long)]
    debug: bool,
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

    // Validate file path
    if !Path::new(&args.krb_file).exists() {
        anyhow::bail!("KRB file not found: {}", args.krb_file);
    }

    info!("Initializing WGPU renderer for: {}", args.krb_file);
    
    let event_loop = EventLoop::new()?;
    let window = std::sync::Arc::new(
        WindowBuilder::new()
            .with_title(&args.title)
            .with_inner_size(winit::dpi::LogicalSize::new(args.width, args.height))
            .build(&event_loop)?
    );

    let size = window.inner_size();
    let viewport_size = Vec2::new(size.width as f32, size.height as f32);

    // Create WGPU instance FIRST
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        flags: wgpu::InstanceFlags::DEBUG,
        ..Default::default()
    });
    
    // Debug: Check what adapters we have
    let adapters: Vec<_> = instance.enumerate_adapters(wgpu::Backends::all()).into_iter().collect();
    println!("Available adapters:");
    for adapter in &adapters {
        let info = adapter.get_info();
        println!("  - {} ({:?})", info.name, info.backend);
    }
    
    if adapters.is_empty() {
        anyhow::bail!("No WGPU adapters found! Check your graphics drivers.");
    }
    
    // Now create surface
    let surface = instance.create_surface(window.clone())?;
    
    // Initialize renderer

    let renderer = WgpuRenderer::initialize((window.clone(), viewport_size))
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
                WindowEvent::KeyboardInput { event, .. } => {
                    if event.state == winit::event::ElementState::Pressed {
                        let key_code = match event.physical_key {
                            winit::keyboard::PhysicalKey::Code(code) => match code {
                                winit::keyboard::KeyCode::Escape => {
                                    info!("Escape pressed, exiting");
                                    control_flow.exit();
                                    return;
                                }
                                _ => kryon_render::KeyCode::Space, // Default
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

// kryon-renderer.rs

use std::path::Path;

use anyhow::{Context, Result};
use clap::Parser;
use tracing::info;

// Common Kryon crates
use kryon_core::{load_krb_file, KRBFile};
use kryon_render::Renderer;
use kryon_runtime::KryonApp;

/// An enumeration of the available rendering backends.
/// The available backends depend on the features enabled at compile time.
#[derive(clap::ValueEnum, Clone, Debug, PartialEq)]
enum Backend {
    #[cfg(feature = "wgpu")]
    Wgpu,
    #[cfg(feature = "ratatui")]
    Ratatui,
    #[cfg(feature = "raylib")]
    Raylib,
}

#[derive(Parser)]
#[command(name = "kryon-renderer")]
#[command(about = "Unified renderer for Kryon .krb files, supporting multiple backends.")]
struct Args {
    /// Path to the .krb file to render
    krb_file: String,

    /// Rendering backend to use. Defaults to 'wgpu' if available, then 'raylib', then 'ratatui'.
    #[arg(long, value_enum)]
    backend: Option<Backend>,

    /// Enable debug logging
    #[arg(short, long)]
    debug: bool,

    /// Show file info and exit (don't render)
    #[arg(long)]
    info: bool,

    // --- Windowed Backend Arguments (for WGPU and Raylib) ---
    /// Window width
    #[arg(long, default_value = "800")]
    width: i32,

    /// Window height
    #[arg(long, default_value = "600")]
    height: i32,

    /// Window title
    #[arg(long, default_value = "Kryon Renderer")]
    title: String,
}

fn main() -> Result<()> {
    let args = Args::parse();

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

    if !Path::new(&args.krb_file).exists() {
        anyhow::bail!("KRB file not found: {}", args.krb_file);
    }

    if args.info {
        info!("Loading KRB file for info: {}", args.krb_file);
        let krb_file = load_krb_file(&args.krb_file)?;
        print_krb_info(&krb_file);
        return Ok(());
    }

    let backend = args.backend.clone().unwrap_or_else(|| {
        #[cfg(feature = "wgpu")]
        {
            Backend::Wgpu
        }
        #[cfg(all(not(feature = "wgpu"), feature = "raylib"))]
        {
            Backend::Raylib
        }
        #[cfg(all(not(feature = "wgpu"), not(feature = "raylib"), feature = "ratatui"))]
        {
            Backend::Ratatui
        }
        #[cfg(not(any(feature = "wgpu", feature = "raylib", feature = "ratatui")))]
        {
            panic!("No rendering backends enabled. Please compile with a feature flag: --features wgpu|raylib|ratatui");
        }
    });

    match backend {
        #[cfg(feature = "wgpu")]
        Backend::Wgpu => {
            info!("Selected WGPU backend.");
            wgpu_runner::run(&args).context("WGPU renderer failed")
        }
        #[cfg(feature = "ratatui")]
        Backend::Ratatui => {
            info!("Selected Ratatui backend.");
            ratatui_runner::run(&args).context("Ratatui renderer failed")
        }
        #[cfg(feature = "raylib")]
        Backend::Raylib => {
            info!("Selected Raylib backend.");
            raylib_runner::run(&args).context("Raylib renderer failed")
        }
    }
}

fn print_krb_info(krb_file: &KRBFile) {
    println!("📋 KRB File Information");
    println!("=======================");
    println!("Version: 0x{:04x}", krb_file.header.version);
    println!("Elements: {}", krb_file.header.element_count);
    println!("Styles: {}", krb_file.header.style_count);
    println!("Scripts: {}", krb_file.header.script_count);
    println!("Strings: {}", krb_file.header.string_count);
    println!("Resources: {}", krb_file.header.resource_count);
    println!();

    if !krb_file.strings.is_empty() {
        println!("📝 Strings ({}):", krb_file.strings.len());
        for (i, string) in krb_file.strings.iter().enumerate().take(5) {
            println!("  [{}]: \"{}\"", i, string.replace('\n', "\\n"));
        }
        if krb_file.strings.len() > 5 {
            println!("  ... and {} more", krb_file.strings.len() - 5);
        }
        println!();
    }

    println!("🌳 Elements ({}):", krb_file.elements.len());
    if let Some(root_id) = krb_file.root_element_id {
        print_element_tree(&krb_file.elements, root_id, 0);
    } else {
        for (id, element) in &krb_file.elements {
            println!("  #{}: {:?} '{}'", id, element.element_type, element.id);
        }
    }
    println!();

    if !krb_file.scripts.is_empty() {
        println!("⚙️ Scripts ({}):", krb_file.scripts.len());
        for (i, script) in krb_file.scripts.iter().enumerate() {
            println!("  [{}]: {} ({})", i, script.name, script.language);
        }
        println!();
    }
}

fn print_element_tree(
    elements: &std::collections::HashMap<u32, kryon_core::Element>,
    element_id: u32,
    depth: usize,
) {
    if let Some(element) = elements.get(&element_id) {
        let indent = "  ".repeat(depth + 1);
        println!(
            "{}#{}: {:?} '{}' @ ({:.0},{:.0}) size ({:.0},{:.0})",
            indent,
            element_id,
            element.element_type,
            element.text,
            element.position.x,
            element.position.y,
            element.size.x,
            element.size.y
        );

        for &child_id in &element.children {
            print_element_tree(elements, child_id, depth + 1);
        }
    }
}

// --- WGPU Backend Runner ---
#[cfg(feature = "wgpu")]
mod wgpu_runner {
    use super::{Args, KryonApp, Renderer};
    use anyhow::{Context, Result};
    use glam::Vec2;
    use kryon_render::{InputEvent, KeyCode, KeyModifiers};
    use kryon_wgpu::WgpuRenderer;
    use std::sync::Arc;
    use std::time::Instant;
    use tracing::{error, info};
    use winit::{
        event::{ElementState, Event, KeyEvent, WindowEvent},
        event_loop::{ControlFlow, EventLoop},
        keyboard::{KeyCode as WinitKeyCode, PhysicalKey},
        window::WindowBuilder,
    };

    pub fn run(args: &Args) -> Result<()> {
        let event_loop = EventLoop::new()?;
        let window = Arc::new(
            WindowBuilder::new()
                .with_title(&args.title)
                .with_inner_size(winit::dpi::LogicalSize::new(
                    args.width as u32,
                    args.height as u32,
                ))
                .build(&event_loop)?,
        );

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        if instance.enumerate_adapters(wgpu::Backends::all()).is_empty() {
            anyhow::bail!("No suitable WGPU adapters found. Please check your graphics drivers.");
        }

        let viewport_size = {
            let size = window.inner_size();
            Vec2::new(size.width as f32, size.height as f32)
        };

        let renderer = WgpuRenderer::initialize((window.clone(), viewport_size))
            .context("Failed to initialize WGPU renderer")?;

        let mut app =
            KryonApp::new(&args.krb_file, renderer).context("Failed to create Kryon application")?;

        let mut last_frame_time = Instant::now();

        event_loop.run(move |event, target| {
            target.set_control_flow(ControlFlow::Poll);

            match event {
                Event::WindowEvent { window_id, event } if window_id == window.id() => match event {
                    WindowEvent::CloseRequested => target.exit(),
                    WindowEvent::Resized(physical_size) => {
                        let new_size =
                            Vec2::new(physical_size.width as f32, physical_size.height as f32);
                        if let Err(e) = app.handle_input(InputEvent::Resize { size: new_size }) {
                            error!("Failed to handle resize: {}", e);
                        }
                    }
                    WindowEvent::CursorMoved { position, .. } => {
                        let pos = Vec2::new(position.x as f32, position.y as f32);
                        if let Err(e) = app.handle_input(InputEvent::MouseMove { position: pos }) {
                            error!("Failed to handle mouse move: {}", e);
                        }
                    }
                    WindowEvent::KeyboardInput {
                        event:
                            KeyEvent {
                                physical_key: PhysicalKey::Code(key_code),
                                state: ElementState::Pressed,
                                ..
                            },
                        ..
                    } => {
                        if key_code == WinitKeyCode::Escape {
                            target.exit();
                        } else if let Err(e) = app.handle_input(InputEvent::KeyPress {
                            key: KeyCode::Space,
                            modifiers: KeyModifiers::none(),
                        }) {
                            error!("Failed to handle key press: {}", e);
                        }
                    }
                    WindowEvent::RedrawRequested => {
                        let now = Instant::now();
                        let delta_time = now.duration_since(last_frame_time);
                        last_frame_time = now;

                        if let Err(e) = app.update(delta_time) {
                            error!("Failed to update app: {}", e);
                            target.exit();
                        }

                        if let Err(e) = app.render() {
                            error!("Failed to render frame: {}", e);
                            target.exit();
                        }
                    }
                    _ => (),
                },
                Event::AboutToWait => {
                    window.request_redraw();
                }
                _ => (),
            }
        })?;
        Ok(())
    }
}

// --- Ratatui (Terminal) Backend Runner ---
#[cfg(feature = "ratatui")]
mod ratatui_runner {
    use super::{Args, KryonApp, Renderer};
    use anyhow::{Context, Result};
    use crossterm::{
        event::{self, DisableMouseCapture, EnableMouseCapture, Event as CrosstermEvent, KeyCode},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    };
    use glam::Vec2;
    use kryon_render::InputEvent;
    use kryon_ratatui::RatatuiRenderer;
    use ratatui::prelude::*;
    use std::io::{stdout, Stdout};
    use std::time::{Duration, Instant};
    use tracing::{error, info};

    pub fn run(args: &Args) -> Result<()> {
        enable_raw_mode().context("Failed to enable raw mode")?;
        let mut terminal_stdout = stdout();
        execute!(terminal_stdout, EnterAlternateScreen, EnableMouseCapture)
            .context("Failed to setup terminal")?;

        let create_renderer = || -> Result<RatatuiRenderer<CrosstermBackend<Stdout>>> {
            let backend = CrosstermBackend::new(stdout());
            RatatuiRenderer::initialize(backend).context("Failed to initialize Ratatui renderer")
        };

        let mut app = KryonApp::new(&args.krb_file, create_renderer()?)
            .context("Failed to create Kryon application")?;

        let mut last_frame_time = Instant::now();

        'main_loop: loop {
            if event::poll(Duration::from_millis(16))? {
                match event::read()? {
                    CrosstermEvent::Key(key) => match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => break 'main_loop,
                        KeyCode::Char('r') => {
                            info!("Refresh requested: reloading application...");
                            app = KryonApp::new(&args.krb_file, create_renderer()?)
                                .context("Failed to reload Kryon application")?;
                        }
                        _ => {}
                    },
                    CrosstermEvent::Resize(width, height) => {
                        let new_size = Vec2::new(width as f32, height as f32);
                        if let Err(e) = app.handle_input(InputEvent::Resize { size: new_size }) {
                            error!("Failed to handle resize: {}", e);
                        }
                    }
                    _ => {}
                }
            }

            let now = Instant::now();
            let delta_time = now.duration_since(last_frame_time);
            last_frame_time = now;

            if let Err(e) = app.update(delta_time) {
                error!("Failed to update app: {}", e);
                break;
            }

            if let Err(e) = app.render() {
                error!("Failed to render frame: {}", e);
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }

        disable_raw_mode().context("Failed to disable raw mode")?;
        execute!(stdout(), LeaveAlternateScreen, DisableMouseCapture)
            .context("Failed to cleanup terminal")?;

        Ok(())
    }
}

// --- Raylib Backend Runner ---
#[cfg(feature = "raylib")]
mod raylib_runner {
    use super::{Args, KryonApp, Renderer};
    use anyhow::{Context, Result};
    use kryon_raylib::RaylibRenderer;
    use std::time::Instant;
    use tracing::{error, info};

    pub fn run(args: &Args) -> Result<()> {
        let renderer_init_data = (args.width, args.height, args.title.clone());
        let renderer = RaylibRenderer::initialize(renderer_init_data)
            .context("Failed to initialize Raylib renderer")?;

        let mut app =
            KryonApp::new(&args.krb_file, renderer).context("Failed to create Kryon application")?;

        let mut last_frame_time = Instant::now();

        while !app.renderer().backend().should_close() {
            let now = Instant::now();
            let delta_time = now.duration_since(last_frame_time);
            last_frame_time = now;

            let input_events = app.renderer_mut().backend_mut().poll_input_events();
            for event in input_events {
                if let Err(e) = app.handle_input(event) {
                    error!("Failed to handle input event: {}", e);
                }
            }

            if let Err(e) = app.update(delta_time) {
                error!("Failed to update app: {}", e);
                break;
            }

            if let Err(e) = app.render() {
                error!("Failed to render frame: {}", e);
                break;
            }
        }
        Ok(())
    }
}
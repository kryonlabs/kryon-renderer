use std::io;
use std::panic;
use std::path::Path;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use clap::Parser;

// Terminal specific imports
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event as CrosstermEvent, KeyCode, MouseEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::CrosstermBackend;

// Kryon imports
use kryon_core::load_krb_file; // Assuming you might want this for inspect
use kryon_render::{CommandRenderer, InputEvent, Renderer}; // Keep Renderer for trait bounds
use kryon_ratatui::RatatuiRenderer;
use kryon_runtime::KryonApp;

#[derive(Parser)]
#[command(name = "kryon-renderer-ratatui")]
#[command(about = "Terminal UI renderer for Kryon .krb files")]
struct Args {
    /// Path to the .krb file to render
    krb_file: String,
    /// Inspect KRB file contents without rendering
    #[arg(long)]
    inspect: bool,
    
    /// Enable standalone rendering mode (auto-wrap non-App elements)
    #[arg(long)]
    standalone: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        let _ = cleanup_terminal();
        original_hook(panic_info);
    }));

    if !Path::new(&args.krb_file).exists() {
        anyhow::bail!("KRB file not found: {}", args.krb_file);
    }

    if args.inspect {
        return inspect_krb_file(&args.krb_file);
    }

    let result = run(&args);

    cleanup_terminal()?;

    if let Err(e) = result {
        eprintln!("Application exited with an error: {:?}", e);
    }

    Ok(())
}

fn run(args: &Args) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout);
    let renderer = RatatuiRenderer::initialize(backend)?;

    let mut app =
        KryonApp::new(&args.krb_file, renderer).context("Failed to create Kryon application")?;

    println!("Starting terminal render loop... (Press 'q' to quit, click on buttons to interact)"); // This is ok, it's user-facing

    let mut last_frame_time = Instant::now();
    loop {
        if event::poll(Duration::from_millis(16))? {
            match event::read()? {
                CrosstermEvent::Key(key) if key.code == KeyCode::Char('q') || key.code == KeyCode::Esc => {
                    break;
                }
                CrosstermEvent::Resize(width, height) => {
                    let event = InputEvent::Resize {
                        size: glam::vec2(width as f32, height as f32),
                    };
                    if let Err(e) = app.handle_input(event) {
                        eprintln!("Failed to handle resize: {:?}", e);
                    }
                }
                CrosstermEvent::Mouse(mouse_event) => {
                    match mouse_event.kind {
                        MouseEventKind::Down(crossterm::event::MouseButton::Left) => {
                            let event = InputEvent::MousePress {
                                position: glam::vec2(mouse_event.column as f32, mouse_event.row as f32),
                                button: kryon_render::MouseButton::Left,
                            };
                            if let Err(e) = app.handle_input(event) {
                                eprintln!("Failed to handle mouse click: {:?}", e);
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }

        let delta_time = last_frame_time.elapsed();
        last_frame_time = Instant::now();
        if let Err(e) = app.update(delta_time) {
            eprintln!("Failed to update app: {:?}", e);
            break;
        }

        if let Err(e) = app.render() {
            eprintln!("Failed to render frame: {:?}", e);
            break;
        }
    }

    Ok(())
}

fn cleanup_terminal() -> Result<()> {
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture)?;
    Ok(())
}

fn inspect_krb_file(krb_path: &str) -> Result<()> {
    println!("🔍 Inspecting KRB file: {}", krb_path);
    let krb_file = load_krb_file(krb_path)?;
    println!("{:#?}", krb_file);
    Ok(())
}
// src/bin/kryon-renderer-ratatui.rs

use std::io::{self, Stdout};
use std::path::Path;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use clap::Parser;
use tracing::{error, info};

// Terminal specific imports
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event as CrosstermEvent, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::{CrosstermBackend, Rect};
use ratatui::Terminal;

// Kryon imports
use kryon_core::{load_krb_file, KRBFile};
use kryon_render::{InputEvent, Renderer};
use kryon_ratatui::RatatuiRenderer;
use kryon_runtime::KryonApp;

// Keep the same argument parser
#[derive(Parser)]
#[command(name = "kryon-renderer-ratatui")]
#[command(about = "Terminal UI renderer for Kryon .krb files")]
struct Args {
    /// Path to the .krb file to render
    krb_file: String,

    /// Enable debug logging
    #[arg(short, long)]
    debug: bool,
}

fn main() -> Result<()> {
    // --- Boilerplate Setup (Args, Logging, File Check) ---
    let args = Args::parse();
    init_logging(args.debug)?;
    if !Path::new(&args.krb_file).exists() {
        anyhow::bail!("KRB file not found: {}", args.krb_file);
    }
    info!("Loading KRB file: {}", args.krb_file);

    // --- Terminal and Renderer Initialization ---
    let mut terminal = setup_terminal()?;

    // Create the renderer with the CrosstermBackend from our setup function
    let backend = CrosstermBackend::new(io::stdout());
    let renderer = RatatuiRenderer::initialize(backend)?;

    // Create the main Kryon application with the new renderer
    let mut app =
        KryonApp::new(&args.krb_file, renderer).context("Failed to create Kryon application")?;

    info!("Starting terminal render loop... (Press 'q' to quit)");
    
    // --- Main Application Loop ---
    let mut last_frame_time = Instant::now();
    'main_loop: loop {
        // Handle terminal input events
        if event::poll(Duration::from_millis(16))? {
            match event::read()? {
                CrosstermEvent::Key(key) if key.code == KeyCode::Char('q') || key.code == KeyCode::Esc => {
                    info!("Exit requested.");
                    break 'main_loop;
                },
                CrosstermEvent::Resize(width, height) => {
                    let event = InputEvent::Resize { size: glam::vec2(width as f32, height as f32) };
                    if let Err(e) = app.handle_input(event) {
                        error!("Failed to handle resize: {}", e);
                    }
                },
                _ => {}, // Ignore other events for now
            }
        }

        // Update application state
        let delta_time = last_frame_time.elapsed();
        last_frame_time = Instant::now();
        if let Err(e) = app.update(delta_time) {
            error!("Failed to update app: {}", e);
            break;
        }

        // Render the UI
        if let Err(e) = app.render() {
            error!("Failed to render frame: {}", e);
            break;
        }
    }
    
    // --- Cleanup ---
    // The `cleanup_terminal` function ensures we always restore the terminal
    // to its original state, even if the loop breaks on an error.
    cleanup_terminal(&mut terminal)?;
    info!("Terminal renderer shutdown complete.");
    Ok(())
}

// --- Helper Functions for Clarity ---

fn init_logging(debug: bool) -> Result<()> {
    let level = if debug { tracing::Level::DEBUG } else { tracing::Level::INFO };
    let subscriber = tracing_subscriber::fmt().with_max_level(level).with_target(false).compact().finish();
    tracing::subscriber::set_global_default(subscriber).context("Failed to set tracing subscriber")
}

// Sets up the terminal for TUI mode
fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend).context("Failed to create terminal")
}

// Restores the terminal to its normal state
fn cleanup_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}

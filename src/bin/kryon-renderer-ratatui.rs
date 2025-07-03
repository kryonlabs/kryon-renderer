// /home/wao/lyra/proj/kryonlabs/kryon-renderer/src/bin/kryon-renderer-ratatui.rs

use std::io;
use std::panic;
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
use ratatui::prelude::CrosstermBackend;

// Kryon imports
use kryon_core::load_krb_file;
use kryon_render::{InputEvent, Renderer};
use kryon_ratatui::RatatuiRenderer;
use kryon_runtime::KryonApp;

#[derive(Parser)]
#[command(name = "kryon-renderer-ratatui")]
#[command(about = "Terminal UI renderer for Kryon .krb files")]
struct Args {
    /// Path to the .krb file to render
    krb_file: String,

    /// Enable debug logging
    #[arg(short, long)]
    debug: bool,

    /// Inspect KRB file contents without rendering
    #[arg(long)]
    inspect: bool,
}

fn main() -> Result<()> {
    // 1. Parse args *before* using them.
    let args = Args::parse();
    
    // 2. Setup Logging and the critical Panic Hook
    init_logging(args.debug)?;
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        // Attempt to clean up the terminal before the program exits from a panic.
        let _ = cleanup_terminal();
        original_hook(panic_info);
    }));

    // 3. Validate file path
    if !Path::new(&args.krb_file).exists() {
        anyhow::bail!("KRB file not found: {}", args.krb_file);
    }

    // 4. Run in Inspection Mode or Render Mode
    if args.inspect {
        // If --inspect is passed, just print the data and exit cleanly.
        return inspect_krb_file(&args.krb_file);
    }
    
    // 5. Run the main application logic, which handles its own errors.
    let result = run(&args);

    // 6. Guaranteed Cleanup: This runs AFTER run() completes, whether it was successful or not.
    cleanup_terminal()?;

    // 7. Now, we can safely return the result from the application.
    result
}

/// The main application function, which contains the setup and render loop.
fn run(args: &Args) -> Result<()> {
    // --- Terminal and Renderer Initialization ---
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    
    // Create the backend that Ratatui will draw to.
    let backend = CrosstermBackend::new(stdout);
    // The renderer now takes ownership of the backend to manage it.
    let renderer = RatatuiRenderer::initialize(backend)?;

    // Create the main Kryon application with the correctly initialized renderer
    let mut app =
        KryonApp::new(&args.krb_file, renderer).context("Failed to create Kryon application")?;

    info!("Starting terminal render loop... (Press 'q' to quit)");

    // --- Main Application Loop ---
    let mut last_frame_time = Instant::now();
    loop {
        // Handle terminal input events
        if event::poll(Duration::from_millis(16))? {
            match event::read()? {
                CrosstermEvent::Key(key) if key.code == KeyCode::Char('q') || key.code == KeyCode::Esc => {
                    info!("Exit requested.");
                    break;
                }
                CrosstermEvent::Resize(width, height) => {
                    let event = InputEvent::Resize {
                        size: glam::vec2(width as f32, height as f32),
                    };
                    if let Err(e) = app.handle_input(event) {
                        error!("Failed to handle resize: {}", e);
                    }
                }
                _ => {} // Ignore other events for now
            }
        }

        // Update application state
        let delta_time = last_frame_time.elapsed();
        last_frame_time = Instant::now();
        if let Err(e) = app.update(delta_time) {
            error!("Failed to update app: {}", e);
            break;
        }

        // Render the UI. The renderer implementation handles drawing to the terminal.
        if let Err(e) = app.render() {
            error!("Failed to render frame: {}", e);
            break;
        }
    }

    Ok(())
}


// --- Helper Functions for Clarity ---

/// Initializes the logging subscriber.
fn init_logging(debug: bool) -> Result<()> {
    let level = if debug {
        tracing::Level::DEBUG
    } else {
        tracing::Level::INFO
    };
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(level)
        .with_target(false)
        .compact()
        .finish();
    tracing::subscriber::set_global_default(subscriber).context("Failed to set tracing subscriber")
}

/// Restores the terminal to its original state. This is crucial for TUI apps.
fn cleanup_terminal() -> Result<()> {
    info!("Restoring terminal.");
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture)?;
    Ok(())
}

/// The inspection function to diagnose .krb file issues without rendering.
fn inspect_krb_file(krb_path: &str) -> Result<()> {
    println!("🔍 Inspecting KRB file: {}", krb_path);

    let krb_file = load_krb_file(krb_path).context("Could not load KRB file for inspection")?;

    println!("📊 File Statistics:");
    println!("  Root element ID: {:?}", krb_file.root_element_id);
    println!("  Total elements: {}", krb_file.elements.len());

    println!("\n📋 Elements (Position and Size):");
    let mut visible_in_terminal = 0;
    for (id, element) in &krb_file.elements {
        println!("\n  Element {}: {:?}", id, element.element_type);
        println!("    Text: {:?}", element.text);
        println!("    Visible: {}", element.visible);
        println!("    Position: ({:.1}, {:.1})", element.position.x, element.position.y);
        println!("    Size: ({:.1}, {:.1})", element.size.x, element.size.y);
        println!("    Background: {:?}", element.background_color);

        // --- DIAGNOSTIC WARNINGS ---
        // Check if the element's top-left corner is within a reasonable terminal grid.
        if element.position.x < 120.0 && element.position.y < 50.0 {
             visible_in_terminal += 1;
        } else {
            println!("    ⚠️  WARNING: Element position is likely outside typical terminal character bounds!");
        }

        if element.size.x == 0.0 || element.size.y == 0.0 {
            println!("    ⚠️  WARNING: Element has zero size and will not be visible.");
        }

        if element.text.is_empty() && element.background_color.w < 0.1 {
            println!("    ⚠️  WARNING: Element is transparent with no text; likely invisible.");
        }
    }

    println!("\n📺 Terminal Compatibility Summary (approx 120x50):");
    if visible_in_terminal == 0 {
        println!("  ❌ No elements appear to be positioned within standard terminal bounds.");
        println!("     This is the most likely reason nothing is rendering.");
        println!("     SUGGESTION: Your `kryon-ratatui` renderer needs to remap pixel coordinates to character cells.");
    } else {
        println!("  ✅ {} element(s) are positioned within potential terminal bounds.", visible_in_terminal);
        println!("     If nothing appears, check for zero-size or invisible elements.");
    }

    Ok(())
}
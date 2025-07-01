// src/bin/kryon-renderer-ratatui.rs

use std::io;
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
use kryon_render::{InputEvent, Renderer};
use kryon_ratatui::RatatuiRenderer;
use kryon_runtime::KryonApp;
use kryon_core::load_krb_file;

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

    /// Inspect KRB file contents without rendering
    #[arg(short, long)]
    inspect: bool,

    /// Force elements to be positioned within screen bounds
    #[arg(short, long)]
    force_bounds: bool,
}

fn main() -> Result<()> {
    // --- Boilerplate Setup (Args, Logging, File Check) ---
    let args = Args::parse();
    init_logging(args.debug)?;
    if !Path::new(&args.krb_file).exists() {
        anyhow::bail!("KRB file not found: {}", args.krb_file);
    }
    info!("Loading KRB file: {}", args.krb_file);

    // --- Inspect Mode ---
    if args.inspect {
        return inspect_krb_file(&args.krb_file);
    }

    // --- Terminal and Renderer Initialization ---
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    
    // Create the renderer with the same backend we're using for terminal management
    let backend = CrosstermBackend::new(stdout);
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
    // Restore the terminal to its original state
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture)?;
    info!("Terminal renderer shutdown complete.");
    Ok(())
}

// --- Helper Functions for Clarity ---

fn init_logging(debug: bool) -> Result<()> {
    let level = if debug { tracing::Level::DEBUG } else { tracing::Level::INFO };
    let subscriber = tracing_subscriber::fmt().with_max_level(level).with_target(false).compact().finish();
    tracing::subscriber::set_global_default(subscriber).context("Failed to set tracing subscriber")
}

fn inspect_krb_file(krb_path: &str) -> Result<()> {
    println!("🔍 Inspecting KRB file: {}", krb_path);
    
    let krb_file = load_krb_file(krb_path)?;
    
    println!("📊 File Statistics:");
    println!("  Root element ID: {:?}", krb_file.root_element_id);
    println!("  Total elements: {}", krb_file.elements.len());
    println!("  Scripts: {}", krb_file.scripts.len());
    
    println!("\n📋 Elements:");
    for (id, element) in &krb_file.elements {
        println!("  Element {id}:");
        println!("    Position: {:?}", element.position);
        println!("    Size: {:?}", element.size);
        println!("    Text: {:?}", element.text);
        println!("    Visible: {}", element.visible);
        println!("    Background: {:?}", element.background_color);
        println!("    Text Color: {:?}", element.text_color);
        
        if element.position.x > 100.0 || element.position.y > 30.0 {
            println!("    ⚠️  WARNING: Element positioned outside typical terminal bounds!");
        }
        
        if element.text.is_empty() && element.background_color.w == 0.0 {
            println!("    ⚠️  WARNING: Element has no visible content (no text, no background)!");
        }
        
        if element.size.x == 0.0 || element.size.y == 0.0 {
            println!("    ⚠️  WARNING: Element has zero size!");
        }
        
        println!();
    }
    
    // Check if anything would be visible in an 80x25 terminal
    let mut visible_elements = 0;
    for element in krb_file.elements.values() {
        if element.visible && 
           element.position.x < 80.0 && element.position.y < 25.0 &&
           (!element.text.is_empty() || element.background_color.w > 0.0) &&
           element.size.x > 0.0 && element.size.y > 0.0 {
            visible_elements += 1;
        }
    }
    
    println!("📺 Terminal Compatibility (80x25):");
    if visible_elements == 0 {
        println!("  ❌ No elements would be visible in a standard terminal!");
        println!("     Consider using --force-bounds to move elements into view.");
    } else {
        println!("  ✅ {} element(s) would be visible", visible_elements);
    }
    
    Ok(())
}

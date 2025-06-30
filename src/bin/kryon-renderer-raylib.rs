use std::path::Path;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use clap::Parser;
use glam::Vec2;
use tracing::{error, info};

use kryon_render::Renderer;
use kryon_runtime::KryonApp;
use kryon_raylib::RaylibRenderer;

#[derive(Parser)]
#[command(name = "kryon-renderer-raylib")]
#[command(about = "Raylib-based renderer for Kryon .krb files")]
struct Args {
    /// Path to the .krb file to render
    krb_file: String,

    /// Window width
    #[arg(long, default_value = "800")]
    width: i32,

    /// Window height
    #[arg(long, default_value = "600")]
    height: i32,

    /// Window title
    #[arg(long, default_value = "Kryon Raylib Renderer")]
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

    info!("Initializing Raylib renderer for: {}", args.krb_file);
    
    // Initialize renderer
    let renderer = RaylibRenderer::initialize((args.width, args.height, args.title))
        .context("Failed to initialize Raylib renderer")?;

    // Create Kryon app
    let mut app = KryonApp::new(&args.krb_file, renderer)
        .context("Failed to create Kryon application")?;

    info!("Starting Raylib render loop...");
    
    let mut last_frame_time = Instant::now();
    
    loop {
        // Check if window should close
        if app.renderer().backend().should_close() {
            info!("Window close requested");
            break;
        }
        
        let now = Instant::now();
        let delta_time = now.duration_since(last_frame_time);
        last_frame_time = now;
        
        // Poll and handle input events
        let input_events = app.renderer_mut().backend_mut().poll_input_events();
        for event in input_events {
            if let Err(e) = app.handle_input(event) {
                error!("Failed to handle input event: {}", e);
            }
        }
        
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
    }
    
    info!("Raylib renderer shutdown complete");
    Ok(())
}
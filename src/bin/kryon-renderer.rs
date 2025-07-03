// /home/wao/lyra/proj/kryonlabs/kryon-renderer/src/bin/kryon-renderer-raylib.rs
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

    /// Window width. Overrides the value in the KRB file.
    #[arg(long)]
    width: Option<i32>,

    /// Window height. Overrides the value in the KRB file.
    #[arg(long)]
    height: Option<i32>,

    /// Window title. Overrides the value in the KRB file.
    #[arg(long)]
    title: Option<String>,

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

    info!("Loading KRB file: {}", args.krb_file);
    
    // Load the application definition first to get window properties
    let krb_file = kryon_core::load_krb_file(&args.krb_file)
        .context("Failed to load KRB file to read window properties")?;

    // Set default values
    let mut width = 800;
    let mut height = 600;
    let mut title = "Kryon Raylib Renderer".to_string();

    // Read properties from the KRB file's root element
    if let Some(root_id) = krb_file.root_element_id {
        if let Some(root_element) = krb_file.elements.get(&root_id) {
            if root_element.size.x > 0.0 {
                width = root_element.size.x as i32;
            }
            if root_element.size.y > 0.0 {
                height = root_element.size.y as i32;
            }
            if !root_element.text.is_empty() {
                title = root_element.text.clone();
            }
        }
    }

    // Allow CLI arguments to override KRB file properties
    let final_width = args.width.unwrap_or(width);
    let final_height = args.height.unwrap_or(height);
    let final_title = args.title.clone().unwrap_or(title);
    
    info!("Initializing Raylib renderer with properties: {}x{} '{}'", final_width, final_height, &final_title);
    
    // Initialize renderer with the final, resolved properties
    let renderer = RaylibRenderer::initialize((final_width, final_height, final_title))
        .context("Failed to initialize Raylib renderer")?;

    // Create Kryon app, passing the original krb_file path
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
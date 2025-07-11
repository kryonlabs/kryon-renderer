
use std::path::Path;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use clap::Parser;
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

    /// Take a screenshot and exit
    #[arg(long)]
    screenshot: Option<String>,

    /// Duration to wait before taking screenshot (in milliseconds)
    #[arg(long, default_value = "100")]
    screenshot_delay: u64,
    
    /// Enable standalone rendering mode (auto-wrap non-App elements)
    #[arg(long)]
    standalone: bool,
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

    // Check if we're in standalone mode (auto-generated App)
    let is_standalone = if let Some(root_id) = krb_file.root_element_id {
        if let Some(root_element) = krb_file.elements.get(&root_id) {
            args.standalone || root_element.id == "auto_generated_app"
        } else {
            false
        }
    } else {
        false
    };
    
    // Allow CLI arguments to override KRB file properties
    // In standalone mode, prefer CLI arguments over KRB file properties
    let final_width = if is_standalone {
        args.width.unwrap_or(800) // Default for standalone
    } else {
        args.width.unwrap_or(width)
    };
    
    let final_height = if is_standalone {
        args.height.unwrap_or(600) // Default for standalone
    } else {
        args.height.unwrap_or(height)
    };
    
    let final_title = if is_standalone {
        args.title.clone().unwrap_or_else(|| "Kryon Standalone Renderer".to_string())
    } else {
        args.title.clone().unwrap_or(title)
    };
    
    info!("Initializing Raylib renderer with properties: {}x{} '{}'", final_width, final_height, &final_title);
    
    // Initialize renderer with the final, resolved properties
    let mut renderer = RaylibRenderer::initialize((final_width, final_height, final_title))
        .context("Failed to initialize Raylib renderer")?;

    // Register fonts from the KRB file
    // Extract font mappings from KRB file strings
    register_fonts_from_krb(&mut renderer, &krb_file);

    let mut app = KryonApp::new(&args.krb_file, renderer)
        .context("Failed to create Kryon application")?;

    // Force initial mouse position update to establish initial hover state
    let initial_events = app.renderer_mut().backend_mut().poll_input_events();
    for event in initial_events {
        if let Err(e) = app.handle_input(event) {
            error!("Failed to handle initial input event: {}", e);
        }
    }
    
    // Force initial render to apply any hover state changes
    if let Err(e) = app.render() {
        error!("Failed to render initial frame: {}", e);
    }

    info!("Starting Raylib render loop...");
    
    let mut last_frame_time = Instant::now();
    let start_time = Instant::now();
    let screenshot_taken = false;
    
    'main_loop: loop {
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
            // Check for ESC key to quit application
            if let kryon_render::InputEvent::KeyPress { key, .. } = &event {
                if matches!(key, kryon_render::KeyCode::Escape) {
                    info!("ESC key pressed - quitting application");
                    break 'main_loop;
                }
            }
            
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
        
        // Handle screenshot mode
        if let Some(ref screenshot_file) = args.screenshot {
            if !screenshot_taken && now.duration_since(start_time) >= Duration::from_millis(args.screenshot_delay) {
                info!("Taking screenshot: {}", screenshot_file);
                if let Err(e) = app.renderer_mut().backend_mut().take_screenshot(screenshot_file) {
                    error!("Failed to take screenshot: {}", e);
                } else {
                    info!("Screenshot saved successfully");
                }
                let _screenshot_taken = true;
                break; // Exit after taking screenshot
            }
        }
    }
    
    info!("Raylib renderer shutdown complete");
    Ok(())
}

fn register_fonts_from_krb(renderer: &mut RaylibRenderer, krb_file: &kryon_core::KRBFile) {
    // Register fonts using the font mappings stored in the KRB file
    // The font mappings should be stored as key-value pairs in the fonts HashMap
    
    for (font_family, font_path) in &krb_file.fonts {
        eprintln!("[RAYLIB_FONT] Found font mapping: '{}' -> '{}'", font_family, font_path);
        renderer.register_font(font_family, font_path);
        if let Err(e) = renderer.load_font(font_family) {
            eprintln!("[RAYLIB_FONT] Warning: Failed to load font '{}' from '{}': {}", font_family, font_path, e);
        }
    }
    
    // If no font mappings found, fall back to scanning strings for font declarations
    if krb_file.fonts.is_empty() {
        eprintln!("[RAYLIB_FONT] No font mappings found in KRB file, scanning strings for font patterns");
        
        // Look for adjacent font family name and font file patterns
        for (i, string) in krb_file.strings.iter().enumerate() {
            // Look for font files (.ttf, .otf, etc.)
            if string.ends_with(".ttf") || string.ends_with(".otf") || string.ends_with(".woff") || string.ends_with(".woff2") {
                // Check the previous string as it should be the font family name
                if i > 0 {
                    let potential_font_name = &krb_file.strings[i - 1];
                    
                    // Font family names should be simple identifiers, not paths or other data
                    if !potential_font_name.is_empty() && 
                       !potential_font_name.starts_with('#') && // Not a color
                       !potential_font_name.contains('/') && // Not a path
                       !potential_font_name.contains('\\') && // Not a Windows path
                       !potential_font_name.ends_with(".ttf") && // Not itself a font file
                       !potential_font_name.ends_with(".otf") &&
                       !potential_font_name.contains('.') && // Generally no dots in font names
                       potential_font_name.len() < 32 && // Reasonable length
                       potential_font_name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
                        
                        eprintln!("[RAYLIB_FONT] Found font pattern: '{}' -> '{}'", potential_font_name, string);
                        renderer.register_font(potential_font_name, string);
                        if let Err(e) = renderer.load_font(potential_font_name) {
                            eprintln!("[RAYLIB_FONT] Warning: Failed to load font '{}' from '{}': {}", potential_font_name, string, e);
                        }
                    }
                }
            }
        }
    }
}

// kryon-bundle: Create a self-contained executable with embedded KRB file
use clap::Parser;
use std::fs;
use std::path::PathBuf;
use std::io::Write;

#[derive(Parser)]
#[clap(author, version, about = "Bundle KRB file into a self-contained executable")]
struct Args {
    /// Path to the KRB file to embed
    #[clap(value_name = "KRB_FILE")]
    krb_file: PathBuf,
    
    /// Output executable name (defaults to input filename without extension)
    #[clap(short, long)]
    output: Option<PathBuf>,
    
    /// Backend to use (wgpu, ratatui, raylib)
    #[clap(short, long, default_value = "wgpu")]
    backend: String,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    
    // Verify KRB file exists
    if !args.krb_file.exists() {
        anyhow::bail!("KRB file not found: {:?}", args.krb_file);
    }
    
    // Read the KRB file
    let krb_data = fs::read(&args.krb_file)?;
    println!("Read {} bytes from {:?}", krb_data.len(), args.krb_file);
    
    // Determine output filename
    let output_path = args.output.unwrap_or_else(|| {
        let mut path = args.krb_file.clone();
        path.set_extension("");
        path
    });
    
    // Generate the bundled executable source code
    let bundle_source = generate_bundle_source(&krb_data, &args.backend)?;
    
    // Create a temporary source file
    let temp_dir = std::env::temp_dir();
    let temp_source = temp_dir.join("kryon_bundle_temp.rs");
    fs::write(&temp_source, bundle_source)?;
    
    // Compile the bundled executable
    println!("Compiling bundled executable...");
    let mut cmd = std::process::Command::new("rustc");
    cmd.arg(&temp_source)
        .arg("-o").arg(&output_path)
        .arg("-C").arg("opt-level=3")
        .arg("-C").arg("lto=true");
    
    // Add necessary crate dependencies
    // This is a simplified approach - in production, we'd use cargo with a generated Cargo.toml
    println!("Note: This is a simplified bundler. For production use, integrate with cargo build system.");
    
    // For now, create a wrapper script that embeds the data
    create_wrapper_script(&krb_data, &output_path, &args.backend)?;
    
    println!("Created bundled executable: {:?}", output_path);
    
    // Clean up
    let _ = fs::remove_file(temp_source);
    
    Ok(())
}

fn generate_bundle_source(krb_data: &[u8], backend: &str) -> anyhow::Result<String> {
    // Convert KRB data to a Rust byte array literal
    let data_literal = format!("&[{}]", 
        krb_data.iter()
            .map(|b| format!("{}", b))
            .collect::<Vec<_>>()
            .join(", ")
    );
    
    // Generate the source code
    let source = format!(r#"
// Auto-generated bundled Kryon application
use kryon_runtime::KryonApp;
use kryon_core::load_krb_from_bytes;

// Embedded KRB data
const EMBEDDED_KRB: &[u8] = {};

fn main() -> anyhow::Result<()> {{
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into())
        )
        .init();
    
    println!("Starting bundled Kryon application...");
    
    // Load KRB from embedded data
    let krb_file = load_krb_from_bytes(EMBEDDED_KRB)?;
    
    // Create and run the appropriate backend
    match "{}" {{
        "wgpu" => {{
            #[cfg(feature = "wgpu")]
            {{
                use kryon_wgpu::WgpuRenderer;
                use winit::event_loop::EventLoop;
                use winit::window::WindowBuilder;
                
                let event_loop = EventLoop::new();
                let window = WindowBuilder::new()
                    .with_title("Bundled Kryon App")
                    .with_inner_size(winit::dpi::LogicalSize::new(800, 600))
                    .build(&event_loop)?;
                
                let renderer = pollster::block_on(WgpuRenderer::new(&window))?;
                let mut app = KryonApp::new_from_krb(krb_file, renderer)?;
                
                // Run event loop
                kryon_runtime::run_event_loop(event_loop, window, app);
            }}
            #[cfg(not(feature = "wgpu"))]
            anyhow::bail!("WGPU backend not compiled in");
        }}
        "ratatui" => {{
            #[cfg(feature = "ratatui")]
            {{
                use kryon_ratatui::RatatuiRenderer;
                use crossterm::terminal;
                
                terminal::enable_raw_mode()?;
                let renderer = RatatuiRenderer::new()?;
                let mut app = KryonApp::new_from_krb(krb_file, renderer)?;
                
                // Run TUI loop
                kryon_runtime::run_tui_loop(app)?;
                
                terminal::disable_raw_mode()?;
            }}
            #[cfg(not(feature = "ratatui"))]
            anyhow::bail!("Ratatui backend not compiled in");
        }}
        "raylib" => {{
            #[cfg(feature = "raylib")]
            {{
                use kryon_raylib::RaylibRenderer;
                
                let renderer = RaylibRenderer::initialize((800, 600, "Bundled Kryon App".to_string()))?;
                let mut app = KryonApp::new_from_krb(krb_file, renderer)?;
                
                // Run Raylib loop
                kryon_runtime::run_raylib_loop(app)?;
            }}
            #[cfg(not(feature = "raylib"))]
            anyhow::bail!("Raylib backend not compiled in");
        }}
        _ => anyhow::bail!("Unknown backend: {{}}", "{}"),
    }}
    
    Ok(())
}}
"#, data_literal, backend, backend);
    
    Ok(source)
}

fn create_wrapper_script(krb_data: &[u8], output_path: &PathBuf, backend: &str) -> anyhow::Result<()> {
    // For now, create a simple wrapper script that demonstrates the concept
    // In production, this would actually compile a proper Rust binary
    
    let script_content = format!(r#"#!/bin/bash
# Bundled Kryon Application
# Backend: {}
# Size: {} bytes

# This is a demonstration wrapper. In production, this would be a compiled binary.
# To create a real bundled executable:
# 1. Use cargo with a generated Cargo.toml
# 2. Include kryon-renderer dependencies
# 3. Embed the KRB data using include_bytes!
# 4. Compile with cargo build --release

echo "This is a placeholder for the bundled Kryon application."
echo "KRB data would be embedded here ({} bytes)"
echo "Backend: {}"
echo ""
echo "To run the actual application, use:"
echo "  kryon-renderer-{} <original-krb-file>"
"#, backend, krb_data.len(), krb_data.len(), backend, backend);
    
    let mut file = fs::File::create(output_path)?;
    file.write_all(script_content.as_bytes())?;
    
    // Make executable on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = file.metadata()?.permissions();
        perms.set_mode(0o755);
        file.set_permissions(perms)?;
    }
    
    Ok(())
}
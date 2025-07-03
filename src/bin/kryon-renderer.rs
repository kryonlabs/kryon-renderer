use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::path::Path;
use std::process::Command;

#[derive(Parser)]
#[command(name = "kryon-renderer")]
#[command(about = "Kryon renderer with multiple backends")]
#[command(version)]
struct Args {
    #[command(subcommand)]
    command: RenderCommand,
}

#[derive(Subcommand)]
enum RenderCommand {
    /// Render with WGPU backend (GPU-accelerated desktop)
    Wgpu {
        /// Path to the .krb file to render
        krb_file: String,
        /// Window width
        #[arg(long)]
        width: Option<i32>,
        /// Window height  
        #[arg(long)]
        height: Option<i32>,
        /// Window title
        #[arg(long)]
        title: Option<String>,
        /// Enable debug logging
        #[arg(short, long)]
        debug: bool,
    },
    /// Render with Ratatui backend (terminal UI)
    Ratatui {
        /// Path to the .krb file to render
        krb_file: String,
        /// Enable debug logging
        #[arg(short, long)]
        debug: bool,
    },
    /// Render with Raylib backend (simple graphics)
    Raylib {
        /// Path to the .krb file to render
        krb_file: String,
        /// Window width
        #[arg(long)]
        width: Option<i32>,
        /// Window height
        #[arg(long)]
        height: Option<i32>,
        /// Window title
        #[arg(long)]
        title: Option<String>,
        /// Enable debug logging
        #[arg(short, long)]
        debug: bool,
    },
    /// Debug renderer (text hierarchy output)
    Debug {
        /// Path to the .krb file to analyze
        krb_file: String,
        /// Output format
        #[arg(long, default_value = "tree")]
        format: String,
        /// Save output to file
        #[arg(long)]
        output: Option<String>,
    },
}

fn main() -> Result<()> {
    let args = Args::parse();

    match args.command {
        RenderCommand::Wgpu { krb_file, width, height, title, debug } => {
            validate_krb_file(&krb_file)?;
            
            let mut cmd_args = vec![];
            if let Some(w) = width {
                cmd_args.extend_from_slice(&["--width", &w.to_string()]);
            }
            if let Some(h) = height {
                cmd_args.extend_from_slice(&["--height", &h.to_string()]);
            }
            if let Some(t) = title {
                cmd_args.extend_from_slice(&["--title", &t]);
            }
            if debug {
                cmd_args.push("--debug");
            }
            cmd_args.push(&krb_file);
            
            run_backend_binary("kryon-renderer-wgpu", &cmd_args)
        }
        
        RenderCommand::Ratatui { krb_file, debug } => {
            validate_krb_file(&krb_file)?;
            
            let mut cmd_args = vec![];
            if debug {
                cmd_args.push("--debug");
            }
            cmd_args.push(&krb_file);
            
            run_backend_binary("kryon-renderer-ratatui", &cmd_args)
        }
        
        RenderCommand::Raylib { krb_file, width, height, title, debug } => {
            validate_krb_file(&krb_file)?;
            
            let mut cmd_args = vec![];
            if let Some(w) = width {
                cmd_args.extend_from_slice(&["--width", &w.to_string()]);
            }
            if let Some(h) = height {
                cmd_args.extend_from_slice(&["--height", &h.to_string()]);
            }
            if let Some(t) = title {
                cmd_args.extend_from_slice(&["--title", &t]);
            }
            if debug {
                cmd_args.push("--debug");
            }
            cmd_args.push(&krb_file);
            
            run_backend_binary("kryon-renderer-raylib", &cmd_args)
        }
        
        RenderCommand::Debug { krb_file, format, output } => {
            validate_krb_file(&krb_file)?;
            
            let mut cmd_args = vec!["--format", &format];
            if let Some(out) = &output {
                cmd_args.extend_from_slice(&["--output", out]);
            }
            cmd_args.push(&krb_file);
            
            run_backend_binary("kryon-renderer-debug", &cmd_args)
        }
    }
}

fn validate_krb_file(path: &str) -> Result<()> {
    if !Path::new(path).exists() {
        anyhow::bail!("KRB file not found: {}", path);
    }
    if !path.ends_with(".krb") {
        anyhow::bail!("File must have .krb extension: {}", path);
    }
    Ok(())
}

fn run_backend_binary(binary_name: &str, args: &[&str]) -> Result<()> {
    let mut cmd = Command::new(binary_name);
    cmd.args(args);
    
    let status = cmd.status()
        .with_context(|| format!("Failed to run {}", binary_name))?;
    
    if !status.success() {
        anyhow::bail!("{} exited with status: {}", binary_name, status);
    }
    
    Ok(())
}
# Kryon Renderer

A modular Rust renderer for the Kryon UI framework with pluggable rendering backends.

## Architecture

***
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   kryon-core    │    │  kryon-layout   │    │  kryon-render   │
│                 │    │                 │    │                 │
│ • KRB Parser    │    │ • Layout Engine │    │ • Render Traits │
│ • Element Types │    │ • Flexbox       │    │ • Commands      │
│ • Properties    │    │ • Constraints   │    │ • Events        │
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │                       │                       │
         └───────────────────────┼───────────────────────┘
                                 │
              ┌─────────────────────────────────┐
              │        kryon-runtime            │
              │                                 │
              │ • App Management               │
              │ • Event System                 │
              │ • Script Integration           │
              │ • Backend Selection            │
              └─────────────────────────────────┘
                                 │
                    ┌────────────┴────────────┐
                    │                         │
         ┌─────────────────┐       ┌─────────────────┐
         │   kryon-wgpu    │       │ kryon-ratatui   │
         │                 │       │                 │
         │ • GPU Rendering │       │ • Terminal UI   │
         │ • Cross-platform│       │ • Text Mode     │
         │ • High Performance      │ • ASCII Graphics│
         └─────────────────┘       └─────────────────┘
***

## Features

- **Modular Backend System**: Easy to switch between rendering backends
- **WGPU Backend**: High-performance GPU rendering for desktop/mobile/web
- **Ratatui Backend**: Terminal-based UI for CLI applications
- **Flexbox Layout**: Modern layout engine with flexible box model
- **Script Integration**: Support for Lua, JavaScript, Python, and Wren
- **Component System**: Reusable UI components with properties
- **Event System**: Mouse, keyboard, and custom event handling

## Quick Start

### WGPU (GPU) Rendering

***bash
cargo run --manifest-path examples/basic-wgpu/Cargo.toml
***

### Terminal (Ratatui) Rendering

***bash
cargo run --manifest-path examples/terminal-ui/Cargo.toml
***

## Usage

***rust
use kryon_runtime::{KryonApp, WgpuRenderer};

// Create renderer
let renderer = WgpuRenderer::initialize(surface)?;

// Load app from KRB file
let mut app = KryonApp::new("path/to/app.krb", renderer)?;

// Game loop
loop {
    app.update(delta_time)?;
    app.render()?;
}
***

## Backend Selection

Choose your rendering backend based on your needs:

- **WGPU**: Best for desktop apps, games, and high-performance UIs
- **Ratatui**: Perfect for CLI tools, server applications, and terminal UIs

## Adding New Backends

1. Create a new crate `kryon-{backend}`
2. Implement the `Renderer` and `CommandRenderer` traits
3. Add feature flags to `kryon-runtime`
4. Update backend selection in `backends.rs`

## Building

***bash
# Build all crates
cargo build --workspace

# Build with specific features
cargo build --features wgpu
cargo build --features ratatui

# Build examples
chmod +x build.sh
./build.sh
***

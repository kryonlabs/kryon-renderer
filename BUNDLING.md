# Kryon Application Bundling

This document describes how to create standalone, self-contained executables from Kryon applications.

## Overview

Kryon supports bundling KRB files directly into executable binaries, creating standalone applications that don't require separate KRB files or runtime dependencies. This is similar to how Electron apps bundle their resources.

## Quick Start

### Using the Bundle Script (Recommended)

```bash
# Bundle a KRB file into a standalone executable
./bundle_krb.sh examples/01_getting_started/hello_world.krb wgpu hello_world_app

# Run the standalone executable
./hello_world_app
```

### Manual Building

```bash
# Set the KRB file to embed
export KRYON_EMBEDDED_KRB_PATH="/absolute/path/to/your/app.krb"

# Build with embedded KRB and desired backend
cargo build --release \
    --bin kryon-standalone \
    --features "embedded_krb,wgpu" \
    --no-default-features
```

## Supported Backends

- **WGPU**: High-performance GPU rendering (recommended for desktop)
- **Ratatui**: Terminal-based UI (for CLI applications)
- **Raylib**: Simple 2D/3D graphics (good for games and multimedia)

## Bundle Script Usage

```bash
./bundle_krb.sh <krb_file> [backend] [output_name]
```

### Arguments

- **`krb_file`**: Path to the .krb file to bundle (required)
- **`backend`**: Rendering backend - `wgpu`, `ratatui`, or `raylib` (default: `wgpu`)
- **`output_name`**: Name of the output executable (default: KRB filename without extension)

### Examples

```bash
# Bundle hello world with WGPU backend
./bundle_krb.sh examples/01_getting_started/hello_world.krb wgpu hello_world

# Bundle counter app with Ratatui (terminal) backend
./bundle_krb.sh examples/02_basic_ui/counter_simple.krb ratatui counter_tui

# Bundle calculator with Raylib backend
./bundle_krb.sh examples/07_advanced/calculator.krb raylib calc_app
```

## Implementation Details

### How It Works

1. **KRB Embedding**: The KRB file is embedded as a byte array using Rust's `include_bytes!` macro
2. **Conditional Compilation**: Only the specified backend is compiled into the executable
3. **Runtime Loading**: The embedded KRB data is parsed at runtime using `load_krb_from_bytes()`
4. **Self-Contained**: No external files or dependencies are needed

### Technical Components

- **`kryon-standalone.rs`**: Template binary for standalone executables
- **`embedded_krb` feature**: Enables KRB embedding via environment variable
- **`load_krb_from_bytes()`**: Function to parse KRB data from memory
- **`KryonApp::new_with_krb()`**: Creates app from pre-loaded KRB file

### File Structure

```
kryon-renderer/
├── src/bin/
│   ├── kryon-standalone.rs    # Standalone executable template
│   └── kryon-bundle.rs        # Advanced bundling tool (WIP)
├── bundle_krb.sh              # Simple bundling script
└── BUNDLING.md                # This documentation
```

## Advanced Usage

### Custom Build Configuration

For advanced use cases, you can manually configure the build:

```bash
# Build with specific optimizations
KRYON_EMBEDDED_KRB_PATH="/path/to/app.krb" \
RUSTFLAGS="-C target-cpu=native" \
cargo build --release \
    --bin kryon-standalone \
    --features "embedded_krb,wgpu" \
    --no-default-features
```

### Cross-Platform Builds

```bash
# Build for Windows from Linux
KRYON_EMBEDDED_KRB_PATH="/path/to/app.krb" \
cargo build --release \
    --target x86_64-pc-windows-gnu \
    --bin kryon-standalone \
    --features "embedded_krb,wgpu" \
    --no-default-features
```

### Minimizing Binary Size

```bash
# Build with size optimizations
KRYON_EMBEDDED_KRB_PATH="/path/to/app.krb" \
RUSTFLAGS="-C opt-level=z -C lto=fat -C codegen-units=1" \
cargo build --release \
    --bin kryon-standalone \
    --features "embedded_krb,ratatui" \
    --no-default-features
```

## Distribution

### Generated Files

When bundling completes, you get:

- **`app_name`**: The standalone executable
- **`app_name_info.txt`**: Information about the build

### Distribution Checklist

- ✅ Single executable file
- ✅ No external dependencies
- ✅ Self-contained KRB data
- ✅ Platform-specific binary
- ✅ Optimized for distribution

### Platform Considerations

- **Linux**: Executable runs on most distributions
- **macOS**: May require code signing for distribution
- **Windows**: `.exe` extension added automatically
- **WASM**: Use the web backend instead of bundling

## Troubleshooting

### Common Issues

#### "No backend enabled" Error

```
No backend enabled! Build with one of: --features wgpu, --features ratatui, --features raylib
```

**Solution**: Ensure you're building with a backend feature:
```bash
cargo build --features "embedded_krb,wgpu"
```

#### "Failed to load embedded KRB data" Error

**Possible causes**:
- KRB file path is incorrect
- KRB file is corrupted
- Environment variable not set correctly

**Solution**: Verify the KRB file and path:
```bash
ls -la "$KRYON_EMBEDDED_KRB_PATH"
file "$KRYON_EMBEDDED_KRB_PATH"
```

#### Large Binary Size

**Solutions**:
- Use the `ratatui` backend for smaller binaries
- Build with size optimizations (see Advanced Usage)
- Strip debug symbols: `strip your_app`

### Debug Mode

Build in debug mode for troubleshooting:

```bash
KRYON_EMBEDDED_KRB_PATH="/path/to/app.krb" \
cargo build \
    --bin kryon-standalone \
    --features "embedded_krb,wgpu"
```

## Performance Considerations

### Backend Comparison

| Backend  | Binary Size | Startup Time | Performance | Use Case |
|----------|-------------|--------------|-------------|----------|
| WGPU     | ~15-25 MB   | Medium       | High        | Desktop apps |
| Ratatui  | ~5-10 MB    | Fast         | Medium      | CLI tools |
| Raylib   | ~8-15 MB    | Fast         | High        | Games/multimedia |

### Optimization Tips

1. **Choose the right backend** for your use case
2. **Use release builds** for distribution
3. **Enable LTO** for smaller binaries
4. **Strip symbols** for production builds
5. **Consider UPX** for additional compression

## Future Enhancements

### Planned Features

- 🔄 **Asset bundling**: Embed fonts, images, and other resources
- 🔄 **Multi-file bundling**: Bundle multiple KRB files as modules
- 🔄 **Compression**: Automatic KRB data compression
- 🔄 **Code signing**: Integrated platform-specific signing
- 🔄 **Installer generation**: Create platform installers

### Advanced Bundling Tool

The `kryon-bundle` binary (in development) will provide:

- Interactive bundling wizard
- Asset discovery and bundling
- Cross-platform build management
- Distribution package creation

## Examples

See the bundled examples in the repository:

- `examples/01_getting_started/hello_world.krb` - Simple hello world
- `examples/02_basic_ui/counter_simple.krb` - Interactive counter
- `examples/07_advanced/calculator.krb` - Complex calculator app

Each can be bundled into a standalone executable for easy distribution.
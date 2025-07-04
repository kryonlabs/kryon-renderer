# Kryon Renderer

A high-performance, cross-platform UI renderer for KRB (Kryon Binary) files with multiple backend support.

## Architecture

```
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
                    ┌────────────┼────────────┐
                    │            │            │
         ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐
         │   kryon-wgpu    │  │ kryon-ratatui   │  │  kryon-raylib   │
         │                 │  │                 │  │                 │
         │ • GPU Rendering │  │ • Terminal UI   │  │ • 2D/3D Graphics│
         │ • Cross-platform│  │ • Text Mode     │  │ • Game-focused  │
         │ • High Performance  │ • ASCII Graphics│  │ • Easy Setup    │
         └─────────────────┘  └─────────────────┘  └─────────────────┘
```

## Features

- **Multiple Rendering Backends**: Choose the best backend for your use case
  - **WGPU Backend**: High-performance GPU rendering for desktop/mobile/web
  - **Ratatui Backend**: Terminal-based UI for CLI applications
  - **Raylib Backend**: Simple 2D/3D graphics for games and multimedia apps
- **Debug Tools**: Comprehensive debugging and inspection capabilities
- **Comprehensive Testing**: Snapshot testing and screenshot-based visual verification
- **Cross-platform**: Runs on Windows, macOS, Linux, and web browsers

## Building

```bash
# Build all backends
cargo build --workspace

# Build specific backend only
cargo build --no-default-features --features wgpu     # GPU rendering
cargo build --no-default-features --features ratatui  # Terminal rendering
cargo build --no-default-features --features raylib   # Raylib rendering

# Build with specific features
cargo build --features wgpu
cargo build --features ratatui
cargo build --features raylib
```

## Usage

### Running with Different Backends

```bash
# WGPU backend (high-performance)
cargo run --features wgpu --bin kryon-renderer-wgpu -- examples/01_getting_started/hello_world.krb

# Ratatui backend (terminal)
cargo run --features ratatui --bin kryon-renderer-ratatui -- examples/01_getting_started/hello_world.krb

# Raylib backend (simple graphics)
cargo run --features raylib --bin kryon-renderer-raylib -- examples/01_getting_started/hello_world.krb

# With custom window size (raylib example)
cargo run --features raylib --bin kryon-renderer-raylib -- examples/01_getting_started/hello_world.krb --width 1024 --height 768
```

### Screenshot Capture

The raylib backend supports screenshot capture for testing and debugging:

```bash
# Take a screenshot and save it
cargo run --features raylib --bin kryon-renderer-raylib -- examples/01_getting_started/hello_world.krb --screenshot output.png

# Take screenshot with delay (useful for letting the UI settle)
cargo run --features raylib --bin kryon-renderer-raylib -- examples/01_getting_started/hello_world.krb --screenshot output.png --screenshot-delay 500
```

## Debug Renderer

Use the debug renderer to inspect KRB file structure, element hierarchy, and properties:

```bash
# Basic tree view
cargo run --bin kryon-renderer-debug -- examples/01_getting_started/hello_world.krb

# Show all properties and layout information
cargo run --bin kryon-renderer-debug -- examples/01_getting_started/hello_world.krb --show-properties --show-layout --show-colors

# Export to JSON format
cargo run --bin kryon-renderer-debug -- examples/01_getting_started/hello_world.krb --format json

# Generate detailed analysis
cargo run --bin kryon-renderer-debug -- examples/01_getting_started/hello_world.krb --format detailed --show-properties

# Save output to file
cargo run --bin kryon-renderer-debug -- examples/01_getting_started/hello_world.krb --output debug_output.txt
```

### Debug Output Example

```
App pos:(0,0) size:(800,600)
    • style_id: 1
└── Container pos:(200,100) size:(200,100)
│       • layout_flags: 0x05
│       • style_id: 2
│   └── Text "Hello World" pos:(256,142) size:(88,16)
│   │       • text: "Hello World"
│   │       • text_alignment: Center
```

## Testing

This project uses multiple testing strategies to ensure correctness and prevent regressions.

### Running Tests

```bash
# Run all tests
cargo test

# Run debug renderer tests
cargo test --test debug_renderer_test

# Run screenshot-based tests (requires graphics environment)
cargo test --test screenshot_test

# Run snapshot tests for ratatui backend
cargo test -p kryon-ratatui
```

### Test Categories

#### 1. Debug Renderer Tests (`tests/debug_renderer_test.rs`)

Validates the debug renderer functionality:

- **Basic output verification**: Ensures tree structure is correct
- **Properties testing**: Verifies all element properties are displayed
- **Format testing**: Tests JSON and detailed output formats
- **Error handling**: Tests invalid file handling
- **Property validation**: Ensures all expected properties are shown

```bash
# Run specific debug renderer tests
cargo test test_debug_renderer_basic_output
cargo test test_debug_renderer_with_properties
cargo test test_debug_renderer_json_output
```

#### 2. Screenshot Tests (`tests/screenshot_test.rs`)

Visual regression testing using the raylib backend:

- **Golden screenshot comparison**: Establishes reference images and compares future renders
- **Basic screenshot generation**: Validates screenshot capture functionality
- **Window size testing**: Tests different resolutions and aspect ratios
- **Automated cleanup**: Manages temporary test files

```bash
# Run screenshot tests
cargo test test_hello_world_screenshot_matches_golden
cargo test test_screenshot_generation_basic
cargo test test_different_window_sizes
```

**Note**: Screenshot tests require a graphics environment and may not work in headless CI environments.

#### 3. Snapshot Tests (Ratatui Backend)

Text-based visual regression testing using the `insta` crate:

```bash
# Run ratatui snapshot tests
cargo test -p kryon-ratatui

# Review snapshot changes
cargo insta review
```

**Snapshot Testing Workflow**:

1. **Run the tests**: `cargo test -p kryon-ratatui`
2. **Initial Failure**: New tests fail initially - this is expected
3. **Review Changes**: `cargo insta review` shows visual diffs
4. **Accept/Reject**: Interactive tool to approve or reject changes
5. **Verify**: Run tests again to confirm they pass

### Testing Best Practices

#### For Debug Renderer
- Add tests when adding new properties or output formats
- Verify that all element properties are correctly displayed
- Test edge cases like empty files or invalid formats

#### For Screenshot Tests
- Update golden screenshots when making intentional visual changes
- Use appropriate screenshot delays to ensure UI is settled
- Test different window sizes to catch layout issues

#### For Snapshot Tests  
- Review snapshots carefully before accepting changes
- Keep snapshots minimal and focused
- Use the ratatui backend as the "source of truth" for visual verification

### Test File Organization

```
tests/
├── debug_renderer_test.rs      # Debug renderer functionality tests
├── screenshot_test.rs          # Visual regression tests with screenshots
├── golden_screenshots/         # Reference images for comparison
│   └── hello_world_golden.png
└── temp_screenshots/           # Temporary test screenshots (auto-cleaned)
```

### Continuous Integration

For CI environments:

```bash
# Run tests that don't require graphics
cargo test --test debug_renderer_test
cargo test -p kryon-ratatui

# Skip screenshot tests in headless environments
cargo test --test screenshot_test || echo "Screenshot tests skipped (no display)"
```

## Backend Selection

Choose your rendering backend based on your needs:

- **WGPU**: Best for desktop apps, games, and high-performance UIs
- **Ratatui**: Perfect for CLI tools, server applications, and terminal UIs  
- **Raylib**: Great for simple games, prototypes, and learning graphics programming

## Development Workflow

1. **Make changes** to renderer code
2. **Run debug renderer** to inspect element structure
3. **Run snapshot tests** to ensure text-based output is correct
4. **Run screenshot tests** to verify visual appearance (if applicable)
5. **Review any test failures** and update golden references if changes are intentional
6. **Verify all tests pass** before committing

Focus on making tests pass - particularly the snapshot tests for text-based verification and screenshot tests for visual correctness.
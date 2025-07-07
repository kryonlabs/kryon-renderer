# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with the Kryon Renderer project.

## Project Overview

The Kryon Renderer is a multi-backend rendering engine for KRB (Kryon Binary) files, supporting desktop (WGPU), terminal (Ratatui), and graphics (Raylib) backends. It provides a complete runtime system with layout engine, script integration, and DOM API for building cross-platform applications.

## Essential Commands

### Building
```bash
# Build all backends
cargo build --workspace

# Build specific backends
cargo build --no-default-features --features wgpu
cargo build --no-default-features --features ratatui  
cargo build --no-default-features --features raylib

# Release builds
cargo build --release --workspace
```

### Testing
```bash
# Core testing command (snapshot testing)
cargo test -p kryon-ratatui
cargo insta review              # Review visual differences

# Run all tests
cargo test

# Test specific backends
cargo test -p kryon-wgpu
cargo test -p kryon-raylib
```

### Running Applications
```bash
# Run with different backends
./target/debug/kryon-renderer-wgpu examples/hello_world.krb
./target/debug/kryon-renderer-ratatui examples/hello_world.krb
./target/debug/kryon-renderer-raylib examples/hello_world.krb

# Debug output
RUST_LOG=debug ./target/debug/kryon-renderer-raylib examples/tabbar_left_demo.krb
```

## Architecture Overview

### Workspace Structure
- **`kryon-core`**: Core types, KRB parser, element definitions, property system
- **`kryon-layout`**: Layout engine with flexbox and absolute positioning
- **`kryon-render`**: Rendering abstractions and command system
- **`kryon-runtime`**: App management, event system, script integration, DOM API
- **`kryon-wgpu`**: GPU rendering backend (high-performance desktop)
- **`kryon-ratatui`**: Terminal UI backend (text-based, snapshot testing)
- **`kryon-raylib`**: Simple 2D/3D graphics backend

### Key Features
- ✅ **Multi-Backend Rendering**: WGPU, Ratatui, Raylib support
- ✅ **Layout Engine**: Flex and absolute positioning with proper layout flags
- ✅ **DOM API**: Complete script integration with element manipulation
- ✅ **Event System**: Click, hover, focus events with script callbacks
- ✅ **Property Resolution**: Style application and element property management
- 🟡 **Percentage Support**: Layout engine percentage-to-pixel conversion (pending)
- 🔴 **Animation System**: Planned for future implementation

## Core Testing Principle

This project's quality is maintained through **snapshot testing** using the `kryon-ratatui` backend as the "source of truth":

### Testing Workflow
1. **Run tests**: `cargo test -p kryon-ratatui`
2. **Review diffs**: `cargo insta review` (shows visual differences)
3. **Accept/reject**: Interactive tool to approve changes
4. **Goal**: Make the diff disappear by fixing code

This text-based approach provides deterministic visual verification without image comparison complexity.

## Script System & DOM API

### Core DOM API (✅ IMPLEMENTED)
The script system provides comprehensive element manipulation:

```lua
-- Element access and modification
local element = getElementById("my_element")
element:setText("New text")
element:setVisible(false)
element:setChecked(true)
element:setStyle("new_style_name")

-- DOM traversal
local parent = element:getParent()
local children = element:getChildren()
local nextSibling = element:getNextSibling()

-- Query methods
local buttons = getElementsByTag("Button")
local styledElements = getElementsByClass("my_style")
local element = querySelector("#my_id")
```

### Implementation Details
- **Location**: `crates/kryon-runtime/src/script_system.rs`
- **Initialization**: `register_dom_functions()` called after `setup_bridge_functions()`
- **Languages**: Lua, JavaScript, Python, Wren support
- **Memory Model**: Element data cloned into script context

## Layout Engine

### Layout System (✅ IMPLEMENTED)
- **Flex Layout**: Row/column with alignment and growth
- **Absolute Layout**: Precise positioning with overlap support
- **Layout Flags**: Proper compilation from KRY styles to binary format
- **Constraint System**: Width/height constraints and scaling

### Layout Flag Values
- `0x00`: Row layout (default)
- `0x01`: Column layout
- `0x02`: Absolute layout (critical for overlays)
- `0x04`: Center alignment
- `0x20`: Grow flag

### Common Issues Fixed
- ✅ **Layout flag compilation**: Styles now correctly generate absolute positioning flags
- ✅ **Overlay positioning**: Content panels properly overlap instead of stacking
- ✅ **DOM API availability**: Functions available when scripts execute

## Development Status

### Completed Features
- ✅ **KRB parsing**: Complete binary format support with debug output
- ✅ **Multi-backend rendering**: WGPU, Ratatui, Raylib implementations
- ✅ **Layout engine**: Flex and absolute positioning with scaling
- ✅ **Script integration**: Full DOM API with element manipulation
- ✅ **Event system**: Click handlers and state management
- ✅ **Property system**: Style application and element property resolution

### Pending Features
- 🟡 **Percentage support**: Layout engine percentage-to-pixel conversion
- 🟡 **Resource management**: Image and font loading system
- 🔴 **Animation system**: Transitions and keyframe animations
- 🔴 **Accessibility**: Screen reader and keyboard navigation support

## Development Workflow

1. **Make changes** to renderer code
2. **Run tests**: `cargo test -p kryon-ratatui`
3. **Review snapshots**: `cargo insta review` for visual diffs
4. **Test backends**: Verify changes work across WGPU, Ratatui, Raylib
5. **Debug output**: Use debug builds to trace layout and rendering issues
6. **Update examples**: Ensure example KRB files work correctly

## Troubleshooting

### Common Issues
- **Layout problems**: Check layout flags in debug output (`[STYLE_LAYOUT]`)
- **Script errors**: Verify DOM API initialization order in runtime
- **Rendering issues**: Compare behavior across different backends
- **Property resolution**: Check style application in KRB parser debug output

### Debug Commands
```bash
# Enable debug logging
RUST_LOG=debug ./target/debug/kryon-renderer-raylib file.krb

# Test specific layout scenarios
cargo test -p kryon-ratatui test_layout
cargo insta review

# Check KRB file structure
cargo run --bin debug_krb file.krb
```

Focus on making tests pass, particularly the snapshot tests which provide the definitive measure of rendering correctness.
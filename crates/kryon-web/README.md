# Kryon Web Renderer

WebAssembly-based rendering backend for Kryon applications, allowing them to run in web browsers.

## Features

- **Canvas2D Rendering**: Basic 2D graphics using HTML5 Canvas
- **WebGL Support**: Hardware-accelerated rendering (planned)
- **DOM Rendering**: HTML element-based rendering for better accessibility
- **Event Handling**: Mouse, keyboard, and touch event support
- **Asset Loading**: Web-optimized asset loading system
- **WASM Integration**: Efficient Rust-to-JavaScript interoperability

## Build Requirements

- Rust toolchain with `wasm32-unknown-unknown` target
- `wasm-pack` for building WebAssembly packages
- Web browser with modern JavaScript support

## Quick Start

1. **Install Dependencies**
   ```bash
   # Install Rust target
   rustup target add wasm32-unknown-unknown
   
   # Install wasm-pack
   cargo install wasm-pack
   ```

2. **Build the Web Package**
   ```bash
   ./build.sh
   ```

3. **Run the Example**
   ```bash
   # Start a local HTTP server
   python3 -m http.server 8000
   
   # Open http://localhost:8000 in your browser
   ```

## Usage

### Basic Integration

```javascript
import init, { KryonWebApp } from './pkg/kryon_web.js';

async function main() {
    // Initialize the WASM module
    await init();
    
    // Create the Kryon web app
    const app = new KryonWebApp();
    
    // Initialize canvas rendering
    await app.init_canvas('my-canvas');
    
    // Load and run a KRB file
    const krbData = await fetch('app.krb').then(r => r.arrayBuffer());
    await app.load_krb(new Uint8Array(krbData));
    
    // Start the render loop
    function render(timestamp) {
        app.render(timestamp);
        requestAnimationFrame(render);
    }
    requestAnimationFrame(render);
}

main();
```

### Render Modes

The web renderer supports multiple rendering modes:

- **Canvas 2D**: Software-based 2D rendering
- **WebGL**: Hardware-accelerated 3D rendering (planned)
- **DOM**: HTML element-based rendering

### Event System

The renderer provides a comprehensive event handling system:

```javascript
// Events are automatically handled by the WebEventHandler
// Mouse events: mousedown, mouseup, mousemove, wheel
// Keyboard events: keydown, keyup
// Touch events: touchstart, touchmove, touchend
// Window events: resize
```

## Architecture

### Core Components

- **CanvasRenderer**: HTML5 Canvas 2D rendering backend
- **DomRenderer**: HTML DOM element rendering backend
- **WebEventHandler**: Cross-platform event handling
- **WebAssetLoader**: Efficient asset loading and caching
- **KryonWebApp**: Main application entry point

### File Structure

```
kryon-web/
├── src/
│   ├── lib.rs              # Main WASM entry point
│   ├── canvas_renderer.rs  # Canvas 2D rendering
│   ├── dom_renderer.rs     # DOM element rendering
│   ├── event_handler.rs    # Event handling system
│   ├── asset_loader.rs     # Asset loading utilities
│   └── utils.rs           # Web utilities
├── index.html             # Example HTML host page
├── build.sh              # Build script
└── README.md             # This file
```

## Browser Support

- Chrome 80+
- Firefox 74+
- Safari 13.1+
- Edge 80+

## Performance Considerations

- Canvas rendering is suitable for simple UIs and 2D graphics
- WebGL mode (planned) will provide better performance for complex graphics
- DOM mode offers better accessibility but may be slower for animations
- Asset loading is optimized with caching and compression

## Development

### Building from Source

```bash
# Development build
cargo build --target wasm32-unknown-unknown

# Release build with optimizations
cargo build --release --target wasm32-unknown-unknown

# Build with wasm-pack
wasm-pack build --target web --out-dir pkg
```

### Testing

The web renderer can be tested by opening the provided HTML file in a web browser. The interface includes:

- Render mode selection (Canvas 2D, WebGL, DOM)
- Sample content loading
- Debug information panel
- Performance metrics

## Limitations

- Text rendering is currently limited to basic Canvas text APIs
- Complex layouts may have performance implications in Canvas mode
- WebGL and WebGPU support is planned but not yet implemented
- Font loading requires additional setup for custom fonts

## Contributing

1. Ensure all changes compile for `wasm32-unknown-unknown` target
2. Test in multiple browsers
3. Maintain compatibility with the core Kryon renderer API
4. Add appropriate error handling for web-specific issues

## License

This project is licensed under the same terms as the main Kryon project.
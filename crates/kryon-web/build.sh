#!/bin/bash
set -e

echo "Building Kryon Web Renderer..."

# Check if wasm-pack is installed
if ! command -v wasm-pack &> /dev/null; then
    echo "Installing wasm-pack..."
    curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
fi

# Build the WebAssembly package
echo "Building WebAssembly package..."
wasm-pack build --target web --out-dir pkg

echo "Build complete! Generated files in pkg/ directory:"
ls -la pkg/

echo ""
echo "To test the web renderer:"
echo "1. Start a local HTTP server in this directory"
echo "2. Open index.html in a browser"
echo ""
echo "Example:"
echo "  python3 -m http.server 8000"
echo "  # Then open http://localhost:8000 in your browser"
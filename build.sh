# build.sh
#!/bin/bash

echo "Building Kryon Renderer..."

# Build all crates
cargo build --workspace

# Build examples
echo "Building examples..."
cargo build --manifest-path examples/basic-wgpu/Cargo.toml
cargo build --manifest-path examples/terminal-ui/Cargo.toml

echo "Build complete!"

# Run tests
echo "Running tests..."
cargo test --workspace

echo "All done! You can now run:"
echo "  cargo run --manifest-path examples/basic-wgpu/Cargo.toml"
echo "  cargo run --manifest-path examples/terminal-ui/Cargo.toml"
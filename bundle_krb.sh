#!/bin/bash

# Script to bundle a KRB file into a standalone executable

set -e

# Check arguments
if [ "$#" -lt 1 ]; then
    echo "Usage: $0 <krb_file> [backend] [output_name]"
    echo "  krb_file: Path to the .krb file to bundle"
    echo "  backend: wgpu, ratatui, or raylib (default: wgpu)"
    echo "  output_name: Name of the output executable (default: input filename without extension)"
    echo ""
    echo "Example: $0 examples/01_getting_started/hello_world.krb wgpu hello_world_app"
    exit 1
fi

KRB_FILE="$1"
BACKEND="${2:-wgpu}"
OUTPUT_NAME="${3:-$(basename "$KRB_FILE" .krb)}"

# Verify KRB file exists
if [ ! -f "$KRB_FILE" ]; then
    echo "Error: KRB file not found: $KRB_FILE"
    exit 1
fi

# Get absolute path to KRB file
KRB_FILE_ABS=$(realpath "$KRB_FILE")

echo "Bundling KRB file: $KRB_FILE"
echo "Backend: $BACKEND"
echo "Output: $OUTPUT_NAME"

# Create temporary source with embedded KRB data
echo "Preparing standalone source with embedded KRB..."

# Convert KRB to byte array
echo "Converting KRB to byte array..."
KRB_BYTES=$(xxd -i "$KRB_FILE_ABS" | grep -E '^\s*0x' | tr -d '\n' | sed 's/, *$//')

# Create a temporary source file with embedded data
TEMP_STANDALONE="src/bin/kryon-standalone-temp.rs"
cp src/bin/kryon-standalone.rs "$TEMP_STANDALONE"

# Replace the placeholder with actual KRB data
sed -i "s/const EMBEDDED_KRB_DATA: &\[u8\] = &\[\];/const EMBEDDED_KRB_DATA: \&[u8] = \&[$KRB_BYTES];/" "$TEMP_STANDALONE"

# Add the temporary binary to Cargo.toml
echo "

[[bin]]
name = \"kryon-standalone-temp\"
path = \"src/bin/kryon-standalone-temp.rs\"" >> Cargo.toml

# Build the standalone executable
echo "Building standalone executable..."
cargo build --release \
    --bin kryon-standalone-temp \
    --features "$BACKEND" \
    --no-default-features

# Copy the executable to the desired output name
cp "target/release/kryon-standalone-temp" "$OUTPUT_NAME"
chmod +x "$OUTPUT_NAME"

# Clean up temporary files
rm "$TEMP_STANDALONE"
# Remove the temporary binary from Cargo.toml
sed -i '/\[\[bin\]\]/,/name = "kryon-standalone-temp"/d' Cargo.toml

echo "Successfully created standalone executable: $OUTPUT_NAME"
echo "Size: $(du -h "$OUTPUT_NAME" | cut -f1)"

# Create a companion script for distribution
cat > "${OUTPUT_NAME}_info.txt" << EOF
Kryon Standalone Application
===========================
Original KRB: $KRB_FILE
Backend: $BACKEND
Build Date: $(date)
Platform: $(uname -s) $(uname -m)

This is a self-contained executable that includes:
- The Kryon runtime
- The $BACKEND rendering backend
- The embedded KRB application data

No additional files or dependencies are required to run this application.
EOF

echo "Created info file: ${OUTPUT_NAME}_info.txt"
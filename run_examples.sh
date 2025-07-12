#!/usr/bin/env bash

# Test script for running all KRB examples with kryon-renderer-raylib
# Compiles .kry files to .krb, then runs each .krb file one by one

set -e  # Exit on any error

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
COMPILER_PATH="$SCRIPT_DIR/../kryon-compiler/target/release/kryc"
RENDERER_PATH="$SCRIPT_DIR/target/release/kryon-renderer-raylib"

# Check if user wants to list examples only
if [ "$1" = "list" ]; then
    echo "📋 KRY Examples List"
    echo "================================"
    
    # Find all .kry files and display them in order
    krb_files=($(find examples -name "*.kry" -type f ! -path "*/widgets/*" | sort))
    
    if [ ${#krb_files[@]} -eq 0 ]; then
        echo "❌ No .kry files found!"
        exit 1
    fi
    
    echo "Found ${#krb_files[@]} examples:"
    echo ""
    
    for i in "${!krb_files[@]}"; do
        echo "  [$((i+1))] ${krb_files[i]}"
    done
    
    echo ""
    echo "================================"
    exit 0
fi

echo "🚀 KRY Examples Test Runner"
echo "================================"

# Check if compiler exists
if [ ! -f "$COMPILER_PATH" ]; then
    echo "❌ Compiler not found at: $COMPILER_PATH"
    echo "Building compiler..."
    cd "$SCRIPT_DIR/../kryon-compiler"
    cargo build --release
    cd "$SCRIPT_DIR"
fi

# Always build the latest raylib renderer to ensure we have recent fixes
echo "🔧 Building latest raylib renderer..."
cd "$SCRIPT_DIR"
cargo build --release --no-default-features --features raylib

echo "✅ Compiler: $COMPILER_PATH"
echo "✅ Renderer: $RENDERER_PATH"
echo ""

# Find all .kry files and compile them (excluding widgets folders)
echo "📦 Compiling .kry files to .krb..."
find examples -name "*.kry" -type f ! -path "*/widgets/*" | while read -r kry_file; do
    krb_file="${kry_file%.kry}.krb"
    echo "  Compiling: $kry_file -> $krb_file"
    "$COMPILER_PATH" "$kry_file" "$krb_file"
done

echo ""

# Find all .krb files and prepare list (excluding widgets folders)
krb_files=($(find examples -name "*.krb" -type f ! -path "*/widgets/*" | sort))

if [ ${#krb_files[@]} -eq 0 ]; then
    echo "❌ No .krb files found!"
    exit 1
fi

echo "🎮 Found ${#krb_files[@]} .krb files to test:"
for i in "${!krb_files[@]}"; do
    echo "  [$((i+1))] ${krb_files[i]}"
done
echo ""

# Run each .krb file
for i in "${!krb_files[@]}"; do
    krb_file="${krb_files[i]}"
    echo "🎯 Running [$((i+1))/${#krb_files[@]}]: $krb_file"
    echo "   (Close the window to continue to next example)"
    
    # Run the renderer with the .krb file
    if "$RENDERER_PATH" "$krb_file"; then
        echo "✅ Completed: $krb_file"
    else
        echo "❌ Failed to run: $krb_file (exit code: $?)"
        echo "   Continue anyway? [y/N]"
        read -r response
        if [[ ! "$response" =~ ^[Yy]$ ]]; then
            echo "🛑 Stopping test run."
            exit 1
        fi
    fi
    echo ""
done

echo "🎉 All examples completed!"
echo "================================"
echo "Summary:"
echo "  Total files tested: ${#krb_files[@]}"
echo "  Compiler: $COMPILER_PATH"  
echo "  Renderer: $RENDERER_PATH"
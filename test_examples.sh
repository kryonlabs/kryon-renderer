#!/bin/env bash

# Kryon Renderer Test Runner
# Tests all example files using the debug renderer

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
EXAMPLES_DIR="$SCRIPT_DIR/examples"
OUTPUT_DIR="$SCRIPT_DIR/test_output"
COMPILER="$SCRIPT_DIR/../kryon-compiler/target/release/kryc"
DEBUG_RENDERER="$SCRIPT_DIR/target/release/kryon-renderer-debug"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "=== Kryon Renderer Test Suite ==="
echo

# Create output directory
mkdir -p "$OUTPUT_DIR"

# Check if tools exist
if [ ! -f "$COMPILER" ]; then
    echo -e "${RED}Error: Compiler not found at $COMPILER${NC}"
    echo "Please build the compiler first: cd kryon-compiler && cargo build --release"
    exit 1
fi

if [ ! -f "$DEBUG_RENDERER" ]; then
    echo -e "${RED}Error: Debug renderer not found at $DEBUG_RENDERER${NC}"
    echo "Please build the renderer first: cargo build --release"
    exit 1
fi

# Function to test a KRY file
test_kry_file() {
    local kry_file="$1"
    local rel_path="${kry_file#$EXAMPLES_DIR/}"
    local krb_file="$OUTPUT_DIR/${rel_path%.kry}.krb"
    local test_output="$OUTPUT_DIR/${rel_path%.kry}_debug.txt"
    
    echo -n "Testing $rel_path... "
    
    # Create directory structure in output
    mkdir -p "$(dirname "$krb_file")"
    mkdir -p "$(dirname "$test_output")"
    
    # Compile KRY to KRB
    if "$COMPILER" "$kry_file" -o "$krb_file" > /dev/null 2>&1; then
        echo -n "compiled... "
        
        # Test with debug renderer
        if "$DEBUG_RENDERER" "$krb_file" --format detailed --show-properties --show-layout --show-colors > "$test_output" 2>&1; then
            echo -e "${GREEN}PASS${NC}"
            return 0
        else
            echo -e "${RED}FAIL (debug render)${NC}"
            return 1
        fi
    else
        echo -e "${RED}FAIL (compile)${NC}"
        return 1
    fi
}

# Function to run all tests in a category
test_category() {
    local category="$1"
    local category_dir="$EXAMPLES_DIR/$category"
    
    if [ ! -d "$category_dir" ]; then
        echo -e "${YELLOW}Category $category not found, skipping${NC}"
        return 0
    fi
    
    echo -e "${YELLOW}=== Testing $category ===${NC}"
    
    local passed=0
    local total=0
    
    # Find all .kry files in the category
    while IFS= read -r -d '' kry_file; do
        if test_kry_file "$kry_file"; then
            ((passed++))
        fi
        ((total++))
    done < <(find "$category_dir" -name "*.kry" -print0)
    
    echo "Category $category: $passed/$total tests passed"
    echo
    
    return $(( total - passed ))
}

# Test all categories
total_passed=0
total_tests=0
failed_categories=0

categories=("basic" "text" "buttons" "layout" "colors" "components" "complex" "tests/text" "tests/buttons" "tests/layout" "tests/colors" "tests/components")

for category in "${categories[@]}"; do
    if test_category "$category"; then
        :  # Category passed
    else
        ((failed_categories++))
    fi
done

# Count all tests
while IFS= read -r -d '' kry_file; do
    ((total_tests++))
done < <(find "$EXAMPLES_DIR" -name "*.kry" -print0)

# Count all successful outputs
successful_outputs=$(find "$OUTPUT_DIR" -name "*_debug.txt" | wc -l)

echo "=== Test Summary ==="
echo "Total KRY files found: $total_tests"
echo "Successful debug outputs: $successful_outputs"
echo "Failed categories: $failed_categories"

if [ $failed_categories -eq 0 ]; then
    echo -e "${GREEN}All tests passed!${NC}"
    exit 0
else
    echo -e "${RED}Some tests failed${NC}"
    exit 1
fi

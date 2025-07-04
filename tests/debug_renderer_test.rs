use std::process::Command;
use std::path::Path;

#[test]
fn test_debug_renderer_basic_output() {
    let krb_path = "examples/01_getting_started/hello_world.krb";
    
    // Verify the KRB file exists
    assert!(Path::new(krb_path).exists(), "KRB file not found: {}", krb_path);
    
    // Run the debug renderer
    let output = Command::new("cargo")
        .args(&["run", "--bin", "kryon-renderer-debug", "--", krb_path])
        .output()
        .expect("Failed to execute debug renderer");
    
    // Check that the command succeeded
    assert!(output.status.success(), "Debug renderer failed: {}", String::from_utf8_lossy(&output.stderr));
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Verify basic structure is present
    assert!(stdout.contains("App"), "Output should contain App element");
    assert!(stdout.contains("Container"), "Output should contain Container element");
    assert!(stdout.contains("Text"), "Output should contain Text element");
    assert!(stdout.contains("Hello World"), "Output should contain Hello World text");
    
    // Verify tree structure
    assert!(stdout.contains("└── Container"), "Should show Container as child of App");
    assert!(stdout.contains("└── Text"), "Should show Text as child of Container");
}

#[test]
fn test_debug_renderer_with_properties() {
    let krb_path = "examples/01_getting_started/hello_world.krb";
    
    // Run the debug renderer with properties
    let output = Command::new("cargo")
        .args(&["run", "--bin", "kryon-renderer-debug", "--", krb_path, "--show-properties", "--show-layout", "--show-colors"])
        .output()
        .expect("Failed to execute debug renderer");
    
    assert!(output.status.success(), "Debug renderer failed: {}", String::from_utf8_lossy(&output.stderr));
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Verify properties are shown
    assert!(stdout.contains("pos:("), "Should show position information");
    assert!(stdout.contains("size:("), "Should show size information");
    assert!(stdout.contains("• text:"), "Should show text property");
    assert!(stdout.contains("• style_id:"), "Should show style_id property");
    
    // Verify layout information
    assert!(stdout.contains("pos:(0,0) size:(800,600)"), "App should have correct size");
    assert!(stdout.contains("pos:(200,100) size:(200,100)"), "Container should have correct position and size");
    
    // Verify new properties are shown
    assert!(stdout.contains("• text_alignment: Center"), "Should show text alignment property");
    assert!(stdout.contains("• layout_flags: 0x05"), "Should show layout flags for container");
}

#[test]
fn test_debug_renderer_json_output() {
    let krb_path = "examples/01_getting_started/hello_world.krb";
    
    // Run the debug renderer with JSON format
    let output = Command::new("cargo")
        .args(&["run", "--bin", "kryon-renderer-debug", "--", krb_path, "--format", "json"])
        .output()
        .expect("Failed to execute debug renderer");
    
    assert!(output.status.success(), "Debug renderer failed: {}", String::from_utf8_lossy(&output.stderr));
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Verify JSON structure
    assert!(stdout.contains("\"version\":"), "Should contain version field");
    assert!(stdout.contains("\"element_count\":"), "Should contain element_count field");
    assert!(stdout.contains("\"elements\":"), "Should contain elements array");
    assert!(stdout.contains("\"type\": \"App\""), "Should contain App element type");
    assert!(stdout.contains("\"type\": \"Container\""), "Should contain Container element type");
    assert!(stdout.contains("\"type\": \"Text\""), "Should contain Text element type");
    
    // Verify JSON is valid by checking it starts and ends correctly
    assert!(stdout.trim().starts_with('{'), "JSON should start with '{{'");
    assert!(stdout.trim().ends_with('}'), "JSON should end with '}}'");
}

#[test]
fn test_debug_renderer_detailed_output() {
    let krb_path = "examples/01_getting_started/hello_world.krb";
    
    // Run the debug renderer with detailed format
    let output = Command::new("cargo")
        .args(&["run", "--bin", "kryon-renderer-debug", "--", krb_path, "--format", "detailed"])
        .output()
        .expect("Failed to execute debug renderer");
    
    assert!(output.status.success(), "Debug renderer failed: {}", String::from_utf8_lossy(&output.stderr));
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Verify detailed analysis sections
    assert!(stdout.contains("=== KRYON BINARY FILE ANALYSIS ==="), "Should contain analysis header");
    assert!(stdout.contains("HEADER:"), "Should contain header section");
    assert!(stdout.contains("STRING TABLE:"), "Should contain string table section");
    assert!(stdout.contains("ELEMENT TREE:"), "Should contain element tree section");
    assert!(stdout.contains("=== END ANALYSIS ==="), "Should contain analysis footer");
    
    // Verify string table content
    assert!(stdout.contains("\"appstyle\""), "Should contain appstyle string");
    assert!(stdout.contains("\"containerstyle\""), "Should contain containerstyle string");
    assert!(stdout.contains("\"Hello World Example\""), "Should contain window title string");
    assert!(stdout.contains("\"Hello World\""), "Should contain text content string");
}

#[test]
fn test_debug_renderer_invalid_file() {
    let krb_path = "non_existent_file.krb";
    
    // Run the debug renderer with non-existent file
    let output = Command::new("cargo")
        .args(&["run", "--bin", "kryon-renderer-debug", "--", krb_path])
        .output()
        .expect("Failed to execute debug renderer");
    
    // Should fail
    assert!(!output.status.success(), "Debug renderer should fail for non-existent file");
    
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not found"), "Should indicate file not found");
}

#[test]
fn test_debug_renderer_properties_validation() {
    let krb_path = "examples/01_getting_started/hello_world.krb";
    
    // Run the debug renderer with all options
    let output = Command::new("cargo")
        .args(&["run", "--bin", "kryon-renderer-debug", "--", krb_path, "--show-properties", "--show-layout", "--show-colors"])
        .output()
        .expect("Failed to execute debug renderer");
    
    assert!(output.status.success(), "Debug renderer failed: {}", String::from_utf8_lossy(&output.stderr));
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Verify that all expected properties from the original KRY file are represented
    // Based on hello_world.kry content:
    
    // App properties
    assert!(stdout.contains("App"), "Should contain App element");
    // The App should have window dimensions (800x600)
    assert!(stdout.contains("size:(800,600)"), "App should have correct window size");
    
    // Container properties  
    assert!(stdout.contains("Container"), "Should contain Container element");
    // The Container should have position (200,100) and size (200,100)
    assert!(stdout.contains("pos:(200,100) size:(200,100)"), "Container should have correct position and size");
    
    // Text properties
    assert!(stdout.contains("Text"), "Should contain Text element");
    assert!(stdout.contains("\"Hello World\""), "Should contain text content");
    // Text should have center alignment (though this might not be visible in current output)
    
    // Style properties (these should be inherited from styles)
    // The debug renderer should ideally show background colors, border properties, etc.
    // but currently it only shows font_size
}
use std::process::Command;
use std::path::{Path, PathBuf};
use std::fs;
use std::env;

const HELLO_WORLD_KRB: &str = "examples/01_getting_started/hello_world.krb";
const GOLDEN_SCREENSHOT: &str = "tests/golden_screenshots/hello_world_golden.png";
const TEST_SCREENSHOT: &str = "tests/temp_screenshots/hello_world_test.png";

fn get_absolute_path(relative_path: &str) -> PathBuf {
    let current_dir = env::current_dir().expect("Failed to get current directory");
    current_dir.join(relative_path)
}

#[test]
fn test_hello_world_screenshot_matches_golden() {
    let temp_screenshot = get_absolute_path(TEST_SCREENSHOT);
    let golden_screenshot = get_absolute_path(GOLDEN_SCREENSHOT);
    let screenshot_filename = "hello_world_test.png";
    
    // Create the temp_screenshots directory if it doesn't exist
    if let Some(parent) = temp_screenshot.parent() {
        fs::create_dir_all(parent).expect("Failed to create temp screenshots directory");
    }
    
    // Verify the KRB file exists
    assert!(Path::new(HELLO_WORLD_KRB).exists(), "KRB file not found: {}", HELLO_WORLD_KRB);
    
    // Clean up any existing screenshot from current directory
    let _ = fs::remove_file(screenshot_filename);
    
    // Take a screenshot using the raylib renderer
    let output = Command::new("cargo")
        .args(&[
            "run", "--features", "raylib", "--bin", "kryon-renderer-raylib", "--",
            HELLO_WORLD_KRB,
            "--screenshot", screenshot_filename,
            "--screenshot-delay", "500"
        ])
        .output()
        .expect("Failed to execute raylib renderer for screenshot");
    
    // Check that the command succeeded
    assert!(output.status.success(), 
        "Raylib renderer failed: {}", 
        String::from_utf8_lossy(&output.stderr));
    
    // Verify the screenshot was created in current directory (raylib limitation)
    assert!(Path::new(screenshot_filename).exists(), 
        "Screenshot file was not created: {}", screenshot_filename);
    
    // Move the screenshot to the desired location
    fs::copy(screenshot_filename, &temp_screenshot)
        .expect("Failed to move screenshot to test directory");
    
    // Check if golden screenshot exists
    if !golden_screenshot.exists() {
        // Create the golden_screenshots directory if it doesn't exist
        if let Some(parent) = golden_screenshot.parent() {
            fs::create_dir_all(parent).expect("Failed to create golden screenshots directory");
        }
        
        // Copy the test screenshot as the golden reference
        fs::copy(&temp_screenshot, &golden_screenshot)
            .expect("Failed to create golden screenshot");
        
        println!("Golden screenshot created: {}", golden_screenshot.display());
        println!("Future test runs will compare against this reference.");
        return;
    }
    
    // Compare the screenshots
    let golden_data = fs::read(&golden_screenshot)
        .expect("Failed to read golden screenshot");
    let test_data = fs::read(&temp_screenshot)
        .expect("Failed to read test screenshot");
    
    // For a more sophisticated comparison, we could use image libraries to compare pixel by pixel
    // For now, we'll do a simple byte-by-byte comparison
    if golden_data != test_data {
        // Save the failing screenshot with a different name for debugging
        let fail_screenshot = get_absolute_path("tests/temp_screenshots/hello_world_failed.png");
        fs::copy(&temp_screenshot, &fail_screenshot)
            .expect("Failed to save failing screenshot");
        
        panic!(
            "Screenshot comparison failed!\n\
            Golden: {} ({} bytes)\n\
            Test: {} ({} bytes)\n\
            Failed screenshot saved as: {}\n\
            \n\
            To update the golden screenshot (if the change is intentional):\n\
            cp {} {}",
            golden_screenshot.display(), golden_data.len(),
            temp_screenshot.display(), test_data.len(),
            fail_screenshot.display(),
            temp_screenshot.display(), golden_screenshot.display()
        );
    }
    
    // Clean up the test screenshot if the test passed
    let _ = fs::remove_file(screenshot_filename);
    let _ = fs::remove_file(&temp_screenshot);
    
    println!("Screenshot test passed! Output matches golden reference.");
}

#[test]
fn test_screenshot_generation_basic() {
    let temp_screenshot_rel = "tests/temp_screenshots/basic_test.png";
    let temp_screenshot = get_absolute_path(temp_screenshot_rel);
    let screenshot_filename = "basic_test.png";
    
    // Create the temp_screenshots directory if it doesn't exist
    if let Some(parent) = temp_screenshot.parent() {
        fs::create_dir_all(parent).expect("Failed to create temp screenshots directory");
    }
    
    // Verify the KRB file exists
    assert!(Path::new(HELLO_WORLD_KRB).exists(), "KRB file not found: {}", HELLO_WORLD_KRB);
    
    // Clean up any existing screenshot from current directory
    let _ = fs::remove_file(screenshot_filename);
    
    // Take a screenshot using the raylib renderer
    let output = Command::new("cargo")
        .args(&[
            "run", "--features", "raylib", "--bin", "kryon-renderer-raylib", "--",
            HELLO_WORLD_KRB,
            "--screenshot", screenshot_filename,
            "--screenshot-delay", "200"
        ])
        .output()
        .expect("Failed to execute raylib renderer for screenshot");
    
    // Check that the command succeeded
    assert!(output.status.success(), 
        "Raylib renderer failed: {}", 
        String::from_utf8_lossy(&output.stderr));
    
    // Verify the screenshot was created in current directory (raylib limitation)
    assert!(Path::new(screenshot_filename).exists(), 
        "Screenshot file was not created: {}", screenshot_filename);
    
    // Move the screenshot to the desired location
    fs::copy(screenshot_filename, &temp_screenshot)
        .expect("Failed to move screenshot to test directory");
    
    // Verify the screenshot has a reasonable file size (should be > 1KB for a valid PNG)
    let metadata = fs::metadata(&temp_screenshot)
        .expect("Failed to get screenshot metadata");
    assert!(metadata.len() > 1024, 
        "Screenshot file is too small ({} bytes), may be invalid", metadata.len());
    
    // Clean up both files
    let _ = fs::remove_file(screenshot_filename);
    let _ = fs::remove_file(&temp_screenshot);
    
    println!("Basic screenshot generation test passed!");
}

#[test]
fn test_different_window_sizes() {
    let test_cases = [
        (400, 300, "small"),
        (1024, 768, "medium"), 
        (1920, 1080, "large"),
    ];
    
    for (width, height, name) in &test_cases {
        let temp_screenshot_rel = format!("tests/temp_screenshots/size_test_{}x{}_{}.png", width, height, name);
        let temp_screenshot = get_absolute_path(&temp_screenshot_rel);
        let screenshot_filename = format!("size_test_{}x{}_{}.png", width, height, name);
        
        // Create the temp_screenshots directory if it doesn't exist
        if let Some(parent) = temp_screenshot.parent() {
            fs::create_dir_all(parent).expect("Failed to create temp screenshots directory");
        }
        
        // Clean up any existing screenshot from current directory
        let _ = fs::remove_file(&screenshot_filename);
        
        // Take a screenshot with specific window size
        let output = Command::new("cargo")
            .args(&[
                "run", "--features", "raylib", "--bin", "kryon-renderer-raylib", "--",
                HELLO_WORLD_KRB,
                "--width", &width.to_string(),
                "--height", &height.to_string(),
                "--screenshot", &screenshot_filename,
                "--screenshot-delay", "200"
            ])
            .output()
            .expect("Failed to execute raylib renderer for screenshot");
        
        // Check that the command succeeded
        assert!(output.status.success(), 
            "Raylib renderer failed for size {}x{}: {}", 
            width, height, String::from_utf8_lossy(&output.stderr));
        
        // Verify the screenshot was created in current directory (raylib limitation)
        assert!(Path::new(&screenshot_filename).exists(), 
            "Screenshot file was not created for size {}x{}: {}", width, height, screenshot_filename);
        
        // Move the screenshot to the desired location
        fs::copy(&screenshot_filename, &temp_screenshot)
            .expect("Failed to move screenshot to test directory");
        
        // Clean up both files
        let _ = fs::remove_file(&screenshot_filename);
        let _ = fs::remove_file(&temp_screenshot);
    }
    
    println!("Different window sizes test passed!");
}

#[cfg(test)]
mod setup {
    use super::*;
    
    // Helper function to clean up test files
    pub fn cleanup_test_files() {
        let temp_dir = "tests/temp_screenshots";
        if Path::new(temp_dir).exists() {
            let _ = fs::remove_dir_all(temp_dir);
        }
    }
}

// Run cleanup after tests
#[cfg(test)]
use std::sync::Once;
static CLEANUP: Once = Once::new();

#[test]
fn test_cleanup() {
    CLEANUP.call_once(|| {
        setup::cleanup_test_files();
    });
}
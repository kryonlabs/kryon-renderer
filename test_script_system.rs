// Test script to verify print functionality
use kryon_runtime::ScriptSystem;
use kryon_core::ScriptEntry;

fn main() -> anyhow::Result<()> {
    let mut script_system = ScriptSystem::new();
    
    // Test basic print functionality
    let test_script = ScriptEntry {
        name: "test_print".to_string(),
        language: "lua".to_string(),
        code: r#"
print("Hello from Lua script!")
print("Testing numbers:", 42, 3.14)
print("Testing booleans:", true, false)
print("Testing nil:", nil)
print("Testing multiple arguments:", "arg1", "arg2", "arg3")

function test_function()
    print("Function call works!")
end

test_function()
        "#.to_string(),
        entry_points: vec!["test_function".to_string()],
    };
    
    // Load the script
    script_system.load_scripts(&[test_script])?;
    
    // Call the function
    script_system.call_function("test_function", vec![])?;
    
    println!("Test completed successfully!");
    Ok(())
}
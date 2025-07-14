// Debug script system execution
use kryon_runtime::ScriptSystem;
use std::collections::HashMap;

fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("debug")
        .init();

    println!("=== DEBUG SCRIPT SYSTEM ===");
    
    // Create script system
    let mut script_system = ScriptSystem::new();
    
    // Initialize template variables like the counter app does
    let mut template_vars = HashMap::new();
    template_vars.insert("counter_value".to_string(), "0".to_string());
    
    println!("1. Initializing template variables...");
    script_system.initialize_template_variables(&template_vars)?;
    
    // Load and execute debug script
    println!("2. Loading debug script...");
    let debug_code = std::fs::read_to_string("debug_script.lua")?;
    script_system.call_function(&debug_code, vec![])?;
    
    // Simulate increment function like in counter
    println!("3. Testing increment function...");
    let increment_code = r#"
        print("=== INCREMENT TEST ===")
        print("Before increment - counter_value:", counter_value)
        counter_value = counter_value + 1
        print("After increment - counter_value:", counter_value)
        print("=== INCREMENT TEST END ===")
    "#;
    
    script_system.call_function(increment_code, vec![])?;
    
    // Check for pending changes
    println!("4. Checking pending changes...");
    let changes = script_system.apply_pending_template_variable_changes()?;
    println!("Pending changes: {:?}", changes);
    
    println!("=== DEBUG COMPLETE ===");
    Ok(())
}
// crates/kryon-runtime/src/script_system.rs
use kryon_core::{ScriptEntry, Element, ElementId, PropertyValue};
use std::collections::HashMap;
use std::time::Duration;
use anyhow::Result;

pub struct ScriptSystem {
    scripts: Vec<ScriptEntry>,
    state: HashMap<String, PropertyValue>,
}

impl ScriptSystem {
    pub fn new() -> Self {
        Self {
            scripts: Vec::new(),
            state: HashMap::new(),
        }
    }
    
    pub fn load_scripts(&mut self, scripts: &[ScriptEntry]) -> Result<()> {
        self.scripts = scripts.to_vec();
        
        for script in &self.scripts {
            tracing::info!("Loaded {} script: {}", script.language, script.name);
            
            // For now, just log the script content
            if !script.code.is_empty() && !script.code.starts_with("external:") {
                tracing::debug!("Script content preview: {}", 
                    &script.code[..script.code.len().min(100)]);
            }
        }
        
        Ok(())
    }
    
    pub fn update(&mut self, _delta_time: Duration, _elements: &mut HashMap<ElementId, Element>) -> Result<()> {
        // Update script systems
        // For now, this is a placeholder
        Ok(())
    }
    
    pub fn call_function(&mut self, function_name: &str, _args: Vec<PropertyValue>) -> Result<()> {
        tracing::debug!("Script function called: {}", function_name);
        
        // Find the script that contains this function
        for script in &self.scripts {
            if script.entry_points.contains(&function_name.to_string()) {
                tracing::debug!("Found function {} in script {}", function_name, script.name);
                // Execute the function (placeholder)
                break;
            }
        }
        
        Ok(())
    }
    
    pub fn set_state(&mut self, key: String, value: PropertyValue) {
        self.state.insert(key, value);
    }
    
    pub fn get_state(&self, key: &str) -> Option<&PropertyValue> {
        self.state.get(key)
    }
}
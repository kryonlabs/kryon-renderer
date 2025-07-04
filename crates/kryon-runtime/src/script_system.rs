// crates/kryon-runtime/src/script_system.rs
use kryon_core::{ScriptEntry, Element, ElementId, PropertyValue};
use std::collections::HashMap;
use std::time::Duration;
use anyhow::Result;
use mlua::Lua;

#[derive(Debug)]
pub struct ScriptSystem {
    scripts: Vec<ScriptEntry>,
    state: HashMap<String, PropertyValue>,
    lua: Lua,
}

impl ScriptSystem {
    pub fn new() -> Self {
        Self {
            scripts: Vec::new(),
            state: HashMap::new(),
            lua: Lua::new(),
        }
    }
    
    pub fn load_scripts(&mut self, scripts: &[ScriptEntry]) -> Result<()> {
        self.scripts = scripts.to_vec();
        
        for script in &self.scripts {
            tracing::info!("Loaded {} script: {}", script.language, script.name);
            
            // Load Lua scripts into the Lua context
            if script.language == "lua" && !script.code.is_empty() && !script.code.starts_with("external:") {
                tracing::debug!("Loading Lua script: {}", script.name);
                tracing::debug!("Script content: {}", script.code);
                
                // Execute the script to load the functions
                match self.lua.load(&script.code).exec() {
                    Ok(()) => {
                        tracing::info!("Successfully loaded Lua script: {}", script.name);
                    }
                    Err(e) => {
                        tracing::error!("Failed to load Lua script '{}': {}", script.name, e);
                    }
                }
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
                
                // Execute the Lua function
                if script.language == "lua" {
                    match self.lua.globals().get::<_, mlua::Function>(function_name) {
                        Ok(lua_function) => {
                            match lua_function.call::<_, ()>(()) {
                                Ok(()) => {
                                }
                                Err(e) => {
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!("Lua function '{}' not found: {}", function_name, e);
                        }
                    }
                }
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
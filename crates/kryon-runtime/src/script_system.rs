// crates/kryon-runtime/src/script_system.rs
use kryon_core::{ScriptEntry, Element, ElementId, PropertyValue, KRBFile};
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
    
    pub fn setup_bridge_functions(&mut self, elements: &HashMap<ElementId, Element>, krb_file: &KRBFile) -> Result<()> {
        let globals = self.lua.globals();
        
        // Helper function to find style ID by name
        let find_style_id = |style_name: &str| -> Option<u8> {
            krb_file.styles.iter()
                .find(|(_, style)| style.name == style_name)
                .map(|(style_id, _)| *style_id)
        };
        
        // Create a table to store element IDs by their IDs
        let element_ids_table = self.lua.create_table()?;
        for (element_id, element) in elements {
            // Use the element's actual ID string if available, otherwise use numeric ID
            let id_str = if element.id.is_empty() {
                format!("element_{}", element_id)
            } else {
                element.id.clone()
            };
            element_ids_table.set(id_str, *element_id)?;
        }
        globals.set("_element_ids", element_ids_table)?;
        
        // Create a table to store style name to ID mappings
        let style_ids_table = self.lua.create_table()?;
        for (style_id, style) in &krb_file.styles {
            style_ids_table.set(style.name.clone(), *style_id)?;
        }
        globals.set("_style_ids", style_ids_table)?;
        
        // Add bridge functions
        let lua_code = r#"
            -- Global variables to track pending changes
            _pending_style_changes = {}
            _pending_state_changes = {}
            _pending_text_changes = {}
            
            -- Function to find element by ID
            function getElementById(element_id)
                local numeric_id = _element_ids[element_id]
                if numeric_id then
                    return {
                        id = element_id,
                        numeric_id = numeric_id,
                        setStyle = function(self, style_name)
                            local style_id = _style_ids[style_name]
                            if style_id then
                                _pending_style_changes[self.numeric_id] = style_id
                                print("Queuing style change for element " .. self.id .. " to style " .. style_name .. " (ID: " .. style_id .. ")")
                            else
                                print("Error: Style '" .. style_name .. "' not found")
                            end
                        end,
                        setChecked = function(self, checked)
                            _pending_state_changes[self.numeric_id] = checked
                            print("Queuing checked state change for element " .. self.id .. " to " .. tostring(checked))
                        end,
                        setText = function(self, text)
                            _pending_text_changes[self.numeric_id] = text
                            print("Queuing text change for element " .. self.id .. " to: " .. text)
                        end
                    }
                else
                    print("Error: Element '" .. element_id .. "' not found")
                    return nil
                end
            end
            
            -- Function to get pending style changes
            function _get_pending_style_changes()
                local changes = _pending_style_changes
                _pending_style_changes = {}
                return changes
            end
            
            -- Function to get pending state changes
            function _get_pending_state_changes()
                local changes = _pending_state_changes
                _pending_state_changes = {}
                return changes
            end
            
            -- Function to get pending text changes
            function _get_pending_text_changes()
                local changes = _pending_text_changes
                _pending_text_changes = {}
                return changes
            end
        "#;
        
        self.lua.load(lua_code).exec()?;
        
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
                                    tracing::debug!("Lua function '{}' executed successfully", function_name);
                                }
                                Err(e) => {
                                    tracing::error!("Error executing Lua function '{}': {}", function_name, e);
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
    
    pub fn apply_pending_style_changes(&mut self, elements: &mut HashMap<ElementId, Element>) -> Result<bool> {
        // Get pending style changes from Lua
        let get_changes_fn: mlua::Function = self.lua.globals().get("_get_pending_style_changes")?;
        let changes_table: mlua::Table = get_changes_fn.call(())?;
        
        let mut changes_applied = false;
        
        // Iterate through the changes table
        for pair in changes_table.pairs::<u32, u8>() {
            let (element_numeric_id, new_style_id) = pair?;
            let element_id: ElementId = element_numeric_id;
            
            if let Some(element) = elements.get_mut(&element_id) {
                tracing::info!("Applying style change: element {} -> style_id {}", element_numeric_id, new_style_id);
                element.style_id = new_style_id;
                changes_applied = true;
            } else {
                tracing::warn!("Could not find element {} to apply style change", element_numeric_id);
            }
        }
        
        Ok(changes_applied)
    }
    
    pub fn apply_pending_state_changes(&mut self, elements: &mut HashMap<ElementId, Element>) -> Result<bool> {
        use kryon_core::InteractionState;
        
        // Get pending state changes from Lua
        let get_changes_fn: mlua::Function = self.lua.globals().get("_get_pending_state_changes")?;
        let changes_table: mlua::Table = get_changes_fn.call(())?;
        
        let mut changes_applied = false;
        
        // Iterate through the changes table
        for pair in changes_table.pairs::<u32, bool>() {
            let (element_numeric_id, checked) = pair?;
            let element_id: ElementId = element_numeric_id;
            
            if let Some(element) = elements.get_mut(&element_id) {
                tracing::info!("Applying state change: element {} -> checked {}", element_numeric_id, checked);
                
                // Update the element's current state to include/exclude Checked flag
                if checked {
                    // Add Checked to the current state (bitwise OR)
                    element.current_state = InteractionState::Checked;
                } else {
                    // Remove Checked from the current state - for now just set to Normal
                    element.current_state = InteractionState::Normal;
                }
                changes_applied = true;
            } else {
                tracing::warn!("Could not find element {} to apply state change", element_numeric_id);
            }
        }
        
        Ok(changes_applied)
    }
    
    pub fn apply_pending_text_changes(&mut self, elements: &mut HashMap<ElementId, Element>) -> Result<bool> {
        // Get pending text changes from Lua
        let get_changes_fn: mlua::Function = self.lua.globals().get("_get_pending_text_changes")?;
        let changes_table: mlua::Table = get_changes_fn.call(())?;
        
        let mut changes_applied = false;
        
        // Iterate through the changes table
        for pair in changes_table.pairs::<u32, String>() {
            let (element_numeric_id, new_text) = pair?;
            let element_id: ElementId = element_numeric_id;
            
            if let Some(element) = elements.get_mut(&element_id) {
                tracing::info!("Applying text change: element {} -> text '{}'", element_numeric_id, new_text);
                element.text = new_text;
                changes_applied = true;
            } else {
                tracing::warn!("Could not find element {} to apply text change", element_numeric_id);
            }
        }
        
        Ok(changes_applied)
    }
    
    pub fn set_state(&mut self, key: String, value: PropertyValue) {
        self.state.insert(key, value);
    }
    
    pub fn get_state(&self, key: &str) -> Option<&PropertyValue> {
        self.state.get(key)
    }
}
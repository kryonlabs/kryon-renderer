//! Lua reactive variable system
//!
//! This module provides reactive template variable support for Lua scripts,
//! enabling automatic UI updates when script variables change. It implements
//! the reactive system using Lua metamethods for transparent variable access.

use std::collections::HashMap;
use std::rc::Rc;
use anyhow::Result;
use mlua::{Lua, Table as LuaTable, Function as LuaFunction};
use crate::script::{
    engine_trait::{ChangeSet, ScriptValue},
    error::ScriptError,
};

/// Lua reactive variable system
/// 
/// This component provides:
/// - Transparent reactive variable access through metamethods
/// - Automatic change tracking for template variables
/// - Integration with the UI template system
/// - Memory-efficient variable management
pub struct LuaReactiveSystem {
    /// Reference to the Lua VM
    lua: Rc<Lua>,
}

impl LuaReactiveSystem {
    /// Create a new reactive system
    pub fn new(lua: Rc<Lua>) -> Result<Self> {
        Ok(Self {
            lua,
        })
    }
    
    /// Setup reactive variables from template variables
    pub fn setup(&mut self, variables: &HashMap<String, String>) -> Result<()> {
        let globals = self.lua.globals();
        
        // Create the template variables table
        let template_vars_table = self.lua.create_table()?;
        for (name, value) in variables {
            template_vars_table.set(name.clone(), value.clone())?;
        }
        globals.set("_template_variables", template_vars_table)?;
        
        // Setup the reactive system code
        let reactive_setup_code = r#"
            -- Initialize reactive template variables
            _template_variable_changes = {}
            
            -- Create reactive variable proxy
            local function create_reactive_variable(name, initial_value)
                local value = initial_value
                return {
                    get = function() return value end,
                    set = function(new_value)
                        if value ~= new_value then
                            value = tostring(new_value)
                            _template_variable_changes[name] = value
                            _template_variables[name] = value
                            -- Immediately notify the template engine for instant UI updates
                            if _immediate_template_update then
                                _immediate_template_update(name, value)
                            end
                        end
                    end,
                    __tostring = function() return value end,
                    __eq = function(other) return value == tostring(other) end,
                    __lt = function(other) return tonumber(value) and tonumber(other) and tonumber(value) < tonumber(other) end,
                    __le = function(other) return tonumber(value) and tonumber(other) and tonumber(value) <= tonumber(other) end,
                    __add = function(other) return (tonumber(value) or 0) + (tonumber(other) or 0) end,
                    __sub = function(other) return (tonumber(value) or 0) - (tonumber(other) or 0) end,
                    __mul = function(other) return (tonumber(value) or 0) * (tonumber(other) or 0) end,
                    __div = function(other) return (tonumber(value) or 0) / (tonumber(other) or 0) end,
                    __concat = function(other) return value .. tostring(other) end
                }
            end
            
            -- Store all reactive variables in a central registry
            _reactive_variables = {}
            
            -- Create reactive variables for each template variable
            for name, value in pairs(_template_variables) do
                local reactive_var = create_reactive_variable(name, value)
                _reactive_variables[name] = reactive_var
            end
            
            -- Set up global metamethod to intercept all global variable access
            local original_g_mt = getmetatable(_G) or {}
            
            setmetatable(_G, {
                __index = function(t, k)
                    -- Check if this is a reactive variable
                    if _reactive_variables[k] then
                        return _reactive_variables[k].get()
                    end
                    -- Use original index behavior
                    if original_g_mt.__index then
                        return original_g_mt.__index(t, k)
                    else
                        return rawget(t, k)
                    end
                end,
                __newindex = function(t, k, v)
                    -- Check if this is a reactive variable
                    if _reactive_variables[k] then
                        -- This is a reactive variable, use the reactive setter
                        _reactive_variables[k].set(v)
                        -- DO NOT use rawset for reactive variables - let all access go through metamethods
                    else
                        -- Not a reactive variable, use normal assignment
                        if original_g_mt.__newindex then
                            original_g_mt.__newindex(t, k, v)
                        else
                            rawset(t, k, v)
                        end
                    end
                end
            })
            
            -- Function to get pending template variable changes (without clearing)
            function _get_reactive_template_variable_changes()
                return _template_variable_changes
            end
            
            -- Function to clear pending template variable changes
            function _clear_template_variable_changes()
                _template_variable_changes = {}
            end
            
            -- Placeholder for immediate update callback
            _immediate_template_update = function(name, value)
                -- Changes are already queued in _template_variable_changes
                -- The Rust side will process them immediately after script execution
            end
        "#;
        
        self.lua.load(reactive_setup_code).exec().map_err(|e| {
            ScriptError::ReactiveVariableSetupFailed {
                error: e.to_string(),
                variable_name: "system".to_string(),
                variable_value: "initialization".to_string(),
            }
        })?;
        
        tracing::debug!("Lua reactive variable system initialized with {} variables", variables.len());
        Ok(())
    }
    
    /// Get pending reactive variable changes (without clearing them)
    pub fn get_pending_changes(&mut self) -> Result<HashMap<String, ChangeSet>> {
        let mut changes = HashMap::new();
        
        // Get reactive variable changes
        if let Ok(get_changes_fn) = self.lua.globals().get::<_, LuaFunction>("_get_reactive_template_variable_changes") {
            if let Ok(changes_table) = get_changes_fn.call::<_, LuaTable>(()) {
                let mut template_changes = HashMap::new();
                for pair in changes_table.pairs::<String, String>() {
                    if let Ok((name, value)) = pair {
                        template_changes.insert(name, value);
                    }
                }
                if !template_changes.is_empty() {
                    changes.insert("template_variables".to_string(), ChangeSet {
                        change_type: "template_variables".to_string(),
                        data: template_changes,
                    });
                }
            }
        }
        
        Ok(changes)
    }
    
    /// Clear pending reactive variable changes
    pub fn clear_pending_changes(&mut self) -> Result<()> {
        if let Ok(clear_changes_fn) = self.lua.globals().get::<_, LuaFunction>("_clear_template_variable_changes") {
            clear_changes_fn.call::<_, ()>(()).map_err(|e| ScriptError::ReactiveVariableSetupFailed {
                error: e.to_string(),
                variable_name: "clear_changes".to_string(),
                variable_value: "".to_string(),
            })?;
        }
        Ok(())
    }
    
    /// Set a reactive variable value
    pub fn set_variable(&mut self, name: &str, value: ScriptValue) -> Result<()> {
        let globals = self.lua.globals();
        
        // Update the template variables table
        if let Ok(template_vars) = globals.get::<_, LuaTable>("_template_variables") {
            let value_str = value.to_string();
            template_vars.set(name, value_str.clone())?;
            
            // Update the reactive variable if it exists
            if let Ok(reactive_vars) = globals.get::<_, LuaTable>("_reactive_variables") {
                if let Ok(reactive_var) = reactive_vars.get::<_, LuaTable>(name) {
                    if let Ok(set_fn) = reactive_var.get::<_, LuaFunction>("set") {
                        set_fn.call::<_, ()>(value_str).map_err(|e| {
                            ScriptError::ReactiveVariableSetupFailed {
                                error: e.to_string(),
                                variable_name: name.to_string(),
                                variable_value: value.to_string(),
                            }
                        })?;
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Get a reactive variable value
    pub fn get_variable(&self, name: &str) -> Option<ScriptValue> {
        let globals = self.lua.globals();
        
        // Try to get from reactive variables first
        if let Ok(reactive_vars) = globals.get::<_, LuaTable>("_reactive_variables") {
            if let Ok(reactive_var) = reactive_vars.get::<_, LuaTable>(name) {
                if let Ok(get_fn) = reactive_var.get::<_, LuaFunction>("get") {
                    if let Ok(value) = get_fn.call::<_, String>(()) {
                        return Some(ScriptValue::String(value));
                    }
                }
            }
        }
        
        // Fallback to template variables table
        if let Ok(template_vars) = globals.get::<_, LuaTable>("_template_variables") {
            if let Ok(value) = template_vars.get::<_, String>(name) {
                return Some(ScriptValue::String(value));
            }
        }
        
        None
    }
    
    /// Get all reactive variable names
    pub fn get_variable_names(&self) -> Vec<String> {
        let globals = self.lua.globals();
        let mut names = Vec::new();
        
        if let Ok(reactive_vars) = globals.get::<_, LuaTable>("_reactive_variables") {
            for pair in reactive_vars.pairs::<String, LuaTable>() {
                if let Ok((name, _)) = pair {
                    names.push(name);
                }
            }
        }
        
        names
    }
    
    /// Add a new reactive variable
    pub fn add_variable(&mut self, name: &str, initial_value: &str) -> Result<()> {
        let add_variable_code = format!(r#"
            -- Add new reactive variable
            if not _reactive_variables then
                _reactive_variables = {{}}
            end
            
            if not _template_variables then
                _template_variables = {{}}
            end
            
            -- Create the reactive variable
            local function create_reactive_variable(name, initial_value)
                local value = initial_value
                return {{
                    get = function() return value end,
                    set = function(new_value)
                        if value ~= new_value then
                            value = tostring(new_value)
                            _template_variable_changes[name] = value
                            _template_variables[name] = value
                            if _immediate_template_update then
                                _immediate_template_update(name, value)
                            end
                        end
                    end
                }}
            end
            
            _reactive_variables["{}"] = create_reactive_variable("{}", "{}")
            _template_variables["{}"] = "{}"
        "#, name, name, initial_value, name, initial_value);
        
        self.lua.load(&add_variable_code).exec().map_err(|e| {
            ScriptError::ReactiveVariableSetupFailed {
                error: e.to_string(),
                variable_name: name.to_string(),
                variable_value: initial_value.to_string(),
            }
        })?;
        
        tracing::debug!("Added reactive variable '{}' with value '{}'", name, initial_value);
        Ok(())
    }
    
    /// Remove a reactive variable
    pub fn remove_variable(&mut self, name: &str) -> Result<()> {
        let remove_variable_code = format!(r#"
            if _reactive_variables then
                _reactive_variables["{}"] = nil
            end
            if _template_variables then
                _template_variables["{}"] = nil
            end
        "#, name, name);
        
        self.lua.load(&remove_variable_code).exec().map_err(|e| {
            ScriptError::ReactiveVariableSetupFailed {
                error: e.to_string(),
                variable_name: name.to_string(),
                variable_value: "removal".to_string(),
            }
        })?;
        
        tracing::debug!("Removed reactive variable '{}'", name);
        Ok(())
    }
    
    /// Reset all reactive variables
    pub fn reset(&mut self) -> Result<()> {
        let reset_code = r#"
            _template_variable_changes = {}
            _reactive_variables = {}
            _template_variables = {}
        "#;
        
        self.lua.load(reset_code).exec().map_err(|e| {
            ScriptError::ReactiveVariableSetupFailed {
                error: e.to_string(),
                variable_name: "system".to_string(),
                variable_value: "reset".to_string(),
            }
        })?;
        
        tracing::debug!("Reset all reactive variables");
        Ok(())
    }
    
    /// Get statistics about the reactive system
    pub fn get_statistics(&self) -> ReactiveSystemStats {
        let variable_names = self.get_variable_names();
        let total_variables = variable_names.len();
        
        // Try to get the number of pending changes
        let mut pending_changes = 0;
        let globals = self.lua.globals();
        if let Ok(changes_table) = globals.get::<_, LuaTable>("_template_variable_changes") {
            for _ in changes_table.pairs::<String, String>() {
                pending_changes += 1;
            }
        }
        
        ReactiveSystemStats {
            total_variables,
            variable_names,
            pending_changes,
        }
    }
}

/// Statistics about the reactive variable system
#[derive(Debug, Clone)]
pub struct ReactiveSystemStats {
    /// Total number of reactive variables
    pub total_variables: usize,
    /// Names of all reactive variables
    pub variable_names: Vec<String>,
    /// Number of pending changes
    pub pending_changes: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_reactive_system_creation() {
        let lua = Lua::new();
        let reactive = LuaReactiveSystem::new(&lua);
        assert!(reactive.is_ok());
    }
    
    #[test]
    fn test_reactive_variable_setup() {
        let lua = Lua::new();
        let mut reactive = LuaReactiveSystem::new(&lua).unwrap();
        
        let mut variables = HashMap::new();
        variables.insert("counter".to_string(), "0".to_string());
        variables.insert("message".to_string(), "Hello".to_string());
        
        let result = reactive.setup(&variables);
        assert!(result.is_ok());
        
        // Check that variables were set up
        let variable_names = reactive.get_variable_names();
        assert!(variable_names.contains(&"counter".to_string()));
        assert!(variable_names.contains(&"message".to_string()));
    }
    
    #[test]
    fn test_reactive_variable_access() {
        let lua = Lua::new();
        let mut reactive = LuaReactiveSystem::new(&lua).unwrap();
        
        let mut variables = HashMap::new();
        variables.insert("test_var".to_string(), "initial_value".to_string());
        reactive.setup(&variables).unwrap();
        
        // Test getting the variable
        let value = reactive.get_variable("test_var");
        assert_eq!(value, Some(ScriptValue::String("initial_value".to_string())));
        
        // Test setting the variable
        reactive.set_variable("test_var", ScriptValue::String("new_value".to_string())).unwrap();
        let updated_value = reactive.get_variable("test_var");
        assert_eq!(updated_value, Some(ScriptValue::String("new_value".to_string())));
    }
    
    #[test]
    fn test_pending_changes() {
        let lua = Lua::new();
        let mut reactive = LuaReactiveSystem::new(&lua).unwrap();
        
        let mut variables = HashMap::new();
        variables.insert("test_var".to_string(), "initial".to_string());
        reactive.setup(&variables).unwrap();
        
        // Make a change through Lua code to trigger the reactive system
        let change_code = "test_var = 'changed_value'";
        lua.load(change_code).exec().unwrap();
        
        // Get pending changes
        let changes = reactive.get_pending_changes().unwrap();
        
        // Check if changes were captured
        assert!(changes.contains_key("template_variables"));
        let template_changes = &changes["template_variables"];
        assert!(template_changes.data.contains_key("test_var"));
        assert_eq!(template_changes.data["test_var"], "changed_value");
    }
    
    #[test]
    fn test_add_remove_variables() {
        let lua = Lua::new();
        let mut reactive = LuaReactiveSystem::new(&lua).unwrap();
        
        // Start with empty system
        reactive.setup(&HashMap::new()).unwrap();
        
        // Add a variable
        reactive.add_variable("new_var", "test_value").unwrap();
        
        let variable_names = reactive.get_variable_names();
        assert!(variable_names.contains(&"new_var".to_string()));
        
        // Remove the variable
        reactive.remove_variable("new_var").unwrap();
        
        let updated_names = reactive.get_variable_names();
        assert!(!updated_names.contains(&"new_var".to_string()));
    }
    
    #[test]
    fn test_statistics() {
        let lua = Lua::new();
        let mut reactive = LuaReactiveSystem::new(&lua).unwrap();
        
        let mut variables = HashMap::new();
        variables.insert("var1".to_string(), "value1".to_string());
        variables.insert("var2".to_string(), "value2".to_string());
        reactive.setup(&variables).unwrap();
        
        let stats = reactive.get_statistics();
        assert_eq!(stats.total_variables, 2);
        assert!(stats.variable_names.contains(&"var1".to_string()));
        assert!(stats.variable_names.contains(&"var2".to_string()));
    }
}

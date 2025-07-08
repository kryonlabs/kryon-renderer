// crates/kryon-runtime/src/lua_vm.rs

#[cfg(feature = "lua-vm")]
use std::collections::HashMap;
#[cfg(feature = "lua-vm")]
use anyhow::Result;
#[cfg(feature = "lua-vm")]
use mlua::{Lua, Value as LuaValue, Function as LuaFunction, Table as LuaTable};
#[cfg(feature = "lua-vm")]
use crate::vm_trait::{ScriptVM, ScriptValue, VMMemoryStats, VMResourceLimits, VMError};

/// Lua VM implementation using mlua with lightweight configuration for microcontrollers
#[cfg(feature = "lua-vm")]
pub struct LuaVM {
    lua: Lua,
    functions: HashMap<String, String>, // function_name -> script_name mapping
    resource_limits: VMResourceLimits,
    memory_stats: VMMemoryStats,
}

#[cfg(feature = "lua-vm")]
impl LuaVM {
    /// Create new Lua VM with microcontroller-optimized settings
    pub fn new() -> Result<Self> {
        // Use LuaJIT for better performance on microcontrollers
        let lua = Lua::new();
        
        // Setup custom print function
        let _ = lua.globals().set("print", lua.create_function(|_, args: mlua::Variadic<LuaValue>| {
            let mut output = Vec::new();
            for arg in args {
                match arg {
                    LuaValue::String(s) => output.push(s.to_str().unwrap_or("").to_string()),
                    LuaValue::Number(n) => output.push(n.to_string()),
                    LuaValue::Integer(i) => output.push(i.to_string()),
                    LuaValue::Boolean(b) => output.push(b.to_string()),
                    LuaValue::Nil => output.push("nil".to_string()),
                    _ => output.push(format!("{:?}", arg)),
                }
            }
            println!("{}", output.join("\t"));
            Ok(())
        })?)?;

        let default_limits = VMResourceLimits::default();
        
        Ok(Self {
            lua,
            functions: HashMap::new(),
            resource_limits: default_limits,
            memory_stats: VMMemoryStats {
                current_usage: 0,
                peak_usage: 0,
                object_count: 0,
                memory_limit: Some(1024 * 1024), // 1MB default
            },
        })
    }

    /// Setup bridge functions for element manipulation
    pub fn setup_bridge_functions(&mut self, element_ids: &HashMap<String, u32>, style_ids: &HashMap<String, u8>) -> Result<()> {
        let globals = self.lua.globals();
        
        // Create element IDs table
        let element_ids_table = self.lua.create_table()?;
        for (element_id, numeric_id) in element_ids {
            element_ids_table.set(element_id.clone(), *numeric_id)?;
        }
        globals.set("_element_ids", element_ids_table)?;
        
        // Create style IDs table
        let style_ids_table = self.lua.create_table()?;
        for (style_name, style_id) in style_ids {
            style_ids_table.set(style_name.clone(), *style_id)?;
        }
        globals.set("_style_ids", style_ids_table)?;
        
        // Add bridge functions
        let bridge_code = r#"
            -- Global variables to track pending changes
            _pending_style_changes = {}
            _pending_state_changes = {}
            _pending_text_changes = {}
            _pending_visibility_changes = {}
            
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
                            end
                        end,
                        setChecked = function(self, checked)
                            _pending_state_changes[self.numeric_id] = checked
                        end,
                        setText = function(self, text)
                            _pending_text_changes[self.numeric_id] = text
                        end,
                        setVisible = function(self, visible)
                            _pending_visibility_changes[self.numeric_id] = visible
                        end
                    }
                else
                    return nil
                end
            end
            
            -- Alias for backward compatibility
            get_element = getElementById
            
            -- Functions to get pending changes
            function _get_pending_style_changes()
                local changes = _pending_style_changes
                _pending_style_changes = {}
                return changes
            end
            
            function _get_pending_state_changes()
                local changes = _pending_state_changes
                _pending_state_changes = {}
                return changes
            end
            
            function _get_pending_text_changes()
                local changes = _pending_text_changes
                _pending_text_changes = {}
                return changes
            end
            
            function _get_pending_visibility_changes()
                local changes = _pending_visibility_changes
                _pending_visibility_changes = {}
                return changes
            end
        "#;
        
        self.lua.load(bridge_code).exec()?;
        Ok(())
    }

    /// Setup reactive template variables
    pub fn setup_reactive_variables(&mut self, variables: &HashMap<String, String>) -> Result<()> {
        let globals = self.lua.globals();
        
        // Create template variables table
        let template_vars_table = self.lua.create_table()?;
        for (name, value) in variables {
            template_vars_table.set(name.clone(), value.clone())?;
        }
        globals.set("_template_variables", template_vars_table)?;
        
        // Setup reactive system
        let reactive_code = r#"
            _template_variable_changes = {}
            _reactive_variables = {}
            
            local function create_reactive_variable(name, initial_value)
                local value = initial_value
                return {
                    get = function() return value end,
                    set = function(new_value)
                        if value ~= new_value then
                            value = tostring(new_value)
                            _template_variable_changes[name] = value
                            _template_variables[name] = value
                        end
                    end
                }
            end
            
            -- Create reactive variables
            for name, value in pairs(_template_variables) do
                _reactive_variables[name] = create_reactive_variable(name, value)
            end
            
            -- Set up global metamethod
            local original_g_mt = getmetatable(_G) or {}
            setmetatable(_G, {
                __index = function(t, k)
                    if _reactive_variables[k] then
                        return _reactive_variables[k].get()
                    end
                    if original_g_mt.__index then
                        return original_g_mt.__index(t, k)
                    else
                        return rawget(t, k)
                    end
                end,
                __newindex = function(t, k, v)
                    if _reactive_variables[k] then
                        _reactive_variables[k].set(v)
                    else
                        if original_g_mt.__newindex then
                            original_g_mt.__newindex(t, k, v)
                        else
                            rawset(t, k, v)
                        end
                    end
                end
            })
            
            function _get_reactive_template_variable_changes()
                local changes = _template_variable_changes
                _template_variable_changes = {}
                return changes
            end
        "#;
        
        self.lua.load(reactive_code).exec()?;
        Ok(())
    }

    /// Convert Lua value to ScriptValue
    fn lua_value_to_script_value(&self, value: LuaValue) -> ScriptValue {
        match value {
            LuaValue::Nil => ScriptValue::Nil,
            LuaValue::Boolean(b) => ScriptValue::Boolean(b),
            LuaValue::Integer(i) => ScriptValue::Integer(i),
            LuaValue::Number(n) => ScriptValue::Number(n),
            LuaValue::String(s) => ScriptValue::String(s.to_str().unwrap_or("").to_string()),
            LuaValue::Table(table) => {
                // Try to determine if it's an array or object
                let mut map = HashMap::new();
                let mut is_array = true;
                let mut max_index = 0;
                
                for pair in table.clone().pairs::<LuaValue, LuaValue>() {
                    if let Ok((key, _val)) = pair {
                        match key {
                            LuaValue::Integer(i) if i > 0 => {
                                max_index = max_index.max(i as usize);
                            }
                            _ => {
                                is_array = false;
                                break;
                            }
                        }
                    }
                }
                
                if is_array && max_index > 0 {
                    // Convert to array
                    let mut array = Vec::with_capacity(max_index);
                    for i in 1..=max_index {
                        if let Ok(val) = table.get::<_, LuaValue>(i as i64) {
                            array.push(self.lua_value_to_script_value(val));
                        } else {
                            array.push(ScriptValue::Nil);
                        }
                    }
                    ScriptValue::Array(array)
                } else {
                    // Convert to object
                    for pair in table.pairs::<LuaValue, LuaValue>() {
                        if let Ok((key, val)) = pair {
                            let key_str = match key {
                                LuaValue::String(s) => s.to_str().unwrap_or("").to_string(),
                                LuaValue::Integer(i) => i.to_string(),
                                LuaValue::Number(n) => n.to_string(),
                                _ => continue,
                            };
                            map.insert(key_str, self.lua_value_to_script_value(val));
                        }
                    }
                    ScriptValue::Object(map)
                }
            }
            _ => ScriptValue::String(format!("{:?}", value)),
        }
    }

    /// Convert ScriptValue to Lua value
    fn script_value_to_lua_value(&self, value: ScriptValue) -> Result<LuaValue> {
        match value {
            ScriptValue::Nil => Ok(LuaValue::Nil),
            ScriptValue::Boolean(b) => Ok(LuaValue::Boolean(b)),
            ScriptValue::Integer(i) => Ok(LuaValue::Integer(i)),
            ScriptValue::Number(f) => Ok(LuaValue::Number(f)),
            ScriptValue::String(s) => Ok(LuaValue::String(self.lua.create_string(&s)?)),
            ScriptValue::Array(arr) => {
                let table = self.lua.create_table()?;
                for (i, item) in arr.into_iter().enumerate() {
                    table.set(i + 1, self.script_value_to_lua_value(item)?)?;
                }
                Ok(LuaValue::Table(table))
            }
            ScriptValue::Object(obj) => {
                let table = self.lua.create_table()?;
                for (key, value) in obj {
                    table.set(key, self.script_value_to_lua_value(value)?)?;
                }
                Ok(LuaValue::Table(table))
            }
        }
    }

    /// Update memory statistics
    fn update_memory_stats(&mut self) {
        // LuaJIT doesn't provide direct memory usage, so we estimate
        self.memory_stats.object_count = self.functions.len();
        // This is a rough estimate - in a real implementation you'd use lua_gc
        self.memory_stats.current_usage = self.memory_stats.object_count * 1024; // Rough estimate
        self.memory_stats.peak_usage = self.memory_stats.peak_usage.max(self.memory_stats.current_usage);
    }
}

#[cfg(feature = "lua-vm")]
impl ScriptVM for LuaVM {
    fn initialize() -> Result<Self> where Self: Sized {
        Self::new()
    }

    fn load_script(&mut self, name: &str, code: &str) -> Result<()> {
        // Check resource limits
        if let Some(max_vars) = self.resource_limits.max_global_variables {
            if self.functions.len() >= max_vars {
                return Err(VMError::ResourceLimitExceeded(
                    format!("Maximum global variables ({}) exceeded", max_vars)
                ).into());
            }
        }

        if let Some(max_string_len) = self.resource_limits.max_string_length {
            if code.len() > max_string_len {
                return Err(VMError::ResourceLimitExceeded(
                    format!("Script too long: {} > {}", code.len(), max_string_len)
                ).into());
            }
        }

        // Execute the script to load functions
        self.lua.load(code).exec().map_err(|e| {
            VMError::ExecutionFailed(format!("Failed to load script '{}': {}", name, e))
        })?;

        // Try to detect function names from the code (simple regex approach)
        let function_regex = regex::Regex::new(r"function\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*\(").unwrap();
        for captures in function_regex.captures_iter(code) {
            if let Some(func_name) = captures.get(1) {
                self.functions.insert(func_name.as_str().to_string(), name.to_string());
            }
        }

        self.update_memory_stats();
        Ok(())
    }

    fn call_function(&mut self, name: &str, args: Vec<ScriptValue>) -> Result<ScriptValue> {
        let function: LuaFunction = self.lua.globals().get(name).map_err(|_| {
            VMError::FunctionNotFound(name.to_string())
        })?;

        // Convert args to Lua values
        let lua_args: Result<Vec<LuaValue>> = args.into_iter()
            .map(|arg| self.script_value_to_lua_value(arg))
            .collect();

        let lua_args = lua_args?;

        // Call the function
        let result: LuaValue = match lua_args.len() {
            0 => function.call(())?,
            1 => function.call(lua_args[0].clone())?,
            _ => {
                // For multiple args, create a tuple
                let tuple = self.lua.create_table()?;
                for (i, arg) in lua_args.iter().enumerate() {
                    tuple.set(i + 1, arg.clone())?;
                }
                function.call(tuple)?
            }
        };

        Ok(self.lua_value_to_script_value(result))
    }

    fn set_global_variable(&mut self, name: &str, value: ScriptValue) -> Result<()> {
        let lua_value = self.script_value_to_lua_value(value)?;
        self.lua.globals().set(name, lua_value)?;
        self.update_memory_stats();
        Ok(())
    }

    fn get_global_variable(&self, name: &str) -> Option<ScriptValue> {
        if let Ok(value) = self.lua.globals().get::<_, LuaValue>(name) {
            Some(self.lua_value_to_script_value(value))
        } else {
            None
        }
    }

    fn get_pending_changes(&mut self) -> Result<HashMap<String, HashMap<String, String>>> {
        let mut changes = HashMap::new();

        // Get template variable changes
        if let Ok(get_changes_fn) = self.lua.globals().get::<_, LuaFunction>("_get_reactive_template_variable_changes") {
            if let Ok(changes_table) = get_changes_fn.call::<_, LuaTable>(()) {
                let mut template_changes = HashMap::new();
                for pair in changes_table.pairs::<String, String>() {
                    if let Ok((name, value)) = pair {
                        template_changes.insert(name, value);
                    }
                }
                if !template_changes.is_empty() {
                    changes.insert("template_variables".to_string(), template_changes);
                }
            }
        }

        // Get style changes
        if let Ok(get_changes_fn) = self.lua.globals().get::<_, LuaFunction>("_get_pending_style_changes") {
            if let Ok(changes_table) = get_changes_fn.call::<_, LuaTable>(()) {
                let mut style_changes = HashMap::new();
                for pair in changes_table.pairs::<u32, u8>() {
                    if let Ok((element_id, style_id)) = pair {
                        style_changes.insert(element_id.to_string(), style_id.to_string());
                    }
                }
                if !style_changes.is_empty() {
                    changes.insert("style_changes".to_string(), style_changes);
                }
            }
        }

        // Get text changes
        if let Ok(get_changes_fn) = self.lua.globals().get::<_, LuaFunction>("_get_pending_text_changes") {
            if let Ok(changes_table) = get_changes_fn.call::<_, LuaTable>(()) {
                let mut text_changes = HashMap::new();
                for pair in changes_table.pairs::<u32, String>() {
                    if let Ok((element_id, text)) = pair {
                        text_changes.insert(element_id.to_string(), text);
                    }
                }
                if !text_changes.is_empty() {
                    changes.insert("text_changes".to_string(), text_changes);
                }
            }
        }

        // Get state changes
        if let Ok(get_changes_fn) = self.lua.globals().get::<_, LuaFunction>("_get_pending_state_changes") {
            if let Ok(changes_table) = get_changes_fn.call::<_, LuaTable>(()) {
                let mut state_changes = HashMap::new();
                for pair in changes_table.pairs::<u32, bool>() {
                    if let Ok((element_id, checked)) = pair {
                        state_changes.insert(element_id.to_string(), checked.to_string());
                    }
                }
                if !state_changes.is_empty() {
                    changes.insert("state_changes".to_string(), state_changes);
                }
            }
        }

        // Get visibility changes
        if let Ok(get_changes_fn) = self.lua.globals().get::<_, LuaFunction>("_get_pending_visibility_changes") {
            if let Ok(changes_table) = get_changes_fn.call::<_, LuaTable>(()) {
                let mut visibility_changes = HashMap::new();
                for pair in changes_table.pairs::<u32, bool>() {
                    if let Ok((element_id, visible)) = pair {
                        visibility_changes.insert(element_id.to_string(), visible.to_string());
                    }
                }
                if !visibility_changes.is_empty() {
                    changes.insert("visibility_changes".to_string(), visibility_changes);
                }
            }
        }

        Ok(changes)
    }

    fn execute_code(&mut self, code: &str) -> Result<ScriptValue> {
        let result: LuaValue = self.lua.load(code).eval().map_err(|e| {
            VMError::ExecutionFailed(e.to_string())
        })?;
        Ok(self.lua_value_to_script_value(result))
    }

    fn has_function(&self, name: &str) -> bool {
        self.lua.globals().get::<_, LuaFunction>(name).is_ok()
    }

    fn get_function_names(&self) -> Vec<String> {
        self.functions.keys().cloned().collect()
    }

    fn reset(&mut self) -> Result<()> {
        // Create a new Lua context to reset everything
        let new_lua = Lua::new();
        self.lua = new_lua;
        self.functions.clear();
        self.memory_stats = VMMemoryStats {
            current_usage: 0,
            peak_usage: 0,
            object_count: 0,
            memory_limit: self.memory_stats.memory_limit,
        };
        Ok(())
    }

    fn get_memory_usage(&self) -> VMMemoryStats {
        self.memory_stats.clone()
    }

    fn language_name(&self) -> &'static str {
        "lua"
    }

    fn set_limits(&mut self, limits: VMResourceLimits) -> Result<()> {
        self.memory_stats.memory_limit = limits.max_memory;
        self.resource_limits = limits;
        Ok(())
    }
}
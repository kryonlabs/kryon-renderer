//! Lua script engine implementation
//!
//! This module provides a comprehensive Lua script engine with:
//! - Bytecode execution support
//! - DOM API bridge integration
//! - Reactive template variables
//! - Memory management and resource limits
//! - Professional error handling

use std::collections::HashMap;
use std::rc::Rc;
use anyhow::Result;
use mlua::{Lua, Value as LuaValue, Function as LuaFunction};
use regex;

use crate::script::{
    engine_trait::{
        ScriptEngine, ScriptEngineFactory, ScriptValue, BridgeData, ChangeSet, 
        EngineMemoryStats, EngineCapabilities
    },
    error::ScriptError,
};

pub mod bytecode;
pub mod bridge;
pub mod reactive;
pub mod native_renderer;

use bytecode::LuaBytecodeExecutor;
use bridge::LuaBridge;
use reactive::LuaReactiveSystem;
use native_renderer::NativeRendererContext;

/// Lua script engine implementation
/// 
/// This engine provides:
/// - Source code and bytecode execution
/// - Full DOM API integration
/// - Reactive template variables
/// - Memory and resource management
pub struct LuaEngine {
    /// The Lua virtual machine
    lua: Rc<Lua>,
    /// Bridge for DOM API
    bridge: LuaBridge,
    /// Reactive variable system
    reactive: LuaReactiveSystem,
    /// Bytecode executor
    bytecode_executor: LuaBytecodeExecutor,
    /// Function registry
    functions: HashMap<String, String>, // function_name -> script_name
    /// Memory statistics
    memory_stats: EngineMemoryStats,
    /// Native renderer contexts
    native_contexts: HashMap<String, NativeRendererContext>,
}

impl LuaEngine {
    /// Create a new Lua engine
    pub fn new() -> Result<Self> {
        let lua = Rc::new(Lua::new());
        
        // Initialize subsystems
        let bridge = LuaBridge::new(lua.clone())?;
        let reactive = LuaReactiveSystem::new(lua.clone())?;
        let bytecode_executor = LuaBytecodeExecutor::new(lua.clone())?;
        
        Ok(Self {
            lua,
            bridge,
            reactive,
            bytecode_executor,
            functions: HashMap::new(),
            memory_stats: EngineMemoryStats {
                current_usage: 0,
                peak_usage: 0,
                object_count: 0,
                memory_limit: Some(1024 * 1024), // 1MB default
            },
            native_contexts: HashMap::new(),
        })
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
                
                // Check if it's an array (consecutive integer keys starting from 1)
                for pair in table.clone().pairs::<LuaValue, LuaValue>() {
                    if let Ok((key, _)) = pair {
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
        // Estimate memory usage (Lua doesn't provide direct access)
        self.memory_stats.object_count = self.functions.len();
        self.memory_stats.current_usage = self.memory_stats.object_count * 1024; // Rough estimate
        self.memory_stats.peak_usage = self.memory_stats.peak_usage.max(self.memory_stats.current_usage);
    }
    
    /// Create a native renderer context for a specific backend
    pub fn create_native_context(&mut self, context_id: String, backend: String, element_id: kryon_core::ElementId, position: glam::Vec2, size: glam::Vec2) -> Result<()> {
        let context = NativeRendererContext::new(
            self.lua.clone(),
            backend,
            element_id,
            position,
            size
        )?;
        
        self.native_contexts.insert(context_id, context);
        Ok(())
    }
    
    /// Execute a native render script
    pub fn execute_native_render_script(&mut self, context_id: &str, script_name: &str) -> Result<()> {
        if let Some(context) = self.native_contexts.get(context_id) {
            context.execute_render_script(script_name)?;
        }
        Ok(())
    }
    
    /// Remove a native renderer context
    pub fn remove_native_context(&mut self, context_id: &str) {
        self.native_contexts.remove(context_id);
    }
}

impl ScriptEngine for LuaEngine {
    fn language_name(&self) -> &'static str {
        "lua"
    }
    
    fn load_script(&mut self, name: &str, code: &str) -> Result<()> {
        // Execute the script to load functions
        self.lua.load(code).exec().map_err(|e| {
            ScriptError::ExecutionFailed {
                function: "load_script".to_string(),
                error: e.to_string(),
                context: format!("Loading script '{}'", name),
            }
        })?;
        
        // Extract function names from the code
        let function_regex = regex::Regex::new(r"function\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*\(").unwrap();
        for captures in function_regex.captures_iter(code) {
            if let Some(func_name) = captures.get(1) {
                self.functions.insert(func_name.as_str().to_string(), name.to_string());
            }
        }
        
        self.update_memory_stats();
        tracing::debug!("Loaded Lua script '{}' with {} functions", name, self.functions.len());
        
        Ok(())
    }
    
    fn load_bytecode(&mut self, name: &str, bytecode: &[u8]) -> Result<()> {
        // Load and execute the bytecode
        self.bytecode_executor.load_and_execute(name, bytecode)?;
        
        // Update function registry - we need to scan the Lua globals for new functions
        let globals = self.lua.globals();
        let function_names = globals.pairs::<String, LuaValue>()
            .filter_map(|pair| {
                if let Ok((key, value)) = pair {
                    if matches!(value, LuaValue::Function(_)) {
                        Some(key)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect::<Vec<String>>();
        
        for func_name in function_names {
            self.functions.insert(func_name, name.to_string());
        }
        
        self.update_memory_stats();
        tracing::debug!("Loaded Lua bytecode '{}' with {} functions", name, self.functions.len());
        
        Ok(())
    }
    
    fn execute_bytecode(&mut self, bytecode: &[u8]) -> Result<()> {
        self.bytecode_executor.execute(bytecode)
    }
    
    fn call_function(&mut self, name: &str, args: Vec<ScriptValue>) -> Result<ScriptValue> {
        let function: LuaFunction = self.lua.globals().get(name).map_err(|_| {
            ScriptError::FunctionNotFound {
                function: name.to_string(),
                available: self.get_function_names().join(", "),
            }
        })?;
        
        // Convert args to Lua values
        let lua_args: Result<Vec<LuaValue>> = args.into_iter()
            .map(|arg| self.script_value_to_lua_value(arg))
            .collect();
        
        let lua_args = lua_args?;
        
        // Call the function
        let result: LuaValue = function.call(lua_args).map_err(|e| {
            ScriptError::ExecutionFailed {
                function: name.to_string(),
                error: e.to_string(),
                context: "Function call execution".to_string(),
            }
        })?;
        
        Ok(self.lua_value_to_script_value(result))
    }
    
    fn has_function(&self, name: &str) -> bool {
        self.lua.globals().get::<_, LuaFunction>(name).is_ok()
    }
    
    fn get_function_names(&self) -> Vec<String> {
        self.functions.keys().cloned().collect()
    }
    
    fn setup_bridge(&mut self, bridge_data: &BridgeData) -> Result<()> {
        self.bridge.setup(bridge_data)
    }
    
    fn setup_reactive_variables(&mut self, variables: &HashMap<String, String>) -> Result<()> {
        self.reactive.setup(variables)
    }
    
    fn execute_on_ready_callbacks(&mut self) -> Result<()> {
        self.bridge.execute_on_ready_callbacks()
    }
    
    fn get_pending_changes(&mut self) -> Result<HashMap<String, ChangeSet>> {
        let mut changes = HashMap::new();
        
        // Get bridge changes (DOM API)
        let bridge_changes = self.bridge.get_pending_changes()?;
        changes.extend(bridge_changes);
        
        // Get reactive variable changes
        let reactive_changes = self.reactive.get_pending_changes()?;
        changes.extend(reactive_changes);
        
        Ok(changes)
    }
    
    fn clear_pending_changes(&mut self) -> Result<()> {
        // Clear bridge changes
        self.bridge.clear_pending_changes()?;
        
        // Clear reactive variable changes
        self.reactive.clear_pending_changes()?;
        
        Ok(())
    }
    
    fn set_global_variable(&mut self, name: &str, value: ScriptValue) -> Result<()> {
        let lua_value = self.script_value_to_lua_value(value)?;
        self.lua.globals().set(name, lua_value).map_err(|e| {
            ScriptError::ExecutionFailed {
                function: "set_global_variable".to_string(),
                error: e.to_string(),
                context: format!("Setting variable '{}'", name),
            }.into()
        })
    }
    
    fn get_global_variable(&self, name: &str) -> Option<ScriptValue> {
        if let Ok(value) = self.lua.globals().get::<_, LuaValue>(name) {
            Some(self.lua_value_to_script_value(value))
        } else {
            None
        }
    }
    
    fn execute_code(&mut self, code: &str) -> Result<ScriptValue> {
        let result: LuaValue = self.lua.load(code).eval().map_err(|e| {
            ScriptError::ExecutionFailed {
                function: "execute_code".to_string(),
                error: e.to_string(),
                context: "Code execution".to_string(),
            }
        })?;
        
        Ok(self.lua_value_to_script_value(result))
    }
    
    fn reset(&mut self) -> Result<()> {
        // Create a new Lua context
        self.lua = Rc::new(Lua::new());
        self.functions.clear();
        self.native_contexts.clear();
        
        // Reinitialize subsystems
        self.bridge = LuaBridge::new(self.lua.clone())?;
        self.reactive = LuaReactiveSystem::new(self.lua.clone())?;
        self.bytecode_executor = LuaBytecodeExecutor::new(self.lua.clone())?;
        
        // Reset memory stats
        self.memory_stats = EngineMemoryStats {
            current_usage: 0,
            peak_usage: 0,
            object_count: 0,
            memory_limit: self.memory_stats.memory_limit,
        };
        
        Ok(())
    }
    
    fn get_memory_usage(&self) -> EngineMemoryStats {
        self.memory_stats.clone()
    }
}

/// Factory for creating Lua engines
pub struct LuaEngineFactory;

impl LuaEngineFactory {
    pub fn new() -> Self {
        Self
    }
}

impl ScriptEngineFactory for LuaEngineFactory {
    fn create_engine(&self) -> Result<Box<dyn ScriptEngine>> {
        let engine = LuaEngine::new()?;
        Ok(Box::new(engine))
    }
    
    fn language_name(&self) -> &'static str {
        "lua"
    }
    
    fn is_available(&self) -> bool {
        true // Lua is always available
    }
    
    fn capabilities(&self) -> EngineCapabilities {
        EngineCapabilities {
            supports_bytecode: true,
            supports_reactive: true,
            supports_dom_api: true,
            supports_events: true,
            embedded_optimized: true,
            supports_jit: true, // LuaJIT when available
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_lua_engine_creation() {
        let engine = LuaEngine::new();
        assert!(engine.is_ok());
    }
    
    #[test]
    fn test_lua_script_loading() {
        let mut engine = LuaEngine::new().unwrap();
        
        let script = r#"
            function hello(name)
                return "Hello, " .. name
            end
        "#;
        
        assert!(engine.load_script("test", script).is_ok());
        assert!(engine.has_function("hello"));
    }
    
    #[test]
    fn test_lua_function_call() {
        let mut engine = LuaEngine::new().unwrap();
        
        let script = r#"
            function add(a, b)
                return a + b
            end
        "#;
        
        engine.load_script("test", script).unwrap();
        
        let args = vec![
            ScriptValue::Integer(5),
            ScriptValue::Integer(3),
        ];
        
        let result = engine.call_function("add", args).unwrap();
        assert_eq!(result, ScriptValue::Integer(8));
    }
    
    #[test]
    fn test_lua_factory() {
        let factory = LuaEngineFactory::new();
        assert_eq!(factory.language_name(), "lua");
        assert!(factory.is_available());
        
        let engine = factory.create_engine();
        assert!(engine.is_ok());
    }
}

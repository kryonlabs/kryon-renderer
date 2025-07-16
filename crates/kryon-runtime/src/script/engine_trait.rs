//! Script engine trait and supporting types
//!
//! This module defines the core interfaces that all script engines must implement.
//! It provides a unified API for bytecode execution, DOM manipulation, and
//! reactive variable management across different scripting languages.

use std::collections::HashMap;
use anyhow::Result;
use kryon_core::{Element, ElementId};
// use crate::script::error::ScriptError;

/// Core value type for inter-language communication
#[derive(Debug, Clone, PartialEq)]
pub enum ScriptValue {
    Nil,
    Boolean(bool),
    Integer(i64),
    Number(f64),
    String(String),
    Array(Vec<ScriptValue>),
    Object(HashMap<String, ScriptValue>),
}

impl ScriptValue {
    /// Convert to string representation
    pub fn to_string(&self) -> String {
        match self {
            ScriptValue::Nil => "nil".to_string(),
            ScriptValue::Boolean(b) => b.to_string(),
            ScriptValue::Integer(i) => i.to_string(),
            ScriptValue::Number(f) => f.to_string(),
            ScriptValue::String(s) => s.clone(),
            ScriptValue::Array(arr) => {
                let items: Vec<String> = arr.iter().map(|v| v.to_string()).collect();
                format!("[{}]", items.join(", "))
            }
            ScriptValue::Object(obj) => {
                let items: Vec<String> = obj.iter()
                    .map(|(k, v)| format!("{}: {}", k, v.to_string()))
                    .collect();
                format!("{{{}}}", items.join(", "))
            }
        }
    }
    
    /// Convert to boolean (JavaScript-like truthiness)
    pub fn to_bool(&self) -> bool {
        match self {
            ScriptValue::Nil => false,
            ScriptValue::Boolean(b) => *b,
            ScriptValue::Integer(i) => *i != 0,
            ScriptValue::Number(f) => *f != 0.0 && !f.is_nan(),
            ScriptValue::String(s) => !s.is_empty(),
            ScriptValue::Array(arr) => !arr.is_empty(),
            ScriptValue::Object(obj) => !obj.is_empty(),
        }
    }
}

/// Bridge data for DOM API setup
#[derive(Debug, Clone)]
pub struct BridgeData {
    /// Mapping of element string IDs to numeric IDs
    pub element_ids: HashMap<String, ElementId>,
    /// Mapping of style names to style IDs
    pub style_ids: HashMap<String, u8>,
    /// Component properties by element ID
    pub component_properties: HashMap<String, HashMap<String, ScriptValue>>,
    /// Current element data
    pub elements_data: HashMap<ElementId, Element>,
    /// Template variables
    pub template_variables: HashMap<String, String>,
}

/// Represents a set of changes from script execution
#[derive(Debug, Clone)]
pub struct ChangeSet {
    /// Type of change (e.g., "style_changes", "text_changes")
    pub change_type: String,
    /// Change data as key-value pairs
    pub data: HashMap<String, String>,
}

/// Core trait that all script engines must implement
/// 
/// This trait provides a high-level interface for:
/// - Executing compiled bytecode
/// - DOM API integration
/// - Reactive variable management
/// - Event handling
pub trait ScriptEngine {
    /// Get the language name for this engine
    fn language_name(&self) -> &'static str;
    
    /// Load a script (source or bytecode) into the engine
    fn load_script(&mut self, name: &str, code: &str) -> Result<()>;
    
    /// Load compiled bytecode into the engine
    fn load_bytecode(&mut self, name: &str, bytecode: &[u8]) -> Result<()>;
    
    /// Execute compiled bytecode directly
    /// This is the primary method for the bytecode migration
    fn execute_bytecode(&mut self, bytecode: &[u8]) -> Result<()>;
    
    /// Call a function with arguments
    fn call_function(&mut self, name: &str, args: Vec<ScriptValue>) -> Result<ScriptValue>;
    
    /// Check if a function exists in this engine
    fn has_function(&self, name: &str) -> bool;
    
    /// Get list of all available functions
    fn get_function_names(&self) -> Vec<String>;
    
    /// Setup the DOM API bridge
    fn setup_bridge(&mut self, bridge_data: &BridgeData) -> Result<()>;
    
    /// Setup reactive template variables
    fn setup_reactive_variables(&mut self, variables: &HashMap<String, String>) -> Result<()>;
    
    /// Execute onReady callbacks
    fn execute_on_ready_callbacks(&mut self) -> Result<()>;
    
    /// Get all pending changes from this engine
    fn get_pending_changes(&mut self) -> Result<HashMap<String, ChangeSet>>;
    
    /// Clear all pending changes from this engine
    fn clear_pending_changes(&mut self) -> Result<()>;
    
    /// Set a global variable
    fn set_global_variable(&mut self, name: &str, value: ScriptValue) -> Result<()>;
    
    /// Get a global variable
    fn get_global_variable(&self, name: &str) -> Option<ScriptValue>;
    
    /// Execute arbitrary code (for debugging/testing)
    fn execute_code(&mut self, code: &str) -> Result<ScriptValue>;
    
    /// Reset the engine state
    fn reset(&mut self) -> Result<()>;
    
    /// Get memory usage statistics
    fn get_memory_usage(&self) -> EngineMemoryStats;
}

/// Memory usage statistics for an engine
#[derive(Debug, Clone)]
pub struct EngineMemoryStats {
    /// Current memory usage in bytes
    pub current_usage: usize,
    /// Peak memory usage
    pub peak_usage: usize,
    /// Number of objects/variables
    pub object_count: usize,
    /// Memory limit (if set)
    pub memory_limit: Option<usize>,
}

/// Factory trait for creating script engines
pub trait ScriptEngineFactory: Send + Sync {
    /// Create a new engine instance
    fn create_engine(&self) -> Result<Box<dyn ScriptEngine>>;
    
    /// Get the language name this factory creates engines for
    fn language_name(&self) -> &'static str;
    
    /// Check if this engine type is available
    fn is_available(&self) -> bool;
    
    /// Get engine capabilities
    fn capabilities(&self) -> EngineCapabilities;
}

/// Engine capabilities for feature detection
#[derive(Debug, Clone)]
pub struct EngineCapabilities {
    /// Supports bytecode execution
    pub supports_bytecode: bool,
    /// Supports reactive variables
    pub supports_reactive: bool,
    /// Supports DOM API
    pub supports_dom_api: bool,
    /// Supports event handling
    pub supports_events: bool,
    /// Memory optimized for embedded systems
    pub embedded_optimized: bool,
    /// Supports just-in-time compilation
    pub supports_jit: bool,
}

// Convenience conversion traits
impl From<bool> for ScriptValue {
    fn from(value: bool) -> Self {
        ScriptValue::Boolean(value)
    }
}

impl From<i32> for ScriptValue {
    fn from(value: i32) -> Self {
        ScriptValue::Integer(value as i64)
    }
}

impl From<i64> for ScriptValue {
    fn from(value: i64) -> Self {
        ScriptValue::Integer(value)
    }
}

impl From<f32> for ScriptValue {
    fn from(value: f32) -> Self {
        ScriptValue::Number(value as f64)
    }
}

impl From<f64> for ScriptValue {
    fn from(value: f64) -> Self {
        ScriptValue::Number(value)
    }
}

impl From<String> for ScriptValue {
    fn from(value: String) -> Self {
        ScriptValue::String(value)
    }
}

impl From<&str> for ScriptValue {
    fn from(value: &str) -> Self {
        ScriptValue::String(value.to_string())
    }
}

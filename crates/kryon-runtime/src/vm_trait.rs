// crates/kryon-runtime/src/vm_trait.rs

use std::collections::HashMap;
use anyhow::Result;
use serde::{Serialize, Deserialize};

/// Shared value type that can be passed between different VMs
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

    /// Convert to boolean (following JavaScript-like truthiness)
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

    /// Convert to number if possible
    pub fn to_number(&self) -> Option<f64> {
        match self {
            ScriptValue::Integer(i) => Some(*i as f64),
            ScriptValue::Number(f) => Some(*f),
            ScriptValue::String(s) => s.parse().ok(),
            ScriptValue::Boolean(true) => Some(1.0),
            ScriptValue::Boolean(false) => Some(0.0),
            _ => None,
        }
    }
}

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

/// Core trait that all script VMs must implement
/// Designed to support lightweight VMs suitable for microcontrollers
/// Note: Send requirement removed for single-threaded embedded environments
pub trait ScriptVM {
    /// Initialize the VM with minimal resource usage
    /// For microcontrollers, this should allocate minimal memory
    fn initialize() -> Result<Self> where Self: Sized;

    /// Load a script into the VM context
    /// Script name is used for debugging and error reporting
    fn load_script(&mut self, name: &str, code: &str) -> Result<()>;

    /// Call a function with arguments and return the result
    /// For lightweight VMs, consider limiting argument complexity
    fn call_function(&mut self, name: &str, args: Vec<ScriptValue>) -> Result<ScriptValue>;

    /// Set a global variable that's accessible across all scripts in this VM
    fn set_global_variable(&mut self, name: &str, value: ScriptValue) -> Result<()>;

    /// Get a global variable value
    fn get_global_variable(&self, name: &str) -> Option<ScriptValue>;

    /// Get all pending changes (for template variables, element properties, etc.)
    /// Returns a map of change type to change data
    fn get_pending_changes(&mut self) -> Result<HashMap<String, HashMap<String, String>>>;

    /// Execute a snippet of code without defining a function
    /// Useful for initialization and one-off operations
    fn execute_code(&mut self, code: &str) -> Result<ScriptValue>;

    /// Check if the VM has a specific function defined
    fn has_function(&self, name: &str) -> bool;

    /// Get list of all defined functions (for debugging/introspection)
    fn get_function_names(&self) -> Vec<String>;

    /// Clear all global variables and functions (reset VM state)
    /// Important for memory management in microcontrollers
    fn reset(&mut self) -> Result<()>;

    /// Get VM memory usage statistics (important for embedded systems)
    fn get_memory_usage(&self) -> VMMemoryStats;

    /// Get the VM's language identifier (e.g., "lua", "javascript", "python")
    fn language_name(&self) -> &'static str;

    /// Set resource limits for the VM (crucial for microcontrollers)
    fn set_limits(&mut self, limits: VMResourceLimits) -> Result<()>;
}

/// Memory usage statistics for VM monitoring
#[derive(Debug, Clone)]
pub struct VMMemoryStats {
    /// Current memory usage in bytes
    pub current_usage: usize,
    /// Peak memory usage since last reset
    pub peak_usage: usize,
    /// Number of allocated objects/variables
    pub object_count: usize,
    /// Memory limit (if set)
    pub memory_limit: Option<usize>,
}

/// Resource limits for VMs (essential for embedded/microcontroller use)
#[derive(Debug, Clone)]
pub struct VMResourceLimits {
    /// Maximum memory usage in bytes (None = unlimited)
    pub max_memory: Option<usize>,
    /// Maximum execution time per function call in milliseconds
    pub max_execution_time: Option<u64>,
    /// Maximum number of global variables
    pub max_global_variables: Option<usize>,
    /// Maximum string length for variables
    pub max_string_length: Option<usize>,
    /// Maximum recursion depth
    pub max_recursion_depth: Option<usize>,
}

impl Default for VMResourceLimits {
    fn default() -> Self {
        Self {
            max_memory: Some(1024 * 1024), // 1MB default for microcontrollers
            max_execution_time: Some(1000), // 1 second per function call
            max_global_variables: Some(100),
            max_string_length: Some(1024),
            max_recursion_depth: Some(32),
        }
    }
}

/// Trait for creating VM instances (factory pattern)
/// Allows for dynamic VM loading and configuration
pub trait VMFactory {
    /// Create a new VM instance with default settings
    fn create_vm(&self) -> Result<Box<dyn ScriptVM>>;

    /// Create a VM with specific resource limits
    fn create_vm_with_limits(&self, limits: VMResourceLimits) -> Result<Box<dyn ScriptVM>>;

    /// Get the language name this factory creates VMs for
    fn language_name(&self) -> &'static str;

    /// Check if this VM type is available (dependencies installed, etc.)
    fn is_available(&self) -> bool;

    /// Get default resource limits for this VM type
    fn default_limits(&self) -> VMResourceLimits;

    /// Get VM capabilities/features supported
    fn capabilities(&self) -> VMCapabilities;
}

/// VM capabilities for feature detection
#[derive(Debug, Clone)]
pub struct VMCapabilities {
    /// Supports arithmetic operations on variables
    pub supports_arithmetic: bool,
    /// Supports string concatenation
    pub supports_string_ops: bool,
    /// Supports object/table types
    pub supports_objects: bool,
    /// Supports array/list types
    pub supports_arrays: bool,
    /// Supports metamethods/operator overloading
    pub supports_metamethods: bool,
    /// Memory usage is optimized for embedded systems
    pub embedded_optimized: bool,
    /// Supports just-in-time compilation
    pub supports_jit: bool,
}

/// Error types specific to VM operations
#[derive(Debug, thiserror::Error)]
pub enum VMError {
    #[error("VM not initialized")]
    NotInitialized,
    
    #[error("Function '{0}' not found")]
    FunctionNotFound(String),
    
    #[error("Script execution failed: {0}")]
    ExecutionFailed(String),
    
    #[error("Memory limit exceeded: {current} bytes (limit: {limit} bytes)")]
    MemoryLimitExceeded { current: usize, limit: usize },
    
    #[error("Execution timeout after {0}ms")]
    ExecutionTimeout(u64),
    
    #[error("Resource limit exceeded: {0}")]
    ResourceLimitExceeded(String),
    
    #[error("Type conversion failed: cannot convert {from} to {to}")]
    TypeConversionFailed { from: String, to: String },
    
    #[error("VM language '{0}' not supported")]
    UnsupportedLanguage(String),
}

pub type VMResult<T> = std::result::Result<T, VMError>;
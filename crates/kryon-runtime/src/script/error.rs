//! Comprehensive error handling for the script system
//!
//! This module provides detailed error types for all aspects of script execution,
//! including compilation errors, runtime errors, bridge errors, and system errors.
//! All errors include helpful context and suggestions for resolution.

use std::collections::HashMap;
use thiserror::Error;

/// Comprehensive script system error types
#[derive(Error, Debug)]
pub enum ScriptError {
    /// Script engine is not initialized
    #[error("Script engine not initialized. Call ScriptSystem::initialize() first.")]
    NotInitialized,
    
    /// Function not found in any loaded script
    #[error(
        "Function '{function}' not found in any loaded script.\n\n\
        Available functions: {available}\n\n\
        Tip: Make sure the script containing this function is loaded and the function name is correct."
    )]
    FunctionNotFound {
        function: String,
        available: String,
    },
    
    /// Script execution failed
    #[error(
        "Script execution failed in function '{function}': {error}\n\n\
        Context: {context}\n\n\
        Tip: Check the script logic and ensure all variables are properly initialized."
    )]
    ExecutionFailed {
        function: String,
        error: String,
        context: String,
    },
    
    /// Script language not supported
    #[error(
        "Script language '{language}' is not supported.\n\n\
        Supported languages: {supported}\n\n\
        To add support for '{language}', enable the corresponding feature flag:\n\
        - lua: Enable 'lua-vm' feature\n\
        - javascript: Enable 'javascript-vm' feature\n\
        - python: Enable 'python-vm' feature\n\
        - wren: Enable 'wren-vm' feature"
    )]
    UnsupportedLanguage {
        language: String,
        supported: String,
    },
    
    /// Bytecode execution failed
    #[error(
        "Bytecode execution failed for script '{script_name}': {error}\n\n\
        This usually indicates corrupted bytecode or incompatible compiler version.\n\
        Try recompiling the script with the current compiler version."
    )]
    BytecodeExecutionFailed {
        script_name: String,
        error: String,
    },
    
    /// Bridge setup failed
    #[error(
        "DOM API bridge setup failed: {error}\n\n\
        Context: {context}\n\n\
        This error occurs when the script engine cannot access UI elements.\n\
        Ensure that elements are properly loaded before setting up scripts."
    )]
    BridgeSetupFailed {
        error: String,
        context: String,
    },
    
    /// Reactive variable setup failed
    #[error(
        "Reactive variable setup failed: {error}\n\n\
        Variable: {variable_name}\n\
        Value: {variable_value}\n\n\
        Tip: Check that the variable name is valid and the value is properly formatted."
    )]
    ReactiveVariableSetupFailed {
        error: String,
        variable_name: String,
        variable_value: String,
    },
    
    /// Type conversion error
    #[error(
        "Type conversion failed: cannot convert {from_type} to {to_type}\n\n\
        Value: {value}\n\n\
        Tip: Ensure the value is in the correct format for the target type."
    )]
    TypeConversionFailed {
        from_type: String,
        to_type: String,
        value: String,
    },
    
    /// Memory limit exceeded
    #[error(
        "Memory limit exceeded: {current_usage} bytes (limit: {limit} bytes)\n\n\
        Engine: {engine_name}\n\n\
        Tip: Optimize your scripts to use less memory or increase the memory limit."
    )]
    MemoryLimitExceeded {
        current_usage: usize,
        limit: usize,
        engine_name: String,
    },
    
    /// Script compilation error (for future bytecode compilation)
    #[error(
        "Script compilation failed for {file_path}:\n\n\
        {compilation_error}\n\n\
        Line {line}: {line_content}\n\
        {marker}\n\n\
        Tip: Fix the syntax error and recompile."
    )]
    CompilationFailed {
        file_path: String,
        compilation_error: String,
        line: usize,
        line_content: String,
        marker: String,
    },
    
    /// Engine registry error
    #[error(
        "Engine registry error: {error}\n\n\
        Available engines: {available_engines}\n\n\
        Tip: Check that the required script engine is properly configured."
    )]
    RegistryError {
        error: String,
        available_engines: String,
    },
    
    /// Configuration error
    #[error(
        "Script system configuration error: {error}\n\n\
        Configuration: {config_details}\n\n\
        Tip: Check your script system configuration settings."
    )]
    ConfigurationError {
        error: String,
        config_details: String,
    },
    
    /// DOM API error
    #[error(
        "DOM API error in function '{function}': {error}\n\n\
        Element: {element_id}\n\
        Operation: {operation}\n\n\
        Tip: Ensure the element exists and the operation is valid for this element type."
    )]
    DOMAPIError {
        function: String,
        error: String,
        element_id: String,
        operation: String,
    },
    
    /// Event handling error
    #[error(
        "Event handling error: {error}\n\n\
        Event type: {event_type}\n\
        Handler: {handler_name}\n\n\
        Tip: Check the event handler implementation for errors."
    )]
    EventHandlingError {
        error: String,
        event_type: String,
        handler_name: String,
    },
    
    /// Native renderer error
    #[error(
        "Native renderer error for backend '{backend}': {error}\n\n\
        Tip: Check that the backend is supported and the native render script is correct."
    )]
    NativeRendererError {
        backend: String,
        error: String,
    },
}

impl ScriptError {
    /// Create a function not found error with helpful context
    pub fn function_not_found(function: String, available: Vec<String>) -> Self {
        Self::FunctionNotFound { 
            function, 
            available: available.join(", ") 
        }
    }
    
    /// Create an execution error with context
    pub fn execution_failed(function: String, error: String, context: String) -> Self {
        Self::ExecutionFailed { function, error, context }
    }
    
    /// Create a bridge setup error
    pub fn bridge_setup_failed(error: String, context: String) -> Self {
        Self::BridgeSetupFailed { error, context }
    }
    
    /// Create a DOM API error
    pub fn dom_api_error(function: String, error: String, element_id: String, operation: String) -> Self {
        Self::DOMAPIError { function, error, element_id, operation }
    }
    
    /// Get error severity level
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            Self::NotInitialized => ErrorSeverity::Critical,
            Self::UnsupportedLanguage { .. } => ErrorSeverity::Critical,
            Self::MemoryLimitExceeded { .. } => ErrorSeverity::Critical,
            Self::CompilationFailed { .. } => ErrorSeverity::High,
            Self::BytecodeExecutionFailed { .. } => ErrorSeverity::High,
            Self::BridgeSetupFailed { .. } => ErrorSeverity::High,
            Self::ExecutionFailed { .. } => ErrorSeverity::Medium,
            Self::ReactiveVariableSetupFailed { .. } => ErrorSeverity::Medium,
            Self::FunctionNotFound { .. } => ErrorSeverity::Low,
            Self::TypeConversionFailed { .. } => ErrorSeverity::Low,
            Self::RegistryError { .. } => ErrorSeverity::Medium,
            Self::ConfigurationError { .. } => ErrorSeverity::High,
            Self::DOMAPIError { .. } => ErrorSeverity::Medium,
            Self::EventHandlingError { .. } => ErrorSeverity::Medium,
            Self::NativeRendererError { .. } => ErrorSeverity::Medium,
        }
    }
    
    /// Get suggested actions for resolving the error
    pub fn suggested_actions(&self) -> Vec<String> {
        match self {
            Self::NotInitialized => vec![
                "Call ScriptSystem::initialize() before using scripts".to_string(),
                "Ensure KRB file and elements are loaded".to_string(),
            ],
            Self::FunctionNotFound { available, .. } => {
                let mut actions = vec!["Check the function name for typos".to_string()];
                if !available.is_empty() {
                    actions.push(format!("Available functions: {}", available));
                }
                actions.push("Ensure the script containing the function is loaded".to_string());
                actions
            },
            Self::UnsupportedLanguage { language, .. } => vec![
                format!("Enable the feature flag for '{}' language", language),
                "Check Cargo.toml for required features".to_string(),
                "Recompile with the appropriate feature flags".to_string(),
            ],
            Self::MemoryLimitExceeded { .. } => vec![
                "Optimize scripts to use less memory".to_string(),
                "Increase memory limits in configuration".to_string(),
                "Remove unused variables and functions".to_string(),
            ],
            _ => vec!["Check the error message for specific guidance".to_string()],
        }
    }
}

/// Error severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorSeverity {
    /// Low severity - system can continue normally
    Low,
    /// Medium severity - functionality may be impaired
    Medium,
    /// High severity - significant functionality loss
    High,
    /// Critical severity - system cannot function
    Critical,
}

/// Context information for error reporting
#[derive(Debug, Clone)]
pub struct ErrorContext {
    /// Script name where error occurred
    pub script_name: Option<String>,
    /// Function name where error occurred
    pub function_name: Option<String>,
    /// Line number in script
    pub line_number: Option<usize>,
    /// Column number in script
    pub column_number: Option<usize>,
    /// Additional context data
    pub extra_data: HashMap<String, String>,
}

impl ErrorContext {
    /// Create a new error context
    pub fn new() -> Self {
        Self {
            script_name: None,
            function_name: None,
            line_number: None,
            column_number: None,
            extra_data: HashMap::new(),
        }
    }
    
    /// Add script context
    pub fn with_script(mut self, name: String) -> Self {
        self.script_name = Some(name);
        self
    }
    
    /// Add function context
    pub fn with_function(mut self, name: String) -> Self {
        self.function_name = Some(name);
        self
    }
    
    /// Add location context
    pub fn with_location(mut self, line: usize, column: usize) -> Self {
        self.line_number = Some(line);
        self.column_number = Some(column);
        self
    }
    
    /// Add extra data
    pub fn with_data(mut self, key: String, value: String) -> Self {
        self.extra_data.insert(key, value);
        self
    }
}

impl Default for ErrorContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Result type for script operations
pub type ScriptResult<T> = std::result::Result<T, ScriptError>;

//! Script engine registry and management
//!
//! This module provides centralized management of script engines,
//! including registration, creation, and coordination of multiple
//! scripting languages within a single application.

use std::collections::HashMap;
use anyhow::Result;
use crate::script::{
    engine_trait::{ScriptEngine, ScriptEngineFactory, BridgeData},
    error::ScriptError,
    lua::LuaEngineFactory,
};

/// Registry for managing script engines
/// 
/// The registry handles:
/// - Engine factory registration
/// - Engine lifecycle management  
/// - Multi-language coordination
/// - Resource management
pub struct ScriptRegistry {
    /// Available engine factories by language name
    factories: HashMap<String, Box<dyn ScriptEngineFactory>>,
    /// Active engine instances by language name
    engines: HashMap<String, Box<dyn ScriptEngine>>,
    /// Configuration settings
    config: RegistryConfig,
}

/// Configuration for the script registry
#[derive(Debug, Clone)]
pub struct RegistryConfig {
    /// Default language to use when none specified
    pub default_language: String,
    /// Maximum number of concurrent engines
    pub max_engines: usize,
    /// Memory limit per engine (in bytes)
    pub memory_limit_per_engine: Option<usize>,
    /// Whether to enable debug logging
    pub debug_logging: bool,
}

impl Default for RegistryConfig {
    fn default() -> Self {
        Self {
            default_language: "lua".to_string(),
            max_engines: 4, // Support Lua, JS, Python, Wren
            memory_limit_per_engine: Some(1024 * 1024), // 1MB per engine
            debug_logging: false,
        }
    }
}

impl ScriptRegistry {
    /// Create a new script registry
    pub fn new() -> Result<Self> {
        let mut registry = Self {
            factories: HashMap::new(),
            engines: HashMap::new(),
            config: RegistryConfig::default(),
        };
        
        // Register available engine factories
        registry.register_builtin_factories();
        
        Ok(registry)
    }
    
    /// Create registry with custom configuration
    pub fn with_config(config: RegistryConfig) -> Result<Self> {
        let mut registry = Self::new()?;
        registry.config = config;
        Ok(registry)
    }
    
    /// Register built-in engine factories based on enabled features
    fn register_builtin_factories(&mut self) {
        // Lua engine (always available since it's the primary language)
        #[cfg(feature = "lua-vm")]
        self.register_factory(Box::new(LuaEngineFactory::new()));
        
        // JavaScript engine (future implementation)
        #[cfg(feature = "javascript-vm")]
        self.register_factory(Box::new(crate::script::javascript::JavaScriptEngineFactory::new()));
        
        // Python engine (future implementation)
        #[cfg(feature = "rustpython-vm")]
        self.register_factory(Box::new(crate::script::python::PythonEngineFactory::new()));
        
        // Wren engine (future implementation)
        #[cfg(feature = "wren-vm")]
        self.register_factory(Box::new(crate::script::wren::WrenEngineFactory::new()));
    }
    
    /// Register a script engine factory
    pub fn register_factory(&mut self, factory: Box<dyn ScriptEngineFactory>) {
        let language = factory.language_name().to_string();
        self.factories.insert(language, factory);
        
        if self.config.debug_logging {
            tracing::debug!("Registered script engine factory for language: {}", 
                          self.factories.keys().last().unwrap());
        }
    }
    
    /// Check if a language is supported
    pub fn is_language_supported(&self, language: &str) -> bool {
        self.factories.contains_key(language) && 
        self.factories[language].is_available()
    }
    
    /// Get list of supported languages
    pub fn supported_languages(&self) -> Vec<String> {
        self.factories.iter()
            .filter(|(_, factory)| factory.is_available())
            .map(|(lang, _)| lang.clone())
            .collect()
    }
    
    /// Get or create an engine for the specified language
    pub fn get_or_create_engine(&mut self, language: &str) -> Result<&mut Box<dyn ScriptEngine>> {
        // Check if engine already exists
        if !self.engines.contains_key(language) {
            self.create_engine(language)?;
        }
        
        // Get supported languages before borrowing engines mutably
        if !self.engines.contains_key(language) {
            let supported = self.supported_languages().join(", ");
            return Err(ScriptError::UnsupportedLanguage {
                language: language.to_string(),
                supported,
            }.into());
        }
        
        Ok(self.engines.get_mut(language).unwrap())
    }
    
    /// Create an engine for the specified language
    fn create_engine(&mut self, language: &str) -> Result<()> {
        // Check engine limit
        if self.engines.len() >= self.config.max_engines {
            return Err(ScriptError::RegistryError {
                error: format!("Maximum number of engines ({}) exceeded", self.config.max_engines),
                available_engines: self.engines.keys().cloned().collect(),
            }.into());
        }
        
        // Get factory for this language
        let factory = self.factories.get(language)
            .ok_or_else(|| ScriptError::UnsupportedLanguage {
                language: language.to_string(),
                supported: self.supported_languages().join(", "),
            })?;
        
        // Check if factory is available
        if !factory.is_available() {
            return Err(ScriptError::UnsupportedLanguage {
                language: language.to_string(),
                supported: self.supported_languages().join(", "),
            }.into());
        }
        
        // Create the engine
        let engine = factory.create_engine().map_err(|e| ScriptError::RegistryError {
            error: format!("Failed to create {} engine: {}", language, e),
            available_engines: self.supported_languages().join(", "),
        })?;
        
        // Store the engine
        self.engines.insert(language.to_string(), engine);
        
        if self.config.debug_logging {
            tracing::info!("Created script engine for language: {}", language);
        }
        
        Ok(())
    }
    
    /// Get an existing engine (read-only)
    pub fn get_engine(&self, language: &str) -> Option<&Box<dyn ScriptEngine>> {
        self.engines.get(language)
    }
    
    /// Get an existing engine (mutable)
    pub fn get_engine_mut(&mut self, language: &str) -> Option<&mut Box<dyn ScriptEngine>> {
        self.engines.get_mut(language)
    }
    
    /// Get all active engines (read-only)
    pub fn get_all_engines(&self) -> Vec<&Box<dyn ScriptEngine>> {
        self.engines.values().collect()
    }
    
    /// Get all active engines (mutable)
    pub fn get_all_engines_mut(&mut self) -> Vec<&mut Box<dyn ScriptEngine>> {
        self.engines.values_mut().collect()
    }
    
    /// Setup bridge data for all engines
    pub fn setup_bridge_for_all_engines(&mut self, bridge_data: &BridgeData) -> Result<()> {
        for (language, engine) in &mut self.engines {
            if let Err(e) = engine.setup_bridge(bridge_data) {
                tracing::error!("Failed to setup bridge for {} engine: {}", language, e);
                // Continue with other engines
            }
        }
        Ok(())
    }
    
    /// Get engine by script name (searches all engines)
    pub fn find_engine_for_function(&mut self, function_name: &str) -> Option<&mut Box<dyn ScriptEngine>> {
        for engine in self.engines.values_mut() {
            if engine.has_function(function_name) {
                return Some(engine);
            }
        }
        None
    }
    
    /// Remove an engine
    pub fn remove_engine(&mut self, language: &str) -> Option<Box<dyn ScriptEngine>> {
        let removed = self.engines.remove(language);
        if removed.is_some() && self.config.debug_logging {
            tracing::info!("Removed script engine for language: {}", language);
        }
        removed
    }
    
    /// Reset all engines
    pub fn reset_all_engines(&mut self) -> Result<()> {
        for (language, engine) in &mut self.engines {
            if let Err(e) = engine.reset() {
                tracing::error!("Failed to reset {} engine: {}", language, e);
                // Continue with other engines
            }
        }
        Ok(())
    }
    
    /// Get registry statistics
    pub fn get_statistics(&self) -> RegistryStatistics {
        let mut total_memory = 0;
        let mut memory_by_language = HashMap::new();
        
        for (language, engine) in &self.engines {
            let memory_stats = engine.get_memory_usage();
            total_memory += memory_stats.current_usage;
            memory_by_language.insert(language.clone(), memory_stats.current_usage);
        }
        
        RegistryStatistics {
            total_engines: self.engines.len(),
            available_factories: self.factories.len(),
            supported_languages: self.supported_languages(),
            total_memory_usage: total_memory,
            memory_by_language,
            max_engines: self.config.max_engines,
        }
    }
    
    /// Validate configuration
    pub fn validate_config(&self) -> Result<()> {
        // Check if default language is supported
        if !self.is_language_supported(&self.config.default_language) {
            return Err(ScriptError::ConfigurationError {
                error: format!("Default language '{}' is not supported", self.config.default_language),
                config_details: format!("Supported languages: {:?}", self.supported_languages()),
            }.into());
        }
        
        // Check if we have at least one supported language
        if self.supported_languages().is_empty() {
            return Err(ScriptError::ConfigurationError {
                error: "No script languages are available".to_string(),
                config_details: "Enable at least one script engine feature (e.g., 'lua-vm')".to_string(),
            }.into());
        }
        
        Ok(())
    }
    
    /// Update configuration
    pub fn update_config(&mut self, config: RegistryConfig) -> Result<()> {
        // Validate new configuration before applying
        let old_config = self.config.clone();
        self.config = config;
        
        if let Err(e) = self.validate_config() {
            // Restore old configuration on validation failure
            self.config = old_config;
            return Err(e);
        }
        
        Ok(())
    }
}

/// Statistics about the script registry
#[derive(Debug, Clone)]
pub struct RegistryStatistics {
    /// Number of active engines
    pub total_engines: usize,
    /// Number of available factories
    pub available_factories: usize,
    /// List of supported languages
    pub supported_languages: Vec<String>,
    /// Total memory usage across all engines
    pub total_memory_usage: usize,
    /// Memory usage by language
    pub memory_by_language: HashMap<String, usize>,
    /// Maximum allowed engines
    pub max_engines: usize,
}

impl Default for ScriptRegistry {
    fn default() -> Self {
        Self::new().expect("Failed to create default ScriptRegistry")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_registry_creation() {
        let registry = ScriptRegistry::new().unwrap();
        assert!(!registry.supported_languages().is_empty());
    }
    
    #[test]
    fn test_language_support_check() {
        let registry = ScriptRegistry::new().unwrap();
        
        // Lua should always be supported
        assert!(registry.is_language_supported("lua"));
        
        // Non-existent language should not be supported
        assert!(!registry.is_language_supported("nonexistent"));
    }
    
    #[test]
    fn test_config_validation() {
        let registry = ScriptRegistry::new().unwrap();
        assert!(registry.validate_config().is_ok());
        
        let mut invalid_config = RegistryConfig::default();
        invalid_config.default_language = "nonexistent".to_string();
        
        let mut registry_with_invalid_config = ScriptRegistry::with_config(invalid_config).unwrap();
        assert!(registry_with_invalid_config.validate_config().is_err());
    }
    
    #[test]
    fn test_statistics() {
        let registry = ScriptRegistry::new().unwrap();
        let stats = registry.get_statistics();
        
        assert!(stats.available_factories > 0);
        assert!(!stats.supported_languages.is_empty());
        assert_eq!(stats.total_engines, 0); // No engines created yet
    }
}

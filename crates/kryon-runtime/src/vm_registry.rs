// crates/kryon-runtime/src/vm_registry.rs

use std::collections::HashMap;
use anyhow::Result;
use crate::vm_trait::{ScriptVM, VMFactory, VMResourceLimits, VMCapabilities, VMError};

#[cfg(feature = "lua-vm")]
use crate::lua_vm::LuaVM;

/// Registry for managing multiple VM types and instances
/// Supports dynamic loading and configuration of VMs
pub struct VMRegistry {
    /// Available VM factories by language name
    available_factories: HashMap<String, Box<dyn VMFactory>>,
    /// Active VM instances
    active_vms: HashMap<String, Box<dyn ScriptVM>>,
    /// Default VM configuration
    default_config: VMConfig,
}

/// Configuration for VM selection and limits
#[derive(Debug, Clone)]
pub struct VMConfig {
    /// List of enabled VM languages
    pub enabled_vms: Vec<String>,
    /// Default VM language for scripts without explicit language specification
    pub default_vm: String,
    /// Per-VM settings
    pub vm_settings: HashMap<String, VMSettings>,
}

/// Settings specific to a VM instance
#[derive(Debug, Clone)]
pub struct VMSettings {
    /// Resource limits for this VM
    pub limits: VMResourceLimits,
    /// VM-specific configuration options
    pub options: HashMap<String, String>,
}

impl Default for VMConfig {
    fn default() -> Self {
        let mut vm_settings = HashMap::new();
        let mut enabled_vms = Vec::new();
        
        // Add VMs based on enabled features
        #[cfg(feature = "lua-vm")]
        {
            enabled_vms.push("lua".to_string());
            vm_settings.insert("lua".to_string(), VMSettings {
                limits: VMResourceLimits {
                    max_memory: Some(512 * 1024), // 512KB for Lua
                    max_execution_time: Some(500), // 500ms max execution
                    max_global_variables: Some(50),
                    max_string_length: Some(512),
                    max_recursion_depth: Some(16),
                },
                options: HashMap::new(),
            });
        }
        
        #[cfg(feature = "javascript-vm")]
        {
            enabled_vms.push("javascript".to_string());
            vm_settings.insert("javascript".to_string(), VMSettings {
                limits: VMResourceLimits {
                    max_memory: Some(256 * 1024), // 256KB for QuickJS
                    max_execution_time: Some(1000),
                    max_global_variables: Some(30),
                    max_string_length: Some(256),
                    max_recursion_depth: Some(16),
                },
                options: HashMap::new(),
            });
        }
        
        #[cfg(feature = "rustpython-vm")]
        {
            enabled_vms.push("python".to_string());
            vm_settings.insert("python".to_string(), VMSettings {
                limits: VMResourceLimits {
                    max_memory: Some(512 * 1024), // 512KB for RustPython (more than MicroPython)
                    max_execution_time: Some(1000),
                    max_global_variables: Some(50),
                    max_string_length: Some(512),
                    max_recursion_depth: Some(16),
                },
                options: HashMap::new(),
            });
        }
        
        #[cfg(feature = "wren-vm")]
        {
            enabled_vms.push("wren".to_string());
            vm_settings.insert("wren".to_string(), VMSettings {
                limits: VMResourceLimits {
                    max_memory: Some(128 * 1024), // 128KB for Wren
                    max_execution_time: Some(500),
                    max_global_variables: Some(20),
                    max_string_length: Some(128),
                    max_recursion_depth: Some(8),
                },
                options: HashMap::new(),
            });
        }

        // Default to the first available VM, or empty if no VMs enabled
        let default_vm = if !enabled_vms.is_empty() {
            enabled_vms[0].clone()
        } else {
            // No VMs enabled - this is valid for apps without scripts
            String::new()
        };

        Self {
            enabled_vms,
            default_vm,
            vm_settings,
        }
    }
}

impl VMRegistry {
    /// Create a new VM registry with default configuration
    pub fn new() -> Self {
        let mut registry = Self {
            available_factories: HashMap::new(),
            active_vms: HashMap::new(),
            default_config: VMConfig::default(),
        };

        // Register VM factories based on enabled features
        #[cfg(feature = "lua-vm")]
        registry.register_factory("lua", Box::new(LuaVMFactory));

        #[cfg(feature = "javascript-vm")]
        registry.register_factory("javascript", Box::new(JavaScriptVMFactory));

        #[cfg(feature = "rustpython-vm")]
        registry.register_factory("python", Box::new(RustPythonVMFactory));

        #[cfg(feature = "wren-vm")]
        registry.register_factory("wren", Box::new(WrenVMFactory));

        registry
    }

    /// Create VM registry with custom configuration
    pub fn with_config(config: VMConfig) -> Self {
        let mut registry = Self::new();
        registry.default_config = config;
        registry
    }

    /// Register a VM factory for a specific language
    pub fn register_factory(&mut self, language: &str, factory: Box<dyn VMFactory>) {
        self.available_factories.insert(language.to_string(), factory);
    }

    /// Check if a VM language is available
    pub fn is_available(&self, language: &str) -> bool {
        self.available_factories.get(language)
            .map(|factory| factory.is_available())
            .unwrap_or(false)
    }

    /// Validate that a script language is supported, providing helpful error message if not
    pub fn validate_script_language(&self, language: &str) -> Result<()> {
        // Check if we have a factory for this language
        if !self.available_factories.contains_key(language) {
            return Err(self.create_missing_vm_error(language));
        }

        // Check if the VM is actually available (dependencies met)
        if !self.is_available(language) {
            return Err(anyhow::anyhow!(
                "VM for '{}' language is registered but not available. This usually means dependencies are missing.",
                language
            ));
        }

        Ok(())
    }

    /// Create helpful error message for missing VM features
    fn create_missing_vm_error(&self, language: &str) -> anyhow::Error {
        let feature_name = match language {
            "lua" => "lua-vm",
            "javascript" => "javascript-vm", 
            "python" => "rustpython-vm",
            "wren" => "wren-vm",
            _ => "unknown-vm",
        };

        let available_vms: Vec<&str> = self.available_factories.keys().map(|s| s.as_str()).collect();
        let available_list = if available_vms.is_empty() {
            "none (no VM features enabled)".to_string()
        } else {
            available_vms.join(", ")
        };

        anyhow::anyhow!(
            "Script language '{}' is not supported.\n\
            \n\
            To fix this, add the '{}' feature to your build:\n\
            \n\
            For cargo build:\n\
            cargo build --features {}\n\
            \n\
            For Cargo.toml:\n\
            kryon-runtime = {{ features = [\"{}\"] }}\n\
            \n\
            Available VMs: {}\n\
            Required for: @function \"{}\" or @script \"{}\"",
            language,
            feature_name,
            feature_name,
            feature_name,
            available_list,
            language,
            language
        )
    }

    /// Get list of available VM languages
    pub fn available_languages(&self) -> Vec<String> {
        self.available_factories.keys()
            .filter(|lang| self.is_available(lang))
            .cloned()
            .collect()
    }

    /// Create a VM instance for a specific language
    pub fn create_vm(&mut self, language: &str) -> Result<()> {
        if self.active_vms.contains_key(language) {
            return Ok(()); // VM already exists
        }

        let factory = self.available_factories.get(language)
            .ok_or_else(|| VMError::UnsupportedLanguage(language.to_string()))?;

        if !factory.is_available() {
            return Err(VMError::UnsupportedLanguage(
                format!("VM '{}' is not available (missing dependencies?)", language)
            ).into());
        }

        // Get settings for this VM
        let settings = self.default_config.vm_settings.get(language)
            .cloned()
            .unwrap_or_else(|| VMSettings {
                limits: factory.default_limits(),
                options: HashMap::new(),
            });

        // Create VM with limits
        let vm = factory.create_vm_with_limits(settings.limits)?;
        self.active_vms.insert(language.to_string(), vm);

        tracing::info!("Created VM for language: {}", language);
        Ok(())
    }

    /// Get a mutable reference to a VM instance
    pub fn get_vm_mut(&mut self, language: &str) -> Option<&mut Box<dyn ScriptVM>> {
        self.active_vms.get_mut(language)
    }

    /// Get an immutable reference to a VM instance
    pub fn get_vm(&self, language: &str) -> Option<&Box<dyn ScriptVM>> {
        self.active_vms.get(language)
    }

    /// Get all active VMs as mutable references
    pub fn get_all_vms_mut(&mut self) -> Vec<&mut Box<dyn ScriptVM>> {
        self.active_vms.values_mut().collect()
    }

    /// Get all active VMs as immutable references
    pub fn get_all_vms(&self) -> Vec<&Box<dyn ScriptVM>> {
        self.active_vms.values().collect()
    }

    /// Initialize VMs for all enabled languages
    pub fn initialize_enabled_vms(&mut self) -> Result<()> {
        let enabled_languages = self.default_config.enabled_vms.clone();
        for language in &enabled_languages {
            if let Err(e) = self.create_vm(language) {
                tracing::warn!("Failed to initialize VM for {}: {}", language, e);
                // Continue with other VMs even if one fails
            }
        }
        Ok(())
    }

    /// Remove a VM instance
    pub fn remove_vm(&mut self, language: &str) -> Option<Box<dyn ScriptVM>> {
        self.active_vms.remove(language)
    }

    /// Get VM capabilities for a language
    pub fn get_capabilities(&self, language: &str) -> Option<VMCapabilities> {
        self.available_factories.get(language)
            .map(|factory| factory.capabilities())
    }

    /// Get total memory usage across all VMs
    pub fn get_total_memory_usage(&self) -> usize {
        self.active_vms.values()
            .map(|vm| vm.get_memory_usage().current_usage)
            .sum()
    }

    /// Get memory usage by language
    pub fn get_memory_usage_by_language(&self) -> HashMap<String, usize> {
        self.active_vms.iter()
            .map(|(lang, vm)| (lang.clone(), vm.get_memory_usage().current_usage))
            .collect()
    }

    /// Reset all VMs
    pub fn reset_all_vms(&mut self) -> Result<()> {
        for vm in self.active_vms.values_mut() {
            vm.reset()?;
        }
        Ok(())
    }

    /// Get the default VM language
    pub fn default_language(&self) -> &str {
        &self.default_config.default_vm
    }

    /// Update configuration
    pub fn update_config(&mut self, config: VMConfig) {
        self.default_config = config;
    }

    /// Check if multi-VM mode is enabled (more than one VM is active)
    pub fn is_multi_vm_mode(&self) -> bool {
        self.active_vms.len() > 1
    }

    /// Get registry statistics
    pub fn get_stats(&self) -> VMRegistryStats {
        VMRegistryStats {
            available_factories: self.available_factories.len(),
            active_vms: self.active_vms.len(),
            enabled_languages: self.default_config.enabled_vms.clone(),
            total_memory_usage: self.get_total_memory_usage(),
            memory_by_language: self.get_memory_usage_by_language(),
        }
    }
}

/// Statistics about the VM registry
#[derive(Debug, Clone)]
pub struct VMRegistryStats {
    pub available_factories: usize,
    pub active_vms: usize,
    pub enabled_languages: Vec<String>,
    pub total_memory_usage: usize,
    pub memory_by_language: HashMap<String, usize>,
}

/// Factory for creating Lua VM instances
pub struct LuaVMFactory;

impl VMFactory for LuaVMFactory {
    fn create_vm(&self) -> Result<Box<dyn ScriptVM>> {
        let vm = LuaVM::new()?;
        Ok(Box::new(vm))
    }

    fn create_vm_with_limits(&self, limits: VMResourceLimits) -> Result<Box<dyn ScriptVM>> {
        let mut vm = LuaVM::new()?;
        vm.set_limits(limits)?;
        Ok(Box::new(vm))
    }

    fn language_name(&self) -> &'static str {
        "lua"
    }

    fn is_available(&self) -> bool {
        // Lua is always available since it's built-in
        true
    }

    fn default_limits(&self) -> VMResourceLimits {
        VMResourceLimits {
            max_memory: Some(512 * 1024), // 512KB for microcontrollers
            max_execution_time: Some(500), // 500ms
            max_global_variables: Some(50),
            max_string_length: Some(512),
            max_recursion_depth: Some(16),
        }
    }

    fn capabilities(&self) -> VMCapabilities {
        VMCapabilities {
            supports_arithmetic: true,
            supports_string_ops: true,
            supports_objects: true,
            supports_arrays: true,
            supports_metamethods: true,
            embedded_optimized: true, // LuaJIT is good for embedded
            supports_jit: true,
        }
    }
}

// Placeholder VM factories for future implementation
#[cfg(feature = "javascript-vm")]
pub struct JavaScriptVMFactory;

#[cfg(feature = "javascript-vm")]
impl VMFactory for JavaScriptVMFactory {
    fn create_vm(&self) -> Result<Box<dyn ScriptVM>> {
        todo!("JavaScript VM implementation pending")
    }

    fn create_vm_with_limits(&self, _limits: VMResourceLimits) -> Result<Box<dyn ScriptVM>> {
        todo!("JavaScript VM implementation pending")
    }

    fn language_name(&self) -> &'static str {
        "javascript"
    }

    fn is_available(&self) -> bool {
        false // Will be true when implemented
    }

    fn default_limits(&self) -> VMResourceLimits {
        VMResourceLimits {
            max_memory: Some(256 * 1024), // 256KB for QuickJS
            max_execution_time: Some(1000),
            max_global_variables: Some(30),
            max_string_length: Some(256),
            max_recursion_depth: Some(16),
        }
    }

    fn capabilities(&self) -> VMCapabilities {
        VMCapabilities {
            supports_arithmetic: true,
            supports_string_ops: true,
            supports_objects: true,
            supports_arrays: true,
            supports_metamethods: false, // JavaScript doesn't have Lua-style metamethods
            embedded_optimized: true, // QuickJS is optimized for embedded
            supports_jit: false, // QuickJS doesn't have JIT
        }
    }
}

#[cfg(feature = "rustpython-vm")]
pub struct RustPythonVMFactory;

#[cfg(feature = "rustpython-vm")]
impl VMFactory for RustPythonVMFactory {
    fn create_vm(&self) -> Result<Box<dyn ScriptVM>> {
        todo!("RustPython VM implementation pending")
    }

    fn create_vm_with_limits(&self, _limits: VMResourceLimits) -> Result<Box<dyn ScriptVM>> {
        todo!("RustPython VM implementation pending")
    }

    fn language_name(&self) -> &'static str {
        "python"
    }

    fn is_available(&self) -> bool {
        false // Will be true when implemented
    }

    fn default_limits(&self) -> VMResourceLimits {
        VMResourceLimits {
            max_memory: Some(512 * 1024), // 512KB for RustPython
            max_execution_time: Some(1000),
            max_global_variables: Some(50),
            max_string_length: Some(512),
            max_recursion_depth: Some(16),
        }
    }

    fn capabilities(&self) -> VMCapabilities {
        VMCapabilities {
            supports_arithmetic: true,
            supports_string_ops: true,
            supports_objects: true,
            supports_arrays: true,
            supports_metamethods: true, // Python has __getattr__, __setattr__
            embedded_optimized: false, // RustPython is not optimized for embedded systems
            supports_jit: false, // RustPython doesn't have JIT
        }
    }
}

#[cfg(feature = "wren-vm")]
pub struct WrenVMFactory;

#[cfg(feature = "wren-vm")]
impl VMFactory for WrenVMFactory {
    fn create_vm(&self) -> Result<Box<dyn ScriptVM>> {
        todo!("Wren VM implementation pending")
    }

    fn create_vm_with_limits(&self, _limits: VMResourceLimits) -> Result<Box<dyn ScriptVM>> {
        todo!("Wren VM implementation pending")
    }

    fn language_name(&self) -> &'static str {
        "wren"
    }

    fn is_available(&self) -> bool {
        false // Will be true when implemented
    }

    fn default_limits(&self) -> VMResourceLimits {
        VMResourceLimits {
            max_memory: Some(128 * 1024), // 128KB for Wren (very lightweight)
            max_execution_time: Some(500),
            max_global_variables: Some(20),
            max_string_length: Some(128),
            max_recursion_depth: Some(8),
        }
    }

    fn capabilities(&self) -> VMCapabilities {
        VMCapabilities {
            supports_arithmetic: true,
            supports_string_ops: true,
            supports_objects: true,
            supports_arrays: true,
            supports_metamethods: false,
            embedded_optimized: true, // Wren is designed for embedded systems
            supports_jit: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vm_registry_creation() {
        let registry = VMRegistry::new();
        assert!(registry.is_available("lua"));
        assert_eq!(registry.available_languages(), vec!["lua"]);
        assert_eq!(registry.default_language(), "lua");
    }

    #[test]
    fn test_vm_creation() {
        let mut registry = VMRegistry::new();
        registry.create_vm("lua").unwrap();
        assert!(registry.get_vm("lua").is_some());
    }

    #[test]
    fn test_vm_capabilities() {
        let registry = VMRegistry::new();
        let caps = registry.get_capabilities("lua").unwrap();
        assert!(caps.supports_arithmetic);
        assert!(caps.supports_metamethods);
        assert!(caps.embedded_optimized);
    }

    #[test]
    fn test_memory_tracking() {
        let mut registry = VMRegistry::new();
        registry.create_vm("lua").unwrap();
        
        let stats = registry.get_stats();
        assert_eq!(stats.active_vms, 1);
        assert!(stats.memory_by_language.contains_key("lua"));
    }
}
// crates/kryon-runtime/src/shared_data.rs

use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use anyhow::Result;
use crate::vm_trait::{ScriptValue, ScriptVM};
use crate::template_engine::TemplateEngine;

/// Listener trait for variable changes across VMs
pub trait ChangeListener: Send + Sync {
    fn on_variable_changed(&self, name: &str, old_value: &str, new_value: &str);
    fn on_element_property_changed(&self, element_id: u32, property: &str, value: &str);
}

/// Shared data layer that synchronizes variables across multiple VMs
/// This is the central coordination point for cross-VM communication
pub struct SharedScriptData {
    /// Global variables accessible by all VMs
    variables: RwLock<HashMap<String, String>>,
    
    /// Template engine for reactive UI updates
    template_engine: Arc<Mutex<TemplateEngine>>,
    
    /// Change listeners that get notified of variable updates
    change_listeners: Vec<Box<dyn ChangeListener>>,
    
    /// Pending element property changes from scripts
    pending_style_changes: Arc<Mutex<HashMap<u32, u8>>>,
    pending_text_changes: Arc<Mutex<HashMap<u32, String>>>,
    pending_state_changes: Arc<Mutex<HashMap<u32, bool>>>,
    pending_visibility_changes: Arc<Mutex<HashMap<u32, bool>>>,
    
    /// Cross-VM function registry (functions callable from any VM)
    cross_vm_functions: RwLock<HashMap<String, CrossVMFunction>>,
}

/// A function that can be called from any VM regardless of where it was defined
#[derive(Debug, Clone)]
pub struct CrossVMFunction {
    pub source_vm: String,
    pub function_name: String,
    pub parameter_types: Vec<String>,
    pub return_type: String,
}

impl SharedScriptData {
    /// Create new shared data with a template engine
    pub fn new(template_engine: TemplateEngine) -> Self {
        Self {
            variables: RwLock::new(HashMap::new()),
            template_engine: Arc::new(Mutex::new(template_engine)),
            change_listeners: Vec::new(),
            pending_style_changes: Arc::new(Mutex::new(HashMap::new())),
            pending_text_changes: Arc::new(Mutex::new(HashMap::new())),
            pending_state_changes: Arc::new(Mutex::new(HashMap::new())),
            pending_visibility_changes: Arc::new(Mutex::new(HashMap::new())),
            cross_vm_functions: RwLock::new(HashMap::new()),
        }
    }

    /// Set a global variable and sync it to all VMs
    pub fn set_variable(&self, name: &str, value: &str, vms: &mut [Box<dyn ScriptVM>]) -> Result<()> {
        let old_value = {
            let mut variables = self.variables.write().unwrap();
            let old = variables.get(name).cloned();
            variables.insert(name.to_string(), value.to_string());
            old
        };

        // Notify change listeners
        if let Some(old) = &old_value {
            if old != value {
                for listener in &self.change_listeners {
                    listener.on_variable_changed(name, old, value);
                }
            }
        } else {
            for listener in &self.change_listeners {
                listener.on_variable_changed(name, "", value);
            }
        }

        // Sync to all VMs
        for vm in vms {
            vm.set_global_variable(name, ScriptValue::String(value.to_string()))?;
        }

        // Update template engine if this is a template variable
        if let Ok(mut engine) = self.template_engine.lock() {
            if engine.set_variable(name, value) {
                tracing::info!("Template variable '{}' updated to '{}' via shared data", name, value);
            }
        }

        Ok(())
    }

    /// Get a global variable value
    pub fn get_variable(&self, name: &str) -> Option<String> {
        self.variables.read().unwrap().get(name).cloned()
    }

    /// Get all variables
    pub fn get_all_variables(&self) -> HashMap<String, String> {
        self.variables.read().unwrap().clone()
    }

    /// Initialize variables from template engine
    pub fn initialize_from_template_engine(&self) -> Result<()> {
        if let Ok(engine) = self.template_engine.lock() {
            let template_vars = engine.get_variables();
            let mut variables = self.variables.write().unwrap();
            for (name, value) in template_vars {
                variables.insert(name.clone(), value.clone());
            }
        }
        Ok(())
    }

    /// Add a change listener
    pub fn add_change_listener(&mut self, listener: Box<dyn ChangeListener>) {
        self.change_listeners.push(listener);
    }

    /// Sync variable changes from a specific VM to all other VMs
    pub fn sync_changes_from_vm(
        &self, 
        source_vm_index: usize, 
        vms: &mut [Box<dyn ScriptVM>]
    ) -> Result<HashMap<String, String>> {
        let mut all_changes = HashMap::new();

        if source_vm_index >= vms.len() {
            return Ok(all_changes);
        }

        // Get changes from the source VM
        let changes = vms[source_vm_index].get_pending_changes()?;

        // Process template variable changes
        if let Some(template_changes) = changes.get("template_variables") {
            for (name, value) in template_changes {
                // Update shared variables
                {
                    let mut variables = self.variables.write().unwrap();
                    variables.insert(name.clone(), value.clone());
                }

                // Sync to all other VMs
                for (vm_index, vm) in vms.iter_mut().enumerate() {
                    if vm_index != source_vm_index {
                        vm.set_global_variable(name, ScriptValue::String(value.clone()))?;
                    }
                }

                // Update template engine
                if let Ok(mut engine) = self.template_engine.lock() {
                    engine.set_variable(name, value);
                }

                all_changes.insert(name.clone(), value.clone());
            }
        }

        // Process other change types (element properties, etc.)
        self.process_element_changes(&changes)?;

        Ok(all_changes)
    }

    /// Process element property changes from scripts
    fn process_element_changes(&self, changes: &HashMap<String, HashMap<String, String>>) -> Result<()> {
        // Process style changes
        if let Some(style_changes) = changes.get("style_changes") {
            let mut pending = self.pending_style_changes.lock().unwrap();
            for (element_id_str, style_id_str) in style_changes {
                if let (Ok(element_id), Ok(style_id)) = (element_id_str.parse::<u32>(), style_id_str.parse::<u8>()) {
                    pending.insert(element_id, style_id);
                }
            }
        }

        // Process text changes
        if let Some(text_changes) = changes.get("text_changes") {
            let mut pending = self.pending_text_changes.lock().unwrap();
            for (element_id_str, text) in text_changes {
                if let Ok(element_id) = element_id_str.parse::<u32>() {
                    pending.insert(element_id, text.clone());
                }
            }
        }

        // Process state changes
        if let Some(state_changes) = changes.get("state_changes") {
            let mut pending = self.pending_state_changes.lock().unwrap();
            for (element_id_str, checked_str) in state_changes {
                if let (Ok(element_id), Ok(checked)) = (element_id_str.parse::<u32>(), checked_str.parse::<bool>()) {
                    pending.insert(element_id, checked);
                }
            }
        }

        // Process visibility changes
        if let Some(visibility_changes) = changes.get("visibility_changes") {
            let mut pending = self.pending_visibility_changes.lock().unwrap();
            for (element_id_str, visible_str) in visibility_changes {
                if let (Ok(element_id), Ok(visible)) = (element_id_str.parse::<u32>(), visible_str.parse::<bool>()) {
                    pending.insert(element_id, visible);
                }
            }
        }

        Ok(())
    }

    /// Get pending style changes and clear them
    pub fn take_pending_style_changes(&self) -> HashMap<u32, u8> {
        let mut pending = self.pending_style_changes.lock().unwrap();
        std::mem::take(&mut *pending)
    }

    /// Get pending text changes and clear them
    pub fn take_pending_text_changes(&self) -> HashMap<u32, String> {
        let mut pending = self.pending_text_changes.lock().unwrap();
        std::mem::take(&mut *pending)
    }

    /// Get pending state changes and clear them
    pub fn take_pending_state_changes(&self) -> HashMap<u32, bool> {
        let mut pending = self.pending_state_changes.lock().unwrap();
        std::mem::take(&mut *pending)
    }

    /// Get pending visibility changes and clear them
    pub fn take_pending_visibility_changes(&self) -> HashMap<u32, bool> {
        let mut pending = self.pending_visibility_changes.lock().unwrap();
        std::mem::take(&mut *pending)
    }

    /// Register a cross-VM function that can be called from any VM
    pub fn register_cross_vm_function(&self, name: &str, function: CrossVMFunction) {
        let mut functions = self.cross_vm_functions.write().unwrap();
        functions.insert(name.to_string(), function);
    }

    /// Check if a cross-VM function exists
    pub fn has_cross_vm_function(&self, name: &str) -> bool {
        self.cross_vm_functions.read().unwrap().contains_key(name)
    }

    /// Get cross-VM function information
    pub fn get_cross_vm_function(&self, name: &str) -> Option<CrossVMFunction> {
        self.cross_vm_functions.read().unwrap().get(name).cloned()
    }

    /// Call a cross-VM function through the appropriate VM
    pub fn call_cross_vm_function(
        &self,
        name: &str,
        args: Vec<ScriptValue>,
        vms: &mut [Box<dyn ScriptVM>]
    ) -> Result<ScriptValue> {
        let function_info = self.get_cross_vm_function(name)
            .ok_or_else(|| anyhow::anyhow!("Cross-VM function '{}' not found", name))?;

        // Find the VM that owns this function
        for vm in vms {
            if vm.language_name() == function_info.source_vm {
                return vm.call_function(&function_info.function_name, args);
            }
        }

        Err(anyhow::anyhow!("Source VM '{}' for function '{}' not found", function_info.source_vm, name))
    }

    /// Get memory usage across all shared data structures
    pub fn get_memory_usage(&self) -> SharedDataMemoryStats {
        let variables_count = self.variables.read().unwrap().len();
        let style_changes_count = self.pending_style_changes.lock().unwrap().len();
        let text_changes_count = self.pending_text_changes.lock().unwrap().len();
        let state_changes_count = self.pending_state_changes.lock().unwrap().len();
        let visibility_changes_count = self.pending_visibility_changes.lock().unwrap().len();
        let cross_vm_functions_count = self.cross_vm_functions.read().unwrap().len();

        SharedDataMemoryStats {
            variables_count,
            pending_changes_count: style_changes_count + text_changes_count + state_changes_count + visibility_changes_count,
            cross_vm_functions_count,
            change_listeners_count: self.change_listeners.len(),
        }
    }
}

/// Memory usage statistics for shared data
#[derive(Debug, Clone)]
pub struct SharedDataMemoryStats {
    pub variables_count: usize,
    pub pending_changes_count: usize,
    pub cross_vm_functions_count: usize,
    pub change_listeners_count: usize,
}

/// Implementation of ChangeListener for template engine integration
pub struct TemplateEngineChangeListener {
    template_engine: Arc<Mutex<TemplateEngine>>,
}

impl TemplateEngineChangeListener {
    pub fn new(template_engine: Arc<Mutex<TemplateEngine>>) -> Self {
        Self { template_engine }
    }
}

impl ChangeListener for TemplateEngineChangeListener {
    fn on_variable_changed(&self, name: &str, _old_value: &str, new_value: &str) {
        if let Ok(mut engine) = self.template_engine.lock() {
            engine.set_variable(name, new_value);
        }
    }

    fn on_element_property_changed(&self, _element_id: u32, _property: &str, _value: &str) {
        // Template engine doesn't directly handle element properties
        // This would be handled by the element system
    }
}
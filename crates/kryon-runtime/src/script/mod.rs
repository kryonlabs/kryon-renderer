//! Modular script system for Kryon Runtime
//!
//! This module provides a comprehensive script execution system with support for:
//! - Multiple scripting languages (Lua, JavaScript, Python, Wren)
//! - Bytecode compilation and execution
//! - DOM API bridge for UI manipulation
//! - Reactive template variables
//! - Professional error handling
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
//! │   ScriptSystem  │───▶│  ScriptEngine   │───▶│  Language VM    │
//! │   (Public API)  │    │     (Trait)     │    │   (Lua, JS...)  │
//! └─────────────────┘    └─────────────────┘    └─────────────────┘
//!           │                       │                       │
//!           ▼                       ▼                       ▼
//! ┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
//! │   Registry      │    │   Bridge API    │    │  Reactive Vars  │
//! │  (Management)   │    │  (DOM, Events)  │    │  (Templates)    │
//! └─────────────────┘    └─────────────────┘    └─────────────────┘
//! ```

use std::collections::HashMap;
use anyhow::Result;
use kryon_core::{ScriptEntry, Element, ElementId, PropertyValue, KRBFile};

pub mod engine_trait;
pub mod error;
pub mod registry;
pub mod lua;

use engine_trait::{ScriptValue, BridgeData, ChangeSet};
use error::ScriptError;
use registry::ScriptRegistry;

/// Main script system coordinator
/// 
/// This is the primary interface for script execution in Kryon Runtime.
/// It manages multiple script engines, handles bytecode execution, and
/// coordinates changes between scripts and the UI system.
pub struct ScriptSystem {
    /// Registry of available script engines
    registry: ScriptRegistry,
    /// Template variables for reactive updates
    template_variables: HashMap<String, String>,
    /// Element data for DOM API
    elements_data: HashMap<ElementId, Element>,
    /// KRB file data for style and resource access
    krb_file: Option<KRBFile>,
}

impl ScriptSystem {
    /// Create a new script system
    pub fn new() -> Result<Self> {
        let registry = ScriptRegistry::new()?;
        
        Ok(Self {
            registry,
            template_variables: HashMap::new(),
            elements_data: HashMap::new(),
            krb_file: None,
        })
    }
    
    /// Initialize the script system with KRB file data
    pub fn initialize(&mut self, krb_file: &KRBFile, elements: &HashMap<ElementId, Element>) -> Result<()> {
        // Store a reference to the KRB file (we don't need to clone the whole thing)
        // self.krb_file = Some(krb_file.clone());
        self.elements_data = elements.clone();
        
        // Initialize bridge data for all engines
        let bridge_data = self.create_bridge_data(krb_file, elements)?;
        self.registry.setup_bridge_for_all_engines(&bridge_data)?;
        
        Ok(())
    }
    
    /// Load compiled scripts from KRB file
    pub fn load_compiled_scripts(&mut self, scripts: &[ScriptEntry]) -> Result<()> {
        for script in scripts {
            self.load_compiled_script(script)?;
        }
        Ok(())
    }
    
    /// Load a single compiled script
    pub fn load_compiled_script(&mut self, script: &ScriptEntry) -> Result<()> {
        // Convert ScriptLanguage to string for engine lookup
        let language = &script.language;
        
        // Get or create engine for this language
        let engine = self.registry.get_or_create_engine(language)?;
        
        // Load bytecode into the engine
        engine.load_bytecode(&script.name, &script.bytecode)?;
        
        Ok(())
    }
    
    /// Execute a function with arguments
    pub fn call_function(&mut self, function_name: &str, args: Vec<PropertyValue>) -> Result<ScriptValue> {
        // Convert PropertyValue to ScriptValue
        let script_args: Vec<ScriptValue> = args.into_iter()
            .map(|pv| self.property_value_to_script_value(pv))
            .collect();
        
        // Try to find the function in any of the active engines
        let mut result = None;
        for engine in self.registry.get_all_engines_mut() {
            if engine.has_function(function_name) {
                result = Some(engine.call_function(function_name, script_args.clone())?);
                break;
            }
        }
        
        result.ok_or_else(|| ScriptError::FunctionNotFound {
            function: function_name.to_string(),
            available: self.get_all_function_names().join(", "),
        }.into())
    }
    
    /// Initialize template variables for reactive updates
    pub fn initialize_template_variables(&mut self, variables: &HashMap<String, String>) -> Result<()> {
        self.template_variables = variables.clone();
        
        // Initialize reactive variables in all engines
        for engine in self.registry.get_all_engines_mut() {
            engine.setup_reactive_variables(variables)?;
        }
        
        Ok(())
    }
    
    /// Execute initialization functions
    pub fn execute_init_functions(&mut self) -> Result<()> {
        // Execute onReady callbacks first
        for engine in self.registry.get_all_engines_mut() {
            engine.execute_on_ready_callbacks()?;
        }
        
        // Execute initialization functions (ending with "_init" or named "init")
        let function_names = self.get_all_function_names();
        for function_name in function_names {
            if function_name.ends_with("_init") || function_name == "init" {
                if let Err(e) = self.call_function(&function_name, vec![]) {
                    tracing::warn!("Failed to execute init function '{}': {}", function_name, e);
                }
            }
        }
        
        Ok(())
    }
    
    /// Get all pending changes from scripts
    pub fn get_pending_changes(&mut self) -> Result<HashMap<String, ChangeSet>> {
        let mut all_changes = HashMap::new();
        
        for engine in self.registry.get_all_engines_mut() {
            let engine_changes = engine.get_pending_changes()?;
            for (change_type, changes) in engine_changes {
                all_changes.insert(change_type, changes);
            }
        }
        
        Ok(all_changes)
    }
    
    /// Clear all pending changes from scripts
    pub fn clear_pending_changes(&mut self) -> Result<()> {
        for engine in self.registry.get_all_engines_mut() {
            engine.clear_pending_changes()?;
        }
        Ok(())
    }
    
    /// Apply pending changes to elements
    pub fn apply_pending_changes(&mut self, elements: &mut HashMap<ElementId, Element>) -> Result<bool> {
        let changes = self.get_pending_changes()?;
        self.apply_pending_dom_changes(elements, &changes)
    }
    
    /// Apply pending DOM changes from a given change set
    pub fn apply_pending_dom_changes(&mut self, elements: &mut HashMap<ElementId, Element>, changes: &HashMap<String, ChangeSet>) -> Result<bool> {
        let mut any_changes = false;
        
        // Apply style changes
        if let Some(style_changes) = changes.get("style_changes") {
            for (element_id_str, style_id_str) in &style_changes.data {
                if let (Ok(element_id), Ok(style_id)) = (element_id_str.parse::<ElementId>(), style_id_str.parse::<u8>()) {
                    if let Some(element) = elements.get_mut(&element_id) {
                        element.style_id = style_id;
                        any_changes = true;
                    }
                }
            }
        }
        
        // Apply text changes
        if let Some(text_changes) = changes.get("text_changes") {
            for (element_id_str, new_text) in &text_changes.data {
                if let Ok(element_id) = element_id_str.parse::<ElementId>() {
                    if let Some(element) = elements.get_mut(&element_id) {
                        element.text = new_text.clone();
                        any_changes = true;
                    }
                }
            }
        }
        
        // Apply visibility changes
        if let Some(visibility_changes) = changes.get("visibility_changes") {
            for (element_id_str, visible_str) in &visibility_changes.data {
                if let (Ok(element_id), Ok(visible)) = (element_id_str.parse::<ElementId>(), visible_str.parse::<bool>()) {
                    if let Some(element) = elements.get_mut(&element_id) {
                        element.visible = visible;
                        any_changes = true;
                    }
                }
            }
        }
        
        // Apply state changes (checked, etc.)
        if let Some(state_changes) = changes.get("state_changes") {
            for (element_id_str, checked_str) in &state_changes.data {
                if let (Ok(element_id), Ok(checked)) = (element_id_str.parse::<ElementId>(), checked_str.parse::<bool>()) {
                    if let Some(element) = elements.get_mut(&element_id) {
                        use kryon_core::InteractionState;
                        element.current_state = if checked {
                            InteractionState::Checked
                        } else {
                            InteractionState::Normal
                        };
                        any_changes = true;
                    }
                }
            }
        }
        
        // Refresh elements data in engines if changes were made
        if any_changes {
            self.elements_data = elements.clone();
            let bridge_data = self.create_bridge_data(
                self.krb_file.as_ref().ok_or_else(|| ScriptError::NotInitialized)?,
                elements
            )?;
            self.registry.setup_bridge_for_all_engines(&bridge_data)?;
        }
        
        Ok(any_changes)
    }
    
    /// Get all function names from all engines
    fn get_all_function_names(&self) -> Vec<String> {
        let mut all_functions = Vec::new();
        for engine in self.registry.get_all_engines() {
            all_functions.extend(engine.get_function_names());
        }
        all_functions
    }
    
    /// Create bridge data for DOM API
    fn create_bridge_data(&self, krb_file: &KRBFile, elements: &HashMap<ElementId, Element>) -> Result<BridgeData> {
        // Create element ID mappings
        let mut element_ids = HashMap::new();
        for (element_id, element) in elements {
            if !element.id.is_empty() {
                element_ids.insert(element.id.clone(), *element_id);
            }
        }
        
        // Create style ID mappings
        let mut style_ids = HashMap::new();
        for (style_id, style) in &krb_file.styles {
            style_ids.insert(style.name.clone(), *style_id);
        }
        
        // Create component properties
        let mut component_properties = HashMap::new();
        for (element_id, element) in elements {
            if !element.custom_properties.is_empty() {
                let mut props = HashMap::new();
                for (prop_name, prop_value) in &element.custom_properties {
                    props.insert(prop_name.clone(), self.property_value_to_script_value(prop_value.clone()));
                }
                component_properties.insert(element_id.to_string(), props.clone());
                
                // Also store by string ID if available
                if !element.id.is_empty() {
                    component_properties.insert(element.id.clone(), props.clone());
                }
            }
        }
        
        Ok(BridgeData {
            element_ids,
            style_ids,
            component_properties,
            elements_data: elements.clone(),
            template_variables: self.template_variables.clone(),
        })
    }
    
    /// Convert PropertyValue to ScriptValue
    fn property_value_to_script_value(&self, value: PropertyValue) -> ScriptValue {
        match value {
            PropertyValue::String(s) => ScriptValue::String(s),
            PropertyValue::Int(i) => ScriptValue::Integer(i as i64),
            PropertyValue::Float(f) => ScriptValue::Number(f as f64),
            PropertyValue::Bool(b) => ScriptValue::Boolean(b),
            PropertyValue::Percentage(p) => ScriptValue::Number(p as f64),
            PropertyValue::Color(color) => {
                let hex = format!("#{:02X}{:02X}{:02X}{:02X}",
                    (color.x * 255.0) as u8,
                    (color.y * 255.0) as u8,
                    (color.z * 255.0) as u8,
                    (color.w * 255.0) as u8
                );
                ScriptValue::String(hex)
            },
            PropertyValue::Resource(res) => ScriptValue::String(res),
            PropertyValue::Transform(_) => ScriptValue::String(format!("{:?}", value)),
            PropertyValue::CSSUnit(css_unit) => ScriptValue::Number(css_unit.value as f64),
            PropertyValue::RichText(rich_text) => ScriptValue::String(rich_text.to_plain_text()),
        }
    }
}

impl Default for ScriptSystem {
    fn default() -> Self {
        Self::new().expect("Failed to create default ScriptSystem")
    }
}

// Extension trait to convert ScriptLanguage to string
/// Supported script languages
#[derive(Debug, Clone, PartialEq)]
pub enum ScriptLanguage {
    Lua,
    JavaScript,
    Python,
    Wren,
}

impl ScriptLanguage {
    fn as_str(&self) -> &'static str {
        match self {
            ScriptLanguage::Lua => "lua",
            ScriptLanguage::JavaScript => "javascript",
            ScriptLanguage::Python => "python",
            ScriptLanguage::Wren => "wren",
        }
    }
}

// crates/kryon-runtime/src/template_engine.rs

use kryon_core::{KRBFile, Element, ElementId, TemplateVariable, TemplateBinding};
use std::collections::HashMap;
use regex::Regex;

/// Template evaluation engine that handles variable substitution and reactive updates
pub struct TemplateEngine {
    /// Template variables with their current values
    variables: HashMap<String, String>,
    /// Template bindings from KRB file
    bindings: Vec<TemplateBinding>,
    /// Template variables from KRB file
    template_variables: Vec<TemplateVariable>,
    /// Compiled regex for template variable extraction
    template_regex: Regex,
}

impl TemplateEngine {
    /// Create a new template engine from KRB file data
    pub fn new(krb_file: &KRBFile) -> Self {
        let template_regex = Regex::new(r"\{\{([a-zA-Z_][a-zA-Z0-9_]*)\}\}").unwrap();
        
        // Initialize variables with their default values
        let mut variables = HashMap::new();
        for template_var in &krb_file.template_variables {
            variables.insert(template_var.name.clone(), template_var.default_value.clone());
        }
        
        Self {
            variables,
            bindings: krb_file.template_bindings.clone(),
            template_variables: krb_file.template_variables.clone(),
            template_regex,
        }
    }
    
    /// Set a template variable value
    pub fn set_variable(&mut self, name: &str, value: &str) -> bool {
        if let Some(var_value) = self.variables.get_mut(name) {
            if *var_value != value {
                *var_value = value.to_string();
                return true; // Variable changed
            }
        }
        false // Variable not found or unchanged
    }
    
    /// Get a template variable value
    pub fn get_variable(&self, name: &str) -> Option<&str> {
        self.variables.get(name).map(|s| s.as_str())
    }
    
    /// Get all template variables
    pub fn get_variables(&self) -> &HashMap<String, String> {
        &self.variables
    }
    
    /// Evaluate a template expression by substituting variables
    pub fn evaluate_expression(&self, expression: &str) -> String {
        let mut result = expression.to_string();
        
        // Replace all {{variable}} patterns with their values
        for capture in self.template_regex.captures_iter(expression) {
            if let Some(var_name) = capture.get(1) {
                let var_name_str = var_name.as_str();
                if let Some(value) = self.variables.get(var_name_str) {
                    let pattern = format!("{{{{{}}}}}", var_name_str);
                    result = result.replace(&pattern, value);
                }
            }
        }
        
        result
    }
    
    /// Update all elements that have template bindings
    pub fn update_elements(&self, elements: &mut HashMap<ElementId, Element>) {
        for binding in &self.bindings {
            if let Some(element) = elements.get_mut(&(binding.element_index as u32)) {
                let evaluated_value = self.evaluate_expression(&binding.template_expression);
                
                // Update the element property based on property_id
                match binding.property_id {
                    0x08 => { // TextContent property
                        element.text = evaluated_value;
                    }
                    // Add more property types as needed
                    _ => {
                        eprintln!("[TEMPLATE] Unknown property ID: 0x{:02X}", binding.property_id);
                    }
                }
            }
        }
    }
    
    /// Get bindings that reference a specific variable
    pub fn get_bindings_for_variable(&self, variable_name: &str) -> Vec<&TemplateBinding> {
        self.bindings.iter()
            .filter(|binding| {
                self.template_regex.captures_iter(&binding.template_expression)
                    .any(|capture| {
                        capture.get(1).map_or(false, |m| m.as_str() == variable_name)
                    })
            })
            .collect()
    }
    
    /// Check if any template bindings exist
    pub fn has_bindings(&self) -> bool {
        !self.bindings.is_empty()
    }
    
    /// Get all template variable names
    pub fn get_variable_names(&self) -> Vec<String> {
        self.variables.keys().cloned().collect()
    }
    
    /// Update a variable and return affected element IDs
    pub fn update_variable_and_get_affected_elements(&mut self, name: &str, value: &str) -> Vec<u32> {
        let mut affected_elements = Vec::new();
        
        if self.set_variable(name, value) {
            // Variable changed, find all affected elements
            let bindings = self.get_bindings_for_variable(name);
            for binding in bindings {
                affected_elements.push(binding.element_index as u32);
            }
        }
        
        affected_elements
    }
}

impl std::fmt::Debug for TemplateEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TemplateEngine")
            .field("variables", &self.variables)
            .field("bindings_count", &self.bindings.len())
            .field("template_variables_count", &self.template_variables.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kryon_core::{KRBFile, KRBHeader, TemplateVariable, TemplateBinding};
    
    fn create_test_krb_file() -> KRBFile {
        let template_variables = vec![
            TemplateVariable {
                name: "counter_value".to_string(),
                value_type: 1,
                default_value: "0".to_string(),
            }
        ];
        
        let template_bindings = vec![
            TemplateBinding {
                element_index: 1,
                property_id: 0x08,
                template_expression: "Count: {{counter_value}}".to_string(),
                variable_indices: vec![0],
            }
        ];
        
        KRBFile {
            header: KRBHeader {
                magic: *b"KRB1",
                version: 0x0500,
                flags: 0,
                element_count: 2,
                style_count: 0,
                component_count: 0,
                script_count: 0,
                string_count: 5,
                resource_count: 0,
                template_variable_count: 1,
                template_binding_count: 1,
            },
            strings: vec![],
            elements: HashMap::new(),
            styles: HashMap::new(),
            root_element_id: None,
            resources: vec![],
            scripts: vec![],
            template_variables,
            template_bindings,
        }
    }
    
    #[test]
    fn test_template_engine_creation() {
        let krb_file = create_test_krb_file();
        let engine = TemplateEngine::new(&krb_file);
        
        assert_eq!(engine.get_variable("counter_value"), Some("0"));
        assert!(engine.has_bindings());
        assert_eq!(engine.get_variable_names().len(), 1);
    }
    
    #[test]
    fn test_variable_setting() {
        let krb_file = create_test_krb_file();
        let mut engine = TemplateEngine::new(&krb_file);
        
        // Setting to same value should return false
        assert!(!engine.set_variable("counter_value", "0"));
        
        // Setting to different value should return true
        assert!(engine.set_variable("counter_value", "5"));
        assert_eq!(engine.get_variable("counter_value"), Some("5"));
    }
    
    #[test]
    fn test_expression_evaluation() {
        let krb_file = create_test_krb_file();
        let mut engine = TemplateEngine::new(&krb_file);
        
        engine.set_variable("counter_value", "42");
        
        let result = engine.evaluate_expression("Count: {{counter_value}}");
        assert_eq!(result, "Count: 42");
        
        let result = engine.evaluate_expression("Value is {{counter_value}} items");
        assert_eq!(result, "Value is 42 items");
    }
    
    #[test]
    fn test_affected_elements() {
        let krb_file = create_test_krb_file();
        let mut engine = TemplateEngine::new(&krb_file);
        
        let affected = engine.update_variable_and_get_affected_elements("counter_value", "10");
        assert_eq!(affected, vec![1]);
        
        // Same value should not affect anything
        let affected = engine.update_variable_and_get_affected_elements("counter_value", "10");
        assert_eq!(affected, Vec::<u32>::new());
    }
}
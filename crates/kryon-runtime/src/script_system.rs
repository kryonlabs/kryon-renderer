// crates/kryon-runtime/src/script_system.rs
use kryon_core::{ScriptEntry, Element, ElementId, PropertyValue, KRBFile};
use std::collections::HashMap;
use std::time::Duration;
use anyhow::Result;
use mlua::Lua;

#[derive(Debug)]
pub struct ScriptSystem {
    scripts: Vec<ScriptEntry>,
    state: HashMap<String, PropertyValue>,
    lua: Lua,
}

impl ScriptSystem {
    pub fn new() -> Self {
        let lua = Lua::new();
        
        // Setup custom print function that forwards to stdout
        let _ = lua.globals().set("print", lua.create_function(|_, args: mlua::Variadic<mlua::Value>| {
            let mut output = Vec::new();
            for arg in args {
                match arg {
                    mlua::Value::String(s) => output.push(s.to_str().unwrap_or("").to_string()),
                    mlua::Value::Number(n) => output.push(n.to_string()),
                    mlua::Value::Integer(i) => output.push(i.to_string()),
                    mlua::Value::Boolean(b) => output.push(b.to_string()),
                    mlua::Value::Nil => output.push("nil".to_string()),
                    _ => output.push(format!("{:?}", arg)),
                }
            }
            println!("{}", output.join("\t"));
            Ok(())
        }).unwrap()).unwrap();
        
        Self {
            scripts: Vec::new(),
            state: HashMap::new(),
            lua,
        }
    }
    
    pub fn load_scripts(&mut self, scripts: &[ScriptEntry]) -> Result<()> {
        self.scripts = scripts.to_vec();
        
        for script in &self.scripts {
            // Load Lua scripts into the Lua context
            if script.language == "lua" && !script.code.is_empty() && !script.code.starts_with("external:") {
                // Execute the script to load the functions
                match self.lua.load(&script.code).exec() {
                    Ok(()) => {
                        // Remove script load logs
                    }
                    Err(e) => {
                        tracing::error!("Failed to load Lua script '{}': {}", script.name, e);
                    }
                }
            }
        }
        
        Ok(())
    }
    
    pub fn setup_bridge_functions(&mut self, elements: &HashMap<ElementId, Element>, krb_file: &KRBFile) -> Result<()> {
        let globals = self.lua.globals();
        
        // Helper function to find style ID by name
        let _find_style_id = |style_name: &str| -> Option<u8> {
            krb_file.styles.iter()
                .find(|(_, style)| style.name == style_name)
                .map(|(style_id, _)| *style_id)
        };
        
        // Create a table to store element IDs by their IDs
        let element_ids_table = self.lua.create_table()?;
        for (element_id, element) in elements {
            // Use the element's actual ID string if available, otherwise use numeric ID
            let id_str = if element.id.is_empty() {
                format!("element_{}", element_id)
            } else {
                element.id.clone()
            };
            element_ids_table.set(id_str, *element_id)?;
        }
        globals.set("_element_ids", element_ids_table)?;
        
        // Create a table to store style name to ID mappings
        let style_ids_table = self.lua.create_table()?;
        for (style_id, style) in &krb_file.styles {
            style_ids_table.set(style.name.clone(), *style_id)?;
        }
        globals.set("_style_ids", style_ids_table)?;
        
        // Add bridge functions
        let lua_code = r#"
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
                                print("Queuing style change for element " .. self.id .. " to style " .. style_name .. " (ID: " .. style_id .. ")")
                            else
                                print("Error: Style '" .. style_name .. "' not found")
                            end
                        end,
                        setChecked = function(self, checked)
                            _pending_state_changes[self.numeric_id] = checked
                            print("Queuing checked state change for element " .. self.id .. " to " .. tostring(checked))
                        end,
                        setText = function(self, text)
                            _pending_text_changes[self.numeric_id] = text
                            print("Queuing text change for element " .. self.id .. " to: " .. text)
                        end,
                        setVisible = function(self, visible)
                            _pending_visibility_changes[self.numeric_id] = visible
                            print("Queuing visibility change for element " .. self.id .. " to " .. tostring(visible))
                        end,
                        -- DOM traversal properties
                        getParent = function(self)
                            return _get_parent_element(self.numeric_id)
                        end,
                        getChildren = function(self)
                            return _get_children_elements(self.numeric_id)
                        end,
                        getFirstChild = function(self)
                            local children = _get_children_elements(self.numeric_id)
                            return children[1] or nil
                        end,
                        getLastChild = function(self)
                            local children = _get_children_elements(self.numeric_id)
                            return children[#children] or nil
                        end,
                        getNextSibling = function(self)
                            return _get_next_sibling(self.numeric_id)
                        end,
                        getPreviousSibling = function(self)
                            return _get_previous_sibling(self.numeric_id)
                        end
                    }
                else
                    print("Error: Element '" .. element_id .. "' not found")
                    return nil
                end
            end
            
            -- DOM query functions
            function getElementsByTag(tag_name)
                return _get_elements_by_tag(tag_name)
            end
            
            function getElementsByClass(class_name)
                return _get_elements_by_class(class_name)
            end
            
            function querySelectorAll(selector)
                return _query_selector_all(selector)
            end
            
            function querySelector(selector)
                local results = _query_selector_all(selector)
                return results[1] or nil
            end
            
            -- Get root element
            function getRootElement()
                return _get_root_element()
            end
            
            -- Alias for backward compatibility
            get_element = getElementById
            
            -- Function to get pending style changes
            function _get_pending_style_changes()
                local changes = _pending_style_changes
                _pending_style_changes = {}
                return changes
            end
            
            -- Function to get pending state changes
            function _get_pending_state_changes()
                local changes = _pending_state_changes
                _pending_state_changes = {}
                return changes
            end
            
            -- Function to get pending text changes
            function _get_pending_text_changes()
                local changes = _pending_text_changes
                _pending_text_changes = {}
                return changes
            end
            
            -- Function to get pending visibility changes
            function _get_pending_visibility_changes()
                local changes = _pending_visibility_changes
                _pending_visibility_changes = {}
                return changes
            end
            
            -- Template variable management (legacy functions for backward compatibility)
            _pending_template_variable_changes = {}
            
            -- Function to set a template variable (legacy - use direct variable assignment instead)
            function setTemplateVariable(name, value)
                -- Use the reactive system if available, otherwise fall back to pending changes
                if _G[name] and type(_G[name]) == "table" and getmetatable(_G[name]) then
                    -- This is a reactive variable
                    _G[name].value = value
                    print("Using reactive variable: " .. name .. " = " .. tostring(value))
                else
                    -- Fall back to legacy system
                    _pending_template_variable_changes[name] = tostring(value)
                    print("Queuing template variable change (legacy): " .. name .. " = " .. tostring(value))
                end
            end
            
            -- Function to get a template variable (legacy - use direct variable access instead)
            function getTemplateVariable(name)
                -- Use the reactive system if available, otherwise fall back to template variables table
                if _G[name] and type(_G[name]) == "table" and getmetatable(_G[name]) then
                    -- This is a reactive variable
                    return tostring(_G[name])
                else
                    -- Fall back to legacy system
                    return _template_variables[name] or nil
                end
            end
            
            -- Function to get all template variables (legacy)
            function getTemplateVariables()
                return _template_variables
            end
            
            -- Function to get pending template variable changes (legacy)
            function _get_pending_template_variable_changes()
                local changes = _pending_template_variable_changes
                _pending_template_variable_changes = {}
                return changes
            end
        "#;
        
        self.lua.load(lua_code).exec()?;
        
        Ok(())
    }
    
    pub fn register_dom_functions(&mut self, elements: &HashMap<ElementId, Element>, krb_file: &KRBFile) -> Result<()> {
        let globals = self.lua.globals();
        
        // Store elements data in Lua global tables for function access
        let elements_table = self.lua.create_table()?;
        for (element_id, element) in elements {
            let element_data = self.lua.create_table()?;
            element_data.set("id", element.id.clone())?;
            element_data.set("element_type", format!("{:?}", element.element_type))?;
            element_data.set("visible", element.visible)?;
            element_data.set("text", element.text.clone())?;
            element_data.set("style_id", element.style_id)?;
            
            // Store parent/children relationships
            if let Some(parent_id) = element.parent {
                element_data.set("parent_id", parent_id)?;
            }
            
            let children_table = self.lua.create_table()?;
            for (i, child_id) in element.children.iter().enumerate() {
                children_table.set(i + 1, *child_id)?;
            }
            element_data.set("children", children_table)?;
            
            elements_table.set(*element_id, element_data)?;
        }
        globals.set("_elements_data", elements_table)?;
        
        // Store styles data
        let styles_table = self.lua.create_table()?;
        for (style_id, style) in &krb_file.styles {
            let style_data = self.lua.create_table()?;
            style_data.set("name", style.name.clone())?;
            styles_table.set(*style_id, style_data)?;
        }
        globals.set("_styles_data", styles_table)?;
        
        // Add DOM functions in Lua
        let dom_functions = r#"
            -- Helper function to create element proxy
            function _create_element_proxy(element_id)
                local element_data = _elements_data[element_id]
                if not element_data then return nil end
                
                return {
                    id = element_data.id,
                    numeric_id = element_id,
                    element_type = element_data.element_type,
                    visible = element_data.visible,
                    text = element_data.text
                }
            end
            
            -- DOM traversal functions
            function _get_parent_element(element_id)
                local element_data = _elements_data[element_id]
                if element_data and element_data.parent_id then
                    return _create_element_proxy(element_data.parent_id)
                end
                return nil
            end
            
            function _get_children_elements(element_id)
                local element_data = _elements_data[element_id]
                local result = {}
                if element_data and element_data.children then
                    for i, child_id in ipairs(element_data.children) do
                        local child_proxy = _create_element_proxy(child_id)
                        if child_proxy then
                            table.insert(result, child_proxy)
                        end
                    end
                end
                return result
            end
            
            function _get_next_sibling(element_id)
                local element_data = _elements_data[element_id]
                if element_data and element_data.parent_id then
                    local parent_data = _elements_data[element_data.parent_id]
                    if parent_data and parent_data.children then
                        for i, child_id in ipairs(parent_data.children) do
                            if child_id == element_id and i < #parent_data.children then
                                return _create_element_proxy(parent_data.children[i + 1])
                            end
                        end
                    end
                end
                return nil
            end
            
            function _get_previous_sibling(element_id)
                local element_data = _elements_data[element_id]
                if element_data and element_data.parent_id then
                    local parent_data = _elements_data[element_data.parent_id]
                    if parent_data and parent_data.children then
                        for i, child_id in ipairs(parent_data.children) do
                            if child_id == element_id and i > 1 then
                                return _create_element_proxy(parent_data.children[i - 1])
                            end
                        end
                    end
                end
                return nil
            end
            
            function _get_elements_by_tag(tag_name)
                local result = {}
                for element_id, element_data in pairs(_elements_data) do
                    if element_data.element_type:lower() == tag_name:lower() then
                        local proxy = _create_element_proxy(element_id)
                        if proxy then
                            table.insert(result, proxy)
                        end
                    end
                end
                return result
            end
            
            function _get_elements_by_class(class_name)
                local result = {}
                for element_id, element_data in pairs(_elements_data) do
                    local style_data = _styles_data[element_data.style_id]
                    if style_data and style_data.name == class_name then
                        local proxy = _create_element_proxy(element_id)
                        if proxy then
                            table.insert(result, proxy)
                        end
                    end
                end
                return result
            end
            
            function _query_selector_all(selector)
                local result = {}
                
                if selector:sub(1, 1) == '#' then
                    -- ID selector
                    local id = selector:sub(2)
                    for element_id, element_data in pairs(_elements_data) do
                        if element_data.id == id then
                            local proxy = _create_element_proxy(element_id)
                            if proxy then
                                table.insert(result, proxy)
                            end
                        end
                    end
                elseif selector:sub(1, 1) == '.' then
                    -- Class selector
                    local class_name = selector:sub(2)
                    return _get_elements_by_class(class_name)
                else
                    -- Tag selector
                    return _get_elements_by_tag(selector)
                end
                
                return result
            end
            
            function _get_root_element()
                for element_id, element_data in pairs(_elements_data) do
                    if not element_data.parent_id then
                        return _create_element_proxy(element_id)
                    end
                end
                return nil
            end
        "#;
        
        self.lua.load(dom_functions).exec()?;
        
        Ok(())
    }
    
    pub fn update(&mut self, _delta_time: Duration, _elements: &mut HashMap<ElementId, Element>) -> Result<()> {
        // Update script systems
        // For now, this is a placeholder
        Ok(())
    }
    
    pub fn call_function(&mut self, function_name: &str, _args: Vec<PropertyValue>) -> Result<()> {
        // Find the script that contains this function
        for script in &self.scripts {
            if script.entry_points.contains(&function_name.to_string()) {
                // Execute the Lua function
                if script.language == "lua" {
                    match self.lua.globals().get::<_, mlua::Function>(function_name) {
                        Ok(lua_function) => {
                            match lua_function.call::<_, ()>(()) {
                                Ok(()) => {
                                    // Remove debug logs
                                }
                                Err(e) => {
                                    tracing::error!("Error executing Lua function '{}': {}", function_name, e);
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!("Lua function '{}' not found: {}", function_name, e);
                        }
                    }
                }
                break;
            }
        }
        
        Ok(())
    }
    
    pub fn apply_pending_style_changes(&mut self, elements: &mut HashMap<ElementId, Element>) -> Result<bool> {
        // Get pending style changes from Lua
        let get_changes_fn: mlua::Function = self.lua.globals().get("_get_pending_style_changes")?;
        let changes_table: mlua::Table = get_changes_fn.call(())?;
        
        let mut changes_applied = false;
        
        // Iterate through the changes table
        for pair in changes_table.pairs::<u32, u8>() {
            let (element_numeric_id, new_style_id) = pair?;
            let element_id: ElementId = element_numeric_id;
            
            if let Some(element) = elements.get_mut(&element_id) {
                tracing::info!("Applying style change: element {} -> style_id {}", element_numeric_id, new_style_id);
                element.style_id = new_style_id;
                changes_applied = true;
            } else {
                tracing::warn!("Could not find element {} to apply style change", element_numeric_id);
            }
        }
        
        Ok(changes_applied)
    }
    
    pub fn apply_pending_state_changes(&mut self, elements: &mut HashMap<ElementId, Element>) -> Result<bool> {
        use kryon_core::InteractionState;
        
        // Get pending state changes from Lua
        let get_changes_fn: mlua::Function = self.lua.globals().get("_get_pending_state_changes")?;
        let changes_table: mlua::Table = get_changes_fn.call(())?;
        
        let mut changes_applied = false;
        
        // Iterate through the changes table
        for pair in changes_table.pairs::<u32, bool>() {
            let (element_numeric_id, checked) = pair?;
            let element_id: ElementId = element_numeric_id;
            
            if let Some(element) = elements.get_mut(&element_id) {
                tracing::info!("Applying state change: element {} -> checked {}", element_numeric_id, checked);
                
                // Update the element's current state to include/exclude Checked flag
                if checked {
                    // Add Checked to the current state (bitwise OR)
                    element.current_state = InteractionState::Checked;
                } else {
                    // Remove Checked from the current state - for now just set to Normal
                    element.current_state = InteractionState::Normal;
                }
                changes_applied = true;
            } else {
                tracing::warn!("Could not find element {} to apply state change", element_numeric_id);
            }
        }
        
        Ok(changes_applied)
    }
    
    pub fn apply_pending_text_changes(&mut self, elements: &mut HashMap<ElementId, Element>) -> Result<bool> {
        // Get pending text changes from Lua
        let get_changes_fn: mlua::Function = self.lua.globals().get("_get_pending_text_changes")?;
        let changes_table: mlua::Table = get_changes_fn.call(())?;
        
        let mut changes_applied = false;
        
        // Iterate through the changes table
        for pair in changes_table.pairs::<u32, String>() {
            let (element_numeric_id, new_text) = pair?;
            let element_id: ElementId = element_numeric_id;
            
            if let Some(element) = elements.get_mut(&element_id) {
                tracing::info!("Applying text change: element {} -> text '{}'", element_numeric_id, new_text);
                element.text = new_text;
                changes_applied = true;
            } else {
                tracing::warn!("Could not find element {} to apply text change", element_numeric_id);
            }
        }
        
        Ok(changes_applied)
    }
    
    pub fn apply_pending_visibility_changes(&mut self, elements: &mut HashMap<ElementId, Element>) -> Result<bool> {
        // Get pending visibility changes from Lua
        let get_changes_fn: mlua::Function = self.lua.globals().get("_get_pending_visibility_changes")?;
        let changes_table: mlua::Table = get_changes_fn.call(())?;
        
        let mut changes_applied = false;
        
        // Iterate through the changes table
        for pair in changes_table.pairs::<u32, bool>() {
            let (element_numeric_id, visible) = pair?;
            let element_id: ElementId = element_numeric_id;
            
            if let Some(element) = elements.get_mut(&element_id) {
                tracing::info!("Applying visibility change: element {} -> visible {}", element_numeric_id, visible);
                element.visible = visible;
                changes_applied = true;
            } else {
                tracing::warn!("Could not find element {} to apply visibility change", element_numeric_id);
            }
        }
        
        Ok(changes_applied)
    }

    pub fn set_state(&mut self, key: String, value: PropertyValue) {
        self.state.insert(key, value);
    }
    
    pub fn get_state(&self, key: &str) -> Option<&PropertyValue> {
        self.state.get(key)
    }
    
    /// Initialize template variables in the Lua context as reactive variables
    pub fn initialize_template_variables(&mut self, variables: &HashMap<String, String>) -> Result<()> {
        let globals = self.lua.globals();
        
        // Create the _template_variables table (internal storage)
        let template_vars_table = self.lua.create_table()?;
        for (name, value) in variables {
            template_vars_table.set(name.clone(), value.clone())?;
        }
        globals.set("_template_variables", template_vars_table)?;
        
        // Create reactive variables using metamethods
        let reactive_setup_code = r#"
            -- Initialize reactive template variables
            _template_variable_changes = {}
            
            -- Create reactive variable proxy
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
                    end,
                    __tostring = function() return value end,
                    __eq = function(other) return value == tostring(other) end,
                    __lt = function(other) return tonumber(value) and tonumber(other) and tonumber(value) < tonumber(other) end,
                    __le = function(other) return tonumber(value) and tonumber(other) and tonumber(value) <= tonumber(other) end,
                    __add = function(other) return (tonumber(value) or 0) + (tonumber(other) or 0) end,
                    __sub = function(other) return (tonumber(value) or 0) - (tonumber(other) or 0) end,
                    __mul = function(other) return (tonumber(value) or 0) * (tonumber(other) or 0) end,
                    __div = function(other) return (tonumber(value) or 0) / (tonumber(other) or 0) end,
                    __concat = function(other) return value .. tostring(other) end
                }
            end
            
            -- Store all reactive variables in a central registry
            _reactive_variables = {}
            
            -- Create reactive variables for each template variable
            for name, value in pairs(_template_variables) do
                local reactive_var = create_reactive_variable(name, value)
                _reactive_variables[name] = reactive_var
                
                -- DO NOT set _G[name] directly - let the metamethod handle all access
            end
            
            
            -- Set up global metamethod to intercept all global variable access
            local original_g_mt = getmetatable(_G) or {}
            
            setmetatable(_G, {
                __index = function(t, k)
                    -- Check if this is a reactive variable
                    if _reactive_variables[k] then
                        return _reactive_variables[k].get()
                    end
                    -- Use original index behavior
                    if original_g_mt.__index then
                        return original_g_mt.__index(t, k)
                    else
                        return rawget(t, k)
                    end
                end,
                __newindex = function(t, k, v)
                    -- Check if this is a reactive variable
                    if _reactive_variables[k] then
                        -- This is a reactive variable, use the reactive setter
                        _reactive_variables[k].set(v)
                        -- DO NOT use rawset for reactive variables - let all access go through metamethods
                    else
                        -- Not a reactive variable, use normal assignment
                        if original_g_mt.__newindex then
                            original_g_mt.__newindex(t, k, v)
                        else
                            rawset(t, k, v)
                        end
                    end
                end
            })
            
            -- Function to get pending template variable changes
            function _get_reactive_template_variable_changes()
                local changes = _template_variable_changes
                _template_variable_changes = {}
                return changes
            end
        "#;
        
        self.lua.load(reactive_setup_code).exec()?;
        Ok(())
    }
    
    /// Apply pending template variable changes and return a map of changes
    pub fn apply_pending_template_variable_changes(&mut self) -> Result<HashMap<String, String>> {
        let globals = self.lua.globals();
        let mut changes = HashMap::new();
        
        // First, try to get reactive changes (new system)
        if let Ok(get_reactive_func) = globals.get::<_, mlua::Function>("_get_reactive_template_variable_changes") {
            let reactive_changes: mlua::Table = get_reactive_func.call(())?;
            
            // Iterate through the reactive changes table
            for pair in reactive_changes.pairs::<String, String>() {
                let (name, value) = pair?;
                changes.insert(name, value);
            }
        }
        
        // Then, get legacy changes (backward compatibility)
        if let Ok(get_legacy_func) = globals.get::<_, mlua::Function>("_get_pending_template_variable_changes") {
            let legacy_changes: mlua::Table = get_legacy_func.call(())?;
            
            // Collect changes first
            let mut legacy_change_pairs = Vec::new();
            for pair in legacy_changes.pairs::<String, String>() {
                let (name, value) = pair?;
                legacy_change_pairs.push((name, value));
            }
            
            // Add to changes map
            for (name, value) in &legacy_change_pairs {
                changes.insert(name.clone(), value.clone());
            }
            
            // Update the _template_variables table with legacy changes
            if !legacy_change_pairs.is_empty() {
                let template_vars_table: mlua::Table = globals.get("_template_variables")?;
                for (name, value) in &legacy_change_pairs {
                    template_vars_table.set(name.clone(), value.clone())?;
                }
            }
        }
        
        // Remove log for applied changes count
        
        Ok(changes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_reactive_template_variables() {
        let mut script_system = ScriptSystem::new();
        
        // Create test template variables
        let mut template_variables = HashMap::new();
        template_variables.insert("counter".to_string(), "0".to_string());
        template_variables.insert("message".to_string(), "Hello".to_string());
        template_variables.insert("enabled".to_string(), "true".to_string());
        
        // Initialize reactive template variables
        script_system.initialize_template_variables(&template_variables).unwrap();
        
        // Test script that uses reactive variables
        let test_script = r#"
            -- Test direct variable access
            assert(tostring(counter) == "0", "Initial counter should be 0")
            assert(tostring(message) == "Hello", "Initial message should be Hello")
            assert(tostring(enabled) == "true", "Initial enabled should be true")
            
            -- Test direct assignment (should trigger reactive updates)
            counter = 5
            message = "World"
            enabled = false
            
            -- Test values after assignment
            assert(tostring(counter) == "5", "Counter should be 5 after assignment")
            assert(tostring(message) == "World", "Message should be World after assignment")
            assert(tostring(enabled) == "false", "Enabled should be false after assignment")
            
            -- Test arithmetic operations
            counter = counter + 10
            assert(tostring(counter) == "15", "Counter should be 15 after addition")
            
            -- Test string concatenation
            message = message .. "!"
            assert(tostring(message) == "World!", "Message should be World! after concatenation")
            
            -- Test legacy functions (backward compatibility)
            assert(getTemplateVariable("counter") == "15", "Legacy getter should work")
            setTemplateVariable("counter", 100)
            assert(tostring(counter) == "100", "Legacy setter should work")
            
            print("All reactive variable tests passed!")
        "#;
        
        // Execute the test script
        script_system.lua.load(test_script).exec().unwrap();
        
        // Check for pending changes
        let changes = script_system.apply_pending_template_variable_changes().unwrap();
        assert!(!changes.is_empty(), "Should have detected changes");
        
        // Verify specific changes
        assert_eq!(changes.get("counter"), Some(&"100".to_string()));
        assert_eq!(changes.get("message"), Some(&"World!".to_string()));
        assert_eq!(changes.get("enabled"), Some(&"false".to_string()));
    }
    
    #[test]
    fn test_reactive_variable_operations() {
        let mut script_system = ScriptSystem::new();
        
        // Create test template variables
        let mut template_variables = HashMap::new();
        template_variables.insert("num1".to_string(), "10".to_string());
        template_variables.insert("num2".to_string(), "5".to_string());
        template_variables.insert("text".to_string(), "Hello".to_string());
        
        // Initialize reactive template variables
        script_system.initialize_template_variables(&template_variables).unwrap();
        
        // Test script for mathematical and string operations
        let test_script = r#"
            -- Test mathematical operations
            local sum = num1 + num2
            assert(sum == 15, "Addition should work: " .. sum)
            
            local diff = num1 - num2
            assert(diff == 5, "Subtraction should work: " .. diff)
            
            local product = num1 * num2
            assert(product == 50, "Multiplication should work: " .. product)
            
            local quotient = num1 / num2
            assert(quotient == 2, "Division should work: " .. quotient)
            
            -- Test comparison operations
            assert(num1 > num2, "Comparison should work")
            assert(num2 < num1, "Reverse comparison should work")
            assert(num1 >= num2, "Greater than or equal should work")
            assert(num2 <= num1, "Less than or equal should work")
            
            -- Test string concatenation
            local greeting = text .. " World"
            assert(greeting == "Hello World", "String concatenation should work")
            
            -- Test equality comparison
            assert(text == "Hello", "String equality should work")
            
            -- Test modification through operations
            num1 = num1 + 5
            assert(tostring(num1) == "15", "Modified num1 should be 15")
            
            text = text .. " World"
            assert(tostring(text) == "Hello World", "Modified text should be Hello World")
            
            print("All reactive variable operations tests passed!")
        "#;
        
        // Execute the test script
        script_system.lua.load(test_script).exec().unwrap();
        
        // Check for pending changes
        let changes = script_system.apply_pending_template_variable_changes().unwrap();
        assert!(!changes.is_empty(), "Should have detected changes from operations");
        
        // Verify specific changes
        assert_eq!(changes.get("num1"), Some(&"15".to_string()));
        assert_eq!(changes.get("text"), Some(&"Hello World".to_string()));
    }
}
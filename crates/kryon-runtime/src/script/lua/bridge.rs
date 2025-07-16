//! Lua DOM API bridge implementation
//!
//! This module provides the bridge between Lua scripts and the Kryon UI system,
//! enabling scripts to manipulate DOM elements, handle events, and manage state.
//! It includes the complete DOM API implementation from the original lua_bridge.lua.

use std::collections::HashMap;
use std::rc::Rc;
use anyhow::Result;
use mlua::{Lua, Table as LuaTable, Function as LuaFunction};
use crate::script::{
    engine_trait::{BridgeData, ChangeSet, ScriptValue},
    error::ScriptError,
};

/// Lua DOM API bridge
/// 
/// This component provides:
/// - DOM element manipulation API
/// - Event handling system
/// - Component property access
/// - Pending change management
pub struct LuaBridge {
    /// Reference to the Lua VM
    lua: Rc<Lua>,
}

impl LuaBridge {
    /// Create a new Lua bridge
    pub fn new(lua: Rc<Lua>) -> Result<Self> {
        let bridge = Self {
            lua,
        };
        
        // Load the bridge API into Lua
        bridge.setup_bridge_api()?;
        
        Ok(bridge)
    }
    
    /// Setup the bridge with element and style data
    pub fn setup(&mut self, bridge_data: &BridgeData) -> Result<()> {
        let globals = self.lua.globals();
        
        // Create element IDs table
        let element_ids_table = self.lua.create_table()?;
        for (element_id, numeric_id) in &bridge_data.element_ids {
            element_ids_table.set(element_id.clone(), *numeric_id)?;
        }
        globals.set("_element_ids", element_ids_table)?;
        
        // Create style IDs table
        let style_ids_table = self.lua.create_table()?;
        for (style_name, style_id) in &bridge_data.style_ids {
            style_ids_table.set(style_name.clone(), *style_id)?;
        }
        globals.set("_style_ids", style_ids_table)?;
        
        // Create component properties table
        let component_properties_table = self.lua.create_table()?;
        for (element_id, properties) in &bridge_data.component_properties {
            let props_table = self.lua.create_table()?;
            for (prop_name, prop_value) in properties {
                self.set_script_value_in_table(&props_table, prop_name, prop_value.clone())?;
            }
            component_properties_table.set(element_id.clone(), props_table)?;
        }
        globals.set("_component_properties", component_properties_table)?;
        
        // Create elements data table
        let elements_table = self.lua.create_table()?;
        for (element_id, element) in &bridge_data.elements_data {
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
        
        tracing::debug!("Lua bridge setup completed with {} elements and {} styles", 
                       bridge_data.elements_data.len(), bridge_data.style_ids.len());
        
        Ok(())
    }
    
    /// Execute onReady callbacks
    pub fn execute_on_ready_callbacks(&mut self) -> Result<()> {
        let execute_ready_code = r#"
            -- Mark the document as ready
            _mark_ready()
            
            if _ready_callbacks then
                for i, callback in ipairs(_ready_callbacks) do
                    local success, error = pcall(callback)
                    if not success then
                        print("Error in onReady callback " .. i .. ": " .. tostring(error))
                    end
                end
                print("Executed " .. #_ready_callbacks .. " onReady callbacks")
                _ready_callbacks = {}
            end
        "#;
        
        self.lua.load(execute_ready_code).exec().map_err(|e| {
            ScriptError::ExecutionFailed {
                function: "execute_on_ready_callbacks".to_string(),
                error: e.to_string(),
                context: "Executing onReady callbacks".to_string(),
            }
        })?;
        
        Ok(())
    }
    
    /// Get pending changes from the bridge
    pub fn get_pending_changes(&mut self) -> Result<HashMap<String, ChangeSet>> {
        let mut changes = HashMap::new();
        
        // Get style changes
        if let Ok(get_changes_fn) = self.lua.globals().get::<_, LuaFunction>("_get_pending_style_changes") {
            if let Ok(changes_table) = get_changes_fn.call::<_, LuaTable>(()) {
                let mut style_changes = HashMap::new();
                for pair in changes_table.pairs::<u32, u8>() {
                    if let Ok((element_id, style_id)) = pair {
                        style_changes.insert(element_id.to_string(), style_id.to_string());
                    }
                }
                if !style_changes.is_empty() {
                    changes.insert("style_changes".to_string(), ChangeSet {
                        change_type: "style_changes".to_string(),
                        data: style_changes,
                    });
                }
            }
        }
        
        // Get text changes
        if let Ok(get_changes_fn) = self.lua.globals().get::<_, LuaFunction>("_get_pending_text_changes") {
            if let Ok(changes_table) = get_changes_fn.call::<_, LuaTable>(()) {
                let mut text_changes = HashMap::new();
                for pair in changes_table.pairs::<u32, String>() {
                    if let Ok((element_id, text)) = pair {
                        text_changes.insert(element_id.to_string(), text);
                    }
                }
                if !text_changes.is_empty() {
                    changes.insert("text_changes".to_string(), ChangeSet {
                        change_type: "text_changes".to_string(),
                        data: text_changes,
                    });
                }
            }
        }
        
        // Get state changes
        if let Ok(get_changes_fn) = self.lua.globals().get::<_, LuaFunction>("_get_pending_state_changes") {
            if let Ok(changes_table) = get_changes_fn.call::<_, LuaTable>(()) {
                let mut state_changes = HashMap::new();
                for pair in changes_table.pairs::<u32, bool>() {
                    if let Ok((element_id, checked)) = pair {
                        state_changes.insert(element_id.to_string(), checked.to_string());
                    }
                }
                if !state_changes.is_empty() {
                    changes.insert("state_changes".to_string(), ChangeSet {
                        change_type: "state_changes".to_string(),
                        data: state_changes,
                    });
                }
            }
        }
        
        // Get visibility changes
        if let Ok(get_changes_fn) = self.lua.globals().get::<_, LuaFunction>("_get_pending_visibility_changes") {
            if let Ok(changes_table) = get_changes_fn.call::<_, LuaTable>(()) {
                let mut visibility_changes = HashMap::new();
                for pair in changes_table.pairs::<u32, bool>() {
                    if let Ok((element_id, visible)) = pair {
                        visibility_changes.insert(element_id.to_string(), visible.to_string());
                    }
                }
                if !visibility_changes.is_empty() {
                    changes.insert("visibility_changes".to_string(), ChangeSet {
                        change_type: "visibility_changes".to_string(),
                        data: visibility_changes,
                    });
                }
            }
        }
        
        Ok(changes)
    }
    
    /// Clear pending changes from the bridge
    pub fn clear_pending_changes(&mut self) -> Result<()> {
        // Clear the DOM API changes by calling the Lua clear function
        if let Ok(clear_fn) = self.lua.globals().get::<_, LuaFunction>("_clear_dom_changes") {
            clear_fn.call::<_, ()>(()).map_err(|e| {
                ScriptError::BridgeSetupFailed {
                    error: e.to_string(),
                    context: "Clearing DOM changes".to_string(),
                }
            })?;
        }
        
        Ok(())
    }
    
    /// Setup the complete bridge API in Lua
    fn setup_bridge_api(&self) -> Result<()> {
        // Load the embedded bridge code
        const LUA_BRIDGE_CODE: &str = include_str!("../../lua_bridge.lua");
        
        self.lua.load(LUA_BRIDGE_CODE).exec().map_err(|e| {
            ScriptError::BridgeSetupFailed {
                error: e.to_string(),
                context: "Loading bridge API code".to_string(),
            }
        })?;
        
        tracing::debug!("Lua bridge API loaded successfully");
        Ok(())
    }
    
    /// Helper to set a ScriptValue in a Lua table
    fn set_script_value_in_table(&self, table: &LuaTable, key: &str, value: ScriptValue) -> Result<()> {
        match value {
            ScriptValue::Nil => table.set(key, mlua::Value::Nil)?,
            ScriptValue::Boolean(b) => table.set(key, b)?,
            ScriptValue::Integer(i) => table.set(key, i)?,
            ScriptValue::Number(f) => table.set(key, f)?,
            ScriptValue::String(s) => table.set(key, s)?,
            ScriptValue::Array(arr) => {
                let lua_table = self.lua.create_table()?;
                for (i, item) in arr.into_iter().enumerate() {
                    self.set_script_value_in_table(&lua_table, &(i + 1).to_string(), item)?;
                }
                table.set(key, lua_table)?;
            },
            ScriptValue::Object(obj) => {
                let lua_table = self.lua.create_table()?;
                for (obj_key, obj_value) in obj {
                    self.set_script_value_in_table(&lua_table, &obj_key, obj_value)?;
                }
                table.set(key, lua_table)?;
            },
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kryon_core::{Element, ElementType, InteractionState};
    use glam::Vec4;
    
    #[test]
    fn test_lua_bridge_creation() {
        let lua = Lua::new();
        let bridge = LuaBridge::new(&lua);
        assert!(bridge.is_ok());
    }
    
    #[test]
    fn test_bridge_api_loading() {
        let lua = Lua::new();
        let bridge = LuaBridge::new(&lua).unwrap();
        
        // Check that bridge API functions are available
        let globals = lua.globals();
        assert!(globals.get::<_, LuaFunction>("getElementById").is_ok());
        assert!(globals.get::<_, LuaFunction>("addEventListener").is_ok());
        assert!(globals.get::<_, LuaFunction>("onReady").is_ok());
    }
    
    #[test]
    fn test_bridge_setup() {
        let lua = Lua::new();
        let mut bridge = LuaBridge::new(&lua).unwrap();
        
        // Create test bridge data
        let mut element_ids = HashMap::new();
        element_ids.insert("test_button".to_string(), 1);
        
        let mut style_ids = HashMap::new();
        style_ids.insert("button_style".to_string(), 10);
        
        let mut elements_data = HashMap::new();
        elements_data.insert(1, Element {
            id: "test_button".to_string(),
            element_type: ElementType::Button,
            visible: true,
            text: "Click me".to_string(),
            style_id: 10,
            current_state: InteractionState::Normal,
            parent: None,
            children: vec![],
            custom_properties: HashMap::new(),
            calculated_rect: Default::default(),
            content_rect: Default::default(),
            transform: Default::default(),
            opacity: 1.0,
            background_color: Vec4::new(1.0, 1.0, 1.0, 1.0),
            border_color: Vec4::new(0.0, 0.0, 0.0, 1.0),
            border_width: 1.0,
            border_radius: 0.0,
        });
        
        let bridge_data = BridgeData {
            element_ids,
            style_ids,
            component_properties: HashMap::new(),
            elements_data,
            template_variables: HashMap::new(),
        };
        
        let result = bridge.setup(&bridge_data);
        assert!(result.is_ok());
        
        // Verify that data was set up correctly
        let globals = lua.globals();
        let element_ids_table: LuaTable = globals.get("_element_ids").unwrap();
        let test_button_id: u32 = element_ids_table.get("test_button").unwrap();
        assert_eq!(test_button_id, 1);
    }
    
    #[test]
    fn test_pending_changes() {
        let lua = Lua::new();
        let mut bridge = LuaBridge::new(&lua).unwrap();
        
        // Simulate some pending changes
        let setup_changes = r#"
            _pending_style_changes[1] = 5
            _pending_text_changes[2] = "New text"
            _pending_visibility_changes[3] = false
        "#;
        
        lua.load(setup_changes).exec().unwrap();
        
        // Get pending changes
        let changes = bridge.get_pending_changes().unwrap();
        
        // Verify changes were captured
        assert!(changes.contains_key("style_changes"));
        assert!(changes.contains_key("text_changes"));
        assert!(changes.contains_key("visibility_changes"));
        
        let style_changes = &changes["style_changes"];
        assert!(style_changes.data.contains_key("1"));
        assert_eq!(style_changes.data["1"], "5");
    }
}

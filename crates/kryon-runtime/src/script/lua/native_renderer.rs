//! Native renderer Lua API bridge
//!
//! This module provides direct access to native rendering APIs from Lua scripts.
//! It allows NativeRendererView elements to execute backend-specific rendering code.

use std::rc::Rc;
use anyhow::Result;
use mlua::{Lua, Table as LuaTable, Function as LuaFunction, Value as LuaValue};
use glam::Vec2;
use crate::script::error::ScriptError;
use kryon_core::ElementId;

/// Native renderer context that provides access to backend-specific APIs
pub struct NativeRendererContext {
    /// Reference to the Lua VM
    lua: Rc<Lua>,
    /// Active backend type (e.g., "raylib", "wgpu")
    backend: String,
    /// Element bounds for rendering constraints
    element_bounds: (Vec2, Vec2), // (position, size)
    /// Element ID for reference
    element_id: ElementId,
}

impl NativeRendererContext {
    /// Create a new native renderer context
    pub fn new(lua: Rc<Lua>, backend: String, element_id: ElementId, position: Vec2, size: Vec2) -> Result<Self> {
        let context = Self {
            lua,
            backend,
            element_bounds: (position, size),
            element_id,
        };
        
        // Setup the native API based on backend
        context.setup_native_api()?;
        
        Ok(context)
    }
    
    /// Setup the native API for the specific backend
    fn setup_native_api(&self) -> Result<()> {
        let globals = self.lua.globals();
        
        match self.backend.as_str() {
            "raylib" => {
                self.setup_raylib_api(&globals)?;
            }
            "wgpu" => {
                self.setup_wgpu_api(&globals)?;
            }
            _ => {
                return Err(ScriptError::NativeRendererError {
                    backend: self.backend.clone(),
                    error: "Unsupported backend".to_string(),
                }.into());
            }
        }
        
        Ok(())
    }
    
    /// Setup Raylib-specific API
    fn setup_raylib_api(&self, globals: &LuaTable) -> Result<()> {
        let (position, size) = self.element_bounds;
        
        // Create a Raylib context object
        let raylib_ctx = self.lua.create_table()?;
        
        // Add utility functions
        raylib_ctx.set("GetElementBounds", self.lua.create_function(move |_, ()| {
            Ok((position.x, position.y, size.x, size.y))
        })?)?;
        
        raylib_ctx.set("GetElementPosition", self.lua.create_function(move |_, ()| {
            Ok((position.x, position.y))
        })?)?;
        
        raylib_ctx.set("GetElementSize", self.lua.create_function(move |_, ()| {
            Ok((size.x, size.y))
        })?)?;
        
        // Add Vector3 constructor (returns a table)
        raylib_ctx.set("Vector3", self.lua.create_function(|lua, (x, y, z): (f32, f32, f32)| {
            let table = lua.create_table()?;
            table.set("x", x)?;
            table.set("y", y)?;
            table.set("z", z)?;
            Ok(table)
        })?)?;
        
        // Add common Raylib colors (as tables)
        let colors = self.lua.create_table()?;
        let create_color = |r: u8, g: u8, b: u8, a: u8| -> Result<LuaTable> {
            let color = self.lua.create_table()?;
            color.set("r", r)?;
            color.set("g", g)?;
            color.set("b", b)?;
            color.set("a", a)?;
            Ok(color)
        };
        
        colors.set("RAYWHITE", create_color(245, 245, 245, 255)?)?;
        colors.set("BLACK", create_color(0, 0, 0, 255)?)?;
        colors.set("WHITE", create_color(255, 255, 255, 255)?)?;
        colors.set("RED", create_color(230, 41, 55, 255)?)?;
        colors.set("GREEN", create_color(0, 228, 48, 255)?)?;
        colors.set("BLUE", create_color(0, 121, 241, 255)?)?;
        colors.set("YELLOW", create_color(255, 161, 0, 255)?)?;
        colors.set("PURPLE", create_color(200, 122, 255, 255)?)?;
        raylib_ctx.set("colors", colors)?;
        
        // Add common Raylib keys
        let keys = self.lua.create_table()?;
        keys.set("SPACE", 32)?;
        keys.set("ESCAPE", 256)?;
        keys.set("ENTER", 257)?;
        keys.set("TAB", 258)?;
        keys.set("BACKSPACE", 259)?;
        keys.set("INSERT", 260)?;
        keys.set("DELETE", 261)?;
        keys.set("RIGHT", 262)?;
        keys.set("LEFT", 263)?;
        keys.set("DOWN", 264)?;
        keys.set("UP", 265)?;
        raylib_ctx.set("keys", keys)?;
        
        // Add basic drawing functions - these will be implemented as stubs
        // The actual implementation will be in the Raylib renderer
        raylib_ctx.set("BeginDrawing", self.lua.create_function(|_, ()| {
            Ok(())
        })?)?;
        
        raylib_ctx.set("EndDrawing", self.lua.create_function(|_, ()| {
            Ok(())
        })?)?;
        
        raylib_ctx.set("ClearBackground", self.lua.create_function(|_, _color: LuaTable| {
            // Store the clear background command for later execution
            Ok(())
        })?)?;
        
        raylib_ctx.set("DrawText", self.lua.create_function(|_, (text, x, y, font_size, _color): (String, i32, i32, i32, LuaTable)| {
            // Store the draw text command for later execution
            Ok(())
        })?)?;
        
        raylib_ctx.set("DrawCube", self.lua.create_function(|_, (_position, _width, _height, _length, _color): (LuaTable, f32, f32, f32, LuaTable)| {
            // Store the draw cube command for later execution
            Ok(())
        })?)?;
        
        raylib_ctx.set("DrawCubeWires", self.lua.create_function(|_, (_position, _width, _height, _length, _color): (LuaTable, f32, f32, f32, LuaTable)| {
            // Store the draw cube wires command for later execution
            Ok(())
        })?)?;
        
        raylib_ctx.set("BeginMode3D", self.lua.create_function(|_, (_camera_pos, _camera_target, _camera_up, _fovy, _near_plane, _far_plane): (LuaTable, LuaTable, LuaTable, f32, f32, f32)| {
            // Store the begin 3D mode command for later execution
            Ok(())
        })?)?;
        
        raylib_ctx.set("EndMode3D", self.lua.create_function(|_, ()| {
            // Store the end 3D mode command for later execution
            Ok(())
        })?)?;
        
        raylib_ctx.set("GetTime", self.lua.create_function(|_, ()| {
            // Return current time in seconds
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default();
            Ok(now.as_secs_f64())
        })?)?;
        
        raylib_ctx.set("IsKeyPressed", self.lua.create_function(|_, key: i32| {
            // For now, always return false - actual implementation will be in the renderer
            Ok(false)
        })?)?;
        
        // Set the global raylib context
        globals.set("rl_ctx", raylib_ctx)?;
        
        Ok(())
    }
    
    /// Setup WGPU-specific API (placeholder for now)
    fn setup_wgpu_api(&self, globals: &LuaTable) -> Result<()> {
        let wgpu_ctx = self.lua.create_table()?;
        
        // Add placeholder WGPU functions
        wgpu_ctx.set("get_device", self.lua.create_function(|_, ()| {
            Ok("wgpu_device_placeholder")
        })?)?;
        
        wgpu_ctx.set("get_queue", self.lua.create_function(|_, ()| {
            Ok("wgpu_queue_placeholder")
        })?)?;
        
        globals.set("wgpu_ctx", wgpu_ctx)?;
        
        Ok(())
    }
    
    /// Execute a native render script
    pub fn execute_render_script(&self, script_name: &str) -> Result<()> {
        let globals = self.lua.globals();
        
        // Get the render script function
        let render_function: LuaFunction = globals.get(script_name).map_err(|e| {
            ScriptError::NativeRendererError {
                backend: self.backend.clone(),
                error: format!("Render script '{}' not found: {}", script_name, e),
            }
        })?;
        
        // Call the render script with the appropriate context
        let context_name = match self.backend.as_str() {
            "raylib" => "rl_ctx",
            "wgpu" => "wgpu_ctx",
            _ => return Err(ScriptError::NativeRendererError {
                backend: self.backend.clone(),
                error: "Unsupported backend".to_string(),
            }.into()),
        };
        
        let context: LuaValue = globals.get(context_name)?;
        render_function.call::<_, ()>(context).map_err(|e| {
            ScriptError::NativeRendererError {
                backend: self.backend.clone(),
                error: format!("Error executing render script '{}': {}", script_name, e),
            }
        })?;
        
        Ok(())
    }
    
    /// Get the element bounds
    pub fn get_element_bounds(&self) -> (Vec2, Vec2) {
        self.element_bounds
    }
    
    /// Get the backend type
    pub fn get_backend(&self) -> &str {
        &self.backend
    }
}

// Raylib types are represented as Lua tables for simplicity

#[cfg(test)]
mod tests {
    use super::*;
    use mlua::Lua;
    
    #[test]
    fn test_native_renderer_context_creation() {
        let lua = Rc::new(Lua::new());
        let context = NativeRendererContext::new(
            lua,
            "raylib".to_string(),
            1,
            Vec2::new(100.0, 100.0),
            Vec2::new(200.0, 200.0)
        );
        assert!(context.is_ok());
    }
    
    // Additional tests can be added here as needed
}
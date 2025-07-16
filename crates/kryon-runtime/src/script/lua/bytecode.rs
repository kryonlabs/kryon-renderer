//! Lua bytecode execution support
//!
//! This module provides bytecode execution capabilities for the Lua engine,
//! enabling the runtime to execute precompiled Lua bytecode directly
//! without parsing source code.

use std::rc::Rc;
use anyhow::Result;
use mlua::{Lua, Function as LuaFunction};
use crate::script::error::ScriptError;

/// Lua bytecode executor
/// 
/// This component handles:
/// - Loading precompiled Lua bytecode
/// - Executing bytecode with proper error handling
/// - Memory management for bytecode execution
/// - Integration with the broader Lua engine
pub struct LuaBytecodeExecutor {
    /// Reference to the Lua VM
    lua: Rc<Lua>,
}

impl LuaBytecodeExecutor {
    /// Create a new bytecode executor
    pub fn new(lua: Rc<Lua>) -> Result<Self> {
        Ok(Self {
            lua,
        })
    }
    
    /// Execute precompiled bytecode
    /// 
    /// This method loads and executes Lua bytecode that was previously
    /// compiled by the kryon-compiler. The bytecode contains the compiled
    /// script functions and is executed in the current Lua context.
    pub fn execute(&mut self, bytecode: &[u8]) -> Result<()> {
        // Validate bytecode format
        if bytecode.is_empty() {
            return Err(ScriptError::BytecodeExecutionFailed {
                script_name: "unknown".to_string(),
                error: "Bytecode is empty".to_string(),
            }.into());
        }
        
        // Check for Lua bytecode header
        if !self.is_valid_lua_bytecode(bytecode) {
            return Err(ScriptError::BytecodeExecutionFailed {
                script_name: "unknown".to_string(),
                error: "Invalid Lua bytecode format".to_string(),
            }.into());
        }
        
        // Load bytecode as a function
        let function = self.lua.load(bytecode).into_function().map_err(|e| {
            ScriptError::BytecodeExecutionFailed {
                script_name: "unknown".to_string(),
                error: format!("Failed to load bytecode: {}", e),
            }
        })?;
        
        // Execute the function to load script definitions
        function.call::<_, ()>(()).map_err(|e| {
            ScriptError::BytecodeExecutionFailed {
                script_name: "unknown".to_string(),
                error: format!("Failed to execute bytecode: {}", e),
            }
        })?;
        
        tracing::debug!("Successfully executed Lua bytecode ({} bytes)", bytecode.len());
        Ok(())
    }
    
    /// Load and execute bytecode with script name for better error reporting
    pub fn load_and_execute(&mut self, script_name: &str, bytecode: &[u8]) -> Result<()> {
        self.execute_named(script_name, bytecode)
    }
    
    /// Execute bytecode with script name for better error reporting
    pub fn execute_named(&mut self, script_name: &str, bytecode: &[u8]) -> Result<()> {
        // Validate bytecode format
        if bytecode.is_empty() {
            return Err(ScriptError::BytecodeExecutionFailed {
                script_name: script_name.to_string(),
                error: "Bytecode is empty".to_string(),
            }.into());
        }
        
        if !self.is_valid_lua_bytecode(bytecode) {
            return Err(ScriptError::BytecodeExecutionFailed {
                script_name: script_name.to_string(),
                error: "Invalid Lua bytecode format".to_string(),
            }.into());
        }
        
        // Load bytecode with script name for debugging
        let function = self.lua.load(bytecode)
            .set_name(script_name)
            .into_function()
            .map_err(|e| {
                ScriptError::BytecodeExecutionFailed {
                    script_name: script_name.to_string(),
                    error: format!("Failed to load bytecode: {}", e),
                }
            })?;
        
        // Execute the function
        function.call::<_, ()>(()).map_err(|e| {
            ScriptError::BytecodeExecutionFailed {
                script_name: script_name.to_string(),
                error: format!("Failed to execute bytecode: {}", e),
            }
        })?;
        
        tracing::debug!("Successfully executed Lua bytecode for '{}' ({} bytes)", 
                       script_name, bytecode.len());
        Ok(())
    }
    
    /// Load bytecode as a function without executing it
    /// 
    /// This is useful for loading functions that will be called later,
    /// or for inspecting bytecode without side effects.
    pub fn load_as_function(&self, bytecode: &[u8]) -> Result<LuaFunction> {
        if !self.is_valid_lua_bytecode(bytecode) {
            return Err(ScriptError::BytecodeExecutionFailed {
                script_name: "unknown".to_string(),
                error: "Invalid Lua bytecode format".to_string(),
            }.into());
        }
        
        self.lua.load(bytecode).into_function().map_err(|e| {
            ScriptError::BytecodeExecutionFailed {
                script_name: "unknown".to_string(),
                error: format!("Failed to load bytecode as function: {}", e),
            }.into()
        })
    }
    
    /// Validate Lua bytecode format
    /// 
    /// This performs basic validation to ensure the bytecode is in a
    /// format that Lua can understand. It checks for the Lua signature
    /// and basic structural integrity.
    fn is_valid_lua_bytecode(&self, bytecode: &[u8]) -> bool {
        // Check minimum size for Lua bytecode header
        if bytecode.len() < 4 {
            return false;
        }
        
        // Check for Lua bytecode signature
        // Lua 5.1/LuaJIT bytecode starts with specific bytes
        // This is a simplified check - in practice, you might want more thorough validation
        
        // LuaJIT bytecode typically starts with 0x1B followed by "LJ"
        if bytecode.len() >= 3 && bytecode[0] == 0x1B && bytecode[1] == b'L' && bytecode[2] == b'J' {
            return true;
        }
        
        // Standard Lua bytecode starts with 0x1B followed by "Lua"
        if bytecode.len() >= 4 && bytecode[0] == 0x1B && 
           bytecode[1] == b'L' && bytecode[2] == b'u' && bytecode[3] == b'a' {
            return true;
        }
        
        // For now, also allow any bytecode that doesn't start with obvious text
        // This is a fallback for development - in production you'd want stricter validation
        !bytecode.starts_with(b"function") && !bytecode.starts_with(b"local")
    }
    
    /// Get bytecode information for debugging
    pub fn get_bytecode_info(&self, bytecode: &[u8]) -> BytecodeInfo {
        BytecodeInfo {
            size: bytecode.len(),
            is_valid: self.is_valid_lua_bytecode(bytecode),
            format_type: self.detect_bytecode_format(bytecode),
        }
    }
    
    /// Detect the specific bytecode format
    fn detect_bytecode_format(&self, bytecode: &[u8]) -> BytecodeFormat {
        if bytecode.len() < 3 {
            return BytecodeFormat::Unknown;
        }
        
        if bytecode[0] == 0x1B && bytecode[1] == b'L' && bytecode[2] == b'J' {
            BytecodeFormat::LuaJIT
        } else if bytecode.len() >= 4 && bytecode[0] == 0x1B && 
                  bytecode[1] == b'L' && bytecode[2] == b'u' && bytecode[3] == b'a' {
            BytecodeFormat::StandardLua
        } else {
            BytecodeFormat::Unknown
        }
    }
}

/// Information about loaded bytecode
#[derive(Debug, Clone)]
pub struct BytecodeInfo {
    /// Size of the bytecode in bytes
    pub size: usize,
    /// Whether the bytecode appears to be valid
    pub is_valid: bool,
    /// The detected bytecode format
    pub format_type: BytecodeFormat,
}

/// Types of Lua bytecode formats
#[derive(Debug, Clone, PartialEq)]
pub enum BytecodeFormat {
    /// Standard Lua bytecode
    StandardLua,
    /// LuaJIT bytecode
    LuaJIT,
    /// Unknown or invalid format
    Unknown,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_bytecode_executor_creation() {
        let lua = Lua::new();
        let executor = LuaBytecodeExecutor::new(&lua);
        assert!(executor.is_ok());
    }
    
    #[test]
    fn test_bytecode_validation() {
        let lua = Lua::new();
        let executor = LuaBytecodeExecutor::new(&lua).unwrap();
        
        // Test empty bytecode
        assert!(!executor.is_valid_lua_bytecode(&[]));
        
        // Test source code (should not be valid bytecode)
        let source_code = b"function test() end";
        assert!(!executor.is_valid_lua_bytecode(source_code));
        
        // Test simulated LuaJIT bytecode
        let luajit_bytecode = b"\x1BLJ\x01\x02\x03";
        assert!(executor.is_valid_lua_bytecode(luajit_bytecode));
        
        // Test simulated standard Lua bytecode
        let lua_bytecode = b"\x1BLua\x51\x00";
        assert!(executor.is_valid_lua_bytecode(lua_bytecode));
    }
    
    #[test]
    fn test_bytecode_format_detection() {
        let lua = Lua::new();
        let executor = LuaBytecodeExecutor::new(&lua).unwrap();
        
        // Test LuaJIT format detection
        let luajit_bytecode = b"\x1BLJ\x01\x02\x03";
        assert_eq!(executor.detect_bytecode_format(luajit_bytecode), BytecodeFormat::LuaJIT);
        
        // Test standard Lua format detection
        let lua_bytecode = b"\x1BLua\x51\x00";
        assert_eq!(executor.detect_bytecode_format(lua_bytecode), BytecodeFormat::StandardLua);
        
        // Test unknown format
        let unknown_bytecode = b"unknown";
        assert_eq!(executor.detect_bytecode_format(unknown_bytecode), BytecodeFormat::Unknown);
    }
    
    #[test]
    fn test_empty_bytecode_handling() {
        let lua = Lua::new();
        let mut executor = LuaBytecodeExecutor::new(&lua).unwrap();
        
        let result = executor.execute(&[]);
        assert!(result.is_err());
        
        if let Err(e) = result {
            assert!(e.to_string().contains("Bytecode is empty"));
        }
    }
}

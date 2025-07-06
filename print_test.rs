#!/usr/bin/env rust-script
//! ```cargo
//! [dependencies]
//! mlua = { version = "0.9", features = ["luajit", "vendored"] }
//! ```

use mlua::Lua;

fn main() -> mlua::Result<()> {
    println!("Testing Lua print functionality...");
    
    let lua = Lua::new();
    
    // Setup custom print function that forwards to stdout
    lua.globals().set("print", lua.create_function(|_, args: mlua::Variadic<mlua::Value>| {
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
    })?)?;
    
    // Test the print function
    lua.load(r#"
        print("Hello from Lua!")
        print("Testing numbers:", 42, 3.14)
        print("Testing booleans:", true, false)
        print("Testing nil:", nil)
        print("Testing multiple arguments:", "arg1", "arg2", "arg3")
    "#).exec()?;
    
    println!("Test completed successfully!");
    Ok(())
}
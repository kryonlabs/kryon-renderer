use anyhow::{Context, Result};
use clap::Parser;
use std::fs;
use std::path::Path;

#[derive(Parser)]
#[command(name = "kryon-renderer-debug")]
#[command(about = "Debug renderer that outputs KRB hierarchy as text")]
struct Args {
    /// Path to the .krb file to analyze
    krb_file: String,

    /// Output format (tree, json, detailed)
    #[arg(long, default_value = "tree")]
    format: String,

    /// Save output to file instead of stdout
    #[arg(long)]
    output: Option<String>,

    /// Show property values
    #[arg(long)]
    show_properties: bool,

    /// Show element positions and sizes
    #[arg(long)]
    show_layout: bool,

    /// Show color values in hex format
    #[arg(long)]
    show_colors: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Validate file path
    if !Path::new(&args.krb_file).exists() {
        anyhow::bail!("KRB file not found: {}", args.krb_file);
    }

    println!("Loading KRB file: {}", args.krb_file);
    
    // Load the KRB file
    let krb_file = kryon_core::load_krb_file(&args.krb_file)
        .context("Failed to load KRB file")?;

    // Generate output based on format
    let output_text = match args.format.as_str() {
        "tree" => generate_tree_output(&krb_file, &args),
        "json" => generate_json_output(&krb_file),
        "detailed" => generate_detailed_output(&krb_file, &args),
        _ => anyhow::bail!("Unknown format: {}. Use 'tree', 'json', or 'detailed'"),
    }?;

    // Output to file or stdout
    if let Some(output_file) = args.output {
        fs::write(&output_file, output_text)
            .with_context(|| format!("Failed to write to file: {}", output_file))?;
        println!("Output written to: {}", output_file);
    } else {
        print!("{}", output_text);
    }

    Ok(())
}

fn generate_tree_output(krb_file: &kryon_core::KrbFile, args: &Args) -> Result<String> {
    let mut output = String::new();
    
    if let Some(root_id) = krb_file.root_element_id {
        render_element_tree(&mut output, krb_file, root_id, 0, args, true)?;
    } else {
        output.push_str("No root element found\n");
    }

    Ok(output)
}

fn render_element_tree(
    output: &mut String, 
    krb_file: &kryon_core::KrbFile, 
    element_id: u32, 
    depth: usize, 
    args: &Args,
    is_last: bool
) -> Result<()> {
    if let Some(element) = krb_file.elements.get(&element_id) {
        // Generate clean tree lines
        let tree_char = if depth == 0 {
            ""
        } else if is_last {
            "└── "
        } else {
            "├── "
        };
        
        let indent = if depth == 0 {
            String::new()
        } else {
            "│   ".repeat(depth - 1) + tree_char
        };
        
        // Element type and basic info  
        output.push_str(&format!("{}{:?}", indent, element.element_type));
        
        if !element.text.is_empty() {
            output.push_str(&format!(" \"{}\"", element.text));
        }
        
        // Layout information
        if args.show_layout {
            output.push_str(&format!(" pos:({:.0},{:.0}) size:({:.0},{:.0})", 
                                   element.position.x, element.position.y,
                                   element.size.x, element.size.y));
        }
        
        // Show key properties inline
        let mut inline_props = Vec::new();
        if let Some(bg_color) = element.properties.get("background_color") {
            if let Some(color_str) = bg_color.as_str() {
                if args.show_colors {
                    inline_props.push(format!("bg:{}", color_str));
                } else {
                    inline_props.push("bg:color".to_string());
                }
            }
        }
        
        if let Some(text_color) = element.properties.get("text_color") {
            if let Some(color_str) = text_color.as_str() {
                if args.show_colors {
                    inline_props.push(format!("color:{}", color_str));
                } else {
                    inline_props.push("color:set".to_string());
                }
            }
        }
        
        if !inline_props.is_empty() {
            output.push_str(&format!(" [{}]", inline_props.join(" ")));
        }
        
        output.push('\n');
        
        // Properties (detailed view)
        if args.show_properties {
            let prop_indent = if depth == 0 {
                "    "
            } else {
                &format!("{}    ", "│   ".repeat(depth))
            };
            
            for (prop_name, prop_value) in &element.properties {
                if prop_name == "background_color" || prop_name == "text_color" {
                    continue; // Already shown inline
                }
                
                output.push_str(&format!("{}• {}: ", prop_indent, prop_name));
                
                if args.show_colors && (prop_name.contains("color") || prop_name.contains("Color")) {
                    if let Some(color_val) = prop_value.as_str() {
                        output.push_str(&format!("{}", color_val));
                        if let Ok(parsed_color) = parse_color_value(color_val) {
                            output.push_str(&format!(" (#{:02X}{:02X}{:02X}{:02X})", 
                                                    parsed_color.r, parsed_color.g, 
                                                    parsed_color.b, parsed_color.a));
                        }
                    }
                } else {
                    output.push_str(&format!("{}", prop_value));
                }
                output.push('\n');
            }
        }
        
        // Render children
        let child_count = element.children.len();
        for (i, child_id) in element.children.iter().enumerate() {
            let is_last_child = i == child_count - 1;
            render_element_tree(output, krb_file, *child_id, depth + 1, args, is_last_child)?;
        }
    }
    
    Ok(())
}

fn generate_json_output(krb_file: &kryon_core::KrbFile) -> Result<String> {
    // Simple JSON representation
    let mut output = String::new();
    output.push_str("{\n");
    output.push_str(&format!("  \"version\": \"{}.{}\",\n", 
                            krb_file.header.version_major, 
                            krb_file.header.version_minor));
    output.push_str(&format!("  \"element_count\": {},\n", krb_file.elements.len()));
    output.push_str(&format!("  \"string_count\": {},\n", krb_file.strings.len()));
    output.push_str("  \"elements\": [\n");
    
    let mut first = true;
    for (id, element) in &krb_file.elements {
        if !first {
            output.push_str(",\n");
        }
        first = false;
        
        output.push_str("    {\n");
        output.push_str(&format!("      \"id\": {},\n", id));
        output.push_str(&format!("      \"type\": \"{:?}\",\n", element.element_type));
        output.push_str(&format!("      \"text\": \"{}\",\n", element.text.replace('"', "\\\"")));
        output.push_str(&format!("      \"position\": [{:.1}, {:.1}],\n", element.position.x, element.position.y));
        output.push_str(&format!("      \"size\": [{:.1}, {:.1}],\n", element.size.x, element.size.y));
        output.push_str(&format!("      \"children\": {:?}\n", element.children));
        output.push_str("    }");
    }
    
    output.push_str("\n  ]\n");
    output.push_str("}\n");
    Ok(output)
}

fn generate_detailed_output(krb_file: &kryon_core::KrbFile, args: &Args) -> Result<String> {
    let mut output = String::new();
    
    output.push_str("=== KRYON BINARY FILE ANALYSIS ===\n\n");
    
    // Header information
    output.push_str("HEADER:\n");
    output.push_str(&format!("  Version: {}.{}\n", 
                            krb_file.header.version_major, 
                            krb_file.header.version_minor));
    output.push_str(&format!("  Flags: 0x{:04X}\n", krb_file.header.flags));
    output.push_str("\n");
    
    // String table
    output.push_str("STRING TABLE:\n");
    for (i, string_val) in krb_file.strings.iter().enumerate() {
        output.push_str(&format!("  [{}]: \"{}\"\n", i, string_val));
    }
    output.push_str("\n");
    
    // Element tree
    output.push_str("ELEMENT TREE:\n");
    if let Some(root_id) = krb_file.root_element_id {
        render_element_tree(&mut output, krb_file, root_id, 0, args)?;
    }
    
    output.push_str("\n=== END ANALYSIS ===\n");
    Ok(output)
}

#[derive(Debug)]
struct ColorValue {
    r: u8,
    g: u8, 
    b: u8,
    a: u8,
}

fn parse_color_value(color_str: &str) -> Result<ColorValue> {
    // Try to parse hex color format #RRGGBBAA or #RRGGBB
    if color_str.starts_with('#') && (color_str.len() == 7 || color_str.len() == 9) {
        let hex = &color_str[1..];
        let r = u8::from_str_radix(&hex[0..2], 16)?;
        let g = u8::from_str_radix(&hex[2..4], 16)?;
        let b = u8::from_str_radix(&hex[4..6], 16)?;
        let a = if hex.len() == 8 {
            u8::from_str_radix(&hex[6..8], 16)?
        } else {
            255
        };
        
        Ok(ColorValue { r, g, b, a })
    } else {
        anyhow::bail!("Invalid color format: {}", color_str)
    }
}
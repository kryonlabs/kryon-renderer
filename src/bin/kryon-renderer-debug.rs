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

    eprintln!("Loading KRB file: {}", args.krb_file);
    
    // Load the KRB file
    let krb_file = kryon_core::load_krb_file(&args.krb_file)
        .context("Failed to load KRB file")?;

    // Generate output based on format
    let output_text = match args.format.as_str() {
        "tree" => generate_tree_output(&krb_file, &args),
        "json" => generate_json_output(&krb_file),
        "detailed" => generate_detailed_output(&krb_file, &args),
        _ => anyhow::bail!("Unknown format: {}. Use 'tree', 'json', or 'detailed'", args.format),
    }?;

    // Output to file or stdout
    if let Some(output_file) = args.output {
        fs::write(&output_file, output_text)
            .with_context(|| format!("Failed to write to file: {}", output_file))?;
        eprintln!("Output written to: {}", output_file);
    } else {
        print!("{}", output_text);
    }

    Ok(())
}

fn generate_tree_output(krb_file: &kryon_core::KRBFile, args: &Args) -> Result<String> {
    let mut output = String::new();
    
    // Show file overview first
    output.push_str(&format!("=== KRB FILE OVERVIEW ===\n"));
    output.push_str(&format!("Elements: {}, Styles: {}, Strings: {}\n", 
        krb_file.elements.len(), krb_file.styles.len(), krb_file.strings.len()));
    output.push_str(&format!("Root Element: {:?}\n\n", krb_file.root_element_id));
    
    // Show styles summary
    if !krb_file.styles.is_empty() {
        output.push_str("=== STYLES SUMMARY ===\n");
        for (style_id, style) in &krb_file.styles {
            output.push_str(&format!("Style {}: '{}' ({} properties)\n", 
                style_id, style.name, style.properties.len()));
            for (prop_id, prop_value) in &style.properties {
                output.push_str(&format!("  Property 0x{:02X}: {:?}\n", prop_id, prop_value));
            }
        }
        output.push_str("\n");
    }
    
    output.push_str("=== ELEMENT TREE ===\n");
    if let Some(root_id) = krb_file.root_element_id {
        render_element_tree(&mut output, krb_file, root_id, 0, args, true)?;
    } else {
        output.push_str("No root element found\n");
    }

    Ok(output)
}

fn render_element_tree(
    output: &mut String, 
    krb_file: &kryon_core::KRBFile, 
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
        
        // Show background color if not transparent
        if element.background_color.w > 0.0 {
            if args.show_colors {
                inline_props.push(format!("bg:#{:02X}{:02X}{:02X}{:02X}", 
                    (element.background_color.x * 255.0) as u8,
                    (element.background_color.y * 255.0) as u8,
                    (element.background_color.z * 255.0) as u8,
                    (element.background_color.w * 255.0) as u8));
            } else {
                inline_props.push("bg:set".to_string());
            }
        }
        
        // Show text color if not default
        if element.text_color != glam::Vec4::new(0.0, 0.0, 0.0, 1.0) {
            if args.show_colors {
                inline_props.push(format!("color:#{:02X}{:02X}{:02X}{:02X}", 
                    (element.text_color.x * 255.0) as u8,
                    (element.text_color.y * 255.0) as u8,
                    (element.text_color.z * 255.0) as u8,
                    (element.text_color.w * 255.0) as u8));
            } else {
                inline_props.push("color:set".to_string());
            }
        }
        
        // Show border if present OR if border_color is set (should auto-apply width)
        if element.border_width > 0.0 || element.border_color.w > 0.0 {
            let width = if element.border_width > 0.0 { element.border_width } else { 1.0 }; // Default width
            if args.show_colors && element.border_color.w > 0.0 {
                inline_props.push(format!("border:{}px #{:02X}{:02X}{:02X}{:02X}", 
                    width,
                    (element.border_color.x * 255.0) as u8,
                    (element.border_color.y * 255.0) as u8,
                    (element.border_color.z * 255.0) as u8,
                    (element.border_color.w * 255.0) as u8));
            } else if element.border_color.w > 0.0 {
                inline_props.push(format!("border:{}px", width));
            } else if element.border_width > 0.0 {
                inline_props.push(format!("border:{}px", element.border_width));
            }
        }
        
        // Show border radius if present
        if element.border_radius > 0.0 {
            inline_props.push(format!("radius:{}", element.border_radius));
        }
        
        // Show opacity if not 1.0
        if element.opacity != 1.0 {
            inline_props.push(format!("opacity:{:.2}", element.opacity));
        }
        
        // Show visibility if hidden
        if !element.visible {
            inline_props.push("hidden".to_string());
        }
        
        // Show disabled state
        if element.disabled {
            inline_props.push("disabled".to_string());
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
            
            // Show style inheritance information
            if element.style_id != 0 {
                if let Some(style) = krb_file.styles.get(&element.style_id) {
                    output.push_str(&format!("{}• Applied style: '{}' (id: {})\n", prop_indent, style.name, element.style_id));
                    for (prop_id, prop_value) in &style.properties {
                        output.push_str(&format!("{}  - Style property 0x{:02X}: {:?}\n", prop_indent, prop_id, prop_value));
                    }
                }
            }
            
            // Show computed final values vs original values
            output.push_str(&format!("{}• COMPUTED FINAL VALUES:\n", prop_indent));
            
            // Show various element properties
            if !element.text.is_empty() {
                output.push_str(&format!("{}• text: \"{}\"\n", prop_indent, element.text));
            }
            if element.font_size != 14.0 {  // Default is 14.0, not 16.0
                output.push_str(&format!("{}• font_size: {}\n", prop_indent, element.font_size));
            }
            if element.font_weight != kryon_core::FontWeight::Normal {
                output.push_str(&format!("{}• font_weight: {:?}\n", prop_indent, element.font_weight));
            }
            if element.text_alignment != kryon_core::TextAlignment::Start {
                output.push_str(&format!("{}• text_alignment: {:?}\n", prop_indent, element.text_alignment));
            }
            
            // Visual properties
            if element.background_color.w > 0.0 {
                if args.show_colors {
                    output.push_str(&format!("{}• background_color: #{:02X}{:02X}{:02X}{:02X}\n", 
                        prop_indent,
                        (element.background_color.x * 255.0) as u8,
                        (element.background_color.y * 255.0) as u8,
                        (element.background_color.z * 255.0) as u8,
                        (element.background_color.w * 255.0) as u8));
                } else {
                    output.push_str(&format!("{}• background_color: set\n", prop_indent));
                }
            }
            if element.text_color != glam::Vec4::new(0.0, 0.0, 0.0, 1.0) {
                if args.show_colors {
                    output.push_str(&format!("{}• text_color: #{:02X}{:02X}{:02X}{:02X}\n", 
                        prop_indent,
                        (element.text_color.x * 255.0) as u8,
                        (element.text_color.y * 255.0) as u8,
                        (element.text_color.z * 255.0) as u8,
                        (element.text_color.w * 255.0) as u8));
                } else {
                    output.push_str(&format!("{}• text_color: set\n", prop_indent));
                }
            }
            
            // Border properties
            if element.border_width > 0.0 {
                output.push_str(&format!("{}• border_width: {}\n", prop_indent, element.border_width));
                if element.border_color.w > 0.0 {
                    if args.show_colors {
                        output.push_str(&format!("{}• border_color: #{:02X}{:02X}{:02X}{:02X}\n", 
                            prop_indent,
                            (element.border_color.x * 255.0) as u8,
                            (element.border_color.y * 255.0) as u8,
                            (element.border_color.z * 255.0) as u8,
                            (element.border_color.w * 255.0) as u8));
                    } else {
                        output.push_str(&format!("{}• border_color: set\n", prop_indent));
                    }
                }
            }
            if element.border_radius > 0.0 {
                output.push_str(&format!("{}• border_radius: {}\n", prop_indent, element.border_radius));
            }
            
            // Other visual properties
            if element.opacity != 1.0 {
                output.push_str(&format!("{}• opacity: {}\n", prop_indent, element.opacity));
            }
            if !element.visible {
                output.push_str(&format!("{}• visible: false\n", prop_indent));
            }
            if element.disabled {
                output.push_str(&format!("{}• disabled: true\n", prop_indent));
            }
            if element.cursor != kryon_core::CursorType::Default {
                output.push_str(&format!("{}• cursor: {:?}\n", prop_indent, element.cursor));
            }
            
            // Layout properties
            if element.layout_flags != 0 {
                output.push_str(&format!("{}• layout_flags: 0x{:02X}\n", prop_indent, element.layout_flags));
            }
            if element.style_id != 0 {
                output.push_str(&format!("{}• style_id: {}\n", prop_indent, element.style_id));
            }
            
            // Component properties
            if let Some(ref component_name) = element.component_name {
                output.push_str(&format!("{}• component_name: \"{}\"\n", prop_indent, component_name));
            }
            if element.is_component_instance {
                output.push_str(&format!("{}• is_component_instance: true\n", prop_indent));
            }
            
            // State
            if element.current_state != kryon_core::InteractionState::Normal {
                output.push_str(&format!("{}• current_state: {:?}\n", prop_indent, element.current_state));
            }
            
            // Custom properties
            for (prop_name, prop_value) in &element.custom_properties {
                output.push_str(&format!("{}• {}: {:?}\n", prop_indent, prop_name, prop_value));
            }
            
            // Event handlers
            if !element.event_handlers.is_empty() {
                for (event_type, handler) in &element.event_handlers {
                    output.push_str(&format!("{}• on_{:?}: \"{}\"\n", prop_indent, event_type, handler));
                }
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

fn generate_json_output(krb_file: &kryon_core::KRBFile) -> Result<String> {
    // Simple JSON representation
    let mut output = String::new();
    output.push_str("{\n");
    output.push_str(&format!("  \"version\": \"{}\",\n", krb_file.header.version));
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

fn generate_detailed_output(krb_file: &kryon_core::KRBFile, args: &Args) -> Result<String> {
    let mut output = String::new();
    
    output.push_str("=== KRYON BINARY FILE ANALYSIS ===\n\n");
    
    // Header information
    output.push_str("HEADER:\n");
    output.push_str(&format!("  Version: {}\n", krb_file.header.version));
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
        render_element_tree(&mut output, krb_file, root_id, 0, args, true)?;
    }
    
    output.push_str("\n=== END ANALYSIS ===\n");
    Ok(output)
}

#[derive(Debug)]
struct _ColorValue {
    r: u8,
    g: u8, 
    b: u8,
    a: u8,
}

fn _parse_color_value(color_str: &str) -> Result<_ColorValue> {
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
        
        Ok(_ColorValue { r, g, b, a })
    } else {
        anyhow::bail!("Invalid color format: {}", color_str)
    }
}
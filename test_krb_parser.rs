use kryon_core::krb::KRBParser;
use std::fs;

fn main() {
    let data = fs::read("test_input_types.krb").expect("Failed to read KRB file");
    let mut parser = KRBParser::new(data);
    let krb_file = parser.parse().expect("Failed to parse KRB file");
    
    println!("Parsed KRB file successfully!");
    println!("Elements: {}", krb_file.elements.len());
    
    for (id, element) in &krb_file.elements {
        println!("\nElement {}: {:?}", id, element.element_type);
        println!("  ID: {}", element.id);
        
        if element.element_type == kryon_core::ElementType::Input {
            if let Some(input_type) = element.custom_properties.get("input_type") {
                println!("  Input Type: {:?}", input_type);
            }
        }
        
        println!("  Custom Properties: {:?}", element.custom_properties);
    }
}
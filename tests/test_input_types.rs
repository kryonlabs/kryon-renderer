use kryon_core::krb::KRBParser;
use kryon_core::ElementType;
use std::fs;

#[test]
fn test_parse_input_types() {
    let data = fs::read("test_input_types.krb").expect("Failed to read KRB file");
    let mut parser = KRBParser::new(data);
    let krb_file = parser.parse().expect("Failed to parse KRB file");
    
    println!("Parsed KRB file successfully!");
    println!("Elements: {}", krb_file.elements.len());
    
    let mut input_count = 0;
    let mut found_types = Vec::new();
    
    for (id, element) in &krb_file.elements {
        println!("\nElement {}: {:?}", id, element.element_type);
        println!("  ID: {}", element.id);
        
        if element.element_type == ElementType::Input {
            input_count += 1;
            if let Some(input_type) = element.custom_properties.get("input_type") {
                println!("  Input Type: {:?}", input_type);
                if let kryon_core::PropertyValue::String(type_str) = input_type {
                    found_types.push(type_str.clone());
                }
            }
        }
    }
    
    // Verify we found all 4 input elements
    assert_eq!(input_count, 4);
    
    // Verify the input types
    assert!(found_types.iter().any(|t| t.contains("text")));
    assert!(found_types.iter().any(|t| t.contains("checkbox")));
    assert!(found_types.iter().any(|t| t.contains("range")));
    assert!(found_types.iter().any(|t| t.contains("email")));
}
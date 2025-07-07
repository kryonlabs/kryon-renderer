use std::collections::HashMap;
use glam::Vec2;
use kryon_core::{load_krb_file, Element, ElementId};
use kryon_layout::{TaffyLayoutEngine, LayoutEngine};

#[test] 
fn test_button_centering() {
    let krb_file = load_krb_file("examples/02_basic_ui/button.krb").unwrap();
    
    let mut layout_engine = TaffyLayoutEngine::new();
    let result = layout_engine.compute_layout(&krb_file.elements, 0, Vec2::new(600.0, 400.0));
    
    println\!("Layout result: {:#?}", result);
    
    if let Some(app_pos) = result.computed_positions.get(&0) {
        println\!("App position: {:?}", app_pos);
    }
    if let Some(button_pos) = result.computed_positions.get(&1) {
        println\!("Button position: {:?}", button_pos);
        // Button should be centered at roughly (225, 175) = (600-150)/2, (400-50)/2
        assert\!(button_pos.x > 200.0 && button_pos.x < 250.0, "Button x position should be centered, got {}", button_pos.x);
        assert\!(button_pos.y > 150.0 && button_pos.y < 200.0, "Button y position should be centered, got {}", button_pos.y);
    }
}

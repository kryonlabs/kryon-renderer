//! DOM-based web renderer using HTML elements
//! 
//! This renderer creates HTML elements directly in the DOM, providing better
//! accessibility and text handling compared to canvas-based rendering.

use wasm_bindgen::prelude::*;
use web_sys::{Document, Element, HtmlElement, Window};
use kryon_render::{Renderer, RenderResult, RenderError, RenderCommand};
use kryon_core::Element as KryonElement;
use kryon_layout::LayoutResult;
use glam::{Vec2, Vec4};
use std::collections::HashMap;

pub struct DomRenderer {
    document: Document,
    container: Element,
    element_map: HashMap<String, Element>,
    next_id: u32,
}

impl DomRenderer {
    pub fn new(container_id: &str) -> Result<Self, JsValue> {
        let window = web_sys::window().ok_or("No window object")?;
        let document = window.document().ok_or("No document object")?;
        
        let container = document
            .get_element_by_id(container_id)
            .ok_or("Container element not found")?;
        
        // Clear existing content
        container.set_inner_html("");
        
        // Set up container styles for proper layout
        let container_style = container
            .dyn_ref::<HtmlElement>()
            .ok_or("Container is not an HTML element")?
            .style();
        
        container_style.set_property("position", "relative")?;
        container_style.set_property("width", "100%")?;
        container_style.set_property("height", "100%")?;
        container_style.set_property("overflow", "hidden")?;
        
        Ok(Self {
            document,
            container,
            element_map: HashMap::new(),
            next_id: 0,
        })
    }
    
    pub fn execute_render_command(&mut self, command: &RenderCommand) -> Result<(), JsValue> {
        match command {
            RenderCommand::DrawRect { position, size, color, border_radius, border_width, border_color, .. } => {
                let element_id = self.get_next_id();
                let div = self.document.create_element("div")?;
                
                let style = div
                    .dyn_ref::<HtmlElement>()
                    .ok_or("Element is not an HTML element")?
                    .style();
                
                // Position and size
                style.set_property("position", "absolute")?;
                style.set_property("left", &format!("{}px", position.x))?;
                style.set_property("top", &format!("{}px", position.y))?;
                style.set_property("width", &format!("{}px", size.x))?;
                style.set_property("height", &format!("{}px", size.y))?;
                
                // Background color
                style.set_property("background-color", &format!(
                    "rgba({}, {}, {}, {})",
                    (color.x * 255.0) as u8,
                    (color.y * 255.0) as u8,
                    (color.z * 255.0) as u8,
                    color.w
                ))?;
                
                // Border radius
                if *border_radius > 0.0 {
                    style.set_property("border-radius", &format!("{}px", border_radius))?;
                }
                
                // Border
                if *border_width > 0.0 {
                    style.set_property("border", &format!(
                        "{}px solid rgba({}, {}, {}, {})",
                        border_width,
                        (border_color.x * 255.0) as u8,
                        (border_color.y * 255.0) as u8,
                        (border_color.z * 255.0) as u8,
                        border_color.w
                    ))?;
                }
                
                self.container.append_child(&div)?;
                self.element_map.insert(element_id, div);
            }
            
            RenderCommand::DrawText { position, text, font_size, color, .. } => {
                let element_id = self.get_next_id();
                let span = self.document.create_element("span")?;
                
                let style = span
                    .dyn_ref::<HtmlElement>()
                    .ok_or("Element is not an HTML element")?
                    .style();
                
                // Position and text
                style.set_property("position", "absolute")?;
                style.set_property("left", &format!("{}px", position.x))?;
                style.set_property("top", &format!("{}px", position.y))?;
                style.set_property("font-size", &format!("{}px", font_size))?;
                style.set_property("font-family", "Arial, sans-serif")?;
                style.set_property("color", &format!(
                    "rgba({}, {}, {}, {})",
                    (color.x * 255.0) as u8,
                    (color.y * 255.0) as u8,
                    (color.z * 255.0) as u8,
                    color.w
                ))?;
                style.set_property("white-space", "nowrap")?;
                style.set_property("pointer-events", "none")?;
                
                span.set_text_content(Some(text));
                
                self.container.append_child(&span)?;
                self.element_map.insert(element_id, span);
            }
            
            RenderCommand::SetClip { position, size } => {
                // Create a clipping container
                let element_id = self.get_next_id();
                let clip_div = self.document.create_element("div")?;
                
                let style = clip_div
                    .dyn_ref::<HtmlElement>()
                    .ok_or("Element is not an HTML element")?
                    .style();
                
                style.set_property("position", "absolute")?;
                style.set_property("left", &format!("{}px", position.x))?;
                style.set_property("top", &format!("{}px", position.y))?;
                style.set_property("width", &format!("{}px", size.x))?;
                style.set_property("height", &format!("{}px", size.y))?;
                style.set_property("overflow", "hidden")?;
                
                self.container.append_child(&clip_div)?;
                self.element_map.insert(element_id, clip_div);
            }
            
            _ => {
                // Other commands not implemented yet
                web_sys::console::log_1(&format!("Unimplemented DOM command: {:?}", command).into());
            }
        }
        
        Ok(())
    }
    
    pub fn clear(&mut self) -> Result<(), JsValue> {
        self.container.set_inner_html("");
        self.element_map.clear();
        self.next_id = 0;
        Ok(())
    }
    
    pub fn resize(&mut self, new_size: Vec2) -> Result<(), JsValue> {
        let container_style = self.container
            .dyn_ref::<HtmlElement>()
            .ok_or("Container is not an HTML element")?
            .style();
        
        container_style.set_property("width", &format!("{}px", new_size.x))?;
        container_style.set_property("height", &format!("{}px", new_size.y))?;
        
        Ok(())
    }
    
    fn get_next_id(&mut self) -> String {
        let id = format!("kryon-element-{}", self.next_id);
        self.next_id += 1;
        id
    }
}
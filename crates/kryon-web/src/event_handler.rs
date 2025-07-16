//! Web event handling for mouse, keyboard, and touch events

use wasm_bindgen::prelude::*;
use web_sys::{Event, EventTarget, KeyboardEvent, MouseEvent, TouchEvent, WheelEvent};
use glam::Vec2;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum WebEvent {
    MouseDown { position: Vec2, button: u32 },
    MouseUp { position: Vec2, button: u32 },
    MouseMove { position: Vec2 },
    MouseWheel { delta: Vec2 },
    KeyDown { key: String, code: String },
    KeyUp { key: String, code: String },
    TouchStart { touches: Vec<Touch> },
    TouchMove { touches: Vec<Touch> },
    TouchEnd { touches: Vec<Touch> },
    Resize { size: Vec2 },
}

#[derive(Debug, Clone)]
pub struct Touch {
    pub id: i32,
    pub position: Vec2,
    pub force: f32,
}

pub struct WebEventHandler {
    event_listeners: HashMap<String, Vec<Closure<dyn FnMut(Event)>>>,
    pending_events: Vec<WebEvent>,
}

impl WebEventHandler {
    pub fn new() -> Self {
        Self {
            event_listeners: HashMap::new(),
            pending_events: Vec::new(),
        }
    }
    
    pub fn setup_event_listeners(&mut self, target: &EventTarget) -> Result<(), JsValue> {
        // Mouse events
        self.add_mouse_listener(target, "mousedown")?;
        self.add_mouse_listener(target, "mouseup")?;
        self.add_mouse_listener(target, "mousemove")?;
        self.add_wheel_listener(target)?;
        
        // Keyboard events
        self.add_keyboard_listener(target, "keydown")?;
        self.add_keyboard_listener(target, "keyup")?;
        
        // Touch events
        self.add_touch_listener(target, "touchstart")?;
        self.add_touch_listener(target, "touchmove")?;
        self.add_touch_listener(target, "touchend")?;
        
        // Window resize
        self.add_resize_listener()?;
        
        Ok(())
    }
    
    fn add_mouse_listener(&mut self, target: &EventTarget, event_type: &str) -> Result<(), JsValue> {
        let event_type_owned = event_type.to_string();
        let closure = Closure::wrap(Box::new(move |event: Event| {
            if let Some(mouse_event) = event.dyn_ref::<MouseEvent>() {
                let position = Vec2::new(mouse_event.client_x() as f32, mouse_event.client_y() as f32);
                let button = mouse_event.button() as u32;
                
                let web_event = match event_type_owned.as_str() {
                    "mousedown" => WebEvent::MouseDown { position, button },
                    "mouseup" => WebEvent::MouseUp { position, button },
                    "mousemove" => WebEvent::MouseMove { position },
                    _ => return,
                };
                
                // Store event for processing
                // Note: In real implementation, we'd need a way to access the event handler
                // This is a simplified version
                web_sys::console::log_1(&format!("Mouse event: {:?}", web_event).into());
            }
        }) as Box<dyn FnMut(Event)>);
        
        target.add_event_listener_with_callback(event_type, closure.as_ref().unchecked_ref())?;
        
        self.event_listeners
            .entry(event_type.to_string())
            .or_insert_with(Vec::new)
            .push(closure);
        
        Ok(())
    }
    
    fn add_wheel_listener(&mut self, target: &EventTarget) -> Result<(), JsValue> {
        let closure = Closure::wrap(Box::new(move |event: Event| {
            if let Some(wheel_event) = event.dyn_ref::<WheelEvent>() {
                let delta = Vec2::new(wheel_event.delta_x() as f32, wheel_event.delta_y() as f32);
                let web_event = WebEvent::MouseWheel { delta };
                
                web_sys::console::log_1(&format!("Wheel event: {:?}", web_event).into());
            }
        }) as Box<dyn FnMut(Event)>);
        
        target.add_event_listener_with_callback("wheel", closure.as_ref().unchecked_ref())?;
        
        self.event_listeners
            .entry("wheel".to_string())
            .or_insert_with(Vec::new)
            .push(closure);
        
        Ok(())
    }
    
    fn add_keyboard_listener(&mut self, target: &EventTarget, event_type: &str) -> Result<(), JsValue> {
        let event_type_owned = event_type.to_string();
        let closure = Closure::wrap(Box::new(move |event: Event| {
            if let Some(keyboard_event) = event.dyn_ref::<KeyboardEvent>() {
                let key = keyboard_event.key();
                let code = keyboard_event.code();
                
                let web_event = match event_type_owned.as_str() {
                    "keydown" => WebEvent::KeyDown { key, code },
                    "keyup" => WebEvent::KeyUp { key, code },
                    _ => return,
                };
                
                web_sys::console::log_1(&format!("Keyboard event: {:?}", web_event).into());
            }
        }) as Box<dyn FnMut(Event)>);
        
        target.add_event_listener_with_callback(event_type, closure.as_ref().unchecked_ref())?;
        
        self.event_listeners
            .entry(event_type.to_string())
            .or_insert_with(Vec::new)
            .push(closure);
        
        Ok(())
    }
    
    fn add_touch_listener(&mut self, target: &EventTarget, event_type: &str) -> Result<(), JsValue> {
        let event_type_owned = event_type.to_string();
        let closure = Closure::wrap(Box::new(move |event: Event| {
            if let Some(touch_event) = event.dyn_ref::<TouchEvent>() {
                let touches = Self::extract_touches(touch_event);
                
                let web_event = match event_type_owned.as_str() {
                    "touchstart" => WebEvent::TouchStart { touches },
                    "touchmove" => WebEvent::TouchMove { touches },
                    "touchend" => WebEvent::TouchEnd { touches },
                    _ => return,
                };
                
                web_sys::console::log_1(&format!("Touch event: {:?}", web_event).into());
            }
        }) as Box<dyn FnMut(Event)>);
        
        target.add_event_listener_with_callback(event_type, closure.as_ref().unchecked_ref())?;
        
        self.event_listeners
            .entry(event_type.to_string())
            .or_insert_with(Vec::new)
            .push(closure);
        
        Ok(())
    }
    
    fn add_resize_listener(&mut self) -> Result<(), JsValue> {
        let window = web_sys::window().ok_or("No window object")?;
        
        let closure = Closure::wrap(Box::new(move |_event: Event| {
            if let Some(window) = web_sys::window() {
                let width = window.inner_width().unwrap_or(JsValue::from(800)).as_f64().unwrap_or(800.0);
                let height = window.inner_height().unwrap_or(JsValue::from(600)).as_f64().unwrap_or(600.0);
                let size = Vec2::new(width as f32, height as f32);
                
                let web_event = WebEvent::Resize { size };
                web_sys::console::log_1(&format!("Resize event: {:?}", web_event).into());
            }
        }) as Box<dyn FnMut(Event)>);
        
        window.add_event_listener_with_callback("resize", closure.as_ref().unchecked_ref())?;
        
        self.event_listeners
            .entry("resize".to_string())
            .or_insert_with(Vec::new)
            .push(closure);
        
        Ok(())
    }
    
    fn extract_touches(touch_event: &TouchEvent) -> Vec<Touch> {
        let mut touches = Vec::new();
        let touch_list = touch_event.touches();
        
        for i in 0..touch_list.length() {
            if let Some(touch) = touch_list.get(i) {
                touches.push(Touch {
                    id: touch.identifier(),
                    position: Vec2::new(touch.client_x() as f32, touch.client_y() as f32),
                    force: touch.force(),
                });
            }
        }
        
        touches
    }
    
    pub fn poll_events(&mut self) -> Vec<WebEvent> {
        let events = self.pending_events.clone();
        self.pending_events.clear();
        events
    }
    
    pub fn push_event(&mut self, event: WebEvent) {
        self.pending_events.push(event);
    }
}
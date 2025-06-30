// crates/kryon-core/src/events.rs
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct EventManager {
    pub handlers: HashMap<String, Vec<EventHandler>>,
}

#[derive(Debug, Clone)]
pub struct EventHandler {
    pub event_type: String,
    pub element_id: String,
    pub handler_code: String,
}

#[derive(Debug, Clone)]
pub struct Event {
    pub event_type: String,
    pub target_id: String,
    pub data: EventData,
}

#[derive(Debug, Clone)]
pub enum EventData {
    Mouse { x: f32, y: f32, button: u8 },
    Keyboard { key_code: u32, text: String },
    Focus { gained: bool },
    Value { value: String },
    None,
}

impl Default for EventManager {
    fn default() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }
}

impl EventManager {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn add_handler(&mut self, event_type: String, handler: EventHandler) {
        self.handlers.entry(event_type).or_insert_with(Vec::new).push(handler);
    }
    
    pub fn handle_event(&self, event: &Event) -> Vec<&EventHandler> {
        self.handlers.get(&event.event_type).map(|handlers| handlers.iter().collect()).unwrap_or_default()
    }
}
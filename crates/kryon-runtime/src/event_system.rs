// crates/kryon-runtime/src/event_system.rs
use kryon_core::{Element, ElementId};
use std::collections::HashMap;
use anyhow::Result;

#[derive(Debug)]
pub struct EventSystem {
    event_queue: Vec<UIEvent>,
}

#[derive(Debug, Clone)]
pub enum UIEvent {
    ElementClicked(ElementId),
    ElementHovered(ElementId),
    ElementFocused(ElementId),
    TextChanged(ElementId, String),
}

impl EventSystem {
    pub fn new() -> Self {
        Self {
            event_queue: Vec::new(),
        }
    }
    
    pub fn queue_event(&mut self, event: UIEvent) {
        self.event_queue.push(event);
    }
    
    pub fn update(&mut self, elements: &mut HashMap<ElementId, Element>) -> Result<()> {
        // Process all queued events
        let events: Vec<_> = self.event_queue.drain(..).collect();
        for event in events {
            self.process_event(event, elements)?;
        }
        Ok(())
    }
    
    fn process_event(&self, event: UIEvent, elements: &mut HashMap<ElementId, Element>) -> Result<()> {
        match event {
            UIEvent::ElementClicked(element_id) => {
                if let Some(element) = elements.get_mut(&element_id) {
                    tracing::debug!("Element clicked: {}", element.id);
                    // Handle click logic
                }
            }
            UIEvent::ElementHovered(element_id) => {
                if let Some(element) = elements.get_mut(&element_id) {
                    tracing::debug!("Element hovered: {}", element.id);
                    // Handle hover logic
                }
            }
            UIEvent::TextChanged(element_id, new_text) => {
                if let Some(element) = elements.get_mut(&element_id) {
                    element.text = new_text;
                    tracing::debug!("Text changed in element: {}", element.id);
                }
            }
            _ => {}
        }
        Ok(())
    }
}
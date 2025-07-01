// crates/kryon-runtime/src/lib.rs
use kryon_core::{KRBFile, Element, ElementId, InteractionState, EventType, load_krb_file};
use kryon_layout::{LayoutEngine, FlexboxLayoutEngine, LayoutResult};
use kryon_render::{ElementRenderer, CommandRenderer, InputEvent, MouseButton, KeyCode};
use glam::Vec2;
use std::collections::HashMap;
use std::time::{Duration, Instant};

pub mod backends;
pub mod event_system;
pub mod script_system;

pub use backends::*;
pub use event_system::*;
pub use script_system::*;

pub struct KryonApp<R: CommandRenderer> {
    // Core data
    krb_file: KRBFile,
    elements: HashMap<ElementId, Element>,
    
    // Systems
    layout_engine: FlexboxLayoutEngine,
    renderer: ElementRenderer<R>,
    event_system: EventSystem,
    script_system: ScriptSystem,
    
    // State
    layout_result: LayoutResult,
    viewport_size: Vec2,
    needs_layout: bool,
    needs_render: bool,
    
    // Timing
    last_frame_time: Instant,
    frame_count: u64,
}

impl<R: CommandRenderer> KryonApp<R> {
    pub fn new(krb_path: &str, renderer: R) -> anyhow::Result<Self> {
        let krb_file = load_krb_file(krb_path)?;
        let mut elements = krb_file.elements.clone();
        
        // Link parent-child relationships
        Self::link_element_hierarchy(&mut elements, &krb_file)?;
        
        let layout_engine = FlexboxLayoutEngine::new().with_debug(true);
        let renderer = ElementRenderer::new(renderer);
        let viewport_size = renderer.viewport_size();
        
        let event_system = EventSystem::new();
        let script_system = ScriptSystem::new();
        
        let mut app = Self {
            krb_file,
            elements,
            layout_engine,
            renderer,
            event_system,
            script_system,
            layout_result: LayoutResult {
                computed_positions: HashMap::new(),
                computed_sizes: HashMap::new(),
            },
            viewport_size,
            needs_layout: true,
            needs_render: true,
            last_frame_time: Instant::now(),
            frame_count: 0,
        };
        
        // Initialize scripts
        app.script_system.load_scripts(&app.krb_file.scripts)?;
        
        // >>>>>>>>> ADD THIS LINE HERE <<<<<<<<<<<
        println!("[KryonApp::new] Loaded KRB. Root element ID: {:?}", app.krb_file.root_element_id);


        Ok(app)
    }
    
    fn link_element_hierarchy(
        elements: &mut HashMap<ElementId, Element>,
        _krb_file: &KRBFile,
    ) -> anyhow::Result<()> {
        // TODO: Implement proper parent-child relationship parsing from KRB format
        // For now, since we don't have actual child data from KRB,
        // just leave the hierarchy as parsed (empty children lists)
        Ok(())
    }
    
    pub fn update(&mut self, delta_time: Duration) -> anyhow::Result<()> {
        // Update script system
        self.script_system.update(delta_time, &mut self.elements)?;
        
        // Process events
        self.event_system.update(&mut self.elements)?;
        
        // Update layout if needed
        if self.needs_layout {
            self.update_layout()?;
            self.needs_layout = false;
            self.needs_render = true;
        }
        
        Ok(())
    }
    
    pub fn render(&mut self) -> anyhow::Result<()> {
        if !self.needs_render {
            return Ok(());
        }
        
        if let Some(root_id) = self.krb_file.root_element_id {
            let clear_color = glam::Vec4::new(0.1, 0.1, 0.1, 1.0); // Dark gray
            
            self.renderer.render_frame(
                &self.elements,
                &self.layout_result,
                root_id,
                clear_color,
            )?;
        }
        
        self.needs_render = false;
        self.frame_count += 1;
        
        // Update timing
        let now = Instant::now();
        let frame_time = now.duration_since(self.last_frame_time);
        self.last_frame_time = now;
        
        // Log FPS occasionally
        if self.frame_count % 60 == 0 {
            let fps = 1.0 / frame_time.as_secs_f32();
            tracing::debug!("FPS: {:.1}", fps);
        }
        
        Ok(())
    }
    
    pub fn handle_input(&mut self, event: InputEvent) -> anyhow::Result<()> {
        match event {
            InputEvent::Resize { size } => {
                self.viewport_size = size;
                self.renderer.resize(size)?;
                self.needs_layout = true;
            }
            InputEvent::MouseMove { position } => {
                self.handle_mouse_move(position)?;
            }
            InputEvent::MousePress { position, button } => {
                self.handle_mouse_press(position, button)?;
            }
            InputEvent::MouseRelease { position, button } => {
                self.handle_mouse_release(position, button)?;
            }
            InputEvent::KeyPress { key, modifiers } => {
                self.handle_key_press(key, modifiers)?;
            }
            _ => {}
        }
        
        Ok(())
    }
    
    // In fn update_layout(&mut self) -> anyhow::Result<()>

fn update_layout(&mut self) -> anyhow::Result<()> {
    if let Some(root_id) = self.krb_file.root_element_id {
        // >>>>>>>>> ADD THIS PRINTLN <<<<<<<<<<<
        println!("[KryonApp] Running layout computation...");

        self.layout_result = self.layout_engine.compute_layout(
            &self.elements,
            root_id,
            self.viewport_size,
        );

        // >>>>>>>>> AND ADD THIS ONE <<<<<<<<<<<
        println!("[KryonApp] Layout computed. Positions: {}, Sizes: {}",
            self.layout_result.computed_positions.len(),
            self.layout_result.computed_sizes.len()
        );
    }
    Ok(())
}

    fn handle_mouse_move(&mut self, position: Vec2) -> anyhow::Result<()> {
        let hovered_element = self.find_element_at_position(position);
        
        // Update hover states
        for (element_id, element) in self.elements.iter_mut() {
            let should_hover = Some(*element_id) == hovered_element;
            let was_hovering = element.current_state == InteractionState::Hover;
            
            if should_hover && !was_hovering {
                element.current_state = InteractionState::Hover;
                self.needs_render = true;
                
                // Trigger hover event
                if let Some(handler) = element.event_handlers.get(&EventType::Hover) {
                    self.script_system.call_function(handler, vec![])?;
                }
            } else if !should_hover && was_hovering {
                element.current_state = InteractionState::Normal;
                self.needs_render = true;
            }
        }
        
        Ok(())
    }
    
    fn handle_mouse_press(&mut self, position: Vec2, button: MouseButton) -> anyhow::Result<()> {
        if button == MouseButton::Left {
            if let Some(element_id) = self.find_element_at_position(position) {
                if let Some(element) = self.elements.get_mut(&element_id) {
                    element.current_state = InteractionState::Active;
                    self.needs_render = true;
                }
            }
        }
        Ok(())
    }
    
    fn handle_mouse_release(&mut self, position: Vec2, button: MouseButton) -> anyhow::Result<()> {
        if button == MouseButton::Left {
            if let Some(element_id) = self.find_element_at_position(position) {
                if let Some(element) = self.elements.get_mut(&element_id) {
                    element.current_state = InteractionState::Hover;
                    self.needs_render = true;
                    
                    // Trigger click event
                    if let Some(handler) = element.event_handlers.get(&EventType::Click) {
                        self.script_system.call_function(handler, vec![])?;
                    }
                }
            }
        }
        Ok(())
    }
    
    fn handle_key_press(&mut self, key: KeyCode, _modifiers: kryon_render::KeyModifiers) -> anyhow::Result<()> {
        // Handle global key events
        match key {
            KeyCode::Escape => {
                // Could trigger app exit
            }
            _ => {}
        }
        Ok(())
    }
    
    fn find_element_at_position(&self, position: Vec2) -> Option<ElementId> {
        // Find the topmost element at the given position
        for (element_id, element) in &self.elements {
            if !element.visible {
                continue;
            }
            
            let element_pos = self.layout_result.computed_positions
                .get(element_id)
                .copied()
                .unwrap_or(element.position);
            let element_size = self.layout_result.computed_sizes
                .get(element_id)
                .copied()
                .unwrap_or(element.size);
            
            if position.x >= element_pos.x
                && position.x <= element_pos.x + element_size.x
                && position.y >= element_pos.y
                && position.y <= element_pos.y + element_size.y
            {
                return Some(*element_id);
            }
        }
        None
    }
    
    pub fn get_element(&self, id: &str) -> Option<&Element> {
        self.elements.iter()
            .find(|(_, element)| element.id == id)
            .map(|(_, element)| element)
    }
    
    pub fn get_element_mut(&mut self, id: &str) -> Option<&mut Element> {
        self.elements.iter_mut()
            .find(|(_, element)| element.id == id)
            .map(|(_, element)| element)
    }
    
    pub fn viewport_size(&self) -> Vec2 {
        self.viewport_size
    }
    
    pub fn mark_needs_layout(&mut self) {
        self.needs_layout = true;
    }
    
    pub fn mark_needs_render(&mut self) {
        self.needs_render = true;
    }
    
    pub fn renderer(&self) -> &ElementRenderer<R> {
        &self.renderer
    }
    
    pub fn renderer_mut(&mut self) -> &mut ElementRenderer<R> {
        &mut self.renderer
    }
}
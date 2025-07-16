// crates/kryon-runtime/src/lib.rs

use kryon_core::{
    KRBFile, Element, ElementId, InteractionState, EventType, load_krb_file,
    StyleComputer,
};
use kryon_layout::{LayoutEngine, TaffyLayoutEngine, LayoutResult};
use kryon_render::{ElementRenderer, CommandRenderer, InputEvent, MouseButton, KeyCode};
use glam::Vec2;
use std::collections::HashMap;
use std::time::{Duration, Instant};

pub mod backends;
pub mod event_system;
pub mod script;
pub mod template_engine;
pub mod shared_data;

pub use backends::*;
pub use event_system::*;
pub use script::ScriptSystem;
pub use template_engine::*;
pub use shared_data::*;

pub struct KryonApp<R: CommandRenderer> {
    // Core data
    krb_file: KRBFile,
    elements: HashMap<ElementId, Element>,
    
    // Systems
    _style_computer: StyleComputer, 
    layout_engine: Box<dyn LayoutEngine>,
    renderer: ElementRenderer<R>,
    event_system: EventSystem,
    script_system: ScriptSystem,
    template_engine: TemplateEngine,
    
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
        Self::new_with_layout_engine(krb_path, renderer, None)
    }
    
    pub fn new_with_layout_engine(krb_path: &str, renderer: R, layout_engine: Option<Box<dyn LayoutEngine>>) -> anyhow::Result<Self> {
        let krb_file = load_krb_file(krb_path)?;
        Self::new_with_krb(krb_file, renderer, layout_engine)
    }
    
    pub fn new_with_krb(krb_file: KRBFile, renderer: R, layout_engine: Option<Box<dyn LayoutEngine>>) -> anyhow::Result<Self> {
        let mut elements = krb_file.elements.clone();
        
        let style_computer = StyleComputer::new(&elements, &krb_file.styles);

        // Link parent-child relationships
        Self::link_element_hierarchy(&mut elements, &krb_file)?;
        
        
        // Use TaffyLayoutEngine as the core layout system
        let layout_engine: Box<dyn LayoutEngine> = layout_engine.unwrap_or_else(|| {
            Box::new(TaffyLayoutEngine::new())
        });
        let renderer = ElementRenderer::new(renderer, style_computer.clone());
        let viewport_size = renderer.viewport_size();
        
        let event_system = EventSystem::new();
        let script_system = ScriptSystem::new()?;
        let template_engine = TemplateEngine::new(&krb_file);
        
        let mut app = Self {
            krb_file,
            elements,
            _style_computer: style_computer,
            layout_engine,
            renderer,
            event_system,
            script_system,
            template_engine,
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
        
        // Initialize the script system with KRB file data
        app.script_system.initialize(&app.krb_file, &app.elements)?;
        
        // Load compiled scripts from KRB file
        app.script_system.load_compiled_scripts(&app.krb_file.scripts)?;
        
        // Initialize template variables in the script system
        // Always initialize template variables from KRB data to ensure script access
        if app.template_engine.has_bindings() {
            tracing::info!("🔍 [INIT_DEBUG] Template engine has bindings, using template variables");
            let template_vars = app.template_engine.get_variables().clone();
            app.script_system.initialize_template_variables(&template_vars)?;
        } else {
            tracing::info!("🔍 [INIT_DEBUG] Template engine has no bindings, extracting variables from KRB");
            // Extract template variables directly from KRB data
            let mut vars = std::collections::HashMap::new();
            for var in &app.krb_file.template_variables {
                vars.insert(var.name.clone(), var.default_value.clone());
            }
            app.script_system.initialize_template_variables(&vars)?;
        }
        
        // Apply any initial changes set by scripts during initialization
        let changes_applied = app.script_system.apply_pending_changes(&mut app.elements)?;
        if changes_applied {
            tracing::info!("Applied initial changes from scripts");
        }
        
        // Initialize template variables (apply default values to elements)
        app.initialize_template_variables()?;
        
        // Execute script initialization functions now that template variables are ready
        app.script_system.execute_init_functions()?;
        
        // Force initial layout computation
        app.update_layout()?;
        app.needs_layout = false; // Reset after initial layout
        
        Ok(app)
    }
    
    fn link_element_hierarchy(
        _elements: &mut HashMap<ElementId, Element>,
        _krb_file: &KRBFile,
    ) -> anyhow::Result<()> {
        // TODO: Implement proper parent-child relationship parsing from KRB format
        // For now, since we don't have actual child data from KRB,
        // just leave the hierarchy as parsed (empty children lists)
        Ok(())
    }
    
    pub fn update(&mut self, _delta_time: Duration) -> anyhow::Result<()> {
        // Get pending changes from scripts and apply both DOM and template variable changes
        let pending_changes = self.script_system.get_pending_changes()?;
        
        // Apply template variable changes first
        if let Some(template_changes) = pending_changes.get("template_variables") {
            for (name, value) in &template_changes.data {
                self.set_template_variable(name, value)?;
            }
            tracing::debug!("Applied {} template variable changes", template_changes.data.len());
        }
        
        // Apply DOM changes from the same change set
        let changes_applied = self.script_system.apply_pending_dom_changes(&mut self.elements, &pending_changes)?;
        
        // Clear changes after applying them
        self.script_system.clear_pending_changes()?;
        
        if changes_applied {
            self.needs_render = true;
        }
        
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
        
        // Note: Forced hover test removed - hover system confirmed working
        
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
        self.layout_result = self.layout_engine.compute_layout(
            &self.elements,
            root_id,
            self.viewport_size,
        );
        
        // Apply computed layout results back to element positions and sizes
        for (&element_id, computed_position) in &self.layout_result.computed_positions {
            if let Some(element) = self.elements.get_mut(&element_id) {
                eprintln!("[LAYOUT_APPLY] Element {}: applying computed position {:?} (was {:?})", 
                    element_id, computed_position, element.position);
                element.position = *computed_position;
            }
        }
        
        for (&element_id, computed_size) in &self.layout_result.computed_sizes {
            if let Some(element) = self.elements.get_mut(&element_id) {
                // Debug: Log size application
                eprintln!("[LAYOUT_APPLY] Element {}: applying computed size {:?} (was {:?})", 
                    element_id, computed_size, element.size);
                element.size = *computed_size;
            }
        }
    }
    Ok(())
}

    fn handle_mouse_move(&mut self, position: Vec2) -> anyhow::Result<()> {
        let hovered_element = self.find_element_at_position(position);
        
        // Determine the cursor type for the hovered element
        let cursor_type = if let Some(element_id) = hovered_element {
            if let Some(element) = self.elements.get(&element_id) {
                element.cursor
            } else {
                kryon_core::CursorType::Default
            }
        } else {
            kryon_core::CursorType::Default
        };
        
        // Update the cursor through the renderer
        self.renderer.backend_mut().set_cursor(cursor_type);
        
        // Update hover states (but preserve checked state)
        for (element_id, element) in self.elements.iter_mut() {
            let should_hover = Some(*element_id) == hovered_element;
            let was_hovering = element.current_state == InteractionState::Hover;
            let is_checked = element.current_state == InteractionState::Checked;
            
            if should_hover && !was_hovering && !is_checked {
                // Only set hover if not already in checked state
                element.current_state = InteractionState::Hover;
                self.needs_render = true;
                
                // Trigger hover event
                if let Some(handler) = element.event_handlers.get(&EventType::Hover) {
                    use kryon_core::PropertyValue;
                    self.script_system.call_function(handler, vec![])?;
                }
            } else if !should_hover && was_hovering && !is_checked {
                // Only reset to normal if not in checked state
                element.current_state = InteractionState::Normal;
                self.needs_render = true;
            }
            // If element is checked, preserve the checked state regardless of hover
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
                // Trigger click event first, before changing any states
                if let Some(element) = self.elements.get(&element_id) {
                    if let Some(handler) = element.event_handlers.get(&EventType::Click) {
                        // Call the click handler function
                        use kryon_core::PropertyValue;
                        self.script_system.call_function(handler, vec![])?;
                        
                        // Apply any pending changes from scripts
                        let changes_applied = self.script_system.apply_pending_changes(&mut self.elements)?;
                        
                        // Apply template variable changes from scripts
                        let pending_changes = self.script_system.get_pending_changes()?;
                        let template_variable_changes = if let Some(template_changes) = pending_changes.get("template_variables") {
                            for (name, value) in &template_changes.data {
                                self.set_template_variable(name, value)?;
                            }
                            !template_changes.data.is_empty()
                        } else {
                            false
                        };
                        
                        if changes_applied || template_variable_changes {
                            tracing::info!("Changes applied, triggering re-render");
                            self.needs_render = true;
                            
                            // Force layout update for visibility changes and template variable changes
                            // This ensures that elements become visible/invisible immediately and template variables update
                            if template_variable_changes {
                                self.update_layout()?;
                                self.needs_layout = false;
                                tracing::info!("🚀 [SCRIPT_IMMEDIATE] Immediate layout update applied for template changes");
                            }
                        }
                        
                        // After script changes are applied, set hover state only for non-checked elements
                        if let Some(element) = self.elements.get_mut(&element_id) {
                            if element.current_state != InteractionState::Checked {
                                element.current_state = InteractionState::Hover;
                                self.needs_render = true;
                            }
                        }
                    } else {
                        // No click handler, just set hover state
                        if let Some(element) = self.elements.get_mut(&element_id) {
                            element.current_state = InteractionState::Hover;
                            self.needs_render = true;
                        }
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
        let mut found_elements = Vec::new();
        
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
                found_elements.push(*element_id);
            }
        }
        
        // Return the highest element ID (topmost)
        found_elements.into_iter().max()
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
    
    // Template variable methods
    
    /// Set a template variable and update affected elements
    pub fn set_template_variable(&mut self, name: &str, value: &str) -> anyhow::Result<()> {
        
        // Force update the template variable (ignore change detection for now)
        self.template_engine.set_variable(name, value);
        
        // Always update elements if we have bindings for this variable
        let bindings_for_var = self.template_engine.get_bindings_for_variable(name);
        let bindings_count = bindings_for_var.len();
        
        if !bindings_for_var.is_empty() {
            // Update the elements with new template values
            self.template_engine.update_elements(&mut self.elements);
            
            // Mark layout and render as needed
            self.mark_needs_layout();
            self.mark_needs_render();
            
            tracing::info!("Template variable '{}' updated to '{}', affected {} elements", 
                name, value, bindings_count);
        }
        
        Ok(())
    }
    
    /// Get a template variable value
    pub fn get_template_variable(&self, name: &str) -> Option<&str> {
        self.template_engine.get_variable(name)
    }
    
    /// Get all template variables
    pub fn get_template_variables(&self) -> &HashMap<String, String> {
        self.template_engine.get_variables()
    }
    
    /// Get all template variable names
    pub fn get_template_variable_names(&self) -> Vec<String> {
        self.template_engine.get_variable_names()
    }
    
    /// Check if template variables are available
    pub fn has_template_variables(&self) -> bool {
        self.template_engine.has_bindings()
    }
    
    /// Initialize template variables (apply initial values to elements)
    pub fn initialize_template_variables(&mut self) -> anyhow::Result<()> {
        if self.template_engine.has_bindings() {
            self.template_engine.update_elements(&mut self.elements);
            self.mark_needs_layout();
            self.mark_needs_render();
            tracing::info!("Template variables initialized");
        }
        Ok(())
    }
}
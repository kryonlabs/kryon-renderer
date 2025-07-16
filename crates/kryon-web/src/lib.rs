//! Kryon Web Renderer
//! 
//! This crate provides WebAssembly-based rendering backends for Kryon applications,
//! allowing them to run in web browsers.

use wasm_bindgen::prelude::*;
use web_sys::console;

mod canvas_renderer;
mod dom_renderer; 
mod event_handler;
mod asset_loader;
mod utils;

#[cfg(feature = "webgpu")]
mod webgpu_renderer;

#[cfg(feature = "winit")]
mod winit_integration;

#[cfg(feature = "webgpu")]
mod shaders;
mod texture_manager;
mod animation;
mod profiler;

#[cfg(test)]
mod tests;

pub use canvas_renderer::CanvasRenderer;
pub use dom_renderer::DomRenderer;
pub use event_handler::WebEventHandler;
pub use asset_loader::WebAssetLoader;

#[cfg(feature = "webgpu")]
pub use webgpu_renderer::WebGpuRenderer;

#[cfg(feature = "winit")]
pub use winit_integration::WinitWebBridge;

pub use texture_manager::{TextureManager, TextureDescriptor, TextureFormat, TextureUsage};
pub use animation::{AnimationSystem, Animation, Transition, AnimationValue, EasingFunction};
pub use profiler::{PerformanceProfiler, PerformanceMetrics, FrameStats};

/// Initialize the web renderer with panic hooks and logging
#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
    
    // Initialize tracing for web
    tracing_web::set_as_global_default();
    
    console::log_1(&"Kryon Web Renderer initialized".into());
}

/// Main entry point for Kryon web applications
#[wasm_bindgen]
pub struct KryonWebApp {
    canvas_renderer: Option<CanvasRenderer>,
    dom_renderer: Option<DomRenderer>,
    #[cfg(feature = "webgpu")]
    webgpu_renderer: Option<WebGpuRenderer>,
    event_handler: WebEventHandler,
    asset_loader: WebAssetLoader,
    animation_system: AnimationSystem,
    texture_manager: Option<TextureManager>,
    profiler: PerformanceProfiler,
}

#[wasm_bindgen]
impl KryonWebApp {
    /// Create a new Kryon web application
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            canvas_renderer: None,
            dom_renderer: None,
            #[cfg(feature = "webgpu")]
            webgpu_renderer: None,
            event_handler: WebEventHandler::new(),
            asset_loader: WebAssetLoader::new(),
            animation_system: AnimationSystem::new(),
            texture_manager: None,
            profiler: PerformanceProfiler::new(),
        }
    }
    
    /// Initialize canvas rendering mode
    #[wasm_bindgen]
    pub fn init_canvas(&mut self, canvas_id: &str) -> Result<(), JsValue> {
        let canvas_renderer = CanvasRenderer::new(canvas_id)?;
        self.canvas_renderer = Some(canvas_renderer);
        Ok(())
    }
    
    /// Initialize DOM rendering mode
    #[wasm_bindgen]
    pub fn init_dom(&mut self, container_id: &str) -> Result<(), JsValue> {
        let dom_renderer = DomRenderer::new(container_id)?;
        self.dom_renderer = Some(dom_renderer);
        Ok(())
    }
    
    /// Initialize WebGPU rendering mode
    #[cfg(feature = "webgpu")]
    #[wasm_bindgen]
    pub async fn init_webgpu(&mut self, canvas_id: &str) -> Result<(), JsValue> {
        let webgpu_renderer = WebGpuRenderer::new(canvas_id).await?;
        self.webgpu_renderer = Some(webgpu_renderer);
        Ok(())
    }
    
    /// Check if WebGPU is available
    #[cfg(feature = "webgpu")]
    #[wasm_bindgen]
    pub fn is_webgpu_available() -> bool {
        WebGpuRenderer::is_available()
    }
    
    /// Load and run a KRB file
    #[wasm_bindgen]
    pub async fn load_krb(&mut self, krb_data: &[u8]) -> Result<(), JsValue> {
        console::log_1(&format!("Loading KRB file, size: {} bytes", krb_data.len()).into());
        
        // Parse KRB data
        // TODO: Implement KRB parsing for web
        
        Ok(())
    }
    
    /// Render a single frame
    #[wasm_bindgen]
    pub fn render(&mut self, timestamp: f64) -> Result<(), JsValue> {
        // Begin frame profiling
        self.profiler.begin_frame(timestamp);
        
        // Update animation system
        self.profiler.begin_timer("animation");
        self.animation_system.update(timestamp);
        self.profiler.end_timer("animation");
        
        // Begin render timing
        self.profiler.begin_timer("render");
        
        // Clear the renderer
        if let Some(canvas_renderer) = &mut self.canvas_renderer {
            canvas_renderer.clear(glam::Vec4::new(0.95, 0.95, 0.95, 1.0))?;
            self.profiler.increment_counter("draw_calls", 1);
        }
        
        if let Some(dom_renderer) = &mut self.dom_renderer {
            dom_renderer.clear()?;
        }
        
        #[cfg(feature = "webgpu")]
        if let Some(webgpu_renderer) = &mut self.webgpu_renderer {
            webgpu_renderer.clear(glam::Vec4::new(0.95, 0.95, 0.95, 1.0))?;
            self.profiler.increment_counter("draw_calls", 1);
        }
        
        // TODO: Render actual content from KRB data with animation values
        
        // End render timing
        self.profiler.end_timer("render");
        
        // End frame profiling
        self.profiler.end_frame(timestamp);
        
        Ok(())
    }
    
    /// Start the render loop
    #[wasm_bindgen]
    pub fn start_render_loop(&mut self) -> Result<(), JsValue> {
        console::log_1(&"Starting render loop".into());
        
        // The render loop is handled by JavaScript's requestAnimationFrame
        // This method is kept for compatibility but the actual loop is in HTML
        
        Ok(())
    }
    
    /// Create a fade in animation
    #[wasm_bindgen]
    pub fn animate_fade_in(&mut self, element_id: &str, duration: f64) -> String {
        let animation = Animation::fade_in(element_id, duration);
        self.animation_system.create_animation(animation)
    }
    
    /// Create a slide in animation
    #[wasm_bindgen]
    pub fn animate_slide_in(&mut self, element_id: &str, from_x: f32, from_y: f32, to_x: f32, to_y: f32, duration: f64) -> String {
        let from = glam::Vec2::new(from_x, from_y);
        let to = glam::Vec2::new(to_x, to_y);
        let animation = Animation::slide_in(element_id, from, to, duration);
        self.animation_system.create_animation(animation)
    }
    
    /// Create a pulse animation
    #[wasm_bindgen]
    pub fn animate_pulse(&mut self, element_id: &str, duration: f64) -> String {
        let animation = Animation::pulse(element_id, duration);
        self.animation_system.create_animation(animation)
    }
    
    /// Animate a property to a target value
    #[wasm_bindgen]
    pub fn animate_property(&mut self, element_id: &str, property: &str, target_value: f32, duration: f64) -> String {
        let animation_value = AnimationValue::Float(target_value);
        let easing = EasingFunction::EaseOut;
        self.animation_system.animate_property(element_id, property, animation_value, duration, easing)
    }
    
    /// Play an animation
    #[wasm_bindgen]
    pub fn play_animation(&mut self, animation_id: &str) {
        self.animation_system.play_animation(animation_id);
    }
    
    /// Pause an animation
    #[wasm_bindgen]
    pub fn pause_animation(&mut self, animation_id: &str) {
        self.animation_system.pause_animation(animation_id);
    }
    
    /// Stop an animation
    #[wasm_bindgen]
    pub fn stop_animation(&mut self, animation_id: &str) {
        self.animation_system.stop_animation(animation_id);
    }
    
    /// Get the number of active animations
    #[wasm_bindgen]
    pub fn get_animation_count(&self) -> usize {
        self.animation_system.get_animation_count()
    }
    
    /// Get the number of active transitions
    #[wasm_bindgen]
    pub fn get_transition_count(&self) -> usize {
        self.animation_system.get_transition_count()
    }
    
    /// Check if an element property is being animated
    #[wasm_bindgen]
    pub fn is_animating(&self, element_id: &str, property: &str) -> bool {
        self.animation_system.is_animating(element_id, property)
    }
    
    /// Get performance statistics
    #[wasm_bindgen]
    pub fn get_performance_stats(&self) -> JsValue {
        self.profiler.export_data()
    }
    
    /// Get performance chart data
    #[wasm_bindgen]
    pub fn get_performance_chart(&self) -> JsValue {
        self.profiler.create_performance_chart()
    }
    
    /// Enable performance profiling
    #[wasm_bindgen]
    pub fn enable_profiling(&mut self) {
        self.profiler.enable();
    }
    
    /// Disable performance profiling
    #[wasm_bindgen]
    pub fn disable_profiling(&mut self) {
        self.profiler.disable();
    }
    
    /// Reset performance statistics
    #[wasm_bindgen]
    pub fn reset_performance_stats(&mut self) {
        self.profiler.reset();
    }
    
    /// Log performance statistics to console
    #[wasm_bindgen]
    pub fn log_performance(&self) {
        self.profiler.log_performance();
    }
}
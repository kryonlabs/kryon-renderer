//! Web-specific WGPU renderer implementation

#[cfg(feature = "web")]
use wasm_bindgen::prelude::*;
#[cfg(feature = "web")]
use web_sys::{HtmlCanvasElement, Window};

use crate::WgpuRenderer;
use kryon_core::Element;
use kryon_render::{Renderer, RenderCommand, RenderResult, RenderError};
use wgpu::{Instance, Surface, Device, Queue, SurfaceConfiguration, TextureFormat};
use glam::Vec2;

#[derive(Debug, thiserror::Error)]
pub enum WebWgpuError {
    #[error("Initialization failed: {0}")]
    InitializationError(String),
    #[error("Render error: {0}")]
    RenderError(String),
}

/// Web-specific WGPU renderer that works with HTML Canvas
pub struct WebWgpuRenderer {
    #[cfg(feature = "web")]
    canvas: HtmlCanvasElement,
    
    instance: Instance,
    surface: Surface,
    device: Device,
    queue: Queue,
    config: SurfaceConfiguration,
    size: Vec2,
    
    // Reuse the core WGPU renderer for actual rendering
    core_renderer: WgpuRenderer,
}

impl WebWgpuRenderer {
    /// Create a new web WGPU renderer from a canvas element
    #[cfg(feature = "web")]
    pub async fn new(canvas_id: &str) -> Result<Self, WebWgpuError> {
        // Get the canvas element
        let window = web_sys::window().ok_or(WebWgpuError::InitializationError("No window object".to_string()))?;
        let document = window.document().ok_or(WebWgpuError::InitializationError("No document object".to_string()))?;
        let canvas = document
            .get_element_by_id(canvas_id)
            .ok_or(WebWgpuError::InitializationError("Canvas element not found".to_string()))?
            .dyn_into::<HtmlCanvasElement>()
            .map_err(|_| WebWgpuError::InitializationError("Element is not a canvas".to_string()))?;
        
        // Set up WebGPU instance
        let instance = Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::BROWSER_WEBGPU,
            ..Default::default()
        });
        
        // Create surface from canvas
        let surface = instance.create_surface_from_canvas(&canvas)
            .map_err(|e| WgpuRendererError::InitializationError(format!("Failed to create surface: {:?}", e)))?;
        
        // Request adapter
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await
            .ok_or(WgpuRendererError::InitializationError("Failed to find adapter".to_string()))?;
        
        // Request device and queue
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits::downlevel_webgl2_defaults(),
                },
                None,
            )
            .await
            .map_err(|e| WgpuRendererError::InitializationError(format!("Failed to request device: {:?}", e)))?;
        
        // Get canvas size
        let size = Vec2::new(canvas.width() as f32, canvas.height() as f32);
        
        // Configure surface
        let config = SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: TextureFormat::Bgra8UnormSrgb,
            width: canvas.width(),
            height: canvas.height(),
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
        };
        
        surface.configure(&device, &config);
        
        // Create core renderer
        let core_renderer = WgpuRenderer::new_with_device_and_queue(
            device.clone(),
            queue.clone(),
            config.format,
            size,
        )?;
        
        Ok(Self {
            canvas,
            instance,
            surface,
            device,
            queue,
            config,
            size,
            core_renderer,
        })
    }
    
    /// Resize the renderer
    pub fn resize(&mut self, new_size: Vec2) {
        self.size = new_size;
        self.config.width = new_size.x as u32;
        self.config.height = new_size.y as u32;
        
        #[cfg(feature = "web")]
        {
            self.canvas.set_width(new_size.x as u32);
            self.canvas.set_height(new_size.y as u32);
        }
        
        self.surface.configure(&self.device, &self.config);
        self.core_renderer.resize(new_size);
    }
    
    /// Get the current size
    pub fn size(&self) -> Vec2 {
        self.size
    }
    
    /// Begin a render pass
    pub fn begin_frame(&mut self) -> Result<(), WgpuRendererError> {
        self.core_renderer.begin_frame()
    }
    
    /// End the render pass and present
    pub fn end_frame(&mut self) -> Result<(), WgpuRendererError> {
        self.core_renderer.end_frame()
    }
    
    /// Execute a render command
    pub fn execute_render_command(&mut self, command: &RenderCommand) -> Result<(), WgpuRendererError> {
        self.core_renderer.execute_render_command(command)
    }
    
    /// Clear the screen
    pub fn clear(&mut self, color: glam::Vec4) -> Result<(), WgpuRendererError> {
        self.core_renderer.clear(color)
    }
    
    /// Check if WebGPU is available
    #[cfg(feature = "web")]
    pub fn is_webgpu_available() -> bool {
        let window = web_sys::window().unwrap();
        js_sys::Reflect::has(&window, &JsValue::from_str("navigator")).unwrap_or(false) &&
        js_sys::Reflect::has(&js_sys::Reflect::get(&window, &JsValue::from_str("navigator")).unwrap(), &JsValue::from_str("gpu")).unwrap_or(false)
    }
    
    /// Create a fallback WebGL-based renderer if WebGPU is not available
    #[cfg(feature = "web")]
    pub async fn new_webgl_fallback(canvas_id: &str) -> Result<Self, WgpuRendererError> {
        // Get the canvas element
        let window = web_sys::window().ok_or(WgpuRendererError::InitializationError("No window object".to_string()))?;
        let document = window.document().ok_or(WgpuRendererError::InitializationError("No document object".to_string()))?;
        let canvas = document
            .get_element_by_id(canvas_id)
            .ok_or(WgpuRendererError::InitializationError("Canvas element not found".to_string()))?
            .dyn_into::<HtmlCanvasElement>()
            .map_err(|_| WgpuRendererError::InitializationError("Element is not a canvas".to_string()))?;
        
        // Set up WebGL instance
        let instance = Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::GL,
            ..Default::default()
        });
        
        // Create surface from canvas
        let surface = instance.create_surface_from_canvas(&canvas)
            .map_err(|e| WgpuRendererError::InitializationError(format!("Failed to create surface: {:?}", e)))?;
        
        // Request adapter
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await
            .ok_or(WgpuRendererError::InitializationError("Failed to find WebGL adapter".to_string()))?;
        
        // Request device and queue
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits::downlevel_webgl2_defaults(),
                },
                None,
            )
            .await
            .map_err(|e| WgpuRendererError::InitializationError(format!("Failed to request device: {:?}", e)))?;
        
        // Get canvas size
        let size = Vec2::new(canvas.width() as f32, canvas.height() as f32);
        
        // Configure surface
        let config = SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: TextureFormat::Rgba8UnormSrgb,
            width: canvas.width(),
            height: canvas.height(),
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
        };
        
        surface.configure(&device, &config);
        
        // Create core renderer
        let core_renderer = WgpuRenderer::new_with_device_and_queue(
            device.clone(),
            queue.clone(),
            config.format,
            size,
        )?;
        
        Ok(Self {
            canvas,
            instance,
            surface,
            device,
            queue,
            config,
            size,
            core_renderer,
        })
    }
}

impl Renderer for WebWgpuRenderer {
    fn render(&mut self, elements: &[Element]) -> RenderResult<()> {
        self.core_renderer.render(elements)
    }
    
    fn resize(&mut self, new_size: Vec2) {
        self.resize(new_size);
    }
    
    fn clear(&mut self, color: glam::Vec4) -> RenderResult<()> {
        self.clear(color).map_err(|e| RenderError::BackendError(format!("WGPU error: {:?}", e)))
    }
}
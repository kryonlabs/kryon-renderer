//! WebGPU renderer for web browsers

use wasm_bindgen::prelude::*;
use web_sys::{HtmlCanvasElement, GpuCanvasContext, GpuDevice, GpuQueue, GpuRenderPassEncoder};
use kryon_render::{Renderer, RenderResult, RenderError, RenderCommand};
use kryon_core::Element;
use kryon_layout::LayoutResult;
use glam::{Vec2, Vec4};

pub struct WebGpuRenderer {
    canvas: HtmlCanvasElement,
    device: GpuDevice,
    queue: GpuQueue,
    context: GpuCanvasContext,
    size: Vec2,
}

impl WebGpuRenderer {
    pub async fn new(canvas_id: &str) -> Result<Self, JsValue> {
        // Get the canvas element
        let window = web_sys::window().ok_or("No window object")?;
        let document = window.document().ok_or("No document object")?;
        let canvas = document
            .get_element_by_id(canvas_id)
            .ok_or("Canvas element not found")?
            .dyn_into::<HtmlCanvasElement>()
            .map_err(|_| "Element is not a canvas")?;
        
        // Check if WebGPU is available
        let navigator = window.navigator();
        let gpu = js_sys::Reflect::get(&navigator, &JsValue::from_str("gpu"))
            .map_err(|_| "WebGPU not available")?;
        
        if gpu.is_undefined() {
            return Err("WebGPU not supported in this browser".into());
        }
        
        // Get WebGPU adapter
        let adapter_promise = js_sys::Reflect::get(&gpu, &JsValue::from_str("requestAdapter"))
            .map_err(|_| "Failed to get requestAdapter")?;
        
        let adapter_promise = js_sys::Function::from(adapter_promise)
            .call0(&gpu)
            .map_err(|_| "Failed to call requestAdapter")?;
        
        let adapter = wasm_bindgen_futures::JsFuture::from(js_sys::Promise::from(adapter_promise))
            .await
            .map_err(|_| "Failed to get WebGPU adapter")?;
        
        // Get WebGPU device
        let device_promise = js_sys::Reflect::get(&adapter, &JsValue::from_str("requestDevice"))
            .map_err(|_| "Failed to get requestDevice")?;
        
        let device_promise = js_sys::Function::from(device_promise)
            .call0(&adapter)
            .map_err(|_| "Failed to call requestDevice")?;
        
        let device = wasm_bindgen_futures::JsFuture::from(js_sys::Promise::from(device_promise))
            .await
            .map_err(|_| "Failed to get WebGPU device")?;
        
        let device = device.dyn_into::<GpuDevice>()
            .map_err(|_| "Failed to cast to GpuDevice")?;
        
        // Get the queue
        let queue = device.queue();
        
        // Get the canvas context
        let context = canvas
            .get_context("webgpu")
            .map_err(|_| "Failed to get WebGPU context")?
            .ok_or("WebGPU context not available")?
            .dyn_into::<GpuCanvasContext>()
            .map_err(|_| "Failed to cast to GpuCanvasContext")?;
        
        let size = Vec2::new(canvas.width() as f32, canvas.height() as f32);
        
        // Configure the context
        let config = js_sys::Object::new();
        js_sys::Reflect::set(&config, &JsValue::from_str("device"), &device)?;
        js_sys::Reflect::set(&config, &JsValue::from_str("format"), &JsValue::from_str("bgra8unorm"))?;
        js_sys::Reflect::set(&config, &JsValue::from_str("alphaMode"), &JsValue::from_str("premultiplied"))?;
        
        context.configure(&config);
        
        Ok(Self {
            canvas,
            device,
            queue,
            context,
            size,
        })
    }
    
    pub fn execute_render_command(&mut self, command: &RenderCommand) -> Result<(), JsValue> {
        match command {
            RenderCommand::DrawRect { position, size, color, .. } => {
                // Basic rectangle rendering using WebGPU
                // This is a simplified implementation
                web_sys::console::log_1(&format!("WebGPU: Drawing rect at {:?}, size {:?}, color {:?}", position, size, color).into());
            }
            
            RenderCommand::DrawText { position, text, color, .. } => {
                // Basic text rendering using WebGPU
                web_sys::console::log_1(&format!("WebGPU: Drawing text '{}' at {:?}, color {:?}", text, position, color).into());
            }
            
            _ => {
                web_sys::console::log_1(&format!("WebGPU: Unimplemented command: {:?}", command).into());
            }
        }
        
        Ok(())
    }
    
    pub fn clear(&mut self, color: Vec4) -> Result<(), JsValue> {
        // Get the current texture
        let texture = self.context.get_current_texture();
        
        // Create a command encoder
        let encoder = self.device.create_command_encoder(&js_sys::Object::new());
        
        // Create a render pass
        let color_attachment = js_sys::Object::new();
        js_sys::Reflect::set(&color_attachment, &JsValue::from_str("view"), &texture.create_view(&js_sys::Object::new()))?;
        js_sys::Reflect::set(&color_attachment, &JsValue::from_str("clearValue"), &js_sys::Object::new())?;
        js_sys::Reflect::set(&color_attachment, &JsValue::from_str("loadOp"), &JsValue::from_str("clear"))?;
        js_sys::Reflect::set(&color_attachment, &JsValue::from_str("storeOp"), &JsValue::from_str("store"))?;
        
        let render_pass_desc = js_sys::Object::new();
        let color_attachments = js_sys::Array::new();
        color_attachments.push(&color_attachment);
        js_sys::Reflect::set(&render_pass_desc, &JsValue::from_str("colorAttachments"), &color_attachments)?;
        
        let render_pass = encoder.begin_render_pass(&render_pass_desc);
        render_pass.end();
        
        // Submit the command
        let commands = js_sys::Array::new();
        commands.push(&encoder.finish(&js_sys::Object::new()));
        self.queue.submit(&commands);
        
        Ok(())
    }
    
    pub fn resize(&mut self, new_size: Vec2) -> Result<(), JsValue> {
        self.size = new_size;
        self.canvas.set_width(new_size.x as u32);
        self.canvas.set_height(new_size.y as u32);
        
        // Reconfigure the context
        let config = js_sys::Object::new();
        js_sys::Reflect::set(&config, &JsValue::from_str("device"), &self.device)?;
        js_sys::Reflect::set(&config, &JsValue::from_str("format"), &JsValue::from_str("bgra8unorm"))?;
        js_sys::Reflect::set(&config, &JsValue::from_str("alphaMode"), &JsValue::from_str("premultiplied"))?;
        
        self.context.configure(&config);
        
        Ok(())
    }
    
    pub fn size(&self) -> Vec2 {
        self.size
    }
    
    /// Check if WebGPU is available in the browser
    pub fn is_available() -> bool {
        if let Some(window) = web_sys::window() {
            let navigator = window.navigator();
            if let Ok(gpu) = js_sys::Reflect::get(&navigator, &JsValue::from_str("gpu")) {
                return !gpu.is_undefined();
            }
        }
        false
    }
}
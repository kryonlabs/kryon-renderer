//! Canvas-based web renderer using HTML5 Canvas and WebGL/WebGPU

use wasm_bindgen::prelude::*;
use web_sys::{HtmlCanvasElement, WebGl2RenderingContext, CanvasRenderingContext2d};
use kryon_render::{Renderer, RenderResult, RenderError, RenderCommand};
use kryon_core::Element;
use kryon_layout::LayoutResult;
use glam::{Vec2, Vec4};

/// Simple base64 encoding for image data
fn base64_encode(data: &[u8]) -> String {
    // This is a simplified implementation - in production, use a proper base64 library
    let alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();
    
    for chunk in data.chunks(3) {
        let mut buf = [0u8; 3];
        for (i, &byte) in chunk.iter().enumerate() {
            buf[i] = byte;
        }
        
        let b0 = buf[0] as usize;
        let b1 = buf[1] as usize;
        let b2 = buf[2] as usize;
        
        let c0 = b0 >> 2;
        let c1 = ((b0 & 0x03) << 4) | (b1 >> 4);
        let c2 = ((b1 & 0x0f) << 2) | (b2 >> 6);
        let c3 = b2 & 0x3f;
        
        result.push(alphabet.chars().nth(c0).unwrap());
        result.push(alphabet.chars().nth(c1).unwrap());
        result.push(if chunk.len() > 1 { alphabet.chars().nth(c2).unwrap() } else { '=' });
        result.push(if chunk.len() > 2 { alphabet.chars().nth(c3).unwrap() } else { '=' });
    }
    
    result
}

pub struct CanvasRenderer {
    canvas: HtmlCanvasElement,
    context_2d: Option<CanvasRenderingContext2d>,
    context_webgl: Option<WebGl2RenderingContext>,
    size: Vec2,
    render_mode: RenderMode,
}

pub enum RenderMode {
    Canvas2D,
    WebGL,
    WebGPU,
}

impl CanvasRenderer {
    pub fn new(canvas_id: &str) -> Result<Self, JsValue> {
        let window = web_sys::window().ok_or("No window object")?;
        let document = window.document().ok_or("No document object")?;
        
        let canvas = document
            .get_element_by_id(canvas_id)
            .ok_or("Canvas element not found")?
            .dyn_into::<HtmlCanvasElement>()
            .map_err(|_| "Element is not a canvas")?;
        
        let size = Vec2::new(canvas.width() as f32, canvas.height() as f32);
        
        // Try to get WebGL2 context first, fallback to 2D
        let (context_2d, context_webgl, render_mode) = if let Ok(webgl_context) = canvas
            .get_context("webgl2")
            .map_err(|_| "Failed to get WebGL2 context")?
            .ok_or("No WebGL2 context")?
            .dyn_into::<WebGl2RenderingContext>()
        {
            (None, Some(webgl_context), RenderMode::WebGL)
        } else {
            let context_2d = canvas
                .get_context("2d")
                .map_err(|_| "Failed to get 2D context")?
                .ok_or("No 2D context")?
                .dyn_into::<CanvasRenderingContext2d>()
                .map_err(|_| "Context is not 2D")?;
            (Some(context_2d), None, RenderMode::Canvas2D)
        };
        
        Ok(Self {
            canvas,
            context_2d,
            context_webgl,
            size,
            render_mode,
        })
    }
    
    pub fn execute_render_command(&mut self, command: &RenderCommand) -> Result<(), JsValue> {
        match &self.render_mode {
            RenderMode::Canvas2D => self.execute_2d_command(command),
            RenderMode::WebGL => self.execute_webgl_command(command),
            RenderMode::WebGPU => self.execute_webgpu_command(command),
        }
    }
    
    fn execute_2d_command(&mut self, command: &RenderCommand) -> Result<(), JsValue> {
        let ctx = self.context_2d.as_ref().ok_or("No 2D context")?;
        
        match command {
            RenderCommand::DrawRect { position, size, color, border_radius, border_width, border_color, .. } => {
                // Fill rectangle
                ctx.set_fill_style(&JsValue::from_str(&format!(
                    "rgba({}, {}, {}, {})", 
                    (color.x * 255.0) as u8,
                    (color.y * 255.0) as u8, 
                    (color.z * 255.0) as u8,
                    color.w
                )));
                
                if *border_radius > 0.0 {
                    self.draw_rounded_rect(ctx, *position, *size, *border_radius)?;
                } else {
                    ctx.fill_rect(position.x as f64, position.y as f64, size.x as f64, size.y as f64);
                }
                
                // Draw border if specified
                if *border_width > 0.0 {
                    ctx.set_stroke_style(&JsValue::from_str(&format!(
                        "rgba({}, {}, {}, {})",
                        (border_color.x * 255.0) as u8,
                        (border_color.y * 255.0) as u8,
                        (border_color.z * 255.0) as u8,
                        border_color.w
                    )));
                    ctx.set_line_width(*border_width as f64);
                    
                    if *border_radius > 0.0 {
                        self.stroke_rounded_rect(ctx, *position, *size, *border_radius)?;
                    } else {
                        ctx.stroke_rect(position.x as f64, position.y as f64, size.x as f64, size.y as f64);
                    }
                }
            }
            
            RenderCommand::DrawText { position, text, font_size, color, .. } => {
                ctx.set_font(&format!("{}px Arial", font_size));
                ctx.set_fill_style(&JsValue::from_str(&format!(
                    "rgba({}, {}, {}, {})",
                    (color.x * 255.0) as u8,
                    (color.y * 255.0) as u8,
                    (color.z * 255.0) as u8,
                    color.w
                )));
                ctx.fill_text(text, position.x as f64, position.y as f64)?;
            }
            
            RenderCommand::SetClip { position, size } => {
                ctx.begin_path();
                ctx.rect(position.x as f64, position.y as f64, size.x as f64, size.y as f64);
                ctx.clip();
            }
            
            RenderCommand::ClearClip => {
                ctx.restore();
                ctx.save();
            }
            
            RenderCommand::DrawImage { position, size, image_data, .. } => {
                // Create an image element and draw it
                let img = web_sys::HtmlImageElement::new()?;
                
                // Convert image data to data URL (simplified)
                let base64 = base64_encode(image_data);
                let data_url = format!("data:image/png;base64,{}", base64);
                img.set_src(&data_url);
                
                // Draw the image (this is async in reality, but simplified here)
                ctx.draw_image_with_html_image_element_and_dw_and_dh(
                    &img,
                    position.x as f64,
                    position.y as f64,
                    size.x as f64,
                    size.y as f64,
                )?;
            }
            
            RenderCommand::DrawLine { start, end, color, width } => {
                ctx.set_stroke_style(&JsValue::from_str(&format!(
                    "rgba({}, {}, {}, {})",
                    (color.x * 255.0) as u8,
                    (color.y * 255.0) as u8,
                    (color.z * 255.0) as u8,
                    color.w
                )));
                ctx.set_line_width(*width as f64);
                ctx.begin_path();
                ctx.move_to(start.x as f64, start.y as f64);
                ctx.line_to(end.x as f64, end.y as f64);
                ctx.stroke();
            }
            
            RenderCommand::DrawCircle { center, radius, color, border_width, border_color } => {
                // Fill circle
                ctx.set_fill_style(&JsValue::from_str(&format!(
                    "rgba({}, {}, {}, {})",
                    (color.x * 255.0) as u8,
                    (color.y * 255.0) as u8,
                    (color.z * 255.0) as u8,
                    color.w
                )));
                ctx.begin_path();
                ctx.arc(center.x as f64, center.y as f64, *radius as f64, 0.0, 2.0 * std::f64::consts::PI)?;
                ctx.fill();
                
                // Draw border if specified
                if *border_width > 0.0 {
                    ctx.set_stroke_style(&JsValue::from_str(&format!(
                        "rgba({}, {}, {}, {})",
                        (border_color.x * 255.0) as u8,
                        (border_color.y * 255.0) as u8,
                        (border_color.z * 255.0) as u8,
                        border_color.w
                    )));
                    ctx.set_line_width(*border_width as f64);
                    ctx.stroke();
                }
            }
            
            _ => {
                // Other commands not implemented yet
                web_sys::console::log_1(&format!("Unimplemented command: {:?}", command).into());
            }
        }
        
        Ok(())
    }
    
    fn execute_webgl_command(&mut self, _command: &RenderCommand) -> Result<(), JsValue> {
        // TODO: Implement WebGL rendering
        Ok(())
    }
    
    fn execute_webgpu_command(&mut self, _command: &RenderCommand) -> Result<(), JsValue> {
        // TODO: Implement WebGPU rendering
        Ok(())
    }
    
    fn draw_rounded_rect(&self, ctx: &CanvasRenderingContext2d, position: Vec2, size: Vec2, radius: f32) -> Result<(), JsValue> {
        let x = position.x as f64;
        let y = position.y as f64;
        let w = size.x as f64;
        let h = size.y as f64;
        let r = radius as f64;
        
        ctx.begin_path();
        ctx.move_to(x + r, y);
        ctx.line_to(x + w - r, y);
        ctx.quadratic_curve_to(x + w, y, x + w, y + r);
        ctx.line_to(x + w, y + h - r);
        ctx.quadratic_curve_to(x + w, y + h, x + w - r, y + h);
        ctx.line_to(x + r, y + h);
        ctx.quadratic_curve_to(x, y + h, x, y + h - r);
        ctx.line_to(x, y + r);
        ctx.quadratic_curve_to(x, y, x + r, y);
        ctx.close_path();
        ctx.fill();
        
        Ok(())
    }
    
    fn stroke_rounded_rect(&self, ctx: &CanvasRenderingContext2d, position: Vec2, size: Vec2, radius: f32) -> Result<(), JsValue> {
        let x = position.x as f64;
        let y = position.y as f64;
        let w = size.x as f64;
        let h = size.y as f64;
        let r = radius as f64;
        
        ctx.begin_path();
        ctx.move_to(x + r, y);
        ctx.line_to(x + w - r, y);
        ctx.quadratic_curve_to(x + w, y, x + w, y + r);
        ctx.line_to(x + w, y + h - r);
        ctx.quadratic_curve_to(x + w, y + h, x + w - r, y + h);
        ctx.line_to(x + r, y + h);
        ctx.quadratic_curve_to(x, y + h, x, y + h - r);
        ctx.line_to(x, y + r);
        ctx.quadratic_curve_to(x, y, x + r, y);
        ctx.close_path();
        ctx.stroke();
        
        Ok(())
    }
    
    pub fn clear(&mut self, color: Vec4) -> Result<(), JsValue> {
        match &self.render_mode {
            RenderMode::Canvas2D => {
                let ctx = self.context_2d.as_ref().ok_or("No 2D context")?;
                ctx.set_fill_style(&JsValue::from_str(&format!(
                    "rgba({}, {}, {}, {})",
                    (color.x * 255.0) as u8,
                    (color.y * 255.0) as u8,
                    (color.z * 255.0) as u8,
                    color.w
                )));
                ctx.fill_rect(0.0, 0.0, self.size.x as f64, self.size.y as f64);
            }
            _ => {
                // TODO: Implement for other render modes
            }
        }
        Ok(())
    }
    
    pub fn resize(&mut self, new_size: Vec2) -> Result<(), JsValue> {
        self.size = new_size;
        self.canvas.set_width(new_size.x as u32);
        self.canvas.set_height(new_size.y as u32);
        Ok(())
    }
}
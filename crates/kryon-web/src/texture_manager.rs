//! Texture and image management system for web rendering

use wasm_bindgen::prelude::*;
use web_sys::{
    GpuDevice, GpuQueue, GpuTexture, GpuTextureView, GpuSampler, 
    HtmlImageElement, HtmlCanvasElement, ImageBitmap, ImageData
};
use glam::Vec2;
use std::collections::HashMap;
use crate::asset_loader::WebAssetLoader;

#[derive(Debug, Clone)]
pub struct TextureDescriptor {
    pub width: u32,
    pub height: u32,
    pub format: TextureFormat,
    pub usage: TextureUsage,
    pub mip_level_count: u32,
    pub sample_count: u32,
}

#[derive(Debug, Clone)]
pub enum TextureFormat {
    Rgba8Unorm,
    Rgba8UnormSrgb,
    Bgra8Unorm,
    Bgra8UnormSrgb,
    R8Unorm,
    Rg8Unorm,
    Depth24Plus,
    Depth32Float,
}

#[derive(Debug, Clone)]
pub struct TextureUsage {
    pub texture_binding: bool,
    pub storage_binding: bool,
    pub render_attachment: bool,
    pub copy_src: bool,
    pub copy_dst: bool,
}

pub struct ManagedTexture {
    pub gpu_texture: GpuTexture,
    pub view: GpuTextureView,
    pub sampler: GpuSampler,
    pub descriptor: TextureDescriptor,
    pub size: Vec2,
}

pub struct TextureManager {
    device: GpuDevice,
    queue: GpuQueue,
    textures: HashMap<String, ManagedTexture>,
    atlas_manager: TextureAtlasManager,
    asset_loader: WebAssetLoader,
}

impl TextureManager {
    pub fn new(device: GpuDevice, queue: GpuQueue) -> Self {
        let atlas_manager = TextureAtlasManager::new(device.clone(), queue.clone());
        
        Self {
            device,
            queue,
            textures: HashMap::new(),
            atlas_manager,
            asset_loader: WebAssetLoader::new(),
        }
    }
    
    /// Load texture from URL
    pub async fn load_texture(&mut self, name: &str, url: &str) -> Result<(), JsValue> {
        let image_data = self.asset_loader.load_image(url).await?;
        
        // Create image element
        let img = HtmlImageElement::new()?;
        let base64_data = base64_encode(&image_data);
        let data_url = format!("data:image/png;base64,{}", base64_data);
        img.set_src(&data_url);
        
        // Wait for image to load
        let promise = js_sys::Promise::new(&mut |resolve, reject| {
            let resolve_clone = resolve.clone();
            let reject_clone = reject.clone();
            
            let onload = Closure::wrap(Box::new(move || {
                resolve_clone.call0(&JsValue::NULL).unwrap();
            }) as Box<dyn FnMut()>);
            
            let onerror = Closure::wrap(Box::new(move || {
                reject_clone.call0(&JsValue::NULL).unwrap();
            }) as Box<dyn FnMut()>);
            
            img.set_onload(Some(onload.as_ref().unchecked_ref()));
            img.set_onerror(Some(onerror.as_ref().unchecked_ref()));
            
            onload.forget();
            onerror.forget();
        });
        
        wasm_bindgen_futures::JsFuture::from(promise).await?;
        
        // Create texture from image
        self.create_texture_from_image(name, &img)?;
        
        Ok(())
    }
    
    /// Create texture from HTML image element
    pub fn create_texture_from_image(&mut self, name: &str, img: &HtmlImageElement) -> Result<(), JsValue> {
        let width = img.natural_width();
        let height = img.natural_height();
        
        let descriptor = TextureDescriptor {
            width,
            height,
            format: TextureFormat::Rgba8UnormSrgb,
            usage: TextureUsage {
                texture_binding: true,
                storage_binding: false,
                render_attachment: false,
                copy_src: false,
                copy_dst: true,
            },
            mip_level_count: 1,
            sample_count: 1,
        };
        
        let texture = self.create_texture(name, &descriptor)?;
        
        // Copy image data to texture
        let source = js_sys::Object::new();
        js_sys::Reflect::set(&source, &JsValue::from_str("source"), img)?;
        
        let destination = js_sys::Object::new();
        js_sys::Reflect::set(&destination, &JsValue::from_str("texture"), &texture.gpu_texture)?;
        
        let size = js_sys::Object::new();
        js_sys::Reflect::set(&size, &JsValue::from_str("width"), &JsValue::from_f64(width as f64))?;
        js_sys::Reflect::set(&size, &JsValue::from_str("height"), &JsValue::from_f64(height as f64))?;
        
        self.queue.copy_external_image_to_texture(&source, &destination, &size);
        
        Ok(())
    }
    
    /// Create texture from raw data
    pub fn create_texture_from_data(&mut self, name: &str, data: &[u8], width: u32, height: u32, format: TextureFormat) -> Result<(), JsValue> {
        let descriptor = TextureDescriptor {
            width,
            height,
            format,
            usage: TextureUsage {
                texture_binding: true,
                storage_binding: false,
                render_attachment: false,
                copy_src: false,
                copy_dst: true,
            },
            mip_level_count: 1,
            sample_count: 1,
        };
        
        let texture = self.create_texture(name, &descriptor)?;
        
        // Copy data to texture
        let destination = js_sys::Object::new();
        js_sys::Reflect::set(&destination, &JsValue::from_str("texture"), &texture.gpu_texture)?;
        
        let size = js_sys::Object::new();
        js_sys::Reflect::set(&size, &JsValue::from_str("width"), &JsValue::from_f64(width as f64))?;
        js_sys::Reflect::set(&size, &JsValue::from_str("height"), &JsValue::from_f64(height as f64))?;
        
        let data_layout = js_sys::Object::new();
        js_sys::Reflect::set(&data_layout, &JsValue::from_str("bytesPerRow"), &JsValue::from_f64((width * 4) as f64))?;
        js_sys::Reflect::set(&data_layout, &JsValue::from_str("rowsPerImage"), &JsValue::from_f64(height as f64))?;
        
        let data_array = js_sys::Uint8Array::from(data);
        self.queue.write_texture(&destination, &data_array, &data_layout, &size);
        
        Ok(())
    }
    
    /// Create empty texture
    pub fn create_texture(&mut self, name: &str, descriptor: &TextureDescriptor) -> Result<&ManagedTexture, JsValue> {
        let texture_desc = js_sys::Object::new();
        js_sys::Reflect::set(&texture_desc, &JsValue::from_str("label"), &JsValue::from_str(name))?;
        
        let size = js_sys::Object::new();
        js_sys::Reflect::set(&size, &JsValue::from_str("width"), &JsValue::from_f64(descriptor.width as f64))?;
        js_sys::Reflect::set(&size, &JsValue::from_str("height"), &JsValue::from_f64(descriptor.height as f64))?;
        
        js_sys::Reflect::set(&texture_desc, &JsValue::from_str("size"), &size)?;
        js_sys::Reflect::set(&texture_desc, &JsValue::from_str("format"), &JsValue::from_str(&format_to_string(&descriptor.format)))?;
        js_sys::Reflect::set(&texture_desc, &JsValue::from_str("usage"), &JsValue::from_f64(usage_to_flags(&descriptor.usage) as f64))?;
        js_sys::Reflect::set(&texture_desc, &JsValue::from_str("mipLevelCount"), &JsValue::from_f64(descriptor.mip_level_count as f64))?;
        js_sys::Reflect::set(&texture_desc, &JsValue::from_str("sampleCount"), &JsValue::from_f64(descriptor.sample_count as f64))?;
        
        let gpu_texture = self.device.create_texture(&texture_desc);
        
        // Create texture view
        let view = gpu_texture.create_view(&js_sys::Object::new());
        
        // Create sampler
        let sampler_desc = js_sys::Object::new();
        js_sys::Reflect::set(&sampler_desc, &JsValue::from_str("magFilter"), &JsValue::from_str("linear"))?;
        js_sys::Reflect::set(&sampler_desc, &JsValue::from_str("minFilter"), &JsValue::from_str("linear"))?;
        js_sys::Reflect::set(&sampler_desc, &JsValue::from_str("mipmapFilter"), &JsValue::from_str("linear"))?;
        js_sys::Reflect::set(&sampler_desc, &JsValue::from_str("addressModeU"), &JsValue::from_str("clamp-to-edge"))?;
        js_sys::Reflect::set(&sampler_desc, &JsValue::from_str("addressModeV"), &JsValue::from_str("clamp-to-edge"))?;
        
        let sampler = self.device.create_sampler(&sampler_desc);
        
        let managed_texture = ManagedTexture {
            gpu_texture,
            view,
            sampler,
            descriptor: descriptor.clone(),
            size: Vec2::new(descriptor.width as f32, descriptor.height as f32),
        };
        
        self.textures.insert(name.to_string(), managed_texture);
        
        Ok(self.textures.get(name).unwrap())
    }
    
    /// Get texture by name
    pub fn get_texture(&self, name: &str) -> Option<&ManagedTexture> {
        self.textures.get(name)
    }
    
    /// Remove texture
    pub fn remove_texture(&mut self, name: &str) {
        self.textures.remove(name);
    }
    
    /// Get texture atlas manager
    pub fn atlas_manager(&mut self) -> &mut TextureAtlasManager {
        &mut self.atlas_manager
    }
    
    /// Generate mipmaps for texture
    pub fn generate_mipmaps(&self, texture_name: &str) -> Result<(), JsValue> {
        let texture = self.get_texture(texture_name)
            .ok_or_else(|| JsValue::from_str("Texture not found"))?;
        
        // Create command encoder
        let encoder = self.device.create_command_encoder(&js_sys::Object::new());
        
        // Generate mipmaps using compute or render passes
        // This is a simplified implementation
        for mip_level in 1..texture.descriptor.mip_level_count {
            let source_view = texture.gpu_texture.create_view(&js_sys::Object::new());
            let dest_view = texture.gpu_texture.create_view(&js_sys::Object::new());
            
            // Create render pass for mipmap generation
            let render_pass_desc = js_sys::Object::new();
            let color_attachments = js_sys::Array::new();
            
            let color_attachment = js_sys::Object::new();
            js_sys::Reflect::set(&color_attachment, &JsValue::from_str("view"), &dest_view)?;
            js_sys::Reflect::set(&color_attachment, &JsValue::from_str("loadOp"), &JsValue::from_str("clear"))?;
            js_sys::Reflect::set(&color_attachment, &JsValue::from_str("storeOp"), &JsValue::from_str("store"))?;
            
            color_attachments.push(&color_attachment);
            js_sys::Reflect::set(&render_pass_desc, &JsValue::from_str("colorAttachments"), &color_attachments)?;
            
            let render_pass = encoder.begin_render_pass(&render_pass_desc);
            render_pass.end();
        }
        
        // Submit commands
        let commands = js_sys::Array::new();
        commands.push(&encoder.finish(&js_sys::Object::new()));
        self.queue.submit(&commands);
        
        Ok(())
    }
}

pub struct TextureAtlasManager {
    device: GpuDevice,
    queue: GpuQueue,
    atlases: HashMap<String, TextureAtlas>,
}

pub struct TextureAtlas {
    pub texture: ManagedTexture,
    pub regions: HashMap<String, AtlasRegion>,
    pub packer: RectPacker,
}

#[derive(Debug, Clone)]
pub struct AtlasRegion {
    pub uv_min: Vec2,
    pub uv_max: Vec2,
    pub size: Vec2,
}

pub struct RectPacker {
    size: Vec2,
    free_rects: Vec<Rect>,
}

#[derive(Debug, Clone)]
struct Rect {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

impl TextureAtlasManager {
    pub fn new(device: GpuDevice, queue: GpuQueue) -> Self {
        Self {
            device,
            queue,
            atlases: HashMap::new(),
        }
    }
    
    pub fn create_atlas(&mut self, name: &str, width: u32, height: u32) -> Result<(), JsValue> {
        let descriptor = TextureDescriptor {
            width,
            height,
            format: TextureFormat::Rgba8UnormSrgb,
            usage: TextureUsage {
                texture_binding: true,
                storage_binding: false,
                render_attachment: false,
                copy_src: false,
                copy_dst: true,
            },
            mip_level_count: 1,
            sample_count: 1,
        };
        
        let texture_desc = js_sys::Object::new();
        js_sys::Reflect::set(&texture_desc, &JsValue::from_str("label"), &JsValue::from_str(&format!("{}_atlas", name)))?;
        
        let size = js_sys::Object::new();
        js_sys::Reflect::set(&size, &JsValue::from_str("width"), &JsValue::from_f64(width as f64))?;
        js_sys::Reflect::set(&size, &JsValue::from_str("height"), &JsValue::from_f64(height as f64))?;
        
        js_sys::Reflect::set(&texture_desc, &JsValue::from_str("size"), &size)?;
        js_sys::Reflect::set(&texture_desc, &JsValue::from_str("format"), &JsValue::from_str("rgba8unorm-srgb"))?;
        js_sys::Reflect::set(&texture_desc, &JsValue::from_str("usage"), &JsValue::from_f64(5))?; // TEXTURE_BINDING | COPY_DST
        
        let gpu_texture = self.device.create_texture(&texture_desc);
        let view = gpu_texture.create_view(&js_sys::Object::new());
        
        let sampler_desc = js_sys::Object::new();
        js_sys::Reflect::set(&sampler_desc, &JsValue::from_str("magFilter"), &JsValue::from_str("linear"))?;
        js_sys::Reflect::set(&sampler_desc, &JsValue::from_str("minFilter"), &JsValue::from_str("linear"))?;
        let sampler = self.device.create_sampler(&sampler_desc);
        
        let managed_texture = ManagedTexture {
            gpu_texture,
            view,
            sampler,
            descriptor,
            size: Vec2::new(width as f32, height as f32),
        };
        
        let atlas = TextureAtlas {
            texture: managed_texture,
            regions: HashMap::new(),
            packer: RectPacker::new(Vec2::new(width as f32, height as f32)),
        };
        
        self.atlases.insert(name.to_string(), atlas);
        
        Ok(())
    }
    
    pub fn add_region(&mut self, atlas_name: &str, region_name: &str, data: &[u8], width: u32, height: u32) -> Result<AtlasRegion, JsValue> {
        let atlas = self.atlases.get_mut(atlas_name)
            .ok_or_else(|| JsValue::from_str("Atlas not found"))?;
        
        let rect = atlas.packer.pack(width, height)
            .ok_or_else(|| JsValue::from_str("No space in atlas"))?;
        
        // Calculate UV coordinates
        let uv_min = Vec2::new(
            rect.x as f32 / atlas.texture.size.x,
            rect.y as f32 / atlas.texture.size.y,
        );
        let uv_max = Vec2::new(
            (rect.x + rect.width) as f32 / atlas.texture.size.x,
            (rect.y + rect.height) as f32 / atlas.texture.size.y,
        );
        
        let region = AtlasRegion {
            uv_min,
            uv_max,
            size: Vec2::new(width as f32, height as f32),
        };
        
        // Copy data to atlas texture
        let destination = js_sys::Object::new();
        js_sys::Reflect::set(&destination, &JsValue::from_str("texture"), &atlas.texture.gpu_texture)?;
        
        let origin = js_sys::Object::new();
        js_sys::Reflect::set(&origin, &JsValue::from_str("x"), &JsValue::from_f64(rect.x as f64))?;
        js_sys::Reflect::set(&origin, &JsValue::from_str("y"), &JsValue::from_f64(rect.y as f64))?;
        js_sys::Reflect::set(&destination, &JsValue::from_str("origin"), &origin)?;
        
        let size = js_sys::Object::new();
        js_sys::Reflect::set(&size, &JsValue::from_str("width"), &JsValue::from_f64(width as f64))?;
        js_sys::Reflect::set(&size, &JsValue::from_str("height"), &JsValue::from_f64(height as f64))?;
        
        let data_layout = js_sys::Object::new();
        js_sys::Reflect::set(&data_layout, &JsValue::from_str("bytesPerRow"), &JsValue::from_f64((width * 4) as f64))?;
        
        let data_array = js_sys::Uint8Array::from(data);
        self.queue.write_texture(&destination, &data_array, &data_layout, &size);
        
        atlas.regions.insert(region_name.to_string(), region.clone());
        
        Ok(region)
    }
    
    pub fn get_atlas(&self, name: &str) -> Option<&TextureAtlas> {
        self.atlases.get(name)
    }
    
    pub fn get_region(&self, atlas_name: &str, region_name: &str) -> Option<&AtlasRegion> {
        self.atlases.get(atlas_name)?.regions.get(region_name)
    }
}

impl RectPacker {
    fn new(size: Vec2) -> Self {
        Self {
            size,
            free_rects: vec![Rect {
                x: 0,
                y: 0,
                width: size.x as u32,
                height: size.y as u32,
            }],
        }
    }
    
    fn pack(&mut self, width: u32, height: u32) -> Option<Rect> {
        // Find best fit rectangle
        let mut best_rect = None;
        let mut best_area = u32::MAX;
        
        for (i, rect) in self.free_rects.iter().enumerate() {
            if rect.width >= width && rect.height >= height {
                let area = rect.width * rect.height;
                if area < best_area {
                    best_area = area;
                    best_rect = Some((i, *rect));
                }
            }
        }
        
        if let Some((index, rect)) = best_rect {
            self.free_rects.remove(index);
            
            // Split remaining space
            if rect.width > width {
                self.free_rects.push(Rect {
                    x: rect.x + width,
                    y: rect.y,
                    width: rect.width - width,
                    height: rect.height,
                });
            }
            
            if rect.height > height {
                self.free_rects.push(Rect {
                    x: rect.x,
                    y: rect.y + height,
                    width: rect.width,
                    height: rect.height - height,
                });
            }
            
            Some(Rect {
                x: rect.x,
                y: rect.y,
                width,
                height,
            })
        } else {
            None
        }
    }
}

// Helper functions

fn format_to_string(format: &TextureFormat) -> &'static str {
    match format {
        TextureFormat::Rgba8Unorm => "rgba8unorm",
        TextureFormat::Rgba8UnormSrgb => "rgba8unorm-srgb",
        TextureFormat::Bgra8Unorm => "bgra8unorm",
        TextureFormat::Bgra8UnormSrgb => "bgra8unorm-srgb",
        TextureFormat::R8Unorm => "r8unorm",
        TextureFormat::Rg8Unorm => "rg8unorm",
        TextureFormat::Depth24Plus => "depth24plus",
        TextureFormat::Depth32Float => "depth32float",
    }
}

fn usage_to_flags(usage: &TextureUsage) -> u32 {
    let mut flags = 0;
    if usage.texture_binding { flags |= 1; }
    if usage.storage_binding { flags |= 2; }
    if usage.render_attachment { flags |= 4; }
    if usage.copy_src { flags |= 8; }
    if usage.copy_dst { flags |= 16; }
    flags
}

fn base64_encode(data: &[u8]) -> String {
    // Simple base64 encoding implementation
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
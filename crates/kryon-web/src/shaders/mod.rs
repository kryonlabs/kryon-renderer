//! WebGPU shader system for advanced rendering

use wasm_bindgen::prelude::*;
use web_sys::{GpuDevice, GpuShaderModule, GpuRenderPipeline, GpuBindGroupLayout};
use glam::{Vec2, Vec4, Mat4};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ShaderDescriptor {
    pub name: String,
    pub vertex_source: String,
    pub fragment_source: String,
    pub uniforms: Vec<UniformDescriptor>,
    pub attributes: Vec<AttributeDescriptor>,
}

#[derive(Debug, Clone)]
pub struct UniformDescriptor {
    pub name: String,
    pub binding: u32,
    pub ty: UniformType,
    pub size: u32,
}

#[derive(Debug, Clone)]
pub struct AttributeDescriptor {
    pub name: String,
    pub location: u32,
    pub format: AttributeFormat,
    pub offset: u32,
}

#[derive(Debug, Clone)]
pub enum UniformType {
    Float,
    Vec2,
    Vec3,
    Vec4,
    Mat2,
    Mat3,
    Mat4,
    Texture2D,
    Sampler,
}

#[derive(Debug, Clone)]
pub enum AttributeFormat {
    Float32,
    Float32x2,
    Float32x3,
    Float32x4,
    Uint32,
    Uint32x2,
    Uint32x3,
    Uint32x4,
}

pub struct ShaderManager {
    device: GpuDevice,
    shaders: HashMap<String, CompiledShader>,
    pipelines: HashMap<String, GpuRenderPipeline>,
    bind_group_layouts: HashMap<String, GpuBindGroupLayout>,
}

pub struct CompiledShader {
    pub vertex_module: GpuShaderModule,
    pub fragment_module: GpuShaderModule,
    pub descriptor: ShaderDescriptor,
}

impl ShaderManager {
    pub fn new(device: GpuDevice) -> Self {
        let mut manager = Self {
            device,
            shaders: HashMap::new(),
            pipelines: HashMap::new(),
            bind_group_layouts: HashMap::new(),
        };
        
        // Load default shaders
        manager.load_default_shaders();
        
        manager
    }
    
    fn load_default_shaders(&mut self) {
        // Basic rectangle shader
        let rect_shader = ShaderDescriptor {
            name: "rect".to_string(),
            vertex_source: include_str!("rect.wgsl").to_string(),
            fragment_source: include_str!("rect.wgsl").to_string(),
            uniforms: vec![
                UniformDescriptor {
                    name: "view_proj".to_string(),
                    binding: 0,
                    ty: UniformType::Mat4,
                    size: 64,
                },
                UniformDescriptor {
                    name: "color".to_string(),
                    binding: 1,
                    ty: UniformType::Vec4,
                    size: 16,
                },
            ],
            attributes: vec![
                AttributeDescriptor {
                    name: "position".to_string(),
                    location: 0,
                    format: AttributeFormat::Float32x2,
                    offset: 0,
                },
                AttributeDescriptor {
                    name: "uv".to_string(),
                    location: 1,
                    format: AttributeFormat::Float32x2,
                    offset: 8,
                },
            ],
        };
        
        // Advanced gradient shader
        let gradient_shader = ShaderDescriptor {
            name: "gradient".to_string(),
            vertex_source: include_str!("gradient.wgsl").to_string(),
            fragment_source: include_str!("gradient.wgsl").to_string(),
            uniforms: vec![
                UniformDescriptor {
                    name: "view_proj".to_string(),
                    binding: 0,
                    ty: UniformType::Mat4,
                    size: 64,
                },
                UniformDescriptor {
                    name: "gradient_start".to_string(),
                    binding: 1,
                    ty: UniformType::Vec4,
                    size: 16,
                },
                UniformDescriptor {
                    name: "gradient_end".to_string(),
                    binding: 2,
                    ty: UniformType::Vec4,
                    size: 16,
                },
                UniformDescriptor {
                    name: "gradient_params".to_string(),
                    binding: 3,
                    ty: UniformType::Vec4,
                    size: 16,
                },
            ],
            attributes: vec![
                AttributeDescriptor {
                    name: "position".to_string(),
                    location: 0,
                    format: AttributeFormat::Float32x2,
                    offset: 0,
                },
                AttributeDescriptor {
                    name: "uv".to_string(),
                    location: 1,
                    format: AttributeFormat::Float32x2,
                    offset: 8,
                },
            ],
        };
        
        // Text rendering shader
        let text_shader = ShaderDescriptor {
            name: "text".to_string(),
            vertex_source: include_str!("text.wgsl").to_string(),
            fragment_source: include_str!("text.wgsl").to_string(),
            uniforms: vec![
                UniformDescriptor {
                    name: "view_proj".to_string(),
                    binding: 0,
                    ty: UniformType::Mat4,
                    size: 64,
                },
                UniformDescriptor {
                    name: "atlas_texture".to_string(),
                    binding: 1,
                    ty: UniformType::Texture2D,
                    size: 0,
                },
                UniformDescriptor {
                    name: "atlas_sampler".to_string(),
                    binding: 2,
                    ty: UniformType::Sampler,
                    size: 0,
                },
                UniformDescriptor {
                    name: "text_color".to_string(),
                    binding: 3,
                    ty: UniformType::Vec4,
                    size: 16,
                },
            ],
            attributes: vec![
                AttributeDescriptor {
                    name: "position".to_string(),
                    location: 0,
                    format: AttributeFormat::Float32x2,
                    offset: 0,
                },
                AttributeDescriptor {
                    name: "tex_coords".to_string(),
                    location: 1,
                    format: AttributeFormat::Float32x2,
                    offset: 8,
                },
            ],
        };
        
        // Compile and store shaders
        if let Ok(compiled) = self.compile_shader(rect_shader) {
            self.shaders.insert("rect".to_string(), compiled);
        }
        
        if let Ok(compiled) = self.compile_shader(gradient_shader) {
            self.shaders.insert("gradient".to_string(), compiled);
        }
        
        if let Ok(compiled) = self.compile_shader(text_shader) {
            self.shaders.insert("text".to_string(), compiled);
        }
    }
    
    pub fn compile_shader(&self, descriptor: ShaderDescriptor) -> Result<CompiledShader, JsValue> {
        // Create vertex shader module
        let vertex_desc = js_sys::Object::new();
        js_sys::Reflect::set(&vertex_desc, &JsValue::from_str("label"), &JsValue::from_str(&format!("{}_vertex", descriptor.name)))?;
        js_sys::Reflect::set(&vertex_desc, &JsValue::from_str("code"), &JsValue::from_str(&descriptor.vertex_source))?;
        let vertex_module = self.device.create_shader_module(&vertex_desc);
        
        // Create fragment shader module
        let fragment_desc = js_sys::Object::new();
        js_sys::Reflect::set(&fragment_desc, &JsValue::from_str("label"), &JsValue::from_str(&format!("{}_fragment", descriptor.name)))?;
        js_sys::Reflect::set(&fragment_desc, &JsValue::from_str("code"), &JsValue::from_str(&descriptor.fragment_source))?;
        let fragment_module = self.device.create_shader_module(&fragment_desc);
        
        Ok(CompiledShader {
            vertex_module,
            fragment_module,
            descriptor,
        })
    }
    
    pub fn create_render_pipeline(&mut self, shader_name: &str, surface_format: &str) -> Result<(), JsValue> {
        let shader = self.shaders.get(shader_name)
            .ok_or_else(|| JsValue::from_str("Shader not found"))?;
        
        // Create bind group layout
        let bind_group_layout = self.create_bind_group_layout(&shader.descriptor)?;
        self.bind_group_layouts.insert(shader_name.to_string(), bind_group_layout.clone());
        
        // Create pipeline layout
        let pipeline_layout_desc = js_sys::Object::new();
        js_sys::Reflect::set(&pipeline_layout_desc, &JsValue::from_str("label"), &JsValue::from_str(&format!("{}_pipeline_layout", shader_name)))?;
        let bind_group_layouts = js_sys::Array::new();
        bind_group_layouts.push(&bind_group_layout);
        js_sys::Reflect::set(&pipeline_layout_desc, &JsValue::from_str("bindGroupLayouts"), &bind_group_layouts)?;
        
        let pipeline_layout = self.device.create_pipeline_layout(&pipeline_layout_desc);
        
        // Create render pipeline
        let pipeline_desc = js_sys::Object::new();
        js_sys::Reflect::set(&pipeline_desc, &JsValue::from_str("label"), &JsValue::from_str(&format!("{}_pipeline", shader_name)))?;
        js_sys::Reflect::set(&pipeline_desc, &JsValue::from_str("layout"), &pipeline_layout)?;
        
        // Vertex state
        let vertex_state = js_sys::Object::new();
        js_sys::Reflect::set(&vertex_state, &JsValue::from_str("module"), &shader.vertex_module)?;
        js_sys::Reflect::set(&vertex_state, &JsValue::from_str("entryPoint"), &JsValue::from_str("vs_main"))?;
        
        // Create vertex buffer layout
        let vertex_buffer_layout = self.create_vertex_buffer_layout(&shader.descriptor)?;
        let vertex_buffers = js_sys::Array::new();
        vertex_buffers.push(&vertex_buffer_layout);
        js_sys::Reflect::set(&vertex_state, &JsValue::from_str("buffers"), &vertex_buffers)?;
        
        js_sys::Reflect::set(&pipeline_desc, &JsValue::from_str("vertex"), &vertex_state)?;
        
        // Fragment state
        let fragment_state = js_sys::Object::new();
        js_sys::Reflect::set(&fragment_state, &JsValue::from_str("module"), &shader.fragment_module)?;
        js_sys::Reflect::set(&fragment_state, &JsValue::from_str("entryPoint"), &JsValue::from_str("fs_main"))?;
        
        // Color targets
        let color_target = js_sys::Object::new();
        js_sys::Reflect::set(&color_target, &JsValue::from_str("format"), &JsValue::from_str(surface_format))?;
        
        let color_targets = js_sys::Array::new();
        color_targets.push(&color_target);
        js_sys::Reflect::set(&fragment_state, &JsValue::from_str("targets"), &color_targets)?;
        
        js_sys::Reflect::set(&pipeline_desc, &JsValue::from_str("fragment"), &fragment_state)?;
        
        // Primitive state
        let primitive_state = js_sys::Object::new();
        js_sys::Reflect::set(&primitive_state, &JsValue::from_str("topology"), &JsValue::from_str("triangle-list"))?;
        js_sys::Reflect::set(&pipeline_desc, &JsValue::from_str("primitive"), &primitive_state)?;
        
        let pipeline = self.device.create_render_pipeline(&pipeline_desc);
        self.pipelines.insert(shader_name.to_string(), pipeline);
        
        Ok(())
    }
    
    fn create_bind_group_layout(&self, descriptor: &ShaderDescriptor) -> Result<GpuBindGroupLayout, JsValue> {
        let entries = js_sys::Array::new();
        
        for uniform in &descriptor.uniforms {
            let entry = js_sys::Object::new();
            js_sys::Reflect::set(&entry, &JsValue::from_str("binding"), &JsValue::from_f64(uniform.binding as f64))?;
            js_sys::Reflect::set(&entry, &JsValue::from_str("visibility"), &JsValue::from_str("vertex | fragment"))?;
            
            let resource = js_sys::Object::new();
            match uniform.ty {
                UniformType::Texture2D => {
                    js_sys::Reflect::set(&resource, &JsValue::from_str("texture"), &js_sys::Object::new())?;
                }
                UniformType::Sampler => {
                    js_sys::Reflect::set(&resource, &JsValue::from_str("sampler"), &js_sys::Object::new())?;
                }
                _ => {
                    let buffer = js_sys::Object::new();
                    js_sys::Reflect::set(&buffer, &JsValue::from_str("type"), &JsValue::from_str("uniform"))?;
                    js_sys::Reflect::set(&resource, &JsValue::from_str("buffer"), &buffer)?;
                }
            }
            
            js_sys::Reflect::set(&entry, &JsValue::from_str("resource"), &resource)?;
            entries.push(&entry);
        }
        
        let layout_desc = js_sys::Object::new();
        js_sys::Reflect::set(&layout_desc, &JsValue::from_str("entries"), &entries)?;
        
        Ok(self.device.create_bind_group_layout(&layout_desc))
    }
    
    fn create_vertex_buffer_layout(&self, descriptor: &ShaderDescriptor) -> Result<js_sys::Object, JsValue> {
        let attributes = js_sys::Array::new();
        let mut stride = 0;
        
        for attr in &descriptor.attributes {
            let attribute = js_sys::Object::new();
            js_sys::Reflect::set(&attribute, &JsValue::from_str("shaderLocation"), &JsValue::from_f64(attr.location as f64))?;
            js_sys::Reflect::set(&attribute, &JsValue::from_str("offset"), &JsValue::from_f64(attr.offset as f64))?;
            
            let format_str = match attr.format {
                AttributeFormat::Float32 => "float32",
                AttributeFormat::Float32x2 => "float32x2",
                AttributeFormat::Float32x3 => "float32x3",
                AttributeFormat::Float32x4 => "float32x4",
                AttributeFormat::Uint32 => "uint32",
                AttributeFormat::Uint32x2 => "uint32x2",
                AttributeFormat::Uint32x3 => "uint32x3",
                AttributeFormat::Uint32x4 => "uint32x4",
            };
            
            js_sys::Reflect::set(&attribute, &JsValue::from_str("format"), &JsValue::from_str(format_str))?;
            attributes.push(&attribute);
            
            stride += match attr.format {
                AttributeFormat::Float32 | AttributeFormat::Uint32 => 4,
                AttributeFormat::Float32x2 | AttributeFormat::Uint32x2 => 8,
                AttributeFormat::Float32x3 | AttributeFormat::Uint32x3 => 12,
                AttributeFormat::Float32x4 | AttributeFormat::Uint32x4 => 16,
            };
        }
        
        let buffer_layout = js_sys::Object::new();
        js_sys::Reflect::set(&buffer_layout, &JsValue::from_str("arrayStride"), &JsValue::from_f64(stride as f64))?;
        js_sys::Reflect::set(&buffer_layout, &JsValue::from_str("attributes"), &attributes)?;
        
        Ok(buffer_layout)
    }
    
    pub fn get_pipeline(&self, shader_name: &str) -> Option<&GpuRenderPipeline> {
        self.pipelines.get(shader_name)
    }
    
    pub fn get_bind_group_layout(&self, shader_name: &str) -> Option<&GpuBindGroupLayout> {
        self.bind_group_layouts.get(shader_name)
    }
    
    pub fn get_shader(&self, shader_name: &str) -> Option<&CompiledShader> {
        self.shaders.get(shader_name)
    }
    
    pub fn reload_shader(&mut self, shader_name: &str, descriptor: ShaderDescriptor) -> Result<(), JsValue> {
        let compiled = self.compile_shader(descriptor)?;
        self.shaders.insert(shader_name.to_string(), compiled);
        
        // Remove old pipeline and bind group layout
        self.pipelines.remove(shader_name);
        self.bind_group_layouts.remove(shader_name);
        
        Ok(())
    }
}
// crates/kryon-wgpu/src/lib.rs
use kryon_render::{
    Renderer, CommandRenderer, RenderCommand, RenderResult, RenderError
};
use kryon_layout::LayoutResult;
use glam::{Vec2, Vec4, Mat4};
use winit::window::Window;

pub mod shaders;
pub mod vertex;
pub mod text;
pub mod resources;

use vertex::*;
use text::TextRenderer;
use resources::ResourceManager;

pub struct WgpuRenderer {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: Vec2,
    
    // Rendering pipeline
    rect_pipeline: wgpu::RenderPipeline,
    text_pipeline: wgpu::RenderPipeline,
    
    // Uniform buffers
    view_proj_buffer: wgpu::Buffer,
    view_proj_bind_group: wgpu::BindGroup,
    
    // Text rendering
    text_renderer: TextRenderer,
    
    // Resource management
    resource_manager: ResourceManager,
    
    // Vertex buffers (reusable)
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
}

pub struct WgpuRenderContext {
    encoder: wgpu::CommandEncoder,
    view: wgpu::TextureView,
}

impl Renderer for WgpuRenderer {
    type Surface = (std::sync::Arc<Window>, Vec2);
    type Context = WgpuRenderContext;
    
    fn initialize(surface: Self::Surface) -> RenderResult<Self> where Self: Sized {
        pollster::block_on(Self::new_async(surface.0, surface.1))
    }
    
    fn begin_frame(&mut self, _clear_color: Vec4) -> RenderResult<Self::Context> {
        let output = self.surface
            .get_current_texture()
            .map_err(|e| RenderError::RenderFailed(format!("Failed to get surface texture: {}", e)))?;
        
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        
        let encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });
        
        Ok(WgpuRenderContext { encoder, view })
    }
    
    fn end_frame(&mut self, context: Self::Context) -> RenderResult<()> {
        self.queue.submit(std::iter::once(context.encoder.finish()));
        Ok(())
    }
    
    fn render_element(
        &mut self,
        _context: &mut Self::Context,
        _element: &kryon_core::Element,
        _layout: &LayoutResult,
        _element_id: kryon_core::ElementId,
    ) -> RenderResult<()> {
        // This method is not used in command-based rendering
        Ok(())
    }
    
    fn resize(&mut self, new_size: Vec2) -> RenderResult<()> {
        if new_size.x > 0.0 && new_size.y > 0.0 {
            self.size = new_size;
            self.config.width = new_size.x as u32;
            self.config.height = new_size.y as u32;
            self.surface.configure(&self.device, &self.config);
            
            // Update projection matrix
            self.update_view_projection()?;
        }
        Ok(())
    }
    
    fn viewport_size(&self) -> Vec2 {
        self.size
    }
}

impl CommandRenderer for WgpuRenderer {
    fn execute_commands(
        &mut self,
        context: &mut Self::Context,
        commands: &[RenderCommand],
    ) -> RenderResult<()> {
        if commands.is_empty() {
            return Ok(());
        }
        
        // Separate commands by type for batching
        let mut rect_commands = Vec::new();
        let mut text_commands = Vec::new();
        let mut image_commands = Vec::new();
        
        for command in commands {
            match command {
                RenderCommand::DrawRect { .. } => rect_commands.push(command),
                RenderCommand::DrawText { .. } => text_commands.push(command),
                RenderCommand::DrawImage { .. } => image_commands.push(command),
                _ => {} // Handle other commands
            }
        }
        
        // Render rectangles
        if !rect_commands.is_empty() {
            self.render_rects(context, &rect_commands)?;
        }
        
        // Render text
        if !text_commands.is_empty() {
            self.render_text(context, &text_commands)?;
        }
        
        // Render images
        if !image_commands.is_empty() {
            self.render_images(context, &image_commands)?;
        }
        
        Ok(())
    }
}

impl WgpuRenderer {

    async fn new_async(window: std::sync::Arc<Window>, size: Vec2) -> RenderResult<Self> {
        eprintln!("Creating WGPU instance...");
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            flags: wgpu::InstanceFlags::DEBUG,
            ..Default::default()
        });

        // The surface is now created *inside* the renderer from the window handle.
        // This resolves the type mismatch and the original ownership panic.
        let surface = instance.create_surface(window)
            .map_err(|e| RenderError::InitializationFailed(format!("Failed to create surface: {}", e)))?;

        // Debug: Enumerate all adapters first
        eprintln!("Enumerating all adapters...");
        let adapters: Vec<_> = instance.enumerate_adapters(wgpu::Backends::all()).into_iter().collect();
        eprintln!("Found {} total adapters:", adapters.len());
        for (i, adapter) in adapters.iter().enumerate() {
            let info = adapter.get_info();
            eprintln!("  Adapter {}: {} ({:?}) - {:?}", i, info.name, info.backend, info.device_type);
        }

        if adapters.is_empty() {
            return Err(RenderError::InitializationFailed("No adapters enumerated by WGPU".to_string()));
        }

        eprintln!("Requesting adapter with surface compatibility...");
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await;

        let adapter = match adapter {
            Some(adapter) => {
                let info = adapter.get_info();
                eprintln!("SUCCESS: Found compatible adapter: {} ({:?})", info.name, info.backend);
                adapter
            }
            None => {
                eprintln!("ERROR: No surface-compatible adapter found!");

                // Try without surface compatibility as fallback
                eprintln!("Trying without surface compatibility...");
                let fallback_adapter = instance
                    .request_adapter(&wgpu::RequestAdapterOptions {
                        power_preference: wgpu::PowerPreference::default(),
                        compatible_surface: None,
                        force_fallback_adapter: false,
                    })
                    .await;

                match fallback_adapter {
                    Some(adapter) => {
                        let info = adapter.get_info();
                        eprintln!("FALLBACK: Using adapter without surface check: {} ({:?})", info.name, info.backend);
                        adapter
                    }
                    None => {
                        return Err(RenderError::InitializationFailed("No adapter found even without surface compatibility".to_string()));
                    }
                }
            }
        };

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                },
                None,
            )
            .await
            .map_err(|e| RenderError::InitializationFailed(format!("Device request failed: {}", e)))?;

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.x as u32,
            height: size.y as u32,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &config);

        // Create uniform buffer for view-projection matrix
        let view_proj_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("View Projection Buffer"),
            size: std::mem::size_of::<ViewProjectionUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let uniform_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("uniform_bind_group_layout"),
        });

        let view_proj_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: view_proj_buffer.as_entire_binding(),
            }],
            label: Some("view_proj_bind_group"),
        });

        // Create shaders
        let rect_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Rectangle Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/rect.wgsl").into()),
        });

        let text_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Text Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/text.wgsl").into()),
        });

        // Create text rendering pipeline
        let text_renderer = TextRenderer::new(&device, &queue)
            .map_err(|e| RenderError::InitializationFailed(format!("Text renderer creation failed: {}", e)))?;

        // The text pipeline needs the bind group layout from the text atlas
        let text_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Text Pipeline Layout"),
            bind_group_layouts: &[&uniform_bind_group_layout, text_renderer.bind_group_layout()],
            push_constant_ranges: &[],
        });

        let text_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Text Pipeline"),
            layout: Some(&text_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &text_shader,
                entry_point: "vs_main",
                buffers: &[TextVertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &text_shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None, // Text can have different winding orders
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        // Create rectangle rendering pipeline
        let rect_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Rectangle Pipeline Layout"),
            bind_group_layouts: &[&uniform_bind_group_layout],
            push_constant_ranges: &[],
        });

        let rect_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Rectangle Pipeline"),
            layout: Some(&rect_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &rect_shader,
                entry_point: "vs_main",
                buffers: &[RectVertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &rect_shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });
        
        // Create vertex and index buffers
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Vertex Buffer"),
            size: 1024 * 1024, // 1MB buffer
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        
        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Index Buffer"),
            size: 1024 * 1024, // 1MB buffer
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let mut renderer = Self {
            surface,
            device,
            queue,
            config,
            size,
            rect_pipeline,
            text_pipeline,
            view_proj_buffer,
            view_proj_bind_group,
            text_renderer,
            resource_manager: ResourceManager::new(),
            vertex_buffer,
            index_buffer,
        };

        renderer.update_view_projection()?;

        Ok(renderer)
    }
    
    fn update_view_projection(&mut self) -> RenderResult<()> {
        let projection = Mat4::orthographic_rh(
            0.0,
            self.size.x,
            self.size.y,
            0.0,
            -1.0,
            1.0,
        );
        
        let uniform = ViewProjectionUniform {
            view_proj: projection.to_cols_array_2d(),
        };
        
        self.queue.write_buffer(
            &self.view_proj_buffer,
            0,
            bytemuck::cast_slice(&[uniform]),
        );
        
        Ok(())
    }
    
    fn render_rects(
        &mut self,
        context: &mut WgpuRenderContext,
        commands: &[&RenderCommand],
    ) -> RenderResult<()> {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let mut index_offset = 0u16;
        
        for command in commands {
            if let RenderCommand::DrawRect {
                position,
                size,
                color,
                border_radius,
                border_width,
                border_color,
            } = command {
                // Generate vertices for rounded rectangle
                let rect_vertices = generate_rounded_rect_vertices(
                    *position,
                    *size,
                    *color,
                    *border_radius,
                    *border_width,
                    *border_color,
                );
                
                // Add vertices and indices
                for vertex in rect_vertices {
                    vertices.push(vertex);
                }
                
                // Generate indices for two triangles (quad)
                indices.extend_from_slice(&[
                    index_offset,
                    index_offset + 1,
                    index_offset + 2,
                    index_offset + 2,
                    index_offset + 3,
                    index_offset,
                ]);
                index_offset += 4;
            }
        }
        
        if vertices.is_empty() {
            return Ok(());
        }
        
        // Upload vertex data
        self.queue.write_buffer(
            &self.vertex_buffer,
            0,
            bytemuck::cast_slice(&vertices),
        );
        
        self.queue.write_buffer(
            &self.index_buffer,
            0,
            bytemuck::cast_slice(&indices),
        );
        
        // Render
        let mut render_pass = context.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Rectangle Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &context.view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });
        
        render_pass.set_pipeline(&self.rect_pipeline);
        render_pass.set_bind_group(0, &self.view_proj_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.draw_indexed(0..indices.len() as u32, 0, 0..1);
        
        Ok(())
    }
    
    fn render_text(
        &mut self,
        context: &mut WgpuRenderContext,
        commands: &[&RenderCommand],
    ) -> RenderResult<()> {
        for command in commands {
            if let RenderCommand::DrawText {
                position,
                text,
                font_size,
                color,
                alignment,
                max_width,
                max_height: _,
            } = command {
                self.text_renderer.render_text(
                    &mut context.encoder,
                    &context.view,
                    &self.text_pipeline,
                    &self.view_proj_bind_group,
                    text,
                    *position,
                    *font_size,
                    *color,
                    *alignment,
                    *max_width,
                ).map_err(|e| RenderError::RenderFailed(format!("Text rendering failed: {}", e)))?;
            }
        }
        Ok(())
    }
    
    fn render_images(
        &mut self,
        _context: &mut WgpuRenderContext,
        _commands: &[&RenderCommand],
    ) -> RenderResult<()> {
        // TODO: Implement image rendering
        Ok(())
    }
}
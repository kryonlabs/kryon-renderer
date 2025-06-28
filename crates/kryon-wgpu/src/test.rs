// crates/kryon-wgpu/src/text.rs
use crate::vertex::TextVertex;
use kryon_render::{RenderResult, RenderError, TextAlignment};
use wgpu::util::DeviceExt;
use glam::{Vec2, Vec4};
use fontdue::{Font, FontSettings};
use std::collections::HashMap;

pub struct TextRenderer {
    font: Font,
    texture_atlas: wgpu::Texture,
    texture_bind_group_layout: wgpu::BindGroupLayoutDescriptor<'static>,
    texture_bind_group: wgpu::BindGroup,
    glyph_cache: HashMap<(char, u32), GlyphInfo>, // (char, font_size) -> glyph info
    atlas_size: u32,
    next_atlas_x: u32,
    next_atlas_y: u32,
    current_row_height: u32,
}

#[derive(Clone)]
struct GlyphInfo {
    tex_coords: (f32, f32, f32, f32), // (u1, v1, u2, v2)
    offset: Vec2,
    advance: f32,
}

impl TextRenderer {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> RenderResult<Self> {
        // Load default font (embedded)
        let font_data = include_bytes!("../assets/Roboto-Regular.ttf");
        let font = Font::from_bytes(font_data as &[u8], FontSettings::default())
            .map_err(|e| RenderError::InitializationFailed(format!("Failed to load font: {}", e)))?;
        
        let atlas_size = 1024;
        
        // Create texture atlas
        let texture_atlas = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Text Atlas"),
            size: wgpu::Extent3d {
                width: atlas_size,
                height: atlas_size,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        
        let texture_view = texture_atlas.create_view(&wgpu::TextureViewDescriptor::default());
        
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        
        let texture_bind_group_layout = wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: Some("texture_bind_group_layout"),
        };
        
        let texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &device.create_bind_group_layout(&texture_bind_group_layout),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
            label: Some("diffuse_bind_group"),
        });
        
        Ok(Self {
            font,
            texture_atlas,
            texture_bind_group_layout,
            texture_bind_group,
            glyph_cache: HashMap::new(),
            atlas_size,
            next_atlas_x: 0,
            next_atlas_y: 0,
            current_row_height: 0,
        })
    }
    
    pub fn render_text(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        pipeline: &wgpu::RenderPipeline,
        view_proj_bind_group: &wgpu::BindGroup,
        text: &str,
        position: Vec2,
        font_size: f32,
        color: Vec4,
        alignment: TextAlignment,
        max_width: Option<f32>,
    ) -> RenderResult<()> {
        let vertices = self.generate_text_vertices(
            text, position, font_size, color, alignment, max_width
        )?;
        
        if vertices.is_empty() {
            return Ok(());
        }
        
        // Create temporary vertex buffer for this text
        let vertex_buffer = encoder.create_buffer_with_data(&wgpu::util::BufferInitDescriptor {
            label: Some("Text Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        
        let indices: Vec<u16> = (0..vertices.len() as u16)
            .collect::<Vec<_>>()
            .chunks(4)
            .flat_map(|chunk| [chunk[0], chunk[1], chunk[2], chunk[2], chunk[3], chunk[0]])
            .collect();
        
        let index_buffer = encoder.create_buffer_with_data(&wgpu::util::BufferInitDescriptor {
            label: Some("Text Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Text Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
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
        
        render_pass.set_pipeline(pipeline);
        render_pass.set_bind_group(0, view_proj_bind_group, &[]);
        render_pass.set_bind_group(1, &self.texture_bind_group, &[]);
        render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
        render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.draw_indexed(0..indices.len() as u32, 0, 0..1);
        
        Ok(())
    }
    
    fn generate_text_vertices(
        &mut self,
        text: &str,
        position: Vec2,
        font_size: f32,
        color: Vec4,
        _alignment: TextAlignment,
        _max_width: Option<f32>,
    ) -> RenderResult<Vec<TextVertex>> {
        let mut vertices = Vec::new();
        let mut x_offset = 0.0;
        let scale = font_size;
        
        for ch in text.chars() {
            let glyph_info = self.get_or_cache_glyph(ch, font_size as u32)?;
            
            // Create quad for this glyph
            let x = position.x + x_offset + glyph_info.offset.x;
            let y = position.y + glyph_info.offset.y;
            let w = (glyph_info.tex_coords.2 - glyph_info.tex_coords.0) * self.atlas_size as f32;
            let h = (glyph_info.tex_coords.3 - glyph_info.tex_coords.1) * self.atlas_size as f32;
            
            // Add vertices for quad
            vertices.push(TextVertex {
                position: [x, y],
                tex_coords: [glyph_info.tex_coords.0, glyph_info.tex_coords.1],
                color: color.into(),
            });
            vertices.push(TextVertex {
                position: [x + w, y],
                tex_coords: [glyph_info.tex_coords.2, glyph_info.tex_coords.1],
                color: color.into(),
            });
            vertices.push(TextVertex {
                position: [x + w, y + h],
                tex_coords: [glyph_info.tex_coords.2, glyph_info.tex_coords.3],
                color: color.into(),
            });
            vertices.push(TextVertex {
                position: [x, y + h],
                tex_coords: [glyph_info.tex_coords.0, glyph_info.tex_coords.3],
                color: color.into(),
            });
            
            x_offset += glyph_info.advance;
        }
        
        Ok(vertices)
    }
    
    fn get_or_cache_glyph(&mut self, ch: char, font_size: u32) -> RenderResult<GlyphInfo> {
        let key = (ch, font_size);
        
        if let Some(glyph_info) = self.glyph_cache.get(&key) {
            return Ok(glyph_info.clone());
        }
        
        // Rasterize the glyph
        let (metrics, bitmap) = self.font.rasterize(ch, font_size as f32);
        
        // TODO: Add glyph to texture atlas and update glyph cache
        // For now, return a dummy glyph info
        let glyph_info = GlyphInfo {
            tex_coords: (0.0, 0.0, 0.1, 0.1),
            offset: Vec2::new(metrics.xmin as f32, metrics.ymin as f32),
            advance: metrics.advance_width,
        };
        
        self.glyph_cache.insert(key, glyph_info.clone());
        Ok(glyph_info)
    }
}
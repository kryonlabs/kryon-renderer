// crates/kryon-wgpu/src/text.rs
use fontdue::{Font, FontSettings};
use glam::{Vec2, Vec4};
use std::collections::HashMap;

pub struct TextRenderer {
    font: Font,
    atlas: TextureAtlas,
    cache: HashMap<TextCacheKey, CachedGlyph>,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct TextCacheKey {
    character: char,
    font_size: u32,
}

#[derive(Debug, Clone)]
struct CachedGlyph {
    texture_coords: [f32; 4], // x, y, width, height in atlas
    metrics: fontdue::Metrics,
}

pub struct TextureAtlas {
    texture: wgpu::Texture,
    _texture_view: wgpu::TextureView,
    bind_group: wgpu::BindGroup,
    bind_group_layout: wgpu::BindGroupLayout,
    size: u32,
    cursor_x: u32,
    cursor_y: u32,
    row_height: u32,
}

impl TextRenderer {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Result<Self, Box<dyn std::error::Error>> {
        // Use a minimal default font - for now we'll create a dummy font
        // In a real implementation, you'd load a proper font file
        let font_data = include_bytes!("../../../assets/fonts/default.ttf");
        let font = Font::from_bytes(font_data as &[u8], FontSettings::default())
            .map_err(|_| "Failed to load default font - you need to provide a font file")?;
        
        let atlas = TextureAtlas::new(device, queue, 1024)?;
        
        Ok(Self {
            font,
            atlas,
            cache: HashMap::new(),
        })
    }
    
    pub fn prepare_text(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        text: &str,
        font_size: f32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let font_size_px = font_size as u32;
        
        for character in text.chars() {
            let key = TextCacheKey {
                character,
                font_size: font_size_px,
            };
            
            if !self.cache.contains_key(&key) {
                let (metrics, bitmap) = self.font.rasterize(character, font_size);
                
                if !bitmap.is_empty() {
                    let texture_coords = self.atlas.add_glyph(device, queue, &bitmap, metrics.width, metrics.height)?;
                    
                    self.cache.insert(key, CachedGlyph {
                        texture_coords,
                        metrics,
                    });
                }
            }
        }
        
        Ok(())
    }
    
    pub fn generate_text_vertices(
        &self,
        text: &str,
        position: Vec2,
        font_size: f32,
        color: Vec4,
    ) -> Vec<crate::vertex::TextVertex> {
        let mut vertices = Vec::new();
        let mut cursor_x = position.x;
        let font_size_px = font_size as u32;
        
        for character in text.chars() {
            let key = TextCacheKey {
                character,
                font_size: font_size_px,
            };
            
            if let Some(cached_glyph) = self.cache.get(&key) {
                let glyph_pos = Vec2::new(
                    cursor_x + cached_glyph.metrics.xmin as f32,
                    position.y + cached_glyph.metrics.ymin as f32,
                );
                
                let glyph_size = Vec2::new(
                    cached_glyph.metrics.width as f32,
                    cached_glyph.metrics.height as f32,
                );
                
                // Generate quad for this glyph
                let tex_coords = cached_glyph.texture_coords;
                
                vertices.extend_from_slice(&[
                    crate::vertex::TextVertex {
                        position: [glyph_pos.x, glyph_pos.y],
                        tex_coords: [tex_coords[0], tex_coords[1]],
                        color: color.into(),
                    },
                    crate::vertex::TextVertex {
                        position: [glyph_pos.x + glyph_size.x, glyph_pos.y],
                        tex_coords: [tex_coords[0] + tex_coords[2], tex_coords[1]],
                        color: color.into(),
                    },
                    crate::vertex::TextVertex {
                        position: [glyph_pos.x + glyph_size.x, glyph_pos.y + glyph_size.y],
                        tex_coords: [tex_coords[0] + tex_coords[2], tex_coords[1] + tex_coords[3]],
                        color: color.into(),
                    },
                    crate::vertex::TextVertex {
                        position: [glyph_pos.x, glyph_pos.y],
                        tex_coords: [tex_coords[0], tex_coords[1]],
                        color: color.into(),
                    },
                    crate::vertex::TextVertex {
                        position: [glyph_pos.x + glyph_size.x, glyph_pos.y + glyph_size.y],
                        tex_coords: [tex_coords[0] + tex_coords[2], tex_coords[1] + tex_coords[3]],
                        color: color.into(),
                    },
                    crate::vertex::TextVertex {
                        position: [glyph_pos.x, glyph_pos.y + glyph_size.y],
                        tex_coords: [tex_coords[0], tex_coords[1] + tex_coords[3]],
                        color: color.into(),
                    },
                ]);
                
                cursor_x += cached_glyph.metrics.advance_width;
            }
        }
        
        vertices
    }

    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.atlas.bind_group
    }
    
    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.atlas.bind_group_layout
    }

    pub fn render_text(
        &mut self,
        _encoder: &mut wgpu::CommandEncoder,
        _target: &wgpu::TextureView,
        _pipeline: &wgpu::RenderPipeline,
        _bind_group: &wgpu::BindGroup,
        text: &str,
        position: Vec2,
        font_size: f32,
        color: Vec4,
        _alignment: kryon_core::TextAlignment,
        _max_width: Option<f32>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Generate vertices for the text
        let vertices = self.generate_text_vertices(text, position, font_size, color);
        
        if vertices.is_empty() {
            return Ok(());
        }
        
        // For now, just return Ok - in a real implementation you'd render the vertices
        // TODO: Implement actual text rendering with the encoder
        Ok(())
    }
}

impl TextureAtlas {
    fn new(device: &wgpu::Device, _queue: &wgpu::Queue, size: u32) -> Result<Self, Box<dyn std::error::Error>> {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Text Atlas"),
            size: wgpu::Extent3d {
                width: size,
                height: size,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        
        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Text Atlas Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Text Atlas Bind Group Layout"),
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
        });
        
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Text Atlas Bind Group"),
            layout: &bind_group_layout,
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
        });
        
        Ok(Self {
            texture,
            _texture_view: texture_view,
            bind_group,
            bind_group_layout,
            size,
            cursor_x: 0,
            cursor_y: 0,
            row_height: 0,
        })
    }
    
    fn add_glyph(
        &mut self,
        _device: &wgpu::Device,
        queue: &wgpu::Queue,
        bitmap: &[u8],
        width: usize,
        height: usize,
    ) -> Result<[f32; 4], Box<dyn std::error::Error>> {
        // Check if we need to move to next row
        if self.cursor_x + width as u32 > self.size {
            self.cursor_x = 0;
            self.cursor_y += self.row_height;
            self.row_height = 0;
        }
        
        // Check if we have space
        if self.cursor_y + height as u32 > self.size {
            return Err("Text atlas is full".into());
        }
        
        // Upload glyph to texture
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: self.cursor_x,
                    y: self.cursor_y,
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            bitmap,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(width as u32),
                rows_per_image: Some(height as u32),
            },
            wgpu::Extent3d {
                width: width as u32,
                height: height as u32,
                depth_or_array_layers: 1,
            },
        );
        
        // Calculate texture coordinates (normalized)
        let tex_coords = [
            self.cursor_x as f32 / self.size as f32,
            self.cursor_y as f32 / self.size as f32,
            width as f32 / self.size as f32,
            height as f32 / self.size as f32,
        ];
        
        // Update cursor
        self.cursor_x += width as u32;
        self.row_height = self.row_height.max(height as u32);
        
        Ok(tex_coords)
    }
}
//! TextManager for cosmic-text integration

use cosmic_text::{
    Attrs, Buffer, Color as CosmicColor, Family, FontSystem, Metrics, Shaping, SwashCache, Weight, Style as CosmicStyle
};
use kryon_core::{RichText, TextSpan, RichFontWeight, RichFontStyle, RichTextAlignment};
use glam::{Vec2, Vec4};
use std::collections::HashMap;

/// Central text management system using cosmic-text
pub struct TextManager {
    /// Font system for loading and managing fonts
    font_system: FontSystem,
    
    /// Swash cache for rasterizing glyphs
    swash_cache: SwashCache,
    
    /// Cache of prepared text buffers for reuse
    buffer_cache: HashMap<String, Buffer>,
    
    /// Default font family
    default_font_family: String,
    
    /// Default font size
    default_font_size: f32,
}

/// Rendered text with glyph positioning information
#[derive(Debug, Clone)]
pub struct RenderedText {
    /// Positioned glyphs ready for rendering
    pub glyphs: Vec<PositionedGlyph>,
    
    /// Total text bounds
    pub bounds: Vec2,
    
    /// Line height used
    pub line_height: f32,
}

/// A single glyph with position and rendering info
#[derive(Debug, Clone)]
pub struct PositionedGlyph {
    /// Glyph position relative to text origin
    pub position: Vec2,
    
    /// Glyph size
    pub size: Vec2,
    
    /// Text color
    pub color: Vec4,
    
    /// Font size
    pub font_size: f32,
    
    /// The actual glyph character
    pub character: char,
    
    /// Glyph ID for texture atlas lookup
    pub glyph_id: u32,
    
    /// Font cache key for identifying which font this glyph belongs to
    pub font_cache_key: String,
}

impl TextManager {
    /// Create a new TextManager
    pub fn new() -> Self {
        Self {
            font_system: FontSystem::new(),
            swash_cache: SwashCache::new(),
            buffer_cache: HashMap::new(),
            default_font_family: "sans-serif".to_string(),
            default_font_size: 16.0,
        }
    }
    
    /// Set the default font family
    pub fn set_default_font_family(&mut self, family: String) {
        self.default_font_family = family;
    }
    
    /// Set the default font size
    pub fn set_default_font_size(&mut self, size: f32) {
        self.default_font_size = size;
    }
    
    /// Render rich text and return positioned glyphs
    pub fn render_rich_text(
        &mut self,
        rich_text: &RichText,
        max_width: Option<f32>,
        default_color: Vec4,
    ) -> RenderedText {
        // For now, create a new buffer each time to avoid borrow checker issues
        // TODO: Implement proper caching strategy
        let buffer = self.create_text_buffer(rich_text, max_width, default_color);
        self.extract_glyphs_from_buffer(&buffer, default_color)
    }
    
    /// Render simple text (backward compatibility)
    pub fn render_simple_text(
        &mut self,
        text: &str,
        font_size: f32,
        color: Vec4,
        max_width: Option<f32>,
    ) -> RenderedText {
        let rich_text = RichText::from_span(
            kryon_core::TextSpan::new(text)
                .with_font_size(font_size)
                .with_color(color)
        );
        
        self.render_rich_text(&rich_text, max_width, color)
    }
    
    /// Create a cosmic-text Buffer from RichText
    fn create_text_buffer(
        &mut self,
        rich_text: &RichText,
        max_width: Option<f32>,
        default_color: Vec4,
    ) -> Buffer {
        let metrics = Metrics::new(
            rich_text.line_height.unwrap_or(self.default_font_size),
            rich_text.line_height.unwrap_or(self.default_font_size * 1.2),
        );
        let mut buffer = Buffer::new(&mut self.font_system, metrics);
        
        // Set buffer width if specified
        if let Some(width) = max_width.or(rich_text.max_width) {
            buffer.set_size(&mut self.font_system, Some(width), None);
        }
        
        // Build the text content with proper attributes
        if rich_text.spans.is_empty() {
            return buffer;
        }
        
        // For cosmic-text, we need to set the entire text at once with proper spans
        let mut full_text = String::new();
        for span in &rich_text.spans {
            full_text.push_str(&span.text);
        }
        
        // Set the text with default attributes
        buffer.set_text(&mut self.font_system, &full_text, Attrs::new(), Shaping::Advanced);
        
        // For cosmic-text 0.13, we'll use a simplified approach
        // Apply the first span's attributes to the entire text
        if let Some(first_span) = rich_text.spans.first() {
            let attrs = self.span_to_attrs(first_span, default_color);
            if let Some(line) = buffer.lines.get_mut(0) {
                line.set_attrs_list(cosmic_text::AttrsList::new(attrs));
            }
        }
        
        // Set text alignment
        if let Some(alignment) = rich_text.alignment {
            let cosmic_align = match alignment {
                RichTextAlignment::Start => cosmic_text::Align::Left,
                RichTextAlignment::Center => cosmic_text::Align::Center,
                RichTextAlignment::End => cosmic_text::Align::Right,
                RichTextAlignment::Justify => cosmic_text::Align::Justified,
            };
            
            for line in buffer.lines.iter_mut() {
                line.set_align(Some(cosmic_align));
            }
        }
        
        // Shape the text
        buffer.shape_until_scroll(&mut self.font_system, false);
        
        buffer
    }
    
    /// Convert a TextSpan to cosmic-text Attrs
    fn span_to_attrs<'a>(&self, span: &'a TextSpan, default_color: Vec4) -> Attrs<'a> {
        let mut attrs = Attrs::new();
        
        // Font family
        if let Some(ref family_name) = span.font_family {
            attrs = attrs.family(Family::Name(family_name));
        } else {
            // Use a default family that doesn't borrow from self to avoid lifetime issues
            attrs = attrs.family(Family::SansSerif);
        }
        
        // Note: cosmic-text Attrs doesn't have a size() method
        // Font size is handled at the metrics level for the entire buffer
        
        // Font weight
        if let Some(weight) = span.font_weight {
            let cosmic_weight = match weight {
                RichFontWeight::Thin => Weight::THIN,
                RichFontWeight::ExtraLight => Weight::EXTRA_LIGHT,
                RichFontWeight::Light => Weight::LIGHT,
                RichFontWeight::Normal => Weight::NORMAL,
                RichFontWeight::Medium => Weight::MEDIUM,
                RichFontWeight::SemiBold => Weight::SEMIBOLD,
                RichFontWeight::Bold => Weight::BOLD,
                RichFontWeight::ExtraBold => Weight::EXTRA_BOLD,
                RichFontWeight::Black => Weight::BLACK,
            };
            attrs = attrs.weight(cosmic_weight);
        }
        
        // Font style
        if let Some(style) = span.font_style {
            let cosmic_style = match style {
                RichFontStyle::Normal => CosmicStyle::Normal,
                RichFontStyle::Italic => CosmicStyle::Italic,
                RichFontStyle::Oblique => CosmicStyle::Oblique,
            };
            attrs = attrs.style(cosmic_style);
        }
        
        // Text color
        let color = span.color.unwrap_or(default_color);
        let cosmic_color = CosmicColor::rgba(
            (color.x * 255.0) as u8,
            (color.y * 255.0) as u8,
            (color.z * 255.0) as u8,
            (color.w * 255.0) as u8,
        );
        attrs = attrs.color(cosmic_color);
        
        attrs
    }
    
    /// Extract positioned glyphs from a cosmic-text Buffer
    fn extract_glyphs_from_buffer(&mut self, buffer: &Buffer, default_color: Vec4) -> RenderedText {
        let mut glyphs = Vec::new();
        let mut max_x = 0.0_f32;
        let mut max_y = 0.0_f32;
        
        for run in buffer.layout_runs() {
            for glyph in run.glyphs.iter() {
                let position = Vec2::new(glyph.x, glyph.y);
                let size = Vec2::new(glyph.w as f32, run.line_height);
                
                // For now, use default color since extracting color from runs is complex
                let color = default_color;
                
                let positioned_glyph = PositionedGlyph {
                    position,
                    size,
                    color,
                    font_size: run.line_height / 1.2, // Approximate font size from line height
                    character: ' ', // Would need to track this from the original text
                    glyph_id: glyph.glyph_id as u32,
                    font_cache_key: format!("font_{}", 0), // Simplified
                };
                
                max_x = max_x.max(position.x + size.x);
                max_y = max_y.max(position.y + size.y);
                
                glyphs.push(positioned_glyph);
            }
        }
        
        RenderedText {
            glyphs,
            bounds: Vec2::new(max_x, max_y),
            line_height: buffer.metrics().line_height,
        }
    }
    
    /// Create a cache key for text rendering
    fn create_cache_key(&self, rich_text: &RichText, max_width: Option<f32>, default_color: Vec4) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        
        // Hash the text content
        rich_text.to_plain_text().hash(&mut hasher);
        
        // Hash span properties
        for span in &rich_text.spans {
            span.font_size.unwrap_or(0.0).to_bits().hash(&mut hasher);
            span.font_family.as_ref().unwrap_or(&self.default_font_family).hash(&mut hasher);
            if let Some(weight) = span.font_weight {
                (weight as u8).hash(&mut hasher);
            }
            if let Some(style) = span.font_style {
                (style as u8).hash(&mut hasher);
            }
        }
        
        // Hash layout properties
        rich_text.line_height.unwrap_or(0.0).to_bits().hash(&mut hasher);
        max_width.unwrap_or(0.0).to_bits().hash(&mut hasher);
        default_color.x.to_bits().hash(&mut hasher);
        default_color.y.to_bits().hash(&mut hasher);
        default_color.z.to_bits().hash(&mut hasher);
        default_color.w.to_bits().hash(&mut hasher);
        
        format!("text_{:x}", hasher.finish())
    }
    
    /// Clear the text cache (useful for memory management)
    pub fn clear_cache(&mut self) {
        self.buffer_cache.clear();
    }
    
    /// Get a reference to the SwashCache for glyph rasterization
    pub fn swash_cache(&mut self) -> &mut SwashCache {
        &mut self.swash_cache
    }
    
    /// Get a reference to the FontSystem
    pub fn font_system(&mut self) -> &mut FontSystem {
        &mut self.font_system
    }
}

impl Default for TextManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kryon_core::TextSpan;
    
    #[test]
    fn test_text_manager_creation() {
        let text_manager = TextManager::new();
        assert_eq!(text_manager.default_font_family, "sans-serif");
        assert_eq!(text_manager.default_font_size, 16.0);
    }
    
    #[test]
    fn test_simple_text_rendering() {
        let mut text_manager = TextManager::new();
        let rendered = text_manager.render_simple_text(
            "Hello, World!",
            16.0,
            Vec4::new(0.0, 0.0, 0.0, 1.0),
            None,
        );
        
        // Should have glyphs
        assert!(!rendered.glyphs.is_empty());
        assert!(rendered.bounds.x > 0.0);
        assert!(rendered.bounds.y > 0.0);
    }
    
    #[test]
    fn test_rich_text_rendering() {
        let mut text_manager = TextManager::new();
        
        let rich_text = RichText::new()
            .add_span(TextSpan::new("Regular text "))
            .add_span(
                TextSpan::new("Bold text")
                    .with_font_weight(RichFontWeight::Bold)
                    .with_color(Vec4::new(1.0, 0.0, 0.0, 1.0))
            );
        
        let rendered = text_manager.render_rich_text(
            &rich_text,
            None,
            Vec4::new(0.0, 0.0, 0.0, 1.0),
        );
        
        // Should have glyphs for both spans
        assert!(!rendered.glyphs.is_empty());
        assert!(rendered.bounds.x > 0.0);
    }
}
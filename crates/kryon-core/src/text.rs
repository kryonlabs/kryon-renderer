//! Rich text types and utilities for cosmic-text integration

use glam::Vec4;

/// A single styled text span for rich text rendering
#[derive(Debug, Clone, PartialEq)]
pub struct TextSpan {
    /// The text content of this span
    pub text: String,
    
    /// Text color (RGBA)
    pub color: Option<Vec4>,
    
    /// Font family name
    pub font_family: Option<String>,
    
    /// Font size in pixels
    pub font_size: Option<f32>,
    
    /// Font weight (normal, bold, etc.)
    pub font_weight: Option<RichFontWeight>,
    
    /// Font style (normal, italic)
    pub font_style: Option<RichFontStyle>,
    
    /// Text decoration (underline, strikethrough, etc.)
    pub text_decoration: Option<RichTextDecoration>,
    
    /// Letter spacing
    pub letter_spacing: Option<f32>,
    
    /// Background color for this span
    pub background_color: Option<Vec4>,
}

/// Font weight values for rich text
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RichFontWeight {
    Thin = 100,
    ExtraLight = 200,
    Light = 300,
    Normal = 400,
    Medium = 500,
    SemiBold = 600,
    Bold = 700,
    ExtraBold = 800,
    Black = 900,
}

/// Font style values for rich text
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RichFontStyle {
    Normal,
    Italic,
    Oblique,
}

/// Text decoration options for rich text
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RichTextDecoration {
    None,
    Underline,
    Overline,
    LineThrough,
}

/// Rich text containing multiple styled spans
#[derive(Debug, Clone, PartialEq)]
pub struct RichText {
    /// Collection of styled text spans
    pub spans: Vec<TextSpan>,
    
    /// Overall text alignment for the entire rich text block
    pub alignment: Option<RichTextAlignment>,
    
    /// Line height for the entire block
    pub line_height: Option<f32>,
    
    /// Maximum width for text wrapping
    pub max_width: Option<f32>,
}

/// Text alignment options for rich text  
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RichTextAlignment {
    Start,
    Center,
    End,
    Justify,
}

impl Default for TextSpan {
    fn default() -> Self {
        Self {
            text: String::new(),
            color: None,
            font_family: None,
            font_size: None,
            font_weight: None,
            font_style: None,
            text_decoration: None,
            letter_spacing: None,
            background_color: None,
        }
    }
}

impl Default for RichFontWeight {
    fn default() -> Self {
        RichFontWeight::Normal
    }
}

impl Default for RichFontStyle {
    fn default() -> Self {
        RichFontStyle::Normal
    }
}

impl Default for RichTextDecoration {
    fn default() -> Self {
        RichTextDecoration::None
    }
}

impl Default for RichTextAlignment {
    fn default() -> Self {
        RichTextAlignment::Start
    }
}

impl Default for RichText {
    fn default() -> Self {
        Self {
            spans: Vec::new(),
            alignment: None,
            line_height: None,
            max_width: None,
        }
    }
}

impl TextSpan {
    /// Create a new text span with just text content
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            ..Default::default()
        }
    }
    
    /// Set the color of this text span
    pub fn with_color(mut self, color: Vec4) -> Self {
        self.color = Some(color);
        self
    }
    
    /// Set the font size of this text span
    pub fn with_font_size(mut self, size: f32) -> Self {
        self.font_size = Some(size);
        self
    }
    
    /// Set the font weight of this text span
    pub fn with_font_weight(mut self, weight: RichFontWeight) -> Self {
        self.font_weight = Some(weight);
        self
    }
    
    /// Set the font style of this text span
    pub fn with_font_style(mut self, style: RichFontStyle) -> Self {
        self.font_style = Some(style);
        self
    }
    
    /// Set the text decoration of this text span
    pub fn with_text_decoration(mut self, decoration: RichTextDecoration) -> Self {
        self.text_decoration = Some(decoration);
        self
    }
}

impl RichText {
    /// Create a new empty rich text
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Create rich text from a single text span
    pub fn from_span(span: TextSpan) -> Self {
        Self {
            spans: vec![span],
            ..Default::default()
        }
    }
    
    /// Create rich text from multiple spans
    pub fn from_spans(spans: Vec<TextSpan>) -> Self {
        Self {
            spans,
            ..Default::default()
        }
    }
    
    /// Add a text span to this rich text
    pub fn add_span(mut self, span: TextSpan) -> Self {
        self.spans.push(span);
        self
    }
    
    /// Set the text alignment for the entire block
    pub fn with_alignment(mut self, alignment: RichTextAlignment) -> Self {
        self.alignment = Some(alignment);
        self
    }
    
    /// Set the line height for the entire block
    pub fn with_line_height(mut self, line_height: f32) -> Self {
        self.line_height = Some(line_height);
        self
    }
    
    /// Set the maximum width for text wrapping
    pub fn with_max_width(mut self, max_width: f32) -> Self {
        self.max_width = Some(max_width);
        self
    }
    
    /// Get the plain text content (all spans concatenated)
    pub fn to_plain_text(&self) -> String {
        self.spans.iter().map(|span| span.text.as_str()).collect::<Vec<_>>().join("")
    }
    
    /// Check if this rich text is empty
    pub fn is_empty(&self) -> bool {
        self.spans.is_empty() || self.spans.iter().all(|span| span.text.is_empty())
    }
}

impl From<String> for RichText {
    fn from(text: String) -> Self {
        Self::from_span(TextSpan::new(text))
    }
}

impl From<&str> for RichText {
    fn from(text: &str) -> Self {
        Self::from_span(TextSpan::new(text))
    }
}

impl From<Vec<String>> for RichText {
    /// Convert a vector of strings to rich text (for backward compatibility with multiline text)
    fn from(lines: Vec<String>) -> Self {
        let text = lines.join("\n");
        Self::from_span(TextSpan::new(text))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_text_span_creation() {
        let span = TextSpan::new("Hello, world!")
            .with_color(Vec4::new(1.0, 0.0, 0.0, 1.0))
            .with_font_size(16.0)
            .with_font_weight(RichFontWeight::Bold);
        
        assert_eq!(span.text, "Hello, world!");
        assert_eq!(span.color, Some(Vec4::new(1.0, 0.0, 0.0, 1.0)));
        assert_eq!(span.font_size, Some(16.0));
        assert_eq!(span.font_weight, Some(RichFontWeight::Bold));
    }
    
    #[test]
    fn test_rich_text_creation() {
        let rich_text = RichText::new()
            .add_span(TextSpan::new("Regular text "))
            .add_span(TextSpan::new("Bold text").with_font_weight(RichFontWeight::Bold))
            .with_alignment(RichTextAlignment::Center);
        
        assert_eq!(rich_text.spans.len(), 2);
        assert_eq!(rich_text.alignment, Some(RichTextAlignment::Center));
        assert_eq!(rich_text.to_plain_text(), "Regular text Bold text");
    }
    
    #[test]
    fn test_plain_text_conversion() {
        let text = "Simple text";
        let rich_text = RichText::from(text);
        
        assert_eq!(rich_text.spans.len(), 1);
        assert_eq!(rich_text.to_plain_text(), text);
    }
    
    #[test]
    fn test_multiline_text_conversion() {
        let lines = vec!["Line 1".to_string(), "Line 2".to_string()];
        let rich_text = RichText::from(lines);
        
        assert_eq!(rich_text.to_plain_text(), "Line 1\nLine 2");
    }
}
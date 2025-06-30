// crates/kryon-render/src/text.rs
use kryon_core::TextAlignment;

/// Text measurement and layout utilities
#[derive(Debug, Clone)]
pub struct TextMetrics {
    pub width: f32,
    pub height: f32,
    pub baseline: f32,
    pub line_height: f32,
}

impl TextMetrics {
    pub fn new(width: f32, height: f32, baseline: f32, line_height: f32) -> Self {
        Self {
            width,
            height,
            baseline,
            line_height,
        }
    }
}

/// Text layout information
#[derive(Debug, Clone)]
pub struct TextLayout {
    pub lines: Vec<TextLine>,
    pub total_width: f32,
    pub total_height: f32,
}

#[derive(Debug, Clone)]
pub struct TextLine {
    pub text: String,
    pub width: f32,
    pub height: f32,
    pub y_offset: f32,
}

/// Text shaping and measurement trait
pub trait TextShaper {
    fn measure_text(&self, text: &str, font_size: f32) -> TextMetrics;
    fn layout_text(&self, text: &str, font_size: f32, max_width: Option<f32>) -> TextLayout;
}

/// Simple text shaper implementation for basic text measurement
pub struct SimpleTextShaper {
    average_char_width: f32,
    line_height_multiplier: f32,
}

impl SimpleTextShaper {
    pub fn new() -> Self {
        Self {
            average_char_width: 0.6, // Approximate ratio of char width to font size
            line_height_multiplier: 1.2,
        }
    }
}

impl Default for SimpleTextShaper {
    fn default() -> Self {
        Self::new()
    }
}

impl TextShaper for SimpleTextShaper {
    fn measure_text(&self, text: &str, font_size: f32) -> TextMetrics {
        let char_width = font_size * self.average_char_width;
        let width = text.chars().count() as f32 * char_width;
        let height = font_size;
        let line_height = font_size * self.line_height_multiplier;
        let baseline = font_size * 0.8; // Approximate baseline position
        
        TextMetrics::new(width, height, baseline, line_height)
    }
    
    fn layout_text(&self, text: &str, font_size: f32, max_width: Option<f32>) -> TextLayout {
        let mut lines = Vec::new();
        let char_width = font_size * self.average_char_width;
        let line_height = font_size * self.line_height_multiplier;
        
        if let Some(max_w) = max_width {
            // Word wrapping
            let words: Vec<&str> = text.split_whitespace().collect();
            let mut current_line = String::new();
            let mut current_width = 0.0;
            let mut y_offset = 0.0;
            
            for word in words {
                let word_width = word.chars().count() as f32 * char_width;
                let space_width = char_width; // Width of space character
                
                if current_line.is_empty() {
                    // First word on line
                    current_line = word.to_string();
                    current_width = word_width;
                } else if current_width + space_width + word_width <= max_w {
                    // Word fits on current line
                    current_line.push(' ');
                    current_line.push_str(word);
                    current_width += space_width + word_width;
                } else {
                    // Word doesn't fit, start new line
                    lines.push(TextLine {
                        text: current_line.clone(),
                        width: current_width,
                        height: font_size,
                        y_offset,
                    });
                    
                    current_line = word.to_string();
                    current_width = word_width;
                    y_offset += line_height;
                }
            }
            
            // Add last line
            if !current_line.is_empty() {
                lines.push(TextLine {
                    text: current_line,
                    width: current_width,
                    height: font_size,
                    y_offset,
                });
            }
        } else {
            // No wrapping, single line
            let width = text.chars().count() as f32 * char_width;
            lines.push(TextLine {
                text: text.to_string(),
                width,
                height: font_size,
                y_offset: 0.0,
            });
        }
        
        let total_width = lines.iter().map(|line| line.width).fold(0.0, f32::max);
        let total_height = if lines.is_empty() {
            0.0
        } else {
            lines.last().unwrap().y_offset + font_size
        };
        
        TextLayout {
            lines,
            total_width,
            total_height,
        }
    }
}

/// Text alignment utilities
pub fn align_text_position(
    text_width: f32,
    container_width: f32,
    alignment: TextAlignment,
) -> f32 {
    match alignment {
        TextAlignment::Start => 0.0,
        TextAlignment::Center => (container_width - text_width) * 0.5,
        TextAlignment::End => container_width - text_width,
        TextAlignment::Justify => 0.0, // Justify is handled during layout
    }
}


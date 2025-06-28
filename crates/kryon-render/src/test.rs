// crates/kryon-render/src/text.rs
use kryon_core::TextAlignment as CoreTextAlignment;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAlignment {
    Start,
    Center,
    End,
    Justify,
}

impl From<CoreTextAlignment> for TextAlignment {
    fn from(alignment: CoreTextAlignment) -> Self {
        match alignment {
            CoreTextAlignment::Start => TextAlignment::Start,
            CoreTextAlignment::Center => TextAlignment::Center,
            CoreTextAlignment::End => TextAlignment::End,
            CoreTextAlignment::Justify => TextAlignment::Justify,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TextMetrics {
    pub width: f32,
    pub height: f32,
    pub baseline: f32,
}

pub trait TextRenderer {
    fn measure_text(&self, text: &str, font_size: f32, max_width: Option<f32>) -> TextMetrics;
    fn load_font(&mut self, name: &str, data: &[u8]) -> Result<(), Box<dyn std::error::Error>>;
}
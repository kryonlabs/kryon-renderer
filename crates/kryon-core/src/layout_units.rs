// crates/kryon-core/src/layout_units.rs
use glam::Vec2;

/// Represents a dimension that can be pixels, percentage, or auto
#[derive(Debug, Clone, PartialEq)]
pub enum LayoutDimension {
    /// Fixed pixel value
    Pixels(f32),
    /// Percentage of parent size (0.0 to 1.0)
    Percentage(f32),
    /// Automatic sizing based on content
    Auto,
    /// Fixed minimum size in pixels
    MinPixels(f32),
    /// Fixed maximum size in pixels
    MaxPixels(f32),
}

impl LayoutDimension {
    /// Convert to pixels given a parent size
    pub fn to_pixels(&self, parent_size: f32) -> f32 {
        match self {
            LayoutDimension::Pixels(px) => *px,
            LayoutDimension::Percentage(pct) => pct * parent_size,
            LayoutDimension::Auto => parent_size, // For now, auto fills parent
            LayoutDimension::MinPixels(px) => *px,
            LayoutDimension::MaxPixels(px) => *px,
        }
    }
    
    /// Check if this dimension is definite (not auto)
    pub fn is_definite(&self) -> bool {
        !matches!(self, LayoutDimension::Auto)
    }
    
    /// Check if this dimension depends on parent size
    pub fn depends_on_parent(&self) -> bool {
        matches!(self, LayoutDimension::Percentage(_) | LayoutDimension::Auto)
    }
    
    /// Create from a string value (like "50%", "100px", "auto")
    pub fn from_string(value: &str) -> Self {
        let value = value.trim();
        
        if value == "auto" {
            return LayoutDimension::Auto;
        }
        
        if value.ends_with('%') {
            if let Ok(pct) = value[..value.len()-1].parse::<f32>() {
                return LayoutDimension::Percentage(pct / 100.0);
            }
        }
        
        if value.ends_with("px") {
            if let Ok(px) = value[..value.len()-2].parse::<f32>() {
                return LayoutDimension::Pixels(px);
            }
        }
        
        // Try parsing as plain number (assume pixels)
        if let Ok(px) = value.parse::<f32>() {
            return LayoutDimension::Pixels(px);
        }
        
        // Default to auto if parsing fails
        LayoutDimension::Auto
    }
}

impl Default for LayoutDimension {
    fn default() -> Self {
        LayoutDimension::Auto
    }
}

/// Represents a 2D size with flexible dimensions
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutSize {
    pub width: LayoutDimension,
    pub height: LayoutDimension,
}

impl LayoutSize {
    pub fn new(width: LayoutDimension, height: LayoutDimension) -> Self {
        Self { width, height }
    }
    
    pub fn pixels(width: f32, height: f32) -> Self {
        Self {
            width: LayoutDimension::Pixels(width),
            height: LayoutDimension::Pixels(height),
        }
    }
    
    pub fn percentage(width_pct: f32, height_pct: f32) -> Self {
        Self {
            width: LayoutDimension::Percentage(width_pct / 100.0),
            height: LayoutDimension::Percentage(height_pct / 100.0),
        }
    }
    
    pub fn auto() -> Self {
        Self {
            width: LayoutDimension::Auto,
            height: LayoutDimension::Auto,
        }
    }
    
    /// Convert to pixel Vec2 given parent size
    pub fn to_pixels(&self, parent_size: Vec2) -> Vec2 {
        Vec2::new(
            self.width.to_pixels(parent_size.x),
            self.height.to_pixels(parent_size.y),
        )
    }
    
    /// Check if both dimensions are definite
    pub fn is_definite(&self) -> bool {
        self.width.is_definite() && self.height.is_definite()
    }
    
    /// Check if this size depends on parent
    pub fn depends_on_parent(&self) -> bool {
        self.width.depends_on_parent() || self.height.depends_on_parent()
    }
}

impl Default for LayoutSize {
    fn default() -> Self {
        LayoutSize::auto()
    }
}

/// Represents a 2D position with flexible dimensions
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutPosition {
    pub x: LayoutDimension,
    pub y: LayoutDimension,
}

impl LayoutPosition {
    pub fn new(x: LayoutDimension, y: LayoutDimension) -> Self {
        Self { x, y }
    }
    
    pub fn pixels(x: f32, y: f32) -> Self {
        Self {
            x: LayoutDimension::Pixels(x),
            y: LayoutDimension::Pixels(y),
        }
    }
    
    pub fn percentage(x_pct: f32, y_pct: f32) -> Self {
        Self {
            x: LayoutDimension::Percentage(x_pct / 100.0),
            y: LayoutDimension::Percentage(y_pct / 100.0),
        }
    }
    
    pub fn zero() -> Self {
        Self {
            x: LayoutDimension::Pixels(0.0),
            y: LayoutDimension::Pixels(0.0),
        }
    }
    
    /// Convert to pixel Vec2 given parent size
    pub fn to_pixels(&self, parent_size: Vec2) -> Vec2 {
        Vec2::new(
            self.x.to_pixels(parent_size.x),
            self.y.to_pixels(parent_size.y),
        )
    }
    
    /// Check if position is zero (pixels only)
    pub fn is_zero(&self) -> bool {
        matches!(self.x, LayoutDimension::Pixels(x) if x == 0.0) &&
        matches!(self.y, LayoutDimension::Pixels(y) if y == 0.0)
    }
}

impl Default for LayoutPosition {
    fn default() -> Self {
        LayoutPosition::zero()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_dimension_parsing() {
        assert_eq!(LayoutDimension::from_string("50%"), LayoutDimension::Percentage(0.5));
        assert_eq!(LayoutDimension::from_string("100px"), LayoutDimension::Pixels(100.0));
        assert_eq!(LayoutDimension::from_string("auto"), LayoutDimension::Auto);
        assert_eq!(LayoutDimension::from_string("200"), LayoutDimension::Pixels(200.0));
    }
    
    #[test]
    fn test_dimension_to_pixels() {
        let parent_size = 400.0;
        
        assert_eq!(LayoutDimension::Pixels(100.0).to_pixels(parent_size), 100.0);
        assert_eq!(LayoutDimension::Percentage(0.5).to_pixels(parent_size), 200.0);
        assert_eq!(LayoutDimension::Auto.to_pixels(parent_size), 400.0);
    }
    
    #[test]
    fn test_layout_size() {
        let parent_size = Vec2::new(800.0, 600.0);
        let size = LayoutSize::percentage(50.0, 75.0);
        
        let pixel_size = size.to_pixels(parent_size);
        assert_eq!(pixel_size, Vec2::new(400.0, 450.0));
    }
}
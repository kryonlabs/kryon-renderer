// crates/kryon-render/src/primitives.rs
use glam::Vec2;

/// Basic geometric primitives for rendering
#[derive(Debug, Clone)]
pub struct Rect {
    pub position: Vec2,
    pub size: Vec2,
}

impl Rect {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            position: Vec2::new(x, y),
            size: Vec2::new(width, height),
        }
    }
    
    pub fn from_position_size(position: Vec2, size: Vec2) -> Self {
        Self { position, size }
    }
    
    pub fn contains_point(&self, point: Vec2) -> bool {
        point.x >= self.position.x 
            && point.x <= self.position.x + self.size.x
            && point.y >= self.position.y 
            && point.y <= self.position.y + self.size.y
    }
    
    pub fn intersects(&self, other: &Rect) -> bool {
        !(self.position.x + self.size.x < other.position.x
            || other.position.x + other.size.x < self.position.x
            || self.position.y + self.size.y < other.position.y
            || other.position.y + other.size.y < self.position.y)
    }
    
    pub fn center(&self) -> Vec2 {
        self.position + self.size * 0.5
    }
    
    pub fn min(&self) -> Vec2 {
        self.position
    }
    
    pub fn max(&self) -> Vec2 {
        self.position + self.size
    }
}

#[derive(Debug, Clone)]
pub struct RoundedRect {
    pub rect: Rect,
    pub border_radius: f32,
}

impl RoundedRect {
    pub fn new(position: Vec2, size: Vec2, border_radius: f32) -> Self {
        Self {
            rect: Rect::from_position_size(position, size),
            border_radius,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Circle {
    pub center: Vec2,
    pub radius: f32,
}

impl Circle {
    pub fn new(center: Vec2, radius: f32) -> Self {
        Self { center, radius }
    }
    
    pub fn contains_point(&self, point: Vec2) -> bool {
        self.center.distance(point) <= self.radius
    }
}

#[derive(Debug, Clone)]
pub struct Line {
    pub start: Vec2,
    pub end: Vec2,
    pub width: f32,
}

impl Line {
    pub fn new(start: Vec2, end: Vec2, width: f32) -> Self {
        Self { start, end, width }
    }
    
    pub fn length(&self) -> f32 {
        self.start.distance(self.end)
    }
    
    pub fn direction(&self) -> Vec2 {
        (self.end - self.start).normalize()
    }
}

/// Color utilities
pub mod color {
    use glam::Vec4;
    
    pub const TRANSPARENT: Vec4 = Vec4::new(0.0, 0.0, 0.0, 0.0);
    pub const BLACK: Vec4 = Vec4::new(0.0, 0.0, 0.0, 1.0);
    pub const WHITE: Vec4 = Vec4::new(1.0, 1.0, 1.0, 1.0);
    pub const RED: Vec4 = Vec4::new(1.0, 0.0, 0.0, 1.0);
    pub const GREEN: Vec4 = Vec4::new(0.0, 1.0, 0.0, 1.0);
    pub const BLUE: Vec4 = Vec4::new(0.0, 0.0, 1.0, 1.0);
    pub const GRAY: Vec4 = Vec4::new(0.5, 0.5, 0.5, 1.0);
    
    pub fn from_hex(hex: u32) -> Vec4 {
        let r = ((hex >> 24) & 0xFF) as f32 / 255.0;
        let g = ((hex >> 16) & 0xFF) as f32 / 255.0;
        let b = ((hex >> 8) & 0xFF) as f32 / 255.0;
        let a = (hex & 0xFF) as f32 / 255.0;
        Vec4::new(r, g, b, a)
    }
    
    pub fn from_rgb(r: u8, g: u8, b: u8) -> Vec4 {
        Vec4::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0)
    }
    
    pub fn from_rgba(r: u8, g: u8, b: u8, a: u8) -> Vec4 {
        Vec4::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, a as f32 / 255.0)
    }
}
// crates/kryon-layout/src/constraints.rs

#[derive(Debug, Clone, Copy)]
pub struct ConstraintBox {
    pub min_width: f32,
    pub max_width: f32,
    pub min_height: f32,
    pub max_height: f32,
    pub definite_width: bool,
    pub definite_height: bool,
}

impl Default for ConstraintBox {
    fn default() -> Self {
        Self {
            min_width: 0.0,
            max_width: f32::INFINITY,
            min_height: 0.0,
            max_height: f32::INFINITY,
            definite_width: false,
            definite_height: false,
        }
    }
}

impl ConstraintBox {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn with_max_size(width: f32, height: f32) -> Self {
        Self {
            min_width: 0.0,
            max_width: width,
            min_height: 0.0,
            max_height: height,
            definite_width: true,
            definite_height: true,
        }
    }
    
    pub fn with_fixed_size(width: f32, height: f32) -> Self {
        Self {
            min_width: width,
            max_width: width,
            min_height: height,
            max_height: height,
            definite_width: true,
            definite_height: true,
        }
    }
    
    pub fn constrain_width(&self, width: f32) -> f32 {
        width.clamp(self.min_width, self.max_width)
    }
    
    pub fn constrain_height(&self, height: f32) -> f32 {
        height.clamp(self.min_height, self.max_height)
    }
    
    pub fn is_width_constrained(&self) -> bool {
        self.max_width != f32::INFINITY
    }
    
    pub fn is_height_constrained(&self) -> bool {
        self.max_height != f32::INFINITY
    }
}
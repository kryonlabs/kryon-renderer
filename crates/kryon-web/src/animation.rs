//! Animation and transition system for web rendering

use wasm_bindgen::prelude::*;
use glam::{Vec2, Vec4};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Animation {
    pub id: String,
    pub target: AnimationTarget,
    pub keyframes: Vec<Keyframe>,
    pub duration: f64,
    pub delay: f64,
    pub easing: EasingFunction,
    pub fill_mode: FillMode,
    pub direction: AnimationDirection,
    pub iteration_count: IterationCount,
    pub play_state: PlayState,
    pub start_time: f64,
    pub current_time: f64,
}

#[derive(Debug, Clone)]
pub enum AnimationTarget {
    Element(String), // Element ID
    Property(String, String), // Element ID, Property name
    Custom(String), // Custom animation target
}

#[derive(Debug, Clone)]
pub struct Keyframe {
    pub time: f64, // 0.0 to 1.0
    pub properties: HashMap<String, AnimationValue>,
    pub easing: Option<EasingFunction>,
}

#[derive(Debug, Clone)]
pub enum AnimationValue {
    Float(f32),
    Vec2(Vec2),
    Vec4(Vec4),
    Color(Vec4),
    String(String),
    Bool(bool),
}

#[derive(Debug, Clone)]
pub enum EasingFunction {
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
    CubicBezier(f32, f32, f32, f32),
    Spring(f32, f32), // stiffness, damping
    Bounce,
    Elastic,
    Custom(String), // Custom easing function name
}

#[derive(Debug, Clone)]
pub enum FillMode {
    None,
    Forwards,
    Backwards,
    Both,
}

#[derive(Debug, Clone)]
pub enum AnimationDirection {
    Normal,
    Reverse,
    Alternate,
    AlternateReverse,
}

#[derive(Debug, Clone)]
pub enum IterationCount {
    Finite(u32),
    Infinite,
}

#[derive(Debug, Clone)]
pub enum PlayState {
    Running,
    Paused,
    Finished,
}

pub struct AnimationSystem {
    animations: HashMap<String, Animation>,
    transitions: HashMap<String, Transition>,
    custom_easings: HashMap<String, Box<dyn Fn(f64) -> f64>>,
    time: f64,
    delta_time: f64,
}

#[derive(Debug, Clone)]
pub struct Transition {
    pub id: String,
    pub target: AnimationTarget,
    pub property: String,
    pub from_value: AnimationValue,
    pub to_value: AnimationValue,
    pub duration: f64,
    pub delay: f64,
    pub easing: EasingFunction,
    pub start_time: f64,
    pub current_time: f64,
    pub state: PlayState,
}

impl AnimationSystem {
    pub fn new() -> Self {
        let mut system = Self {
            animations: HashMap::new(),
            transitions: HashMap::new(),
            custom_easings: HashMap::new(),
            time: 0.0,
            delta_time: 0.0,
        };
        
        // Register default easing functions
        system.register_default_easings();
        
        system
    }
    
    fn register_default_easings(&mut self) {
        self.custom_easings.insert("bounce".to_string(), Box::new(|t| {
            if t < 1.0 / 2.75 {
                7.5625 * t * t
            } else if t < 2.0 / 2.75 {
                let t = t - 1.5 / 2.75;
                7.5625 * t * t + 0.75
            } else if t < 2.5 / 2.75 {
                let t = t - 2.25 / 2.75;
                7.5625 * t * t + 0.9375
            } else {
                let t = t - 2.625 / 2.75;
                7.5625 * t * t + 0.984375
            }
        }));
        
        self.custom_easings.insert("elastic".to_string(), Box::new(|t| {
            if t == 0.0 || t == 1.0 {
                t
            } else {
                let p = 0.3;
                let s = p / 4.0;
                let t = t - 1.0;
                -((2.0_f64).powf(10.0 * t) * ((t - s) * (2.0 * std::f64::consts::PI) / p).sin())
            }
        }));
    }
    
    pub fn update(&mut self, current_time: f64) {
        self.delta_time = current_time - self.time;
        self.time = current_time;
        
        // Update animations
        let mut finished_animations = Vec::new();
        
        for (id, animation) in self.animations.iter_mut() {
            if animation.play_state == PlayState::Running {
                animation.current_time += self.delta_time;
                
                let progress = self.calculate_animation_progress(animation);
                
                if progress >= 1.0 {
                    match animation.iteration_count {
                        IterationCount::Finite(count) => {
                            // Handle finite iterations
                            if animation.current_time >= animation.duration * count as f64 {
                                animation.play_state = PlayState::Finished;
                                finished_animations.push(id.clone());
                            }
                        }
                        IterationCount::Infinite => {
                            // Reset for infinite animations
                            animation.current_time = animation.current_time % animation.duration;
                        }
                    }
                }
            }
        }
        
        // Update transitions
        let mut finished_transitions = Vec::new();
        
        for (id, transition) in self.transitions.iter_mut() {
            if transition.state == PlayState::Running {
                transition.current_time += self.delta_time;
                
                let progress = (transition.current_time - transition.delay) / transition.duration;
                
                if progress >= 1.0 {
                    transition.state = PlayState::Finished;
                    finished_transitions.push(id.clone());
                }
            }
        }
        
        // Clean up finished animations and transitions
        for id in finished_animations {
            self.animations.remove(&id);
        }
        
        for id in finished_transitions {
            self.transitions.remove(&id);
        }
    }
    
    fn calculate_animation_progress(&self, animation: &Animation) -> f64 {
        let adjusted_time = animation.current_time - animation.delay;
        if adjusted_time < 0.0 {
            return 0.0;
        }
        
        let progress = (adjusted_time % animation.duration) / animation.duration;
        
        match animation.direction {
            AnimationDirection::Normal => progress,
            AnimationDirection::Reverse => 1.0 - progress,
            AnimationDirection::Alternate => {
                let cycle = (adjusted_time / animation.duration) as u32;
                if cycle % 2 == 0 { progress } else { 1.0 - progress }
            }
            AnimationDirection::AlternateReverse => {
                let cycle = (adjusted_time / animation.duration) as u32;
                if cycle % 2 == 0 { 1.0 - progress } else { progress }
            }
        }
    }
    
    pub fn create_animation(&mut self, mut animation: Animation) -> String {
        animation.start_time = self.time;
        let id = animation.id.clone();
        self.animations.insert(id.clone(), animation);
        id
    }
    
    pub fn create_transition(&mut self, mut transition: Transition) -> String {
        transition.start_time = self.time;
        let id = transition.id.clone();
        self.transitions.insert(id.clone(), transition);
        id
    }
    
    pub fn play_animation(&mut self, id: &str) {
        if let Some(animation) = self.animations.get_mut(id) {
            animation.play_state = PlayState::Running;
        }
    }
    
    pub fn pause_animation(&mut self, id: &str) {
        if let Some(animation) = self.animations.get_mut(id) {
            animation.play_state = PlayState::Paused;
        }
    }
    
    pub fn stop_animation(&mut self, id: &str) {
        self.animations.remove(id);
    }
    
    pub fn get_animated_value(&self, element_id: &str, property: &str) -> Option<AnimationValue> {
        // Check transitions first
        for transition in self.transitions.values() {
            if let AnimationTarget::Property(ref target_id, ref target_prop) = transition.target {
                if target_id == element_id && target_prop == property && transition.state == PlayState::Running {
                    let progress = (transition.current_time - transition.delay) / transition.duration;
                    let progress = progress.clamp(0.0, 1.0);
                    let eased_progress = self.apply_easing(progress, &transition.easing);
                    
                    return Some(self.interpolate_value(&transition.from_value, &transition.to_value, eased_progress));
                }
            }
        }
        
        // Check animations
        for animation in self.animations.values() {
            if let AnimationTarget::Element(ref target_id) = animation.target {
                if target_id == element_id && animation.play_state == PlayState::Running {
                    let progress = self.calculate_animation_progress(animation);
                    let eased_progress = self.apply_easing(progress, &animation.easing);
                    
                    // Find the appropriate keyframe
                    if let Some(value) = self.get_keyframe_value(animation, property, eased_progress) {
                        return Some(value);
                    }
                }
            }
        }
        
        None
    }
    
    fn get_keyframe_value(&self, animation: &Animation, property: &str, progress: f64) -> Option<AnimationValue> {
        let mut prev_keyframe = None;
        let mut next_keyframe = None;
        
        for keyframe in &animation.keyframes {
            if keyframe.time <= progress {
                prev_keyframe = Some(keyframe);
            } else {
                next_keyframe = Some(keyframe);
                break;
            }
        }
        
        match (prev_keyframe, next_keyframe) {
            (Some(prev), Some(next)) => {
                if let (Some(prev_value), Some(next_value)) = (prev.properties.get(property), next.properties.get(property)) {
                    let t = (progress - prev.time) / (next.time - prev.time);
                    let easing = next.easing.as_ref().unwrap_or(&animation.easing);
                    let eased_t = self.apply_easing(t, easing);
                    Some(self.interpolate_value(prev_value, next_value, eased_t))
                } else {
                    None
                }
            }
            (Some(keyframe), None) => {
                keyframe.properties.get(property).cloned()
            }
            (None, Some(keyframe)) => {
                keyframe.properties.get(property).cloned()
            }
            (None, None) => None,
        }
    }
    
    fn apply_easing(&self, t: f64, easing: &EasingFunction) -> f64 {
        match easing {
            EasingFunction::Linear => t,
            EasingFunction::EaseIn => t * t,
            EasingFunction::EaseOut => 1.0 - (1.0 - t) * (1.0 - t),
            EasingFunction::EaseInOut => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    1.0 - 2.0 * (1.0 - t) * (1.0 - t)
                }
            }
            EasingFunction::CubicBezier(x1, y1, x2, y2) => {
                self.cubic_bezier(t, *x1, *y1, *x2, *y2)
            }
            EasingFunction::Spring(stiffness, damping) => {
                self.spring_easing(t, *stiffness, *damping)
            }
            EasingFunction::Bounce => {
                if let Some(easing_fn) = self.custom_easings.get("bounce") {
                    easing_fn(t)
                } else {
                    t
                }
            }
            EasingFunction::Elastic => {
                if let Some(easing_fn) = self.custom_easings.get("elastic") {
                    easing_fn(t)
                } else {
                    t
                }
            }
            EasingFunction::Custom(name) => {
                if let Some(easing_fn) = self.custom_easings.get(name) {
                    easing_fn(t)
                } else {
                    t
                }
            }
        }
    }
    
    fn cubic_bezier(&self, t: f64, x1: f32, y1: f32, x2: f32, y2: f32) -> f64 {
        // Simplified cubic bezier implementation
        let t = t as f32;
        let u = 1.0 - t;
        let tt = t * t;
        let uu = u * u;
        let uuu = uu * u;
        let ttt = tt * t;
        
        let p = uuu * 0.0 + 3.0 * uu * t * y1 + 3.0 * u * tt * y2 + ttt * 1.0;
        p as f64
    }
    
    fn spring_easing(&self, t: f64, stiffness: f32, damping: f32) -> f64 {
        let t = t as f32;
        let omega = (stiffness / damping).sqrt();
        let zeta = damping / (2.0 * stiffness.sqrt());
        
        if zeta < 1.0 {
            // Underdamped
            let omega_d = omega * (1.0 - zeta * zeta).sqrt();
            1.0 - ((-zeta * omega * t).exp() * (omega_d * t).cos()) as f64
        } else {
            // Overdamped or critically damped
            1.0 - ((-omega * t).exp()) as f64
        }
    }
    
    fn interpolate_value(&self, from: &AnimationValue, to: &AnimationValue, t: f64) -> AnimationValue {
        let t = t as f32;
        
        match (from, to) {
            (AnimationValue::Float(a), AnimationValue::Float(b)) => {
                AnimationValue::Float(a + (b - a) * t)
            }
            (AnimationValue::Vec2(a), AnimationValue::Vec2(b)) => {
                AnimationValue::Vec2(a.lerp(*b, t))
            }
            (AnimationValue::Vec4(a), AnimationValue::Vec4(b)) => {
                AnimationValue::Vec4(a.lerp(*b, t))
            }
            (AnimationValue::Color(a), AnimationValue::Color(b)) => {
                AnimationValue::Color(a.lerp(*b, t))
            }
            (AnimationValue::Bool(a), AnimationValue::Bool(b)) => {
                AnimationValue::Bool(if t < 0.5 { *a } else { *b })
            }
            _ => to.clone(),
        }
    }
    
    pub fn register_custom_easing(&mut self, name: String, easing_fn: Box<dyn Fn(f64) -> f64>) {
        self.custom_easings.insert(name, easing_fn);
    }
    
    pub fn animate_property(&mut self, element_id: &str, property: &str, to_value: AnimationValue, duration: f64, easing: EasingFunction) -> String {
        let id = format!("{}_{}_transition", element_id, property);
        
        // For now, we'll use a default "from" value
        let from_value = match to_value {
            AnimationValue::Float(_) => AnimationValue::Float(0.0),
            AnimationValue::Vec2(_) => AnimationValue::Vec2(Vec2::ZERO),
            AnimationValue::Vec4(_) => AnimationValue::Vec4(Vec4::ZERO),
            AnimationValue::Color(_) => AnimationValue::Color(Vec4::ZERO),
            AnimationValue::Bool(_) => AnimationValue::Bool(false),
            AnimationValue::String(_) => AnimationValue::String(String::new()),
        };
        
        let transition = Transition {
            id: id.clone(),
            target: AnimationTarget::Property(element_id.to_string(), property.to_string()),
            property: property.to_string(),
            from_value,
            to_value,
            duration,
            delay: 0.0,
            easing,
            start_time: self.time,
            current_time: 0.0,
            state: PlayState::Running,
        };
        
        self.transitions.insert(id.clone(), transition);
        id
    }
    
    pub fn is_animating(&self, element_id: &str, property: &str) -> bool {
        // Check transitions
        for transition in self.transitions.values() {
            if let AnimationTarget::Property(ref target_id, ref target_prop) = transition.target {
                if target_id == element_id && target_prop == property && transition.state == PlayState::Running {
                    return true;
                }
            }
        }
        
        // Check animations
        for animation in self.animations.values() {
            if let AnimationTarget::Element(ref target_id) = animation.target {
                if target_id == element_id && animation.play_state == PlayState::Running {
                    // Check if this animation affects the property
                    for keyframe in &animation.keyframes {
                        if keyframe.properties.contains_key(property) {
                            return true;
                        }
                    }
                }
            }
        }
        
        false
    }
    
    pub fn get_animation_count(&self) -> usize {
        self.animations.len()
    }
    
    pub fn get_transition_count(&self) -> usize {
        self.transitions.len()
    }
    
    pub fn clear_all(&mut self) {
        self.animations.clear();
        self.transitions.clear();
    }
}

// Helper functions for creating common animations

impl Animation {
    pub fn fade_in(element_id: &str, duration: f64) -> Self {
        let mut keyframes = Vec::new();
        
        let mut start_properties = HashMap::new();
        start_properties.insert("opacity".to_string(), AnimationValue::Float(0.0));
        keyframes.push(Keyframe {
            time: 0.0,
            properties: start_properties,
            easing: None,
        });
        
        let mut end_properties = HashMap::new();
        end_properties.insert("opacity".to_string(), AnimationValue::Float(1.0));
        keyframes.push(Keyframe {
            time: 1.0,
            properties: end_properties,
            easing: None,
        });
        
        Self {
            id: format!("{}_fade_in", element_id),
            target: AnimationTarget::Element(element_id.to_string()),
            keyframes,
            duration,
            delay: 0.0,
            easing: EasingFunction::EaseOut,
            fill_mode: FillMode::Forwards,
            direction: AnimationDirection::Normal,
            iteration_count: IterationCount::Finite(1),
            play_state: PlayState::Running,
            start_time: 0.0,
            current_time: 0.0,
        }
    }
    
    pub fn slide_in(element_id: &str, from: Vec2, to: Vec2, duration: f64) -> Self {
        let mut keyframes = Vec::new();
        
        let mut start_properties = HashMap::new();
        start_properties.insert("position".to_string(), AnimationValue::Vec2(from));
        keyframes.push(Keyframe {
            time: 0.0,
            properties: start_properties,
            easing: None,
        });
        
        let mut end_properties = HashMap::new();
        end_properties.insert("position".to_string(), AnimationValue::Vec2(to));
        keyframes.push(Keyframe {
            time: 1.0,
            properties: end_properties,
            easing: None,
        });
        
        Self {
            id: format!("{}_slide_in", element_id),
            target: AnimationTarget::Element(element_id.to_string()),
            keyframes,
            duration,
            delay: 0.0,
            easing: EasingFunction::EaseOut,
            fill_mode: FillMode::Forwards,
            direction: AnimationDirection::Normal,
            iteration_count: IterationCount::Finite(1),
            play_state: PlayState::Running,
            start_time: 0.0,
            current_time: 0.0,
        }
    }
    
    pub fn pulse(element_id: &str, duration: f64) -> Self {
        let mut keyframes = Vec::new();
        
        let mut start_properties = HashMap::new();
        start_properties.insert("scale".to_string(), AnimationValue::Vec2(Vec2::ONE));
        keyframes.push(Keyframe {
            time: 0.0,
            properties: start_properties,
            easing: None,
        });
        
        let mut mid_properties = HashMap::new();
        mid_properties.insert("scale".to_string(), AnimationValue::Vec2(Vec2::new(1.1, 1.1)));
        keyframes.push(Keyframe {
            time: 0.5,
            properties: mid_properties,
            easing: None,
        });
        
        let mut end_properties = HashMap::new();
        end_properties.insert("scale".to_string(), AnimationValue::Vec2(Vec2::ONE));
        keyframes.push(Keyframe {
            time: 1.0,
            properties: end_properties,
            easing: None,
        });
        
        Self {
            id: format!("{}_pulse", element_id),
            target: AnimationTarget::Element(element_id.to_string()),
            keyframes,
            duration,
            delay: 0.0,
            easing: EasingFunction::EaseInOut,
            fill_mode: FillMode::None,
            direction: AnimationDirection::Normal,
            iteration_count: IterationCount::Infinite,
            play_state: PlayState::Running,
            start_time: 0.0,
            current_time: 0.0,
        }
    }
}
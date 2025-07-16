//! Performance profiler for web rendering

use wasm_bindgen::prelude::*;
use web_sys::console;
use std::collections::{HashMap, VecDeque};

#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    pub frame_time: f64,
    pub render_time: f64,
    pub animation_time: f64,
    pub event_processing_time: f64,
    pub memory_usage: f64,
    pub draw_calls: u32,
    pub triangles_rendered: u32,
    pub textures_loaded: u32,
    pub shaders_compiled: u32,
}

#[derive(Debug, Clone)]
pub struct FrameStats {
    pub fps: f64,
    pub frame_time_avg: f64,
    pub frame_time_min: f64,
    pub frame_time_max: f64,
    pub render_time_avg: f64,
    pub memory_usage_mb: f64,
    pub total_draw_calls: u32,
    pub total_triangles: u32,
}

pub struct PerformanceProfiler {
    enabled: bool,
    frame_history: VecDeque<PerformanceMetrics>,
    history_size: usize,
    current_frame: PerformanceMetrics,
    timers: HashMap<String, f64>,
    counters: HashMap<String, u32>,
    last_gc_time: f64,
    gc_interval: f64,
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self {
            frame_time: 0.0,
            render_time: 0.0,
            animation_time: 0.0,
            event_processing_time: 0.0,
            memory_usage: 0.0,
            draw_calls: 0,
            triangles_rendered: 0,
            textures_loaded: 0,
            shaders_compiled: 0,
        }
    }
}

impl PerformanceProfiler {
    pub fn new() -> Self {
        Self {
            enabled: true,
            frame_history: VecDeque::new(),
            history_size: 60, // Keep 60 frames of history
            current_frame: PerformanceMetrics::default(),
            timers: HashMap::new(),
            counters: HashMap::new(),
            last_gc_time: 0.0,
            gc_interval: 1000.0, // 1 second
        }
    }
    
    pub fn enable(&mut self) {
        self.enabled = true;
    }
    
    pub fn disable(&mut self) {
        self.enabled = false;
    }
    
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
    
    pub fn begin_frame(&mut self, timestamp: f64) {
        if !self.enabled {
            return;
        }
        
        self.current_frame = PerformanceMetrics::default();
        self.timers.insert("frame".to_string(), timestamp);
        
        // Update memory usage
        self.current_frame.memory_usage = self.get_memory_usage();
        
        // Periodic garbage collection of old data
        if timestamp - self.last_gc_time > self.gc_interval {
            self.cleanup_old_data();
            self.last_gc_time = timestamp;
        }
    }
    
    pub fn end_frame(&mut self, timestamp: f64) {
        if !self.enabled {
            return;
        }
        
        if let Some(start_time) = self.timers.get("frame") {
            self.current_frame.frame_time = timestamp - start_time;
        }
        
        // Add to history
        self.frame_history.push_back(self.current_frame.clone());
        
        // Keep history size manageable
        if self.frame_history.len() > self.history_size {
            self.frame_history.pop_front();
        }
        
        // Clear timers and counters for next frame
        self.timers.clear();
        self.counters.clear();
    }
    
    pub fn begin_timer(&mut self, name: &str) {
        if !self.enabled {
            return;
        }
        
        let timestamp = self.get_timestamp();
        self.timers.insert(name.to_string(), timestamp);
    }
    
    pub fn end_timer(&mut self, name: &str) {
        if !self.enabled {
            return;
        }
        
        let timestamp = self.get_timestamp();
        
        if let Some(start_time) = self.timers.get(name) {
            let duration = timestamp - start_time;
            
            match name {
                "render" => self.current_frame.render_time += duration,
                "animation" => self.current_frame.animation_time += duration,
                "events" => self.current_frame.event_processing_time += duration,
                _ => {}
            }
        }
    }
    
    pub fn increment_counter(&mut self, name: &str, value: u32) {
        if !self.enabled {
            return;
        }
        
        let current = self.counters.get(name).unwrap_or(&0);
        self.counters.insert(name.to_string(), current + value);
        
        match name {
            "draw_calls" => self.current_frame.draw_calls += value,
            "triangles" => self.current_frame.triangles_rendered += value,
            "textures" => self.current_frame.textures_loaded += value,
            "shaders" => self.current_frame.shaders_compiled += value,
            _ => {}
        }
    }
    
    pub fn get_frame_stats(&self) -> FrameStats {
        if self.frame_history.is_empty() {
            return FrameStats {
                fps: 0.0,
                frame_time_avg: 0.0,
                frame_time_min: 0.0,
                frame_time_max: 0.0,
                render_time_avg: 0.0,
                memory_usage_mb: 0.0,
                total_draw_calls: 0,
                total_triangles: 0,
            };
        }
        
        let frame_times: Vec<f64> = self.frame_history.iter().map(|f| f.frame_time).collect();
        let render_times: Vec<f64> = self.frame_history.iter().map(|f| f.render_time).collect();
        
        let frame_time_avg = frame_times.iter().sum::<f64>() / frame_times.len() as f64;
        let frame_time_min = frame_times.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let frame_time_max = frame_times.iter().fold(0.0, |a, &b| a.max(b));
        let render_time_avg = render_times.iter().sum::<f64>() / render_times.len() as f64;
        
        let fps = if frame_time_avg > 0.0 {
            1000.0 / frame_time_avg
        } else {
            0.0
        };
        
        let total_draw_calls = self.frame_history.iter().map(|f| f.draw_calls).sum();
        let total_triangles = self.frame_history.iter().map(|f| f.triangles_rendered).sum();
        
        FrameStats {
            fps,
            frame_time_avg,
            frame_time_min,
            frame_time_max,
            render_time_avg,
            memory_usage_mb: self.get_memory_usage(),
            total_draw_calls,
            total_triangles,
        }
    }
    
    pub fn get_current_metrics(&self) -> &PerformanceMetrics {
        &self.current_frame
    }
    
    pub fn get_frame_history(&self) -> &VecDeque<PerformanceMetrics> {
        &self.frame_history
    }
    
    pub fn log_performance(&self) {
        if !self.enabled {
            return;
        }
        
        let stats = self.get_frame_stats();
        
        console::log_1(&format!(
            "Performance: FPS: {:.1}, Frame: {:.2}ms, Render: {:.2}ms, Memory: {:.1}MB, Draw Calls: {}, Triangles: {}",
            stats.fps,
            stats.frame_time_avg,
            stats.render_time_avg,
            stats.memory_usage_mb,
            stats.total_draw_calls,
            stats.total_triangles
        ).into());
    }
    
    pub fn export_data(&self) -> JsValue {
        let stats = self.get_frame_stats();
        
        let data = js_sys::Object::new();
        js_sys::Reflect::set(&data, &JsValue::from_str("fps"), &JsValue::from_f64(stats.fps)).unwrap();
        js_sys::Reflect::set(&data, &JsValue::from_str("frameTimeAvg"), &JsValue::from_f64(stats.frame_time_avg)).unwrap();
        js_sys::Reflect::set(&data, &JsValue::from_str("frameTimeMin"), &JsValue::from_f64(stats.frame_time_min)).unwrap();
        js_sys::Reflect::set(&data, &JsValue::from_str("frameTimeMax"), &JsValue::from_f64(stats.frame_time_max)).unwrap();
        js_sys::Reflect::set(&data, &JsValue::from_str("renderTimeAvg"), &JsValue::from_f64(stats.render_time_avg)).unwrap();
        js_sys::Reflect::set(&data, &JsValue::from_str("memoryUsageMb"), &JsValue::from_f64(stats.memory_usage_mb)).unwrap();
        js_sys::Reflect::set(&data, &JsValue::from_str("drawCalls"), &JsValue::from_f64(stats.total_draw_calls as f64)).unwrap();
        js_sys::Reflect::set(&data, &JsValue::from_str("triangles"), &JsValue::from_f64(stats.total_triangles as f64)).unwrap();
        
        // Add frame history
        let history_array = js_sys::Array::new();
        for frame in &self.frame_history {
            let frame_obj = js_sys::Object::new();
            js_sys::Reflect::set(&frame_obj, &JsValue::from_str("frameTime"), &JsValue::from_f64(frame.frame_time)).unwrap();
            js_sys::Reflect::set(&frame_obj, &JsValue::from_str("renderTime"), &JsValue::from_f64(frame.render_time)).unwrap();
            js_sys::Reflect::set(&frame_obj, &JsValue::from_str("animationTime"), &JsValue::from_f64(frame.animation_time)).unwrap();
            js_sys::Reflect::set(&frame_obj, &JsValue::from_str("drawCalls"), &JsValue::from_f64(frame.draw_calls as f64)).unwrap();
            js_sys::Reflect::set(&frame_obj, &JsValue::from_str("triangles"), &JsValue::from_f64(frame.triangles_rendered as f64)).unwrap();
            history_array.push(&frame_obj);
        }
        js_sys::Reflect::set(&data, &JsValue::from_str("frameHistory"), &history_array).unwrap();
        
        data.into()
    }
    
    pub fn create_performance_chart(&self) -> JsValue {
        let chart_data = js_sys::Object::new();
        
        // FPS over time
        let fps_data = js_sys::Array::new();
        for (i, frame) in self.frame_history.iter().enumerate() {
            let point = js_sys::Object::new();
            js_sys::Reflect::set(&point, &JsValue::from_str("x"), &JsValue::from_f64(i as f64)).unwrap();
            let fps = if frame.frame_time > 0.0 { 1000.0 / frame.frame_time } else { 0.0 };
            js_sys::Reflect::set(&point, &JsValue::from_str("y"), &JsValue::from_f64(fps)).unwrap();
            fps_data.push(&point);
        }
        js_sys::Reflect::set(&chart_data, &JsValue::from_str("fps"), &fps_data).unwrap();
        
        // Frame time over time
        let frame_time_data = js_sys::Array::new();
        for (i, frame) in self.frame_history.iter().enumerate() {
            let point = js_sys::Object::new();
            js_sys::Reflect::set(&point, &JsValue::from_str("x"), &JsValue::from_f64(i as f64)).unwrap();
            js_sys::Reflect::set(&point, &JsValue::from_str("y"), &JsValue::from_f64(frame.frame_time)).unwrap();
            frame_time_data.push(&point);
        }
        js_sys::Reflect::set(&chart_data, &JsValue::from_str("frameTime"), &frame_time_data).unwrap();
        
        // Memory usage over time
        let memory_data = js_sys::Array::new();
        for (i, frame) in self.frame_history.iter().enumerate() {
            let point = js_sys::Object::new();
            js_sys::Reflect::set(&point, &JsValue::from_str("x"), &JsValue::from_f64(i as f64)).unwrap();
            js_sys::Reflect::set(&point, &JsValue::from_str("y"), &JsValue::from_f64(frame.memory_usage)).unwrap();
            memory_data.push(&point);
        }
        js_sys::Reflect::set(&chart_data, &JsValue::from_str("memory"), &memory_data).unwrap();
        
        chart_data.into()
    }
    
    fn get_timestamp(&self) -> f64 {
        let window = web_sys::window().unwrap();
        let performance = window.performance().unwrap();
        performance.now()
    }
    
    fn get_memory_usage(&self) -> f64 {
        let window = web_sys::window().unwrap();
        if let Ok(performance) = js_sys::Reflect::get(&window, &JsValue::from_str("performance")) {
            if let Ok(memory) = js_sys::Reflect::get(&performance, &JsValue::from_str("memory")) {
                if let Ok(used_heap) = js_sys::Reflect::get(&memory, &JsValue::from_str("usedJSHeapSize")) {
                    if let Some(used) = used_heap.as_f64() {
                        return used / 1024.0 / 1024.0; // Convert to MB
                    }
                }
            }
        }
        0.0
    }
    
    fn cleanup_old_data(&mut self) {
        // Remove old performance data to prevent memory leaks
        if self.frame_history.len() > self.history_size * 2 {
            self.frame_history.truncate(self.history_size);
        }
        
        // Clear any stale timers
        self.timers.retain(|_, &mut timestamp| {
            let now = self.get_timestamp();
            now - timestamp < 1000.0 // Keep timers for max 1 second
        });
    }
    
    pub fn reset(&mut self) {
        self.frame_history.clear();
        self.timers.clear();
        self.counters.clear();
        self.current_frame = PerformanceMetrics::default();
    }
    
    pub fn get_performance_warnings(&self) -> Vec<String> {
        let mut warnings = Vec::new();
        let stats = self.get_frame_stats();
        
        if stats.fps < 30.0 {
            warnings.push("Low FPS detected - consider optimizing render performance".to_string());
        }
        
        if stats.frame_time_avg > 33.0 {
            warnings.push("High frame time - target 16.67ms for 60fps".to_string());
        }
        
        if stats.memory_usage_mb > 100.0 {
            warnings.push("High memory usage - consider reducing texture/mesh data".to_string());
        }
        
        if stats.total_draw_calls > 1000 {
            warnings.push("High draw call count - consider batching geometry".to_string());
        }
        
        if stats.render_time_avg > 16.0 {
            warnings.push("High render time - optimize shaders or reduce complexity".to_string());
        }
        
        warnings
    }
}

// Performance utilities for measuring specific operations
pub struct ScopedTimer<'a> {
    profiler: &'a mut PerformanceProfiler,
    timer_name: String,
}

impl<'a> ScopedTimer<'a> {
    pub fn new(profiler: &'a mut PerformanceProfiler, name: &str) -> Self {
        profiler.begin_timer(name);
        Self {
            profiler,
            timer_name: name.to_string(),
        }
    }
}

impl<'a> Drop for ScopedTimer<'a> {
    fn drop(&mut self) {
        self.profiler.end_timer(&self.timer_name);
    }
}

// Macros for easier profiling
#[macro_export]
macro_rules! profile_scope {
    ($profiler:expr, $name:expr) => {
        let _timer = ScopedTimer::new($profiler, $name);
    };
}

#[macro_export]
macro_rules! profile_function {
    ($profiler:expr) => {
        let _timer = ScopedTimer::new($profiler, &format!("{}:{}", file!(), line!()));
    };
}
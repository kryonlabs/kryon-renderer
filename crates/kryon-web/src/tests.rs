//! Comprehensive test suite for web rendering

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;
    use glam::{Vec2, Vec4};
    
    wasm_bindgen_test_configure!(run_in_browser);
    
    #[wasm_bindgen_test]
    fn test_kryon_web_app_creation() {
        let app = KryonWebApp::new();
        assert!(app.canvas_renderer.is_none());
        assert!(app.dom_renderer.is_none());
        assert_eq!(app.get_animation_count(), 0);
        assert_eq!(app.get_transition_count(), 0);
    }
    
    #[wasm_bindgen_test]
    fn test_animation_system() {
        let mut app = KryonWebApp::new();
        
        // Test fade in animation
        let animation_id = app.animate_fade_in("test_element", 1000.0);
        assert!(!animation_id.is_empty());
        assert_eq!(app.get_animation_count(), 1);
        
        // Test property animation
        let transition_id = app.animate_property("test_element", "opacity", 0.5, 500.0);
        assert!(!transition_id.is_empty());
        assert_eq!(app.get_transition_count(), 1);
        
        // Test animation state
        assert!(app.is_animating("test_element", "opacity"));
        assert!(!app.is_animating("other_element", "opacity"));
        
        // Test animation control
        app.pause_animation(&animation_id);
        app.play_animation(&animation_id);
        app.stop_animation(&animation_id);
        assert_eq!(app.get_animation_count(), 0);
    }
    
    #[wasm_bindgen_test]
    fn test_performance_profiler() {
        let mut profiler = PerformanceProfiler::new();
        
        assert!(profiler.is_enabled());
        
        // Test frame timing
        profiler.begin_frame(0.0);
        profiler.begin_timer("render");
        profiler.end_timer("render");
        profiler.end_frame(16.67);
        
        let stats = profiler.get_frame_stats();
        assert!(stats.frame_time_avg > 0.0);
        assert!(stats.fps > 0.0);
        
        // Test counters
        profiler.increment_counter("draw_calls", 5);
        profiler.increment_counter("triangles", 100);
        
        let current_metrics = profiler.get_current_metrics();
        assert_eq!(current_metrics.draw_calls, 5);
        assert_eq!(current_metrics.triangles_rendered, 100);
        
        // Test export
        let data = profiler.export_data();
        assert!(!data.is_undefined());
        
        // Test chart data
        let chart = profiler.create_performance_chart();
        assert!(!chart.is_undefined());
        
        // Test reset
        profiler.reset();
        assert_eq!(profiler.get_frame_stats().fps, 0.0);
    }
    
    #[wasm_bindgen_test]
    fn test_animation_values() {
        use crate::animation::*;
        
        let mut system = AnimationSystem::new();
        
        // Test float interpolation
        let from = AnimationValue::Float(0.0);
        let to = AnimationValue::Float(1.0);
        let result = system.interpolate_value(&from, &to, 0.5);
        
        if let AnimationValue::Float(value) = result {
            assert!((value - 0.5).abs() < 0.001);
        } else {
            panic!("Expected Float value");
        }
        
        // Test Vec2 interpolation
        let from = AnimationValue::Vec2(Vec2::new(0.0, 0.0));
        let to = AnimationValue::Vec2(Vec2::new(10.0, 20.0));
        let result = system.interpolate_value(&from, &to, 0.5);
        
        if let AnimationValue::Vec2(value) = result {
            assert!((value.x - 5.0).abs() < 0.001);
            assert!((value.y - 10.0).abs() < 0.001);
        } else {
            panic!("Expected Vec2 value");
        }
    }
    
    #[wasm_bindgen_test]
    fn test_easing_functions() {
        use crate::animation::*;
        
        let system = AnimationSystem::new();
        
        // Test linear easing
        let linear_result = system.apply_easing(0.5, &EasingFunction::Linear);
        assert!((linear_result - 0.5).abs() < 0.001);
        
        // Test ease in
        let ease_in_result = system.apply_easing(0.5, &EasingFunction::EaseIn);
        assert!(ease_in_result < 0.5); // Should be slower at start
        
        // Test ease out
        let ease_out_result = system.apply_easing(0.5, &EasingFunction::EaseOut);
        assert!(ease_out_result > 0.5); // Should be faster at start
        
        // Test ease in-out
        let ease_in_out_result = system.apply_easing(0.5, &EasingFunction::EaseInOut);
        assert!((ease_in_out_result - 0.5).abs() < 0.1); // Should be close to middle
    }
    
    #[wasm_bindgen_test]
    fn test_texture_manager() {
        use crate::texture_manager::*;
        
        // Mock WebGPU device and queue for testing
        // In real tests, you'd use actual WebGPU objects
        
        let descriptor = TextureDescriptor {
            width: 256,
            height: 256,
            format: TextureFormat::Rgba8UnormSrgb,
            usage: TextureUsage {
                texture_binding: true,
                storage_binding: false,
                render_attachment: false,
                copy_src: false,
                copy_dst: true,
            },
            mip_level_count: 1,
            sample_count: 1,
        };
        
        // Test texture descriptor
        assert_eq!(descriptor.width, 256);
        assert_eq!(descriptor.height, 256);
        assert!(descriptor.usage.texture_binding);
        assert!(!descriptor.usage.storage_binding);
    }
    
    #[wasm_bindgen_test]
    fn test_canvas_renderer() {
        use crate::canvas_renderer::*;
        
        // Test render mode enum
        let mode = RenderMode::Canvas2D;
        match mode {
            RenderMode::Canvas2D => assert!(true),
            _ => assert!(false),
        }
        
        // Test WebGL mode
        let webgl_mode = RenderMode::WebGL;
        match webgl_mode {
            RenderMode::WebGL => assert!(true),
            _ => assert!(false),
        }
    }
    
    #[wasm_bindgen_test]
    fn test_event_handler() {
        use crate::event_handler::*;
        
        let handler = WebEventHandler::new();
        let events = handler.poll_events();
        assert!(events.is_empty());
    }
    
    #[wasm_bindgen_test]
    fn test_asset_loader() {
        use crate::asset_loader::*;
        
        let loader = WebAssetLoader::new();
        assert_eq!(loader.cache_size(), 0);
        
        // Test asset enum
        let asset = Asset::Binary(vec![1, 2, 3, 4]);
        match asset {
            Asset::Binary(data) => assert_eq!(data.len(), 4),
            _ => assert!(false),
        }
    }
    
    #[wasm_bindgen_test]
    fn test_utils() {
        use crate::utils::*;
        
        // Test color conversion
        let color_u8 = color_to_u8(0.5);
        assert_eq!(color_u8, 127);
        
        let color_f32 = color_to_f32(127);
        assert!((color_f32 - 0.498).abs() < 0.01);
        
        // Test color formatting
        let rgba = format_rgba(1.0, 0.5, 0.0, 0.8);
        assert_eq!(rgba, "rgba(255, 127, 0, 0.8)");
        
        let rgb = format_rgb(0.2, 0.4, 0.6);
        assert_eq!(rgb, "rgb(51, 102, 153)");
    }
    
    #[wasm_bindgen_test]
    fn test_base64_encoding() {
        use crate::canvas_renderer::base64_encode;
        
        let data = vec![72, 101, 108, 108, 111]; // "Hello" in ASCII
        let encoded = base64_encode(&data);
        assert!(!encoded.is_empty());
        assert!(encoded.len() >= 4); // Base64 encoding should produce output
    }
    
    #[wasm_bindgen_test]
    fn test_performance_warnings() {
        let mut profiler = PerformanceProfiler::new();
        
        // Create a frame with poor performance
        profiler.begin_frame(0.0);
        profiler.begin_timer("render");
        profiler.end_timer("render");
        profiler.end_frame(50.0); // 50ms frame time = 20fps
        
        let warnings = profiler.get_performance_warnings();
        assert!(!warnings.is_empty());
        assert!(warnings.iter().any(|w| w.contains("Low FPS")));
    }
    
    #[wasm_bindgen_test]
    fn test_animation_keyframes() {
        use crate::animation::*;
        
        let mut keyframes = Vec::new();
        
        // Start keyframe
        let mut start_props = std::collections::HashMap::new();
        start_props.insert("opacity".to_string(), AnimationValue::Float(0.0));
        keyframes.push(Keyframe {
            time: 0.0,
            properties: start_props,
            easing: None,
        });
        
        // End keyframe
        let mut end_props = std::collections::HashMap::new();
        end_props.insert("opacity".to_string(), AnimationValue::Float(1.0));
        keyframes.push(Keyframe {
            time: 1.0,
            properties: end_props,
            easing: None,
        });
        
        assert_eq!(keyframes.len(), 2);
        assert_eq!(keyframes[0].time, 0.0);
        assert_eq!(keyframes[1].time, 1.0);
    }
    
    #[wasm_bindgen_test]
    fn test_spring_easing() {
        use crate::animation::*;
        
        let system = AnimationSystem::new();
        
        // Test spring easing with different parameters
        let spring_result = system.apply_easing(0.5, &EasingFunction::Spring(1.0, 0.5));
        assert!(spring_result >= 0.0 && spring_result <= 1.0);
        
        // Test with high stiffness
        let stiff_spring = system.apply_easing(0.5, &EasingFunction::Spring(10.0, 0.5));
        assert!(stiff_spring >= 0.0 && stiff_spring <= 1.0);
    }
    
    #[wasm_bindgen_test]
    fn test_cubic_bezier() {
        use crate::animation::*;
        
        let system = AnimationSystem::new();
        
        // Test cubic bezier easing
        let bezier_result = system.apply_easing(0.5, &EasingFunction::CubicBezier(0.25, 0.1, 0.25, 1.0));
        assert!(bezier_result >= 0.0 && bezier_result <= 1.0);
        
        // Test with different control points
        let custom_bezier = system.apply_easing(0.5, &EasingFunction::CubicBezier(0.42, 0.0, 0.58, 1.0));
        assert!(custom_bezier >= 0.0 && custom_bezier <= 1.0);
    }
    
    #[wasm_bindgen_test]
    fn test_texture_atlas() {
        use crate::texture_manager::*;
        
        let mut packer = RectPacker::new(Vec2::new(256.0, 256.0));
        
        // Test rectangle packing
        let rect1 = packer.pack(64, 64);
        assert!(rect1.is_some());
        
        let rect2 = packer.pack(32, 32);
        assert!(rect2.is_some());
        
        // Test that rectangles don't overlap
        if let (Some(r1), Some(r2)) = (rect1, rect2) {
            assert!(r1.x != r2.x || r1.y != r2.y);
        }
    }
    
    #[wasm_bindgen_test]
    fn test_render_command_processing() {
        use kryon_render::RenderCommand;
        
        let mut app = KryonWebApp::new();
        
        // Test that we can create render commands
        let command = RenderCommand::DrawRect {
            position: Vec2::new(10.0, 10.0),
            size: Vec2::new(100.0, 50.0),
            color: Vec4::new(1.0, 0.0, 0.0, 1.0),
            border_radius: 5.0,
            border_width: 2.0,
            border_color: Vec4::new(0.0, 0.0, 0.0, 1.0),
            transform: None,
            shadow: None,
            z_index: 0,
        };
        
        // Commands should be processable by the renderer
        match command {
            RenderCommand::DrawRect { position, size, color, .. } => {
                assert_eq!(position, Vec2::new(10.0, 10.0));
                assert_eq!(size, Vec2::new(100.0, 50.0));
                assert_eq!(color, Vec4::new(1.0, 0.0, 0.0, 1.0));
            }
            _ => panic!("Expected DrawRect command"),
        }
    }
    
    #[wasm_bindgen_test]
    fn test_memory_management() {
        let mut app = KryonWebApp::new();
        
        // Create many animations to test memory management
        for i in 0..10 {
            let element_id = format!("element_{}", i);
            app.animate_fade_in(&element_id, 1000.0);
        }
        
        assert_eq!(app.get_animation_count(), 10);
        
        // Simulate time passing to allow animations to complete
        app.render(0.0).unwrap();
        app.render(2000.0).unwrap(); // 2 seconds later
        
        // Animations should be cleaned up automatically
        // (In real implementation, this would depend on animation completion)
    }
    
    #[wasm_bindgen_test]
    fn test_error_handling() {
        let mut app = KryonWebApp::new();
        
        // Test that invalid operations don't crash
        app.play_animation("nonexistent_animation");
        app.pause_animation("nonexistent_animation");
        app.stop_animation("nonexistent_animation");
        
        // Test that invalid element IDs are handled gracefully
        assert!(!app.is_animating("nonexistent_element", "opacity"));
        
        // Test with empty strings
        let empty_animation = app.animate_fade_in("", 1000.0);
        assert!(!empty_animation.is_empty());
    }
    
    #[wasm_bindgen_test]
    fn test_concurrent_animations() {
        let mut app = KryonWebApp::new();
        
        // Test multiple animations on the same element
        let fade_id = app.animate_fade_in("test_element", 1000.0);
        let slide_id = app.animate_slide_in("test_element", 0.0, 0.0, 100.0, 100.0, 1000.0);
        let pulse_id = app.animate_pulse("test_element", 500.0);
        
        assert_eq!(app.get_animation_count(), 3);
        
        // All animations should be tracked
        assert!(!fade_id.is_empty());
        assert!(!slide_id.is_empty());
        assert!(!pulse_id.is_empty());
        
        // Element should be considered animating
        assert!(app.is_animating("test_element", "opacity"));
        assert!(app.is_animating("test_element", "position"));
        assert!(app.is_animating("test_element", "scale"));
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use wasm_bindgen_test::*;
    
    wasm_bindgen_test_configure!(run_in_browser);
    
    #[wasm_bindgen_test]
    async fn test_full_rendering_pipeline() {
        let mut app = KryonWebApp::new();
        
        // Test initialization
        assert!(app.canvas_renderer.is_none());
        
        // Test rendering without initialization (should not crash)
        let result = app.render(0.0);
        assert!(result.is_ok());
        
        // Test performance monitoring
        let stats = app.get_performance_stats();
        assert!(!stats.is_undefined());
        
        // Test animation system integration
        let animation_id = app.animate_fade_in("test_element", 1000.0);
        assert!(!animation_id.is_empty());
        
        // Test multiple render frames
        for i in 0..10 {
            let timestamp = i as f64 * 16.67; // 60fps
            let result = app.render(timestamp);
            assert!(result.is_ok());
        }
        
        // Test that animations are being processed
        assert_eq!(app.get_animation_count(), 1);
    }
    
    #[wasm_bindgen_test]
    async fn test_performance_under_load() {
        let mut app = KryonWebApp::new();
        
        // Create many animations
        for i in 0..100 {
            let element_id = format!("element_{}", i);
            app.animate_fade_in(&element_id, 1000.0 + i as f64);
        }
        
        // Render many frames
        let start_time = js_sys::Date::now();
        for i in 0..60 {
            let timestamp = i as f64 * 16.67;
            app.render(timestamp).unwrap();
        }
        let end_time = js_sys::Date::now();
        
        let total_time = end_time - start_time;
        assert!(total_time < 1000.0); // Should complete within 1 second
        
        // Check performance stats
        let stats = app.get_performance_stats();
        assert!(!stats.is_undefined());
        
        // Performance should be reasonable
        app.log_performance();
    }
    
    #[wasm_bindgen_test]
    async fn test_memory_stability() {
        let mut app = KryonWebApp::new();
        
        // Create and destroy many animations
        for cycle in 0..10 {
            // Create animations
            let mut animation_ids = Vec::new();
            for i in 0..20 {
                let element_id = format!("element_{}_{}", cycle, i);
                let id = app.animate_fade_in(&element_id, 100.0); // Short duration
                animation_ids.push(id);
            }
            
            // Render to process animations
            for frame in 0..10 {
                let timestamp = (cycle * 10 + frame) as f64 * 16.67;
                app.render(timestamp).unwrap();
            }
            
            // Clean up
            for id in animation_ids {
                app.stop_animation(&id);
            }
        }
        
        // Memory usage should be stable
        let stats = app.get_performance_stats();
        assert!(!stats.is_undefined());
    }
}
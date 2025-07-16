//! Utility functions for web rendering

use wasm_bindgen::prelude::*;
use web_sys::{Window, Document, Performance};
use glam::Vec2;

/// Get the current timestamp in milliseconds
pub fn get_timestamp() -> f64 {
    let window = web_sys::window().unwrap();
    let performance = window.performance().unwrap();
    performance.now()
}

/// Get the device pixel ratio for high-DPI displays
pub fn get_device_pixel_ratio() -> f64 {
    let window = web_sys::window().unwrap();
    window.device_pixel_ratio()
}

/// Convert CSS pixels to device pixels
pub fn css_to_device_pixels(css_pixels: f32) -> f32 {
    css_pixels * get_device_pixel_ratio() as f32
}

/// Convert device pixels to CSS pixels
pub fn device_to_css_pixels(device_pixels: f32) -> f32 {
    device_pixels / get_device_pixel_ratio() as f32
}

/// Get the viewport size in CSS pixels
pub fn get_viewport_size() -> Vec2 {
    let window = web_sys::window().unwrap();
    let width = window.inner_width().unwrap_or(JsValue::from(800)).as_f64().unwrap_or(800.0);
    let height = window.inner_height().unwrap_or(JsValue::from(600)).as_f64().unwrap_or(600.0);
    Vec2::new(width as f32, height as f32)
}

/// Check if the browser supports WebGL2
pub fn supports_webgl2() -> bool {
    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    let canvas = document.create_element("canvas").unwrap();
    let canvas = canvas.dyn_into::<web_sys::HtmlCanvasElement>().unwrap();
    
    canvas.get_context("webgl2").is_ok()
}

/// Check if the browser supports WebGPU
pub fn supports_webgpu() -> bool {
    let window = web_sys::window().unwrap();
    js_sys::Reflect::has(&window, &JsValue::from_str("navigator")).unwrap_or(false) &&
    js_sys::Reflect::has(&js_sys::Reflect::get(&window, &JsValue::from_str("navigator")).unwrap(), &JsValue::from_str("gpu")).unwrap_or(false)
}

/// Convert a color from 0-1 range to 0-255 range
pub fn color_to_u8(color: f32) -> u8 {
    (color.clamp(0.0, 1.0) * 255.0) as u8
}

/// Convert a color from 0-255 range to 0-1 range
pub fn color_to_f32(color: u8) -> f32 {
    color as f32 / 255.0
}

/// Format a color as a CSS rgba string
pub fn format_rgba(r: f32, g: f32, b: f32, a: f32) -> String {
    format!(
        "rgba({}, {}, {}, {})",
        color_to_u8(r),
        color_to_u8(g),
        color_to_u8(b),
        a.clamp(0.0, 1.0)
    )
}

/// Format a color as a CSS rgb string
pub fn format_rgb(r: f32, g: f32, b: f32) -> String {
    format!(
        "rgb({}, {}, {})",
        color_to_u8(r),
        color_to_u8(g),
        color_to_u8(b)
    )
}

/// Log a message to the browser console
pub fn log(message: &str) {
    web_sys::console::log_1(&JsValue::from_str(message));
}

/// Log an error to the browser console
pub fn error(message: &str) {
    web_sys::console::error_1(&JsValue::from_str(message));
}

/// Log a warning to the browser console
pub fn warn(message: &str) {
    web_sys::console::warn_1(&JsValue::from_str(message));
}

/// Check if we're running in a secure context (HTTPS)
pub fn is_secure_context() -> bool {
    let window = web_sys::window().unwrap();
    window.is_secure_context()
}

/// Get the user agent string
pub fn get_user_agent() -> String {
    let window = web_sys::window().unwrap();
    let navigator = window.navigator();
    navigator.user_agent().unwrap_or_else(|_| "Unknown".to_string())
}

/// Check if the browser is on a mobile device
pub fn is_mobile() -> bool {
    let user_agent = get_user_agent().to_lowercase();
    user_agent.contains("mobile") || 
    user_agent.contains("android") || 
    user_agent.contains("iphone") || 
    user_agent.contains("ipad")
}

/// Request an animation frame
pub fn request_animation_frame(callback: &js_sys::Function) -> Result<i32, JsValue> {
    let window = web_sys::window().unwrap();
    window.request_animation_frame(callback)
}

/// Cancel an animation frame request
pub fn cancel_animation_frame(id: i32) {
    let window = web_sys::window().unwrap();
    window.cancel_animation_frame(id);
}

/// Set up a panic hook for better error reporting in the browser
pub fn setup_panic_hook() {
    console_error_panic_hook::set_once();
}

/// Convert a JavaScript array to a Rust Vec<u8>
pub fn js_array_to_vec(array: &js_sys::Uint8Array) -> Vec<u8> {
    array.to_vec()
}

/// Convert a Rust Vec<u8> to a JavaScript Uint8Array
pub fn vec_to_js_array(vec: &[u8]) -> js_sys::Uint8Array {
    js_sys::Uint8Array::from(vec)
}

/// Measure text width using canvas context
pub fn measure_text_width(text: &str, font: &str) -> Result<f64, JsValue> {
    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    let canvas = document.create_element("canvas")?;
    let canvas = canvas.dyn_into::<web_sys::HtmlCanvasElement>()?;
    let context = canvas.get_context("2d")?
        .unwrap()
        .dyn_into::<web_sys::CanvasRenderingContext2d>()?;
    
    context.set_font(font);
    let metrics = context.measure_text(text)?;
    Ok(metrics.width())
}

/// Get the current page URL
pub fn get_current_url() -> String {
    let window = web_sys::window().unwrap();
    let location = window.location();
    location.href().unwrap_or_else(|_| "about:blank".to_string())
}

/// Check if localStorage is available
pub fn has_local_storage() -> bool {
    let window = web_sys::window().unwrap();
    window.local_storage().is_ok()
}

/// Store data in localStorage
pub fn store_local_data(key: &str, value: &str) -> Result<(), JsValue> {
    let window = web_sys::window().unwrap();
    let storage = window.local_storage()?.ok_or("No localStorage")?;
    storage.set_item(key, value)?;
    Ok(())
}

/// Retrieve data from localStorage
pub fn get_local_data(key: &str) -> Result<Option<String>, JsValue> {
    let window = web_sys::window().unwrap();
    let storage = window.local_storage()?.ok_or("No localStorage")?;
    storage.get_item(key)
}
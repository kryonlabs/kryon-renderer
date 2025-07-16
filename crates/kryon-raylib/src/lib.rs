// crates/kryon-raylib/src/lib.rs
use kryon_render::{
    Renderer, CommandRenderer, RenderCommand, RenderResult, InputEvent, MouseButton, KeyCode, KeyModifiers, TextManager
};
use kryon_core::{CursorType, TransformData, TransformPropertyType, CSSUnit};
use kryon_layout::LayoutResult;
use glam::{Vec2, Vec4};
use raylib::prelude::*;
use raylib::ffi;
use std::collections::HashMap;
use kryon_render::RenderError;

pub struct RaylibRenderer {
    handle: RaylibHandle,
    thread: RaylibThread,
    size: Vec2,
    textures: HashMap<String, Texture2D>,
    fonts: HashMap<String, Font>,  // Font cache: font_family_name -> Font
    font_paths: HashMap<String, String>,  // Font mappings: font_family_name -> file_path
    text_manager: TextManager,  // Cosmic-text integration
    pending_commands: Vec<RenderCommand>,
    prev_mouse_pos: Vec2,
    current_cursor: CursorType,
}

pub struct RaylibRenderContext {
    // Empty context - commands are stored in renderer
}

impl Renderer for RaylibRenderer {
    type Surface = (i32, i32, String); // (width, height, title)
    type Context = RaylibRenderContext;
    
    fn initialize(surface: Self::Surface) -> RenderResult<Self> where Self: Sized {
        let (width, height, title) = surface;
        let (mut rl, thread) = raylib::init()
            .size(width, height)
            .title(&title)
            .build();
        
        rl.set_target_fps(60);
        
        // Enable mouse cursor and ensure window can receive input
        rl.show_cursor();
        
        eprintln!("[RAYLIB_INIT] Window initialized: {}x{}, cursor visible: {}", 
            width, height, !rl.is_cursor_hidden());
        
        Ok(Self {
            handle: rl,
            thread,
            size: Vec2::new(width as f32, height as f32),
            textures: HashMap::new(),
            fonts: HashMap::new(),
            font_paths: HashMap::new(),
            text_manager: TextManager::new(),
            pending_commands: Vec::new(),
            prev_mouse_pos: Vec2::new(-1.0, -1.0), // Initialize to invalid position
            current_cursor: CursorType::Default,
        })
    }
    
    fn begin_frame(&mut self, _clear_color: Vec4) -> RenderResult<Self::Context> {
        self.pending_commands.clear();
        Ok(RaylibRenderContext {})
    }
    
    fn end_frame(&mut self, _context: Self::Context) -> RenderResult<()> {
        // Execute all pending commands in one drawing session
        let commands = std::mem::take(&mut self.pending_commands); // Move commands out
        
        // Commands are already sorted by z_index in the render pipeline
        
        {
            let mut d = self.handle.begin_drawing(&self.thread);
            
            // Clear with stored color if needed
            let clear_color = Vec4::new(0.1, 0.1, 0.1, 1.0); // Default dark gray
            let raylib_color = vec4_to_raylib_color(clear_color);
            d.clear_background(raylib_color);
            
            // Execute all commands without borrowing self
            for command in &commands {

                Self::execute_single_command_impl(&mut d, &mut self.textures, &self.fonts, &mut self.text_manager, command)?;
            }
        }
        
        // Drawing handle is automatically dropped here, ending the frame
        Ok(())
    }
    
    fn render_element(
        &mut self,
        _context: &mut Self::Context,
        _element: &kryon_core::Element,
        _layout: &LayoutResult,
        _element_id: kryon_core::ElementId,
    ) -> RenderResult<()> {
        // This method is not used in command-based rendering
        Ok(())
    }
    
    fn resize(&mut self, new_size: Vec2) -> RenderResult<()> {
        self.size = new_size;
        // Raylib handles window resizing automatically
        Ok(())
    }
    
    fn viewport_size(&self) -> Vec2 {
        Vec2::new(self.handle.get_screen_width() as f32, self.handle.get_screen_height() as f32)
    }
}

impl CommandRenderer for RaylibRenderer {
    fn execute_commands(
        &mut self,
        _context: &mut Self::Context,
        commands: &[RenderCommand],
    ) -> RenderResult<()> {
        // Pre-load any textures we might need before adding to pending commands
        for command in commands {
            if let RenderCommand::DrawImage { source, .. } = command {
                // Try to load the texture (will cache it if successful)
                let _ = self.load_texture(source); // Ignore errors here, will handle in drawing
            }
        }
        
        // Store commands to be executed in end_frame
        self.pending_commands.extend_from_slice(commands);
        Ok(())
    }
    
    fn set_cursor(&mut self, cursor_type: CursorType) {
        self.set_cursor_internal(cursor_type);
    }
}

impl RaylibRenderer {
    pub fn should_close(&self) -> bool {
        self.handle.window_should_close()
    }
    
    pub fn take_screenshot(&mut self, filename: &str) -> RenderResult<()> {
        self.handle.take_screenshot(&self.thread, filename);
        Ok(())
    }
    
    pub fn get_handle(&self) -> &RaylibHandle {
        &self.handle
    }
    
    /// Load a texture from file and cache it for future use
    /// Tries multiple locations: current dir, relative to KRB file, etc.
    pub fn load_texture(&mut self, path: &str) -> RenderResult<()> {
        if !self.textures.contains_key(path) {
            let resolved_path = self.resolve_image_path(path);
            if let Some(actual_path) = resolved_path {
                match raylib::texture::Image::load_image(&actual_path) {
                    Ok(image) => {
                        let texture = self.handle.load_texture_from_image(&self.thread, &image)
                            .map_err(|e| RenderError::RenderFailed(format!("Failed to create texture: {}", e)))?;
                        self.textures.insert(path.to_string(), texture);
                        eprintln!("[RAYLIB] Loaded and cached texture: {} (found at: {})", path, actual_path);
                    }
                    Err(e) => {
                        return Err(RenderError::ResourceNotFound(format!("Failed to load image {}: {}", actual_path, e)));
                    }
                }
            } else {
                return Err(RenderError::ResourceNotFound(format!("Image file not found: {}", path)));
            }
        }
        Ok(())
    }
    
    /// Resolve image path by checking multiple locations
    fn resolve_image_path(&self, path: &str) -> Option<String> {
        resolve_image_path_static(path)
    }
    
    /// Register a font family with its file path
    pub fn register_font(&mut self, font_family: &str, font_path: &str) {
        self.font_paths.insert(font_family.to_string(), font_path.to_string());
        eprintln!("[RAYLIB_FONT] Registered font '{}' -> '{}'", font_family, font_path);
    }
    
    /// Load a font from file and cache it for future use
    pub fn load_font(&mut self, font_family: &str) -> RenderResult<()> {
        if !self.fonts.contains_key(font_family) {
            if let Some(font_path) = self.font_paths.get(font_family) {
                let resolved_path = resolve_font_path_static(font_path);
                if let Some(actual_path) = resolved_path {
                    match self.handle.load_font(&self.thread, &actual_path) {
                        Ok(font) => {
                            self.fonts.insert(font_family.to_string(), font);
                            eprintln!("[RAYLIB_FONT] Loaded and cached font '{}' from: {}", font_family, actual_path);
                        }
                        Err(e) => {
                            eprintln!("[RAYLIB_FONT] Failed to load font '{}' from {}: {}", font_family, actual_path, e);
                            return Err(RenderError::ResourceNotFound(format!("Failed to load font {}: {}", actual_path, e)));
                        }
                    }
                } else {
                    eprintln!("[RAYLIB_FONT] Font file not found: {}", font_path);
                    return Err(RenderError::ResourceNotFound(format!("Font file not found: {}", font_path)));
                }
            } else {
                eprintln!("[RAYLIB_FONT] Font family '{}' not registered", font_family);
                // Don't treat this as an error - just use default font
            }
        }
        Ok(())
    }
    
    /// Get a loaded font by family name, or None if not loaded/default
    #[allow(dead_code)]
    fn get_font(&self, font_family: &str) -> Option<&Font> {
        self.fonts.get(font_family)
    }
    
    /// Manually poll input events from the OS - this is what EndDrawing() normally does
    pub fn poll_input_events_from_os(&mut self) {
        unsafe {
            // Call the same function that raylib's EndDrawing() calls
            ffi::PollInputEvents();
        }
    }
    
    /// Set the mouse cursor type
    pub fn set_cursor_internal(&mut self, cursor_type: CursorType) {
        if self.current_cursor != cursor_type {
            match cursor_type {
                CursorType::Default => self.handle.set_mouse_cursor(MouseCursor::MOUSE_CURSOR_DEFAULT),
                CursorType::Pointer => self.handle.set_mouse_cursor(MouseCursor::MOUSE_CURSOR_POINTING_HAND),
                CursorType::Text => self.handle.set_mouse_cursor(MouseCursor::MOUSE_CURSOR_IBEAM),
                CursorType::Move => self.handle.set_mouse_cursor(MouseCursor::MOUSE_CURSOR_RESIZE_ALL),
                CursorType::NotAllowed => self.handle.set_mouse_cursor(MouseCursor::MOUSE_CURSOR_NOT_ALLOWED),
            }
            self.current_cursor = cursor_type;
        }
    }
    
    pub fn poll_input_events(&mut self) -> Vec<InputEvent> {
        // CRITICAL: Poll input events from OS FIRST before querying any input state
        self.poll_input_events_from_os();
        
        let mut events = Vec::new();
        
        
        // Handle window resize
        if self.handle.is_window_resized() {
            let new_size = Vec2::new(
                self.handle.get_screen_width() as f32,
                self.handle.get_screen_height() as f32
            );
            events.push(InputEvent::Resize { size: new_size });
        }
        
        // Handle mouse position - read fresh every frame
        let mouse_pos = Vec2::new(
            self.handle.get_mouse_x() as f32,
            self.handle.get_mouse_y() as f32
        );
        
        // Generate mouse move events if position changed OR if this is the first time reading mouse position
        let is_first_mouse_read = self.prev_mouse_pos.x < 0.0; // Initial position is (-1, -1)
        if mouse_pos != self.prev_mouse_pos || is_first_mouse_read {
            events.push(InputEvent::MouseMove { position: mouse_pos });
            self.prev_mouse_pos = mouse_pos;
        }
        
        // Mouse button events
        if self.handle.is_mouse_button_pressed(raylib::consts::MouseButton::MOUSE_BUTTON_LEFT) {
            events.push(InputEvent::MousePress {
                position: mouse_pos,
                button: MouseButton::Left,
            });
        }
        
        if self.handle.is_mouse_button_released(raylib::consts::MouseButton::MOUSE_BUTTON_LEFT) {
            events.push(InputEvent::MouseRelease {
                position: mouse_pos,
                button: MouseButton::Left,
            });
        }
        
        if self.handle.is_mouse_button_pressed(raylib::consts::MouseButton::MOUSE_BUTTON_RIGHT) {
            events.push(InputEvent::MousePress {
                position: mouse_pos,
                button: MouseButton::Right,
            });
        }
        
        if self.handle.is_mouse_button_released(raylib::consts::MouseButton::MOUSE_BUTTON_RIGHT) {
            events.push(InputEvent::MouseRelease {
                position: mouse_pos,
                button: MouseButton::Right,
            });
        }
        
        // Keyboard events - check ALL keys that might be pressed
        while let Some(key) = self.handle.get_key_pressed() {
            if let Some(kryon_key) = raylib_key_to_kryon_key(key) {
                events.push(InputEvent::KeyPress {
                    key: kryon_key,
                    modifiers: KeyModifiers {
                        shift: self.handle.is_key_down(KeyboardKey::KEY_LEFT_SHIFT) || self.handle.is_key_down(KeyboardKey::KEY_RIGHT_SHIFT),
                        ctrl: self.handle.is_key_down(KeyboardKey::KEY_LEFT_CONTROL) || self.handle.is_key_down(KeyboardKey::KEY_RIGHT_CONTROL),
                        alt: self.handle.is_key_down(KeyboardKey::KEY_LEFT_ALT) || self.handle.is_key_down(KeyboardKey::KEY_RIGHT_ALT),
                        meta: self.handle.is_key_down(KeyboardKey::KEY_LEFT_SUPER) || self.handle.is_key_down(KeyboardKey::KEY_RIGHT_SUPER),
                    },
                });
            }
        }
        
        events
    }
    
    fn execute_single_command_impl(
        d: &mut RaylibDrawHandle,
        textures: &mut HashMap<String, Texture2D>,
        fonts: &HashMap<String, Font>,
        text_manager: &mut TextManager,
        command: &RenderCommand,
    ) -> RenderResult<()> {
        match command {
            RenderCommand::DrawRect {
                position,
                size,
                color,
                border_radius: _,
                border_width,
                border_color,
                transform,
                shadow,
                z_index: _,
            } => {
                let rect = Rectangle::new(position.x, position.y, size.x, size.y);
                let raylib_color = vec4_to_raylib_color(*color);
                
                // Draw shadow if specified
                if let Some(shadow_str) = shadow {
                    if !shadow_str.is_empty() && shadow_str != "none" {
                        // Parse shadow string: "offset-x offset-y blur-radius spread-radius color"
                        // Example: "0 4px 6px rgba(0, 0, 0, 0.1)"
                        if let Some(shadow_values) = parse_box_shadow(shadow_str) {
                            let shadow_rect = Rectangle::new(
                                rect.x + shadow_values.offset_x,
                                rect.y + shadow_values.offset_y,
                                rect.width + shadow_values.spread_radius * 2.0,
                                rect.height + shadow_values.spread_radius * 2.0,
                            );
                            
                            // For now, draw a simple shadow without blur (Raylib limitation)
                            // In a real implementation, you'd render multiple offset rectangles with decreasing opacity for blur
                            let shadow_color = vec4_to_raylib_color(shadow_values.color);
                            d.draw_rectangle_rec(shadow_rect, shadow_color);
                        }
                    }
                }
                
                // Debug output for positioning investigation
                eprintln!("[RAYLIB_DRAW] DrawRect pos=({}, {}), size=({}, {}), color=({}, {}, {}, {})", 
                    position.x, position.y, size.x, size.y, color.x, color.y, color.z, color.w);
                
                // Apply transform if present
                if let Some(transform_data) = transform {
                    let (scale, rotation, translation) = extract_transform_values(transform_data);
                    
                    // Apply transformations manually (modern Raylib API)
                    let center_x = position.x + size.x / 2.0;
                    let center_y = position.y + size.y / 2.0;
                    
                    // Calculate transformed rectangle
                    let transformed_rect = Rectangle::new(
                        center_x - (size.x * scale.x) / 2.0 + translation.x,
                        center_y - (size.y * scale.y) / 2.0 + translation.y,
                        size.x * scale.x,
                        size.y * scale.y
                    );
                    
                    // Draw filled rectangle with transform
                    if color.w > 0.0 {
                        if rotation != 0.0 {
                            // Use draw_rectangle_pro for rotation
                            d.draw_rectangle_pro(
                                transformed_rect,
                                Vector2::new(transformed_rect.width / 2.0, transformed_rect.height / 2.0),
                                rotation.to_degrees(),
                                raylib_color
                            );
                        } else {
                            d.draw_rectangle_rec(transformed_rect, raylib_color);
                        }
                    }
                    
                    // Draw border
                    if *border_width > 0.0 {
                        let border_raylib_color = vec4_to_raylib_color(*border_color);
                        d.draw_rectangle_lines_ex(
                            transformed_rect, 
                            *border_width, 
                            border_raylib_color
                        );
                    }
                } else {
                    // Draw without transform (original behavior)
                    if color.w > 0.0 {
                        d.draw_rectangle_rec(rect, raylib_color);
                    }
                    
                    if *border_width > 0.0 {
                        let border_raylib_color = vec4_to_raylib_color(*border_color);
                        d.draw_rectangle_lines_ex(
                            rect, 
                            *border_width, 
                            border_raylib_color
                        );
                    }
                }
            }
            RenderCommand::DrawRichText {
                position,
                rich_text,
                max_width,
                max_height: _,
                default_color,
                alignment: _,
                transform,
                z_index: _,
            } => {
                // Use TextManager to render rich text with cosmic-text
                let rendered = text_manager.render_rich_text(
                    rich_text,
                    *max_width,
                    *default_color,
                );
                
                // Apply transform offset if present
                let base_offset = if let Some(transform_data) = transform {
                    let (_, _, translation) = extract_transform_values(transform_data);
                    Vec2::new(position.x + translation.x, position.y + translation.y)
                } else {
                    *position
                };
                
                // Render each positioned glyph
                for glyph in &rendered.glyphs {
                    let glyph_pos = base_offset + glyph.position;
                    let raylib_color = vec4_to_raylib_color(glyph.color);
                    
                    // For now, render each character as individual text
                    // TODO: Implement proper glyph texture atlas rendering
                    let char_str = glyph.character.to_string();
                    d.draw_text(
                        &char_str,
                        glyph_pos.x as i32,
                        glyph_pos.y as i32,
                        glyph.font_size as i32,
                        raylib_color,
                    );
                }
                
                eprintln!("[RAYLIB_RICH_TEXT] Rendered {} glyphs, bounds: {:?}", 
                    rendered.glyphs.len(), rendered.bounds);
            }
            RenderCommand::DrawText {
                position,
                text,
                font_size,
                color,
                alignment,
                max_width,
                max_height,
                transform,
                font_family,
                z_index: _,
            } => {
                let raylib_color = vec4_to_raylib_color(*color);
                
                // Determine which font to use
                let (text_width, custom_font) = if let Some(font_name) = font_family {
                    if let Some(font) = fonts.get(font_name) {
                        // Use custom font - calculate width using font's base size
                        let base_size = font.base_size() as f32;
                        let scale = *font_size / base_size;
                        let width = d.measure_text(text, font.base_size() as i32) as f32 * scale;
                        eprintln!("[RAYLIB_FONT] Using custom font '{}' for text '{}'", font_name, text);
                        (width, Some(font))
                    } else {
                        // Font not loaded or not found, use default
                        eprintln!("[RAYLIB_FONT] Font '{}' not loaded, using default", font_name);
                        (d.measure_text(text, *font_size as i32) as f32, None)
                    }
                } else {
                    // No custom font specified, use default
                    (d.measure_text(text, *font_size as i32) as f32, None)
                };
                let text_height = *font_size;
                
                let (text_x, text_y) = match alignment {
                    kryon_core::TextAlignment::Start => (position.x, position.y),
                    kryon_core::TextAlignment::Center => {
                        let container_width = max_width.unwrap_or(text_width);
                        let container_height = max_height.unwrap_or(text_height);
                        (
                            position.x + (container_width - text_width) / 2.0,
                            position.y + (container_height - text_height) / 2.0,
                        )
                    },
                    kryon_core::TextAlignment::End => {
                        let container_width = max_width.unwrap_or(text_width);
                        (position.x + container_width - text_width, position.y)
                    },
                    kryon_core::TextAlignment::Justify => {
                        // For justify, treat as start alignment for now (complex justification requires word spacing)
                        (position.x, position.y)
                    },
                };
                
                // Apply transform if present
                if let Some(transform_data) = transform {
                    let (scale, rotation, translation) = extract_transform_values(transform_data);
                    
                    // Apply transformations using raylib's transformation matrix
                    let center_x = text_x + text_width / 2.0;
                    let center_y = text_y + text_height / 2.0;
                    
                    // Apply transformations manually (modern Raylib API)
                    let transformed_x = center_x - (text_width * scale.x) / 2.0 + translation.x;
                    let transformed_y = center_y - (text_height * scale.y) / 2.0 + translation.y;
                    
                    // Draw text with transform
                    if rotation != 0.0 {
                        // For rotation, use draw_text_pro if available, otherwise fall back to basic draw_text
                        // Note: draw_text_pro may not be available in all Raylib versions
                        if let Some(font) = custom_font {
                            d.draw_text_pro(
                                font,
                                text,
                                Vector2::new(transformed_x, transformed_y),
                                Vector2::zero(),
                                0.0, // rotation
                                *font_size as f32 * scale.y,
                                1.0, // spacing
                                raylib_color,
                            );
                        } else {
                            d.draw_text(
                                text,
                                transformed_x as i32,
                                transformed_y as i32,
                                (*font_size as f32 * scale.y) as i32,
                                raylib_color,
                            );
                        }
                    } else {
                        if let Some(font) = custom_font {
                            d.draw_text_pro(
                                font,
                                text,
                                Vector2::new(transformed_x, transformed_y),
                                Vector2::zero(),
                                0.0, // rotation
                                *font_size as f32 * scale.y,
                                1.0, // spacing
                                raylib_color,
                            );
                        } else {
                            d.draw_text(
                                text,
                                transformed_x as i32,
                                transformed_y as i32,
                                (*font_size as f32 * scale.y) as i32,
                                raylib_color,
                            );
                        }
                    }
                } else {
                    // Draw without transform (original behavior)
                    if let Some(font) = custom_font {
                        d.draw_text_pro(
                            font,
                            text,
                            Vector2::new(text_x, text_y),
                            Vector2::zero(),
                            0.0, // rotation
                            *font_size,
                            1.0, // spacing
                            raylib_color,
                        );
                    } else {
                        d.draw_text(
                            text,
                            text_x as i32,
                            text_y as i32,
                            *font_size as i32,
                            raylib_color,
                        );
                    }
                }
            }
            RenderCommand::DrawImage {
                position,
                size,
                source,
                opacity,
                transform,
            } => {
                eprintln!("[RAYLIB] DrawImage match arm reached for: {}", source);
                
                // Check if we have a cached texture
                if let Some(texture) = textures.get(source) {
                    // Draw the actual texture
                    let dest_rect = Rectangle::new(position.x, position.y, size.x, size.y);
                    let source_rect = Rectangle::new(0.0, 0.0, texture.width as f32, texture.height as f32);
                    let tint = Color::new(255, 255, 255, (*opacity * 255.0) as u8);
                    
                    // Apply transform if present
                    if let Some(transform_data) = transform {
                        let (scale, rotation, translation) = extract_transform_values(transform_data);
                        
                        // Apply transformations manually (modern Raylib API)
                        let center_x = position.x + size.x / 2.0;
                        let center_y = position.y + size.y / 2.0;
                        
                        // Calculate transformed rectangle
                        let transformed_dest = Rectangle::new(
                            center_x - (size.x * scale.x) / 2.0 + translation.x,
                            center_y - (size.y * scale.y) / 2.0 + translation.y,
                            size.x * scale.x,
                            size.y * scale.y
                        );
                        
                        // Draw texture with transform
                        d.draw_texture_pro(
                            texture,
                            source_rect,
                            transformed_dest,
                            Vector2::new(transformed_dest.width / 2.0, transformed_dest.height / 2.0),
                            rotation.to_degrees(),
                            tint,
                        );
                    } else {
                        // Draw without transform (original behavior)
                        d.draw_texture_pro(
                            texture,
                            source_rect,
                            dest_rect,
                            Vector2::zero(),
                            0.0, // rotation
                            tint,
                        );
                    }
                    
                    eprintln!("[RAYLIB] Drew texture: {} at ({:.1},{:.1}) size ({:.1},{:.1})", 
                        source, position.x, position.y, size.x, size.y);
                } else {
                    // No cached texture - draw appropriate placeholder
                    let resolved_path = resolve_image_path_static(source);
                    if resolved_path.is_some() {
                        // File exists but failed to load or wasn't cached
                        let error_color = Color::new(150, 50, 50, (*opacity * 255.0) as u8);
                        d.draw_rectangle(
                            position.x as i32,
                            position.y as i32,
                            size.x as i32,
                            size.y as i32,
                            error_color,
                        );
                        
                        let text = "LOAD ERROR";
                        let filename = std::path::Path::new(source).file_name().unwrap_or_default().to_string_lossy();
                        
                        let font_size = 12;
                        let text_width = d.measure_text(text, font_size);
                        let file_width = d.measure_text(&filename, 10);
                        
                        let text_x = position.x + (size.x - text_width as f32) / 2.0;
                        let text_y = position.y + (size.y - font_size as f32 * 2.0) / 2.0;
                        let file_x = position.x + (size.x - file_width as f32) / 2.0;
                        let file_y = text_y + font_size as f32 + 2.0;
                        
                        d.draw_text(text, text_x as i32, text_y as i32, font_size, Color::WHITE);
                        d.draw_text(&filename, file_x as i32, file_y as i32, 10, Color::WHITE);
                        
                        eprintln!("[RAYLIB] Image file exists but texture not cached: {}", source);
                    } else {
                        // File doesn't exist
                        let notfound_color = Color::new(150, 150, 50, (*opacity * 255.0) as u8);
                        d.draw_rectangle(
                            position.x as i32,
                            position.y as i32,
                            size.x as i32,
                            size.y as i32,
                            notfound_color,
                        );
                        
                        let text = "NOT FOUND";
                        let filename = std::path::Path::new(source).file_name().unwrap_or_default().to_string_lossy();
                        
                        let font_size = 12;
                        let text_width = d.measure_text(text, font_size);
                        let file_width = d.measure_text(&filename, 10);
                        
                        let text_x = position.x + (size.x - text_width as f32) / 2.0;
                        let text_y = position.y + (size.y - font_size as f32 * 2.0) / 2.0;
                        let file_x = position.x + (size.x - file_width as f32) / 2.0;
                        let file_y = text_y + font_size as f32 + 2.0;
                        
                        d.draw_text(text, text_x as i32, text_y as i32, font_size, Color::WHITE);
                        d.draw_text(&filename, file_x as i32, file_y as i32, 10, Color::WHITE);
                        
                        eprintln!("[RAYLIB] Image file not found: {}", source);
                    }
                }
            }
            RenderCommand::SetClip { position, size } => {
                let _scissor = d.begin_scissor_mode(
                    position.x as i32,
                    position.y as i32,
                    size.x as i32,
                    size.y as i32,
                );
            }
            RenderCommand::ClearClip => {
                // Raylib handles scissor mode differently - it's scoped to the draw handle
                // This is a no-op since scissor mode will end when the draw context ends
            },
            RenderCommand::DrawTextInput {
                position,
                size,
                text,
                placeholder,
                font_size,
                text_color,
                background_color,
                border_color,
                border_width,
                border_radius: _,
                is_focused,
                is_readonly: _,
                transform: _,
            } => {
                // Draw background
                let rect = Rectangle::new(position.x, position.y, size.x, size.y);
                let bg_color = vec4_to_raylib_color(*background_color);
                d.draw_rectangle_rec(rect, bg_color);
                
                // Draw border
                if *border_width > 0.0 {
                    let border_raylib_color = vec4_to_raylib_color(*border_color);
                    d.draw_rectangle_lines_ex(rect, *border_width, border_raylib_color);
                }
                
                // Draw focus indicator if focused
                if *is_focused {
                    let focus_color = Color::BLUE;
                    d.draw_rectangle_lines_ex(rect, 2.0, focus_color);
                }
                
                // Draw text or placeholder
                let display_text = if text.is_empty() && !placeholder.is_empty() {
                    placeholder
                } else {
                    text
                };
                
                if !display_text.is_empty() {
                    let text_raylib_color = vec4_to_raylib_color(*text_color);
                    let text_x = position.x + 5.0; // Small padding
                    let text_y = position.y + (size.y - *font_size) / 2.0; // Vertically center
                    
                    d.draw_text(display_text, text_x as i32, text_y as i32, *font_size as i32, text_raylib_color);
                }
            },
            RenderCommand::DrawCheckbox {
                position,
                size,
                is_checked,
                text,
                font_size,
                text_color,
                background_color,
                border_color,
                border_width,
                check_color,
                transform: _,
            } => {
                // Draw checkbox square
                let checkbox_size = size.y.min(20.0); // Max 20px checkbox
                let checkbox_rect = Rectangle::new(position.x, position.y, checkbox_size, checkbox_size);
                
                // Draw background
                let bg_color = vec4_to_raylib_color(*background_color);
                d.draw_rectangle_rec(checkbox_rect, bg_color);
                
                // Draw border
                if *border_width > 0.0 {
                    let border_raylib_color = vec4_to_raylib_color(*border_color);
                    d.draw_rectangle_lines_ex(checkbox_rect, *border_width, border_raylib_color);
                }
                
                // Draw checkmark if checked
                if *is_checked {
                    let check_raylib_color = vec4_to_raylib_color(*check_color);
                    let check_x = position.x + checkbox_size * 0.2;
                    let check_y = position.y + checkbox_size * 0.5;
                    let check_end_x = position.x + checkbox_size * 0.8;
                    let check_end_y = position.y + checkbox_size * 0.8;
                    
                    // Simple checkmark (two lines)
                    d.draw_line_ex(
                        Vector2::new(check_x, check_y),
                        Vector2::new(check_x + checkbox_size * 0.2, check_end_y - checkbox_size * 0.1),
                        2.0,
                        check_raylib_color
                    );
                    d.draw_line_ex(
                        Vector2::new(check_x + checkbox_size * 0.2, check_end_y - checkbox_size * 0.1),
                        Vector2::new(check_end_x, position.y + checkbox_size * 0.2),
                        2.0,
                        check_raylib_color
                    );
                }
                
                // Draw text label
                if !text.is_empty() {
                    let text_raylib_color = vec4_to_raylib_color(*text_color);
                    let text_x = position.x + checkbox_size + 5.0; // Small gap after checkbox
                    let text_y = position.y + (checkbox_size - *font_size) / 2.0; // Vertically center with checkbox
                    
                    d.draw_text(text, text_x as i32, text_y as i32, *font_size as i32, text_raylib_color);
                }
            },
            RenderCommand::DrawSlider {
                position,
                size,
                value,
                min_value,
                max_value,
                track_color,
                thumb_color,
                border_color,
                border_width,
                transform: _,
            } => {
                // Draw track background
                let track_rect = Rectangle::new(position.x, position.y + size.y * 0.4, size.x, size.y * 0.2);
                let track_raylib_color = vec4_to_raylib_color(*track_color);
                d.draw_rectangle_rec(track_rect, track_raylib_color);
                
                // Draw track border
                if *border_width > 0.0 {
                    let border_raylib_color = vec4_to_raylib_color(*border_color);
                    d.draw_rectangle_lines_ex(track_rect, *border_width, border_raylib_color);
                }
                
                // Calculate thumb position
                let value_ratio = (value - min_value) / (max_value - min_value);
                let thumb_x = position.x + (size.x - 10.0) * value_ratio; // 10px thumb width
                let thumb_y = position.y;
                let thumb_size = 10.0;
                
                // Draw thumb
                let thumb_rect = Rectangle::new(thumb_x, thumb_y, thumb_size, size.y);
                let thumb_raylib_color = vec4_to_raylib_color(*thumb_color);
                d.draw_rectangle_rec(thumb_rect, thumb_raylib_color);
                
                // Draw thumb border
                if *border_width > 0.0 {
                    let border_raylib_color = vec4_to_raylib_color(*border_color);
                    d.draw_rectangle_lines_ex(thumb_rect, *border_width, border_raylib_color);
                }
            },
            RenderCommand::DrawScrollbar {
                position,
                size,
                orientation,
                scroll_position,
                content_size,
                viewport_size,
                track_color,
                thumb_color,
                border_color,
                border_width,
                z_index: _,
            } => {
                use kryon_render::ScrollbarOrientation;
                
                // Draw track background
                let track_rect = Rectangle::new(position.x, position.y, size.x, size.y);
                let track_raylib_color = vec4_to_raylib_color(*track_color);
                d.draw_rectangle_rec(track_rect, track_raylib_color);
                
                // Draw track border
                if *border_width > 0.0 {
                    let border_raylib_color = vec4_to_raylib_color(*border_color);
                    d.draw_rectangle_lines_ex(track_rect, *border_width, border_raylib_color);
                }
                
                // Calculate thumb size and position
                let thumb_ratio = viewport_size / content_size;
                let scroll_ratio = scroll_position / (content_size - viewport_size).max(1.0);
                
                let (thumb_rect, thumb_size) = match orientation {
                    ScrollbarOrientation::Vertical => {
                        let thumb_height = (size.y * thumb_ratio).max(20.0); // Minimum thumb height
                        let thumb_y = position.y + (size.y - thumb_height) * scroll_ratio;
                        (Rectangle::new(position.x, thumb_y, size.x, thumb_height), thumb_height)
                    }
                    ScrollbarOrientation::Horizontal => {
                        let thumb_width = (size.x * thumb_ratio).max(20.0); // Minimum thumb width
                        let thumb_x = position.x + (size.x - thumb_width) * scroll_ratio;
                        (Rectangle::new(thumb_x, position.y, thumb_width, size.y), thumb_width)
                    }
                };
                
                // Draw thumb
                let thumb_raylib_color = vec4_to_raylib_color(*thumb_color);
                d.draw_rectangle_rec(thumb_rect, thumb_raylib_color);
                
                // Draw thumb border
                if *border_width > 0.0 {
                    let border_raylib_color = vec4_to_raylib_color(*border_color);
                    d.draw_rectangle_lines_ex(thumb_rect, *border_width, border_raylib_color);
                }
            },
            RenderCommand::SetCanvasSize(_) => {},
            // Canvas rendering commands
            RenderCommand::BeginCanvas { canvas_id: _, position: _, size: _ } => {
                // For Raylib, canvas rendering is just direct drawing
                // BeginCanvas/EndCanvas are markers for organization
            },
            RenderCommand::EndCanvas => {
                // Nothing special needed for Raylib
            },
            RenderCommand::DrawCanvasRect { position, size, fill_color, stroke_color, stroke_width } => {
                let rect = Rectangle::new(position.x, position.y, size.x, size.y);
                
                // Draw fill if specified
                if let Some(fill) = fill_color {
                    let fill_raylib_color = vec4_to_raylib_color(*fill);
                    d.draw_rectangle_rec(rect, fill_raylib_color);
                }
                
                // Draw stroke if specified
                if let Some(stroke) = stroke_color {
                    let stroke_raylib_color = vec4_to_raylib_color(*stroke);
                    d.draw_rectangle_lines_ex(rect, *stroke_width, stroke_raylib_color);
                }
            },
            RenderCommand::DrawCanvasCircle { center, radius, fill_color, stroke_color, stroke_width } => {
                let center_vec = Vector2::new(center.x, center.y);
                
                // Draw fill if specified
                if let Some(fill) = fill_color {
                    let fill_raylib_color = vec4_to_raylib_color(*fill);
                    d.draw_circle_v(center_vec, *radius, fill_raylib_color);
                }
                
                // Draw stroke if specified
                if let Some(stroke) = stroke_color {
                    let stroke_raylib_color = vec4_to_raylib_color(*stroke);
                    d.draw_circle_lines(center.x as i32, center.y as i32, *radius, stroke_raylib_color);
                    
                    // Draw additional circles for stroke width if needed
                    if *stroke_width > 1.0 {
                        for i in 1..(stroke_width.ceil() as i32) {
                            d.draw_circle_lines(center.x as i32, center.y as i32, radius + i as f32, stroke_raylib_color);
                        }
                    }
                }
            },
            RenderCommand::DrawCanvasLine { start, end, color, width } => {
                let start_vec = Vector2::new(start.x, start.y);
                let end_vec = Vector2::new(end.x, end.y);
                let raylib_color = vec4_to_raylib_color(*color);
                
                if *width <= 1.0 {
                    d.draw_line_v(start_vec, end_vec, raylib_color);
                } else {
                    d.draw_line_ex(start_vec, end_vec, *width, raylib_color);
                }
            },
            RenderCommand::DrawCanvasText { position, text, font_size, color, font_family, alignment } => {
                let raylib_color = vec4_to_raylib_color(*color);
                
                // Determine which font to use
                let (text_width, custom_font) = if let Some(font_name) = font_family {
                    if let Some(font) = fonts.get(font_name) {
                        let base_size = font.base_size() as f32;
                        let scale = *font_size / base_size;
                        let width = d.measure_text(text, font.base_size() as i32) as f32 * scale;
                        (width, Some(font))
                    } else {
                        (d.measure_text(text, *font_size as i32) as f32, None)
                    }
                } else {
                    (d.measure_text(text, *font_size as i32) as f32, None)
                };
                
                // Calculate position based on alignment
                let text_x = match alignment {
                    kryon_core::TextAlignment::Start => position.x,
                    kryon_core::TextAlignment::Center => position.x - text_width / 2.0,
                    kryon_core::TextAlignment::End => position.x - text_width,
                    kryon_core::TextAlignment::Justify => position.x, // Treat as start for now
                };
                
                // Draw text with appropriate font
                if let Some(font) = custom_font {
                    d.draw_text_pro(
                        font,
                        text,
                        Vector2::new(text_x, position.y),
                        Vector2::zero(),
                        0.0, // rotation
                        *font_size,
                        1.0, // spacing
                        raylib_color,
                    );
                } else {
                    d.draw_text(text, text_x as i32, position.y as i32, *font_size as i32, raylib_color);
                }
            },
            RenderCommand::DrawCanvasEllipse { center, rx, ry, fill_color, stroke_color, stroke_width } => {
                // Draw fill if specified
                if let Some(fill) = fill_color {
                    let fill_raylib_color = vec4_to_raylib_color(*fill);
                    // Raylib doesn't have a direct ellipse function, so approximate with a polygon
                    let num_segments = 32;
                    let mut points = Vec::new();
                    for i in 0..num_segments {
                        let angle = (i as f32 / num_segments as f32) * 2.0 * std::f32::consts::PI;
                        let x = center.x + rx * angle.cos();
                        let y = center.y + ry * angle.sin();
                        points.push(Vector2::new(x, y));
                    }
                    
                    // Draw filled polygon (simplified approximation)
                    for i in 1..(points.len() - 1) {
                        d.draw_triangle(points[0], points[i], points[i + 1], fill_raylib_color);
                    }
                }
                
                // Draw stroke if specified
                if let Some(stroke) = stroke_color {
                    let stroke_raylib_color = vec4_to_raylib_color(*stroke);
                    let num_segments = 32;
                    for i in 0..num_segments {
                        let angle1 = (i as f32 / num_segments as f32) * 2.0 * std::f32::consts::PI;
                        let angle2 = ((i + 1) as f32 / num_segments as f32) * 2.0 * std::f32::consts::PI;
                        let x1 = center.x + rx * angle1.cos();
                        let y1 = center.y + ry * angle1.sin();
                        let x2 = center.x + rx * angle2.cos();
                        let y2 = center.y + ry * angle2.sin();
                        
                        d.draw_line_ex(
                            Vector2::new(x1, y1),
                            Vector2::new(x2, y2),
                            *stroke_width,
                            stroke_raylib_color
                        );
                    }
                }
            },
            RenderCommand::DrawCanvasPolygon { points, fill_color, stroke_color, stroke_width } => {
                if points.len() < 3 {
                    return Ok(()); // Need at least 3 points for a polygon
                }
                
                let raylib_points: Vec<Vector2> = points.iter()
                    .map(|p| Vector2::new(p.x, p.y))
                    .collect();
                
                // Draw fill if specified
                if let Some(fill) = fill_color {
                    let fill_raylib_color = vec4_to_raylib_color(*fill);
                    // Triangulate the polygon for filling (simple fan triangulation from first vertex)
                    for i in 1..(raylib_points.len() - 1) {
                        d.draw_triangle(raylib_points[0], raylib_points[i], raylib_points[i + 1], fill_raylib_color);
                    }
                }
                
                // Draw stroke if specified
                if let Some(stroke) = stroke_color {
                    let stroke_raylib_color = vec4_to_raylib_color(*stroke);
                    for i in 0..raylib_points.len() {
                        let next_i = (i + 1) % raylib_points.len();
                        d.draw_line_ex(raylib_points[i], raylib_points[next_i], *stroke_width, stroke_raylib_color);
                    }
                }
            },
            RenderCommand::DrawCanvasPath { path_data, fill_color, stroke_color, stroke_width } => {
                // SVG path parsing is complex - for now, just draw a placeholder
                eprintln!("[RAYLIB] DrawCanvasPath not fully implemented, path_data: {}", path_data);
                
                // Draw a simple placeholder rectangle to indicate path rendering
                if let Some(fill) = fill_color {
                    let fill_raylib_color = vec4_to_raylib_color(*fill);
                    d.draw_rectangle(10, 10, 50, 20, fill_raylib_color);
                }
                
                if let Some(stroke) = stroke_color {
                    let stroke_raylib_color = vec4_to_raylib_color(*stroke);
                    d.draw_rectangle_lines_ex(Rectangle::new(10.0, 10.0, 50.0, 20.0), *stroke_width, stroke_raylib_color);
                }
                
                // TODO: Implement SVG path parsing and rendering
                d.draw_text("SVG Path", 15, 15, 10, Color::WHITE);
            },
            RenderCommand::DrawCanvasImage { source, position, size, opacity } => {
                // Similar to regular DrawImage but for canvas context
                if let Some(texture) = textures.get(source) {
                    let dest_rect = Rectangle::new(position.x, position.y, size.x, size.y);
                    let source_rect = Rectangle::new(0.0, 0.0, texture.width as f32, texture.height as f32);
                    let tint = Color::new(255, 255, 255, (*opacity * 255.0) as u8);
                    
                    d.draw_texture_pro(
                        texture,
                        source_rect,
                        dest_rect,
                        Vector2::zero(),
                        0.0, // rotation
                        tint,
                    );
                } else {
                    // Draw placeholder for missing image
                    let placeholder_color = Color::new(100, 100, 100, (*opacity * 255.0) as u8);
                    d.draw_rectangle(position.x as i32, position.y as i32, size.x as i32, size.y as i32, placeholder_color);
                    d.draw_text("IMG", (position.x + 5.0) as i32, (position.y + 5.0) as i32, 12, Color::WHITE);
                }
            }
            // WASM View rendering commands
            RenderCommand::BeginWasmView { wasm_id: _, position: _, size: _ } => {
                // For Raylib, WASM view rendering is just direct drawing
                // BeginWasmView/EndWasmView are markers for organization
            }
            RenderCommand::EndWasmView => {
                // Nothing special needed for Raylib
            }
            RenderCommand::ExecuteWasmFunction { function_name: _, params: _ } => {
                // WASM function execution would be handled by a separate WASM runtime
                // This command is just a marker for the rendering pipeline
            }
            RenderCommand::NativeRendererView { position, size, backend, script_name, element_id, config: _, z_index: _ } => {
                // Handle NativeRendererView rendering for Raylib backend
                if backend == "raylib" {
                    // TODO: Execute the native render script here
                    // This would need to be coordinated with the script system
                    eprintln!("[NATIVE_RENDERER] Raylib NativeRendererView '{}' should execute script: '{}'", element_id, script_name);
                    
                    // Draw a border to show the native view bounds
                    let border_color = Color::new(100, 100, 100, 255);
                    d.draw_rectangle_lines(
                        position.x as i32,
                        position.y as i32,
                        size.x as i32,
                        size.y as i32,
                        border_color,
                    );
                    
                    // For now, draw a placeholder to show NativeRendererView is working
                    d.draw_rectangle(
                        (position.x + 2.0) as i32,
                        (position.y + 2.0) as i32,
                        (size.x - 4.0) as i32,
                        (size.y - 4.0) as i32,
                        Color::new(50, 50, 150, 100), // Semi-transparent blue
                    );
                    
                    d.draw_text(
                        &format!("Native Raylib View\nScript: {}", script_name),
                        (position.x + 10.0) as i32,
                        (position.y + 10.0) as i32,
                        16,
                        Color::WHITE,
                    );
                } else {
                    // Draw a placeholder for non-Raylib backends
                    d.draw_rectangle(
                        position.x as i32,
                        position.y as i32,
                        size.x as i32,
                        size.y as i32,
                        Color::new(100, 100, 100, 100), // Gray placeholder
                    );
                    
                    d.draw_text(
                        &format!("Unsupported backend: {}", backend),
                        (position.x + 10.0) as i32,
                        (position.y + 10.0) as i32,
                        16,
                        Color::WHITE,
                    );
                }
            }
        }
        Ok(())
    }
}

fn vec4_to_raylib_color(color: Vec4) -> Color {
    let r = (color.x * 255.0) as u8;
    let g = (color.y * 255.0) as u8;
    let b = (color.z * 255.0) as u8;
    let a = (color.w * 255.0) as u8;

    Color::new(r, g, b, a)
}

/// Extract transform values from TransformData
fn extract_transform_values(transform: &TransformData) -> (Vec2, f32, Vec2) {
    let mut scale = Vec2::new(1.0, 1.0);
    let mut rotation = 0.0f32;
    let mut translation = Vec2::new(0.0, 0.0);
    
    for property in &transform.properties {
        match property.property_type {
            TransformPropertyType::Scale => {
                let value = css_unit_to_pixels(&property.value);
                scale = Vec2::new(value, value);
            }
            TransformPropertyType::ScaleX => {
                scale.x = css_unit_to_pixels(&property.value);
            }
            TransformPropertyType::ScaleY => {
                scale.y = css_unit_to_pixels(&property.value);
            }
            TransformPropertyType::TranslateX => {
                translation.x = css_unit_to_pixels(&property.value);
            }
            TransformPropertyType::TranslateY => {
                translation.y = css_unit_to_pixels(&property.value);
            }
            TransformPropertyType::Rotate => {
                rotation = css_unit_to_radians(&property.value);
            }
            // Add other transform properties as needed
            _ => {
                eprintln!("[RAYLIB_TRANSFORM] Unsupported transform property: {:?}", property.property_type);
            }
        }
    }
    
    (scale, rotation, translation)
}

/// Convert CSS unit value to pixels (simplified)
fn css_unit_to_pixels(unit_value: &kryon_core::CSSUnitValue) -> f32 {
    match unit_value.unit {
        CSSUnit::Pixels => unit_value.value as f32,
        CSSUnit::Number => unit_value.value as f32,
        CSSUnit::Em => unit_value.value as f32 * 16.0, // Assume 16px base
        CSSUnit::Rem => unit_value.value as f32 * 16.0, // Assume 16px base
        CSSUnit::Percentage => unit_value.value as f32 / 100.0,
        _ => {
            eprintln!("[RAYLIB_TRANSFORM] Unsupported CSS unit for size: {:?}", unit_value.unit);
            unit_value.value as f32
        }
    }
}

/// Convert CSS unit value to radians for rotation
fn css_unit_to_radians(unit_value: &kryon_core::CSSUnitValue) -> f32 {
    match unit_value.unit {
        CSSUnit::Degrees => unit_value.value as f32 * std::f32::consts::PI / 180.0,
        CSSUnit::Radians => unit_value.value as f32,
        CSSUnit::Turns => unit_value.value as f32 * 2.0 * std::f32::consts::PI,
        _ => {
            eprintln!("[RAYLIB_TRANSFORM] Unsupported CSS unit for rotation: {:?}", unit_value.unit);
            unit_value.value as f32
        }
    }
}



fn raylib_key_to_kryon_key(key: KeyboardKey) -> Option<KeyCode> {
    match key {
        KeyboardKey::KEY_SPACE => Some(KeyCode::Space),
        KeyboardKey::KEY_ESCAPE => Some(KeyCode::Escape),
        KeyboardKey::KEY_ENTER => Some(KeyCode::Enter),
        KeyboardKey::KEY_TAB => Some(KeyCode::Tab),
        KeyboardKey::KEY_BACKSPACE => Some(KeyCode::Backspace),
        KeyboardKey::KEY_DELETE => Some(KeyCode::Delete),
        
        // Convert letters to characters
        KeyboardKey::KEY_A => Some(KeyCode::Character('a')),
        KeyboardKey::KEY_B => Some(KeyCode::Character('b')),
        KeyboardKey::KEY_C => Some(KeyCode::Character('c')),
        KeyboardKey::KEY_D => Some(KeyCode::Character('d')),
        KeyboardKey::KEY_E => Some(KeyCode::Character('e')),
        KeyboardKey::KEY_F => Some(KeyCode::Character('f')),
        KeyboardKey::KEY_G => Some(KeyCode::Character('g')),
        KeyboardKey::KEY_H => Some(KeyCode::Character('h')),
        KeyboardKey::KEY_I => Some(KeyCode::Character('i')),
        KeyboardKey::KEY_J => Some(KeyCode::Character('j')),
        KeyboardKey::KEY_K => Some(KeyCode::Character('k')),
        KeyboardKey::KEY_L => Some(KeyCode::Character('l')),
        KeyboardKey::KEY_M => Some(KeyCode::Character('m')),
        KeyboardKey::KEY_N => Some(KeyCode::Character('n')),
        KeyboardKey::KEY_O => Some(KeyCode::Character('o')),
        KeyboardKey::KEY_P => Some(KeyCode::Character('p')),
        KeyboardKey::KEY_Q => Some(KeyCode::Character('q')),
        KeyboardKey::KEY_R => Some(KeyCode::Character('r')),
        KeyboardKey::KEY_S => Some(KeyCode::Character('s')),
        KeyboardKey::KEY_T => Some(KeyCode::Character('t')),
        KeyboardKey::KEY_U => Some(KeyCode::Character('u')),
        KeyboardKey::KEY_V => Some(KeyCode::Character('v')),
        KeyboardKey::KEY_W => Some(KeyCode::Character('w')),
        KeyboardKey::KEY_X => Some(KeyCode::Character('x')),
        KeyboardKey::KEY_Y => Some(KeyCode::Character('y')),
        KeyboardKey::KEY_Z => Some(KeyCode::Character('z')),
        
        // Convert numbers to characters
        KeyboardKey::KEY_ZERO => Some(KeyCode::Character('0')),
        KeyboardKey::KEY_ONE => Some(KeyCode::Character('1')),
        KeyboardKey::KEY_TWO => Some(KeyCode::Character('2')),
        KeyboardKey::KEY_THREE => Some(KeyCode::Character('3')),
        KeyboardKey::KEY_FOUR => Some(KeyCode::Character('4')),
        KeyboardKey::KEY_FIVE => Some(KeyCode::Character('5')),
        KeyboardKey::KEY_SIX => Some(KeyCode::Character('6')),
        KeyboardKey::KEY_SEVEN => Some(KeyCode::Character('7')),
        KeyboardKey::KEY_EIGHT => Some(KeyCode::Character('8')),
        KeyboardKey::KEY_NINE => Some(KeyCode::Character('9')),
        
        // Convert symbols to characters
        KeyboardKey::KEY_APOSTROPHE => Some(KeyCode::Character('\'')),
        KeyboardKey::KEY_COMMA => Some(KeyCode::Character(',')),
        KeyboardKey::KEY_MINUS => Some(KeyCode::Character('-')),
        KeyboardKey::KEY_PERIOD => Some(KeyCode::Character('.')),
        KeyboardKey::KEY_SLASH => Some(KeyCode::Character('/')),
        KeyboardKey::KEY_SEMICOLON => Some(KeyCode::Character(';')),
        KeyboardKey::KEY_EQUAL => Some(KeyCode::Character('=')),
        KeyboardKey::KEY_LEFT_BRACKET => Some(KeyCode::Character('[')),
        KeyboardKey::KEY_BACKSLASH => Some(KeyCode::Character('\\')),
        KeyboardKey::KEY_RIGHT_BRACKET => Some(KeyCode::Character(']')),
        KeyboardKey::KEY_GRAVE => Some(KeyCode::Character('`')),
        
        _ => None,
    }
}

/// Resolve font path by checking multiple locations  
fn resolve_font_path_static(path: &str) -> Option<String> {
    // Try the path as-is first (relative to current working directory)
    if std::path::Path::new(path).exists() {
        eprintln!("[RAYLIB_FONT] Found font at current path: {}", path);
        return Some(path.to_string());
    }
    
    // Get the KRB file path from command line args if available
    let args: Vec<String> = std::env::args().collect();
    if let Some(krb_arg) = args.iter().find(|arg| arg.ends_with(".krb")) {
        if let Some(krb_dir) = std::path::Path::new(krb_arg).parent() {
            let krb_relative_path = krb_dir.join(path);
            if krb_relative_path.exists() {
                let resolved = krb_relative_path.to_string_lossy().to_string();
                eprintln!("[RAYLIB_FONT] Found font relative to KRB: {}", resolved);
                return Some(resolved);
            }
        }
    }
    
    // Try some common relative paths for fonts
    let common_paths = [
        format!("assets/fonts/{}", path),
        format!("fonts/{}", path),
        format!("resources/fonts/{}", path),
        format!("assets/{}", path),
    ];
    
    for test_path in &common_paths {
        if std::path::Path::new(test_path).exists() {
            eprintln!("[RAYLIB_FONT] Found font at common path: {}", test_path);
            return Some(test_path.clone());
        }
    }
    
    eprintln!("[RAYLIB_FONT] Font not found in any location: {}", path);
    None
}

/// Resolve image path by checking multiple locations
fn resolve_image_path_static(path: &str) -> Option<String> {
    // Try the path as-is first (relative to current working directory)
    if std::path::Path::new(path).exists() {
        eprintln!("[RAYLIB] Found image at current path: {}", path);
        return Some(path.to_string());
    }
    
    // Get the KRB file path from command line args if available
    let args: Vec<String> = std::env::args().collect();
    if let Some(krb_arg) = args.iter().find(|arg| arg.ends_with(".krb")) {
        if let Some(krb_dir) = std::path::Path::new(krb_arg).parent() {
            let krb_relative_path = krb_dir.join(path);
            if krb_relative_path.exists() {
                let resolved = krb_relative_path.to_string_lossy().to_string();
                eprintln!("[RAYLIB] Found image relative to KRB: {}", resolved);
                return Some(resolved);
            }
        }
    }
    
    // Try some common relative paths
    let common_paths = [
        format!("assets/{}", path),
        format!("images/{}", path),
        format!("resources/{}", path),
    ];
    
    for test_path in &common_paths {
        if std::path::Path::new(test_path).exists() {
            eprintln!("[RAYLIB] Found image at common path: {}", test_path);
            return Some(test_path.clone());
        }
    }
    
    eprintln!("[RAYLIB] Image not found in any location: {}", path);
    None
}

#[derive(Debug)]
struct BoxShadowValues {
    offset_x: f32,
    offset_y: f32,
    blur_radius: f32,
    spread_radius: f32,
    color: Vec4,
}

/// Parse CSS box-shadow string
/// Format: "offset-x offset-y blur-radius spread-radius color"
/// Example: "0 4px 6px rgba(0, 0, 0, 0.1)" or "2px 2px 4px #00000040"
fn parse_box_shadow(shadow_str: &str) -> Option<BoxShadowValues> {
    let parts: Vec<&str> = shadow_str.trim().split_whitespace().collect();
    
    if parts.len() < 3 {
        return None; // Need at least offset-x, offset-y, and color
    }
    
    // Parse offset-x and offset-y (required)
    let offset_x = parse_pixel_value(parts.get(0)?);
    let offset_y = parse_pixel_value(parts.get(1)?);
    
    // Parse optional blur-radius and spread-radius
    let mut blur_radius = 0.0;
    let mut spread_radius = 0.0;
    let mut color_index = 2;
    
    // Check if we have more numeric values before the color
    if parts.len() > 3 {
        // Try to parse as numeric value
        if let Ok(_) = parts[2].trim_end_matches("px").parse::<f32>() {
            blur_radius = parse_pixel_value(parts[2]);
            color_index = 3;
            
            if parts.len() > 4 {
                if let Ok(_) = parts[3].trim_end_matches("px").parse::<f32>() {
                    spread_radius = parse_pixel_value(parts[3]);
                    color_index = 4;
                }
            }
        }
    }
    
    // Parse color (could be rgba() function or hex)
    let color_str = if color_index < parts.len() {
        if parts[color_index].starts_with("rgba(") {
            // Reconstruct rgba() function from remaining parts
            parts[color_index..].join(" ")
        } else {
            parts[color_index].to_string()
        }
    } else {
        "#00000040".to_string() // Default shadow color
    };
    
    let color = parse_color_string(&color_str)?;
    
    Some(BoxShadowValues {
        offset_x,
        offset_y,
        blur_radius,
        spread_radius,
        color,
    })
}

/// Simple pixel value parser for shadow values
fn parse_pixel_value(value: &str) -> f32 {
    if value.ends_with("px") {
        value.trim_end_matches("px").parse().unwrap_or(0.0)
    } else {
        value.parse().unwrap_or(0.0)
    }
}

/// Parse color string (hex or rgba)
fn parse_color_string(color_str: &str) -> Option<Vec4> {
    let color_str = color_str.trim();
    
    if color_str.starts_with("rgba(") && color_str.ends_with(")") {
        // Parse rgba(r, g, b, a)
        let inner = &color_str[5..color_str.len()-1];
        let parts: Vec<&str> = inner.split(',').collect();
        if parts.len() == 4 {
            let r = parts[0].trim().parse::<f32>().ok()? / 255.0;
            let g = parts[1].trim().parse::<f32>().ok()? / 255.0;
            let b = parts[2].trim().parse::<f32>().ok()? / 255.0;
            let a = parts[3].trim().parse::<f32>().ok()?;
            return Some(Vec4::new(r, g, b, a));
        }
    } else if color_str.starts_with("#") {
        // Parse hex color
        let hex = &color_str[1..];
        if hex.len() == 6 {
            // #RRGGBB
            let r = u8::from_str_radix(&hex[0..2], 16).ok()? as f32 / 255.0;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()? as f32 / 255.0;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()? as f32 / 255.0;
            return Some(Vec4::new(r, g, b, 1.0));
        } else if hex.len() == 8 {
            // #RRGGBBAA
            let r = u8::from_str_radix(&hex[0..2], 16).ok()? as f32 / 255.0;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()? as f32 / 255.0;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()? as f32 / 255.0;
            let a = u8::from_str_radix(&hex[6..8], 16).ok()? as f32 / 255.0;
            return Some(Vec4::new(r, g, b, a));
        }
    }
    
    None
}

impl Drop for RaylibRenderer {
    fn drop(&mut self) {
        // Clean up all loaded textures before raylib context is destroyed
        eprintln!("[RAYLIB] Cleaning up {} textures", self.textures.len());
        self.textures.clear();
        
        // Clean up all loaded fonts before raylib context is destroyed
        eprintln!("[RAYLIB] Cleaning up {} fonts", self.fonts.len());
        self.fonts.clear();
        
        eprintln!("[RAYLIB] Resource cleanup complete");
    }
}
// crates/kryon-raylib/src/lib.rs
use kryon_render::{
    Renderer, CommandRenderer, RenderCommand, RenderResult, InputEvent, MouseButton, KeyCode, KeyModifiers
};
use kryon_core::CursorType;
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
        
        {
            let mut d = self.handle.begin_drawing(&self.thread);
            
            // Clear with stored color if needed
            let clear_color = Vec4::new(0.1, 0.1, 0.1, 1.0); // Default dark gray
            let raylib_color = vec4_to_raylib_color(clear_color);
            d.clear_background(raylib_color);
            
            // Execute all commands without borrowing self
            for command in &commands {

                Self::execute_single_command_impl(&mut d, &mut self.textures, command)?;
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
            } => {
                let rect = Rectangle::new(position.x, position.y, size.x, size.y);
                let raylib_color = vec4_to_raylib_color(*color);
                
                // Draw filled rectangle
                if color.w > 0.0 {
                    d.draw_rectangle_rec(rect, raylib_color);
                }
                
                // Draw border
                if *border_width > 0.0 {
                    let border_raylib_color = vec4_to_raylib_color(*border_color);
                    d.draw_rectangle_lines_ex(
                        rect, 
                        *border_width, 
                        border_raylib_color
                    );
                }
            }
            RenderCommand::DrawText {
                position,
                text,
                font_size,
                color,
                alignment,
                max_width,
                max_height,
            } => {
                let raylib_color = vec4_to_raylib_color(*color);
                
                // Calculate text positioning based on alignment
                let text_width = d.measure_text(text, *font_size as i32) as f32;
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
                
                d.draw_text(
                    text,
                    text_x as i32,
                    text_y as i32,
                    *font_size as i32,
                    raylib_color,
                );
            }
            RenderCommand::DrawImage {
                position,
                size,
                source,
                opacity,
            } => {
                eprintln!("[RAYLIB] DrawImage match arm reached for: {}", source);
                
                // Check if we have a cached texture
                if let Some(texture) = textures.get(source) {
                    // Draw the actual texture
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
            RenderCommand::SetCanvasSize(_) => {}
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
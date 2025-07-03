// crates/kryon-raylib/src/lib.rs
use kryon_render::{
    Renderer, CommandRenderer, RenderCommand, RenderResult, InputEvent, MouseButton, KeyCode, KeyModifiers
};
use kryon_layout::LayoutResult;
use glam::{Vec2, Vec4};
use raylib::prelude::*;
use std::collections::HashMap;

pub struct RaylibRenderer {
    handle: RaylibHandle,
    thread: RaylibThread,
    size: Vec2,
    textures: HashMap<String, Texture2D>,
    pending_commands: Vec<RenderCommand>,
    prev_mouse_pos: Vec2,
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
                eprintln!("[RaylibRenderer] Attempting to draw: {:?}", command);

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
        // Store commands to be executed in end_frame
        self.pending_commands.extend_from_slice(commands);
        Ok(())
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
    
    pub fn poll_input_events(&mut self) -> Vec<InputEvent> {
        let mut events = Vec::new();
        
        // Handle window resize
        if self.handle.is_window_resized() {
            let new_size = Vec2::new(
                self.handle.get_screen_width() as f32,
                self.handle.get_screen_height() as f32
            );
            events.push(InputEvent::Resize { size: new_size });
        }
        
        // Handle mouse events - READ POSITION EVERY FRAME
        // Force fresh read by clearing any potential cache
        
        // Method 1: Standard position
        let raw_mouse_x = self.handle.get_mouse_x() as f32;
        let raw_mouse_y = self.handle.get_mouse_y() as f32;
        
        // Method 2: Use raylib vector directly
        let raylib_mouse_vec = self.handle.get_mouse_position();
        
        // Method 3: Use mouse delta for tracking
        let mouse_delta = self.handle.get_mouse_delta();
        
        // Use the raylib vector method instead of separate x/y calls
        let mouse_pos = Vec2::new(raylib_mouse_vec.x, raylib_mouse_vec.y);
        
        // Check if mouse is on screen and window has focus
        let is_on_screen = self.handle.is_cursor_on_screen();
        let window_focused = self.handle.is_window_focused();
        
        // DEBUG - Log mouse position changes and window state
        static mut FRAME_COUNT: u32 = 0;
        unsafe { FRAME_COUNT += 1; }
        
        // ALWAYS log mouse position every frame
        eprintln!("Mouse: ({:.1}, {:.1})", mouse_pos.x, mouse_pos.y);
        
        // Generate mouse move events for ANY position change (no threshold)
        let is_first_read = self.prev_mouse_pos.x < 0.0;
        let mouse_moved = mouse_pos != self.prev_mouse_pos;
        let has_delta = mouse_delta.x != 0.0 || mouse_delta.y != 0.0;
        
        // Use delta as backup detection method
        if mouse_moved || is_first_read || has_delta {
            if mouse_moved {
                eprintln!("[MOUSE_DEBUG] *** MOUSE MOVED *** from ({:.1}, {:.1}) to ({:.1}, {:.1})", 
                    self.prev_mouse_pos.x, self.prev_mouse_pos.y, mouse_pos.x, mouse_pos.y);
            }
            if has_delta {
                eprintln!("[MOUSE_DEBUG] *** MOUSE DELTA *** delta: ({:.1}, {:.1})", mouse_delta.x, mouse_delta.y);
                // If we have delta but no position change, compute new position from delta
                if !mouse_moved && has_delta {
                    let new_pos = Vec2::new(
                        self.prev_mouse_pos.x + mouse_delta.x,
                        self.prev_mouse_pos.y + mouse_delta.y
                    );
                    eprintln!("[MOUSE_DEBUG] Using delta-computed position: ({:.1}, {:.1})", new_pos.x, new_pos.y);
                    events.push(InputEvent::MouseMove { position: new_pos });
                    self.prev_mouse_pos = new_pos;
                } else {
                    events.push(InputEvent::MouseMove { position: mouse_pos });
                    self.prev_mouse_pos = mouse_pos;
                }
            } else {
                events.push(InputEvent::MouseMove { position: mouse_pos });
                self.prev_mouse_pos = mouse_pos;
            }
        }
        
        // Mouse buttons - use current mouse position
        if self.handle.is_mouse_button_pressed(raylib::consts::MouseButton::MOUSE_BUTTON_LEFT) {
            eprintln!("[MOUSE_DEBUG] *** MOUSE PRESS *** at ({:.1}, {:.1})", mouse_pos.x, mouse_pos.y);
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
        
        // Keyboard events
        if let Some(key) = self.handle.get_key_pressed() {
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
        _textures: &mut HashMap<String, Texture2D>,
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
                position: _,
                size: _,
                source,
                opacity: _,
            } => {
                // TODO: Image loading requires access to handle and thread
                // For now, skip image rendering - this would need a different approach
                tracing::warn!("Image rendering not yet implemented for Raylib backend: {}", source);
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

    // >>>>>>>>> ADD THIS PRINTLN <<<<<<<<<<<
    eprintln!("[vec4_to_raylib_color] Final u8 color: r={}, g={}, b={}, a={}", r, g, b, a);
    
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
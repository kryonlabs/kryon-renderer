//! Integration between winit and web renderers

use wasm_bindgen::prelude::*;
use web_sys::{HtmlCanvasElement, Event, EventTarget};
use winit::{
    event::{Event as WinitEvent, WindowEvent, DeviceEvent, ElementState, MouseButton, VirtualKeyCode},
    event_loop::{EventLoop, ControlFlow},
    window::{Window, WindowBuilder},
    dpi::PhysicalSize,
};
use crate::{WebEvent, WebEventHandler};
use glam::Vec2;

/// Bridge between winit events and web events
pub struct WinitWebBridge {
    event_handler: WebEventHandler,
    canvas: HtmlCanvasElement,
    window: Window,
}

impl WinitWebBridge {
    pub fn new(canvas_id: &str, event_loop: &EventLoop<()>) -> Result<Self, JsValue> {
        // Get the canvas element
        let window = web_sys::window().ok_or("No window object")?;
        let document = window.document().ok_or("No document object")?;
        let canvas = document
            .get_element_by_id(canvas_id)
            .ok_or("Canvas element not found")?
            .dyn_into::<HtmlCanvasElement>()
            .map_err(|_| "Element is not a canvas")?;
        
        // Create winit window
        let winit_window = WindowBuilder::new()
            .with_title("Kryon Web Application")
            .with_inner_size(PhysicalSize::new(
                canvas.width() as u32,
                canvas.height() as u32,
            ))
            .build(event_loop)
            .map_err(|e| JsValue::from_str(&format!("Failed to create winit window: {}", e)))?;
        
        let mut event_handler = WebEventHandler::new();
        
        // Set up event listeners on the canvas
        event_handler.setup_event_listeners(canvas.as_ref())?;
        
        Ok(Self {
            event_handler,
            canvas,
            window: winit_window,
        })
    }
    
    /// Convert web events to winit events
    pub fn process_web_events(&mut self) -> Vec<WinitEvent<()>> {
        let web_events = self.event_handler.poll_events();
        let mut winit_events = Vec::new();
        
        for web_event in web_events {
            match web_event {
                WebEvent::MouseDown { position, button } => {
                    winit_events.push(WinitEvent::WindowEvent {
                        window_id: self.window.id(),
                        event: WindowEvent::MouseInput {
                            device_id: winit::event::DeviceId::dummy(),
                            state: ElementState::Pressed,
                            button: match button {
                                0 => MouseButton::Left,
                                1 => MouseButton::Middle,
                                2 => MouseButton::Right,
                                _ => MouseButton::Other(button as u16),
                            },
                        },
                    });
                    
                    winit_events.push(WinitEvent::WindowEvent {
                        window_id: self.window.id(),
                        event: WindowEvent::CursorMoved {
                            device_id: winit::event::DeviceId::dummy(),
                            position: winit::dpi::PhysicalPosition::new(position.x as f64, position.y as f64),
                        },
                    });
                }
                
                WebEvent::MouseUp { position, button } => {
                    winit_events.push(WinitEvent::WindowEvent {
                        window_id: self.window.id(),
                        event: WindowEvent::MouseInput {
                            device_id: winit::event::DeviceId::dummy(),
                            state: ElementState::Released,
                            button: match button {
                                0 => MouseButton::Left,
                                1 => MouseButton::Middle,
                                2 => MouseButton::Right,
                                _ => MouseButton::Other(button as u16),
                            },
                        },
                    });
                }
                
                WebEvent::MouseMove { position } => {
                    winit_events.push(WinitEvent::WindowEvent {
                        window_id: self.window.id(),
                        event: WindowEvent::CursorMoved {
                            device_id: winit::event::DeviceId::dummy(),
                            position: winit::dpi::PhysicalPosition::new(position.x as f64, position.y as f64),
                        },
                    });
                }
                
                WebEvent::KeyDown { key, code } => {
                    if let Some(virtual_key) = key_string_to_virtual_key(&key) {
                        winit_events.push(WinitEvent::WindowEvent {
                            window_id: self.window.id(),
                            event: WindowEvent::KeyboardInput {
                                device_id: winit::event::DeviceId::dummy(),
                                input: winit::event::KeyboardInput {
                                    scancode: 0,
                                    state: ElementState::Pressed,
                                    virtual_keycode: Some(virtual_key),
                                    modifiers: winit::event::ModifiersState::default(),
                                },
                                is_synthetic: false,
                            },
                        });
                    }
                }
                
                WebEvent::KeyUp { key, code } => {
                    if let Some(virtual_key) = key_string_to_virtual_key(&key) {
                        winit_events.push(WinitEvent::WindowEvent {
                            window_id: self.window.id(),
                            event: WindowEvent::KeyboardInput {
                                device_id: winit::event::DeviceId::dummy(),
                                input: winit::event::KeyboardInput {
                                    scancode: 0,
                                    state: ElementState::Released,
                                    virtual_keycode: Some(virtual_key),
                                    modifiers: winit::event::ModifiersState::default(),
                                },
                                is_synthetic: false,
                            },
                        });
                    }
                }
                
                WebEvent::Resize { size } => {
                    winit_events.push(WinitEvent::WindowEvent {
                        window_id: self.window.id(),
                        event: WindowEvent::Resized(PhysicalSize::new(
                            size.x as u32,
                            size.y as u32,
                        )),
                    });
                }
                
                _ => {
                    // Handle other events as needed
                }
            }
        }
        
        winit_events
    }
    
    /// Get the winit window
    pub fn window(&self) -> &Window {
        &self.window
    }
    
    /// Get the canvas element
    pub fn canvas(&self) -> &HtmlCanvasElement {
        &self.canvas
    }
    
    /// Update canvas size to match winit window
    pub fn sync_canvas_size(&mut self) {
        let size = self.window.inner_size();
        self.canvas.set_width(size.width);
        self.canvas.set_height(size.height);
    }
}

/// Convert a key string to a winit VirtualKeyCode
fn key_string_to_virtual_key(key: &str) -> Option<VirtualKeyCode> {
    match key {
        "Escape" => Some(VirtualKeyCode::Escape),
        "F1" => Some(VirtualKeyCode::F1),
        "F2" => Some(VirtualKeyCode::F2),
        "F3" => Some(VirtualKeyCode::F3),
        "F4" => Some(VirtualKeyCode::F4),
        "F5" => Some(VirtualKeyCode::F5),
        "F6" => Some(VirtualKeyCode::F6),
        "F7" => Some(VirtualKeyCode::F7),
        "F8" => Some(VirtualKeyCode::F8),
        "F9" => Some(VirtualKeyCode::F9),
        "F10" => Some(VirtualKeyCode::F10),
        "F11" => Some(VirtualKeyCode::F11),
        "F12" => Some(VirtualKeyCode::F12),
        "Tab" => Some(VirtualKeyCode::Tab),
        "Enter" => Some(VirtualKeyCode::Return),
        "Backspace" => Some(VirtualKeyCode::Back),
        "Delete" => Some(VirtualKeyCode::Delete),
        "Insert" => Some(VirtualKeyCode::Insert),
        "Home" => Some(VirtualKeyCode::Home),
        "End" => Some(VirtualKeyCode::End),
        "PageUp" => Some(VirtualKeyCode::PageUp),
        "PageDown" => Some(VirtualKeyCode::PageDown),
        "ArrowUp" => Some(VirtualKeyCode::Up),
        "ArrowDown" => Some(VirtualKeyCode::Down),
        "ArrowLeft" => Some(VirtualKeyCode::Left),
        "ArrowRight" => Some(VirtualKeyCode::Right),
        "Shift" => Some(VirtualKeyCode::LShift),
        "Control" => Some(VirtualKeyCode::LControl),
        "Alt" => Some(VirtualKeyCode::LAlt),
        "Meta" => Some(VirtualKeyCode::LWin),
        " " => Some(VirtualKeyCode::Space),
        "0" => Some(VirtualKeyCode::Key0),
        "1" => Some(VirtualKeyCode::Key1),
        "2" => Some(VirtualKeyCode::Key2),
        "3" => Some(VirtualKeyCode::Key3),
        "4" => Some(VirtualKeyCode::Key4),
        "5" => Some(VirtualKeyCode::Key5),
        "6" => Some(VirtualKeyCode::Key6),
        "7" => Some(VirtualKeyCode::Key7),
        "8" => Some(VirtualKeyCode::Key8),
        "9" => Some(VirtualKeyCode::Key9),
        "a" | "A" => Some(VirtualKeyCode::A),
        "b" | "B" => Some(VirtualKeyCode::B),
        "c" | "C" => Some(VirtualKeyCode::C),
        "d" | "D" => Some(VirtualKeyCode::D),
        "e" | "E" => Some(VirtualKeyCode::E),
        "f" | "F" => Some(VirtualKeyCode::F),
        "g" | "G" => Some(VirtualKeyCode::G),
        "h" | "H" => Some(VirtualKeyCode::H),
        "i" | "I" => Some(VirtualKeyCode::I),
        "j" | "J" => Some(VirtualKeyCode::J),
        "k" | "K" => Some(VirtualKeyCode::K),
        "l" | "L" => Some(VirtualKeyCode::L),
        "m" | "M" => Some(VirtualKeyCode::M),
        "n" | "N" => Some(VirtualKeyCode::N),
        "o" | "O" => Some(VirtualKeyCode::O),
        "p" | "P" => Some(VirtualKeyCode::P),
        "q" | "Q" => Some(VirtualKeyCode::Q),
        "r" | "R" => Some(VirtualKeyCode::R),
        "s" | "S" => Some(VirtualKeyCode::S),
        "t" | "T" => Some(VirtualKeyCode::T),
        "u" | "U" => Some(VirtualKeyCode::U),
        "v" | "V" => Some(VirtualKeyCode::V),
        "w" | "W" => Some(VirtualKeyCode::W),
        "x" | "X" => Some(VirtualKeyCode::X),
        "y" | "Y" => Some(VirtualKeyCode::Y),
        "z" | "Z" => Some(VirtualKeyCode::Z),
        _ => None,
    }
}
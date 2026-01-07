// src/lib.rs - Minimal
pub mod capture;
pub mod colorant;
pub mod mouse;

pub use capture::Capture;
pub use colorant::{ColorantEngine, Config, Action};
pub use mouse::ArduinoMouse;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

// Virtual key codes
pub const VK_F1: i32 = 0x70;
pub const VK_F2: i32 = 0x71;
pub const VK_F5: i32 = 0x76;
pub const VK_LSHIFT: i32 = 0xA0;
pub const VK_LCONTROL: i32 = 0xA2;
pub const VK_LMENU: i32 = 0xA4;
pub const VK_SPACE: i32 = 0x20;
pub const VK_LBUTTON: i32 = 0x01;
pub const VK_F: i32 = 0x46;

// Key check mask
pub const KEY_PRESSED_MASK: i16 = -32768i16;

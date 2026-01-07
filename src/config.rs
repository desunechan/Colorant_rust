use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ColorRange {
    pub lower: [u8; 3],
    pub upper: [u8; 3],
}

impl Default for ColorRange {
    fn default() -> Self {
        Self {
            lower: [140, 120, 180],
            upper: [160, 200, 255],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub x_fov: u32,
    pub y_fov: u32,
    pub flick_speed: f32,
    pub move_speed: f32,
    pub ingame_sensitivity: f32,
    pub hotkey_toggle: String,
    pub hotkey_window: String,
    pub color_range: ColorRange,
    pub screen_width: u32,
    pub screen_height: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            x_fov: 75,
            y_fov: 75,
            flick_speed: 4.67,
            move_speed: 0.434,
            ingame_sensitivity: 0.23,
            hotkey_toggle: "F1".to_string(),
            hotkey_window: "F2".to_string(),
            color_range: ColorRange::default(),
            screen_width: 1920,
            screen_height: 1080,
        }
    }
}

impl Config {
    pub fn calculate_speeds(&mut self) {
        self.flick_speed = 1.07437623 * (self.ingame_sensitivity.powf(-0.9936827126));
        self.move_speed = 1.0 / (10.0 * self.ingame_sensitivity);
    }
}
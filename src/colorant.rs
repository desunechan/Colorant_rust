use anyhow::Result;
use crate::capture::Capture;
use crate::mouse::ArduinoMouse;
use std::time::Duration;
use log::info;

#[derive(Debug, Clone, Copy)]
pub struct Config {
    pub x: i32,
    pub y: i32,
    pub x_fov: u32,
    pub y_fov: u32,
    pub ingame_sensitivity: f32,
    pub move_speed: f32,
    pub flick_speed: f32,
    pub lower_hsv: [u8; 3],  // CHANGED: Now HSV (H: 0-180, S: 0-255, V: 0-255)
    pub upper_hsv: [u8; 3],  // CHANGED: Now HSV
}

impl Default for Config {
    fn default() -> Self {
        Self {
            x: 0,
            y: 0,
            x_fov: 75,
            y_fov: 75,
            ingame_sensitivity: 0.23,
            move_speed: 0.435,
            flick_speed: 4.628,
            // EXACT Python HSV values from OpenCV
            lower_hsv: [140, 120, 180],  // H: 140-160, S: 120-200, V: 180-255
            upper_hsv: [160, 200, 255],
        }
    }
}

impl Config {
    pub fn calculate_speeds(&mut self) {
        self.flick_speed = 1.07437623 * self.ingame_sensitivity.powf(-0.9936827126);
        self.move_speed = 1.0 / (10.0 * self.ingame_sensitivity);
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Action {
    Move,
    Click,
    Flick,
}

pub struct ColorantEngine {
    config: Config,
    capture: Capture,
    mouse: ArduinoMouse,
    toggled: bool,
}

impl ColorantEngine {
    pub async fn new(config: Config) -> Result<Self> {
        let mut config = config;
        if config.move_speed == 0.0 || config.flick_speed == 0.0 {
            config.calculate_speeds();
        }
        
        let capture = Capture::with_fov(
            config.x,
            config.y,
            config.x_fov,
            config.y_fov,
        )?;
        
        let mouse_config = crate::mouse::MouseConfig::default();
        let mouse = ArduinoMouse::new(mouse_config)?;
        
        let engine = Self {
            config,
            capture,
            mouse,
            toggled: false,
        };
        
        Ok(engine)
    }
    
    pub fn toggle(&mut self) -> bool {
        self.toggled = !self.toggled;
        
        if self.toggled {
            self.capture.resume();
            info!("Colorant: ENABLED");
        } else {
            self.capture.pause();
            info!("Colorant: DISABLED");
        }
        
        self.toggled
    }
    
    pub fn is_enabled(&self) -> bool {
        self.toggled
    }
    
    pub async fn process_action(&mut self, action: Action) -> Result<()> {
        if !self.toggled {
            return Ok(());
        }
        
        let frame = match self.capture.get_frame_blocking(Duration::from_millis(100)) {
            Some(frame) => frame,
            None => return Ok(()),
        };
        
        // Find target using HSV color space (matching Python OpenCV)
        let target_pos = self.find_target_hsv(&frame).await;
        
        if let Some((target_x, target_y)) = target_pos {
            match action {
                Action::Move => {
                    let x_diff = target_x as f32 - (self.config.x_fov as f32 / 2.0);
                    let y_diff = target_y as f32 - (self.config.y_fov as f32 / 2.0);
                    
                    self.mouse.move_mouse(
                        x_diff * self.config.move_speed,
                        y_diff * self.config.move_speed,
                    ).await?;
                }
                
                Action::Click => {
                    let center_x_fov = self.config.x_fov as f32 / 2.0;
                    let center_y_fov = self.config.y_fov as f32 / 2.0;
                    
                    if (target_x as f32 - center_x_fov).abs() <= 4.0 &&
                       (target_y as f32 - center_y_fov).abs() <= 10.0 {
                        self.mouse.click().await?;
                    }
                }
                
                Action::Flick => {
                    let x_diff = (target_x as f32 + 2.0) - (self.config.x_fov as f32 / 2.0);
                    let y_diff = (target_y as f32) - (self.config.y_fov as f32 / 2.0);
                    
                    let flick_x = x_diff * self.config.flick_speed;
                    let flick_y = y_diff * self.config.flick_speed;
                    
                    self.mouse.flick(flick_x, flick_y).await?;
                    self.mouse.click().await?;
                    self.mouse.flick(-flick_x, -flick_y).await?;
                }
            }
        }
        
        Ok(())
    }
    
    async fn find_target_hsv(&self, frame: &image::RgbImage) -> Option<(i32, i32)> {
        let mut total_x = 0i64;
        let mut total_y = 0i64;
        let mut pixel_count = 0i64;
        
        for y in 0..frame.height() {
            for x in 0..frame.width() {
                let pixel = frame.get_pixel(x, y);
                let [r, g, b] = pixel.0;
                
                // Convert RGB to HSV (OpenCV-style: H 0-180, S 0-255, V 0-255)
                let (h, s, v) = self.rgb_to_hsv_opencv(r, g, b);
                
                // Check against Python HSV ranges
                if h >= self.config.lower_hsv[0] && h <= self.config.upper_hsv[0] &&
                   s >= self.config.lower_hsv[1] && s <= self.config.upper_hsv[1] &&
                   v >= self.config.lower_hsv[2] && v <= self.config.upper_hsv[2] {
                    total_x += x as i64;
                    total_y += y as i64;
                    pixel_count += 1;
                }
            }
        }
        
        if pixel_count > 0 {
            // Return center of mass (average position)
            Some((
                (total_x / pixel_count) as i32,
                (total_y / pixel_count) as i32
            ))
        } else {
            None
        }
    }
    
    fn rgb_to_hsv_opencv(&self, r: u8, g: u8, b: u8) -> (u8, u8, u8) {
        let r_f = r as f32 / 255.0;
        let g_f = g as f32 / 255.0;
        let b_f = b as f32 / 255.0;
        
        let max = r_f.max(g_f.max(b_f));
        let min = r_f.min(g_f.min(b_f));
        let delta = max - min;
        
        // Calculate Value (0-255)
        let v = (max * 255.0) as u8;
        
        // Calculate Saturation (0-255)
        let s = if max > 0.0 {
            (delta / max * 255.0) as u8
        } else {
            0
        };
        
        // Calculate Hue (OpenCV: 0-180 instead of 0-360)
        let mut h = 0.0;
        if delta > 0.0 {
            if max == r_f {
                h = 60.0 * (((g_f - b_f) / delta) % 6.0);
            } else if max == g_f {
                h = 60.0 * (((b_f - r_f) / delta) + 2.0);
            } else if max == b_f {
                h = 60.0 * (((r_f - g_f) / delta) + 4.0);
            }
            
            if h < 0.0 {
                h += 360.0;
            }
        }
        
        // OpenCV scales H to 0-180 (divide by 2)
        let h_out = (h / 2.0) as u8;
        
        (h_out, s, v)
    }
    
    pub fn close(&mut self) {
        self.capture.stop();
        self.mouse.close();
        info!("Colorant engine stopped");
    }
}

impl Drop for ColorantEngine {
    fn drop(&mut self) {
        self.close();
    }
}
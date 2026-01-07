use anyhow::Result;
use crate::capture::Capture;
use crate::mouse::ArduinoMouse;
use std::time::Duration;
use log::{info, debug};

#[derive(Debug, Clone, Copy)]
pub struct Config {
    pub x: i32,
    pub y: i32,
    pub x_fov: u32,
    pub y_fov: u32,
    pub ingame_sensitivity: f32,
    pub move_speed: f32,
    pub flick_speed: f32,
    pub lower_hsv: [u8; 3],
    pub upper_hsv: [u8; 3],
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
            // Python OpenCV HSV ranges for purple targets
            lower_hsv: [140, 120, 180],
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
            info!("🎯 Colorant: ENABLED");
        } else {
            self.capture.pause();
            info!("⏸️  Colorant: DISABLED");
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
            None => {
                println!("[ERROR] No frame captured from screen");
                return Ok(());
            }
        };
        
        // DEBUG: Print center pixel color
        let center_x = frame.width() / 2;
        let center_y = frame.height() / 2;
        let pixel = frame.get_pixel(center_x, center_y);
        let [r, g, b] = pixel.0;
        let (h, s, v) = self.rgb_to_hsv_opencv(r, g, b);
        println!("[DEBUG] Center pixel at ({}, {}):", center_x, center_y);
        println!("        RGB: ({}, {}, {})", r, g, b);
        println!("        HSV: ({}, {}, {})", h, s, v);
        println!("[DEBUG] Looking for H:{}-{} S:{}-{} V:{}-{}", 
            self.config.lower_hsv[0], self.config.upper_hsv[0],
            self.config.lower_hsv[1], self.config.upper_hsv[1],
            self.config.lower_hsv[2], self.config.upper_hsv[2]);
        
        // Find target using HSV color space
        let target_pos = self.find_target_hsv(&frame);
        
        match target_pos {
            Some((target_x, target_y)) => {
                println!("[SUCCESS] Found target at ({}, {})", target_x, target_y);
                println!("[INFO] FOV center: ({}, {})", 
                    self.config.x_fov / 2, self.config.y_fov / 2);
                
                match action {
                    Action::Move => {
                        let x_diff = target_x as f32 - (self.config.x_fov as f32 / 2.0);
                        let y_diff = target_y as f32 - (self.config.y_fov as f32 / 2.0);
                        
                        println!("[MOVE] Difference: x={:.2}, y={:.2}", x_diff, y_diff);
                        println!("[MOVE] Command: x={:.2}, y={:.2}", 
                            x_diff * self.config.move_speed, 
                            y_diff * self.config.move_speed);
                        
                        self.mouse.move_mouse(
                            x_diff * self.config.move_speed,
                            y_diff * self.config.move_speed,
                        ).await?;
                    }
                    
                    Action::Click => {
                        let center_x_fov = self.config.x_fov as f32 / 2.0;
                        let center_y_fov = self.config.y_fov as f32 / 2.0;
                        
                        println!("[CLICK] Checking if centered...");
                        println!("        Target: ({}, {})", target_x, target_y);
                        println!("        Center: ({:.1}, {:.1})", center_x_fov, center_y_fov);
                        println!("        Diff: x={:.1}, y={:.1}", 
                            target_x as f32 - center_x_fov,
                            target_y as f32 - center_y_fov);
                        
                        if (target_x as f32 - center_x_fov).abs() <= 4.0 &&
                           (target_y as f32 - center_y_fov).abs() <= 10.0 {
                            println!("[CLICK] Target centered - clicking!");
                            self.mouse.click().await?;
                        } else {
                            println!("[CLICK] Target not centered - no click");
                        }
                    }
                    
                    Action::Flick => {
                        let x_diff = target_x as f32 - (self.config.x_fov as f32 / 2.0);
                        let y_diff = target_y as f32 - (self.config.y_fov as f32 / 2.0);
                        
                        let flick_x = x_diff * self.config.flick_speed;
                        let flick_y = y_diff * self.config.flick_speed;
                        
                        println!("[FLICK] Command: x={:.2}, y={:.2}", flick_x, flick_y);
                        
                        self.mouse.flick(flick_x, flick_y).await?;
                        self.mouse.click().await?;
                        self.mouse.flick(-flick_x, -flick_y).await?;
                    }
                }
            }
            None => {
                println!("[ERROR] No target found in the capture area!");
                println!("[TIPS] 1. Check if purple target is visible");
                println!("       2. Adjust HSV ranges if needed");
                println!("       3. Verify screen capture is working");
            }
        }
        
        Ok(())
    }
    
    // FIXED: Removed 'async' keyword - this is a synchronous function
    fn find_target_hsv(&self, frame: &image::RgbImage) -> Option<(i32, i32)> {
        let mut total_x = 0i64;
        let mut total_y = 0i64;
        let mut pixel_count = 0i64;
        
        println!("[SCAN] Scanning {}x{} image for purple...", 
            frame.width(), frame.height());
        
        // Scan the entire frame
        for y in 0..frame.height() {
            for x in 0..frame.width() {
                let pixel = frame.get_pixel(x, y);
                let [r, g, b] = pixel.0;
                
                // Convert RGB to HSV (OpenCV-style)
                let (h, s, v) = self.rgb_to_hsv_opencv(r, g, b);
                
                // Check against HSV ranges
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
            let avg_x = (total_x / pixel_count) as i32;
            let avg_y = (total_y / pixel_count) as i32;
            
            println!("[SCAN] Found {} purple pixels", pixel_count);
            println!("[SCAN] Center of mass: ({}, {})", avg_x, avg_y);
            
            Some((avg_x, avg_y))
        } else {
            println!("[SCAN] No purple pixels found at all!");
            println!("[SCAN] Try adjusting HSV ranges or check color");
            None
        }
    }
    
    // CORRECTED RGB to HSV conversion (OpenCV style)
    fn rgb_to_hsv_opencv(&self, r: u8, g: u8, b: u8) -> (u8, u8, u8) {
        let rf = r as f32 / 255.0;
        let gf = g as f32 / 255.0;
        let bf = b as f32 / 255.0;
        
        let max = rf.max(gf.max(bf));
        let min = rf.min(gf.min(bf));
        let delta = max - min;
        
        // Value (brightness)
        let v = (max * 255.0).round() as u8;
        
        // Saturation
        let s = if max > 0.0 {
            (delta / max * 255.0).round() as u8
        } else {
            0
        };
        
        // Hue calculation (OpenCV: 0-180 range)
        let mut h = 0.0_f32;
        
        if delta > 0.0 {
            if max == rf {
                h = 60.0 * ((gf - bf) / delta);
            } else if max == gf {
                h = 60.0 * ((bf - rf) / delta + 2.0);
            } else if max == bf {
                h = 60.0 * ((rf - gf) / delta + 4.0);
            }
            
            // Normalize to 0-360
            if h < 0.0 {
                h += 360.0;
            }
        }
        
        // OpenCV uses 0-180 range (divide by 2)
        let h_out = (h / 2.0).round() as u8;
        
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

// src/mouse.rs - Fixed
use rand::{RngCore, rngs::StdRng, SeedableRng};
use anyhow::Result;
use serialport::SerialPort;  // REMOVED SerialPortType
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use log::info;

#[derive(Debug, Clone)]
pub struct MouseConfig {
    pub baud_rate: u32,
    pub filter_length: usize,
    pub reconnect_attempts: u32,
    pub reconnect_delay_ms: u64,
    pub humanize_delay: bool,
    pub min_click_delay_ms: u64,
    pub max_click_delay_ms: u64,
}

impl Default for MouseConfig {
    fn default() -> Self {
        Self {
            baud_rate: 115200,
            filter_length: 3,
            reconnect_attempts: 5,
            reconnect_delay_ms: 1000,
            humanize_delay: true,
            min_click_delay_ms: 10,
            max_click_delay_ms: 100,
        }
    }
}

pub struct ArduinoMouse {
    config: MouseConfig,
    port: Arc<Mutex<Option<Box<dyn SerialPort>>>>,
    port_name: String,
    x_history: Vec<f32>,
    y_history: Vec<f32>,
    last_reconnect: Instant,
    is_connected: bool,
}

impl ArduinoMouse {
    pub fn new(config: MouseConfig) -> Result<Self> {
        let port_name = Self::find_arduino_port()?;
        
        let mut mouse = Self {
            config,
            port: Arc::new(Mutex::new(None)),
            port_name: port_name.clone(),
            x_history: Vec::new(),
            y_history: Vec::new(),
            last_reconnect: Instant::now(),
            is_connected: false,
        };
        
        mouse.connect()?;
        
        Ok(mouse)
    }
    
    fn find_arduino_port() -> Result<String> {
        let ports = serialport::available_ports()?;
        
        if ports.is_empty() {
            anyhow::bail!("No COM ports found");
        }
        
        // Try common ports first
        let common_ports = ["COM9", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM10"];
        
        for port_name in common_ports {
            if ports.iter().any(|p| p.port_name == port_name) {
                if Self::test_port(port_name).is_ok() {
                    return Ok(port_name.to_string());
                }
            }
        }
        
        // Try any port
        for port in &ports {
            if Self::test_port(&port.port_name).is_ok() {
                return Ok(port.port_name.clone());
            }
        }
        
        anyhow::bail!("No suitable COM port found");
    }
    
    fn test_port(port_name: &str) -> Result<()> {
        let mut port = serialport::new(port_name, 115200)
            .timeout(Duration::from_millis(100))
            .open()?;
            
        port.write(b"\n")?;
        std::thread::sleep(Duration::from_millis(100));
        
        Ok(())
    }
    
    fn connect(&mut self) -> Result<()> {
        let port = serialport::new(&self.port_name, self.config.baud_rate)
            .timeout(Duration::from_millis(100))
            .open()?;
            
        *self.port.lock().unwrap() = Some(port);
        self.is_connected = true;
        self.last_reconnect = Instant::now();
        
        std::thread::sleep(Duration::from_secs(2));
        
        info!("Connected to Arduino on {}", self.port_name);
        
        Ok(())
    }
    
    fn reconnect(&mut self) -> Result<()> {
        if self.last_reconnect.elapsed() < Duration::from_millis(self.config.reconnect_delay_ms) {
            return Err(anyhow::anyhow!("Too soon to reconnect"));
        }
        
        // Close existing
        if let Some(mut port) = self.port.lock().unwrap().take() {
            let _ = port.flush();
        }
        
        // Try to reconnect
        self.connect()
    }
    
    pub async fn move_mouse(&mut self, x: f32, y: f32) -> Result<()> {
        if !self.is_connected {
            self.reconnect()?;
        }
        
        self.x_history.push(x);
        self.y_history.push(y);
        
        if self.x_history.len() > self.config.filter_length {
            self.x_history.remove(0);
            self.y_history.remove(0);
        }
        
        let smooth_x = self.x_history.iter().sum::<f32>() / self.x_history.len() as f32;
        let smooth_y = self.y_history.iter().sum::<f32>() / self.y_history.len() as f32;
        
        let final_x = if smooth_x < 0.0 {
            (smooth_x + 256.0) as u8
        } else {
            smooth_x as u8
        };
        
        let final_y = if smooth_y < 0.0 {
            (smooth_y + 256.0) as u8
        } else {
            smooth_y as u8
        };
        
        if let Some(port) = self.port.lock().unwrap().as_mut() {
            port.write(&[b'M', final_x, final_y])?;
            Ok(())
        } else {
            self.is_connected = false;
            Err(anyhow::anyhow!("Serial port not open"))
        }
    }
    
    pub async fn flick(&mut self, x: f32, y: f32) -> Result<()> {
        self.move_mouse(x, y).await
    }
    
    pub async fn click(&mut self) -> Result<()> {
        if !self.is_connected {
            self.reconnect()?;
        }
        
        if self.config.humanize_delay {
            // Use StdRng with from_entropy (requires SeedableRng trait)
            let mut rng = StdRng::from_entropy();
            
            let delay = rng.next_u32() as u64 % 
                (self.config.max_click_delay_ms - self.config.min_click_delay_ms + 1) 
                + self.config.min_click_delay_ms;
                
            tokio::time::sleep(Duration::from_millis(delay)).await;
        }
        
        if let Some(port) = self.port.lock().unwrap().as_mut() {
            port.write(&[b'C'])?;
            Ok(())
        } else {
            self.is_connected = false;
            Err(anyhow::anyhow!("Serial port not open"))
        }
    }
    
    pub fn is_connected(&self) -> bool {
        self.is_connected
    }
    
    pub fn close(&mut self) {
        if let Some(mut port) = self.port.lock().unwrap().take() {
            let _ = port.flush();
        }
        self.is_connected = false;
        info!("Arduino connection closed");
    }
}

impl Drop for ArduinoMouse {
    fn drop(&mut self) {
        self.close();
    }
}

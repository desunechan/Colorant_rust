use colorant_rust::*;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::time::Duration;
use tokio::time::sleep;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::Win32::System::Console::*;
use log::LevelFilter;

// Hide console window
fn hide_console() {
    unsafe {
        let console_window = GetConsoleWindow();
        if console_window.0 != 0 {
            ShowWindow(console_window, SW_HIDE);
        }
    }
}

// Setup logging to file for debugging
fn setup_logging() {
    env_logger::builder()
        .filter_level(LevelFilter::Warn) // Only show warnings and errors
        .format(|buf, record| {
            use std::io::Write;
            writeln!(buf, "[{}] {}: {}", 
                chrono::Local::now().format("%H:%M:%S"),
                record.level(),
                record.args()
            )
        })
        .target(env_logger::Target::Pipe(Box::new(std::fs::File::create("colorant.log").unwrap())))
        .init();
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Hide console window
    hide_console();
    
    // Setup logging
    setup_logging();
    
    // Spoof process name
    //process::spoof_process_name("svchost.exe");
    
    // Create configuration
    let mut config = Config::default();
    config.calculate_speeds();
    
    println!("==========================================");
    println!("🎮 Colorant Rust v{}", VERSION);
    println!("==========================================");
    
    println!("📋 Configuration:");
    println!("   FOV: {}x{}", config.x_fov, config.y_fov);
    println!("   Sensitivity: {:.2}", config.ingame_sensitivity);
    println!("   Move Speed: {:.3}", config.move_speed);
    println!("   Flick Speed: {:.3}", config.flick_speed);
    
    // Create engine
    let engine = Arc::new(Mutex::new(ColorantEngine::new(config).await?));
    
    println!("\n🎯 COLORANT SYSTEM ACTIVE");
    println!("=========================");
    println!("✅ Arduino connected");
    println!("🎮 Press F1 to toggle aimbot");
    println!("🎮 Hold movement keys when aimbot is ON");
    println!("🎮 Press Ctrl+C to exit");
    println!("\n[SYSTEM] Starting monitoring loop...");

    let mut engine_lock = engine.lock().await;
    engine_lock.toggle(); // Enable for testing
    
    // Quick test capture
    println!("\n🔍 Running color detection test...");
    if let Err(e) = engine_lock.process_action(Action::Move).await {
        println!("[TEST] Initial test failed: {}", e);
    } else {
        println!("[TEST] Initial test completed");
    }
    
    engine_lock.toggle(); // Disable after test
    
    // Create hotkey manager
    //let hotkey_config = hotkey::HotkeyConfig::default();
    /*let hotkey_manager = hotkey::HotkeyManager::new(hotkey_config);
    
    // Setup F1 toggle hotkey
    hotkey_manager.register_hotkey(VK_F1, {
        let engine_clone = Arc::clone(&engine);
        move || {
            let engine = engine_clone.clone();
            tokio::spawn(async move {
                let mut engine_lock = engine.lock().await;
                let enabled = engine_lock.toggle();
                println!("[HOTKEY] F1 pressed - Aimbot: {}", 
                    if enabled { "✅ ENABLED" } else { "⏸️  DISABLED" });
            });
        }
    });
    */
    
    
    // Main monitoring loop
    let mut last_f1_state = false;
    let mut last_f5_state = false;
    let mut last_f2_state = false;
    
    loop {
        // Check all keys at once
        let (f1_pressed, f2_pressed, f5_pressed, should_move, alt_pressed) = unsafe {
            use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
            
            (
                (GetAsyncKeyState(VK_F1) & KEY_PRESSED_MASK) != 0,
                (GetAsyncKeyState(VK_F2) & KEY_PRESSED_MASK) != 0,
                (GetAsyncKeyState(VK_F5) & KEY_PRESSED_MASK) != 0,
                (GetAsyncKeyState(VK_LSHIFT) & KEY_PRESSED_MASK) != 0 ||
                (GetAsyncKeyState(VK_LBUTTON) & KEY_PRESSED_MASK) != 0 ||
                (GetAsyncKeyState(VK_LCONTROL) & KEY_PRESSED_MASK) != 0 ||
                (GetAsyncKeyState(VK_F) & KEY_PRESSED_MASK) != 0 ||
                (GetAsyncKeyState(VK_SPACE) & KEY_PRESSED_MASK) != 0,
                (GetAsyncKeyState(VK_LMENU) & KEY_PRESSED_MASK) != 0,
            )
        };
        
        // Handle F1 toggle
        if f1_pressed && !last_f1_state {
            let mut engine_lock = engine.lock().await;
            let enabled = engine_lock.toggle();
            println!("[HOTKEY] F1 pressed - Aimbot: {}", 
                if enabled { "✅ ENABLED" } else { "⏸️  DISABLED" });
        }
        last_f1_state = f1_pressed;
        
        // Get engine state
        let engine_enabled = {
            let engine_lock = engine.lock().await;
            engine_lock.is_enabled()
        };
        
        // Monitor Vanguard (auto-toggle)
        /*{
            let mut engine_lock = engine.lock().await;
            engine_lock.monitor_vanguard().await;
        }*/
        
        if engine_enabled {
            // Process movement - DO NOT spawn new threads
            if should_move {
                let mut engine_lock = engine.lock().await;
                if let Err(e) = engine_lock.process_action(Action::Move).await {
                    eprintln!("[ERROR] Move failed: {}", e);
                }
            }
            
            // Process Alt click
            if alt_pressed {
                let mut engine_lock = engine.lock().await;
                if let Err(e) = engine_lock.process_action(Action::Click).await {
                    eprintln!("[ERROR] Click failed: {}", e);
                }
            }
            
            // Process F5 flick
            if f5_pressed && !last_f5_state {
                let mut engine_lock = engine.lock().await;
                if let Err(e) = engine_lock.process_action(Action::Flick).await {
                    eprintln!("[ERROR] Flick failed: {}", e);
                }
            }
        }
        
        last_f5_state = f5_pressed;
        
        // Handle F2 status
        if f2_pressed && !last_f2_state {
            println!("[STATUS] Aimbot: {}", 
                if engine_enabled { "ACTIVE" } else { "INACTIVE" });
        }
        last_f2_state = f2_pressed;
        
        // Small delay
        sleep(Duration::from_millis(10)).await;
    }
}

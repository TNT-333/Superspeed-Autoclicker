#![windows_subsystem = "windows"]

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread;
use std::time::{Duration, Instant};
use std::io::{BufReader, Cursor};
use std::fs::File;
use std::io::{Write, BufRead};

use enigo::*;
use rdev::{listen, Event, EventType, Button, Key as RdevKey};

use eframe::{egui, Frame, App};

use rodio;

// Simple config structure
#[derive(Debug)]
struct Config {
    hotkey: String,  // e.g. "Insert"
    fast_mode: bool,
    target_cps: f64,
    left_click: bool,
    middle_click: bool,
    right_click: bool,
    play_sound: bool,
}

impl Config {
    fn load() -> Option<Config> {
        let file = File::open("config.txt").ok()?;
        let reader = BufReader::new(file);
        // Set defaults
        let mut hotkey = "Insert".to_string();
        let mut fast_mode = true;
        let mut target_cps = 10.0;
        let mut left_click = true;
        let mut middle_click = false;
        let mut right_click = false;
        let mut play_sound = false;

        for line in reader.lines() {
            if let Ok(line) = line {
                let parts: Vec<&str> = line.splitn(2, '=').collect();
                if parts.len() != 2 { continue; }
                let key = parts[0].trim();
                let value = parts[1].trim();
                match key {
                    "hotkey" => hotkey = value.to_string(),
                    "fast_mode" => fast_mode = value.parse::<bool>().unwrap_or(true),
                    "target_cps" => target_cps = value.parse::<f64>().unwrap_or(10.0),
                    "left_click" => left_click = value.parse::<bool>().unwrap_or(true),
                    "middle_click" => middle_click = value.parse::<bool>().unwrap_or(false),
                    "right_click" => right_click = value.parse::<bool>().unwrap_or(false),
                    "play_sound" => play_sound = value.parse::<bool>().unwrap_or(false),
                    _ => {},
                }
            }
        }

        Some(Config {
            hotkey,
            fast_mode,
            target_cps,
            left_click,
            middle_click,
            right_click,
            play_sound,
        })
    }

    fn save(&self) -> std::io::Result<()> {
        let mut file = File::create("config.txt")?;
        writeln!(file, "hotkey={}", self.hotkey)?;
        writeln!(file, "fast_mode={}", self.fast_mode)?;
        writeln!(file, "target_cps={}", self.target_cps)?;
        writeln!(file, "left_click={}", self.left_click)?;
        writeln!(file, "middle_click={}", self.middle_click)?;
        writeln!(file, "right_click={}", self.right_click)?;
        writeln!(file, "play_sound={}", self.play_sound)?;
        Ok(())
    }
}

// Utility functions for converting rdev::Key to/from String.
fn key_to_string(key: &rdev::Key) -> String {
    match key {
        rdev::Key::Insert => "Insert".to_string(),
        rdev::Key::F1 => "F1".to_string(),
        rdev::Key::F2 => "F2".to_string(),
        rdev::Key::F3 => "F3".to_string(),
        rdev::Key::F4 => "F4".to_string(),
        rdev::Key::F5 => "F5".to_string(),
        rdev::Key::F6 => "F6".to_string(),
        rdev::Key::F7 => "F7".to_string(),
        rdev::Key::F8 => "F8".to_string(),
        rdev::Key::F9 => "F9".to_string(),
        rdev::Key::F10 => "F10".to_string(),
        rdev::Key::F11 => "F11".to_string(),
        rdev::Key::F12 => "F12".to_string(),
        rdev::Key::KeyA => "A".to_string(),
        rdev::Key::KeyB => "B".to_string(),
        rdev::Key::KeyC => "C".to_string(),
        rdev::Key::KeyD => "D".to_string(),
        rdev::Key::KeyE => "E".to_string(),
        rdev::Key::KeyF => "F".to_string(),
        rdev::Key::KeyG => "G".to_string(),
        rdev::Key::KeyH => "H".to_string(),
        rdev::Key::KeyI => "I".to_string(),
        rdev::Key::KeyJ => "J".to_string(),
        rdev::Key::KeyK => "K".to_string(),
        rdev::Key::KeyL => "L".to_string(),
        rdev::Key::KeyM => "M".to_string(),
        rdev::Key::KeyN => "N".to_string(),
        rdev::Key::KeyO => "O".to_string(),
        rdev::Key::KeyP => "P".to_string(),
        rdev::Key::KeyQ => "Q".to_string(),
        rdev::Key::KeyR => "R".to_string(),
        rdev::Key::KeyS => "S".to_string(),
        rdev::Key::KeyT => "T".to_string(),
        rdev::Key::KeyU => "U".to_string(),
        rdev::Key::KeyV => "V".to_string(),
        rdev::Key::KeyW => "W".to_string(),
        rdev::Key::KeyX => "X".to_string(),
        rdev::Key::KeyY => "Y".to_string(),
        rdev::Key::KeyZ => "Z".to_string(),
        _ => format!("{:?}", key),
    }
}

fn string_to_rdev_key(s: &str) -> Option<rdev::Key> {
    let s_lower = s.to_lowercase();
    match s_lower.as_str() {
        "insert" => Some(rdev::Key::Insert),
        "f1" => Some(rdev::Key::F1),
        "f2" => Some(rdev::Key::F2),
        "f3" => Some(rdev::Key::F3),
        "f4" => Some(rdev::Key::F4),
        "f5" => Some(rdev::Key::F5),
        "f6" => Some(rdev::Key::F6),
        "f7" => Some(rdev::Key::F7),
        "f8" => Some(rdev::Key::F8),
        "f9" => Some(rdev::Key::F9),
        "f10" => Some(rdev::Key::F10),
        "f11" => Some(rdev::Key::F11),
        "f12" => Some(rdev::Key::F12),
        _ if s.len() == 1 => {
            let ch = s.chars().next()?.to_ascii_uppercase();
            match ch {
                'A' => Some(rdev::Key::KeyA),
                'B' => Some(rdev::Key::KeyB),
                'C' => Some(rdev::Key::KeyC),
                'D' => Some(rdev::Key::KeyD),
                'E' => Some(rdev::Key::KeyE),
                'F' => Some(rdev::Key::KeyF),
                'G' => Some(rdev::Key::KeyG),
                'H' => Some(rdev::Key::KeyH),
                'I' => Some(rdev::Key::KeyI),
                'J' => Some(rdev::Key::KeyJ),
                'K' => Some(rdev::Key::KeyK),
                'L' => Some(rdev::Key::KeyL),
                'M' => Some(rdev::Key::KeyM),
                'N' => Some(rdev::Key::KeyN),
                'O' => Some(rdev::Key::KeyO),
                'P' => Some(rdev::Key::KeyP),
                'Q' => Some(rdev::Key::KeyQ),
                'R' => Some(rdev::Key::KeyR),
                'S' => Some(rdev::Key::KeyS),
                'T' => Some(rdev::Key::KeyT),
                'U' => Some(rdev::Key::KeyU),
                'V' => Some(rdev::Key::KeyV),
                'W' => Some(rdev::Key::KeyW),
                'X' => Some(rdev::Key::KeyX),
                'Y' => Some(rdev::Key::KeyY),
                'Z' => Some(rdev::Key::KeyZ),
                _ => None,
            }
        },
        _ => None,
    }
}

/// Shared application state.
struct AppState {
    clicking: Arc<AtomicBool>,
    hotkey: Arc<Mutex<rdev::Key>>,
    changing_hotkey: Arc<AtomicBool>,
    clicks: Arc<Mutex<Vec<Instant>>>,
    fast_mode: Arc<AtomicBool>,
    target_cps: Arc<Mutex<f64>>,
    // Mouse button selections.
    left_click: bool,
    middle_click: bool,
    right_click: bool,
    selected_buttons: Arc<Mutex<Vec<MouseButton>>>,
    play_sound: Arc<AtomicBool>,
}

impl AppState {
    fn from_config(config: Config) -> Self {
        let hotkey = string_to_rdev_key(&config.hotkey).unwrap_or(rdev::Key::Insert);
        let mut buttons = Vec::new();
        if config.left_click { buttons.push(MouseButton::Left); }
        if config.middle_click { buttons.push(MouseButton::Middle); }
        if config.right_click { buttons.push(MouseButton::Right); }
        AppState {
            clicking: Arc::new(AtomicBool::new(false)),
            hotkey: Arc::new(Mutex::new(hotkey)),
            changing_hotkey: Arc::new(AtomicBool::new(false)),
            clicks: Arc::new(Mutex::new(Vec::new())),
            fast_mode: Arc::new(AtomicBool::new(config.fast_mode)),
            target_cps: Arc::new(Mutex::new(config.target_cps)),
            left_click: config.left_click,
            middle_click: config.middle_click,
            right_click: config.right_click,
            selected_buttons: Arc::new(Mutex::new(buttons)),
            play_sound: Arc::new(AtomicBool::new(config.play_sound)),
        }
    }

    fn to_config(&self) -> Config {
        let hotkey = {
            let hk = self.hotkey.lock().unwrap();
            key_to_string(&hk)
        };
        let fast_mode = self.fast_mode.load(Ordering::SeqCst);
        let target_cps = *self.target_cps.lock().unwrap();
        let left_click = self.left_click;
        let middle_click = self.middle_click;
        let right_click = self.right_click;
        let play_sound = self.play_sound.load(Ordering::SeqCst);
        Config {
            hotkey,
            fast_mode,
            target_cps,
            left_click,
            middle_click,
            right_click,
            play_sound,
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        AppState::from_config(Config {
            hotkey: "Insert".to_string(),
            fast_mode: true,
            target_cps: 10.0,
            left_click: true,
            middle_click: false,
            right_click: false,
            play_sound: false,
        })
    }
}

impl App for AppState {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        // Remove old clicks (older than 1 sec) for CPS count.
        let now = Instant::now();
        {
            let mut clicks = self.clicks.lock().unwrap();
            clicks.retain(|&t| now.duration_since(t) <= Duration::from_secs(1));
        }
        let cps = self.clicks.lock().unwrap().len() as f64;

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Superspeed Autoclicker");

            // Autoclicker toggle.
            ui.horizontal(|ui| {
                let state_str = if self.clicking.load(Ordering::SeqCst) {
                    "Running"
                } else {
                    "Stopped"
                };
                ui.label(format!("Autoclicker: {}", state_str));
                if ui.button("Toggle Autoclicker").clicked() {
                    let new_state = !self.clicking.load(Ordering::SeqCst);
                    self.clicking.store(new_state, Ordering::SeqCst);
                }
            });
            ui.separator();

            // Hotkey section.
            {
                let current_hotkey = {
                    let hk = self.hotkey.lock().unwrap();
                    key_to_string(&hk)
                };
                ui.label(format!("Start/Stop Key: {}", current_hotkey));
                if ui.button("Change start/stop key").clicked() {
                    self.changing_hotkey.store(true, Ordering::SeqCst);
                }
                if self.changing_hotkey.load(Ordering::SeqCst) {
                    ui.label("Press a key to set as new hotkey...");
                }
            }
            ui.separator();

            // Click rate mode.
            let fast_mode = self.fast_mode.load(Ordering::SeqCst);
            ui.horizontal(|ui| {
                ui.label("Click Rate Mode:");
                if ui.radio_value(&mut { fast_mode }, true, "Superspeed").clicked() {
                    self.fast_mode.store(true, Ordering::SeqCst);
                }
                if ui.radio_value(&mut { fast_mode }, false, "Consistent Rate").clicked() {
                    self.fast_mode.store(false, Ordering::SeqCst);
                }
            });
            if !self.fast_mode.load(Ordering::SeqCst) {
                let mut cps_target = *self.target_cps.lock().unwrap();
                ui.add(egui::Slider::new(&mut cps_target, 1.0..=1000.0).text("Target CPS (1-1000)"));
                *self.target_cps.lock().unwrap() = cps_target;
            }
            ui.separator();

            // Mouse button selection.
            ui.heading("Mouse Button Actions");
            ui.checkbox(&mut self.left_click, "Left Click");
            ui.checkbox(&mut self.middle_click, "Middle Click");
            ui.checkbox(&mut self.right_click, "Right Click");
            {
                let mut buttons = self.selected_buttons.lock().unwrap();
                buttons.clear();
                if self.left_click { buttons.push(MouseButton::Left); }
                if self.middle_click { buttons.push(MouseButton::Middle); }
                if self.right_click { buttons.push(MouseButton::Right); }
            }
            ui.separator();

            // Sound option.
            let mut play_sound_val = self.play_sound.load(Ordering::SeqCst);
            ui.checkbox(&mut play_sound_val, "Play sound on click");
            self.play_sound.store(play_sound_val, Ordering::SeqCst);
            ui.separator();

            // CPS test.
            egui::CollapsingHeader::new("CPS Test")
                .default_open(true)
                .show(ui, |ui| {
                    ui.label(format!("Current CPS: {:.1}", cps));
                    ui.label("Click anywhere using the left mouse button to test.");
                });
            ui.separator();

            // Save Config button.
            if ui.button("Save Config").clicked() {
                if let Err(e) = self.to_config().save() {
                    ui.label(format!("Failed to save config: {}", e));
                } else {
                    ui.label("Config saved.");
                }
            }
        });

        ctx.request_repaint();
    }
}

fn main() {
    // Initialize rodio for sound playback.
    let (_stream, stream_handle) =
        rodio::OutputStream::try_default().expect("Failed to get audio output stream");
    let stream_handle = Arc::new(stream_handle);
    let click_sound_data =
        Arc::new(std::fs::read("click.wav").expect("Failed to read click.wav"));

    // Load configuration (or use defaults).
    let app_state = if let Some(config) = Config::load() {
        AppState::from_config(config)
    } else {
        AppState::default()
    };

    // Autoclicker thread.
    let clicking_for_clicker = app_state.clicking.clone();
    let fast_mode = app_state.fast_mode.clone();
    let target_cps = app_state.target_cps.clone();
    let play_sound = app_state.play_sound.clone();
    let selected_buttons = app_state.selected_buttons.clone();

    thread::spawn(move || {
        let mut enigo = Enigo::new();
        loop {
            if clicking_for_clicker.load(Ordering::SeqCst) {
                let buttons = selected_buttons.lock().unwrap().clone();
                if buttons.is_empty() {
                    enigo.mouse_click(MouseButton::Left);
                } else {
                    for btn in &buttons {
                        enigo.mouse_down(*btn);
                        enigo.mouse_up(*btn);
                    }
                    if play_sound.load(Ordering::SeqCst) {
                        let stream_handle = stream_handle.clone();
                        let click_sound_data = click_sound_data.clone();
                        thread::spawn(move || {
                            let cursor = Cursor::new(click_sound_data.as_ref().clone());
                            let decoder = rodio::Decoder::new(BufReader::new(cursor)).unwrap();
                            let sink = rodio::Sink::try_new(&stream_handle).unwrap();
                            sink.append(decoder);
                            sink.sleep_until_end();
                        });
                    }
                    if !fast_mode.load(Ordering::SeqCst) {
                        let cps = *target_cps.lock().unwrap();
                        let sleep_duration = Duration::from_secs_f64(1.0 / cps);
                        thread::sleep(sleep_duration);
                    } else {
                        // In fast mode, yield briefly.
                        std::hint::spin_loop();
                    }
                }
            } else {
                thread::sleep(Duration::from_millis(10));
            }
        }
    });

    // Global event listener for toggling, hotkey change, and CPS counting.
    {
        let clicking_for_listener = app_state.clicking.clone();
        let hotkey = app_state.hotkey.clone();
        let changing_hotkey = app_state.changing_hotkey.clone();
        let clicks = app_state.clicks.clone();
        thread::spawn(move || {
            let callback = move |event: Event| {
                match event.event_type {
                    EventType::KeyPress(key) => {
                        if changing_hotkey.load(Ordering::SeqCst) {
                            if let Ok(mut hk) = hotkey.lock() {
                                *hk = key;
                            }
                            changing_hotkey.store(false, Ordering::SeqCst);
                        } else {
                            let current_hotkey = { *hotkey.lock().unwrap() };
                            if key == current_hotkey {
                                let new_state = !clicking_for_listener.load(Ordering::SeqCst);
                                clicking_for_listener.store(new_state, Ordering::SeqCst);
                            }
                        }
                    }
                    EventType::ButtonPress(Button::Left) => {
                        let now = Instant::now();
                        if let Ok(mut clicks) = clicks.lock() {
                            clicks.push(now);
                        }
                    }
                    _ => {}
                }
            };

            if let Err(error) = listen(callback) {
                eprintln!("Error in global listener: {:?}", error);
            }
        });
    }

    // Configure eframe native options.
    let mut native_options = eframe::NativeOptions::default();
    native_options.initial_window_size = Some(egui::vec2(330.0, 370.0));
    
    eframe::run_native("Superspeed Autoclicker", native_options, Box::new(|_cc| Box::new(app_state)));
}
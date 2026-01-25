mod audio;
mod model;
mod transcription;
mod injector;

use anyhow::Result;
use ringbuf::HeapRb;
use std::time::Duration;
use log::{info, error};
mod settings;

use std::sync::{Arc, atomic::{AtomicBool}};
use std::thread;
use tauri::{AppHandle, Emitter, Manager};

use audio::capture::AudioCapture;
use audio::vad::{EnergyVad, VadState};
use model::ModelManager;
use transcription::TranscriptionEngine;
use injector::TextInjector;

const SAMPLE_RATE: u32 = 16000; 
const FRAME_SIZE_MS: u64 = 30;  
const RINGBUF_SIZE: usize = 16000 * 10; 

#[derive(serde::Serialize, Clone)]
struct VadPayload {
    state: String,
    rms: f32,
}

#[derive(serde::Serialize, Clone)]
struct TranscriptionPayload {
    text: String,
}

pub fn start_engine(app: AppHandle) -> Result<()> {
    info!("Starting FlowType Engine...");

    // 1. Prepare Model (Blocking download - might delay startup, ideal to move to async command or separate thread)
    // For now, we do it in a thread to avoid blocking main loop
    thread::spawn(move || {
        if let Err(e) = run_engine_loop(app) {
            error!("Engine crashed: {}", e);
        }
    });

    Ok(())
}

fn run_engine_loop(app: AppHandle) -> Result<()> {
    // Load Settings
    let mgr = settings::SettingsManager::new(&app);
    let saved_settings = mgr.load();
    info!("Loaded Settings: {:?}", saved_settings);

    // 2. Prepare Model
    let model_mgr = ModelManager::new();
    let model_path = model_mgr.get_or_download_model("tiny.en")?; 

    // 3. Setup Channels
    let (tx_audio, rx_audio) = crossbeam_channel::unbounded::<Vec<f32>>();
    let (tx_text, rx_text) = crossbeam_channel::unbounded::<String>();
    
    let running = Arc::new(AtomicBool::new(true));
    let running_clone = running.clone();
    
    let auto_space = Arc::new(AtomicBool::new(saved_settings.auto_space)); 
    let auto_space_clone = auto_space.clone();
    app.manage(auto_space.clone());

    let silence_timeout = Arc::new(std::sync::atomic::AtomicU64::new(saved_settings.silence_timeout)); 
    let silence_timeout_clone = silence_timeout.clone();
    app.manage(silence_timeout.clone());

    let allow_commands = Arc::new(AtomicBool::new(saved_settings.allow_commands)); 
    let allow_commands_clone = allow_commands.clone();
    app.manage(allow_commands.clone());

    // 4. Injector Thread
    let app_handle_inj = app.clone(); // Clone for injector thread
    thread::spawn(move || {
        let injector = match TextInjector::new() {
            Ok(i) => i,
            Err(e) => {
                error!("Failed to init injector: {}", e);
                return;
            }
        };
        while let Ok(mut text) = rx_text.recv() {
            // Check for auto-space
            if auto_space_clone.load(std::sync::atomic::Ordering::Relaxed) {
                text.push(' ');
            }

            // Emit to frontend
            app_handle_inj.emit("transcription", TranscriptionPayload { text: text.clone() }).ok();
            
            // Inject to OS
            let commands_enabled = allow_commands_clone.load(std::sync::atomic::Ordering::Relaxed);
            if let Err(e) = injector.inject(&text, commands_enabled) {
                error!("Injection failed: {}", e);
            }
        }
    });

    // 5. Transcription Thread
    let _app_handle_tx = app.clone();
    thread::spawn(move || {
        let mut engine = match TranscriptionEngine::new(model_path) {
            Ok(e) => e,
            Err(e) => {
                 error!("Failed to init transcription engine: {}", e);
                 return;
            }
        };
        engine.run(rx_audio, tx_text, running_clone);
    });

    // 6. RingBuffer
    let ring = HeapRb::<f32>::new(RINGBUF_SIZE);
    let (producer, mut consumer) = ring.split();

    // 7. Audio Capture & Resampler
    let (_capture, source_rate) = AudioCapture::init(producer)?;
    info!("Audio capture started at {}Hz. Target: {}Hz", source_rate, SAMPLE_RATE);

    let mut resampler = audio::resample::AudioResampler::new(
        source_rate as usize, 
        SAMPLE_RATE as usize, 
        (source_rate as u64 * FRAME_SIZE_MS / 1000) as usize
    )?;

    // 8. VAD (Increased sensitivity: 0.015 -> 0.008)
    let mut vad = EnergyVad::new(0.008, 0.005, 300, 500, FRAME_SIZE_MS);
    let mut current_timeout = saved_settings.silence_timeout; // Use saved timeout

    // 9. Loop
    let chunk_samples = (source_rate as u64 * FRAME_SIZE_MS / 1000) as usize; 
    let mut buffer = Vec::with_capacity(chunk_samples);
    let mut voice_buffer = Vec::<f32>::new();
    
    // Pre-roll buffer (500ms) to catch the start of sentences
    let pre_roll_frames = (0.5 * 1000.0 / FRAME_SIZE_MS as f32) as usize; // ~16 chunks
    let mut pre_roll_buffer = std::collections::VecDeque::<Vec<f32>>::with_capacity(pre_roll_frames);

    let mut last_state = VadState::Silence;

    loop {
        // Update timeout dynamically
        let target_timeout = silence_timeout_clone.load(std::sync::atomic::Ordering::Relaxed);
        if target_timeout != current_timeout {
            current_timeout = target_timeout;
            vad.update_stop_window(current_timeout, FRAME_SIZE_MS);
            info!("⏳ VAD Silence Timeout updated to {}ms", current_timeout);
        }

        std::thread::sleep(Duration::from_millis(FRAME_SIZE_MS));
        buffer.clear();
        let available = consumer.len();
        if available > 0 {
            for _ in 0..available {
                if let Some(s) = consumer.pop() { buffer.push(s); }
            }
        }

        if !buffer.is_empty() {
             let rms = EnergyVad::calculate_rms(&buffer);
             let state = vad.process(rms);

             // Maintain pre-roll buffer during silence
             if matches!(state, VadState::Silence) {
                 if pre_roll_buffer.len() >= pre_roll_frames {
                     pre_roll_buffer.pop_front();
                 }
                 pre_roll_buffer.push_back(buffer.clone());
             }

             // State Transition: Silence -> Speaking
             if matches!(last_state, VadState::Silence) && matches!(state, VadState::Speaking) {
                 info!("🗣️ Speech started! Prepending {}ms of audio", pre_roll_buffer.len() as u64 * FRAME_SIZE_MS);
                 // Dump pre-roll into voice buffer
                 for chunk in pre_roll_buffer.iter() {
                     voice_buffer.extend_from_slice(chunk);
                 }
                 pre_roll_buffer.clear(); // Clear to avoid duplicating if we toggle rapidly
             }

             if matches!(state, VadState::Speaking) {
                 voice_buffer.extend_from_slice(&buffer);
             } 
             
             // State Transition: Speaking -> Silence
             if matches!(last_state, VadState::Speaking) && matches!(state, VadState::Silence) {
                 if !voice_buffer.is_empty() {
                     info!("🗣️ Speech ended. Resampling {} samples...", voice_buffer.len());
                     if let Ok(resampled) = resampler.resample(&voice_buffer) {
                         let rms_resampled = EnergyVad::calculate_rms(&resampled);
                         info!("✅ Resampled to {} samples (RMS: {:.4}). Sending to Whisper...", resampled.len(), rms_resampled);
                         tx_audio.send(resampled).ok();
                     }
                     voice_buffer.clear();
                 }
             }

             if discriminant(&state) != discriminant(&last_state) {
                 let state_str = match state {
                     VadState::Speaking => "speaking",
                     VadState::Silence => "silence",
                 };
                 app.emit("vad-update", VadPayload { state: state_str.to_string(), rms }).ok();
                 last_state = state;
             } 
        }
    }
}

#[tauri::command]
fn minimize_window(window: tauri::Window) {
  window.minimize().unwrap();
}

fn discriminant<T>(v: &T) -> std::mem::Discriminant<T> {
    std::mem::discriminant(v)
}

#[tauri::command]
fn set_auto_space(state: bool, auto_space: tauri::State<'_, Arc<AtomicBool>>, app: tauri::AppHandle) {
    auto_space.store(state, std::sync::atomic::Ordering::Relaxed);
    info!("Auto-space set to: {}", state);
    
    let mgr = settings::SettingsManager::new(&app);
    let mut current = mgr.load();
    current.auto_space = state;
    mgr.save(&current);
}

#[tauri::command]
fn set_silence_timeout(ms: u64, timeout: tauri::State<'_, Arc<std::sync::atomic::AtomicU64>>, app: tauri::AppHandle) {
    timeout.store(ms, std::sync::atomic::Ordering::Relaxed);
    
    let mgr = settings::SettingsManager::new(&app);
    let mut current = mgr.load();
    current.silence_timeout = ms;
    mgr.save(&current);
}

#[tauri::command]
fn set_allow_commands(state: bool, allow_commands: tauri::State<'_, Arc<AtomicBool>>, app: tauri::AppHandle) {
    allow_commands.store(state, std::sync::atomic::Ordering::Relaxed);
    info!("Voice Commands set to: {}", state);

    let mgr = settings::SettingsManager::new(&app);
    let mut current = mgr.load();
    current.allow_commands = state;
    mgr.save(&current);
}

#[tauri::command]
fn get_settings(app: tauri::AppHandle) -> settings::AppSettings {
    let mgr = settings::SettingsManager::new(&app);
    mgr.load()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .invoke_handler(tauri::generate_handler![
        minimize_window, 
        set_auto_space, 
        set_silence_timeout, 
        set_allow_commands,
        get_settings
    ])
    .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, Some(vec!["--minimized"])))
    .plugin(tauri_plugin_log::Builder::default().build())
    .setup(|app| {
        // Start the engine
        let handle = app.handle().clone();
        // Spawning separate thread for engine allows setup to complete
        start_engine(handle)?;
        Ok(())
    })
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}

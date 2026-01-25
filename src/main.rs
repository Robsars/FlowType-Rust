mod audio;
mod model;
mod transcription;
mod injector;

use anyhow::Result;

use ringbuf::HeapRb;
use std::time::{Duration, Instant};
use log::{info, error};
use std::sync::{Arc, atomic::{AtomicBool}};
use std::thread;

use audio::capture::AudioCapture;
use audio::vad::{EnergyVad, VadState};
use model::ModelManager;
use transcription::TranscriptionEngine;
use injector::TextInjector;

// Constants
const SAMPLE_RATE: u32 = 16000; 
const FRAME_SIZE_MS: u64 = 30;  
const RINGBUF_SIZE: usize = 16000 * 10; 

fn main() -> Result<()> {
    // 1. Init Logger
    env_logger::init();
    info!("Starting FlowType (Rust Phase 3)...");

    // 2. Prepare Model (Blocking download)
    let model_mgr = ModelManager::new();
    let model_path = model_mgr.get_or_download_model("tiny.en")?; 

    // 3. Setup Channels & Threading
    // Channel: Main(VAD) -> Transcription
    let (tx_audio, rx_audio) = crossbeam_channel::unbounded::<Vec<f32>>();
    // Channel: Transcription -> Injector
    let (tx_text, rx_text) = crossbeam_channel::unbounded::<String>();
    
    let running = Arc::new(AtomicBool::new(true));
    let running_clone = running.clone();

    // 4. Start Injector Thread (Must handle COM/UI msg pump if needed, but our helper does CoInit)
    thread::spawn(move || {
        let injector = match TextInjector::new() {
            Ok(i) => i,
            Err(e) => {
                error!("Failed to init injector: {}", e);
                return;
            }
        };
        info!("Injector Ready.");
        
        while let Ok(text) = rx_text.recv() {
            if let Err(e) = injector.inject(&text) {
                error!("Injection failed: {}", e);
            }
        }
    });

    // 5. Start Transcription Thread
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

    // 6. Setup Audio RingBuffer
    let ring = HeapRb::<f32>::new(RINGBUF_SIZE);
    let (producer, mut consumer) = ring.split();

    // 7. Start Audio Capture
    let _capture = AudioCapture::init(producer)?;
    info!("Audio capture started. Using 'tiny.en' model.");

    // 8. Init VAD
    let mut vad = EnergyVad::new(0.015, 0.005, 300, 500, FRAME_SIZE_MS);

    // 9. Main Processing Loop
    let chunk_size = (48000 * FRAME_SIZE_MS / 1000) as usize; 
    
    let mut buffer = Vec::with_capacity(chunk_size);
    let mut voice_buffer = Vec::<f32>::new(); // Accumulates while speaking
    let mut last_state = VadState::Silence;

    info!("Listening... (Press Ctrl+C to stop)");

    loop {
        let _start_time = Instant::now();
        std::thread::sleep(Duration::from_millis(FRAME_SIZE_MS));

        // Pull audio from RingBuffer
        buffer.clear();
        let available = consumer.len();
        if available > 0 {
            for _ in 0..available {
                if let Some(s) = consumer.pop() {
                    buffer.push(s);
                }
            }
        }

        if !buffer.is_empty() {
             let rms = EnergyVad::calculate_rms(&buffer);
             let state = vad.process(rms);

             // VAD State Machine
             if matches!(state, VadState::Speaking) {
                 voice_buffer.extend_from_slice(&buffer);
             } 
             
             if matches!(last_state, VadState::Speaking) && matches!(state, VadState::Silence) {
                 info!("🗣️ Speech ended. Sending {} samples to Brain...", voice_buffer.len());
                 if !voice_buffer.is_empty() {
                     tx_audio.send(voice_buffer.clone()).ok();
                     voice_buffer.clear();
                 }
             }

             if discriminant(&state) != discriminant(&last_state) {
                 match state {
                     VadState::Speaking => info!("🗣️  SPEAKING (Energy: {:.4})", rms),
                     VadState::Silence => info!("🤫 SILENCE  (Energy: {:.4})", rms),
                 }
                 last_state = state;
             }
        }
    }
}

fn discriminant<T>(v: &T) -> std::mem::Discriminant<T> {
    std::mem::discriminant(v)
}

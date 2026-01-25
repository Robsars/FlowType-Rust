use anyhow::{Result, Context};
use crossbeam_channel::Receiver;
use log::{info, error};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use whisper_rs::{WhisperContext, FullParams, SamplingStrategy};

pub struct TranscriptionEngine {
    context: WhisperContext,
}

impl TranscriptionEngine {
    pub fn new<P: AsRef<Path>>(model_path: P) -> Result<Self> {
        let context = WhisperContext::new_with_params(
            model_path.as_ref().to_str().unwrap(), 
            whisper_rs::WhisperContextParameters::default()
        ).context("Failed to load Whisper model")?;

        Ok(Self { context })
    }

    /// Run the transcription loop.
    pub fn run(&mut self, rx: Receiver<Vec<f32>>, tx_text: crossbeam_channel::Sender<String>, running: Arc<AtomicBool>) {
        info!("Transcription Engine IDLE. Waiting for audio...");

        let mut state = self.context.create_state().expect("failed to create state");

        while running.load(Ordering::Relaxed) {
            // Block until we get a chunk. 
            // In a real app, we might handle 'is_partial' logic here.
            // For now, assume each chunk is a "phrase" sent by VAD.
            if let Ok(audio_data) = rx.recv() {
                if audio_data.is_empty() { continue; }

                info!("Processing {} samples...", audio_data.len());
                let t0 = std::time::Instant::now();

                // Configure Params
                let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
                params.set_print_progress(false);
                params.set_print_special(false);
                params.set_print_realtime(false);
                params.set_print_timestamps(false); // We just want text
                params.set_language(Some("en"));
                
                // Run Inference
                // Note: full() expects f32, 16kHz
                if let Err(e) = state.full(params, &audio_data[..]) {
                     error!("Whisper inference failed: {}", e);
                     continue;
                }

                // Extract Text
                let num_segments = state.full_n_segments().unwrap_or(0);
                let mut full_text = String::new();
                for i in 0..num_segments {
                    if let Ok(segment) = state.full_get_segment_text(i) {
                         full_text.push_str(&segment);
                    }
                }

                let dt = t0.elapsed();
                let text = full_text.trim().to_string();
                if !text.is_empty() {
                    info!("📝 Text ({:?}): {}", dt, text);
                    tx_text.send(text).ok();
                }
            } else {
                // Channel closed
                break;
            }
        }
        info!("Transcription Engine stopped.");
    }
}

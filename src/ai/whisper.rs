use whisper_rs::{WhisperContext, WhisperParams, FullParams, SamplingStrategy};
use anyhow::{Context, Result};
use std::path::Path;

pub struct WhisperEngine {
    ctx: WhisperContext,
}

impl WhisperEngine {
    pub fn new(model_path: &str) -> Result<Self> {
        if !Path::new(model_path).exists() {
            return Err(anyhow::anyhow!("Whisper model not found at {}", model_path));
        }
        
        let ctx = WhisperContext::new_with_params(model_path, Default::default())
            .context("Failed to load Whisper model")?;
            
        Ok(Self { ctx })
    }

    pub fn transcribe(&self, audio_data: &[f32]) -> Result<String> {
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        
        // Optimize for speed
        params.set_n_threads(4);
        params.set_language(Some("en"));
        params.set_single_segment(true); // Usually true for short dictation
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);

        let mut state = self.ctx.create_state().context("Failed to create state")?;
        state.full(params, audio_data).context("Failed to run transcription")?;

        let num_segments = state.full_n_segments().context("Failed to get segments")?;
        let mut result = String::new();
        for i in 0..num_segments {
            if let Ok(segment) = state.full_get_segment_text(i) {
                result.push_str(&segment);
            }
        }

        Ok(result.trim().to_string())
    }
}

use std::collections::VecDeque;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum VadAction {
    Silence,
    Speaking,
}

#[derive(Debug, Clone, Copy)]
pub enum VadState {
    Silence,
    Speaking,
}

pub struct EnergyVad {
    // Config
    start_threshold: f32,
    stop_threshold: f32,
    start_window_frames: usize,
    stop_window_frames: usize,
    
    // State
    current_state: VadState,
    energy_history: VecDeque<f32>,
}

impl EnergyVad {
    pub fn new(
        start_threshold: f32,
        stop_threshold: f32,
        start_window_ms: u64,
        stop_window_ms: u64,
        frame_rate_ms: u64, // How many ms per processed chunk?
    ) -> Self {
        let start_frames = (start_window_ms / frame_rate_ms).max(1) as usize;
        let stop_frames = (stop_window_ms / frame_rate_ms).max(1) as usize;

        Self {
            start_threshold,
            stop_threshold,
            start_window_frames: start_frames,
            stop_window_frames: stop_frames,
            current_state: VadState::Silence,
            energy_history: VecDeque::with_capacity(std::cmp::max(start_frames, stop_frames)),
        }
    }

    /// Calculates RMS (Root Mean Square) energy of a chunk
    pub fn calculate_rms(samples: &[f32]) -> f32 {
        if samples.is_empty() {
            return 0.0;
        }
        let sum_squares: f32 = samples.iter().map(|s| s * s).sum();
        (sum_squares / samples.len() as f32).sqrt()
    }

    /// Process a chunk of audio and return the current state
    pub fn process(&mut self, rms: f32) -> VadState {
        // Add to history
        if self.energy_history.len() >= self.stop_window_frames.max(self.start_window_frames) {
            self.energy_history.pop_front();
        }
        self.energy_history.push_back(rms);

        match self.current_state {
            VadState::Silence => {
                // To transition to Speaking, we need energy > start_threshold for start_window_frames
                if self.check_window(self.start_threshold, self.start_window_frames, true) {
                    self.current_state = VadState::Speaking;
                }
            }
            VadState::Speaking => {
                // To transition to Silence, we need energy < stop_threshold for stop_window_frames
                if self.check_window(self.stop_threshold, self.stop_window_frames, false) {
                    self.current_state = VadState::Silence;
                }
            }
        }

        self.current_state
    }

    /// Check if the last `window_size` frames are all above (greater=true) or below (greater=false) threshold
    fn check_window(&self, threshold: f32, window_size: usize, greater: bool) -> bool {
        if self.energy_history.len() < window_size {
            return false;
        }
        
        // Check the last `window_size` items
        let start_idx = self.energy_history.len() - window_size;
        for i in 0..window_size {
            let val = self.energy_history[start_idx + i];
            if greater {
                if val <= threshold { return false; }
            } else {
                if val >= threshold { return false; }
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rms_calculation() {
        let samples = vec![1.0, -1.0, 1.0, -1.0];
        let rms = EnergyVad::calculate_rms(&samples);
        assert!((rms - 1.0).abs() < 0.001);

        let silence = vec![0.0; 100];
        assert_eq!(EnergyVad::calculate_rms(&silence), 0.0);
    }

    #[test]
    fn test_state_transitions() {
        // Frame rate = 10ms
        // Start window = 30ms (3 frames)
        // Stop window = 50ms (5 frames)
        let mut vad = EnergyVad::new(0.5, 0.2, 30, 50, 10);
        
        // Initial state
        assert!(matches!(vad.current_state, VadState::Silence));

        // 1. Trigger Talk (Need 3 frames > 0.5)
        vad.process(0.6); // 1
        assert!(matches!(vad.current_state, VadState::Silence));
        vad.process(0.6); // 2
        assert!(matches!(vad.current_state, VadState::Silence));
        let state = vad.process(0.6); // 3 -> Trigger!
        assert!(matches!(state, VadState::Speaking));

        // 2. Sustain Talk
        vad.process(0.4); // > stop_threshold (0.2), so still speaking
        assert!(matches!(vad.current_state, VadState::Speaking));

        // 3. Stop Talk (Need 5 frames < 0.2)
        vad.process(0.1); // 1
        vad.process(0.1); // 2
        vad.process(0.1); // 3
        vad.process(0.1); // 4
        assert!(matches!(vad.current_state, VadState::Speaking)); // Still speaking
        let state = vad.process(0.1); // 5 -> Drop!
        assert!(matches!(state, VadState::Silence));
    }
}

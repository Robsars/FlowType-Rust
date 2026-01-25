pub mod engine;

// We will send audio chunks (f32 vectors) to the engine
pub type AudioChunk = Vec<f32>;

// The logic will reside in engine.rs
pub use engine::TranscriptionEngine;

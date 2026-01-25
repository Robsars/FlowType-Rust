use rubato::{Resampler, FastFixedIn, InterpolationType, InterpolationPoint};
use anyhow::Result;

pub struct AudioResampler {
    resampler: FastFixedIn<f32>,
    input_buffer: Vec<f32>,
}

impl AudioResampler {
    pub fn new(source_rate: usize, target_rate: usize, chunk_size: usize) -> Result<Self> {
        // We use FastFixedIn for low latency and consistent chunk sizes
        let resampler = FastFixedIn::<f32>::new(
            target_rate as f64 / source_rate as f64,
            2.0, // max ratio
            InterpolationType::Linear,
            chunk_size,
            1, // mono
        )?;
        
        Ok(Self {
            resampler,
            input_buffer: Vec::new(),
        })
    }

    pub fn resample(&mut self, input: &[f32]) -> Result<Vec<f32>> {
        // FastFixedIn expects the exact chunk size specified in new()
        // We'll just collect and process
        let mut output = Vec::new();
        let chunk_size = self.resampler.input_frames_next();
        
        self.input_buffer.extend_from_slice(input);
        
        while self.input_buffer.len() >= chunk_size {
            let chunk: Vec<f32> = self.input_buffer.drain(..chunk_size).collect();
            let mut resampled = self.resampler.process(&[chunk], None)?;
            output.append(&mut resampled[0]);
        }
        
        Ok(output)
    }
}

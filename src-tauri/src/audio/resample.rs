use rubato::{Resampler, FastFixedIn, PolynomialDegree};
use anyhow::Result;

pub struct AudioResampler {
    resampler: FastFixedIn<f32>,
}

impl AudioResampler {
    pub fn new(source_rate: usize, target_rate: usize, chunk_size: usize) -> Result<Self> {
        let resampler = FastFixedIn::<f32>::new(
            target_rate as f64 / source_rate as f64,
            2.0, // max ratio
            PolynomialDegree::Linear,
            chunk_size,
            1, // mono
        )?;
        
        Ok(Self {
            resampler,
        })
    }

    pub fn resample(&mut self, input: &[f32]) -> Result<Vec<f32>> {
        let frames_required = self.resampler.input_frames_next();
        let mut output = Vec::with_capacity((input.len() as f64 * 0.34) as usize); // Approx capacity
        
        // Process in chunks of the exact size rubato expects
        for chunk in input.chunks(frames_required) {
             let chunk_vec = if chunk.len() == frames_required {
                 chunk.to_vec()
             } else {
                 // Pad potential last chunk with zeros
                 let mut padded = chunk.to_vec();
                 padded.resize(frames_required, 0.0);
                 padded
             };

             let mut resampled_chunk = self.resampler.process(&[chunk_vec], None)?;
             output.append(&mut resampled_chunk[0]);
        }
        
        Ok(output)
    }
}

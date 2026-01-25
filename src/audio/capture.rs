use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use anyhow::{Context, Result};
use log::{info, error};
use ringbuf::HeapProducer;

pub struct AudioCapture {
    _stream: cpal::Stream,
}

impl AudioCapture {
    pub fn init(mut producer: HeapProducer<f32>) -> Result<Self> {
        let host = cpal::default_host();
        
        // 1. Get Default Input Device
        let device = host.default_input_device()
            .context("No input device found")?;
        
        info!("Input device: {}", device.name().unwrap_or("Unknown".to_string()));

        // 2. Configure Stream
        let config = device.default_input_config()
            .context("Failed to get default input config")?;
            
        info!("Default config: Channels={}, SampleRate={}", config.channels(), config.sample_rate().0);

        // We want to handle errors from the stream
        let err_fn = |err| error!("an error occurred on stream: {}", err);

        // 3. Build Stream based on sample format
        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => device.build_input_stream(
                &config.into(),
                move |data: &[f32], _: &_| write_f32(data, &mut producer),
                err_fn,
                None,
            )?,
            cpal::SampleFormat::I16 => device.build_input_stream(
                &config.into(),
                move |data: &[i16], _: &_| write_i16(data, &mut producer),
                err_fn,
                None,
            )?,
            cpal::SampleFormat::U16 => device.build_input_stream(
                &config.into(),
                move |data: &[u16], _: &_| write_u16(data, &mut producer),
                err_fn,
                None,
            )?,
            sample_format => anyhow::bail!("Unsupported sample format '{:?}'", sample_format),
        };

        // 4. Start Stream
        stream.play()?;

        Ok(AudioCapture { _stream: stream })
    }
}

fn write_f32(input: &[f32], producer: &mut HeapProducer<f32>) {
    for &sample in input {
        if producer.push(sample).is_err() {}
    }
}

fn write_i16(input: &[i16], producer: &mut HeapProducer<f32>) {
    for &sample in input {
        // i16 to f32 standard conversion
        let sample_f32 = sample as f32 / 32768.0;
        if producer.push(sample_f32).is_err() {}
    }
}

fn write_u16(input: &[u16], producer: &mut HeapProducer<f32>) {
    for &sample in input {
        // u16 to f32 standard conversion (0 -> -1.0, 65535 -> 1.0)
        let sample_f32 = (sample as f32 - 32768.0) / 32768.0;
        if producer.push(sample_f32).is_err() {}
    }
}

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use anyhow::{Context, Result};
use log::{info, error};
use ringbuf::HeapProducer;

pub struct AudioCapture {
    _stream: cpal::Stream,
}

impl AudioCapture {
    pub fn init(mut producer: HeapProducer<f32>) -> Result<(Self, u32)> {
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
        let channels = config.channels() as usize;
        let sample_rate = config.sample_rate().0;
        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => device.build_input_stream(
                &config.into(),
                move |data: &[f32], _: &_| write_f32(data, channels, &mut producer),
                err_fn,
                None,
            )?,
            cpal::SampleFormat::I16 => device.build_input_stream(
                &config.into(),
                move |data: &[i16], _: &_| write_i16(data, channels, &mut producer),
                err_fn,
                None,
            )?,
            cpal::SampleFormat::U16 => device.build_input_stream(
                &config.into(),
                move |data: &[u16], _: &_| write_u16(data, channels, &mut producer),
                err_fn,
                None,
            )?,
            sample_format => anyhow::bail!("Unsupported sample format '{:?}'", sample_format),
        };

        stream.play()?;
        Ok((AudioCapture { _stream: stream }, sample_rate))
    }
}

fn write_f32(input: &[f32], channels: usize, producer: &mut HeapProducer<f32>) {
    for frame in input.chunks(channels) {
        let sample = if channels == 2 {
            (frame[0] + frame[1]) / 2.0
        } else {
            frame[0]
        };
        if producer.push(sample).is_err() {}
    }
}

fn write_i16(input: &[i16], channels: usize, producer: &mut HeapProducer<f32>) {
    for frame in input.chunks(channels) {
        let sample = if channels == 2 {
            ((frame[0] as f32 / 32768.0) + (frame[1] as f32 / 32768.0)) / 2.0
        } else {
            frame[0] as f32 / 32768.0
        };
        if producer.push(sample).is_err() {}
    }
}

fn write_u16(input: &[u16], channels: usize, producer: &mut HeapProducer<f32>) {
    for frame in input.chunks(channels) {
        let sample = if channels == 2 {
             let s1 = (frame[0] as f32 - 32768.0) / 32768.0;
             let s2 = (frame[1] as f32 - 32768.0) / 32768.0;
             (s1 + s2) / 2.0
        } else {
            (frame[0] as f32 - 32768.0) / 32768.0
        };
        if producer.push(sample).is_err() {}
    }
}

//! Audio output via cpal. Falls back to a silent real-time thread so the
//! transport still runs when no output device is available.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, Sample, SizedSample};
use krabton_engine::audio_in::InputRing;
use krabton_engine::Engine;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

pub enum AudioOutput {
    Device { _stream: cpal::Stream, name: String },
    Null { stop: Arc<AtomicBool> },
}

impl AudioOutput {
    pub fn description(&self) -> String {
        match self {
            AudioOutput::Device { name, .. } => name.clone(),
            AudioOutput::Null { .. } => "No audio device (silent)".into(),
        }
    }
}

impl Drop for AudioOutput {
    fn drop(&mut self) {
        if let AudioOutput::Null { stop } = self {
            stop.store(true, Ordering::Relaxed);
        }
    }
}

/// Picks the default output device and reports its sample rate.
pub fn probe() -> Result<(cpal::Device, cpal::SupportedStreamConfig), String> {
    let host = cpal::default_host();
    let device = host.default_output_device().ok_or("no default output device")?;
    let config = device.default_output_config().map_err(|e| e.to_string())?;
    Ok((device, config))
}

pub fn start(device: cpal::Device, config: cpal::SupportedStreamConfig, engine: Engine) -> Result<AudioOutput, String> {
    let name = device.name().unwrap_or_else(|_| "Audio device".into());
    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => build::<f32>(&device, &config.into(), engine),
        cpal::SampleFormat::I16 => build::<i16>(&device, &config.into(), engine),
        cpal::SampleFormat::U16 => build::<u16>(&device, &config.into(), engine),
        cpal::SampleFormat::I32 => build::<i32>(&device, &config.into(), engine),
        other => return Err(format!("unsupported sample format {other:?}")),
    }?;
    stream.play().map_err(|e| e.to_string())?;
    Ok(AudioOutput::Device { _stream: stream, name: format!("{name} @ {} Hz", config_rate(&device)) })
}

fn config_rate(device: &cpal::Device) -> u32 {
    device.default_output_config().map(|c| c.sample_rate().0).unwrap_or(0)
}

fn build<T>(device: &cpal::Device, config: &cpal::StreamConfig, mut engine: Engine) -> Result<cpal::Stream, String>
where
    T: SizedSample + FromSample<f32>,
{
    let channels = config.channels as usize;
    let mut scratch: Vec<f32> = vec![0.0; 8192];
    device
        .build_output_stream(
            config,
            move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                let frames = data.len() / channels.max(1);
                if scratch.len() < frames * 2 {
                    scratch.resize(frames * 2, 0.0);
                }
                let buf = &mut scratch[..frames * 2];
                engine.process(buf);
                for (f, frame) in data.chunks_mut(channels).enumerate() {
                    let (l, r) = (buf[f * 2], buf[f * 2 + 1]);
                    for (c, s) in frame.iter_mut().enumerate() {
                        let v = match c {
                            0 => l,
                            1 => r,
                            _ => 0.0,
                        };
                        let v = if channels == 1 { (l + r) * 0.5 } else { v };
                        *s = T::from_sample(v);
                    }
                }
            },
            |err| eprintln!("audio stream error: {err}"),
            None,
        )
        .map_err(|e| e.to_string())
}

/// Runs the engine in real time without a device.
pub fn start_null(mut engine: Engine) -> AudioOutput {
    let stop = Arc::new(AtomicBool::new(false));
    let flag = stop.clone();
    std::thread::Builder::new()
        .name("krabton-null-audio".into())
        .spawn(move || {
            let block = 512usize;
            let period = Duration::from_secs_f64(block as f64 / engine.sample_rate() as f64);
            let mut buf = vec![0.0f32; block * 2];
            let mut next = Instant::now();
            while !flag.load(Ordering::Relaxed) {
                engine.process(&mut buf);
                next += period;
                let now = Instant::now();
                if next > now {
                    std::thread::sleep(next - now);
                } else {
                    next = now;
                }
            }
        })
        .ok();
    AudioOutput::Null { stop }
}

/// The audio interface input, kept open while tracks listen to it.
pub struct AudioInputDevice {
    _stream: cpal::Stream,
    pub name: String,
    pub channels: usize,
}

/// Opens the default input device at the engine's sample rate. The
/// samples go into `InputRing` for the engine to read.
pub fn start_input(sample_rate: u32) -> Result<(AudioInputDevice, Arc<InputRing>), String> {
    let host = cpal::default_host();
    let device = host.default_input_device().ok_or("no audio input device")?;
    let name = device.name().unwrap_or_else(|_| "Audio input".into());
    let mut chosen: Option<cpal::SupportedStreamConfig> = None;
    for range in device.supported_input_configs().map_err(|e| e.to_string())? {
        if range.min_sample_rate().0 <= sample_rate && range.max_sample_rate().0 >= sample_rate {
            let c = range.with_sample_rate(cpal::SampleRate(sample_rate));
            let better = match &chosen {
                None => true,
                Some(old) => c.sample_format() == cpal::SampleFormat::F32 && old.sample_format() != cpal::SampleFormat::F32,
            };
            if better {
                chosen = Some(c);
            }
        }
    }
    let config = chosen.ok_or_else(|| format!("{name} cannot run at {sample_rate} Hz like the output"))?;
    let channels = config.channels() as usize;
    let ring = Arc::new(InputRing::new(16384, channels));
    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => build_input::<f32>(&device, &config.clone().into(), ring.clone()),
        cpal::SampleFormat::I16 => build_input::<i16>(&device, &config.clone().into(), ring.clone()),
        cpal::SampleFormat::U16 => build_input::<u16>(&device, &config.clone().into(), ring.clone()),
        cpal::SampleFormat::I32 => build_input::<i32>(&device, &config.clone().into(), ring.clone()),
        other => return Err(format!("unsupported input sample format {other:?}")),
    }?;
    stream.play().map_err(|e| e.to_string())?;
    Ok((AudioInputDevice { _stream: stream, name, channels }, ring))
}

fn build_input<T>(device: &cpal::Device, config: &cpal::StreamConfig, ring: Arc<InputRing>) -> Result<cpal::Stream, String>
where
    T: SizedSample,
    f32: FromSample<T>,
{
    let channels = (config.channels as usize).max(1);
    // Converted in fixed chunks, so the callback never allocates.
    let mut scratch = [0.0f32; 4096];
    let chunk = (scratch.len() / channels) * channels;
    device
        .build_input_stream(
            config,
            move |data: &[T], _: &cpal::InputCallbackInfo| {
                for part in data.chunks(chunk) {
                    for (d, s) in scratch.iter_mut().zip(part) {
                        *d = f32::from_sample_(*s);
                    }
                    ring.push(&scratch[..part.len()], channels);
                }
            },
            |err| eprintln!("audio input error: {err}"),
            None,
        )
        .map_err(|e| e.to_string())
}

// Silence an unused-import warning on targets where `Sample` is only used via bounds.
#[allow(dead_code)]
fn _assert_sample<T: Sample>() {}

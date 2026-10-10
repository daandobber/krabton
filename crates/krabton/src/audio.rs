//! Audio output via cpal, or on Linux straight to PulseAudio / PipeWire
//! (see `pulse`). Falls back to a silent real-time thread so the transport
//! still runs when no output device is available.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, Sample, SizedSample};
use krabton_engine::audio_in::InputRing;
use krabton_engine::Engine;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

pub enum AudioOutput {
    Device { _stream: cpal::Stream, name: String },
    /// A thread writing to PulseAudio / PipeWire.
    Pulse { stop: Arc<AtomicBool>, name: String },
    Null { stop: Arc<AtomicBool> },
}

impl AudioOutput {
    pub fn description(&self) -> String {
        match self {
            AudioOutput::Device { name, .. } | AudioOutput::Pulse { name, .. } => name.clone(),
            AudioOutput::Null { .. } => "No audio device (silent)".into(),
        }
    }
}

impl Drop for AudioOutput {
    fn drop(&mut self) {
        if let AudioOutput::Null { stop } | AudioOutput::Pulse { stop, .. } = self {
            stop.store(true, Ordering::Relaxed);
        }
    }
}

/// Rate Krabton runs at on PulseAudio / PipeWire.
const PULSE_RATE: u32 = 48000;

/// Plays the engine on a PulseAudio stream from its own thread.
fn start_pulse(mut engine: Engine, mut out: crate::pulse::PulseStream) -> AudioOutput {
    let stop = Arc::new(AtomicBool::new(false));
    let flag = stop.clone();
    let spawned = std::thread::Builder::new().name("krabton-pulse".into()).spawn(move || {
        let mut buf = vec![0.0f32; 256 * 2];
        while !flag.load(Ordering::Relaxed) {
            engine.process(&mut buf);
            if let Err(e) = out.write(&buf) {
                eprintln!("{e}");
                break;
            }
        }
    });
    if let Err(e) = spawned {
        eprintln!("could not start the audio thread: {e}");
    }
    AudioOutput::Pulse { stop, name: format!("{} @ {PULSE_RATE} Hz", crate::pulse::NAME) }
}

/// Whether to try PulseAudio / PipeWire (Linux, with a server running).
fn use_pulse() -> bool {
    cfg!(target_os = "linux") && crate::pulse::server_running()
}

/// Names of the output devices (for the Options menu). On Linux these are
/// ALSA's: `pipewire` and `pulse` go through the sound server.
pub fn output_device_names() -> Vec<String> {
    let host = cpal::default_host();
    let mut names: Vec<String> = host.output_devices().map(|ds| ds.filter_map(|d| d.name().ok()).collect()).unwrap_or_default();
    names.dedup();
    if cfg!(target_os = "linux") {
        names.insert(0, crate::pulse::NAME.to_string());
    }
    names
}

/// Opens an output and starts an engine on it (`make` builds one for a
/// sample rate). Tries `preferred` first; on Linux then PulseAudio /
/// PipeWire directly (when a server runs); then the system default, then,
/// when those fail, the sound server's ALSA devices (`pipewire`, `pulse`)
/// and the rest. Returns the output with its engine handle, or every error.
pub fn open_output<H>(preferred: Option<&str>, mut make: impl FnMut(f32) -> (Engine, H)) -> Result<(AudioOutput, H), String> {
    let host = cpal::default_host();
    let mut errors: Vec<String> = Vec::new();
    let mut tried: Vec<String> = Vec::new();
    let mut attempt = |dev: cpal::Device, errors: &mut Vec<String>, tried: &mut Vec<String>| -> Option<(AudioOutput, H)> {
        let name = dev.name().unwrap_or_else(|_| "?".into());
        if tried.contains(&name) {
            return None;
        }
        tried.push(name.clone());
        let config = match dev.default_output_config() {
            Ok(c) => c,
            Err(e) => {
                errors.push(format!("{name}: {e}"));
                return None;
            }
        };
        let (engine, handle) = make(config.sample_rate().0 as f32);
        match start(dev, config, engine) {
            Ok(out) => Some((out, handle)),
            Err(e) => {
                errors.push(format!("{name}: {e}"));
                None
            }
        }
    };
    let all = || host.output_devices().map(|ds| ds.collect::<Vec<_>>()).unwrap_or_default();
    let want = preferred.filter(|w| !w.is_empty() && *w != crate::pulse::NAME);
    if let Some(want) = want {
        for dev in all().into_iter().filter(|d| d.name().is_ok_and(|n| n == want)) {
            if let Some(ok) = attempt(dev, &mut errors, &mut tried) {
                return Ok(ok);
            }
        }
    }
    if use_pulse() {
        match crate::pulse::PulseStream::open(false, PULSE_RATE, 1024) {
            Ok(stream) => {
                let (engine, handle) = make(PULSE_RATE as f32);
                return Ok((start_pulse(engine, stream), handle));
            }
            Err(e) => errors.push(e),
        }
    }
    if let Some(dev) = host.default_output_device() {
        if let Some(ok) = attempt(dev, &mut errors, &mut tried) {
            return Ok(ok);
        }
    }
    // The default did not open (on Linux often because the sound server
    // holds the card): the server's own devices first, then anything.
    let mut rest = all();
    rest.sort_by_key(|d| {
        let n = d.name().unwrap_or_default();
        match n.as_str() {
            "pipewire" => 0,
            "pulse" => 1,
            "default" => 2,
            _ if n.starts_with("sysdefault") => 3,
            _ if n.starts_with("hw:") => 5,
            _ => 4,
        }
    });
    for dev in rest.into_iter().take(16) {
        if let Some(ok) = attempt(dev, &mut errors, &mut tried) {
            return Ok(ok);
        }
    }
    Err(if errors.is_empty() { "no output devices found".into() } else { errors.join("; ") })
}

/// A hint for when no output opens.
pub fn no_audio_hint() -> &'static str {
    if cfg!(target_os = "linux") {
        "On Linux Krabton plays to PulseAudio / PipeWire (needs libpulse, and pipewire-pulse with PipeWire), or else through ALSA (pipewire-alsa or pulseaudio-alsa). Options > Audio output picks a device."
    } else {
        "Check that an output device is connected and not in exclusive use. Options > Audio output picks a device."
    }
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
    _keep: InputKeep,
    pub name: String,
    pub channels: usize,
}

/// What keeps an input running.
enum InputKeep {
    #[allow(dead_code)]
    Cpal(cpal::Stream),
    /// The reading thread stops when this is set.
    Pulse(Arc<AtomicBool>),
}

impl Drop for InputKeep {
    fn drop(&mut self) {
        if let InputKeep::Pulse(stop) = self {
            stop.store(true, Ordering::Relaxed);
        }
    }
}

/// Records from PulseAudio / PipeWire into an `InputRing` on a thread.
fn start_pulse_input(sample_rate: u32) -> Result<(AudioInputDevice, Arc<InputRing>), String> {
    let mut stream = crate::pulse::PulseStream::open(true, sample_rate, 512)?;
    let ring = Arc::new(InputRing::new(16384, 2));
    let stop = Arc::new(AtomicBool::new(false));
    let (flag, r) = (stop.clone(), ring.clone());
    std::thread::Builder::new()
        .name("krabton-pulse-in".into())
        .spawn(move || {
            let mut buf = vec![0.0f32; 256 * 2];
            while !flag.load(Ordering::Relaxed) {
                if let Err(e) = stream.read(&mut buf) {
                    eprintln!("{e}");
                    break;
                }
                r.push(&buf, 2);
            }
        })
        .map_err(|e| e.to_string())?;
    Ok((AudioInputDevice { _keep: InputKeep::Pulse(stop), name: crate::pulse::NAME.into(), channels: 2 }, ring))
}

/// Opens the default input device at the engine's sample rate. The
/// samples go into `InputRing` for the engine to read.
pub fn start_input(sample_rate: u32) -> Result<(AudioInputDevice, Arc<InputRing>), String> {
    if use_pulse() {
        match start_pulse_input(sample_rate) {
            Ok(ok) => return Ok(ok),
            Err(e) => eprintln!("{e}"),
        }
    }
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
    Ok((AudioInputDevice { _keep: InputKeep::Cpal(stream), name, channels }, ring))
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

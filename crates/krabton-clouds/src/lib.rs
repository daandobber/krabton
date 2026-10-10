//! A Rust port of the DSP code of the Mutable Instruments Clouds granular
//! audio processor, by Emilie Gillet (MIT license, see the headers of each
//! module and <https://github.com/pichenettes/eurorack>).
//!
//! Clouds records its input into a buffer of about one to eight seconds
//! (depending on the quality setting) and plays it back in one of four
//! ways: overlapping grains, a WSOLA pitch-shifter/time-stretcher, a
//! looping delay, or a phase vocoder. A diffuser, pitch shifter, filters,
//! feedback and a reverb follow, then the dry/wet mix.
//!
//! The module runs at 32 kHz in blocks of 32 frames. [`Clouds`] wraps the
//! processor for a host: it takes stereo audio at any rate and block length,
//! converts it to and from 32 kHz with windowed-sinc resamplers, and
//! smooths the parameters like the module's knob scanning does.

#![allow(clippy::too_many_arguments, clippy::needless_range_loop, clippy::new_without_default)]

pub mod audio_buffer;
pub mod correlator;
pub mod dsp;
pub mod fx;
pub mod granular;
pub mod looping;
pub mod parameters;
pub mod processor;
pub mod pvoc;
pub mod resampler;
pub mod resources;
pub mod src_2x;
pub mod wsola;

use processor::GranularProcessor;
use resampler::Resampler;

/// The rate the module runs at.
pub const SAMPLE_RATE: f32 = 32000.0;
/// Frames per block.
pub const MAX_BLOCK_SIZE: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlaybackMode {
    Granular,
    Stretch,
    LoopingDelay,
    Spectral,
}

#[derive(Clone, Copy, Debug)]
pub struct Params {
    pub mode: PlaybackMode,
    /// 0: 16-bit stereo 32 kHz (1 s), 1: 16-bit mono (1.9 s), 2: 8-bit
    /// mu-law stereo 16 kHz (4 s), 3: 8-bit mu-law mono 16 kHz (7.4 s).
    pub quality: u8,
    pub position: f32,
    pub size: f32,
    /// Semitones, -48..48.
    pub pitch: f32,
    pub density: f32,
    pub texture: f32,
    pub dry_wet: f32,
    pub stereo_spread: f32,
    pub feedback: f32,
    pub reverb: f32,
    /// Level of the FREEZE gate: a rising edge freezes, a falling edge
    /// releases.
    pub freeze: bool,
    /// Level of the TRIG input: a rising edge triggers.
    pub trigger: bool,
}

impl Default for Params {
    /// Knobs at noon, the factory blend settings (stereo spread at half,
    /// no feedback or reverb), except for a little more DENSITY so the
    /// granular mode makes sound, and an even dry/wet mix.
    fn default() -> Self {
        Self {
            mode: PlaybackMode::Granular,
            quality: 0,
            position: 0.5,
            size: 0.5,
            pitch: 0.0,
            density: 0.7,
            texture: 0.5,
            dry_wet: 0.5,
            stereo_spread: 0.5,
            feedback: 0.0,
            reverb: 0.0,
            freeze: false,
            trigger: false,
        }
    }
}

/// Zeros prefilled in the output queue: covers the block of 32 frames the
/// module needs before it produces anything.
const PREFILL: usize = 34;
const FIFO_SIZE: usize = 128;

/// The module's output stage halves the signal (`SoftConvert`), and the
/// dry/wet crossfade is equal-power: this brings the dry signal at
/// dry/wet = 0 back to unity.
fn output_gain() -> f32 {
    2.0 / resources::LUT_XFADE_OUT[0]
}

/// Parameter values after the smoothing of the module's CV scaler.
#[derive(Clone, Copy, Debug, Default)]
struct Smoothed {
    position: f32,
    size: f32,
    pitch: f32,
    density: f32,
    texture: f32,
    dry_wet: f32,
    stereo_spread: f32,
    feedback: f32,
    reverb: f32,
}

#[inline]
fn unit(x: f32) -> f32 {
    // `max`/`min` drop NaN; the result is 0 for NaN.
    x.max(0.0).min(1.0)
}

pub struct Clouds {
    processor: Box<GranularProcessor>,
    down: Resampler,
    up: Resampler,
    host_rate: u32,
    in_block: [[i16; 2]; MAX_BLOCK_SIZE],
    in_count: usize,
    fifo: [[f32; 2]; FIFO_SIZE],
    fifo_read: usize,
    fifo_len: usize,
    smoothed: Smoothed,
    smoothed_valid: bool,
    trigger_level: bool,
    pending_trigger: bool,
    freeze_level: bool,
    pending_freeze: Option<bool>,
    output_gain: f32,
    underruns: u64,
}

impl Clouds {
    pub fn new(host_sample_rate: f32) -> Self {
        let host_rate =
            if host_sample_rate.is_finite() { host_sample_rate.round().clamp(8000.0, 768000.0) as u32 } else { 48000 };
        let mut c = Self {
            processor: Box::new(GranularProcessor::new()),
            down: Resampler::new(host_rate, SAMPLE_RATE as u32),
            up: Resampler::new(SAMPLE_RATE as u32, host_rate),
            host_rate,
            in_block: [[0; 2]; MAX_BLOCK_SIZE],
            in_count: 0,
            fifo: [[0.0; 2]; FIFO_SIZE],
            fifo_read: 0,
            fifo_len: PREFILL,
            smoothed: Smoothed::default(),
            smoothed_valid: false,
            trigger_level: false,
            pending_trigger: false,
            freeze_level: false,
            pending_freeze: None,
            output_gain: output_gain(),
            underruns: 0,
        };
        c.processor.set_quality(0);
        c.processor.prepare();
        c
    }

    /// Delay of the dry signal through the converters and the block
    /// buffering, in host samples (rounded).
    pub fn latency(&self) -> usize {
        let h = self.host_rate as f64;
        (resampler::HALF as f64 + (resampler::HALF + 1 + PREFILL) as f64 * h / SAMPLE_RATE as f64).round() as usize
    }

    /// Processes `left` and `right` in place (the common length of both).
    pub fn process(&mut self, p: &Params, left: &mut [f32], right: &mut [f32]) {
        if p.trigger && !self.trigger_level {
            self.pending_trigger = true;
        }
        self.trigger_level = p.trigger;
        if p.freeze != self.freeze_level {
            self.pending_freeze = Some(p.freeze);
            self.freeze_level = p.freeze;
        }

        for (l, r) in left.iter_mut().zip(right.iter_mut()) {
            // To 32 kHz.
            self.down.push([*l, *r]);
            while let Some(y) = self.down.next_output() {
                self.in_block[self.in_count] = [to_codec(y[0]), to_codec(y[1])];
                self.in_count += 1;
                if self.in_count == MAX_BLOCK_SIZE {
                    self.in_count = 0;
                    self.run_block(p);
                }
            }

            // Back to the host rate.
            let (y, needed) = self.up.output_and_advance();
            for _ in 0..needed {
                let x = if self.fifo_len > 0 {
                    let x = self.fifo[self.fifo_read];
                    self.fifo_read = (self.fifo_read + 1) % FIFO_SIZE;
                    self.fifo_len -= 1;
                    x
                } else {
                    self.underruns += 1;
                    [0.0; 2]
                };
                self.up.push(x);
            }
            *l = y[0];
            *r = y[1];
        }
    }

    fn update_parameters(&mut self, p: &Params) {
        let target = Smoothed {
            position: unit(p.position),
            size: unit(p.size),
            pitch: if p.pitch.is_nan() { 0.0 } else { p.pitch.clamp(-48.0, 48.0) },
            density: unit(p.density),
            texture: unit(p.texture),
            dry_wet: unit(p.dry_wet),
            stereo_spread: unit(p.stereo_spread),
            feedback: unit(p.feedback),
            reverb: unit(p.reverb),
        };
        let s = &mut self.smoothed;
        if !self.smoothed_valid {
            *s = target;
            self.smoothed_valid = true;
        } else {
            // The filter coefficients of the module's knobs (cv_scaler.cc),
            // and the jump-or-glide of its V/OCT input for the pitch.
            dsp::one_pole(&mut s.position, target.position, 0.05);
            dsp::one_pole(&mut s.size, target.size, 0.01);
            dsp::one_pole(&mut s.density, target.density, 0.01);
            dsp::one_pole(&mut s.texture, target.texture, 0.01);
            dsp::one_pole(&mut s.dry_wet, target.dry_wet, 0.05);
            dsp::one_pole(&mut s.stereo_spread, target.stereo_spread, 0.05);
            dsp::one_pole(&mut s.feedback, target.feedback, 0.05);
            dsp::one_pole(&mut s.reverb, target.reverb, 0.05);
            if (target.pitch - s.pitch).abs() > 0.5 {
                s.pitch = target.pitch;
            } else {
                dsp::one_pole(&mut s.pitch, target.pitch, 0.2);
            }
        }

        let proc = &mut self.processor;
        proc.set_playback_mode(p.mode);
        proc.set_quality(p.quality.min(3));
        let params = &mut proc.parameters;
        params.position = s.position;
        params.size = s.size;
        params.pitch = s.pitch;
        params.density = s.density;
        params.texture = s.texture;
        params.dry_wet = s.dry_wet;
        params.stereo_spread = s.stereo_spread;
        params.feedback = s.feedback;
        params.reverb = s.reverb;
        if let Some(freeze) = self.pending_freeze.take() {
            params.freeze = freeze;
        }
        params.trigger = std::mem::take(&mut self.pending_trigger);
        params.gate = self.trigger_level;
    }

    fn run_block(&mut self, p: &Params) {
        self.update_parameters(p);
        self.processor.prepare();
        let mut out = [[0i16; 2]; MAX_BLOCK_SIZE];
        self.processor.process(&self.in_block, &mut out);
        self.processor.parameters.trigger = false;
        for frame in out {
            if self.fifo_len < FIFO_SIZE {
                let w = (self.fifo_read + self.fifo_len) % FIFO_SIZE;
                self.fifo[w] = [
                    frame[0] as f32 / 32768.0 * self.output_gain,
                    frame[1] as f32 / 32768.0 * self.output_gain,
                ];
                self.fifo_len += 1;
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn underruns(&self) -> u64 {
        self.underruns
    }

    #[cfg(test)]
    pub(crate) fn fifo_len(&self) -> usize {
        self.fifo_len
    }
}

/// The codec: host level 1.0 is full scale, clipped to 16 bits.
#[inline]
fn to_codec(x: f32) -> i16 {
    dsp::clip16((x * 32768.0) as i32) as i16
}

#[cfg(test)]
mod tests;

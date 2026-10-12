//! A Rust port of the DSP code of the Mutable Instruments Beads texture
//! synthesizer, by Emilie Gillet (MIT license, see the headers of each
//! module and <https://github.com/pichenettes/eurorack/tree/master/beads>).
//!
//! Beads records its input and plays it back as grains: up to 32 at once,
//! each from its own place in the recording (TIME), as long as SIZE says
//! (the left third of SIZE plays them backwards), shaped by SHAPE, at
//! PITCH, seeded regularly or randomly as DENSITY says, or by the SEED
//! button or a clock. The attenuverters randomize each grain's TIME, SIZE,
//! SHAPE and PITCH. With SIZE fully up it turns into a delay. FEEDBACK,
//! DRY/WET and REVERB follow; four quality settings colour the recording,
//! from clean 16-bit to an 8-bit µ-law cassette with wow and flutter. With
//! no input it plays its wavetables instead.
//!
//! [`Beads`] wraps it for a host: stereo audio at any rate and block length,
//! converted to the module's own rate and back, the knobs smoothed as its
//! pots are.

#![allow(clippy::too_many_arguments, clippy::needless_range_loop, clippy::new_without_default)]

pub mod delay;
pub mod delay_line;
pub mod fx;
pub mod grain;
pub mod player;
pub mod processor;
pub mod random;
pub mod synth;
pub mod trigger;
pub mod util;

use krabton_clouds::resampler::{self, Resampler};
pub use grain::GrainView;
pub use processor::Media;
use processor::{MixParameters, Processor};
use random::{ModulatedParameter, Parameters, RandomizedParameter};
use synth::{Synth, SYNTH_RATE};
use trigger::SeedButtonState;
use util::{extract_gate_flags, one_pole, soft_clip, GateFlags, GATE_FLAG_LOW};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    /// Grains of what comes in.
    Input,
    /// Nothing comes in: the wavetable synth.
    Wavetables,
}

/// The SEED button.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Seed {
    /// Latched: grains keep coming, as DENSITY says.
    Latched,
    /// Held (true) or not: grains only while held.
    Button(bool),
}

/// A clock on the SEED input: the song's beats.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Clock {
    /// The beat at the start of this block.
    pub beat: f64,
    /// Beats per host sample.
    pub beats_per_sample: f64,
    /// Beats per pulse (0.25: sixteenths).
    pub division: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct Params {
    pub time: f32,
    pub time_amount: f32,
    pub size: f32,
    pub size_amount: f32,
    pub shape: f32,
    pub shape_amount: f32,
    /// Semitones.
    pub pitch: f32,
    pub pitch_amount: f32,
    pub density: f32,
    pub feedback: f32,
    pub dry_wet: f32,
    pub reverb: f32,
    pub freeze: bool,
    pub seed: Seed,
    pub clock: Option<Clock>,
    pub quality: Media,
    pub mono: bool,
    pub source: Source,
    /// Linear gain on the input.
    pub input_gain: f32,
}

impl Default for Params {
    /// Knobs at noon, a little to the right on DENSITY (random grains),
    /// half dry, a little reverb, latched, Hi-Fi.
    fn default() -> Self {
        Self {
            time: 0.5,
            time_amount: 0.0,
            size: 0.5,
            size_amount: 0.0,
            shape: 0.5,
            shape_amount: 0.0,
            pitch: 0.0,
            pitch_amount: 0.0,
            density: 0.7,
            feedback: 0.0,
            dry_wet: 0.5,
            reverb: 0.2,
            freeze: false,
            seed: Seed::Latched,
            clock: None,
            quality: Media::Digital,
            mono: false,
            source: Source::Input,
            input_gain: 1.0,
        }
    }
}

/// What a display can show.
#[derive(Clone, Copy, Debug, Default)]
pub struct Status {
    /// How full the recording is.
    pub filled: f32,
    /// SIZE fully up: a delay.
    pub delay_mode: bool,
    /// The grains' gate (the light of SEED).
    pub gate: bool,
    /// The wavetable bank, in synth mode.
    pub bank: usize,
}

const FIFO_SIZE: usize = 256;
const RATES: [u32; 3] = [24000, 32000, 48000];

#[derive(Clone, Copy, Debug, Default)]
struct Knobs {
    time: f32,
    time_amount: f32,
    size: f32,
    size_amount: f32,
    shape: f32,
    shape_amount: f32,
    pitch: f32,
    pitch_amount: f32,
    density: f32,
    feedback: f32,
    dry_wet: f32,
    reverb: f32,
}

pub struct Beads {
    host_rate: u32,
    processor: Box<Processor>,
    synth: Box<Synth>,
    media: Media,
    mono: bool,
    source: Source,
    /// Converters to and from each of the module's rates.
    down: Vec<Resampler>,
    up: Vec<Resampler>,
    rate_index: usize,
    block: Vec<[f32; 2]>,
    block_size: usize,
    out_block: Vec<[f32; 2]>,
    fifo: Vec<[f32; 2]>,
    fifo_read: usize,
    fifo_len: usize,
    knobs: Knobs,
    knobs_valid: bool,
    clock_beat: f64,
    seed_previous: GateFlags,
    gates: Vec<GateFlags>,
    freeze: Vec<bool>,
    status: Status,
    views: Vec<GrainView>,
}

fn rate_index(rate: f32) -> usize {
    RATES.iter().position(|&r| r as f32 == rate).unwrap_or(2)
}

impl Beads {
    pub fn new(host_sample_rate: f32) -> Self {
        let host_rate = if host_sample_rate.is_finite() { host_sample_rate.round().clamp(8000.0, 768000.0) as u32 } else { 48000 };
        let mut b = Self {
            host_rate,
            processor: Box::new(Processor::new()),
            synth: Box::new(Synth::new()),
            media: Media::Digital,
            mono: false,
            source: Source::Input,
            down: RATES.iter().map(|&r| Resampler::new(host_rate, r)).collect(),
            up: RATES.iter().map(|&r| Resampler::new(r, host_rate)).collect(),
            rate_index: 2,
            block: Vec::with_capacity(64),
            block_size: 12,
            out_block: vec![[0.0; 2]; 64],
            fifo: vec![[0.0; 2]; FIFO_SIZE],
            fifo_read: 0,
            fifo_len: 0,
            knobs: Knobs::default(),
            knobs_valid: false,
            clock_beat: 0.0,
            seed_previous: GATE_FLAG_LOW,
            gates: vec![GATE_FLAG_LOW; 64],
            freeze: vec![false; 64],
            status: Status::default(),
            views: Vec::with_capacity(64),
        };
        b.reconfigure(Media::Digital, false, Source::Input);
        b
    }

    fn module_rate(&self) -> f32 {
        if self.source == Source::Wavetables {
            SYNTH_RATE
        } else {
            self.media.sample_rate()
        }
    }

    fn reconfigure(&mut self, media: Media, mono: bool, source: Source) {
        if source == Source::Input && (media != self.media || mono != self.mono || !self.knobs_valid) {
            self.processor.configure(media, mono);
        }
        self.media = media;
        self.mono = mono;
        self.source = source;
        let rate = self.module_rate();
        self.rate_index = rate_index(rate);
        self.block_size = (rate / 4000.0) as usize;
        self.block.clear();
        // Prefill a block so the output never waits for one.
        self.fifo_read = 0;
        self.fifo_len = self.block_size + 2;
        self.fifo.fill([0.0; 2]);
    }

    /// Delay of the dry signal through the converters and the blocks, in
    /// host samples (rounded).
    pub fn latency(&self) -> usize {
        let h = self.host_rate as f64;
        let rate = self.module_rate() as f64;
        (resampler::HALF as f64 + (resampler::HALF + 1 + self.block_size + 2) as f64 * h / rate).round() as usize
    }

    pub fn status(&self) -> Status {
        self.status
    }

    /// The loudest grains now (up to `out.len()`), loudest first; returns
    /// how many there are.
    pub fn grains(&mut self, out: &mut [GrainView]) -> usize {
        self.views.clear();
        match self.source {
            Source::Input => self.processor.grains(&mut self.views),
            Source::Wavetables => self.synth.grains(&mut self.views),
        }
        self.views.sort_by(|a, b| b.level.total_cmp(&a.level));
        let n = self.views.len().min(out.len());
        out[..n].copy_from_slice(&self.views[..n]);
        n
    }

    /// Processes `left` and `right` in place (the common length of both).
    pub fn process(&mut self, p: &Params, left: &mut [f32], right: &mut [f32]) {
        if p.quality != self.media || p.mono != self.mono || p.source != self.source {
            self.reconfigure(p.quality, p.mono, p.source);
        }
        if let Some(c) = p.clock {
            // Follow the song's position, jumping when it jumps.
            if (c.beat - self.clock_beat).abs() > 0.25 {
                self.clock_beat = c.beat;
            } else {
                self.clock_beat += (c.beat - self.clock_beat) * 0.1;
            }
        }
        let ri = self.rate_index;
        // Into the module: the input stage of the tape settings saturates.
        let clip_amount = match self.media {
            Media::DirtyTape => 1.0,
            Media::CleanTape => 0.5,
            _ => 0.0,
        };
        for (l, r) in left.iter_mut().zip(right.iter_mut()) {
            let mut x = [*l * p.input_gain, *r * p.input_gain];
            for v in x.iter_mut() {
                let bias = 0.05;
                let soft = (1.0 - bias) * soft_clip(*v + bias) - bias;
                let hard = v.clamp(-1.0, 1.0);
                *v = hard + (soft - hard) * clip_amount;
            }
            self.down[ri].push(x);
            while let Some(y) = self.down[ri].next_output() {
                self.block.push(y);
                if self.block.len() == self.block_size {
                    self.run_block(p);
                    self.block.clear();
                }
            }
            let (y, needed) = self.up[ri].output_and_advance();
            for _ in 0..needed {
                let x = if self.fifo_len > 0 {
                    let x = self.fifo[self.fifo_read];
                    self.fifo_read = (self.fifo_read + 1) % FIFO_SIZE;
                    self.fifo_len -= 1;
                    x
                } else {
                    [0.0; 2]
                };
                self.up[ri].push(x);
            }
            *l = y[0];
            *r = y[1];
        }
    }

    fn smooth_knobs(&mut self, p: &Params) {
        let unit = |x: f32| x.max(0.0).min(1.0);
        let bi = |x: f32| x.max(-1.0).min(1.0);
        let target = Knobs {
            time: unit(p.time),
            time_amount: bi(p.time_amount),
            size: unit(p.size),
            size_amount: bi(p.size_amount),
            shape: unit(p.shape),
            shape_amount: bi(p.shape_amount),
            pitch: if p.pitch.is_finite() { p.pitch.clamp(-48.0, 48.0) } else { 0.0 },
            pitch_amount: bi(p.pitch_amount),
            density: unit(p.density),
            feedback: unit(p.feedback),
            dry_wet: unit(p.dry_wet),
            reverb: unit(p.reverb),
        };
        if !self.knobs_valid {
            self.knobs = target;
            self.knobs_valid = true;
            return;
        }
        // The pots' low-pass, once per block.
        let k = &mut self.knobs;
        for (s, t) in [
            (&mut k.time, target.time),
            (&mut k.time_amount, target.time_amount),
            (&mut k.size, target.size),
            (&mut k.size_amount, target.size_amount),
            (&mut k.shape, target.shape),
            (&mut k.shape_amount, target.shape_amount),
            (&mut k.pitch, target.pitch),
            (&mut k.pitch_amount, target.pitch_amount),
            (&mut k.density, target.density),
            (&mut k.feedback, target.feedback),
            (&mut k.dry_wet, target.dry_wet),
            (&mut k.reverb, target.reverb),
        ] {
            one_pole(s, t, 0.05);
        }
    }

    fn run_block(&mut self, p: &Params) {
        self.smooth_knobs(p);
        let n = self.block_size;
        let rate = self.module_rate() as f64;
        // The clock on SEED, sample by sample.
        let seed_input = if let Some(c) = p.clock {
            let step = c.beats_per_sample * self.host_rate as f64 / rate;
            let division = c.division.max(1e-3);
            for i in 0..n {
                let high = (self.clock_beat / division).rem_euclid(1.0) < 0.25;
                self.seed_previous = extract_gate_flags(self.seed_previous, high);
                self.gates[i] = self.seed_previous;
                self.clock_beat += step;
            }
            Some(&self.gates[..n])
        } else {
            None
        };
        let button = match p.seed {
            Seed::Latched => SeedButtonState::Locked,
            Seed::Button(true) => SeedButtonState::On,
            Seed::Button(false) => SeedButtonState::Off,
        };
        self.freeze[..n].fill(p.freeze);
        let k = self.knobs;
        let rp = |base: f32, amount: f32| RandomizedParameter { base, attenuverter: amount, cv: 0.0, has_cv: false };
        let params = Parameters { time: rp(k.time, k.time_amount), size: rp(k.size, k.size_amount), shape: rp(k.shape, k.shape_amount), pitch: rp(k.pitch, k.pitch_amount), monophonic_mode: false };
        let density = ModulatedParameter { base: k.density, cv: 0.0 };
        let mix = MixParameters { feedback: k.feedback, dry_wet: k.dry_wet, reverb_amount: k.reverb };
        let out = &mut self.out_block[..n];
        match self.source {
            Source::Input => {
                self.processor.process(button, seed_input, &self.freeze[..n], &density, &params, &mix, &self.block[..n], out);
                self.status = Status { filled: self.processor.filled(), delay_mode: self.processor.delay_mode(), gate: self.processor.gate(), bank: 0 };
            }
            Source::Wavetables => {
                let quality = self.media as usize;
                self.synth.render(button, seed_input, &self.freeze[..n], quality, &density, &params, &mix, out);
                self.status = Status { filled: 1.0, delay_mode: false, gate: false, bank: self.synth.bank() };
            }
        }
        // The output stage: dry at unity (the module's dry gain is 0.75).
        let gain = if self.source == Source::Input { 1.0 / 0.75 } else { 1.0 / 0.5 };
        for frame in out.iter() {
            if self.fifo_len < FIFO_SIZE {
                let w = (self.fifo_read + self.fifo_len) % FIFO_SIZE;
                self.fifo[w] = [frame[0].clamp(-1.0, 1.0) * gain, frame[1].clamp(-1.0, 1.0) * gain];
                self.fifo_len += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests;

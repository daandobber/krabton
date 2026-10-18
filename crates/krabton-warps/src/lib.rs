//! A Rust port of the DSP of Mutable Instruments Warps, the meta-modulator,
//! for Krabton: the input (the modulator) against Warps' internal
//! oscillator (the carrier) through one of its cross-modulation algorithms
//! (crossfade, wavefolding, analog-style diode ring modulation, digital
//! ring modulation, XOR, comparator, each blended into the next) or its
//! twenty-band vocoder with formant shifting.
//!
//! The module runs at 96 kHz; this crate converts the host's rate to it and
//! back with windowed-sinc resamplers (as `krabton-clouds` does for
//! Clouds) and feeds the modulator blocks of 48 samples.
//!
//! Original code: Copyright 2014 Emilie Gillet (Mutable Instruments), MIT
//! license:
//!
//! > Permission is hereby granted, free of charge, to any person obtaining a
//! > copy of this software and associated documentation files (the
//! > "Software"), to deal in the Software without restriction, including
//! > without limitation the rights to use, copy, modify, merge, publish,
//! > distribute, sublicense, and/or sell copies of the Software, and to permit
//! > persons to whom the Software is furnished to do so, subject to the
//! > following conditions:
//! >
//! > The above copyright notice and this permission notice shall be included
//! > in all copies or substantial portions of the Software.
//! >
//! > THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS
//! > OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
//! > MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN
//! > NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM,
//! > DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR
//! > OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE
//! > USE OR OTHER DEALINGS IN THE SOFTWARE.
//!
//! The Rust port is part of Krabton and also MIT licensed.

#![allow(clippy::needless_range_loop)]

pub mod dsp;
pub mod modulator;
pub mod oscillator;
pub mod resampler;
pub mod resources;
pub mod vocoder;

use modulator::Modulator;
use resampler::Resampler;

/// The module's sample rate.
pub const SAMPLE_RATE: f32 = 96000.0;
/// Samples (at 96 kHz) per block for the modulator: a multiple of 12, as the vocoder's bank needs.
const BLOCK: usize = 48;
/// Output samples queued before the first block is out (so the resampler never waits).
const PREFILL: usize = BLOCK + 8;
const FIFO: usize = 4 * BLOCK + 64;

/// The controls, as the module's knobs (0..1) but the carrier's note and shape.
#[derive(Clone, Copy, Debug)]
pub struct Params {
    /// The algorithm knob: crossfade, fold, analog ring, digital ring, XOR,
    /// comparator, then (past about 0.7) the vocoder, its release longer to the end.
    pub algorithm: f32,
    /// Timbre: each algorithm's parameter; the vocoder's formant shift.
    pub timbre: f32,
    /// The input's level and drive (Level 2).
    pub drive: f32,
    /// The internal oscillator's pitch (MIDI note).
    pub note: f32,
    /// 1 sine, 2 triangle, 3 saw (for the vocoder: saw, pulse, noise).
    pub shape: i32,
}

impl Default for Params {
    fn default() -> Self {
        Self { algorithm: 0.25, timbre: 0.5, drive: 0.6, note: 48.0, shape: 1 }
    }
}

pub struct Warps {
    modulator: Box<Modulator>,
    down: Resampler,
    up: Resampler,
    host_rate: u32,
    in_block: [f32; BLOCK],
    out_block: [f32; BLOCK],
    in_count: usize,
    fifo: [f32; FIFO],
    fifo_read: usize,
    fifo_len: usize,
}

fn unit(x: f32) -> f32 {
    if x.is_finite() {
        x.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

impl Warps {
    pub fn new(host_sample_rate: f32) -> Self {
        let host_rate = if host_sample_rate.is_finite() { host_sample_rate.round().clamp(8000.0, 768000.0) as u32 } else { 48000 };
        Self {
            modulator: Box::new(Modulator::new(SAMPLE_RATE)),
            down: Resampler::new(host_rate, SAMPLE_RATE as u32),
            up: Resampler::new(SAMPLE_RATE as u32, host_rate),
            host_rate,
            in_block: [0.0; BLOCK],
            out_block: [0.0; BLOCK],
            in_count: 0,
            fifo: [0.0; FIFO],
            fifo_read: 0,
            fifo_len: PREFILL,
        }
    }

    /// The delay of the processed signal, in host samples (rounded).
    pub fn latency(&self) -> usize {
        let h = self.host_rate as f64;
        (resampler::HALF as f64 + (resampler::HALF + 1 + PREFILL) as f64 * h / SAMPLE_RATE as f64).round() as usize
    }

    /// Takes `input` (the modulator, mono) and writes the processed signal to `out`.
    pub fn process(&mut self, p: &Params, input: &[f32], out: &mut [f32]) {
        let mp = &mut self.modulator.parameters;
        mp.modulation_algorithm = unit(p.algorithm);
        mp.modulation_parameter = unit(p.timbre);
        mp.channel_drive = [0.0, unit(p.drive)];
        mp.note = if p.note.is_finite() { p.note.clamp(0.0, 127.0) } else { 48.0 };
        mp.carrier_shape = p.shape.clamp(1, 3);
        for (x, o) in input.iter().zip(out.iter_mut()) {
            // To 96 kHz.
            let x = if x.is_finite() { x.clamp(-4.0, 4.0) } else { 0.0 };
            self.down.push([x, 0.0]);
            while let Some(y) = self.down.next_output() {
                self.in_block[self.in_count] = y[0];
                self.in_count += 1;
                if self.in_count == BLOCK {
                    self.in_count = 0;
                    self.modulator.process(&self.in_block, &mut self.out_block);
                    for &v in self.out_block.iter() {
                        if self.fifo_len < FIFO {
                            let w = (self.fifo_read + self.fifo_len) % FIFO;
                            self.fifo[w] = v;
                            self.fifo_len += 1;
                        }
                    }
                }
            }
            // Back to the host rate.
            let (y, needed) = self.up.output_and_advance();
            for _ in 0..needed {
                let v = if self.fifo_len > 0 {
                    let v = self.fifo[self.fifo_read];
                    self.fifo_read = (self.fifo_read + 1) % FIFO;
                    self.fifo_len -= 1;
                    v
                } else {
                    0.0
                };
                self.up.push([v, 0.0]);
            }
            *o = y[0];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rms(x: &[f32]) -> f32 {
        (x.iter().map(|v| v * v).sum::<f32>() / x.len().max(1) as f32).sqrt()
    }

    fn run(p: &Params, input: &[f32]) -> Vec<f32> {
        let mut w = Warps::new(48000.0);
        let mut out = vec![0.0; input.len()];
        for (i, o) in input.chunks(256).zip(out.chunks_mut(256)) {
            w.process(p, i, o);
        }
        out
    }

    fn sine(f: f32, n: usize) -> Vec<f32> {
        (0..n).map(|i| (std::f32::consts::TAU * f * i as f32 / 48000.0).sin() * 0.5).collect()
    }

    #[test]
    fn the_converters_keep_their_gain() {
        // An upsampler's kernel sums to its ratio, a downsampler's to one.
        let up: f32 = resources::SRC_UP_6_48.iter().sum::<f32>() * 2.0;
        let down: f32 = resources::SRC_DOWN_6_48.iter().sum::<f32>() * 2.0;
        assert!((up - 6.0).abs() < 0.05, "{up}");
        assert!((down - 1.0).abs() < 0.01, "{down}");
    }

    #[test]
    fn every_algorithm_sounds_and_stays_finite() {
        let input = sine(220.0, 48000);
        // (at 1.0 the vocoder freezes the spectrum it has: from silence, silence)
        for k in 0..=9 {
            for shape in 1..=3 {
                let p = Params { algorithm: k as f32 / 10.0, timbre: 0.6, drive: 0.6, note: 60.0, shape };
                let out = run(&p, &input);
                assert!(out.iter().all(|x| x.is_finite()), "alg {k} shape {shape}");
                let level = rms(&out[24000..]);
                // (a single tone through the vocoder opens one band: with the noise carrier, quiet)
                let least = if k >= 7 { 0.003 } else { 0.01 };
                assert!(level > least && level < 1.5, "alg {k} shape {shape}: {level}");
            }
        }
    }

    #[test]
    fn ring_modulation_makes_sum_and_difference_tones() {
        // A 1 kHz input ringed by a 440 Hz sine: little left at 1 kHz.
        let input = sine(1000.0, 48000);
        let p = Params { algorithm: 3.0 / 8.0 + 0.001, timbre: 0.0, drive: 0.5, note: 69.0, shape: 1 };
        let out = run(&p, &input);
        let x = &out[24000..];
        let at = |f: f32| {
            let (mut re, mut im) = (0.0f32, 0.0f32);
            for (i, v) in x.iter().enumerate() {
                let ph = std::f32::consts::TAU * f * i as f32 / 48000.0;
                re += v * ph.cos();
                im += v * ph.sin();
            }
            (re * re + im * im).sqrt() / x.len() as f32
        };
        assert!(at(1440.0) > 3.0 * at(1000.0), "{} {}", at(1440.0), at(1000.0));
        assert!(at(560.0) > 3.0 * at(1000.0), "{} {}", at(560.0), at(1000.0));
    }

    #[test]
    fn the_vocoder_follows_its_input() {
        // Silence in: the vocoder is quiet; a signal in: it speaks.
        let p = Params { algorithm: 0.85, timbre: 0.5, drive: 0.7, note: 48.0, shape: 1 };
        let quiet = run(&p, &vec![0.0; 24000]);
        let loud = run(&p, &sine(300.0, 48000));
        assert!(rms(&quiet[12000..]) < 0.01, "{}", rms(&quiet[12000..]));
        assert!(rms(&loud[24000..]) > 0.05, "{}", rms(&loud[24000..]));
    }
}

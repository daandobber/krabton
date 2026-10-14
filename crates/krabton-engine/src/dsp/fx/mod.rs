//! More audio effects, one module each: modulation (Phaser, Flanger, Auto
//! Pan, Frequency Shifter, Rotary), dynamics (Gate, Transient, Multiband),
//! delays and loops (Echo, Beat Repeat), pitch (Shifter), filters (Auto Wah),
//! resonance (Resonators) and character (Vinyl). Written for Krabton.

pub mod airwindows;
pub mod auto_pan;
pub mod auto_wah;
pub mod hall;
pub mod lattice;
pub mod massive;
pub mod beads;
pub mod beat_repeat;
pub mod clouds;
pub mod comb;
pub mod convolution;
pub mod echo;
pub mod flanger;
pub mod freq_shifter;
pub mod gate;
pub mod glitch;
pub mod grains;
pub mod moo;
pub mod multiband;
pub mod open_air;
pub mod pedals;
pub mod pitch_delay;
pub mod plate;
pub mod phaser;
pub mod resonators;
pub mod rotary;
pub mod shifter;
pub mod sky;
pub mod smear;
pub mod snowglobe;
pub mod slam;
pub mod spring;
pub mod telephone;
pub mod transient;
pub mod verb;
pub mod vinyl;

use std::f32::consts::TAU;

/// Choices for a tempo-synced LFO; "Free" uses the rate in Hz.
pub const LFO_SYNC: &[&str] = &["Free", "1/16", "1/8", "1/4", "1/2", "1 Bar", "2 Bars", "4 Bars"];
const LFO_SYNC_BEATS: [f64; 8] = [0.0, 0.25, 0.5, 1.0, 2.0, 4.0, 8.0, 16.0];

/// An LFO that runs free at a rate in Hz, or locks to the song position
/// when synced and the transport plays.
#[derive(Clone, Copy, Default)]
pub struct Lfo {
    pub phase: f32,
}

impl Lfo {
    /// Phase (0..1) for the next `n` samples, written to `out`.
    pub fn run(&mut self, rate_hz: f32, sync: f32, ctx: &super::effects::FxCtx, out: &mut [f32]) {
        let k = (sync.round().max(0.0) as usize).min(LFO_SYNC_BEATS.len() - 1);
        if k > 0 && ctx.playing {
            let beats = LFO_SYNC_BEATS[k];
            let per_sample = ctx.bpm as f64 / 60.0 / ctx.sr as f64;
            for (i, o) in out.iter_mut().enumerate() {
                *o = ((ctx.beat + i as f64 * per_sample) / beats).rem_euclid(1.0) as f32;
            }
            self.phase = out.last().copied().unwrap_or(self.phase);
            return;
        }
        let rate = if k > 0 { ctx.bpm / 60.0 / LFO_SYNC_BEATS[k] as f32 } else { rate_hz };
        let dp = rate.max(0.0) / ctx.sr;
        for o in out.iter_mut() {
            self.phase += dp;
            if self.phase >= 1.0 {
                self.phase -= 1.0;
            }
            *o = self.phase;
        }
    }
}

#[inline]
pub fn sine(phase: f32) -> f32 {
    (phase * TAU).sin()
}

/// Wraps a phase into 0..1.
#[inline]
pub fn wrap(p: f32) -> f32 {
    p - p.floor()
}

/// A delay line read with cubic (Hermite) interpolation.
pub struct DelayLine {
    buf: Vec<f32>,
    pos: usize,
}

impl DelayLine {
    pub fn new(len: usize) -> Self {
        Self { buf: vec![0.0; len.max(4)], pos: 0 }
    }

    pub fn len(&self) -> usize {
        self.buf.len()
    }

    #[inline]
    pub fn write(&mut self, x: f32) {
        self.pos = (self.pos + 1) % self.buf.len();
        self.buf[self.pos] = x;
    }

    /// The sample written `delay` samples ago (fractional, at least 1).
    #[inline]
    pub fn read(&self, delay: f32) -> f32 {
        let n = self.buf.len();
        let d = delay.clamp(1.0, (n - 3) as f32);
        let i = d.floor() as usize;
        let f = d - i as f32;
        let at = |k: usize| self.buf[(self.pos + n * 2 - k) % n];
        let (xm1, x0, x1, x2) = (at(i - 1), at(i), at(i + 1), at(i + 2));
        let c1 = 0.5 * (x1 - xm1);
        let c2 = xm1 - 2.5 * x0 + 2.0 * x1 - 0.5 * x2;
        let c3 = 0.5 * (x2 - xm1) + 1.5 * (x0 - x1);
        ((c3 * f + c2) * f + c1) * f + x0
    }

    pub fn clear(&mut self) {
        self.buf.fill(0.0);
    }
}

/// A pitch shifter on one channel: two read heads sweep a short delay,
/// crossfaded so one is always away from its jump.
pub struct PitchShifter {
    line: DelayLine,
    phase: f32,
}

impl PitchShifter {
    /// Room for windows up to `max_window_s` seconds.
    pub fn new(sr: f32, max_window_s: f32) -> Self {
        Self { line: DelayLine::new((sr * max_window_s) as usize + 8), phase: 0.0 }
    }

    /// Shifts by `ratio` (2 = an octave up) with a window of `window` samples.
    #[inline]
    pub fn tick(&mut self, x: f32, ratio: f32, window: f32) -> f32 {
        let window = window.clamp(16.0, (self.line.len() - 8) as f32);
        self.line.write(x);
        self.phase = wrap(self.phase + (1.0 - ratio) / window);
        let a = self.phase;
        let b = wrap(a + 0.5);
        let ya = self.line.read(1.0 + a * window);
        let yb = self.line.read(1.0 + b * window);
        ya * (std::f32::consts::PI * a).sin() + yb * (std::f32::consts::PI * b).sin()
    }

    pub fn clear(&mut self) {
        self.line.clear();
    }
}

/// A Schroeder all-pass on a delay line (its taps can be read, as in plate
/// reverbs).
pub struct Allpass {
    pub line: DelayLine,
}

impl Allpass {
    pub fn new(len: usize) -> Self {
        Self { line: DelayLine::new(len + 4) }
    }

    /// One sample through an all-pass of `delay` samples and coefficient `g`.
    #[inline]
    pub fn tick(&mut self, x: f32, delay: f32, g: f32) -> f32 {
        let d = self.line.read(delay);
        let v = x + g * d;
        self.line.write(v);
        d - g * v
    }
}

/// An envelope follower with separate attack and release times.
#[derive(Clone, Copy, Default)]
pub struct Follower {
    pub env: f32,
    att: f32,
    rel: f32,
}

impl Follower {
    pub fn set(&mut self, attack_s: f32, release_s: f32, sr: f32) {
        self.att = 1.0 - (-1.0 / (attack_s.max(1e-5) * sr)).exp();
        self.rel = 1.0 - (-1.0 / (release_s.max(1e-5) * sr)).exp();
    }

    #[inline]
    pub fn tick(&mut self, x: f32) -> f32 {
        let c = if x > self.env { self.att } else { self.rel };
        self.env += (x - self.env) * c;
        self.env
    }
}

#[cfg(test)]
pub(crate) mod test_util {
    use super::super::effects::FxCtx;

    pub fn ctx() -> FxCtx {
        FxCtx { sr: 48000.0, bpm: 120.0, beat: 0.0, playing: false }
    }

    pub fn tone(freq: f32, secs: f32, amp: f32) -> Vec<f32> {
        (0..(secs * 48000.0) as usize).map(|i| amp * (i as f32 * freq * std::f32::consts::TAU / 48000.0).sin()).collect()
    }

    pub fn peak(v: &[f32]) -> f32 {
        v.iter().fold(0.0f32, |m, x| m.max(x.abs()))
    }

    pub fn rms(v: &[f32]) -> f32 {
        (v.iter().map(|x| x * x).sum::<f32>() / v.len().max(1) as f32).sqrt()
    }

    /// Strongest frequency between `lo` and `hi` Hz (by correlation with sines).
    pub fn strongest(v: &[f32], lo: f32, hi: f32, step: f32) -> f32 {
        let mut best = (0.0, lo);
        let mut f = lo;
        while f <= hi {
            let w = f * std::f32::consts::TAU / 48000.0;
            let (mut s, mut c) = (0.0f32, 0.0f32);
            for (i, x) in v.iter().enumerate() {
                s += x * (w * i as f32).sin();
                c += x * (w * i as f32).cos();
            }
            let m = s * s + c * c;
            if m > best.0 {
                best = (m, f);
            }
            f += step;
        }
        best.1
    }
}

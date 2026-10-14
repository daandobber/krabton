//! Slam: hits made to hit. The start of every sound pushed forward (Punch),
//! driven into a warm saturation (Drive), and pumping with the beat as if a
//! kick ducked it (Pump: dips on every beat and comes back up, Shape sets how
//! quickly).

use crate::dsp::effects::FxCtx;
use crate::dsp::fast_tanh;
use crate::params::{ParamSpec, Unit};

pub const PUNCH: usize = 0;
pub const DRIVE: usize = 1;
pub const PUMP: usize = 2;
pub const SHAPE: usize = 3;
pub const MIX: usize = 4;

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::lin("Punch", "Slam", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Drive", "Slam", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::lin("Pump", "Pump", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Shape", "Pump", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Mix", "Output", 0.0, 1.0, 1.0, Unit::Percent),
];

pub struct Slam {
    fast: f32,
    slow: f32,
    /// The beat when the transport is stopped (it pumps at the tempo anyway).
    free_beat: f64,
    sr: f32,
}

impl Slam {
    pub fn new(sr: f32) -> Self {
        Self { fast: 0.0, slow: 0.0, free_beat: 0.0, sr }
    }

    /// How loud the pump lets the sound through at a place in the beat (0..1).
    fn pump(phase: f32, depth: f32, shape: f32) -> f32 {
        let speed = 3.0 + 12.0 * (1.0 - shape);
        1.0 - depth * (-phase * speed).exp()
    }

    pub fn process(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32], ctx: &FxCtx) {
        let n = l.len().min(r.len());
        let sr = self.sr;
        let punch = p[PUNCH].clamp(0.0, 1.0);
        let drive = 1.0 + 6.0 * p[DRIVE].clamp(0.0, 1.0);
        let depth = p[PUMP].clamp(0.0, 1.0) * 0.9;
        let shape = p[SHAPE].clamp(0.0, 1.0);
        let mix = p[MIX].clamp(0.0, 1.0);
        // Two peak followers: one that rises at once, one that takes 15 ms;
        // at the start of a sound the first is ahead of the second.
        let kf = (-1.0 / (0.005 * sr)).exp();
        let (ka, kr) = (1.0 - (-1.0 / (0.015 * sr)).exp(), (-1.0 / (0.12 * sr)).exp());
        let per_frame = ctx.bpm as f64 / 60.0 / sr as f64;
        let start = if ctx.playing { ctx.beat } else { self.free_beat };
        for i in 0..n {
            let (x, y) = (l[i], r[i]);
            let a = x.abs().max(y.abs());
            self.fast = a.max(self.fast * kf);
            self.slow = if a > self.slow { self.slow + (a - self.slow) * ka } else { self.slow * kr };
            let lift = 1.0 + punch * 3.0 * ((self.fast - self.slow).max(0.0) / (self.slow + 1e-3)).min(1.0);
            let beat = start + i as f64 * per_frame;
            let g = if depth > 0.0 { Self::pump(beat.fract() as f32, depth, shape) } else { 1.0 };
            let (sx, sy) = (fast_tanh(x * lift * drive) / drive.sqrt() * g, fast_tanh(y * lift * drive) / drive.sqrt() * g);
            l[i] = x * (1.0 - mix) + sx * mix;
            r[i] = y * (1.0 - mix) + sy * mix;
        }
        self.free_beat = (start + n as f64 * per_frame).fract();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_pumps_with_the_beat() {
        let sr = 48000.0;
        let mut s = Slam::new(sr);
        let mut p = crate::params::defaults(SPECS);
        p[PUNCH] = 0.0;
        p[DRIVE] = 0.0;
        p[PUMP] = 1.0;
        // At 120 BPM a beat is 24000 frames: quiet at its start, loud near its end.
        let ctx = FxCtx { sr, bpm: 120.0, beat: 0.0, playing: true };
        let mut l = vec![0.3; 24000];
        let mut r = vec![0.3; 24000];
        s.process(&p, &mut l, &mut r, &ctx);
        assert!(l[100] < 0.1, "ducked at the beat: {}", l[100]);
        assert!(l[23000] > 0.25, "back up before the next: {}", l[23000]);
    }

    #[test]
    fn punch_lifts_the_start_of_a_sound() {
        let sr = 48000.0;
        let mut s = Slam::new(sr);
        let mut p = crate::params::defaults(SPECS);
        p[PUNCH] = 1.0;
        p[DRIVE] = 0.0;
        let ctx = FxCtx { sr, bpm: 120.0, beat: 0.0, playing: false };
        let mut l = vec![0.0; 4800];
        for (i, x) in l.iter_mut().enumerate().skip(100) {
            *x = 0.2 * ((i as f32) * 0.3).sin();
        }
        let mut r = l.clone();
        s.process(&p, &mut l, &mut r, &ctx);
        let early = l[100..600].iter().fold(0.0f32, |m, x| m.max(x.abs()));
        let late = l[4000..].iter().fold(0.0f32, |m, x| m.max(x.abs()));
        assert!(early > late * 1.2, "start {early}, later {late}");
    }
}

//! Moo: a delay whose echoes shift in frequency. Every repeat is moved by
//! the same number of Hz (Shift) up, down, or (Both) up on the left and down
//! on the right, and fed back in, so each one slides further: the sound
//! spirals away, bends into a low moo, or climbs like a barber's pole.
//! Delay is the time between repeats, Feedback how many, Tone how dark.

use super::freq_shifter::Hilbert;
use super::DelayLine;
use crate::dsp::effects::FxCtx;
use crate::dsp::{fast_tanh, OnePole};
use crate::params::{ParamSpec, Unit};
use std::f32::consts::TAU;

pub const SHIFT: usize = 0;
pub const DELAY: usize = 1;
pub const FEEDBACK: usize = 2;
pub const SIDEBAND: usize = 3;
pub const TONE: usize = 4;
pub const MIX: usize = 5;

pub const SIDEBANDS: &[&str] = &["Up", "Down", "Both"];

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::log("Shift", "Moo", 1.0, 1000.0, 30.0, Unit::Hz),
    ParamSpec::log("Delay", "Moo", 0.02, 1.5, 0.25, Unit::Sec),
    ParamSpec::lin("Feedback", "Moo", 0.0, 0.97, 0.6, Unit::Percent),
    ParamSpec::choice("Sideband", "Moo", SIDEBANDS, 1),
    ParamSpec::lin("Tone", "Moo", 0.0, 1.0, 0.65, Unit::Percent),
    ParamSpec::lin("Mix", "Output", 0.0, 1.0, 0.4, Unit::Percent),
];

pub struct Moo {
    lines: [DelayLine; 2],
    hilbert: [Hilbert; 2],
    damp: [OnePole; 2],
    phase: f32,
    sr: f32,
}

impl Moo {
    pub fn new(sr: f32) -> Self {
        let len = (sr * 1.5) as usize + 16;
        Self { lines: [DelayLine::new(len), DelayLine::new(len)], hilbert: Default::default(), damp: [OnePole::default(); 2], phase: 0.0, sr }
    }

    pub fn process(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32], _ctx: &FxCtx) {
        let n = l.len().min(r.len());
        let sr = self.sr;
        let shift = p[SHIFT].clamp(0.0, 2000.0);
        let delay = p[DELAY].clamp(0.02, 1.5) * sr;
        let fb = p[FEEDBACK].clamp(0.0, 0.97);
        let band = (p[SIDEBAND].round().max(0.0) as usize).min(SIDEBANDS.len() - 1);
        // Up shifts with the quadrature added, down with it taken away.
        let signs = match band {
            0 => [1.0, 1.0],
            1 => [-1.0, -1.0],
            _ => [1.0, -1.0],
        };
        let cutoff = 600.0 + 17000.0 * p[TONE].clamp(0.0, 1.0).powi(2);
        for d in self.damp.iter_mut() {
            d.set(cutoff, sr);
        }
        let mix = p[MIX].clamp(0.0, 1.0);
        let dp = shift / sr;
        for i in 0..n {
            self.phase = super::wrap(self.phase + dp);
            let (s, c) = (self.phase * TAU).sin_cos();
            for (ch, x) in [&mut l[i], &mut r[i]].into_iter().enumerate() {
                let back = self.lines[ch].read(delay);
                let (re, im) = self.hilbert[ch].tick(back);
                let moved = self.damp[ch].lp(re * c + signs[ch] * im * s);
                let moved = if moved.is_finite() { moved } else { 0.0 };
                self.lines[ch].write(fast_tanh(*x + fb * moved));
                *x = *x * (1.0 - mix) + moved * mix;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_util::*;
    use super::*;

    #[test]
    fn each_echo_moves_by_the_shift() {
        let mut m = Moo::new(48000.0);
        let mut p = crate::params::defaults(SPECS);
        p[SHIFT] = 100.0;
        p[DELAY] = 0.25;
        p[FEEDBACK] = 0.5;
        p[SIDEBAND] = 0.0;
        p[TONE] = 1.0;
        p[MIX] = 1.0;
        // A tenth of a second of 1 kHz, then quiet: the echoes come after it.
        let mut l = tone(1000.0, 1.0, 0.5);
        for x in l.iter_mut().skip(4800) {
            *x = 0.0;
        }
        let mut r = l.clone();
        m.process(&p, &mut l, &mut r, &ctx());
        // First echo at a quarter second: 100 Hz up; the second: 200 Hz up.
        let first = strongest(&l[12500..16300], 900.0, 1400.0, 10.0);
        let second = strongest(&l[24500..28300], 900.0, 1400.0, 10.0);
        assert!((first - 1100.0).abs() <= 15.0, "first echo at {first}");
        assert!((second - 1200.0).abs() <= 15.0, "second echo at {second}");
        // Down the other way.
        let mut m = Moo::new(48000.0);
        p[SIDEBAND] = 1.0;
        let mut l = tone(1000.0, 1.0, 0.5);
        for x in l.iter_mut().skip(4800) {
            *x = 0.0;
        }
        let mut r = l.clone();
        m.process(&p, &mut l, &mut r, &ctx());
        let first = strongest(&l[12500..16300], 700.0, 1100.0, 10.0);
        assert!((first - 900.0).abs() <= 15.0, "down: {first}");
        assert!(l.iter().chain(&r).all(|x| x.is_finite() && x.abs() < 2.0));
    }
}

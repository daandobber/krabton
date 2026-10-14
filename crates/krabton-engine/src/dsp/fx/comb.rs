//! Comb: a delay so short it rings at a pitch. Tuned to a note, fed back
//! (or turned over, an octave down and hollow), damped in the loop, wobbling
//! a little, the right channel a little higher than the left: plucked metal,
//! drones out of drums, a voice through a tube.

use super::DelayLine;
use crate::dsp::effects::FxCtx;
use crate::dsp::{fast_tanh, midi_to_freq, OnePole};
use crate::params::{ParamSpec, Unit};
use std::f32::consts::TAU;

pub const PITCH: usize = 0;
pub const FINE: usize = 1;
pub const FEEDBACK: usize = 2;
pub const DAMP: usize = 3;
pub const WOBBLE: usize = 4;
pub const SPREAD: usize = 5;
pub const MIX: usize = 6;

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::int("Pitch", "Comb", 24.0, 96.0, 48.0, Unit::None),
    ParamSpec::lin("Fine", "Comb", -50.0, 50.0, 0.0, Unit::Cents),
    ParamSpec::lin("Feedback", "Comb", -0.99, 0.99, 0.85, Unit::Percent),
    ParamSpec::lin("Damp", "Comb", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::lin("Wobble", "Motion", 0.0, 1.0, 0.1, Unit::Percent),
    ParamSpec::lin("Spread", "Motion", 0.0, 1.0, 0.2, Unit::Percent),
    ParamSpec::lin("Mix", "Output", 0.0, 1.0, 0.5, Unit::Percent),
];

pub struct Comb {
    lines: [DelayLine; 2],
    damp: [OnePole; 2],
    lfo: f32,
    sr: f32,
}

impl Comb {
    pub fn new(sr: f32) -> Self {
        let len = (sr / midi_to_freq(23.0)) as usize * 2 + 16;
        Self { lines: [DelayLine::new(len), DelayLine::new(len)], damp: [OnePole::default(); 2], lfo: 0.0, sr }
    }

    pub fn process(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32], _ctx: &FxCtx) {
        let n = l.len().min(r.len());
        let sr = self.sr;
        let note = p[PITCH].round() + p[FINE] / 100.0;
        let fb = p[FEEDBACK].clamp(-0.99, 0.99);
        let cutoff = 18000.0 * (1.0 - p[DAMP].clamp(0.0, 1.0)).powi(2) + 400.0;
        for d in self.damp.iter_mut() {
            d.set(cutoff, sr);
        }
        let mix = p[MIX].clamp(0.0, 1.0);
        let wobble = p[WOBBLE].clamp(0.0, 1.0) * 0.012;
        let spread = p[SPREAD].clamp(0.0, 1.0) * 0.35;
        let period = [sr / midi_to_freq(note), sr / midi_to_freq(note + spread)];
        // Louder feedback rings louder: bring it back down.
        let norm = 1.0 - fb.abs() * 0.85;
        for i in 0..n {
            self.lfo = (self.lfo + 0.37 / sr).fract();
            let w = 1.0 + wobble * (TAU * self.lfo).sin();
            for (c, x) in [&mut l[i], &mut r[i]].into_iter().enumerate() {
                // The loop is a sample longer than the read: take it off, so it rings in tune.
                let back = self.damp[c].lp(self.lines[c].read(period[c] * if c == 0 { w } else { 2.0 - w } - 1.0));
                let v = fast_tanh(*x + fb * back);
                self.lines[c].write(v);
                *x = *x * (1.0 - mix) + v * norm * mix;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_rings_at_its_pitch() {
        let sr = 48000.0;
        let mut c = Comb::new(sr);
        let mut p = crate::params::defaults(SPECS);
        p[PITCH] = 57.0; // 220 Hz
        p[MIX] = 1.0;
        p[DAMP] = 0.0;
        p[WOBBLE] = 0.0;
        p[SPREAD] = 0.0;
        let ctx = FxCtx { sr, bpm: 120.0, beat: 0.0, playing: false };
        let mut l = vec![0.0; 9600];
        let mut r = vec![0.0; 9600];
        l[0] = 1.0;
        r[0] = 1.0;
        c.process(&p, &mut l, &mut r, &ctx);
        // An echo every period (218 samples at 220 Hz).
        // The strongest repeat in the ringing is one period of 220 Hz.
        let tail = &l[2000..9600];
        let corr = |lag: usize| tail.iter().zip(&tail[lag..]).map(|(a, b)| a * b).sum::<f32>();
        let best = (150..300).max_by(|&a, &b| corr(a).total_cmp(&corr(b))).unwrap();
        assert!((best as f32 - sr / 220.0).abs() < 1.5, "repeats every {best} frames");
        assert!(tail.iter().map(|x| x.abs()).fold(0.0f32, f32::max) > 0.01, "still ringing");
        assert!(l.iter().all(|x| x.is_finite() && x.abs() < 2.0));
    }
}

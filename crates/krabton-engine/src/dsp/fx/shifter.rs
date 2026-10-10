//! Shifter: a pitch shifter. Two read heads sweep through a short delay at
//! a different speed than it is written, crossfading so one is always away
//! from the jump; feedback makes rising or falling cascades.

use super::DelayLine;
use crate::dsp::effects::FxCtx;
use crate::params::{ParamSpec, Unit};
use std::f32::consts::PI;

pub const PITCH: usize = 0;
pub const FINE: usize = 1;
pub const WINDOW: usize = 2;
pub const FEEDBACK: usize = 3;
pub const WIDE: usize = 4;
pub const MIX: usize = 5;

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::int("Pitch", "Pitch", -24.0, 24.0, 7.0, Unit::Semi),
    ParamSpec::lin("Fine", "Pitch", -50.0, 50.0, 0.0, Unit::Cents),
    ParamSpec::log("Window", "Pitch", 15.0, 200.0, 60.0, Unit::Ms),
    ParamSpec::lin("Feedback", "Output", 0.0, 0.95, 0.0, Unit::Percent),
    ParamSpec::lin("Wide", "Output", 0.0, 50.0, 0.0, Unit::Cents),
    ParamSpec::lin("Mix", "Output", 0.0, 1.0, 1.0, Unit::Percent),
];

pub struct Shifter {
    lines: [DelayLine; 2],
    /// Head position (0..1 of the window) per channel.
    phase: [f32; 2],
    last: [f32; 2],
}

impl Shifter {
    pub fn new(sr: f32) -> Self {
        let len = (sr * 0.45) as usize;
        Self { lines: [DelayLine::new(len), DelayLine::new(len)], phase: [0.0, 0.0], last: [0.0; 2] }
    }

    pub fn process(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32], ctx: &FxCtx) {
        let n = l.len().min(r.len());
        let sr = ctx.sr;
        let window = (p[WINDOW].clamp(5.0, 400.0) * 0.001 * sr).min((self.lines[0].len() - 8) as f32 * 0.5);
        let semis = p[PITCH].round() + p[FINE] * 0.01;
        let wide = p[WIDE].clamp(0.0, 100.0) * 0.01;
        let ratio = [2f32.powf((semis - wide) / 12.0), 2f32.powf((semis + wide) / 12.0)];
        let (fb, mix) = (p[FEEDBACK].clamp(0.0, 0.95), p[MIX].clamp(0.0, 1.0));
        for i in 0..n {
            for ch in 0..2 {
                let x = if ch == 0 { l[i] } else { r[i] };
                // The delay shrinks (pitch up) or grows (down) through the window.
                self.phase[ch] = super::wrap(self.phase[ch] + (1.0 - ratio[ch]) / window);
                let a = self.phase[ch];
                let b = super::wrap(a + 0.5);
                let ya = self.lines[ch].read(1.0 + a * window);
                let yb = self.lines[ch].read(1.0 + b * window);
                // Each head is silent at its jump.
                let wet = ya * (PI * a).sin() + yb * (PI * b).sin();
                let wet = if wet.is_finite() { wet * 0.8 } else { 0.0 };
                self.lines[ch].write(x + fb * wet);
                self.last[ch] = wet;
                let out = x * (1.0 - mix) + wet * mix;
                if ch == 0 {
                    l[i] = out;
                } else {
                    r[i] = out;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_util::*;
    use super::*;

    #[test]
    fn an_octave_up_doubles_the_frequency() {
        let mut s = Shifter::new(48000.0);
        let mut p = crate::params::defaults(SPECS);
        p[PITCH] = 12.0;
        let mut l = tone(300.0, 1.0, 0.5);
        let mut r = l.clone();
        for (cl, cr) in l.chunks_mut(256).zip(r.chunks_mut(256)) {
            s.process(&p, cl, cr, &ctx());
        }
        let got = strongest(&l[24000..], 200.0, 800.0, 5.0);
        assert!((got - 600.0).abs() <= 10.0, "{got}");
    }
}

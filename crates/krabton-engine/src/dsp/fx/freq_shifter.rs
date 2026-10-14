//! Frequency Shifter: moves every partial by the same number of Hz (not a
//! pitch shift, so harmonics go inharmonic), or ring modulates. The shift
//! uses a Hilbert transform built from two all-pass chains (Olli
//! Niemitalo's coefficients), multiplied by a quadrature oscillator.

use crate::dsp::effects::FxCtx;
use crate::params::{ParamSpec, Unit};
use std::f32::consts::TAU;

pub const MODE: usize = 0;
pub const COARSE: usize = 1;
pub const FINE: usize = 2;
pub const WIDE: usize = 3;
pub const FEEDBACK: usize = 4;
pub const MIX: usize = 5;

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::choice("Mode", "Shifter", &["Shift", "Ring"], 0),
    ParamSpec::int("Coarse", "Shifter", -2000.0, 2000.0, 100.0, Unit::Hz),
    ParamSpec::lin("Fine", "Shifter", -50.0, 50.0, 0.0, Unit::Hz),
    ParamSpec::toggle("Wide", "Shifter", false),
    ParamSpec::lin("Feedback", "Output", 0.0, 0.95, 0.0, Unit::Percent),
    ParamSpec::lin("Mix", "Output", 0.0, 1.0, 1.0, Unit::Percent),
];

const A_I: [f32; 4] = [0.692_387_8, 0.936_065_43, 0.988_229_5, 0.998_748_84];
const A_Q: [f32; 4] = [0.402_192_12, 0.856_171_1, 0.972_290_95, 0.995_288_5];

/// One path of the Hilbert pair: four second-order all-pass sections on z^-2.
#[derive(Clone, Copy, Default)]
struct Path {
    x1: [f32; 4],
    x2: [f32; 4],
    y1: [f32; 4],
    y2: [f32; 4],
}

impl Path {
    #[inline]
    fn tick(&mut self, coefs: &[f32; 4], mut x: f32) -> f32 {
        for k in 0..4 {
            let a2 = coefs[k] * coefs[k];
            let y = a2 * (x + self.y2[k]) - self.x2[k];
            self.x2[k] = self.x1[k];
            self.x1[k] = x;
            self.y2[k] = self.y1[k];
            self.y1[k] = y;
            x = y;
        }
        x
    }
}

/// A Hilbert pair: a signal and its quadrature (90 degrees behind at every
/// frequency), to shift frequencies with.
#[derive(Clone, Copy, Default)]
pub(crate) struct Hilbert {
    i: Path,
    q: Path,
    /// Path I is one sample late.
    i_delay: f32,
}

impl Hilbert {
    /// (in phase, quadrature) of `x`.
    #[inline]
    pub(crate) fn tick(&mut self, x: f32) -> (f32, f32) {
        let i_out = self.i_delay;
        self.i_delay = self.i.tick(&A_I, x);
        (i_out, self.q.tick(&A_Q, x))
    }
}

#[derive(Default)]
pub struct FreqShifter {
    hilbert: [Hilbert; 2],
    phase: f32,
    last: [f32; 2],
}

impl FreqShifter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn process(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32], ctx: &FxCtx) {
        let n = l.len().min(r.len());
        let ring = p[MODE] >= 0.5;
        let shift = p[COARSE].round() + p[FINE];
        let wide = p[WIDE] >= 0.5;
        let (fb, mix) = (p[FEEDBACK].clamp(0.0, 0.95), p[MIX].clamp(0.0, 1.0));
        let dp = if ring { shift.abs() } else { shift } / ctx.sr;
        for i in 0..n {
            self.phase = super::wrap(self.phase + dp);
            let (s, c) = (self.phase * TAU).sin_cos();
            for ch in 0..2 {
                let x = if ch == 0 { l[i] } else { r[i] };
                let v = x + fb * self.last[ch];
                let wet = if ring {
                    v * c
                } else {
                    let (i_out, q_out) = self.hilbert[ch].tick(v);
                    // The right side goes the other way when wide.
                    let sign = if wide && ch == 1 { -1.0 } else { 1.0 };
                    i_out * c + sign * q_out * s
                };
                let wet = if wet.is_finite() { wet } else { 0.0 };
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
    fn a_sine_moves_up_by_the_shift() {
        let mut f = FreqShifter::new();
        let p = crate::params::defaults(SPECS);
        let mut l = tone(1000.0, 0.5, 0.5);
        let mut r = l.clone();
        f.process(&p, &mut l, &mut r, &ctx());
        let tail = &l[4800..];
        let got = strongest(tail, 800.0, 1300.0, 10.0);
        assert!((got - 1100.0).abs() <= 10.0, "{got}");
        assert!(peak(tail) < 0.7 && peak(tail) > 0.3);
    }

    #[test]
    fn ring_mode_makes_sum_and_difference() {
        let mut f = FreqShifter::new();
        let mut p = crate::params::defaults(SPECS);
        p[MODE] = 1.0;
        p[COARSE] = 300.0;
        let mut l = tone(1000.0, 0.5, 0.5);
        let mut r = l.clone();
        f.process(&p, &mut l, &mut r, &ctx());
        let got = strongest(&l[4800..], 600.0, 1400.0, 10.0);
        assert!((got - 700.0).abs() <= 10.0 || (got - 1300.0).abs() <= 10.0, "{got}");
    }
}

//! Multiband: three bands, each compressed downwards above a threshold and
//! upwards below another, the over-the-top way: quiet details come up, loud
//! peaks go down. Bands are split with Linkwitz-Riley crossovers that add
//! back up flat.

use super::Follower;
use crate::dsp::effects::FxCtx;
use crate::dsp::{db_to_gain, gain_to_db, Svf};
use crate::params::{ParamSpec, Unit};

pub const DEPTH: usize = 0;
pub const TIME: usize = 1;
pub const DOWNWARD: usize = 2;
pub const UPWARD: usize = 3;
pub const LOW: usize = 4;
pub const MID: usize = 5;
pub const HIGH: usize = 6;
pub const SPLIT_LOW: usize = 7;
pub const SPLIT_HIGH: usize = 8;
pub const OUTPUT: usize = 9;

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::lin("Depth", "Dynamics", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::lin("Time", "Dynamics", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Downward", "Dynamics", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::lin("Upward", "Dynamics", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::lin("Low", "Bands", -24.0, 24.0, 0.0, Unit::Db),
    ParamSpec::lin("Mid", "Bands", -24.0, 24.0, 0.0, Unit::Db),
    ParamSpec::lin("High", "Bands", -24.0, 24.0, 0.0, Unit::Db),
    ParamSpec::log("Low/Mid", "Bands", 40.0, 1000.0, 120.0, Unit::Hz),
    ParamSpec::log("Mid/High", "Bands", 1000.0, 12000.0, 2500.0, Unit::Hz),
    ParamSpec::lin("Output", "Output", -24.0, 24.0, 0.0, Unit::Db),
];

/// A fourth-order Linkwitz-Riley low/high pair (two Butterworth stages each).
#[derive(Clone, Copy)]
struct Lr4 {
    lp: [Svf; 2],
    hp: [Svf; 2],
}

impl Lr4 {
    fn new() -> Self {
        Self { lp: [Svf::new(); 2], hp: [Svf::new(); 2] }
    }

    fn set(&mut self, f: f32, sr: f32) {
        for s in self.lp.iter_mut().chain(self.hp.iter_mut()) {
            s.set_q(f, std::f32::consts::FRAC_1_SQRT_2, sr);
        }
    }

    #[inline]
    fn split(&mut self, x: f32) -> (f32, f32) {
        let lo1 = self.lp[0].tick(x).low;
        let lo = self.lp[1].tick(lo1).low;
        let hi1 = self.hp[0].tick(x).high;
        let hi = self.hp[1].tick(hi1).high;
        (lo, hi)
    }
}

/// Per band: downward and upward thresholds (dB) and fixed band trims.
const THRESH: [(f32, f32); 3] = [(-20.0, -34.0), (-22.0, -36.0), (-24.0, -38.0)];

pub struct Multiband {
    /// Per channel: the two crossovers, and an all-pass for the low band.
    x1: [Lr4; 2],
    x2: [Lr4; 2],
    low_ap: [Lr4; 2],
    det: [Follower; 3],
    gain: [f32; 3],
}

impl Multiband {
    pub fn new() -> Self {
        Self { x1: [Lr4::new(); 2], x2: [Lr4::new(); 2], low_ap: [Lr4::new(); 2], det: [Follower::default(); 3], gain: [1.0; 3] }
    }

    pub fn process(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32], ctx: &FxCtx) {
        let n = l.len().min(r.len());
        let sr = ctx.sr;
        let f1 = p[SPLIT_LOW].clamp(20.0, 2000.0);
        let f2 = p[SPLIT_HIGH].clamp(f1 * 2.0, 0.45 * sr);
        for c in 0..2 {
            self.x1[c].set(f1, sr);
            self.x2[c].set(f2, sr);
            self.low_ap[c].set(f2, sr);
        }
        // Time scales attack and release (fast for the highs).
        let t = 0.25 * 8f32.powf(p[TIME].clamp(0.0, 1.0));
        for (b, d) in self.det.iter_mut().enumerate() {
            let k = [1.6, 1.0, 0.5][b];
            d.set(0.004 * t * k, 0.09 * t * k, sr);
        }
        let depth = p[DEPTH].clamp(0.0, 1.0);
        let down = 1.0 + 9.0 * p[DOWNWARD].clamp(0.0, 1.0);
        let up = 1.0 + 3.0 * p[UPWARD].clamp(0.0, 1.0);
        let band_gain = [db_to_gain(p[LOW]), db_to_gain(p[MID]), db_to_gain(p[HIGH])];
        let out = db_to_gain(p[OUTPUT]);
        let smooth = 1.0 - (-1.0 / (0.001 * sr)).exp();
        for i in 0..n {
            let mut bands = [[0.0f32; 3]; 2];
            for c in 0..2 {
                let x = if c == 0 { l[i] } else { r[i] };
                let (lo, rest) = self.x1[c].split(x);
                let (mid, hi) = self.x2[c].split(rest);
                // The low band goes through the same phase turn as the others.
                let (a, b) = self.low_ap[c].split(lo);
                bands[c] = [a + b, mid, hi];
            }
            for b in 0..3 {
                let level = gain_to_db(self.det[b].tick(bands[0][b].abs().max(bands[1][b].abs())));
                let (td, tu) = THRESH[b];
                let mut g_db = 0.0;
                if level > td {
                    g_db -= (level - td) * (1.0 - 1.0 / down);
                } else if level < tu && level > -75.0 {
                    g_db += ((tu - level) * (1.0 - 1.0 / up)).min(24.0);
                }
                let g = db_to_gain(g_db * depth);
                self.gain[b] += (g - self.gain[b]) * smooth;
            }
            let mut y = [0.0f32; 2];
            for c in 0..2 {
                for b in 0..3 {
                    y[c] += bands[c][b] * self.gain[b] * band_gain[b];
                }
            }
            l[i] = y[0] * out;
            r[i] = y[1] * out;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_util::*;
    use super::*;

    #[test]
    fn at_zero_depth_the_bands_add_back_up() {
        let mut m = Multiband::new();
        let mut p = crate::params::defaults(SPECS);
        p[DEPTH] = 0.0;
        for f in [60.0, 1000.0, 8000.0] {
            let mut l = tone(f, 0.5, 0.5);
            let mut r = l.clone();
            m.process(&p, &mut l, &mut r, &ctx());
            let level = rms(&l[12000..]) / rms(&tone(f, 0.5, 0.5)[12000..]);
            assert!((level - 1.0).abs() < 0.1, "{f}: {level}");
        }
    }

    #[test]
    fn quiet_comes_up_and_loud_goes_down() {
        let run = |amp: f32| {
            let mut m = Multiband::new();
            let p = crate::params::defaults(SPECS);
            let mut l = tone(1000.0, 1.0, amp);
            let mut r = l.clone();
            m.process(&p, &mut l, &mut r, &ctx());
            rms(&l[24000..]) / (amp * std::f32::consts::FRAC_1_SQRT_2)
        };
        assert!(run(0.005) > 2.0, "{}", run(0.005));
        assert!(run(0.9) < 0.5, "{}", run(0.9));
    }
}

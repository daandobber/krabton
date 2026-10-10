//! Auto Wah: an envelope filter. The louder you play, the further the
//! resonant filter opens (or closes, with a negative sensitivity).

use super::Follower;
use crate::dsp::effects::FxCtx;
use crate::dsp::{FilterMode, Svf};
use crate::params::{ParamSpec, Unit};

pub const SENSITIVITY: usize = 0;
pub const ATTACK: usize = 1;
pub const RELEASE: usize = 2;
pub const LOW: usize = 3;
pub const HIGH: usize = 4;
pub const RESONANCE: usize = 5;
pub const MODE: usize = 6;
pub const MIX: usize = 7;

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::lin("Sensitivity", "Envelope", -1.0, 1.0, 0.6, Unit::Percent),
    ParamSpec::log("Attack", "Envelope", 1.0, 200.0, 6.0, Unit::Ms),
    ParamSpec::log("Release", "Envelope", 10.0, 2000.0, 160.0, Unit::Ms),
    ParamSpec::log("Low", "Filter", 60.0, 3000.0, 300.0, Unit::Hz),
    ParamSpec::log("High", "Filter", 300.0, 12000.0, 3000.0, Unit::Hz),
    ParamSpec::lin("Resonance", "Filter", 0.0, 1.0, 0.65, Unit::Percent),
    ParamSpec::choice("Mode", "Filter", &["Lowpass", "Bandpass", "Highpass"], 1),
    ParamSpec::lin("Mix", "Output", 0.0, 1.0, 1.0, Unit::Percent),
];

pub struct AutoWah {
    env: Follower,
    f: [Svf; 2],
}

impl AutoWah {
    pub fn new() -> Self {
        Self { env: Follower::default(), f: [Svf::new(), Svf::new()] }
    }

    pub fn process(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32], ctx: &FxCtx) {
        let n = l.len().min(r.len());
        let sr = ctx.sr;
        self.env.set(p[ATTACK].max(0.1) * 0.001, p[RELEASE].max(1.0) * 0.001, sr);
        let sens = p[SENSITIVITY].clamp(-1.0, 1.0);
        let (lo, hi) = (p[LOW].clamp(20.0, 0.4 * sr), p[HIGH].clamp(20.0, 0.45 * sr));
        let res = p[RESONANCE].clamp(0.0, 1.0) * 0.95;
        let mode = match p[MODE].round() as i32 {
            0 => FilterMode::Low,
            2 => FilterMode::High,
            _ => FilterMode::Band,
        };
        let mix = p[MIX].clamp(0.0, 1.0);
        for i in 0..n {
            let (xl, xr) = (l[i], r[i]);
            // Level 0..1 (about -40..0 dB), shaped by the sensitivity.
            let e = self.env.tick(xl.abs().max(xr.abs()));
            let level = ((crate::dsp::gain_to_db(e) + 40.0) / 40.0).clamp(0.0, 1.0);
            let amt = if sens >= 0.0 { level * sens } else { 1.0 + level * sens };
            let fc = lo * (hi / lo).powf(amt.clamp(0.0, 1.0));
            if i % 8 == 0 {
                for f in self.f.iter_mut() {
                    f.set(fc, res, sr);
                }
            }
            let yl = self.f[0].process(xl, mode);
            let yr = self.f[1].process(xr, mode);
            l[i] = xl * (1.0 - mix) + yl * mix;
            r[i] = xr * (1.0 - mix) + yr * mix;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_util::*;
    use super::*;

    #[test]
    fn it_wahs_without_blowing_up() {
        let mut w = AutoWah::new();
        let p = crate::params::defaults(SPECS);
        let mut rng = crate::dsp::Rng::new(5);
        let mut l: Vec<f32> = (0..48000).map(|i| rng.bipolar() * if i % 12000 < 3000 { 0.8 } else { 0.02 }).collect();
        let mut r = l.clone();
        w.process(&p, &mut l, &mut r, &ctx());
        assert!(l.iter().all(|x| x.is_finite()) && peak(&l) < 4.0 && rms(&l) > 0.01);
    }
}

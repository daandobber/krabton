//! De-Esser: tames the sharp s of a voice and the fizz on a cymbal, and only
//! when they are there. It listens above *Frequency*, and turns just that part
//! down for as long as it is over the *Threshold*. *Listen* lets you hear what
//! it is taking away.

use super::super::{db_to_gain, gain_to_db, Svf};
use crate::dsp::effects::FxCtx;
use crate::params::{ParamSpec, Unit};

pub const FREQUENCY: usize = 0;
pub const THRESHOLD: usize = 1;
pub const RANGE: usize = 2;
pub const LISTEN: usize = 3;

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::log("Frequency", "De-Ess", 2000.0, 14000.0, 6000.0, Unit::Hz),
    ParamSpec::lin("Threshold", "De-Ess", -60.0, 0.0, -28.0, Unit::Db),
    ParamSpec::lin("Range", "De-Ess", 0.0, 24.0, 10.0, Unit::Db),
    ParamSpec::toggle("Listen", "De-Ess", false),
];

/// What the filter's band part counts for at this Q (one over it).
const KEEP: f32 = 1.0 / 0.707;

pub struct DeEsser {
    sr: f32,
    high: [Svf; 2],
    env: f32,
}

impl DeEsser {
    pub fn new(sr: f32) -> Self {
        Self { sr, high: [Svf::new(); 2], env: 0.0 }
    }

    pub fn process(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32], _ctx: &FxCtx) {
        let sr = self.sr;
        let n = l.len().min(r.len());
        let hz = p[FREQUENCY].clamp(200.0, sr * 0.45);
        for f in self.high.iter_mut() {
            f.set_q(hz, 0.707, sr);
        }
        let threshold = p[THRESHOLD];
        let range = p[RANGE].max(0.0);
        let listen = p[LISTEN] >= 0.5;
        let attack = 1.0 - (-1.0 / (0.001 * sr)).exp();
        let release = 1.0 - (-1.0 / (0.06 * sr)).exp();
        for i in 0..n {
            let (dl, dr) = (l[i], r[i]);
            let (a_l, a_r) = (self.high[0].tick(dl), self.high[1].tick(dr));
            let a = a_l.high.abs().max(a_r.high.abs());
            self.env += (a - self.env) * if a > self.env { attack } else { release };
            let over = (gain_to_db(self.env.max(1e-6)) - threshold).max(0.0);
            let g = db_to_gain(-over.min(range));
            // A shelf from the filter's own three parts, down by `g` at the top:
            // what is below it is untouched, however far it is turned down.
            let root = g.sqrt() * KEEP;
            let (wet_l, wet_r) = (a_l.low + root * a_l.band + g * a_l.high, a_r.low + root * a_r.band + g * a_r.high);
            l[i] = if listen { dl - wet_l } else { wet_l };
            r[i] = if listen { dr - wet_r } else { wet_r };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scale::{Scale, ScaleKind};

    const SR: f32 = 48000.0;

    fn run(p: &[f32], hz: f32, level: f32) -> f32 {
        let n = 24000;
        let mut l: Vec<f32> = (0..n).map(|i| (std::f32::consts::TAU * hz * i as f32 / SR).sin() * level).collect();
        let mut r = l.clone();
        let ctx = FxCtx { sr: SR, bpm: 120.0, beat: 0.0, playing: true, scale: Scale::new(0, ScaleKind::Minor), note: None };
        let mut d = DeEsser::new(SR);
        for (a, b) in l.chunks_mut(256).zip(r.chunks_mut(256)) {
            d.process(p, a, b, &ctx);
        }
        (l[12000..].iter().map(|v| v * v).sum::<f32>() / 12000.0).sqrt() / (level * std::f32::consts::FRAC_1_SQRT_2)
    }

    #[test]
    fn a_loud_s_is_turned_down_by_up_to_the_range() {
        let p = crate::params::defaults(SPECS);
        let g = run(&p, 12000.0, 0.5);
        assert!(g < 0.45, "{g}");
        let mut more = p.clone();
        more[RANGE] = 0.0;
        assert!(run(&more, 12000.0, 0.5) > 0.98, "no range, nothing happens");
    }

    #[test]
    fn a_quiet_s_is_left_alone() {
        let p = crate::params::defaults(SPECS);
        let g = run(&p, 9000.0, 0.005);
        assert!(g > 0.95, "{g}");
    }

    #[test]
    fn the_body_of_the_voice_is_left_alone() {
        let p = crate::params::defaults(SPECS);
        let g = run(&p, 300.0, 0.5);
        assert!(g > 0.97, "{g}");
    }

    #[test]
    fn listen_plays_what_it_takes_away() {
        let mut p = crate::params::defaults(SPECS);
        p[LISTEN] = 1.0;
        assert!(run(&p, 9000.0, 0.5) > 0.3, "the s");
        assert!(run(&p, 300.0, 0.5) < 0.02, "not the body");
    }
}

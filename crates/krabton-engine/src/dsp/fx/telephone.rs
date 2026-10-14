//! Telephone: a voice down the line. A narrow band (Band: from an old radio
//! to a tin can), squashed and driven (Squash), with the line's hiss,
//! crackle and dropouts (Line), in mono.

use crate::dsp::effects::FxCtx;
use crate::dsp::{fast_tanh, OnePole, Rng, Svf};
use crate::params::{ParamSpec, Unit};

pub const BAND: usize = 0;
pub const SQUASH: usize = 1;
pub const LINE: usize = 2;
pub const MIX: usize = 3;

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::lin("Band", "Telephone", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Squash", "Telephone", 0.0, 1.0, 0.4, Unit::Percent),
    ParamSpec::lin("Line", "Telephone", 0.0, 1.0, 0.2, Unit::Percent),
    ParamSpec::lin("Mix", "Output", 0.0, 1.0, 1.0, Unit::Percent),
];

pub struct Telephone {
    hp: Svf,
    lp: Svf,
    env: f32,
    rng: Rng,
    crackle: f32,
    dropout: f32,
    hiss: OnePole,
    sr: f32,
}

impl Telephone {
    pub fn new(sr: f32) -> Self {
        Self { hp: Svf::new(), lp: Svf::new(), env: 0.0, rng: Rng::new(0x7E1E), crackle: 0.0, dropout: 1.0, hiss: OnePole::default(), sr }
    }

    pub fn process(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32], _ctx: &FxCtx) {
        let n = l.len().min(r.len());
        let sr = self.sr;
        let band = p[BAND].clamp(0.0, 1.0);
        self.hp.set(250.0 + 650.0 * band, 0.35, sr);
        self.lp.set(4200.0 - 2200.0 * band, 0.45, sr);
        self.hiss.set(3000.0, sr);
        let squash = p[SQUASH].clamp(0.0, 1.0);
        let drive = 1.0 + squash * 10.0;
        let line = p[LINE].clamp(0.0, 1.0);
        let mix = p[MIX].clamp(0.0, 1.0);
        let release = (-1.0 / (0.08 * sr)).exp();
        for i in 0..n {
            let x = (l[i] + r[i]) * 0.5;
            let band_x = self.lp.tick(self.hp.tick(x).high).low;
            // A quick compressor, then the drive.
            self.env = (band_x.abs()).max(self.env * release);
            let comp = 1.0 / (1.0 + squash * 6.0 * self.env);
            let mut y = fast_tanh(band_x * comp * drive) / drive.sqrt();
            if line > 0.0 {
                if self.rng.next_u32() % (40000 - (line * 36000.0) as u32) == 0 {
                    self.crackle = self.rng.bipolar() * 0.5;
                }
                self.crackle *= 0.9;
                if self.rng.next_u32() % 400_000 < (line * line * 40.0) as u32 {
                    self.dropout = 0.0;
                }
                self.dropout = (self.dropout + 1.0 / (0.05 * sr)).min(1.0);
                y = y * (0.2 + 0.8 * self.dropout) + self.crackle * line + self.hiss.lp(self.rng.bipolar()) * line * 0.03;
            }
            l[i] = l[i] * (1.0 - mix) + y * mix;
            r[i] = r[i] * (1.0 - mix) + y * mix;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_lows_do_not_get_through() {
        let sr = 48000.0;
        let mut t = Telephone::new(sr);
        let mut p = crate::params::defaults(SPECS);
        p[LINE] = 0.0;
        p[SQUASH] = 0.0;
        let ctx = FxCtx { sr, bpm: 120.0, beat: 0.0, playing: false };
        let mut tone = |f: f32| -> f32 {
            let mut l: Vec<f32> = (0..24000).map(|i| (std::f32::consts::TAU * f * i as f32 / sr).sin() * 0.5).collect();
            let mut r = l.clone();
            t.process(&p, &mut l, &mut r, &ctx);
            (l[12000..].iter().map(|v| v * v).sum::<f32>() / 12000.0).sqrt()
        };
        let low = tone(80.0);
        let mid = tone(1000.0);
        assert!(low < mid * 0.15, "80 Hz {low}, 1 kHz {mid}");
    }
}

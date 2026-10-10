//! Vinyl: an old record. Crackle and dust pops, surface hiss, motor rumble,
//! wow from an off-centre hole, and wear that narrows the bandwidth.

use super::DelayLine;
use crate::dsp::effects::FxCtx;
use crate::dsp::{OnePole, Rng, Svf};
use crate::params::{ParamSpec, Unit};
use std::f32::consts::TAU;

pub const CRACKLE: usize = 0;
pub const POPS: usize = 1;
pub const HISS: usize = 2;
pub const RUMBLE: usize = 3;
pub const WOW: usize = 4;
pub const WEAR: usize = 5;
pub const WIDTH: usize = 6;
pub const MIX: usize = 7;

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::lin("Crackle", "Surface", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::lin("Pops", "Surface", 0.0, 1.0, 0.2, Unit::Percent),
    ParamSpec::lin("Hiss", "Surface", 0.0, 1.0, 0.1, Unit::Percent),
    ParamSpec::lin("Rumble", "Surface", 0.0, 1.0, 0.1, Unit::Percent),
    ParamSpec::lin("Wow", "Record", 0.0, 1.0, 0.2, Unit::Percent),
    ParamSpec::lin("Wear", "Record", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::lin("Width", "Record", 0.0, 1.0, 0.8, Unit::Percent),
    ParamSpec::lin("Mix", "Output", 0.0, 1.0, 1.0, Unit::Percent),
];

pub struct Vinyl {
    rng: Rng,
    lines: [DelayLine; 2],
    lp: [Svf; 2],
    hp: [OnePole; 2],
    crackle_tone: [Svf; 2],
    pop_tone: OnePole,
    hiss_tone: [OnePole; 2],
    rumble: OnePole,
    /// A click being played: its level and how fast it fades.
    click: [f32; 2],
    pop: f32,
    pop_sign: f32,
    phase: f32,
}

impl Vinyl {
    pub fn new(sr: f32) -> Self {
        let len = (sr * 0.01) as usize + 8;
        Self {
            rng: Rng::new(0x33A),
            lines: [DelayLine::new(len), DelayLine::new(len)],
            lp: [Svf::new(), Svf::new()],
            hp: [OnePole::default(); 2],
            crackle_tone: [Svf::new(), Svf::new()],
            pop_tone: OnePole::default(),
            hiss_tone: [OnePole::default(); 2],
            rumble: OnePole::default(),
            click: [0.0; 2],
            pop: 0.0,
            pop_sign: 1.0,
            phase: 0.0,
        }
    }

    pub fn process(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32], ctx: &FxCtx) {
        let n = l.len().min(r.len());
        let sr = ctx.sr;
        let wear = p[WEAR].clamp(0.0, 1.0);
        for f in self.lp.iter_mut() {
            f.set_q((18000.0 * (1.0 - 0.85 * wear)).min(0.45 * sr), 0.8, sr);
        }
        for f in self.hp.iter_mut() {
            f.set(20.0 + 280.0 * wear, sr);
        }
        for f in self.crackle_tone.iter_mut() {
            f.set_q(3500.0, 1.2, sr);
        }
        for f in self.hiss_tone.iter_mut() {
            f.set(6000.0, sr);
        }
        self.pop_tone.set(900.0, sr);
        self.rumble.set(35.0, sr);
        let crackle = p[CRACKLE].clamp(0.0, 1.0);
        let pops = p[POPS].clamp(0.0, 1.0);
        let hiss = p[HISS].clamp(0.0, 1.0) * 0.02;
        let rumble = p[RUMBLE].clamp(0.0, 1.0) * 0.15;
        let wow = p[WOW].clamp(0.0, 1.0) * 0.0015 * sr;
        let width = p[WIDTH].clamp(0.0, 1.0);
        let mix = p[MIX].clamp(0.0, 1.0);
        // Clicks per second.
        let crackle_rate = crackle * crackle * 400.0 / sr;
        let pop_rate = pops * pops * 3.0 / sr;
        let click_decay = (-1.0 / (0.0003 * sr)).exp();
        let pop_decay = (-1.0 / (0.004 * sr)).exp();
        for i in 0..n {
            // 33 1/3 RPM: the hole is off centre once per turn.
            self.phase = super::wrap(self.phase + 0.555 / sr);
            let d = 4.0 + wow * (1.0 + (self.phase * TAU).sin());
            let (xl, xr) = (l[i], r[i]);
            let mid = 0.5 * (xl + xr);
            let side = 0.5 * (xl - xr) * width;
            let (xl2, xr2) = (mid + side, mid - side);
            self.lines[0].write(xl2);
            self.lines[1].write(xr2);
            for ch in 0..2 {
                if self.rng.unipolar() < crackle_rate {
                    self.click[ch] = self.rng.bipolar() * (0.05 + 0.25 * self.rng.unipolar());
                }
            }
            if self.rng.unipolar() < pop_rate {
                self.pop = 0.3 + 0.5 * self.rng.unipolar();
                self.pop_sign = if self.rng.unipolar() < 0.5 { -1.0 } else { 1.0 };
            }
            let pop = self.pop_tone.lp(self.pop * self.pop_sign);
            self.pop *= pop_decay;
            let rum = self.rumble.lp(self.rng.bipolar()) * rumble * 8.0;
            let mut out = [0.0f32; 2];
            for ch in 0..2 {
                let mut y = self.lines[ch].read(d);
                y = self.hp[ch].hp(self.lp[ch].tick(y).low);
                let c = self.crackle_tone[ch].tick(self.click[ch]).band * 2.0;
                self.click[ch] *= click_decay;
                let h = self.hiss_tone[ch].lp(self.rng.bipolar()) * hiss;
                out[ch] = y + c + pop + h + rum;
            }
            l[i] = xl * (1.0 - mix) + out[0] * mix;
            r[i] = xr * (1.0 - mix) + out[1] * mix;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_util::*;
    use super::*;

    #[test]
    fn it_crackles_on_silence_and_passes_music() {
        let mut v = Vinyl::new(48000.0);
        let mut p = crate::params::defaults(SPECS);
        p[CRACKLE] = 1.0;
        let mut l = vec![0.0; 48000];
        let mut r = l.clone();
        v.process(&p, &mut l, &mut r, &ctx());
        assert!(peak(&l) > 0.01 && peak(&l) < 2.0);
        let mut l = tone(440.0, 1.0, 0.5);
        let mut r = l.clone();
        v.process(&p, &mut l, &mut r, &ctx());
        assert!(rms(&l) > 0.2 && l.iter().all(|x| x.is_finite()));
    }
}

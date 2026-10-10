//! Rotary: a rotating speaker cabinet. Highs go to a spinning horn, lows to
//! a spinning drum; each turns the level up and down and moves the sound
//! nearer and farther (Doppler), heard by two microphones. The rotors speed
//! up and slow down with inertia, the horn faster than the drum.

use super::DelayLine;
use crate::dsp::effects::FxCtx;
use crate::dsp::{fast_tanh, Svf};
use crate::params::{ParamSpec, Unit};
use std::f32::consts::TAU;

pub const SPEED: usize = 0;
pub const ACCEL: usize = 1;
pub const SPREAD: usize = 2;
pub const BALANCE: usize = 3;
pub const DRIVE: usize = 4;
pub const MIX: usize = 5;

pub const SPEEDS: &[&str] = &["Stop", "Slow", "Fast"];

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::choice("Speed", "Rotor", SPEEDS, 1),
    ParamSpec::lin("Inertia", "Rotor", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Mic Spread", "Rotor", 0.0, 1.0, 0.7, Unit::Percent),
    ParamSpec::lin("Horn/Drum", "Mix", -1.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Drive", "Mix", 0.0, 1.0, 0.2, Unit::Percent),
    ParamSpec::lin("Mix", "Mix", 0.0, 1.0, 1.0, Unit::Percent),
];

/// Rotor speeds in Hz: (slow, fast) for horn and drum.
const HORN: (f32, f32) = (0.83, 6.7);
const DRUM: (f32, f32) = (0.67, 5.7);

pub struct Rotary {
    split: Svf,
    horn_line: DelayLine,
    drum_line: DelayLine,
    horn_rate: f32,
    drum_rate: f32,
    horn_phase: f32,
    drum_phase: f32,
}

impl Rotary {
    pub fn new(sr: f32) -> Self {
        let len = (sr * 0.01) as usize + 8;
        Self {
            split: Svf::new(),
            horn_line: DelayLine::new(len),
            drum_line: DelayLine::new(len),
            horn_rate: HORN.0,
            drum_rate: DRUM.0,
            horn_phase: 0.0,
            drum_phase: 0.25,
        }
    }

    pub fn process(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32], ctx: &FxCtx) {
        let n = l.len().min(r.len());
        let sr = ctx.sr;
        let speed = p[SPEED].round() as i32;
        let (horn_target, drum_target) = match speed {
            0 => (0.0, 0.0),
            2 => (HORN.1, DRUM.1),
            _ => (HORN.0, DRUM.0),
        };
        // Horn reaches its speed in about 0.4..2.4 s, the drum in 1.5..6 s.
        let inertia = p[ACCEL].clamp(0.0, 1.0);
        let horn_c = 1.0 / ((0.4 + 2.0 * inertia) * sr);
        let drum_c = 1.0 / ((1.5 + 4.5 * inertia) * sr);
        let spread = p[SPREAD].clamp(0.0, 1.0) * 0.25;
        let bal = p[BALANCE].clamp(-1.0, 1.0);
        let (horn_g, drum_g) = ((1.0 + bal).min(1.0), (1.0 - bal).min(1.0));
        let drive = p[DRIVE].clamp(0.0, 1.0);
        let pre = 1.0 + 6.0 * drive;
        let post = 1.0 / pre.sqrt();
        let mix = p[MIX].clamp(0.0, 1.0);
        self.split.set_q(800.0, 0.6, sr);
        let doppler = 0.0007 * sr;
        for i in 0..n {
            self.horn_rate += (horn_target - self.horn_rate) * horn_c * 4.0;
            self.drum_rate += (drum_target - self.drum_rate) * drum_c * 4.0;
            self.horn_phase = super::wrap(self.horn_phase + self.horn_rate / sr);
            self.drum_phase = super::wrap(self.drum_phase + self.drum_rate / sr);
            let (xl, xr) = (l[i], r[i]);
            let mono = 0.5 * (xl + xr);
            let x = if drive > 0.0 { fast_tanh(mono * pre) * post } else { mono };
            let bands = self.split.tick(x);
            let (low, high) = (bands.low, bands.high + bands.band);
            self.horn_line.write(high);
            self.drum_line.write(low);
            let mut out = [0.0f32; 2];
            for (ch, o) in out.iter_mut().enumerate() {
                let side = if ch == 0 { -spread } else { spread };
                let hp = (self.horn_phase + side) * TAU;
                let dp = (self.drum_phase + side) * TAU;
                // The horn's distance moves its sound in time and level.
                let horn = self.horn_line.read(doppler * (1.2 + hp.sin())) * (1.0 + 0.45 * hp.cos());
                let drum = self.drum_line.read(2.0) * (1.0 + 0.2 * dp.cos());
                *o = horn * horn_g * 0.7 + drum * drum_g * 0.8;
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
    fn fast_spins_the_level_around() {
        let mut ro = Rotary::new(48000.0);
        let mut p = crate::params::defaults(SPECS);
        p[SPEED] = 2.0;
        p[ACCEL] = 0.0;
        let mut l = tone(2000.0, 3.0, 0.5);
        let mut r = l.clone();
        for (cl, cr) in l.chunks_mut(512).zip(r.chunks_mut(512)) {
            ro.process(&p, cl, cr, &ctx());
        }
        assert!(l.iter().chain(&r).all(|x| x.is_finite()));
        // The level of the last second goes up and down several times.
        let env: Vec<f32> = l[96000..].chunks(480).map(rms).collect();
        let (lo, hi) = env.iter().fold((f32::MAX, 0.0f32), |(a, b), &v| (a.min(v), b.max(v)));
        assert!(hi > lo * 1.5, "{lo} {hi}");
        assert!(peak(&l) < 2.0);
    }
}

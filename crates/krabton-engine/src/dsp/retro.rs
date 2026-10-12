//! Retro machines: more drum machines from before and around the 808.
//!
//! - Rhythm Box: the preset boxes of the late sixties and seventies, all
//!   damped resonators and gated noise: a soft bass drum, a snare, a
//!   "chick" hat, a cymbal, bongos, claves, a rasping guiro and maracas.
//! - Hexagon: the electronic kits of the early eighties: sines that sweep
//!   down (the toms' "pew"), a kick with a big drop, a snare of tone and noise.
//! - Digital 80s: the first sampling drum machines: punchy sounds through an
//!   8-bit companded converter at a low rate, a gated snare, a clap.
//! - Pocket: a small battery rhythm machine: a short kick and snare, thin
//!   metallic hats and cymbal, a clap.
//!
//! Original code, not samples.

use super::drums::DrumVoice;
use super::{decay_coef, fast_tanh, FilterMode, Rng, Svf};
use std::f32::consts::TAU;

/// Six square oscillators for the small machines' metal (Hz).
const METAL: [f32; 6] = [370.0, 548.0, 663.0, 941.0, 972.0, 1440.0];

#[derive(Clone, Copy)]
pub struct RetroVoice {
    sr: f32,
    vel: f32,
    t: u32,
    ph: [f32; 6],
    f: [f32; 2],
    /// Pitch above the base (a multiplier above 1) and how fast it falls.
    sweep: f32,
    sweep_coef: f32,
    sweep_depth: f32,
    /// Tone, second tone and noise envelopes.
    env: [f32; 3],
    coef: [f32; 3],
    /// Noise held open this many samples before it fades (a gate).
    gate: u32,
    click: f32,
    click_coef: f32,
    filt: [Svf; 2],
    /// The guiro's rasp.
    rasp: f32,
    /// Digital: sample and hold at a low rate.
    crush: bool,
    hold: f32,
    hold_left: f32,
    active: bool,
}

impl Default for RetroVoice {
    fn default() -> Self {
        Self {
            sr: 48000.0,
            vel: 1.0,
            t: 0,
            ph: [0.0; 6],
            f: [0.0; 2],
            sweep: 0.0,
            sweep_coef: 0.0,
            sweep_depth: 0.0,
            env: [0.0; 3],
            coef: [0.0; 3],
            gate: 0,
            click: 0.0,
            click_coef: 0.0,
            filt: [Svf::new(); 2],
            rasp: 0.0,
            crush: false,
            hold: 0.0,
            hold_left: 0.0,
            active: false,
        }
    }
}

/// Moves an oscillator's phase on by `f` Hz; the new phase.
#[inline]
fn adv(p: &mut f32, sr: f32, f: f32) -> f32 {
    *p = (*p + f / sr).fract();
    *p
}

#[inline]
fn sine(p: f32) -> f32 {
    (p * TAU).sin()
}

#[inline]
fn square(p: f32) -> f32 {
    if p < 0.5 {
        1.0
    } else {
        -1.0
    }
}

/// An 8-bit companded converter: what the early sampling machines did to
/// make 8 bits sound like more.
#[inline]
fn eight_bit(x: f32) -> f32 {
    let mu = 255.0f32;
    let c = x.signum() * (1.0 + mu * x.abs().min(1.0)).ln() / (1.0 + mu).ln();
    let q = (c * 127.0).round() / 127.0;
    q.signum() * ((1.0 + mu).powf(q.abs()) - 1.0) / mu
}

impl RetroVoice {
    pub fn is_active(&self) -> bool {
        self.active
    }

    #[allow(clippy::too_many_arguments)]
    pub fn trigger(&mut self, voice: DrumVoice, vel: f32, tune: f32, decay: f32, tone: f32, a: f32, sr: f32) {
        use DrumVoice::*;
        *self = Self { hold: self.hold, ..Self::default() };
        self.sr = sr;
        self.vel = vel.clamp(0.0, 1.0);
        self.active = true;
        self.ph = [0.0, 0.13, 0.31, 0.47, 0.61, 0.83];
        self.env = [1.0, 1.0, 1.0];
        let d = decay.max(0.05);
        let dc = |s: f32| decay_coef(s, sr);
        let tone = tone.clamp(0.0, 1.0);
        self.click_coef = dc(0.003);
        match voice {
            RbKick => {
                self.f = [68.0 * tune, 0.0];
                self.coef = [dc(0.28 * d), 0.0, 0.0];
            }
            RbSnare => {
                self.f = [220.0 * tune, 0.0];
                self.coef = [dc(0.06 * d), 0.0, dc(0.16 * d)];
                self.filt[0].set_q(1800.0 + 2000.0 * tone, 0.9, sr);
            }
            RbHat | RbMaracas => {
                self.coef = [0.0, 0.0, dc(if voice == RbHat { 0.05 } else { 0.035 } * d)];
                self.filt[0].set(if voice == RbHat { 6500.0 } else { 8000.0 } * (0.8 + 0.4 * tone), 0.1, sr);
            }
            RbCymbal => {
                self.coef = [0.0, 0.0, dc(0.75 * d)];
                self.filt[0].set(4500.0 * (0.8 + 0.4 * tone), 0.1, sr);
                self.filt[1].set_q(9000.0, 0.8, sr);
            }
            RbBongoHigh | RbBongoLow => {
                self.f = [if voice == RbBongoHigh { 430.0 } else { 300.0 } * tune, 0.0];
                self.sweep = 0.06;
                self.sweep_coef = dc(0.02);
                self.coef = [dc(0.09 * d), 0.0, 0.0];
            }
            RbClaves => {
                self.f = [2480.0 * tune, 0.0];
                self.coef = [dc(0.035 * d), 0.0, 0.0];
            }
            RbGuiro => {
                self.coef = [0.0, 0.0, dc(0.22 * d)];
                self.filt[0].set_q(2800.0 * (0.8 + 0.4 * tone), 1.6, sr);
                self.f = [36.0, 0.0];
            }
            HxKick => {
                self.f = [52.0 * tune, 0.0];
                self.sweep = 3.5 * (0.5 + a);
                self.sweep_coef = dc(0.05);
                self.coef = [dc(0.4 * d), 0.0, 0.0];
                self.click = 0.6;
            }
            HxSnare => {
                self.f = [175.0 * tune, 0.0];
                self.sweep = 0.4;
                self.sweep_coef = dc(0.08);
                self.coef = [dc(0.22 * d), 0.0, dc(0.25 * d)];
                self.filt[0].set_q(2600.0 * (0.8 + 0.4 * tone), 0.7, sr);
                self.click = 0.5;
            }
            HxTomHigh | HxTomMid | HxTomLow => {
                let base = match voice {
                    HxTomHigh => 210.0,
                    HxTomMid => 155.0,
                    _ => 105.0,
                };
                self.f = [base * tune, 0.0];
                // The "pew": far above, and down.
                self.sweep = 0.4 + 2.0 * a;
                self.sweep_coef = dc(0.28);
                self.coef = [dc(0.6 * d), 0.0, dc(0.05)];
                self.filt[0].set_q(1500.0, 0.7, sr);
                self.click = 0.5;
            }
            DgKick => {
                self.f = [58.0 * tune, 0.0];
                self.sweep = 1.4;
                self.sweep_coef = dc(0.025);
                self.coef = [dc(0.42 * d), 0.0, 0.0];
                self.click = 0.8;
                self.crush = true;
            }
            DgSnare => {
                self.f = [190.0 * tune, 340.0 * tune];
                self.sweep = 0.2;
                self.sweep_coef = dc(0.02);
                self.coef = [dc(0.12 * d), dc(0.08 * d), dc(0.06)];
                // The gate: a burst of noise held open, then cut (a gated room).
                self.gate = ((0.06 + 0.25 * a) * d * sr) as u32;
                self.filt[0].set_q(4200.0 * (0.8 + 0.4 * tone), 0.5, sr);
                self.crush = true;
            }
            DgClap => {
                self.coef = [0.0, 0.0, dc(0.2 * d)];
                self.filt[0].set_q(1200.0 * (0.8 + 0.4 * tone), 1.4, sr);
                self.crush = true;
            }
            DgRim => {
                self.f = [1650.0 * tune, 480.0 * tune];
                self.coef = [dc(0.025 * d), dc(0.03 * d), dc(0.01)];
                self.filt[0].set_q(3500.0, 0.8, sr);
                self.crush = true;
            }
            DgHat | DgOpenHat => {
                self.coef = [dc(if voice == DgHat { 0.06 } else { 0.6 } * d), 0.0, dc(if voice == DgHat { 0.05 } else { 0.5 } * d)];
                self.filt[0].set(7000.0 * (0.8 + 0.4 * tone), 0.1, sr);
                self.filt[1].set_q(10500.0, 0.6, sr);
                self.crush = true;
            }
            DgTomHigh | DgTomLow => {
                self.f = [if voice == DgTomHigh { 180.0 } else { 120.0 } * tune, 0.0];
                self.sweep = 0.35;
                self.sweep_coef = dc(0.09);
                self.coef = [dc(0.38 * d), 0.0, dc(0.03)];
                self.filt[0].set_q(1200.0, 0.7, sr);
                self.click = 0.5;
                self.crush = true;
            }
            DgCowbell => {
                self.f = [587.0 * tune, 845.0 * tune];
                self.coef = [dc(0.28 * d), dc(0.28 * d), 0.0];
                self.filt[0].set_q(2000.0 * (0.8 + 0.4 * tone), 1.2, sr);
                self.crush = true;
            }
            PkKick => {
                self.f = [62.0 * tune, 0.0];
                self.sweep = 0.3;
                self.sweep_coef = dc(0.012);
                self.coef = [dc(0.18 * d), 0.0, 0.0];
                self.click = 0.4;
            }
            PkSnare => {
                self.f = [235.0 * tune, 0.0];
                self.coef = [dc(0.07 * d), 0.0, dc(0.14 * d)];
                self.filt[0].set(1600.0 * (0.8 + 0.4 * tone), 0.1, sr);
            }
            PkHat | PkOpenHat | PkCymbal => {
                let (len, hp, bp) = match voice {
                    PkHat => (0.04, 7000.0, 8500.0),
                    PkOpenHat => (0.32, 7000.0, 8500.0),
                    _ => (0.9, 5000.0, 6500.0),
                };
                self.f = [tune, 0.0];
                self.coef = [dc(len * d), 0.0, dc(len * d * 0.8)];
                self.filt[0].set(hp * (0.8 + 0.4 * tone), 0.1, sr);
                self.filt[1].set_q(bp * (0.8 + 0.4 * tone), 0.8, sr);
            }
            PkClap => {
                self.coef = [0.0, 0.0, dc(0.12 * d)];
                self.filt[0].set_q(1100.0 * (0.8 + 0.4 * tone), 1.2, sr);
            }
            _ => self.active = false,
        }
        self.sweep_depth = self.sweep;
    }

    #[inline]
    pub fn tick(&mut self, voice: DrumVoice, rng: &mut Rng) -> f32 {
        use DrumVoice::*;
        if !self.active {
            return 0.0;
        }
        let sr = self.sr;
        let t = self.t;
        self.t = self.t.saturating_add(1);
        let pitch = 1.0 + self.sweep;
        let n = rng.bipolar();
        let click = n * self.click;
        let y = match voice {
            RbKick | RbBongoHigh | RbBongoLow | RbClaves | PkKick | HxKick | HxTomHigh | HxTomMid | HxTomLow | DgKick | DgTomHigh | DgTomLow => {
                let body = sine(adv(&mut self.ph[0], sr, self.f[0] * pitch)) * self.env[0];
                let noise = if matches!(voice, HxTomHigh | HxTomMid | HxTomLow | DgTomHigh | DgTomLow) { self.filt[0].process(n, FilterMode::Band) * self.env[2] * 0.6 } else { 0.0 };
                body + noise + click * 0.3
            }
            RbSnare | PkSnare | HxSnare => {
                let body = sine(adv(&mut self.ph[0], sr, self.f[0] * pitch)) * self.env[0];
                let mode = if voice == PkSnare { FilterMode::High } else { FilterMode::Band };
                let noise = self.filt[0].process(n, mode) * self.env[2];
                body * 0.5 + noise * if voice == HxSnare { 1.2 } else { 1.0 } + click * 0.2
            }
            RbHat | RbMaracas => self.filt[0].process(n, FilterMode::High) * self.env[2],
            RbCymbal => {
                let h = self.filt[0].process(n, FilterMode::High);
                (h * 0.6 + self.filt[1].process(h, FilterMode::Band)) * self.env[2] * 0.8
            }
            RbGuiro => {
                // A rasp: the noise opens and shuts with each ridge of the gourd.
                self.rasp = (self.rasp + self.f[0] / sr).fract();
                let ridge = (1.0 - self.rasp).powi(3);
                self.filt[0].process(n, FilterMode::Band) * self.env[2] * ridge * 2.2
            }
            DgSnare => {
                let body = sine(adv(&mut self.ph[0], sr, self.f[0] * pitch)) * self.env[0] + sine(adv(&mut self.ph[1], sr, self.f[1] * pitch)) * self.env[1] * 0.5;
                let open = if t < self.gate { 1.0 } else { self.env[2] };
                body * 0.6 + self.filt[0].process(n, FilterMode::Band) * open * 0.9
            }
            DgClap | PkClap => {
                // Three quick claps and the tail.
                let ms = t as f32 * 1000.0 / sr;
                let burst = if ms < 30.0 { if (ms % 10.0) < 3.0 { 1.0 } else { 0.15 } } else { self.env[2] };
                self.filt[0].process(n, FilterMode::Band) * burst * 1.6
            }
            DgRim => sine(adv(&mut self.ph[0], sr, self.f[0])) * self.env[0] * 0.7 + sine(adv(&mut self.ph[1], sr, self.f[1])) * self.env[1] * 0.5 + self.filt[0].process(n, FilterMode::Band) * self.env[2],
            DgHat | DgOpenHat => {
                let mut m = 0.0;
                for (k, f) in METAL.iter().enumerate() {
                    m += square(adv(&mut self.ph[k], sr, f * 1.9));
                }
                let x = m / 6.0 * self.env[0] * 0.5 + n * self.env[2];
                let h = self.filt[0].process(x, FilterMode::High);
                self.filt[1].process(h, FilterMode::Band) + h * 0.3
            }
            DgCowbell => {
                let x = square(adv(&mut self.ph[0], sr, self.f[0])) + square(adv(&mut self.ph[1], sr, self.f[1]));
                self.filt[0].process(x * 0.5, FilterMode::Band) * self.env[0] * 1.5
            }
            PkHat | PkOpenHat | PkCymbal => {
                let mut m = 0.0;
                for (k, f) in METAL.iter().enumerate() {
                    m += square(adv(&mut self.ph[k], sr, f * self.f[0] * if voice == PkCymbal { 1.2 } else { 1.6 }));
                }
                let x = m / 6.0 * self.env[0] + n * 0.4 * self.env[2];
                let h = self.filt[0].process(x, FilterMode::High);
                self.filt[1].process(h, FilterMode::Band) * 1.4 + h * 0.4
            }
            _ => 0.0,
        };
        for (e, c) in self.env.iter_mut().zip(self.coef) {
            *e *= c;
        }
        self.sweep *= self.sweep_coef;
        self.click *= self.click_coef;
        let mut out = fast_tanh(y * 1.1);
        if self.crush {
            // The converter runs at about 28 kHz.
            self.hold_left -= 28000.0 / sr;
            if self.hold_left <= 0.0 {
                self.hold_left += 1.0;
                self.hold = eight_bit(out);
            }
            out = self.hold;
        }
        if t > (0.02 * sr) as u32 && self.t % 256 == 0 && self.env.iter().all(|e| *e < 1e-4) && t >= self.gate {
            self.active = false;
        }
        let _ = self.vel;
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_retro_voice_sounds_and_ends() {
        for v in DrumVoice::RETRO {
            let mut r = RetroVoice::default();
            let mut rng = Rng::new(9);
            r.trigger(v, 0.9, 1.0, 1.0, 0.5, 0.5, 48000.0);
            let x: Vec<f32> = (0..48000 * 3).map(|_| r.tick(v, &mut rng)).collect();
            assert!(x.iter().all(|s| s.is_finite() && s.abs() <= 1.0), "{v:?}");
            let peak = x[..4800].iter().fold(0.0f32, |m, s| m.max(s.abs()));
            assert!(peak > 0.05, "{v:?} {peak}");
            assert!(!r.is_active(), "{v:?} ends");
        }
    }

    #[test]
    fn the_hexagon_tom_sweeps_down() {
        let v = DrumVoice::HxTomMid;
        let mut r = RetroVoice::default();
        let mut rng = Rng::new(1);
        r.trigger(v, 0.9, 1.0, 1.0, 0.5, 0.5, 48000.0);
        let x: Vec<f32> = (0..48000).map(|_| r.tick(v, &mut rng)).collect();
        let crossings = |a: &[f32]| a.windows(2).filter(|w| w[0] < 0.0 && w[1] >= 0.0).count();
        assert!(crossings(&x[0..4800]) > crossings(&x[14400..19200]) + 5);
    }

    #[test]
    fn the_digital_voices_go_through_eight_bits() {
        // Few distinct levels in a quiet passage, as an 8-bit converter gives.
        let v = DrumVoice::DgTomLow;
        let mut r = RetroVoice::default();
        let mut rng = Rng::new(3);
        r.trigger(v, 0.9, 1.0, 1.0, 0.5, 0.5, 48000.0);
        let x: Vec<f32> = (0..48000).map(|_| r.tick(v, &mut rng)).collect();
        let mut levels: Vec<i64> = x[9000..12000].iter().map(|s| (s * 1e6) as i64).collect();
        levels.sort();
        levels.dedup();
        assert!(levels.len() < 200, "{}", levels.len());
    }
}

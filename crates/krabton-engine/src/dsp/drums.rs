//! Drums: a synthesized drum rack with optional per-pad samples.

use super::*;
use crate::project::DrumPad;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DrumVoice {
    Kick,
    Snare,
    ClosedHat,
    OpenHat,
    Clap,
    Tom,
    Rim,
    Cowbell,
}

impl DrumVoice {
    pub const ALL: [DrumVoice; 8] = [
        DrumVoice::Kick,
        DrumVoice::Snare,
        DrumVoice::ClosedHat,
        DrumVoice::OpenHat,
        DrumVoice::Clap,
        DrumVoice::Tom,
        DrumVoice::Rim,
        DrumVoice::Cowbell,
    ];

    pub fn name(self) -> &'static str {
        match self {
            DrumVoice::Kick => "Kick",
            DrumVoice::Snare => "Snare",
            DrumVoice::ClosedHat => "Closed Hat",
            DrumVoice::OpenHat => "Open Hat",
            DrumVoice::Clap => "Clap",
            DrumVoice::Tom => "Tom",
            DrumVoice::Rim => "Rim",
            DrumVoice::Cowbell => "Cowbell",
        }
    }

    /// General MIDI note for this voice, so MIDI drum controllers just work.
    pub fn gm_note(self) -> u8 {
        match self {
            DrumVoice::Kick => 36,
            DrumVoice::Rim => 37,
            DrumVoice::Snare => 38,
            DrumVoice::Clap => 39,
            DrumVoice::ClosedHat => 42,
            DrumVoice::Tom => 45,
            DrumVoice::OpenHat => 46,
            DrumVoice::Cowbell => 56,
        }
    }
}

pub fn kit_names() -> &'static [&'static str] {
    &["808 Krab", "909 Krab", "Lo-Fi Krab"]
}

pub fn kit(name: &str) -> Vec<DrumPad> {
    let order = [
        DrumVoice::Kick,
        DrumVoice::Rim,
        DrumVoice::Snare,
        DrumVoice::Clap,
        DrumVoice::ClosedHat,
        DrumVoice::Tom,
        DrumVoice::OpenHat,
        DrumVoice::Cowbell,
    ];
    let mut pads: Vec<DrumPad> = order.iter().map(|&v| DrumPad::new(v)).collect();
    for pad in pads.iter_mut() {
        match (name, pad.voice) {
            ("808 Krab", DrumVoice::Kick) => {
                pad.decay = 1.8;
                pad.tone = 0.15;
            }
            ("808 Krab", DrumVoice::Snare) => pad.tone = 0.35,
            ("808 Krab", DrumVoice::ClosedHat | DrumVoice::OpenHat) => pad.tone = 0.4,
            ("909 Krab", DrumVoice::Kick) => {
                pad.decay = 0.8;
                pad.tune = 2.0;
                pad.tone = 0.65;
            }
            ("909 Krab", DrumVoice::Snare) => {
                pad.tone = 0.75;
                pad.tune = 1.0;
            }
            ("909 Krab", DrumVoice::ClosedHat | DrumVoice::OpenHat) => pad.tone = 0.8,
            ("Lo-Fi Krab", _) => {
                pad.tune -= 3.0;
                pad.tone = (pad.tone - 0.25).max(0.0);
                pad.decay *= 0.8;
            }
            _ => {}
        }
    }
    pads
}

const MAX_PADS: usize = 16;
const METAL: [f32; 6] = [205.3, 304.4, 369.6, 522.7, 540.0, 800.0];

#[derive(Clone, Copy)]
struct PadVoice {
    active: bool,
    t: u32,
    vel: f32,
    phase: [f32; 6],
    amp: f32,
    amp_coef: f32,
    env2: f32,
    env2_coef: f32,
    env3: f32,
    env3_coef: f32,
    f1: Svf,
    f2: Svf,
    rng: Rng,
    choke: f32,
    choking: bool,
    sample_pos: f64,
}

impl PadVoice {
    fn new(seed: u32) -> Self {
        Self {
            active: false,
            t: 0,
            vel: 0.0,
            phase: [0.0; 6],
            amp: 0.0,
            amp_coef: 0.0,
            env2: 0.0,
            env2_coef: 0.0,
            env3: 0.0,
            env3_coef: 0.0,
            f1: Svf::new(),
            f2: Svf::new(),
            rng: Rng::new(seed),
            choke: 1.0,
            choking: false,
            sample_pos: 0.0,
        }
    }

    fn trigger(&mut self, pad: &DrumPad, vel: f32, sr: f32) {
        self.active = true;
        self.t = 0;
        self.vel = vel;
        self.choke = 1.0;
        self.choking = false;
        self.sample_pos = 0.0;
        self.amp = 1.0;
        self.env2 = 1.0;
        self.env3 = 1.0;
        self.f1.reset();
        self.f2.reset();
        for (i, p) in self.phase.iter_mut().enumerate() {
            *p = if i % 2 == 0 { 0.0 } else { 0.37 * i as f32 % 1.0 };
        }
        let d = pad.decay.max(0.05);
        let tr = 2f32.powf(pad.tune / 12.0);
        let tone = pad.tone;
        self.env3_coef = decay_coef(0.004, sr);
        match pad.voice {
            DrumVoice::Kick => {
                self.amp_coef = decay_coef(0.5 * d, sr);
                self.env2_coef = decay_coef(0.07, sr);
                self.env3_coef = decay_coef(0.005, sr);
            }
            DrumVoice::Snare => {
                self.amp_coef = decay_coef(0.28 * d, sr);
                self.env2_coef = decay_coef(0.14 * d, sr);
                self.f1.set(1200.0 + tone * 3000.0, 0.0, sr);
            }
            DrumVoice::ClosedHat | DrumVoice::OpenHat => {
                let len = if pad.voice == DrumVoice::ClosedHat { 0.09 } else { 0.7 };
                self.amp_coef = decay_coef(len * d, sr);
                self.env2_coef = 0.0;
                self.f1.set_q((8000.0 + tone * 4000.0) * tr.sqrt(), 1.2, sr);
                self.f2.set(6500.0, 0.0, sr);
            }
            DrumVoice::Clap => {
                self.amp_coef = decay_coef(0.4 * d, sr);
                self.env2_coef = 0.0;
                self.f1.set_q(1100.0 * tr * (0.8 + tone * 0.6), 2.0, sr);
            }
            DrumVoice::Tom => {
                self.amp_coef = decay_coef(0.55 * d, sr);
                self.env2_coef = decay_coef(0.18, sr);
            }
            DrumVoice::Rim => {
                self.amp_coef = decay_coef(0.06 * d, sr);
                self.env2_coef = decay_coef(0.02, sr);
                self.f1.set_q(3200.0 * tr, 3.0, sr);
            }
            DrumVoice::Cowbell => {
                self.amp_coef = decay_coef(0.45 * d, sr);
                self.env2_coef = decay_coef(0.04, sr);
                self.f1.set_q(2640.0 * tr, 2.5, sr);
            }
        }
        if let Some(s) = &pad.sample {
            // Samples fade out over their length times the decay setting.
            let secs = s.duration() as f32 * d.min(1.0);
            self.amp_coef = if d >= 1.0 { 1.0 } else { decay_coef(secs.max(0.01), sr) };
        }
    }

    #[inline]
    fn tick(&mut self, pad: &DrumPad, tr: f32, sr: f32) -> f32 {
        let tone = pad.tone;
        let out = match pad.voice {
            DrumVoice::Kick => {
                let f0 = 44.0 * tr;
                let f1 = (150.0 + tone * 140.0) * tr;
                let f = f0 + (f1 - f0) * self.env2;
                self.phase[0] = wrap_phase(self.phase[0] + f / sr);
                let body = (self.phase[0] * TAU).sin() * self.amp;
                let click = self.rng.bipolar() * self.env3 * (0.1 + tone * 0.5);
                fast_tanh((body + click) * 1.6) * 0.9
            }
            DrumVoice::Snare => {
                self.phase[0] = wrap_phase(self.phase[0] + 185.0 * tr / sr);
                self.phase[1] = wrap_phase(self.phase[1] + 330.0 * tr / sr);
                let body = ((self.phase[0] * TAU).sin() * 0.6 + (self.phase[1] * TAU).sin() * 0.4) * self.env2;
                let n = self.rng.bipolar();
                let nb = self.f1.tick(n);
                let noise = (nb.high * 0.7 + nb.band * 0.5) * self.amp;
                body * (0.8 - tone * 0.4) + noise * (0.45 + tone * 0.55)
            }
            DrumVoice::ClosedHat | DrumVoice::OpenHat => {
                let mut sum = 0.0;
                for (k, f) in METAL.iter().enumerate() {
                    self.phase[k] = wrap_phase(self.phase[k] + f * 1.4 * tr / sr);
                    sum += if self.phase[k] < 0.5 { 1.0 } else { -1.0 };
                }
                let x = sum * (1.0 - tone * 0.5) / 6.0 + self.rng.bipolar() * (0.25 + tone * 0.5);
                let y = self.f1.process(x, FilterMode::Band);
                let y = self.f2.process(y, FilterMode::High);
                y * self.amp * 1.8
            }
            DrumVoice::Clap => {
                let ts = self.t as f32 / sr;
                let burst = if ts < 0.032 {
                    let tb = ts % 0.0105;
                    (-tb / 0.0022).exp()
                } else {
                    0.0
                };
                let env = burst.max(self.amp * 0.55);
                let n = self.rng.bipolar();
                self.f1.process(n, FilterMode::Band) * env * 2.2
            }
            DrumVoice::Tom => {
                let f = (95.0 + 75.0 * self.env2) * tr * (1.0 + tone * 0.6);
                self.phase[0] = wrap_phase(self.phase[0] + f / sr);
                let body = (self.phase[0] * TAU).sin() * self.amp;
                body + self.rng.bipolar() * self.env3 * 0.2
            }
            DrumVoice::Rim => {
                self.phase[0] = wrap_phase(self.phase[0] + 1700.0 * tr / sr);
                let ping = (self.phase[0] * TAU).sin() * self.env2;
                let n = self.f1.process(self.rng.bipolar(), FilterMode::Band);
                (ping * 0.6 + n * (0.6 + tone)) * self.amp
            }
            DrumVoice::Cowbell => {
                self.phase[0] = wrap_phase(self.phase[0] + 540.0 * tr / sr);
                self.phase[1] = wrap_phase(self.phase[1] + 800.0 * tr / sr);
                let a = if self.phase[0] < 0.5 { 1.0 } else { -1.0 };
                let b = if self.phase[1] < 0.5 { 1.0 } else { -1.0 };
                let y = self.f1.process((a + b) * 0.5, FilterMode::Band);
                y * (self.amp * 0.6 + self.env2 * 0.5) * 1.5
            }
        };
        self.amp *= self.amp_coef;
        self.env2 *= self.env2_coef;
        self.env3 *= self.env3_coef;
        self.t = self.t.saturating_add(1);
        if self.amp < 1e-4 && self.env2 < 1e-4 {
            self.active = false;
        }
        out
    }
}

pub struct DrumState {
    voices: [PadVoice; MAX_PADS],
}

impl Default for DrumState {
    fn default() -> Self {
        Self::new()
    }
}

impl DrumState {
    pub fn new() -> Self {
        let mut voices = [PadVoice::new(1); MAX_PADS];
        for (i, v) in voices.iter_mut().enumerate() {
            v.rng = Rng::new(0xC0FF_EE00 ^ (i as u32 * 104_729 + 7));
        }
        Self { voices }
    }

    pub fn note_on(&mut self, pads: &[DrumPad], note: u8, vel: f32, sr: f32) {
        for (i, pad) in pads.iter().enumerate().take(MAX_PADS) {
            if pad.note != note {
                continue;
            }
            if pad.voice == DrumVoice::ClosedHat {
                for (j, other) in pads.iter().enumerate().take(MAX_PADS) {
                    if other.voice == DrumVoice::OpenHat && self.voices[j].active {
                        self.voices[j].choking = true;
                    }
                }
            }
            self.voices[i].trigger(pad, vel, sr);
        }
    }

    pub fn all_off(&mut self) {
        for v in self.voices.iter_mut() {
            if v.active {
                v.choking = true;
            }
        }
    }

    pub fn render(&mut self, pads: &[DrumPad], l: &mut [f32], r: &mut [f32], sr: f32) {
        let n = l.len().min(r.len());
        let choke_coef = decay_coef(0.015, sr);
        for (i, pad) in pads.iter().enumerate().take(MAX_PADS) {
            let v = &mut self.voices[i];
            if !v.active {
                continue;
            }
            let vel_gain = v.vel.powf(1.3).max(0.02);
            let gain = db_to_gain(pad.level_db) * vel_gain * 0.32;
            let (pl, pr) = pan_gains(pad.pan);
            let tr = 2f32.powf(pad.tune / 12.0);
            if let Some(s) = &pad.sample {
                let rate = tr as f64 * s.sample_rate as f64 / sr as f64;
                let frames = s.frames() as f64;
                for i in 0..n {
                    if v.sample_pos >= frames {
                        v.active = false;
                        break;
                    }
                    let (sl, sr2) = s.read(v.sample_pos);
                    let g = gain * v.amp * v.choke;
                    l[i] += sl * g * pl;
                    r[i] += sr2 * g * pr;
                    v.sample_pos += rate;
                    v.amp *= v.amp_coef;
                    if v.choking {
                        v.choke *= choke_coef;
                    }
                    if v.amp < 1e-4 || v.choke < 1e-4 {
                        v.active = false;
                        break;
                    }
                }
                continue;
            }
            for i in 0..n {
                let y = v.tick(pad, tr, sr) * gain * v.choke;
                l[i] += y * pl;
                r[i] += y * pr;
                if v.choking {
                    v.choke *= choke_coef;
                    if v.choke < 1e-4 {
                        v.active = false;
                    }
                }
                if !v.active {
                    break;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_pad_makes_sound_and_ends() {
        let pads = kit("909 Krab");
        let mut d = DrumState::new();
        for pad in &pads {
            d.note_on(&pads, pad.note, 1.0, 48000.0);
        }
        let mut l = vec![0.0; 48000];
        let mut r = vec![0.0; 48000];
        d.render(&pads, &mut l, &mut r, 48000.0);
        let peak = l.iter().fold(0.0f32, |m, v| m.max(v.abs()));
        assert!(peak > 0.1 && peak < 4.0, "peak {peak}");
        for _ in 0..10 {
            d.render(&pads, &mut l, &mut r, 48000.0);
        }
        assert!(d.voices.iter().all(|v| !v.active));
    }
}

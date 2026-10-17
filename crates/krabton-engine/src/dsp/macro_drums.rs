//! Macro drums: twelve compact drum models, each played with the same few
//! macros (Decay, Tone, Sweep, Noise, FM, Drive): kick, snare, hat, clap,
//! tom, rim, FM, noise, cowbell, cymbal, shaker and clave. A port of the core
//! of Bogie for VCV Rack by Pierre-Emmanuel Surga (MIT; notice below), with
//! its squares band-limited and a short fade where it stopped dead.
//!
//! MIT License, Copyright (c) 2026 Pierre-Emmanuel Surga. Permission is
//! hereby granted, free of charge, to any person obtaining a copy of this
//! software and associated documentation files (the "Software"), to deal in
//! the Software without restriction, including without limitation the rights
//! to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
//! copies of the Software, and to permit persons to whom the Software is
//! furnished to do so, subject to the following conditions: The above
//! copyright notice and this permission notice shall be included in all
//! copies or substantial portions of the Software. THE SOFTWARE IS PROVIDED
//! "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT
//! NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR
//! PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT
//! HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN
//! ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN
//! CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

use std::f32::consts::PI;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Model {
    Kick,
    Snare,
    Hat,
    Clap,
    Tom,
    Rim,
    Fm,
    Noise,
    Cowbell,
    Cymbal,
    Shaker,
    Clave,
}

/// The macros, 0 to 1.
#[derive(Clone, Copy, Debug)]
pub struct Macros {
    pub decay: f32,
    pub tone: f32,
    pub sweep: f32,
    pub noise: f32,
    pub fm: f32,
    pub drive: f32,
}

impl Default for Macros {
    fn default() -> Self {
        Self { decay: 72.0 / 255.0, tone: 128.0 / 255.0, sweep: 0.5, noise: 0.5, fm: 0.5, drive: 0.0 }
    }
}

/// The level each model is calibrated to.
const MODEL_GAIN: [f32; 12] = [1.335575, 1.253229, 1.202260, 1.352630, 1.331551, 1.335482, 1.335162, 1.286303, 1.332512, 1.084859, 1.337063, 1.329288];

/// Length (s) at a Decay setting.
pub fn duration(model: Model, decay: f32) -> f32 {
    let d = decay.clamp(0.0, 1.0);
    0.018 + d * d * if model == Model::Hat { 1.55 } else { 2.0 }
}

/// The Decay setting for a length (s).
pub fn decay_for(model: Model, secs: f32) -> f32 {
    let k = if model == Model::Hat { 1.55 } else { 2.0 };
    ((secs - 0.018).max(0.0) / k).sqrt().clamp(0.0, 1.0)
}

fn poly_blep(t: f32, dt: f32) -> f32 {
    if t < dt {
        let t = t / dt;
        t + t - t * t - 1.0
    } else if t > 1.0 - dt {
        let t = (t - 1.0) / dt;
        t * t + t + t + 1.0
    } else {
        0.0
    }
}

#[derive(Clone, Copy)]
pub struct MacroVoice {
    model: Model,
    m: Macros,
    sr: f32,
    freq: f32,
    age: f32,
    duration: f32,
    noise_low: f32,
    noise_k: f32,
    phase: [f32; 8],
    rng: u32,
    vel: f32,
    active: bool,
}

impl Default for MacroVoice {
    fn default() -> Self {
        Self { model: Model::Kick, m: Macros::default(), sr: 48000.0, freq: 110.0, age: 0.0, duration: 0.0, noise_low: 0.0, noise_k: 0.0, phase: [0.0; 8], rng: 0x4352_5452, vel: 1.0, active: false }
    }
}

impl MacroVoice {
    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn trigger(&mut self, model: Model, freq: f32, m: Macros, vel: f32, sr: f32) {
        self.model = model;
        self.m = Macros {
            decay: m.decay.clamp(0.0, 1.0),
            tone: m.tone.clamp(0.0, 1.0),
            sweep: m.sweep.clamp(0.0, 1.0),
            noise: m.noise.clamp(0.0, 1.0),
            fm: m.fm.clamp(0.0, 1.0),
            drive: m.drive.clamp(0.0, 1.0),
        };
        self.sr = sr.max(1000.0);
        self.freq = freq.clamp(10.0, self.sr * 0.45);
        self.age = 0.0;
        self.noise_low = 0.0;
        self.phase = [0.0; 8];
        self.duration = duration(model, self.m.decay);
        // The original's noise filter, as it is at 48 kHz.
        let k48 = 0.015 + self.m.tone * 0.25;
        self.noise_k = 1.0 - (1.0 - k48).powf(48000.0 / self.sr);
        self.vel = vel.clamp(0.0, 1.0);
        self.active = true;
    }

    fn random(&mut self) -> f32 {
        self.rng = self.rng.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (self.rng >> 8) as f32 * (1.0 / 8_388_608.0) - 1.0
    }

    /// Sine (0), triangle (1) or a band-limited square (2).
    fn osc(&mut self, freq: f32, shape: u8, slot: usize) -> f32 {
        let dt = (freq / self.sr).clamp(0.0, 0.49);
        let p = &mut self.phase[slot];
        *p += dt;
        *p -= p.floor();
        let ph = *p;
        match shape {
            1 => 1.0 - 4.0 * (ph - 0.5).abs(),
            2 => {
                let mut y = if ph < 0.5 { 1.0 } else { -1.0 };
                y += poly_blep(ph, dt);
                y -= poly_blep((ph + 0.5).fract(), dt);
                y
            }
            _ => (2.0 * PI * ph).sin(),
        }
    }

    pub fn tick(&mut self) -> f32 {
        if !self.active {
            return 0.0;
        }
        if self.age >= self.duration {
            self.active = false;
            return 0.0;
        }
        let Macros { tone, sweep, noise, fm, drive, .. } = self.m;
        let age = self.age;
        let f0 = self.freq;
        let progress = age / self.duration;
        let fast = (-age / (0.006 + 0.050 * (1.0 - tone))).exp();
        let envelope = (-5.5 * progress).exp();
        let n = self.random();
        self.noise_low += (n - self.noise_low) * self.noise_k;
        let bright = n - self.noise_low;
        let sample = match self.model {
            Model::Kick => {
                let f = f0 * (1.0 + sweep * 7.0 * fast);
                self.osc(f, 0, 0) * 0.90 + self.osc(f * 2.0, 1, 1) * fm * 0.30 + bright * noise * 0.45 * (-age / 0.006).exp()
            }
            Model::Snare => {
                let body = self.osc(f0 * (0.65 + tone * 1.05) * (1.0 + sweep * 1.1 * fast), 1, 0) * (1.0 - noise * 0.75);
                body + self.osc(f0 * (1.20 + fm * 1.50), 0, 1) * (0.12 + fm * 0.58) + bright * noise * 0.95
            }
            Model::Hat | Model::Cymbal => {
                let cym = self.model == Model::Cymbal;
                let ratios = if cym { [1.31, 1.79, 2.41, 3.16, 4.07, 5.23] } else { [1.18, 1.56, 2.17, 2.87, 3.73, 4.61] };
                let base = if cym { (250.0 + tone * 360.0 + f0 * 0.45) * (1.0 + sweep * 0.90 * fast) } else { (180.0 + tone * 260.0 + f0 * 0.30) * (1.0 + sweep * 0.75 * fast) };
                let mut metal = 0.0;
                for (i, r) in ratios.iter().enumerate() {
                    let fi = i as f32;
                    let (spread, tau, level) = if cym { (0.18, 0.10 + fi * 0.075 + tone * 0.70, 0.05 + fm * 0.06) } else { (0.20, 0.025 + fi * 0.020 + tone * 0.070, 0.06 + fm * 0.06) };
                    metal += self.osc(base * (r + fm * (fi + 1.0) * spread), 2, i) * (-age / tau).exp() * level;
                }
                metal + if cym { bright * (0.10 + noise * 0.90) } else { bright * noise * 1.25 }
            }
            Model::Clap => {
                let spacing = 0.010 + sweep * 0.018;
                let burst = if age < 0.010 || (age > spacing && age < spacing + 0.010) || (age > spacing * 2.0 && age < spacing * 2.0 + 0.010) { 1.0 } else { 0.55 };
                bright * (0.25 + noise * 0.75) * burst + self.osc(f0 * (2.0 + tone * 8.0), 2, 0) * fm * 0.55 * fast
            }
            Model::Tom => self.osc(f0 * (0.90 + tone * 0.35) * (1.0 + sweep * 1.3 * fast), 0, 0) + self.osc(f0 * (1.6 + fm * 1.4), 0, 1) * fm * 0.55 + bright * noise * 0.55 * fast,
            Model::Rim => self.osc(f0 * (0.85 + tone * 0.45) * (1.0 + sweep * 0.25), 1, 0) * 0.70 + self.osc(f0 * (1.7 + fm * 2.5), 0, 1) * (0.10 + fm * 0.55) + bright * noise * 0.50 * fast,
            Model::Fm => {
                let ratio = 0.25 * 2f32.powf(tone * 2.5);
                let modulation = self.osc(f0 * ratio, 0, 1) * fm * 9.0 * fast;
                self.osc(f0 * (1.0 + sweep * 2.0 * fast + modulation), 0, 0) * 0.85 + bright * noise * 0.45
            }
            Model::Noise => (bright * (1.0 - sweep * 0.75) + self.noise_low * sweep * 0.75) * (0.2 + noise * 0.8) + self.osc(f0 * (1.0 + tone * 5.0), 0, 0) * fm * 0.22,
            Model::Cowbell => {
                let modulation = self.osc(f0 * 2.37, 0, 2) * fm * 1.5;
                let pitch = 1.0 + sweep * 0.80 * fast;
                self.osc(f0 * pitch * (1.0 + modulation), 2, 0) * 0.55 + self.osc(f0 * pitch * (1.39 + tone * 0.18 + modulation * 0.42), 2, 1) * 0.45 + bright * noise * 0.12
            }
            Model::Shaker => bright * (0.28 + noise * 0.72) * (0.45 + 0.55 * (age * (28.0 + tone * 75.0 + sweep * 180.0)).sin()) + self.osc(f0 * (4.0 + tone * 6.0), 2, 0) * fm * 0.16 * fast,
            Model::Clave => self.osc(f0 * (2.2 + tone * 1.8) * (1.0 + sweep * 1.5 * fast), 1, 0) * 0.85 + self.osc(f0 * (3.7 + fm * 1.3), 0, 1) * 0.25 + bright * noise * 0.20 * fast,
        };
        let d = 1.0 + drive * 5.5;
        self.age += 1.0 / self.sr;
        // (a 4 ms fade at the end, where the original stops dead)
        let fade = ((self.duration - age) / 0.004).clamp(0.0, 1.0);
        (sample * d).tanh() / d.tanh() * envelope * MODEL_GAIN[self.model as usize] * fade * (0.4 + 0.6 * self.vel)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [Model; 12] = [Model::Kick, Model::Snare, Model::Hat, Model::Clap, Model::Tom, Model::Rim, Model::Fm, Model::Noise, Model::Cowbell, Model::Cymbal, Model::Shaker, Model::Clave];

    #[test]
    fn every_model_sounds_and_ends_when_it_should() {
        for model in ALL {
            let mut v = MacroVoice::default();
            let freq = if model == Model::Kick { 55.0 } else { 220.0 };
            v.trigger(model, freq, Macros::default(), 1.0, 44100.0);
            let n = (duration(model, Macros::default().decay) * 44100.0) as usize;
            let out: Vec<f32> = (0..n + 10).map(|_| v.tick()).collect();
            let rms = (out[..n / 2].iter().map(|x| x * x).sum::<f32>() / (n / 2) as f32).sqrt();
            assert!(rms > 0.02, "{model:?}: {rms}");
            assert!(out.iter().all(|x| x.is_finite() && x.abs() < 2.0), "{model:?}");
            assert!(!v.is_active(), "{model:?}");
        }
    }

    #[test]
    fn decay_maps_both_ways() {
        for secs in [0.05, 0.2, 0.9] {
            assert!((duration(Model::Kick, decay_for(Model::Kick, secs)) - secs).abs() < 1e-4);
        }
    }
}

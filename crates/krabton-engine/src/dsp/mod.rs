//! DSP building blocks plus the instruments and effects built from them.

pub mod acoustic;
pub mod analog;
pub mod bassline;
pub mod biquad;
pub mod chime_sim;
pub mod lens_comp;
pub mod lens_eq;
pub mod lens_limiter;
pub mod machines;
pub mod braids_synth;
pub mod drums;
pub mod elements_synth;
pub mod facet;
pub mod coast;
pub mod crimson;
pub mod silicon;
pub mod drone;
pub mod dx7;
pub mod fft;
pub mod fm;
pub mod fx;
pub mod guitar;
pub mod macro_osc;
pub mod monk;
pub mod mono101;
pub mod effects;
pub mod organ;
pub mod reverb;
pub mod real_drums;
pub mod retro;
pub mod rings_synth;
pub mod sampler;
pub mod sf2;
pub mod street_organ;
pub mod shaker_synth;
pub mod shakers;
pub mod stk_mallets;
pub mod stk_strings;
pub mod stk_synth;
pub mod stk_voice;
pub mod stk_winds;
pub mod synth;
pub mod waves;
pub mod wind_chimes;
pub mod beads_synth;
pub mod birds;
pub mod nature;
pub mod pan_flute;
pub mod free_reed;
pub mod harmonica;
pub mod melodica;
pub mod calculator;
pub mod chip;
pub mod console;
pub mod handheld;
pub mod micro;
pub mod warp;
pub mod hybrid;
pub mod oidos;
pub mod quartet;
pub mod v2;
pub mod peaks;
pub mod peaks_tables;
pub mod macro_drums;
pub mod circuit_kick;
pub mod chip_drums;
pub mod chipfm;
pub mod tides;
pub mod theremin;

use std::f32::consts::PI;

pub const TAU: f32 = 2.0 * PI;

#[inline]
pub fn db_to_gain(db: f32) -> f32 {
    if db <= -90.0 {
        0.0
    } else {
        10f32.powf(db / 20.0)
    }
}

#[inline]
pub fn gain_to_db(g: f32) -> f32 {
    if g <= 1e-6 {
        -120.0
    } else {
        20.0 * g.log10()
    }
}

#[inline]
pub fn midi_to_freq(note: f32) -> f32 {
    440.0 * 2f32.powf((note - 69.0) / 12.0)
}

/// Rational tanh approximation, good enough for saturation.
#[inline]
pub fn fast_tanh(x: f32) -> f32 {
    if x > 3.0 {
        1.0
    } else if x < -3.0 {
        -1.0
    } else {
        let x2 = x * x;
        x * (27.0 + x2) / (27.0 + 9.0 * x2)
    }
}

/// Balance-law pan: unity at center, one side fades out.
#[inline]
pub fn pan_gains(pan: f32) -> (f32, f32) {
    let p = pan.clamp(-1.0, 1.0);
    ((1.0 - p).min(1.0), (1.0 + p).min(1.0))
}

/// Small, fast xorshift noise source.
#[derive(Clone, Copy, Debug)]
pub struct Rng(u32);

impl Rng {
    pub const fn new(seed: u32) -> Self {
        Self(if seed == 0 { 0x9E37_79B9 } else { seed })
    }

    #[inline]
    pub fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        x
    }

    /// Uniform in [-1, 1).
    #[inline]
    pub fn bipolar(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 * (2.0 / 16_777_216.0) - 1.0
    }

    /// Uniform in [0, 1).
    #[inline]
    pub fn unipolar(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 * (1.0 / 16_777_216.0)
    }
}

/// Time constant (seconds to fall ~40 dB) to a per-sample multiplier.
#[inline]
pub fn decay_coef(seconds: f32, sr: f32) -> f32 {
    (-4.6 / (seconds.max(0.0005) * sr)).exp()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

#[derive(Clone, Copy, Debug)]
pub struct AdsrCoefs {
    attack_inc: f32,
    decay: f32,
    sustain: f32,
    release: f32,
}

impl AdsrCoefs {
    /// `rate` is the rate at which `next` will be called (usually the sample rate).
    pub fn new(a: f32, d: f32, s: f32, r: f32, rate: f32) -> Self {
        Self {
            attack_inc: 1.0 / (a.max(0.0005) * rate).max(1.0),
            decay: decay_coef(d, rate),
            sustain: s.clamp(0.0, 1.0),
            release: decay_coef(r, rate),
        }
    }
}

/// Linear-attack, exponential decay/release envelope.
#[derive(Clone, Copy, Debug)]
pub struct Adsr {
    pub stage: Stage,
    pub level: f32,
}

impl Default for Adsr {
    fn default() -> Self {
        Self::new()
    }
}

impl Adsr {
    pub const fn new() -> Self {
        Self { stage: Stage::Idle, level: 0.0 }
    }

    /// Restarts the attack from the current level (no click on retrigger).
    pub fn trigger(&mut self) {
        self.stage = Stage::Attack;
    }

    pub fn release(&mut self) {
        if self.stage != Stage::Idle {
            self.stage = Stage::Release;
        }
    }

    pub fn reset(&mut self) {
        self.stage = Stage::Idle;
        self.level = 0.0;
    }

    pub fn is_active(&self) -> bool {
        self.stage != Stage::Idle
    }

    #[inline]
    pub fn next(&mut self, c: &AdsrCoefs) -> f32 {
        match self.stage {
            Stage::Idle => {}
            Stage::Attack => {
                self.level += c.attack_inc;
                if self.level >= 1.0 {
                    self.level = 1.0;
                    self.stage = Stage::Decay;
                }
            }
            Stage::Decay => {
                self.level = c.sustain + (self.level - c.sustain) * c.decay;
                if (self.level - c.sustain).abs() < 1e-4 {
                    self.level = c.sustain;
                    self.stage = Stage::Sustain;
                }
            }
            Stage::Sustain => self.level = c.sustain,
            Stage::Release => {
                self.level *= c.release;
                if self.level < 1e-4 {
                    self.level = 0.0;
                    self.stage = Stage::Idle;
                }
            }
        }
        self.level
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilterMode {
    Low,
    High,
    Band,
    Notch,
}

impl FilterMode {
    pub fn from_index(i: f32) -> Self {
        match i.round() as i32 {
            1 => FilterMode::High,
            2 => FilterMode::Band,
            3 => FilterMode::Notch,
            _ => FilterMode::Low,
        }
    }
}

/// Topology-preserving state variable filter (Simper/Cytomic).
#[derive(Clone, Copy, Debug, Default)]
pub struct Svf {
    ic1: f32,
    ic2: f32,
    a1: f32,
    a2: f32,
    a3: f32,
    k: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct SvfOut {
    pub low: f32,
    pub band: f32,
    pub high: f32,
}

impl Svf {
    pub fn new() -> Self {
        let mut f = Self::default();
        f.set(1000.0, 0.0, 48000.0);
        f
    }

    /// `res` in 0..1, where 1 is close to self-oscillation.
    pub fn set(&mut self, cutoff: f32, res: f32, sr: f32) {
        let fc = cutoff.clamp(10.0, sr * 0.49);
        let g = (PI * fc / sr).tan();
        let k = 2.0 - 1.97 * res.clamp(0.0, 1.0);
        self.set_gk(g, k);
    }

    /// Butterworth-style Q for crossovers.
    pub fn set_q(&mut self, cutoff: f32, q: f32, sr: f32) {
        let fc = cutoff.clamp(10.0, sr * 0.49);
        let g = (PI * fc / sr).tan();
        self.set_gk(g, 1.0 / q.max(0.05));
    }

    fn set_gk(&mut self, g: f32, k: f32) {
        self.k = k;
        self.a1 = 1.0 / (1.0 + g * (g + k));
        self.a2 = g * self.a1;
        self.a3 = g * self.a2;
    }

    pub fn reset(&mut self) {
        self.ic1 = 0.0;
        self.ic2 = 0.0;
    }

    #[inline]
    pub fn tick(&mut self, v0: f32) -> SvfOut {
        let v3 = v0 - self.ic2;
        let v1 = self.a1 * self.ic1 + self.a2 * v3;
        let v2 = self.ic2 + self.a2 * self.ic1 + self.a3 * v3;
        self.ic1 = 2.0 * v1 - self.ic1;
        self.ic2 = 2.0 * v2 - self.ic2;
        // Guard against blow-ups from denormals / extreme modulation.
        if !self.ic1.is_finite() || !self.ic2.is_finite() {
            self.reset();
        }
        SvfOut { low: v2, band: v1, high: v0 - self.k * v1 - v2 }
    }

    #[inline]
    pub fn process(&mut self, x: f32, mode: FilterMode) -> f32 {
        let o = self.tick(x);
        match mode {
            FilterMode::Low => o.low,
            FilterMode::High => o.high,
            FilterMode::Band => o.band,
            FilterMode::Notch => o.low + o.high,
        }
    }
}

/// One-pole lowpass, handy for damping and smoothing.
#[derive(Clone, Copy, Debug, Default)]
pub struct OnePole {
    pub z: f32,
    pub a: f32,
}

impl OnePole {
    pub fn set(&mut self, cutoff: f32, sr: f32) {
        self.a = 1.0 - (-TAU * cutoff.clamp(1.0, sr * 0.49) / sr).exp();
    }

    #[inline]
    pub fn lp(&mut self, x: f32) -> f32 {
        self.z += self.a * (x - self.z);
        self.z
    }

    #[inline]
    pub fn hp(&mut self, x: f32) -> f32 {
        x - self.lp(x)
    }
}

/// DC blocker.
#[derive(Clone, Copy, Debug, Default)]
pub struct DcBlock {
    x1: f32,
    y1: f32,
}

impl DcBlock {
    #[inline]
    pub fn tick(&mut self, x: f32) -> f32 {
        let y = x - self.x1 + 0.995 * self.y1;
        self.x1 = x;
        self.y1 = y;
        y
    }
}

#[inline]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wave {
    Saw,
    Square,
    Triangle,
    Sine,
}

impl Wave {
    pub fn from_index(i: f32) -> Self {
        match i.round() as i32 {
            1 => Wave::Square,
            2 => Wave::Triangle,
            3 => Wave::Sine,
            _ => Wave::Saw,
        }
    }
}

/// Band-limited (PolyBLEP) oscillator sample at phase `t` in [0, 1).
#[inline]
pub fn osc(wave: Wave, t: f32, dt: f32) -> f32 {
    match wave {
        Wave::Saw => 2.0 * t - 1.0 - poly_blep(t, dt),
        Wave::Square => {
            let mut v = if t < 0.5 { 1.0 } else { -1.0 };
            v += poly_blep(t, dt);
            let t2 = if t + 0.5 >= 1.0 { t - 0.5 } else { t + 0.5 };
            v - poly_blep(t2, dt)
        }
        Wave::Triangle => 4.0 * (t - 0.5).abs() - 1.0,
        Wave::Sine => (t * TAU).sin(),
    }
}

#[inline]
pub fn wrap_phase(p: f32) -> f32 {
    if p >= 1.0 {
        p - p.floor()
    } else {
        p
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adsr_reaches_sustain_and_dies() {
        let c = AdsrCoefs::new(0.01, 0.05, 0.5, 0.05, 1000.0);
        let mut e = Adsr::new();
        e.trigger();
        for _ in 0..500 {
            e.next(&c);
        }
        assert_eq!(e.stage, Stage::Sustain);
        assert!((e.level - 0.5).abs() < 1e-3);
        e.release();
        for _ in 0..500 {
            e.next(&c);
        }
        assert!(!e.is_active());
    }

    #[test]
    fn svf_lowpass_passes_dc() {
        let mut f = Svf::new();
        f.set(1000.0, 0.0, 48000.0);
        let mut y = 0.0;
        for _ in 0..10000 {
            y = f.process(1.0, FilterMode::Low);
        }
        assert!((y - 1.0).abs() < 1e-3);
    }

    #[test]
    fn rng_is_bipolar() {
        let mut r = Rng::new(1);
        for _ in 0..1000 {
            let v = r.bipolar();
            assert!((-1.0..1.0).contains(&v));
        }
    }
}

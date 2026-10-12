//! The stmlib pieces Beads uses that the Clouds port does not have yet
//! (`dsp/dsp.h`, `dsp/filter.h`, `dsp/hysteresis_quantizer.h`,
//! `dsp/hysteresis_filter.h`, `utils/gate_flags.h`), and the ones it does,
//! re-exported.
//!
//! Copyright 2012-2020 Emilie Gillet, MIT license. Rust port for Krabton.

pub use krabton_clouds::dsp::{clip16, integral_fractional, interpolate, one_pole, semitones_to_ratio, slope, CosineOscillator, ParameterInterpolator, Random};
pub use krabton_clouds::resources::LUT_SIN as LUT_SINE;

#[inline]
pub fn smooth_step(x: f32) -> f32 {
    x * x * (3.0 - 2.0 * x)
}

#[inline]
pub fn crossfade(a: f32, b: f32, fade: f32) -> f32 {
    a + (b - a) * fade
}

#[inline]
pub fn soft_limit(x: f32) -> f32 {
    x * (27.0 + x * x) / (27.0 + 9.0 * x * x)
}

#[inline]
pub fn soft_clip(x: f32) -> f32 {
    if x < -3.0 {
        -1.0
    } else if x > 3.0 {
        1.0
    } else {
        soft_limit(x)
    }
}

/// The `FREQUENCY_FAST` tangent approximation.
#[inline]
fn tan_fast(f: f32) -> f32 {
    const PI: f64 = std::f64::consts::PI;
    const A: f32 = (3.260e-01 * PI * PI * PI) as f32;
    const B: f32 = (1.823e-01 * PI * PI * PI * PI * PI) as f32;
    let f2 = f * f;
    f * (std::f32::consts::PI + f2 * (A + B * f2))
}

/// stmlib's state variable filter, one sample at a time.
#[derive(Clone, Copy, Debug, Default)]
pub struct Svf {
    g: f32,
    r: f32,
    h: f32,
    s1: f32,
    s2: f32,
}

impl Svf {
    pub fn init(&mut self) {
        // set_f_q<FREQUENCY_DIRTY>(0.01, 100).
        const A: f32 = (3.736e-01 * std::f64::consts::PI * std::f64::consts::PI * std::f64::consts::PI) as f32;
        let f = 0.01f32;
        self.g = f * (std::f32::consts::PI + A * f * f);
        self.r = 1.0 / 100.0;
        self.h = 1.0 / (1.0 + self.r * self.g + self.g * self.g);
        self.s1 = 0.0;
        self.s2 = 0.0;
    }

    pub fn set_f_q_fast(&mut self, f: f32, resonance: f32) {
        self.g = tan_fast(f);
        self.r = 1.0 / resonance;
        self.h = 1.0 / (1.0 + self.r * self.g + self.g * self.g);
    }

    /// The coefficients of another filter.
    pub fn set(&mut self, other: &Svf) {
        self.g = other.g;
        self.r = other.r;
        self.h = other.h;
    }

    #[inline]
    fn tick(&mut self, x: f32) -> (f32, f32) {
        let hp = (x - self.r * self.s1 - self.g * self.s1 - self.s2) * self.h;
        let bp = self.g * hp + self.s1;
        self.s1 = self.g * hp + bp;
        let lp = self.g * bp + self.s2;
        self.s2 = self.g * bp + lp;
        (lp, hp)
    }

    #[inline]
    pub fn low_pass(&mut self, x: f32) -> f32 {
        self.tick(x).0
    }

    #[inline]
    pub fn high_pass(&mut self, x: f32) -> f32 {
        self.tick(x).1
    }
}

/// Quantizes 0..1 to a step, with hysteresis at the boundaries.
#[derive(Clone, Copy, Debug, Default)]
pub struct HysteresisQuantizer2 {
    num_steps: i32,
    hysteresis: f32,
    scale: f32,
    offset: f32,
    quantized: i32,
}

impl HysteresisQuantizer2 {
    pub fn init(&mut self, num_steps: i32, hysteresis: f32, symmetric: bool) {
        self.num_steps = num_steps;
        self.hysteresis = hysteresis;
        self.scale = if symmetric { (num_steps - 1) as f32 } else { num_steps as f32 };
        self.offset = if symmetric { 0.0 } else { -0.5 };
        self.quantized = 0;
    }

    pub fn process(&mut self, value: f32) -> i32 {
        let value = value * self.scale + self.offset;
        let sign = if value > self.quantized as f32 { -1.0 } else { 1.0 };
        let q = ((value + sign * self.hysteresis + 0.5) as i32).clamp(0, (self.num_steps - 1).max(0));
        self.quantized = q;
        q
    }

    pub fn num_steps(&self) -> i32 {
        self.num_steps
    }
}

/// Follows a value, but only once it moved further than a threshold.
#[derive(Clone, Copy, Debug, Default)]
pub struct HysteresisFilter {
    value: f32,
    threshold: f32,
}

impl HysteresisFilter {
    pub fn init(&mut self, threshold: f32) {
        self.value = 0.0;
        self.threshold = threshold;
    }

    pub fn process(&mut self, value: f32) -> f32 {
        if self.threshold == 0.0 {
            self.value = value;
        } else {
            let error = value - self.value;
            if error > self.threshold {
                self.value = value - self.threshold;
            } else if error < -self.threshold {
                self.value = value + self.threshold;
            }
        }
        self.value
    }
}

/// Gate states with their edges.
pub type GateFlags = u8;
pub const GATE_FLAG_LOW: GateFlags = 0;
pub const GATE_FLAG_HIGH: GateFlags = 1;
pub const GATE_FLAG_RISING: GateFlags = 2;
pub const GATE_FLAG_FALLING: GateFlags = 4;

#[inline]
pub fn extract_gate_flags(previous: GateFlags, current: bool) -> GateFlags {
    let previous = previous & GATE_FLAG_HIGH;
    if current {
        if previous != 0 {
            GATE_FLAG_HIGH
        } else {
            GATE_FLAG_RISING | GATE_FLAG_HIGH
        }
    } else if previous != 0 {
        GATE_FLAG_FALLING
    } else {
        GATE_FLAG_LOW
    }
}

/// The dumb sine oscillator used for internal modulations (0..1).
#[derive(Clone, Copy, Debug, Default)]
pub struct SineOscillator {
    phase: f32,
    frequency: f32,
}

impl SineOscillator {
    pub fn init(&mut self, frequency: f32) {
        self.phase = 0.0;
        self.frequency = frequency;
    }

    pub fn phase(&self) -> f32 {
        self.phase
    }

    #[inline]
    pub fn render(&mut self) -> f32 {
        self.phase += self.frequency;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }
        0.5 + 0.5 * interpolate(&LUT_SINE, self.phase, 1024.0)
    }

    #[inline]
    pub fn render_at(&mut self, frequency: f32) -> f32 {
        self.frequency = frequency;
        self.render()
    }
}

/// A one-pole peak limiter, stereo.
#[derive(Clone, Copy, Debug, Default)]
pub struct Limiter {
    peak: f32,
    peak_lp: f32,
    decay: f32,
    max_peak: f32,
}

impl Limiter {
    pub fn init(&mut self, decay: f32, max_peak: f32) {
        self.peak = 0.5;
        self.peak_lp = 0.5;
        self.decay = decay;
        self.max_peak = max_peak;
    }

    pub fn process(&mut self, frames: &mut [[f32; 2]]) {
        for f in frames.iter_mut() {
            let peak = f[0].abs().max(f[1].abs()).min(40.0);
            slope(&mut self.peak, peak, 0.1, self.decay);
            let p = self.peak;
            slope(&mut self.peak_lp, p, 0.5, self.decay);
            let gain = if self.peak_lp <= self.max_peak { 1.0 } else { self.max_peak / self.peak_lp };
            f[0] *= gain;
            f[1] *= gain;
        }
    }
}

/// µ-law, as the codec of old telephones: 8 bits, finer near silence.
pub fn mu_law_to_linear(u: u8) -> f32 {
    let u = !u;
    let mut t: i32 = (((u & 0xf) as i32) << 3) + 0x84;
    t <<= ((u & 0x70) as i32) >> 4;
    let v = if u & 0x80 != 0 { 0x84 - t } else { t - 0x84 };
    v as f32 / 32768.0
}

pub fn linear_to_mu_law(pcm: i16) -> u8 {
    let mut pcm_val: i32 = (pcm as i32) >> 2;
    let mask: i32 = if pcm_val < 0 {
        pcm_val = -pcm_val;
        0x7f
    } else {
        0xff
    };
    if pcm_val > 8159 {
        pcm_val = 8159;
    }
    pcm_val += 0x84 >> 2;
    let seg = if pcm_val <= 0x3f {
        0
    } else if pcm_val <= 0x7f {
        1
    } else if pcm_val <= 0xff {
        2
    } else if pcm_val <= 0x1ff {
        3
    } else if pcm_val <= 0x3ff {
        4
    } else if pcm_val <= 0x7ff {
        5
    } else if pcm_val <= 0xfff {
        6
    } else if pcm_val <= 0x1fff {
        7
    } else {
        8
    };
    if seg >= 8 {
        (0x7f ^ mask) as u8
    } else {
        let uval = (seg << 4) | ((pcm_val >> (seg + 1)) & 0x0f);
        (uval ^ mask) as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mu_law_matches_the_module_table() {
        // A few entries of lut_ulaw_f.
        assert!((mu_law_to_linear(0) + 0.98035).abs() < 1e-4);
        assert!((mu_law_to_linear(16) + 0.48816).abs() < 1e-4);
        assert!((mu_law_to_linear(128) - 0.98035).abs() < 1e-4);
        assert!(mu_law_to_linear(255).abs() < 1e-6);
        // Round trip within a step.
        for x in [-0.9f32, -0.3, -0.01, 0.0, 0.004, 0.2, 0.7] {
            let y = mu_law_to_linear(linear_to_mu_law((x * 32767.0) as i16));
            assert!((x - y).abs() < 0.02 + 0.04 * x.abs(), "{x} {y}");
        }
    }
}

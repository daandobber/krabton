//! Small DSP helpers from stmlib (`dsp/dsp.h`, `dsp/units.h`,
//! `dsp/rsqrt.h`, `dsp/atan.h`, `dsp/filter.h`, `dsp/cosine_oscillator.h`,
//! `dsp/parameter_interpolator.h`, `utils/random.h`).
//!
//! Copyright 2012-2014 Emilie Gillet, MIT license. Rust port for Krabton.

use crate::resources::{ATAN_LUT, LUT_PITCH_RATIO_HIGH, LUT_PITCH_RATIO_LOW};

/// Splits `x` into its integral part (truncated towards zero, like a C
/// cast) and the fractional rest (`MAKE_INTEGRAL_FRACTIONAL`).
#[inline]
pub fn integral_fractional(x: f32) -> (i32, f32) {
    let i = x as i32;
    (i, x - i as f32)
}

/// Linear interpolation in a table; `index` goes from 0 to 1 over `size`
/// entries. The position is clamped to the table, where C++ would read
/// past it. Tables must have at least two entries.
#[inline]
pub fn interpolate(table: &[f32], index: f32, size: f32) -> f32 {
    let last = (table.len() - 1) as f32;
    let x = (index * size).clamp(0.0, last);
    let i = (x as usize).min(table.len() - 2);
    let f = x - i as f32;
    let a = table[i];
    let b = table[i + 1];
    a + (b - a) * f
}

#[inline]
pub fn crossfade(a: f32, b: f32, fade: f32) -> f32 {
    a + (b - a) * fade
}

#[inline]
pub fn soft_limit(x: f32) -> f32 {
    x * (27.0 + x * x) / (27.0 + 9.0 * x * x)
}

/// The `ssat #16` of the module.
#[inline]
pub fn clip16(x: i32) -> i32 {
    x.clamp(-32768, 32767)
}

/// Float to codec sample, with the soft limiter of the output stage.
#[inline]
pub fn soft_convert(x: f32) -> i16 {
    clip16((soft_limit(x * 0.5) * 32768.0) as i32) as i16
}

/// 2^(semitones / 12), from two tables (valid from -128 to +127 semitones;
/// clamped to that range here).
#[inline]
pub fn semitones_to_ratio(semitones: f32) -> f32 {
    let pitch = (semitones + 128.0).clamp(0.0, 255.999);
    let (i, f) = integral_fractional(pitch);
    LUT_PITCH_RATIO_HIGH[i as usize] * LUT_PITCH_RATIO_LOW[((f * 256.0) as usize).min(255)]
}

#[inline]
pub fn fast_rsqrt_carmack(x: f32) -> f32 {
    let i = 0x5f37_59dfu32.wrapping_sub(x.to_bits() >> 1);
    let y = f32::from_bits(i);
    y * (1.5 - (x * 0.5 * y * y))
}

/// Angle (0..65535 for a full turn) and magnitude of (x, y).
#[inline]
pub fn fast_atan2r(y: f32, x: f32) -> (u16, f32) {
    let squared_magnitude = x * x + y * y;
    if squared_magnitude == 0.0 {
        return (0, 0.0);
    }
    let rinv = fast_rsqrt_carmack(squared_magnitude);
    let r = rinv * squared_magnitude;
    const SIGN_MASK: u32 = 0x8000_0000;
    let ux_s = SIGN_MASK & x.to_bits();
    let uy_s = SIGN_MASK & y.to_bits();
    let quadrant = ((!ux_s & uy_s) >> 29) | (ux_s >> 30);
    let (x, y) = (x.abs(), y.abs());
    let lut = |v: f32| ATAN_LUT[((v * rinv * 512.0 + 0.5) as u32).min(512) as usize];
    let mut angle: u16 = if y > x { 16384u16.wrapping_sub(lut(x)) } else { lut(y) };
    if ux_s ^ uy_s != 0 {
        angle = angle.wrapping_neg();
    }
    (angle.wrapping_add((quadrant << 14) as u16), r)
}

/// The stmlib linear congruential generator. The module shares one global
/// state; here every processor owns its own.
#[derive(Clone, Copy, Debug)]
pub struct Random {
    state: u32,
}

impl Default for Random {
    fn default() -> Self {
        Self { state: 0x21 }
    }
}

impl Random {
    #[inline]
    pub fn get_word(&mut self) -> u32 {
        self.state = self.state.wrapping_mul(1664525).wrapping_add(1013904223);
        self.state
    }

    #[inline]
    pub fn get_sample(&mut self) -> i16 {
        (self.get_word() >> 16) as i16
    }

    #[inline]
    pub fn get_float(&mut self) -> f32 {
        self.get_word() as f32 / 4294967296.0
    }
}

/// `ONE_POLE`.
#[inline]
pub fn one_pole(out: &mut f32, input: f32, coefficient: f32) {
    *out += coefficient * (input - *out);
}

/// `SLOPE`.
#[inline]
pub fn slope(out: &mut f32, input: f32, positive: f32, negative: f32) {
    let error = input - *out;
    *out += if error > 0.0 { positive } else { negative } * error;
}

/// Linear ramp of a parameter over a block (`ParameterInterpolator`); the
/// state reaches the new value when the ramp is dropped.
pub struct ParameterInterpolator<'a> {
    state: &'a mut f32,
    value: f32,
    increment: f32,
}

impl<'a> ParameterInterpolator<'a> {
    pub fn new(state: &'a mut f32, new_value: f32, size: usize) -> Self {
        let value = *state;
        let increment = (new_value - value) / size.max(1) as f32;
        Self { state, value, increment }
    }

    #[inline]
    pub fn next(&mut self) -> f32 {
        self.value += self.increment;
        self.value
    }
}

impl Drop for ParameterInterpolator<'_> {
    fn drop(&mut self) {
        *self.state = self.value;
    }
}

/// The `FREQUENCY_FAST` tangent approximation of stmlib.
#[inline]
fn tan_fast(f: f32) -> f32 {
    const PI: f64 = std::f64::consts::PI;
    const A: f32 = (3.260e-01 * PI * PI * PI) as f32;
    const B: f32 = (1.823e-01 * PI * PI * PI * PI * PI) as f32;
    let f2 = f * f;
    f * (std::f32::consts::PI + f2 * (A + B * f2))
}

/// The "naive" (TPT) state variable filter of stmlib.
#[derive(Clone, Copy, Debug, Default)]
pub struct Svf {
    g: f32,
    r: f32,
    h: f32,
    state_1: f32,
    state_2: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilterMode {
    LowPass,
    HighPass,
}

impl Svf {
    pub fn init(&mut self) {
        // set_f_q<FREQUENCY_DIRTY>(0.01, 100).
        const A: f32 = (3.736e-01 * std::f64::consts::PI * std::f64::consts::PI * std::f64::consts::PI) as f32;
        let f = 0.01f32;
        self.g = f * (std::f32::consts::PI + A * f * f);
        self.r = 1.0 / 100.0;
        self.h = 1.0 / (1.0 + self.r * self.g + self.g * self.g);
        self.state_1 = 0.0;
        self.state_2 = 0.0;
    }

    /// Copies the coefficients of another filter.
    pub fn set(&mut self, other: &Svf) {
        self.g = other.g;
        self.r = other.r;
        self.h = other.h;
    }

    pub fn set_f_q_fast(&mut self, f: f32, resonance: f32) {
        self.g = tan_fast(f);
        self.r = 1.0 / resonance;
        self.h = 1.0 / (1.0 + self.r * self.g + self.g * self.g);
    }

    /// Filters one channel of interleaved stereo frames in place.
    pub fn process_frames(&mut self, mode: FilterMode, frames: &mut [[f32; 2]], channel: usize) {
        let (g, r, h) = (self.g, self.r, self.h);
        let (mut s1, mut s2) = (self.state_1, self.state_2);
        for frame in frames.iter_mut() {
            let input = frame[channel];
            let hp = (input - r * s1 - g * s1 - s2) * h;
            let bp = g * hp + s1;
            s1 = g * hp + bp;
            let lp = g * bp + s2;
            s2 = g * bp + lp;
            frame[channel] = if mode == FilterMode::LowPass { lp } else { hp };
        }
        self.state_1 = s1;
        self.state_2 = s2;
    }
}

/// Sine oscillator by a resonant IIR (`COSINE_OSCILLATOR_APPROXIMATE`).
#[derive(Clone, Copy, Debug, Default)]
pub struct CosineOscillator {
    y1: f32,
    y0: f32,
    iir_coefficient: f32,
    initial_amplitude: f32,
}

impl CosineOscillator {
    pub fn init_approximate(&mut self, frequency: f32) {
        let mut sign = 16.0;
        let mut frequency = frequency - 0.25;
        if frequency < 0.0 {
            frequency = -frequency;
        } else if frequency > 0.5 {
            frequency -= 0.5;
        } else {
            sign = -16.0;
        }
        self.iir_coefficient = sign * frequency * (1.0 - 2.0 * frequency);
        self.initial_amplitude = self.iir_coefficient * 0.25;
        self.start();
    }

    pub fn start(&mut self) {
        self.y1 = self.initial_amplitude;
        self.y0 = 0.5;
    }

    #[inline]
    pub fn value(&self) -> f32 {
        self.y1 + 0.5
    }

    #[inline]
    pub fn next(&mut self) -> f32 {
        let temp = self.y0;
        self.y0 = self.iir_coefficient * self.y0 - self.y1;
        self.y1 = temp;
        temp + 0.5
    }
}

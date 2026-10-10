//! Small DSP helpers from stmlib (`dsp/dsp.h`, `dsp/units.h`,
//! `utils/random.h`) and the constants of `rings/dsp/dsp.h`.
//!
//! Copyright 2015 Emilie Gillet, MIT license. Rust port for Krabton.

use crate::resources::{LUT_PITCH_RATIO_HIGH, LUT_PITCH_RATIO_LOW};

/// The sample rate the tables and algorithms were made for.
pub const SAMPLE_RATE: f32 = 48000.0;
/// Block size: controls (strum, notes, envelopes) update once per block.
pub const BLOCK_SIZE: usize = 24;
/// Normalized frequency of A3 (MIDI note 69).
pub const A3: f32 = 440.0 / SAMPLE_RATE;

/// Splits `x` into its integral part (truncated towards zero, like a C
/// cast) and the fractional rest.
#[inline]
pub fn integral_fractional(x: f32) -> (i32, f32) {
    let i = x as i32;
    (i, x - i as f32)
}

/// Linear interpolation in a table; `index` goes from 0 to 1 over `size`
/// entries. Out-of-range indices read the first or last entry instead of
/// beyond the table.
#[inline]
pub fn interpolate(table: &[f32], index: f32, size: f32) -> f32 {
    let x = (index * size).clamp(0.0, (table.len() - 1) as f32);
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

#[inline]
pub fn clip16(x: i32) -> i32 {
    x.clamp(-32768, 32767)
}

/// `SLOPE`: a one-pole follower with different rise and fall rates.
#[inline]
pub fn slope(out: &mut f32, input: f32, positive: f32, negative: f32) {
    let error = input - *out;
    *out += if error > 0.0 { positive } else { negative } * error;
}

/// `SLEW`: moves towards `input` by at most `delta`.
#[inline]
pub fn slew(out: &mut f32, input: f32, delta: f32) {
    let error = (input - *out).clamp(-delta, delta);
    *out += error;
}

/// `ONE_POLE`.
#[inline]
pub fn one_pole(out: &mut f32, input: f32, coefficient: f32) {
    *out += coefficient * (input - *out);
}

/// 2^(semitones / 12), from two tables (valid from -128 to +127 semitones;
/// clamped outside).
#[inline]
pub fn semitones_to_ratio(semitones: f32) -> f32 {
    let pitch = if semitones.is_nan() { 128.0 } else { (semitones + 128.0).clamp(0.0, 255.999) };
    let (i, f) = integral_fractional(pitch);
    LUT_PITCH_RATIO_HIGH[i as usize] * LUT_PITCH_RATIO_LOW[((f * 256.0) as usize).min(255)]
}

/// The stmlib linear congruential generator. The C++ code shares one global
/// state; here each user owns one.
#[derive(Clone, Copy, Debug)]
pub struct Rng(u32);

impl Rng {
    pub fn new(seed: u32) -> Self {
        Self(seed)
    }

    #[inline]
    pub fn word(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1664525).wrapping_add(1013904223);
        self.0
    }

    /// 0 to 1.
    #[inline]
    pub fn float(&mut self) -> f32 {
        self.word() as f32 / 4294967296.0
    }
}

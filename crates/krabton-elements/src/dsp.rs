//! Small DSP helpers from stmlib (`dsp/dsp.h`, `dsp/units.h`,
//! `utils/random.h`).
//!
//! Copyright 2014 Emilie Gillet, MIT license. Rust port for Krabton.

use crate::resources::{LUT_PITCH_RATIO_HIGH, LUT_PITCH_RATIO_LOW};
use std::cell::Cell;

/// Splits `x` into its integral part (truncated towards zero, like a C
/// cast) and the fractional rest.
#[inline]
pub fn integral_fractional(x: f32) -> (i32, f32) {
    let i = x as i32;
    (i, x - i as f32)
}

/// Linear interpolation in a table; `index` goes from 0 to 1 over `size`
/// entries.
#[inline]
pub fn interpolate(table: &[f32], index: f32, size: f32) -> f32 {
    let (i, f) = integral_fractional(index * size);
    let i = i.max(0) as usize;
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

/// 2^(semitones / 12), from two tables (valid from -128 to +127 semitones).
#[inline]
pub fn semitones_to_ratio(semitones: f32) -> f32 {
    let pitch = (semitones + 128.0).clamp(0.0, 255.999);
    let (i, f) = integral_fractional(pitch);
    LUT_PITCH_RATIO_HIGH[i as usize] * LUT_PITCH_RATIO_LOW[(f * 256.0) as usize]
}

thread_local! {
    static RNG: Cell<u32> = const { Cell::new(0x21) };
}

/// The stmlib linear congruential generator. The C++ code shares one global
/// state; here every audio thread has its own.
#[inline]
pub fn random_word() -> u32 {
    RNG.with(|r| {
        let s = r.get().wrapping_mul(1664525).wrapping_add(1013904223);
        r.set(s);
        s
    })
}

#[inline]
pub fn random_float() -> f32 {
    random_word() as f32 / 4294967296.0
}

pub fn random_seed(seed: u32) {
    RNG.with(|r| r.set(seed));
}

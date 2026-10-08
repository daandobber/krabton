//! The stmlib helpers Braids uses: table interpolation, mixing, clipping,
//! the global random generator and the parameter interpolation macros.
//!
//! Based on `stmlib/utils/dsp.h`, `stmlib/utils/random.h` and
//! `braids/parameter_interpolation.h` (Copyright Emilie Gillet, MIT).
//! Arithmetic wraps exactly like the C++ code on the original hardware.

use core::sync::atomic::{AtomicU32, Ordering};

/// `CLIP(x)`: clamps to the signed 16-bit range (symmetric).
#[inline(always)]
pub fn clip(x: i32) -> i32 {
    x.clamp(-32767, 32767)
}

/// `CONSTRAIN(var, min, max)`.
#[inline(always)]
pub fn constrain(x: i32, min: i32, max: i32) -> i32 {
    if x < min {
        min
    } else if x > max {
        max
    } else {
        x
    }
}

/// `Mix(int16_t a, int16_t b, uint16_t balance)`.
#[inline(always)]
pub fn mix(a: i16, b: i16, balance: u16) -> i16 {
    let bal = balance as i32;
    ((a as i32).wrapping_mul(65535 - bal).wrapping_add((b as i32).wrapping_mul(bal)) >> 16) as i16
}

/// `Mix(uint16_t a, uint16_t b, uint16_t balance)`.
#[inline(always)]
pub fn mix_u16(a: u16, b: u16, balance: u16) -> u16 {
    let bal = balance as u32;
    ((a as u32).wrapping_mul(65535 - bal).wrapping_add((b as u32).wrapping_mul(bal)) >> 16) as u16
}

/// `Interpolate824(const int16_t*, uint32_t phase)`.
#[inline(always)]
pub fn interpolate824(table: &[i16], phase: u32) -> i16 {
    let i = (phase >> 24) as usize;
    let a = table[i] as i32;
    let b = table[i + 1] as i32;
    a.wrapping_add((b - a).wrapping_mul(((phase >> 8) & 0xffff) as i32) >> 16) as i16
}

/// `Interpolate824(const uint16_t*, uint32_t phase)`.
#[inline(always)]
pub fn interpolate824_u16(table: &[u16], phase: u32) -> u16 {
    let i = (phase >> 24) as usize;
    let a = table[i] as u32;
    let b = table[i + 1] as u32;
    a.wrapping_add(b.wrapping_sub(a).wrapping_mul((phase >> 8) & 0xffff) >> 16) as u16
}

/// `Interpolate824(const uint8_t*, uint32_t phase)`.
#[inline(always)]
pub fn interpolate824_u8(table: &[u8], phase: u32) -> i16 {
    let i = (phase >> 24) as usize;
    let a = table[i] as i32;
    let b = table[i + 1] as i32;
    ((a << 8).wrapping_add((b - a).wrapping_mul((phase & 0xffffff) as i32) >> 16) - 32768) as i16
}

/// `Interpolate88(const uint16_t*, uint16_t index)`.
#[inline(always)]
pub fn interpolate88_u16(table: &[u16], index: u16) -> u16 {
    let i = (index >> 8) as usize;
    let a = table[i] as i32;
    let b = table[i + 1] as i32;
    (a + ((b - a) * (index & 0xff) as i32 >> 8)) as u16
}

/// `Interpolate88(const int16_t*, uint16_t index)`.
#[inline(always)]
pub fn interpolate88(table: &[i16], index: u16) -> i16 {
    let i = (index >> 8) as usize;
    let a = table[i] as i32;
    let b = table[i + 1] as i32;
    (a + ((b - a) * (index & 0xff) as i32 >> 8)) as i16
}

/// `Interpolate1022(const int16_t*, uint32_t phase)`.
#[inline(always)]
pub fn interpolate1022(table: &[i16], phase: u32) -> i16 {
    let i = (phase >> 22) as usize;
    let a = table[i] as i32;
    let b = table[i + 1] as i32;
    a.wrapping_add((b - a).wrapping_mul(((phase >> 6) & 0xffff) as i32) >> 16) as i16
}

/// `Interpolate115(const int16_t*, uint16_t phase)`.
#[inline(always)]
pub fn interpolate115(table: &[i16], phase: u16) -> i16 {
    let i = (phase >> 5) as usize;
    let a = table[i] as i32;
    let b = table[i + 1] as i32;
    (a + ((b - a) * (phase & 0x1f) as i32 >> 5)) as i16
}

/// `Crossfade(const int16_t*, const int16_t*, uint32_t, uint16_t)`.
#[inline(always)]
pub fn crossfade(table_a: &[i16], table_b: &[i16], phase: u32, balance: u16) -> i16 {
    let a = interpolate824(table_a, phase) as i32;
    let b = interpolate824(table_b, phase) as i32;
    a.wrapping_add((b - a).wrapping_mul(balance as i32) >> 16) as i16
}

/// `Crossfade(const uint8_t*, const uint8_t*, uint32_t, uint16_t)`.
#[inline(always)]
pub fn crossfade_u8(table_a: &[u8], table_b: &[u8], phase: u32, balance: u16) -> i16 {
    let a = interpolate824_u8(table_a, phase) as i32;
    let b = interpolate824_u8(table_b, phase) as i32;
    a.wrapping_add((b - a).wrapping_mul(balance as i32) >> 16) as i16
}

/// `Crossfade1022(const int16_t*, const int16_t*, uint32_t, uint16_t)`.
#[inline(always)]
pub fn crossfade1022(table_a: &[i16], table_b: &[i16], phase: u32, balance: u16) -> i16 {
    let a = interpolate1022(table_a, phase) as i32;
    let b = interpolate1022(table_b, phase) as i32;
    a.wrapping_add((b - a).wrapping_mul(balance as i32) >> 16) as i16
}

/// `Crossfade115(const int16_t*, const int16_t*, uint16_t, uint16_t)`.
#[inline(always)]
pub fn crossfade115(table_a: &[i16], table_b: &[i16], phase: u16, balance: u16) -> i16 {
    let a = interpolate115(table_a, phase) as i32;
    let b = interpolate115(table_b, phase) as i32;
    a.wrapping_add((b - a).wrapping_mul(balance as i32) >> 16) as i16
}

/// Unsigned division that yields 0 for a zero divisor, like the Cortex-M4
/// divider Braids runs on (division by zero does not trap there).
#[inline(always)]
pub fn udiv(a: u32, b: u32) -> u32 {
    if b == 0 {
        0
    } else {
        a / b
    }
}

/// Signed division with the same zero-divisor behaviour (and wrapping for
/// `i32::MIN / -1`).
#[inline(always)]
pub fn sdiv(a: i32, b: i32) -> i32 {
    if b == 0 {
        0
    } else {
        a.wrapping_div(b)
    }
}

// ---------------------------------------------------------------- random

static RNG_STATE: AtomicU32 = AtomicU32::new(0x2101_1971);

/// `stmlib::Random::GetWord()`: one global linear congruential generator,
/// shared by every oscillator like on the module.
#[inline(always)]
pub fn random_word() -> u32 {
    let s = RNG_STATE.load(Ordering::Relaxed).wrapping_mul(1664525).wrapping_add(1013904223);
    RNG_STATE.store(s, Ordering::Relaxed);
    s
}

/// `stmlib::Random::GetSample()`.
#[inline(always)]
pub fn random_sample() -> i16 {
    (random_word() >> 16) as i16
}

/// `stmlib::Random::Seed()`.
pub fn random_seed(seed: u32) {
    RNG_STATE.store(seed, Ordering::Relaxed);
}

// ---------------------------------------------------------------- parameter interpolation

/// `BEGIN_INTERPOLATE_PARAMETER_0/1` and friends: a linear ramp from the
/// previous block's value to the current one across `size` samples.
/// `next()` is `INTERPOLATE_PARAMETER_x` (it advances first, then returns).
#[derive(Clone, Copy)]
pub struct ParamRamp {
    start: i32,
    delta: i32,
    increment: i32,
    xfade: i32,
}

impl ParamRamp {
    #[inline(always)]
    pub fn new(previous: i16, current: i16, size: usize) -> Self {
        Self { start: previous as i32, delta: current as i32 - previous as i32, increment: 32767 / size.max(1) as i32, xfade: 0 }
    }

    #[inline(always)]
    pub fn next(&mut self) -> i32 {
        self.xfade = self.xfade.wrapping_add(self.increment);
        self.start.wrapping_add(self.delta.wrapping_mul(self.xfade) >> 15)
    }
}

/// `BEGIN_INTERPOLATE_PHASE_INCREMENT`: `next()` is
/// `INTERPOLATE_PHASE_INCREMENT` followed by reading `phase_increment`.
/// `value()` gives the current value (for `END_INTERPOLATE_PHASE_INCREMENT`).
#[derive(Clone, Copy)]
pub struct PhaseIncrementRamp {
    value: u32,
    step: u32,
}

impl PhaseIncrementRamp {
    #[inline(always)]
    pub fn new(previous: u32, current: u32, size: usize) -> Self {
        let size = size.max(1) as u32;
        let step = if previous < current { (current - previous) / size } else { !((previous - current) / size) };
        Self { value: previous, step }
    }

    #[inline(always)]
    pub fn next(&mut self) -> u32 {
        self.value = self.value.wrapping_add(self.step);
        self.value
    }

    #[inline(always)]
    pub fn value(&self) -> u32 {
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn helpers_match_the_c_semantics() {
        assert_eq!(mix(32767, -32768, 0), 32766);
        assert_eq!(mix(0, 32767, 65535), 32766);
        assert_eq!(mix_u16(65535, 0, 0), 65534);
        let t: [i16; 3] = [0, 1000, -1000];
        assert_eq!(interpolate824(&t, 0x0080_0000), 500);
        assert_eq!(interpolate88(&t, 0x0180), 0);
        let u: [u16; 3] = [1000, 0, 500];
        assert_eq!(interpolate824_u16(&u, 0x0080_0000), 500);
        let b: [u8; 2] = [0, 255];
        assert_eq!(interpolate824_u8(&b, 0), -32768);
        let mut r = PhaseIncrementRamp::new(100, 50, 5);
        for _ in 0..5 {
            r.next();
        }
        // `~x` is -x - 1 per step, exactly like the C macro.
        assert_eq!(r.value(), 45);
        let mut p = ParamRamp::new(0, 32767, 24);
        let mut last = 0;
        for _ in 0..24 {
            last = p.next();
        }
        assert!(last > 32000);
    }
}

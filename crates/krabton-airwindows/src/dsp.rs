//! Pieces every Airwindows plug-in repeats inline: the `fpd` xorshift
//! generators, the input noise floor that keeps the math out of denormals,
//! the 32-bit floating-point dither, and the bits of the newer reverbs that
//! run the tank at 44.1/48 kHz and interpolate back up.
//!
//! Copyright (c) Chris Johnson (airwindows), MIT license. Rust port for Krabton.

use std::sync::atomic::{AtomicU32, Ordering};

static SEED: AtomicU32 = AtomicU32::new(0x9e37_79b9);

/// A startup value for an `fpd` generator. The originals use
/// `fpd = 1.0; while (fpd < 16386) fpd = rand()*UINT32_MAX;`, so any value of
/// at least 16386 will do; this one differs per instance.
pub(crate) fn seed() -> u32 {
    let mut x = SEED.fetch_add(0x6d2b_79f5, Ordering::Relaxed);
    x = (x ^ (x >> 15)).wrapping_mul(0x2c1b_3c6d);
    x = (x ^ (x >> 12)).wrapping_mul(0x297a_2d39);
    x ^= x >> 15;
    if x < 16386 {
        x = x.wrapping_add(0x0100_0000);
    }
    x
}

/// `fpd ^= fpd << 13; fpd ^= fpd >> 17; fpd ^= fpd << 5;`
#[inline]
pub(crate) fn xorshift(fpd: &mut u32) {
    *fpd ^= *fpd << 13;
    *fpd ^= *fpd >> 17;
    *fpd ^= *fpd << 5;
}

/// `if (fabs(x)<1.18e-23) x = fpd * 1.18e-17;` at the top of every sample.
#[inline]
pub(crate) fn noise_floor(x: f64, fpd: u32) -> f64 {
    if x.abs() < 1.18e-23 {
        fpd as f64 * 1.18e-17
    } else {
        x
    }
}

/// `fpd / UINT32_MAX`, 0..1.
#[inline]
pub(crate) fn unit(fpd: u32) -> f64 {
    fpd as f64 / u32::MAX as f64
}

/// The exponent `frexpf` returns (0 for zero, infinities and NaN).
#[inline]
fn frexpf_exponent(x: f32) -> i32 {
    if x == 0.0 || !x.is_finite() {
        return 0;
    }
    let biased = ((x.to_bits() >> 23) & 0xff) as i32;
    if biased == 0 {
        let scaled = x * 33_554_432.0; // 2^25, makes subnormals normal
        ((scaled.to_bits() >> 23) & 0xff) as i32 - 126 - 25
    } else {
        biased - 126
    }
}

/// `pow(2, e)` for the exponents the dither needs.
#[inline]
fn exp2i(e: i32) -> f64 {
    f64::from_bits(((e.clamp(-1022, 1023) + 1023) as u64) << 52)
}

/// The "32 bit stereo floating point dither" that ends every
/// `processReplacing`: advances the generator and adds noise at the level of
/// the last bit of the `f32` the sample is about to become.
#[inline]
pub(crate) fn dither32(x: f64, fpd: &mut u32) -> f64 {
    let expon = frexpf_exponent(x as f32);
    xorshift(fpd);
    x + ((*fpd as f64 - 0x7fff_ffff as f64) * 5.5e-36 * exp2i(expon + 62))
}

/// `int cycleEnd = floor(overallscale);` clamped to 1..4: the newer reverbs
/// and delays compute one tank sample every `cycleEnd` samples.
#[inline]
pub(crate) fn cycle_end(overallscale: f64) -> usize {
    (overallscale.floor() as i64).clamp(1, 4) as usize
}

/// After a tank sample: spread the new value over `lastRef[0..cycleEnd]`.
#[inline]
pub(crate) fn fill_last_ref(last_ref: &mut [f64], cycle_end: usize, x: f64) {
    match cycle_end {
        4 => {
            last_ref[0] = last_ref[4]; //start from previous last
            last_ref[2] = (last_ref[0] + x) / 2.0; //half
            last_ref[1] = (last_ref[0] + last_ref[2]) / 2.0; //one quarter
            last_ref[3] = (last_ref[2] + x) / 2.0; //three quarters
            last_ref[4] = x; //full
        }
        3 => {
            last_ref[0] = last_ref[3]; //start from previous last
            last_ref[2] = (last_ref[0] + last_ref[0] + x) / 3.0; //third
            last_ref[1] = (last_ref[0] + x + x) / 3.0; //two thirds
            last_ref[3] = x; //full
        }
        2 => {
            last_ref[0] = last_ref[2]; //start from previous last
            last_ref[1] = (last_ref[0] + x) / 2.0; //half
            last_ref[2] = x; //full
        }
        _ => last_ref[0] = x,
    }
}

/// The `switch (cycleEnd)` fall-through "multi-pole average using lastRef[]".
#[inline]
pub(crate) fn multipole_average(last_ref: &mut [f64], cycle_end: usize, mut x: f64) -> f64 {
    if cycle_end >= 4 {
        last_ref[8] = x;
        x = (x + last_ref[7]) * 0.5;
        last_ref[7] = last_ref[8];
    }
    if cycle_end >= 3 {
        last_ref[8] = x;
        x = (x + last_ref[6]) * 0.5;
        last_ref[6] = last_ref[8];
    }
    if cycle_end >= 2 {
        last_ref[8] = x;
        x = (x + last_ref[5]) * 0.5;
        last_ref[5] = last_ref[8];
    }
    x
}

/// One of the reverb tank delays: a left and a right buffer sharing one
/// `count` and one `delay`, as in the originals (`aAL`, `aAR`, `countA`,
/// `delayA`).
pub(crate) struct Line {
    pub(crate) l: Box<[f64]>,
    pub(crate) r: Box<[f64]>,
    pub(crate) count: usize,
    pub(crate) delay: usize,
}

impl Line {
    pub(crate) fn new(len: usize) -> Self {
        Self { l: vec![0.0; len].into_boxed_slice(), r: vec![0.0; len].into_boxed_slice(), count: 1, delay: 0 }
    }

    pub(crate) fn reset(&mut self) {
        self.l.fill(0.0);
        self.r.fill(0.0);
        self.count = 1;
    }

    /// `delayA = 5003.0*size;` (kept inside the buffer).
    #[inline]
    pub(crate) fn set_delay(&mut self, samples: f64) {
        let d = if samples > 0.0 { samples as usize } else { 0 };
        self.delay = d.min(self.l.len() - 1);
    }

    /// `aAL[countA] = l; aAR[countA] = r; countA++; if (countA < 0 || countA >
    /// delayA) countA = 0;` then read `aAL[countA]`, `aAR[countA]`.
    #[inline]
    pub(crate) fn tick(&mut self, l: f64, r: f64) -> (f64, f64) {
        self.l[self.count] = l;
        self.r[self.count] = r;
        self.count += 1;
        if self.count > self.delay {
            self.count = 0;
        }
        (self.l[self.count], self.r[self.count])
    }
}

/// `b + c + d` for `a - (b + c + d)`: the sum of all but entry `skip`, added
/// left to right like the originals spell it out.
#[inline]
pub(crate) fn others(x: &[f64], skip: usize) -> f64 {
    let mut sum = 0.0;
    let mut first = true;
    for (i, &v) in x.iter().enumerate() {
        if i != skip {
            if first {
                sum = v;
                first = false;
            } else {
                sum += v;
            }
        }
    }
    sum
}

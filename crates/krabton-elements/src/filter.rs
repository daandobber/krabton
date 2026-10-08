//! Filters, delay lines and the cosine oscillator of stmlib
//! (`dsp/filter.h`, `dsp/delay_line.h`, `dsp/cosine_oscillator.h`).
//!
//! Copyright 2014 Emilie Gillet, MIT license. Rust port for Krabton.

use crate::dsp::integral_fractional;
use std::f64::consts::PI;

/// How `tan(pi * f)` is approximated when setting a cutoff.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Approx {
    Exact,
    Accurate,
    Fast,
    Dirty,
}

const PI_F: f32 = PI as f32;

#[inline]
pub fn tan_approx(f: f32, approx: Approx) -> f32 {
    match approx {
        Approx::Exact => (PI_F * f.min(0.497)).tan(),
        Approx::Dirty => {
            const A: f32 = (3.736e-01 * PI * PI * PI) as f32;
            f * (PI_F + A * f * f)
        }
        Approx::Fast => {
            const A: f32 = (3.260e-01 * PI * PI * PI) as f32;
            const B: f32 = (1.823e-01 * PI * PI * PI * PI * PI) as f32;
            let f2 = f * f;
            f * (PI_F + f2 * (A + B * f2))
        }
        Approx::Accurate => {
            const A: f32 = (3.333314036e-01 * PI * PI * PI) as f32;
            const B: f32 = (1.333923995e-01 * PI * PI * PI * PI * PI) as f32;
            const C: f32 = (5.33740603e-02 * PI * PI * PI * PI * PI * PI * PI) as f32;
            const D: f32 = (2.900525e-03 * PI * PI * PI * PI * PI * PI * PI * PI * PI) as f32;
            const E: f32 = (9.5168091e-03 * PI * PI * PI * PI * PI * PI * PI * PI * PI * PI * PI) as f32;
            let f2 = f * f;
            f * (PI_F + f2 * (A + f2 * (B + f2 * (C + f2 * (D + f2 * E)))))
        }
    }
}

/// Zero-delay-feedback state variable filter.
#[derive(Clone, Copy, Debug)]
pub struct Svf {
    g: f32,
    r: f32,
    h: f32,
    state_1: f32,
    state_2: f32,
}

impl Default for Svf {
    fn default() -> Self {
        let mut f = Self { g: 0.0, r: 0.0, h: 0.0, state_1: 0.0, state_2: 0.0 };
        f.set_f_q(0.01, 100.0, Approx::Dirty);
        f
    }
}

impl Svf {
    pub fn reset(&mut self) {
        self.state_1 = 0.0;
        self.state_2 = 0.0;
    }

    #[inline]
    pub fn g(&self) -> f32 {
        self.g
    }

    #[inline]
    pub fn set_g_r_h(&mut self, g: f32, r: f32, h: f32) {
        self.g = g;
        self.r = r;
        self.h = h;
    }

    #[inline]
    pub fn set_g_r(&mut self, g: f32, r: f32) {
        self.g = g;
        self.r = r;
        self.h = 1.0 / (1.0 + r * g + g * g);
    }

    #[inline]
    pub fn set_g_q(&mut self, g: f32, resonance: f32) {
        self.g = g;
        self.r = 1.0 / resonance;
        self.h = 1.0 / (1.0 + self.r * g + g * g);
    }

    #[inline]
    pub fn set_f_q(&mut self, f: f32, resonance: f32, approx: Approx) {
        self.g = tan_approx(f, approx);
        self.r = 1.0 / resonance;
        self.h = 1.0 / (1.0 + self.r * self.g + self.g * self.g);
    }

    /// One step; returns (low-pass, band-pass).
    #[inline]
    fn step(&mut self, input: f32) -> (f32, f32) {
        let hp = (input - self.r * self.state_1 - self.g * self.state_1 - self.state_2) * self.h;
        let bp = self.g * hp + self.state_1;
        self.state_1 = self.g * hp + bp;
        let lp = self.g * bp + self.state_2;
        self.state_2 = self.g * bp + lp;
        (lp, bp)
    }

    #[inline]
    pub fn process_lp(&mut self, input: f32) -> f32 {
        self.step(input).0
    }

    #[inline]
    pub fn process_bp(&mut self, input: f32) -> f32 {
        self.step(input).1
    }

    #[inline]
    pub fn process_bp_normalized(&mut self, input: f32) -> f32 {
        self.step(input).1 * self.r
    }

    pub fn process_lp_in_place(&mut self, buf: &mut [f32]) {
        for x in buf.iter_mut() {
            *x = self.step(*x).0;
        }
    }
}

/// One-pole DC blocker.
#[derive(Clone, Copy, Debug, Default)]
pub struct DcBlocker {
    pole: f32,
    x: f32,
    y: f32,
}

impl DcBlocker {
    pub fn new(pole: f32) -> Self {
        Self { pole, x: 0.0, y: 0.0 }
    }

    pub fn process(&mut self, buf: &mut [f32]) {
        let (mut x, mut y) = (self.x, self.y);
        for s in buf.iter_mut() {
            let old_x = x;
            x = *s;
            y = y * self.pole + x - old_x;
            *s = y;
        }
        self.x = x;
        self.y = y;
    }
}

/// Cosine oscillator by IIR, with the approximate coefficient of stmlib.
/// Outputs 0..1.
#[derive(Clone, Copy, Debug, Default)]
pub struct CosineOscillator {
    y1: f32,
    y0: f32,
    iir_coefficient: f32,
    initial_amplitude: f32,
}

impl CosineOscillator {
    pub fn approximate(frequency: f32) -> Self {
        let mut o = Self::default();
        o.init_approximate(frequency);
        o
    }

    #[inline]
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

    #[inline]
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

/// A delay line of `N` samples (heap allocated).
#[derive(Clone, Debug)]
pub struct DelayLine<const N: usize> {
    line: Box<[f32]>,
    write_ptr: usize,
    delay: usize,
}

impl<const N: usize> Default for DelayLine<N> {
    fn default() -> Self {
        Self { line: vec![0.0; N].into_boxed_slice(), write_ptr: 0, delay: 1 }
    }
}

impl<const N: usize> DelayLine<N> {
    pub fn reset(&mut self) {
        self.line.fill(0.0);
        self.delay = 1;
        self.write_ptr = 0;
    }

    #[inline]
    pub fn set_delay(&mut self, delay: usize) {
        self.delay = delay;
    }

    #[inline]
    pub fn write(&mut self, sample: f32) {
        self.line[self.write_ptr] = sample;
        self.write_ptr = (self.write_ptr + N - 1) % N;
    }

    #[inline]
    pub fn allpass(&mut self, sample: f32, delay: usize, coefficient: f32) -> f32 {
        let read = self.line[(self.write_ptr + delay) % N];
        let write = sample + coefficient * read;
        self.write(write);
        -write * coefficient + read
    }

    #[inline]
    pub fn read(&self) -> f32 {
        self.line[(self.write_ptr + self.delay) % N]
    }

    #[inline]
    pub fn read_at(&self, delay: usize) -> f32 {
        self.line[(self.write_ptr + delay) % N]
    }

    #[inline]
    pub fn read_linear(&self, delay: f32) -> f32 {
        let (i, f) = integral_fractional(delay);
        let i = i.max(0) as usize;
        let a = self.line[(self.write_ptr + i) % N];
        let b = self.line[(self.write_ptr + i + 1) % N];
        a + (b - a) * f
    }

    #[inline]
    pub fn read_hermite(&self, delay: f32) -> f32 {
        let (i, f) = integral_fractional(delay);
        let t = self.write_ptr + i.max(0) as usize + N;
        let xm1 = self.line[(t - 1) % N];
        let x0 = self.line[t % N];
        let x1 = self.line[(t + 1) % N];
        let x2 = self.line[(t + 2) % N];
        let c = (x1 - xm1) * 0.5;
        let v = x0 - x1;
        let w = c + v;
        let a = w + v + (x2 - x0) * 0.5;
        let b_neg = w + a;
        (((a * f) - b_neg) * f + c) * f + x0
    }
}

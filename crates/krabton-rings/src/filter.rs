//! Filters, delay lines, the cosine oscillator and the parameter
//! interpolator of stmlib (`dsp/filter.h`, `dsp/delay_line.h`,
//! `dsp/cosine_oscillator.h`, `dsp/parameter_interpolator.h`).
//!
//! Copyright 2015 Emilie Gillet, MIT license. Rust port for Krabton.

use crate::dsp::integral_fractional;
use std::f64::consts::PI;

/// How `tan(pi * f)` is approximated when setting a cutoff.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Approx {
    Accurate,
    Fast,
    Dirty,
}

const PI_F: f32 = PI as f32;

#[inline]
pub fn tan_approx(f: f32, approx: Approx) -> f32 {
    match approx {
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

    pub fn process_lp_in_place(&mut self, buf: &mut [f32]) {
        for x in buf.iter_mut() {
            *x = self.step(*x).0;
        }
    }

    pub fn process_bp_buffer(&mut self, input: &[f32], out: &mut [f32]) {
        for (o, &x) in out.iter_mut().zip(input) {
            *o = self.step(x).1;
        }
    }
}

/// The naive Chamberlin state variable filter.
#[derive(Clone, Copy, Debug)]
pub struct NaiveSvf {
    f: f32,
    damp: f32,
    lp: f32,
    bp: f32,
}

impl Default for NaiveSvf {
    fn default() -> Self {
        let mut f = Self { f: 0.0, damp: 0.0, lp: 0.0, bp: 0.0 };
        f.set_f_q(0.01, 100.0);
        f
    }
}

impl NaiveSvf {
    /// The approximate (non-exact) frequency mapping, the only one Rings
    /// uses.
    pub fn set_f_q(&mut self, f: f32, resonance: f32) {
        let f = f.min(0.158);
        self.f = 2.0 * PI_F * f;
        self.damp = 1.0 / resonance;
    }

    /// One step; returns the high-pass output (the low-pass one is then in
    /// [`lp`](Self::lp)).
    #[inline]
    pub fn process_hp(&mut self, input: f32) -> f32 {
        let bp_normalized = self.bp * self.damp;
        let notch = input - bp_normalized;
        self.lp += self.f * self.bp;
        let hp = notch - self.lp;
        self.bp += self.f * hp;
        hp
    }

    #[inline]
    pub fn lp(&self) -> f32 {
        self.lp
    }

    /// `Split`, one sample: returns (low-pass, high-pass).
    #[inline]
    pub fn split(&mut self, input: f32) -> (f32, f32) {
        let hp = self.process_hp(input);
        (self.lp, hp)
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
        for s in buf.iter_mut() {
            *s = self.process_sample(*s);
        }
    }

    #[inline]
    pub fn process_sample(&mut self, s: f32) -> f32 {
        let old_x = self.x;
        self.x = s;
        self.y = self.y * self.pole + s - old_x;
        self.y
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
    #[allow(clippy::should_implement_trait)]
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

    /// Sets every sample of the line to `value`.
    pub fn fill(&mut self, value: f32) {
        self.line.fill(value);
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
        let read = self.line[(self.write_ptr + delay % N) % N];
        let write = sample + coefficient * read;
        self.write(write);
        -write * coefficient + read
    }

    /// `Read()`, at the delay set with [`set_delay`](Self::set_delay).
    #[inline]
    pub fn read(&self) -> f32 {
        self.line[(self.write_ptr + self.delay % N) % N]
    }

    /// `Read(float)`: linear interpolation.
    #[inline]
    pub fn read_linear(&self, delay: f32) -> f32 {
        let (i, f) = integral_fractional(delay.max(0.0));
        let i = i as usize % N;
        let a = self.line[(self.write_ptr + i) % N];
        let b = self.line[(self.write_ptr + i + 1) % N];
        a + (b - a) * f
    }

    #[inline]
    pub fn read_hermite(&self, delay: f32) -> f32 {
        let (i, f) = integral_fractional(delay.max(0.0));
        let t = self.write_ptr + i as usize % N + N;
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

/// A value that ramps linearly to a target over a block, one step per
/// `next()` (stmlib's `ParameterInterpolator`; store `value` back after).
pub struct Ramp {
    pub value: f32,
    increment: f32,
}

impl Ramp {
    #[inline]
    pub fn new(state: f32, target: f32, size: usize) -> Self {
        Self { value: state, increment: (target - state) / size.max(1) as f32 }
    }

    #[inline]
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> f32 {
        self.value += self.increment;
        self.value
    }
}

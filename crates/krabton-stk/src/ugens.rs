//! The STK unit generators the instruments are built from.
//!
//! Ported from the Synthesis ToolKit in C++: `OnePole`, `OneZero`,
//! `PoleZero`, `BiQuad`, `Iir`, `Fir` and `FormSwep` (`Filter.h` and
//! friends), the interpolating delay lines `DelayL` and `DelayA`, the
//! `Envelope` and `ADSR` generators, `Noise`, `SineWave`, `Modulate`, the
//! nonlinear `ReedTable`, `JetTable` and `BowTable`, and the `FileWvIn` /
//! `FileLoop` wave players (reading rawwaves kept in memory, see
//! [`crate::rawwaves`]).
//!
//! Copyright (c) 1995--2023 Perry R. Cook and Gary P. Scavone (STK license,
//! see the crate documentation). Rust port for Krabton.
//!
//! Differences from STK: every object works at the sample rate it is given
//! (there is no global `Stk::sampleRate`), state is `f64` like STK's default
//! `StkFloat`, delay lines are allocated once and setters that STK rejects
//! with a warning are clamped or ignored instead of printing. `Noise` uses a
//! per-instance xorshift generator instead of the C library `rand()`.

pub const PI: f64 = std::f64::consts::PI;
pub const TWO_PI: f64 = 2.0 * PI;
pub const ONE_OVER_128: f64 = 1.0 / 128.0;

/// STK's default sample rate. Constants that STK gives "per sample"
/// (envelope rates, decay radii, physics time steps) are taken to be meant
/// at this rate and converted with [`rate`] and [`radius`].
pub const REFERENCE_RATE: f64 = 44100.0;

/// Converts a per-sample increment meant for 44.1 kHz to `sr`, keeping the
/// time it takes the same.
#[inline]
pub fn rate(per_sample: f64, sr: f64) -> f64 {
    per_sample * REFERENCE_RATE / sr
}

/// Converts a per-sample decay factor or pole radius meant for 44.1 kHz to
/// `sr`, keeping the decay time (and resonance bandwidth) the same.
#[inline]
pub fn radius(r: f64, sr: f64) -> f64 {
    if r > 0.0 {
        r.powf(REFERENCE_RATE / sr)
    } else {
        r
    }
}

/// `Filter::phaseDelay`: the phase delay in samples of a filter with the
/// given numerator (times `gain`) and denominator at `frequency`.
pub fn phase_delay(b: &[f64], a: &[f64], gain: f64, frequency: f64, sr: f64) -> f64 {
    if frequency <= 0.0 || frequency > 0.5 * sr {
        return 0.0;
    }
    let omega_t = TWO_PI * frequency / sr;
    let (mut real, mut imag) = (0.0, 0.0);
    for (i, &bi) in b.iter().enumerate() {
        real += bi * (i as f64 * omega_t).cos();
        imag -= bi * (i as f64 * omega_t).sin();
    }
    real *= gain;
    imag *= gain;
    let mut phase = imag.atan2(real);
    let (mut real, mut imag) = (0.0, 0.0);
    for (i, &ai) in a.iter().enumerate() {
        real += ai * (i as f64 * omega_t).cos();
        imag -= ai * (i as f64 * omega_t).sin();
    }
    phase -= imag.atan2(real);
    phase = (-phase) % TWO_PI;
    phase / omega_t
}

// ---------------------------------------------------------------------------
// Filters

/// `OnePole`: y[n] = b0 g x[n] - a1 y[n-1].
#[derive(Clone, Debug)]
pub struct OnePole {
    pub gain: f64,
    b0: f64,
    a1: f64,
    y1: f64,
}

impl OnePole {
    pub fn new(pole: f64) -> Self {
        let mut f = Self { gain: 1.0, b0: 1.0, a1: 0.0, y1: 0.0 };
        f.set_pole(pole);
        f
    }

    /// Sets the pole, normalizing the peak gain to one. Ignored for
    /// |pole| >= 1 like in STK.
    pub fn set_pole(&mut self, pole: f64) {
        if !(pole.abs() < 1.0) {
            return;
        }
        self.b0 = if pole > 0.0 { 1.0 - pole } else { 1.0 + pole };
        self.a1 = -pole;
    }

    #[inline]
    pub fn tick(&mut self, input: f64) -> f64 {
        let y = self.b0 * self.gain * input - self.a1 * self.y1;
        self.y1 = y;
        y
    }

    #[inline]
    pub fn last_out(&self) -> f64 {
        self.y1
    }

    pub fn clear(&mut self) {
        self.y1 = 0.0;
    }

    pub fn phase_delay(&self, frequency: f64, sr: f64) -> f64 {
        phase_delay(&[self.b0], &[1.0, self.a1], self.gain, frequency, sr)
    }
}

/// `OneZero`: y[n] = b0 g x[n] + b1 g x[n-1].
#[derive(Clone, Debug)]
pub struct OneZero {
    pub gain: f64,
    b0: f64,
    b1: f64,
    x1: f64,
    last: f64,
}

impl OneZero {
    pub fn new(zero: f64) -> Self {
        let mut f = Self { gain: 1.0, b0: 1.0, b1: 0.0, x1: 0.0, last: 0.0 };
        f.set_zero(zero);
        f
    }

    pub fn set_zero(&mut self, zero: f64) {
        self.b0 = if zero > 0.0 { 1.0 / (1.0 + zero) } else { 1.0 / (1.0 - zero) };
        self.b1 = -zero * self.b0;
    }

    #[inline]
    pub fn tick(&mut self, input: f64) -> f64 {
        let x0 = self.gain * input;
        self.last = self.b1 * self.x1 + self.b0 * x0;
        self.x1 = x0;
        self.last
    }

    pub fn clear(&mut self) {
        self.x1 = 0.0;
        self.last = 0.0;
    }

    pub fn phase_delay(&self, frequency: f64, sr: f64) -> f64 {
        phase_delay(&[self.b0, self.b1], &[], self.gain, frequency, sr)
    }
}

/// `PoleZero`: one pole and one zero (DC blocker, allpass, tone holes).
#[derive(Clone, Debug)]
pub struct PoleZero {
    pub gain: f64,
    pub b0: f64,
    pub b1: f64,
    pub a1: f64,
    x1: f64,
    y1: f64,
}

impl Default for PoleZero {
    fn default() -> Self {
        Self { gain: 1.0, b0: 1.0, b1: 0.0, a1: 0.0, x1: 0.0, y1: 0.0 }
    }
}

impl PoleZero {
    /// `setBlockZero`: a DC blocking filter with the pole at `pole`.
    pub fn set_block_zero(&mut self, pole: f64) {
        if !(pole.abs() < 1.0) {
            return;
        }
        self.b0 = 1.0;
        self.b1 = -1.0;
        self.a1 = -pole;
    }

    /// A DC blocker with STK's default pole (0.99 at 44.1 kHz).
    pub fn dc_blocker(sr: f64) -> Self {
        let mut f = Self::default();
        f.set_block_zero(radius(0.99, sr));
        f
    }

    #[inline]
    pub fn tick(&mut self, input: f64) -> f64 {
        let x0 = self.gain * input;
        let y = self.b0 * x0 + self.b1 * self.x1 - self.a1 * self.y1;
        self.x1 = x0;
        self.y1 = y;
        y
    }

    #[inline]
    pub fn last_out(&self) -> f64 {
        self.y1
    }

    pub fn clear(&mut self) {
        self.x1 = 0.0;
        self.y1 = 0.0;
    }
}

/// `BiQuad`: a two-pole, two-zero filter.
#[derive(Clone, Debug)]
pub struct BiQuad {
    pub gain: f64,
    pub b0: f64,
    pub b1: f64,
    pub b2: f64,
    pub a1: f64,
    pub a2: f64,
    x1: f64,
    x2: f64,
    y1: f64,
    y2: f64,
}

impl Default for BiQuad {
    fn default() -> Self {
        Self { gain: 1.0, b0: 1.0, b1: 0.0, b2: 0.0, a1: 0.0, a2: 0.0, x1: 0.0, x2: 0.0, y1: 0.0, y2: 0.0 }
    }
}

impl BiQuad {
    pub fn set_coefficients(&mut self, b0: f64, b1: f64, b2: f64, a1: f64, a2: f64) {
        self.b0 = b0;
        self.b1 = b1;
        self.b2 = b2;
        self.a1 = a1;
        self.a2 = a2;
    }

    /// `setResonance`: poles at `frequency` with `radius`; with `normalize`
    /// the zeros go to DC and Nyquist for unity peak gain, otherwise the
    /// numerator becomes 1. Ignored for out-of-range arguments, like STK.
    pub fn set_resonance(&mut self, frequency: f64, radius: f64, normalize: bool, sr: f64) {
        if !(0.0..=0.5 * sr).contains(&frequency) || !(0.0..1.0).contains(&radius) {
            return;
        }
        self.a2 = radius * radius;
        self.a1 = -2.0 * radius * (TWO_PI * frequency / sr).cos();
        if normalize {
            self.b0 = 0.5 - 0.5 * self.a2;
            self.b1 = 0.0;
            self.b2 = -self.b0;
        } else {
            self.b0 = 1.0;
            self.b1 = 0.0;
            self.b2 = 0.0;
        }
    }

    pub fn set_equal_gain_zeroes(&mut self) {
        self.b0 = 1.0;
        self.b1 = 0.0;
        self.b2 = -1.0;
    }

    #[inline]
    pub fn tick(&mut self, input: f64) -> f64 {
        let x0 = self.gain * input;
        let y = self.b0 * x0 + self.b1 * self.x1 + self.b2 * self.x2 - self.a2 * self.y2 - self.a1 * self.y1;
        self.x2 = self.x1;
        self.x1 = x0;
        self.y2 = self.y1;
        self.y1 = y;
        y
    }

    #[inline]
    pub fn last_out(&self) -> f64 {
        self.y1
    }

    pub fn clear(&mut self) {
        self.x1 = 0.0;
        self.x2 = 0.0;
        self.y1 = 0.0;
        self.y2 = 0.0;
    }
}

/// `Iir`: a general direct-form filter, here limited to four coefficients
/// on each side (all the Recorder needs) so it never allocates.
#[derive(Clone, Debug)]
pub struct Iir {
    pub gain: f64,
    b: [f64; 4],
    a: [f64; 4],
    nb: usize,
    na: usize,
    x: [f64; 4],
    y: [f64; 4],
}

impl Default for Iir {
    fn default() -> Self {
        Self { gain: 1.0, b: [1.0, 0.0, 0.0, 0.0], a: [1.0, 0.0, 0.0, 0.0], nb: 1, na: 1, x: [0.0; 4], y: [0.0; 4] }
    }
}

impl Iir {
    /// `setCoefficients`: state is kept when the orders do not change, and
    /// everything is normalized by `a[0]`.
    pub fn set_coefficients(&mut self, b: &[f64], a: &[f64]) {
        let nb = b.len().clamp(1, 4);
        let na = a.len().clamp(1, 4);
        if a[0] == 0.0 || !a[0].is_finite() {
            return;
        }
        if nb != self.nb {
            self.nb = nb;
            self.x = [0.0; 4];
        }
        self.b[..nb].copy_from_slice(&b[..nb]);
        if na != self.na {
            self.na = na;
            self.y = [0.0; 4];
        }
        self.a[..na].copy_from_slice(&a[..na]);
        if self.a[0] != 1.0 {
            let a0 = self.a[0];
            for v in &mut self.b[..nb] {
                *v /= a0;
            }
            for v in &mut self.a[1..na] {
                *v /= a0;
            }
        }
    }

    #[inline]
    pub fn tick(&mut self, input: f64) -> f64 {
        self.y[0] = 0.0;
        self.x[0] = self.gain * input;
        for i in (1..self.nb).rev() {
            self.y[0] += self.b[i] * self.x[i];
            self.x[i] = self.x[i - 1];
        }
        self.y[0] += self.b[0] * self.x[0];
        for i in (1..self.na).rev() {
            self.y[0] += -self.a[i] * self.y[i];
            self.y[i] = self.y[i - 1];
        }
        self.y[0]
    }

    pub fn clear(&mut self) {
        self.x = [0.0; 4];
        self.y = [0.0; 4];
    }
}

/// `Fir` with two coefficients (the Twang loop filter).
#[derive(Clone, Debug)]
pub struct Fir2 {
    pub gain: f64,
    b0: f64,
    b1: f64,
    x1: f64,
}

impl Fir2 {
    pub fn new(b0: f64, b1: f64) -> Self {
        Self { gain: 1.0, b0, b1, x1: 0.0 }
    }

    #[inline]
    pub fn tick(&mut self, input: f64) -> f64 {
        let x0 = self.gain * input;
        let y = self.b1 * self.x1 + self.b0 * x0;
        self.x1 = x0;
        y
    }

    pub fn clear(&mut self) {
        self.x1 = 0.0;
    }

    pub fn phase_delay(&self, frequency: f64, sr: f64) -> f64 {
        phase_delay(&[self.b0, self.b1], &[], self.gain, frequency, sr)
    }
}

/// `FormSwep`: a sweepable formant (resonance) filter.
#[derive(Clone, Debug)]
pub struct FormSwep {
    gain: f64,
    b0: f64,
    b2: f64,
    a1: f64,
    a2: f64,
    x1: f64,
    x2: f64,
    y1: f64,
    y2: f64,
    dirty: bool,
    frequency: f64,
    radius: f64,
    start_frequency: f64,
    start_radius: f64,
    start_gain: f64,
    target_frequency: f64,
    target_radius: f64,
    target_gain: f64,
    delta_frequency: f64,
    delta_radius: f64,
    delta_gain: f64,
    sweep_state: f64,
    sweep_rate: f64,
}

impl Default for FormSwep {
    fn default() -> Self {
        Self {
            gain: 1.0,
            b0: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
            dirty: false,
            frequency: 0.0,
            radius: 0.0,
            start_frequency: 0.0,
            start_radius: 0.0,
            start_gain: 1.0,
            target_frequency: 0.0,
            target_radius: 0.0,
            target_gain: 1.0,
            delta_frequency: 0.0,
            delta_radius: 0.0,
            delta_gain: 0.0,
            sweep_state: 0.0,
            sweep_rate: 0.002,
        }
    }
}

impl FormSwep {
    pub fn set_resonance(&mut self, frequency: f64, radius: f64, sr: f64) {
        if !(0.0..=0.5 * sr).contains(&frequency) || !(0.0..1.0).contains(&radius) {
            return;
        }
        self.radius = radius;
        self.frequency = frequency;
        self.a2 = radius * radius;
        self.a1 = -2.0 * radius * (TWO_PI * frequency / sr).cos();
        self.b0 = 0.5 - 0.5 * self.a2;
        self.b2 = -self.b0;
    }

    /// `setTargets`: start sweeping to a new frequency, radius and gain.
    pub fn set_targets(&mut self, frequency: f64, radius: f64, gain: f64, sr: f64) {
        if !(0.0..=0.5 * sr).contains(&frequency) || !(0.0..1.0).contains(&radius) {
            return;
        }
        self.dirty = true;
        self.start_frequency = self.frequency;
        self.start_radius = self.radius;
        self.start_gain = self.gain;
        self.target_frequency = frequency;
        self.target_radius = radius;
        self.target_gain = gain;
        self.delta_frequency = frequency - self.frequency;
        self.delta_radius = radius - self.radius;
        self.delta_gain = gain - self.gain;
        self.sweep_state = 0.0;
    }

    pub fn set_sweep_rate(&mut self, rate: f64) {
        if (0.0..=1.0).contains(&rate) {
            self.sweep_rate = rate;
        }
    }

    #[inline]
    pub fn tick(&mut self, input: f64, sr: f64) -> f64 {
        if self.dirty {
            self.sweep_state += self.sweep_rate;
            if self.sweep_state >= 1.0 {
                self.sweep_state = 1.0;
                self.dirty = false;
                self.radius = self.target_radius;
                self.frequency = self.target_frequency;
                self.gain = self.target_gain;
            } else {
                self.radius = self.start_radius + self.delta_radius * self.sweep_state;
                self.frequency = self.start_frequency + self.delta_frequency * self.sweep_state;
                self.gain = self.start_gain + self.delta_gain * self.sweep_state;
            }
            self.set_resonance(self.frequency, self.radius, sr);
        }
        let x0 = self.gain * input;
        let y = self.b0 * x0 + self.b2 * self.x2 - self.a2 * self.y2 - self.a1 * self.y1;
        self.x2 = self.x1;
        self.x1 = x0;
        self.y2 = self.y1;
        self.y1 = y;
        y
    }

    pub fn clear(&mut self) {
        self.x1 = 0.0;
        self.x2 = 0.0;
        self.y1 = 0.0;
        self.y2 = 0.0;
    }
}

// ---------------------------------------------------------------------------
// Delay lines

/// `DelayL`: a delay line with linear interpolation.
#[derive(Clone, Debug)]
pub struct DelayL {
    buf: Vec<f64>,
    in_point: usize,
    out_point: usize,
    delay: f64,
    alpha: f64,
    om_alpha: f64,
    next_output: f64,
    do_next_out: bool,
    last: f64,
}

impl DelayL {
    /// A delay line holding up to `max_delay` samples (allocated here, once).
    pub fn new(delay: f64, max_delay: usize) -> Self {
        let mut d = Self {
            buf: vec![0.0; max_delay.max(1) + 1],
            in_point: 0,
            out_point: 0,
            delay: 0.0,
            alpha: 0.0,
            om_alpha: 1.0,
            next_output: 0.0,
            do_next_out: true,
            last: 0.0,
        };
        d.set_delay(delay);
        d
    }

    pub fn max_delay(&self) -> usize {
        self.buf.len() - 1
    }

    /// Sets the delay in samples, clamped to `0..=max_delay` (STK ignores
    /// out-of-range values with a warning).
    pub fn set_delay(&mut self, delay: f64) {
        let len = self.buf.len();
        let delay = if delay.is_finite() { delay.clamp(0.0, (len - 1) as f64) } else { 0.0 };
        let mut out_pointer = self.in_point as f64 - delay;
        self.delay = delay;
        while out_pointer < 0.0 {
            out_pointer += len as f64;
        }
        self.out_point = out_pointer as usize;
        self.alpha = out_pointer - self.out_point as f64;
        self.om_alpha = 1.0 - self.alpha;
        if self.out_point >= len {
            self.out_point = 0;
        }
        self.do_next_out = true;
    }

    pub fn delay(&self) -> f64 {
        self.delay
    }

    #[inline]
    pub fn last_out(&self) -> f64 {
        self.last
    }

    #[inline]
    fn next_out(&mut self) -> f64 {
        if self.do_next_out {
            let len = self.buf.len();
            let mut y = self.buf[self.out_point] * self.om_alpha;
            y += if self.out_point + 1 < len { self.buf[self.out_point + 1] } else { self.buf[0] } * self.alpha;
            self.next_output = y;
            self.do_next_out = false;
        }
        self.next_output
    }

    #[inline]
    pub fn tick(&mut self, input: f64) -> f64 {
        let len = self.buf.len();
        self.buf[self.in_point] = input;
        self.in_point += 1;
        if self.in_point == len {
            self.in_point = 0;
        }
        self.last = self.next_out();
        self.do_next_out = true;
        self.out_point += 1;
        if self.out_point == len {
            self.out_point = 0;
        }
        self.last
    }

    pub fn clear(&mut self) {
        self.buf.iter_mut().for_each(|v| *v = 0.0);
        self.last = 0.0;
        self.next_output = 0.0;
        self.do_next_out = true;
    }
}

/// `DelayA`: a delay line with first-order allpass interpolation.
#[derive(Clone, Debug)]
pub struct DelayA {
    buf: Vec<f64>,
    in_point: usize,
    out_point: usize,
    delay: f64,
    coeff: f64,
    ap_input: f64,
    next_output: f64,
    do_next_out: bool,
    last: f64,
}

impl DelayA {
    pub fn new(delay: f64, max_delay: usize) -> Self {
        let mut d = Self {
            buf: vec![0.0; max_delay.max(1) + 1],
            in_point: 0,
            out_point: 0,
            delay: 0.0,
            coeff: 0.0,
            ap_input: 0.0,
            next_output: 0.0,
            do_next_out: true,
            last: 0.0,
        };
        d.set_delay(delay);
        d
    }

    pub fn max_delay(&self) -> usize {
        self.buf.len() - 1
    }

    /// Sets the delay, clamped to `0.5..=max_delay`.
    pub fn set_delay(&mut self, delay: f64) {
        let len = self.buf.len();
        let delay = if delay.is_finite() { delay.clamp(0.5, (len - 1) as f64) } else { 0.5 };
        let mut out_pointer = self.in_point as f64 - delay + 1.0;
        self.delay = delay;
        while out_pointer < 0.0 {
            out_pointer += len as f64;
        }
        self.out_point = out_pointer as usize;
        if self.out_point >= len {
            self.out_point = 0;
        }
        let mut alpha = 1.0 + self.out_point as f64 - out_pointer;
        if alpha < 0.5 {
            self.out_point += 1;
            if self.out_point >= len {
                self.out_point -= len;
            }
            alpha += 1.0;
        }
        self.coeff = (1.0 - alpha) / (1.0 + alpha);
    }

    pub fn delay(&self) -> f64 {
        self.delay
    }

    #[inline]
    pub fn last_out(&self) -> f64 {
        self.last
    }

    #[inline]
    fn next_out(&mut self) -> f64 {
        if self.do_next_out {
            self.next_output = -self.coeff * self.last + self.ap_input + self.coeff * self.buf[self.out_point];
            self.do_next_out = false;
        }
        self.next_output
    }

    #[inline]
    pub fn tick(&mut self, input: f64) -> f64 {
        let len = self.buf.len();
        self.buf[self.in_point] = input;
        self.in_point += 1;
        if self.in_point == len {
            self.in_point = 0;
        }
        self.last = self.next_out();
        self.do_next_out = true;
        self.ap_input = self.buf[self.out_point];
        self.out_point += 1;
        if self.out_point == len {
            self.out_point = 0;
        }
        self.last
    }

    pub fn clear(&mut self) {
        self.buf.iter_mut().for_each(|v| *v = 0.0);
        self.last = 0.0;
        self.ap_input = 0.0;
        self.next_output = 0.0;
        self.do_next_out = true;
    }
}

// ---------------------------------------------------------------------------
// Envelopes

/// `Envelope`: a linear ramp to a target at a per-sample rate.
#[derive(Clone, Debug)]
pub struct Envelope {
    value: f64,
    target: f64,
    rate: f64,
    active: bool,
}

impl Default for Envelope {
    fn default() -> Self {
        Self { value: 0.0, target: 0.0, rate: 0.001, active: false }
    }
}

impl Envelope {
    pub fn set_rate(&mut self, rate: f64) {
        if rate >= 0.0 {
            self.rate = rate;
        }
    }

    pub fn set_target(&mut self, target: f64) {
        self.target = target;
        if self.value != self.target {
            self.active = true;
        }
    }

    pub fn set_value(&mut self, value: f64) {
        self.active = false;
        self.target = value;
        self.value = value;
    }

    pub fn key_on(&mut self) {
        self.set_target(1.0);
    }

    pub fn key_off(&mut self) {
        self.set_target(0.0);
    }

    pub fn value(&self) -> f64 {
        self.value
    }

    #[inline]
    pub fn tick(&mut self) -> f64 {
        if self.active {
            if self.target > self.value {
                self.value += self.rate;
                if self.value >= self.target {
                    self.value = self.target;
                    self.active = false;
                }
            } else {
                self.value -= self.rate;
                if self.value <= self.target {
                    self.value = self.target;
                    self.active = false;
                }
            }
        }
        self.value
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdsrState {
    Attack,
    Decay,
    Sustain,
    Release,
    Idle,
}

/// `ADSR`: attack, decay, sustain, release envelope.
#[derive(Clone, Debug)]
pub struct Adsr {
    state: AdsrState,
    value: f64,
    target: f64,
    attack_rate: f64,
    decay_rate: f64,
    release_rate: f64,
    release_time: f64,
    sustain_level: f64,
    sr: f64,
}

impl Adsr {
    pub fn new(sr: f64) -> Self {
        Self {
            state: AdsrState::Idle,
            value: 0.0,
            target: 0.0,
            attack_rate: rate(0.001, sr),
            decay_rate: rate(0.001, sr),
            release_rate: rate(0.005, sr),
            release_time: -1.0,
            sustain_level: 0.5,
            sr,
        }
    }

    pub fn key_on(&mut self) {
        if self.target <= 0.0 {
            self.target = 1.0;
        }
        self.state = AdsrState::Attack;
    }

    pub fn key_off(&mut self) {
        self.target = 0.0;
        self.state = AdsrState::Release;
        if self.release_time > 0.0 {
            self.release_rate = self.value / (self.release_time * self.sr);
        }
    }

    pub fn set_attack_rate(&mut self, r: f64) {
        if r >= 0.0 {
            self.attack_rate = r;
        }
    }

    pub fn set_release_rate(&mut self, r: f64) {
        if r >= 0.0 {
            self.release_rate = r;
            self.release_time = -1.0;
        }
    }

    pub fn set_sustain_level(&mut self, level: f64) {
        if level >= 0.0 {
            self.sustain_level = level;
        }
    }

    pub fn set_attack_time(&mut self, t: f64) {
        if t > 0.0 {
            self.attack_rate = 1.0 / (t * self.sr);
        }
    }

    pub fn set_decay_time(&mut self, t: f64) {
        if t > 0.0 {
            self.decay_rate = (1.0 - self.sustain_level) / (t * self.sr);
        }
    }

    pub fn set_release_time(&mut self, t: f64) {
        if t > 0.0 {
            self.release_rate = self.sustain_level / (t * self.sr);
            self.release_time = t;
        }
    }

    pub fn set_all_times(&mut self, a: f64, d: f64, s: f64, r: f64) {
        self.set_attack_time(a);
        self.set_sustain_level(s);
        self.set_decay_time(d);
        self.set_release_time(r);
    }

    /// `setTarget`: a new sustain level, approached from where it is now.
    pub fn set_target(&mut self, target: f64) {
        if target < 0.0 {
            return;
        }
        self.target = target;
        self.set_sustain_level(target);
        if self.value < self.target {
            self.state = AdsrState::Attack;
        }
        if self.value > self.target {
            self.state = AdsrState::Decay;
        }
    }

    pub fn state(&self) -> AdsrState {
        self.state
    }

    pub fn value(&self) -> f64 {
        self.value
    }

    #[inline]
    pub fn tick(&mut self) -> f64 {
        match self.state {
            AdsrState::Attack => {
                self.value += self.attack_rate;
                if self.value >= self.target {
                    self.value = self.target;
                    self.target = self.sustain_level;
                    self.state = AdsrState::Decay;
                }
            }
            AdsrState::Decay => {
                if self.value > self.sustain_level {
                    self.value -= self.decay_rate;
                    if self.value <= self.sustain_level {
                        self.value = self.sustain_level;
                        self.state = AdsrState::Sustain;
                    }
                } else {
                    self.value += self.decay_rate;
                    if self.value >= self.sustain_level {
                        self.value = self.sustain_level;
                        self.state = AdsrState::Sustain;
                    }
                }
            }
            AdsrState::Release => {
                self.value -= self.release_rate;
                if self.value <= 0.0 {
                    self.value = 0.0;
                    self.state = AdsrState::Idle;
                }
            }
            AdsrState::Sustain | AdsrState::Idle => {}
        }
        self.value
    }
}

// ---------------------------------------------------------------------------
// Generators

/// `Noise`: uniform white noise in [-1, 1).
#[derive(Clone, Debug)]
pub struct Noise {
    state: u32,
    last: f64,
}

impl Noise {
    pub fn new(seed: u32) -> Self {
        Self { state: seed.wrapping_mul(2_654_435_761).max(1), last: 0.0 }
    }

    #[inline]
    pub fn tick(&mut self) -> f64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.state = x;
        self.last = 2.0 * (x as f64 / 4_294_967_296.0) - 1.0;
        self.last
    }

    #[inline]
    pub fn last_out(&self) -> f64 {
        self.last
    }
}

const SINE_TABLE_SIZE: usize = 2048;

fn sine_table() -> &'static [f64] {
    static TABLE: std::sync::OnceLock<Vec<f64>> = std::sync::OnceLock::new();
    TABLE.get_or_init(|| {
        (0..=SINE_TABLE_SIZE).map(|i| (TWO_PI * i as f64 / SINE_TABLE_SIZE as f64).sin()).collect()
    })
}

/// `SineWave`: a 2048-point table oscillator with linear interpolation.
#[derive(Clone, Debug)]
pub struct SineWave {
    table: &'static [f64],
    time: f64,
    rate: f64,
    sr: f64,
}

impl SineWave {
    pub fn new(sr: f64) -> Self {
        Self { table: sine_table(), time: 0.0, rate: 1.0, sr }
    }

    pub fn set_frequency(&mut self, frequency: f64) {
        self.rate = SINE_TABLE_SIZE as f64 * frequency / self.sr;
    }

    #[inline]
    pub fn tick(&mut self) -> f64 {
        let size = SINE_TABLE_SIZE as f64;
        if !(self.time.is_finite() && self.rate.is_finite()) {
            self.time = 0.0;
            self.rate = 0.0;
        }
        if !(0.0..size).contains(&self.time) {
            self.time = self.time.rem_euclid(size);
            if self.time >= size {
                self.time = 0.0;
            }
        }
        let i = self.time as usize;
        let alpha = self.time - i as f64;
        let mut y = self.table[i];
        y += alpha * (self.table[i + 1] - y);
        self.time += self.rate;
        y
    }
}

/// `Modulate`: periodic vibrato plus filtered random modulation.
#[derive(Clone, Debug)]
pub struct Modulate {
    vibrato: SineWave,
    noise: Noise,
    filter: OnePole,
    vibrato_gain: f64,
    noise_rate: u32,
    noise_counter: u32,
}

impl Modulate {
    pub fn new(sr: f64, seed: u32) -> Self {
        let mut vibrato = SineWave::new(sr);
        vibrato.set_frequency(6.0);
        let noise_rate = (330.0 * sr / 22050.0) as u32;
        let mut filter = OnePole::new(radius(0.999, sr));
        filter.gain = 0.05;
        Self { vibrato, noise: Noise::new(seed), filter, vibrato_gain: 0.04, noise_rate, noise_counter: noise_rate }
    }

    pub fn set_vibrato_rate(&mut self, r: f64) {
        self.vibrato.set_frequency(r);
    }

    pub fn set_vibrato_gain(&mut self, g: f64) {
        self.vibrato_gain = g;
    }

    pub fn set_random_gain(&mut self, g: f64) {
        self.filter.gain = g;
    }

    #[inline]
    pub fn tick(&mut self) -> f64 {
        let mut y = self.vibrato_gain * self.vibrato.tick();
        let c = self.noise_counter;
        self.noise_counter += 1;
        if c >= self.noise_rate {
            self.noise.tick();
            self.noise_counter = 0;
        }
        y += self.filter.tick(self.noise.last_out());
        y
    }
}

// ---------------------------------------------------------------------------
// Nonlinear tables

/// `ReedTable`: a one-breakpoint reed table, clipped to [-1, 1].
#[derive(Clone, Debug)]
pub struct ReedTable {
    pub offset: f64,
    pub slope: f64,
}

impl Default for ReedTable {
    fn default() -> Self {
        Self { offset: 0.6, slope: -0.8 }
    }
}

impl ReedTable {
    #[inline]
    pub fn tick(&self, input: f64) -> f64 {
        (self.offset + self.slope * input).clamp(-1.0, 1.0)
    }
}

/// `JetTable`: the cubic jet nonlinearity x (x^2 - 1), clipped to [-1, 1].
#[inline]
pub fn jet_table(input: f64) -> f64 {
    (input * (input * input - 1.0)).clamp(-1.0, 1.0)
}

/// `BowTable`: the bow friction curve (|slope (x + offset)| + 0.75)^-4.
#[derive(Clone, Debug)]
pub struct BowTable {
    pub offset: f64,
    pub slope: f64,
    pub min_output: f64,
    pub max_output: f64,
}

impl Default for BowTable {
    fn default() -> Self {
        Self { offset: 0.0, slope: 0.1, min_output: 0.01, max_output: 0.98 }
    }
}

impl BowTable {
    #[inline]
    pub fn tick(&self, input: f64) -> f64 {
        let sample = (input + self.offset) * self.slope;
        let y = (sample.abs() + 0.75).powi(-4);
        y.max(self.min_output).min(self.max_output)
    }
}

// ---------------------------------------------------------------------------
// Wave players

/// `FileWvIn` reading a normalized rawwave from memory: plays once, then
/// outputs zero until reset.
#[derive(Clone, Debug)]
pub struct WvIn {
    data: &'static [f64],
    time: f64,
    rate: f64,
    finished: bool,
}

impl WvIn {
    /// A player for `data` (recorded at 22050 Hz like all STK rawwaves),
    /// with STK's default rate for `sr`. It starts out finished (silent).
    pub fn new(data: &'static [f64], sr: f64) -> Self {
        Self { data, time: 0.0, rate: 22050.0 / sr, finished: true }
    }

    pub fn set_rate(&mut self, rate: f64) {
        if rate.is_finite() {
            self.rate = rate;
        }
    }

    pub fn reset(&mut self) {
        self.time = 0.0;
        self.finished = false;
    }

    pub fn is_finished(&self) -> bool {
        self.finished
    }

    #[inline]
    pub fn tick(&mut self) -> f64 {
        if self.finished {
            return 0.0;
        }
        let last = self.data.len() as f64 - 1.0;
        if self.time < 0.0 || self.time > last {
            self.finished = true;
            return 0.0;
        }
        let i = self.time as usize;
        let alpha = self.time - i as f64;
        let mut y = self.data[i];
        if alpha > 0.0 {
            y += alpha * (self.data[i + 1] - y);
        }
        self.time += self.rate;
        y
    }
}

/// `FileLoop` reading a normalized rawwave from memory, looping forever.
/// `data` holds the wave plus a copy of its first sample at the end.
#[derive(Clone, Debug)]
pub struct WaveLoop {
    data: &'static [f64],
    time: f64,
    rate: f64,
}

impl WaveLoop {
    pub fn new(data: &'static [f64], sr: f64) -> Self {
        Self { data, time: 0.0, rate: 22050.0 / sr }
    }

    /// Number of samples in one cycle of the loop.
    pub fn size(&self) -> f64 {
        (self.data.len() - 1) as f64
    }

    pub fn set_rate(&mut self, rate: f64) {
        if rate.is_finite() {
            self.rate = rate;
        }
    }

    #[inline]
    pub fn tick(&mut self) -> f64 {
        let size = self.size();
        if !self.time.is_finite() {
            self.time = 0.0;
        }
        if !(0.0..size).contains(&self.time) {
            self.time = self.time.rem_euclid(size);
            if self.time >= size {
                self.time = 0.0;
            }
        }
        let i = self.time as usize;
        let alpha = self.time - i as f64;
        let mut y = self.data[i];
        if alpha > 0.0 {
            y += alpha * (self.data[i + 1] - y);
        }
        self.time += self.rate;
        y
    }
}

//! The delay-line effects engine and the effects built with it: the reverb
//! of the String and Reverb model and the easter egg, and the chorus and
//! ensemble of the easter egg (`rings/dsp/fx/fx_engine.h`, `reverb.h`,
//! `chorus.h`, `ensemble.h`).
//!
//! Copyright 2015 Emilie Gillet, MIT license. Rust port for Krabton.

use crate::dsp::{clip16, integral_fractional, interpolate, SAMPLE_RATE};
use crate::filter::CosineOscillator;
use crate::resources::LUT_SINE;

/// 16-bit delay memory, saturating like the module's.
#[inline]
fn compress(value: f32) -> i16 {
    clip16((value * 32768.0) as i32) as i16
}

#[inline]
fn decompress(value: i16) -> f32 {
    value as f32 / 32768.0
}

/// A delay line inside the engine's memory.
#[derive(Clone, Copy, Debug)]
pub struct Line {
    base: i32,
    length: i32,
}

/// Lays out delay lines back to back, with one sample of slack each.
const fn layout<const K: usize>(lengths: [i32; K]) -> [Line; K] {
    let mut lines = [Line { base: 0, length: 0 }; K];
    let mut base = 0;
    let mut i = 0;
    while i < K {
        lines[i] = Line { base, length: lengths[i] };
        base += lengths[i] + 1;
        i += 1;
    }
    lines
}

/// Reads at the end of the line.
const TAIL: i32 = -1;

/// The engine, with `SIZE` samples of 16-bit memory.
pub struct FxEngine<const SIZE: usize> {
    buffer: Box<[i16]>,
    write_ptr: i32,
    lfo: [CosineOscillator; 2],
}

impl<const SIZE: usize> FxEngine<SIZE> {
    pub fn new() -> Self {
        Self { buffer: vec![0; SIZE].into_boxed_slice(), write_ptr: 0, lfo: [CosineOscillator::default(); 2] }
    }

    pub fn clear(&mut self) {
        self.buffer.fill(0);
        self.write_ptr = 0;
    }

    pub fn set_lfo_frequency(&mut self, index: usize, frequency: f32) {
        self.lfo[index].init_approximate(frequency * 32.0);
    }

    #[inline]
    pub fn start(&mut self) -> Context<'_, SIZE> {
        self.write_ptr -= 1;
        if self.write_ptr < 0 {
            self.write_ptr += SIZE as i32;
        }
        let lfo_value = if self.write_ptr & 31 == 0 {
            [self.lfo[0].next(), self.lfo[1].next()]
        } else {
            [self.lfo[0].value(), self.lfo[1].value()]
        };
        Context { accumulator: 0.0, previous_read: 0.0, lfo_value, buffer: &mut self.buffer, write_ptr: self.write_ptr }
    }
}

impl<const SIZE: usize> Default for FxEngine<SIZE> {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Context<'a, const SIZE: usize> {
    accumulator: f32,
    previous_read: f32,
    lfo_value: [f32; 2],
    buffer: &'a mut [i16],
    write_ptr: i32,
}

impl<const SIZE: usize> Context<'_, SIZE> {
    const MASK: i32 = SIZE as i32 - 1;

    #[inline]
    fn index(&self, d: Line, offset: i32) -> usize {
        let offset = if offset == TAIL { d.length - 1 } else { offset };
        ((self.write_ptr + d.base + offset) & Self::MASK) as usize
    }

    #[inline]
    pub fn load(&mut self, value: f32) {
        self.accumulator = value;
    }

    /// `Read(value, scale)`: adds an outside value.
    #[inline]
    pub fn read_value(&mut self, value: f32, scale: f32) {
        self.accumulator += value * scale;
    }

    /// `Write(value, scale)`: `value = acc; acc *= scale`.
    #[inline]
    pub fn write_value(&mut self, value: &mut f32, scale: f32) {
        *value = self.accumulator;
        self.accumulator *= scale;
    }

    #[inline]
    pub fn write(&mut self, d: Line, offset: i32, scale: f32) {
        let i = self.index(d, offset);
        self.buffer[i] = compress(self.accumulator);
        self.accumulator *= scale;
    }

    #[inline]
    pub fn write_all_pass(&mut self, d: Line, scale: f32) {
        self.write(d, 0, scale);
        self.accumulator += self.previous_read;
    }

    #[inline]
    pub fn read(&mut self, d: Line, offset: i32, scale: f32) {
        let r = decompress(self.buffer[self.index(d, offset)]);
        self.previous_read = r;
        self.accumulator += r * scale;
    }

    #[inline]
    pub fn lp(&mut self, state: &mut f32, coefficient: f32) {
        *state += coefficient * (self.accumulator - *state);
        self.accumulator = *state;
    }

    /// Reads `d` at a fractional `offset`, linearly interpolated.
    #[inline]
    pub fn interpolate(&mut self, d: Line, offset: f32, scale: f32) {
        let (i, f) = integral_fractional(offset);
        let a = decompress(self.buffer[((self.write_ptr + i + d.base) & Self::MASK) as usize]);
        let b = decompress(self.buffer[((self.write_ptr + i + d.base + 1) & Self::MASK) as usize]);
        let x = a + (b - a) * f;
        self.previous_read = x;
        self.accumulator += x * scale;
    }

    /// Like [`interpolate`](Self::interpolate), with the offset modulated
    /// by one of the engine's LFOs.
    #[inline]
    pub fn interpolate_lfo(&mut self, d: Line, offset: f32, lfo: usize, amplitude: f32, scale: f32) {
        let offset = offset + amplitude * self.lfo_value[lfo];
        self.interpolate(d, offset, scale);
    }
}

/// The Griesinger-style plate reverb, with 16-bit memory.
pub struct Reverb {
    engine: FxEngine<32768>,
    amount: f32,
    input_gain: f32,
    reverb_time: f32,
    diffusion: f32,
    lp: f32,
    lp_decay_1: f32,
    lp_decay_2: f32,
}

impl Default for Reverb {
    fn default() -> Self {
        let mut engine = FxEngine::new();
        engine.set_lfo_frequency(0, 0.5 / SAMPLE_RATE);
        engine.set_lfo_frequency(1, 0.3 / SAMPLE_RATE);
        Self { engine, amount: 0.0, input_gain: 0.0, reverb_time: 0.0, diffusion: 0.625, lp: 0.7, lp_decay_1: 0.0, lp_decay_2: 0.0 }
    }
}

impl Reverb {
    const LINES: [Line; 10] = layout([150, 214, 319, 527, 2182, 2690, 4501, 2525, 2197, 6312]);

    pub fn clear(&mut self) {
        self.engine.clear();
    }

    /// Clears the memory and the filters.
    pub fn reset(&mut self) {
        self.engine.clear();
        self.lp_decay_1 = 0.0;
        self.lp_decay_2 = 0.0;
    }

    pub fn set_amount(&mut self, amount: f32) {
        self.amount = amount;
    }

    pub fn set_input_gain(&mut self, gain: f32) {
        self.input_gain = gain;
    }

    pub fn set_time(&mut self, time: f32) {
        self.reverb_time = time;
    }

    pub fn set_diffusion(&mut self, diffusion: f32) {
        self.diffusion = diffusion;
    }

    pub fn set_lp(&mut self, lp: f32) {
        self.lp = lp;
    }

    pub fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        // This is the Griesinger topology described in the Dattorro paper
        // (4 AP diffusers on the input, then a loop of 2x 2AP+1Delay).
        // Modulation is applied to the two long delays for a slow
        // shimmer/chorus effect.
        let [ap1, ap2, ap3, ap4, dap1a, dap1b, del1, dap2a, dap2b, del2] = Self::LINES;
        let kap = self.diffusion;
        let klp = self.lp;
        let krt = self.reverb_time;
        let amount = self.amount;
        let gain = self.input_gain;
        let mut lp_1 = self.lp_decay_1;
        let mut lp_2 = self.lp_decay_2;
        for (l, r) in left.iter_mut().zip(right.iter_mut()) {
            let mut wet = 0.0;
            let mut apout = 0.0;
            let mut c = self.engine.start();

            c.read_value(*l + *r, gain);

            // Diffuse through 4 allpasses.
            for ap in [ap1, ap2, ap3, ap4] {
                c.read(ap, TAIL, kap);
                c.write_all_pass(ap, -kap);
            }
            c.write_value(&mut apout, 1.0);

            // Main reverb loop.
            c.load(apout);
            c.interpolate_lfo(del2, 6261.0, 1, 50.0, krt);
            c.lp(&mut lp_1, klp);
            c.read(dap1a, TAIL, -kap);
            c.write_all_pass(dap1a, kap);
            c.read(dap1b, TAIL, kap);
            c.write_all_pass(dap1b, -kap);
            c.write(del1, 0, 2.0);
            c.write_value(&mut wet, 0.0);

            *l += (wet - *l) * amount;

            c.load(apout);
            c.interpolate_lfo(del1, 4460.0, 0, 40.0, krt);
            c.lp(&mut lp_2, klp);
            c.read(dap2a, TAIL, kap);
            c.write_all_pass(dap2a, -kap);
            c.read(dap2b, TAIL, -kap);
            c.write_all_pass(dap2b, kap);
            c.write(del2, 0, 2.0);
            c.write_value(&mut wet, 0.0);

            *r += (wet - *r) * amount;
        }
        self.lp_decay_1 = lp_1;
        self.lp_decay_2 = lp_2;
    }
}

/// Stereo chorus of the easter egg.
pub struct Chorus {
    engine: FxEngine<2048>,
    amount: f32,
    depth: f32,
    phase_1: f32,
    phase_2: f32,
}

impl Default for Chorus {
    fn default() -> Self {
        Self { engine: FxEngine::new(), amount: 0.0, depth: 0.0, phase_1: 0.0, phase_2: 0.0 }
    }
}

impl Chorus {
    const LINE: Line = layout([2047])[0];

    pub fn clear(&mut self) {
        self.engine.clear();
    }

    pub fn set_amount(&mut self, amount: f32) {
        self.amount = amount;
    }

    pub fn set_depth(&mut self, depth: f32) {
        self.depth = depth * 384.0;
    }

    pub fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        let line = Self::LINE;
        for (l, r) in left.iter_mut().zip(right.iter_mut()) {
            let mut c = self.engine.start();
            let dry_amount = 1.0 - self.amount * 0.5;

            // Update LFO.
            self.phase_1 += 4.17e-06;
            if self.phase_1 >= 1.0 {
                self.phase_1 -= 1.0;
            }
            self.phase_2 += 5.417e-06;
            if self.phase_2 >= 1.0 {
                self.phase_2 -= 1.0;
            }
            let sin_1 = interpolate(&LUT_SINE, self.phase_1, 4096.0);
            let cos_1 = interpolate(&LUT_SINE, self.phase_1 + 0.25, 4096.0);
            let sin_2 = interpolate(&LUT_SINE, self.phase_2, 4096.0);
            let cos_2 = interpolate(&LUT_SINE, self.phase_2 + 0.25, 4096.0);

            let mut wet = 0.0;

            // Sum L & R channel to send to chorus line.
            c.read_value(*l, 0.5);
            c.read_value(*r, 0.5);
            c.write(line, 0, 0.0);

            c.interpolate(line, sin_1 * self.depth + 1200.0, 0.5);
            c.interpolate(line, sin_2 * self.depth + 800.0, 0.5);
            c.write_value(&mut wet, 0.0);
            *l = wet * self.amount + *l * dry_amount;

            c.interpolate(line, cos_1 * self.depth + 800.0, 0.5);
            c.interpolate(line, cos_2 * self.depth + 1200.0, 0.5);
            c.write_value(&mut wet, 0.0);
            *r = wet * self.amount + *r * dry_amount;
        }
    }
}

/// Three-phase string ensemble of the easter egg.
pub struct Ensemble {
    engine: FxEngine<4096>,
    amount: f32,
    depth: f32,
    phase_1: f32,
    phase_2: f32,
}

impl Default for Ensemble {
    fn default() -> Self {
        Self { engine: FxEngine::new(), amount: 0.0, depth: 0.0, phase_1: 0.0, phase_2: 0.0 }
    }
}

impl Ensemble {
    const LINES: [Line; 2] = layout([2047, 2047]);

    pub fn clear(&mut self) {
        self.engine.clear();
    }

    pub fn set_amount(&mut self, amount: f32) {
        self.amount = amount;
    }

    pub fn set_depth(&mut self, depth: f32) {
        self.depth = depth * 128.0;
    }

    pub fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        let [line_l, line_r] = Self::LINES;
        for (l, r) in left.iter_mut().zip(right.iter_mut()) {
            let mut c = self.engine.start();
            let dry_amount = 1.0 - self.amount * 0.5;

            // Update LFO.
            self.phase_1 += 1.57e-05;
            if self.phase_1 >= 1.0 {
                self.phase_1 -= 1.0;
            }
            self.phase_2 += 1.37e-04;
            if self.phase_2 >= 1.0 {
                self.phase_2 -= 1.0;
            }
            let phi_1 = (self.phase_1 * 4096.0) as i32;
            let slow_0 = LUT_SINE[(phi_1 & 4095) as usize];
            let slow_120 = LUT_SINE[((phi_1 + 1365) & 4095) as usize];
            let slow_240 = LUT_SINE[((phi_1 + 2730) & 4095) as usize];
            let phi_2 = (self.phase_2 * 4096.0) as i32;
            let fast_0 = LUT_SINE[(phi_2 & 4095) as usize];
            let fast_120 = LUT_SINE[((phi_2 + 1365) & 4095) as usize];
            let fast_240 = LUT_SINE[((phi_2 + 2730) & 4095) as usize];

            let a = self.depth;
            let b = self.depth * 0.1;

            let mod_1 = slow_0 * a + fast_0 * b;
            let mod_2 = slow_120 * a + fast_120 * b;
            let mod_3 = slow_240 * a + fast_240 * b;

            let mut wet = 0.0;

            // Sum L & R channel to send to chorus line.
            c.read_value(*l, 1.0);
            c.write(line_l, 0, 0.0);
            c.read_value(*r, 1.0);
            c.write(line_r, 0, 0.0);

            c.interpolate(line_l, mod_1 + 1024.0, 0.33);
            c.interpolate(line_l, mod_2 + 1024.0, 0.33);
            c.interpolate(line_r, mod_3 + 1024.0, 0.33);
            c.write_value(&mut wet, 0.0);
            *l = wet * self.amount + *l * dry_amount;

            c.interpolate(line_r, mod_1 + 1024.0, 0.33);
            c.interpolate(line_r, mod_2 + 1024.0, 0.33);
            c.interpolate(line_l, mod_3 + 1024.0, 0.33);
            c.write_value(&mut wet, 0.0);
            *r = wet * self.amount + *r * dry_amount;
        }
    }
}

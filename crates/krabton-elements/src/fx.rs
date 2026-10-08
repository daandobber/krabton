//! The delay-line effects engine and the two effects built with it: the
//! diffuser of the blow exciter and the reverb (`elements/dsp/fx/*.h`).
//!
//! Copyright 2014 Emilie Gillet, MIT license. Rust port for Krabton.

use crate::dsp::{clip16, integral_fractional};
use crate::filter::CosineOscillator;
use crate::SAMPLE_RATE;

/// How samples are stored in the delay memory.
pub trait Storage: Copy + Default {
    fn decompress(self) -> f32;
    fn compress(value: f32) -> Self;
}

impl Storage for f32 {
    #[inline]
    fn decompress(self) -> f32 {
        self
    }
    #[inline]
    fn compress(value: f32) -> Self {
        value
    }
}

/// 16-bit storage, saturating like the module's.
impl Storage for i16 {
    #[inline]
    fn decompress(self) -> f32 {
        self as f32 / 32768.0
    }
    #[inline]
    fn compress(value: f32) -> Self {
        clip16((value * 32768.0) as i32) as i16
    }
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

pub struct FxEngine<T: Storage, const SIZE: usize> {
    buffer: Box<[T]>,
    write_ptr: i32,
    lfo: [CosineOscillator; 2],
}

impl<T: Storage, const SIZE: usize> FxEngine<T, SIZE> {
    pub fn new() -> Self {
        Self { buffer: vec![T::default(); SIZE].into_boxed_slice(), write_ptr: 0, lfo: [CosineOscillator::default(); 2] }
    }

    pub fn clear(&mut self) {
        self.buffer.fill(T::default());
        self.write_ptr = 0;
    }

    pub fn set_lfo_frequency(&mut self, index: usize, frequency: f32) {
        self.lfo[index].init_approximate(frequency * 32.0);
    }

    #[inline]
    pub fn start(&mut self) -> Context<'_, T, SIZE> {
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

impl<T: Storage, const SIZE: usize> Default for FxEngine<T, SIZE> {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Context<'a, T: Storage, const SIZE: usize> {
    accumulator: f32,
    previous_read: f32,
    lfo_value: [f32; 2],
    buffer: &'a mut [T],
    write_ptr: i32,
}

impl<T: Storage, const SIZE: usize> Context<'_, T, SIZE> {
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

    #[inline]
    pub fn read_value(&mut self, value: f32, scale: f32) {
        self.accumulator += value * scale;
    }

    /// `value = acc; acc *= scale`.
    #[inline]
    pub fn write_value(&mut self, value: &mut f32, scale: f32) {
        *value = self.accumulator;
        self.accumulator *= scale;
    }

    #[inline]
    pub fn write(&mut self, d: Line, offset: i32, scale: f32) {
        let i = self.index(d, offset);
        self.buffer[i] = T::compress(self.accumulator);
        self.accumulator *= scale;
    }

    #[inline]
    pub fn write_all_pass(&mut self, d: Line, scale: f32) {
        self.write(d, 0, scale);
        self.accumulator += self.previous_read;
    }

    #[inline]
    pub fn read(&mut self, d: Line, offset: i32, scale: f32) {
        let r = self.buffer[self.index(d, offset)].decompress();
        self.previous_read = r;
        self.accumulator += r * scale;
    }

    #[inline]
    pub fn lp(&mut self, state: &mut f32, coefficient: f32) {
        *state += coefficient * (self.accumulator - *state);
        self.accumulator = *state;
    }

    #[inline]
    pub fn interpolate_lfo(&mut self, d: Line, offset: f32, lfo: usize, amplitude: f32, scale: f32) {
        let offset = offset + amplitude * self.lfo_value[lfo];
        let (i, f) = integral_fractional(offset);
        let a = self.buffer[((self.write_ptr + i + d.base) & Self::MASK) as usize].decompress();
        let b = self.buffer[((self.write_ptr + i + d.base + 1) & Self::MASK) as usize].decompress();
        let x = a + (b - a) * f;
        self.previous_read = x;
        self.accumulator += x * scale;
    }
}

/// Four all-pass filters smearing the blow exciter.
pub struct Diffuser {
    engine: FxEngine<f32, 1024>,
}

impl Default for Diffuser {
    fn default() -> Self {
        Self { engine: FxEngine::new() }
    }
}

impl Diffuser {
    const LINES: [Line; 4] = layout([126, 180, 269, 444]);

    pub fn clear(&mut self) {
        self.engine.clear();
    }

    pub fn process(&mut self, in_out: &mut [f32]) {
        let [ap1, ap2, ap3, ap4] = Self::LINES;
        const KAP: f32 = 0.625;
        for x in in_out.iter_mut() {
            let mut c = self.engine.start();
            c.read_value(*x, 1.0);
            for ap in [ap1, ap2, ap3, ap4] {
                c.read(ap, TAIL, KAP);
                c.write_all_pass(ap, -KAP);
            }
            c.write_value(x, 0.0);
        }
    }
}

/// The Griesinger-style plate reverb of Elements, with 16-bit memory.
pub struct Reverb {
    engine: FxEngine<i16, 32768>,
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
        // Modulation is applied in the loop of the first diffuser AP for
        // additional smearing; and to the two long delays for a slow shimmer
        // and chorus effect.
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

            // Smear AP1 inside the loop.
            c.interpolate_lfo(ap1, 10.0, 0, 80.0, 1.0);
            c.write(ap1, 100, 0.0);

            c.read_value(*l + *r, gain);

            // Diffuse through 4 allpasses.
            for ap in [ap1, ap2, ap3, ap4] {
                c.read(ap, TAIL, kap);
                c.write_all_pass(ap, -kap);
            }
            c.write_value(&mut apout, 1.0);

            // Main reverb loop.
            c.load(apout);
            c.interpolate_lfo(del2, 6211.0, 1, 100.0, krt);
            c.lp(&mut lp_1, klp);
            c.read(dap1a, TAIL, -kap);
            c.write_all_pass(dap1a, kap);
            c.read(dap1b, TAIL, kap);
            c.write_all_pass(dap1b, -kap);
            c.write(del1, 0, 2.0);
            c.write_value(&mut wet, 0.0);

            *l += (wet - *l) * amount;

            c.load(apout);
            c.read(del1, TAIL, krt);
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

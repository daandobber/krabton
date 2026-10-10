//! The delay-line effects engine and the three effects built with it: the
//! diffuser, the pitch shifter of the looping delay mode and the reverb
//! (`clouds/dsp/fx/fx_engine.h`, `diffuser.h`, `pitch_shifter.h`,
//! `reverb.h`).
//!
//! Copyright 2014 Emilie Gillet, MIT license. Rust port for Krabton.

use crate::dsp::{clip16, integral_fractional, one_pole, CosineOscillator};

/// How samples are stored in the delay memory.
pub trait Storage: Copy + Default + Send + 'static {
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

/// 12-bit format: 16-bit words, 4096 per unit (room for +/-8).
#[derive(Clone, Copy, Debug, Default)]
pub struct Fixed12(i16);

impl Storage for Fixed12 {
    #[inline]
    fn decompress(self) -> f32 {
        self.0 as f32 / 4096.0
    }
    #[inline]
    fn compress(value: f32) -> Self {
        Fixed12(clip16((value * 4096.0) as i32) as i16)
    }
}

/// 16-bit format.
#[derive(Clone, Copy, Debug, Default)]
pub struct Fixed16(i16);

impl Storage for Fixed16 {
    #[inline]
    fn decompress(self) -> f32 {
        self.0 as f32 / 32768.0
    }
    #[inline]
    fn compress(value: f32) -> Self {
        Fixed16(clip16((value * 32768.0) as i32) as i16)
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
        (self.write_ptr.wrapping_add(d.base).wrapping_add(offset) & Self::MASK) as usize
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
    pub fn interpolate(&mut self, d: Line, offset: f32, scale: f32) {
        let (i, f) = integral_fractional(offset);
        let base = self.write_ptr.wrapping_add(i).wrapping_add(d.base);
        let a = self.buffer[(base & Self::MASK) as usize].decompress();
        let b = self.buffer[(base.wrapping_add(1) & Self::MASK) as usize].decompress();
        let x = a + (b - a) * f;
        self.previous_read = x;
        self.accumulator += x * scale;
    }

    #[inline]
    pub fn interpolate_lfo(&mut self, d: Line, offset: f32, lfo: usize, amplitude: f32, scale: f32) {
        let offset = offset + amplitude * self.lfo_value[lfo];
        self.interpolate(d, offset, scale);
    }
}

/// Four all-pass filters per channel, smearing the output in time.
pub struct Diffuser {
    engine: FxEngine<f32, 2048>,
    amount: f32,
}

impl Default for Diffuser {
    fn default() -> Self {
        Self { engine: FxEngine::new(), amount: 0.0 }
    }
}

impl Diffuser {
    const LINES: [Line; 8] = layout([126, 180, 269, 444, 151, 205, 245, 405]);

    pub fn init(&mut self) {
        self.engine.clear();
    }

    pub fn set_amount(&mut self, amount: f32) {
        self.amount = amount;
    }

    pub fn process(&mut self, in_out: &mut [[f32; 2]]) {
        let [apl1, apl2, apl3, apl4, apr1, apr2, apr3, apr4] = Self::LINES;
        const KAP: f32 = 0.625;
        let amount = self.amount;
        for frame in in_out.iter_mut() {
            let mut c = self.engine.start();
            let mut wet = 0.0;
            c.read_value(frame[0], 1.0);
            for ap in [apl1, apl2, apl3, apl4] {
                c.read(ap, TAIL, KAP);
                c.write_all_pass(ap, -KAP);
            }
            c.write_value(&mut wet, 0.0);
            frame[0] += amount * (wet - frame[0]);

            c.read_value(frame[1], 1.0);
            for ap in [apr1, apr2, apr3, apr4] {
                c.read(ap, TAIL, KAP);
                c.write_all_pass(ap, -KAP);
            }
            c.write_value(&mut wet, 0.0);
            frame[1] += amount * (wet - frame[1]);
        }
    }
}

/// Two crossfaded delay taps sweeping through a delay line.
pub struct PitchShifter {
    engine: FxEngine<Fixed16, 4096>,
    phase: f32,
    ratio: f32,
    size: f32,
}

impl Default for PitchShifter {
    fn default() -> Self {
        Self { engine: FxEngine::new(), phase: 0.0, ratio: 1.0, size: 2047.0 }
    }
}

impl PitchShifter {
    const LINES: [Line; 2] = layout([2047, 2047]);

    pub fn init(&mut self) {
        self.engine.clear();
        self.phase = 0.0;
        self.size = 2047.0;
    }

    pub fn clear(&mut self) {
        self.engine.clear();
    }

    pub fn set_ratio(&mut self, ratio: f32) {
        self.ratio = ratio;
    }

    pub fn set_size(&mut self, size: f32) {
        let target_size = 128.0 + (2047.0 - 128.0) * size * size * size;
        one_pole(&mut self.size, target_size, 0.05);
    }

    pub fn process(&mut self, in_out: &mut [[f32; 2]]) {
        let [left, right] = Self::LINES;
        for frame in in_out.iter_mut() {
            let mut c = self.engine.start();
            self.phase += (1.0 - self.ratio) / self.size;
            if self.phase >= 1.0 {
                self.phase -= 1.0;
            }
            if self.phase <= 0.0 {
                self.phase += 1.0;
            }
            let tri = 2.0 * if self.phase >= 0.5 { 1.0 - self.phase } else { self.phase };
            let phase = self.phase * self.size;
            let mut half = phase + self.size * 0.5;
            if half >= self.size {
                half -= self.size;
            }

            c.read_value(frame[0], 1.0);
            c.write(left, 0, 0.0);
            c.interpolate(left, phase, tri);
            c.interpolate(left, half, 1.0 - tri);
            c.write_value(&mut frame[0], 0.0);

            c.read_value(frame[1], 1.0);
            c.write(right, 0, 0.0);
            c.interpolate(right, phase, tri);
            c.interpolate(right, half, 1.0 - tri);
            c.write_value(&mut frame[1], 0.0);
        }
    }
}

/// The Griesinger-style plate reverb of Clouds, with 12-bit memory.
pub struct Reverb {
    engine: FxEngine<Fixed12, 16384>,
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
        let mut r = Self {
            engine: FxEngine::new(),
            amount: 0.0,
            input_gain: 0.0,
            reverb_time: 0.0,
            diffusion: 0.625,
            lp: 0.7,
            lp_decay_1: 0.0,
            lp_decay_2: 0.0,
        };
        r.init();
        r
    }
}

impl Reverb {
    const LINES: [Line; 10] = layout([113, 162, 241, 399, 1653, 2038, 3411, 1913, 1663, 4782]);

    pub fn init(&mut self) {
        self.engine.clear();
        self.engine.set_lfo_frequency(0, 0.5 / 32000.0);
        self.engine.set_lfo_frequency(1, 0.3 / 32000.0);
        self.lp = 0.7;
        self.diffusion = 0.625;
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

    pub fn process(&mut self, in_out: &mut [[f32; 2]]) {
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
        for frame in in_out.iter_mut() {
            let mut wet = 0.0;
            let mut apout = 0.0;
            let mut c = self.engine.start();

            // Smear AP1 inside the loop.
            c.interpolate_lfo(ap1, 10.0, 0, 60.0, 1.0);
            c.write(ap1, 100, 0.0);

            c.read_value(frame[0] + frame[1], gain);

            // Diffuse through 4 allpasses.
            for ap in [ap1, ap2, ap3, ap4] {
                c.read(ap, TAIL, kap);
                c.write_all_pass(ap, -kap);
            }
            c.write_value(&mut apout, 1.0);

            // Main reverb loop.
            c.load(apout);
            c.interpolate_lfo(del2, 4680.0, 1, 100.0, krt);
            c.lp(&mut lp_1, klp);
            c.read(dap1a, TAIL, -kap);
            c.write_all_pass(dap1a, kap);
            c.read(dap1b, TAIL, kap);
            c.write_all_pass(dap1b, -kap);
            c.write(del1, 0, 2.0);
            c.write_value(&mut wet, 0.0);

            frame[0] += (wet - frame[0]) * amount;

            c.load(apout);
            c.read(del1, TAIL, krt);
            c.lp(&mut lp_2, klp);
            c.read(dap2a, TAIL, kap);
            c.write_all_pass(dap2a, -kap);
            c.read(dap2b, TAIL, -kap);
            c.write_all_pass(dap2b, kap);
            c.write(del2, 0, 2.0);
            c.write_value(&mut wet, 0.0);

            frame[1] += (wet - frame[1]) * amount;
        }
        self.lp_decay_1 = lp_1;
        self.lp_decay_2 = lp_2;
    }
}

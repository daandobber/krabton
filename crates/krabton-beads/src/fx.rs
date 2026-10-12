//! The reverb and the tape's wow and flutter (`beads/dsp/fx/fx_engine.h`,
//! `reverb.h`, `wow_and_flutter.h`). The reverb is the Griesinger topology
//! of the Dattorro paper, four all-passes on the input then a loop of two
//! all-pass pairs and delays, in a 16-bit memory; its first diffuser and
//! long delays are modulated.
//!
//! Copyright 2017 Emilie Gillet, MIT license. Rust port for Krabton.

use crate::util::{clip16, integral_fractional, one_pole, CosineOscillator, ParameterInterpolator, Random, SineOscillator};

const SIZE: usize = 32768;
const MASK: i32 = SIZE as i32 - 1;

/// (base, length) of each line in the memory.
#[derive(Clone, Copy)]
struct Line {
    base: i32,
    length: i32,
}

const fn lines() -> [Line; 10] {
    let lengths = [150, 214, 319, 527, 2182, 2690, 4501, 2525, 2197, 6312];
    let mut out = [Line { base: 0, length: 0 }; 10];
    let mut base = 0;
    let mut i = 0;
    while i < 10 {
        out[i] = Line { base, length: lengths[i] };
        base += lengths[i] + 1;
        i += 1;
    }
    out
}

const LINES: [Line; 10] = lines();
const AP1: usize = 0;
const AP2: usize = 1;
const AP3: usize = 2;
const AP4: usize = 3;
const DAP1A: usize = 4;
const DAP1B: usize = 5;
const DEL1: usize = 6;
const DAP2A: usize = 7;
const DAP2B: usize = 8;
const DEL2: usize = 9;
/// "TAIL": the far end of a line.
const TAIL: i32 = -1;

/// One pass through the memory: an accumulator and the last value read.
struct Context<'a> {
    acc: f32,
    previous_read: f32,
    lfo: [f32; 2],
    buffer: &'a mut [i16],
    write_ptr: i32,
}

impl Context<'_> {
    #[inline]
    fn at(&self, line: usize, offset: i32) -> usize {
        let l = LINES[line];
        let o = if offset == TAIL { l.length - 1 } else { offset };
        ((self.write_ptr + l.base + o) & MASK) as usize
    }

    #[inline]
    fn load(&mut self, v: f32) {
        self.acc = v;
    }

    #[inline]
    fn read_value(&mut self, v: f32, scale: f32) {
        self.acc += v * scale;
    }

    #[inline]
    fn read(&mut self, line: usize, offset: i32, scale: f32) {
        let r = self.buffer[self.at(line, offset)] as f32 / 32768.0;
        self.previous_read = r;
        self.acc += r * scale;
    }

    #[inline]
    fn write(&mut self, line: usize, offset: i32, scale: f32) {
        let i = self.at(line, offset);
        self.buffer[i] = clip16((self.acc * 32768.0) as i32) as i16;
        self.acc *= scale;
    }

    #[inline]
    fn write_all_pass(&mut self, line: usize, offset: i32, scale: f32) {
        self.write(line, offset, scale);
        self.acc += self.previous_read;
    }

    #[inline]
    fn lp(&mut self, state: &mut f32, coefficient: f32) {
        *state += coefficient * (self.acc - *state);
        self.acc = *state;
    }

    #[inline]
    fn interpolate(&mut self, line: usize, offset: f32, lfo: usize, amplitude: f32, scale: f32) {
        let offset = offset + amplitude * self.lfo[lfo];
        let (oi, of) = integral_fractional(offset);
        let base = LINES[line].base;
        let a = self.buffer[((self.write_ptr + oi + base) & MASK) as usize] as f32 / 32768.0;
        let b = self.buffer[((self.write_ptr + oi + base + 1) & MASK) as usize] as f32 / 32768.0;
        let x = a + (b - a) * of;
        self.previous_read = x;
        self.acc += x * scale;
    }
}

pub struct Reverb {
    buffer: Vec<i16>,
    write_ptr: i32,
    lfo: [CosineOscillator; 2],
    mix: f32,
    wet: f32,
    dry: f32,
    send: f32,
    reverb_time: f32,
    lp: f32,
    lp_1: f32,
    lp_2: f32,
}

impl Reverb {
    pub fn new(sample_rate: f32) -> Self {
        let mut r = Self { buffer: vec![0; SIZE], write_ptr: 0, lfo: [CosineOscillator::default(); 2], mix: 0.0, wet: 0.0, dry: 0.0, send: 0.0, reverb_time: 0.5, lp: 0.7, lp_1: 0.0, lp_2: 0.0 };
        r.reset(sample_rate);
        r
    }

    pub fn reset(&mut self, sample_rate: f32) {
        self.lfo[0].init_approximate(0.5 / sample_rate * 32.0);
        self.lfo[1].init_approximate(0.3 / sample_rate * 32.0);
        self.lp = 0.7;
        self.lp_1 = 0.0;
        self.lp_2 = 0.0;
        self.mix = 0.0;
        self.dry = 0.0;
        self.wet = 0.0;
        self.send = 0.0;
    }

    pub fn clear(&mut self) {
        self.buffer.fill(0);
        self.write_ptr = 0;
    }

    pub fn set_mix(&mut self, mix: f32) {
        self.mix = mix;
    }

    pub fn set_time(&mut self, time: f32) {
        self.reverb_time = time;
    }

    pub fn set_lp(&mut self, lp: f32) {
        self.lp = lp;
    }

    pub fn process(&mut self, frames: &mut [[f32; 2]]) {
        let size = frames.len();
        let kap = 0.625f32;
        let klp = self.lp;
        let krt = self.reverb_time;
        let input_gain = 0.2 * (1.0 - 0.333 * self.reverb_time);
        let output_gain = 0.4 / input_gain;
        let send_sqrt = self.mix * (2.0 - self.mix);
        let send_amount = send_sqrt * (2.0 - send_sqrt);
        let dry = (1.0 - self.mix) * (1.0 + self.mix);
        let (mut wet_s, mut dry_s, mut send_s) = (self.wet, self.dry, self.send);
        let (mut lp_1, mut lp_2) = (self.lp_1, self.lp_2);
        {
            let mut wet_m = ParameterInterpolator::new(&mut wet_s, output_gain, size);
            let mut dry_m = ParameterInterpolator::new(&mut dry_s, dry, size);
            let mut send_m = ParameterInterpolator::new(&mut send_s, send_amount * input_gain, size);
            for f in frames.iter_mut() {
                let dry_amount = dry_m.next();
                let wet_amount = wet_m.next();
                let send = send_m.next();
                // Start: one step on, the LFOs every 32 samples.
                self.write_ptr -= 1;
                if self.write_ptr < 0 {
                    self.write_ptr += SIZE as i32;
                }
                let lfo = if self.write_ptr & 31 == 0 { [self.lfo[0].next(), self.lfo[1].next()] } else { [self.lfo[0].value(), self.lfo[1].value()] };
                let mut c = Context { acc: 0.0, previous_read: 0.0, lfo, buffer: &mut self.buffer, write_ptr: self.write_ptr };
                // Smear AP1 inside the loop.
                c.interpolate(AP1, 10.0, 0, 80.0, 1.0);
                c.write(AP1, 100, 0.0);
                c.read_value(f[0] + f[1], send);
                // Diffuse through four all-passes.
                for ap in [AP1, AP2, AP3, AP4] {
                    c.read(ap, TAIL, kap);
                    c.write_all_pass(ap, 0, -kap);
                }
                let apout = c.acc;
                // The loop, left.
                c.load(apout);
                c.interpolate(DEL2, 6261.0, 1, 50.0, krt);
                c.lp(&mut lp_1, klp);
                c.read(DAP1A, TAIL, -kap);
                c.write_all_pass(DAP1A, 0, kap);
                c.read(DAP1B, TAIL, kap);
                c.write_all_pass(DAP1B, 0, -kap);
                c.write(DEL1, 0, 1.0);
                let wet = c.acc;
                f[0] = f[0] * dry_amount + wet * wet_amount;
                // And right.
                c.load(apout);
                c.interpolate(DEL1, 4460.0, 0, 40.0, krt);
                c.lp(&mut lp_2, klp);
                c.read(DAP2A, TAIL, kap);
                c.write_all_pass(DAP2A, 0, -kap);
                c.read(DAP2B, TAIL, -kap);
                c.write_all_pass(DAP2B, 0, kap);
                c.write(DEL2, 0, 1.0);
                let wet = c.acc;
                f[1] = f[1] * dry_amount + wet * wet_amount;
            }
        }
        self.wet = wet_s;
        self.dry = dry_s;
        self.send = send_s;
        self.lp_1 = lp_1;
        self.lp_2 = lp_2;
    }
}

/// The wobble of a worn cassette: a slow wow, a quicker flutter, a random
/// drift of both, through a short modulated delay and a low-pass.
pub struct WowAndFlutter {
    one_hertz: f32,
    delay: f32,
    amount: f32,
    lp: f32,
    random_frequency: f32,
    lp_state: [f32; 2],
    random_modulation: SineOscillator,
    wow: SineOscillator,
    flutter: SineOscillator,
    line: [[f32; 128]; 2],
    write_ptr: usize,
}

impl WowAndFlutter {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            one_hertz: 1.0 / sample_rate,
            delay: 0.0,
            amount: 1.0,
            lp: 1.0,
            random_frequency: 0.125 / sample_rate,
            lp_state: [0.0; 2],
            random_modulation: SineOscillator::default(),
            wow: SineOscillator::default(),
            flutter: SineOscillator::default(),
            line: [[0.0; 128]; 2],
            write_ptr: 0,
        }
    }

    pub fn set_lp(&mut self, lp: f32) {
        self.lp = lp;
    }

    pub fn process(&mut self, frames: &mut [[f32; 2]], rng: &mut Random) {
        let size = frames.len();
        let previous_phase = self.random_modulation.phase();
        let random_modulation = self.random_modulation.render_at(self.random_frequency * size as f32);
        if self.random_modulation.phase() >= 0.25 && previous_phase < 0.25 {
            self.random_frequency = self.one_hertz * (0.05 + rng.get_float() * 0.2);
        }
        let fm = 0.5 + 1.5 * random_modulation * random_modulation;
        let amount = (1.0 - 0.4 * random_modulation) * self.amount;
        let wow = self.wow.render_at(self.one_hertz * 0.23 * size as f32 * fm);
        let flutter = self.flutter.render_at(self.one_hertz * 1.99 * size as f32);
        let delay = 1.0 + (105.0 * wow + 17.0 * flutter) * amount;
        let mut state = self.delay;
        {
            let mut delay_m = ParameterInterpolator::new(&mut state, delay, size);
            for f in frames.iter_mut() {
                let d = delay_m.next();
                // stmlib's DelayLine: written backwards, read linearly.
                self.write_ptr = (self.write_ptr + 127) % 128;
                self.line[0][self.write_ptr] = f[0];
                self.line[1][self.write_ptr] = f[1];
                let (di, df) = integral_fractional(d);
                for c in 0..2 {
                    // stmlib reads at its write pointer (one below ours) plus d.
                    let i = self.write_ptr + di.max(0) as usize + 127;
                    let a = self.line[c][i % 128];
                    let b = self.line[c][(i + 1) % 128];
                    one_pole(&mut self.lp_state[c], a + (b - a) * df, self.lp);
                }
                *f = self.lp_state;
            }
        }
        self.delay = state;
    }
}

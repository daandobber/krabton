//! The little rack reverbs of the late eighties: an emulation of their DSP
//! programs, 228 of them over three models, after midiverb_emulator by thement
//! (<https://github.com/thement/midiverb_emulator>, GPL-3.0). The machine is a
//! fixed-point DSP at about 23.4 kHz with 16 k of 16-bit delay memory that
//! hears one channel and answers two; it has no parameters, only programs.

#![allow(unused_assignments, unused_mut, unused_variables, clippy::all)]

mod programs;

use programs::{MODEL_1, MODEL_2, MODEL_FX};

/// The rate the machine runs at (Hz).
pub const RATE: f64 = 23_400.0;
/// What full scale is in the machine's numbers (13 bits).
pub const FULL: f32 = 4095.0;
const MEMORY: usize = 0x4000;
/// The LFOs step once every so many samples.
const LFO_EVERY: u32 = 8;

pub type Dram = [i16; MEMORY];

/// One program: its name and its code.
pub struct Program {
    pub name: &'static str,
    pub run: fn(i32, &mut Dram, i32, i32, i32) -> (i16, i16),
}

/// A model: its programs, and the number its display starts from.
pub struct Model {
    pub name: &'static str,
    pub programs: &'static [Program],
    pub first: usize,
}

pub static MODELS: [Model; 3] = [
    Model { name: "Rack I", programs: &MODEL_1, first: 1 },
    Model { name: "Rack Fx", programs: &MODEL_FX, first: 1 },
    Model { name: "Rack II", programs: &MODEL_2, first: 0 },
];

#[inline(always)]
fn rd(d: &Dram, at: i32) -> i32 {
    d[(at & 0x3fff) as usize] as i32
}

#[inline(always)]
fn wr(d: &mut Dram, at: i32, v: i32) {
    d[(at & 0x3fff) as usize] = v as i16;
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Shape {
    Triangle,
    Sine,
}

/// The LFOs of Rack II's programs 50 to 69 (flanges, choruses and such).
struct Patch {
    program: usize,
    shape: Shape,
    max: i32,
    top: (u32, u32),
    rate: (i32, i32),
}

const TRI_FULL: (Shape, i32) = (Shape::Triangle, 0xffff);
const TRI_HALF: (Shape, i32) = (Shape::Triangle, 0x7fff);
const TRI_QUARTER: (Shape, i32) = (Shape::Triangle, 0x3fff);
const SIN_FULL: (Shape, i32) = (Shape::Sine, 0x7eff);
const SIN_HALF: (Shape, i32) = (Shape::Sine, 0x3eff);
const SIN_QUARTER: (Shape, i32) = (Shape::Sine, 0x1eff);

const fn patch(program: usize, kind: (Shape, i32), top: (u32, u32), rate: (i32, i32)) -> Patch {
    Patch { program, shape: kind.0, max: kind.1, top, rate }
}

static PATCHES: [Patch; 20] = [
    patch(50, TRI_QUARTER, (0, 0), (0x03, 0x03)),
    patch(51, TRI_QUARTER, (0, 0), (0x03, 0x05)),
    patch(52, TRI_QUARTER, (0, 0), (0x05, 0x05)),
    patch(53, TRI_HALF, (0, 0), (0x06, 0x07)),
    patch(54, TRI_HALF, (0, 0), (0x05, 0x05)),
    patch(55, TRI_HALF, (0, 0), (0x0c, 0x0c)),
    patch(56, TRI_HALF, (0, 0), (0x11, 0x11)),
    patch(57, TRI_FULL, (0, 0), (0x13, 0x17)),
    patch(58, TRI_FULL, (0, 0), (0x0d, 0x0e)),
    patch(59, TRI_FULL, (0, 0), (0x17, 0x11)),
    patch(60, SIN_FULL, (1, 3), (0x27, 0x1b)),
    patch(61, SIN_HALF, (2, 3), (0x2e, 0x3d)),
    patch(62, SIN_FULL, (2, 1), (0x2d, 0x3a)),
    patch(63, SIN_FULL, (3, 2), (0x40, 0x45)),
    patch(64, SIN_HALF, (2, 3), (0x4f, 0x79)),
    patch(65, SIN_HALF, (2, 1), (0x94, 0x7a)),
    patch(66, SIN_FULL, (2, 1), (0x4f, 0x41)),
    patch(67, SIN_HALF, (1, 2), (0x70, 0x8e)),
    // The quarter-range sines run at four times the rate.
    patch(68, SIN_QUARTER, (1, 1), (0xea * 4, 0xd9 * 4)),
    patch(69, SIN_QUARTER, (1, 1), (0xff * 4, 0xff * 4)),
];

#[derive(Clone, Copy)]
struct Lfo {
    shape: Shape,
    rate: i32,
    max: i32,
    value: i32,
    up: bool,
    x: i32,
    y: i32,
}

impl Lfo {
    fn new(shape: Shape, rate: i32, max: i32) -> Self {
        Self { shape, rate, max, value: 0, up: true, x: 0, y: max & 0xff00 }
    }

    fn step(&mut self) -> u32 {
        match self.shape {
            Shape::Triangle => {
                if self.up {
                    self.value += self.rate;
                    if self.value > self.max {
                        self.value = self.max;
                        self.up = false;
                    }
                } else {
                    self.value -= self.rate;
                    if self.value < 0 {
                        self.value = 0;
                        self.up = true;
                    }
                }
                self.value as u32
            }
            Shape::Sine => {
                let limit = self.max;
                self.y = (self.y + ((self.x * self.rate) >> 16)).clamp(-limit, limit);
                self.x = (self.x - ((self.y * self.rate) >> 16)).clamp(-limit, limit);
                ((self.x as u16) ^ 0x8000) as u32
            }
        }
    }
}

/// The machine.
pub struct Machine {
    dram: Box<Dram>,
    ptr: i32,
    model: usize,
    program: usize,
    lfos: Option<([Lfo; 2], (u32, u32))>,
    lfo_values: (i32, i32),
    lfo_count: u32,
}

impl Default for Machine {
    fn default() -> Self {
        Self::new()
    }
}

impl Machine {
    pub fn new() -> Self {
        let mut m = Self { dram: Box::new([0; MEMORY]), ptr: 0, model: 2, program: 1, lfos: None, lfo_values: (0, 0), lfo_count: 0 };
        m.load();
        m
    }

    /// A program of a model (out of range: the nearest). A new one starts from
    /// silence, as the machine does. Nothing is allocated.
    pub fn select(&mut self, model: usize, program: usize) {
        let model = model.min(MODELS.len() - 1);
        let program = program.min(MODELS[model].programs.len() - 1);
        if (model, program) != (self.model, self.program) {
            (self.model, self.program) = (model, program);
            self.load();
        }
    }

    fn load(&mut self) {
        self.dram.fill(0);
        self.ptr = 0;
        self.lfo_count = 0;
        self.lfos = None;
        self.lfo_values = (0, 0);
        if self.model == 2 {
            if let Some(p) = PATCHES.iter().find(|p| p.program == self.program) {
                self.lfos = Some(([Lfo::new(p.shape, p.rate.0, p.max), Lfo::new(p.shape, p.rate.1, p.max)], p.top));
                self.step_lfos();
            }
        }
    }

    fn step_lfos(&mut self) {
        if let Some((lfos, top)) = &mut self.lfos {
            self.lfo_values = ((lfos[0].step() | (top.0 << 16)) as i32, (lfos[1].step() | (top.1 << 16)) as i32);
        }
    }

    /// One sample of the machine: its input (in its own numbers), left and right.
    #[inline]
    pub fn tick(&mut self, input: i16) -> (i16, i16) {
        let run = MODELS[self.model].programs[self.program].run;
        let out = run(input as i32, &mut self.dram, self.ptr, self.lfo_values.0, self.lfo_values.1);
        self.ptr = (self.ptr + 1) & 0x3fff;
        if self.lfos.is_some() {
            self.lfo_count += 1;
            if self.lfo_count >= LFO_EVERY {
                self.lfo_count = 0;
                self.step_lfos();
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn energy(v: &[i16]) -> f64 {
        v.iter().map(|&x| (x as f64).powi(2)).sum::<f64>()
    }

    #[test]
    fn the_models_hold_all_their_programs_with_their_names() {
        assert_eq!(MODELS[0].programs.len(), 64);
        assert_eq!(MODELS[1].programs.len(), 64);
        assert_eq!(MODELS[2].programs.len(), 100);
        assert_eq!(MODELS[0].programs[0].name, ".2 Sec SMALL BRIGHT");
        assert_eq!(MODELS[2].programs[0].name, "Defeat");
        assert_eq!(MODELS[2].programs[15].name, "Medium Bright 1.1 Sec");
        assert_eq!(MODELS[2].programs[50].name, "Triggered Flange");
    }

    #[test]
    fn every_program_runs_loud_and_stays_in_its_numbers() {
        for (mi, model) in MODELS.iter().enumerate() {
            for pi in 0..model.programs.len() {
                let mut m = Machine::new();
                m.select(mi, pi);
                // Loud as it goes: a noise, then silence.
                let mut seed = 12345u32;
                for i in 0..12000 {
                    seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                    let x = if i < 6000 { ((seed >> 16) as i32 - 32768) as i32 / 8 } else { 0 };
                    m.tick(x as i16);
                }
            }
        }
    }

    #[test]
    fn a_reverb_rings_on_after_the_sound_and_defeat_is_silent() {
        let ring = |model: usize, program: usize| {
            let mut m = Machine::new();
            m.select(model, program);
            let mut tail = Vec::new();
            for i in 0..23400 {
                let x = if i < 200 { 3000 } else { 0 };
                let (l, r) = m.tick(x);
                if i > 2000 {
                    tail.push(l);
                    tail.push(r);
                }
            }
            energy(&tail)
        };
        assert!(ring(2, 15) > 1e5, "Medium Bright 1.1 Sec: {}", ring(2, 15));
        assert!(ring(2, 6) > ring(2, 1), "a long one rings on longer than a short one");
        assert_eq!(ring(2, 0), 0.0, "Defeat");
    }

    #[test]
    fn its_two_sides_are_not_the_same() {
        let mut m = Machine::new();
        m.select(2, 15);
        let mut differs = 0;
        for i in 0..23400 {
            let (l, r) = m.tick(if i < 400 { 3000 } else { 0 });
            if l != r {
                differs += 1;
            }
        }
        assert!(differs > 5000, "{differs}");
    }

    #[test]
    fn a_modulated_program_moves_and_a_plain_one_does_not() {
        let mut m = Machine::new();
        m.select(2, 60);
        let first = m.lfo_values;
        let mut moved = false;
        for _ in 0..10_000 {
            m.tick(1000);
            moved |= m.lfo_values != first;
        }
        assert!(moved);
        m.select(2, 15);
        assert_eq!(m.lfo_values, (0, 0));
        assert!(m.lfos.is_none());
    }

    #[test]
    fn the_triangle_turns_at_its_ends_and_the_sine_stays_within_them() {
        let mut t = Lfo::new(Shape::Triangle, 0x1000, 0x3fff);
        let vals: Vec<u32> = (0..12).map(|_| t.step()).collect();
        assert_eq!(*vals.iter().max().unwrap(), 0x3fff);
        assert!(vals[4] < 0x3fff && vals[7] == 0, "{vals:?}");
        let mut s = Lfo::new(Shape::Sine, 0x40, 0x3eff);
        let mut seen = (u32::MAX, 0u32);
        for _ in 0..200_000 {
            let v = s.step();
            seen = (seen.0.min(v), seen.1.max(v));
        }
        assert!(seen.0 < 0x8000 && seen.1 > 0x8000 && seen.1 <= 0xffff, "{seen:?}");
    }

    #[test]
    fn choosing_a_program_clears_the_memory_and_a_bad_number_is_the_nearest() {
        let mut m = Machine::new();
        for _ in 0..4000 {
            m.tick(2000);
        }
        m.select(0, 3);
        assert!(m.dram.iter().all(|&x| x == 0));
        m.select(0, 9999);
        assert_eq!(m.program, 63);
        m.select(99, 0);
        assert_eq!(m.model, 2);
    }
}

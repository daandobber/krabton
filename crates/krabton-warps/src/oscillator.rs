//! Warps' internal oscillator: a sine, band-limited triangle, saw and
//! pulse (polyBLEP), and filtered noise (`warps/dsp/oscillator.cc`).
//!
//! Copyright 2014 Emilie Gillet, MIT license (see the crate's `lib.rs`).

use crate::dsp::{Interp, Random, Svf};
use crate::resources::{LUT_MIDI_TO_F_HIGH, LUT_MIDI_TO_F_LOW, LUT_SIN};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Shape {
    Sine,
    Triangle,
    Saw,
    Pulse,
    Noise,
}

impl Shape {
    pub fn from_index(i: i32) -> Self {
        match i {
            0 => Shape::Sine,
            1 => Shape::Triangle,
            2 => Shape::Saw,
            3 => Shape::Pulse,
            _ => Shape::Noise,
        }
    }
}

const TO_FLOAT: f32 = 1.0 / 4_294_967_296.0;
const TO_U32: f32 = 4_294_967_296.0;

pub struct Oscillator {
    high: bool,
    phase: f32,
    phase_increment: f32,
    next_sample: f32,
    lp_state: f32,
    hp_state: f32,
    external_input_level: f32,
    filter: Svf,
    rng: Random,
}

/// The phase increment of a MIDI pitch at 96 kHz (the module's tables).
pub fn midi_to_increment(midi_pitch: f32) -> f32 {
    let pitch = (midi_pitch * 256.0) as i32;
    let pitch = (32768 + (pitch - 20480).clamp(-32768, 32767)) as usize;
    LUT_MIDI_TO_F_HIGH[(pitch >> 8).min(255)] * LUT_MIDI_TO_F_LOW[pitch & 0xff]
}

fn this_blep(t: f32) -> f32 {
    0.5 * t * t
}

fn next_blep(t: f32) -> f32 {
    let t = 1.0 - t;
    -0.5 * t * t
}

impl Oscillator {
    pub fn new() -> Self {
        Self { high: false, phase: 0.0, phase_increment: 100.0 / 96000.0, next_sample: 0.0, lp_state: 0.0, hp_state: 0.0, external_input_level: 0.0, filter: Svf::default(), rng: Random::new() }
    }

    /// Renders `out` (the same length as `modulation`), returning the gain it wants applied.
    pub fn render(&mut self, shape: Shape, note: f32, modulation: &[f32], out: &mut [f32]) -> f32 {
        match shape {
            Shape::Sine => self.render_sine(note, modulation, out),
            Shape::Noise => self.render_noise(note, modulation, out),
            s => self.render_polyblep(s, note, modulation, out),
        }
    }

    fn render_sine(&mut self, note: f32, modulation: &[f32], out: &mut [f32]) -> f32 {
        let mut phase = self.phase;
        let mut inc = Interp::new(&mut self.phase_increment, midi_to_increment(note), out.len());
        for (o, &m) in out.iter_mut().zip(modulation.iter()) {
            phase += inc.next();
            if phase >= 1.0 {
                phase -= 1.0;
            }
            let mut modulated = (phase * TO_U32) as u32;
            modulated = modulated.wrapping_add((m * 0.5 * TO_U32) as i32 as u32);
            let integral = (modulated >> 22) as usize;
            let fractional = (modulated << 10) as f32 * TO_FLOAT;
            let a = LUT_SIN[integral];
            let b = LUT_SIN[integral + 1];
            *o = a + (b - a) * fractional;
        }
        self.phase = phase;
        1.0
    }

    fn render_polyblep(&mut self, shape: Shape, note: f32, modulation: &[f32], out: &mut [f32]) -> f32 {
        let mut phase = self.phase;
        let mut inc = Interp::new(&mut self.phase_increment, midi_to_increment(note), out.len());
        let mut next_sample = self.next_sample;
        let mut high = self.high;
        let mut lp_state = self.lp_state;
        let mut hp_state = self.hp_state;
        for (o, &m) in out.iter_mut().zip(modulation.iter()) {
            let mut this_sample = next_sample;
            next_sample = 0.0;
            let mut modulated = inc.next() * (1.0 + m);
            if modulated <= 0.0 {
                modulated = 1.0e-7;
            }
            phase += modulated;
            if shape == Shape::Triangle {
                if !high && phase >= 0.5 {
                    let t = (phase - 0.5) / modulated;
                    this_sample += this_blep(t);
                    next_sample += next_blep(t);
                    high = true;
                }
                if phase >= 1.0 {
                    phase -= 1.0;
                    let t = phase / modulated;
                    this_sample -= this_blep(t);
                    next_sample -= next_blep(t);
                    high = false;
                }
                let integrator = modulated * 0.0625;
                next_sample += if phase < 0.5 { 0.0 } else { 1.0 };
                this_sample = 128.0 * (this_sample - 0.5);
                lp_state += integrator * (this_sample - lp_state);
                *o = lp_state;
            } else {
                if phase >= 1.0 {
                    phase -= 1.0;
                    let t = phase / modulated;
                    this_sample -= this_blep(t);
                    next_sample -= next_blep(t);
                }
                next_sample += phase;
                if shape == Shape::Saw {
                    this_sample = this_sample * 2.0 - 1.0;
                    lp_state += 0.3 * (this_sample - lp_state);
                    *o = lp_state;
                } else {
                    lp_state += 0.25 * ((hp_state - this_sample) - lp_state);
                    *o = 4.0 * lp_state;
                    hp_state = this_sample;
                }
            }
        }
        self.high = high;
        self.phase = phase;
        self.next_sample = next_sample;
        self.lp_state = lp_state;
        self.hp_state = hp_state;
        if shape == Shape::Pulse {
            0.025 / (0.0002 + self.phase_increment)
        } else {
            1.0
        }
    }

    /// The internal signal ducked under an external one when it is present.
    fn duck(&mut self, internal_and_out: &mut [f32], external: &[f32]) {
        let mut level = self.external_input_level;
        for (d, &e) in internal_and_out.iter_mut().zip(external.iter()) {
            let error = e * e - level;
            level += if error > 0.0 { 0.01 } else { 0.0001 } * error;
            let gain = (1.0 - 32.0 * level).max(0.0);
            *d = e + gain * (*d - e);
        }
        self.external_input_level = level;
    }

    fn render_noise(&mut self, note: f32, modulation: &[f32], out: &mut [f32]) -> f32 {
        for o in out.iter_mut() {
            let noise = self.rng.word() as f32 * TO_FLOAT;
            *o = 2.0 * noise - 1.0;
        }
        self.duck(out, modulation);
        self.filter.set_f_q(midi_to_increment(note) * 4.0, 1.0);
        self.filter.low_pass(out);
        1.0
    }
}

impl Default for Oscillator {
    fn default() -> Self {
        Self::new()
    }
}

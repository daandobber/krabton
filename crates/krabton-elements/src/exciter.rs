//! Exciter models (`elements/dsp/exciter.h`, `exciter.cc`): sample players,
//! mallet, plectrum, particles, flow and noise.
//!
//! Copyright 2014 Emilie Gillet, MIT license. Rust port for Krabton.

use crate::dsp::{integral_fractional, random_float, random_word, semitones_to_ratio};
use crate::filter::Svf;
use crate::resources::{
    sample, LUT_APPROX_SVF_G, LUT_APPROX_SVF_GAIN, LUT_APPROX_SVF_H, LUT_APPROX_SVF_R, SMP_BOUNDARIES, SMP_NOISE_SAMPLE, SMP_SAMPLE_DATA,
};
use crate::SAMPLE_RATE;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum ExciterModel {
    GranularSamplePlayer,
    SamplePlayer,
    Mallet,
    Plectrum,
    Particles,
    Flow,
    Noise,
}

impl ExciterModel {
    fn from_index(i: i32) -> Self {
        match i {
            i32::MIN..=0 => Self::GranularSamplePlayer,
            1 => Self::SamplePlayer,
            2 => Self::Mallet,
            3 => Self::Plectrum,
            4 => Self::Particles,
            5 => Self::Flow,
            _ => Self::Noise,
        }
    }
}

pub const FLAG_RISING_EDGE: u8 = 1;
pub const FLAG_FALLING_EDGE: u8 = 2;
pub const FLAG_GATE: u8 = 4;

#[derive(Clone, Debug)]
pub struct Exciter {
    model: ExciterModel,
    parameter: f32,
    timbre: f32,
    lp: Svf,
    damp_state: f32,
    particle_state: f32,
    particle_range: f32,
    damping: f32,
    signature: f32,
    phase: u32,
    delay: u32,
    plectrum_delay: u32,
}

impl Default for Exciter {
    fn default() -> Self {
        Self {
            model: ExciterModel::Mallet,
            parameter: 0.0,
            timbre: 0.99,
            lp: Svf::default(),
            damp_state: 0.0,
            particle_state: 0.5,
            particle_range: 0.0,
            damping: 0.0,
            signature: 0.0,
            phase: 0,
            delay: 0,
            plectrum_delay: 0,
        }
    }
}

#[inline]
fn pulse_amplitude(cutoff: f32) -> f32 {
    LUT_APPROX_SVF_GAIN[(cutoff * 256.0) as usize]
}

impl Exciter {
    pub fn set_signature(&mut self, signature: f32) {
        self.signature = signature;
    }

    pub fn set_model(&mut self, model: ExciterModel) {
        self.model = model;
    }

    pub fn set_parameter(&mut self, parameter: f32) {
        self.parameter = parameter;
    }

    pub fn set_timbre(&mut self, timbre: f32) {
        self.timbre = timbre;
    }

    /// Picks a model between `first` and `last` and its parameter from one
    /// continuous control.
    pub fn set_meta(&mut self, meta: f32, first: ExciterModel, last: ExciterModel) {
        let meta = meta * (last as i32 - first as i32 + 1) as f32;
        let (i, f) = integral_fractional(meta);
        self.model = ExciterModel::from_index(first as i32 + i);
        self.parameter = f;
    }

    pub fn damping(&self) -> f32 {
        self.damping
    }

    pub fn process(&mut self, flags: u8, out: &mut [f32]) {
        self.damping = 0.0;
        match self.model {
            ExciterModel::GranularSamplePlayer => self.process_granular_sample_player(out),
            ExciterModel::SamplePlayer => self.process_sample_player(flags, out),
            ExciterModel::Mallet => self.process_mallet(flags, out),
            ExciterModel::Plectrum => self.process_plectrum(flags, out),
            ExciterModel::Particles => self.process_particles(flags, out),
            ExciterModel::Flow => self.process_flow(flags, out),
            ExciterModel::Noise => self.process_noise(out),
        }
        if self.model != ExciterModel::GranularSamplePlayer && self.model != ExciterModel::SamplePlayer {
            let cutoff_index = (self.timbre * 256.0) as usize;
            if self.model == ExciterModel::Noise {
                let resonance_index = (self.parameter * 256.0) as usize;
                self.lp.set_g_r(LUT_APPROX_SVF_G[cutoff_index], LUT_APPROX_SVF_R[resonance_index]);
            } else {
                self.lp.set_g_r_h(LUT_APPROX_SVF_G[cutoff_index], 2.0, LUT_APPROX_SVF_H[cutoff_index]);
            }
            self.lp.process_lp_in_place(out);
        }
    }

    fn process_granular_sample_player(&mut self, out: &mut [f32]) {
        let restart_prob = (0.01f32 * 4294967296.0) as u32;
        let restart_point = ((self.parameter * 32767.0) as u32) << 17;
        let phase_increment = (131072.0 * semitones_to_ratio(72.0 * self.timbre - 60.0)) as u32;
        let base = (self.signature * 8192.0) as usize;
        let mut phase = self.phase;
        for o in out.iter_mut() {
            let phase_integral = (phase >> 17) as usize;
            let phase_fractional = (phase & 0x1ffff) as f32 / 131072.0;
            let a = sample(&SMP_NOISE_SAMPLE, base + phase_integral);
            let b = sample(&SMP_NOISE_SAMPLE, base + phase_integral + 1);
            *o = (a + (b - a) * phase_fractional) / 32768.0;
            phase = phase.wrapping_add(phase_increment);
            if random_word() < restart_prob {
                phase = restart_point;
            }
        }
        self.phase = phase;
        self.damping = 0.0;
    }

    fn process_sample_player(&mut self, flags: u8, out: &mut [f32]) {
        let index = (1.0 - self.parameter) * 8.0;
        let (mut index_integral, mut index_fractional) = integral_fractional(index);
        if index_integral >= 8 {
            index_integral = 7;
            index_fractional = 1.0;
        }
        let index_integral = index_integral.max(0) as usize;
        let offset_1 = SMP_BOUNDARIES[index_integral];
        let offset_2 = SMP_BOUNDARIES[index_integral + 1];
        let length_1 = (offset_2 - offset_1 - 1) as u32;
        let length_2 = (SMP_BOUNDARIES[index_integral + 2] - offset_2 - 1) as u32;
        let phase_increment = (65536.0 * semitones_to_ratio(72.0 * self.timbre - 36.0 + 7.0)) as u32;

        let mut damp = self.damp_state;
        let mut phase = self.phase;
        if flags & FLAG_RISING_EDGE != 0 {
            damp = 0.0;
            phase = 0;
        }
        if flags & FLAG_GATE == 0 {
            damp = 1.0 - 0.95 * (1.0 - damp);
        }
        for o in out.iter_mut() {
            let phase_integral = phase >> 16;
            let phase_fractional = (phase & 0xffff) as f32 / 65536.0;
            let mut sample_1 = 0.0;
            let mut sample_2 = 0.0;
            let mut step = false;
            if phase_integral < length_1 {
                let i = offset_1 + phase_integral as usize;
                let (a, b) = (sample(&SMP_SAMPLE_DATA, i), sample(&SMP_SAMPLE_DATA, i + 1));
                sample_1 = a + (b - a) * phase_fractional;
                step = true;
            }
            if phase_integral < length_2 {
                let i = offset_2 + phase_integral as usize;
                let (a, b) = (sample(&SMP_SAMPLE_DATA, i), sample(&SMP_SAMPLE_DATA, i + 1));
                sample_2 = a + (b - a) * phase_fractional;
                step = true;
            }
            if step {
                phase = phase.wrapping_add(phase_increment);
            }
            *o = (sample_1 + (sample_2 - sample_1) * index_fractional) / 65536.0;
        }
        self.phase = phase;
        self.damping = damp * if self.parameter >= 0.8 { self.parameter * 5.0 - 4.0 } else { 0.0 };
        self.damp_state = damp;
    }

    fn process_mallet(&mut self, flags: u8, out: &mut [f32]) {
        out.fill(0.0);
        if flags & FLAG_RISING_EDGE != 0 {
            self.damp_state = 0.0;
            out[0] = pulse_amplitude(self.timbre);
        }
        if flags & FLAG_GATE == 0 {
            self.damp_state = 1.0 - 0.95 * (1.0 - self.damp_state);
        }
        self.damping = self.damp_state * (1.0 - self.parameter);
    }

    fn process_plectrum(&mut self, flags: u8, out: &mut [f32]) {
        let amplitude = pulse_amplitude(self.timbre);
        let mut damp = self.damp_state;
        let mut impulse = 0.0;
        if flags & FLAG_RISING_EDGE != 0 {
            impulse = -amplitude * (0.05 + self.signature * 0.2);
            self.plectrum_delay = (4096.0 * self.parameter * self.parameter) as u32 + 64;
        }
        for o in out.iter_mut() {
            if self.plectrum_delay != 0 {
                self.plectrum_delay -= 1;
                if self.plectrum_delay == 0 {
                    impulse = amplitude;
                }
                damp = 1.0 - 0.997 * (1.0 - damp);
            } else {
                damp *= 0.9;
            }
            *o = impulse;
            impulse = 0.0;
        }
        self.damping = damp * 0.5;
        self.damp_state = damp;
    }

    fn process_particles(&mut self, flags: u8, out: &mut [f32]) {
        if flags & FLAG_RISING_EDGE != 0 {
            let r = random_float();
            self.particle_state = 1.0 - 0.6 * r * r;
            self.delay = 0;
            self.particle_range = 1.0;
        }
        out.fill(0.0);
        if flags & FLAG_GATE != 0 {
            let up_probability = (0.7f32 * 4294967296.0) as u32;
            let down_probability = (0.3f32 * 4294967296.0) as u32;
            let amplitude = pulse_amplitude(self.timbre);
            for o in out.iter_mut() {
                if self.delay == 0 {
                    let amount = random_float();
                    let amount = 1.05 + 0.5 * amount * amount;
                    if random_word() > up_probability {
                        self.particle_state *= amount;
                        if self.particle_state >= self.particle_range + 0.25 {
                            self.particle_state = self.particle_range + 0.25;
                        }
                    } else if random_word() < down_probability {
                        self.particle_state /= amount;
                        if self.particle_state <= 0.02 {
                            self.particle_state = 0.02;
                        }
                    }
                    self.delay = (self.particle_state * 0.15 * SAMPLE_RATE) as u32;
                    let gain = 1.0 - self.particle_range;
                    let gain = gain * gain;
                    *o = self.particle_state * amplitude * (1.0 - gain);
                    let decay_factor = 1.0 - self.parameter;
                    self.particle_range *= 1.0 - decay_factor * decay_factor * 0.5;
                } else {
                    self.delay -= 1;
                }
            }
        }
    }

    fn process_flow(&mut self, flags: u8, out: &mut [f32]) {
        let p = self.parameter;
        let scale = p * p * p * p;
        let threshold = 0.0001 + scale * 0.125;
        if flags & FLAG_RISING_EDGE != 0 {
            self.particle_state = 0.5;
        }
        for o in out.iter_mut() {
            let sample = random_float();
            if sample < threshold {
                self.particle_state = -self.particle_state;
            }
            *o = self.particle_state + (sample - 0.5 - self.particle_state) * scale;
        }
    }

    fn process_noise(&mut self, out: &mut [f32]) {
        for o in out.iter_mut() {
            *o = random_float() - 0.5;
        }
    }
}

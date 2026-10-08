//! Multistage envelope (`elements/dsp/multistage_envelope.h`). Elements uses
//! it as an ADSR clocked once per 16-sample block.
//!
//! Copyright 2014 Emilie Gillet, MIT license. Rust port for Krabton.

use crate::resources::{LUT_ENV_EXPO, LUT_ENV_INCREMENTS, LUT_ENV_LINEAR, LUT_ENV_QUARTIC};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum EnvelopeShape {
    #[default]
    Linear,
    Exponential,
    Quartic,
}

pub const FLAG_RISING_EDGE: u8 = 1;
pub const FLAG_FALLING_EDGE: u8 = 2;
pub const FLAG_GATE: u8 = 4;

const MAX_SEGMENTS: usize = 6;

#[derive(Clone, Debug)]
pub struct MultistageEnvelope {
    level: [f32; MAX_SEGMENTS],
    time: [f32; MAX_SEGMENTS],
    shape: [EnvelopeShape; MAX_SEGMENTS],
    segment: usize,
    start_value: f32,
    value: f32,
    phase: f32,
    num_segments: usize,
    sustain_point: usize,
    loop_start: usize,
    loop_end: usize,
    hard_reset: bool,
}

impl Default for MultistageEnvelope {
    fn default() -> Self {
        let mut e = Self {
            level: [0.0; MAX_SEGMENTS],
            time: [0.0; MAX_SEGMENTS],
            shape: [EnvelopeShape::Linear; MAX_SEGMENTS],
            segment: 0,
            start_value: 0.0,
            value: 0.0,
            phase: 0.0,
            num_segments: 0,
            sustain_point: 0,
            loop_start: 0,
            loop_end: 0,
            hard_reset: false,
        };
        e.set_adsr(0.0, 0.25, 0.25, 0.5);
        e.segment = e.num_segments;
        e
    }
}

#[inline]
fn interpolate8(table: &[f32], index: f32) -> f32 {
    let index = index * 256.0;
    let integral = index as usize;
    let fractional = index - integral as f32;
    let a = table[integral];
    let b = table[integral + 1];
    a + (b - a) * fractional
}

impl MultistageEnvelope {
    pub fn process(&mut self, flags: u8) -> f32 {
        if flags & FLAG_RISING_EDGE != 0 {
            self.start_value = if self.segment == self.num_segments || self.hard_reset { self.level[0] } else { self.value };
            self.segment = 0;
            self.phase = 0.0;
        } else if flags & FLAG_FALLING_EDGE != 0 && self.sustain_point != 0 {
            self.start_value = self.value;
            self.segment = self.sustain_point;
            self.phase = 0.0;
        } else if self.phase >= 1.0 {
            self.start_value = self.level[self.segment + 1];
            self.segment += 1;
            self.phase = 0.0;
            if self.segment == self.loop_end {
                self.segment = self.loop_start;
            }
        }
        let done = self.segment == self.num_segments;
        let sustained = self.sustain_point != 0 && self.segment == self.sustain_point && flags & FLAG_GATE != 0;
        let phase_increment = if !sustained && !done { interpolate8(&LUT_ENV_INCREMENTS, self.time[self.segment]) } else { 0.0 };
        let table: &[f32] = match self.shape[self.segment.min(MAX_SEGMENTS - 1)] {
            EnvelopeShape::Linear => &LUT_ENV_LINEAR,
            EnvelopeShape::Exponential => &LUT_ENV_EXPO,
            EnvelopeShape::Quartic => &LUT_ENV_QUARTIC,
        };
        let t = interpolate8(table, self.phase.min(1.0));
        self.phase += phase_increment;
        let target = self.level[(self.segment + 1).min(MAX_SEGMENTS - 1)];
        self.value = self.start_value + (target - self.start_value) * t;
        self.value
    }

    pub fn set_hard_reset(&mut self, hard_reset: bool) {
        self.hard_reset = hard_reset;
    }

    pub fn set_adsr(&mut self, attack: f32, decay: f32, sustain: f32, release: f32) {
        self.num_segments = 3;
        self.sustain_point = 2;
        self.level[0] = 0.0;
        self.level[1] = 1.0;
        self.level[2] = sustain;
        self.level[3] = 0.0;
        self.time[0] = attack.clamp(0.0, 1.0);
        self.time[1] = decay.clamp(0.0, 1.0);
        self.time[2] = release.clamp(0.0, 1.0);
        self.shape[0] = EnvelopeShape::Quartic;
        self.shape[1] = EnvelopeShape::Exponential;
        self.shape[2] = EnvelopeShape::Exponential;
        self.loop_start = 0;
        self.loop_end = 0;
    }

    pub fn set_ad(&mut self, attack: f32, decay: f32) {
        self.num_segments = 2;
        self.sustain_point = 0;
        self.level[0] = 0.0;
        self.level[1] = 1.0;
        self.level[2] = 0.0;
        self.time[0] = attack.clamp(0.0, 1.0);
        self.time[1] = decay.clamp(0.0, 1.0);
        self.shape[0] = EnvelopeShape::Linear;
        self.shape[1] = EnvelopeShape::Exponential;
        self.loop_start = 0;
        self.loop_end = 0;
    }
}

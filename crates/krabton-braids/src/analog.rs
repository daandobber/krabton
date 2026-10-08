//! Band-limited "analog" waveforms with hard sync: saw, variable saw, CS-80
//! style saw, square, triangle, sine, folded triangle/sine and buzz.
//!
//! Port of `braids/analog_oscillator.h/.cc` (Copyright 2012 Emilie Gillet,
//! MIT). Integer arithmetic follows the C++ code exactly.

use crate::dsp::{crossfade, interpolate824, interpolate88, ParamRamp, PhaseIncrementRamp};
use crate::resources::{LUT_OSCILLATOR_INCREMENTS, WAVEFORM_TABLE, WAV_SINE, WS_SINE_FOLD, WS_TRI_FOLD};

const K_NUM_ZONES: usize = 15;
const K_HIGHEST_NOTE: i32 = 128 * 128;
const K_PITCH_TABLE_START: i32 = 128 * 128;
const K_OCTAVE: i32 = 12 * 128;
/// `WAV_BANDLIMITED_COMB_0` in `waveform_table`.
const WAV_BANDLIMITED_COMB_0: usize = 3;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AnalogShape {
    Saw,
    VariableSaw,
    CSaw,
    Square,
    Triangle,
    Sine,
    TriangleFold,
    SineFold,
    Buzz,
}

/// Unsigned division that gives 0 for a zero divisor, like the Cortex-M
/// hardware divider the original runs on (no trap).
#[inline(always)]
fn udiv(a: u32, b: u32) -> u32 {
    if b == 0 {
        0
    } else {
        a / b
    }
}

#[inline(always)]
fn this_blep_sample(t: u32) -> i32 {
    let t = t.min(65535);
    (t * t >> 18) as i32
}

#[inline(always)]
fn next_blep_sample(t: u32) -> i32 {
    let t = 65535 - t.min(65535);
    -((t * t >> 18) as i32)
}

/// Folded triangle helper: `(phase_16 << 1) ^ (phase_16 & 0x8000 ? 0xffff : 0)`
/// stored into an `int16_t`, then `+= 32768`.
#[inline(always)]
fn triangle_from_phase(phase: u32) -> i16 {
    let phase_16 = (phase >> 16) as u16;
    let mask = if phase_16 & 0x8000 != 0 { 0xffff } else { 0 };
    let t = (((phase_16 as i32) << 1) ^ mask) as i16;
    (t as i32 + 32768) as i16
}

pub struct AnalogOscillator {
    phase: u32,
    phase_increment: u32,
    previous_phase_increment: u32,
    high: bool,
    parameter: i16,
    previous_parameter: i16,
    aux_parameter: i16,
    discontinuity_depth: i16,
    pitch: i16,
    next_sample: i32,
    shape: AnalogShape,
    previous_shape: Option<AnalogShape>,
}

impl Default for AnalogOscillator {
    fn default() -> Self {
        Self::new()
    }
}

impl AnalogOscillator {
    pub fn new() -> Self {
        let mut o = Self {
            phase: 0,
            phase_increment: 1,
            previous_phase_increment: 0,
            high: false,
            parameter: 0,
            previous_parameter: 0,
            aux_parameter: 0,
            discontinuity_depth: -16383,
            pitch: 60 << 7,
            next_sample: 0,
            shape: AnalogShape::Saw,
            previous_shape: None,
        };
        o.init();
        o
    }

    pub fn init(&mut self) {
        self.phase = 0;
        self.phase_increment = 1;
        self.high = false;
        self.parameter = 0;
        self.previous_parameter = 0;
        self.aux_parameter = 0;
        self.discontinuity_depth = -16383;
        self.pitch = 60 << 7;
        self.next_sample = 0;
    }

    pub fn set_shape(&mut self, shape: AnalogShape) {
        self.shape = shape;
    }

    pub fn set_pitch(&mut self, pitch: i16) {
        self.pitch = pitch;
    }

    pub fn set_parameter(&mut self, parameter: i16) {
        self.parameter = parameter;
    }

    pub fn set_aux_parameter(&mut self, parameter: i16) {
        self.aux_parameter = parameter;
    }

    pub fn phase_increment(&self) -> u32 {
        self.phase_increment
    }

    pub fn reset(&mut self) {
        self.phase = self.phase_increment.wrapping_neg();
    }

    fn compute_phase_increment(midi_pitch: i16) -> u32 {
        let mut midi_pitch = midi_pitch as i32;
        if midi_pitch >= K_HIGHEST_NOTE {
            midi_pitch = K_HIGHEST_NOTE - 1;
        }
        let mut ref_pitch = midi_pitch - K_PITCH_TABLE_START;
        let mut num_shifts = 0u32;
        while ref_pitch < 0 {
            ref_pitch += K_OCTAVE;
            num_shifts += 1;
        }
        let a = LUT_OSCILLATOR_INCREMENTS[(ref_pitch >> 4) as usize];
        let b = LUT_OSCILLATOR_INCREMENTS[((ref_pitch >> 4) + 1) as usize];
        let phase_increment = a.wrapping_add(((b.wrapping_sub(a) as i32).wrapping_mul(ref_pitch & 0xf) >> 4) as u32);
        phase_increment.checked_shr(num_shifts).unwrap_or(0)
    }

    /// Renders `buffer.len()` samples. `sync_in[i] != 0` marks a sync edge
    /// (its value is the fractional reset time + 1); `sync_out`, when given,
    /// receives this oscillator's own resets in the same format.
    pub fn render(&mut self, sync_in: &[u8], buffer: &mut [i16], sync_out: Option<&mut [u8]>) {
        if self.previous_shape != Some(self.shape) {
            self.init();
            self.previous_shape = Some(self.shape);
        }
        self.phase_increment = Self::compute_phase_increment(self.pitch);
        if self.pitch as i32 > K_HIGHEST_NOTE {
            self.pitch = K_HIGHEST_NOTE as i16;
        } else if self.pitch < 0 {
            self.pitch = 0;
        }
        match self.shape {
            AnalogShape::Saw => self.render_saw(sync_in, buffer, sync_out),
            AnalogShape::VariableSaw => self.render_variable_saw(sync_in, buffer, sync_out),
            AnalogShape::CSaw => self.render_csaw(sync_in, buffer, sync_out),
            AnalogShape::Square => self.render_square(sync_in, buffer, sync_out),
            AnalogShape::Triangle => self.render_triangle(sync_in, buffer),
            AnalogShape::Sine => self.render_sine(sync_in, buffer),
            AnalogShape::TriangleFold => self.render_triangle_fold(sync_in, buffer),
            AnalogShape::SineFold => self.render_sine_fold(sync_in, buffer),
            AnalogShape::Buzz => self.render_buzz(sync_in, buffer),
        }
    }

    #[inline(always)]
    fn sync_at(sync_in: &[u8], i: usize) -> u8 {
        sync_in.get(i).copied().unwrap_or(0)
    }

    #[inline(always)]
    fn write_sync(sync_out: &mut Option<&mut [u8]>, i: usize, phase: u32, phase_increment: u32) {
        if let Some(out) = sync_out.as_deref_mut() {
            if let Some(slot) = out.get_mut(i) {
                *slot = if phase < phase_increment { (udiv(phase, phase_increment >> 7) + 1) as u8 } else { 0 };
            }
        }
    }

    fn render_csaw(&mut self, sync_in: &[u8], buffer: &mut [i16], mut sync_out: Option<&mut [u8]>) {
        let n = buffer.len();
        let mut ramp = PhaseIncrementRamp::new(self.previous_phase_increment, self.phase_increment, n);
        let mut next_sample = self.next_sample;
        for i in 0..n {
            let mut sync_reset = false;
            let mut self_reset = false;
            let mut transition_during_reset = false;
            let mut reset_time: u32 = 0;
            let phase_increment = ramp.next();
            let mut pw = (self.parameter as i32 as u32).wrapping_mul(49152);
            if pw < phase_increment.wrapping_mul(8) {
                pw = phase_increment.wrapping_mul(8);
            }
            let mut this_sample = next_sample;
            next_sample = 0;

            let s = Self::sync_at(sync_in, i);
            if s != 0 {
                reset_time = ((s as i32 - 1) as u32) << 9;
                let phase_at_reset = self.phase.wrapping_add((65535u32.wrapping_sub(reset_time)).wrapping_mul(phase_increment >> 16));
                sync_reset = true;
                transition_during_reset = false;
                if phase_at_reset < self.phase || (!self.high && phase_at_reset >= pw) {
                    transition_during_reset = true;
                }
                if self.phase >= pw {
                    self.discontinuity_depth = (-2048 + (self.aux_parameter as i32 >> 2)) as i16;
                    let before = (phase_at_reset >> 18) as i32;
                    let after = self.discontinuity_depth as i32;
                    let discontinuity = after - before;
                    this_sample += discontinuity * this_blep_sample(reset_time) >> 15;
                    next_sample += discontinuity * next_blep_sample(reset_time) >> 15;
                }
            }

            self.phase = self.phase.wrapping_add(phase_increment);
            if self.phase < phase_increment {
                self_reset = true;
            }
            Self::write_sync(&mut sync_out, i, self.phase, phase_increment);

            while transition_during_reset || !sync_reset {
                if !self.high {
                    if self.phase < pw {
                        break;
                    }
                    let t = udiv(self.phase.wrapping_sub(pw), phase_increment >> 16);
                    let before = self.discontinuity_depth;
                    let after = (self.phase >> 18) as i16;
                    let discontinuity = (after as i32 - before as i32) as i16 as i32;
                    this_sample += discontinuity * this_blep_sample(t) >> 15;
                    next_sample += discontinuity * next_blep_sample(t) >> 15;
                    self.high = true;
                }
                if self.high {
                    if !self_reset {
                        break;
                    }
                    self_reset = false;
                    self.discontinuity_depth = (-2048 + (self.aux_parameter as i32 >> 2)) as i16;
                    let t = udiv(self.phase, phase_increment >> 16);
                    let before: i16 = 16383;
                    let after = self.discontinuity_depth;
                    let discontinuity = (after as i32 - before as i32) as i16 as i32;
                    this_sample += discontinuity * this_blep_sample(t) >> 15;
                    next_sample += discontinuity * next_blep_sample(t) >> 15;
                    self.high = false;
                }
            }

            if sync_reset {
                self.phase = reset_time.wrapping_mul(phase_increment >> 16);
                self.high = false;
            }

            next_sample += if self.phase < pw { self.discontinuity_depth as i32 } else { (self.phase >> 18) as i32 };
            buffer[i] = ((this_sample - 8192) << 1) as i16;
        }
        self.next_sample = next_sample;
        self.previous_phase_increment = ramp.value();
    }

    fn render_square(&mut self, sync_in: &[u8], buffer: &mut [i16], mut sync_out: Option<&mut [u8]>) {
        let n = buffer.len();
        let mut ramp = PhaseIncrementRamp::new(self.previous_phase_increment, self.phase_increment, n);
        if self.parameter > 32000 {
            self.parameter = 32000;
        }
        let mut next_sample = self.next_sample;
        for i in 0..n {
            let mut sync_reset = false;
            let mut self_reset = false;
            let mut transition_during_reset = false;
            let mut reset_time: u32 = 0;
            let phase_increment = ramp.next();
            let pw = ((32768 - self.parameter as i32) as u32) << 16;
            let mut this_sample = next_sample;
            next_sample = 0;

            let s = Self::sync_at(sync_in, i);
            if s != 0 {
                reset_time = ((s as i32 - 1) as u32) << 9;
                let phase_at_reset = self.phase.wrapping_add((65535u32.wrapping_sub(reset_time)).wrapping_mul(phase_increment >> 16));
                sync_reset = true;
                if phase_at_reset < self.phase || (!self.high && phase_at_reset >= pw) {
                    transition_during_reset = true;
                }
                if phase_at_reset >= pw {
                    this_sample -= this_blep_sample(reset_time);
                    next_sample -= next_blep_sample(reset_time);
                }
            }

            self.phase = self.phase.wrapping_add(phase_increment);
            if self.phase < phase_increment {
                self_reset = true;
            }
            Self::write_sync(&mut sync_out, i, self.phase, phase_increment);

            while transition_during_reset || !sync_reset {
                if !self.high {
                    if self.phase < pw {
                        break;
                    }
                    let t = udiv(self.phase.wrapping_sub(pw), phase_increment >> 16);
                    this_sample += this_blep_sample(t);
                    next_sample += next_blep_sample(t);
                    self.high = true;
                }
                if self.high {
                    if !self_reset {
                        break;
                    }
                    self_reset = false;
                    let t = udiv(self.phase, phase_increment >> 16);
                    this_sample -= this_blep_sample(t);
                    next_sample -= next_blep_sample(t);
                    self.high = false;
                }
            }

            if sync_reset {
                self.phase = reset_time.wrapping_mul(phase_increment >> 16);
                self.high = false;
            }

            next_sample += if self.phase < pw { 0 } else { 32767 };
            buffer[i] = ((this_sample - 16384) << 1) as i16;
        }
        self.next_sample = next_sample;
        self.previous_phase_increment = ramp.value();
    }

    fn render_saw(&mut self, sync_in: &[u8], buffer: &mut [i16], mut sync_out: Option<&mut [u8]>) {
        let n = buffer.len();
        let mut ramp = PhaseIncrementRamp::new(self.previous_phase_increment, self.phase_increment, n);
        let mut next_sample = self.next_sample;
        for i in 0..n {
            let mut sync_reset = false;
            let mut self_reset = false;
            let mut transition_during_reset = false;
            let mut reset_time: u32 = 0;
            let phase_increment = ramp.next();
            let mut this_sample = next_sample;
            next_sample = 0;

            let s = Self::sync_at(sync_in, i);
            if s != 0 {
                reset_time = ((s as i32 - 1) as u32) << 9;
                let phase_at_reset = self.phase.wrapping_add((65535u32.wrapping_sub(reset_time)).wrapping_mul(phase_increment >> 16));
                sync_reset = true;
                if phase_at_reset < self.phase {
                    transition_during_reset = true;
                }
                let discontinuity = (phase_at_reset >> 17) as i32;
                this_sample -= discontinuity * this_blep_sample(reset_time) >> 15;
                next_sample -= discontinuity * next_blep_sample(reset_time) >> 15;
            }

            self.phase = self.phase.wrapping_add(phase_increment);
            if self.phase < phase_increment {
                self_reset = true;
            }
            Self::write_sync(&mut sync_out, i, self.phase, phase_increment);

            if (transition_during_reset || !sync_reset) && self_reset {
                let t = udiv(self.phase, phase_increment >> 16);
                this_sample -= this_blep_sample(t);
                next_sample -= next_blep_sample(t);
            }

            if sync_reset {
                self.phase = reset_time.wrapping_mul(phase_increment >> 16);
                self.high = false;
            }

            next_sample += (self.phase >> 17) as i32;
            buffer[i] = ((this_sample - 16384) << 1) as i16;
        }
        self.next_sample = next_sample;
        self.previous_phase_increment = ramp.value();
    }

    fn render_variable_saw(&mut self, sync_in: &[u8], buffer: &mut [i16], mut sync_out: Option<&mut [u8]>) {
        let n = buffer.len();
        let mut ramp = PhaseIncrementRamp::new(self.previous_phase_increment, self.phase_increment, n);
        let mut next_sample = self.next_sample;
        if self.parameter < 1024 {
            self.parameter = 1024;
        }
        for i in 0..n {
            let mut sync_reset = false;
            let mut self_reset = false;
            let mut transition_during_reset = false;
            let mut reset_time: u32 = 0;
            let phase_increment = ramp.next();
            let pw = (self.parameter as i32 as u32) << 16;
            let mut this_sample = next_sample;
            next_sample = 0;

            let s = Self::sync_at(sync_in, i);
            if s != 0 {
                reset_time = ((s as i32 - 1) as u32) << 9;
                let phase_at_reset = self.phase.wrapping_add((65535u32.wrapping_sub(reset_time)).wrapping_mul(phase_increment >> 16));
                sync_reset = true;
                if phase_at_reset < self.phase || (!self.high && phase_at_reset >= pw) {
                    transition_during_reset = true;
                }
                let before = (phase_at_reset >> 18) as i32 + (phase_at_reset.wrapping_sub(pw) >> 18) as i32;
                let after = 0i32 + (0u32.wrapping_sub(pw) >> 18) as i32;
                let discontinuity = after - before;
                this_sample += discontinuity * this_blep_sample(reset_time) >> 15;
                next_sample += discontinuity * next_blep_sample(reset_time) >> 15;
            }

            self.phase = self.phase.wrapping_add(phase_increment);
            if self.phase < phase_increment {
                self_reset = true;
            }
            Self::write_sync(&mut sync_out, i, self.phase, phase_increment);

            while transition_during_reset || !sync_reset {
                if !self.high {
                    if self.phase < pw {
                        break;
                    }
                    let t = udiv(self.phase.wrapping_sub(pw), phase_increment >> 16);
                    this_sample -= this_blep_sample(t) >> 1;
                    next_sample -= next_blep_sample(t) >> 1;
                    self.high = true;
                }
                if self.high {
                    if !self_reset {
                        break;
                    }
                    self_reset = false;
                    let t = udiv(self.phase, phase_increment >> 16);
                    this_sample -= this_blep_sample(t) >> 1;
                    next_sample -= next_blep_sample(t) >> 1;
                    self.high = false;
                }
            }

            if sync_reset {
                self.phase = reset_time.wrapping_mul(phase_increment >> 16);
                self.high = false;
            }

            next_sample += (self.phase >> 18) as i32;
            next_sample += (self.phase.wrapping_sub(pw) >> 18) as i32;
            buffer[i] = ((this_sample - 16384) << 1) as i16;
        }
        self.next_sample = next_sample;
        self.previous_phase_increment = ramp.value();
    }

    fn render_triangle(&mut self, sync_in: &[u8], buffer: &mut [i16]) {
        let n = buffer.len();
        let mut ramp = PhaseIncrementRamp::new(self.previous_phase_increment, self.phase_increment, n);
        let mut phase = self.phase;
        for i in 0..n {
            let phase_increment = ramp.next();
            if Self::sync_at(sync_in, i) != 0 {
                phase = 0;
            }
            phase = phase.wrapping_add(phase_increment >> 1);
            let triangle = triangle_from_phase(phase);
            buffer[i] = triangle >> 1;
            phase = phase.wrapping_add(phase_increment >> 1);
            let triangle = triangle_from_phase(phase);
            buffer[i] = (buffer[i] as i32 + (triangle >> 1) as i32) as i16;
        }
        self.phase = phase;
        self.previous_phase_increment = ramp.value();
    }

    fn render_sine(&mut self, sync_in: &[u8], buffer: &mut [i16]) {
        let n = buffer.len();
        let mut phase = self.phase;
        let mut ramp = PhaseIncrementRamp::new(self.previous_phase_increment, self.phase_increment, n);
        for i in 0..n {
            let phase_increment = ramp.next();
            phase = phase.wrapping_add(phase_increment);
            if Self::sync_at(sync_in, i) != 0 {
                phase = 0;
            }
            buffer[i] = interpolate824(&WAV_SINE, phase);
        }
        self.previous_phase_increment = ramp.value();
        self.phase = phase;
    }

    fn render_triangle_fold(&mut self, sync_in: &[u8], buffer: &mut [i16]) {
        let n = buffer.len();
        let mut phase = self.phase;
        let mut ramp = PhaseIncrementRamp::new(self.previous_phase_increment, self.phase_increment, n);
        let mut param = ParamRamp::new(self.previous_parameter, self.parameter, n);
        for i in 0..n {
            let parameter = param.next();
            let phase_increment = ramp.next();
            let gain = (2048 + (parameter * 30720 >> 15)) as i16 as i32;
            if Self::sync_at(sync_in, i) != 0 {
                phase = 0;
            }
            // 2x oversampled wavefolder.
            phase = phase.wrapping_add(phase_increment >> 1);
            let mut triangle = triangle_from_phase(phase);
            triangle = (triangle as i32 * gain >> 15) as i16;
            triangle = interpolate88(&WS_TRI_FOLD, (triangle as i32 + 32768) as u16);
            buffer[i] = triangle >> 1;
            phase = phase.wrapping_add(phase_increment >> 1);
            let mut triangle = triangle_from_phase(phase);
            triangle = (triangle as i32 * gain >> 15) as i16;
            triangle = interpolate88(&WS_TRI_FOLD, (triangle as i32 + 32768) as u16);
            buffer[i] = (buffer[i] as i32 + (triangle >> 1) as i32) as i16;
        }
        self.previous_parameter = self.parameter;
        self.previous_phase_increment = ramp.value();
        self.phase = phase;
    }

    fn render_sine_fold(&mut self, sync_in: &[u8], buffer: &mut [i16]) {
        let n = buffer.len();
        let mut phase = self.phase;
        let mut ramp = PhaseIncrementRamp::new(self.previous_phase_increment, self.phase_increment, n);
        let mut param = ParamRamp::new(self.previous_parameter, self.parameter, n);
        for i in 0..n {
            let parameter = param.next();
            let phase_increment = ramp.next();
            let gain = (2048 + (parameter * 30720 >> 15)) as i16 as i32;
            if Self::sync_at(sync_in, i) != 0 {
                phase = 0;
            }
            // 2x oversampled wavefolder.
            phase = phase.wrapping_add(phase_increment >> 1);
            let mut sine = interpolate824(&WAV_SINE, phase);
            sine = (sine as i32 * gain >> 15) as i16;
            sine = interpolate88(&WS_SINE_FOLD, (sine as i32 + 32768) as u16);
            buffer[i] = sine >> 1;
            phase = phase.wrapping_add(phase_increment >> 1);
            let mut sine = interpolate824(&WAV_SINE, phase);
            sine = (sine as i32 * gain >> 15) as i16;
            sine = interpolate88(&WS_SINE_FOLD, (sine as i32 + 32768) as u16);
            buffer[i] = (buffer[i] as i32 + (sine >> 1) as i32) as i16;
        }
        self.previous_parameter = self.parameter;
        self.previous_phase_increment = ramp.value();
        self.phase = phase;
    }

    fn render_buzz(&mut self, sync_in: &[u8], buffer: &mut [i16]) {
        let shifted_pitch = self.pitch as i32 + ((32767 - self.parameter as i32) >> 1);
        let crossfade_amount = (shifted_pitch << 6) as u16;
        let mut index = (shifted_pitch >> 10).max(0) as usize;
        if index >= K_NUM_ZONES {
            index = K_NUM_ZONES - 1;
        }
        let wave_1 = WAVEFORM_TABLE[WAV_BANDLIMITED_COMB_0 + index];
        index += 1;
        if index >= K_NUM_ZONES {
            index = K_NUM_ZONES - 1;
        }
        let wave_2 = WAVEFORM_TABLE[WAV_BANDLIMITED_COMB_0 + index];
        for i in 0..buffer.len() {
            self.phase = self.phase.wrapping_add(self.phase_increment);
            if Self::sync_at(sync_in, i) != 0 {
                self.phase = 0;
            }
            buffer[i] = crossfade(wave_1, wave_2, self.phase, crossfade_amount);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHAPES: [AnalogShape; 9] = [
        AnalogShape::Saw,
        AnalogShape::VariableSaw,
        AnalogShape::CSaw,
        AnalogShape::Square,
        AnalogShape::Triangle,
        AnalogShape::Sine,
        AnalogShape::TriangleFold,
        AnalogShape::SineFold,
        AnalogShape::Buzz,
    ];

    fn render(shape: AnalogShape, note: i16, param: i16, blocks: usize) -> Vec<i16> {
        let mut o = AnalogOscillator::new();
        let sync = [0u8; 24];
        let mut out = Vec::with_capacity(blocks * 24);
        let mut buf = [0i16; 24];
        for _ in 0..blocks {
            // Like the macro oscillator: everything is set before each block
            // (a shape change resets the pitch for one block, as on the module).
            o.set_shape(shape);
            o.set_pitch(note << 7);
            o.set_parameter(param);
            o.set_aux_parameter(param);
            o.render(&sync, &mut buf, None);
            out.extend_from_slice(&buf);
        }
        out
    }

    #[test]
    fn every_shape_sounds_at_every_pitch() {
        for shape in SHAPES {
            for note in [24, 48, 60, 84, 108] {
                for param in [0, 16384, 32767] {
                    let out = render(shape, note, param, 4000);
                    let peak = out.iter().map(|s| (*s as i32).abs()).max().unwrap();
                    assert!(peak > 500, "{shape:?} note {note} param {param}: peak {peak}");
                }
            }
        }
    }

    #[test]
    fn saw_is_in_tune() {
        // A4 at 96 kHz for one second.
        let out = render(AnalogShape::Sine, 69, 0, 4000);
        let ups = out.windows(2).filter(|w| w[0] < 0 && w[1] >= 0).count() as f32;
        assert!((ups - 440.0).abs() < 440.0 * 0.03, "sine A4 at {ups} Hz");
        let out = render(AnalogShape::Saw, 69, 0, 4000);
        // The ramp crosses zero upward once per cycle.
        let ups = out.windows(2).filter(|w| w[0] < 0 && w[1] >= 0).count() as f32;
        assert!((ups - 440.0).abs() < 440.0 * 0.03, "saw A4 at {ups} Hz");
    }

    #[test]
    fn hard_sync_follows_the_master() {
        let mut master = AnalogOscillator::new();
        let mut slave = AnalogOscillator::new();
        let none = [0u8; 24];
        let mut sync = [0u8; 24];
        let mut a = [0i16; 24];
        let mut b = [0i16; 24];
        let mut resets = 0;
        for _ in 0..4000 {
            master.set_shape(AnalogShape::Saw);
            slave.set_shape(AnalogShape::Saw);
            master.set_pitch(57 << 7);
            slave.set_pitch(64 << 7);
            sync.fill(0);
            master.render(&none, &mut a, Some(&mut sync));
            resets += sync.iter().filter(|s| **s != 0).count();
            slave.render(&sync, &mut b, None);
        }
        assert!((resets as f32 - 220.0).abs() < 8.0, "{resets} master cycles");
    }
}

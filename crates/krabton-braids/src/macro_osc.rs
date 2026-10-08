//! The macro-oscillator: picks and combines the analog and digital
//! oscillators for each of the Braids shapes.
//!
//! Port of `braids/macro_oscillator.cc` (Copyright 2012 Emilie Gillet, MIT).

use crate::analog::{AnalogOscillator, AnalogShape};
use crate::digital::DigitalOscillator;
use crate::dsp::{clip, interpolate824_u16, interpolate88, mix, ParamRamp};
use crate::resources::{LUT_SVF_CUTOFF, WS_VIOLENT_OVERDRIVE};
use crate::BLOCK_SIZE;

/// Every shape: the code shown on the module and a readable name. The last
/// one (the "?" easter egg) is not reachable from the module's menu.
pub const SHAPES: [(&str, &str); 48] = [
    ("CSAW", "CSaw"),
    ("MRPH", "Morph"),
    ("SAW-", "Saw / Square"),
    ("FOLD", "Sine / Tri Fold"),
    ("BUZZ", "Buzz"),
    ("SUB-", "Square Sub"),
    ("SUB/", "Saw Sub"),
    ("SYN-", "Square Sync"),
    ("SYN/", "Saw Sync"),
    ("//x3", "Triple Saw"),
    ("-_x3", "Triple Square"),
    ("/\\x3", "Triple Triangle"),
    ("SIx3", "Triple Sine"),
    ("RING", "Triple Ring Mod"),
    ("SWRM", "Saw Swarm"),
    ("COMB", "Saw Comb"),
    ("TOY*", "Toy"),
    ("ZLPF", "Digital LP Filter"),
    ("ZPKF", "Digital Peak Filter"),
    ("ZBPF", "Digital BP Filter"),
    ("ZHPF", "Digital HP Filter"),
    ("VOSM", "VOSIM"),
    ("VOWL", "Vowel"),
    ("VFOF", "Vowel FOF"),
    ("HARM", "Harmonics"),
    ("FM  ", "FM"),
    ("FBFM", "Feedback FM"),
    ("WTFM", "Chaotic FM"),
    ("PLUK", "Plucked"),
    ("BOWD", "Bowed"),
    ("BLOW", "Blown"),
    ("FLUT", "Fluted"),
    ("BELL", "Bell"),
    ("DRUM", "Drum"),
    ("KICK", "Kick"),
    ("CYMB", "Cymbal"),
    ("SNAR", "Snare"),
    ("WTBL", "Wavetables"),
    ("WMAP", "Wave Map"),
    ("WLIN", "Wave Line"),
    ("WTx4", "Wave Paraphonic"),
    ("NOIS", "Filtered Noise"),
    ("TWNQ", "Twin Peaks Noise"),
    ("CLKN", "Clocked Noise"),
    ("CLOU", "Granular Cloud"),
    ("PRTC", "Particle Noise"),
    ("QPSK", "Digital Modulation"),
    ("????", "Question Mark"),
];
pub const SHAPE_COUNT: usize = 48;

const CSAW: usize = 0;
const MORPH: usize = 1;
const SAW_SQUARE: usize = 2;
const SINE_TRIANGLE: usize = 3;
const BUZZ: usize = 4;
const SQUARE_SUB: usize = 5;
const SAW_SUB: usize = 6;
const SQUARE_SYNC: usize = 7;
const SAW_SYNC: usize = 8;
const TRIPLE_SAW: usize = 9;
const TRIPLE_SQUARE: usize = 10;
const TRIPLE_TRIANGLE: usize = 11;
const TRIPLE_SINE: usize = 12;
const TRIPLE_RING_MOD: usize = 13;
const SAW_COMB: usize = 15;
/// `OSC_SHAPE_COMB_FILTER` in the digital oscillator.
const DIGITAL_COMB_FILTER: usize = 2;

const SEMI: i16 = 128;

static INTERVALS: [i16; 65] = [
    -24 * SEMI, -24 * SEMI, -24 * SEMI + 4,
    -23 * SEMI, -22 * SEMI, -21 * SEMI, -20 * SEMI, -19 * SEMI, -18 * SEMI,
    -17 * SEMI - 4, -17 * SEMI,
    -16 * SEMI, -15 * SEMI, -14 * SEMI, -13 * SEMI,
    -12 * SEMI - 4, -12 * SEMI,
    -11 * SEMI, -10 * SEMI, -9 * SEMI, -8 * SEMI,
    -7 * SEMI - 4, -7 * SEMI,
    -6 * SEMI, -5 * SEMI, -4 * SEMI, -3 * SEMI, -2 * SEMI, -1 * SEMI,
    -24, -8, -4, 0, 4, 8, 24,
    1 * SEMI, 2 * SEMI, 3 * SEMI, 4 * SEMI, 5 * SEMI, 6 * SEMI,
    7 * SEMI, 7 * SEMI + 4,
    8 * SEMI, 9 * SEMI, 10 * SEMI, 11 * SEMI,
    12 * SEMI, 12 * SEMI + 4,
    13 * SEMI, 14 * SEMI, 15 * SEMI, 16 * SEMI,
    17 * SEMI, 17 * SEMI + 4,
    18 * SEMI, 19 * SEMI, 20 * SEMI, 21 * SEMI, 22 * SEMI, 23 * SEMI,
    24 * SEMI - 4, 24 * SEMI, 24 * SEMI,
];

pub struct MacroOscillator {
    parameter: [i16; 2],
    previous_parameter: [i16; 2],
    pitch: i16,
    sync_buffer: [u8; BLOCK_SIZE],
    temp_buffer: [i16; BLOCK_SIZE],
    lp_state: i32,
    analog: [AnalogOscillator; 3],
    digital: Box<DigitalOscillator>,
    shape: usize,
}

impl MacroOscillator {
    pub fn new() -> Self {
        let mut m = Self {
            parameter: [0; 2],
            previous_parameter: [0; 2],
            pitch: 60 << 7,
            sync_buffer: [0; BLOCK_SIZE],
            temp_buffer: [0; BLOCK_SIZE],
            lp_state: 0,
            analog: [AnalogOscillator::new(), AnalogOscillator::new(), AnalogOscillator::new()],
            digital: DigitalOscillator::new_boxed(),
            shape: 0,
        };
        m.init();
        m
    }

    pub fn init(&mut self) {
        for a in self.analog.iter_mut() {
            a.init();
        }
        self.digital.init();
        self.lp_state = 0;
        self.previous_parameter = [0; 2];
    }

    pub fn set_shape(&mut self, shape: usize) {
        let shape = shape.min(SHAPE_COUNT - 1);
        if shape != self.shape {
            self.strike();
        }
        self.shape = shape;
    }

    pub fn shape(&self) -> usize {
        self.shape
    }

    pub fn set_pitch(&mut self, pitch: i16) {
        self.pitch = pitch;
    }

    pub fn pitch(&self) -> i16 {
        self.pitch
    }

    /// TIMBRE and COLOR, 0..32767.
    pub fn set_parameters(&mut self, parameter_1: i16, parameter_2: i16) {
        self.parameter = [parameter_1, parameter_2];
    }

    /// Restarts the percussive and physical-model shapes.
    pub fn strike(&mut self) {
        self.digital.strike();
    }

    /// Renders up to [`BLOCK_SIZE`] samples. `sync` holds 1 where an
    /// external sync/trigger edge happens (it may be all zeros).
    pub fn render(&mut self, sync: &[u8], buffer: &mut [i16]) {
        let n = buffer.len().min(BLOCK_SIZE);
        let buffer = &mut buffer[..n];
        let sync = &sync[..n.min(sync.len())];
        match self.shape {
            CSAW => self.render_csaw(sync, buffer),
            MORPH => self.render_morph(sync, buffer),
            SAW_SQUARE => self.render_saw_square(sync, buffer),
            SINE_TRIANGLE => self.render_sine_triangle(sync, buffer),
            BUZZ => self.render_buzz(sync, buffer),
            SQUARE_SUB | SAW_SUB => self.render_sub(sync, buffer),
            SQUARE_SYNC | SAW_SYNC => self.render_dual_sync(sync, buffer),
            TRIPLE_SAW..=TRIPLE_SINE => self.render_triple(sync, buffer),
            SAW_COMB => self.render_saw_comb(sync, buffer),
            _ => self.render_digital(sync, buffer),
        }
    }

    fn render_csaw(&mut self, sync: &[u8], buffer: &mut [i16]) {
        let a = &mut self.analog[0];
        a.set_pitch(self.pitch);
        a.set_shape(AnalogShape::CSaw);
        a.set_parameter(self.parameter[0]);
        a.set_aux_parameter(self.parameter[1]);
        a.render(sync, buffer, None);
        let shift = (-(self.parameter[1] as i32 - 32767) >> 4) as i16;
        for s in buffer.iter_mut() {
            let v = *s as i32 + shift as i32;
            *s = ((v * 13) >> 3) as i16;
        }
    }

    fn render_morph(&mut self, sync: &[u8], buffer: &mut [i16]) {
        let n = buffer.len();
        self.analog[0].set_pitch(self.pitch);
        self.analog[1].set_pitch(self.pitch);
        let p0 = self.parameter[0] as i32;
        let balance: u16;
        if p0 <= 10922 {
            self.analog[0].set_parameter(0);
            self.analog[1].set_parameter(0);
            self.analog[0].set_shape(AnalogShape::Triangle);
            self.analog[1].set_shape(AnalogShape::Saw);
            balance = (p0 * 6) as u16;
        } else if p0 <= 21845 {
            self.analog[0].set_parameter(0);
            self.analog[1].set_parameter(0);
            self.analog[0].set_shape(AnalogShape::Square);
            self.analog[1].set_shape(AnalogShape::Saw);
            balance = (65535 - (p0 - 10923) * 6) as u16;
        } else {
            self.analog[0].set_parameter(((p0 - 21846) * 3) as i16);
            self.analog[1].set_parameter(0);
            self.analog[0].set_shape(AnalogShape::Square);
            self.analog[1].set_shape(AnalogShape::Sine);
            balance = 0;
        }
        let temp = &mut self.temp_buffer[..n];
        self.analog[0].render(sync, buffer, None);
        self.analog[1].render(sync, temp, None);

        let mut lp_cutoff = self.pitch as i32 - (self.parameter[1] as i32 >> 1) + 128 * 128;
        lp_cutoff = lp_cutoff.clamp(0, 32767);
        let f = interpolate824_u16(&LUT_SVF_CUTOFF, (lp_cutoff as u32) << 17) as i32;
        let mut lp_state = self.lp_state;
        let mut fuzz_amount = (self.parameter[1] as i32) << 1;
        if self.pitch as i32 > (80 << 7) {
            fuzz_amount -= (self.pitch as i32 - (80 << 7)) << 4;
            if fuzz_amount < 0 {
                fuzz_amount = 0;
            }
        }
        for (s, t) in buffer.iter_mut().zip(temp.iter()) {
            let sample = mix(*s, *t, balance);
            let mut shifted_sample = sample as i32;
            lp_state += (shifted_sample - lp_state) * f >> 15;
            lp_state = clip(lp_state);
            shifted_sample = lp_state + 32768;
            let fuzzed = interpolate88(&WS_VIOLENT_OVERDRIVE, shifted_sample as u16);
            *s = mix(sample, fuzzed, fuzz_amount as u16);
        }
        self.lp_state = lp_state;
    }

    fn render_saw_square(&mut self, sync: &[u8], buffer: &mut [i16]) {
        let n = buffer.len();
        self.analog[0].set_parameter(self.parameter[0]);
        self.analog[1].set_parameter(self.parameter[0]);
        self.analog[0].set_pitch(self.pitch);
        self.analog[1].set_pitch(self.pitch);
        self.analog[0].set_shape(AnalogShape::VariableSaw);
        self.analog[1].set_shape(AnalogShape::Square);
        let square = &mut self.temp_buffer[..n];
        self.analog[0].render(sync, buffer, None);
        self.analog[1].render(sync, square, None);
        let mut ramp = ParamRamp::new(self.previous_parameter[1], self.parameter[1], n);
        for (s, q) in buffer.iter_mut().zip(square.iter()) {
            let parameter_1 = ramp.next();
            let balance = (parameter_1 << 1) as u16;
            let attenuated_square = ((*q as i32) * 148 >> 8) as i16;
            *s = mix(*s, attenuated_square, balance);
        }
        self.previous_parameter[1] = self.parameter[1];
    }

    fn render_triple(&mut self, sync: &[u8], buffer: &mut [i16]) {
        let n = buffer.len();
        let base_shape = match self.shape {
            TRIPLE_SAW => AnalogShape::Saw,
            TRIPLE_TRIANGLE => AnalogShape::Triangle,
            TRIPLE_SQUARE => AnalogShape::Square,
            _ => AnalogShape::Sine,
        };
        for a in self.analog.iter_mut() {
            a.set_parameter(0);
        }
        self.analog[0].set_pitch(self.pitch);
        for i in 0..2 {
            let p = self.parameter[i] as i32;
            let detune_1 = INTERVALS[(p >> 9) as usize] as i32;
            let detune_2 = INTERVALS[(((p >> 8) + 1) >> 1) as usize] as i32;
            let xfade = ((p << 8) as u16) as i32;
            let detune = detune_1 + ((detune_2 - detune_1) * xfade >> 16);
            self.analog[i + 1].set_pitch((self.pitch as i32 + detune) as i16);
        }
        for a in self.analog.iter_mut() {
            a.set_shape(base_shape);
        }
        buffer.fill(0);
        for i in 0..3 {
            let temp = &mut self.temp_buffer[..n];
            self.analog[i].render(sync, temp, None);
            for (s, t) in buffer.iter_mut().zip(temp.iter()) {
                *s = (*s as i32 + ((*t as i32) * 21 >> 6)) as i16;
            }
        }
    }

    fn render_sub(&mut self, sync: &[u8], buffer: &mut [i16]) {
        let n = buffer.len();
        let base_shape = if self.shape == SQUARE_SUB { AnalogShape::Square } else { AnalogShape::VariableSaw };
        self.analog[0].set_parameter(self.parameter[0]);
        self.analog[0].set_shape(base_shape);
        self.analog[0].set_pitch(self.pitch);
        self.analog[1].set_parameter(0);
        self.analog[1].set_shape(AnalogShape::Square);
        let octave: i32 = if self.parameter[1] < 16384 { 24 << 7 } else { 12 << 7 };
        self.analog[1].set_pitch((self.pitch as i32 - octave) as i16);
        let temp = &mut self.temp_buffer[..n];
        self.analog[0].render(sync, buffer, None);
        self.analog[1].render(sync, temp, None);
        let mut ramp = ParamRamp::new(self.previous_parameter[1], self.parameter[1], n);
        for (s, t) in buffer.iter_mut().zip(temp.iter()) {
            let parameter_1 = ramp.next();
            let sub_gain = ((if parameter_1 < 16384 { 16383 - parameter_1 } else { parameter_1 - 16384 }) << 1) as u16;
            *s = mix(*s, *t, sub_gain);
        }
        self.previous_parameter[1] = self.parameter[1];
    }

    fn render_dual_sync(&mut self, sync: &[u8], buffer: &mut [i16]) {
        let n = buffer.len();
        let base_shape = if self.shape == SQUARE_SYNC { AnalogShape::Square } else { AnalogShape::Saw };
        self.analog[0].set_parameter(0);
        self.analog[0].set_shape(base_shape);
        self.analog[0].set_pitch(self.pitch);
        self.analog[1].set_parameter(0);
        self.analog[1].set_shape(base_shape);
        self.analog[1].set_pitch((self.pitch as i32 + (self.parameter[0] as i32 >> 2)) as i16);
        let sync_out = &mut self.sync_buffer[..n];
        sync_out.fill(0);
        self.analog[0].render(sync, buffer, Some(sync_out));
        let temp = &mut self.temp_buffer[..n];
        self.analog[1].render(&self.sync_buffer[..n], temp, None);
        let mut ramp = ParamRamp::new(self.previous_parameter[1], self.parameter[1], n);
        for (s, t) in buffer.iter_mut().zip(self.temp_buffer[..n].iter()) {
            let parameter_1 = ramp.next();
            let balance = (parameter_1 << 1) as u16;
            *s = ((mix(*s, *t, balance) >> 2) as i32 * 3) as i16;
        }
        self.previous_parameter[1] = self.parameter[1];
    }

    fn render_sine_triangle(&mut self, sync: &[u8], buffer: &mut [i16]) {
        let n = buffer.len();
        let pitch = self.pitch as i32;
        let attenuation_sine = (32767 - 6 * (pitch - (92 << 7))).clamp(0, 32767);
        let attenuation_tri = (32767 - 7 * (pitch - (80 << 7))).clamp(0, 32767);
        let timbre = self.parameter[0] as i32;
        self.analog[0].set_parameter((timbre * attenuation_sine >> 15) as i16);
        self.analog[1].set_parameter((timbre * attenuation_tri >> 15) as i16);
        self.analog[0].set_pitch(self.pitch);
        self.analog[1].set_pitch(self.pitch);
        self.analog[0].set_shape(AnalogShape::SineFold);
        self.analog[1].set_shape(AnalogShape::TriangleFold);
        let temp = &mut self.temp_buffer[..n];
        self.analog[0].render(sync, buffer, None);
        self.analog[1].render(sync, temp, None);
        let mut ramp = ParamRamp::new(self.previous_parameter[1], self.parameter[1], n);
        for (s, t) in buffer.iter_mut().zip(temp.iter()) {
            let parameter_1 = ramp.next();
            let balance = (parameter_1 << 1) as u16;
            *s = mix(*s, *t, balance);
        }
        self.previous_parameter[1] = self.parameter[1];
    }

    fn render_buzz(&mut self, sync: &[u8], buffer: &mut [i16]) {
        let n = buffer.len();
        self.analog[0].set_parameter(self.parameter[0]);
        self.analog[0].set_shape(AnalogShape::Buzz);
        self.analog[0].set_pitch(self.pitch);
        self.analog[1].set_parameter(self.parameter[0]);
        self.analog[1].set_shape(AnalogShape::Buzz);
        self.analog[1].set_pitch((self.pitch as i32 + (self.parameter[1] as i32 >> 8)) as i16);
        let temp = &mut self.temp_buffer[..n];
        self.analog[0].render(sync, buffer, None);
        self.analog[1].render(sync, temp, None);
        for (s, t) in buffer.iter_mut().zip(temp.iter()) {
            *s >>= 1;
            *s = (*s as i32 + (*t >> 1) as i32) as i16;
        }
    }

    fn render_digital(&mut self, sync: &[u8], buffer: &mut [i16]) {
        self.digital.set_parameters(self.parameter[0], self.parameter[1]);
        self.digital.set_pitch(self.pitch);
        self.digital.set_shape(self.shape - TRIPLE_RING_MOD);
        self.digital.render(sync, buffer);
    }

    fn render_saw_comb(&mut self, sync: &[u8], buffer: &mut [i16]) {
        self.analog[0].set_parameter(0);
        self.analog[0].set_pitch(self.pitch);
        self.analog[0].set_shape(AnalogShape::Saw);
        self.analog[0].render(sync, buffer, None);
        self.digital.set_parameters(self.parameter[0], self.parameter[1]);
        self.digital.set_pitch(self.pitch);
        self.digital.set_shape(DIGITAL_COMB_FILTER);
        self.digital.render(sync, buffer);
    }
}

impl Default for MacroOscillator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_shape_renders_at_every_setting() {
        for shape in 0..SHAPE_COUNT {
            let mut m = MacroOscillator::new();
            let sync = [0u8; BLOCK_SIZE];
            let mut buf = [0i16; BLOCK_SIZE];
            let mut peak = 0i32;
            for (note, timbre, color) in [(36, 0, 0), (60, 16384, 16384), (84, 32767, 32767), (48, 32767, 0), (72, 0, 32767)] {
                m.strike();
                for _ in 0..2000 {
                    m.set_shape(shape);
                    m.set_pitch(note << 7);
                    m.set_parameters(timbre, color);
                    m.render(&sync, &mut buf);
                    peak = peak.max(buf.iter().map(|s| (*s as i32).abs()).max().unwrap());
                }
            }
            assert!(peak > 100, "{} is silent", SHAPES[shape].1);
        }
    }

    #[test]
    fn csaw_is_in_tune() {
        let mut m = MacroOscillator::new();
        let sync = [0u8; BLOCK_SIZE];
        let mut buf = [0i16; BLOCK_SIZE];
        let mut out = Vec::new();
        for _ in 0..4000 {
            m.set_shape(3); // sine / triangle fold at zero timbre: a plain sine
            m.set_pitch(69 << 7);
            m.set_parameters(0, 0);
            m.render(&sync, &mut buf);
            out.extend_from_slice(&buf);
        }
        let ups = out.windows(2).filter(|w| w[0] < 0 && w[1] >= 0).count() as f32;
        assert!((ups - 440.0).abs() < 13.0, "A4 at {ups} Hz");
    }
}

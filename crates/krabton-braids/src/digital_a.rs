//! Digital oscillator render functions, part A: triple ring modulator, saw
//! swarm, comb filter, toy, digital filters, VOSIM, vowels, FOF vowels,
//! harmonics, the three FM flavours, struck bell and struck drum.
//!
//! Port of `braids/digital_oscillator.cc` (Copyright 2012 Emilie Gillet,
//! MIT). Integer arithmetic follows the C++ code.

use crate::digital::{
    DigitalOscillator, K_COMB_DELAY_LENGTH, K_FIR4_COEFFICIENTS, K_FIR4_DC_OFFSET, K_NUM_ADDITIVE_HARMONICS, K_NUM_BELL_PARTIALS, K_NUM_DRUM_PARTIALS,
    K_NUM_FORMANTS, OSC_SHAPE_DIGITAL_FILTER_LP,
};
use crate::dsp::{clip, interpolate824, interpolate824_u16, interpolate88, mix, random_sample, sdiv, udiv, ParamRamp};
use crate::resources::{LUT_BELL, LUT_SVF_CUTOFF, LUT_SVF_DAMP, WAV_FORMANT_SINE, WAV_FORMANT_SQUARE, WAV_SINE, WS_MODERATE_OVERDRIVE};

#[inline(always)]
fn sync_at(sync: &[u8], i: usize) -> bool {
    sync.get(i).copied().unwrap_or(0) != 0
}

const K_PHASE_RESET: [u32; 4] = [0, 0x8000_0000, 0x4000_0000, 0x8000_0000];

struct PhonemeDefinition {
    formant_frequency: [u8; 3],
    formant_amplitude: [u8; 3],
}

const fn ph(f: [u8; 3], a: [u8; 3]) -> PhonemeDefinition {
    PhonemeDefinition { formant_frequency: f, formant_amplitude: a }
}

static VOWELS_DATA: [PhonemeDefinition; 9] = [
    ph([27, 40, 89], [15, 13, 1]),
    ph([18, 51, 62], [13, 12, 6]),
    ph([15, 69, 93], [14, 12, 7]),
    ph([10, 84, 110], [13, 10, 8]),
    ph([23, 44, 87], [15, 12, 1]),
    ph([13, 29, 80], [13, 8, 0]),
    ph([6, 46, 81], [12, 3, 0]),
    ph([9, 51, 95], [15, 3, 0]),
    ph([6, 73, 99], [7, 3, 14]),
];

static CONSONANT_DATA: [PhonemeDefinition; 8] = [
    ph([6, 54, 121], [9, 9, 0]),
    ph([18, 50, 51], [12, 10, 5]),
    ph([11, 24, 70], [13, 8, 0]),
    ph([15, 69, 74], [14, 12, 7]),
    ph([16, 37, 111], [14, 8, 1]),
    ph([18, 51, 62], [14, 12, 6]),
    ph([6, 26, 81], [5, 5, 5]),
    ph([6, 73, 99], [7, 10, 14]),
];

type FormantTable = [[[i16; K_NUM_FORMANTS]; K_NUM_FORMANTS]; K_NUM_FORMANTS];

static FORMANT_F_DATA: FormantTable = [
    // bass
    [
        [9519, 10738, 12448, 12636, 12892],
        [8620, 11720, 12591, 12932, 13158],
        [7579, 11891, 12768, 13122, 13323],
        [8620, 10013, 12591, 12768, 13010],
        [8324, 9519, 12591, 12831, 13048],
    ],
    // tenor
    [
        [9696, 10821, 12810, 13010, 13263],
        [8620, 11827, 12768, 13228, 13477],
        [7908, 12038, 12932, 13263, 13452],
        [8620, 10156, 12768, 12932, 13085],
        [8324, 9519, 12852, 13010, 13296],
    ],
    // countertenor
    [
        [9730, 10902, 12892, 13085, 13330],
        [8832, 11953, 12852, 13085, 13296],
        [7749, 12014, 13010, 13330, 13483],
        [8781, 10211, 12852, 13085, 13296],
        [8448, 9627, 12892, 13085, 13363],
    ],
    // alto
    [
        [10156, 10960, 12932, 13427, 14195],
        [8620, 11692, 12852, 13296, 14195],
        [8324, 11827, 12852, 13550, 14195],
        [8881, 10156, 12956, 13427, 14195],
        [8160, 9860, 12708, 13427, 14195],
    ],
    // soprano
    [
        [10156, 10960, 13010, 13667, 14195],
        [8324, 12187, 12932, 13489, 14195],
        [7749, 12337, 13048, 13667, 14195],
        [8881, 10156, 12956, 13609, 14195],
        [8160, 9860, 12852, 13609, 14195],
    ],
];

static FORMANT_A_DATA: FormantTable = [
    // bass
    [
        [16384, 7318, 5813, 5813, 1638],
        [16384, 4115, 5813, 4115, 2062],
        [16384, 518, 2596, 1301, 652],
        [16384, 4617, 1460, 1638, 163],
        [16384, 1638, 411, 652, 259],
    ],
    // tenor
    [
        [16384, 8211, 7318, 6522, 1301],
        [16384, 3269, 4115, 3269, 1638],
        [16384, 2913, 2062, 1638, 518],
        [16384, 5181, 4115, 4115, 821],
        [16384, 1638, 2314, 3269, 821],
    ],
    // countertenor
    [
        [16384, 8211, 1159, 1033, 206],
        [16384, 3269, 2062, 1638, 1638],
        [16384, 1033, 1033, 259, 259],
        [16384, 5181, 821, 1301, 326],
        [16384, 1638, 1159, 518, 326],
    ],
    // alto
    [
        [16384, 10337, 1638, 259, 16],
        [16384, 1033, 518, 291, 16],
        [16384, 1638, 518, 259, 16],
        [16384, 5813, 2596, 652, 29],
        [16384, 4115, 518, 163, 10],
    ],
    // soprano
    [
        [16384, 8211, 411, 1638, 51],
        [16384, 1638, 2913, 163, 25],
        [16384, 4115, 821, 821, 103],
        [16384, 4617, 1301, 1301, 51],
        [16384, 2596, 291, 163, 16],
    ],
];

static K_BELL_PARTIALS: [i16; K_NUM_BELL_PARTIALS] = [-1284, -1283, -184, -183, 385, 1175, 1536, 2233, 2434, 2934, 3110];
static K_BELL_PARTIAL_AMPLITUDES: [i16; K_NUM_BELL_PARTIALS] = [8192, 5488, 8192, 14745, 21872, 13680, 11960, 10895, 10895, 6144, 10895];
static K_BELL_PARTIAL_DECAY_LONG: [u16; K_NUM_BELL_PARTIALS] = [65533, 65533, 65533, 65532, 65531, 65531, 65530, 65529, 65527, 65523, 65519];
static K_BELL_PARTIAL_DECAY_SHORT: [u16; K_NUM_BELL_PARTIALS] = [65308, 65283, 65186, 65123, 64839, 64889, 64632, 64409, 64038, 63302, 62575];
static K_DRUM_PARTIALS: [i16; K_NUM_DRUM_PARTIALS] = [0, 0, 1041, 1747, 1846, 3072];
static K_DRUM_PARTIAL_AMPLITUDE: [i16; K_NUM_DRUM_PARTIALS] = [16986, 2654, 3981, 5308, 3981, 2985];
static K_DRUM_PARTIAL_DECAY_LONG: [u16; K_NUM_DRUM_PARTIALS] = [65533, 65531, 65531, 65531, 65531, 65516];
static K_DRUM_PARTIAL_DECAY_SHORT: [u16; K_NUM_DRUM_PARTIALS] = [65083, 64715, 64715, 64715, 64715, 62312];

fn interpolate_formant_parameter(table: &FormantTable, x: i16, y: i16, formant: usize) -> i16 {
    let x_index = ((x >> 13) as u16 as usize).min(K_NUM_FORMANTS - 2);
    let x_mix = (x << 3) as u16 as i32;
    let y_index = ((y >> 13) as u16 as usize).min(K_NUM_FORMANTS - 2);
    let y_mix = (y << 3) as u16 as i32;
    let mut a = table[x_index][y_index][formant] as i32;
    let b = table[x_index + 1][y_index][formant] as i32;
    let mut c = table[x_index][y_index + 1][formant] as i32;
    let d = table[x_index + 1][y_index + 1][formant] as i32;
    a = (a + ((b - a) * x_mix >> 16)) as i16 as i32;
    c = (c + ((d - c) * x_mix >> 16)) as i16 as i32;
    (a + ((c - a) * y_mix >> 16)) as i16
}

impl DigitalOscillator {
    /// `ComputePhaseIncrement(int16_t)` called with an `int` argument.
    #[inline(always)]
    fn increment_for(&self, pitch: i32) -> u32 {
        self.compute_phase_increment(pitch as i16)
    }

    pub(crate) fn render_triple_ring_mod(&mut self, sync: &[u8], buffer: &mut [i16]) {
        let mut phase = self.phase.wrapping_add(1 << 30);
        let increment = self.phase_increment;
        let mut modulator_phase = self.state.vow.formant_phase[0];
        let mut modulator_phase_2 = self.state.vow.formant_phase[1];
        let modulator_phase_increment = self.increment_for(self.pitch as i32 + ((self.parameter[0] as i32 - 16384) >> 2));
        let modulator_phase_increment_2 = self.increment_for(self.pitch as i32 + ((self.parameter[1] as i32 - 16384) >> 2));
        for (i, out) in buffer.iter_mut().enumerate() {
            phase = phase.wrapping_add(increment);
            if sync_at(sync, i) {
                phase = 0;
                modulator_phase = 0;
                modulator_phase_2 = 0;
            }
            modulator_phase = modulator_phase.wrapping_add(modulator_phase_increment);
            modulator_phase_2 = modulator_phase_2.wrapping_add(modulator_phase_increment_2);
            let mut result = interpolate824(&WAV_SINE, phase);
            result = ((result as i32 * interpolate824(&WAV_SINE, modulator_phase) as i32) >> 16) as i16;
            result = ((result as i32 * interpolate824(&WAV_SINE, modulator_phase_2) as i32) >> 16) as i16;
            result = interpolate88(&WS_MODERATE_OVERDRIVE, (result as i32 + 32768) as u16);
            *out = result;
        }
        self.phase = phase.wrapping_sub(1 << 30);
        self.state.vow.formant_phase[0] = modulator_phase;
        self.state.vow.formant_phase[1] = modulator_phase_2;
    }

    pub(crate) fn render_saw_swarm(&mut self, sync: &[u8], buffer: &mut [i16]) {
        let mut detune = self.parameter[0] as i32 + 1024;
        detune = (detune * detune) >> 9;
        let mut increments = [0u32; 7];
        for (i, inc) in increments.iter_mut().enumerate() {
            let saw_detune = detune * (i as i32 - 3);
            let detune_integral = saw_detune >> 16;
            let detune_fractional = saw_detune & 0xffff;
            let increment_a = self.increment_for(self.pitch as i32 + detune_integral) as i32;
            let increment_b = self.increment_for(self.pitch as i32 + detune_integral + 1) as i32;
            *inc = increment_a.wrapping_add(increment_b.wrapping_sub(increment_a).wrapping_mul(detune_fractional) >> 16) as u32;
        }
        if self.strike {
            for p in self.state.saw.phase.iter_mut() {
                *p = crate::dsp::random_word();
            }
            self.strike = false;
        }
        let p1 = self.parameter[1] as i32;
        let mut hp_cutoff = self.pitch as i32;
        if p1 < 10922 {
            hp_cutoff += ((p1 - 10922) * 24) >> 5;
        } else {
            hp_cutoff += ((p1 - 10922) * 12) >> 5;
        }
        hp_cutoff = hp_cutoff.clamp(0, 32767);
        let f = interpolate824_u16(&LUT_SVF_CUTOFF, (hp_cutoff as u32) << 17) as i32;
        let damp = LUT_SVF_DAMP[0] as i32;
        let mut bp = self.state.saw.bp;
        let mut lp = self.state.saw.lp;
        for (i, out) in buffer.iter_mut().enumerate() {
            if sync_at(sync, i) {
                for p in self.state.saw.phase.iter_mut() {
                    *p = 0;
                }
            }
            self.phase = self.phase.wrapping_add(increments[0]);
            for k in 0..6 {
                self.state.saw.phase[k] = self.state.saw.phase[k].wrapping_add(increments[k + 1]);
            }
            let mut sample: i32 = -28672;
            sample += (self.phase >> 19) as i32;
            for k in 0..6 {
                sample += (self.state.saw.phase[k] >> 19) as i32;
            }
            sample = interpolate88(&WS_MODERATE_OVERDRIVE, (sample + 32768) as u16) as i32;
            let notch = sample - (bp.wrapping_mul(damp) >> 15);
            lp = lp.wrapping_add(f.wrapping_mul(bp) >> 15);
            lp = clip(lp);
            let hp = notch - lp;
            bp = bp.wrapping_add(f.wrapping_mul(hp) >> 15);
            *out = clip(hp) as i16;
        }
        self.state.saw.lp = lp;
        self.state.saw.bp = bp;
    }

    pub(crate) fn render_comb(&mut self, _sync: &[u8], buffer: &mut [i16]) {
        // Filter the delay time to avoid clicks/glitches.
        let pitch = self.pitch as i32 + ((self.parameter[0] as i32 - 16384) >> 1);
        let mut filtered_pitch = self.state.ffm.previous_sample as i32;
        filtered_pitch = (15 * filtered_pitch + pitch) >> 4;
        self.state.ffm.previous_sample = filtered_pitch as i16;

        let mut delay = self.compute_delay(filtered_pitch as i16);
        if delay > (K_COMB_DELAY_LENGTH as u32) << 16 {
            delay = (K_COMB_DELAY_LENGTH as u32) << 16;
        }
        let delay_integral = delay >> 16;
        let delay_fractional = (delay & 0xffff) as i32;

        // Warp the resonance curve to have a more precise adjustment in the extrema.
        let mut resonance = (((self.parameter[1] as i32) << 1) - 32768) as i16;
        resonance = interpolate88(&WS_MODERATE_OVERDRIVE, (resonance as i32 + 32768) as u16);

        let len = K_COMB_DELAY_LENGTH as u32;
        let mut delay_ptr = self.phase % len;
        let dl = &mut self.delay_lines.comb;
        for out in buffer.iter_mut() {
            let input = *out as i32;
            let offset = delay_ptr.wrapping_add(2 * len).wrapping_sub(delay_integral);
            let a = dl[(offset % len) as usize] as i32;
            let b = dl[(offset.wrapping_sub(1) % len) as usize] as i32;
            let delayed_sample = a + (((b - a) * (delay_fractional >> 1)) >> 15);
            let mut feedback = (delayed_sample * resonance as i32 >> 15) + (input >> 1);
            feedback = clip(feedback);
            dl[delay_ptr as usize] = feedback as i16;
            let o = clip((input + (delayed_sample << 1)) >> 1);
            *out = o as i16;
            delay_ptr = (delay_ptr + 1) % len;
        }
        self.phase = delay_ptr;
    }

    pub(crate) fn render_toy(&mut self, sync: &[u8], buffer: &mut [i16]) {
        // 4 times oversampling.
        self.phase_increment >>= 2;
        let phase_increment = self.phase_increment;
        let mut phase = self.phase;
        let mut decimation_counter = self.state.toy.decimation_counter;
        let decimation_count = (512 - (self.parameter[0] as i32 >> 6)) as u16;
        let mut held_sample = self.state.toy.held_sample;
        for (i, out) in buffer.iter_mut().enumerate() {
            let mut filtered_sample: i32 = 0;
            if sync_at(sync, i) {
                phase = 0;
            }
            for coefficient in K_FIR4_COEFFICIENTS {
                phase = phase.wrapping_add(phase_increment);
                if decimation_counter >= decimation_count {
                    let x = (self.parameter[1] >> 8) as u8;
                    let xi = x as u32;
                    held_sample = (((phase >> 24) ^ (xi << 1)) & !xi).wrapping_add(xi >> 1) as u8;
                    decimation_counter = 0;
                }
                filtered_sample = filtered_sample.wrapping_add(coefficient.wrapping_mul(held_sample as u32) as i32);
                decimation_counter = decimation_counter.wrapping_add(1);
            }
            *out = ((filtered_sample >> 8) as u32).wrapping_sub(K_FIR4_DC_OFFSET) as i16;
        }
        self.state.toy.held_sample = held_sample;
        self.state.toy.decimation_counter = decimation_counter;
        self.phase = phase;
    }

    pub(crate) fn render_digital_filter(&mut self, sync: &[u8], buffer: &mut [i16]) {
        let size = buffer.len().max(1) as u32;
        let mut shifted_pitch = (self.pitch as i32 + ((self.parameter[0] as i32 - 2048) >> 1)) as i16;
        if shifted_pitch > 16383 {
            shifted_pitch = 16383;
        }
        let mut modulator_phase = self.state.res.modulator_phase;
        let mut square_modulator_phase = self.state.res.square_modulator_phase;
        let mut square_integrator = self.state.res.integrator;
        let filter_type = (self.shape - OSC_SHAPE_DIGITAL_FILTER_LP) as u8;

        let mut modulator_phase_increment = self.state.res.modulator_phase_increment;
        let target_increment = self.compute_phase_increment(shifted_pitch);
        let modulator_phase_increment_increment = if modulator_phase_increment < target_increment {
            (target_increment - modulator_phase_increment) / size
        } else {
            !((modulator_phase_increment - target_increment) / size)
        };
        let p1 = self.parameter[1] as i32;
        for (i, out) in buffer.iter_mut().enumerate() {
            self.phase = self.phase.wrapping_add(self.phase_increment);
            modulator_phase_increment = modulator_phase_increment.wrapping_add(modulator_phase_increment_increment);
            modulator_phase = modulator_phase.wrapping_add(modulator_phase_increment);
            let integrator_gain = (modulator_phase_increment >> 14) as u16 as i32;

            if sync_at(sync, i) {
                self.state.res.polarity = true;
                self.phase = 0;
                modulator_phase = 0;
                square_modulator_phase = 0;
                square_integrator = 0;
            }

            square_modulator_phase = square_modulator_phase.wrapping_add(modulator_phase_increment);
            if self.phase < self.phase_increment {
                modulator_phase = K_PHASE_RESET[filter_type as usize & 3];
            }
            if (self.phase << 1) < (self.phase_increment << 1) {
                self.state.res.polarity = !self.state.res.polarity;
                square_modulator_phase = K_PHASE_RESET[(filter_type as usize & 1) + 2];
            }

            let carrier = interpolate824(&WAV_SINE, modulator_phase) as i32;
            let square_carrier = interpolate824(&WAV_SINE, square_modulator_phase) as i32;

            let saw = !(self.phase >> 16) as u16 as i32;
            let double_saw = !(self.phase >> 15) as u16 as i32;
            let triangle = ((self.phase >> 15) ^ if self.phase & 0x8000_0000 != 0 { 0xffff } else { 0 }) as u16 as i32;
            let window = if p1 < 16384 { saw } else { triangle };

            let mut pulse = square_carrier.wrapping_mul(double_saw) >> 16;
            if self.state.res.polarity {
                pulse = -pulse;
            }
            square_integrator = square_integrator.wrapping_add(pulse.wrapping_mul(integrator_gain) >> 16);
            square_integrator = clip(square_integrator);

            let saw_tri_signal: i16;
            let mut square_signal: i16;
            if filter_type & 2 != 0 {
                saw_tri_signal = (carrier.wrapping_mul(window) >> 16) as i16;
                square_signal = pulse as i16;
            } else {
                saw_tri_signal = ((window.wrapping_mul(carrier + 32768) >> 16) - 32768) as i16;
                square_signal = square_integrator as i16;
                if filter_type == 1 {
                    square_signal = ((pulse + square_integrator) >> 1) as i16;
                }
            }
            let balance = ((if p1 < 16384 { p1 } else { !p1 }) << 2) as u16;
            *out = mix(saw_tri_signal, square_signal, balance);
        }
        self.state.res.modulator_phase = modulator_phase;
        self.state.res.square_modulator_phase = square_modulator_phase;
        self.state.res.integrator = square_integrator;
        self.state.res.modulator_phase_increment = modulator_phase_increment;
    }

    pub(crate) fn render_vosim(&mut self, sync: &[u8], buffer: &mut [i16]) {
        for i in 0..2 {
            self.state.vow.formant_increment[i] = self.compute_phase_increment(self.parameter[i] >> 1);
        }
        for (i, out) in buffer.iter_mut().enumerate() {
            self.phase = self.phase.wrapping_add(self.phase_increment);
            if sync_at(sync, i) {
                self.phase = 0;
            }
            let mut sample: i32 = 16384 + 8192;
            self.state.vow.formant_phase[0] = self.state.vow.formant_phase[0].wrapping_add(self.state.vow.formant_increment[0]);
            sample += (interpolate824(&WAV_SINE, self.state.vow.formant_phase[0]) >> 1) as i32;
            self.state.vow.formant_phase[1] = self.state.vow.formant_phase[1].wrapping_add(self.state.vow.formant_increment[1]);
            sample += (interpolate824(&WAV_SINE, self.state.vow.formant_phase[1]) >> 2) as i32;
            sample = sample * (interpolate824_u16(&LUT_BELL, self.phase) >> 1) as i32 >> 15;
            if self.phase < self.phase_increment {
                self.state.vow.formant_phase[0] = 0;
                self.state.vow.formant_phase[1] = 0;
                sample = 0;
            }
            sample -= 16384 + 8192;
            *out = sample as i16;
        }
    }

    pub(crate) fn render_vowel(&mut self, _sync: &[u8], buffer: &mut [i16]) {
        let vowel_index = ((self.parameter[0] >> 12).max(0) as usize).min(VOWELS_DATA.len() - 2);
        let balance = (self.parameter[0] & 0x0fff) as u16 as i32;
        let formant_shift = (200 + (self.parameter[1] as i32 >> 6)) as u16;
        if self.strike {
            self.strike = false;
            self.state.vow.consonant_frames = 160;
            let index = ((random_sample() as i32 + 1) & 7) as usize;
            for i in 0..3 {
                self.state.vow.formant_increment[i] = (CONSONANT_DATA[index].formant_frequency[i] as u32).wrapping_mul(0x1000).wrapping_mul(formant_shift as u32);
                self.state.vow.formant_amplitude[i] = CONSONANT_DATA[index].formant_amplitude[i] as u32;
            }
            self.state.vow.noise = if index >= 6 { 4095 } else { 0 };
        }

        if self.state.vow.consonant_frames != 0 {
            self.state.vow.consonant_frames -= 1;
        } else {
            let a = &VOWELS_DATA[vowel_index];
            let b = &VOWELS_DATA[vowel_index + 1];
            for i in 0..3 {
                self.state.vow.formant_increment[i] =
                    ((a.formant_frequency[i] as i32 * (0x1000 - balance) + b.formant_frequency[i] as i32 * balance) * formant_shift as i32) as u32;
                self.state.vow.formant_amplitude[i] = ((a.formant_amplitude[i] as i32 * (0x1000 - balance) + b.formant_amplitude[i] as i32 * balance) >> 12) as u32;
            }
            self.state.vow.noise = 0;
        }
        let noise = self.state.vow.noise as i32;

        for out in buffer.iter_mut() {
            self.phase = self.phase.wrapping_add(self.phase_increment);
            let mut sample: i16 = 0;
            let vow = &mut self.state.vow;
            vow.formant_phase[0] = vow.formant_phase[0].wrapping_add(vow.formant_increment[0]);
            let phaselet = ((vow.formant_phase[0] >> 24) & 0xf0) as usize;
            sample = (sample as i32 + WAV_FORMANT_SINE[(phaselet | vow.formant_amplitude[0] as usize) & 0xff] as i32) as i16;

            vow.formant_phase[1] = vow.formant_phase[1].wrapping_add(vow.formant_increment[1]);
            let phaselet = ((vow.formant_phase[1] >> 24) & 0xf0) as usize;
            sample = (sample as i32 + WAV_FORMANT_SINE[(phaselet | vow.formant_amplitude[1] as usize) & 0xff] as i32) as i16;

            vow.formant_phase[2] = vow.formant_phase[2].wrapping_add(vow.formant_increment[2]);
            let phaselet = ((vow.formant_phase[2] >> 24) & 0xf0) as usize;
            sample = (sample as i32 + WAV_FORMANT_SQUARE[(phaselet | vow.formant_amplitude[2] as usize) & 0xff] as i32) as i16;

            sample = (sample as i32 as u32).wrapping_mul(255 - (self.phase >> 24)) as i16;
            let phase_noise = (random_sample() as i32).wrapping_mul(noise);
            if self.phase.wrapping_add(phase_noise as u32) < self.phase_increment {
                vow.formant_phase = [0; 3];
                sample = 0;
            }
            *out = interpolate88(&WS_MODERATE_OVERDRIVE, (sample as i32 + 32768) as u16);
        }
    }

    pub(crate) fn render_vowel_fof(&mut self, _sync: &[u8], buffer: &mut [i16]) {
        // The original implementation used FOF but it is cheaper to render a
        // proper bank of 5 SVF.
        let mut amplitudes = [0i16; K_NUM_FORMANTS];
        let mut svf_lp = [0i32; K_NUM_FORMANTS];
        let mut svf_bp = [0i32; K_NUM_FORMANTS];
        let mut svf_f = [0i16; K_NUM_FORMANTS];
        for i in 0..K_NUM_FORMANTS {
            let frequency = interpolate_formant_parameter(&FORMANT_F_DATA, self.parameter[1], self.parameter[0], i) as i32 + (12 << 7);
            svf_f[i] = interpolate824_u16(&LUT_SVF_CUTOFF, (frequency as u32) << 17) as i16;
            amplitudes[i] = interpolate_formant_parameter(&FORMANT_A_DATA, self.parameter[1], self.parameter[0], i);
            if self.init {
                svf_lp[i] = 0;
                svf_bp[i] = 0;
            } else {
                svf_lp[i] = self.state.fof.svf_lp[i];
                svf_bp[i] = self.state.fof.svf_bp[i];
            }
        }
        if self.init {
            self.init = false;
        }

        let mut phase = self.phase;
        let mut previous_sample = self.state.fof.previous_sample as i32;
        let mut next_saw_sample = self.state.fof.next_saw_sample;
        let increment = self.phase_increment << 1;
        let n = buffer.len();
        let mut k = 0;
        while k + 1 < n {
            let mut this_saw_sample = next_saw_sample;
            next_saw_sample = 0;
            phase = phase.wrapping_add(increment);
            if phase < increment {
                let mut t = udiv(phase, increment >> 16);
                if t > 65535 {
                    t = 65535;
                }
                this_saw_sample -= (t * t >> 18) as i32;
                t = 65535 - t;
                next_saw_sample -= -((t * t >> 18) as i32);
            }
            next_saw_sample += (phase >> 17) as i32;
            let input = this_saw_sample;
            let mut out: i32 = 0;
            for i in 0..5 {
                let notch = input - (svf_bp[i] >> 6);
                svf_lp[i] = svf_lp[i].wrapping_add((svf_f[i] as i32).wrapping_mul(svf_bp[i]) >> 15);
                svf_lp[i] = clip(svf_lp[i]);
                let hp = notch - svf_lp[i];
                svf_bp[i] = svf_bp[i].wrapping_add((svf_f[i] as i32).wrapping_mul(hp) >> 15);
                svf_bp[i] = clip(svf_bp[i]);
                // The original uses the first amplitude for every formant.
                out += svf_bp[i] * amplitudes[0] as i32 >> 17;
            }
            out = clip(out);
            buffer[k] = ((out + previous_sample) >> 1) as i16;
            buffer[k + 1] = out as i16;
            previous_sample = out;
            k += 2;
        }
        self.phase = phase;
        self.state.fof.next_saw_sample = next_saw_sample;
        self.state.fof.previous_sample = previous_sample as i16;
        self.state.fof.svf_lp = svf_lp;
        self.state.fof.svf_bp = svf_bp;
    }

    fn fm_modulator_increment(&self) -> u32 {
        self.increment_for((12 << 7) + self.pitch as i32 + ((self.parameter[1] as i32 - 16384) >> 1)) >> 1
    }

    pub(crate) fn render_fm(&mut self, sync: &[u8], buffer: &mut [i16]) {
        let mut modulator_phase = self.state.modulator_phase;
        let modulator_phase_increment = self.fm_modulator_increment();
        let mut ramp = ParamRamp::new(self.previous_parameter[0], self.parameter[0], buffer.len());
        for (i, out) in buffer.iter_mut().enumerate() {
            let parameter_0 = ramp.next();
            self.phase = self.phase.wrapping_add(self.phase_increment);
            if sync_at(sync, i) {
                self.phase = 0;
                modulator_phase = 0;
            }
            modulator_phase = modulator_phase.wrapping_add(modulator_phase_increment);
            let pm = ((interpolate824(&WAV_SINE, modulator_phase) as i32).wrapping_mul(parameter_0) << 2) as u32;
            *out = interpolate824(&WAV_SINE, self.phase.wrapping_add(pm));
        }
        self.previous_parameter[0] = self.parameter[0];
        self.state.modulator_phase = modulator_phase;
    }

    pub(crate) fn render_feedback_fm(&mut self, sync: &[u8], buffer: &mut [i16]) {
        let mut previous_sample = self.state.ffm.previous_sample;
        let mut modulator_phase = self.state.ffm.modulator_phase;
        let mut attenuation = self.pitch as i32 - (72 << 7) + ((self.parameter[1] as i32 - 16384) >> 1);
        attenuation = (32767 - attenuation * 4).clamp(0, 32767);
        let modulator_phase_increment = self.fm_modulator_increment();
        let mut ramp = ParamRamp::new(self.previous_parameter[0], self.parameter[0], buffer.len());
        for (i, out) in buffer.iter_mut().enumerate() {
            let parameter_0 = ramp.next();
            self.phase = self.phase.wrapping_add(self.phase_increment);
            if sync_at(sync, i) {
                self.phase = 0;
                modulator_phase = 0;
            }
            modulator_phase = modulator_phase.wrapping_add(modulator_phase_increment);
            let p = parameter_0.wrapping_mul(attenuation) >> 15;
            let mut pm = (previous_sample as i32) << 14;
            pm = (interpolate824(&WAV_SINE, modulator_phase.wrapping_add(pm as u32)) as i32).wrapping_mul(p) << 1;
            previous_sample = interpolate824(&WAV_SINE, self.phase.wrapping_add(pm as u32));
            *out = previous_sample;
        }
        self.previous_parameter[0] = self.parameter[0];
        self.state.ffm.previous_sample = previous_sample;
        self.state.ffm.modulator_phase = modulator_phase;
    }

    pub(crate) fn render_chaotic_feedback_fm(&mut self, sync: &[u8], buffer: &mut [i16]) {
        let modulator_phase_increment = self.fm_modulator_increment();
        let mut previous_sample = self.state.ffm.previous_sample;
        let mut modulator_phase = self.state.ffm.modulator_phase;
        let mut ramp = ParamRamp::new(self.previous_parameter[0], self.parameter[0], buffer.len());
        for (i, out) in buffer.iter_mut().enumerate() {
            let parameter_0 = ramp.next();
            self.phase = self.phase.wrapping_add(self.phase_increment);
            if sync_at(sync, i) {
                self.phase = 0;
                modulator_phase = 0;
            }
            let pm = (interpolate824(&WAV_SINE, modulator_phase) as i32).wrapping_mul(parameter_0) << 1;
            previous_sample = interpolate824(&WAV_SINE, self.phase.wrapping_add(pm as u32));
            *out = previous_sample;
            modulator_phase = modulator_phase.wrapping_add((modulator_phase_increment >> 8).wrapping_mul((129 + (previous_sample as i32 >> 9)) as u32));
        }
        self.previous_parameter[0] = self.parameter[0];
        self.state.ffm.previous_sample = previous_sample;
        self.state.ffm.modulator_phase = modulator_phase;
    }

    pub(crate) fn render_struck_bell(&mut self, _sync: &[u8], buffer: &mut [i16]) {
        // To save CPU, only a third of the partials get a new frequency per
        // block (this makes fast pitch changes sound "arpeggiated").
        let mut first_partial = self.state.add.current_partial.min(K_NUM_BELL_PARTIALS);
        let mut last_partial = (self.state.add.current_partial + 3).min(K_NUM_BELL_PARTIALS);
        self.state.add.current_partial = (first_partial + 3) % K_NUM_BELL_PARTIALS;

        if self.strike {
            for i in 0..K_NUM_BELL_PARTIALS {
                self.state.add.partial_amplitude[i] = K_BELL_PARTIAL_AMPLITUDES[i] as i32;
                self.state.add.partial_phase[i] = 1 << 30;
            }
            self.strike = false;
            first_partial = 0;
            last_partial = K_NUM_BELL_PARTIALS;
        }

        for i in first_partial..last_partial {
            let mut partial_pitch = (self.pitch as i32 + K_BELL_PARTIALS[i] as i32) as i16;
            if i & 1 != 0 {
                partial_pitch = (partial_pitch as i32 + (self.parameter[1] as i32 >> 7)) as i16;
            } else {
                partial_pitch = (partial_pitch as i32 - (self.parameter[1] as i32 >> 7)) as i16;
            }
            self.state.add.partial_phase_increment[i] = self.compute_phase_increment(partial_pitch) << 1;
        }

        // A "droning" bell with no energy loss when the parameter is at its maximum.
        if self.parameter[0] < 32000 {
            for i in 0..K_NUM_BELL_PARTIALS {
                let decay_long = K_BELL_PARTIAL_DECAY_LONG[i] as i32;
                let decay_short = K_BELL_PARTIAL_DECAY_SHORT[i] as i32;
                let mut balance = ((32767 - self.parameter[0] as i32) >> 8) as i16;
                balance = (balance as i32 * balance as i32 >> 7) as i16;
                let decay = decay_long - ((decay_long - decay_short) * balance as i32 >> 7);
                self.state.add.partial_amplitude[i] = self.state.add.partial_amplitude[i].wrapping_mul(decay) >> 16;
            }
        }

        let mut previous_sample = self.state.add.previous_sample as i32;
        let n = buffer.len();
        let mut k = 0;
        while k + 1 < n {
            let mut out: i32 = 0;
            for i in 0..K_NUM_BELL_PARTIALS {
                let a = &mut self.state.add;
                a.partial_phase[i] = a.partial_phase[i].wrapping_add(a.partial_phase_increment[i]);
                let partial = interpolate824(&WAV_SINE, a.partial_phase[i]) as i32;
                out = out.wrapping_add(partial.wrapping_mul(a.partial_amplitude[i]) >> 17);
            }
            out = clip(out);
            buffer[k] = ((out + previous_sample) >> 1) as i16;
            buffer[k + 1] = out as i16;
            previous_sample = out;
            k += 2;
        }
        self.state.add.previous_sample = previous_sample as i16;
    }

    pub(crate) fn render_harmonics(&mut self, sync: &[u8], buffer: &mut [i16]) {
        let mut phase = self.phase;
        let mut previous_sample = self.state.add.previous_sample as i32;
        let phase_increment = self.phase_increment << 1;
        let mut target_amplitude = [0i32; K_NUM_ADDITIVE_HARMONICS];
        let mut amplitude = [0i32; K_NUM_ADDITIVE_HARMONICS];

        let p0 = self.parameter[0] as i32;
        let p1 = self.parameter[1] as i32;
        let peak = (K_NUM_ADDITIVE_HARMONICS as i32 * p0) >> 7;
        let second_peak = (peak >> 1) + K_NUM_ADDITIVE_HARMONICS as i32 * 128;
        let second_peak_amount = p1 * p1 >> 15;
        let sqrtsqrt_width = if p1 < 16384 { p1 >> 6 } else { 511 - (p1 >> 6) };
        let sqrt_width = sqrtsqrt_width * sqrtsqrt_width >> 10;
        let width = sqrt_width * sqrt_width + 4;
        let mut total: i32 = 0;
        for (i, target) in target_amplitude.iter_mut().enumerate() {
            let x = (i as i32) << 8;
            let mut d = x - peak;
            let mut g = sdiv(32768 * 128, 128 + sdiv(d * d, width));
            d = x - second_peak;
            g += sdiv(second_peak_amount * 128, 128 + sdiv(d * d, width));
            total = total.wrapping_add(g);
            *target = g;
        }

        let attenuation = sdiv(2147483647, total);
        for i in 0..K_NUM_ADDITIVE_HARMONICS {
            if (phase_increment >> 16).wrapping_mul(i as u32 + 1) > 0x4000 {
                target_amplitude[i] = 0;
            } else {
                target_amplitude[i] = target_amplitude[i].wrapping_mul(attenuation) >> 16;
            }
            amplitude[i] = self.state.hrm.amplitude[i];
        }

        let n = buffer.len();
        let mut k = 0;
        let mut s = 0usize;
        while k + 1 < n {
            phase = phase.wrapping_add(phase_increment);
            // `*sync++ || *sync++`: the second byte is only read when the first is 0.
            let first = sync_at(sync, s);
            s += 1;
            let hit = first || {
                let second = sync_at(sync, s);
                s += 1;
                second
            };
            if hit {
                phase = 0;
            }
            let mut out: i32 = 0;
            for i in 0..K_NUM_ADDITIVE_HARMONICS {
                out = out.wrapping_add((interpolate824(&WAV_SINE, phase.wrapping_mul(i as u32 + 1)) as i32).wrapping_mul(amplitude[i]) >> 15);
                amplitude[i] += (target_amplitude[i] - amplitude[i]) >> 8;
            }
            out = clip(out);
            buffer[k] = ((out + previous_sample) >> 1) as i16;
            buffer[k + 1] = out as i16;
            previous_sample = out;
            k += 2;
        }
        self.state.add.previous_sample = previous_sample as i16;
        self.phase = phase;
        self.state.hrm.amplitude = amplitude;
    }

    pub(crate) fn render_struck_drum(&mut self, _sync: &[u8], buffer: &mut [i16]) {
        if self.strike {
            let reset_phase = self.state.add.partial_amplitude[0] < 1024;
            for i in 0..K_NUM_DRUM_PARTIALS {
                self.state.add.target_partial_amplitude[i] = K_DRUM_PARTIAL_AMPLITUDE[i] as i32;
                if reset_phase {
                    self.state.add.partial_phase[i] = 1 << 30;
                }
            }
            self.strike = false;
        } else if self.parameter[0] < 32000 {
            for i in 0..K_NUM_DRUM_PARTIALS {
                let decay_long = K_DRUM_PARTIAL_DECAY_LONG[i] as i32;
                let decay_short = K_DRUM_PARTIAL_DECAY_SHORT[i] as i32;
                let mut balance = ((32767 - self.parameter[0] as i32) >> 8) as i16;
                balance = (balance as i32 * balance as i32 >> 7) as i16;
                let decay = decay_long - ((decay_long - decay_short) * balance as i32 >> 7);
                self.state.add.target_partial_amplitude[i] = self.state.add.partial_amplitude[i].wrapping_mul(decay) >> 16;
            }
        }

        for i in 0..K_NUM_DRUM_PARTIALS {
            let partial_pitch = (self.pitch as i32 + K_DRUM_PARTIALS[i] as i32) as i16;
            self.state.add.partial_phase_increment[i] = self.compute_phase_increment(partial_pitch) << 1;
        }

        let mut previous_sample = self.state.add.previous_sample as i32;
        let cutoff = ((self.pitch as i32 - 12 * 128) + (self.parameter[1] as i32 >> 2)).clamp(0, 32767);
        let f = interpolate824_u16(&LUT_SVF_CUTOFF, (cutoff as u32) << 16) as i32;
        let mut lp_state_0 = self.state.add.lp_noise[0];
        let mut lp_state_1 = self.state.add.lp_noise[1];
        let mut lp_state_2 = self.state.add.lp_noise[2];
        let p1 = self.parameter[1] as i32;
        let harmonics_gain = if p1 < 12888 { p1 + 4096 } else { 16384 };
        let mut noise_mode_gain = if p1 < 16384 { 0 } else { p1 - 16384 };
        noise_mode_gain = noise_mode_gain * 12888 >> 14;

        let n = buffer.len();
        let fade_increment = sdiv(65536, n as i32);
        let mut fade: i32 = 0;
        let mut k = 0;
        while k + 1 < n {
            fade += fade_increment;
            let mut harmonics: i32 = 0;
            let noise = (random_sample() as i32).clamp(-16384, 16384);
            lp_state_0 = lp_state_0.wrapping_add((noise - lp_state_0).wrapping_mul(f) >> 15);
            lp_state_1 = lp_state_1.wrapping_add((lp_state_0 - lp_state_1).wrapping_mul(f) >> 15);
            lp_state_2 = lp_state_2.wrapping_add((lp_state_1 - lp_state_2).wrapping_mul(f) >> 15);

            let mut partials = [0i32; K_NUM_DRUM_PARTIALS];
            for i in 0..K_NUM_DRUM_PARTIALS {
                let a = &mut self.state.add;
                a.partial_phase[i] = a.partial_phase[i].wrapping_add(a.partial_phase_increment[i]);
                let mut partial = interpolate824(&WAV_SINE, a.partial_phase[i]) as i32;
                let amplitude = a.partial_amplitude[i].wrapping_add((a.target_partial_amplitude[i] - a.partial_amplitude[i]).wrapping_mul(fade) >> 15);
                partial = partial.wrapping_mul(amplitude) >> 16;
                harmonics = harmonics.wrapping_add(partial);
                partials[i] = partial;
            }
            let mut sample = partials[0];
            let noise_mode_1 = partials[1].wrapping_mul(lp_state_2) >> 8;
            let noise_mode_2 = partials[3].wrapping_mul(lp_state_2) >> 9;
            sample = sample.wrapping_add(noise_mode_1.wrapping_mul(12288 - noise_mode_gain) >> 14);
            sample = sample.wrapping_add(noise_mode_2.wrapping_mul(noise_mode_gain) >> 14);
            sample = sample.wrapping_add(harmonics.wrapping_mul(harmonics_gain) >> 14);
            sample = clip(sample);
            buffer[k] = ((sample + previous_sample) >> 1) as i16;
            buffer[k + 1] = sample as i16;
            previous_sample = sample;
            k += 2;
        }
        self.state.add.previous_sample = previous_sample as i16;
        self.state.add.lp_noise = [lp_state_0, lp_state_1, lp_state_2];
        for i in 0..K_NUM_BELL_PARTIALS {
            self.state.add.partial_amplitude[i] = self.state.add.target_partial_amplitude[i];
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::digital::DigitalOscillator;

    /// Renders `blocks` blocks of 24 samples, striking at the start and halfway.
    fn render(shape: usize, note: i16, p: (i16, i16), blocks: usize) -> Vec<i16> {
        let mut d = DigitalOscillator::new_boxed();
        let sync = [0u8; 24];
        let mut out = Vec::with_capacity(blocks * 24);
        let mut buf = [0i16; 24];
        for b in 0..blocks {
            if b == 0 || b == blocks / 2 {
                d.strike();
            }
            d.set_shape(shape);
            d.set_pitch(note << 7);
            d.set_parameters(p.0, p.1);
            d.render(&sync, &mut buf);
            out.extend_from_slice(&buf);
        }
        out
    }

    #[test]
    fn part_a_shapes_sound() {
        // fn_table_ positions: 0..=14 and 19, 20 (2 = comb filter needs an input).
        for shape in (0..=14).chain([19, 20]) {
            if shape == 2 {
                continue;
            }
            for note in [36, 60, 84] {
                for p in [(0, 0), (16384, 16384), (32767, 32767), (32767, 0)] {
                    let out = render(shape, note, p, 4000);
                    let peak = out.iter().map(|s| (*s as i32).abs()).max().unwrap();
                    assert!(peak > 100, "shape {shape} note {note} params {p:?}: peak {peak}");
                }
            }
        }
    }

    #[test]
    fn fm_is_in_tune() {
        // With no modulation the FM carrier is a plain sine.
        let out = render(12, 69, (0, 16384), 4000);
        let ups = out.windows(2).filter(|w| w[0] < 0 && w[1] >= 0).count() as f32;
        assert!((ups - 440.0).abs() < 13.0, "FM carrier at {ups} Hz");
    }

    #[test]
    fn comb_filters_an_input() {
        let mut d = DigitalOscillator::new_boxed();
        let sync = [0u8; 24];
        let mut buf = [0i16; 24];
        let mut peak = 0;
        for b in 0..2000 {
            for (i, s) in buf.iter_mut().enumerate() {
                *s = if (b * 24 + i) % 200 < 100 { 8000 } else { -8000 };
            }
            d.set_shape(2);
            d.set_pitch(60 << 7);
            d.set_parameters(16384, 30000);
            d.render(&sync, &mut buf);
            peak = peak.max(buf.iter().map(|s| (*s as i32).abs()).max().unwrap());
        }
        assert!(peak > 1000);
    }
}

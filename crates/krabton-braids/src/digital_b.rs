//! Digital oscillator render functions, part B: physical models (plucked,
//! bowed, blown, fluted), wavetables, noises, granular cloud, particle
//! noise, QPSK, the morse easter egg and the kick, snare and cymbal.
//!
//! Port of `braids/digital_oscillator.cc` (Copyright 2012 Emilie Gillet,
//! MIT). Integer arithmetic follows the C++ code.

use crate::digital::{DigitalOscillator, DigitalOscillatorState, K_NUM_PLUCK_VOICES, K_WG_BORE_LENGTH, K_WG_BRIDGE_LENGTH, K_WG_FBORE_LENGTH, K_WG_JET_LENGTH, K_WG_NECK_LENGTH};
use crate::dsp::{clip, crossfade_u8, interpolate1022, interpolate824, interpolate824_u16, interpolate88, mix, random_sample, random_word, udiv};
use crate::resources::{
    LUT_BLOWING_ENVELOPE, LUT_BLOWING_JET, LUT_BOWING_ENVELOPE, LUT_BOWING_FRICTION, LUT_FLUTE_BODY_FILTER, LUT_GRANULAR_ENVELOPE, LUT_GRANULAR_ENVELOPE_RATE,
    LUT_RESONATOR_COEFFICIENT, LUT_RESONATOR_SCALE, LUT_SVF_CUTOFF, LUT_SVF_DAMP, LUT_SVF_SCALE, WAV_SINE, WS_MODERATE_OVERDRIVE, WT_CODE, WT_MAP, WT_WAVES,
};
use crate::svf::SvfMode;

#[inline(always)]
fn sync_at(sync: &[u8], i: usize) -> bool {
    sync.get(i).copied().unwrap_or(0) != 0
}

/// `ptr % len` for a C expression like `(uint16_ptr - 1) % len`, which is
/// negative (undefined) when the pointer is 0; wrap it instead.
#[inline(always)]
fn wrap_index(v: i32, len: usize) -> usize {
    v.rem_euclid(len as i32) as usize
}

/// A 129-byte wave of `wt_waves`.
#[inline(always)]
fn wave(index: usize) -> &'static [u8] {
    let i = index.min(WT_WAVES.len() / 129 - 1);
    &WT_WAVES[i * 129..i * 129 + 129]
}

const K_BRIDGE_LP_GAIN: i32 = 14008;
const K_BRIDGE_LP_POLE1: i32 = 18022;
const K_BIQUAD_GAIN: i32 = 6553;
const K_BIQUAD_POLE1: i32 = 6948;
const K_BIQUAD_POLE2: i32 = -2959;

const K_BREATH_PRESSURE: i32 = 26214;
const K_REFLECTION_COEFFICIENT: i32 = -3891;
const K_REED_SLOPE: i32 = -1229;
const K_REED_OFFSET: i32 = 22938;

/// `0.99 * 4096` stored in a `uint16_t`.
const K_DC_BLOCKING_POLE: i32 = 4055;
const LUT_BOWING_ENVELOPE_SIZE: u16 = 752;
const LUT_BLOWING_ENVELOPE_SIZE: u16 = 392;

struct WavetableDefinition {
    num_steps: u8,
    wave_index: [u8; 17],
}

const fn wt(num_steps: u8, w: [u8; 17]) -> WavetableDefinition {
    WavetableDefinition { num_steps, wave_index: w }
}

static WAVETABLE_DEFINITIONS: [WavetableDefinition; 20] = [
    // 01 male
    wt(16, [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 15]),
    // 02 female
    wt(16, [16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 31]),
    // 03 choir
    wt(16, [32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 47]),
    // 04 space voice
    wt(16, [48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 63]),
    // 05 tampura
    wt(16, [64, 65, 66, 67, 68, 68, 69, 70, 71, 72, 73, 73, 74, 75, 75, 76, 76]),
    // 06 shamus
    wt(16, [77, 78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88, 89, 90, 91, 92, 92]),
    // 07 swept string
    wt(16, [93, 94, 95, 96, 97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 108, 108]),
    // 08 bowed
    wt(16, [109, 110, 111, 112, 113, 114, 115, 116, 117, 118, 119, 120, 121, 122, 123, 124, 124]),
    // 09 cello
    wt(16, [125, 126, 127, 128, 129, 130, 131, 132, 132, 132, 132, 132, 132, 132, 132, 132, 132]),
    // 10 vibes
    wt(16, [133, 134, 135, 136, 137, 138, 139, 140, 141, 142, 143, 144, 144, 144, 145, 145, 145]),
    // 11 slap
    wt(16, [146, 147, 148, 149, 150, 151, 151, 151, 152, 152, 152, 152, 153, 153, 153, 153, 153]),
    // 12 piano
    wt(8, [154, 154, 154, 154, 154, 154, 155, 156, 156, 0, 0, 0, 0, 0, 0, 0, 0]),
    // 13 organ
    wt(16, [176, 157, 158, 159, 160, 161, 162, 163, 164, 165, 166, 167, 168, 169, 170, 171, 171]),
    // 14 waves
    wt(16, [172, 173, 174, 175, 176, 177, 178, 179, 180, 181, 182, 183, 184, 185, 186, 187, 187]),
    // 15 digital
    wt(16, [176, 188, 189, 190, 191, 192, 193, 194, 195, 196, 197, 198, 199, 200, 201, 202, 202]),
    // 16 drone 1
    wt(16, [203, 205, 204, 205, 212, 206, 207, 208, 208, 209, 210, 210, 211, 211, 212, 212, 212]),
    // 17 drone 2
    wt(8, [213, 213, 213, 214, 215, 216, 217, 218, 219, 0, 0, 0, 0, 0, 0, 0, 0]),
    // 18 metallic
    wt(16, [220, 221, 222, 223, 224, 225, 226, 227, 228, 229, 230, 231, 232, 233, 234, 235, 235]),
    // 19 fantasy
    wt(16, [236, 237, 238, 239, 240, 241, 242, 243, 244, 245, 246, 247, 248, 249, 250, 251, 251]),
    // 20 bell
    wt(4, [252, 253, 254, 255, 254, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
];

static WAVE_LINE: [u8; 64] = [
    187, 179, 154, 155, 135, 134, 137, 19, 24, 3, 8, 66, 79, 25, 180, 174, 64, 127, 198, 15, 10, 7, 11, 0, 191, 192, 115, 238, 237, 236, 241, 47, 70, 76, 235, 26,
    133, 208, 34, 175, 183, 146, 147, 148, 150, 151, 152, 153, 117, 138, 32, 33, 35, 125, 199, 201, 30, 31, 193, 27, 29, 21, 18, 182,
];

static MINI_WAVE_LINE: [u8; 33] = [
    157, 161, 171, 188, 189, 191, 192, 193, 196, 198, 201, 234, 232, 229, 226, 224, 1, 2, 3, 4, 5, 8, 12, 32, 36, 42, 47, 252, 254, 141, 139, 135, 174,
];

const SEMI: u16 = 128;

static CHORDS: [[u16; 3]; 17] = [
    [2, 4, 6],
    [16, 32, 48],
    [2 * SEMI, 7 * SEMI, 12 * SEMI],
    [3 * SEMI, 7 * SEMI, 10 * SEMI],
    [3 * SEMI, 7 * SEMI, 12 * SEMI],
    [3 * SEMI, 7 * SEMI, 14 * SEMI],
    [3 * SEMI, 7 * SEMI, 17 * SEMI],
    [7 * SEMI, 12 * SEMI, 19 * SEMI],
    // `3 + 12 SEMI` expands to `3 + 12 * 128` in the C macro.
    [7 * SEMI, 3 + 12 * SEMI, 5 + 19 * SEMI],
    [4 * SEMI, 7 * SEMI, 17 * SEMI],
    [4 * SEMI, 7 * SEMI, 14 * SEMI],
    [4 * SEMI, 7 * SEMI, 12 * SEMI],
    [4 * SEMI, 7 * SEMI, 11 * SEMI],
    [5 * SEMI, 7 * SEMI, 12 * SEMI],
    [4, 7 * SEMI, 12 * SEMI],
    [4, 4 + 12 * SEMI, 12 * SEMI],
    [4, 4 + 12 * SEMI, 12 * SEMI],
];

const K_PARTICLE_NOISE_DECAY: u32 = 64763;
/// `32768 * 0.996 * 0.996` and `32768 * 0.996`, truncated to int32.
const K_RESONANCE_SQUARED: i32 = 32507;
const K_RESONANCE_FACTOR: i32 = 32636;

static K_CONSTELLATION_Q: [i32; 4] = [23100, -23100, -23100, 23100];
static K_CONSTELLATION_I: [i32; 4] = [23100, 23100, -23100, -23100];

impl DigitalOscillator {
    pub(crate) fn render_plucked(&mut self, _sync: &[u8], buffer: &mut [i16]) {
        self.phase_increment <<= 1;
        if self.strike {
            self.active_voice += 1;
            if self.active_voice as usize >= K_NUM_PLUCK_VOICES {
                self.active_voice = 0;
            }
            // Find the optimal oversampling rate.
            let phase_increment = self.phase_increment;
            let width = (3 * self.parameter[1] as i32) >> 1;
            let p = &mut self.state.plk[self.active_voice as usize];
            let mut increment = phase_increment as i32;
            p.shift = 0;
            while increment > (2 << 22) {
                increment >>= 1;
                p.shift += 1;
            }
            p.size = 1024 >> p.shift;
            p.mask = p.size - 1;
            p.write_ptr = 0;
            p.max_phase_increment = phase_increment << 1;
            p.phase_increment = phase_increment;
            p.initialization_ptr = (p.size as i64 * (8192 + width) as i64 >> 16).max(0) as usize;
            self.strike = false;
        }

        // Update the phase increment of the latest note, but do not
        // transpose too high above the original pitch.
        let current = &mut self.state.plk[self.active_voice as usize];
        current.phase_increment = self.phase_increment.min(current.max_phase_increment);

        // Loss and stretching factors.
        let p0 = self.parameter[0] as i32;
        let update_probability: u32 = if p0 < 16384 { 65535 } else { (131072 - (p0 >> 3) * 31) as u32 };
        let mut loss = (4096 - (self.phase_increment >> 14) as i32) as i16 as i32;
        if loss < 256 {
            loss = 256;
        }
        if p0 < 16384 {
            loss = (loss * (16384 - p0) >> 14) as i16 as i32;
        } else {
            loss = 0;
        }

        let mut previous_sample = self.state.plk[0].previous_sample as i32;
        let n = buffer.len();
        let mut k = 0;
        while k + 1 < n {
            let mut sample: i32 = 0;
            for i in 0..K_NUM_PLUCK_VOICES {
                let p = &mut self.state.plk[i];
                let dl = &mut self.delay_lines.ks[i * 1025..i * 1025 + 1025];
                if p.initialization_ptr != 0 {
                    // Initialization: fill the delay line with white noise.
                    p.initialization_ptr -= 1;
                    let ip = p.initialization_ptr.min(1024);
                    let excitation_sample = (dl[ip] as i32 + 3 * random_sample() as i32) >> 2;
                    dl[ip] = excitation_sample as i16;
                    sample += excitation_sample;
                } else {
                    p.phase = p.phase.wrapping_add(p.phase_increment);
                    let read_ptr = (((p.phase >> (22 + p.shift).min(31)) as usize) + 2) & p.mask;
                    let mut write_ptr = p.write_ptr;
                    while write_ptr != read_ptr {
                        let next = (write_ptr + 1) & p.mask;
                        let a = dl[write_ptr] as i32;
                        let b = dl[next] as i32;
                        let probability = random_word();
                        if (probability & 0xffff) <= update_probability {
                            let mut sum = a + b;
                            sum = if sum < 0 { -(-sum >> 1) } else { sum >> 1 };
                            if loss != 0 {
                                sum = sum * (32768 - loss) >> 15;
                            }
                            dl[write_ptr] = sum as i16;
                        }
                        if write_ptr == 0 {
                            dl[p.size] = dl[0];
                        }
                        write_ptr = next;
                    }
                    p.write_ptr = write_ptr;
                    sample += interpolate1022(dl, p.phase.checked_shr(p.shift as u32).unwrap_or(0)) as i32;
                }
            }
            sample = clip(sample);
            buffer[k] = ((previous_sample + sample) >> 1) as i16;
            buffer[k + 1] = sample as i16;
            previous_sample = sample;
            k += 2;
        }
        self.state.plk[0].previous_sample = previous_sample as i16;
    }

    pub(crate) fn render_bowed(&mut self, _sync: &[u8], buffer: &mut [i16]) {
        if self.strike {
            self.delay_lines.bowed_bridge.fill(0);
            self.delay_lines.bowed_neck.fill(0);
            self.state = DigitalOscillatorState::default();
            self.strike = false;
        }
        let parameter_0 = (172 - (self.parameter[0] as i32 >> 8)) as i16 as i32;
        let parameter_1 = (6 + (self.parameter[1] as i32 >> 9)) as i16;

        let mut delay_ptr = self.state.phy.delay_ptr;
        let mut excitation_ptr = self.state.phy.excitation_ptr;
        let mut lp_state = self.state.phy.lp_state;
        let mut biquad_y0 = self.state.phy.filter_state[0];
        let mut biquad_y1 = self.state.phy.filter_state[1];

        // Delay times and interpolation coefficients.
        let mut delay = (self.delay >> 1).wrapping_sub(2 << 16); // compensation for the 1-pole delay
        let mut bridge_delay = (delay >> 8).wrapping_mul(parameter_1 as i32 as u32);
        // Transpose an octave up when the note is too low to fit the delays.
        while delay.wrapping_sub(bridge_delay) > ((K_WG_NECK_LENGTH as u32 - 1) << 16) || bridge_delay > ((K_WG_BRIDGE_LENGTH as u32 - 1) << 16) {
            delay >>= 1;
            bridge_delay >>= 1;
        }
        let bridge_delay_integral = (bridge_delay >> 16) as u16;
        let bridge_delay_fractional = (bridge_delay & 0xffff) as u16;
        let neck_delay = delay.wrapping_sub(bridge_delay);
        let neck_delay_integral = neck_delay >> 16;
        let neck_delay_fractional = (neck_delay & 0xffff) as u16;
        let mut previous_sample = self.state.phy.previous_sample as i32;

        let dl_b = &mut self.delay_lines.bowed_bridge;
        let dl_n = &mut self.delay_lines.bowed_neck;
        let n = buffer.len();
        let mut k = 0;
        // Rendered at half the sample rate (the body filter coefficients
        // would round badly otherwise).
        while k + 1 < n {
            self.phase = self.phase.wrapping_add(self.phase_increment);
            let bridge_delay_ptr = (delay_ptr as u32 + 2 * K_WG_BRIDGE_LENGTH as u32).wrapping_sub(bridge_delay_integral as u32) as u16;
            let neck_delay_ptr = (delay_ptr as u32 + 2 * K_WG_NECK_LENGTH as u32).wrapping_sub(neck_delay_integral) as u16;
            let bridge_dl_a = dl_b[bridge_delay_ptr as usize % K_WG_BRIDGE_LENGTH] as i16;
            let bridge_dl_b = dl_b[wrap_index(bridge_delay_ptr as i32 - 1, K_WG_BRIDGE_LENGTH)] as i16;
            let nut_dl_a = dl_n[neck_delay_ptr as usize % K_WG_NECK_LENGTH] as i16;
            let nut_dl_b = dl_n[wrap_index(neck_delay_ptr as i32 - 1, K_WG_NECK_LENGTH)] as i16;
            let bridge_value = (mix(bridge_dl_a, bridge_dl_b, bridge_delay_fractional) as i32) << 8;
            let nut_value = (mix(nut_dl_a, nut_dl_b, neck_delay_fractional) as i32) << 8;
            lp_state = (bridge_value * K_BRIDGE_LP_GAIN).wrapping_add(lp_state.wrapping_mul(K_BRIDGE_LP_POLE1)) >> 15;
            let bridge_reflection = -lp_state;
            let nut_reflection = -nut_value;
            let string_velocity = bridge_reflection + nut_reflection;
            let e = excitation_ptr as usize;
            let mut bow_velocity = LUT_BOWING_ENVELOPE[(e >> 1).min(LUT_BOWING_ENVELOPE.len() - 1)] as i32;
            bow_velocity += LUT_BOWING_ENVELOPE[((e + 1) >> 1).min(LUT_BOWING_ENVELOPE.len() - 1)] as i32;
            bow_velocity >>= 1;
            let velocity_delta = bow_velocity - string_velocity;

            let mut friction = velocity_delta.wrapping_mul(parameter_0) >> 5;
            if friction < 0 {
                friction = -friction;
            }
            if friction >= (1 << 17) {
                friction = (1 << 17) - 1;
            }
            friction = LUT_BOWING_FRICTION[(friction >> 9) as usize] as i32;
            let new_velocity = friction.wrapping_mul(velocity_delta) >> 15;
            dl_n[delay_ptr as usize % K_WG_NECK_LENGTH] = ((bridge_reflection + new_velocity) >> 8) as i8;
            dl_b[delay_ptr as usize % K_WG_BRIDGE_LENGTH] = ((nut_reflection + new_velocity) >> 8) as i8;
            delay_ptr = delay_ptr.wrapping_add(1);

            let mut temp = bridge_value.wrapping_mul(K_BIQUAD_GAIN) >> 15;
            temp += biquad_y0.wrapping_mul(K_BIQUAD_POLE1) >> 12;
            temp += biquad_y1.wrapping_mul(K_BIQUAD_POLE2) >> 12;
            let mut out = temp - biquad_y1;
            biquad_y1 = biquad_y0;
            biquad_y0 = temp;

            out = clip(out);
            buffer[k] = ((out + previous_sample) >> 1) as i16;
            buffer[k + 1] = out as i16;
            previous_sample = out;
            excitation_ptr = excitation_ptr.wrapping_add(1);
            k += 2;
        }
        if (excitation_ptr >> 1) >= LUT_BOWING_ENVELOPE_SIZE - 32 {
            excitation_ptr = (LUT_BOWING_ENVELOPE_SIZE - 32) << 1;
        }
        self.state.phy.delay_ptr = delay_ptr % K_WG_NECK_LENGTH as u16;
        self.state.phy.excitation_ptr = excitation_ptr;
        self.state.phy.lp_state = lp_state;
        self.state.phy.filter_state = [biquad_y0, biquad_y1];
        self.state.phy.previous_sample = previous_sample as i16;
    }

    pub(crate) fn render_blown(&mut self, _sync: &[u8], buffer: &mut [i16]) {
        let mut delay_ptr = self.state.phy.delay_ptr;
        let mut lp_state = self.state.phy.lp_state;
        if self.strike {
            self.delay_lines.bore.fill(0);
            self.strike = false;
        }
        let mut delay = (self.delay >> 1).wrapping_sub(1 << 16);
        while delay > ((K_WG_BORE_LENGTH as u32 - 1) << 16) {
            delay >>= 1;
        }
        let bore_delay_integral = (delay >> 16) as u16;
        let bore_delay_fractional = (delay & 0xffff) as u16;
        let parameter = (28000 - (self.parameter[0] as i32 >> 1)) as u16 as i32;
        let mut filter_state = self.state.phy.filter_state[0] as i16;
        let normalized_pitch = (((self.pitch as i32 - 8192 + (self.parameter[1] as i32 >> 1)) >> 7) as i16).clamp(0, 127);
        let filter_coefficient = LUT_FLUTE_BODY_FILTER[normalized_pitch as usize] as i32;
        let dl = &mut self.delay_lines.bore;
        for out_sample in buffer.iter_mut() {
            self.phase = self.phase.wrapping_add(self.phase_increment);
            let mut breath_pressure = (random_sample() as i32).wrapping_mul(parameter) >> 15;
            breath_pressure = breath_pressure * K_BREATH_PRESSURE >> 15;
            breath_pressure += K_BREATH_PRESSURE;

            let bore_delay_ptr = (delay_ptr as u32 + 2 * K_WG_BORE_LENGTH as u32).wrapping_sub(bore_delay_integral as u32) as u16;
            let dl_a = dl[bore_delay_ptr as usize % K_WG_BORE_LENGTH];
            let dl_b = dl[wrap_index(bore_delay_ptr as i32 - 1, K_WG_BORE_LENGTH)];
            let dl_value = mix(dl_a, dl_b, bore_delay_fractional) as i32;

            let mut pressure_delta = (dl_value >> 1) + lp_state;
            lp_state = dl_value >> 1;
            pressure_delta = K_REFLECTION_COEFFICIENT.wrapping_mul(pressure_delta) >> 12;
            pressure_delta -= breath_pressure;
            let mut reed = (pressure_delta.wrapping_mul(K_REED_SLOPE) >> 12) + K_REED_OFFSET;
            reed = clip(reed);
            let mut out = pressure_delta.wrapping_mul(reed) >> 15;
            out += breath_pressure;
            out = clip(out);
            dl[delay_ptr as usize % K_WG_BORE_LENGTH] = out as i16;
            delay_ptr = delay_ptr.wrapping_add(1);
            filter_state = ((filter_coefficient * out + (4096 - filter_coefficient) * filter_state as i32) >> 12) as i16;
            *out_sample = filter_state;
        }
        self.state.phy.filter_state[0] = filter_state as i32;
        self.state.phy.delay_ptr = delay_ptr % K_WG_BORE_LENGTH as u16;
        self.state.phy.lp_state = lp_state;
    }

    pub(crate) fn render_fluted(&mut self, _sync: &[u8], buffer: &mut [i16]) {
        let mut delay_ptr = self.state.phy.delay_ptr;
        let mut excitation_ptr = self.state.phy.excitation_ptr;
        let mut lp_state = self.state.phy.lp_state;
        let mut dc_blocking_x0 = self.state.phy.filter_state[0];
        let mut dc_blocking_y0 = self.state.phy.filter_state[1];
        if self.strike {
            excitation_ptr = 0;
            self.delay_lines.fluted_bore.fill(0);
            self.delay_lines.fluted_jet.fill(0);
            lp_state = 0;
            self.strike = false;
        }

        let mut bore_delay = (self.delay << 1).wrapping_sub(2 << 16);
        let mut jet_delay = (bore_delay >> 8).wrapping_mul((48 + (self.parameter[1] as i32 >> 10)) as u32);
        bore_delay = bore_delay.wrapping_sub(jet_delay);
        while bore_delay > ((K_WG_FBORE_LENGTH as u32 - 1) << 16) || jet_delay > ((K_WG_JET_LENGTH as u32 - 1) << 16) {
            bore_delay >>= 1;
            jet_delay >>= 1;
        }
        let bore_delay_integral = (bore_delay >> 16) as u16;
        let bore_delay_fractional = (bore_delay & 0xffff) as u16;
        let jet_delay_integral = jet_delay >> 16;
        let jet_delay_fractional = (jet_delay & 0xffff) as u16;

        let breath_intensity = (2100 - (self.parameter[0] as i32 >> 4)) as u16 as i32;
        // The C++ indexes with `pitch_ >> 7`, which can pass the 128-entry
        // table for very high notes.
        let filter_coefficient = LUT_FLUTE_BODY_FILTER[((self.pitch as i32 >> 7).clamp(0, 127)) as usize] as i32;
        let dl_b = &mut self.delay_lines.fluted_bore;
        let dl_j = &mut self.delay_lines.fluted_jet;
        let n = buffer.len();
        for (idx, out_sample) in buffer.iter_mut().enumerate() {
            let size_after = n - 1 - idx; // `size` after `while (size--)`
            self.phase = self.phase.wrapping_add(self.phase_increment);

            let bore_delay_ptr = (delay_ptr as u32 + 2 * K_WG_FBORE_LENGTH as u32).wrapping_sub(bore_delay_integral as u32) as u16;
            let jet_delay_ptr = (delay_ptr as u32 + 2 * K_WG_JET_LENGTH as u32).wrapping_sub(jet_delay_integral) as u16;
            let bore_dl_a = dl_b[bore_delay_ptr as usize % K_WG_FBORE_LENGTH] as i16;
            let bore_dl_b = dl_b[wrap_index(bore_delay_ptr as i32 - 1, K_WG_FBORE_LENGTH)] as i16;
            let jet_dl_a = dl_j[jet_delay_ptr as usize % K_WG_JET_LENGTH] as i16;
            let jet_dl_b = dl_j[wrap_index(jet_delay_ptr as i32 - 1, K_WG_JET_LENGTH)] as i16;
            let bore_value = (mix(bore_dl_a, bore_dl_b, bore_delay_fractional) as i32) << 9;
            let jet_value = (mix(jet_dl_a, jet_dl_b, jet_delay_fractional) as i32) << 9;

            let mut breath_pressure = LUT_BLOWING_ENVELOPE[(excitation_ptr as usize).min(LUT_BLOWING_ENVELOPE.len() - 1)] as i32;
            breath_pressure <<= 1;
            let mut random_pressure = (random_sample() as i32).wrapping_mul(breath_intensity) >> 12;
            random_pressure = random_pressure.wrapping_mul(breath_pressure) >> 15;
            breath_pressure += random_pressure;

            lp_state = ((-filter_coefficient).wrapping_mul(bore_value).wrapping_add((4096 - filter_coefficient).wrapping_mul(lp_state))) >> 12;
            let mut reflection = lp_state;
            dc_blocking_y0 = K_DC_BLOCKING_POLE.wrapping_mul(dc_blocking_y0) >> 12;
            dc_blocking_y0 += reflection - dc_blocking_x0;
            dc_blocking_x0 = reflection;
            reflection = dc_blocking_y0;

            let mut pressure_delta = breath_pressure - (reflection >> 1);
            dl_j[delay_ptr as usize % K_WG_JET_LENGTH] = (pressure_delta >> 9) as i8;

            pressure_delta = jet_value;
            let jet_table_index = pressure_delta.clamp(0, 65535);
            pressure_delta = LUT_BLOWING_JET[(jet_table_index >> 8) as usize] as i32 + (reflection >> 1);
            dl_b[delay_ptr as usize % K_WG_FBORE_LENGTH] = (pressure_delta >> 9) as i8;
            delay_ptr = delay_ptr.wrapping_add(1);

            *out_sample = clip(bore_value >> 1) as i16;
            if size_after & 3 != 0 {
                excitation_ptr = excitation_ptr.wrapping_add(1);
            }
        }
        if excitation_ptr >= LUT_BLOWING_ENVELOPE_SIZE - 32 {
            excitation_ptr = LUT_BLOWING_ENVELOPE_SIZE - 32;
        }
        self.state.phy.delay_ptr = delay_ptr;
        self.state.phy.excitation_ptr = excitation_ptr;
        self.state.phy.lp_state = lp_state;
        self.state.phy.filter_state = [dc_blocking_x0, dc_blocking_y0];
    }

    pub(crate) fn render_wavetables(&mut self, sync: &[u8], buffer: &mut [i16]) {
        // Hysteresis on the second parameter, so a single bit of noise cannot
        // cause a sharp wavetable switch.
        let p1 = self.parameter[1] as i32;
        let prev1 = self.previous_parameter[1] as i32;
        if p1 > prev1 + 64 || p1 < prev1 - 64 {
            self.previous_parameter[1] = self.parameter[1];
        }
        let wavetable_index = (((self.previous_parameter[1] as i32 as u32).wrapping_mul(20)) >> 15).min(19) as usize;
        let wt = &WAVETABLE_DEFINITIONS[wavetable_index];
        let wave_pointer = (((self.parameter[0] as i32) << 1) as u32).wrapping_mul(wt.num_steps as u32);
        let base = ((wave_pointer >> 16) as usize).min(15);
        let wave_0 = wave(wt.wave_index[base] as usize);
        let wave_1 = wave(wt.wave_index[base + 1] as usize);
        let phase_increment = self.phase_increment >> 1;
        for (i, out) in buffer.iter_mut().enumerate() {
            // 2x naive oversampling.
            self.phase = self.phase.wrapping_add(phase_increment);
            if sync_at(sync, i) {
                self.phase = 0;
            }
            let mut sample = crossfade_u8(wave_0, wave_1, self.phase >> 1, wave_pointer as u16) >> 1;
            self.phase = self.phase.wrapping_add(phase_increment);
            sample = (sample as i32 + (crossfade_u8(wave_0, wave_1, self.phase >> 1, wave_pointer as u16) >> 1) as i32) as i16;
            *out = sample;
        }
    }

    pub(crate) fn render_wave_map(&mut self, sync: &[u8], buffer: &mut [i16]) {
        // The grid is 16x16, so there are 15 interpolation squares.
        let p = [((self.parameter[0] as i32 * 15) >> 4) as u16, ((self.parameter[1] as i32 * 15) >> 4) as u16];
        let wave_xfade = [p[0] << 5, p[1] << 5];
        let wave_coordinate = [p[0] >> 11, p[1] >> 11];
        let mut waves: [[&[u8]; 2]; 2] = [[&[], &[]], [&[], &[]]];
        for i in 0..2 {
            for j in 0..2 {
                let wave_index = ((wave_coordinate[0] as usize + i) * 16 + (wave_coordinate[1] as usize + j)).min(255);
                waves[i][j] = wave(WT_MAP[wave_index] as usize);
            }
        }
        let phase_increment = self.phase_increment >> 1;
        for (i, out) in buffer.iter_mut().enumerate() {
            // 2x naive oversampling.
            self.phase = self.phase.wrapping_add(phase_increment);
            if sync_at(sync, i) {
                self.phase = 0;
            }
            let ph = self.phase >> 1;
            let mut sample = mix(crossfade_u8(waves[0][0], waves[0][1], ph, wave_xfade[1]), crossfade_u8(waves[1][0], waves[1][1], ph, wave_xfade[1]), wave_xfade[0]) >> 1;
            self.phase = self.phase.wrapping_add(phase_increment);
            let ph = self.phase >> 1;
            sample = (sample as i32
                + (mix(crossfade_u8(waves[0][0], waves[0][1], ph, wave_xfade[1]), crossfade_u8(waves[1][0], waves[1][1], ph, wave_xfade[1]), wave_xfade[0]) >> 1) as i32)
                as i16;
            *out = sample;
        }
    }

    pub(crate) fn render_wave_line(&mut self, sync: &[u8], buffer: &mut [i16]) {
        self.smoothed_parameter = (3 * self.smoothed_parameter + ((self.parameter[0] as i32) << 1)) >> 2;
        let scan = self.smoothed_parameter as u16;
        let wave_0 = wave(WAVE_LINE[((self.previous_parameter[0] >> 9).max(0) as usize).min(63)] as usize);
        let wave_1 = wave(WAVE_LINE[((scan >> 10) as usize).min(63)] as usize);
        // The C++ reads one past the end of `wave_line` at the very top.
        let wave_2 = wave(WAVE_LINE[((scan >> 10) as usize + 1).min(63)] as usize);

        let smooth_xfade = scan << 6;
        let mut rough_xfade: u16 = 0;
        let rough_xfade_increment = (32768 / buffer.len().max(1)) as u16;
        let balance = ((self.parameter[1] as i32) << 3) as u16;
        let mut phase = self.phase;
        let phase_increment = self.phase_increment >> 1;
        let p1 = self.parameter[1];

        for (i, out) in buffer.iter_mut().enumerate() {
            if sync_at(sync, i) {
                phase = 0;
            }
            let mut sample: i32 = 0;
            for _ in 0..2 {
                let (a, b) = if p1 < 8192 {
                    let rough = crossfade_u8(wave_0, wave_1, (phase >> 1) & 0xfe00_0000, rough_xfade);
                    let smooth = crossfade_u8(wave_0, wave_1, phase >> 1, rough_xfade);
                    (rough, smooth)
                } else if p1 < 16384 {
                    let rough = crossfade_u8(wave_0, wave_1, phase >> 1, rough_xfade);
                    let smooth = crossfade_u8(wave_1, wave_2, phase >> 1, smooth_xfade);
                    (rough, smooth)
                } else if p1 < 24576 {
                    let smooth = crossfade_u8(wave_1, wave_2, phase >> 1, smooth_xfade);
                    let rough = crossfade_u8(wave_1, wave_2, (phase >> 1) & 0xfe00_0000, smooth_xfade);
                    (smooth, rough)
                } else {
                    let smooth = crossfade_u8(wave_1, wave_2, (phase >> 1) & 0xfe00_0000, smooth_xfade);
                    let rough = crossfade_u8(wave_1, wave_2, (phase >> 1) & 0xf800_0000, smooth_xfade);
                    (smooth, rough)
                };
                sample += mix(a, b, balance) as i32;
                phase = phase.wrapping_add(phase_increment);
                if p1 < 16384 {
                    rough_xfade = rough_xfade.wrapping_add(rough_xfade_increment);
                }
            }
            *out = (sample >> 1) as i16;
        }
        self.phase = phase;
        self.previous_parameter[0] = (self.smoothed_parameter >> 1) as i16;
    }

    pub(crate) fn render_wave_paraphonic(&mut self, _sync: &[u8], buffer: &mut [i16]) {
        if self.strike {
            for i in 0..4 {
                self.state.saw.phase[i] = random_word();
            }
            self.strike = false;
        }
        let phase_increment_0 = self.phase_increment;
        let mut phase_0 = self.state.saw.phase[0];
        let mut phase_1 = self.state.saw.phase[1];
        let mut phase_2 = self.state.saw.phase[2];
        let mut phase_3 = self.state.saw.phase[3];

        let p1 = self.parameter[1] as i32;
        let chord_integral = ((p1 >> 11).max(0) as usize).min(15);
        let mut chord_fractional = (p1 << 5) as u16;
        if chord_fractional < 30720 {
            chord_fractional = 0;
        } else if chord_fractional >= 34816 {
            chord_fractional = 65535;
        } else {
            chord_fractional = (chord_fractional - 30720).wrapping_mul(16);
        }
        let mut phase_increment = [0u32; 3];
        for (i, inc) in phase_increment.iter_mut().enumerate() {
            let detune_1 = CHORDS[chord_integral][i] as i32;
            let detune_2 = CHORDS[chord_integral + 1][i] as i32;
            let detune = (detune_1 + ((detune_2 - detune_1) * chord_fractional as i32 >> 16)) as u16;
            *inc = self.compute_phase_increment((self.pitch as i32 + detune as i32) as i16);
        }

        let p0 = self.parameter[0] as i32;
        let wave_1 = wave(MINI_WAVE_LINE[((p0 >> 10).max(0) as usize).min(32)] as usize);
        let wave_2 = wave(MINI_WAVE_LINE[((p0 >> 10).max(0) as usize + 1).min(32)] as usize);
        let wave_xfade = (p0 << 6) as u16;

        let n = buffer.len();
        let mut k = 0;
        while k + 1 < n {
            for o in 0..2 {
                phase_0 = phase_0.wrapping_add(phase_increment_0);
                phase_1 = phase_1.wrapping_add(phase_increment[0]);
                phase_2 = phase_2.wrapping_add(phase_increment[1]);
                phase_3 = phase_3.wrapping_add(phase_increment[2]);
                let mut sample: i32 = 0;
                sample += crossfade_u8(wave_1, wave_2, phase_0 >> 1, wave_xfade) as i32;
                sample += crossfade_u8(wave_1, wave_2, phase_1 >> 1, wave_xfade) as i32;
                sample += crossfade_u8(wave_1, wave_2, phase_2 >> 1, wave_xfade) as i32;
                sample += crossfade_u8(wave_1, wave_2, phase_3 >> 1, wave_xfade) as i32;
                buffer[k + o] = (sample >> 2) as i16;
            }
            k += 2;
        }
        self.state.saw.phase[0] = phase_0;
        self.state.saw.phase[1] = phase_1;
        self.state.saw.phase[2] = phase_2;
        self.state.saw.phase[3] = phase_3;
    }

    pub(crate) fn render_filtered_noise(&mut self, _sync: &[u8], buffer: &mut [i16]) {
        let f = interpolate824_u16(&LUT_SVF_CUTOFF, (self.pitch as i32 as u32) << 17) as i32;
        let damp = interpolate824_u16(&LUT_SVF_DAMP, (self.parameter[0] as i32 as u32) << 17) as i32;
        let scale = interpolate824_u16(&LUT_SVF_SCALE, (self.parameter[0] as i32 as u32) << 17) as i32;
        let mut bp = self.state.svf.bp;
        let mut lp = self.state.svf.lp;
        let p1 = self.parameter[1] as i32;
        // Morph between LP, BP and HP.
        let (bp_gain, lp_gain, hp_gain) = if p1 < 16384 { (p1, 16384 - p1, 0) } else { (32767 - p1, 0, p1 - 16384) };
        let gain_correction = if f > scale { scale * 32767 / f } else { 32767 };
        for out in buffer.iter_mut() {
            let input = (random_sample() >> 1) as i32;
            let notch = input - (bp.wrapping_mul(damp) >> 15);
            lp = lp.wrapping_add(f.wrapping_mul(bp) >> 15);
            lp = clip(lp);
            let hp = notch - lp;
            bp = bp.wrapping_add(f.wrapping_mul(hp) >> 15);
            let mut result: i32 = 0;
            result += lp_gain.wrapping_mul(lp) >> 14;
            result += bp_gain.wrapping_mul(bp) >> 14;
            result += hp_gain.wrapping_mul(hp) >> 14;
            result = clip(result);
            result = result * gain_correction >> 15;
            *out = interpolate88(&WS_MODERATE_OVERDRIVE, (result + 32768) as u16);
        }
        self.state.svf.lp = lp;
        self.state.svf.bp = bp;
    }

    pub(crate) fn render_twin_peaks_noise(&mut self, _sync: &[u8], buffer: &mut [i16]) {
        let mut y11 = self.state.pno.filter_state[0][0];
        let mut y12 = self.state.pno.filter_state[0][1];
        let mut y21 = self.state.pno.filter_state[1][0];
        let mut y22 = self.state.pno.filter_state[1][1];
        let q = (65240 + (self.parameter[0] as i32 >> 7)) as u32;
        let q_squared = (q.wrapping_mul(q) >> 17) as i32;
        let p1 = (self.pitch as i32).clamp(0, 16383);
        let mut c1 = interpolate824_u16(&LUT_RESONATOR_COEFFICIENT, (p1 as u32) << 17) as i32;
        let s1 = interpolate824_u16(&LUT_RESONATOR_SCALE, (p1 as u32) << 17) as i32;
        let p2 = (((self.pitch as i32 + ((self.parameter[1] as i32 - 16384) >> 1)) as i16) as i32).clamp(0, 16383);
        let mut c2 = interpolate824_u16(&LUT_RESONATOR_COEFFICIENT, (p2 as u32) << 17) as i32;
        let s2 = interpolate824_u16(&LUT_RESONATOR_SCALE, (p2 as u32) << 17) as i32;
        c1 = ((c1 as u32).wrapping_mul(q) >> 16) as i32;
        c2 = ((c2 as u32).wrapping_mul(q) >> 16) as i32;
        let makeup_gain = 8191 - (self.parameter[0] as i32 >> 2);

        let n = buffer.len();
        let mut k = 0;
        while k + 1 < n {
            let sample = (random_sample() >> 1) as i32;
            let (mut y10, mut y20) = if sample > 0 { (sample * s1 >> 16, sample * s2 >> 16) } else { (-((-sample) * s1 >> 16), -((-sample) * s2 >> 16)) };
            y10 += y11.wrapping_mul(c1) >> 15;
            y10 -= y12.wrapping_mul(q_squared) >> 15;
            y10 = clip(y10);
            y12 = y11;
            y11 = y10;

            y20 += y21.wrapping_mul(c2) >> 15;
            y20 -= y22.wrapping_mul(q_squared) >> 15;
            y20 = clip(y20);
            y22 = y21;
            y21 = y20;

            y10 += y20;
            y10 += y10.wrapping_mul(makeup_gain) >> 13;
            y10 = clip(y10);
            let s = interpolate88(&WS_MODERATE_OVERDRIVE, (y10 + 32768) as u16);
            buffer[k] = s;
            buffer[k + 1] = s;
            k += 2;
        }
        self.state.pno.filter_state[0] = [y11, y12];
        self.state.pno.filter_state[1] = [y21, y22];
    }

    pub(crate) fn render_clocked_noise(&mut self, sync: &[u8], buffer: &mut [i16]) {
        let p1 = self.parameter[1] as i32;
        if p1 > self.previous_parameter[1] as i32 + 64 || p1 < self.previous_parameter[1] as i32 - 64 {
            self.previous_parameter[1] = self.parameter[1];
        }
        let p0 = self.parameter[0] as i32;
        if p0 > self.previous_parameter[0] as i32 + 16 || p0 < self.previous_parameter[0] as i32 - 16 {
            self.previous_parameter[0] = self.parameter[0];
        }
        if self.strike {
            self.state.clk.seed = random_word() as i32;
            self.strike = false;
        }

        // Shift the coarse range up to reach clock rates close to the sample rate.
        let mut phase = self.phase;
        let mut phase_increment = self.phase_increment;
        for _ in 0..3 {
            if phase_increment < (1u32 << 31) {
                phase_increment <<= 1;
            }
        }
        // Period of the random generator.
        self.state.clk.cycle_phase_increment = self.compute_phase_increment((self.previous_parameter[0] as i32 - 16384) as i16) << 1;
        // Number of quantization steps.
        let mut num_steps = (1 + (self.previous_parameter[1] as i32 >> 10)) as u32;
        if num_steps == 1 {
            num_steps = 2;
        }
        let quantizer_divider = udiv(65536, num_steps);
        let state = &mut self.state.clk;
        for (i, out) in buffer.iter_mut().enumerate() {
            phase = phase.wrapping_add(phase_increment);
            if sync_at(sync, i) {
                phase = 0;
            }
            // Clock.
            if phase < phase_increment {
                state.rng_state = state.rng_state.wrapping_mul(1664525).wrapping_add(1013904223);
                state.cycle_phase = state.cycle_phase.wrapping_add(state.cycle_phase_increment);
                // Enforce the period.
                if state.cycle_phase < state.cycle_phase_increment {
                    state.rng_state = state.seed as u32;
                    state.cycle_phase = state.cycle_phase_increment;
                }
                let mut sample = state.rng_state as u16;
                let qd = quantizer_divider as u16;
                if qd != 0 {
                    sample = sample.wrapping_sub(sample % qd);
                }
                sample = sample.wrapping_add(qd >> 1);
                state.sample = sample as i16;
                // Make the clock rate an exact divisor of the sample rate.
                phase = phase_increment;
            }
            *out = state.sample;
        }
        self.phase = phase;
    }

    pub(crate) fn render_granular_cloud(&mut self, _sync: &[u8], buffer: &mut [i16]) {
        let p0 = self.parameter[0] as i32;
        let p1 = self.parameter[1] as i32;
        for i in 0..4 {
            let phase_increment = self.phase_increment;
            let g = &mut self.state.grain[i];
            // A grain at the end of its envelope gets reset.
            if g.envelope_phase > (1 << 24) || g.envelope_phase_increment == 0 {
                g.envelope_phase_increment = 0;
                if (random_word() & 0xffff) < 0x4000 {
                    g.envelope_phase_increment = (LUT_GRANULAR_ENVELOPE_RATE[((p0 >> 7).max(0) as usize).min(256)] as u32) << 3;
                    g.envelope_phase = 0;
                    g.phase_increment = phase_increment;
                    let pitch_mod = (random_sample() as i32).wrapping_mul(p1) >> 16;
                    let phi = (phase_increment >> 8) as i32;
                    if pitch_mod < 0 {
                        g.phase_increment = g.phase_increment.wrapping_add(phi.wrapping_mul(pitch_mod >> 8) as u32);
                    } else {
                        g.phase_increment = g.phase_increment.wrapping_add(phi.wrapping_mul(pitch_mod >> 7) as u32);
                    }
                }
            }
        }
        let env_max = LUT_GRANULAR_ENVELOPE.len() - 1;
        for out in buffer.iter_mut() {
            let mut sample: i32 = 0;
            for g in self.state.grain.iter_mut() {
                g.phase = g.phase.wrapping_add(g.phase_increment);
                g.envelope_phase = g.envelope_phase.wrapping_add(g.envelope_phase_increment);
                let env = LUT_GRANULAR_ENVELOPE[((g.envelope_phase >> 16) as usize).min(env_max)] as i32;
                sample += (interpolate824(&WAV_SINE, g.phase) as i32).wrapping_mul(env) >> 17;
            }
            *out = sample.clamp(-32768, 32767) as i16;
        }
    }

    pub(crate) fn render_particle_noise(&mut self, _sync: &[u8], buffer: &mut [i16]) {
        let mut amplitude = self.state.pno.amplitude;
        let density = (1024 + self.parameter[0] as i32) as u32;
        let pno = self.state.pno;
        let [mut y11, mut y12] = pno.filter_state[0];
        let [mut y21, mut y22] = pno.filter_state[1];
        let [mut y31, mut y32] = pno.filter_state[2];
        let [mut s1, mut s2, mut s3] = pno.filter_scale;
        let [mut c1, mut c2, mut c3] = pno.filter_coefficient;
        let pitch = self.pitch as i32;
        let p1 = self.parameter[1] as i32;
        let resonator = |p: i32| -> (i32, i32) {
            let p = (p as i16 as i32).clamp(0, 16383) as u32;
            (interpolate824_u16(&LUT_RESONATOR_COEFFICIENT, p << 17) as i32, interpolate824_u16(&LUT_RESONATOR_SCALE, p << 17) as i32)
        };

        let n = buffer.len();
        let mut k = 0;
        while k + 1 < n {
            let noise = random_word();
            if (noise & 0x7f_ffff) < density {
                amplitude = 65535;
                let noise_a = ((noise & 0x0fff) as i32 - 0x800) as i16 as i32;
                let noise_b = (((noise >> 15) & 0x1fff) as i32 - 0x1000) as i16 as i32;
                (c1, s1) = resonator(pitch + (3 * noise_a * p1 >> 17) + 0x600);
                (c2, s2) = resonator(pitch + (noise_a * p1 >> 15) + 0x980);
                (c3, s3) = resonator(pitch + (noise_b * p1 >> 16) + 0x790);
                c1 = c1 * K_RESONANCE_FACTOR >> 15;
                c2 = c2 * K_RESONANCE_FACTOR >> 15;
                c3 = c3 * K_RESONANCE_FACTOR >> 15;
            }
            let sample = ((noise as i16 as i32) * amplitude as i32) >> 16;
            amplitude = ((amplitude as u32 * K_PARTICLE_NOISE_DECAY) >> 16) as u16;

            let (mut y10, mut y20, mut y30) = if sample > 0 {
                (sample * s1 >> 16, sample * s2 >> 16, sample * s3 >> 16)
            } else {
                (-((-sample) * s1 >> 16), -((-sample) * s2 >> 16), -((-sample) * s3 >> 16))
            };
            y10 += y11.wrapping_mul(c1) >> 15;
            y10 -= y12.wrapping_mul(K_RESONANCE_SQUARED) >> 15;
            y10 = clip(y10);
            y12 = y11;
            y11 = y10;

            y20 += y21.wrapping_mul(c2) >> 15;
            y20 -= y22.wrapping_mul(K_RESONANCE_SQUARED) >> 15;
            y20 = clip(y20);
            y22 = y21;
            y21 = y20;

            y30 += y31.wrapping_mul(c3) >> 15;
            y30 -= y32.wrapping_mul(K_RESONANCE_SQUARED) >> 15;
            y30 = clip(y30);
            y32 = y31;
            y31 = y30;

            y10 += y20 + y30;
            y10 = clip(y10);
            buffer[k] = y10 as i16;
            buffer[k + 1] = y10 as i16;
            k += 2;
        }
        let pno = &mut self.state.pno;
        pno.amplitude = amplitude;
        pno.filter_state = [[y11, y12], [y21, y22], [y31, y32]];
        pno.filter_scale = [s1, s2, s3];
        pno.filter_coefficient = [c1, c2, c3];
    }

    pub(crate) fn render_digital_modulation(&mut self, _sync: &[u8], buffer: &mut [i16]) {
        let mut phase = self.phase;
        let increment = self.phase_increment;
        let mut symbol_stream_phase = self.state.dmd.symbol_phase;
        let symbol_stream_phase_increment = self.compute_phase_increment((self.pitch as i32 - 1536 + ((self.parameter[0] as i32 - 32767) >> 3)) as i16);
        let mut data_byte = self.state.dmd.data_byte;
        if self.strike {
            self.state.dmd.symbol_count = 0;
            self.strike = false;
        }
        let p1 = self.parameter[1] as i32;
        for out in buffer.iter_mut() {
            phase = phase.wrapping_add(increment);
            symbol_stream_phase = symbol_stream_phase.wrapping_add(symbol_stream_phase_increment);
            if symbol_stream_phase < symbol_stream_phase_increment {
                let d = &mut self.state.dmd;
                d.symbol_count = d.symbol_count.wrapping_add(1);
                if d.symbol_count & 3 == 0 {
                    if d.symbol_count >= 64 + 4 * 256 {
                        d.symbol_count = 0;
                    }
                    if d.symbol_count < 32 {
                        data_byte = 0x00;
                    } else if d.symbol_count < 48 {
                        data_byte = 0x99;
                    } else if d.symbol_count < 64 {
                        data_byte = 0xcc;
                    } else {
                        d.filter_state = (d.filter_state * 3 + p1) >> 2;
                        data_byte = (d.filter_state >> 7) as u8;
                    }
                } else {
                    data_byte >>= 2;
                }
            }
            let i = interpolate824(&WAV_SINE, phase) as i32;
            let q = interpolate824(&WAV_SINE, phase.wrapping_add(1 << 30)) as i32;
            let s = (data_byte & 3) as usize;
            *out = ((K_CONSTELLATION_Q[s] * q >> 15) + (K_CONSTELLATION_I[s] * i >> 15)) as i16;
        }
        self.phase = phase;
        self.state.dmd.symbol_phase = symbol_stream_phase;
        self.state.dmd.data_byte = data_byte;
    }

    /// The hidden "?" shape: a morse code message over a noisy radio.
    pub(crate) fn render_question_mark(&mut self, _sync: &[u8], buffer: &mut [i16]) {
        if self.strike {
            let state = &mut self.state.clk;
            state.rng_state = 0;
            state.cycle_phase = 0;
            state.sample = 10;
            state.cycle_phase_increment = u32::MAX;
            state.seed = 32767;
            self.strike = false;
        }
        let mut phase = self.phase;
        let increment = self.phase_increment;
        let dit_duration = (3600 + ((32767 - self.parameter[0] as i32) >> 2)) as u32;
        let p1 = self.parameter[1] as i32;
        let noise_threshold = 1024 + (p1 >> 3);
        let state = &mut self.state.clk;
        for out in buffer.iter_mut() {
            phase = phase.wrapping_add(increment);
            let mut sample: i32 = if state.rng_state != 0 { (interpolate824(&WAV_SINE, phase) as i32 * 3) >> 2 } else { 0 };
            state.cycle_phase = state.cycle_phase.wrapping_add(1);
            if state.cycle_phase > dit_duration {
                state.sample = state.sample.wrapping_sub(1);
                if state.sample == 0 {
                    state.cycle_phase_increment = state.cycle_phase_increment.wrapping_add(1);
                    state.rng_state = (state.rng_state == 0) as u32;
                    let address = (state.cycle_phase_increment >> 2) as usize % WT_CODE.len();
                    let shift = (state.cycle_phase_increment & 0x3) << 1;
                    state.sample = ((2 << ((WT_CODE[address] >> shift) & 3)) - 1) as i16;
                    if state.sample == 15 {
                        state.sample = 100;
                        state.rng_state = 0;
                        state.cycle_phase_increment = u32::MAX;
                    }
                    phase = 1 << 30;
                }
                state.cycle_phase = 0;
            }
            state.seed = state.seed.wrapping_add((random_sample() >> 2) as i32);
            let mut noise_intensity = state.seed >> 8;
            if noise_intensity < 0 {
                noise_intensity = -noise_intensity;
            }
            noise_intensity = noise_intensity.max(noise_threshold).min(16000);
            let mut noise = (random_sample() as i32).wrapping_mul(noise_intensity) >> 15;
            noise = noise * WAV_SINE[((phase >> 22) & 0xff) as usize] as i32 >> 15;
            sample += noise;
            sample = clip(sample);
            let distorted = sample * sample >> 14;
            sample += distorted * p1 >> 15;
            sample = clip(sample);
            *out = sample as i16;
        }
        self.phase = phase;
    }

    pub(crate) fn render_kick(&mut self, _sync: &[u8], buffer: &mut [i16]) {
        if self.init {
            self.pulse[0].init();
            self.pulse[0].set_delay(0);
            self.pulse[0].set_decay(3340);
            self.pulse[1].init();
            self.pulse[1].set_delay(48); // 1 ms at 48 kHz, as in the original
            self.pulse[1].set_decay(3072);
            self.pulse[2].init();
            self.pulse[2].set_delay(192); // 4 ms at 48 kHz
            self.pulse[2].set_decay(4093);
            self.svf[0].init();
            self.svf[0].set_punch(32768);
            self.svf[0].set_mode(SvfMode::Bp);
            self.init = false;
        }
        if self.strike {
            self.strike = false;
            self.pulse[0].trigger(275251); // 12 * 32768 * 0.7
            self.pulse[1].trigger(-13763); // -19662 * 0.7
            self.pulse[2].trigger(18000);
            self.svf[0].set_punch(24000);
        }
        let decay = self.parameter[0] as i32 as u32;
        let mut scaled = 65535u32.wrapping_sub(decay << 1);
        let squared = scaled.wrapping_mul(scaled) >> 16;
        scaled = squared.wrapping_mul(scaled) >> 18;
        self.svf[0].set_resonance((32768 - 128 - scaled as i32) as i16);

        let mut coefficient = self.parameter[1] as i32 as u32;
        coefficient = coefficient.wrapping_mul(coefficient) >> 15;
        coefficient = coefficient.wrapping_mul(coefficient) >> 15;
        let lp_coefficient = 128 + (coefficient >> 1) as i32 * 3;
        let mut lp_state = self.state.svf.lp;

        let n = buffer.len();
        let mut k = 0;
        while k + 1 < n {
            let mut excitation: i32 = 0;
            excitation += self.pulse[0].process();
            excitation += if !self.pulse[1].done() { 16384 } else { 0 };
            excitation += self.pulse[1].process();
            self.pulse[2].process();
            let f = self.pitch as i32 + if self.pulse[2].done() { 0 } else { 17 << 7 };
            self.svf[0].set_frequency(f as i16);
            for j in 0..2 {
                let resonator_output = (excitation >> 4) + self.svf[0].process(excitation);
                lp_state = lp_state.wrapping_add((resonator_output - lp_state).wrapping_mul(lp_coefficient) >> 15);
                lp_state = clip(lp_state);
                buffer[k + j] = lp_state as i16;
            }
            k += 2;
        }
        self.state.svf.lp = lp_state;
    }

    pub(crate) fn render_snare(&mut self, _sync: &[u8], buffer: &mut [i16]) {
        if self.init {
            self.pulse[0].init();
            self.pulse[0].set_delay(0);
            self.pulse[0].set_decay(1536);
            self.pulse[1].init();
            self.pulse[1].set_delay(48);
            self.pulse[1].set_decay(3072);
            self.pulse[2].init();
            self.pulse[2].set_delay(48);
            self.pulse[2].set_decay(1200);
            self.pulse[3].init();
            self.pulse[3].set_delay(0);
            self.svf[0].init();
            self.svf[1].init();
            self.svf[2].init();
            self.svf[2].set_resonance(2000);
            self.svf[2].set_mode(SvfMode::Bp);
            self.init = false;
        }
        let p1 = self.parameter[1] as i32;
        if self.strike {
            let mut decay = 49152 - self.pitch as i32;
            decay += if p1 < 16384 { 0 } else { p1 - 16384 };
            if decay > 65535 {
                decay = 65535;
            }
            self.svf[0].set_resonance((29000 + (decay >> 5)) as i16);
            self.svf[1].set_resonance((26500 + (decay >> 5)) as i16);
            self.pulse[3].set_decay((4092 + (decay >> 14)) as u16);
            self.pulse[0].trigger(15 * 32768);
            self.pulse[1].trigger(-32768);
            self.pulse[2].trigger(13107);
            let snappy = p1.min(14336);
            self.pulse[3].trigger(512 + (snappy << 1));
            self.strike = false;
        }
        self.svf[0].set_frequency((self.pitch as i32 + (12 << 7)) as i16);
        self.svf[1].set_frequency((self.pitch as i32 + (24 << 7)) as i16);
        self.svf[2].set_frequency((self.pitch as i32 + (60 << 7)) as i16);
        let g_1 = 22000 - (self.parameter[0] as i32 >> 1);
        let g_2 = 22000 + (self.parameter[0] as i32 >> 1);

        let n = buffer.len();
        let mut k = 0;
        while k + 1 < n {
            let mut excitation_1: i32 = 0;
            excitation_1 += self.pulse[0].process();
            excitation_1 += self.pulse[1].process();
            excitation_1 += if !self.pulse[1].done() { 2621 } else { 0 };
            let mut excitation_2: i32 = 0;
            excitation_2 += self.pulse[2].process();
            excitation_2 += if !self.pulse[2].done() { 13107 } else { 0 };
            let noise_sample = (random_sample() as i32).wrapping_mul(self.pulse[3].process()) >> 15;

            let mut sd: i32 = 0;
            sd += (self.svf[0].process(excitation_1) + (excitation_1 >> 4)).wrapping_mul(g_1) >> 15;
            sd += (self.svf[1].process(excitation_2) + (excitation_2 >> 4)).wrapping_mul(g_2) >> 15;
            sd += self.svf[2].process(noise_sample);
            sd = clip(sd);
            buffer[k] = sd as i16;
            buffer[k + 1] = sd as i16;
            k += 2;
        }
    }

    pub(crate) fn render_cymbal(&mut self, _sync: &[u8], buffer: &mut [i16]) {
        if self.init {
            self.svf[0].init();
            self.svf[0].set_mode(SvfMode::Bp);
            self.svf[0].set_resonance(12000);
            self.svf[1].init();
            self.svf[1].set_mode(SvfMode::Hp);
            self.svf[1].set_resonance(2000);
            self.init = false;
        }
        let mut increments = [0u32; 7];
        let note = (40 << 7) + (self.pitch as i32 >> 1);
        increments[0] = self.compute_phase_increment(note as i16);
        let root = increments[0] >> 10;
        increments[1] = root.wrapping_mul(24273) >> 4;
        increments[2] = root.wrapping_mul(12561) >> 4;
        increments[3] = root.wrapping_mul(18417) >> 4;
        increments[4] = root.wrapping_mul(22452) >> 4;
        increments[5] = root.wrapping_mul(31858) >> 4;
        increments[6] = increments[0].wrapping_mul(24);

        let xfade = self.parameter[1] as i32;
        self.svf[0].set_frequency(self.parameter[0] >> 1);
        self.svf[1].set_frequency(self.parameter[0] >> 1);
        for out in buffer.iter_mut() {
            self.phase = self.phase.wrapping_add(increments[6]);
            let hat = &mut self.state.hat;
            if self.phase < increments[6] {
                hat.rng_state = hat.rng_state.wrapping_mul(1664525).wrapping_add(1013904223);
            }
            let mut hat_noise: i32 = 0;
            for i in 0..6 {
                hat.phase[i] = hat.phase[i].wrapping_add(increments[i]);
                hat_noise += (hat.phase[i] >> 31) as i32;
            }
            hat_noise -= 3;
            hat_noise *= 5461;
            let rng = hat.rng_state;
            hat_noise = clip(self.svf[0].process(hat_noise));
            let mut noise = ((rng >> 16) as i32) - 32768;
            noise = clip(self.svf[1].process(noise >> 1));
            *out = (hat_noise + ((noise - hat_noise).wrapping_mul(xfade) >> 15)) as i16;
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::digital::DigitalOscillator;

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
    fn part_b_shapes_sound() {
        for shape in (15..=18).chain(21..=34) {
            for note in [36, 60, 84] {
                for p in [(0, 0), (16384, 16384), (32767, 32767), (32767, 0)] {
                    let out = render(shape, note, p, 6000);
                    let peak = out.iter().map(|s| (*s as i32).abs()).max().unwrap();
                    assert!(peak > 100, "shape {shape} note {note} params {p:?}: peak {peak}");
                }
            }
        }
    }

    #[test]
    fn extreme_pitches_do_not_panic() {
        for shape in 0..=34 {
            for pitch in [i16::MIN, -2000, 0, 140 * 128, i16::MAX] {
                let mut d = DigitalOscillator::new_boxed();
                let mut buf = [0i16; 24];
                for _ in 0..200 {
                    d.set_shape(shape);
                    d.set_pitch(pitch);
                    d.set_parameters(32767, 32767);
                    d.render(&[1; 24], &mut buf);
                }
            }
        }
    }
}

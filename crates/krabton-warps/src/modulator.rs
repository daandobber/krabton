//! Warps' modulator: the input through a saturating amplifier with a
//! noise gate, the carrier from the internal oscillator, and either the
//! cross-modulation algorithms (crossfade, fold, analog and digital ring
//! modulation, XOR, comparator; neighbours blended, six times
//! oversampled) or the vocoder (`warps/dsp/modulator.cc`).
//!
//! Only the internal carrier is used here (Krabton has no second input to
//! give it), and not the easter egg.
//!
//! Copyright 2014 Emilie Gillet, MIT license (see the crate's `lib.rs`).

use crate::dsp::{clip16, interpolate, interpolate_centered, soft_clip, soft_limit, Down, Interp, Up};
use crate::oscillator::{Oscillator, Shape};
use crate::resources::{LUT_BIPOLAR_FOLD, LUT_XFADE_IN, LUT_XFADE_OUT, SRC_DOWN_6_48, SRC_UP_6_48};
use crate::vocoder::Vocoder;

pub const MAX_BLOCK: usize = 96;
const OVERSAMPLING: usize = 6;

/// The module's controls (all 0..1 but `note` and `carrier_shape`).
#[derive(Clone, Copy, Debug)]
pub struct Parameters {
    pub channel_drive: [f32; 2],
    pub modulation_algorithm: f32,
    pub modulation_parameter: f32,
    pub note: f32,
    /// 1 = sine, 2 = triangle, 3 = saw (the vocoder's carrier: saw, pulse, noise).
    pub carrier_shape: i32,
}

impl Default for Parameters {
    fn default() -> Self {
        Self { channel_drive: [0.0, 0.0], modulation_algorithm: 0.0, modulation_parameter: 0.0, note: 48.0, carrier_shape: 1 }
    }
}

impl Parameters {
    /// A non-linear response for the parameter of the algorithms between 1 and 4.
    fn skewed_modulation_parameter(&self) -> f32 {
        let a = self.modulation_algorithm;
        let skew = if a <= 1.0 {
            a
        } else if a >= 5.0 {
            1.0
        } else if a >= 4.0 {
            5.0 - a
        } else {
            0.0
        };
        let p = self.modulation_parameter;
        p * (1.0 + skew * (p - 1.0))
    }
}

struct SaturatingAmplifier {
    drive: f32,
    level: f32,
    post_gain: f32,
    pre_gain: f32,
}

impl SaturatingAmplifier {
    fn new() -> Self {
        Self { drive: 0.0, level: 0.0, post_gain: 0.0, pre_gain: 0.0 }
    }

    fn process(&mut self, drive: f32, limit: f32, input: &[f32], out: &mut [f32], out_raw: &mut [f32]) {
        let size = input.len();
        let mut drive_mod = Interp::new(&mut self.drive, drive, size);
        let mut level = self.level;
        for i in 0..size {
            let mut s = input[i];
            let error = s * s - level;
            level += error * if error > 0.0 { 0.1 } else { 0.0001 };
            s *= if level <= 0.0001 { (1.0 / 0.0001) * level } else { 1.0 };
            out[i] = s;
            out_raw[i] += s * drive_mod.next();
        }
        self.level = level;
        let d2 = drive * drive;
        let pre_a = drive * 0.5;
        let pre_b = d2 * d2 * drive * 24.0;
        let pre = pre_a + (pre_b - pre_a) * d2;
        let squished = drive * (2.0 - drive);
        let post = 1.0 / soft_clip(0.33 + squished * (pre - 0.33));
        let mut pre_mod = Interp::new(&mut self.pre_gain, pre, size);
        let mut post_mod = Interp::new(&mut self.post_gain, post, size);
        for o in out.iter_mut().take(size) {
            let p = pre_mod.next() * *o;
            let q = soft_clip(p) * post_mod.next();
            *o = p + (q - p) * limit;
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Xmod {
    Xfade,
    Fold,
    AnalogRing,
    DigitalRing,
    Xor,
    Comparator,
    Nop,
}

/// The algorithm pairs, blended by the algorithm's fraction.
const PAIRS: [(Xmod, Xmod); 6] = [(Xmod::Xfade, Xmod::Fold), (Xmod::Fold, Xmod::AnalogRing), (Xmod::AnalogRing, Xmod::DigitalRing), (Xmod::DigitalRing, Xmod::Xor), (Xmod::Xor, Xmod::Comparator), (Xmod::Comparator, Xmod::Nop)];

/// The diode non-linearity (Julian Parker, "A simple digital model of the diode-based ring modulator", DAFx-11).
#[inline]
fn diode(x: f32) -> f32 {
    let sign = if x > 0.0 { 1.0 } else { -1.0 };
    let mut dead = x.abs() - 0.667;
    dead += dead.abs();
    dead *= dead;
    0.043_247_658 * dead * sign
}

#[inline]
fn xmod(alg: Xmod, x_1: f32, x_2: f32, parameter: f32) -> f32 {
    match alg {
        Xmod::Xfade => {
            let fade_in = interpolate(&LUT_XFADE_IN, parameter, 256.0);
            let fade_out = interpolate(&LUT_XFADE_OUT, parameter, 256.0);
            x_1 * fade_in + x_2 * fade_out
        }
        Xmod::Fold => {
            let mut sum = x_1 + x_2 + x_1 * x_2 * 0.25;
            sum *= 0.02 + parameter;
            const SCALE: f32 = 2048.0 / ((1.0 + 1.0 + 0.25) * 1.02);
            interpolate_centered(&LUT_BIPOLAR_FOLD, 2048, sum, SCALE)
        }
        Xmod::AnalogRing => {
            let (modulator, carrier) = (x_1, x_2 * 2.0);
            let ring = (diode(modulator + carrier) + diode(modulator - carrier)) * (4.0 + parameter * 24.0);
            soft_limit(ring)
        }
        Xmod::DigitalRing => {
            let ring = 4.0 * x_1 * x_2 * (1.0 + parameter * 8.0);
            ring / (1.0 + ring.abs())
        }
        Xmod::Xor => {
            let a = clip16((x_1 * 32768.0) as i32);
            let b = clip16((x_2 * 32768.0) as i32);
            let m = (a ^ b) as f32 / 32768.0;
            let sum = (x_1 + x_2) * 0.7;
            sum + (m - sum) * parameter
        }
        Xmod::Comparator => {
            let (modulator, carrier) = (x_1, x_2);
            let x = parameter * 2.995;
            let xi = (x as usize).min(2);
            let xf = x - xi as f32;
            let direct = if modulator < carrier { modulator } else { carrier };
            let window = if modulator.abs() > carrier.abs() { modulator } else { carrier };
            let window_2 = if modulator.abs() > carrier.abs() { modulator.abs() } else { -carrier.abs() };
            let threshold = if carrier > 0.05 { carrier } else { modulator };
            let seq = [direct, threshold, window, window_2];
            seq[xi] + (seq[xi + 1] - seq[xi]) * xf
        }
        Xmod::Nop => x_1,
    }
}

pub struct Modulator {
    pub parameters: Parameters,
    previous: Parameters,
    amplifier: SaturatingAmplifier,
    xmod_oscillator: Oscillator,
    vocoder_oscillator: Oscillator,
    src_up: [Up; 2],
    src_down: Down,
    vocoder: Vocoder,
    internal_modulation: Vec<f32>,
    buffer: [Vec<f32>; 3],
    src_buffer: [Vec<f32>; 2],
}

impl Modulator {
    pub fn new(sample_rate: f32) -> Self {
        let previous = Parameters { channel_drive: [0.0, 0.0], modulation_algorithm: 0.0, modulation_parameter: 0.0, note: 48.0, carrier_shape: 0 };
        Self {
            parameters: Parameters::default(),
            previous,
            amplifier: SaturatingAmplifier::new(),
            xmod_oscillator: Oscillator::new(),
            vocoder_oscillator: Oscillator::new(),
            src_up: [Up::new(OVERSAMPLING, &SRC_UP_6_48), Up::new(OVERSAMPLING, &SRC_UP_6_48)],
            src_down: Down::new(OVERSAMPLING, &SRC_DOWN_6_48),
            vocoder: Vocoder::new(sample_rate),
            internal_modulation: vec![0.0; MAX_BLOCK],
            buffer: [vec![0.0; MAX_BLOCK], vec![0.0; MAX_BLOCK], vec![0.0; MAX_BLOCK]],
            src_buffer: [vec![0.0; MAX_BLOCK * OVERSAMPLING], vec![0.0; MAX_BLOCK * OVERSAMPLING]],
        }
    }

    /// Processes a block (at most 96 samples, a multiple of 12, at 96 kHz):
    /// `modulator` is the input; returns the main output in `out`.
    pub fn process(&mut self, modulator_in: &[f32], out: &mut [f32]) {
        let size = modulator_in.len().min(MAX_BLOCK);
        let p = self.parameters;
        let vocoder_amount = ((p.modulation_algorithm - 0.7) * 20.0 + 0.5).clamp(0.0, 1.0);
        let [carrier, modulator, aux] = &mut self.buffer;
        let (carrier, modulator, aux) = (&mut carrier[..size], &mut modulator[..size], &mut aux[..size]);
        aux.fill(0.0);
        self.amplifier.process(p.channel_drive[1], 1.0 - vocoder_amount, &modulator_in[..size], modulator, aux);
        let im = &mut self.internal_modulation[..size];
        im.fill(0.0);
        let xmod_shape = Shape::from_index(p.carrier_shape - 1);
        let vocoder_shape = Shape::from_index(p.carrier_shape + 1);
        const XMOD_CARRIER_GAIN: f32 = 0.5;
        if vocoder_amount == 0.0 {
            self.xmod_oscillator.render(xmod_shape, p.note, im, aux);
            for i in 0..size {
                carrier[i] = aux[i] * XMOD_CARRIER_GAIN;
            }
        } else if vocoder_amount >= 0.5 {
            let gain = self.vocoder_oscillator.render(vocoder_shape, p.note, im, aux);
            for i in 0..size {
                carrier[i] = aux[i] * gain;
            }
        } else {
            let balance = vocoder_amount * 2.0;
            self.xmod_oscillator.render(xmod_shape, p.note, im, carrier);
            let gain = self.vocoder_oscillator.render(vocoder_shape, p.note, im, aux);
            for i in 0..size {
                let (a, b) = (carrier[i], aux[i]);
                aux[i] = a + (b - a) * balance;
                carrier[i] = a * XMOD_CARRIER_GAIN + (b * gain - a * XMOD_CARRIER_GAIN) * balance;
            }
        }
        if vocoder_amount < 0.5 {
            let n = size * OVERSAMPLING;
            let [oc, om] = &mut self.src_buffer;
            self.src_up[0].process(carrier, &mut oc[..n]);
            self.src_up[1].process(modulator, &mut om[..n]);
            let algorithm = (p.modulation_algorithm * 8.0).min(5.999);
            let previous_algorithm = (self.previous.modulation_algorithm * 8.0).min(5.999);
            let (ai, af) = (algorithm as usize, algorithm.fract());
            let (pi, mut pf) = (previous_algorithm as usize, previous_algorithm.fract());
            if ai != pi {
                pf = af;
            }
            let (alg_a, alg_b) = PAIRS[ai.min(5)];
            let mut parameter = self.previous.skewed_modulation_parameter();
            let parameter_end = p.skewed_modulation_parameter();
            let mut balance = pf;
            let step = 1.0 / n as f32;
            let p_inc = (parameter_end - parameter) * step;
            let b_inc = (af - balance) * step;
            // (the output written over the carrier's oversampled buffer)
            for i in 0..n {
                let (x_1, x_2) = (om[i], oc[i]);
                let a = xmod(alg_a, x_1, x_2, parameter);
                let b = xmod(alg_b, x_1, x_2, parameter);
                oc[i] = a + (b - a) * balance;
                parameter += p_inc;
                balance += b_inc;
            }
            self.src_down.process(&oc[..n], &mut out[..size]);
        } else {
            let release = (4.0 * (p.modulation_algorithm - 0.75)).clamp(0.0, 1.0);
            self.vocoder.set_release_time(release * (2.0 - release));
            self.vocoder.set_formant_shift(p.modulation_parameter);
            self.vocoder.process(modulator, carrier, &mut out[..size]);
        }
        let transition = 2.0 * if vocoder_amount < 0.5 { vocoder_amount } else { 1.0 - vocoder_amount };
        if transition != 0.0 {
            for i in 0..size {
                out[i] += transition * (modulator[i] - out[i]);
            }
        }
        for o in out[..size].iter_mut() {
            *o = (*o).clamp(-1.0, 32767.0 / 32768.0);
        }
        self.previous = p;
    }
}

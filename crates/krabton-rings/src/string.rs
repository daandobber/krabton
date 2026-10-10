//! Comb filter / Karplus-Strong string with dispersion
//! (`rings/dsp/string.h`, `string.cc`).
//!
//! Copyright 2015 Emilie Gillet, MIT license. Rust port for Krabton.

use crate::dsp::{crossfade, interpolate, semitones_to_ratio, Rng, SAMPLE_RATE};
use crate::filter::{Approx, DcBlocker, DelayLine, Ramp, Svf};
use crate::resources::LUT_SVF_SHIFT;

pub const DELAY_LINE_SIZE: usize = 2048;

#[derive(Clone, Copy, Debug, Default)]
struct DampingFilter {
    x: f32,
    x_: f32,
    brightness: f32,
    brightness_increment: f32,
    damping: f32,
    damping_increment: f32,
}

impl DampingFilter {
    fn configure(&mut self, damping: f32, brightness: f32, size: usize) {
        if size == 0 {
            self.damping = damping;
            self.brightness = brightness;
            self.damping_increment = 0.0;
            self.brightness_increment = 0.0;
        } else {
            let step = 1.0 / size as f32;
            self.damping_increment = (damping - self.damping) * step;
            self.brightness_increment = (brightness - self.brightness) * step;
        }
    }

    #[inline]
    fn process(&mut self, x: f32) -> f32 {
        let h0 = (1.0 + self.brightness) * 0.5;
        let h1 = (1.0 - self.brightness) * 0.25;
        let y = self.damping * (h0 * self.x + h1 * (x + self.x_));
        self.x_ = self.x;
        self.x = x;
        self.brightness += self.brightness_increment;
        self.damping += self.damping_increment;
        y
    }
}

#[derive(Clone, Debug)]
pub struct KsString {
    frequency: f32,
    dispersion: f32,
    brightness: f32,
    damping: f32,
    position: f32,
    delay: f32,
    clamped_position: f32,
    previous_dispersion: f32,
    previous_damping_compensation: f32,
    enable_dispersion: bool,
    dispersion_noise: f32,
    src_phase: f32,
    out_sample: [f32; 2],
    aux_sample: [f32; 2],
    curved_bridge: f32,
    string: DelayLine<DELAY_LINE_SIZE>,
    stretch: DelayLine<{ DELAY_LINE_SIZE / 2 }>,
    fir_damping_filter: DampingFilter,
    iir_damping_filter: Svf,
    dc_blocker: DcBlocker,
    rng: Rng,
}

impl KsString {
    pub fn new(seed: u32) -> Self {
        let mut s = Self {
            frequency: 0.0,
            dispersion: 0.0,
            brightness: 0.0,
            damping: 0.0,
            position: 0.0,
            delay: 0.0,
            clamped_position: 0.0,
            previous_dispersion: 0.0,
            previous_damping_compensation: 0.0,
            enable_dispersion: false,
            dispersion_noise: 0.0,
            src_phase: 0.0,
            out_sample: [0.0; 2],
            aux_sample: [0.0; 2],
            curved_bridge: 0.0,
            string: DelayLine::default(),
            stretch: DelayLine::default(),
            fir_damping_filter: DampingFilter::default(),
            iir_damping_filter: Svf::default(),
            dc_blocker: DcBlocker::default(),
            rng: Rng::new(seed),
        };
        s.init(false);
        s
    }

    pub fn init(&mut self, enable_dispersion: bool) {
        self.enable_dispersion = enable_dispersion;
        self.string.reset();
        self.stretch.reset();
        self.fir_damping_filter = DampingFilter::default();
        self.iir_damping_filter = Svf::default();

        self.frequency = 220.0 / SAMPLE_RATE;
        self.dispersion = 0.25;
        self.brightness = 0.5;
        self.damping = 0.3;
        self.position = 0.8;

        self.delay = 1.0 / self.frequency;
        self.clamped_position = 0.0;
        self.previous_dispersion = 0.0;
        self.dispersion_noise = 0.0;
        self.curved_bridge = 0.0;
        self.previous_damping_compensation = 0.0;
        self.out_sample = [0.0; 2];
        self.aux_sample = [0.0; 2];
        self.dc_blocker = DcBlocker::new(1.0 - 20.0 / SAMPLE_RATE);
    }

    /// Glides towards `frequency` by `coefficient` (1: jumps).
    pub fn set_frequency(&mut self, frequency: f32, coefficient: f32) {
        self.frequency += coefficient * (frequency - self.frequency);
    }

    /// The dispersion, already mapped to -1..1 by the caller.
    pub fn set_dispersion(&mut self, dispersion: f32) {
        self.dispersion = dispersion;
    }

    pub fn set_brightness(&mut self, brightness: f32) {
        self.brightness = brightness;
    }

    pub fn set_damping(&mut self, damping: f32) {
        self.damping = damping;
    }

    pub fn set_position(&mut self, position: f32) {
        self.position = position;
    }

    /// Adds the string output to `out` and the comb-filtered pickup to `aux`.
    pub fn process(&mut self, input: &[f32], out: &mut [f32], aux: &mut [f32]) {
        let size = input.len();
        let mut delay = 1.0 / self.frequency;
        delay = delay.clamp(4.0, DELAY_LINE_SIZE as f32 - 4.0);

        // If there is not enough delay time in the delay line, we play at the
        // lowest possible note and we upsample on the fly with a shitty
        // linear interpolator. We don't care because it's a corner case
        // (f0 < 11.7Hz).
        let mut src_ratio = delay * self.frequency;
        if src_ratio >= 0.9999 {
            // When we are above 11.7 Hz, we make sure that the linear
            // interpolator does not get in the way.
            self.src_phase = 1.0;
            src_ratio = 1.0;
        }

        let clamped_position = 0.5 - 0.98 * (self.position - 0.5).abs();

        // Linearly interpolate all comb-related CV parameters for each sample.
        let mut delay_modulation = Ramp::new(self.delay, delay, size);
        let mut position_modulation = Ramp::new(self.clamped_position, clamped_position, size);
        let mut dispersion_modulation = Ramp::new(self.previous_dispersion, self.dispersion, size);

        // For damping/absorption, the interpolation is done in the filter
        // code.
        let lf_damping = self.damping * (2.0 - self.damping);
        let rt60 = 0.07 * semitones_to_ratio(lf_damping * 96.0) * SAMPLE_RATE;
        let rt60_base_2_12 = (-120.0 * delay / src_ratio / rt60).max(-127.0);
        let mut damping_coefficient = semitones_to_ratio(rt60_base_2_12);
        let mut brightness = self.brightness * self.brightness;
        let noise_filter = semitones_to_ratio((self.brightness - 1.0) * 48.0);
        let mut damping_cutoff =
            (24.0 + self.damping * self.damping * 48.0 + self.brightness * self.brightness * 24.0).min(84.0);
        let mut damping_f = (self.frequency * semitones_to_ratio(damping_cutoff)).min(0.499);

        // Crossfade to infinite decay.
        if self.damping >= 0.95 {
            let to_infinite = 20.0 * (self.damping - 0.95);
            damping_coefficient += to_infinite * (1.0 - damping_coefficient);
            brightness += to_infinite * (1.0 - brightness);
            damping_f += to_infinite * (0.4999 - damping_f);
            damping_cutoff += to_infinite * (128.0 - damping_cutoff);
        }

        self.fir_damping_filter.configure(damping_coefficient, brightness, size);
        self.iir_damping_filter.set_f_q(damping_f, 0.5, Approx::Accurate);
        let mut damping_compensation_modulation =
            Ramp::new(self.previous_damping_compensation, 1.0 - interpolate(&LUT_SVF_SHIFT, damping_cutoff, 1.0), size);

        for n in 0..size {
            self.src_phase += src_ratio;
            if self.src_phase > 1.0 {
                self.src_phase -= 1.0;

                let mut delay = delay_modulation.next();
                let comb_delay = delay * position_modulation.next();
                delay *= damping_compensation_modulation.next(); // IIR delay.
                delay -= 1.0; // FIR delay.

                let mut s;
                if self.enable_dispersion {
                    let noise = 2.0 * self.rng.float() - 1.0;
                    let noise = noise * (1.0 / (0.2 + noise_filter));
                    self.dispersion_noise += noise_filter * (noise - self.dispersion_noise);

                    let dispersion = dispersion_modulation.next();
                    let stretch_point = if dispersion <= 0.0 { 0.0 } else { dispersion * (2.0 - dispersion) * 0.475 };
                    let noise_amount = if dispersion > 0.75 { 4.0 * (dispersion - 0.75) } else { 0.0 };
                    let mut bridge_curving = if dispersion < 0.0 { -dispersion } else { 0.0 };

                    let noise_amount = noise_amount * noise_amount * 0.025;
                    let ac_blocking_amount = bridge_curving;

                    bridge_curving = bridge_curving * bridge_curving * 0.01;
                    let ap_gain = -0.618 * dispersion / (0.15 + dispersion.abs());

                    let mut delay_fm = 1.0;
                    delay_fm += self.dispersion_noise * noise_amount;
                    delay_fm -= self.curved_bridge * bridge_curving;
                    delay *= delay_fm;

                    let ap_delay = delay * stretch_point;
                    let main_delay = delay - ap_delay;
                    if ap_delay >= 4.0 && main_delay >= 4.0 {
                        s = self.string.read_hermite(main_delay);
                        s = self.stretch.allpass(s, ap_delay as usize, ap_gain);
                    } else {
                        s = self.string.read_hermite(delay);
                    }
                    let s_ac = self.dc_blocker.process_sample(s);
                    s += ac_blocking_amount * (s_ac - s);

                    let value = s.abs() - 0.025;
                    let sign = if s > 0.0 { 1.0 } else { -1.5 };
                    self.curved_bridge = (value.abs() + value) * sign;
                } else {
                    s = self.string.read_hermite(delay);
                }

                s += input[n]; // When f0 < 11.7 Hz, causes ugly bitcrushing on the input!
                s = self.fir_damping_filter.process(s);
                s = self.iir_damping_filter.process_lp(s);
                self.string.write(s);

                self.out_sample[1] = self.out_sample[0];
                self.aux_sample[1] = self.aux_sample[0];

                self.out_sample[0] = s;
                self.aux_sample[0] = self.string.read_linear(comb_delay);
            }
            out[n] += crossfade(self.out_sample[1], self.out_sample[0], self.src_phase);
            aux[n] += crossfade(self.aux_sample[1], self.aux_sample[0], self.src_phase);
        }
        self.delay = delay_modulation.value;
        self.clamped_position = position_modulation.value;
        self.previous_dispersion = dispersion_modulation.value;
        self.previous_damping_compensation = damping_compensation_modulation.value;
    }
}

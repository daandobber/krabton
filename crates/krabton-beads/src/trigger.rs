//! The random or periodic pulse train that seeds grains
//! (`beads/dsp/trigger_generator.cc`). DENSITY left of noon: regular
//! grains, slower towards noon (fully left they come at audio rate); right
//! of noon: random grains, more of them further right; around noon: only
//! the SEED button. With a clock on the
//! SEED input and the button latched, DENSITY divides the clock (left) or
//! lets each pulse through with a probability (right).
//!
//! Copyright 2017 Emilie Gillet, MIT license. Rust port for Krabton.

use crate::util::{extract_gate_flags, one_pole, semitones_to_ratio, GateFlags, HysteresisQuantizer2, Random, GATE_FLAG_HIGH, GATE_FLAG_LOW, GATE_FLAG_RISING};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SeedButtonState {
    Off,
    On,
    Locked,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct TriggerOutput {
    pub trigger: bool,
    pub gate: bool,
    pub fractional_sample: f32,
    pub regularity: f32,
    pub phase: f32,
}

const MAX_GRAIN_FREQUENCY: f32 = 130.812_78;
const DIVIDER_RESET_INTERVAL: f32 = 4.0;
const AUDIO_RATE_INTERVAL: f32 = 1.0 / 48.0;
const DIVISION_RATIOS: [i32; 8] = [1, 2, 3, 4, 6, 8, 12, 16];

pub struct TriggerGenerator {
    phase: f32,
    frequency: f32,
    target_frequency: f32,
    frequency_lp_coefficient: f32,
    ratio: f32,
    one_hertz: f32,
    division: i32,
    audio_rate: bool,
    divider_quantizer: HysteresisQuantizer2,
    max_pulse_frequency: f32,
    audio_rate_interval: i32,
    accept_pulse: bool,
    pulse_count: i32,
    elapsed_since_last_pulse: i32,
    divider_reset_interval: i32,
    default_reset_interval: i32,
    previous_flags: GateFlags,
    merged: Vec<GateFlags>,
}

impl TriggerGenerator {
    pub fn new(sample_rate: f32) -> Self {
        let mut t = Self {
            phase: 0.0,
            frequency: 0.0,
            target_frequency: 0.0,
            frequency_lp_coefficient: 1.0,
            ratio: 1.0,
            one_hertz: 0.0,
            division: 1,
            audio_rate: false,
            divider_quantizer: HysteresisQuantizer2::default(),
            max_pulse_frequency: 0.0,
            audio_rate_interval: 0,
            accept_pulse: true,
            pulse_count: 0,
            elapsed_since_last_pulse: 0,
            divider_reset_interval: 0,
            default_reset_interval: 0,
            previous_flags: GATE_FLAG_LOW,
            merged: Vec::with_capacity(4096),
        };
        t.init(sample_rate);
        t
    }

    pub fn init(&mut self, sample_rate: f32) {
        self.one_hertz = 1.0 / sample_rate;
        self.max_pulse_frequency = MAX_GRAIN_FREQUENCY / sample_rate;
        self.default_reset_interval = (sample_rate * DIVIDER_RESET_INTERVAL) as i32;
        self.divider_reset_interval = self.default_reset_interval;
        self.audio_rate_interval = (sample_rate * AUDIO_RATE_INTERVAL) as i32;
        self.audio_rate = false;
        self.divider_quantizer.init(8, 0.075, false);
        self.accept_pulse = true;
        self.pulse_count = 0;
        self.elapsed_since_last_pulse = (sample_rate / 2.0) as i32;
        self.phase = 0.0;
        self.frequency = 1.0 / self.elapsed_since_last_pulse as f32;
        self.target_frequency = self.frequency;
        self.frequency_lp_coefficient = 1.0;
        self.division = 1;
        self.ratio = 1.0;
        self.previous_flags = GATE_FLAG_LOW;
    }

    fn render_pulse_train(&mut self, frequency: f32, randomize: bool, out: &mut [TriggerOutput], rng: &mut Random) {
        if !randomize {
            self.ratio = 1.0;
        }
        for (i, o) in out.iter_mut().enumerate() {
            let f = (frequency * self.ratio).clamp(0.000_001, 0.25);
            let flags = self.merged[i];
            let gate = flags & GATE_FLAG_HIGH != 0;
            let trigger = flags & GATE_FLAG_RISING != 0;
            self.phase += if gate { f } else { 0.0 };
            let mut t = 0.0;
            let mut reset = false;
            if trigger {
                self.phase = 0.0;
                reset = true;
            } else if self.phase >= 1.0 {
                self.phase -= 1.0;
                t = self.phase / f;
                reset = true;
            }
            if reset && randomize {
                // Poisson-ish: exponentially distributed intervals.
                self.ratio = -1.0 / log_one_minus_x(0.1 + 0.9 * rng.get_float());
            }
            *o = TriggerOutput { fractional_sample: t, trigger: reset && gate, gate: self.phase < 0.5 && gate, regularity: if randomize { 0.0 } else { 1.0 }, phase: self.phase };
        }
    }

    fn render_divider(&mut self, input: &[GateFlags], density: f32, out: &mut [TriggerOutput], rng: &mut Random) {
        let division = if density < 0.48 { DIVISION_RATIOS[self.divider_quantizer.process(density * 2.15) as usize] } else { 1 };
        let probability_sqrt = if density < 0.48 { 1.01 } else { ((density - 0.52) * 2.15).max(0.0) };
        let probability = probability_sqrt * probability_sqrt;
        let regularity = if probability >= 1.0 { 1.0 } else { 0.0 };
        for (i, o) in out.iter_mut().enumerate() {
            let target = self.target_frequency;
            one_pole(&mut self.frequency, target, self.frequency_lp_coefficient);
            self.phase += self.frequency;
            let flags = input[i];
            let gate = flags & GATE_FLAG_HIGH != 0;
            let trigger = flags & GATE_FLAG_RISING != 0;
            if trigger {
                // After a long silence, resynchronize with the first pulse.
                if self.elapsed_since_last_pulse >= self.divider_reset_interval {
                    self.pulse_count = 0;
                    self.phase = 0.0;
                }
                self.divider_reset_interval = self.default_reset_interval.max(4 * self.elapsed_since_last_pulse);
                // Pulses faster than 48 Hz: follow them with a free-running,
                // smoothed phasor.
                let threshold = self.audio_rate_interval + if self.audio_rate { 8 } else { 0 };
                self.audio_rate = self.elapsed_since_last_pulse < threshold && probability >= 1.0;
                self.accept_pulse = rng.get_float() < probability && (self.pulse_count % division) == 0;
                if self.accept_pulse {
                    let period = self.elapsed_since_last_pulse as f32;
                    let frequency = 1.0 / period;
                    if probability >= 1.0 {
                        self.target_frequency = frequency / division as f32;
                    } else {
                        self.target_frequency = frequency * probability;
                    }
                    if self.audio_rate {
                        let mut no_glide = division != self.division;
                        self.division = division;
                        let up = (1.02 + 2.0 * frequency) * self.frequency;
                        let down = (0.98 - 2.0 * frequency) * self.frequency;
                        no_glide |= self.target_frequency > up || self.target_frequency < down;
                        self.frequency_lp_coefficient = if no_glide { 1.0 } else { period * self.one_hertz };
                    } else {
                        self.phase = 0.0;
                        self.frequency_lp_coefficient = 1.0;
                    }
                } else if probability < 1.0 {
                    self.target_frequency *= 1.0 - probability;
                }
                self.pulse_count = (self.pulse_count + 1) % 48;
                self.elapsed_since_last_pulse = 0;
            }
            self.elapsed_since_last_pulse += 1;
            if self.audio_rate {
                let mut t = 0.0;
                let reset = self.phase >= 1.0;
                if reset {
                    self.phase -= 1.0;
                    t = self.phase / self.frequency;
                }
                *o = TriggerOutput { fractional_sample: t, trigger: reset, gate: self.phase < 0.5, regularity, phase: self.phase };
            } else {
                if self.phase >= 1.0 {
                    self.phase = 1.0;
                }
                *o = TriggerOutput { fractional_sample: 0.0, trigger: self.accept_pulse && trigger, gate: self.accept_pulse && gate, regularity, phase: self.phase };
            }
        }
    }

    /// Grain triggers for a block. `external` is the SEED input (if
    /// patched), one flag per sample.
    #[allow(clippy::too_many_arguments)]
    pub fn render(&mut self, button: SeedButtonState, external: Option<&[GateFlags]>, density: f32, linear_fm: f32, exponential_fm: f32, out: &mut [TriggerOutput], rng: &mut Random) {
        let size = out.len();
        if let (Some(ext), SeedButtonState::Locked) = (external, button) {
            // The CV only swings the knob here.
            let density = (density + linear_fm + exponential_fm).clamp(0.0, 1.0);
            self.render_divider(ext, density, out, rng);
            return;
        }
        let density = (density + linear_fm).clamp(0.0, 1.0);
        let fm = semitones_to_ratio(exponential_fm * 60.0);
        // The button and the SEED input, merged into one gate.
        self.merged.clear();
        for i in 0..size {
            let mut gate = button != SeedButtonState::Off;
            if let Some(ext) = external {
                gate |= ext[i] & GATE_FLAG_HIGH != 0;
            }
            self.previous_flags = extract_gate_flags(self.previous_flags, gate);
            self.merged.push(self.previous_flags);
        }
        if density < 0.48 {
            let f = self.max_pulse_frequency * semitones_to_ratio(-density * 192.0) * fm;
            self.render_pulse_train(f, false, out, rng);
        } else if density > 0.52 {
            let f = 0.5 * self.max_pulse_frequency * semitones_to_ratio(-(1.0 - density) * 192.0) * fm;
            self.render_pulse_train(f, true, out, rng);
        } else if button == SeedButtonState::Locked {
            let o = TriggerOutput { fractional_sample: 0.0, trigger: false, gate: false, regularity: 0.0, phase: self.phase };
            out.fill(o);
        } else {
            // Manual taps.
            for (i, o) in out.iter_mut().enumerate() {
                self.phase += self.frequency;
                let tap = self.merged[i] & GATE_FLAG_RISING != 0;
                if tap {
                    self.phase = 0.0;
                    self.frequency = 1.0 / self.elapsed_since_last_pulse.max(1) as f32;
                    self.elapsed_since_last_pulse = 0;
                }
                self.elapsed_since_last_pulse += 1;
                if self.phase >= 1.0 {
                    self.phase = 1.0;
                }
                *o = TriggerOutput { fractional_sample: 0.0, trigger: tap, gate: self.merged[i] & GATE_FLAG_HIGH != 0, regularity: 1.0 - (density - 0.48) * 25.0, phase: self.phase };
            }
        }
    }
}

#[inline]
fn log_one_minus_x(x: f32) -> f32 {
    -x * (1.0 + x * (0.5 + x * (0.3333 + x * 0.25)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regular_grains_come_slower_towards_noon() {
        let count = |density: f32| {
            let mut t = TriggerGenerator::new(48000.0);
            let mut rng = Random::default();
            let mut out = vec![TriggerOutput::default(); 480];
            let mut n = 0;
            for _ in 0..200 {
                t.render(SeedButtonState::Locked, None, density, 0.0, 0.0, &mut out, &mut rng);
                n += out.iter().filter(|o| o.trigger).count();
            }
            n
        };
        assert!(count(0.1) > 4 * count(0.4).max(1), "{} {}", count(0.1), count(0.4));
        assert_eq!(count(0.5), 0);
    }

    #[test]
    fn a_clock_is_divided_left_of_noon() {
        let mut t = TriggerGenerator::new(48000.0);
        let mut rng = Random::default();
        // A pulse every 4800 samples.
        let mut prev = GATE_FLAG_LOW;
        let ext: Vec<GateFlags> = (0..48000)
            .map(|i| {
                prev = extract_gate_flags(prev, i % 4800 < 100);
                prev
            })
            .collect();
        let mut out = vec![TriggerOutput::default(); 48000];
        // density 0.1 * 2.15 = 0.215 of 8 steps: a division of 2.
        t.render(SeedButtonState::Locked, Some(&ext), 0.1, 0.0, 0.0, &mut out, &mut rng);
        let n = out.iter().filter(|o| o.trigger).count();
        assert_eq!(n, 5, "{n}");
    }
}

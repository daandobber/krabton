//! Grain scheduling (`beads/dsp/granular_player.h`,
//! `delay_granular_player.h`, `wavetable_granular_player.h`): 32 grains;
//! when a new one starts, the ones most nearly done are faded out quickly
//! so that one of them can be taken without a click.
//!
//! Copyright 2017 Emilie Gillet, MIT license. Rust port for Krabton.

use crate::delay_line::DelayLine;
use crate::grain::{DelayGrain, Grain, WavetableGrain, HEAD_CROSSING_SAFETY_ZONE, WAVETABLE_ROOT_NOTE};
use crate::random::{bounded_random_float, Parameters};
use crate::trigger::TriggerOutput;
use crate::util::{semitones_to_ratio, Random};

const INSTANT_CROSSFADE_TIME: f32 = 0.005;
const SIMPLE_SCHEDULER_THRESHOLD: f32 = 0.025;
const PREEMPTIVE_CROSSFADE_TIME: f32 = 0.1;
pub const NUM_GRAINS: usize = 32;

pub struct GranularPlayer<G: Grain + Copy + Default> {
    pub grains: [G; NUM_GRAINS],
    priority: [usize; NUM_GRAINS],
    preemptive_crossfade_time: i32,
    instant_crossfade_time: i32,
    simple_scheduler_threshold: i32,
    max_grain_duration: f32,
    pub one_hertz: f32,
    grain_interval: i32,
}

impl<G: Grain + Copy + Default> GranularPlayer<G> {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            grains: [G::default(); NUM_GRAINS],
            priority: std::array::from_fn(|i| i),
            instant_crossfade_time: (sample_rate * INSTANT_CROSSFADE_TIME) as i32,
            preemptive_crossfade_time: (sample_rate * PREEMPTIVE_CROSSFADE_TIME) as i32,
            simple_scheduler_threshold: (sample_rate * SIMPLE_SCHEDULER_THRESHOLD) as i32,
            max_grain_duration: sample_rate * 4.0,
            one_hertz: 1.0 / sample_rate,
            grain_interval: 1,
        }
    }

    pub fn reset_order(&mut self) {
        self.priority = std::array::from_fn(|i| i);
    }

    fn compute_spread(trigger: &TriggerOutput, p: &Parameters) -> f32 {
        let total = p.time.randomness_amount() + p.size.randomness_amount() + p.shape.randomness_amount() + p.pitch.randomness_amount() + 1.0 - trigger.regularity;
        total.min(1.0)
    }

    /// SIZE to a duration in samples: up to 4 s at noon... the left third
    /// plays backwards.
    pub fn compute_grain_duration(&self, duration: f32) -> f32 {
        let reverse = duration < 0.333_333_3;
        let clamped_shortness = if reverse { duration * 3.0 * (2.0 - duration * 3.0) } else { 1.0 - 1.5 * (duration - 0.333_333_3) };
        let shortness = (1.025 * clamped_shortness).min(1.0);
        semitones_to_ratio(shortness * -108.0) * self.max_grain_duration
    }

    pub fn count_active(&self) -> usize {
        self.grains.iter().filter(|g| !g.core().done()).count()
    }

    pub fn amplitude_and_pan(&self, trigger: &TriggerOutput, p: &Parameters, rng: &mut Random) -> (f32, f32) {
        let active = (self.count_active() + 1) as f32;
        let amplitude = 1.0 / active.sqrt();
        let spread = Self::compute_spread(trigger, p);
        let pan_spread = 0.5 * active * spread;
        (amplitude, bounded_random_float(pan_spread, rng))
    }

    pub fn new_grain(&self) -> usize {
        self.priority[0]
    }

    pub fn tick(&mut self) {
        self.grain_interval += 1;
    }

    /// After a grain started: the first ones in line are faded out, so they
    /// can be taken quietly by the next grains.
    pub fn sort_grain_list(&mut self, monophonic_mode: bool) {
        if monophonic_mode {
            self.priority.rotate_left(1);
            let first = self.priority[0];
            let fade = self.grain_interval.min(self.instant_crossfade_time);
            self.grains[first].core_mut().fade_out(fade);
            for i in 1..NUM_GRAINS - 1 {
                let g = self.priority[i];
                self.grains[g].core_mut().fade_out(self.instant_crossfade_time);
            }
        } else {
            let mut fade_time;
            if self.grain_interval < self.simple_scheduler_threshold {
                // Grains coming fast: the first one is surely nearly done.
                self.priority.rotate_left(1);
                fade_time = [self.grain_interval + 1, self.instant_crossfade_time, self.instant_crossfade_time, self.preemptive_crossfade_time];
                if self.grain_interval > self.instant_crossfade_time {
                    fade_time.swap(0, 2);
                }
            } else {
                // The ones with the least time left go first.
                let grains = &self.grains;
                self.priority.sort_by_key(|&i| grains[i].core().remaining_samples);
                fade_time = [self.instant_crossfade_time, self.instant_crossfade_time, self.grain_interval, self.preemptive_crossfade_time];
                if self.grain_interval > self.preemptive_crossfade_time {
                    fade_time.swap(2, 3);
                }
            }
            for (i, &f) in fade_time.iter().enumerate() {
                let g = self.priority[i];
                self.grains[g].core_mut().fade_out(f);
            }
        }
        self.grain_interval = 0;
    }
}

/// Grains reading the recording.
pub struct DelayGranularPlayer {
    pub player: GranularPlayer<DelayGrain>,
}

impl DelayGranularPlayer {
    pub fn new(sample_rate: f32) -> Self {
        let mut player = GranularPlayer::<DelayGrain>::new(sample_rate);
        for g in player.grains.iter_mut() {
            g.init(sample_rate);
        }
        Self { player }
    }

    pub fn reset(&mut self, sample_rate: f32) {
        for g in self.player.grains.iter_mut() {
            g.init(sample_rate);
        }
        self.player.reset_order();
    }

    pub fn render(&mut self, line: &DelayLine, trigger: &TriggerOutput, p: &Parameters, out: &mut [f32; 2], rng: &mut Random) {
        if trigger.trigger {
            let duration = p.size.value(rng);
            let reverse = duration < 0.333_333_3;
            let (amplitude, pan) = self.player.amplitude_and_pan(trigger, p, rng);
            let safety = HEAD_CROSSING_SAFETY_ZONE + 1;
            let max_delay = (line.duration() - 2 * safety).max(0);
            let time = p.time.value(rng);
            let speed = if reverse { -1.0 } else { 1.0 } * p.pitch.pitch_value(rng);
            let grain_duration = self.player.compute_grain_duration(duration);
            let shape = p.shape.value(rng);
            let g = self.player.new_grain();
            self.player.grains[g].start(line, time * max_delay as f32 + safety as f32, speed, grain_duration, shape, amplitude, pan, trigger.fractional_sample);
            self.player.sort_grain_list(p.monophonic_mode);
        }
        for g in self.player.grains.iter_mut() {
            g.render(line, out);
        }
        self.player.tick();
    }
}

/// Grains playing the wavetables (the synth, with nothing coming in).
pub struct WavetableGranularPlayer {
    pub player: GranularPlayer<WavetableGrain>,
}

impl WavetableGranularPlayer {
    pub fn new(sample_rate: f32) -> Self {
        let mut player = GranularPlayer::<WavetableGrain>::new(sample_rate);
        for g in player.grains.iter_mut() {
            g.init(sample_rate);
        }
        Self { player }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render(&mut self, waves: &[i16], bank: usize, bit_shift: u32, trigger: &TriggerOutput, freeze: bool, p: &Parameters, out: &mut [f32; 2], rng: &mut Random) {
        if trigger.trigger && !freeze {
            let (amplitude, pan) = self.player.amplitude_and_pan(trigger, p, rng);
            let wave = p.time.value(rng);
            let frequency = p.pitch.pitch_value(rng) * WAVETABLE_ROOT_NOTE * self.player.one_hertz;
            let duration = self.player.compute_grain_duration(p.size.value(rng));
            let shape = p.shape.value(rng);
            let g = self.player.new_grain();
            self.player.grains[g].start(waves, bank, bit_shift, wave, frequency, duration, shape, amplitude, pan, trigger.fractional_sample);
            self.player.sort_grain_list(p.monophonic_mode);
        }
        for g in self.player.grains.iter_mut() {
            g.render(waves, out, freeze);
        }
        self.player.tick();
    }
}

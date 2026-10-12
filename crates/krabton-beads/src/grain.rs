//! Grains (`beads/dsp/grain.h`, `grain_envelope.h`,
//! `voice_stealing_envelope.h`, `delay_grain.h`, `wavetable_grain.h`): a
//! grain reads the recording (or a wavetable) at its own speed, shaped by
//! an envelope from square through decay and bell to reverse; a grain that
//! must make room for a new one fades out quickly.
//!
//! Copyright 2017 Emilie Gillet, MIT license. Rust port for Krabton.

use crate::delay_line::DelayLine;
use crate::util::{integral_fractional, one_pole, smooth_step};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Stage {
    Attack = 0,
    Sustain = 1,
    Release = 2,
    Done = 3,
}

#[derive(Clone, Copy, Debug)]
pub struct GrainEnvelope {
    stage: Stage,
    phase: f32,
    increment: [f32; 4],
    smoothness: f32,
    min_attack: f32,
    min_release: f32,
}

impl Default for GrainEnvelope {
    fn default() -> Self {
        Self { stage: Stage::Done, phase: 0.0, increment: [0.5, 0.5, 0.5, 0.0], smoothness: 0.0, min_attack: 24.0, min_release: 480.0 }
    }
}

impl GrainEnvelope {
    pub fn init(&mut self, sample_rate: f32) {
        *self = Self { min_attack: 0.0005 * sample_rate, min_release: 0.010 * sample_rate, ..Self::default() };
    }

    fn start(&mut self, duration: f32, shape: f32) {
        self.stage = Stage::Attack;
        self.phase = 0.0;
        let sustain = (1.0 - 3.0 * shape).max(0.0);
        let mut shape = (shape - 0.333_333_33) * 1.5;
        if shape < 0.0 {
            shape = 0.0;
        } else if shape < 0.5 {
            shape = shape * shape * shape * 4.0;
        }
        let attack = shape;
        let release = (1.0 - attack) * (1.0 - sustain);
        let attack_duration = (duration * attack).max(self.min_attack * (1.0 + 3.0 * sustain));
        let sustain_duration = (duration * sustain).max(2.0);
        let release_duration = (duration * release).max(self.min_release);
        self.increment[0] = 1.0 / attack_duration;
        self.increment[1] = 1.0 / sustain_duration;
        self.increment[2] = 1.0 / release_duration;
        self.smoothness = if attack < 0.5 { 2.0 * attack } else { 2.0 - 2.0 * attack };
    }

    pub fn start_at(&mut self, duration: f32, shape: f32, fractional_sample: f32) {
        self.start(duration, shape);
        self.phase += self.increment[self.stage as usize] * fractional_sample;
        self.wrap_phase();
    }

    pub fn stop(&mut self) {
        self.stage = Stage::Done;
    }

    fn wrap_phase(&mut self) {
        if self.phase >= 1.0 && self.stage != Stage::Done {
            let t = (self.phase - 1.0) / self.increment[self.stage as usize];
            self.stage = match self.stage {
                Stage::Attack => Stage::Sustain,
                Stage::Sustain => Stage::Release,
                _ => Stage::Done,
            };
            self.phase = t * self.increment[self.stage as usize];
        }
    }

    #[inline]
    pub fn render(&mut self) -> f32 {
        if self.stage == Stage::Done {
            return 0.0;
        }
        let value = if self.stage == Stage::Sustain {
            1.0
        } else {
            let attack = self.stage == Stage::Attack;
            let t = if attack { self.phase } else { 1.0 - self.phase };
            let base = if attack { t } else { (t * t) * (t * t) };
            base + (smooth_step(t) - base) * self.smoothness
        };
        self.phase += self.increment[self.stage as usize];
        self.wrap_phase();
        value
    }

    pub fn done(&self) -> bool {
        self.stage >= Stage::Done
    }
}

#[derive(Clone, Copy, Debug)]
pub struct VoiceStealingEnvelope {
    value: f32,
    decrement: f32,
}

impl Default for VoiceStealingEnvelope {
    fn default() -> Self {
        Self { value: 1.0, decrement: 0.0 }
    }
}

impl VoiceStealingEnvelope {
    pub fn reset_and_pause(&mut self) {
        self.value = 1.0;
        self.decrement = 0.0;
    }

    pub fn start(&mut self, duration: f32) -> i32 {
        self.decrement = 1.0 / duration.max(1.0);
        (self.value * duration) as i32
    }

    #[inline]
    pub fn render(&mut self) -> f32 {
        self.value -= self.decrement;
        self.value.max(0.0)
    }

    pub fn done(&self) -> bool {
        self.value <= 0.0
    }
}

/// What every kind of grain has.
#[derive(Clone, Copy, Debug, Default)]
pub struct GrainCore {
    pub remaining_samples: i32,
    pub envelope: GrainEnvelope,
    pub voice_stealing: VoiceStealingEnvelope,
}

impl GrainCore {
    pub fn done(&self) -> bool {
        self.envelope.done() || self.voice_stealing.done()
    }

    pub fn fade_out(&mut self, duration: i32) {
        if self.remaining_samples > duration {
            self.remaining_samples = self.voice_stealing.start(duration as f32);
        }
    }
}

pub trait Grain {
    fn core(&self) -> &GrainCore;
    fn core_mut(&mut self) -> &mut GrainCore;
}

/// Where a grain is, for a display: how far back it reads (0 now, 1 the
/// oldest), how loud it is now, its pitch (semitones) and direction.
#[derive(Clone, Copy, Debug, Default)]
pub struct GrainView {
    pub position: f32,
    pub level: f32,
    pub pitch: f32,
    pub reverse: bool,
}

/// The length of the zone near the recording head where a grain fades.
pub const HEAD_CROSSING_SAFETY_ZONE: i32 = 256;

/// A grain reading the recording.
#[derive(Clone, Copy, Debug)]
pub struct DelayGrain {
    core: GrainCore,
    /// The envelope now, for a display.
    level: f32,
    gain: [[f32; 2]; 2],
    delay: i32,
    speed: i32,
    initial_delay: i32,
}

impl Default for DelayGrain {
    fn default() -> Self {
        Self { core: GrainCore::default(), level: 0.0, gain: [[0.5; 2]; 2], delay: 0, speed: 1024, initial_delay: 0 }
    }
}

impl Grain for DelayGrain {
    fn core(&self) -> &GrainCore {
        &self.core
    }
    fn core_mut(&mut self) -> &mut GrainCore {
        &mut self.core
    }
}

impl DelayGrain {
    pub fn init(&mut self, sample_rate: f32) {
        *self = Self::default();
        self.core.envelope.init(sample_rate);
        self.core.voice_stealing.reset_and_pause();
        self.core.remaining_samples = 0;
    }

    #[allow(clippy::too_many_arguments)]
    pub fn start(&mut self, line: &DelayLine, mut delay: f32, speed: f32, mut duration: f32, shape: f32, gain: f32, pan: f32, fractional_sample: f32) {
        let max_delay = line.max_delay() as f32;
        let consumption_speed = speed - if line.frozen() { 0.0 } else { 1.0 };
        delay -= fractional_sample * consumption_speed;
        // Room enough in the buffer, and not running into the recording head.
        if consumption_speed != 0.0 {
            duration = duration.min(max_delay / consumption_speed.abs());
        }
        if consumption_speed > 0.0 {
            delay = delay.max(duration * consumption_speed);
        } else if consumption_speed < 0.0 {
            delay = delay.min(max_delay + duration * consumption_speed);
        }
        self.delay = (delay * 1024.0) as i32;
        self.initial_delay = self.delay;
        self.speed = (speed * 1024.0) as i32;
        // Roughly equal power (a first-order square root).
        let mut pan = pan;
        if line.channels() == 2 {
            if pan < 0.5 {
                pan *= 2.0;
                self.gain[0][0] = gain;
                self.gain[1][0] = 0.0;
                self.gain[1][1] = pan * (2.0 - pan) * gain;
                self.gain[0][1] = (1.0 - pan) * (1.0 + pan) * gain;
            } else {
                pan = (1.0 - pan) * 2.0;
                self.gain[1][1] = gain;
                self.gain[0][1] = 0.0;
                self.gain[0][0] = pan * (2.0 - pan) * gain;
                self.gain[1][0] = (1.0 - pan) * (1.0 + pan) * gain;
            }
        } else {
            self.gain[1][0] = pan * (2.0 - pan) * gain;
            self.gain[0][0] = (1.0 - pan) * (1.0 + pan) * gain;
            self.gain[0][1] = 0.0;
            self.gain[1][1] = 0.0;
        }
        self.core.envelope.start_at(duration, shape, fractional_sample);
        self.core.remaining_samples = duration as i32;
        self.core.voice_stealing.reset_and_pause();
    }

    pub fn view(&self, max_delay: i32) -> Option<GrainView> {
        if self.core.done() {
            return None;
        }
        let tail = (max_delay.max(1) as f32) * 1024.0;
        let speed = self.speed as f32 / 1024.0;
        Some(GrainView { position: (self.delay as f32 / tail).clamp(0.0, 1.0), level: self.level, pitch: 12.0 * speed.abs().max(1e-3).log2(), reverse: speed < 0.0 })
    }

    /// Plays one sample (the module's "stop at the recording head" policy,
    /// fading near it).
    #[inline]
    pub fn render(&mut self, line: &DelayLine, out: &mut [f32; 2]) {
        if self.core.done() {
            return;
        }
        self.delay -= self.speed;
        if !line.frozen() {
            self.delay += 1024;
        }
        let tail = line.max_delay() << 10;
        let fade_zone = HEAD_CROSSING_SAFETY_ZONE << 10;
        if self.delay < 0 || self.delay >= tail {
            self.delay = 0;
            self.core.envelope.stop();
        }
        let d = (if self.delay >= tail / 2 { self.delay - tail } else { self.delay }).abs();
        let fade_ramp = if d < fade_zone { d as f32 / fade_zone as f32 } else { 1.0 };
        let mut sample = line.read_hermite_fixed(self.delay);
        let fade = smooth_step(fade_ramp);
        let env = self.core.envelope.render() * self.core.voice_stealing.render() * fade;
        self.level = env;
        sample[0] *= env;
        sample[1] *= env;
        out[0] += sample[0] * self.gain[0][0] + sample[1] * self.gain[0][1];
        out[1] += sample[0] * self.gain[1][0] + sample[1] * self.gain[1][1];
        self.core.remaining_samples -= 1;
        if self.core.done() {
            self.core.remaining_samples = 0;
        }
    }
}

/// The 16 waves of a table, 256 points each (with 4 for interpolation).
pub const TABLE_SIZE: usize = 256;
pub const NUM_WAVES_PER_TABLE: usize = 16;
pub const WAVETABLE_ROOT_NOTE: f32 = 130.81;

#[inline]
pub fn interpolate_wave_hermite(table: &[i16], index_integral: usize, f: f32) -> f32 {
    let xm1 = table[index_integral] as f32;
    let x0 = table[index_integral + 1] as f32;
    let x1 = table[index_integral + 2] as f32;
    let x2 = table[index_integral + 3] as f32;
    let c = (x1 - xm1) * 0.5;
    let v = x0 - x1;
    let w = c + v;
    let a = w + v + (x2 - x0) * 0.5;
    let b_neg = w + a;
    (((a * f) - b_neg) * f + c) * f + x0
}

/// The waves are stored integrated; this turns them back (with a gentle
/// low-pass against aliasing).
#[derive(Clone, Copy, Debug, Default)]
pub struct Differentiator {
    previous: f32,
    lp: f32,
    lp_2: f32,
    cutoff: f32,
    scale: f32,
}

impl Differentiator {
    pub fn init(&mut self, value: f32, scale: f32, cutoff: f32) {
        self.previous = value;
        self.lp = 0.0;
        self.lp_2 = 0.0;
        self.scale = scale;
        self.cutoff = cutoff;
    }

    #[inline]
    pub fn process(&mut self, s: f32) -> f32 {
        one_pole(&mut self.lp, (s - self.previous) * self.scale, self.cutoff);
        let lp = self.lp;
        one_pole(&mut self.lp_2, lp, self.cutoff);
        self.previous = s;
        self.lp_2
    }
}

/// A grain playing a wavetable.
#[derive(Clone, Copy, Debug, Default)]
pub struct WavetableGrain {
    core: GrainCore,
    /// The wave (0..1), for a display.
    wave: f32,
    phase: f32,
    frequency: f32,
    envelope_sample: f32,
    scale_up: f32,
    scale_down: f32,
    gain_l: f32,
    gain_r: f32,
    wave_fractional: f32,
    wave_start: usize,
    differentiator: Differentiator,
}

impl Grain for WavetableGrain {
    fn core(&self) -> &GrainCore {
        &self.core
    }
    fn core_mut(&mut self) -> &mut GrainCore {
        &mut self.core
    }
}

impl WavetableGrain {
    pub fn init(&mut self, sample_rate: f32) {
        *self = Self { scale_up: 256.0, scale_down: 1.0 / 256.0, ..Self::default() };
        self.core.envelope.init(sample_rate);
        self.core.voice_stealing.reset_and_pause();
    }

    #[allow(clippy::too_many_arguments)]
    pub fn start(&mut self, waves: &[i16], bank: usize, bit_shift: u32, wave: f32, frequency: f32, duration: f32, shape: f32, gain: f32, pan: f32, fractional_sample: f32) {
        self.wave = wave;
        let wave = wave * (NUM_WAVES_PER_TABLE - 1) as f32;
        let (wi, wf) = integral_fractional(wave);
        let wi = (wi.max(0) as usize).min(NUM_WAVES_PER_TABLE - 2);
        self.wave_fractional = wf;
        self.wave_start = (bank * NUM_WAVES_PER_TABLE + wi) * (TABLE_SIZE + 4);
        let frequency = frequency.min(0.25);
        self.core.remaining_samples = duration as i32;
        self.core.envelope.start_at(duration, shape, fractional_sample);
        self.gain_l = gain * (1.0 - pan) * (1.0 + pan);
        self.gain_r = gain * pan * (2.0 - pan);
        self.core.voice_stealing.reset_and_pause();
        self.scale_up = (1u32 << bit_shift) as f32;
        self.scale_down = 1.0 / self.scale_up;
        self.phase = fractional_sample * frequency;
        self.frequency = frequency;
        let first = self.read_sample(waves, self.phase);
        self.differentiator.init(first, 1.0 / (frequency * 131072.0), (TABLE_SIZE as f32 * frequency).min(1.0));
    }

    /// `one_hertz`: 1 / the sample rate.
    pub fn view(&self, one_hertz: f32) -> Option<GrainView> {
        if self.core.done() {
            return None;
        }
        let pitch = 12.0 * (self.frequency / (WAVETABLE_ROOT_NOTE * one_hertz)).max(1e-6).log2();
        Some(GrainView { position: self.wave, level: self.envelope_sample, pitch, reverse: false })
    }

    #[inline]
    fn read_sample(&self, waves: &[i16], phase: f32) -> f32 {
        let p = phase * TABLE_SIZE as f32;
        let (pi, pf) = integral_fractional(p);
        let pi = pi.clamp(0, TABLE_SIZE as i32 - 1) as usize;
        let start = self.wave_start;
        let x0 = interpolate_wave_hermite(&waves[start..], pi, pf);
        let x1 = interpolate_wave_hermite(&waves[start + TABLE_SIZE + 4..], pi, pf);
        x0 + (x1 - x0) * self.wave_fractional
    }

    #[inline]
    pub fn render(&mut self, waves: &[i16], out: &mut [f32; 2], freeze: bool) {
        if self.core.done() {
            return;
        }
        self.phase += self.frequency;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }
        let s = self.read_sample(waves, self.phase);
        let mut sample = self.differentiator.process(s);
        sample = ((sample * self.scale_up) as i32) as f32 * self.scale_down;
        if !freeze {
            self.envelope_sample = self.core.envelope.render() * self.core.voice_stealing.render();
            self.core.remaining_samples -= 1;
            if self.core.done() {
                self.core.remaining_samples = 0;
            }
        }
        sample *= self.envelope_sample;
        out[0] += sample * self.gain_l;
        out[1] += sample * self.gain_r;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_envelope_shapes_go_from_square_to_reverse() {
        // Square: sustains at full level; reverse: rises to the end.
        let run = |shape: f32| {
            let mut e = GrainEnvelope::default();
            e.init(48000.0);
            e.start_at(4800.0, shape, 0.0);
            (0..4800).map(|_| e.render()).collect::<Vec<f32>>()
        };
        let square = run(0.0);
        assert!(square[2400] > 0.99 && square[100] > 0.9);
        let reverse = run(1.0);
        assert!(reverse[4000] > reverse[1000] && reverse[1000] < 0.4);
        // Bell: attack 0.5 of the shape range past a third.
        let bell = run(0.667);
        assert!(bell[2400] > bell[300] && bell[2400] > bell[4500]);
    }
}

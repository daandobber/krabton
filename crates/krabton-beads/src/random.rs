//! Parameters subject to internal random modulation or to a CV
//! (`beads/dsp/randomized_parameter.h`, `modulated_parameter.h`).
//!
//! On the module the attenuverter of TIME, SIZE, SHAPE and PITCH does two
//! things: with a CV patched it scales the CV (counterclockwise: random
//! modulation gated by the CV), with nothing patched it sets how much each
//! grain's value is randomized (clockwise: uniformly; counterclockwise:
//! now and then, by larger jumps).
//!
//! Copyright 2017 Emilie Gillet, MIT license. Rust port for Krabton.

use crate::util::{semitones_to_ratio, smooth_step, Random};

#[inline]
pub fn centered_uniform(rng: &mut Random) -> f32 {
    2.0 * rng.get_float() - 1.0
}

#[inline]
pub fn pseudo_gaussian(rng: &mut Random) -> f32 {
    let u = rng.get_float();
    u * (u * (4.0 * u - 6.0) + 4.0) - 1.0
}

#[inline]
pub fn bounded_random_float(spread: f32, rng: &mut Random) -> f32 {
    let u = rng.get_float();
    let v = rng.get_float();
    0.5 + (u - 0.5) * (spread * v).min(1.0)
}

/// A smooth random walk between random points.
#[derive(Clone, Copy, Debug, Default)]
pub struct RandomWalkModulation {
    phase: f32,
    frequency: f32,
    from: f32,
    to: f32,
}

impl RandomWalkModulation {
    pub fn init(&mut self, frequency: f32) {
        self.frequency = frequency;
        self.phase = 0.0;
        self.from = 0.0;
        self.to = 1.0;
    }

    pub fn value(&mut self, attenuverter: f32, step_size: usize, rng: &mut Random) -> f32 {
        let frequency = if attenuverter <= 0.0 { 0.25 } else { 4.0 } * self.frequency * step_size as f32;
        self.phase += frequency;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
            self.from = self.to;
            self.to = centered_uniform(rng);
        }
        self.from + (self.to - self.from) * smooth_step(self.phase)
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct RandomizedParameter {
    pub base: f32,
    pub attenuverter: f32,
    pub cv: f32,
    pub has_cv: bool,
}

impl RandomizedParameter {
    pub fn randomness_amount(&self) -> f32 {
        if self.has_cv {
            (-self.attenuverter).max(0.0)
        } else {
            self.attenuverter.abs()
        }
    }

    pub fn value_with(&self, modulation: Option<&mut RandomWalkModulation>, step_size: usize, range: f32, clamp: bool, rng: &mut Random) -> f32 {
        let mut result = self.base;
        if !self.has_cv {
            match modulation {
                Some(m) => result += self.attenuverter * range * m.value(self.attenuverter, step_size, rng),
                None => {
                    let blend = if self.attenuverter > 0.0 { self.attenuverter } else { (self.attenuverter * rng.get_float() * -3.0).min(1.0) };
                    let random_value = if !clamp { centered_uniform(rng) * range } else { rng.get_float() * range - result };
                    result += random_value * blend;
                }
            }
        } else if self.attenuverter < 0.0 {
            let extra = match modulation {
                Some(m) => m.value(1.0, step_size, rng),
                None => pseudo_gaussian(rng),
            };
            result += self.cv.max(0.0) * -self.attenuverter * extra;
        } else {
            result += self.attenuverter * self.cv;
        }
        if clamp {
            result = result.clamp(0.0, 1.0);
        }
        result
    }

    /// A new value for a grain, 0..1.
    pub fn value(&self, rng: &mut Random) -> f32 {
        self.value_with(None, 0, 1.0, true, rng)
    }

    /// A new playback speed for a grain, from a pitch in semitones.
    pub fn pitch_value(&self, rng: &mut Random) -> f32 {
        semitones_to_ratio(self.value_with(None, 0, 24.0, false, rng))
    }
}

/// The parameters of the grains.
#[derive(Clone, Copy, Debug, Default)]
pub struct Parameters {
    pub time: RandomizedParameter,
    pub size: RandomizedParameter,
    pub shape: RandomizedParameter,
    pub pitch: RandomizedParameter,
    pub monophonic_mode: bool,
}

/// A parameter and its CV.
#[derive(Clone, Copy, Debug, Default)]
pub struct ModulatedParameter {
    pub base: f32,
    pub cv: f32,
}

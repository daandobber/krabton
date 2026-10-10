//! Granular playback: grains overlap-added from the recording buffer
//! (`clouds/dsp/grain.h`, `clouds/dsp/granular_sample_player.h`).
//!
//! Copyright 2014 Emilie Gillet, MIT license. Rust port for Krabton.

use crate::audio_buffer::{AudioBuffer, Interpolation, Resolution};
use crate::dsp::{crossfade, fast_rsqrt_carmack, interpolate, one_pole, semitones_to_ratio, slope, Random};
use crate::parameters::Parameters;
use crate::resources::{LUT_GRAIN_SIZE, LUT_SIN, LUT_WINDOW};
use crate::MAX_BLOCK_SIZE;

pub const MAX_NUM_GRAINS: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum GrainQuality {
    Low,
    Medium,
    High,
}

impl GrainQuality {
    fn interpolation(self) -> Interpolation {
        match self {
            GrainQuality::Low => Interpolation::Zoh,
            GrainQuality::Medium => Interpolation::Linear,
            GrainQuality::High => Interpolation::Hermite,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Grain {
    first_sample: i32,
    phase: i32,
    phase_increment: i32,
    pre_delay: i32,
    envelope_smoothness: f32,
    envelope_slope: f32,
    envelope_phase: f32,
    envelope_phase_increment: f32,
    gain_l: f32,
    gain_r: f32,
    active: bool,
    recommended_quality: GrainQuality,
}

impl Default for Grain {
    fn default() -> Self {
        Self {
            first_sample: 0,
            phase: 0,
            phase_increment: 0,
            pre_delay: 0,
            envelope_smoothness: 0.0,
            envelope_slope: 0.0,
            envelope_phase: 2.0,
            envelope_phase_increment: 0.0,
            gain_l: 0.0,
            gain_r: 0.0,
            active: false,
            recommended_quality: GrainQuality::High,
        }
    }
}

impl Grain {
    pub fn init(&mut self) {
        self.active = false;
        self.envelope_phase = 2.0;
    }

    pub fn start(
        &mut self,
        pre_delay: i32,
        buffer_size: i32,
        start: i32,
        width: i32,
        phase_increment: i32,
        window_shape: f32,
        gain_l: f32,
        gain_r: f32,
        recommended_quality: GrainQuality,
    ) {
        self.pre_delay = pre_delay;
        // The C++ `%` can leave this negative in corner cases; wrap instead.
        self.first_sample = start.wrapping_add(buffer_size).rem_euclid(buffer_size.max(1));
        self.phase_increment = phase_increment;
        self.phase = 0;
        self.envelope_phase = 0.0;
        self.envelope_phase_increment = 2.0 / width.max(1) as f32;
        if window_shape >= 0.5 {
            self.envelope_smoothness = (window_shape - 0.5) * 2.0;
            self.envelope_slope = 0.0;
        } else {
            self.envelope_smoothness = 0.0;
            self.envelope_slope = 0.5 / (window_shape + 0.01);
        }
        self.active = true;
        self.gain_l = gain_l;
        self.gain_r = gain_r;
        self.recommended_quality = recommended_quality;
    }

    #[inline]
    pub fn active(&self) -> bool {
        self.active
    }

    /// Renders the envelope; -1 marks the end of the grain.
    fn render_envelope(&mut self, use_lut: bool, quality: GrainQuality, destination: &mut [f32]) {
        let increment = self.envelope_phase_increment;
        let smoothness = self.envelope_smoothness;
        let slope = self.envelope_slope;
        let mut phase = self.envelope_phase;
        for d in destination.iter_mut() {
            let mut gain = phase;
            gain = if gain >= 1.0 { 2.0 - gain } else { gain };
            if use_lut {
                if quality == GrainQuality::High {
                    let window = interpolate(&LUT_WINDOW, gain, 4096.0);
                    gain += smoothness * (window - gain);
                }
            } else if quality >= GrainQuality::Medium {
                gain *= slope;
                if gain >= 1.0 {
                    gain = 1.0;
                }
            }
            phase += increment;
            if phase >= 2.0 {
                *d = -1.0;
                break;
            }
            *d = gain;
        }
        self.envelope_phase = phase;
    }

    /// Adds the grain to the interleaved block `destination`.
    pub fn overlap_add<R: Resolution>(
        &mut self,
        buffer: &[AudioBuffer<R>; 2],
        num_channels: i32,
        destination: &mut [[f32; 2]],
        envelope: &mut [f32; MAX_BLOCK_SIZE],
    ) {
        if !self.active {
            return;
        }
        let quality = self.recommended_quality;
        // Rendering is done on blocks. The pre-delay allows grains to start
        // at arbitrary samples within a block.
        let skip = (self.pre_delay.max(0) as usize).min(destination.len());
        self.pre_delay -= skip as i32;
        let destination = &mut destination[skip..];
        let size = destination.len().min(MAX_BLOCK_SIZE);

        let use_lut = self.envelope_smoothness != 0.0;
        self.render_envelope(use_lut, quality, &mut envelope[..size]);

        let method = quality.interpolation();
        let phase_increment = self.phase_increment;
        let first_sample = self.first_sample;
        let (gain_l, gain_r) = (self.gain_l, self.gain_r);
        let mut phase = self.phase;
        for (out, &gain) in destination[..size].iter_mut().zip(envelope.iter()) {
            if gain == -1.0 {
                self.active = false;
                break;
            }
            let sample_index = first_sample.wrapping_add(phase >> 16);
            let fractional = (phase & 65535) as u16;
            let l = buffer[0].read(method, sample_index, fractional) * gain;
            if num_channels == 1 {
                out[0] += l * gain_l;
                out[1] += l * gain_r;
            } else {
                let r = buffer[1].read(method, sample_index, fractional) * gain;
                out[0] += l * gain_l + r * (1.0 - gain_r);
                out[1] += r * gain_r + l * (1.0 - gain_l);
            }
            phase = phase.wrapping_add(phase_increment);
        }
        self.phase = phase;
    }
}

pub struct GranularSamplePlayer {
    max_num_grains: i32,
    num_midfi_grains: i32,
    num_channels: i32,
    num_grains: f32,
    gain_normalization: f32,
    grain_size_hint: f32,
    grain_rate_phasor: f32,
    grains: [Grain; MAX_NUM_GRAINS],
    available_grains: [i32; MAX_NUM_GRAINS],
    envelope_buffer: [f32; MAX_BLOCK_SIZE],
}

impl Default for GranularSamplePlayer {
    fn default() -> Self {
        Self {
            max_num_grains: 32,
            num_midfi_grains: 24,
            num_channels: 2,
            num_grains: 0.0,
            gain_normalization: 1.0,
            grain_size_hint: 1024.0,
            grain_rate_phasor: 0.0,
            grains: [Grain::default(); MAX_NUM_GRAINS],
            available_grains: [0; MAX_NUM_GRAINS],
            envelope_buffer: [0.0; MAX_BLOCK_SIZE],
        }
    }
}

impl GranularSamplePlayer {
    pub fn init(&mut self, num_channels: i32, max_num_grains: i32) {
        self.max_num_grains = max_num_grains.clamp(1, MAX_NUM_GRAINS as i32);
        self.num_midfi_grains = 3 * self.max_num_grains / 4;
        self.gain_normalization = 1.0;
        for g in self.grains.iter_mut() {
            g.init();
        }
        self.num_grains = 0.0;
        self.num_channels = num_channels;
        self.grain_size_hint = 1024.0;
    }

    pub fn play<R: Resolution>(
        &mut self,
        buffer: &[AudioBuffer<R>; 2],
        parameters: &Parameters,
        out: &mut [[f32; 2]],
        rng: &mut Random,
    ) {
        let size = out.len();
        let mut overlap = parameters.granular.overlap;
        overlap = overlap * overlap * overlap;
        let target_num_grains = self.max_num_grains as f32 * overlap;
        let mut p = target_num_grains / self.grain_size_hint;
        let space_between_grains = self.grain_size_hint / target_num_grains;
        if parameters.granular.use_deterministic_seed {
            p = -1.0;
        } else {
            self.grain_rate_phasor = -1000.0;
        }

        // Build a list of available grains.
        let mut num_available_grains = self.fill_available_grains_list();

        // Try to schedule new grains.
        let mut seed_trigger = parameters.trigger;
        for t in 0..size {
            self.grain_rate_phasor += 1.0;
            let seed_probabilistic = rng.get_float() < p && target_num_grains > self.num_grains;
            let seed_deterministic = self.grain_rate_phasor >= space_between_grains;
            let seed = seed_probabilistic || seed_deterministic || seed_trigger;
            if num_available_grains > 0 && seed {
                num_available_grains -= 1;
                let index = self.available_grains[num_available_grains as usize] as usize;
                let quality =
                    if num_available_grains < self.num_midfi_grains { GrainQuality::Medium } else { GrainQuality::High };
                let buffer_size = buffer[0].size();
                let buffer_head = buffer[0].head() - size as i32 + t as i32;
                self.schedule_grain(index, parameters, t as i32, buffer_size, buffer_head, quality, rng);
                self.grain_rate_phasor = 0.0;
                seed_trigger = false;
            }
        }

        // Overlap grains.
        for frame in out.iter_mut() {
            *frame = [0.0; 2];
        }
        let num_channels = self.num_channels;
        for i in 0..self.max_num_grains as usize {
            let g = &mut self.grains[i];
            g.overlap_add(buffer, num_channels, out, &mut self.envelope_buffer);
        }

        // Compute normalization factor.
        let active_grains = self.max_num_grains - num_available_grains;
        slope(&mut self.num_grains, active_grains as f32, 0.9, 0.2);

        let mut gain_normalization =
            if self.num_grains > 2.0 { fast_rsqrt_carmack(self.num_grains - 1.0) } else { 1.0 };
        let window_gain = (1.0 + 2.0 * parameters.granular.window_shape).clamp(1.0, 2.0);
        gain_normalization *= crossfade(1.0, window_gain, parameters.granular.overlap);

        // Apply gain normalization.
        for frame in out.iter_mut() {
            one_pole(&mut self.gain_normalization, gain_normalization, 0.01);
            frame[0] *= self.gain_normalization;
            frame[1] *= self.gain_normalization;
        }
    }

    fn fill_available_grains_list(&mut self) -> i32 {
        let mut n = 0;
        for i in 0..self.max_num_grains as usize {
            if !self.grains[i].active() {
                self.available_grains[n] = i as i32;
                n += 1;
            }
        }
        n as i32
    }

    fn schedule_grain(
        &mut self,
        index: usize,
        parameters: &Parameters,
        pre_delay: i32,
        buffer_size: i32,
        buffer_head: i32,
        quality: GrainQuality,
        rng: &mut Random,
    ) {
        let position = parameters.position;
        let pitch = parameters.pitch;
        let window_shape = parameters.granular.window_shape;
        let mut grain_size = interpolate(&LUT_GRAIN_SIZE, parameters.size, 256.0);
        let pitch_ratio = semitones_to_ratio(pitch);
        let inv_pitch_ratio = semitones_to_ratio(-pitch);
        let pan = 0.5 + parameters.stereo_spread * (rng.get_float() - 0.5);
        let (gain_l, gain_r);
        if self.num_channels == 1 {
            gain_l = interpolate(&LUT_SIN, pan, 256.0);
            gain_r = interpolate(&LUT_SIN[256..], pan, 256.0);
        } else if pan < 0.5 {
            gain_l = 1.0;
            gain_r = 2.0 * pan;
        } else {
            gain_r = 1.0;
            gain_l = 2.0 * (1.0 - pan);
        }

        if pitch_ratio > 1.0 {
            // The grain's play-head moves faster than the buffer record-head.
            // We must make sure that the grain will not consume too much
            // data. In some situations, it might be necessary to reduce the
            // size of the grain.
            grain_size = grain_size.min(buffer_size as f32 * 0.25 * inv_pitch_ratio);
        }

        let eaten_by_play_head = grain_size * pitch_ratio;
        let eaten_by_recording_head = grain_size;

        let mut available = 0.0f32;
        available += buffer_size as f32;
        available -= eaten_by_play_head;
        available -= eaten_by_recording_head;

        let size = (grain_size as i32) & !1;
        let start = buffer_head.wrapping_sub((position * available + eaten_by_play_head) as i32);
        self.grains[index].start(
            pre_delay,
            buffer_size,
            start,
            size,
            (pitch_ratio * 65536.0) as u32 as i32,
            window_shape,
            gain_l,
            gain_r,
            quality,
        );
        one_pole(&mut self.grain_size_hint, grain_size, 0.1);
    }
}

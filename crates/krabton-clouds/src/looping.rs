//! Looping delay: a delay line that turns into a looper when frozen, with
//! tap-tempo sync from the trigger input
//! (`clouds/dsp/looping_sample_player.h`).
//!
//! Copyright 2014 Emilie Gillet, MIT license. Rust port for Krabton.

use crate::audio_buffer::{AudioBuffer, Resolution};
use crate::dsp::semitones_to_ratio;
use crate::parameters::Parameters;

pub const CROSSFADE_DURATION: f32 = 64.0;

#[derive(Default)]
pub struct LoopingSamplePlayer {
    phase: f32,
    current_delay: f32,
    loop_point: f32,
    loop_duration: f32,
    tail_start: f32,
    tail_duration: f32,
    loop_reset: f32,
    synchronized: bool,
    num_channels: i32,
    tap_delay: i32,
    tap_delay_counter: i32,
}

impl LoopingSamplePlayer {
    pub fn init(&mut self, num_channels: i32) {
        self.num_channels = num_channels;
        self.phase = 0.0;
        self.current_delay = 0.0;
        self.loop_point = 0.0;
        self.loop_duration = 0.0;
        self.tap_delay = 0;
        self.tap_delay_counter = 0;
        self.synchronized = false;
        self.tail_duration = 1.0;
    }

    #[inline]
    pub fn synchronized(&self) -> bool {
        self.synchronized
    }

    #[inline]
    fn read<R: Resolution>(&self, buffer: &[AudioBuffer<R>; 2], position: i32, out: &mut [f32; 2], gain: f32, add: bool) {
        let integral = position >> 12;
        let fractional = (position << 4) as u16;
        let l = buffer[0].read_hermite(integral, fractional) * gain;
        let r = if self.num_channels == 1 { l } else { buffer[1].read_hermite(integral, fractional) * gain };
        if add {
            out[0] += l;
            out[1] += r;
        } else {
            out[0] = l;
            out[1] = r;
        }
    }

    pub fn play<R: Resolution>(&mut self, buffer: &[AudioBuffer<R>; 2], parameters: &Parameters, out: &mut [[f32; 2]]) {
        let size = out.len() as i32;
        let buffer_size = buffer[0].size();
        let max_delay = (buffer_size as f32 - CROSSFADE_DURATION) as i32;
        self.tap_delay_counter = self.tap_delay_counter.saturating_add(size);
        if self.tap_delay_counter > max_delay {
            self.tap_delay = 0;
            self.tap_delay_counter = 0;
            self.synchronized = false;
        }
        if parameters.trigger {
            self.tap_delay = self.tap_delay_counter;
            self.tap_delay_counter = 0;
            self.synchronized = self.tap_delay > 128;
            self.loop_reset = self.phase;
            self.phase = 0.0;
        }

        if !parameters.freeze {
            let mut remaining = size;
            for frame in out.iter_mut() {
                remaining -= 1;
                let mut target_delay = parameters.position * max_delay as f32;
                if self.synchronized {
                    target_delay = self.tap_delay as f32;
                }
                let error = target_delay - self.current_delay;
                let delay = self.current_delay + 0.00005 * error;
                self.current_delay = delay;
                let mut delay_int = (buffer[0].head() - 4 - remaining + buffer_size) << 12;
                delay_int = delay_int.wrapping_sub((delay * 4096.0) as i32);
                self.read(buffer, delay_int, frame, 1.0, false);
            }
            self.phase = 0.0;
        } else {
            let mut loop_point = parameters.position * max_delay as f32 * 15.0 / 16.0;
            loop_point += CROSSFADE_DURATION;
            let d = parameters.size;
            let mut loop_duration = (0.01 + 0.99 * d * d * d) * max_delay as f32;
            if self.synchronized {
                loop_duration = self.tap_delay as f32;
            }
            if loop_point + loop_duration >= max_delay as f32 {
                loop_point = max_delay as f32 - loop_duration;
            }
            let phase_increment = if self.synchronized { 1.0 } else { semitones_to_ratio(parameters.pitch) };

            for frame in out.iter_mut() {
                if self.phase >= self.loop_duration || self.phase == 0.0 {
                    if self.phase >= self.loop_duration {
                        self.loop_reset = self.loop_duration;
                    }
                    if self.loop_reset >= self.loop_duration {
                        self.loop_reset = self.loop_duration;
                    }
                    self.tail_start = self.loop_duration - self.loop_reset + self.loop_point;
                    self.phase = 0.0;
                    self.tail_duration = CROSSFADE_DURATION.min(CROSSFADE_DURATION * phase_increment);
                    self.loop_point = loop_point;
                    self.loop_duration = loop_duration;
                }
                self.phase += phase_increment;

                let mut gain = 1.0;
                if self.tail_duration != 0.0 {
                    gain = (self.phase / self.tail_duration).clamp(0.0, 1.0);
                }
                let delay_int = (buffer[0].head() - 4 + buffer_size) << 12;
                let position =
                    delay_int.wrapping_sub(((self.loop_duration - self.phase + self.loop_point) * 4096.0) as i32);
                self.read(buffer, position, frame, gain, false);

                if gain != 1.0 {
                    let gain = 1.0 - gain;
                    let position = delay_int.wrapping_sub(((-self.phase + self.tail_start) * 4096.0) as i32);
                    self.read(buffer, position, frame, gain, true);
                }
            }
        }
    }
}

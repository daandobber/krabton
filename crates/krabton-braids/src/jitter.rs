//! Slow analog-style pitch drift (`braids/vco_jitter_source.h`, Copyright
//! 2012 Emilie Gillet, MIT).

use crate::dsp::random_word;
use crate::resources::WAV_SINE;

#[derive(Clone, Copy, Default)]
pub struct VcoJitterSource {
    phase_step: u32,
    phase: u32,
    external_temperature: i32,
    room_temperature: i32,
}

impl VcoJitterSource {
    /// Pitch offset (1/128 semitones) for a drift `intensity` (0..32767).
    #[inline]
    pub fn render(&mut self, intensity: i32) -> i16 {
        let toss = random_word() as u16;
        if toss == 0 {
            self.phase_step = self.phase_step.wrapping_mul(1664525).wrapping_add(1013904223);
            self.phase = self.phase.wrapping_add((self.phase_step >> 16).wrapping_mul(self.phase_step >> 16));
            self.external_temperature = (WAV_SINE[(self.phase >> 24) as usize] as i32) << 8;
        }
        self.room_temperature += (self.external_temperature - self.room_temperature) >> 16;
        (self.room_temperature.wrapping_mul(intensity) >> 19) as i16
    }
}

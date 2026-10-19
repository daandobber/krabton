#![allow(dead_code)]

use crate::lfo::Lfo;
use crate::modulation_envelope::ModulationEnvelope;
use crate::oscillator::Oscillator;
use crate::loop_mode::LoopMode;
use crate::region_pair::RegionPair;
use crate::shape::Shape;
use crate::soundfont_math::SoundFontMath;
use crate::volume_envelope::VolumeEnvelope;

#[allow(unused)]
#[non_exhaustive]
pub(crate) struct RegionEx {}

impl RegionEx {
    pub(crate) fn start_oscillator(oscillator: &mut Oscillator, region: &RegionPair, shape: &Shape) {
        let sample_rate = region.instrument.sample_sample_rate;
        let loop_mode = region.get_sample_modes();
        let start = region.get_sample_start();
        let end = region.get_sample_end();
        let start_loop = region.get_sample_start_loop();
        // Krabton: later into the sample, up to its loop (or most of the way to its end).
        let to = if loop_mode != LoopMode::NoLoop && start_loop > start && start_loop < end { start_loop } else { start + (end - start) * 9 / 10 };
        let start = start + ((to - start).max(0) as f32 * shape.start.clamp(0_f32, 1_f32)) as i32;
        let end_loop = region.get_sample_end_loop();
        let root_key = region.get_root_key();
        let coarse_tune = region.get_coarse_tune();
        let fine_tune = region.get_fine_tune();
        let scale_tuning = region.get_scale_tuning();

        oscillator.start(
            loop_mode,
            sample_rate,
            start,
            end,
            start_loop,
            end_loop,
            root_key,
            coarse_tune,
            fine_tune,
            scale_tuning,
        );
    }

    pub(crate) fn start_volume_envelope(
        envelope: &mut VolumeEnvelope,
        region: &RegionPair,
        key: i32,
        _velocity: i32,
        shape: &Shape,
    ) {
        // If the release time is shorter than 10 ms, it will be clamped to 10 ms to avoid pop noise.

        let delay = region.get_delay_volume_envelope();
        let attack = Shape::time(region.get_attack_volume_envelope(), shape.attack);
        let hold = region.get_hold_volume_envelope()
            * SoundFontMath::key_number_to_multiplying_factor(
                region.get_key_number_to_volume_envelope_hold(),
                key,
            );
        let decay = Shape::time(
            region.get_decay_volume_envelope()
                * SoundFontMath::key_number_to_multiplying_factor(
                    region.get_key_number_to_volume_envelope_decay(),
                    key,
                ),
            shape.decay,
        );
        let sustain = SoundFontMath::decibels_to_linear(-Shape::sustain_db(region.get_sustain_volume_envelope(), shape.sustain));
        let release = SoundFontMath::max(Shape::time(region.get_release_volume_envelope(), shape.release), 0.01_f32);

        envelope.start(delay, attack, hold, decay, sustain, release);
    }

    pub(crate) fn start_modulation_envelope(
        envelope: &mut ModulationEnvelope,
        region: &RegionPair,
        key: i32,
        velocity: i32,
    ) {
        // According to the implementation of TinySoundFont, the attack time should be adjusted by the velocity.

        let delay = region.get_delay_modulation_envelope();
        let attack = region.get_attack_modulation_envelope() * ((145 - velocity) as f32 / 144_f32);
        let hold = region.get_hold_modulation_envelope()
            * SoundFontMath::key_number_to_multiplying_factor(
                region.get_key_number_to_modulation_envelope_hold(),
                key,
            );
        let decay = region.get_decay_modulation_envelope()
            * SoundFontMath::key_number_to_multiplying_factor(
                region.get_key_number_to_modulation_envelope_decay(),
                key,
            );
        let sustain = 1_f32 - region.get_sustain_modulation_envelope() / 100_f32;
        let release = region.get_release_modulation_envelope();

        envelope.start(delay, attack, hold, decay, sustain, release);
    }

    pub(crate) fn start_vibrato(lfo: &mut Lfo, region: &RegionPair, _key: i32, _velocity: i32, shape: &Shape) {
        lfo.start(
            region.get_delay_vibrato_lfo() + shape.vib_delay.max(0_f32),
            region.get_frequency_vibrato_lfo() * shape.vib_rate.max(0.01_f32),
        );
    }

    pub(crate) fn start_modulation(lfo: &mut Lfo, region: &RegionPair, _key: i32, _velocity: i32, shape: &Shape) {
        lfo.start(
            region.get_delay_modulation_lfo(),
            region.get_frequency_modulation_lfo() * shape.mod_rate.max(0.01_f32),
        );
    }
}

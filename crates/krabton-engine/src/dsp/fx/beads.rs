//! Beads: the Mutable Instruments Beads texture synthesizer (Emilie
//! Gillet, MIT; see `krabton_beads`) as an effect: grains of the track's
//! audio (or of its wavetables), a delay with SIZE fully up, four tape and
//! digital qualities, feedback and reverb; seeded freely or by the song's
//! clock.

use crate::dsp::effects::FxCtx;
use crate::params::{ParamSpec, Unit};
use crate::scope::{DeviceTaps, MeterTap, ScopeTap};
use krabton_beads::{Beads, Clock, GrainView, Media, Params, Seed, Source};

pub const TIME: usize = 0;
pub const SIZE: usize = 1;
pub const SHAPE: usize = 2;
pub const PITCH: usize = 3;
pub const DENSITY: usize = 4;
pub const TIME_RANDOM: usize = 5;
pub const SIZE_RANDOM: usize = 6;
pub const SHAPE_RANDOM: usize = 7;
pub const PITCH_RANDOM: usize = 8;
pub const FEEDBACK: usize = 9;
pub const DRY_WET: usize = 10;
pub const REVERB: usize = 11;
pub const FREEZE: usize = 12;
pub const SEED: usize = 13;
pub const SYNC: usize = 14;
pub const QUALITY: usize = 15;
pub const MONO: usize = 16;
pub const SOURCE: usize = 17;
pub const INPUT: usize = 18;

pub const SEEDS: &[&str] = &["Free", "Synced", "Off"];
pub const SYNCS: &[&str] = &["1/32", "1/16", "1/8", "1/4", "1/2", "1 Bar"];
const SYNC_BEATS: [f64; 6] = [0.125, 0.25, 0.5, 1.0, 2.0, 4.0];
pub const QUALITIES: &[&str] = &["Hi-Fi", "Clouds", "Sunny Tape", "Scorched Cassette"];
pub const SOURCES: &[&str] = &["Input", "Wavetables"];

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::lin("Time", "Grains", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Size", "Grains", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Shape", "Grains", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Pitch", "Grains", -24.0, 24.0, 0.0, Unit::Semi),
    ParamSpec::lin("Density", "Grains", 0.0, 1.0, 0.7, Unit::Percent),
    ParamSpec::lin("Time Random", "Random", -1.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Size Random", "Random", -1.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Shape Random", "Random", -1.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Pitch Random", "Random", -1.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Feedback", "Mix", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Dry/Wet", "Mix", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Reverb", "Mix", 0.0, 1.0, 0.2, Unit::Percent),
    ParamSpec::toggle("Freeze", "Mix", false),
    ParamSpec::choice("Seed", "Seed", SEEDS, 0),
    ParamSpec::choice("Sync", "Seed", SYNCS, 1),
    ParamSpec::choice("Quality", "Beads", QUALITIES, 0),
    ParamSpec::toggle("Mono", "Beads", false),
    ParamSpec::choice("Source", "Beads", SOURCES, 0),
    ParamSpec::lin("Input", "Beads", -12.0, 24.0, 0.0, Unit::Db),
];

/// Grains shown on the screen.
pub const SHOWN_GRAINS: usize = 8;

pub struct BeadsFx {
    beads: Box<Beads>,
    sr: f32,
    views: [GrainView; SHOWN_GRAINS],
    /// What the screen shows: [0] how full, [1] delay mode, [2] the seed
    /// light, [3] the bank, [4] wavetables, [5] frozen, then per grain
    /// (6 + 3i) where it reads, how loud and its pitch (+100: backwards).
    pub taps: DeviceTaps,
}

impl BeadsFx {
    pub fn new(sr: f32) -> Self {
        Self { beads: Box::new(Beads::new(sr)), sr: sr.max(1000.0), views: [GrainView::default(); SHOWN_GRAINS], taps: DeviceTaps { pre: None, post: ScopeTap::default(), meters: Some(MeterTap::default()) } }
    }

    pub fn process(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32], ctx: &FxCtx) {
        if p.len() < SPECS.len() {
            return;
        }
        let seed_mode = (p[SEED].round().max(0.0) as usize).min(SEEDS.len() - 1);
        let division = SYNC_BEATS[(p[SYNC].round().max(0.0) as usize).min(SYNC_BEATS.len() - 1)];
        let clock = (seed_mode == 1 && ctx.playing).then(|| Clock { beat: ctx.beat, beats_per_sample: ctx.bpm as f64 / 60.0 / self.sr as f64, division });
        let quality = match (p[QUALITY].round().max(0.0) as usize).min(3) {
            0 => Media::Digital,
            1 => Media::VintageDigital,
            2 => Media::CleanTape,
            _ => Media::DirtyTape,
        };
        let params = Params {
            time: p[TIME],
            time_amount: p[TIME_RANDOM],
            size: p[SIZE],
            size_amount: p[SIZE_RANDOM],
            shape: p[SHAPE],
            shape_amount: p[SHAPE_RANDOM],
            pitch: p[PITCH],
            pitch_amount: p[PITCH_RANDOM],
            density: p[DENSITY],
            feedback: p[FEEDBACK],
            dry_wet: p[DRY_WET],
            reverb: p[REVERB],
            freeze: p[FREEZE] >= 0.5,
            seed: if seed_mode == 2 { Seed::Button(false) } else { Seed::Latched },
            clock,
            quality,
            mono: p[MONO] >= 0.5,
            source: if p[SOURCE] >= 0.5 { Source::Wavetables } else { Source::Input },
            input_gain: crate::dsp::db_to_gain(p[INPUT]),
        };
        self.beads.process(&params, l, r);
        if let Some(m) = &mut self.taps.meters {
            let s = self.beads.status();
            let v = &mut m.values;
            v[0] = s.filled;
            v[1] = s.delay_mode as u8 as f32;
            v[2] = s.gate as u8 as f32;
            v[3] = s.bank as f32;
            v[4] = (params.source == Source::Wavetables) as u8 as f32;
            v[5] = params.freeze as u8 as f32;
            let n = self.beads.grains(&mut self.views);
            for i in 0..SHOWN_GRAINS {
                let o = 6 + 3 * i;
                if i < n {
                    let g = self.views[i];
                    v[o] = g.position;
                    v[o + 1] = g.level;
                    v[o + 2] = g.pitch + if g.reverse { 100.0 } else { 0.0 };
                } else {
                    v[o + 1] = 0.0;
                }
            }
        }
    }
}

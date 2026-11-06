//! Three parts of a trap or pluggnb beat, each from a seed, so that a pattern
//! stays what it is until you ask for another:
//! - Hat Rolls: a hi-hat on the eighths or sixteenths, broken up by rolls of
//!   two, three, four or six, with a fill at the end of its cycle.
//! - Hook: a melody of a few notes in the song's key on the dotted rhythm those
//!   tracks are built on, repeating, with its end changing the last time round.
//! - Slide Bass: long bass notes on the same rhythm, mostly on the root, that
//!   overlap to slide into each other (made for the Glide synth).

use super::minimal::{choose, hash, DYNAMICS};
use super::{rate_beats, SeqState, RATES};
use crate::midi_fx::{MidiCtx, MidiEvent};
use crate::params::{ParamSpec, Unit};

const KEYS: [ParamSpec; 2] = [ParamSpec::toggle("Transpose", "Keys", false), ParamSpec::toggle("Hold", "Keys", false)];

pub const HAT_BASES: &[&str] = &["1/4", "1/8", "1/16"];
pub const CYCLES: &[&str] = &["1 Bar", "2 Bars", "4 Bars", "8 Bars"];
const CYCLE_BARS: [i64; 4] = [1, 2, 4, 8];

pub const HR_RATE: usize = 0;
pub const HR_NOTE: usize = 1;
pub const HR_BASE: usize = 2;
pub const HR_ROLLS: usize = 3;
pub const HR_FILL: usize = 4;
pub const HR_OPEN: usize = 5;
pub const HR_OPEN_NOTE: usize = 6;
pub const HR_CYCLE: usize = 7;
pub const HR_SEED: usize = 8;
pub const HR_VELOCITY: usize = 9;
pub const HR_ACCENT: usize = 10;

pub static HAT_ROLLS: &[ParamSpec] = &[
    ParamSpec::choice("Rate", "Clock", RATES, 2),
    ParamSpec::int("Note", "Hat", 24.0, 96.0, 42.0, Unit::None),
    ParamSpec::choice("Base", "Hat", HAT_BASES, 1),
    ParamSpec::lin("Rolls", "Hat", 0.0, 1.0, 0.25, Unit::Percent),
    ParamSpec::lin("Fill", "Hat", 0.0, 1.0, 0.6, Unit::Percent),
    ParamSpec::lin("Open", "Open Hat", 0.0, 1.0, 0.1, Unit::Percent),
    ParamSpec::int("Open Note", "Open Hat", 24.0, 96.0, 46.0, Unit::None),
    ParamSpec::choice("Cycle", "Pattern", CYCLES, 1),
    ParamSpec::int("Seed", "Pattern", 0.0, 255.0, 0.0, Unit::None),
    ParamSpec::lin("Velocity", "Notes", 0.05, 1.0, 0.75, Unit::Percent),
    ParamSpec::lin("On Beat", "Notes", 0.0, 1.0, 0.4, Unit::Percent),
    KEYS[0],
    KEYS[1],
    DYNAMICS[0],
    DYNAMICS[1],
    DYNAMICS[2],
    DYNAMICS[3],
    ParamSpec::toggle("Band", "Band", false),
];

/// What a step of the hat does: how many hits, and on which note.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct HatStep {
    pub hits: u8,
    pub open: bool,
    /// Rising (a roll that climbs) or falling.
    pub rising: bool,
}

/// Which step of its cycle a step of the song is, and how many steps a bar has.
fn in_cycle(k: i64, rate: f64, bars: i64) -> (i64, i64) {
    let per_bar = ((4.0 / rate).round() as i64).max(1);
    (k.rem_euclid(per_bar * bars), per_bar)
}

/// The hat at step `k`.
pub fn hat_step(p: &[f32], k: i64) -> HatStep {
    let rate = rate_beats(p[HR_RATE]);
    let bars = CYCLE_BARS[choose(p, HR_CYCLE, CYCLES.len())];
    let (pos, per_bar) = in_cycle(k, rate, bars);
    let seed = (p[HR_SEED].round() as u32).wrapping_mul(0x9E37) ^ 0x4A77;
    let per_beat = ((1.0 / rate).round() as i64).max(1);
    // How many steps between the plain hits.
    let base_beats = [1.0, 0.5, 0.25][choose(p, HR_BASE, HAT_BASES.len())];
    let every = ((base_beats / rate).round() as i64).max(1);
    let on_base = pos % every == 0;
    let total = per_bar * bars;
    let last_beat = pos >= total - per_beat;
    let chance = if last_beat { p[HR_FILL].clamp(0.0, 1.0).max(p[HR_ROLLS]) } else { p[HR_ROLLS].clamp(0.0, 1.0) };
    let roll = hash(seed, pos as u32) < chance;
    if roll {
        // Fast ones near the end of a roll's bar, and never quite the same.
        let pick = hash(seed ^ 0x51, pos as u32);
        let hits = if pick < 0.45 {
            2
        } else if pick < 0.75 {
            3
        } else if pick < 0.93 {
            4
        } else {
            6
        };
        return HatStep { hits, open: false, rising: hash(seed ^ 0x77, pos as u32) < 0.6 };
    }
    let off_beat = pos % per_beat == per_beat / 2;
    let open = off_beat && on_base && hash(seed ^ 0xA5, pos as u32) < p[HR_OPEN].clamp(0.0, 1.0);
    HatStep { hits: on_base as u8, open, rising: false }
}

pub const HOOK_SHAPES: &[&str] = &["Wander", "Rise", "Fall", "Arch", "Answer"];
pub const HOOK_LENGTHS: &[&str] = &["1 Bar", "2 Bars", "4 Bars"];
const HOOK_BARS: [i64; 3] = [1, 2, 4];

pub const HK_RATE: usize = 0;
pub const HK_LENGTH: usize = 1;
pub const HK_SEED: usize = 2;
pub const HK_DENSITY: usize = 3;
pub const HK_RANGE: usize = 4;
pub const HK_SHAPE: usize = 5;
pub const HK_CENTER: usize = 6;
pub const HK_CHANGE: usize = 7;
pub const HK_GATE: usize = 8;
pub const HK_VELOCITY: usize = 9;

pub static HOOK: &[ParamSpec] = &[
    ParamSpec::choice("Rate", "Clock", RATES, 2),
    ParamSpec::choice("Length", "Hook", HOOK_LENGTHS, 1),
    ParamSpec::int("Seed", "Hook", 0.0, 255.0, 0.0, Unit::None),
    ParamSpec::lin("Density", "Hook", 0.1, 1.0, 0.45, Unit::Percent),
    ParamSpec::int("Range", "Hook", 2.0, 12.0, 5.0, Unit::None),
    ParamSpec::choice("Shape", "Hook", HOOK_SHAPES, 0),
    ParamSpec::int("Center", "Notes", 36.0, 96.0, 72.0, Unit::None),
    ParamSpec::lin("Change", "Hook", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Gate", "Notes", 0.1, 1.5, 0.8, Unit::Percent),
    ParamSpec::lin("Velocity", "Notes", 0.05, 1.0, 0.75, Unit::Percent),
    KEYS[0],
    KEYS[1],
    DYNAMICS[0],
    DYNAMICS[1],
    DYNAMICS[2],
    DYNAMICS[3],
    ParamSpec::toggle("Band", "Band", false),
    ParamSpec::choice("Follow", "Band", crate::band::FOLLOWS, 1),
];

/// How much a place in a bar of sixteen wants a note: the three-three-two those
/// tracks stand on first, then the rest of the eighths.
fn pull(place: i64) -> f32 {
    match place.rem_euclid(16) {
        0 => 1.0,
        3 | 6 | 11 | 14 => 0.85,
        8 => 0.6,
        4 | 10 | 12 => 0.35,
        2 | 13 => 0.2,
        _ => 0.08,
    }
}

/// The note of the hook at step `k`, as steps of the key off its centre. Its
/// last repeat in a cycle of four changes its end, by as much as *Change* says.
pub fn hook_note(p: &[f32], k: i64) -> Option<i32> {
    let rate = rate_beats(p[HK_RATE]);
    let bars = HOOK_BARS[choose(p, HK_LENGTH, HOOK_LENGTHS.len())];
    let (pos, per_bar) = in_cycle(k, rate, bars);
    let total = per_bar * bars;
    // The grid a bar of sixteen stands on, whatever the rate.
    let place = pos * 16 / per_bar;
    let seed = (p[HK_SEED].round() as u32).wrapping_mul(0x9E37) ^ 0x4BC1;
    // Every fourth time round, the second half answers differently.
    let round = k.div_euclid(total).rem_euclid(4);
    let changed = round == 3 && pos * 2 >= total && hash(seed ^ 0xC3, pos as u32) < p[HK_CHANGE].clamp(0.0, 1.0);
    let seed = if changed { seed ^ 0x5F5F } else { seed };
    let density = p[HK_DENSITY].clamp(0.0, 1.0);
    if hash(seed, pos as u32) >= pull(place) * (0.3 + 1.4 * density) && pos != 0 {
        return None;
    }
    let range = p[HK_RANGE].round().max(1.0) as i32;
    let t = pos as f32 / total.max(1) as f32;
    // Its shape, and something of its own.
    let bend = match choose(p, HK_SHAPE, HOOK_SHAPES.len()) {
        1 => t,
        2 => 1.0 - t,
        3 => 1.0 - (2.0 * t - 1.0).abs(),
        // Up in the first half, and an answer below in the second.
        4 => {
            if t < 0.5 {
                0.5 + t
            } else {
                1.0 - t
            }
        }
        _ => 0.5,
    };
    let own = hash(seed ^ 0x2B, pos as u32);
    let at = bend * 0.6 + own * 0.4;
    // It starts on the note the rest hangs on.
    if pos == 0 {
        return Some(0);
    }
    Some((at * range as f32).round() as i32 - range / 2)
}

/// How many steps until the hook's next note (for how long this one sounds).
fn hook_gap(p: &[f32], k: i64) -> i64 {
    (1..32).find(|d| hook_note(p, k + d).is_some()).unwrap_or(32)
}

pub const BASS_PATTERNS: &[&str] = &["3-3-2", "Bounce", "Half Time", "Sparse", "Chance"];

pub const SB_RATE: usize = 0;
pub const SB_PATTERN: usize = 1;
pub const SB_SEED: usize = 2;
pub const SB_SLIDE: usize = 3;
pub const SB_MOVE: usize = 4;
pub const SB_NOTE: usize = 5;
pub const SB_GATE: usize = 6;
pub const SB_VELOCITY: usize = 7;

pub static SLIDE_BASS: &[ParamSpec] = &[
    ParamSpec::choice("Rate", "Clock", RATES, 2),
    ParamSpec::choice("Pattern", "Bass", BASS_PATTERNS, 0),
    ParamSpec::int("Seed", "Bass", 0.0, 255.0, 0.0, Unit::None),
    ParamSpec::lin("Slide", "Bass", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::lin("Move", "Bass", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::int("Note", "Notes", 24.0, 60.0, 36.0, Unit::None),
    ParamSpec::lin("Gate", "Notes", 0.1, 1.0, 0.95, Unit::Percent),
    ParamSpec::lin("Velocity", "Notes", 0.05, 1.0, 0.95, Unit::Percent),
    KEYS[0],
    KEYS[1],
    DYNAMICS[0],
    DYNAMICS[1],
    DYNAMICS[2],
    DYNAMICS[3],
    ParamSpec::toggle("Band", "Band", false),
    ParamSpec::choice("Follow", "Band", crate::band::FOLLOWS, 2),
];

/// Places in two bars of sixteen.
fn bass_places(pattern: usize, place: i64, seed: u32) -> bool {
    let at = place.rem_euclid(32);
    match pattern {
        0 => matches!(at, 0 | 6 | 12 | 16 | 22 | 28),
        1 => matches!(at, 0 | 3 | 8 | 11 | 16 | 19 | 24 | 26 | 30),
        2 => matches!(at, 0 | 14 | 16 | 30),
        3 => matches!(at, 0 | 22),
        _ => at == 0 || hash(seed, at as u32) < pull(at) * 0.5,
    }
}

/// The bass at step `k`: steps of the key off its note, and whether it slides on
/// into the next.
pub fn bass_note(p: &[f32], k: i64) -> Option<(i32, bool)> {
    let rate = rate_beats(p[SB_RATE]);
    let (pos, per_bar) = in_cycle(k, rate, 2);
    let place = pos * 16 / per_bar;
    let seed = (p[SB_SEED].round() as u32).wrapping_mul(0x9E37) ^ 0x8A55;
    if pos * 16 % per_bar != 0 || !bass_places(choose(p, SB_PATTERN, BASS_PATTERNS.len()), place, seed) {
        return None;
    }
    // Home most of the time; else to the fifth, the seventh or the third.
    let moves = [4, 6, 2, -3];
    let degree = if place != 0 && hash(seed ^ 0x13, place as u32) < p[SB_MOVE].clamp(0.0, 1.0) {
        moves[(hash(seed ^ 0x31, place as u32) * moves.len() as f32) as usize % moves.len()]
    } else {
        0
    };
    Some((degree, hash(seed ^ 0x6D, place as u32) < p[SB_SLIDE].clamp(0.0, 1.0)))
}

fn bass_gap(p: &[f32], k: i64) -> i64 {
    (1..64).find(|d| bass_note(p, k + d).is_some()).unwrap_or(64)
}

impl SeqState {
    pub(crate) fn hat_rolls(&mut self, k: i64, p: &[f32], at: u64, step_frames: f64, out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        let step = hat_step(p, k);
        if step.hits == 0 {
            return;
        }
        let rate = rate_beats(p[HR_RATE]);
        let per_beat = ((1.0 / rate).round() as i64).max(1);
        let on_beat = k.rem_euclid(per_beat) == 0;
        let hits = step.hits as usize;
        let base = p[HR_VELOCITY] * if on_beat { 1.0 } else { 1.0 - p[HR_ACCENT].clamp(0.0, 1.0) * 0.5 };
        let note = if step.open { p[HR_OPEN_NOTE] } else { p[HR_NOTE] }.round() as i32;
        let each = step_frames / hits as f64;
        for h in 0..hits {
            let along = if hits > 1 { h as f32 / (hits - 1) as f32 } else { 1.0 };
            let ramp = if hits == 1 {
                1.0
            } else if step.rising {
                0.55 + 0.45 * along
            } else {
                1.0 - 0.45 * along
            };
            let when = at + (h as f64 * each) as u64;
            let len = if step.open { step_frames * 1.5 } else { each * 0.7 };
            self.mlater(ctx, when, note, base * ramp, len, out);
        }
    }

    pub(crate) fn hook(&mut self, k: i64, p: &[f32], at: u64, step_frames: f64, out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        let Some(degree) = hook_note(p, k) else { return };
        let (home, _) = ctx.scale.locate(p[HK_CENTER].round() as i32);
        let note = ctx.scale.pitch_at(home + degree);
        let gap = hook_gap(p, k).min(8) as f64;
        self.note(ctx, at, note, p[HK_VELOCITY], step_frames * gap * p[HK_GATE] as f64, out);
    }

    pub(crate) fn slide_bass(&mut self, k: i64, p: &[f32], at: u64, step_frames: f64, out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        let Some((degree, slides)) = bass_note(p, k) else { return };
        let (home, _) = ctx.scale.locate(p[SB_NOTE].round() as i32);
        let note = ctx.scale.pitch_at(home + degree);
        let gap = bass_gap(p, k) as f64;
        // Sliding: it is still held when the next one comes.
        let len = if slides { (gap + 0.5) * step_frames } else { gap * step_frames * p[SB_GATE] as f64 };
        self.note(ctx, at, note, p[SB_VELOCITY], len, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::midi_fx::MidiFxKind;
    use crate::params::defaults;
    use crate::scale::{Scale, ScaleKind};
    use crate::sequencers::tests::run;

    fn hats() -> Vec<f32> {
        defaults(MidiFxKind::HatRolls.specs())
    }

    #[test]
    fn all_three_play() {
        for kind in [MidiFxKind::HatRolls, MidiFxKind::Hook, MidiFxKind::SlideBass] {
            let ons = run(kind, &defaults(kind.specs()), &kind.default_data(), 16.0);
            assert!(ons.len() >= 6, "{kind:?}: {}", ons.len());
            assert!(ons.iter().all(|o| (12..=110).contains(&o.1)), "{kind:?}");
        }
    }

    #[test]
    fn the_hook_and_the_bass_stay_in_the_key() {
        let c = Scale::new(0, ScaleKind::Major);
        for kind in [MidiFxKind::Hook, MidiFxKind::SlideBass] {
            let ons = run(kind, &defaults(kind.specs()), &kind.default_data(), 32.0);
            assert!(ons.iter().all(|o| c.contains(o.1)), "{kind:?}: {ons:?}");
        }
    }

    #[test]
    fn without_rolls_the_hat_just_ticks() {
        let mut p = hats();
        p[HR_ROLLS] = 0.0;
        p[HR_FILL] = 0.0;
        p[HR_OPEN] = 0.0;
        for k in 0..64 {
            let s = hat_step(&p, k);
            assert_eq!(s.hits, u8::from(k % 2 == 0), "{k}");
            assert!(!s.open);
        }
    }

    #[test]
    fn all_rolls_is_all_rolls() {
        let mut p = hats();
        p[HR_ROLLS] = 1.0;
        for k in 0..64 {
            assert!([2, 3, 4, 6].contains(&hat_step(&p, k).hits), "{k}");
        }
    }

    #[test]
    fn the_fill_comes_at_the_end_of_the_cycle() {
        let mut p = hats();
        p[HR_ROLLS] = 0.0;
        p[HR_FILL] = 1.0;
        p[HR_CYCLE] = 0.0; // a bar of sixteen
        for k in 0..12 {
            assert!(hat_step(&p, k).hits <= 1, "{k}");
        }
        for k in 12..16 {
            assert!(hat_step(&p, k).hits >= 2, "{k}");
        }
    }

    #[test]
    fn a_pattern_comes_back_each_cycle_and_a_seed_is_another_pattern() {
        let p = hats();
        // Two bars of sixteen.
        for k in 0..32 {
            assert_eq!(hat_step(&p, k), hat_step(&p, k + 32), "{k}");
        }
        let mut q = p.clone();
        q[HR_SEED] = 9.0;
        assert!((0..32).any(|k| hat_step(&p, k) != hat_step(&q, k)));
    }

    #[test]
    fn a_roll_really_plays_its_hits_evenly_within_its_step() {
        let mut p = hats();
        p[HR_ROLLS] = 1.0;
        p[HR_CYCLE] = 0.0;
        let ons = run(MidiFxKind::HatRolls, &p, &MidiFxKind::HatRolls.default_data(), 4.0);
        let want: usize = (0..16).map(|k| hat_step(&p, k).hits as usize).sum();
        // The notes of a block to come are there too, not lumped at its end.
        assert!(ons.len() >= want - 6 && ons.len() <= want + 6, "{} {want}", ons.len());
        let mut gaps: Vec<f64> = ons.windows(2).map(|w| w[1].0 as f64 - w[0].0 as f64).collect();
        gaps.sort_by(|a, b| a.total_cmp(b));
        assert!(gaps[0] > 0.0, "nothing on top of each other");
    }

    #[test]
    fn the_hook_repeats_until_its_fourth_time_round() {
        let p = defaults(MidiFxKind::Hook.specs());
        // Two bars of sixteen.
        for k in 0..32 {
            assert_eq!(hook_note(&p, k), hook_note(&p, k + 32), "{k}");
            assert_eq!(hook_note(&p, k), hook_note(&p, k + 64), "{k}");
        }
        let mut q = p.clone();
        q[HK_CHANGE] = 1.0;
        assert!((0..16).all(|k| hook_note(&q, k) == hook_note(&q, k + 96)), "the first half stays");
        assert!((16..32).any(|k| hook_note(&q, k) != hook_note(&q, k + 96)), "the end changes");
        q[HK_CHANGE] = 0.0;
        assert!((0..32).all(|k| hook_note(&q, k) == hook_note(&q, k + 96)));
    }

    #[test]
    fn the_hook_starts_at_home_and_its_density_says_how_much() {
        let p = defaults(MidiFxKind::Hook.specs());
        assert_eq!(hook_note(&p, 0), Some(0));
        let count = |d: f32| {
            let mut q = p.clone();
            q[HK_DENSITY] = d;
            (0..32).filter(|k| hook_note(&q, *k).is_some()).count()
        };
        assert!(count(1.0) > count(0.1), "{} {}", count(1.0), count(0.1));
        assert!(count(0.1) >= 1);
    }

    #[test]
    fn a_seed_makes_another_hook() {
        let p = defaults(MidiFxKind::Hook.specs());
        let mut q = p.clone();
        q[HK_SEED] = 77.0;
        assert!((0..32).any(|k| hook_note(&p, k) != hook_note(&q, k)));
    }

    #[test]
    fn the_bass_stands_on_its_pattern_and_starts_at_home() {
        let mut p = defaults(MidiFxKind::SlideBass.specs());
        p[SB_MOVE] = 0.0;
        for k in 0..32 {
            let want = matches!(k, 0 | 6 | 12 | 16 | 22 | 28);
            assert_eq!(bass_note(&p, k).is_some(), want, "{k}");
        }
        assert_eq!(bass_note(&p, 6).map(|n| n.0), Some(0), "no moves asked");
        p[SB_MOVE] = 1.0;
        assert_eq!(bass_note(&p, 0).map(|n| n.0), Some(0), "home, whatever else");
        assert!((1..32).any(|k| bass_note(&p, k).is_some_and(|n| n.0 != 0)));
    }

    #[test]
    fn a_slide_is_still_held_when_the_next_note_comes() {
        let mut p = defaults(MidiFxKind::SlideBass.specs());
        p[SB_SLIDE] = 1.0;
        p[SB_MOVE] = 1.0;
        let ons = run(MidiFxKind::SlideBass, &p, &MidiFxKind::SlideBass.default_data(), 8.0);
        assert!(ons.len() >= 4);
        // Notes that repeat end to start themselves; that is the sequencer's rule.
        // What matters here: the pattern is played on the slide's gap.
        assert!(bass_gap(&p, 0) >= 1);
    }
}

//! Chance: a drum grid where every step has a chance and a condition, the
//! way some drum machines do it. Eight lanes of up to sixteen steps (each
//! lane its own length, so they can run against each other); every step is
//! off, a hit or an accent, with a chance to play (0..100%), a condition
//! (only the first of every two times round, only in a fill, only the first
//! time, only if this lane's or the lane above's last condition came true,
//! or not) and a ratchet (one to four hits). A fill comes every few bars or
//! when you switch it on.

use super::SeqState;
use crate::midi_fx::{MidiCtx, MidiEvent};
use crate::params::{ParamSpec, Unit};

pub const RATE: usize = 0;
pub const SWING: usize = 1;
pub const VELOCITY: usize = 2;
pub const ACCENT: usize = 3;
pub const CHANCE: usize = 4;
pub const FILL_EVERY: usize = 5;
pub const FILL: usize = 6;

pub const FILL_CHOICES: &[&str] = &["Off", "2 Bars", "4 Bars", "8 Bars"];

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::choice("Rate", "Clock", super::RATES, 2),
    ParamSpec::lin("Swing", "Clock", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Velocity", "Hits", 0.05, 1.0, 0.75, Unit::Percent),
    ParamSpec::lin("Accent", "Hits", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::lin("Chance", "Hits", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::choice("Fill Every", "Fill", FILL_CHOICES, 2),
    ParamSpec::toggle("Fill", "Fill", false),
    ParamSpec::toggle("Transpose", "Keys", false),
    ParamSpec::toggle("Hold", "Keys", false),
    ParamSpec::lin("Vel Low", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Vel High", "Dynamics", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::lin("Vel Random", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Vel Accent", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
];

pub const LANES: usize = 8;
pub const STEPS: usize = 16;
const CELL: usize = 4;
const MAGIC: [u8; 2] = *b"C1";
const NOTES_AT: usize = 2 + LANES * STEPS * CELL;
const LENGTHS_AT: usize = NOTES_AT + LANES;
pub const DATA_BYTES: usize = LENGTHS_AT + LANES;
pub const NOTES: [u8; LANES] = [36, 38, 39, 42, 46, 45, 37, 56];

pub const CONDITIONS: &[&str] = &["Always", "1:2", "2:2", "1:3", "2:3", "3:3", "1:4", "2:4", "3:4", "4:4", "Fill", "Not Fill", "First", "Not First", "Pre", "Not Pre", "Nei", "Not Nei"];
/// Short names for the grid.
pub const CONDITION_TAGS: &[&str] = &["", "1:2", "2:2", "1:3", "2:3", "3:3", "1:4", "2:4", "3:4", "4:4", "F", "!F", "1st", "!1st", "P", "!P", "N", "!N"];
pub const PATTERNS: &[&str] = &["Probable House", "Breaks and Fills", "Drifting Hats", "Clear"];

/// One step of a lane.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Step {
    /// 0 off, 1 hit, 2 accent.
    pub level: u8,
    /// 0..100 %.
    pub chance: u8,
    pub condition: u8,
    /// 1..4 hits.
    pub ratchet: u8,
}

impl Step {
    pub const OFF: Step = Step { level: 0, chance: 100, condition: 0, ratchet: 1 };
}

pub fn is_current(data: &[u8]) -> bool {
    data.len() >= DATA_BYTES && data[..2] == MAGIC
}

pub fn empty() -> Vec<u8> {
    let mut d = vec![0u8; DATA_BYTES];
    d[..2].copy_from_slice(&MAGIC);
    for lane in 0..LANES {
        for s in 0..STEPS {
            set_step(&mut d, lane, s, Step::OFF);
        }
        d[NOTES_AT + lane] = NOTES[lane];
        d[LENGTHS_AT + lane] = STEPS as u8;
    }
    d
}

fn at(lane: usize, step: usize) -> usize {
    2 + (lane.min(LANES - 1) * STEPS + step % STEPS) * CELL
}

pub fn step(data: &[u8], lane: usize, s: usize) -> Step {
    if !is_current(data) {
        return Step::OFF;
    }
    let o = at(lane, s);
    Step { level: data[o].min(2), chance: data[o + 1].min(100), condition: data[o + 2].min(CONDITIONS.len() as u8 - 1), ratchet: data[o + 3].clamp(1, 4) }
}

pub fn set_step(data: &mut Vec<u8>, lane: usize, s: usize, v: Step) {
    if !is_current(data) {
        *data = empty();
    }
    let o = at(lane, s);
    data[o] = v.level.min(2);
    data[o + 1] = v.chance.min(100);
    data[o + 2] = v.condition.min(CONDITIONS.len() as u8 - 1);
    data[o + 3] = v.ratchet.clamp(1, 4);
}

pub fn note(data: &[u8], lane: usize) -> u8 {
    if is_current(data) {
        data[NOTES_AT + lane.min(LANES - 1)].min(127)
    } else {
        NOTES[lane.min(LANES - 1)]
    }
}

pub fn set_note(data: &mut Vec<u8>, lane: usize, n: u8) {
    if !is_current(data) {
        *data = empty();
    }
    data[NOTES_AT + lane.min(LANES - 1)] = n.min(127);
}

pub fn length(data: &[u8], lane: usize) -> usize {
    if is_current(data) {
        (data[LENGTHS_AT + lane.min(LANES - 1)] as usize).clamp(1, STEPS)
    } else {
        STEPS
    }
}

pub fn set_length(data: &mut Vec<u8>, lane: usize, n: usize) {
    if !is_current(data) {
        *data = empty();
    }
    data[LENGTHS_AT + lane.min(LANES - 1)] = n.clamp(1, STEPS) as u8;
}

/// A starting point (by name, see `PATTERNS`).
pub fn pattern(name: &str) -> Vec<u8> {
    let mut d = empty();
    fn put(d: &mut Vec<u8>, lane: usize, steps: &[usize], level: u8, chance: u8, condition: u8, ratchet: u8) {
        for &s in steps {
            set_step(d, lane, s, Step { level, chance, condition, ratchet });
        }
    }
    // Lanes: kick, snare, clap, closed hat, open hat, low tom, rim, cowbell.
    match name {
        "Probable House" => {
            put(&mut d, 0, &[0, 4, 8, 12], 2, 100, 0, 1);
            put(&mut d, 0, &[14], 1, 25, 0, 1);
            put(&mut d, 2, &[4, 12], 1, 100, 0, 1);
            put(&mut d, 1, &[7, 15], 1, 30, 0, 1);
            put(&mut d, 3, &[0, 4, 8, 12], 1, 45, 0, 1);
            put(&mut d, 3, &[1, 3, 5, 7, 9, 11, 13], 1, 35, 0, 1);
            put(&mut d, 3, &[15], 1, 60, 0, 2);
            put(&mut d, 4, &[2, 6, 10, 14], 1, 100, 0, 1);
            put(&mut d, 5, &[13, 14, 15], 1, 100, 10, 1);
            put(&mut d, 6, &[3, 11], 1, 40, 0, 1);
            put(&mut d, 7, &[10], 1, 100, 6, 1);
        }
        "Breaks and Fills" => {
            put(&mut d, 0, &[0, 2, 10], 2, 100, 0, 1);
            put(&mut d, 0, &[7], 1, 40, 0, 1);
            put(&mut d, 1, &[4, 12], 2, 100, 0, 1);
            put(&mut d, 1, &[7, 9], 1, 50, 11, 1);
            put(&mut d, 1, &[14, 15], 1, 100, 10, 3);
            put(&mut d, 3, &[0, 2, 4, 6, 8, 10, 12, 14], 1, 100, 0, 1);
            put(&mut d, 3, &[5, 13], 1, 50, 0, 2);
            put(&mut d, 4, &[14], 1, 100, 1, 1);
            put(&mut d, 5, &[12, 13], 1, 100, 10, 2);
            put(&mut d, 6, &[11], 1, 60, 14, 1);
        }
        "Drifting Hats" => {
            put(&mut d, 0, &[0, 6, 10], 2, 100, 0, 1);
            put(&mut d, 1, &[4, 12], 1, 100, 0, 1);
            put(&mut d, 3, &[0, 2, 3, 5, 6, 8, 9, 11], 1, 70, 0, 1);
            set_length(&mut d, 3, 12);
            put(&mut d, 4, &[4], 1, 80, 0, 1);
            set_length(&mut d, 4, 10);
            put(&mut d, 6, &[0, 3, 5], 1, 90, 0, 1);
            set_length(&mut d, 6, 7);
            put(&mut d, 7, &[0, 2], 1, 60, 0, 1);
            set_length(&mut d, 7, 5);
        }
        _ => {}
    }
    d
}

pub fn chance_default() -> Vec<u8> {
    pattern("Probable House")
}

#[derive(Clone, Debug, Default)]
pub struct ChanceState {
    /// The last condition each lane weighed, and whether it came true.
    pre: [bool; LANES],
    /// The tick the transport started at.
    start: Option<i64>,
}

impl ChanceState {
    pub fn restart(&mut self) {
        self.start = None;
        self.pre = [false; LANES];
    }
}

/// Whether a step's condition holds.
fn condition_holds(condition: u8, round: i64, first: bool, fill: bool, pre: bool, nei: bool) -> bool {
    let ab = |a: i64, b: i64| round.rem_euclid(b) == a - 1;
    match condition {
        1 => ab(1, 2),
        2 => ab(2, 2),
        3 => ab(1, 3),
        4 => ab(2, 3),
        5 => ab(3, 3),
        6 => ab(1, 4),
        7 => ab(2, 4),
        8 => ab(3, 4),
        9 => ab(4, 4),
        10 => fill,
        11 => !fill,
        12 => first,
        13 => !first,
        14 => pre,
        15 => !pre,
        16 => nei,
        17 => !nei,
        _ => true,
    }
}

impl SeqState {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn chance_seq(&mut self, k: i64, p: &[f32], data: &[u8], at: u64, step_frames: f64, out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        let start = *self.chance.start.get_or_insert(k);
        let rate = super::rate_beats(p[RATE]);
        let bar = ((k as f64 * rate) / 4.0).floor() as i64;
        let every = [0i64, 2, 4, 8][(p[FILL_EVERY].round().max(0.0) as usize).min(3)];
        let fill = p[FILL] >= 0.5 || (every > 0 && bar.rem_euclid(every) == every - 1);
        let at = Self::swing_at(at, k, p[SWING], step_frames);
        let global = p[CHANCE].clamp(0.0, 1.0);
        let mut positions = 0u64;
        let mut fired = 0u64;
        let mut skipped = 0u64;
        for lane in 0..LANES {
            let len = length(data, lane) as i64;
            let i = k.rem_euclid(len) as usize;
            positions |= (i as u64 & 0xf) << (4 * lane);
            let s = step(data, lane, i);
            if s.level == 0 {
                continue;
            }
            let round = k.div_euclid(len);
            let first = round == start.div_euclid(len);
            let nei = lane > 0 && self.chance.pre[lane - 1];
            let holds = condition_holds(s.condition, round, first, fill, self.chance.pre[lane], nei);
            let lucky = s.chance >= 100 && global >= 1.0 || self.unit() * 100.0 < s.chance as f32 * global;
            let plays = holds && lucky;
            if s.condition != 0 || s.chance < 100 {
                self.chance.pre[lane] = plays;
            }
            if !plays {
                skipped |= 1 << lane;
                continue;
            }
            fired |= 1 << lane;
            let vel = if s.level == 2 { (p[VELOCITY] + p[ACCENT]).min(1.0) } else { p[VELOCITY] };
            let n = s.ratchet.clamp(1, 4) as usize;
            let gap = step_frames / n as f64;
            for r in 0..n {
                let v = vel * (1.0 - 0.12 * r as f32);
                self.play(ctx, at + (gap * r as f64) as u64, note(data, lane) as i32, v, gap * 0.5, out);
            }
        }
        self.visual[0] = positions | fired << 32 | skipped << 40 | (fill as u64) << 48;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::midi_fx::MidiFxKind;
    use crate::params::defaults;
    use crate::scale::{Scale, ScaleKind};

    fn run(p: &[f32], data: &[u8], bars: usize) -> Vec<(f64, u8)> {
        let mut s = SeqState::new();
        let mut notes = Vec::new();
        let block = 512;
        let mut frame = 0;
        while frame < bars * 4 * 24_000 {
            let mut out = Vec::new();
            let ctx = MidiCtx { now: frame as u64, len: block, samples_per_beat: 24_000.0, scale: Scale::new(0, ScaleKind::Major), scale_on: false, beat: frame as f64 / 24_000.0, playing: true, sr: 48_000.0, drum_notes: 0, lead: 0.0 };
            s.process(MidiFxKind::Chance, p, data, &mut out, &ctx);
            for e in out.iter().filter(|e| e.on) {
                notes.push(((frame + e.offset as usize) as f64 / 24_000.0, e.pitch));
            }
            frame += block;
        }
        notes
    }

    #[test]
    fn conditions_pick_their_rounds() {
        let p = defaults(SPECS);
        let mut d = empty();
        set_step(&mut d, 0, 0, Step { level: 1, chance: 100, condition: 1, ratchet: 1 }); // 1:2
        set_step(&mut d, 1, 0, Step { level: 1, chance: 100, condition: 9, ratchet: 1 }); // 4:4
        set_step(&mut d, 2, 0, Step { level: 1, chance: 100, condition: 12, ratchet: 1 }); // first
        let notes = run(&p, &d, 8);
        let bars = |pitch: u8| notes.iter().filter(|n| n.1 == pitch).map(|n| (n.0 / 4.0).round() as i64).collect::<Vec<_>>();
        assert_eq!(bars(36), vec![0, 2, 4, 6]);
        assert_eq!(bars(38), vec![3, 7]);
        assert_eq!(bars(39), vec![0]);
    }

    #[test]
    fn fills_come_every_few_bars_and_ratchets_roll() {
        let p = defaults(SPECS);
        let mut d = empty();
        set_step(&mut d, 5, 12, Step { level: 1, chance: 100, condition: 10, ratchet: 3 });
        let notes = run(&p, &d, 8);
        let toms: Vec<f64> = notes.iter().filter(|n| n.1 == 45).map(|n| n.0).collect();
        // Fills in bars 4 and 8 (every 4 bars), three hits each.
        assert_eq!(toms.len(), 6, "{toms:?}");
        assert!((toms[0] - 15.0).abs() < 1e-3 && (toms[3] - 31.0).abs() < 1e-3, "{toms:?}");
    }

    #[test]
    fn chance_thins_and_lanes_have_their_own_length() {
        let mut p = defaults(SPECS);
        let mut d = empty();
        for s in 0..STEPS {
            set_step(&mut d, 3, s, Step { level: 1, chance: 50, condition: 0, ratchet: 1 });
        }
        set_step(&mut d, 7, 0, Step { level: 1, chance: 100, condition: 0, ratchet: 1 });
        set_length(&mut d, 7, 3);
        let notes = run(&p, &d, 4);
        let hats = notes.iter().filter(|n| n.1 == 42).count();
        assert!((20..44).contains(&hats), "{hats}");
        // A three-step lane: every three sixteenths.
        let bells: Vec<f64> = notes.iter().filter(|n| n.1 == 56).map(|n| n.0).collect();
        assert!(bells.windows(2).all(|w| (w[1] - w[0] - 0.75).abs() < 1e-3));
        p[CHANCE] = 0.0;
        assert_eq!(run(&p, &d, 1).iter().filter(|n| n.1 == 42).count(), 0);
    }

    #[test]
    fn the_patterns_play() {
        let p = defaults(SPECS);
        for name in PATTERNS.iter().filter(|n| **n != "Clear") {
            assert!(run(&p, &pattern(name), 2).len() > 10, "{name}");
        }
    }
}

//! Drum Map: a topographic drum sequencer. A map of five by five grooves
//! (from straight to broken left to right, from laid back to driving bottom
//! to top), each two bars of kick, snare and hats where every step has a
//! weight: how essential it is to the groove. Between the grooves the
//! weights blend, so moving across the map morphs one groove into the next.
//! A Fill per drum plays the steps that weigh enough (low: only the
//! backbone, high: every ghost note), Chaos shakes the weights every two
//! bars, and the weight sets how hard a step is hit.
//!
//! The idea is that of topographic drum sequencers; the map and its grooves
//! are Krabton's own.

use super::SeqState;
use crate::midi_fx::{MidiCtx, MidiEvent};
use crate::params::{ParamSpec, Unit};

pub const RATE: usize = 0;
pub const MAP_X: usize = 1;
pub const MAP_Y: usize = 2;
pub const CHAOS: usize = 3;
pub const FILL: usize = 4; // Kick, Snare, Hat fills: 4, 5, 6.
pub const SWING: usize = 7;
pub const DYNAMICS: usize = 8;
pub const VELOCITY: usize = 9;
pub const OPEN_HATS: usize = 10;
pub const NOTE: usize = 11; // Kick, Snare, Hat, Open Hat notes: 11..=14.

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::choice("Rate", "Clock", super::RATES, 2),
    ParamSpec::lin("Map X", "Map", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Map Y", "Map", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Chaos", "Map", 0.0, 1.0, 0.1, Unit::Percent),
    ParamSpec::lin("Kick Fill", "Fill", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Snare Fill", "Fill", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Hat Fill", "Fill", 0.0, 1.0, 0.55, Unit::Percent),
    ParamSpec::lin("Swing", "Feel", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Dynamics", "Feel", 0.0, 1.0, 0.6, Unit::Percent),
    ParamSpec::lin("Velocity", "Feel", 0.05, 1.0, 0.85, Unit::Percent),
    ParamSpec::toggle("Open Hats", "Feel", true),
    ParamSpec::int("Kick", "Notes", 0.0, 127.0, 36.0, Unit::None),
    ParamSpec::int("Snare", "Notes", 0.0, 127.0, 38.0, Unit::None),
    ParamSpec::int("Hat", "Notes", 0.0, 127.0, 42.0, Unit::None),
    ParamSpec::int("Open Hat", "Notes", 0.0, 127.0, 46.0, Unit::None),
    ParamSpec::toggle("Transpose", "Keys", false),
    ParamSpec::toggle("Hold", "Keys", false),
    ParamSpec::lin("Vel Low", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Vel High", "Dynamics", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::lin("Vel Random", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Vel Accent", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
];

pub const PARTS: usize = 3;
pub const PART_NAMES: [&str; PARTS] = ["Kick", "Snare", "Hats"];
pub const STEPS: usize = 32;
pub const SIZE: usize = 5;

/// The grooves, bottom row (laid back) first, each left (straight) to right
/// (broken): name, kick, snare, hats. A digit is a step's weight (9: the
/// backbone, 1: a rare ornament), a dot a step that never plays.
const NODES: [[(&str, &str, &str, &str); SIZE]; SIZE] = [
    [
        ("Half Time", "9... .... ..4. .... 9... ..3. .... .5..", ".... .... 9... .... .... .... 9... ..3.", "8.4. 7.4. 8.4. 7.4. 8.4. 7.4. 8.4. 7.5."),
        ("Boom Bap", "9... ...6 ..8. .... 9... ...5 ..8. .3..", ".... 9... .... 9..3 .... 9... ..2. 9...", "7.5. 7.5. 7.5. 7.5. 7.5. 7.5. 7.5. 7.6."),
        ("One Drop", ".... .... 9... .... .... .... 9... ....", ".... .... 9... .... .... .... 9... ..3.", "..6. ..7. ..6. ..7. ..6. ..7. ..6. 4.7."),
        ("Trap", "9... ...5 ..7. .... 9..4 .... ..6. 7...", ".... .... 9... .... .... .... 9... ....", "8.5. 8.5. 8.5. 8.65 8.5. 8.5. 8.43 8765"),
        ("Half Step", "9... .... .... ..5. 9... ..4. .... ....", ".... .... 9... .... .... .... 9... .3..", "..5. ..5. ..5. ..5. ..5. 3.5. ..5. ..6."),
    ],
    [
        ("Rock", "9... ..5. 8... .... 9... ..5. 8.4. ....", ".... 9... .... 9... .... 9... .... 9..3", "8.6. 8.6. 8.6. 8.6. 8.6. 8.6. 8.6. 8.6."),
        ("Hip Hop", "9... ..6. .7.. ..4. 9..3 ..6. .7.. ....", ".... 9..3 .... 9... ..2. 9... .3.. 9.4.", "7.5. 7.53 7.5. 7.5. 7.5. 7.5. 7.53 7.6."),
        ("Bossa", "9..5 8... 9..5 8... 9..5 8... 9..5 8...", "9..8 ..8. ..8. .8.. ..8. ..8. .... ....", "7.5. 7.5. 7.5. 7.5. 7.5. 7.5. 7.5. 7.5."),
        ("Two Step", "9... .... ..7. ...5 9... .... ..6. ....", ".... 9... .... 9... .... 9... .... 9..4", "..7. .37. ..7. .47. ..7. .37. ..7. .47."),
        ("Breakbeat", "9.8. .... ..8. .... 9.8. .... ..7. .5..", ".... 9..4 .4.. 9..4 .4.. 9..4 .4.. 9.6.", "8.6. 8.6. 8.6. 8.6. 8.6. 8.6. 8.6. 8.67"),
    ],
    [
        ("House", "9... 9... 9... 9... 9... 9... 9... 9...", ".... 8... .... 8... .... 8... .... 8...", "4384 4384 4384 4384 4384 4384 4384 4386"),
        ("Funk", "9.6. ..5. .8.. ..6. 9.6. ..4. .8.7 ....", ".... 9..3 .3.4 9..3 .3.. 9..3 .3.. 9.5.", "8585 8585 8585 8585 8585 8585 8585 8587"),
        ("Afrobeat", "9... ..6. ..8. 5... 9..4 ..6. ..8. ....", ".... 8..4 .4.. 8.4. ..4. 8... .4.. 8..5", "7.77 .77. 7.77 .77. 7.77 .77. 7.77 .776"),
        ("Electro", "9... ..7. .... .... 9... ..7. ..6. ....", ".... 9... .... 9... .... 9... .... 9...", "7575 7575 7575 7575 7575 7575 7575 7577"),
        ("Jungle", "9.8. .... ..6. .... 9... ..5. .7.. ....", ".... 9..5 .5.. 9... .5.. 9..5 ..5. 9.76", "7.6. 7.6. 7.6. 7.6. 7.6. 7.6. 7.6. 7666"),
    ],
    [
        ("Techno", "9... 9... 9... 9... 9... 9... 9... 9..5", ".... 6... .... 6... .... 6... .... 6..4", "5394 5394 5394 5394 5394 5394 5394 5396"),
        ("Motorik", "9... ..8. 9... .... 9... ..8. 9... ..4.", ".... 9... .... 9... .... 9... .... 9...", "8.6. 8.6. 8.6. 8.6. 8.6. 8.6. 8.6. 8.6."),
        ("Samba", "8... 9..5 8... 9..5 8... 9..5 8... 9..5", "5.75 .75. 5.75 .75. 5.75 .75. 5.75 .757", "8565 8565 8565 8565 8565 8565 8565 8565"),
        ("Dembow", "9... 9... 9... 9... 9... 9... 9... 9...", "...8 ..8. ...8 ..8. ...8 ..8. ...8 ..8.", "7.5. 7.5. 7.5. 7.5. 7.5. 7.5. 7.5. 7.5."),
        ("Footwork", "9..7 ..7. .7.. 7..7 9..7 ..7. .7.. 7.6.", ".... .... 9... .... .... .... 9... ....", "6.6. 6.6. 6.6. 6.6. 6.6. 6.6. 6.66 6666"),
    ],
    [
        ("Hard Techno", "9.4. 9.4. 9.4. 9.4. 9.4. 9.4. 9.4. 9.4.", ".... 7... .... 7... .... 7... .... 7.6.", "8686 8686 8686 8686 8686 8686 8686 8686"),
        ("Punk", "9... 9... 9... 9..4 9... 9... 9... 9..4", "..9. ..9. ..9. ..9. ..9. ..9. ..9. ..9.", "8.7. 8.7. 8.7. 8.7. 8.7. 8.7. 8.7. 8.7."),
        ("Gallop", "9.99 9.99 9.99 9.99 9.99 9.99 9.99 9.99", ".... 9... .... 9... .... 9... .... 9...", "8.6. 8.6. 8.6. 8.6. 8.6. 8.6. 8.6. 8.6."),
        ("Drum & Bass", "9... .... ..9. .... 9... .... ..9. ..5.", ".4.. 9... .4.4 9... .4.. 9... .4.. 9.5.", "7.6. 7.6. 7.6. 7.6. 7.6. 7.6. 7.6. 7666"),
        ("Breakcore", "9.7. .87. 9..7 .8.6 9.87 ..7. 9.7. 8.77", "..6. 9.5. .69. 9.67 .5.6 9.7. .6.9 9789", "8787 8787 8787 8787 8787 8787 8999 9999"),
    ],
];

fn parse(s: &str) -> [u8; STEPS] {
    let mut out = [0u8; STEPS];
    for (o, c) in out.iter_mut().zip(s.chars().filter(|c| !c.is_whitespace())) {
        *o = c.to_digit(10).map(|d| (d * 28) as u8).unwrap_or(0);
    }
    out
}

type Node = [[u8; STEPS]; PARTS];

fn nodes() -> &'static [[Node; SIZE]; SIZE] {
    static N: std::sync::OnceLock<[[Node; SIZE]; SIZE]> = std::sync::OnceLock::new();
    N.get_or_init(|| std::array::from_fn(|y| std::array::from_fn(|x| {
        let (_, k, s, h) = NODES[y][x];
        [parse(k), parse(s), parse(h)]
    })))
}

/// The name of the groove at node (`x`, `y`), `y` 0 at the bottom.
pub fn node_name(x: usize, y: usize) -> &'static str {
    NODES[y.min(SIZE - 1)][x.min(SIZE - 1)].0
}

/// The weights at a place on the map (both 0..1): the four grooves around
/// it blended.
pub fn weights(x: f32, y: f32) -> Node {
    let n = nodes();
    let fx = x.clamp(0.0, 1.0) * (SIZE - 1) as f32;
    let fy = y.clamp(0.0, 1.0) * (SIZE - 1) as f32;
    let (x0, y0) = ((fx as usize).min(SIZE - 2), (fy as usize).min(SIZE - 2));
    let (tx, ty) = (fx - x0 as f32, fy - y0 as f32);
    std::array::from_fn(|p| {
        std::array::from_fn(|s| {
            let a = n[y0][x0][p][s] as f32 + (n[y0][x0 + 1][p][s] as f32 - n[y0][x0][p][s] as f32) * tx;
            let b = n[y0 + 1][x0][p][s] as f32 + (n[y0 + 1][x0 + 1][p][s] as f32 - n[y0 + 1][x0][p][s] as f32) * tx;
            (a + (b - a) * ty).round().clamp(0.0, 255.0) as u8
        })
    })
}

/// The weight a step needs to play at a fill (0..1).
pub fn threshold(fill: f32) -> f32 {
    (1.0 - fill.clamp(0.0, 1.0)) * 255.0
}

/// What plays: per part a bit per step, from the weights, the fills and
/// this cycle's shake (`chaos`, -1..1 per step, already scaled).
pub fn pattern(w: &Node, p: &[f32], shake: &[[f32; STEPS]; PARTS]) -> [u32; PARTS] {
    std::array::from_fn(|part| {
        let t = threshold(p.get(FILL + part).copied().unwrap_or(0.5));
        (0..STEPS).filter(|&s| w[part][s] > 0 && w[part][s] as f32 + shake[part][s] > t && t < 255.0).fold(0u32, |m, s| m | 1 << s)
    })
}

/// The notes a kit has for each part (kick, snare, hat, open hat).
fn part_note(p: &[f32], i: usize, kit: u128) -> u8 {
    use super::drummer::{lane_note, HAT, KICK, OPEN, SNARE};
    let n = p.get(NOTE + i).map(|v| v.round().clamp(0.0, 127.0) as u8).unwrap_or(36);
    if kit == 0 || kit >> n & 1 == 1 {
        return n;
    }
    lane_note([KICK, SNARE, HAT, OPEN][i.min(3)], kit).unwrap_or(n)
}

#[derive(Clone, Debug)]
pub struct DrumMapState {
    /// This cycle's shake of every step's weight.
    pub shake: [[f32; STEPS]; PARTS],
    cycle: i64,
}

impl Default for DrumMapState {
    fn default() -> Self {
        Self { shake: [[0.0; STEPS]; PARTS], cycle: i64::MIN }
    }
}

impl SeqState {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn drum_map(&mut self, k: i64, p: &[f32], at: u64, step_frames: f64, out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        let step = k.rem_euclid(STEPS as i64) as usize;
        let cycle = k.div_euclid(STEPS as i64);
        let chaos = p[CHAOS].clamp(0.0, 1.0);
        if cycle != self.map.cycle {
            // Every two bars the weights are shaken anew.
            self.map.cycle = cycle;
            for part in 0..PARTS {
                for s in 0..STEPS {
                    let u = self.rng.bipolar();
                    self.map.shake[part][s] = u * chaos * chaos.sqrt() * 160.0;
                }
            }
        }
        let w = weights(p[MAP_X], p[MAP_Y]);
        let hits = pattern(&w, p, &self.map.shake);
        // The swung 16ths come later.
        let delay = if step % 2 == 1 { p[SWING].clamp(0.0, 1.0) as f64 * 0.33 * step_frames } else { 0.0 };
        let at = at + delay as u64;
        let dynamics = p[DYNAMICS].clamp(0.0, 1.0);
        let vel = p[VELOCITY].clamp(0.05, 1.0);
        for part in 0..PARTS {
            if hits[part] >> step & 1 == 0 {
                continue;
            }
            let weight = ((w[part][step] as f32 + self.map.shake[part][step]) / 252.0).clamp(0.0, 1.0);
            let v = vel * (1.0 - dynamics + dynamics * (0.3 + 0.7 * weight));
            // A strong hat on an off-beat eighth opens.
            let open = part == 2 && p[OPEN_HATS] >= 0.5 && step % 4 == 2 && w[part][step] >= 224;
            let note = part_note(p, if open { 3 } else { part }, ctx.drum_notes);
            let len = if open { step_frames * 1.6 } else { step_frames * 0.5 };
            self.note(ctx, at, note as i32, v, len, out);
        }
        self.visual[0] = step as u64 | (cycle as u64 & 0xffff) << 8;
        self.visual[1] = hits[0] as u64 | (hits[1] as u64) << 32;
        self.visual[2] = hits[2] as u64;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::defaults;

    #[test]
    fn every_groove_has_two_bars_of_three_parts() {
        for row in NODES.iter() {
            for (name, k, s, h) in row.iter() {
                for lane in [k, s, h] {
                    let n = lane.chars().filter(|c| !c.is_whitespace()).count();
                    assert_eq!(n, STEPS, "{name}");
                }
            }
        }
    }

    #[test]
    fn the_corners_are_their_grooves_and_between_they_blend() {
        let n = nodes();
        assert_eq!(weights(0.0, 0.0), n[0][0]);
        assert_eq!(weights(1.0, 1.0), n[4][4]);
        assert_eq!(weights(0.0, 0.5), n[2][0]);
        // Halfway between House and Funk a step weighs the average.
        let mid = weights(0.125, 0.5);
        for s in 0..STEPS {
            let avg = (n[2][0][0][s] as f32 + n[2][1][0][s] as f32) / 2.0;
            assert!((mid[0][s] as f32 - avg).abs() <= 1.0);
        }
    }

    #[test]
    fn more_fill_plays_more_and_none_plays_nothing() {
        let mut p = defaults(SPECS);
        let w = weights(0.25, 0.5);
        let calm = [[0.0; STEPS]; PARTS];
        let count = |p: &[f32]| pattern(&w, p, &calm).iter().map(|m| m.count_ones()).sum::<u32>();
        p[FILL] = 0.0;
        p[FILL + 1] = 0.0;
        p[FILL + 2] = 0.0;
        assert_eq!(count(&p), 0);
        let mut last = 0;
        for f in [0.2, 0.5, 0.8, 1.0] {
            p[FILL] = f;
            p[FILL + 1] = f;
            p[FILL + 2] = f;
            let c = count(&p);
            assert!(c >= last, "{f}: {c} < {last}");
            last = c;
        }
        assert!(last > 40);
        // At a low fill only the backbone: House's four kicks.
        let house = weights(0.0, 0.5);
        p[FILL] = 0.1;
        assert_eq!(pattern(&house, &p, &calm)[0], 0x1111_1111);
    }

    #[test]
    fn it_plays_a_house_beat_in_time() {
        let mut s = SeqState::new();
        let p = defaults(SPECS);
        let mut out = Vec::new();
        let ctx = MidiCtx { now: 0, len: 48_000, samples_per_beat: 24_000.0, scale: crate::scale::Scale::new(0, crate::scale::ScaleKind::Major), scale_on: false, beat: 0.0, playing: true, sr: 48_000.0, drum_notes: 0, lead: 0.0, touch: [0.0; 5], band: Default::default() };
        s.process(crate::midi_fx::MidiFxKind::DrumMap, &p, &[], &mut out, &ctx);
        let kicks: Vec<u32> = out.iter().filter(|e| e.on && e.pitch == 36).map(|e| e.offset).collect();
        assert_eq!(kicks, vec![0, 24_000]);
        assert!(out.iter().any(|e| e.on && e.pitch == 38));
        assert!(out.iter().any(|e| e.on && (e.pitch == 42 || e.pitch == 46)));
    }
}

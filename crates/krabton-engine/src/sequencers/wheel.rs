//! Wheel: a big wheel with a note in each car, and a rider going round it who
//! plays every car it passes. Gravity has the rider: from the top it falls and
//! goes faster and faster to the bottom, then slows climbing back up, so the
//! notes come in rushes and hesitations, as on James Holden's records rather
//! than on a grid. A turn still takes exactly its bars, so it stays with the
//! song. *Swing* has it rock to and fro about the bottom instead, playing the
//! cars it reaches both ways. Its height over a turn is the sine of the unit
//! circle, bent by gravity: the editor draws both.

use super::minimal::DYNAMICS;
use super::nature::frame_of;
use super::SeqState;
use crate::midi_fx::{MidiCtx, MidiEvent};
use crate::params::{ParamSpec, Unit};
use std::f64::consts::{PI, TAU};

pub const TURN: usize = 0;
pub const MOTION: usize = 1;
pub const GRAVITY: usize = 2;
pub const REACH: usize = 3;
pub const CARS: usize = 4;
pub const BASE: usize = 5;
pub const GATE: usize = 6;
pub const VELOCITY: usize = 7;
pub const RUSH: usize = 8;
pub const FIRST_CAR: usize = 9;
pub const MAX_CARS: usize = 16;

pub const TURNS: &[&str] = &["1 Bar", "2 Bars", "4 Bars", "8 Bars", "16 Bars"];
const TURN_BEATS: [f64; 5] = [4.0, 8.0, 16.0, 32.0, 64.0];
pub const MOTIONS: &[&str] = &["Wheel", "Swing"];

const fn car(name: &'static str, note: f32) -> ParamSpec {
    ParamSpec::int(name, "Cars", 0.0, 15.0, note, Unit::None)
}

const KEYS: [ParamSpec; 2] = [ParamSpec::toggle("Transpose", "Keys", false), ParamSpec::toggle("Hold", "Keys", false)];

pub static WHEEL: &[ParamSpec] = &[
    ParamSpec::choice("Turn", "Wheel", TURNS, 1),
    ParamSpec::choice("Motion", "Wheel", MOTIONS, 0),
    ParamSpec::lin("Gravity", "Wheel", 0.0, 1.0, 0.6, Unit::Percent),
    ParamSpec::lin("Reach", "Wheel", 0.1, 1.0, 0.6, Unit::Percent),
    ParamSpec::int("Cars", "Wheel", 3.0, 16.0, 8.0, Unit::None),
    ParamSpec::int("Base Note", "Notes", 24.0, 96.0, 60.0, Unit::None),
    ParamSpec::lin("Gate", "Notes", 0.05, 2.0, 0.3, Unit::None),
    ParamSpec::lin("Velocity", "Notes", 0.05, 1.0, 0.8, Unit::Percent),
    ParamSpec::lin("Rush", "Notes", 0.0, 1.0, 0.4, Unit::Percent),
    car("Car 1", 1.0),
    car("Car 2", 3.0),
    car("Car 3", 5.0),
    car("Car 4", 0.0),
    car("Car 5", 8.0),
    car("Car 6", 6.0),
    car("Car 7", 0.0),
    car("Car 8", 4.0),
    car("Car 9", 2.0),
    car("Car 10", 5.0),
    car("Car 11", 0.0),
    car("Car 12", 7.0),
    car("Car 13", 9.0),
    car("Car 14", 0.0),
    car("Car 15", 3.0),
    car("Car 16", 6.0),
    KEYS[0],
    KEYS[1],
    DYNAMICS[0],
    DYNAMICS[1],
    DYNAMICS[2],
    DYNAMICS[3],
    ParamSpec::toggle("Band", "Band", false),
    ParamSpec::choice("Follow", "Band", crate::band::FOLLOWS, 0),
];

/// How many cars it has.
pub fn cars(p: &[f32]) -> usize {
    (p[CARS].round().max(1.0) as usize).clamp(3, MAX_CARS)
}

/// Where car `k` hangs: radians clockwise from the top.
pub fn car_angle(p: &[f32], k: usize) -> f64 {
    TAU * k as f64 / cars(p) as f64
}

/// How long a turn (or a swing there and back) takes, in beats.
pub fn turn_beats(p: &[f32]) -> f64 {
    TURN_BEATS[(p[TURN].round().max(0.0) as usize).min(TURN_BEATS.len() - 1)]
}

fn swings(p: &[f32]) -> bool {
    p[MOTION] >= 0.5
}

fn gravity(p: &[f32]) -> f64 {
    p[GRAVITY].clamp(0.0, 1.0) as f64
}

/// How fast the rider goes at an angle (1 at the top): faster as it has fallen.
fn speed(g: f64, angle: f64) -> f64 {
    (1.0 + 24.0 * g * (1.0 - angle.cos()) * 0.5).sqrt()
}

const STEPS: usize = 256;

/// How far through a turn (0..1) the rider is at each of `STEPS` angles.
fn table(g: f64) -> [f64; STEPS + 1] {
    let mut t = [0.0; STEPS + 1];
    let d = TAU / STEPS as f64;
    for i in 0..STEPS {
        t[i + 1] = t[i] + d / speed(g, (i as f64 + 0.5) * d);
    }
    let total = t[STEPS];
    for v in t.iter_mut() {
        *v /= total;
    }
    t
}

/// The rider's angle a share of the way through a turn (0..1): radians
/// clockwise from the top, going on past a turn for the wheel; about the bottom
/// (π) for a swing.
pub fn angle_at(p: &[f32], frac: f64) -> f64 {
    let g = gravity(p);
    if swings(p) {
        let reach = p[REACH].clamp(0.05, 1.0) as f64 * PI;
        let f = frac.rem_euclid(1.0);
        // From an even to and fro (no gravity) to a pendulum's.
        let tri = if f < 0.25 {
            4.0 * f
        } else if f < 0.75 {
            2.0 - 4.0 * f
        } else {
            4.0 * f - 4.0
        };
        return PI + reach * ((1.0 - g) * tri + g * (TAU * f).sin());
    }
    let t = table(g);
    let turns = frac.floor();
    let f = frac - turns;
    // The step of the table it is in.
    let i = t.partition_point(|v| *v <= f).clamp(1, STEPS) - 1;
    let within = (f - t[i]) / (t[i + 1] - t[i]).max(1e-12);
    turns * TAU + (i as f64 + within) * TAU / STEPS as f64
}

/// The rider's angle at a beat of the song.
pub fn angle_at_beat(p: &[f32], beat: f64) -> f64 {
    angle_at(p, beat / turn_beats(p))
}

/// How fast the rider goes at an angle, from 0 (stopped) to 1 (its fastest).
pub fn rush_at(p: &[f32], angle: f64) -> f64 {
    if swings(p) {
        let reach = p[REACH].clamp(0.05, 1.0) as f64 * PI;
        return (1.0 - ((angle - PI).abs() / reach)).clamp(0.0, 1.0);
    }
    let g = gravity(p);
    speed(g, angle) / speed(g, PI)
}

/// The note in car `k` (steps of the key above the base note), or none.
pub fn car_note(p: &[f32], k: usize) -> Option<i32> {
    let v = p.get(FIRST_CAR + k).copied().unwrap_or(0.0).round() as i32;
    (v > 0).then_some(v - 1)
}

/// The cars the rider passes between two beats, in order: (beat, car).
pub fn passes(p: &[f32], b0: f64, b1: f64, out: &mut impl FnMut(f64, usize)) {
    let (a0, a1) = (angle_at_beat(p, b0), angle_at_beat(p, b1));
    if (a1 - a0).abs() < 1e-12 {
        return;
    }
    let (lo, hi) = (a0.min(a1), a0.max(a1));
    let mut hits: [(f64, usize); 2 * MAX_CARS] = [(0.0, 0); 2 * MAX_CARS];
    let mut n = 0;
    for k in 0..cars(p) {
        let a = car_angle(p, k);
        // Every turn of this car's angle between the two.
        let mut m = ((lo - a) / TAU).floor();
        while a + m * TAU <= hi {
            let x = a + m * TAU;
            // Passing it: just past it, not sitting on it from before.
            let crossed = if a1 > a0 { x > a0 && x <= a1 } else { x < a0 && x >= a1 };
            if crossed && n < hits.len() {
                let f = (x - a0) / (a1 - a0);
                hits[n] = (b0 + f * (b1 - b0), k);
                n += 1;
            }
            m += 1.0;
        }
    }
    let hits = &mut hits[..n];
    hits.sort_by(|a, b| a.0.total_cmp(&b.0));
    for &(beat, k) in hits.iter() {
        out(beat, k);
    }
}

impl SeqState {
    pub(crate) fn wheel(&mut self, p: &[f32], out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        let (b0, b1) = (ctx.beat, ctx.beat + ctx.len as f64 / ctx.samples_per_beat);
        let mut due: [(f64, usize); 2 * MAX_CARS] = [(0.0, 0); 2 * MAX_CARS];
        let mut n = 0;
        passes(p, b0, b1, &mut |beat, k| {
            if n < due.len() {
                due[n] = (beat, k);
                n += 1;
            }
        });
        let (home, _) = ctx.scale.locate(p[BASE].round() as i32);
        let rush = p[RUSH].clamp(0.0, 1.0);
        for &(beat, k) in &due[..n] {
            // A pass on a block's edge is seen by both blocks: it plays once.
            if beat <= self.wheel_last[k] + 1e-6 {
                continue;
            }
            self.wheel_last[k] = beat;
            let Some(step) = car_note(p, k) else { continue };
            let fast = rush_at(p, angle_at_beat(p, beat)) as f32;
            let vel = p[VELOCITY] * (1.0 - rush * (1.0 - fast));
            let pitch = ctx.scale.pitch_at(home + step);
            self.play(ctx, frame_of(ctx, beat), pitch, vel, p[GATE] as f64 * ctx.samples_per_beat, out);
        }
        // For the editor: where the rider is.
        let a = angle_at_beat(p, b1).rem_euclid(TAU);
        self.visual[0] = (a / TAU * 65535.0) as u64;
        self.visual[1] = ((b1 / turn_beats(p)).rem_euclid(1.0) * 65535.0) as u64;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::midi_fx::MidiFxKind;
    use crate::params::defaults;
    use crate::scale::{Scale, ScaleKind};
    use crate::sequencers::tests::run;

    fn wheel() -> Vec<f32> {
        defaults(MidiFxKind::Wheel.specs())
    }

    fn all_cars(mut p: Vec<f32>) -> Vec<f32> {
        for k in 0..MAX_CARS {
            p[FIRST_CAR + k] = 1.0;
        }
        p
    }

    /// The passes over so many beats, a little at a time as the engine sees them.
    fn hits(p: &[f32], beats: f64) -> Vec<(f64, usize)> {
        let mut t = Vec::new();
        // Counted, not added up: a sum of hundredths falls short of the end.
        let chunks = (beats * 100.0).round() as usize;
        for i in 0..chunks {
            passes(p, i as f64 / 100.0, (i + 1) as f64 / 100.0, &mut |at, k| t.push((at, k)));
        }
        t
    }

    fn times(p: &[f32], beats: f64) -> Vec<f64> {
        hits(p, beats).into_iter().map(|h| h.0).collect()
    }

    #[test]
    fn without_gravity_the_cars_come_evenly() {
        let mut p = wheel();
        p[GRAVITY] = 0.0;
        p[TURN] = 0.0;
        p[CARS] = 8.0;
        let t = times(&p, 4.0);
        assert_eq!(t.len(), 8, "{t:?}");
        for (k, b) in t.iter().enumerate() {
            assert!((b - (k + 1) as f64 * 0.5).abs() < 1e-6, "{k}: {b}");
        }
    }

    #[test]
    fn with_gravity_it_hangs_at_the_top_and_rushes_through_the_bottom() {
        let mut p = wheel();
        p[GRAVITY] = 1.0;
        p[TURN] = 0.0;
        p[CARS] = 8.0;
        let t = times(&p, 4.0);
        assert_eq!(t.len(), 8);
        // From the top to the next car takes longest, through the bottom least.
        let gaps: Vec<f64> = std::iter::once(t[0]).chain(t.windows(2).map(|w| w[1] - w[0])).collect();
        let (first, bottom) = (gaps[0], gaps[4]);
        assert!(first > bottom * 2.5, "{gaps:?}");
        // Still a turn in its bars.
        assert!((t[7] - 4.0).abs() < 1e-6, "{}", t[7]);
    }

    #[test]
    fn it_goes_round_and_round() {
        let p = wheel();
        let one = times(&p, turn_beats(&p));
        let two = times(&p, 2.0 * turn_beats(&p));
        assert_eq!(two.len(), 2 * one.len());
        for (a, b) in one.iter().zip(&two[one.len()..]) {
            assert!((b - a - turn_beats(&p)).abs() < 1e-6);
        }
    }

    #[test]
    fn swinging_it_plays_only_the_cars_it_reaches_both_ways() {
        let mut p = all_cars(wheel());
        p[MOTION] = 1.0;
        p[REACH] = 0.45;
        p[CARS] = 8.0;
        let mut cars_hit: Vec<usize> = hits(&p, turn_beats(&p)).into_iter().map(|h| h.1).collect();
        // About the bottom (car 4), up to 81 degrees either way: cars 3, 4 and 5.
        cars_hit.sort();
        assert_eq!(cars_hit, [3, 3, 4, 4, 5, 5], "both ways");
        assert!(rush_at(&p, PI) > 0.99 && rush_at(&p, PI + 0.45 * PI) < 0.01);
    }

    #[test]
    fn its_notes_are_in_the_key_and_faster_is_louder() {
        let ons = run(MidiFxKind::Wheel, &wheel(), &MidiFxKind::Wheel.default_data(), 16.0);
        assert!(ons.len() >= 8, "{}", ons.len());
        let c = Scale::new(0, ScaleKind::Major);
        assert!(ons.iter().all(|o| c.contains(o.1)), "{ons:?}");
        let p = wheel();
        assert!(rush_at(&p, PI) > rush_at(&p, 0.3) * 1.5);
    }

    #[test]
    fn an_empty_car_rests() {
        let mut p = wheel();
        for k in 0..MAX_CARS {
            p[FIRST_CAR + k] = 0.0;
        }
        assert!(run(MidiFxKind::Wheel, &p, &MidiFxKind::Wheel.default_data(), 8.0).is_empty());
        assert_eq!(car_note(&p, 0), None);
        p[FIRST_CAR] = 3.0;
        assert_eq!(car_note(&p, 0), Some(2));
    }

    #[test]
    fn the_angle_runs_on_smoothly_and_its_table_inverts() {
        let mut p = wheel();
        p[GRAVITY] = 0.8;
        let mut last = angle_at(&p, 0.0);
        for k in 1..=400 {
            let a = angle_at(&p, k as f64 / 200.0);
            assert!(a > last, "{k}");
            last = a;
        }
        assert!((angle_at(&p, 1.0) - TAU).abs() < 1e-9);
        assert!((angle_at(&p, 0.5) - PI).abs() < 1e-9, "half the time to the bottom: it is symmetric");
    }
}

//! Bits: rhythms from bytebeat formulas. A counter goes up one every step;
//! a formula of it (t * (t >> 5 | t >> 8) and friends) gives a byte, and
//! each of four drums listens to one of its eight bits: it plays when its
//! bit is on (Level) or when it turns on (Edge). The low bits of a counter
//! are the even divisions of the bar, and the formulas fold and multiply
//! them into rhythms that repeat in powers of two, with breaks and turns of
//! their own. A loop length keeps them to a few bars; Seed starts the
//! counter elsewhere for another rhythm from the same formula.

use super::SeqState;
use crate::midi_fx::{MidiCtx, MidiEvent};
use crate::params::{ParamSpec, Unit};

pub const RATE: usize = 0;
pub const FORMULA: usize = 1;
pub const A: usize = 2;
pub const B: usize = 3;
pub const SEED: usize = 4;
pub const LOOP: usize = 5;
pub const MODE: usize = 6;
pub const DYNAMICS: usize = 7;
pub const VELOCITY: usize = 8;
pub const BIT: usize = 9; // Four lanes' bits: 9..=12.
pub const NOTE: usize = 13; // And their drums: 13..=16.

pub const LANES: usize = 4;
pub const FORMULAS: &[&str] = &["t", "t*a", "t*(t>>a)", "t^(t>>a)", "t*(t>>a|t>>b)", "t&t>>a", "t|t>>a", "(t*a)&(t>>b)", "(t*a)^(t>>b)", "t*a/(b+1)", "t%(a+b)", "(t>>a)*(t>>b)"];
pub const LOOPS: &[&str] = &["Off", "1 Bar", "2 Bars", "4 Bars", "8 Bars"];
pub const MODES: &[&str] = &["Level", "Edge"];

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::choice("Rate", "Clock", super::RATES, 2),
    ParamSpec::choice("Formula", "Formula", FORMULAS, 4),
    ParamSpec::int("A", "Formula", 1.0, 16.0, 5.0, Unit::None),
    ParamSpec::int("B", "Formula", 1.0, 16.0, 8.0, Unit::None),
    ParamSpec::int("Seed", "Formula", 0.0, 255.0, 0.0, Unit::None),
    ParamSpec::choice("Loop", "Formula", LOOPS, 3),
    ParamSpec::choice("Mode", "Play", MODES, 1),
    ParamSpec::lin("Dynamics", "Play", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Velocity", "Play", 0.05, 1.0, 0.8, Unit::Percent),
    ParamSpec::int("Bit 1", "Lanes", 0.0, 7.0, 4.0, Unit::None),
    ParamSpec::int("Bit 2", "Lanes", 0.0, 7.0, 6.0, Unit::None),
    ParamSpec::int("Bit 3", "Lanes", 0.0, 7.0, 1.0, Unit::None),
    ParamSpec::int("Bit 4", "Lanes", 0.0, 7.0, 3.0, Unit::None),
    ParamSpec::int("Drum 1", "Lanes", 0.0, 127.0, 36.0, Unit::None),
    ParamSpec::int("Drum 2", "Lanes", 0.0, 127.0, 38.0, Unit::None),
    ParamSpec::int("Drum 3", "Lanes", 0.0, 127.0, 42.0, Unit::None),
    ParamSpec::int("Drum 4", "Lanes", 0.0, 127.0, 39.0, Unit::None),
    ParamSpec::toggle("Transpose", "Keys", false),
    ParamSpec::toggle("Hold", "Keys", false),
];

/// The formula's byte at step `t`.
pub fn byte(formula: usize, t: u32, a: u32, b: u32) -> u8 {
    let (a, b) = (a.clamp(1, 16), b.clamp(1, 16));
    let v = match formula {
        1 => t.wrapping_mul(a),
        2 => t.wrapping_mul(t >> a),
        3 => t ^ (t >> a),
        4 => t.wrapping_mul((t >> a) | (t >> b)),
        5 => t & (t >> a),
        6 => t | (t >> a),
        7 => t.wrapping_mul(a) & (t >> b),
        8 => t.wrapping_mul(a) ^ (t >> b),
        9 => t.wrapping_mul(a) / (b + 1),
        10 => t % (a + b),
        11 => (t >> a).wrapping_mul(t >> b),
        _ => t,
    };
    v as u8
}

/// The counter at song step `k`: looped to a few bars (of `steps_per_bar`)
/// and moved by the seed.
pub fn counter(k: i64, p: &[f32], steps_per_bar: i64) -> u32 {
    let bars = [0i64, 1, 2, 4, 8][(p[LOOP].round().max(0.0) as usize).min(4)];
    let k = if bars > 0 { k.rem_euclid(bars * steps_per_bar.max(1)) } else { k.max(0) };
    (k as u32).wrapping_add(p[SEED].round().clamp(0.0, 255.0) as u32)
}

/// Whether lane `lane` plays at counter `t` (and how hard: the bits above
/// its own make the dynamics).
pub fn hit(p: &[f32], t: u32, lane: usize) -> Option<f32> {
    let f = (p[FORMULA].round().max(0.0) as usize).min(FORMULAS.len() - 1);
    let (a, b) = (p[A].round() as u32, p[B].round() as u32);
    let bit = p[BIT + lane].round().clamp(0.0, 7.0) as u32;
    let now = byte(f, t, a, b);
    let on = now >> bit & 1 == 1;
    let plays = if p[MODE] >= 0.5 {
        let before = byte(f, t.wrapping_sub(1), a, b);
        on && (before >> bit & 1 == 0 || t == 0)
    } else {
        on
    };
    plays.then(|| {
        let upper = (now as u32 >> (bit + 1)) as f32 / (255u32 >> (bit + 1)).max(1) as f32;
        upper.clamp(0.0, 1.0)
    })
}

impl SeqState {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn bits(&mut self, k: i64, p: &[f32], at: u64, step_frames: f64, out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        let rate = super::rate_beats(p[RATE]);
        let per_bar = (4.0 / rate).round() as i64;
        let t = counter(k, p, per_bar);
        let dynamics = p[DYNAMICS].clamp(0.0, 1.0);
        let vel = p[VELOCITY].clamp(0.05, 1.0);
        let mut fired = 0u64;
        for lane in 0..LANES {
            if let Some(upper) = hit(p, t, lane) {
                fired |= 1 << lane;
                let v = vel * (1.0 - dynamics + dynamics * (0.4 + 0.6 * upper));
                let note = p[NOTE + lane].round().clamp(0.0, 127.0) as i32;
                self.note(ctx, at, note, v, step_frames * 0.5, out);
            }
        }
        self.visual[0] = t as u64 | fired << 32;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::defaults;

    #[test]
    fn the_counter_is_the_divisions_of_the_bar() {
        let mut p = defaults(SPECS);
        p[FORMULA] = 0.0;
        p[MODE] = 1.0;
        p[BIT] = 2.0;
        // Bit 2 of the counter turns on every fourth step, from the third.
        let hits: Vec<u32> = (0..16).filter(|&t| hit(&p, t, 0).is_some()).collect();
        assert_eq!(hits, vec![4, 12]);
        p[MODE] = 0.0;
        let level: Vec<u32> = (0..8).filter(|&t| hit(&p, t, 0).is_some()).collect();
        assert_eq!(level, vec![4, 5, 6, 7]);
    }

    #[test]
    fn a_loop_repeats_and_every_formula_plays() {
        let p = defaults(SPECS);
        // Four bars of sixteenths.
        assert_eq!(counter(3, &p, 16), counter(67, &p, 16));
        for f in 0..FORMULAS.len() {
            let mut p = defaults(SPECS);
            p[FORMULA] = f as f32;
            p[A] = 2.0;
            p[B] = 4.0;
            let n: usize = (0..64u32).map(|t| (0..LANES).filter(|&l| hit(&p, t, l).is_some()).count()).sum();
            assert!(n > 4, "{}: {n}", FORMULAS[f]);
        }
    }
}

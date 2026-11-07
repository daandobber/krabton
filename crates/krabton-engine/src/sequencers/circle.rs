//! Unit Circle: a point P going round the unit circle at angle θ (from the
//! positive x-axis, anticlockwise, as in a maths book). On the circle lie N
//! points at θₖ = 2πk/N, each with a note or a rest; P plays every one it
//! passes. Its angular speed ω can vary with θ (*Warp*: slow at the top, fast
//! at the bottom, as a pendulum's), or it can *Oscillate* about the bottom,
//! θ = θ꜀ + A·sin(2πt). The pitch can come from the points, or from sin θ,
//! cos θ or tan θ of P (of mθ, with *Harmonic* m), so the melody is the curve:
//! a sine wave, or tan's leaps at its asymptotes; sec θ, csc θ and cot θ too.
//! The functions can also set how long the notes are, which octave, and turn
//! the track's mod wheel (CC 1) as P goes round, so cos θ can sweep a filter
//! while sin θ plays the tune. More phases put more points P on the circle,
//! 2π/R apart. A revolution takes exactly its *Period*.

use super::minimal::{hash, DYNAMICS};
use super::nature::frame_of;
use super::SeqState;
use crate::midi_fx::{MidiCtx, MidiEvent};
use crate::params::{ParamSpec, Unit};
use std::f64::consts::{FRAC_PI_2, PI, TAU};

pub const PERIOD: usize = 0;
pub const MOTION: usize = 1;
pub const WARP: usize = 2;
pub const AMPLITUDE: usize = 3;
pub const POINTS: usize = 4;
pub const PHASES: usize = 5;
pub const REVERSE: usize = 6;
pub const OFFSET: usize = 7;
pub const PITCH: usize = 8;
pub const HARMONIC: usize = 9;
pub const RANGE: usize = 10;
pub const VEL_FROM: usize = 11;
pub const BASE: usize = 12;
pub const GATE: usize = 13;
pub const VELOCITY: usize = 14;
pub const CHANCE: usize = 15;
pub const CLIMB: usize = 16;
pub const CLIMB_TURNS: usize = 17;
pub const FIRST_POINT: usize = 18;
pub const MAX_POINTS: usize = 16;
pub const MAX_PHASES: usize = 4;
pub const GATE_FROM: usize = 42;
pub const OCTAVE_FROM: usize = 43;
pub const MOD_FROM: usize = 44;
/// What plays, each its own voice: the points' notes, then sin θ to cot θ.
pub const PLAY_POINTS: usize = 45;
/// Each voice's own transpose (steps of the key), in the same order.
pub const SHIFT_FIRST: usize = 52;
pub const VOICES: usize = 7;

pub const PERIODS: &[&str] = &["1/4 Beat", "1/2 Beat", "1 Beat", "2 Beats", "1 Bar", "2 Bars", "4 Bars", "8 Bars", "16 Bars"];
const PERIOD_BEATS: [f64; 9] = [0.25, 0.5, 1.0, 2.0, 4.0, 8.0, 16.0, 32.0, 64.0];
pub const MOTIONS: &[&str] = &["Rotate", "Oscillate"];
/// What sets a note's pitch: the point's note, or a function of θ.
pub const PITCHES: &[&str] = &["Points", "sin θ", "cos θ", "tan θ", "sec θ", "csc θ", "cot θ"];
pub const VELOCITIES: &[&str] = &["Fixed", "Speed ω", "|sin θ|", "|cos θ|", "|tan θ|", "|sec θ|", "|csc θ|", "|cot θ|"];
/// What sets a note's length, its octave, the mod wheel.
pub const GATES: &[&str] = &["Fixed", "Speed ω", "|sin θ|", "|cos θ|", "|tan θ|", "|sec θ|", "|csc θ|", "|cot θ|"];
pub const OCTAVES: &[&str] = &["Off", "sin θ", "cos θ", "tan θ", "sec θ", "csc θ", "cot θ"];
pub const MODS: &[&str] = &["Off", "sin θ", "cos θ", "tan θ", "sec θ", "csc θ", "cot θ", "Speed ω"];
/// The functions, numbered from 1 as in `value`.
pub const FUNCTIONS: [&str; 6] = ["sin", "cos", "tan", "sec", "csc", "cot"];

const fn point(name: &'static str, note: f32) -> ParamSpec {
    ParamSpec::int(name, "Points", 0.0, 15.0, note, Unit::None)
}

const KEYS: [ParamSpec; 2] = [ParamSpec::toggle("Transpose", "Keys", false), ParamSpec::toggle("Hold", "Keys", false)];

pub static CIRCLE: &[ParamSpec] = &[
    ParamSpec::choice("Period", "Circle", PERIODS, 5),
    ParamSpec::choice("Motion", "Circle", MOTIONS, 0),
    ParamSpec::lin("Warp", "Circle", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Amplitude", "Circle", 0.1, 1.0, 0.6, Unit::Percent),
    ParamSpec::int("Points", "Circle", 3.0, 16.0, 8.0, Unit::None),
    ParamSpec::int("Phases", "Circle", 1.0, 4.0, 1.0, Unit::None),
    ParamSpec::toggle("Reverse", "Circle", false),
    ParamSpec::lin("Offset", "Circle", 0.0, 1.0, 0.0, Unit::Percent),
    // Was the one thing that played; now the Play switches say (old sets keep it).
    ParamSpec::choice("Pitch", crate::params::SCREEN_ONLY, PITCHES, 0),
    ParamSpec::int("Harmonic", "Function", 1.0, 8.0, 1.0, Unit::None),
    ParamSpec::int("Range", "Function", 2.0, 24.0, 8.0, Unit::None),
    ParamSpec::choice("Vel From", "Function", VELOCITIES, 1),
    ParamSpec::int("Base Note", "Notes", 24.0, 96.0, 60.0, Unit::None),
    ParamSpec::lin("Gate", "Notes", 0.02, 2.0, 0.25, Unit::None),
    ParamSpec::lin("Velocity", "Notes", 0.05, 1.0, 0.8, Unit::Percent),
    ParamSpec::lin("Chance", "Notes", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::int("Climb", "Notes", -7.0, 7.0, 0.0, Unit::None),
    ParamSpec::int("Climb Turns", "Notes", 1.0, 8.0, 4.0, Unit::None),
    point("Point 1", 1.0),
    point("Point 2", 3.0),
    point("Point 3", 5.0),
    point("Point 4", 0.0),
    point("Point 5", 8.0),
    point("Point 6", 6.0),
    point("Point 7", 0.0),
    point("Point 8", 4.0),
    point("Point 9", 2.0),
    point("Point 10", 5.0),
    point("Point 11", 0.0),
    point("Point 12", 7.0),
    point("Point 13", 9.0),
    point("Point 14", 0.0),
    point("Point 15", 3.0),
    point("Point 16", 6.0),
    KEYS[0],
    KEYS[1],
    DYNAMICS[0],
    DYNAMICS[1],
    DYNAMICS[2],
    DYNAMICS[3],
    ParamSpec::toggle("Band", "Band", false),
    ParamSpec::choice("Follow", "Band", crate::band::FOLLOWS, 0),
    ParamSpec::choice("Gate From", "Function", GATES, 0),
    ParamSpec::choice("Octave From", "Function", OCTAVES, 0),
    ParamSpec::choice("Mod From", "Function", MODS, 0),
    ParamSpec::toggle("Play Points", crate::params::SCREEN_ONLY, true),
    ParamSpec::toggle("Play sin θ", crate::params::SCREEN_ONLY, false),
    ParamSpec::toggle("Play cos θ", crate::params::SCREEN_ONLY, false),
    ParamSpec::toggle("Play tan θ", crate::params::SCREEN_ONLY, false),
    ParamSpec::toggle("Play sec θ", crate::params::SCREEN_ONLY, false),
    ParamSpec::toggle("Play csc θ", crate::params::SCREEN_ONLY, false),
    ParamSpec::toggle("Play cot θ", crate::params::SCREEN_ONLY, false),
    ParamSpec::int("Shift Points", crate::params::SCREEN_ONLY, -21.0, 21.0, 0.0, Unit::None),
    ParamSpec::int("Shift sin θ", crate::params::SCREEN_ONLY, -21.0, 21.0, 0.0, Unit::None),
    ParamSpec::int("Shift cos θ", crate::params::SCREEN_ONLY, -21.0, 21.0, 0.0, Unit::None),
    ParamSpec::int("Shift tan θ", crate::params::SCREEN_ONLY, -21.0, 21.0, 0.0, Unit::None),
    ParamSpec::int("Shift sec θ", crate::params::SCREEN_ONLY, -21.0, 21.0, 0.0, Unit::None),
    ParamSpec::int("Shift csc θ", crate::params::SCREEN_ONLY, -21.0, 21.0, 0.0, Unit::None),
    ParamSpec::int("Shift cot θ", crate::params::SCREEN_ONLY, -21.0, 21.0, 0.0, Unit::None),
];

/// N, the points on the circle.
pub fn points(p: &[f32]) -> usize {
    (p[POINTS].round().max(1.0) as usize).clamp(3, MAX_POINTS)
}

/// R, the points P going round, 2π/R apart.
pub fn phases(p: &[f32]) -> usize {
    (p[PHASES].round().max(1.0) as usize).clamp(1, MAX_PHASES)
}

/// θₖ: where point `k` lies.
pub fn point_angle(p: &[f32], k: usize) -> f64 {
    TAU * k as f64 / points(p) as f64
}

/// The beats one revolution takes.
pub fn period_beats(p: &[f32]) -> f64 {
    PERIOD_BEATS[(p[PERIOD].round().max(0.0) as usize).min(PERIOD_BEATS.len() - 1)]
}

fn oscillates(p: &[f32]) -> bool {
    p[MOTION] >= 0.5
}

fn warp(p: &[f32]) -> f64 {
    p[WARP].clamp(0.0, 1.0) as f64
}

/// θ₀, where P starts (radians).
fn offset(p: &[f32]) -> f64 {
    p[OFFSET].clamp(0.0, 1.0) as f64 * TAU
}

/// ω at θ, relative to its slowest (at the top): ω = √(1 + 12k(1 − sin θ)).
fn omega(k: f64, theta: f64) -> f64 {
    (1.0 + 12.0 * k * (1.0 - theta.sin())).sqrt()
}

const STEPS: usize = 256;

/// The share of a revolution P takes to get each of `STEPS` steps round from θ₀.
fn table(k: f64, from: f64, dir: f64) -> [f64; STEPS + 1] {
    let mut t = [0.0; STEPS + 1];
    let d = TAU / STEPS as f64;
    for i in 0..STEPS {
        t[i + 1] = t[i] + d / omega(k, from + dir * (i as f64 + 0.5) * d);
    }
    let total = t[STEPS];
    for v in t.iter_mut() {
        *v /= total;
    }
    t
}

/// θ a share of the way through a revolution (unwrapped: it goes on growing,
/// or shrinking in reverse).
pub fn theta_at(p: &[f32], frac: f64) -> f64 {
    let k = warp(p);
    let dir = if p[REVERSE] >= 0.5 { -1.0 } else { 1.0 };
    if oscillates(p) {
        // About the bottom, turned by the offset; from an even to and fro to a pendulum's.
        let reach = p[AMPLITUDE].clamp(0.05, 1.0) as f64 * PI;
        let f = frac.rem_euclid(1.0);
        let tri = if f < 0.25 {
            4.0 * f
        } else if f < 0.75 {
            2.0 - 4.0 * f
        } else {
            4.0 * f - 4.0
        };
        return -FRAC_PI_2 + offset(p) + dir * reach * ((1.0 - k) * tri + k * (TAU * f).sin());
    }
    let from = offset(p);
    let t = table(k, from, dir);
    let turns = frac.floor();
    let f = frac - turns;
    let i = t.partition_point(|v| *v <= f).clamp(1, STEPS) - 1;
    let within = (f - t[i]) / (t[i + 1] - t[i]).max(1e-12);
    from + dir * (turns * TAU + (i as f64 + within) * TAU / STEPS as f64)
}

/// θ at a beat of the song.
pub fn theta_at_beat(p: &[f32], beat: f64) -> f64 {
    theta_at(p, beat / period_beats(p))
}

/// How fast P goes at θ, 0 (still) to 1 (its fastest).
pub fn speed_at(p: &[f32], theta: f64) -> f64 {
    if oscillates(p) {
        let reach = p[AMPLITUDE].clamp(0.05, 1.0) as f64 * PI;
        let centre = -FRAC_PI_2 + offset(p);
        let d = (theta - centre + PI).rem_euclid(TAU) - PI;
        return (1.0 - d.abs() / reach).clamp(0.0, 1.0);
    }
    let k = warp(p);
    omega(k, theta) / omega(k, -FRAC_PI_2)
}

/// tan θ kept in sight: its leaps cut off at ±2.
pub fn tan_clipped(theta: f64) -> f64 {
    clipped(theta.tan())
}

/// A value that runs off to infinity, cut off at ±2.
pub fn clipped(v: f64) -> f64 {
    if v.is_finite() {
        v.clamp(-2.0, 2.0)
    } else {
        2.0
    }
}

/// A function of θ as it is (1 sin, 2 cos, 3 tan, 4 sec, 5 csc, 6 cot), cut off at ±2.
pub fn value(which: usize, theta: f64) -> f64 {
    match which {
        1 => theta.sin(),
        2 => theta.cos(),
        3 => clipped(theta.tan()),
        4 => clipped(1.0 / theta.cos()),
        5 => clipped(1.0 / theta.sin()),
        6 => clipped(theta.cos() / theta.sin()),
        _ => 0.0,
    }
}

/// A function of θ (0: none, then as `value`) from −1 to 1.
pub fn function(which: usize, theta: f64) -> f64 {
    match which {
        1 | 2 => value(which, theta),
        3..=6 => value(which, theta) / 2.0,
        _ => 0.0,
    }
}

/// A choice param's value as an index into its list.
fn choice(p: &[f32], param: usize, len: usize) -> usize {
    (p.get(param).copied().unwrap_or(0.0).round().max(0.0) as usize).min(len - 1)
}

/// How long a pass plays, as a share of the gate.
pub fn pass_gate(p: &[f32], theta: f64) -> f64 {
    match choice(p, GATE_FROM, GATES.len()) {
        0 => 1.0,
        1 => 0.1 + 0.9 * speed_at(p, theta),
        w => 0.1 + 0.9 * function(w - 1, theta).abs(),
    }
}

/// Octaves up or down a pass goes (−1..1).
pub fn pass_octave(p: &[f32], theta: f64) -> i32 {
    match choice(p, OCTAVE_FROM, OCTAVES.len()) {
        0 => 0,
        w => function(w, theta).round() as i32,
    }
}

/// Where the mod wheel is (0..1) with P at θ; None: it is left alone.
pub fn mod_wheel(p: &[f32], theta: f64) -> Option<f32> {
    match choice(p, MOD_FROM, MODS.len()) {
        0 => None,
        7 => Some(speed_at(p, theta) as f32),
        w => Some(((function(w, theta) + 1.0) * 0.5) as f32),
    }
}

/// The note on point `k` (steps of the key above the base note), or a rest.
pub fn point_note(p: &[f32], k: usize) -> Option<i32> {
    let v = p.get(FIRST_POINT + k).copied().unwrap_or(0.0).round() as i32;
    (v > 0).then_some(v - 1)
}

/// The points P passes between two beats, in order: (beat, point, phase).
pub fn passes(p: &[f32], b0: f64, b1: f64, out: &mut impl FnMut(f64, usize, usize)) {
    let (a0, a1) = (theta_at_beat(p, b0), theta_at_beat(p, b1));
    if (a1 - a0).abs() < 1e-12 {
        return;
    }
    let (lo, hi) = (a0.min(a1), a0.max(a1));
    let mut hits: [(f64, usize, usize); 4 * MAX_POINTS * MAX_PHASES] = [(0.0, 0, 0); 4 * MAX_POINTS * MAX_PHASES];
    let mut n = 0;
    let r_count = phases(p);
    for r in 0..r_count {
        // Phase r is 2πr/R ahead of P: it is on point k when θ = θₖ − 2πr/R.
        let lead = TAU * r as f64 / r_count as f64;
        for k in 0..points(p) {
            let a = point_angle(p, k) - lead;
            let mut m = ((lo - a) / TAU).floor();
            while a + m * TAU <= hi {
                let x = a + m * TAU;
                let crossed = if a1 > a0 { x > a0 && x <= a1 } else { x < a0 && x >= a1 };
                if crossed && n < hits.len() {
                    hits[n] = (b0 + (x - a0) / (a1 - a0) * (b1 - b0), k, r);
                    n += 1;
                }
                m += 1.0;
            }
        }
    }
    let hits = &mut hits[..n];
    hits.sort_by(|a, b| a.0.total_cmp(&b.0));
    for &(beat, k, r) in hits.iter() {
        out(beat, k, r);
    }
}

/// Which voices play: [0] the points' notes, then sin θ ... cot θ, each its
/// own melody at once. A set from before the switches plays what its Pitch said.
pub fn voices(p: &[f32]) -> [bool; VOICES] {
    let old = (p[PITCH].round().max(0.0) as usize).min(PITCHES.len() - 1);
    let on = |v: usize| p.get(PLAY_POINTS + v).is_some_and(|x| *x >= 0.5);
    let mut out = [false; VOICES];
    out[0] = on(0) && old == 0;
    for (f, slot) in out.iter_mut().enumerate().skip(1) {
        *slot = on(f) || old == f;
    }
    out
}

/// The step of the key a pass plays in the voice that Pitch names.
pub fn pass_step(p: &[f32], k: usize, theta: f64, revolution: i64) -> Option<i32> {
    voice_step(p, k, (p[PITCH].round().max(0.0) as usize).min(PITCHES.len() - 1), theta, revolution)
}

/// The step of the key voice `which` plays as P passes point `k`: the point's
/// note (0), or the function's value at θ; with the climb so far. A rest is a
/// rest in every voice.
pub fn voice_step(p: &[f32], k: usize, which: usize, theta: f64, revolution: i64) -> Option<i32> {
    let note = point_note(p, k)?;
    let base = if which == 0 {
        note
    } else {
        let m = p[HARMONIC].round().clamp(1.0, 8.0) as f64;
        let range = p[RANGE].round().clamp(1.0, 24.0) as f64;
        (function(which, m * theta) * range * 0.5).round() as i32
    };
    let turns = p[CLIMB_TURNS].round().clamp(1.0, 8.0) as i64;
    Some(base + p[CLIMB].round() as i32 * revolution.rem_euclid(turns) as i32)
}

/// How hard a pass plays (before the overall velocity), 0..1.
pub fn pass_level(p: &[f32], theta: f64) -> f64 {
    match (p[VEL_FROM].round().max(0.0) as usize).min(VELOCITIES.len() - 1) {
        1 => 0.35 + 0.65 * speed_at(p, theta),
        0 => 1.0,
        w => 0.2 + 0.8 * function(w - 1, theta).abs(),
    }
}

/// Fills for the points: arpeggios, scales, chords and the like.
pub const FILLS: &[&str] = &["Arp Up", "Arp Down", "Arp Up-Down", "Scale Up", "Scale Down", "Chord", "Fifths", "Octaves", "Pentatonic", "Every Other", "Random", "Clear"];

/// The points' values (0 a rest, else a step of the key plus one) for a fill.
pub fn fill(which: usize, n: usize, seed: u32) -> Vec<f32> {
    let arp = [0, 2, 4, 7, 9, 11, 14];
    let penta = [0, 1, 2, 4, 5, 7, 8, 9, 11, 12, 14];
    (0..n)
        .map(|k| {
            let step: Option<i32> = match which {
                0 => Some(arp[k % arp.len()]),
                1 => Some(arp[(arp.len() - 1) - k % arp.len()]),
                2 => {
                    let period = 2 * (arp.len() - 1);
                    let i = k % period;
                    Some(arp[if i < arp.len() { i } else { period - i }])
                }
                3 => Some((k % 15) as i32),
                4 => Some(14 - (k % 15) as i32),
                5 => Some([0, 2, 4][k % 3]),
                6 => Some([0, 4, 7, 11][k % 4]),
                7 => Some([0, 7, 14, 7][k % 4]),
                8 => Some(penta[k % penta.len()]),
                9 => (k % 2 == 0).then_some(arp[(k / 2) % arp.len()]),
                10 => (hash(seed, k as u32) < 0.8).then(|| (hash(seed ^ 0x5bd1, k as u32) * 12.0) as i32),
                _ => None,
            };
            step.map_or(0.0, |s| (s.clamp(0, 14) + 1) as f32)
        })
        .collect()
}

impl SeqState {
    pub(crate) fn circle(&mut self, p: &[f32], out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        let (b0, b1) = (ctx.beat, ctx.beat + ctx.len as f64 / ctx.samples_per_beat);
        // Short stretches, so that a fast revolution keeps its curve.
        let period = period_beats(p);
        let pieces = (((b1 - b0) / (period / 64.0)).ceil() as usize).clamp(1, 64);
        let (home, _) = ctx.scale.locate(p[BASE].round() as i32);
        let chance = p[CHANCE].clamp(0.0, 1.0);
        for s in 0..pieces {
            let (c0, c1) = (b0 + (b1 - b0) * s as f64 / pieces as f64, b0 + (b1 - b0) * (s + 1) as f64 / pieces as f64);
            let mut due: [(f64, usize, usize); 2 * MAX_POINTS * MAX_PHASES] = [(0.0, 0, 0); 2 * MAX_POINTS * MAX_PHASES];
            let mut n = 0;
            passes(p, c0, c1, &mut |beat, k, r| {
                if n < due.len() {
                    due[n] = (beat, k, r);
                    n += 1;
                }
            });
            for &(beat, k, r) in &due[..n] {
                let slot = r * MAX_POINTS + k;
                // A pass on the edge of two stretches is seen by both: it plays once.
                if beat <= self.circle_last[slot] + 1e-6 {
                    continue;
                }
                self.circle_last[slot] = beat;
                let revolution = (beat / period).floor() as i64;
                if chance < 1.0 && hash(slot as u32 * 131 + 7, revolution as u32) >= chance {
                    continue;
                }
                let theta = theta_at_beat(p, beat);
                let vel = p[VELOCITY] * pass_level(p, theta) as f32;
                let octave = pass_octave(p, theta) * ctx.scale.len().max(1) as i32;
                let gate = p[GATE] as f64 * pass_gate(p, theta);
                // Every voice that plays, each note once.
                let mut sounded = [i32::MIN; VOICES];
                for (v, on) in voices(p).into_iter().enumerate() {
                    let Some(step) = voice_step(p, k, v, theta, revolution).filter(|_| on) else { continue };
                    // Each voice its own transpose, in steps of the key.
                    let shift = p.get(SHIFT_FIRST + v).copied().unwrap_or(0.0).round() as i32;
                    let pitch = ctx.scale.pitch_at(home + step + octave + shift);
                    if sounded.contains(&pitch) {
                        continue;
                    }
                    sounded[v] = pitch;
                    self.play(ctx, frame_of(ctx, beat), pitch, vel, gate * ctx.samples_per_beat, out);
                }
            }
        }
        // For the editor: where in its revolution P is.
        self.visual[1] = ((b1 / period).rem_euclid(1.0) * 65535.0) as u64;
        // The mod wheel, as P is now.
        self.circle_mod = mod_wheel(p, theta_at_beat(p, b1));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::midi_fx::MidiFxKind;
    use crate::params::defaults;
    use crate::scale::{Scale, ScaleKind};
    use crate::sequencers::tests::run;

    fn circle() -> Vec<f32> {
        defaults(MidiFxKind::Circle.specs())
    }

    fn all_points(mut p: Vec<f32>) -> Vec<f32> {
        for k in 0..MAX_POINTS {
            p[FIRST_POINT + k] = 1.0;
        }
        p
    }

    /// The passes over so many beats, a little at a time as the engine sees them.
    fn hits(p: &[f32], beats: f64) -> Vec<(f64, usize, usize)> {
        let mut t = Vec::new();
        // Counted, not added up: a sum of hundredths falls short of the end.
        let chunks = (beats * 100.0).round() as usize;
        for i in 0..chunks {
            passes(p, i as f64 / 100.0, (i + 1) as f64 / 100.0, &mut |at, k, r| t.push((at, k, r)));
        }
        t
    }

    fn one_bar(mut p: Vec<f32>) -> Vec<f32> {
        p[PERIOD] = 4.0;
        p
    }

    #[test]
    fn without_warp_the_points_come_evenly_from_theta_zero() {
        let mut p = one_bar(circle());
        p[WARP] = 0.0;
        p[POINTS] = 8.0;
        let t = hits(&p, 4.0);
        assert_eq!(t.len(), 8, "{t:?}");
        for (k, h) in t.iter().enumerate() {
            assert!((h.0 - (k + 1) as f64 * 0.5).abs() < 1e-6, "{k}: {h:?}");
            assert_eq!(h.1, (k + 1) % 8, "anticlockwise, point by point");
        }
    }

    #[test]
    fn with_warp_it_is_slow_at_the_top_and_fast_at_the_bottom() {
        let mut p = one_bar(circle());
        p[WARP] = 1.0;
        p[POINTS] = 8.0;
        let t: Vec<f64> = hits(&p, 4.0).into_iter().map(|h| h.0).collect();
        // Points 1, 2, 3 lie about the top (π/2), 5, 6, 7 about the bottom (3π/2).
        let gap = |k: usize| t[k] - t[k - 1];
        assert!(gap(2) > gap(6) * 2.5, "{t:?}");
        assert!((t[7] - 4.0).abs() < 1e-6, "still a revolution in its bar");
    }

    #[test]
    fn it_goes_round_and_round_and_in_reverse_the_other_way() {
        let p = circle();
        let one = hits(&p, period_beats(&p));
        let two = hits(&p, 2.0 * period_beats(&p));
        assert_eq!(two.len(), 2 * one.len());
        let mut q = one_bar(p.clone());
        q[WARP] = 0.0;
        q[REVERSE] = 1.0;
        q[POINTS] = 4.0;
        let order: Vec<usize> = hits(&q, 4.0).into_iter().map(|h| h.1).collect();
        assert_eq!(order, [3, 2, 1, 0]);
    }

    #[test]
    fn more_phases_play_more_points_p() {
        let mut p = all_points(one_bar(circle()));
        p[WARP] = 0.0;
        p[POINTS] = 6.0;
        p[PHASES] = 2.0;
        let t = hits(&p, 4.0);
        assert_eq!(t.len(), 12);
        assert_eq!(t.iter().filter(|h| h.2 == 1).count(), 6);
    }

    #[test]
    fn oscillating_it_plays_only_what_it_reaches_both_ways() {
        let mut p = all_points(one_bar(circle()));
        p[MOTION] = 1.0;
        p[AMPLITUDE] = 0.45;
        p[POINTS] = 8.0;
        let mut seen: Vec<usize> = hits(&p, 4.0).into_iter().map(|h| h.1).collect();
        seen.sort();
        // About the bottom (point 6, 3π/2), 81° either way: points 5, 6, 7.
        assert_eq!(seen, [5, 5, 6, 6, 7, 7]);
    }

    #[test]
    fn the_pitch_can_follow_sin_cos_or_tan() {
        let mut p = circle();
        p[RANGE] = 8.0;
        p[PITCH] = 1.0;
        assert_eq!(pass_step(&p, 0, FRAC_PI_2, 0), Some(4), "sin π/2 = 1: the top of the range");
        assert_eq!(pass_step(&p, 0, -FRAC_PI_2, 0), Some(-4));
        p[PITCH] = 2.0;
        assert_eq!(pass_step(&p, 0, 0.0, 0), Some(4), "cos 0 = 1");
        p[PITCH] = 3.0;
        assert_eq!(pass_step(&p, 0, 1.5, 0), Some(4), "tan near π/2: cut off at the top");
        p[HARMONIC] = 2.0;
        p[PITCH] = 1.0;
        assert_eq!(pass_step(&p, 0, FRAC_PI_2 / 2.0, 0), Some(4), "sin 2θ");
        p[FIRST_POINT] = 0.0;
        assert_eq!(pass_step(&p, 0, 0.0, 0), None, "a rest stays a rest");
    }

    #[test]
    fn sec_csc_and_cot_are_what_they_are_and_cut_off_at_their_asymptotes() {
        assert!((value(4, 0.0) - 1.0).abs() < 1e-9, "sec 0 = 1");
        assert!((value(5, FRAC_PI_2) - 1.0).abs() < 1e-9, "csc π/2 = 1");
        assert!((value(6, PI / 4.0) - 1.0).abs() < 1e-9, "cot π/4 = 1");
        assert_eq!(value(4, FRAC_PI_2), 2.0, "sec at π/2 runs off: cut");
        assert_eq!(value(6, 0.0), 2.0);
        let mut p = circle();
        p[PITCH] = 4.0;
        p[RANGE] = 8.0;
        assert_eq!(pass_step(&p, 0, 0.0, 0), Some(2), "sec 0 = 1 is half its range of ±2");
    }

    #[test]
    fn the_functions_set_length_octave_and_the_mod_wheel() {
        let mut p = circle();
        assert_eq!(pass_gate(&p, 1.0), 1.0, "fixed");
        p[GATE_FROM] = 2.0;
        assert!((pass_gate(&p, FRAC_PI_2) - 1.0).abs() < 1e-9, "|sin π/2|");
        assert!((pass_gate(&p, 0.0) - 0.1).abs() < 1e-9);
        assert_eq!(pass_octave(&p, 0.0), 0);
        p[OCTAVE_FROM] = 2.0;
        assert_eq!(pass_octave(&p, 0.0), 1, "cos 0");
        assert_eq!(pass_octave(&p, PI), -1);
        assert_eq!(mod_wheel(&p, 0.0), None);
        p[MOD_FROM] = 2.0;
        assert_eq!(mod_wheel(&p, 0.0), Some(1.0), "cos 0 = 1: all the way up");
        assert_eq!(mod_wheel(&p, PI), Some(0.0));
    }

    #[test]
    fn it_turns_the_mod_wheel_as_it_goes() {
        use crate::midi_fx::{MidiCtx, MidiFxState};
        let mut p = circle();
        p[MOD_FROM] = 1.0;
        p[PERIOD] = 4.0;
        let spb = 24000.0;
        let mut st = MidiFxState::new(MidiFxKind::Circle);
        let mut seen = Vec::new();
        for b in 0..(4.0 * spb / 256.0) as u64 {
            let now = b * 256;
            let ctx = MidiCtx { now, len: 256, samples_per_beat: spb, scale: Scale::new(0, ScaleKind::Major), scale_on: true, beat: now as f64 / spb, playing: true, sr: 48000.0, drum_notes: 0, lead: 0.0, touch: [0.0; 5], band: Default::default() };
            let mut out = Vec::new();
            st.process(MidiFxKind::Circle, &p, &[], &[], &mut out, &ctx);
            seen.push(st.circle_out().unwrap());
        }
        let (lo, hi) = seen.iter().fold((1.0f32, 0.0f32), |m, v| (m.0.min(*v), m.1.max(*v)));
        assert!(lo < 0.05 && hi > 0.95, "{lo} {hi}");
    }

    #[test]
    fn several_voices_play_at_once_each_its_own_melody() {
        let mut p = all_points(circle());
        p[PERIOD] = 4.0;
        let points_only = run(MidiFxKind::Circle, &p, &MidiFxKind::Circle.default_data(), 4.0).len();
        p[PLAY_POINTS + 1] = 1.0;
        p[PLAY_POINTS + 2] = 1.0;
        p[PLAY_POINTS + 3] = 1.0;
        let four = run(MidiFxKind::Circle, &p, &MidiFxKind::Circle.default_data(), 4.0).len();
        assert!(four > points_only * 2, "{points_only} {four}");
        assert_eq!(voices(&p), [true, true, true, true, false, false, false]);
        p[PLAY_POINTS] = 0.0;
        assert!(!voices(&p)[0], "the points' own notes can be left out");
        // A set from before the switches: what its Pitch said, and not the points.
        let mut old = circle();
        old[PITCH] = 2.0;
        assert_eq!(voices(&old), [false, false, true, false, false, false, false]);
    }

    #[test]
    fn each_voice_can_be_transposed_on_its_own() {
        let mut p = all_points(circle());
        p[PERIOD] = 4.0;
        p[POINTS] = 4.0;
        p[PLAY_POINTS + 2] = 1.0;
        let plain = run(MidiFxKind::Circle, &p, &MidiFxKind::Circle.default_data(), 4.0);
        p[SHIFT_FIRST + 2] = 7.0;
        let shifted = run(MidiFxKind::Circle, &p, &MidiFxKind::Circle.default_data(), 4.0);
        // The cos voice an octave (seven steps of C major) up, the points where they were.
        let top = |v: &[(f64, u8)]| v.iter().map(|o| o.1).max().unwrap();
        assert_eq!(top(&shifted), top(&plain) + 12, "{plain:?} {shifted:?}");
        assert!(plain.iter().any(|o| o.1 == 60) && shifted.iter().any(|o| o.1 == 60), "the points' C4 stays where it was");
        assert_eq!(CIRCLE[SHIFT_FIRST].name, "Shift Points");
        assert_eq!(CIRCLE[PLAY_POINTS].name, "Play Points");
    }

    #[test]
    fn it_climbs_a_step_each_revolution_and_comes_back() {
        let mut p = circle();
        p[CLIMB] = 2.0;
        p[CLIMB_TURNS] = 3.0;
        let s: Vec<i32> = (0..4).map(|r| pass_step(&p, 0, 0.0, r).unwrap()).collect();
        assert_eq!(s, [0, 2, 4, 0]);
    }

    #[test]
    fn its_notes_are_in_the_key() {
        for pitch in 0..4 {
            let mut p = circle();
            p[PITCH] = pitch as f32;
            p[PERIOD] = 2.0;
            let ons = run(MidiFxKind::Circle, &p, &MidiFxKind::Circle.default_data(), 8.0);
            assert!(ons.len() >= 8, "{pitch}: {}", ons.len());
            let c = Scale::new(0, ScaleKind::Major);
            assert!(ons.iter().all(|o| c.contains(o.1)), "{ons:?}");
        }
    }

    #[test]
    fn fast_it_still_plays_every_point() {
        let mut p = all_points(circle());
        p[PERIOD] = 0.0; // a quarter of a beat
        p[POINTS] = 8.0;
        let ons = run(MidiFxKind::Circle, &p, &MidiFxKind::Circle.default_data(), 4.0);
        // Sixteen revolutions of eight points.
        assert!((ons.len() as i64 - 128).abs() <= 2, "{}", ons.len());
    }

    #[test]
    fn chance_leaves_some_out() {
        let mut p = all_points(circle());
        p[CHANCE] = 0.5;
        p[PERIOD] = 2.0;
        let some = run(MidiFxKind::Circle, &p, &MidiFxKind::Circle.default_data(), 16.0).len();
        p[CHANCE] = 1.0;
        let all = run(MidiFxKind::Circle, &p, &MidiFxKind::Circle.default_data(), 16.0).len();
        assert!(some < all * 3 / 4 && some > all / 4, "{some} of {all}");
    }

    #[test]
    fn the_fills_are_what_they_say() {
        assert_eq!(fill(0, 4, 0), [1.0, 3.0, 5.0, 8.0], "an arpeggio up");
        assert_eq!(fill(5, 4, 0), [1.0, 3.0, 5.0, 1.0], "a chord");
        assert_eq!(fill(11, 3, 0), [0.0; 3], "clear");
        assert_eq!(fill(9, 4, 0)[1], 0.0, "every other");
        assert!(fill(10, 16, 1) != fill(10, 16, 2), "random");
        for w in 0..FILLS.len() {
            assert!(fill(w, 16, 3).iter().all(|v| (0.0..=15.0).contains(v)), "{}", FILLS[w]);
        }
    }

    #[test]
    fn a_set_with_the_wheel_in_it_still_loads() {
        let k: MidiFxKind = serde_json::from_str("\"Wheel\"").unwrap();
        assert_eq!(k, MidiFxKind::Circle);
    }

    #[test]
    fn theta_runs_on_smoothly() {
        let mut p = circle();
        p[WARP] = 0.8;
        let mut last = theta_at(&p, 0.0);
        for k in 1..=400 {
            let a = theta_at(&p, k as f64 / 200.0);
            assert!(a > last, "{k}");
            last = a;
        }
        assert!((theta_at(&p, 1.0) - TAU).abs() < 1e-9);
    }
}

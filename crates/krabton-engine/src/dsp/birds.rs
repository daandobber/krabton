//! Birds: songbirds that sing by themselves. Every bird has a place (left
//! or right, near or far) and a repertoire of songs of its species, built
//! from syllables the way birds sing them: whistles that glide and bend,
//! trills, harsh chirps full of harmonics, hoots with breath in them, the
//! drumming of a woodpecker. A bird repeats a song type a while before it
//! takes another, as real birds do; between songs it sometimes flies to
//! another branch. Far birds sound softer, duller and wetter.
//!
//! Play a key: the bird whose voice is nearest the key sings at once, and
//! Follow Key moves its song onto the key. They sing while the song plays,
//! always, or while a key is held (Sings). Original code.

use super::effects::FxCtx;
use super::fx::hall::{self, Hall};
use super::{db_to_gain, midi_to_freq, pan_gains, OnePole, Rng, Svf};
use crate::params::{ParamSpec, Unit};
use std::f32::consts::{LN_2, PI, TAU};

pub const SINGS: usize = 0;
pub const BIRD_1: usize = 1;
pub const NUMBER_1: usize = 2;
pub const BIRD_2: usize = 3;
pub const NUMBER_2: usize = 4;
pub const BIRD_3: usize = 5;
pub const NUMBER_3: usize = 6;
pub const BUSY: usize = 7;
pub const VARIETY: usize = 8;
pub const TEMPO: usize = 9;
pub const PITCH: usize = 10;
pub const FOLLOW: usize = 11;
pub const DISTANCE: usize = 12;
pub const WIDTH: usize = 13;
pub const SPACE: usize = 14;
pub const VOLUME: usize = 15;

pub const SING_MODES: &[&str] = &["While Playing", "Always", "While a Key Is Held"];
pub const SPECIES: &[&str] = &["Blackbird", "Robin", "Song Thrush", "Nightingale", "Wren", "Chaffinch", "Great Tit", "Skylark", "Sparrow", "Cuckoo", "Wood Pigeon", "Tawny Owl", "Loon", "Gull", "Woodpecker", "Blackcap", "Chiffchaff", "Willow Warbler", "Yellowhammer", "Collared Dove", "Golden Oriole", "Crow", "Curlew", "Swift", "Hermit Thrush", "Wood Thrush", "Cardinal", "Mourning Dove", "Bellbird", "Kookaburra"];

/// Three kinds of bird, up to four of each.
pub const SLOTS: usize = 3;
pub const PER_SLOT: usize = 4;
pub const MAX_BIRDS: usize = SLOTS * PER_SLOT;

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::choice("Sings", "Birds", SING_MODES, 0),
    ParamSpec::choice("Bird 1", "Birds", SPECIES, 0),
    ParamSpec::int("Number 1", "Birds", 0.0, 4.0, 1.0, Unit::None),
    ParamSpec::choice("Bird 2", "Birds", SPECIES, 1),
    ParamSpec::int("Number 2", "Birds", 0.0, 4.0, 1.0, Unit::None),
    ParamSpec::choice("Bird 3", "Birds", SPECIES, 6),
    ParamSpec::int("Number 3", "Birds", 0.0, 4.0, 1.0, Unit::None),
    ParamSpec::lin("Busy", "Song", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Variety", "Song", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Tempo", "Song", 0.5, 2.0, 1.0, Unit::Ratio),
    ParamSpec::lin("Pitch", "Song", -12.0, 12.0, 0.0, Unit::Semi),
    ParamSpec::lin("Follow Key", "Song", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Distance", "Place", 0.0, 1.0, 0.35, Unit::Percent),
    ParamSpec::lin("Width", "Place", 0.0, 1.0, 0.8, Unit::Percent),
    ParamSpec::lin("Space", "Place", 0.0, 1.0, 0.35, Unit::Percent),
    ParamSpec::lin("Volume", "Place", -24.0, 12.0, 0.0, Unit::Db),
];

pub fn preset_names() -> &'static [&'static str] {
    &["Garden Morning", "Dawn Chorus", "Woodland", "Woodland Edge", "Nightingale Night", "Meadow", "Moorland", "Lake at Dusk", "Seaside", "Village", "Summer Evening", "Lone Blackbird", "Owl and Cuckoo", "Hermit Thrush Forest", "Bellbird Valley", "Outback"]
}

pub fn preset(name: &str) -> Vec<f32> {
    let mut p = crate::params::defaults(SPECS);
    let sp = |n: &str| SPECIES.iter().position(|s| *s == n).unwrap_or(0) as f32;
    let mut birds = |a: (&str, f32), b: (&str, f32), c: (&str, f32)| {
        p[BIRD_1] = sp(a.0);
        p[NUMBER_1] = a.1;
        p[BIRD_2] = sp(b.0);
        p[NUMBER_2] = b.1;
        p[BIRD_3] = sp(c.0);
        p[NUMBER_3] = c.1;
    };
    let mut more: &[(usize, f32)] = &[];
    match name {
        "Dawn Chorus" => {
            birds(("Song Thrush", 1.0), ("Robin", 2.0), ("Wren", 1.0));
            more = &[(BUSY, 0.8), (VARIETY, 0.6), (DISTANCE, 0.45), (SPACE, 0.4)];
        }
        "Woodland" => {
            birds(("Chaffinch", 2.0), ("Woodpecker", 1.0), ("Cuckoo", 1.0));
            more = &[(BUSY, 0.55), (DISTANCE, 0.5), (SPACE, 0.45)];
        }
        "Nightingale Night" => {
            birds(("Nightingale", 1.0), ("Tawny Owl", 1.0), ("Nightingale", 0.0));
            more = &[(BUSY, 0.75), (DISTANCE, 0.4), (SPACE, 0.55)];
        }
        "Meadow" => {
            birds(("Skylark", 2.0), ("Sparrow", 2.0), ("Great Tit", 1.0));
            more = &[(BUSY, 0.6), (DISTANCE, 0.5), (SPACE, 0.25)];
        }
        "Lake at Dusk" => {
            birds(("Loon", 2.0), ("Tawny Owl", 0.0), ("Wood Pigeon", 1.0));
            more = &[(BUSY, 0.45), (DISTANCE, 0.65), (SPACE, 0.75), (WIDTH, 1.0)];
        }
        "Seaside" => {
            birds(("Gull", 3.0), ("Sparrow", 0.0), ("Gull", 0.0));
            more = &[(BUSY, 0.5), (DISTANCE, 0.55), (SPACE, 0.3), (WIDTH, 1.0)];
        }
        "Village" => {
            birds(("Sparrow", 4.0), ("Wood Pigeon", 1.0), ("Great Tit", 1.0));
            more = &[(BUSY, 0.65), (DISTANCE, 0.3), (SPACE, 0.2)];
        }
        "Lone Blackbird" => {
            birds(("Blackbird", 1.0), ("Robin", 0.0), ("Great Tit", 0.0));
            more = &[(BUSY, 0.7), (VARIETY, 0.7), (DISTANCE, 0.25), (SPACE, 0.6)];
        }
        "Woodland Edge" => {
            birds(("Blackcap", 1.0), ("Chiffchaff", 1.0), ("Willow Warbler", 1.0));
            more = &[(BUSY, 0.6), (VARIETY, 0.6), (DISTANCE, 0.35), (SPACE, 0.35)];
        }
        "Moorland" => {
            birds(("Curlew", 2.0), ("Skylark", 1.0), ("Cuckoo", 1.0));
            more = &[(BUSY, 0.5), (DISTANCE, 0.55), (SPACE, 0.45), (WIDTH, 1.0)];
        }
        "Summer Evening" => {
            birds(("Swift", 3.0), ("Blackbird", 1.0), ("Collared Dove", 1.0));
            more = &[(BUSY, 0.6), (DISTANCE, 0.4), (SPACE, 0.4), (WIDTH, 1.0)];
        }
        "Hermit Thrush Forest" => {
            birds(("Hermit Thrush", 1.0), ("Wood Thrush", 1.0), ("Mourning Dove", 1.0));
            more = &[(BUSY, 0.55), (VARIETY, 0.6), (DISTANCE, 0.45), (SPACE, 0.65)];
        }
        "Bellbird Valley" => {
            birds(("Bellbird", 2.0), ("Hermit Thrush", 1.0), ("Golden Oriole", 1.0));
            more = &[(BUSY, 0.55), (DISTANCE, 0.45), (SPACE, 0.7), (WIDTH, 1.0)];
        }
        "Outback" => {
            birds(("Kookaburra", 2.0), ("Crow", 1.0), ("Cardinal", 0.0));
            more = &[(BUSY, 0.45), (DISTANCE, 0.55), (SPACE, 0.35), (WIDTH, 1.0)];
        }
        "Owl and Cuckoo" => {
            birds(("Tawny Owl", 1.0), ("Cuckoo", 1.0), ("Wood Pigeon", 1.0));
            more = &[(BUSY, 0.45), (DISTANCE, 0.6), (SPACE, 0.6)];
        }
        _ => {}
    }
    for &(i, v) in more {
        p[i] = v;
    }
    p
}

/// Per species: the pitch its song sits around (Hz), how long it rests
/// between songs (s), how many songs of one type before another, and how
/// far off it usually is (added to Distance).
struct Kind {
    home: f32,
    rest: (f32, f32),
    repeat: (u32, u32),
    far: f32,
}

const KINDS: [Kind; 30] = [
    Kind { home: 2100.0, rest: (2.5, 6.0), repeat: (1, 1), far: 0.0 },
    Kind { home: 4500.0, rest: (2.5, 6.0), repeat: (1, 2), far: 0.0 },
    Kind { home: 3000.0, rest: (0.6, 2.0), repeat: (1, 1), far: 0.05 },
    Kind { home: 2600.0, rest: (1.0, 3.0), repeat: (1, 2), far: 0.05 },
    Kind { home: 5500.0, rest: (4.0, 9.0), repeat: (2, 4), far: 0.0 },
    Kind { home: 4300.0, rest: (5.0, 9.0), repeat: (3, 8), far: 0.05 },
    Kind { home: 4800.0, rest: (3.0, 8.0), repeat: (3, 7), far: 0.0 },
    Kind { home: 4200.0, rest: (4.0, 10.0), repeat: (1, 1), far: 0.35 },
    Kind { home: 3300.0, rest: (1.0, 4.0), repeat: (2, 5), far: 0.0 },
    Kind { home: 620.0, rest: (8.0, 18.0), repeat: (1, 1), far: 0.35 },
    Kind { home: 480.0, rest: (7.0, 15.0), repeat: (1, 1), far: 0.1 },
    Kind { home: 470.0, rest: (12.0, 25.0), repeat: (1, 1), far: 0.3 },
    Kind { home: 850.0, rest: (9.0, 20.0), repeat: (1, 1), far: 0.45 },
    Kind { home: 1150.0, rest: (4.0, 12.0), repeat: (1, 2), far: 0.25 },
    Kind { home: 1050.0, rest: (6.0, 14.0), repeat: (1, 1), far: 0.3 },
    Kind { home: 2800.0, rest: (2.0, 6.0), repeat: (1, 2), far: 0.05 },
    Kind { home: 5200.0, rest: (3.0, 8.0), repeat: (2, 5), far: 0.05 },
    Kind { home: 4200.0, rest: (3.0, 7.0), repeat: (1, 3), far: 0.05 },
    Kind { home: 5000.0, rest: (4.0, 9.0), repeat: (3, 6), far: 0.1 },
    Kind { home: 480.0, rest: (6.0, 14.0), repeat: (1, 1), far: 0.1 },
    Kind { home: 1700.0, rest: (4.0, 10.0), repeat: (1, 3), far: 0.15 },
    Kind { home: 750.0, rest: (6.0, 15.0), repeat: (1, 1), far: 0.3 },
    Kind { home: 2500.0, rest: (8.0, 18.0), repeat: (1, 1), far: 0.4 },
    Kind { home: 7000.0, rest: (3.0, 8.0), repeat: (1, 1), far: 0.2 },
    Kind { home: 3800.0, rest: (2.0, 5.0), repeat: (1, 1), far: 0.15 },
    Kind { home: 2400.0, rest: (2.0, 5.0), repeat: (1, 2), far: 0.1 },
    Kind { home: 3000.0, rest: (3.0, 8.0), repeat: (2, 4), far: 0.05 },
    Kind { home: 480.0, rest: (8.0, 15.0), repeat: (1, 1), far: 0.15 },
    Kind { home: 2600.0, rest: (2.0, 6.0), repeat: (1, 2), far: 0.1 },
    Kind { home: 1600.0, rest: (10.0, 25.0), repeat: (1, 1), far: 0.3 },
];

/// One syllable: a tone gliding from `f0` to `f1` (bent up or down in the
/// middle by `bend` octaves), with vibrato or trill (fm), tremolo (am), four
/// harmonics and a band of breath or rasp around the pitch.
#[derive(Clone, Copy, Debug)]
struct Syl {
    dur: f32,
    gap: f32,
    f0: f32,
    f1: f32,
    bend: f32,
    fm_rate: f32,
    fm_depth: f32,
    am_rate: f32,
    am_depth: f32,
    h: [f32; 4],
    noise: f32,
    q: f32,
    att: f32,
    rel: f32,
    amp: f32,
}

/// A clear whistle.
fn whistle(dur: f32, f0: f32, f1: f32, bend: f32, amp: f32, gap: f32) -> Syl {
    Syl {
        dur,
        gap,
        f0,
        f1,
        bend,
        fm_rate: 0.0,
        fm_depth: 0.0,
        am_rate: 0.0,
        am_depth: 0.0,
        h: [1.0, 0.1, 0.03, 0.0],
        noise: 0.02,
        q: 8.0,
        att: (dur * 0.15).clamp(0.002, 0.04),
        rel: (dur * 0.25).clamp(0.003, 0.08),
        amp,
    }
}

/// A rough note, rich in harmonics.
fn harsh(dur: f32, f0: f32, f1: f32, bend: f32, amp: f32, gap: f32) -> Syl {
    Syl { h: [1.0, 0.65, 0.45, 0.28], noise: 0.22, q: 4.0, ..whistle(dur, f0, f1, bend, amp, gap) }
}

const MAX_SYL: usize = 256;

/// Uniform in a..b.
fn rr(r: &mut Rng, a: f32, b: f32) -> f32 {
    a + (b - a) * r.unipolar()
}

/// Spread evenly in pitch between a and b.
fn lr(r: &mut Rng, a: f32, b: f32) -> f32 {
    a * (b / a).powf(r.unipolar())
}

fn ri(r: &mut Rng, a: u32, b: u32) -> u32 {
    a + (r.next_u32() % (b - a + 1))
}

/// Writes a song of the species into `out`, its shape from `r` (a song
/// type's own seed).
fn compose(species: usize, r: &mut Rng, out: &mut Vec<Syl>) {
    out.clear();
    match species {
        // Blackbird: a few low, fluting notes that bend, then a squeaky twitter.
        0 => {
            let base = lr(r, 1300.0, 2100.0);
            for _ in 0..ri(r, 3, 7) {
                let f0 = base * 2f32.powf(rr(r, -0.4, 0.6));
                let f1 = f0 * 2f32.powf(rr(r, -0.5, 0.4));
                let mut s = whistle(rr(r, 0.09, 0.28), f0, f1, rr(r, -0.25, 0.35), rr(r, 0.6, 1.0), rr(r, 0.02, 0.07));
                s.h = [1.0, 0.22, 0.07, 0.02];
                s.fm_rate = rr(r, 8.0, 20.0);
                s.fm_depth = rr(r, 0.0, 0.012);
                push(out, s);
            }
            if r.unipolar() < 0.7 {
                for _ in 0..ri(r, 3, 9) {
                    let f0 = lr(r, 4000.0, 7500.0);
                    let mut s = whistle(rr(r, 0.012, 0.035), f0, f0 * rr(r, 0.5, 0.8), 0.1, rr(r, 0.3, 0.5), rr(r, 0.01, 0.03));
                    s.h = [1.0, 0.3, 0.1, 0.0];
                    s.noise = 0.08;
                    push(out, s);
                }
            }
        }
        // Robin: thin, high, liquid notes in quick changes, some trilled.
        1 => {
            for _ in 0..ri(r, 10, 24) {
                let f0 = lr(r, 2500.0, 8000.0);
                let f1 = f0 * 2f32.powf(rr(r, -0.7, 0.7));
                let long = r.unipolar() < 0.12;
                let mut s = whistle(if long { rr(r, 0.2, 0.35) } else { rr(r, 0.025, 0.11) }, f0, f1, rr(r, -0.3, 0.3), rr(r, 0.4, 0.9), rr(r, 0.005, 0.04));
                s.h = [1.0, 0.06, 0.0, 0.0];
                if r.unipolar() < 0.3 {
                    s.fm_rate = rr(r, 35.0, 70.0);
                    s.fm_depth = rr(r, 0.04, 0.12);
                }
                push(out, s);
            }
        }
        // Song Thrush: a short phrase sung two to four times over.
        2 => {
            let mut phrase = [whistle(0.1, 3000.0, 3000.0, 0.0, 1.0, 0.03); 5];
            let n = ri(r, 2, 5) as usize;
            for s in phrase.iter_mut().take(n) {
                let f0 = lr(r, 1800.0, 5000.0);
                let f1 = f0 * 2f32.powf(rr(r, -0.6, 0.5));
                *s = match ri(r, 0, 2) {
                    0 => whistle(rr(r, 0.06, 0.16), f0, f1, rr(r, -0.2, 0.3), rr(r, 0.7, 1.0), rr(r, 0.02, 0.06)),
                    1 => harsh(rr(r, 0.04, 0.1), f0 * 0.8, f1 * 0.8, 0.2, rr(r, 0.6, 0.9), rr(r, 0.02, 0.05)),
                    _ => Syl { fm_rate: rr(r, 40.0, 80.0), fm_depth: 0.08, ..whistle(rr(r, 0.08, 0.15), f0, f1, 0.0, 0.8, 0.03) },
                };
            }
            for rep in 0..ri(r, 2, 4) {
                for (i, s) in phrase.iter().take(n).enumerate() {
                    let mut s = *s;
                    if i == n - 1 {
                        s.gap = rr(r, 0.12, 0.25);
                    }
                    s.amp *= 1.0 - 0.08 * rep as f32;
                    push(out, s);
                }
            }
        }
        // Nightingale: slow whistles swelling, then a hard, fast trill (or
        // low chooks).
        3 => {
            if r.unipolar() < 0.65 {
                let k = ri(r, 3, 7);
                let f = lr(r, 1400.0, 2600.0);
                for i in 0..k {
                    let mut s = whistle(rr(r, 0.12, 0.22), f, f * rr(r, 0.95, 1.08), 0.05, 0.3 + 0.7 * (i + 1) as f32 / k as f32, rr(r, 0.08, 0.14));
                    s.att = s.dur * 0.4;
                    s.h = [1.0, 0.05, 0.0, 0.0];
                    push(out, s);
                }
            }
            if r.unipolar() < 0.75 {
                let rate = rr(r, 12.0, 25.0);
                let f0 = lr(r, 2000.0, 5000.0);
                let f1 = f0 * rr(r, 0.4, 0.7);
                for _ in 0..(rate * rr(r, 0.5, 1.3)) as u32 {
                    let mut s = whistle(0.6 / rate, f0, f1, 0.0, 0.9, 0.4 / rate);
                    s.h = [1.0, 0.45, 0.25, 0.1];
                    s.noise = 0.12;
                    push(out, s);
                }
            } else {
                let f = lr(r, 900.0, 1300.0);
                for _ in 0..ri(r, 3, 6) {
                    push(out, Syl { noise: 0.3, q: 3.0, ..harsh(0.05, f, f * 0.9, 0.1, 0.8, 0.06) });
                }
            }
            if r.unipolar() < 0.4 {
                let f = lr(r, 3000.0, 5000.0);
                push(out, whistle(rr(r, 0.15, 0.3), f, f * rr(r, 1.1, 1.4), 0.1, 0.8, 0.05));
            }
        }
        // Wren: loud, very fast trills of tiny notes, segment after segment.
        4 => {
            for _ in 0..ri(r, 4, 7) {
                let f0 = lr(r, 3500.0, 8000.0);
                let f1 = f0 * 2f32.powf(rr(r, -0.8, 0.3));
                let (dur, gap, amp) = (rr(r, 0.018, 0.03), rr(r, 0.008, 0.02), rr(r, 0.8, 1.0));
                for _ in 0..ri(r, 6, 16) {
                    let mut s = whistle(dur, f0, f1, 0.0, amp, gap);
                    s.h = [1.0, 0.15, 0.05, 0.0];
                    push(out, s);
                }
            }
        }
        // Chaffinch: a falling trill that speeds up, a second phrase, then the
        // flourish at the end.
        5 => {
            let mut f = lr(r, 5500.0, 7000.0);
            let mut gap = 0.07;
            for _ in 0..ri(r, 5, 9) {
                push(out, whistle(rr(r, 0.03, 0.04), f, f * 0.75, 0.05, 0.8, gap));
                f *= rr(r, 0.94, 0.98);
                gap *= 0.88;
            }
            let f2 = lr(r, 3000.0, 4000.0);
            for _ in 0..ri(r, 3, 6) {
                push(out, whistle(0.035, f2, f2 * 1.15, 0.0, 0.85, 0.035));
            }
            let mut s = whistle(rr(r, 0.15, 0.25), lr(r, 4500.0, 6000.0), lr(r, 1800.0, 2600.0), 0.3, 1.0, 0.05);
            s.h = [1.0, 0.4, 0.2, 0.1];
            s.noise = 0.05;
            push(out, s);
        }
        // Great Tit: "tee-cha tee-cha tee-cha".
        6 => {
            let hi = lr(r, 4500.0, 6500.0);
            let lo = hi * rr(r, 0.6, 0.8);
            let three = r.unipolar() < 0.3;
            for _ in 0..ri(r, 3, 7) {
                push(out, whistle(rr(r, 0.09, 0.12), hi, hi * 0.97, 0.0, 0.9, 0.03));
                if three {
                    push(out, whistle(0.08, hi * 0.98, hi * 0.95, 0.0, 0.8, 0.03));
                }
                push(out, whistle(0.07, lo, lo * 0.85, 0.0, 0.85, rr(r, 0.07, 0.1)));
            }
        }
        // Skylark: a long, rambling warble with hardly a pause.
        7 => {
            let total = rr(r, 6.0, 14.0);
            let mut t = 0.0;
            while t < total && out.len() < MAX_SYL - 12 {
                let f0 = lr(r, 2500.0, 6500.0);
                match ri(r, 0, 3) {
                    0 => {
                        let s = whistle(rr(r, 0.02, 0.06), f0, f0 * 2f32.powf(rr(r, -0.8, 0.8)), rr(r, -0.3, 0.3), rr(r, 0.5, 0.9), rr(r, 0.005, 0.05));
                        t += s.dur + s.gap;
                        out.push(s);
                    }
                    1 => {
                        let rate = rr(r, 25.0, 40.0);
                        for _ in 0..ri(r, 4, 10) {
                            let s = whistle(0.55 / rate, f0, f0 * 0.8, 0.0, 0.7, 0.45 / rate);
                            t += s.dur + s.gap;
                            out.push(s);
                        }
                    }
                    2 => {
                        let s = Syl { fm_rate: rr(r, 15.0, 40.0), fm_depth: 0.05, ..whistle(rr(r, 0.08, 0.2), f0, f0 * rr(r, 0.8, 1.2), 0.0, 0.8, rr(r, 0.01, 0.04)) };
                        t += s.dur + s.gap;
                        out.push(s);
                    }
                    _ => {
                        let s = Syl { h: [1.0, 0.3, 0.15, 0.05], ..whistle(rr(r, 0.1, 0.25), f0 * 0.8, f0 * rr(r, 0.7, 1.1), 0.2, 0.8, rr(r, 0.01, 0.05)) };
                        t += s.dur + s.gap;
                        out.push(s);
                    }
                }
            }
        }
        // Sparrow: chirps, "chilp", here and there.
        8 => {
            for _ in 0..ri(r, 2, 9) {
                let f0 = lr(r, 2800.0, 3600.0);
                push(out, Syl { noise: 0.12, q: 5.0, ..harsh(rr(r, 0.06, 0.11), f0, f0 * rr(r, 0.8, 1.1), rr(r, 0.2, 0.45), rr(r, 0.6, 1.0), rr(r, 0.12, 0.35)) });
            }
        }
        // Cuckoo: "cu-coo", a falling third, over and over.
        9 => {
            let hi = lr(r, 560.0, 720.0);
            let lo = hi * 2f32.powf(-rr(r, 3.0, 4.2) / 12.0);
            let hollow = |s: Syl| Syl { h: [1.0, 0.04, 0.09, 0.01], noise: 0.06, q: 6.0, att: 0.025, rel: 0.04, ..s };
            for _ in 0..ri(r, 4, 10) {
                push(out, hollow(whistle(0.13, hi, hi * 0.97, 0.0, 1.0, 0.09)));
                push(out, hollow(whistle(0.23, lo, lo * 0.96, 0.0, 0.9, rr(r, 0.45, 0.6))));
            }
        }
        // Wood Pigeon: "coo-COO-coo, coo-coo", soft and breathy.
        10 => {
            let f = lr(r, 420.0, 540.0);
            let coo = |dur: f32, amp: f32, gap: f32| Syl { h: [1.0, 0.18, 0.05, 0.0], noise: 0.18, q: 3.0, att: 0.06, rel: 0.1, ..whistle(dur, f * 1.02, f * 0.93, 0.04, amp, gap) };
            for _ in 0..ri(r, 3, 5) {
                push(out, coo(0.28, 0.75, 0.06));
                push(out, coo(0.55, 1.0, 0.06));
                push(out, coo(0.28, 0.75, 0.25));
                push(out, coo(0.3, 0.8, 0.06));
                push(out, coo(0.3, 0.75, 0.35));
            }
            push(out, coo(0.3, 0.8, 0.1));
        }
        // Tawny Owl: a hoot, a pause, then the long wavering hoot; or the
        // female's "ke-wick".
        11 => {
            let f = lr(r, 430.0, 520.0);
            let hoot = |s: Syl| Syl { h: [1.0, 0.2, 0.08, 0.02], noise: 0.2, q: 4.0, ..s };
            if r.unipolar() < 0.7 {
                push(out, hoot(Syl { att: 0.08, rel: 0.15, ..whistle(0.55, f, f * 0.92, 0.05, 1.0, rr(r, 2.5, 4.0)) }));
                push(out, hoot(Syl { att: 0.02, rel: 0.05, ..whistle(0.12, f * 1.02, f * 0.98, 0.0, 0.6, 0.15) }));
                let rate = rr(r, 8.0, 12.0);
                push(out, hoot(Syl { am_rate: rate, am_depth: 0.7, fm_rate: rate, fm_depth: 0.02, att: 0.05, rel: 0.3, ..whistle(1.3, f * 1.05, f * 0.9, 0.0, 1.0, 0.1) }));
            } else {
                push(out, Syl { noise: 0.35, q: 3.0, ..harsh(0.3, 1500.0, 2200.0, 0.2, 0.8, 0.1) });
            }
        }
        // Loon: the long, haunting wail, or its tremolo.
        12 => {
            let f = lr(r, 600.0, 700.0);
            let wail = |dur: f32, f0: f32, f1: f32| Syl { h: [1.0, 0.35, 0.15, 0.05], fm_rate: 5.0, fm_depth: 0.006, noise: 0.04, att: 0.04, rel: 0.05, ..whistle(dur, f0, f1, 0.0, 1.0, 0.0) };
            if r.unipolar() < 0.6 {
                let up = rr(r, 1.45, 1.6);
                push(out, wail(rr(r, 0.7, 1.0), f, f * 1.04));
                push(out, wail(rr(r, 1.1, 1.6), f * up, f * up * 0.98));
                push(out, Syl { rel: 0.4, ..wail(rr(r, 0.7, 1.0), f * 1.25, f * 1.12) });
            } else {
                let rate = rr(r, 7.0, 10.0);
                let g = f * rr(r, 1.4, 1.7);
                push(out, Syl { am_rate: rate, am_depth: 0.85, fm_rate: rate, fm_depth: 0.03, rel: 0.2, ..wail(rr(r, 1.2, 2.0), g, g * 0.97) });
            }
        }
        // Gull: the long call, "kyow kyow", then laughing.
        13 => {
            let f = lr(r, 900.0, 1300.0);
            for i in 0..ri(r, 3, 7) {
                let low = if i < 2 { 0.85 } else { 1.0 };
                push(out, Syl { noise: 0.25, q: 3.0, h: [1.0, 0.75, 0.55, 0.4], ..harsh(rr(r, 0.14, 0.25), f * low, f * low * 0.75, 0.4, 0.9, rr(r, 0.06, 0.12)) });
            }
            let mut g = f * 1.1;
            for _ in 0..ri(r, 5, 11) {
                push(out, Syl { noise: 0.25, q: 3.0, ..harsh(0.07, g, g * 0.85, 0.15, 0.8, 0.09) });
                g *= 0.985;
            }
        }
        // Woodpecker: drumming on a dead branch, a burst of knocks.
        14 => {
            let f = lr(r, 700.0, 1400.0);
            let mut rate = rr(r, 16.0, 22.0);
            let n = ri(r, 10, 24);
            for i in 0..n {
                let amp = 1.0 - 0.4 * i as f32 / n as f32;
                push(out, Syl { h: [0.3, 0.0, 0.0, 0.0], noise: 1.0, q: 12.0, att: 0.0005, rel: 0.011, ..whistle(0.012, f, f * 0.98, 0.0, amp, 1.0 / rate - 0.012) });
                rate *= 0.99;
            }
        }
        // Blackcap: a quiet, scratchy warble that breaks into a loud, clear
        // fluting at the end.
        15 => {
            for _ in 0..ri(r, 8, 15) {
                let f0 = lr(r, 2500.0, 5000.0);
                let s = if r.unipolar() < 0.4 { harsh(rr(r, 0.02, 0.05), f0 * 0.8, f0 * 0.7, 0.1, rr(r, 0.25, 0.4), rr(r, 0.01, 0.03)) } else { whistle(rr(r, 0.02, 0.06), f0, f0 * 2f32.powf(rr(r, -0.5, 0.5)), 0.1, rr(r, 0.3, 0.5), rr(r, 0.01, 0.03)) };
                push(out, s);
            }
            for _ in 0..ri(r, 4, 7) {
                let f0 = lr(r, 1800.0, 3500.0);
                push(out, Syl { h: [1.0, 0.2, 0.05, 0.0], ..whistle(rr(r, 0.08, 0.15), f0, f0 * 2f32.powf(rr(r, -0.4, 0.4)), rr(r, -0.2, 0.3), rr(r, 0.85, 1.0), rr(r, 0.015, 0.04)) });
            }
        }
        // Chiffchaff: "chiff chaff chiff chiff chaff", two notes in no order.
        16 => {
            let hi = lr(r, 5400.0, 6200.0);
            let lo = hi * rr(r, 0.75, 0.82);
            for _ in 0..ri(r, 6, 14) {
                let f = if r.unipolar() < 0.5 { hi } else { lo };
                push(out, whistle(rr(r, 0.06, 0.08), f, f * 0.85, 0.05, rr(r, 0.75, 1.0), rr(r, 0.25, 0.35)));
            }
        }
        // Willow Warbler: a soft cascade going down the scale.
        17 => {
            let n = ri(r, 10, 16);
            let mut f = lr(r, 5000.0, 6000.0);
            let step = (3000.0 / f).powf(1.0 / n as f32);
            for i in 0..n {
                let amp = 0.45 + 0.55 * (PI * (i as f32 + 0.5) / n as f32).sin();
                push(out, whistle(rr(r, 0.05, 0.08), f, f * 0.9, 0.08, amp, rr(r, 0.02, 0.04)));
                f *= step * rr(r, 0.98, 1.02);
            }
        }
        // Yellowhammer: "a little bit of bread and no cheese".
        18 => {
            let f = lr(r, 5500.0, 6500.0);
            for _ in 0..ri(r, 6, 10) {
                push(out, Syl { h: [1.0, 0.3, 0.1, 0.0], ..whistle(0.04, f, f * 0.92, 0.0, 0.8, 0.06) });
            }
            if r.unipolar() < 0.5 {
                push(out, whistle(0.12, f * 1.05, f, 0.0, 0.9, 0.02));
            }
            let g = lr(r, 3700.0, 4300.0);
            push(out, Syl { h: [1.0, 0.25, 0.08, 0.0], ..whistle(rr(r, 0.4, 0.6), g, g * 0.92, 0.0, 1.0, 0.05) });
        }
        // Collared Dove: "coo-COO-cuk", soft and breathy.
        19 => {
            let f = lr(r, 450.0, 520.0);
            let coo = |dur: f32, k: f32, amp: f32, gap: f32| Syl { h: [1.0, 0.15, 0.05, 0.0], noise: 0.18, q: 3.0, att: 0.04, rel: 0.08, ..whistle(dur, f * k, f * k * 0.96, 0.03, amp, gap) };
            for _ in 0..ri(r, 3, 8) {
                push(out, coo(0.15, 1.0, 0.7, 0.05));
                push(out, coo(0.45, 1.05, 1.0, 0.05));
                push(out, coo(0.2, 0.9, 0.75, rr(r, 0.35, 0.5)));
            }
        }
        // Golden Oriole: a liquid, fluting "weela-weeo".
        20 => {
            for _ in 0..ri(r, 3, 5) {
                let f0 = lr(r, 1200.0, 2200.0);
                push(out, Syl { h: [1.0, 0.3, 0.1, 0.0], ..whistle(rr(r, 0.1, 0.2), f0, f0 * 2f32.powf(rr(r, -0.5, 0.5)), rr(r, -0.4, 0.4), rr(r, 0.8, 1.0), rr(r, 0.01, 0.03)) });
            }
        }
        // Crow: "kraa, kraa, kraa", harsh and hoarse.
        21 => {
            let f = lr(r, 600.0, 900.0);
            for _ in 0..ri(r, 2, 5) {
                push(out, Syl { h: [1.0, 0.8, 0.7, 0.6], noise: 0.6, q: 2.0, att: 0.02, rel: 0.06, ..harsh(rr(r, 0.25, 0.4), f, f * 0.85, 0.1, 1.0, rr(r, 0.3, 0.5)) });
            }
        }
        // Curlew: slow rising calls, then the bubbling trill.
        22 => {
            let f = lr(r, 1700.0, 1900.0);
            for _ in 0..ri(r, 2, 3) {
                push(out, Syl { h: [1.0, 0.15, 0.04, 0.0], att: 0.05, ..whistle(0.3, f, f * 1.45, -0.05, 0.85, rr(r, 0.25, 0.4)) });
            }
            let n = ri(r, 10, 20);
            for i in 0..n {
                let k = i as f32 / n as f32;
                let g = f * (1.25 + 0.4 * k);
                push(out, Syl { fm_rate: 20.0, fm_depth: 0.03, h: [1.0, 0.15, 0.04, 0.0], ..whistle(0.08, g, g * 1.08, 0.0, 0.7 + 0.3 * k, 0.015) });
            }
            push(out, whistle(0.35, f * 1.6, f * 1.1, 0.05, 0.8, 0.05));
        }
        // Swift: the screaming of a party racing past.
        23 => {
            for _ in 0..ri(r, 1, 3) {
                let f0 = lr(r, 6000.0, 8000.0);
                push(out, Syl { fm_rate: rr(r, 80.0, 120.0), fm_depth: 0.06, noise: 0.25, q: 4.0, h: [1.0, 0.4, 0.15, 0.0], ..whistle(rr(r, 0.4, 0.8), f0, f0 * rr(r, 0.85, 1.05), 0.05, 0.9, rr(r, 0.1, 0.3)) });
            }
        }
        // Hermit Thrush: a long pure note, then an ethereal spiral of fluting
        // notes above it; each song type at its own pitch.
        24 => {
            let f = lr(r, 2000.0, 3500.0);
            push(out, Syl { fm_rate: 6.0, fm_depth: 0.004, att: 0.04, ..whistle(rr(r, 0.25, 0.4), f, f * 0.99, 0.0, 0.9, 0.02) });
            let chord = [1.5f32, 1.68, 2.0, 2.25, 2.52];
            for _ in 0..ri(r, 5, 10) {
                let g = f * chord[ri(r, 0, 4) as usize];
                push(out, Syl { fm_rate: 30.0, fm_depth: 0.01, ..whistle(rr(r, 0.04, 0.06), g, g * rr(r, 0.97, 1.03), 0.0, rr(r, 0.6, 0.85), 0.008) });
            }
        }
        // Wood Thrush: "bup bup, ee-oh-LAY", the last a buzzing trill.
        25 => {
            for _ in 0..2 {
                push(out, Syl { h: [1.0, 0.2, 0.0, 0.0], ..whistle(0.05, 1200.0, 1150.0, 0.0, 0.4, 0.06) });
            }
            let f = lr(r, 1700.0, 2000.0);
            push(out, Syl { h: [1.0, 0.5, 0.1, 0.0], ..whistle(0.15, f, f * 1.05, 0.05, 0.85, 0.02) });
            push(out, Syl { h: [1.0, 0.5, 0.1, 0.0], ..whistle(0.15, f * 1.22, f * 1.18, 0.05, 0.9, 0.03) });
            let g = lr(r, 4000.0, 6000.0);
            push(out, Syl { fm_rate: 50.0, fm_depth: 0.1, h: [1.0, 0.6, 0.2, 0.05], noise: 0.08, ..whistle(0.25, g, g * 0.9, 0.0, 0.8, 0.05) });
        }
        // Cardinal: "cheer cheer cheer" down-slurs, or "birdy birdy" up-slurs.
        26 => {
            if r.unipolar() < 0.6 {
                for _ in 0..ri(r, 4, 10) {
                    push(out, whistle(rr(r, 0.15, 0.25), lr(r, 3600.0, 4200.0), lr(r, 1900.0, 2300.0), 0.05, 1.0, rr(r, 0.05, 0.1)));
                }
            } else {
                for _ in 0..ri(r, 4, 8) {
                    push(out, whistle(0.08, lr(r, 2300.0, 2700.0), lr(r, 4200.0, 4800.0), 0.0, 0.95, 0.05));
                }
            }
        }
        // Mourning Dove: "coo-OOO-oo-oo-oo", soft and sad.
        27 => {
            let f = lr(r, 430.0, 480.0);
            let coo = |dur: f32, f0: f32, f1: f32, amp: f32, gap: f32| Syl { h: [1.0, 0.12, 0.03, 0.0], noise: 0.2, q: 3.0, att: 0.06, rel: 0.12, fm_rate: 5.0, fm_depth: 0.005, ..whistle(dur, f0, f1, 0.02, amp, gap) };
            push(out, coo(0.3, f, f * 1.15, 0.7, 0.04));
            push(out, coo(0.6, f * 1.15, f * 1.06, 1.0, 0.15));
            for _ in 0..3 {
                push(out, coo(0.4, f, f * 0.97, 0.8, 0.12));
            }
        }
        // Bellbird: pure, ringing notes like small bells, on a few pitches of
        // its own scale.
        28 => {
            let f = lr(r, 1500.0, 2500.0);
            let scale = [1.0f32, 1.125, 1.25, 1.5, 1.68, 2.0];
            for _ in 0..ri(r, 3, 8) {
                let g = f * scale[ri(r, 0, 5) as usize];
                let dur = rr(r, 0.15, 0.4);
                push(out, Syl { h: [1.0, 0.0, 0.15, 0.0], att: 0.003, rel: dur * 0.8, fm_rate: 4.0, fm_depth: 0.002, ..whistle(dur, g, g, 0.0, rr(r, 0.7, 1.0), rr(r, 0.05, 0.25)) });
            }
        }
        // Kookaburra: low chuckles rising into loud laughter, then fading.
        _ => {
            let f = lr(r, 800.0, 1200.0);
            for _ in 0..ri(r, 5, 8) {
                push(out, Syl { noise: 0.3, q: 3.0, ..harsh(0.06, f, f * 0.9, 0.1, 0.5, 0.06) });
            }
            let rate = rr(r, 8.0, 12.0);
            let n = (rate * rr(r, 1.5, 3.0)) as u32;
            for i in 0..n {
                let k = (PI * i as f32 / n as f32).sin();
                let g = f * (1.3 + 1.2 * k);
                push(out, Syl { noise: 0.3, q: 3.0, ..harsh(0.6 / rate, g * 0.8, g * 1.2, 0.3, 0.5 + 0.5 * k, 0.4 / rate) });
            }
        }
    }
}

fn push(out: &mut Vec<Syl>, s: Syl) {
    if out.len() < MAX_SYL {
        out.push(s);
    }
}

struct Bird {
    on: bool,
    species: usize,
    /// Its place: left..right (-1..1) and nearer or further than Distance.
    u: f32,
    v: f32,
    /// Flying to a new place: where from, and how far along.
    from: (f32, f32),
    fly: f32,
    songs: [u32; 4],
    kind: usize,
    repeats: u32,
    song: Vec<Syl>,
    idx: usize,
    t: f32,
    rest: f32,
    phase: f32,
    fm_ph: f32,
    am_ph: f32,
    band: Svf,
    band2: Svf,
    band_norm: f32,
    air: OnePole,
    /// This song's transposition and loudness.
    ratio: f32,
    amp: f32,
    /// Its own voice: a little higher or lower, a little quicker or slower
    /// than others of its kind.
    voice: f32,
    pace: f32,
    /// How far it is now (0 near .. 1 far), for the screen.
    dist: f32,
    level: f32,
    freq: f32,
    rng: Rng,
}

impl Bird {
    fn new(seed: u32) -> Self {
        let mut rng = Rng::new(seed.wrapping_mul(2_654_435_761).max(1));
        let songs = [rng.next_u32(), rng.next_u32(), rng.next_u32(), rng.next_u32()];
        let voice = 2f32.powf(rng.bipolar() * 1.5 / 12.0);
        let pace = 0.9 + 0.2 * rng.unipolar();
        Self {
            on: false,
            species: usize::MAX,
            u: rng.bipolar(),
            v: rng.bipolar(),
            from: (0.0, 0.0),
            fly: 1.0,
            songs,
            kind: 0,
            repeats: 0,
            song: Vec::with_capacity(MAX_SYL),
            idx: 0,
            t: 0.0,
            rest: 0.0,
            phase: 0.0,
            fm_ph: 0.0,
            am_ph: 0.0,
            band: Svf::new(),
            band2: Svf::new(),
            band_norm: 1.0,
            air: OnePole::default(),
            ratio: 1.0,
            amp: 1.0,
            voice: 1.0,
            pace: 1.0,
            dist: 0.5,
            level: 0.0,
            freq: 0.0,
            rng,
        }
        .with_voice(voice, pace)
    }

    fn with_voice(mut self, voice: f32, pace: f32) -> Self {
        self.voice = voice;
        self.pace = pace;
        self
    }

    fn singing(&self) -> bool {
        self.idx < self.song.len()
    }

    /// Where it is now (flying or sitting).
    fn place(&self) -> (f32, f32) {
        let k = self.fly.clamp(0.0, 1.0);
        let k = k * k * (3.0 - 2.0 * k);
        (self.from.0 + (self.u - self.from.0) * k, self.from.1 + (self.v - self.from.1) * k)
    }

    /// Starts its next song; `variety` sets how much it changes.
    fn start(&mut self, variety: f32, tempo: f32, ratio: f32, amp: f32) {
        let kind = &KINDS[self.species.min(KINDS.len() - 1)];
        if self.repeats == 0 || self.rng.unipolar() < variety * 0.25 {
            self.kind = (self.kind + 1 + (self.rng.next_u32() % 3) as usize) % 4;
            self.repeats = ri(&mut self.rng, kind.repeat.0, kind.repeat.1);
            // Very varied singers learn new songs now and then.
            if self.rng.unipolar() < variety * 0.3 {
                self.songs[self.kind] = self.rng.next_u32();
            }
        }
        self.repeats = self.repeats.saturating_sub(1);
        let mut r = Rng::new(self.songs[self.kind]);
        compose(self.species, &mut r, &mut self.song);
        // Each rendition a little different.
        let j = 0.04 * variety;
        let k = self.pace / tempo.clamp(0.25, 4.0);
        for s in self.song.iter_mut() {
            let d = 1.0 + j * self.rng.bipolar();
            s.f0 *= d * self.voice;
            s.f1 *= (1.0 + j * self.rng.bipolar()) * self.voice;
            s.dur *= k * (1.0 + 2.0 * j * self.rng.bipolar());
            s.gap *= k * (1.0 + 3.0 * j * self.rng.bipolar());
            s.gap = s.gap.max(0.0);
            s.att = s.att.min(s.dur * 0.5);
            s.rel = s.rel.min(s.dur * 0.6);
        }
        self.ratio = ratio;
        self.amp = amp;
        self.idx = 0;
        self.t = 0.0;
        self.fm_ph = 0.0;
        self.am_ph = 0.0;
    }

    /// The pitch the song sits around (Hz), weighted by how long each note is.
    fn center(&self) -> f32 {
        let (mut sum, mut w) = (0.0f32, 0.0f32);
        for s in &self.song {
            sum += 0.5 * (s.f0.ln() + s.f1.ln()) * s.dur;
            w += s.dur;
        }
        if w > 0.0 {
            (sum / w).exp()
        } else {
            KINDS[self.species.min(KINDS.len() - 1)].home
        }
    }

    /// A rest before the next song.
    fn rest_for(&mut self, busy: f32) {
        let kind = &KINDS[self.species.min(KINDS.len() - 1)];
        let f = 2.5 * (0.35f32 / 2.5).powf(busy.clamp(0.0, 1.0));
        self.rest = rr(&mut self.rng, kind.rest.0, kind.rest.1) * f;
    }
}

pub struct BirdsState {
    /// The song plays (set by the engine every block).
    pub playing: bool,
    /// They were singing in the last block (a fresh start staggers them).
    sang: bool,
    sr: f32,
    birds: Vec<Bird>,
    held: usize,
    noise: Rng,
    hall: Box<Hall>,
    hall_p: Vec<f32>,
    dl: Vec<f32>,
    dr: Vec<f32>,
    wl: Vec<f32>,
    wr: Vec<f32>,
}

const CHUNK: usize = 64;

impl BirdsState {
    pub fn new(sr: f32) -> Self {
        Self {
            playing: false,
            sang: false,
            sr: sr.max(1000.0),
            birds: (0..MAX_BIRDS).map(|i| Bird::new(0x5EED_B12D ^ (i as u32 + 1).wrapping_mul(0x9E37_79B9))).collect(),
            held: 0,
            noise: Rng::new(0x0B1D_5EED),
            hall: Box::new(Hall::new(sr)),
            hall_p: crate::params::defaults(hall::SPECS),
            dl: vec![0.0; CHUNK],
            dr: vec![0.0; CHUNK],
            wl: vec![0.0; CHUNK],
            wr: vec![0.0; CHUNK],
        }
    }

    /// The birds there are for these params: changing a kind gives new birds
    /// of that kind, the others stay.
    fn arrange(&mut self, p: &[f32]) {
        for slot in 0..SLOTS {
            let species = (p[BIRD_1 + 2 * slot].round().max(0.0) as usize).min(SPECIES.len() - 1);
            let count = (p[NUMBER_1 + 2 * slot].round().max(0.0) as usize).min(PER_SLOT);
            for j in 0..PER_SLOT {
                let b = &mut self.birds[slot * PER_SLOT + j];
                if b.species != species {
                    b.species = species;
                    b.song.clear();
                    b.idx = 0;
                    b.repeats = 0;
                    // Not all at once: each starts after a while.
                    let k = &KINDS[species];
                    b.rest = rr(&mut b.rng, 0.2, 0.3 + 0.6 * k.rest.1);
                }
                let on = j < count;
                if on && !b.on {
                    b.rest = b.rest.min(rr(&mut b.rng, 0.2, 2.0));
                }
                b.on = on;
            }
        }
    }

    pub fn note_on(&mut self, p: &[f32], note: u8, vel: f32) {
        if p.len() < SPECS.len() {
            return;
        }
        self.held += 1;
        self.arrange(p);
        let want = midi_to_freq(note as f32);
        let pitch = 2f32.powf(p[PITCH].clamp(-24.0, 24.0) / 12.0);
        // The bird whose voice is nearest the key; one that is quiet first.
        let pick = (0..MAX_BIRDS).filter(|&i| self.birds[i].on).min_by(|&a, &b| {
            let d = |i: usize| (KINDS[self.birds[i].species].home * pitch / want).ln().abs() + if self.birds[i].singing() { 0.3 } else { 0.0 };
            d(a).total_cmp(&d(b))
        });
        let Some(i) = pick else { return };
        let (variety, tempo, follow) = (p[VARIETY].clamp(0.0, 1.0), p[TEMPO], p[FOLLOW].clamp(0.0, 1.0));
        let b = &mut self.birds[i];
        b.start(variety, tempo, 1.0, 0.35 + 0.65 * vel.clamp(0.0, 1.0));
        let to_key = want / b.center();
        b.ratio = pitch.powf(1.0 - follow) * to_key.powf(follow);
        b.fly = 1.0;
    }

    pub fn note_off(&mut self, _p: &[f32], _note: u8) {
        self.held = self.held.saturating_sub(1);
    }

    pub fn all_off(&mut self) {
        self.held = 0;
    }

    /// The screen: [0] on, [1] how many places, then per bird (4 + 5i): its
    /// place across (-1..1) and away (0..1), how loud it sings now, the
    /// pitch (Hz / 10000) and the species (-1: no bird).
    pub fn visual(&self, out: &mut [f32; crate::engine::INST_VALS]) {
        out[0] = 1.0;
        out[1] = MAX_BIRDS as f32;
        for (i, b) in self.birds.iter().enumerate() {
            let o = 4 + 5 * i;
            if o + 4 >= out.len() {
                break;
            }
            let (x, _) = b.place();
            out[o] = x;
            out[o + 1] = b.dist;
            out[o + 2] = b.level;
            out[o + 3] = b.freq / 10000.0;
            out[o + 4] = if b.on { b.species as f32 } else { -1.0 };
        }
    }

    pub fn render(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32]) {
        if p.len() < SPECS.len() {
            return;
        }
        self.arrange(p);
        let sings = match (p[SINGS].round().max(0.0) as usize).min(SING_MODES.len() - 1) {
            0 => self.playing,
            1 => true,
            _ => self.held > 0,
        };
        let n = l.len().min(r.len());
        let sr = self.sr;
        let (busy, variety, tempo) = (p[BUSY].clamp(0.0, 1.0), p[VARIETY].clamp(0.0, 1.0), p[TEMPO]);
        let pitch = 2f32.powf(p[PITCH].clamp(-24.0, 24.0) / 12.0);
        let (distance, width, space) = (p[DISTANCE].clamp(0.0, 1.0), p[WIDTH].clamp(0.0, 1.0), p[SPACE].clamp(0.0, 1.0));
        let gain = db_to_gain(p[VOLUME]) * 0.32;
        // Starting to sing: not all at once, each after a while of its own.
        if sings && !self.sang {
            let f = 2.5 * (0.35f32 / 2.5).powf(busy);
            for b in self.birds.iter_mut().filter(|b| b.on && !b.singing()) {
                let k = &KINDS[b.species.min(KINDS.len() - 1)];
                b.rest = rr(&mut b.rng, 0.1, 0.8) * 0.5 * (k.rest.0 + k.rest.1) * f;
            }
            // One of them begins soon.
            if let Some(b) = self.birds.iter_mut().find(|b| b.on && !b.singing()) {
                b.rest = b.rest.min(0.3);
            }
        }
        self.sang = sings;
        let mut done = 0;
        while done < n {
            let len = CHUNK.min(n - done);
            let dt = len as f32 / sr;
            // Birds take turns: one about to sing waits while others sing,
            // more patiently the less busy they are.
            let mut singing_now = self.birds.iter().filter(|b| b.on && b.singing() && b.dist < 0.85).count();
            let patience = 0.04 + 0.65 * busy * busy;
            self.dl[..len].fill(0.0);
            self.dr[..len].fill(0.0);
            self.wl[..len].fill(0.0);
            self.wr[..len].fill(0.0);
            for b in self.birds.iter_mut() {
                if b.fly < 1.0 {
                    b.fly += dt / 1.4;
                }
                if !b.singing() {
                    b.level *= 0.9;
                    b.freq = 0.0;
                    if !b.on {
                        continue;
                    }
                    b.dist = (distance + 0.3 * b.place().1 + KINDS[b.species].far).clamp(0.0, 1.0);
                    b.rest -= dt;
                    if b.rest <= 0.0 {
                        if sings && b.fly >= 1.0 {
                            // Now and then it flies off to sing elsewhere.
                            if singing_now > 0 && b.rng.unipolar() > patience.powi(singing_now as i32) {
                                // Someone else is singing: wait for a gap.
                                b.rest = rr(&mut b.rng, 0.3, 1.5);
                            } else if b.song.is_empty() || b.rng.unipolar() > 0.08 * (1.0 + variety) {
                                b.start(variety, tempo, pitch, 1.0);
                                b.rest_for(busy);
                                singing_now += 1;
                            } else {
                                b.from = b.place();
                                b.u = b.rng.bipolar();
                                b.v = b.rng.bipolar();
                                b.fly = 0.0;
                                b.rest = 1.5;
                            }
                        } else {
                            b.rest = 0.25;
                        }
                    }
                    if !b.singing() {
                        continue;
                    }
                }
                // Its place: how loud, how dull, how much room.
                let (x, v) = b.place();
                let d = (distance + 0.3 * v + KINDS[b.species].far).clamp(0.0, 1.0);
                b.dist = d;
                let g = b.amp / (1.0 + 5.0 * d * d);
                b.air.set(1500.0 + 14000.0 * (1.0 - d).powf(1.5), sr);
                let (pl, pr) = pan_gains(x * width);
                let send = g * (0.2 + 0.8 * d) * space * 1.6;
                let mut peak = 0.0f32;
                for j in 0..len {
                    let Some(&s) = b.song.get(b.idx) else { break };
                    let mut y = 0.0;
                    if b.t < s.dur {
                        let u = b.t / s.dur;
                        let lf = s.f0.ln() + (s.f1.ln() - s.f0.ln()) * u + s.bend * LN_2 * (PI * u).sin();
                        let mut f = lf.exp() * b.ratio;
                        if s.fm_depth > 0.0 {
                            b.fm_ph = (b.fm_ph + s.fm_rate / sr).fract();
                            f *= 1.0 + s.fm_depth * (TAU * b.fm_ph).sin();
                        }
                        let f = f.clamp(40.0, 0.45 * sr);
                        b.freq = f;
                        b.phase = (b.phase + f / sr).fract();
                        let (s1, c1) = (TAU * b.phase).sin_cos();
                        let s2 = 2.0 * s1 * c1;
                        let c2 = 1.0 - 2.0 * s1 * s1;
                        let s3 = s2 * c1 + c2 * s1;
                        let s4 = 2.0 * s2 * c2;
                        let lim = 0.45 * sr;
                        let mut tone = s.h[0] * s1;
                        if 2.0 * f < lim {
                            tone += s.h[1] * s2;
                        }
                        if 3.0 * f < lim {
                            tone += s.h[2] * s3;
                        }
                        if 4.0 * f < lim {
                            tone += s.h[3] * s4;
                        }
                        if s.noise > 0.0 {
                            if j % 16 == 0 {
                                let q = s.q.max(0.5);
                                b.band.set_q(f, q, sr);
                                b.band2.set_q(f, q, sr);
                                b.band_norm = (sr / (PI * f / q)).sqrt() * 0.56 / (q * q);
                            }
                            // Two bands in a row: the breath stays near the pitch.
                            let nb = b.band.tick(self.noise.bipolar()).band;
                            tone += s.noise * b.band2.tick(nb).band * b.band_norm;
                        }
                        let env = (b.t / s.att.max(1e-4)).min(1.0) * ((s.dur - b.t) / s.rel.max(1e-4)).min(1.0);
                        let env = env * env * (3.0 - 2.0 * env);
                        let am = if s.am_depth > 0.0 {
                            b.am_ph = (b.am_ph + s.am_rate / sr).fract();
                            1.0 - s.am_depth * (0.5 - 0.5 * (TAU * b.am_ph).cos())
                        } else {
                            1.0
                        };
                        y = s.amp * env * am * tone;
                    } else {
                        b.freq = 0.0;
                    }
                    b.t += 1.0 / sr;
                    if b.t >= s.dur + s.gap {
                        b.idx += 1;
                        b.t = 0.0;
                    }
                    let y = b.air.lp(y) * g;
                    peak = peak.max(y.abs());
                    self.dl[j] += y * pl;
                    self.dr[j] += y * pr;
                    if send > 0.0 {
                        let w = y * send / g.max(1e-6);
                        self.wl[j] += w * pl;
                        self.wr[j] += w * pr;
                    }
                }
                b.level = b.level.max(peak * 1.5).min(1.0) * 0.97;
            }
            if space > 0.001 {
                let hp = &mut self.hall_p;
                hp[hall::PREDELAY] = 25.0;
                hp[hall::SIZE] = 0.55 + 0.4 * space;
                hp[hall::DECAY] = 1.8 + 4.0 * space;
                hp[hall::LOW_DECAY] = 0.8;
                hp[hall::DAMPING] = 6500.0;
                hp[hall::DIFFUSION] = 0.75;
                hp[hall::MODULATION] = 0.2;
                hp[hall::FREEZE] = 0.0;
                hp[hall::WIDTH] = 1.0;
                hp[hall::MIX] = 1.0;
                let ctx = FxCtx { sr, bpm: 120.0, beat: 0.0, playing: false };
                self.hall.process(&self.hall_p, &mut self.wl[..len], &mut self.wr[..len], &ctx, None);
            }
            for j in 0..len {
                l[done + j] += ((self.dl[j] + self.wl[j]) * gain).clamp(-1.5, 1.5);
                r[done + j] += ((self.dr[j] + self.wr[j]) * gain).clamp(-1.5, 1.5);
            }
            done += len;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 48000.0;

    fn run(st: &mut BirdsState, p: &[f32], secs: f32) -> (Vec<f32>, Vec<f32>) {
        let n = (secs * SR) as usize;
        let (mut l, mut r) = (vec![0.0; n], vec![0.0; n]);
        for (a, b) in l.chunks_mut(256).zip(r.chunks_mut(256)) {
            st.render(p, a, b);
        }
        (l, r)
    }

    fn mag(x: &[f32], hz: f32) -> f32 {
        let w = TAU as f64 * hz as f64 / SR as f64;
        let n = x.len() as f64;
        let (mut re, mut im) = (0.0f64, 0.0f64);
        for (i, &s) in x.iter().enumerate() {
            re += s as f64 * (w * i as f64).cos();
            im += s as f64 * (w * i as f64).sin();
        }
        ((re * re + im * im).sqrt() / n) as f32
    }

    /// Energy well below and well above a frequency (an octave either
    /// side, by four-pole filters).
    fn split(x: &[f32], hz: f32) -> (f32, f32) {
        let mut lp = [OnePole::default(); 4];
        let mut hp = [OnePole::default(); 4];
        for f in lp.iter_mut() {
            f.set(hz * 0.7, SR);
        }
        for f in hp.iter_mut() {
            f.set(hz * 1.4, SR);
        }
        let (mut lo, mut hi) = (0.0, 0.0);
        for &v in x {
            let a = lp.iter_mut().fold(v, |s, f| f.lp(s));
            let b = hp.iter_mut().fold(v, |s, f| f.hp(s));
            lo += a * a;
            hi += b * b;
        }
        (lo, hi)
    }

    #[test]
    fn every_species_composes_songs_that_fit() {
        for sp in 0..SPECIES.len() {
            for seed in 1..40u32 {
                let mut out = Vec::with_capacity(MAX_SYL);
                compose(sp, &mut Rng::new(seed * 7919), &mut out);
                assert!(!out.is_empty() && out.len() <= MAX_SYL, "{}", SPECIES[sp]);
                for s in &out {
                    assert!(s.dur > 0.0 && s.gap >= 0.0 && s.f0 > 50.0 && s.f1 > 50.0 && s.f0 < 12000.0, "{} {s:?}", SPECIES[sp]);
                }
            }
        }
    }

    #[test]
    fn every_preset_sings_and_stays_bounded() {
        for name in preset_names() {
            let mut p = preset(name);
            p[SINGS] = 1.0;
            p[BUSY] = 1.0;
            let mut st = BirdsState::new(SR);
            let (l, r) = run(&mut st, &p, 25.0);
            let peak = l.iter().chain(&r).fold(0.0f32, |m, v| m.max(v.abs()));
            assert!(l.iter().chain(&r).all(|v| v.is_finite()), "{name}");
            assert!(peak > 0.01 && peak < 1.2, "{name}: {peak}");
            if std::env::var("BIRD_REPORT").is_ok() {
                let rms = (l.iter().map(|v| v * v).sum::<f32>() / l.len() as f32).sqrt();
                eprintln!("{name}: peak {peak:.3} rms {rms:.4}");
            }
        }
    }

    #[test]
    fn the_cuckoo_answers_a_low_key_the_robin_a_high_one() {
        let mut p = preset("Garden Morning");
        p[SINGS] = 2.0;
        p[BIRD_1] = 9.0; // cuckoo
        p[NUMBER_1] = 1.0;
        p[BIRD_2] = 1.0; // robin
        p[NUMBER_2] = 1.0;
        p[NUMBER_3] = 0.0;
        p[SPACE] = 0.0;
        let mut st = BirdsState::new(SR);
        // A short press: only the bird it calls sings.
        st.note_on(&p, 76, 0.9);
        st.note_off(&p, 76);
        let (l, _) = run(&mut st, &p, 1.5);
        let (lo, hi) = split(&l, 1500.0);
        assert!(lo > 5.0 * hi, "low {lo} high {hi}");
        let mut st = BirdsState::new(SR);
        // A short press: only the bird it calls sings.
        st.note_on(&p, 100, 0.9);
        st.note_off(&p, 100);
        let (l, _) = run(&mut st, &p, 1.5);
        let (lo, hi) = split(&l, 1500.0);
        assert!(hi > 5.0 * lo, "low {lo} high {hi}");
    }

    #[test]
    fn follow_key_puts_the_song_on_the_key() {
        let mut p = preset("Garden Morning");
        p[SINGS] = 2.0;
        p[BIRD_1] = 10.0; // wood pigeon: one pitch, many coos
        p[NUMBER_1] = 1.0;
        p[NUMBER_2] = 0.0;
        p[NUMBER_3] = 0.0;
        p[SPACE] = 0.0;
        p[FOLLOW] = 1.0;
        let mut st = BirdsState::new(SR);
        st.note_on(&p, 64, 0.9); // E4, 330 Hz
        let (l, _) = run(&mut st, &p, 2.0);
        let f = midi_to_freq(64.0);
        assert!(mag(&l, f) > 3.0 * mag(&l, f * 1.3), "{} {}", mag(&l, f), mag(&l, f * 1.3));
    }

    #[test]
    fn they_take_turns_and_do_not_all_start_at_once() {
        let mut p = preset("Woodland Edge");
        p[SINGS] = 0.0;
        p[BUSY] = 0.25;
        for slot in 0..SLOTS {
            p[NUMBER_1 + 2 * slot] = 2.0;
        }
        let mut st = BirdsState::new(SR);
        st.playing = true;
        let (mut l, mut r) = (vec![0.0f32; 256], vec![0.0f32; 256]);
        let mut first = [f32::NAN; MAX_BIRDS];
        let (mut some, mut more) = (0, 0);
        let blocks = (40.0 * SR / 256.0) as usize;
        for k in 0..blocks {
            st.render(&p, &mut l, &mut r);
            let now = k as f32 * 256.0 / SR;
            let singing: Vec<usize> = (0..MAX_BIRDS).filter(|&i| st.birds[i].on && st.birds[i].singing()).collect();
            for &i in &singing {
                if first[i].is_nan() {
                    first[i] = now;
                }
            }
            some += usize::from(!singing.is_empty());
            more += usize::from(singing.len() > 1);
        }
        let starts: Vec<f32> = first.iter().copied().filter(|v| !v.is_nan()).collect();
        assert!(starts.len() >= 4, "{starts:?}");
        let spread = starts.iter().copied().fold(f32::MIN, f32::max) - starts.iter().copied().fold(f32::MAX, f32::min);
        assert!(spread > 2.0, "they all began within {spread} s");
        // Mostly one at a time.
        assert!((more as f32) < 0.3 * some as f32, "{more} of {some}");
    }

    #[test]
    fn they_wait_for_a_key_when_told_to() {
        let mut p = preset("Village");
        p[SINGS] = 2.0;
        let mut st = BirdsState::new(SR);
        let (l, _) = run(&mut st, &p, 6.0);
        assert!(l.iter().all(|v| v.abs() < 1e-4));
    }
}

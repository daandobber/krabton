//! Sing: type words or sounds and it sings them. The text is cut into
//! syllables (romaji words into theirs, kana into morae, other words whole or
//! on their hyphens), and each syllable becomes a note that carries it as its
//! lyric, for the Singer. How they go is set as a song is: a rhythm (even, a
//! lilt, the pop of 3+3+2, a slow chant, free), a melody over the phrase in
//! the song's key (an arch, rising, falling, waves, a walk, one note, free),
//! longer last syllables of words, breaths between words, the words forwards,
//! backwards, to and fro or shuffled, and a little different each time round.
//! A Singer hears its notes early (`MidiCtx::lead`), so its consonants start
//! before the beat and the vowels land on it, as with a clip's notes.
//!
//! Its data is the text, then (after a 0) what was drawn on its sheet: for
//! each place in the phrase a note (scale steps, for the Drawn melody) and a
//! length (steps; 0 follows the rhythm).

use super::{rate_beats, SeqState};
use crate::midi_fx::{Lyric, MidiCtx, MidiEvent};
use crate::params::{ParamSpec, Unit};
use crate::singer::kana;

pub const RATE: usize = 0;
pub const RHYTHM: usize = 1;
pub const MELODY: usize = 2;
pub const RANGE: usize = 3;
pub const BASE: usize = 4;
pub const WORD_END: usize = 5;
pub const BREATHS: usize = 6;
pub const ORDER: usize = 7;
pub const VARIATION: usize = 8;
pub const GATE: usize = 9;
pub const VELOCITY: usize = 10;

pub const RHYTHMS: &[&str] = &["Even", "Lilt", "Pop", "Chant", "Free"];
pub const MELODIES: &[&str] = &["Arch", "Rise", "Fall", "Wave", "Walk", "Repeat", "Free", "Drawn"];
/// The melody drawn on the sheet.
pub const DRAWN: usize = 7;
pub const ORDERS: &[&str] = &["Forward", "Backward", "Ping-Pong", "Shuffle"];

/// At most this many syllables are sung.
const MAX_SYLLABLES: usize = 128;

pub static SING: &[ParamSpec] = &[
    ParamSpec::choice("Rate", "Clock", super::RATES, 4),
    ParamSpec::choice("Rhythm", "Song", RHYTHMS, 0),
    ParamSpec::choice("Melody", "Song", MELODIES, 0),
    ParamSpec::int("Range", "Song", 0.0, 12.0, 5.0, Unit::None),
    ParamSpec::int("Base Note", "Notes", 36.0, 84.0, 60.0, Unit::None),
    ParamSpec::int("Word End", "Song", 0.0, 4.0, 1.0, Unit::None),
    ParamSpec::lin("Breaths", "Song", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::choice("Order", "Song", ORDERS, 0),
    ParamSpec::lin("Variation", "Song", 0.0, 1.0, 0.2, Unit::Percent),
    ParamSpec::lin("Gate", "Notes", 0.1, 1.0, 0.9, Unit::Percent),
    ParamSpec::lin("Velocity", "Notes", 0.05, 1.0, 0.8, Unit::Percent),
    ParamSpec::toggle("Transpose", "Keys", false),
    ParamSpec::toggle("Hold", "Keys", false),
    ParamSpec::lin("Vel Low", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Vel High", "Dynamics", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::lin("Vel Random", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Vel Accent", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
];

/// What it sings at first: an old Japanese song (Sakura Sakura).
pub fn default_text() -> Vec<u8> {
    b"sakura sakura yayoi no sora wa".to_vec()
}

/// One syllable sung in a pass.
#[derive(Clone, Debug, PartialEq)]
pub struct Sung {
    /// Which syllable (into `Plan::syllables`).
    pub syllable: usize,
    /// Its first step in the pass, and how many steps it sounds.
    pub start: u32,
    pub steps: u32,
    /// Scale steps above the base note.
    pub degree: i32,
    /// The first of its word (a little stronger).
    pub word_start: bool,
}

/// A pass through the text: its syllables, how each is sung, and how many
/// steps it lasts (the same every pass).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Plan {
    pub syllables: Vec<String>,
    pub sung: Vec<Sung>,
    pub length: u32,
}

/// The text in its data (before what was drawn).
pub fn text(data: &[u8]) -> &[u8] {
    data.split(|&b| b == 0).next().unwrap_or(&[])
}

/// What was drawn: a (note, length) for each place in the phrase.
pub fn drawn(data: &[u8]) -> Vec<(i8, u8)> {
    match data.iter().position(|&b| b == 0) {
        Some(i) => data[i + 1..].chunks_exact(2).map(|c| (c[0] as i8, c[1])).collect(),
        None => Vec::new(),
    }
}

/// The data for a text and what was drawn for it.
pub fn with_drawn(text: &[u8], drawn: &[(i8, u8)]) -> Vec<u8> {
    let mut out = text.iter().copied().filter(|&b| b != 0).collect::<Vec<u8>>();
    if !drawn.is_empty() {
        out.push(0);
        for &(d, l) in drawn {
            out.extend_from_slice(&[d as u8, l]);
        }
    }
    out
}

/// The text's words, each as its syllables.
pub fn words(data: &[u8]) -> Vec<Vec<String>> {
    let text = String::from_utf8_lossy(text(data));
    let mut left = MAX_SYLLABLES;
    let mut out = Vec::new();
    for w in text.split(|c: char| c.is_whitespace() || ",.;:!?/()\"、。！？「」".contains(c)).filter(|w| !w.is_empty()) {
        let mut s = kana::split_lyrics(w);
        s.truncate(left);
        left -= s.len();
        if !s.is_empty() {
            out.push(s);
        }
        if left == 0 {
            break;
        }
    }
    out
}

fn fnv(data: &[u8]) -> u64 {
    data.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, &b| (h ^ b as u64).wrapping_mul(0x0100_0000_01b3))
}

/// A number from 0 to 1, the same for the same seeds.
fn unit(a: u64, b: u64) -> f32 {
    let mut z = a ^ b.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    (z >> 40) as f32 / (1u64 << 24) as f32
}

fn pick(v: f32, n: usize) -> usize {
    (v.round().max(0.0) as usize).min(n - 1)
}

/// How pass `pass` through the text goes.
pub fn plan(p: &[f32], data: &[u8], pass: u64) -> Plan {
    let words = words(data);
    // Seeded by the text alone: drawing does not change the rest.
    let seed = fnv(text(data));
    let drawn = drawn(data);
    let get = |i: usize, or: f32| p.get(i).copied().unwrap_or(or);
    let rhythm = pick(get(RHYTHM, 0.0), RHYTHMS.len());
    let melody = pick(get(MELODY, 0.0), MELODIES.len());
    let order = pick(get(ORDER, 0.0), ORDERS.len());
    let range = get(RANGE, 5.0).round().clamp(0.0, 24.0) as i32;
    let word_end = get(WORD_END, 1.0).round().clamp(0.0, 8.0) as u32;
    let breaths = get(BREATHS, 0.0).clamp(0.0, 1.0);
    let variation = get(VARIATION, 0.0).clamp(0.0, 1.0);
    let mut syllables = Vec::new();
    let mut by_word: Vec<Vec<usize>> = Vec::new();
    for w in &words {
        by_word.push((syllables.len()..syllables.len() + w.len()).collect());
        syllables.extend(w.iter().cloned());
    }
    // The words' order this pass (each word's syllables as they go).
    let mut runs: Vec<Vec<usize>> = by_word.clone();
    let backward = order == 1 || (order == 2 && pass % 2 == 1);
    if backward {
        runs.reverse();
        for r in runs.iter_mut() {
            r.reverse();
        }
    } else if order == 3 {
        for i in (1..runs.len()).rev() {
            let j = (unit(seed ^ pass, i as u64 + 77) * (i + 1) as f32) as usize;
            runs.swap(i, j.min(i));
        }
    }
    let n: usize = runs.iter().map(Vec::len).sum();
    let rest = if rhythm == 3 { 2 } else { 1 };
    let mut sung = Vec::with_capacity(n);
    let mut at = 0u32;
    let mut i = 0usize;
    for (wi, run) in runs.iter().enumerate() {
        for (k, &syl) in run.iter().enumerate() {
            let mut steps = match rhythm {
                1 => [2, 1][i % 2],
                2 => [3, 3, 2][i % 3],
                3 => 2,
                4 => [1, 1, 1, 2, 2, 3][(unit(seed, i as u64) * 6.0) as usize % 6],
                _ => 1,
            };
            if k + 1 == run.len() {
                steps += word_end;
            }
            // A length drawn on the sheet.
            if let Some(&(_, l)) = drawn.get(i).filter(|d| d.1 > 0) {
                steps = l as u32;
            }
            let t = if n > 1 { i as f32 / (n - 1) as f32 } else { 0.0 };
            let r = range as f32;
            let mut degree = match melody {
                0 => ((std::f32::consts::PI * t).sin() * r).round() as i32,
                1 => (t * r).round() as i32,
                2 => ((1.0 - t) * r).round() as i32,
                3 => ((0.5 - 0.5 * (std::f32::consts::TAU * 2.0 * t).cos()) * r).round() as i32,
                5 => {
                    if k + 1 == run.len() && run.len() > 1 {
                        range.min(2)
                    } else {
                        0
                    }
                }
                6 => (unit(seed ^ 0xF4EE, i as u64) * (r + 1.0)).floor().min(r) as i32,
                DRAWN => drawn.get(i).map(|d| d.0 as i32).unwrap_or(0),
                _ => 0,
            };
            if melody == 4 {
                // A walk: a step up, down or none from the note before, kept in range.
                let prev = sung.last().map(|s: &Sung| s.degree).unwrap_or(0);
                let u = unit(seed ^ 0x3A1C, i as u64);
                let mut d = prev + if u < 0.35 { -1 } else if u < 0.5 { 0 } else { 1 };
                if d < 0 || d > range {
                    d = prev - (d - prev);
                }
                degree = d.clamp(0, range.max(0));
            }
            // Each time round a little different (the first as it is).
            if pass > 0 && variation > 0.0 && unit(seed ^ pass.wrapping_mul(0x51ED), i as u64) < variation * 0.6 {
                degree += if unit(seed ^ pass, i as u64 + 1000) < 0.5 { 1 } else { -1 };
                degree = degree.clamp(-2, range + 2);
            }
            sung.push(Sung { syllable: syl, start: at, steps, degree, word_start: k == 0 });
            at += steps;
            i += 1;
        }
        // A breath after the word, now and then.
        if breaths > 0.0 && unit(seed ^ 0xB4EA, wi as u64) < breaths {
            at += rest;
        }
    }
    Plan { syllables, sung, length: at }
}

/// The plan's key: the text and the parameters that shape it.
fn key(p: &[f32], data: &[u8]) -> u64 {
    let mut h = fnv(data);
    for &i in &[RHYTHM, MELODY, RANGE, WORD_END, BREATHS, ORDER, VARIATION] {
        h = (h ^ p.get(i).copied().unwrap_or(0.0).to_bits() as u64).wrapping_mul(0x0100_0000_01b3);
    }
    h
}

/// What it keeps between blocks.
pub struct SingState {
    /// Where the next block should start; anything else is a jump.
    next_beat: f64,
    /// How far ahead the notes were sent.
    until: f64,
    last_step: i64,
    /// The plans made (by key and pass), the last two.
    cache: Vec<(u64, u64, Plan)>,
    /// Notes sent ahead: (the beat they sound on, which, pass), for the screen.
    sent: Vec<(f64, u32, u64)>,
}

impl Default for SingState {
    fn default() -> Self {
        Self { next_beat: f64::NEG_INFINITY, until: 0.0, last_step: i64::MIN, cache: Vec::with_capacity(2), sent: Vec::with_capacity(64) }
    }
}

impl SingState {
    fn plan(&mut self, p: &[f32], data: &[u8], pass: u64) -> &Plan {
        let k = key(p, data);
        let i = match self.cache.iter().position(|(ck, cp, _)| *ck == k && *cp == pass) {
            Some(i) => i,
            None => {
                if self.cache.len() >= 2 {
                    self.cache.remove(0);
                }
                self.cache.push((k, pass, plan(p, data, pass)));
                self.cache.len() - 1
            }
        };
        &self.cache[i].2
    }
}

impl SeqState {
    /// The steps due in this block, plus (for a Singer) those due a lead later.
    pub(crate) fn sing(&mut self, p: &[f32], data: &[u8], out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        let rate = rate_beats(p[RATE]);
        let spb = ctx.samples_per_beat;
        let lead = ctx.lead.max(0.0) * ctx.sr as f64 / spb;
        let beat0 = ctx.beat;
        let beat1 = beat0 + ctx.len as f64 / spb;
        if (beat0 - self.sing.next_beat).abs() > 1e-3 {
            // A start, a loop, a locate: from here (what is due at once sung now).
            self.sing.until = beat0;
            self.sing.last_step = i64::MIN;
            self.sing.sent.clear();
        }
        self.sing.next_beat = beat1;
        let hi = beat1 + lead;
        let lo = self.sing.until.max(beat0);
        if hi > lo {
            self.sing.until = hi;
            let mut k = (lo / rate - 1e-9).ceil() as i64;
            while (k as f64) * rate < hi {
                if k >= 0 && k != self.sing.last_step {
                    self.sing.last_step = k;
                    let beat = k as f64 * rate;
                    let send = beat - lead;
                    // Sent a whole lead ahead: the Singer times its consonant before the beat.
                    let early = lead > 0.0 && send >= beat0 - 1e-9;
                    let at = ctx.now + ((send - beat0).max(0.0) * spb) as u64;
                    self.sing_step(k, beat, p, data, at, rate * spb, early, out, ctx);
                }
                k += 1;
            }
        }
        // The screen shows what sounds now, not what was sent ahead.
        let mut now = None;
        self.sing.sent.retain(|&(b, i, pass)| {
            if b <= beat1 {
                now = Some((i, pass));
                false
            } else {
                true
            }
        });
        if let Some((i, pass)) = now {
            self.visual[0] = i as u64 + 1;
            self.visual[1] = pass;
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn sing_step(&mut self, k: i64, beat: f64, p: &[f32], data: &[u8], at: u64, step_frames: f64, early: bool, out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        let length = self.sing.plan(p, data, 0).length as i64;
        if length == 0 {
            return;
        }
        let pass = k.div_euclid(length) as u64;
        let pos = k.rem_euclid(length) as u32;
        let plan = self.sing.plan(p, data, pass);
        let Some(i) = plan.sung.iter().position(|s| s.start == pos) else { return };
        let s = plan.sung[i].clone();
        let lyric = Lyric::new(&plan.syllables[s.syllable], early);
        let (b, _) = ctx.scale.locate(p[BASE].round() as i32);
        let pitch = ctx.scale.pitch_at(b + s.degree);
        let vel = p[VELOCITY] * if s.word_start { 1.0 } else { 0.85 };
        self.note(ctx, at, pitch, vel, s.steps as f64 * step_frames * p[GATE] as f64, out);
        if let Some(e) = out.last_mut().filter(|e| e.on) {
            e.lyric = lyric;
        }
        if self.sing.sent.len() < self.sing.sent.capacity() {
            self.sing.sent.push((beat, i as u32, pass));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::midi_fx::{MidiFxKind, MidiFxState};
    use crate::params::defaults;
    use crate::scale::{Scale, ScaleKind};

    /// What it sings over `beats` beats: (beat sent, pitch, lyric, early).
    fn run(p: &[f32], data: &[u8], beats: f64, lead: f64) -> Vec<(f64, u8, String, bool)> {
        let spb = 24_000.0;
        let mut st = MidiFxState::new(MidiFxKind::Sing);
        let mut ons = Vec::new();
        for b in 0..(beats * spb / 256.0) as u64 {
            let now = b * 256;
            let ctx = MidiCtx { now, len: 256, samples_per_beat: spb, scale: Scale::new(0, ScaleKind::Major), scale_on: true, beat: now as f64 / spb, playing: true, sr: 48_000.0, drum_notes: 0, lead };
            let mut out = Vec::new();
            st.process(MidiFxKind::Sing, p, data, &[], &mut out, &ctx);
            for e in out.iter().filter(|e| e.on) {
                ons.push(((now + e.offset as u64) as f64 / spb, e.pitch, e.lyric.as_str().to_string(), e.lyric.early));
            }
        }
        ons
    }

    fn quiet() -> Vec<f32> {
        let mut p = defaults(SING);
        p[BREATHS] = 0.0;
        p[VARIATION] = 0.0;
        p
    }

    #[test]
    fn it_sings_each_syllable_with_its_lyric() {
        let p = quiet();
        let ons = run(&p, b"sakura sakura", 8.0, 0.0);
        let lyrics: Vec<&str> = ons.iter().map(|o| o.2.as_str()).collect();
        assert_eq!(&lyrics[..6], ["sa", "ku", "ra", "sa", "ku", "ra"]);
        // An eighth each, the last of a word one longer: sa ku ra(2) sa ku ra(2).
        let at: Vec<f64> = ons.iter().take(6).map(|o| o.0).collect();
        assert_eq!(at, [0.0, 0.5, 1.0, 2.0, 2.5, 3.0]);
        // An arch: up in the middle, down at the ends.
        let plan = plan(&p, b"sakura sakura", 0);
        let d: Vec<i32> = plan.sung.iter().map(|s| s.degree).collect();
        assert_eq!(d[0], 0);
        assert!(d[2] > d[0] && d[3] > d[5], "{d:?}");
        assert_eq!(plan.length, 8);
    }

    #[test]
    fn a_singer_hears_it_early() {
        let p = quiet();
        let lead = crate::singer::LEAD_SECS;
        let ons = run(&p, b"la li lo", 6.0, lead);
        let lead_beats = lead * 48_000.0 / 24_000.0;
        // The first comes at once (no time to be early); the rest a lead before their step.
        assert_eq!((ons[0].0, ons[0].3), (0.0, false));
        assert!((ons[1].0 - (1.0 - lead_beats)).abs() < 0.02 && ons[1].3, "{:?}", ons[1]);
        assert_eq!(ons.iter().map(|o| o.2.as_str()).take(3).collect::<Vec<_>>(), ["la", "li", "lo"]);
    }

    #[test]
    fn backwards_and_to_and_fro() {
        let mut p = quiet();
        p[ORDER] = 1.0;
        let back = plan(&p, b"sakura", 0);
        let s: Vec<&str> = back.sung.iter().map(|s| back.syllables[s.syllable].as_str()).collect();
        assert_eq!(s, ["ra", "ku", "sa"]);
        p[ORDER] = 2.0;
        let (a, b) = (plan(&p, b"sakura no", 0), plan(&p, b"sakura no", 1));
        assert_eq!(a.length, b.length, "every pass as long");
        assert_ne!(a.sung[0].syllable, b.sung[0].syllable);
    }

    #[test]
    fn kana_and_words_split_into_syllables() {
        assert_eq!(words("さくら, hap-py!".as_bytes()), [vec!["さ", "く", "ら"], vec!["hap", "py"]]);
        assert!(words(b"").is_empty());
        let p = quiet();
        assert_eq!(plan(&p, b"   ", 0).length, 0);
        assert!(run(&p, b"", 4.0, 0.0).is_empty());
    }

    #[test]
    fn a_melody_drawn_on_the_sheet_is_sung() {
        let mut p = quiet();
        p[MELODY] = DRAWN as f32;
        let data = with_drawn(b"la li lo", &[(0, 0), (4, 0), (-2, 3)]);
        assert_eq!(text(&data), b"la li lo");
        assert_eq!(drawn(&data), [(0, 0), (4, 0), (-2, 3)]);
        let plan = plan(&p, &data, 0);
        assert_eq!(plan.sung.iter().map(|s| s.degree).collect::<Vec<_>>(), [0, 4, -2]);
        // The last one drawn three long; the others as the rhythm goes (a word's end one longer).
        assert_eq!(plan.sung.iter().map(|s| s.steps).collect::<Vec<_>>(), [2, 2, 3]);
        // Another melody leaves the drawn notes be, but keeps the drawn lengths.
        p[MELODY] = 0.0;
        assert_eq!(super::plan(&p, &data, 0).sung[2].steps, 3);
        let ons = run(&p, &data, 2.0, 0.0);
        assert_eq!(ons.first().map(|o| o.2.as_str()), Some("la"));
    }

    #[test]
    fn to_clip_keeps_the_lyrics() {
        let p = quiet();
        let notes = crate::midi_fx::render_notes(MidiFxKind::Sing, &p, b"sakura", 4.0, 120.0, Scale::new(0, ScaleKind::Major), true, 0);
        let lyrics: Vec<&str> = notes.iter().map(|n| n.lyric.as_str()).collect();
        assert_eq!(&lyrics[..3], ["sa", "ku", "ra"]);
    }
}

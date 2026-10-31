//! Grooves, as in Live's Groove Pool: the feel of a beat (how late or early
//! each step of the grid falls, how loud it is) taken from a recording or a
//! MIDI clip, or one of the swings made here, put on MIDI clips.
//!
//! A groove is a round of steps on a grid (sixteenths, say, over a bar):
//! each step moves the notes on it by some beats and makes them louder or
//! softer. A clip that uses one keeps its notes where they are; they play
//! moved (`Feel`, in the engine when it plays them, so it can be turned up
//! and down while the clip plays), and Commit writes the feel into them.
//! Notes between steps move by as much as the steps either side, in the
//! measure they lie between them.
//!
//! Extract Groove listens for where the hits of a clip fall against the
//! grid: each hit goes to the step it is a little late or early for (late
//! more readily than early, as a swung note is late), the steps' shifts are
//! what their hits have in common over the rounds, and their accents how
//! much louder or softer than the average their hits are. The groove keeps
//! the beats where they are, so a clip does not move as a whole.

use crate::project::{note_random, Note};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

/// A groove: `steps` on a grid of `base` beats, one round of them
/// `steps.len() * base` beats long, again and again from a clip's start.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Groove {
    pub name: String,
    /// Beats between the steps (0.25: sixteenths).
    pub base: f64,
    pub steps: Vec<GrooveStep>,
}

/// A step of a groove.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct GrooveStep {
    /// How far a note on it moves, in beats (later when more than 0).
    pub shift: f64,
    /// How much louder (up to 1: twice) or softer (down to -1: silent) a
    /// note on it plays.
    pub accent: f32,
}

/// A clip's groove: which (by name, from the project's pool or the ones
/// made here) and how much of it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ClipGroove {
    pub name: String,
    /// How much of the groove's timing the notes take (0..1).
    #[serde(default = "one")]
    pub timing: f32,
    /// How much of its accents (0..1).
    #[serde(default = "one")]
    pub velocity: f32,
    /// How far the notes wander from where they would be, a little
    /// differently each time round (0..1, at most `RANDOM_BEATS`).
    #[serde(default)]
    pub random: f32,
}

fn one() -> f32 {
    1.0
}

impl ClipGroove {
    /// The groove named, all of its timing and accents.
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into(), timing: 1.0, velocity: 1.0, random: 0.0 }
    }
}

/// How far Random moves a note at 100 %, either way (beats: about 25 ms at
/// 120 BPM).
pub const RANDOM_BEATS: f64 = 0.05;

impl Groove {
    /// One round of it, in beats.
    pub fn len(&self) -> f64 {
        self.steps.len() as f64 * self.base
    }

    pub fn is_empty(&self) -> bool {
        self.steps.is_empty() || self.base <= 0.0
    }

    /// How far a note at `beat` (of its clip) moves, and its accent: the
    /// steps either side shift it in the measure it lies between them; the
    /// nearer one gives the accent.
    pub fn at(&self, beat: f64) -> (f64, f32) {
        if self.is_empty() {
            return (0.0, 0.0);
        }
        let n = self.steps.len() as i64;
        let x = beat / self.base;
        let i = x.floor();
        let f = x - i;
        let a = self.steps[(i as i64).rem_euclid(n) as usize];
        let b = self.steps[(i as i64 + 1).rem_euclid(n) as usize];
        let shift = a.shift * (1.0 - f) + b.shift * f;
        (shift, if f < 0.5 { a.accent } else { b.accent })
    }

    /// A swing: of every pair of steps of `base` beats the second goes
    /// late, to `percent` of the pair (50: straight, 66.7: triplets). A
    /// bar of 4/4 long.
    pub fn swing(name: impl Into<String>, base: f64, percent: f64) -> Self {
        let shift = (percent / 100.0 - 0.5) * 2.0 * base;
        let pairs = (4.0 / (2.0 * base)).round().max(1.0) as usize;
        let steps = (0..pairs * 2).map(|k| GrooveStep { shift: if k % 2 == 1 { shift } else { 0.0 }, accent: 0.0 }).collect();
        Self { name: name.into(), base, steps }
    }
}

/// A groove at work on a clip: the groove and how much of it.
#[derive(Clone, Copy, Debug)]
pub struct Feel<'a> {
    pub groove: &'a Groove,
    pub timing: f64,
    pub velocity: f32,
    pub random: f64,
}

impl<'a> Feel<'a> {
    pub fn new(groove: &'a Groove, amounts: &ClipGroove) -> Self {
        Self { groove, timing: amounts.timing.clamp(0.0, 1.0) as f64, velocity: amounts.velocity.clamp(0.0, 1.0), random: amounts.random.clamp(0.0, 1.0) as f64 }
    }

    /// Where note `n` (the clip's `idx`th) starts on round `k` of a clip
    /// `len` beats long, and how loud it plays: moved by the groove, kept
    /// inside the clip (a note at its start that the groove would pull
    /// earlier stays there).
    pub fn place(&self, n: &Note, idx: usize, k: i64, len: f64) -> (f64, f32) {
        let (shift, accent) = self.groove.at(n.start);
        let mut start = n.start + shift * self.timing;
        if self.random > 0.0 {
            start += (note_random(k, idx, n, 3) as f64 * 2.0 - 1.0) * self.random * RANDOM_BEATS;
        }
        let start = start.clamp(0.0, (len - 1e-6).max(0.0));
        let vel = if accent != 0.0 && self.velocity > 0.0 { (n.velocity * (1.0 + self.velocity * accent)).clamp(0.01, 1.0) } else { n.velocity };
        (start, vel)
    }
}

/// Writes the feel into `notes` (of a clip `len` beats long), as they play
/// the first time round (Commit): each where it plays (as long), as loud.
pub fn commit(notes: &mut [Note], feel: &Feel, len: f64) {
    let placed: Vec<(f64, f32)> = notes.iter().enumerate().map(|(i, n)| feel.place(n, i, 0, len)).collect();
    for (n, (start, vel)) in notes.iter_mut().zip(placed) {
        if n.start >= len || n.start < 0.0 {
            continue;
        }
        n.start = start;
        n.velocity = vel;
    }
}

/// The grooves Krabton comes with: swings of sixteenths and of eighths
/// from barely to fully, a shuffle, a laid-back feel and a pushed one.
pub fn built_in() -> &'static [Groove] {
    static ALL: OnceLock<Vec<Groove>> = OnceLock::new();
    ALL.get_or_init(|| {
        let mut v = Vec::new();
        for pct in [54.0, 58.0, 62.0, 66.0, 71.0, 75.0] {
            v.push(Groove::swing(format!("Swing 16ths {pct:.0}%"), 0.25, pct));
        }
        for pct in [54.0, 58.0, 62.0, 66.0, 71.0, 75.0] {
            v.push(Groove::swing(format!("Swing 8ths {pct:.0}%"), 0.5, pct));
        }
        // A shuffle: eighths in triplets, the beats leaning on.
        let mut shuffle = Groove::swing("Shuffle", 0.5, 66.7);
        for (k, s) in shuffle.steps.iter_mut().enumerate() {
            s.accent = if k % 2 == 0 { 0.12 } else { -0.2 };
        }
        v.push(shuffle);
        // Laid back: lightly swung sixteenths, the backbeat (2 and 4) behind
        // the beat and the in-between sixteenths soft, as in a lazy hip-hop beat.
        let mut lazy = Groove::swing("Laid Back", 0.25, 57.0);
        for (k, s) in lazy.steps.iter_mut().enumerate() {
            if k == 4 || k == 12 {
                s.shift = 0.04;
            }
            s.accent = match k % 4 {
                0 => 0.1,
                2 => -0.05,
                _ => -0.3,
            };
        }
        v.push(lazy);
        // Pushed: the off-beats a little ahead, as a drummer who drives.
        let mut push = Groove::swing("Push", 0.25, 50.0);
        for (k, s) in push.steps.iter_mut().enumerate() {
            s.shift = match k % 4 {
                1 | 3 => -0.015,
                2 => -0.02,
                _ => 0.0,
            };
            s.accent = if k % 4 == 2 { 0.1 } else { 0.0 };
        }
        v.push(push);
        v
    })
}

/// A groove by name: the pool's (`pool`, the project's) first, then the
/// ones made here.
pub fn find<'a>(pool: &'a [Groove], name: &str) -> Option<&'a Groove> {
    pool.iter().find(|g| g.name == name).or_else(|| built_in().iter().find(|g| g.name == name))
}

/// The leaning of late hits over early ones (in steps) when a hit is put
/// on its step: a hit up to this much past the middle between two steps
/// still goes to the earlier one (a swung note is late).
const LATE_BIAS: f64 = 0.1;

/// The groove of hits at `beats` (from the start of what was heard, with
/// how strong each is, 0..1), on a grid of `base` beats, a round `cycle`
/// beats long.
pub fn extract(name: impl Into<String>, hits: &[(f64, f32)], base: f64, cycle: f64) -> Groove {
    let base = base.max(1.0 / 64.0);
    let n = ((cycle / base).round() as usize).max(1);
    let mut shifts: Vec<Vec<f64>> = vec![Vec::new(); n];
    let mut levels: Vec<Vec<f32>> = vec![Vec::new(); n];
    for &(b, w) in hits {
        let j = ((b / base) - LATE_BIAS).round();
        let shift = b - j * base;
        if shift.abs() > 0.6 * base {
            continue;
        }
        let k = (j as i64).rem_euclid(n as i64) as usize;
        shifts[k].push(shift);
        levels[k].push(w);
    }
    let median = |v: &mut Vec<f64>| -> Option<f64> {
        if v.is_empty() {
            return None;
        }
        v.sort_by(f64::total_cmp);
        let m = v.len() / 2;
        Some(if v.len() % 2 == 1 { v[m] } else { 0.5 * (v[m - 1] + v[m]) })
    };
    let mut got: Vec<Option<f64>> = shifts.iter_mut().map(median).collect();
    // A step no hit fell on moves as its like in the other beats do (the
    // second sixteenth of every beat alike), else not at all.
    let per_beat = ((1.0 / base).round() as usize).max(1);
    let like: Vec<Option<f64>> = (0..per_beat)
        .map(|p| {
            let v: Vec<f64> = (p..n).step_by(per_beat).filter_map(|k| got[k]).collect();
            (!v.is_empty()).then(|| v.iter().sum::<f64>() / v.len() as f64)
        })
        .collect();
    for (k, g) in got.iter_mut().enumerate() {
        if g.is_none() {
            *g = like[k % per_beat];
        }
    }
    // The beats stay where they are: what the steps on them share comes off all.
    let on_beat: Vec<f64> = (0..n).step_by(per_beat).filter(|&k| !shifts[k].is_empty()).filter_map(|k| got[k]).collect();
    let centre = if on_beat.is_empty() { 0.0 } else { on_beat.iter().sum::<f64>() / on_beat.len() as f64 };
    let level: Vec<Option<f32>> = levels.iter().map(|v| (!v.is_empty()).then(|| v.iter().sum::<f32>() / v.len() as f32)).collect();
    let heard: Vec<f32> = level.iter().flatten().copied().collect();
    let mean = if heard.is_empty() { 0.0 } else { heard.iter().sum::<f32>() / heard.len() as f32 };
    let steps = (0..n)
        .map(|k| GrooveStep {
            shift: got[k].map_or(0.0, |s| s - centre),
            accent: match level[k] {
                Some(l) if mean > 1e-6 => (l / mean - 1.0).clamp(-1.0, 1.0),
                _ => 0.0,
            },
        })
        .collect();
    Groove { name: name.into(), base, steps }
}

/// The hits of MIDI notes for [`extract`]: where notes start (one hit for
/// all that start together, as loud as the loudest), with their velocities.
pub fn note_hits(notes: &[Note], len: f64) -> Vec<(f64, f32)> {
    let mut v: Vec<(f64, f32)> = notes.iter().filter(|n| n.start >= 0.0 && n.start < len).map(|n| (n.start, n.velocity)).collect();
    v.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut out: Vec<(f64, f32)> = Vec::with_capacity(v.len());
    for (b, w) in v {
        match out.last_mut() {
            Some(last) if b - last.0 < 1e-3 => last.1 = last.1.max(w),
            _ => out.push((b, w)),
        }
    }
    out
}

/// A name for a new groove of the pool: `name`, or with a number after it
/// when the pool (or the ones made here) has it already.
pub fn free_name(pool: &[Groove], name: &str) -> String {
    let taken = |n: &str| find(pool, n).is_some();
    if !taken(name) {
        return name.to_string();
    }
    (2..).map(|k| format!("{name} {k}")).find(|n| !taken(n)).unwrap_or_default()
}

/// The hits of an audio clip `len` beats long for [`extract`]: its
/// transients (as warping finds them) in what it plays (its loop, or from
/// its Start to its End marker), in beats from there through its warp
/// markers, with how strong each is; and how many beats that is.
pub fn audio_hits(a: &crate::project::AudioClip, len: f64) -> (Vec<(f64, f32)>, f64) {
    let (b0, b1) = if a.looping { (a.loop_start, a.loop_start + len.max(0.0)) } else { (a.start, a.end_beat().max(a.start)) };
    let src = if a.reverse { a.sample.reversed() } else { &a.sample };
    let an = src.analysis();
    let sr = src.sample_rate.max(1) as f64;
    let hits = an
        .transients
        .iter()
        .zip(&an.strengths)
        .map(|(&f, &w)| (a.beat_at(f as f64 / sr) - b0, w))
        .filter(|&(b, _)| b > -0.05 && b < b1 - b0 - 0.05)
        .map(|(b, w)| (b.max(0.0), w))
        .collect();
    (hits, b1 - b0)
}

/// Extract Groove: the groove of a clip (its hits, or a MIDI clip's
/// notes) on sixteenths, a round a bar long (`beats_per_bar`), or as long as
/// the clip when it is shorter. None when it has too few hits.
pub fn from_clip(clip: &crate::project::Clip, name: impl Into<String>, beats_per_bar: f64) -> Option<Groove> {
    let (hits, len) = match &clip.content {
        crate::project::ClipContent::Audio(a) => audio_hits(a, clip.length),
        crate::project::ClipContent::Midi { notes } => (note_hits(notes, clip.length), clip.length),
    };
    if hits.len() < 3 {
        return None;
    }
    let base = 0.25;
    let cycle = if len >= beats_per_bar - 1e-6 { beats_per_bar } else { ((len / base).round() * base).max(base) };
    Some(extract(name, &hits, base, cycle))
}

impl crate::project::Project {
    /// The feel `clip` plays with: its groove (from the pool, or one made
    /// here) and how much of it. None without one, or when its groove is gone.
    pub fn feel<'a>(&'a self, clip: &'a crate::project::Clip) -> Option<Feel<'a>> {
        let amounts = clip.groove.as_ref()?;
        find(&self.grooves, &amounts.name).map(|g| Feel::new(g, amounts))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hats(swing: f64, bars: usize) -> Vec<(f64, f32)> {
        // Sixteenths, the second of every pair late, the beats loud.
        let late = (swing / 100.0 - 0.5) * 0.5;
        (0..bars * 16).map(|k| (k as f64 * 0.25 + if k % 2 == 1 { late } else { 0.0 }, if k % 4 == 0 { 1.0 } else { 0.5 })).collect()
    }

    #[test]
    fn a_swung_beat_gives_its_swing() {
        for swing in [54.0, 62.0, 66.7, 75.0] {
            let g = extract("Hats", &hats(swing, 2), 0.25, 4.0);
            let want = (swing / 100.0 - 0.5) * 0.5;
            assert_eq!(g.steps.len(), 16);
            for (k, s) in g.steps.iter().enumerate() {
                let w = if k % 2 == 1 { want } else { 0.0 };
                assert!((s.shift - w).abs() < 1e-9, "{swing}%: step {k} moves {} not {w}", s.shift);
            }
            // The beats louder than the rest.
            assert!(g.steps[0].accent > 0.3 && g.steps[1].accent < -0.1, "{:?}", g.steps[0]);
        }
    }

    #[test]
    fn a_late_beat_moves_nothing_as_a_whole() {
        // Everything 10 ms late at 120 BPM: no groove at all.
        let hits: Vec<(f64, f32)> = (0..16).map(|k| (k as f64 * 0.25 + 0.02, 1.0)).collect();
        let g = extract("Late", &hits, 0.25, 4.0);
        assert!(g.steps.iter().all(|s| s.shift.abs() < 1e-9 && s.accent == 0.0), "{g:?}");
    }

    #[test]
    fn steps_without_hits_follow_their_likes() {
        // Eighth-note hats with one swung sixteenth, after the third beat.
        let mut hits: Vec<(f64, f32)> = (0..8).map(|k| (k as f64 * 0.5, 1.0)).collect();
        hits.push((2.25 + 0.06, 1.0));
        hits.sort_by(|a, b| a.0.total_cmp(&b.0));
        let g = extract("Sparse", &hits, 0.25, 4.0);
        for k in [1, 5, 9, 13] {
            assert!((g.steps[k].shift - 0.06).abs() < 1e-9, "step {k}: {:?}", g.steps[k]);
        }
    }

    #[test]
    fn a_groove_moves_the_notes_by_its_amount() {
        let g = Groove::swing("Swing 16ths 66%", 0.25, 66.0);
        let late = 0.16 * 0.5;
        let n = |start: f64| Note::new(60, start, 0.25, 0.8);
        for (amount, want) in [(0.0, 0.0), (0.5, late * 0.5), (1.0, late)] {
            let f = Feel::new(&g, &ClipGroove { timing: amount, ..ClipGroove::new("x") });
            assert_eq!(f.place(&n(0.0), 0, 0, 4.0).0, 0.0);
            assert!((f.place(&n(0.25), 0, 0, 4.0).0 - (0.25 + want)).abs() < 1e-12, "{amount}");
            assert!((f.place(&n(1.75), 0, 0, 4.0).0 - (1.75 + want)).abs() < 1e-12);
            // An eighth stays, a thirty-second between moves half as much.
            assert_eq!(f.place(&n(0.5), 0, 0, 4.0).0, 0.5);
            assert!((f.place(&n(0.125), 0, 0, 4.0).0 - (0.125 + want * 0.5)).abs() < 1e-12);
        }
    }

    #[test]
    fn accents_and_random_are_as_much_as_asked() {
        let mut g = Groove::swing("x", 0.25, 50.0);
        g.steps[0].accent = 0.5;
        g.steps[1].accent = -0.5;
        let n0 = Note::new(60, 0.0, 0.25, 0.6);
        let n1 = Note::new(60, 0.25, 0.25, 0.6);
        for (amount, loud, soft) in [(0.0, 0.6, 0.6), (0.5, 0.75, 0.45), (1.0, 0.9, 0.3)] {
            let f = Feel::new(&g, &ClipGroove { velocity: amount, ..ClipGroove::new("x") });
            assert!((f.place(&n0, 0, 0, 4.0).1 - loud).abs() < 1e-6);
            assert!((f.place(&n1, 1, 0, 4.0).1 - soft).abs() < 1e-6);
        }
        // Random: within its reach, the same for the same round, another the next.
        let f = Feel::new(&g, &ClipGroove { random: 1.0, ..ClipGroove::new("x") });
        let a = f.place(&n1, 1, 0, 4.0).0;
        assert!((a - 0.25).abs() <= RANDOM_BEATS && a != 0.25);
        assert_eq!(a, f.place(&n1, 1, 0, 4.0).0);
        assert_ne!(a, f.place(&n1, 1, 1, 4.0).0);
    }

    #[test]
    fn commit_writes_what_plays() {
        let g = find(&[], "Laid Back").unwrap();
        let notes: Vec<Note> = (0..16).map(|k| Note::new(36 + k as u8 % 3, k as f64 * 0.25, 0.2, 0.7)).collect();
        let amounts = ClipGroove { timing: 0.8, velocity: 0.6, random: 0.3, ..ClipGroove::new("Laid Back") };
        let f = Feel::new(g, &amounts);
        let mut done = notes.clone();
        commit(&mut done, &f, 4.0);
        for (i, (n, d)) in notes.iter().zip(&done).enumerate() {
            let (s, v) = f.place(n, i, 0, 4.0);
            assert_eq!((d.start, d.velocity), (s, v));
            assert_eq!(d.length, n.length, "as long as it was");
        }
    }

    #[test]
    fn the_built_in_grooves_have_their_names_and_lengths() {
        let all = built_in();
        assert!(all.len() >= 15);
        assert!(all.iter().all(|g| (g.len() - 4.0).abs() < 1e-9), "a bar each");
        let sixteenths = find(&[], "Swing 16ths 66%").unwrap();
        assert!((sixteenths.steps[1].shift - 0.08).abs() < 1e-9);
        let eighths = find(&[], "Swing 8ths 58%").unwrap();
        assert!((eighths.steps[1].shift - 0.08).abs() < 1e-9 && eighths.base == 0.5);
        // The pool's own come first, and new names do not clash.
        let pool = vec![Groove { name: "Shuffle".into(), base: 0.25, steps: vec![GrooveStep::default(); 4] }];
        assert_eq!(find(&pool, "Shuffle").unwrap().base, 0.25);
        assert_eq!(free_name(&pool, "Shuffle"), "Shuffle 2");
        assert_eq!(free_name(&pool, "Break Groove"), "Break Groove");
    }

    #[test]
    fn a_swung_recording_gives_its_swing() {
        use crate::analysis::tests::drum;
        use crate::project::{AudioClip, Clip, ClipContent};
        use crate::sample::{SampleData, SampleRef};
        // Two bars at 120 BPM: a kick on the beats, hats on every sixteenth,
        // the second of each pair at 62 %, a snare on 2 and 4.
        let sr = 48_000;
        let beat = sr as usize / 2;
        let late = (0.62 - 0.5) * 0.5;
        let mut x = vec![0.0f32; beat * 8];
        for k in 0..32 {
            let at = ((k as f64 * 0.25 + if k % 2 == 1 { late } else { 0.0 }) * beat as f64) as usize;
            let mut add = |kind: char, seed: u32| {
                for (i, v) in drum(kind, sr, seed).iter().enumerate() {
                    if let Some(o) = x.get_mut(at + i) {
                        *o += v;
                    }
                }
            };
            add('h', 3 + k);
            if k % 4 == 0 {
                add(if k % 8 == 4 { 's' } else { 'k' }, 99 + k);
            }
        }
        let s = SampleRef::new(SampleData::from_buffers("Swung", None, sr, x.clone(), x));
        let mut clip = Clip::new_midi(1, "Swung", [0, 0, 0], 8.0);
        clip.content = ClipContent::Audio(AudioClip::new(s, 120.0));
        let g = from_clip(&clip, "Swung Groove", 4.0).expect("a groove");
        assert_eq!((g.steps.len(), g.base), (16, 0.25));
        for (k, st) in g.steps.iter().enumerate() {
            let want = if k % 2 == 1 { late } else { 0.0 };
            // Within 3 ms at 120 BPM.
            assert!((st.shift - want).abs() < 0.006, "step {k}: {} for {want}", st.shift);
        }
        // The beats (kicks and snares) louder.
        assert!(g.steps[0].accent > 0.0 && g.steps[1].accent < 0.0, "{:?}", &g.steps[..4]);
        // From a MIDI clip of the same feel, the same groove.
        let notes: Vec<Note> = (0..32).map(|k| Note::new(42, k as f64 * 0.25 + if k % 2 == 1 { late } else { 0.0 }, 0.1, 0.8)).collect();
        let midi = Clip { content: ClipContent::Midi { notes }, ..Clip::new_midi(2, "Hats", [0, 0, 0], 8.0) };
        let m = from_clip(&midi, "Hats Groove", 4.0).unwrap();
        assert!((m.steps[1].shift - late).abs() < 1e-9 && m.steps[2].shift.abs() < 1e-9);
    }

    #[test]
    fn notes_together_are_one_hit() {
        let notes = vec![Note::new(36, 0.0, 0.25, 0.5), Note::new(42, 0.0, 0.25, 0.9), Note::new(42, 0.5, 0.25, 0.4), Note::new(42, 9.0, 0.25, 0.4)];
        assert_eq!(note_hits(&notes, 4.0), vec![(0.0, 0.9), (0.5, 0.4)]);
    }
}

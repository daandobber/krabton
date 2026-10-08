//! Scale-aware MIDI tools for the piano roll, in the spirit of Live 12's
//! transformations and generators. All functions work on a note list and a
//! selection (indices); an empty selection means "all notes".

use crate::project::Note;
use crate::scale::Scale;

/// Small deterministic RNG so tools are reproducible for a given seed.
pub struct Rand(u64);

impl Rand {
    pub fn new(seed: u64) -> Self {
        Self(seed ^ 0x9E37_79B9_7F4A_7C15)
    }

    pub fn next_u64(&mut self) -> u64 {
        // SplitMix64
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in [0, 1).
    pub fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    pub fn range(&mut self, lo: i32, hi: i32) -> i32 {
        lo + (self.unit() * (hi - lo + 1) as f64).floor() as i32
    }
}

fn targets(notes: &[Note], sel: &[usize]) -> Vec<usize> {
    if sel.is_empty() {
        (0..notes.len()).collect()
    } else {
        let mut v: Vec<usize> = sel.iter().copied().filter(|&i| i < notes.len()).collect();
        v.sort_unstable();
        v.dedup();
        v
    }
}

/// Notes in `idx` grouped by (nearly) identical start time, sorted by pitch.
fn chord_groups(notes: &[Note], idx: &[usize]) -> Vec<Vec<usize>> {
    let mut sorted = idx.to_vec();
    sorted.sort_by(|&a, &b| notes[a].start.total_cmp(&notes[b].start).then(notes[a].pitch.cmp(&notes[b].pitch)));
    let mut groups: Vec<Vec<usize>> = Vec::new();
    for i in sorted {
        match groups.last_mut() {
            Some(g) if (notes[g[0]].start - notes[i].start).abs() < 1e-4 => g.push(i),
            _ => groups.push(vec![i]),
        }
    }
    groups
}

/// Moves out-of-scale notes to the nearest scale note. Returns how many moved.
pub fn snap_to_scale(notes: &mut [Note], sel: &[usize], scale: &Scale) -> usize {
    let mut moved = 0;
    for i in targets(notes, sel) {
        let p = scale.snap(notes[i].pitch);
        if p != notes[i].pitch {
            notes[i].pitch = p;
            moved += 1;
        }
    }
    moved
}

/// Transposes by scale steps instead of semitones.
pub fn transpose_degrees(notes: &mut [Note], sel: &[usize], scale: &Scale, degrees: i32) {
    for i in targets(notes, sel) {
        notes[i].pitch = scale.step(notes[i].pitch, degrees);
    }
}

/// Turns every target note into a diatonic chord of `size` notes (3 = triad,
/// 4 = seventh chord). Returns the indices of all chord notes.
pub fn chordify(notes: &mut Vec<Note>, sel: &[usize], scale: &Scale, size: usize) -> Vec<usize> {
    let mut out = Vec::new();
    for i in targets(notes, sel) {
        let chord = scale.chord(notes[i].pitch, size.max(1));
        let Some(&root) = chord.first() else { continue };
        notes[i].pitch = root;
        out.push(i);
        let base = notes[i].clone();
        for &p in chord.iter().skip(1) {
            let exists = notes.iter().any(|n| n.pitch == p && (n.start - base.start).abs() < 1e-4);
            if !exists {
                notes.push(Note { pitch: p, velocity: base.velocity * 0.9, ..base.clone() });
                out.push(notes.len() - 1);
            }
        }
    }
    out
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArpStyle {
    Up,
    Down,
    UpDown,
    Random,
}

impl ArpStyle {
    pub const ALL: [ArpStyle; 4] = [ArpStyle::Up, ArpStyle::Down, ArpStyle::UpDown, ArpStyle::Random];

    pub fn name(self) -> &'static str {
        match self {
            ArpStyle::Up => "Up",
            ArpStyle::Down => "Down",
            ArpStyle::UpDown => "Up/Down",
            ArpStyle::Random => "Random",
        }
    }
}

/// Replaces each chord (notes starting together) with an arpeggio over the
/// chord's duration. Returns the indices of the generated notes.
pub fn arpeggiate(notes: &mut Vec<Note>, sel: &[usize], rate: f64, style: ArpStyle, seed: u64) -> Vec<usize> {
    let rate = rate.max(1.0 / 64.0);
    let idx = targets(notes, sel);
    let groups = chord_groups(notes, &idx);
    let mut rng = Rand::new(seed);
    let mut generated = Vec::new();
    for g in &groups {
        let start = notes[g[0]].start;
        let span = g.iter().map(|&i| notes[i].length).fold(0.0, f64::max).max(rate);
        let pitches: Vec<u8> = g.iter().map(|&i| notes[i].pitch).collect();
        let vel = g.iter().map(|&i| notes[i].velocity).fold(0.0, f32::max);
        let seq: Vec<u8> = match style {
            ArpStyle::Up => pitches.clone(),
            ArpStyle::Down => pitches.iter().rev().copied().collect(),
            ArpStyle::UpDown => {
                let mut s = pitches.clone();
                if pitches.len() > 2 {
                    s.extend(pitches.iter().rev().skip(1).take(pitches.len() - 2));
                } else if pitches.len() == 2 {
                    // Up/down of two notes is just alternating.
                }
                s
            }
            ArpStyle::Random => pitches.clone(),
        };
        let steps = ((span / rate) - 1e-9).ceil().max(1.0) as usize;
        for k in 0..steps {
            let p = if style == ArpStyle::Random { pitches[rng.range(0, pitches.len() as i32 - 1) as usize] } else { seq[k % seq.len()] };
            let t = start + k as f64 * rate;
            let len = (rate * 0.9).min(start + span - t);
            if len > 1e-4 {
                generated.push(Note::new(p, t, len, if k % 2 == 0 { vel } else { vel * 0.85 }));
            }
        }
    }
    let remove: std::collections::HashSet<usize> = groups.iter().flatten().copied().collect();
    let mut keep: Vec<Note> = notes.iter().enumerate().filter(|(i, _)| !remove.contains(i)).map(|(_, n)| n.clone()).collect();
    let first = keep.len();
    keep.extend(generated);
    *notes = keep;
    (first..notes.len()).collect()
}

/// Offsets the notes of each chord like a guitar strum (`amount` beats per
/// string; negative strums downwards).
pub fn strum(notes: &mut [Note], sel: &[usize], amount: f64) {
    let idx = targets(notes, sel);
    for g in chord_groups(notes, &idx) {
        let order: Vec<usize> = if amount >= 0.0 { g.clone() } else { g.iter().rev().copied().collect() };
        for (k, &i) in order.iter().enumerate() {
            let shift = (k as f64 * amount.abs()).min(notes[i].length * 0.9);
            notes[i].start += shift;
            notes[i].length -= shift;
        }
    }
}

/// Extends each note until the next note starts (per pitch-independent timeline).
pub fn legato(notes: &mut [Note], sel: &[usize], clip_len: f64) {
    let idx = targets(notes, sel);
    let mut starts: Vec<f64> = idx.iter().map(|&i| notes[i].start).collect();
    starts.sort_by(f64::total_cmp);
    starts.dedup_by(|a, b| (*a - *b).abs() < 1e-4);
    for &i in &idx {
        let s = notes[i].start;
        let next = starts.iter().copied().find(|&t| t > s + 1e-4).unwrap_or(clip_len);
        notes[i].length = (next - s).max(1.0 / 64.0);
    }
}

/// Random timing and velocity variation.
pub fn humanize(notes: &mut [Note], sel: &[usize], timing: f64, velocity: f32, seed: u64) {
    let mut rng = Rand::new(seed);
    for i in targets(notes, sel) {
        let dt = (rng.unit() * 2.0 - 1.0) * timing;
        let dv = (rng.unit() as f32 * 2.0 - 1.0) * velocity;
        notes[i].start = (notes[i].start + dt).max(0.0);
        notes[i].velocity = (notes[i].velocity + dv).clamp(0.05, 1.0);
    }
}

/// Generates a melody in the scale, a random walk over scale degrees
/// (Live 12's "Seed" generator).
pub fn seed_melody(scale: &Scale, length: f64, step: f64, density: f64, low: u8, high: u8, seed: u64) -> Vec<Note> {
    let step = step.max(1.0 / 16.0);
    let (low, high) = (low.min(high), low.max(high));
    let mut rng = Rand::new(seed);
    let (lo_deg, _) = scale.locate(scale.snap(low) as i32);
    let (hi_deg, _) = scale.locate(scale.snap(high) as i32);
    let mut deg = rng.range(lo_deg, hi_deg.max(lo_deg));
    let mut out: Vec<Note> = Vec::new();
    let mut t = 0.0;
    while t < length - 1e-9 {
        let on_beat = (t.fract()).abs() < 1e-9;
        let p = if on_beat { density.max(0.0).min(1.0) * 0.6 + 0.4 } else { density };
        if rng.unit() < p {
            deg += rng.range(-2, 2);
            if deg < lo_deg {
                deg = lo_deg + (lo_deg - deg);
            }
            if deg > hi_deg {
                deg = hi_deg - (deg - hi_deg);
            }
            deg = deg.clamp(lo_deg, hi_deg.max(lo_deg));
            let pitch = scale.pitch_at(deg).clamp(0, 127) as u8;
            let len = if rng.unit() < 0.25 { step * 2.0 } else { step };
            let len = len.min(length - t);
            let vel = (0.6 + rng.unit() * 0.4) as f32 * if on_beat { 1.0 } else { 0.85 };
            out.push(Note::new(pitch, t, len * 0.95, vel));
        }
        t += step;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scale::ScaleKind;

    fn c_major() -> Scale {
        Scale::new(0, ScaleKind::Major)
    }

    #[test]
    fn snap_moves_only_outsiders() {
        let mut notes = vec![Note::new(60, 0.0, 1.0, 1.0), Note::new(61, 1.0, 1.0, 1.0), Note::new(66, 2.0, 1.0, 1.0)];
        let moved = snap_to_scale(&mut notes, &[], &c_major());
        assert_eq!(moved, 2);
        assert!(notes.iter().all(|n| c_major().contains(n.pitch)));
    }

    #[test]
    fn chordify_builds_diatonic_triads() {
        let mut notes = vec![Note::new(62, 0.0, 1.0, 1.0)];
        let idx = chordify(&mut notes, &[], &c_major(), 3);
        assert_eq!(idx.len(), 3);
        let mut pitches: Vec<u8> = notes.iter().map(|n| n.pitch).collect();
        pitches.sort();
        assert_eq!(pitches, vec![62, 65, 69]);
    }

    #[test]
    fn arpeggiate_spreads_chord() {
        let mut notes = vec![Note::new(60, 0.0, 1.0, 1.0), Note::new(64, 0.0, 1.0, 1.0), Note::new(67, 0.0, 1.0, 1.0)];
        let idx = arpeggiate(&mut notes, &[], 0.25, ArpStyle::Up, 1);
        assert_eq!(idx.len(), 4);
        let pitches: Vec<u8> = idx.iter().map(|&i| notes[i].pitch).collect();
        assert_eq!(pitches, vec![60, 64, 67, 60]);
        assert!((notes[idx[3]].start - 0.75).abs() < 1e-9);
    }

    #[test]
    fn strum_and_legato() {
        let mut notes = vec![Note::new(60, 0.0, 1.0, 1.0), Note::new(64, 0.0, 1.0, 1.0), Note::new(67, 2.0, 0.5, 1.0)];
        strum(&mut notes, &[0, 1], 0.1);
        assert!((notes[1].start - 0.1).abs() < 1e-9);
        legato(&mut notes, &[], 4.0);
        assert!((notes[2].length - 2.0).abs() < 1e-9);
    }

    #[test]
    fn seed_stays_in_scale_and_range() {
        let s = Scale::new(9, ScaleKind::Minor);
        let notes = seed_melody(&s, 16.0, 0.25, 0.6, 57, 81, 42);
        assert!(!notes.is_empty());
        for n in &notes {
            assert!(s.contains(n.pitch));
            assert!((57..=81).contains(&n.pitch));
            assert!(n.start + n.length <= 16.0 + 1e-9);
        }
        // Deterministic for a given seed.
        assert_eq!(notes, seed_melody(&s, 16.0, 0.25, 0.6, 57, 81, 42));
    }

    #[test]
    fn humanize_keeps_values_sane() {
        let mut notes: Vec<Note> = (0..32).map(|i| Note::new(60, i as f64 * 0.25, 0.25, 0.8)).collect();
        humanize(&mut notes, &[], 0.02, 0.2, 7);
        assert!(notes.iter().all(|n| n.start >= 0.0 && (0.05..=1.0).contains(&n.velocity)));
    }
}

//! Musical scales: highlighting, snapping, scale-degree math and key changes.

use serde::{Deserialize, Serialize};

pub const NOTE_NAMES: [&str; 12] = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ScaleKind {
    Major,
    Minor,
    Dorian,
    Phrygian,
    Lydian,
    Mixolydian,
    Locrian,
    HarmonicMinor,
    MelodicMinor,
    PhrygianDominant,
    HungarianMinor,
    MajorPentatonic,
    MinorPentatonic,
    Blues,
    WholeTone,
    InSen,
    Hirajoshi,
    Chromatic,
}

impl ScaleKind {
    pub const ALL: [ScaleKind; 18] = [
        ScaleKind::Major,
        ScaleKind::Minor,
        ScaleKind::Dorian,
        ScaleKind::Phrygian,
        ScaleKind::Lydian,
        ScaleKind::Mixolydian,
        ScaleKind::Locrian,
        ScaleKind::HarmonicMinor,
        ScaleKind::MelodicMinor,
        ScaleKind::PhrygianDominant,
        ScaleKind::HungarianMinor,
        ScaleKind::MajorPentatonic,
        ScaleKind::MinorPentatonic,
        ScaleKind::Blues,
        ScaleKind::WholeTone,
        ScaleKind::InSen,
        ScaleKind::Hirajoshi,
        ScaleKind::Chromatic,
    ];

    pub fn name(self) -> &'static str {
        match self {
            ScaleKind::Major => "Major",
            ScaleKind::Minor => "Minor",
            ScaleKind::Dorian => "Dorian",
            ScaleKind::Phrygian => "Phrygian",
            ScaleKind::Lydian => "Lydian",
            ScaleKind::Mixolydian => "Mixolydian",
            ScaleKind::Locrian => "Locrian",
            ScaleKind::HarmonicMinor => "Harmonic Minor",
            ScaleKind::MelodicMinor => "Melodic Minor",
            ScaleKind::PhrygianDominant => "Phrygian Dominant",
            ScaleKind::HungarianMinor => "Hungarian Minor",
            ScaleKind::MajorPentatonic => "Major Pentatonic",
            ScaleKind::MinorPentatonic => "Minor Pentatonic",
            ScaleKind::Blues => "Blues",
            ScaleKind::WholeTone => "Whole Tone",
            ScaleKind::InSen => "In Sen",
            ScaleKind::Hirajoshi => "Hirajoshi",
            ScaleKind::Chromatic => "Chromatic",
        }
    }

    /// Semitone offsets from the root, ascending, starting at 0.
    pub fn intervals(self) -> &'static [u8] {
        match self {
            ScaleKind::Major => &[0, 2, 4, 5, 7, 9, 11],
            ScaleKind::Minor => &[0, 2, 3, 5, 7, 8, 10],
            ScaleKind::Dorian => &[0, 2, 3, 5, 7, 9, 10],
            ScaleKind::Phrygian => &[0, 1, 3, 5, 7, 8, 10],
            ScaleKind::Lydian => &[0, 2, 4, 6, 7, 9, 11],
            ScaleKind::Mixolydian => &[0, 2, 4, 5, 7, 9, 10],
            ScaleKind::Locrian => &[0, 1, 3, 5, 6, 8, 10],
            ScaleKind::HarmonicMinor => &[0, 2, 3, 5, 7, 8, 11],
            ScaleKind::MelodicMinor => &[0, 2, 3, 5, 7, 9, 11],
            ScaleKind::PhrygianDominant => &[0, 1, 4, 5, 7, 8, 10],
            ScaleKind::HungarianMinor => &[0, 2, 3, 6, 7, 8, 11],
            ScaleKind::MajorPentatonic => &[0, 2, 4, 7, 9],
            ScaleKind::MinorPentatonic => &[0, 3, 5, 7, 10],
            ScaleKind::Blues => &[0, 3, 5, 6, 7, 10],
            ScaleKind::WholeTone => &[0, 2, 4, 6, 8, 10],
            ScaleKind::InSen => &[0, 1, 5, 7, 10],
            ScaleKind::Hirajoshi => &[0, 2, 3, 7, 8],
            ScaleKind::Chromatic => &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
        }
    }
}

/// A root note (pitch class 0..11, C = 0) plus a scale kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Scale {
    pub root: u8,
    pub kind: ScaleKind,
}

impl Default for Scale {
    fn default() -> Self {
        Self { root: 0, kind: ScaleKind::Major }
    }
}

impl Scale {
    pub fn new(root: u8, kind: ScaleKind) -> Self {
        Self { root: root % 12, kind }
    }

    pub fn intervals(&self) -> &'static [u8] {
        self.kind.intervals()
    }

    pub fn len(&self) -> usize {
        self.intervals().len()
    }

    pub fn is_empty(&self) -> bool {
        self.intervals().is_empty()
    }

    pub fn name(&self) -> String {
        format!("{} {}", NOTE_NAMES[self.root as usize % 12], self.kind.name())
    }

    /// The note names of the scale, e.g. "A B C D E F G".
    pub fn note_names(&self) -> String {
        self.intervals().iter().map(|&i| NOTE_NAMES[(self.root + i) as usize % 12]).collect::<Vec<_>>().join(" ")
    }

    fn pitch_class(&self, pitch: i32) -> i32 {
        (pitch - self.root as i32).rem_euclid(12)
    }

    pub fn contains(&self, pitch: u8) -> bool {
        let pc = self.pitch_class(pitch as i32);
        self.intervals().iter().any(|&i| i as i32 == pc)
    }

    pub fn is_root(&self, pitch: u8) -> bool {
        self.pitch_class(pitch as i32) == 0
    }

    /// Scale degree (0-based) of an in-scale pitch.
    pub fn degree_of(&self, pitch: u8) -> Option<usize> {
        let pc = self.pitch_class(pitch as i32);
        self.intervals().iter().position(|&i| i as i32 == pc)
    }

    /// Absolute scale degree at or below `pitch`, plus the chromatic
    /// remainder above that degree (0 for in-scale notes).
    pub fn locate(&self, pitch: i32) -> (i32, i32) {
        let iv = self.intervals();
        let n = iv.len() as i32;
        let rel = pitch - self.root as i32;
        let oct = rel.div_euclid(12);
        let pc = rel.rem_euclid(12);
        let d = iv.iter().rposition(|&x| x as i32 <= pc).unwrap_or(0);
        (oct * n + d as i32, pc - iv[d] as i32)
    }

    /// Pitch of an absolute scale degree (inverse of [`Scale::locate`]).
    pub fn pitch_at(&self, abs_degree: i32) -> i32 {
        let iv = self.intervals();
        let n = iv.len() as i32;
        self.root as i32 + abs_degree.div_euclid(n) * 12 + iv[abs_degree.rem_euclid(n) as usize] as i32
    }

    /// Nearest in-scale pitch (ties go down).
    pub fn snap(&self, pitch: u8) -> u8 {
        for d in 0..12i32 {
            for p in [pitch as i32 - d, pitch as i32 + d] {
                if (0..=127).contains(&p) && self.contains(p as u8) {
                    return p as u8;
                }
            }
        }
        pitch
    }

    /// Moves a pitch by a number of scale steps (snapping it into the scale first).
    pub fn step(&self, pitch: u8, degrees: i32) -> u8 {
        let (abs, _) = self.locate(self.snap(pitch) as i32);
        self.pitch_at(abs + degrees).clamp(0, 127) as u8
    }

    /// Diatonic chord built from stacked thirds (every other scale degree).
    pub fn chord(&self, pitch: u8, size: usize) -> Vec<u8> {
        let (abs, _) = self.locate(self.snap(pitch) as i32);
        (0..size as i32).map(|k| self.pitch_at(abs + 2 * k)).filter(|p| (0..=127).contains(p)).map(|p| p as u8).collect()
    }
}

/// Where a note goes when the project changes key from `from` to `to`.
///
/// Scales with the same number of notes map degree to degree (A minor's
/// fifth becomes C major's fifth), moving the root by the smallest interval.
/// Otherwise the notes are transposed by the root change and snapped into the
/// new scale.
pub fn convert_pitch(pitch: u8, from: &Scale, to: &Scale) -> u8 {
    let mut delta = (to.root as i32 - from.root as i32).rem_euclid(12);
    if delta > 6 {
        delta -= 12;
    }
    let p = pitch as i32;
    if from.len() == to.len() {
        let n = from.len() as i32;
        let (abs, rem) = from.locate(p);
        let oct = abs.div_euclid(n);
        let d = abs.rem_euclid(n) as usize;
        let np = from.root as i32 + delta + oct * 12 + to.intervals()[d] as i32 + rem;
        np.clamp(0, 127) as u8
    } else {
        to.snap((p + delta).clamp(0, 127) as u8)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contains_and_degrees() {
        let a_minor = Scale::new(9, ScaleKind::Minor);
        assert_eq!(a_minor.note_names(), "A B C D E F G");
        assert!(a_minor.contains(60)); // C
        assert!(!a_minor.contains(61)); // C#
        assert_eq!(a_minor.degree_of(57), Some(0)); // A
        assert_eq!(a_minor.degree_of(64), Some(4)); // E
        assert!(a_minor.is_root(69));
    }

    #[test]
    fn locate_and_pitch_at_roundtrip() {
        for kind in ScaleKind::ALL {
            let s = Scale::new(4, kind);
            for p in 0..=127u8 {
                if s.contains(p) {
                    let (abs, rem) = s.locate(p as i32);
                    assert_eq!(rem, 0);
                    assert_eq!(s.pitch_at(abs), p as i32, "{kind:?} {p}");
                }
            }
        }
    }

    #[test]
    fn snap_and_step() {
        let c = Scale::new(0, ScaleKind::Major);
        assert_eq!(c.snap(61), 60); // C# -> C (tie goes down)
        assert_eq!(c.snap(63), 62); // D# -> D
        assert_eq!(c.step(60, 1), 62);
        assert_eq!(c.step(60, 7), 72);
        assert_eq!(c.step(60, -1), 59);
        assert_eq!(c.chord(62, 3), vec![62, 65, 69]); // D minor triad
        assert_eq!(c.chord(67, 4), vec![67, 71, 74, 77]); // G7
    }

    #[test]
    fn minor_to_major_keeps_degrees() {
        let a_minor = Scale::new(9, ScaleKind::Minor);
        let c_major = Scale::new(0, ScaleKind::Major);
        assert_eq!(convert_pitch(57, &a_minor, &c_major), 60); // A -> C
        assert_eq!(convert_pitch(60, &a_minor, &c_major), 64); // C (b3) -> E (3)
        assert_eq!(convert_pitch(64, &a_minor, &c_major), 67); // E (5) -> G (5)
        assert_eq!(convert_pitch(65, &a_minor, &c_major), 69); // F (b6) -> A (6)
        // Same key: nothing moves.
        for p in 0..=127u8 {
            assert_eq!(convert_pitch(p, &a_minor, &a_minor), p);
        }
    }

    #[test]
    fn different_sizes_snap_into_new_scale() {
        let c_major = Scale::new(0, ScaleKind::Major);
        let penta = Scale::new(0, ScaleKind::MinorPentatonic);
        for p in 30..100u8 {
            assert!(penta.contains(convert_pitch(p, &c_major, &penta)));
        }
    }
}

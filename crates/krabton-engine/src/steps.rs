//! A clip's notes seen as a step sequencer: a row per pitch, each with its own
//! number of steps and its own rate, so that rows can drift against each other.
//! The notes stay the clip's notes: a row shorter than the clip repeats through
//! it, and what is done to a step is done to all its repeats.

use crate::project::Note;
use serde::{Deserialize, Serialize};

/// The rates a row can step at: (name, beats).
pub const RATES: [(&str, f64); 7] = [("1/32", 0.125), ("1/16T", 1.0 / 6.0), ("1/16", 0.25), ("1/8T", 1.0 / 3.0), ("1/8", 0.5), ("1/4T", 2.0 / 3.0), ("1/4", 1.0)];
/// 1/16.
pub const DEFAULT_RATE: u8 = 2;

/// A row's settings, kept in the clip.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct StepRow {
    pub pitch: u8,
    pub steps: u16,
    pub rate: u8,
}

impl StepRow {
    /// A row as long as the clip, at this rate.
    pub fn new(pitch: u8, rate: u8, clip_length: f64) -> Self {
        Self { pitch, steps: most_steps(rate, clip_length), rate }
    }

    /// The row with no more steps than go into the clip: a step past its end
    /// would never play.
    pub fn fit(mut self, clip_length: f64) -> Self {
        self.steps = self.steps.clamp(1, most_steps(self.rate, clip_length));
        self
    }

    /// Whether it runs the whole clip through without repeating.
    pub fn fills(&self, clip_length: f64) -> bool {
        self.steps >= most_steps(self.rate, clip_length)
    }

    /// How long a step is (beats).
    pub fn step_len(&self) -> f64 {
        rate_beats(self.rate)
    }

    /// Where step `k` falls in the clip, on every repeat of the row.
    fn places(&self, k: usize, clip_length: f64) -> Vec<f64> {
        let len = self.step_len();
        let n = self.steps.max(1) as usize;
        let mut out = Vec::new();
        let mut rep = 0usize;
        loop {
            let at = (k % n + rep * n) as f64 * len;
            if at >= clip_length - 1e-9 {
                return out;
            }
            out.push(at);
            rep += 1;
        }
    }

    /// Which step a time in the clip is on (for the playhead).
    pub fn step_at(&self, beat: f64) -> usize {
        ((beat / self.step_len()).floor().max(0.0) as usize) % self.steps.max(1) as usize
    }

    fn holds(&self, n: &Note, at: f64) -> bool {
        n.pitch == self.pitch && (n.start - at).abs() < self.step_len() * 0.5 - 1e-9
    }
}

/// How many steps of this rate go into a clip.
pub fn most_steps(rate: u8, clip_length: f64) -> u16 {
    ((clip_length / rate_beats(rate) + 1e-9).floor() as u16).max(1)
}

pub fn rate_beats(rate: u8) -> f64 {
    RATES[(rate as usize).min(RATES.len() - 1)].1
}

/// The note on step `k`, where the row first plays it.
pub fn step_note<'a>(notes: &'a [Note], row: &StepRow, k: usize) -> Option<&'a Note> {
    let at = k as f64 * row.step_len();
    notes.iter().find(|n| row.holds(n, at))
}

/// Puts a note on step `k` (every repeat), or takes it off.
pub fn set_step(notes: &mut Vec<Note>, clip_length: f64, row: &StepRow, k: usize, on: bool, velocity: f32) {
    let places = row.places(k, clip_length);
    notes.retain(|n| !places.iter().any(|&at| row.holds(n, at)));
    if on {
        let len = row.step_len() * 0.9;
        for at in places {
            notes.push(Note::new(row.pitch, at, len, velocity));
        }
    }
    notes.sort_by(|a, b| a.start.total_cmp(&b.start).then(a.pitch.cmp(&b.pitch)));
}

/// Changes the note on step `k`, on every repeat. `f` gets the note and where
/// the step is.
pub fn edit_step(notes: &mut [Note], clip_length: f64, row: &StepRow, k: usize, mut f: impl FnMut(&mut Note, f64)) {
    for at in row.places(k, clip_length) {
        for n in notes.iter_mut().filter(|n| row.holds(n, at)) {
            f(n, at);
        }
    }
}

/// All the row's steps: what is on each (a copy of the note, without its place).
fn pattern(notes: &[Note], row: &StepRow) -> Vec<Option<Note>> {
    (0..row.steps as usize).map(|k| step_note(notes, row, k).cloned()).collect()
}

fn write_pattern(notes: &mut Vec<Note>, clip_length: f64, row: &StepRow, pat: &[Option<Note>]) {
    notes.retain(|n| n.pitch != row.pitch);
    for (k, step) in pat.iter().enumerate() {
        if let Some(n) = step {
            let off = n.start - k as f64 * row.step_len();
            for at in row.places(k, clip_length) {
                let mut c = n.clone();
                c.start = (at + off).max(0.0);
                notes.push(c);
            }
        }
    }
    notes.sort_by(|a, b| a.start.total_cmp(&b.start).then(a.pitch.cmp(&b.pitch)));
}

/// A row's new settings: its first pattern stays, laid out again.
pub fn change_row(notes: &mut Vec<Note>, clip_length: f64, row: &StepRow, new: &StepRow) {
    let old = pattern(notes, row);
    let pat: Vec<Option<Note>> = (0..new.steps as usize)
        .map(|k| {
            old.get(k).cloned().flatten().map(|mut n| {
                // Its place within the step stays, in the new step's size.
                let off = (n.start - k as f64 * row.step_len()) / row.step_len();
                n.start = k as f64 * new.step_len() + off * new.step_len();
                n.length = n.length / row.step_len() * new.step_len();
                n
            })
        })
        .collect();
    write_pattern(notes, clip_length, new, &pat);
}

/// Moves the row's pattern round by so many steps.
pub fn shift_row(notes: &mut Vec<Note>, clip_length: f64, row: &StepRow, by: i32) {
    let n = row.steps.max(1) as i32;
    let old = pattern(notes, row);
    let pat: Vec<Option<Note>> = (0..n)
        .map(|k| {
            let from = (k - by).rem_euclid(n);
            old[from as usize].clone().map(|mut x| {
                x.start = x.start - from as f64 * row.step_len() + k as f64 * row.step_len();
                x
            })
        })
        .collect();
    write_pattern(notes, clip_length, row, &pat);
}

/// Takes everything off the row.
pub fn clear_row(notes: &mut Vec<Note>, row: &StepRow) {
    notes.retain(|n| n.pitch != row.pitch);
}

/// `hits` evenly over `steps` (Euclid's way: the classic rhythms come out of it).
pub fn euclid(hits: usize, steps: usize) -> Vec<bool> {
    if steps == 0 {
        return Vec::new();
    }
    let hits = hits.min(steps);
    (0..steps).map(|k| hits > 0 && (k * hits) % steps < hits).collect()
}

/// Fills the row with a rhythm.
pub fn fill_row(notes: &mut Vec<Note>, clip_length: f64, row: &StepRow, on: &[bool], velocity: f32) {
    clear_row(notes, row);
    for (k, &hit) in on.iter().enumerate().take(row.steps as usize) {
        if hit {
            set_step(notes, clip_length, row, k, true, velocity);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(steps: u16, rate: u8) -> StepRow {
        StepRow { pitch: 36, steps, rate }
    }

    #[test]
    fn a_row_as_long_as_the_clip() {
        assert_eq!(StepRow::new(36, 2, 4.0).steps, 16);
        assert_eq!(StepRow::new(36, 4, 4.0).steps, 8);
        assert_eq!(StepRow::new(36, 1, 4.0).steps, 24, "triplets");
    }

    #[test]
    fn a_row_never_has_steps_past_the_end_of_the_clip() {
        assert_eq!(row(32, 2).fit(4.0).steps, 16);
        assert_eq!(row(16, 4).fit(4.0).steps, 8);
        assert_eq!(row(3, 2).fit(4.0).steps, 3, "a shorter one stays");
        assert_eq!(StepRow::new(36, 6, 0.5).steps, 1, "at least one");
        assert!(row(16, 2).fills(4.0));
        assert!(!row(3, 2).fills(4.0));
    }

    #[test]
    fn a_note_early_on_its_step_stays_early_when_the_row_moves() {
        let mut notes = vec![Note::new(36, 0.2, 0.1, 0.7)];
        let r = row(4, 2);
        shift_row(&mut notes, 1.0, &r, 1);
        assert!((notes[0].start - 0.45).abs() < 1e-9, "{}", notes[0].start);
    }

    #[test]
    fn a_step_goes_on_and_off() {
        let mut notes = Vec::new();
        let r = row(16, 2);
        set_step(&mut notes, 4.0, &r, 3, true, 0.8);
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].start, 0.75);
        assert_eq!(notes[0].pitch, 36);
        assert!(step_note(&notes, &r, 3).is_some());
        assert!(step_note(&notes, &r, 2).is_none());
        set_step(&mut notes, 4.0, &r, 3, false, 0.8);
        assert!(notes.is_empty());
    }

    #[test]
    fn a_short_row_repeats_through_the_clip_and_an_edit_reaches_every_repeat() {
        let mut notes = Vec::new();
        let r = row(3, 2);
        set_step(&mut notes, 3.0, &r, 1, true, 0.5);
        let starts: Vec<f64> = notes.iter().map(|n| n.start).collect();
        assert_eq!(starts, [0.25, 1.0, 1.75, 2.5]);
        edit_step(&mut notes, 3.0, &r, 1, |n, _| n.velocity = 1.0);
        assert!(notes.iter().all(|n| n.velocity == 1.0));
        set_step(&mut notes, 3.0, &r, 1, false, 0.5);
        assert!(notes.is_empty());
    }

    #[test]
    fn other_rows_are_left_alone() {
        let mut notes = vec![Note::new(40, 0.75, 0.2, 0.7)];
        set_step(&mut notes, 4.0, &row(16, 2), 3, true, 0.8);
        assert_eq!(notes.len(), 2);
        clear_row(&mut notes, &row(16, 2));
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].pitch, 40);
    }

    #[test]
    fn a_note_a_little_off_its_step_is_still_on_it() {
        let notes = vec![Note::new(36, 0.8, 0.2, 0.7)];
        assert!(step_note(&notes, &row(16, 2), 3).is_some());
        assert!(step_note(&notes, &row(16, 2), 4).is_none());
    }

    #[test]
    fn the_playhead_finds_the_step_of_each_row() {
        assert_eq!(row(16, 2).step_at(1.0), 4);
        assert_eq!(row(3, 2).step_at(1.0), 1, "round again");
        assert_eq!(row(5, 6).step_at(7.0), 2);
    }

    #[test]
    fn shifting_goes_round_and_keeps_what_a_note_says() {
        let mut notes = Vec::new();
        let r = row(4, 2);
        set_step(&mut notes, 1.0, &r, 3, true, 0.33);
        shift_row(&mut notes, 1.0, &r, 1);
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].start, 0.0, "off the end comes back at the start");
        assert_eq!(notes[0].velocity, 0.33);
        shift_row(&mut notes, 1.0, &r, -1);
        assert_eq!(notes[0].start, 0.75);
    }

    #[test]
    fn a_new_length_or_rate_keeps_the_pattern() {
        let mut notes = Vec::new();
        let r = row(4, 2);
        set_step(&mut notes, 2.0, &r, 1, true, 0.6);
        // Twice as slow, in the same two beats.
        let slow = StepRow { rate: 4, ..r };
        change_row(&mut notes, 2.0, &r, &slow);
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].start, 0.5);
        assert!((notes[0].length - 0.45).abs() < 1e-9, "{}", notes[0].length);
        // Shorter: what falls off goes, the rest repeats.
        let mut notes = Vec::new();
        set_step(&mut notes, 2.0, &r, 1, true, 0.6);
        set_step(&mut notes, 2.0, &r, 3, true, 0.6);
        change_row(&mut notes, 2.0, &r, &row(2, 2));
        let starts: Vec<f64> = notes.iter().map(|n| n.start).collect();
        assert_eq!(starts, [0.25, 0.75, 1.25, 1.75]);
    }

    #[test]
    fn euclid_spreads_the_hits_as_evenly_as_it_can() {
        let hits = |h, s| euclid(h, s).iter().filter(|x| **x).count();
        assert_eq!(hits(3, 8), 3);
        assert_eq!(hits(5, 16), 5);
        assert_eq!(hits(9, 4), 4, "no more than there are steps");
        assert_eq!(euclid(4, 16).iter().enumerate().filter(|(_, x)| **x).map(|(k, _)| k).collect::<Vec<_>>(), [0, 4, 8, 12]);
        assert_eq!(euclid(0, 4), [false; 4]);
        assert!(euclid(1, 0).is_empty());
    }

    #[test]
    fn filling_replaces_the_row() {
        let mut notes = Vec::new();
        let r = row(8, 2);
        set_step(&mut notes, 2.0, &r, 1, true, 0.6);
        fill_row(&mut notes, 2.0, &r, &euclid(2, 8), 0.9);
        let starts: Vec<f64> = notes.iter().map(|n| n.start).collect();
        assert_eq!(starts, [0.0, 1.0]);
    }
}

//! Note editing as in Live's MIDI Note Editor: copy, cut, paste, duplicate
//! and the moves of the arrow keys, on a note list and a selection
//! (indices). Unlike the MIDI tools, an empty selection means no notes.

use crate::project::{Note, MIN_TIDY_LENGTH};

const EPS: f64 = 1e-6;

/// The selection, in range, sorted, each index once.
fn picked(notes: &[Note], sel: &[usize]) -> Vec<usize> {
    let mut v: Vec<usize> = sel.iter().copied().filter(|&i| i < notes.len()).collect();
    v.sort_unstable();
    v.dedup();
    v
}

fn step(grid: f64) -> f64 {
    if grid > 0.0 {
        grid.max(1.0 / 64.0)
    } else {
        1.0 / 64.0
    }
}

/// The grid line at or before `b` (a hair of rounding error doesn't count).
pub fn grid_floor(b: f64, grid: f64) -> f64 {
    let g = step(grid);
    (b / g + EPS).floor() * g
}

/// The grid line at or after `b`.
pub fn grid_ceil(b: f64, grid: f64) -> f64 {
    let g = step(grid);
    (b / g - EPS).ceil() * g
}

/// The time the selected notes take, as Live's time selection around them:
/// from the grid line at or before the first start to the one at or after
/// the last end (at least one grid step).
pub fn time_range(notes: &[Note], sel: &[usize], grid: f64) -> Option<(f64, f64)> {
    let idx = picked(notes, sel);
    let first = idx.iter().map(|&i| notes[i].start).reduce(f64::min)?;
    let last_end = idx.iter().map(|&i| notes[i].end()).fold(first, f64::max);
    let start = grid_floor(first, grid).max(0.0);
    let end = grid_ceil(last_end, grid).max(start + step(grid));
    Some((start, end))
}

/// Notes put away by Copy or Cut: their starts relative to the start of the
/// time they were copied from, and how long that time is (a paste right
/// after another goes that much further on).
#[derive(Clone, Debug, PartialEq)]
pub struct NoteClip {
    pub notes: Vec<Note>,
    pub span: f64,
}

/// Copies the selected notes out of the time `range` (start, end).
pub fn copy(notes: &[Note], sel: &[usize], range: (f64, f64)) -> Option<NoteClip> {
    let idx = picked(notes, sel);
    if idx.is_empty() {
        return None;
    }
    let (from, to) = range;
    let notes = idx.iter().map(|&i| Note { start: (notes[i].start - from).max(0.0), ..notes[i].clone() }).collect();
    Some(NoteClip { notes, span: (to - from).max(1.0 / 64.0) })
}

/// Takes the selected notes out.
pub fn delete(notes: &mut Vec<Note>, sel: &[usize]) {
    let idx = picked(notes, sel);
    let mut i = 0;
    notes.retain(|_| {
        let keep = idx.binary_search(&i).is_err();
        i += 1;
        keep
    });
}

/// Where each note went after a list lost some (None: gone).
fn retain_map(notes: &mut Vec<Note>, gone: &[bool]) -> Vec<Option<usize>> {
    let mut next = 0;
    let map = gone
        .iter()
        .map(|&g| {
            if g {
                None
            } else {
                next += 1;
                Some(next - 1)
            }
        })
        .collect();
    let mut i = 0;
    notes.retain(|_| {
        let keep = !gone[i];
        i += 1;
        keep
    });
    map
}

/// The notes in `keep` win where they lie over others of their pitch, as
/// in Live when notes are pasted, duplicated or dropped onto others: one
/// that starts under a kept note goes, one that runs on into it ends where
/// it starts (or goes, when less than a 1/32 would be left). Returns where
/// each note went (None: gone).
pub fn make_room(notes: &mut Vec<Note>, keep: &[usize]) -> Vec<Option<usize>> {
    let keep = picked(notes, keep);
    let mut gone = vec![false; notes.len()];
    for i in 0..notes.len() {
        if keep.binary_search(&i).is_ok() {
            continue;
        }
        for &k in &keep {
            let (o, kn) = (&notes[i], &notes[k]);
            if o.pitch != kn.pitch {
                continue;
            }
            if o.start >= kn.start - EPS && o.start < kn.end() - EPS {
                gone[i] = true;
                break;
            }
            if o.start < kn.start && o.end() > kn.start + EPS {
                let length = kn.start - o.start;
                if length < MIN_TIDY_LENGTH {
                    gone[i] = true;
                    break;
                }
                notes[i].length = length;
            }
        }
    }
    retain_map(notes, &gone)
}

/// Puts the clip's notes in at `at`; they win over the notes they land
/// on. Returns where the pasted notes are (the new selection).
pub fn paste(notes: &mut Vec<Note>, clip: &NoteClip, at: f64) -> Vec<usize> {
    let base = notes.len();
    let at = at.max(0.0);
    notes.extend(clip.notes.iter().map(|n| Note { start: n.start + at, ..n.clone() }));
    let fresh: Vec<usize> = (base..notes.len()).collect();
    let map = make_room(notes, &fresh);
    fresh.iter().filter_map(|&i| map[i]).collect()
}

/// Ctrl+D: the selected notes again right after the time `range` they take,
/// keeping their places in it. Returns the copies (the new selection).
pub fn duplicate(notes: &mut Vec<Note>, sel: &[usize], range: (f64, f64)) -> Vec<usize> {
    match copy(notes, sel, range) {
        Some(clip) => paste(notes, &clip, range.1),
        None => Vec::new(),
    }
}

/// Moves the pitch of every selected note with `f`, but only when all of
/// them can go (a chord keeps its shape at the edge of the keyboard).
/// Returns whether they moved.
pub fn move_pitch(notes: &mut [Note], sel: &[usize], f: impl Fn(u8) -> Option<u8>) -> bool {
    let idx = picked(notes, sel);
    let Some(to) = idx.iter().map(|&i| f(notes[i].pitch)).collect::<Option<Vec<u8>>>() else { return false };
    if idx.is_empty() {
        return false;
    }
    for (&i, p) in idx.iter().zip(to) {
        notes[i].pitch = p;
    }
    true
}

/// Up and down by semitones (12: an octave), within 0..127.
pub fn transpose(notes: &mut [Note], sel: &[usize], semis: i32) -> bool {
    move_pitch(notes, sel, |p| u8::try_from(p as i32 + semis).ok().filter(|&p| p <= 127))
}

/// Up and down by rows of the editor (the pads of a drum kit): `rows` are
/// the pitches it shows, in any order. A note between rows moves by
/// semitones.
pub fn step_rows(notes: &mut [Note], sel: &[usize], rows: &[u8], steps: i32) -> bool {
    let mut rows = rows.to_vec();
    rows.sort_unstable();
    rows.dedup();
    move_pitch(notes, sel, |p| match rows.binary_search(&p) {
        Ok(r) => usize::try_from(r as i32 + steps).ok().and_then(|r| rows.get(r).copied()),
        Err(_) => u8::try_from(p as i32 + steps).ok().filter(|&p| p <= 127),
    })
}

/// Moves the selected notes later (or earlier) by `delta` beats, never
/// before the clip's start. Returns how far they went.
pub fn nudge(notes: &mut [Note], sel: &[usize], delta: f64) -> f64 {
    let idx = picked(notes, sel);
    let Some(first) = idx.iter().map(|&i| notes[i].start).reduce(f64::min) else { return 0.0 };
    let delta = delta.max(-first);
    for &i in &idx {
        notes[i].start = (notes[i].start + delta).max(0.0);
    }
    delta
}

/// Longer or shorter by `delta` beats, never shorter than `min`.
pub fn stretch(notes: &mut [Note], sel: &[usize], delta: f64, min: f64) {
    for i in picked(notes, sel) {
        notes[i].length = (notes[i].length + delta).max(min);
    }
}

/// Louder or softer by `delta` (0..1 is the whole range), at least 1 of 127.
pub fn change_velocity(notes: &mut [Note], sel: &[usize], delta: f32) {
    for i in picked(notes, sel) {
        notes[i].velocity = (notes[i].velocity + delta).clamp(1.0 / 127.0, 1.0);
    }
}

/// Where Ctrl+V puts notes, as in Live: right after the selected notes'
/// time when notes are selected, else at the insert marker (set by clicking
/// an empty spot), else at the clip's start.
pub fn paste_position(selection: Option<(f64, f64)>, marker: Option<f64>) -> f64 {
    match (selection, marker) {
        (Some((_, end)), _) => end,
        (None, Some(at)) => at.max(0.0),
        (None, None) => 0.0,
    }
}

/// Notes on the system clipboard (so another Krabton window can paste
/// them too): this line, then the notes as JSON.
pub const CLIPBOARD_TAG: &str = "Krabton notes";

impl NoteClip {
    /// The clip as text for the system clipboard.
    pub fn to_text(&self) -> String {
        let body = serde_json::json!({ "span": self.span, "notes": self.notes });
        format!("{CLIPBOARD_TAG}\n{body}")
    }

    /// Notes from the system clipboard, if it holds some.
    pub fn from_text(text: &str) -> Option<Self> {
        let body = text.strip_prefix(CLIPBOARD_TAG)?.trim_start();
        let v: serde_json::Value = serde_json::from_str(body).ok()?;
        let span = v.get("span")?.as_f64()?;
        let notes: Vec<Note> = serde_json::from_value(v.get("notes")?.clone()).ok()?;
        (span > 0.0).then_some(Self { notes, span })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn n(pitch: u8, start: f64, length: f64) -> Note {
        Note::new(pitch, start, length, 0.8)
    }

    fn starts(notes: &[Note]) -> Vec<(u8, f64)> {
        let mut v: Vec<(u8, f64)> = notes.iter().map(|n| (n.pitch, n.start)).collect();
        v.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
        v
    }

    #[test]
    fn time_range_rounds_out_to_the_grid() {
        let notes = vec![n(60, 0.1, 0.2), n(64, 1.0, 0.5)];
        assert_eq!(time_range(&notes, &[0, 1], 0.25), Some((0.0, 1.5)));
        // On the grid already: stays.
        assert_eq!(time_range(&notes, &[1], 0.25), Some((1.0, 1.5)));
        // A tiny note still takes a whole grid step.
        let tiny = vec![n(60, 2.0, 0.01)];
        assert_eq!(time_range(&tiny, &[0], 0.5), Some((2.0, 2.5)));
        assert_eq!(time_range(&notes, &[], 0.25), None);
        // A riff that ends with a rest ends where its last note does (as Live).
        let riff = vec![n(60, 0.0, 0.5), n(62, 1.0, 0.5), n(64, 3.0, 0.5)];
        assert_eq!(time_range(&riff, &[0, 1, 2], 0.25), Some((0.0, 3.5)));
    }

    #[test]
    fn duplicate_lands_right_after_and_keeps_going() {
        let mut notes = vec![n(60, 0.0, 0.25), n(64, 0.5, 0.5)];
        let range = time_range(&notes, &[0, 1], 0.25).unwrap();
        let sel = duplicate(&mut notes, &[0, 1], range);
        assert_eq!(starts(&notes), vec![(60, 0.0), (64, 0.5), (60, 1.0), (64, 1.5)]);
        assert_eq!(sel.len(), 2);
        assert!(sel.iter().all(|&i| notes[i].start >= 1.0));
        // Again from the copies: the third lot after the second.
        let range = time_range(&notes, &sel, 0.25).unwrap();
        assert_eq!(range, (1.0, 2.0));
        let sel = duplicate(&mut notes, &sel, range);
        assert_eq!(notes.len(), 6);
        assert!(sel.iter().all(|&i| notes[i].start >= 2.0));
    }

    #[test]
    fn duplicate_by_an_off_grid_span_rounds_up_to_the_grid() {
        // Ends at 0.6: with a 1/16 grid the copy starts at 0.75.
        let mut notes = vec![n(60, 0.0, 0.6)];
        let range = time_range(&notes, &[0], 0.25).unwrap();
        duplicate(&mut notes, &[0], range);
        assert_eq!(starts(&notes), vec![(60, 0.0), (60, 0.75)]);
        // A time selection drawn wider (a marquee over a whole bar) steps a bar.
        let mut notes = vec![n(60, 0.0, 0.6)];
        duplicate(&mut notes, &[0], (0.0, 4.0));
        assert_eq!(starts(&notes), vec![(60, 0.0), (60, 4.0)]);
    }

    #[test]
    fn copy_keeps_timing_and_pitch_relative_to_the_range() {
        let notes = vec![n(60, 1.1, 0.25), n(67, 1.5, 0.5), n(50, 3.0, 1.0)];
        let clip = copy(&notes, &[1, 0], (1.0, 2.0)).unwrap();
        assert_eq!(clip.span, 1.0);
        assert_eq!(starts(&clip.notes).len(), 2);
        assert!((clip.notes[0].start - 0.1).abs() < 1e-9 && clip.notes[0].pitch == 60);
        assert!((clip.notes[1].start - 0.5).abs() < 1e-9 && clip.notes[1].pitch == 67);
        assert!(copy(&notes, &[], (0.0, 1.0)).is_none());
    }

    #[test]
    fn paste_goes_into_another_clip_at_its_marker() {
        let source = vec![n(60, 4.25, 0.25), n(64, 4.75, 0.25)];
        let clip = copy(&source, &[0, 1], time_range(&source, &[0, 1], 0.25).unwrap()).unwrap();
        let mut other = vec![n(40, 0.0, 1.0)];
        let sel = paste(&mut other, &clip, 2.0);
        assert_eq!(starts(&other), vec![(40, 0.0), (60, 2.0), (64, 2.5)]);
        assert_eq!(sel, vec![1, 2]);
        // Pasted past the clip's end they stay there (the loop isn't grown).
        let sel = paste(&mut other, &clip, 9.0);
        assert!(sel.iter().all(|&i| other[i].start >= 9.0));
    }

    #[test]
    fn pasted_notes_win_over_the_ones_they_land_on() {
        // A note under the paste goes, one that rings into it is cut short,
        // other pitches stay.
        let mut notes = vec![n(60, 1.0, 0.25), n(60, 0.0, 2.0), n(62, 1.0, 1.0)];
        let clip = NoteClip { notes: vec![n(60, 0.0, 0.5)], span: 0.5 };
        let sel = paste(&mut notes, &clip, 1.0);
        assert_eq!(notes.len(), 3);
        assert_eq!(sel.len(), 1);
        assert_eq!(notes[sel[0]].start, 1.0);
        let long = notes.iter().find(|n| n.pitch == 60 && n.start == 0.0).unwrap();
        assert_eq!(long.length, 1.0);
        assert!(notes.iter().any(|n| n.pitch == 62));
        // Pasted straight onto copies of themselves: just one of each.
        let mut notes = vec![n(60, 0.0, 0.5), n(64, 0.0, 0.5)];
        let clip = copy(&notes, &[0, 1], (0.0, 0.5)).unwrap();
        let sel = paste(&mut notes, &clip, 0.0);
        assert_eq!(notes.len(), 2);
        assert_eq!(sel, vec![0, 1]);
    }

    #[test]
    fn make_room_reports_where_notes_went() {
        let mut notes = vec![n(60, 0.0, 1.0), n(60, 0.5, 0.25), n(61, 0.0, 1.0), n(60, 0.49, 1.0)];
        let map = make_room(&mut notes, &[3]);
        // The first runs into the kept one (shortened to 0.49), the second starts under it.
        assert_eq!(map, vec![Some(0), None, Some(1), Some(2)]);
        assert!((notes[0].length - 0.49).abs() < 1e-9);
        // Shortened to less than a 1/32 it goes.
        let mut notes = vec![n(60, 0.0, 1.0), n(60, 0.01, 1.0)];
        let map = make_room(&mut notes, &[1]);
        assert_eq!(map, vec![None, Some(0)]);
    }

    #[test]
    fn delete_takes_only_the_selected() {
        let mut notes = vec![n(60, 0.0, 1.0), n(62, 1.0, 1.0), n(64, 2.0, 1.0)];
        delete(&mut notes, &[2, 0, 9]);
        assert_eq!(starts(&notes), vec![(62, 1.0)]);
    }

    #[test]
    fn octaves_stop_at_the_ends_of_the_keyboard() {
        let mut notes = vec![n(120, 0.0, 1.0), n(100, 0.0, 1.0), n(5, 1.0, 1.0)];
        // 120 + 12 > 127: the chord doesn't move at all (it keeps its shape).
        assert!(!transpose(&mut notes, &[0, 1], 12));
        assert_eq!((notes[0].pitch, notes[1].pitch), (120, 100));
        assert!(transpose(&mut notes, &[0, 1], 7));
        assert_eq!((notes[0].pitch, notes[1].pitch), (127, 107));
        assert!(!transpose(&mut notes, &[2], -12));
        assert!(transpose(&mut notes, &[2], -5));
        assert_eq!(notes[2].pitch, 0);
        // Unselected notes stay where they are.
        assert!(transpose(&mut notes, &[1], -1));
        assert_eq!(notes[0].pitch, 127);
    }

    #[test]
    fn drum_rows_step_from_pad_to_pad() {
        let rows = [36, 38, 42, 46];
        let mut notes = vec![n(36, 0.0, 0.25), n(42, 0.5, 0.25)];
        assert!(step_rows(&mut notes, &[0, 1], &rows, 1));
        assert_eq!((notes[0].pitch, notes[1].pitch), (38, 46));
        // The top pad can't go higher: none of them move.
        assert!(!step_rows(&mut notes, &[0, 1], &rows, 1));
        assert!(step_rows(&mut notes, &[0], &rows, -1));
        assert_eq!(notes[0].pitch, 36);
    }

    #[test]
    fn nudge_never_goes_before_the_start() {
        let mut notes = vec![n(60, 0.25, 1.0), n(62, 1.0, 1.0), n(64, 0.0, 1.0)];
        assert_eq!(nudge(&mut notes, &[0, 1], -0.5), -0.25);
        assert_eq!((notes[0].start, notes[1].start), (0.0, 0.75));
        assert_eq!(notes[2].start, 0.0);
        assert_eq!(nudge(&mut notes, &[0], 0.25), 0.25);
        stretch(&mut notes, &[0], -2.0, 0.0625);
        assert_eq!(notes[0].length, 0.0625);
        change_velocity(&mut notes, &[0], 1.0);
        assert_eq!(notes[0].velocity, 1.0);
        change_velocity(&mut notes, &[0], -2.0);
        assert!((notes[0].velocity - 1.0 / 127.0).abs() < 1e-6);
    }

    #[test]
    fn paste_lands_after_the_selection_or_at_the_marker() {
        assert_eq!(paste_position(Some((1.0, 2.0)), Some(8.0)), 2.0);
        assert_eq!(paste_position(None, Some(8.0)), 8.0);
        assert_eq!(paste_position(None, None), 0.0);
    }

    #[test]
    fn clipboard_text_round_trips() {
        let mut a = n(61, 0.5, 0.25);
        a.chance = 0.5;
        let clip = NoteClip { notes: vec![n(60, 0.0, 1.0), a], span: 2.0 };
        assert_eq!(NoteClip::from_text(&clip.to_text()), Some(clip));
        assert_eq!(NoteClip::from_text("just some text"), None);
    }
}

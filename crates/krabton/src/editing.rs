//! Editing commands shared by the views: the clipboard (clips and notes),
//! duplicate, and the arrow keys (nudge and transpose notes, move clips,
//! move the selection).

use crate::app::{App, ClipSel, DetailTab, TrackSel, View};
use egui::{Key, Modifiers};
use krabton_engine::{ArrangementClip, Clip, Note, TrackKind};

/// What Ctrl+C put away.
#[derive(Clone)]
pub enum Clipboard {
    Clip(Clip),
    /// Notes with their starts relative to the first one, and the span
    /// (length to step by when pasting repeatedly).
    Notes { notes: Vec<Note>, span: f64 },
}

/// The distance to step by after a group of notes: a whole bar for
/// riffs, a beat for short figures, at least the grid.
fn note_span(first: f64, last_end: f64, grid: f64, bar: f64) -> f64 {
    let raw = (last_end - first).max(grid);
    let unit = if raw > bar * 0.5 {
        bar
    } else if raw > 0.5 {
        1.0
    } else {
        grid.max(1.0 / 64.0)
    };
    (raw / unit - 1e-6).ceil().max(1.0) * unit
}

impl App {
    /// The MIDI clip open in the clip view, if any.
    fn open_midi_clip(&self) -> Option<ClipSel> {
        if self.detail != DetailTab::Clip {
            return None;
        }
        let sel = self.sel_clip?;
        self.clip_ref(sel).filter(|c| c.is_midi()).map(|_| sel)
    }

    /// True when keys should act on the selected notes.
    pub(crate) fn notes_focus(&self) -> bool {
        self.open_midi_clip().is_some() && !self.piano.selected.is_empty()
    }

    fn selected_notes(&self, sel: ClipSel) -> Vec<Note> {
        let notes = self.clip_ref(sel).map(|c| c.notes().to_vec()).unwrap_or_default();
        let mut idx: Vec<usize> = self.piano.selected.iter().copied().filter(|&i| i < notes.len()).collect();
        idx.sort_unstable();
        idx.into_iter().map(|i| notes[i].clone()).collect()
    }

    pub(crate) fn edit_copy(&mut self) {
        if self.notes_focus() {
            let Some(sel) = self.open_midi_clip() else { return };
            let notes = self.selected_notes(sel);
            let first = notes.iter().map(|n| n.start).fold(f64::MAX, f64::min);
            let last_end = notes.iter().map(|n| n.end()).fold(0.0, f64::max);
            let span = note_span(first, last_end, self.piano.grid, self.project.beats_per_bar_f());
            let rel: Vec<Note> = notes.iter().map(|n| Note { start: n.start - first, ..n.clone() }).collect();
            let count = rel.len();
            self.clipboard = Some(Clipboard::Notes { notes: rel, span });
            self.piano.paste_at = Some(first + span);
            self.toast(format!("{count} note(s) copied"));
        } else if let Some(sel) = self.sel_clip {
            if let Some(c) = self.clip_ref(sel).cloned() {
                self.clipboard = Some(Clipboard::Clip(c));
                self.toast("Clip copied");
            }
        }
    }

    pub(crate) fn edit_cut(&mut self) {
        if self.notes_focus() {
            // Pasting right away puts the notes back where they were.
            let first = self.open_midi_clip().map(|s| self.selected_notes(s)).and_then(|v| v.iter().map(|n| n.start).reduce(f64::min));
            self.edit_copy();
            self.piano.paste_at = first;
            self.piano_delete_selected();
        } else {
            self.edit_copy();
            if let Some(sel) = self.sel_clip {
                self.delete_clip(sel);
            }
        }
    }

    pub(crate) fn edit_paste(&mut self) {
        match self.clipboard.clone() {
            Some(Clipboard::Notes { notes, span }) => {
                let Some(sel) = self.open_midi_clip() else {
                    self.toast("Open a MIDI clip to paste notes");
                    return;
                };
                let at = self.piano.paste_at.unwrap_or(0.0).max(0.0);
                self.paste_notes(sel, &notes, at);
                self.piano.paste_at = Some(at + span);
            }
            Some(Clipboard::Clip(clip)) => self.paste_clip(clip),
            None => {}
        }
    }

    /// Ctrl+D: notes right after themselves, or the clip.
    pub(crate) fn edit_duplicate(&mut self) {
        if self.notes_focus() {
            let Some(sel) = self.open_midi_clip() else { return };
            let notes = self.selected_notes(sel);
            let first = notes.iter().map(|n| n.start).fold(f64::MAX, f64::min);
            let last_end = notes.iter().map(|n| n.end()).fold(0.0, f64::max);
            let span = note_span(first, last_end, self.piano.grid, self.project.beats_per_bar_f());
            let rel: Vec<Note> = notes.iter().map(|n| Note { start: n.start - first, ..n.clone() }).collect();
            self.paste_notes(sel, &rel, first + span);
            self.piano.paste_at = Some(first + 2.0 * span);
        } else if let Some(sel) = self.sel_clip {
            self.duplicate_clip(sel);
        }
    }

    /// Adds notes (relative starts) at `at`, selects them and grows the
    /// clip when they reach past its end.
    fn paste_notes(&mut self, sel: ClipSel, notes: &[Note], at: f64) {
        let bar = self.project.beats_per_bar_f();
        let Some(clip) = self.clip_mut(sel) else { return };
        let old_len = clip.length;
        let Some(v) = clip.notes_mut() else { return };
        let base = v.len();
        let mut end: f64 = 0.0;
        let mut last_start: f64 = 0.0;
        for n in notes {
            let n = Note { start: n.start + at, ..n.clone() };
            end = end.max(n.end());
            last_start = last_start.max(n.start);
            v.push(n);
        }
        self.piano.selected = (base..base + notes.len()).collect();
        // Grow the clip when notes start past its end (a note that only
        // rings past the end is fine, like a tail).
        if last_start >= old_len - 1e-9 {
            let new_len = (end / bar - 1e-9).ceil() * bar;
            self.extend_clip(sel, new_len);
            self.toast(format!("Clip extended to {} bars", (new_len / bar).round()));
        }
    }

    /// Makes a clip (and its arrangement region, when it showed the whole
    /// clip) at least `len` beats long.
    fn extend_clip(&mut self, sel: ClipSel, len: f64) {
        match sel {
            ClipSel::Slot { .. } => {
                if let Some(c) = self.clip_mut(sel) {
                    c.length = c.length.max(len);
                }
            }
            ClipSel::Arr { track, id } => {
                if let Some(ac) = self.project.tracks.get_mut(track).and_then(|t| t.arrangement.iter_mut().find(|a| a.id == id)) {
                    let whole = (ac.length - ac.clip.length).abs() < 1e-6 && ac.offset == 0.0;
                    ac.clip.length = ac.clip.length.max(len);
                    if whole {
                        ac.length = ac.clip.length;
                    }
                }
            }
        }
    }

    /// Pastes a clip where the user is looking: the arrangement at the
    /// insert position, or the selected session slot.
    fn paste_clip(&mut self, clip: Clip) {
        let track = match (self.view, self.sel_clip, self.sel_track) {
            (View::Arrangement, Some(ClipSel::Arr { track, .. }), _) => Some(track),
            (_, _, TrackSel::Track(t)) => Some(t),
            (_, Some(s), _) => Some(s.track()),
            _ => None,
        };
        let Some(track) = track.filter(|&t| t < self.project.tracks.len()) else {
            self.toast("Select a track to paste the clip");
            return;
        };
        if (self.project.tracks[track].kind == TrackKind::Midi) != clip.is_midi() {
            self.toast("That clip doesn't fit this track type");
            return;
        }
        let c = self.project.fresh_clip(&clip);
        match self.view {
            View::Arrangement => {
                let start = self.engine.status.song_pos.load().max(0.0);
                let id = self.project.alloc_id();
                let length = c.length;
                self.project.tracks[track].arrangement.push(ArrangementClip { id, start, length, offset: 0.0, clip: c });
                self.select_clip(ClipSel::Arr { track, id });
                // The next paste goes right after this one.
                self.cmd(krabton_engine::Command::SetPosition(start + length));
            }
            View::Session => {
                let scene = match self.sel_clip {
                    Some(ClipSel::Slot { track: t, scene }) if t == track => scene,
                    _ => self.sel_scene,
                };
                if scene >= self.project.scenes.len() {
                    return;
                }
                self.project.tracks[track].slots[scene] = Some(c);
                self.select_clip(ClipSel::Slot { track, scene });
            }
        }
    }

    /// Arrow keys. Returns true when they were used.
    pub(crate) fn edit_arrow(&mut self, key: Key, m: Modifiers) -> bool {
        let (dx, dy): (i32, i32) = match key {
            Key::ArrowLeft => (-1, 0),
            Key::ArrowRight => (1, 0),
            Key::ArrowUp => (0, 1),
            Key::ArrowDown => (0, -1),
            _ => return false,
        };
        if self.notes_focus() {
            self.arrow_notes(dx, dy, m);
            return true;
        }
        if let (View::Arrangement, Some(ClipSel::Arr { track, id })) = (self.view, self.sel_clip) {
            self.arrow_arr_clip(track, id, dx, -dy, m);
            return true;
        }
        match self.view {
            View::Session => self.arrow_session(dx, -dy),
            View::Arrangement => {
                if dy != 0 {
                    self.step_track(-dy);
                } else {
                    let g = if self.arr.grid > 0.0 { self.arr.grid } else { 1.0 };
                    let g = if m.shift { g * 0.25 } else { g };
                    let pos = self.engine.status.song_pos.load();
                    let next = ((pos / g).round() + dx as f64) * g;
                    self.cmd(krabton_engine::Command::SetPosition(next.max(0.0)));
                }
            }
        }
        true
    }

    fn arrow_notes(&mut self, dx: i32, dy: i32, m: Modifiers) {
        let Some(sel) = self.open_midi_clip() else { return };
        let grid = self.piano.grid;
        let selected = self.piano.selected.clone();
        let folded = self.project.scale_mode && self.piano.fold;
        let scale = self.project.scale;
        let mut grow_to = 0.0f64;
        let Some(notes) = self.clip_mut(sel).and_then(|c| c.notes_mut()) else { return };
        if dx != 0 {
            if m.shift {
                // Shift+Left/Right: shorter or longer.
                for (i, n) in notes.iter_mut().enumerate() {
                    if selected.contains(&i) {
                        n.length = (n.length + dx as f64 * grid).max(grid.min(1.0 / 16.0));
                    }
                }
            } else {
                // Move by the grid (Alt: a quarter of it); never before the clip start.
                let step = if m.alt { grid * 0.25 } else { grid };
                let first = notes.iter().enumerate().filter(|(i, _)| selected.contains(i)).map(|(_, n)| n.start).fold(f64::MAX, f64::min);
                let delta = (dx as f64 * step).max(-first);
                for (i, n) in notes.iter_mut().enumerate() {
                    if selected.contains(&i) {
                        n.start = (n.start + delta).max(0.0);
                        // Only a start past the end grows the clip.
                        grow_to = grow_to.max(n.start + grid.min(n.length));
                    }
                }
            }
        } else if dy != 0 {
            for (i, n) in notes.iter_mut().enumerate() {
                if !selected.contains(&i) {
                    continue;
                }
                n.pitch = if m.shift {
                    (n.pitch as i32 + 12 * dy).clamp(0, 127) as u8
                } else if folded {
                    scale.step(n.pitch, dy)
                } else {
                    (n.pitch as i32 + dy).clamp(0, 127) as u8
                };
            }
        }
        let len = self.clip_ref(sel).map(|c| c.length).unwrap_or(0.0);
        if grow_to > len + grid * 0.5 {
            let bar = self.project.beats_per_bar_f();
            self.extend_clip(sel, (grow_to / bar - 1e-9).ceil() * bar);
        }
        // Let the moved notes be heard (with Preview on).
        if dy != 0 && self.prefs.note_preview {
            let track = sel.track();
            if let Some(top) = self.selected_notes(sel).iter().map(|n| n.pitch).max() {
                let ids = self.preview_note_on(track, top, 0.7);
                self.note_off(top, &ids);
            }
        }
    }

    fn arrow_arr_clip(&mut self, track: usize, id: u64, dx: i32, dtrack: i32, m: Modifiers) {
        if dx != 0 {
            let g = if self.arr.grid > 0.0 { self.arr.grid } else { 1.0 };
            let g = if m.shift { g * 0.25 } else { g };
            if let Some(ac) = self.project.tracks.get_mut(track).and_then(|t| t.arrangement.iter_mut().find(|a| a.id == id)) {
                ac.start = (ac.start + dx as f64 * g).max(0.0);
            }
            return;
        }
        // Up/Down: to the next track of the same kind.
        let kind = self.project.tracks[track].kind;
        let mut t = track as i32;
        loop {
            t += dtrack;
            if t < 0 || t as usize >= self.project.tracks.len() {
                return;
            }
            if self.project.tracks[t as usize].kind == kind {
                break;
            }
        }
        let to = t as usize;
        if let Some(pos) = self.project.tracks[track].arrangement.iter().position(|a| a.id == id) {
            let ac = self.project.tracks[track].arrangement.remove(pos);
            self.project.tracks[to].arrangement.push(ac);
            self.select_clip(ClipSel::Arr { track: to, id });
        }
    }

    fn step_track(&mut self, dt: i32) {
        let n = self.project.tracks.len();
        if n == 0 {
            return;
        }
        let cur = match self.sel_track {
            TrackSel::Track(t) => t as i32,
            _ => 0,
        };
        let t = (cur + dt).clamp(0, n as i32 - 1) as usize;
        self.sel_track = TrackSel::Track(t);
        if !matches!(self.sel_clip, Some(ClipSel::Arr { track, .. }) if track == t) {
            self.sel_clip = None;
        }
    }

    /// Moves the session selection; the clip in the new slot gets selected.
    fn arrow_session(&mut self, dx: i32, dscene: i32) {
        let ns = self.project.scenes.len();
        let nt = self.project.tracks.len();
        if ns == 0 {
            return;
        }
        self.sel_scene = (self.sel_scene as i32 + dscene).clamp(0, ns as i32 - 1) as usize;
        if dx != 0 && nt > 0 {
            self.step_track(dx);
        }
        if let TrackSel::Track(t) = self.sel_track {
            let s = self.sel_scene;
            if matches!(self.project.tracks.get(t).and_then(|tr| tr.slots.get(s)), Some(Some(_))) {
                self.sel_clip = Some(ClipSel::Slot { track: t, scene: s });
            } else if matches!(self.sel_clip, Some(ClipSel::Slot { .. })) {
                self.sel_clip = None;
            }
        }
    }

    /// Copies an arrangement clip into a session slot.
    pub(crate) fn arr_clip_to_slot(&mut self, from: (usize, u64), to: (usize, usize)) {
        let Some(ac) = self.project.tracks.get(from.0).and_then(|t| t.arrangement.iter().find(|a| a.id == from.1)).cloned() else { return };
        let (tt, ts) = to;
        let Some(target) = self.project.tracks.get(tt) else { return };
        if (target.kind == TrackKind::Midi) != ac.clip.is_midi() || ts >= target.slots.len() {
            self.toast("That clip doesn't fit this track type");
            return;
        }
        let c = self.project.fresh_clip(&ac.clip);
        self.project.tracks[tt].slots[ts] = Some(c);
        self.select_clip(ClipSel::Slot { track: tt, scene: ts });
        self.toast("Copied to the session");
    }
}

#[cfg(test)]
mod tests {
    use super::note_span;

    #[test]
    fn spans_round_to_musical_units() {
        assert_eq!(note_span(0.0, 3.75, 0.25, 4.0), 4.0);
        assert_eq!(note_span(4.5, 5.25, 0.25, 4.0), 1.0);
        assert_eq!(note_span(0.0, 0.25, 0.25, 4.0), 0.25);
        assert_eq!(note_span(0.0, 6.0, 0.25, 4.0), 8.0);
    }
}

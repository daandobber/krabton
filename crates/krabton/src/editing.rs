//! Editing commands shared by the views: the clipboard (clips and notes),
//! duplicate, and the arrow keys (nudge and transpose notes, move clips,
//! move the selection).

use crate::app::{App, ClipSel, DetailTab, TrackSel, View};
use egui::{Key, Modifiers};
use krabton_engine::note_edit::{self, NoteClip};
use krabton_engine::{ArrangementClip, Clip, Note, TrackKind};

impl App {
    /// The MIDI clip open in the clip view, if any.
    pub(crate) fn open_midi_clip(&self) -> Option<ClipSel> {
        if self.detail != DetailTab::Clip {
            return None;
        }
        let sel = self.sel_clip?;
        self.clip_ref(sel).filter(|c| c.is_midi()).map(|_| sel)
    }

    /// True when the note editor has the keys: a MIDI clip open in the clip
    /// view, and the clip view clicked last (Live's focus: a click in the
    /// session or the arrangement gives the keys back to the clips there).
    pub(crate) fn notes_focus(&self) -> bool {
        self.piano.focused && self.open_midi_clip().is_some()
    }

    /// True when keys should act on the selected notes.
    pub(crate) fn notes_picked(&self) -> bool {
        self.notes_focus() && !self.piano.selected.is_empty()
    }

    /// The selected notes' indices, sorted.
    fn note_selection(&self) -> Vec<usize> {
        let mut idx: Vec<usize> = self.piano.selected.iter().copied().collect();
        idx.sort_unstable();
        idx
    }

    fn selected_notes(&self, sel: ClipSel) -> Vec<Note> {
        let notes = self.clip_ref(sel).map(|c| c.notes().to_vec()).unwrap_or_default();
        self.note_selection().into_iter().filter_map(|i| notes.get(i).cloned()).collect()
    }

    /// The time the selected notes take, Live's time selection: the one a
    /// box drew around them, else from the grid line before the first to
    /// the one after the last.
    pub(crate) fn notes_range(&self, sel: ClipSel) -> Option<(f64, f64)> {
        let notes = self.clip_ref(sel)?.notes();
        let idx = self.note_selection();
        let fit = note_edit::time_range(notes, &idx, self.piano.grid)?;
        let first = idx.iter().filter_map(|&i| notes.get(i)).map(|n| n.start).fold(f64::MAX, f64::min);
        let last = idx.iter().filter_map(|&i| notes.get(i)).map(|n| n.start).fold(0.0, f64::max);
        match self.piano.time_sel {
            Some((a, b)) if a <= first + 1e-9 && b > last + 1e-9 => Some((a, b)),
            _ => Some(fit),
        }
    }

    /// Ctrl+C in the note editor. The time selection stays, so a paste
    /// right after goes just after it (as in Live).
    pub(crate) fn notes_copy(&mut self) -> bool {
        let Some(sel) = self.open_midi_clip() else { return false };
        let Some(range) = self.notes_range(sel) else { return false };
        let notes = self.clip_ref(sel).map(|c| c.notes().to_vec()).unwrap_or_default();
        let Some(clip) = note_edit::copy(&notes, &self.note_selection(), range) else { return false };
        let count = clip.notes.len();
        self.os_clipboard = Some(clip.to_text());
        self.note_clipboard = Some(clip);
        self.notes_copied_last = true;
        self.piano.time_sel = Some(range);
        self.toast(format!("{count} note(s) copied"));
        true
    }

    /// Ctrl+X: copies and takes them out; a paste puts them back.
    pub(crate) fn notes_cut(&mut self) {
        if self.notes_copy() {
            self.piano_delete_selected();
        }
    }

    /// Ctrl+V into the open clip: after the selected notes' time, at the
    /// insert marker or at the start. The pasted notes are selected, so the
    /// next paste goes after them. The loop stays as it is (as in Live):
    /// notes past its end wait there in the dark.
    pub(crate) fn notes_paste(&mut self, clip: NoteClip) {
        let Some(sel) = self.open_midi_clip() else {
            self.toast("Open a MIDI clip to paste notes");
            return;
        };
        let selection = if self.piano.selected.is_empty() { None } else { self.notes_range(sel) };
        let at = note_edit::paste_position(selection, self.piano.paste_at);
        let Some(notes) = self.clip_mut(sel).and_then(|c| c.notes_mut()) else { return };
        let pasted = note_edit::paste(notes, &clip, at);
        let count = pasted.len();
        self.piano.selected = pasted.into_iter().collect();
        self.piano.time_sel = Some((at, at + clip.span));
        self.piano.paste_at = Some(at + clip.span);
        self.piano.focused = true;
        self.toast(format!("{count} note(s) pasted"));
    }

    /// Ctrl+D: the selected notes again right after their time, selected,
    /// so pressing it again goes on.
    pub(crate) fn notes_duplicate(&mut self) {
        let Some(sel) = self.open_midi_clip() else { return };
        let Some((from, to)) = self.notes_range(sel) else { return };
        let idx = self.note_selection();
        let Some(notes) = self.clip_mut(sel).and_then(|c| c.notes_mut()) else { return };
        let copies = note_edit::duplicate(notes, &idx, (from, to));
        self.piano.selected = copies.into_iter().collect();
        self.piano.time_sel = Some((to, to + (to - from)));
        self.piano.paste_at = Some(to + (to - from));
    }

    /// The selected notes win over the ones they lie on (after a move), and
    /// the selection follows the notes that stay.
    pub(crate) fn notes_make_room(&mut self, sel: ClipSel) {
        let idx = self.note_selection();
        let Some(notes) = self.clip_mut(sel).and_then(|c| c.notes_mut()) else { return };
        let map = note_edit::make_room(notes, &idx);
        self.piano.selected = idx.iter().filter_map(|&i| map.get(i).copied().flatten()).collect();
    }

    pub(crate) fn edit_copy(&mut self) {
        if self.notes_focus() {
            // Nothing selected: nothing to copy (as in Live).
            self.notes_copy();
        } else if let Some(sel) = self.sel_clip {
            if let Some(c) = self.clip_ref(sel).cloned() {
                self.os_clipboard = Some(format!("Krabton clip: {}", c.name));
                self.clipboard = Some(c);
                self.notes_copied_last = false;
                self.toast("Clip copied");
            }
        }
    }

    pub(crate) fn edit_cut(&mut self) {
        if self.notes_focus() {
            self.notes_cut();
        } else {
            self.edit_copy();
            if let Some(sel) = self.sel_clip {
                self.delete_clip(sel);
            }
        }
    }

    /// Ctrl+V. `text` is what the system clipboard held (notes from another
    /// Krabton window come that way). In the note editor it pastes notes; in
    /// the main view what was copied last: notes into the open clip, or the
    /// clip.
    pub(crate) fn edit_paste(&mut self, text: &str) {
        let outside = NoteClip::from_text(text);
        let notes_last = outside.is_some() || self.notes_copied_last;
        let notes = outside.or_else(|| self.note_clipboard.clone());
        let into_notes = self.notes_focus() || self.clipboard.is_none() || (notes_last && self.open_midi_clip().is_some());
        if into_notes {
            match notes {
                Some(clip) => self.notes_paste(clip),
                None if self.notes_focus() => self.toast("No notes copied yet"),
                None => {}
            }
        } else if let Some(clip) = self.clipboard.clone() {
            self.paste_clip(clip);
        }
    }

    /// Ctrl+D: notes right after themselves, or the clip.
    pub(crate) fn edit_duplicate(&mut self) {
        if self.notes_focus() {
            self.notes_duplicate();
        } else if let Some(sel) = self.sel_clip {
            self.duplicate_clip(sel);
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
                self.project.tracks[track].arrangement.push(ArrangementClip::new(id, start, length, c));
                self.select_clip(ClipSel::Arr { track, id });
                // The next paste goes right after this one.
                self.cmd(krabton_engine::Command::SetPosition(start + length));
            }
            // Pasting in the Rack (or the Tape) goes into the session, like in Session.
            View::Session | View::Studio | View::Tape | View::Touch => {
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
        if self.notes_picked() {
            self.arrow_notes(dx, dy, m);
            return true;
        }
        if self.notes_focus() {
            // No notes selected: Left and Right move the insert marker.
            if dx != 0 {
                let g = self.piano.grid;
                let at = note_edit::grid_floor(self.piano.paste_at.unwrap_or(0.0), g) + dx as f64 * g;
                self.piano.paste_at = Some(at.max(0.0));
            }
            return true;
        }
        if let (View::Arrangement, Some(ClipSel::Arr { track, id })) = (self.view, self.sel_clip) {
            self.arrow_arr_clip(track, id, dx, -dy, m);
            return true;
        }
        match self.view {
            View::Session => self.arrow_session(dx, -dy),
            // Up and down pick a tape track, left and right move a bar.
            View::Tape => {
                if dy != 0 {
                    self.big.tape.lane = (self.big.tape.lane as i32 - dy).clamp(0, krabton_engine::tape::TAPE_TRACKS as i32 - 1) as usize;
                } else {
                    self.tape_step_bar(dx as f64);
                }
            }
            // Up and down pick the track in the Rack (and the one Touch plays).
            View::Studio | View::Touch => {
                if dy != 0 {
                    self.step_track(-dy);
                }
            }
            View::Arrangement => {
                if dy != 0 {
                    self.step_track(-dy);
                } else {
                    let g = { let g = self.arr.grid_now(self.project.beats_per_bar_f()); if g > 0.0 { g } else { 1.0 } };
                    let g = if m.shift { g * 0.25 } else { g };
                    let pos = self.engine.status.song_pos.load();
                    let next = ((pos / g).round() + dx as f64) * g;
                    self.cmd(krabton_engine::Command::SetPosition(next.max(0.0)));
                }
            }
        }
        true
    }

    /// The arrow keys on the selected notes, as in Live: Up and Down a
    /// semitone (a scale step with the rows folded, a pad in a drum kit),
    /// Shift an octave, Alt the velocity; Left and Right by the grid (Alt a
    /// quarter of it), Shift+Left/Right shorter or longer. A chord at the
    /// edge of the keyboard stays whole. The loop doesn't grow: notes moved
    /// past its end wait there in the dark.
    fn arrow_notes(&mut self, dx: i32, dy: i32, m: Modifiers) {
        let Some(sel) = self.open_midi_clip() else { return };
        let grid = self.piano.grid;
        let idx = self.note_selection();
        let (rows, _, drums) = self.piano_rows(sel.track());
        let folded = self.project.scale_mode && self.piano.fold && !drums;
        let scale = self.project.scale;
        let Some(notes) = self.clip_mut(sel).and_then(|c| c.notes_mut()) else { return };
        let mut moved = false;
        if dx != 0 && m.shift {
            note_edit::stretch(notes, &idx, dx as f64 * grid, grid.min(1.0 / 16.0));
            moved = true;
        } else if dx != 0 {
            let step = if m.alt { grid * 0.25 } else { grid };
            let d = note_edit::nudge(notes, &idx, dx as f64 * step);
            self.piano.time_sel = self.piano.time_sel.map(|(a, b)| (a + d, b + d));
            moved = d != 0.0;
        } else if dy != 0 && m.alt {
            note_edit::change_velocity(notes, &idx, dy as f32 * 10.0 / 127.0);
        } else if dy != 0 {
            moved = if m.shift {
                note_edit::transpose(notes, &idx, 12 * dy)
            } else if drums {
                note_edit::step_rows(notes, &idx, &rows, dy)
            } else if folded {
                note_edit::move_pitch(notes, &idx, |p| Some(scale.step(p, dy)).filter(|&q| q != p))
            } else {
                note_edit::transpose(notes, &idx, dy)
            };
        }
        if moved {
            self.notes_make_room(sel);
        }
        // Let the moved notes be heard (with Preview on).
        if dy != 0 && moved && self.prefs.note_preview {
            let track = sel.track();
            if let Some(top) = self.selected_notes(sel).iter().map(|n| n.pitch).max() {
                let ids = self.preview_note_on(track, top, 0.7);
                self.note_off(top, &ids);
            }
        }
    }

    fn arrow_arr_clip(&mut self, track: usize, id: u64, dx: i32, dtrack: i32, m: Modifiers) {
        if dx != 0 {
            let g = { let g = self.arr.grid_now(self.project.beats_per_bar_f()); if g > 0.0 { g } else { 1.0 } };
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


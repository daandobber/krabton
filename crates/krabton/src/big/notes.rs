//! Big Picture's Notes page: the selected clip as a grid of steps to put
//! notes in. Folded by default (the notes of the key, and any pitch that
//! already has a note), a row per pad on a drum track. Move with the D-pad,
//! A puts a note in (or takes it out) and lets you hear it, X and Y make it
//! a step shorter or longer, the triggers set how hard it plays, B goes back
//! to the clips. With a finger: tap or paint, drag the note names to scroll.

use super::input::{Frame, Nav};
use super::{fit_text, Glyph, Page, Sz, TILE, TILE_HI};
use crate::app::{App, ClipSel};
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, Id, Rect, Sense, Stroke, Ui};
use krabton_engine::engine::Command;
use krabton_engine::{Instrument, Note};

pub const GRIDS: [(f64, &str); 3] = [(0.5, "1/8"), (0.25, "1/16"), (0.125, "1/32")];

pub struct NoteState {
    /// The cursor: step and row.
    pub step: i64,
    pub row: usize,
    pub first_step: i64,
    pub first_row: usize,
    pub grid: usize,
    pub fold: bool,
    /// The clip the view was last centred for.
    pub shown: Option<ClipSel>,
    /// A note being heard: (pitch, tracks, when it stops).
    pub preview: Option<(u8, Vec<u64>, f64)>,
    /// Painting with a finger: notes go in (true) or out.
    pub paint: Option<bool>,
}

impl Default for NoteState {
    fn default() -> Self {
        Self { step: 0, row: 0, first_step: 0, first_row: 0, grid: 1, fold: true, shown: None, preview: None, paint: None }
    }
}

impl App {
    /// The rows of the editor, top to bottom: (pitch, name, is a root).
    fn note_rows(&self, t: usize, notes: &[Note]) -> (Vec<(u8, String, bool)>, bool) {
        if let Some(Instrument::Drums { pads, .. }) = self.project.tracks.get(t).and_then(|tr| tr.instrument.as_ref()) {
            let mut v: Vec<(u8, String, bool)> = pads.iter().map(|p| (p.note, p.name.clone(), false)).collect();
            v.sort_by(|a, b| b.0.cmp(&a.0));
            v.dedup_by_key(|x| x.0);
            return (v, true);
        }
        let scale = self.project.scale;
        let used = |p: u8| notes.iter().any(|n| n.pitch == p);
        let rows = (0..=127u8).rev().filter(|&p| !self.big.notes.fold || scale.contains(p) || used(p)).map(|p| (p, krabton_engine::note_name(p), scale.is_root(p))).collect();
        (rows, false)
    }

    /// Hears a note for a moment.
    fn audition(&mut self, t: usize, pitch: u8, vel: f32, now: f64) {
        if let Some((p, ids, _)) = self.big.notes.preview.take() {
            self.note_off(p, &ids);
        }
        let ids = self.preview_note_on(t, pitch, vel);
        self.big.notes.preview = Some((pitch, ids, now + 0.25));
    }

    /// Lets a heard note go when its moment is over.
    pub(crate) fn end_audition(&mut self, now: f64) {
        if self.big.notes.preview.as_ref().is_some_and(|p| now >= p.2) {
            if let Some((p, ids, _)) = self.big.notes.preview.take() {
                self.note_off(p, &ids);
            }
        }
    }

    pub(crate) fn big_notes(&mut self, ui: &mut Ui, r: Rect, s: &Sz, mut inp: Frame) -> Vec<(Glyph, &'static str)> {
        let p = ui.painter().clone();
        let now = ui.input(|i| i.time);
        let Some(ClipSel::Slot { track: t, scene }) = self.sel_clip.filter(|c| matches!(c, ClipSel::Slot { .. })) else {
            text(&p, pos2(r.left(), r.top() + s.u), Align2::LEFT_CENTER, "Choose a slot on the Clips page first", s.f(), TEXT_DIM);
            if inp.take(Nav::B) {
                self.big.page = Page::Clips;
            }
            return vec![(Glyph::Btn(Nav::B), "Clips")];
        };
        let Some((track_name, track_id, midi, color)) = self.project.tracks.get(t).map(|tr| (tr.name.clone(), tr.id, tr.kind == krabton_engine::TrackKind::Midi, tr.color)) else { return Vec::new() };
        let accent = rgb(color);
        let sel = ClipSel::Slot { track: t, scene };
        if !midi {
            text(&p, pos2(r.left(), r.top() + s.u), Align2::LEFT_CENTER, "Notes are for MIDI tracks", s.f(), TEXT_DIM);
            if inp.take(Nav::B) {
                self.big.page = Page::Clips;
            }
            return vec![(Glyph::Btn(Nav::B), "Clips")];
        }
        // An empty slot: A makes a clip.
        if self.clip_ref(sel).is_none() {
            let msg = format!("{track_name}: an empty slot. A (or a tap) makes a clip here");
            let br = Rect::from_center_size(r.center(), vec2((s.u * 12.0).min(r.width()), s.u * 2.0));
            let resp = ui.interact(br, Id::new("big_new_clip"), Sense::click());
            super::tile(&p, br, TILE_HI, true);
            fit_text(&p, br.center(), Align2::CENTER_CENTER, &msg, s.font, br.width() - s.u, TEXT);
            if inp.take(Nav::A) || resp.clicked() {
                self.create_midi_clip(t, scene);
            }
            if inp.take(Nav::B) {
                self.big.page = Page::Clips;
            }
            return vec![(Glyph::Btn(Nav::A), "New clip"), (Glyph::Btn(Nav::B), "Clips")];
        }
        let clip = self.clip_ref(sel).cloned().unwrap_or_else(|| krabton_engine::Clip::new_midi(0, "", [0, 0, 0], 4.0));
        let notes = clip.notes().to_vec();
        let (rows, drums) = self.note_rows(t, &notes);
        let grid = GRIDS[self.big.notes.grid.min(GRIDS.len() - 1)].0;
        let len = clip.length.max(grid);
        let steps = (len / grid).round().max(1.0) as i64;
        let bpb = self.project.beats_per_bar_f();

        // ---------------------------------------------------------------- the toolbar
        let bar_h = s.u * 1.15;
        let tb = Rect::from_min_size(r.min, vec2(r.width(), bar_h));
        let mut x = tb.left();
        let mut button = |ui: &mut Ui, label: &str, w: f32, on: bool, id: &str| -> bool {
            let br = Rect::from_min_size(pos2(x, tb.top()), vec2(w, bar_h - s.u * 0.12));
            x += w + s.u * 0.15;
            let resp = ui.interact(br, Id::new(("big_notes_btn", id)), Sense::click());
            rect_fill(&p, br, 8.0, if on { accent } else if resp.hovered() { TILE_HI } else { TILE });
            fit_text(&p, br.center(), Align2::CENTER_CENTER, label, s.font, br.width() - 6.0, if on { text_on(accent) } else { TEXT });
            resp.clicked()
        };
        let st = self.status();
        let playing_slot = st.tracks.get(t).map(|ts| ts.playing_slot.load(std::sync::atomic::Ordering::Relaxed)).unwrap_or(-1);
        let clip_pos = st.tracks.get(t).map(|ts| ts.clip_pos.load()).unwrap_or(-1.0);
        let this_plays = playing_slot == scene as i32;
        if button(ui, "< Clips", s.u * 2.4, false, "back") {
            self.big.page = Page::Clips;
        }
        if button(ui, if this_plays { "Stop" } else { "Play" }, s.u * 2.0, this_plays, "play") {
            let id = track_id;
            if this_plays {
                self.cmd(Command::StopTrack { track: id });
            } else {
                self.cmd(Command::LaunchClip { track: id, slot: scene });
            }
        }
        let bars = len / bpb;
        if button(ui, "-", s.u * 1.0, false, "shorter") {
            if let Some(c) = self.clip_mut(sel) {
                c.length = (c.length - bpb).max(bpb);
            }
        }
        let lr = Rect::from_min_size(pos2(x, tb.top()), vec2(s.u * 2.6, bar_h - s.u * 0.12));
        x += s.u * 2.75;
        rect_fill(&p, lr, 8.0, BG);
        fit_text(&p, lr.center(), Align2::CENTER_CENTER, &format!("{} bar{}", (bars * 100.0).round() / 100.0, if (bars - 1.0).abs() < 1e-6 { "" } else { "s" }), s.font, lr.width() - 6.0, TEXT);
        let mut button = |ui: &mut Ui, label: &str, w: f32, on: bool, id: &str| -> bool {
            let br = Rect::from_min_size(pos2(x, tb.top()), vec2(w, bar_h - s.u * 0.12));
            x += w + s.u * 0.15;
            let resp = ui.interact(br, Id::new(("big_notes_btn", id)), Sense::click());
            rect_fill(&p, br, 8.0, if on { accent } else if resp.hovered() { TILE_HI } else { TILE });
            fit_text(&p, br.center(), Align2::CENTER_CENTER, label, s.font, br.width() - 6.0, if on { text_on(accent) } else { TEXT });
            resp.clicked()
        };
        if button(ui, "+", s.u * 1.0, false, "longer") {
            if let Some(c) = self.clip_mut(sel) {
                c.length = (c.length + bpb).min(bpb * 64.0);
            }
        }
        if button(ui, GRIDS[self.big.notes.grid].1, s.u * 1.9, false, "grid") {
            self.big.notes.grid = (self.big.notes.grid + 1) % GRIDS.len();
        }
        if !drums && button(ui, "Fold", s.u * 1.9, self.big.notes.fold, "fold") {
            self.big.notes.fold = !self.big.notes.fold;
            self.big.notes.shown = None;
        }
        if button(ui, "x2", s.u * 1.3, false, "double") {
            if let Some(c) = self.clip_mut(sel) {
                let l = c.length;
                if let Some(v) = c.notes_mut() {
                    let copies: Vec<Note> = v.iter().filter(|n| n.start < l).map(|n| Note { start: n.start + l, ..n.clone() }).collect();
                    v.extend(copies);
                }
                c.length = (l * 2.0).min(bpb * 64.0);
            }
        }
        if button(ui, "Clear", s.u * 2.0, false, "clear") {
            if let Some(v) = self.clip_mut(sel).and_then(|c| c.notes_mut()) {
                v.clear();
            }
        }
        let name_w = (tb.right() - x).max(0.0);
        if name_w > s.u * 2.0 {
            fit_text(&p, pos2(tb.right(), tb.center().y), Align2::RIGHT_CENTER, &format!("{} · {}", track_name, clip.name), s.font, name_w - s.u * 0.3, TEXT_DIM);
        }

        // ---------------------------------------------------------------- the grid
        let keys_w = s.u * 2.6;
        let ruler_h = s.u * 0.7;
        let area = Rect::from_min_max(pos2(r.left(), tb.bottom() + s.u * 0.2), r.max);
        let grid_r = Rect::from_min_max(pos2(area.left() + keys_w, area.top() + ruler_h), area.max);
        let cell_w = (s.u * 0.95).max(grid_r.width() / steps as f32).min(s.u * 2.2);
        let row_h = (s.u * 0.9).max(if drums { grid_r.height() / rows.len().max(1) as f32 } else { 0.0 }).min(s.u * 1.4);
        let vis_steps = ((grid_r.width() / cell_w).floor() as i64).max(1);
        let vis_rows = ((grid_r.height() / row_h).floor() as usize).max(1);
        let n_rows = rows.len().max(1);
        let ns = &mut self.big.notes;
        // A new clip: the cursor goes to its notes (or the middle of the keys).
        if ns.shown != Some(sel) {
            ns.shown = Some(sel);
            let target = notes.iter().map(|n| n.pitch as i32).sum::<i32>().checked_div(notes.len() as i32).unwrap_or(60);
            ns.row = rows.iter().position(|r| r.0 as i32 <= target).unwrap_or(n_rows / 2);
            ns.step = 0;
            ns.first_step = 0;
            ns.first_row = ns.row.saturating_sub(vis_rows / 2);
        }
        ns.row = ns.row.min(n_rows - 1);
        ns.step = ns.step.clamp(0, steps - 1);
        let mut toggle_at: Option<(i64, usize)> = None;
        let mut resize = 0i32;
        let mut back = false;
        for nav in inp.pressed.drain(..) {
            match nav {
                Nav::Left => ns.step = (ns.step - 1).max(0),
                Nav::Right => ns.step = (ns.step + 1).min(steps - 1),
                Nav::Up => ns.row = ns.row.saturating_sub(1),
                Nav::Down => ns.row = (ns.row + 1).min(n_rows - 1),
                Nav::A => toggle_at = Some((ns.step, ns.row)),
                Nav::X => resize -= 1,
                Nav::Y => resize += 1,
                Nav::B => back = true,
                _ => {}
            }
        }
        // Keep the cursor in view.
        if ns.step < ns.first_step {
            ns.first_step = ns.step;
        } else if ns.step >= ns.first_step + vis_steps {
            ns.first_step = ns.step + 1 - vis_steps;
        }
        if ns.row < ns.first_row {
            ns.first_row = ns.row;
        } else if ns.row >= ns.first_row + vis_rows {
            ns.first_row = ns.row + 1 - vis_rows;
        }
        // Fingers and the wheel scroll: the names up and down, the ruler sideways.
        let keys_r = Rect::from_min_max(pos2(area.left(), grid_r.top()), pos2(grid_r.left(), grid_r.bottom()));
        let ruler_r = Rect::from_min_max(pos2(grid_r.left(), area.top()), pos2(grid_r.right(), grid_r.top()));
        let kresp = ui.interact(keys_r, Id::new("big_notes_keys"), Sense::drag());
        let k = super::scrolled_rows(ui, &kresp, keys_r.union(grid_r), row_h, Id::new("big_notes_vscroll"));
        if k != 0 {
            ns.first_row = (ns.first_row as i64 + k).clamp(0, n_rows.saturating_sub(vis_rows) as i64) as usize;
            ns.row = ns.row.clamp(ns.first_row, (ns.first_row + vis_rows - 1).min(n_rows - 1));
        }
        let rresp = ui.interact(ruler_r, Id::new("big_notes_ruler"), Sense::drag());
        if rresp.dragged() {
            let acc_id = Id::new("big_notes_hscroll");
            let acc = ui.ctx().memory(|m| m.data.get_temp::<f32>(acc_id).unwrap_or(0.0)) + rresp.drag_delta().x;
            let kk = (acc / cell_w).trunc();
            ui.ctx().memory_mut(|m| m.data.insert_temp(acc_id, acc - kk * cell_w));
            ns.first_step = (ns.first_step - kk as i64).clamp(0, (steps - vis_steps).max(0));
            ns.step = ns.step.clamp(ns.first_step, (ns.first_step + vis_steps - 1).min(steps - 1));
        }
        ns.first_step = ns.first_step.clamp(0, (steps - vis_steps).max(0));
        ns.first_row = ns.first_row.min(n_rows.saturating_sub(vis_rows));
        let (first_step, first_row, cur_step, cur_row) = (ns.first_step, ns.first_row, ns.step, ns.row);
        let cell_rect = |st: i64, rw: usize| Rect::from_min_size(pos2(grid_r.left() + (st - first_step) as f32 * cell_w, grid_r.top() + (rw - first_row) as f32 * row_h), vec2(cell_w, row_h));
        // Touch: tap or paint.
        let gresp = ui.interact(grid_r, Id::new("big_notes_grid"), Sense::click_and_drag());
        let cell_at = |pt: egui::Pos2| -> Option<(i64, usize)> {
            if !grid_r.contains(pt) {
                return None;
            }
            let st = first_step + ((pt.x - grid_r.left()) / cell_w).floor() as i64;
            let rw = first_row + ((pt.y - grid_r.top()) / row_h).floor() as usize;
            (st < steps && rw < n_rows).then_some((st, rw))
        };
        let note_at = |notes: &[Note], st: i64, pitch: u8| notes.iter().position(|n| n.pitch == pitch && n.start <= st as f64 * grid + 1e-6 && st as f64 * grid < n.start + n.length - 1e-6);
        if gresp.drag_started() || gresp.clicked() {
            if let Some((st, rw)) = gresp.interact_pointer_pos().and_then(cell_at) {
                let on = note_at(&notes, st, rows[rw].0).is_none();
                self.big.notes.paint = Some(on);
                self.big.notes.step = st;
                self.big.notes.row = rw;
                toggle_at = Some((st, rw));
            }
        } else if gresp.dragged() {
            if let (Some(on), Some((st, rw))) = (self.big.notes.paint, gresp.interact_pointer_pos().and_then(cell_at)) {
                let here = note_at(&notes, st, rows[rw].0).is_some();
                if here != on {
                    toggle_at = Some((st, rw));
                }
                self.big.notes.step = st;
                self.big.notes.row = rw;
            }
        }
        if gresp.drag_stopped() {
            self.big.notes.paint = None;
        }
        // Edits.
        if let Some((st, rw)) = toggle_at {
            let pitch = rows[rw].0;
            let mut heard = None;
            if let Some(v) = self.clip_mut(sel).and_then(|c| c.notes_mut()) {
                match note_at(v, st, pitch) {
                    Some(i) => {
                        v.remove(i);
                    }
                    None => {
                        v.push(Note::new(pitch, st as f64 * grid, grid, 0.8));
                        heard = Some(pitch);
                    }
                }
            }
            if let Some(pitch) = heard {
                self.audition(t, pitch, 0.8, now);
            }
        }
        let cur_pitch = rows[cur_row].0;
        if resize != 0 || inp.turn.abs() > 0.01 {
            let turn = inp.turn * inp.dt;
            if let Some(v) = self.clip_mut(sel).and_then(|c| c.notes_mut()) {
                if let Some(i) = note_at(v, cur_step, cur_pitch) {
                    let n = &mut v[i];
                    if resize != 0 {
                        n.length = (n.length + resize as f64 * grid).clamp(grid, len - n.start);
                    }
                    n.velocity = (n.velocity + turn * 0.8).clamp(0.05, 1.0);
                }
            }
        }
        if back {
            self.big.page = Page::Clips;
        }

        // ---------------------------------------------------------------- drawing
        let notes = self.clip_ref(sel).map(|c| c.notes().to_vec()).unwrap_or_default();
        rect_fill(&p, grid_r, 6.0, Color32::from_rgb(0x1c, 0x1c, 0x20));
        let last_step = (first_step + vis_steps).min(steps);
        let last_row = (first_row + vis_rows).min(n_rows);
        for rw in first_row..last_row {
            let (pitch, name, root) = &rows[rw];
            let y = grid_r.top() + (rw - first_row) as f32 * row_h;
            let row_r = Rect::from_min_size(pos2(grid_r.left(), y), vec2(grid_r.width(), row_h));
            if *root {
                p.rect_filled(row_r, 0.0, shade(accent, 0.16));
            }
            let kr = Rect::from_min_size(pos2(area.left(), y + 1.0), vec2(keys_w - s.u * 0.15, row_h - 2.0));
            rect_fill(&p, kr, 4.0, if rw == cur_row { TILE_HI } else { TILE });
            fit_text(&p, pos2(kr.left() + s.u * 0.2, kr.center().y), Align2::LEFT_CENTER, name, s.small.max(row_h * 0.45), kr.width() - s.u * 0.3, if *root { accent } else { TEXT });
            let _ = pitch;
            p.line_segment([pos2(grid_r.left(), y + row_h), pos2(grid_r.right(), y + row_h)], Stroke::new(1.0_f32, LINE_SOFT));
        }
        // Beats and bars.
        let steps_per_beat = (1.0 / grid).round() as i64;
        let steps_per_bar = (bpb / grid).round() as i64;
        for st in first_step..=last_step {
            let x = grid_r.left() + (st - first_step) as f32 * cell_w;
            let (w, c): (f32, Color32) = if st % steps_per_bar.max(1) == 0 { (1.5, LINE) } else if st % steps_per_beat.max(1) == 0 { (1.0, LINE) } else { (1.0, LINE_SOFT) };
            p.line_segment([pos2(x, grid_r.top()), pos2(x, grid_r.bottom())], Stroke::new(w, c));
            if st % steps_per_bar.max(1) == 0 && st < steps {
                text(&p, pos2(x + 3.0, area.top() + ruler_h * 0.5), Align2::LEFT_CENTER, format!("{}", st / steps_per_bar.max(1) + 1), s.s(), TEXT_DIM);
            }
        }
        // The notes.
        let view_start = first_step as f64 * grid;
        let view_end = last_step as f64 * grid;
        for n in &notes {
            let Some(rw) = rows.iter().position(|r| r.0 == n.pitch) else { continue };
            if rw < first_row || rw >= last_row || n.start + n.length <= view_start || n.start >= view_end {
                continue;
            }
            let x0 = grid_r.left() + ((n.start.max(view_start) - view_start) / grid) as f32 * cell_w;
            let x1 = grid_r.left() + ((((n.start + n.length).min(view_end)) - view_start) / grid) as f32 * cell_w;
            let y = grid_r.top() + (rw - first_row) as f32 * row_h;
            let nr = Rect::from_min_max(pos2(x0 + 1.5, y + 2.0), pos2((x1 - 1.5).max(x0 + 4.0), y + row_h - 2.0));
            rect_fill(&p, nr, 4.0, mix(shade(accent, 0.45), accent, n.velocity));
        }
        // The cursor and where the clip plays.
        if cur_step >= first_step && cur_step < last_step && cur_row >= first_row && cur_row < last_row {
            let cr = cell_rect(cur_step, cur_row);
            rect_line(&p, cr.expand(1.0), 4.0, 2.5, super::FOCUS);
        }
        if this_plays && clip_pos >= 0.0 {
            let ps = clip_pos / grid;
            if ps >= first_step as f64 && ps < last_step as f64 {
                let x = grid_r.left() + (ps - first_step as f64) as f32 * cell_w;
                p.line_segment([pos2(x, area.top()), pos2(x, grid_r.bottom())], Stroke::new(2.0_f32, TEXT));
            }
        }
        vec![(Glyph::Dpad, "Move"), (Glyph::Btn(Nav::A), "Note"), (Glyph::Btn(Nav::X), "Shorter"), (Glyph::Btn(Nav::Y), "Longer"), (Glyph::Triggers, "Velocity"), (Glyph::Btn(Nav::B), "Clips")]
    }
}

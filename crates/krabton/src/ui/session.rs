//! Session view: the clip launcher grid, scene launchers and the mixer.

use crate::app::{App, ArrClipDrag, BrowserItem, ClipSel, Deferred, DropTarget, RenameTarget, SlotDrag, TrackSel};
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Id, Rect, Sense, Stroke, Ui, UiBuilder};
use krabton_engine::engine::{EngineStatus, QUEUE_STOP};
use krabton_engine::modulation::ModTarget;
use krabton_engine::{fader_gain, Command, TrackKind, PALETTE};
use std::sync::atomic::Ordering;
use std::time::Instant;

/// Track columns, until the user drags them wider or narrower.
const COL_W: f32 = 112.0;
const HEADER_H: f32 = 24.0;
const SLOT_H: f32 = 24.0;
const STOP_H: f32 = 24.0;
const MIXER_H: f32 = 226.0;
/// A shorter mixer drops the send and pan knobs.
const MIXER_MIN: f32 = 100.0;
const COMPACT_BELOW: f32 = 176.0;
use crate::ui::io_section::IO_H;
const MASTER_W: f32 = 132.0;
const DROP_W: f32 = 150.0;
const TRI_W: f32 = 20.0;

#[derive(Default)]
pub struct SessionState {
    pub scroll_x: f32,
    pub scroll_y: f32,
    pub taps: Vec<Instant>,
    /// The In/Out section under the clip slots is hidden.
    pub hide_io: bool,
}

fn child(ui: &mut Ui, rect: Rect) -> Ui {
    let clip = rect.intersect(ui.clip_rect());
    let mut c = ui.new_child(UiBuilder::new().max_rect(rect));
    c.set_clip_rect(clip);
    c
}

/// Color swatches for context menus. Returns the picked color.
pub fn color_menu(ui: &mut Ui) -> Option<[u8; 3]> {
    let mut picked = None;
    ui.menu_button("Color", |ui| {
        egui::Grid::new("palette").spacing(vec2(2.0, 2.0)).show(ui, |ui| {
            for (i, c) in PALETTE.iter().enumerate() {
                let (r, resp) = ui.allocate_exact_size(vec2(18.0, 18.0), Sense::click());
                ui.painter().rect_filled(r, 2.0, rgb(*c));
                if resp.hovered() {
                    rect_line(ui.painter(), r, 2.0, 1.5, egui::Color32::WHITE);
                }
                if resp.clicked() {
                    picked = Some(*c);
                    ui.close();
                }
                if i % 8 == 7 {
                    ui.end_row();
                }
            }
        });
    });
    picked
}

/// Toggles a few times per second, for queued clips.
pub fn blink() -> bool {
    let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0);
    (t / 180) % 2 == 0
}

impl App {
    pub(crate) fn ui_session(&mut self, ui: &mut Ui) {
        // An arrangement clip dragged over here and let go: it stays where it was.
        if !ui.input(|i| i.pointer.any_down()) {
            self.arr_cancel_drag();
        }
        let full = ui.available_rect_before_wrap();
        ui.allocate_rect(full, Sense::hover());
        let status = self.status();
        let nt = self.project.tracks.len();
        let nr = self.project.returns.len();
        let ns = self.project.scenes.len();

        // The In/Out section sits between the clip slots and the mixer; when
        // space is short the mixer gives way (it turns compact).
        // Its height is the user's (drag the line above it) or a share of the view.
        let col_w = crate::prefs::Layout::or(self.prefs.layout.track_w, COL_W).clamp(60.0, 320.0);
        let grid_min = HEADER_H + SLOT_H;
        let room = (full.height() - grid_min - if self.session.hide_io { 0.0 } else { IO_H }).max(MIXER_MIN);
        let wanted = if self.prefs.layout.mixer_h > 0.0 { self.prefs.layout.mixer_h } else { MIXER_H.min(full.height() * 0.62) };
        let mixer_h = wanted.clamp(MIXER_MIN, room);
        let io_h = if self.session.hide_io { 0.0 } else { IO_H.min((full.height() - mixer_h - grid_min).max(0.0)) };
        let grid = Rect::from_min_max(full.min, pos2(full.right(), full.bottom() - mixer_h - io_h));
        let io = Rect::from_min_max(pos2(full.left(), grid.bottom()), pos2(full.right(), grid.bottom() + io_h));
        let mixer = Rect::from_min_max(pos2(full.left(), io.bottom()), full.max);
        let cols_right = full.right() - MASTER_W;
        let header = Rect::from_min_max(grid.min, pos2(cols_right, grid.top() + HEADER_H));
        let slots = Rect::from_min_max(pos2(grid.left(), header.bottom()), pos2(cols_right, grid.bottom()));
        let master_header = Rect::from_min_max(pos2(cols_right, grid.top()), pos2(full.right(), header.bottom()));
        let master_slots = Rect::from_min_max(pos2(cols_right, header.bottom()), pos2(full.right(), grid.bottom()));
        let mixer_cols = Rect::from_min_max(mixer.min, pos2(cols_right, mixer.bottom()));
        let mixer_master = Rect::from_min_max(pos2(cols_right, mixer.top()), mixer.max);

        // Scrolling.
        let content_w = (nt + nr) as f32 * col_w + DROP_W;
        let content_h = ns as f32 * SLOT_H + STOP_H + 40.0;
        if ui.rect_contains_pointer(full) {
            let (d, shift) = ui.input(|i| (i.smooth_scroll_delta, i.modifiers.shift));
            let over_grid = ui.rect_contains_pointer(slots) || ui.rect_contains_pointer(master_slots);
            if shift || !over_grid {
                self.session.scroll_x -= if d.x != 0.0 { d.x } else { d.y };
            } else {
                self.session.scroll_x -= d.x;
                self.session.scroll_y -= d.y;
            }
        }
        self.session.scroll_x = self.session.scroll_x.clamp(0.0, (content_w - slots.width()).max(0.0));
        self.session.scroll_y = self.session.scroll_y.clamp(0.0, (content_h - slots.height()).max(0.0));
        let sx = self.session.scroll_x;
        let sy = self.session.scroll_y;
        let col_x = |i: usize| slots.left() + i as f32 * col_w - sx;
        let row_y = |s: usize| slots.top() + s as f32 * SLOT_H - sy;

        let p = ui.painter();
        rect_fill(p, grid, 0.0, BG);
        rect_fill(p, mixer, 0.0, PANEL);
        p.line_segment([pos2(full.left(), mixer.top()), pos2(full.right(), mixer.top())], Stroke::new(1.0_f32, LINE));
        p.line_segment([pos2(cols_right, full.top()), pos2(cols_right, full.bottom())], Stroke::new(1.0_f32, LINE));

        // Track and return headers.
        {
            let mut hui = child(ui, header);
            for i in 0..nt {
                let r = Rect::from_min_size(pos2(col_x(i), header.top()), vec2(col_w - 1.0, HEADER_H - 1.0));
                if r.intersects(header) {
                    self.track_header(&mut hui, i, r);
                }
            }
            for j in 0..nr {
                let r = Rect::from_min_size(pos2(col_x(nt + j), header.top()), vec2(col_w - 1.0, HEADER_H - 1.0));
                if r.intersects(header) {
                    self.return_header(&mut hui, j, r);
                }
            }
        }

        // Clip slots.
        {
            let mut sui = child(ui, slots);
            for i in 0..nt {
                let x = col_x(i);
                if x + col_w < slots.left() || x > slots.right() {
                    continue;
                }
                for s in 0..ns {
                    let r = Rect::from_min_size(pos2(x + 1.0, row_y(s) + 1.0), vec2(col_w - 3.0, SLOT_H - 2.0));
                    if r.intersects(slots) {
                        self.slot(&mut sui, i, s, r, &status);
                    }
                }
                let r = Rect::from_min_size(pos2(x + 1.0, row_y(ns) + 3.0), vec2(col_w - 3.0, STOP_H - 4.0));
                self.track_stop_button(&mut sui, i, r, &status);
            }
            for j in 0..nr {
                let x = col_x(nt + j);
                let r = Rect::from_min_max(pos2(x, slots.top()), pos2(x + col_w - 1.0, slots.bottom()));
                rect_fill(sui.painter(), r, 0.0, shade(PANEL, 0.85));
            }
            let drop = Rect::from_min_max(pos2(col_x(nt + nr) + 4.0, slots.top() + 4.0), pos2(col_x(nt + nr) + DROP_W - 8.0, slots.bottom() - 8.0));
            self.drop_zone(&mut sui, drop);
        }

        // Master column: scene launchers.
        {
            let mut mui = child(ui, master_header.union(master_slots));
            let p = mui.painter().clone();
            // Click the header for the master's effects; drop effects on it to add them there.
            let head = mui.interact(master_header, Id::new("master_head"), Sense::click());
            let selected = self.sel_track == TrackSel::Master;
            let hovering = head.dnd_hover_payload::<BrowserItem>().is_some();
            rect_fill(&p, master_header.shrink(0.5), 0.0, if selected || head.hovered() { shade(PANEL3, 1.25) } else { PANEL3 });
            if hovering {
                rect_line(&p, master_header.shrink(1.0), 2.0, 1.5, ACCENT);
            }
            text(&p, master_header.center(), Align2::CENTER_CENTER, "Master", body(), if selected { ACCENT } else { TEXT });
            if head.clicked() {
                self.sel_track = TrackSel::Master;
                self.show_devices();
            }
            if let Some(item) = drop_payload::<BrowserItem>(&head) {
                self.apply_item(&item, DropTarget::Chain(TrackSel::Master));
                self.sel_track = TrackSel::Master;
            }
            head.on_hover_text("The master: click for its effects, or drop effects here");
            // Shows or hides the In/Out section, like Live's I-O button.
            let b = Rect::from_min_size(pos2(master_header.right() - 30.0, master_header.top() + 4.0), vec2(26.0, master_header.height() - 8.0));
            let shown = !self.session.hide_io;
            if button(&mut mui, b, Id::new("io_toggle"), "I/O", shown, ACCENT).on_hover_text("Show or hide In/Out (Ctrl+Alt+I)").clicked() {
                self.session.hide_io = shown;
            }
            let mut mui = child(&mut mui, master_slots);
            for s in 0..ns {
                let r = Rect::from_min_size(pos2(master_slots.left() + 2.0, row_y(s) + 1.0), vec2(MASTER_W - 4.0, SLOT_H - 2.0));
                if r.intersects(master_slots) {
                    self.scene_cell(&mut mui, s, r);
                }
            }
            let r = Rect::from_min_size(pos2(master_slots.left() + 2.0, row_y(ns) + 3.0), vec2(MASTER_W - 4.0, STOP_H - 4.0));
            let resp = mui.interact(r, Id::new("stop_all"), Sense::click());
            rect_fill(mui.painter(), r, 2.0, if resp.hovered() { PANEL3 } else { PANEL2 });
            stop_icon(mui.painter(), pos2(r.left() + 12.0, r.center().y), 8.0, TEXT);
            text(mui.painter(), pos2(r.left() + 24.0, r.center().y), Align2::LEFT_CENTER, "Stop All Clips", small(), TEXT);
            if resp.clicked() {
                self.cmd(Command::StopAllClips);
            }
            let r = Rect::from_min_size(pos2(master_slots.left() + 2.0, row_y(ns) + STOP_H + 4.0), vec2(MASTER_W - 4.0, 18.0));
            let resp = mui.interact(r, Id::new("add_scene"), Sense::click());
            rect_fill(mui.painter(), r, 2.0, if resp.hovered() { PANEL3 } else { BG });
            text(mui.painter(), r.center(), Align2::CENTER_CENTER, "+ Scene", small(), TEXT_DIM);
            if resp.clicked() {
                let n = self.project.scenes.len();
                self.deferred.push(Deferred::InsertScene(n));
            }
        }

        // In/Out section.
        if io_h >= IO_H - 0.5 {
            let io_cols = Rect::from_min_max(io.min, pos2(cols_right, io.bottom()));
            let mut iui = child(ui, io_cols);
            for i in 0..nt {
                let r = Rect::from_min_size(pos2(col_x(i), io.top()), vec2(col_w - 1.0, io_h));
                if r.intersects(io_cols) {
                    self.track_io(&mut iui, i, r, &status);
                }
            }
            for j in 0..nr {
                let r = Rect::from_min_size(pos2(col_x(nt + j), io.top()), vec2(col_w - 1.0, io_h));
                if r.intersects(io_cols) {
                    self.return_io(&mut iui, r);
                }
            }
            let mut mi = child(ui, Rect::from_min_max(pos2(cols_right, io.top()), pos2(full.right(), io.bottom())));
            self.master_io(&mut mi, Rect::from_min_max(pos2(cols_right + 1.0, io.top()), pos2(full.right(), io.bottom())));
            ui.painter().line_segment([pos2(full.left(), io.top()), pos2(full.right(), io.top())], Stroke::new(1.0_f32, LINE));
        }

        // Mixer strips.
        {
            let mut xui = child(ui, mixer_cols);
            for i in 0..nt {
                let r = Rect::from_min_size(pos2(col_x(i), mixer.top() + 1.0), vec2(col_w - 1.0, mixer.height() - 1.0));
                if r.intersects(mixer_cols) {
                    self.track_strip(&mut xui, i, r);
                }
            }
            for j in 0..nr {
                let r = Rect::from_min_size(pos2(col_x(nt + j), mixer.top() + 1.0), vec2(col_w - 1.0, mixer.height() - 1.0));
                if r.intersects(mixer_cols) {
                    self.return_strip(&mut xui, j, r);
                }
            }
            let mut mm = child(ui, mixer_master);
            self.master_strip(&mut mm, mixer_master.shrink2(vec2(2.0, 1.0)));
        }

        // Horizontal scrollbar.
        if content_w > slots.width() {
            let bar = Rect::from_min_max(pos2(slots.left(), grid.bottom() - 6.0), pos2(cols_right, grid.bottom() - 1.0));
            let frac = slots.width() / content_w;
            let w = (bar.width() * frac).max(30.0);
            let x = bar.left() + (bar.width() - w) * (sx / (content_w - slots.width()).max(1.0));
            let thumb = Rect::from_min_size(pos2(x, bar.top()), vec2(w, bar.height()));
            let resp = ui.interact(thumb, Id::new("session_hscroll"), Sense::drag());
            if resp.dragged() {
                self.session.scroll_x += resp.drag_delta().x / frac;
            }
            rect_fill(ui.painter(), thumb, 2.0, if resp.hovered() || resp.dragged() { TEXT_DIM } else { PANEL3 });
        }

        // Dividers: the line above the In/Out section and mixer, and the
        // right edge of every track header.
        let split = Rect::from_min_max(pos2(full.left(), grid.bottom() - 1.0), pos2(full.right(), grid.bottom() + 5.0));
        let resp = splitter(ui, Id::new("session_mixer_split"), split, false);
        if resp.dragged() {
            self.prefs.layout.mixer_h = (mixer_h - resp.drag_delta().y).clamp(MIXER_MIN, room);
            self.layout_dirty = true;
        }
        grip_dots(ui.painter(), pos2(cols_right - 20.0, grid.bottom() + 2.0), false, TEXT_DIM);
        for i in 0..nt + nr {
            let x = col_x(i) + col_w - 1.0;
            if x < slots.left() || x > cols_right {
                continue;
            }
            let r = Rect::from_center_size(pos2(x, header.center().y), vec2(7.0, HEADER_H));
            let resp = splitter(ui, Id::new(("session_col_split", i)), r, true);
            if resp.dragged() {
                // Every column changes, so this edge moves (i + 1) times as far.
                self.prefs.layout.track_w = (col_w + resp.drag_delta().x / (i + 1) as f32).clamp(60.0, 320.0);
                self.layout_dirty = true;
            }
        }
    }

    fn track_header(&mut self, ui: &mut Ui, i: usize, r: Rect) {
        if i >= self.project.tracks.len() {
            return;
        }
        let selected = self.is_selected(i);
        let t = &self.project.tracks[i];
        let color = rgb(t.color);
        let fill = if selected { color } else { shade(color, 0.72) };
        let name = t.name.clone();
        let id = Id::new(("track_header", t.id));
        let resp = ui.interact(r, id, Sense::click());
        let p = ui.painter();
        rect_fill(p, r, 2.0, fill);
        text_in(p, r.shrink2(vec2(2.0, 0.0)), &name, body(), text_on(fill));
        if selected {
            rect_line(p, r, 2.0, 1.5, egui::Color32::WHITE);
        }
        if let Some(item) = resp.dnd_hover_payload::<BrowserItem>() {
            let _ = item;
            rect_line(p, r, 2.0, 2.0, ACCENT);
        }
        if let Some(item) = drop_payload::<BrowserItem>(&resp) {
            self.apply_item(&item, DropTarget::Track(i));
        }
        if resp.clicked() {
            let ctrl = ui.input(|k| k.modifiers.ctrl || k.modifiers.command);
            self.click_track(i, ctrl);
            self.show_devices();
        }
        if resp.double_clicked() {
            self.renaming = Some((RenameTarget::Track(i), name.clone()));
        }
        resp.context_menu(|ui| {
            if ui.button("Rename").clicked() {
                self.renaming = Some((RenameTarget::Track(i), name.clone()));
                ui.close();
            }
            if ui.button("Duplicate").clicked() {
                self.deferred.push(Deferred::DuplicateTrack(i));
                ui.close();
            }
            if ui.button("Record into new audio track").on_hover_text("Adds an armed audio track that listens to this track, ready to record it").clicked() {
                self.deferred.push(Deferred::RecordIntoAudio(i));
                ui.close();
            }
            if ui.button("Delete").clicked() {
                self.deferred.push(Deferred::DeleteTrack(i));
                ui.close();
            }
            ui.separator();
            if i > 0 && ui.button("Move Left").clicked() {
                self.deferred.push(Deferred::MoveTrack { from: i, to: i - 1 });
                ui.close();
            }
            if i + 1 < self.project.tracks.len() && ui.button("Move Right").clicked() {
                self.deferred.push(Deferred::MoveTrack { from: i, to: i + 1 });
                ui.close();
            }
            if let Some(c) = color_menu(ui) {
                if let Some(t) = self.project.tracks.get_mut(i) {
                    t.color = c;
                }
            }
        });
    }

    fn return_header(&mut self, ui: &mut Ui, j: usize, r: Rect) {
        if j >= self.project.returns.len() {
            return;
        }
        let ret = &self.project.returns[j];
        let selected = self.sel_track == TrackSel::Return(j);
        let fill = if selected { shade(rgb(ret.color), 0.9) } else { shade(rgb(ret.color), 0.55) };
        let name = ret.name.clone();
        let resp = ui.interact(r, Id::new(("return_header", ret.id)), Sense::click());
        let p = ui.painter();
        rect_fill(p, r, 2.0, fill);
        text_in(p, r, &name, body(), text_on(fill));
        if selected {
            rect_line(p, r, 2.0, 1.5, egui::Color32::WHITE);
        }
        if let Some(item) = drop_payload::<BrowserItem>(&resp) {
            self.apply_item(&item, DropTarget::Chain(TrackSel::Return(j)));
        }
        if resp.clicked() {
            self.sel_track = TrackSel::Return(j);
            self.show_devices();
        }
        if resp.double_clicked() {
            self.renaming = Some((RenameTarget::Return(j), name.clone()));
        }
        resp.context_menu(|ui| {
            if ui.button("Rename").clicked() {
                self.renaming = Some((RenameTarget::Return(j), name.clone()));
                ui.close();
            }
            if ui.button("Delete Return").clicked() {
                self.deferred.push(Deferred::DeleteReturn(j));
                ui.close();
            }
        });
    }

    fn slot(&mut self, ui: &mut Ui, i: usize, s: usize, r: Rect, status: &EngineStatus) {
        if self.project.tracks.get(i).map(|t| s >= t.slots.len()).unwrap_or(true) {
            return;
        }
        let track = &self.project.tracks[i];
        let ts = &status.tracks[i.min(status.tracks.len() - 1)];
        let playing = ts.playing_slot.load(Ordering::Relaxed) == s as i32;
        let queued = ts.queued_slot.load(Ordering::Relaxed) == s as i32;
        let selected = self.sel_clip == Some(ClipSel::Slot { track: i, scene: s });
        let resp = ui.interact(r, Id::new(("slot", track.id, s)), Sense::click_and_drag());
        let in_tri = resp.interact_pointer_pos().or(resp.hover_pos()).map(|p| p.x < r.left() + TRI_W).unwrap_or(false);
        let p = ui.painter().clone();
        let tri_center = pos2(r.left() + TRI_W * 0.5, r.center().y);
        let clip_info = track.slots[s].as_ref().map(|c| (c.name.clone(), c.color, c.length, c.sound.is_some()));
        let armed = track.arm;
        let kind = track.kind;
        let spb = status.sample_rate.load() as f64 * 60.0 / self.project.bpm.max(1.0);
        let take = self.take_on(track.id, crate::recording::TakeTarget::Slot(s)).map(|t| (t.beats(spb), t.end.is_some(), t.start));

        if let Some((name, color, length, keeps)) = &clip_info {
            let base = rgb(*color);
            let fill = if playing { base } else { shade(base, 0.72) };
            rect_fill(&p, r, 2.0, fill);
            let tri_box = Rect::from_min_size(r.min, vec2(TRI_W, r.height()));
            if playing || queued {
                rect_fill(&p, tri_box.shrink(2.0), 2.0, shade(base, 0.3));
            }
            let tri_col = if playing {
                PLAY
            } else if queued {
                if blink() {
                    PLAY
                } else {
                    TEXT_DARK
                }
            } else if resp.hovered() && in_tri {
                egui::Color32::WHITE
            } else {
                shade(text_on(fill), 0.9)
            };
            play_icon(&p, tri_center, 10.0, tri_col);
            let mut name_rect = Rect::from_min_max(pos2(r.left() + TRI_W, r.top()), r.max);
            if *keeps {
                // It keeps its own sound: a knob.
                name_rect.max.x -= 16.0;
                sound_mark(&p, pos2(r.right() - 9.0, r.center().y), text_on(fill));
            }
            text_in(&p, name_rect, name, small(), text_on(fill));
            if playing {
                let pos = ts.clip_pos.load().max(0.0);
                let frac = (pos / length.max(1e-6)).clamp(0.0, 1.0) as f32;
                let bar = Rect::from_min_size(pos2(r.left() + TRI_W, r.bottom() - 3.0), vec2((r.width() - TRI_W) * frac, 2.0));
                rect_fill(&p, bar, 0.0, shade(base, 0.25));
            }
        } else {
            rect_fill(&p, r, 2.0, if resp.hovered() { shade(SLOT, 1.2) } else { SLOT });
            if let Some((beats, stopping, start)) = take {
                // Recording into this slot.
                rect_fill(&p, r, 2.0, shade(REC, 0.35));
                rec_icon(&p, tri_center, 8.0, if blink() || stopping { REC } else { shade(REC, 0.6) });
                let label = if start.is_none_or(|s| status.session_pos.load() < s) && beats == 0.0 {
                    "Waiting...".to_string()
                } else {
                    format!("{} {:.1}", if stopping { "Stopping" } else { "Recording" }, beats / self.project.beats_per_bar_f())
                };
                text_in(&p, Rect::from_min_max(pos2(r.left() + TRI_W, r.top()), r.max), &label, small(), egui::Color32::WHITE);
            } else if armed && (kind == TrackKind::Midi || kind == TrackKind::Audio) {
                rec_icon(&p, tri_center, 8.0, if resp.hovered() && in_tri { REC } else { shade(REC, 0.5) });
            } else {
                stop_icon(&p, tri_center, 6.0, if resp.hovered() && in_tri { TEXT } else { LINE });
            }
        }
        if s == self.sel_scene && clip_info.is_none() {
            rect_line(&p, r, 2.0, 1.0, LINE);
        }
        if selected {
            rect_line(&p, r, 2.0, 1.5, egui::Color32::WHITE);
        }

        // Drag & drop.
        if clip_info.is_some() {
            resp.dnd_set_drag_payload(SlotDrag { track: i, scene: s });
        }
        if resp.dnd_hover_payload::<SlotDrag>().is_some() || resp.dnd_hover_payload::<BrowserItem>().is_some() || resp.dnd_hover_payload::<ArrClipDrag>().is_some() {
            rect_line(&p, r, 2.0, 2.0, ACCENT);
        }
        if let Some(drag) = drop_payload::<ArrClipDrag>(&resp) {
            // Dragging to the session copies: the arrangement clip goes back.
            self.arr_cancel_drag();
            if let Some(from) = self.project.tracks.iter().position(|t| t.arrangement.iter().any(|a| a.id == drag.id)) {
                self.arr_clip_to_slot((from, drag.id), (i, s));
            }
            return;
        }
        if let Some(from) = drop_payload::<SlotDrag>(&resp) {
            let copy = ui.input(|i| i.modifiers.ctrl || i.modifiers.command);
            self.move_slot_clip(*from, (i, s), copy);
            return;
        }
        if let Some(item) = drop_payload::<BrowserItem>(&resp) {
            self.apply_item(&item, DropTarget::Slot { track: i, scene: s });
            return;
        }

        let track_id = self.project.tracks[i].id;
        if resp.clicked() {
            match &clip_info {
                Some(_) => {
                    if in_tri {
                        self.cmd(Command::LaunchClip { track: track_id, slot: s });
                    }
                    self.select_clip(ClipSel::Slot { track: i, scene: s });
                }
                None => {
                    self.sel_track = TrackSel::Track(i);
                    self.sel_scene = s;
                    self.sel_clip = None;
                    if in_tri {
                        if take.is_some() {
                            self.stop_slot_take(i);
                        } else if armed && kind == TrackKind::Audio {
                            self.start_slot_take(i, s);
                        } else if armed && kind == TrackKind::Midi {
                            self.create_midi_clip(i, s);
                            if let Some(Some(c)) = self.project.tracks[i].slots.get_mut(s) {
                                c.length = self.project.beats_per_bar as f64 * 4.0;
                                c.name = "Recording".into();
                            }
                            self.cmd(Command::LaunchClip { track: track_id, slot: s });
                            if !self.recording {
                                self.recording = true;
                            }
                        } else {
                            self.cmd(Command::StopTrack { track: track_id });
                        }
                    }
                }
            }
        }
        if resp.double_clicked() && clip_info.is_none() && kind == TrackKind::Midi {
            self.create_midi_clip(i, s);
        }
        let sel = ClipSel::Slot { track: i, scene: s };
        resp.context_menu(|ui| {
            if let Some((name, _, _, keeps)) = &clip_info {
                if ui.button("Launch").clicked() {
                    self.cmd(Command::LaunchClip { track: track_id, slot: s });
                    ui.close();
                }
                sound_menu(self, ui, sel, *keeps);
                if ui.button("Rename").clicked() {
                    self.renaming = Some((RenameTarget::Clip(sel), name.clone()));
                    ui.close();
                }
                if ui.button("Duplicate      Ctrl+D").clicked() {
                    self.duplicate_clip(sel);
                    ui.close();
                }
                if ui.button("Copy to Arrangement").on_hover_text("Places the clip at the playhead on this track's timeline").clicked() {
                    self.copy_clip_to_arrangement(sel);
                    ui.close();
                }
                if kind == TrackKind::Audio {
                    ui.menu_button("Export Audio", |ui| {
                        if ui.button("WAV...").on_hover_text("This clip on its own, as it plays").clicked() {
                            self.export_clip(sel, false);
                            ui.close();
                        }
                        if ui.button("MP3...").on_hover_text("320 kbit/s, through LAME or ffmpeg on this computer").clicked() {
                            self.export_clip(sel, true);
                            ui.close();
                        }
                    });
                }
                if ui.button("Delete         Del").clicked() {
                    self.delete_clip(sel);
                    ui.close();
                }
                if let Some(c) = color_menu(ui) {
                    if let Some(clip) = self.clip_mut(sel) {
                        clip.color = c;
                    }
                }
            } else {
                if kind == TrackKind::Midi && ui.button("Insert MIDI Clip").clicked() {
                    self.create_midi_clip(i, s);
                    ui.close();
                }
                if ui.button("Stop Track").clicked() {
                    self.cmd(Command::StopTrack { track: track_id });
                    ui.close();
                }
            }
        });
    }

    fn track_stop_button(&mut self, ui: &mut Ui, i: usize, r: Rect, status: &EngineStatus) {
        if i >= self.project.tracks.len() {
            return;
        }
        let ts = &status.tracks[i.min(status.tracks.len() - 1)];
        let playing = ts.playing_slot.load(Ordering::Relaxed) >= 0;
        let queued_stop = ts.queued_slot.load(Ordering::Relaxed) == QUEUE_STOP;
        let id = self.project.tracks[i].id;
        let resp = ui.interact(r, Id::new(("track_stop", id)), Sense::click());
        let p = ui.painter();
        rect_fill(p, r, 2.0, if resp.hovered() { PANEL3 } else { PANEL2 });
        let c = if queued_stop {
            if blink() {
                ACCENT
            } else {
                TEXT_DIM
            }
        } else if playing {
            TEXT
        } else {
            LINE
        };
        stop_icon(p, r.center(), 8.0, c);
        if resp.on_hover_text("Stop clip on this track").clicked() {
            self.cmd(Command::StopTrack { track: id });
        }
    }

    fn drop_zone(&mut self, ui: &mut Ui, r: Rect) {
        if r.width() < 20.0 {
            return;
        }
        let resp = ui.interact(r, Id::new("new_track_drop"), Sense::click());
        let hovering = resp.dnd_hover_payload::<BrowserItem>().is_some();
        let p = ui.painter();
        let c = if hovering { ACCENT } else { LINE_SOFT };
        rect_line(p, r, 4.0, 1.0, c);
        let galley_rect = Rect::from_center_size(r.center(), vec2(r.width() - 10.0, 60.0));
        let pc = p.with_clip_rect(galley_rect);
        for (k, line) in ["Drop instruments,", "effects or samples", "here to add a track"].iter().enumerate() {
            pc.text(pos2(r.center().x, r.center().y - 14.0 + k as f32 * 14.0), Align2::CENTER_CENTER, *line, small(), if hovering { ACCENT } else { TEXT_DIM });
        }
        if let Some(item) = drop_payload::<BrowserItem>(&resp) {
            self.apply_item(&item, DropTarget::NewTrack);
        }
        if resp.double_clicked() {
            self.add_midi_track(Some(krabton_engine::Instrument::synth("Init")));
        }
        resp.context_menu(|ui| {
            if ui.button("Add MIDI Track").clicked() {
                self.add_midi_track(Some(krabton_engine::Instrument::synth("Init")));
                ui.close();
            }
            if ui.button("Add Audio Track").clicked() {
                self.add_audio_track();
                ui.close();
            }
        });
    }

    fn scene_cell(&mut self, ui: &mut Ui, s: usize, r: Rect) {
        if s >= self.project.scenes.len() {
            return;
        }
        let name = self.project.scenes[s].name.clone();
        let selected = self.sel_scene == s;
        let resp = ui.interact(r, Id::new(("scene", s)), Sense::click());
        let in_tri = resp.hover_pos().or(resp.interact_pointer_pos()).map(|p| p.x < r.left() + TRI_W).unwrap_or(false);
        let p = ui.painter();
        let fill = if selected { PANEL3 } else { PANEL2 };
        rect_fill(p, r, 2.0, if resp.hovered() { shade(fill, 1.2) } else { fill });
        play_icon(p, pos2(r.left() + TRI_W * 0.5, r.center().y), 10.0, if resp.hovered() && in_tri { PLAY } else { TEXT });
        text_in(p, Rect::from_min_max(pos2(r.left() + TRI_W, r.top()), r.max), &name, small(), TEXT);
        if selected {
            rect_line(p, r, 2.0, 1.0, ACCENT);
        }
        if resp.clicked() {
            self.sel_scene = s;
            if in_tri {
                self.cmd(Command::LaunchScene(s));
            }
        }
        if resp.double_clicked() && !in_tri {
            self.renaming = Some((RenameTarget::Scene(s), name.clone()));
        }
        resp.context_menu(|ui| {
            if ui.button("Launch Scene     Enter").clicked() {
                self.cmd(Command::LaunchScene(s));
                ui.close();
            }
            if ui.button("Rename").clicked() {
                self.renaming = Some((RenameTarget::Scene(s), name.clone()));
                ui.close();
            }
            if ui.button("Insert Scene     Ctrl+I").clicked() {
                self.deferred.push(Deferred::InsertScene(s + 1));
                ui.close();
            }
            if ui.button("Duplicate Scene").clicked() {
                self.deferred.push(Deferred::DuplicateScene(s));
                ui.close();
            }
            if ui.button("Delete Scene").clicked() {
                self.deferred.push(Deferred::DeleteScene(s));
                ui.close();
            }
        });
    }

    fn track_strip(&mut self, ui: &mut Ui, i: usize, r: Rect) {
        if i >= self.project.tracks.len() {
            return;
        }
        let id = self.project.tracks[i].id;
        let color = rgb(self.project.tracks[i].color);
        let selected = self.sel_track == TrackSel::Track(i);
        let p = ui.painter().clone();
        rect_fill(&p, r, 0.0, if selected { shade(PANEL2, 1.15) } else { PANEL2 });
        rect_fill(&p, Rect::from_min_size(r.min, vec2(r.width(), 3.0)), 0.0, color);
        let bg = ui.interact(r, Id::new(("strip_bg", id)), Sense::click());
        if bg.clicked() {
            self.sel_track = TrackSel::Track(i);
            self.show_devices();
        }
        if let Some(item) = drop_payload::<BrowserItem>(&bg) {
            self.apply_item(&item, DropTarget::Track(i));
        }

        // Sends (left out when the mixer is short).
        let compact = r.height() < COMPACT_BELOW;
        let n_ret = if compact { 0 } else { self.project.returns.len().min(4) };
        let knob = 24.0;
        for j in 0..n_ret {
            let c = pos2(r.left() + 16.0 + j as f32 * 27.0, r.top() + 22.0);
            let kr = Rect::from_center_size(c, vec2(knob, knob));
            let v = self.project.tracks[i].mixer.sends.get(j).copied().unwrap_or(0.0);
            let (resp, new) = knob_widget(ui, kr, Id::new(("send", id, j)), v, 0.0, false, shade(rgb(self.project.returns[j].color), 0.9));
            if let Some(n) = new {
                if let Some(s) = self.project.tracks[i].mixer.sends.get_mut(j) {
                    *s = n;
                }
            }
            if let Some(now) = self.envelope_now(i, ModTarget::Send { track: id, send: j as u16 }) {
                knob_auto_mark(&p, kr, now);
            }
            text(&p, pos2(c.x, kr.bottom() + 6.0), Align2::CENTER_CENTER, (b'A' + j as u8) as char, small(), TEXT_DIM);
            resp.on_hover_text(format!("Send {} : {:.0} %", (b'A' + j as u8) as char, v * 100.0));
        }

        // Pan.
        if !compact {
            let pan_c = pos2(r.center().x, r.top() + 66.0);
            let pr = Rect::from_center_size(pan_c, vec2(26.0, 26.0));
            let pan = self.project.tracks[i].mixer.pan;
            let (resp, new) = knob_widget(ui, pr, Id::new(("pan", id)), (pan + 1.0) * 0.5, 0.5, true, ACCENT);
            if let Some(n) = new {
                self.project.tracks[i].mixer.pan = (n * 2.0 - 1.0).clamp(-1.0, 1.0);
            }
            if let Some(now) = self.envelope_now(i, ModTarget::Pan(id)) {
                knob_auto_mark(&p, pr, now);
            }
            resp.on_hover_text(pan_text(pan));
            text(&p, pos2(pan_c.x, pr.bottom() + 6.0), Align2::CENTER_CENTER, pan_text(pan), small(), TEXT_DIM);
        }

        // Fader + meter.
        let area = Rect::from_min_max(pos2(r.left() + 6.0, r.top() + if compact { 8.0 } else { 94.0 }), pos2(r.right() - 6.0, r.bottom() - 30.0));
        let mut vol = self.project.tracks[i].mixer.volume_db;
        let fr = Rect::from_min_size(pos2(area.center().x - 22.0, area.top()), vec2(22.0, area.height()));
        if fader(ui, fr, Id::new(("vol", id)), &mut vol, color) {
            self.project.tracks[i].mixer.volume_db = vol;
        }
        if let Some(now) = self.envelope_now(i, ModTarget::Volume(id)) {
            fader_auto_mark(&p, fr, now);
        }
        let (ml, mr) = self.meters.get(i).copied().unwrap_or((0.0, 0.0));
        meter(&p, Rect::from_min_size(pos2(area.center().x + 6.0, area.top() + 4.0), vec2(9.0, area.height() - 8.0)), ml, mr);
        let db_txt = if fader_gain(vol) == 0.0 { "-inf".to_string() } else { format!("{vol:.1}") };
        text(&p, pos2(area.right() - 2.0, area.top() + 4.0), Align2::RIGHT_TOP, db_txt, small(), TEXT_DIM);

        // Activator / solo / arm.
        let by = r.bottom() - 25.0;
        let bw = (r.width() - 14.0) / 3.0;
        let mixer = &self.project.tracks[i].mixer;
        let (mute, solo, arm, kind) = (mixer.mute, mixer.solo, self.project.tracks[i].arm, self.project.tracks[i].kind);
        let b1 = Rect::from_min_size(pos2(r.left() + 4.0, by), vec2(bw, 18.0));
        if button(ui, b1, Id::new(("act", id)), &format!("{}", i + 1), !mute, ACCENT).on_hover_text("Track activator (mute)").clicked() {
            self.project.tracks[i].mixer.mute = !mute;
        }
        let b2 = Rect::from_min_size(pos2(b1.right() + 3.0, by), vec2(bw, 18.0));
        let solo_resp = button(ui, b2, Id::new(("solo", id)), "S", solo, SOLO).on_hover_text("Solo (Ctrl+click for multi-solo)");
        if solo_resp.clicked() {
            let multi = ui.input(|i| i.modifiers.ctrl || i.modifiers.command);
            if !multi {
                for (k, t) in self.project.tracks.iter_mut().enumerate() {
                    if k != i {
                        t.mixer.solo = false;
                    }
                }
            }
            self.project.tracks[i].mixer.solo = !solo;
        }
        let b3 = Rect::from_min_size(pos2(b2.right() + 3.0, by), vec2(bw, 18.0));
        // MIDI tracks record notes, audio tracks record their input.
        if kind == TrackKind::Midi || kind == TrackKind::Audio {
            let resp = button(ui, b3, Id::new(("arm", id)), "", arm, REC).on_hover_text("Arm for recording (Ctrl+click for more; with several tracks selected, all of them)");
            rec_icon(ui.painter(), b3.center(), 8.0, if arm { TEXT_DARK } else { REC });
            if resp.clicked() {
                let multi = ui.input(|i| i.modifiers.ctrl || i.modifiers.command);
                self.click_arm(i, multi);
            }
        }
    }

    fn return_strip(&mut self, ui: &mut Ui, j: usize, r: Rect) {
        if j >= self.project.returns.len() {
            return;
        }
        let ret_id = self.project.returns[j].id;
        let color = rgb(self.project.returns[j].color);
        let selected = self.sel_track == TrackSel::Return(j);
        let p = ui.painter().clone();
        rect_fill(&p, r, 0.0, if selected { shade(PANEL2, 1.15) } else { shade(PANEL2, 0.9) });
        rect_fill(&p, Rect::from_min_size(r.min, vec2(r.width(), 3.0)), 0.0, color);
        let bg = ui.interact(r, Id::new(("ret_bg", ret_id)), Sense::click());
        if bg.clicked() {
            self.sel_track = TrackSel::Return(j);
            self.show_devices();
        }
        if let Some(item) = drop_payload::<BrowserItem>(&bg) {
            self.apply_item(&item, DropTarget::Chain(TrackSel::Return(j)));
        }
        let compact = r.height() < COMPACT_BELOW;
        if !compact {
            let pan_c = pos2(r.center().x, r.top() + 66.0);
            let pr = Rect::from_center_size(pan_c, vec2(26.0, 26.0));
            let pan = self.project.returns[j].mixer.pan;
            let (_, new) = knob_widget(ui, pr, Id::new(("rpan", ret_id)), (pan + 1.0) * 0.5, 0.5, true, ACCENT);
            if let Some(n) = new {
                self.project.returns[j].mixer.pan = n * 2.0 - 1.0;
            }
            text(&p, pos2(pan_c.x, pr.bottom() + 6.0), Align2::CENTER_CENTER, pan_text(pan), small(), TEXT_DIM);
        }
        let area = Rect::from_min_max(pos2(r.left() + 6.0, r.top() + if compact { 8.0 } else { 94.0 }), pos2(r.right() - 6.0, r.bottom() - 30.0));
        let mut vol = self.project.returns[j].mixer.volume_db;
        let fr = Rect::from_min_size(pos2(area.center().x - 22.0, area.top()), vec2(22.0, area.height()));
        if fader(ui, fr, Id::new(("rvol", ret_id)), &mut vol, color) {
            self.project.returns[j].mixer.volume_db = vol;
        }
        let (ml, mr) = self.return_meters.get(j).copied().unwrap_or((0.0, 0.0));
        meter(&p, Rect::from_min_size(pos2(area.center().x + 6.0, area.top() + 4.0), vec2(9.0, area.height() - 8.0)), ml, mr);
        let by = r.bottom() - 25.0;
        let mute = self.project.returns[j].mixer.mute;
        let b = Rect::from_min_size(pos2(r.left() + 4.0, by), vec2(r.width() - 8.0, 18.0));
        let label = format!("{}", (b'A' + j as u8) as char);
        if button(ui, b, Id::new(("ract", ret_id)), &label, !mute, ACCENT).clicked() {
            self.project.returns[j].mixer.mute = !mute;
        }
    }

    fn master_strip(&mut self, ui: &mut Ui, r: Rect) {
        let p = ui.painter().clone();
        let selected = self.sel_track == TrackSel::Master;
        rect_fill(&p, r, 0.0, if selected { shade(PANEL2, 1.15) } else { PANEL2 });
        rect_fill(&p, Rect::from_min_size(r.min, vec2(r.width(), 3.0)), 0.0, ACCENT);
        let bg = ui.interact(r, Id::new("master_bg"), Sense::click());
        if bg.clicked() {
            self.sel_track = TrackSel::Master;
            self.show_devices();
        }
        if let Some(item) = drop_payload::<BrowserItem>(&bg) {
            self.apply_item(&item, DropTarget::Chain(TrackSel::Master));
        }
        text(&p, pos2(r.center().x, r.top() + 18.0), Align2::CENTER_CENTER, "Master", body(), TEXT);
        let area = Rect::from_min_max(pos2(r.left() + 10.0, r.top() + 40.0), pos2(r.right() - 10.0, r.bottom() - 30.0));
        let mut vol = self.project.master.volume_db;
        let fr = Rect::from_min_size(pos2(area.center().x - 26.0, area.top()), vec2(24.0, area.height()));
        if fader(ui, fr, Id::new("master_vol"), &mut vol, ACCENT) {
            self.project.master.volume_db = vol;
        }
        let (ml, mr) = self.master_meter;
        meter(&p, Rect::from_min_size(pos2(area.center().x + 6.0, area.top() + 4.0), vec2(14.0, area.height() - 8.0)), ml, mr);
        let db_txt = if fader_gain(vol) == 0.0 { "-inf dB".to_string() } else { format!("{vol:.1} dB") };
        text(&p, pos2(r.center().x, r.bottom() - 16.0), Align2::CENTER_CENTER, db_txt, small(), TEXT_DIM);
    }
}

fn knob_widget(ui: &mut Ui, r: Rect, id: Id, norm: f32, default: f32, bipolar: bool, color: egui::Color32) -> (egui::Response, Option<f32>) {
    knob(ui, r, id, norm, default, bipolar, color)
}

pub fn pan_text(pan: f32) -> String {
    let p = (pan * 50.0).round() as i32;
    match p {
        0 => "C".into(),
        p if p < 0 => format!("{}L", -p),
        p => format!("{p}R"),
    }
}

/// The mark of a clip that keeps its track's sound: a little knob.
pub(crate) fn sound_mark(p: &egui::Painter, c: egui::Pos2, col: egui::Color32) {
    let stroke = Stroke::new(1.4_f32, col);
    p.circle_stroke(c, 4.5, stroke);
    p.line_segment([c, c + vec2(2.4, -2.6)], stroke);
    for a in [0.75f32, 2.25] {
        let d = vec2((a * std::f32::consts::PI).cos(), (a * std::f32::consts::PI).sin());
        p.line_segment([c + d * 6.0, c + d * 7.5], stroke);
    }
}

/// What a clip menu offers for the sound a clip keeps.
pub(crate) fn sound_menu(app: &mut App, ui: &mut Ui, sel: ClipSel, keeps: bool) {
    let label = if keeps { "Keep the Sound Anew" } else { "Keep the Track's Sound" };
    let tip = "The clip remembers the track's instrument, MIDI effects and audio effects with all their settings: when it starts, the track plays with that sound (on the very beat), and while it plays what you change is kept in it";
    if ui.button(label).on_hover_text(tip).clicked() {
        app.keep_sound_in_clip(sel);
        ui.close();
    }
    if keeps && ui.button("Forget Its Sound").on_hover_text("The clip plays with whatever sound the track has").clicked() {
        app.forget_clip_sound(sel);
        ui.close();
    }
}

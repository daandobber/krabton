//! Arrangement view: a timeline with one lane per track.

use crate::app::{App, ArrClipDrag, BrowserItem, ClipSel, DetailTab, DropTarget, RenameTarget, SlotDrag, TrackSel};
use crate::theme::*;
use crate::ui::session::color_menu;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, CursorIcon, Id, Rect, Sense, Stroke, Ui, UiBuilder};
use krabton_engine::{ArrangementClip, Clip, ClipContent, Command, TrackKind};
use std::sync::atomic::Ordering;

const HEAD_W: f32 = 168.0;
const RULER_H: f32 = 26.0;
const TOOLBAR_H: f32 = 28.0;
/// The arranger track (song sections).
const SECTION_H: f32 = 20.0;
const LANE_H: f32 = 62.0;
const EDGE: f32 = 6.0;

#[derive(Clone, Copy, PartialEq, Eq)]
enum DragKind {
    Move,
    ResizeEnd,
    ResizeStart,
}

#[derive(Clone, Copy)]
struct ArrDrag {
    track: usize,
    orig_track: usize,
    id: u64,
    kind: DragKind,
    orig_start: f64,
    orig_len: f64,
    orig_offset: f64,
    press_x: f32,
}

#[derive(Clone, Copy)]
enum LoopDrag {
    Start,
    End,
    Move { orig_start: f64, orig_end: f64, press_x: f32 },
}

/// A section being moved or copied on the arranger track.
#[derive(Clone, Copy)]
struct SectionDrag {
    id: u64,
    /// Beats from the section's start to where it was grabbed.
    grab: f64,
    copy: bool,
}

pub struct ArrState {
    pub px_per_beat: f32,
    pub scroll_x: f32,
    pub scroll_y: f32,
    pub grid: f64,
    pub follow: bool,
    drag: Option<ArrDrag>,
    loop_drag: Option<LoopDrag>,
    /// Top of the first lane on screen (for dragging clips between tracks).
    lane_top: f32,
    sec_drag: Option<SectionDrag>,
    pub sel_section: Option<u64>,
    /// Automation mode: the lines over the clips, and the lanes under them.
    pub(crate) automation: bool,
    /// Paint the automation along the grid instead of placing points.
    pub(crate) auto_draw: bool,
    pub(crate) auto_drag: Option<crate::ui::automation::AutoDrag>,
    /// Where each track starts (from the first), and the end of the last.
    pub(crate) tops: Vec<f32>,
}

impl Default for ArrState {
    fn default() -> Self {
        Self {
            px_per_beat: 18.0,
            scroll_x: 0.0,
            scroll_y: 0.0,
            grid: 4.0,
            follow: true,
            drag: None,
            loop_drag: None,
            lane_top: 0.0,
            sec_drag: None,
            sel_section: None,
            automation: false,
            auto_draw: false,
            auto_drag: None,
            tops: Vec::new(),
        }
    }
}

fn snap(v: f64, grid: f64, free: bool) -> f64 {
    if free || grid <= 0.0 {
        v
    } else {
        (v / grid).round() * grid
    }
}

impl App {
    /// Puts a clip that was being dragged back where the drag started (it
    /// ended in the session view, where it was copied to a slot).
    pub(crate) fn arr_cancel_drag(&mut self) {
        let Some(d) = self.arr.drag.take() else { return };
        let Some(t) = self.project.tracks.iter().position(|t| t.arrangement.iter().any(|a| a.id == d.id)) else { return };
        let Some(at) = self.project.tracks[t].arrangement.iter().position(|a| a.id == d.id) else { return };
        let mut ac = self.project.tracks[t].arrangement.remove(at);
        ac.start = d.orig_start;
        ac.length = d.orig_len;
        ac.offset = d.orig_offset;
        let back = d.orig_track.min(self.project.tracks.len() - 1);
        self.project.tracks[back].arrangement.push(ac);
    }

    pub(crate) fn ui_arrangement(&mut self, ui: &mut Ui) {
        let full = ui.available_rect_before_wrap();
        ui.allocate_rect(full, Sense::hover());
        let status = self.status();
        let bpb = self.project.beats_per_bar_f();
        let nt = self.project.tracks.len();
        let head_w = crate::prefs::Layout::or(self.prefs.layout.arr_head_w, HEAD_W).clamp(100.0, 420.0);
        let lane_h = self.lane_h();

        let toolbar = Rect::from_min_size(full.min, vec2(full.width(), TOOLBAR_H));
        // The arranger track (song sections) sits between the toolbar and the ruler.
        let arranger = Rect::from_min_max(pos2(full.left() + head_w, toolbar.bottom()), pos2(full.right(), toolbar.bottom() + SECTION_H));
        let ruler = Rect::from_min_max(pos2(full.left() + head_w, arranger.bottom()), pos2(full.right(), arranger.bottom() + RULER_H));
        let lanes = Rect::from_min_max(pos2(full.left() + head_w, ruler.bottom()), full.max);
        let heads = Rect::from_min_max(pos2(full.left(), ruler.bottom()), pos2(full.left() + head_w, full.bottom()));

        {
            let p = ui.painter();
            rect_fill(p, full, 0.0, BG);
            rect_fill(p, toolbar, 0.0, PANEL);
            rect_fill(p, heads, 0.0, PANEL);
            rect_fill(p, ruler, 0.0, PANEL2);
            rect_fill(p, Rect::from_min_max(pos2(full.left(), toolbar.bottom()), pos2(full.left() + head_w, ruler.bottom())), 0.0, PANEL2);
        }

        // Toolbar (regular egui layout).
        {
            let mut tui = ui.new_child(UiBuilder::new().max_rect(toolbar.shrink2(vec2(6.0, 3.0))).layout(egui::Layout::left_to_right(egui::Align::Center)));
            if tui.button("Scenes to Arrangement").on_hover_text("Append every non-empty scene to the timeline").clicked() {
                let n = self.project.scenes_to_arrangement();
                self.toast(format!("Placed {n} scene(s) in the arrangement"));
            }
            tui.checkbox(&mut self.arr.follow, "Follow");
            tui.label(egui::RichText::new("Grid").small().color(TEXT_DIM));
            let grids = [(bpb, "1 Bar"), (2.0, "1/2"), (1.0, "1/4"), (0.5, "1/8"), (0.25, "1/16"), (0.0, "Off")];
            let cur = grids.iter().find(|g| (g.0 - self.arr.grid).abs() < 1e-9).map(|g| g.1).unwrap_or("1 Bar");
            egui::ComboBox::from_id_salt("arr_grid").width(56.0).selected_text(cur).show_ui(&mut tui, |ui| {
                for (g, name) in grids {
                    ui.selectable_value(&mut self.arr.grid, g, name);
                }
            });
            if tui.button("-").on_hover_text("Zoom out (Ctrl+wheel)").clicked() {
                self.arr.px_per_beat = (self.arr.px_per_beat / 1.4).max(2.0);
            }
            if tui.button("+").on_hover_text("Zoom in").clicked() {
                self.arr.px_per_beat = (self.arr.px_per_beat * 1.4).min(400.0);
            }
            tui.separator();
            let auto = self.arr.automation;
            let a = egui::Button::new(egui::RichText::new("Automation").color(if auto { TEXT_DARK } else { TEXT })).fill(if auto { crate::ui::automation::AUTO } else { PANEL3 });
            if tui.add(a).on_hover_text("Show and draw automation over the clips: pick a parameter in a track's header, + for more lanes").clicked() {
                self.arr.automation = !auto;
            }
            if auto {
                let draw = self.arr.auto_draw;
                if tui.selectable_label(draw, "Draw").on_hover_text("Paint the line along the grid (off: click for points, drag them)").clicked() {
                    self.arr.auto_draw = !draw;
                }
            }
            tui.label(egui::RichText::new("Double-click a MIDI lane to create a clip. Drag edges to resize. Alt = no snap.").small().color(TEXT_DIM));
        }

        // Zoom and scroll.
        if ui.rect_contains_pointer(lanes.union(ruler).union(arranger)) {
            let (d, shift, zoom, pointer) = ui.input(|i| (i.smooth_scroll_delta, i.modifiers.shift, i.zoom_delta(), i.pointer.hover_pos()));
            if zoom != 1.0 {
                let px = pointer.map(|p| p.x).unwrap_or(lanes.left());
                let beat_at = (px - lanes.left() + self.arr.scroll_x) / self.arr.px_per_beat;
                self.arr.px_per_beat = (self.arr.px_per_beat * zoom).clamp(2.0, 400.0);
                self.arr.scroll_x = beat_at * self.arr.px_per_beat - (px - lanes.left());
            } else if shift {
                self.arr.scroll_x -= if d.x != 0.0 { d.x } else { d.y };
            } else {
                self.arr.scroll_x -= d.x;
                self.arr.scroll_y -= d.y;
            }
        } else if ui.rect_contains_pointer(heads) {
            self.arr.scroll_y -= ui.input(|i| i.smooth_scroll_delta.y);
        }
        let playing = status.playing.load(Ordering::Relaxed);
        let song_pos = status.song_pos.load();
        let ppb = self.arr.px_per_beat;
        if self.arr.follow && playing && self.arr.drag.is_none() {
            let x = song_pos as f32 * ppb - self.arr.scroll_x;
            if x > lanes.width() * 0.85 || x < 0.0 {
                self.arr.scroll_x = song_pos as f32 * ppb - lanes.width() * 0.15;
            }
        }
        self.arr.scroll_x = self.arr.scroll_x.max(0.0);
        // Where each track starts: its lane, and in Automation mode its
        // automation lanes under it.
        let auto_h = (lane_h * 0.7).clamp(30.0, 120.0);
        let mut tops = Vec::with_capacity(nt + 1);
        let mut acc = 0.0f32;
        for tr in &self.project.tracks {
            tops.push(acc);
            acc += lane_h + if self.arr.automation { tr.auto_lanes.len() as f32 * auto_h } else { 0.0 };
        }
        tops.push(acc);
        let content_h = acc + 60.0;
        self.arr.scroll_y = self.arr.scroll_y.clamp(0.0, (content_h - lanes.height()).max(0.0));
        let sx = self.arr.scroll_x;
        let sy = self.arr.scroll_y;
        let bx = |beat: f64| lanes.left() + beat as f32 * ppb - sx;
        let xb = |x: f32| ((x - lanes.left() + sx) / ppb) as f64;
        let lane_y = |i: usize| lanes.top() + tops.get(i).copied().unwrap_or(acc) - sy;
        self.arr.lane_top = lane_y(0);
        // A drag that ended elsewhere (e.g. dropped in the session view).
        if !ui.input(|i| i.pointer.any_down()) {
            self.arr.drag = None;
        }

        let p = ui.painter().clone();

        // Grid lines (behind lanes).
        let first_beat = xb(lanes.left()).floor().max(0.0);
        let last_beat = xb(lanes.right()).ceil();
        let lp = p.with_clip_rect(lanes.union(ruler));
        let step = if ppb < 6.0 { bpb * 4.0 } else if ppb < 12.0 { bpb } else { 1.0 };
        let mut b = (first_beat / step).floor() * step;
        while b <= last_beat {
            let x = bx(b);
            let is_bar = (b / bpb).fract().abs() < 1e-9;
            lp.line_segment([pos2(x, lanes.top()), pos2(x, lanes.bottom())], Stroke::new(1.0_f32, if is_bar { LINE_SOFT } else { shade(LINE_SOFT, 0.7) }));
            if is_bar {
                let bar = (b / bpb) as i64 + 1;
                let label_every = if ppb * bpb as f32 > 40.0 { 1 } else if ppb * bpb as f32 > 12.0 { 4 } else { 16 };
                if (bar - 1) % label_every == 0 {
                    lp.line_segment([pos2(x, ruler.bottom() - 8.0), pos2(x, ruler.bottom())], Stroke::new(1.0_f32, TEXT_DIM));
                    lp.text(pos2(x + 3.0, ruler.bottom() - 4.0), Align2::LEFT_BOTTOM, bar, small(), TEXT_DIM);
                }
            }
            b += step;
        }

        // Loop brace + ruler interaction.
        self.ruler(ui, ruler, &bx, &xb);
        ui.painter().text(pos2(full.left() + 8.0, arranger.center().y), Align2::LEFT_CENTER, "Arranger", small(), TEXT_DIM);
        self.arranger_track(ui, arranger, &bx, &xb);

        // Lanes.
        let mut lui = ui.new_child(UiBuilder::new().max_rect(lanes));
        lui.set_clip_rect(lanes.intersect(ui.clip_rect()));
        let mut hui = ui.new_child(UiBuilder::new().max_rect(heads));
        hui.set_clip_rect(heads.intersect(ui.clip_rect()));
        let free = ui.input(|i| i.modifiers.alt);
        for i in 0..nt {
            let y = lane_y(i);
            let lane = Rect::from_min_size(pos2(lanes.left(), y), vec2(lanes.width(), lane_h - 1.0));
            if !lane.intersects(lanes) {
                continue;
            }
            let head = Rect::from_min_size(pos2(heads.left(), y), vec2(head_w - 1.0, lane_h - 1.0));
            self.arr_head(&mut hui, i, head);
            self.arr_lane(&mut lui, i, lane, &bx, &xb, free);
            // The automation lanes under it.
            if self.arr.automation {
                let n = self.project.tracks.get(i).map(|t| t.auto_lanes.len()).unwrap_or(0);
                for k in 0..n {
                    let ky = y + lane_h + k as f32 * auto_h;
                    let kl = Rect::from_min_size(pos2(lanes.left(), ky), vec2(lanes.width(), auto_h - 1.0));
                    let kh = Rect::from_min_size(pos2(heads.left(), ky), vec2(head_w - 1.0, auto_h - 1.0));
                    if !kl.intersects(lanes) {
                        continue;
                    }
                    lui.painter().rect_filled(kl, 0.0, PANEL2);
                    lui.painter().line_segment([pos2(kl.left(), kl.bottom() + 0.5), pos2(kl.right(), kl.bottom() + 0.5)], Stroke::new(1.0_f32, LINE_SOFT));
                    self.auto_lane_head(&mut hui, i, k, kh);
                    if let Some(target) = self.project.tracks.get(i).and_then(|t| t.auto_lanes.get(k)).copied() {
                        self.auto_lane(&mut lui, i, target, kl, &bx, &xb, free, false);
                    }
                }
            }
        }
        self.arr.tops = tops.clone();
        if nt == 0 {
            p.text(lanes.center(), Align2::CENTER_CENTER, "No tracks yet: drop an instrument from the browser", body(), TEXT_DIM);
        }
        // The master, always at the bottom of the headers: click for its
        // effects, or drop effects on it.
        {
            let mr = Rect::from_min_max(pos2(heads.left(), heads.bottom() - 26.0), pos2(heads.right() - 4.0, heads.bottom() - 2.0));
            let resp = hui.interact(mr, Id::new("arr_master"), Sense::click());
            let selected = self.sel_track == TrackSel::Master;
            let hp = hui.painter().clone();
            rect_fill(&hp, mr, 3.0, if selected || resp.hovered() { shade(PANEL3, 1.25) } else { PANEL3 });
            if resp.dnd_hover_payload::<BrowserItem>().is_some() {
                rect_line(&hp, mr, 3.0, 1.5, ACCENT);
            }
            rect_fill(&hp, Rect::from_min_size(mr.min, vec2(3.0, mr.height())), 1.0, ACCENT);
            hp.text(pos2(mr.left() + 10.0, mr.center().y), Align2::LEFT_CENTER, "Master", body(), if selected { ACCENT } else { TEXT });
            let n = self.project.master.devices.len();
            if n > 0 {
                hp.text(pos2(mr.right() - 6.0, mr.center().y), Align2::RIGHT_CENTER, format!("{n} fx"), small(), TEXT_DIM);
            }
            if resp.clicked() {
                self.sel_track = TrackSel::Master;
                self.show_devices();
            }
            if let Some(item) = drop_payload::<BrowserItem>(&resp) {
                self.apply_item(&item, DropTarget::Chain(TrackSel::Master));
                self.sel_track = TrackSel::Master;
            }
            resp.on_hover_text("The master: click for its effects, or drop effects here");
        }
        // Drop area below the last lane creates a new track.
        let below = Rect::from_min_max(pos2(lanes.left(), lane_y(nt)), pos2(lanes.right(), lane_y(nt) + 50.0));
        if below.intersects(lanes) {
            let resp = lui.interact(below, Id::new("arr_new_track"), Sense::hover());
            if resp.dnd_hover_payload::<BrowserItem>().is_some() {
                rect_line(&p, below, 3.0, 1.0, ACCENT);
            }
            if let Some(item) = drop_payload::<BrowserItem>(&resp) {
                self.apply_item(&item, DropTarget::NewTrack);
            }
        }

        // Loop region shading and playhead.
        let pc = p.with_clip_rect(lanes.union(ruler));
        if self.project.loop_enabled {
            let l0 = bx(self.project.loop_start);
            let l1 = bx(self.project.loop_end);
            pc.rect_filled(Rect::from_min_max(pos2(l0, lanes.top()), pos2(l1, lanes.bottom())), 0.0, egui::Color32::from_rgba_unmultiplied(255, 167, 51, 10));
        }
        let px = bx(song_pos);
        pc.line_segment([pos2(px, ruler.top()), pos2(px, lanes.bottom())], Stroke::new(1.5_f32, if playing { egui::Color32::WHITE } else { TEXT_DIM }));
        pc.add(egui::Shape::convex_polygon(vec![pos2(px - 5.0, ruler.top()), pos2(px + 5.0, ruler.top()), pos2(px, ruler.top() + 7.0)], TEXT, Stroke::NONE));
        if self.arr.drag.is_some() && !ui.input(|i| i.pointer.any_down()) {
            self.arr.drag = None;
        }

        // Dividers: the right edge of the track headers, and the bottom of
        // every lane (all lanes share one height).
        let split = Rect::from_min_max(pos2(heads.right() - 3.0, heads.top()), pos2(heads.right() + 3.0, heads.bottom()));
        let resp = splitter(ui, Id::new("arr_head_split"), split, true);
        if resp.dragged() {
            self.prefs.layout.arr_head_w = (head_w + resp.drag_delta().x).clamp(100.0, 420.0);
            self.layout_dirty = true;
        }
        for i in 0..nt {
            let y = lane_y(i) + lane_h - 1.0;
            if y < heads.top() || y > heads.bottom() {
                continue;
            }
            let r = Rect::from_min_max(pos2(heads.left(), y - 3.0), pos2(heads.right() - 4.0, y + 3.0));
            let resp = splitter(ui, Id::new(("arr_lane_split", i)), r, false);
            if resp.dragged() {
                self.prefs.layout.lane_h = (lane_h + resp.drag_delta().y / (i + 1) as f32).clamp(28.0, 240.0);
                self.layout_dirty = true;
            }
        }
    }

    /// The arranger track: song sections (Studio One style). Double-click
    /// to add one; drag one to move it with everything in it, Ctrl+drag to
    /// copy it; drag its right edge to resize it.
    fn arranger_track(&mut self, ui: &mut Ui, r: Rect, bx: &dyn Fn(f64) -> f32, xb: &dyn Fn(f32) -> f64) {
        let bpb = self.project.beats_per_bar_f();
        let p = ui.painter_at(r);
        rect_fill(&p, r, 0.0, PANEL);
        p.line_segment([r.left_bottom(), r.right_bottom()], Stroke::new(1.0_f32, LINE));
        let sections = self.project.sections.clone();
        // Bars, pulled onto section edges when close.
        let edges: Vec<f64> = sections.iter().flat_map(|s| [s.start, s.end()]).collect();
        let snap = |b: f64| {
            let bar = ((b / bpb).round() * bpb).max(0.0);
            edges.iter().copied().filter(|e| (e - b).abs() < bpb * 0.5).min_by(|x, y| (x - b).abs().total_cmp(&(y - b).abs())).unwrap_or(bar)
        };
        let bg = ui.interact(r, Id::new("arranger_bg"), Sense::click());
        if bg.double_clicked() {
            if let Some(pos) = bg.interact_pointer_pos() {
                let at = ((xb(pos.x) / bpb).floor() * bpb).max(0.0);
                let next = sections.iter().map(|s| s.start).filter(|&s| s > at).fold(f64::MAX, f64::min);
                let len = (4.0 * bpb).min(next - at);
                if len >= bpb && !sections.iter().any(|s| at >= s.start && at < s.end()) {
                    self.arr.sel_section = Some(self.project.add_section(at, len));
                }
            }
        }
        if sections.is_empty() {
            p.text(pos2(r.left() + 8.0, r.center().y), Align2::LEFT_CENTER, "Double-click to add a section (Intro, Verse, Chorus...)", small(), TEXT_DIM);
        }
        let mut action: Option<(u64, &str)> = None;
        for s in &sections {
            let sr = Rect::from_min_max(pos2(bx(s.start), r.top() + 2.0), pos2(bx(s.end()), r.bottom() - 2.0));
            if !sr.intersects(r) {
                continue;
            }
            let body = ui.interact(sr, Id::new(("section", s.id)), Sense::click_and_drag());
            let edge = ui.interact(Rect::from_min_max(pos2(sr.right() - 5.0, sr.top()), sr.max), Id::new(("section_edge", s.id)), Sense::drag());
            let selected = self.arr.sel_section == Some(s.id);
            let c = rgb(s.color);
            p.rect_filled(sr, 3.0, if selected { c } else { shade(c, 0.7) });
            if selected {
                rect_line(&p, sr, 3.0, 1.5, egui::Color32::WHITE);
            }
            p.with_clip_rect(sr.shrink(2.0)).text(pos2(sr.left() + 5.0, sr.center().y), Align2::LEFT_CENTER, &s.name, small(), text_on(c));
            if edge.hovered() || edge.dragged() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
            }
            if edge.dragged() {
                if let Some(pos) = edge.interact_pointer_pos() {
                    let next = sections.iter().map(|x| x.start).filter(|&x| x > s.start).fold(f64::MAX, f64::min);
                    let end = ((xb(pos.x) / bpb).round() * bpb).clamp(s.start + bpb, next);
                    if let Some(x) = self.project.sections.iter_mut().find(|x| x.id == s.id) {
                        x.length = end - x.start;
                    }
                }
            } else if body.drag_started() {
                let grab = ui.input(|i| i.pointer.press_origin()).map(|o| xb(o.x) - s.start).unwrap_or(0.0);
                let copy = ui.input(|i| i.modifiers.ctrl || i.modifiers.command);
                self.arr.sec_drag = Some(SectionDrag { id: s.id, grab, copy });
                self.arr.sel_section = Some(s.id);
            }
            if body.clicked() {
                self.arr.sel_section = Some(s.id);
            }
            if body.double_clicked() {
                self.renaming = Some((RenameTarget::Section(s.id), s.name.clone()));
            }
            body.context_menu(|ui| {
                for (label, what) in [("Rename", "rename"), ("Loop this section", "loop"), ("Duplicate", "dup"), ("Delete section (keep the music)", "del"), ("Delete with its music", "delall"), ("Next colour", "color")] {
                    if ui.button(label).clicked() {
                        action = Some((s.id, what));
                        ui.close();
                    }
                }
            });
            body.on_hover_text(format!("{}: bars {} to {}. Drag to move it with its music, Ctrl+drag to copy, right-click for more.", s.name, (s.start / bpb) as i64 + 1, (s.end() / bpb) as i64));
        }
        if let Some((id, what)) = action {
            let Some(s) = sections.iter().find(|s| s.id == id) else { return };
            match what {
                "rename" => self.renaming = Some((RenameTarget::Section(id), s.name.clone())),
                "loop" => {
                    self.project.loop_start = s.start;
                    self.project.loop_end = s.end();
                    self.project.loop_enabled = true;
                }
                "dup" => {
                    self.arr.sel_section = self.project.copy_section(id, None);
                }
                "del" => self.project.delete_section(id, false),
                "delall" => self.project.delete_section(id, true),
                _ => {
                    let palette = [[0x5b, 0x8d, 0xd9], [0x4c, 0xb8, 0x7a], [0xe0, 0x8a, 0x3c], [0xb0, 0x6c, 0xd8], [0xd8, 0x5a, 0x7a], [0x3c, 0xb4, 0xc0]];
                    if let Some(x) = self.project.sections.iter_mut().find(|x| x.id == id) {
                        let k = palette.iter().position(|c| *c == x.color).map(|k| k + 1).unwrap_or(0);
                        x.color = palette[k % palette.len()];
                    }
                }
            }
        }
        // Moving or copying: an outline where it would land, done on release.
        if let Some(d) = self.arr.sec_drag {
            let Some(s) = sections.iter().find(|s| s.id == d.id) else {
                self.arr.sec_drag = None;
                return;
            };
            if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
                let to = snap(xb(pos.x) - d.grab);
                let ghost = Rect::from_min_max(pos2(bx(to), r.top() + 1.0), pos2(bx(to + s.length), r.bottom() - 1.0));
                let fg = ui.ctx().layer_painter(egui::LayerId::new(egui::Order::Foreground, Id::new("section_ghost")));
                rect_line(&fg, ghost, 3.0, 2.0, if d.copy { PLAY } else { ACCENT });
                fg.text(ghost.left_top() + vec2(4.0, -2.0), Align2::LEFT_BOTTOM, if d.copy { "copy here" } else { "move here" }, small(), TEXT);
                if !ui.input(|i| i.pointer.any_down()) {
                    if d.copy {
                        self.arr.sel_section = self.project.copy_section(d.id, Some(to));
                    } else {
                        self.project.move_section(d.id, to);
                    }
                    self.arr.sec_drag = None;
                }
            } else {
                self.arr.sec_drag = None;
            }
        }
    }

    /// Height of the arrangement's lanes.
    pub(crate) fn lane_h(&self) -> f32 {
        crate::prefs::Layout::or(self.prefs.layout.lane_h, LANE_H).clamp(28.0, 240.0)
    }

    fn ruler(&mut self, ui: &mut Ui, ruler: Rect, bx: &dyn Fn(f64) -> f32, xb: &dyn Fn(f32) -> f64) {
        let p = ui.painter().with_clip_rect(ruler);
        let brace = Rect::from_min_max(pos2(bx(self.project.loop_start), ruler.top() + 2.0), pos2(bx(self.project.loop_end), ruler.top() + 10.0));
        let col = if self.project.loop_enabled { ACCENT } else { TEXT_DIM };
        rect_fill(&p, brace, 2.0, shade(col, 0.8));
        let bpb = self.project.beats_per_bar_f();

        let brace_hit = brace.expand2(vec2(EDGE, 2.0));
        let bresp = ui.interact(brace_hit.intersect(ruler), Id::new("loop_brace"), Sense::click_and_drag());
        let hover_x = bresp.hover_pos().map(|p| p.x);
        if let Some(x) = hover_x {
            if (x - brace.left()).abs() < EDGE || (x - brace.right()).abs() < EDGE {
                ui.ctx().set_cursor_icon(CursorIcon::ResizeHorizontal);
            }
        }
        if bresp.drag_started() {
            let x = ui.input(|i| i.pointer.press_origin()).map(|p| p.x).unwrap_or(0.0);
            self.arr.loop_drag = Some(if (x - brace.left()).abs() < EDGE {
                LoopDrag::Start
            } else if (x - brace.right()).abs() < EDGE {
                LoopDrag::End
            } else {
                LoopDrag::Move { orig_start: self.project.loop_start, orig_end: self.project.loop_end, press_x: x }
            });
        }
        if bresp.dragged() {
            if let (Some(d), Some(pos)) = (self.arr.loop_drag, bresp.interact_pointer_pos()) {
                let grid = if self.arr.grid > 0.0 { self.arr.grid } else { 1.0 };
                match d {
                    LoopDrag::Start => self.project.loop_start = snap(xb(pos.x), grid, false).clamp(0.0, self.project.loop_end - grid),
                    LoopDrag::End => self.project.loop_end = snap(xb(pos.x), grid, false).max(self.project.loop_start + grid),
                    LoopDrag::Move { orig_start, orig_end, press_x } => {
                        let d = snap((pos.x - press_x) as f64 / self.arr.px_per_beat as f64, grid, false);
                        let d = d.max(-orig_start);
                        self.project.loop_start = orig_start + d;
                        self.project.loop_end = orig_end + d;
                    }
                }
            }
        }
        if bresp.drag_stopped() {
            self.arr.loop_drag = None;
        }
        if bresp.double_clicked() {
            self.project.loop_enabled = !self.project.loop_enabled;
        }
        bresp.on_hover_text("Loop brace: drag to move, drag edges to resize, double-click to toggle");

        let below = Rect::from_min_max(pos2(ruler.left(), brace_hit.bottom().min(ruler.bottom())), ruler.max);
        let resp = ui.interact(below, Id::new("ruler"), Sense::click_and_drag());
        if resp.clicked() || resp.dragged() {
            if let Some(pos) = resp.interact_pointer_pos() {
                let grid = if self.arr.grid > 0.0 { self.arr.grid.min(bpb) } else { 0.0 };
                let beat = snap(xb(pos.x).max(0.0), grid, ui.input(|i| i.modifiers.alt));
                self.cmd(Command::SetPosition(beat));
            }
        }
        resp.context_menu(|ui| {
            let beat = ui.ctx().input(|i| i.pointer.press_origin()).map(|p| xb(p.x)).unwrap_or(0.0);
            let beat = (beat / bpb).round() * bpb;
            if ui.button("Set Loop Start Here").clicked() {
                self.project.loop_start = beat.min(self.project.loop_end - 1.0).max(0.0);
                self.project.loop_enabled = true;
                ui.close();
            }
            if ui.button("Set Loop End Here").clicked() {
                self.project.loop_end = beat.max(self.project.loop_start + 1.0);
                self.project.loop_enabled = true;
                ui.close();
            }
        });
    }

    fn arr_head(&mut self, ui: &mut Ui, i: usize, r: Rect) {
        if i >= self.project.tracks.len() {
            return;
        }
        let t = &self.project.tracks[i];
        let id = t.id;
        let selected = self.sel_track == TrackSel::Track(i);
        let color = rgb(t.color);
        let name = t.name.clone();
        let (mute, solo, arm, kind) = (t.mixer.mute, t.mixer.solo, t.arm, t.kind);
        let p = ui.painter().clone();
        let resp = ui.interact(r, Id::new(("arr_head", id)), Sense::click());
        rect_fill(&p, r, 0.0, if selected { PANEL3 } else { PANEL2 });
        rect_fill(&p, Rect::from_min_size(r.min, vec2(5.0, r.height())), 0.0, color);
        text_in(&p, Rect::from_min_max(pos2(r.left() + 8.0, r.top() + 2.0), pos2(r.right() - 4.0, r.top() + 22.0)), &name, body(), TEXT);
        if self.arr.automation {
            // What the line over the clips shows, and + for another lane.
            let cr = Rect::from_min_max(pos2(r.left() + 8.0, r.top() + 20.0), pos2(r.right() - 40.0, r.top() + 38.0));
            let cur = self.project.tracks[i].auto_shown;
            if let Some(pick) = self.auto_chooser(ui, cr, i, cur, Id::new(("auto_pick", id))) {
                self.project.tracks[i].auto_shown = pick;
            }
            let plus = Rect::from_min_size(pos2(r.right() - 36.0, r.top() + 21.0), vec2(18.0, 16.0));
            if button(ui, plus, Id::new(("auto_plus", id)), "+", false, ACCENT).on_hover_text("Another automation lane under this track").clicked() {
                self.add_auto_lane(i);
            }
        } else {
            let kind_txt = if kind == TrackKind::Midi { "MIDI" } else { "Audio" };
            text(&p, pos2(r.left() + 12.0, r.top() + 28.0), Align2::LEFT_CENTER, kind_txt, small(), TEXT_DIM);
        }
        if resp.clicked() {
            let ctrl = ui.input(|k| k.modifiers.ctrl || k.modifiers.command);
            self.click_track(i, ctrl);
            self.show_devices();
        }
        if resp.double_clicked() {
            self.renaming = Some((RenameTarget::Track(i), name));
        }
        if let Some(item) = drop_payload::<BrowserItem>(&resp) {
            self.apply_item(&item, DropTarget::Track(i));
        }
        let by = r.bottom() - 22.0;
        let b1 = Rect::from_min_size(pos2(r.left() + 10.0, by), vec2(30.0, 17.0));
        if button(ui, b1, Id::new(("arr_act", id)), &format!("{}", i + 1), !mute, ACCENT).clicked() {
            self.project.tracks[i].mixer.mute = !mute;
        }
        let b2 = Rect::from_min_size(pos2(b1.right() + 4.0, by), vec2(22.0, 17.0));
        if button(ui, b2, Id::new(("arr_solo", id)), "S", solo, SOLO).clicked() {
            self.project.tracks[i].mixer.solo = !solo;
        }
        if kind == TrackKind::Midi {
            let b3 = Rect::from_min_size(pos2(b2.right() + 4.0, by), vec2(22.0, 17.0));
            let resp = button(ui, b3, Id::new(("arr_arm", id)), "", arm, REC).on_hover_text("Arm for recording (Ctrl+click for more)");
            rec_icon(ui.painter(), b3.center(), 7.0, if arm { TEXT_DARK } else { REC });
            if resp.clicked() {
                let multi = ui.input(|k| k.modifiers.ctrl || k.modifiers.command);
                self.click_arm(i, multi);
            }
        }
        let (ml, mr) = self.meters.get(i).copied().unwrap_or((0.0, 0.0));
        meter(&p, Rect::from_min_size(pos2(r.right() - 14.0, r.top() + 6.0), vec2(8.0, r.height() - 12.0)), ml, mr);
    }

    fn arr_lane(&mut self, ui: &mut Ui, i: usize, lane: Rect, bx: &dyn Fn(f64) -> f32, xb: &dyn Fn(f32) -> f64, free: bool) {
        if i >= self.project.tracks.len() {
            return;
        }
        let tid = self.project.tracks[i].id;
        let kind = self.project.tracks[i].kind;
        let color = rgb(self.project.tracks[i].color);
        let bpb = self.project.beats_per_bar_f();
        let p = ui.painter().clone();
        p.line_segment([pos2(lane.left(), lane.bottom() + 0.5), pos2(lane.right(), lane.bottom() + 0.5)], Stroke::new(1.0_f32, LINE_SOFT));

        let lresp = ui.interact(lane, Id::new(("lane", tid)), Sense::click());
        let grid = self.arr.grid;
        if lresp.clicked() {
            if let Some(pos) = lresp.interact_pointer_pos() {
                self.sel_track = TrackSel::Track(i);
                self.sel_clip = None;
                let beat = snap(xb(pos.x).max(0.0), grid.min(1.0).max(0.0), free);
                self.cmd(Command::SetPosition(beat));
            }
        }
        if lresp.double_clicked() && kind == TrackKind::Midi {
            if let Some(pos) = lresp.interact_pointer_pos() {
                let start = (xb(pos.x).max(0.0) / bpb).floor() * bpb;
                let id = self.project.alloc_id();
                let cid = self.project.alloc_id();
                let clip = Clip::new_midi(cid, "MIDI Clip", self.project.tracks[i].color, bpb);
                self.project.tracks[i].arrangement.push(ArrangementClip { id, start, length: bpb, offset: 0.0, clip });
                self.select_clip(ClipSel::Arr { track: i, id });
            }
        }
        // `hover_pos` is empty while something else is dragged; use the pointer.
        let hover_beat = if lresp.contains_pointer() { ui.input(|inp| inp.pointer.hover_pos()).map(|p| xb(p.x).max(0.0)) } else { None };
        if lresp.dnd_hover_payload::<BrowserItem>().is_some() || lresp.dnd_hover_payload::<SlotDrag>().is_some() {
            rect_line(&p, lane, 0.0, 1.5, ACCENT);
        }
        if let Some(item) = drop_payload::<BrowserItem>(&lresp) {
            let beat = snap(hover_beat.unwrap_or(0.0), if grid > 0.0 { grid } else { 1.0 }, free);
            let target = if matches!(&*item, BrowserItem::Sample(_)) { DropTarget::ArrLane { track: i, beat } } else { DropTarget::Track(i) };
            self.apply_item(&item, target);
        }
        if let Some(from) = drop_payload::<SlotDrag>(&lresp) {
            let beat = snap(hover_beat.unwrap_or(0.0), bpb, free);
            let src = self.project.tracks.get(from.track).and_then(|t| t.slots.get(from.scene)).and_then(|s| s.clone());
            if let Some(clip) = src {
                if clip.is_midi() == (kind == TrackKind::Midi) {
                    let id = self.project.alloc_id();
                    let clip = self.project.fresh_clip(&clip);
                    let length = clip.length;
                    self.project.tracks[i].arrangement.push(ArrangementClip { id, start: beat, length, offset: 0.0, clip });
                    self.select_clip(ClipSel::Arr { track: i, id });
                }
            }
        }

        let clips: Vec<(u64, f64, f64)> = self.project.tracks[i].arrangement.iter().map(|a| (a.id, a.start, a.end())).collect();
        for (cid, start, end) in clips {
            let x0 = bx(start);
            let x1 = bx(end);
            if x1 < lane.left() || x0 > lane.right() {
                continue;
            }
            let r = Rect::from_min_max(pos2(x0, lane.top() + 1.0), pos2(x1.max(x0 + 3.0), lane.bottom() - 1.0));
            self.arr_clip(ui, i, cid, r, lane, color, free);
        }
        // Automation over the clips.
        if self.arr.automation {
            if let Some(target) = self.project.tracks.get(i).and_then(|t| t.auto_shown) {
                self.auto_lane(ui, i, target, lane, bx, xb, free, true);
            }
        }
        // A recording in progress grows from where it started.
        let spb = self.engine.status.sample_rate.load() as f64 * 60.0 / self.project.bpm.max(1.0);
        if let Some(take) = self.take_on(tid, crate::recording::TakeTarget::Arrangement) {
            if let Some(start) = take.start {
                let (x0, x1) = (bx(start), bx(start + take.beats(spb)));
                let r = Rect::from_min_max(pos2(x0, lane.top() + 1.0), pos2(x1.max(x0 + 3.0), lane.bottom() - 1.0));
                let p = ui.painter();
                p.rect_filled(r, 2.0, shade(REC, 0.45));
                p.rect_stroke(r, 2.0, Stroke::new(1.0_f32, REC), egui::StrokeKind::Inside);
                p.text(r.left_top() + vec2(4.0, 3.0), Align2::LEFT_TOP, "Recording", small(), egui::Color32::WHITE);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn arr_clip(&mut self, ui: &mut Ui, i: usize, cid: u64, r: Rect, lane: Rect, track_color: egui::Color32, free: bool) {
        let sel = ClipSel::Arr { track: i, id: cid };
        let selected = self.sel_clip == Some(sel);
        let vis = r.intersect(lane);
        let resp = ui.interact(vis, Id::new(("arr_clip", cid)), Sense::click_and_drag());
        let ppb = self.arr.px_per_beat as f64;
        let grid = self.arr.grid;

        // Edge hover cursor.
        if let Some(h) = resp.hover_pos() {
            if (h.x - r.right()).abs() < EDGE || (h.x - r.left()).abs() < EDGE {
                ui.ctx().set_cursor_icon(CursorIcon::ResizeHorizontal);
            }
        }
        if resp.drag_started() {
            let px = ui.input(|i| i.pointer.press_origin()).map(|p| p.x).unwrap_or(r.center().x);
            let kind = if (px - r.right()).abs() < EDGE {
                DragKind::ResizeEnd
            } else if (px - r.left()).abs() < EDGE {
                DragKind::ResizeStart
            } else {
                DragKind::Move
            };
            if let Some(ac) = self.project.tracks[i].arrangement.iter().find(|a| a.id == cid).cloned() {
                self.arr.drag = Some(ArrDrag { track: i, orig_track: i, id: cid, kind, orig_start: ac.start, orig_len: ac.length, orig_offset: ac.offset, press_x: px });
                // Ctrl+drag leaves a copy behind.
                if matches!(kind, DragKind::Move) && ui.input(|i| i.modifiers.ctrl || i.modifiers.command) {
                    let id = self.project.alloc_id();
                    let clip = self.project.fresh_clip(&ac.clip);
                    self.project.tracks[i].arrangement.push(krabton_engine::ArrangementClip { id, clip, ..ac });
                }
            }
            self.select_clip(sel);
        }
        resp.dnd_set_drag_payload(ArrClipDrag { id: cid });
        if resp.dragged() {
            if let (Some(d), Some(pos)) = (self.arr.drag, resp.interact_pointer_pos()) {
                if d.id == cid && d.track == i {
                    let delta = (pos.x - d.press_x) as f64 / ppb;
                    let g = if grid > 0.0 { grid } else { 0.0 };
                    let clip_len = self.project.tracks[i].arrangement.iter().find(|a| a.id == cid).map(|a| a.clip.length).unwrap_or(1.0);
                    let min_len = if g > 0.0 { g.min(1.0) } else { 0.0625 };
                    // Moving up or down takes the clip to another track of the same kind.
                    let mut i = i;
                    if matches!(d.kind, DragKind::Move) {
                        // The track under the pointer (its own lane, not its automation lanes).
                        let rel = pos.y - self.arr.lane_top;
                        let nt = self.project.tracks.len();
                        let lane_h = self.lane_h();
                        let row = self.arr.tops.iter().take(nt).rposition(|&top| rel >= top).filter(|&k| rel < self.arr.tops[k] + lane_h);
                        if let Some(to) = row {
                            if to != i && self.project.tracks[to].kind == self.project.tracks[i].kind {
                                if let Some(at) = self.project.tracks[i].arrangement.iter().position(|a| a.id == cid) {
                                    let ac = self.project.tracks[i].arrangement.remove(at);
                                    self.project.tracks[to].arrangement.push(ac);
                                    if let Some(dd) = self.arr.drag.as_mut() {
                                        dd.track = to;
                                    }
                                    self.sel_clip = Some(ClipSel::Arr { track: to, id: cid });
                                    self.sel_track = TrackSel::Track(to);
                                    i = to;
                                }
                            }
                        }
                    }
                    if let Some(ac) = self.project.tracks[i].arrangement.iter_mut().find(|a| a.id == cid) {
                        match d.kind {
                            DragKind::Move => ac.start = snap(d.orig_start + delta, g, free).max(0.0),
                            DragKind::ResizeEnd => {
                                let end = snap(d.orig_start + d.orig_len + delta, g, free);
                                ac.length = (end - ac.start).max(min_len);
                            }
                            DragKind::ResizeStart => {
                                let end = d.orig_start + d.orig_len;
                                let ns = snap(d.orig_start + delta, g, free).clamp(0.0, end - min_len);
                                ac.start = ns;
                                ac.length = end - ns;
                                ac.offset = (d.orig_offset + (ns - d.orig_start)).rem_euclid(clip_len.max(1e-6));
                            }
                        }
                    }
                }
            }
        }
        if resp.drag_stopped() {
            self.arr.drag = None;
        }
        if resp.clicked() {
            self.select_clip(sel);
        }
        if resp.double_clicked() {
            self.detail = DetailTab::Clip;
        }

        let Some(ac) = self.project.tracks[i].arrangement.iter().find(|a| a.id == cid) else { return };
        let p = ui.painter().clone();
        let base = rgb(ac.clip.color);
        let _ = track_color;
        let playing_here = self.engine.status.tracks.get(i).map(|t| t.arr_clip.load(Ordering::Relaxed) == cid).unwrap_or(false);
        rect_fill(&p, r, 2.0, shade(base, 0.35));
        let head = Rect::from_min_size(r.min, vec2(r.width(), 13.0));
        rect_fill(&p, head, 2.0, if playing_here { base } else { shade(base, 0.85) });
        text_in(&p, head, &ac.clip.name, small(), text_on(base));
        let body = Rect::from_min_max(pos2(r.left(), head.bottom() + 1.0), pos2(r.right(), r.bottom() - 2.0));
        let pc = p.with_clip_rect(body.intersect(lane));
        let origin = ac.start - ac.offset;
        let len = ac.clip.length.max(1.0 / 64.0);
        let x_of = |beat: f64| r.left() + ((beat - ac.start) * ppb) as f32;
        match &ac.clip.content {
            ClipContent::Midi { notes } => {
                if !notes.is_empty() {
                    let lo = notes.iter().map(|n| n.pitch).min().unwrap_or(60) as f32;
                    let hi = notes.iter().map(|n| n.pitch).max().unwrap_or(60) as f32;
                    let span = (hi - lo).max(8.0);
                    let note_h = (body.height() / (span + 1.0)).clamp(1.5, 5.0);
                    let mut k = ((ac.start - origin) / len).floor();
                    let mut guard = 0;
                    while origin + k * len < ac.end() && guard < 256 {
                        for n in notes.iter().filter(|n| n.start < len) {
                            let t0 = origin + k * len + n.start;
                            let t1 = (t0 + n.length.min(len - n.start)).min(ac.end());
                            if t1 <= ac.start || t0 >= ac.end() {
                                continue;
                            }
                            let y = body.bottom() - ((n.pitch as f32 - lo) / span) * (body.height() - note_h) - note_h;
                            let nr = Rect::from_min_max(pos2(x_of(t0.max(ac.start)), y), pos2(x_of(t1).max(x_of(t0) + 1.0), y + note_h));
                            pc.rect_filled(nr, 0.0, base);
                        }
                        k += 1.0;
                        guard += 1;
                    }
                }
            }
            ClipContent::Audio(a) => {
                let s = &a.sample;
                let src_fpb = s.sample_rate as f64 * 60.0 / a.bpm.max(1.0);
                let mid = body.center().y;
                let half = body.height() * 0.5;
                let x_start = body.left().max(lane.left()) as i32;
                let x_end = body.right().min(lane.right()) as i32;
                for x in x_start..x_end {
                    let beat = ac.start + (x as f32 - r.left()) as f64 / ppb;
                    let local = (beat - origin).rem_euclid(len);
                    let frame = (local * src_fpb) as usize;
                    let idx = frame / krabton_engine::sample::PEAK_BLOCK;
                    let step = ((src_fpb / ppb) as usize / krabton_engine::sample::PEAK_BLOCK).max(1);
                    let mut mn = 0.0f32;
                    let mut mx = 0.0f32;
                    for j in idx..(idx + step).min(s.peaks.len()) {
                        mn = mn.min(s.peaks[j].0);
                        mx = mx.max(s.peaks[j].1);
                    }
                    pc.line_segment([pos2(x as f32, mid - mx * half), pos2(x as f32, mid - mn * half + 0.5)], Stroke::new(1.0_f32, base));
                }
            }
        }
        // Loop boundaries inside the clip.
        let mut k = ((ac.start - origin) / len).floor() + 1.0;
        while origin + k * len < ac.end() && k < 512.0 {
            let x = x_of(origin + k * len);
            pc.line_segment([pos2(x, body.top()), pos2(x, body.bottom())], Stroke::new(1.0_f32, shade(base, 0.6)));
            k += 1.0;
        }
        if selected {
            rect_line(&p, r, 2.0, 1.5, egui::Color32::WHITE);
        }

        let name = ac.clip.name.clone();
        let is_audio = matches!(ac.clip.content, ClipContent::Audio(_));
        resp.context_menu(|ui| {
            if ui.button("Rename").clicked() {
                self.renaming = Some((RenameTarget::Clip(sel), name.clone()));
                ui.close();
            }
            if ui.button("Duplicate      Ctrl+D").clicked() {
                self.duplicate_clip(sel);
                ui.close();
            }
            let scene = self.sel_scene;
            if ui.button(format!("Copy to Session (scene {})", scene + 1)).on_hover_text("Puts a copy in this track's slot of the selected scene").clicked() {
                self.arr_clip_to_slot((i, cid), (i, scene));
                ui.close();
            }
            if is_audio {
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
        });
    }
}

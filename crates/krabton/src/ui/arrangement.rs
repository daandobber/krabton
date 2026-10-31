//! Arrangement view: a timeline with one lane per track.

use crate::app::{App, ArrClipDrag, BrowserItem, ClipSel, DetailTab, DropTarget, RenameTarget, SlotDrag, TrackSel};
use crate::import::FileTarget;
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
/// The overview of the whole song, over the arranger track.
const OVERVIEW_H: f32 = 18.0;

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
    /// More clips picked along with `sel_clip` (Shift-click, or a box).
    pub(crate) picked: Vec<u64>,
    /// A box being drawn over empty lanes to pick clips: where it began (beat, y).
    band: Option<(f64, f32)>,
    /// The picked clips being dragged along: (id, where each started).
    group: Vec<(u64, f64)>,
}

impl ArrState {
    /// The grid edits snap to: the one chosen, or with Auto the finest whose
    /// lines lie at least 14 points apart at this zoom.
    pub fn grid_now(&self, bpb: f64) -> f64 {
        if self.grid >= 0.0 {
            return self.grid;
        }
        for g in [0.25, 0.5, 1.0, bpb, bpb * 4.0] {
            if g * self.px_per_beat as f64 >= 14.0 {
                return g;
            }
        }
        bpb * 16.0
    }
}

impl Default for ArrState {
    fn default() -> Self {
        Self {
            px_per_beat: 18.0,
            scroll_x: 0.0,
            scroll_y: 0.0,
            grid: -1.0,
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
            picked: Vec::new(),
            band: None,
            group: Vec::new(),
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
        // The whole song in a strip, then the arranger track (song sections) and the ruler.
        let overview = Rect::from_min_max(pos2(full.left() + head_w, toolbar.bottom()), pos2(full.right(), toolbar.bottom() + OVERVIEW_H));
        let arranger = Rect::from_min_max(pos2(full.left() + head_w, overview.bottom()), pos2(full.right(), overview.bottom() + SECTION_H));
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
            let grids = [(-1.0, "Auto"), (bpb, "1 Bar"), (2.0, "1/2"), (1.0, "1/4"), (0.5, "1/8"), (0.25, "1/16"), (2.0 / 3.0, "1/4 T"), (1.0 / 3.0, "1/8 T"), (1.0 / 6.0, "1/16 T"), (0.0, "Off")];
            let cur = grids.iter().find(|g| (g.0 - self.arr.grid).abs() < 1e-9).map(|g| g.1).unwrap_or("Auto");
            egui::ComboBox::from_id_salt("arr_grid").width(60.0).selected_text(cur).show_ui(&mut tui, |ui| {
                for (g, name) in grids {
                    ui.selectable_value(&mut self.arr.grid, g, name).on_hover_text(if g < 0.0 { "The grid follows the zoom: finer as you zoom in" } else { "" });
                }
            });
            if tui.button("-").on_hover_text("Zoom out (Ctrl+wheel)").clicked() {
                self.arr.px_per_beat = (self.arr.px_per_beat / 1.4).max(2.0);
            }
            if tui.button("+").on_hover_text("Zoom in").clicked() {
                self.arr.px_per_beat = (self.arr.px_per_beat * 1.4).min(400.0);
            }
            if tui.button("Fit").on_hover_text("The whole song in view").clicked() {
                let end = self.song_end().max(4.0 * bpb);
                self.arr.px_per_beat = (((full.width() - head_w - 20.0) / end as f32).clamp(2.0, 400.0)) as f32;
                self.arr.scroll_x = 0.0;
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
            tui.label(egui::RichText::new("Double-click a MIDI lane for a clip · drag on empty lanes to pick clips (Shift: more) · Ctrl+E split · 0 on/off · Alt: no snap").small().color(TEXT_DIM));
        }

        // Zoom and scroll.
        if ui.rect_contains_pointer(lanes.union(ruler).union(arranger).union(overview)) {
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
        for (k, tr) in self.project.tracks.iter().enumerate() {
            tops.push(acc);
            acc += self.track_h(k) + if self.arr.automation { tr.auto_lanes.len() as f32 * auto_h } else { 0.0 };
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

        // Grid lines (behind lanes): bars, beats and sixteenths as the zoom allows.
        let first_beat = xb(lanes.left()).floor().max(0.0);
        let last_beat = xb(lanes.right()).ceil();
        let lp = p.with_clip_rect(lanes.union(ruler));
        let bar_px = ppb * bpb as f32;
        let step = if ppb >= 48.0 {
            0.25
        } else if ppb >= 10.0 {
            1.0
        } else if bar_px >= 8.0 {
            bpb
        } else {
            bpb * 4.0
        };
        let label_every = if bar_px > 40.0 { 1 } else if bar_px > 12.0 { 4 } else { 16 };
        let bpm = self.project.bpm.max(1.0);
        let mut b = (first_beat / step).floor() * step;
        while b <= last_beat {
            let x = bx(b);
            let is_bar = (b / bpb).fract().abs() < 1e-9 || (b / bpb).fract() > 1.0 - 1e-9;
            let is_beat = b.fract().abs() < 1e-9;
            let line = if is_bar { LINE_SOFT } else if is_beat { shade(LINE_SOFT, 0.75) } else { shade(LINE_SOFT, 0.55) };
            lp.line_segment([pos2(x, lanes.top()), pos2(x, lanes.bottom())], Stroke::new(1.0_f32, line));
            if is_bar {
                let bar = (b / bpb).round() as i64 + 1;
                if (bar - 1) % label_every == 0 {
                    lp.line_segment([pos2(x, ruler.bottom() - 8.0), pos2(x, ruler.bottom())], Stroke::new(1.0_f32, TEXT_DIM));
                    let r = tabular_text(&lp, pos2(x + 3.0, ruler.bottom() - 4.0), Align2::LEFT_BOTTOM, &bar.to_string(), small(), TEXT_DIM);
                    // And the time there, where it fits.
                    if bar_px * label_every as f32 > 84.0 {
                        let secs = b * 60.0 / bpm;
                        let t = format!("{}:{:02}", (secs / 60.0) as i64, (secs % 60.0) as i64);
                        tabular_text(&lp, pos2(r.right() + 6.0, ruler.bottom() - 4.0), Align2::LEFT_BOTTOM, &t, egui::FontId::proportional(9.0), shade(TEXT_DIM, 0.75));
                    }
                }
            } else if is_beat && ppb >= 34.0 {
                let bar = (b / bpb).floor() as i64 + 1;
                let beat = (b - (bar - 1) as f64 * bpb).round() as i64 + 1;
                lp.line_segment([pos2(x, ruler.bottom() - 4.0), pos2(x, ruler.bottom())], Stroke::new(1.0_f32, shade(TEXT_DIM, 0.7)));
                tabular_text(&lp, pos2(x + 2.0, ruler.bottom() - 4.0), Align2::LEFT_BOTTOM, &format!("{bar}.{beat}"), egui::FontId::proportional(9.0), shade(TEXT_DIM, 0.7));
            }
            b += step;
        }

        // The whole song: every clip, the view's window over it; click or drag to go there.
        self.overview(ui, overview, lanes);

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
            let th = self.track_h(i);
            let lane = Rect::from_min_size(pos2(lanes.left(), y), vec2(lanes.width(), th - 1.0));
            if !lane.intersects(lanes) && !(self.arr.automation && lane_y(i + 1) > lanes.top()) {
                continue;
            }
            let head = Rect::from_min_size(pos2(heads.left(), y), vec2(head_w - 1.0, th - 1.0));
            self.arr_head(&mut hui, i, head);
            self.arr_lane(&mut lui, i, lane, &bx, &xb, free);
            // The automation lanes under it.
            if self.arr.automation {
                let n = self.project.tracks.get(i).map(|t| t.auto_lanes.len()).unwrap_or(0);
                for k in 0..n {
                    let ky = y + th + k as f32 * auto_h;
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
                match &*item {
                    // A MIDI file's tracks start on the bar it was dropped at.
                    BrowserItem::MidiFile(path) => {
                        let bpb = self.project.beats_per_bar_f();
                        let at = ui.input(|i| i.pointer.hover_pos()).map(|p| (xb(p.x).max(0.0) / bpb).round() * bpb).unwrap_or(0.0);
                        if let Some(song) = self.read_midi_file(path) {
                            self.midi_file_tracks(&item.label(), &song, Some(at));
                        }
                    }
                    _ => self.apply_item(&item, DropTarget::NewTrack),
                }
            }
        }
        // Files from the desktop below the tracks: a new audio track, from the bar they are dropped at.
        let empty = Rect::from_min_max(pos2(lanes.left(), lane_y(nt).max(lanes.top())), lanes.max);
        if let Some(fp) = self.imports.drag.pos.filter(|fp| empty.height() > 4.0 && empty.contains(*fp)) {
            let bpb = self.project.beats_per_bar_f();
            let at = (xb(fp.x).max(0.0) / bpb).floor() * bpb;
            self.file_over(&p, empty, FileTarget::NewTrack { beat: Some(at) });
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
            let th = self.track_h(i);
            let y = lane_y(i) + th - 1.0;
            if y < heads.top() || y > heads.bottom() {
                continue;
            }
            let r = Rect::from_min_max(pos2(heads.left(), y - 3.0), pos2(heads.right() - 4.0, y + 3.0));
            let resp = splitter(ui, Id::new(("arr_lane_split", i)), r, false).on_hover_text("Drag: this track's height (Shift: every track's); double-click: back to the usual");
            if resp.dragged() {
                if ui.input(|k| k.modifiers.shift) {
                    // Every lane: the usual height, and no track keeps its own.
                    self.prefs.layout.lane_h = (lane_h + resp.drag_delta().y / (i + 1) as f32).clamp(28.0, 240.0);
                    self.layout_dirty = true;
                    for t in &mut self.project.tracks {
                        t.lane_h = 0.0;
                    }
                } else {
                    self.project.tracks[i].lane_h = (th + resp.drag_delta().y).clamp(28.0, 240.0);
                }
            }
            if resp.double_clicked() {
                self.project.tracks[i].lane_h = 0.0;
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
            let edge_l = ui.interact(Rect::from_min_max(sr.min, pos2(sr.left() + 5.0, sr.bottom())), Id::new(("section_edge_l", s.id)), Sense::drag());
            if edge_l.hovered() || edge_l.dragged() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
            }
            if edge_l.dragged() {
                if let Some(pos) = edge_l.interact_pointer_pos() {
                    let prev = sections.iter().map(|x| x.end()).filter(|&x| x <= s.start + 1e-9).fold(0.0, f64::max);
                    let start = ((xb(pos.x) / bpb).round() * bpb).clamp(prev, s.end() - bpb);
                    if let Some(x) = self.project.sections.iter_mut().find(|x| x.id == s.id) {
                        x.length = x.end() - start;
                        x.start = start;
                    }
                }
            }
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
            if edge_l.dragged() {
            } else if edge.dragged() {
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

    /// Track `i`'s lane height: its own, or the usual.
    pub(crate) fn track_h(&self, i: usize) -> f32 {
        match self.project.tracks.get(i).map(|t| t.lane_h) {
            Some(h) if h > 0.0 => h.clamp(28.0, 240.0),
            _ => self.lane_h(),
        }
    }

    /// Where the song ends: its last clip, section or the loop.
    pub(crate) fn song_end(&self) -> f64 {
        let clips = self.project.tracks.iter().flat_map(|t| t.arrangement.iter().map(|a| a.end())).fold(0.0f64, f64::max);
        let sections = self.project.sections.iter().map(|s| s.end()).fold(0.0f64, f64::max);
        let lp = if self.project.loop_enabled { self.project.loop_end } else { 0.0 };
        clips.max(sections).max(lp)
    }

    /// The whole song in a strip: a line of clips per track, the part in
    /// view framed. Click or drag to look there.
    fn overview(&mut self, ui: &mut Ui, r: Rect, lanes: Rect) {
        let bpb = self.project.beats_per_bar_f();
        let total = (self.song_end() + 8.0 * bpb).max(32.0 * bpb);
        let p = ui.painter_at(r);
        rect_fill(&p, r, 0.0, BG);
        let x_of = |b: f64| r.left() + (b / total) as f32 * r.width();
        let nt = self.project.tracks.len().max(1);
        let row = ((r.height() - 4.0) / nt as f32).clamp(1.0, 4.0);
        for (k, t) in self.project.tracks.iter().enumerate() {
            let y = r.top() + 2.0 + k as f32 * row;
            if y + row > r.bottom() {
                break;
            }
            for a in &t.arrangement {
                let c = rgb(a.clip.color);
                p.rect_filled(Rect::from_min_max(pos2(x_of(a.start), y), pos2(x_of(a.end()).max(x_of(a.start) + 1.0), y + row - 0.5)), 0.0, if a.muted { shade(c, 0.35) } else { c });
            }
        }
        for s in &self.project.sections {
            p.line_segment([pos2(x_of(s.start), r.top()), pos2(x_of(s.start), r.bottom())], Stroke::new(1.0_f32, shade(rgb(s.color), 0.8)));
        }
        let ppb = self.arr.px_per_beat as f64;
        let v0 = self.arr.scroll_x as f64 / ppb;
        let v1 = v0 + lanes.width() as f64 / ppb;
        let view = Rect::from_min_max(pos2(x_of(v0), r.top() + 0.5), pos2(x_of(v1).min(r.right()), r.bottom() - 0.5));
        p.rect_filled(view, 2.0, egui::Color32::from_white_alpha(18));
        p.rect_stroke(view, 2.0, Stroke::new(1.0_f32, egui::Color32::from_white_alpha(90)), egui::StrokeKind::Inside);
        let song_pos = self.engine.status.song_pos.load();
        p.line_segment([pos2(x_of(song_pos), r.top()), pos2(x_of(song_pos), r.bottom())], Stroke::new(1.0_f32, egui::Color32::WHITE));
        p.line_segment([r.left_bottom(), r.right_bottom()], Stroke::new(1.0_f32, LINE));
        let resp = ui.interact(r, Id::new("arr_overview"), Sense::click_and_drag());
        if resp.clicked() || resp.dragged() {
            if let Some(pos) = resp.interact_pointer_pos() {
                let at = ((pos.x - r.left()) / r.width()) as f64 * total;
                self.arr.scroll_x = ((at - (v1 - v0) * 0.5) * ppb).max(0.0) as f32;
                self.arr.follow = false;
            }
        }
        resp.on_hover_text("The whole song: click or drag to look there (Follow goes off)");
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
                let g = self.arr.grid_now(bpb);
                let grid = if g > 0.0 { g } else { 1.0 };
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
                let g = self.arr.grid_now(bpb);
                let grid = if g > 0.0 { g.min(bpb) } else { 0.0 };
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
        let selected = self.sel_track == TrackSel::Track(i) || self.sel_more.contains(&id);
        let color = rgb(t.color);
        let name = t.name.clone();
        let (mute, solo, arm, kind) = (t.mixer.mute, t.mixer.solo, t.arm, t.kind);
        let (vol, pan) = (t.mixer.volume_db, t.mixer.pan);
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
        self.file_over(&p, r, FileTarget::Track(i));
        let by = r.bottom() - 22.0;
        let b1 = Rect::from_min_size(pos2(r.left() + 10.0, by), vec2(30.0, 17.0));
        if button(ui, b1, Id::new(("arr_act", id)), &format!("{}", i + 1), !mute, ACCENT).clicked() {
            self.project.tracks[i].mixer.mute = !mute;
        }
        let b2 = Rect::from_min_size(pos2(b1.right() + 4.0, by), vec2(22.0, 17.0));
        if button(ui, b2, Id::new(("arr_solo", id)), "S", solo, SOLO).clicked() {
            self.project.tracks[i].mixer.solo = !solo;
        }
        let b3 = Rect::from_min_size(pos2(b2.right() + 4.0, by), vec2(22.0, 17.0));
        if kind == TrackKind::Midi || kind == TrackKind::Audio {
            let resp = button(ui, b3, Id::new(("arr_arm", id)), "", arm, REC).on_hover_text("Arm for recording (Ctrl+click for more)");
            rec_icon(ui.painter(), b3.center(), 7.0, if arm { TEXT_DARK } else { REC });
            if resp.clicked() {
                let multi = ui.input(|k| k.modifiers.ctrl || k.modifiers.command);
                self.click_arm(i, multi);
            }
        }
        // Volume and pan, where there is room.
        let knob_s = (r.height() - 30.0).clamp(0.0, 22.0);
        let room = r.right() - 18.0 - b3.right();
        if knob_s >= 16.0 && room >= knob_s * 2.0 + 10.0 {
            let ky = r.bottom() - 4.0 - knob_s;
            let vr = Rect::from_min_size(pos2(r.right() - 18.0 - knob_s * 2.0 - 4.0, ky), vec2(knob_s, knob_s));
            let (resp, new) = knob(ui, vr, Id::new(("arr_vol", id)), db_to_fader(vol), db_to_fader(0.0), false, color);
            if let Some(n) = new {
                self.project.tracks[i].mixer.volume_db = fader_to_db(n);
            }
            resp.on_hover_text(format!("Volume: {}", if krabton_engine::fader_gain(vol) == 0.0 { "-inf".into() } else { format!("{vol:.1} dB") }));
            let pr = Rect::from_min_size(pos2(r.right() - 18.0 - knob_s, ky), vec2(knob_s, knob_s));
            let (resp, new) = knob(ui, pr, Id::new(("arr_pan", id)), (pan + 1.0) * 0.5, 0.5, true, color);
            if let Some(n) = new {
                self.project.tracks[i].mixer.pan = (n * 2.0 - 1.0).clamp(-1.0, 1.0);
            }
            resp.on_hover_text(format!("Pan: {}", crate::ui::session::pan_text(pan)));
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

        let lresp = ui.interact(lane, Id::new(("lane", tid)), Sense::click_and_drag());
        let grid = self.arr.grid_now(bpb);
        if lresp.clicked() {
            if let Some(pos) = lresp.interact_pointer_pos() {
                self.sel_track = TrackSel::Track(i);
                self.sel_clip = None;
                let beat = snap(xb(pos.x).max(0.0), grid.min(1.0).max(0.0), free);
                self.cmd(Command::SetPosition(beat));
            }
        }
        // A box drawn over empty lanes picks the clips it touches (Shift: more).
        if lresp.drag_started() {
            if let Some(o) = ui.input(|i| i.pointer.press_origin()) {
                self.arr.band = Some((xb(o.x), o.y));
                if !ui.input(|i| i.modifiers.shift) {
                    self.arr.picked.clear();
                    self.sel_clip = None;
                }
            }
        }
        if let (true, Some((b0, y0)), Some(pos)) = (lresp.dragged(), self.arr.band, lresp.interact_pointer_pos()) {
            let b1 = xb(pos.x);
            let (lo, hi) = (b0.min(b1), b0.max(b1));
            let (ya, yb) = (y0.min(pos.y), y0.max(pos.y));
            let band = Rect::from_min_max(pos2(bx(lo), ya), pos2(bx(hi), yb));
            let fg = ui.ctx().layer_painter(egui::LayerId::new(egui::Order::Foreground, Id::new("arr_band")));
            fg.rect_filled(band.intersect(ui.clip_rect()), 0.0, egui::Color32::from_rgba_unmultiplied(0x9f, 0xd8, 0xff, 24));
            fg.rect_stroke(band.intersect(ui.clip_rect()), 0.0, Stroke::new(1.0_f32, SELECT), egui::StrokeKind::Inside);
            let mut hits = Vec::new();
            for k in 0..self.project.tracks.len() {
                let top = self.arr.lane_top + self.arr.tops.get(k).copied().unwrap_or(0.0);
                if top > yb || top + self.track_h(k) < ya {
                    continue;
                }
                hits.extend(self.project.tracks[k].arrangement.iter().filter(|a| a.start < hi && a.end() > lo).map(|a| (k, a.id)));
            }
            let keep = ui.input(|i| i.modifiers.shift);
            if !keep {
                self.arr.picked.clear();
            }
            for (_, id) in &hits {
                if !self.arr.picked.contains(id) {
                    self.arr.picked.push(*id);
                }
            }
            if let Some(&(k, id)) = hits.first() {
                self.sel_clip = Some(ClipSel::Arr { track: k, id });
            }
        }
        if lresp.drag_stopped() {
            self.arr.band = None;
        }
        if lresp.double_clicked() && kind == TrackKind::Midi {
            if let Some(pos) = lresp.interact_pointer_pos() {
                let start = (xb(pos.x).max(0.0) / bpb).floor() * bpb;
                let id = self.project.alloc_id();
                let cid = self.project.alloc_id();
                let clip = Clip::new_midi(cid, "MIDI Clip", self.project.tracks[i].color, bpb);
                self.project.tracks[i].arrangement.push(ArrangementClip::new(id, start, bpb, clip));
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
            let target = if matches!(&*item, BrowserItem::Sample(_) | BrowserItem::Clip { .. } | BrowserItem::MidiFile(_)) { DropTarget::ArrLane { track: i, beat } } else { DropTarget::Track(i) };
            self.apply_item(&item, target);
        }
        // Files from the desktop: one after another from the beat they are dropped at.
        if let Some(fp) = self.imports.drag.pos.filter(|fp| lane.contains(*fp)) {
            let beat = snap(xb(fp.x).max(0.0), if grid > 0.0 { grid } else { 1.0 }, free);
            self.file_over(&p, lane, FileTarget::Lane { track: i, beat });
        }
        // An audio lane asks for a file, to put at the bar double-clicked.
        if lresp.double_clicked() && kind == TrackKind::Audio {
            if let Some(pos) = lresp.interact_pointer_pos() {
                let start = (xb(pos.x).max(0.0) / bpb).floor() * bpb;
                self.load_audio_dialog(FileTarget::Lane { track: i, beat: start });
            }
        }
        if let Some(from) = drop_payload::<SlotDrag>(&lresp) {
            let beat = snap(hover_beat.unwrap_or(0.0), bpb, free);
            let src = self.project.tracks.get(from.track).and_then(|t| t.slots.get(from.scene)).and_then(|s| s.clone());
            if let Some(clip) = src {
                if clip.is_midi() == (kind == TrackKind::Midi) {
                    let id = self.project.alloc_id();
                    let clip = self.project.fresh_clip(&clip);
                    let length = clip.length;
                    self.project.tracks[i].arrangement.push(ArrangementClip::new(id, beat, length, clip));
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
        let selected = self.sel_clip == Some(sel) || self.arr.picked.contains(&cid);
        let vis = r.intersect(lane);
        let resp = ui.interact(vis, Id::new(("arr_clip", cid)), Sense::click_and_drag());
        let ppb = self.arr.px_per_beat as f64;
        let grid = self.arr.grid_now(self.project.beats_per_bar_f());

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
            // Dragging one of the picked clips takes all of them along.
            let together = matches!(kind, DragKind::Move) && selected && !self.arr.picked.is_empty();
            self.arr.group.clear();
            if together {
                let picked: Vec<(usize, u64)> = self.arr_selection();
                for (t, id) in picked {
                    if id != cid {
                        if let Some(a) = self.project.tracks[t].arrangement.iter().find(|a| a.id == id) {
                            self.arr.group.push((id, a.start));
                        }
                    }
                }
            } else {
                self.arr.picked.clear();
            }
            if let Some(ac) = self.project.tracks[i].arrangement.iter().find(|a| a.id == cid).cloned() {
                self.arr.drag = Some(ArrDrag { track: i, orig_track: i, id: cid, kind, orig_start: ac.start, orig_len: ac.length, orig_offset: ac.offset, press_x: px });
                // Ctrl+drag leaves a copy behind (of each picked one).
                if matches!(kind, DragKind::Move) && ui.input(|i| i.modifiers.ctrl || i.modifiers.command) {
                    let mut copies: Vec<(usize, krabton_engine::ArrangementClip)> = vec![(i, ac.clone())];
                    for (id, _) in &self.arr.group {
                        if let Some((t, a)) = self.project.tracks.iter().enumerate().find_map(|(t, tr)| tr.arrangement.iter().find(|a| a.id == *id).map(|a| (t, a.clone()))) {
                            copies.push((t, a));
                        }
                    }
                    for (t, a) in copies {
                        let id = self.project.alloc_id();
                        let clip = self.project.fresh_clip(&a.clip);
                        self.project.tracks[t].arrangement.push(krabton_engine::ArrangementClip { id, clip, ..a });
                    }
                }
            }
            let keep = std::mem::take(&mut self.arr.picked);
            self.select_clip(sel);
            self.arr.picked = keep;
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
                    if matches!(d.kind, DragKind::Move) && self.arr.group.is_empty() {
                        // The track under the pointer (its own lane, not its automation lanes).
                        let rel = pos.y - self.arr.lane_top;
                        let nt = self.project.tracks.len();
                        let row = self.arr.tops.iter().take(nt).rposition(|&top| rel >= top).filter(|&k| rel < self.arr.tops[k] + self.track_h(k));
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
                    // The others picked move by as much (none before the start).
                    if matches!(d.kind, DragKind::Move) && !self.arr.group.is_empty() {
                        let earliest = self.arr.group.iter().map(|g| g.1).fold(d.orig_start, f64::min);
                        let moved = (snap(d.orig_start + delta, g, free) - d.orig_start).max(-earliest);
                        let group = self.arr.group.clone();
                        for t in self.project.tracks.iter_mut() {
                            for a in t.arrangement.iter_mut() {
                                if let Some((_, from)) = group.iter().find(|x| x.0 == a.id) {
                                    a.start = from + moved;
                                }
                            }
                        }
                        if let Some(ac) = self.project.tracks[i].arrangement.iter_mut().find(|a| a.id == cid) {
                            ac.start = d.orig_start + moved;
                        }
                    } else if let Some(ac) = self.project.tracks[i].arrangement.iter_mut().find(|a| a.id == cid) {
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
            self.arr.group.clear();
        }
        if resp.clicked() {
            // Shift or Ctrl: pick it too (or let it go); a plain click picks it alone.
            if ui.input(|k| k.modifiers.shift || k.modifiers.ctrl || k.modifiers.command) {
                if let Some(ClipSel::Arr { id, .. }) = self.sel_clip {
                    if !self.arr.picked.contains(&id) {
                        self.arr.picked.push(id);
                    }
                }
                if let Some(k) = self.arr.picked.iter().position(|&x| x == cid) {
                    self.arr.picked.remove(k);
                    if self.sel_clip == Some(sel) {
                        self.sel_clip = self.arr_selection().first().map(|&(t, id)| ClipSel::Arr { track: t, id });
                    }
                } else {
                    self.arr.picked.push(cid);
                    let keep = std::mem::take(&mut self.arr.picked);
                    self.select_clip(sel);
                    self.arr.picked = keep;
                }
            } else {
                self.arr.picked.clear();
                self.select_clip(sel);
            }
        }
        if resp.double_clicked() {
            self.detail = DetailTab::Clip;
        }

        // An audio clip's fades: a handle at each top corner, drag it in.
        let audio_clip = self.project.tracks[i].arrangement.iter().find(|a| a.id == cid).is_some_and(|a| matches!(a.clip.content, ClipContent::Audio(_)));
        if audio_clip && r.width() > 28.0 && r.height() > 26.0 {
            let (fi, fo, len) = self.project.tracks[i].arrangement.iter().find(|a| a.id == cid).map(|a| (a.fade_in, a.fade_out, a.length)).unwrap_or((0.0, 0.0, 1.0));
            let y = r.top() + 18.0;
            for (k, x) in [(0, r.left() + (fi * ppb) as f32), (1, r.right() - (fo * ppb) as f32)] {
                let x = x.clamp(r.left() + 4.0, r.right() - 4.0);
                let hr = Rect::from_center_size(pos2(x, y), vec2(10.0, 10.0));
                let h = ui.interact(hr.intersect(lane), Id::new(("arr_fade", cid, k)), Sense::drag());
                let hot = h.hovered() || h.dragged();
                if hot {
                    ui.ctx().set_cursor_icon(CursorIcon::ResizeHorizontal);
                }
                if let Some(pos) = h.interact_pointer_pos().filter(|_| h.dragged()) {
                    if let Some(ac) = self.project.tracks[i].arrangement.iter_mut().find(|a| a.id == cid) {
                        if k == 0 {
                            ac.fade_in = (((pos.x - r.left()) as f64) / ppb).clamp(0.0, (len - ac.fade_out).max(0.0));
                        } else {
                            ac.fade_out = (((r.right() - pos.x) as f64) / ppb).clamp(0.0, (len - ac.fade_in).max(0.0));
                        }
                    }
                }
                if h.double_clicked() {
                    if let Some(ac) = self.project.tracks[i].arrangement.iter_mut().find(|a| a.id == cid) {
                        if k == 0 {
                            ac.fade_in = 0.0;
                        } else {
                            ac.fade_out = 0.0;
                        }
                    }
                }
                let painter = ui.painter();
                painter.rect_filled(hr.shrink(1.5), 1.5, if hot { egui::Color32::WHITE } else { egui::Color32::from_white_alpha(170) });
                painter.rect_stroke(hr.shrink(1.5), 1.5, Stroke::new(1.0_f32, egui::Color32::from_black_alpha(160)), egui::StrokeKind::Outside);
                h.on_hover_text(if k == 0 { "Fade in: drag; double-click for none" } else { "Fade out: drag; double-click for none" });
            }
        }

        let Some(ac) = self.project.tracks[i].arrangement.iter().find(|a| a.id == cid) else { return };
        let p = ui.painter().clone();
        let base = if ac.muted { egui::Color32::from_gray(0x70) } else { rgb(ac.clip.color) };
        let _ = track_color;
        let playing_here = self.engine.status.tracks.get(i).map(|t| t.arr_clip.load(Ordering::Relaxed) == cid).unwrap_or(false);
        rect_fill(&p, r, 2.0, shade(base, 0.35));
        let head = Rect::from_min_size(r.min, vec2(r.width(), 13.0));
        rect_fill(&p, head, 2.0, if playing_here { base } else { shade(base, 0.85) });
        let mut name_box = head;
        if ac.clip.sound.is_some() && head.width() > 40.0 {
            name_box.max.x -= 16.0;
            crate::ui::session::sound_mark(&p, pos2(head.right() - 9.0, head.center().y + 1.0), text_on(base));
        }
        text_in(&p, name_box, &ac.clip.name, small(), text_on(base));
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
        // The fades: the curve, and the faded part darker.
        for (k, f) in [(0, ac.fade_in), (1, ac.fade_out)] {
            if f <= 1e-6 {
                continue;
            }
            let w = (f * ppb) as f32;
            let (x0, x1) = if k == 0 { (r.left(), r.left() + w) } else { (r.right() - w, r.right()) };
            let corner = if k == 0 { pos2(r.left(), body.top()) } else { pos2(r.right(), body.top()) };
            let curve: Vec<egui::Pos2> = (0..=16)
                .map(|j| {
                    let t = j as f32 / 16.0;
                    let x = x0 + (x1 - x0) * t;
                    let g = if k == 0 { (t * std::f32::consts::FRAC_PI_2).sin() } else { ((1.0 - t) * std::f32::consts::FRAC_PI_2).sin() };
                    pos2(x, body.bottom() - g * body.height())
                })
                .collect();
            let mut m = egui::epaint::Mesh::default();
            m.colored_vertex(corner, egui::Color32::from_black_alpha(110));
            for q in &curve {
                m.colored_vertex(*q, egui::Color32::from_black_alpha(110));
            }
            for j in 0..curve.len() as u32 - 1 {
                m.add_triangle(0, 1 + j, 2 + j);
            }
            pc.add(egui::Shape::mesh(m));
            pc.add(egui::Shape::line(curve, Stroke::new(1.5_f32, egui::Color32::from_white_alpha(200))));
        }
        if ac.muted {
            p.text(pos2(r.right() - 4.0, r.bottom() - 3.0), Align2::RIGHT_BOTTOM, "off", small(), TEXT_DIM);
        }
        if selected {
            rect_line(&p, r, 2.0, 1.5, egui::Color32::WHITE);
        }

        let name = ac.clip.name.clone();
        let is_audio = matches!(ac.clip.content, ClipContent::Audio(_));
        let keeps = ac.clip.sound.is_some();
        let muted = ac.muted;
        let bpb = self.project.beats_per_bar_f();
        let left = r.left();
        let start = ac.start;
        resp.context_menu(|ui| {
            // Where it was right-clicked, on the grid (Alt: anywhere).
            let at = ui.ctx().input(|i| i.pointer.press_origin()).map(|p| start + ((p.x - left) as f64) / ppb);
            if let Some(at) = at {
                let at = snap(at, if grid > 0.0 { grid.min(bpb) } else { 0.0 }, ui.input(|k| k.modifiers.alt));
                if ui.button("Split Here          Ctrl+E").on_hover_text("Cut the clip in two here (Ctrl+E cuts at the playhead)").clicked() {
                    self.project.split_clip(i, cid, at);
                    ui.close();
                }
            }
            if ui.button(if muted { "Switch On                0" } else { "Switch Off              0" }).on_hover_text("A clip switched off stays where it is and does not play").clicked() {
                if !self.arr.picked.contains(&cid) {
                    self.arr.picked.clear();
                    self.select_clip(sel);
                }
                self.arr_toggle_off();
                ui.close();
            }
            if ui.button("Rename").clicked() {
                self.renaming = Some((RenameTarget::Clip(sel), name.clone()));
                ui.close();
            }
            crate::ui::session::sound_menu(self, ui, sel, keeps);
            if ui.button("Duplicate      Ctrl+D").clicked() {
                self.duplicate_clip(sel);
                ui.close();
            }
            let scene = self.sel_scene;
            if ui.button(format!("Copy to Session (scene {})", scene + 1)).on_hover_text("Puts a copy in this track's slot of the selected scene").clicked() {
                self.arr_clip_to_slot((i, cid), (i, scene));
                ui.close();
            }
            if !is_audio && ui.button("Export MIDI File...").clicked() {
                self.export_midi_clip(sel);
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
                if ui.button("Slice to New MIDI Track...").on_hover_text("Cut the clip at its hits or on the beat: a Drum Rack of the slices and a clip that plays them, on a new track").clicked() {
                    self.open_slice_dialog(sel);
                    ui.close();
                }
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

    /// The arrangement clips picked now: (track, id), the selected one first.
    pub(crate) fn arr_selection(&self) -> Vec<(usize, u64)> {
        let mut out = Vec::new();
        if let Some(ClipSel::Arr { track, id }) = self.sel_clip {
            if self.project.tracks.get(track).is_some_and(|t| t.arrangement.iter().any(|a| a.id == id)) {
                out.push((track, id));
            }
        }
        for &id in &self.arr.picked {
            if out.iter().any(|x| x.1 == id) {
                continue;
            }
            if let Some(t) = self.project.tracks.iter().position(|t| t.arrangement.iter().any(|a| a.id == id)) {
                out.push((t, id));
            }
        }
        out
    }

    /// Deletes the picked clips.
    pub(crate) fn arr_delete_selection(&mut self) {
        let sel = self.arr_selection();
        for t in self.project.tracks.iter_mut() {
            t.arrangement.retain(|a| !sel.iter().any(|x| x.1 == a.id));
        }
        self.arr.picked.clear();
        self.sel_clip = None;
    }

    /// Copies the picked clips right after them, as a block, and picks the copies.
    pub(crate) fn arr_duplicate_selection(&mut self) {
        let sel = self.arr_selection();
        let clips: Vec<(usize, krabton_engine::ArrangementClip)> = sel.iter().filter_map(|&(t, id)| self.project.tracks[t].arrangement.iter().find(|a| a.id == id).map(|a| (t, a.clone()))).collect();
        if clips.is_empty() {
            return;
        }
        let from = clips.iter().map(|c| c.1.start).fold(f64::MAX, f64::min);
        let to = clips.iter().map(|c| c.1.end()).fold(0.0, f64::max);
        let mut picked = Vec::new();
        for (t, a) in clips {
            let id = self.project.alloc_id();
            let clip = self.project.fresh_clip(&a.clip);
            self.project.tracks[t].arrangement.push(krabton_engine::ArrangementClip { id, clip, start: a.start + (to - from), ..a });
            picked.push((t, id));
        }
        self.arr.picked = picked.iter().map(|p| p.1).collect();
        self.sel_clip = picked.first().map(|&(t, id)| ClipSel::Arr { track: t, id });
    }

    /// Cuts at the playhead: the picked clips under it, else every clip of
    /// the selected track under it.
    pub(crate) fn arr_split_at_playhead(&mut self) {
        let at = self.engine.status.song_pos.load();
        let mut targets: Vec<(usize, u64)> = self.arr_selection().into_iter().filter(|&(t, id)| self.project.tracks[t].arrangement.iter().any(|a| a.id == id && a.start < at && a.end() > at)).collect();
        if targets.is_empty() {
            if let TrackSel::Track(t) = self.sel_track {
                if let Some(tr) = self.project.tracks.get(t) {
                    targets = tr.arrangement.iter().filter(|a| a.start < at && a.end() > at).map(|a| (t, a.id)).collect();
                }
            }
        }
        if targets.is_empty() {
            self.toast("Nothing to cut at the playhead: pick a clip under it, or a track");
            return;
        }
        for (t, id) in targets {
            self.project.split_clip(t, id, at);
        }
    }

    /// Switches the picked clips off (or on again, when they all are off).
    pub(crate) fn arr_toggle_off(&mut self) {
        let sel = self.arr_selection();
        let all_off = sel.iter().all(|&(t, id)| self.project.tracks[t].arrangement.iter().any(|a| a.id == id && a.muted));
        for (t, id) in sel {
            if let Some(a) = self.project.tracks[t].arrangement.iter_mut().find(|a| a.id == id) {
                a.muted = !all_off;
            }
        }
    }
}

//! Automation on the arrangement, as in Live: with Automation on, a track
//! shows one parameter's line over its clips (pick it in the track's
//! header) and more in lanes of their own under it (+). Click to put a
//! point, drag a point to move it, double-click it to take it away; with
//! Draw, paint the line along the grid. The line moves the mixer, the
//! instrument and the effects while the track follows the arrangement.

use crate::app::App;
use crate::theme::*;
use crate::ui::envelope_view::EnvTarget;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, Id, Rect, RichText, Sense, Stroke, Ui, UiBuilder};
use krabton_engine::modulation::ModTarget;

/// The colour of automation, as in Live.
pub(crate) const AUTO: Color32 = Color32::from_rgb(0xff, 0x52, 0x52);
/// Where a point is grabbed (pixels).
const GRAB: f32 = 7.0;

/// A point (or a painting) being dragged.
#[derive(Clone, Copy)]
pub(crate) struct AutoDrag {
    pub track: usize,
    pub target: ModTarget,
    /// The point being moved; None while painting.
    pub point: Option<usize>,
}

fn snap(v: f64, grid: f64, free: bool) -> f64 {
    if free || grid <= 0.0 {
        v
    } else {
        (v / grid).round() * grid
    }
}

impl App {
    /// The name of a target on track `t`, with its group.
    pub(crate) fn auto_target_info(&self, t: usize, target: ModTarget) -> Option<EnvTarget> {
        self.env_targets(t).into_iter().find(|e| e.target == target)
    }

    /// A chooser of what a lane shows: every parameter of the track, the
    /// ones with a line marked.
    pub(crate) fn auto_chooser(&mut self, ui: &mut Ui, rect: Rect, t: usize, current: Option<ModTarget>, id: Id) -> Option<Option<ModTarget>> {
        let targets = self.env_targets(t);
        let has = |tg: ModTarget| self.project.tracks.get(t).and_then(|tr| tr.automation_of(tg)).is_some_and(|e| !e.points.is_empty());
        let label = match current.and_then(|c| targets.iter().find(|e| e.target == c)) {
            Some(e) => format!("{}{}", if has(e.target) { "~ " } else { "" }, e.name),
            None => "None".to_string(),
        };
        let mut out = None;
        let mut child = ui.new_child(UiBuilder::new().max_rect(rect).layout(egui::Layout::left_to_right(egui::Align::Center)));
        egui::ComboBox::from_id_salt(id).width(rect.width() - 8.0).height(420.0).selected_text(RichText::new(label).small()).show_ui(&mut child, |ui| {
            if ui.selectable_label(current.is_none(), "None").clicked() {
                out = Some(None);
            }
            let mut group = String::new();
            for e in &targets {
                if e.group != group {
                    group = e.group.clone();
                    ui.label(RichText::new(&group).small().color(TEXT_DIM));
                }
                let mark = if has(e.target) { "~ " } else { "   " };
                let text = RichText::new(format!("{mark}{}", e.name)).color(if has(e.target) { AUTO } else { TEXT });
                if ui.selectable_label(current == Some(e.target), text).clicked() {
                    out = Some(Some(e.target));
                }
            }
        });
        out
    }

    /// A lane's automation line over `rect` (`over_clips`: on the track's
    /// own lane, leaving the clips' title bars free).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn auto_lane(&mut self, ui: &mut Ui, t: usize, target: ModTarget, rect: Rect, bx: &dyn Fn(f64) -> f32, xb: &dyn Fn(f32) -> f64, free: bool, over_clips: bool) {
        let Some(track) = self.project.tracks.get(t) else { return };
        let tid = track.id;
        let area = if over_clips { Rect::from_min_max(pos2(rect.left(), rect.top() + 14.0), rect.max) } else { rect.shrink2(vec2(0.0, 2.0)) };
        if area.height() < 6.0 {
            return;
        }
        let p = ui.painter().clone().with_clip_rect(rect.intersect(ui.clip_rect()));
        if over_clips {
            // The clips step back so the line stands out.
            p.rect_filled(area, 0.0, Color32::from_black_alpha(80));
        }
        let y_of = |v: f32| area.bottom() - 2.0 - v.clamp(0.0, 1.0) * (area.height() - 4.0);
        let v_of = |y: f32| ((area.bottom() - 2.0 - y) / (area.height() - 4.0)).clamp(0.0, 1.0);
        let env = track.automation_of(target).cloned();
        let base = self.env_base(t, target).unwrap_or(0.5);
        let grid = self.arr.grid;
        let pointer = ui.input(|i| i.pointer.hover_pos());
        let near = |pts: &[(f64, f32)], pos: egui::Pos2| pts.iter().position(|(b, v)| (bx(*b) - pos.x).abs() < GRAB && (y_of(*v) - pos.y).abs() < GRAB);

        // The line.
        match env.as_ref().filter(|e| !e.points.is_empty()) {
            Some(e) => {
                let (b0, b1) = (xb(area.left()), xb(area.right()));
                let mut line = vec![pos2(area.left(), y_of(e.value_at(b0).unwrap_or(base)))];
                for &(b, v) in e.points.iter().filter(|(b, _)| *b > b0 && *b < b1) {
                    line.push(pos2(bx(b), y_of(v)));
                }
                line.push(pos2(area.right(), y_of(e.value_at(b1).unwrap_or(base))));
                p.add(egui::Shape::line(line, Stroke::new(1.6_f32, AUTO)));
                let hot = pointer.filter(|pt| area.contains(*pt)).and_then(|pt| near(&e.points, pt));
                for (k, &(b, v)) in e.points.iter().enumerate() {
                    let x = bx(b);
                    if x < area.left() - 4.0 || x > area.right() + 4.0 {
                        continue;
                    }
                    let big = hot == Some(k) || self.arr.auto_drag.is_some_and(|d| d.track == t && d.target == target && d.point == Some(k));
                    p.circle_filled(pos2(x, y_of(v)), if big { 4.5 } else { 3.0 }, if big { Color32::WHITE } else { AUTO });
                }
            }
            None => {
                // No line yet: where the parameter is now, faint.
                let y = y_of(base);
                p.line_segment([pos2(area.left(), y), pos2(area.right(), y)], Stroke::new(1.0_f32, AUTO.gamma_multiply(0.4)));
            }
        }

        // Editing.
        let id = Id::new(("auto_lane", tid, format!("{target:?}"), over_clips));
        let resp = ui.interact(area, id, Sense::click_and_drag());
        let draw = self.arr.auto_draw;
        let step = if grid > 0.0 { grid.min(1.0) } else { 0.25 };
        if resp.drag_started() {
            if let Some(pos) = ui.input(|i| i.pointer.press_origin()) {
                let pts = env.as_ref().map(|e| e.points.clone()).unwrap_or_default();
                if let Some(k) = near(&pts, pos) {
                    self.arr.auto_drag = Some(AutoDrag { track: t, target, point: Some(k) });
                } else if draw {
                    self.arr.auto_drag = Some(AutoDrag { track: t, target, point: None });
                } else {
                    // A new point, and on with it.
                    let beat = snap(xb(pos.x).max(0.0), step, free);
                    let e = self.project.tracks[t].automation_mut(target);
                    e.set(beat, v_of(pos.y), 1e-6);
                    let k = e.points.iter().position(|p| (p.0 - beat).abs() < 1e-9);
                    self.arr.auto_drag = Some(AutoDrag { track: t, target, point: k });
                }
            }
        }
        if resp.dragged() {
            if let (Some(d), Some(pos)) = (self.arr.auto_drag, resp.interact_pointer_pos()) {
                if d.track == t && d.target == target {
                    let e = self.project.tracks[t].automation_mut(target);
                    match d.point {
                        Some(k) if k < e.points.len() => {
                            // Between its neighbours, so the line stays in order.
                            let lo = if k > 0 { e.points[k - 1].0 } else { 0.0 };
                            let hi = e.points.get(k + 1).map(|p| p.0).unwrap_or(f64::MAX);
                            let beat = snap(xb(pos.x), step, free).clamp(lo, hi);
                            e.points[k] = (beat, v_of(pos.y));
                        }
                        None => {
                            let beat = snap(xb(pos.x).max(0.0), step, false);
                            e.set(beat, v_of(pos.y), step * 0.49);
                        }
                        _ => {}
                    }
                }
            }
        }
        if resp.drag_stopped() {
            self.arr.auto_drag = None;
        }
        if resp.clicked() {
            if let Some(pos) = resp.interact_pointer_pos() {
                let pts = env.as_ref().map(|e| e.points.clone()).unwrap_or_default();
                if near(&pts, pos).is_none() {
                    let beat = snap(xb(pos.x).max(0.0), step, free);
                    self.project.tracks[t].automation_mut(target).set(beat, v_of(pos.y), 1e-6);
                }
            }
        }
        if resp.double_clicked() {
            if let Some(pos) = resp.interact_pointer_pos() {
                let e = self.project.tracks[t].automation_mut(target);
                if let Some(k) = near(&e.points, pos) {
                    e.points.remove(k);
                }
            }
        }
        // What the line says where the pointer is.
        if let Some(pos) = resp.hover_pos() {
            if let Some(info) = self.auto_target_info(t, target) {
                let v = env.as_ref().and_then(|e| e.value_at(xb(pos.x))).unwrap_or(base);
                let at = v_of(pos.y);
                let txt = format!("{}: {}  (here {})", info.name, info.format(at), info.format(v));
                p.text(pos2(pos.x + 10.0, (pos.y - 12.0).max(area.top() + 6.0)), Align2::LEFT_CENTER, txt, small(), Color32::WHITE);
            }
        }
        let has = env.as_ref().is_some_and(|e| !e.points.is_empty());
        resp.context_menu(|ui| {
            if ui.add_enabled(has, egui::Button::new("Clear this automation")).clicked() {
                if let Some(tr) = self.project.tracks.get_mut(t) {
                    tr.automation.retain(|e| e.target != target);
                }
                ui.close();
            }
            if !over_clips && ui.button("Hide this lane").clicked() {
                if let Some(tr) = self.project.tracks.get_mut(t) {
                    tr.auto_lanes.retain(|x| *x != target);
                }
                ui.close();
            }
        });
    }

    /// The header of an extra automation lane: what it shows, and x.
    pub(crate) fn auto_lane_head(&mut self, ui: &mut Ui, t: usize, k: usize, r: Rect) {
        let Some(target) = self.project.tracks.get(t).and_then(|tr| tr.auto_lanes.get(k)).copied() else { return };
        let tid = self.project.tracks[t].id;
        let p = ui.painter().clone();
        rect_fill(&p, r, 0.0, PANEL2);
        rect_fill(&p, Rect::from_min_size(r.min, vec2(3.0, r.height())), 0.0, AUTO.gamma_multiply(0.6));
        let chooser = Rect::from_min_max(pos2(r.left() + 10.0, r.top() + 4.0), pos2(r.right() - 26.0, r.top() + 24.0));
        if let Some(pick) = self.auto_chooser(ui, chooser, t, Some(target), Id::new(("auto_lane_pick", tid, k))) {
            if let Some(tr) = self.project.tracks.get_mut(t) {
                match pick {
                    Some(new) if !tr.auto_lanes.contains(&new) => tr.auto_lanes[k] = new,
                    None => {
                        tr.auto_lanes.remove(k);
                    }
                    _ => {}
                }
            }
        }
        let x = Rect::from_min_size(pos2(r.right() - 22.0, r.top() + 5.0), vec2(16.0, 16.0));
        if button(ui, x, Id::new(("auto_lane_x", tid, k)), "x", false, ACCENT).on_hover_text("Hide this lane (its automation stays)").clicked() {
            if let Some(tr) = self.project.tracks.get_mut(t) {
                if k < tr.auto_lanes.len() {
                    tr.auto_lanes.remove(k);
                }
            }
        }
    }

    /// Adds a lane under track `t`: the next parameter with a line not shown
    /// yet, or the volume.
    pub(crate) fn add_auto_lane(&mut self, t: usize) {
        let targets = self.env_targets(t);
        let Some(tr) = self.project.tracks.get(t) else { return };
        let shown = |tg: ModTarget| tr.auto_shown == Some(tg) || tr.auto_lanes.contains(&tg);
        let next = tr
            .automation
            .iter()
            .filter(|e| !e.points.is_empty())
            .map(|e| e.target)
            .find(|tg| !shown(*tg))
            .or_else(|| targets.iter().map(|e| e.target).find(|tg| !shown(*tg)));
        if let (Some(n), Some(tr)) = (next, self.project.tracks.get_mut(t)) {
            tr.auto_lanes.push(n);
        }
    }
}

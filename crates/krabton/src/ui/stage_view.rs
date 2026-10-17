//! The Stage: a map of the room seen from above, every track with a Stage
//! on it as a name tag where it stands, the listener's head where it sits.
//! Drag a track to move it, the head to move the listener. The device
//! shows it beside its knobs; a window shows it big, with the room's
//! settings and the tracks not on the stage yet.

use super::modulation::ModView;
use crate::app::App;
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, Context, FontId, Id, Pos2, Rect, RichText, Sense, Shape, Stroke, Ui};
use krabton_engine::dsp::effects::EffectKind;
use krabton_engine::dsp::fx::stage as st;
use krabton_engine::params::ParamKind;

const WALL: Color32 = Color32::from_rgb(0x3a, 0x22, 0x16);
const WALL_EDGE: Color32 = Color32::from_rgb(0x5c, 0x38, 0x24);
const FLOOR_A: Color32 = Color32::from_rgb(0x9a, 0x55, 0x2e);
const FLOOR_B: Color32 = Color32::from_rgb(0x8e, 0x4d, 0x29);
const CHALK: Color32 = Color32::from_rgba_premultiplied(0xc8, 0xc0, 0xb8, 0x70);

/// A track on the stage.
#[derive(Clone)]
pub(crate) struct Dot {
    pub dev: u64,
    pub name: String,
    pub color: Color32,
    pub across: f32,
    pub depth: f32,
    pub spread: f32,
    /// How loud it plays now (0 to 1).
    pub level: f32,
}

/// What a drag on the map did.
#[derive(Default)]
pub(crate) struct MapEdit {
    /// Tracks moved: (device, across, depth).
    pub moved: Vec<(u64, f32, f32)>,
    /// The listener moved (its depth).
    pub listener: Option<f32>,
}

#[derive(Clone, Copy, PartialEq)]
enum Grab {
    Dot(u64),
    Listener,
}

/// The room in `rect`: walls, a wooden floor, chalk arcs on the stage, the
/// tracks and the listener. `room` holds the room's parameters (a Stage's).
pub(crate) fn stage_map(ui: &mut Ui, rect: Rect, id: Id, room: &[f32], dots: &[Dot], selected: Option<u64>) -> MapEdit {
    let mut edit = MapEdit::default();
    if room.len() < st::SPECS.len() {
        return edit;
    }
    let resp = ui.interact(rect, id.with("map"), Sense::click_and_drag());
    let (w, d, _) = st::room(room);
    let avail = rect.shrink2(vec2(18.0, 16.0));
    let scale = (avail.width() / w).min(avail.height() / d).max(0.1);
    let floor = Rect::from_center_size(avail.center(), vec2(w * scale, d * scale));
    let to_screen = |x: f32, y: f32| pos2(floor.center().x + x * scale, floor.top() + y * scale);
    let from_screen = |p: Pos2| ((p.x - floor.center().x) / scale, (p.y - floor.top()) / scale);
    let painter = ui.painter_at(rect);
    rect_fill(&painter, rect, 3.0, Color32::from_rgb(0x12, 0x0c, 0x0a));
    // The walls and the floor's boards.
    painter.rect_filled(floor.expand(9.0), 5.0, WALL);
    painter.rect_stroke(floor.expand(9.0), 5.0, Stroke::new(1.5_f32, WALL_EDGE), egui::StrokeKind::Inside);
    let boards = ((w / 0.6).round() as usize).clamp(8, 60);
    let bw = floor.width() / boards as f32;
    for i in 0..boards {
        let r = Rect::from_min_size(pos2(floor.left() + i as f32 * bw, floor.top()), vec2(bw, floor.height()));
        painter.rect_filled(r, 0.0, if i % 2 == 0 { FLOOR_A } else { FLOOR_B });
    }
    // Two metres a square (drawn on the floor only).
    let on_floor = ui.painter_at(floor);
    let grid = Stroke::new(0.5_f32, Color32::from_rgba_premultiplied(0x20, 0x10, 0x08, 0x40));
    let mut gx = 2.0;
    while gx < w * 0.5 {
        for s in [-1.0, 1.0] {
            let x = floor.center().x + s * gx * scale;
            on_floor.line_segment([pos2(x, floor.top()), pos2(x, floor.bottom())], grid);
        }
        gx += 2.0;
    }
    let mut gy = 2.0;
    while gy < d {
        let y = floor.top() + gy * scale;
        on_floor.line_segment([pos2(floor.left(), y), pos2(floor.right(), y)], grid);
        gy += 2.0;
    }
    // The stage: chalk arcs round its front, opening towards the listener.
    let front = to_screen(0.0, 0.3);
    for k in 1..=3 {
        let radius = k as f32 * 0.17 * d * scale;
        let pts: Vec<Pos2> = (0..=40).map(|i| {
            let a = (-80.0 + 160.0 * i as f32 / 40.0).to_radians();
            pos2(front.x + radius * a.sin(), front.y + radius * a.cos())
        }).collect();
        on_floor.add(Shape::line(pts, Stroke::new(1.0_f32, CHALK)));
    }
    for a in [-50.0f32, -17.0, 17.0, 50.0] {
        let a = a.to_radians();
        let (r0, r1) = (0.17 * d * scale, 0.51 * d * scale);
        on_floor.line_segment([pos2(front.x + r0 * a.sin(), front.y + r0 * a.cos()), pos2(front.x + r1 * a.sin(), front.y + r1 * a.cos())], Stroke::new(0.8_f32, CHALK));
    }
    // The listener: a head, its nose to the front.
    let mut p = room.to_vec();
    let ((_, _), (lx, ly)) = st::places(&p);
    let head = to_screen(lx, ly);
    for (dx, dy) in [(-9.0, 4.0), (9.0, 4.0), (0.0, -6.0)] {
        painter.circle(head + vec2(dx, dy), 12.0, Color32::from_rgba_premultiplied(0xe8, 0xe4, 0xe0, 0x50), Stroke::new(1.0_f32, Color32::from_gray(230)));
    }
    painter.circle_filled(head, 5.0, Color32::from_gray(40));
    painter.add(Shape::convex_polygon(vec![head + vec2(0.0, -15.0), head + vec2(-4.0, -9.0), head + vec2(4.0, -9.0)], Color32::from_gray(40), Stroke::NONE));
    // Where each one is, on the screen.
    let half = |spread: f32| spread.clamp(0.0, 1.0) * (w * 0.12).min(3.0);
    let mut place = |dot: &Dot| {
        p[st::ACROSS] = dot.across;
        p[st::DEPTH] = dot.depth;
        let ((sx, sy), _) = st::places(&p);
        (sx, sy)
    };
    let spots: Vec<Pos2> = dots.iter().map(|dt| {
        let (sx, sy) = place(dt);
        to_screen(sx, sy)
    }).collect();
    // A drag takes the nearest within reach (the selected first), or the head.
    let grab_key = id.with("grab");
    let pointer = resp.interact_pointer_pos().or(resp.hover_pos());
    if resp.drag_started() {
        let at = resp.interact_pointer_pos().unwrap_or(rect.center());
        let mut best: Option<(f32, Grab)> = None;
        for (dt, q) in dots.iter().zip(&spots) {
            let dist = q.distance(at) - if Some(dt.dev) == selected { 4.0 } else { 0.0 };
            if dist < 18.0 && best.is_none_or(|b| dist < b.0) {
                best = Some((dist, Grab::Dot(dt.dev)));
            }
        }
        if best.is_none() && head.distance(at) < 24.0 {
            best = Some((0.0, Grab::Listener));
        }
        ui.data_mut(|dd| dd.insert_temp(grab_key, best.map(|b| b.1)));
    }
    let grabbed: Option<Grab> = ui.data(|dd| dd.get_temp::<Option<Grab>>(grab_key).flatten());
    if let (true, Some(g), Some(at)) = (resp.dragged(), grabbed, resp.interact_pointer_pos()) {
        let (x, y) = from_screen(at);
        match g {
            Grab::Dot(dev) => {
                let across = (x / (w * 0.5 - 0.4)).clamp(-1.0, 1.0);
                let depth = ((y - 0.3) / (d - 0.6)).clamp(0.0, 1.0);
                edit.moved.push((dev, (across * 100.0).round() / 100.0, (depth * 100.0).round() / 100.0));
            }
            Grab::Listener => edit.listener = Some((((y - 0.3) / (d - 0.6)).clamp(0.0, 1.0) * 100.0).round() / 100.0),
        }
    }
    if resp.drag_stopped() {
        ui.data_mut(|dd| dd.insert_temp::<Option<Grab>>(grab_key, None));
    }
    // The tracks: a ring, its loudness a glow, a tag with its name, its width.
    let tag_font = FontId::proportional(11.0);
    let mut hovered: Option<usize> = None;
    for (i, (dt, &q)) in dots.iter().zip(&spots).enumerate() {
        let moved = edit.moved.iter().find(|m| m.0 == dt.dev);
        let q = match moved {
            Some(&(_, a, dp)) => {
                p[st::ACROSS] = a;
                p[st::DEPTH] = dp;
                let ((x, y), _) = st::places(&p);
                to_screen(x, y)
            }
            None => q,
        };
        let sel = Some(dt.dev) == selected;
        let hw = half(dt.spread) * scale;
        if hw > 4.0 {
            for s in [-1.0, 1.0] {
                let c = q + vec2(s * hw, 0.0);
                painter.line_segment([q, c], Stroke::new(1.0_f32, dt.color.gamma_multiply(0.6)));
                painter.circle(c, 4.5, Color32::from_gray(30), Stroke::new(1.5_f32, Color32::from_rgb(0xe0, 0xa8, 0x30)));
            }
        }
        if dt.level > 0.01 {
            painter.circle_filled(q, 8.0 + 10.0 * dt.level.min(1.0), dt.color.gamma_multiply(0.25 * dt.level.min(1.0)));
        }
        painter.circle(q, if sel { 8.0 } else { 6.5 }, dt.color, Stroke::new(if sel { 2.5_f32 } else { 1.5 }, if sel { Color32::WHITE } else { Color32::from_gray(25) }));
        // A little arrow to the listener.
        let to = (head - q).normalized();
        let tip = q + to * 13.0;
        let side = vec2(-to.y, to.x) * 3.5;
        painter.add(Shape::convex_polygon(vec![tip, q + to * 9.0 + side, q + to * 9.0 - side], dt.color, Stroke::NONE));
        // The name tag.
        let galley = painter.layout_no_wrap(dt.name.clone(), tag_font.clone(), Color32::WHITE);
        let tag = Rect::from_center_size(q + vec2(0.0, -17.0), galley.size() + vec2(8.0, 3.0));
        painter.rect_filled(tag, 2.0, if sel { dt.color } else { dt.color.gamma_multiply(0.85) });
        if sel {
            painter.rect_stroke(tag, 2.0, Stroke::new(1.0_f32, Color32::WHITE), egui::StrokeKind::Outside);
        }
        painter.galley(tag.min + vec2(4.0, 1.5), galley, Color32::WHITE);
        if pointer.is_some_and(|pt| pt.distance(q) < 14.0 || tag.contains(pt)) {
            hovered = Some(i);
        }
    }
    if dots.is_empty() {
        text(&painter, floor.center(), Align2::CENTER_CENTER, "No track on the stage yet", small(), TEXT);
    }
    // How far from the listener.
    if let Some(i) = hovered.filter(|_| grabbed.is_none()) {
        let dt = &dots[i];
        p[st::ACROSS] = dt.across;
        p[st::DEPTH] = dt.depth;
        resp.clone().on_hover_text(format!("{}: {:.1} m from the listener. Drag to move it", dt.name, st::distance(&p)));
    } else if pointer.is_some_and(|pt| pt.distance(head) < 24.0) {
        resp.clone().on_hover_text("The listener: drag up or down");
    }
    if grabbed.is_some() || dots.iter().any(|dt| dt.level > 0.01) {
        ui.ctx().request_repaint();
    }
    edit
}

/// What the device asked for besides its own knobs.
#[derive(Default)]
pub(crate) struct StageAct {
    pub map: MapEdit,
    pub every_track: bool,
    pub window: bool,
}

/// The device: the map (as wide as dragged), buttons on it, the knobs.
#[allow(clippy::too_many_arguments)]
pub(crate) fn stage_panel(ui: &mut Ui, id: Id, dev_id: u64, p: &mut Vec<f32>, accent: Color32, h: f32, mv: Option<&mut ModView>, dots: &[Dot], width: &mut f32) -> (bool, StageAct) {
    let mut act = StageAct::default();
    let mut changed = false;
    if p.len() < st::SPECS.len() {
        return (false, act);
    }
    ui.horizontal_top(|ui| {
        let (rect, _) = ui.allocate_exact_size(vec2(*width, (h - 30.0).max(120.0)), Sense::hover());
        // This device's own place as it is now (its knobs may have moved it).
        let mut shown: Vec<Dot> = dots.to_vec();
        if let Some(me) = shown.iter_mut().find(|d| d.dev == dev_id) {
            me.across = p[st::ACROSS];
            me.depth = p[st::DEPTH];
            me.spread = p[st::SPREAD];
        }
        act.map = stage_map(ui, rect, id, p, &shown, Some(dev_id));
        width_grip(ui, id.with("display_width"), rect, width, 240.0, 1400.0);
        if let Some(&(_, a, d)) = act.map.moved.iter().find(|m| m.0 == dev_id) {
            p[st::ACROSS] = a;
            p[st::DEPTH] = d;
            changed = true;
        }
        if let Some(l) = act.map.listener {
            p[st::LISTENER] = l;
            changed = true;
        }
        let b1 = Rect::from_min_size(pos2(rect.left() + 6.0, rect.bottom() - 24.0), vec2(96.0, 18.0));
        if ui.put(b1, egui::Button::new(RichText::new("On every track").small())).on_hover_text("A Stage on every track that has none yet, the tracks spread over the stage").clicked() {
            act.every_track = true;
        }
        let b2 = Rect::from_min_size(pos2(b1.right() + 4.0, b1.top()), vec2(62.0, 18.0));
        if ui.put(b2, egui::Button::new(RichText::new("Big map").small())).on_hover_text("The stage in a window of its own, with every track").clicked() {
            act.window = true;
        }
        ui.add_space(6.0);
        changed |= super::devices::param_grid_mod(ui, id, st::SPECS, p, accent, h - 24.0, mv);
    });
    (changed, act)
}

impl App {
    /// Every Stage on a track: where it stands, how loud the track is.
    pub(crate) fn stage_dots(&self) -> Vec<Dot> {
        let mut out = Vec::new();
        for (i, t) in self.project.tracks.iter().enumerate() {
            for dev in t.devices.iter().filter(|d| d.kind == EffectKind::Stage && d.params.len() >= st::SPECS.len()) {
                let level = self.meters.get(i).map(|m| m.0.max(m.1)).unwrap_or(0.0);
                out.push(Dot { dev: dev.id, name: t.name.clone(), color: rgb(t.color), across: dev.params[st::ACROSS], depth: dev.params[st::DEPTH], spread: dev.params[st::SPREAD], level });
            }
        }
        out
    }

    /// Every Stage in the set, mutably (tracks, returns and the master).
    fn stage_devices_mut(&mut self) -> impl Iterator<Item = &mut krabton_engine::Device> {
        let p = &mut self.project;
        p.tracks.iter_mut().flat_map(|t| t.devices.iter_mut()).chain(p.returns.iter_mut().flat_map(|r| r.devices.iter_mut())).chain(p.master.devices.iter_mut()).filter(|d| d.kind == EffectKind::Stage)
    }

    /// The room as `params` has it, into every Stage: one room for all.
    pub(crate) fn share_stage_room(&mut self, params: &[f32]) {
        if params.len() < st::SPECS.len() {
            return;
        }
        for dev in self.stage_devices_mut() {
            if dev.params.len() < st::SPECS.len() {
                continue;
            }
            for &k in &st::ROOM {
                if dev.params[k] != params[k] {
                    dev.params[k] = params[k];
                }
            }
        }
    }

    /// Tracks moved on a map.
    pub(crate) fn apply_stage_map(&mut self, edit: &MapEdit) {
        for &(dev, across, depth) in &edit.moved {
            if let Some(d) = self.stage_devices_mut().find(|d| d.id == dev) {
                d.params[st::ACROSS] = across;
                d.params[st::DEPTH] = depth;
            }
        }
        if let Some(l) = edit.listener {
            for d in self.stage_devices_mut() {
                d.params[st::LISTENER] = l;
            }
        }
    }

    /// A Stage on every track without one, at the end of its chain, in the
    /// room the others share; the tracks spread over the stage like a band
    /// (drums at the back).
    pub(crate) fn stage_every_track(&mut self) {
        let room: Option<Vec<f32>> = self.project.all_devices().find(|d| d.kind == EffectKind::Stage).map(|d| d.params.clone());
        let missing: Vec<usize> = (0..self.project.tracks.len()).filter(|&i| !self.project.tracks[i].devices.iter().any(|d| d.kind == EffectKind::Stage)).collect();
        let others: Vec<usize> = missing.iter().copied().filter(|&i| !self.project.tracks[i].instrument.as_ref().is_some_and(|x| x.is_drums())).collect();
        let n = others.len().max(1);
        for &i in &missing {
            let mut dev = self.project.make_device(EffectKind::Stage);
            if let Some(r) = &room {
                for &k in &st::ROOM {
                    dev.params[k] = r[k];
                }
            }
            let drums = self.project.tracks[i].instrument.as_ref().is_some_and(|x| x.is_drums());
            if drums {
                dev.params[st::ACROSS] = 0.0;
                dev.params[st::DEPTH] = 0.05;
                dev.params[st::SPREAD] = 0.6;
            } else {
                let j = others.iter().position(|&o| o == i).unwrap_or(0);
                let a = if n == 1 { 0.0 } else { (-65.0 + 130.0 * j as f32 / (n - 1) as f32).to_radians() };
                dev.params[st::ACROSS] = ((0.8 * a.sin()) * 100.0).round() / 100.0;
                dev.params[st::DEPTH] = ((0.08 + 0.3 * a.cos()) * 100.0).round() / 100.0;
            }
            self.project.tracks[i].devices.push(dev);
        }
        self.toast(if missing.is_empty() { "Every track is on the stage already".to_string() } else { format!("{} track(s) put on the stage", missing.len()) });
    }

    /// The stage, big: the map with every track, the room's settings, and
    /// the tracks not on it yet.
    pub(crate) fn ui_stage_window(&mut self, ctx: &Context) {
        if !self.stage_window {
            return;
        }
        let mut open = true;
        let dots = self.stage_dots();
        let room: Option<Vec<f32>> = self.project.all_devices().find(|d| d.kind == EffectKind::Stage).map(|d| d.params.clone());
        let mut edit = None;
        let mut room_new: Option<Vec<f32>> = None;
        let mut every = false;
        let mut add: Option<usize> = None;
        let missing: Vec<(usize, String)> = self.project.tracks.iter().enumerate().filter(|(_, t)| !t.devices.iter().any(|d| d.kind == EffectKind::Stage)).map(|(i, t)| (i, t.name.clone())).collect();
        egui::Window::new("Stage").open(&mut open).default_size(vec2(900.0, 620.0)).resizable(true).show(ctx, |ui| {
            match &room {
                Some(r) => {
                    let mut r = r.clone();
                    let mut touched = false;
                    ui.horizontal_wrapped(|ui| {
                        for &k in &st::ROOM {
                            let spec = &st::SPECS[k];
                            match spec.kind {
                                ParamKind::Toggle => {
                                    let mut on = r[k] >= 0.5;
                                    if ui.checkbox(&mut on, spec.name).changed() {
                                        r[k] = if on { 1.0 } else { 0.0 };
                                        touched = true;
                                    }
                                }
                                ParamKind::Choice(names) => {
                                    let cur = (r[k].round().max(0.0) as usize).min(names.len() - 1);
                                    egui::ComboBox::from_id_salt(("stage_room", k)).width(96.0).selected_text(names[cur]).show_ui(ui, |ui| {
                                        for (i, nm) in names.iter().enumerate() {
                                            if ui.selectable_label(i == cur, *nm).clicked() {
                                                r[k] = i as f32;
                                                touched = true;
                                            }
                                        }
                                    });
                                }
                                _ => {
                                    ui.label(RichText::new(spec.name).small().color(TEXT_DIM));
                                    let mut v = r[k];
                                    if ui.add(egui::Slider::new(&mut v, spec.min..=spec.max).show_value(true)).changed() {
                                        r[k] = v;
                                        touched = true;
                                    }
                                }
                            }
                        }
                        ui.label(RichText::new(format!("tail {:.1} s", st::rt60(&r))).small().color(TEXT_DIM));
                    });
                    if touched {
                        room_new = Some(r.clone());
                    }
                    let avail = ui.available_size();
                    let (rect, _) = ui.allocate_exact_size(vec2(avail.x, (avail.y - 34.0).max(200.0)), Sense::hover());
                    edit = Some(stage_map(ui, rect, Id::new("stage_window"), &r, &dots, None));
                }
                None => {
                    ui.label("No track has a Stage yet.");
                }
            }
            ui.horizontal_wrapped(|ui| {
                if !missing.is_empty() {
                    ui.label(RichText::new("Not on the stage:").small().color(TEXT_DIM));
                    for (i, name) in &missing {
                        if ui.small_button(format!("+ {name}")).on_hover_text("Put it on the stage").clicked() {
                            add = Some(*i);
                        }
                    }
                    if ui.small_button("All of them").clicked() {
                        every = true;
                    }
                }
            });
        });
        if let Some(r) = room_new {
            self.share_stage_room(&r);
        }
        if let Some(e) = edit {
            self.apply_stage_map(&e);
        }
        if every {
            self.stage_every_track();
        }
        if let Some(i) = add {
            let mut dev = self.project.make_device(EffectKind::Stage);
            if let Some(r) = &room {
                for &k in &st::ROOM {
                    dev.params[k] = r[k];
                }
            }
            dev.params[st::DEPTH] = 0.4;
            if let Some(t) = self.project.tracks.get_mut(i) {
                t.devices.push(dev);
            }
        }
        if !open {
            self.stage_window = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Event, Modifiers, PointerButton, RawInput};
    use krabton_engine::params::defaults;

    /// Frames of pointer moves (and presses) over a map of `dots`; the edits.
    fn drive(dots: &[Dot], frames: &[(Pos2, Option<bool>)]) -> Vec<MapEdit> {
        let ctx = egui::Context::default();
        let room = defaults(st::SPECS);
        let mut edits = Vec::new();
        for f in std::iter::once(None).chain(frames.iter().map(Some)) {
            let mut input = RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(800.0, 700.0))), ..Default::default() };
            if let Some(&(pos, press)) = f {
                input.events.push(Event::PointerMoved(pos));
                if let Some(pressed) = press {
                    input.events.push(Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE });
                }
            }
            let _ = ctx.run(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    edits.push(stage_map(ui, Rect::from_min_size(pos2(0.0, 0.0), vec2(600.0, 600.0)), Id::new("t"), &room, dots, None));
                });
            });
        }
        edits
    }

    #[test]
    fn dragging_a_track_moves_it_and_the_head_moves_the_listener() {
        let dot = Dot { dev: 7, name: "Bass".into(), color: Color32::RED, across: 0.0, depth: 0.3, spread: 0.0, level: 0.0 };
        // Where it is: the room (16 x 22.4 m) fitted into 600 x 600 less the margins.
        let room = defaults(st::SPECS);
        let (w, d, _) = st::room(&room);
        let scale = ((600.0 - 36.0) / w).min((600.0 - 32.0) / d);
        let top = 300.0 - d * scale / 2.0;
        let ((sx, sy), (_, ly)) = st::places(&room);
        let at = pos2(300.0 + sx * scale, top + sy * scale);
        let to = at + vec2(120.0, 60.0);
        let edits = drive(&[dot.clone()], &[(at, Some(true)), (at + vec2(10.0, 5.0), None), (to, None), (to, Some(false))]);
        let moved = edits.iter().flat_map(|e| e.moved.iter()).last().copied().expect("moved");
        assert_eq!(moved.0, 7);
        assert!(moved.1 > 0.2 && moved.2 > 0.35, "{moved:?}");
        // The head.
        let head = pos2(300.0, top + ly * scale);
        let edits = drive(&[dot], &[(head, Some(true)), (head + vec2(0.0, -10.0), None), (head + vec2(0.0, -80.0), None), (head + vec2(0.0, -80.0), Some(false))]);
        let l = edits.iter().filter_map(|e| e.listener).last().expect("listener");
        assert!(l < 0.8, "{l}");
    }
}

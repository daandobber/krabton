//! The Map: the room seen from above with the tracks you put on it, each a
//! name tag where it stands with a fan the way it faces, the listener's head
//! where it sits. Drag a track to move it, the dot on its fan to turn it,
//! the head to move the listener; right-click a track for more. Beside it
//! the chosen track's place and the tracks to put on the map; below a
//! mixer. Above it the room: none at first (only where they stand), as
//! much of it as you like.

use super::windows::tool_window;
use crate::app::App;
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, Context, FontId, Id, Pos2, Rect, RichText, Sense, Shape, Stroke, Ui, UiBuilder};
use krabton_engine::dsp::fx::stage::{self as st, MapRoom, Place};

const WALL: Color32 = Color32::from_rgb(0x3a, 0x22, 0x16);
const WALL_EDGE: Color32 = Color32::from_rgb(0x5c, 0x38, 0x24);
const FLOOR_A: Color32 = Color32::from_rgb(0x9a, 0x55, 0x2e);
const FLOOR_B: Color32 = Color32::from_rgb(0x8e, 0x4d, 0x29);
const CHALK: Color32 = Color32::from_rgba_premultiplied(0xc8, 0xc0, 0xb8, 0x70);

/// A track on the map.
#[derive(Clone)]
pub(crate) struct Dot {
    pub track: u64,
    pub name: String,
    pub color: Color32,
    pub place: Place,
    /// How loud it plays now (0 to 1).
    pub level: f32,
}

/// What the mouse did on the map.
#[derive(Default)]
pub(crate) struct MapEdit {
    pub moved: Option<(u64, f32, f32)>,
    pub turned: Option<(u64, f32)>,
    pub listener: Option<f32>,
    pub select: Option<u64>,
    pub face_listener: Option<u64>,
    pub remove: Option<u64>,
}

#[derive(Clone, Copy, PartialEq)]
enum Grab {
    /// A track, and where on it the mouse took it (so it does not jump).
    Move(u64, egui::Vec2),
    Turn(u64),
    Listener,
}

/// A Stage's parameters for the room and a place in it.
fn params(room: &MapRoom, place: &Place) -> [f32; 15] {
    st::map_params(room, place)
}

/// Where the handle that turns a track sits: on its fan, so far out.
const HANDLE: f32 = 30.0;

/// The map in `rect`.
pub(crate) fn map(ui: &mut Ui, rect: Rect, id: Id, room: &MapRoom, dots: &[Dot], selected: Option<u64>) -> MapEdit {
    let mut edit = MapEdit::default();
    let resp = ui.interact(rect, id.with("map"), Sense::click_and_drag());
    let base = params(room, &Place::default());
    let (w, d, _) = st::room(&base);
    let avail = rect.shrink2(vec2(22.0, 20.0));
    let scale = (avail.width() / w).min(avail.height() / d).max(0.1);
    let floor = Rect::from_center_size(avail.center(), vec2(w * scale, d * scale));
    let to_screen = |x: f32, y: f32| pos2(floor.center().x + x * scale, floor.top() + y * scale);
    let from_screen = |p: Pos2| ((p.x - floor.center().x) / scale, (p.y - floor.top()) / scale);
    let painter = ui.painter_at(rect);
    rect_fill(&painter, rect, 3.0, Color32::from_rgb(0x12, 0x0c, 0x0a));
    painter.rect_filled(floor.expand(9.0), 5.0, WALL);
    painter.rect_stroke(floor.expand(9.0), 5.0, Stroke::new(1.5_f32, WALL_EDGE), egui::StrokeKind::Inside);
    let boards = ((w / 0.6).round() as usize).clamp(8, 60);
    let bw = floor.width() / boards as f32;
    for i in 0..boards {
        let r = Rect::from_min_size(pos2(floor.left() + i as f32 * bw, floor.top()), vec2(bw, floor.height()));
        painter.rect_filled(r, 0.0, if i % 2 == 0 { FLOOR_A } else { FLOOR_B });
    }
    let on_floor = ui.painter_at(floor);
    let grid = Stroke::new(0.5_f32, Color32::from_rgba_premultiplied(0x20, 0x10, 0x08, 0x40));
    let mut g = 2.0;
    while g < w.max(d) {
        for s in [-1.0, 1.0] {
            let x = floor.center().x + s * g * scale;
            on_floor.line_segment([pos2(x, floor.top()), pos2(x, floor.bottom())], grid);
        }
        let y = floor.top() + g * scale;
        on_floor.line_segment([pos2(floor.left(), y), pos2(floor.right(), y)], grid);
        g += 2.0;
    }
    // The stage's chalk arcs, opening towards the listener.
    let front = to_screen(0.0, 0.3);
    for k in 1..=3 {
        let radius = k as f32 * 0.15 * d * scale;
        let pts: Vec<Pos2> = (0..=40).map(|i| {
            let a = (-80.0 + 160.0 * i as f32 / 40.0).to_radians();
            pos2(front.x + radius * a.sin(), front.y + radius * a.cos())
        }).collect();
        on_floor.add(Shape::line(pts, Stroke::new(1.0_f32, CHALK)));
    }
    // The listener.
    let ((_, _), (lx, ly)) = st::places(&base);
    let head = to_screen(lx, ly);
    for (dx, dy) in [(-9.0, 4.0), (9.0, 4.0), (0.0, -6.0)] {
        painter.circle(head + vec2(dx, dy), 12.0, Color32::from_rgba_premultiplied(0xe8, 0xe4, 0xe0, 0x50), Stroke::new(1.0_f32, Color32::from_gray(230)));
    }
    painter.circle_filled(head, 5.0, Color32::from_gray(40));
    painter.add(Shape::convex_polygon(vec![head + vec2(0.0, -15.0), head + vec2(-4.0, -9.0), head + vec2(4.0, -9.0)], Color32::from_gray(40), Stroke::NONE));

    // Where each one is now on the screen, its tag, and its handle.
    let tag_font = FontId::proportional(11.5);
    let spot = |pl: &Place| {
        let mut p = base;
        p[st::ACROSS] = pl.across;
        p[st::DEPTH] = pl.depth;
        let ((sx, sy), _) = st::places(&p);
        to_screen(sx, sy)
    };
    let handle_of = |q: Pos2, facing: f32| {
        // Facing 0 points at the back (down the screen); 90 to the right.
        let (fx, fy) = st::facing_vector(facing);
        q + vec2(fx, fy) * HANDLE
    };
    let tags: Vec<Rect> = dots.iter().map(|dt| {
        let q = spot(&dt.place);
        let galley = painter.layout_no_wrap(dt.name.clone(), tag_font.clone(), Color32::WHITE);
        Rect::from_center_size(q + vec2(0.0, -19.0), galley.size() + vec2(10.0, 4.0))
    }).collect();
    // What is under the mouse: the selected one's handle first, then tracks (the selected on top), then the head.
    let hit = |at: Pos2| -> Option<Grab> {
        if let Some(dt) = dots.iter().find(|d| Some(d.track) == selected) {
            if handle_of(spot(&dt.place), dt.place.facing).distance(at) < 11.0 {
                return Some(Grab::Turn(dt.track));
            }
        }
        let mut best: Option<(f32, Grab)> = None;
        for (dt, tag) in dots.iter().zip(&tags) {
            let q = spot(&dt.place);
            let dist = if tag.contains(at) { 0.0 } else { q.distance(at) } - if Some(dt.track) == selected { 3.0 } else { 0.0 };
            if dist < 16.0 && best.is_none_or(|b| dist < b.0) {
                best = Some((dist, Grab::Move(dt.track, at - q)));
            }
        }
        if let Some(b) = best {
            return Some(b.1);
        }
        (head.distance(at) < 24.0).then_some(Grab::Listener)
    };
    let grab_key = id.with("grab");
    if resp.drag_started() {
        let at = resp.interact_pointer_pos().unwrap_or(rect.center());
        let g = hit(at);
        if let Some(Grab::Move(t, _) | Grab::Turn(t)) = g {
            edit.select = Some(t);
        }
        ui.data_mut(|dd| dd.insert_temp(grab_key, g));
    }
    if resp.clicked() {
        if let Some(Grab::Move(t, _) | Grab::Turn(t)) = resp.interact_pointer_pos().and_then(hit) {
            edit.select = Some(t);
        }
    }
    let grabbed: Option<Grab> = ui.data(|dd| dd.get_temp::<Option<Grab>>(grab_key).flatten());
    if let (true, Some(g), Some(at)) = (resp.dragged(), grabbed, resp.interact_pointer_pos()) {
        match g {
            Grab::Move(t, off) => {
                let (x, y) = from_screen(at - off);
                let across = (x / (w * 0.5 - 0.4)).clamp(-1.0, 1.0);
                let depth = ((y - 0.3) / (d - 0.6)).clamp(0.0, 1.0);
                edit.moved = Some((t, (across * 100.0).round() / 100.0, (depth * 200.0).round() / 200.0));
            }
            Grab::Turn(t) => {
                if let Some(dt) = dots.iter().find(|d| d.track == t) {
                    let q = spot(&dt.place);
                    let v = at - q;
                    if v.length() > 4.0 {
                        let deg = v.x.atan2(v.y).to_degrees();
                        edit.turned = Some((t, (deg / 5.0).round() * 5.0));
                    }
                }
            }
            Grab::Listener => {
                let (_, y) = from_screen(at);
                edit.listener = Some((((y - 0.3) / (d - 0.6)).clamp(0.05, 1.0) * 100.0).round() / 100.0);
            }
        }
    }
    if resp.drag_stopped() {
        ui.data_mut(|dd| dd.insert_temp::<Option<Grab>>(grab_key, None));
    }
    // Right-click: more for the track under the mouse.
    let menu_key = id.with("menu_track");
    if resp.secondary_clicked() {
        let t = resp.interact_pointer_pos().and_then(hit).and_then(|g| match g {
            Grab::Move(t, _) | Grab::Turn(t) => Some(t),
            Grab::Listener => None,
        });
        ui.data_mut(|dd| dd.insert_temp(menu_key, t));
        if let Some(t) = t {
            edit.select = Some(t);
        }
    }
    let menu_track: Option<u64> = ui.data(|dd| dd.get_temp::<Option<u64>>(menu_key).flatten());
    resp.context_menu(|ui| match menu_track {
        Some(t) => {
            if ui.button("Face the listener").clicked() {
                edit.face_listener = Some(t);
                ui.close();
            }
            if ui.button("Take it off the map").clicked() {
                edit.remove = Some(t);
                ui.close();
            }
        }
        None => {
            ui.label(RichText::new("Right-click a track for more").small().color(TEXT_DIM));
        }
    });

    // The tracks: the way they face as a fan, a ring, a glow as they play, the tag.
    let hover = resp.hover_pos();
    let mut hovered: Option<usize> = None;
    let order: Vec<usize> = {
        let mut o: Vec<usize> = (0..dots.len()).collect();
        o.sort_by_key(|&i| Some(dots[i].track) == selected);
        o
    };
    for &i in &order {
        let dt = &dots[i];
        let mut pl = dt.place;
        if let Some((t, a, dp)) = edit.moved {
            if t == dt.track {
                pl.across = a;
                pl.depth = dp;
            }
        }
        if let Some((t, f)) = edit.turned {
            if t == dt.track {
                pl.facing = f;
            }
        }
        let q = spot(&pl);
        let sel = Some(dt.track) == selected;
        // The fan: narrow for a beam, wide for all round.
        let spread = (170.0 - 120.0 * pl.beam.clamp(0.0, 1.0)).to_radians() * 0.5;
        let a0 = pl.facing.to_radians();
        let reach = if sel { HANDLE + 6.0 } else { 22.0 };
        let mut fan = vec![q];
        for k in 0..=12 {
            let a = a0 - spread + 2.0 * spread * k as f32 / 12.0;
            fan.push(q + vec2(a.sin(), a.cos()) * reach);
        }
        painter.add(Shape::convex_polygon(fan, dt.color.gamma_multiply(if sel { 0.35 } else { 0.18 }), Stroke::NONE));
        if dt.level > 0.01 {
            painter.circle_filled(q, 8.0 + 10.0 * dt.level.min(1.0), dt.color.gamma_multiply(0.25 * dt.level.min(1.0)));
        }
        painter.circle(q, if sel { 8.0 } else { 6.5 }, dt.color, Stroke::new(if sel { 2.5_f32 } else { 1.5 }, if sel { Color32::WHITE } else { Color32::from_gray(25) }));
        if sel {
            let h = handle_of(q, pl.facing);
            painter.line_segment([q, h], Stroke::new(1.5_f32, Color32::WHITE));
            painter.circle(h, 6.0, dt.color, Stroke::new(2.0_f32, Color32::WHITE));
        }
        let galley = painter.layout_no_wrap(dt.name.clone(), tag_font.clone(), Color32::WHITE);
        let tag = Rect::from_center_size(q + vec2(0.0, -19.0), galley.size() + vec2(10.0, 4.0));
        painter.rect_filled(tag, 3.0, if sel { dt.color } else { dt.color.gamma_multiply(0.85) });
        if sel {
            painter.rect_stroke(tag, 3.0, Stroke::new(1.0_f32, Color32::WHITE), egui::StrokeKind::Outside);
        }
        painter.galley(tag.min + vec2(5.0, 2.0), galley, Color32::WHITE);
        if hover.is_some_and(|h| h.distance(q) < 14.0 || tag.contains(h)) {
            hovered = Some(i);
        }
    }
    if dots.is_empty() {
        text(&painter, floor.center(), Align2::CENTER_CENTER, "Tick a track (on the right) to put it on the map", small(), TEXT);
    }
    if grabbed.is_none() {
        if let Some(i) = hovered {
            let dt = &dots[i];
            let p = params(room, &dt.place);
            resp.clone().on_hover_text(format!("{}: {:.1} m from the listener. Drag to move it, the dot on its fan to turn it, right-click for more", dt.name, st::distance(&p)));
        } else if hover.is_some_and(|h| h.distance(head) < 24.0) {
            resp.clone().on_hover_text("The listener: drag up or down");
        }
    }
    if grabbed.is_some() || dots.iter().any(|dt| dt.level > 0.01) {
        ui.ctx().request_repaint();
    }
    edit
}

/// A track's place when it comes on the map: where its mixer pans it, on the
/// line where it keeps its level (`nth` of those at the same spot a little
/// nearer or further, so they can be told apart).
fn first_place(room: &MapRoom, pan: f32, nth: usize) -> Place {
    let step: f32 = [0.0, -0.06, 0.06, -0.12, 0.12, -0.18, 0.18][nth.min(6)];
    let across = st::across_for(&params(room, &Place::default()), pan);
    Place { across, depth: (0.3 + step).clamp(0.0, 1.0), ..Place::default() }
}

impl App {
    fn map_dots(&self) -> Vec<Dot> {
        self.project
            .tracks
            .iter()
            .enumerate()
            .filter_map(|(i, t)| {
                let place = t.place?;
                let level = self.meters.get(i).map(|m| m.0.max(m.1)).unwrap_or(0.0);
                Some(Dot { track: t.id, name: t.name.clone(), color: rgb(t.color), place, level })
            })
            .collect()
    }

    /// Puts track `i` on the map (where its pan had it; the pan then goes
    /// to the middle, the map pans it now), or takes it off (the pan back).
    pub(crate) fn set_on_map(&mut self, i: usize, on: bool) {
        let taken: Vec<(f32, f32)> = self.project.tracks.iter().filter_map(|t| t.place.map(|p| (p.across, p.depth))).collect();
        let room = self.project.map;
        let Some(t) = self.project.tracks.get_mut(i) else { return };
        if on && t.place.is_none() {
            let pan = t.mixer.pan;
            let nth = (0..7).find(|&k| {
                let p = first_place(&room, pan, k);
                !taken.iter().any(|&(a, d)| (a - p.across).abs() < 0.05 && (d - p.depth).abs() < 0.03)
            });
            t.place = Some(first_place(&room, pan, nth.unwrap_or(0)));
            t.mixer.pan = 0.0;
        } else if !on {
            if let Some(p) = t.place.take() {
                t.mixer.pan = (st::direction(&params(&room, &p)) * 100.0).round() / 100.0;
            }
        }
    }

    /// The tracks on the map side by side across the stage (drums in the
    /// middle), on the line where they keep their level, facing the listener.
    pub(crate) fn spread_map(&mut self) {
        let room = self.project.map;
        let base = params(&room, &Place::default());
        let ((_, _), listener) = st::places(&base);
        let on: Vec<usize> = (0..self.project.tracks.len()).filter(|&i| self.project.tracks[i].place.is_some()).collect();
        let drums: Vec<usize> = on.iter().copied().filter(|&i| self.project.tracks[i].instrument.as_ref().is_some_and(|x| x.is_drums())).collect();
        let others: Vec<usize> = on.iter().copied().filter(|i| !drums.contains(i)).collect();
        let n = others.len();
        for (j, &i) in others.iter().enumerate() {
            // Out from the middle, left and right in turn (the drums take the middle).
            let slot = if drums.is_empty() { j as f32 - (n as f32 - 1.0) / 2.0 } else { (j / 2 + 1) as f32 * if j % 2 == 0 { -1.0 } else { 1.0 } };
            let pan = (slot * 0.35).clamp(-0.9, 0.9);
            let mut pl = self.project.tracks[i].place.unwrap_or_default();
            pl.across = (st::across_for(&base, pan) * 100.0).round() / 100.0;
            pl.depth = 0.3;
            let ((sx, sy), _) = st::places(&params(&room, &pl));
            pl.facing = (st::facing_towards((sx, sy), listener) / 5.0).round() * 5.0;
            self.project.tracks[i].place = Some(pl);
        }
        for (j, &i) in drums.iter().enumerate() {
            let mut pl = self.project.tracks[i].place.unwrap_or_default();
            pl.across = 0.12 * j as f32;
            pl.depth = 0.24;
            pl.facing = 0.0;
            self.project.tracks[i].place = Some(pl);
        }
    }

    pub(crate) fn ui_map_window(&mut self, ctx: &Context) {
        if !self.map_window {
            return;
        }
        let (keep, _) = tool_window(ctx, "map", "Map", vec2(1080.0, 720.0), |ui| self.map_body(ui));
        if !keep {
            self.map_window = false;
        }
    }

    fn map_body(&mut self, ui: &mut Ui) {
        let mut room = self.project.map;
        // The room.
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("Room").strong());
            ui.add(egui::Slider::new(&mut room.room, 0.0..=1.0).show_value(false)).on_hover_text("How much of the room you hear: none at first (the tracks are only placed), its walls and its tail as you turn it up");
            ui.label(RichText::new(if room.room <= 0.0 { "off".to_string() } else { format!("{:.0} %", room.room * 100.0) }).small().color(TEXT_DIM));
            ui.separator();
            ui.label(RichText::new("Size").small().color(TEXT_DIM));
            ui.add(egui::Slider::new(&mut room.size, 4.0..=60.0).suffix(" m").max_decimals(0));
            ui.label(RichText::new("Walls").small().color(TEXT_DIM));
            ui.add(egui::Slider::new(&mut room.walls, 0.0..=1.0).show_value(false)).on_hover_text("Hard (stone) to soft (curtains)");
            ui.label(RichText::new("Reverb").small().color(TEXT_DIM));
            ui.add(egui::Slider::new(&mut room.reverb, 0.0..=1.0).show_value(false));
            ui.separator();
            ui.label(RichText::new("Falloff").small().color(TEXT_DIM));
            ui.add(egui::Slider::new(&mut room.falloff, 0.0..=1.0).show_value(false)).on_hover_text("How much quieter further away");
            ui.label(RichText::new("Air").small().color(TEXT_DIM));
            ui.add(egui::Slider::new(&mut room.air, 0.0..=1.0).show_value(false)).on_hover_text("How much duller further away");
            ui.checkbox(&mut room.delay, "Delay").on_hover_text("Further away is later, as in a real room (a few ms a metre)");
            ui.checkbox(&mut room.headphones, "Headphones").on_hover_text("For headphones: the far ear in the head's shadow");
            let p = params(&room, &Place::default());
            if room.room > 0.0 {
                ui.label(RichText::new(format!("tail {:.1} s", st::rt60(&p))).small().color(TEXT_DIM));
            }
        });
        if room != self.project.map {
            self.project.map = room;
        }
        ui.add_space(4.0);
        let full = ui.available_rect_before_wrap();
        let mixer_h = 168.0f32.min(full.height() * 0.4);
        let side_w = 230.0f32.min(full.width() * 0.3);
        let map_rect = Rect::from_min_max(full.min, pos2(full.right() - side_w - 8.0, full.bottom() - mixer_h - 8.0));
        let side = Rect::from_min_max(pos2(map_rect.right() + 8.0, full.top()), pos2(full.right(), map_rect.bottom()));
        let mixer = Rect::from_min_max(pos2(full.left(), map_rect.bottom() + 8.0), full.max);
        ui.allocate_rect(full, Sense::hover());

        // The map.
        let dots = self.map_dots();
        let selected = self.map_selected.filter(|s| dots.iter().any(|d| d.track == *s));
        let edit = map(ui, map_rect, Id::new("map_window"), &room, &dots, selected);
        self.apply_map_edit(&edit);

        // The chosen one, and which tracks are on the map.
        let mut side_ui = ui.new_child(UiBuilder::new().max_rect(side.shrink(2.0)));
        self.map_side(&mut side_ui);
        // The mixer.
        let mut mix_ui = ui.new_child(UiBuilder::new().max_rect(mixer));
        self.map_mixer(&mut mix_ui, mixer.height());
    }

    fn apply_map_edit(&mut self, edit: &MapEdit) {
        if let Some(t) = edit.select {
            self.map_selected = Some(t);
        }
        let ((_, _), listener) = st::places(&params(&self.project.map, &Place::default()));
        for t in self.project.tracks.iter_mut() {
            let Some(pl) = t.place.as_mut() else { continue };
            if let Some((id, a, d)) = edit.moved {
                if id == t.id {
                    pl.across = a;
                    pl.depth = d;
                }
            }
            if let Some((id, f)) = edit.turned {
                if id == t.id {
                    pl.facing = f;
                }
            }
            if edit.face_listener == Some(t.id) {
                let p = params(&self.project.map, pl);
                let ((sx, sy), _) = st::places(&p);
                pl.facing = (st::facing_towards((sx, sy), listener) / 5.0).round() * 5.0;
            }
        }
        if let Some(l) = edit.listener {
            self.project.map.listener = l;
        }
        if let Some(t) = edit.remove {
            if let Some(i) = self.project.tracks.iter().position(|x| x.id == t) {
                self.set_on_map(i, false);
            }
        }
    }

    /// The chosen track's place, and the tracks to tick onto the map.
    fn map_side(&mut self, ui: &mut Ui) {
        let sel = self.map_selected.and_then(|id| self.project.tracks.iter().position(|t| t.id == id && t.place.is_some()));
        egui::Frame::new().fill(PANEL2).corner_radius(4.0).inner_margin(8).show(ui, |ui| {
            ui.set_width(ui.available_width());
            match sel {
                Some(i) => {
                    let name = self.project.tracks[i].name.clone();
                    let color = rgb(self.project.tracks[i].color);
                    ui.label(RichText::new(name).strong().color(color));
                    let mut pl = self.project.tracks[i].place.unwrap_or_default();
                    let before = pl;
                    let p = params(&self.project.map, &pl);
                    ui.label(RichText::new(format!("{:.1} m from the listener", st::distance(&p))).small().color(TEXT_DIM));
                    egui::Grid::new("map_place").num_columns(2).spacing([6.0, 4.0]).show(ui, |ui| {
                        ui.label(RichText::new("Across").small());
                        ui.add(egui::Slider::new(&mut pl.across, -1.0..=1.0).show_value(false));
                        ui.end_row();
                        ui.label(RichText::new("Depth").small());
                        ui.add(egui::Slider::new(&mut pl.depth, 0.0..=1.0).show_value(false));
                        ui.end_row();
                        ui.label(RichText::new("Width").small()).on_hover_text("How much of its own stereo it keeps");
                        ui.add(egui::Slider::new(&mut pl.width, 0.0..=1.0).show_value(false));
                        ui.end_row();
                        ui.label(RichText::new("Facing").small()).on_hover_text("Which way it sounds: turned away it is quieter and duller");
                        ui.add(egui::Slider::new(&mut pl.facing, -180.0..=180.0).suffix("°").max_decimals(0));
                        ui.end_row();
                        ui.label(RichText::new("Beam").small()).on_hover_text("All round (0) to only where it faces");
                        ui.add(egui::Slider::new(&mut pl.beam, 0.0..=1.0).show_value(false));
                        ui.end_row();
                    });
                    ui.horizontal(|ui| {
                        if ui.small_button("Face the listener").clicked() {
                            let ((sx, sy), l) = st::places(&params(&self.project.map, &pl));
                            pl.facing = (st::facing_towards((sx, sy), l) / 5.0).round() * 5.0;
                        }
                        if ui.small_button("Off the map").clicked() {
                            self.set_on_map(i, false);
                        }
                    });
                    if pl != before {
                        if let Some(t) = self.project.tracks.get_mut(i) {
                            if t.place.is_some() {
                                t.place = Some(pl);
                            }
                        }
                    }
                }
                None => {
                    ui.label(RichText::new("Click a track on the map to set its place here").small().color(TEXT_DIM));
                }
            }
        });
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new("On the map").small().color(TEXT_DIM));
            if ui.small_button("All").clicked() {
                for i in 0..self.project.tracks.len() {
                    self.set_on_map(i, true);
                }
            }
            if ui.small_button("None").clicked() {
                for i in 0..self.project.tracks.len() {
                    self.set_on_map(i, false);
                }
            }
            if ui.small_button("Spread").on_hover_text("Lay the tracks on the map out side by side over the stage, like a band (drums in the middle), each facing the listener").clicked() {
                self.spread_map();
            }
        });
        egui::ScrollArea::vertical().id_salt("map_tracks").show(ui, |ui| {
            for i in 0..self.project.tracks.len() {
                let (name, color, mut on) = {
                    let t = &self.project.tracks[i];
                    (t.name.clone(), rgb(t.color), t.place.is_some())
                };
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(vec2(8.0, 14.0), Sense::hover());
                    rect_fill(ui.painter(), r, 2.0, color);
                    if ui.checkbox(&mut on, name).changed() {
                        self.set_on_map(i, on);
                        if on {
                            self.map_selected = Some(self.project.tracks[i].id);
                        }
                    }
                });
            }
        });
    }

    /// A strip per track: its name, on the map or not, its fader and meter,
    /// mute and solo.
    fn map_mixer(&mut self, ui: &mut Ui, h: f32) {
        egui::ScrollArea::horizontal().id_salt("map_mixer").show(ui, |ui| {
            ui.horizontal_top(|ui| {
                for i in 0..self.project.tracks.len() {
                    let level = self.meters.get(i).copied().unwrap_or((0.0, 0.0));
                    let selected = self.map_selected == Some(self.project.tracks[i].id);
                    let t = &mut self.project.tracks[i];
                    let color = rgb(t.color);
                    let mut put = None;
                    let mut pick = false;
                    egui::Frame::new().fill(if selected { PANEL3 } else { PANEL2 }).corner_radius(4.0).inner_margin(4).show(ui, |ui| {
                        ui.set_width(72.0);
                        ui.set_height(h - 12.0);
                        ui.vertical_centered(|ui| {
                            let name = ui.add(egui::Label::new(RichText::new(&t.name).small().color(color)).truncate().sense(Sense::click()));
                            if name.clicked() {
                                pick = true;
                            }
                            let mut on = t.place.is_some();
                            if ui.checkbox(&mut on, RichText::new("map").small()).on_hover_text("On the map").changed() {
                                put = Some(on);
                            }
                            ui.horizontal(|ui| {
                                ui.spacing_mut().slider_width = (h - 92.0).max(40.0);
                                ui.add(egui::Slider::new(&mut t.mixer.volume_db, -60.0..=6.0).vertical().show_value(false));
                                // The meter.
                                let (m, _) = ui.allocate_exact_size(vec2(8.0, (h - 92.0).max(40.0)), Sense::hover());
                                rect_fill(ui.painter(), m, 2.0, BG);
                                for (k, v) in [level.0, level.1].iter().enumerate() {
                                    let db = 20.0 * v.max(1e-6).log10();
                                    let frac = ((db + 60.0) / 66.0).clamp(0.0, 1.0);
                                    let col = Rect::from_min_max(pos2(m.left() + k as f32 * 4.0, m.bottom() - frac * m.height()), pos2(m.left() + k as f32 * 4.0 + 3.0, m.bottom()));
                                    rect_fill(ui.painter(), col, 1.0, if db > -3.0 { REC } else { PLAY });
                                }
                            });
                            ui.label(RichText::new(format!("{:+.1}", t.mixer.volume_db)).small().color(TEXT_DIM));
                            ui.horizontal(|ui| {
                                let m = ui.add(egui::Button::new(RichText::new("M").small().color(if t.mixer.mute { TEXT_DARK } else { TEXT })).fill(if t.mixer.mute { ACCENT } else { PANEL3 }));
                                if m.clicked() {
                                    t.mixer.mute = !t.mixer.mute;
                                }
                                let s = ui.add(egui::Button::new(RichText::new("S").small().color(if t.mixer.solo { TEXT_DARK } else { TEXT })).fill(if t.mixer.solo { SELECT } else { PANEL3 }));
                                if s.clicked() {
                                    t.mixer.solo = !t.mixer.solo;
                                }
                            });
                        });
                    });
                    let id = t.id;
                    if pick {
                        self.map_selected = Some(id);
                    }
                    if let Some(on) = put {
                        self.set_on_map(i, on);
                        if on {
                            self.map_selected = Some(id);
                        }
                    }
                }
            });
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Event, Modifiers, PointerButton, RawInput};

    /// Frames of pointer moves (and presses) over a map of `dots`; the edits.
    fn drive(dots: &[Dot], selected: Option<u64>, frames: &[(Pos2, Option<bool>)]) -> Vec<MapEdit> {
        let ctx = egui::Context::default();
        let room = MapRoom::default();
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
                    edits.push(map(ui, Rect::from_min_size(pos2(0.0, 0.0), vec2(600.0, 600.0)), Id::new("t"), &room, dots, selected));
                });
            });
        }
        edits
    }

    /// Where a place is on the 600 x 600 test map.
    fn screen_of(pl: &Place) -> Pos2 {
        let room = MapRoom::default();
        let p = params(&room, pl);
        let (w, d, _) = st::room(&p);
        let scale = ((600.0 - 44.0) / w).min((600.0 - 40.0) / d);
        let top = 300.0 - d * scale / 2.0;
        let ((sx, sy), _) = st::places(&p);
        pos2(300.0 + sx * scale, top + sy * scale)
    }

    #[test]
    fn dragging_a_track_moves_it_from_where_it_was_taken() {
        let dot = Dot { track: 7, name: "Bass".into(), color: Color32::RED, place: Place::default(), level: 0.0 };
        // Taken by its tag (above the dot): it moves by as much as the mouse, no jump.
        let at = screen_of(&dot.place) + vec2(0.0, -19.0);
        let to = at + vec2(120.0, 60.0);
        let edits = drive(&[dot.clone()], None, &[(at, Some(true)), (at + vec2(10.0, 5.0), None), (to, None), (to, Some(false))]);
        assert!(edits.iter().any(|e| e.select == Some(7)));
        let moved = edits.iter().filter_map(|e| e.moved).last().expect("moved");
        assert_eq!(moved.0, 7);
        assert!(moved.1 > 0.2 && moved.2 > 0.35, "{moved:?}");
    }

    #[test]
    fn the_handle_turns_the_selected_track() {
        let dot = Dot { track: 3, name: "Keys".into(), color: Color32::BLUE, place: Place::default(), level: 0.0 };
        let q = screen_of(&dot.place);
        // Facing 0: the handle straight below it; dragged to its right side: facing 90.
        let h = q + vec2(0.0, HANDLE);
        let to = q + vec2(HANDLE, 0.0);
        let edits = drive(&[dot], Some(3), &[(h, Some(true)), (h + vec2(6.0, -4.0), None), (to, None), (to, Some(false))]);
        let turned = edits.iter().filter_map(|e| e.turned).last().expect("turned");
        assert_eq!(turned, (3, 90.0));
    }

    #[test]
    fn a_track_comes_on_the_map_where_its_pan_had_it() {
        let room = MapRoom::default();
        assert_eq!(first_place(&room, 0.0, 0), Place::default());
        for pan in [-0.8f32, -0.3, 0.5] {
            let p = first_place(&room, pan, 0);
            let back = st::direction(&params(&room, &p));
            assert!((back - pan).abs() < 0.02, "{pan} {back}");
        }
        assert!(first_place(&room, 0.0, 1).depth < 0.3 && first_place(&room, 0.0, 2).depth > 0.3);
    }
}

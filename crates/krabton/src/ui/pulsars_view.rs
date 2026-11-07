//! The Pulsars view: a field to place pulsars, orbs, gates, chances and
//! reflectors on, and to join them with lines; the pulses flying down the lines
//! as the song plays, the nodes lighting up as they are struck. Drag a node's
//! middle to move it, its edge to another node to join them, the field to look
//! round it, the wheel to zoom. What a node does is set on the right. See
//! `krabton_engine::pulsars`.

use crate::app::App;
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, Id, Pos2, Rect, RichText, Sense, Shape, Stroke, Ui, Vec2};
use krabton_engine::pulsars::{self as kp, PKind, PNode, RATES};
use krabton_engine::{note_name, TrackKind};
use std::collections::HashMap;
use std::time::Instant;

/// A node's size on the field.
const R: f32 = 18.0;
const SIDE_W: f32 = 250.0;

#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Tool {
    Select,
    Pulsar,
    Orb,
    Gate,
    Chance,
    Reflector,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Sel {
    Node(u64),
    Link(usize),
}

#[derive(Clone, Copy)]
enum Drag {
    Move(u64, Vec2),
    Link(u64),
    Pan,
}

pub(crate) struct PulsarsUi {
    pan: Vec2,
    zoom: f32,
    pub(crate) tool: Tool,
    pub(crate) selected: Option<Sel>,
    drag: Option<Drag>,
    flashes: HashMap<u64, Instant>,
    menu: Option<Sel>,
}

impl Default for PulsarsUi {
    fn default() -> Self {
        Self { pan: Vec2::ZERO, zoom: 1.0, tool: Tool::Select, selected: None, drag: None, flashes: HashMap::new(), menu: None }
    }
}

/// Which part of a node a press at `d` (from its middle) takes: the middle
/// moves it, the edge draws a line from it.
pub(crate) fn takes_middle(d: f32) -> bool {
    d < R * 0.6
}

/// How far a point is from a line between `a` and `b`.
pub(crate) fn to_segment(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ab = b - a;
    let t = ((p - a).dot(ab) / ab.length_sq().max(1e-6)).clamp(0.0, 1.0);
    (a + ab * t - p).length()
}

/// The notes an orb can play: steps of the key from C3.
const DEGREES: std::ops::RangeInclusive<i32> = -7..=14;

impl App {
    fn pulsar_tracks(&self) -> Vec<(u64, String)> {
        self.project.tracks.iter().filter(|t| t.kind == TrackKind::Midi && !t.is_group).map(|t| (t.id, t.name.clone())).collect()
    }

    /// The track a new orb plays: the selected one if it takes notes, else the first that does.
    fn orb_track(&self) -> Option<u64> {
        let sel = match self.sel_track {
            crate::app::TrackSel::Track(i) => self.project.tracks.get(i).filter(|t| t.kind == TrackKind::Midi).map(|t| t.id),
            _ => None,
        };
        sel.or_else(|| self.pulsar_tracks().first().map(|t| t.0))
    }

    fn new_node(&mut self, kind: PKind, at: Pos2) -> u64 {
        let id = self.project.alloc_id();
        self.project.pulsars.nodes.push(PNode { id, x: at.x, y: at.y, kind });
        id
    }

    fn orb_kind(&self) -> PKind {
        let orbs = self.project.pulsars.nodes.iter().filter(|n| matches!(n.kind, PKind::Orb { .. })).count();
        PKind::Orb { track: self.orb_track(), degree: [0, 2, 4, 7, 9, 4][orbs % 6], octave: 0, velocity: 0.8, length: 0.25 }
    }

    pub(crate) fn pulsar_example_now(&mut self) {
        self.pulsar_example();
    }

    /// A field to start from: a pulsar and orbs round it at different distances.
    fn pulsar_example(&mut self) {
        let p = self.new_node(PKind::Pulsar { rate: 6, on: true }, pos2(0.0, 0.0));
        let mut last = p;
        for (k, (dist, deg)) in [(160.0, 0), (210.0, 4), (130.0, 2), (250.0, 7), (190.0, 9)].into_iter().enumerate() {
            let a = k as f32 / 5.0 * std::f32::consts::TAU;
            let mut kind = self.orb_kind();
            if let PKind::Orb { degree, .. } = &mut kind {
                *degree = deg;
            }
            let o = self.new_node(kind, pos2(a.cos() * dist, a.sin() * dist));
            self.project.pulsars.link(p, o);
            if k % 2 == 1 {
                self.project.pulsars.link(last, o);
            }
            last = o;
        }
    }

    pub(crate) fn ui_pulsars(&mut self, ui: &mut Ui) {
        let full = ui.available_rect_before_wrap();
        ui.allocate_rect(full, Sense::hover());
        let bar = Rect::from_min_size(full.min, vec2(full.width(), 34.0));
        let side = Rect::from_min_max(pos2(full.right() - SIDE_W, bar.bottom()), full.max);
        let field = Rect::from_min_max(pos2(full.left(), bar.bottom()), pos2(side.left(), full.bottom()));
        rect_fill(ui.painter(), bar, 0.0, PANEL2);
        rect_fill(ui.painter(), side, 0.0, PANEL);
        let mut bui = ui.new_child(egui::UiBuilder::new().max_rect(bar.shrink2(vec2(8.0, 4.0))).layout(egui::Layout::left_to_right(egui::Align::Center)));
        self.pulsars_bar(&mut bui);
        self.pulsars_field(ui, field);
        let mut sui = ui.new_child(egui::UiBuilder::new().max_rect(side.shrink(10.0)).layout(egui::Layout::top_down(egui::Align::Min)));
        self.pulsars_side(&mut sui);
        if self.is_playing() || self.pulsars_ui.drag.is_some() || !self.pulsars_ui.flashes.is_empty() {
            ui.ctx().request_repaint();
        }
    }

    fn pulsars_bar(&mut self, ui: &mut Ui) {
        ui.label(RichText::new("PULSARS").strong().color(accent_now()));
        ui.add_space(10.0);
        let tools = [
            (Tool::Select, "Select", "Move nodes, join them, look round the field"),
            (Tool::Pulsar, "+ Pulsar", "Click the field: a pulsar, which fires a pulse down its lines on the beat"),
            (Tool::Orb, "+ Orb", "Click the field: an orb, which plays its note on its track when a pulse arrives"),
            (Tool::Gate, "+ Gate", "Lets one pulse in so many through"),
            (Tool::Chance, "+ Chance", "Lets a pulse through so often"),
            (Tool::Reflector, "+ Reflector", "Sends a pulse back the way it came"),
        ];
        for (tool, name, tip) in tools {
            if ui.selectable_label(self.pulsars_ui.tool == tool, name).on_hover_text(tip).clicked() {
                self.pulsars_ui.tool = tool;
            }
        }
        ui.separator();
        ui.label(RichText::new("Speed").small().color(TEXT_DIM));
        ui.add(egui::DragValue::new(&mut self.project.pulsars.speed).range(20.0..=1000.0).speed(1.0).suffix(" px a beat"))
            .on_hover_text("How far a pulse travels in a beat: a line twice as long takes twice as long");
        if ui.button("Example").on_hover_text("A pulsar with orbs round it to start from").clicked() {
            self.pulsar_example();
        }
        if !self.project.pulsars.is_empty() && ui.button("Clear").clicked() {
            self.project.pulsars = Default::default();
            self.pulsars_ui.selected = None;
        }
    }

    fn pulsars_field(&mut self, ui: &mut Ui, field: Rect) {
        let p = ui.painter().with_clip_rect(field);
        p.rect_filled(field, 0.0, Color32::from_rgb(0x0b, 0x0d, 0x16));
        // A few stars.
        for k in 0..90u32 {
            let h = |x: u32| (x.wrapping_mul(2654435761) >> 8) as f32 / 16_777_216.0;
            let at = pos2(field.left() + h(k * 3 + 1) * field.width(), field.top() + h(k * 7 + 5) * field.height());
            p.circle_filled(at, 0.6 + h(k * 11) * 1.2, Color32::from_white_alpha((40.0 + h(k * 13) * 80.0) as u8));
        }
        let ui_state = &self.pulsars_ui;
        let (pan, zoom) = (ui_state.pan, ui_state.zoom);
        let c = field.center();
        let to_screen = move |w: Pos2| c + (w.to_vec2() - pan) * zoom;
        let to_world = move |s: Pos2| pos2(((s - c) / zoom + pan).x, ((s - c) / zoom + pan).y);
        let set = self.project.pulsars.clone();
        let place = |id: u64| set.node(id).map(|n| pos2(n.x, n.y));

        // What struck what, since last frame.
        let now = Instant::now();
        let status = self.status();
        for k in status.pulsars.take_struck() {
            if let Some(n) = set.nodes.get(k) {
                self.pulsars_ui.flashes.insert(n.id, now);
            }
        }
        self.pulsars_ui.flashes.retain(|_, t| t.elapsed().as_secs_f32() < 0.35);

        let resp = ui.interact(field, Id::new("pulsars_field"), Sense::click_and_drag());
        let pointer = resp.hover_pos().or(resp.interact_pointer_pos());
        let world_at = pointer.map(to_world);
        let node_at = |w: Pos2| set.nodes.iter().rev().map(|n| (n.id, (pos2(n.x, n.y) - w).length())).filter(|(_, d)| *d < R * 1.35).min_by(|a, b| a.1.total_cmp(&b.1));
        let link_at = |w: Pos2| {
            set.links.iter().enumerate().filter_map(|(k, l)| Some((k, to_segment(w, place(l.a)?, place(l.b)?)))).filter(|(_, d)| *d < 7.0 / zoom).min_by(|a, b| a.1.total_cmp(&b.1)).map(|(k, _)| k)
        };

        // The wheel zooms about the pointer.
        if let Some(sp) = pointer.filter(|_| resp.hovered()) {
            let wheel = ui.input(|i| i.smooth_scroll_delta.y);
            if wheel != 0.0 {
                let before = to_world(sp);
                let z = (zoom * (wheel * 0.002).exp()).clamp(0.25, 4.0);
                self.pulsars_ui.zoom = z;
                self.pulsars_ui.pan = before.to_vec2() - (sp - c) / z;
            }
        }
        // The hand.
        if resp.drag_started() {
            if let Some(w) = world_at {
                self.pulsars_ui.drag = Some(match node_at(w) {
                    Some((id, d)) if takes_middle(d) => Drag::Move(id, w - place(id).unwrap_or(w)),
                    Some((id, _)) => Drag::Link(id),
                    None => Drag::Pan,
                });
            }
        }
        if resp.dragged() {
            match (self.pulsars_ui.drag, world_at) {
                (Some(Drag::Move(id, off)), Some(w)) => {
                    if let Some(n) = self.project.pulsars.nodes.iter_mut().find(|n| n.id == id) {
                        n.x = (w - off).x;
                        n.y = (w - off).y;
                    }
                }
                (Some(Drag::Pan), _) => self.pulsars_ui.pan -= resp.drag_delta() / zoom,
                _ => {}
            }
        }
        if resp.drag_stopped() {
            if let (Some(Drag::Link(from)), Some(w)) = (self.pulsars_ui.drag, world_at) {
                if let Some((to, _)) = node_at(w) {
                    self.project.pulsars.link(from, to);
                }
            }
            self.pulsars_ui.drag = None;
        }
        if resp.clicked() {
            if let Some(w) = world_at {
                let tool = self.pulsars_ui.tool;
                self.pulsars_ui.selected = match (node_at(w), link_at(w)) {
                    (Some((id, _)), _) => Some(Sel::Node(id)),
                    (None, Some(k)) if tool == Tool::Select => Some(Sel::Link(k)),
                    _ => {
                        let kind = match tool {
                            Tool::Select => None,
                            Tool::Pulsar => Some(PKind::Pulsar { rate: 3, on: true }),
                            Tool::Orb => Some(self.orb_kind()),
                            Tool::Gate => Some(PKind::Gate { every: 2 }),
                            Tool::Chance => Some(PKind::Chance { chance: 0.5 }),
                            Tool::Reflector => Some(PKind::Reflector),
                        };
                        kind.map(|k| Sel::Node(self.new_node(k, w)))
                    }
                };
            }
        }
        if resp.secondary_clicked() {
            self.pulsars_ui.menu = world_at.and_then(|w| node_at(w).map(|n| Sel::Node(n.0)).or_else(|| link_at(w).map(Sel::Link)));
        }
        let menu = self.pulsars_ui.menu;
        resp.context_menu(|ui| match menu {
            Some(Sel::Node(id)) => {
                if ui.button("Delete").clicked() {
                    self.project.pulsars.remove(id);
                    self.pulsars_ui.selected = None;
                    ui.close();
                }
            }
            Some(Sel::Link(k)) => {
                if ui.button("One way / both ways").clicked() {
                    if let Some(l) = self.project.pulsars.links.get_mut(k) {
                        l.one_way = !l.one_way;
                    }
                    ui.close();
                }
                if ui.button("Delete line").clicked() {
                    if k < self.project.pulsars.links.len() {
                        self.project.pulsars.links.remove(k);
                    }
                    self.pulsars_ui.selected = None;
                    ui.close();
                }
            }
            None => {
                ui.label(RichText::new("Right-click a node or a line").small().color(TEXT_DIM));
            }
        });
        // Delete what is picked.
        if resp.hovered() && ui.input(|i| i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace)) {
            match self.pulsars_ui.selected.take() {
                Some(Sel::Node(id)) => self.project.pulsars.remove(id),
                Some(Sel::Link(k)) if k < self.project.pulsars.links.len() => {
                    self.project.pulsars.links.remove(k);
                }
                _ => {}
            }
        }

        // The lines.
        let set = self.project.pulsars.clone();
        let place = |id: u64| set.node(id).map(|n| pos2(n.x, n.y));
        for (k, l) in set.links.iter().enumerate() {
            let (Some(a), Some(b)) = (place(l.a), place(l.b)) else { continue };
            let (sa, sb) = (to_screen(a), to_screen(b));
            let picked = self.pulsars_ui.selected == Some(Sel::Link(k));
            let color = if picked { accent_now() } else { Color32::from_rgb(0x4a, 0x55, 0x78) };
            p.line_segment([sa, sb], Stroke::new(if picked { 3.0 } else { 2.0_f32 }, color));
            if l.one_way {
                let mid = sa + (sb - sa) * 0.5;
                let d = (sb - sa).normalized();
                let n = vec2(-d.y, d.x);
                p.add(Shape::convex_polygon(vec![mid + d * 7.0, mid - d * 5.0 + n * 5.0, mid - d * 5.0 - n * 5.0], color, Stroke::NONE));
            }
        }
        // The pulses under way.
        let order = kp::line_order(&set);
        for (line, from_a, along) in status.pulsars.read() {
            let Some(l) = order.get(line).and_then(|&k| set.links.get(k)) else { continue };
            let (Some(a), Some(b)) = (place(l.a), place(l.b)) else { continue };
            let (from, to) = if from_a { (a, b) } else { (b, a) };
            let at = to_screen(from + (to - from) * along);
            p.circle_filled(at, 9.0 * zoom.sqrt(), Color32::from_rgba_unmultiplied(0xff, 0xe0, 0x80, 40));
            p.circle_filled(at, 4.0 * zoom.sqrt(), Color32::from_rgb(0xff, 0xf2, 0xc0));
        }
        // A line being drawn.
        if let (Some(Drag::Link(from)), Some(sp)) = (self.pulsars_ui.drag, pointer) {
            if let Some(a) = place(from) {
                p.line_segment([to_screen(a), sp], Stroke::new(2.0_f32, accent_now()));
            }
        }
        // The nodes.
        let flashes = self.pulsars_ui.flashes.clone();
        let flash = |id: u64| flashes.get(&id).map_or(0.0, |t| 1.0 - t.elapsed().as_secs_f32() / 0.35);
        let tracks = self.project.tracks.clone();
        for n in &set.nodes {
            let at = to_screen(pos2(n.x, n.y));
            let r = R * zoom;
            let lit = flash(n.id);
            let picked = self.pulsars_ui.selected == Some(Sel::Node(n.id));
            match n.kind {
                PKind::Pulsar { rate, on } => {
                    let base = if on { Color32::from_rgb(0xff, 0xa7, 0x33) } else { Color32::from_gray(0x60) };
                    for k in 0..8 {
                        let a = k as f32 / 8.0 * std::f32::consts::TAU;
                        let d = vec2(a.cos(), a.sin());
                        p.line_segment([at + d * r * 0.9, at + d * r * (1.35 + 0.3 * lit)], Stroke::new(2.0_f32, base));
                    }
                    p.circle_filled(at, r * 0.75, mix(base, Color32::WHITE, lit * 0.7));
                    text(&p, at + vec2(0.0, r * 1.7), Align2::CENTER_CENTER, RATES[(rate as usize).min(RATES.len() - 1)].0, small(), TEXT_DIM);
                }
                PKind::Orb { track, degree, octave, .. } => {
                    let color = track.and_then(|id| tracks.iter().find(|t| t.id == id)).map_or(Color32::from_gray(0x70), |t| rgb(t.color));
                    let fill = mix(color, Color32::WHITE, lit * 0.6);
                    p.circle_filled(at, r * (0.85 + 0.2 * lit), fill);
                    let name = note_name(kp::orb_pitch(&self.project.scale, degree, octave));
                    text(&p, at, Align2::CENTER_CENTER, &name, small(), text_on(fill));
                }
                PKind::Gate { every } => {
                    let box_r = Rect::from_center_size(at, vec2(r * 1.6, r * 1.6));
                    p.rect_filled(box_r, 4.0, mix(Color32::from_rgb(0x3a, 0x44, 0x60), Color32::WHITE, lit * 0.5));
                    text(&p, at, Align2::CENTER_CENTER, &format!("÷{every}"), small(), TEXT);
                }
                PKind::Chance { chance } => {
                    let d = r * 0.95;
                    p.add(Shape::convex_polygon(vec![at + vec2(0.0, -d), at + vec2(d, 0.0), at + vec2(0.0, d), at + vec2(-d, 0.0)], mix(Color32::from_rgb(0x5a, 0x3a, 0x70), Color32::WHITE, lit * 0.5), Stroke::NONE));
                    text(&p, at, Align2::CENTER_CENTER, &format!("{:.0}%", chance * 100.0), small(), TEXT);
                }
                PKind::Reflector => {
                    p.circle_stroke(at, r * 0.8, Stroke::new(4.0_f32, mix(Color32::from_rgb(0x6c, 0xd0, 0xff), Color32::WHITE, lit * 0.6)));
                    text(&p, at, Align2::CENTER_CENTER, "Ref", small(), TEXT);
                }
            }
            if picked {
                p.circle_stroke(at, r * 1.25, Stroke::new(1.5_f32, Color32::WHITE));
            }
        }
        // Where the hand is: the middle moves, the edge joins.
        if let (Some(w), None) = (world_at, self.pulsars_ui.drag) {
            if let Some((id, d)) = node_at(w) {
                if let Some(at) = place(id).map(to_screen) {
                    if takes_middle(d) {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
                    } else {
                        p.circle_stroke(at, R * zoom * 1.3, Stroke::new(1.0_f32, Color32::from_white_alpha(120)));
                        ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
                    }
                }
            }
        }
        if set.is_empty() {
            text(&p, field.center() - vec2(0.0, 12.0), Align2::CENTER_CENTER, "Pick + Pulsar or + Orb above and click the field to place it.", body(), TEXT);
            text(&p, field.center() + vec2(0.0, 12.0), Align2::CENTER_CENTER, "Drag from the edge of one node to another to join them, or start from the Example.", small(), TEXT_DIM);
        }
    }

    fn pulsars_side(&mut self, ui: &mut Ui) {
        let scale = self.project.scale;
        let tracks = self.pulsar_tracks();
        match self.pulsars_ui.selected {
            Some(Sel::Node(id)) => {
                let Some(n) = self.project.pulsars.nodes.iter_mut().find(|n| n.id == id) else {
                    self.pulsars_ui.selected = None;
                    return;
                };
                ui.label(RichText::new(n.kind.name()).strong().size(16.0).color(TEXT));
                ui.add_space(6.0);
                match &mut n.kind {
                    PKind::Pulsar { rate, on } => {
                        ui.checkbox(on, "On").on_hover_text("Fires its pulses while the song plays");
                        ui.label(RichText::new("Fires every").small().color(TEXT_DIM));
                        egui::ComboBox::from_id_salt(("pulsar_rate", id)).selected_text(RATES[(*rate as usize).min(RATES.len() - 1)].0).show_ui(ui, |ui| {
                            for (k, r) in RATES.iter().enumerate() {
                                ui.selectable_value(rate, k as u8, r.0);
                            }
                        });
                    }
                    PKind::Orb { track, degree, octave, velocity, length } => {
                        ui.label(RichText::new("Plays on").small().color(TEXT_DIM));
                        let name = track.and_then(|t| tracks.iter().find(|x| x.0 == t)).map_or("Nothing".to_string(), |t| t.1.clone());
                        egui::ComboBox::from_id_salt(("orb_track", id)).width(200.0).selected_text(name).show_ui(ui, |ui| {
                            for (t, name) in &tracks {
                                ui.selectable_value(track, Some(*t), name);
                            }
                        });
                        ui.label(RichText::new("Note").small().color(TEXT_DIM));
                        let now = note_name(kp::orb_pitch(&scale, *degree, *octave));
                        let o = *octave;
                        egui::ComboBox::from_id_salt(("orb_note", id)).width(80.0).height(320.0).selected_text(now).show_ui(ui, |ui| {
                            for d in DEGREES.rev() {
                                ui.selectable_value(degree, d, note_name(kp::orb_pitch(&scale, d, o)));
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Octave").small().color(TEXT_DIM));
                            if ui.small_button("-").clicked() {
                                *octave = (*octave - 1).max(-3);
                            }
                            ui.label(format!("{:+}", *octave));
                            if ui.small_button("+").clicked() {
                                *octave = (*octave + 1).min(3);
                            }
                        });
                        ui.label(RichText::new("Velocity").small().color(TEXT_DIM));
                        ui.add(egui::Slider::new(velocity, 0.05..=1.0).show_value(false));
                        ui.label(RichText::new("Length").small().color(TEXT_DIM));
                        let len_name = RATES.iter().min_by(|a, b| (a.1 - *length as f64).abs().total_cmp(&(b.1 - *length as f64).abs())).map_or("", |r| r.0);
                        egui::ComboBox::from_id_salt(("orb_len", id)).selected_text(len_name).show_ui(ui, |ui| {
                            for r in RATES.iter() {
                                if ui.selectable_label((*length as f64 - r.1).abs() < 1e-6, r.0).clicked() {
                                    *length = r.1 as f32;
                                }
                            }
                        });
                    }
                    PKind::Gate { every } => {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Lets through one in").small().color(TEXT_DIM));
                            if ui.small_button("-").clicked() {
                                *every = (*every - 1).max(1);
                            }
                            ui.label(format!("{every}"));
                            if ui.small_button("+").clicked() {
                                *every = (*every + 1).min(16);
                            }
                        });
                    }
                    PKind::Chance { chance } => {
                        ui.label(RichText::new("Lets a pulse through").small().color(TEXT_DIM));
                        let mut pct = *chance * 100.0;
                        if ui.add(egui::Slider::new(&mut pct, 0.0..=100.0).suffix(" %")).changed() {
                            *chance = pct / 100.0;
                        }
                    }
                    PKind::Reflector => {
                        ui.label(RichText::new("Sends a pulse back the way it came, and what it passes plays again").small().color(TEXT_DIM));
                    }
                }
                ui.add_space(10.0);
                if ui.button("Delete").clicked() {
                    self.project.pulsars.remove(id);
                    self.pulsars_ui.selected = None;
                }
            }
            Some(Sel::Link(k)) => {
                let speed = self.project.pulsars.speed.max(1.0);
                let Some(len) = self.project.pulsars.links.get(k).map(|l| self.project.pulsars.length(l)) else {
                    self.pulsars_ui.selected = None;
                    return;
                };
                ui.label(RichText::new("Line").strong().size(16.0).color(TEXT));
                ui.label(RichText::new(format!("A pulse takes {:.2} beats down it", len / speed)).small().color(TEXT_DIM));
                if let Some(l) = self.project.pulsars.links.get_mut(k) {
                    ui.checkbox(&mut l.one_way, "One way").on_hover_text("Pulses go one way down it only (the arrow)");
                    if ui.button("Turn round").clicked() {
                        std::mem::swap(&mut l.a, &mut l.b);
                    }
                }
                if ui.button("Delete").clicked() {
                    self.project.pulsars.links.remove(k);
                    self.pulsars_ui.selected = None;
                }
            }
            None => {
                ui.label(RichText::new("Pulsars").strong().size(16.0).color(TEXT));
                ui.add_space(4.0);
                for line in [
                    "A pulsar fires a pulse down each of its lines on the beat.",
                    "A pulse takes as long as its line is long: where things are is the rhythm.",
                    "An orb plays its note on its track, and the pulse goes on down its other lines.",
                    "Drag a node's middle to move it, its edge to another to join them. Drag the field to look round, the wheel zooms.",
                    "Click a node or a line to set it; right-click for more.",
                ] {
                    ui.label(RichText::new(line).small().color(TEXT_DIM));
                    ui.add_space(3.0);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_middle_moves_and_the_edge_joins() {
        assert!(takes_middle(0.0) && takes_middle(R * 0.5));
        assert!(!takes_middle(R * 0.9));
    }

    #[test]
    fn a_line_is_found_near_it() {
        let (a, b) = (pos2(0.0, 0.0), pos2(100.0, 0.0));
        assert!((to_segment(pos2(50.0, 5.0), a, b) - 5.0).abs() < 1e-4);
        assert!((to_segment(pos2(-10.0, 0.0), a, b) - 10.0).abs() < 1e-4, "past its end: to the end");
    }
}

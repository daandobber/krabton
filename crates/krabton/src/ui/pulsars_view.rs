//! The Pulsars view: a field in deep space to place pulsars, orbs and the
//! things between them on (gates, chances, reflectors, delays, routers,
//! shifts), and to join them with lines; the pulses streaking down the lines
//! as the song plays, in their pulsar's colour, the nodes flaring and sending
//! out a ring as they are struck. Drag a node's middle to move it, its edge to
//! another node to join them (or to empty space for a new node there), the
//! field to look round it, the wheel to zoom. What a node does is set on the
//! right. See `krabton_engine::pulsars`.

use crate::app::App;
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, Id, Painter, Pos2, Rect, RichText, Sense, Shape, Stroke, Ui, Vec2};
use krabton_engine::pulsars::{self as kp, PKind, PNode, CHORDS, MODES, RATES};
use krabton_engine::{note_name, Command, Instrument, TrackKind};
use std::collections::HashMap;
use std::f32::consts::TAU;
use std::time::Instant;

/// A node's size on the field.
const R: f32 = 18.0;
const SIDE_W: f32 = 250.0;
/// How long a struck node flares, and its ring goes out.
const FLARE: f32 = 0.7;

#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Tool {
    Select,
    Pulsar,
    Orb,
    Gate,
    Chance,
    Reflector,
    Delay,
    Router,
    Shift,
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
    /// Whether to fit the whole field into view next time it is drawn.
    fit: bool,
}

impl Default for PulsarsUi {
    fn default() -> Self {
        Self { pan: Vec2::ZERO, zoom: 1.0, tool: Tool::Select, selected: None, drag: None, flashes: HashMap::new(), menu: None, fit: false }
    }
}

/// The look (pan, zoom) that shows nodes at these places in a field this big.
pub(crate) fn fit_view(places: &[Pos2], size: Vec2) -> (Vec2, f32) {
    if places.is_empty() {
        return (Vec2::ZERO, 1.0);
    }
    let r = Rect::from_points(places).expand(R * 3.0);
    let zoom = (size.x / r.width()).min(size.y / r.height()).clamp(0.25, 1.5);
    (r.center().to_vec2(), zoom)
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

/// A colour round the wheel.
pub(crate) fn hue(h: f32, s: f32, v: f32) -> Color32 {
    egui::ecolor::Hsva::new(h.rem_euclid(1.0), s, v, 1.0).into()
}

/// A note's colour: its pitch class round the wheel.
pub(crate) fn note_hue(pitch: u8) -> f32 {
    (pitch % 12) as f32 / 12.0
}

/// A pulsar's colour (by its place): one after another round the wheel.
pub(crate) fn pulsar_hue(place: usize) -> f32 {
    0.08 + place as f32 * 0.618_034
}

/// Light that adds to what is under it.
fn light(c: Color32, k: f32) -> Color32 {
    let k = k.clamp(0.0, 1.0);
    Color32::from_rgba_premultiplied((c.r() as f32 * k) as u8, (c.g() as f32 * k) as u8, (c.b() as f32 * k) as u8, 0)
}

/// Light round a point, fading smoothly to nothing at `r`.
fn radial(p: &Painter, at: Pos2, r: f32, c: Color32) {
    if r < 0.5 || c == Color32::TRANSPARENT {
        return;
    }
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(at, c);
    let n = 40u32;
    for k in 0..n {
        let a = k as f32 / n as f32 * TAU;
        // A soft middle, a long fade.
        mesh.colored_vertex(at + vec2(a.cos(), a.sin()) * r * 0.45, light(c, 0.35));
        mesh.colored_vertex(at + vec2(a.cos(), a.sin()) * r, Color32::TRANSPARENT);
    }
    for k in 0..n {
        let (m0, o0, m1, o1) = (1 + 2 * k, 2 + 2 * k, 1 + 2 * ((k + 1) % n), 2 + 2 * ((k + 1) % n));
        mesh.add_triangle(0, m0, m1);
        mesh.add_triangle(m0, o0, o1);
        mesh.add_triangle(m0, o1, m1);
    }
    p.add(Shape::mesh(mesh));
}

/// A soft glow round a point.
fn glow(p: &Painter, at: Pos2, r: f32, c: Color32, k: f32) {
    radial(p, at, r * 2.6, light(c, k * 0.5));
}

/// A number 0..1 that looks random, the same for the same `x`.
fn hash(x: u32) -> f32 {
    let mut x = x;
    x ^= x >> 16;
    x = x.wrapping_mul(0x7feb_352d);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846c_a68b);
    x ^= x >> 16;
    (x >> 8) as f32 / 16_777_216.0
}

fn tool_kind(tool: Tool) -> Option<PKind> {
    match tool {
        Tool::Select | Tool::Orb => None,
        Tool::Pulsar => Some(PKind::pulsar()),
        Tool::Gate => Some(PKind::Gate { every: 2 }),
        Tool::Chance => Some(PKind::Chance { chance: 0.5 }),
        Tool::Reflector => Some(PKind::Reflector),
        Tool::Delay => Some(PKind::Delay { wait: 1 }),
        Tool::Router => Some(PKind::Router),
        Tool::Shift => Some(PKind::Shift { steps: 2 }),
    }
}

impl App {
    fn pulsar_tracks(&self) -> Vec<(u64, String)> {
        self.project.tracks.iter().filter(|t| t.kind == TrackKind::Midi && !t.is_group).map(|t| (t.id, t.name.clone())).collect()
    }

    /// The pads of a track's kit: (note, name); none if it plays no kit.
    fn pulsar_pads(&self, track: Option<u64>) -> Vec<(u8, String)> {
        let Some(inst) = track.and_then(|id| self.project.tracks.iter().find(|t| t.id == id)).and_then(|t| t.instrument.as_ref()) else { return Vec::new() };
        if let Instrument::Drums { pads, .. } = inst {
            return pads.iter().map(|p| (p.note, p.name.clone())).collect();
        }
        let kit = inst.drum_notes();
        (0..128u8).filter(|&n| kit & (1u128 << n) != 0).map(|n| (n, note_name(n))).collect()
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
        let track = self.orb_track();
        let mut kind = PKind::orb(track, [0, 2, 4, 7, 9, 4][orbs % 6]);
        // On a kit, a pad.
        let pads = self.pulsar_pads(track);
        if let (PKind::Orb { note, .. }, false) = (&mut kind, pads.is_empty()) {
            *note = Some(pads[orbs % pads.len()].0);
        }
        kind
    }

    fn kind_for(&self, tool: Tool) -> Option<PKind> {
        if tool == Tool::Orb {
            Some(self.orb_kind())
        } else {
            tool_kind(tool)
        }
    }

    pub(crate) fn pulsar_example_now(&mut self) {
        self.pulsar_example();
    }

    /// Places an example, and shows it all.
    fn pulsar_example(&mut self) {
        self.pulsar_example_nodes();
        self.pulsars_ui.fit = true;
    }

    /// A field to start from: a pulsar with orbs round it at different
    /// distances, a reflector out past them; a second pulsar firing now and
    /// then into a delay, a shift and chords.
    fn pulsar_example_nodes(&mut self) {
        // Notes, not pads: a track that plays notes if there is one.
        let melodic = self.pulsar_tracks().into_iter().map(|t| t.0).find(|&t| self.pulsar_pads(Some(t)).is_empty()).or_else(|| self.orb_track());
        let orb = |degree: i32| PKind::orb(melodic, degree);
        let p = self.new_node(PKind::Pulsar { rate: 5, on: true, mode: 0, chance: 1.0, strength: 1.0 }, pos2(0.0, 0.0));
        let mut last = p;
        let mut far = p;
        for (k, (dist, deg)) in [(160.0, 0), (210.0, 4), (130.0, 2), (250.0, 7), (190.0, 9)].into_iter().enumerate() {
            let a = k as f32 / 5.0 * TAU - TAU / 4.0;
            let o = self.new_node(orb(deg), pos2(a.cos() * dist, a.sin() * dist));
            self.project.pulsars.link(p, o);
            if k % 2 == 1 {
                self.project.pulsars.link(last, o);
            }
            if k == 3 {
                far = o;
            }
            last = o;
        }
        if let Some(n) = self.project.pulsars.node(far).map(|n| pos2(n.x, n.y)) {
            let r = self.new_node(PKind::Reflector, n + n.to_vec2().normalized() * 90.0);
            self.project.pulsars.link(far, r);
        }
        let q = self.new_node(PKind::Pulsar { rate: 3, on: true, mode: 1, chance: 0.35, strength: 0.7 }, pos2(-600.0, -60.0));
        let d = self.new_node(PKind::Delay { wait: 2 }, pos2(-600.0, 110.0));
        let s = self.new_node(PKind::Shift { steps: 2 }, pos2(-480.0, 210.0));
        let mut chord = orb(0);
        if let PKind::Orb { octave, chord: c, length, .. } = &mut chord {
            (*octave, *c, *length) = (-1, 2, 1.0);
        }
        let c1 = self.new_node(chord, pos2(-740.0, 120.0));
        let c2 = self.new_node(chord, pos2(-400.0, 360.0));
        for (a, b) in [(q, d), (d, s), (s, c2), (q, c1)] {
            self.project.pulsars.link(a, b);
        }
        for l in self.project.pulsars.links.iter_mut().filter(|l| [d, s, c1, c2].contains(&l.b) && [q, d, s].contains(&l.a)) {
            l.one_way = true;
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
        egui::ScrollArea::vertical().id_salt("pulsars_side").show(&mut sui, |ui| self.pulsars_side(ui));
        // The field is alive: stars twinkle, pulsars turn.
        ui.ctx().request_repaint();
    }

    fn pulsars_bar(&mut self, ui: &mut Ui) {
        ui.label(RichText::new("PULSARS").strong().color(accent_now()));
        ui.add_space(10.0);
        let tools = [
            (Tool::Select, "Select", "Move nodes, join them, look round the field"),
            (Tool::Pulsar, "Pulsar", "Click the field: a pulsar, which fires a pulse down its lines on the beat (or by chance, or when you click it)"),
            (Tool::Orb, "Orb", "Click the field: an orb, which plays its note (or chord, or pad) on its track when a pulse arrives"),
            (Tool::Gate, "Gate", "Lets one pulse in so many through"),
            (Tool::Chance, "Chance", "Lets a pulse through so often"),
            (Tool::Delay, "Delay", "Holds a pulse a while before it goes on"),
            (Tool::Router, "Router", "Sends each pulse down the next of its lines in turn"),
            (Tool::Shift, "Shift", "The orbs after it play so many steps of the key higher (or lower)"),
            (Tool::Reflector, "Reflector", "Sends a pulse back the way it came"),
        ];
        for (k, (tool, name, tip)) in tools.into_iter().enumerate() {
            if k == 1 || k == 3 {
                ui.add_space(6.0);
            }
            if ui.selectable_label(self.pulsars_ui.tool == tool, name).on_hover_text(tip).clicked() {
                self.pulsars_ui.tool = tool;
            }
        }
        ui.separator();
        ui.label(RichText::new("Speed").small().color(TEXT_DIM));
        ui.add(egui::DragValue::new(&mut self.project.pulsars.speed).range(20.0..=1000.0).speed(1.0).suffix(" px a beat"))
            .on_hover_text("How far a pulse travels in a beat: a line twice as long takes twice as long");
        if ui.button("Example").on_hover_text("Pulsars with orbs round them to start from").clicked() {
            self.pulsar_example();
        }
        if ui.button("Fit").on_hover_text("Shows the whole field").clicked() {
            self.pulsars_ui.fit = true;
        }
        if !self.project.pulsars.is_empty() && ui.button("Clear").clicked() {
            self.project.pulsars = Default::default();
            self.pulsars_ui.selected = None;
        }
    }

    /// Deep space: a nebula and stars, further off than the field, so they move less as it does.
    fn pulsars_space(p: &Painter, field: Rect, pan: Vec2, zoom: f32, time: f32) {
        p.rect_filled(field, 0.0, Color32::from_rgb(0x05, 0x06, 0x0e));
        let c = field.center();
        let clouds = [(-500.0, -200.0, 0.74, 420.0), (380.0, 160.0, 0.52, 360.0), (-60.0, 380.0, 0.86, 300.0), (620.0, -340.0, 0.62, 280.0), (-820.0, 300.0, 0.58, 340.0)];
        for (k, (x, y, h, size)) in clouds.into_iter().enumerate() {
            let at = c + (vec2(x, y) - pan * 0.35) * (0.5 + zoom * 0.5);
            let breathe = 1.0 + 0.04 * (time * 0.2 + k as f32).sin();
            radial(p, at, size * breathe * (0.6 + zoom * 0.4), light(hue(h, 0.75, 0.5), 0.3));
        }
        for (layer, (count, depth, size)) in [(160u32, 0.08f32, 0.9f32), (60, 0.25, 1.5)].into_iter().enumerate() {
            for k in 0..count {
                let s = k * 7 + layer as u32 * 1013;
                let x = (hash(s + 1) * field.width() - pan.x * depth * zoom).rem_euclid(field.width());
                let y = (hash(s + 5) * field.height() - pan.y * depth * zoom).rem_euclid(field.height());
                let twinkle = 0.65 + 0.35 * (time * (0.6 + hash(s + 9) * 2.0) + hash(s + 3) * TAU).sin();
                let a = (30.0 + hash(s + 13) * 140.0) * twinkle;
                let star = hue(hash(s + 17) * 0.2 + 0.55, 0.15 * hash(s + 19), 1.0);
                p.circle_filled(pos2(field.left() + x, field.top() + y), (0.4 + hash(s + 11)) * size, light(star, a / 255.0));
            }
        }
    }

    fn pulsars_field(&mut self, ui: &mut Ui, field: Rect) {
        let p = ui.painter().with_clip_rect(field);
        if std::mem::take(&mut self.pulsars_ui.fit) {
            let places: Vec<Pos2> = self.project.pulsars.nodes.iter().map(|n| pos2(n.x, n.y)).collect();
            (self.pulsars_ui.pan, self.pulsars_ui.zoom) = fit_view(&places, field.size());
        }
        let time = ui.input(|i| i.time) as f32;
        let ui_state = &self.pulsars_ui;
        let (pan, zoom) = (ui_state.pan, ui_state.zoom);
        Self::pulsars_space(&p, field, pan, zoom, time);
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
        self.pulsars_ui.flashes.retain(|_, t| t.elapsed().as_secs_f32() < FLARE);

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
            // Where it was pressed, not where the hand has got to by now.
            let pressed = ui.input(|i| i.pointer.press_origin()).map(to_world).or(world_at);
            if let Some(w) = pressed {
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
                match node_at(w) {
                    Some((to, _)) => self.project.pulsars.link(from, to),
                    // Into empty space: a new node there (an orb, or what the tool makes), joined.
                    None if place(from).is_some_and(|a| (a - w).length() > R * 2.5) => {
                        let tool = if self.pulsars_ui.tool == Tool::Select { Tool::Orb } else { self.pulsars_ui.tool };
                        if let Some(kind) = self.kind_for(tool) {
                            let to = self.new_node(kind, w);
                            self.project.pulsars.link(from, to);
                            self.pulsars_ui.selected = Some(Sel::Node(to));
                        }
                    }
                    None => {}
                }
            }
            self.pulsars_ui.drag = None;
        }
        if resp.clicked() {
            if let Some(w) = world_at {
                let tool = self.pulsars_ui.tool;
                self.pulsars_ui.selected = match (node_at(w), link_at(w)) {
                    (Some((id, _)), _) => {
                        // A pulsar fired by hand fires.
                        if matches!(set.node(id).map(|n| n.kind), Some(PKind::Pulsar { mode: 2, on: true, .. })) && self.is_playing() {
                            self.cmd(Command::PulsarFire { node: id });
                        }
                        Some(Sel::Node(id))
                    }
                    (None, Some(k)) if tool == Tool::Select => Some(Sel::Link(k)),
                    _ => self.kind_for(tool).map(|k| Sel::Node(self.new_node(k, w))),
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

        let set = self.project.pulsars.clone();
        let place = |id: u64| set.node(id).map(|n| pos2(n.x, n.y));
        let scale = self.project.scale;
        // The pulses under way, and the lines they light.
        let order = kp::line_order(&set);
        let shown = status.pulsars.read();
        let mut hot: HashMap<usize, Color32> = HashMap::new();
        for s in &shown {
            if let Some(&k) = order.get(s.line) {
                hot.insert(k, hue(pulsar_hue(s.origin), 0.75, 1.0));
            }
        }
        // The lines.
        for (k, l) in set.links.iter().enumerate() {
            let (Some(a), Some(b)) = (place(l.a), place(l.b)) else { continue };
            let (sa, sb) = (to_screen(a), to_screen(b));
            let picked = self.pulsars_ui.selected == Some(Sel::Link(k));
            let base = if picked { accent_now() } else { Color32::from_rgb(0x34, 0x3e, 0x62) };
            if let Some(&col) = hot.get(&k) {
                p.line_segment([sa, sb], Stroke::new(7.0_f32, light(col, 0.12)));
                p.line_segment([sa, sb], Stroke::new(2.0_f32, mix(base, col, 0.45)));
            } else {
                p.line_segment([sa, sb], Stroke::new(if picked { 3.0 } else { 1.5_f32 }, base));
            }
            if l.one_way {
                let d = (sb - sa).normalized();
                let n = vec2(-d.y, d.x);
                for t in [0.35, 0.65] {
                    let mid = sa + (sb - sa) * t;
                    p.add(Shape::convex_polygon(vec![mid + d * 6.0, mid - d * 4.0 + n * 4.5, mid - d * 4.0 - n * 4.5], mix(base, Color32::WHITE, 0.25), Stroke::NONE));
                }
            }
        }
        // The pulses: comets in their pulsar's colour.
        for s in &shown {
            let Some(l) = order.get(s.line).and_then(|&k| set.links.get(k)) else { continue };
            let (Some(a), Some(b)) = (place(l.a), place(l.b)) else { continue };
            let (from, to) = if s.from_a { (a, b) } else { (b, a) };
            let (sf, st) = (to_screen(from), to_screen(to));
            let len = (st - sf).length().max(1.0);
            let col = hue(pulsar_hue(s.origin), 0.6, 1.0);
            let r = 3.5 * zoom.sqrt();
            let at = sf + (st - sf) * s.along;
            // The tail: so many pixels long (no further back than where it left), thinning and fading.
            let tail = (46.0 * zoom.sqrt()).min(s.along * len);
            if tail > 1.0 {
                let d = (st - sf) / len;
                let n = vec2(-d.y, d.x) * r * 0.9;
                let mut mesh = egui::Mesh::default();
                mesh.colored_vertex(at + n, light(col, 0.8));
                mesh.colored_vertex(at - n, light(col, 0.8));
                mesh.colored_vertex(at - d * tail, Color32::TRANSPARENT);
                mesh.add_triangle(0, 1, 2);
                p.add(Shape::mesh(mesh));
            }
            glow(&p, at, 4.0 * zoom.sqrt(), col, 0.9);
            p.circle_filled(at, r, mix(col, Color32::WHITE, 0.75));
        }
        // A line being drawn.
        if let (Some(Drag::Link(from)), Some(sp)) = (self.pulsars_ui.drag, pointer) {
            if let Some(a) = place(from) {
                let onto = world_at.and_then(node_at).is_some();
                p.line_segment([to_screen(a), sp], Stroke::new(2.0_f32, accent_now()));
                if !onto {
                    p.circle_stroke(sp, R * zoom * 0.8, Stroke::new(1.0_f32, Color32::from_white_alpha(90)));
                }
            }
        }
        // The nodes.
        let flashes = self.pulsars_ui.flashes.clone();
        let flash = |id: u64| flashes.get(&id).map_or(0.0, |t| (1.0 - t.elapsed().as_secs_f32() / FLARE).max(0.0));
        let tracks = self.project.tracks.clone();
        let playing = self.is_playing();
        for (place_k, n) in set.nodes.iter().enumerate() {
            let at = to_screen(pos2(n.x, n.y));
            let r = R * zoom;
            let lit = flash(n.id);
            let picked = self.pulsars_ui.selected == Some(Sel::Node(n.id));
            // The ring a strike sends out.
            let ring = |col: Color32| {
                if lit > 0.0 {
                    let t = 1.0 - lit;
                    p.circle_stroke(at, r * (1.0 + 2.2 * t), Stroke::new(2.5 * lit, light(col, lit)));
                }
            };
            match n.kind {
                PKind::Pulsar { rate, on, mode, .. } => {
                    let h = pulsar_hue(place_k);
                    let col = if on { hue(h, 0.7, 1.0) } else { Color32::from_gray(0x60) };
                    glow(&p, at, r * 0.9, col, if on { 0.45 + 0.6 * lit } else { 0.1 });
                    ring(col);
                    // The rays turn.
                    let spin = if on && playing { time * 0.7 } else { 0.0 } + place_k as f32;
                    let rays = 12;
                    for k in 0..rays {
                        let a = spin + k as f32 / rays as f32 * TAU;
                        let d = vec2(a.cos(), a.sin());
                        let long = if k % 2 == 0 { 1.55 } else { 1.2 } + 0.45 * lit;
                        p.line_segment([at + d * r * 0.7, at + d * r * long], Stroke::new(1.6_f32, light(col, 0.8)));
                    }
                    if mode == 2 {
                        // By hand: a ring to press.
                        p.circle_stroke(at, r * 0.82, Stroke::new(1.5_f32, mix(col, Color32::WHITE, 0.4)));
                    }
                    p.circle_filled(at, r * 0.6, mix(col, Color32::WHITE, 0.35 + lit * 0.6));
                    p.circle_filled(at, r * 0.32, mix(col, Color32::WHITE, 0.85));
                    let label = match mode {
                        1 => format!("{} ?", RATES[(rate as usize).min(RATES.len() - 1)].0),
                        2 => "Click".to_string(),
                        _ => RATES[(rate as usize).min(RATES.len() - 1)].0.to_string(),
                    };
                    text(&p, at + vec2(0.0, r * 1.95), Align2::CENTER_CENTER, label, small(), mix(col, TEXT_DIM, 0.4));
                }
                PKind::Orb { track, chord, note, .. } => {
                    let notes = kp::orb_notes(&scale, &n.kind, 0);
                    let pitch = notes.first().copied().unwrap_or(60);
                    let col = hue(note_hue(pitch), 0.65, 1.0);
                    let track_col = track.and_then(|id| tracks.iter().find(|t| t.id == id)).map_or(Color32::from_gray(0x70), |t| rgb(t.color));
                    glow(&p, at, r * 0.8, col, 0.25 + 0.7 * lit);
                    ring(col);
                    let body = r * (0.85 + 0.15 * lit);
                    p.circle_filled(at, body, hue(note_hue(pitch), 0.75, 0.22 + 0.35 * lit));
                    // A sphere: lit from up and to the left.
                    radial(&p, at - vec2(body, body) * 0.25, body * 1.05, light(col, 0.55 + 0.4 * lit));
                    radial(&p, at - vec2(body, body) * 0.38, body * 0.4, light(Color32::WHITE, 0.4));
                    p.circle_stroke(at, body, Stroke::new(2.0_f32, track_col));
                    let name = match note {
                        Some(n) => self.pulsar_pads(track).into_iter().find(|p| p.0 == n).map_or_else(|| note_name(n), |p| p.1.chars().take(5).collect()),
                        None => note_name(pitch),
                    };
                    text(&p, at + vec2(0.0, body + 10.0), Align2::CENTER_CENTER, &name, small(), mix(col, TEXT, 0.5));
                    if chord > 0 && note.is_none() {
                        // A dot a note of the chord.
                        let k = notes.len();
                        for i in 0..k {
                            let a = -TAU / 4.0 + (i as f32 - (k - 1) as f32 / 2.0) * 0.38;
                            p.circle_filled(at + vec2(a.cos(), a.sin()) * (body + 5.0), 2.2, hue(note_hue(notes[i]), 0.5, 1.0));
                        }
                    }
                }
                PKind::Gate { every } => {
                    let col = Color32::from_rgb(0xe8, 0xb0, 0x40);
                    glow(&p, at, r * 0.7, col, 0.15 + 0.8 * lit);
                    ring(col);
                    let box_r = Rect::from_center_size(at, vec2(r * 1.5, r * 1.5));
                    p.rect_filled(box_r, 4.0, mix(Color32::from_rgb(0x2a, 0x22, 0x10), col, 0.2 + lit * 0.6));
                    p.rect_stroke(box_r, 4.0, Stroke::new(1.5_f32, col), egui::StrokeKind::Inside);
                    text(&p, at, Align2::CENTER_CENTER, format!("÷{every}"), small(), Color32::WHITE);
                }
                PKind::Chance { chance } => {
                    let col = Color32::from_rgb(0xc0, 0x70, 0xff);
                    glow(&p, at, r * 0.7, col, 0.15 + 0.8 * lit);
                    ring(col);
                    let d = r * 0.95;
                    let pts = vec![at + vec2(0.0, -d), at + vec2(d, 0.0), at + vec2(0.0, d), at + vec2(-d, 0.0)];
                    p.add(Shape::convex_polygon(pts, mix(Color32::from_rgb(0x24, 0x14, 0x34), col, 0.2 + lit * 0.6), Stroke::new(1.5_f32, col)));
                    text(&p, at, Align2::CENTER_CENTER, format!("{:.0}%", chance * 100.0), small(), Color32::WHITE);
                }
                PKind::Reflector => {
                    let col = Color32::from_rgb(0x6c, 0xd0, 0xff);
                    glow(&p, at, r * 0.7, col, 0.2 + 0.8 * lit);
                    ring(col);
                    p.circle_stroke(at, r * 0.8, Stroke::new(3.0_f32, mix(col, Color32::WHITE, lit * 0.6)));
                    p.circle_stroke(at, r * 0.45, Stroke::new(1.5_f32, col));
                }
                PKind::Delay { wait } => {
                    let col = Color32::from_rgb(0x50, 0xe0, 0xb0);
                    glow(&p, at, r * 0.7, col, 0.15 + 0.8 * lit);
                    ring(col);
                    p.circle_filled(at, r * 0.8, mix(Color32::from_rgb(0x0e, 0x24, 0x20), col, 0.15 + lit * 0.5));
                    p.circle_stroke(at, r * 0.8, Stroke::new(1.5_f32, col));
                    // A clock's hand, going round while a pulse waits.
                    let a = time * if lit > 0.0 { 6.0 } else { 0.0 } - TAU / 4.0;
                    p.line_segment([at, at + vec2(a.cos(), a.sin()) * r * 0.6], Stroke::new(1.5_f32, col));
                    text(&p, at + vec2(0.0, r * 1.5), Align2::CENTER_CENTER, RATES[(wait as usize).min(RATES.len() - 1)].0, small(), mix(col, TEXT_DIM, 0.4));
                }
                PKind::Router => {
                    let col = Color32::from_rgb(0x9c, 0xe0, 0x50);
                    glow(&p, at, r * 0.7, col, 0.15 + 0.8 * lit);
                    ring(col);
                    let pts: Vec<Pos2> = (0..6).map(|k| at + vec2((k as f32 / 6.0 * TAU).cos(), (k as f32 / 6.0 * TAU).sin()) * r * 0.9).collect();
                    p.add(Shape::convex_polygon(pts, mix(Color32::from_rgb(0x18, 0x24, 0x0c), col, 0.15 + lit * 0.5), Stroke::new(1.5_f32, col)));
                    // Three ways out from the middle.
                    for k in 0..3 {
                        let a = -0.6 + k as f32 * 0.6;
                        p.line_segment([at - vec2(r * 0.35, 0.0), at + vec2(a.cos(), a.sin()) * r * 0.5], Stroke::new(1.5_f32, Color32::WHITE));
                    }
                }
                PKind::Shift { steps } => {
                    let col = Color32::from_rgb(0xff, 0x6a, 0xa8);
                    glow(&p, at, r * 0.7, col, 0.15 + 0.8 * lit);
                    ring(col);
                    let up = if steps >= 0 { 1.0 } else { -1.0 };
                    let pts = vec![at + vec2(0.0, -r * up), at + vec2(r * 0.95, r * 0.6 * up), at + vec2(-r * 0.95, r * 0.6 * up)];
                    p.add(Shape::convex_polygon(pts, mix(Color32::from_rgb(0x30, 0x10, 0x20), col, 0.2 + lit * 0.5), Stroke::new(1.5_f32, col)));
                    text(&p, at + vec2(0.0, r * 0.15 * up), Align2::CENTER_CENTER, format!("{steps:+}"), small(), Color32::WHITE);
                }
            }
            if picked {
                p.circle_stroke(at, r * 1.4, Stroke::new(1.5_f32, Color32::WHITE));
            }
        }
        // Where the hand is: the middle moves, the edge joins.
        if let (Some(w), None) = (world_at, self.pulsars_ui.drag) {
            if let Some((id, d)) = node_at(w) {
                if let Some(at) = place(id).map(to_screen) {
                    if takes_middle(d) {
                        let manual = matches!(set.node(id).map(|n| n.kind), Some(PKind::Pulsar { mode: 2, .. }));
                        ui.ctx().set_cursor_icon(if manual { egui::CursorIcon::PointingHand } else { egui::CursorIcon::Grab });
                    } else {
                        p.circle_stroke(at, R * zoom * 1.3, Stroke::new(1.0_f32, Color32::from_white_alpha(120)));
                        ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
                    }
                }
            }
        }
        if set.is_empty() {
            text(&p, field.center() - vec2(0.0, 12.0), Align2::CENTER_CENTER, "Pick Pulsar or Orb above and click the field to place it.", body(), TEXT);
            text(&p, field.center() + vec2(0.0, 12.0), Align2::CENTER_CENTER, "Drag from the edge of one node to another to join them, or start from the Example.", small(), TEXT_DIM);
        }
    }

    fn pulsars_side(&mut self, ui: &mut Ui) {
        let scale = self.project.scale;
        let tracks = self.pulsar_tracks();
        let playing = self.is_playing();
        match self.pulsars_ui.selected {
            Some(Sel::Node(id)) => {
                let orb_track = match self.project.pulsars.node(id).map(|n| n.kind) {
                    Some(PKind::Orb { track, .. }) => track,
                    _ => None,
                };
                let pads = self.pulsar_pads(orb_track);
                let mut fire = false;
                let Some(n) = self.project.pulsars.nodes.iter_mut().find(|n| n.id == id) else {
                    self.pulsars_ui.selected = None;
                    return;
                };
                ui.label(RichText::new(n.kind.name()).strong().size(16.0).color(TEXT));
                ui.add_space(6.0);
                let dim = |ui: &mut Ui, s: &str| {
                    ui.label(RichText::new(s).small().color(TEXT_DIM));
                };
                match &mut n.kind {
                    PKind::Pulsar { rate, on, mode, chance, strength } => {
                        ui.checkbox(on, "On").on_hover_text("Fires its pulses while the song plays");
                        dim(ui, "Fires");
                        egui::ComboBox::from_id_salt(("pulsar_mode", id)).selected_text(MODES[(*mode as usize).min(MODES.len() - 1)]).show_ui(ui, |ui| {
                            for (k, m) in MODES.iter().enumerate() {
                                ui.selectable_value(mode, k as u8, *m);
                            }
                        });
                        if *mode == 2 {
                            ui.add_space(4.0);
                            if ui.add_enabled(playing && *on, egui::Button::new("Fire")).on_hover_text("Fires it now (or click it on the field while the song plays)").on_disabled_hover_text("Play the song to fire it").clicked() {
                                fire = true;
                            }
                        } else {
                            dim(ui, "Every");
                            egui::ComboBox::from_id_salt(("pulsar_rate", id)).selected_text(RATES[(*rate as usize).min(RATES.len() - 1)].0).show_ui(ui, |ui| {
                                for (k, r) in RATES.iter().enumerate() {
                                    ui.selectable_value(rate, k as u8, r.0);
                                }
                            });
                        }
                        if *mode == 1 {
                            dim(ui, "Chance it fires");
                            let mut pct = *chance * 100.0;
                            if ui.add(egui::Slider::new(&mut pct, 0.0..=100.0).suffix(" %")).changed() {
                                *chance = pct / 100.0;
                            }
                        }
                        dim(ui, "Strength (how hard its notes play)");
                        let mut pct = *strength * 100.0;
                        if ui.add(egui::Slider::new(&mut pct, 5.0..=100.0).suffix(" %")).changed() {
                            *strength = pct / 100.0;
                        }
                    }
                    PKind::Orb { track, degree, octave, velocity, length, chord, note } => {
                        dim(ui, "Plays on");
                        let name = track.and_then(|t| tracks.iter().find(|x| x.0 == t)).map_or("Nothing".to_string(), |t| t.1.clone());
                        egui::ComboBox::from_id_salt(("orb_track", id)).width(200.0).selected_text(name).show_ui(ui, |ui| {
                            for (t, name) in &tracks {
                                ui.selectable_value(track, Some(*t), name);
                            }
                        });
                        if !pads.is_empty() {
                            dim(ui, "Pad");
                            let now = note.and_then(|n| pads.iter().find(|p| p.0 == n)).map_or("Pick a pad".to_string(), |p| p.1.clone());
                            egui::ComboBox::from_id_salt(("orb_pad", id)).width(200.0).height(320.0).selected_text(now).show_ui(ui, |ui| {
                                for (n, name) in &pads {
                                    ui.selectable_value(note, Some(*n), name);
                                }
                            });
                        } else {
                            dim(ui, "Note");
                            let now = match *note {
                                Some(n) => note_name(n),
                                None => note_name(kp::orb_pitch(&scale, *degree, *octave)),
                            };
                            let o = *octave;
                            egui::ComboBox::from_id_salt(("orb_note", id)).width(80.0).height(320.0).selected_text(now).show_ui(ui, |ui| {
                                for d in DEGREES.rev() {
                                    if ui.selectable_label(note.is_none() && *degree == d, note_name(kp::orb_pitch(&scale, d, o))).clicked() {
                                        *degree = d;
                                        *note = None;
                                    }
                                }
                            });
                            ui.horizontal(|ui| {
                                dim(ui, "Octave");
                                if ui.small_button("-").clicked() {
                                    *octave = (*octave - 1).max(-3);
                                }
                                ui.label(format!("{:+}", *octave));
                                if ui.small_button("+").clicked() {
                                    *octave = (*octave + 1).min(3);
                                }
                            });
                            dim(ui, "Plays");
                            egui::ComboBox::from_id_salt(("orb_chord", id)).selected_text(CHORDS[(*chord as usize).min(CHORDS.len() - 1)].0).show_ui(ui, |ui| {
                                for (k, c) in CHORDS.iter().enumerate() {
                                    ui.selectable_value(chord, k as u8, c.0);
                                }
                            });
                        }
                        dim(ui, "Velocity");
                        ui.add(egui::Slider::new(velocity, 0.05..=1.0).show_value(false));
                        dim(ui, "Length");
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
                            dim(ui, "Lets through one in");
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
                        dim(ui, "Lets a pulse through");
                        let mut pct = *chance * 100.0;
                        if ui.add(egui::Slider::new(&mut pct, 0.0..=100.0).suffix(" %")).changed() {
                            *chance = pct / 100.0;
                        }
                    }
                    PKind::Reflector => dim(ui, "Sends a pulse back the way it came, and what it passes plays again"),
                    PKind::Delay { wait } => {
                        dim(ui, "Holds a pulse for");
                        egui::ComboBox::from_id_salt(("delay_wait", id)).selected_text(RATES[(*wait as usize).min(RATES.len() - 1)].0).show_ui(ui, |ui| {
                            for (k, r) in RATES.iter().enumerate() {
                                ui.selectable_value(wait, k as u8, r.0);
                            }
                        });
                    }
                    PKind::Router => dim(ui, "Sends each pulse down the next of its lines in turn, round and round"),
                    PKind::Shift { steps } => {
                        dim(ui, "The orbs after it play higher by");
                        ui.horizontal(|ui| {
                            if ui.small_button("-").clicked() {
                                *steps = (*steps - 1).max(-14);
                            }
                            ui.label(format!("{steps:+} steps"));
                            if ui.small_button("+").clicked() {
                                *steps = (*steps + 1).min(14);
                            }
                        });
                        dim(ui, "Steps of the song's key; shifts add up along the way");
                    }
                }
                ui.add_space(10.0);
                if ui.button("Delete").clicked() {
                    self.project.pulsars.remove(id);
                    self.pulsars_ui.selected = None;
                }
                if fire {
                    self.cmd(Command::PulsarFire { node: id });
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
                    ui.checkbox(&mut l.one_way, "One way").on_hover_text("Pulses go one way down it only (the arrows)");
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
                    "A pulsar fires a pulse down each of its lines: on the beat, by chance, or when you click it.",
                    "A pulse takes as long as its line is long: where things are is the rhythm.",
                    "An orb plays its note, chord or pad on its track, and the pulse goes on down its other lines.",
                    "Gates, chances, delays, routers, shifts and reflectors change what goes on.",
                    "Drag a node's middle to move it, its edge to another to join them, or to empty space for a new orb. Drag the field to look round, the wheel zooms.",
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

    #[test]
    fn fit_shows_every_node() {
        let (pan, zoom) = fit_view(&[pos2(-500.0, 0.0), pos2(500.0, 100.0)], vec2(800.0, 600.0));
        assert_eq!(pan, vec2(0.0, 50.0));
        assert!(zoom * (1000.0 + R * 6.0) <= 800.0 + 1e-3, "{zoom}");
        assert_eq!(fit_view(&[], vec2(800.0, 600.0)), (Vec2::ZERO, 1.0));
    }

    #[test]
    fn stars_are_scattered_not_in_lines() {
        // Neighbouring seeds land far apart.
        let near = (0..200u32).filter(|&k| (hash(k * 7 + 1) - hash(k * 7 + 8)).abs() < 0.02).count();
        assert!(near < 12, "{near}");
        assert!((0..1000).all(|k| (0.0..1.0).contains(&hash(k))));
    }

    #[test]
    fn notes_have_their_colour_and_pulsars_differ() {
        assert_eq!(note_hue(60), note_hue(72), "an octave: the same colour");
        assert_ne!(note_hue(60), note_hue(67));
        let (a, b) = (pulsar_hue(0).rem_euclid(1.0), pulsar_hue(1).rem_euclid(1.0));
        assert!((a - b).abs() > 0.2, "{a} {b}");
        assert_eq!(light(Color32::WHITE, 0.5).a(), 0, "light adds");
    }

    #[test]
    fn every_tool_but_select_and_orb_makes_its_node() {
        assert!(tool_kind(Tool::Select).is_none());
        for (t, name) in [(Tool::Pulsar, "Pulsar"), (Tool::Delay, "Delay"), (Tool::Router, "Router"), (Tool::Shift, "Shift"), (Tool::Reflector, "Reflector")] {
            assert_eq!(tool_kind(t).unwrap().name(), name);
        }
    }
}

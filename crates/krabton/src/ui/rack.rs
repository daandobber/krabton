//! The Modular rack: rows of eurorack-style modules with knobs and jacks,
//! patched by dragging cables from jack to jack.

use crate::app::App;
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, Context, Id, Pos2, Rect, RichText, Sense, Shape, Stroke, Ui};
use krabton_engine::modular::{self, presets, Module, ModuleKind, Patch, CATEGORIES, ROW_HP};
use krabton_engine::params::ParamSpec;
use krabton_engine::Instrument;

/// One HP in pixels, and the height of a 3U row.
const HP: f32 = 15.0;
const ROW_H: f32 = 380.0;
const RAIL: f32 = 14.0;
const TITLE_H: f32 = 27.0;
const KNOB_W: f32 = 52.0;
const KNOB_H: f32 = 58.0;
const JACK_W: f32 = 38.0;
const JACK_H: f32 = 36.0;
const JACK_R: f32 = 7.5;

const CABLE_COLORS: [Color32; 6] = [
    Color32::from_rgb(0xff, 0x5a, 0x4a),
    Color32::from_rgb(0xff, 0xc4, 0x2e),
    Color32::from_rgb(0x4c, 0xd9, 0x64),
    Color32::from_rgb(0x4d, 0xa8, 0xff),
    Color32::from_rgb(0xc0, 0x6c, 0xff),
    Color32::from_rgb(0xf2, 0xf2, 0xf2),
];

/// What the rack window is doing with the mouse.
#[derive(Default)]
pub struct RackUi {
    /// The track whose rack is open.
    pub open: Option<u64>,
    /// A cable being drawn from (module, port, is output).
    cable: Option<(u32, u8, bool)>,
    /// A module being moved, and where it was grabbed.
    moving: Option<(u32, egui::Vec2)>,
}

fn category_color(cat: &str) -> Color32 {
    match cat {
        "Sources" => Color32::from_rgb(0x3b, 0x3b, 0x44),
        "Filters & Amps" => Color32::from_rgb(0x3f, 0x39, 0x36),
        "Envelopes & LFOs" => Color32::from_rgb(0x36, 0x3e, 0x3a),
        "Sequencing" => Color32::from_rgb(0x3c, 0x37, 0x42),
        "Utilities" => Color32::from_rgb(0x38, 0x3a, 0x3e),
        _ => Color32::from_rgb(0x30, 0x30, 0x34),
    }
}

fn accent_color(cat: &str) -> Color32 {
    match cat {
        "Sources" => Color32::from_rgb(0x4d, 0xa8, 0xff),
        "Filters & Amps" => Color32::from_rgb(0xff, 0xa7, 0x33),
        "Envelopes & LFOs" => Color32::from_rgb(0x4c, 0xd9, 0x64),
        "Sequencing" => Color32::from_rgb(0xc0, 0x6c, 0xff),
        "Utilities" => Color32::from_rgb(0xb0, 0xb0, 0xb8),
        _ => Color32::from_rgb(0xff, 0x5a, 0x4a),
    }
}

fn module_rect(origin: Pos2, m: &Module) -> Rect {
    let hp = m.kind.info().hp as f32;
    Rect::from_min_size(origin + vec2(m.x as f32 * HP + 1.0, m.row as f32 * ROW_H + RAIL), vec2(hp * HP - 2.0, ROW_H - 2.0 * RAIL))
}

/// Where a module's knobs and jacks go: (knob cells, input jacks, output jacks, output area).
fn layout(rect: Rect, kind: ModuleKind, n_params: usize) -> (Vec<Rect>, Vec<Pos2>, Vec<Pos2>, Option<Rect>) {
    let info = kind.info();
    let cols = (((rect.width() - 6.0) / KNOB_W).floor() as usize).max(1);
    let x0 = rect.left() + (rect.width() - cols as f32 * KNOB_W) * 0.5;
    let mut knobs = Vec::new();
    for i in 0..n_params {
        let (c, r) = (i % cols, i / cols);
        knobs.push(Rect::from_min_size(pos2(x0 + c as f32 * KNOB_W, rect.top() + TITLE_H + r as f32 * KNOB_H), vec2(KNOB_W - 2.0, KNOB_H - 2.0)));
    }
    let knob_rows = n_params.div_ceil(cols);
    let jcols = (((rect.width() - 6.0) / JACK_W).floor() as usize).max(1);
    let jx0 = |n: usize| rect.left() + (rect.width() - n.min(jcols) as f32 * JACK_W) * 0.5;
    let place = |n: usize, top: f32| -> Vec<Pos2> { (0..n).map(|i| pos2(jx0(n) + (i % jcols) as f32 * JACK_W + JACK_W * 0.5, top + (i / jcols) as f32 * JACK_H + JACK_R + 4.0)).collect() };
    let in_top = rect.top() + TITLE_H + knob_rows as f32 * KNOB_H + 4.0;
    let ins = place(info.inputs.len(), in_top);
    let out_top = in_top + info.inputs.len().div_ceil(jcols) as f32 * JACK_H + 4.0;
    let outs = place(info.outputs.len(), out_top);
    let out_area = (!info.outputs.is_empty()).then(|| Rect::from_min_max(pos2(rect.left() + 3.0, out_top - 2.0), pos2(rect.right() - 3.0, out_top + info.outputs.len().div_ceil(jcols) as f32 * JACK_H + 2.0)));
    (knobs, ins, outs, out_area)
}

/// A cable that hangs a little, like a real one.
fn cable_shape(a: Pos2, b: Pos2, color: Color32) -> Vec<Shape> {
    let sag = 30.0 + 0.25 * a.distance(b);
    let (c1, c2) = (a + vec2(0.0, sag), b + vec2(0.0, sag));
    let pts: Vec<Pos2> = (0..=24)
        .map(|i| {
            let t = i as f32 / 24.0;
            let u = 1.0 - t;
            let x = u * u * u * a.x + 3.0 * u * u * t * c1.x + 3.0 * u * t * t * c2.x + t * t * t * b.x;
            let y = u * u * u * a.y + 3.0 * u * u * t * c1.y + 3.0 * u * t * t * c2.y + t * t * t * b.y;
            pos2(x, y)
        })
        .collect();
    vec![
        Shape::line(pts.clone(), Stroke::new(5.0_f32, Color32::from_black_alpha(140))),
        Shape::line(pts, Stroke::new(3.5_f32, color.gamma_multiply(0.92))),
        Shape::circle_filled(a, 4.5, color),
        Shape::circle_filled(b, 4.5, color),
    ]
}

impl App {
    pub(crate) fn ui_rack(&mut self, ctx: &Context) {
        let Some(track_id) = self.rack.open else { return };
        let Some(ti) = self.project.tracks.iter().position(|t| t.id == track_id) else {
            self.rack.open = None;
            return;
        };
        let title = format!("Modular - {}", self.project.tracks[ti].name);
        let mut open = true;
        let rack = &mut self.rack;
        let Some(Instrument::Modular { preset, params, patch }) = &mut self.project.tracks[ti].instrument else {
            rack.open = None;
            return;
        };
        egui::Window::new(title)
            .id(Id::new(("modular_rack", track_id)))
            .open(&mut open)
            .default_size([1180.0, 720.0])
            .resizable(true)
            .collapsible(true)
            .show(ctx, |ui| rack_ui(ui, track_id, rack, preset, params, patch));
        if !open {
            self.rack.open = None;
        }
    }
}

fn rack_ui(ui: &mut Ui, track_id: u64, rack: &mut RackUi, preset: &mut String, params: &mut Vec<f32>, patch: &mut Patch) {
    // Toolbar: add modules, presets, voices.
    ui.horizontal(|ui| {
        for cat in CATEGORIES {
            ui.menu_button(cat, |ui| {
                for kind in ModuleKind::ALL.into_iter().filter(|k| k.info().category == cat) {
                    let info = kind.info();
                    if ui.button(format!("{}  ({} HP)", info.name, info.hp)).on_hover_text(info.description).clicked() {
                        patch.add(kind, 0);
                        ui.close();
                    }
                }
            });
        }
        ui.separator();
        ui.label(RichText::new("Patch").small().color(TEXT_DIM));
        egui::ComboBox::from_id_salt(("modular_preset_rack", track_id)).width(120.0).selected_text(preset.as_str()).show_ui(ui, |ui| {
            for name in presets::preset_names() {
                if ui.selectable_label(preset == name, *name).clicked() {
                    let (p, pt) = presets::preset(name);
                    *preset = name.to_string();
                    *params = p;
                    *patch = pt;
                }
            }
        });
        if params.len() > modular::VOICES {
            ui.label(RichText::new("Voices").small().color(TEXT_DIM));
            let mut v = params[modular::VOICES].round() as i32;
            if ui.add(egui::DragValue::new(&mut v).range(1..=8)).on_hover_text("1: mono (legato, clocks and drones run free). More: every voice runs its own copy of the patch").changed() {
                params[modular::VOICES] = v as f32;
            }
        }
        ui.label(RichText::new("Drag from a jack to a jack to patch; drag an input's cable to move it; right-click a jack to unplug, a module's name to remove it").small().color(TEXT_DIM));
    });
    ui.separator();
    let rows = patch.modules.iter().map(|m| m.row + 1).max().unwrap_or(0) + 1;
    let size = vec2(ROW_HP as f32 * HP + 2.0, rows as f32 * ROW_H);
    egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
        let (canvas, _) = ui.allocate_exact_size(size, Sense::hover());
        let origin = canvas.min;
        let p = ui.painter_at(canvas);
        p.rect_filled(canvas, 0.0, Color32::from_rgb(0x14, 0x14, 0x16));
        // Rails with their holes.
        for row in 0..rows {
            for y in [row as f32 * ROW_H, (row + 1) as f32 * ROW_H - RAIL] {
                let r = Rect::from_min_size(origin + vec2(0.0, y + 2.0), vec2(size.x, RAIL - 4.0));
                p.rect_filled(r, 2.0, Color32::from_rgb(0x8c, 0x8e, 0x94));
                for k in 0..ROW_HP {
                    p.circle_filled(pos2(origin.x + k as f32 * HP + HP * 0.5, r.center().y), 1.6, Color32::from_rgb(0x4a, 0x4b, 0x50));
                }
            }
        }
        let mut jacks: Vec<(u32, u8, bool, Pos2)> = Vec::new();
        let mut remove: Option<u32> = None;
        let mut duplicate: Option<u32> = None;
        let mut unplug: Option<(u32, u8, bool)> = None;
        for mi in 0..patch.modules.len() {
            let (id, kind) = (patch.modules[mi].id, patch.modules[mi].kind);
            let info = kind.info();
            let rect = module_rect(origin, &patch.modules[mi]);
            let accent = accent_color(info.category);
            p.rect_filled(rect, 3.0, category_color(info.category));
            p.rect_stroke(rect, 3.0, Stroke::new(1.0_f32, Color32::from_rgb(0x55, 0x56, 0x5c)), egui::StrokeKind::Inside);
            // Screws.
            for c in [rect.left_top() + vec2(7.0, 6.0), rect.right_top() + vec2(-7.0, 6.0), rect.left_bottom() + vec2(7.0, -6.0), rect.right_bottom() + vec2(-7.0, -6.0)] {
                p.circle_filled(c, 2.5, Color32::from_rgb(0x9a, 0x9b, 0xa0));
            }
            let title = Rect::from_min_size(rect.min + vec2(0.0, 5.0), vec2(rect.width(), TITLE_H - 10.0));
            p.text(title.center(), Align2::CENTER_CENTER, info.name, egui::FontId::proportional(11.5), Color32::WHITE);
            p.line_segment([pos2(rect.left() + 6.0, title.bottom() + 1.0), pos2(rect.right() - 6.0, title.bottom() + 1.0)], Stroke::new(2.0_f32, accent));
            // Move by the title; right-click it for more.
            let head = ui.interact(title, Id::new(("rack_head", track_id, id)), Sense::click_and_drag());
            if head.drag_started() {
                let grab = ui.input(|i| i.pointer.press_origin()).unwrap_or(rect.min) - rect.min;
                rack.moving = Some((id, grab));
            }
            head.context_menu(|ui| {
                if ui.button("Duplicate").clicked() {
                    duplicate = Some(id);
                    ui.close();
                }
                if ui.button("Remove module").clicked() {
                    remove = Some(id);
                    ui.close();
                }
            });
            let head = head.on_hover_text(format!("{}: {}\nDrag the name to move it", info.name, info.description));
            let _ = head;
            // Knobs. The Macros module shows the instrument's macros.
            let macros = kind == ModuleKind::Macros;
            let n_knobs = if macros { 8 } else { info.params.len() };
            let (knobs, ins, outs, out_area) = layout(rect, kind, n_knobs);
            if let Some(a) = out_area {
                p.rect_filled(a, 3.0, Color32::from_rgb(0x1c, 0x1c, 0x1f));
                p.text(pos2(a.right() - 3.0, a.top() + 1.0), Align2::RIGHT_TOP, "out", egui::FontId::proportional(8.0), TEXT_DIM);
            }
            for (k, cell) in knobs.iter().enumerate() {
                if macros {
                    if let Some(v) = params.get_mut(k) {
                        let spec: &ParamSpec = &modular::SPECS[k];
                        let short = ParamSpec { name: ["M1", "M2", "M3", "M4", "M5", "M6", "M7", "M8"][k], ..*spec };
                        param_cell(ui, *cell, Id::new(("rack_macro", track_id, k)), &short, v, accent);
                    }
                } else if let (Some(spec), Some(v)) = (info.params.get(k), patch.modules[mi].params.get_mut(k)) {
                    param_cell(ui, *cell, Id::new(("rack_knob", track_id, id, k)), spec, v, accent);
                }
            }
            for (is_out, list, names) in [(false, &ins, info.inputs), (true, &outs, info.outputs)] {
                for (j, c) in list.iter().enumerate() {
                    let jr = Rect::from_center_size(*c, vec2(JACK_R * 2.6, JACK_R * 2.6));
                    let resp = ui.interact(jr, Id::new(("rack_jack", track_id, id, is_out, j)), Sense::click_and_drag());
                    let hot = resp.hovered() || resp.dragged();
                    p.circle_filled(*c, JACK_R + 1.5, if hot { Color32::WHITE } else { Color32::from_rgb(0xb4, 0xb6, 0xbc) });
                    p.circle_filled(*c, JACK_R - 1.0, Color32::from_rgb(0x0e, 0x0e, 0x10));
                    p.text(*c + vec2(0.0, JACK_R + 2.0), Align2::CENTER_TOP, names[j], egui::FontId::proportional(8.5), if is_out { Color32::WHITE } else { TEXT_DIM });
                    if resp.drag_started() {
                        // An input with a cable: pick that cable up by its plug.
                        let existing = if is_out { None } else { patch.cables.iter().position(|cb| cb.to == id && cb.input as usize == j) };
                        match existing {
                            Some(k) => {
                                let cb = patch.cables.remove(k);
                                rack.cable = Some((cb.from, cb.out, true));
                            }
                            None => rack.cable = Some((id, j as u8, is_out)),
                        }
                    }
                    if resp.secondary_clicked() {
                        unplug = Some((id, j as u8, is_out));
                    }
                    resp.on_hover_text(format!("{} {}: {}", info.name, if is_out { "output" } else { "input" }, names[j]));
                    jacks.push((id, j as u8, is_out, *c));
                }
            }
        }
        // Cables on top.
        let fg = ui.ctx().layer_painter(egui::LayerId::new(egui::Order::Foreground, Id::new(("rack_cables", track_id)))).with_clip_rect(ui.clip_rect());
        let at = |id: u32, port: u8, out: bool| jacks.iter().find(|j| j.0 == id && j.1 == port && j.2 == out).map(|j| j.3);
        for cb in &patch.cables {
            if let (Some(a), Some(b)) = (at(cb.from, cb.out, true), at(cb.to, cb.input, false)) {
                for s in cable_shape(a, b, CABLE_COLORS[cb.color as usize % CABLE_COLORS.len()]) {
                    fg.add(s);
                }
            }
        }
        let pointer = ui.input(|i| i.pointer.interact_pos());
        let released = ui.input(|i| i.pointer.any_released() || !i.pointer.any_down());
        if let Some((id, port, is_out)) = rack.cable {
            if let (Some(start), Some(pos)) = (at(id, port, is_out), pointer) {
                for s in cable_shape(start, pos, CABLE_COLORS[patch.cables.len() % CABLE_COLORS.len()]) {
                    fg.add(s);
                }
            }
            if released {
                if let Some(pos) = pointer {
                    if let Some(&(tid, tport, tout, _)) = jacks.iter().filter(|j| j.3.distance(pos) < JACK_R * 2.0).min_by(|a, b| a.3.distance(pos).total_cmp(&b.3.distance(pos))) {
                        match (is_out, tout) {
                            (true, false) => patch.connect(id, port, tid, tport),
                            (false, true) => patch.connect(tid, tport, id, port),
                            _ => {}
                        }
                    }
                }
                rack.cable = None;
            }
        }
        if let Some((id, grab)) = rack.moving {
            if let (Some(m), Some(pos)) = (patch.modules.iter().find(|m| m.id == id), pointer) {
                let hp = m.kind.info().hp;
                let x = (((pos.x - grab.x - origin.x) / HP).round().max(0.0) as u16).min(ROW_HP - hp);
                let row = ((pos.y - origin.y) / ROW_H).floor().max(0.0) as u16;
                let ghost = Rect::from_min_size(origin + vec2(x as f32 * HP, row as f32 * ROW_H + RAIL), vec2(hp as f32 * HP, ROW_H - 2.0 * RAIL));
                fg.rect_stroke(ghost, 3.0, Stroke::new(2.0_f32, ACCENT), egui::StrokeKind::Inside);
                if released {
                    let (fx, frow) = patch.free_spot(hp, row, x, Some(id));
                    if let Some(m) = patch.modules.iter_mut().find(|m| m.id == id) {
                        m.x = fx;
                        m.row = frow;
                    }
                    rack.moving = None;
                }
            } else {
                rack.moving = None;
            }
        }
        if let Some((id, port, is_out)) = unplug {
            patch.cables.retain(|c| if is_out { !(c.from == id && c.out == port) } else { !(c.to == id && c.input == port) });
        }
        if let Some(id) = duplicate {
            if let Some(m) = patch.module(id).cloned() {
                let new = patch.add(m.kind, m.row);
                if let Some(n) = patch.modules.iter_mut().find(|x| x.id == new) {
                    n.params = m.params;
                }
            }
        }
        if let Some(id) = remove {
            patch.remove(id);
        }
    });
}

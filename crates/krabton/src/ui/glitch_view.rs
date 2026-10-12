//! The Glitch Sequencer's face. A header with the master section (Amount,
//! Seed, De-Click, the grid, Step Envelope, Overdrive, Master Filter,
//! Output); a bar to pick the effect you paint with, randomise, templates
//! and the pattern bank; the lane, where you paint blocks of any length; and
//! a coloured column per module with its knobs, solo, on/off, Init and Rand
//! (and per slot, holding any other effect). It starts folded: the essentials
//! of the master section and only the column of the effect you paint with;
//! the strip on its right unfolds everything. With room enough the columns
//! sit under the lane; in a low panel they sit beside it. It also opens in a
//! window of its own.

use crate::app::{App, TrackSel};
use crate::theme::*;
use crate::ui::modulation::{knob_overlay, ModView};
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, Context, FontId, Id, Painter, Rect, Sense, Shape, Stroke, Ui};
use krabton_engine::dsp::effects::EffectKind;
use krabton_engine::dsp::fx::glitch::{self as gl, Randomise, MODULES, MODULE_PARAMS, SPECS};
use krabton_engine::engine::EngineStatus;
use krabton_engine::modulation::ModOwner;
use krabton_engine::params::{ParamKind, ParamSpec};
use krabton_engine::project::Device;

/// What the panel asks of its owner (it cannot make devices itself).
#[derive(Default)]
pub(crate) struct GlitchEdit {
    pub add: Option<EffectKind>,
    pub remove: Option<usize>,
    /// Open it in its own window.
    pub window: bool,
}

pub(crate) fn module_color(m: u8) -> Color32 {
    match m {
        1 => Color32::from_rgb(0xe5, 0x2d, 0x2d),
        2 => Color32::from_rgb(0xf2, 0x86, 0x1b),
        3 => Color32::from_rgb(0xf0, 0xd5, 0x1c),
        4 => Color32::from_rgb(0x4c, 0xcf, 0x3a),
        5 => Color32::from_rgb(0x2c, 0xd3, 0xd3),
        6 => Color32::from_rgb(0x2e, 0x8a, 0xee),
        7 => Color32::from_rgb(0x3d, 0x4f, 0xe0),
        8 => Color32::from_rgb(0x9b, 0x3d, 0xe6),
        9 => Color32::from_rgb(0xe6, 0x3c, 0xcc),
        10 => Color32::from_rgb(0xd9, 0xd9, 0xd9),
        11 => Color32::from_rgb(0xc8, 0xb4, 0x96),
        12 => Color32::from_rgb(0x96, 0xc8, 0xb0),
        13 => Color32::from_rgb(0xb0, 0xa0, 0xd0),
        _ => Color32::WHITE,
    }
}

/// Text on a module's colour.
fn ink(m: u8) -> Color32 {
    if matches!(m, 3..=5) || m >= gl::FIRST_SLOT || m == 0 {
        TEXT_DARK
    } else {
        Color32::WHITE
    }
}

fn short(m: u8) -> String {
    match m {
        0 => "R".into(),
        m if m >= gl::FIRST_SLOT => format!("S{}", m - gl::FIRST_SLOT + 1),
        m => m.to_string(),
    }
}

const HEAD_H: f32 = 76.0;
const BAR_H: f32 = 44.0;
const LEFT_W: f32 = 1052.0;
/// The left part when folded.
const LEFT_C: f32 = 846.0;
const STRIP_W: f32 = 18.0;
const SUB_W: f32 = 136.0;
const COL_HEAD: f32 = 44.0;
const ROW_H: f32 = 40.0;
const RING: f32 = 32.0;
const CELL: f32 = 56.0;
/// From this height on the columns sit under the lane.
const TALL: f32 = 330.0;

const BODY: Color32 = Color32::from_rgb(0x1b, 0x1b, 0x1d);

fn seed_now() -> u32 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.subsec_nanos()).unwrap_or(1) | 1
}

/// How a parameter reads in the panel.
fn format(k: usize, v: f32) -> String {
    match k {
        gl::SEED if v.round() == 0.0 => "Free".into(),
        gl::ENV_LENGTH => format!("{v:.2} steps"),
        gl::LENGTH => format!("{} steps", v.round() as i32),
        gl::MOD_SPREAD => format!("{v:.0}\u{b0}"),
        gl::SH_MIN | gl::SH_MAX => format!("{} step{}", v.round() as i32, if v.round() as i32 == 1 { "" } else { "s" }),
        _ => SPECS[k].format(v),
    }
}

/// A knob drawn as a thick ring. `hit` takes the mouse. Returns true when changed.
#[allow(clippy::too_many_arguments)]
fn ring(ui: &mut Ui, p: &Painter, knob: Rect, hit: Rect, id: Id, spec: &ParamSpec, value: &mut f32, color: Color32, hover: &str) -> bool {
    let resp = ui.interact(hit, id, Sense::click_and_drag());
    let discrete = spec.is_discrete();
    let steps = if discrete { (spec.max - spec.min).round().max(1.0) } else { 0.0 };
    let speed = if steps > 0.0 { (167.0 / (steps * 24.0)).max(1.0) } else { 1.0 };
    let k = if ui.input(|i| i.modifiers.shift) { 0.0015 } else { 0.006 } * speed;
    let d = -resp.drag_delta().y * k + resp.drag_delta().x * k * 0.5;
    let norm = spec.to_norm(*value);
    let mut changed = false;
    if let Some(n) = drag_position(ui, &resp, id, norm, d) {
        let v = spec.from_norm(n);
        if v != *value {
            *value = v;
            changed = true;
        }
    }
    let choice = matches!(spec.kind, ParamKind::Choice(_) | ParamKind::Toggle);
    if choice {
        // A click steps on, a right-click back.
        let n = (spec.max - spec.min).round() as i32 + 1;
        let cur = (*value - spec.min).round() as i32;
        let next = if resp.clicked() {
            Some((cur + 1) % n)
        } else if resp.secondary_clicked() {
            Some((cur + n - 1) % n)
        } else {
            None
        };
        if let Some(i) = next {
            *value = spec.min + i as f32;
            changed = true;
        }
    } else if resp.double_clicked() {
        *value = spec.default;
        changed = true;
    }
    let v = if discrete { spec.to_norm(*value) } else { shown_position(ui, &resp, id, spec.to_norm(*value)) };
    ring_face(p, knob, v, spec.min < 0.0 && spec.max > 0.0, color, resp.hovered() || resp.dragged());
    let tip = if choice { "click: next, right-click: back, or drag" } else { "drag (Shift: fine), double-click: reset" };
    resp.on_hover_text(format!("{hover}\n{tip}"));
    changed
}

/// A knob with its name and value beside it (the module columns).
#[allow(clippy::too_many_arguments)]
fn knob_row(ui: &mut Ui, p: &Painter, r: Rect, id: Id, spec: &ParamSpec, value: &mut f32, shown: String, color: Color32) -> (bool, Rect) {
    let knob = Rect::from_center_size(pos2(r.left() + 4.0 + RING * 0.5, r.center().y), vec2(RING, RING));
    let changed = ring(ui, p, knob, r, id, spec, value, color, &format!("{}: {}", spec.name, shown));
    let tx = knob.right() + 7.0;
    let clip = p.with_clip_rect(Rect::from_min_max(pos2(tx, r.top()), r.max).intersect(p.clip_rect()));
    clip.text(pos2(tx, r.center().y - 7.0), Align2::LEFT_CENTER, spec.name, small(), TEXT_DIM);
    clip.text(pos2(tx, r.center().y + 7.0), Align2::LEFT_CENTER, shown, small(), TEXT);
    (changed, knob)
}

/// A knob with its name above and value below (the header).
fn knob_cell(ui: &mut Ui, p: &Painter, r: Rect, id: Id, k: usize, value: &mut f32, color: Color32) -> (bool, Rect) {
    let spec = &SPECS[k];
    let name = spec.name.trim_start_matches("Env ").trim_start_matches("Drive ");
    p.text(pos2(r.center().x, r.top() + 6.0), Align2::CENTER_CENTER, name, small(), TEXT_DIM);
    let knob = Rect::from_center_size(pos2(r.center().x, r.top() + 13.0 + RING * 0.5 + 2.0), vec2(RING + 4.0, RING + 4.0));
    let shown = format(k, *value);
    let changed = ring(ui, p, knob, r, id, spec, value, color, &format!("{}: {}", spec.name, shown));
    p.text(pos2(r.center().x, r.bottom() - 6.0), Align2::CENTER_CENTER, format(k, *value), small(), TEXT);
    (changed, knob)
}

/// Columns: the nine modules, the slots in use and one to fill.
fn columns(children: &[Device]) -> Vec<u8> {
    let mut v: Vec<u8> = (1..=gl::BUILT_IN as u8).collect();
    for s in 0..(children.len() + 1).min(gl::SLOTS) {
        v.push(gl::FIRST_SLOT + s as u8);
    }
    v
}

/// The effects that can go in a slot, by category.
fn effect_menu(ui: &mut Ui, edit: &mut GlitchEdit) {
    for cat in EffectKind::CATEGORIES {
        let kinds: Vec<EffectKind> = EffectKind::ALL.into_iter().filter(|k| k.category() == cat && *k != EffectKind::Glitch && *k != EffectKind::Plugin).collect();
        if kinds.is_empty() {
            continue;
        }
        ui.menu_button(cat, |ui| {
            for k in kinds {
                if ui.button(k.name()).on_hover_text(k.description()).clicked() {
                    edit.add = Some(k);
                }
            }
        });
    }
}

fn knob_count(m: u8, children: &[Device]) -> usize {
    if (m as usize) <= gl::BUILT_IN {
        MODULE_PARAMS[m as usize - 1].len() - 1
    } else {
        children.get((m - gl::FIRST_SLOT) as usize).map(|d| d.params.len().min(d.kind.specs().len())).unwrap_or(0)
    }
}

fn column_width(m: u8, children: &[Device], col_h: f32) -> f32 {
    let rows = (((col_h - COL_HEAD - 6.0) / ROW_H).floor() as usize).max(1);
    knob_count(m, children).div_ceil(rows).max(1) as f32 * SUB_W
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn glitch_panel(ui: &mut Ui, id: Id, dev_id: u64, p: &mut [f32], data: &mut Vec<u8>, children: &mut [Device], h: f32, status: &EngineStatus, mut mv: Option<&mut ModView>, in_window: bool) -> (bool, GlitchEdit) {
    let mut changed = false;
    let mut edit = GlitchEdit::default();
    if p.len() < SPECS.len() {
        return (false, edit);
    }
    if !gl::is_current(data) {
        gl::migrate(data);
        changed = true;
    }
    let live = status.meters.read(dev_id).filter(|m| m[3] > 0.5);
    let pattern = (p[gl::PATTERN].round().max(0.0) as usize).min(gl::PATTERNS - 1);
    let length = p[gl::LENGTH].round().clamp(1.0, gl::STEPS as f32) as usize;
    let chosen_id = id.with("chosen");
    let mut chosen: u8 = ui.ctx().memory(|m| m.data.get_temp(chosen_id).unwrap_or(3));
    if chosen >= gl::FIRST_SLOT && (chosen - gl::FIRST_SLOT) as usize >= children.len() {
        chosen = 3;
    }
    // Folded (as it starts): fewer knobs and only the column of the effect
    // you paint with. Unfolded (and in its window): everything.
    let fold_id = id.with("unfolded");
    let unfolded = in_window || ui.ctx().memory(|m| m.data.get_temp(fold_id).unwrap_or(false));
    let shown_id = id.with("shown");
    let mut shown: u8 = ui.ctx().memory(|m| m.data.get_temp(shown_id).unwrap_or(3));
    if chosen != 0 {
        shown = chosen;
    }
    if shown == 0 || (shown >= gl::FIRST_SLOT && (shown - gl::FIRST_SLOT) as usize >= children.len()) {
        shown = 3;
    }
    let left_w = if unfolded { LEFT_W } else { LEFT_C };
    let strip = if in_window { 0.0 } else { STRIP_W + 4.0 };

    // Layout.
    let h = h.max(190.0);
    let tall = h >= TALL;
    let cols = if unfolded { columns(children) } else { vec![shown] };
    let col_h = if tall { (h - HEAD_H - BAR_H - 110.0).max(COL_HEAD + ROW_H + 6.0) } else { h };
    let widths: Vec<f32> = cols.iter().map(|&m| column_width(m, children, col_h)).collect();
    let cols_w: f32 = widths.iter().sum::<f32>() + 4.0 * cols.len() as f32;
    let total_w = if tall { left_w.max(cols_w) } else { left_w + 8.0 + cols_w } + strip;
    let (rect, _) = ui.allocate_exact_size(vec2(total_w, h), Sense::hover());
    let painter = ui.painter().with_clip_rect(rect.intersect(ui.clip_rect()));
    rect_fill(&painter, rect, 4.0, BODY);
    let inner = Rect::from_min_max(rect.min, pos2(rect.right() - strip, rect.bottom()));
    let left = Rect::from_min_size(inner.min, vec2(if tall { inner.width() } else { left_w }, if tall { h - col_h - 6.0 } else { h }));
    let head = Rect::from_min_size(left.min, vec2(left_w, HEAD_H));
    let bar = Rect::from_min_size(pos2(left.left(), head.bottom() + 2.0), vec2(left_w, BAR_H));
    let lane_box = Rect::from_min_max(pos2(left.left() + 6.0, bar.bottom() + 4.0), pos2(left.right() - 6.0, left.bottom() - 4.0));
    let cols_rect = if tall { Rect::from_min_max(pos2(inner.left(), inner.bottom() - col_h), inner.max) } else { Rect::from_min_max(pos2(left.right() + 8.0, inner.top()), inner.max) };

    // ------------------------------------------------------------ header
    let title = Rect::from_min_size(head.min, vec2(128.0, HEAD_H));
    painter.text(pos2(title.left() + 12.0, title.top() + 24.0), Align2::LEFT_CENTER, "Glitch", FontId::proportional(30.0), Color32::WHITE);
    painter.text(pos2(title.left() + 14.0, title.top() + 46.0), Align2::LEFT_CENTER, "SEQUENCER", FontId::proportional(10.5), TEXT_DIM);
    if !in_window {
        let b = Rect::from_min_size(pos2(title.left() + 12.0, title.bottom() - 20.0), vec2(96.0, 16.0));
        if button(ui, b, id.with("window"), "Open big", false, accent_now()).on_hover_text("Open it in a window of its own").clicked() {
            edit.window = true;
        }
    }
    let all: [(&str, &[usize]); 6] = [
        ("", &[gl::AMOUNT, gl::SEED, gl::DECLICK]),
        ("Grid", &[gl::STEP, gl::LENGTH]),
        ("Step Envelope", &[gl::ENV_LENGTH, gl::ENV_VOLUME]),
        ("Overdrive", &[gl::OD_MIX, gl::OD_GAIN]),
        ("Master Filter", &[gl::FILTER_MODE, gl::CUTOFF, gl::RESONANCE]),
        ("Output", &[gl::VARIATION, gl::MIX, gl::OUTPUT]),
    ];
    let groups: Vec<(&str, &[usize])> = if unfolded { all.to_vec() } else { vec![all[0], all[1], all[2], all[5]] };
    let mut x = title.right();
    for (gi, (name, ks)) in groups.iter().enumerate() {
        let w = ks.len() as f32 * CELL + 12.0;
        let g = Rect::from_min_size(pos2(x, head.top() + 4.0), vec2(w, HEAD_H - 8.0));
        if gi > 0 {
            painter.line_segment([pos2(x, g.top() + 2.0), pos2(x, g.bottom() - 2.0)], Stroke::new(1.0_f32, LINE));
        }
        let top = if name.is_empty() { g.top() } else { g.top() + 12.0 };
        if !name.is_empty() {
            painter.text(pos2(g.center().x, g.top() + 5.0), Align2::CENTER_CENTER, *name, FontId::proportional(12.0), Color32::WHITE);
        }
        for (i, &k) in ks.iter().enumerate() {
            let cell = Rect::from_min_max(pos2(g.left() + 6.0 + i as f32 * CELL, top), pos2(g.left() + 6.0 + (i + 1) as f32 * CELL, g.bottom()));
            if k == gl::FILTER_MODE {
                // Four small buttons rather than a knob.
                painter.text(pos2(cell.center().x, cell.top() + 6.0), Align2::CENTER_CENTER, "Mode", small(), TEXT_DIM);
                for (j, label) in ["Off", "LP", "HP", "BP"].iter().enumerate() {
                    let b = Rect::from_min_size(pos2(cell.left() + 2.0 + (j % 2) as f32 * 26.0, cell.top() + 14.0 + (j / 2) as f32 * 19.0), vec2(24.0, 17.0));
                    let on = p[k].round() as usize == j;
                    if button(ui, b, id.with(("fmode", j)), label, on, accent_now()).on_hover_text(gl::FILTER_MODES[j]).clicked() {
                        p[k] = j as f32;
                        changed = true;
                    }
                }
                continue;
            }
            let mut v = p[k];
            let (c, knob) = knob_cell(ui, &painter, cell, id.with(("h", k)), k, &mut v, accent_now());
            if c {
                p[k] = v;
                changed = true;
            }
            if let Some(mv) = mv.as_deref_mut() {
                knob_overlay(ui, cell, knob, id.with(("hm", k)), &SPECS[k], p[k], k, mv);
            }
        }
        x += w;
    }

    // ------------------------------------------------------------ bar
    let label_y = bar.top() + 7.0;
    let row = Rect::from_min_size(pos2(bar.left(), bar.top() + 15.0), vec2(bar.width(), 24.0));
    let mut x = bar.left() + 12.0;
    painter.text(pos2(x, label_y), Align2::LEFT_CENTER, "Paint with", FontId::proportional(12.0), Color32::WHITE);
    let palette: Vec<u8> = std::iter::once(0).chain(1..=gl::BUILT_IN as u8).chain((0..children.len()).map(|s| gl::FIRST_SLOT + s as u8)).collect();
    for &m in &palette {
        let w = if m >= gl::FIRST_SLOT { 28.0 } else { 22.0 };
        let b = Rect::from_min_size(pos2(x, row.top()), vec2(w, 22.0));
        let resp = ui.interact(b, id.with(("pal", m)), Sense::click());
        let col = if m == 0 { Color32::from_gray(235) } else { module_color(m) };
        rect_fill(&painter, b, 2.0, if resp.hovered() { mix(col, Color32::WHITE, 0.25) } else { col });
        painter.text(b.center(), Align2::CENTER_CENTER, short(m), FontId::proportional(12.5), ink(m));
        if m == chosen {
            rect_line(&painter, b.expand(2.0), 3.0, 2.0, Color32::WHITE);
        }
        let tip = match m {
            0 => "R: rest (erase): the music as it is".to_string(),
            m if m >= gl::FIRST_SLOT => format!("{}: {}", short(m), children.get((m - gl::FIRST_SLOT) as usize).map(|d| d.name().to_string()).unwrap_or_default()),
            m => MODULES[m as usize - 1].to_string(),
        };
        if resp.on_hover_text(format!("{tip}\nPick it, then paint blocks on the lane")).clicked() {
            chosen = m;
        }
        x += w + 3.0;
    }
    if children.len() < gl::SLOTS {
        let b = Rect::from_min_size(pos2(x, row.top()), vec2(22.0, 22.0));
        let resp = button(ui, b, id.with("pal_add"), "+", false, accent_now()).on_hover_text("Put another effect in a slot, to paint with it");
        egui::Popup::menu(&resp).show(|ui| effect_menu(ui, &mut edit));
        x += 25.0;
    }
    // The modules on, and the slots: what the dice pick from.
    let pool: Vec<u8> = (1..=gl::BUILT_IN as u8).filter(|m| p[MODULE_PARAMS[*m as usize - 1][0]] >= 0.5).chain((0..children.len()).map(|s| gl::FIRST_SLOT + s as u8)).collect();
    let template_id = id.with("template");
    let mut template: usize = ui.ctx().memory(|m| m.data.get_temp(template_id).unwrap_or(2));
    x += 18.0;
    painter.text(pos2(x, label_y), Align2::LEFT_CENTER, "Randomise", FontId::proportional(12.0), Color32::WHITE);
    for (label, what, tip) in [("Steps", Randomise::Steps, "New block lengths (from the template); the effects stay where they were"), ("FX", Randomise::Fx, "New effects in the blocks there are"), ("Both", Randomise::Both, "New blocks and effects")] {
        let b = Rect::from_min_size(pos2(x, row.top()), vec2(46.0, 22.0));
        if button(ui, b, id.with(("rnd", label)), label, false, accent_now()).on_hover_text(tip).clicked() {
            gl::randomise(data, pattern, length, what, template, &pool, seed_now());
            changed = true;
        }
        x += 49.0;
    }
    x += 14.0;
    if unfolded {
        painter.text(pos2(x, label_y), Align2::LEFT_CENTER, "Templates", FontId::proportional(12.0), Color32::WHITE);
    }
    for t in (0..gl::TEMPLATES.len()).filter(|_| unfolded) {
        let b = Rect::from_min_size(pos2(x, row.top()), vec2(22.0, 22.0));
        let tip = ["Even pairs of steps", "Blocks of four", "Mixed: one to six steps", "Long: four to sixteen steps"][t];
        if button(ui, b, id.with(("tpl", t)), &(t + 1).to_string(), t == template, SELECT).on_hover_text(format!("{tip}: cuts the lane in this rhythm (Randomise Steps uses it too)")).clicked() {
            template = t;
            gl::randomise(data, pattern, length, Randomise::Steps, t, &pool, seed_now());
            changed = true;
        }
        x += 25.0;
    }
    if unfolded {
        x += 14.0;
    }
    painter.text(pos2(x, label_y), Align2::LEFT_CENTER, "Pattern Bank", FontId::proportional(12.0), Color32::WHITE);
    for k in 0..gl::PATTERNS {
        let b = Rect::from_min_size(pos2(x, row.top()), vec2(22.0, 22.0));
        let resp = button(ui, b, id.with(("pat", k)), &(k + 1).to_string(), k == pattern, accent_now());
        if !gl::blocks(data, k, 64).is_empty() && k != pattern {
            painter.circle_filled(pos2(b.center().x, b.bottom() - 3.0), 1.5, TEXT_DIM);
        }
        if resp.clicked() {
            p[gl::PATTERN] = k as f32;
            changed = true;
        }
        resp.on_hover_text("Pattern (automate or modulate Pattern to switch); right-click to copy or clear").context_menu(|ui| {
            if k != pattern && ui.button(format!("Copy pattern {} here", pattern + 1)).clicked() {
                gl::copy_pattern(data, pattern, k);
                changed = true;
                ui.close();
            }
            if ui.button("Clear").clicked() {
                gl::clear(data, k);
                changed = true;
                ui.close();
            }
        });
        x += 25.0;
    }
    x += 14.0;
    let b = Rect::from_min_size(pos2(x, row.top()), vec2(48.0, 22.0));
    if button(ui, b, id.with("clear"), "Clear", false, accent_now()).on_hover_text("Clear this pattern").clicked() {
        gl::clear(data, pattern);
        changed = true;
    }
    ui.ctx().memory_mut(|m| m.data.insert_temp(template_id, template));

    // ------------------------------------------------------------ lane
    let ruler = Rect::from_min_size(lane_box.min, vec2(lane_box.width(), 12.0));
    let lane = Rect::from_min_max(pos2(lane_box.left(), ruler.bottom() + 1.0), lane_box.max);
    let cw = lane.width() / length as f32;
    rect_fill(&painter, lane, 3.0, Color32::from_rgb(0x0f, 0x0f, 0x11));
    let step_beats = [0.5, 0.25, 0.125, 0.0625][(p[gl::STEP].round().max(0.0) as usize).min(3)];
    let per_beat = (1.0f64 / step_beats).round() as usize;
    for i in 0..=length {
        let xx = lane.left() + i as f32 * cw;
        let beat = i % per_beat == 0;
        let bar_line = i % (per_beat * 4) == 0;
        painter.line_segment([pos2(xx, ruler.bottom() - if beat { 6.0 } else { 3.0 }), pos2(xx, ruler.bottom())], Stroke::new(1.0_f32, if beat { TEXT_DIM } else { LINE }));
        if i < length {
            painter.line_segment([pos2(xx, lane.top()), pos2(xx, lane.bottom())], Stroke::new(1.0_f32, if bar_line { LINE } else if beat { Color32::from_gray(0x2a) } else { Color32::from_gray(0x1c) }));
        }
        if beat && i < length && cw * per_beat as f32 > 14.0 {
            painter.text(pos2(xx + 2.0, ruler.top() + 4.0), Align2::LEFT_CENTER, format!("{}", i / per_beat + 1), FontId::proportional(9.5), TEXT_DIM);
        }
    }
    let blocks = gl::blocks(data, pattern, length);
    let now_block = live.map(|m| m[4] as i32).filter(|s| *s >= 0).map(|s| s as usize);
    let now_module = live.map(|m| m[1] as u8).unwrap_or(0);
    for b in &blocks {
        let r = Rect::from_min_max(pos2(lane.left() + b.start as f32 * cw + 1.0, lane.top() + 2.0), pos2(lane.left() + (b.start + b.len) as f32 * cw - 1.0, lane.bottom() - 2.0));
        let col = module_color(b.module);
        let playing = now_block == Some(b.start);
        let fired = playing && now_module == b.module;
        let c = if fired { mix(col, Color32::WHITE, 0.3) } else { col.gamma_multiply(0.55 + 0.45 * b.chance as f32 / 100.0) };
        rect_fill(&painter, r, 2.0, c);
        // A darker band at the bottom, like a shadow, and the chance as its height.
        let band = Rect::from_min_max(pos2(r.left(), r.bottom() - (r.height() * 0.22)), r.max);
        rect_fill(&painter, band, 2.0, Color32::from_black_alpha(50));
        if b.chance < 100 {
            let ch = Rect::from_min_max(pos2(r.left() + 2.0, r.bottom() - 4.0), pos2(r.left() + 2.0 + (r.width() - 4.0) * b.chance as f32 / 100.0, r.bottom() - 2.0));
            rect_fill(&painter, ch, 1.0, ink(b.module).gamma_multiply(0.7));
        }
        let pc = painter.with_clip_rect(r.intersect(painter.clip_rect()));
        pc.text(pos2(r.left() + 4.0, r.top() + 9.0), Align2::LEFT_CENTER, short(b.module), FontId::proportional(13.0), ink(b.module));
        if b.chance < 100 && r.width() > 34.0 {
            pc.text(pos2(r.left() + 4.0, r.bottom() - 12.0), Align2::LEFT_CENTER, format!("{}%", b.chance), FontId::proportional(9.5), ink(b.module));
        }
        if playing {
            rect_line(&painter, r, 2.0, if fired { 2.0 } else { 1.0 }, if fired { Color32::WHITE } else { TEXT_DIM });
        }
    }
    if let Some(m) = live {
        // The playhead.
        let at = match now_block.and_then(|s| blocks.iter().find(|b| b.start == s)) {
            Some(b) => b.start as f32 + m[2].clamp(0.0, 1.0) * b.len as f32,
            None => m[0].max(0.0) + 0.5,
        };
        let xx = lane.left() + at * cw;
        painter.line_segment([pos2(xx, ruler.top()), pos2(xx, lane.bottom())], Stroke::new(1.5_f32, Color32::WHITE));
        ui.ctx().request_repaint();
    }
    // Painting: press and drag over the steps; what you drag over becomes one block.
    let resp = ui.interact(lane, id.with("lane"), Sense::click_and_drag());
    let cell_at = |x: f32| (((x - lane.left()) / cw).floor() as i32).clamp(0, length as i32 - 1) as usize;
    let drag_id = id.with("lane_drag");
    if let Some(pos) = resp.interact_pointer_pos() {
        let i = cell_at(pos.x);
        if resp.drag_started() || (resp.clicked() && !resp.dragged()) {
            ui.ctx().memory_mut(|m| m.data.insert_temp(drag_id, (i, data.clone())));
        }
        if resp.dragged() || resp.drag_started() || resp.clicked() {
            if let Some((a, before)) = ui.ctx().memory(|m| m.data.get_temp::<(usize, Vec<u8>)>(drag_id)) {
                let mut d = before;
                gl::paint(&mut d, pattern, a, i, chosen);
                if d != *data {
                    *data = d;
                    changed = true;
                }
            }
        }
        if resp.drag_stopped() || resp.clicked() {
            ui.ctx().memory_mut(|m| m.data.remove::<(usize, Vec<u8>)>(drag_id));
        }
    }
    // Right-click a block: its chance, erase it, or make it the effect you paint with.
    let menu_id = id.with("lane_menu_block");
    if resp.secondary_clicked() {
        if let Some(pos) = resp.interact_pointer_pos().or(resp.hover_pos()) {
            let i = cell_at(pos.x);
            ui.ctx().memory_mut(|m| m.data.insert_temp(menu_id, gl::block_at(data, pattern, length, i)));
        }
    }
    let hovered_block = resp.hover_pos().and_then(|pos| gl::block_at(data, pattern, length, cell_at(pos.x)));
    let tip = match hovered_block {
        Some(b) => format!("{} · {} step{} · chance {}%\nDrag: paint with the chosen effect · right-click: chance, erase", if b.module >= gl::FIRST_SLOT { short(b.module) } else { MODULES[b.module as usize - 1].to_string() }, b.len, if b.len == 1 { "" } else { "s" }, b.chance),
        None => "Drag over the steps to paint a block with the chosen effect (R erases)".into(),
    };
    resp.on_hover_text(tip).context_menu(|ui| {
        let Some(b) = ui.ctx().memory(|m| m.data.get_temp::<Option<gl::Block>>(menu_id)).flatten() else {
            ui.label("No block here");
            return;
        };
        ui.label(egui::RichText::new("Chance").small().color(TEXT_DIM));
        ui.horizontal(|ui| {
            for c in [100u8, 75, 50, 25, 10] {
                if ui.selectable_label(b.chance == c, format!("{c}%")).clicked() {
                    gl::set_chance(data, pattern, b.start, c);
                    changed = true;
                    ui.close();
                }
            }
        });
        ui.separator();
        if chosen != 0 && chosen != b.module && ui.button(format!("Make it {}", if chosen >= gl::FIRST_SLOT { short(chosen) } else { MODULES[chosen as usize - 1].to_string() })).clicked() {
            gl::paint(data, pattern, b.start, b.start + b.len - 1, chosen);
            gl::set_chance(data, pattern, b.start, b.chance);
            changed = true;
            ui.close();
        }
        if ui.button("Erase").clicked() {
            gl::paint(data, pattern, b.start, b.start + b.len - 1, 0);
            changed = true;
            ui.close();
        }
    });

    // ------------------------------------------------------------ module columns
    let solo = p[gl::SOLO].round().max(0.0) as u8;
    let mut x = cols_rect.left() + 2.0;
    for (ci, &m) in cols.iter().enumerate() {
        let w = widths[ci];
        let col = Rect::from_min_size(pos2(x, cols_rect.top() + 2.0), vec2(w, cols_rect.height() - 4.0));
        x += w + 4.0;
        let color = module_color(m);
        let slot = (m >= gl::FIRST_SLOT).then(|| (m - gl::FIRST_SLOT) as usize);
        let filled = slot.map(|s| s < children.len()).unwrap_or(true);
        let on = match slot {
            None => p[MODULE_PARAMS[m as usize - 1][0]] >= 0.5,
            Some(s) => children.get(s).map(|d| d.enabled).unwrap_or(false),
        };
        let active = now_module == m && live.is_some();
        let hr = Rect::from_min_size(col.min, vec2(w, COL_HEAD));
        rect_fill(&painter, col, 3.0, Color32::from_rgb(0x15, 0x15, 0x17));
        let head_col = if !filled {
            Color32::from_gray(0x2c)
        } else if !on {
            color.gamma_multiply(0.35)
        } else if active {
            mix(color, Color32::WHITE, 0.25)
        } else {
            color
        };
        rect_fill(&painter, hr, 3.0, head_col);
        if m == chosen {
            rect_line(&painter, col, 3.0, 1.5, Color32::WHITE);
        }
        let ink_c = if filled && on { ink(m) } else { TEXT };
        painter.text(pos2(hr.left() + 7.0, hr.center().y), Align2::LEFT_CENTER, short(m), FontId::proportional(26.0), ink_c);
        let name = match slot {
            None => MODULES[m as usize - 1].to_string(),
            Some(s) => children.get(s).map(|d| d.name().to_string()).unwrap_or_else(|| format!("Slot {}", s + 1)),
        };
        let nx = hr.left() + if m >= gl::FIRST_SLOT { 40.0 } else { 30.0 };
        painter.with_clip_rect(Rect::from_min_max(pos2(nx, hr.top()), pos2(hr.right() - 2.0, hr.bottom())).intersect(painter.clip_rect())).text(pos2(nx, hr.top() + 11.0), Align2::LEFT_CENTER, &name, FontId::proportional(13.0), ink_c);
        // Clicking the name picks it to paint with.
        let pick = Rect::from_min_max(hr.min, pos2(nx + 70.0, hr.top() + 20.0));
        if filled && ui.interact(pick, id.with(("pick", m)), Sense::click()).on_hover_text("Click: paint with this").clicked() {
            chosen = m;
        }
        let by = hr.bottom() - 19.0;
        match slot {
            None => {
                let mut bx = nx;
                let k_on = MODULE_PARAMS[m as usize - 1][0];
                let b = Rect::from_min_size(pos2(bx, by), vec2(25.0, 16.0));
                if button(ui, b, id.with(("on", m)), if on { "On" } else { "Off" }, on, Color32::from_gray(30)).on_hover_text("This module on or off (off: its blocks play the music as it is)").clicked() {
                    p[k_on] = if on { 0.0 } else { 1.0 };
                    changed = true;
                }
                bx += 27.0;
                let b = Rect::from_min_size(pos2(bx, by), vec2(17.0, 16.0));
                if button(ui, b, id.with(("solo", m)), "S", solo == m, SOLO).on_hover_text("Solo: only this module plays its blocks").clicked() {
                    p[gl::SOLO] = if solo == m { 0.0 } else { m as f32 };
                    changed = true;
                }
                bx += 19.0;
                let b = Rect::from_min_size(pos2(bx, by), vec2(28.0, 16.0));
                if button(ui, b, id.with(("init", m)), "Init", false, accent_now()).on_hover_text("Its knobs back to how they start").clicked() {
                    for &k in &MODULE_PARAMS[m as usize - 1][1..] {
                        p[k] = SPECS[k].default;
                    }
                    changed = true;
                }
                bx += 30.0;
                let b = Rect::from_min_size(pos2(bx, by), vec2(31.0, 16.0));
                if button(ui, b, id.with(("rand", m)), "Rand", false, accent_now()).on_hover_text("Its knobs at random").clicked() {
                    let mut s = seed_now();
                    for &k in &MODULE_PARAMS[m as usize - 1][1..] {
                        s ^= s << 13;
                        s ^= s >> 17;
                        s ^= s << 5;
                        p[k] = SPECS[k].from_norm((s >> 8) as f32 / 16_777_216.0);
                    }
                    changed = true;
                }
            }
            Some(s) if filled => {
                let b = Rect::from_min_size(pos2(nx, by), vec2(26.0, 16.0));
                if button(ui, b, id.with(("son", s)), if on { "On" } else { "Off" }, on, Color32::from_gray(30)).on_hover_text("The effect on or off").clicked() {
                    if let Some(d) = children.get_mut(s) {
                        d.enabled = !on;
                        changed = true;
                    }
                }
                let b = Rect::from_min_size(pos2(nx + 28.0, by), vec2(18.0, 16.0));
                if button(ui, b, id.with(("sx", s)), "x", false, REC).on_hover_text("Take it out (its blocks play the music as it is)").clicked() {
                    edit.remove = Some(s);
                }
            }
            Some(s) => {
                let b = Rect::from_min_size(pos2(nx, by - 2.0), vec2(80.0, 18.0));
                let resp = button(ui, b, id.with(("sadd", s)), "+ Effect", false, accent_now()).on_hover_text("Put any effect in this slot; it plays on the blocks you paint with it");
                egui::Popup::menu(&resp).show(|ui| effect_menu(ui, &mut edit));
            }
        }
        // The knobs.
        let rows = (((col.height() - COL_HEAD - 6.0) / ROW_H).floor() as usize).max(1);
        let body = Rect::from_min_max(pos2(col.left(), hr.bottom() + 4.0), col.max);
        match slot {
            None => {
                for (j, &k) in MODULE_PARAMS[m as usize - 1][1..].iter().enumerate() {
                    let r = Rect::from_min_size(pos2(body.left() + (j / rows) as f32 * SUB_W, body.top() + (j % rows) as f32 * ROW_H), vec2(SUB_W - 2.0, ROW_H - 2.0));
                    let mut v = p[k];
                    let shown = format(k, v);
                    let (c, knob) = knob_row(ui, &painter, r, id.with(("k", k)), &SPECS[k], &mut v, shown, color);
                    if c {
                        p[k] = v;
                        changed = true;
                    }
                    if let Some(mv) = mv.as_deref_mut() {
                        knob_overlay(ui, r, knob, id.with(("km", k)), &SPECS[k], p[k], k, mv);
                    }
                }
            }
            Some(s) => {
                if let Some(dev) = children.get_mut(s) {
                    let specs = dev.kind.specs();
                    for j in 0..specs.len().min(dev.params.len()) {
                        let r = Rect::from_min_size(pos2(body.left() + (j / rows) as f32 * SUB_W, body.top() + (j % rows) as f32 * ROW_H), vec2(SUB_W - 2.0, ROW_H - 2.0));
                        let shown = specs[j].format(dev.params[j]);
                        let (c, _) = knob_row(ui, &painter, r, id.with(("slot", s, j)), &specs[j], &mut dev.params[j], shown, color);
                        changed |= c;
                    }
                }
            }
        }
    }
    // The strip that folds and unfolds it.
    if !in_window {
        let s = Rect::from_min_max(pos2(rect.right() - STRIP_W, rect.top()), rect.max);
        let resp = ui.interact(s, id.with("fold"), Sense::click());
        let hot = resp.hovered();
        rect_fill(&painter, s, 3.0, if hot { PANEL3 } else { Color32::from_rgb(0x24, 0x24, 0x28) });
        let c = s.center();
        let d = if unfolded { -1.0 } else { 1.0 };
        let tri = vec![c + vec2(-3.5 * d, -7.0), c + vec2(4.0 * d, 0.0), c + vec2(-3.5 * d, 7.0)];
        painter.add(Shape::convex_polygon(tri, if hot { Color32::WHITE } else { accent_now() }, Stroke::NONE));
        let tip = if unfolded { "Fold: the essentials, and only the column of the effect you paint with" } else { "Unfold: every module's column and the whole master section" };
        if resp.on_hover_text(tip).clicked() {
            ui.ctx().memory_mut(|m| m.data.insert_temp(fold_id, !unfolded));
        }
    }
    if chosen != 0 {
        shown = chosen;
    }
    ui.ctx().memory_mut(|m| {
        m.data.insert_temp(chosen_id, chosen);
        m.data.insert_temp(shown_id, shown);
    });
    (changed, edit)
}

impl App {
    /// The track chain and position of a device, by its id.
    fn find_device(&self, dev_id: u64) -> Option<(TrackSel, usize, String)> {
        for (i, t) in self.project.tracks.iter().enumerate() {
            if let Some(d) = t.devices.iter().position(|d| d.id == dev_id) {
                return Some((TrackSel::Track(i), d, t.name.clone()));
            }
        }
        for (j, t) in self.project.returns.iter().enumerate() {
            if let Some(d) = t.devices.iter().position(|d| d.id == dev_id) {
                return Some((TrackSel::Return(j), d, t.name.clone()));
            }
        }
        self.project.master.devices.iter().position(|d| d.id == dev_id).map(|d| (TrackSel::Master, d, "Master".to_string()))
    }

    /// A slot filled or emptied. True when something changed.
    pub(crate) fn apply_glitch_edit(&mut self, children: &mut Vec<Device>, data: &mut Vec<u8>, edit: &GlitchEdit) -> bool {
        let mut changed = false;
        if let Some(k) = edit.add {
            if children.len() < gl::SLOTS {
                children.push(self.project.make_device(k));
                changed = true;
            }
        }
        if let Some(s) = edit.remove.filter(|s| *s < children.len()) {
            children.remove(s);
            gl::remove_slot(data, s);
            changed = true;
        }
        changed
    }

    /// The Glitch Sequencer opened big, in a window of its own.
    pub(crate) fn ui_glitch_window(&mut self, ctx: &Context) {
        let Some(dev_id) = self.glitch_window else { return };
        let Some((sel, d, track)) = self.find_device(dev_id) else {
            self.glitch_window = None;
            return;
        };
        let _accent = AccentScope::new(self.track_accent(sel));
        let Some(dev) = self.chain_mut(sel).and_then(|c| c.get_mut(d)) else { return };
        let (mut params, mut data, mut children) = (dev.params.clone(), dev.data.clone(), dev.children.clone());
        let mut mv = self.mod_view(ModOwner::Device(dev_id));
        let status = self.status();
        let mut open = true;
        let mut out = (false, GlitchEdit::default());
        let wid = Id::new(("glitch_window", dev_id));
        let screen = ctx.screen_rect();
        egui::Window::new(format!("Glitch Sequencer - {track}"))
            .id(wid)
            .open(&mut open)
            .default_size([(screen.width() - 40.0).min(1440.0), (screen.height() - 80.0).min(680.0)])
            .min_size([600.0, 420.0])
            .resizable(true)
            .collapsible(true)
            .show(ctx, |ui| {
                let h = ui.available_height().max(420.0);
                egui::ScrollArea::horizontal().show(ui, |ui| {
                    out = glitch_panel(ui, wid.with("panel"), dev_id, &mut params, &mut data, &mut children, h, &status, mv.as_mut(), true);
                });
            });
        self.apply_mod_view(mv);
        let (mut changed, edit) = out;
        changed |= self.apply_glitch_edit(&mut children, &mut data, &edit);
        if changed {
            if let Some(dev) = self.chain_mut(sel).and_then(|c| c.get_mut(d)) {
                dev.params = params;
                dev.data = data;
                dev.children = children;
            }
        }
        if !open {
            self.glitch_window = None;
        }
    }
}

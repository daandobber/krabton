//! The Glitch Sequencer's face: the pattern as a grid (a row per module, a
//! column per step: click or drag to set steps, drag the bars below for
//! each step's chance), the patterns A to D, dice and clear; the settings of
//! the module you pick (click its name) and of the sequencer; and the four
//! slots, each holding any other effect with its knobs.

use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, FontId, Id, Rect, RichText, Sense, Stroke, Ui};
use krabton_engine::dsp::effects::EffectKind;
use krabton_engine::dsp::fx::glitch::{self as gl, MODULES, SPECS};
use krabton_engine::engine::EngineStatus;
use krabton_engine::project::Device;

/// What the panel asks of its owner (it cannot make devices itself).
#[derive(Default)]
pub(crate) struct GlitchEdit {
    pub add: Option<EffectKind>,
    pub remove: Option<usize>,
}

pub(crate) fn module_color(m: u8) -> Color32 {
    match m {
        1 => Color32::from_rgb(0xff, 0x9f, 0x43),
        2 => Color32::from_rgb(0x5e, 0xc8, 0xff),
        3 => Color32::from_rgb(0xc0, 0x84, 0xfc),
        4 => Color32::from_rgb(0xff, 0x6b, 0x6b),
        5 => Color32::from_rgb(0x4c, 0xd9, 0x9a),
        6 => Color32::from_rgb(0xff, 0xd4, 0x3b),
        7 => Color32::from_rgb(0x9a, 0xa5, 0xb8),
        8 => Color32::from_rgb(0x48, 0xe0, 0xd0),
        9 => Color32::from_rgb(0xf0, 0x7a, 0xc0),
        _ => Color32::from_rgb(0xe8, 0xe8, 0xe8),
    }
}

/// The params each module has (indices into SPECS).
fn module_params(m: u8) -> &'static [usize] {
    match m {
        1 => &[gl::REPEAT, gl::PITCH, gl::DECAY],
        2 => &[gl::FADE],
        3 => &[gl::RANGE],
        4 => &[gl::TAPE, gl::CURVE],
        5 => &[gl::SPEED],
        6 => &[gl::BITS, gl::DOWNSAMPLE],
        7 => &[gl::GATE_RATE, gl::DUTY],
        8 => &[gl::FILTER, gl::FROM, gl::TO, gl::RESONANCE],
        9 => &[gl::TIME, gl::FEEDBACK],
        _ => &[],
    }
}

const GLOBAL: [usize; 6] = [gl::STEP, gl::LENGTH, gl::VARIATION, gl::CHANCE, gl::MIX, gl::OUTPUT];
const CELL: f32 = 54.0;

#[allow(clippy::too_many_arguments)]
pub(crate) fn glitch_panel(ui: &mut Ui, id: Id, dev_id: u64, p: &mut [f32], data: &mut Vec<u8>, children: &mut [Device], accent: Color32, h: f32, status: &EngineStatus) -> (bool, GlitchEdit) {
    let mut changed = false;
    let mut edit = GlitchEdit::default();
    if data.len() < gl::DATA_BYTES {
        data.resize(gl::DATA_BYTES, 0);
        changed = true;
    }
    let live = status.meters.read(dev_id).filter(|m| m[3] > 0.5);
    let pattern = (p[gl::PATTERN].round().max(0.0) as usize).min(gl::PATTERNS - 1);
    let length = p[gl::LENGTH].round().clamp(1.0, gl::STEPS as f32) as usize;
    let sel_id = id.with("module");
    let mut selected: u8 = ui.ctx().memory(|m| m.data.get_temp(sel_id).unwrap_or(1));
    ui.horizontal_top(|ui| {
        // ------------------------------------------------ the pattern grid
        let rows: Vec<u8> = (1..=gl::BUILT_IN as u8).chain((0..children.len().min(gl::SLOTS)).map(|s| gl::FIRST_SLOT + s as u8)).collect();
        let label_w = 74.0;
        let cw = 13.0;
        let head = 22.0;
        let chance_h = 16.0;
        let rh = ((h - head - chance_h - 30.0) / rows.len() as f32).clamp(9.0, 18.0);
        // 16 columns, or 32 once the pattern is longer.
        let shown = if length > 16 { gl::STEPS } else { 16 };
        let w = label_w + shown as f32 * cw + 4.0;
        let grid_h = head + rh * rows.len() as f32 + chance_h + 6.0;
        let (rect, _) = ui.allocate_exact_size(vec2(w, grid_h), Sense::hover());
        let painter = ui.painter().clone();
        rect_fill(&painter, rect, 3.0, KNOB);
        // Patterns, dice, clear.
        for (k, name) in gl::PATTERN_NAMES.iter().enumerate() {
            let b = Rect::from_min_size(pos2(rect.left() + 4.0 + k as f32 * 24.0, rect.top() + 3.0), vec2(22.0, 16.0));
            if button(ui, b, id.with(("pat", k)), name, k == pattern, accent).on_hover_text("Pattern (automate or modulate Pattern to switch)").clicked() {
                p[gl::PATTERN] = k as f32;
                changed = true;
            }
        }
        let dice = Rect::from_min_size(pos2(rect.left() + 104.0, rect.top() + 3.0), vec2(44.0, 16.0));
        if button(ui, dice, id.with("dice"), "dice", false, accent).on_hover_text("A new random pattern").clicked() {
            let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.subsec_nanos()).unwrap_or(1) | 1;
            gl::random_pattern(data, pattern, length, 0.45, children.len(), seed);
            changed = true;
        }
        let clear = Rect::from_min_size(pos2(dice.right() + 4.0, rect.top() + 3.0), vec2(44.0, 16.0));
        if button(ui, clear, id.with("clear"), "clear", false, accent).on_hover_text("Clear this pattern").clicked() {
            for i in 0..gl::STEPS {
                gl::set_step(data, pattern, i, 0, 100);
            }
            changed = true;
        }
        let grid = Rect::from_min_size(pos2(rect.left() + label_w, rect.top() + head), vec2(shown as f32 * cw, rh * rows.len() as f32));
        let now_step = live.map(|m| m[0] as i32).filter(|s| *s >= 0);
        let now_module = live.map(|m| m[1] as u8).unwrap_or(0);
        // Row names: click to show that module's settings.
        for (ri, &m) in rows.iter().enumerate() {
            let r = Rect::from_min_size(pos2(rect.left() + 2.0, grid.top() + ri as f32 * rh), vec2(label_w - 4.0, rh));
            let resp = ui.interact(r, id.with(("row", m)), Sense::click());
            let on = selected == m;
            if on {
                rect_fill(&painter, r, 2.0, mix(KNOB, module_color(m), 0.25));
            }
            let name = if m >= gl::FIRST_SLOT { children.get((m - gl::FIRST_SLOT) as usize).map(|d| d.name().to_string()).unwrap_or_default() } else { MODULES[m as usize - 1].to_string() };
            let playing = now_module == m;
            painter.text(pos2(r.left() + 4.0, r.center().y), Align2::LEFT_CENTER, name, FontId::proportional((rh * 0.7).clamp(9.0, 12.0)), if playing { Color32::WHITE } else if on { module_color(m) } else { TEXT_DIM });
            if resp.on_hover_text("Click: its settings").clicked() {
                selected = m;
            }
        }
        // The cells.
        for i in 0..shown {
            let x = grid.left() + i as f32 * cw;
            let (m, chance) = gl::step_at(data, pattern, i);
            let inside = i < length;
            if i % 4 == 0 {
                painter.line_segment([pos2(x, grid.top()), pos2(x, grid.bottom() + chance_h + 2.0)], Stroke::new(1.0_f32, if i % 16 == 0 { LINE } else { LINE_SOFT }));
            }
            let here = now_step == Some(i as i32);
            if here {
                rect_fill(&painter, Rect::from_min_size(pos2(x, grid.top()), vec2(cw, grid.height())), 0.0, Color32::from_white_alpha(18));
            }
            for (ri, &rm) in rows.iter().enumerate() {
                let c = Rect::from_min_size(pos2(x + 1.0, grid.top() + ri as f32 * rh + 1.0), vec2(cw - 2.0, rh - 2.0));
                if m == rm {
                    let col = module_color(rm).gamma_multiply(if inside { 0.35 + 0.65 * chance as f32 / 100.0 } else { 0.25 });
                    rect_fill(&painter, c, 2.0, if here && now_module == rm { Color32::WHITE } else { col });
                } else if inside {
                    rect_fill(&painter, c, 2.0, Color32::from_white_alpha(8));
                }
            }
            // Chance below.
            let cb = Rect::from_min_size(pos2(x + 2.0, grid.bottom() + 3.0), vec2(cw - 4.0, chance_h));
            if m != 0 {
                let ch = cb.height() * chance as f32 / 100.0;
                rect_fill(&painter, Rect::from_min_max(pos2(cb.left(), cb.bottom() - ch), cb.right_bottom()), 1.0, module_color(m).gamma_multiply(0.7));
            }
        }
        if length < shown {
            let x = grid.left() + length as f32 * cw;
            rect_fill(&painter, Rect::from_min_max(pos2(x, grid.top()), pos2(grid.right(), grid.bottom() + chance_h + 2.0)), 0.0, Color32::from_black_alpha(90));
        }
        // Painting: press on a cell sets (or clears) it, dragging carries on.
        let paint = ui.interact(grid, id.with("grid"), Sense::click_and_drag());
        let paint_id = id.with("paint");
        if let Some(pos) = paint.interact_pointer_pos() {
            let i = (((pos.x - grid.left()) / cw).floor() as i32).clamp(0, shown as i32 - 1) as usize;
            let ri = (((pos.y - grid.top()) / rh).floor() as i32).clamp(0, rows.len() as i32 - 1) as usize;
            let rm = rows[ri];
            if paint.drag_started() || paint.clicked() {
                let (m, _) = gl::step_at(data, pattern, i);
                let v = if m == rm { 0u8 } else { rm };
                ui.ctx().memory_mut(|mm| mm.data.insert_temp(paint_id, v));
            }
            if paint.dragged() || paint.clicked() || paint.drag_started() {
                // What the press set (this module, or nothing), along the drag.
                let v: u8 = ui.ctx().memory(|mm| mm.data.get_temp(paint_id).unwrap_or(rm));
                let (m, chance) = gl::step_at(data, pattern, i);
                if m != v {
                    gl::set_step(data, pattern, i, v, if chance == 0 { 100 } else { chance });
                    changed = true;
                }
            }
        }
        paint.on_hover_text("Click or drag: set steps to this module (again to clear)");
        // Chance bars: drag up and down.
        let crect = Rect::from_min_size(pos2(grid.left(), grid.bottom() + 3.0), vec2(grid.width(), chance_h));
        let cresp = ui.interact(crect, id.with("chance"), Sense::click_and_drag());
        if let Some(pos) = cresp.interact_pointer_pos().filter(|_| cresp.dragged() || cresp.clicked()) {
            let i = (((pos.x - grid.left()) / cw).floor() as i32).clamp(0, shown as i32 - 1) as usize;
            let v = (((crect.bottom() - pos.y) / crect.height()).clamp(0.0, 1.0) * 100.0).round() as u8;
            let (m, chance) = gl::step_at(data, pattern, i);
            if m != 0 && chance != v.max(5) {
                gl::set_step(data, pattern, i, m, v.max(5));
                changed = true;
            }
        }
        cresp.on_hover_text("Drag up or down: the chance this step plays");
        painter.text(pos2(rect.left() + 4.0, crect.center().y), Align2::LEFT_CENTER, "chance", small(), TEXT_DIM);
        if live.is_some() {
            ui.ctx().request_repaint();
        }

        ui.add_space(6.0);
        // ------------------------------------------------ settings
        ui.vertical(|ui| {
            ui.label(RichText::new("Sequencer").small().color(TEXT_DIM));
            ui.horizontal(|ui| {
                for &k in GLOBAL.iter() {
                    let (r, _) = ui.allocate_exact_size(vec2(CELL, CELL), Sense::hover());
                    changed |= param_cell(ui, r, id.with(("g", k)), &SPECS[k], &mut p[k], accent);
                }
            });
            if selected <= gl::BUILT_IN as u8 {
                ui.label(RichText::new(MODULES[selected.max(1) as usize - 1]).small().color(module_color(selected)));
                ui.horizontal(|ui| {
                    for &k in module_params(selected) {
                        let (r, _) = ui.allocate_exact_size(vec2(CELL, CELL), Sense::hover());
                        changed |= param_cell(ui, r, id.with(("m", k)), &SPECS[k], &mut p[k], module_color(selected));
                    }
                });
            } else {
                ui.label(RichText::new("Its knobs are in its slot, on the right").small().color(TEXT_DIM));
            }
        });

        ui.add_space(6.0);
        // ------------------------------------------------ the slots
        for s in 0..gl::SLOTS {
            if s > children.len() {
                break;
            }
            ui.vertical(|ui| {
                ui.set_min_width(110.0);
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                let m = gl::FIRST_SLOT + s as u8;
                let count = children.len();
                match children.get_mut(s) {
                    Some(dev) => {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(format!("Slot {}", s + 1)).small().color(module_color(m)));
                            ui.label(RichText::new(dev.name()).strong().color(TEXT));
                            let mut on = dev.enabled;
                            if ui.checkbox(&mut on, "").on_hover_text("On/off").changed() {
                                dev.enabled = on;
                                changed = true;
                            }
                            if ui.small_button("x").on_hover_text("Take it out (its steps play the music as it is)").clicked() {
                                edit.remove = Some(s);
                            }
                        });
                        let specs = dev.kind.specs();
                        let rows = (((h - 40.0) / CELL).floor() as usize).max(1);
                        let cols = specs.len().div_ceil(rows).max(1);
                        ui.horizontal_top(|ui| {
                            for c in 0..cols {
                                ui.vertical(|ui| {
                                    for rr in 0..rows {
                                        let k = c * rows + rr;
                                        if k < specs.len() && k < dev.params.len() {
                                            let (r, _) = ui.allocate_exact_size(vec2(CELL, CELL), Sense::hover());
                                            changed |= param_cell(ui, r, id.with(("slot", s, k)), &specs[k], &mut dev.params[k], module_color(m));
                                        }
                                    }
                                });
                            }
                        });
                    }
                    None if s == count => {
                        ui.label(RichText::new(format!("Slot {}", s + 1)).small().color(module_color(m)));
                        let resp = ui.add(egui::Button::new(RichText::new("+ Effect").color(TEXT)).fill(PANEL3)).on_hover_text("Put an effect in this slot; it plays on the steps you give its row");
                        egui::Popup::menu(&resp).show(|ui| {
                            for cat in EffectKind::CATEGORIES {
                                let kinds: Vec<EffectKind> = EffectKind::ALL.into_iter().filter(|k| k.category() == cat && *k != EffectKind::Glitch).collect();
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
                        });
                    }
                    None => {}
                }
            });
            if s < children.len() {
                ui.add_space(6.0);
            }
        }
    });
    ui.ctx().memory_mut(|m| m.data.insert_temp(sel_id, selected));
    (changed, edit)
}

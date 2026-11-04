//! Munich's face: a black panel between walnut cheeks, read from left to right
//! as the signal goes (oscillators, mixer, filter, contours, modulation, keys,
//! echo). Knobs for what turns, a click for ranges and the keyboard's follow
//! (click: the next, right-click: the one before), dropdowns for wave, source
//! and echo time. The cutoff shows where the contour takes it.

use super::inst_screen::InstLive;
use super::modulation::{cell_overlay, ModView};
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, FontId, Id, Rect, RichText, Sense, Stroke, Ui};
use krabton_engine::dsp::munich as m;
use krabton_engine::params::ParamKind;

const CELL: f32 = 44.0;
const GAP: f32 = 12.0;

/// A choice clicked through: its name above, its value on the button.
fn clicker(ui: &mut Ui, r: Rect, id: Id, k: usize, params: &mut [f32], accent: Color32) -> bool {
    let spec = &m::SPECS[k];
    let ParamKind::Choice(opts) = spec.kind else { return false };
    let p = ui.painter().clone();
    let (head, body) = r.split_top_bottom_at_y(r.top() + 12.0);
    p.with_clip_rect(head).text(head.center(), Align2::CENTER_CENTER, crate::skin::label_text(spec.name), small(), crate::skin::ink_dim());
    let now = params[k].round().clamp(0.0, opts.len() as f32 - 1.0) as usize;
    let b = Rect::from_center_size(body.center(), vec2(body.width() - 4.0, body.height().min(20.0) - 2.0));
    let resp = button(ui, b, id, opts[now], false, accent).on_hover_text("Click: the next, right-click: the one before");
    rect_line(&p, b, 2.0, 1.0, crate::skin::ink_dim());
    let step = if resp.clicked() {
        1
    } else if resp.secondary_clicked() {
        -1
    } else {
        return false;
    };
    params[k] = (now as i32 + step).rem_euclid(opts.len() as i32) as f32;
    true
}

/// A choice from a list: its name above, a dropdown below.
fn dropdown(ui: &mut Ui, r: Rect, id: Id, k: usize, params: &mut [f32]) -> bool {
    let spec = &m::SPECS[k];
    let ParamKind::Choice(opts) = spec.kind else { return false };
    let p = ui.painter().clone();
    let (head, body) = r.split_top_bottom_at_y(r.top() + 12.0);
    p.with_clip_rect(head).text(head.center(), Align2::CENTER_CENTER, crate::skin::label_text(spec.name), small(), crate::skin::ink_dim());
    let now = params[k].round().clamp(0.0, opts.len() as f32 - 1.0) as usize;
    let mut changed = false;
    let inner = Rect::from_center_size(body.center(), vec2(body.width() - 2.0, 20.0));
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(inner));
    egui::ComboBox::from_id_salt(id).width(inner.width() - 4.0).selected_text(RichText::new(opts[now]).small()).show_ui(&mut child, |ui| {
        for (i, o) in opts.iter().enumerate() {
            if ui.selectable_label(i == now, *o).clicked() && i != now {
                params[k] = i as f32;
                changed = true;
            }
        }
    });
    changed
}

/// The panel. `live`: the contours, the cutoff, whether a key is down.
pub(crate) fn munich_panel(ui: &mut Ui, id: Id, params: &mut [f32], live: &InstLive, height: f32, mut mv: Option<&mut ModView>) -> bool {
    if params.len() < m::SPECS.len() {
        return false;
    }
    let accent = accent_now();
    let h = height.clamp(150.0, 300.0);
    let rows3 = (h - 16.0) / 3.0;
    let rows2 = (h - 16.0) / 2.0;
    let free = params[m::ECHO_TIME].round() as usize == m::ECHO_TIMES.len() - 1;
    let osc_w = 62.0 + 86.0 + CELL * 2.0 + 8.0;
    let widths = [osc_w, CELL + 6.0, CELL * 3.0 + 8.0, CELL * 4.0 + 8.0, 100.0 + CELL * 1.0 + 8.0, CELL * 2.0 + 8.0, CELL * 2.0 + 60.0 + 8.0, CELL + 6.0];
    let total: f32 = widths.iter().sum::<f32>() + GAP * (widths.len() as f32 - 1.0);
    let (rect, _) = ui.allocate_exact_size(vec2(total, h), Sense::hover());
    let p = ui.painter().clone();
    let mut changed = false;
    let mut x = rect.left();
    let mut next = |w: usize| {
        let at = x;
        x += widths[w] + GAP;
        at
    };
    let title = |at: f32, w: usize, s: &str, first: bool| {
        p.text(pos2(at + 3.0, rect.top() + 6.0), Align2::LEFT_CENTER, crate::skin::label_text(s), small(), crate::skin::ink());
        p.line_segment([pos2(at + 3.0, rect.top() + 13.0), pos2(at + widths[w] - 3.0, rect.top() + 13.0)], Stroke::new(1.0_f32, crate::skin::ink_dim()));
        if !first {
            p.line_segment([pos2(at - GAP * 0.5, rect.top() + 2.0), pos2(at - GAP * 0.5, rect.bottom() - 2.0)], Stroke::new(1.0_f32, LINE_SOFT));
        }
    };
    let top = rect.top() + 16.0;
    macro_rules! knob {
        ($r:expr, $k:expr) => {{
            let r: Rect = $r;
            changed |= param_cell(ui, r, id.with(("k", $k)), &m::SPECS[$k], &mut params[$k], accent);
            if let Some(mv) = mv.as_deref_mut() {
                cell_overlay(ui, r, id.with(("o", $k)), &m::SPECS[$k], params[$k], $k, mv);
            }
        }};
    }

    // The oscillators, one a row: range, wave, detune, level.
    let at = next(0);
    title(at, 0, "Oscillators", true);
    for (row, (range, wave, detune, level)) in [(m::RANGE1, m::WAVE1, None, m::LEVEL1), (m::RANGE2, m::WAVE2, Some(m::DETUNE2), m::LEVEL2), (m::RANGE3, m::WAVE3, Some(m::DETUNE3), m::LEVEL3)].into_iter().enumerate() {
        let y = top + row as f32 * rows3;
        let mut cx = at;
        changed |= clicker(ui, Rect::from_min_size(pos2(cx, y), vec2(60.0, rows3 - 2.0)), id.with(("range", row)), range, params, accent);
        cx += 62.0;
        changed |= dropdown(ui, Rect::from_min_size(pos2(cx, y), vec2(84.0, rows3 - 2.0)), id.with(("wave", row)), wave, params);
        cx += 86.0;
        match detune {
            Some(d) => knob!(Rect::from_min_size(pos2(cx, y), vec2(CELL, rows3)), d),
            None => {
                let c = Rect::from_min_size(pos2(cx, y), vec2(CELL, rows3)).center();
                p.text(c, Align2::CENTER_CENTER, format!("{}", row + 1), FontId::proportional(18.0), crate::skin::ink_dim());
            }
        }
        cx += CELL;
        knob!(Rect::from_min_size(pos2(cx, y), vec2(CELL, rows3)), level);
    }

    // The mixer.
    let at = next(1);
    title(at, 1, "Mixer", false);
    for (i, k) in [m::NOISE, m::DRIVE, m::FEEDBACK].into_iter().enumerate() {
        knob!(Rect::from_min_size(pos2(at + 3.0, top + i as f32 * rows3), vec2(CELL, rows3)), k);
    }

    // The ladder: two big knobs, what moves it under them.
    let at = next(2);
    title(at, 2, "Ladder", false);
    let big = CELL * 1.5;
    knob!(Rect::from_min_size(pos2(at, top), vec2(big + 2.0, rows2 + 8.0)), m::CUTOFF);
    knob!(Rect::from_min_size(pos2(at + big + 2.0, top), vec2(big + 2.0, rows2 + 8.0)), m::EMPHASIS);
    let lower = top + rows2 + 8.0;
    knob!(Rect::from_min_size(pos2(at, lower), vec2(big + 2.0, rows2 - 8.0)), m::CONTOUR);
    changed |= clicker(ui, Rect::from_min_size(pos2(at + big + 4.0, lower + 4.0), vec2(big, 36.0)), id.with("keyboard"), m::KEYBOARD, params, accent);
    // Where the contour takes the filter now.
    let keyed = live.vals[3] > 0.5 || live.vals[1] > 0.001;
    if keyed {
        let meter = Rect::from_min_size(pos2(at + 3.0, lower - 5.0), vec2(big * 2.0, 3.0));
        p.rect_filled(meter, 1.0, shade(accent, 0.25));
        let f = live.vals[2].clamp(0.0, 1.0);
        p.rect_filled(Rect::from_min_size(meter.min, vec2(meter.width() * f, meter.height())), 1.0, accent);
        ui.ctx().request_repaint();
    }

    // The two contours.
    let at = next(3);
    title(at, 3, "Filter Contour / Loudness", false);
    for (row, ks) in [[m::F_ATTACK, m::F_DECAY, m::F_SUSTAIN, m::F_RELEASE], [m::ATTACK, m::DECAY, m::SUSTAIN, m::RELEASE]].into_iter().enumerate() {
        for (i, k) in ks.into_iter().enumerate() {
            knob!(Rect::from_min_size(pos2(at + i as f32 * CELL, top + row as f32 * rows2), vec2(CELL, rows2)), k);
        }
    }

    // Modulation.
    let at = next(4);
    title(at, 4, "Modulation", false);
    changed |= dropdown(ui, Rect::from_min_size(pos2(at, top), vec2(100.0, rows3 - 2.0)), id.with("source"), m::MOD_SOURCE, params);
    knob!(Rect::from_min_size(pos2(at + 102.0, top), vec2(CELL, rows3)), m::MOD_RATE);
    knob!(Rect::from_min_size(pos2(at + 8.0, top + rows3), vec2(CELL + 4.0, rows3 * 2.0)), m::MOD_PITCH);
    knob!(Rect::from_min_size(pos2(at + 60.0, top + rows3), vec2(CELL + 4.0, rows3 * 2.0)), m::MOD_FILTER);

    // The keys.
    let at = next(5);
    title(at, 5, "Keys", false);
    knob!(Rect::from_min_size(pos2(at, top), vec2(CELL, rows3)), m::GLIDE);
    knob!(Rect::from_min_size(pos2(at + CELL, top), vec2(CELL, rows3)), m::VELOCITY);
    knob!(Rect::from_min_size(pos2(at, top + rows3), vec2(CELL, rows3)), m::DRIFT);
    changed |= clicker(ui, Rect::from_min_size(pos2(at + CELL, top + rows3 + 8.0), vec2(CELL + 4.0, 36.0)), id.with("priority"), m::PRIORITY, params, accent);
    changed |= clicker(ui, Rect::from_min_size(pos2(at + 4.0, top + rows3 * 2.0 + 6.0), vec2(CELL * 2.0, 36.0)), id.with("trigger"), m::TRIGGER, params, accent);

    // The echo.
    let at = next(6);
    title(at, 6, "Echo", false);
    knob!(Rect::from_min_size(pos2(at, top), vec2(CELL, rows2)), m::ECHO_MIX);
    changed |= dropdown(ui, Rect::from_min_size(pos2(at + CELL + 2.0, top), vec2(CELL + 58.0, rows3 - 2.0)), id.with("echo_time"), m::ECHO_TIME, params);
    if free {
        knob!(Rect::from_min_size(pos2(at + CELL + 30.0, top + rows3 - 6.0), vec2(CELL, rows2 - rows3 + 10.0)), m::ECHO_FREE);
    }
    for (i, k) in [m::ECHO_FEEDBACK, m::ECHO_SPREAD, m::ECHO_TAPE].into_iter().enumerate() {
        knob!(Rect::from_min_size(pos2(at + i as f32 * (CELL + 6.0), top + rows2), vec2(CELL + 6.0, rows2)), k);
    }

    // Out.
    let at = next(7);
    title(at, 7, "Out", false);
    knob!(Rect::from_min_size(pos2(at + 3.0, top), vec2(CELL, rows2)), m::VOLUME);
    changed
}

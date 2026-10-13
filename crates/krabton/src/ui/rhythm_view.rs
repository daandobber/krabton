//! The rhythm generators' faces. Drum Map: the map of grooves (drag to
//! move across it) and the two bars it plays, every step's weight a bar and
//! each drum's fill a line to drag (what reaches above it plays). Marbles:
//! the T section's last ticks per output with its déjà vu loop, and the X
//! section's distribution with the three marbles where they fell.

use super::seq_view::Live;
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, Id, Rect, Sense, Stroke, Ui};
use krabton_engine::sequencers::drum_map as dm;
use krabton_engine::sequencers::marbles as mb;

const CLOCK: Color32 = Color32::from_rgb(0x8c, 0x8c, 0x92);

fn set(params: &mut [f32], i: usize, v: f32) -> bool {
    let v = (v * 100.0).round() / 100.0;
    if params[i] != v {
        params[i] = v;
        true
    } else {
        false
    }
}

/// A caption on a dark backing, readable over the map.
fn label(p: &egui::Painter, pos: egui::Pos2, anchor: Align2, s: &str, c: Color32) {
    let galley = p.layout_no_wrap(s.to_string(), small(), c);
    let r = anchor.anchor_size(pos, galley.size());
    p.rect_filled(r.expand2(vec2(3.0, 1.0)), 3.0, KNOB.gamma_multiply(0.85));
    p.galley(r.min, galley, c);
}

// ---------------------------------------------------------------- drum map

pub(crate) fn drum_map(ui: &mut Ui, id: Id, params: &mut [f32], live: Live, accent: Color32, height: f32) -> bool {
    let mut changed = false;
    let side = (height - 4.0).clamp(90.0, 210.0);
    let label_w = 44.0;
    let cell = 9.0;
    let lanes_w = label_w + dm::STEPS as f32 * cell + 6.0;
    let (rect, _) = ui.allocate_exact_size(vec2(side + 10.0 + lanes_w, height), Sense::hover());
    let p = ui.painter().clone();

    // ---------------------------------------------------------------- the map
    let map = Rect::from_min_size(rect.min, vec2(side, side));
    rect_fill(&p, map, 3.0, KNOB);
    let n = 16;
    let cw = map.width() / n as f32;
    let busy = |x: f32, y: f32| dm::weights(x, y).iter().flatten().map(|&w| w as f32).sum::<f32>();
    let most = busy(1.0, 1.0).max(1.0);
    for gy in 0..n {
        for gx in 0..n {
            let (x, y) = ((gx as f32 + 0.5) / n as f32, (gy as f32 + 0.5) / n as f32);
            let t = (busy(x, y) / most).clamp(0.0, 1.0);
            let r = Rect::from_min_size(pos2(map.left() + gx as f32 * cw, map.bottom() - (gy + 1) as f32 * cw), vec2(cw + 0.5, cw + 0.5));
            p.rect_filled(r, 0.0, mix(KNOB, accent, 0.05 + 0.4 * t * t));
        }
    }
    let at = |x: f32, y: f32| pos2(map.left() + x * map.width(), map.bottom() - y * map.height());
    for gy in 0..dm::SIZE {
        for gx in 0..dm::SIZE {
            let c = at(gx as f32 / 4.0 * 0.9 + 0.05, gy as f32 / 4.0 * 0.9 + 0.05);
            p.circle_filled(c, 2.0, shade(accent, 0.5));
        }
    }
    let resp = ui.interact(map, id.with("map"), Sense::click_and_drag());
    if let Some(pt) = resp.interact_pointer_pos().filter(|_| resp.dragged() || resp.clicked()) {
        let x = ((pt.x - map.left()) / map.width() - 0.05) / 0.9;
        let y = ((map.bottom() - pt.y) / map.height() - 0.05) / 0.9;
        changed |= set(params, dm::MAP_X, x.clamp(0.0, 1.0));
        changed |= set(params, dm::MAP_Y, y.clamp(0.0, 1.0));
    }
    let (mx, my) = (params[dm::MAP_X].clamp(0.0, 1.0), params[dm::MAP_Y].clamp(0.0, 1.0));
    let cur = at(mx * 0.9 + 0.05, my * 0.9 + 0.05);
    p.line_segment([pos2(cur.x, map.top()), pos2(cur.x, map.bottom())], Stroke::new(1.0_f32, shade(accent, 0.35)));
    p.line_segment([pos2(map.left(), cur.y), pos2(map.right(), cur.y)], Stroke::new(1.0_f32, shade(accent, 0.35)));
    p.circle_filled(cur, 9.0, accent.gamma_multiply(0.2));
    p.circle_filled(cur, 5.0, accent);
    let nearest = |x: f32, y: f32| dm::node_name((x * 4.0).round() as usize, (y * 4.0).round() as usize);
    label(&p, map.left_top() + vec2(4.0, 3.0), Align2::LEFT_TOP, "driving", TEXT_DIM);
    label(&p, map.left_bottom() + vec2(4.0, -3.0), Align2::LEFT_BOTTOM, "laid back · straight", TEXT_DIM);
    label(&p, map.right_bottom() + vec2(-4.0, -3.0), Align2::RIGHT_BOTTOM, "broken", TEXT_DIM);
    label(&p, map.right_top() + vec2(-4.0, 3.0), Align2::RIGHT_TOP, nearest(mx, my), accent);
    let hover = resp.hover_pos().map(|pt| (((pt.x - map.left()) / map.width() - 0.05) / 0.9, ((map.bottom() - pt.y) / map.height() - 0.05) / 0.9));
    let tip = match hover {
        Some((x, y)) => format!("{}: drag across the map to morph between grooves (left straight, right broken; bottom laid back, top driving)", nearest(x.clamp(0.0, 1.0), y.clamp(0.0, 1.0))),
        None => String::new(),
    };
    resp.on_hover_text(tip);

    // ---------------------------------------------------------------- the lanes
    let w = dm::weights(mx, my);
    let calm = [[0.0; dm::STEPS]; dm::PARTS];
    let vis = live.on();
    let hits: [u32; dm::PARTS] = match vis {
        Some(v) => [v[1] as u32, (v[1] >> 32) as u32, v[2] as u32],
        None => dm::pattern(&w, params, &calm),
    };
    let step = vis.map(|v| (v[0] & 0xff) as usize);
    let lanes = Rect::from_min_size(pos2(map.right() + 10.0, rect.top()), vec2(lanes_w, height));
    let lane_h = ((height - 6.0) / dm::PARTS as f32).min(60.0);
    let colors = [accent, SELECT, PLAY];
    for part in 0..dm::PARTS {
        let lr = Rect::from_min_size(pos2(lanes.left(), lanes.top() + part as f32 * lane_h), vec2(lanes_w, lane_h - 4.0));
        let bars = Rect::from_min_max(pos2(lr.left() + label_w, lr.top()), lr.max);
        rect_fill(&p, bars, 2.0, KNOB);
        text(&p, pos2(lr.left(), lr.center().y - 6.0), Align2::LEFT_CENTER, dm::PART_NAMES[part], small(), TEXT);
        let fill = params[dm::FILL + part].clamp(0.0, 1.0);
        text(&p, pos2(lr.left(), lr.center().y + 7.0), Align2::LEFT_CENTER, format!("{:.0}%", fill * 100.0), small(), TEXT_DIM);
        let col = colors[part];
        for s in 0..dm::STEPS {
            let x = bars.left() + 3.0 + s as f32 * cell;
            if s % 4 == 0 {
                p.line_segment([pos2(x - 1.0, bars.top() + 2.0), pos2(x - 1.0, bars.bottom() - 2.0)], Stroke::new(1.0_f32, if s % 16 == 0 { LINE } else { LINE_SOFT }));
            }
            let weight = w[part][s] as f32 / 255.0;
            if weight > 0.0 {
                let top = bars.bottom() - 3.0 - weight * (bars.height() - 6.0);
                let r = Rect::from_min_max(pos2(x, top), pos2(x + cell - 2.0, bars.bottom() - 3.0));
                let on = hits[part] >> s & 1 == 1;
                let now = step == Some(s);
                let c = match (on, now) {
                    (true, true) => Color32::WHITE,
                    (true, false) => col,
                    (false, _) => shade(col, 0.28),
                };
                p.rect_filled(r, 1.0, c);
            } else if step == Some(s) {
                p.circle_filled(pos2(x + cell * 0.5 - 1.0, bars.bottom() - 4.0), 1.5, TEXT_DIM);
            }
        }
        // The fill: only what reaches above the line plays.
        let ly = bars.bottom() - 3.0 - (1.0 - fill) * (bars.height() - 6.0);
        p.line_segment([pos2(bars.left(), ly), pos2(bars.right(), ly)], Stroke::new(1.5_f32, shade(col, 0.9)));
        let resp = ui.interact(bars, id.with(("lane", part)), Sense::click_and_drag());
        if let Some(pt) = resp.interact_pointer_pos().filter(|_| resp.dragged() || resp.clicked()) {
            let f = 1.0 - ((bars.bottom() - 3.0 - pt.y) / (bars.height() - 6.0)).clamp(0.0, 1.0);
            changed |= set(params, dm::FILL + part, f);
        }
        resp.on_hover_text(format!("{}: every step's bar is how much it belongs to the groove; drag the line down for more of them, up for fewer", dm::PART_NAMES[part]));
    }
    changed
}

// ---------------------------------------------------------------- marbles

pub(crate) fn marbles(ui: &mut Ui, id: Id, params: &mut [f32], live: Live, accent: Color32, height: f32) -> bool {
    let mut changed = false;
    let tw = 170.0;
    let xw = 190.0;
    let (rect, _) = ui.allocate_exact_size(vec2(tw + 8.0 + xw, height), Sense::hover());
    let p = ui.painter().clone();
    let vis = live.on();
    let notes_mode = params[mb::PLAY] >= 0.5;
    // T1 and T3 the dice, T2 the clock (all dim while the device is off).
    let colors = if accent == TEXT_DIM { [TEXT_DIM; 3] } else { [ACCENT, CLOCK, SELECT] };

    // ---------------------------------------------------------------- T
    let tr = Rect::from_min_size(rect.min, vec2(tw, height));
    rect_fill(&p, tr, 3.0, KNOB);
    let model = mb::MODELS[(params[mb::MODEL].round().max(0.0) as usize).min(mb::MODELS.len() - 1)];
    text(&p, tr.left_top() + vec2(6.0, 4.0), Align2::LEFT_TOP, format!("T · {model}"), small(), accent);
    let row_h = ((height - 58.0) / 3.0).clamp(12.0, 26.0);
    let dot_w = (tw - 34.0) / 16.0;
    for c in 0..3 {
        let y = tr.top() + 26.0 + c as f32 * row_h;
        let on = params[mb::T_ON + c] >= 0.5;
        text(&p, pos2(tr.left() + 6.0, y), Align2::LEFT_CENTER, format!("T{}", c + 1), small(), if on { TEXT } else { TEXT_DIM });
        let hist = vis.map(|v| (v[0] >> (16 * c)) as u16).unwrap_or(0);
        let gate = vis.is_some_and(|v| (v[2] >> (28 + c)) & 1 == 1);
        for k in 0..16 {
            let x = tr.left() + 30.0 + k as f32 * dot_w + dot_w * 0.5;
            let hit = hist >> (15 - k) & 1 == 1;
            let col = if on { colors[c] } else { TEXT_DIM };
            if hit {
                p.circle_filled(pos2(x, y), (dot_w * 0.32).min(4.5), col);
            } else {
                p.circle_filled(pos2(x, y), 1.2, shade(col, 0.35));
            }
        }
        if gate && on {
            let x = tr.left() + 30.0 + 15.5 * dot_w;
            p.circle_filled(pos2(x, y), (dot_w * 0.6).min(8.0), colors[c].gamma_multiply(0.25));
        }
    }
    // The déjà vu loop.
    let length = params[mb::LENGTH].round().clamp(1.0, 16.0) as usize;
    let t_loop = params[mb::T_LOOP].round() as usize;
    let dv = mb::deja_vu_amount(params[mb::DEJA_VU].clamp(0.0, 1.0));
    let ly = tr.bottom() - 14.0;
    let lw = (tw - 70.0) / 16.0;
    let step = vis.map(|v| ((v[0] >> 48) & 0xff) as usize);
    let loop_txt = match t_loop {
        0 => "loop off".to_string(),
        2 => "locked".to_string(),
        _ if dv == 0.5 => "locked".to_string(),
        _ if dv < 0.5 => format!("{:.0}% new", ((2.0 * dv - 1.0).powi(2) * 100.0)),
        _ => format!("{:.0}% jumps", ((2.0 * dv - 1.0).powi(2) * 100.0)),
    };
    text(&p, pos2(tr.left() + 6.0, ly), Align2::LEFT_CENTER, "déjà vu", small(), TEXT_DIM);
    for k in 0..length {
        let x = tr.left() + 52.0 + k as f32 * lw + lw * 0.5;
        let now = step == Some(k) && t_loop != 0;
        p.circle_filled(pos2(x, ly), if now { 3.0 } else { 1.8 }, if now { accent } else if t_loop == 0 { LINE } else { shade(accent, 0.45) });
    }
    text(&p, pos2(tr.right() - 5.0, ly - 12.0), Align2::RIGHT_CENTER, loop_txt, small(), TEXT_DIM);
    let resp = ui.interact(tr, id.with("t"), Sense::click_and_drag());
    if let Some(pt) = resp.interact_pointer_pos().filter(|_| resp.dragged() || resp.clicked()) {
        changed |= set(params, mb::BIAS, ((pt.x - tr.left() - 30.0) / (tw - 34.0)).clamp(0.0, 1.0));
        changed |= set(params, mb::JITTER, ((tr.bottom() - 24.0 - pt.y) / (tr.height() - 50.0)).clamp(0.0, 1.0));
    }
    resp.on_hover_text("T2 is the clock, T1 and T3 the dice (the last 16 ticks, newest right). Drag: left and right for Bias (left favours T1), up for Jitter");

    // ---------------------------------------------------------------- X
    let xr = Rect::from_min_size(pos2(tr.right() + 8.0, rect.top()), vec2(xw, height));
    rect_fill(&p, xr, 3.0, KNOB);
    let spread = params[mb::SPREAD].clamp(0.0, 1.0);
    let bias = params[mb::X_BIAS].clamp(0.0, 1.0);
    text(&p, xr.left_top() + vec2(6.0, 4.0), Align2::LEFT_TOP, if notes_mode { "X · notes" } else { "X · dynamics" }, small(), accent);
    let plot = Rect::from_min_max(pos2(xr.left() + 8.0, xr.top() + 22.0), pos2(xr.right() - 8.0, xr.bottom() - 30.0));
    // The distribution as a histogram of evenly spread draws.
    let bins = 30;
    let mut hist = vec![0f32; bins];
    let draws = 600;
    for k in 0..draws {
        let u = (k as f32 + 0.5) / draws as f32;
        let x = mb::x_value(u, spread, bias);
        hist[((x * bins as f32) as usize).min(bins - 1)] += 1.0;
    }
    let top = hist.iter().cloned().fold(1.0f32, f32::max);
    let bw = plot.width() / bins as f32;
    for (k, h) in hist.iter().enumerate() {
        let hh = (h / top).sqrt() * plot.height() * 0.85;
        let r = Rect::from_min_max(pos2(plot.left() + k as f32 * bw + 0.5, plot.bottom() - hh), pos2(plot.left() + (k + 1) as f32 * bw - 0.5, plot.bottom()));
        p.rect_filled(r, 1.0, shade(accent, 0.3));
    }
    // The notes that Steps lets through (as notes).
    if notes_mode {
        let range = params[mb::RANGE].round().clamp(1.0, 4.0);
        let base = params[mb::BASE].round();
        let steps = params[mb::STEPS].clamp(0.0, 1.0);
        let semis = (range * 12.0) as i32;
        for s in 0..=semis {
            let pitch = base - range * 6.0 + s as f32;
            if mb::quantize(pitch, steps, &live.scale, true) == pitch as i32 {
                let x = plot.left() + s as f32 / semis as f32 * plot.width();
                let root = live.scale.is_root(pitch.clamp(0.0, 127.0) as u8);
                p.line_segment([pos2(x, plot.bottom() + 2.0), pos2(x, plot.bottom() + if root { 9.0 } else { 6.0 })], Stroke::new(1.0_f32, if root { TEXT } else { TEXT_DIM }));
            }
        }
    }
    // The marbles.
    let xs: [f32; 3] = match vis {
        Some(v) => std::array::from_fn(|c| ((v[1] >> (16 * c)) & 0xffff) as f32 / 65535.0),
        None => [bias; 3],
    };
    let notes: [u8; 3] = vis.map(|v| std::array::from_fn(|c| ((v[2] >> (8 * c)) & 0xff) as u8)).unwrap_or([0; 3]);
    for c in 0..3 {
        let x = plot.left() + xs[c].clamp(0.0, 1.0) * plot.width();
        let y = plot.bottom() - 8.0 - c as f32 * 13.0;
        let col = if vis.is_some() { colors[c] } else { shade(colors[c], 0.5) };
        p.circle_filled(pos2(x, y), 5.5, col);
        p.circle_filled(pos2(x - 1.8, y - 1.8), 1.6, Color32::from_white_alpha(150));
        if vis.is_some() && notes_mode && params[mb::T_ON + c] >= 0.5 {
            text(&p, pos2(x + 8.0, y), Align2::LEFT_CENTER, krabton_engine::note_name(notes[c]), small(), TEXT_DIM);
        }
    }
    text(&p, pos2(xr.left() + 6.0, xr.bottom() - 4.0), Align2::LEFT_BOTTOM, format!("spread {:.0}%", spread * 100.0), small(), TEXT_DIM);
    text(&p, pos2(xr.right() - 6.0, xr.bottom() - 4.0), Align2::RIGHT_BOTTOM, format!("bias {:.0}%", bias * 100.0), small(), TEXT_DIM);
    let resp = ui.interact(xr, id.with("x"), Sense::click_and_drag());
    if let Some(pt) = resp.interact_pointer_pos().filter(|_| resp.dragged() || resp.clicked()) {
        changed |= set(params, mb::X_BIAS, ((pt.x - plot.left()) / plot.width()).clamp(0.0, 1.0));
        changed |= set(params, mb::SPREAD, ((plot.bottom() - pt.y) / plot.height()).clamp(0.0, 1.0));
    }
    resp.on_hover_text(if notes_mode {
        "Where the three X values fall (each changes when its T plays, and plays it). Drag: left and right for Bias, up for Spread"
    } else {
        "Where the three X values fall: how hard each T hits. Drag: left and right for Bias, up for Spread"
    });
    if vis.is_some() {
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(30));
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::Pos2;
    use krabton_engine::params::defaults;

    #[test]
    fn the_rhythm_screens_draw() {
        let ctx = egui::Context::default();
        let live = Live { vis: Some([0x0f0f_00ff_1234, u64::MAX / 3, 0x1234_5678, 0]), playing: true, scale: krabton_engine::Scale::new(0, krabton_engine::ScaleKind::Major), kit: 0 };
        let mut map = defaults(dm::SPECS);
        let mut marb = defaults(mb::SPECS);
        for notes in [0.0, 1.0] {
            marb[mb::PLAY] = notes;
            for _ in 0..2 {
                let _ = ctx.run(egui::RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1400.0, 800.0))), ..Default::default() }, |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        drum_map(ui, Id::new("m"), &mut map, live, ACCENT, 200.0);
                        marbles(ui, Id::new("b"), &mut marb, live, ACCENT, 200.0);
                    });
                });
            }
        }
    }
}

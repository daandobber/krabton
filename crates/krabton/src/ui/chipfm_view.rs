//! Chip FM's screen: the algorithm as boxes and wires (click it for the
//! next one; the loop on operator 1 is its feedback), the chip (click to
//! change it), the channels of the chip with each of their four operators
//! lit as loud as its envelope is now, and the operators' envelopes.

use super::inst_screen::{caption, InstLive, SCREEN_BG};
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, FontId, Id, Pos2, Rect, Sense, Shape, Stroke, Ui};
use krabton_engine::dsp::chipfm::*;

const DIM: Color32 = Color32::from_rgb(0x4a, 0x4e, 0x58);
const FAINT: Color32 = Color32::from_rgb(0x1e, 0x21, 0x28);
const YELLOW: Color32 = Color32::from_rgb(0xff, 0xd4, 0x3b);

fn get(p: &[f32], i: usize) -> f32 {
    p.get(i).copied().unwrap_or(SPECS[i].default)
}

fn set(p: &mut [f32], i: usize, v: f32) -> bool {
    if p.get(i).is_some_and(|x| *x != v) {
        p[i] = v;
        true
    } else {
        false
    }
}

/// Each algorithm: where the four operators sit (column, row), the wires
/// between them, and which ones sound.
fn layout(alg: usize) -> ([(f32, f32); 4], &'static [(usize, usize)], [bool; 4]) {
    match alg {
        0 => ([(0.0, 0.5), (1.0, 0.5), (2.0, 0.5), (3.0, 0.5)], &[(0, 1), (1, 2), (2, 3)], [false, false, false, true]),
        1 => ([(0.0, 0.0), (0.0, 1.0), (1.0, 0.5), (2.0, 0.5)], &[(0, 2), (1, 2), (2, 3)], [false, false, false, true]),
        2 => ([(1.0, 0.0), (0.0, 1.0), (1.0, 1.0), (2.0, 0.5)], &[(0, 3), (1, 2), (2, 3)], [false, false, false, true]),
        3 => ([(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (2.0, 0.5)], &[(0, 1), (1, 3), (2, 3)], [false, false, false, true]),
        4 => ([(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)], &[(0, 1), (2, 3)], [false, true, false, true]),
        5 => ([(0.0, 0.5), (1.0, -0.2), (1.0, 0.5), (1.0, 1.2)], &[(0, 1), (0, 2), (0, 3)], [false, true, true, true]),
        6 => ([(0.0, -0.2), (1.0, -0.2), (1.0, 0.5), (1.0, 1.2)], &[(0, 1)], [false, true, true, true]),
        _ => ([(0.0, 0.5), (1.0, 0.5), (2.0, 0.5), (3.0, 0.5)], &[], [true, true, true, true]),
    }
}

/// An operator's envelope, roughly: attack, decay to the sustain level, the
/// sustain rate's slope, release.
fn env_points(p: &[f32], k: usize, r: Rect) -> Vec<Pos2> {
    let speed = |rate: f32, max: f32| 1.0 - (rate / max).clamp(0.0, 1.0);
    let a = 0.02 + speed(get(p, op(k, AR)), 31.0) * 0.25;
    let d = 0.02 + speed(get(p, op(k, DR)), 31.0) * 0.3;
    let sl = 1.0 - get(p, op(k, SL)) / 15.0;
    let srate = get(p, op(k, SR)) / 31.0;
    let rel = 0.02 + speed(get(p, op(k, RR)), 15.0) * 0.25;
    let hold = (1.0 - a - d - rel).max(0.05);
    let level = 1.0 - get(p, op(k, TL)) / 127.0;
    let y = |v: f32| r.bottom() - (v * level).clamp(0.0, 1.0) * r.height();
    let x = |t: f32| r.left() + t * r.width();
    let s_end = (sl * (1.0 - srate * 0.9)).max(0.0);
    vec![pos2(x(0.0), y(0.0)), pos2(x(a), y(1.0)), pos2(x(a + d), y(sl)), pos2(x(a + d + hold), y(s_end)), pos2(x(1.0), y(0.0))]
}

pub(crate) fn chipfm_screen(ui: &mut Ui, id: Id, p: &mut [f32], live: &InstLive, height: f32) -> bool {
    let w = 440.0;
    let (rect, _) = ui.allocate_exact_size(vec2(w, height), Sense::hover());
    let painter = ui.painter_at(rect);
    rect_fill(&painter, rect, 6.0, SCREEN_BG);
    rect_line(&painter, rect, 6.0, 1.0, Color32::from_rgb(0x26, 0x29, 0x31));
    let accent = accent_now();
    let mut changed = false;
    caption(&painter, rect.left_top() + vec2(10.0, 6.0), Align2::LEFT_TOP, "Chip FM", accent);
    let chip = get(p, CHIP).round().clamp(0.0, 1.0) as usize;
    let channels = if chip == 0 { 6 } else { 8 };
    // The chip (click: the other one).
    let chip_label = format!("{}  ·  {} CHANNELS", CHIPS[chip].to_uppercase(), channels);
    let cr = Rect::from_min_size(pos2(rect.right() - 170.0, rect.top() + 4.0), vec2(160.0, 16.0));
    let resp = ui.interact(cr, id.with("chip"), Sense::click());
    painter.text(cr.right_center(), Align2::RIGHT_CENTER, chip_label, FontId::monospace(9.0), if resp.hovered() { accent } else { TEXT_DIM });
    if resp.clicked() {
        changed |= set(p, CHIP, ((chip + 1) % 2) as f32);
    }
    let top = rect.top() + 24.0;
    let half = (rect.height() - 34.0) * 0.5;
    // The algorithm.
    let ar = Rect::from_min_size(pos2(rect.left() + 10.0, top), vec2(220.0, half * 2.0 - 4.0));
    painter.rect_filled(ar, 3.0, FAINT);
    let alg = get(p, ALGORITHM).round().clamp(0.0, 7.0) as usize;
    let (pos, wires, carriers) = layout(alg);
    let cols = pos.iter().map(|q| q.0).fold(0.0, f32::max) + 1.0;
    let inner = ar.shrink2(vec2(16.0, 26.0));
    let bw = 30.0;
    let at = |k: usize| {
        let (c, r) = pos[k];
        pos2(inner.left() + (c + 0.5) / cols * (inner.width() - 30.0), inner.top() + 10.0 + r * (inner.height() - 30.0) * 0.6)
    };
    let newest = live.vals[6] > 0.0;
    for &(a, b) in wires {
        painter.line_segment([at(a) + vec2(bw * 0.5, 0.0), at(b) - vec2(bw * 0.5, 0.0)], Stroke::new(1.5_f32, DIM));
    }
    let out_x = inner.right() - 4.0;
    for k in 0..4 {
        let c = at(k);
        if carriers[k] {
            painter.line_segment([c + vec2(bw * 0.5, 0.0), pos2(out_x, c.y)], Stroke::new(1.5_f32, accent.gamma_multiply(0.6)));
        }
        let lvl = (0..channels).map(|ch| live.vals[8 + ch * 4 + k]).fold(0.0f32, f32::max);
        let box_r = Rect::from_center_size(c, vec2(bw, 20.0));
        let tl = get(p, op(k, TL));
        let fill = if tl >= 127.0 { FAINT } else { Color32::from_rgb(0x2c, 0x30, 0x3a) };
        painter.rect_filled(box_r, 3.0, fill);
        if newest && lvl > 0.0 {
            painter.rect_filled(Rect::from_min_max(pos2(box_r.left(), box_r.bottom() - lvl * box_r.height()), box_r.max), 3.0, (if carriers[k] { accent } else { YELLOW }).gamma_multiply(0.6));
        }
        painter.rect_stroke(box_r, 3.0, Stroke::new(1.0_f32, if carriers[k] { accent } else { YELLOW }), egui::StrokeKind::Inside);
        painter.text(c, Align2::CENTER_CENTER, format!("{}", k + 1), FontId::monospace(10.0), TEXT);
        painter.text(c + vec2(0.0, 13.0), Align2::CENTER_TOP, format!("x{}", get(p, op(k, MUL)).round()), FontId::monospace(7.5), TEXT_DIM);
    }
    painter.line_segment([pos2(out_x, inner.top()), pos2(out_x, inner.bottom())], Stroke::new(2.0_f32, accent.gamma_multiply(0.5)));
    // The feedback loop on operator 1.
    let fb = get(p, FEEDBACK).round();
    if fb > 0.0 {
        let c = at(0);
        let pts = vec![c + vec2(bw * 0.5, -4.0), c + vec2(bw * 0.5 + 6.0, -4.0), c + vec2(bw * 0.5 + 6.0, -16.0), c + vec2(-bw * 0.5 - 6.0, -16.0), c + vec2(-bw * 0.5 - 6.0, -4.0), c + vec2(-bw * 0.5, -4.0)];
        painter.add(Shape::line(pts, Stroke::new(1.2_f32, YELLOW)));
        painter.text(c + vec2(0.0, -17.0), Align2::CENTER_BOTTOM, format!("FB {fb}"), FontId::monospace(7.5), YELLOW);
    }
    painter.text(ar.left_top() + vec2(4.0, 2.0), Align2::LEFT_TOP, format!("ALGORITHM {alg}"), FontId::monospace(8.5), TEXT);
    let resp = ui.interact(ar, id.with("alg"), Sense::click());
    if resp.clicked() {
        changed |= set(p, ALGORITHM, ((alg + 1) % 8) as f32);
    } else if resp.secondary_clicked() {
        changed |= set(p, ALGORITHM, ((alg + 7) % 8) as f32);
    }
    // The channels: four operators each, lit by their envelopes.
    let gr = Rect::from_min_size(pos2(ar.right() + 6.0, top), vec2(rect.right() - 10.0 - ar.right() - 6.0, half - 4.0));
    painter.rect_filled(gr, 3.0, FAINT);
    painter.text(gr.left_top() + vec2(4.0, 2.0), Align2::LEFT_TOP, "CHANNELS", FontId::monospace(8.5), TEXT);
    let cell_w = (gr.width() - 30.0) / channels as f32;
    let cell_h = (gr.height() - 18.0) / 4.0;
    for ch in 0..channels {
        for k in 0..4 {
            let lvl = live.vals[8 + ch * 4 + k];
            let c = Rect::from_min_size(pos2(gr.left() + 24.0 + ch as f32 * cell_w, gr.top() + 15.0 + k as f32 * cell_h), vec2(cell_w - 2.0, cell_h - 2.0));
            let color = if carriers[k] { accent } else { YELLOW };
            painter.rect_filled(c, 1.0, if lvl > 0.0 { color.gamma_multiply(0.15 + 0.85 * lvl) } else { Color32::from_rgb(0x26, 0x29, 0x31) });
        }
    }
    for k in 0..4 {
        painter.text(pos2(gr.left() + 6.0, gr.top() + 15.0 + (k as f32 + 0.5) * cell_h), Align2::LEFT_CENTER, format!("{}", k + 1), FontId::monospace(7.5), TEXT_DIM);
    }
    // The envelopes.
    let er = Rect::from_min_size(pos2(gr.left(), top + half), vec2(gr.width(), half - 4.0));
    painter.rect_filled(er, 3.0, FAINT);
    painter.text(er.left_top() + vec2(4.0, 2.0), Align2::LEFT_TOP, "ENVELOPES", FontId::monospace(8.5), TEXT);
    let shape = er.shrink2(vec2(6.0, 8.0)).translate(vec2(0.0, 4.0));
    for k in 0..4 {
        if get(p, op(k, TL)) >= 127.0 {
            continue;
        }
        let color = if carriers[k] { accent } else { YELLOW };
        painter.add(Shape::line(env_points(p, k, shape), Stroke::new(1.2_f32, color.gamma_multiply(0.5 + 0.15 * k as f32))));
    }
    if newest {
        ui.ctx().request_repaint();
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::RawInput;

    #[test]
    fn the_chipfm_screen_draws_every_preset() {
        let ctx = egui::Context::default();
        for playing in [false, true] {
            let mut vals = [0.0; krabton_engine::engine::INST_VALS];
            if playing {
                vals[0] = 2.0;
                vals[6] = 1.0;
                for i in 8..40 {
                    vals[i] = 0.5;
                }
            }
            let live = InstLive { wave: vec![0.0; 64], notes: 1 << 60, vals };
            for name in preset_names() {
                let mut p = preset(name);
                let _ = ctx.run(RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1200.0, 800.0))), ..Default::default() }, |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        chipfm_screen(ui, Id::new("c"), &mut p, &live, 200.0);
                    });
                });
            }
        }
    }
}

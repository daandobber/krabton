//! The Unit Circle's editor, drawn as in a maths book: the circle with its
//! axes, P at θ with its radius, sin θ (red) up from the x-axis, cos θ (blue)
//! along it, tan θ (green) on the tangent at x = 1; the values in a box; and
//! the curve of the function P's notes follow, running on to the left from P
//! as it turns, with a dot where a point played.

use super::seq_more_view::{degree_name, drag_steps};
use super::seq_view::Live;
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, Id, Pos2, Rect, RichText, Sense, Shape, Stroke, Ui};
use krabton_engine::sequencers::circle as uc;
use std::f64::consts::TAU;

const SIN: Color32 = Color32::from_rgb(0xe8, 0x3a, 0x2e);
const COS: Color32 = Color32::from_rgb(0x2f, 0x6b, 0xff);
const TAN: Color32 = Color32::from_rgb(0x2f, 0xc0, 0x6a);
const BOX: Color32 = Color32::from_rgb(0xf0, 0x86, 0x80);

/// A point of the circle on screen: θ anticlockwise from the positive x-axis.
pub(crate) fn on_circle(c: Pos2, r: f32, theta: f64) -> Pos2 {
    c + vec2(theta.cos() as f32, -(theta.sin() as f32)) * r
}

/// What a click on a point does: a rest gets the base note, a note becomes a rest.
pub(crate) fn clicked_point(value: f32) -> f32 {
    if value.round() > 0.0 {
        0.0
    } else {
        1.0
    }
}

/// Which function the curve shows: the one the pitch follows, else sin.
fn traced(params: &[f32]) -> usize {
    match params[uc::PITCH].round() as usize {
        2 => 2,
        3 => 3,
        _ => 1,
    }
}

pub(crate) fn circle(ui: &mut Ui, id: Id, params: &mut [f32], live: Live, height: f32) -> bool {
    if params.len() < uc::CIRCLE.len() {
        return false;
    }
    // Its buttons above it.
    ui.vertical(|ui| drawn(ui, id, params, live, height)).inner
}

fn drawn(ui: &mut Ui, id: Id, params: &mut [f32], live: Live, height: f32) -> bool {
    let mut changed = false;
    let n = uc::points(params);
    // Fill the points, or turn them round.
    ui.horizontal(|ui| {
        ui.menu_button(RichText::new("Fill").small(), |ui| {
            for (w, name) in uc::FILLS.iter().enumerate() {
                if ui.button(*name).clicked() {
                    let seed = (ui.input(|i| i.time) * 1000.0) as u32;
                    for (k, v) in uc::fill(w, uc::MAX_POINTS, seed).into_iter().enumerate() {
                        params[uc::FIRST_POINT + k] = v;
                    }
                    changed = true;
                    ui.close();
                }
            }
        })
        .response
        .on_hover_text("Fills the points with an arpeggio, a scale, a chord...");
        for (label, by, tip) in [("<", 1usize, "Every note one point back"), (">", n - 1, "Every note one point on")] {
            if ui.small_button(label).on_hover_text(tip).clicked() {
                let old: Vec<f32> = (0..n).map(|k| params[uc::FIRST_POINT + k]).collect();
                for k in 0..n {
                    params[uc::FIRST_POINT + k] = old[(k + by) % n];
                }
                changed = true;
            }
        }
        ui.label(RichText::new("Click a point: a note or a rest. Drag it up or down: which note").small().color(TEXT_DIM));
    });

    let size = (height - 22.0).max(150.0);
    let r = size / 2.0 - 30.0;
    let width = TAU as f32 * r + size;
    let (rect, _) = ui.allocate_exact_size(vec2(width, size), Sense::hover());
    rect_fill(ui.painter(), rect, 3.0, KNOB);
    let p = ui.painter().with_clip_rect(rect);
    let c = pos2(rect.right() - size / 2.0, rect.center().y + 8.0);

    let period = uc::period_beats(params);
    let frac = live.on().map_or(0.0, |v| (v[1] & 0xffff) as f64 / 65535.0);
    let theta = uc::theta_at(params, frac);

    // The axes and the circle.
    p.line_segment([pos2(rect.left(), c.y), pos2(rect.right(), c.y)], Stroke::new(1.0_f32, LINE));
    p.line_segment([pos2(c.x, c.y - r - 14.0), pos2(c.x, c.y + r + 14.0)], Stroke::new(1.0_f32, LINE));
    p.circle_stroke(c, r, Stroke::new(2.0_f32, COS));

    // The curve of the function, time running back to the left from P.
    let which = traced(params);
    let color = [SIN, SIN, COS, TAN][which];
    let at = on_circle(c, r, theta);
    let y_of = |th: f64| c.y - r * match which {
        2 => th.cos(),
        3 => uc::tan_clipped(th),
        _ => th.sin(),
    } as f32;
    let span = TAU as f32 * r;
    let mut line: Vec<Pos2> = Vec::new();
    let mut last_y = None;
    for i in 0..=240 {
        let u = i as f64 / 240.0;
        let q = pos2(at.x - u as f32 * span, y_of(uc::theta_at(params, frac - u)));
        // tan leaps at its asymptotes: the line breaks there.
        if last_y.is_some_and(|y: f32| (y - q.y).abs() > r) && line.len() > 1 {
            p.add(Shape::line(std::mem::take(&mut line), Stroke::new(2.0_f32, color)));
        }
        last_y = Some(q.y);
        line.push(q);
    }
    if line.len() > 1 {
        p.add(Shape::line(line, Stroke::new(2.0_f32, color)));
    }
    // Where the points played on it.
    let now = frac * period;
    let mut played = Vec::new();
    let pieces = 160;
    for i in 0..pieces {
        let b0 = now - period * (i + 1) as f64 / pieces as f64;
        let b1 = now - period * i as f64 / pieces as f64;
        uc::passes(params, b0, b1, &mut |beat, k, _| played.push((beat, k)));
    }
    for (beat, k) in played {
        let u = ((now - beat) / period) as f32;
        let q = pos2(at.x - u * span, y_of(uc::theta_at_beat(params, beat)));
        if uc::point_note(params, k).is_some() {
            p.circle_filled(q, 3.5, Color32::WHITE);
        }
    }
    // From P to the curve, when the curve is not sin's.
    let end = pos2(at.x, y_of(theta));
    if (end.y - at.y).abs() > 1.0 {
        p.line_segment([at, end], Stroke::new(1.0_f32, shade(color, 0.6)));
    }

    // θ, sin θ, cos θ and tan θ on the circle.
    let foot = pos2(at.x, c.y);
    p.line_segment([c, at], Stroke::new(1.5_f32, TEXT_DIM));
    p.line_segment([c, foot], Stroke::new(3.0_f32, COS));
    p.line_segment([foot, at], Stroke::new(3.0_f32, SIN));
    let t = theta.tan();
    if t.abs() < 2.2 {
        let touch = pos2(c.x + r, c.y - r * t as f32);
        p.line_segment([pos2(c.x + r, c.y), touch], Stroke::new(3.0_f32, TAN));
        p.line_segment([c, touch], Stroke::new(1.0_f32, shade(TAN, 0.7)));
    }
    let arc: Vec<Pos2> = (0..=24).map(|i| on_circle(c, 16.0, theta.rem_euclid(TAU) * i as f64 / 24.0)).collect();
    p.add(Shape::line(arc, Stroke::new(1.0_f32, TEXT)));

    // The points on the circle, and their notes.
    let phases = uc::phases(params);
    for k in 0..n {
        let a = uc::point_angle(params, k);
        let dot = on_circle(c, r, a);
        let slot = uc::FIRST_POINT + k;
        let note = uc::point_note(params, k);
        // Lit as P (or another phase) has just passed it.
        let hit = live.on().is_some() && note.is_some() && (0..phases).any(|ph| {
            let lead = TAU * ph as f64 / phases as f64;
            let d = ((theta + lead) - a).rem_euclid(TAU);
            d.min(TAU - d) < 0.2
        });
        p.circle_filled(dot, if hit { 6.0 } else { 4.0 }, if note.is_some() { if hit { Color32::WHITE } else { accent_now() } } else { KNOB });
        p.circle_stroke(dot, 4.0, Stroke::new(1.0_f32, accent_now()));
        let label_at = on_circle(c, r + 17.0, a);
        let hit_r = Rect::from_center_size(label_at, vec2(28.0, 16.0));
        let (resp, moved) = drag_steps(ui, hit_r, id.with(("point", k)), &mut params[slot], 0.0, 15.0, 8.0);
        changed |= moved;
        if resp.clicked() {
            params[slot] = clicked_point(params[slot]);
            changed = true;
        }
        let label = note.map_or_else(|| "-".to_string(), |s| degree_name(&live, params[uc::BASE], s));
        if resp.hovered() {
            p.rect_filled(hit_r, 3.0, shade(accent_now(), 0.35));
        }
        text(&p, label_at, Align2::CENTER_CENTER, &label, small(), if note.is_some() { TEXT } else { TEXT_DIM });
        resp.on_hover_text(format!("θ{k} = {:.2}: click for a note or a rest, drag up or down for which note", a));
    }
    // P, and the other phases.
    for ph in 1..phases {
        p.circle_filled(on_circle(c, r, theta + TAU * ph as f64 / phases as f64), 5.0, Color32::from_gray(0x30));
    }
    p.circle_filled(at, 7.0, Color32::BLACK);
    p.circle_stroke(at, 7.0, Stroke::new(1.5_f32, Color32::WHITE));

    // The values, in their box.
    let values = format!("θ = {:.3}   sin θ = {:.3}   cos θ = {:.3}   tan θ = {:.3}", theta.rem_euclid(TAU), theta.sin() + 0.0, theta.cos() + 0.0, t + 0.0);
    let box_r = Rect::from_center_size(pos2(rect.center().x - size * 0.25, rect.top() + 12.0), vec2(330.0, 18.0));
    p.rect_filled(box_r, 2.0, BOX);
    p.rect_stroke(box_r, 2.0, Stroke::new(1.0_f32, Color32::BLACK), egui::StrokeKind::Inside);
    text(&p, box_r.center(), Align2::CENTER_CENTER, &values, small(), Color32::BLACK);
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_click_gives_a_rest_a_note_and_a_note_a_rest() {
        assert_eq!(clicked_point(0.0), 1.0);
        assert_eq!(clicked_point(4.0), 0.0);
    }

    #[test]
    fn theta_goes_anticlockwise_from_the_x_axis() {
        let c = pos2(100.0, 100.0);
        let zero = on_circle(c, 50.0, 0.0);
        assert!((zero.x - 150.0).abs() < 1e-3 && (zero.y - 100.0).abs() < 1e-3);
        let up = on_circle(c, 50.0, TAU / 4.0);
        assert!((up.y - 50.0).abs() < 1e-3, "a quarter turn: straight up");
    }
}

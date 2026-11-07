//! The Unit Circle's editor, drawn as in a maths book: the circle on the left
//! with its axes, P at θ with its radius, and on it sin θ (red, up from the
//! x-axis), cos θ (blue, along it), tan θ (green, on the tangent at x = 1),
//! cot θ (yellow, on the tangent at y = 1), sec θ and csc θ (along the axes, to
//! where the tangent at P meets them). To the right the waves of those shown
//! run on as P turns, newest at the line by the circle, each tied to its point
//! on the circle; the values in a box; a dot where a point played.

use super::seq_more_view::{degree_name, drag_steps};
use super::seq_view::Live;
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, Id, Pos2, Rect, RichText, Sense, Shape, Stroke, Ui};
use krabton_engine::sequencers::circle as uc;
use std::f64::consts::TAU;

/// Each function's colour: sin, cos, tan, sec, csc, cot.
const COLORS: [Color32; 6] = [
    Color32::from_rgb(0xe8, 0x3a, 0x2e),
    Color32::from_rgb(0x2f, 0x6b, 0xff),
    Color32::from_rgb(0x2f, 0xc0, 0x6a),
    Color32::from_rgb(0xb0, 0x5c, 0xf0),
    Color32::from_rgb(0xff, 0x9a, 0x2e),
    Color32::from_rgb(0xe8, 0xd0, 0x3a),
];
const BOX: Color32 = Color32::from_rgb(0xf0, 0x86, 0x80);

/// A point of the circle on screen: θ anticlockwise from the positive x-axis.
pub(crate) fn on_circle(c: Pos2, r: f32, theta: f64) -> Pos2 {
    c + vec2(theta.cos() as f32, -(theta.sin() as f32)) * r
}

/// A point of the plane (x, y in radii) on screen.
fn at_xy(c: Pos2, r: f32, x: f64, y: f64) -> Pos2 {
    c + vec2(x as f32 * r, -(y as f32) * r)
}

/// What a click on a point does: a rest gets the base note, a note becomes a rest.
pub(crate) fn clicked_point(value: f32) -> f32 {
    if value.round() > 0.0 {
        0.0
    } else {
        1.0
    }
}

/// The functions shown at first: sin and cos, what plays, what turns the wheel.
pub(crate) fn shown_at_first(params: &[f32]) -> u8 {
    let mut bits = 0b11u8 | playing(params);
    for (param, offset) in [(uc::OCTAVE_FROM, 1usize), (uc::MOD_FROM, 1)] {
        let w = params.get(param).map_or(0, |v| v.round().max(0.0) as usize);
        if w >= offset && w - offset < 6 {
            bits |= 1 << (w - offset);
        }
    }
    bits
}

/// The functions that play, as bits (sin first).
pub(crate) fn playing(params: &[f32]) -> u8 {
    let v = uc::voices(params);
    (0..6).filter(|f| v[f + 1]).fold(0u8, |m, f| m | 1 << f)
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
    let shown_id = id.with("circle_shown");
    let mut shown: u8 = ui.ctx().memory(|m| m.data.get_temp(shown_id)).unwrap_or_else(|| shown_at_first(params));
    // Fill the points, turn them round; which functions to see.
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
        ui.add_space(8.0);
        // What plays: a tick each; what is shown: its button.
        let mut points = params[uc::PLAY_POINTS] >= 0.5;
        if ui.checkbox(&mut points, RichText::new("Points").small()).on_hover_text("The points' own notes play").changed() {
            params[uc::PLAY_POINTS] = if points { 1.0 } else { 0.0 };
            if !points && params[uc::PITCH] >= 1.0 {
                params[uc::PITCH] = 0.0;
            }
            changed = true;
        }
        for (f, name) in uc::FUNCTIONS.iter().enumerate() {
            let mut plays = uc::voices(params)[f + 1];
            if ui.checkbox(&mut plays, "").on_hover_text(format!("{name} θ plays: its own melody, with the others ticked")).changed() {
                params[uc::PLAY_POINTS + 1 + f] = if plays { 1.0 } else { 0.0 };
                // An old set's Pitch gives way to the switches.
                if params[uc::PITCH].round() as usize == f + 1 {
                    params[uc::PITCH] = 0.0;
                }
                if plays {
                    shown |= 1 << f;
                }
                changed = true;
            }
            let on = shown & (1 << f) != 0;
            let text = RichText::new(*name).small().color(if on { COLORS[f] } else { TEXT_DIM });
            let button = egui::Button::new(text).fill(if on { PANEL3 } else { Color32::TRANSPARENT }).stroke(Stroke::new(1.0_f32, if on { COLORS[f] } else { LINE }));
            if ui.add(button).on_hover_text(format!("Show {name} θ on the circle and its wave")).clicked() {
                shown ^= 1 << f;
            }
        }
        ui.label(RichText::new("Click a point: a note or a rest. Drag it up or down: which note").small().color(TEXT_DIM));
    });
    ui.ctx().memory_mut(|m| m.data.insert_temp(shown_id, shown));

    let size = (height - 22.0).max(150.0);
    let r = size / 2.0 - 30.0;
    let gap = 18.0;
    let span = TAU as f32 * r;
    let width = size + gap + span + 8.0;
    let (rect, _) = ui.allocate_exact_size(vec2(width, size), Sense::hover());
    rect_fill(ui.painter(), rect, 3.0, KNOB);
    let p = ui.painter().with_clip_rect(rect);
    let c = pos2(rect.left() + size / 2.0, rect.center().y + 8.0);
    // Where now is on the waves: they run on to the right from here.
    let x0 = c.x + r + gap;

    let period = uc::period_beats(params);
    let frac = live.on().map_or(0.0, |v| (v[1] & 0xffff) as f64 / 65535.0);
    let theta = uc::theta_at(params, frac);

    // The axes, the circle, the line of now.
    p.line_segment([pos2(rect.left(), c.y), pos2(rect.right(), c.y)], Stroke::new(1.0_f32, LINE));
    p.line_segment([pos2(c.x, c.y - r - 14.0), pos2(c.x, c.y + r + 14.0)], Stroke::new(1.0_f32, LINE));
    p.line_segment([pos2(x0, c.y - r - 6.0), pos2(x0, c.y + r + 6.0)], Stroke::new(1.0_f32, LINE));
    p.circle_stroke(c, r, Stroke::new(2.0_f32, COLORS[1]));

    // The waves, the oldest furthest to the right.
    let wave_y = |f: usize, th: f64| c.y - r * uc::value(f + 1, th) as f32;
    for f in (0..6).filter(|f| shown & (1 << f) != 0) {
        let mut line: Vec<Pos2> = Vec::new();
        let mut last_y: Option<f32> = None;
        for i in 0..=240 {
            let u = i as f64 / 240.0;
            let q = pos2(x0 + u as f32 * span, wave_y(f, uc::theta_at(params, frac - u)));
            // At an asymptote the line breaks.
            if last_y.is_some_and(|y| (y - q.y).abs() > r) && line.len() > 1 {
                p.add(Shape::line(std::mem::take(&mut line), Stroke::new(2.0_f32, COLORS[f])));
                line.clear();
            }
            last_y = Some(q.y);
            line.push(q);
        }
        if line.len() > 1 {
            p.add(Shape::line(line, Stroke::new(2.0_f32, COLORS[f])));
        }
    }

    // Where the points played: on each wave that plays (else on sin's).
    let plays = playing(params);
    let follow: Vec<usize> = if plays == 0 { vec![0] } else { (0..6).filter(|f| plays & (1 << f) != 0).collect() };
    let now = frac * period;
    let mut played = Vec::new();
    let pieces = 160;
    for i in 0..pieces {
        let b0 = now - period * (i + 1) as f64 / pieces as f64;
        let b1 = now - period * i as f64 / pieces as f64;
        uc::passes(params, b0, b1, &mut |beat, k, _| played.push((beat, k)));
    }
    for (beat, k) in played {
        if uc::point_note(params, k).is_some() {
            let u = ((now - beat) / period) as f32;
            let th = uc::theta_at_beat(params, beat);
            for &f in &follow {
                p.circle_filled(pos2(x0 + u * span, wave_y(f, th)), 3.5, Color32::WHITE);
            }
        }
    }

    // On the circle: P, its radius, and each function shown, tied to its wave.
    let at = on_circle(c, r, theta);
    let (s, co) = (theta.sin(), theta.cos());
    p.line_segment([c, at], Stroke::new(1.5_f32, TEXT_DIM));
    let thick = |f: usize| Stroke::new(3.0_f32, COLORS[f]);
    let tie = |f: usize, from: Pos2| {
        let to = pos2(x0, wave_y(f, theta));
        p.line_segment([from, to], Stroke::new(1.0_f32, COLORS[f].gamma_multiply(0.55)));
    };
    let near = |v: f64| v.is_finite() && v.abs() < 2.3;
    if shown & 1 != 0 {
        p.line_segment([pos2(at.x, c.y), at], thick(0));
        tie(0, at);
    }
    if shown & 2 != 0 {
        p.line_segment([c, pos2(at.x, c.y)], thick(1));
        tie(1, at_xy(c, r, 0.0, co));
    }
    let t = s / co;
    if shown & 4 != 0 && near(t) {
        let touch = at_xy(c, r, 1.0, t);
        p.line_segment([at_xy(c, r, 1.0, 0.0), touch], thick(2));
        p.line_segment([c, touch], Stroke::new(1.0_f32, COLORS[2].gamma_multiply(0.6)));
        tie(2, touch);
    }
    // The tangent at P meets the x-axis at sec θ and the y-axis at csc θ.
    let (sec, csc, cot) = (1.0 / co, 1.0 / s, co / s);
    if shown & 8 != 0 && near(sec) {
        let end = at_xy(c, r, sec, 0.0);
        p.line_segment([c, end], Stroke::new(3.0_f32, COLORS[3]));
        p.line_segment([at, end], Stroke::new(1.0_f32, COLORS[3].gamma_multiply(0.6)));
        tie(3, at_xy(c, r, 0.0, sec));
    }
    if shown & 16 != 0 && near(csc) {
        let end = at_xy(c, r, 0.0, csc);
        p.line_segment([c, end], Stroke::new(3.0_f32, COLORS[4]));
        p.line_segment([at, end], Stroke::new(1.0_f32, COLORS[4].gamma_multiply(0.6)));
        tie(4, end);
    }
    if shown & 32 != 0 && near(cot) {
        let touch = at_xy(c, r, cot, 1.0);
        p.line_segment([at_xy(c, r, 0.0, 1.0), touch], thick(5));
        tie(5, at_xy(c, r, 0.0, cot));
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
            let d = ((theta + TAU * ph as f64 / phases as f64) - a).rem_euclid(TAU);
            d.min(TAU - d) < 0.2
        });
        let fill = match (note.is_some(), hit) {
            (true, true) => Color32::WHITE,
            (true, false) => accent_now(),
            _ => KNOB,
        };
        p.circle_filled(dot, if hit { 6.0 } else { 4.0 }, fill);
        p.circle_stroke(dot, 4.0, Stroke::new(1.0_f32, accent_now()));
        let label_at = on_circle(c, r + 17.0, a);
        let hit_r = Rect::from_center_size(label_at, vec2(28.0, 16.0));
        let (resp, moved) = drag_steps(ui, hit_r, id.with(("point", k)), &mut params[slot], 0.0, 15.0, 8.0);
        changed |= moved;
        if resp.clicked() {
            params[slot] = clicked_point(params[slot]);
            changed = true;
        }
        let label = note.map_or_else(|| "-".to_string(), |st| degree_name(&live, params[uc::BASE], st));
        if resp.hovered() {
            p.rect_filled(hit_r, 3.0, shade(accent_now(), 0.35));
        }
        text(&p, label_at, Align2::CENTER_CENTER, &label, small(), if note.is_some() { TEXT } else { TEXT_DIM });
        resp.on_hover_text(format!("θ{k} = {a:.2}: click for a note or a rest, drag up or down for which note"));
    }
    // P, and the other phases.
    for ph in 1..phases {
        p.circle_filled(on_circle(c, r, theta + TAU * ph as f64 / phases as f64), 5.0, Color32::from_gray(0x30));
    }
    p.circle_filled(at, 7.0, Color32::BLACK);
    p.circle_stroke(at, 7.0, Stroke::new(1.5_f32, Color32::WHITE));

    // The values of those shown, in their box.
    // Fewer decimals when there are many.
    let count = (0..6).filter(|f| shown & (1 << f) != 0).count();
    let digits = if count > 3 { 2 } else { 3 };
    let mut values = format!("θ = {:.*}", digits, theta.rem_euclid(TAU));
    for (f, name) in uc::FUNCTIONS.iter().enumerate() {
        if shown & (1 << f) != 0 {
            let v = [s, co, t, sec, csc, cot][f];
            let v = if v.is_finite() && v.abs() < 1e4 { format!("{:.*}", digits, v + 0.0) } else { "∞".to_string() };
            values.push_str(&format!("  {name} θ = {v}"));
        }
    }
    let galley = p.layout_no_wrap(values, small(), Color32::BLACK);
    let box_w = (galley.size().x + 14.0).min(width - 12.0);
    // Over the waves, but never past the edge.
    let cx = (x0 + span * 0.5).clamp(rect.left() + box_w * 0.5 + 6.0, rect.right() - box_w * 0.5 - 6.0);
    let box_r = Rect::from_center_size(pos2(cx, rect.top() + 12.0), vec2(box_w, 18.0));
    p.rect_filled(box_r, 2.0, BOX);
    p.rect_stroke(box_r, 2.0, Stroke::new(1.0_f32, Color32::BLACK), egui::StrokeKind::Inside);
    p.galley(box_r.center() - galley.size() * 0.5, galley, Color32::BLACK);
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

    #[test]
    fn sin_cos_and_what_the_notes_follow_are_shown_at_first() {
        let mut p = krabton_engine::params::defaults(uc::CIRCLE);
        assert_eq!(shown_at_first(&p), 0b11);
        p[uc::PLAY_POINTS + 3] = 1.0; // tan θ plays
        p[uc::MOD_FROM] = 6.0; // cot θ
        assert_eq!(shown_at_first(&p), 0b100111);
        assert_eq!(playing(&p), 0b100);
    }
}

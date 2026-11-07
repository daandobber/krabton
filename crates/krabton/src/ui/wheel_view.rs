//! The Wheel's editor: the wheel with its cars, the rider on it, and beside it
//! the rider's height over a turn (the unit circle's sine, bent by gravity) with
//! a tick where each car plays: the rushes and the hesitations, to see.

use super::seq_more_view::{degree_name, drag_steps};
use super::seq_view::Live;
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, Id, Rect, Sense, Shape, Stroke, Ui};
use krabton_engine::sequencers::wheel as wh;
use std::f64::consts::TAU;

const CAR: egui::Vec2 = egui::Vec2::new(30.0, 18.0);

/// A point on the wheel: `angle` clockwise from the top.
fn on_wheel(c: egui::Pos2, r: f32, angle: f64) -> egui::Pos2 {
    c + vec2(angle.sin() as f32, -(angle.cos() as f32)) * r
}

/// What a click on a car does: an empty one gets the base note, else it empties.
pub(crate) fn clicked_car(value: f32) -> f32 {
    if value.round() > 0.0 {
        0.0
    } else {
        1.0
    }
}

pub(crate) fn wheel(ui: &mut Ui, id: Id, params: &mut [f32], live: Live, height: f32) -> bool {
    if params.len() < wh::WHEEL.len() {
        return false;
    }
    let size = height.max(150.0);
    let plot_w = (size * 1.4).max(200.0);
    let (rect, _) = ui.allocate_exact_size(vec2(size + plot_w + 12.0, size), Sense::hover());
    rect_fill(ui.painter(), rect, 3.0, KNOB);
    let p = ui.painter().clone();
    let c = pos2(rect.left() + size / 2.0, rect.center().y);
    let r = size / 2.0 - CAR.y - 4.0;
    let n = wh::cars(params);
    let vis = live.on();
    let rider = vis.map(|v| (v[0] & 0xffff) as f64 / 65535.0 * TAU);
    let along = vis.map(|v| (v[1] & 0xffff) as f64 / 65535.0);
    let accent = accent_now();

    // The wheel: its rim, its spokes, its hub.
    p.circle_stroke(c, r, Stroke::new(2.0_f32, shade(accent, 0.55)));
    for k in 0..n {
        p.line_segment([c, on_wheel(c, r, wh::car_angle(params, k))], Stroke::new(1.0_f32, LINE));
    }
    p.circle_filled(c, 4.0, shade(accent, 0.7));
    // Swinging, how far it goes either way.
    if params[wh::MOTION] >= 0.5 {
        let reach = params[wh::REACH].clamp(0.05, 1.0) as f64 * std::f64::consts::PI;
        for side in [-1.0, 1.0] {
            let a = std::f64::consts::PI + side * reach;
            p.line_segment([c, on_wheel(c, r + 6.0, a)], Stroke::new(1.0_f32, shade(accent, 0.4)));
        }
    }

    let mut changed = false;
    for k in 0..n {
        let a = wh::car_angle(params, k);
        let at = on_wheel(c, r, a);
        // A car hangs below where it is held, the right way up.
        let car = Rect::from_center_size(at + vec2(0.0, CAR.y * 0.5), CAR);
        p.line_segment([at, car.center_top()], Stroke::new(1.0_f32, TEXT_DIM));
        let slot = wh::FIRST_CAR + k;
        let note = wh::car_note(params, k);
        // Lit as the rider has just gone past it.
        let hit = rider.is_some_and(|ra| {
            let d = (ra - a).rem_euclid(TAU);
            d < 0.22 || d > TAU - 0.04
        }) && note.is_some();
        let (resp, moved) = drag_steps(ui, car, id.with(("car", k)), &mut params[slot], 0.0, 15.0, 8.0);
        changed |= moved;
        if resp.clicked() {
            params[slot] = clicked_car(params[slot]);
            changed = true;
        }
        let fill = match (note, hit) {
            (Some(_), true) => Color32::WHITE,
            (Some(_), false) => accent,
            (None, _) => SLOT,
        };
        p.rect_filled(car, 4.0, fill);
        if resp.hovered() {
            p.rect_stroke(car, 4.0, Stroke::new(1.0_f32, Color32::WHITE), egui::StrokeKind::Outside);
        }
        let label = note.map_or_else(|| "-".to_string(), |s| degree_name(&live, params[wh::BASE], s));
        text(&p, car.center(), Align2::CENTER_CENTER, &label, small(), if note.is_some() { Color32::BLACK } else { TEXT_DIM });
        resp.on_hover_text("Click: a note or empty. Drag up or down: which note");
    }
    if let Some(a) = rider {
        p.circle_filled(on_wheel(c, r, a), 6.0, Color32::BLACK);
        p.circle_stroke(on_wheel(c, r, a), 6.0, Stroke::new(1.5_f32, Color32::WHITE));
    }

    // The rider's height over a turn, and where the cars play on it.
    let plot = Rect::from_min_max(pos2(rect.left() + size + 8.0, rect.top() + 16.0), pos2(rect.right() - 8.0, rect.bottom() - 22.0));
    let mid = plot.center().y;
    p.line_segment([pos2(plot.left(), mid), pos2(plot.right(), mid)], Stroke::new(1.0_f32, LINE));
    p.line_segment([plot.left_top(), plot.left_bottom()], Stroke::new(1.0_f32, LINE));
    let height_at = |frac: f64| mid - (wh::angle_at(params, frac).cos() as f32) * plot.height() * 0.5;
    let points: Vec<egui::Pos2> = (0..=160).map(|i| {
        let f = i as f64 / 160.0;
        pos2(plot.left() + f as f32 * plot.width(), height_at(f))
    }).collect();
    p.add(Shape::line(points, Stroke::new(2.0_f32, Color32::from_rgb(0xe8, 0x4a, 0x3c))));
    let turn = wh::turn_beats(params);
    let mut ticks = Vec::new();
    let chunks = (turn * 50.0).round() as usize;
    for i in 0..chunks {
        wh::passes(params, i as f64 / 50.0, (i + 1) as f64 / 50.0, &mut |at, k| ticks.push((at / turn, k)));
    }
    for (f, k) in ticks {
        let x = plot.left() + f as f32 * plot.width();
        let on = wh::car_note(params, k).is_some();
        p.line_segment([pos2(x, plot.bottom() + 4.0), pos2(x, plot.bottom() + 14.0)], Stroke::new(if on { 2.0_f32 } else { 1.0_f32 }, if on { accent } else { LINE }));
    }
    if let Some(f) = along {
        let at = pos2(plot.left() + f as f32 * plot.width(), height_at(f));
        p.circle_filled(at, 5.0, Color32::BLACK);
        p.circle_stroke(at, 5.0, Stroke::new(1.5_f32, Color32::WHITE));
    }
    text(&p, plot.left_top() + vec2(4.0, -12.0), Align2::LEFT_TOP, "height over a turn", small(), TEXT_DIM);
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_click_fills_an_empty_car_and_empties_a_full_one() {
        assert_eq!(clicked_car(0.0), 1.0);
        assert_eq!(clicked_car(5.0), 0.0);
    }

    #[test]
    fn the_top_of_the_wheel_is_up() {
        let c = pos2(100.0, 100.0);
        let top = on_wheel(c, 50.0, 0.0);
        assert!((top.y - 50.0).abs() < 1e-3 && (top.x - 100.0).abs() < 1e-3);
        let right = on_wheel(c, 50.0, TAU / 4.0);
        assert!((right.x - 150.0).abs() < 1e-3, "clockwise");
    }
}

//! The Video's controls, as a VJ desk has them: value bars (the name and
//! the value inside; drag along to set, Shift for fine, double-click for
//! where it starts), choices side by side when they are few or as tiles
//! when they are many, tiles to add things from, and the Visuals' scenes
//! and colours drawn small.

use crate::theme::*;
use egui::{pos2, vec2, Align2, Color32, CornerRadius, Painter, Pos2, Rect, Response, RichText, Sense, Shape, Stroke, Ui, Vec2};
use std::f32::consts::TAU;

/// How tall a bar or a chip is.
pub(crate) const H: f32 = 20.0;

/// A value bar: `label` on the left inside it, `text` of the value on the
/// right, filled as far as the value is (from 0 when the range has it in the
/// middle). `live`: where what moves it is now (a thin line under it).
#[allow(clippy::too_many_arguments)]
pub(crate) fn bar(ui: &mut Ui, value: &mut f32, lo: f32, hi: f32, default: f32, label: &str, text: impl Fn(f32) -> String, width: f32, live: Option<f32>, tip: &str) -> Response {
    let w = if width > 0.0 { width } else { ui.available_width() };
    let (r, mut resp) = ui.allocate_exact_size(vec2(w.max(24.0), H), Sense::click_and_drag());
    let span = (hi - lo).max(1e-6);
    let before = *value;
    if resp.dragged() {
        let fine = if ui.input(|i| i.modifiers.shift) { 0.1 } else { 1.0 };
        *value = (*value + resp.drag_delta().x / r.width() * span * fine).clamp(lo, hi);
    } else if resp.double_clicked() {
        *value = default.clamp(lo, hi);
    } else if resp.clicked() {
        if let Some(p) = resp.interact_pointer_pos() {
            *value = lo + ((p.x - r.left()) / r.width()).clamp(0.0, 1.0) * span;
        }
    }
    if *value != before {
        resp.mark_changed();
    }
    let p = ui.painter();
    let hot = resp.hovered() || resp.dragged();
    p.rect_filled(r, 3.0, if hot { PANEL3.gamma_multiply(1.25) } else { PANEL3 });
    let at = |v: f32| r.left() + ((v - lo) / span).clamp(0.0, 1.0) * r.width();
    let zero = if lo < 0.0 && hi > 0.0 { at(0.0) } else { r.left() };
    let x = at(*value);
    let fill = Rect::from_min_max(pos2(zero.min(x), r.top()), pos2(zero.max(x), r.bottom()));
    p.rect_filled(fill, 3.0, ACCENT.gamma_multiply(if hot { 0.75 } else { 0.55 }));
    p.line_segment([pos2(x, r.top() + 2.0), pos2(x, r.bottom() - 2.0)], Stroke::new(2.0_f32, ACCENT));
    if let Some(v) = live {
        let l = Rect::from_min_size(pos2(r.left(), r.bottom() - 2.0), vec2(r.width() * v.clamp(0.0, 1.0), 2.0));
        p.rect_filled(l, 1.0, TEXT);
    }
    let clip = p.with_clip_rect(r.shrink(1.0));
    if !label.is_empty() {
        clip.text(pos2(r.left() + 6.0, r.center().y), Align2::LEFT_CENTER, label, small(), TEXT);
    }
    clip.text(pos2(r.right() - 6.0, r.center().y), Align2::RIGHT_CENTER, text(*value), small(), TEXT);
    if hot {
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
    }
    resp.on_hover_text(format!("{tip}\nDrag along (Shift: finer), click to put it there, double-click: as it starts"))
}

/// A fader standing up, in colour `col`: filled to its value, lit up to
/// `live` (where the music takes it now). Drag up and down (Shift: finer),
/// click to put it there, double-click: `default`. `dim`: not in use.
#[allow(clippy::too_many_arguments)]
pub(crate) fn fader(ui: &mut Ui, value: &mut f32, live: f32, default: f32, size: Vec2, col: Color32, dim: bool, tip: &str) -> Response {
    let (r, mut resp) = ui.allocate_exact_size(size, Sense::click_and_drag());
    let before = *value;
    if resp.dragged() {
        let fine = if ui.input(|i| i.modifiers.shift) { 0.1 } else { 1.0 };
        *value = (*value - resp.drag_delta().y / r.height() * fine).clamp(0.0, 1.0);
    } else if resp.double_clicked() {
        *value = default;
    } else if resp.clicked() {
        if let Some(p) = resp.interact_pointer_pos() {
            *value = ((r.bottom() - p.y) / r.height()).clamp(0.0, 1.0);
        }
    }
    if *value != before {
        resp.mark_changed();
    }
    let p = ui.painter();
    let hot = resp.hovered() || resp.dragged();
    let track = Rect::from_center_size(r.center(), vec2(r.width().min(14.0), r.height()));
    p.rect_filled(track, 3.0, if hot { PANEL3.gamma_multiply(1.3) } else { PANEL3 });
    let y = |v: f32| track.bottom() - v.clamp(0.0, 1.0) * track.height();
    let base = if dim { col.gamma_multiply(0.35) } else { col.gamma_multiply(0.75) };
    if live > *value + 0.005 {
        p.rect_filled(Rect::from_min_max(pos2(track.left(), y(live)), pos2(track.right(), track.bottom())), 3.0, col.gamma_multiply(0.35));
    }
    p.rect_filled(Rect::from_min_max(pos2(track.left(), y(*value)), pos2(track.right(), track.bottom())), 3.0, base);
    if live > 0.005 && !dim {
        p.line_segment([pos2(track.left(), y(live)), pos2(track.right(), y(live))], Stroke::new(1.5_f32, col));
    }
    // The handle.
    let hy = y(*value);
    let handle = Rect::from_center_size(pos2(r.center().x, hy), vec2(r.width(), 6.0));
    p.rect_filled(handle, 2.0, if hot { TEXT } else if dim { TEXT_DIM } else { Color32::from_gray(0xd8) });
    if hot {
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
    }
    resp.on_hover_text(format!("{tip}
{:.0} %. Drag up or down (Shift: finer), double-click: back", *value * 100.0))
}

/// A percentage.
pub(crate) fn pct(v: f32) -> String {
    format!("{:.0} %", v * 100.0)
}

/// Few short choices side by side, as one row of buttons. Whether it changed.
pub(crate) fn segmented<T: PartialEq + Clone>(ui: &mut Ui, value: &mut T, options: &[(T, String)], tip: &str) -> bool {
    let mut changed = false;
    let n = options.len();
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.x = 1.0;
        ui.spacing_mut().button_padding = vec2(7.0, 2.0);
        for (k, (v, name)) in options.iter().enumerate() {
            let on = *value == *v;
            let (l, r) = (if k == 0 { 4 } else { 0 }, if k + 1 == n { 4 } else { 0 });
            let b = egui::Button::new(RichText::new(name).small().color(if on { TEXT_DARK } else { TEXT })).fill(if on { ACCENT } else { PANEL3 }).corner_radius(CornerRadius { nw: l, sw: l, ne: r, se: r }).min_size(vec2(0.0, H));
            if ui.add(b).on_hover_text(tip).clicked() && !on {
                *value = v.clone();
                changed = true;
            }
        }
    });
    changed
}

/// A chip showing the choice; clicked, the choices as tiles. Whether it changed.
pub(crate) fn chip<T: PartialEq + Clone>(ui: &mut Ui, id: impl std::hash::Hash, width: f32, value: &mut T, options: &[(T, String)], tip: &str) -> bool {
    let shown = options.iter().find(|o| o.0 == *value).map_or("?", |o| o.1.as_str()).to_string();
    let b = egui::Button::new(RichText::new(shown).small().color(TEXT)).fill(PANEL3).corner_radius(4.0);
    let resp = ui.add_sized([width, H], b).on_hover_text(tip);
    // A small triangle: it opens.
    let c = pos2(resp.rect.right() - 8.0, resp.rect.center().y);
    ui.painter().add(Shape::convex_polygon(vec![c + vec2(-3.5, -1.5), c + vec2(3.5, -1.5), c + vec2(0.0, 2.5)], TEXT_DIM, Stroke::NONE));
    let mut changed = false;
    let cols = if options.len() > 9 { 3 } else if options.len() > 3 { 2 } else { 1 };
    egui::Popup::menu(&resp).show(|ui| {
        egui::Grid::new(egui::Id::new(&id).with("tiles")).spacing([3.0, 3.0]).show(ui, |ui| {
            for (k, (v, name)) in options.iter().enumerate() {
                let on = *value == *v;
                let t = egui::Button::new(RichText::new(name).small().color(if on { TEXT_DARK } else { TEXT })).fill(if on { ACCENT } else { PANEL3 }).corner_radius(3.0);
                if ui.add_sized([112.0, 24.0], t).clicked() {
                    if !on {
                        *value = v.clone();
                        changed = true;
                    }
                    ui.close();
                }
                if (k + 1) % cols == 0 {
                    ui.end_row();
                }
            }
        });
    });
    changed
}

/// One of `options`: side by side when they are few and short, else a chip of tiles.
pub(crate) fn pick<T: PartialEq + Clone>(ui: &mut Ui, id: impl std::hash::Hash, width: f32, value: &mut T, options: &[(T, String)], tip: &str) -> bool {
    let chars: usize = options.iter().map(|o| o.1.chars().count()).sum();
    if options.len() <= 4 && chars <= 22 {
        segmented(ui, value, options, tip)
    } else {
        chip(ui, id, width, value, options, tip)
    }
}

/// A tile to click: a picture drawn by `draw` over its name. `on`: lit.
pub(crate) fn tile(ui: &mut Ui, size: Vec2, on: bool, label: &str, tip: &str, draw: impl FnOnce(&Painter, Rect, Color32)) -> Response {
    let (r, resp) = ui.allocate_exact_size(size, Sense::click());
    let p = ui.painter();
    let hot = resp.hovered();
    p.rect_filled(r, 4.0, if on { ACCENT.gamma_multiply(0.3) } else if hot { PANEL3.gamma_multiply(1.3) } else { PANEL3 });
    if on || hot {
        p.rect_stroke(r, 4.0, Stroke::new(1.0_f32, if on { ACCENT } else { LINE }), egui::StrokeKind::Inside);
    }
    let label_h = if label.is_empty() { 0.0 } else { 14.0 };
    let pic = Rect::from_min_max(r.min + vec2(4.0, 4.0), pos2(r.right() - 4.0, r.bottom() - 2.0 - label_h));
    draw(&p.with_clip_rect(pic), pic, if on { ACCENT } else { TEXT });
    if !label.is_empty() {
        p.with_clip_rect(r).text(pos2(r.center().x, r.bottom() - 8.0), Align2::CENTER_CENTER, label, small(), if on { TEXT } else { TEXT_DIM });
    }
    resp.on_hover_text(tip)
}

/// A tile that is only a word (to add an effect, a preset).
pub(crate) fn word_tile(ui: &mut Ui, label: &str, enabled: bool, tip: &str) -> Response {
    let b = egui::Button::new(RichText::new(label).small().color(if enabled { TEXT } else { TEXT_DIM })).fill(if enabled { PANEL3 } else { PANEL2 }).corner_radius(3.0).min_size(vec2(0.0, 22.0));
    ui.add_enabled(enabled, b).on_hover_text(tip)
}

/// A number 0..1 that looks random, the same for the same `n`.
fn h(n: u32) -> f32 {
    let mut x = n.wrapping_mul(0x9E37_79B9) ^ 0x85EB_CA6B;
    x ^= x >> 15;
    x = x.wrapping_mul(0x2C1B_3C6D);
    x ^= x >> 12;
    (x >> 8) as f32 / 16_777_216.0
}

fn line(p: &Painter, pts: Vec<Pos2>, c: Color32, w: f32) {
    p.add(Shape::line(pts, Stroke::new(w, c)));
}

/// The Visuals' scene `k` drawn small in `r` (see `VISUAL_SCENES`).
pub(crate) fn scene_icon(p: &Painter, r: Rect, k: usize, c: Color32) {
    let at = |x: f32, y: f32| pos2(r.left() + x * r.width(), r.top() + y * r.height());
    let dim = c.gamma_multiply(0.5);
    match k {
        // Ridges: lines rolling, the nearer higher in the middle.
        0 => {
            for i in 0..5 {
                let y = 0.2 + i as f32 * 0.17;
                let pts = (0..=16).map(|s| {
                    let x = s as f32 / 16.0;
                    at(x, y - (-(x - 0.5).powi(2) * 18.0).exp() * (0.1 + i as f32 * 0.04) * (1.0 + (x * 20.0 + i as f32).sin() * 0.3))
                });
                line(p, pts.collect(), if i > 2 { c } else { dim }, 1.0);
            }
        }
        // Lattice: beams going away to a middle.
        1 => {
            let m = r.center();
            let inner = Rect::from_center_size(m, r.size() * 0.3);
            for (a, b) in [(r.left_top(), inner.left_top()), (r.right_top(), inner.right_top()), (r.left_bottom(), inner.left_bottom()), (r.right_bottom(), inner.right_bottom())] {
                p.line_segment([a, b], Stroke::new(1.0_f32, dim));
            }
            for s in [1.0, 0.65, 0.3] {
                p.rect_stroke(Rect::from_center_size(m, r.size() * s), 0.0, Stroke::new(1.0_f32, if s < 0.5 { dim } else { c }), egui::StrokeKind::Inside);
            }
        }
        // Swarm: a ball of points.
        2 => {
            let rad = r.height() * 0.42;
            for i in 0..70 {
                let (a, d) = (h(i) * TAU, h(i + 99).sqrt() * rad);
                p.circle_filled(r.center() + vec2(a.cos() * d, a.sin() * d), 0.9, if h(i + 7) > 0.5 { c } else { dim });
            }
        }
        // Data: blocks and a bar.
        3 => {
            for x in 0..10 {
                for y in 0..5 {
                    if h(x * 13 + y) > 0.55 {
                        let b = Rect::from_min_size(at(x as f32 / 10.0 + 0.01, y as f32 / 8.0 + 0.02), vec2(r.width() / 10.0 - 2.0, r.height() / 8.0 - 2.0));
                        p.rect_filled(b, 0.0, dim);
                    }
                }
                let hgt = h(x + 50) * 0.35;
                p.rect_filled(Rect::from_min_max(at(x as f32 / 10.0 + 0.01, 1.0 - hgt), at((x + 1) as f32 / 10.0 - 0.01, 1.0)), 0.0, c);
            }
        }
        // Tunnel: rings and spokes.
        4 => {
            for i in 1..=4 {
                p.circle_stroke(r.center(), r.height() * 0.11 * i as f32, Stroke::new(1.0_f32, if i % 2 == 0 { c } else { dim }));
            }
            for i in 0..8 {
                let a = i as f32 / 8.0 * TAU;
                p.line_segment([r.center() + vec2(a.cos(), a.sin()) * r.height() * 0.1, r.center() + vec2(a.cos(), a.sin()) * r.width() * 0.6], Stroke::new(1.0_f32, dim));
            }
        }
        // Scope: the wave and a spectrum.
        5 => {
            line(p, (0..=40).map(|s| at(s as f32 / 40.0, 0.4 + (s as f32 * 0.6).sin() * 0.2 * (s as f32 * 0.13).cos())).collect(), c, 1.2);
            for x in 0..20 {
                let hgt = (0.25 - (x as f32 - 4.0).abs() * 0.012).max(0.03) * (0.6 + h(x) * 0.6);
                p.rect_filled(Rect::from_min_max(at(x as f32 / 20.0, 1.0 - hgt), at((x as f32 + 0.6) / 20.0, 1.0)), 0.0, dim);
            }
        }
        // Contours: lines round a hill.
        6 => {
            for i in 1..=4 {
                let s = i as f32 * 0.11;
                let pts = (0..=32).map(|t| {
                    let a = t as f32 / 32.0 * TAU;
                    let w = 1.0 + (a * 3.0 + i as f32).sin() * 0.15;
                    r.center() + vec2(a.cos() * r.width() * s * 1.3, a.sin() * r.height() * s * 1.6) * w
                });
                line(p, pts.collect(), if i % 2 == 0 { c } else { dim }, 1.0);
            }
        }
        // Shards: cells.
        7 => {
            let pts: Vec<Pos2> = (0..9).map(|i| at(0.1 + h(i) * 0.8, 0.1 + h(i + 20) * 0.8)).collect();
            for (i, a) in pts.iter().enumerate() {
                let mut near: Vec<&Pos2> = pts.iter().filter(|b| *b != a).collect();
                near.sort_by(|x, y| (**x - *a).length().total_cmp(&(**y - *a).length()));
                for b in near.iter().take(2) {
                    p.line_segment([*a, **b], Stroke::new(1.0_f32, if i % 3 == 0 { c } else { dim }));
                }
            }
        }
        // Note Grid: cells, a few lit.
        8 => {
            for x in 0..6 {
                for y in 0..4 {
                    let b = Rect::from_min_size(at(x as f32 / 6.0, y as f32 / 4.0), vec2(r.width() / 6.0 - 2.0, r.height() / 4.0 - 2.0));
                    if h(x * 7 + y * 3) > 0.75 {
                        p.rect_filled(b, 1.0, c);
                    } else {
                        p.rect_stroke(b, 1.0, Stroke::new(1.0_f32, dim), egui::StrokeKind::Inside);
                    }
                }
            }
        }
        // Strings: lines, one shaking.
        9 => {
            for i in 0..5 {
                let y = 0.15 + i as f32 * 0.17;
                let shake = if i == 2 { 0.12 } else { 0.0 };
                line(p, (0..=30).map(|s| at(s as f32 / 30.0, y + (s as f32 / 30.0 * TAU * 3.0).sin() * shake * (s as f32 / 30.0 * std::f32::consts::PI).sin())).collect(), if i == 2 { c } else { dim }, 1.0);
            }
        }
        // Milk: a picture inside itself, turned a little each time.
        _ => {
            let m = r.center();
            for i in 0..7 {
                let s = 0.95f32.powi(i * 2) * r.height() * 0.48;
                let a = i as f32 * 0.22;
                let pts: Vec<Pos2> = (0..=4).map(|k| {
                    let t = a + k as f32 / 4.0 * TAU + TAU / 8.0;
                    m + vec2(t.cos() * s * 1.5, t.sin() * s)
                }).collect();
                line(p, pts, if i % 2 == 0 { c } else { dim }, 1.0);
            }
            line(p, (0..=40).map(|s| at(s as f32 / 40.0, 0.5 + (s as f32 * 0.7).sin() * 0.12)).collect(), c, 1.0);
        }
    }
}

/// The Visuals' colours `k` (see `VISUAL_PALETTES`) along `t` (as the shader has them); `own` for its own colour.
pub(crate) fn palette(k: usize, t: f32, own: Color32) -> Color32 {
    let mix = |a: [f32; 3], b: [f32; 3]| Color32::from_rgb(((a[0] + (b[0] - a[0]) * t) * 255.0) as u8, ((a[1] + (b[1] - a[1]) * t) * 255.0) as u8, ((a[2] + (b[2] - a[2]) * t) * 255.0) as u8);
    match k {
        0 => own,
        1 => mix([0.15, 0.55, 1.0], [0.85, 0.97, 1.0]),
        2 => mix([1.0, 0.22, 0.04], [1.0, 0.86, 0.35]),
        3 => mix([1.0, 0.1, 0.72], [0.1, 0.9, 1.0]),
        4 => mix([0.3, 1.0, 0.1], [0.95, 1.0, 0.25]),
        _ => {
            let c = |o: f32| ((0.5 + 0.5 * (TAU * (t + o)).cos()) * 255.0) as u8;
            Color32::from_rgb(c(0.0), c(0.33), c(0.67))
        }
    }
}

/// A swatch of colours `k` in `r`.
pub(crate) fn swatch(p: &Painter, r: Rect, k: usize, own: Color32) {
    let n = 12;
    for i in 0..n {
        let x0 = r.left() + r.width() * i as f32 / n as f32;
        let x1 = r.left() + r.width() * (i + 1) as f32 / n as f32 + 0.5;
        p.rect_filled(Rect::from_min_max(pos2(x0, r.top()), pos2(x1, r.bottom())), 0.0, palette(k, i as f32 / (n - 1) as f32, own));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_colours_run_as_the_shader_has_them() {
        assert_eq!(palette(0, 0.3, Color32::RED), Color32::RED, "its own colour");
        assert_eq!(palette(2, 0.0, Color32::RED), Color32::from_rgb(255, 56, 10), "Ember starts red");
        assert_ne!(palette(5, 0.0, Color32::RED), palette(5, 0.5, Color32::RED), "Spectrum runs round");
    }

    #[test]
    fn a_hash_is_the_same_every_time_and_spread() {
        assert_eq!(h(5), h(5));
        let avg = (0..1000).map(h).sum::<f32>() / 1000.0;
        assert!((avg - 0.5).abs() < 0.05, "{avg}");
    }
}

//! Drawing a skin: the panel, the logo, the knobs and the groups.

use super::{Case, Font, Groups, Knob, Skin, Surface};
use crate::theme::{mix, shade, small, text_on};
use crate::widgets::{arc_points, KNOB_RANGE, KNOB_START};
use egui::epaint::Mesh;
use egui::{pos2, vec2, Align2, Color32, Painter, Pos2, Rect, Response, Sense, Shape, Stroke, Ui};

/// The width of wooden cheeks.
pub const CHEEK: f32 = 12.0;
/// The height of a line of colour across the top.
pub const STRIPE: f32 = 3.0;

fn lum(c: Color32) -> f32 {
    0.299 * c.r() as f32 + 0.587 * c.g() as f32 + 0.114 * c.b() as f32
}

/// Whether a skin's panel is light (and its ink dark).
pub(crate) fn light(s: &Skin) -> bool {
    lum(mix(s.top, s.bottom, 0.5)) > 140.0
}

/// A small deterministic hash, for the grain of materials.
fn hash(mut x: u32) -> u32 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x7feb_352d);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846c_a68b);
    x ^ (x >> 16)
}

fn alpha(c: Color32, a: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), a)
}

/// The outline of a rounded rectangle, clockwise.
fn rounded(rect: Rect, r: f32) -> Vec<Pos2> {
    let r = r.min(rect.width() * 0.5).min(rect.height() * 0.5);
    let mut pts = Vec::with_capacity(28);
    let corners = [
        (pos2(rect.right() - r, rect.top() + r), -0.5f32),
        (pos2(rect.right() - r, rect.bottom() - r), 0.0),
        (pos2(rect.left() + r, rect.bottom() - r), 0.5),
        (pos2(rect.left() + r, rect.top() + r), 1.0),
    ];
    for (c, start) in corners {
        for k in 0..=6 {
            let a = (start + k as f32 / 12.0) * std::f32::consts::PI;
            pts.push(c + vec2(a.cos(), a.sin()) * r);
        }
    }
    pts
}

/// A filled convex outline, its colour by `col` per point.
fn fan(pts: &[Pos2], centre: Pos2, centre_col: Color32, col: impl Fn(Pos2) -> Color32) -> Mesh {
    let mut m = Mesh::default();
    m.colored_vertex(centre, centre_col);
    for p in pts {
        m.colored_vertex(*p, col(*p));
    }
    let n = pts.len() as u32;
    for i in 0..n {
        m.add_triangle(0, 1 + i, 1 + (i + 1) % n);
    }
    m
}

/// A rounded rectangle shaded from `top` to `bottom`.
fn gradient(rect: Rect, r: f32, top: Color32, bottom: Color32) -> Mesh {
    let at = |y: f32| mix(top, bottom, ((y - rect.top()) / rect.height().max(1.0)).clamp(0.0, 1.0));
    fan(&rounded(rect, r), rect.center(), at(rect.center().y), |p| at(p.y))
}

/// A rounded rectangle shaded from `middle` out to `rim`.
fn radial(rect: Rect, r: f32, middle: Color32, rim: Color32) -> Vec<Shape> {
    let outer = rounded(rect, r);
    let c = rect.center();
    let half = mix(middle, rim, 0.55);
    let inner: Vec<Pos2> = outer.iter().map(|p| c + (*p - c) * 0.55).collect();
    let mut ring = Mesh::default();
    let n = outer.len() as u32;
    for (o, i) in outer.iter().zip(&inner) {
        ring.colored_vertex(*o, rim);
        ring.colored_vertex(*i, half);
    }
    for k in 0..n {
        let (a, b) = (2 * k, 2 * ((k + 1) % n));
        ring.add_triangle(a, a + 1, b);
        ring.add_triangle(b, a + 1, b + 1);
    }
    vec![Shape::mesh(ring), Shape::mesh(fan(&inner, c, middle, |_| half))]
}

/// Wood grain in `rect`: wavy lines along it (across if `vertical`).
fn wood_grain(rect: Rect, base: Color32, vertical: bool, seed: u32, out: &mut Vec<Shape>) {
    let (len, across) = if vertical { (rect.height(), rect.width()) } else { (rect.width(), rect.height()) };
    let n = (across / 3.2).ceil() as usize;
    let dark = shade(base, 0.62);
    // Light wood carries dark print: its grain stays fainter.
    let soft = if lum(base) > 140.0 { 0.55 } else { 1.0 };
    for k in 0..n {
        let h = hash(seed.wrapping_add(k as u32 * 977));
        let off = k as f32 * 3.2 + (h % 100) as f32 / 100.0 * 2.0;
        let f1 = 0.011 + (h >> 8 & 15) as f32 * 0.0011;
        let ph = (h >> 12 & 255) as f32 / 40.0;
        let amp = 0.6 + (h >> 20 & 7) as f32 * 0.35;
        let a = ((40 + (h >> 4 & 63)) as f32 * soft) as u8;
        let steps = (len / 14.0).ceil().max(2.0) as usize;
        let pts: Vec<Pos2> = (0..=steps)
            .map(|i| {
                let t = i as f32 / steps as f32 * len;
                let w = off + (t * f1 + ph).sin() * amp + (t * f1 * 3.1 + ph * 2.0).sin() * amp * 0.35;
                if vertical {
                    pos2(rect.left() + w.clamp(0.0, across), rect.top() + t)
                } else {
                    pos2(rect.left() + t, rect.top() + w.clamp(0.0, across))
                }
            })
            .collect();
        out.push(Shape::line(pts, Stroke::new(1.0_f32, alpha(dark, a))));
    }
}

/// A screw head at `c`.
fn screw(c: Pos2, seed: u32, out: &mut Vec<Shape>) {
    out.push(Shape::circle_filled(c + vec2(0.0, 0.8), 3.6, Color32::from_black_alpha(90)));
    out.push(Shape::circle_filled(c, 3.4, Color32::from_rgb(0x8a, 0x8d, 0x91)));
    out.push(Shape::circle_filled(c - vec2(0.6, 0.6), 2.4, Color32::from_rgb(0xc4, 0xc7, 0xca)));
    let a = (hash(seed) % 628) as f32 / 100.0;
    let d = vec2(a.cos(), a.sin()) * 2.6;
    out.push(Shape::line_segment([c - d, c + d], Stroke::new(1.0_f32, Color32::from_rgb(0x3a, 0x3c, 0x3f))));
}

/// The panel of a device in skin `s`, for the frame at `rect`; `track` is
/// the colour of its track (a strip on the left edge). `ppp` is the screen's
/// pixels per point: the texture keeps to whole pixels from the panel's
/// corner and comes from the look, not from where the panel is, so it holds
/// still while the view moves.
pub fn panel(rect: Rect, s: &Skin, track: Option<Color32>, ppp: f32) -> Shape {
    let r = 5.0;
    let mut out = Vec::new();
    let ppp = ppp.max(0.5);
    let px = 1.0 / ppp;
    let snap = |v: f32| (v * ppp).round() / ppp;
    let seed = hash(u32::from_le_bytes(s.top.to_array()) ^ u32::from_le_bytes(s.bottom.to_array()).rotate_left(11));
    // A shadow under it.
    out.push(Shape::rect_filled(rect.translate(vec2(0.0, 2.0)), r, Color32::from_black_alpha(70)));
    match s.surface {
        Surface::Burst | Surface::Velvet => out.extend(radial(rect, r, s.top, s.bottom)),
        _ => out.push(Shape::mesh(gradient(rect, r, s.top, s.bottom))),
    }
    let inner = rect.shrink2(vec2(if s.cheeks.is_some() { CHEEK } else { 2.0 }, 2.0));
    // Where the texture starts, on a pixel; its size in pixels.
    let (x0, y0) = (snap(inner.left()), snap(inner.top()));
    let (w_px, h_px) = ((inner.width() * ppp).max(1.0) as u32, (inner.height() * ppp).max(1.0) as u32);
    match s.surface {
        Surface::Brushed => {
            let mut row = 0u32;
            let mut k = 0u32;
            while row < h_px {
                let h = hash(k ^ seed);
                let a = 7 + (h % 10) as u8;
                let c = if h & 32 != 0 { Color32::from_white_alpha(a + 2) } else { Color32::from_black_alpha(a + 8) };
                let y = y0 + (row as f32 + 0.5) * px;
                out.push(Shape::line_segment([pos2(x0, y), pos2(inner.right(), y)], Stroke::new(px, c)));
                row += 2 + (h >> 8 & 1);
                k += 1;
            }
        }
        Surface::Grain => {
            let mut m = Mesh::default();
            let n = ((w_px as f32 * h_px as f32) / (70.0 * ppp * ppp)) as u32;
            for k in 0..n.min(6000) {
                let hx = hash(k.wrapping_mul(2654435761) ^ seed);
                let hy = hash(hx ^ 0x9e37_79b9);
                let (dx, dy) = (hx % w_px, hy % h_px);
                let c = if hx & 1 == 0 { Color32::from_white_alpha(16) } else { Color32::from_black_alpha(34) };
                m.add_colored_rect(Rect::from_min_size(pos2(x0 + dx as f32 * px, y0 + dy as f32 * px), vec2(px, px)), c);
            }
            out.push(Shape::mesh(m));
        }
        Surface::Carbon => {
            let h = inner.height();
            let mut k = 0u32;
            loop {
                let x = x0 + (k * 3) as f32 * px;
                if x >= inner.right() + h {
                    break;
                }
                // The line x - (y - top), cut to the panel.
                let y_a = (y0 + x - inner.right()).max(y0);
                let y_b = (y0 + x - x0).min(inner.bottom());
                if y_b > y_a {
                    let at = |y: f32| pos2(x - (y - y0), y);
                    let c = if k % 2 == 0 { Color32::from_black_alpha(55) } else { Color32::from_white_alpha(7) };
                    out.push(Shape::line_segment([at(y_a), at(y_b)], Stroke::new(1.6_f32, c)));
                }
                k += 1;
            }
        }
        Surface::Wood => wood_grain(Rect::from_min_size(pos2(x0, y0), inner.size()), mix(s.top, s.bottom, 0.5), false, seed, &mut out),
        Surface::Scan => {
            let mut row = 0u32;
            while row < h_px {
                let y = y0 + (row as f32 + 0.5) * px;
                out.push(Shape::line_segment([pos2(x0, y), pos2(inner.right(), y)], Stroke::new(px, Color32::from_black_alpha(60))));
                row += (3.0 * ppp).round().max(2.0) as u32;
            }
        }
        Surface::Gloss => {
            let band = Rect::from_min_max(inner.min, pos2(inner.right(), inner.top() + inner.height() * 0.42));
            out.push(Shape::mesh(gradient(band, 3.0, Color32::from_white_alpha(26), Color32::from_white_alpha(0))));
        }
        Surface::Flat | Surface::Burst | Surface::Velvet => {}
    }
    if let Some(wood) = s.cheeks {
        for (k, side) in [Rect::from_min_size(rect.min, vec2(CHEEK, rect.height())), Rect::from_min_size(pos2(rect.right() - CHEEK, rect.top()), vec2(CHEEK, rect.height()))].into_iter().enumerate() {
            out.push(Shape::mesh(gradient(side, if k == 0 { r } else { r }, shade(wood, 1.15), shade(wood, 0.8))));
            let side_px = Rect::from_min_size(pos2(snap(side.left() + 0.5), snap(side.top())), vec2(side.width() - 1.0, side.height()));
            wood_grain(side_px, wood, true, seed ^ (k as u32 + 7), &mut out);
            let edge = if k == 0 { side.right() } else { side.left() };
            out.push(Shape::line_segment([pos2(edge, side.top()), pos2(edge, side.bottom())], Stroke::new(1.0_f32, Color32::from_black_alpha(110))));
        }
    }
    for (k, c) in s.stripes.iter().enumerate() {
        let y = rect.top() + 3.0 + k as f32 * STRIPE;
        out.push(Shape::rect_filled(Rect::from_min_max(pos2(inner.left(), y), pos2(inner.right(), y + STRIPE - 0.8)), 0.0, *c));
    }
    if s.screws {
        let i = if s.cheeks.is_some() { CHEEK + 6.0 } else { 7.0 };
        let corners = [pos2(rect.left() + i, rect.top() + 7.0), pos2(rect.right() - i, rect.top() + 7.0), pos2(rect.left() + i, rect.bottom() - 7.0), pos2(rect.right() - i, rect.bottom() - 7.0)];
        for (k, c) in corners.into_iter().enumerate() {
            screw(pos2(snap(c.x), snap(c.y)), seed ^ k as u32, &mut out);
        }
    }
    // A bevel: light along the top, dark along the bottom, an outline.
    out.push(Shape::line_segment([pos2(rect.left() + r, rect.top() + 0.8), pos2(rect.right() - r, rect.top() + 0.8)], Stroke::new(1.0_f32, Color32::from_white_alpha(if light(s) { 110 } else { 34 }))));
    out.push(Shape::line_segment([pos2(rect.left() + r, rect.bottom() - 0.8), pos2(rect.right() - r, rect.bottom() - 0.8)], Stroke::new(1.0_f32, Color32::from_black_alpha(90))));
    out.push(Shape::rect_stroke(rect, r, Stroke::new(1.0_f32, Color32::from_black_alpha(150)), egui::StrokeKind::Inside));
    if let Some(t) = track {
        out.push(Shape::rect_filled(Rect::from_min_size(pos2(rect.left() + 1.0, rect.top() + r), vec2(3.0, rect.height() - 2.0 * r)), 1.5, t));
    }
    Shape::Vec(out)
}

/// A device's name as its logo; click and right-click like the plain name.
pub fn logo(ui: &mut Ui, text: &str, s: &Skin) -> Response {
    let text = match s.logo_case {
        Case::Upper => text.to_uppercase(),
        Case::Lower => text.to_lowercase(),
        Case::AsIs => text.to_owned(),
    };
    let galley = ui.painter().layout_no_wrap(text, s.logo.id(s.logo_size), s.logo_ink);
    let pad = if s.plate.is_some() { vec2(7.0, 1.5) } else { vec2(1.0, 0.0) };
    let (rect, resp) = ui.allocate_exact_size(galley.size() + pad * 2.0, Sense::click());
    let p = ui.painter();
    if let Some(plate) = s.plate {
        p.rect_filled(rect.translate(vec2(0.0, 1.0)), 3.0, Color32::from_black_alpha(80));
        p.rect_filled(rect, 3.0, plate);
    }
    let pos = rect.min + pad;
    let on_light = match s.plate {
        Some(pl) => lum(pl) > 140.0,
        None => light(s),
    };
    if matches!(s.logo, Font::Neon | Font::Space | Font::Future) && !on_light {
        let glow = alpha(s.logo_ink, 40);
        for d in [vec2(-1.0, 0.0), vec2(1.0, 0.0), vec2(0.0, -1.0), vec2(0.0, 1.0), vec2(0.0, 2.0)] {
            p.galley_with_override_text_color(pos + d, galley.clone(), glow);
        }
    } else {
        // Printed or engraved: a light edge under it on light panels, a shadow on dark ones.
        let edge = if on_light { Color32::from_white_alpha(120) } else { Color32::from_black_alpha(150) };
        p.galley_with_override_text_color(pos + vec2(0.0, 1.0), galley.clone(), edge);
    }
    p.galley(pos, galley, s.logo_ink);
    resp
}

/// The backdrop and name of a group of knobs at `rect` (the name in its top
/// 14 points).
pub fn group_backdrop(p: &Painter, rect: Rect, name: &str, s: &Skin, first: bool) {
    let label = if s.caps { name.to_uppercase() } else { name.to_owned() };
    let font = small();
    match s.groups {
        Groups::Plain => {
            if !first {
                p.line_segment([pos2(rect.left(), rect.top() + 2.0), pos2(rect.left(), rect.bottom() - 2.0)], Stroke::new(1.0_f32, alpha(s.dim, 70)));
            }
            let r = p.text(pos2(rect.left() + 5.0, rect.top() + 6.0), Align2::LEFT_CENTER, label, font, s.ink);
            p.line_segment([pos2(r.left(), r.bottom() + 0.5), pos2(r.left() + 10.0, r.bottom() + 0.5)], Stroke::new(1.5_f32, s.accent));
        }
        Groups::Inset => {
            let r = rect.shrink2(vec2(2.0, 0.5));
            p.rect_filled(r, 4.0, alpha(s.well, if light(s) { 70 } else { 120 }));
            p.rect_stroke(r, 4.0, Stroke::new(1.0_f32, alpha(if light(s) { Color32::WHITE } else { Color32::BLACK }, 60)), egui::StrokeKind::Inside);
            p.text(pos2(r.left() + 5.0, rect.top() + 7.0), Align2::LEFT_CENTER, label, font, s.ink);
        }
        Groups::Header => {
            let bar = Rect::from_min_size(pos2(rect.left() + 2.0, rect.top() + 0.5), vec2(rect.width() - 4.0, 12.0));
            p.rect_filled(Rect::from_min_max(bar.min, pos2(bar.right(), rect.bottom() - 0.5)), 3.0, alpha(s.well, if light(s) { 40 } else { 70 }));
            p.rect_filled(bar, 3.0, s.accent);
            p.text(pos2(bar.left() + 5.0, bar.center().y), Align2::LEFT_CENTER, label, font, text_on(s.accent));
        }
        Groups::Outline => {
            let r = Rect::from_min_max(pos2(rect.left() + 2.0, rect.top() + 6.0), pos2(rect.right() - 2.0, rect.bottom() - 0.5));
            let line = Stroke::new(1.0_f32, alpha(s.dim, 150));
            let galley = p.layout_no_wrap(label, font, s.ink);
            let gap = Rect::from_min_size(pos2(r.left() + 6.0, rect.top()), vec2(galley.size().x + 6.0, 12.0));
            // The outline, broken where the name sits.
            let pts = [pos2(gap.left(), r.top()), pos2(r.left() + 3.0, r.top()), pos2(r.left(), r.top() + 3.0), pos2(r.left(), r.bottom() - 3.0), pos2(r.left() + 3.0, r.bottom()), pos2(r.right() - 3.0, r.bottom()), pos2(r.right(), r.bottom() - 3.0), pos2(r.right(), r.top() + 3.0), pos2(r.right() - 3.0, r.top()), pos2(gap.right(), r.top())];
            p.add(Shape::line(pts.to_vec(), line));
            p.galley(pos2(gap.left() + 3.0, rect.top() + 6.0 - galley.size().y * 0.5), galley, s.ink);
        }
    }
}

/// Where a value fills a knob's arc, as fractions of its travel.
fn span(v: f32, bipolar: bool) -> (f32, f32) {
    if bipolar {
        (0.5f32.min(v), 0.5f32.max(v))
    } else {
        (0.0, v)
    }
}

fn arc(p: &Painter, c: Pos2, r: f32, a: f32, b: f32, w: f32, col: Color32) {
    if b - a > 0.004 {
        p.add(Shape::line(arc_points(c, r, KNOB_START + KNOB_RANGE * a, KNOB_START + KNOB_RANGE * b), Stroke::new(w, col)));
    }
}

fn ticks(p: &Painter, c: Pos2, r0: f32, r1: f32, n: usize, col: Color32) {
    for k in 0..n {
        let a = KNOB_START + KNOB_RANGE * k as f32 / (n - 1) as f32;
        let d = vec2(a.cos(), a.sin());
        p.line_segment([c + d * r0, c + d * r1], Stroke::new(1.0_f32, col));
    }
}

/// A metal disc lit from the top left, lightest `light`.
fn metal(p: &Painter, c: Pos2, r: f32, light: Color32) {
    let dark = shade(light, 0.5);
    for k in 0..6 {
        let t = k as f32 / 5.0;
        let rr = r * (1.0 - t * 0.85);
        p.circle_filled(c - vec2(0.18, 0.22) * r * t, rr, mix(dark, light, 0.25 + t * 0.75));
    }
}

/// A knob in skin `s`: value `v` (0..1), its colour `col`, `hot` under the pointer.
pub fn knob_face(p: &Painter, rect: Rect, v: f32, bipolar: bool, col: Color32, hot: bool, s: &Skin) {
    let c = rect.center();
    let size = rect.width().min(rect.height());
    let r = size * 0.5 - 3.0;
    let ang = KNOB_START + KNOB_RANGE * v;
    let dir = vec2(ang.cos(), ang.sin());
    let (a, b) = span(v, bipolar);
    let track = alpha(s.dim, 70);
    let big = size > 48.0;
    match s.knob {
        Knob::Ring => crate::widgets::ring_face_plain(p, rect, v, bipolar, col, hot),
        Knob::Chrome => {
            arc(p, c, r + 1.5, 0.0, 1.0, 2.0, track);
            arc(p, c, r + 1.5, a, b, 2.0, col);
            let rb = r - 2.0;
            p.circle_filled(c + vec2(0.0, 1.5), rb + 1.0, Color32::from_black_alpha(110));
            p.circle_filled(c, rb, shade(s.cap, 0.42));
            let n = if big { 40 } else { 24 };
            for k in 0..n {
                let t = k as f32 / n as f32 * std::f32::consts::TAU;
                let d = vec2(t.cos(), t.sin());
                p.line_segment([c + d * (rb - if big { 4.0 } else { 2.5 }), c + d * rb], Stroke::new(1.0_f32, if k % 2 == 0 { shade(s.cap, 0.75) } else { shade(s.cap, 0.3) }));
            }
            let rt = rb - if big { 5.0 } else { 3.0 };
            metal(p, c, rt, s.cap);
            let pc = if hot { col } else { Color32::from_rgb(0x22, 0x22, 0x24) };
            p.line_segment([c + dir * rt * 0.25, c + dir * (rt - 1.0)], Stroke::new(if big { 3.0_f32 } else { 2.0 }, pc));
        }
        Knob::Vintage => {
            ticks(p, c, r + 0.5, r + 3.0, 11, s.dim);
            arc(p, c, r - 0.5, a, b, 1.5, alpha(col, 200));
            let rs = r - 2.5;
            p.circle_filled(c + vec2(0.0, 1.5), rs + 0.5, Color32::from_black_alpha(120));
            p.circle_filled(c, rs, s.cap);
            p.circle_stroke(c, rs - 1.0, Stroke::new(1.0_f32, Color32::from_white_alpha(18)));
            let rt = rs * 0.6;
            metal(p, c, rt, Color32::from_rgb(0xd9, 0xdb, 0xde));
            let pc = if hot { col } else { Color32::from_rgb(0xf4, 0xf0, 0xe6) };
            p.line_segment([c + dir * (rt + 1.0), c + dir * (rs - 0.5)], Stroke::new(if big { 3.0_f32 } else { 2.2 }, pc));
        }
        Knob::Cap => {
            arc(p, c, r + 1.0, 0.0, 1.0, 2.5, track);
            arc(p, c, r + 1.0, a, b, 2.5, col);
            let rb = r - 2.0;
            p.circle_filled(c + vec2(0.0, 1.5), rb + 0.5, Color32::from_black_alpha(110));
            p.circle_filled(c, rb, Color32::from_rgb(0x17, 0x17, 0x19));
            let rc = rb * 0.74;
            p.circle_filled(c, rc, s.cap);
            p.circle_filled(c - vec2(0.25, 0.3) * rc, rc * 0.45, alpha(Color32::WHITE, 30));
            let pc = if hot { col } else { text_on(s.cap) };
            p.line_segment([c + dir * rc * 0.2, c + dir * (rb - 0.5)], Stroke::new(if big { 3.0_f32 } else { 2.0 }, pc));
        }
        Knob::Neon => {
            p.circle_filled(c, r + 1.0, Color32::from_black_alpha(90));
            arc(p, c, r - 1.0, 0.0, 1.0, 3.0, alpha(col, 45));
            arc(p, c, r - 1.0, a, b, 9.0, alpha(col, 30));
            arc(p, c, r - 1.0, a, b, 6.0, alpha(col, 70));
            arc(p, c, r - 1.0, a, b, 3.0, mix(col, Color32::WHITE, if hot { 0.7 } else { 0.35 }));
            p.circle_filled(c, r - 6.0, Color32::from_rgb(0x08, 0x09, 0x0e));
            p.line_segment([c + dir * (r - 6.0) * 0.3, c + dir * (r - 7.0)], Stroke::new(2.0_f32, col));
            p.circle_filled(c + dir * (r - 1.0), if big { 3.5 } else { 2.5 }, Color32::WHITE);
        }
        Knob::Chicken => {
            ticks(p, c, r - 0.5, r + 2.5, 11, s.dim);
            arc(p, c, r + 1.0, a, b, 1.5, alpha(col, 220));
            let rs = r * 0.62;
            p.circle_filled(c + vec2(0.0, 1.5), rs + 1.0, Color32::from_black_alpha(110));
            p.circle_filled(c, rs, s.cap);
            let perp = vec2(-dir.y, dir.x);
            let head = vec![c + dir * (r - 1.5), c + perp * rs * 0.55 + dir * rs * 0.2, c + perp * rs * 0.7 - dir * rs * 0.5, c - dir * rs * 0.9, c - perp * rs * 0.7 - dir * rs * 0.5, c - perp * rs * 0.55 + dir * rs * 0.2];
            p.add(Shape::convex_polygon(head, s.cap, Stroke::new(1.0_f32, Color32::from_black_alpha(120))));
            p.line_segment([c, c + dir * (r - 3.0)], Stroke::new(if big { 2.5_f32 } else { 1.6 }, if hot { col } else { text_on(s.cap) }));
        }
        Knob::Euro => {
            ticks(p, c, r + 0.5, r + 2.5, 7, s.dim);
            arc(p, c, r - 0.5, a, b, 1.5, col);
            let rb = r * 0.8;
            p.circle_filled(c + vec2(0.0, 1.2), rb + 0.5, Color32::from_black_alpha(100));
            p.circle_filled(c, rb, shade(s.cap, 0.82));
            let n = if big { 30 } else { 18 };
            for k in 0..n {
                let t = k as f32 / n as f32 * std::f32::consts::TAU;
                p.circle_filled(c + vec2(t.cos(), t.sin()) * (rb - 0.5), 0.9, shade(s.cap, 0.55));
            }
            p.circle_filled(c, rb * 0.78, s.cap);
            let pc = if hot { col } else if lum(s.cap) > 140.0 { Color32::from_rgb(0x20, 0x20, 0x22) } else { Color32::WHITE };
            p.line_segment([c + dir * rb * 0.15, c + dir * (rb - 1.0)], Stroke::new(if big { 3.0_f32 } else { 1.8 }, pc));
        }
        Knob::Pixel => {
            let n = if big { 15 } else { 9 };
            let q = if big { 5.0 } else { 3.5 };
            for k in 0..n {
                let t = k as f32 / (n - 1) as f32;
                let at = KNOB_START + KNOB_RANGE * t;
                let pos = c + vec2(at.cos(), at.sin()) * (r - q * 0.3);
                let lit = t >= a - 0.001 && t <= b + 0.001 && b - a > 0.004;
                let block = Rect::from_center_size(pos2(pos.x.round(), pos.y.round()), vec2(q, q));
                p.rect_filled(block, 0.0, if lit { col } else { alpha(s.dim, 60) });
            }
            let body = Rect::from_center_size(c, vec2(r * 1.05, r * 1.05));
            p.rect_filled(body.translate(vec2(0.0, 1.5)), 1.0, Color32::from_black_alpha(110));
            p.rect_filled(body, 1.0, s.cap);
            p.rect_stroke(body, 1.0, Stroke::new(1.0_f32, shade(s.cap, 1.4)), egui::StrokeKind::Inside);
            let dot = c + dir * r * 0.32;
            p.rect_filled(Rect::from_center_size(pos2(dot.x.round(), dot.y.round()), vec2(q, q)), 0.0, if hot { Color32::WHITE } else { text_on(s.cap) });
        }
        Knob::Led => {
            let n = if big { 21 } else { 13 };
            for k in 0..n {
                let t = k as f32 / (n - 1) as f32;
                let at = KNOB_START + KNOB_RANGE * t;
                let pos = c + vec2(at.cos(), at.sin()) * (r + 0.5);
                let lit = t >= a - 0.001 && t <= b + 0.001 && b - a > 0.004;
                if lit {
                    p.circle_filled(pos, if big { 3.6 } else { 2.6 }, alpha(col, 60));
                }
                p.circle_filled(pos, if big { 1.9 } else { 1.4 }, if lit { mix(col, Color32::WHITE, 0.25) } else { alpha(s.dim, 70) });
            }
            let rr = r - if big { 6.0 } else { 4.5 };
            p.circle_filled(c + vec2(0.0, 1.5), rr + 0.5, Color32::from_black_alpha(110));
            p.circle_filled(c, rr, shade(s.cap, 0.85));
            p.circle_filled(c - vec2(0.12, 0.16) * rr, rr * 0.86, s.cap);
            let pc = if hot { col } else { text_on(s.cap) };
            p.line_segment([c + dir * rr * 0.45, c + dir * (rr - 1.5)], Stroke::new(if big { 3.0_f32 } else { 2.0 }, pc));
        }
    }
}

//! A wind chime drawn from the simulation's state: the top disk with its
//! hook, the tubes hanging in a ring on their strings (seen a little from
//! above, so the ones at the back are higher and darker), the clapper among
//! them and the sail below. Tubes lean as they swing and shimmer while they
//! ring; the clapper is where it is when it strikes, so what you see is what
//! you hear. Used by the Chimes sequencer and the Wind Chimes instrument.

use crate::theme::*;
use egui::{pos2, vec2, Align2, Color32, FontId, Painter, Pos2, Rect, Shape, Stroke};
use krabton_engine::dsp::chime_sim::{self, MAX_TUBES};

/// What the chime looks like.
pub(crate) struct ChimeLook {
    pub shape: chime_sim::Shape,
    pub color: Color32,
    /// 0..1: how thick the tubes are.
    pub thickness: f32,
    pub names: Vec<String>,
}

/// Where everything is now.
#[derive(Default)]
pub(crate) struct ChimeState {
    pub clapper: [f32; 2],
    pub wind: f32,
    pub dir: f32,
    pub tube: [[f32; 2]; MAX_TUBES],
    /// 0..1: how strongly each tube rings.
    pub ring: [f32; MAX_TUBES],
}

pub(crate) fn draw(p: &Painter, rect: Rect, look: &ChimeLook, st: &ChimeState, t: f32) {
    let s = &look.shape;
    let n = s.tubes.clamp(1, MAX_TUBES);
    let cx = rect.center().x;
    let r = (rect.width() * 0.27).min(rect.height() * 0.3);
    let squash = 0.28;
    let y0 = rect.top() + 26.0;
    let string = 12.0;
    // From the bottom up: the sail, its string, the longest tube.
    let sail_size = 0.6 + 0.8 * s.sail.clamp(0.0, 1.0);
    let sail_y = rect.bottom() - 10.0 - 26.0 * sail_size;
    let longest = (sail_y - 16.0 - y0 - string).max(30.0);
    let proj = |x: f32, z: f32, y: f32| pos2(cx + x * r, y + z * r * squash);
    // Wind: streaks drifting the way it blows.
    let wx = st.dir.cos();
    let streaks = (st.wind * 16.0) as usize;
    for k in 0..streaks {
        let seed = (k as f32 * 12.9898).sin() * 43_758.547;
        let fy = seed - seed.floor();
        let speed = 0.25 + 0.5 * st.wind;
        let ph = ((t * speed + fy * 7.0) * wx.signum()).rem_euclid(1.0);
        let x = rect.left() + ph * rect.width();
        let y = rect.top() + 16.0 + fy * (rect.height() - 30.0);
        let len = (10.0 + 18.0 * st.wind) * wx.abs().max(0.3);
        p.line_segment([pos2(x, y), pos2(x + len * wx.signum(), y)], Stroke::new(1.0_f32, Color32::from_white_alpha((14.0 + 40.0 * st.wind.min(1.0)) as u8)));
    }
    // The hook and the top disk.
    let wood = Color32::from_rgb(0x8a, 0x5c, 0x36);
    p.line_segment([pos2(cx, rect.top() + 4.0), pos2(cx, y0 - 6.0)], Stroke::new(1.5_f32, TEXT_DIM));
    p.circle_stroke(pos2(cx, rect.top() + 6.0), 3.0, Stroke::new(1.5_f32, TEXT_DIM));
    for k in 0..4 {
        let a = std::f32::consts::FRAC_PI_4 + k as f32 * std::f32::consts::FRAC_PI_2;
        p.line_segment([pos2(cx, y0 - 6.0), proj(a.cos() * 0.9, a.sin() * 0.9, y0)], Stroke::new(1.0_f32, TEXT_DIM.gamma_multiply(0.6)));
    }
    let disk = vec2(r * 1.1, r * 1.1 * squash);
    let draw_disk = || {
        p.add(Shape::ellipse_filled(pos2(cx, y0 + 4.0), disk, shade(wood, 0.6)));
        p.add(Shape::rect_filled(Rect::from_min_max(pos2(cx - disk.x, y0), pos2(cx + disk.x, y0 + 4.0)), 0.0, shade(wood, 0.6)));
        p.add(Shape::ellipse_filled(pos2(cx, y0), disk, wood));
        p.add(Shape::ellipse_stroke(pos2(cx, y0), disk, Stroke::new(1.0_f32, shade(wood, 1.3))));
    };
    let mut disk_drawn = false;
    // Everything that hangs, back to front.
    enum Item {
        Tube(usize),
        Clapper,
    }
    let mut items: Vec<(f32, Item)> = (0..n).map(|i| (s.rest(i)[1] + st.tube[i][1], Item::Tube(i))).collect();
    items.push((st.clapper[1], Item::Clapper));
    items.sort_by(|a, b| a.0.total_cmp(&b.0));
    let tube_w = 4.0 + 6.0 * look.thickness.clamp(0.0, 1.0);
    let clapper_y = y0 + string + longest * 0.42;
    for (depth, item) in items {
        // The disk hides the tops of the tubes behind it.
        if !disk_drawn && depth > -0.2 {
            draw_disk();
            disk_drawn = true;
        }
        // The back is darker.
        let light = 0.6 + 0.4 * ((depth + 1.2) / 2.4).clamp(0.0, 1.0);
        match item {
            Item::Tube(i) => {
                let rest = s.rest(i);
                let d = st.tube[i];
                let len = s.lengths[i].clamp(0.15, 1.0) * longest;
                let a = proj(rest[0], rest[1], y0);
                let top = proj(rest[0] + d[0] * 0.3, rest[1] + d[1] * 0.3, y0 + string);
                let bottom = proj(rest[0] + d[0] * 1.7, rest[1] + d[1] * 1.7, y0 + string + len);
                p.line_segment([a, top], Stroke::new(1.0_f32, TEXT_DIM.gamma_multiply(light)));
                let ring = st.ring[i].clamp(0.0, 1.0);
                let base = mix(shade(look.color, light), Color32::WHITE, 0.55 * ring);
                // Ringing: the tube shivers.
                let wobble = if ring > 0.02 { ring * 1.6 * (t * 70.0 + i as f32 * 1.7).sin() } else { 0.0 };
                let top = top + vec2(wobble * 0.3, 0.0);
                let bottom = bottom + vec2(wobble, 0.0);
                p.line_segment([top, bottom], Stroke::new(tube_w, base));
                p.circle_filled(bottom, tube_w * 0.5, base);
                p.add(Shape::ellipse_filled(top, vec2(tube_w * 0.5, tube_w * 0.18), shade(base, 0.6)));
                // The shine along it.
                let side = vec2(-tube_w * 0.22, 0.0);
                p.line_segment([top + side, bottom + side], Stroke::new((tube_w * 0.18).max(1.0), mix(base, Color32::WHITE, 0.45).gamma_multiply(0.8)));
                if ring > 0.05 {
                    p.circle_stroke(bottom, tube_w + 10.0 * (1.0 - ring), Stroke::new(1.0_f32, base.gamma_multiply(ring)));
                }
                if let Some(name) = look.names.get(i) {
                    p.text(bottom + vec2(0.0, tube_w * 0.5 + 2.0), Align2::CENTER_TOP, name, FontId::monospace(9.0), if ring > 0.05 { TEXT } else { TEXT_DIM.gamma_multiply(light) });
                }
            }
            Item::Clapper => {
                let c = proj(st.clapper[0], st.clapper[1], clapper_y);
                p.line_segment([pos2(cx, y0 + 2.0), c], Stroke::new(1.0_f32, TEXT_DIM.gamma_multiply(light)));
                let cr = s.clapper_r() * r;
                let col = shade(wood, 0.75 + 0.35 * light);
                let thick = 3.0 + 4.0 * s.clapper.clamp(0.0, 1.0);
                p.add(Shape::ellipse_filled(c + vec2(0.0, thick), vec2(cr, cr * squash), shade(col, 0.6)));
                p.add(egui::Shape::rect_filled(Rect::from_min_max(pos2(c.x - cr, c.y), pos2(c.x + cr, c.y + thick)), 0.0, shade(col, 0.6)));
                p.add(Shape::ellipse_filled(c, vec2(cr, cr * squash), col));
                // The sail below: it hangs further out the way the wind blows.
                let sway = [st.dir.cos() * st.wind * 0.18, st.dir.sin() * st.wind * 0.18];
                let sp = proj(st.clapper[0] * 1.5 + sway[0], st.clapper[1] * 1.5 + sway[1], sail_y);
                p.line_segment([c + vec2(0.0, thick), sp], Stroke::new(1.0_f32, TEXT_DIM));
                sail(p, sp, (sp.x - c.x) / (sail_y - clapper_y).max(1.0), sail_size, shade(wood, 1.15));
            }
        }
    }
    if !disk_drawn {
        draw_disk();
    }
}

/// A leaf-shaped sail hanging from `top`, leaning by `lean` (radians).
fn sail(p: &Painter, top: Pos2, lean: f32, size: f32, color: Color32) {
    let (h, w) = (26.0 * size, 9.0 * size);
    let (sn, cs) = lean.clamp(-0.6, 0.6).sin_cos();
    let rot = |x: f32, y: f32| top + vec2(x * cs + y * sn, -x * sn + y * cs);
    let pts: Vec<Pos2> = (0..=16)
        .map(|k| {
            let a = k as f32 / 16.0 * std::f32::consts::TAU;
            let y = h * 0.5 * (1.0 - a.cos());
            let x = w * a.sin() * (0.35 + 0.65 * (y / h));
            rot(x, y)
        })
        .collect();
    p.add(Shape::convex_polygon(pts, color, Stroke::new(1.0_f32, shade(color, 0.7))));
}

/// The colour of a Wind Chimes material.
pub(crate) fn material_color(m: usize) -> Color32 {
    match m {
        1 => Color32::from_rgb(0xd8, 0xae, 0x52),
        2 => Color32::from_rgb(0xa8, 0xae, 0xb8),
        3 => Color32::from_rgb(0xc4, 0xb0, 0x6c),
        4 => Color32::from_rgb(0x9c, 0xd8, 0xe8),
        5 => Color32::from_rgb(0x9a, 0x62, 0x3c),
        _ => Color32::from_rgb(0xcc, 0xd4, 0xdc),
    }
}

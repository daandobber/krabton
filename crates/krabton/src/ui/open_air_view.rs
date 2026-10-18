//! The Open Air device: the place, with the sound in it. Waves go out from
//! the source while it sounds; trunks light up as a wave reaches them, and
//! in a canyon, over a lake or in a cave the echoes are drawn as they are
//! heard: waves from the mirror images of the source behind each wall, or
//! under the water. Around it the ambience: wind, rain, lightning, crickets
//! and the birds singing. Drag up or down: how far away the sound is.

use super::inst_screen::{caption, SCREEN_BG};
use super::modulation::ModView;
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, Id, Painter, Pos2, Rect, Sense, Shape, Stroke, Ui};
use krabton_engine::dsp::fx::open_air::*;
use krabton_engine::EngineStatus;

const DIM: Color32 = Color32::from_rgb(0x4a, 0x4e, 0x58);
const NIGHT: Color32 = Color32::from_rgb(0x12, 0x16, 0x22);
const TRUNK: Color32 = Color32::from_rgb(0x3a, 0x2e, 0x26);
const LEAF: Color32 = Color32::from_rgb(0x24, 0x40, 0x2c);
const ROCK: Color32 = Color32::from_rgb(0x4a, 0x3c, 0x34);
const WATER: Color32 = Color32::from_rgb(0x1c, 0x3c, 0x58);
const GRASS: Color32 = Color32::from_rgb(0x34, 0x5a, 0x3c);

fn hash(k: f32) -> f32 {
    let s = (k * 12.9898 + 78.233).sin() * 43_758.547;
    s - s.floor()
}

/// The waves sent out: when, and how loud.
#[derive(Clone, Default)]
struct Pulses {
    list: Vec<(f64, f32)>,
    last: f32,
    last_t: f64,
}

/// Distance as metres, roughly: 2 m near, 150 m far.
fn metres(d: f32) -> f32 {
    2.0 * 75f32.powf(d.clamp(0.0, 1.0))
}

#[allow(clippy::too_many_arguments)]
pub fn open_air_panel(ui: &mut Ui, id: Id, dev_id: u64, p: &mut Vec<f32>, accent: Color32, h: f32, mv: Option<&mut ModView>, status: &EngineStatus, width: &mut f32) -> bool {
    if p.len() < SPECS.len() {
        return false;
    }
    let m = status.meters.read(dev_id).unwrap_or([0.0; krabton_engine::scope::METER_VALUES]);
    let mut changed = false;
    ui.horizontal_top(|ui| {
        let (rect, resp) = ui.allocate_exact_size(vec2(*width, (h - 30.0).max(110.0)), Sense::click_and_drag());
        width_grip(ui, id.with("display_width"), rect, width, 240.0, 1200.0);
        let inner = rect.shrink2(vec2(8.0, 20.0));
        if let Some(pos) = resp.interact_pointer_pos().filter(|_| resp.dragged() || resp.clicked()) {
            let d = (((inner.bottom() - pos.y) / inner.height()).clamp(0.0, 1.0) * 100.0).round() / 100.0;
            if p[DISTANCE] != d {
                p[DISTANCE] = d;
                changed = true;
            }
        }
        if resp.hovered() || resp.dragged() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
        }
        draw(ui, id, rect, p, &m, accent);
        resp.on_hover_text("Drag up or down: how far away the sound is. Waves go out while it sounds; the echoes come from the mirror images of the source behind the walls or under the water");
        ui.add_space(6.0);
        changed |= crate::ui::devices::param_grid_mod(ui, id, SPECS, p, accent, h - 24.0, mv);
    });
    changed
}

fn draw(ui: &Ui, id: Id, rect: Rect, p: &[f32], m: &[f32], accent: Color32) {
    let painter = ui.painter_at(rect);
    rect_fill(&painter, rect, 6.0, SCREEN_BG);
    let now = ui.input(|i| i.time);
    let t = now as f32;
    let place = (p[PLACE].round().max(0.0) as usize).min(PLACES.len() - 1);
    let amb = (p[AMBIENCE].round().max(0.0) as usize).min(AMBIENCES.len() - 1);
    let distance = p[DISTANCE].clamp(0.0, 1.0);
    let size = p[SIZE].clamp(0.0, 1.0);
    let r = rect.shrink(1.0);
    let horizon = r.top() + r.height() * 0.36;
    let ground = r.bottom() - 6.0;
    // Where the sound is: near is low and big, far is up by the horizon.
    let src = pos2(r.center().x, ground - 14.0 - (ground - horizon - 24.0) * distance);
    let scale = 1.0 - 0.6 * distance;
    // The sky.
    let flash = m[7];
    for band in 0..5 {
        let y0 = r.top() + band as f32 / 5.0 * (horizon - r.top());
        let c = mix(NIGHT, Color32::from_rgb(0x26, 0x2a, 0x40), band as f32 / 5.0);
        painter.rect_filled(Rect::from_min_max(pos2(r.left(), y0), pos2(r.right(), y0 + (horizon - r.top()) / 5.0 + 1.0)), 0.0, c);
    }
    if flash > 0.05 {
        painter.rect_filled(Rect::from_min_max(r.min, pos2(r.right(), horizon)), 0.0, Color32::from_white_alpha((flash * 80.0) as u8));
    }
    painter.rect_filled(Rect::from_min_max(pos2(r.left(), horizon), r.max), 0.0, mix(SCREEN_BG, GRASS, 0.25));
    // The waves sent out while it sounds.
    let mut pulses: Pulses = ui.data_mut(|d| d.get_temp::<Pulses>(id.with("pulses")).unwrap_or_default());
    let level = m[0];
    if level > 0.02 && (level > pulses.last * 1.25 + 0.01 || now - pulses.last_t > 0.35) && now - pulses.last_t > 0.08 {
        pulses.list.push((now, level.min(1.0)));
        pulses.last_t = now;
    }
    pulses.last = level;
    let speed = if place == 2 { 70.0 } else { 110.0 } * (0.7 + 0.6 * size);
    let max_r = rect.width() * 1.4;
    pulses.list.retain(|(t0, _)| ((now - t0) as f32) * speed < max_r);
    let rings: Vec<(f32, f32)> = pulses.list.iter().map(|&(t0, a)| (((now - t0) as f32) * speed, a)).collect();
    ui.data_mut(|d| d.insert_temp(id.with("pulses"), pulses));
    let ring = |painter: &Painter, at: Pos2, rad: f32, a: f32, color: Color32, squash: f32| {
        if rad > 1.0 && a > 0.01 {
            painter.add(Shape::ellipse_stroke(at, vec2(rad, rad * squash), Stroke::new(1.2_f32, color.gamma_multiply(a.min(1.0)))));
        }
    };
    // How bright a wave is where it passes, at `d` px from where it started.
    let lit = |d: f32| rings.iter().map(|&(rad, a)| a * (-((d - rad) / 12.0).powi(2)).exp()).sum::<f32>().min(1.0);
    match place {
        // Forest: rows of trunks, further rows smaller and darker; a trunk
        // lights as a wave reaches it.
        0 => {
            for row in 0..4 {
                let depth = row as f32 / 3.0;
                let y = horizon + 6.0 + (ground - horizon - 6.0) * (1.0 - depth) * 0.95;
                let n = 7 + row * 3;
                let tw = 7.0 - 4.5 * depth;
                for k in 0..n {
                    let x = r.left() + (k as f32 + 0.3 + 0.4 * hash(k as f32 + row as f32 * 31.0)) / n as f32 * r.width();
                    let top = y - (70.0 - 40.0 * depth) * (0.8 + 0.4 * hash(k as f32 * 3.0 + row as f32));
                    let foot = pos2(x, y);
                    let d = ((foot.x - src.x).powi(2) + ((foot.y - src.y) * 2.0).powi(2)).sqrt();
                    let glow = lit(d) * (0.3 + 0.7 * m[4].min(1.0) * 3.0).min(1.0);
                    let c = mix(shade(TRUNK, 0.5 + 0.5 * (1.0 - depth)), accent, 0.7 * glow);
                    painter.line_segment([pos2(x, top), foot], Stroke::new(tw, c));
                    let sway = (t * 0.7 + k as f32).sin() * (1.0 + 4.0 * m[5]);
                    painter.circle_filled(pos2(x + sway, top), 10.0 - 5.0 * depth, shade(LEAF, 0.6 + 0.5 * (1.0 - depth)));
                }
            }
        }
        // Meadow: open grass, a treeline far off.
        1 => {
            for k in 0..30 {
                let x = r.left() + k as f32 / 30.0 * r.width();
                painter.circle_filled(pos2(x, horizon - 4.0 - 4.0 * hash(k as f32)), 6.0 + 3.0 * hash(k as f32 + 9.0), shade(LEAF, 0.5));
            }
            let tree_glow = lit((src.y - horizon) * 2.0);
            painter.line_segment([pos2(r.left(), horizon), pos2(r.right(), horizon)], Stroke::new(2.0_f32, mix(shade(LEAF, 0.6), accent, 0.6 * tree_glow)));
        }
        // Canyon: rock walls left and right; the echoes come from mirror
        // images of the source behind them.
        2 => {
            let gap = r.width() * (0.18 + 0.2 * size);
            let (wl, wr) = (src.x - gap, src.x + gap);
            painter.add(Shape::convex_polygon(vec![pos2(r.left(), r.top()), pos2(wl, horizon), pos2(wl, ground), pos2(r.left(), r.bottom())], ROCK, Stroke::NONE));
            painter.add(Shape::convex_polygon(vec![pos2(r.right(), r.top()), pos2(r.right(), r.bottom()), pos2(wr, ground), pos2(wr, horizon)], shade(ROCK, 0.85), Stroke::NONE));
            let walls = painter.with_clip_rect(Rect::from_min_max(pos2(wl, r.top()), pos2(wr, r.bottom())));
            for &(rad, a) in &rings {
                // Behind the left wall, behind the right, and twice over.
                for (img, k) in [(2.0 * wl - src.x, 0.6), (2.0 * wr - src.x, 0.6), (2.0 * wl - (2.0 * wr - src.x), 0.35), (2.0 * wr - (2.0 * wl - src.x), 0.35)] {
                    ring(&walls, pos2(img, src.y), rad, a * k, accent, 0.5);
                }
            }
            for x in [wl, wr] {
                painter.line_segment([pos2(x, horizon), pos2(x, ground)], Stroke::new(1.5_f32, mix(shade(ROCK, 1.3), accent, 0.5 * lit((x - src.x).abs()))));
            }
        }
        // Lake: the water below throws the sound back; the far shore answers.
        3 => {
            let water = horizon + (ground - horizon) * 0.25;
            painter.rect_filled(Rect::from_min_max(pos2(r.left(), water), r.max), 0.0, WATER);
            for k in 0..24 {
                let x = r.left() + k as f32 / 24.0 * r.width();
                painter.circle_filled(pos2(x, horizon - 3.0 - 3.0 * hash(k as f32 + 4.0)), 5.0 + 2.0 * hash(k as f32), shade(LEAF, 0.45));
            }
            let shore = lit((src.y - horizon) * 2.0);
            painter.line_segment([pos2(r.left(), horizon), pos2(r.right(), horizon)], Stroke::new(1.5_f32, mix(shade(LEAF, 0.7), accent, 0.7 * shore)));
            // Ripples on the water.
            for k in 0..6 {
                let ph = (t * 0.2 + hash(k as f32)).fract();
                let at = pos2(r.left() + hash(k as f32 + 50.0) * r.width(), water + 6.0 + hash(k as f32 + 60.0) * (ground - water - 10.0));
                painter.add(Shape::ellipse_stroke(at, vec2(4.0 + 14.0 * ph, 1.5 + 4.0 * ph), Stroke::new(1.0_f32, Color32::from_white_alpha(((1.0 - ph) * 40.0) as u8))));
            }
            // The source's mirror image under the water.
            let img = pos2(src.x, 2.0 * water - src.y);
            painter.circle_filled(img, 4.0 * scale, accent.gamma_multiply(0.25));
            let under = painter.with_clip_rect(Rect::from_min_max(pos2(r.left(), water), r.max));
            for &(rad, a) in &rings {
                ring(&under, img, rad, a * 0.5, accent, 0.4);
            }
        }
        // Cave: dark rock all around, close; many echoes at once.
        _ => {
            painter.rect_filled(r, 6.0, Color32::from_rgb(0x14, 0x12, 0x12));
            let arch: Vec<Pos2> = (0..=24)
                .map(|k| {
                    let a = std::f32::consts::PI * k as f32 / 24.0;
                    pos2(r.center().x - a.cos() * r.width() * (0.35 + 0.12 * size), ground - a.sin() * (ground - r.top()) * (0.75 + 0.15 * size))
                })
                .collect();
            for k in 0..24 {
                painter.add(Shape::convex_polygon(vec![arch[k], arch[k + 1], pos2(arch[k + 1].x, ground), pos2(arch[k].x, ground)], Color32::from_rgb(0x22, 0x1e, 0x1c), Stroke::NONE));
            }
            painter.add(Shape::line(arch.clone(), Stroke::new(2.0_f32, mix(ROCK, accent, 0.5 * lit((r.width() * 0.3).max(40.0))))));
            // Stalactites, and drips falling.
            for k in 0..9 {
                let at = arch[3 + k * 2];
                painter.add(Shape::convex_polygon(vec![at + vec2(-3.0, 0.0), at + vec2(3.0, 0.0), at + vec2(0.0, 10.0 + 8.0 * hash(k as f32))], ROCK, Stroke::NONE));
                if amb > 0 {
                    let ph = (t * 0.4 + hash(k as f32 + 3.0)).fract();
                    painter.circle_filled(at + vec2(0.0, 12.0 + ph * (ground - at.y - 12.0)), 1.2, Color32::from_rgb(0x90, 0xb8, 0xd8).gamma_multiply(1.0 - ph * 0.5));
                }
            }
            let inside = painter.with_clip_rect(rect);
            for &(rad, a) in &rings {
                for (dx, k) in [(-1.0f32, 0.4), (1.0, 0.4)] {
                    ring(&inside, pos2(src.x + dx * r.width() * 0.6, src.y), rad, a * k, accent, 0.6);
                }
            }
        }
    }
    // The direct sound's waves, and the source.
    for &(rad, a) in &rings {
        ring(&painter, src, rad, a, accent, if place == 3 { 0.4 } else { 0.55 });
    }
    painter.circle_filled(src, 9.0 * scale + 6.0 * level.min(1.0), accent.gamma_multiply(0.15 + 0.3 * level.min(1.0)));
    painter.circle_filled(src, 4.0 * scale, accent);
    // The ambience: wind, rain, crickets, birds.
    let wind = m[5];
    for s in 0..(wind * 12.0) as usize {
        let fy = hash(s as f32 + 200.0);
        let ph = (t * (0.25 + 0.5 * wind) + fy * 7.0).fract();
        let (x, y) = (r.left() + ph * r.width(), r.top() + 10.0 + fy * (ground - r.top() - 20.0));
        painter.line_segment([pos2(x, y), pos2(x + 10.0 + 16.0 * wind, y)], Stroke::new(1.0_f32, Color32::from_white_alpha((14.0 + 30.0 * wind.min(1.0)) as u8)));
    }
    let rain = m[6];
    for d in 0..(rain * 60.0) as usize {
        let x0 = hash(d as f32 + 300.0);
        let ph = (t * (1.6 + hash(d as f32 + 400.0)) + x0 * 3.0).fract();
        let (x, y) = (r.left() + x0 * r.width(), r.top() + ph * (r.height() - 4.0));
        painter.line_segment([pos2(x, y), pos2(x + 2.0, y + 7.0)], Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(0xa0, 0xc0, 0xe0, 100)));
    }
    if m[8] > 0.01 {
        for c in 0..6 {
            let at = pos2(r.left() + 20.0 + hash(c as f32 + 500.0) * (r.width() - 40.0), ground - 3.0 - 8.0 * hash(c as f32 + 510.0));
            let e = (m[8] * (0.5 + 0.5 * (t * 7.0 + c as f32).sin())).max(0.0);
            painter.circle_filled(at, 1.5 + 3.0 * e, Color32::from_rgb(0xd0, 0xf0, 0x60).gamma_multiply(0.15 + 0.6 * e));
        }
    }
    for b in 0..8 {
        let (x, lv) = (m[15 + 2 * b], m[16 + 2 * b]);
        if lv <= 0.0 {
            continue;
        }
        let at = pos2(r.left() + 14.0 + (0.5 + 0.5 * x.clamp(-1.0, 1.0)) * (r.width() - 28.0), r.top() + 26.0 + 30.0 * hash(b as f32 + 700.0) + (t * 1.5 + b as f32).sin() * 1.5);
        let singing = (lv - 0.001).max(0.0);
        painter.circle_filled(at, 3.0, Color32::from_gray(0x50));
        painter.circle_filled(at + vec2(2.5, -1.5), 1.8, Color32::from_gray(0x50));
        if singing > 0.02 {
            painter.circle_filled(at, 5.0 + 6.0 * singing.min(1.0), accent.gamma_multiply(0.15 + 0.4 * singing.min(1.0)));
            for k in 0..3 {
                let ph = (t * 1.2 + k as f32 * 0.33).fract();
                painter.circle_filled(at + vec2(6.0 + 10.0 * ph, -4.0 - 10.0 * ph), 1.0, accent.gamma_multiply((1.0 - ph) * singing.min(1.0)));
            }
        }
    }
    // Captions.
    let title = if amb > 0 { format!("{} · {}", PLACES[place], AMBIENCES[amb].to_lowercase()) } else { PLACES[place].to_string() };
    caption(&painter, rect.left_top() + vec2(10.0, 6.0), Align2::LEFT_TOP, &title, accent);
    caption(&painter, pos2(rect.right() - 10.0, rect.top() + 6.0), Align2::RIGHT_TOP, &format!("{:>3.0} m away", metres(distance)), DIM);
    if amb > 0 && m[31] < 0.5 {
        caption(&painter, pos2(rect.center().x, rect.bottom() - 6.0), Align2::CENTER_BOTTOM, "the ambience plays while the song plays", DIM);
    }
    ui.ctx().request_repaint_after(std::time::Duration::from_millis(33));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_place_draws() {
        let ctx = egui::Context::default();
        let status = krabton_engine::EngineStatus::new();
        for place in 0..PLACES.len() {
            let mut p = krabton_engine::params::defaults(SPECS);
            p[PLACE] = place as f32;
            p[AMBIENCE] = 2.0;
            let mut width = 400.0;
            for _ in 0..3 {
                let _ = ctx.run(egui::RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1200.0, 800.0))), ..Default::default() }, |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        open_air_panel(ui, Id::new("o"), 1, &mut p, accent_now(), 220.0, None, &status, &mut width);
                    });
                });
            }
        }
    }
}

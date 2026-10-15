//! Screens for the free reeds: the Melodica (its keyboard in a plastic
//! body, the mouthpiece or the tube, the air coming out) and the Harmonica
//! (the harp with its ten holes, the blow note above each and the draw note
//! below, the holes that sound glowing, breath going in or coming out, the
//! bends and overblows marked, the hands cupped around it). They draw what
//! the engine reports, so what you see is what you hear.

use super::inst_screen::{caption, InstLive, SCREEN_BG};
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, FontId, Id, Painter, Pos2, Rect, Sense, Shape, Stroke, Ui};

const DIM: Color32 = Color32::from_rgb(0x4a, 0x4e, 0x58);
const FAINT: Color32 = Color32::from_rgb(0x1e, 0x21, 0x28);
const NAMES: [&str; 12] = ["C", "C#", "D", "Eb", "E", "F", "F#", "G", "Ab", "A", "Bb", "B"];

fn frame(ui: &Ui, rect: Rect) -> Painter {
    let p = ui.painter_at(rect);
    rect_fill(&p, rect, 6.0, SCREEN_BG);
    rect_line(&p, rect, 6.0, 1.0, Color32::from_rgb(0x26, 0x29, 0x31));
    p
}

fn set(p: &mut [f32], i: usize, v: f32) -> bool {
    if p[i] != v {
        p[i] = v;
        true
    } else {
        false
    }
}

/// A steady pseudo-random number for `k` (0..1).
fn hash(k: f32) -> f32 {
    let s = (k * 12.9898 + 78.233).sin() * 43_758.547;
    s - s.floor()
}

/// Dots of breath streaming from `at` along `dir`.
fn breath(p: &Painter, at: Pos2, dir: egui::Vec2, lv: f32, t: f32, seed: f32) {
    for d in 0..8 {
        let ph = (t * 1.8 + hash(d as f32 + seed)).fract();
        let side = vec2(-dir.y, dir.x) * (hash(d as f32 + seed + 1.0) - 0.5) * 10.0;
        let pos = at + dir * (ph * 26.0) + side;
        p.circle_filled(pos, 1.2, Color32::from_white_alpha(((1.0 - ph) * 140.0 * lv.min(1.0)) as u8));
    }
}

/// The air meter at the right edge: how much breath is set, lit by how much flows.
fn air_meter(p: &Painter, rect: Rect, set_to: f32, flow: f32, accent: Color32) {
    let meter = Rect::from_min_size(pos2(rect.right() - 12.0, rect.top() + 26.0), vec2(4.0, rect.height() - 40.0));
    p.rect_filled(meter, 2.0, FAINT);
    let fill = meter.height() * set_to.clamp(0.0, 1.0);
    p.rect_filled(Rect::from_min_max(pos2(meter.left(), meter.bottom() - fill), meter.max), 2.0, accent.gamma_multiply(0.35 + 0.65 * flow.min(1.0)));
}

// ---------------------------------------------------------------- melodica

/// The melodica: an alto's 32 keys (F3 to C6) in a plastic body, the
/// mouthpiece at the left or the tube to the mouth. Drag up or down: the breath.
pub(crate) fn melodica_screen(ui: &mut Ui, id: Id, p: &mut [f32], live: &InstLive, height: f32) -> bool {
    use krabton_engine::dsp::melodica::*;
    let w = 330.0;
    let (rect, _) = ui.allocate_exact_size(vec2(w, height), Sense::hover());
    let resp = ui.interact(rect, id, Sense::click_and_drag());
    let mut changed = false;
    if resp.dragged() {
        let dy = resp.drag_delta().y;
        changed |= set(p, BREATH, ((p[BREATH] - dy * 0.005) * 100.0).round().clamp(0.0, 100.0) / 100.0);
    }
    let painter = frame(ui, rect);
    let accent = accent_now();
    let t = ui.input(|i| i.time) as f32;
    let v = &live.vals;
    let on = v[0] > 0.5;
    let mut sounding = [0.0f32; 128];
    if on {
        for i in 0..8 {
            let lv = v[3 + 2 * i];
            if lv > 0.02 {
                let n = v[2 + 2 * i] as usize;
                if n < 128 {
                    sounding[n] = sounding[n].max(lv.min(1.0));
                }
            }
        }
    }
    let air = if on { v[1] } else { 0.0 };
    // The body.
    let green = Color32::from_rgb(0x3f, 0x8a, 0x6a);
    let body = Rect::from_min_max(pos2(rect.left() + 44.0, rect.center().y - 34.0), pos2(rect.right() - 22.0, rect.center().y + 40.0));
    painter.rect_filled(body, 10.0, shade(green, 0.9));
    painter.rect_filled(Rect::from_min_max(body.min, pos2(body.right(), body.top() + 8.0)), 8.0, mix(green, Color32::WHITE, 0.2));
    // The keys: white, then black over them.
    const WHITE: [usize; 7] = [0, 2, 4, 5, 7, 9, 11];
    let (lo, hi) = (53usize, 84usize);
    let whites: Vec<usize> = (lo..=hi).filter(|n| WHITE.contains(&(n % 12))).collect();
    let keys = Rect::from_min_max(pos2(body.left() + 8.0, body.top() + 14.0), pos2(body.right() - 8.0, body.bottom() - 8.0));
    let kw = keys.width() / whites.len() as f32;
    for (i, &n) in whites.iter().enumerate() {
        let r = Rect::from_min_size(pos2(keys.left() + i as f32 * kw, keys.top()), vec2(kw - 1.0, keys.height()));
        let lv = sounding[n];
        painter.rect_filled(r, 2.0, mix(Color32::from_gray(225), accent, 0.8 * lv));
        if n % 12 == 0 {
            painter.text(pos2(r.center().x, r.bottom() - 3.0), Align2::CENTER_BOTTOM, krabton_engine::note_name(n as u8), FontId::monospace(7.0), Color32::from_gray(90));
        }
    }
    for n in lo..=hi {
        if WHITE.contains(&(n % 12)) {
            continue;
        }
        let left = whites.iter().position(|&w| w == n - 1).unwrap_or(0);
        let r = Rect::from_min_size(pos2(keys.left() + (left as f32 + 1.0) * kw - kw * 0.3, keys.top()), vec2(kw * 0.6, keys.height() * 0.6));
        painter.rect_filled(r, 2.0, mix(Color32::from_gray(30), accent, 0.85 * sounding[n]));
    }
    // The mouthpiece, or the tube.
    let tube = p[TUBE].clamp(0.0, 1.0);
    let mp = pos2(body.left() - 4.0, body.center().y);
    painter.rect_filled(Rect::from_center_size(mp, vec2(10.0, 12.0)), 3.0, Color32::from_gray(200));
    let mouth = if tube > 0.3 {
        // A loop of tube up to the mouth.
        let end = pos2(rect.left() + 16.0, rect.top() + 30.0);
        let pts: Vec<Pos2> = (0..=16)
            .map(|i| {
                let u = i as f32 / 16.0;
                let a = mp.lerp(end, u);
                a + vec2(-22.0 * (u * std::f32::consts::PI).sin(), 0.0)
            })
            .collect();
        painter.add(Shape::line(pts, Stroke::new(4.0_f32, Color32::from_rgba_unmultiplied(220, 230, 235, 120))));
        painter.circle_filled(end, 4.0, Color32::from_gray(200));
        end
    } else {
        pos2(mp.x - 8.0, mp.y)
    };
    if air > 0.02 {
        breath(&painter, mouth, vec2(-0.6, -0.8), air, t, 3.0);
    }
    air_meter(&painter, rect, p[BREATH], air, accent);
    let mode = if p[PLAYING] >= 0.5 { "one breath" } else { "own air for each key" };
    caption(&painter, rect.left_top() + vec2(10.0, 6.0), Align2::LEFT_TOP, "Melodica", accent);
    caption(&painter, pos2(rect.right() - 20.0, rect.top() + 6.0), Align2::RIGHT_TOP, &if tube > 0.3 { format!("{mode}, tube") } else { mode.to_string() }, DIM);
    ui.ctx().request_repaint_after(std::time::Duration::from_millis(33));
    resp.on_hover_text("Drag up or down: the breath. The keys that sound light up");
    changed
}

// ---------------------------------------------------------------- harmonica

/// The harmonica: ten holes, the blow note above and the draw note below.
/// Drag up or down: the hands (cupped closed, or open).
pub(crate) fn harmonica_screen(ui: &mut Ui, id: Id, p: &mut [f32], live: &InstLive, height: f32) -> bool {
    use krabton_engine::dsp::harmonica::*;
    let w = 340.0;
    let (rect, _) = ui.allocate_exact_size(vec2(w, height), Sense::hover());
    let resp = ui.interact(rect, id, Sense::click_and_drag());
    let mut changed = false;
    if resp.dragged() {
        let dy = resp.drag_delta().y;
        changed |= set(p, HANDS, ((p[HANDS] - dy * 0.005) * 100.0).round().clamp(0.0, 100.0) / 100.0);
    }
    let painter = frame(ui, rect);
    let accent = accent_now();
    let t = ui.input(|i| i.time) as f32;
    let v = &live.vals;
    let on = v[0] > 0.5;
    let low = if on && v[1] > 0.0 { v[1] as i32 } else { 60 };
    let holes_mode = !on || v[24] > 0.5;
    let key = if on { v[25] as usize % 12 } else { 0 };
    // The harp.
    let metal = Color32::from_rgb(0xc4, 0xc8, 0xcf);
    let comb = Color32::from_rgb(0x2a, 0x2c, 0x33);
    let harp = Rect::from_center_size(rect.center() + vec2(0.0, 6.0), vec2(w - 70.0, 64.0));
    // The hands cupped around it, behind it: closer the more they close.
    let shut = if on { v[3] } else { p[HANDS].clamp(0.0, 1.0) };
    if shut > 0.05 {
        let skin = Color32::from_rgba_unmultiplied(0xd8, 0xa8, 0x88, (25.0 + 55.0 * shut) as u8);
        let size = vec2(10.0 + 16.0 * shut, 30.0 + 8.0 * shut);
        painter.add(Shape::ellipse_filled(pos2(harp.left() - 4.0 + 8.0 * shut, harp.center().y), size, skin));
        painter.add(Shape::ellipse_filled(pos2(harp.right() + 4.0 - 8.0 * shut, harp.center().y), size, skin));
    }
    painter.rect_filled(harp, 8.0, comb);
    let cover_top = Rect::from_min_max(harp.min, pos2(harp.right(), harp.top() + 18.0));
    let cover_bot = Rect::from_min_max(pos2(harp.left(), harp.bottom() - 18.0), harp.max);
    for c in [cover_top, cover_bot] {
        painter.rect_filled(c, 7.0, metal);
        painter.line_segment([pos2(c.left() + 8.0, c.top() + 5.0), pos2(c.right() - 8.0, c.top() + 5.0)], Stroke::new(1.5_f32, Color32::from_white_alpha(90)));
    }
    let hw = (harp.width() - 20.0) / 10.0;
    for i in 0..10 {
        let cx = harp.left() + 10.0 + (i as f32 + 0.5) * hw;
        let hole = Rect::from_center_size(pos2(cx, harp.center().y), vec2(hw * 0.55, 14.0));
        let lv = if on { v[4 + i] } else { 0.0 };
        let level = lv.abs().min(1.0);
        painter.rect_filled(hole, 2.0, mix(Color32::from_gray(8), accent, 0.75 * level));
        if level > 0.02 {
            painter.circle_filled(hole.center(), 10.0 + 8.0 * level, accent.gamma_multiply(0.15 * level));
            // Blown: the breath goes in; drawn: it comes out.
            if lv > 0.0 {
                breath(&painter, hole.center() + vec2(0.0, 30.0), vec2(0.0, -1.0), level, t, i as f32);
            } else {
                breath(&painter, hole.center() + vec2(0.0, 8.0), vec2(0.0, 1.0), level, t, i as f32);
            }
            let bend = v[14 + i];
            let mark = if bend < -0.5 { "OB".to_string() } else if bend > 0.5 { format!("-{}", bend.round() as i32) } else { String::new() };
            if !mark.is_empty() {
                painter.text(pos2(cx, cover_bot.bottom() + 12.0), Align2::CENTER_CENTER, mark, FontId::monospace(10.0), accent);
            }
        }
        // The hole's number, its blow note above and its draw note below.
        painter.text(pos2(cx, cover_top.top() + 11.0), Align2::CENTER_CENTER, format!("{}", i + 1), FontId::monospace(8.0), Color32::from_gray(70));
        if holes_mode {
            let blow = (low + BLOW[i]).rem_euclid(12) as usize;
            let draw = (low + DRAW[i]).rem_euclid(12) as usize;
            let up = lv > 0.02;
            let down = lv < -0.02;
            painter.text(pos2(cx, harp.top() - 9.0), Align2::CENTER_CENTER, NAMES[blow], FontId::monospace(9.0), if up { accent } else { DIM });
            painter.text(pos2(cx, harp.bottom() + 9.0 + if v[14 + i].abs() > 0.5 && on { 12.0 } else { 0.0 }), Align2::CENTER_CENTER, NAMES[draw], FontId::monospace(9.0), if down { accent } else { DIM });
        }
    }
    air_meter(&painter, rect, p[BREATH], if on { v[2] } else { 0.0 }, accent);
    let what = if holes_mode {
        let pos = POSITIONS[(p[POSITION].round().max(0.0) as usize).min(POSITIONS.len() - 1)];
        if p[KEY].round() < 0.5 {
            format!("{} harp, {}", NAMES[key], pos.split(' ').next().unwrap_or(""))
        } else {
            format!("{} harp", NAMES[key])
        }
    } else {
        "chromatic, free".to_string()
    };
    caption(&painter, rect.left_top() + vec2(10.0, 6.0), Align2::LEFT_TOP, "Harmonica", accent);
    caption(&painter, pos2(rect.right() - 20.0, rect.top() + 6.0), Align2::RIGHT_TOP, &what, DIM);
    ui.ctx().request_repaint_after(std::time::Duration::from_millis(33));
    resp.on_hover_text("Drag up or down: the hands, cupped closed or open. Blown holes take breath in, drawn ones let it out; -1, -2, -3 are bends, OB an overblow");
    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::RawInput;

    fn show(f: impl Fn(&mut Ui, &InstLive)) {
        let ctx = egui::Context::default();
        let mut vals = [0.0f32; krabton_engine::engine::INST_VALS];
        for (i, v) in vals.iter_mut().enumerate() {
            *v = (i as f32 * 0.37).fract();
        }
        vals[0] = 1.0;
        vals[1] = 60.0;
        let live = InstLive { wave: vec![0.0; 256], notes: 0, vals };
        for _ in 0..3 {
            let _ = ctx.run(RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1200.0, 800.0))), ..Default::default() }, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| f(ui, &live));
            });
        }
    }

    #[test]
    fn reed_screens_draw() {
        use krabton_engine::dsp::*;
        for name in melodica::preset_names() {
            show(|ui, live| {
                let mut p = melodica::preset(name);
                melodica_screen(ui, Id::new("m"), &mut p, live, 200.0);
            });
        }
        for name in harmonica::preset_names() {
            show(|ui, live| {
                let mut p = harmonica::preset(name);
                harmonica_screen(ui, Id::new("h"), &mut p, live, 200.0);
            });
        }
    }
}

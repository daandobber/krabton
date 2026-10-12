//! Screens for Birds (the birds on their branches with a sonogram of their
//! songs above them), Nature (a landscape that moves with the wind, the
//! rain, the lightning, the waves, the crickets, the frogs and the fire)
//! and Pan Flute (its pipes, or the flute with its holes). They draw what
//! the engine reports, so what you see is what you hear, and they always
//! move a little.

use super::inst_screen::{caption, InstLive, SCREEN_BG};
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, FontId, Id, Painter, Pos2, Rect, Sense, Shape, Stroke, Ui};

const DIM: Color32 = Color32::from_rgb(0x4a, 0x4e, 0x58);
const FAINT: Color32 = Color32::from_rgb(0x1e, 0x21, 0x28);
const NIGHT: Color32 = Color32::from_rgb(0x12, 0x16, 0x22);
const HILL: Color32 = Color32::from_rgb(0x1a, 0x22, 0x24);
const HILL_NEAR: Color32 = Color32::from_rgb(0x16, 0x1d, 0x1c);
const GRASS: Color32 = Color32::from_rgb(0x3c, 0x6a, 0x48);
const WATER: Color32 = Color32::from_rgb(0x1f, 0x4a, 0x6a);

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

/// Rolling hills along `y`, filled down to the bottom of `r`.
fn hills(p: &Painter, r: Rect, y: f32, amp: f32, seed: f32, color: Color32) {
    let n = 24;
    let mut pts: Vec<Pos2> = (0..=n)
        .map(|k| {
            let x = k as f32 / n as f32;
            let h = (x * 5.0 + seed).sin() * 0.5 + (x * 11.0 + seed * 2.0).sin() * 0.25;
            pos2(r.left() + x * r.width(), y - amp * (0.5 + 0.5 * h))
        })
        .collect();
    pts.push(r.right_bottom());
    pts.push(r.left_bottom());
    // Not convex: draw it as strips.
    for k in 0..n {
        let (a, b) = (pts[k], pts[k + 1]);
        p.add(Shape::convex_polygon(vec![a, b, pos2(b.x, r.bottom()), pos2(a.x, r.bottom())], color, Stroke::NONE));
    }
}

// ---------------------------------------------------------------- birds

/// The species' colour (blackbird black with an orange beak, robin's red
/// breast, and so on), and its beak.
fn bird_colors(species: usize) -> (Color32, Color32) {
    match species {
        0 => (Color32::from_rgb(0x2a, 0x2a, 0x30), Color32::from_rgb(0xf0, 0xa0, 0x20)),
        1 => (Color32::from_rgb(0xd8, 0x6a, 0x3a), Color32::from_rgb(0x30, 0x2a, 0x28)),
        2 => (Color32::from_rgb(0x9a, 0x80, 0x60), Color32::from_rgb(0x50, 0x44, 0x38)),
        3 => (Color32::from_rgb(0x8a, 0x6e, 0x54), Color32::from_rgb(0x50, 0x44, 0x38)),
        4 => (Color32::from_rgb(0x8e, 0x66, 0x42), Color32::from_rgb(0x40, 0x34, 0x28)),
        5 => (Color32::from_rgb(0xc8, 0x8a, 0x78), Color32::from_rgb(0x70, 0x80, 0x90)),
        6 => (Color32::from_rgb(0xe0, 0xd0, 0x40), Color32::from_rgb(0x20, 0x20, 0x20)),
        7 => (Color32::from_rgb(0xa8, 0x94, 0x70), Color32::from_rgb(0x60, 0x50, 0x40)),
        8 => (Color32::from_rgb(0x9c, 0x7a, 0x56), Color32::from_rgb(0x40, 0x38, 0x30)),
        9 => (Color32::from_rgb(0x88, 0x90, 0x9c), Color32::from_rgb(0xd0, 0xb0, 0x30)),
        10 => (Color32::from_rgb(0x9a, 0xa0, 0xb4), Color32::from_rgb(0xe0, 0x90, 0x90)),
        11 => (Color32::from_rgb(0x9a, 0x70, 0x48), Color32::from_rgb(0xd0, 0xc0, 0x80)),
        12 => (Color32::from_rgb(0x20, 0x24, 0x28), Color32::from_rgb(0x30, 0x30, 0x34)),
        13 => (Color32::from_rgb(0xe8, 0xec, 0xf0), Color32::from_rgb(0xf0, 0xc0, 0x30)),
        14 => (Color32::from_rgb(0x40, 0x40, 0x44), Color32::from_rgb(0xd0, 0x30, 0x30)),
        15 => (Color32::from_rgb(0x90, 0x94, 0x8c), Color32::from_rgb(0x20, 0x20, 0x20)),
        16 | 17 => (Color32::from_rgb(0xa8, 0xa8, 0x6a), Color32::from_rgb(0x50, 0x48, 0x38)),
        18 => (Color32::from_rgb(0xe8, 0xd0, 0x30), Color32::from_rgb(0x70, 0x68, 0x60)),
        19 | 27 => (Color32::from_rgb(0xc8, 0xb8, 0xa0), Color32::from_rgb(0x30, 0x30, 0x30)),
        20 => (Color32::from_rgb(0xf0, 0xd0, 0x20), Color32::from_rgb(0xc0, 0x60, 0x70)),
        21 => (Color32::from_rgb(0x18, 0x18, 0x1c), Color32::from_rgb(0x28, 0x28, 0x2c)),
        22 => (Color32::from_rgb(0x9a, 0x84, 0x64), Color32::from_rgb(0x40, 0x34, 0x2c)),
        23 => (Color32::from_rgb(0x30, 0x2c, 0x2c), Color32::from_rgb(0x20, 0x20, 0x20)),
        24 | 25 => (Color32::from_rgb(0xa0, 0x78, 0x58), Color32::from_rgb(0xd0, 0xb0, 0x90)),
        26 => (Color32::from_rgb(0xd8, 0x20, 0x28), Color32::from_rgb(0xf0, 0x90, 0x40)),
        28 => (Color32::from_rgb(0x6a, 0x9a, 0x50), Color32::from_rgb(0x30, 0x30, 0x30)),
        _ => (Color32::from_rgb(0xb0, 0xa0, 0x80), Color32::from_rgb(0x40, 0x40, 0x40)),
    }
}

/// A bird seen from the side at `at`, facing `dir` (1 right, -1 left);
/// singing opens its beak, flying spreads its wings.
fn bird(p: &Painter, at: Pos2, size: f32, dir: f32, body: Color32, beak: Color32, singing: f32, flap: Option<f32>, accent: Color32) {
    let s = size;
    if singing > 0.02 {
        p.circle_filled(at, s * 2.2, accent.gamma_multiply(0.08 + 0.25 * singing));
    }
    // Tail.
    p.add(Shape::convex_polygon(vec![at + vec2(-dir * s * 0.6, -s * 0.1), at + vec2(-dir * s * 1.7, s * 0.2), at + vec2(-dir * s * 1.6, s * 0.5)], shade(body, 0.8), Stroke::NONE));
    p.add(Shape::ellipse_filled(at, vec2(s, s * 0.62), body));
    let head = at + vec2(dir * s * 0.75, -s * 0.5);
    p.circle_filled(head, s * 0.45, body);
    p.circle_filled(head + vec2(dir * s * 0.16, -s * 0.08), (s * 0.09).max(0.8), Color32::from_gray(0xf0));
    // The beak: open while it sings.
    let tip = head + vec2(dir * s * 0.75, 0.0);
    let open = singing.min(1.0) * s * 0.28;
    p.add(Shape::convex_polygon(vec![head + vec2(dir * s * 0.35, -s * 0.1), tip + vec2(0.0, -open), head + vec2(dir * s * 0.35, 0.02)], beak, Stroke::NONE));
    p.add(Shape::convex_polygon(vec![head + vec2(dir * s * 0.35, 0.02), tip + vec2(0.0, open), head + vec2(dir * s * 0.35, s * 0.12)], beak, Stroke::NONE));
    match flap {
        Some(f) => {
            let up = f.sin() * s * 1.3;
            p.add(Shape::convex_polygon(vec![at + vec2(-s * 0.3, -s * 0.1), at + vec2(s * 0.3, -s * 0.1), at + vec2(-dir * s * 0.2, -s * 0.2 - up)], shade(body, 1.25), Stroke::NONE));
        }
        None => {
            p.add(Shape::ellipse_filled(at + vec2(-dir * s * 0.15, -s * 0.05), vec2(s * 0.6, s * 0.32), shade(body, 0.85)));
            // Legs on the branch.
            p.line_segment([at + vec2(0.0, s * 0.5), at + vec2(0.0, s * 0.95)], Stroke::new(1.0_f32, DIM));
        }
    }
}

/// What a Birds screen remembers between frames: the songs' traces for
/// the sonogram, where each bird was (to see it fly) and which way it faces.
#[derive(Clone, Default)]
struct Trails {
    /// (time, per bird: pitch Hz, level).
    cols: std::collections::VecDeque<(f64, [(f32, f32); 12])>,
    last: [(f32, f32); 12],
    facing: [f32; 12],
}

/// Birds: a sonogram of what they sing above, the birds on their branches
/// below, near ones low and big, far ones high and small. Drag left or
/// right: how busy they are.
pub(crate) fn birds_screen(ui: &mut Ui, id: Id, p: &mut [f32], live: &InstLive, height: f32) -> bool {
    use krabton_engine::dsp::birds::*;
    let w = 320.0;
    let (rect, _) = ui.allocate_exact_size(vec2(w, height), Sense::hover());
    let resp = ui.interact(rect, id, Sense::click_and_drag());
    let mut changed = false;
    if let Some(pos) = resp.interact_pointer_pos().filter(|_| resp.dragged() || resp.clicked()) {
        let v = (((pos.x - rect.left() - 10.0) / (w - 20.0)).clamp(0.0, 1.0) * 100.0).round() / 100.0;
        changed |= set(p, BUSY, v);
    }
    let painter = frame(ui, rect);
    let accent = accent_now();
    let now = ui.input(|i| i.time);
    let t = now as f32;
    let v = &live.vals;
    let on = v[0] > 0.5;
    // The birds now: across, how far (0..1), how loud, pitch, species.
    let mut birds = [(0.0f32, 0.5f32, 0.0f32, 0.0f32, -1.0f32); MAX_BIRDS];
    for (i, b) in birds.iter_mut().enumerate() {
        let o = 4 + 5 * i;
        if on && o + 4 < v.len() {
            *b = (v[o], v[o + 1], v[o + 2], v[o + 3] * 10000.0, v[o + 4]);
        } else {
            // Not playing yet: the birds from the params, sitting quietly.
            let slot = i / PER_SLOT;
            let here = (i % PER_SLOT) < p[NUMBER_1 + 2 * slot].round().max(0.0) as usize;
            *b = (hash(i as f32) * 2.0 - 1.0, (p[DISTANCE] + 0.6 * (hash(i as f32 + 7.0) - 0.5)).clamp(0.0, 1.0), 0.0, 0.0, if here { p[BIRD_1 + 2 * slot].round() } else { -1.0 });
        }
    }
    let mut tr: Trails = ui.data_mut(|d| d.get_temp::<Trails>(id).unwrap_or_default());
    let mut col = [(0.0f32, 0.0f32); 12];
    for (i, b) in birds.iter().enumerate() {
        col[i] = (b.3, b.2);
    }
    tr.cols.push_back((now, col));
    let pps = 70.0; // pixels a second
    let keep = (w / pps) as f64 + 0.5;
    while tr.cols.front().is_some_and(|c| now - c.0 > keep) {
        tr.cols.pop_front();
    }
    // ---- the sonogram
    let son = Rect::from_min_max(rect.left_top() + vec2(8.0, 22.0), pos2(rect.right() - 8.0, rect.top() + height * 0.42));
    painter.rect_filled(son, 4.0, NIGHT);
    let fy = |f: f32| son.bottom() - ((f.max(250.0) / 250.0).ln() / (11000.0f32 / 250.0).ln()).clamp(0.0, 1.0) * son.height();
    for hz in [500.0, 1000.0, 2000.0, 4000.0, 8000.0] {
        let y = fy(hz);
        painter.line_segment([pos2(son.left(), y), pos2(son.right(), y)], Stroke::new(1.0_f32, FAINT));
        painter.text(pos2(son.left() + 2.0, y - 1.0), Align2::LEFT_BOTTOM, if hz >= 1000.0 { format!("{}k", hz / 1000.0) } else { format!("{hz}") }, FontId::monospace(8.0), DIM);
    }
    let sp = painter.with_clip_rect(son);
    let cols: Vec<&(f64, [(f32, f32); 12])> = tr.cols.iter().collect();
    for i in 0..MAX_BIRDS {
        if birds[i].4 < 0.0 {
            continue;
        }
        let (c, _) = bird_colors(birds[i].4 as usize);
        let c = mix(c, accent, 0.45);
        for k in 1..cols.len() {
            let (a, b) = (cols[k - 1], cols[k]);
            let (fa, la) = a.1[i];
            let (fb, lb) = b.1[i];
            if la > 0.02 && lb > 0.02 && fa > 0.0 && fb > 0.0 {
                let xa = son.right() - (now - a.0) as f32 * pps;
                let xb = son.right() - (now - b.0) as f32 * pps;
                let lv = lb.min(1.0);
                sp.line_segment([pos2(xa, fy(fa)), pos2(xb, fy(fb))], Stroke::new(1.0 + 2.5 * lv, c.gamma_multiply(0.35 + 0.65 * lv)));
            }
        }
    }
    // ---- the scene
    let scene = Rect::from_min_max(pos2(rect.left() + 1.0, son.bottom() + 4.0), rect.right_bottom() - vec2(1.0, 1.0));
    let horizon = scene.top() + scene.height() * 0.28;
    hills(&painter, scene, horizon + 8.0, 10.0, 1.3, HILL);
    hills(&painter, scene, scene.top() + scene.height() * 0.62, 12.0, 4.1, HILL_NEAR);
    // Two branches across, swaying a little.
    let sway = |x: f32| (t * 0.8 + x * 0.01).sin() * 1.5;
    for (y, thick) in [(scene.top() + scene.height() * 0.48, 2.0_f32), (scene.top() + scene.height() * 0.82, 3.0_f32)] {
        let pts: Vec<Pos2> = (0..=16).map(|k| {
            let x = scene.left() + k as f32 / 16.0 * scene.width();
            pos2(x, y + ((k as f32) * 0.9).sin() * 3.0 + sway(x))
        }).collect();
        painter.add(Shape::line(pts, Stroke::new(thick, Color32::from_rgb(0x4a, 0x38, 0x2c))));
    }
    let mut names: Vec<&str> = Vec::new();
    for slot in 0..SLOTS {
        if p[NUMBER_1 + 2 * slot] >= 0.5 {
            let n = SPECIES[(p[BIRD_1 + 2 * slot].round().max(0.0) as usize).min(SPECIES.len() - 1)];
            if !names.contains(&n) {
                names.push(n);
            }
        }
    }
    // Far birds first.
    let mut order: Vec<usize> = (0..MAX_BIRDS).filter(|&i| birds[i].4 >= 0.0).collect();
    order.sort_by(|&a, &b| birds[b].1.total_cmp(&birds[a].1));
    for i in order {
        let (x, d, level, _f, sp_i) = birds[i];
        let near = 1.0 - d.clamp(0.0, 1.0);
        let px = scene.left() + 14.0 + (0.5 + 0.5 * x.clamp(-1.0, 1.0)) * (scene.width() - 28.0);
        let y_far = scene.top() + scene.height() * 0.48;
        let y_near = scene.top() + scene.height() * 0.82;
        let py = y_far + (y_near - y_far) * near + sway(px) - 4.0 - 4.0 * near;
        let moved = ((x - tr.last[i].0).abs() + (d - tr.last[i].1).abs()) > 0.0005;
        if (x - tr.last[i].0).abs() > 0.0005 {
            tr.facing[i] = (x - tr.last[i].0).signum();
        }
        if tr.facing[i] == 0.0 {
            tr.facing[i] = if hash(i as f32 + 3.0) > 0.5 { 1.0 } else { -1.0 };
        }
        tr.last[i] = (x, d);
        // Sitting birds bob now and then; flying ones flap.
        let bob = ((t * 1.7 + i as f32 * 2.3).sin().max(0.85) - 0.85) * 12.0;
        let flap = moved.then_some(t * 22.0 + i as f32);
        let size = 3.0 + 4.5 * near;
        let (body, beak) = bird_colors(sp_i as usize);
        bird(&painter, pos2(px, py - bob - if moved { 10.0 } else { 0.0 }), size, tr.facing[i], body, beak, level, flap, accent);
    }
    ui.data_mut(|d| d.insert_temp(id, tr));
    caption(&painter, rect.left_top() + vec2(10.0, 6.0), Align2::LEFT_TOP, &if names.is_empty() { "no birds".to_string() } else { names.join(" · ") }, accent);
    caption(&painter, pos2(rect.right() - 10.0, rect.top() + 6.0), Align2::RIGHT_TOP, &format!("busy {:.0}%", p[BUSY] * 100.0), DIM);
    ui.ctx().request_repaint_after(std::time::Duration::from_millis(33));
    resp.on_hover_text("Drag left or right: how busy they are. Above, the sonogram of what they sing; play a key and the bird whose voice is nearest sings");
    changed
}

// ---------------------------------------------------------------- nature

/// Nature: a landscape at dusk. The wind bends the grass and blows streaks
/// across, rain falls slanting with it, lightning lights the sky before the
/// thunder, waves roll in and break, crickets glow as they chirp, frogs
/// puff their throats, the fire flickers. Drag left or right: the wind.
pub(crate) fn nature_screen(ui: &mut Ui, id: Id, p: &mut [f32], live: &InstLive, height: f32) -> bool {
    use krabton_engine::dsp::nature::*;
    let w = 320.0;
    let (rect, _) = ui.allocate_exact_size(vec2(w, height), Sense::hover());
    let resp = ui.interact(rect, id, Sense::click_and_drag());
    let mut changed = false;
    if let Some(pos) = resp.interact_pointer_pos().filter(|_| resp.dragged() || resp.clicked()) {
        let v = (((pos.x - rect.left() - 10.0) / (w - 20.0)).clamp(0.0, 1.0) * 100.0).round() / 100.0;
        changed |= set(p, WIND, v);
    }
    let painter = frame(ui, rect);
    let accent = accent_now();
    let t = ui.input(|i| i.time) as f32;
    let v = &live.vals;
    let on = v[0] > 0.5;
    let k = |i: usize| p[i].clamp(0.0, 1.0);
    // Live when playing; from the params, calmer, when not.
    let wind = if on { v[1] } else { k(WIND) * (0.8 + 0.2 * (t * 0.7).sin()) };
    let dir = if on { v[2] } else { 0.3 };
    let rain = if on { v[5] } else { k(RAIN) * 0.5 };
    let flash = if on { v[6] } else { 0.0 };
    let rumble = if on { v[7] } else { 0.0 };
    let shake = vec2((t * 63.0).sin(), (t * 71.0).cos()) * rumble * 2.0;
    let r = rect.translate(shake);
    let ground = r.top() + r.height() * 0.62;
    // The sky: a little lighter towards the horizon, white in a flash.
    let sky = Rect::from_min_max(r.left_top() + vec2(1.0, 1.0), pos2(r.right() - 1.0, ground));
    for band in 0..6 {
        let y0 = sky.top() + band as f32 / 6.0 * sky.height();
        let c = mix(NIGHT, Color32::from_rgb(0x2a, 0x2c, 0x44), band as f32 / 6.0);
        painter.rect_filled(Rect::from_min_max(pos2(sky.left(), y0), pos2(sky.right(), y0 + sky.height() / 6.0 + 1.0)), 0.0, c);
    }
    // Stars, unless it is cloudy.
    let cloudy = (k(RAIN) + k(THUNDER)).min(1.0);
    for s in 0..40 {
        let (x, y) = (hash(s as f32), hash(s as f32 + 100.0));
        let tw = 0.5 + 0.5 * (t * (1.0 + 2.0 * hash(s as f32 + 50.0)) + s as f32).sin();
        let a = (1.0 - cloudy) * (0.25 + 0.6 * tw);
        painter.circle_filled(pos2(sky.left() + x * sky.width(), sky.top() + y * sky.height() * 0.7), 0.8, Color32::from_white_alpha((a * 200.0) as u8));
    }
    // Clouds drifting with the wind.
    if cloudy > 0.0 {
        for c in 0..6 {
            let speed = 4.0 + 30.0 * wind;
            let x = (hash(c as f32 + 9.0) * sky.width() + t * speed * dir.signum().max(0.2)).rem_euclid(sky.width() + 80.0) - 40.0;
            let y = sky.top() + 10.0 + hash(c as f32 + 19.0) * sky.height() * 0.4;
            let col = mix(Color32::from_rgb(0x30, 0x32, 0x3c), Color32::WHITE, flash * 0.6).gamma_multiply(0.6 + 0.4 * cloudy);
            for b in 0..3 {
                painter.add(Shape::ellipse_filled(pos2(sky.left() + x + b as f32 * 14.0, y + (b as f32 - 1.0).abs() * 3.0), vec2(16.0, 8.0), col));
            }
        }
    }
    if flash > 0.05 {
        painter.rect_filled(sky, 0.0, Color32::from_white_alpha((flash * 90.0) as u8));
        if flash > 0.5 {
            // The bolt.
            let mut x = sky.left() + sky.width() * (0.2 + 0.6 * hash((t * 3.0).floor()));
            let mut y = sky.top() + 10.0;
            let mut pts = vec![pos2(x, y)];
            while y < ground - 10.0 {
                y += 8.0 + 6.0 * hash(y + x);
                x += (hash(y * 1.7) - 0.5) * 18.0;
                pts.push(pos2(x, y));
            }
            painter.add(Shape::line(pts, Stroke::new(1.5_f32, Color32::from_white_alpha((flash * 255.0) as u8))));
        }
    }
    // The wind singing: waves in the air.
    let whistle = if on { v[3] } else { 0.0 };
    if whistle > 0.02 {
        for line in 0..3 {
            let y0 = sky.top() + sky.height() * (0.3 + 0.15 * line as f32);
            let pts: Vec<Pos2> = (0..=40).map(|j| {
                let x = sky.left() + j as f32 / 40.0 * sky.width();
                pos2(x, y0 + (x * 0.08 - t * 4.0 + line as f32).sin() * 3.0 * whistle.min(1.0))
            }).collect();
            painter.add(Shape::line(pts, Stroke::new(1.0_f32, accent.gamma_multiply(0.2 + 0.5 * whistle.min(1.0)))));
        }
    }
    hills(&painter, r, ground + 2.0, 14.0, 2.2, HILL);
    // The water: sea, or a brook.
    let sea = k(SEA);
    let stream = k(STREAM);
    let water_top = r.top() + r.height() * 0.8;
    if sea > 0.0 {
        let sea_r = Rect::from_min_max(pos2(r.left() + 1.0, water_top), r.right_bottom() - vec2(1.0, 1.0));
        painter.rect_filled(sea_r, 0.0, WATER);
        for i in 0..2 {
            let (u, size) = if on { (v[9 + 2 * i], v[10 + 2 * i]) } else { (((t / 9.0) + i as f32 * 0.5).fract(), 0.5 * sea) };
            // A wave rolls in from the left and breaks halfway.
            let x = sea_r.left() + u.clamp(0.0, 1.0) * sea_r.width();
            let rise = if u < 0.5 { u / 0.5 } else { (1.0 - (u - 0.5) * 2.0).max(0.0) };
            let h = 4.0 + 12.0 * size * rise;
            let pts: Vec<Pos2> = (0..=20).map(|j| {
                let dx = (j as f32 / 20.0 - 0.5) * 70.0;
                pos2(x + dx, water_top + 4.0 - h * (-(dx / 22.0).powi(2)).exp())
            }).collect();
            let mut poly = pts.clone();
            poly.push(pos2(x + 35.0, water_top + 6.0));
            poly.push(pos2(x - 35.0, water_top + 6.0));
            for j in 0..20 {
                painter.add(Shape::convex_polygon(vec![pts[j], pts[j + 1], pos2(pts[j + 1].x, water_top + 6.0), pos2(pts[j].x, water_top + 6.0)], WATER, Stroke::NONE));
            }
            let foam = if u > 0.42 { (1.0 - (u - 0.42) * 2.0).clamp(0.0, 1.0) } else { 0.0 };
            painter.add(Shape::line(pts, Stroke::new(1.0 + 2.0 * foam, mix(Color32::from_rgb(0x6a, 0x9c, 0xc0), Color32::WHITE, foam))));
        }
    } else if stream > 0.0 {
        let pts: Vec<Pos2> = (0..=30).map(|j| {
            let x = r.left() + j as f32 / 30.0 * r.width();
            pos2(x, water_top + 6.0 + (x * 0.04).sin() * 4.0)
        }).collect();
        painter.add(Shape::line(pts.clone(), Stroke::new(6.0 + 6.0 * stream, WATER)));
        for b in 0..(4.0 + 10.0 * stream) as usize {
            let ph = (t * 0.6 + hash(b as f32)).fract();
            let x = r.left() + ((hash(b as f32 + 3.0) + t * 0.05).fract()) * r.width();
            let y = water_top + 6.0 + (x * 0.04).sin() * 4.0;
            painter.circle_stroke(pos2(x, y), 1.0 + 4.0 * ph, Stroke::new(1.0_f32, Color32::from_white_alpha(((1.0 - ph) * 90.0) as u8)));
        }
    }
    // Grass, bending with the wind (each blade its own gusts).
    let blades = 46;
    let grass_y = if sea > 0.0 { water_top } else { r.bottom() - 3.0 };
    for b in 0..blades {
        let x = r.left() + 4.0 + b as f32 / blades as f32 * (r.width() - 8.0);
        let h = 8.0 + 10.0 * hash(b as f32 + 30.0);
        let lean = dir.signum().max(-1.0) * (wind * (0.55 + 0.35 * (t * 2.5 + b as f32 * 0.6).sin())) + 0.08 * (t * 1.3 + b as f32).sin();
        let lean = lean.clamp(-1.3, 1.3);
        let tip = pos2(x + lean.sin() * h, grass_y - lean.cos() * h);
        painter.line_segment([pos2(x, grass_y), tip], Stroke::new(1.2_f32, shade(GRASS, 0.7 + 0.5 * hash(b as f32 + 60.0))));
    }
    // The tree: its leaves shiver in the wind.
    let leaves = if on { v[4] } else { 0.0 };
    let trunk = pos2(r.left() + 40.0, ground + 6.0);
    painter.line_segment([trunk, trunk - vec2(0.0, 34.0)], Stroke::new(4.0_f32, Color32::from_rgb(0x3a, 0x2c, 0x22)));
    for c in 0..9 {
        let a = c as f32 / 9.0 * std::f32::consts::TAU;
        let jig = vec2((t * 9.0 + c as f32).sin(), (t * 11.0 + c as f32).cos()) * (0.6 + 2.5 * leaves) * (0.3 + wind);
        let at = trunk - vec2(0.0, 44.0) + vec2(a.cos() * 14.0, a.sin() * 9.0) + jig;
        painter.circle_filled(at, 9.0, shade(GRASS, 0.55 + 0.2 * hash(c as f32)));
    }
    // Wind streaks.
    for s in 0..(wind * 14.0) as usize {
        let fy = hash(s as f32 + 200.0);
        let ph = ((t * (0.25 + 0.5 * wind) + fy * 7.0) * if dir < 0.0 { -1.0 } else { 1.0 }).rem_euclid(1.0);
        let x = r.left() + ph * r.width();
        let y = r.top() + 16.0 + fy * (ground - r.top() - 10.0);
        let len = 10.0 + 18.0 * wind;
        painter.line_segment([pos2(x, y), pos2(x + len * dir.signum(), y)], Stroke::new(1.0_f32, Color32::from_white_alpha((14.0 + 40.0 * wind.min(1.0)) as u8)));
    }
    // Rain.
    if rain > 0.0 {
        let slant = dir.signum() * wind.min(1.2) * 6.0;
        for d in 0..(rain * 70.0) as usize {
            let x0 = hash(d as f32 + 300.0);
            let ph = (t * (1.6 + hash(d as f32 + 400.0)) + x0 * 3.0).fract();
            let x = r.left() + x0 * r.width() + slant * ph * 4.0;
            let y = r.top() + ph * (r.height() - 4.0);
            painter.line_segment([pos2(x, y), pos2(x + slant, y + 7.0)], Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(0xa0, 0xc0, 0xe0, 110)));
        }
    }
    // Crickets glow when they chirp.
    if k(CRICKETS) > 0.0 {
        for c in 0..6 {
            let e = if on { v[16 + c] } else { 0.0 };
            let at = pos2(r.left() + 70.0 + hash(c as f32 + 500.0) * (r.width() - 90.0), grass_y - 4.0 - 10.0 * hash(c as f32 + 510.0));
            painter.circle_filled(at, 1.2, Color32::from_rgb(0x60, 0x70, 0x30));
            if e > 0.02 {
                painter.circle_filled(at, 1.5 + 4.0 * e, Color32::from_rgb(0xd0, 0xf0, 0x60).gamma_multiply(0.15 + 0.6 * e));
            }
        }
    }
    // Frogs puff their throats.
    if k(FROGS) > 0.0 {
        for f in 0..5 {
            let e = if on { v[22 + f] } else { 0.0 };
            let at = pos2(r.left() + 90.0 + f as f32 * 44.0 + 10.0 * hash(f as f32 + 600.0), grass_y - 2.0);
            painter.add(Shape::ellipse_filled(at, vec2(5.0, 3.0), Color32::from_rgb(0x4a, 0x7a, 0x3a)));
            painter.circle_filled(at + vec2(3.0, -2.5), 1.0, Color32::from_gray(0xe0));
            if e > 0.02 {
                painter.circle_filled(at + vec2(4.0, 1.0), 1.0 + 3.5 * e, Color32::from_rgb(0xc8, 0xd8, 0x90).gamma_multiply(0.8));
            }
        }
    }
    // The fire.
    if k(FIRE) > 0.0 {
        let fl = if on { v[15] } else { k(FIRE) * 0.6 };
        let base = pos2(r.right() - 60.0, grass_y - 2.0);
        painter.line_segment([base + vec2(-10.0, 2.0), base + vec2(10.0, -1.0)], Stroke::new(3.0_f32, Color32::from_rgb(0x5a, 0x3a, 0x22)));
        painter.line_segment([base + vec2(-9.0, -1.0), base + vec2(9.0, 2.0)], Stroke::new(3.0_f32, Color32::from_rgb(0x4a, 0x30, 0x1c)));
        painter.circle_filled(base - vec2(0.0, 10.0), 22.0 * fl.max(0.2), Color32::from_rgb(0xff, 0x80, 0x20).gamma_multiply(0.08));
        for (j, (col, sc)) in [(Color32::from_rgb(0xe0, 0x50, 0x10), 1.0), (Color32::from_rgb(0xff, 0xa0, 0x20), 0.7), (Color32::from_rgb(0xff, 0xe0, 0x80), 0.4)].into_iter().enumerate() {
            let h = (10.0 + 18.0 * fl) * sc * (0.85 + 0.15 * (t * 13.0 + j as f32).sin());
            let wv = 7.0 * sc;
            let tip = base + vec2((t * 7.0 + j as f32).sin() * 2.0, -h);
            painter.add(Shape::convex_polygon(vec![base + vec2(-wv, -1.0), tip, base + vec2(wv, -1.0)], col, Stroke::NONE));
        }
        for s in 0..5 {
            let ph = (t * 0.7 + hash(s as f32 + 700.0)).fract();
            let at = base + vec2((hash(s as f32 + 710.0) - 0.5) * 16.0 + (t * 3.0 + s as f32).sin() * 3.0, -14.0 - ph * 40.0);
            painter.circle_filled(at, 1.0, Color32::from_rgb(0xff, 0xc0, 0x40).gamma_multiply(1.0 - ph));
        }
    }
    // What plays.
    let mut parts: Vec<String> = Vec::new();
    if k(WIND) > 0.0 {
        parts.push("wind".into());
    }
    if k(RAIN) > 0.0 {
        parts.push(format!("rain on {}", SURFACES[(p[RAIN_ON].round().max(0.0) as usize).min(SURFACES.len() - 1)].to_lowercase()));
    }
    if k(THUNDER) > 0.0 {
        parts.push("thunder".into());
    }
    if stream > 0.0 {
        parts.push("brook".into());
    }
    if sea > 0.0 {
        parts.push("sea".into());
    }
    if k(CRICKETS) > 0.0 {
        parts.push(format!("crickets at {:.0} degrees", 10.0 + 22.0 * k(WARMTH)));
    }
    if k(FROGS) > 0.0 {
        parts.push("frogs".into());
    }
    if k(FIRE) > 0.0 {
        parts.push("fire".into());
    }
    caption(&painter, rect.left_top() + vec2(10.0, 6.0), Align2::LEFT_TOP, &parts.join(" · "), accent);
    caption(&painter, pos2(rect.right() - 10.0, rect.top() + 6.0), Align2::RIGHT_TOP, &format!("wind {:.0}%", wind * 100.0), DIM);
    ui.ctx().request_repaint_after(std::time::Duration::from_millis(33));
    resp.on_hover_text("Drag left or right: the wind. Play keys and the wind whistles on those notes");
    changed
}

// ---------------------------------------------------------------- pan flute

/// The pipes of a pan flute (two octaves of a major scale from the key's
/// C; a note between two pipes lights both, as a player bends it), or the
/// flute with its finger holes. Breath streams over the pipes that sound.
/// Drag up or down: the breath.
pub(crate) fn pan_flute_screen(ui: &mut Ui, id: Id, p: &mut [f32], live: &InstLive, height: f32) -> bool {
    use krabton_engine::dsp::pan_flute::*;
    let w = 300.0;
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
    let model = (p[MODEL].round().max(0.0) as usize).min(MODELS.len() - 1);
    // The notes sounding and how loud.
    let mut sounding: Vec<(u8, f32)> = Vec::new();
    if on {
        for i in 0..8 {
            let lv = v[3 + 2 * i];
            if lv > 0.02 {
                sounding.push((v[2 + 2 * i] as u8, lv.min(1.0)));
            }
        }
    }
    let bamboo = Color32::from_rgb(0xc8, 0xa8, 0x6a);
    let wood = Color32::from_rgb(0x8a, 0x5c, 0x36);
    let breath_dots = |painter: &Painter, at: Pos2, lv: f32, seed: f32| {
        for d in 0..8 {
            let ph = (t * 1.8 + hash(d as f32 + seed)).fract();
            let pos = at + vec2(-14.0 + ph * 28.0, -4.0 - 6.0 * hash(d as f32 + seed + 1.0) - ph * 4.0);
            painter.circle_filled(pos, 1.2, Color32::from_white_alpha(((1.0 - ph) * 140.0 * lv) as u8));
        }
    };
    let transpose = p[TRANSPOSE].round() as i32;
    if model == 0 {
        // Pipes: two octaves of C major and a top C, from C4 (moved by Transpose).
        const STEPS: [i32; 15] = [0, 2, 4, 5, 7, 9, 11, 12, 14, 16, 17, 19, 21, 23, 24];
        let low = 60 + transpose;
        let n = STEPS.len();
        let pw = (w - 40.0) / n as f32;
        let top = rect.top() + 34.0;
        let longest = rect.height() - 60.0;
        for (i, &s) in STEPS.iter().enumerate() {
            let note = low + s;
            // Length as the pitch: halves every octave.
            let len = longest * 2f32.powf(-(s as f32) / 12.0 * 0.85);
            let x = rect.left() + 20.0 + i as f32 * pw;
            let pipe = Rect::from_min_size(pos2(x + 1.0, top), vec2(pw - 3.0, len));
            // How much it sounds: the nearest pipe to each note.
            let mut lv = 0.0f32;
            for &(nn, l) in &sounding {
                let d = (nn as i32 + transpose - note).abs();
                if d == 0 {
                    lv = lv.max(l);
                } else if d == 1 && !STEPS.contains(&(nn as i32 + transpose - low)) {
                    lv = lv.max(l * 0.5);
                }
            }
            let c = mix(bamboo, accent, 0.6 * lv);
            painter.rect_filled(pipe, 3.0, shade(c, 0.85 + 0.15 * hash(i as f32)));
            painter.rect_filled(Rect::from_min_size(pipe.min + vec2(2.0, 2.0), vec2(2.0, pipe.height() - 4.0)), 1.0, mix(c, Color32::WHITE, 0.35).gamma_multiply(0.7));
            painter.add(Shape::ellipse_filled(pos2(pipe.center().x, top), vec2(pipe.width() * 0.5, 2.5), Color32::from_rgb(0x20, 0x18, 0x10)));
            if lv > 0.02 {
                painter.circle_filled(pos2(pipe.center().x, top), 10.0 * lv + 3.0, accent.gamma_multiply(0.18 * lv));
                breath_dots(&painter, pos2(pipe.center().x, top), lv, i as f32 * 3.1);
            }
            if s % 12 == 0 {
                painter.text(pos2(pipe.center().x, pipe.bottom() + 2.0), Align2::CENTER_TOP, krabton_engine::note_name(note.clamp(0, 127) as u8), FontId::monospace(8.0), DIM);
            }
        }
        // The binding across.
        let by = top + 22.0;
        painter.rect_filled(Rect::from_min_max(pos2(rect.left() + 16.0, by), pos2(rect.right() - 16.0, by + 6.0)), 2.0, wood);
        painter.rect_filled(Rect::from_min_max(pos2(rect.left() + 16.0, by + 34.0), pos2(rect.right() - 16.0, by + 38.0)), 2.0, shade(wood, 0.8));
    } else {
        // A flute with six holes; an ocarina is round.
        let lv = sounding.iter().map(|s| s.1).fold(0.0, f32::max);
        let note = sounding.first().map(|s| s.0 as i32 + transpose).unwrap_or(60 + transpose);
        // Holes open from the foot as the note climbs the scale.
        const DEGREE: [usize; 12] = [0, 0, 1, 1, 2, 3, 3, 4, 4, 5, 5, 6];
        let open = if sounding.is_empty() { 0 } else { DEGREE[note.rem_euclid(12) as usize] };
        let c = mix(if model == 4 { wood } else { bamboo }, accent, 0.4 * lv);
        let mid = rect.center() + vec2(0.0, 8.0);
        if model == 2 {
            painter.add(Shape::ellipse_filled(mid, vec2(80.0, 44.0), c));
            painter.add(Shape::ellipse_stroke(mid, vec2(80.0, 44.0), Stroke::new(1.0_f32, shade(c, 0.7))));
            let mouth = mid + vec2(-96.0, -6.0);
            painter.rect_filled(Rect::from_center_size(mouth, vec2(34.0, 14.0)), 4.0, c);
            for h in 0..6 {
                let at = mid + vec2(-45.0 + h as f32 * 18.0, -10.0 + (h % 2) as f32 * 12.0);
                let o = 5 - h < open;
                painter.circle_filled(at, 5.0, if o { Color32::from_rgb(0x20, 0x18, 0x10) } else { shade(c, 0.75) });
                if !o {
                    painter.circle_stroke(at, 5.0, Stroke::new(1.0_f32, shade(c, 0.6)));
                }
            }
            if lv > 0.02 {
                breath_dots(&painter, mouth + vec2(-10.0, -6.0), lv, 7.0);
            }
        } else {
            // Shakuhachi held down at a slant; the side-blown ones level.
            let slant = if model == 1 { 0.35f32 } else { 0.0 };
            let (sn, cs) = slant.sin_cos();
            let len = w - 50.0;
            let a = mid - vec2(cs, sn) * len * 0.5;
            let b = mid + vec2(cs, sn) * len * 0.5;
            let thick = if model == 1 { 16.0 } else { 12.0 };
            painter.line_segment([a, b], Stroke::new(thick, c));
            painter.line_segment([a + vec2(0.0, -thick * 0.25), b + vec2(0.0, -thick * 0.25)], Stroke::new(2.0_f32, mix(c, Color32::WHITE, 0.35).gamma_multiply(0.6)));
            // Nodes of the bamboo.
            for nd in 1..5 {
                let at = a + (b - a) * (nd as f32 / 5.0);
                painter.line_segment([at - vec2(-sn, cs) * thick * 0.5, at + vec2(-sn, cs) * thick * 0.5], Stroke::new(1.5_f32, shade(c, 0.7)));
            }
            // The mouth: at the top end, or a hole near it.
            let mouth = if model == 1 { a } else { a + (b - a) * 0.1 };
            painter.circle_filled(mouth, 3.5, Color32::from_rgb(0x20, 0x18, 0x10));
            for h in 0..6 {
                let at = a + (b - a) * (0.45 + h as f32 * 0.075);
                let o = 5 - h < open;
                painter.circle_filled(at, 3.5, if o { Color32::from_rgb(0x20, 0x18, 0x10) } else { shade(c, 0.75) });
                if !o {
                    painter.circle_filled(at, 4.5, Color32::from_rgb(0xd0, 0xa0, 0x80).gamma_multiply(0.7));
                }
            }
            if lv > 0.02 {
                breath_dots(&painter, mouth, lv, 5.0);
                painter.circle_filled(mouth, 8.0 + 8.0 * lv, accent.gamma_multiply(0.15 * lv));
            }
        }
        if !sounding.is_empty() {
            caption(&painter, pos2(rect.center().x, rect.bottom() - 8.0), Align2::CENTER_BOTTOM, &krabton_engine::note_name(note.clamp(0, 127) as u8), accent);
        }
    }
    // The breath meter.
    let br = if on { v[1] } else { 0.0 };
    let meter = Rect::from_min_size(pos2(rect.right() - 12.0, rect.top() + 26.0), vec2(4.0, rect.height() - 40.0));
    painter.rect_filled(meter, 2.0, FAINT);
    let fill = meter.height() * p[BREATH].clamp(0.0, 1.0);
    painter.rect_filled(Rect::from_min_max(pos2(meter.left(), meter.bottom() - fill), meter.max), 2.0, accent.gamma_multiply(0.35 + 0.65 * br.min(1.0)));
    caption(&painter, rect.left_top() + vec2(10.0, 6.0), Align2::LEFT_TOP, MODELS[model], accent);
    let voices = p[VOICES].round() as usize;
    caption(&painter, pos2(rect.right() - 20.0, rect.top() + 6.0), Align2::RIGHT_TOP, &if voices <= 1 { "one voice".to_string() } else { format!("{voices} voices") }, DIM);
    ui.ctx().request_repaint_after(std::time::Duration::from_millis(33));
    resp.on_hover_text("Drag up or down: the breath. The pipes (or holes) of the notes you play light up");
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
        let live = InstLive { wave: vec![0.0; 256], notes: 0, vals };
        for _ in 0..3 {
            let _ = ctx.run(RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1200.0, 800.0))), ..Default::default() }, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| f(ui, &live));
            });
        }
    }

    #[test]
    fn nature_screens_draw() {
        use krabton_engine::dsp::*;
        show(|ui, live| {
            let mut p = birds::preset("Dawn Chorus");
            birds_screen(ui, Id::new("b"), &mut p, live, 220.0);
        });
        for name in nature::preset_names() {
            show(|ui, live| {
                let mut p = nature::preset(name);
                nature_screen(ui, Id::new("n"), &mut p, live, 220.0);
            });
        }
        for m in 0..pan_flute::MODELS.len() {
            show(|ui, live| {
                let mut p = pan_flute::preset("Andes Pan Flute");
                p[pan_flute::MODEL] = m as f32;
                pan_flute_screen(ui, Id::new("f"), &mut p, live, 200.0);
            });
        }
    }
}

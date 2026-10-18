//! V2's screen: its three oscillators' waves (click for the next mode,
//! right-click back), its two filters' curves (drag for the first's cutoff
//! and resonance, click for its mode), its two envelopes with where the
//! newest note is on them, and the slots of the matrix in use.

use super::inst_screen::{caption, InstLive, SCREEN_BG};
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, FontId, Id, Painter, Pos2, Rect, Sense, Shape, Stroke, Ui};
use krabton_engine::dsp::v2::*;

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

/// An oscillator's wave at phase `x` (0 to 1).
fn wave(mode: usize, color: f32, x: f32) -> f32 {
    let c = (color / 128.0).clamp(0.01, 0.99);
    match mode {
        1 => {
            if x < c {
                -1.0 + 2.0 * x / c
            } else {
                1.0 - 2.0 * (x - c) / (1.0 - c)
            }
        }
        2 => {
            if x < c {
                1.0
            } else {
                -1.0
            }
        }
        3 => (std::f32::consts::TAU * x).cos(),
        4 => {
            let h = ((x * 997.0).sin() * 43758.547).fract();
            (h * 2.0 - 1.0) * (0.3 + 0.7 * (1.0 - c))
        }
        5 => (std::f32::consts::TAU * x + 1.5 * (std::f32::consts::TAU * x * 2.0).sin()).cos(),
        _ => 0.0,
    }
}

fn line(painter: &Painter, r: Rect, f: impl Fn(f32) -> f32, color: Color32) {
    let pts: Vec<Pos2> = (0..=100)
        .map(|k| {
            let x = k as f32 / 100.0;
            pos2(r.left() + x * r.width(), r.center().y - f(x).clamp(-1.2, 1.2) * r.height() * 0.42)
        })
        .collect();
    painter.add(Shape::line(pts, Stroke::new(1.5_f32, color)));
}

/// A filter's gain at `f` (Hz), roughly: the state-variable modes, and four poles for the Moog ones.
fn response(mode: usize, cutoff: f32, reso: f32, f: f32) -> f32 {
    let fc = 20.0 * 1000f32.powf(cutoff / 127.0);
    let x = f / fc;
    let q = 0.5 + 6.0 * (reso / 127.0).powi(2);
    let lp2 = 1.0 / ((1.0 - x * x).powi(2) + (x / q).powi(2)).sqrt();
    match mode {
        1 => lp2,
        2 => (x / q) * lp2,
        3 => x * x * lp2,
        4 => (1.0 - x * x).abs() * lp2,
        5 => 1.0,
        6 => (1.0 / (1.0 + x.powi(8)).sqrt()) * (1.0 + 3.0 * (reso / 127.0) * (-(x - 1.0).powi(2) * 8.0).exp()),
        7 => (x.powi(4) / (1.0 + x.powi(8)).sqrt()) * (1.0 + 3.0 * (reso / 127.0) * (-(x - 1.0).powi(2) * 8.0).exp()),
        _ => 1.0,
    }
}

/// An envelope's shape: attack up, decay to the sustain, held, released.
fn env_shape(p: &[f32], e: usize, r: Rect) -> Vec<Pos2> {
    let a = 0.02 + get(p, env(e, 0)) / 127.0 * 0.3;
    let d = 0.02 + get(p, env(e, 1)) / 127.0 * 0.3;
    let s = get(p, env(e, 2)) / 127.0;
    let rel = 0.02 + get(p, env(e, 4)) / 127.0 * 0.3;
    let tilt = (get(p, env(e, 3)) - 64.0) / 64.0;
    let hold = 1.0 - a - d - rel;
    let y = |v: f32| r.bottom() - v.clamp(0.0, 1.0) * r.height();
    let x = |t: f32| r.left() + t * r.width();
    let s_end = (s * (1.0 + tilt * 0.6)).clamp(0.0, 1.0);
    vec![pos2(x(0.0), y(0.0)), pos2(x(a), y(1.0)), pos2(x(a + d), y(s)), pos2(x(a + d + hold), y(s_end)), pos2(x(1.0), y(0.0))]
}

pub(crate) fn v2_screen(ui: &mut Ui, id: Id, p: &mut [f32], live: &InstLive, height: f32) -> bool {
    let w = 440.0;
    let (rect, _) = ui.allocate_exact_size(vec2(w, height), Sense::hover());
    let painter = ui.painter_at(rect);
    rect_fill(&painter, rect, 6.0, SCREEN_BG);
    rect_line(&painter, rect, 6.0, 1.0, Color32::from_rgb(0x26, 0x29, 0x31));
    let accent = accent_now();
    let mut changed = false;
    caption(&painter, rect.left_top() + vec2(10.0, 6.0), Align2::LEFT_TOP, "V2", accent);
    let playing = live.vals[6] > 0.0;
    let info = if playing { format!("{} VOICES", live.vals[0].round()) } else { format!("POLY {}", get(p, POLY).round()) };
    painter.text(rect.right_top() + vec2(-10.0, 7.0), Align2::RIGHT_TOP, info, FontId::monospace(9.0), TEXT_DIM);
    let top = rect.top() + 24.0;
    let inner = w - 20.0;
    let row = (rect.height() - 34.0) / 3.0;
    // The oscillators.
    for i in 0..3 {
        let r = Rect::from_min_size(pos2(rect.left() + 10.0 + i as f32 * inner / 3.0, top), vec2(inner / 3.0 - 6.0, row - 6.0));
        painter.rect_filled(r, 3.0, FAINT);
        let mode = get(p, osc(i, O_MODE)).round().clamp(0.0, (OSC_MODES.len() - 1) as f32) as usize;
        let on = mode != 0 && get(p, osc(i, O_VOLUME)) > 0.0;
        let color = get(p, osc(i, O_COLOR));
        line(&painter, r.shrink2(vec2(6.0, 10.0)), |x| wave(mode, color, (x * 2.0).fract()), if on { accent } else { DIM });
        let ring = i > 0 && get(p, osc(i, O_RING)) >= 0.5;
        let label = format!("{} {}{}", i + 1, OSC_MODES[mode].to_uppercase(), if ring { "  RING" } else { "" });
        painter.text(r.left_top() + vec2(4.0, 2.0), Align2::LEFT_TOP, label, FontId::monospace(8.5), if on { TEXT } else { DIM });
        let resp = ui.interact(r, id.with(("osc", i)), Sense::click());
        if resp.clicked() {
            changed |= set(p, osc(i, O_MODE), ((mode + 1) % OSC_MODES.len()) as f32);
        } else if resp.secondary_clicked() {
            changed |= set(p, osc(i, O_MODE), ((mode + OSC_MODES.len() - 1) % OSC_MODES.len()) as f32);
        }
    }
    // The filters.
    let fr = Rect::from_min_size(pos2(rect.left() + 10.0, top + row), vec2(inner * 0.62, row - 6.0));
    painter.rect_filled(fr, 3.0, FAINT);
    let resp = ui.interact(fr, id.with("filter"), Sense::click_and_drag());
    let m1 = get(p, flt(0, 0)).round().clamp(0.0, 7.0) as usize;
    if resp.dragged() {
        let d = resp.drag_delta();
        changed |= set(p, flt(0, 1), (get(p, flt(0, 1)) + d.x / fr.width() * 127.0).round().clamp(0.0, 127.0));
        changed |= set(p, flt(0, 2), (get(p, flt(0, 2)) - d.y / fr.height() * 127.0).round().clamp(0.0, 127.0));
    }
    if resp.clicked() {
        changed |= set(p, flt(0, 0), ((m1 + 1) % FLT_MODES.len()) as f32);
    } else if resp.secondary_clicked() {
        changed |= set(p, flt(0, 0), ((m1 + FLT_MODES.len() - 1) % FLT_MODES.len()) as f32);
    }
    let m2 = get(p, flt(1, 0)).round().clamp(0.0, 7.0) as usize;
    let routing = get(p, ROUTING).round().clamp(0.0, 2.0) as usize;
    for (k, (mode, color)) in [(m1, accent), (m2, YELLOW)].into_iter().enumerate() {
        if mode == 0 || (k == 1 && routing == 0) {
            continue;
        }
        let (c, q) = (get(p, flt(k, 1)), get(p, flt(k, 2)));
        let pts: Vec<Pos2> = (0..=90)
            .map(|i| {
                let x = i as f32 / 90.0;
                let f = 20.0 * 1000f32.powf(x);
                let db = 20.0 * response(mode, c, q, f).max(1e-4).log10();
                pos2(fr.left() + x * fr.width(), fr.center().y - (db / 36.0).clamp(-0.5, 0.45) * fr.height())
            })
            .collect();
        painter.add(Shape::line(pts, Stroke::new(1.5_f32, color)));
    }
    let flabel = if routing == 0 || m2 == 0 { FLT_MODES[m1].to_uppercase() } else { format!("{} + {}  {}", FLT_MODES[m1].to_uppercase(), FLT_MODES[m2].to_uppercase(), ROUTINGS[routing].to_uppercase()) };
    painter.text(fr.left_top() + vec2(4.0, 2.0), Align2::LEFT_TOP, flabel, FontId::monospace(8.5), TEXT);
    // The distortion and effects, as words.
    let er = Rect::from_min_max(pos2(fr.right() + 6.0, fr.top()), pos2(rect.right() - 10.0, fr.bottom()));
    painter.rect_filled(er, 3.0, FAINT);
    let dist = DIST_MODES[get(p, VDIST).round().clamp(0.0, 10.0) as usize];
    let lines = [
        format!("DIST  {}", dist.to_uppercase()),
        format!("CHORUS  {}", if (get(p, CHORUS) - 64.0).abs() > 0.5 { "ON" } else { "OFF" }),
        format!("REVERB  {}", get(p, REVERB_SEND).round()),
        format!("DELAY  {}", get(p, DELAY_SEND).round()),
        format!("BOOST  {}", get(p, BOOST).round()),
    ];
    for (k, t) in lines.iter().enumerate() {
        painter.text(er.left_top() + vec2(6.0, 4.0 + k as f32 * 12.0), Align2::LEFT_TOP, t, FontId::monospace(8.5), TEXT_DIM);
    }
    // The envelopes, and the matrix.
    let er2 = Rect::from_min_size(pos2(rect.left() + 10.0, top + 2.0 * row), vec2(inner * 0.42, row - 6.0));
    painter.rect_filled(er2, 3.0, FAINT);
    let shape_r = er2.shrink2(vec2(6.0, 8.0));
    for (e, color) in [(0, accent), (1, YELLOW)] {
        painter.add(Shape::line(env_shape(p, e, shape_r), Stroke::new(1.5_f32, if e == 0 { color } else { color.gamma_multiply(0.7) })));
        if playing {
            let lvl = live.vals[1 + e].clamp(0.0, 1.0);
            let y = shape_r.bottom() - lvl * shape_r.height();
            painter.circle_filled(pos2(shape_r.right() - 4.0 - e as f32 * 8.0, y), 3.0, color);
        }
    }
    painter.text(er2.left_top() + vec2(4.0, 2.0), Align2::LEFT_TOP, "AMP EG  ·  EG 2", FontId::monospace(8.5), TEXT);
    let mr = Rect::from_min_max(pos2(er2.right() + 6.0, er2.top()), pos2(rect.right() - 10.0, er2.bottom()));
    painter.rect_filled(mr, 3.0, FAINT);
    let mut y = mr.top() + 4.0;
    for s in 0..MOD_SLOTS {
        let src = get(p, modslot(s, 0)).round() as usize;
        let amount = get(p, modslot(s, 1));
        // (Volume is the constant full channel volume: not worth a line)
        if src == 0 || src == 8 || src >= SOURCES.len() || (amount - 64.0).abs() < 0.5 {
            continue;
        }
        let dest = (get(p, modslot(s, 2)).round().max(0.0) as usize).min(DESTS.len() - 1);
        let pct = ((amount - 64.0) / 64.0 * 100.0).round();
        if y + 9.0 > mr.bottom() {
            break;
        }
        painter.text(pos2(mr.left() + 6.0, y), Align2::LEFT_TOP, format!("{} > {} {:+}%", SOURCES[src].to_uppercase(), DESTS[dest].to_uppercase(), pct), FontId::monospace(7.5), TEXT_DIM);
        y += 9.5;
    }
    if playing {
        ui.ctx().request_repaint();
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::RawInput;

    #[test]
    fn the_v2_screen_draws_every_preset() {
        let ctx = egui::Context::default();
        for playing in [false, true] {
            let mut vals = [0.0; krabton_engine::engine::INST_VALS];
            if playing {
                vals[0] = 3.0;
                vals[1] = 0.8;
                vals[2] = 0.4;
                vals[6] = 1.0;
            }
            let live = InstLive { wave: vec![0.0; 64], notes: 1 << 60, vals };
            for name in preset_names() {
                let mut p = preset(name);
                let _ = ctx.run(RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1200.0, 800.0))), ..Default::default() }, |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        v2_screen(ui, Id::new("v"), &mut p, &live, 200.0);
                    });
                });
            }
        }
    }
}

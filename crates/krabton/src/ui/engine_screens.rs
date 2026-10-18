//! Screens for Bassline (the acid filter), Drone (tanpura strings, pad,
//! swells and air), Waves (the wavetable stacks) and Guitar (the neck).
//! Like the other instrument screens they show what the engine reports
//! about the sound and edit the instrument's params.

use super::inst_screen::{caption, InstLive, SCREEN_BG};
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, Id, Pos2, Rect, Sense, Shape, Stroke, Ui};

const RED: Color32 = Color32::from_rgb(0xff, 0x4e, 0x62);
const YELLOW: Color32 = Color32::from_rgb(0xff, 0xd4, 0x3b);
const GREEN: Color32 = Color32::from_rgb(0x30, 0xd9, 0x8f);
const BLUE: Color32 = Color32::from_rgb(0x4d, 0x8d, 0xff);
const DIM: Color32 = Color32::from_rgb(0x4a, 0x4e, 0x58);
const FAINT: Color32 = Color32::from_rgb(0x1e, 0x21, 0x28);

fn frame(ui: &Ui, rect: Rect) -> egui::Painter {
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

/// The last `n` samples of the scope, drawn across `r`.
fn wave_line(p: &egui::Painter, r: Rect, live: &InstLive, n: usize, gain: f32, color: Color32) {
    let w = &live.wave[live.wave.len().saturating_sub(n)..];
    if w.len() < 2 {
        p.line_segment([pos2(r.left(), r.center().y), pos2(r.right(), r.center().y)], Stroke::new(1.0_f32, FAINT));
        return;
    }
    let pts: Vec<Pos2> = w.iter().enumerate().map(|(i, v)| pos2(r.left() + i as f32 / (w.len() - 1) as f32 * r.width(), r.center().y - (v * gain).clamp(-1.0, 1.0) * r.height() * 0.5)).collect();
    p.add(Shape::line(pts, Stroke::new(1.2_f32, color)));
}

fn led(p: &egui::Painter, at: Pos2, label: &str, on: bool, color: Color32) {
    p.circle_filled(at, 4.0, if on { color } else { FAINT });
    if on {
        p.circle_filled(at, 8.0, color.gamma_multiply(0.15));
    }
    caption(p, at + vec2(8.0, 0.0), Align2::LEFT_CENTER, label, if on { color } else { DIM });
}

// ---------------------------------------------------------------- bassline

/// The resonant low-pass of the acid voice: the curve follows the cutoff as
/// the envelope sweeps it (ghosts of where it was trail behind), with the
/// accent and slide lights and the wave below. Drag: left and right the
/// cutoff, up and down the resonance.
pub(crate) fn bassline_screen(ui: &mut Ui, id: Id, p: &mut [f32], live: &InstLive, height: f32) -> bool {
    bassline_screen_w(ui, id, p, live, 300.0, height)
}

/// [`bassline_screen`] `w` wide.
pub(crate) fn bassline_screen_w(ui: &mut Ui, id: Id, p: &mut [f32], live: &InstLive, w: f32, height: f32) -> bool {
    use krabton_engine::dsp::bassline::*;
    let (rect, resp) = ui.allocate_exact_size(vec2(w, height), Sense::click_and_drag());
    let plot = Rect::from_min_max(rect.min + vec2(10.0, 26.0), pos2(rect.right() - 10.0, rect.bottom() - 40.0));
    let (f_lo, f_hi) = (30.0f32, 12000.0f32);
    let x_of = |f: f32| plot.left() + ((f.max(f_lo) / f_lo).ln() / (f_hi / f_lo).ln()).clamp(0.0, 1.0) * plot.width();
    let mut changed = false;
    if let Some(pos) = resp.interact_pointer_pos().filter(|_| resp.dragged() || resp.clicked()) {
        let fx = ((pos.x - plot.left()) / plot.width()).clamp(0.0, 1.0);
        let spec = &SPECS[CUTOFF];
        let hz = (f_lo * (f_hi / f_lo).powf(fx)).clamp(spec.min, spec.max).round();
        let res = (((plot.bottom() - pos.y) / plot.height()).clamp(0.0, 1.0) * 100.0).round() / 100.0;
        changed |= set(p, CUTOFF, hz);
        changed |= set(p, RESONANCE, res);
    }
    if resp.hovered() || resp.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
    }
    let painter = frame(ui, rect);
    let sounding = live.vals[6] > 0.5;
    let res = p[RESONANCE].clamp(0.0, 1.0);
    // Four poles with feedback, the way the ladder rings at its cutoff.
    let k = 3.6 * res;
    let curve = |fc: f32| -> Vec<Pos2> {
        (0..=90)
            .map(|i| {
                let f = f_lo * (f_hi / f_lo).powf(i as f32 / 90.0);
                let x = f / fc.max(1.0);
                // (1 + jx)^4 + k
                let (mut re, mut im) = (1.0f32, 0.0f32);
                for _ in 0..4 {
                    (re, im) = (re - im * x, im + re * x);
                }
                let mag = (1.0 + k) / ((re + k).powi(2) + im * im).sqrt();
                let db = 20.0 * mag.max(1e-6).log10();
                pos2(x_of(f), plot.top() + ((24.0 - db) / 72.0).clamp(0.0, 1.0) * plot.height())
            })
            .collect()
    };
    for db in [0.0f32, -24.0, -48.0] {
        let y = plot.top() + (24.0 - db) / 72.0 * plot.height();
        painter.line_segment([pos2(plot.left(), y), pos2(plot.right(), y)], Stroke::new(1.0_f32, FAINT));
    }
    for f in [100.0f32, 1000.0, 10000.0] {
        let x = x_of(f);
        painter.line_segment([pos2(x, plot.top()), pos2(x, plot.bottom())], Stroke::new(1.0_f32, FAINT));
        caption(&painter, pos2(x + 2.0, plot.bottom()), Align2::LEFT_BOTTOM, if f < 500.0 { "100" } else if f < 5000.0 { "1k" } else { "10k" }, DIM);
    }
    // Where the knob is, and where the envelope has the filter now.
    painter.add(Shape::line(curve(p[CUTOFF]), Stroke::new(1.0_f32, DIM)));
    let ghosts_id = id.with("ghosts");
    let mut ghosts: Vec<f32> = ui.ctx().memory(|m| m.data.get_temp(ghosts_id).unwrap_or_default());
    if sounding {
        ghosts.push(live.vals[0]);
        if ghosts.len() > 24 {
            ghosts.remove(0);
        }
    } else {
        ghosts.clear();
    }
    for (i, &g) in ghosts.iter().enumerate() {
        let a = (i + 1) as f32 / ghosts.len() as f32;
        let x = x_of(g);
        painter.line_segment([pos2(x, plot.bottom() - 6.0), pos2(x, plot.bottom())], Stroke::new(2.0_f32, accent_now().gamma_multiply(0.15 + 0.5 * a)));
    }
    if sounding {
        let amp = live.vals[1].clamp(0.0, 1.5);
        painter.add(Shape::line(curve(live.vals[0]), Stroke::new(2.0_f32, mix(accent_now(), YELLOW, (amp - 0.5).max(0.0)))));
        let x = x_of(live.vals[0]);
        painter.line_segment([pos2(x, plot.top()), pos2(x, plot.bottom())], Stroke::new(1.0_f32, accent_now().gamma_multiply(0.4)));
    }
    ui.ctx().memory_mut(|m| m.data.insert_temp(ghosts_id, ghosts));
    caption(&painter, rect.left_top() + vec2(10.0, 8.0), Align2::LEFT_TOP, &format!("cutoff {:.0} hz  res {:.0}%", if sounding { live.vals[0] } else { p[CUTOFF] }, res * 100.0), accent_now());
    if sounding {
        let n = live.vals[2].round().clamp(0.0, 127.0) as u8;
        caption(&painter, pos2(rect.right() - 10.0, rect.top() + 8.0), Align2::RIGHT_TOP, &krabton_engine::note_name(n), YELLOW);
    }
    let ly = rect.bottom() - 30.0;
    led(&painter, pos2(rect.left() + 14.0, ly), "accent", sounding && live.vals[3] > 0.5, RED);
    led(&painter, pos2(rect.left() + 84.0, ly), "slide", sounding && live.vals[4] > 0.5, YELLOW);
    let wave = Rect::from_min_max(pos2(rect.left() + 150.0, ly - 9.0), pos2(rect.right() - 10.0, ly + 9.0));
    wave_line(&painter, wave, live, 600, 2.0, GREEN);
    caption(&painter, pos2(rect.left() + 10.0, rect.bottom() - 6.0), Align2::LEFT_BOTTOM, &format!("env {:.0}%  decay {:.0} ms  accent {:.0}%", p[ENV_MOD] * 100.0, p[DECAY], p[ACCENT] * 100.0), DIM);
    if sounding || resp.dragged() {
        ui.ctx().request_repaint();
    }
    resp.on_hover_text("Drag: left and right the cutoff, up and down the resonance");
    changed
}

// ---------------------------------------------------------------- drone

/// The tanpura's four strings shiver when plucked (the one plucked last is
/// lit); behind them the pad glows, the swelling pairs breathe and the air
/// drifts by, each as loud as its layer.
pub(crate) fn drone_screen(ui: &mut Ui, id: Id, p: &[f32], live: &InstLive, height: f32) {
    use krabton_engine::dsp::drone::*;
    let w = 290.0;
    let (rect, _) = ui.allocate_exact_size(vec2(w, height), Sense::hover());
    let painter = frame(ui, rect);
    let t = ui.input(|i| i.time) as f32;
    let on = live.vals[0] > 0.0;
    let v = &live.vals;
    let layer = |k: usize| if on { v[1 + k].clamp(0.0, 1.5) } else { 0.0 };
    let stage = Rect::from_min_max(rect.min + vec2(6.0, 18.0), pos2(rect.right() - 6.0, rect.bottom() - 46.0));
    // The pad: a soft glow that breathes.
    let pad = layer(1);
    if pad > 0.0 {
        let c = stage.center() + vec2(30.0, 0.0);
        for k in 0..6 {
            let r = 14.0 + k as f32 * 9.0 + 4.0 * (t * 0.4 + k as f32).sin();
            painter.circle_filled(c, r, BLUE.gamma_multiply(0.05 * pad * (6 - k) as f32 / 6.0));
        }
    }
    // The air: motes drifting by.
    let air = layer(3);
    let motes = (air * 40.0) as usize;
    for k in 0..motes {
        let s = (k as f32 * 12.9898).sin() * 43_758.547;
        let fx = s - s.floor();
        let fy = (k as f32 * 0.618_034).fract();
        let x = stage.left() + ((fx + t * (0.03 + 0.04 * fy)) % 1.0) * stage.width();
        let y = stage.top() + fy * stage.height() + 4.0 * (t * 0.7 + k as f32).sin();
        painter.circle_filled(pos2(x, y), 1.2, Color32::from_white_alpha((30.0 + 60.0 * air.min(1.0)) as u8));
    }
    // The swells: two pairs of orbs on the right.
    let swell = layer(2);
    for k in 0..4 {
        let c = pos2(stage.right() - 52.0 + (k % 2) as f32 * 30.0, stage.top() + 26.0 + (k / 2) as f32 * 42.0);
        let e = if on { v[10 + k].clamp(0.0, 1.0) } else { 0.0 };
        let r = 5.0 + 11.0 * e;
        painter.circle_filled(c, r, GREEN.gamma_multiply((0.15 + 0.6 * e) * swell.min(1.0)));
        painter.circle_stroke(c, 16.0, Stroke::new(1.0_f32, FAINT));
    }
    // The tanpura: four strings and their names.
    let first = FIRST_STRINGS[(p[FIRST_STRING].round().max(0.0) as usize).min(FIRST_STRINGS.len() - 1)];
    let names = [first, "Sa", "Sa", "sa"];
    let last = v[9].round() as usize;
    for s in 0..4 {
        let x = stage.left() + 22.0 + s as f32 * 22.0;
        let e = if on { v[5 + s] * layer(0).min(1.0) } else { 0.0 };
        let amp = (e * 40.0).min(6.0);
        let lit = on && s == last;
        let c = if lit { YELLOW } else { mix(DIM, YELLOW, (amp / 6.0).min(1.0)) };
        let pts: Vec<Pos2> = (0..=24)
            .map(|i| {
                let f = i as f32 / 24.0;
                let y = stage.top() + 6.0 + f * (stage.height() - 12.0);
                pos2(x + amp * (std::f32::consts::PI * f).sin() * (t * 70.0 + s as f32 * 2.0).sin(), y)
            })
            .collect();
        painter.add(Shape::line(pts, Stroke::new(if s == 3 { 2.0_f32 } else { 1.4 }, c)));
        caption(&painter, pos2(x, stage.bottom() + 2.0), Align2::CENTER_TOP, names[s], if lit { YELLOW } else { DIM });
    }
    // The layers' levels.
    let labels = ["tanpura", "pad", "swell", "air"];
    let colors = [YELLOW, BLUE, GREEN, Color32::from_rgb(0xc8, 0xc8, 0xd0)];
    let bw = (rect.width() - 20.0) / 4.0;
    for k in 0..4 {
        let x = rect.left() + 10.0 + k as f32 * bw;
        let y = rect.bottom() - 9.0;
        let set = p[[TANPURA, PAD, SWELL, AIR][k]].clamp(0.0, 1.0);
        rect_fill(&painter, Rect::from_min_size(pos2(x, y), vec2(bw - 8.0, 3.0)), 1.0, FAINT);
        rect_fill(&painter, Rect::from_min_size(pos2(x, y), vec2((bw - 8.0) * set, 3.0)), 1.0, colors[k].gamma_multiply(0.35));
        rect_fill(&painter, Rect::from_min_size(pos2(x, y), vec2((bw - 8.0) * (layer(k) / 1.2).min(1.0), 3.0)), 1.0, colors[k]);
        caption(&painter, pos2(x, y - 2.0), Align2::LEFT_BOTTOM, labels[k], if layer(k) > 0.01 { colors[k] } else { DIM });
    }
    let title = if on { format!("{}  ·  {} voice{}", krabton_engine::note_name(v[14].round().clamp(0.0, 127.0) as u8), v[0] as usize, if v[0] as usize == 1 { "" } else { "s" }) } else { "hold a key".to_string() };
    caption(&painter, rect.left_top() + vec2(10.0, 6.0), Align2::LEFT_TOP, &title, accent_now());
    caption(&painter, pos2(rect.right() - 10.0, rect.top() + 6.0), Align2::RIGHT_TOP, &format!("evolve {:.0}%", p[EVOLVE].clamp(0.0, 1.0) * 100.0), DIM);
    if on {
        ui.ctx().request_repaint();
    }
    let _ = id;
}

// ---------------------------------------------------------------- waves

/// Each oscillator's bank as a stack of its eight waves, the one it plays
/// now drawn bright where it sits in the stack. Drag a stack up or down to
/// move through it.
pub(crate) fn waves_screen(ui: &mut Ui, id: Id, p: &mut [f32], live: &InstLive, height: f32) -> bool {
    use krabton_engine::dsp::waves::*;
    let w = 330.0;
    let (rect, _) = ui.allocate_exact_size(vec2(w, height), Sense::hover());
    let mut changed = false;
    let on = live.vals[0] > 0.0;
    let half = (w - 12.0) / 2.0;
    let mut stacks = Vec::new();
    for k in 0..2 {
        let r = Rect::from_min_size(pos2(rect.left() + 6.0 + k as f32 * half, rect.top() + 18.0), vec2(half - 4.0, height - 44.0));
        let resp = ui.interact(r, id.with(("stack", k)), Sense::click_and_drag());
        let pos_i = [POS1, POS2][k];
        if let Some(pos) = resp.interact_pointer_pos().filter(|_| resp.dragged() || resp.clicked()) {
            let f = ((r.bottom() - 10.0 - pos.y) / (r.height() - 30.0)).clamp(0.0, 1.0);
            changed |= set(p, pos_i, (f * 100.0).round() / 100.0);
        }
        if resp.hovered() || resp.dragged() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
        }
        resp.on_hover_text("Drag up or down: where in the bank this oscillator plays");
        stacks.push(r);
    }
    let painter = frame(ui, rect);
    for (k, r) in stacks.iter().enumerate() {
        let (bank_i, pos_i, level_i) = [(BANK1, POS1, LEVEL1), (BANK2, POS2, LEVEL2)][k];
        let bank = (p[bank_i].round().max(0.0) as usize).min(BANKS.len() - 1);
        let audible = p[level_i] > 0.0;
        let pos = if on { live.vals[1 + k] } else { p[pos_i].clamp(0.0, 1.0) * (BANK_SIZE - 1) as f32 };
        let color = if k == 0 { accent_now() } else { BLUE };
        // Back to front: wave 7 at the top right, wave 0 at the bottom left.
        let ww = r.width() - 40.0;
        let wh = 22.0;
        let dy = (r.height() - wh - 8.0) / (BANK_SIZE - 1) as f32;
        let at = |i: f32| pos2(r.left() + 4.0 + i * 4.0, r.bottom() - wh * 0.5 - 4.0 - i * dy);
        let draw = |base: Pos2, samples: &dyn Fn(usize) -> f32, stroke: Stroke| {
            let n = 60;
            let pts: Vec<Pos2> = (0..=n).map(|j| pos2(base.x + j as f32 / n as f32 * ww, base.y - samples(j * (WAVE_LEN - 1) / n) * wh * 0.5)).collect();
            painter.add(Shape::line(pts, stroke));
        };
        for i in (0..BANK_SIZE).rev() {
            let wv = wave(bank * BANK_SIZE + i);
            let near = (1.0 - (i as f32 - pos).abs()).max(0.0);
            draw(at(i as f32), &|j| wv[j], Stroke::new(1.0_f32, mix(FAINT, color, 0.25 + 0.35 * near)));
        }
        // The wave it plays now, between its two neighbours.
        let lo = (pos.floor() as usize).min(BANK_SIZE - 2);
        let fr = (pos - lo as f32).clamp(0.0, 1.0);
        let (a, b) = (wave(bank * BANK_SIZE + lo), wave(bank * BANK_SIZE + lo + 1));
        let bright = if audible { color } else { DIM };
        draw(at(pos), &|j| a[j] * (1.0 - fr) + b[j] * fr, Stroke::new(2.0_f32, bright));
        caption(&painter, pos2(r.left() + 2.0, rect.top() + 6.0), Align2::LEFT_TOP, &format!("osc {} {}", k + 1, BANKS[bank]), if audible { color } else { DIM });
        caption(&painter, pos2(r.right() - 2.0, rect.top() + 6.0), Align2::RIGHT_TOP, &format!("{:.1}", pos + 1.0), DIM);
    }
    // Envelopes and the filter.
    let y = rect.bottom() - 12.0;
    let bar = |x: f32, label: &str, v: f32, c: Color32| {
        caption(&painter, pos2(x, y), Align2::LEFT_CENTER, label, DIM);
        let r = Rect::from_min_size(pos2(x + 30.0, y - 2.0), vec2(50.0, 4.0));
        rect_fill(&painter, r, 1.0, FAINT);
        rect_fill(&painter, Rect::from_min_size(r.min, vec2(50.0 * v.clamp(0.0, 1.0), 4.0)), 1.0, c);
    };
    bar(rect.left() + 10.0, "amp", if on { live.vals[3] } else { 0.0 }, GREEN);
    bar(rect.left() + 100.0, "flt", if on { live.vals[4] } else { 0.0 }, YELLOW);
    let cut = if on { live.vals[5] } else { p[CUTOFF] };
    caption(&painter, pos2(rect.right() - 10.0, y), Align2::RIGHT_CENTER, &format!("{} {:.0} hz", FILTERS[(p[FILTER].round().max(0.0) as usize).min(FILTERS.len() - 1)].to_lowercase(), cut), DIM);
    if on {
        ui.ctx().request_repaint();
    }
    changed
}

// ---------------------------------------------------------------- guitar

/// The neck, high string on top as in tab: where each string is held (open
/// strings ringed at the nut), the strings shivering while they ring, the
/// capo and the hand. Click the neck to move the hand's home position.
pub(crate) fn guitar_screen(ui: &mut Ui, id: Id, p: &mut [f32], live: &InstLive, height: f32) -> bool {
    use krabton_engine::dsp::guitar::*;
    let w = 360.0;
    let (rect, _) = ui.allocate_exact_size(vec2(w, height), Sense::hover());
    let resp = ui.interact(rect, id, Sense::click());
    let neck = Rect::from_min_max(pos2(rect.left() + 36.0, rect.top() + 24.0), pos2(rect.right() - 10.0, rect.bottom() - 26.0));
    const FRETS: i32 = 15;
    let span = 1.0 - 2f32.powf(-(FRETS as f32) / 12.0);
    let fret_x = |n: f32| neck.left() + neck.width() * (1.0 - 2f32.powf(-n / 12.0)) / span;
    let mut changed = false;
    if let Some(pos) = resp.interact_pointer_pos().filter(|_| resp.clicked()) {
        // The fret under the pointer becomes the home position.
        let f = (0..=FRETS).find(|&n| fret_x(n as f32) >= pos.x).unwrap_or(FRETS);
        changed |= set(p, POSITION, (f - 1).clamp(0, 12) as f32);
    }
    let painter = frame(ui, rect);
    let model = (p[MODEL].round().max(0.0) as usize).min(MODELS.len() - 1);
    let electric = is_electric(model);
    let wood = if electric { Color32::from_rgb(0x2a, 0x1d, 0x14) } else { Color32::from_rgb(0x3a, 0x28, 0x18) };
    rect_fill(&painter, neck, 2.0, wood);
    let capo = p[CAPO].round().clamp(0.0, 12.0) as i32;
    // Where the hand is.
    let hand = live.vals[12].clamp(0.0, 12.0);
    let hx0 = fret_x(hand.max(0.0));
    let hx1 = fret_x((hand + 4.0).min(FRETS as f32));
    rect_fill(&painter, Rect::from_min_max(pos2(hx0, neck.top()), pos2(hx1, neck.bottom())), 0.0, Color32::from_white_alpha(10));
    for n in 1..=FRETS {
        let x = fret_x(n as f32);
        painter.line_segment([pos2(x, neck.top()), pos2(x, neck.bottom())], Stroke::new(1.5_f32, Color32::from_rgb(0x9a, 0x9a, 0xa0)));
        if matches!(n, 3 | 5 | 7 | 9 | 15) {
            painter.circle_filled(pos2((fret_x(n as f32 - 1.0) + x) * 0.5, neck.center().y), 3.0, Color32::from_rgb(0xd8, 0xd0, 0xc0).gamma_multiply(0.6));
        }
        if n == 12 {
            for dy in [-0.25f32, 0.25] {
                painter.circle_filled(pos2((fret_x(11.0) + x) * 0.5, neck.center().y + dy * neck.height()), 3.0, Color32::from_rgb(0xd8, 0xd0, 0xc0).gamma_multiply(0.6));
            }
        }
        if matches!(n, 3 | 5 | 7 | 9 | 12 | 15) {
            caption(&painter, pos2((fret_x(n as f32 - 1.0) + x) * 0.5, neck.bottom() + 2.0), Align2::CENTER_TOP, &n.to_string(), DIM);
        }
    }
    painter.line_segment([pos2(neck.left(), neck.top()), pos2(neck.left(), neck.bottom())], Stroke::new(4.0_f32, Color32::from_rgb(0xe8, 0xe0, 0xd0)));
    if capo > 0 {
        let x = (fret_x(capo as f32 - 1.0) + fret_x(capo as f32)) * 0.5 + 4.0;
        rect_fill(&painter, Rect::from_center_size(pos2(x, neck.center().y), vec2(6.0, neck.height() + 8.0)), 2.0, Color32::from_rgb(0x20, 0x20, 0x24));
        rect_line(&painter, Rect::from_center_size(pos2(x, neck.center().y), vec2(6.0, neck.height() + 8.0)), 2.0, 1.0, DIM);
    }
    let tuning = (p[TUNING].round().max(0.0) as usize).min(TUNINGS.len() - 1);
    let open = open_strings(model, tuning);
    // The strings it has, spread over the neck (a ukulele's four wider apart).
    let strings: Vec<usize> = (0..6).filter(|&s| open[s].is_some()).collect();
    let count = strings.len().max(1);
    let gap = neck.height() / count as f32;
    let t = ui.input(|i| i.time) as f32;
    let mut any = false;
    for (row, &s) in strings.iter().enumerate() {
        let y = neck.top() + gap * (count - 1 - row) as f32 + gap * 0.5;
        let fret = live.vals[s];
        let level = live.vals[6 + s].max(0.0);
        let held = fret >= 0.0;
        let from = if held && fret as i32 > capo { (fret_x(fret - 1.0) + fret_x(fret)) * 0.5 } else { neck.left() };
        let thick = 1.0 + (5 - s.min(5)) as f32 * 0.3 * if MODELS[model] == "Bass" { 1.8 } else { 1.0 };
        let amp = if held { (level * 30.0).min(3.0) } else { 0.0 };
        let base = if held { mix(Color32::from_rgb(0xb0, 0xb0, 0xb8), YELLOW, (amp / 3.0).min(1.0)) } else { Color32::from_rgb(0x90, 0x90, 0x98) };
        // Behind the finger the string is still; from it to the bridge it rings.
        painter.line_segment([pos2(neck.left(), y), pos2(from, y)], Stroke::new(thick, Color32::from_rgb(0x90, 0x90, 0x98)));
        if amp > 0.05 {
            any = true;
            let n = 40;
            let pts: Vec<Pos2> = (0..=n)
                .map(|i| {
                    let f = i as f32 / n as f32;
                    let x = from + f * (neck.right() - from);
                    pos2(x, y + amp * (std::f32::consts::PI * f).sin() * (t * 60.0 + s as f32 * 1.3).sin())
                })
                .collect();
            painter.add(Shape::line(pts, Stroke::new(thick, base)));
        } else {
            painter.line_segment([pos2(from, y), pos2(neck.right(), y)], Stroke::new(thick, base));
        }
        if held {
            if fret as i32 <= capo {
                painter.circle_stroke(pos2(neck.left() - 8.0, y), 4.0, Stroke::new(1.5_f32, YELLOW));
            } else {
                painter.circle_filled(pos2(from, y), 5.5, YELLOW);
                caption(&painter, pos2(from, y), Align2::CENTER_CENTER, &format!("{}", fret as i32 - capo), Color32::BLACK);
            }
        }
        // One String: its string is marked.
        if p.get(STRING_MODE).is_some_and(|m| m.round() as usize == 3) && 6 - p.get(ONE_STRING).map(|v| v.round().clamp(1.0, 6.0) as usize).unwrap_or(3) == s {
            painter.line_segment([pos2(neck.left(), y), pos2(neck.right(), y)], Stroke::new(thick + 3.0, YELLOW.gamma_multiply(0.12)));
        }
        let name = krabton_engine::note_name(open[s].unwrap_or(0) + capo as u8);
        let name = name.trim_end_matches(|c: char| c.is_ascii_digit() || c == '-');
        caption(&painter, pos2(rect.left() + 8.0, y), Align2::LEFT_CENTER, name, if held { YELLOW } else { DIM });
    }
    let tuned = tuning_name(model, tuning);
    let model = MODELS[model];
    let mode = (p.get(STRING_MODE).copied().unwrap_or(0.0).round().max(0.0) as usize).min(STRING_MODES.len() - 1);
    let title = if mode == 0 { format!("{}  ·  {}", model, tuned) } else { format!("{}  ·  {}  ·  {}", model, tuned, STRING_MODES[mode].to_lowercase()) };
    caption(&painter, rect.left_top() + vec2(10.0, 6.0), Align2::LEFT_TOP, &title, accent_now());
    if capo > 0 {
        caption(&painter, pos2(rect.right() - 10.0, rect.top() + 6.0), Align2::RIGHT_TOP, &format!("capo {capo}"), DIM);
    }
    caption(&painter, pos2(rect.right() - 10.0, rect.bottom() - 6.0), Align2::RIGHT_BOTTOM, &format!("home fret {}", p[POSITION].round()), DIM);
    if any || live.level() > 0.001 {
        ui.ctx().request_repaint();
    }
    resp.on_hover_text("Click the neck: where the hand waits for the next notes");
    changed
}

// ---------------------------------------------------------------- wind chimes

/// The wind chime as the engine simulates it, in its material's colour:
/// every strike you hear is the clapper meeting a tube you see. Drag left
/// or right for the wind.
pub(crate) fn wind_chimes_screen(ui: &mut Ui, id: Id, p: &mut [f32], live: &InstLive, height: f32) -> bool {
    use krabton_engine::dsp::chime_sim::MAX_TUBES;
    use krabton_engine::dsp::wind_chimes::*;
    let w = 300.0;
    let (rect, _) = ui.allocate_exact_size(vec2(w, height), Sense::hover());
    let resp = ui.interact(rect, id, Sense::click_and_drag());
    let mut changed = false;
    if let Some(pos) = resp.interact_pointer_pos().filter(|_| resp.dragged() || resp.clicked()) {
        let v = (((pos.x - rect.left() - 10.0) / (w - 20.0)).clamp(0.0, 1.0) * 1.5 * 100.0).round() / 100.0;
        changed |= set(p, WIND, v);
    }
    let painter = frame(ui, rect);
    let s = shape(p);
    let f = tube_freqs(p);
    let names = (0..s.tubes).map(|i| krabton_engine::note_name((p[BASE_NOTE] + 12.0 * (f[i] / f[0]).log2()).round().clamp(0.0, 127.0) as u8)).collect();
    let mat = (p[MATERIAL].round().max(0.0) as usize).min(MATERIALS.len() - 1);
    let v = &live.vals;
    let on = v[4] > 0.5;
    let mut st = super::chime_view::ChimeState { clapper: [v[0], v[1]], wind: if on { v[2] } else { 0.0 }, dir: if on { v[3] } else { 0.4 }, ..Default::default() };
    if on {
        for i in 0..MAX_TUBES {
            st.tube[i] = [v[5 + i], v[13 + i]];
            st.ring[i] = (v[21 + i] * 3.0).min(1.0);
        }
        ui.ctx().request_repaint();
    }
    let look = super::chime_view::ChimeLook { shape: s, color: super::chime_view::material_color(mat), thickness: p[THICKNESS], names };
    let t = ui.input(|i| i.time) as f32;
    super::chime_view::draw(&painter, rect, &look, &st, t);
    caption(&painter, rect.left_top() + vec2(10.0, 6.0), Align2::LEFT_TOP, &format!("{} · {} tubes", MATERIALS[mat], s.tubes), accent_now());
    caption(&painter, pos2(rect.right() - 10.0, rect.top() + 6.0), Align2::RIGHT_TOP, &format!("wind {:.0}%", if on { v[2] } else { p[WIND] } * 100.0), DIM);
    resp.on_hover_text("Drag left or right: the wind. Every strike is the clapper meeting a tube; play a key to touch one");
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
        vals[0] = 2.0;
        vals[6] = 1.0;
        let live = InstLive { wave: (0..2048).map(|i| (i as f32 * 0.05).sin() * 0.3).collect(), notes: 1 << 45, vals };
        for _ in 0..2 {
            let _ = ctx.run(RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1200.0, 800.0))), ..Default::default() }, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| f(ui, &live));
            });
        }
    }

    #[test]
    fn engine_screens_draw() {
        use krabton_engine::dsp::*;
        show(|ui, live| {
            let mut p = bassline::preset("Squelch");
            bassline_screen(ui, Id::new("b"), &mut p, live, 160.0);
        });
        show(|ui, live| drone_screen(ui, Id::new("d"), &drone::preset("Tanpura"), live, 160.0));
        show(|ui, live| {
            let mut p = waves::preset(waves::preset_names()[0]);
            waves_screen(ui, Id::new("w"), &mut p, live, 160.0);
        });
        show(|ui, live| {
            let mut p = wind_chimes::preset("Temple Brass");
            wind_chimes_screen(ui, Id::new("c"), &mut p, live, 200.0);
        });
        show(|ui, live| {
            let mut p = guitar::preset("Steel Acoustic");
            p[guitar::CAPO] = 2.0;
            guitar_screen(ui, Id::new("g"), &mut p, live, 160.0);
        });
    }
}

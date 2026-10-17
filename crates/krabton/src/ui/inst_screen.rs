//! Instrument screens: a small display per instrument, bright lines on
//! black like the screens of hardware synths, showing what the sound is
//! doing. Particles fly around a jar, drawbars slide and the rotor turns,
//! voices run along their envelopes, strings and resonators vibrate. Some
//! parts can be grabbed: drawbars, envelope handles, the rotor's speed.

use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, FontId, Id, Painter, Pos2, Rect, Response, Sense, Shape, Stroke, Ui};
use krabton_engine::dsp::stk_synth::{self as stk, Family};
use krabton_engine::dsp::{analog, elements_synth, organ, rings_synth, shaker_synth, synth};
use krabton_engine::engine::{EngineStatus, INST_VALS};
use krabton_engine::params::ParamSpec;
use std::f32::consts::{PI, TAU};

pub const SCREEN_BG: Color32 = Color32::from_rgb(0x0a, 0x0b, 0x0e);
const BLUE: Color32 = Color32::from_rgb(0x4d, 0x8d, 0xff);
const GREEN: Color32 = Color32::from_rgb(0x30, 0xd9, 0x8f);
const RED: Color32 = Color32::from_rgb(0xff, 0x4e, 0x62);
const YELLOW: Color32 = Color32::from_rgb(0xff, 0xd4, 0x3b);
const WHITE: Color32 = Color32::from_rgb(0xf0, 0xf0, 0xf0);
const DIM: Color32 = Color32::from_rgb(0x4a, 0x4e, 0x58);
const FAINT: Color32 = Color32::from_rgb(0x1e, 0x21, 0x28);

/// What the engine reports about the instrument on screen.
pub(crate) struct InstLive {
    /// The latest samples (mono), oldest first.
    pub wave: Vec<f32>,
    /// Notes held, bit n = MIDI note n.
    pub notes: u128,
    pub vals: [f32; INST_VALS],
}

impl Default for InstLive {
    fn default() -> Self {
        Self { wave: Vec::new(), notes: 0, vals: [0.0; INST_VALS] }
    }
}

impl InstLive {
    /// What the instrument on track `t` sounds like now.
    pub(crate) fn read(status: &EngineStatus, t: usize) -> Self {
        let Some(ts) = status.tracks.get(t) else { return Self::default() };
        let (mut l, mut r) = (Vec::new(), Vec::new());
        ts.inst_scope.read(4096, &mut l, &mut r);
        Self { wave: l, notes: ts.inst_notes(), vals: ts.inst_values() }
    }

    /// Loudness of the last few milliseconds (RMS).
    pub(crate) fn level(&self) -> f32 {
        let tail = &self.wave[self.wave.len().saturating_sub(1024)..];
        if tail.is_empty() {
            return 0.0;
        }
        (tail.iter().map(|x| x * x).sum::<f32>() / tail.len() as f32).sqrt()
    }

    fn held(&self) -> impl Iterator<Item = u8> + '_ {
        (0..128u8).filter(|&n| self.notes >> n & 1 == 1)
    }
}

fn rnd(seed: &mut u32) -> f32 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 17;
    *seed ^= *seed << 5;
    (*seed >> 8) as f32 / 16_777_216.0
}

fn bip(seed: &mut u32) -> f32 {
    rnd(seed) * 2.0 - 1.0
}

/// The black screen, with its outline.
fn screen(ui: &mut Ui, w: f32, h: f32) -> Rect {
    let (rect, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
    let p = ui.painter();
    rect_fill(p, rect, 6.0, SCREEN_BG);
    rect_line(p, rect, 6.0, 1.0, Color32::from_rgb(0x26, 0x29, 0x31));
    rect
}

pub(super) fn caption(p: &Painter, pos: Pos2, anchor: Align2, s: &str, c: Color32) -> Rect {
    p.text(pos, anchor, s.to_uppercase(), FontId::monospace(10.0), c)
}

fn note_name(n: u8) -> String {
    krabton_engine::note_name(n)
}

/// A strip of keys with the held notes lit.
fn keys(p: &Painter, r: Rect, notes: u128, lo: u8, hi: u8, color: Color32) {
    let white: Vec<u8> = (lo..=hi).filter(|n| !matches!(n % 12, 1 | 3 | 6 | 8 | 10)).collect();
    let kw = r.width() / white.len().max(1) as f32;
    for (i, &n) in white.iter().enumerate() {
        let k = Rect::from_min_size(pos2(r.left() + i as f32 * kw, r.top()), vec2(kw - 1.0, r.height()));
        rect_fill(p, k, 1.0, if notes >> n & 1 == 1 { color } else { FAINT });
    }
    for n in lo..=hi {
        if matches!(n % 12, 1 | 3 | 6 | 8 | 10) {
            let i = white.iter().position(|&w| w > n).unwrap_or(white.len());
            let k = Rect::from_center_size(pos2(r.left() + i as f32 * kw - 0.5, r.top() + r.height() * 0.3), vec2(kw * 0.6, r.height() * 0.6));
            rect_fill(p, k, 1.0, if notes >> n & 1 == 1 { color } else { SCREEN_BG });
        }
    }
}

/// Grabs a handle: while dragged, returns where the pointer is.
fn handle(ui: &mut Ui, id: Id, at: Pos2, size: f32) -> (Response, Option<Pos2>) {
    let resp = ui.interact(Rect::from_center_size(at, vec2(size, size)), id, Sense::click_and_drag());
    if resp.hovered() || resp.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
    }
    let pos = resp.interact_pointer_pos().filter(|_| resp.dragged());
    (resp, pos)
}

/// A parameter's place between its minimum and maximum, logarithmically
/// for time-like ones.
fn log_frac(spec: &ParamSpec, v: f32) -> f32 {
    let (lo, hi) = (spec.min.max(1e-4), spec.max.max(spec.min + 1e-3));
    ((v.max(lo) / lo).ln() / (hi / lo).ln()).clamp(0.0, 1.0)
}

fn log_value(spec: &ParamSpec, frac: f32) -> f32 {
    let (lo, hi) = (spec.min.max(1e-4), spec.max.max(spec.min + 1e-3));
    lo * (hi / lo).powf(frac.clamp(0.0, 1.0))
}

// ---------------------------------------------------------------- particles

/// One grain in the jar: position (0..1 of the jar), speed, flash, layer.
type Grain = [f32; 6];

/// Particles: the shaker's objects in a jar, kicked by every shake, flashing
/// as they collide (each flash is a collision you hear).
pub(crate) fn particles(ui: &mut Ui, id: Id, p: &mut [f32], live: &InstLive, height: f32) -> bool {
    use shaker_synth::*;
    let rect = screen(ui, 236.0, height);
    let painter = ui.painter_at(rect);
    let model = (p[MODEL].round().max(0.0) as usize).min(MODEL_NAMES.len() - 1);
    let layer = (p[LAYER].round().max(0.0) as usize).min(LAYERS.len() - 1);
    let color_of = |m: usize| [accent_now(), YELLOW, GREEN, BLUE, WHITE, GREEN, YELLOW, BLUE, RED, accent_now(), RED][m % 11];
    // 0 dot, 1 jingle (ring), 2 stick, 3 dust.
    let glyph_of = |m: usize| [0, 0, 0, 1, 1, 2, 3, 1, 0, 0, 0][m % 11];
    let jar = Rect::from_min_max(pos2(rect.left() + 20.0, rect.top() + 22.0), pos2(rect.right() - 12.0, rect.bottom() - 12.0));
    let n_main = 6 + (p[OBJECTS].clamp(0.0, 1.0) * 48.0) as usize;
    let n_layer = if layer > 0 { 4 + (p[LAYER_MIX].clamp(0.0, 1.0) * 30.0) as usize } else { 0 };
    let radius = |l: bool| {
        let g = if l { 0.5 } else { p[GRAIN].clamp(0.0, 1.0) };
        if glyph_of(if l { layer - 1 } else { model }) == 3 {
            1.2
        } else {
            1.8 + g * 2.6
        }
    };

    let sid = id.with("grains");
    // seen: shake and collision counters, random seed, time of the last shake, its direction.
    let (mut grains, mut seen): (Vec<Grain>, [f32; 5]) = ui.ctx().memory(|m| m.data.get_temp(sid)).unwrap_or_else(|| (Vec::new(), [-1.0, -1.0, 12345.0, -10.0, 0.0]));
    let mut seed = seen[2] as u32 | 1;
    let total = n_main + n_layer;
    while grains.len() < total {
        let l = (grains.len() >= n_main) as u8 as f32;
        grains.push([rnd(&mut seed), 0.6 + 0.4 * rnd(&mut seed), 0.0, 0.0, 0.0, l]);
    }
    grains.truncate(total);
    for (i, g) in grains.iter_mut().enumerate() {
        g[5] = (i >= n_main) as u8 as f32;
    }

    let dt = ui.input(|i| i.stable_dt).clamp(0.001, 0.05);
    let energy = (live.vals[0] + live.vals[1]).clamp(0.0, 1.5);
    // New shakes and collisions since the last frame (the counters wrap at 2^24).
    let delta = |now: f32, before: f32| if before < 0.0 { 0.0 } else { (now - before).rem_euclid(16_777_216.0).min(500.0) };
    let shakes = delta(live.vals[2], seen[0]);
    let hits = delta(live.vals[3], seen[1]);
    seen[0] = live.vals[2];
    seen[1] = live.vals[3];
    let scatter = p[SCATTER].clamp(0.0, 1.0);
    // Air drag: a short shaker settles quicker.
    let friction = 0.25 + (1.0 - p[LENGTH].clamp(0.0, 1.0)) * 0.9;
    let rpx = radius(false);
    let per_row = ((jar.width() / (2.0 * rpx * 1.15)).floor() as usize).max(1);
    let mut moving = false;
    // A shake throws the grains up (a soft one a little, a hard one to the
    // lid); in a roll the jar goes up and down in turn, so they swirl. The
    // first shake after a pause always goes up.
    let force = (energy * 3.0).sqrt().min(1.0);
    let now = ui.input(|i| i.time) as f32;
    let mut up = true;
    if shakes > 0.0 {
        up = now - seen[3] > 0.3 || seen[4] < 0.5;
        seen[3] = now;
        seen[4] = if up { 1.0 } else { 0.0 };
    }
    for (i, g) in grains.iter_mut().enumerate() {
        if shakes > 0.0 {
            let kick = (2.0 + 4.6 * force) * (0.3 + 0.7 * rnd(&mut seed));
            if up {
                g[3] = -kick;
            } else {
                g[3] = g[3].max(kick * 0.7);
            }
            g[2] += bip(&mut seed) * (0.4 + 1.8 * scatter);
        }
        g[2] += bip(&mut seed) * energy.sqrt() * 14.0 * dt;
        g[3] += bip(&mut seed) * energy.sqrt() * 14.0 * dt + 9.0 * dt;
        let f = (1.0 - friction * dt).max(0.0);
        g[2] *= f;
        g[3] *= f;
        g[0] += g[2] * dt;
        g[1] += g[3] * dt;
        let r = radius(g[5] > 0.5);
        let (rx, ry) = (r / jar.width(), r / jar.height());
        // A heap: each grain rests on its own row.
        let floor = 1.0 - ry - ((i / per_row) as f32 * 2.0 * ry * 0.9).min(0.6);
        if g[0] < rx || g[0] > 1.0 - rx {
            g[0] = g[0].clamp(rx, 1.0 - rx);
            g[2] = -g[2] * 0.5;
        }
        if g[1] > floor {
            g[1] = floor;
            g[3] = -g[3] * 0.3;
            g[2] *= 0.8;
        }
        if g[1] < ry {
            g[1] = ry;
            g[3] = g[3].abs() * 0.5;
        }
        g[4] = (g[4] - dt * 7.0).max(0.0);
        moving |= g[2].abs() + g[3].abs() > 0.02;
    }
    // Every collision lights a grain.
    for _ in 0..(hits as usize).min(24) {
        let k = (rnd(&mut seed) * grains.len() as f32) as usize;
        if let Some(g) = grains.get_mut(k) {
            g[4] = 1.0;
        }
    }
    seen[2] = seed as f32;

    // The jar.
    rect_line(&painter, jar.expand(3.0), 8.0, 1.5, DIM);
    painter.line_segment([pos2(jar.center().x - 16.0, jar.top() - 7.0), pos2(jar.center().x + 16.0, jar.top() - 7.0)], Stroke::new(3.0_f32, DIM));
    // Energy, up the side.
    let bar = Rect::from_min_max(pos2(rect.left() + 8.0, jar.top()), pos2(rect.left() + 12.0, jar.bottom()));
    rect_fill(&painter, bar, 2.0, FAINT);
    let e = (energy * 3.0).sqrt().clamp(0.0, 1.0);
    rect_fill(&painter, Rect::from_min_max(pos2(bar.left(), bar.bottom() - e * bar.height()), bar.max), 2.0, color_of(model));
    let ring = p[RING].clamp(0.0, 1.0);
    for g in &grains {
        let l = g[5] > 0.5;
        let m = if l { layer.saturating_sub(1) } else { model };
        let base = color_of(m);
        let c = if l { mix(base, WHITE, 0.15) } else { base };
        let c = mix(c, WHITE, g[4] * 0.8);
        let r = radius(l) * (1.0 + g[4] * 0.4);
        let at = pos2(jar.left() + g[0] * jar.width(), jar.top() + g[1] * jar.height());
        if g[4] > 0.05 && ring > 0.2 {
            painter.circle_stroke(at, r * (1.6 + ring * 3.0 * (1.0 - g[4])), Stroke::new(1.0_f32, c.gamma_multiply(g[4] * ring * 0.6)));
        }
        match glyph_of(m) {
            1 => {
                painter.circle_stroke(at, r, Stroke::new(1.6_f32, c));
            }
            2 => {
                let a = g[2].atan2(-g[3].abs() - 0.3) + (g[0] * 7.0).sin();
                let d = vec2(a.sin(), a.cos()) * r * 1.8;
                painter.line_segment([at - d, at + d], Stroke::new(2.0_f32, c));
            }
            _ => {
                painter.circle_filled(at, r, c);
            }
        }
    }
    caption(&painter, pos2(rect.left() + 8.0, rect.top() + 6.0), Align2::LEFT_TOP, MODEL_NAMES[model], color_of(model));
    if layer > 0 {
        caption(&painter, pos2(rect.left() + 8.0 + 8.0 * MODEL_NAMES[model].len() as f32, rect.top() + 6.0), Align2::LEFT_TOP, &format!("+{}", LAYERS[layer]), mix(color_of(layer - 1), WHITE, 0.15));
    }
    if p[HOLD] >= 0.5 {
        caption(&painter, pos2(rect.right() - 8.0, rect.top() + 6.0), Align2::RIGHT_TOP, "hold", RED);
    }
    if moving || energy > 0.0 || shakes > 0.0 {
        ui.ctx().request_repaint();
    }
    ui.ctx().memory_mut(|m| m.data.insert_temp(sid, (grains, seen)));
    false
}

// ---------------------------------------------------------------- organ

/// Organ: the nine drawbars (drag them) and the rotating speaker seen from
/// above (click it for off, slow, fast), percussion lamp, held keys.
pub(crate) fn organ_screen(ui: &mut Ui, id: Id, p: &mut [f32], live: &InstLive, height: f32) -> bool {
    use organ::*;
    let w = 300.0;
    let rect = screen(ui, w, height);
    let mut changed = false;
    let top = rect.top() + 22.0;
    let bottom = rect.bottom() - 34.0;
    let col = 20.0;
    // Brown for the sub-octaves, white for the octaves, black for the rest.
    let bar_color = [
        Color32::from_rgb(0x9a, 0x5b, 0x2c),
        Color32::from_rgb(0x9a, 0x5b, 0x2c),
        WHITE,
        WHITE,
        Color32::from_rgb(0x30, 0x32, 0x38),
        WHITE,
        Color32::from_rgb(0x30, 0x32, 0x38),
        Color32::from_rgb(0x30, 0x32, 0x38),
        WHITE,
    ];
    let short = ["16", "5", "8", "4", "3", "2", "1.6", "1.3", "1"];
    for k in 0..9 {
        let x = rect.left() + 14.0 + k as f32 * col;
        let column = Rect::from_min_max(pos2(x - col / 2.0, top - 6.0), pos2(x + col / 2.0, bottom + 6.0));
        let resp = ui.interact(column, id.with(("bar", k)), Sense::click_and_drag());
        if let Some(pos) = resp.interact_pointer_pos().filter(|_| resp.dragged() || resp.clicked()) {
            let v = (((pos.y - top) / (bottom - top)) * 8.0).round().clamp(0.0, 8.0);
            if p[k] != v {
                p[k] = v;
                changed = true;
            }
        }
        let painter = ui.painter();
        let v = p[k].round().clamp(0.0, 8.0);
        let y = top + v / 8.0 * (bottom - top);
        painter.line_segment([pos2(x, top - 6.0), pos2(x, bottom + 6.0)], Stroke::new(6.0_f32, FAINT));
        for s in 1..=8 {
            let ty = top + s as f32 / 8.0 * (bottom - top);
            painter.line_segment([pos2(x - 5.0, ty), pos2(x - 3.0, ty)], Stroke::new(1.0_f32, DIM));
        }
        painter.line_segment([pos2(x, top - 6.0), pos2(x, y)], Stroke::new(2.0_f32, DIM));
        let knob = Rect::from_center_size(pos2(x, y), vec2(12.0, 9.0));
        rect_fill(painter, knob, 2.0, bar_color[k]);
        if k == 4 || k == 6 || k == 7 {
            rect_line(painter, knob, 2.0, 1.0, TEXT_DIM);
        }
        if resp.hovered() || resp.dragged() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
        }
        let painter = ui.painter();
        painter.text(pos2(x, bottom + 9.0), Align2::CENTER_TOP, short[k], FontId::monospace(8.0), TEXT_DIM);
        resp.on_hover_text(format!("{} drawbar: drag up or down ({})", DRAWBAR_NAMES[k], v as i32));
    }
    let painter = ui.painter_at(rect);
    caption(&painter, pos2(rect.left() + 8.0, rect.top() + 6.0), Align2::LEFT_TOP, &format!("{}{}{}{}{}{}{}{}{}", p[0] as i32, p[1] as i32, p[2] as i32, p[3] as i32, p[4] as i32, p[5] as i32, p[6] as i32, p[7] as i32, p[8] as i32), WHITE);

    // The rotating speaker, from above.
    let c = pos2(rect.right() - 63.0, rect.top() + 14.0 + (height - 64.0) * 0.5);
    let rad = ((height - 64.0) * 0.4).clamp(20.0, 46.0);
    let speed = p[ROTARY].round().clamp(0.0, 2.0) as usize;
    let rot = ui.interact(Rect::from_center_size(c, vec2(rad * 2.0, rad * 2.0)), id.with("rotor"), Sense::click());
    if rot.clicked() {
        p[ROTARY] = ((speed + 1) % 3) as f32;
        changed = true;
    }
    let painter = ui.painter_at(rect);
    painter.circle_stroke(c, rad, Stroke::new(1.5_f32, DIM));
    let drum = live.vals[1] * TAU;
    let arc: Vec<Pos2> = (0..=16).map(|i| {
        let a = drum - 0.7 + 1.4 * i as f32 / 16.0;
        c + vec2(a.cos(), a.sin()) * rad * 0.62
    }).collect();
    painter.add(Shape::line(arc, Stroke::new(6.0_f32, accent_now().gamma_multiply(0.8))));
    let horn = live.vals[0] * TAU;
    let d = vec2(horn.cos(), horn.sin());
    let side = vec2(-d.y, d.x);
    let mouth = c + d * rad * 0.88;
    painter.line_segment([c - d * rad * 0.45, mouth], Stroke::new(2.0_f32, WHITE));
    painter.add(Shape::convex_polygon(vec![c + d * rad * 0.55 + side * 2.0, mouth + side * 8.0, mouth - side * 8.0, c + d * rad * 0.55 - side * 2.0], WHITE, Stroke::NONE));
    painter.circle_filled(c - d * rad * 0.45, 4.0, DIM);
    painter.circle_filled(c, 3.0, WHITE);
    caption(&painter, pos2(c.x, c.y + rad + 4.0), Align2::CENTER_TOP, ["rotary off", "slow", "fast"][speed], if speed == 2 { accent_now() } else { TEXT_DIM });
    rot.on_hover_text("Click: off, slow, fast");

    // Percussion lamp.
    let lamp = pos2(rect.right() - 112.0, rect.top() + 12.0);
    let lr = ui.interact(Rect::from_center_size(lamp + vec2(14.0, 0.0), vec2(44.0, 14.0)), id.with("perc"), Sense::click());
    if lr.clicked() {
        p[PERC] = if p[PERC] >= 0.5 { 0.0 } else { 1.0 };
        changed = true;
    }
    let painter = ui.painter_at(rect);
    let on = p[PERC] >= 0.5;
    painter.circle_filled(lamp, 4.0, if on { mix(DIM, RED, 0.4 + 0.6 * (live.vals[2] * 3.0).min(1.0)) } else { FAINT });
    caption(&painter, lamp + vec2(8.0, 0.0), Align2::LEFT_CENTER, "perc", if on { RED } else { DIM });
    lr.on_hover_text("Click: percussion on or off");
    let vib = (p[VIBRATO].round().max(0.0) as usize).min(VIBRATOS.len() - 1);
    caption(&painter, pos2(rect.right() - 8.0, rect.top() + 6.0), Align2::RIGHT_TOP, &format!("vib {}", VIBRATOS[vib].to_lowercase()), if vib > 0 { BLUE } else { DIM });
    keys(&painter, Rect::from_min_max(pos2(rect.right() - 118.0, rect.bottom() - 20.0), pos2(rect.right() - 8.0, rect.bottom() - 6.0)), live.notes, 36, 84, GREEN);
    if live.vals[3] > 0.0 || speed > 0 && live.level() > 1e-4 {
        ui.ctx().request_repaint();
    }
    changed
}

// ---------------------------------------------------------------- subtractive

#[derive(Clone, Copy, PartialEq)]
enum Flt {
    Lp24,
    Lp12,
    Bp,
    Hp,
    Notch,
}

/// Where a subtractive synth keeps its settings.
struct SubLayout {
    specs: &'static [ParamSpec],
    /// Oscillator waves (0 saw, 1 pulse, 2 triangle, 3 sine) and pulse widths.
    osc: fn(&[f32]) -> [(usize, f32); 2],
    /// Osc 2's frequency as a ratio of osc 1's.
    ratio: fn(&[f32]) -> f32,
    /// Levels of osc 1, osc 2, sub and noise.
    levels: fn(&[f32]) -> [f32; 4],
    filter: fn(&[f32]) -> Flt,
    cutoff: usize,
    reso: usize,
    amp: [usize; 4],
    flt: [usize; 4],
    /// LFO wave (as `lfo_part` draws them) and rate (Hz).
    lfo: fn(&[f32]) -> (usize, f32),
    /// Which parameter group shows which screen.
    groups: &'static [(&'static str, Part)],
}

#[derive(Clone, Copy)]
enum Part {
    Osc(usize),
    Mix,
    Filter,
    Env(bool),
    Lfo,
}

static ANALOG: SubLayout = SubLayout {
    specs: analog::SPECS,
    osc: |p| [(p[analog::WAVE1].round() as usize, p[analog::PW1]), (p[analog::WAVE2].round() as usize, p[analog::PW2])],
    ratio: |p| 2f32.powf(p[analog::OCTAVE2].round() - p[analog::OCTAVE1].round() + p[analog::SEMI2].round() / 12.0 + p[analog::DETUNE2] / 1200.0),
    levels: |p| [p[analog::MIX1], p[analog::MIX2], p[analog::SUB], p[analog::NOISE]],
    filter: |p| match p[analog::FILTER].round() as i32 {
        0 => Flt::Lp24,
        1 => Flt::Lp12,
        2 => Flt::Bp,
        _ => Flt::Hp,
    },
    cutoff: analog::CUTOFF,
    reso: analog::RESONANCE,
    amp: [analog::ATTACK, analog::DECAY, analog::SUSTAIN, analog::RELEASE],
    flt: [analog::F_ATTACK, analog::F_DECAY, analog::F_SUSTAIN, analog::F_RELEASE],
    lfo: |p| (p[analog::LFO_WAVE].round().max(0.0) as usize, p[analog::LFO_RATE]),
    groups: &[("Osc 1", Part::Osc(0)), ("Osc 2", Part::Osc(1)), ("Mixer", Part::Mix), ("Filter", Part::Filter), ("Filter Env", Part::Env(false)), ("Amp Env", Part::Env(true)), ("LFO", Part::Lfo)],
};

static SYNTH: SubLayout = SubLayout {
    specs: synth::SPECS,
    osc: |p| [(p[synth::OSC1_WAVE].round() as usize, 0.5), (p[synth::OSC2_WAVE].round() as usize, 0.5)],
    ratio: |p| 2f32.powf(p[synth::OSC2_SEMI].round() / 12.0 + p[synth::OSC2_FINE] / 1200.0),
    levels: |p| [1.0 - p[synth::OSC_MIX], p[synth::OSC_MIX], p[synth::SUB], p[synth::NOISE]],
    filter: |p| match p[synth::FILTER_MODE].round() as i32 {
        0 => Flt::Lp12,
        1 => Flt::Hp,
        2 => Flt::Bp,
        _ => Flt::Notch,
    },
    cutoff: synth::CUTOFF,
    reso: synth::RESO,
    amp: [synth::AMP_A, synth::AMP_D, synth::AMP_S, synth::AMP_R],
    flt: [synth::FLT_A, synth::FLT_D, synth::FLT_S, synth::FLT_R],
    lfo: |p| (1, p[synth::LFO_RATE]),
    groups: &[("Oscillators", Part::Mix), ("Filter", Part::Filter), ("Amp Env", Part::Env(true)), ("Filter Env", Part::Env(false)), ("LFO", Part::Lfo)],
};

fn osc_at(wave: usize, pw: f32, ph: f32) -> f32 {
    let ph = ph.rem_euclid(1.0);
    match wave {
        0 => 1.0 - 2.0 * ph,
        1 => {
            if ph < pw.clamp(0.05, 0.95) {
                1.0
            } else {
                -1.0
            }
        }
        2 => 1.0 - 4.0 * (ph - 0.5).abs(),
        _ => (ph * TAU).sin(),
    }
}

/// Gain of the filter at `x` = frequency / cutoff.
fn filter_gain(kind: Flt, x: f32, q: f32) -> f32 {
    let d = ((1.0 - x * x).powi(2) + (x / q).powi(2)).sqrt().max(1e-6);
    match kind {
        Flt::Lp12 => 1.0 / d,
        Flt::Lp24 => 1.0 / d / ((1.0 - x * x).powi(2) + (x / 0.6).powi(2)).sqrt().max(1e-6),
        Flt::Bp => (x / q) / d,
        Flt::Hp => x * x / d,
        Flt::Notch => (1.0 - x * x).abs() / d,
    }
}

/// Analog with screens by its dials: each oscillator's wave, the mix, the
/// filter, the envelopes (drag their handles) and the LFO.
pub(crate) fn analog_grid(ui: &mut Ui, id: Id, p: &mut [f32], live: &InstLive, height: f32, mv: Option<&mut super::modulation::ModView>) -> bool {
    sub_grid(ui, id, &ANALOG, p, live, height, mv)
}

pub(crate) fn synth_grid(ui: &mut Ui, id: Id, p: &mut [f32], live: &InstLive, height: f32, mv: Option<&mut super::modulation::ModView>) -> bool {
    sub_grid(ui, id, &SYNTH, p, live, height, mv)
}

fn sub_grid(ui: &mut Ui, id: Id, s: &'static SubLayout, p: &mut [f32], live: &InstLive, height: f32, mv: Option<&mut super::modulation::ModView>) -> bool {
    let part = |name: &str| s.groups.iter().find(|g| g.0 == name).map(|g| g.1);
    let changed = super::devices::param_grid_screens(ui, id, s.specs, p, accent_now(), height, mv, 104.0, &|name| part(name).is_some(), |ui, name, rect, p| {
        let Some(which) = part(name) else { return false };
        let painter = ui.painter_at(rect);
        rect_fill(&painter, rect, 4.0, SCREEN_BG);
        match which {
            Part::Osc(k) => {
                osc_part(&painter, rect, s, p, Some(k));
                false
            }
            Part::Mix => {
                osc_part(&painter, rect, s, p, None);
                false
            }
            Part::Filter => {
                filter_part(&painter, rect, s, p, live);
                false
            }
            Part::Env(amp) => env_part(ui, id.with(("env", amp)), rect, s, p, live, amp),
            Part::Lfo => {
                lfo_part(ui, &painter, rect, s, p);
                false
            }
        }
    });
    if live.vals[0] > 0.0 {
        ui.ctx().request_repaint();
    }
    changed
}

/// One oscillator (two periods of osc 1, so osc 2's tuning shows as more or
/// fewer cycles), or the mix of both with the sub and noise.
fn osc_part(painter: &Painter, rect: Rect, s: &SubLayout, p: &[f32], which: Option<usize>) {
    let r = rect.shrink2(vec2(5.0, 6.0)).translate(vec2(0.0, 5.0));
    let osc = (s.osc)(p);
    let ratio = (s.ratio)(p);
    let lv = (s.levels)(p);
    let names = ["saw", "pulse", "tri", "sine"];
    let mut seed = 99u32;
    let pts: Vec<Pos2> = (0..=120)
        .map(|i| {
            let ph = i as f32 / 120.0 * 2.0;
            let y = match which {
                Some(0) => osc_at(osc[0].0, osc[0].1, ph),
                Some(_) => osc_at(osc[1].0, osc[1].1, ph * ratio),
                None => {
                    let norm = (lv[0] + lv[1] + lv[2]).max(0.2);
                    (osc_at(osc[0].0, osc[0].1, ph) * lv[0] + osc_at(osc[1].0, osc[1].1, ph * ratio) * lv[1] + (ph * 0.5 * TAU).sin() * lv[2]) / norm + bip(&mut seed) * lv[3] * 0.3
                }
            };
            pos2(r.left() + i as f32 / 120.0 * r.width(), r.center().y - y.clamp(-1.2, 1.2) * r.height() * 0.4)
        })
        .collect();
    painter.line_segment([pos2(r.left(), r.center().y), pos2(r.right(), r.center().y)], Stroke::new(1.0_f32, FAINT));
    // A silent oscillator is drawn dim.
    let level = match which {
        Some(k) => lv[k],
        None => 1.0,
    };
    painter.add(Shape::line(pts, Stroke::new(1.6_f32, BLUE.gamma_multiply(0.35 + 0.65 * level.clamp(0.0, 1.0)))));
    let label = match which {
        Some(k) => names[osc[k].0.min(3)].to_string(),
        None => "mix".to_string(),
    };
    caption(painter, rect.left_top() + vec2(5.0, 4.0), Align2::LEFT_TOP, &label, BLUE);
}

/// The filter's response at the knob (dim) and where the newest voice's
/// envelope has it now.
fn filter_part(painter: &Painter, rect: Rect, s: &SubLayout, p: &[f32], live: &InstLive) {
    let r = rect.shrink2(vec2(4.0, 6.0)).translate(vec2(0.0, 6.0));
    let kind = (s.filter)(p);
    let q = 0.55 + p[s.reso].clamp(0.0, 1.0).powf(1.5) * 9.0;
    let (f_lo, f_hi) = (30.0_f32, 18000.0_f32);
    let curve = |fc: f32| -> Vec<Pos2> {
        (0..=60)
            .map(|i| {
                let f = f_lo * (f_hi / f_lo).powf(i as f32 / 60.0);
                let db = 20.0 * filter_gain(kind, f / fc.max(1.0), q).max(1e-4).log10();
                pos2(r.left() + i as f32 / 60.0 * r.width(), r.top() + r.height() * (1.0 - ((db + 30.0) / 48.0).clamp(0.0, 1.0)))
            })
            .collect()
    };
    let live_cut = if live.vals[0] > 0.0 { live.vals[25] } else { 0.0 };
    painter.add(Shape::line(curve(p[s.cutoff]), Stroke::new(if live_cut > 0.0 { 1.0_f32 } else { 1.8 }, if live_cut > 0.0 { DIM } else { accent_now() })));
    if live_cut > 0.0 {
        painter.add(Shape::line(curve(live_cut), Stroke::new(1.8_f32, accent_now())));
    }
    let kname = match kind {
        Flt::Lp24 => "lp24",
        Flt::Lp12 => "lp12",
        Flt::Bp => "bp",
        Flt::Hp => "hp",
        Flt::Notch => "notch",
    };
    let fc = if live_cut > 0.0 { live_cut } else { p[s.cutoff] };
    let fc_s = if fc >= 1000.0 { format!("{:.1}k", fc / 1000.0) } else { format!("{fc:.0}") };
    caption(painter, rect.left_top() + vec2(5.0, 4.0), Align2::LEFT_TOP, &format!("{kname} {fc_s}"), accent_now());
}

/// An envelope with handles to drag (attack; decay and sustain; release)
/// and the voices running along it.
fn env_part(ui: &mut Ui, id: Id, rect: Rect, s: &SubLayout, p: &mut [f32], live: &InstLive, amp: bool) -> bool {
    let idx = if amp { s.amp } else { s.flt };
    let color = if amp { GREEN } else { accent_now() };
    let env_r = Rect::from_min_max(rect.left_top() + vec2(6.0, 16.0), rect.right_bottom() - vec2(6.0, 6.0));
    let seg_max = env_r.width() * 0.27;
    let sus_w = env_r.width() - 3.0 * seg_max;
    let a = log_frac(&s.specs[idx[0]], p[idx[0]]).max(0.02) * seg_max;
    let d = log_frac(&s.specs[idx[1]], p[idx[1]]).max(0.02) * seg_max;
    let rl = log_frac(&s.specs[idx[3]], p[idx[3]]).max(0.02) * seg_max;
    let sus = p[idx[2]].clamp(0.0, 1.0);
    let x0 = env_r.left();
    let (xa, xd) = (x0 + a, x0 + a + d);
    let xs = xd + sus_w * 0.6;
    let xr = xs + rl;
    let y_of = |l: f32| env_r.bottom() - l.clamp(0.0, 1.05) * env_r.height();
    let mut pts = vec![pos2(x0, y_of(0.0))];
    for i in 1..=8 {
        let t = i as f32 / 8.0;
        pts.push(pos2(x0 + (xa - x0) * t, y_of(1.0 - (1.0 - t).powf(1.6))));
    }
    for i in 1..=10 {
        let t = i as f32 / 10.0;
        pts.push(pos2(xa + (xd - xa) * t, y_of(sus + (1.0 - sus) * (-4.0 * t).exp() * (1.0 - t))));
    }
    pts.push(pos2(xs, y_of(sus)));
    for i in 1..=10 {
        let t = i as f32 / 10.0;
        pts.push(pos2(xs + (xr - xs) * t, y_of(sus * (-4.0 * t).exp() * (1.0 - t))));
    }
    let painter = ui.painter_at(rect);
    painter.add(Shape::line(pts, Stroke::new(1.8_f32, color)));
    caption(&painter, rect.left_top() + vec2(5.0, 4.0), Align2::LEFT_TOP, if amp { "amp" } else { "filter" }, color);
    // The voices: where each is on the envelope.
    let voices = (live.vals[0].round() as usize).min(6);
    for k in (0..voices).rev() {
        let (level, stage) = if amp { (live.vals[1 + k * 4], live.vals[2 + k * 4]) } else { (live.vals[3 + k * 4], live.vals[4 + k * 4]) };
        if stage <= 0.0 {
            continue;
        }
        let x = match stage.round() as i32 {
            1 => x0 + (xa - x0) * level.clamp(0.0, 1.0),
            2 => {
                let t = if 1.0 - sus > 0.01 { ((1.0 - level) / (1.0 - sus)).clamp(0.0, 1.0) } else { 1.0 };
                if t > 0.97 {
                    (xd + xs) * 0.5
                } else {
                    xa + (xd - xa) * t
                }
            }
            _ => xs + (xr - xs) * (1.0 - level / sus.max(0.05)).clamp(0.0, 1.0),
        };
        painter.circle_filled(pos2(x, y_of(level)), 3.0, WHITE.gamma_multiply(1.0 - k as f32 * 0.13));
    }
    // Handles.
    let mut changed = false;
    let set = |p: &mut [f32], i: usize, v: f32, changed: &mut bool| {
        if (p[i] - v).abs() > 1e-6 {
            p[i] = v;
            *changed = true;
        }
    };
    if let (_, Some(pos)) = handle(ui, id.with(0), pos2(xa, y_of(1.0)), 12.0) {
        set(p, idx[0], log_value(&s.specs[idx[0]], (pos.x - x0) / seg_max), &mut changed);
    }
    if let (_, Some(pos)) = handle(ui, id.with(1), pos2(xd, y_of(sus)), 12.0) {
        set(p, idx[1], log_value(&s.specs[idx[1]], (pos.x - xa) / seg_max), &mut changed);
        set(p, idx[2], ((env_r.bottom() - pos.y) / env_r.height()).clamp(0.0, 1.0), &mut changed);
    }
    if let (_, Some(pos)) = handle(ui, id.with(2), pos2(xr, y_of(0.0)), 12.0) {
        set(p, idx[3], log_value(&s.specs[idx[3]], (pos.x - xs) / seg_max), &mut changed);
    }
    let painter = ui.painter_at(rect);
    for at in [pos2(xa, y_of(1.0)), pos2(xd, y_of(sus)), pos2(xr, y_of(0.0))] {
        painter.circle_filled(at, 3.0, SCREEN_BG);
        painter.circle_stroke(at, 3.0, Stroke::new(1.5_f32, color));
    }
    changed
}

/// The LFO's wave with a dot running along it at its rate.
fn lfo_part(ui: &Ui, painter: &Painter, rect: Rect, s: &SubLayout, p: &[f32]) {
    let r = rect.shrink2(vec2(5.0, 6.0)).translate(vec2(0.0, 5.0));
    let (wave, rate) = (s.lfo)(p);
    let shape = |ph: f32| -> f32 {
        let ph = ph.rem_euclid(1.0);
        match wave {
            0 => 1.0 - 4.0 * (ph - 0.5).abs(),
            1 => (ph * TAU).sin(),
            2 => 1.0 - 2.0 * ph,
            3 => {
                if ph < 0.5 {
                    1.0
                } else {
                    -1.0
                }
            }
            // Sample and hold: steps that look random.
            _ => (((ph * 4.0).floor() * 12.9898).sin() * 43758.545).fract() * 2.0 - 1.0,
        }
    };
    let pts: Vec<Pos2> = (0..=100).map(|i| {
        let ph = i as f32 / 100.0 * 2.0;
        pos2(r.left() + i as f32 / 100.0 * r.width(), r.center().y - shape(ph) * r.height() * 0.4)
    }).collect();
    painter.add(Shape::line(pts, Stroke::new(1.5_f32, YELLOW)));
    let ph = (ui.input(|i| i.time) as f32 * rate).fract() * 2.0 % 2.0;
    painter.circle_filled(pos2(r.left() + ph / 2.0 * r.width(), r.center().y - shape(ph) * r.height() * 0.4), 3.0, WHITE);
    caption(painter, rect.left_top() + vec2(5.0, 4.0), Align2::LEFT_TOP, &format!("{rate:.1} hz"), YELLOW);
    ui.ctx().request_repaint_after(std::time::Duration::from_millis(33));
}

// ---------------------------------------------------------------- rings

/// Rings: a string per voice, vibrating as loud as the sound, with as many
/// overtones as the brightness lets through, plucked at Position.
pub(crate) fn rings_screen(ui: &mut Ui, p: &[f32], live: &InstLive, height: f32) {
    use rings_synth::*;
    let w = 240.0;
    let rect = screen(ui, w, height);
    let painter = ui.painter_at(rect);
    let model = (p[MODEL].round().max(0.0) as usize).min(MODELS.len() - 1);
    let color = [BLUE, GREEN, accent_now(), RED, YELLOW, WHITE][model % 6];
    let strings = [1, 2, 4][(p[POLYPHONY].round().max(0.0) as usize).min(2)];
    let t = ui.input(|i| i.time) as f32;
    let level = (live.level() * 6.0).min(1.0);
    let held: Vec<u8> = live.held().collect();
    let area = Rect::from_min_max(pos2(rect.left() + 34.0, rect.top() + 24.0), pos2(rect.right() - 12.0, rect.bottom() - 14.0));
    let modes = 1 + (p[BRIGHTNESS].clamp(0.0, 1.0) * 6.0) as usize;
    let stretch = 1.0 + p[STRUCTURE].clamp(0.0, 1.0) * 0.6;
    let pos = p[POSITION].clamp(0.0, 1.0);
    for s in 0..strings {
        let y0 = area.top() + (s as f32 + 0.5) / strings as f32 * area.height();
        let amp = level * (area.height() / strings as f32 * 0.4) * if s < held.len().max(1) { 1.0 } else { 0.35 };
        let pts: Vec<Pos2> = (0..=80)
            .map(|i| {
                let x = i as f32 / 80.0;
                let mut y = 0.0;
                for k in 1..=modes {
                    let kf = (k as f32).powf(stretch);
                    // Each mode is as strong as the pluck point excites it.
                    y += (kf * PI * x).sin() * (kf * PI * pos).sin() * (t * (3.0 + s as f32) * kf + k as f32).cos() / kf;
                }
                pos2(area.left() + x * area.width(), y0 + y * amp)
            })
            .collect();
        painter.add(Shape::line(pts, Stroke::new(1.6_f32, if level > 0.01 { color } else { DIM })));
        painter.circle_filled(pos2(area.left(), y0), 2.5, DIM);
        painter.circle_filled(pos2(area.right(), y0), 2.5, DIM);
        let px = area.left() + pos * area.width();
        painter.add(Shape::convex_polygon(vec![pos2(px, y0 - 7.0), pos2(px - 3.5, y0 - 12.0), pos2(px + 3.5, y0 - 12.0)], DIM, Stroke::NONE));
        if let Some(&n) = held.get(s) {
            caption(&painter, pos2(rect.left() + 8.0, y0), Align2::LEFT_CENTER, &note_name(n), color);
        }
    }
    caption(&painter, pos2(rect.left() + 8.0, rect.top() + 6.0), Align2::LEFT_TOP, MODELS[model], color);
    if p[EGG] >= 0.5 {
        caption(&painter, pos2(rect.right() - 8.0, rect.top() + 6.0), Align2::RIGHT_TOP, "peace", RED);
    }
    if level > 0.005 {
        ui.ctx().request_repaint();
    }
}

// ---------------------------------------------------------------- elements

/// Elements: the three exciters (bow, blow, strike) feeding the resonator,
/// whose rim wobbles with its modes; the space around it rings out.
pub(crate) fn elements_screen(ui: &mut Ui, id: Id, p: &[f32], live: &InstLive, height: f32) {
    use elements_synth::*;
    let w = 250.0;
    let rect = screen(ui, w, height);
    let painter = ui.painter_at(rect);
    let t = ui.input(|i| i.time) as f32;
    let level = (live.level() * 5.0).min(1.0);
    // A strike shows as a flash when the sound jumps up.
    let prev_id = id.with("prev");
    let prev: f32 = ui.ctx().memory(|m| m.data.get_temp(prev_id)).unwrap_or(0.0);
    let flash_id = id.with("flash");
    let mut flash: f32 = ui.ctx().memory(|m| m.data.get_temp(flash_id)).unwrap_or(0.0);
    if level > prev * 1.6 + 0.05 {
        flash = 1.0;
    }
    flash = (flash - ui.input(|i| i.stable_dt) * 4.0).max(0.0);
    ui.ctx().memory_mut(|m| {
        m.data.insert_temp(prev_id, level);
        m.data.insert_temp(flash_id, flash);
    });
    let col_x = rect.left() + 40.0;
    let rows = [rect.top() + 34.0, rect.center().y + 4.0, rect.bottom() - 26.0];
    let (bow, blow, strike) = (p[BOW].clamp(0.0, 1.0), p[BLOW].clamp(0.0, 1.0), p[STRIKE].clamp(0.0, 1.0));
    // Bow: a bow sliding back and forth.
    let bc = mix(DIM, BLUE, bow);
    let slide = (t * (1.5 + 4.0 * p[BOW_TIMBRE])).sin() * 10.0 * bow * level.max(0.15);
    painter.line_segment([pos2(col_x - 22.0 + slide, rows[0] - 4.0), pos2(col_x + 14.0 + slide, rows[0] - 4.0)], Stroke::new(2.0_f32, bc));
    for k in 0..6 {
        let x = col_x - 20.0 + k as f32 * 6.0 + slide;
        painter.line_segment([pos2(x, rows[0] - 4.0), pos2(x + 3.0, rows[0] + 3.0)], Stroke::new(1.0_f32, bc));
    }
    // Blow: air streaming in.
    let ac = mix(DIM, GREEN, blow);
    for k in 0..8 {
        let ph = (t * (0.6 + 1.8 * p[FLOW]) + k as f32 * 0.37).fract();
        let x = col_x - 24.0 + ph * 40.0;
        let y = rows[1] + ((k * 7) % 5) as f32 * 2.5 - 5.0;
        painter.line_segment([pos2(x, y), pos2(x + 5.0, y)], Stroke::new(1.5_f32, ac.gamma_multiply(1.0 - ph)));
    }
    // Strike: a mallet that swings in when struck.
    let sc = mix(DIM, accent_now(), strike);
    let swing = flash * strike;
    let head = pos2(col_x + 8.0 - swing * 6.0, rows[2] - 8.0 + swing * 6.0);
    painter.line_segment([pos2(col_x - 20.0, rows[2] + 6.0), head], Stroke::new(1.5_f32, sc));
    painter.circle_filled(head, 4.0 + p[MALLET].clamp(0.0, 1.0) * 2.0, sc);
    for (k, (name, v)) in [("bow", bow), ("blow", blow), ("strike", strike)].iter().enumerate() {
        caption(&painter, pos2(rect.left() + 6.0, rows[k] + 9.0), Align2::LEFT_TOP, name, if *v > 0.01 { TEXT_DIM } else { FAINT });
    }
    // The resonator.
    let c = pos2(rect.right() - 82.0, rect.center().y + 6.0);
    let rad = ((height - 40.0) * 0.36).clamp(18.0, 52.0);
    let space = p[SPACE].clamp(0.0, 1.0);
    for k in 0..3 {
        let ph = (t * 0.5 + k as f32 / 3.0).fract();
        painter.circle_stroke(c, rad * (1.1 + ph * 0.5 * space), Stroke::new(1.0_f32, WHITE.gamma_multiply(space * level * (1.0 - ph) * 0.5)));
    }
    let geometry = p[GEOMETRY].clamp(0.0, 1.0);
    let modes = 2 + (p[BRIGHTNESS].clamp(0.0, 1.0) * 6.0) as usize;
    let amp = rad * 0.18 * level;
    let pts: Vec<Pos2> = (0..=120)
        .map(|i| {
            let a = i as f32 / 120.0 * TAU;
            let mut r = rad;
            for k in 2..=modes {
                let kf = k as f32 * (1.0 + geometry * 0.7 * (k as f32).sqrt() / 2.0);
                r += (kf * a + t * kf * 1.3).sin() * amp / k as f32;
            }
            c + vec2(a.cos(), a.sin()) * r
        })
        .collect();
    let model = (p[MODEL].round().max(0.0) as usize).min(MODEL_NAMES.len() - 1);
    let rc = mix(WHITE, mix(BLUE, accent_now(), geometry), 0.6);
    painter.add(Shape::closed_line(pts, Stroke::new(1.8_f32, if level > 0.01 { rc } else { DIM })));
    for (k, (v, col)) in [(bow, BLUE), (blow, GREEN), (strike, accent_now())].iter().enumerate() {
        if *v > 0.01 {
            let y = rows[k];
            painter.line_segment([pos2(col_x + 24.0, y), pos2(c.x - rad - 6.0, c.y + (k as f32 - 1.0) * 8.0)], Stroke::new(1.0_f32, col.gamma_multiply(0.25 + 0.6 * v * level.max(0.2))));
        }
    }
    caption(&painter, pos2(rect.left() + 6.0, rect.top() + 6.0), Align2::LEFT_TOP, MODEL_NAMES[model], WHITE);
    caption(&painter, pos2(c.x, rect.top() + 6.0), Align2::CENTER_TOP, geometry_name(p[MODEL]), TEXT_DIM);
    if level > 0.005 || flash > 0.0 {
        ui.ctx().request_repaint();
    }
}

// ---------------------------------------------------------------- STK families

/// Notes played lately, with how long ago they were let go (0 while held):
/// short notes from a sequencer show too.
fn recent_notes(ui: &Ui, id: Id, live: &InstLive) -> Vec<(u8, f32)> {
    let mid = id.with("recent");
    let dt = ui.input(|i| i.stable_dt).clamp(0.001, 0.1);
    let mut v: Vec<(u8, f32)> = ui.ctx().memory(|m| m.data.get_temp(mid)).unwrap_or_default();
    for e in v.iter_mut() {
        e.1 += dt;
    }
    for n in live.held() {
        match v.iter_mut().find(|e| e.0 == n) {
            Some(e) => e.1 = 0.0,
            None => v.push((n, 0.0)),
        }
    }
    v.retain(|e| e.1 < 2.5);
    v.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    v.truncate(8);
    ui.ctx().memory_mut(|m| m.data.insert_temp(mid, v.clone()));
    v
}

/// The STK instruments: bars or a bowl for the mallets, strings, and a
/// tube with air for the winds.
pub(crate) fn stk_screen(ui: &mut Ui, id: Id, family: Family, p: &[f32], live: &InstLive, height: f32) {
    let models = stk::models(family);
    let k = stk::model_of(family, p);
    let name = models[k].name;
    let level = (live.level() * 6.0).min(1.0);
    let recent = recent_notes(ui, id, live);
    // The model's controls by name, with their values (0..1).
    let ctl: Vec<(&str, f32)> = stk::controls(&models[k]).iter().enumerate().filter_map(|(i, c)| c.map(|c| (c.name, p.get(stk::CTL0 + i).copied().unwrap_or(0.0)))).collect();
    match family {
        Family::Mallets => mallets_screen(ui, k, name, level, &recent, &ctl, height),
        Family::Strings => strings_screen(ui, k, name, level, &recent, height),
        Family::Winds => winds_screen(ui, k, name, level, &recent, height),
    }
    if level > 0.003 || !recent.is_empty() {
        ui.ctx().request_repaint();
    }
}

/// The value (0..1) of the model's control called `name`, if it has one.
fn control(ctl: &[(&str, f32)], name: &str) -> Option<f32> {
    ctl.iter().find(|c| c.0 == name).map(|c| c.1.clamp(0.0, 1.0))
}

/// A small legend: a dot in the colour of what it describes, a name, a value.
fn legend(painter: &Painter, at: Pos2, color: Color32, name: &str, v: f32) -> f32 {
    painter.circle_filled(at + vec2(3.0, 0.0), 3.0, color);
    let r = caption(painter, at + vec2(9.0, 0.0), Align2::LEFT_CENTER, &format!("{name} {:.0}", v * 100.0), TEXT_DIM);
    r.right() + 10.0 - at.x
}

/// Voice: a little choir of singers (as many as Singers). Their mouths make
/// the vowel, morphing between the two as the engine does; louder singing
/// opens them further and lifts eyebrows and arms; vibrato sways their
/// heads; breath puffs; with an undertone they become monks in robes,
/// eyes closed, the chant rumbling in their chests; with an overtone a
/// whistling line rises, on the harmonic it sings.
pub(crate) fn voice_screen(ui: &mut Ui, id: Id, p: &[f32], live: &InstLive, height: f32) {
    use krabton_engine::dsp::stk_voice::*;
    let w = 280.0;
    let rect = screen(ui, w, height);
    let painter = ui.painter_at(rect);
    let t = ui.input(|i| i.time) as f32;
    let level = (live.level() * 6.0).min(1.0);
    let singing = level > 0.02;
    let sing = level;
    let morph = if live.vals[0] > 0.0 { live.vals[1] } else { p[MORPH] };
    let (va, vb) = ((p[VOWEL].round().max(0.0) as usize).min(MODELS.len() - 1), (p[VOWEL_B].round().max(0.0) as usize).min(MODELS.len() - 1));
    let (forms, _) = blend(va, vb, morph, 0.0);
    let open = ((forms[0].0 - 250.0) / 600.0).clamp(0.0, 1.0);
    let wide = ((forms[1].0 - 700.0) / 1700.0).clamp(0.0, 1.0);
    let breath = p[BREATH].clamp(0.0, 1.0);
    let vib = p[VIB_DEPTH].clamp(0.0, 1.0);
    let under = p[UNDERTONE].clamp(0.0, 1.0);
    let over = p[OVERTONE].clamp(0.0, 1.0);
    let monk = under > 0.3;
    let n = (p[SINGERS].round() as usize).clamp(1, 4);
    // Bigger singers for lower formants.
    let size = (1.0 - 0.25 * p[FORMANT].clamp(-1.0, 1.0)) * if n > 2 { 0.72 } else if n == 2 { 0.85 } else { 1.0 };
    let head_r = ((height * 0.17).clamp(18.0, 30.0) * size).max(12.0);
    let note = live.held().next();
    let lift = note.map(|n| ((n as f32 - 60.0) / 24.0).clamp(-0.5, 0.8)).unwrap_or(0.0);
    let skin = Color32::from_rgb(0xf2, 0xd4, 0xb8);
    let robe = Color32::from_rgb(0xd8, 0x6a, 0x1c);
    let shirts = [BLUE, GREEN, RED, YELLOW];
    let brown = Color32::from_rgb(0x5a, 0x3a, 0x22);
    let span = rect.width() - 90.0;
    for s in 0..n {
        let x = rect.left() + 20.0 + span * (s as f32 + 0.5) / n as f32;
        let back = (s % 2 == 1 && n > 2) as u8 as f32;
        let sway = if singing { (t * p[VIB_RATE] * TAU + s as f32 * 1.7).sin() * vib * 5.0 } else { 0.0 };
        let hc = pos2(x + sway, rect.bottom() - head_r * 2.4 - 6.0 - lift * 4.0 - back * 10.0);
        let cloth = if monk { robe } else { shirts[s % 4] };
        let shoulders = hc.y + head_r + 5.0;
        let body = vec![pos2(hc.x - sway - head_r * 1.15, rect.bottom()), pos2(hc.x - sway - head_r * 0.8, shoulders), pos2(hc.x - sway + head_r * 0.8, shoulders), pos2(hc.x - sway + head_r * 1.15, rect.bottom())];
        painter.add(Shape::convex_polygon(body, cloth, Stroke::NONE));
        if monk {
            // The chant rumbling in the chest.
            for k in 0..3 {
                let ph = (t * 1.3 + k as f32 / 3.0).fract();
                painter.circle_stroke(pos2(hc.x - sway, shoulders + head_r * 0.9), head_r * (0.3 + ph * 0.6), Stroke::new(1.2_f32, robe.gamma_multiply(1.6).gamma_multiply(sing * under * (1.0 - ph))));
            }
            // A sash over the shoulder.
            painter.line_segment([pos2(hc.x - sway - head_r * 0.7, shoulders + 2.0), pos2(hc.x - sway + head_r * 0.9, rect.bottom())], Stroke::new(3.0_f32, Color32::from_rgb(0x9a, 0x1c, 0x1c)));
        } else {
            let arm_up = sing;
            for side in [-1.0f32, 1.0] {
                let sh = pos2(hc.x - sway + side * head_r * 0.8, shoulders + 4.0);
                let elbow = sh + vec2(side * head_r * 0.7, head_r * (0.8 - 1.2 * arm_up));
                let hand = elbow + vec2(side * head_r * 0.4, -head_r * (0.2 + 0.9 * arm_up));
                painter.line_segment([sh, elbow], Stroke::new(4.0_f32, cloth));
                painter.line_segment([elbow, hand], Stroke::new(3.0_f32, skin));
                painter.circle_filled(hand, 3.0, skin);
            }
        }
        painter.circle_filled(hc, head_r, skin);
        if !monk {
            for i in 0..5 {
                let a = -PI * (0.3 + 0.1 * i as f32);
                painter.line_segment([hc + vec2(a.cos(), a.sin()) * head_r * 0.85, hc + vec2(a.cos(), a.sin()) * (head_r + 5.0)], Stroke::new(2.0_f32, brown));
            }
        }
        // Eyes: closed for monks, blinking now and then for the others.
        let blink = ((t * 0.37 + s as f32 * 0.21).fract()) > 0.97;
        let brow = 2.0 + 4.0 * sing;
        for side in [-1.0f32, 1.0] {
            let e = hc + vec2(side * head_r * 0.36, -head_r * (0.22 + 0.08 * lift));
            if monk || blink {
                painter.line_segment([e - vec2(3.0, -1.0), e + vec2(3.0, 1.0)], Stroke::new(1.5_f32, brown));
            } else {
                painter.circle_filled(e, 2.6, SCREEN_BG);
                painter.circle_filled(e + vec2(0.7, -0.7 - lift), 0.9, WHITE);
                painter.line_segment([e + vec2(-4.0, -4.0 - brow), e + vec2(4.0, -4.0 - brow - side * sing * 1.5)], Stroke::new(1.5_f32, brown));
            }
            painter.circle_filled(hc + vec2(side * head_r * 0.58, head_r * 0.18), 3.5, RED.gamma_multiply(0.25 * sing));
        }
        // The mouth makes the vowel.
        let mc = hc + vec2(0.0, head_r * 0.42);
        let mw = head_r * (0.18 + 0.32 * wide);
        let mh = 1.5 + head_r * 0.42 * open * (0.15 + 0.85 * sing);
        let mouth: Vec<Pos2> = (0..20).map(|i| {
            let a = i as f32 / 20.0 * TAU;
            mc + vec2(a.cos() * mw, a.sin() * mh)
        }).collect();
        painter.add(Shape::convex_polygon(mouth, Color32::from_rgb(0x6a, 0x10, 0x1a), Stroke::new(1.2_f32, Color32::from_rgb(0xc0, 0x40, 0x50))));
        if singing && breath > 0.05 {
            for i in 0..6 {
                let ph = (t * 1.5 + i as f32 * 0.29 + s as f32 * 0.1).fract();
                let q = mc + vec2(mw + 3.0 + ph * 24.0, ((i * 5) % 7) as f32 - 3.0 - ph * 5.0);
                painter.circle_filled(q, 0.8 + ph * 1.2, WHITE.gamma_multiply((1.0 - ph) * breath * 0.8));
            }
        }
        // The overtone: a thin whistling line rising from the mouth.
        if singing && over > 0.05 {
            let harm = live.vals[2].max(1.0);
            let pts: Vec<Pos2> = (0..=30).map(|i| {
                let k = i as f32 / 30.0;
                pos2(mc.x + k * 18.0 + (k * harm * 2.0 + t * 8.0).sin() * 3.0, mc.y - k * (mc.y - rect.top() - 12.0))
            }).collect();
            painter.add(Shape::line(pts, Stroke::new(1.2_f32, mix(BLUE, WHITE, 0.5).gamma_multiply(over * sing))));
        }
    }
    if singing && !monk {
        // Notes floating up from the first singer.
        for i in 0..3 {
            let ph = (t * 0.45 + i as f32 / 3.0).fract();
            let q = pos2(rect.left() + 20.0 + span / n as f32 * 0.5 + head_r + 14.0 + ph * 30.0 + (ph * 9.0 + i as f32).sin() * 5.0, rect.bottom() - head_r * 3.0 - ph * (rect.height() - head_r * 3.5));
            let c = YELLOW.gamma_multiply((1.0 - ph) * sing);
            painter.circle_filled(q, 3.0, c);
            painter.line_segment([q + vec2(3.0, 0.0), q + vec2(3.0, -10.0)], Stroke::new(1.3_f32, c));
            painter.line_segment([q + vec2(3.0, -10.0), q + vec2(7.0, -6.0)], Stroke::new(1.3_f32, c));
        }
    }
    // What they sing: the vowel, or the two it moves between.
    let moving = p[MORPH_LFO] > 0.01 || p[MORPH] > 0.01;
    let text = if moving { format!("{}~{}", MODELS[va], MODELS[vb]) } else { MODELS[va].to_string() };
    let bubble = Rect::from_min_size(rect.left_top() + vec2(8.0, 6.0), vec2(if moving { 76.0 } else { 46.0 }, 20.0));
    rect_fill(&painter, bubble, 8.0, WHITE);
    caption(&painter, bubble.center(), Align2::CENTER_CENTER, &text, SCREEN_BG);
    if moving {
        // Where the morph is between the two.
        let x = bubble.left() + 6.0 + morph * (bubble.width() - 12.0);
        painter.line_segment([pos2(bubble.left() + 6.0, bubble.bottom() + 3.0), pos2(bubble.right() - 6.0, bubble.bottom() + 3.0)], Stroke::new(1.0_f32, DIM));
        painter.circle_filled(pos2(x, bubble.bottom() + 3.0), 2.5, WHITE);
    }
    let mut y = rect.top() + 12.0;
    for (c, name, v) in [(WHITE, "breath", breath), (BLUE, "vib", vib), (robe, "under", under), (mix(BLUE, WHITE, 0.5), "over", over)] {
        legend(&painter, pos2(rect.right() - 74.0, y), c, name, v);
        y += 14.0;
    }
    if p[GLIDE] > 0.0015 {
        caption(&painter, pos2(rect.right() - 8.0, rect.bottom() - 18.0), Align2::RIGHT_BOTTOM, &format!("glide {:.2}s", p[GLIDE]), TEXT_DIM);
    }
    if let Some(n) = note {
        caption(&painter, pos2(rect.right() - 8.0, rect.bottom() - 6.0), Align2::RIGHT_BOTTOM, &note_name(n), TEXT_DIM);
    }
    if singing {
        ui.ctx().request_repaint();
    }
    let _ = id;
}

/// Mallets: what the knobs set shows. The stick's head is as soft or hard as
/// Hardness (a big woolly head or a small hard one), it lands where Stick
/// Position says on every bar, vibrato spins the fans in the resonators, and
/// the struck bar lights up and shakes. The banded waveguides get a bar, a
/// glass harmonica or a bowl, bowed or struck.
fn mallets_screen(ui: &mut Ui, k: usize, name: &str, level: f32, recent: &[(u8, f32)], ctl: &[(&str, f32)], height: f32) {
    let w = 300.0;
    let rect = screen(ui, w, height);
    let painter = ui.painter_at(rect);
    let t = ui.input(|i| i.time) as f32;
    caption(&painter, rect.left_top() + vec2(8.0, 6.0), Align2::LEFT_TOP, name, YELLOW);
    let fresh = recent.first().map(|r| (r.0, r.1)).filter(|r| r.1 < 1.5);
    let hit_now = recent.iter().any(|r| r.1 == 0.0) && level > 0.02;
    if k >= 9 {
        banded_screen(&painter, rect, k, level, t, fresh, ctl);
        return;
    }
    let hardness = control(ctl, "Stick Hardness").unwrap_or(0.5);
    let position = control(ctl, "Stick Position").unwrap_or(0.5);
    let vib_gain = control(ctl, "Vibrato Gain").unwrap_or(0.0);
    let vib_freq = control(ctl, "Vibrato Frequency").unwrap_or(0.5);
    let stick_mix = control(ctl, "Direct Stick Mix").unwrap_or(0.1);
    // The instrument: bars from C3 to C5, longer for lower notes.
    let lo = 48u8;
    let whites: Vec<u8> = (lo..=lo + 24).filter(|n| !matches!(n % 12, 1 | 3 | 6 | 8 | 10)).collect();
    let area = Rect::from_min_max(pos2(rect.left() + 10.0, rect.top() + 24.0), pos2(rect.right() - 10.0, rect.bottom() - 22.0));
    let bw = area.width() / whites.len() as f32;
    let wood = !matches!(k, 1 | 4 | 6);
    let base = if wood { Color32::from_rgb(0xb0, 0x6a, 0x30) } else { Color32::from_rgb(0x9a, 0xa4, 0xb4) };
    let blocks = matches!(k, 3 | 5);
    let mut struck_at: Option<Pos2> = None;
    for (i, &n) in whites.iter().enumerate() {
        let full = area.height() * 0.62;
        let len = if blocks { full * 0.45 } else { full * (1.0 - i as f32 / whites.len() as f32 * 0.45) };
        let x = area.left() + i as f32 * bw;
        let top = area.top() + (full - len) * 0.5;
        let hit = recent.iter().filter(|r| r.0 == n || (r.0 == n + 1 && matches!(r.0 % 12, 1 | 3 | 6 | 8 | 10))).map(|r| (1.0 - r.1 / 1.5).max(0.0)).fold(0.0f32, f32::max);
        let glow = hit * (0.3 + 0.7 * level.max(0.2));
        let shake = (t * 40.0 + i as f32).sin() * glow * 1.5;
        let bar = Rect::from_min_size(pos2(x + 1.5 + shake, top), vec2(bw - 3.0, len));
        rect_fill(&painter, bar, if blocks { 3.0 } else { 2.0 }, mix(shade(base, 0.45), mix(base, WHITE, 0.4), glow));
        // Where the stick lands on every bar.
        let sy = bar.top() + position * bar.height();
        painter.line_segment([pos2(bar.left() + 2.0, sy), pos2(bar.right() - 2.0, sy)], Stroke::new(1.0_f32, YELLOW.gamma_multiply(0.35)));
        if hit > 0.5 {
            struck_at.get_or_insert(pos2(bar.center().x, sy));
        }
        // The resonator below, with its vibrato fan.
        let tube = Rect::from_min_max(pos2(bar.center().x - 3.0, area.top() + full + 4.0), pos2(bar.center().x + 3.0, area.bottom() - 2.0 - i as f32 * 0.8));
        rect_fill(&painter, tube, 2.0, FAINT);
        if vib_gain > 0.02 {
            let a = t * (2.0 + 14.0 * vib_freq) + i as f32;
            let c = tube.center_top() + vec2(0.0, 4.0);
            painter.line_segment([c - vec2(a.cos(), a.sin() * 0.4) * 4.0, c + vec2(a.cos(), a.sin() * 0.4) * 4.0], Stroke::new(1.5_f32, BLUE.gamma_multiply(0.3 + 0.7 * vib_gain)));
        }
    }
    // The stick: down on the struck bar for a moment, else waiting above.
    let head_r = 3.0 + (1.0 - hardness) * 6.0;
    let head_c = mix(Color32::from_rgb(0xd8, 0x7a, 0xb0), WHITE, hardness);
    let rest = pos2(area.right() - 20.0, area.top() + 8.0);
    let at = match (struck_at, hit_now || fresh.is_some_and(|f| f.1 < 0.15)) {
        (Some(p), true) => p,
        (Some(p), false) => p + vec2(10.0, -18.0),
        (None, _) => rest,
    };
    painter.line_segment([at, at + vec2(34.0, -26.0)], Stroke::new(2.0_f32, Color32::from_rgb(0xc8, 0xa0, 0x70)));
    painter.circle_filled(at, head_r, head_c);
    if hardness < 0.4 {
        // Wool.
        for j in 0..6 {
            let a = j as f32 / 6.0 * TAU;
            painter.circle_stroke(at + vec2(a.cos(), a.sin()) * head_r * 0.5, head_r * 0.35, Stroke::new(0.8_f32, shade(head_c, 0.7)));
        }
    }
    if let Some(p) = struck_at.filter(|_| hit_now || fresh.is_some_and(|f| f.1 < 0.25)) {
        // The click of the stick itself.
        painter.circle_stroke(p, 4.0 + 10.0 * stick_mix, Stroke::new(1.5_f32, WHITE.gamma_multiply(0.4 + 0.6 * stick_mix)));
    }
    let mut x = rect.left() + 8.0;
    let y = rect.bottom() - 9.0;
    x += legend(&painter, pos2(x, y), head_c, "hard", hardness);
    x += legend(&painter, pos2(x, y), YELLOW, "pos", position);
    x += legend(&painter, pos2(x, y), BLUE, "vib", vib_gain);
    legend(&painter, pos2(x, y), WHITE, "click", stick_mix);
}

/// Banded waveguides: a bar, a glass harmonica or a bowl, rubbed by a bow
/// (pressure and motion show) or struck when Set Striking is on.
fn banded_screen(painter: &Painter, rect: Rect, k: usize, level: f32, t: f32, fresh: Option<(u8, f32)>, ctl: &[(&str, f32)]) {
    let pressure = control(ctl, "Bow Pressure").unwrap_or(0.0);
    let motion = control(ctl, "Bow Motion").unwrap_or(0.0);
    let striking = control(ctl, "Set Striking").unwrap_or(0.0) >= 0.5;
    let gain = control(ctl, "Gain").unwrap_or(0.99);
    let vib = control(ctl, "Vibrato Frequency").unwrap_or(0.0);
    let c = pos2(rect.center().x, rect.center().y + 2.0);
    let rad = ((rect.height() - 50.0) * 0.42).clamp(20.0, 56.0);
    let color = if level > 0.01 { YELLOW } else { DIM };
    let wob = level * 3.0 * (1.0 + vib);
    let spin = t * (0.5 + 3.0 * motion);
    // Rings spreading as it sounds, more of them the longer it rings.
    let rings = 1 + (gain * 3.0) as usize;
    for j in 0..rings {
        let ph = (t * 0.6 + j as f32 / rings as f32).fract();
        painter.circle_stroke(c, rad * (1.05 + ph * 0.4), Stroke::new(1.0_f32, YELLOW.gamma_multiply(level * (1.0 - ph) * 0.5)));
    }
    let contact = match k {
        11 => {
            // Glass harmonica: discs on a rod, seen from the side.
            for j in 0..5 {
                let r = rad * (1.0 - j as f32 * 0.14);
                let x = c.x - rad + j as f32 * rad * 0.45;
                let squeeze = 0.35;
                let pts: Vec<Pos2> = (0..=40).map(|i| { let a = i as f32 / 40.0 * TAU; pos2(x + a.cos() * r * squeeze, c.y + a.sin() * r + (a * 3.0 + t * 7.0).sin() * wob) }).collect();
                painter.add(Shape::closed_line(pts, Stroke::new(1.5_f32, color.gamma_multiply(1.0 - j as f32 * 0.12))));
            }
            painter.line_segment([pos2(c.x - rad * 1.5, c.y), pos2(c.x + rad * 1.6, c.y)], Stroke::new(1.0_f32, DIM));
            pos2(c.x - rad, c.y - rad)
        }
        12 => {
            // A bowl from above, its rim wobbling.
            let pts: Vec<Pos2> = (0..=90).map(|i| { let a = i as f32 / 90.0 * TAU; c + vec2(a.cos(), a.sin()) * (rad + (a * 2.0 + t * 9.0).sin() * wob) }).collect();
            painter.add(Shape::closed_line(pts, Stroke::new(2.0_f32, color)));
            painter.circle_stroke(c, rad * 0.7, Stroke::new(1.0_f32, FAINT));
            c + vec2(spin.cos(), spin.sin()) * (rad + 5.0)
        }
        _ => {
            // A bar.
            let bar = Rect::from_center_size(c, vec2(rad * 3.6, 16.0));
            let pts: Vec<Pos2> = (0..=40).map(|i| { let x = i as f32 / 40.0; pos2(bar.left() + x * bar.width(), bar.center().y + (x * PI * 2.0).sin() * (t * 20.0).sin() * wob) }).collect();
            rect_fill(painter, bar, 2.0, shade(Color32::from_rgb(0x9a, 0xa4, 0xb4), 0.5 + 0.5 * level));
            painter.add(Shape::line(pts, Stroke::new(1.0_f32, color)));
            pos2(bar.left() + bar.width() * (0.5 + 0.35 * spin.sin()), bar.top() - 2.0)
        }
    };
    if striking {
        let down = fresh.is_some_and(|f| f.1 < 0.15) || level > 0.3;
        let head = contact + if down { vec2(0.0, 0.0) } else { vec2(8.0, -14.0) };
        painter.line_segment([head, head + vec2(30.0, -24.0)], Stroke::new(2.0_f32, Color32::from_rgb(0xc8, 0xa0, 0x70)));
        painter.circle_filled(head, 5.0, WHITE);
    } else {
        // The bow (or stick) rubbing, pressed as hard as Bow Pressure.
        painter.circle_filled(contact, 3.0 + pressure * 4.0, WHITE.gamma_multiply(0.4 + 0.6 * level.max(0.2)));
        painter.circle_stroke(contact, 6.0 + pressure * 8.0, Stroke::new(1.0_f32, accent_now().gamma_multiply(pressure * level.max(0.3))));
    }
    let mut x = rect.left() + 8.0;
    let y = rect.bottom() - 9.0;
    if striking {
        caption(painter, pos2(x, y), Align2::LEFT_CENTER, "struck", WHITE);
        x += 54.0;
    } else {
        x += legend(painter, pos2(x, y), accent_now(), "press", pressure);
        x += legend(painter, pos2(x, y), WHITE, "motion", motion);
    }
    x += legend(painter, pos2(x, y), YELLOW, "ring", gain);
    legend(painter, pos2(x, y), BLUE, "vib", vib);
}

/// A string per recent note, vibrating with the sound: bowed strings get a
/// bow, the others a plucking point.
fn strings_screen(ui: &mut Ui, k: usize, name: &str, level: f32, recent: &[(u8, f32)], height: f32) {
    let w = 260.0;
    let rect = screen(ui, w, height);
    let painter = ui.painter_at(rect);
    let t = ui.input(|i| i.time) as f32;
    let bowed = k == 0;
    let color = [GREEN, accent_now(), YELLOW, RED, BLUE][k % 5];
    caption(&painter, pos2(rect.left() + 8.0, rect.top() + 6.0), Align2::LEFT_TOP, name, color);
    let n = recent.len().clamp(1, 4);
    let area = Rect::from_min_max(pos2(rect.left() + 36.0, rect.top() + 24.0), pos2(rect.right() - 12.0, rect.bottom() - 12.0));
    for s in 0..n {
        let y0 = area.top() + (s as f32 + 0.5) / n as f32 * area.height();
        let fresh = recent.get(s).map(|r| (1.0 - r.1 / 2.0).max(0.0)).unwrap_or(0.0);
        let amp = level * fresh * (area.height() / n as f32 * 0.38);
        // The sitar buzzes with many overtones.
        let modes = if k == 3 { 7 } else { 3 };
        let pts: Vec<Pos2> = (0..=80)
            .map(|i| {
                let x = i as f32 / 80.0;
                let mut y = 0.0;
                for m in 1..=modes {
                    let mf = m as f32;
                    y += (mf * PI * x).sin() * (t * 7.0 * mf + s as f32).cos() / mf;
                }
                pos2(area.left() + x * area.width(), y0 + y * amp)
            })
            .collect();
        painter.add(Shape::line(pts, Stroke::new(1.5_f32, if amp > 0.3 { color } else { DIM })));
        painter.circle_filled(pos2(area.left(), y0), 2.5, DIM);
        painter.circle_filled(pos2(area.right(), y0), 2.5, DIM);
        if let Some(r) = recent.get(s) {
            caption(&painter, pos2(rect.left() + 6.0, y0), Align2::LEFT_CENTER, &note_name(r.0), if r.1 == 0.0 { color } else { DIM });
        }
    }
    if bowed {
        // The bow, moving back and forth across the strings.
        let x = area.left() + area.width() * (0.72 + 0.12 * (t * 1.3).sin() * level.max(0.1));
        painter.line_segment([pos2(x - 10.0, area.top() - 6.0), pos2(x + 10.0, area.bottom() + 4.0)], Stroke::new(3.0_f32, Color32::from_rgb(0x9a, 0x5b, 0x2c)));
        painter.line_segment([pos2(x - 7.0, area.top() - 6.0), pos2(x + 13.0, area.bottom() + 4.0)], Stroke::new(1.0_f32, WHITE.gamma_multiply(0.7)));
    }
}

/// A tube with its tone holes; the breath runs through it as fast as the
/// sound is loud, and a standing wave shows inside.
fn winds_screen(ui: &mut Ui, k: usize, name: &str, level: f32, recent: &[(u8, f32)], height: f32) {
    let w = 280.0;
    let rect = screen(ui, w, height);
    let painter = ui.painter_at(rect);
    let t = ui.input(|i| i.time) as f32;
    let color = [BLUE, GREEN, YELLOW, accent_now(), BLUE, GREEN, WHITE][k % 7];
    caption(&painter, pos2(rect.left() + 8.0, rect.top() + 6.0), Align2::LEFT_TOP, name, color);
    let note = recent.first().map(|r| r.0).unwrap_or(60);
    let cy = rect.center().y + 6.0;
    if k == 4 {
        // A blown bottle: air across its neck, the air inside swinging.
        let c = pos2(rect.center().x + 20.0, cy + 10.0);
        let body = Rect::from_center_size(c, vec2(70.0, (height - 70.0).clamp(30.0, 80.0)));
        rect_line(&painter, body, 14.0, 2.0, color);
        painter.line_segment([pos2(c.x - 9.0, body.top()), pos2(c.x - 9.0, body.top() - 18.0)], Stroke::new(2.0_f32, color));
        painter.line_segment([pos2(c.x + 9.0, body.top()), pos2(c.x + 9.0, body.top() - 18.0)], Stroke::new(2.0_f32, color));
        for i in 0..8 {
            let ph = (t * (0.5 + 2.5 * level) + i as f32 * 0.13).fract();
            let x = c.x - 60.0 + ph * 50.0;
            let y = body.top() - 22.0 + (i % 3) as f32 * 2.0;
            painter.line_segment([pos2(x, y), pos2(x + 6.0, y + ph * 4.0)], Stroke::new(1.2_f32, WHITE.gamma_multiply((1.0 - ph) * level)));
        }
        let fill = body.shrink(4.0);
        let wave: Vec<Pos2> = (0..=30).map(|i| {
            let x = i as f32 / 30.0;
            pos2(fill.left() + x * fill.width(), fill.center().y + (x * PI * 2.0 + t * 6.0).sin() * 4.0 * level)
        }).collect();
        painter.add(Shape::line(wave, Stroke::new(1.5_f32, color.gamma_multiply(0.7))));
        return;
    }
    if k == 6 {
        // A whistle: the pea spins with the breath.
        let c = pos2(rect.center().x, cy);
        let r = ((height - 50.0) * 0.32).clamp(16.0, 40.0);
        painter.circle_stroke(c, r, Stroke::new(2.0_f32, color));
        painter.line_segment([pos2(c.x - r * 2.2, c.y - r + 2.0), pos2(c.x, c.y - r + 2.0)], Stroke::new(2.0_f32, color));
        painter.line_segment([pos2(c.x - r * 2.2, c.y - r + 14.0), pos2(c.x - r * 0.9, c.y - r + 14.0)], Stroke::new(2.0_f32, color));
        let a = t * (2.0 + 14.0 * level);
        painter.circle_filled(c + vec2(a.cos(), a.sin()) * r * 0.6, 4.0, WHITE);
        return;
    }
    // The tube, with a bell as wide as the instrument has.
    let (x0, x1) = (rect.left() + 24.0, rect.right() - 46.0);
    let half = 9.0;
    let bell = [4.0, 8.0, 20.0, 26.0, 0.0, 8.0, 0.0][k % 7];
    let top: Vec<Pos2> = (0..=40).map(|i| {
        let x = i as f32 / 40.0;
        pos2(x0 + x * (x1 - x0 + 30.0), cy - half - bell * x.powi(6))
    }).collect();
    let bot: Vec<Pos2> = top.iter().map(|q| pos2(q.x, 2.0 * cy - q.y)).collect();
    painter.add(Shape::line(top, Stroke::new(2.0_f32, color)));
    painter.add(Shape::line(bot, Stroke::new(2.0_f32, color)));
    painter.line_segment([pos2(x0, cy - half), pos2(x0, cy + half)], Stroke::new(2.0_f32, color));
    // Tone holes: the higher the note in its octave, the more are open.
    let open = ((note % 12) as usize * 7 / 12).min(6);
    for h in 0..6 {
        let x = x0 + 40.0 + h as f32 * ((x1 - x0 - 70.0) / 5.0);
        let c = pos2(x, cy - half - 5.0);
        if h >= 6 - open {
            painter.circle_filled(c, 3.5, color);
        } else {
            painter.circle_stroke(c, 3.5, Stroke::new(1.2_f32, DIM));
        }
    }
    // The standing wave inside, more half-waves for higher registers.
    let halves = 1 + (note.saturating_sub(48) / 12) as usize;
    let wave: Vec<Pos2> = (0..=80)
        .map(|i| {
            let x = i as f32 / 80.0;
            pos2(x0 + x * (x1 - x0), cy + (x * PI * halves as f32).sin() * (t * 9.0).cos() * (half - 2.0) * level)
        })
        .collect();
    painter.add(Shape::line(wave, Stroke::new(1.2_f32, color.gamma_multiply(0.6))));
    // Breath streaming in.
    for i in 0..10 {
        let ph = (t * (0.3 + 2.0 * level) + i as f32 * 0.1).fract();
        let x = x0 + ph * (x1 - x0);
        let y = cy + ((i * 3) % 5) as f32 * 2.5 - 5.0;
        painter.line_segment([pos2(x, y), pos2(x + 5.0, y)], Stroke::new(1.0_f32, WHITE.gamma_multiply((1.0 - ph) * level)));
    }
    caption(&painter, pos2(rect.left() + 8.0, rect.bottom() - 6.0), Align2::LEFT_BOTTOM, &note_name(note), TEXT_DIM);
}

// ---------------------------------------------------------------- monk

/// Monk: a chanting monk whose mouth makes the vowel, his head as big as
/// Head says (a low voice is a big head), rising and sinking with the pitch
/// as it glides; the choir stands behind him and the echo rings out. Drag
/// anywhere on the screen: up and down is the vowel, left and right the head.
pub(crate) fn monk_screen(ui: &mut Ui, id: Id, p: &mut [f32], live: &InstLive, height: f32) -> bool {
    use krabton_engine::dsp::monk::*;
    let w = 280.0;
    let (rect, resp) = ui.allocate_exact_size(vec2(w, height), Sense::click_and_drag());
    let mut changed = false;
    if let Some(pos) = resp.interact_pointer_pos().filter(|_| resp.dragged() || resp.clicked()) {
        let head = ((pos.x - rect.left() - 10.0) / (rect.width() - 20.0)).clamp(0.0, 1.0);
        let vowel = ((rect.bottom() - 10.0 - pos.y) / (rect.height() - 20.0)).clamp(0.0, 1.0);
        let (head, vowel) = ((head * 100.0).round() / 100.0, (vowel * 100.0).round() / 100.0);
        if p[HEAD] != head || p[VOWEL] != vowel {
            p[HEAD] = head;
            p[VOWEL] = vowel;
            changed = true;
        }
    }
    if resp.hovered() || resp.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
    }
    let painter = ui.painter_at(rect);
    rect_fill(&painter, rect, 6.0, SCREEN_BG);
    rect_line(&painter, rect, 6.0, 1.0, Color32::from_rgb(0x26, 0x29, 0x31));
    let t = ui.input(|i| i.time) as f32;
    let level = (live.level() * 5.0).min(1.0);
    let singing = live.vals[0] > 0.0 && level > 0.01;
    let vowel = if live.vals[0] > 0.0 { live.vals[1] } else { p[VOWEL] };
    let head = p[HEAD].clamp(0.0, 1.0);
    // The pad: a faint grid, and where the knobs are.
    for k in 1..4 {
        let y = rect.top() + rect.height() * k as f32 / 4.0;
        painter.line_segment([pos2(rect.left() + 4.0, y), pos2(rect.right() - 4.0, y)], Stroke::new(1.0_f32, FAINT));
    }
    let px = rect.left() + 10.0 + head * (rect.width() - 20.0);
    let py = rect.bottom() - 10.0 - p[VOWEL].clamp(0.0, 1.0) * (rect.height() - 20.0);
    painter.line_segment([pos2(px, rect.top() + 4.0), pos2(px, rect.bottom() - 4.0)], Stroke::new(1.0_f32, accent_now().gamma_multiply(0.25)));
    painter.line_segment([pos2(rect.left() + 4.0, py), pos2(rect.right() - 4.0, py)], Stroke::new(1.0_f32, accent_now().gamma_multiply(0.25)));
    painter.circle_stroke(pos2(px, py), 5.0, Stroke::new(1.5_f32, accent_now()));
    // The echo rings out to the right.
    let echo = p[ECHO].clamp(0.0, 1.0);
    let spacing = 10.0 + 22.0 * p[ECHO_TIME].clamp(0.0, 1.0);
    let robe = Color32::from_rgb(0xd8, 0x6a, 0x1c);
    let skin = Color32::from_rgb(0xe8, 0xc0, 0x98);
    // Pitch: up and down as it glides.
    let pitch = if live.vals[0] > 0.0 { live.vals[2] } else { 48.0 };
    let lift = ((pitch - 48.0) / 24.0).clamp(-0.6, 0.8);
    let head_r = (height * 0.17) * (1.25 - 0.5 * head);
    let base_y = rect.bottom() - head_r * 1.6;
    let cx = rect.left() + 108.0;
    let hc = pos2(cx, base_y - head_r * 1.2 - lift * 12.0);
    if singing && echo > 0.01 {
        for k in 0..4 {
            let ph = (t * 0.6 + k as f32 / 4.0).fract();
            let r = head_r + 8.0 + ph * spacing * 4.0;
            let pts: Vec<Pos2> = (0..=20).map(|i| {
                let a = -0.6 + 1.2 * i as f32 / 20.0;
                hc + vec2(a.cos(), a.sin()) * r
            }).collect();
            painter.add(Shape::line(pts, Stroke::new(1.5_f32, YELLOW.gamma_multiply(echo * (1.0 - ph) * level))));
        }
    }
    // The choir behind him.
    let singers = (p[UNISON].round() as usize).clamp(1, 10);
    let behind = (singers - 1).min(4);
    for k in 0..behind {
        let side = if k % 2 == 0 { -1.0 } else { 1.0 };
        let off = (k / 2 + 1) as f32;
        let r = head_r * (0.7 - 0.08 * off);
        let c = pos2(cx + side * off * head_r * 1.15, hc.y - r * 0.5 - off * 4.0);
        let shade_k = 0.55 - 0.1 * off;
        painter.add(Shape::convex_polygon(vec![pos2(c.x - r * 1.2, rect.bottom()), pos2(c.x - r * 0.9, c.y + r), pos2(c.x + r * 0.9, c.y + r), pos2(c.x + r * 1.2, rect.bottom())], shade(robe, shade_k), Stroke::NONE));
        painter.circle_filled(c, r, shade(skin, shade_k));
        let mo = r * 0.25 * (0.2 + 0.8 * level) * (1.0 - (vowel - 0.5).abs());
        painter.add(Shape::ellipse_filled(c + vec2(0.0, r * 0.4), vec2(r * 0.25, mo.max(1.0)), shade(Color32::from_rgb(0x6a, 0x10, 0x1a), shade_k)));
    }
    if singers > 5 {
        caption(&painter, pos2(cx + head_r * 2.6, rect.bottom() - 10.0), Align2::LEFT_BOTTOM, &format!("+{}", singers - 5), TEXT_DIM);
    }
    // The monk: robe with a sash, bald head, closed eyes.
    let shoulders = hc.y + head_r * 0.95;
    painter.add(Shape::convex_polygon(vec![pos2(cx - head_r * 1.5, rect.bottom()), pos2(cx - head_r * 1.0, shoulders), pos2(cx + head_r * 1.0, shoulders), pos2(cx + head_r * 1.5, rect.bottom())], robe, Stroke::NONE));
    painter.line_segment([pos2(cx - head_r * 0.9, shoulders + 2.0), pos2(cx + head_r * 1.2, rect.bottom())], Stroke::new(4.0_f32, Color32::from_rgb(0x9a, 0x1c, 0x1c)));
    painter.circle_filled(hc, head_r, skin);
    painter.circle_filled(hc + vec2(-head_r * 0.35, -head_r * 0.45), head_r * 0.18, Color32::from_white_alpha(40));
    let brown = Color32::from_rgb(0x5a, 0x3a, 0x22);
    for side in [-1.0f32, 1.0] {
        let e = hc + vec2(side * head_r * 0.36, -head_r * 0.12);
        let arc: Vec<Pos2> = (0..=8).map(|i| {
            let a = PI * i as f32 / 8.0;
            e + vec2(-a.cos() * head_r * 0.14, a.sin() * head_r * 0.06)
        }).collect();
        painter.add(Shape::line(arc, Stroke::new(1.5_f32, brown)));
        painter.circle_filled(hc + vec2(side * head_r * 0.98, 0.0), head_r * 0.14, skin);
    }
    // The mouth: open as far as the first formant is high, as wide as the second.
    let f = formants(vowel, 0.5);
    let open = ((f[0] - 260.0) / 560.0).clamp(0.0, 1.0);
    let wide = ((f[1] - 600.0) / 1550.0).clamp(0.0, 1.0);
    let mc = hc + vec2(0.0, head_r * 0.45);
    let mw = head_r * (0.16 + 0.28 * wide);
    let mh = 1.5 + head_r * (0.08 + 0.32 * open) * (0.25 + 0.75 * level);
    painter.add(Shape::ellipse_filled(mc, vec2(mw, mh), Color32::from_rgb(0x6a, 0x10, 0x1a)));
    painter.add(Shape::ellipse_stroke(mc, vec2(mw, mh), Stroke::new(1.2_f32, Color32::from_rgb(0xb0, 0x50, 0x40))));
    if singing && p[BREATH] > 0.05 {
        for i in 0..6 {
            let ph = (t * 1.4 + i as f32 * 0.31).fract();
            let q = mc + vec2(mw + 3.0 + ph * 26.0, ((i * 5) % 7) as f32 - 3.0 - ph * 4.0);
            painter.circle_filled(q, 0.8 + ph * 1.2, WHITE.gamma_multiply((1.0 - ph) * p[BREATH] * 0.7));
        }
    }
    let vowel_names = ["u", "o", "a", "e", "i"];
    let vn = vowel_names[((vowel * 4.0).round() as usize).min(4)];
    caption(&painter, rect.left_top() + vec2(8.0, 6.0), Align2::LEFT_TOP, &format!("vowel {vn}"), accent_now());
    caption(&painter, pos2(rect.right() - 8.0, rect.top() + 6.0), Align2::RIGHT_TOP, &format!("head {:.0}", head * 100.0), TEXT_DIM);
    if p[GLIDE] > 0.001 {
        caption(&painter, pos2(rect.right() - 8.0, rect.top() + 20.0), Align2::RIGHT_TOP, &format!("glide {:.0}", p[GLIDE] * 100.0), TEXT_DIM);
    }
    if singing {
        ui.ctx().request_repaint();
    }
    resp.on_hover_text("Drag: up and down the vowel, left and right the head");
    let _ = id;
    changed
}

// ---------------------------------------------------------------- grand piano

/// How far the piano's samples are.
pub(crate) enum PianoLib {
    Missing,
    Installing(String, f32),
    Failed(String),
    Ready,
}

/// The grand piano from above: the case with its lid, the strings (they
/// shine while they sound), the dampers (lifted by the pedal or a held key),
/// the keys and the pedals. Click the left pedal for una corda, the right
/// one for the sustain. Without the samples it offers to download them.
/// Returns (changed, install clicked).
pub(crate) fn piano_screen(ui: &mut Ui, id: Id, p: &mut [f32], live: &InstLive, height: f32, lib: PianoLib) -> (bool, bool) {
    use krabton_engine::piano::*;
    let w = 300.0;
    let rect = screen(ui, w, height);
    let mut changed = false;
    let mut install = false;
    let ready = matches!(lib, PianoLib::Ready);
    let t = ui.input(|i| i.time) as f32;
    let level = (live.level() * 4.0).min(1.0);
    let pedal = live.vals[1] > 0.5 || p[SUSTAIN] >= 0.5;
    // Sounding keys, from the engine (A0 = bit 0).
    let mut sounding = [false; 88];
    for (k, s) in sounding.iter_mut().enumerate() {
        *s = (live.vals[3 + k / 22] as u32) >> (k % 22) & 1 == 1;
    }
    let kb = Rect::from_min_max(pos2(rect.left() + 10.0, rect.bottom() - 34.0), pos2(rect.right() - 10.0, rect.bottom() - 18.0));
    let top = rect.top() + 20.0;
    // The case: straight on the bass side, the bentside curving in to the tail.
    let x0 = kb.left();
    let x1 = kb.right();
    let case_y = |x: f32| {
        let u = ((x - x0) / (x1 - x0)).clamp(0.0, 1.0);
        top + (kb.top() - 4.0 - top) * (0.15 + 0.85 * u.powf(1.6)) * u.powf(0.35)
    };
    let painter = ui.painter_at(rect);
    let dim = if ready { 1.0 } else { 0.35 };
    let mut outline: Vec<Pos2> = vec![pos2(x0, kb.top() - 2.0), pos2(x0, top + 6.0)];
    for i in 0..=40 {
        let x = x0 + (x1 - x0) * i as f32 / 40.0;
        outline.push(pos2(x, case_y(x).min(kb.top() - 2.0)));
    }
    outline.push(pos2(x1, kb.top() - 2.0));
    painter.add(Shape::line(outline.clone(), Stroke::new(2.0_f32, WHITE.gamma_multiply(0.8 * dim))));
    // The lid, raised on the straight side as far as Lid opens it.
    let lid = p[LID].clamp(0.0, 1.0);
    if lid > 0.02 {
        let lift = 14.0 * lid;
        let pts: Vec<Pos2> = outline.iter().map(|q| pos2(q.x - lift * (1.0 - (q.x - x0) / (x1 - x0)), q.y - lift * 0.6)).collect();
        painter.add(Shape::line(pts, Stroke::new(1.0_f32, DIM.gamma_multiply(dim))));
        painter.line_segment([pos2(x0 + 30.0, top + 30.0), pos2(x0 + 30.0 - lift, top + 30.0 - lift * 0.6)], Stroke::new(1.0_f32, DIM.gamma_multiply(dim)));
    }
    // Strings: one line for every third key, the ones sounding shine.
    let damper_y = kb.top() - 12.0;
    for k in (0..88).step_by(2) {
        let x = x0 + 4.0 + (x1 - x0 - 8.0) * (k as f32 + 0.5) / 88.0;
        let y_end = case_y(x) + 6.0;
        let on = sounding[k] || sounding.get(k + 1).copied().unwrap_or(false);
        let (c, wv) = if on { (YELLOW, 1.2 * level) } else { (DIM, 0.0) };
        if wv > 0.05 {
            let pts: Vec<Pos2> = (0..=12).map(|i| {
                let f = i as f32 / 12.0;
                pos2(x + (f * PI).sin() * (t * 40.0 + k as f32).sin() * wv, damper_y + (y_end - damper_y) * f)
            }).collect();
            painter.add(Shape::line(pts, Stroke::new(1.0_f32, c.gamma_multiply(dim))));
        } else {
            painter.line_segment([pos2(x, damper_y), pos2(x, y_end)], Stroke::new(0.7_f32, c.gamma_multiply(0.6 * dim)));
        }
    }
    // Dampers: down on the strings, lifted by the pedal or the key.
    let held: Vec<bool> = (0..88).map(|k| live.notes >> (21 + k) & 1 == 1).collect();
    for k in (0..66).step_by(3) {
        let x = x0 + 4.0 + (x1 - x0 - 8.0) * (k as f32 + 1.0) / 88.0;
        let up = pedal || held[k] || held[k + 1] || held[k + 2];
        let y = damper_y - if up { 5.0 } else { 0.0 };
        rect_fill(&painter, Rect::from_center_size(pos2(x, y), vec2(4.0, 3.0)), 1.0, if up { accent_now().gamma_multiply(dim) } else { DIM.gamma_multiply(dim) });
    }
    // The keyboard.
    keys(&painter, kb, live.notes, 21, 108, GREEN);
    for k in 0..88 {
        if sounding[k] && !held[k] && !matches!((k + 21) % 12, 1 | 3 | 6 | 8 | 10) {
            let white = (21..=(21 + k as u8)).filter(|n| !matches!(n % 12, 1 | 3 | 6 | 8 | 10)).count() - 1;
            let kw = kb.width() / 52.0;
            rect_fill(&painter, Rect::from_min_size(pos2(kb.left() + white as f32 * kw, kb.bottom() - 4.0), vec2(kw - 1.0, 3.0)), 0.0, YELLOW.gamma_multiply(0.6));
        }
    }
    // The pedals: una corda (left), sostenuto (middle, not used), sustain (right).
    let pc = pos2(rect.center().x, rect.bottom() - 9.0);
    let pedals = [(pc - vec2(20.0, 0.0), p[SOFT] > 0.01, "una corda"), (pc, false, ""), (pc + vec2(20.0, 0.0), pedal, "sustain")];
    for (k, (at, on, tip)) in pedals.iter().enumerate() {
        let r = Rect::from_center_size(*at, vec2(12.0, 6.0));
        if k != 1 && ready {
            let resp = ui.interact(r.expand(3.0), id.with(("pedal", k)), Sense::click());
            if resp.clicked() {
                if k == 0 {
                    p[SOFT] = if p[SOFT] > 0.01 { 0.0 } else { 0.6 };
                } else {
                    p[SUSTAIN] = if p[SUSTAIN] >= 0.5 { 0.0 } else { 1.0 };
                }
                changed = true;
            }
            resp.on_hover_text(format!("Click: {tip} on or off"));
        }
        let painter = ui.painter_at(rect);
        rect_fill(&painter, r, 2.0, if *on { accent_now() } else { DIM.gamma_multiply(dim) });
    }
    let painter = ui.painter_at(rect);
    caption(&painter, rect.left_top() + vec2(8.0, 6.0), Align2::LEFT_TOP, "grand piano · salamander", WHITE.gamma_multiply(dim));
    match lib {
        PianoLib::Ready => {
            if pedal {
                caption(&painter, pos2(rect.right() - 8.0, rect.top() + 6.0), Align2::RIGHT_TOP, "pedal", accent_now());
            }
            if level > 0.005 || live.vals[0] > 0.0 {
                ui.ctx().request_repaint();
            }
        }
        PianoLib::Missing | PianoLib::Failed(_) => {
            let b = Rect::from_center_size(pos2(rect.center().x, rect.center().y - 8.0), vec2(220.0, 26.0));
            let resp = ui.interact(b, id.with("install"), Sense::click());
            let painter = ui.painter_at(rect);
            rect_fill(&painter, b, 4.0, if resp.hovered() { mix(accent_now(), WHITE, 0.2) } else { accent_now() });
            painter.text(b.center(), Align2::CENTER_CENTER, "Download the concert grand", egui::FontId::proportional(13.0), TEXT_DARK);
            let note = match &lib {
                PianoLib::Failed(e) => e.clone(),
                _ => "750 MB, once; 1.3 GB on disk".to_string(),
            };
            painter.text(pos2(b.center().x, b.bottom() + 10.0), Align2::CENTER_CENTER, note, small(), if matches!(lib, PianoLib::Failed(_)) { RED } else { TEXT_DIM });
            painter.text(pos2(b.center().x, b.bottom() + 24.0), Align2::CENTER_CENTER, "Salamander Grand Piano by Alexander Holm, CC BY 3.0", small(), TEXT_DIM);
            install = resp.clicked();
            resp.on_hover_text("Downloads the Salamander Grand Piano (a grand in 16 velocity layers) and prepares it for Krabton");
        }
        PianoLib::Installing(what, frac) => {
            let bar = Rect::from_center_size(pos2(rect.center().x, rect.center().y - 6.0), vec2(220.0, 8.0));
            rect_fill(&painter, bar, 3.0, FAINT);
            rect_fill(&painter, Rect::from_min_size(bar.min, vec2(bar.width() * frac.clamp(0.0, 1.0), bar.height())), 3.0, accent_now());
            painter.text(pos2(bar.center().x, bar.bottom() + 12.0), Align2::CENTER_CENTER, what, small(), TEXT);
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(200));
        }
    }
    (changed, install)
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::RawInput;

    #[test]
    fn every_screen_draws_quiet_and_sounding() {
        let ctx = egui::Context::default();
        let wave: Vec<f32> = (0..4096).map(|i| (i as f32 * 0.05).sin() * 0.5).collect();
        for loud in [false, true] {
            let live = InstLive {
                wave: if loud { wave.clone() } else { vec![0.0; 4096] },
                notes: if loud { 1u128 << 60 | 1u128 << 64 | 1u128 << 127 } else { 0 },
                vals: [if loud { 0.6 } else { 0.0 }; INST_VALS],
            };
            for _ in 0..3 {
                let _ = ctx.run(RawInput::default(), |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        let id = Id::new("t");
                        particles(ui, id, &mut shaker_synth::preset("Maraca Roll"), &live, 150.0);
                        organ_screen(ui, id, &mut organ::preset("Full Organ"), &live, 150.0);
                        analog_grid(ui, id, &mut analog::preset("Pluck"), &live, 150.0, None);
                        synth_grid(ui, id, &mut synth::preset(synth::preset_names()[0]), &live, 150.0, None);
                        rings_screen(ui, &rings_synth::preset(rings_synth::preset_names()[0]), &live, 150.0);
                        elements_screen(ui, id, &elements_synth::preset(elements_synth::preset_names()[0]), &live, 150.0);
                        for lib in [PianoLib::Missing, PianoLib::Installing("Downloading".into(), 0.4), PianoLib::Ready] {
                            piano_screen(ui, id, &mut krabton_engine::piano::preset("Concert Grand"), &live, 150.0, lib);
                        }
                        for preset in krabton_engine::dsp::monk::preset_names() {
                            monk_screen(ui, id, &mut krabton_engine::dsp::monk::preset(preset), &live, 150.0);
                        }
                        for preset in krabton_engine::dsp::stk_voice::preset_names() {
                            voice_screen(ui, id, &krabton_engine::dsp::stk_voice::preset(preset), &live, 150.0);
                        }
                        for (f, fam) in [Family::Mallets, Family::Strings, Family::Winds].into_iter().enumerate() {
                            for k in 0..stk::models(fam).len() {
                                stk_screen(ui, id.with((f, k)), fam, &stk::defaults_for(fam, k), &live, 150.0);
                            }
                        }
                    });
                });
            }
        }
    }
}

// ---------------------------------------------------------------- facet

/// The colours of Facet's four knobs.
pub(crate) const FACET_COLORS: [Color32; 4] = [BLUE, GREEN, WHITE, Color32::from_rgb(0xff, 0x8c, 0x2e)];

/// Facet: the engine's cycle as the four knobs shape it (with the sound
/// itself behind it while notes play), and the knobs in their colours. Drag
/// a knob up or down to turn it; click the engine's name for the next one
/// (right-click: the one before).
pub(crate) fn facet_screen(ui: &mut Ui, id: Id, p: &mut [f32], live: &InstLive, height: f32) -> bool {
    use krabton_engine::dsp::facet;
    if p.len() < facet::SPECS.len() {
        let _ = screen(ui, 300.0, height);
        return false;
    }
    let w = 300.0;
    let rect = screen(ui, w, height);
    let mut changed = false;
    let engine = facet::engine_of(p);
    let names = facet::knob_names(engine);
    let k = [p[facet::BLUE], p[facet::GREEN], p[facet::WHITE], p[facet::ORANGE]];
    // The engine's name: click for the next.
    let title = Rect::from_min_size(rect.min + vec2(8.0, 6.0), vec2(140.0, 16.0));
    let tresp = ui.interact(title, id.with("engine"), Sense::click());
    let n = facet::ENGINES.len() as f32;
    if tresp.clicked() {
        p[facet::ENGINE] = (engine as f32 + 1.0) % n;
        changed = true;
    }
    if tresp.secondary_clicked() {
        p[facet::ENGINE] = (engine as f32 + n - 1.0) % n;
        changed = true;
    }
    let painter = ui.painter().clone();
    caption(&painter, title.left_center(), Align2::LEFT_CENTER, &format!("{} ›", facet::ENGINES[engine]), if tresp.hovered() { WHITE } else { FACET_COLORS[engine % 4] });
    let voices = live.vals[0].round() as usize;
    if voices > 0 {
        caption(&painter, pos2(rect.right() - 8.0, rect.top() + 14.0), Align2::RIGHT_CENTER, &format!("{voices} voice{}", if voices == 1 { "" } else { "s" }), DIM);
    }
    // The wave: two cycles of what the knobs make, the sound behind it.
    let wave_r = Rect::from_min_max(pos2(rect.left() + 10.0, rect.top() + 26.0), pos2(rect.right() - 10.0, rect.top() + height * 0.6));
    let mid = wave_r.center().y;
    let amp = wave_r.height() * 0.42;
    if live.level() > 0.002 {
        let tail = &live.wave[live.wave.len().saturating_sub(600)..];
        let peak = tail.iter().fold(0.0f32, |m, x| m.max(x.abs())).max(0.05);
        let pts: Vec<Pos2> = tail.iter().enumerate().map(|(i, x)| pos2(wave_r.left() + wave_r.width() * i as f32 / tail.len().max(1) as f32, mid - x / peak * amp)).collect();
        painter.add(Shape::line(pts, Stroke::new(1.0_f32, DIM)));
    }
    painter.line_segment([pos2(wave_r.left(), mid), pos2(wave_r.right(), mid)], Stroke::new(1.0_f32, FAINT));
    let cyc = facet::preview(engine, k, 160);
    let pts: Vec<Pos2> = (0..320).map(|i| pos2(wave_r.left() + wave_r.width() * i as f32 / 319.0, mid - cyc[i % 160].clamp(-1.3, 1.3) / 1.3 * amp)).collect();
    painter.add(Shape::line(pts, Stroke::new(2.0_f32, FACET_COLORS[engine % 4])));
    // The four knobs.
    let row_top = wave_r.bottom() + 8.0;
    let kw = (w - 20.0) / 4.0;
    let radius = ((rect.bottom() - row_top - 22.0) * 0.5).clamp(8.0, 22.0);
    for i in 0..4 {
        let c = pos2(rect.left() + 10.0 + kw * (i as f32 + 0.5), row_top + radius + 2.0);
        let resp = ui.interact(Rect::from_center_size(c, vec2(kw - 4.0, radius * 2.0 + 18.0)), id.with(("knob", i)), Sense::click_and_drag());
        if resp.dragged() {
            let v = (p[facet::BLUE + i] - resp.drag_delta().y / 150.0).clamp(0.0, 1.0);
            if v != p[facet::BLUE + i] {
                p[facet::BLUE + i] = v;
                changed = true;
            }
        }
        if resp.hovered() || resp.dragged() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
        }
        let col = FACET_COLORS[i];
        painter.circle_filled(c, radius, FAINT);
        painter.circle_stroke(c, radius, Stroke::new(1.5_f32, shade(col, 0.6)));
        // The value as an arc from the bottom left.
        let v = k[i].clamp(0.0, 1.0);
        let a0 = PI * 0.75;
        let pts: Vec<Pos2> = (0..=24).map(|s| {
            let a = a0 + v * PI * 1.5 * s as f32 / 24.0;
            c + vec2(a.cos(), a.sin()) * (radius - 3.0)
        }).collect();
        painter.add(Shape::line(pts, Stroke::new(3.0_f32, col)));
        let a = a0 + v * PI * 1.5;
        painter.line_segment([c, c + vec2(a.cos(), a.sin()) * (radius - 3.0)], Stroke::new(2.0_f32, col));
        caption(&painter, pos2(c.x, c.y + radius + 9.0), Align2::CENTER_CENTER, names[i], if resp.hovered() { WHITE } else { shade(col, 0.85) });
    }
    changed
}

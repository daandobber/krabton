//! Screens for the old machines: the VL-Tone (its display showing the
//! eight digits of its program, as the VL-1 showed them, each one clicked
//! up or down; the sound and the rhythm; its little keys) and the sound
//! chips: a lane for every channel of the NES, the Game Boy and the SID,
//! its wave drawn as the chip makes it (pulses of their duty, the stepped
//! triangle, the wave table, the SID's waves and their mixes, noise),
//! lit while it sounds, with its level in the chip's own steps; the SID's
//! filter beneath. Click a wave for the next one (right-click: back), drag
//! a level, drag the SID's filter for cutoff and resonance.

use super::inst_screen::{caption, InstLive};
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, FontId, Id, Painter, Pos2, Rect, Sense, Shape, Stroke, Ui};
use krabton_engine::dsp::{chip, gameboy, nes, sid, vl1};

#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Chip {
    Nes,
    GameBoy,
    Sid,
}

struct Palette {
    bg: Color32,
    line: Color32,
    bright: Color32,
    dim: Color32,
}

fn palette(c: Chip) -> Palette {
    match c {
        // The grey box and its red.
        Chip::Nes => Palette { bg: Color32::from_rgb(0x16, 0x16, 0x18), line: Color32::from_rgb(0x2c, 0x2c, 0x30), bright: Color32::from_rgb(0xe4, 0x3a, 0x2a), dim: Color32::from_rgb(0x70, 0x70, 0x76) },
        // The green screen's four shades.
        Chip::GameBoy => Palette { bg: Color32::from_rgb(0x0f, 0x38, 0x0f), line: Color32::from_rgb(0x30, 0x62, 0x30), bright: Color32::from_rgb(0x9b, 0xbc, 0x0f), dim: Color32::from_rgb(0x8b, 0xac, 0x0f).gamma_multiply(0.55) },
        // Blue on blue.
        Chip::Sid => Palette { bg: Color32::from_rgb(0x35, 0x28, 0x79), line: Color32::from_rgb(0x4a, 0x3d, 0x96), bright: Color32::from_rgb(0xb3, 0xa8, 0xff), dim: Color32::from_rgb(0x6c, 0x5e, 0xb5) },
    }
}

fn set(p: &mut [f32], i: usize, v: f32) -> bool {
    if p.get(i).is_some_and(|x| *x != v) {
        p[i] = v;
        true
    } else {
        false
    }
}

fn get(p: &[f32], i: usize) -> f32 {
    p.get(i).copied().unwrap_or(0.0)
}

/// A wave over two cycles, 0 to 1, as the chip makes it.
#[derive(Clone, Copy)]
enum Wave {
    Pulse(f32),
    Triangle,
    Noise,
    Table(usize),
    Sid(usize, f32),
}

fn noise_at(k: usize, t: f32) -> f32 {
    let s = (k / 3) as u32 ^ ((t * 15.0) as u32).wrapping_mul(2_654_435_761);
    (s.wrapping_mul(2_246_822_519) >> 28) as f32 / 15.0
}

fn wave_points(w: Wave, n: usize, t: f32) -> Vec<f32> {
    let table = if let Wave::Table(i) = w { Some(gameboy::wave_table(i)) } else { None };
    (0..n)
        .map(|k| {
            let ph = (k as f32 / n as f32 * 2.0).fract();
            match w {
                Wave::Pulse(d) => (ph < d) as u8 as f32,
                Wave::Triangle => {
                    let step = (ph * 32.0).floor() as i32;
                    let v = if step < 16 { 15 - step } else { step - 16 };
                    v as f32 / 15.0
                }
                Wave::Noise => noise_at(k, t),
                Wave::Table(_) => table.map(|tb| tb[(ph * 32.0) as usize & 31] as f32 / 15.0).unwrap_or(0.0),
                Wave::Sid(choice, pw) => {
                    let tri = if ph < 0.5 { ph * 2.0 } else { 2.0 - ph * 2.0 };
                    let pulse = (ph >= pw) as u8 as f32;
                    let q = |x: f32| (x * 4095.0) as u32;
                    let and = |a: &[f32]| a.iter().fold(0xFFF, |acc, &x| acc & q(x)) as f32 / 4095.0;
                    match choice {
                        1 => tri,
                        2 => ph,
                        3 => pulse,
                        4 => noise_at(k, t),
                        5 => and(&[tri, ph]),
                        6 => and(&[tri, pulse]),
                        7 => and(&[ph, pulse]),
                        8 => and(&[tri, ph, pulse]),
                        _ => 0.0,
                    }
                }
            }
        })
        .collect()
}

fn draw_wave(p: &Painter, r: Rect, w: Wave, amp: f32, t: f32, color: Color32) {
    let n = 96;
    let pts: Vec<Pos2> = wave_points(w, n, t)
        .iter()
        .enumerate()
        .map(|(k, v)| pos2(r.left() + r.width() * k as f32 / (n - 1) as f32, r.center().y + (0.5 - v) * r.height() * amp))
        .collect();
    // Steps, not slopes: a chip holds each value.
    let mut stepped = Vec::with_capacity(pts.len() * 2);
    for (k, q) in pts.iter().enumerate() {
        if k > 0 {
            stepped.push(pos2(q.x, pts[k - 1].y));
        }
        stepped.push(*q);
    }
    p.add(Shape::line(stepped, Stroke::new(1.5_f32, color)));
}

/// Sixteen steps of level (or as many as the channel has).
fn draw_level(p: &Painter, r: Rect, steps: usize, set_to: f32, live: f32, pal: &Palette) {
    let w = r.width() / steps as f32;
    for k in 0..steps {
        let x = r.left() + k as f32 * w;
        let cell = Rect::from_min_max(pos2(x + 1.0, r.top()), pos2(x + w - 1.0, r.bottom()));
        let frac = (k + 1) as f32 / steps as f32;
        let c = if frac <= live + 1e-3 {
            pal.bright
        } else if frac <= set_to + 1e-3 {
            pal.dim
        } else {
            pal.line
        };
        p.rect_filled(cell, 1.0, c);
    }
}

struct Lane {
    name: &'static str,
    wave: Wave,
    /// The level param (index, steps), if it has one.
    level: Option<(usize, usize)>,
    set_to: f32,
    /// What a click on the wave changes: (param, choices).
    shape: Option<(usize, usize)>,
}

fn lanes(which: Chip, p: &[f32], live: &InstLive) -> Vec<Lane> {
    let duty = |i: usize| [0.125, 0.25, 0.5, 0.75][get(p, i).round().clamp(0.0, 3.0) as usize];
    let lvl = |i: usize, max: f32| (get(p, i) / max).clamp(0.0, 1.0);
    match which {
        Chip::Nes => vec![
            Lane { name: "Pulse 1", wave: Wave::Pulse(duty(nes::P1_DUTY)), level: Some((nes::P1_LEVEL, 15)), set_to: lvl(nes::P1_LEVEL, 15.0), shape: Some((nes::P1_DUTY, 4)) },
            Lane { name: "Pulse 2", wave: Wave::Pulse(duty(nes::P2_DUTY)), level: Some((nes::P2_LEVEL, 15)), set_to: lvl(nes::P2_LEVEL, 15.0), shape: Some((nes::P2_DUTY, 4)) },
            Lane { name: "Triangle", wave: Wave::Triangle, level: Some((nes::TRI_ON, 1)), set_to: lvl(nes::TRI_ON, 1.0), shape: None },
            Lane { name: if get(p, nes::NOISE_SHORT) >= 0.5 { "Noise ~" } else { "Noise" }, wave: Wave::Noise, level: Some((nes::NOISE_LEVEL, 15)), set_to: lvl(nes::NOISE_LEVEL, 15.0), shape: Some((nes::NOISE_SHORT, 2)) },
        ],
        Chip::GameBoy => vec![
            Lane { name: "Pulse 1", wave: Wave::Pulse(duty(gameboy::P1_DUTY)), level: Some((gameboy::P1_LEVEL, 15)), set_to: lvl(gameboy::P1_LEVEL, 15.0), shape: Some((gameboy::P1_DUTY, 4)) },
            Lane { name: "Pulse 2", wave: Wave::Pulse(duty(gameboy::P2_DUTY)), level: Some((gameboy::P2_LEVEL, 15)), set_to: lvl(gameboy::P2_LEVEL, 15.0), shape: Some((gameboy::P2_DUTY, 4)) },
            Lane {
                name: "Wave",
                wave: Wave::Table(get(p, gameboy::WAVEFORM).round().clamp(0.0, (gameboy::WAVES.len() - 1) as f32) as usize),
                level: Some((gameboy::WAVE_LEVEL, 3)),
                set_to: lvl(gameboy::WAVE_LEVEL, 3.0),
                shape: Some((gameboy::WAVEFORM, gameboy::WAVES.len())),
            },
            Lane { name: if get(p, gameboy::NOISE_WIDTH) >= 0.5 { "Noise 7" } else { "Noise" }, wave: Wave::Noise, level: Some((gameboy::NOISE_LEVEL, 15)), set_to: lvl(gameboy::NOISE_LEVEL, 15.0), shape: Some((gameboy::NOISE_WIDTH, 2)) },
        ],
        Chip::Sid => (0..3)
            .map(|v| {
                let choice = get(p, sid::voice_param(v, sid::V_WAVE)).round().clamp(0.0, 8.0) as usize;
                // While it sounds, the width the sweep has moved it to.
                let pw = if live.vals[v * 4] > 0.0 && live.vals[v * 4 + 2] > 0.0 { live.vals[v * 4 + 2] } else { get(p, sid::voice_param(v, sid::V_PW)) };
                Lane {
                    name: ["Voice 1", "Voice 2", "Voice 3"][v],
                    wave: Wave::Sid(choice, pw),
                    level: None,
                    set_to: if choice > 0 { 1.0 } else { 0.0 },
                    shape: Some((sid::voice_param(v, sid::V_WAVE), sid::WAVES.len())),
                }
            })
            .collect(),
    }
}

/// The screen of a sound chip.
pub(crate) fn chip_screen(ui: &mut Ui, id: Id, which: Chip, p: &mut [f32], live: &InstLive, height: f32) -> bool {
    let w = 360.0;
    let (rect, _) = ui.allocate_exact_size(vec2(w, height), Sense::hover());
    let pal = palette(which);
    let painter = ui.painter_at(rect);
    rect_fill(&painter, rect, 6.0, pal.bg);
    rect_line(&painter, rect, 6.0, 1.0, pal.line);
    let t = ui.input(|i| i.time) as f32;
    let mut changed = false;
    let (title, mode_i) = match which {
        Chip::Nes => ("NES  2A03", nes::MODE),
        Chip::GameBoy => ("GAME BOY  DMG", gameboy::MODE),
        Chip::Sid => (if get(p, sid::MODEL) >= 0.5 { "SID  8580" } else { "SID  6581" }, sid::MODE),
    };
    caption(&painter, rect.left_top() + vec2(10.0, 6.0), Align2::LEFT_TOP, title, pal.bright);
    let mode = chip::mode_of(get(p, mode_i));
    // The mode, clicked round.
    let mode_rect = Rect::from_min_size(pos2(rect.right() - 90.0, rect.top() + 3.0), vec2(82.0, 16.0));
    let mresp = ui.interact(mode_rect, id.with("mode"), Sense::click());
    caption(&painter, mode_rect.right_center() - vec2(2.0, 0.0), Align2::RIGHT_CENTER, chip::MODES[mode], if mresp.hovered() { pal.bright } else { pal.dim });
    if mresp.clicked() {
        changed |= set(p, mode_i, ((mode + 1) % chip::MODES.len()) as f32);
    }
    let ls = lanes(which, p, live);
    let rows = ls.len() + usize::from(which == Chip::Sid);
    let top = rect.top() + 24.0;
    let lane_h = ((rect.bottom() - 6.0 - top) / rows as f32).max(12.0);
    for (i, lane) in ls.iter().enumerate() {
        let r = Rect::from_min_size(pos2(rect.left() + 8.0, top + i as f32 * lane_h), vec2(w - 16.0, lane_h - 4.0));
        painter.rect_filled(r, 3.0, pal.line.gamma_multiply(0.45));
        let level = live.vals[i * 4].clamp(0.0, 1.0);
        let on = level > 0.0;
        let label = painter.text(r.left_top() + vec2(6.0, 3.0), Align2::LEFT_TOP, lane.name.to_uppercase(), FontId::monospace(9.5), if on { pal.bright } else { pal.dim });
        if on {
            let note = live.vals[i * 4 + 1].round().clamp(0.0, 127.0) as u8;
            painter.text(pos2(label.left(), label.bottom() + 2.0), Align2::LEFT_TOP, krabton_engine::note_name(note), FontId::monospace(10.0), pal.bright);
        }
        let wave_r = Rect::from_min_max(pos2(r.left() + 70.0, r.top() + 3.0), pos2(r.right() - 74.0, r.bottom() - 3.0));
        let amp = if on { 0.35 + 0.65 * level } else { 0.25 + 0.5 * lane.set_to };
        let color = if on { pal.bright } else if lane.set_to > 0.0 { pal.dim } else { pal.line };
        draw_wave(&painter, wave_r, lane.wave, amp, t, color);
        if let Some((pi, n)) = lane.shape {
            let resp = ui.interact(wave_r, id.with(("shape", i)), Sense::click());
            if resp.hovered() {
                painter.rect_stroke(wave_r.expand(2.0), 2.0, Stroke::new(1.0_f32, pal.dim), egui::StrokeKind::Outside);
            }
            let cur = get(p, pi).round().max(0.0) as usize % n;
            if resp.clicked() {
                changed |= set(p, pi, ((cur + 1) % n) as f32);
            } else if resp.secondary_clicked() {
                changed |= set(p, pi, ((cur + n - 1) % n) as f32);
            }
            let name = match which {
                Chip::GameBoy if pi == gameboy::WAVEFORM => Some(gameboy::WAVES[cur]),
                Chip::Sid => Some(sid::WAVES[cur]),
                _ => None,
            };
            // Its name under the lane's, where there is room.
            if let Some(name) = name.filter(|_| r.height() > 44.0) {
                painter.text(pos2(r.left() + 6.0, r.bottom() - 3.0), Align2::LEFT_BOTTOM, name.to_uppercase(), FontId::monospace(8.5), pal.dim);
            }
        }
        let lr = Rect::from_min_max(pos2(r.right() - 66.0, r.center().y - 4.0), pos2(r.right() - 6.0, r.center().y + 4.0));
        match lane.level {
            Some((pi, steps)) => {
                draw_level(&painter, lr, steps.max(1), lane.set_to, if on { level } else { 0.0 }, &pal);
                let resp = ui.interact(lr.expand2(vec2(0.0, 4.0)), id.with(("level", i)), Sense::click_and_drag());
                if let Some(pos) = resp.interact_pointer_pos().filter(|_| resp.clicked() || resp.dragged()) {
                    let f = ((pos.x - lr.left()) / lr.width()).clamp(0.0, 1.0);
                    changed |= set(p, pi, (f * steps as f32).round());
                }
            }
            None => draw_level(&painter, lr, 15, lane.set_to, level, &pal),
        }
    }
    if which == Chip::Sid {
        let r = Rect::from_min_size(pos2(rect.left() + 8.0, top + 3.0 * lane_h), vec2(w - 16.0, lane_h - 4.0));
        changed |= sid_filter(ui, id.with("filter"), &painter, r, p, live, &pal);
    }
    if live.vals.iter().take(16).any(|v| *v > 0.0) {
        ui.ctx().request_repaint();
    }
    changed
}

/// The SID's filter: its response, cutoff across, resonance up.
fn sid_filter(ui: &mut Ui, id: Id, painter: &Painter, r: Rect, p: &mut [f32], live: &InstLive, pal: &Palette) -> bool {
    let mut changed = false;
    painter.rect_filled(r, 3.0, pal.line.gamma_multiply(0.45));
    let mode = get(p, sid::FILTER_MODE).round().clamp(0.0, 7.0) as usize;
    painter.text(r.left_top() + vec2(6.0, 3.0), Align2::LEFT_TOP, format!("FILTER\n{}", sid::FILTERS[mode].to_uppercase()), FontId::monospace(9.5), pal.dim);
    let area = Rect::from_min_max(pos2(r.left() + 70.0, r.top() + 3.0), pos2(r.right() - 6.0, r.bottom() - 3.0));
    let resp = ui.interact(area, id, Sense::click_and_drag());
    if resp.dragged() {
        let d = resp.drag_delta();
        changed |= set(p, sid::CUTOFF, (get(p, sid::CUTOFF) + d.x * 2047.0 / area.width()).round().clamp(0.0, 2047.0));
        let res = (get(p, sid::RESONANCE) - d.y * 0.15).clamp(0.0, 15.0);
        changed |= set(p, sid::RESONANCE, res);
    }
    if resp.drag_stopped() {
        let res = get(p, sid::RESONANCE).round();
        changed |= set(p, sid::RESONANCE, res);
    }
    if resp.clicked() {
        changed |= set(p, sid::FILTER_MODE, ((mode + 1) % sid::FILTERS.len()) as f32);
    }
    let model = get(p, sid::MODEL).round() as usize;
    // The cutoff where the envelope has it now.
    let env = live.vals[0].clamp(0.0, 1.0);
    let reg = (get(p, sid::CUTOFF) + get(p, sid::ENV_CUTOFF) * 2047.0 * env).clamp(0.0, 2047.0);
    let fc = sid::cutoff_hz(reg, model);
    let q = 0.707 * 2f32.powf(get(p, sid::RESONANCE).round() / 15.0 * if model == 0 { 1.6 } else { 2.0 });
    let bits = [0u8, 1, 2, 4, 5, 3, 6, 7][mode];
    let n = 80;
    let pts: Vec<Pos2> = (0..n)
        .map(|k| {
            let x = k as f32 / (n - 1) as f32;
            let f = 30.0 * (18000.0f32 / 30.0).powf(x);
            let w = f / fc;
            // A two-pole response: low, band and high, summed as the mode has them.
            let den = ((1.0 - w * w).powi(2) + (w / q).powi(2)).sqrt();
            let mut g = 0.0;
            if bits == 0 {
                g = 1.0;
            }
            if bits & 1 != 0 {
                g += 1.0 / den;
            }
            if bits & 2 != 0 {
                g += (w / q) / den;
            }
            if bits & 4 != 0 {
                g += w * w / den;
            }
            let db = 20.0 * g.max(1e-4).log10();
            let y = area.center().y - (db / 30.0).clamp(-0.5, 0.5) * area.height() * 0.9;
            pos2(area.left() + x * area.width(), y)
        })
        .collect();
    painter.add(Shape::line(pts, Stroke::new(1.5_f32, if bits == 0 { pal.dim } else { pal.bright })));
    let fx = area.left() + ((fc / 30.0).ln() / (18000.0f32 / 30.0).ln()).clamp(0.0, 1.0) * area.width();
    painter.line_segment([pos2(fx, area.top()), pos2(fx, area.bottom())], Stroke::new(1.0_f32, pal.dim));
    changed
}

// ---------------------------------------------------------------- VL-Tone

const LCD: Color32 = Color32::from_rgb(0x9c, 0xa8, 0x8e);
const LCD_INK: Color32 = Color32::from_rgb(0x22, 0x26, 0x20);
const LCD_GHOST: Color32 = Color32::from_rgb(0x8d, 0x99, 0x80);
const BODY: Color32 = Color32::from_rgb(0x2a, 0x2b, 0x2f);

/// The VL-Tone: its display with the program's eight digits (sound,
/// attack, decay, sustain level, sustain time, release, vibrato, tremolo),
/// the sound and the rhythm, and its keys.
pub(crate) fn vl1_screen(ui: &mut Ui, id: Id, p: &mut [f32], live: &InstLive, height: f32) -> bool {
    use vl1::*;
    let w = 330.0;
    let (rect, _) = ui.allocate_exact_size(vec2(w, height), Sense::hover());
    let painter = ui.painter_at(rect);
    rect_fill(&painter, rect, 8.0, BODY);
    rect_line(&painter, rect, 8.0, 1.0, Color32::from_rgb(0x44, 0x46, 0x4c));
    let mut changed = false;
    let accent = accent_now();
    caption(&painter, rect.left_top() + vec2(12.0, 7.0), Align2::LEFT_TOP, "VL-Tone", Color32::from_rgb(0xc8, 0xc8, 0xcc));
    // The display.
    let lcd_h = (height * 0.42).clamp(44.0, 90.0);
    let lcd = Rect::from_min_size(pos2(rect.left() + 12.0, rect.top() + 24.0), vec2(w - 24.0, lcd_h));
    painter.rect_filled(lcd, 3.0, LCD);
    let digits = [SOUND, ATTACK, DECAY, SUSTAIN_LEVEL, SUSTAIN_TIME, RELEASE, VIBRATO, TREMOLO];
    let heads = ["S", "A", "D", "L", "T", "R", "V", "T"];
    let cell = (lcd.width() - 16.0) / 8.0;
    let font = FontId::monospace((lcd_h * 0.45).clamp(16.0, 34.0));
    for (k, &pi) in digits.iter().enumerate() {
        let c = Rect::from_min_size(pos2(lcd.left() + 8.0 + k as f32 * cell, lcd.top() + 12.0), vec2(cell, lcd_h - 16.0));
        painter.text(pos2(c.center().x, lcd.top() + 3.0), Align2::CENTER_TOP, heads[k], FontId::monospace(8.0), LCD_INK);
        // The segments that are off show faintly, as on an LCD.
        painter.text(c.center(), Align2::CENTER_CENTER, "8", font.clone(), LCD_GHOST);
        let v = get(p, pi).round().clamp(0.0, 9.0) as usize;
        painter.text(c.center(), Align2::CENTER_CENTER, v.to_string(), font.clone(), LCD_INK);
        let resp = ui.interact(c, id.with(("digit", k)), Sense::click());
        if resp.hovered() {
            painter.rect_stroke(c.shrink(1.0), 2.0, Stroke::new(1.0_f32, LCD_INK.gamma_multiply(0.5)), egui::StrokeKind::Inside);
        }
        if resp.clicked() {
            changed |= set(p, pi, ((v + 1) % 10) as f32);
        } else if resp.secondary_clicked() {
            changed |= set(p, pi, ((v + 9) % 10) as f32);
        }
    }
    // The sound and the rhythm, a mark on its beat.
    let sound = SOUNDS[(get(p, SOUND).round().max(0.0) as usize).min(SOUNDS.len() - 1)];
    let rhythm = (get(p, RHYTHM).round().max(0.0) as usize).min(RHYTHMS.len() - 1);
    let below = lcd.bottom() + 6.0;
    painter.text(pos2(lcd.left(), below), Align2::LEFT_TOP, sound.to_uppercase(), FontId::monospace(11.0), accent);
    let rr = Rect::from_min_size(pos2(lcd.right() - 120.0, below - 2.0), vec2(120.0, 16.0));
    let resp = ui.interact(rr, id.with("rhythm"), Sense::click());
    if resp.clicked() {
        changed |= set(p, RHYTHM, ((rhythm + 1) % RHYTHMS.len()) as f32);
    } else if resp.secondary_clicked() {
        changed |= set(p, RHYTHM, ((rhythm + RHYTHMS.len() - 1) % RHYTHMS.len()) as f32);
    }
    let hit = live.vals[3];
    painter.text(rr.right_top() + vec2(0.0, 2.0), Align2::RIGHT_TOP, RHYTHMS[rhythm].to_uppercase(), FontId::monospace(11.0), if resp.hovered() { TEXT } else { TEXT_DIM });
    if rhythm > 0 && hit > 0.0 {
        let c = pos2(rr.left() + 8.0, rr.center().y + 1.0);
        painter.circle_filled(c, if hit >= 1.5 { 3.0 } else { 4.5 }, accent);
    }
    // The keys: G to B, two and a half octaves, as on the VL-1.
    let kt = below + 22.0;
    let kr = Rect::from_min_max(pos2(rect.left() + 12.0, kt), pos2(rect.right() - 12.0, rect.bottom() - 10.0));
    if kr.height() > 8.0 {
        let (lo, hi) = (55u8, 83u8);
        let n = (hi - lo + 1) as usize;
        let kw = kr.width() / n as f32;
        // The notes held, folded into its keys.
        let held: Vec<i32> = (0..128i32)
            .filter(|&m| live.notes & (1u128 << m) != 0)
            .map(|mut m| {
                while m < lo as i32 {
                    m += 12;
                }
                while m > hi as i32 {
                    m -= 12;
                }
                m
            })
            .collect();
        for k in 0..n {
            let note = lo as i32 + k as i32;
            let black = matches!(note % 12, 1 | 3 | 6 | 8 | 10);
            let r = Rect::from_min_size(pos2(kr.left() + k as f32 * kw + 1.0, kr.top() + if black { 0.0 } else { kr.height() * 0.25 }), vec2(kw - 2.0, kr.height() * 0.75));
            let lit = held.contains(&note);
            let base = if black { Color32::from_rgb(0x3c, 0x3d, 0x43) } else { Color32::from_rgb(0x8a, 0x8c, 0x92) };
            painter.rect_filled(r, 2.0, if lit { accent } else { base });
        }
    }
    if live.vals[4] > 0.0 || (rhythm > 0 && hit > 0.0) {
        ui.ctx().request_repaint();
    }
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
        vals[1] = 60.0;
        let live = InstLive { wave: vec![0.0; 256], notes: 1 << 60, vals };
        for _ in 0..3 {
            let _ = ctx.run(RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1200.0, 800.0))), ..Default::default() }, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| f(ui, &live));
            });
        }
    }

    #[test]
    fn the_old_machines_draw() {
        for name in vl1::preset_names() {
            show(|ui, live| {
                let mut p = vl1::preset(name);
                vl1_screen(ui, Id::new("v"), &mut p, live, 200.0);
            });
        }
        for (which, names, make) in [
            (Chip::Nes, nes::preset_names(), nes::preset as fn(&str) -> Vec<f32>),
            (Chip::GameBoy, gameboy::preset_names(), gameboy::preset),
            (Chip::Sid, sid::preset_names(), sid::preset),
        ] {
            for name in names {
                show(|ui, live| {
                    let mut p = make(name);
                    chip_screen(ui, Id::new("c"), which, &mut p, live, 180.0);
                });
            }
        }
    }

    #[test]
    fn the_sid_waves_mix_as_the_chip_does() {
        let saw = wave_points(Wave::Sid(2, 0.5), 64, 0.0);
        let mixed = wave_points(Wave::Sid(7, 0.5), 64, 0.0);
        // Saw and pulse: only where the pulse is high, and never above the saw.
        assert!(mixed.iter().zip(&saw).all(|(m, s)| *m <= *s + 1e-6));
        assert!(mixed[..16].iter().all(|m| *m == 0.0));
    }
}

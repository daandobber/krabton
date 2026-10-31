//! The Looper's face, laid out as a loop pedal: the big round button (a
//! click is the foot on the pedal: record, overdub, play, overdub...) with
//! the loop drawn round it and a ring that fills as it goes round, the
//! layers and the length in its middle; beside it Stop, Clear, Undo, Redo,
//! Multiply, half and double speed and Reverse; the settings (tempo,
//! quantize, length, after recording, monitor) as buttons to click through;
//! Feedback, Speed and Volume on faders; and the loop to drag out (or To
//! Clip).

use super::modulation::{slider_overlay, ModView};
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, FontId, Id, Rect, Sense, Shape, Stroke, Ui};
use krabton_engine::dsp::fx::looper::{self as lp, LoopAction, LoopMemory, LoopState};
use std::sync::atomic::Ordering;
use std::sync::Arc;

const RECORDING: Color32 = Color32::from_rgb(0xff, 0x45, 0x3a);
const OVERDUB: Color32 = Color32::from_rgb(0xff, 0xb3, 0x30);
const PLAYING: Color32 = Color32::from_rgb(0x4c, 0xd9, 0x64);
const IDLE: Color32 = Color32::from_rgb(0x6a, 0x72, 0x7c);

/// What the face asks of the app.
pub(crate) enum LooperAct {
    ToClip,
    Drag,
}

fn state_color(s: LoopState) -> Color32 {
    match s {
        LoopState::Recording => RECORDING,
        LoopState::Overdubbing => OVERDUB,
        LoopState::Playing => PLAYING,
        _ => IDLE,
    }
}

/// The Looper's panel. Returns whether a parameter changed, and what the
/// app is to do.
pub(crate) fn looper_panel(ui: &mut Ui, id: Id, p: &mut [f32], mem: Option<&Arc<LoopMemory>>, h: f32, mut mv: Option<&mut ModView>) -> (bool, Option<LooperAct>) {
    if p.len() < lp::SPECS.len() {
        return (false, None);
    }
    let mut act = None;
    let mut changed = false;
    let h = (h - 26.0).max(120.0);
    ui.horizontal_top(|ui| {
        pedal(ui, id, mem, h);
        ui.add_space(8.0);
        changed |= controls(ui, id, p, mem, h);
        ui.add_space(8.0);
        changed |= settings(ui, id, p, h);
        ui.add_space(8.0);
        changed |= faders(ui, id, p, h, mv.as_deref_mut());
        ui.add_space(8.0);
        act = out(ui, id, mem, h);
    });
    if mem.is_some_and(|m| !matches!(m.state(), LoopState::Empty | LoopState::Stopped) || m.waiting.load(Ordering::Relaxed)) {
        ui.ctx().request_repaint();
    }
    (changed, act)
}

/// The big button, the loop round it and where it is.
fn pedal(ui: &mut Ui, id: Id, mem: Option<&Arc<LoopMemory>>, h: f32) {
    let (rect, resp) = ui.allocate_exact_size(vec2(h, h), Sense::click());
    let painter = ui.painter_at(rect.expand(2.0));
    let c = rect.center();
    let outer = h * 0.5 - 4.0;
    let state = mem.map(|m| m.state()).unwrap_or(LoopState::Empty);
    let waiting = mem.is_some_and(|m| m.waiting.load(Ordering::Relaxed));
    let color = state_color(state);
    let time = ui.input(|i| i.time);
    let blink = if waiting { 0.45 + 0.55 * ((time * 6.0).sin() * 0.5 + 0.5) as f32 } else { 1.0 };
    painter.circle_filled(c, outer, Color32::from_rgb(0x10, 0x13, 0x16));
    painter.circle_stroke(c, outer, Stroke::new(1.0_f32, Color32::from_gray(60)));
    // The loop's waveform, round the rim (twelve o'clock is its start).
    let ring_in = outer * 0.62;
    let ring_out = outer * 0.92;
    let (len, pos, layers, bpm, rate) = match mem {
        Some(m) => (m.len.load(Ordering::Relaxed), m.pos.load(), m.layers.load(Ordering::Relaxed), m.loop_bpm.load(), m.rate()),
        None => (0, 0.0, 0, 0.0, 48000.0),
    };
    let at = |f: f32| -std::f32::consts::FRAC_PI_2 + f * std::f32::consts::TAU;
    if let Some(m) = mem.filter(|_| len > 0 && state != LoopState::Recording) {
        let peaks = m.peaks(96);
        for (k, v) in peaks.iter().enumerate() {
            let a = at((k as f32 + 0.5) / peaks.len() as f32);
            let r0 = ring_in + (ring_out - ring_in) * 0.5 * (1.0 - v);
            let r1 = ring_in + (ring_out - ring_in) * 0.5 * (1.0 + v);
            let dir = vec2(a.cos(), a.sin());
            painter.line_segment([c + dir * r0, c + dir * r1.max(r0 + 1.0)], Stroke::new(2.0_f32, Color32::from_gray(110)));
        }
    }
    // Where it plays: a ring filled from the start to here.
    let frac = if len > 0 && state != LoopState::Recording { (pos / len as f64).clamp(0.0, 1.0) as f32 } else { 0.0 };
    let track_r = outer - 2.0;
    painter.add(Shape::closed_line(arc_points(c, track_r, 0.0, std::f32::consts::TAU), Stroke::new(4.0_f32, Color32::from_gray(40))));
    if state == LoopState::Recording {
        // Recording: the ring turns round once a second, growing.
        let spin = (time as f32 * 0.8).fract();
        painter.add(Shape::line(arc_points(c, track_r, at(spin), at(spin) + 1.2), Stroke::new(4.0_f32, color)));
    } else if frac > 0.0 {
        painter.add(Shape::line(arc_points(c, track_r, at(0.0), at(frac)), Stroke::new(4.0_f32, color.gamma_multiply(blink))));
        let dir = vec2(at(frac).cos(), at(frac).sin());
        painter.line_segment([c + dir * ring_in * 0.9, c + dir * (outer + 1.0)], Stroke::new(1.5_f32, Color32::WHITE));
    }
    if waiting {
        painter.circle_stroke(c, outer + 1.0, Stroke::new(2.0_f32, color.gamma_multiply(blink).gamma_multiply(0.8)));
    }
    // The button in the middle.
    let button_r = ring_in * 0.86;
    let hot = resp.hovered() && ui.rect_contains_pointer(Rect::from_center_size(c, vec2(button_r * 2.0, button_r * 2.0)));
    let face = mix(Color32::from_rgb(0x22, 0x26, 0x2b), color, if state == LoopState::Empty { 0.08 } else { 0.25 });
    painter.circle_filled(c, button_r, if hot { mix(face, Color32::WHITE, 0.08) } else { face });
    painter.circle_stroke(c, button_r, Stroke::new(2.0_f32, color.gamma_multiply(blink)));
    let next = match state {
        LoopState::Empty => "REC",
        LoopState::Recording => "DUB",
        LoopState::Overdubbing => "PLAY",
        LoopState::Playing => "DUB",
        LoopState::Stopped => "PLAY",
    };
    let big = FontId::proportional((button_r * 0.36).clamp(11.0, 22.0));
    text(&painter, c - vec2(0.0, button_r * 0.28), Align2::CENTER_CENTER, if waiting { "waiting" } else { state.name() }, big, color);
    let small_font = FontId::proportional((button_r * 0.2).clamp(9.0, 13.0));
    let secs = len as f64 / rate.max(1.0) as f64;
    let length = if len == 0 {
        "no loop".to_string()
    } else if bpm > 0.0 {
        let beats = secs * bpm as f64 / 60.0;
        let bar = mem.map(|m| m.bar_beats.load()).unwrap_or(4.0).max(1.0) as f64;
        let bars = beats / bar;
        if (bars - bars.round()).abs() < 0.02 {
            format!("{} bar{}", bars.round(), if bars.round() == 1.0 { "" } else { "s" })
        } else {
            format!("{beats:.1} beats")
        }
    } else {
        format!("{secs:.1} s")
    };
    text(&painter, c + vec2(0.0, button_r * 0.12), Align2::CENTER_CENTER, length, small_font.clone(), Color32::from_gray(200));
    let layer_text = if layers == 1 { "1 layer".to_string() } else { format!("{layers} layers") };
    text(&painter, c + vec2(0.0, button_r * 0.42), Align2::CENTER_CENTER, layer_text, small_font.clone(), Color32::from_gray(150));
    text(&painter, pos2(rect.left() + 2.0, rect.bottom() - 6.0), Align2::LEFT_CENTER, format!("click: {next}"), small(), crate::skin::ink_dim());
    if let (true, Some(m)) = (resp.clicked(), mem) {
        m.press(LoopAction::Pedal);
    }
    resp.on_hover_text("The pedal: record, then overdub, play, overdub... (F8 on the selected track; Shift+F8 stops). A press waits for the next bar or beat when it follows the song");
    let _ = id;
}

/// Stop, Clear, Undo, Redo, Multiply, half and double speed, Reverse.
fn controls(ui: &mut Ui, id: Id, p: &mut [f32], mem: Option<&Arc<LoopMemory>>, h: f32) -> bool {
    let w = 132.0;
    let (rect, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
    let accent = accent_now();
    let undo = mem.map(|m| m.undo.load(Ordering::Relaxed)).unwrap_or(0);
    let row_h = ((h - 3.0 * 3.0) / 4.0).min(34.0);
    let half = (w - 4.0) / 2.0;
    let cell = |k: usize| Rect::from_min_size(pos2(rect.left() + (k % 2) as f32 * (half + 4.0), rect.top() + (k / 2) as f32 * (row_h + 3.0)), vec2(half, row_h));
    let press = |a: LoopAction| {
        if let Some(m) = mem {
            m.press(a);
        }
    };
    let mut changed = false;
    let buttons: [(&str, &str, bool); 8] = [
        ("Stop", "Stop the loop (it stays; the pedal plays it again from its start)", false),
        ("Clear", "Throw the loop away", false),
        ("Undo", "Take back the last overdub", undo & 1 != 0),
        ("Redo", "Put the overdub taken back again", undo & 2 != 0),
        ("x2", "Multiply: the loop twice as long, played twice (to overdub over the longer loop)", false),
        ("Rev", "Play the loop backwards", p[lp::REVERSE] >= 0.5),
        ("1/2", "Half speed: an octave down, twice as long", false),
        ("2x", "Double speed: an octave up, half as long", false),
    ];
    for (k, (label, tip, on)) in buttons.iter().enumerate() {
        let resp = button(ui, cell(k), id.with(("lp_btn", k)), label, *on, accent).on_hover_text(*tip);
        if !resp.clicked() {
            continue;
        }
        match k {
            0 => press(LoopAction::Stop),
            1 => press(LoopAction::Clear),
            2 => press(LoopAction::Undo),
            3 => press(LoopAction::Redo),
            4 => press(LoopAction::Multiply),
            5 => {
                p[lp::REVERSE] = if p[lp::REVERSE] >= 0.5 { 0.0 } else { 1.0 };
                changed = true;
            }
            6 => {
                p[lp::SPEED] = (p[lp::SPEED] - 12.0).max(-36.0);
                changed = true;
            }
            _ => {
                p[lp::SPEED] = (p[lp::SPEED] + 12.0).min(36.0);
                changed = true;
            }
        }
    }
    changed
}

/// A row of buttons for a choice: the one it is lit.
fn choice_row(ui: &mut Ui, id: Id, rect: Rect, p: &mut [f32], pi: usize, names: &[&str], tips: &[&str]) -> bool {
    let n = names.len();
    let bw = rect.width() / n as f32;
    let mut changed = false;
    for (k, name) in names.iter().enumerate() {
        let r = Rect::from_min_size(pos2(rect.left() + k as f32 * bw, rect.top()), vec2(bw - 3.0, rect.height()));
        let on = p[pi].round() as usize == k;
        if button(ui, r, id.with(("row", pi, k)), name, on, accent_now()).on_hover_text(tips.get(k).copied().unwrap_or("")).clicked() {
            p[pi] = k as f32;
            changed = true;
        }
    }
    changed
}

/// A button that steps through a choice: click for the next, right-click
/// for the one before.
fn step_button(ui: &mut Ui, id: Id, rect: Rect, p: &mut [f32], pi: usize, names: &[&str], tip: &str) -> bool {
    let k = (p[pi].round().max(0.0) as usize).min(names.len() - 1);
    let resp = button(ui, rect, id.with(("step", pi)), names[k], k > 0, accent_now()).on_hover_text(tip);
    if resp.clicked() {
        p[pi] = ((k + 1) % names.len()) as f32;
        true
    } else if resp.secondary_clicked() {
        p[pi] = ((k + names.len() - 1) % names.len()) as f32;
        true
    } else {
        false
    }
}

/// Tempo, Quantize, Length, after recording and Monitor.
fn settings(ui: &mut Ui, id: Id, p: &mut [f32], h: f32) -> bool {
    let w = 210.0;
    let (rect, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
    let painter = ui.painter().clone();
    let ink = crate::skin::ink_dim();
    let row_h = ((h - 4.0 * 13.0 - 4.0 * 4.0) / 5.0).clamp(16.0, 26.0);
    let mut y = rect.top();
    let mut changed = false;
    let label = |y: &mut f32, s: &str| {
        text(&painter, pos2(rect.left(), *y + 6.0), Align2::LEFT_CENTER, s, small(), ink);
        *y += 13.0;
    };
    label(&mut y, "Tempo");
    changed |= choice_row(ui, id, Rect::from_min_size(pos2(rect.left(), y), vec2(w, row_h)), p, lp::TEMPO, &["Free", "Follow", "Set & Follow"], &[
        "A loop of its own length; presses come at once",
        "Starts and ends on the beat (Quantize) and follows the song's tempo",
        "The same, and a first loop recorded with the song stopped sets its tempo (whole bars)",
    ]);
    y += row_h + 4.0;
    label(&mut y, "Quantize · Length");
    let half = (w - 3.0) / 2.0;
    changed |= step_button(ui, id, Rect::from_min_size(pos2(rect.left(), y), vec2(half - 3.0, row_h)), p, lp::QUANTIZE, lp::QUANTIZES, "When a press comes, with the song playing: on the next bar, beat or note (click for the next, right-click back)");
    changed |= step_button(ui, id, Rect::from_min_size(pos2(rect.left() + half + 3.0, y), vec2(half - 3.0, row_h)), p, lp::LENGTH, lp::LENGTHS, "Record so many bars, then go on by itself (Free: until the pedal)");
    y += row_h + 4.0;
    label(&mut y, "After recording");
    changed |= choice_row(ui, id, Rect::from_min_size(pos2(rect.left(), y), vec2(w, row_h)), p, lp::AFTER_REC, &["Overdub", "Play"], &["The pedal goes from recording to overdubbing", "The pedal goes from recording to playing"]);
    y += row_h + 4.0;
    label(&mut y, "Hear the input");
    changed |= choice_row(ui, id, Rect::from_min_size(pos2(rect.left(), y), vec2(w, row_h)), p, lp::MONITOR, &["Always", "Never", "Rec/OD"], &["What comes in is always heard", "Only the loop is heard", "What comes in is heard while recording or overdubbing"]);
    changed
}

/// Feedback, Speed and Volume.
fn faders(ui: &mut Ui, id: Id, p: &mut [f32], h: f32, mut mv: Option<&mut ModView>) -> bool {
    let fw = 46.0;
    let list = [lp::FEEDBACK, lp::SPEED, lp::VOLUME];
    let (rect, _) = ui.allocate_exact_size(vec2(fw * list.len() as f32, h), Sense::hover());
    let mut changed = false;
    for (k, &pi) in list.iter().enumerate() {
        let r = Rect::from_min_size(pos2(rect.left() + k as f32 * fw, rect.top()), vec2(fw - 4.0, h));
        let spec = &lp::SPECS[pi];
        let mut v = p[pi];
        let (c, track) = param_slider(ui, r, id.with(("lp_fader", pi)), spec, &mut v, accent_now());
        if c {
            p[pi] = v;
            changed = true;
        }
        if let Some(mv) = mv.as_deref_mut() {
            slider_overlay(ui, r, track, id.with(("lp_mod", pi)), spec, p[pi], pi, mv);
        }
    }
    changed
}

/// The loop to drag out of the device, and To Clip.
fn out(ui: &mut Ui, id: Id, mem: Option<&Arc<LoopMemory>>, h: f32) -> Option<LooperAct> {
    let w = 84.0;
    let (rect, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
    let has = mem.is_some_and(|m| m.len.load(Ordering::Relaxed) > 0 && !matches!(m.state(), LoopState::Empty | LoopState::Recording));
    let grip = Rect::from_min_size(rect.min, vec2(w, (h - 34.0).max(40.0)));
    let resp = ui.interact(grip, id.with("lp_drag"), Sense::drag());
    let p = ui.painter();
    let ink = crate::skin::ink_dim();
    rect_fill(p, grip, 4.0, if resp.hovered() && has { mix(crate::skin::switch_off(), Color32::WHITE, 0.1) } else { crate::skin::switch_off() });
    rect_line(p, grip, 4.0, 1.0, if has { accent_now() } else { Color32::from_gray(70) });
    // A little waveform to hold.
    if let Some(m) = mem.filter(|_| has) {
        let peaks = m.peaks(24);
        let inner = grip.shrink2(vec2(8.0, grip.height() * 0.3));
        let bw = inner.width() / peaks.len() as f32;
        for (k, v) in peaks.iter().enumerate() {
            let x = inner.left() + (k as f32 + 0.5) * bw;
            let hh = (v * inner.height() * 0.5).max(1.0);
            p.line_segment([pos2(x, inner.center().y - hh), pos2(x, inner.center().y + hh)], Stroke::new(2.0_f32, accent_now()));
        }
    }
    text(p, pos2(grip.center().x, grip.top() + 10.0), Align2::CENTER_CENTER, "Drag me", small(), if has { crate::skin::ink() } else { ink });
    let mut act = None;
    if has {
        let resp = resp.on_hover_text("Drag the loop onto a slot, a track's lane in the arrangement or an empty place: an audio clip of it");
        if resp.drag_started() {
            act = Some(LooperAct::Drag);
        }
        if resp.dragged() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
        } else if resp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
        }
    }
    let b = Rect::from_min_size(pos2(rect.left(), rect.bottom() - 28.0), vec2(w, 26.0));
    if button(ui, b, id.with("lp_clip"), "To Clip", false, accent_now()).on_hover_text("The loop as an audio clip on a new audio track").clicked() && has {
        act = Some(LooperAct::ToClip);
    }
    act
}

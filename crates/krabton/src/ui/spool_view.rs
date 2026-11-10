//! Spool's face: a tape machine's dark screen. Along the top the reels,
//! the sound's name and length and the whole tape drawn, the four heads'
//! windows lit on it in their colours; under it a lane for each head (its
//! number switches it on and off, its arrows play it forwards or
//! backwards, the double arrow back and forth, its speed dragged up and
//! down) with its window as a block on the tape: drag it along, or its
//! edges for longer and shorter, and the head runs through it. On the
//! right each head's level and pan; under the lanes how every pass is
//! shaped and the grains; beside it Capture, how the heads run and the mix.

use crate::app::BrowserItem;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, FontId, Id, Rect, Sense, Stroke, StrokeKind, Ui};
use krabton_engine::dsp::fx::looper::{LoopMemory, LoopState};
use krabton_engine::dsp::fx::spool::*;
use std::path::PathBuf;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::Arc;

/// What the face asks of the app.
pub(crate) enum SpoolAct {
    Load(PathBuf),
    Clear,
}

/// The heads' colours: teal, amber, coral and green.
pub(crate) const HEAD_COLOURS: [Color32; 4] = [Color32::from_rgb(0x2e, 0xc4, 0xc8), Color32::from_rgb(0xe0, 0xa8, 0x18), Color32::from_rgb(0xe8, 0x6a, 0x72), Color32::from_rgb(0x34, 0xd6, 0x8e)];
const SCREEN: Color32 = Color32::from_rgb(0x13, 0x15, 0x18);
const LANE: Color32 = Color32::from_rgb(0x1c, 0x1f, 0x23);
const INK: Color32 = Color32::from_rgb(0xd9, 0xde, 0xe3);
const DIM: Color32 = Color32::from_rgb(0x7a, 0x82, 0x8b);
const REC_RED: Color32 = Color32::from_rgb(0xff, 0x3b, 0x30);

fn mono(size: f32) -> FontId {
    FontId::monospace(size)
}

/// A number dragged up and down (Shift: finer; double-click: where it starts).
fn drag_value(ui: &mut Ui, r: Rect, id: Id, v: &mut f32, lo: f32, hi: f32, log: bool, default: f32) -> (egui::Response, bool) {
    let resp = ui.interact(r, id, Sense::click_and_drag());
    let mut changed = false;
    if resp.dragged() {
        let d = -resp.drag_delta().y + resp.drag_delta().x * 0.25;
        let k = if ui.input(|i| i.modifiers.shift) { 0.1 } else { 1.0 };
        let was = *v;
        *v = if log { (v.max(lo) * (d * 0.012 * k).exp()).clamp(lo, hi) } else { (*v + d * (hi - lo) * 0.005 * k).clamp(lo, hi) };
        changed = *v != was;
    }
    if resp.double_clicked() {
        *v = default;
        changed = true;
    }
    if resp.hovered() || resp.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
    }
    (resp, changed)
}

/// A semitone count: click for one up, right-click for one down (Shift: an octave).
fn click_semis(ui: &mut Ui, r: Rect, id: Id, v: &mut f32, lo: f32, hi: f32) -> (egui::Response, bool) {
    let resp = ui.interact(r, id, Sense::click());
    let by = if ui.input(|i| i.modifiers.shift) { 12.0 } else { 1.0 };
    let was = *v;
    if resp.clicked() {
        *v = (*v + by).clamp(lo, hi);
    }
    if resp.secondary_clicked() {
        *v = (*v - by).clamp(lo, hi);
    }
    if resp.double_clicked() {
        *v = 0.0;
    }
    (resp, *v != was)
}

/// A little knob in `col`, dragged up and down.
fn knob(ui: &mut Ui, c: egui::Pos2, rad: f32, id: Id, v: &mut f32, lo: f32, hi: f32, default: f32, col: Color32, bipolar: bool) -> (egui::Response, bool) {
    let r = Rect::from_center_size(c, vec2(rad * 2.0, rad * 2.0));
    let (resp, changed) = drag_value(ui, r, id, v, lo, hi, false, default);
    let p = ui.painter();
    p.circle_filled(c, rad, SCREEN);
    p.circle_stroke(c, rad, Stroke::new(1.0_f32, Color32::from_white_alpha(28)));
    let t = ((*v - lo) / (hi - lo)).clamp(0.0, 1.0);
    let (a0, a1) = (-2.35f32, 2.35f32);
    let at = |x: f32| a0 + (a1 - a0) * x;
    let from = if bipolar { 0.5 } else { 0.0 };
    let (s, e) = if t < from { (t, from) } else { (from, t) };
    let pts: Vec<egui::Pos2> = (0..=16).map(|k| {
        let a = at(s + (e - s) * k as f32 / 16.0);
        c + vec2(a.sin(), -a.cos()) * (rad - 2.0)
    }).collect();
    p.add(egui::Shape::line(pts, Stroke::new(2.5_f32, col)));
    let a = at(t);
    p.line_segment([c, c + vec2(a.sin(), -a.cos()) * (rad - 3.0)], Stroke::new(1.5_f32, INK));
    (resp, changed)
}

/// The tape's loudness in `n` stretches, kept while the tape is the same.
fn peaks(ui: &Ui, id: Id, m: &LoopMemory, n: usize) -> Arc<Vec<f32>> {
    let key = id.with("spool_peaks");
    let sig = (m.generation.load(Relaxed), m.len.load(Relaxed), n);
    let recording = m.state() == LoopState::Recording;
    if let Some((s, v)) = ui.data(|d| d.get_temp::<((u32, usize, usize), Arc<Vec<f32>>)>(key)) {
        if s == sig && !recording {
            return v;
        }
    }
    let v = Arc::new(m.peaks(n));
    ui.data_mut(|d| d.insert_temp(key, (sig, v.clone())));
    v
}

fn wave(p: &egui::Painter, r: Rect, pk: &[f32], col: Color32) {
    let n = pk.len().max(1);
    let w = r.width() / n as f32;
    for (k, v) in pk.iter().enumerate() {
        let x = r.left() + (k as f32 + 0.5) * w;
        let hh = (v.sqrt() * r.height() * 0.48).max(0.5);
        p.line_segment([pos2(x, r.center().y - hh), pos2(x, r.center().y + hh)], Stroke::new(w.max(1.0) * 0.8, col));
    }
}

/// A head's window on the tape (0..1), as the engine places it.
fn window_of(p: &[f32], h: usize) -> (f32, f32) {
    let len = p[head(h, H_LENGTH)].clamp(0.002, 1.0);
    let a = p[head(h, H_START)].clamp(0.0, 1.0).min(1.0 - len);
    (a, a + len)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn spool_panel(ui: &mut Ui, id: Id, p: &mut [f32], mem: Option<&Arc<LoopMemory>>, name: Option<&str>, h: f32, width: &mut f32) -> (bool, Option<SpoolAct>) {
    let mut changed = false;
    let mut act = None;
    if p.len() < SPECS.len() {
        return (false, None);
    }
    let hh = (h - 30.0).max(170.0);
    let t = ui.input(|i| i.time);
    let rate = mem.map_or(48000.0, |m| m.rate());
    let len = mem.map_or(0, |m| m.len.load(Relaxed).min(m.capacity()));
    let capturing = mem.is_some_and(|m| m.state() == LoopState::Recording) || p[CAPTURE] >= 0.5;
    let running: Vec<f32> = (0..HEADS).map(|k| mem.map_or(-1.0, |m| m.heads[k].load())).collect();
    let any_running = running.iter().any(|v| *v >= 0.0);
    if any_running || capturing {
        ui.ctx().request_repaint();
    }
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        // Capture: a strip down the left.
        let (cap, cresp) = ui.allocate_exact_size(vec2(26.0, hh), Sense::click());
        {
            let pt = ui.painter();
            let lit = capturing && (t * 2.0).fract() < 0.6;
            rect_fill(pt, cap, 5.0, if capturing { Color32::from_rgb(0x3a, 0x12, 0x10) } else { Color32::from_rgb(0x24, 0x27, 0x2b) });
            pt.circle_filled(pos2(cap.center().x, cap.top() + 16.0), 6.0, if lit { REC_RED } else { Color32::from_rgb(0x6a, 0x2a, 0x26) });
            for (k, ch) in "CAPTURE".chars().enumerate() {
                text(pt, pos2(cap.center().x, cap.top() + 36.0 + k as f32 * 13.0), Align2::CENTER_CENTER, ch, mono(10.0), if capturing { REC_RED } else { DIM });
            }
            if capturing {
                text(pt, pos2(cap.center().x, cap.bottom() - 12.0), Align2::CENTER_CENTER, format!("{:.0}", len as f32 / rate), mono(10.0), INK);
            }
        }
        if cresp.on_hover_text("Capture: what comes into Spool recorded onto its tape until you click again (the sound it had goes)").clicked() {
            p[CAPTURE] = if p[CAPTURE] >= 0.5 { 0.0 } else { 1.0 };
            changed = true;
        }

        // The screen.
        let sw = width.clamp(420.0, 1400.0);
        let (screen, sresp) = ui.allocate_exact_size(vec2(sw, hh), Sense::click());
        let pt = ui.painter().with_clip_rect(screen);
        rect_fill(&pt, screen, 6.0, SCREEN);
        let inner = screen.shrink(6.0);
        let head_h = 22.0;
        let over_h = 28.0;
        let foot_h = 30.0;
        let lanes_r = Rect::from_min_max(pos2(inner.left(), inner.top() + head_h + over_h + 4.0), pos2(inner.right(), inner.bottom() - foot_h - 4.0));
        let lane_h = lanes_r.height() / HEADS as f32;
        // The reels, turning while a head runs.
        let reel_y = inner.top() + head_h * 0.5;
        let spin = if any_running { (t * 2.6) as f32 } else { 0.0 };
        for (k, cx) in [inner.left() + 14.0, inner.left() + 50.0].into_iter().enumerate() {
            let c = pos2(cx, reel_y);
            pt.circle_stroke(c, 8.5, Stroke::new(1.5_f32, INK.gamma_multiply(0.8)));
            pt.circle_filled(c, 2.0, INK);
            for s in 0..3 {
                let a = spin * if k == 0 { 1.0 } else { 1.3 } + s as f32 * 2.094;
                pt.line_segment([c + vec2(a.cos(), a.sin()) * 2.5, c + vec2(a.cos(), a.sin()) * 7.0], Stroke::new(1.2_f32, INK.gamma_multiply(0.7)));
            }
        }
        pt.line_segment([pos2(inner.left() + 22.0, reel_y + 8.0), pos2(inner.left() + 42.0, reel_y + 8.0)], Stroke::new(1.0_f32, DIM));
        let title = if capturing { "capturing...".to_string() } else { name.map(str::to_string).unwrap_or_else(|| if len > 0 { "Captured".into() } else { "Drop a sound here, or Capture".into() }) };
        text(&pt, pos2(inner.left() + 70.0, reel_y), Align2::LEFT_CENTER, title, mono(11.0), DIM);
        let secs = len as f32 / rate;
        text(&pt, pos2(inner.right() - 2.0, reel_y), Align2::RIGHT_CENTER, format!("{}:{:02}", (secs / 60.0) as u32, (secs % 60.0) as u32), mono(11.0), DIM);
        // The whole tape, the windows lit on it.
        let over = Rect::from_min_size(pos2(inner.left() + 70.0, inner.top() + head_h), vec2(inner.width() - 70.0, over_h));
        let pk = mem.map(|m| peaks(ui, id, m, 220));
        if let Some(pk) = &pk {
            wave(&pt, over, pk, DIM.gamma_multiply(0.7));
            for h in 0..HEADS {
                if p[head(h, H_ON)] < 0.5 {
                    continue;
                }
                let (a, b) = window_of(p, h);
                let n = pk.len();
                let (ka, kb) = ((a * n as f32) as usize, ((b * n as f32).ceil() as usize).min(n));
                if kb > ka {
                    let r = Rect::from_min_max(pos2(over.left() + over.width() * ka as f32 / n as f32, over.top()), pos2(over.left() + over.width() * kb as f32 / n as f32, over.bottom()));
                    wave(&pt, r, &pk[ka..kb], HEAD_COLOURS[h]);
                }
            }
            for (h, at) in running.iter().enumerate() {
                if *at >= 0.0 {
                    let x = over.left() + over.width() * at.clamp(0.0, 1.0);
                    pt.line_segment([pos2(x, over.top()), pos2(x, over.bottom())], Stroke::new(1.0_f32, HEAD_COLOURS[h]));
                }
            }
        }
        // A sound dropped on the screen.
        if let Some(item) = drop_payload::<BrowserItem>(&sresp) {
            if let BrowserItem::Sample(path) = &*item {
                act = Some(SpoolAct::Load(path.clone()));
            }
        }
        if sresp.dnd_hover_payload::<BrowserItem>().is_some() {
            rect_line(ui.painter(), screen, 6.0, 2.0, HEAD_COLOURS[0]);
        }
        if sresp.double_clicked() && len == 0 {
            if let Some(path) = crate::import::audio_file_dialog().pick_file() {
                act = Some(SpoolAct::Load(path));
            }
        }

        // The lanes.
        for h in 0..HEADS {
            let col = HEAD_COLOURS[h];
            let lane = Rect::from_min_size(pos2(lanes_r.left(), lanes_r.top() + h as f32 * lane_h), vec2(lanes_r.width(), lane_h - 2.0));
            let on = p[head(h, H_ON)] >= 0.5;
            rect_fill(&pt, lane, 3.0, LANE);
            // Its number: on and off.
            let num = Rect::from_min_size(lane.min, vec2(18.0, lane.height()));
            let nresp = ui.interact(num, id.with(("num", h)), Sense::click());
            rect_fill(&pt, num, 3.0, if on { col } else { Color32::from_rgb(0x2a, 0x2e, 0x33) });
            text(&pt, num.center(), Align2::CENTER_CENTER, format!("{}", h + 1), mono(11.0), if on { Color32::BLACK } else { DIM });
            if nresp.on_hover_text("This head on or off").clicked() {
                p[head(h, H_ON)] = if on { 0.0 } else { 1.0 };
                changed = true;
            }
            // Backwards, its speed, forwards, back and forth.
            let cy = lane.center().y;
            let rev = p[head(h, H_REVERSE)] >= 0.5;
            let pp = p[head(h, H_PINGPONG)] >= 0.5;
            let arrow = |pt: &egui::Painter, c: egui::Pos2, left: bool, lit: bool| {
                pt.circle_stroke(c, 7.0, Stroke::new(1.3_f32, if lit { col } else { DIM }));
                text(pt, c, Align2::CENTER_CENTER, if left { "<" } else { ">" }, mono(10.0), if lit { col } else { DIM });
            };
            let lc = pos2(num.right() + 13.0, cy);
            let rc = pos2(num.right() + 84.0, cy);
            arrow(&pt, lc, true, rev);
            arrow(&pt, rc, false, !rev);
            let lr = ui.interact(Rect::from_center_size(lc, vec2(16.0, 16.0)), id.with(("rev", h)), Sense::click());
            if lr.on_hover_text("Backwards, as a tape turned over").clicked() && !rev {
                p[head(h, H_REVERSE)] = 1.0;
                changed = true;
            }
            let rr = ui.interact(Rect::from_center_size(rc, vec2(16.0, 16.0)), id.with(("fwd", h)), Sense::click());
            if rr.on_hover_text("Forwards").clicked() && rev {
                p[head(h, H_REVERSE)] = 0.0;
                changed = true;
            }
            let speed_r = Rect::from_center_size(pos2((lc.x + rc.x) * 0.5, cy), vec2(44.0, 16.0));
            let mut sp = p[head(h, H_SPEED)];
            let (sresp, c) = drag_value(ui, speed_r, id.with(("speed", h)), &mut sp, 0.05, 4.0, true, 1.0);
            if c {
                p[head(h, H_SPEED)] = sp;
                changed = true;
            }
            text(&pt, speed_r.center(), Align2::CENTER_CENTER, format!("{sp:.2}"), mono(12.0), col);
            sresp.on_hover_text("Its speed: drag up or down (Shift: finer; double-click: 1.00). Without Stretch the pitch goes with it, as tape");
            let ppc = pos2(rc.x + 20.0, cy);
            text(&pt, ppc, Align2::CENTER_CENTER, "<>", mono(11.0), if pp { col } else { DIM });
            let ppr = ui.interact(Rect::from_center_size(ppc, vec2(18.0, 16.0)), id.with(("pp", h)), Sense::click());
            if ppr.on_hover_text("Back and forth through its window (ping-pong), rather than round").clicked() {
                p[head(h, H_PINGPONG)] = if pp { 0.0 } else { 1.0 };
                changed = true;
            }
            // Its pitch: click up, right-click down.
            let pr = Rect::from_center_size(pos2(ppc.x + 26.0, cy), vec2(28.0, 16.0));
            let mut pi = p[head(h, H_PITCH)];
            let (presp, c) = click_semis(ui, pr, id.with(("pitch", h)), &mut pi, -24.0, 24.0);
            if c {
                p[head(h, H_PITCH)] = pi;
                changed = true;
            }
            text(&pt, pr.center(), Align2::CENTER_CENTER, if pi == 0.0 { "0st".to_string() } else { format!("{pi:+.0}") }, mono(10.0), if pi != 0.0 { col } else { DIM });
            presp.on_hover_text("Its own pitch: click a semitone up, right-click down (Shift: an octave; double-click: none)");
            // Its window on the tape.
            let tl = Rect::from_min_max(pos2(pr.right() + 6.0, lane.top() + 2.0), pos2(lane.right() - 2.0, lane.bottom() - 2.0));
            rect_fill(&pt, tl, 2.0, SCREEN.gamma_multiply(0.9));
            if let Some(pk) = &pk {
                wave(&pt, tl.shrink2(vec2(0.0, 3.0)), pk, Color32::from_white_alpha(18));
            }
            let (a, b) = window_of(p, h);
            let block = Rect::from_min_max(pos2(tl.left() + tl.width() * a, tl.top()), pos2((tl.left() + tl.width() * b).max(tl.left() + tl.width() * a + 3.0), tl.bottom()));
            pt.rect_filled(block, 2.0, col.gamma_multiply(if on { 0.75 } else { 0.25 }));
            if pp {
                pt.line_segment([pos2(block.left() + 3.0, block.center().y), pos2(block.right() - 3.0, block.center().y)], Stroke::new(1.0_f32, Color32::from_black_alpha(90)));
            }
            if running[h] >= 0.0 {
                let x = tl.left() + tl.width() * running[h].clamp(0.0, 1.0);
                pt.line_segment([pos2(x, tl.top()), pos2(x, tl.bottom())], Stroke::new(1.5_f32, INK));
            }
            let tresp = ui.interact(tl, id.with(("tl", h)), Sense::click_and_drag());
            let mode_id = id.with(("tl_mode", h));
            if tresp.drag_started() {
                let x = tresp.interact_pointer_pos().map_or(block.center().x, |q| q.x);
                let mode: u8 = if (x - block.left()).abs() < 6.0 {
                    1
                } else if (x - block.right()).abs() < 6.0 {
                    2
                } else {
                    if x < block.left() || x > block.right() {
                        // Started outside: the window jumps there first.
                        let l = b - a;
                        p[head(h, H_START)] = (((x - tl.left()) / tl.width()) - l * 0.5).clamp(0.0, 1.0 - l);
                    }
                    0
                };
                ui.data_mut(|d| d.insert_temp(mode_id, mode));
                changed = true;
            }
            if tresp.dragged() {
                let dx = tresp.drag_delta().x / tl.width();
                let mode = ui.data(|d| d.get_temp::<u8>(mode_id)).unwrap_or(0);
                let (a, b) = window_of(p, h);
                match mode {
                    1 => {
                        let na = (a + dx).clamp(0.0, b - 0.002);
                        p[head(h, H_START)] = na;
                        p[head(h, H_LENGTH)] = (b - na).clamp(0.002, 1.0);
                    }
                    2 => {
                        p[head(h, H_START)] = a;
                        p[head(h, H_LENGTH)] = (b + dx - a).clamp(0.002, 1.0 - a);
                    }
                    _ => {
                        let l = b - a;
                        p[head(h, H_START)] = (a + dx).clamp(0.0, 1.0 - l);
                    }
                }
                changed = true;
            }
            if let Some(q) = tresp.hover_pos() {
                if (q.x - block.left()).abs() < 6.0 || (q.x - block.right()).abs() < 6.0 {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                } else if block.contains(q) {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
                }
            }
            if tresp.clicked() {
                let x = tresp.interact_pointer_pos().map_or(block.center().x, |q| q.x);
                if !block.contains(pos2(x, block.center().y)) {
                    let l = b - a;
                    p[head(h, H_START)] = (((x - tl.left()) / tl.width()) - l * 0.5).clamp(0.0, 1.0 - l);
                    changed = true;
                }
            }
            tresp.on_hover_text(format!("Head {}'s window: drag it along, its edges for longer or shorter, or click elsewhere to put it there ({:.1} % of the tape, {:.2} s)", h + 1, (b - a) * 100.0, (b - a) * secs));
        }

        // How every pass is shaped, and the grains.
        let foot = Rect::from_min_max(pos2(inner.left(), inner.bottom() - foot_h), inner.max);
        pt.line_segment([pos2(foot.left(), foot.top() - 2.0), pos2(foot.right(), foot.top() - 2.0)], Stroke::new(1.0_f32, Color32::from_white_alpha(14)));
        let cells: [(usize, &str, &str); 6] = [
            (TONE, "TONE", "Darker one way, thinner the other (on the heads)"),
            (TRANSPOSE, "PITCH", "Every head up or down: click a semitone up, right-click down (Shift: an octave)"),
            (FADE_IN, "FADE IN", "How each pass of a window swells in"),
            (FADE_OUT, "FADE OUT", "How each pass of a window fades away"),
            (SPRAY, "SPRAY", "Stretch: how far the grains scatter round the head"),
            (GRAIN, "GRAIN", "Stretch: how long each grain is"),
        ];
        let cw = (foot.width() - 70.0) / cells.len() as f32;
        for (k, (pi, label, tip)) in cells.into_iter().enumerate() {
            let r = Rect::from_min_size(pos2(foot.left() + k as f32 * cw, foot.top()), vec2(cw - 4.0, foot_h));
            text(&pt, pos2(r.center().x, r.top() + 7.0), Align2::CENTER_CENTER, label, mono(8.5), DIM);
            let spec = &SPECS[pi];
            let mut v = p[pi];
            let vr = Rect::from_min_max(pos2(r.left(), r.top() + 12.0), r.max);
            let (resp, c) = if pi == TRANSPOSE {
                click_semis(ui, vr, id.with(("foot", pi)), &mut v, spec.min, spec.max)
            } else {
                drag_value(ui, vr, id.with(("foot", pi)), &mut v, spec.min, spec.max, pi == GRAIN, spec.default)
            };
            if c {
                p[pi] = v;
                changed = true;
            }
            let shown = match pi {
                TONE => format!("{:+.0} %", v * 100.0),
                TRANSPOSE => format!("{v:+.0} st"),
                GRAIN => format!("{v:.0} ms"),
                _ => format!("{:.1} %", v * 100.0),
            };
            let stretch_only = matches!(pi, SPRAY | GRAIN) && p[STRETCH] < 0.5;
            text(&pt, pos2(r.center().x, r.top() + 21.0), Align2::CENTER_CENTER, shown, mono(12.0), if stretch_only { DIM } else { HEAD_COLOURS[3] });
            resp.on_hover_text(tip);
        }
        let st = Rect::from_min_max(pos2(foot.right() - 66.0, foot.top() + 6.0), pos2(foot.right(), foot.bottom() - 2.0));
        let on = p[STRETCH] >= 0.5;
        pt.rect_stroke(st, 3.0, Stroke::new(1.2_f32, if on { HEAD_COLOURS[3] } else { DIM }), StrokeKind::Inside);
        if on {
            pt.rect_filled(st.shrink(2.0), 2.0, HEAD_COLOURS[3].gamma_multiply(0.2));
        }
        text(&pt, st.center(), Align2::CENTER_CENTER, "Stretch", mono(11.0), if on { HEAD_COLOURS[3] } else { DIM });
        let sr = ui.interact(st, id.with("stretch"), Sense::click());
        if sr.on_hover_text("Grains instead of tape: speed and pitch part, so a head slowed to a crawl holds its pitch (a drone), Spray and Grain shape them").clicked() {
            p[STRETCH] = if on { 0.0 } else { 1.0 };
            changed = true;
        }
        width_grip(ui, id.with("spool_width"), screen, width, 420.0, 1400.0);

        // Each head's level and pan.
        let (col_r, _) = ui.allocate_exact_size(vec2(58.0, hh), Sense::hover());
        for h in 0..HEADS {
            let cy = lanes_r.top() + (h as f32 + 0.5) * lane_h;
            let mut lv = p[head(h, H_LEVEL)];
            let peak = mem.map_or(0.0, |m| m.head_peaks[h].load()).clamp(0.0, 1.0);
            let c1 = pos2(col_r.left() + 14.0, cy);
            if peak > 0.01 {
                ui.painter().circle_filled(c1, 13.0, HEAD_COLOURS[h].gamma_multiply(0.15 + 0.35 * peak));
            }
            let (r1, c) = knob(ui, c1, 11.0, id.with(("lvl", h)), &mut lv, 0.0, 1.0, 0.7, HEAD_COLOURS[h], false);
            if c {
                p[head(h, H_LEVEL)] = lv;
                changed = true;
            }
            r1.on_hover_text(format!("Head {}'s level: {:.0} %", h + 1, lv * 100.0));
            let mut pn = p[head(h, H_PAN)];
            let (r2, c) = knob(ui, pos2(col_r.left() + 42.0, cy), 8.0, id.with(("pan", h)), &mut pn, -1.0, 1.0, 0.0, HEAD_COLOURS[h].gamma_multiply(0.6), true);
            if c {
                p[head(h, H_PAN)] = pn;
                changed = true;
            }
            r2.on_hover_text(format!("Head {}'s pan: {}", h + 1, if pn.abs() < 0.01 { "centre".to_string() } else if pn < 0.0 { format!("{:.0} left", -pn * 100.0) } else { format!("{:.0} right", pn * 100.0) }));
        }

        // How the heads run, and the mix.
        ui.vertical(|ui| {
            ui.set_width(112.0);
            ui.spacing_mut().item_spacing.y = 3.0;
            ui.label(egui::RichText::new("Playback").small().color(crate::skin::ink_dim()));
            let mode = p[PLAYBACK].round() as usize;
            let tips = ["The heads run while the song plays, winding down when it stops and up when it starts", "The heads run all the time", "The heads run while the track holds a note (a MIDI track, or notes played into it), from their starts on every new note, and the note transposes them (C3: as set)"];
            for (k, nm) in PLAYBACKS.iter().enumerate() {
                let lit = mode == k;
                let b = egui::Button::new(egui::RichText::new(*nm).small().color(if lit { Color32::BLACK } else { crate::skin::ink() })).fill(if lit { HEAD_COLOURS[1] } else { crate::skin::well().0 }).min_size(vec2(100.0, 18.0));
                if ui.add(b).on_hover_text(tips[k]).clicked() {
                    p[PLAYBACK] = k as f32;
                    changed = true;
                }
            }
            ui.add_space(4.0);
            let (row, _) = ui.allocate_exact_size(vec2(112.0, 52.0), Sense::hover());
            for (k, (pi, label, tip)) in [(INERTIA, "TAPE", "How long the tape takes to wind down and up again (a tape stop, a tape start)"), (INPUT, "INPUT", "How much of what comes in is heard as well"), (LEVEL, "LEVEL", "The heads' level")].into_iter().enumerate() {
                let c = pos2(row.left() + 18.0 + k as f32 * 38.0, row.top() + 18.0);
                let spec = &SPECS[pi];
                let mut v = p[pi];
                let (r, ch) = knob(ui, c, 13.0, id.with(("mix", pi)), &mut v, spec.min, spec.max, spec.default, HEAD_COLOURS[0], false);
                if ch {
                    p[pi] = v;
                    changed = true;
                }
                text(ui.painter(), pos2(c.x, row.top() + 42.0), Align2::CENTER_CENTER, label, mono(8.5), crate::skin::ink_dim());
                let shown = if pi == LEVEL { format!("{v:+.1} dB") } else { format!("{:.0} %", v * 100.0) };
                r.on_hover_text(format!("{tip}: {shown}"));
            }
            ui.horizontal(|ui| {
                if ui.small_button("Load...").on_hover_text("A sound for the tape (or drop one from the browser on the screen)").clicked() {
                    if let Some(path) = crate::import::audio_file_dialog().pick_file() {
                        act = Some(SpoolAct::Load(path));
                    }
                }
                if len > 0 && ui.small_button("Clear").on_hover_text("An empty tape").clicked() {
                    act = Some(SpoolAct::Clear);
                }
            });
        });
    });
    (changed, act)
}

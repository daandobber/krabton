//! Screens for Warp, Hybrid and Quartet. Warp's is its envelopes: a tab for each of
//! the five, its eight steps drawn in time (a slow step long, a fast one
//! short), each point dragged up and down for its level and sideways for
//! its rate; right-click a point to hold on it while the key is down (S),
//! double-click it to end there (E); the step a voice is on lights up. Beside
//! them the two lines' waves, bent as far as they are now (click one for the
//! next wave). Hybrid's: its two oscillators' waves (click for the next
//! engine) and its filters' curves (drag for the first's cutoff and
//! resonance, click for its kind). Quartet's: its four oscillators, each
//! its mode (click for the next, right-click back) and its wave (the same),
//! lit as loud as it plays; the matrix slots in use; the envelopes moving.

use super::inst_screen::{caption, InstLive, SCREEN_BG};
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, FontId, Id, Painter, Pos2, Rect, Sense, Shape, Stroke, Ui};
use krabton_engine::dsp::{hybrid, quartet, warp};

const DIM: Color32 = Color32::from_rgb(0x4a, 0x4e, 0x58);
const FAINT: Color32 = Color32::from_rgb(0x1e, 0x21, 0x28);
const YELLOW: Color32 = Color32::from_rgb(0xff, 0xd4, 0x3b);

fn frame(ui: &Ui, rect: Rect) -> Painter {
    let p = ui.painter_at(rect);
    rect_fill(&p, rect, 6.0, SCREEN_BG);
    rect_line(&p, rect, 6.0, 1.0, Color32::from_rgb(0x26, 0x29, 0x31));
    p
}

fn get(p: &[f32], i: usize) -> f32 {
    p.get(i).copied().unwrap_or(0.0)
}

fn set(p: &mut [f32], i: usize, v: f32) -> bool {
    if p.get(i).is_some_and(|x| *x != v) {
        p[i] = v;
        true
    } else {
        false
    }
}

fn draw_wave(painter: &Painter, r: Rect, f: impl Fn(f32) -> f32, color: Color32) {
    let n = 120;
    let pts: Vec<Pos2> = (0..=n)
        .map(|k| {
            let x = k as f32 / n as f32;
            pos2(r.left() + x * r.width(), r.center().y - f(x).clamp(-1.2, 1.2) * r.height() * 0.42)
        })
        .collect();
    painter.add(Shape::line(pts, Stroke::new(1.5_f32, color)));
}

/// Where each step's point is: x from the time it takes, y from its level.
fn env_points(p: &[f32], e: usize, r: Rect) -> Vec<Pos2> {
    let mut times = Vec::with_capacity(8);
    let mut prev = 0.0;
    for k in 0..8 {
        let lvl = get(p, warp::env_param(e, warp::LEVEL + k));
        let t = warp::rate_secs(get(p, warp::env_param(e, warp::RATE + k))) * ((lvl - prev).abs() / 99.0).max(0.08);
        // Long ones squeezed, so a slow step does not push the rest away.
        times.push(14.0 + 60.0 * t / (t + 0.4));
        prev = lvl;
    }
    let total: f32 = times.iter().sum();
    let scale = ((r.width() - 8.0) / total).min(1.6);
    let mut x = r.left();
    (0..8)
        .map(|k| {
            x += times[k] * scale;
            pos2(x, r.bottom() - get(p, warp::env_param(e, warp::LEVEL + k)) / 99.0 * r.height())
        })
        .collect()
}

pub(crate) fn warp_screen(ui: &mut Ui, id: Id, p: &mut [f32], live: &InstLive, height: f32) -> bool {
    use warp::*;
    let w = 470.0;
    let (rect, _) = ui.allocate_exact_size(vec2(w, height), Sense::hover());
    let painter = frame(ui, rect);
    let accent = accent_now();
    let mut changed = false;
    caption(&painter, rect.left_top() + vec2(10.0, 6.0), Align2::LEFT_TOP, "Warp", accent);
    // The tabs.
    let tab_id = id.with("tab");
    let mut tab: usize = ui.ctx().memory(|m| m.data.get_temp(tab_id)).unwrap_or(ENV_AMP1);
    let mut x = rect.left() + 56.0;
    for (e, name) in ENV_NAMES.iter().enumerate() {
        let tr = Rect::from_min_size(pos2(x, rect.top() + 4.0), vec2(56.0, 15.0));
        let resp = ui.interact(tr, id.with(("tab", e)), Sense::click());
        let on = tab == e;
        if on {
            painter.rect_filled(tr, 3.0, accent.gamma_multiply(0.25));
        }
        painter.text(tr.center(), Align2::CENTER_CENTER, name.to_uppercase(), FontId::monospace(9.0), if on { accent } else if resp.hovered() { TEXT } else { TEXT_DIM });
        if resp.clicked() {
            tab = e;
        }
        x += 58.0;
    }
    ui.ctx().memory_mut(|m| m.data.insert_temp(tab_id, tab));
    // The envelope.
    let er = Rect::from_min_max(pos2(rect.left() + 10.0, rect.top() + 28.0), pos2(rect.right() - 128.0, rect.bottom() - 18.0));
    for k in 0..=4 {
        let y = er.bottom() - er.height() * k as f32 / 4.0;
        painter.line_segment([pos2(er.left(), y), pos2(er.right(), y)], Stroke::new(1.0_f32, FAINT));
    }
    let e = tab;
    let pts = env_points(p, e, er);
    let sus = get(p, env_param(e, SUSTAIN)).round().clamp(1.0, 8.0) as usize - 1;
    let end = get(p, env_param(e, END)).round().clamp(1.0, 8.0) as usize - 1;
    let start = pos2(er.left(), er.bottom());
    let mut line = vec![start];
    line.extend(pts.iter().take(end + 1).copied());
    painter.add(Shape::line(line, Stroke::new(1.5_f32, accent)));
    // The steps past its end, faint.
    if end < 7 {
        painter.add(Shape::line(pts[end..].to_vec(), Stroke::new(1.0_f32, DIM)));
    }
    let on_step = live.vals[6 + e];
    let playing = live.vals[11] > 0.0;
    for (k, &pt) in pts.iter().enumerate() {
        let r = Rect::from_center_size(pt, vec2(14.0, 14.0));
        let resp = ui.interact(r, id.with(("pt", e, k)), Sense::click_and_drag());
        let hot = resp.hovered() || resp.dragged();
        let lit = playing && on_step.round() as usize == k + 1;
        let c = if k > end { DIM } else if lit { YELLOW } else { accent };
        painter.circle_filled(pt, if hot { 5.0 } else { 3.5 }, c);
        if k == sus {
            painter.text(pt + vec2(0.0, -8.0), Align2::CENTER_BOTTOM, "S", FontId::monospace(9.0), TEXT);
        }
        if k == end {
            painter.text(pt + vec2(0.0, if k == sus { -18.0 } else { -8.0 }), Align2::CENTER_BOTTOM, "E", FontId::monospace(9.0), TEXT);
        }
        if resp.dragged() {
            let d = resp.drag_delta();
            let li = env_param(e, LEVEL + k);
            let ri = env_param(e, RATE + k);
            changed |= set(p, li, (get(p, li) - d.y / er.height() * 99.0).clamp(0.0, 99.0));
            changed |= set(p, ri, (get(p, ri) - d.x * 0.35).clamp(0.0, 99.0));
        }
        if resp.drag_stopped() {
            for i in [env_param(e, LEVEL + k), env_param(e, RATE + k)] {
                let v = get(p, i).round();
                changed |= set(p, i, v);
            }
        }
        if resp.secondary_clicked() {
            changed |= set(p, env_param(e, SUSTAIN), (k + 1) as f32);
        }
        if resp.double_clicked() {
            changed |= set(p, env_param(e, END), (k + 1) as f32);
        }
        if hot {
            let tip = format!("step {}  rate {:.0}  level {:.0}", k + 1, get(p, env_param(e, RATE + k)), get(p, env_param(e, LEVEL + k)));
            painter.text(pos2(er.left(), rect.bottom() - 4.0), Align2::LEFT_BOTTOM, tip, FontId::monospace(9.0), TEXT);
        }
    }
    if playing {
        let lv = live.vals[1 + e].clamp(0.0, 1.0);
        let y = er.bottom() - lv * er.height();
        painter.line_segment([pos2(er.left(), y), pos2(er.right(), y)], Stroke::new(1.0_f32, YELLOW.gamma_multiply(0.35)));
    }
    // The two lines' waves, bent as far as they are now.
    let line = get(p, LINE).round() as usize;
    for o in 0..2 {
        let wr = Rect::from_min_size(pos2(rect.right() - 118.0, rect.top() + 26.0 + o as f32 * ((rect.height() - 34.0) / 2.0)), vec2(108.0, (rect.height() - 42.0) / 2.0));
        painter.rect_filled(wr, 3.0, FAINT);
        let (wi, warp_i, env_i) = if o == 0 { (WAVE1_A, WARP1, ENV_WARP1) } else { (WAVE2_A, WARP2, ENV_WARP2) };
        let used = match line {
            0 | 2 => o == 0,
            1 => o == 1,
            _ => true,
        };
        let wave_k = get(p, wi).round().clamp(0.0, (WAVES.len() - 1) as f32) as usize;
        let env = if playing { live.vals[1 + env_i] } else { get(p, env_param(env_i, LEVEL)) / 99.0 };
        let d = get(p, warp_i) * env.clamp(0.0, 1.0);
        draw_wave(&painter, wr.shrink2(vec2(4.0, 6.0)), |x| wave(wave_k, (x * 2.0).fract(), d, 0.004), if used { accent } else { DIM });
        painter.text(wr.left_top() + vec2(4.0, 2.0), Align2::LEFT_TOP, format!("{} {}", o + 1, WAVES[wave_k].to_uppercase()), FontId::monospace(8.5), if used { TEXT } else { DIM });
        let resp = ui.interact(wr, id.with(("wave", o)), Sense::click());
        if resp.clicked() {
            changed |= set(p, wi, ((wave_k + 1) % WAVES.len()) as f32);
        } else if resp.secondary_clicked() {
            changed |= set(p, wi, ((wave_k + WAVES.len() - 1) % WAVES.len()) as f32);
        }
    }
    if playing {
        ui.ctx().request_repaint();
    }
    changed
}

/// A filter's response (0 to 1) at `f`, roughly, for the screen.
fn response(kind: usize, f: f32, fc: f32, reso: f32) -> f32 {
    let w = f / fc;
    let q = 0.7 + 6.0 * reso;
    let lp = 1.0 / ((1.0 - w * w).powi(2) + (w / q).powi(2)).sqrt();
    match kind {
        0 => 1.0,
        // four poles: the resonant two and two more
        1 => lp / (1.0 + w * w),
        2 => lp,
        3 => (w / q) * lp,
        4 => w * w * lp,
        5 => (1.0 - w * w).abs() * lp,
        6 => 1.0 / (1.0 - 0.9 * reso * (std::f32::consts::TAU * w).cos()).abs().max(0.05) * (1.0 - 0.9 * reso),
        7 => {
            let pos = ((fc / 200.0).ln() / (20.0f32).ln()).clamp(0.0, 1.0);
            let fs = [300.0 + 430.0 * pos, 870.0 + 1400.0 * pos, 2240.0 + 700.0 * pos];
            fs.iter().enumerate().map(|(i, &ff)| [1.0, 0.7, 0.4][i] / (1.0 + ((f / ff).ln() * (3.0 + 10.0 * reso)).powi(2))).sum()
        }
        _ => 0.5 * (1.0 + (4.0 * (w.ln() * 1.5).atan()).cos() * (0.4 + 0.5 * reso)),
    }
}

pub(crate) fn hybrid_screen(ui: &mut Ui, id: Id, p: &mut [f32], live: &InstLive, height: f32) -> bool {
    use hybrid::*;
    let w = 380.0;
    let (rect, _) = ui.allocate_exact_size(vec2(w, height), Sense::hover());
    let painter = frame(ui, rect);
    let accent = accent_now();
    let mut changed = false;
    caption(&painter, rect.left_top() + vec2(10.0, 6.0), Align2::LEFT_TOP, "Hybrid", accent);
    let playing = live.vals[6] > 0.0;
    let menv = if playing { live.vals[2] } else { 0.0 };
    let top = rect.top() + 22.0;
    let half_h = (rect.height() - 30.0) * 0.5;
    // The oscillators.
    for o in 0..2 {
        let (ei, si, li) = if o == 0 { (ENGINE1, SHAPE1, LEVEL1) } else { (ENGINE2, SHAPE2, LEVEL2) };
        let r = Rect::from_min_size(pos2(rect.left() + 8.0 + o as f32 * (w - 16.0) / 2.0, top), vec2((w - 16.0) / 2.0 - 4.0, half_h - 4.0));
        painter.rect_filled(r, 3.0, FAINT);
        let e = get(p, ei).round().clamp(0.0, (ENGINES.len() - 1) as f32) as usize;
        let shape = (get(p, si) + menv * get(p, M_SHAPE)).clamp(0.0, 1.0);
        let on = get(p, li) > 0.0;
        draw_wave(&painter, r.shrink2(vec2(6.0, 10.0)), |x| osc(e, shape, (x * 2.0).fract(), 0.003) * if e == 8 { (1.0 - x * 0.6).max(0.0) } else { 1.0 }, if on { accent } else { DIM });
        let mod_name = if o == 1 { MODS[get(p, MOD2).round().clamp(0.0, 3.0) as usize] } else { "Off" };
        let label = if o == 1 && mod_name != "Off" { format!("{} {}  {}", o + 1, ENGINES[e].to_uppercase(), mod_name.to_uppercase()) } else { format!("{} {}", o + 1, ENGINES[e].to_uppercase()) };
        painter.text(r.left_top() + vec2(4.0, 2.0), Align2::LEFT_TOP, label, FontId::monospace(8.5), if on { TEXT } else { DIM });
        let resp = ui.interact(r, id.with(("osc", o)), Sense::click());
        if resp.clicked() {
            changed |= set(p, ei, ((e + 1) % ENGINES.len()) as f32);
        } else if resp.secondary_clicked() {
            changed |= set(p, ei, ((e + ENGINES.len() - 1) % ENGINES.len()) as f32);
        }
    }
    // The filters.
    let fr = Rect::from_min_size(pos2(rect.left() + 8.0, top + half_h), vec2(w - 16.0, half_h - 4.0));
    painter.rect_filled(fr, 3.0, FAINT);
    let resp = ui.interact(fr, id.with("filter"), Sense::click_and_drag());
    if resp.dragged() {
        let d = resp.drag_delta();
        let c = (get(p, F1_CUTOFF) * 2f32.powf(d.x / fr.width() * 10.0)).clamp(20.0, 20000.0);
        changed |= set(p, F1_CUTOFF, c);
        changed |= set(p, F1_RESO, (get(p, F1_RESO) - d.y / fr.height()).clamp(0.0, 1.0));
    }
    let k1 = get(p, F1_TYPE).round().clamp(0.0, (FILTERS.len() - 1) as f32) as usize;
    if resp.clicked() {
        changed |= set(p, F1_TYPE, ((k1 + 1) % FILTERS.len()) as f32);
    } else if resp.secondary_clicked() {
        changed |= set(p, F1_TYPE, ((k1 + FILTERS.len() - 1) % FILTERS.len()) as f32);
    }
    let k2 = get(p, F2_TYPE).round().clamp(0.0, (FILTERS.len() - 1) as f32) as usize;
    let moved = |c: f32, env: f32| c * 2f32.powf(env * menv * 6.0);
    let curves = [(k1, moved(get(p, F1_CUTOFF), get(p, F1_ENV)), get(p, F1_RESO), accent), (k2, moved(get(p, F2_CUTOFF), get(p, F2_ENV)), get(p, F2_RESO), YELLOW)];
    for (k, fc, reso, color) in curves {
        if k == 0 && color == YELLOW {
            continue;
        }
        let n = 100;
        let pts: Vec<Pos2> = (0..=n)
            .map(|i| {
                let x = i as f32 / n as f32;
                let f = 20.0 * 1000f32.powf(x);
                let db = 20.0 * response(k, f, fc, reso).max(1e-4).log10();
                pos2(fr.left() + x * fr.width(), fr.center().y - (db / 36.0).clamp(-0.5, 0.45) * fr.height())
            })
            .collect();
        painter.add(Shape::line(pts, Stroke::new(1.5_f32, color)));
    }
    let routing = ROUTINGS[get(p, ROUTING).round().clamp(0.0, 1.0) as usize];
    let label = if k2 == 0 { FILTERS[k1].to_uppercase() } else { format!("{} + {}  {}", FILTERS[k1].to_uppercase(), FILTERS[k2].to_uppercase(), routing.to_uppercase()) };
    painter.text(fr.left_top() + vec2(4.0, 2.0), Align2::LEFT_TOP, label, FontId::monospace(8.5), TEXT);
    if playing {
        ui.ctx().request_repaint();
    }
    changed
}

pub(crate) fn quartet_screen(ui: &mut Ui, id: Id, p: &mut [f32], live: &InstLive, height: f32) -> bool {
    use quartet::*;
    let w = 470.0;
    let (rect, _) = ui.allocate_exact_size(vec2(w, height), Sense::hover());
    let painter = frame(ui, rect);
    let accent = accent_now();
    let mut changed = false;
    caption(&painter, rect.left_top() + vec2(10.0, 6.0), Align2::LEFT_TOP, "Quartet", accent);
    let playing = live.vals[7] > 0.0;
    let top = rect.top() + 22.0;
    let lane_h = ((rect.height() - 28.0) / 4.0).max(16.0);
    let lanes_w = 300.0;
    for o in 0..4 {
        let r = Rect::from_min_size(pos2(rect.left() + 8.0, top + o as f32 * lane_h), vec2(lanes_w, lane_h - 4.0));
        painter.rect_filled(r, 3.0, FAINT);
        let mode = get(p, osc_param(o, O_MODE)).round().clamp(0.0, (MODES.len() - 1) as f32) as usize;
        let n = waves_of(mode).max(1);
        let wave = get(p, osc_param(o, O_WAVE)).round().max(0.0) as usize % n;
        let param = if playing { live.vals[12 + o] } else { get(p, osc_param(o, O_PARAM)) };
        let level = if playing { live.vals[8 + o] } else { get(p, osc_param(o, O_LEVEL)) };
        let on = mode != 0 && level > 0.0;
        // The mode: click for the next.
        let mr = Rect::from_min_size(r.min, vec2(92.0, r.height()));
        let resp = ui.interact(mr, id.with(("mode", o)), Sense::click());
        painter.text(mr.left_top() + vec2(5.0, 3.0), Align2::LEFT_TOP, format!("{} {}", o + 1, MODES[mode].to_uppercase()), FontId::monospace(9.0), if resp.hovered() { TEXT } else if on { accent } else { DIM });
        if mode != 0 {
            painter.text(mr.left_bottom() + vec2(5.0, -3.0), Align2::LEFT_BOTTOM, wave_name(mode, wave), FontId::proportional(10.0), TEXT_DIM);
        }
        if resp.clicked() {
            changed |= set(p, osc_param(o, O_MODE), ((mode + 1) % MODES.len()) as f32);
        } else if resp.secondary_clicked() {
            changed |= set(p, osc_param(o, O_MODE), ((mode + MODES.len() - 1) % MODES.len()) as f32);
        }
        // The wave: click for the next.
        let wr = Rect::from_min_max(pos2(mr.right() + 4.0, r.top() + 4.0), pos2(r.right() - 34.0, r.bottom() - 4.0));
        if mode != 0 {
            draw_wave(&painter, wr, |x| preview(mode, wave, param, (x * 2.0).fract()), if on { accent } else { DIM });
            let resp = ui.interact(wr, id.with(("wave", o)), Sense::click());
            if resp.clicked() {
                changed |= set(p, osc_param(o, O_WAVE), ((wave + 1) % n) as f32);
            } else if resp.secondary_clicked() {
                changed |= set(p, osc_param(o, O_WAVE), ((wave + n - 1) % n) as f32);
            }
            if resp.hovered() {
                painter.rect_stroke(wr.expand(2.0), 2.0, Stroke::new(1.0_f32, DIM), egui::StrokeKind::Outside);
            }
        }
        // Its level, and which filter it goes to.
        let lr = Rect::from_min_max(pos2(r.right() - 28.0, r.top() + 4.0), pos2(r.right() - 22.0, r.bottom() - 4.0));
        painter.rect_filled(lr, 1.0, Color32::from_rgb(0x2a, 0x2d, 0x35));
        let lv = (level / 1.2).clamp(0.0, 1.0);
        painter.rect_filled(Rect::from_min_max(pos2(lr.left(), lr.bottom() - lv * lr.height()), lr.max), 1.0, if on { accent } else { DIM });
        let bus = get(p, osc_param(o, O_BUS));
        painter.text(pos2(r.right() - 4.0, r.center().y), Align2::RIGHT_CENTER, if bus < 0.34 { "A" } else if bus > 0.66 { "B" } else { "AB" }, FontId::monospace(8.5), TEXT_DIM);
    }
    // The matrix slots in use.
    let mx = rect.left() + lanes_w + 18.0;
    painter.text(pos2(mx, top), Align2::LEFT_TOP, "MATRIX", FontId::monospace(9.0), TEXT_DIM);
    let mut y = top + 13.0;
    let mut any = false;
    for s in 0..SLOTS {
        let src = get(p, slot_param(s, 0)).round().clamp(0.0, (SOURCES.len() - 1) as f32) as usize;
        let dest = get(p, slot_param(s, 1)).round().clamp(0.0, (DESTS.len() - 1) as f32) as usize;
        let amt = get(p, slot_param(s, 2));
        if src == 0 || dest == 0 || amt == 0.0 {
            continue;
        }
        any = true;
        if y < rect.bottom() - 46.0 {
            painter.text(pos2(mx, y), Align2::LEFT_TOP, format!("{} > {} {:+.0}%", SOURCES[src], DESTS[dest], amt * 100.0), FontId::proportional(10.0), TEXT);
            y += 13.0;
        }
    }
    if !any {
        painter.text(pos2(mx, y), Align2::LEFT_TOP, "nothing yet: set a Source,
a Dest and an Amount", FontId::proportional(10.0), DIM);
    }
    // The envelopes, as they move.
    let er = Rect::from_min_max(pos2(mx, rect.bottom() - 34.0), pos2(rect.right() - 10.0, rect.bottom() - 8.0));
    let bw = er.width() / ENVS as f32;
    for e in 0..ENVS {
        let b = Rect::from_min_size(pos2(er.left() + e as f32 * bw + 2.0, er.top()), vec2(bw - 4.0, er.height() - 10.0));
        painter.rect_filled(b, 1.0, FAINT);
        let lv = if playing { live.vals[1 + e].clamp(0.0, 1.0) } else { 0.0 };
        painter.rect_filled(Rect::from_min_max(pos2(b.left(), b.bottom() - lv * b.height()), b.max), 1.0, if e == 0 { accent } else { YELLOW.gamma_multiply(0.8) });
        painter.text(pos2(b.center().x, er.bottom()), Align2::CENTER_BOTTOM, if e == 0 { "A".to_string() } else { (e + 1).to_string() }, FontId::monospace(8.0), TEXT_DIM);
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

    fn show(f: impl Fn(&mut Ui, &InstLive)) {
        let ctx = egui::Context::default();
        let mut vals = [0.0f32; krabton_engine::engine::INST_VALS];
        for (i, v) in vals.iter_mut().enumerate() {
            *v = (i as f32 * 0.37).fract();
        }
        vals[6] = 2.0;
        vals[11] = 1.0;
        let live = InstLive { wave: vec![0.0; 256], notes: 1 << 60, vals };
        for _ in 0..3 {
            let _ = ctx.run(RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1200.0, 800.0))), ..Default::default() }, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| f(ui, &live));
            });
        }
    }

    #[test]
    fn warp_and_hybrid_screens_draw() {
        for name in warp::preset_names() {
            show(|ui, live| {
                let mut p = warp::preset(name);
                warp_screen(ui, Id::new("w"), &mut p, live, 200.0);
            });
        }
        for name in hybrid::preset_names() {
            show(|ui, live| {
                let mut p = hybrid::preset(name);
                hybrid_screen(ui, Id::new("h"), &mut p, live, 200.0);
            });
        }
        for name in quartet::preset_names() {
            show(|ui, live| {
                let mut p = quartet::preset(name);
                quartet_screen(ui, Id::new("q"), &mut p, live, 200.0);
            });
        }
    }

    #[test]
    fn every_filter_curve_is_drawable() {
        for k in 0..hybrid::FILTERS.len() {
            for i in 0..50 {
                let f = 20.0 * 1000f32.powf(i as f32 / 49.0);
                let r = response(k, f, 1000.0, 0.7);
                assert!(r.is_finite() && r >= 0.0, "{k} {f} {r}");
            }
        }
    }
}

//! The mechanical sequencers' screens: the Organ Grinder's book (folded
//! cardboard punched with the holes it plays, sliding past the reading
//! bar) and the Music Box's cylinder (its pins turning towards the comb).
//! Both draw the bars the engine makes from the same parameters, so the
//! holes and pins are the notes to come. And both turn: grab the crank and
//! turn it (the book goes as fast as you do; let go and it runs on at the
//! Crank speed, or stops when the song does), or wind the box's key.

use super::seq_view::Live;
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, Id, Pos2, Rect, Sense, Stroke, Ui};
use krabton_engine::sequencers::mechanical::{self as mech, Hole, Lane};
use std::f32::consts::TAU;

fn choice(params: &[f32], i: usize, n: usize) -> usize {
    (params[i].round().max(0.0) as usize).min(n - 1)
}

/// Where the engine has the book or the cylinder: (bar, tick in it), or the start.
fn now(live: &Live, tpb: u16) -> (i64, f32) {
    match live.vis {
        Some(v) if v[3] > 0 || live.playing => {
            let pos = v[3] as f64 / 16.0;
            let bar = (pos / tpb as f64).floor() as i64;
            (bar, (pos - bar as f64 * tpb as f64) as f32)
        }
        _ => (0, 0.0),
    }
}

/// How fast the engine turns it now (1: the song's tempo, backwards below 0) and how wound its spring is.
fn speed_tension(live: &Live) -> (f32, f32) {
    match live.vis {
        Some(v) => {
            let s = ((v[0] >> 40) & 0xff) as f32 / 100.0;
            (if (v[0] >> 56) & 1 == 1 { -s } else { s }, ((v[0] >> 32) & 0xff) as f32 / 255.0)
        }
        None => (0.0, 0.0),
    }
}

/// What a hand on the crank did this frame.
#[derive(Default)]
struct Turned {
    /// Turns a second (smoothed; backwards below 0) while held; 0 the moment it is let go.
    speed: Option<f32>,
    /// Let go after a turn and a half or more: how its speed went through a turn.
    learned: Option<[f32; mech::FEEL_BINS]>,
}

/// What the crank learns while you turn it: the speed in each part of the
/// turn, how many frames went into each, how far it has gone, where it began.
#[derive(Clone, Copy, Default)]
struct Learn {
    sum: [f32; mech::FEEL_BINS],
    count: [f32; mech::FEEL_BINS],
    travel: f32,
    phase0: f32,
}

impl Learn {
    /// The speed through a turn, each part against the average (empty parts
    /// as the average), smoothed a little.
    fn feel(&self) -> [f32; mech::FEEL_BINS] {
        let n = mech::FEEL_BINS;
        let total: f32 = self.sum.iter().sum::<f32>() / self.count.iter().sum::<f32>().max(1.0);
        let mut raw = [total; mech::FEEL_BINS];
        for i in 0..n {
            if self.count[i] > 0.0 {
                raw[i] = self.sum[i] / self.count[i];
            }
        }
        let mut out = [1.0f32; mech::FEEL_BINS];
        for i in 0..n {
            let v = 0.25 * raw[(i + n - 1) % n] + 0.5 * raw[i] + 0.25 * raw[(i + 1) % n];
            out[i] = v / total.max(1e-3);
        }
        out
    }
}

/// A crank wheel: drawn at the turn `angle`, the feel it is turned with as
/// a ring inside it (wider where it goes quicker). Dragged round, it reports
/// how fast it is turned, and when let go after a turn and a half how your
/// hand went through a turn.
fn crank(ui: &mut Ui, id: Id, r: Rect, angle: f32, feel: &mech::Feel, accent: Color32) -> Turned {
    let resp = ui.interact(r, id, Sense::drag()).on_hover_text("Grab the handle and turn: the music goes as fast as you turn (the other way round, backwards). Turn it a few times and let go: it keeps your way of turning (Feel: Yours)");
    let c = r.center();
    let rad = r.width().min(r.height()) * 0.42;
    let p = ui.painter();
    let iron = Color32::from_rgb(0x3a, 0x3d, 0x44);
    let gold = Color32::from_rgb(0xd8, 0xb0, 0x4a);
    let mem = id.with("crank");
    let learn_id = id.with("learn");
    let mut out = Turned::default();
    let mut shown = angle;
    if resp.drag_started() {
        ui.data_mut(|d| d.insert_temp(learn_id, Learn { phase0: (angle / TAU).rem_euclid(1.0), ..Default::default() }));
    }
    if resp.dragged() {
        if let Some(pt) = ui.input(|i| i.pointer.interact_pos()) {
            let a = (pt.y - c.y).atan2(pt.x - c.x);
            let t = ui.input(|i| i.time);
            let (last_a, last_t, speed) = ui.data(|d| d.get_temp::<(f32, f64, f32)>(mem)).unwrap_or((a, t, 0.0));
            let mut da = a - last_a;
            if da > TAU / 2.0 {
                da -= TAU;
            } else if da < -TAU / 2.0 {
                da += TAU;
            }
            let dt = (t - last_t).max(1e-3) as f32;
            // Clockwise turns it forward, the other way backwards; a still hand holds it.
            let measured = da / TAU / dt;
            let speed = speed + (measured - speed) * (dt / 0.12).min(1.0);
            ui.data_mut(|d| d.insert_temp(mem, (a, t, speed)));
            // Where in the turn the hand is, and how fast it goes there.
            let mut l = ui.data(|d| d.get_temp::<Learn>(learn_id)).unwrap_or_default();
            if da != 0.0 {
                let bin = (((l.phase0 + l.travel).rem_euclid(1.0)) * mech::FEEL_BINS as f32) as usize % mech::FEEL_BINS;
                l.sum[bin] += measured.abs();
                l.count[bin] += 1.0;
                l.travel += da / TAU;
            }
            ui.data_mut(|d| d.insert_temp(learn_id, l));
            shown = a;
            out.speed = Some(speed);
        }
    } else if resp.drag_stopped() {
        ui.data_mut(|d| d.remove::<(f32, f64, f32)>(mem));
        if let Some(l) = ui.data(|d| d.get_temp::<Learn>(learn_id)) {
            if l.travel.abs() >= 1.5 {
                out.learned = Some(l.feel());
            }
        }
        ui.data_mut(|d| d.remove::<Learn>(learn_id));
        out.speed = Some(0.0);
    }
    // The feel: a ring, wider where the hand goes quicker (the turn starts on the right).
    let pts: Vec<Pos2> = (0..=64)
        .map(|k| {
            let ph = k as f64 / 64.0;
            let rr = rad * 0.62 * (1.0 + 0.45 * (feel.speed(ph, 0) as f32 - 1.0)).clamp(0.4, 1.5);
            let a = TAU * ph as f32;
            c + vec2(a.cos(), a.sin()) * rr
        })
        .collect();
    p.add(egui::Shape::line(pts, Stroke::new(1.5_f32, if feel.learned() { accent } else { accent.gamma_multiply(0.45) })));
    // The wheel: its rim, four spokes turning, the handle on the rim.
    p.circle_stroke(c, rad, Stroke::new(3.0_f32, iron));
    for k in 0..4 {
        let a = shown + k as f32 * TAU / 4.0;
        p.line_segment([c, c + vec2(a.cos(), a.sin()) * rad], Stroke::new(2.0_f32, iron));
    }
    p.circle_filled(c, rad * 0.16, gold);
    let h: Pos2 = c + vec2(shown.cos(), shown.sin()) * rad;
    p.circle_filled(h, rad * 0.22, if resp.dragged() || resp.hovered() { accent } else { gold });
    p.circle_stroke(h, rad * 0.22, Stroke::new(1.0_f32, Color32::from_black_alpha(160)));
    out
}

/// How many beats a turn of the crank is (for how fast a hand turns at the song's tempo).
fn turn_beats(params: &[f32], organ: bool) -> f32 {
    ((krabton_engine::sequencers::rate_beats(params[0]) / mech::TICKS) * mech::ticks_per_turn(params, organ)) as f32
}

/// What the screen does with a hand on the crank: its speed into Turning
/// (taking hold makes the crank the drive), its learned turn into the feel.
#[allow(clippy::too_many_arguments)]
fn take(turned: Turned, params: &mut [f32], data: &mut Vec<u8>, live: &Live, organ: bool, turning: usize, drive: usize, crank_drive: f32, feel: usize) -> bool {
    let mut changed = false;
    if let Some(turns) = turned.speed {
        let let_go = turns == 0.0;
        if !let_go || params[turning] != 0.0 {
            let nominal = (live.bpm as f32 / 60.0) / turn_beats(params, organ).max(0.05);
            // At least a crawl while held (a still hand holds it), backwards below 0.
            let s = (turns.abs() / nominal.max(0.01)).clamp(0.02, 4.0);
            params[turning] = if let_go { 0.0 } else if turns < 0.0 { -s } else { s };
            if !let_go && params[drive] < 0.5 {
                params[drive] = crank_drive;
            }
            changed = true;
        }
    }
    if let Some(bins) = turned.learned {
        *data = mech::encode_feel(&bins);
        params[feel] = 1.0;
        changed = true;
    }
    changed
}

/// The book: lanes for the drums, the bass, the accompaniment and the tune
/// (counter melody paler), a fold at every bar, the reading bar a quarter
/// in; and the crank that pulls it through.
pub(crate) fn organ(ui: &mut Ui, id: Id, params: &mut [f32], data: &mut Vec<u8>, live: Live, accent: Color32, height: f32) -> bool {
    let (rect, _) = ui.allocate_exact_size(vec2(370.0, height), Sense::hover());
    let p = ui.painter().clone();
    let card = Color32::from_rgb(0xd9, 0xc3, 0x93);
    let ink = Color32::from_rgb(0x3a, 0x2a, 0x18);
    rect_fill(&p, rect, 3.0, KNOB);
    let book = Rect::from_min_max(pos2(rect.left() + 6.0, rect.top() + 6.0), pos2(rect.right() - 76.0, rect.bottom() - 20.0));
    p.rect_filled(book, 2.0, card);
    let tpb = mech::organ_ticks_per_bar(params);
    let (bar, tick) = now(&live, tpb);
    let px = book.width() / (tpb as f32 * 2.6);
    let read_x = book.left() + book.width() * 0.22;
    let x_of = |b: i64, t: f32| read_x + ((b - bar) as f32 * tpb as f32 + t - tick) * px;
    let lanes = [(Lane::Drum, 0.0, 0.14), (Lane::Bass, 0.14, 0.32), (Lane::Accomp, 0.32, 0.55), (Lane::Melody, 0.55, 1.0)];
    let band = |lane: Lane| lanes.iter().find(|l| l.0 == if lane == Lane::Counter { Lane::Melody } else { lane }).map(|l| (l.1, l.2)).unwrap_or((0.0, 1.0));
    for (_, lo, _) in lanes.iter().skip(1) {
        let y = book.bottom() - lo * book.height();
        p.line_segment([pos2(book.left(), y), pos2(book.right(), y)], Stroke::new(1.0_f32, ink.gamma_multiply(0.25)));
    }
    let clip = p.with_clip_rect(book);
    let running = live.vis.is_some() && (live.playing || speed_tension(&live).0 != 0.0);
    let mut holes: Vec<Hole> = Vec::with_capacity(160);
    for b in bar..bar + 3 {
        let fx = x_of(b, 0.0);
        clip.line_segment([pos2(fx, book.top()), pos2(fx, book.bottom())], Stroke::new(1.0_f32, ink.gamma_multiply(0.4)));
        mech::organ_bar(params, b, &mut holes);
        for h in &holes {
            let (lo, hi) = band(h.lane);
            let (dlo, dhi) = match h.lane {
                Lane::Drum => (23.0, 30.0),
                Lane::Bass => (-5.0, 8.0),
                Lane::Accomp => (-1.0, 11.0),
                _ => (-8.0, 13.0),
            };
            let f = ((h.deg as f32 - dlo) / (dhi - dlo)).clamp(0.0, 1.0);
            let y = book.bottom() - (lo + (hi - lo) * (0.1 + 0.8 * f)) * book.height();
            let x0 = x_of(b, h.at as f32);
            let x1 = x_of(b, (h.at + h.len) as f32) - 1.0;
            let c = if running && x0 <= read_x && x1 >= read_x { accent } else if h.lane == Lane::Counter { ink.gamma_multiply(0.5) } else { ink };
            clip.rect_filled(Rect::from_min_max(pos2(x0, y - 2.0), pos2(x1.max(x0 + 2.0), y + 2.0)), 2.0, c);
        }
    }
    p.line_segment([pos2(read_x, book.top() - 2.0), pos2(read_x, book.bottom() + 2.0)], Stroke::new(2.0_f32, accent));
    // The crank: a turn a bar, half a bar or a beat.
    let wheel = Rect::from_min_max(pos2(book.right() + 6.0, rect.top() + 6.0), pos2(rect.right() - 6.0, rect.bottom() - 20.0));
    let pos = bar as f64 * tpb as f64 + tick as f64;
    let angle = TAU * (pos / mech::ticks_per_turn(params, true)).rem_euclid(1.0) as f32;
    let feel = mech::Feel::of(params, data, true);
    let turned = crank(ui, id.with("organ_crank"), wheel, angle, &feel, accent);
    let changed = take(turned, params, data, &live, true, mech::OG_TURNING, mech::OG_DRIVE, 1.0, mech::OG_FEEL);
    let style = mech::ORGAN_STYLES[choice(params, mech::OG_STYLE, mech::ORGAN_STYLES.len())];
    let tune = params[mech::OG_TUNE].round() as i32;
    let drive = mech::ORGAN_DRIVES[choice(params, mech::OG_DRIVE, mech::ORGAN_DRIVES.len())];
    text(&p, pos2(rect.left() + 8.0, rect.bottom() - 4.0), Align2::LEFT_BOTTOM, &format!("{style} · book {tune} · {drive}"), small(), TEXT_DIM);
    let (speed, _) = speed_tension(&live);
    let speed_text = if drive == "Crank" { format!("{:.0}%", speed * 100.0) } else { format!("bar {}", bar.rem_euclid(32) + 1) };
    text(&p, pos2(rect.right() - 8.0, rect.bottom() - 4.0), Align2::RIGHT_BOTTOM, &speed_text, small(), TEXT_DIM);
    if running || changed {
        ui.ctx().request_repaint();
    }
    changed
}

/// A winding key: dragged round (clockwise or not) it winds the spring.
fn key(ui: &mut Ui, id: Id, r: Rect, tension: f32, accent: Color32) -> Option<f32> {
    let resp = ui.interact(r, id, Sense::drag()).on_hover_text("Wind the spring: turn the key round, four turns and it is wound");
    let c = r.center() - vec2(0.0, 6.0);
    let rad = r.width().min(r.height()) * 0.36;
    let p = ui.painter();
    let mem = id.with("key");
    let angle = ui.data(|d| d.get_temp::<f32>(mem.with("shown"))).unwrap_or(0.0);
    let mut out = None;
    let mut shown = angle;
    if resp.dragged() {
        if let Some(pt) = ui.input(|i| i.pointer.interact_pos()) {
            let a = (pt.y - c.y).atan2(pt.x - c.x);
            let last = ui.data(|d| d.get_temp::<f32>(mem)).unwrap_or(a);
            let mut da = a - last;
            if da > TAU / 2.0 {
                da -= TAU;
            } else if da < -TAU / 2.0 {
                da += TAU;
            }
            ui.data_mut(|d| d.insert_temp(mem, a));
            shown = angle + da;
            out = Some(da.abs() / TAU);
        }
    } else if resp.drag_stopped() {
        ui.data_mut(|d| d.remove::<f32>(mem));
    }
    ui.data_mut(|d| d.insert_temp(mem.with("shown"), shown));
    // A butterfly key on its stem.
    let brass = Color32::from_rgb(0xc9, 0xa2, 0x4c);
    let dir = vec2(shown.cos(), shown.sin());
    let side = vec2(-dir.y, dir.x);
    for s in [-1.0f32, 1.0] {
        let tip = c + dir * rad * s;
        p.add(egui::Shape::convex_polygon(vec![c + side * rad * 0.12, tip + side * rad * 0.45, tip - side * rad * 0.45, c - side * rad * 0.12], if resp.dragged() || resp.hovered() { accent } else { brass }, Stroke::new(1.0_f32, Color32::from_black_alpha(140))));
    }
    p.circle_filled(c, rad * 0.18, Color32::from_rgb(0x5a, 0x45, 0x1c));
    // How wound the spring is.
    let gauge = Rect::from_min_size(pos2(r.left() + 4.0, r.bottom() - 9.0), vec2(r.width() - 8.0, 5.0));
    p.rect_filled(gauge, 2.0, PANEL3);
    p.rect_filled(Rect::from_min_size(gauge.min, vec2(gauge.width() * tension.clamp(0.0, 1.0), gauge.height())), 2.0, accent);
    out
}

/// The cylinder: its pins coming round towards the comb along the bottom,
/// a tooth for each note the comb has, lit as it is plucked; and at its
/// side the crank, or the spring's key.
pub(crate) fn music_box(ui: &mut Ui, id: Id, params: &mut [f32], data: &mut Vec<u8>, live: Live, accent: Color32, height: f32) -> bool {
    let (rect, _) = ui.allocate_exact_size(vec2(340.0, height), Sense::hover());
    let p = ui.painter().clone();
    rect_fill(&p, rect, 3.0, KNOB);
    let brass = Color32::from_rgb(0xc9, 0xa2, 0x4c);
    let steel = Color32::from_rgb(0xc8, 0xcc, 0xd2);
    let comb_h = 18.0;
    let cyl = Rect::from_min_max(pos2(rect.left() + 10.0, rect.top() + 8.0), pos2(rect.right() - 74.0, rect.bottom() - 22.0 - comb_h));
    for i in 0..6 {
        let f = i as f32 / 6.0;
        let band = Rect::from_min_max(pos2(cyl.left(), cyl.top() + f * cyl.height()), pos2(cyl.right(), cyl.top() + (f + 1.0 / 6.0) * cyl.height()));
        let light = 1.0 - (f + 1.0 / 12.0 - 0.45).abs() * 1.4;
        p.rect_filled(band, 0.0, mix(Color32::from_rgb(0x5a, 0x45, 0x1c), brass, light.clamp(0.2, 1.0)));
    }
    let comb = choice(params, mech::MB_COMB, mech::BOX_COMBS.len());
    let (lo, hi) = [(-7, 10), (-11, 17), (-14, 21)][comb];
    let teeth = (hi - lo + 1) as usize;
    let tw = cyl.width() / teeth as f32;
    let tpb = mech::box_ticks_per_bar(params);
    let (bar, tick) = now(&live, tpb);
    let (speed, tension) = speed_tension(&live);
    let running = live.vis.is_some() && (live.playing || speed != 0.0);
    let py = cyl.height() / (tpb as f32 * 2.0);
    let mut holes: Vec<Hole> = Vec::with_capacity(160);
    let mut plucked = vec![0.0f32; teeth];
    let clip = p.with_clip_rect(cyl);
    for b in bar..bar + 3 {
        mech::box_bar(params, b, &mut holes);
        for h in &holes {
            let col = (h.deg - lo).clamp(0, teeth as i32 - 1) as usize;
            let dt = (b - bar) as f32 * tpb as f32 + h.at as f32 - tick;
            if running && (-2.0..0.5).contains(&dt) {
                plucked[col] = plucked[col].max(1.0 - (-dt) / 2.0);
            }
            let y = cyl.bottom() - dt * py;
            let x = cyl.left() + (col as f32 + 0.5) * tw;
            let r = if h.lane == Lane::Melody { 2.2 } else { 1.7 };
            clip.circle_filled(pos2(x, y), r, if dt < 0.5 && running { accent } else { Color32::from_rgb(0x2a, 0x22, 0x14) });
        }
    }
    let top = cyl.bottom() + 2.0;
    for (i, &pl) in plucked.iter().enumerate() {
        let x = cyl.left() + (i as f32 + 0.5) * tw;
        let len = comb_h * (1.0 - 0.45 * i as f32 / teeth as f32);
        p.line_segment([pos2(x, top), pos2(x, top + len)], Stroke::new((tw * 0.55).clamp(1.0, 5.0), mix(steel, accent, pl.clamp(0.0, 1.0))));
    }
    // At its side: the spring's key, or the crank.
    let drive = choice(params, mech::MB_DRIVE, mech::BOX_DRIVES.len());
    let side = Rect::from_min_max(pos2(cyl.right() + 6.0, rect.top() + 6.0), pos2(rect.right() - 6.0, rect.bottom() - 20.0));
    let mut changed = false;
    if drive == 1 {
        if let Some(turns) = key(ui, id.with("box_key"), side, tension, accent) {
            if turns > 0.0 {
                params[mech::MB_WOUND] = (params[mech::MB_WOUND] + turns).min(1_000_000.0);
                changed = true;
            }
        }
    } else {
        let pos = bar as f64 * tpb as f64 + tick as f64;
        let angle = TAU * (pos / mech::ticks_per_turn(params, false)).rem_euclid(1.0) as f32;
        let feel = mech::Feel::of(params, data, false);
        let turned = crank(ui, id.with("box_crank"), side, angle, &feel, accent);
        // Taking hold in the song's time makes the crank the box's drive.
        let crank_drive = if drive == 0 { 2.0 } else { params[mech::MB_DRIVE] };
        changed |= take(turned, params, data, &live, false, mech::MB_TURNING, mech::MB_DRIVE, crank_drive, mech::MB_FEEL);
    }
    let metre = mech::BOX_METRES[choice(params, mech::MB_METRE, mech::BOX_METRES.len())];
    let tune = params[mech::MB_TUNE].round() as i32;
    let drive_name = mech::BOX_DRIVES[drive];
    text(&p, pos2(rect.left() + 8.0, rect.bottom() - 4.0), Align2::LEFT_BOTTOM, &format!("{metre} · tune {tune} · {}", mech::BOX_COMBS[comb]), small(), TEXT_DIM);
    text(&p, pos2(rect.right() - 8.0, rect.bottom() - 4.0), Align2::RIGHT_BOTTOM, drive_name, small(), TEXT_DIM);
    if running || changed {
        ui.ctx().request_repaint();
    }
    changed
}

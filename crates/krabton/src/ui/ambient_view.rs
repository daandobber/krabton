//! Editors for the ambient sequencers: Loops (tape loops as rings), Bloom
//! (ripples on a pond), Chimes (tubes in the wind), Drift Chords (voices as
//! lines through time), Tintinnabuli (melody and triad voice on a staff) and
//! Pedal (the held note, big). Like the others they show what the engine
//! reports and edit the device's params.

use super::seq_more_view::{chip, degree_name, drag_steps, trail};
use super::seq_view::Live;
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, FontId, Id, Rect, Sense, Shape, Stroke, Ui};
use krabton_engine::sequencers::ambient;
use std::f32::consts::{FRAC_PI_2, TAU};

fn voice_color(i: usize) -> Color32 {
    [
        ACCENT,
        SELECT,
        PLAY,
        Color32::from_rgb(0xe0, 0x84, 0xff),
        Color32::from_rgb(0xff, 0x7a, 0x9c),
        Color32::from_rgb(0x5e, 0xe0, 0xd0),
        Color32::from_rgb(0xf0, 0xe0, 0x6a),
    ][i % 7]
}

fn dice_seed() -> u32 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.subsec_nanos()).unwrap_or(1) | 1
}

fn rnd(seed: &mut u32) -> f32 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 17;
    *seed ^= *seed << 5;
    (*seed >> 8) as f32 / 16_777_216.0
}

// ---------------------------------------------------------------- loops

/// One ring per loop, the play head at the top: a note sounds as its dot
/// passes it. Next to it each loop's length (in beats) and its note, both
/// dragged up or down.
pub(crate) fn loops(ui: &mut Ui, id: Id, params: &mut [f32], live: Live, height: f32) -> bool {
    let n = (params[0].round() as usize).clamp(1, 7);
    let side = height.min(170.0);
    let list_w = 96.0;
    let (rect, _) = ui.allocate_exact_size(vec2(side + list_w + 4.0, height), Sense::hover());
    let p = ui.painter().clone();
    rect_fill(&p, rect, 3.0, KNOB);
    let c = pos2(rect.left() + side * 0.5, rect.top() + height * 0.5);
    let r_max = side * 0.5 - 8.0;
    let r_min = r_max * 0.28;
    let vis = live.on();
    p.line_segment([pos2(c.x, c.y - r_max - 6.0), pos2(c.x, c.y - r_min + 6.0)], Stroke::new(1.0_f32, LINE));
    let speed = params[1].clamp(0.1, 8.0);
    for i in 0..n {
        let r = if n == 1 { r_max } else { r_min + (r_max - r_min) * i as f32 / (n - 1) as f32 };
        let col = voice_color(i);
        p.circle_stroke(c, r, Stroke::new(1.0_f32, col.gamma_multiply(0.3)));
        let period = params[6 + i].round().max(1.0) / speed;
        let phase = match vis {
            Some(v) => ((v[0] >> (i * 8)) & 0xff) as f32 / 255.0,
            None => (1.0 - ((i as f32 + 1.0) * 0.618_034).fract()).fract(),
        };
        let at = |ph: f32| {
            let a = -FRAC_PI_2 + ph * TAU;
            c + vec2(a.cos(), a.sin()) * r
        };
        // The tape just behind the dot, then the dot, bright just after it played.
        let pts: Vec<_> = (0..=10).map(|k| at(phase - 0.12 * k as f32 / 10.0)).collect();
        p.add(Shape::line(pts, Stroke::new(2.0_f32, col.gamma_multiply(0.35))));
        let flash = if vis.is_some() { (1.0 - phase * period / 1.5).max(0.0) } else { 0.0 };
        p.circle_filled(at(phase), 3.0 + flash * 3.0, mix(col, Color32::WHITE, flash));
        if flash > 0.0 {
            p.circle_stroke(at(0.0), 4.0 + (1.0 - flash) * 10.0, Stroke::new(1.0_f32, col.gamma_multiply(flash)));
        }
    }
    // The list: length and note per loop.
    let mut changed = false;
    let lx = rect.left() + side + 2.0;
    let rh = ((height - 26.0) / 7.0).clamp(12.0, 17.0);
    text(&p, pos2(lx + 12.0, rect.top() + 3.0), Align2::LEFT_TOP, "beats", small(), TEXT_DIM);
    text(&p, pos2(lx + 52.0, rect.top() + 3.0), Align2::LEFT_TOP, "note", small(), TEXT_DIM);
    for i in 0..n {
        let y = rect.top() + 16.0 + i as f32 * rh;
        p.circle_filled(pos2(lx + 5.0, y + rh * 0.5), 3.0, voice_color(i));
        let lr = Rect::from_min_size(pos2(lx + 12.0, y + 1.0), vec2(36.0, rh - 2.0));
        let label = format!("{}", params[6 + i].round());
        changed |= chip(ui, lr, id.with(("len", i)), &label, voice_color(i), &mut params[6 + i], 3.0, 64.0, "Drag up or down: this loop's length in beats");
        let nr = Rect::from_min_size(pos2(lx + 52.0, y + 1.0), vec2(40.0, rh - 2.0));
        let name = degree_name(&live, params[3], params[13 + i].round() as i32);
        changed |= chip(ui, nr, id.with(("note", i)), &name, TEXT, &mut params[13 + i], -14.0, 14.0, "Drag up or down: this loop's note, in the scale");
    }
    let dr = Rect::from_min_size(pos2(lx + 12.0, rect.bottom() - 17.0), vec2(80.0, 14.0));
    if button(ui, dr, id.with("dice"), "dice", false, ACCENT).on_hover_text("New notes and new loop lengths: the loops drift apart in another way").clicked() {
        let mut seed = dice_seed();
        // Lengths that rarely meet, notes that mostly fit the chord.
        const LENGTHS: [f32; 12] = [11.0, 13.0, 17.0, 19.0, 21.0, 23.0, 25.0, 29.0, 31.0, 34.0, 37.0, 43.0];
        for i in 0..7 {
            params[6 + i] = LENGTHS[(rnd(&mut seed) * 12.0) as usize % 12];
            let octave = [-1.0, 0.0, 0.0, 1.0][(rnd(&mut seed) * 4.0) as usize % 4];
            let degree = if rnd(&mut seed) < 0.7 { [0.0, 2.0, 4.0][(rnd(&mut seed) * 3.0) as usize % 3] } else { (rnd(&mut seed) * 7.0).floor() };
            params[13 + i] = (octave * 7.0 + degree).clamp(-14.0, 14.0);
        }
        changed = true;
    }
    if vis.is_some() {
        ui.ctx().request_repaint();
    }
    changed
}

// ---------------------------------------------------------------- bloom

/// How many bytes were newly shifted into the low end of `v` since the last
/// frame (none the first time: what was there before is not news).
fn fresh_count(ui: &Ui, id: Id, v: u64) -> usize {
    let Some(last) = ui.ctx().memory(|m| m.data.get_temp::<u64>(id)) else {
        ui.ctx().memory_mut(|m| m.data.insert_temp(id, v));
        return 0;
    };
    if v == last {
        return 0;
    }
    ui.ctx().memory_mut(|m| m.data.insert_temp(id, v));
    (1..8).find(|&k| v >> (8 * k) == last & (u64::MAX >> (8 * k))).unwrap_or(8)
}

/// A pond, as in Eno's Bloom: click it to drop a note (the higher, the
/// higher the note) and watch it ripple out again with every echo. In Loop
/// the notes stay as dots and keep coming round (click a dot to take it
/// away); in Fade Away they echo softer and softer until they are gone.
/// Left alone for two bars it blooms by itself, around your notes. It plays
/// with the transport stopped too.
pub(crate) fn bloom(ui: &mut Ui, id: Id, params: &[f32], data: &mut Vec<u8>, live: Live, accent: Color32, height: f32) -> bool {
    let w = 270.0;
    let (rect, resp) = ui.allocate_exact_size(vec2(w, height), Sense::click());
    let p = ui.painter().clone();
    rect_fill(&p, rect, 3.0, KNOB);
    let now = ui.input(|i| i.time);
    let range = params[5].round().max(2.0) as i32;
    let decay = params[3].clamp(0.1, 0.99);
    let pond = Rect::from_min_max(rect.min + vec2(6.0, 6.0), rect.max - vec2(6.0, 20.0));
    rect_fill(&p, pond, 10.0, mix(KNOB, SELECT, 0.05));
    let (lo_y, hi_y) = (pond.bottom() - 9.0, pond.top() + 9.0);
    let y_of = |d: i32| lo_y - (d as f32 / range as f32).clamp(0.0, 1.0) * (lo_y - hi_y);
    // The tonic in every octave, faintly, so you know where you are.
    for d in (0..=range).filter(|d| d % 7 == 0) {
        let y = y_of(d);
        let mut x = pond.left() + 22.0;
        while x < pond.right() - 6.0 {
            p.circle_filled(pos2(x, y), 0.8, Color32::from_white_alpha(28));
            x += 9.0;
        }
        text(&p, pos2(pond.left() + 4.0, y), Align2::LEFT_CENTER, degree_name(&live, params[6], d), small(), Color32::from_white_alpha(50));
    }
    // Where you dropped notes: (degree, across 0..1, time).
    let taps_id = id.with("taps");
    let mut taps: Vec<(i32, f32, f64)> = ui.ctx().memory(|m| m.data.get_temp(taps_id).unwrap_or_default());
    let mut changed = false;
    let looping = params.get(ambient::TAPS).is_some_and(|v| *v >= 0.5);
    let spot = |d: i32, across: f32| pos2(pond.left() + 10.0 + across * (pond.width() - 20.0), y_of(d));
    let tap_spot = |t: &ambient::Tap| spot(ambient::bloom_degree(t.height, range), t.across as f32 / 255.0);
    // The dot under the pointer, if any (Loop: clicking it takes it away).
    let hovered_dot = resp.hover_pos().filter(|_| looping).and_then(|pos| ambient::bloom_taps(data).position(|t| tap_spot(&t).distance(pos) < 7.0));
    if let Some(pos) = resp.interact_pointer_pos().filter(|pos| resp.clicked() && pond.contains(*pos)) {
        if let Some(i) = hovered_dot {
            ambient::bloom_remove_tap(data, i);
        } else {
            let up = ((lo_y - pos.y) / (lo_y - hi_y)).clamp(0.0, 1.0);
            let across = ((pos.x - pond.left()) / pond.width()).clamp(0.0, 1.0);
            let h = (up * 255.0).round() as u8;
            // Stamped with where the loop is now, so it keeps that place.
            let phase = live.vis.map(|v| (v[3] & 0xffff) as u16).unwrap_or(0);
            ambient::bloom_add_tap(data, h, (across * 255.0).round() as u8, phase);
            taps.push((ambient::bloom_degree(h, range), across, now));
        }
        changed = true;
    }
    taps.retain(|t| now - t.2 < 120.0);
    if taps.len() > 32 {
        taps.drain(..taps.len() - 32);
    }
    // Ripples: every note the engine plays, where its tap fell (or where
    // Bloom dropped it by itself).
    let rip_id = id.with("ripples");
    let mut ripples: Vec<(i32, f32, f32, f64)> = ui.ctx().memory(|m| m.data.get_temp(rip_id).unwrap_or_default());
    if let Some(v) = live.vis {
        for k in (0..fresh_count(ui, id.with("last"), v[0])).rev() {
            let d = ((v[0] >> (8 * k)) & 0xff) as i32 - 64;
            let across = ((v[2] >> (8 * k)) & 0xff) as f32 / 255.0;
            // An echo of a note still rippling is fainter than it.
            let strength = ripples.iter().rev().find(|r| r.0 == d && now - r.3 < 8.0).map(|r| r.2 * decay).unwrap_or(1.0);
            ripples.push((d, across, strength, now));
        }
    }
    ripples.retain(|r| now - r.3 < 3.0);
    if ripples.len() > 48 {
        ripples.drain(..ripples.len() - 48);
    }
    // In Loop, your notes stay on the pond as dots, lit as they sound.
    if looping {
        for (i, t) in ambient::bloom_taps(data).enumerate() {
            let at = tap_spot(&t);
            let d = ambient::bloom_degree(t.height, range);
            let lit = ripples.iter().rev().find(|r| r.0 == d && (r.1 - t.across as f32 / 255.0).abs() < 0.01).map(|r| (1.0 - (now - r.3) as f32 / 1.5).max(0.0)).unwrap_or(0.0);
            p.circle_filled(at, 3.0 + 1.5 * lit, mix(shade(accent, 0.55), Color32::WHITE, 0.6 * lit));
            if hovered_dot == Some(i) {
                p.circle_stroke(at, 7.0, Stroke::new(1.0_f32, REC));
            }
        }
    }
    for &(d, across, s, t) in &ripples {
        let age = (now - t) as f32;
        let at = spot(d, across);
        let a = s * (1.0 - age / 3.0).max(0.0);
        for k in 0..3 {
            let rr = 2.0 + (age - k as f32 * 0.25).max(0.0) * 22.0;
            if age > k as f32 * 0.25 {
                p.add(Shape::ellipse_stroke(at, vec2(rr, rr * 0.45), Stroke::new(1.5_f32, accent.gamma_multiply(a * (1.0 - k as f32 * 0.3)))));
            }
        }
        if age < 0.25 {
            p.circle_filled(at, 3.5 * s.max(0.3), mix(accent, Color32::WHITE, 0.5).gamma_multiply(1.0 - age / 0.25));
        }
    }
    // Just dropped: a drop falls in, before the engine's ripple answers.
    for &(d, across, t) in taps.iter().filter(|t| now - t.2 < 0.15) {
        let f = ((now - t) / 0.15) as f32;
        let at = spot(d, across);
        p.line_segment([at - vec2(0.0, 14.0 * (1.0 - f) + 6.0), at - vec2(0.0, 14.0 * (1.0 - f))], Stroke::new(2.0_f32, Color32::WHITE));
    }
    if let Some(&(d, across, _, _)) = ripples.last() {
        text(&p, spot(d, across) + vec2(0.0, -9.0), Align2::CENTER_BOTTOM, degree_name(&live, params[6], d), small(), TEXT_DIM);
    }
    // Under the pointer: the note a click would drop.
    if let Some(pos) = resp.hover_pos().filter(|pos| pond.contains(*pos) && hovered_dot.is_none()) {
        let up = ((lo_y - pos.y) / (lo_y - hi_y)).clamp(0.0, 1.0);
        let d = ambient::bloom_degree((up * 255.0).round() as u8, range);
        p.circle_stroke(pos2(pos.x, y_of(d)), 4.0, Stroke::new(1.0_f32, Color32::from_white_alpha(90)));
        text(&p, pos2(pos.x + 8.0, y_of(d)), Align2::LEFT_CENTER, degree_name(&live, params[6], d), small(), TEXT);
        ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
    }
    let echo = ambient::ECHOES[(params[1].round().max(0.0) as usize).min(ambient::ECHOES.len() - 1)];
    let count = ambient::bloom_taps(data).count();
    let foot = if looping && count > 0 {
        format!("{count} looping · every {echo}")
    } else {
        match live.vis {
            Some(v) if v[1] & 0xff > 0 => format!("{} blooming · echoes every {echo}", v[1] & 0xff),
            _ => "click the pond: higher is higher".to_string(),
        }
    };
    text(&p, pos2(rect.left() + 6.0, rect.bottom() - 3.0), Align2::LEFT_BOTTOM, foot, small(), TEXT_DIM);
    if count > 0 {
        let cr = Rect::from_min_size(pos2(rect.right() - 46.0, rect.bottom() - 17.0), vec2(40.0, 14.0));
        if button(ui, cr, id.with("clear"), "clear", false, accent).on_hover_text("Take all your notes off the pond").clicked() {
            data.clear();
            changed = true;
        }
    }
    if !ripples.is_empty() || live.vis.is_some_and(|v| v[1] & 0xff > 0) || live.on().is_some() {
        ui.ctx().request_repaint();
    }
    ui.ctx().memory_mut(|m| {
        m.data.insert_temp(rip_id, ripples);
        m.data.insert_temp(taps_id, taps);
    });
    resp.on_hover_text(if looping { "Click to drop a note (the higher, the higher the note): it keeps coming round. Click a dot to take it away" } else { "Click to drop a note (the higher, the higher the note): it echoes away. Left alone, Bloom plays by itself" });
    changed
}

// ---------------------------------------------------------------- chimes

/// The wind chime itself, as the engine simulates it: the clapper swings,
/// and a tube lights up exactly when it is struck. Drag left or right to
/// set the wind.
pub(crate) fn chimes(ui: &mut Ui, id: Id, params: &mut [f32], live: Live, height: f32) -> bool {
    let w = 260.0;
    let (rect, resp) = ui.allocate_exact_size(vec2(w, height), Sense::click_and_drag());
    let p = ui.painter_at(rect);
    rect_fill(&p, rect, 3.0, KNOB);
    let mut changed = false;
    if let Some(pos) = resp.interact_pointer_pos().filter(|_| resp.dragged() || resp.clicked()) {
        let v = (((pos.x - rect.left() - 10.0) / (w - 20.0)).clamp(0.0, 1.0) * 100.0).round() / 100.0;
        if params[0] != v {
            params[0] = v;
            changed = true;
        }
    }
    let shape = ambient::chime_shape(&live.scale, params);
    let (count, notes) = ambient::chime_notes(&live.scale, params);
    let names = notes[..count].iter().map(|n| krabton_engine::note_name((*n).clamp(0, 127) as u8)).collect();
    let mut st = super::chime_view::ChimeState::default();
    if let Some(v) = live.on() {
        let i16v = |b: u64| (b as u16 as i16) as f32 / 10000.0;
        let i8v = |b: u64| (b as u8 as i8) as f32 / 200.0;
        st.clapper = [i16v(v[0]), i16v(v[0] >> 16)];
        st.wind = ((v[0] >> 32) & 0xff) as f32 / 255.0 * 1.5;
        st.dir = ((v[0] >> 40) & 0xff) as f32 / 255.0 * std::f32::consts::TAU;
        for i in 0..count {
            st.tube[i] = [i8v(v[1] >> (8 * i)), i8v(v[2] >> (8 * i))];
            st.ring[i] = ((v[3] >> (8 * i)) & 0xff) as f32 / 255.0;
        }
        ui.ctx().request_repaint();
    } else {
        st.wind = params[0];
        st.dir = 0.4;
    }
    let look = super::chime_view::ChimeLook { shape, color: super::chime_view::material_color(0), thickness: 0.5, names };
    let t = ui.input(|i| i.time) as f32;
    super::chime_view::draw(&p, rect, &look, &st, t);
    text(&p, pos2(rect.left() + 6.0, rect.top() + 4.0), Align2::LEFT_TOP, format!("wind {:.0}%", st.wind * 100.0), small(), TEXT_DIM);
    resp.on_hover_text("Drag left or right: the wind. A note sounds whenever the clapper meets a tube");
    let _ = id;
    changed
}

// ---------------------------------------------------------------- drift chords

/// The voices as lines through time (now at the right edge), stepping when
/// they move; the chord sounding now is spelled out next to them.
pub(crate) fn drift_chords(ui: &mut Ui, id: Id, params: &[f32], live: Live, height: f32) -> bool {
    let w = 250.0;
    let (rect, _) = ui.allocate_exact_size(vec2(w, height), Sense::hover());
    let p = ui.painter().clone();
    rect_fill(&p, rect, 3.0, KNOB);
    let now = ui.input(|i| i.time);
    let vis = live.on();
    let range = params[6].round().max(7.0) as i32;
    let unpack = |v: u64, n: usize| -> Vec<i32> { (0..n).map(|i| ((v >> (i * 8)) & 0xff) as i32 - 64).collect() };
    let chord: Vec<i32> = match vis {
        Some(v) if v[1] & 0xff > 0 => unpack(v[0], (v[1] & 0xff) as usize),
        _ => {
            let spread = params[3].round().clamp(1.0, 4.0) as i32;
            (0..(params[0].round() as usize).clamp(3, 6)).map(|i| (i as i32 * spread).min(range)).collect()
        }
    };
    // The history: a new entry whenever a voice moves.
    let hist_id = id.with("hist");
    let prev: Vec<(Vec<i32>, f64)> = ui.ctx().memory(|m| m.data.get_temp(hist_id).unwrap_or_default());
    let new = vis.is_some() && prev.last().map(|h| h.0 != chord).unwrap_or(true);
    let hist = trail(ui, hist_id, new.then(|| (chord.clone(), now)), 24);
    let plot = Rect::from_min_max(rect.min + vec2(6.0, 8.0), pos2(rect.right() - 54.0, rect.bottom() - 8.0));
    let y_of = |d: i32| plot.bottom() - (d as f32 + 0.5) / (range + 1) as f32 * plot.height();
    for d in 0..=range {
        if d % 7 == 0 {
            p.line_segment([pos2(plot.left(), y_of(d)), pos2(plot.right(), y_of(d))], Stroke::new(1.0_f32, LINE_SOFT));
        }
    }
    let window = 40.0;
    let x_of = |t: f64| plot.right() - ((now - t) / window).clamp(0.0, 1.0) as f32 * plot.width();
    if vis.is_some() && !hist.is_empty() {
        for i in 0..chord.len() {
            let col = voice_color(i);
            let mut pts: Vec<egui::Pos2> = Vec::new();
            for (k, (c, t)) in hist.iter().enumerate() {
                let Some(&d) = c.get(i) else { continue };
                let x0 = x_of(*t);
                let x1 = hist.get(k + 1).map(|h| x_of(h.1)).unwrap_or(plot.right());
                if let Some(&last) = pts.last() {
                    pts.push(pos2(x0, last.y));
                }
                pts.push(pos2(x0, y_of(d)));
                pts.push(pos2(x1, y_of(d)));
            }
            p.add(Shape::line(pts, Stroke::new(2.0_f32, col)));
        }
        ui.ctx().request_repaint();
    } else {
        for (i, &d) in chord.iter().enumerate() {
            p.line_segment([pos2(plot.left(), y_of(d)), pos2(plot.right(), y_of(d))], Stroke::new(2.0_f32, voice_color(i).gamma_multiply(0.4)));
        }
    }
    // The chord now, high to low; the voices that just moved light up.
    let moved: Vec<bool> = match (hist.len() >= 2, vis.is_some()) {
        (true, true) => {
            let (a, b) = (&hist[hist.len() - 2].0, &hist[hist.len() - 1].0);
            let fresh = now - hist[hist.len() - 1].1 < 1.5;
            (0..chord.len()).map(|i| fresh && a.get(i) != b.get(i)).collect()
        }
        _ => vec![false; chord.len()],
    };
    let mut order: Vec<usize> = (0..chord.len()).collect();
    order.sort_by_key(|&i| -chord[i]);
    let lh = ((height - 12.0) / 6.0).min(16.0);
    for (row, &i) in order.iter().enumerate() {
        let y = rect.top() + 8.0 + row as f32 * lh;
        let r = Rect::from_min_size(pos2(rect.right() - 50.0, y), vec2(44.0, lh - 2.0));
        if moved[i] {
            rect_fill(&p, r, 2.0, voice_color(i));
        }
        text(&p, r.center(), Align2::CENTER_CENTER, degree_name(&live, params[5], chord[i]), small(), if moved[i] { TEXT_DARK } else { voice_color(i) });
    }
    false
}

// ---------------------------------------------------------------- tintinnabuli

/// A staff: the melody as filled notes, the triad voice as open notes
/// paired with them, the held tonic underneath.
pub(crate) fn tintinnabuli(ui: &mut Ui, id: Id, params: &[f32], live: Live, accent: Color32, height: f32) -> bool {
    let w = 250.0;
    let (rect, _) = ui.allocate_exact_size(vec2(w, height), Sense::hover());
    let p = ui.painter().clone();
    rect_fill(&p, rect, 3.0, KNOB);
    let vis = live.on();
    let range = params[1].round().clamp(3.0, 14.0) as i32;
    let drone = params[4] >= 0.5;
    // (melody, triad voice) per step, a new entry each step the engine counts.
    let step_id = id.with("step");
    let fresh = vis.and_then(|v| {
        let last: u64 = ui.ctx().memory(|m| m.data.get_temp(step_id).unwrap_or(u64::MAX));
        (v[1] != last).then(|| {
            ui.ctx().memory_mut(|m| m.data.insert_temp(step_id, v[1]));
            ((v[0] & 0xff) as i32 - 64, ((v[0] >> 8) & 0xff) as i32 - 64)
        })
    });
    let hist = trail(ui, id.with("notes"), fresh, 16);
    let (lo, hi) = (-range / 2 - 3, range + 3);
    let staff = Rect::from_min_max(rect.min + vec2(8.0, 8.0), pos2(rect.right() - 8.0, rect.bottom() - if drone { 34.0 } else { 20.0 }));
    let step_h = staff.height() / (hi - lo) as f32;
    let y_of = |d: i32| staff.bottom() - (d - lo) as f32 * step_h;
    // A line every other step, like a staff; the triad's steps marked.
    for d in lo..=hi {
        if d.rem_euclid(2) == 0 {
            p.line_segment([pos2(staff.left(), y_of(d)), pos2(staff.right(), y_of(d))], Stroke::new(1.0_f32, LINE_SOFT));
        }
        if matches!(d.rem_euclid(7), 0 | 2 | 4) {
            text(&p, pos2(staff.left() + 1.0, y_of(d)), Align2::LEFT_CENTER, "·", small(), SELECT.gamma_multiply(0.6));
        }
    }
    let head = vec2(5.0, (step_h * 0.9).clamp(3.0, 4.5));
    let dx = (staff.width() - 20.0) / 16.0;
    let n = hist.len();
    for (k, &(m, t)) in hist.iter().enumerate() {
        let x = staff.right() - 8.0 - (n - 1 - k) as f32 * dx;
        let age = (n - 1 - k) as f32 / 16.0;
        let a = 1.0 - age * 0.7;
        p.line_segment([pos2(x, y_of(m)), pos2(x, y_of(t))], Stroke::new(1.0_f32, LINE.gamma_multiply(a)));
        p.add(Shape::ellipse_filled(pos2(x, y_of(m)), head, accent.gamma_multiply(a)));
        p.add(Shape::ellipse_stroke(pos2(x, y_of(t)), head, Stroke::new(1.5_f32, SELECT.gamma_multiply(a))));
    }
    if drone {
        let y = rect.bottom() - 26.0;
        p.line_segment([pos2(staff.left(), y), pos2(staff.right(), y)], Stroke::new(3.0_f32, if vis.is_some() { mix(PANEL3, PLAY, 0.6) } else { PANEL3 }));
        text(&p, pos2(staff.left() + 2.0, y - 3.0), Align2::LEFT_BOTTOM, format!("tonic {}", degree_name(&live, params[5], -7)), small(), TEXT_DIM);
    }
    let foot = match hist.last() {
        Some(&(m, t)) if vis.is_some() => format!("melody {}  ·  triad {}", degree_name(&live, params[5], m), degree_name(&live, params[5], t)),
        _ => format!("triad voice: {}", ambient::POSITIONS[(params[3].round().max(0.0) as usize).min(ambient::POSITIONS.len() - 1)].to_lowercase()),
    };
    text(&p, pos2(rect.left() + 6.0, rect.bottom() - 3.0), Align2::LEFT_BOTTOM, foot, small(), TEXT_DIM);
    if vis.is_some() {
        ui.ctx().request_repaint();
    }
    false
}

// ---------------------------------------------------------------- pedal

/// The held note, big, and the notes held with it; the dots count the bars
/// until it may move. Drag the note up or down to change it.
pub(crate) fn pedal(ui: &mut Ui, id: Id, params: &mut [f32], live: Live, accent: Color32, height: f32) -> bool {
    let w = 200.0;
    let (rect, _) = ui.allocate_exact_size(vec2(w, height), Sense::hover());
    let vis = live.on();
    let root = vis.map(|v| (v[0] & 0xff) as i32 - 64).unwrap_or(0);
    let name_r = Rect::from_center_size(pos2(rect.center().x, rect.top() + height * 0.42), vec2(110.0, 48.0));
    let (resp, changed) = drag_steps(ui, name_r, id.with("base"), &mut params[6], 12.0, 72.0, 6.0);
    let p = ui.painter().clone();
    rect_fill(&p, rect, 3.0, KNOB);
    let now = ui.input(|i| i.time) as f32;
    if vis.is_some() {
        // Breathing slowly while it holds.
        let breath = 0.5 + 0.5 * (now * 0.9).sin();
        for k in 0..4 {
            p.circle_filled(name_r.center(), 22.0 + k as f32 * 7.0 + breath * 4.0, accent.gamma_multiply(0.05 + 0.02 * (3 - k) as f32));
        }
        ui.ctx().request_repaint();
    }
    let hot = resp.hovered() || resp.dragged();
    text(&p, name_r.center(), Align2::CENTER_CENTER, degree_name(&live, params[6], root), FontId::proportional(32.0), if hot { Color32::WHITE } else if vis.is_some() { accent } else { TEXT });
    if hot {
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
    }
    let mut with = Vec::new();
    if params[3] >= 0.5 {
        with.push(degree_name(&live, params[6], root + 7));
    }
    if params[4] >= 0.5 {
        with.push(degree_name(&live, params[6], root + 4));
    }
    if !with.is_empty() {
        text(&p, pos2(name_r.center().x, name_r.bottom() + 2.0), Align2::CENTER_TOP, format!("with {}", with.join(" and ")), small(), TEXT_DIM);
    }
    if root != 0 {
        text(&p, pos2(name_r.center().x, name_r.top() - 2.0), Align2::CENTER_BOTTOM, format!("away from {}", degree_name(&live, params[6], 0)), small(), TEXT_DIM);
    }
    let change = [1usize, 2, 4, 8, 0][(params[0].round().max(0.0) as usize).min(4)];
    let bar = vis.map(|v| v[1] as usize).unwrap_or(0);
    let y = rect.bottom() - 12.0;
    if change == 0 {
        text(&p, pos2(rect.center().x, y), Align2::CENTER_CENTER, "holds forever", small(), TEXT_DIM);
    } else {
        let gap = 14.0;
        let x0 = rect.center().x - gap * (change as f32 - 1.0) * 0.5;
        for k in 0..change {
            let here = vis.is_some() && bar % change == k;
            let c = pos2(x0 + k as f32 * gap, y);
            if here {
                p.circle_filled(c, 4.0, accent);
            } else {
                p.circle_stroke(c, 3.0, Stroke::new(1.0_f32, LINE));
            }
        }
    }
    resp.on_hover_text("Drag up or down: the note the pedal holds");
    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Pos2, RawInput};
    use krabton_engine::midi_fx::MidiFxKind;
    use krabton_engine::params::defaults;
    use krabton_engine::sequencers::ambient;

    /// Draws the editor for a few frames with live data and returns its params.
    fn show(kind: MidiFxKind, vis: [u64; 4]) -> Vec<f32> {
        let mut params = defaults(kind.specs());
        let ctx = egui::Context::default();
        for f in 0..3u64 {
            let input = RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1200.0, 800.0))), time: Some(f as f64 * 0.1), ..Default::default() };
            let _ = ctx.run(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let v = [vis[0].rotate_left(f as u32 * 8), vis[1] + f, vis[2], vis[3]];
                    let live = Live { vis: Some(v), playing: true, scale: krabton_engine::Scale::default() };
                    super::super::seq_view::editor(ui, Id::new("t"), kind, &mut params, &mut Vec::new(), live, &mut 0, ACCENT, 160.0);
                });
            });
        }
        params
    }

    #[test]
    fn clicking_the_pond_drops_a_note() {
        use egui::{Event, Modifiers, PointerButton, Pos2};
        let mut params = defaults(MidiFxKind::Bloom.specs());
        let mut data: Vec<u8> = Vec::new();
        let ctx = egui::Context::default();
        let mut origin = Pos2::ZERO;
        for f in 0..3 {
            let mut input = RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1200.0, 800.0))), ..Default::default() };
            if f > 0 {
                let at = origin + vec2(100.0, 20.0);
                input.events.push(Event::PointerMoved(at));
                input.events.push(Event::PointerButton { pos: at, button: PointerButton::Primary, pressed: f == 1, modifiers: Modifiers::NONE });
            }
            let _ = ctx.run(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    origin = ui.cursor().min;
                    let live = Live { vis: None, playing: false, scale: krabton_engine::Scale::default() };
                    super::super::seq_view::editor(ui, Id::new("t"), MidiFxKind::Bloom, &mut params, &mut data, live, &mut 0, ACCENT, 160.0);
                });
            });
        }
        let taps: Vec<_> = ambient::bloom_taps(&data).collect();
        assert_eq!(taps.len(), 1, "{data:?}");
        assert_eq!(taps[0].seq, 1);
        // 20 px from the top of a 160 px editor: near the top of the range.
        assert!(taps[0].height > 200, "{taps:?}");
    }

    #[test]
    fn ambient_editors_draw_live_state() {
        show(MidiFxKind::Loops, [0x10_40_80_c0_f0, 5, 0, 0]);
        show(MidiFxKind::Bloom, [0x40_42_44, 3 | 14 << 8, 0, 0]);
        show(MidiFxKind::Chimes, [0x1_2_3_4_5_6, 450 | 6 << 16, 0, 0]);
        show(MidiFxKind::DriftChords, [0x48_46_43_40, 4 | 14 << 8, 0, 0]);
        show(MidiFxKind::Tintinnabuli, [0x1_42_41, 7, 0, 0]);
        show(MidiFxKind::Pedal, [0x1_44, 3, 0, 0]);
    }
}

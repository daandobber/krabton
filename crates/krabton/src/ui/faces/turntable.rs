//! Turntable as a DJ deck: the platter with the record on it (drag it round
//! to scratch: the hand turns the record, let go and the slipmat brings it
//! back up to the platter's speed), the pitch fader with its range, Sync
//! and Key Lock, the whole record with the needle, its hot cues and the
//! loop (click to drop the needle), the deck's buttons (start and stop,
//! brake, backspin, loop, cut, the eight hot cues and six scratches held to
//! play them in the song's time), the motor, the cut fader and DJ filter,
//! and the vinyl's crackle, hiss and wow.

use super::{draw, Ctl, Ctx, Face, Presets};
use crate::theme::*;
use crate::ui::inst_screen::InstLive;
use crate::ui::modulation::ModView;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, FontId, Id, Rect, Sense, Shape, Stroke, Ui};
use krabton_engine::dsp::turntable::{self as tt, DeckAct, Record};
use krabton_engine::sample::SampleRef;
use std::f32::consts::TAU;

pub(crate) fn face() -> Face {
    Face::new(71.5, 12.0)
        .section(0.0, 0.0, 13.0, 12.0, "Turntable")
        .preset(0.3, 0.3, 12.4, 1.3)
        .screen(0.3, 1.8, 12.4, 9.4, "platter")
        .section(13.2, 0.0, 7.0, 12.0, "Pitch")
        .ctl(13.4, 0.3, 2.4, 10.9, "Pitch", Ctl::Fader)
        .ctl(15.8, 0.3, 2.2, 5.3, "Range", Ctl::Switch)
        .ctl(15.8, 5.9, 2.2, 5.3, "RPM", Ctl::Switch)
        .ctl(18.0, 0.3, 2.0, 5.3, "Sync", Ctl::Switch)
        .ctl(18.0, 5.9, 2.0, 5.3, "Key Lock", Ctl::Switch)
        .section(20.4, 0.0, 20.0, 12.0, "Record")
        .screen(20.6, 0.3, 19.6, 3.6, "record")
        .screen(20.6, 4.1, 19.6, 7.1, "pads")
        .section(40.6, 0.0, 7.0, 12.0, "Motor")
        .row(40.7, 0.3, 2.25, 5.3, &["Start", "Stop", "Brake"], Ctl::Knob)
        .row(40.7, 5.9, 2.25, 5.3, &["Backspin", "Slipmat"], Ctl::Knob)
        .section(47.8, 0.0, 9.6, 12.0, "Time")
        .ctl(48.0, 0.3, 3.0, 5.3, "Song", Ctl::Switch)
        .ctl(51.0, 0.3, 3.0, 5.3, "Quantize", Ctl::Switch)
        .ctl(54.0, 0.3, 3.2, 5.3, "Loop", Ctl::Selector)
        .named(48.0, 5.9, 4.6, 5.3, "Move Rate", Ctl::Switch, "Scratch")
        .named(52.6, 5.9, 4.6, 5.3, "Move Size", Ctl::Knob, "Size")
        .section(57.6, 0.0, 7.9, 12.0, "Mixer")
        .ctl(57.8, 0.3, 2.3, 10.9, "Fader", Ctl::Fader)
        .col(60.1, 0.3, 2.6, 5.45, &["Filter", "Resonance"], Ctl::Knob)
        .col(62.7, 0.3, 2.6, 5.45, &["Curve", "Volume"], Ctl::Knob)
        .section(65.7, 0.0, 5.8, 12.0, "Vinyl")
        .col(65.9, 0.3, 2.7, 5.45, &["Crackle", "Noise"], Ctl::Knob)
        .ctl(68.6, 0.3, 2.7, 5.45, "Wow", Ctl::Knob)
}

/// What the face does to the deck this frame.
pub(crate) struct Deck {
    pub acts: Vec<DeckAct>,
    pub said: Option<String>,
}

/// What the engine says of the deck (see `TurntableState::visual`).
struct Now {
    pos: f32,
    len: f32,
    speed: f32,
    motor: bool,
    hand: bool,
    moving: i32,
    looping: bool,
    loop_start: f32,
    loop_len: f32,
    gain: f32,
    target: f32,
    waiting: bool,
    lock: f32,
}

impl Now {
    fn of(live: &InstLive) -> Self {
        let v = &live.vals;
        Self {
            pos: v[0],
            len: v[1],
            speed: v[2],
            motor: v[4] > 0.5,
            hand: v[5] > 0.5,
            moving: v[6] as i32,
            looping: v[7] > 0.5,
            loop_start: v[8],
            loop_len: v[9],
            gain: v[11],
            target: v[12],
            waiting: v[13] > 0.5,
            lock: v[14],
        }
    }
}

/// Puts a record on the deck: it is listened to for its tempo and cues.
fn put_on(s: SampleRef, sample: &mut Option<SampleRef>, record: &mut Record) {
    *record = tt::listen(&s);
    *sample = Some(s);
}

/// The Turntable's panel: a row to put a record on, then the deck.
#[allow(clippy::too_many_arguments)]
pub(crate) fn panel(ui: &mut Ui, id: Id, preset: &mut String, params: &mut Vec<f32>, sample: &mut Option<SampleRef>, record: &mut Record, live: &InstLive, height: f32, mv: Option<&mut ModView>) -> Deck {
    let mut deck = Deck { acts: Vec::new(), said: None };
    if params.len() < tt::SPECS.len() {
        return deck;
    }
    let mut load = |path: &std::path::Path, sample: &mut Option<SampleRef>, record: &mut Record| match SampleRef::load(path) {
        Ok(s) => put_on(s, sample, record),
        Err(e) => deck.said = Some(format!("Could not load the record: {e}")),
    };
    ui.horizontal(|ui| {
        if ui.button("Load record...").on_hover_text("A song, a loop or a break (WAV, FLAC, MP3 or AIFF) to put on the deck: its tempo and eight hot cues are found as it goes on (or drop one on the deck)").clicked() {
            if let Some(path) = crate::import::audio_file_dialog().pick_file() {
                load(&path, sample, record);
            }
        }
        match sample.as_ref() {
            Some(s) => {
                let tempo = if record.bpm > 0.0 { format!("  {:.1} BPM", record.bpm) } else { String::new() };
                ui.label(egui::RichText::new(format!("{}  {:.1} s{tempo}", s.name, s.duration())).color(accent_now()));
                if ui.small_button("x").on_hover_text("Take the record off").clicked() {
                    *sample = None;
                    *record = Record::default();
                }
            }
            None => {
                ui.label(egui::RichText::new("no record on the deck: load one or drop a sample on it").small().color(crate::skin::ink_dim()));
            }
        }
    });
    // A record put on without being listened to (an older set, a script): now.
    if let (Some(s), false) = (sample.as_ref(), record.heard) {
        if s.analysis_ready().is_some() {
            *record = tt::listen(s);
        }
    }
    let h = (height - 28.0).max(120.0);
    let face = face();
    let now = Now::of(live);
    let turn = tt::turn_secs(params) as f32;
    let mut acts = Vec::new();
    let resp = {
        let sample_now = sample.clone();
        let rec = &mut *record;
        let mut cx = Ctx { id, keys: tt::SPECS, specs: tt::SPECS, params, mv, presets: Some(Presets { current: preset, names: tt::preset_names(), load: tt::preset }) };
        ui.scope(|ui| {
            draw(ui, &face, h, &mut cx, &mut |ui, which, rect, p| {
                match which {
                    "platter" => platter(ui, id.with("platter"), rect, &now, turn, sample_now.as_ref(), &mut acts),
                    "record" => overview(ui, id.with("record"), rect, &now, p, sample_now.as_ref(), rec, &mut acts),
                    "pads" => return pads(ui, id.with("pads"), rect, &now, rec, &mut acts),
                    _ => {}
                }
                false
            });
        })
        .response
    };
    deck.acts = acts;
    // A sample from the browser goes on the deck, or a file from outside
    // dropped on it (or anywhere, when the system does not say where).
    if let Some(item) = drop_payload::<crate::app::BrowserItem>(&resp) {
        if let crate::app::BrowserItem::Sample(path) = &*item {
            load(path, sample, record);
        }
    }
    let here = crate::import::file_drop_pos(ui.ctx()).is_none_or(|p| resp.rect.contains(p));
    let files_over = ui.ctx().input(|i| !i.raw.hovered_files.is_empty());
    if resp.dnd_hover_payload::<crate::app::BrowserItem>().is_some() || (files_over && here) {
        rect_line(ui.painter(), resp.rect, 6.0, 2.0, accent_now());
    }
    let dropped: Vec<std::path::PathBuf> = ui.ctx().input(|i| i.raw.dropped_files.iter().filter_map(|f| f.path.clone()).collect());
    if let Some(path) = dropped.iter().find(|p| krabton_engine::sample::is_audio_file(p)).filter(|_| here) {
        load(path, sample, record);
        crate::import::take_file_drop(ui.ctx());
    }
    if now.motor || now.speed.abs() > 1e-4 || now.hand || now.moving >= 0 || now.waiting {
        ui.ctx().request_repaint();
    }
    deck
}

/// The platter and the record on it, turned by the hand.
fn platter(ui: &mut Ui, id: Id, r: Rect, now: &Now, turn: f32, sample: Option<&SampleRef>, acts: &mut Vec<DeckAct>) {
    let p = ui.painter().clone();
    let (well, _) = crate::skin::well();
    let d = r.height().min(r.width() * 0.8);
    let c = pos2(r.left() + d * 0.5 + 2.0, r.center().y);
    let rad = d * 0.5 - 2.0;
    // The deck's plinth.
    p.rect_filled(r, 6.0, mix(well, Color32::BLACK, 0.2));
    // The platter's rim and its strobe dots, turning with it.
    p.circle_filled(c + vec2(0.0, 2.0), rad, Color32::from_black_alpha(120));
    p.circle_filled(c, rad, Color32::from_rgb(0x9a, 0x9c, 0xa0));
    let angle = now.pos / turn.max(0.1) * TAU;
    for k in 0..48 {
        let a = angle + k as f32 / 48.0 * TAU;
        let at = c + vec2(a.cos(), a.sin()) * (rad - 2.5);
        p.circle_filled(at, 1.3, if k % 2 == 0 { Color32::from_rgb(0x5c, 0x5e, 0x62) } else { Color32::from_rgb(0xd2, 0xd4, 0xd6) });
    }
    // The record: its grooves, a gleam, the label.
    let vr = rad - 6.0;
    p.circle_filled(c, vr, Color32::from_rgb(0x12, 0x12, 0x14));
    let groove = Stroke::new(1.0_f32, Color32::from_rgb(0x22, 0x22, 0x26));
    let mut g = vr * 0.97;
    while g > vr * 0.4 {
        p.circle_stroke(c, g, groove);
        g -= (vr * 0.035).max(2.0);
    }
    // Where the needle is along the grooves.
    let along = if now.len > 0.0 { (now.pos / now.len).clamp(0.0, 1.0) } else { 0.0 };
    let needle_r = vr * (0.95 - 0.53 * along);
    for a in [-0.55f32, 2.6] {
        let pts: Vec<_> = (0..12).map(|k| a + k as f32 * 0.04).map(|t| c + vec2(t.cos(), t.sin()) * vr * 0.8).collect();
        p.add(Shape::line(pts, Stroke::new(vr * 0.25, Color32::from_white_alpha(7))));
    }
    let label = vr * 0.36;
    p.circle_filled(c, label, accent_now());
    // A mark on the label, turning with the record.
    let mark = c + vec2(angle.cos(), angle.sin()) * label;
    p.line_segment([c + vec2(angle.cos(), angle.sin()) * label * 0.3, mark], Stroke::new(2.5_f32, text_on(accent_now())));
    let name = sample.map(|s| s.name.as_str()).unwrap_or("no record");
    let font = super::fit(&p, name, FontId::proportional((label * 0.3).clamp(7.0, 12.0)), label * 1.5);
    p.text(c + vec2(0.0, -label * 0.5), Align2::CENTER_CENTER, name, font, text_on(accent_now()));
    p.circle_filled(c, 2.5, Color32::from_rgb(0xd8, 0xd8, 0xd8));
    // The tone arm, from its post down to the needle.
    let post = pos2((c.x + rad + (r.right() - c.x - rad) * 0.5).min(r.right() - 8.0), r.top() + d * 0.16);
    let head = c + vec2(0.35, -0.94).normalized() * needle_r;
    p.circle_filled(post, d * 0.06, Color32::from_rgb(0x55, 0x57, 0x5b));
    p.line_segment([post, head], Stroke::new(3.0_f32, Color32::from_rgb(0xc8, 0xca, 0xcc)));
    p.rect_filled(Rect::from_center_size(head, vec2(7.0, 11.0)), 1.5, Color32::from_rgb(0x2a, 0x2a, 0x2c));
    // To its right: the speed now, the motor's light, the fader's gain.
    let side = Rect::from_min_max(pos2(c.x + rad + 4.0, r.top() + d * 0.32), pos2(r.right() - 3.0, r.bottom() - 4.0));
    if side.width() > 18.0 {
        let ink = crate::skin::well().1;
        let f = FontId::monospace((side.width() * 0.22).clamp(8.0, 12.0));
        // The pitch the motor is set to (a deck's readout), the record's own speed while the hand has it.
        let pct = if now.hand || now.moving >= 0 { (now.speed - 1.0) * 100.0 } else { (now.target - 1.0) * 100.0 };
        tabular_text(&p, pos2(side.center().x, side.top()), Align2::CENTER_TOP, &format!("{pct:+.1}%"), f.clone(), ink);
        let light = if now.motor { accent_now() } else { Color32::from_rgb(0x40, 0x40, 0x44) };
        p.circle_filled(pos2(side.center().x, side.top() + 26.0), 4.0, light);
        if now.lock > 0.5 {
            p.text(pos2(side.center().x, side.top() + 40.0), Align2::CENTER_TOP, "KEY", f.clone(), accent_now());
        }
        let g = Rect::from_min_max(pos2(side.center().x - 3.0, side.top() + 56.0), pos2(side.center().x + 3.0, side.bottom()));
        if g.height() > 8.0 {
            p.rect_filled(g, 2.0, Color32::from_black_alpha(150));
            let top = g.bottom() - g.height() * now.gain.clamp(0.0, 1.0);
            p.rect_filled(Rect::from_min_max(pos2(g.left(), top), g.max), 2.0, accent_now());
        }
    }
    // The hand: drag round the platter to turn the record.
    let hit = Rect::from_center_size(c, vec2(rad * 2.0, rad * 2.0));
    let resp = ui.interact(hit, id, Sense::drag());
    let key = id.with("hand");
    if let Some(pt) = ui.input(|i| i.pointer.interact_pos()) {
        let a = (pt - c).y.atan2((pt - c).x);
        if resp.drag_started() {
            ui.ctx().memory_mut(|m| m.data.insert_temp(key, (a, 0.0f32)));
            acts.push(DeckAct::Hand(Some(0.0)));
        } else if resp.dragged() {
            let (last, turned) = ui.ctx().memory(|m| m.data.get_temp::<(f32, f32)>(key)).unwrap_or((a, 0.0));
            let mut da = a - last;
            if da > std::f32::consts::PI {
                da -= TAU;
            } else if da < -std::f32::consts::PI {
                da += TAU;
            }
            let turned = turned + da / TAU;
            ui.ctx().memory_mut(|m| m.data.insert_temp(key, (a, turned)));
            acts.push(DeckAct::Hand(Some(turned as f64)));
        }
    }
    if resp.drag_stopped() {
        ui.ctx().memory_mut(|m| m.data.remove::<(f32, f32)>(key));
        acts.push(DeckAct::Hand(None));
    }
    resp.on_hover_text("Drag round the record to scratch it (clockwise: forward). Let go and the slipmat brings it back up to the platter's speed");
}

/// The whole record: its wave, the needle, the cues and the loop. Click to drop the needle there.
#[allow(clippy::too_many_arguments)]
fn overview(ui: &mut Ui, id: Id, r: Rect, now: &Now, p: &[f32], sample: Option<&SampleRef>, rec: &Record, acts: &mut Vec<DeckAct>) {
    let painter = ui.painter().clone();
    let (well, ink) = crate::skin::well();
    painter.rect_filled(r, 4.0, well);
    let head = Rect::from_min_size(r.min + vec2(4.0, 2.0), vec2(r.width() - 8.0, 13.0));
    let wave = Rect::from_min_max(pos2(r.left() + 3.0, head.bottom() + 2.0), r.max - vec2(3.0, 3.0));
    let f = FontId::monospace(10.0);
    let Some(s) = sample else {
        painter.text(r.center(), Align2::CENTER_CENTER, "drop a song, a loop or a break here", FontId::proportional(11.0), mix(ink, well, 0.4));
        return;
    };
    let dur = s.duration().max(1e-3) as f32;
    // The record's tempo, and where the motor plays it.
    let speed = now.target;
    let left = format!("{} / {}", clock(now.pos), clock(dur));
    tabular_text(&painter, head.left_center(), Align2::LEFT_CENTER, &left, f.clone(), ink);
    let right = if rec.bpm > 0.0 {
        format!("{:.1} BPM  {:+.1}%  {:.1}", rec.bpm, (speed - 1.0) * 100.0, rec.bpm * speed as f64)
    } else {
        format!("{:+.1}%", (speed - 1.0) * 100.0)
    };
    let right = if p[tt::SYNC] >= 0.5 && rec.bpm > 0.0 { format!("SYNC  {right}") } else { right };
    tabular_text(&painter, head.right_center(), Align2::RIGHT_CENTER, &right, f, if p[tt::SYNC] >= 0.5 { accent_now() } else { ink });
    let x_of = |secs: f32| wave.left() + (secs / dur).clamp(0.0, 1.0) * wave.width();
    // The loop.
    if now.looping {
        let a = Rect::from_min_max(pos2(x_of(now.loop_start), wave.top()), pos2(x_of(now.loop_start + now.loop_len).max(x_of(now.loop_start) + 2.0), wave.bottom()));
        painter.rect_filled(a, 0.0, mix(accent_now(), well, 0.7));
    }
    // The wave, from its peaks.
    let peaks = &s.peaks;
    let cols = wave.width().max(1.0) as usize;
    let per = peaks.len() as f32 / cols as f32;
    let wc = mix(ink, well, 0.35);
    let played = mix(accent_now(), well, 0.15);
    let mid = wave.center().y;
    let hh = wave.height() * 0.5;
    let at = x_of(now.pos);
    for x in 0..cols {
        let a = (x as f32 * per) as usize;
        let b = (((x + 1) as f32 * per) as usize).max(a + 1).min(peaks.len());
        if a >= peaks.len() {
            break;
        }
        let (lo, hi) = peaks[a..b].iter().fold((0.0f32, 0.0f32), |(l, h), &(mn, mx)| (l.min(mn), h.max(mx)));
        let px = wave.left() + x as f32 + 0.5;
        painter.line_segment([pos2(px, mid - hi.min(1.0) * hh), pos2(px, mid - lo.max(-1.0) * hh)], Stroke::new(1.0_f32, if px < at { played } else { wc }));
    }
    // The beats of the record, faintly, if it has a tempo.
    if rec.bpm > 0.0 {
        let bar = (4.0 * 60.0 / rec.bpm) as f32;
        let bars = ((dur - rec.beat as f32) / bar) as usize;
        if bars > 0 && wave.width() / bars as f32 > 3.0 {
            for k in 0..=bars {
                let x = x_of(rec.beat as f32 + k as f32 * bar);
                painter.line_segment([pos2(x, wave.bottom() - 3.0), pos2(x, wave.bottom())], Stroke::new(1.0_f32, mix(ink, well, 0.5)));
            }
        }
    }
    // The hot cues.
    for (k, cue) in rec.cues.iter().enumerate() {
        let Some(secs) = cue else { continue };
        let x = x_of(*secs as f32);
        painter.line_segment([pos2(x, wave.top()), pos2(x, wave.bottom())], Stroke::new(1.0_f32, cue_color(k)));
        let tag = Rect::from_min_size(pos2(x, wave.top()), vec2(10.0, 10.0));
        painter.rect_filled(tag, 1.0, cue_color(k));
        painter.text(tag.center(), Align2::CENTER_CENTER, format!("{}", k + 1), FontId::monospace(8.0), Color32::BLACK);
    }
    // The needle.
    painter.line_segment([pos2(at, wave.top() - 2.0), pos2(at, wave.bottom() + 1.0)], Stroke::new(2.0_f32, Color32::WHITE));
    let resp = ui.interact(wave, id, Sense::click_and_drag());
    if let Some(pt) = resp.interact_pointer_pos() {
        if resp.clicked() || resp.dragged() {
            let secs = ((pt.x - wave.left()) / wave.width()).clamp(0.0, 1.0) * dur;
            acts.push(DeckAct::Jump { secs: secs as f64, play: false });
        }
    }
    resp.on_hover_text("The record: click or drag to drop the needle there");
}

fn clock(secs: f32) -> String {
    let s = secs.max(0.0);
    format!("{}:{:04.1}", (s / 60.0) as u32, s % 60.0)
}

fn cue_color(k: usize) -> Color32 {
    const C: [Color32; 8] = [
        Color32::from_rgb(0xff, 0x4d, 0x4d),
        Color32::from_rgb(0xff, 0x9f, 0x1c),
        Color32::from_rgb(0xff, 0xe0, 0x3b),
        Color32::from_rgb(0x5c, 0xe0, 0x6a),
        Color32::from_rgb(0x2e, 0xd3, 0xc8),
        Color32::from_rgb(0x4d, 0x9d, 0xff),
        Color32::from_rgb(0xa9, 0x6b, 0xff),
        Color32::from_rgb(0xff, 0x6b, 0xc8),
    ];
    C[k % 8]
}

/// A pad held down: (pressed now, let go now, held).
fn hold(ui: &Ui, resp: &egui::Response, id: Id) -> (bool, bool, bool) {
    let down = resp.is_pointer_button_down_on();
    let key = id.with("down");
    let was = ui.ctx().memory(|m| m.data.get_temp::<bool>(key)).unwrap_or(false);
    if down != was {
        ui.ctx().memory_mut(|m| m.data.insert_temp(key, down));
    }
    (down && !was, was && !down, down)
}

/// A pad: lit when on, its colour when it has one.
fn pad(ui: &mut Ui, r: Rect, id: Id, label: &str, lit: bool, color: Option<Color32>) -> egui::Response {
    let resp = ui.interact(r, id, Sense::click_and_drag());
    let p = ui.painter();
    let (well, ink) = crate::skin::well();
    let base = color.map_or(well, |c| mix(c, well, 0.55));
    let fill = if lit || resp.is_pointer_button_down_on() {
        color.unwrap_or_else(accent_now)
    } else if resp.hovered() {
        mix(base, Color32::WHITE, 0.12)
    } else {
        base
    };
    p.rect_filled(r.translate(vec2(0.0, 1.5)), 4.0, Color32::from_black_alpha(110));
    p.rect_filled(r, 4.0, fill);
    let font = super::fit(p, label, FontId::proportional((r.height() * 0.32).clamp(8.0, 12.0)), r.width() - 4.0);
    p.text(r.center(), Align2::CENTER_CENTER, label, font, if fill == base { ink } else { text_on(fill) });
    resp
}

/// The deck's buttons: the transport, the hot cues, the scratches.
fn pads(ui: &mut Ui, id: Id, r: Rect, now: &Now, rec: &mut Record, acts: &mut Vec<DeckAct>) -> bool {
    let rows = 3.0;
    let rh = r.height() / rows;
    let row = |k: f32| Rect::from_min_size(pos2(r.left(), r.top() + k * rh), vec2(r.width(), rh));
    let mut changed = false;
    // The transport.
    let top = row(0.0);
    let w = top.width() / 5.0;
    let cell = |rr: Rect, k: usize, w: f32| Rect::from_min_size(pos2(rr.left() + k as f32 * w, rr.top()), vec2(w, rr.height())).shrink(2.5);
    let play_label = if now.waiting { "WAIT" } else if now.motor { "STOP" } else { "START" };
    if pad(ui, cell(top, 0, w), id.with("play"), play_label, now.motor, None).on_hover_text("Start or stop the motor (on the beat or bar when the song plays and Quantize is on)").clicked() {
        acts.push(DeckAct::StartStop);
    }
    if pad(ui, cell(top, 1, w), id.with("brake"), "BRAKE", false, None).on_hover_text("The power off: the platter runs down in the Brake time").clicked() {
        acts.push(DeckAct::Brake);
    }
    if pad(ui, cell(top, 2, w), id.with("spin"), "SPIN", false, None).on_hover_text("A backspin: the record thrown back, running down in the Backspin time").clicked() {
        acts.push(DeckAct::Backspin);
    }
    if pad(ui, cell(top, 3, w), id.with("loop"), "LOOP", now.looping, None).on_hover_text("Loop the Loop's length from the beat the needle is in (click again to go on)").clicked() {
        acts.push(DeckAct::Loop(!now.looping));
    }
    let cut = pad(ui, cell(top, 4, w), id.with("cut"), "CUT", false, None).on_hover_text("Cut the sound while held");
    match hold(ui, &cut, id.with("cut")) {
        (true, _, _) => acts.push(DeckAct::Cut(true)),
        (_, true, _) => acts.push(DeckAct::Cut(false)),
        _ => {}
    }
    // The hot cues: click to jump there and play (an empty one, or with
    // Shift: set it where the needle is); right-click to clear one.
    let mid = row(1.0);
    let w = mid.width() / tt::CUES as f32;
    for k in 0..tt::CUES {
        let set = rec.cues[k].is_some();
        let resp = pad(ui, cell(mid, k, w), id.with(("cue", k)), &format!("{}", k + 1), false, set.then(|| cue_color(k)));
        let shift = ui.input(|i| i.modifiers.shift);
        let resp = resp.on_hover_text(match rec.cues[k] {
            Some(s) => format!("Hot cue {}: {}\nClick to jump there and play; Shift-click to set it where the needle is; right-click to clear it", k + 1, clock(s as f32)),
            None => format!("Hot cue {}: empty\nClick (or Shift-click) to set it where the needle is", k + 1),
        });
        if resp.clicked() {
            if shift || !set {
                rec.cues[k] = Some(now.pos as f64);
                changed = true;
            } else if let Some(s) = rec.cues[k] {
                acts.push(DeckAct::Jump { secs: s, play: true });
            }
        }
        if resp.secondary_clicked() && set {
            rec.cues[k] = None;
            changed = true;
        }
    }
    // The scratches, played in the song's time while held.
    let low = row(2.0);
    let w = low.width() / tt::MOVES.len() as f32;
    for (k, name) in tt::MOVES.iter().enumerate() {
        let resp = pad(ui, cell(low, k, w), id.with(("move", k)), name, now.moving == k as i32, None).on_hover_text(format!("{name}: a scratch played in the song's time while held"));
        match hold(ui, &resp, id.with(("move", k))) {
            (true, _, _) => acts.push(DeckAct::Move(k, true)),
            (_, true, _) => acts.push(DeckAct::Move(k, false)),
            _ => {}
        }
    }
    changed
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_face_holds_every_parameter() {
        super::super::check(&super::face(), krabton_engine::dsp::turntable::SPECS);
    }
}

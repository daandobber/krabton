//! Analog Princess as a drum machine: the pattern bank (A to H, the chain,
//! the length and what to do with a pattern), the twelve tracks as buttons,
//! the selected track's steps as big pads with the playhead going over them
//! and its rows of velocity, roll, chance, accent, flam, note and slide under
//! them, the selected track's sound in knobs, and the FRY section with Deep
//! Fry big and the stutter, tape stop and channel surf as buttons to hold.
//! Click a step to set it, right-click it for the next roll, drag it up or
//! down for its velocity; on the rows, click for the next, right-click for
//! the one before.

use super::{fit, knob, knob_rect, names_display, screen_box, section};
use crate::theme::*;
use crate::ui::inst_screen::InstLive;
use crate::ui::modulation::{knob_overlay, ModView};
use egui::{pos2, vec2, Align2, Color32, FontId, Id, Rect, Sense, Stroke, StrokeKind, Ui};
use krabton_engine::dsp::princess_machine::{self as pm, Machine, Pattern, Step, TRACKS};
use krabton_engine::params::ParamSpec;
use krabton_engine::Scale;

/// The panel's size in units.
const W: f32 = 95.3;
const H: f32 = 15.5;
/// Where the sections sit (x, width).
const PAT_X: f32 = 0.0;
const PAT_W: f32 = 9.0;
const SEQ_X: f32 = 9.3;
const SEQ_W: f32 = 35.4;
const SND_X: f32 = 45.0;
const SND_W: f32 = 23.4;
const FRY_X: f32 = 68.7;
const FRY_W: f32 = 26.6;
/// The steps' rows under the pads.
const LANES: [&str; 7] = ["Vel", "Roll", "Chance", "Accent", "Flam", "Note", "Slide"];
/// Lengths a track can have of its own (0: the pattern's).
const TRACK_LENS: [u8; 18] = [0, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 24, 32];

/// What the face remembers between frames.
#[derive(Clone, Default)]
struct View {
    track: usize,
    page: usize,
    clip: Option<Pattern>,
    /// The hold buttons left on (right-clicked).
    latched: [bool; 3],
}

fn spec(i: usize) -> &'static ParamSpec {
    &pm::SPECS[i]
}

/// The pads' colours without the machine's look.
const PADS: [Color32; 4] = [Color32::from_rgb(0xff, 0x4f, 0xb0), Color32::from_rgb(0xb0, 0x4f, 0xff), Color32::from_rgb(0x4f, 0xe0, 0xff), Color32::from_rgb(0xff, 0xe1, 0x4f)];

/// The colour of a track's pads.
fn track_colour(t: usize) -> Color32 {
    let pads = crate::skin::current().map(|s| s.pads).filter(|p| p.len() >= 4).unwrap_or(&PADS);
    match t {
        pm::BASS | pm::KICK => pads[0],
        pm::SNARE | pm::CLAP | pm::RIM => pads[1],
        pm::HAT | pm::OPEN => pads[2],
        _ => pads[3],
    }
}

fn small_font(u: f32) -> FontId {
    FontId::proportional((u * 0.48).clamp(8.0, 12.0))
}

/// A button with a light: its text, on or off.
fn led_button(ui: &mut Ui, r: Rect, id: Id, text: &str, on: bool, lit: bool, u: f32) -> egui::Response {
    let resp = ui.interact(r, id, Sense::click());
    let p = ui.painter();
    let (well, well_ink) = crate::skin::well();
    let fill = if on { mix(well, accent_now(), 0.55) } else if resp.hovered() { mix(well, Color32::WHITE, 0.12) } else { well };
    p.rect_filled(r.translate(vec2(0.0, 1.5)), 3.0, Color32::from_black_alpha(100));
    p.rect_filled(r, 3.0, fill);
    if lit {
        p.rect_stroke(r.expand(1.0), 3.0, Stroke::new(2.0_f32, accent_now()), StrokeKind::Outside);
    }
    let text = crate::skin::label_text(text);
    let f = fit(p, &text, small_font(u), r.width() - 4.0);
    p.text(r.center(), Align2::CENTER_CENTER, text, f, if on { text_on(fill) } else { well_ink });
    resp
}

/// A value to click through: click for the next, right-click for the one
/// before. Returns whether it changed.
fn cycler(ui: &mut Ui, r: Rect, id: Id, sp: &ParamSpec, v: &mut f32, label: &str, u: f32) -> bool {
    let p = ui.painter().clone();
    let label = crate::skin::label_text(label);
    let lf = fit(&p, &label, small_font(u), r.width());
    let lh = lf.size + 3.0;
    p.text(pos2(r.center().x, r.top() + lh * 0.5), Align2::CENTER_CENTER, label, lf, crate::skin::ink());
    let b = Rect::from_min_max(pos2(r.left() + 1.0, r.top() + lh), pos2(r.right() - 1.0, (r.top() + lh + u * 1.3).min(r.bottom())));
    let resp = ui.interact(b, id, Sense::click());
    let n = (sp.max - sp.min).round() as i32 + 1;
    let cur = (*v - sp.min).round() as i32;
    let mut next = cur;
    if resp.clicked() {
        next = (cur + 1).rem_euclid(n);
    }
    if resp.secondary_clicked() {
        next = (cur - 1).rem_euclid(n);
    }
    screen_box(&p, b, u);
    let text = sp.format(sp.min + next as f32);
    let f = fit(&p, &text, crate::skin::Font::Lcd.id((b.height() * 0.5).clamp(9.0, 15.0)), b.width() - 4.0);
    p.text(b.center(), Align2::CENTER_CENTER, text, f, accent_now());
    resp.on_hover_text(format!("{}: click for the next, right-click for the one before", sp.name));
    if next != cur {
        *v = sp.min + next as f32;
        true
    } else {
        false
    }
}

/// The parameters as the face plays them, with the modulation shown on them.
struct Knobs<'a> {
    id: Id,
    params: &'a mut [f32],
    mv: Option<&'a mut ModView>,
    u: f32,
}

impl Knobs<'_> {
    fn knob(&mut self, ui: &mut Ui, r: Rect, pi: usize, label: &str) {
        let sp = spec(pi);
        let id = self.id.with(pi);
        let mut v = self.params[pi];
        if knob(ui, r, id, sp, &mut v, label, self.u) {
            self.params[pi] = v;
        }
        if let Some(mv) = self.mv.as_deref_mut() {
            knob_overlay(ui, r, knob_rect(r, self.u), id.with("mod"), sp, self.params[pi], pi, mv);
        }
    }

    fn cycler(&mut self, ui: &mut Ui, r: Rect, pi: usize, label: &str) {
        let mut v = self.params[pi];
        if cycler(ui, r, self.id.with(pi), spec(pi), &mut v, label, self.u) {
            self.params[pi] = v;
        }
    }

    fn toggle(&mut self, ui: &mut Ui, r: Rect, pi: usize, label: &str) {
        let on = self.params[pi] >= 0.5;
        let b = Rect::from_center_size(r.center(), vec2(r.width() - 2.0, (self.u * 1.4).min(r.height())));
        if led_button(ui, b, self.id.with(pi), label, on, false, self.u).on_hover_text(spec(pi).name).clicked() {
            self.params[pi] = if on { 0.0 } else { 1.0 };
        }
    }
}

/// The name of a step's note: the 808's as a note, the rest in semitones.
fn note_text(params: &[f32], key: Scale, t: usize, s: Step) -> String {
    let voice = params[pm::tp(t, pm::T_VOICE)].round() as usize;
    if voice == 0 {
        note_name(midi_808(params, key, s.note) as u8)
    } else if s.note == 0 {
        String::new()
    } else {
        format!("{:+}", s.note)
    }
}

fn note_name(n: u8) -> String {
    krabton_engine::note_name(n)
}

/// The MIDI note an 808 step plays.
fn midi_808(params: &[f32], key: Scale, note: i8) -> i32 {
    let in_key = params[pm::SCALE] >= 0.5;
    let root = if in_key { key.root as i32 } else { params[pm::ROOT].round() as i32 };
    let m = (24 + root.rem_euclid(12) + note as i32).clamp(0, 127);
    if in_key && key.kind != krabton_engine::ScaleKind::Chromatic {
        key.snap(m as u8) as i32
    } else {
        m
    }
}

/// A step's note one up (`dir` 1) or down: in the song's key for an 808 in
/// Song Key, else a semitone.
fn step_note(params: &[f32], key: Scale, t: usize, note: i8, dir: i32) -> i8 {
    let voice = params[pm::tp(t, pm::T_VOICE)].round() as usize;
    if voice == 0 && params[pm::SCALE] >= 0.5 && key.kind != krabton_engine::ScaleKind::Chromatic {
        let now = midi_808(params, key, note);
        let next = key.step(now.clamp(0, 127) as u8, dir) as i32;
        return (next - (24 + key.root as i32)).clamp(-24, 36) as i8;
    }
    (note as i32 + dir).clamp(-24, 36) as i8
}

/// Analog Princess's panel.
#[allow(clippy::too_many_arguments)]
pub(crate) fn panel(ui: &mut Ui, id: Id, preset: &mut String, params: &mut Vec<f32>, machine: &mut Machine, live: &InstLive, key: Scale, height: f32, mv: Option<&mut ModView>) {
    if params.len() < pm::SPECS.len() {
        return;
    }
    machine.fix();
    let u = (height / H).clamp(10.0, 24.0);
    let (rect, _) = ui.allocate_exact_size(vec2(W * u, H * u), Sense::hover());
    let at = |x: f32, y: f32, w: f32, h: f32| Rect::from_min_size(rect.min + vec2(x * u, y * u), vec2(w * u, h * u));
    let p = ui.painter().clone();
    let vid = id.with("view");
    let mut st: View = ui.ctx().data(|d| d.get_temp(vid)).unwrap_or_default();
    st.track = st.track.min(TRACKS - 1);
    section(&p, at(PAT_X, 0.0, PAT_W, H), "Patterns", u);
    section(&p, at(SEQ_X, 0.0, SEQ_W, H), "Sequencer", u);
    section(&p, at(SND_X, 0.0, SND_W, H), "Sound", u);
    section(&p, at(FRY_X, 0.0, FRY_W, H), "Fry", u);
    let playing = live.vals[32] > 0.5;
    let sel = (params[pm::PATTERN].round().max(0.0) as usize).min(pm::PATTERNS - 1);

    // ---------------------------------------------------------- patterns
    let x0 = PAT_X + 0.3;
    if let Some(i) = names_display(ui, at(x0, 0.3, PAT_W - 0.6, 1.5), id.with("preset"), pm::preset_names(), preset.as_str(), u) {
        let name = pm::preset_names()[i];
        *preset = name.to_string();
        *params = pm::preset(name);
        *machine = pm::preset_machine(name);
    }
    let bw = (PAT_W - 0.6) / 4.0;
    let chain_on = params[pm::CHAIN] >= 0.5;
    let playing_pat = live.vals[0].round() as usize;
    for k in 0..pm::PATTERNS {
        let r = at(x0 + (k % 4) as f32 * bw, 2.2 + (k / 4) as f32 * 1.9, bw, 1.9).shrink(0.12 * u);
        let empty = machine.patterns[k].is_empty();
        let lit = playing && playing_pat == k;
        let resp = led_button(ui, r, id.with(("pat", k)), pm::PATTERN_NAMES[k], k == sel, lit, u);
        if empty && k != sel {
            p.text(r.right_top() + vec2(-3.0, 2.0), Align2::RIGHT_TOP, "-", small_font(u), crate::skin::ink_dim());
        }
        let resp = resp.on_hover_text(format!("Pattern {}: click to play and edit it (it starts when the one playing ends), right-click to add it to the chain", pm::PATTERN_NAMES[k]));
        if resp.clicked() {
            params[pm::PATTERN] = k as f32;
        }
        if resp.secondary_clicked() && machine.chain.len() < pm::MAX_CHAIN {
            machine.chain.push(k as u8);
        }
    }
    // The chain: on or off, and what it plays.
    let cr = at(x0, 6.1, 2.4, 1.4);
    if led_button(ui, cr, id.with("chain"), "Chain", chain_on, false, u).on_hover_text("Play the chain instead of one pattern").clicked() {
        params[pm::CHAIN] = if chain_on { 0.0 } else { 1.0 };
    }
    let list = at(x0 + 2.6, 6.1, PAT_W - 0.6 - 2.6, 1.4);
    screen_box(&p, list, u);
    let resp = ui.interact(list, id.with("chain_list"), Sense::click()).on_hover_text("The chain: right-click a pattern to add it; click here to take the last one off, right-click to empty it");
    if resp.clicked() {
        machine.chain.pop();
    }
    if resp.secondary_clicked() {
        machine.chain.clear();
    }
    {
        let n = machine.chain.len().max(1);
        let cw = ((list.width() - 6.0) / n as f32).min(u * 0.9);
        let now = live.vals[2].round() as usize;
        for (i, &c) in machine.chain.iter().enumerate() {
            let pos = pos2(list.left() + 3.0 + (i as f32 + 0.5) * cw, list.center().y);
            let hot = chain_on && playing && i == now;
            let f = crate::skin::Font::Lcd.id((cw * 1.1).clamp(7.0, list.height() * 0.6));
            p.text(pos, Align2::CENTER_CENTER, pm::PATTERN_NAMES[c as usize % pm::PATTERNS], f, if hot { Color32::WHITE } else { accent_now() });
        }
        if machine.chain.is_empty() {
            p.text(list.center(), Align2::CENTER_CENTER, "EMPTY", small_font(u), shade(accent_now(), 0.5));
        }
    }
    // The pattern's length.
    let pat_len = machine.patterns[sel].length;
    for (k, len) in [16u8, 32, 64].into_iter().enumerate() {
        let r = at(x0 + k as f32 * (PAT_W - 0.6) / 3.0, 7.8, (PAT_W - 0.6) / 3.0, 1.3).shrink(0.1 * u);
        if led_button(ui, r, id.with(("len", k)), &len.to_string(), pat_len == len, false, u).on_hover_text(format!("{len} steps")).clicked() {
            machine.patterns[sel].length = len;
        }
    }
    // What to do with the pattern.
    let seed = (ui.input(|i| i.time) * 1000.0) as u32;
    let voices: [usize; TRACKS] = std::array::from_fn(|t| params[pm::tp(t, pm::T_VOICE)].round() as usize);
    let acts = ["Copy", "Paste", "Clear", "Random", "Human", "Seq"];
    for (k, a) in acts.iter().enumerate() {
        let r = at(x0 + (k % 3) as f32 * (PAT_W - 0.6) / 3.0, 9.3 + (k / 3) as f32 * 1.35, (PAT_W - 0.6) / 3.0, 1.35).shrink(0.1 * u);
        let on = k == 5 && params[pm::SEQUENCER] >= 0.5;
        let tip = match k {
            0 => "Copy this pattern",
            1 => "Paste the pattern copied over this one",
            2 => "Empty this pattern",
            3 => "Make up a new pattern",
            4 => "Move its velocities a little, as a hand plays",
            _ => "The sequencer: off, only notes play it",
        };
        if led_button(ui, r, id.with(("act", k)), a, on, false, u).on_hover_text(tip).clicked() {
            let pat = &mut machine.patterns[sel];
            match k {
                0 => st.clip = Some(pat.clone()),
                1 => {
                    if let Some(c) = &st.clip {
                        *pat = c.clone();
                    }
                }
                2 => pat.clear(None),
                3 => pat.randomise(None, &voices, seed),
                4 => pat.humanise(None, seed),
                _ => params[pm::SEQUENCER] = if on { 0.0 } else { 1.0 },
            }
        }
    }
    let mut kn = Knobs { id, params: &mut params[..], mv, u };
    let kw = (PAT_W - 0.6) / 3.0;
    kn.knob(ui, at(x0, 12.0, kw, 2.6), pm::SWING, "Swing");
    kn.knob(ui, at(x0 + kw, 12.0, kw, 2.6), pm::FLAM, "Flam");
    kn.knob(ui, at(x0 + 2.0 * kw, 12.0, kw, 2.6), pm::ROOM_SIZE, "Room");

    // ---------------------------------------------------------- tracks
    let sx = SEQ_X + 0.3;
    let tw = (SEQ_W - 0.6) / TRACKS as f32;
    for t in 0..TRACKS {
        let r = at(sx + t as f32 * tw, 0.3, tw, 1.9).shrink(0.1 * u);
        let resp = ui.interact(r, id.with(("track", t)), Sense::click());
        let muted = kn.params[pm::tp(t, pm::T_MUTE)] >= 0.5;
        let col = track_colour(t);
        let (well, well_ink) = crate::skin::well();
        let on = t == st.track;
        let fill = if on { mix(well, col, 0.6) } else if resp.hovered() { mix(well, Color32::WHITE, 0.1) } else { well };
        p.rect_filled(r.translate(vec2(0.0, 1.5)), 3.0, Color32::from_black_alpha(100));
        p.rect_filled(r, 3.0, fill);
        p.rect_filled(Rect::from_min_size(r.left_bottom() - vec2(0.0, 3.0), vec2(r.width(), 3.0)), 1.5, if muted { shade(col, 0.3) } else { col });
        // A light that flashes with its hits.
        let flash = live.vals[8 + t].clamp(0.0, 1.0);
        let led = r.right_top() + vec2(-u * 0.3, u * 0.3);
        p.circle_filled(led, (u * 0.16).max(2.5), mix(Color32::from_black_alpha(160), col, flash));
        let name = crate::skin::label_text(pm::TRACK_NAMES[t]);
        let f = fit(&p, &name, small_font(u), r.width() - 4.0);
        let ink = if on { text_on(fill) } else if muted { shade(well_ink, 0.45) } else { well_ink };
        p.text(r.center() + vec2(0.0, -1.0), Align2::CENTER_CENTER, name, f, ink);
        if muted {
            p.text(r.left_top() + vec2(3.0, 2.0), Align2::LEFT_TOP, "M", small_font(u * 0.8), REC);
        }
        let resp = resp.on_hover_text(format!("{}: click to edit it, right-click to mute it", pm::TRACK_NAMES[t]));
        if resp.clicked() {
            st.track = t;
        }
        if resp.secondary_clicked() {
            kn.params[pm::tp(t, pm::T_MUTE)] = if muted { 0.0 } else { 1.0 };
        }
    }

    // ---------------------------------------------------------- steps
    let t = st.track;
    let col = track_colour(t);
    let pattern = &mut machine.patterns[sel];
    let len = pattern.len();
    let tlen = pattern.track_len(t);
    let pages = len.max(tlen).div_ceil(16);
    st.page = st.page.min(pages - 1);
    let lx = sx + 2.4;
    let pw = (SEQ_W - 0.6 - 2.4) / 16.0;
    let shown_step = if playing && playing_pat == sel { live.vals[20 + t].round() as i32 } else { -1 };
    let (well, well_ink) = crate::skin::well();
    let is_808 = kn.params[pm::tp(t, pm::T_VOICE)].round() as usize == 0;
    for c in 0..16 {
        let i = st.page * 16 + c;
        let r = at(lx + c as f32 * pw, 2.5, pw, 3.6).shrink(0.13 * u);
        let inside = i < tlen;
        let s = pattern.lanes[t].steps[i];
        let resp = ui.interact(r, id.with(("pad", i)), Sense::click_and_drag());
        let beat = c % 4 == 0;
        let base = if beat { mix(well, Color32::WHITE, 0.07) } else { well };
        let base = if inside { base } else { shade(base, 0.5) };
        p.rect_filled(r.translate(vec2(0.0, 2.0)), 4.0, Color32::from_black_alpha(120));
        p.rect_filled(r, 4.0, base);
        if s.on {
            let v = s.velocity();
            let lit = mix(base, col, 0.35 + 0.45 * v);
            p.rect_filled(r, 4.0, lit);
            // The velocity as how far up it is filled.
            let fill = Rect::from_min_max(pos2(r.left(), r.bottom() - r.height() * v), r.max);
            p.rect_filled(fill, 4.0, mix(lit, col, 0.6));
            if s.accent {
                p.rect_stroke(r.shrink(1.5), 3.0, Stroke::new(2.0_f32, Color32::WHITE), StrokeKind::Inside);
            }
        }
        if shown_step == i as i32 {
            p.rect_stroke(r.expand(1.5), 5.0, Stroke::new(2.5_f32, Color32::WHITE), StrokeKind::Outside);
            p.rect_filled(r, 4.0, Color32::from_white_alpha(40));
        }
        let ink = if s.on { text_on(col) } else { shade(well_ink, 0.5) };
        p.text(r.left_top() + vec2(3.0, 2.0), Align2::LEFT_TOP, (i + 1).to_string(), small_font(u * 0.85), ink);
        if s.on {
            let mut marks = String::new();
            if s.roll > 0 {
                marks.push_str(match s.roll {
                    6 => "UP",
                    7 => "DN",
                    r => pm::ROLLS[r as usize],
                });
            }
            if s.flam {
                marks.push_str(" F");
            }
            if !marks.is_empty() {
                let text = if s.roll > 0 && s.roll < 6 { format!("x{}", marks.trim()) } else { marks.trim().to_string() };
                p.text(pos2(r.center().x, r.top() + r.height() * 0.38), Align2::CENTER_CENTER, text, FontId::proportional((u * 0.62).clamp(9.0, 15.0)), ink);
            }
            let nt = note_text(kn.params, key, t, s);
            let bottom = if s.prob < 100 { format!("{nt} {}%", s.prob) } else { nt };
            if !bottom.trim().is_empty() {
                let f = fit(&p, bottom.trim(), small_font(u * 0.9), r.width() - 2.0);
                p.text(pos2(r.center().x, r.bottom() - 3.0), Align2::CENTER_BOTTOM, bottom.trim(), f, ink);
            }
            if s.slide {
                p.line_segment([pos2(r.right() - u * 0.5, r.bottom() - u * 0.9), pos2(r.right() + u * 0.2, r.bottom() - u * 1.4)], Stroke::new(2.0_f32, Color32::WHITE));
            }
        }
        let step = &mut pattern.lanes[t].steps[i];
        if resp.clicked() {
            if step.on {
                step.on = false;
            } else {
                if step.vel == 0 {
                    *step = Step { note: step.note, ..Step::hit(100) };
                }
                step.on = true;
            }
        }
        if resp.secondary_clicked() {
            if !step.on {
                *step = Step { note: step.note, ..Step::hit(100) };
            }
            step.roll = (step.roll + 1) % pm::ROLLS.len() as u8;
        }
        if resp.dragged() && step.on {
            let dv = -resp.drag_delta().y * 127.0 / r.height().max(1.0);
            step.vel = (step.vel as f32 + dv).round().clamp(1.0, 127.0) as u8;
        }
        resp.on_hover_text(format!("Step {}: click to set it, right-click for the next roll, drag up or down for its velocity", i + 1));
    }
    // The steps' rows.
    let lh = 0.93;
    for (row, name) in LANES.iter().enumerate() {
        let y = 6.35 + row as f32 * lh;
        let label = at(sx, y, 2.3, lh);
        let text = crate::skin::label_text(name);
        let f = fit(&p, &text, small_font(u), label.width() - 4.0);
        p.text(label.right_center() - vec2(4.0, 0.0), Align2::RIGHT_CENTER, text, f, crate::skin::ink_dim());
        for c in 0..16 {
            let i = st.page * 16 + c;
            let r = at(lx + c as f32 * pw, y, pw, lh).shrink(0.07 * u);
            let s = pattern.lanes[t].steps[i];
            let resp = ui.interact(r, id.with(("lane", row, i)), Sense::click_and_drag());
            let hot = resp.hovered();
            p.rect_filled(r, 2.0, if hot { mix(well, Color32::WHITE, 0.1) } else { well });
            let dim = !s.on;
            let ink = if dim { shade(well_ink, 0.35) } else { well_ink };
            let font = small_font(u * 0.9);
            let step = &mut pattern.lanes[t].steps[i];
            let (clicked, back) = (resp.clicked(), resp.secondary_clicked());
            match row {
                0 => {
                    // Velocity, as a bar to drag.
                    let v = if s.on { s.velocity() } else { 0.0 };
                    let bar = Rect::from_min_max(pos2(r.left() + 1.0, r.bottom() - 1.0 - (r.height() - 2.0) * v), pos2(r.right() - 1.0, r.bottom() - 1.0));
                    p.rect_filled(bar, 1.5, col);
                    if (resp.is_pointer_button_down_on() || resp.dragged()) && step.on {
                        if let Some(pos) = resp.interact_pointer_pos() {
                            step.vel = ((1.0 - (pos.y - r.top()) / r.height()) * 127.0).round().clamp(1.0, 127.0) as u8;
                        }
                    }
                }
                1 => {
                    if s.on && s.roll > 0 {
                        p.text(r.center(), Align2::CENTER_CENTER, pm::ROLLS[s.roll as usize], font.clone(), ink);
                    }
                    let n = pm::ROLLS.len() as i32;
                    if step.on && (clicked || back) {
                        step.roll = (step.roll as i32 + if clicked { 1 } else { -1 }).rem_euclid(n) as u8;
                    }
                }
                2 => {
                    if s.on && s.prob < 100 {
                        p.text(r.center(), Align2::CENTER_CENTER, format!("{}", s.prob), font.clone(), ink);
                    }
                    let k = pm::CHANCES.iter().position(|&c| c == step.prob).unwrap_or(0) as i32;
                    let n = pm::CHANCES.len() as i32;
                    if step.on && (clicked || back) {
                        step.prob = pm::CHANCES[(k + if clicked { 1 } else { -1 }).rem_euclid(n) as usize];
                    }
                }
                3 | 4 | 6 => {
                    let on = match row {
                        3 => s.accent,
                        4 => s.flam,
                        _ => s.slide,
                    };
                    if on && s.on {
                        p.circle_filled(r.center(), (r.height() * 0.28).max(2.5), col);
                    }
                    if step.on && (clicked || back) {
                        match row {
                            3 => step.accent = !step.accent,
                            4 => step.flam = !step.flam,
                            _ => step.slide = !step.slide,
                        }
                    }
                }
                _ => {
                    let nt = note_text(kn.params, key, t, s);
                    if s.on && !nt.is_empty() {
                        let f = fit(&p, &nt, font.clone(), r.width() - 2.0);
                        p.text(r.center(), Align2::CENTER_CENTER, nt, f, ink);
                    }
                    if step.on && (clicked || back) {
                        step.note = step_note(kn.params, key, t, step.note, if clicked { 1 } else { -1 });
                    }
                }
            }
            let tip = match row {
                0 => "Velocity: drag in it",
                1 => "Roll: 2 or 4 hits in the step, 3 or 6 over two (triplets), 8 over two, or ramping up or down over a beat. Click for the next, right-click for the one before",
                2 => "Chance it plays (%): click for less, right-click for more",
                3 => "Accent: louder and driven harder",
                4 => "Flam: a soft hit just before",
                5 if is_808 => "Note: click a step up, right-click a step down (in the song's key with Scale on Song Key)",
                5 => "Note: semitones up (click) or down (right-click)",
                _ => "Slide: glide into the next step's note without a new hit",
            };
            resp.on_hover_text(tip);
        }
    }
    // Pages, the track's own length and what to do with the track.
    let by = 6.35 + LANES.len() as f32 * lh + 0.15;
    for k in 0..4 {
        let r = at(sx + k as f32 * 1.6, by, 1.6, 0.95).shrink(0.07 * u);
        let has = k < pages;
        let resp = led_button(ui, r, id.with(("page", k)), &format!("{}", k + 1), k == st.page && has, false, u);
        if has && resp.on_hover_text(format!("Steps {} to {}", k * 16 + 1, k * 16 + 16)).clicked() {
            st.page = k;
        }
    }
    {
        let r = at(sx + 6.6, by, 4.6, 0.95).shrink(0.07 * u);
        let own = pattern.lanes[t].length;
        let k = TRACK_LENS.iter().position(|&l| l == own).unwrap_or(0) as i32;
        let text = if own == 0 { "Steps: pattern".to_string() } else { format!("Steps: {own}") };
        let resp = led_button(ui, r, id.with("track_len"), &text, own > 0, false, u).on_hover_text("How many steps this track goes round in, its own (a polymeter) or the pattern's: click for more, right-click for fewer");
        let n = TRACK_LENS.len() as i32;
        if resp.clicked() {
            pattern.lanes[t].length = TRACK_LENS[(k + 1).rem_euclid(n) as usize];
        }
        if resp.secondary_clicked() {
            pattern.lanes[t].length = TRACK_LENS[(k - 1).rem_euclid(n) as usize];
        }
    }
    for (k, a) in ["Clear", "Random", "Human"].iter().enumerate() {
        let r = at(sx + 11.6 + k as f32 * 2.9, by, 2.9, 0.95).shrink(0.07 * u);
        if led_button(ui, r, id.with(("track_act", k)), a, false, false, u).on_hover_text(format!("{a}: this track only")).clicked() {
            match k {
                0 => pattern.clear(Some(t)),
                1 => pattern.randomise(Some(t), &voices, seed),
                _ => pattern.humanise(Some(t), seed),
            }
        }
    }
    let info = at(sx + 20.6, by, SEQ_W - 0.6 - 20.6, 0.95);
    let text = format!("{} · PATTERN {} · {} STEPS", pm::TRACK_NAMES[t].to_uppercase(), pm::PATTERN_NAMES[sel], len);
    let f = fit(&p, &text, small_font(u), info.width());
    p.text(info.right_center(), Align2::RIGHT_CENTER, text, f, crate::skin::ink_dim());

    // ---------------------------------------------------------- sound
    let x = SND_X + 0.3;
    let name = crate::skin::label_text(pm::TRACK_NAMES[t]);
    let nr = at(x, 0.3, 3.6, 1.5);
    let f = fit(&p, &name, FontId::proportional((u * 0.9).clamp(11.0, 20.0)), nr.width());
    p.text(nr.left_center(), Align2::LEFT_CENTER, name, f, col);
    let voice = kn.params[pm::tp(t, pm::T_VOICE)].round().max(0.0) as usize;
    let vr = at(x + 3.7, 0.3, if is_808 { 9.0 } else { 15.0 }, 1.5);
    if let Some(v) = names_display(ui, vr, id.with(("voice", t)), pm::VOICES, pm::VOICES[voice.min(pm::VOICES.len() - 1)], u) {
        kn.params[pm::tp(t, pm::T_VOICE)] = v as f32;
    }
    if is_808 {
        kn.cycler(ui, at(x + 13.0, 0.1, 3.4, 1.9), pm::ROOT, "Root");
        kn.cycler(ui, at(x + 16.6, 0.1, 6.0, 1.9), pm::SCALE, "Scale");
    }
    let [ka, kb] = pm::voice_knobs(voice);
    let cw = (SND_W - 0.6) / 8.0;
    let ch = 4.1;
    let rows: [[(usize, &str); 8]; 2] = [
        [(pm::T_TUNE, "Tune"), (pm::T_DECAY, "Decay"), (pm::T_TONE, if is_808 { "Heat" } else { "Tone" }), (pm::T_A, ka), (pm::T_B, kb), (pm::T_PITCH, "Pitch"), (pm::T_PITCH_TIME, "P.Time"), (pm::T_LEVEL, "Level")],
        [(pm::T_DRIVE, "Drive"), (pm::T_CLIP, "Clip"), (pm::T_RING, "Ring"), (pm::T_RING_HZ, "Ring Hz"), (pm::T_BITS, "Bits"), (pm::T_RATE, "Down"), (pm::T_FILTER, "LP | HP"), (pm::T_PAN, "Pan")],
    ];
    for (ri, row) in rows.iter().enumerate() {
        for (ci, (k, label)) in row.iter().enumerate() {
            kn.knob(ui, at(x + ci as f32 * cw, 2.0 + ri as f32 * ch, cw, ch), pm::tp(t, *k), label);
        }
    }
    let y3 = 2.0 + 2.0 * ch;
    kn.knob(ui, at(x, y3, cw, ch), pm::tp(t, pm::T_ROOM), "Room");
    kn.knob(ui, at(x + cw, y3, cw, ch), pm::tp(t, pm::T_FRY), "Fry");
    let x3 = x + 2.0 * cw;
    if is_808 {
        kn.knob(ui, at(x3, y3, cw, ch), pm::GLIDE, "Glide");
        kn.cycler(ui, at(x3 + cw, y3, cw, ch), pm::DIST, "Dist");
        kn.knob(ui, at(x3 + 2.0 * cw, y3, cw, ch), pm::SUB_808, "Sub");
        kn.knob(ui, at(x3 + 3.0 * cw, y3, cw, ch), pm::CLICK, "Click");
        kn.knob(ui, at(x3 + 4.0 * cw, y3, cw, ch), pm::RING_TUNE, "Ring St");
        kn.knob(ui, at(x3 + 5.0 * cw, y3, cw, ch), pm::RING_DECAY, "Ring Dec");
    } else {
        kn.cycler(ui, at(x3, y3, 2.0 * cw, ch), pm::tp(t, pm::T_CHOKE), "Choke");
        kn.toggle(ui, at(x3 + 2.0 * cw, y3 + 0.6, 1.6 * cw, 1.6), pm::tp(t, pm::T_MUTE), "Mute");
        let hint = at(x3 + 3.8 * cw, y3 + 0.3, 2.2 * cw, ch);
        let text = "a choke group cuts the others in it (the hats)";
        let gal = p.layout(text.to_string(), small_font(u * 0.85), crate::skin::ink_dim(), hint.width());
        p.galley(hint.left_top(), gal, crate::skin::ink_dim());
    }

    // ---------------------------------------------------------- fry
    let fx = FRY_X + 0.3;
    kn.knob(ui, at(fx, 0.2, 5.4, 5.6), pm::DEEP_FRY, "Deep Fry");
    let w7 = (FRY_W - 0.6 - 5.6) / 7.0;
    let row: [(usize, &str, bool); 7] = [(pm::F_DRIVE, "Drive", false), (pm::F_CLIP, "Clip", false), (pm::F_CLIP_MODE, "Mode", true), (pm::F_BITS, "Bits", false), (pm::F_DOWN, "Down", false), (pm::F_RING, "Ring", false), (pm::F_RING_HZ, "Ring Hz", false)];
    let w10 = (FRY_W - 0.6) / 10.0;
    let row2: [(usize, &str, bool); 10] = [
        (pm::PUMP, "Pump", false),
        (pm::PUMP_TIME, "Release", false),
        (pm::PUMP_KEY, "Key", true),
        (pm::STUTTER, "Stutter", false),
        (pm::STUTTER_RATE, "Rate", true),
        (pm::TAPE, "Tape", false),
        (pm::TAPE_TIME, "Time", true),
        (pm::CHANNEL_SURF, "Surf", false),
        (pm::CEILING, "Ceiling", false),
        (pm::VOLUME, "Volume", false),
    ];
    let cells = row.iter().enumerate().map(|(k, c)| (at(fx + 5.6 + k as f32 * w7, 0.4, w7, 5.2), c)).chain(row2.iter().enumerate().map(|(k, c)| (at(fx + k as f32 * w10, 6.0, w10, 4.0), c)));
    for (r, (pi, label, cyc)) in cells {
        if *cyc {
            kn.cycler(ui, Rect::from_min_size(r.min + vec2(0.0, r.height() * 0.2), vec2(r.width(), r.height() * 0.6)), *pi, label);
        } else {
            kn.knob(ui, r, *pi, label);
        }
    }
    // How much the pump and the limiter take.
    let hw = (FRY_W - 0.6 - 4.0) / 3.0;
    let meters = at(fx + 3.0 * hw, 10.2, FRY_W - 0.6 - 3.0 * hw, 3.9).shrink(0.15 * u);
    for (k, (v, name)) in [(live.vals[3], "PUMP"), (live.vals[4], "LIMIT")].into_iter().enumerate() {
        let r = Rect::from_min_size(pos2(meters.left() + k as f32 * meters.width() * 0.5, meters.top()), vec2(meters.width() * 0.5, meters.height())).shrink(2.0);
        let bar = Rect::from_min_max(pos2(r.left(), r.top()), pos2(r.right(), r.bottom() - u * 0.6));
        screen_box(&p, bar, u);
        let h = bar.height() * v.clamp(0.0, 1.0);
        p.rect_filled(Rect::from_min_max(bar.min, pos2(bar.right(), bar.top() + h)), 1.0, accent_now());
        let f = fit(&p, name, small_font(u * 0.75), r.width() + 4.0);
        p.text(pos2(r.center().x, r.bottom()), Align2::CENTER_BOTTOM, name, f, crate::skin::ink_dim());
    }
    // The buttons to hold: on while held, right-click to leave one on.
    let holds = [(pm::STUTTER_HOLD, "Stutter", 5usize), (pm::TAPE_HOLD, "Tape Stop", 6), (pm::SURF_HOLD, "Surf", 7)];
    for (k, (pi, label, light)) in holds.into_iter().enumerate() {
        let r = at(fx + k as f32 * hw, 10.4, hw, 3.6).shrink(0.15 * u);
        let resp = ui.interact(r, id.with(("hold", k)), Sense::click_and_drag());
        if resp.secondary_clicked() {
            st.latched[k] = !st.latched[k];
        }
        let down = resp.is_pointer_button_down_on() && ui.input(|i| i.pointer.primary_down());
        let want = if down || st.latched[k] { 1.0 } else { 0.0 };
        if kn.params[pi] != want {
            kn.params[pi] = want;
        }
        let active = live.vals[light] > 0.5 || want > 0.5;
        let c = track_colour([0, 2, 4][k]);
        p.rect_filled(r.translate(vec2(0.0, 2.5)), 6.0, Color32::from_black_alpha(130));
        p.rect_filled(r, 6.0, if active { c } else if resp.hovered() { mix(well, c, 0.25) } else { mix(well, c, 0.12) });
        if st.latched[k] {
            p.rect_stroke(r.expand(1.5), 6.0, Stroke::new(2.0_f32, Color32::WHITE), StrokeKind::Outside);
        }
        let text = crate::skin::label_text(label).to_uppercase();
        let f = fit(&p, &text, FontId::proportional((u * 0.75).clamp(10.0, 18.0)), r.width() - 6.0);
        p.text(r.center(), Align2::CENTER_CENTER, text, f, if active { text_on(c) } else { crate::skin::well().1 });
        resp.on_hover_text(format!("{label}: hold it down, or right-click to leave it on"));
    }
    drop(kn);
    ui.ctx().data_mut(|d| d.insert_temp(vid, st));
    if playing || live.vals[8..20].iter().any(|v| *v > 0.01) {
        ui.ctx().request_repaint();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_step_goes_up_the_songs_key() {
        let mut params = pm::preset("Init");
        let key = Scale::new(9, krabton_engine::ScaleKind::Minor);
        params[pm::SCALE] = 1.0;
        // From A (the root) up the key: B, C, D.
        let mut n = 0i8;
        let mut seen = Vec::new();
        for _ in 0..3 {
            n = step_note(&params, key, pm::BASS, n, 1);
            seen.push(midi_808(&params, key, n) % 12);
        }
        assert_eq!(seen, vec![11, 0, 2]);
        assert_eq!(midi_808(&params, key, step_note(&params, key, pm::BASS, 0, -1)) % 12, 7);
        // Chromatic: a semitone.
        params[pm::SCALE] = 0.0;
        assert_eq!(step_note(&params, key, pm::BASS, 0, 1), 1);
        assert_eq!(note_text(&params, key, pm::BASS, Step::hit(100)), "C0");
        assert_eq!(note_text(&params, key, pm::HAT, Step { note: 3, ..Step::hit(100) }), "+3");
    }
}

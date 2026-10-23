//! The Touch view: the selected track played with plates, made to play with
//! (see `sequencers::touch`). Along the top: how it plays (notes, an
//! arpeggio, strummed, echoing; one note or a chord per plate; the pulser
//! with swing, Latch and Run, chance and wander), the sixteen-step sequencer
//! (switched on and off, Trigger and Pitch apart; each step a note, one to
//! eight pulses and a gate for what they play, a value, chance or ratchet,
//! tie, accent and skip; the note and the value drawn, the rest clicked
//! (a click one further, a right-click one back, dragged across: copied);
//! with
//! dice, mutate, shift, reverse, clear and a recorder that writes the plates
//! played into the steps), and the patch bay:
//! cables from ten outputs to eight of the instrument's knobs, each with its
//! amount. Along the bottom the plates (a row, wide, or a grid of rows a
//! fourth apart; in the key, pentatonic or chromatic; or the Tonnetz, its
//! triangles chords to press and the steps' chords): higher up a plate
//! presses harder, the letter keys and fingers on a touch screen press them
//! too; with Slide on, a plate pressed and the finger moved along bends its
//! note smoothly to wherever the finger is (otherwise dragging across plays
//! plate after plate). Beside them the motion: an XY pad, the LFO,
//! and a shape to draw (or to record with the pad). The pulser's controls
//! are ladders of values to click; the envelope's are sliders (knobs with
//! the *Sliders* pref off).
//!
//! *Record* (as the transport's) records what the plates play into a clip;
//! *Capture* makes a clip of what was just played, recorded or not.
//!
//! It works on the track's Touch device (put first in its MIDI chain when the
//! view first shows the track) and on Touch modulators routed to the
//! instrument, so what is played and patched here is saved, automatable and
//! seen in the Mod tab as well.

use crate::app::{App, ClipSel, TrackSel};
use crate::theme::*;
use crate::widgets::*;
use egui::epaint::CubicBezierShape;
use egui::{pos2, vec2, Align2, Color32, CursorIcon, FontId, Id, Mesh, Pos2, Rect, Sense, Stroke, Ui};
use krabton_engine::engine::Command;
use krabton_engine::midi_fx::MidiFxKind;
use krabton_engine::modulation::{instrument_specs, ModKind, ModOwner, ModTarget, MAX_MODS};
use krabton_engine::params::{ParamKind, ParamSpec};
use krabton_engine::project::TrackKind;
use krabton_engine::{Clip, Note};
use krabton_engine::sequencers::touch::{self, Gate, Step, Tri, Turn, MAX_PULSES, MAX_RATCHET, N_OUT, N_PATCH, N_SHAPE, N_STEPS, OUTPUTS};

/// The colours of the outputs, and of their cables.
pub(crate) const OUT_COLORS: [Color32; N_OUT] = [
    Color32::from_rgb(0xff, 0x9a, 0x3c),
    Color32::from_rgb(0xff, 0x5a, 0x7a),
    Color32::from_rgb(0xf5, 0xd0, 0x4a),
    Color32::from_rgb(0x58, 0xd0, 0x8a),
    Color32::from_rgb(0x4f, 0xc8, 0xf0),
    Color32::from_rgb(0xb0, 0x8c, 0xff),
    Color32::from_rgb(0xc8, 0xf0, 0x5a),
    Color32::from_rgb(0xf0, 0xf0, 0xf4),
    Color32::from_rgb(0x3c, 0xe0, 0xc8),
    Color32::from_rgb(0xff, 0x6a, 0xe0),
    Color32::from_rgb(0xff, 0xc8, 0x8a),
    Color32::from_rgb(0x8a, 0xb4, 0xff),
];

/// What the outputs are, for their tooltips.
const OUT_HELP: [&str; N_OUT] = [
    "How hard the plates are pressed (higher up a plate: harder)",
    "The pulse: an envelope on every pulse (or every touch), as long as Decay",
    "The steps' values (the sequencer's Value lane)",
    "A random value, new on every pulse (or touch)",
    "A random value drifting slowly from one to the next",
    "Where along the plates the hand is, low to high",
    "The LFO (its shape and rate under Motion)",
    "The shape drawn under Motion, looping at its rate",
    "The XY pad, left to right",
    "The XY pad, bottom to top",
    "The envelope: once on every touch or pulse (Transient), held while a plate is (Sustained), or round and round (Cycle)",
    "The value of the plate pressed (set each plate's with Values)",
];

/// What the view keeps between frames.
#[derive(Default)]
pub struct TouchViewState {
    /// The plates held (by the pointer or fingers): their notes and the track.
    held: Vec<(u8, u64)>,
    /// Fingers on the plates (touch screens): id, where, how hard if the screen says.
    fingers: Vec<(u64, Pos2, Option<f32>)>,
    /// How hard and where, as last sent to the track.
    sent: Option<(u64, f32, f32)>,
    /// The XY pad: where its dot is, and what was sent.
    xy: Option<(f32, f32)>,
    xy_sent: Option<(u64, f32, f32)>,
    /// A cable being drawn, from an output (true, which) or a patch point (false, which).
    drawing: Option<(bool, usize)>,
    /// The pulses counted last, and when the count changed (the pulse light).
    pulses: u32,
    flash: f64,
    /// The recorder: the step the next plate played goes into.
    rec: Option<usize>,
    /// Recording the shape with the XY pad.
    shape_rec: bool,
    /// A lane being clicked and dragged across: the step clicked, as the
    /// click left it (the steps dragged over get the same).
    paint: Option<Step>,
    /// What the third lane shows: the steps' values, chances or ratchets.
    lane_tab: usize,
    /// The plates' values are being set (dragging sets them instead of playing).
    values: bool,
    /// Sliding: the note pressed first, the bend (semitones) it is at now and
    /// the one last sent, with its track.
    slide_from: Option<u8>,
    bend_now: f32,
    bend_sent: Option<(u64, f32)>,
    /// What the plates played lately, for Capture, and whether on the
    /// song's clock (the transport going) or on one of their own.
    played: Vec<Played>,
    played_on_song: bool,
    /// The Tonnetz's triangle pressed last (Rec writes a new one into the
    /// steps), and everything pressed on it (triangles, and notes as `None`
    /// with their place): when that changes, all of it plays anew.
    tri_down: Option<Tri>,
    surface_down: Vec<(Option<Tri>, i32, i32)>,
}

/// The notes' names, from C.
const PITCH_NAMES: [&str; 12] = ["C", "C#", "D", "Eb", "E", "F", "F#", "G", "Ab", "A", "Bb", "B"];

/// A triangle's chord, its root `base` (a note) plus its own.
fn chord_name(base: i32, t: Tri) -> String {
    format!("{}{}", PITCH_NAMES[(base + t.root()).rem_euclid(12) as usize], if t.minor { "m" } else { "" })
}

/// Whether `pt` is inside the triangle `c`.
fn inside(pt: Pos2, c: &[Pos2; 3]) -> bool {
    let side = |a: Pos2, b: Pos2| (b.x - a.x) * (pt.y - a.y) - (b.y - a.y) * (pt.x - a.x);
    let (d0, d1, d2) = (side(c[0], c[1]), side(c[1], c[2]), side(c[2], c[0]));
    !((d0 < 0.0 || d1 < 0.0 || d2 < 0.0) && (d0 > 0.0 || d1 > 0.0 || d2 > 0.0))
}

/// A walk across the Tonnetz from `from`: each triangle beside the one
/// before (P, L or R, never straight back).
fn tonnetz_walk(from: Tri, n: usize, seed: f64) -> Vec<Tri> {
    let mut r = Dice::new(seed);
    let mut out = vec![from];
    let mut last: Option<Turn> = None;
    while out.len() < n {
        let turns: Vec<Turn> = [Turn::P, Turn::L, Turn::R].into_iter().filter(|&t| Some(t) != last).collect();
        let t = turns[((r.unit() * turns.len() as f32) as usize).min(turns.len() - 1)];
        out.push(out[out.len() - 1].turn(t));
        last = Some(t);
    }
    out
}

/// A note the plates played: when (in beats), how long (`end` None while
/// it is held).
#[derive(Clone, Copy, Debug)]
struct Played {
    track: u64,
    pitch: u8,
    vel: f32,
    start: f64,
    end: Option<f64>,
}

/// How many notes Capture remembers.
const PLAYED_MAX: usize = 4096;

/// What was played last as a clip, its length and its notes: the notes since
/// the last pause of two bars or more, from the bar the first falls in (played
/// with the song stopped: from the first itself) to whole bars past the last,
/// one, two, four, eight or sixteen (the last sixteen at most). Notes still
/// held end `now`; `quantize` puts them on sixteenths.
fn captured(played: &[Played], now: f64, bar: f64, on_bars: bool, quantize: bool) -> Option<(f64, Vec<Note>)> {
    let last = played.len().checked_sub(1)?;
    let end_of = |p: &Played| p.end.unwrap_or(now).max(p.start);
    // Back to the last pause.
    let mut first = last;
    let mut from = played[last].start;
    for i in (0..last).rev() {
        if end_of(&played[i]) < from - 2.0 * bar {
            break;
        }
        first = i;
        from = from.min(played[i].start);
    }
    let group = &played[first..];
    let end = group.iter().map(end_of).fold(f64::MIN, f64::max);
    let q = |b: f64| if quantize { (b * 4.0).round() / 4.0 } else { b };
    let mut start = if on_bars { (from / bar + 1e-9).floor() * bar } else { q(from) };
    let need = ((end - start) / bar - 1e-6).ceil().max(1.0);
    let bars = [1.0, 2.0, 4.0, 8.0, 16.0].into_iter().find(|&b| b >= need).unwrap_or(16.0);
    if need > 16.0 {
        start = if on_bars { ((end / bar) - 1e-9).ceil() * bar - 16.0 * bar } else { end - 16.0 * bar };
    }
    let length = bars * bar;
    let notes = group
        .iter()
        .filter(|p| p.start >= start - 1e-9)
        .map(|p| {
            let at = q(p.start - start).clamp(0.0, length - 1.0 / 32.0);
            let len = (end_of(p) - p.start).max(1.0 / 32.0).min(length - at);
            Note::new(p.pitch, at, len, p.vel)
        })
        .collect();
    Some((length, notes))
}

/// Where along its row of plates a point is, as a pitch between theirs: a
/// plate's own around its middle (pressed there, it is in tune), sliding
/// from one to the next between them.
fn slide_pitch(plates: &[(Rect, u8)], pt: Pos2) -> Option<f32> {
    let off = |r: &Rect| if pt.y < r.top() { r.top() - pt.y } else if pt.y > r.bottom() { pt.y - r.bottom() } else { 0.0 };
    let row_y = plates.iter().min_by(|a, b| off(&a.0).total_cmp(&off(&b.0)))?.0.center().y;
    let mut row: Vec<(f32, f32)> = plates.iter().filter(|(r, _)| (r.center().y - row_y).abs() < 1.0).map(|(r, p)| (r.center().x, *p as f32)).collect();
    row.sort_by(|a, b| a.0.total_cmp(&b.0));
    let (first, last) = (*row.first()?, *row.last()?);
    if pt.x <= first.0 {
        return Some(first.1);
    }
    for w in row.windows(2) {
        if pt.x <= w[1].0 {
            let f = ((pt.x - w[0].0) / (w[1].0 - w[0].0).max(1.0)).clamp(0.0, 1.0);
            let f = ((f - 0.3) / 0.4).clamp(0.0, 1.0);
            return Some(w[0].1 + (w[1].1 - w[0].1) * f);
        }
    }
    Some(last.1)
}

/// A track's Touch modulator for one output.
fn touch_mod(app: &App, track: u64, out: usize) -> Option<u64> {
    app.project.modulators.iter().find(|m| m.kind == ModKind::Touch && m.track == Some(track) && m.params.first().map(|v| v.round() as usize) == Some(out)).map(|m| m.id)
}

fn continuous(s: &ParamSpec) -> bool {
    matches!(s.kind, ParamKind::Linear | ParamKind::Log | ParamKind::Int) && s.group != krabton_engine::params::SCREEN_ONLY && s.max > s.min
}

/// The knobs of an instrument worth patching first: the filter or timbre,
/// the shape or resonance, the decay, a drive or modulation, a rate,
/// something of the space, the attack and the level of a part; then
/// whatever is left.
pub(crate) fn touch_picks(specs: &[ParamSpec]) -> Vec<usize> {
    const WANT: [&[&str]; N_PATCH] = [
        &["cutoff", "filter", "bright", "timbre", "tone", "colo"],
        &["reso", "harmonic", "morph", "shape", "wave", "fold"],
        &["decay", "release", "damp", "length"],
        &["drive", "fm", "mod", "detune", "spread", "noise"],
        &["vibrato", "lfo", "rate", "speed"],
        &["space", "reverb", "delay", "chorus", "pan"],
        &["attack"],
        &["sub", "mix", "level", "balance"],
    ];
    let mut picks: Vec<usize> = Vec::new();
    for words in WANT {
        if let Some(i) = specs.iter().enumerate().position(|(i, s)| continuous(s) && !picks.contains(&i) && words.iter().any(|w| s.name.to_lowercase().contains(w))) {
            picks.push(i);
        }
    }
    for (i, s) in specs.iter().enumerate() {
        if picks.len() >= N_PATCH {
            break;
        }
        if continuous(s) && !picks.contains(&i) && !s.name.to_lowercase().contains("volume") {
            picks.push(i);
        }
    }
    picks.truncate(N_PATCH);
    picks
}

/// The plates' notes, low to high: in the key, its pentatonic, or all
/// twelve, from the base note.
pub(crate) fn plate_pitch(key: &krabton_engine::Scale, base: i32, set: usize, i: i32) -> u8 {
    let (d0, _) = key.locate(base);
    let per = (key.locate(base + 12).0 - d0).max(1);
    let p = match set {
        2 => base + i,
        1 if per == 7 => {
            // The pentatonic of the key: major's without its 4th and 7th, minor's without its 2nd and 6th.
            let major = key.pitch_at(d0 + 2) - key.pitch_at(d0) == 4;
            let degs: [i32; 5] = if major { [0, 1, 2, 4, 5] } else { [0, 2, 3, 4, 6] };
            let (oct, k) = (i.div_euclid(5), i.rem_euclid(5));
            key.pitch_at(d0 + oct * 7 + degs[k as usize])
        }
        _ => key.pitch_at(d0 + i),
    };
    p.clamp(0, 127) as u8
}

/// How many plates an octave holds in a note set.
fn per_octave(key: &krabton_engine::Scale, base: i32, set: usize) -> i32 {
    let (d0, _) = key.locate(base);
    let per = (key.locate(base + 12).0 - d0).max(1);
    match set {
        2 => 12,
        1 if per == 7 => 5,
        _ => per,
    }
}

/// A vertical gradient, top to bottom.
fn gradient(p: &egui::Painter, r: Rect, top: Color32, bottom: Color32) {
    let mut m = Mesh::default();
    m.colored_vertex(r.left_top(), top);
    m.colored_vertex(r.right_top(), top);
    m.colored_vertex(r.right_bottom(), bottom);
    m.colored_vertex(r.left_bottom(), bottom);
    m.add_triangle(0, 1, 2);
    m.add_triangle(0, 2, 3);
    p.add(egui::Shape::mesh(m));
}

fn section(p: &egui::Painter, r: Rect, title: &str) -> Rect {
    p.rect_filled(r, 6.0, PANEL);
    p.rect_stroke(r, 6.0, Stroke::new(1.0_f32, LINE_SOFT), egui::StrokeKind::Inside);
    p.text(r.left_top() + vec2(10.0, 8.0), Align2::LEFT_TOP, title, FontId::proportional(11.0), TEXT_DIM);
    Rect::from_min_max(r.min + vec2(10.0, 26.0), r.max - vec2(10.0, 8.0))
}

/// A knob for one of Touch's parameters, `name` above it and its value below.
fn knob_named(ui: &mut Ui, r: Rect, id: Id, spec: &ParamSpec, name: &str, value: &mut f32, color: Color32) -> bool {
    let p = ui.painter().clone();
    p.text(pos2(r.center().x, r.top()), Align2::CENTER_TOP, name, FontId::proportional(10.5), TEXT_DIM);
    let side = (r.height() - 28.0).min(r.width() - 6.0).max(18.0);
    let k = Rect::from_center_size(pos2(r.center().x, r.top() + 13.0 + side * 0.5), vec2(side, side));
    let (resp, new) = knob(ui, k, id, spec.to_norm(*value), spec.to_norm(spec.default), false, color);
    let mut changed = false;
    if let Some(n) = new {
        *value = spec.from_norm(n);
        changed = true;
    }
    resp.on_hover_text(format!("{}: {}", spec.name, spec.format(*value)));
    p.text(pos2(r.center().x, k.bottom() + 1.0), Align2::CENTER_TOP, spec.format(*value), FontId::proportional(10.0), TEXT);
    changed
}

/// A font of `size` made smaller until `text` fits in `width`.
fn fitted(p: &egui::Painter, text: &str, size: f32, width: f32) -> FontId {
    let w = p.layout_no_wrap(text.to_string(), FontId::proportional(size), TEXT).size().x;
    FontId::proportional(if w > width { (size * width / w).max(7.5) } else { size })
}

/// A slider as on a west-coast panel, lying down for a row: a long slot with
/// a scale under it and a coloured cap, its name on the left, its value on
/// the right. Drag it (Shift: fine), click to put it there, double-click:
/// back to where it starts.
fn slider_across(ui: &mut Ui, r: Rect, id: Id, spec: &ParamSpec, name: &str, value: &mut f32, color: Color32) -> bool {
    let p = ui.painter().clone();
    let name_w = (r.width() * 0.26).clamp(30.0, 46.0);
    let value_w = (r.width() * 0.3).clamp(36.0, 52.0);
    p.text(pos2(r.left(), r.center().y), Align2::LEFT_CENTER, name, fitted(&p, name, 10.0, name_w - 2.0), TEXT_DIM);
    let text = spec.format(*value);
    p.text(pos2(r.right(), r.center().y), Align2::RIGHT_CENTER, &text, fitted(&p, &text, 10.0, value_w - 4.0), TEXT);
    let slot = Rect::from_min_max(pos2(r.left() + name_w, r.center().y - 2.5), pos2(r.right() - value_w, r.center().y + 2.5));
    let cap_w = (slot.width() * 0.1).clamp(8.0, 12.0);
    let cap_h = (r.height() - 4.0).clamp(10.0, 18.0);
    let travel = slot.shrink2(vec2(cap_w * 0.5, 0.0));
    let hit = Rect::from_min_max(pos2(slot.left() - 2.0, r.top()), pos2(slot.right() + 2.0, r.bottom()));
    let resp = ui.interact(hit, id, Sense::click_and_drag());
    let fine = ui.input(|i| i.modifiers.shift);
    let d = resp.drag_delta().x / travel.width().max(1.0) * if fine { 0.2 } else { 1.0 };
    let mut changed = false;
    let mut set = |v: f32, value: &mut f32| {
        if v != *value {
            *value = v;
            changed = true;
        }
    };
    if let Some(n) = drag_position(ui, &resp, id, spec.to_norm(*value), d) {
        set(spec.from_norm(n), value);
    }
    if resp.double_clicked() {
        set(spec.default, value);
    } else if resp.clicked() {
        if let Some(pt) = resp.interact_pointer_pos() {
            set(spec.from_norm((pt.x - travel.left()) / travel.width().max(1.0)), value);
        }
    }
    let shown = shown_position(ui, &resp, id, spec.to_norm(*value));
    for t in 0..=10 {
        let x = travel.left() + t as f32 / 10.0 * travel.width();
        let len = if t % 5 == 0 { 4.0 } else { 2.5 };
        p.line_segment([pos2(x, slot.bottom() + 2.0), pos2(x, slot.bottom() + 2.0 + len)], Stroke::new(1.0_f32, LINE));
    }
    p.rect_filled(slot, 2.5, SLOT);
    let x = travel.left() + shown * travel.width();
    let from = if spec.min < 0.0 && spec.max > 0.0 { travel.center().x } else { travel.left() };
    p.rect_filled(Rect::from_min_max(pos2(x.min(from), slot.top() + 1.0), pos2(x.max(from), slot.bottom() - 1.0)), 2.0, shade(color, 0.6));
    let cap = Rect::from_center_size(pos2(x, slot.center().y), vec2(cap_w, cap_h));
    let lit = resp.hovered() || resp.dragged();
    p.rect_filled(cap.translate(vec2(0.0, 1.5)), 3.0, Color32::from_black_alpha(90));
    p.rect_filled(cap, 3.0, if lit { mix(color, Color32::WHITE, 0.25) } else { color });
    p.rect_stroke(cap, 3.0, Stroke::new(1.0_f32, shade(color, 0.5)), egui::StrokeKind::Inside);
    p.line_segment([pos2(cap.center().x, cap.top() + 3.0), pos2(cap.center().x, cap.bottom() - 3.0)], Stroke::new(1.5_f32, shade(color, 0.35)));
    resp.on_hover_text(format!("{}: {}\nDrag (Shift: fine), click to put it there, double-click to reset", spec.name, spec.format(*value)));
    changed
}

/// The values a ladder offers for one of Touch's parameters, low to high: a
/// few that matter (a choice's or a short count's every one).
fn rungs(k: usize, spec: &ParamSpec) -> Vec<f32> {
    match k {
        touch::GATE => vec![0.1, 0.25, 0.5, 0.75, 1.0],
        touch::SWING => vec![0.0, 0.1, 0.2, 0.33, 0.5],
        touch::CHANCE => vec![0.2, 0.4, 0.6, 0.8, 1.0],
        touch::WANDER | touch::PRESS_VEL => vec![0.0, 0.25, 0.5, 0.75, 1.0],
        touch::DECAY => vec![0.05, 0.12, 0.25, 0.6, 1.5],
        touch::STRUM => vec![0.01, 0.02, 0.04, 0.08, 0.16],
        touch::FEEDBACK => vec![0.2, 0.4, 0.6, 0.8, 0.95],
        _ => match spec.kind {
            ParamKind::Choice(items) => (0..items.len()).map(|i| i as f32).collect(),
            ParamKind::Int if spec.max - spec.min <= 8.0 => (spec.min as i32..=spec.max as i32).map(|i| i as f32).collect(),
            _ => (0..5).map(|j| spec.from_norm(j as f32 / 4.0)).collect(),
        },
    }
}

/// A ladder of buttons for one of Touch's parameters, standing where a slider
/// would: each rung a value (written on it), lit up to the one it is on. A
/// click puts it on a rung.
fn ladder(ui: &mut Ui, r: Rect, id: Id, spec: &ParamSpec, name: &str, rungs: &[f32], value: &mut f32, color: Color32) -> bool {
    let p = ui.painter().clone();
    p.text(pos2(r.center().x, r.top()), Align2::CENTER_TOP, name, fitted(&p, name, 10.5, r.width() - 2.0), TEXT_DIM);
    let body = Rect::from_min_max(pos2(r.left() + 2.0, r.top() + 16.0), pos2(r.right() - 2.0, r.bottom() - 15.0));
    let n = rungs.len().max(1);
    let rh = body.height() / n as f32;
    let rung_at = |y: f32| (((body.bottom() - y) / rh).floor().max(0.0) as usize).min(n - 1);
    let resp = ui.interact(body, id, Sense::click_and_drag());
    let mut changed = false;
    if let Some(pt) = resp.interact_pointer_pos().filter(|_| resp.clicked() || resp.dragged()) {
        let v = rungs[rung_at(pt.y)];
        if v != *value {
            *value = v;
            changed = true;
        }
    }
    let text = spec.format(*value);
    p.text(pos2(r.center().x, r.bottom()), Align2::CENTER_BOTTOM, &text, fitted(&p, &text, 10.0, r.width() - 2.0), TEXT);
    // The rung it is on: the nearest (a value set elsewhere may lie between two).
    let norm = spec.to_norm(*value);
    let at = (0..n).min_by(|&a, &b| (spec.to_norm(rungs[a]) - norm).abs().total_cmp(&(spec.to_norm(rungs[b]) - norm).abs())).unwrap_or(0);
    let choice = matches!(spec.kind, ParamKind::Choice(_));
    let hover = resp.hover_pos().map(|pt| rung_at(pt.y));
    for (j, &v) in rungs.iter().enumerate() {
        let cell = Rect::from_min_size(pos2(body.left(), body.bottom() - (j + 1) as f32 * rh), vec2(body.width(), rh)).shrink2(vec2(0.0, 1.0));
        let fill = if j == at {
            color
        } else if j < at && !choice {
            shade(color, 0.4)
        } else {
            SLOT
        };
        p.rect_filled(cell, 3.0, if hover == Some(j) { mix(fill, TEXT, 0.15) } else { fill });
        let label = spec.format(v);
        p.text(cell.center(), Align2::CENTER_CENTER, &label, fitted(&p, &label, (rh * 0.7).clamp(7.5, 9.5), cell.width() - 3.0), if j == at { text_on(color) } else { TEXT_DIM });
    }
    resp.on_hover_text(format!("{}: {text}\nClick a rung to put it there", spec.name));
    changed
}

/// One more (`by` 1) or one fewer (-1) of a count from one to `most`, round
/// from the top to one and back.
fn turn_count(n: u8, most: u8, by: i32) -> u8 {
    ((n.clamp(1, most) as i32 - 1 + by).rem_euclid(most as i32) + 1) as u8
}

/// The chances a click goes through, from always down.
const CHANCES: [f32; 4] = [1.0, 0.75, 0.5, 0.25];

/// The next of [`CHANCES`] (`by` 1) or the one before (-1), from the nearest.
fn turn_chance(c: f32, by: i32) -> f32 {
    let at = (0..CHANCES.len()).min_by(|&a, &b| (CHANCES[a] - c).abs().total_cmp(&(CHANCES[b] - c).abs())).unwrap_or(0);
    CHANCES[(at as i32 + by).rem_euclid(CHANCES.len() as i32) as usize]
}

/// A step with what lane `k` shows of it (the third lane: tab `tab`) turned
/// by a click (`by` 1) or a right-click (-1): its pulses, chance, ratchets or
/// gate one further, its tie, accent or skip switched.
fn turn(s: Step, k: usize, tab: usize, by: i32) -> Step {
    match k {
        1 => Step { pulses: turn_count(s.pulses, MAX_PULSES, by), ..s },
        2 if tab == 1 => Step { chance: turn_chance(s.chance, by), ..s },
        2 => Step { ratchet: turn_count(s.ratchet, MAX_RATCHET, by), ..s },
        3 => Step { gate: Gate::from_index((s.gate.index() as i32 + by).rem_euclid(Gate::ALL.len() as i32) as usize), ..s },
        4 => Step { tie: !s.tie, ..s },
        5 => Step { accent: !s.accent, ..s },
        _ => Step { skip: !s.skip, ..s },
    }
}

/// `s` with what lane `k` shows taken from `from` (a click dragged across).
fn copy_lane(s: Step, from: Step, k: usize, tab: usize) -> Step {
    match k {
        1 => Step { pulses: from.pulses, ..s },
        2 if tab == 1 => Step { chance: from.chance, ..s },
        2 => Step { ratchet: from.ratchet, ..s },
        3 => Step { gate: from.gate, ..s },
        4 => Step { tie: from.tie, ..s },
        5 => Step { accent: from.accent, ..s },
        _ => Step { skip: from.skip, ..s },
    }
}

/// A lit button that switches something on and off.
fn toggle(ui: &mut Ui, r: Rect, id: Id, label: &str, on: bool, lamp: Option<f32>, tip: &str) -> bool {
    let resp = ui.interact(r, id, Sense::click()).on_hover_text(tip);
    let p = ui.painter();
    let fill = if on { mix(PANEL3, accent_now(), 0.35) } else { PANEL3 };
    p.rect_filled(r, 5.0, if resp.hovered() { mix(fill, TEXT, 0.08) } else { fill });
    p.rect_stroke(r, 5.0, Stroke::new(1.0_f32, if on { accent_now() } else { LINE }), egui::StrokeKind::Inside);
    let mut tx = r.center().x;
    if let Some(l) = lamp {
        let c = pos2(r.left() + 10.0, r.center().y);
        p.circle_filled(c, 4.0, mix(shade(REC, 0.35), REC, l.clamp(0.0, 1.0)));
        tx += 5.0;
    }
    p.text(pos2(tx, r.center().y), Align2::CENTER_CENTER, label, FontId::proportional(11.5), if on { TEXT } else { TEXT_DIM });
    resp.clicked()
}

/// A plain button.
fn button(ui: &mut Ui, r: Rect, id: Id, label: &str, tip: &str) -> bool {
    let resp = ui.interact(r, id, Sense::click()).on_hover_text(tip);
    let p = ui.painter();
    p.rect_filled(r, 4.0, if resp.hovered() { mix(PANEL3, TEXT, 0.1) } else { PANEL3 });
    p.text(r.center(), Align2::CENTER_CENTER, label, FontId::proportional(11.0), TEXT);
    resp.clicked()
}

/// The little arrow of a list that opens.
fn down_arrow(p: &egui::Painter, c: Pos2) {
    p.add(egui::Shape::convex_polygon(vec![c + vec2(-4.0, -2.0), c + vec2(4.0, -2.0), c + vec2(0.0, 3.0)], TEXT_DIM, Stroke::NONE));
}

/// A choice shown as its value; a click opens the list.
fn choice(ui: &mut Ui, r: Rect, id: Id, spec: &ParamSpec, value: &mut f32) -> bool {
    let ParamKind::Choice(items) = spec.kind else { return false };
    let resp = ui.interact(r, id, Sense::click()).on_hover_text(spec.name);
    let p = ui.painter();
    p.rect_filled(r, 4.0, if resp.hovered() { PANEL3 } else { SLOT });
    let i = (value.round().max(0.0) as usize).min(items.len() - 1);
    let name_w = p.layout_no_wrap(spec.name.into(), FontId::proportional(10.5), TEXT_DIM).size().x;
    if r.width() > name_w + 70.0 {
        p.text(pos2(r.left() + 7.0, r.center().y), Align2::LEFT_CENTER, spec.name, FontId::proportional(10.5), TEXT_DIM);
        p.text(pos2(r.left() + 13.0 + name_w, r.center().y), Align2::LEFT_CENTER, items[i], FontId::proportional(11.5), TEXT);
    } else {
        p.text(pos2(r.left() + 7.0, r.center().y), Align2::LEFT_CENTER, items[i], FontId::proportional(11.5), TEXT);
    }
    down_arrow(p, pos2(r.right() - 10.0, r.center().y));
    let mut changed = false;
    egui::Popup::menu(&resp).show(|ui| {
        for (k, it) in items.iter().enumerate() {
            if ui.selectable_label(k == i, *it).clicked() {
                *value = k as f32;
                changed = true;
                ui.close();
            }
        }
    });
    changed
}

fn bezier(a: Pos2, b: Pos2) -> [Pos2; 4] {
    let dx = (b.x - a.x).abs();
    let sag = 18.0 + dx * 0.12 + (b.y - a.y).abs() * 0.05;
    [a, pos2(a.x + dx * 0.4, a.y + sag), pos2(b.x - dx * 0.4, b.y + sag), b]
}

fn bezier_mid(c: &[Pos2; 4]) -> Pos2 {
    let w = [0.125, 0.375, 0.375, 0.125];
    pos2(c.iter().zip(w).map(|(p, w)| p.x * w).sum(), c.iter().zip(w).map(|(p, w)| p.y * w).sum())
}

/// A small random generator for the dice (seeded from the clock).
struct Dice(u64);

impl Dice {
    fn new(seed: f64) -> Self {
        Dice((seed.to_bits() ^ 0x9E37_79B9_7F4A_7C15) | 1)
    }

    fn unit(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 40) as f32 / (1u64 << 24) as f32
    }
}

/// Rolls a new sequence: a walk through the key with small steps more
/// likely, values, a few rests, longer steps and other gates, ratchets,
/// accents and steps by chance.
fn dice(data: &mut Vec<u8>, n: usize, seed: f64) {
    let mut r = Dice::new(seed);
    let mut note = 0i32;
    let tris = tonnetz_walk(Tri::default(), n, seed * 1.7 + 0.3);
    for i in 0..n {
        let u = r.unit();
        let mv = if u < 0.3 { 0 } else if u < 0.7 { 1 } else if u < 0.9 { 2 } else { 4 };
        note = (note + if r.unit() < 0.5 { mv } else { -mv }).clamp(-7, 9);
        let u = r.unit();
        let gate = if i > 0 && u < 0.14 {
            Gate::Rest
        } else if u < 0.24 {
            Gate::Every
        } else if u < 0.31 {
            Gate::Hold
        } else if u < 0.35 {
            Gate::Alternate
        } else if u < 0.38 {
            Gate::Roll
        } else if u < 0.41 {
            Gate::Last
        } else {
            Gate::Single
        };
        let s = Step {
            note,
            value: r.unit(),
            chance: if r.unit() < 0.15 { 0.5 + r.unit() * 0.4 } else { 1.0 },
            ratchet: if r.unit() < 0.12 { 2 + (r.unit() * 4.0) as u8 } else { 1 },
            pulses: if r.unit() < 0.22 { 2 + (r.unit() * 3.0) as u8 } else { 1 },
            gate,
            tie: r.unit() < 0.06,
            accent: i % 4 == 0 && r.unit() < 0.6,
            skip: false,
            tri: tris[i],
        };
        touch::set_step(data, i, s);
    }
}

/// Changes a few steps a little.
fn mutate(data: &mut Vec<u8>, n: usize, seed: f64) {
    let mut r = Dice::new(seed);
    for _ in 0..(1 + n / 5) {
        let i = ((r.unit() * n as f32) as usize).min(n - 1);
        let mut s = touch::step(data, i);
        match (r.unit() * 7.0) as u32 {
            6 => s.tri = s.tri.turn([Turn::P, Turn::L, Turn::R][((r.unit() * 3.0) as usize).min(2)]),
            0 => s.note = (s.note + if r.unit() < 0.5 { 1 } else { -1 }).clamp(-14, 14),
            1 => s.note = (s.note + if r.unit() < 0.5 { 2 } else { -2 }).clamp(-14, 14),
            2 => s.gate = if s.rests() { Gate::Single } else { Gate::Rest },
            3 => s.ratchet = if s.ratchet > 1 { 1 } else { 2 + (r.unit() * 3.0) as u8 },
            4 => s.pulses = if s.pulses > 1 { 1 } else { 2 + (r.unit() * 2.0) as u8 },
            _ => s.gate = Gate::from_index((s.gate.index() + 1) % Gate::ALL.len()),
        }
        s.value = (s.value + (r.unit() - 0.5) * 0.3).clamp(0.0, 1.0);
        touch::set_step(data, i, s);
    }
}

impl App {
    /// The track the Touch view plays: the selected one, if it has an instrument.
    pub(crate) fn touch_track(&self) -> Option<usize> {
        let TrackSel::Track(t) = self.sel_track else { return None };
        let tr = self.project.tracks.get(t)?;
        (tr.kind == TrackKind::Midi && tr.instrument.is_some()).then_some(t)
    }

    /// The track's Touch device; the first time, it is put at the front of
    /// its MIDI chain, with knobs of the instrument on its patch points and
    /// the pressure patched to the first.
    pub(crate) fn touch_device(&mut self, ti: usize) -> usize {
        if let Some(i) = self.project.tracks[ti].midi_devices.iter().position(|d| d.kind == MidiFxKind::Touch) {
            return i;
        }
        let tid = self.project.tracks[ti].id;
        let specs = self.project.tracks[ti].instrument.as_ref().and_then(instrument_specs).unwrap_or(&[]);
        let picks = touch_picks(specs);
        let mut dev = self.project.make_midi_device(MidiFxKind::Touch);
        for (k, &param) in picks.iter().enumerate() {
            touch::set_patch(&mut dev.data, k, Some(param as u16));
        }
        // Played here, it starts folded away in the device chain.
        self.project.folded.insert(dev.id);
        self.project.tracks[ti].midi_devices.insert(0, dev);
        if let Some(&first) = picks.first() {
            self.touch_connect(tid, 0, first as u16, 0.5);
        }
        0
    }

    /// A cable from output `out` to the instrument's parameter `param`.
    fn touch_connect(&mut self, track: u64, out: usize, param: u16, amount: f32) {
        let id = match touch_mod(self, track, out) {
            Some(id) => id,
            None => {
                if self.project.modulators.len() >= MAX_MODS {
                    self.toast(format!("At most {MAX_MODS} modulators: take one away in the Mod tab"));
                    return;
                }
                let mut m = self.project.make_modulator(ModKind::Touch);
                m.params[0] = out as f32;
                m.track = Some(track);
                let tname = self.project.tracks.iter().find(|t| t.id == track).map(|t| t.name.clone()).unwrap_or_default();
                m.name = format!("Touch {} ({tname})", OUTPUTS[out]);
                let id = m.id;
                self.project.modulators.push(m);
                id
            }
        };
        self.project.add_route(id, ModTarget::Param { owner: ModOwner::Instrument(track), param }, amount);
    }

    /// Takes a cable away (and its modulator, when nothing else is patched from it).
    fn touch_unpatch(&mut self, route: u64) {
        let source = self.project.mod_routes.iter().find(|r| r.id == route).map(|r| r.source);
        self.project.mod_routes.retain(|r| r.id != route);
        if let Some(src) = source {
            let used = self.project.mod_routes.iter().any(|r| r.source == src);
            if !used && self.project.modulators.iter().any(|m| m.id == src && m.kind == ModKind::Touch) {
                self.project.modulators.retain(|m| m.id != src);
                self.project.prune_mod_routes();
            }
        }
    }

    /// The cables of a track's Touch: (output, patch point, route, amount).
    fn touch_cables(&self, track: u64, data: &[u8]) -> Vec<(usize, usize, u64, f32)> {
        let mut out = Vec::new();
        for k in 0..N_OUT {
            let Some(src) = touch_mod(self, track, k) else { continue };
            for r in self.project.mod_routes.iter().filter(|r| r.source == src) {
                if let ModTarget::Param { owner: ModOwner::Instrument(t), param } = r.target {
                    if let Some(slot) = (0..N_PATCH).find(|&s| t == track && touch::patch(data, s) == Some(param)) {
                        out.push((k, slot, r.id, r.amount));
                    }
                }
            }
        }
        out
    }

    /// A plate pressed from the keys (`step` plates up from the base note).
    pub(crate) fn touch_key(&mut self, step: i32) -> Option<(u8, Vec<u64>)> {
        let ti = self.touch_track()?;
        let di = self.touch_device(ti);
        let params = &self.project.tracks[ti].midi_devices[di].params;
        let base = params.get(touch::BASE).copied().unwrap_or(48.0).round() as i32;
        let set = params.get(touch::NOTES).copied().unwrap_or(0.0).round().clamp(0.0, 2.0) as usize;
        let pitch = plate_pitch(&self.project.sounding_key(), base, set, step);
        let id = self.project.tracks[ti].id;
        let vel = self.kb_velocity;
        let value = touch::plate_value(&self.project.tracks[ti].midi_devices[di].data, step.max(0) as usize);
        self.unbend();
        self.cmd(Command::TouchPlate { track: id, value });
        self.touch_record(ti, di, pitch);
        self.cmd(Command::Touch { track: id, pressure: vel, position: 0.5 });
        self.touch_note_on(ti, pitch, vel);
        Some((pitch, vec![id]))
    }

    /// The clock the plates' notes are remembered on: the song's (the
    /// transport going) or one of their own, in beats at the tempo.
    fn touch_clock(&self) -> (bool, f64) {
        static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
        if self.is_playing() {
            (true, self.status().session_pos.load())
        } else {
            (false, START.get_or_init(std::time::Instant::now).elapsed().as_secs_f64() * self.project.bpm / 60.0)
        }
    }

    /// A plate's note starts: it sounds, Record takes it, Capture remembers it.
    fn touch_note_on(&mut self, ti: usize, pitch: u8, vel: f32) {
        let track = self.project.tracks[ti].id;
        self.cmd(Command::NoteOn { track, pitch, velocity: vel });
        self.rec_note_on(ti, pitch, vel);
        let (on_song, start) = self.touch_clock();
        let pl = &mut self.touch_view;
        if pl.played_on_song != on_song {
            // The transport started or stopped: another clock.
            pl.played.clear();
            pl.played_on_song = on_song;
        }
        if pl.played.len() >= PLAYED_MAX {
            pl.played.drain(..PLAYED_MAX / 4);
        }
        pl.played.push(Played { track, pitch, vel, start, end: None });
    }

    /// A plate's note ends.
    fn touch_note_off(&mut self, track: u64, pitch: u8) {
        self.cmd(Command::NoteOff { track, pitch });
        self.finish_rec_note(track, pitch, None);
        self.touch_played_off(track, pitch);
    }

    /// Capture's note of `pitch` on `track` ends now (if it is one).
    pub(crate) fn touch_played_off(&mut self, track: u64, pitch: u8) {
        if self.touch_view.played.is_empty() {
            return;
        }
        let (on_song, now) = self.touch_clock();
        let pl = &mut self.touch_view;
        if let Some(p) = pl.played.iter_mut().rev().find(|p| p.track == track && p.pitch == pitch && p.end.is_none()) {
            p.end = Some(if on_song == pl.played_on_song { now.max(p.start) } else { p.start + 0.25 });
        }
    }

    /// Capture: what the plates played last on track `ti` becomes a clip in
    /// the next free slot, and plays.
    pub(crate) fn touch_capture(&mut self, ti: usize) {
        let track = self.project.tracks[ti].id;
        let (on_song, now) = self.touch_clock();
        let mine: Vec<Played> = self.touch_view.played.iter().filter(|p| p.track == track).copied().collect();
        let now = if on_song == self.touch_view.played_on_song { now } else { mine.iter().map(|p| p.end.unwrap_or(p.start)).fold(0.0, f64::max) };
        let bar = self.project.beats_per_bar_f();
        let Some((length, notes)) = captured(&mine, now, bar, self.touch_view.played_on_song, self.rec_quantize) else {
            self.toast("Nothing played on the plates yet to capture");
            return;
        };
        let ns = self.project.scenes.len();
        let from = self.sel_scene.min(ns.saturating_sub(1));
        let slot = match (from..ns).find(|&s| self.project.tracks[ti].slots[s].is_none()) {
            Some(s) => s,
            None => {
                self.project.add_scene(ns);
                ns
            }
        };
        let id = self.project.alloc_id();
        let t = &mut self.project.tracks[ti];
        let mut clip = Clip::new_midi(id, "Capture", t.color, length);
        let n = notes.len();
        if let Some(v) = clip.notes_mut() {
            *v = notes;
        }
        t.slots[slot] = Some(clip);
        self.touch_view.played.retain(|p| p.track != track);
        self.cmd(Command::LaunchClip { track, slot });
        self.select_clip(ClipSel::Slot { track: ti, scene: slot });
        let bars = (length / bar).round() as u32;
        self.toast(format!("Captured {n} note{} into a clip of {bars} bar{}", if n == 1 { "" } else { "s" }, if bars == 1 { "" } else { "s" }));
    }

    /// The Tonnetz in place of the plates: the lattice of notes a fifth apart
    /// along it and a major third apart up it (the middle the base note),
    /// every triangle a triad. A note pressed (the middle of its dot) plays
    /// that note alone; a triangle pressed plays its chord (its notes asked
    /// for, the root last; where a finger is higher up, harder), and with Rec
    /// on goes into the steps; sliding onto another triangle (or note) plays
    /// it whole, the notes it shares too. The notes sounding light up wherever
    /// they are on it; with the sequencer on (it plays the steps' chords by
    /// itself, around the middle) the steps' triangles carry their numbers,
    /// the one playing lit.
    #[allow(clippy::too_many_arguments)]
    fn tonnetz_plates(&mut self, ui: &mut Ui, area: Rect, ti: usize, di: usize, playing: Option<usize>, sounding: u128, tcol: Color32) -> Vec<(u8, f32, f32)> {
        let p = ui.painter().with_clip_rect(area.expand(2.0));
        let accent = accent_now();
        let tid = self.project.tracks[ti].id;
        let dev = &self.project.tracks[ti].midi_devices[di];
        let n_on = (dev.params[touch::STEPS].round() as usize).clamp(1, N_STEPS);
        let seq = dev.params[touch::SEQ] >= 0.5 && dev.params[touch::PITCH] >= 0.5;
        let base = dev.params[touch::BASE].round() as i32;
        let steps: Vec<Step> = (0..n_on).map(|i| touch::step(&dev.data, i)).collect();
        // Big: three rows of triangles (more where there is the height), four and a bit along.
        let h3 = 3f32.sqrt() * 0.5;
        let side = (area.height() / 3.0 / h3).min(area.width() / 4.2).max(12.0);
        let th = side * h3;
        let rows = ((area.height() / th).floor() as i8).clamp(3, 6);
        let (low, high) = (-(rows / 2), rows - rows / 2 - 1);
        let mid = pos2(area.center().x, area.center().y + (low + high + 1) as f32 * 0.5 * th);
        let point = |a: i32, b: i32| pos2(mid.x + (a as f32 + b as f32 * 0.5) * side, mid.y - b as f32 * th);
        let corners = |t: Tri| -> [Pos2; 3] {
            let (a, b) = (t.a as i32, t.b as i32);
            if t.minor {
                [point(a + 1, b), point(a, b + 1), point(a + 1, b + 1)]
            } else {
                [point(a, b), point(a + 1, b), point(a, b + 1)]
            }
        };
        let centre = |c: &[Pos2; 3]| pos2((c[0].x + c[1].x + c[2].x) / 3.0, (c[0].y + c[1].y + c[2].y) / 3.0);
        // The triangles whose middle is on it (cut off at its edges), and the notes at their corners.
        let room = area.expand(1.0);
        let mut tris: Vec<(Tri, [Pos2; 3])> = Vec::new();
        for b in low..=high {
            for a in -8..=7i8 {
                for minor in [false, true] {
                    let t = Tri::new(a, b, minor);
                    let c = corners(t);
                    if room.contains(centre(&c)) {
                        tris.push((t, c));
                    }
                }
            }
        }
        let rr = (side * 0.2).clamp(10.0, 24.0);
        let mut dots: Vec<(Pos2, i32, i32)> = Vec::new();
        for b in low as i32..=high as i32 + 1 {
            for a in -9..=8 {
                let q = point(a, b);
                if area.expand(rr * 0.5).contains(q) {
                    dots.push((q, a, b));
                }
            }
        }
        let dot_at = |pt: Pos2| dots.iter().find(|d| d.0.distance(pt) <= rr).copied();
        let tri_at = |pt: Pos2| tris.iter().find(|(_, c)| inside(pt, c)).copied();
        // Where it is pressed: fingers, or the pointer.
        let resp = ui.interact(area, Id::new(("tonnetz", tid)), Sense::click_and_drag());
        for ev in ui.input(|i| i.events.clone()) {
            if let egui::Event::Touch { id, phase, pos, force, .. } = ev {
                let fid = id.0.wrapping_add(1);
                let fingers = &mut self.touch_view.fingers;
                match phase {
                    egui::TouchPhase::Start if area.contains(pos) => fingers.push((fid, pos, force)),
                    egui::TouchPhase::Move => {
                        if let Some(f) = fingers.iter_mut().find(|f| f.0 == fid) {
                            (f.1, f.2) = (pos, force);
                        }
                    }
                    egui::TouchPhase::End | egui::TouchPhase::Cancel => fingers.retain(|f| f.0 != fid),
                    _ => {}
                }
            }
        }
        let touching = !self.touch_view.fingers.is_empty() || ui.input(|i| i.any_touches());
        let mut contacts: Vec<(Pos2, Option<f32>)> = self.touch_view.fingers.iter().map(|f| (f.1, f.2)).collect();
        if let Some(pt) = resp.interact_pointer_pos().filter(|_| resp.is_pointer_button_down_on() && !touching) {
            contacts.push((pt, None));
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }
        let mut want: Vec<(u8, f32, f32)> = Vec::new();
        let mut pressed: Vec<Tri> = Vec::new();
        let mut pressed_dots: Vec<(i32, i32)> = Vec::new();
        for (pt, force) in contacts {
            let along = ((pt.x - area.left()) / area.width()).clamp(0.0, 1.0);
            // In the middle of a note: that note alone, in the octave nearest the middle.
            if let Some((q, a, b)) = dot_at(pt) {
                let pressure = force.filter(|f| *f > 0.0).map(|f| f.clamp(0.05, 1.0)).unwrap_or(((1.0 - (pt.y - q.y + rr) / (2.0 * rr)) * 0.8 + 0.2).clamp(0.05, 1.0));
                let note = (base + (7 * a + 4 * b + 6).rem_euclid(12) - 6).clamp(0, 127) as u8;
                want.push((note, pressure, along));
                pressed_dots.push((a, b));
                continue;
            }
            let Some((t, c)) = tri_at(pt) else { continue };
            let top = c.iter().map(|q| q.y).fold(f32::MAX, f32::min);
            let pressure = force.filter(|f| *f > 0.0).map(|f| f.clamp(0.05, 1.0)).unwrap_or(((1.0 - (pt.y - top) / th) * 0.8 + 0.2).clamp(0.05, 1.0));
            // Its notes around the middle, the root last.
            let root = (base + t.root()).rem_euclid(12);
            let mut notes = touch::triad(base.clamp(0, 127) as u8, t);
            notes.sort_by_key(|&n| (n as i32 - root).rem_euclid(12) == 0);
            for n in notes {
                want.push((n, pressure, along));
            }
            pressed.push(t);
        }
        // Slid onto another triangle or note: all of it anew, the notes it shares with the last too.
        let down: Vec<(Option<Tri>, i32, i32)> = pressed.iter().map(|&t| (Some(t), 0, 0)).chain(pressed_dots.iter().map(|&(a, b)| (None, a, b))).collect();
        if !down.is_empty() && !self.touch_view.surface_down.is_empty() && down != self.touch_view.surface_down {
            for (pitch, t) in std::mem::take(&mut self.touch_view.held) {
                self.touch_note_off(t, pitch);
            }
        }
        self.touch_view.surface_down = down;
        let first = pressed.first().copied();
        if first.is_some() && first != self.touch_view.tri_down {
            if let (Some(at), Some(t)) = (self.touch_view.rec, first) {
                // Rec: into the step it is at, and on to the next (the sequence plays them once switched on).
                let dev = &mut self.project.tracks[ti].midi_devices[di];
                let s = touch::step(&dev.data, at);
                let gate = if s.rests() { Gate::Single } else { s.gate };
                touch::set_step(&mut dev.data, at, Step { tri: t, gate, ..s });
                dev.params[touch::PITCH] = 1.0;
                dev.preset.clear();
                self.touch_view.rec = Some((at + 1) % n_on);
            }
        }
        self.touch_view.tri_down = first;
        // The steps' chords around the middle (where each is, or the same chord elsewhere on it).
        let shown: Vec<Option<Tri>> = steps.iter().map(|s| tris.iter().find(|x| x.0 == s.tri).or_else(|| tris.iter().find(|x| x.0.classes() == s.tri.classes())).map(|x| x.0)).collect();
        let now = playing.filter(|_| seq).and_then(|i| shown.get(i).copied().flatten());
        let hover_dot = resp.hover_pos().and_then(|pt| dot_at(pt));
        let hover = resp.hover_pos().filter(|_| hover_dot.is_none()).and_then(|pt| tri_at(pt)).map(|x| x.0);
        for (t, c) in &tris {
            let used = seq && shown.contains(&Some(*t));
            let mut fill = if t.minor { Color32::from_rgb(0x1e, 0x1e, 0x23) } else { Color32::from_rgb(0x2b, 0x2b, 0x31) };
            if used {
                fill = mix(fill, accent, 0.22);
            }
            if now == Some(*t) {
                fill = mix(fill, accent, 0.7);
            }
            if pressed.contains(t) {
                fill = mix(tcol, Color32::WHITE, 0.15);
            } else if hover == Some(*t) {
                fill = mix(fill, TEXT, 0.1);
            }
            p.add(egui::Shape::convex_polygon(c.to_vec(), fill, Stroke::new(1.0_f32, LINE)));
            let size = (side * 0.17).clamp(9.0, 20.0);
            let lit = pressed.contains(t) || now == Some(*t);
            p.text(centre(c) + vec2(0.0, if t.minor { -th * 0.06 } else { th * 0.06 }), Align2::CENTER_CENTER, chord_name(base, *t), FontId::proportional(size), if lit { TEXT } else if used { mix(TEXT_DIM, TEXT, 0.5) } else { TEXT_DIM });
        }
        // The notes at the corners, lit while they sound.
        let classes_sounding: Vec<i32> = (0..128).filter(|n| sounding >> n & 1 == 1).map(|n: i32| n.rem_euclid(12)).collect();
        for &(q, a, b) in &dots {
            let pc = (base + 7 * a + 4 * b).rem_euclid(12);
            let on = classes_sounding.contains(&pc);
            let down = pressed_dots.contains(&(a, b));
            let fill = if down {
                mix(tcol, Color32::WHITE, 0.3)
            } else if on {
                tcol
            } else if a == 0 && b == 0 {
                mix(PANEL, accent, 0.35)
            } else {
                PANEL
            };
            let fill = if hover_dot.is_some_and(|d| d.1 == a && d.2 == b) && !down { mix(fill, TEXT, 0.12) } else { fill };
            p.circle_filled(q, rr, fill);
            p.circle_stroke(q, rr, Stroke::new(if down { 2.0_f32 } else { 1.0 }, if on || down { mix(tcol, Color32::WHITE, 0.5) } else { LINE }));
            p.text(q, Align2::CENTER_CENTER, PITCH_NAMES[pc as usize], FontId::proportional(rr * 0.85), if on || down { text_on(tcol) } else { TEXT });
        }
        // The sequence: its steps' numbers under their chords' names.
        if seq {
            let at_of = |t: Tri| tris.iter().find(|x| x.0 == t).map(|x| centre(&x.1));
            let mut labels: Vec<(Pos2, String)> = Vec::new();
            for (i, t) in shown.iter().enumerate() {
                let Some(c) = t.and_then(at_of) else { continue };
                match labels.iter_mut().find(|l| l.0 == c) {
                    Some(l) => l.1 = format!("{} {}", l.1, i + 1),
                    None => labels.push((c, (i + 1).to_string())),
                }
            }
            let playing_at = now.and_then(at_of);
            for (c, text) in labels {
                let font = fitted(&p, &text, (side * 0.11).clamp(9.0, 13.0), side * 0.45);
                p.text(c + vec2(0.0, th * 0.22), Align2::CENTER_CENTER, text, font, if playing_at == Some(c) { text_on(accent) } else { shade(accent, 0.9) });
            }
        }
        if let Some((_, a, b)) = hover_dot {
            resp.on_hover_text(format!("{}: press to play it alone", PITCH_NAMES[(base + 7 * a + 4 * b).rem_euclid(12) as usize]));
        } else if let Some(t) = hover {
            let rec = self.touch_view.rec.map(|i| format!("; Rec writes it into step {}", i + 1)).unwrap_or_default();
            resp.on_hover_text(format!("{}: press to play the chord{rec}", chord_name(base, t)));
        }
        want
    }

    /// The track bent by sliding back to its notes.
    fn unbend(&mut self) {
        if let Some((t, b)) = self.touch_view.bend_sent.take() {
            if b != 0.0 {
                self.cmd(Command::PitchBend { track: t, semis: 0.0 });
            }
        }
        self.touch_view.bend_now = 0.0;
    }

    /// The recorder: a plate played goes into the next step (the sequencer
    /// on, its notes moving).
    fn touch_record(&mut self, ti: usize, di: usize, pitch: u8) {
        let Some(at) = self.touch_view.rec else { return };
        let key = self.project.sounding_key();
        let dev = &mut self.project.tracks[ti].midi_devices[di];
        let base = dev.params[touch::BASE].round() as i32;
        let n = (dev.params[touch::STEPS].round() as usize).clamp(1, N_STEPS);
        let note = (key.locate(pitch as i32).0 - key.locate(base).0).clamp(-14, 14);
        let mut s = touch::step(&dev.data, at);
        s.note = note;
        if s.rests() {
            s.gate = Gate::Single;
        }
        touch::set_step(&mut dev.data, at, s);
        dev.params[touch::SEQ] = 1.0;
        dev.params[touch::PITCH] = 1.0;
        dev.preset.clear();
        self.touch_view.rec = Some((at + 1) % n);
    }

    pub(crate) fn ui_touch(&mut self, ui: &mut Ui) {
        let full = ui.max_rect();
        ui.painter().rect_filled(full, 0.0, BG);
        let Some(ti) = self.touch_track() else {
            self.touch_pick(ui, full);
            return;
        };
        let di = self.touch_device(ti);
        let tid = self.project.tracks[ti].id;
        let now = ui.input(|i| i.time);
        let status = self.status();
        let vis = status.seqs.read(self.project.tracks[ti].midi_devices[di].id);
        let seen = vis.map(touch::read_visual).unwrap_or(touch::Seen { step: 0, steps: 8, pulses: 0, last_note: 0, seq: false, pulse: 0, outs: [0.0; N_OUT] });
        let outs = seen.outs;
        if seen.pulses != self.touch_view.pulses {
            self.touch_view.pulses = seen.pulses;
            self.touch_view.flash = now;
        }
        let lamp = (1.0 - ((now - self.touch_view.flash) / 0.12) as f32).clamp(0.0, 1.0);
        let accent = accent_now();
        let specs = touch::TOUCH;
        let inst_specs = self.project.tracks[ti].instrument.as_ref().and_then(instrument_specs).unwrap_or(&[]);
        let held_notes = status.tracks.get(ti).map(|t| t.inst_notes()).unwrap_or(0);
        let key = self.project.sounding_key();
        let base_note = self.project.tracks[ti].midi_devices[di].params[touch::BASE].round() as i32;
        let tr = &self.project.tracks[ti];
        let tcol = Color32::from_rgb(tr.color[0], tr.color[1], tr.color[2]);

        let r = full.shrink2(vec2(12.0, 10.0));
        let header = Rect::from_min_size(r.min, vec2(r.width(), 28.0));
        let body = Rect::from_min_max(pos2(r.left(), header.bottom() + 8.0), r.max);
        let top_h = (body.height() * 0.58).clamp(240.0, 400.0).min(body.height() - 110.0);
        let top = Rect::from_min_size(body.min, vec2(body.width(), top_h));
        let bottom = Rect::from_min_max(pos2(body.left(), top.bottom() + 10.0), body.max);
        let w = top.width();
        let play_r = Rect::from_min_size(top.min, vec2(232.0, top.height()));
        let patch_w = (w * 0.30).clamp(260.0, 380.0);
        let patch_r = Rect::from_min_max(pos2(top.right() - patch_w, top.top()), top.max);
        let seq_r = Rect::from_min_max(pos2(play_r.right() + 10.0, top.top()), pos2(patch_r.left() - 10.0, top.bottom()));
        let motion_w = (bottom.width() * 0.40).clamp(330.0, 480.0);
        let motion_r = Rect::from_min_max(pos2(bottom.right() - motion_w, bottom.top()), bottom.max);
        let plates_r = Rect::from_min_max(bottom.min, pos2(motion_r.left() - 10.0, bottom.bottom()));
        let p = ui.painter().clone();

        // ---------------------------------------------------------------- header
        p.text(header.left_center(), Align2::LEFT_CENTER, "TOUCH", FontId::proportional(18.0), accent);
        let chip = Rect::from_min_size(pos2(header.left() + 78.0, header.top() + 2.0), vec2(250.0, 24.0));
        let chip_resp = ui.interact(chip, Id::new("touch_track"), Sense::click()).on_hover_text("The track Touch plays: click for another (up and down pick it too)");
        p.rect_filled(chip, 5.0, if chip_resp.hovered() { PANEL3 } else { PANEL2 });
        p.rect_filled(Rect::from_min_size(chip.min, vec2(5.0, chip.height())), 2.0, tcol);
        let inst_name = tr.instrument.as_ref().map(|i| i.track_name()).unwrap_or_default();
        p.text(pos2(chip.left() + 12.0, chip.center().y), Align2::LEFT_CENTER, format!("{}  ·  {inst_name}", tr.name), FontId::proportional(12.5), TEXT);
        down_arrow(&p, pos2(chip.right() - 11.0, chip.center().y));
        let mut pick_track = None;
        egui::Popup::menu(&chip_resp).show(|ui| {
            for (i, t) in self.project.tracks.iter().enumerate() {
                if t.kind == TrackKind::Midi && t.instrument.is_some() && ui.selectable_label(i == ti, &t.name).clicked() {
                    pick_track = Some(i);
                    ui.close();
                }
            }
        });
        let preset_r = Rect::from_min_size(pos2(chip.right() + 10.0, chip.top()), vec2(170.0, 24.0));
        let preset_resp = ui.interact(preset_r, Id::new("touch_preset"), Sense::click()).on_hover_text("Touch's presets: how it plays, the rhythm and the sequence (the plates and the patch stay)");
        let preset_name = self.project.tracks[ti].midi_devices[di].preset.clone();
        p.rect_filled(preset_r, 5.0, if preset_resp.hovered() { PANEL3 } else { PANEL2 });
        p.text(pos2(preset_r.left() + 10.0, preset_r.center().y), Align2::LEFT_CENTER, if preset_name.is_empty() { "Presets" } else { preset_name.as_str() }, FontId::proportional(12.0), TEXT);
        down_arrow(&p, pos2(preset_r.right() - 11.0, preset_r.center().y));
        let mut pick_preset = None;
        egui::Popup::menu(&preset_resp).show(|ui| {
            for (i, pr) in krabton_engine::presets::midi(MidiFxKind::Touch).iter().enumerate() {
                if ui.selectable_label(pr.name == preset_name, pr.name).clicked() {
                    pick_preset = Some(i);
                    ui.close();
                }
            }
        });
        let rec_r = Rect::from_min_size(pos2(preset_r.right() + 10.0, preset_r.top()), vec2(84.0, 24.0));
        let recording = self.recording;
        if toggle(ui, rec_r, Id::new("touch_rec_clip"), "Record", recording, Some(if recording { 1.0 } else { 0.0 }), "Records what you play on the plates into a clip on this track (the transport's Record): into the clip playing, or a new one in the next free slot; the transport starts") {
            self.toggle_record();
        }
        let cap_r = Rect::from_min_size(pos2(rec_r.right() + 6.0, rec_r.top()), vec2(76.0, 24.0));
        let cap_tip = "Capture: what you just played on the plates (since the last pause) becomes a clip in the next free slot and plays, also when you were not recording";
        if button(ui, cap_r, Id::new("touch_capture"), "Capture", cap_tip) {
            self.touch_capture(ti);
        }
        let sliders = !self.prefs.knobs;
        let hint = "higher up a plate: harder  ·  press and move along: slide  ·  keys A–L and W–P play the plates";
        let hw = p.layout_no_wrap(hint.into(), FontId::proportional(11.0), TEXT_DIM).size().x;
        if header.right() - hw > cap_r.right() + 20.0 {
            p.text(header.right_center(), Align2::RIGHT_CENTER, hint, FontId::proportional(11.0), TEXT_DIM);
        }

        let dev = &mut self.project.tracks[ti].midi_devices[di];
        let mut changed = false;
        let flip = |v: &mut f32| *v = if *v >= 0.5 { 0.0 } else { 1.0 };

        // ---------------------------------------------------------------- play
        let inner = section(&p, play_r, "PLAY");
        let row_h = 22.0;
        let mut y = inner.top();
        let half = (inner.width() - 4.0) / 2.0;
        changed |= choice(ui, Rect::from_min_size(pos2(inner.left(), y), vec2(half, row_h)), Id::new(("touch_play", tid)), &specs[touch::PLAY], &mut dev.params[touch::PLAY]);
        changed |= choice(ui, Rect::from_min_size(pos2(inner.left() + half + 4.0, y), vec2(half, row_h)), Id::new(("touch_voicing", tid)), &specs[touch::VOICING], &mut dev.params[touch::VOICING]);
        y += row_h + 5.0;
        let tw = (inner.width() - 8.0) / 3.0;
        let pulse_on = dev.params[touch::PULSE] >= 0.5;
        let arp = dev.params[touch::PLAY].round() as usize == 1;
        if toggle(ui, Rect::from_min_size(pos2(inner.left(), y), vec2(tw, 24.0)), Id::new(("touch_pulse", tid)), "Pulse", pulse_on || arp, Some(if pulse_on || arp { lamp } else { 0.0 }), "The pulser: plays the held (or latched) plate again in time, also with the song stopped (an arpeggio always pulses)") {
            flip(&mut dev.params[touch::PULSE]);
            changed = true;
        }
        let latch_on = dev.params[touch::LATCH] >= 0.5;
        if toggle(ui, Rect::from_min_size(pos2(inner.left() + tw + 4.0, y), vec2(tw, 24.0)), Id::new(("touch_latch", tid)), "Latch", latch_on, None, "Keeps the last plate (or chord) going after you let go; the next touch takes over") {
            flip(&mut dev.params[touch::LATCH]);
            changed = true;
        }
        let run_on = dev.params[touch::RUN] >= 0.5;
        if toggle(ui, Rect::from_min_size(pos2(inner.left() + 2.0 * (tw + 4.0), y), vec2(tw, 24.0)), Id::new(("touch_run", tid)), "Run", run_on, None, "Plays with no plate held: the pulser (and the sequence) from the base note; a plate moves it") {
            flip(&mut dev.params[touch::RUN]);
            changed = true;
        }
        y += 24.0 + 5.0;
        changed |= choice(ui, Rect::from_min_size(pos2(inner.left(), y), vec2(inner.width(), row_h)), Id::new(("touch_rate", tid)), &specs[touch::RATE], &mut dev.params[touch::RATE]);
        y += row_h + 6.0;
        let kn = Rect::from_min_max(pos2(inner.left(), y), inner.max);
        let mode = dev.params[touch::PLAY].round() as usize;
        let extra: [usize; 2] = match mode {
            1 => [touch::ARP_ORDER, touch::OCTAVES],
            2 => [touch::STRUM, touch::PRESS_VEL],
            3 | 4 => [touch::ECHOES, touch::FEEDBACK],
            _ => [touch::PRESS_VEL, touch::DECAY],
        };
        // Each a ladder of values to click.
        let cells = [touch::GATE, touch::SWING, touch::CHANCE, touch::WANDER, extra[0], extra[1]];
        let sw = kn.width() / cells.len() as f32;
        for (n, k) in cells.into_iter().enumerate() {
            let color = if n < 2 { OUT_COLORS[1] } else if n < 4 { OUT_COLORS[3] } else { accent };
            let cell = Rect::from_min_size(pos2(kn.left() + n as f32 * sw, kn.top()), vec2(sw, kn.height())).shrink2(vec2(1.5, 2.0));
            let name = match k {
                touch::PRESS_VEL => "Press",
                touch::ARP_ORDER => "Order",
                touch::FEEDBACK => "Fade",
                _ => specs[k].name,
            };
            changed |= ladder(ui, cell, Id::new(("touch_knob", tid, k)), &specs[k], name, &rungs(k, &specs[k]), &mut dev.params[k], color);
        }

        // ---------------------------------------------------------------- sequence
        let inner = section(&p, seq_r, "SEQUENCE");
        let seq_on = dev.params[touch::SEQ] >= 0.5;
        let tbw = 64.0;
        let tby = seq_r.top() + 4.0;
        let mut x = seq_r.right() - 10.0 - 3.0 * (tbw + 4.0);
        for (k, label, tip) in [
            (touch::SEQ, "On", "The sequencer on: each pulse (or each touch, without the pulser) a step further"),
            (touch::TRIGGER, "Trigger", "The steps decide which notes play: each step's gate over its pulses, its chance, ratchets, accents, ties"),
            (touch::PITCH, "Pitch", "The steps move the notes: through the key (the Note lane), or on the Tonnetz to their chords (the Chord lane)"),
        ] {
            let on = dev.params[k] >= 0.5;
            let lit = if k == touch::SEQ { on } else { on && seq_on };
            if toggle(ui, Rect::from_min_size(pos2(x, tby), vec2(tbw, 20.0)), Id::new(("touch_seqsw", tid, k)), label, lit, (k == touch::SEQ).then_some(if seq_on { lamp } else { 0.0 }), tip) {
                flip(&mut dev.params[k]);
                changed = true;
            }
            x += tbw + 4.0;
        }
        let tonnetz = dev.params[touch::LAYOUT].round() as usize == touch::TONNETZ_LAYOUT;
        let n_on = (dev.params[touch::STEPS].round() as usize).clamp(1, N_STEPS);
        // The tools.
        let tools = Rect::from_min_size(inner.min, vec2(inner.width(), 22.0));
        let order_w = (tools.width() * 0.26).clamp(96.0, 150.0);
        changed |= choice(ui, Rect::from_min_size(tools.min, vec2(order_w, 22.0)), Id::new(("touch_order", tid)), &specs[touch::ORDER], &mut dev.params[touch::ORDER]);
        let names: [(&str, &str); 7] = [
            ("Rec", "Record: the plates you play go into the steps, one after the other (the sequencer goes on)"),
            ("Dice", "A new sequence: a walk through the key, with some rests, longer steps, gates, ratchets, accents and chances"),
            ("Mutate", "Changes a few steps a little"),
            ("◀", "Shift the steps one to the left"),
            ("▶", "Shift the steps one to the right"),
            ("Rev", "Reverse the steps"),
            ("Clear", "Every step plain: the base note, one pulse, Single, no ratchets"),
        ];
        let bw = ((tools.width() - order_w - 6.0) / names.len() as f32 - 3.0).max(24.0);
        let mut tool = None;
        for (k, (label, tip)) in names.iter().enumerate() {
            let br = Rect::from_min_size(pos2(tools.left() + order_w + 6.0 + k as f32 * (bw + 3.0), tools.top()), vec2(bw, 22.0));
            if k == 0 {
                let on = self.touch_view.rec.is_some();
                let resp = ui.interact(br, Id::new(("touch_rec", tid)), Sense::click()).on_hover_text(*tip);
                p.rect_filled(br, 4.0, if on { shade(REC, 0.55) } else if resp.hovered() { mix(PANEL3, TEXT, 0.1) } else { PANEL3 });
                p.circle_filled(pos2(br.left() + 9.0, br.center().y), 3.5, if on { REC } else { shade(REC, 0.5) });
                p.text(pos2(br.center().x + 4.0, br.center().y), Align2::CENTER_CENTER, *label, FontId::proportional(11.0), TEXT);
                if resp.clicked() {
                    self.touch_view.rec = if on { None } else { Some(0) };
                }
            } else if button(ui, br, Id::new(("touch_tool", tid, k)), label, tip) {
                tool = Some(k);
            }
        }
        if let Some(k) = tool {
            let n = n_on;
            match k {
                1 => dice(&mut dev.data, n, now),
                2 => mutate(&mut dev.data, n, now),
                3 | 4 => {
                    let all: Vec<Step> = (0..n).map(|i| touch::step(&dev.data, i)).collect();
                    for i in 0..n {
                        let from = if k == 3 { (i + 1) % n } else { (i + n - 1) % n };
                        touch::set_step(&mut dev.data, i, all[from]);
                    }
                }
                5 => {
                    let all: Vec<Step> = (0..n).map(|i| touch::step(&dev.data, i)).collect();
                    for i in 0..n {
                        touch::set_step(&mut dev.data, i, all[n - 1 - i]);
                    }
                }
                _ => {
                    for i in 0..N_STEPS {
                        touch::set_step(&mut dev.data, i, Step { value: 0.5, ..Step::default() });
                    }
                }
            }
            if k == 1 || k == 2 {
                dev.params[touch::SEQ] = 1.0;
            }
            changed = true;
        }
        // The lanes.
        let lanes = Rect::from_min_max(pos2(inner.left(), tools.bottom() + 8.0), inner.max);
        let label_w = 46.0;
        let grid = Rect::from_min_max(pos2(lanes.left() + label_w, lanes.top()), lanes.max);
        let col_w = grid.width() / N_STEPS as f32;
        let num_h = 15.0;
        let gh = grid.height() - num_h - 4.0;
        // The small lanes keep their height; the note lane takes most of the rest.
        let small = [24.0, 13.0, 13.0, 13.0];
        let rest_h = (gh - small.iter().sum::<f32>()).max(30.0);
        let heights = [rest_h * 0.46, rest_h * 0.24, rest_h * 0.30, small[0], small[1], small[2], small[3]];
        let tab = self.touch_view.lane_tab.min(2);
        let lane_names = ["Note", "Pulses", "", "Gate", "Tie", "Accent", "Skip"];
        let tab_names = ["Value", "Chance", "Ratchet"];
        let tab_tips = [
            "Each step's value, for the Steps output (patch it to a knob). Drag across to draw",
            "How likely each of the step's notes plays (with Trigger). Click: 100, 75, 50, 25 %, round again; right-click: back; drag across to copy",
            "How many notes each of the step's notes is split into, one to eight (with Trigger and the pulser). Click: one more, right-click: one fewer; drag across to copy",
        ];
        let tips = [
            if tonnetz {
                "Each step's chord, a triangle of the Tonnetz; with the sequencer On they play by themselves, one a pulse (Rate). Rec writes the triangles you press into them (click a step to start there); Dice makes new ones; right-click: a rest"
            } else {
                "Each step's note, up and down through the key (drag across to draw; double-click: back to the base; right-click: a rest)"
            },
            "How many pulses each step lasts, one to eight. Click: one more (after eight: one), right-click: one fewer; drag across to copy",
            tab_tips[tab],
            "What each step plays on its pulses (with Trigger): Single (the first), Every (each one), Hold (one note over them all), Rest, Last, Alternate (every other one), Roll (one more note each pulse). Click: the next, right-click: the one before; drag across to copy",
            "Steps held into the next (legato, with Trigger). Click to switch, drag across for more",
            "Steps played louder (with Trigger). Click to switch, drag across for more",
            "Steps passed over. Click to switch, drag across for more",
        ];
        let mut lane_rects = [Rect::NOTHING; 7];
        let mut ly = grid.top();
        for (k, &h) in heights.iter().enumerate() {
            lane_rects[k] = Rect::from_min_size(pos2(grid.left(), ly), vec2(grid.width(), (h - 2.0).max(4.0)));
            ly += h;
        }
        let rec_at = self.touch_view.rec;
        // The step playing, and the one recorded next.
        for i in 0..N_STEPS {
            let col = Rect::from_min_max(pos2(grid.left() + i as f32 * col_w, grid.top()), pos2(grid.left() + (i + 1) as f32 * col_w, grid.bottom() - num_h - 2.0));
            if seq_on && i == seen.step && i < n_on {
                p.rect_filled(col.shrink2(vec2(1.0, 0.0)), 3.0, shade(accent, 0.16 + 0.12 * lamp));
            }
            if rec_at == Some(i) {
                p.rect_stroke(col.shrink(1.0), 3.0, Stroke::new(1.5_f32, REC), egui::StrokeKind::Inside);
            }
        }
        // The note and the value are drawn; the rest is clicked.
        let clicked_lane = |k: usize| k != 0 && !(k == 2 && tab == 0);
        for (k, lr) in lane_rects.iter().enumerate() {
            if k == 2 {
                // Its tab: what it shows, a click for the next.
                let th = lr.height().min(22.0);
                let tr = Rect::from_min_size(pos2(lanes.left() - 2.0, lr.center().y - th * 0.5), vec2(label_w - 6.0, th));
                let resp = ui.interact(tr, Id::new(("touch_tab", tid)), Sense::click()).on_hover_text(format!("Shows: {}. Click: {} (right-click: {})", tab_names[tab], tab_names[(tab + 1) % 3], tab_names[(tab + 2) % 3]));
                p.rect_filled(tr, 3.0, if resp.hovered() { mix(PANEL3, TEXT, 0.1) } else { PANEL3 });
                p.text(tr.center(), Align2::CENTER_CENTER, tab_names[tab], fitted(&p, tab_names[tab], 9.5, tr.width() - 4.0), TEXT);
                if resp.clicked() {
                    self.touch_view.lane_tab = (tab + 1) % 3;
                } else if resp.secondary_clicked() {
                    self.touch_view.lane_tab = (tab + 2) % 3;
                }
            } else {
                let name = if k == 0 && tonnetz { "Chord" } else { lane_names[k] };
                p.text(pos2(lanes.left(), lr.center().y), Align2::LEFT_CENTER, name, FontId::proportional(if lr.height() < 12.0 { 8.5 } else { 10.0 }), TEXT_DIM);
            }
            let col_of = |x: f32| (((x - lr.left()) / col_w).floor().max(0.0) as usize).min(N_STEPS - 1);
            // The Gate lane says what the step under the pointer does.
            let hovered = ui.input(|i| i.pointer.hover_pos()).filter(|pt| lr.contains(*pt)).map(|pt| col_of(pt.x));
            let tip = match hovered.filter(|_| k == 3) {
                Some(i) => {
                    let st = touch::step(&dev.data, i);
                    format!("Step {}: {}, {} pulse{}, ratchet ×{}\n\n{}", i + 1, st.gate.name(), st.pulses, if st.pulses == 1 { "" } else { "s" }, st.ratchet, tips[k])
                }
                None => tips[k].to_string(),
            };
            let resp = ui.interact(*lr, Id::new(("touch_lane", tid, k)), Sense::click_and_drag()).on_hover_text(tip);
            let frac = |yy: f32| ((lr.bottom() - yy) / lr.height()).clamp(0.0, 1.0);
            if clicked_lane(k) {
                // A click turns the step one further, a right-click one back; dragged on, the steps passed over get the same.
                if resp.drag_started() || resp.clicked() || resp.secondary_clicked() {
                    if let Some(pt) = ui.input(|i| i.pointer.press_origin()).or(resp.interact_pointer_pos()) {
                        let back = resp.secondary_clicked() || ui.input(|i| i.pointer.button_down(egui::PointerButton::Secondary));
                        let i = col_of(pt.x);
                        let s = turn(touch::step(&dev.data, i), k, tab, if back { -1 } else { 1 });
                        touch::set_step(&mut dev.data, i, s);
                        self.touch_view.paint = Some(s);
                        changed = true;
                    }
                }
                if let (Some(pt), Some(from)) = (resp.interact_pointer_pos().filter(|_| resp.dragged()), self.touch_view.paint) {
                    let i = col_of(pt.x);
                    let s = touch::step(&dev.data, i);
                    let to = copy_lane(s, from, k, tab);
                    if to != s {
                        touch::set_step(&mut dev.data, i, to);
                        changed = true;
                    }
                }
            } else if k == 0 && tonnetz {
                // A step's chord: click it, and the triangles pressed go in from there.
                if let Some(pt) = resp.interact_pointer_pos().filter(|_| resp.clicked()) {
                    self.touch_view.rec = Some(col_of(pt.x));
                }
            } else if let Some(pt) = resp.interact_pointer_pos().filter(|_| resp.dragged() || resp.clicked()) {
                let i = col_of(pt.x);
                let mut s = touch::step(&dev.data, i);
                if k == 0 {
                    s.note = ((frac(pt.y) * 2.0 - 1.0) * 14.0).round() as i32;
                } else {
                    s.value = frac(pt.y);
                }
                touch::set_step(&mut dev.data, i, s);
                changed = true;
            }
            if resp.double_clicked() && k == 0 {
                if let Some(pt) = resp.interact_pointer_pos() {
                    let i = col_of(pt.x);
                    let s = touch::step(&dev.data, i);
                    touch::set_step(&mut dev.data, i, Step { note: 0, ..s });
                    changed = true;
                }
            }
            if resp.secondary_clicked() && k == 0 {
                if let Some(pt) = resp.interact_pointer_pos() {
                    let i = col_of(pt.x);
                    let s = touch::step(&dev.data, i);
                    let gate = if s.rests() { Gate::Single } else { Gate::Rest };
                    touch::set_step(&mut dev.data, i, Step { gate, ..s });
                    changed = true;
                }
            }
            for i in 0..N_STEPS {
                let c = Rect::from_min_size(pos2(lr.left() + i as f32 * col_w, lr.top()), vec2(col_w, lr.height())).shrink2(vec2(1.5, 0.0));
                let s = touch::step(&dev.data, i);
                let active = i < n_on;
                let now = seq_on && active && i == seen.step;
                let col = |base: Color32| if !active || s.skip { shade(base, 0.25) } else if seq_on { base } else { shade(base, 0.55) };
                let under = clicked_lane(k) && hovered == Some(i);
                p.rect_filled(c, 2.0, if under { mix(SLOT, TEXT, 0.12) } else { SLOT });
                match k {
                    0 if tonnetz => {
                        // Its chord's name; the step a triangle goes into marked.
                        let name = chord_name(base_note, s.tri);
                        let tc = if s.rests() || !active { TEXT_DIM } else { col(TEXT) };
                        if active && s.tri.minor {
                            p.rect_filled(c.shrink(1.0), 2.0, mix(SLOT, OUT_COLORS[5], 0.12));
                        } else if active {
                            p.rect_filled(c.shrink(1.0), 2.0, mix(SLOT, OUT_COLORS[0], 0.12));
                        }
                        p.text(c.center(), Align2::CENTER_CENTER, &name, fitted(&p, &name, 11.0, c.width() - 2.0), tc);
                    }
                    0 => {
                        let mid = c.center().y;
                        let yv = mid - s.note as f32 / 14.0 * c.height() * 0.5;
                        p.line_segment([pos2(c.left(), mid), pos2(c.right(), mid)], Stroke::new(1.0_f32, LINE));
                        let bar_col = if s.rests() { shade(TEXT_DIM, 0.4) } else { col(OUT_COLORS[2]) };
                        p.rect_filled(Rect::from_two_pos(pos2(c.left() + 1.0, mid), pos2(c.right() - 1.0, yv)), 1.5, shade(bar_col, 0.7));
                        p.line_segment([pos2(c.left() + 1.0, yv), pos2(c.right() - 1.0, yv)], Stroke::new(2.0_f32, bar_col));
                        if col_w >= 20.0 && s.note != 0 {
                            p.text(pos2(c.center().x, c.top() + 1.0), Align2::CENTER_TOP, format!("{:+}", s.note), FontId::proportional(9.0), if active { TEXT } else { TEXT_DIM });
                        }
                    }
                    1 | 2 if clicked_lane(k) => {
                        // Its count (or chance) written large, and under it a mark for each pulse (the one it is at lit), each ratchet note, or how likely.
                        let (text, plain, marks, cc) = match k {
                            1 => (s.pulses.to_string(), s.pulses <= 1, s.pulses.clamp(1, MAX_PULSES) as usize, OUT_COLORS[1]),
                            _ if tab == 1 => (format!("{}%", (s.chance * 100.0).round()), s.chance >= 0.999, 0, OUT_COLORS[3]),
                            _ => (format!("×{}", s.ratchet.max(1)), s.ratchet <= 1, s.ratchet.clamp(1, MAX_RATCHET) as usize, accent),
                        };
                        let foot = if c.height() >= 16.0 { 5.0 } else { 0.0 };
                        let tc = if !active { shade(TEXT_DIM, 0.5) } else if plain { TEXT_DIM } else { TEXT };
                        let size = ((c.height() - foot) * 0.62).clamp(8.0, 12.5);
                        p.text(pos2(c.center().x, c.center().y - foot * 0.5), Align2::CENTER_CENTER, &text, fitted(&p, &text, size, c.width() - 2.0), tc);
                        if foot > 0.0 {
                            let fr = Rect::from_min_max(pos2(c.left() + 2.0, c.bottom() - 4.0), pos2(c.right() - 2.0, c.bottom() - 1.5));
                            if marks == 0 {
                                p.rect_filled(Rect::from_min_size(fr.min, vec2(fr.width() * s.chance, fr.height())), 1.0, shade(col(cc), 0.8));
                            } else {
                                let mw = fr.width() / marks as f32;
                                for j in 0..marks {
                                    let m = Rect::from_min_size(pos2(fr.left() + j as f32 * mw, fr.top()), vec2((mw - 1.0).max(0.8), fr.height()));
                                    let lit = k == 1 && now && j == seen.pulse;
                                    p.rect_filled(m, 0.5, if lit { mix(cc, TEXT, 0.6 * lamp + 0.2) } else { shade(col(cc), 0.8) });
                                }
                            }
                        }
                    }
                    2 => {
                        let v = s.value;
                        p.rect_filled(Rect::from_min_max(pos2(c.left() + 1.0, c.bottom() - v * c.height()), pos2(c.right() - 1.0, c.bottom())), 1.5, shade(col(OUT_COLORS[2]), 0.75));
                    }
                    3 => {
                        // Its name, and what it plays on each of its pulses.
                        let g = s.gate.index();
                        let label = if col_w >= 34.0 { ["1st", "Every", "Hold", "Rest", "Last", "Alt", "Roll"][g] } else { ["1", "E", "H", "-", "L", "A", "R"][g] };
                        p.text(pos2(c.center().x, c.top() + 1.0), Align2::CENTER_TOP, label, FontId::proportional(8.5), if s.rests() || !active { TEXT_DIM } else { col(TEXT) });
                        let pr = Rect::from_min_max(pos2(c.left() + 2.0, c.bottom() - 7.0), pos2(c.right() - 2.0, c.bottom() - 2.0));
                        let n = s.pulses.clamp(1, MAX_PULSES);
                        let sw = pr.width() / n as f32;
                        for j in 0..n {
                            let slot = Rect::from_min_size(pos2(pr.left() + j as f32 * sw, pr.top()), vec2(sw, pr.height()));
                            let lit = now && j as usize == seen.pulse;
                            let cc = if lit { TEXT } else { col(accent) };
                            match touch::plays(&s, j) {
                                (_, true) => {
                                    p.rect_filled(Rect::from_min_max(pos2(slot.left() + 0.5, slot.top()), pos2(pr.right() - 0.5, slot.bottom())), 1.0, if now { TEXT } else { col(accent) });
                                    break;
                                }
                                (0, _) => {
                                    p.rect_filled(Rect::from_center_size(slot.center(), vec2((sw - 1.5).max(1.0), 1.0)), 0.0, if lit { TEXT_DIM } else { shade(TEXT_DIM, 0.4) });
                                }
                                (notes, _) => {
                                    let tw = (sw - 1.0) / notes as f32;
                                    for t in 0..notes {
                                        let x = slot.left() + 0.5 + t as f32 * tw;
                                        p.rect_filled(Rect::from_min_max(pos2(x, slot.top()), pos2(x + (tw - 0.8).max(0.8), slot.bottom())), 0.0, cc);
                                    }
                                }
                            }
                        }
                    }
                    _ => {
                        let on = match k {
                            4 => s.tie,
                            5 => s.accent,
                            _ => s.skip,
                        };
                        if on {
                            let cc = match k {
                                4 => SELECT,
                                5 => REC,
                                _ => TEXT_DIM,
                            };
                            p.rect_filled(c.shrink(1.5), 2.0, if active { cc } else { shade(cc, 0.25) });
                        }
                    }
                }
            }
        }
        // The step numbers: a click sets how many.
        for i in 0..N_STEPS {
            let nb = Rect::from_min_size(pos2(grid.left() + i as f32 * col_w, grid.bottom() - num_h), vec2(col_w, num_h));
            let resp = ui.interact(nb, Id::new(("touch_n", tid, i)), Sense::click()).on_hover_text("Click: this many steps");
            if resp.clicked() {
                dev.params[touch::STEPS] = (i + 1) as f32;
                changed = true;
            }
            let active = i < n_on;
            p.text(nb.center(), Align2::CENTER_CENTER, format!("{}", i + 1), FontId::proportional(9.5), if active { TEXT } else { shade(TEXT_DIM, 0.6) });
            if i + 1 == n_on {
                p.line_segment([pos2(nb.right() - 1.0, nb.top()), pos2(nb.right() - 1.0, nb.bottom())], Stroke::new(2.0_f32, accent));
            }
        }
        p.text(pos2(lanes.left(), grid.bottom() - num_h * 0.5), Align2::LEFT_CENTER, format!("{n_on} steps"), FontId::proportional(9.5), TEXT_DIM);

        // ---------------------------------------------------------------- patch bay
        let inner = section(&p, patch_r, "PATCH");
        let data = self.project.tracks[ti].midi_devices[di].data.clone();
        let cables = self.touch_cables(tid, &data);
        let src_h = (inner.height() / N_OUT as f32).min(30.0);
        let dst_h = (inner.height() / N_PATCH as f32).min(34.0);
        let jack_r = (src_h * 0.3).clamp(5.0, 7.0);
        let src_x = inner.left() + 66.0;
        let dst_x = inner.right() - (inner.width() * 0.42).clamp(110.0, 170.0);
        let src_at = |k: usize| pos2(src_x, inner.top() + src_h * (k as f32 + 0.5));
        let dst_at = |k: usize| pos2(dst_x, inner.top() + dst_h * (k as f32 + 0.5));
        let pointer = ui.input(|i| i.pointer.interact_pos());
        let released = ui.input(|i| i.pointer.any_released());
        let mut connect: Option<(usize, usize)> = None;
        let mut unpatch: Option<u64> = None;
        let mut amounts: Vec<(u64, f32)> = Vec::new();
        let mut set_slot: Option<(usize, Option<u16>)> = None;
        for k in 0..N_OUT {
            let c = src_at(k);
            let jr = Rect::from_center_size(c, vec2(jack_r * 2.8, jack_r * 2.8));
            let resp = ui.interact(jr, Id::new(("touch_src", tid, k)), Sense::drag()).on_hover_text(format!("{}: {}. Drag to a knob on the right", OUTPUTS[k], OUT_HELP[k]));
            if resp.drag_started() {
                self.touch_view.drawing = Some((true, k));
            }
            if resp.hovered() || resp.dragged() {
                ui.ctx().set_cursor_icon(CursorIcon::Grab);
            }
            let v = outs[k];
            p.circle_filled(c, jack_r + 2.5, mix(PANEL2, OUT_COLORS[k], 0.15 + 0.6 * v));
            p.circle_filled(c, jack_r, Color32::from_rgb(0x10, 0x10, 0x12));
            p.circle_stroke(c, jack_r, Stroke::new(2.0_f32, OUT_COLORS[k]));
            p.text(pos2(c.x - jack_r - 6.0, c.y), Align2::RIGHT_CENTER, OUTPUTS[k], FontId::proportional(10.5), TEXT);
            let m = Rect::from_min_size(pos2(c.x + jack_r + 5.0, c.y - 1.5), vec2(22.0, 3.0));
            p.rect_filled(m, 1.5, SLOT);
            p.rect_filled(Rect::from_min_size(m.min, vec2(m.width() * v, m.height())), 1.5, OUT_COLORS[k]);
        }
        for s in 0..N_PATCH {
            let c = dst_at(s);
            let param = touch::patch(&data, s);
            let jr = Rect::from_center_size(c, vec2(jack_r * 2.8, jack_r * 2.8));
            let resp = ui.interact(jr, Id::new(("touch_dst", tid, s)), Sense::drag()).on_hover_text("Drag to an output on the left to patch it here");
            if resp.drag_started() && param.is_some() {
                self.touch_view.drawing = Some((false, s));
            }
            let moved: f32 = cables.iter().filter(|c| c.1 == s).map(|c| c.3 * outs[c.0]).sum();
            p.circle_filled(c, jack_r + 2.5, mix(PANEL2, accent, (moved.abs() * 0.8).min(0.8)));
            p.circle_filled(c, jack_r, Color32::from_rgb(0x10, 0x10, 0x12));
            p.circle_stroke(c, jack_r, Stroke::new(2.0_f32, if param.is_some() { TEXT } else { LINE }));
            let label = Rect::from_min_max(pos2(c.x + jack_r + 6.0, c.y - 10.0), pos2(inner.right(), c.y + 10.0));
            let lresp = ui.interact(label, Id::new(("touch_dst_pick", tid, s)), Sense::click()).on_hover_text("The knob this point moves: click for another");
            p.rect_filled(label, 4.0, if lresp.hovered() { PANEL3 } else { SLOT });
            let name = param.and_then(|i| inst_specs.get(i as usize)).map(|sp| sp.name).unwrap_or("—");
            p.text(pos2(label.left() + 7.0, label.center().y), Align2::LEFT_CENTER, name, FontId::proportional(11.0), if param.is_some() { TEXT } else { TEXT_DIM });
            if moved.abs() > 0.002 {
                let mid = label.center().x;
                let wbar = moved.clamp(-1.0, 1.0) * label.width() * 0.45;
                p.rect_filled(Rect::from_two_pos(pos2(mid, label.bottom() - 3.0), pos2(mid + wbar, label.bottom() - 1.0)), 1.0, accent);
            }
            egui::Popup::menu(&lresp).show(|ui| {
                if ui.selectable_label(param.is_none(), "—  (nothing)").clicked() {
                    set_slot = Some((s, None));
                    ui.close();
                }
                egui::ScrollArea::vertical().max_height(360.0).show(ui, |ui| {
                    let mut group = "";
                    for (i, sp) in inst_specs.iter().enumerate() {
                        if !continuous(sp) {
                            continue;
                        }
                        if sp.group != group {
                            group = sp.group;
                            ui.label(egui::RichText::new(group).small().color(TEXT_DIM));
                        }
                        if ui.selectable_label(param == Some(i as u16), sp.name).clicked() {
                            set_slot = Some((s, Some(i as u16)));
                            ui.close();
                        }
                    }
                });
            });
        }
        for &(k, s, rid, amount) in &cables {
            let pts = bezier(src_at(k), dst_at(s));
            let glow = 0.45 + 0.55 * outs[k];
            p.add(CubicBezierShape::from_points_stroke(pts, false, Color32::TRANSPARENT, Stroke::new(4.0_f32, shade(OUT_COLORS[k], 0.3))));
            p.add(CubicBezierShape::from_points_stroke(pts, false, Color32::TRANSPARENT, Stroke::new(2.2_f32, shade(OUT_COLORS[k], glow))));
            let mid = bezier_mid(&pts);
            let kr = Rect::from_center_size(mid, vec2(20.0, 20.0));
            let pname = touch::patch(&data, s).and_then(|i| inst_specs.get(i as usize)).map(|sp| sp.name).unwrap_or("?");
            let resp = ui
                .interact(kr, Id::new(("touch_amt", rid)), Sense::click_and_drag())
                .on_hover_text(format!("{} → {}: {:+.0}%. Drag up or down (Shift: fine); double-click takes the cable away", OUTPUTS[k], pname, amount * 100.0));
            let mut a = amount;
            if resp.dragged() {
                let fine = ui.input(|i| i.modifiers.shift);
                a = (a - resp.drag_delta().y * if fine { 0.002 } else { 0.008 }).clamp(-1.0, 1.0);
                amounts.push((rid, a));
            }
            if resp.double_clicked() || resp.secondary_clicked() {
                unpatch = Some(rid);
            }
            p.circle_filled(mid, 9.0, Color32::from_rgb(0x14, 0x14, 0x16));
            p.circle_stroke(mid, 9.0, Stroke::new(1.0_f32, shade(OUT_COLORS[k], 0.6)));
            let start = -std::f32::consts::FRAC_PI_2;
            let arc: Vec<Pos2> = (0..=16)
                .map(|j| {
                    let t = start + a * std::f32::consts::PI * j as f32 / 16.0;
                    mid + vec2(t.cos(), t.sin()) * 6.0
                })
                .collect();
            p.add(egui::Shape::line(arc, Stroke::new(2.0_f32, OUT_COLORS[k])));
        }
        if let (Some((from_src, k)), Some(pt)) = (self.touch_view.drawing, pointer) {
            let (a, col) = if from_src { (src_at(k), OUT_COLORS[k]) } else { (dst_at(k), TEXT) };
            let pts = if from_src { bezier(a, pt) } else { bezier(pt, a) };
            p.add(CubicBezierShape::from_points_stroke(pts, false, Color32::TRANSPARENT, Stroke::new(2.2_f32, col)));
            if released {
                let near = |c: Pos2| pt.distance(c) < jack_r * 2.4;
                if from_src {
                    if let Some(s) = (0..N_PATCH).find(|&s| near(dst_at(s)) && touch::patch(&data, s).is_some()) {
                        connect = Some((k, s));
                    }
                } else if let Some(o) = (0..N_OUT).find(|&o| near(src_at(o))) {
                    connect = Some((o, k));
                }
                self.touch_view.drawing = None;
            }
        }

        // ---------------------------------------------------------------- motion
        let inner = section(&p, motion_r, "MOTION");
        let dev = &mut self.project.tracks[ti].midi_devices[di];
        let side = inner.height().min(inner.width() * 0.36);
        let pad = Rect::from_min_size(inner.min, vec2(side, side));
        let spring = dev.params[touch::SPRING] >= 0.5;
        let pad_resp = ui.interact(pad, Id::new(("touch_xy", tid)), Sense::click_and_drag()).on_hover_text("The XY pad: its X and Y are outputs to patch. With Spring it goes back to the middle when you let go");
        p.rect_filled(pad, 6.0, Color32::from_rgb(0x12, 0x12, 0x15));
        for j in 1..4 {
            let f = j as f32 / 4.0;
            p.line_segment([pos2(pad.left() + pad.width() * f, pad.top() + 3.0), pos2(pad.left() + pad.width() * f, pad.bottom() - 3.0)], Stroke::new(1.0_f32, LINE_SOFT));
            p.line_segment([pos2(pad.left() + 3.0, pad.top() + pad.height() * f), pos2(pad.right() - 3.0, pad.top() + pad.height() * f)], Stroke::new(1.0_f32, LINE_SOFT));
        }
        let mut xy = self.touch_view.xy.unwrap_or((0.5, 0.5));
        let pad_down = pad_resp.is_pointer_button_down_on();
        if let Some(pt) = pad_resp.interact_pointer_pos().filter(|_| pad_down) {
            xy = (((pt.x - pad.left()) / pad.width()).clamp(0.0, 1.0), ((pad.bottom() - pt.y) / pad.height()).clamp(0.0, 1.0));
            ui.ctx().set_cursor_icon(CursorIcon::Crosshair);
        } else if spring {
            xy = (0.5, 0.5);
        }
        self.touch_view.xy = Some(xy);
        let dot = pos2(pad.left() + xy.0 * pad.width(), pad.bottom() - xy.1 * pad.height());
        p.line_segment([pos2(dot.x, pad.top() + 2.0), pos2(dot.x, pad.bottom() - 2.0)], Stroke::new(1.0_f32, shade(OUT_COLORS[8], 0.5)));
        p.line_segment([pos2(pad.left() + 2.0, dot.y), pos2(pad.right() - 2.0, dot.y)], Stroke::new(1.0_f32, shade(OUT_COLORS[9], 0.5)));
        p.circle_filled(dot, 8.0, shade(accent, 0.35));
        p.circle_stroke(dot, 8.0, Stroke::new(2.0_f32, accent));
        p.rect_stroke(pad, 6.0, Stroke::new(1.0_f32, LINE), egui::StrokeKind::Inside);
        // Beside it: the LFO and the shape.
        let col_w = (inner.width() - side - 16.0) * 0.56;
        let colr = Rect::from_min_max(pos2(pad.right() + 8.0, inner.top()), pos2(pad.right() + 8.0 + col_w, inner.bottom()));
        let envr = Rect::from_min_max(pos2(colr.right() + 8.0, inner.top()), inner.max);
        let ch = 20.0;
        let hw2 = (colr.width() - 4.0) / 2.0;
        p.text(pos2(colr.left(), colr.top() + 6.0), Align2::LEFT_CENTER, "LFO", FontId::proportional(10.0), TEXT_DIM);
        changed |= choice(ui, Rect::from_min_size(pos2(colr.left(), colr.top() + 13.0), vec2(hw2, ch)), Id::new(("touch_lfo_shape", tid)), &specs[touch::LFO_SHAPE], &mut dev.params[touch::LFO_SHAPE]);
        changed |= choice(ui, Rect::from_min_size(pos2(colr.left() + hw2 + 4.0, colr.top() + 13.0), vec2(hw2, ch)), Id::new(("touch_lfo_rate", tid)), &specs[touch::LFO_RATE], &mut dev.params[touch::LFO_RATE]);
        let lfo_m = Rect::from_min_size(pos2(colr.left(), colr.top() + 13.0 + ch + 4.0), vec2(colr.width(), 4.0));
        p.rect_filled(lfo_m, 2.0, SLOT);
        p.rect_filled(Rect::from_min_size(lfo_m.min, vec2(lfo_m.width() * outs[6], 4.0)), 2.0, OUT_COLORS[6]);
        p.text(pos2(colr.left(), lfo_m.bottom() + 10.0), Align2::LEFT_CENTER, "SHAPE", FontId::proportional(10.0), TEXT_DIM);
        let sy = lfo_m.bottom() + 17.0;
        let bottom_row = colr.bottom() - ch;
        let shape_r = Rect::from_min_max(pos2(colr.left(), sy), pos2(colr.right(), (bottom_row - 4.0 - ch - 4.0).max(sy + 14.0)));
        let sresp = ui.interact(shape_r, Id::new(("touch_shape", tid)), Sense::click_and_drag()).on_hover_text("The shape: draw it (it loops at its rate, an output to patch)");
        p.rect_filled(shape_r, 4.0, SLOT);
        if let Some(pt) = sresp.interact_pointer_pos().filter(|_| sresp.dragged() || sresp.clicked()) {
            let j = (((pt.x - shape_r.left()) / shape_r.width() * N_SHAPE as f32).floor().max(0.0) as usize).min(N_SHAPE - 1);
            touch::set_shape(&mut dev.data, j, (shape_r.bottom() - pt.y) / shape_r.height());
            changed = true;
        }
        let pts: Vec<Pos2> = (0..=64)
            .map(|j| {
                let f = j as f32 / 64.0;
                pos2(shape_r.left() + f * shape_r.width(), shape_r.bottom() - touch::shape_at(&dev.data, f) * shape_r.height())
            })
            .collect();
        p.add(egui::Shape::line(pts, Stroke::new(1.5_f32, OUT_COLORS[7])));
        p.circle_filled(pos2(shape_r.left() + 3.0, shape_r.bottom() - outs[7] * shape_r.height()), 3.0, OUT_COLORS[7]);
        changed |= choice(ui, Rect::from_min_size(pos2(colr.left(), shape_r.bottom() + 4.0), vec2(colr.width(), ch)), Id::new(("touch_shape_rate", tid)), &specs[touch::SHAPE_RATE], &mut dev.params[touch::SHAPE_RATE]);
        let rec_on = self.touch_view.shape_rec;
        if toggle(ui, Rect::from_min_size(pos2(colr.left(), bottom_row), vec2(hw2, ch)), Id::new(("touch_shape_rec", tid)), "Rec", rec_on, Some(if rec_on { 1.0 } else { 0.0 }), "Records the XY pad's height into the shape while you move it (the shape loops at its rate)") {
            self.touch_view.shape_rec = !rec_on;
        }
        if toggle(ui, Rect::from_min_size(pos2(colr.left() + hw2 + 4.0, bottom_row), vec2(hw2, ch)), Id::new(("touch_spring", tid)), "Spring", spring, None, "The XY pad goes back to the middle when you let go") {
            flip(&mut dev.params[touch::SPRING]);
            changed = true;
        }
        // The envelope: its mode, its meter, attack and decay; and how many levels the random has.
        p.text(pos2(envr.left(), envr.top() + 6.0), Align2::LEFT_CENTER, "ENV", FontId::proportional(10.0), TEXT_DIM);
        changed |= choice(ui, Rect::from_min_size(pos2(envr.left(), envr.top() + 13.0), vec2(envr.width(), ch)), Id::new(("touch_env_mode", tid)), &specs[touch::ENV_MODE], &mut dev.params[touch::ENV_MODE]);
        let env_m = Rect::from_min_size(pos2(envr.left(), envr.top() + 13.0 + ch + 4.0), vec2(envr.width(), 4.0));
        p.rect_filled(env_m, 2.0, SLOT);
        p.rect_filled(Rect::from_min_size(env_m.min, vec2(env_m.width() * outs[10], 4.0)), 2.0, OUT_COLORS[10]);
        let kn = Rect::from_min_max(pos2(envr.left(), env_m.bottom() + 8.0), envr.max);
        let kw = kn.width() / 3.0;
        let rh = (kn.height() / 3.0).min(28.0);
        for (n, (k, name)) in [(touch::ENV_ATTACK, "Rise"), (touch::ENV_DECAY, "Fall"), (touch::RANDOM_STEPS, "Levels")].into_iter().enumerate() {
            let color = if n < 2 { OUT_COLORS[10] } else { OUT_COLORS[3] };
            let id = Id::new(("touch_env_knob", tid, k));
            changed |= if sliders {
                // Little height here: the sliders lie down, one under the other.
                slider_across(ui, Rect::from_min_size(pos2(kn.left(), kn.top() + n as f32 * rh), vec2(kn.width(), rh)), id, &specs[k], name, &mut dev.params[k], color)
            } else {
                let cell = Rect::from_min_size(pos2(kn.left() + n as f32 * kw, kn.top()), vec2(kw, kn.height().min(84.0)));
                knob_named(ui, cell.shrink(1.0), id, &specs[k], name, &mut dev.params[k], color)
            };
        }
        // Recording the shape: the pad's height where the shape is now.
        if self.touch_view.shape_rec && pad_down {
            let beats = [0.25, 0.5, 1.0, 2.0, 4.0, 8.0, 16.0, 32.0][(dev.params[touch::SHAPE_RATE].round().max(0.0) as usize).min(7)];
            let pos = if status.playing.load(std::sync::atomic::Ordering::Relaxed) { status.song_pos.load() } else { now * self.project.bpm / 60.0 };
            let j = ((pos / beats).rem_euclid(1.0) * N_SHAPE as f64) as usize;
            touch::set_shape(&mut dev.data, j.min(N_SHAPE - 1), xy.1);
            changed = true;
        }

        // ---------------------------------------------------------------- the plates
        let dev = &mut self.project.tracks[ti].midi_devices[di];
        let on_tonnetz = dev.params[touch::LAYOUT].round() as usize == touch::TONNETZ_LAYOUT;
        let layout = (dev.params[touch::LAYOUT].round().max(0.0) as usize).min(2);
        let set = (dev.params[touch::NOTES].round().max(0.0) as usize).min(2);
        p.rect_filled(plates_r, 6.0, PANEL);
        let bar = Rect::from_min_size(plates_r.min + vec2(10.0, 6.0), vec2(plates_r.width() - 20.0, 20.0));
        p.text(bar.left_center(), Align2::LEFT_CENTER, if on_tonnetz { "TONNETZ" } else { "PLATES" }, FontId::proportional(11.0), TEXT_DIM);
        // From the right: the octave, Slide and Values; the two choices share what is left.
        let base_txt = format!("from {}", krabton_engine::note_name(base_note.clamp(0, 127) as u8));
        let base_w = p.layout_no_wrap(base_txt.clone(), FontId::proportional(11.0), TEXT).size().x;
        let slide_r = Rect::from_min_size(pos2(bar.right() - 46.0 - base_w - 10.0 - 60.0, bar.top()), vec2(60.0, 20.0));
        let values_r = Rect::from_min_size(pos2(slide_r.left() - 4.0 - 66.0, bar.top()), vec2(66.0, 20.0));
        let cw = ((values_r.left() - 8.0 - bar.left() - 60.0 - 4.0 - 20.0) / 2.0).clamp(70.0, 128.0);
        changed |= choice(ui, Rect::from_min_size(pos2(bar.left() + 60.0, bar.top()), vec2(cw, 20.0)), Id::new(("touch_layout", tid)), &specs[touch::LAYOUT], &mut dev.params[touch::LAYOUT]);
        let values_on = self.touch_view.values && !on_tonnetz;
        let slide = dev.params.get(touch::SLIDE).copied().unwrap_or(1.0) >= 0.5 && !on_tonnetz;
        if on_tonnetz {
            let hint = "a dot: its note  ·  a triangle: its chord  ·  Sequence On: the Chord lane plays by itself";
            let hr = Rect::from_min_max(pos2(bar.left() + 70.0 + cw, bar.top()), pos2(bar.right() - 46.0 - base_w - 14.0, bar.bottom()));
            if hr.width() > 60.0 {
                p.text(hr.left_center(), Align2::LEFT_CENTER, hint, fitted(&p, hint, 10.5, hr.width()), TEXT_DIM);
            }
        } else {
            changed |= choice(ui, Rect::from_min_size(pos2(bar.left() + 64.0 + cw, bar.top()), vec2(cw + 20.0, 20.0)), Id::new(("touch_notes", tid)), &specs[touch::NOTES], &mut dev.params[touch::NOTES]);
            if toggle(ui, values_r, Id::new(("touch_values", tid)), "Values", values_on, None, "Each plate's own value, the Plate output: on, drag a plate up or down to set it (it plays again when you switch this off)") {
                self.touch_view.values = !values_on;
            }
            if toggle(ui, slide_r, Id::new(("touch_slide", tid)), "Slide", slide, None, "Slide: press a plate and move along the plates, and its note bends smoothly to wherever you are (in tune around a plate's middle). Off: moving across plays plate after plate") {
                if let Some(v) = dev.params.get_mut(touch::SLIDE) {
                    *v = if slide { 0.0 } else { 1.0 };
                }
                changed = true;
            }
        }
        p.text(pos2(bar.right() - 46.0, bar.center().y), Align2::RIGHT_CENTER, base_txt, FontId::proportional(11.0), TEXT);
        for (dir, glyph, x) in [(-1, "◀", bar.right() - 34.0), (1, "▶", bar.right() - 10.0)] {
            let br = Rect::from_center_size(pos2(x, bar.center().y), vec2(18.0, 18.0));
            let r2 = ui.interact(br, Id::new(("touch_oct", tid, dir)), Sense::click()).on_hover_text(if dir < 0 { "The plates (or the Tonnetz's middle) an octave lower" } else { "The plates (or the Tonnetz's middle) an octave higher" });
            p.text(br.center(), Align2::CENTER_CENTER, glyph, FontId::proportional(12.0), if r2.hovered() { TEXT } else { TEXT_DIM });
            if r2.clicked() {
                dev.params[touch::BASE] = (dev.params[touch::BASE] + 12.0 * dir as f32).clamp(24.0, 84.0);
                changed = true;
            }
        }
        let area = Rect::from_min_max(pos2(plates_r.left() + 10.0, bar.bottom() + 8.0), plates_r.max - vec2(10.0, 8.0));
        let (want, pressed_plate, bend_to, keys_down) = if on_tonnetz {
            self.touch_view.slide_from = None;
            let want = self.tonnetz_plates(ui, area, ti, di, seq_on.then_some(seen.step), held_notes, tcol);
            (want, None, None, self.held_keys.values().any(|(_, ids)| ids.contains(&tid)))
        } else {
            let per = per_octave(&key, base_note, set);
            // The plates: rows of columns, each row a fourth (or so) above the one under it.
            let (cols, rows, step_up) = match layout {
                1 => ((per + 1) as usize, 1usize, 0),
                2 => ((per + 1) as usize, 3usize, if set == 2 { 5 } else if set == 1 { 2 } else { 3 }),
                _ => ((per * 2 + 1).min(((area.width() / 24.0) as i32).max(8)) as usize, 1usize, 0),
            };
            let gap = 5.0;
            let pw = (area.width() - gap * (cols as f32 - 1.0)) / cols as f32;
            let ph = (area.height() - gap * (rows as f32 - 1.0)) / rows as f32;
            let mut plates: Vec<(Rect, u8)> = Vec::with_capacity(cols * rows);
            let mut plate_index: Vec<usize> = Vec::with_capacity(cols * rows);
            for row in 0..rows {
                for c in 0..cols {
                    let pr = Rect::from_min_size(pos2(area.left() + c as f32 * (pw + gap), area.bottom() - (row + 1) as f32 * ph - row as f32 * gap), vec2(pw, ph));
                    let idx = c + row * step_up as usize;
                    plates.push((pr, plate_pitch(&key, base_note, set, idx as i32)));
                    plate_index.push(idx);
                }
            }
            let resp = ui.interact(area, Id::new(("touch_plates", tid)), Sense::click_and_drag());
            let plate_of = |pt: Pos2| plates.iter().position(|(r, _)| r.expand(gap * 0.5).contains(pt));
            let press_of = |pt: Pos2, r: Rect| (1.0 - (pt.y - r.top()) / r.height()).clamp(0.05, 1.0);
            let along = |pt: Pos2| ((pt.x - area.left()) / area.width()).clamp(0.0, 1.0);
            for ev in ui.input(|i| i.events.clone()) {
                if let egui::Event::Touch { id, phase, pos, force, .. } = ev {
                    let fid = id.0.wrapping_add(1);
                    let fingers = &mut self.touch_view.fingers;
                    match phase {
                        egui::TouchPhase::Start if area.contains(pos) => fingers.push((fid, pos, force)),
                        egui::TouchPhase::Move => {
                            if let Some(f) = fingers.iter_mut().find(|f| f.0 == fid) {
                                (f.1, f.2) = (pos, force);
                            }
                        }
                        egui::TouchPhase::End | egui::TouchPhase::Cancel => fingers.retain(|f| f.0 != fid),
                        _ => {}
                    }
                }
            }
            let mut want: Vec<(u8, f32, f32)> = Vec::new();
            let mut pressed_plate: Option<usize> = None;
            // Sliding: where the bend is going, and the plate under the finger (lit).
            let mut bend_to: Option<f32> = None;
            let mut slid: Option<(Pos2, u8, f32)> = None;
            if values_on {
                // Setting the plates' values: a drag sets the one under it.
                if let Some(pt) = resp.interact_pointer_pos().filter(|_| resp.is_pointer_button_down_on()) {
                    if let Some(i) = plate_of(pt) {
                        touch::set_plate_value(&mut dev.data, plate_index[i], press_of(pt, plates[i].0));
                        changed = true;
                    }
                    ui.ctx().set_cursor_icon(CursorIcon::ResizeVertical);
                }
                self.touch_view.fingers.clear();
            } else if slide {
                // One contact (the first finger, or the pointer): its note pressed
                // first, bent to where it is now.
                let touching = !self.touch_view.fingers.is_empty() || ui.input(|i| i.any_touches());
                let mouse = resp.interact_pointer_pos().filter(|_| resp.is_pointer_button_down_on() && !touching).map(|pt| (pt, None));
                let contact = self.touch_view.fingers.first().map(|f| (f.1, f.2)).or(mouse);
                match contact {
                    Some((pt, force)) => {
                        let under = plate_of(pt);
                        if self.touch_view.slide_from.is_none() {
                            if let Some(i) = under {
                                self.touch_view.slide_from = Some(plates[i].1);
                                pressed_plate = Some(plate_index[i]);
                            }
                        }
                        if let Some(from) = self.touch_view.slide_from {
                            let r = under.map(|i| plates[i].0).unwrap_or(area);
                            let pressure = force.filter(|f| *f > 0.0).map(|f| f.clamp(0.05, 1.0)).unwrap_or_else(|| press_of(pt, r));
                            want.push((from, pressure, along(pt)));
                            let to = slide_pitch(&plates, pt).unwrap_or(from as f32);
                            bend_to = Some(to - from as f32);
                            slid = Some((pt, under.map(|i| plates[i].1).unwrap_or(from), to));
                        }
                        if mouse.is_some() {
                            ui.ctx().set_cursor_icon(CursorIcon::ResizeHorizontal);
                        }
                    }
                    None => self.touch_view.slide_from = None,
                }
            } else {
                self.touch_view.slide_from = None;
                for &(_, pt, force) in &self.touch_view.fingers {
                    if let Some(i) = plate_of(pt) {
                        want.push((plates[i].1, force.filter(|f| *f > 0.0).map(|f| f.clamp(0.05, 1.0)).unwrap_or_else(|| press_of(pt, plates[i].0)), along(pt)));
                        pressed_plate = Some(plate_index[i]);
                    }
                }
                let touching = !self.touch_view.fingers.is_empty() || ui.input(|i| i.any_touches());
                if let Some(pt) = resp.interact_pointer_pos().filter(|_| resp.is_pointer_button_down_on() && !touching) {
                    if let Some(i) = plate_of(pt) {
                        want.push((plates[i].1, press_of(pt, plates[i].0), along(pt)));
                        pressed_plate = Some(plate_index[i]);
                    }
                    ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                }
            }
            let keys_down = self.held_keys.values().any(|(_, ids)| ids.contains(&tid));
            let root = key.root as i32;
            let lit: Vec<(u8, f32)> = match slid {
                Some((_, under, _)) => want.iter().map(|w| (under, w.1)).collect(),
                None => want.iter().map(|w| (w.0, w.1)).collect(),
            };
            for (n, &(pr, pitch)) in plates.iter().enumerate() {
                let value = touch::plate_value(&dev.data, plate_index[n]);
                let sounding = held_notes >> pitch & 1 == 1 && slid.is_none();
                let held = lit.iter().any(|w| w.0 == pitch) || self.held_keys.values().any(|(pp, ids)| *pp == pitch && ids.contains(&tid));
                let is_root = (pitch as i32 - root).rem_euclid(12) == 0;
                gradient(&p, pr, Color32::from_rgb(0x2e, 0x2e, 0x34), Color32::from_rgb(0x1c, 0x1c, 0x20));
                // How hard: the plate fills up to it.
                if let Some(&(_, pressure)) = lit.iter().find(|w| w.0 == pitch) {
                    let fill = Rect::from_min_max(pos2(pr.left(), pr.bottom() - pressure * pr.height()), pr.max);
                    gradient(&p, fill, mix(tcol, Color32::WHITE, 0.25), shade(tcol, 0.6));
                } else if held {
                    gradient(&p, pr, shade(tcol, 0.8), shade(tcol, 0.4));
                }
                if sounding {
                    p.rect_filled(pr, 5.0, Color32::from_rgba_unmultiplied(tcol.r(), tcol.g(), tcol.b(), 60));
                    p.rect_stroke(pr, 5.0, Stroke::new(2.0_f32, mix(tcol, Color32::WHITE, 0.35)), egui::StrokeKind::Inside);
                } else {
                    p.rect_stroke(pr, 5.0, Stroke::new(1.0_f32, LINE), egui::StrokeKind::Inside);
                }
                // Its own value: a column up its side (the whole plate while setting them).
                if values_on {
                    let fill = Rect::from_min_max(pos2(pr.left() + 2.0, pr.bottom() - value * pr.height()), pr.max - vec2(2.0, 0.0));
                    p.rect_filled(fill, 3.0, shade(OUT_COLORS[11], 0.55));
                    p.line_segment([pos2(pr.left() + 2.0, fill.top()), pos2(pr.right() - 2.0, fill.top())], Stroke::new(2.0_f32, OUT_COLORS[11]));
                } else {
                    let tick = Rect::from_min_max(pos2(pr.right() - 4.0, pr.bottom() - value * (pr.height() - 10.0) - 4.0), pos2(pr.right() - 2.0, pr.bottom() - 4.0));
                    p.rect_filled(tick, 1.0, shade(OUT_COLORS[11], 0.7));
                }
                // Its edge: the key's root bright.
                let edge = Rect::from_min_size(pr.min + vec2(4.0, 3.0), vec2(pr.width() - 8.0, 3.0));
                p.rect_filled(edge, 1.5, if is_root { accent } else { shade(tcol, 0.55) });
                if pr.height() > 26.0 {
                    p.text(pos2(pr.center().x, pr.bottom() - 4.0), Align2::CENTER_BOTTOM, krabton_engine::note_name(pitch), FontId::proportional(if pw > 34.0 { 10.5 } else { 9.0 }), if sounding || held { TEXT } else { TEXT_DIM });
                }
            }
            // Sliding: a line where the finger is, the note it is at above it.
            if let Some((pt, _, to)) = slid {
                let x = pt.x.clamp(area.left(), area.right());
                p.line_segment([pos2(x, area.top()), pos2(x, area.bottom())], Stroke::new(2.0_f32, mix(tcol, Color32::WHITE, 0.5)));
                let near = to.round();
                let cents = ((to - near) * 100.0).round() as i32;
                let label = if cents == 0 { krabton_engine::note_name(near.clamp(0.0, 127.0) as u8) } else { format!("{} {cents:+}", krabton_engine::note_name(near.clamp(0.0, 127.0) as u8)) };
                let lr = Rect::from_center_size(pos2(x.clamp(area.left() + 30.0, area.right() - 30.0), area.top() + 10.0), vec2(58.0, 18.0));
                p.rect_filled(lr, 4.0, shade(tcol, 0.45));
                p.text(lr.center(), Align2::CENTER_CENTER, label, FontId::proportional(11.0), TEXT);
            }
            (want, pressed_plate, bend_to, keys_down)
        };

        // ---------------------------------------------------------------- what was asked
        let (pressure, position) = match want.iter().fold(None::<(f32, f32)>, |a, w| Some((a.map_or(w.1, |a| a.0.max(w.1)), w.2))) {
            Some(pp) => pp,
            None if keys_down => (self.kb_velocity, self.touch_view.sent.map(|s| s.2).unwrap_or(0.5)),
            None => (0.0, self.touch_view.sent.filter(|s| s.0 == tid).map(|s| s.2).unwrap_or(0.5)),
        };
        // The plates pressed now against before: what is let go stops, what is new starts.
        let before = std::mem::take(&mut self.touch_view.held);
        for &(pitch, t) in &before {
            if t != tid || !want.iter().any(|w| w.0 == pitch) {
                self.touch_note_off(t, pitch);
            }
        }
        let mut now_held: Vec<(u8, u64)> = Vec::with_capacity(want.len());
        for &(pitch, pr, po) in &want {
            if now_held.iter().any(|h| h.0 == pitch) {
                continue;
            }
            if !before.iter().any(|&(bp, t)| bp == pitch && t == tid) {
                self.unbend();
                if let Some(k) = pressed_plate {
                    let value = touch::plate_value(&self.project.tracks[ti].midi_devices[di].data, k);
                    self.cmd(Command::TouchPlate { track: tid, value });
                }
                self.cmd(Command::Touch { track: tid, pressure: pressure.max(pr), position: po });
                self.touch_note_on(ti, pitch, pr);
                if !on_tonnetz {
                    self.touch_record(ti, di, pitch);
                }
            }
            now_held.push((pitch, tid));
        }
        self.touch_view.held = now_held;
        if let Some(to) = bend_to {
            // A little smoothing, so it glides rather than steps.
            let dt = ui.input(|i| i.stable_dt).clamp(0.0, 0.1);
            let now = self.touch_view.bend_now;
            let next = now + (to - now) * (1.0 - (-dt / 0.025).exp());
            self.touch_view.bend_now = if (to - next).abs() < 0.002 { to } else { next };
            if self.touch_view.bend_now != to {
                ui.ctx().request_repaint();
            }
            let b = self.touch_view.bend_now;
            if self.touch_view.bend_sent.is_none_or(|(t, s)| t != tid || (s - b).abs() > 0.003) {
                self.cmd(Command::PitchBend { track: tid, semis: b });
                self.touch_view.bend_sent = Some((tid, b));
            }
        } else if !slide && self.touch_view.bend_sent.is_some() {
            self.unbend();
        }
        let fresh = match self.touch_view.sent {
            Some((t, pr, po)) => t != tid || (pr - pressure).abs() > 0.004 || (po - position).abs() > 0.004,
            None => true,
        };
        if fresh {
            self.cmd(Command::Touch { track: tid, pressure, position });
            self.touch_view.sent = Some((tid, pressure, position));
        }
        let xy_fresh = match self.touch_view.xy_sent {
            Some((t, x0, y0)) => t != tid || (x0 - xy.0).abs() > 0.003 || (y0 - xy.1).abs() > 0.003,
            None => true,
        };
        if xy_fresh {
            self.cmd(Command::TouchXY { track: tid, x: xy.0, y: xy.1 });
            self.touch_view.xy_sent = Some((tid, xy.0, xy.1));
        }
        if let Some((s, param)) = set_slot {
            let routes: Vec<u64> = cables.iter().filter(|c| c.1 == s).map(|c| c.2).collect();
            match param {
                Some(np) => {
                    for r in self.project.mod_routes.iter_mut().filter(|r| routes.contains(&r.id)) {
                        r.target = ModTarget::Param { owner: ModOwner::Instrument(tid), param: np };
                    }
                }
                None => {
                    for rid in routes {
                        self.touch_unpatch(rid);
                    }
                }
            }
            touch::set_patch(&mut self.project.tracks[ti].midi_devices[di].data, s, param);
        }
        for (rid, a) in amounts {
            if let Some(r) = self.project.mod_routes.iter_mut().find(|r| r.id == rid) {
                r.amount = a;
            }
        }
        if let Some(rid) = unpatch {
            self.touch_unpatch(rid);
        }
        if let Some((k, s)) = connect {
            if let Some(param) = touch::patch(&data, s) {
                if !cables.iter().any(|c| c.0 == k && c.1 == s) {
                    self.touch_connect(tid, k, param, 0.5);
                }
            }
        }
        if let Some(i) = pick_preset {
            self.touch_preset(ti, di, i);
        } else if changed {
            self.project.tracks[ti].midi_devices[di].preset.clear();
        }
        if let Some(i) = pick_track {
            self.touch_release_all();
            self.sel_track = TrackSel::Track(i);
        }
    }

    /// A Touch preset on a track's Touch: the way it plays; the plates'
    /// layout and notes, the patch and the shape stay.
    pub(crate) fn touch_preset(&mut self, ti: usize, di: usize, i: usize) {
        let Some(pr) = krabton_engine::presets::midi(MidiFxKind::Touch).get(i) else { return };
        let d = &mut self.project.tracks[ti].midi_devices[di];
        let kept = [touch::LAYOUT, touch::NOTES, touch::BASE, touch::SPRING];
        let keep = kept.map(|k| d.params.get(k).copied().unwrap_or(0.0));
        d.params = krabton_engine::presets::apply_to(touch::TOUCH, pr, &krabton_engine::params::defaults(touch::TOUCH));
        for (k, v) in kept.into_iter().zip(keep) {
            d.params[k] = v;
        }
        if let Some(f) = pr.data {
            let old = d.data.clone();
            d.data = f();
            for s in 0..N_PATCH {
                touch::set_patch(&mut d.data, s, touch::patch(&old, s));
            }
            for j in 0..N_SHAPE {
                touch::set_shape(&mut d.data, j, touch::shape(&old, j));
            }
        }
        d.preset = pr.name.to_string();
    }

    /// Whether the Touch view holds a plate or a pressure.
    pub(crate) fn touch_view_holds(&self) -> bool {
        !self.touch_view.held.is_empty() || self.touch_view.sent.is_some_and(|s| s.1 > 0.0)
    }

    /// Lets go of the plates held (leaving the view, another track).
    pub(crate) fn touch_release_all(&mut self) {
        for (pitch, t) in std::mem::take(&mut self.touch_view.held) {
            self.touch_note_off(t, pitch);
        }
        self.touch_view.fingers.clear();
        self.touch_view.slide_from = None;
        self.unbend();
        if let Some((t, _, po)) = self.touch_view.sent.take() {
            self.cmd(Command::Touch { track: t, pressure: 0.0, position: po });
        }
    }

    /// No track to play: the ones that could be.
    fn touch_pick(&mut self, ui: &mut Ui, r: Rect) {
        let p = ui.painter().clone();
        let c = r.center();
        p.text(pos2(c.x, c.y - 70.0), Align2::CENTER_CENTER, "TOUCH", FontId::proportional(26.0), accent_now());
        p.text(pos2(c.x, c.y - 38.0), Align2::CENTER_CENTER, "Pick a track with an instrument to play with plates, a sequencer and cables", FontId::proportional(13.0), TEXT_DIM);
        let tracks: Vec<(usize, String, Color32)> = self
            .project
            .tracks
            .iter()
            .enumerate()
            .filter(|(_, t)| t.kind == TrackKind::Midi && t.instrument.is_some())
            .map(|(i, t)| (i, t.name.clone(), Color32::from_rgb(t.color[0], t.color[1], t.color[2])))
            .collect();
        if tracks.is_empty() {
            p.text(pos2(c.x, c.y), Align2::CENTER_CENTER, "There is none yet: add an instrument from the browser", FontId::proportional(13.0), TEXT);
            return;
        }
        let w = 220.0;
        let cols = ((r.width() - 40.0) / (w + 10.0)).floor().max(1.0) as usize;
        for (n, (i, name, col)) in tracks.into_iter().enumerate() {
            let x = c.x + ((n % cols) as f32 - (cols as f32 - 1.0) * 0.5) * (w + 10.0);
            let b = Rect::from_center_size(pos2(x, c.y + (n / cols) as f32 * 36.0), vec2(w, 30.0));
            let resp = ui.interact(b, Id::new(("touch_pick", i)), Sense::click());
            p.rect_filled(b, 5.0, if resp.hovered() { PANEL3 } else { PANEL2 });
            p.rect_filled(Rect::from_min_size(b.min, vec2(5.0, b.height())), 2.0, col);
            p.text(pos2(b.left() + 14.0, b.center().y), Align2::LEFT_CENTER, name, FontId::proportional(12.5), TEXT);
            if resp.clicked() {
                self.sel_track = TrackSel::Track(i);
            }
        }
    }
}

/// Touch in the track's MIDI chain (the Devices tab): its steps (drag them)
/// and what it gives out now; it is played in the Touch view.
pub(crate) fn device_editor(ui: &mut Ui, id: Id, params: &[f32], data: &mut Vec<u8>, live: super::seq_view::Live, height: f32) -> bool {
    let w = 300.0;
    let (rect, _) = ui.allocate_exact_size(vec2(w, height), Sense::hover());
    let p = ui.painter().clone();
    rect_fill(&p, rect, 3.0, KNOB);
    let seen = live.vis.map(touch::read_visual);
    let (step, outs) = seen.map(|s| (s.step, s.outs)).unwrap_or((0, [0.0; N_OUT]));
    let n_on = (params.get(touch::STEPS).copied().unwrap_or(8.0).round() as usize).clamp(1, N_STEPS);
    let seq_on = params.get(touch::SEQ).copied().unwrap_or(0.0) >= 0.5;
    let mut changed = false;
    let inner = rect.shrink(6.0);
    let foot = 16.0;
    let steps = Rect::from_min_max(inner.min, pos2(inner.left() + inner.width() * 0.62, inner.bottom() - foot));
    let col_w = steps.width() / N_STEPS as f32;
    let nr_h = steps.height() * 0.62;
    for i in 0..N_STEPS {
        let col = Rect::from_min_size(pos2(steps.left() + i as f32 * col_w, steps.top()), vec2(col_w, steps.height())).shrink2(vec2(1.0, 0.0));
        let active = i < n_on;
        let c = if active && seq_on { OUT_COLORS[2] } else { TEXT_DIM };
        let s = touch::step(data, i);
        let nr = Rect::from_min_size(col.min, vec2(col.width(), nr_h));
        let vr = Rect::from_min_max(pos2(col.left(), nr.bottom() + 3.0), col.max);
        let resp = ui.interact(nr, id.with(("note", i)), Sense::click_and_drag()).on_hover_text("This step's note, in scale steps");
        if let Some(pt) = resp.interact_pointer_pos().filter(|_| resp.dragged() || resp.clicked()) {
            touch::set_step(data, i, Step { note: (((nr.center().y - pt.y) / (nr.height() * 0.5)) * 14.0).round() as i32, ..s });
            changed = true;
        }
        let resp = ui.interact(vr, id.with(("val", i)), Sense::click_and_drag()).on_hover_text("This step's value, for the Steps output");
        if let Some(pt) = resp.interact_pointer_pos().filter(|_| resp.dragged() || resp.clicked()) {
            touch::set_step(data, i, Step { value: ((vr.bottom() - pt.y) / vr.height()).clamp(0.0, 1.0), ..touch::step(data, i) });
            changed = true;
        }
        let s = touch::step(data, i);
        if i == step && active && seq_on {
            p.rect_filled(col.expand(1.0), 2.0, shade(accent_now(), 0.22));
        }
        p.rect_filled(nr, 2.0, SLOT);
        let y = nr.center().y - s.note as f32 / 14.0 * nr.height() * 0.5;
        p.rect_filled(Rect::from_two_pos(pos2(nr.left() + 1.0, nr.center().y), pos2(nr.right() - 1.0, y)), 1.0, shade(if s.rests() || s.skip { TEXT_DIM } else { c }, 0.7));
        p.rect_filled(vr, 2.0, SLOT);
        p.rect_filled(Rect::from_min_max(pos2(vr.left() + 1.0, vr.bottom() - s.value * vr.height()), pos2(vr.right() - 1.0, vr.bottom())), 1.0, shade(c, 0.55));
    }
    let meters = Rect::from_min_max(pos2(steps.right() + 10.0, inner.top()), pos2(inner.right(), inner.bottom() - foot));
    let row = meters.height() / N_OUT as f32;
    for (k, name) in OUTPUTS.iter().enumerate() {
        let y = meters.top() + row * (k as f32 + 0.5);
        p.text(pos2(meters.left(), y), Align2::LEFT_CENTER, *name, FontId::proportional(9.0), TEXT_DIM);
        let m = Rect::from_min_size(pos2(meters.left() + 44.0, y - 1.5), vec2(meters.width() - 44.0, 3.0));
        p.rect_filled(m, 1.5, SLOT);
        p.rect_filled(Rect::from_min_size(m.min, vec2(m.width() * outs[k], m.height())), 1.5, OUT_COLORS[k]);
    }
    p.text(pos2(inner.left(), inner.bottom()), Align2::LEFT_BOTTOM, "Played in the Touch view (top bar): plates, sequencer, cables", FontId::proportional(10.0), TEXT_DIM);
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_patch_points_go_to_the_playable_knobs_first() {
        let picks = touch_picks(krabton_engine::dsp::synth::SPECS);
        assert_eq!(picks.len(), N_PATCH);
        let names: Vec<&str> = picks.iter().map(|&i| krabton_engine::dsp::synth::SPECS[i].name).collect();
        assert!(names[0].to_lowercase().contains("cutoff") || names[0].to_lowercase().contains("filter"), "{names:?}");
        assert!(picks.iter().all(|&i| continuous(&krabton_engine::dsp::synth::SPECS[i])));
        let mut seen = std::collections::HashSet::new();
        assert!(picks.iter().all(|i| seen.insert(*i)), "each knob once");
        assert!(touch_picks(&[]).is_empty());
    }

    #[test]
    fn the_plates_lie_in_the_key_its_pentatonic_or_all_twelve() {
        let a_minor = krabton_engine::Scale::new(9, krabton_engine::ScaleKind::Minor);
        let key: Vec<u8> = (0..8).map(|i| plate_pitch(&a_minor, 57, 0, i)).collect();
        assert_eq!(key, vec![57, 59, 60, 62, 64, 65, 67, 69]);
        let penta: Vec<u8> = (0..6).map(|i| plate_pitch(&a_minor, 57, 1, i)).collect();
        assert_eq!(penta, vec![57, 60, 62, 64, 67, 69], "A minor pentatonic");
        let c_major = krabton_engine::Scale::new(0, krabton_engine::ScaleKind::Major);
        let penta: Vec<u8> = (0..6).map(|i| plate_pitch(&c_major, 60, 1, i)).collect();
        assert_eq!(penta, vec![60, 62, 64, 67, 69, 72], "C major pentatonic");
        assert_eq!(plate_pitch(&c_major, 60, 2, 1), 61);
        assert_eq!((per_octave(&c_major, 60, 0), per_octave(&c_major, 60, 1), per_octave(&c_major, 60, 2)), (7, 5, 12));
    }

    #[test]
    fn sliding_is_in_tune_on_a_plate_and_glides_between() {
        // Three plates in a row, C D E, 40 wide with 10 between: middles at 20, 70, 120.
        let plates: Vec<(Rect, u8)> = [60u8, 62, 64].iter().enumerate().map(|(i, &p)| (Rect::from_min_size(pos2(i as f32 * 50.0, 0.0), vec2(40.0, 100.0)), p)).collect();
        let at = |x: f32, y: f32| slide_pitch(&plates, pos2(x, y)).unwrap();
        assert_eq!((at(20.0, 50.0), at(70.0, 10.0), at(120.0, 90.0)), (60.0, 62.0, 64.0), "the middles");
        assert_eq!((at(28.0, 50.0), at(62.0, 50.0)), (60.0, 62.0), "near the middle: still in tune");
        assert!((at(45.0, 50.0) - 61.0).abs() < 1e-4, "half way: half way");
        assert!(at(40.0, 50.0) > 60.0 && at(40.0, 50.0) < at(50.0, 50.0), "it glides");
        assert_eq!((at(-30.0, 50.0), at(400.0, 50.0)), (60.0, 64.0), "past the ends: the ends");
        // Two rows (a grid): the row the finger is in.
        let mut grid = plates.clone();
        grid.extend([65u8, 67, 69].iter().enumerate().map(|(i, &p)| (Rect::from_min_size(pos2(i as f32 * 50.0, -110.0), vec2(40.0, 100.0)), p)));
        assert_eq!(slide_pitch(&grid, pos2(70.0, -50.0)), Some(67.0));
        assert_eq!(slide_pitch(&grid, pos2(70.0, 50.0)), Some(62.0));
    }

    #[test]
    fn a_click_turns_a_step_one_further_and_round() {
        assert_eq!((turn_count(1, 8, 1), turn_count(7, 8, 1), turn_count(8, 8, 1), turn_count(1, 8, -1)), (2, 8, 1, 8));
        assert_eq!((turn_chance(1.0, 1), turn_chance(0.25, 1), turn_chance(1.0, -1), turn_chance(0.6, 1)), (0.75, 1.0, 0.25, 0.25));
        let s = Step { pulses: 3, ratchet: 1, gate: Gate::Single, ..Step::default() };
        assert_eq!(turn(s, 1, 0, 1).pulses, 4);
        assert_eq!(turn(s, 2, 2, -1).ratchet, MAX_RATCHET);
        assert_eq!(turn(s, 3, 0, 1).gate, Gate::Every);
        assert_eq!(turn(s, 3, 0, -1).gate, Gate::Roll);
        assert!(turn(s, 5, 0, 1).accent && !turn(turn(s, 5, 0, 1), 5, 0, 1).accent);
        // Dragged across: only that lane is copied.
        let from = turn(s, 1, 0, 1);
        let other = Step { note: 5, pulses: 1, accent: true, ..Step::default() };
        let to = copy_lane(other, from, 1, 0);
        assert_eq!((to.pulses, to.note, to.accent), (4, 5, true));
    }

    #[test]
    fn the_ladders_rungs_go_up_inside_their_range() {
        for k in [touch::GATE, touch::SWING, touch::CHANCE, touch::WANDER, touch::PRESS_VEL, touch::DECAY, touch::ARP_ORDER, touch::OCTAVES, touch::STRUM, touch::ECHOES, touch::FEEDBACK] {
            let spec = &touch::TOUCH[k];
            let r = rungs(k, spec);
            assert!((2..=8).contains(&r.len()), "{}: {} rungs", spec.name, r.len());
            assert!(r.windows(2).all(|w| w[0] < w[1]), "{}: {r:?}", spec.name);
            assert!(r.iter().all(|&v| v >= spec.min - 1e-6 && v <= spec.max + 1e-6), "{}: {r:?}", spec.name);
        }
    }

    #[test]
    fn capture_takes_what_was_played_since_the_last_pause() {
        let n = |pitch: u8, start: f64, end: Option<f64>| Played { track: 1, pitch, vel: 0.8, start, end };
        // An old phrase, a long pause, then a bar and a half on the song's clock from beat 17.
        let played = [n(40, 1.0, Some(2.0)), n(60, 17.0, Some(17.5)), n(62, 18.1, Some(19.0)), n(64, 21.0, Some(22.0)), n(67, 22.0, None)];
        let (len, notes) = captured(&played, 22.5, 4.0, true, true).unwrap();
        assert_eq!(len, 8.0, "from bar 5 (beat 16) to past beat 22: two bars");
        let got: Vec<(u8, f64, f64)> = notes.iter().map(|x| (x.pitch, x.start, x.length)).collect();
        assert_eq!(got.len(), 4, "the old phrase is not in it: {got:?}");
        assert_eq!((got[0].0, got[0].1), (60, 1.0));
        assert_eq!(got[1].1, 2.0, "on a sixteenth");
        assert!((got[3].2 - 0.5).abs() < 1e-9, "a held note ends now");
        // Played with the song stopped: from the first note on.
        let (len, notes) = captured(&[n(60, 13.3, Some(14.0)), n(64, 14.0, Some(15.0))], 15.0, 4.0, false, true).unwrap();
        assert_eq!((len, notes[0].start), (4.0, 0.0));
        // Longer than sixteen bars: the last sixteen.
        let long: Vec<Played> = (0..80).map(|k| n(60, k as f64, Some(k as f64 + 0.5))).collect();
        let (len, notes) = captured(&long, 80.0, 4.0, true, false).unwrap();
        assert_eq!(len, 64.0);
        assert_eq!(notes.len(), 64);
        assert!(captured(&[], 0.0, 4.0, true, true).is_none());
    }

    #[test]
    fn a_walk_on_the_tonnetz_moves_to_a_neighbour_each_step() {
        let w = tonnetz_walk(Tri::default(), 16, 3.25);
        assert_eq!((w.len(), w[0]), (16, Tri::default()));
        for p in w.windows(2) {
            let shared = p[1].classes().iter().filter(|c| p[0].classes().contains(c)).count();
            assert_eq!(shared, 2, "{p:?}");
        }
        assert_eq!(chord_name(60, Tri::default()), "C");
        assert_eq!(chord_name(57, Tri::new(0, 0, true)), "C#m", "A's L: C#m");
        let tri = [pos2(0.0, 10.0), pos2(10.0, 10.0), pos2(5.0, 0.0)];
        assert!(inside(pos2(5.0, 6.0), &tri) && !inside(pos2(0.0, 0.0), &tri));
    }

    #[test]
    fn dice_and_mutate_stay_in_range() {
        let mut d = touch::default_data();
        dice(&mut d, 16, 1.25);
        for i in 0..16 {
            let s = touch::step(&d, i);
            assert!((-14..=14).contains(&s.note) && (1..=MAX_RATCHET).contains(&s.ratchet) && (1..=MAX_PULSES).contains(&s.pulses));
        }
        assert!(!touch::step(&d, 0).rests(), "the first step plays");
        let before = d.clone();
        mutate(&mut d, 16, 7.5);
        assert_ne!(d, before, "something changed");
    }
}

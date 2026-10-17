//! Visual editors for the sequencer MIDI devices: step lanes, euclidean
//! rings, the Turing register, the Life board, bouncing balls and the
//! drunk walk. They edit the device's params and pattern data and show the
//! live state the engine reports.

use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, Id, Rect, Sense, Shape, Stroke, Ui, UiBuilder};
use krabton_engine::midi_fx::MidiFxKind;
use krabton_engine::sequencers::{self, LANES, LANE_BYTES, LANE_MAX, LANE_MIN, LANE_NAMES};
use std::f32::consts::TAU;

const LANE_SHORT: [&str; LANES] = ["Trig", "Note", "Oct", "Len", "Prob"];
const RING_COLORS: [Color32; 4] =
    [ACCENT, SELECT, PLAY, Color32::from_rgb(0xe0, 0x84, 0xff)];

/// Live state for an editor: the engine's packed visual, if this device is
/// the one the engine reports on, and whether the transport runs.
#[derive(Clone, Copy)]
pub(crate) struct Live {
    pub vis: Option<[u64; 4]>,
    pub playing: bool,
    /// The project scale (for note and chord names).
    pub scale: krabton_engine::Scale,
    /// The notes of the track's drum kit (0: no kit).
    pub kit: u128,
}

impl Live {
    pub(crate) fn on(&self) -> Option<[u64; 4]> {
        self.vis.filter(|_| self.playing)
    }
}

/// Draws the editor for `kind`. Returns true when params or data changed.
#[allow(clippy::too_many_arguments)]
pub(crate) fn editor(ui: &mut Ui, id: Id, kind: MidiFxKind, params: &mut [f32], data: &mut Vec<u8>, live: Live, lane: &mut usize, accent: Color32, height: f32) -> bool {
    let height = height.max(90.0);
    match kind {
        MidiFxKind::Polymeter => polymeter(ui, id, data, live, lane, accent, height),
        MidiFxKind::Euclid => euclid(ui, id, params, live, height),
        MidiFxKind::Turing => turing(ui, id, params, live, accent, height),
        MidiFxKind::Life => life(ui, id, data, live, accent, height),
        MidiFxKind::Bounce => super::seq_more_view::bounce(ui, id, params, live, height),
        MidiFxKind::Orbits => super::seq_more_view::orbits(ui, id, params, live, height),
        MidiFxKind::Chaos => super::seq_more_view::chaos(ui, id, params, live, height),
        MidiFxKind::Automaton => super::seq_more_view::automaton(ui, id, params, live, accent, height),
        MidiFxKind::Ant => super::seq_more_view::ant(ui, params, live, accent, height),
        MidiFxKind::Pong => super::seq_more_view::pong(ui, id, params, live, height),
        MidiFxKind::Markov => super::seq_more_view::markov(ui, id, params, live, accent, height),
        MidiFxKind::Rain => super::seq_more_view::rain(ui, id, params, live, accent, height),
        MidiFxKind::Words => super::seq_more_view::words(ui, id, params, data, live, accent, height),
        MidiFxKind::Lorenz => super::seq_more_view::lorenz(ui, id, params, live, accent, height),
        MidiFxKind::Loops => super::ambient_view::loops(ui, id, params, live, height),
        MidiFxKind::Bloom => super::ambient_view::bloom(ui, id, params, data, live, accent, height),
        MidiFxKind::Chimes => super::ambient_view::chimes(ui, id, params, live, height),
        MidiFxKind::DriftChords => super::ambient_view::drift_chords(ui, id, params, live, height),
        MidiFxKind::Tintinnabuli => super::ambient_view::tintinnabuli(ui, id, params, live, accent, height),
        MidiFxKind::Pedal => super::ambient_view::pedal(ui, id, params, live, accent, height),
        MidiFxKind::Glass => super::minimal_view::glass(ui, params, live, accent, height),
        MidiFxKind::Satie => super::minimal_view::satie(ui, id, params, live, accent, height),
        MidiFxKind::Riley => super::minimal_view::riley(ui, params, live, accent, height),
        MidiFxKind::Reich => super::composers_view::reich(ui, params, live, accent, height),
        MidiFxKind::Bach => super::composers_view::bach(ui, params, live, accent, height),
        MidiFxKind::Nancarrow => super::composers_view::nancarrow(ui, params, live, accent, height),
        MidiFxKind::Debussy => super::composers_view::debussy(ui, params, live, accent, height),
        MidiFxKind::Feldman => super::composers_view::feldman(ui, live, accent, height),
        MidiFxKind::Mozart => super::masters_view::mozart(ui, params, live, accent, height),
        MidiFxKind::Chopin => super::masters_view::chopin(ui, params, live, accent, height),
        MidiFxKind::Ravel => super::masters_view::ravel(ui, params, live, accent, height),
        MidiFxKind::Bartok => super::masters_view::bartok(ui, params, live, accent, height),
        MidiFxKind::Messiaen => super::masters_view::messiaen(ui, params, live, accent, height),
        MidiFxKind::Ligeti => super::masters_view::ligeti(ui, params, live, accent, height),
        MidiFxKind::Cage => super::masters_view::cage(ui, params, live, accent, height),
        MidiFxKind::Einaudi => super::masters_view::einaudi(ui, params, live, accent, height),
        MidiFxKind::GuitarChords | MidiFxKind::Ukulele => super::chords_view::guitar_chords(ui, id, kind, params, data, live, accent, height),
        MidiFxKind::Drummer => super::drummer_view::drummer(ui, id, params, data, live, accent, height),
        MidiFxKind::DrumMap => super::rhythm_view::drum_map(ui, id, params, live, accent, height),
        MidiFxKind::Marbles => super::rhythm_view::marbles(ui, id, params, live, accent, height),
        MidiFxKind::Chance => super::rhythm_more_view::chance(ui, id, params, data, live, accent, height),
        MidiFxKind::Bits => super::rhythm_more_view::bits(ui, id, params, live, accent, height),
        MidiFxKind::Percussion => super::rhythm_more_view::percussion(ui, id, params, live, accent, height),
        MidiFxKind::Spinner => super::played_view::spinner(ui, id, params, live, accent, height),
        MidiFxKind::Endless => super::played_view::endless(ui, id, params, data, live, accent, height),
        MidiFxKind::Hands => super::played_view::hands(ui, id, params, live, accent, height),
        MidiFxKind::Draw => super::played_view::draw(ui, id, params, data, live, accent, height),
        MidiFxKind::Drunk => drunk(ui, id, params, live, accent, height),
        MidiFxKind::Step => super::classic_view::step(ui, id, params, data, live, lane, accent, height),
        MidiFxKind::Acid => super::classic_view::acid(ui, id, params, data, live, accent, height),
        MidiFxKind::DrumGrid => super::classic_view::drum_grid(ui, id, params, data, live, accent, height),
        MidiFxKind::Progression => super::classic_view::progression(ui, id, params, data, live, accent, height),
        MidiFxKind::PianoPlayer | MidiFxKind::BassPlayer | MidiFxKind::Banjo | MidiFxKind::Section => super::players_view::player(ui, id, kind, params, data, live, accent, height),
        _ => false,
    }
}

/// A small pseudo-random generator for the editors' "random" buttons.
fn rand_u32() -> u32 {
    use std::sync::atomic::{AtomicU32, Ordering};
    static STATE: AtomicU32 = AtomicU32::new(0);
    let mut x = STATE.load(Ordering::Relaxed);
    if x == 0 {
        x = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.subsec_nanos()).unwrap_or(1) | 1;
    }
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    STATE.store(x, Ordering::Relaxed);
    x
}

/// Press-and-drag painting: returns the value chosen on press (the inverse
/// of the cell under the pointer) for as long as the button is held.
fn paint_value(ui: &Ui, resp: &egui::Response, id: Id, current: impl FnOnce() -> bool) -> Option<bool> {
    let pressed = resp.hovered() && ui.input(|i| i.pointer.primary_pressed());
    if pressed {
        let v = !current();
        ui.ctx().memory_mut(|m| m.data.insert_temp(id, v));
    }
    if resp.is_pointer_button_down_on() || pressed {
        ui.ctx().memory(|m| m.data.get_temp::<bool>(id))
    } else {
        None
    }
}

fn small_button(ui: &mut Ui, r: Rect, id: Id, label: &str) -> bool {
    button(ui, r, id, label, false, accent_now()).clicked()
}

// ---------------------------------------------------------------- polymeter

fn polymeter(ui: &mut Ui, id: Id, data: &mut Vec<u8>, live: Live, lane: &mut usize, accent: Color32, height: f32) -> bool {
    let mut changed = false;
    if data.len() < LANES * LANE_BYTES {
        *data = sequencers::polymeter_default();
        changed = true;
    }
    *lane = (*lane).min(LANES - 1);
    let cw = 15.0;
    let w = 16.0 * cw;
    let (rect, _) = ui.allocate_exact_size(vec2(w, height), Sense::hover());
    let p = ui.painter().clone();

    // Lane tabs.
    let tab_w = w / LANES as f32;
    for l in 0..LANES {
        let r = Rect::from_min_size(pos2(rect.left() + l as f32 * tab_w, rect.top()), vec2(tab_w - 2.0, 16.0));
        if button(ui, r, id.with(("lane", l)), LANE_SHORT[l], *lane == l, accent).on_hover_text(LANE_NAMES[l]).clicked() {
            *lane = l;
        }
    }

    let foot_h = 18.0;
    let row_h = 5.0;
    let ov_h = LANES as f32 * row_h;
    let ed = Rect::from_min_max(pos2(rect.left(), rect.top() + 20.0), pos2(rect.right(), rect.bottom() - foot_h - ov_h - 8.0));
    let l = *lane;
    let o = l * LANE_BYTES;
    let (vals, len, div) = sequencers::lane(data, l);
    let pos_of = |v: [u64; 4], l: usize| ((v[0] >> (l * 8)) & 0xff) as usize;
    let pos = live.on().map(|v| pos_of(v, l));
    rect_fill(&p, ed, 3.0, KNOB);
    let (min, max) = (LANE_MIN[l] as f32, LANE_MAX[l] as f32);
    for i in 0..16 {
        let c = Rect::from_min_size(pos2(ed.left() + i as f32 * cw + 1.0, ed.top() + 2.0), vec2(cw - 2.0, ed.height() - 4.0));
        let active = i < len;
        if pos == Some(i) {
            rect_fill(&p, c, 2.0, PANEL3);
        } else if i % 4 == 0 {
            rect_fill(&p, c, 2.0, shade(PANEL2, 0.9));
        }
        let col = if !active {
            shade(accent, 0.22)
        } else if pos == Some(i) {
            mix(accent, Color32::WHITE, 0.45)
        } else {
            accent
        };
        if l == 0 {
            let sq = Rect::from_center_size(c.center(), vec2(cw - 4.0, cw - 4.0));
            if vals[i] == 1 {
                rect_fill(&p, sq, 2.0, col);
            } else {
                rect_line(&p, sq, 2.0, 1.0, shade(col, 0.6));
            }
        } else {
            let t = (vals[i] as f32 - min) / (max - min);
            let bh = 3.0 + t * (c.height() - 3.0);
            let bar = Rect::from_min_max(pos2(c.left() + 1.0, c.bottom() - bh), pos2(c.right() - 1.0, c.bottom()));
            rect_fill(&p, bar, 1.5, if active { shade(col, 0.75) } else { col });
            p.line_segment([pos2(bar.left(), bar.top()), pos2(bar.right(), bar.top())], Stroke::new(2.0_f32, col));
        }
    }
    let resp = ui.interact(ed, id.with("steps"), Sense::click_and_drag());
    if let Some(pt) = resp.interact_pointer_pos().or_else(|| resp.hover_pos()) {
        let i = ((pt.x - ed.left()) / cw).floor();
        if (0.0..16.0).contains(&i) {
            let i = i as usize;
            if l == 0 {
                if let Some(v) = paint_value(ui, &resp, id.with("paint"), || vals[i] == 1) {
                    let nv = v as u8;
                    if data[o + i] != nv {
                        data[o + i] = nv;
                        changed = true;
                    }
                }
            } else if resp.is_pointer_button_down_on() {
                let t = ((ed.bottom() - 2.0 - pt.y) / (ed.height() - 4.0)).clamp(0.0, 1.0);
                let v = (min + (t * (max - min)).round()) as u8;
                if data[o + i] != v {
                    data[o + i] = v;
                    changed = true;
                }
            }
        }
    }
    resp.on_hover_text(match l {
        0 => "Click or drag to set triggers",
        1 => "Scale degree per step (0 = base note)",
        2 => "Octave per step (middle = no shift)",
        3 => "Note length in steps",
        _ => "Probability per step (full = always)",
    });

    // Overview: every lane's loop and playhead, to see the polymeter drift.
    let ov = Rect::from_min_size(pos2(rect.left(), ed.bottom() + 4.0), vec2(w, ov_h));
    for ll in 0..LANES {
        let (_, llen, _) = sequencers::lane(data, ll);
        let lpos = live.on().map(|v| pos_of(v, ll));
        let y = ov.top() + ll as f32 * row_h;
        for i in 0..16 {
            let c = Rect::from_min_size(pos2(ov.left() + i as f32 * cw + 1.0, y + 0.5), vec2(cw - 2.0, row_h - 1.0));
            let fill = if i >= llen {
                PANEL
            } else if lpos == Some(i) {
                Color32::WHITE
            } else if ll == l {
                shade(accent, 0.6)
            } else {
                PANEL3
            };
            rect_fill(&p, c, 1.0, fill);
        }
        let row = Rect::from_min_size(pos2(ov.left(), y), vec2(w, row_h));
        if ui.interact(row, id.with(("ov", ll)), Sense::click()).on_hover_text(LANE_NAMES[ll]).clicked() {
            *lane = ll;
        }
    }

    // Footer: loop length, clock division, randomize.
    let foot = Rect::from_min_size(pos2(rect.left(), rect.bottom() - foot_h), vec2(w, foot_h));
    let mut len_v = len as u8;
    let mut div_v = div as u8;
    // A child ui, so these widgets do not move the parent's cursor.
    let mut fu = ui.new_child(UiBuilder::new().max_rect(foot).layout(egui::Layout::left_to_right(egui::Align::Center)));
    fu.add_sized([70.0, foot_h], egui::DragValue::new(&mut len_v).range(1..=16).prefix("Loop ")).on_hover_text("Loop length of this lane");
    fu.add_sized([56.0, foot_h], egui::DragValue::new(&mut div_v).range(1..=8).prefix("÷")).on_hover_text("Clock division of this lane");
    if len_v as usize != len {
        data[o + 16] = len_v;
        changed = true;
    }
    if div_v as usize != div {
        data[o + 17] = div_v;
        changed = true;
    }
    let r_rand = Rect::from_min_size(pos2(foot.right() - 48.0, foot.top() + 1.0), vec2(48.0, foot_h - 2.0));
    if small_button(ui, r_rand, id.with("rand"), "Random") {
        for i in 0..16 {
            let span = (LANE_MAX[l] - LANE_MIN[l]) as u32 + 1;
            data[o + i] = match l {
                0 => (rand_u32() % 3 == 0) as u8,
                2 => [1, 2, 2, 2, 3][(rand_u32() % 5) as usize],
                3 => [1, 1, 1, 2, 3][(rand_u32() % 5) as usize],
                4 => [2, 3, 4, 4, 4][(rand_u32() % 5) as usize],
                _ => LANE_MIN[l] + (rand_u32() % span) as u8,
            };
        }
        changed = true;
    }
    changed
}

// ---------------------------------------------------------------- euclid

fn euclid(ui: &mut Ui, id: Id, params: &mut [f32], live: Live, height: f32) -> bool {
    let d = height.min(200.0);
    let (rect, resp) = ui.allocate_exact_size(vec2(d, height), Sense::click());
    let p = ui.painter().clone();
    let center = rect.center();
    let outer = d * 0.5 - 6.0;
    let gap = (outer - 14.0) / 4.0;
    let radius = |r: usize| outer - r as f32 * gap;
    let mut changed = false;
    for r in 0..4 {
        let b = 2 + r * 4;
        if params.len() < b + 4 {
            break;
        }
        let steps = params[b].round().max(1.0) as usize;
        let pulses = params[b + 1].round() as usize;
        let rot = params[b + 2].round() as usize;
        let pos = live.on().map(|v| ((v[0] >> (r * 8)) & 0xff) as usize);
        let rad = radius(r);
        let col = RING_COLORS[r];
        p.circle_stroke(center, rad, Stroke::new(1.0_f32, LINE_SOFT));
        let at = |i: usize| {
            let a = -TAU / 4.0 + i as f32 / steps as f32 * TAU;
            pos2(center.x + rad * a.cos(), center.y + rad * a.sin())
        };
        let hits: Vec<usize> = (0..steps).filter(|&i| sequencers::euclid_hit(i, steps, pulses, rot)).collect();
        if hits.len() >= 2 {
            let pts: Vec<_> = hits.iter().map(|&i| at(i)).collect();
            p.add(Shape::closed_line(pts, Stroke::new(1.0_f32, shade(col, 0.45))));
        }
        let dot = (gap * 0.28).clamp(2.0, 5.0);
        for i in 0..steps {
            let hit = hits.contains(&i);
            let now = pos == Some(i);
            let c = at(i);
            if now {
                p.circle_filled(c, dot + 2.0, if hit { Color32::WHITE } else { TEXT_DIM });
            } else if hit {
                p.circle_filled(c, dot, col);
            } else {
                p.circle_filled(c, dot * 0.5, shade(col, 0.35));
            }
        }
    }
    // Which ring is under the pointer?
    let ring_at = |pt: egui::Pos2| -> Option<usize> {
        let dist = (pt - center).length();
        (0..4).min_by(|&a, &b| (radius(a) - dist).abs().total_cmp(&(radius(b) - dist).abs())).filter(|&r| (radius(r) - dist).abs() < gap * 0.6)
    };
    if let Some(r) = resp.hover_pos().and_then(ring_at) {
        let b = 2 + r * 4;
        p.circle_stroke(center, radius(r), Stroke::new(1.5_f32, shade(RING_COLORS[r], 0.7)));
        let steps = params[b].round().max(1.0);
        if resp.clicked() {
            let next = params[b + 1].round() + 1.0;
            params[b + 1] = if next > steps { 0.0 } else { next };
            changed = true;
        }
        if resp.secondary_clicked() {
            params[b + 2] = (params[b + 2].round() + 1.0) % steps;
            changed = true;
        }
        let scroll = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll.abs() > 0.5 {
            let acc_id = id.with("scroll");
            let acc = ui.ctx().memory(|m| m.data.get_temp::<f32>(acc_id).unwrap_or(0.0)) + scroll;
            let stepsz = 30.0;
            let n = (acc / stepsz).trunc();
            ui.ctx().memory_mut(|m| m.data.insert_temp(acc_id, acc - n * stepsz));
            if n != 0.0 {
                params[b] = (params[b] + n).clamp(1.0, 16.0);
                params[b + 1] = params[b + 1].min(params[b]);
                changed = true;
            }
        }
    }
    resp.on_hover_text("Click a ring: one more pulse · right-click: rotate · scroll: steps");
    changed
}

// ---------------------------------------------------------------- turing

fn rotl(reg: u32, k: u32, len: u32) -> u32 {
    let mask = (1u32 << len) - 1;
    let k = k % len;
    ((reg << k) | (reg >> (len - k))) & mask
}

fn turing(ui: &mut Ui, id: Id, params: &mut [f32], live: Live, accent: Color32, height: f32) -> bool {
    let w = 240.0;
    let (rect, _) = ui.allocate_exact_size(vec2(w, height), Sense::hover());
    let p = ui.painter().clone();
    let len = params[1].round().clamp(2.0, 16.0) as u32;
    let reg = live.vis.map(|v| (v[0] & 0xffff) as u32).unwrap_or(0b1011_0010_0110_1001) & ((1 << len) - 1);
    let mut changed = false;

    // The register: one LED per bit, newest on the right.
    let cw = w / 16.0;
    for i in 0..16u32 {
        let c = pos2(rect.left() + (i as f32 + 0.5) * cw, rect.top() + 8.0);
        if i < len {
            let bit = (reg >> (len - 1 - i)) & 1 == 1;
            p.circle_filled(c, 5.0, if bit { accent } else { PANEL3 });
        } else {
            p.circle_stroke(c, 3.0, Stroke::new(1.0_f32, LINE_SOFT));
        }
    }

    // The melody the loop will play if nothing changes (rotations of the register).
    let bars = Rect::from_min_max(pos2(rect.left(), rect.top() + 20.0), pos2(rect.right(), rect.bottom() - 22.0));
    rect_fill(&p, bars, 3.0, KNOB);
    let mask = (1u32 << len) - 1;
    for k in 0..len {
        let r = rotl(reg, k, len);
        let value = (r & 0xff) as f32 / mask.min(0xff) as f32;
        let gate = ((r.wrapping_mul(0x9E37) >> 5) & 0xff) as f32 / 255.0;
        let plays = gate < params[3];
        let bw = bars.width() / len as f32;
        let x = bars.left() + k as f32 * bw;
        let bh = 3.0 + value * (bars.height() - 8.0);
        let bar = Rect::from_min_max(pos2(x + 1.5, bars.bottom() - 2.0 - bh), pos2(x + bw - 1.5, bars.bottom() - 2.0));
        let col = match (k == 0 && live.playing && live.vis.is_some(), plays) {
            (true, _) => Color32::WHITE,
            (false, true) => accent,
            (false, false) => shade(accent, 0.25),
        };
        rect_fill(&p, bar, 1.5, col);
    }
    ui.interact(bars, id.with("bars"), Sense::hover()).on_hover_text("The loop ahead: bright bars play, dim ones are skipped by Density");

    // Lock / slip, like the big knob on the hardware.
    let change = params[2];
    let locked = change <= 0.001;
    let r_lock = Rect::from_min_size(pos2(rect.left(), rect.bottom() - 18.0), vec2(56.0, 18.0));
    if button(ui, r_lock, id.with("lock"), if locked { "Locked" } else { "Lock" }, locked, accent).on_hover_text("Freeze the loop (Change = 0)").clicked() {
        params[2] = if locked { 0.1 } else { 0.0 };
        changed = true;
    }
    let r_slip = Rect::from_min_size(pos2(rect.left() + 60.0, rect.bottom() - 18.0), vec2(56.0, 18.0));
    if button(ui, r_slip, id.with("slip"), "Mutate", change >= 0.5, accent).on_hover_text("Let it change a lot (Change = 50%)").clicked() {
        params[2] = if change >= 0.5 { 0.1 } else { 0.5 };
        changed = true;
    }
    text(&p, pos2(rect.right(), rect.bottom() - 9.0), Align2::RIGHT_CENTER, format!("{len} steps · {:.0}% change", change * 100.0), small(), TEXT_DIM);
    changed
}

// ---------------------------------------------------------------- life

fn life(ui: &mut Ui, id: Id, data: &mut Vec<u8>, live: Live, accent: Color32, height: f32) -> bool {
    let mut changed = false;
    if data.len() < 16 {
        data.resize(16, 0);
        changed = true;
    }
    let cs = ((height - 26.0) / 8.0).clamp(8.0, 16.0);
    let w = cs * 16.0;
    let (rect, _) = ui.allocate_exact_size(vec2(w.max(200.0), height), Sense::hover());
    let p = ui.painter().clone();
    let board = Rect::from_min_size(rect.min, vec2(w, cs * 8.0));
    rect_fill(&p, board, 3.0, KNOB);
    let seed = sequencers::life_rows(data);
    let cells = live.on().map(|v| sequencers::life_from_visual(&v));
    let col = live.on().map(|v| (v[2] & 0xff) as usize);
    for y in 0..8 {
        for x in 0..16 {
            let c = Rect::from_min_size(pos2(board.left() + x as f32 * cs, board.top() + y as f32 * cs), vec2(cs, cs)).shrink(1.0);
            let s = (seed[y] >> x) & 1 == 1;
            let alive = cells.map(|c| (c[y] >> x) & 1 == 1).unwrap_or(s);
            if col == Some(x) {
                rect_fill(&p, c, 2.0, PANEL3);
            }
            if alive {
                let fill = if col == Some(x) { Color32::WHITE } else { accent };
                rect_fill(&p, c, 2.0, fill);
            }
            if s && cells.is_some() {
                rect_line(&p, c, 2.0, 1.0, shade(accent, 0.7));
            }
        }
    }
    let resp = ui.interact(board, id.with("board"), Sense::click_and_drag());
    if let Some(pt) = resp.interact_pointer_pos().or_else(|| resp.hover_pos()) {
        let x = ((pt.x - board.left()) / cs).floor();
        let y = ((pt.y - board.top()) / cs).floor();
        if (0.0..16.0).contains(&x) && (0.0..8.0).contains(&y) {
            let (x, y) = (x as usize, y as usize);
            if let Some(v) = paint_value(ui, &resp, id.with("paint"), || (seed[y] >> x) & 1 == 1) {
                let i = y * 2 + x / 8;
                let bit = 1u8 << (x % 8);
                let nv = if v { data[i] | bit } else { data[i] & !bit };
                if nv != data[i] {
                    data[i] = nv;
                    changed = true;
                }
            }
        }
    }
    resp.on_hover_text("Draw the seed. Rows are scale steps (top = high); the board plays column by column and evolves.");

    let foot = Rect::from_min_size(pos2(rect.left(), rect.bottom() - 18.0), vec2(rect.width(), 18.0));
    let bw = 48.0;
    let mut x = foot.left();
    let set = |rows: [u16; 8], data: &mut Vec<u8>| {
        *data = rows.iter().flat_map(|r| r.to_le_bytes()).collect();
    };
    if small_button(ui, Rect::from_min_size(pos2(x, foot.top()), vec2(bw, 18.0)), id.with("clear"), "Clear") {
        set([0; 8], data);
        changed = true;
    }
    x += bw + 4.0;
    if small_button(ui, Rect::from_min_size(pos2(x, foot.top()), vec2(bw, 18.0)), id.with("rand"), "Random") {
        let rows: [u16; 8] = std::array::from_fn(|_| (rand_u32() & rand_u32()) as u16);
        set(rows, data);
        changed = true;
    }
    x += bw + 4.0;
    if small_button(ui, Rect::from_min_size(pos2(x, foot.top()), vec2(bw, 18.0)), id.with("glider"), "Glider") {
        let mut rows = seed;
        let gx = (rand_u32() % 13) as u16;
        let gy = (rand_u32() % 6) as usize;
        rows[gy] |= 0b010 << gx;
        rows[gy + 1] |= 0b100 << gx;
        rows[gy + 2] |= 0b111 << gx;
        set(rows, data);
        changed = true;
    }
    if let Some(v) = live.on() {
        text(&p, pos2(foot.right(), foot.center().y), Align2::RIGHT_CENTER, format!("gen {}", v[2] >> 8), small(), TEXT_DIM);
    }
    changed
}

// ---------------------------------------------------------------- drunk

fn drunk(ui: &mut Ui, id: Id, params: &mut [f32], live: Live, accent: Color32, height: f32) -> bool {
    let w = 220.0;
    let (rect, _) = ui.allocate_exact_size(vec2(w, height), Sense::hover());
    let p = ui.painter().clone();
    rect_fill(&p, rect, 3.0, KNOB);
    let range = params[2].round().max(2.0) as usize;
    let pos = live.on().map(|v| (v[0] & 0xff) as usize);
    let ladder = Rect::from_min_size(rect.min + vec2(6.0, 6.0), vec2(22.0, rect.height() - 12.0));
    let y_of = |k: f32| ladder.bottom() - k / range as f32 * ladder.height();
    for k in 0..=range {
        let y = y_of(k as f32);
        let on = pos == Some(k);
        p.line_segment([pos2(ladder.left(), y), pos2(ladder.right(), y)], Stroke::new(if on { 3.0_f32 } else { 1.0 }, if on { accent } else { LINE }));
    }
    // A scrolling trail of the walk.
    let hist_id = id.with("trail");
    let mut hist: Vec<u8> = ui.ctx().memory(|m| m.data.get_temp(hist_id).unwrap_or_default());
    if let Some(k) = pos {
        hist.push(k as u8);
        if hist.len() > 180 {
            hist.remove(0);
        }
        ui.ctx().memory_mut(|m| m.data.insert_temp(hist_id, hist.clone()));
    }
    let plot = Rect::from_min_max(pos2(ladder.right() + 8.0, ladder.top()), pos2(rect.right() - 6.0, ladder.bottom()));
    if hist.len() >= 2 {
        let dx = plot.width() / 179.0;
        let x0 = plot.right() - (hist.len() - 1) as f32 * dx;
        let pts: Vec<_> = hist.iter().enumerate().map(|(i, &k)| pos2(x0 + i as f32 * dx, y_of(k.min(range as u8) as f32))).collect();
        p.add(Shape::line(pts, Stroke::new(1.5_f32, accent)));
    }
    if let Some(k) = pos {
        p.circle_filled(pos2(plot.right(), y_of(k as f32)), 4.0, Color32::WHITE);
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Event, Modifiers, PointerButton, Pos2, RawInput};
    use krabton_engine::params::defaults;

    /// Runs the editor for a few frames, feeding one list of input events
    /// per frame. Positions are relative to the editor's top-left corner.
    fn drive(kind: MidiFxKind, params: &mut Vec<f32>, data: &mut Vec<u8>, lane: &mut usize, frames: &[Vec<(Pos2, Option<bool>)>]) {
        let ctx = egui::Context::default();
        let mut origin = Pos2::ZERO;
        for (i, evs) in std::iter::once(&Vec::new()).chain(frames.iter()).enumerate() {
            let mut input = RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1200.0, 800.0))), ..Default::default() };
            for &(p, press) in evs {
                let pos = origin + p.to_vec2();
                input.events.push(Event::PointerMoved(pos));
                if let Some(pressed) = press {
                    input.events.push(Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE });
                }
            }
            let _ = ctx.run(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    if i == 0 {
                        origin = ui.cursor().min;
                    }
                    let live = Live { vis: None, playing: false, scale: krabton_engine::Scale::default(), kit: 0 };
                    editor(ui, Id::new("t"), kind, params, data, live, lane, accent_now(), 190.0);
                });
            });
        }
    }

    #[test]
    fn life_board_paints_by_dragging() {
        let mut p = defaults(sequencers::LIFE);
        let mut data = vec![0u8; 16];
        let cell = |x: f32, y: f32| pos2((x + 0.5) * 16.0, (y + 0.5) * 16.0);
        let frames = vec![vec![(cell(2.0, 3.0), Some(true))], vec![(cell(5.0, 3.0), None)], vec![(cell(5.0, 3.0), Some(false))]];
        drive(MidiFxKind::Life, &mut p, &mut data, &mut 0, &frames);
        let rows = sequencers::life_rows(&data);
        assert_eq!(rows[3], (1 << 2) | (1 << 5), "{rows:?}");
        // Pressing a live cell erases.
        let frames = vec![vec![(cell(2.0, 3.0), Some(true))], vec![(cell(2.0, 3.0), Some(false))]];
        drive(MidiFxKind::Life, &mut p, &mut data, &mut 0, &frames);
        assert_eq!(sequencers::life_rows(&data)[3], 1 << 5);
    }

    #[test]
    fn polymeter_steps_toggle_and_values_drag() {
        let mut p = defaults(sequencers::POLYMETER);
        let mut data = sequencers::polymeter_default();
        let step = |i: f32, y: f32| pos2((i + 0.5) * 15.0, y);
        assert_eq!(data[1], 0);
        let frames = vec![vec![(step(1.0, 80.0), Some(true))], vec![(step(1.0, 80.0), Some(false))]];
        drive(MidiFxKind::Polymeter, &mut p, &mut data, &mut 0, &frames);
        assert_eq!(data[1], 1, "trigger toggled on");
        // Note lane: drag to the top sets the highest degree.
        let mut lane = 1;
        let frames = vec![vec![(step(0.0, 23.0), Some(true))], vec![(step(0.0, 23.0), Some(false))]];
        drive(MidiFxKind::Polymeter, &mut p, &mut data, &mut lane, &frames);
        assert_eq!(data[LANE_BYTES], LANE_MAX[1]);
    }

    #[test]
    fn bounce_columns_set_drop_and_note() {
        let mut p = defaults(sequencers::BOUNCE);
        let mut data = Vec::new();
        // Column 2 (ball 2): click low in the column for a short drop.
        let frames = vec![vec![(pos2(52.5, 115.0), Some(true))], vec![(pos2(52.5, 115.0), Some(false))]];
        drive(MidiFxKind::Bounce, &mut p, &mut data, &mut 0, &frames);
        assert_eq!(p[17], 1.0);
        // A dimmed column adds balls up to it.
        let frames = vec![vec![(pos2(6.0 + 6.0 * 31.0 + 15.0, 60.0), Some(true))], vec![(pos2(6.0 + 6.0 * 31.0 + 15.0, 60.0), Some(false))]];
        drive(MidiFxKind::Bounce, &mut p, &mut data, &mut 0, &frames);
        assert_eq!(p[0], 7.0);
        // Ball 1's note: drag up.
        let frames = vec![
            vec![(pos2(21.5, 177.0), Some(true))],
            vec![(pos2(21.5, 160.0), None)],
            vec![(pos2(21.5, 147.0), None)],
            vec![(pos2(21.5, 147.0), Some(false))],
        ];
        drive(MidiFxKind::Bounce, &mut p, &mut data, &mut 0, &frames);
        assert!(p[8] >= 3.0, "note {}", p[8]);
    }

    #[test]
    fn bounce_dice_rolls_new_notes() {
        let mut p = defaults(sequencers::BOUNCE);
        let mut data = Vec::new();
        let before: Vec<f32> = p[8..16].to_vec();
        // The "notes" button sits at the top right, left of "drops".
        let at = pos2(6.0 + 8.0 * 31.0 + 6.0 - 2.0 * 46.0 - 8.0 + 20.0, 8.0);
        let frames = vec![vec![(at, Some(true))], vec![(at, Some(false))]];
        drive(MidiFxKind::Bounce, &mut p, &mut data, &mut 0, &frames);
        assert_ne!(p[8..16].to_vec(), before, "notes rolled");
        assert!(p[8..16].iter().all(|n| (-5.0..=14.0).contains(n) && n.fract() == 0.0));
    }

    #[test]
    fn automaton_rule_pictures_flip_bits() {
        let mut p = defaults(sequencers::more::AUTOMATON);
        let mut data = Vec::new();
        assert_eq!(p[1], 90.0);
        let frames = vec![vec![(pos2(14.0, 11.0), Some(true))], vec![(pos2(14.0, 11.0), Some(false))]];
        drive(MidiFxKind::Automaton, &mut p, &mut data, &mut 0, &frames);
        assert_eq!(p[1], 218.0);
    }

    #[test]
    fn every_sequencer_editor_draws() {
        for kind in MidiFxKind::SEQUENCERS {
            let mut p = defaults(kind.specs());
            let mut data = kind.default_data();
            drive(kind, &mut p, &mut data, &mut 0, &[vec![(pos2(40.0, 40.0), None)]]);
        }
    }

    #[test]
    fn satie_chords_click_into_your_own() {
        use krabton_engine::sequencers::minimal as mn;
        let mut p = defaults(mn::SATIE);
        let mut data = Vec::new();
        // The first chord (Sway: IV, I): click it on to V.
        let at = pos2(40.0, 44.0);
        drive(MidiFxKind::Satie, &mut p, &mut data, &mut 0, &[vec![(at, Some(true))], vec![(at, Some(false))]]);
        assert_eq!(p[mn::S_HARMONY], mn::SATIE_CUSTOM as f32);
        assert_eq!(p[mn::S_CHORD], 5.0);
        assert_eq!(p[mn::S_CHORD + 1], 1.0);
        assert_eq!(p[mn::S_CHORD + 2], 0.0);
        // The form: click for the next.
        let at = pos2(20.0, 190.0 - 12.0);
        drive(MidiFxKind::Satie, &mut p, &mut data, &mut 0, &[vec![(at, Some(true))], vec![(at, Some(false))]]);
        assert_eq!(p[mn::S_FORM], 1.0);
    }

    #[test]
    fn euclid_ring_click_adds_a_pulse() {
        let mut p = defaults(sequencers::EUCLID);
        let mut data = Vec::new();
        assert_eq!(p[3], 4.0);
        let top_of_outer_ring = pos2(95.0, 95.0 - 89.0);
        let frames = vec![vec![(top_of_outer_ring, Some(true))], vec![(top_of_outer_ring, Some(false))]];
        drive(MidiFxKind::Euclid, &mut p, &mut data, &mut 0, &frames);
        assert_eq!(p[3], 5.0);
    }
}

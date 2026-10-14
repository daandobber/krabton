//! Big Picture's Tape page: the four tracks of the tape running past the
//! playhead in the middle, two reels turning at the tape's speed, and the
//! tape tricks on the controller. Up and down choose a track, A arms it to
//! record (what the played track plays, or the input), left and right move a
//! bar, B winds back to the start (of the loop), X plays backwards, Y loops
//! the bar. The right stick scrubs the tape
//! by hand, the left bends its speed, LT brakes it to a stop and RT stutters
//! the bit just played (the harder, the shorter). Menu goes to the row of
//! modes: arm, mute and level of the track, speed, reverse, loop, lift, drop,
//! erase, undo and where the track records from. A finger drags the tape.

use super::input::{Frame, Nav};
use super::{fit_text, tile, Glyph, Sz, TILE, TILE_HI};
use crate::app::App;
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, Id, Painter, Rect, Sense, Stroke, Ui};
use krabton_engine::engine::Command;
use krabton_engine::tape::{TapeSource, MAX_SPEED, MIN_SPEED, OVERVIEW, TAPE_TRACKS};

/// The colours of the four tracks.
const LANES: [Color32; TAPE_TRACKS] = [Color32::from_rgb(0x4f, 0x9d, 0xf7), Color32::from_rgb(0x58, 0xd0, 0x8a), Color32::from_rgb(0xd8, 0xd8, 0xde), Color32::from_rgb(0xff, 0x9a, 0x3c)];

/// How long a stutter repeats, by how far RT is pulled (beats).
const STUTTERS: [f64; 4] = [1.0, 0.5, 0.25, 0.125];

#[derive(Default)]
pub struct TapeState {
    pub lane: usize,
    /// The row of modes has the focus, on this item.
    pub bar_focus: Option<usize>,
    /// The reels' turn (radians).
    angle: f32,
    /// The tape tricks held now (sent to the engine when they change).
    brake: bool,
    stutter: Option<usize>,
    scrub: Option<f32>,
    bend: f32,
    /// Where a finger took the tape (its x, the tape's frame then).
    drag: Option<(f32, f64)>,
}

#[derive(Clone, Copy, PartialEq)]
enum TapeAct {
    Arm,
    Mute,
    Level,
    Speed,
    Reverse,
    Loop,
    Lift,
    Drop,
    Erase,
    Undo,
    Source,
}

struct Item {
    label: String,
    lit: bool,
    act: TapeAct,
    help: &'static str,
    group: u8,
    stepper: bool,
}

/// Semitones from a speed ratio, and back.
fn semis(speed: f32) -> f32 {
    12.0 * speed.max(1e-3).log2()
}

fn ratio(semis: f32) -> f32 {
    (semis / 12.0).exp2().clamp(MIN_SPEED, MAX_SPEED)
}

impl App {
    /// The tape on the desktop: the same view, worked with the mouse and
    /// the arrow keys.
    pub(crate) fn ui_tape(&mut self, ui: &mut Ui) {
        let r = ui.max_rect().shrink(10.0);
        let s = Sz::of(r);
        let inp = Frame { dt: ui.input(|i| i.stable_dt).min(0.1), ..Default::default() };
        self.big_tape(ui, r, &s, inp);
    }

    /// Moves the tape `by` bars (to a bar line).
    pub(crate) fn tape_step_bar(&mut self, by: f64) {
        let st = self.status();
        let bar_secs = self.project.beats_per_bar_f() * 60.0 / self.project.bpm.max(1.0);
        let bar = bar_secs * st.reel.rate() as f64;
        let at = ((st.reel.pos.load() / bar).round() + by).max(0.0) * bar_secs;
        self.cmd(Command::TapeLocate(at));
    }

    /// Lets go of the tape tricks (leaving the page).
    pub(crate) fn tape_tricks_off(&mut self) {
        let t = &mut self.big.tape;
        let cmds = [
            std::mem::take(&mut t.brake).then_some(Command::TapeBrake(false)),
            t.stutter.take().map(|_| Command::TapeStutter(None)),
            t.scrub.take().map(|_| Command::TapeScrub(None)),
            (t.bend != 0.0 && t.bend != 1.0).then_some(Command::TapeBend(1.0)),
        ];
        t.bend = 1.0;
        t.drag = None;
        for c in cmds.into_iter().flatten() {
            self.cmd(c);
        }
    }

    fn tape_items(&self) -> Vec<Item> {
        let tape = &self.project.tape;
        let lane = self.big.tape.lane;
        let track = &tape.tracks[lane];
        let item = |label: String, lit: bool, act: TapeAct, help: &'static str, group: u8, stepper: bool| Item { label, lit, act, help, group, stepper };
        let source = match track.source {
            TapeSource::Played => "Played".to_string(),
            TapeSource::Input => "Input".to_string(),
            TapeSource::Track(id) => self.project.tracks.iter().find(|t| t.id == id).map(|t| t.name.clone()).unwrap_or_else(|| "?".into()),
        };
        vec![
            item("Rec".into(), tape.armed == Some(lane), TapeAct::Arm, "Arm this track: it records while the transport runs (A)", 0, false),
            item("Mute".into(), track.mute, TapeAct::Mute, "Silence this track", 0, false),
            item(format!("{:+.1} dB", track.volume_db), false, TapeAct::Level, "This track's level (up and down)", 0, true),
            item(format!("{:+.0} st", semis(tape.speed)), false, TapeAct::Speed, "The tape's speed, in semitones: a quarter to twice as fast, the pitch going with it", 1, true),
            item("Reverse".into(), tape.reverse, TapeAct::Reverse, "Play the tape backwards (X)", 1, false),
            item("Loop".into(), tape.loop_on, TapeAct::Loop, "Loop the bar the tape is in (Y)", 1, false),
            item("Lift".into(), false, TapeAct::Lift, "Take the bar (or the loop) out of this track, to drop elsewhere", 2, false),
            item("Drop".into(), false, TapeAct::Drop, "Put what was lifted at the start of this bar (or loop)", 2, false),
            item("Erase".into(), false, TapeAct::Erase, "Silence the bar (or the loop) on this track", 2, false),
            item("Undo".into(), false, TapeAct::Undo, "Undo the last take or edit; again: redo it", 2, false),
            item(format!("From: {source}"), false, TapeAct::Source, "What this track records: the track being played, the audio input, or a track", 3, true),
        ]
    }

    fn tape_act(&mut self, act: TapeAct, step: i32) {
        let lane = self.big.tape.lane;
        match act {
            TapeAct::Arm if step == 0 => self.tape_arm(lane),
            TapeAct::Mute if step == 0 => self.project.tape.tracks[lane].mute ^= true,
            TapeAct::Level => {
                let v = &mut self.project.tape.tracks[lane].volume_db;
                *v = if step == 0 { 0.0 } else { (*v + 1.5 * step as f32).clamp(-36.0, 6.0) };
            }
            TapeAct::Speed => {
                let s = semis(self.project.tape.speed).round();
                self.project.tape.speed = if step == 0 { 1.0 } else { ratio(s + step as f32) };
            }
            TapeAct::Reverse if step == 0 => self.project.tape.reverse ^= true,
            TapeAct::Loop if step == 0 => self.tape_toggle_loop(),
            TapeAct::Lift if step == 0 => self.tape_lift(lane),
            TapeAct::Drop if step == 0 => self.tape_drop(lane),
            TapeAct::Erase if step == 0 => self.tape_erase(lane),
            TapeAct::Undo if step == 0 => self.tape_undo(),
            TapeAct::Source => {
                let mut all = vec![TapeSource::Played, TapeSource::Input];
                all.extend(self.project.tracks.iter().map(|t| TapeSource::Track(t.id)));
                let src = &mut self.project.tape.tracks[lane].source;
                let i = all.iter().position(|s| s == src).unwrap_or(0) as i32;
                *src = all[(i + if step == 0 { 1 } else { step }).rem_euclid(all.len() as i32) as usize];
            }
            _ => {}
        }
    }

    /// Loops the bar the tape is in (or stops looping).
    fn tape_toggle_loop(&mut self) {
        let tape = &mut self.project.tape;
        if tape.loop_on {
            tape.loop_on = false;
            return;
        }
        let sr = self.status().reel.rate() as f64;
        let bar = self.project.beats_per_bar_f() * 60.0 / self.project.bpm.max(1.0);
        let at = self.status().reel.pos.load() / sr;
        let a = (at / bar).floor() * bar;
        let tape = &mut self.project.tape;
        tape.loop_in = a;
        tape.loop_out = a + bar;
        tape.loop_on = true;
    }

    pub(crate) fn big_tape(&mut self, ui: &mut Ui, r: Rect, s: &Sz, mut inp: Frame) -> Vec<(Glyph, &'static str)> {
        let st = self.status();
        let reel = &st.reel;
        let sr = reel.rate() as f64;
        let bar_secs = self.project.beats_per_bar_f() * 60.0 / self.project.bpm.max(1.0);
        let bar = bar_secs * sr;
        let items = self.tape_items();
        let mut act: Option<(TapeAct, i32)> = None;
        // The row of modes takes the D-pad, A and B when it has the focus.
        let menu_nav = |n: &Nav| n.is_direction() || matches!(n, Nav::A | Nav::B);
        if let Some(mut f) = self.big.tape.bar_focus {
            f = f.min(items.len() - 1);
            for nav in inp.pressed.iter().copied().filter(menu_nav) {
                let it = &items[f];
                match nav {
                    Nav::Left => f = (f + items.len() - 1) % items.len(),
                    Nav::Right => f = (f + 1) % items.len(),
                    Nav::Up if it.stepper => act = Some((it.act, 1)),
                    Nav::Down if it.stepper => act = Some((it.act, -1)),
                    Nav::A => act = Some((it.act, 0)),
                    Nav::B => {
                        self.big.tape.bar_focus = None;
                        break;
                    }
                    _ => {}
                }
            }
            inp.pressed.retain(|n| !menu_nav(n));
            if self.big.tape.bar_focus.is_some() {
                self.big.tape.bar_focus = Some(f);
            }
        }
        for nav in std::mem::take(&mut inp.pressed) {
            match nav {
                Nav::Up => self.big.tape.lane = self.big.tape.lane.saturating_sub(1),
                Nav::Down => self.big.tape.lane = (self.big.tape.lane + 1).min(TAPE_TRACKS - 1),
                Nav::Left => self.tape_step_bar(-1.0),
                Nav::Right => self.tape_step_bar(1.0),
                Nav::A => act = Some((TapeAct::Arm, 0)),
                Nav::B => {
                    let t = &self.project.tape;
                    self.cmd(Command::TapeLocate(if t.loop_on { t.loop_in } else { 0.0 }));
                }
                Nav::X => act = Some((TapeAct::Reverse, 0)),
                Nav::Y => act = Some((TapeAct::Loop, 0)),
                _ => {}
            }
        }
        // The tricks: scrub (right stick), bend (left stick), brake (LT), stutter (RT).
        let (sx, _) = inp.stick;
        let scrub = (sx.abs() > 0.2).then(|| sx * 3.0);
        if scrub.is_some() != self.big.tape.scrub.is_some() || scrub.zip(self.big.tape.scrub).is_some_and(|(a, b)| (a - b).abs() > 0.05) {
            self.big.tape.scrub = scrub;
            self.cmd(Command::TapeScrub(scrub));
        }
        let (lx, _) = inp.lstick;
        let bend = if lx.abs() > 0.15 { lx.exp2() } else { 1.0 };
        let last_bend = if self.big.tape.bend == 0.0 { 1.0 } else { self.big.tape.bend };
        if (bend - last_bend).abs() > 0.01 {
            self.big.tape.bend = bend;
            self.cmd(Command::TapeBend(bend));
        }
        let brake = if self.big.tape.brake { inp.lt > 0.4 } else { inp.lt > 0.5 };
        if brake != self.big.tape.brake {
            self.big.tape.brake = brake;
            self.cmd(Command::TapeBrake(brake));
        }
        let stutter = if inp.rt > 0.25 { Some((((inp.rt - 0.25) / 0.75 * STUTTERS.len() as f32) as usize).min(STUTTERS.len() - 1)) } else { None };
        if stutter != self.big.tape.stutter {
            self.big.tape.stutter = stutter;
            self.cmd(Command::TapeStutter(stutter.map(|k| STUTTERS[k])));
        }

        let p = ui.painter().clone();
        // The head: where the tape is, how fast, what it does; the reels.
        let head_h = s.u * 1.6;
        let head = Rect::from_min_size(r.min, vec2(r.width(), head_h));
        let pos = reel.pos.load();
        let secs = pos / sr;
        let beats = secs * self.project.bpm / 60.0;
        let bpb = self.project.beats_per_bar_f();
        let recording = reel.recording.load(std::sync::atomic::Ordering::Relaxed);
        let time = format!("{}:{:04.1}", (secs / 60.0).floor() as i64, secs % 60.0);
        text(&p, pos2(head.left(), head.center().y - s.u * 0.1), Align2::LEFT_CENTER, &time, egui::FontId::monospace(s.font * 1.7), if recording { REC } else { TEXT });
        let bar_txt = format!("bar {}.{}", (beats / bpb).floor() as i64 + 1, (beats % bpb).floor() as i64 + 1);
        text(&p, pos2(head.left() + s.u * 5.6, head.center().y), Align2::LEFT_CENTER, &bar_txt, s.f(), TEXT_DIM);
        let speed = reel.speed.load();
        let tape = &self.project.tape;
        let mut flags = vec![format!("×{:.2}", speed.abs())];
        if tape.reverse {
            flags.push("REV".into());
        }
        if tape.loop_on {
            flags.push("LOOP".into());
        }
        if recording {
            flags.push("REC".into());
        }
        fit_text(&p, pos2(head.left() + s.u * 9.6, head.center().y), Align2::LEFT_CENTER, &flags.join("  "), s.font, s.u * 7.0, if recording { REC } else { accent_now() });
        // Two reels, turning with the tape.
        self.big.tape.angle += speed * inp.dt * 3.0;
        let rr = head_h * 0.42;
        for (k, cx) in [head.right() - rr * 3.6, head.right() - rr * 1.2].into_iter().enumerate() {
            let c = pos2(cx, head.center().y);
            p.circle_stroke(c, rr, Stroke::new(2.0_f32, TEXT_DIM));
            p.circle_filled(c, rr * 0.25, TEXT_DIM);
            for spoke in 0..3 {
                let a = self.big.tape.angle * if k == 0 { 1.0 } else { 1.3 } + spoke as f32 * std::f32::consts::TAU / 3.0;
                p.line_segment([c + vec2(a.cos(), a.sin()) * rr * 0.3, c + vec2(a.cos(), a.sin()) * rr * 0.85], Stroke::new(2.0_f32, TEXT_DIM));
            }
        }
        // The row of modes.
        let bar_r = Rect::from_min_size(pos2(r.left(), head.bottom() + s.u * 0.1), vec2(r.width(), s.u * 1.15));
        if let Some(a) = self.tape_modes(ui, bar_r, s, &items) {
            act = Some(a);
        }
        // The lanes: the tape runs past the playhead in the middle.
        let area = Rect::from_min_max(pos2(r.left(), bar_r.bottom() + s.u * 0.3), r.max);
        let gap = s.u * 0.15;
        let lane_h = (area.height() - gap * (TAPE_TRACKS - 1) as f32) / TAPE_TRACKS as f32;
        let label_w = s.u * 2.4;
        let wave_l = area.left() + label_w + s.u * 0.2;
        let wave_w = area.right() - wave_l;
        let span = 4.0 * bar;
        let x_of = |f: f64| wave_l + wave_w * 0.5 + ((f - pos) / span) as f32 * wave_w;
        let lanes_r = Rect::from_min_max(pos2(wave_l, area.top()), area.max);
        let drag = ui.interact(lanes_r, Id::new("big_tape_drag"), Sense::click_and_drag());
        if drag.drag_started() {
            self.big.tape.drag = drag.interact_pointer_pos().map(|pt| (pt.x, pos));
        }
        if let (Some((x0, f0)), Some(pt)) = (self.big.tape.drag, drag.interact_pointer_pos()) {
            if drag.dragged() {
                let to = (f0 - ((pt.x - x0) / wave_w) as f64 * span).max(0.0);
                self.cmd(Command::TapeLocate(to / sr));
            }
        }
        if drag.drag_stopped() {
            self.big.tape.drag = None;
        }
        if drag.clicked() {
            if let Some(pt) = drag.interact_pointer_pos() {
                self.big.tape.lane = (((pt.y - area.top()) / (lane_h + gap)) as usize).min(TAPE_TRACKS - 1);
            }
        }
        let tape = &self.project.tape;
        for k in 0..TAPE_TRACKS {
            let lr = Rect::from_min_size(pos2(area.left(), area.top() + k as f32 * (lane_h + gap)), vec2(area.width(), lane_h));
            let on = k == self.big.tape.lane;
            let color = LANES[k];
            // The label: its number, armed, muted, its level.
            let lab = Rect::from_min_size(lr.min, vec2(label_w, lane_h));
            let lresp = ui.interact(lab, Id::new(("big_tape_lane", k)), Sense::click());
            tile(&p, lab, if on { shade(color, 0.45) } else { TILE }, on && self.big.tape.bar_focus.is_none());
            text(&p, pos2(lab.left() + s.u * 0.45, lab.center().y), Align2::LEFT_CENTER, (k + 1).to_string(), s.big(), if on { TEXT } else { color });
            let armed = tape.armed == Some(k);
            if armed {
                p.circle_filled(pos2(lab.right() - s.u * 0.45, lab.top() + s.u * 0.45), s.u * 0.2, REC);
            }
            if tape.tracks[k].mute {
                text(&p, pos2(lab.right() - s.u * 0.45, lab.bottom() - s.u * 0.4), Align2::CENTER_CENTER, "M", s.f(), TEXT_DIM);
            }
            let level = reel.levels[k].swap(0.0);
            let meter = Rect::from_min_max(pos2(lab.right() - s.u * 0.18, lab.bottom() - (lab.height() - 8.0) * level.min(1.0) - 4.0), pos2(lab.right() - s.u * 0.08, lab.bottom() - 4.0));
            rect_fill(&p, meter, 1.0, color);
            if lresp.clicked() {
                self.big.tape.lane = k;
            }
            // The tape.
            let wr = Rect::from_min_max(pos2(wave_l, lr.top()), lr.max);
            rect_fill(&p, wr, 6.0, if on { PANEL2 } else { PANEL });
            if tape.loop_on && tape.loop_out > tape.loop_in {
                let (a, b) = (x_of(tape.loop_in * sr).max(wr.left()), x_of(tape.loop_out * sr).min(wr.right()));
                if b > a {
                    p.rect_filled(Rect::from_min_max(pos2(a, wr.top()), pos2(b, wr.bottom())), 0.0, shade(accent_now(), 0.22));
                }
            }
            draw_bars(&p, wr, pos, span, bar, bar / bpb);
            if let Some(sp) = reel.spool(k) {
                draw_wave(&p, wr, pos, span, sp, if armed && recording { mix(color, REC, 0.5) } else { color });
            }
        }
        // The playhead.
        let px = wave_l + wave_w * 0.5;
        p.line_segment([pos2(px, area.top() - 2.0), pos2(px, area.bottom() + 2.0)], Stroke::new(2.0_f32, if recording { REC } else { TEXT }));
        if let Some((a, step)) = act {
            self.tape_act(a, step);
        }
        if self.big.tape.bar_focus.is_some() {
            vec![(Glyph::Dpad, "Choose, change"), (Glyph::Btn(Nav::A), "Set"), (Glyph::Btn(Nav::B), "Back to the tape")]
        } else {
            vec![(Glyph::Dpad, "Track, bar"), (Glyph::Btn(Nav::A), "Rec"), (Glyph::Btn(Nav::B), "To start"), (Glyph::Btn(Nav::X), "Reverse"), (Glyph::Btn(Nav::Y), "Loop"), (Glyph::Sticks, "Bend, scrub"), (Glyph::Triggers, "Brake, stutter")]
        }
    }

    /// The row of modes under the head; returns a button pressed.
    fn tape_modes(&mut self, ui: &mut Ui, bar: Rect, s: &Sz, items: &[Item]) -> Option<(TapeAct, i32)> {
        let p = ui.painter().clone();
        let accent = LANES[self.big.tape.lane];
        let font = s.f();
        let pad_x = s.u * 0.7;
        let (in_gap, group_gap) = (s.u * 0.08, s.u * 0.35);
        let widths: Vec<f32> = items.iter().map(|it| p.layout_no_wrap(it.label.clone(), font.clone(), TEXT).size().x + pad_x + if it.stepper { s.u * 0.6 } else { 0.0 }).collect();
        let groups = items.windows(2).filter(|w| w[0].group != w[1].group).count();
        let gaps = (items.len() - 1 - groups) as f32 * in_gap + groups as f32 * group_gap;
        let k = ((bar.width() - gaps) / widths.iter().sum::<f32>()).min(1.0);
        let h = bar.height() - s.u * 0.2;
        let pointer = ui.input(|i| i.pointer.interact_pos());
        let mut x = bar.left();
        let mut act = None;
        for (i, it) in items.iter().enumerate() {
            if i > 0 {
                x += if items[i - 1].group != it.group { group_gap } else { in_gap };
            }
            let w = widths[i] * k;
            let br = Rect::from_min_size(pos2(x, bar.top()), vec2(w, h));
            let resp = ui.interact(br, Id::new(("big_tape_mode", i)), Sense::click());
            let lit = it.lit;
            let fill = if lit && it.act == TapeAct::Arm { REC } else if lit { shade(accent, 0.8) } else if resp.hovered() { TILE_HI } else { TILE };
            tile(&p, br, fill, self.big.tape.bar_focus == Some(i));
            fit_text(&p, br.center(), Align2::CENTER_CENTER, &it.label, s.font, w - if it.stepper { s.u * 1.0 } else { 6.0 }, if lit { TEXT_DARK } else { TEXT });
            if it.stepper {
                text(&p, pos2(br.left() + s.u * 0.25, br.center().y), Align2::CENTER_CENTER, "‹", s.f(), TEXT_DIM);
                text(&p, pos2(br.right() - s.u * 0.25, br.center().y), Align2::CENTER_CENTER, "›", s.f(), TEXT_DIM);
            }
            if resp.on_hover_text(it.help).clicked() {
                let step = if !it.stepper {
                    0
                } else if pointer.is_some_and(|pt| pt.x < br.center().x) {
                    -1
                } else {
                    1
                };
                act = Some((it.act, step));
            }
            x += w;
        }
        act
    }
}

/// Bar lines (and beat lines when there is room) across a lane.
fn draw_bars(p: &Painter, r: Rect, pos: f64, span: f64, bar: f64, beat: f64) {
    let x_of = |f: f64| r.center().x + ((f - pos) / span) as f32 * r.width();
    let first = ((pos - span * 0.5) / beat).floor().max(0.0) as i64;
    let last = ((pos + span * 0.5) / beat).ceil() as i64;
    let beat_px = (beat / span) as f32 * r.width();
    for b in first..=last {
        let f = b as f64 * beat;
        let x = x_of(f);
        if x < r.left() || x > r.right() {
            continue;
        }
        let on_bar = (f / bar).fract().abs() < 1e-6 || (f / bar).fract() > 1.0 - 1e-6;
        if on_bar {
            p.line_segment([pos2(x, r.top()), pos2(x, r.bottom())], Stroke::new(1.0_f32, LINE));
        } else if beat_px > 12.0 {
            p.line_segment([pos2(x, r.top() + r.height() * 0.4), pos2(x, r.bottom() - r.height() * 0.4)], Stroke::new(1.0_f32, LINE.gamma_multiply(0.75)));
        }
    }
}

/// A track's sound across a lane, from its overview.
fn draw_wave(p: &Painter, r: Rect, pos: f64, span: f64, sp: &krabton_engine::tape::Spool, color: Color32) {
    let cols = (r.width() / 2.0) as usize;
    let mid = r.center().y;
    let half = r.height() * 0.45;
    for c in 0..cols {
        let f0 = pos - span * 0.5 + span * c as f64 / cols as f64;
        let f1 = f0 + span / cols as f64;
        if f1 < 0.0 {
            continue;
        }
        let (k0, k1) = ((f0.max(0.0) as usize) / OVERVIEW, ((f1.max(0.0) as usize) / OVERVIEW).max((f0.max(0.0) as usize) / OVERVIEW + 1));
        let peak = (k0..k1.min(sp.overview_len())).map(|k| sp.peak(k)).fold(0.0f32, f32::max);
        if peak > 0.002 {
            let x = r.left() + c as f32 * 2.0 + 1.0;
            let h = (peak.sqrt() * half).max(1.0);
            p.line_segment([pos2(x, mid - h), pos2(x, mid + h)], Stroke::new(1.6_f32, color));
        }
    }
}

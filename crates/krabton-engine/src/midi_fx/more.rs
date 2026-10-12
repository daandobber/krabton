//! More MIDI effects: Strum (chords played as strums), Note Repeat (held
//! notes roll in time), Humanize (timing, velocity and swing), Note Filter
//! (chance, key and velocity ranges, every Nth note), Mono (one note at a
//! time, with priority and legato) and Quantize (notes moved onto the grid).

use super::{MidiCtx, MidiEvent, MidiFxKind, Scheduler};
use crate::dsp::Rng;
use crate::params::{ParamSpec, Unit};

const DIRECTIONS: &[&str] = &["Up", "Down", "Alternate", "Random"];
pub const RATES: &[&str] = &["1/32", "1/16 T", "1/16", "1/8 T", "1/8", "1/4"];
const RATE_BEATS: [f64; 6] = [0.125, 1.0 / 6.0, 0.25, 1.0 / 3.0, 0.5, 1.0];

pub static STRUM: &[ParamSpec] = &[
    ParamSpec::log("Time", "Strum", 1.0, 250.0, 30.0, Unit::Ms),
    ParamSpec::choice("Direction", "Strum", DIRECTIONS, 0),
    ParamSpec::lin("Velocity", "Strum", -1.0, 1.0, -0.3, Unit::Percent),
    ParamSpec::lin("Humanize", "Strum", 0.0, 1.0, 0.1, Unit::Percent),
];
pub static NOTE_REPEAT: &[ParamSpec] = &[
    ParamSpec::choice("Rate", "Repeat", RATES, 2),
    ParamSpec::lin("Gate", "Repeat", 0.05, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Decay", "Repeat", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::toggle("On Grid", "Repeat", true),
];
pub static HUMANIZE: &[ParamSpec] = &[
    ParamSpec::lin("Timing", "Humanize", 0.0, 40.0, 8.0, Unit::Ms),
    ParamSpec::lin("Velocity", "Humanize", 0.0, 1.0, 0.15, Unit::Percent),
    ParamSpec::lin("Swing", "Swing", 0.0, 0.75, 0.0, Unit::Percent),
    ParamSpec::choice("Swing On", "Swing", &["1/8", "1/16"], 1),
];
pub static NOTE_FILTER: &[ParamSpec] = &[
    ParamSpec::lin("Chance", "Filter", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::int("Every", "Filter", 1.0, 8.0, 1.0, Unit::None),
    ParamSpec::int("Low Key", "Keys", 0.0, 127.0, 0.0, Unit::None),
    ParamSpec::int("High Key", "Keys", 0.0, 127.0, 127.0, Unit::None),
    ParamSpec::lin("Min Velocity", "Velocity", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Max Velocity", "Velocity", 0.0, 1.0, 1.0, Unit::Percent),
];
pub static MONO: &[ParamSpec] = &[ParamSpec::choice("Priority", "Mono", &["Last", "Low", "High"], 0), ParamSpec::toggle("Legato", "Mono", false)];
pub static QUANTIZE: &[ParamSpec] = &[ParamSpec::choice("Grid", "Quantize", RATES, 2), ParamSpec::lin("Amount", "Quantize", 0.0, 1.0, 1.0, Unit::Percent)];

pub struct MoreState {
    sched: Scheduler,
    rng: Rng,
    /// Per input pitch: how many frames its notes were moved (the note-off follows).
    delay: [u64; 128],
    /// Note Filter: whether the held note got through.
    passed: [bool; 128],
    /// Held notes in the order played.
    held: Vec<(u8, f32)>,
    /// Note Repeat: the next repeat of each held pitch and how many came before.
    next: [u64; 128],
    count: [u32; 128],
    /// Mono: the note that sounds.
    sounding: Option<u8>,
    strum_down: bool,
    counter: u32,
}

impl MoreState {
    pub fn new() -> Self {
        Self {
            sched: Scheduler::new(),
            rng: Rng::new(0x5_7E0),
            delay: [0; 128],
            passed: [false; 128],
            held: Vec::with_capacity(32),
            next: [0; 128],
            count: [0; 128],
            sounding: None,
            strum_down: false,
            counter: 0,
        }
    }

    pub fn reset(&mut self) {
        self.sched.events.clear();
        self.held.clear();
        self.passed = [false; 128];
        self.delay = [0; 128];
        self.sounding = None;
        self.counter = 0;
    }

    /// The first frame on a grid of `step` beats at or after frame `at`.
    fn grid_after(ctx: &MidiCtx, at: u64, step: f64) -> u64 {
        let spb = ctx.samples_per_beat.max(1.0);
        let beat = ctx.beat + (at as f64 - ctx.now as f64) / spb;
        let g = (beat / step - 1e-9).ceil() * step;
        (ctx.now as f64 + (g - ctx.beat) * spb).round().max(at as f64) as u64
    }

    pub fn process(&mut self, kind: MidiFxKind, p: &[f32], input: &[MidiEvent], out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        let spb = ctx.samples_per_beat.max(1.0);
        match kind {
            MidiFxKind::Strum => self.strum(p, input, ctx),
            MidiFxKind::NoteRepeat => self.repeat(p, input, ctx),
            MidiFxKind::Humanize => {
                let frames_per_ms = ctx.frames_per_second() / 1000.0;
                let (timing, vel_amt, swing) = (p[0].clamp(0.0, 100.0), p[1].clamp(0.0, 1.0), p[2].clamp(0.0, 0.9) as f64);
                let grid = if p[3] >= 0.5 { 0.25 } else { 0.5 };
                for e in input {
                    let at = ctx.now + e.offset as u64;
                    if e.on {
                        let mut d = (self.rng.unipolar() * timing * frames_per_ms) as u64;
                        // Swing: notes on an off-beat of the grid come later.
                        let beat = ctx.beat + e.offset as f64 / spb;
                        let k = (beat / grid).round();
                        if (k as i64).rem_euclid(2) == 1 && (beat - k * grid).abs() < 0.12 * grid {
                            d += (swing * grid * spb) as u64;
                        }
                        self.delay[e.pitch as usize & 127] = d;
                        let vel = (e.vel + self.rng.bipolar() * vel_amt * 0.5).clamp(0.02, 1.0);
                        self.sched.push(at + d, MidiEvent::on(0, e.pitch, vel));
                    } else {
                        self.sched.push(at + self.delay[e.pitch as usize & 127], MidiEvent::off(0, e.pitch));
                    }
                }
            }
            MidiFxKind::NoteFilter => {
                let every = p[1].round().clamp(1.0, 8.0) as u32;
                let (lo, hi) = (p[2].min(p[3]).round() as u8, p[2].max(p[3]).round() as u8);
                let (vlo, vhi) = (p[4].min(p[5]), p[4].max(p[5]));
                for e in input {
                    let i = e.pitch as usize & 127;
                    if e.on {
                        let in_range = (lo..=hi).contains(&e.pitch) && e.vel >= vlo - 1e-4 && e.vel <= vhi + 1e-4;
                        let mut pass = in_range;
                        if in_range {
                            pass = self.counter % every == 0 && self.rng.unipolar() < p[0].clamp(0.0, 1.0) + 1e-6;
                            self.counter = self.counter.wrapping_add(1);
                        }
                        if pass {
                            out.push(*e);
                        }
                        self.passed[i] = pass;
                    } else if std::mem::take(&mut self.passed[i]) {
                        out.push(*e);
                    }
                }
            }
            MidiFxKind::Mono => {
                let legato = p[1] >= 0.5;
                for e in input {
                    self.held.retain(|h| h.0 != e.pitch);
                    if e.on && self.held.len() < self.held.capacity() {
                        self.held.push((e.pitch, e.vel));
                    }
                    let chosen = match p[0].round() as i32 {
                        1 => self.held.iter().min_by_key(|h| h.0).copied(),
                        2 => self.held.iter().max_by_key(|h| h.0).copied(),
                        _ => self.held.last().copied(),
                    };
                    let now = self.sounding;
                    match chosen {
                        None => {
                            if let Some(s) = now {
                                out.push(MidiEvent::off(e.offset, s));
                            }
                            self.sounding = None;
                        }
                        Some((c, v)) if Some(c) != now => {
                            // Legato: the new note starts before the old one ends.
                            let vel = if e.on && e.pitch == c { e.vel } else { v };
                            if legato {
                                out.push(MidiEvent::on(e.offset, c, vel));
                            }
                            if let Some(s) = now {
                                out.push(MidiEvent::off(e.offset, s));
                            }
                            if !legato {
                                out.push(MidiEvent::on(e.offset, c, vel));
                            }
                            self.sounding = Some(c);
                        }
                        _ => {}
                    }
                }
            }
            MidiFxKind::Quantize => {
                let step = RATE_BEATS[(p[0].round().max(0.0) as usize).min(RATE_BEATS.len() - 1)];
                let amount = p[1].clamp(0.0, 1.0) as f64;
                for e in input {
                    let at = ctx.now + e.offset as u64;
                    let i = e.pitch as usize & 127;
                    if e.on {
                        // Only later is possible: a note just after a line stays.
                        let beat = ctx.beat + e.offset as f64 / spb;
                        let since = beat - (beat / step).floor() * step;
                        let d = if since < 0.2 * step { 0 } else { ((Self::grid_after(ctx, at, step) - at) as f64 * amount) as u64 };
                        self.delay[i] = d;
                        self.sched.push(at + d, *e);
                    } else {
                        self.sched.push(at + self.delay[i], *e);
                    }
                }
            }
            _ => out.extend_from_slice(input),
        }
        self.sched.emit_due(ctx, out);
        out.sort_by_key(|e| e.offset);
    }

    fn strum(&mut self, p: &[f32], input: &[MidiEvent], ctx: &MidiCtx) {
        let frames_per_ms = ctx.frames_per_second() / 1000.0;
        let time = p[0].clamp(0.0, 1000.0) * frames_per_ms;
        let dir = p[1].round() as i32;
        let tilt = p[2].clamp(-1.0, 1.0);
        let human = p[3].clamp(0.0, 1.0);
        let mut i = 0;
        while i < input.len() {
            let e = input[i];
            if !e.on {
                self.sched.push(ctx.now + e.offset as u64 + self.delay[e.pitch as usize & 127], e);
                i += 1;
                continue;
            }
            // The chord: every note-on at this offset.
            let mut chord: Vec<MidiEvent> = Vec::new();
            let mut j = i;
            while j < input.len() && input[j].offset == e.offset {
                if input[j].on && chord.len() < 16 {
                    chord.push(input[j]);
                } else if !input[j].on {
                    self.sched.push(ctx.now + input[j].offset as u64 + self.delay[input[j].pitch as usize & 127], input[j]);
                }
                j += 1;
            }
            chord.sort_by_key(|n| n.pitch);
            let down = match dir {
                1 => true,
                2 => {
                    self.strum_down = !self.strum_down;
                    !self.strum_down
                }
                3 => self.rng.unipolar() < 0.5,
                _ => false,
            };
            if down {
                chord.reverse();
            }
            if dir == 3 {
                // Random order.
                for k in (1..chord.len()).rev() {
                    let r = (self.rng.unipolar() * (k + 1) as f32) as usize;
                    chord.swap(k, r.min(k));
                }
            }
            let n = chord.len().max(2) - 1;
            for (k, note) in chord.iter().enumerate() {
                let jitter = self.rng.unipolar() * human * time * 0.5;
                let d = (k as f32 * time + jitter) as u64;
                self.delay[note.pitch as usize & 127] = d;
                let vel = (note.vel * (1.0 + tilt * k as f32 / n as f32)).clamp(0.02, 1.0);
                self.sched.push(ctx.now + note.offset as u64 + d, MidiEvent::on(0, note.pitch, vel));
            }
            i = j;
        }
    }

    fn repeat(&mut self, p: &[f32], input: &[MidiEvent], ctx: &MidiCtx) {
        let step_beats = RATE_BEATS[(p[0].round().max(0.0) as usize).min(RATE_BEATS.len() - 1)];
        let step = (step_beats * ctx.samples_per_beat).max(16.0) as u64;
        let gate = ((p[1].clamp(0.05, 1.0) as f64 * step as f64) as u64).clamp(8, step);
        let decay = p[2].clamp(0.0, 1.0);
        let on_grid = p[3] >= 0.5;
        for e in input {
            let i = e.pitch as usize & 127;
            let at = ctx.now + e.offset as u64;
            self.held.retain(|h| h.0 != e.pitch);
            if e.on {
                if self.held.len() < self.held.capacity() {
                    self.held.push((e.pitch, e.vel));
                }
                // The first hit plays at once; the repeats follow (on the grid).
                self.sched.push(at, MidiEvent::on(0, e.pitch, e.vel));
                self.sched.push(at + gate, MidiEvent::off(0, e.pitch));
                self.next[i] = if on_grid { Self::grid_after(ctx, at + 1, step_beats) } else { at + step };
                self.count[i] = 0;
            }
            // Note-offs need nothing: every hit already has its own end.
        }
        let end = ctx.now + ctx.len as u64;
        for &(pitch, vel) in &self.held {
            let i = pitch as usize;
            while self.next[i] < end {
                self.count[i] += 1;
                let v = (vel * (1.0 - decay * 0.5).powi(self.count[i] as i32)).max(0.05);
                self.sched.push(self.next[i], MidiEvent::on(0, pitch, v));
                self.sched.push(self.next[i] + gate, MidiEvent::off(0, pitch));
                self.next[i] += step;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::MidiFxState;
    use super::*;
    use crate::params::defaults;
    use crate::scale::{Scale, ScaleKind};

    /// Runs a device over blocks of 256 frames with the song clock running
    /// (24000 frames a beat); `input` holds (absolute frame, event).
    fn run(kind: MidiFxKind, p: &[f32], input: &[(u64, MidiEvent)], blocks: u64) -> Vec<(u64, MidiEvent)> {
        let mut st = MidiFxState::new(kind);
        let mut all = Vec::new();
        for b in 0..blocks {
            let now = b * 256;
            let evs: Vec<MidiEvent> = input.iter().filter(|(t, _)| *t >= now && *t < now + 256).map(|(t, e)| MidiEvent { offset: (t - now) as u32, ..*e }).collect();
            let ctx = MidiCtx { now, len: 256, samples_per_beat: 24000.0, scale: Scale::new(0, ScaleKind::Major), scale_on: true, beat: now as f64 / 24000.0, playing: true, sr: 48000.0, drum_notes: 0 };
            let mut out = Vec::new();
            st.process(kind, p, &[], &evs, &mut out, &ctx);
            all.extend(out.into_iter().map(|e| (now + e.offset as u64, e)));
        }
        all
    }

    fn chord(at: u64, off: u64) -> Vec<(u64, MidiEvent)> {
        let mut v: Vec<(u64, MidiEvent)> = [60, 64, 67].iter().map(|&p| (at, MidiEvent::on(0, p, 0.9))).collect();
        v.extend([60, 64, 67].iter().map(|&p| (off, MidiEvent::off(0, p))));
        v
    }

    #[test]
    fn strum_spreads_a_chord_upwards() {
        let mut p = defaults(MidiFxKind::Strum.specs());
        p[3] = 0.0;
        let out = run(MidiFxKind::Strum, &p, &chord(0, 48000), 200);
        let ons: Vec<(u64, u8)> = out.iter().filter(|e| e.1.on).map(|e| (e.0, e.1.pitch)).collect();
        assert_eq!(ons.iter().map(|o| o.1).collect::<Vec<_>>(), vec![60, 64, 67]);
        assert!(ons[1].0 > ons[0].0 + 1000 && ons[2].0 > ons[1].0 + 1000, "{ons:?}");
        assert_eq!(out.iter().filter(|e| !e.1.on).count(), 3);
        // Every note ends after it starts.
        for (t, e) in out.iter().filter(|e| !e.1.on) {
            let start = ons.iter().find(|o| o.1 == e.pitch).unwrap().0;
            assert!(*t > start);
        }
    }

    #[test]
    fn note_repeat_rolls_held_notes_in_time() {
        let p = defaults(MidiFxKind::NoteRepeat.specs()); // 1/16 = 6000 frames
        let input = [(0, MidiEvent::on(0, 36, 1.0)), (24000, MidiEvent::off(0, 36))];
        let out = run(MidiFxKind::NoteRepeat, &p, &input, 200);
        let ons: Vec<u64> = out.iter().filter(|e| e.1.on).map(|e| e.0).collect();
        assert_eq!(ons, vec![0, 6000, 12000, 18000], "{ons:?}");
        assert_eq!(out.iter().filter(|e| !e.1.on).count(), 4);
    }

    #[test]
    fn humanize_swings_the_off_beats() {
        let mut p = defaults(MidiFxKind::Humanize.specs());
        p[0] = 0.0;
        p[1] = 0.0;
        p[2] = 0.5;
        let input = [(0, MidiEvent::on(0, 60, 0.8)), (3000, MidiEvent::off(0, 60)), (6000, MidiEvent::on(0, 62, 0.8)), (9000, MidiEvent::off(0, 62))];
        let out = run(MidiFxKind::Humanize, &p, &input, 100);
        let ons: Vec<u64> = out.iter().filter(|e| e.1.on).map(|e| e.0).collect();
        // The second 16th moves half a 16th later.
        assert_eq!(ons, vec![0, 9000]);
        assert_eq!(out.iter().find(|e| !e.1.on && e.1.pitch == 62).unwrap().0, 12000);
    }

    #[test]
    fn note_filter_keeps_every_other_note_in_range() {
        let mut p = defaults(MidiFxKind::NoteFilter.specs());
        p[1] = 2.0;
        p[3] = 70.0;
        let mut input = Vec::new();
        for k in 0..8u64 {
            input.push((k * 1000, MidiEvent::on(0, 60, 0.8)));
            input.push((k * 1000 + 500, MidiEvent::off(0, 60)));
        }
        input.push((9000, MidiEvent::on(0, 80, 0.8)));
        let out = run(MidiFxKind::NoteFilter, &p, &input, 50);
        assert_eq!(out.iter().filter(|e| e.1.on).count(), 4);
        assert_eq!(out.iter().filter(|e| !e.1.on).count(), 4);
        assert!(out.iter().all(|e| e.1.pitch == 60));
    }

    #[test]
    fn mono_plays_one_note_and_goes_back() {
        let p = defaults(MidiFxKind::Mono.specs());
        let input = [(0, MidiEvent::on(0, 60, 0.8)), (1000, MidiEvent::on(0, 64, 0.8)), (2000, MidiEvent::off(0, 64)), (3000, MidiEvent::off(0, 60))];
        let out = run(MidiFxKind::Mono, &p, &input, 20);
        let seq: Vec<(bool, u8)> = out.iter().map(|e| (e.1.on, e.1.pitch)).collect();
        assert_eq!(seq, vec![(true, 60), (false, 60), (true, 64), (false, 64), (true, 60), (false, 60)]);
    }

    #[test]
    fn quantize_moves_late_notes_to_the_next_line() {
        let p = defaults(MidiFxKind::Quantize.specs()); // 1/16 = 6000 frames
        let input = [(4000, MidiEvent::on(0, 60, 0.8)), (5000, MidiEvent::off(0, 60)), (6200, MidiEvent::on(0, 62, 0.8))];
        let out = run(MidiFxKind::Quantize, &p, &input, 50);
        let ons: Vec<(u64, u8)> = out.iter().filter(|e| e.1.on).map(|e| (e.0, e.1.pitch)).collect();
        assert_eq!(ons, vec![(6000, 60), (6200, 62)]);
        assert_eq!(out.iter().find(|e| !e.1.on).unwrap().0, 7000);
    }
}

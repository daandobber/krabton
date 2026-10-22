//! Touch: a track played as on a performance synth's touch plates, made to
//! play with. The plates (on the Touch view, or the computer keys) give the
//! notes and how hard they are pressed; a pulser plays the held plate again in
//! time (also with the transport stopped), five steps move the notes through
//! the key and give a value of their own, chance leaves pulses out or lets a
//! note wander, and Latch keeps the last plate going. What it makes is patched
//! to the instrument's knobs through Touch modulators (`ModKind::Touch`): the
//! pressure, the pulse, the steps, a stepped random, a drifting random and
//! where along the plates the hand is.
//!
//! Its data: five steps (a note in scale steps, -7..7, and a value 0..255),
//! then six patch points (the instrument's parameters the Touch view shows,
//! u16 each, little-endian, 0xFFFF: none).

use super::SeqState;
use crate::midi_fx::{MidiCtx, MidiEvent};
use crate::params::{ParamSpec, Unit};

pub const PULSE: usize = 0;
pub const RATE: usize = 1;
pub const DECAY: usize = 2;
pub const LATCH: usize = 3;
pub const STEPS: usize = 4;
pub const ORDER: usize = 5;
pub const MOVE: usize = 6;
pub const CHANCE: usize = 7;
pub const WANDER: usize = 8;
pub const GATE: usize = 9;
pub const BASE: usize = 10;
pub const PRESS_VEL: usize = 11;

pub const RATES: &[&str] = &["1/32", "1/16 T", "1/16", "1/8 T", "1/8", "1/4 T", "1/4", "1/2", "1 Bar"];
const RATE_BEATS: [f64; 9] = [0.125, 1.0 / 6.0, 0.25, 1.0 / 3.0, 0.5, 2.0 / 3.0, 1.0, 2.0, 4.0];
pub const ORDERS: &[&str] = &["Forward", "Pendulum", "Random"];
/// What the Touch modulators can take from it.
pub const OUTPUTS: &[&str] = &["Pressure", "Pulse", "Steps", "Random", "Drift", "Position"];
pub const N_STEPS: usize = 5;
pub const N_PATCH: usize = 6;
/// Plates held at once, notes sounding at once.
const MAX_HELD: usize = 16;

pub static TOUCH: &[ParamSpec] = &[
    ParamSpec::toggle("Pulse", "Pulser", false),
    ParamSpec::choice("Rate", "Pulser", RATES, 4),
    ParamSpec::log("Decay", "Pulser", 0.02, 2.0, 0.25, Unit::Sec),
    ParamSpec::toggle("Latch", "Plates", false),
    ParamSpec::int("Steps", "Steps", 1.0, 5.0, 5.0, Unit::None),
    ParamSpec::choice("Order", "Steps", ORDERS, 0),
    ParamSpec::toggle("Move Notes", "Steps", false),
    ParamSpec::lin("Chance", "Chance", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::lin("Wander", "Chance", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Gate", "Notes", 0.05, 1.0, 0.6, Unit::Percent),
    ParamSpec::int("Base Note", "Plates", 24.0, 84.0, 48.0, Unit::None),
    ParamSpec::lin("Press Velocity", "Plates", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::lin("Vel Low", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Vel High", "Dynamics", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::lin("Vel Random", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Vel Accent", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
];

/// The steps it starts with: a little tune, and values rising and falling.
pub fn default_data() -> Vec<u8> {
    steps_data(&[(0, 40), (2, 200), (4, 110), (2, 255), (-1, 0)])
}

/// Data with these steps and no patch points.
pub fn steps_data(steps: &[(i8, u8); N_STEPS]) -> Vec<u8> {
    let mut d = Vec::with_capacity(N_STEPS * 2 + N_PATCH * 2);
    for &(a, b) in steps {
        d.push(a as u8);
        d.push(b);
    }
    for _ in 0..N_PATCH {
        d.extend_from_slice(&0xFFFFu16.to_le_bytes());
    }
    d
}

/// Step `i`: its note (scale steps) and its value (0..1).
pub fn step(data: &[u8], i: usize) -> (i32, f32) {
    let a = data.get(i * 2).map(|&b| (b as i8).clamp(-7, 7) as i32).unwrap_or(0);
    let b = data.get(i * 2 + 1).map(|&b| b as f32 / 255.0).unwrap_or(0.0);
    (a, b)
}

/// Data that is short (older, or empty) filled up from the defaults.
fn whole(data: &mut Vec<u8>) {
    let fresh = default_data();
    if data.len() < fresh.len() {
        let n = data.len();
        data.extend_from_slice(&fresh[n..]);
    }
}

pub fn set_step(data: &mut Vec<u8>, i: usize, note: i32, value: f32) {
    whole(data);
    data[i * 2] = note.clamp(-7, 7) as i8 as u8;
    data[i * 2 + 1] = (value.clamp(0.0, 1.0) * 255.0).round() as u8;
}

/// The instrument parameter on patch point `k`, if one is set.
pub fn patch(data: &[u8], k: usize) -> Option<u16> {
    let at = N_STEPS * 2 + k * 2;
    let v = u16::from_le_bytes([*data.get(at)?, *data.get(at + 1)?]);
    (v != 0xFFFF).then_some(v)
}

pub fn set_patch(data: &mut Vec<u8>, k: usize, param: Option<u16>) {
    whole(data);
    let at = N_STEPS * 2 + k * 2;
    data[at..at + 2].copy_from_slice(&param.unwrap_or(0xFFFF).to_le_bytes());
}

/// What the screen is told: [0] the step, how many, the pulses so far, the
/// note played last; [1] the six outputs, a byte each.
pub fn read_visual(v: [u64; 4]) -> (usize, usize, u32, u8, [f32; 6]) {
    let outs = std::array::from_fn(|k| ((v[1] >> (k * 8)) & 0xFF) as f32 / 255.0);
    ((v[0] & 0xFF) as usize, ((v[0] >> 8) & 0xFF) as usize, ((v[0] >> 16) & 0xFFFF) as u32, ((v[0] >> 32) & 0x7F) as u8, outs)
}

/// A small fixed set of notes (the audio thread never allocates).
#[derive(Clone, Copy, Default)]
struct Notes {
    n: usize,
    v: [u8; MAX_HELD],
}

impl Notes {
    fn remove(&mut self, x: u8) {
        if let Some(i) = self.v[..self.n].iter().position(|&y| y == x) {
            self.v.copy_within(i + 1..self.n, i);
            self.n -= 1;
        }
    }

    fn push(&mut self, x: u8) {
        self.remove(x);
        if self.n == MAX_HELD {
            self.v.copy_within(1..MAX_HELD, 0);
            self.n -= 1;
        }
        self.v[self.n] = x;
        self.n += 1;
    }

    fn last(&self) -> Option<u8> {
        self.n.checked_sub(1).map(|i| self.v[i])
    }

    fn contains(&self, x: u8) -> bool {
        self.v[..self.n].contains(&x)
    }
}

#[derive(Default)]
pub struct TouchState {
    /// The plates held, the last pressed last.
    held: Notes,
    /// The plate that keeps playing with Latch on.
    latched: Option<u8>,
    /// Without the pulser: what each plate sounds, (plate, note).
    sounding: [(u8, u8); MAX_HELD],
    n_sounding: usize,
    /// The pulser's own time (beats), when the song does not run.
    clock: f64,
    last_pulse: Option<i64>,
    step: usize,
    backwards: bool,
    /// The outputs: the pulse's envelope, the stepped and the drifting random.
    env: f32,
    random: f32,
    drift: f32,
    drift_to: f32,
    drift_wait: f32,
    /// The pressure the last note was played with (a latched plate keeps it).
    pressure: f32,
    pulses: u32,
    last_note: u8,
}

impl TouchState {
    fn forget(&mut self, plate: u8) -> Option<u8> {
        let i = self.sounding[..self.n_sounding].iter().position(|s| s.0 == plate)?;
        let note = self.sounding[i].1;
        self.sounding.copy_within(i + 1..self.n_sounding, i);
        self.n_sounding -= 1;
        Some(note)
    }

    fn remember(&mut self, plate: u8, note: u8) {
        if self.n_sounding == MAX_HELD {
            self.sounding.copy_within(1..MAX_HELD, 0);
            self.n_sounding -= 1;
        }
        self.sounding[self.n_sounding] = (plate, note);
        self.n_sounding += 1;
    }
}

fn unit(rng: &mut crate::dsp::Rng) -> f32 {
    (rng.bipolar() + 1.0) * 0.5
}

impl SeqState {
    /// One block of Touch: the plates that came in, the pulser, the outputs.
    pub(crate) fn touch(&mut self, p: &[f32], data: &[u8], out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        let get = |i: usize, or: f32| p.get(i).copied().unwrap_or(or);
        self.shift_in_note = false;
        let pulse_on = get(PULSE, 0.0) >= 0.5;
        let latch = get(LATCH, 0.0) >= 0.5;
        let rate = RATE_BEATS[(get(RATE, 4.0).round().max(0.0) as usize).min(RATE_BEATS.len() - 1)];
        let spb = ctx.samples_per_beat.max(1.0);
        let pressure = ctx.touch[0].clamp(0.0, 1.0);
        let press_vel = get(PRESS_VEL, 0.8).clamp(0.0, 1.0);
        let vel_of = |pressed: f32| (0.8 * (1.0 - press_vel) + pressed.clamp(0.05, 1.0) * press_vel).clamp(0.05, 1.0);

        // The plates that came in: held, let go, played at once below.
        let mut presses = [(0u8, 0f32); MAX_HELD];
        let mut n_press = 0;
        let mut releases = [0u8; MAX_HELD];
        let mut n_release = 0;
        for e in out.iter() {
            if e.on {
                self.touch.held.push(e.pitch);
                self.touch.latched = Some(e.pitch);
                if n_press < MAX_HELD {
                    presses[n_press] = (e.pitch, e.vel);
                    n_press += 1;
                }
            } else {
                self.touch.held.remove(e.pitch);
                if n_release < MAX_HELD {
                    releases[n_release] = e.pitch;
                    n_release += 1;
                }
            }
        }
        out.clear();
        if self.touch.held.n > 0 {
            self.touch.pressure = pressure.max(0.05);
        }

        if !pulse_on {
            // Latch off: what is let go stops (a plate pressed again starts over).
            for &plate in &releases[..n_release] {
                if !latch {
                    if let Some(note) = self.touch.forget(plate) {
                        self.touch_end(note, out);
                    }
                }
            }
            if !latch {
                // Latch just turned off: the plates no longer held stop.
                let mut i = 0;
                while i < self.touch.n_sounding {
                    let (plate, note) = self.touch.sounding[i];
                    if self.touch.held.contains(plate) {
                        i += 1;
                    } else {
                        self.touch.forget(plate);
                        self.touch_end(note, out);
                    }
                }
            } else if n_press > 0 {
                // Latch on: a new touch ends what was latched.
                while self.touch.n_sounding > 0 {
                    let (plate, note) = self.touch.sounding[0];
                    if self.touch.held.contains(plate) && presses[..n_press].iter().any(|p| p.0 == plate) {
                        break;
                    }
                    self.touch.forget(plate);
                    self.touch_end(note, out);
                }
            }
            // Each touch a step further.
            for &(plate, vel) in &presses[..n_press] {
                self.touch_advance(p, data);
                self.touch_trigger();
                let note = self.touch_note(p, data, ctx, plate);
                if let Some(old) = self.touch.forget(plate) {
                    self.touch_end(old, out);
                }
                self.note(ctx, ctx.now, note as i32, vel_of(vel), 1e12, out);
                self.touch.remember(plate, note);
                self.touch.last_note = note;
            }
            self.touch.clock += ctx.len as f64 / spb;
        } else {
            // The pulser: a touch plays at once, the pulses play it again.
            let len = (rate * spb * get(GATE, 0.6).clamp(0.05, 1.0) as f64).max(1.0);
            for &(plate, vel) in &presses[..n_press] {
                self.touch_trigger();
                let note = self.touch_note(p, data, ctx, plate);
                self.note(ctx, ctx.now, note as i32, vel_of(vel), len, out);
                self.touch.last_note = note;
            }
            // Notes held from before the pulser came on end now.
            while self.touch.n_sounding > 0 {
                let (plate, note) = self.touch.sounding[0];
                self.touch.forget(plate);
                self.touch_end(note, out);
            }
            let b0 = if ctx.playing { ctx.beat } else { self.touch.clock };
            let b1 = b0 + ctx.len as f64 / spb;
            self.touch.clock = b1;
            let mut k = (b0 / rate - 1e-9).ceil() as i64;
            // A pulse on a block's edge belongs to the next one (rounding).
            while (k as f64) * rate < b1 - 1e-9 {
                if Some(k) != self.touch.last_pulse {
                    self.touch.last_pulse = Some(k);
                    let at = ctx.now + (((k as f64) * rate - b0).max(0.0) * spb).round() as u64;
                    self.touch_advance(p, data);
                    self.touch_trigger();
                    let source = self.touch.held.last().or(if latch { self.touch.latched } else { None });
                    if let Some(plate) = source {
                        // Chance: some pulses rest.
                        if unit(&mut self.rng) < get(CHANCE, 1.0) {
                            let note = self.touch_note(p, data, ctx, plate);
                            let pressed = if self.touch.held.n > 0 { pressure } else { self.touch.pressure };
                            self.note(ctx, at, note as i32, vel_of(pressed), len, out);
                            self.touch.last_note = note;
                        }
                    }
                }
                k += 1;
            }
        }

        // The outputs, at the end of the block.
        let secs = ctx.len as f32 / ctx.sr.max(1.0);
        self.touch.env *= (-secs / get(DECAY, 0.25).max(0.005)).exp();
        if !pulse_on {
            // Without the pulser the drift wanders on its own.
            self.touch.drift_wait -= secs;
            if self.touch.drift_wait <= 0.0 {
                self.touch.drift_wait = 1.5;
                self.touch.drift_to = unit(&mut self.rng);
            }
        }
        let tau = if pulse_on { (rate * spb / ctx.sr.max(1.0) as f64) as f32 * 2.0 } else { 1.2 };
        self.touch.drift += (self.touch.drift_to - self.touch.drift) * (1.0 - (-secs / tau.max(0.05)).exp());
        let steps = (get(STEPS, 5.0).round() as usize).clamp(1, N_STEPS);
        let outs = [pressure, self.touch.env, step(data, self.touch.step.min(steps - 1)).1, self.touch.random, self.touch.drift, ctx.touch[1].clamp(0.0, 1.0)];
        self.touch_out = outs;
        let packed = outs.iter().enumerate().fold(0u64, |a, (k, v)| a | (((v.clamp(0.0, 1.0) * 255.0).round() as u64) << (k * 8)));
        self.visual[0] = self.touch.step as u64 | (steps as u64) << 8 | ((self.touch.pulses & 0xFFFF) as u64) << 16 | ((self.touch.last_note & 0x7F) as u64) << 32;
        self.visual[1] = packed;
    }

    /// A pulse (or, without the pulser, a touch): the envelope again, a new random.
    fn touch_trigger(&mut self) {
        self.touch.env = 1.0;
        self.touch.random = unit(&mut self.rng);
        self.touch.drift_to = unit(&mut self.rng);
        self.touch.pulses = self.touch.pulses.wrapping_add(1);
    }

    /// The next step, in the order set.
    fn touch_advance(&mut self, p: &[f32], _data: &[u8]) {
        let n = (p.get(STEPS).copied().unwrap_or(5.0).round() as usize).clamp(1, N_STEPS);
        let t = &mut self.touch;
        if n == 1 {
            t.step = 0;
            return;
        }
        t.step = match p.get(ORDER).map(|v| v.round() as usize).unwrap_or(0) {
            1 => {
                // Pendulum: there and back, the ends once.
                if t.step + 1 >= n {
                    t.backwards = true;
                } else if t.step == 0 {
                    t.backwards = false;
                }
                if t.backwards {
                    t.step.min(n - 1).saturating_sub(1)
                } else {
                    (t.step + 1).min(n - 1)
                }
            }
            2 => {
                let r = (unit(&mut self.rng) * n as f32) as usize;
                r.min(n - 1)
            }
            _ => (t.step + 1) % n,
        };
    }

    /// What a plate sounds now: moved by the step, sometimes wandering.
    fn touch_note(&mut self, p: &[f32], data: &[u8], ctx: &MidiCtx, plate: u8) -> u8 {
        let get = |i: usize, or: f32| p.get(i).copied().unwrap_or(or);
        let mut shift = 0;
        if get(MOVE, 0.0) >= 0.5 {
            let n = (get(STEPS, 5.0).round() as usize).clamp(1, N_STEPS);
            shift += step(data, self.touch.step.min(n - 1)).0;
        }
        if get(WANDER, 0.0) > 0.0 && unit(&mut self.rng) < get(WANDER, 0.0) {
            let by = if unit(&mut self.rng) < 0.7 { 1 } else { 2 };
            shift += if unit(&mut self.rng) < 0.5 { by } else { -by };
        }
        if shift == 0 {
            return plate;
        }
        let (d, rest) = ctx.scale.locate(plate as i32);
        (ctx.scale.pitch_at(d + shift) + rest).clamp(0, 127) as u8
    }

    /// Ends a note sounding until its plate was let go.
    fn touch_end(&mut self, note: u8, out: &mut Vec<MidiEvent>) {
        if let Some(i) = self.offs.iter().position(|o| o.1 == note) {
            self.offs.swap_remove(i);
            out.push(MidiEvent::off(0, note));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::midi_fx::{MidiFxKind, MidiFxState};
    use crate::params::defaults;
    use crate::scale::{Scale, ScaleKind};

    const SPB: f64 = 24_000.0;

    fn ctx(now: u64, playing: bool, pressure: f32) -> MidiCtx {
        MidiCtx { now, len: 256, samples_per_beat: SPB, scale: Scale::new(0, ScaleKind::Major), scale_on: true, beat: now as f64 / SPB, playing, sr: 48_000.0, drum_notes: 0, lead: 0.0, touch: [pressure, 0.5] }
    }

    /// Runs `blocks` blocks with plate events at given blocks: (block, pitch, on).
    fn run(p: &[f32], data: &[u8], blocks: u64, plates: &[(u64, u8, bool)], playing: bool) -> (Vec<(u64, u8, bool)>, MidiFxState) {
        let mut st = MidiFxState::new(MidiFxKind::Touch);
        let mut evs = Vec::new();
        for b in 0..blocks {
            let now = b * 256;
            let input: Vec<MidiEvent> = plates.iter().filter(|e| e.0 == b).map(|e| if e.2 { MidiEvent::on(0, e.1, 0.9) } else { MidiEvent::off(0, e.1) }).collect();
            let mut out = Vec::with_capacity(64);
            st.process(MidiFxKind::Touch, p, data, &input, &mut out, &ctx(now, playing, 0.7));
            for e in &out {
                evs.push((now + e.offset as u64, e.pitch, e.on));
            }
        }
        (evs, st)
    }

    #[test]
    fn a_plate_plays_while_it_is_held() {
        let p = defaults(TOUCH);
        let (evs, _) = run(&p, &default_data(), 40, &[(2, 60, true), (30, 60, false)], false);
        assert_eq!(evs, vec![(512, 60, true), (30 * 256, 60, false)]);
    }

    #[test]
    fn the_pulser_plays_the_held_plate_again_with_the_transport_stopped() {
        let mut p = defaults(TOUCH);
        p[PULSE] = 1.0;
        p[RATE] = 6.0; // a beat
        let (evs, _) = run(&p, &default_data(), (4.0 * SPB / 256.0) as u64, &[(1, 62, true)], false);
        let ons: Vec<u64> = evs.iter().filter(|e| e.2 && e.1 == 62).map(|e| e.0).collect();
        // At once, then on every beat.
        assert_eq!(ons.len(), 4, "{ons:?}");
        assert!(ons[1..].iter().all(|&t| (t as f64 / SPB).fract().abs() < 0.02 || (t as f64 / SPB).fract() > 0.98), "{ons:?}");
    }

    #[test]
    fn steps_move_the_notes_through_the_key() {
        let mut p = defaults(TOUCH);
        p[MOVE] = 1.0;
        let data = steps_data(&[(0, 0), (2, 0), (4, 0), (0, 0), (0, 0)]);
        let presses: Vec<(u64, u8, bool)> = (0..3).flat_map(|i| [(i * 4 + 1, 60, true), (i * 4 + 3, 60, false)]).collect();
        let (evs, _) = run(&p, &data, 16, &presses, false);
        let ons: Vec<u8> = evs.iter().filter(|e| e.2).map(|e| e.1).collect();
        // Each touch a step further: two up (E), four up (G), back to C... in C major.
        assert_eq!(ons, vec![64, 67, 60]);
    }

    #[test]
    fn latch_keeps_the_last_plate_and_a_new_touch_ends_it() {
        let mut p = defaults(TOUCH);
        p[LATCH] = 1.0;
        let (evs, _) = run(&p, &default_data(), 20, &[(1, 60, true), (3, 60, false), (10, 64, true), (12, 64, false)], false);
        assert_eq!(evs, vec![(256, 60, true), (2560, 60, false), (2560, 64, true)]);
    }

    #[test]
    fn its_outputs_reach_the_modulators() {
        let mut p = defaults(TOUCH);
        p[PULSE] = 1.0;
        let (_, st) = run(&p, &default_data(), 30, &[(1, 60, true)], true);
        let o = st.touch_out().expect("touch");
        assert!((o[0] - 0.7).abs() < 1e-3, "pressure {o:?}");
        assert!(o[1] > 0.0 && o[1] <= 1.0, "the pulse decays {o:?}");
        assert!((o[5] - 0.5).abs() < 1e-3, "position {o:?}");
        let (step, n, pulses, _, outs) = read_visual(st.visual().unwrap());
        assert!(step < n && n == 5 && pulses > 0 && (outs[0] - 0.7).abs() < 0.01);
    }

    #[test]
    fn data_keeps_steps_and_patch_points() {
        let mut d = default_data();
        set_step(&mut d, 3, -5, 0.5);
        set_patch(&mut d, 2, Some(17));
        assert_eq!(step(&d, 3), (-5, 128.0 / 255.0));
        assert_eq!((patch(&d, 2), patch(&d, 0)), (Some(17), None));
        let mut short = vec![1u8, 2];
        set_patch(&mut short, 5, Some(3));
        assert_eq!((patch(&short, 5), step(&short, 0).0), (Some(3), 1));
    }
}

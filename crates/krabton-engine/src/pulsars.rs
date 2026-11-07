//! Pulsars: a field of nodes joined by lines. A pulsar fires a pulse down each
//! of its lines on the beat (every so many beats); a pulse takes as long to
//! travel a line as the line is long (the field's *speed*, in pixels a beat),
//! so where things are placed is the rhythm. A pulse arriving at an orb plays
//! the orb's note on its track, and goes on down the orb's other lines; a
//! gate lets every so many through, a chance some, a reflector sends it back
//! the way it came. A node hears a pulse once, so a ring of orbs does not ring
//! for ever, and a pulse dies after so many lines.
//!
//! The project holds the field (`PulsarSet`); the engine runs it (`PulsarRt`)
//! sample-true, in the session's time, and the notes go to the tracks as
//! played notes do.

use crate::midi_fx::MidiEvent;
use crate::scale::Scale;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

/// How often a pulsar fires: (name, beats).
pub const RATES: [(&str, f64); 8] = [("1/16", 0.25), ("1/8", 0.5), ("1/8 D", 0.75), ("1/4", 1.0), ("1/4 D", 1.5), ("1/2", 2.0), ("1 Bar", 4.0), ("2 Bars", 8.0)];
/// Lines a pulse travels before it dies.
pub const MAX_HOPS: u8 = 24;
/// What the field can hold, and how many pulses can be under way.
pub const MAX_NODES: usize = 256;
pub const MAX_FLIGHTS: usize = 1024;
/// The pulses under way the editor is told of.
pub const SHOWN_FLIGHTS: usize = 128;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PulsarSet {
    pub nodes: Vec<PNode>,
    pub links: Vec<PLink>,
    /// How far a pulse travels in a beat (pixels of the field).
    #[serde(default = "default_speed")]
    pub speed: f32,
}

fn default_speed() -> f32 {
    160.0
}

impl Default for PulsarSet {
    fn default() -> Self {
        Self { nodes: Vec::new(), links: Vec::new(), speed: default_speed() }
    }
}

impl PulsarSet {
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub fn node(&self, id: u64) -> Option<&PNode> {
        self.nodes.iter().find(|n| n.id == id)
    }

    /// A line's length (pixels), from where its nodes are.
    pub fn length(&self, l: &PLink) -> f32 {
        match (self.node(l.a), self.node(l.b)) {
            (Some(a), Some(b)) => ((a.x - b.x).powi(2) + (a.y - b.y).powi(2)).sqrt(),
            _ => 0.0,
        }
    }

    /// Takes a node away, and its lines.
    pub fn remove(&mut self, id: u64) {
        self.nodes.retain(|n| n.id != id);
        self.links.retain(|l| l.a != id && l.b != id);
    }

    /// Joins two nodes (once; a line back the other way makes it two-way).
    pub fn link(&mut self, a: u64, b: u64) {
        if a == b || self.node(a).is_none() || self.node(b).is_none() {
            return;
        }
        if let Some(l) = self.links.iter_mut().find(|l| (l.a == b && l.b == a) || (l.a == a && l.b == b)) {
            l.one_way = false;
            return;
        }
        self.links.push(PLink { a, b, one_way: false });
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PNode {
    pub id: u64,
    pub x: f32,
    pub y: f32,
    pub kind: PKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum PKind {
    /// Fires a pulse down each of its lines, every `rate` (see `RATES`).
    Pulsar { rate: u8, on: bool },
    /// Plays a note on a track: so many steps of the song's key from C3, in octave `octave`.
    Orb { track: Option<u64>, degree: i32, octave: i32, velocity: f32, length: f32 },
    /// Lets one pulse in `every` through.
    Gate { every: u8 },
    /// Lets a pulse through so often (0..1).
    Chance { chance: f32 },
    /// Sends a pulse back the way it came.
    Reflector,
}

impl PKind {
    pub fn name(&self) -> &'static str {
        match self {
            PKind::Pulsar { .. } => "Pulsar",
            PKind::Orb { .. } => "Orb",
            PKind::Gate { .. } => "Gate",
            PKind::Chance { .. } => "Chance",
            PKind::Reflector => "Reflector",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct PLink {
    pub a: u64,
    pub b: u64,
    /// Pulses go from `a` to `b` only.
    #[serde(default)]
    pub one_way: bool,
}

/// The pitch an orb plays.
pub fn orb_pitch(scale: &Scale, degree: i32, octave: i32) -> u8 {
    let (home, _) = scale.locate(48 + 12 * octave);
    scale.pitch_at(home + degree).clamp(0, 127) as u8
}

/// A pulse under way: down which line, from which end, since when, until when.
#[derive(Clone, Copy, Debug)]
pub struct Flight {
    pub line: usize,
    /// From the line's `a` end (else from `b`).
    pub from_a: bool,
    pub start: f64,
    pub arrive: f64,
    pub pulse: u64,
    pub hops: u8,
}

/// A line resolved to node places: (a, b, length, one way).
#[derive(Clone, Copy)]
struct Line {
    a: usize,
    b: usize,
    len: f64,
    one_way: bool,
}

/// Where a note goes: the track (by id), how far into the block, the event.
pub type Out = (u64, MidiEvent);

/// The field running in the engine.
pub struct PulsarRt {
    lines: Vec<Line>,
    /// Each node's last pulse heard, and a gate's count.
    heard: Vec<u64>,
    counts: Vec<u32>,
    flights: Vec<Flight>,
    /// Notes to end: (beat, track, pitch).
    offs: Vec<(f64, u64, u8)>,
    next_pulse: u64,
    rng: u32,
    /// Nodes struck since the editor last looked (a bit each).
    pub struck: [u64; MAX_NODES / 64],
}

impl Default for PulsarRt {
    fn default() -> Self {
        Self {
            lines: Vec::with_capacity(4 * MAX_NODES),
            heard: vec![0; MAX_NODES],
            counts: vec![0; MAX_NODES],
            flights: Vec::with_capacity(MAX_FLIGHTS),
            offs: Vec::with_capacity(512),
            next_pulse: 1,
            rng: 0x9E37_79B9,
            struck: [0; MAX_NODES / 64],
        }
    }
}

impl PulsarRt {
    /// The field changed: its lines again (pulses under way keep going while their line is there).
    pub fn resolve(&mut self, set: &PulsarSet) {
        self.lines.clear();
        let place = |id: u64| set.nodes.iter().take(MAX_NODES).position(|n| n.id == id);
        for l in &set.links {
            if let (Some(a), Some(b)) = (place(l.a), place(l.b)) {
                if self.lines.len() < self.lines.capacity() {
                    self.lines.push(Line { a, b, len: set.length(l) as f64, one_way: l.one_way });
                }
            }
        }
        let n = self.lines.len();
        self.flights.retain(|f| f.line < n);
    }

    /// The pulses under way.
    pub fn flights(&self) -> &[Flight] {
        &self.flights
    }

    /// Everything stops: the notes end now.
    pub fn stop(&mut self, out: &mut impl FnMut(Out)) {
        self.flights.clear();
        for (_, track, pitch) in self.offs.drain(..) {
            out((track, MidiEvent::off(0, pitch)));
        }
    }

    fn random(&mut self) -> f32 {
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.rng = x;
        (x >> 8) as f32 / 16_777_216.0
    }

    /// Sends a pulse from node `from` down its lines (`except` the one it came by).
    fn send(&mut self, from: usize, except: Option<usize>, at: f64, pulse: u64, hops: u8, speed: f64) {
        if hops == 0 {
            return;
        }
        for (k, line) in self.lines.iter().enumerate() {
            if Some(k) == except || self.flights.len() >= self.flights.capacity() {
                continue;
            }
            let from_a = if line.a == from {
                true
            } else if line.b == from && !line.one_way {
                false
            } else {
                continue;
            };
            let travel = (line.len / speed.max(1.0)).max(1e-4);
            self.flights.push(Flight { line: k, from_a, start: at, arrive: at + travel, pulse, hops: hops - 1 });
        }
    }

    /// Runs the field from beat `b0` to `b1` (the session's), `spb` frames a
    /// beat: the pulsars fire, the pulses travel, the orbs play into `out`.
    pub fn run(&mut self, set: &PulsarSet, scale: &Scale, b0: f64, b1: f64, spb: f64, frames: usize, out: &mut impl FnMut(Out)) {
        let speed = set.speed.max(1.0) as f64;
        let offset = |beat: f64| (((beat - b0) * spb).max(0.0) as usize).min(frames.saturating_sub(1)) as u32;
        // The pulsars fire.
        for (i, n) in set.nodes.iter().enumerate().take(MAX_NODES) {
            if let PKind::Pulsar { rate, on: true } = n.kind {
                let every = RATES[(rate as usize).min(RATES.len() - 1)].1;
                let mut k = (b0 / every - 1e-9).ceil();
                while k * every < b1 {
                    let at = k * every;
                    let pulse = self.next_pulse;
                    self.next_pulse += 1;
                    self.heard[i] = pulse;
                    self.struck[i / 64] |= 1 << (i % 64);
                    self.send(i, None, at, pulse, MAX_HOPS, speed);
                    k += 1.0;
                }
            }
        }
        // The pulses that arrive, in the order they do (those sent on may arrive too).
        loop {
            let Some((j, f)) = self.flights.iter().enumerate().filter(|(_, f)| f.arrive < b1).min_by(|a, b| a.1.arrive.total_cmp(&b.1.arrive)).map(|(j, f)| (j, *f)) else { break };
            self.flights.swap_remove(j);
            let line = self.lines[f.line];
            let to = if f.from_a { line.b } else { line.a };
            let Some(node) = set.nodes.get(to) else { continue };
            let reflector = matches!(node.kind, PKind::Reflector);
            if self.heard[to] == f.pulse && !reflector {
                continue;
            }
            self.heard[to] = f.pulse;
            self.struck[to / 64] |= 1 << (to % 64);
            let at = f.arrive;
            let goes_on = match node.kind {
                PKind::Orb { track, degree, octave, velocity, length } => {
                    if let Some(track) = track {
                        let pitch = orb_pitch(scale, degree, octave);
                        out((track, MidiEvent::on(offset(at), pitch, velocity.clamp(0.01, 1.0))));
                        if self.offs.len() < self.offs.capacity() {
                            self.offs.push((at + length.max(0.01) as f64, track, pitch));
                        }
                    }
                    true
                }
                PKind::Pulsar { .. } => true,
                PKind::Gate { every } => {
                    self.counts[to] += 1;
                    let open = self.counts[to] >= every.max(1) as u32;
                    if open {
                        self.counts[to] = 0;
                    }
                    open
                }
                PKind::Chance { chance } => self.random() < chance,
                PKind::Reflector => {
                    // Back down the line it came by, the way it came: a new
                    // pulse, so what it passed on the way plays again.
                    if f.hops > 0 && self.flights.len() < self.flights.capacity() && !line.one_way {
                        let pulse = self.next_pulse;
                        self.next_pulse += 1;
                        self.heard[to] = pulse;
                        self.flights.push(Flight { line: f.line, from_a: !f.from_a, start: at, arrive: at + (line.len / speed).max(1e-4), pulse, hops: f.hops - 1 });
                    }
                    false
                }
            };
            if goes_on {
                self.send(to, Some(f.line), at, f.pulse, f.hops, speed);
            }
        }
        // The notes that end now.
        let mut i = 0;
        while i < self.offs.len() {
            let (at, track, pitch) = self.offs[i];
            if at < b1 {
                out((track, MidiEvent::off(offset(at), pitch)));
                self.offs.swap_remove(i);
            } else {
                i += 1;
            }
        }
    }

    /// The nodes struck since last asked (and forgets them).
    pub fn take_struck(&mut self) -> [u64; MAX_NODES / 64] {
        std::mem::take(&mut self.struck)
    }
}

/// What the editor is told: the pulses under way (each its line, the end it
/// left, how far along: see `pack`), and the nodes struck since it last looked.
pub struct PulsarStatus {
    pub flights: [AtomicU64; SHOWN_FLIGHTS],
    pub count: AtomicU32,
    pub struck: [AtomicU64; MAX_NODES / 64],
}

impl Default for PulsarStatus {
    fn default() -> Self {
        Self { flights: std::array::from_fn(|_| AtomicU64::new(0)), count: AtomicU32::new(0), struck: std::array::from_fn(|_| AtomicU64::new(0)) }
    }
}

impl PulsarStatus {
    /// The engine says where things are, at beat `now`.
    pub fn publish(&self, rt: &mut PulsarRt, now: f64) {
        let n = rt.flights.len().min(SHOWN_FLIGHTS);
        for (slot, f) in self.flights.iter().zip(rt.flights.iter()) {
            slot.store(pack(f.line, f.from_a, progress(f, now)), Ordering::Relaxed);
        }
        self.count.store(n as u32, Ordering::Relaxed);
        for (slot, bits) in self.struck.iter().zip(rt.take_struck()) {
            if bits != 0 {
                slot.fetch_or(bits, Ordering::Relaxed);
            }
        }
    }

    /// The pulses under way: (line, from its `a` end, how far along).
    pub fn read(&self) -> Vec<(usize, bool, f32)> {
        let n = (self.count.load(Ordering::Relaxed) as usize).min(SHOWN_FLIGHTS);
        self.flights[..n].iter().map(|f| unpack(f.load(Ordering::Relaxed))).collect()
    }

    /// The nodes struck since last asked, by place.
    pub fn take_struck(&self) -> Vec<usize> {
        let mut out = Vec::new();
        for (w, slot) in self.struck.iter().enumerate() {
            let bits = slot.swap(0, Ordering::Relaxed);
            for b in 0..64 {
                if bits & (1 << b) != 0 {
                    out.push(w * 64 + b);
                }
            }
        }
        out
    }
}

/// A pulse under way for the editor, in one number.
pub fn pack(line: usize, from_a: bool, along: f64) -> u64 {
    (line as u64 & 0xffff) | (from_a as u64) << 16 | ((along.clamp(0.0, 1.0) * 65535.0) as u64) << 32
}

pub fn unpack(v: u64) -> (usize, bool, f32) {
    ((v & 0xffff) as usize, v >> 16 & 1 == 1, ((v >> 32) & 0xffff) as f32 / 65535.0)
}

impl PulsarRt {
    /// Whether anything is still under way or sounding.
    pub fn busy(&self) -> bool {
        !self.flights.is_empty() || !self.offs.is_empty()
    }
}

/// The lines of a field as the engine numbers them (those whose nodes are there).
pub fn line_order(set: &PulsarSet) -> Vec<usize> {
    let place = |id: u64| set.nodes.iter().take(MAX_NODES).position(|n| n.id == id);
    set.links.iter().enumerate().filter(|(_, l)| place(l.a).is_some() && place(l.b).is_some()).map(|(k, _)| k).take(4 * MAX_NODES).collect()
}

/// Where a pulse is along its line (0 at the end it left, 1 where it arrives).
pub fn progress(f: &Flight, beat: f64) -> f64 {
    ((beat - f.start) / (f.arrive - f.start).max(1e-9)).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scale::ScaleKind;

    fn c_major() -> Scale {
        Scale::new(0, ScaleKind::Major)
    }

    fn orb(id: u64, x: f32, degree: i32) -> PNode {
        PNode { id, x, y: 0.0, kind: PKind::Orb { track: Some(7), degree, octave: 0, velocity: 0.8, length: 0.25 } }
    }

    fn pulsar(id: u64, x: f32, rate: u8) -> PNode {
        PNode { id, x, y: 0.0, kind: PKind::Pulsar { rate, on: true } }
    }

    /// Runs the field over so many beats in blocks; the notes struck: (beat, pitch).
    fn run(set: &PulsarSet, beats: f64) -> Vec<(f64, u8)> {
        let mut rt = PulsarRt::default();
        rt.resolve(set);
        let spb = 24000.0;
        let mut ons = Vec::new();
        // Counted, not added up: a sum of blocks runs past the end.
        let blocks = (beats * spb / 256.0).round() as usize;
        for i in 0..blocks {
            let (b0, b1) = (i as f64 * 256.0 / spb, (i + 1) as f64 * 256.0 / spb);
            rt.run(set, &c_major(), b0, b1, spb, 256, &mut |(track, e): Out| {
                assert_eq!(track, 7);
                if e.on {
                    ons.push((b0 + e.offset as f64 / spb, e.pitch));
                }
            });
        }
        ons
    }

    fn field(nodes: Vec<PNode>, links: &[(u64, u64)]) -> PulsarSet {
        let mut s = PulsarSet { nodes, links: Vec::new(), speed: 100.0 };
        for &(a, b) in links {
            s.link(a, b);
        }
        s
    }

    #[test]
    fn a_pulse_takes_as_long_as_its_line_is_long() {
        // A quarter-note pulsar 100 px from an orb at 100 px a beat: a beat later.
        let s = field(vec![pulsar(1, 0.0, 3), orb(2, 100.0, 0)], &[(1, 2)]);
        let ons = run(&s, 4.0);
        assert_eq!(ons.len(), 3, "{ons:?}");
        assert!((ons[0].0 - 1.0).abs() < 0.02, "{ons:?}");
        assert_eq!(ons[0].1, 48, "C3 in C major");
        // Twice as far, twice as long.
        let s = field(vec![pulsar(1, 0.0, 3), orb(2, 200.0, 2)], &[(1, 2)]);
        let ons = run(&s, 4.0);
        assert!((ons[0].0 - 2.0).abs() < 0.02, "{ons:?}");
        assert_eq!(ons[0].1, 52, "two steps up: E3");
    }

    #[test]
    fn a_pulse_goes_on_from_orb_to_orb_but_not_back() {
        let s = field(vec![pulsar(1, 0.0, 6), orb(2, 100.0, 0), orb(3, 200.0, 4)], &[(1, 2), (2, 3)]);
        let ons = run(&s, 4.0);
        assert_eq!(ons.iter().map(|o| o.1).collect::<Vec<_>>(), [48, 55], "{ons:?}");
        assert!((ons[1].0 - 2.0).abs() < 0.02);
    }

    #[test]
    fn a_ring_does_not_ring_for_ever() {
        let s = field(vec![pulsar(1, 0.0, 7), orb(2, 100.0, 0), orb(3, 100.0, 1), orb(4, 50.0, 2)], &[(1, 2), (2, 3), (3, 4), (4, 2)]);
        let ons = run(&s, 6.0);
        assert!(ons.len() <= 3, "each orb once a pulse: {ons:?}");
    }

    #[test]
    fn a_one_way_line_goes_one_way() {
        let mut s = field(vec![pulsar(1, 0.0, 6), orb(2, 100.0, 0)], &[(2, 1)]);
        s.links[0].one_way = true;
        assert!(run(&s, 4.0).is_empty(), "from the orb to the pulsar only");
        s.links[0] = PLink { a: 1, b: 2, one_way: true };
        assert_eq!(run(&s, 4.0).len(), 1);
    }

    #[test]
    fn a_gate_lets_every_so_many_through() {
        let s = field(vec![pulsar(1, 0.0, 3), PNode { id: 2, x: 50.0, y: 0.0, kind: PKind::Gate { every: 2 } }, orb(3, 100.0, 0)], &[(1, 2), (2, 3)]);
        let ons = run(&s, 8.0);
        assert!((3..=4).contains(&ons.len()), "half of eight: {ons:?}");
    }

    #[test]
    fn chance_lets_some_through() {
        let s = field(vec![pulsar(1, 0.0, 0), PNode { id: 2, x: 50.0, y: 0.0, kind: PKind::Chance { chance: 0.5 } }, orb(3, 100.0, 0)], &[(1, 2), (2, 3)]);
        let n = run(&s, 16.0).len();
        assert!(n > 15 && n < 50, "about half of 63: {n}");
    }

    #[test]
    fn a_reflector_sends_it_back() {
        // Pulsar - orb - reflector: the orb plays going out and coming back.
        let s = field(vec![pulsar(1, 0.0, 7), orb(2, 100.0, 0), PNode { id: 3, x: 150.0, y: 0.0, kind: PKind::Reflector }], &[(1, 2), (2, 3)]);
        let ons = run(&s, 4.0);
        assert_eq!(ons.len(), 2, "{ons:?}");
        assert!((ons[1].0 - 2.0).abs() < 0.02, "half a beat there, half a beat back: {ons:?}");
    }

    #[test]
    fn its_notes_end_after_their_length() {
        let s = field(vec![pulsar(1, 0.0, 6), orb(2, 100.0, 0)], &[(1, 2)]);
        let mut rt = PulsarRt::default();
        rt.resolve(&s);
        let mut evs = Vec::new();
        rt.run(&s, &c_major(), 0.0, 2.0, 1000.0, 2000, &mut |(_, e): Out| evs.push((e.on, e.offset)));
        assert_eq!(evs, [(true, 1000), (false, 1250)]);
    }

    #[test]
    fn stopping_ends_the_notes() {
        let s = field(vec![pulsar(1, 0.0, 6), orb(2, 100.0, 0)], &[(1, 2)]);
        let mut rt = PulsarRt::default();
        let mut s2 = s.clone();
        if let PKind::Orb { length, .. } = &mut s2.nodes[1].kind {
            *length = 8.0;
        }
        rt.resolve(&s2);
        rt.run(&s2, &c_major(), 0.0, 1.5, 1000.0, 1500, &mut |_| {});
        let mut offs = 0;
        rt.stop(&mut |(_, e): Out| offs += usize::from(!e.on));
        assert_eq!(offs, 1);
        assert!(rt.flights().is_empty());
    }

    #[test]
    fn the_editor_is_told_where_the_pulses_are() {
        let (line, from_a, along) = unpack(pack(300, true, 0.25));
        assert!(line == 300 && from_a && (along - 0.25).abs() < 1e-4);
        let (line, from_a, along) = unpack(pack(7, false, 0.5));
        assert_eq!((line, from_a), (7, false));
        assert!((along - 0.5).abs() < 1e-4);
        let s = field(vec![pulsar(1, 0.0, 6), orb(2, 100.0, 0)], &[(1, 2)]);
        let mut rt = PulsarRt::default();
        rt.resolve(&s);
        rt.run(&s, &c_major(), 0.0, 0.5, 1000.0, 500, &mut |_| {});
        let st = PulsarStatus::default();
        st.publish(&mut rt, 0.5);
        let shown = st.read();
        assert_eq!(shown.len(), 1);
        assert!((shown[0].2 - 0.5).abs() < 1e-3, "half way down a line of a beat");
        assert_eq!(st.take_struck(), [0], "the pulsar fired");
        assert!(st.take_struck().is_empty());
        assert_eq!(line_order(&s), [0]);
    }

    #[test]
    fn a_field_survives_a_save_and_lines_are_made_once() {
        let mut s = field(vec![pulsar(1, 0.0, 3), orb(2, 100.0, 0)], &[(1, 2), (2, 1), (1, 1)]);
        assert_eq!(s.links.len(), 1);
        let back: PulsarSet = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        assert_eq!(back, s);
        s.remove(2);
        assert!(s.links.is_empty());
    }
}

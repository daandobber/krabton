//! Pulsars: a field of nodes joined by lines. A pulsar fires a pulse down each
//! of its lines (on the beat, by chance, or when clicked); a pulse takes as
//! long to travel a line as the line is long (the field's *speed*, in pixels a
//! beat), so where things are placed is the rhythm. A pulse arriving at an orb
//! plays the orb's note (or chord) on its track and goes on down the orb's
//! other lines; a gate lets every so many through, a chance some, a reflector
//! sends it back the way it came, a delay holds it a while, a router sends each
//! pulse down the next of its lines in turn, a shift moves the notes it will
//! play up or down the key. A node hears a pulse once, so a ring of orbs does
//! not ring for ever, and a pulse dies after so many lines.
//!
//! The project holds the field (`PulsarSet`); the engine runs it (`PulsarRt`)
//! sample-true, in the session's time, and the notes go to the tracks as
//! played notes do.

use crate::midi_fx::MidiEvent;
use crate::scale::Scale;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

/// How often a pulsar fires, how long a delay holds, how long a note is: (name, beats).
pub const RATES: [(&str, f64); 8] = [("1/16", 0.25), ("1/8", 0.5), ("1/8 D", 0.75), ("1/4", 1.0), ("1/4 D", 1.5), ("1/2", 2.0), ("1 Bar", 4.0), ("2 Bars", 8.0)];
/// How a pulsar fires.
pub const MODES: [&str; 3] = ["Steady", "Random", "Manual"];
/// What an orb plays: one note, or a chord on it (steps of the key).
pub const CHORDS: [(&str, &[i32]); 4] = [("One Note", &[0]), ("Fifth", &[0, 4]), ("Triad", &[0, 2, 4]), ("Seventh", &[0, 2, 4, 6])];
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

fn one() -> f32 {
    1.0
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
    /// Fires a pulse down each of its lines: every `rate` (see `RATES`), or
    /// then by `chance` (Random), or when clicked (Manual); its pulses play
    /// as hard as `strength` says.
    Pulsar {
        rate: u8,
        on: bool,
        #[serde(default)]
        mode: u8,
        #[serde(default = "one")]
        chance: f32,
        #[serde(default = "one")]
        strength: f32,
    },
    /// Plays a note (or a chord on it, see `CHORDS`) on a track: so many steps
    /// of the song's key from C3, in octave `octave`; or a fixed note (a drum).
    Orb {
        track: Option<u64>,
        degree: i32,
        octave: i32,
        velocity: f32,
        length: f32,
        #[serde(default)]
        chord: u8,
        #[serde(default)]
        note: Option<u8>,
    },
    /// Lets one pulse in `every` through.
    Gate { every: u8 },
    /// Lets a pulse through so often (0..1).
    Chance { chance: f32 },
    /// Sends a pulse back the way it came.
    Reflector,
    /// Holds a pulse so long (see `RATES`) before it goes on.
    Delay { wait: u8 },
    /// Sends each pulse down the next of its lines in turn.
    Router,
    /// The notes a pulse plays from here on are so many steps of the key higher.
    Shift { steps: i8 },
}

impl PKind {
    pub fn name(&self) -> &'static str {
        match self {
            PKind::Pulsar { .. } => "Pulsar",
            PKind::Orb { .. } => "Orb",
            PKind::Gate { .. } => "Gate",
            PKind::Chance { .. } => "Chance",
            PKind::Reflector => "Reflector",
            PKind::Delay { .. } => "Delay",
            PKind::Router => "Router",
            PKind::Shift { .. } => "Shift",
        }
    }

    /// A pulsar as it is placed.
    pub fn pulsar() -> Self {
        PKind::Pulsar { rate: 3, on: true, mode: 0, chance: 1.0, strength: 1.0 }
    }

    /// An orb as it is placed.
    pub fn orb(track: Option<u64>, degree: i32) -> Self {
        PKind::Orb { track, degree, octave: 0, velocity: 0.8, length: 0.25, chord: 0, note: None }
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

/// The notes an orb plays with a pulse that is `shift` steps up the key.
pub fn orb_notes(scale: &Scale, kind: &PKind, shift: i32) -> Vec<u8> {
    match *kind {
        PKind::Orb { note: Some(n), .. } => vec![n],
        PKind::Orb { degree, octave, chord, .. } => {
            let steps = CHORDS[(chord as usize).min(CHORDS.len() - 1)].1;
            steps.iter().map(|s| orb_pitch(scale, degree + shift + s, octave)).collect()
        }
        _ => Vec::new(),
    }
}

/// A pulse under way: down which line, from which end, since when, until
/// when; what it carries (its shift up the key, how hard, which pulsar).
#[derive(Clone, Copy, Debug)]
pub struct Flight {
    pub line: usize,
    /// From the line's `a` end (else from `b`).
    pub from_a: bool,
    pub start: f64,
    pub arrive: f64,
    pub carry: Carry,
}

/// What a pulse carries from node to node.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Carry {
    pub pulse: u64,
    pub hops: u8,
    pub shift: i32,
    pub strength: f32,
    /// The pulsar it came from (its place), for its colour.
    pub origin: u8,
}

/// A line resolved to node places: (a, b, length, one way).
#[derive(Clone, Copy)]
struct Line {
    a: usize,
    b: usize,
    len: f64,
    one_way: bool,
}

/// Where a note goes: the track (by id), the event.
pub type Out = (u64, MidiEvent);

/// The field running in the engine.
pub struct PulsarRt {
    lines: Vec<Line>,
    /// Each node's last pulse heard, and a gate's or router's count.
    heard: Vec<u64>,
    counts: Vec<u32>,
    flights: Vec<Flight>,
    /// Notes to end: (beat, track, pitch).
    offs: Vec<(f64, u64, u8)>,
    /// Pulsars clicked: they fire at the start of the next block.
    fire: Vec<u64>,
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
            fire: Vec::with_capacity(16),
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

    /// A pulsar clicked fires.
    pub fn fire(&mut self, node: u64) {
        if self.fire.len() < self.fire.capacity() {
            self.fire.push(node);
        }
    }

    /// Whether anything is still under way or sounding.
    pub fn busy(&self) -> bool {
        !self.flights.is_empty() || !self.offs.is_empty()
    }

    /// Everything stops: the notes end now.
    pub fn stop(&mut self, out: &mut impl FnMut(Out)) {
        self.flights.clear();
        self.fire.clear();
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

    /// The lines a pulse at node `from` can leave by (`except` the one it came by).
    fn ways_out(&self, from: usize, except: Option<usize>) -> impl Iterator<Item = (usize, bool)> + '_ {
        self.lines.iter().enumerate().filter_map(move |(k, line)| {
            if Some(k) == except {
                None
            } else if line.a == from {
                Some((k, true))
            } else if line.b == from && !line.one_way {
                Some((k, false))
            } else {
                None
            }
        })
    }

    fn launch(&mut self, line: usize, from_a: bool, at: f64, carry: Carry, speed: f64) {
        if self.flights.len() >= self.flights.capacity() || carry.hops == 0 {
            return;
        }
        let travel = (self.lines[line].len / speed.max(1.0)).max(1e-4);
        self.flights.push(Flight { line, from_a, start: at, arrive: at + travel, carry: Carry { hops: carry.hops - 1, ..carry } });
    }

    /// Sends a pulse from node `from` down all its lines (`except` the one it came by).
    fn send(&mut self, from: usize, except: Option<usize>, at: f64, carry: Carry, speed: f64) {
        let mut ways = [(0usize, false); 64];
        let mut n = 0;
        for w in self.ways_out(from, except) {
            if n < ways.len() {
                ways[n] = w;
                n += 1;
            }
        }
        for &(k, from_a) in &ways[..n] {
            self.launch(k, from_a, at, carry, speed);
        }
    }

    fn fire_pulsar(&mut self, i: usize, at: f64, strength: f32, speed: f64) {
        let pulse = self.next_pulse;
        self.next_pulse += 1;
        self.heard[i] = pulse;
        self.struck[i / 64] |= 1 << (i % 64);
        let carry = Carry { pulse, hops: MAX_HOPS, shift: 0, strength, origin: i.min(255) as u8 };
        self.send(i, None, at, carry, speed);
    }

    /// Runs the field from beat `b0` to `b1` (the session's), `spb` frames a
    /// beat: the pulsars fire, the pulses travel, the orbs play into `out`.
    pub fn run(&mut self, set: &PulsarSet, scale: &Scale, b0: f64, b1: f64, spb: f64, frames: usize, out: &mut impl FnMut(Out)) {
        let speed = set.speed.max(1.0) as f64;
        let offset = |beat: f64| (((beat - b0) * spb).max(0.0) as usize).min(frames.saturating_sub(1)) as u32;
        // Pulsars clicked.
        let clicked: Vec<u64> = self.fire.drain(..).collect();
        for id in clicked {
            if let Some((i, n)) = set.nodes.iter().enumerate().take(MAX_NODES).find(|(_, n)| n.id == id) {
                if let PKind::Pulsar { strength, .. } = n.kind {
                    self.fire_pulsar(i, b0, strength, speed);
                }
            }
        }
        // The pulsars fire on their beats (by chance, for a random one).
        for (i, n) in set.nodes.iter().enumerate().take(MAX_NODES) {
            if let PKind::Pulsar { rate, on: true, mode, chance, strength } = n.kind {
                if mode == 2 {
                    continue;
                }
                let every = RATES[(rate as usize).min(RATES.len() - 1)].1;
                let mut k = (b0 / every - 1e-9).ceil();
                while k * every < b1 {
                    if mode == 0 || self.random() < chance {
                        self.fire_pulsar(i, k * every, strength, speed);
                    }
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
            if self.heard[to] == f.carry.pulse && !reflector {
                continue;
            }
            self.heard[to] = f.carry.pulse;
            self.struck[to / 64] |= 1 << (to % 64);
            let at = f.arrive;
            let mut carry = f.carry;
            match node.kind {
                PKind::Orb { track, velocity, length, .. } => {
                    if let Some(track) = track {
                        let vel = (velocity * carry.strength).clamp(0.01, 1.0);
                        for pitch in orb_notes(scale, &node.kind, carry.shift) {
                            out((track, MidiEvent::on(offset(at), pitch, vel)));
                            if self.offs.len() < self.offs.capacity() {
                                self.offs.push((at + length.max(0.01) as f64, track, pitch));
                            }
                        }
                    }
                    self.send(to, Some(f.line), at, carry, speed);
                }
                PKind::Pulsar { .. } => self.send(to, Some(f.line), at, carry, speed),
                PKind::Gate { every } => {
                    self.counts[to] += 1;
                    if self.counts[to] >= every.max(1) as u32 {
                        self.counts[to] = 0;
                        self.send(to, Some(f.line), at, carry, speed);
                    }
                }
                PKind::Chance { chance } => {
                    if self.random() < chance {
                        self.send(to, Some(f.line), at, carry, speed);
                    }
                }
                PKind::Reflector => {
                    // Back down the line it came by, the way it came: a new
                    // pulse, so what it passed on the way plays again.
                    if !line.one_way {
                        let pulse = self.next_pulse;
                        self.next_pulse += 1;
                        self.heard[to] = pulse;
                        self.launch(f.line, !f.from_a, at, Carry { pulse, ..carry }, speed);
                    }
                }
                PKind::Delay { wait } => {
                    let held = at + RATES[(wait as usize).min(RATES.len() - 1)].1;
                    self.send(to, Some(f.line), held, carry, speed);
                }
                PKind::Router => {
                    let mut ways = [(0usize, false); 64];
                    let mut n = 0;
                    for w in self.ways_out(to, Some(f.line)) {
                        if n < ways.len() {
                            ways[n] = w;
                            n += 1;
                        }
                    }
                    if n > 0 {
                        let (k, from_a) = ways[self.counts[to] as usize % n];
                        self.counts[to] = self.counts[to].wrapping_add(1);
                        self.launch(k, from_a, at, carry, speed);
                    }
                }
                PKind::Shift { steps } => {
                    carry.shift += steps as i32;
                    self.send(to, Some(f.line), at, carry, speed);
                }
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
/// left, how far along, its pulsar: see `pack`), and the nodes struck since it last looked.
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

/// A pulse under way as the editor sees it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shown {
    pub line: usize,
    pub from_a: bool,
    pub along: f32,
    /// Its pulsar's place.
    pub origin: usize,
}

impl PulsarStatus {
    /// The engine says where things are, at beat `now`.
    pub fn publish(&self, rt: &mut PulsarRt, now: f64) {
        let n = rt.flights.len().min(SHOWN_FLIGHTS);
        for (slot, f) in self.flights.iter().zip(rt.flights.iter()) {
            slot.store(pack(f.line, f.from_a, progress(f, now), f.carry.origin as usize), Ordering::Relaxed);
        }
        self.count.store(n as u32, Ordering::Relaxed);
        for (slot, bits) in self.struck.iter().zip(rt.take_struck()) {
            if bits != 0 {
                slot.fetch_or(bits, Ordering::Relaxed);
            }
        }
    }

    /// The pulses under way.
    pub fn read(&self) -> Vec<Shown> {
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
pub fn pack(line: usize, from_a: bool, along: f64, origin: usize) -> u64 {
    (line as u64 & 0xffff) | (from_a as u64) << 16 | (origin as u64 & 0xff) << 17 | ((along.clamp(0.0, 1.0) * 65535.0) as u64) << 32
}

pub fn unpack(v: u64) -> Shown {
    Shown { line: (v & 0xffff) as usize, from_a: v >> 16 & 1 == 1, origin: (v >> 17 & 0xff) as usize, along: ((v >> 32) & 0xffff) as f32 / 65535.0 }
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
        PNode { id, x, y: 0.0, kind: PKind::orb(Some(7), degree) }
    }

    fn pulsar(id: u64, x: f32, rate: u8) -> PNode {
        PNode { id, x, y: 0.0, kind: PKind::Pulsar { rate, on: true, mode: 0, chance: 1.0, strength: 1.0 } }
    }

    fn node(id: u64, x: f32, kind: PKind) -> PNode {
        PNode { id, x, y: 0.0, kind }
    }

    /// Runs the field over so many beats in blocks; the notes struck: (beat, pitch, velocity).
    fn run_with(set: &PulsarSet, beats: f64, rt: &mut PulsarRt) -> Vec<(f64, u8, f32)> {
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
                    ons.push((b0 + e.offset as f64 / spb, e.pitch, e.vel));
                }
            });
        }
        ons
    }

    fn run(set: &PulsarSet, beats: f64) -> Vec<(f64, u8, f32)> {
        run_with(set, beats, &mut PulsarRt::default())
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
        let s = field(vec![pulsar(1, 0.0, 3), node(2, 50.0, PKind::Gate { every: 2 }), orb(3, 100.0, 0)], &[(1, 2), (2, 3)]);
        let ons = run(&s, 8.0);
        assert!((3..=4).contains(&ons.len()), "half of eight: {ons:?}");
    }

    #[test]
    fn chance_lets_some_through() {
        let s = field(vec![pulsar(1, 0.0, 0), node(2, 50.0, PKind::Chance { chance: 0.5 }), orb(3, 100.0, 0)], &[(1, 2), (2, 3)]);
        let n = run(&s, 16.0).len();
        assert!(n > 15 && n < 50, "about half of 63: {n}");
    }

    #[test]
    fn a_reflector_sends_it_back() {
        // Pulsar - orb - reflector: the orb plays going out and coming back.
        let s = field(vec![pulsar(1, 0.0, 7), orb(2, 100.0, 0), node(3, 150.0, PKind::Reflector)], &[(1, 2), (2, 3)]);
        let ons = run(&s, 4.0);
        assert_eq!(ons.len(), 2, "{ons:?}");
        assert!((ons[1].0 - 2.0).abs() < 0.02, "half a beat there, half a beat back: {ons:?}");
    }

    #[test]
    fn a_delay_holds_a_pulse() {
        let s = field(vec![pulsar(1, 0.0, 7), node(2, 50.0, PKind::Delay { wait: 3 }), orb(3, 100.0, 0)], &[(1, 2), (2, 3)]);
        let ons = run(&s, 4.0);
        assert!((ons[0].0 - 2.0).abs() < 0.02, "a beat of lines and a beat held: {ons:?}");
    }

    #[test]
    fn a_router_sends_each_pulse_down_the_next_line() {
        let s = field(vec![pulsar(1, 0.0, 3), node(2, 50.0, PKind::Router), orb(3, 100.0, 0), orb(4, 100.0, 2), orb(5, 100.0, 4)], &[(1, 2), (2, 3), (2, 4), (2, 5)]);
        let notes: Vec<u8> = run(&s, 6.0).iter().map(|o| o.1).collect();
        assert_eq!(&notes[..4], [48, 52, 55, 48], "one at a time, in turn: {notes:?}");
    }

    #[test]
    fn a_shift_moves_what_a_pulse_plays_and_an_orb_plays_chords() {
        let s = field(vec![pulsar(1, 0.0, 7), node(2, 50.0, PKind::Shift { steps: 2 }), orb(3, 100.0, 0)], &[(1, 2), (2, 3)]);
        assert_eq!(run(&s, 4.0)[0].1, 52, "C3 two steps up the key");
        let mut s = field(vec![pulsar(1, 0.0, 7), orb(2, 100.0, 0)], &[(1, 2)]);
        if let PKind::Orb { chord, .. } = &mut s.nodes[1].kind {
            *chord = 2;
        }
        let mut notes: Vec<u8> = run(&s, 4.0).iter().map(|o| o.1).collect();
        notes.sort();
        assert_eq!(notes, [48, 52, 55], "a triad");
        if let PKind::Orb { note, .. } = &mut s.nodes[1].kind {
            *note = Some(38);
        }
        assert_eq!(run(&s, 4.0).iter().map(|o| o.1).collect::<Vec<_>>(), [38], "a fixed note: a drum");
    }

    #[test]
    fn a_pulsar_can_fire_by_chance_or_by_hand_and_as_hard_as_it_says() {
        let mut s = field(vec![pulsar(1, 0.0, 0), orb(2, 100.0, 0)], &[(1, 2)]);
        if let PKind::Pulsar { mode, chance, strength, .. } = &mut s.nodes[0].kind {
            *mode = 1;
            *chance = 0.5;
            *strength = 0.5;
        }
        let ons = run(&s, 16.0);
        assert!(ons.len() > 15 && ons.len() < 50, "{}", ons.len());
        assert!(ons.iter().all(|o| (o.2 - 0.4).abs() < 1e-6), "half as hard: {:?}", ons[0]);
        // By hand: nothing until it is clicked.
        if let PKind::Pulsar { mode, .. } = &mut s.nodes[0].kind {
            *mode = 2;
        }
        assert!(run(&s, 4.0).is_empty());
        let mut rt = PulsarRt::default();
        rt.fire(1);
        assert_eq!(run_with(&s, 4.0, &mut rt).len(), 1);
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
        let mut s = field(vec![pulsar(1, 0.0, 6), orb(2, 100.0, 0)], &[(1, 2)]);
        if let PKind::Orb { length, .. } = &mut s.nodes[1].kind {
            *length = 8.0;
        }
        let mut rt = PulsarRt::default();
        rt.resolve(&s);
        rt.run(&s, &c_major(), 0.0, 1.5, 1000.0, 1500, &mut |_| {});
        let mut offs = 0;
        rt.stop(&mut |(_, e): Out| offs += usize::from(!e.on));
        assert_eq!(offs, 1);
        assert!(rt.flights().is_empty());
    }

    #[test]
    fn the_editor_is_told_where_the_pulses_are() {
        let shown = unpack(pack(300, true, 0.25, 9));
        assert!(shown.line == 300 && shown.from_a && shown.origin == 9 && (shown.along - 0.25).abs() < 1e-4);
        let s = field(vec![pulsar(1, 0.0, 6), orb(2, 100.0, 0)], &[(1, 2)]);
        let mut rt = PulsarRt::default();
        rt.resolve(&s);
        rt.run(&s, &c_major(), 0.0, 0.5, 1000.0, 500, &mut |_| {});
        let st = PulsarStatus::default();
        st.publish(&mut rt, 0.5);
        let shown = st.read();
        assert_eq!(shown.len(), 1);
        assert!((shown[0].along - 0.5).abs() < 1e-3, "half way down a line of a beat");
        assert_eq!(shown[0].origin, 0, "from the pulsar in place 0");
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
        // A field from before chords and modes still loads.
        let old = r#"{"nodes":[{"id":1,"x":0.0,"y":0.0,"kind":{"Pulsar":{"rate":3,"on":true}}},{"id":2,"x":1.0,"y":0.0,"kind":{"Orb":{"track":7,"degree":0,"octave":0,"velocity":0.8,"length":0.25}}}],"links":[]}"#;
        let old: PulsarSet = serde_json::from_str(old).unwrap();
        assert_eq!(old.nodes[0].kind, PKind::Pulsar { rate: 3, on: true, mode: 0, chance: 1.0, strength: 1.0 });
        s.remove(2);
        assert!(s.links.is_empty());
    }
}

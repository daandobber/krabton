//! Grid controller logic, shared by a real monome grid (via serialosc) and
//! the on-screen virtual grid. Three modes:
//!
//! - Session: columns are tracks, rows are scenes; press to launch, the
//!   bottom row stops tracks, the right column launches scenes.
//! - Keys: an in-key isomorphic keyboard (rows a fourth apart in scale steps).
//! - Sequencer: edits the selected track's first sequencer on the grid.
//!
//! A monome arc controls the first four parameters of the selected track's
//! first MIDI device (or volume, pan and sends when there is none).

use crate::app::{App, TrackSel};
use krabton_engine::midi_fx::MidiFxKind;
use krabton_engine::sequencers::{self, LANES, LANE_BYTES, LANE_MAX, LANE_MIN};
use krabton_engine::{Command, TrackKind};
use krabton_monome::{Event, Monome};
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

/// Acid notes reachable from the grid rows 0..5 (top to bottom).
const ACID_GRID_NOTES: [u8; 6] = [12, 10, 7, 5, 3, 0];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GridMode {
    Session,
    Keys,
    Sequencer,
}

impl GridMode {
    pub const ALL: [GridMode; 3] = [GridMode::Session, GridMode::Keys, GridMode::Sequencer];

    pub fn name(self) -> &'static str {
        match self {
            GridMode::Session => "Session",
            GridMode::Keys => "Keys",
            GridMode::Sequencer => "Sequencer",
        }
    }
}

pub struct GridState {
    pub monome: Option<Monome>,
    pub error: Option<String>,
    pub mode: GridMode,
    pub width: u32,
    pub height: u32,
    pub leds: Vec<u8>,
    pub track_offset: usize,
    pub scene_offset: usize,
    pub poly_lane: usize,
    held_notes: HashMap<(u32, u32), (u8, Vec<u64>)>,
    last_send: Instant,
    pub virtual_down: Option<(u32, u32)>,
}

impl GridState {
    pub fn new() -> Self {
        // KRABTON_SERIALOSC_PORT points at another serialosc (for tests).
        let port = std::env::var("KRABTON_SERIALOSC_PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(krabton_monome::SERIALOSC_PORT);
        // Grids serialosc does not see are opened on their serial port, except
        // in test windows (they must never take over a real grid), unless
        // KRABTON_SERIAL_GRID=1 asks for it.
        let test = std::env::var_os("KRABTON_SERIALOSC_PORT").is_some() || std::env::var_os("KRABTON_TEST_WINDOW").is_some();
        let serial = std::env::var("KRABTON_SERIAL_GRID").map(|v| v == "1").unwrap_or(!test);
        let (monome, error) = match Monome::with_serialosc_port(port) {
            Ok(mut m) => {
                m.set_serial(serial);
                m.discover();
                (Some(m), None)
            }
            Err(e) => (None, Some(format!("UDP socket: {e}"))),
        };
        Self {
            monome,
            error,
            mode: GridMode::Session,
            width: 16,
            height: 8,
            leds: vec![0; 128],
            track_offset: 0,
            scene_offset: 0,
            poly_lane: 0,
            held_notes: HashMap::new(),
            last_send: Instant::now(),
            virtual_down: None,
        }
    }

    fn set(&mut self, x: u32, y: u32, level: u8) {
        if x < self.width && y < self.height {
            let i = (y * self.width + x) as usize;
            self.leds[i] = self.leds[i].max(level.min(15));
        }
    }
}

fn blink() -> bool {
    crate::ui::session::blink()
}

impl App {
    /// The selected track and its first sequencer device, if any.
    pub(crate) fn grid_sequencer(&self) -> Option<(usize, usize, MidiFxKind)> {
        let TrackSel::Track(t) = self.sel_track else { return None };
        let track = self.project.tracks.get(t)?;
        let d = track.midi_devices.iter().position(|m| m.kind.is_sequencer())?;
        Some((t, d, track.midi_devices[d].kind))
    }

    /// Polls hardware, handles its events, redraws LEDs and sends them.
    pub(crate) fn grid_frame(&mut self) {
        let events = self.grid.monome.as_mut().map(|m| m.poll()).unwrap_or_default();
        for e in events {
            match e {
                Event::Key { x, y, down } => self.grid_key(x, y, down),
                Event::GridConnected { id, width, height } => {
                    self.grid.width = width.clamp(8, 16);
                    self.grid.height = height.clamp(8, 16);
                    self.toast(format!("monome grid {id} connected ({width}x{height})"));
                }
                Event::ArcConnected { id } => self.toast(format!("monome arc {id} connected")),
                Event::Disconnected { id } => self.toast(format!("monome {id} disconnected")),
                Event::EncDelta { n, delta } => self.arc_delta(n as usize, delta),
                Event::EncKey { .. } | Event::Devices => {}
            }
        }
        self.grid_render();
        if self.grid.last_send.elapsed() >= Duration::from_millis(33) {
            self.grid.last_send = Instant::now();
            let (w, h) = (self.grid.width, self.grid.height);
            let leds = self.grid.leds.clone();
            let rings = self.arc_rings();
            if let Some(m) = self.grid.monome.as_mut() {
                m.grid_leds(&leds, w, h);
                for (n, ring) in rings.iter().enumerate() {
                    m.ring_leds(n as u32, ring);
                }
            }
        }
    }

    /// A key press or release, from hardware or the virtual grid.
    pub(crate) fn grid_key(&mut self, x: u32, y: u32, down: bool) {
        let (w, h) = (self.grid.width, self.grid.height);
        if x >= w || y >= h {
            return;
        }
        match self.grid.mode {
            GridMode::Session => {
                if !down {
                    return;
                }
                let scene_col = w >= 16;
                let track_cols = if scene_col { w - 1 } else { w };
                if scene_col && x == w - 1 {
                    if y == h - 1 {
                        self.cmd(Command::StopAllClips);
                    } else {
                        let s = self.grid.scene_offset + y as usize;
                        if s < self.project.scenes.len() {
                            self.cmd(Command::LaunchScene(s));
                            self.sel_scene = s;
                        }
                    }
                    return;
                }
                if x >= track_cols {
                    return;
                }
                let t = self.grid.track_offset + x as usize;
                let Some(track) = self.project.tracks.get(t) else { return };
                let id = track.id;
                if y == h - 1 {
                    self.cmd(Command::StopTrack { track: id });
                    return;
                }
                let s = self.grid.scene_offset + y as usize;
                match track.slots.get(s) {
                    Some(Some(_)) => {
                        self.cmd(Command::LaunchClip { track: id, slot: s });
                        self.sel_track = TrackSel::Track(t);
                    }
                    Some(None) => self.cmd(Command::StopTrack { track: id }),
                    None => {}
                }
            }
            GridMode::Keys => {
                if down {
                    let pitch = self.grid_key_pitch(x, y);
                    let played = self.note_on(pitch, 0.85);
                    self.grid.held_notes.insert((x, y), played);
                } else if let Some((pitch, ids)) = self.grid.held_notes.remove(&(x, y)) {
                    self.note_off(pitch, &ids);
                }
            }
            GridMode::Sequencer => {
                if down {
                    self.grid_sequencer_key(x, y);
                }
            }
        }
    }

    fn grid_key_pitch(&self, x: u32, y: u32) -> u8 {
        let scale = self.project.scale;
        let base = (self.kb_octave + 1) * 12 + scale.root as i32;
        let (abs, _) = scale.locate(base);
        let degree = x as i32 + (self.grid.height - 1 - y) as i32 * 3;
        scale.pitch_at(abs + degree).clamp(0, 127) as u8
    }

    fn grid_sequencer_key(&mut self, x: u32, y: u32) {
        let Some((t, d, kind)) = self.grid_sequencer() else { return };
        let h = self.grid.height;
        let lane_sel = self.grid.poly_lane;
        // Where Bloom's loop is now, to stamp a dropped note with.
        let bloom_phase = self.status().tracks.get(t).map(|ts| (ts.seq[3].load(std::sync::atomic::Ordering::Relaxed) & 0xffff) as u16).unwrap_or(0);
        let dev = &mut self.project.tracks[t].midi_devices[d];
        match kind {
            MidiFxKind::Life => {
                if dev.data.len() < 16 {
                    dev.data.resize(16, 0);
                }
                if x < 16 && y < 8 {
                    let i = y as usize * 2 + (x as usize / 8);
                    dev.data[i] ^= 1 << (x % 8);
                }
            }
            MidiFxKind::Polymeter => {
                if dev.data.len() < LANES * LANE_BYTES {
                    dev.data = sequencers::polymeter_default();
                }
                if y == h - 1 {
                    if (x as usize) < LANES {
                        self.grid.poly_lane = x as usize;
                    }
                    return;
                }
                if x >= 16 {
                    return;
                }
                let o = lane_sel * LANE_BYTES + x as usize;
                if lane_sel == 0 {
                    dev.data[o] ^= 1;
                } else {
                    let rows = (h - 2) as f32;
                    let max = LANE_MAX[lane_sel] as f32;
                    let min = LANE_MIN[lane_sel] as f32;
                    let v = min + ((rows - y as f32) / rows * (max - min)).round();
                    dev.data[o] = v.clamp(min, max) as u8;
                }
            }
            MidiFxKind::Euclid => {
                if y >= 4 && y < 8 {
                    let r = (y - 4) as usize;
                    let pi = 2 + r * 4 + 1;
                    let pulses = x as f32 + 1.0;
                    dev.params[pi] = if (dev.params[pi] - pulses).abs() < 0.5 { 0.0 } else { pulses.min(16.0) };
                }
            }
            MidiFxKind::Turing => {
                if y == h - 2 {
                    dev.params[1] = (x as f32 + 1.0).clamp(2.0, 16.0);
                } else if y == h - 1 {
                    dev.params[2] = x as f32 / (self.grid.width - 1) as f32;
                }
            }
            MidiFxKind::Bounce => {
                let i = (x / 2) as usize;
                if y == h - 1 {
                    dev.params[0] = (x as f32 + 1.0).clamp(1.0, 8.0);
                } else if i < sequencers::MAX_BALLS {
                    // Even columns: how high the ball drops from; odd: its note.
                    let up = (h - 2 - y) as f32;
                    if x % 2 == 0 {
                        dev.params[16 + i] = (up * 5.0 / (h - 2) as f32).round().clamp(0.0, 5.0);
                    } else {
                        dev.params[8 + i] = up;
                    }
                    dev.params[0] = dev.params[0].max(i as f32 + 1.0);
                }
            }
            MidiFxKind::Orbits => {
                if (y as usize) < 8 {
                    dev.params[5 + y as usize] = (x as f32 + 1.0).clamp(1.0, 16.0);
                    dev.params[0] = dev.params[0].max(y as f32 + 1.0);
                }
            }
            MidiFxKind::Chaos => {
                dev.params[1] = 2.8 + x as f32 / (self.grid.width - 1) as f32 * 1.2;
            }
            MidiFxKind::GuitarChords => {
                use krabton_engine::sequencers::strum as gs;
                if dev.data.len() < gs::DATA_BYTES {
                    dev.data = gs::strum_default();
                }
                if y == h - 1 && (x as usize) < gs::PATTERN_STEPS {
                    let o = krabton_engine::classic_seq::PROG_SLOTS * krabton_engine::classic_seq::PROG_BYTES + x as usize;
                    dev.data[o] = match dev.data[o] {
                        gs::REST => gs::DOWN,
                        gs::DOWN => gs::UP,
                        _ => gs::REST,
                    };
                } else if y == h - 2 && (x as usize) < krabton_engine::classic_seq::PROG_SLOTS {
                    dev.params[gs::CHORDS] = x as f32 + 1.0;
                }
            }
            MidiFxKind::Bloom => {
                // Drop a note: the higher the row, the higher the note. In
                // Loop, pressing a lit note's pad takes it away.
                use krabton_engine::sequencers::ambient::{bloom_add_tap, bloom_remove_tap, bloom_taps, TAPS};
                let w = self.grid.width;
                let looping = dev.params.get(TAPS).is_some_and(|v| *v >= 0.5);
                let cell = |t: &krabton_engine::sequencers::ambient::Tap| ((t.across as u32 * (w - 1) + 127) / 255, (h - 1) - (t.height as u32 * (h - 1) + 127) / 255);
                let hit = bloom_taps(&dev.data).position(|t| looping && cell(&t) == (x, y));
                if let Some(i) = hit {
                    bloom_remove_tap(&mut dev.data, i);
                } else {
                    let up = ((h - 1 - y.min(h - 1)) as f32 / (h - 1).max(1) as f32 * 255.0).round() as u8;
                    let across = (x as f32 / (w - 1).max(1) as f32 * 255.0).round() as u8;
                    bloom_add_tap(&mut dev.data, up, across, bloom_phase);
                }
            }
            MidiFxKind::Automaton => {
                if y == h - 1 && x < 8 {
                    let rule = dev.params[1].round().clamp(0.0, 255.0) as u32 ^ (1 << (7 - x));
                    dev.params[1] = rule as f32;
                }
            }
            MidiFxKind::Markov => {
                if y == h - 1 && x < 16 {
                    dev.params[1 + (x / 4) as usize] = ((x % 4) + 1) as f32 / 4.0;
                }
            }
            MidiFxKind::Drunk => {
                if y == h - 1 {
                    dev.params[3] = x as f32 / (self.grid.width - 1) as f32;
                }
            }
            MidiFxKind::Step => {
                use krabton_engine::classic_seq::STEP_BYTES;
                if dev.data.len() < 16 * STEP_BYTES {
                    dev.data = krabton_engine::classic_seq::step_default();
                }
                if x >= 16 {
                    return;
                }
                let o = x as usize * STEP_BYTES;
                if y == h - 1 {
                    dev.data[o] ^= 1;
                } else {
                    // Rows are scale degrees, top = highest.
                    let degree = (h - 2 - y) as u8;
                    if dev.data[o] != 0 && dev.data[o + 1] == degree {
                        dev.data[o] = 0;
                    } else {
                        dev.data[o] = 1;
                        dev.data[o + 1] = degree;
                    }
                }
            }
            MidiFxKind::Acid => {
                use krabton_engine::classic_seq::ACID_BYTES;
                if dev.data.len() < 16 * ACID_BYTES {
                    dev.data = krabton_engine::classic_seq::acid_default();
                }
                if x >= 16 {
                    return;
                }
                let o = x as usize * ACID_BYTES;
                match y {
                    0..=5 => {
                        let semi = ACID_GRID_NOTES[y as usize];
                        if dev.data[o] == 1 && dev.data[o + 1] == semi {
                            dev.data[o] = 0;
                        } else {
                            dev.data[o] = 1;
                            dev.data[o + 1] = semi;
                        }
                    }
                    6 => dev.data[o + 3] ^= 1,
                    7 => dev.data[o + 4] ^= 1,
                    _ => {}
                }
            }
            MidiFxKind::DrumGrid => {
                if dev.data.len() < krabton_engine::classic_seq::DRUM_BYTES {
                    dev.data = krabton_engine::classic_seq::drum_default();
                }
                if x < 16 && (y as usize) < krabton_engine::classic_seq::DRUM_LANES {
                    let o = y as usize * 16 + x as usize;
                    dev.data[o] = (dev.data[o] + 1) % 3;
                }
            }
            MidiFxKind::Chance => {
                use krabton_engine::sequencers::chance as ch;
                if (x as usize) < ch::STEPS && (y as usize) < ch::LANES {
                    let s = ch::step(&dev.data, y as usize, x as usize);
                    ch::set_step(&mut dev.data, y as usize, x as usize, ch::Step { level: (s.level + 1) % 3, ..s });
                }
            }
            MidiFxKind::Bits => {
                use krabton_engine::sequencers::bits as bt;
                // Four columns per drum: a row gives it that bit (bit 7 on top).
                let lane = (x / 4) as usize;
                if lane < bt::LANES && y < 8 {
                    dev.params[bt::BIT + lane] = (7 - y) as f32;
                }
            }
            MidiFxKind::Percussion => {
                use krabton_engine::sequencers::percussion as pc;
                let st = pc::style(&dev.params);
                let row = y as usize;
                if row == st.parts.len() {
                    dev.params[pc::LEAD] = if dev.params[pc::LEAD] >= 0.5 { 0.0 } else { 1.0 };
                } else if row < st.parts.len() && st.parts.len() > 1 {
                    dev.params[pc::DENSITY] = row as f32 / (st.parts.len() - 1) as f32;
                }
            }
            MidiFxKind::DrumMap => {
                use krabton_engine::sequencers::drum_map as dm;
                // A 5x5 map on the lower left; fills and chaos as faders on the right.
                if x < 5 && (3..8).contains(&y) {
                    dev.params[dm::MAP_X] = x as f32 / 4.0;
                    dev.params[dm::MAP_Y] = (7 - y) as f32 / 4.0;
                } else if (6..16).contains(&x) && (4..8).contains(&y) {
                    let i = if y == 7 { dm::CHAOS } else { dm::FILL + (y - 4) as usize };
                    dev.params[i] = (x - 6) as f32 / 9.0;
                }
            }
            MidiFxKind::Marbles => {
                use krabton_engine::sequencers::marbles as mb;
                let unit = x.min(15) as f32 / 15.0;
                match y {
                    4 => dev.params[mb::DEJA_VU] = unit,
                    5 => dev.params[mb::LENGTH] = (x + 1).min(16) as f32,
                    6 => dev.params[mb::BIAS] = unit,
                    7 if (x as usize) < mb::MODELS.len() => dev.params[mb::MODEL] = x as f32,
                    7 if (9..16).contains(&x) => dev.params[mb::JITTER] = (x - 9) as f32 / 6.0,
                    _ => {}
                }
            }
            MidiFxKind::Progression => {
                use krabton_engine::classic_seq::{PROG_BYTES, PROG_SLOTS, QUALITIES};
                if dev.data.len() < PROG_SLOTS * PROG_BYTES {
                    dev.data = krabton_engine::classic_seq::progression_default();
                }
                if (x as usize) < PROG_SLOTS {
                    let o = x as usize * PROG_BYTES;
                    if y == h - 1 {
                        dev.data[o + 1] = (dev.data[o + 1] + 1) % QUALITIES.len() as u8;
                    } else if y < 7 {
                        // Rows are degrees VII (top) to I.
                        let degree = 7 - y as u8;
                        dev.data[o] = if dev.data[o] == degree { 0 } else { degree };
                    }
                }
            }
            _ => {}
        }
    }

    /// Recomputes `self.grid.leds` for the current mode.
    pub(crate) fn grid_render(&mut self) {
        let (w, h) = (self.grid.width, self.grid.height);
        self.grid.leds = vec![0; (w * h) as usize];
        let status = self.status();
        let playing = status.playing.load(Ordering::Relaxed);
        match self.grid.mode {
            GridMode::Session => {
                let scene_col = w >= 16;
                let track_cols = if scene_col { w - 1 } else { w };
                for gx in 0..track_cols {
                    let t = self.grid.track_offset + gx as usize;
                    let Some(track) = self.project.tracks.get(t) else { continue };
                    let ts = &status.tracks[t.min(status.tracks.len() - 1)];
                    let playing_slot = ts.playing_slot.load(Ordering::Relaxed);
                    let queued = ts.queued_slot.load(Ordering::Relaxed);
                    for gy in 0..h - 1 {
                        let s = self.grid.scene_offset + gy as usize;
                        let level = match track.slots.get(s) {
                            Some(Some(_)) if playing_slot == s as i32 => 15,
                            Some(Some(_)) if queued == s as i32 => {
                                if blink() {
                                    15
                                } else {
                                    5
                                }
                            }
                            Some(Some(_)) => 5,
                            Some(None) if track.arm && track.kind == TrackKind::Midi => 1,
                            _ => 0,
                        };
                        self.grid.set(gx, gy, level);
                    }
                    let stop = if playing_slot >= 0 { 7 } else { 2 };
                    self.grid.set(gx, h - 1, stop);
                }
                if scene_col {
                    for gy in 0..h - 1 {
                        let s = self.grid.scene_offset + gy as usize;
                        if s < self.project.scenes.len() {
                            let any = self.project.tracks.iter().any(|t| matches!(t.slots.get(s), Some(Some(_))));
                            self.grid.set(w - 1, gy, if any { 8 } else { 2 });
                        }
                    }
                    self.grid.set(w - 1, h - 1, 4);
                }
            }
            GridMode::Keys => {
                let scale = self.project.scale;
                let held: Vec<u8> = self.grid.held_notes.values().map(|v| v.0).collect();
                for y in 0..h {
                    for x in 0..w {
                        let p = self.grid_key_pitch(x, y);
                        let level = if held.contains(&p) {
                            15
                        } else if scale.is_root(p) {
                            10
                        } else {
                            3
                        };
                        self.grid.set(x, y, level);
                    }
                }
            }
            GridMode::Sequencer => {
                let Some((t, d, kind)) = self.grid_sequencer() else {
                    // No sequencer on the selected track: a dim frame.
                    for x in 0..w {
                        self.grid.set(x, 0, 2);
                        self.grid.set(x, h - 1, 2);
                    }
                    return;
                };
                let ts = &status.tracks[t.min(status.tracks.len() - 1)];
                let vis: [u64; 4] = std::array::from_fn(|i| ts.seq[i].load(Ordering::Relaxed));
                let first_seq = self.project.tracks[t].midi_devices.iter().position(|m| m.kind.is_sequencer()) == Some(d);
                let dev = self.project.tracks[t].midi_devices[d].clone();
                match kind {
                    MidiFxKind::Life => {
                        let seed = sequencers::life_rows(&dev.data);
                        let live = if playing && first_seq { sequencers::life_from_visual(&vis) } else { seed };
                        let col = (vis[2] & 0xff) as u32;
                        for y in 0..8.min(h) {
                            for x in 0..16.min(w) {
                                if playing && x == col {
                                    self.grid.set(x, y, 2);
                                }
                                if (live[y as usize] >> x) & 1 == 1 {
                                    self.grid.set(x, y, 13);
                                } else if (seed[y as usize] >> x) & 1 == 1 {
                                    self.grid.set(x, y, 4);
                                }
                            }
                        }
                    }
                    MidiFxKind::Polymeter => {
                        let lane = self.grid.poly_lane;
                        let (vals, len, _) = sequencers::lane(&dev.data, lane);
                        let pos = ((vis[0] >> (lane * 8)) & 0xff) as u32;
                        let rows = h - 2;
                        for x in 0..16.min(w) {
                            if x as usize >= len {
                                continue;
                            }
                            if playing && x == pos {
                                for y in 0..=rows {
                                    self.grid.set(x, y, 2);
                                }
                            }
                            if lane == 0 {
                                if vals[x as usize] == 1 {
                                    self.grid.set(x, rows, 12);
                                }
                            } else {
                                let (min, max) = (LANE_MIN[lane] as f32, LANE_MAX[lane] as f32);
                                let v = vals[x as usize] as f32;
                                let top = rows - ((v - min) / (max - min) * rows as f32).round() as u32;
                                for y in top..=rows {
                                    self.grid.set(x, y, if y == top { 12 } else { 4 });
                                }
                            }
                        }
                        for l in 0..LANES as u32 {
                            self.grid.set(l, h - 1, if l as usize == lane { 15 } else { 4 });
                        }
                    }
                    MidiFxKind::Euclid => {
                        for r in 0..4u32 {
                            let b = 2 + r as usize * 4;
                            let steps = dev.params[b].round().max(1.0) as u32;
                            let pulses = dev.params[b + 1].round() as u32;
                            let pos = ((vis[0] >> (r * 8)) & 0xff) as u32;
                            for x in 0..steps.min(w) {
                                let hit = sequencers::euclid_hit(x as usize, steps as usize, pulses as usize, dev.params[b + 2].round() as usize);
                                let level = if playing && x == pos { 15 } else if hit { 9 } else { 2 };
                                self.grid.set(x, r, level);
                            }
                            if h >= 8 {
                                for x in 0..pulses.min(w) {
                                    self.grid.set(x, 4 + r, 5);
                                }
                            }
                        }
                    }
                    MidiFxKind::Turing => {
                        let reg = vis[0] & 0xffff;
                        let len = dev.params[1].round() as u32;
                        for x in 0..len.min(w) {
                            self.grid.set(x, 0, if (reg >> (len - 1 - x)) & 1 == 1 { 12 } else { 2 });
                        }
                        for x in 0..len.min(w) {
                            self.grid.set(x, h - 2, 5);
                        }
                        let change = (dev.params[2] * (w - 1) as f32).round() as u32;
                        for x in 0..=change.min(w - 1) {
                            self.grid.set(x, h - 1, 5);
                        }
                    }
                    MidiFxKind::Bounce => {
                        let n = (dev.params[0].round() as u32).clamp(1, 8);
                        let rows = (h - 2) as f32;
                        for i in 0..n {
                            let x = i * 2;
                            let drop = dev.params[16 + i as usize].round().clamp(0.0, 5.0);
                            let top = (rows * drop / 5.0).round().max(1.0);
                            let y = if playing && first_seq { ((vis[0] >> (i * 8)) & 0xff) as f32 / 255.0 } else { 1.0 };
                            self.grid.set(x, h - 2 - top as u32, 3);
                            self.grid.set(x, (h - 2) - (y * top).round() as u32, 15);
                            let note = dev.params[8 + i as usize].round();
                            if (0.0..=rows).contains(&note) {
                                self.grid.set(x + 1, h - 2 - note as u32, 5);
                            }
                        }
                        for x in 0..n.min(w) {
                            self.grid.set(x, h - 1, 3);
                        }
                    }
                    MidiFxKind::Orbits => {
                        let n = (dev.params[0].round() as u32).clamp(1, 8);
                        for i in 0..n.min(h) {
                            let speed = dev.params[5 + i as usize].round() as u32;
                            self.grid.set(speed.saturating_sub(1).min(w - 1), i, 4);
                            if playing && first_seq {
                                let phase = ((vis[0] >> (i * 8)) & 0xff) as u32;
                                self.grid.set(phase * w / 256, i, if phase < 20 { 15 } else { 10 });
                            }
                        }
                    }
                    MidiFxKind::Chaos => {
                        // The bifurcation diagram, with the setting as a column.
                        let r_now = dev.params[1];
                        let col = (((r_now - 2.8) / 1.2) * (w - 1) as f32).round() as u32;
                        for x in 0..w {
                            let r = 2.8 + x as f32 / (w - 1) as f32 * 1.2;
                            let mut v = 0.5_f32;
                            for _ in 0..100 {
                                v = r * v * (1.0 - v);
                            }
                            for _ in 0..24 {
                                v = r * v * (1.0 - v);
                                let gy = ((1.0 - v) * (h - 1) as f32).round() as u32;
                                self.grid.set(x, gy, if x == col { 7 } else { 2 });
                            }
                        }
                        if playing && first_seq {
                            let v = (vis[0] & 0xffff) as f32 / 65535.0;
                            self.grid.set(col, ((1.0 - v) * (h - 1) as f32).round() as u32, 15);
                        }
                    }
                    MidiFxKind::Automaton => {
                        let rows = (h - 1) as usize;
                        if playing && first_seq {
                            for k in 0..rows.min(16) {
                                let r = 16 - rows.min(16) + k;
                                let row = (vis[r / 4] >> ((r % 4) * 16)) & 0xffff;
                                for i in 0..16u32 {
                                    if (row >> i) & 1 == 1 {
                                        self.grid.set(15 - i, k as u32, if k + 1 == rows { 15 } else { 4 + (k * 6 / rows) as u8 });
                                    }
                                }
                            }
                        }
                        let rule = dev.params[1].round().clamp(0.0, 255.0) as u32;
                        for x in 0..8 {
                            self.grid.set(x, h - 1, if (rule >> (7 - x)) & 1 == 1 { 12 } else { 2 });
                        }
                    }
                    MidiFxKind::Ant => {
                        if playing && first_seq {
                            for y in 0..8.min(h) {
                                for x in 0..8 {
                                    if (vis[0] >> (y * 8 + x)) & 1 == 1 {
                                        self.grid.set(x, y, 5);
                                    }
                                }
                            }
                            let n = ((vis[1] >> 32) & 0xff).clamp(1, 3);
                            for i in 0..n {
                                let b = (vis[1] >> (i * 8)) as u32;
                                self.grid.set(b & 7, (b >> 3) & 7, 15);
                            }
                        }
                    }
                    MidiFxKind::Pong => {
                        if playing && first_seq {
                            let n = (vis[1] as u32).clamp(1, 2);
                            for i in 0..n {
                                let x = ((vis[0] >> (i * 16)) & 0xff) as u32;
                                let y = ((vis[0] >> (i * 16 + 8)) & 0xff) as u32;
                                self.grid.set(x * (w - 1) / 255, (255 - y) * (h - 1) / 255, 15);
                            }
                        }
                    }
                    MidiFxKind::Markov | MidiFxKind::Lorenz => {
                        if playing && first_seq {
                            let (x, y) = if kind == MidiFxKind::Markov {
                                let pos = (vis[0] & 0xff) as u32;
                                let range = ((vis[0] >> 8) & 0xff).max(1) as u32;
                                (pos * (w - 1) / range, (h - 1) / 2)
                            } else {
                                let xi = (vis[0] & 0xffff) as u32;
                                let zi = ((vis[0] >> 16) & 0xffff) as u32;
                                (xi * (w - 1) / 65535, (65535 - zi) * (h - 2) / 65535)
                            };
                            for gy in 0..h - 1 {
                                self.grid.set(x, gy, if gy == y { 15 } else if kind == MidiFxKind::Markov { 3 } else { 0 });
                            }
                        }
                        if kind == MidiFxKind::Markov {
                            for g in 0..4u32 {
                                let v = (dev.params[1 + g as usize] * 4.0).round().clamp(0.0, 4.0) as u32;
                                for k in 0..v {
                                    self.grid.set(g * 4 + k, h - 1, 3 + g as u8 * 2);
                                }
                            }
                        }
                    }
                    MidiFxKind::Rain => {
                        if playing && first_seq {
                            let range = vis[1].max(2) as u32;
                            for k in 0..8u32.min(h) {
                                let d = ((vis[0] >> (k * 8)) & 0xff) as u32;
                                self.grid.set(d.min(range) * (w - 1) / range, k, 15u8.saturating_sub(k as u8 * 2));
                            }
                        }
                    }
                    MidiFxKind::Words => {
                        if playing && first_seq && !dev.data.is_empty() {
                            let i = vis[0] as usize;
                            if let Some((d, _)) = dev.data.get(i).and_then(|&b| sequencers::more::letter_degree(b)) {
                                self.grid.set((i as u32) % w, (h - 2) - (d as u32 % (h - 1)), 15);
                            }
                            let done = (i as u32 + 1) * w / dev.data.len() as u32;
                            for x in 0..done.min(w) {
                                self.grid.set(x, h - 1, 3);
                            }
                        }
                    }
                    MidiFxKind::Loops => {
                        // A row per loop, its tape running left to right; bright as it plays.
                        let n = (dev.params[0].round() as u32).clamp(1, 7);
                        for i in 0..n.min(h) {
                            let len = dev.params[6 + i as usize].round() as u32;
                            self.grid.set((len * w / 65).min(w - 1), i, 2);
                            if playing && first_seq {
                                let phase = ((vis[0] >> (i * 8)) & 0xff) as u32;
                                self.grid.set(phase * w / 256, i, if phase < 12 { 15 } else { 7 });
                            }
                        }
                    }
                    MidiFxKind::GuitarChords => {
                        use krabton_engine::sequencers::strum as gs;
                        // The chord on the neck: high string on top, frets across.
                        let shape = if playing && first_seq && vis[1] != u64::MAX { Some(gs::unpack_shape(vis[1])) } else { None };
                        if let Some(shape) = shape {
                            for (s, f) in shape.iter().enumerate() {
                                if let Some(f) = f {
                                    self.grid.set((*f as u32).min(w - 1), 5 - s as u32, if *f == 0 { 6 } else { 15 });
                                }
                            }
                        }
                        let chords = dev.params[gs::CHORDS].round().clamp(1.0, 8.0) as u32;
                        let slot = (vis[0] & 0xff) as u32;
                        let step = ((vis[0] >> 8) & 0xff) as u32;
                        for x in 0..chords.min(w) {
                            self.grid.set(x, h - 2, if playing && first_seq && x == slot { 15 } else { 4 });
                        }
                        for x in 0..(gs::PATTERN_STEPS as u32).min(w) {
                            let s = gs::pattern_at(&dev.data, x as usize);
                            let b = if playing && first_seq && x == step { 15 } else if s == gs::REST { 0 } else if s == gs::UP || s == gs::SOFT_UP { 5 } else { 9 };
                            self.grid.set(x, h - 1, b);
                        }
                    }
                    MidiFxKind::Bloom => {
                        // Your looping notes, dim; the last notes played where
                        // they fell, bright. Higher rows are higher notes.
                        if dev.params.get(krabton_engine::sequencers::ambient::TAPS).is_some_and(|v| *v >= 0.5) {
                            for tp in krabton_engine::sequencers::ambient::bloom_taps(&dev.data) {
                                self.grid.set((tp.across as u32 * (w - 1) + 127) / 255, (h - 1) - (tp.height as u32 * (h - 1) + 127) / 255, 4);
                            }
                        }
                        if first_seq {
                            let range = ((vis[1] >> 8) & 0xff).max(2) as i32;
                            for k in (0..8u32).rev() {
                                if (vis[0] >> (k * 8)) & 0xff != 0 {
                                    let d = ((vis[0] >> (k * 8)) & 0xff) as i32 - 64;
                                    let y = (h - 1) - (d.clamp(0, range) as u32 * (h - 1)) / range as u32;
                                    let x = (((vis[2] >> (k * 8)) & 0xff) as u32 * (w - 1)) / 255;
                                    self.grid.set(x, y, 15u8.saturating_sub(k as u8 * 2));
                                }
                            }
                        }
                    }
                    MidiFxKind::Chimes => {
                        // The tubes hanging from the top as long as they are, lit
                        // while they ring; the wind on the bottom row.
                        let shape = krabton_engine::sequencers::ambient::chime_shape(&self.project.scale, &dev.params);
                        let n = shape.tubes as u32;
                        for j in 0..n {
                            let x = j * (w - 1) / (n - 1).max(1);
                            let len = ((shape.lengths[j as usize] * (h - 2) as f32).round() as u32).clamp(1, h - 1);
                            let ring = if playing && first_seq { ((vis[3] >> (8 * j)) & 0xff) as f32 / 255.0 } else { 0.0 };
                            for y in 0..len {
                                self.grid.set(x, y, (3.0 + 12.0 * ring).round() as u8);
                            }
                        }
                        let wind = if playing && first_seq { ((vis[0] >> 32) & 0xff) as f32 / 255.0 * 1.5 } else { dev.params[0] };
                        for x in 0..((wind * w as f32).round() as u32).min(w) {
                            self.grid.set(x, h - 1, 4);
                        }
                    }
                    MidiFxKind::DriftChords => {
                        // A row per voice, from the bottom: where in the range it sits now.
                        if playing && first_seq {
                            let n = (vis[1] & 0xff) as u32;
                            let range = ((vis[1] >> 8) & 0xff).max(1) as i32;
                            for i in 0..n.min(h) {
                                let d = ((vis[0] >> (i * 8)) & 0xff) as i32 - 64;
                                self.grid.set((d.clamp(0, range) as u32 * (w - 1)) / range as u32, h - 1 - i, 15 - i as u8);
                            }
                        }
                    }
                    MidiFxKind::Tintinnabuli => {
                        // The melody (bright) and its triad voice (dim), a column per step.
                        if playing && first_seq {
                            let m = (vis[0] & 0xff) as i32 - 64;
                            let t = ((vis[0] >> 8) & 0xff) as i32 - 64;
                            let x = (vis[1] as u32) % w;
                            let y = |d: i32| ((h as i32 - 2) - (d + 4).clamp(0, h as i32 * 2 - 3) / 2) as u32;
                            self.grid.set(x, y(t), 6);
                            self.grid.set(x, y(m), 15);
                            if (vis[0] >> 16) & 1 == 1 {
                                for x in 0..w {
                                    self.grid.set(x, h - 1, 3);
                                }
                            }
                        }
                    }
                    MidiFxKind::Pedal => {
                        // The held note as a block: the tonic in the middle, moves to either side.
                        if playing && first_seq {
                            let root = (vis[0] & 0xff) as i32 - 64;
                            let x = (((root + 7).clamp(0, 14) as u32) * (w - 2)) / 14;
                            for y in 2..h - 2 {
                                self.grid.set(x, y, 12);
                                self.grid.set(x + 1, y, 12);
                            }
                            let change = [1u32, 2, 4, 8, 0][(dev.params[0].round().max(0.0) as usize).min(4)];
                            for k in 0..change {
                                self.grid.set(k * 2, h - 1, if vis[1] as u32 % change == k { 15 } else { 3 });
                            }
                        }
                    }
                    MidiFxKind::Drunk => {
                        let pos = (vis[0] & 0xff) as u32;
                        let range = ((vis[0] >> 8) & 0xff).max(1) as u32;
                        let x = pos * (w - 1) / range;
                        for y in 0..h - 1 {
                            self.grid.set(x, y, if y == (h - 1) / 2 { 15 } else { 3 });
                        }
                        let rest = (dev.params[3] * (w - 1) as f32).round() as u32;
                        for x in 0..=rest.min(w - 1) {
                            self.grid.set(x, h - 1, 4);
                        }
                    }
                    MidiFxKind::Step => {
                        let steps = dev.params[1].round().clamp(1.0, 16.0) as u32;
                        let pos = (vis[0] & 0xff) as u32;
                        for x in 0..16.min(w) {
                            let s = krabton_engine::classic_seq::step_at(&dev.data, x as usize);
                            let dim = if x < steps { 1 } else { 0 };
                            if playing && x == pos {
                                for y in 0..h - 1 {
                                    self.grid.set(x, y, 2);
                                }
                            }
                            let row = (h - 2).saturating_sub((s.degree as u32).min(h - 2));
                            if s.on {
                                self.grid.set(x, row, if x < steps { 12 } else { 4 });
                            }
                            self.grid.set(x, h - 1, if s.on { 8 + dim * 2 } else { dim });
                        }
                    }
                    MidiFxKind::Acid => {
                        let pos = (vis[0] & 0xff) as u32;
                        for x in 0..16.min(w) {
                            let s = krabton_engine::classic_seq::acid_at(&dev.data, x as usize);
                            if playing && x == pos {
                                for y in 0..6 {
                                    self.grid.set(x, y, 2);
                                }
                            }
                            if s.gate == 1 {
                                // The nearest grid row for the note.
                                let row = ACID_GRID_NOTES.iter().enumerate().min_by_key(|(_, n)| (**n as i32 - s.semitone as i32).abs()).map(|(i, _)| i as u32).unwrap_or(5);
                                self.grid.set(x, row, if s.accent { 15 } else { 10 });
                            } else if s.gate == 2 {
                                self.grid.set(x, 5, 4);
                            }
                            if s.accent {
                                self.grid.set(x, 6, 9);
                            }
                            if s.slide {
                                self.grid.set(x, 7.min(h - 1), 9);
                            }
                        }
                    }
                    MidiFxKind::DrumGrid => {
                        let steps = dev.params[1].round().clamp(1.0, 16.0) as u32;
                        let pos = (vis[0] & 0xff) as u32;
                        for lane in 0..(krabton_engine::classic_seq::DRUM_LANES as u32).min(h) {
                            for x in 0..16.min(w) {
                                let level = match krabton_engine::classic_seq::drum_cell(&dev.data, lane as usize, x as usize) {
                                    2 => 15,
                                    1 => 8,
                                    _ if playing && x == pos => 3,
                                    _ if x % 4 == 0 && x < steps => 1,
                                    _ => 0,
                                };
                                self.grid.set(x, lane, if x < steps { level } else { level / 3 });
                            }
                        }
                    }
                    MidiFxKind::Chance => {
                        use krabton_engine::sequencers::chance as ch;
                        let live = playing && first_seq;
                        for lane in 0..ch::LANES.min(h as usize) {
                            let len = ch::length(&dev.data, lane);
                            let pos = ((vis[0] >> (4 * lane)) & 0xf) as usize;
                            for x in 0..(ch::STEPS as u32).min(w) {
                                let s = ch::step(&dev.data, lane, x as usize);
                                let mut level = match s.level {
                                    2 => 15,
                                    1 if s.chance < 100 || s.condition != 0 => 6,
                                    1 => 10,
                                    _ if live && x as usize == pos => 3,
                                    _ if x % 4 == 0 => 1,
                                    _ => 0,
                                };
                                if x as usize >= len {
                                    level /= 3;
                                }
                                self.grid.set(x, lane as u32, level);
                            }
                        }
                    }
                    MidiFxKind::Bits => {
                        use krabton_engine::sequencers::bits as bt;
                        let p = &dev.params;
                        let f = (p[bt::FORMULA].round().max(0.0) as usize).min(bt::FORMULAS.len() - 1);
                        let (a, b) = (p[bt::A].round() as u32, p[bt::B].round() as u32);
                        let t_now = if playing && first_seq { vis[0] as u32 } else { 0 };
                        let start = t_now - t_now % 16;
                        for x in 0..16.min(w) {
                            let t = start.wrapping_add(x);
                            let byte = bt::byte(f, t, a, b);
                            for bit in 0..8u32.min(h) {
                                let lane = (0..bt::LANES).find(|&l| p[bt::BIT + l].round() as u32 == bit);
                                let on = byte >> bit & 1 == 1;
                                let level = match (on, lane) {
                                    (true, Some(l)) if bt::hit(p, t, l).is_some() => 15,
                                    (true, Some(_)) => 7,
                                    (true, None) => 3,
                                    (false, Some(_)) => 1,
                                    _ if t == t_now && playing => 2,
                                    _ => 0,
                                };
                                self.grid.set(x, 7 - bit, level);
                            }
                        }
                    }
                    MidiFxKind::Percussion => {
                        use krabton_engine::sequencers::percussion as pc;
                        let st = pc::style(&dev.params);
                        let live = playing && first_seq;
                        let step = (vis[0] & 0xff) as usize;
                        let bar = if live { ((vis[0] >> 16) & 0xff) as usize } else { 0 };
                        let playing_parts = pc::parts_playing(st, dev.params[pc::DENSITY]);
                        let improvising = live && (vis[1] >> 8) & 1 == 1;
                        for row in 0..(st.parts.len() + 1).min(h as usize) {
                            let is_lead = row == st.parts.len();
                            let pat = if is_lead { st.lead.1 } else { st.parts[row].1 };
                            let on = if is_lead { dev.params[pc::LEAD] >= 0.5 } else { row < playing_parts };
                            for x in 0..(st.steps as u32).min(w) {
                                let i = x as usize;
                                let hit = if is_lead && improvising {
                                    vis[2] >> i & 1 == 1
                                } else if is_lead {
                                    pc::symbol(pat, i) != '.'
                                } else {
                                    pc::symbol(pat, (bar % 2) * st.steps + i) != '.'
                                };
                                let level = match (hit, on) {
                                    (true, true) if live && i == step => 15,
                                    (true, true) => 9,
                                    (true, false) => 2,
                                    _ if live && i == step => 3,
                                    _ => 0,
                                };
                                self.grid.set(x, row as u32, level);
                            }
                        }
                    }
                    MidiFxKind::DrumMap => {
                        use krabton_engine::sequencers::drum_map as dm;
                        let (mx, my) = (dev.params[dm::MAP_X], dev.params[dm::MAP_Y]);
                        let w8 = dm::weights(mx, my);
                        let live = playing && first_seq;
                        let step = (vis[0] & 0xff) as usize;
                        let bar = if live { step / 16 } else { 0 };
                        let hits: [u32; dm::PARTS] = if live {
                            [vis[1] as u32, (vis[1] >> 32) as u32, vis[2] as u32]
                        } else {
                            dm::pattern(&w8, &dev.params, &[[0.0; dm::STEPS]; dm::PARTS])
                        };
                        for part in 0..dm::PARTS.min(h as usize) {
                            for x in 0..16.min(w) {
                                let s = bar * 16 + x as usize;
                                let level = if hits[part] >> s & 1 == 1 {
                                    if live && s == step { 15 } else { 10 }
                                } else if live && s == step {
                                    4
                                } else if w8[part][s] > 0 {
                                    2
                                } else {
                                    0
                                };
                                self.grid.set(x, part as u32, level);
                            }
                        }
                        if h >= 8 {
                            let (nx, ny) = ((mx * 4.0).round() as u32, (my * 4.0).round() as u32);
                            for gy in 0..5u32 {
                                for gx in 0..5u32.min(w) {
                                    self.grid.set(gx, 7 - gy, if (gx, gy) == (nx, ny) { 15 } else { 3 });
                                }
                            }
                            for (row, i) in [(4u32, dm::FILL), (5, dm::FILL + 1), (6, dm::FILL + 2), (7, dm::CHAOS)] {
                                let v = (dev.params[i].clamp(0.0, 1.0) * 9.0).round() as u32;
                                for x in 6..16.min(w) {
                                    let k = x - 6;
                                    self.grid.set(x, row, if k == v { 12 } else if k < v { 5 } else { 1 });
                                }
                            }
                        }
                    }
                    MidiFxKind::Marbles => {
                        use krabton_engine::sequencers::marbles as mb;
                        let live = playing && first_seq;
                        for c in 0..3u32.min(h) {
                            let hist = if live { (vis[0] >> (16 * c)) as u16 } else { 0 };
                            for x in 0..16.min(w) {
                                let hit = hist >> (15 - x) & 1 == 1;
                                self.grid.set(x, c, if hit { if x == 15 { 15 } else { 10 } } else { 1 });
                            }
                        }
                        if h >= 8 {
                            if live {
                                for c in 0..3 {
                                    let xv = ((vis[1] >> (16 * c)) & 0xffff) as f32 / 65535.0;
                                    let x = (xv * 15.0).round() as u32;
                                    self.grid.set(x.min(w - 1), 3, [15, 9, 6][c]);
                                }
                            }
                            let fader = |v: f32| (v.clamp(0.0, 1.0) * 15.0).round() as u32;
                            let dv = fader(dev.params[mb::DEJA_VU]);
                            let len = dev.params[mb::LENGTH].round().clamp(1.0, 16.0) as u32;
                            let bias = fader(dev.params[mb::BIAS]);
                            for x in 0..16.min(w) {
                                self.grid.set(x, 4, if x == dv { 12 } else if x < dv { 4 } else { 1 });
                                self.grid.set(x, 5, if x + 1 == len { 12 } else if x < len { 4 } else { 1 });
                                self.grid.set(x, 6, if x == bias { 12 } else { 2 });
                            }
                            let model = dev.params[mb::MODEL].round() as u32;
                            for x in 0..(mb::MODELS.len() as u32).min(w) {
                                self.grid.set(x, 7, if x == model { 15 } else { 3 });
                            }
                            let jitter = (dev.params[mb::JITTER].clamp(0.0, 1.0) * 6.0).round() as u32;
                            for x in 9..16.min(w) {
                                self.grid.set(x, 7, if x - 9 == jitter { 12 } else if x - 9 < jitter { 4 } else { 1 });
                            }
                        }
                    }
                    MidiFxKind::Progression => {
                        let chords = dev.params[1].round().clamp(1.0, 8.0) as u32;
                        let current = (vis[0] & 0xff) as u32;
                        for x in 0..8.min(w) {
                            let (degree, quality, _) = krabton_engine::classic_seq::prog_slot(&dev.data, x as usize);
                            let used = x < chords;
                            if playing && x == current {
                                for y in 0..7.min(h) {
                                    self.grid.set(x, y, 2);
                                }
                            }
                            if degree > 0 {
                                self.grid.set(x, 7 - degree as u32, if used { 13 } else { 4 });
                            }
                            self.grid.set(x, h - 1, quality * 2 + if used { 3 } else { 1 });
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    /// The parameters an arc controls: (label, value 0..1).
    pub(crate) fn arc_targets(&self) -> Vec<(String, f32)> {
        // In the Mod tab the arc plays the selected modulator.
        if let Some(m) = self.arc_modulator() {
            let specs = m.kind.specs();
            return specs.iter().zip(&m.params).take(4).map(|(s, v)| (s.name.to_string(), s.to_norm(*v))).collect();
        }
        let TrackSel::Track(t) = self.sel_track else { return Vec::new() };
        let Some(track) = self.project.tracks.get(t) else { return Vec::new() };
        if let Some(dev) = track.midi_devices.first() {
            let specs = dev.kind.specs();
            return specs.iter().zip(&dev.params).take(4).map(|(s, v)| (s.name.to_string(), s.to_norm(*v))).collect();
        }
        let m = &track.mixer;
        vec![
            ("Volume".into(), crate::widgets::db_to_fader(m.volume_db)),
            ("Pan".into(), (m.pan + 1.0) * 0.5),
            ("Send A".into(), m.sends.first().copied().unwrap_or(0.0)),
            ("Send B".into(), m.sends.get(1).copied().unwrap_or(0.0)),
        ]
    }

    fn arc_modulator(&self) -> Option<&krabton_engine::modulation::Modulator> {
        if self.detail != crate::app::DetailTab::Mod {
            return None;
        }
        let id = self.sel_mod?;
        self.project.modulators.iter().find(|m| m.id == id)
    }

    fn arc_delta(&mut self, n: usize, delta: i32) {
        let step = delta as f32 / 256.0;
        if let Some(id) = self.arc_modulator().map(|m| m.id) {
            if let Some(m) = self.project.modulators.iter_mut().find(|m| m.id == id) {
                let specs = m.kind.specs();
                if let (Some(spec), Some(v)) = (specs.get(n), m.params.get_mut(n)) {
                    *v = spec.from_norm(spec.to_norm(*v) + step);
                }
            }
            return;
        }
        let TrackSel::Track(t) = self.sel_track else { return };
        let Some(track) = self.project.tracks.get_mut(t) else { return };
        if let Some(dev) = track.midi_devices.first_mut() {
            let specs = dev.kind.specs();
            if let (Some(spec), Some(v)) = (specs.get(n), dev.params.get_mut(n)) {
                *v = spec.from_norm(spec.to_norm(*v) + step);
            }
            return;
        }
        let m = &mut track.mixer;
        match n {
            0 => m.volume_db = crate::widgets::fader_to_db(crate::widgets::db_to_fader(m.volume_db) + step),
            1 => m.pan = (m.pan + step * 2.0).clamp(-1.0, 1.0),
            k => {
                if let Some(s) = m.sends.get_mut(k - 2) {
                    *s = (*s + step).clamp(0.0, 1.0);
                }
            }
        }
    }

    fn arc_rings(&self) -> Vec<[u8; 64]> {
        self.arc_targets()
            .iter()
            .map(|(_, v)| {
                let mut ring = [0u8; 64];
                let lit = (v.clamp(0.0, 1.0) * 63.0).round() as usize;
                for (i, led) in ring.iter_mut().enumerate() {
                    *led = if i == lit { 15 } else if i < lit { 6 } else { 0 };
                }
                ring
            })
            .collect()
    }
}

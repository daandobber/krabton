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
        let (monome, error) = match Monome::with_serialosc_port(port) {
            Ok(mut m) => {
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
                if y == h - 1 {
                    dev.params[0] = (x as f32 + 1.0).clamp(1.0, 8.0);
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
                        let n = (vis[1] as u32).clamp(1, 8);
                        for i in 0..n {
                            let y = ((vis[0] >> (i * 8)) & 0xff) as f32 / 255.0;
                            let x = i * 2;
                            let gy = (h - 2) - (y * (h - 2) as f32).round() as u32;
                            self.grid.set(x, gy, 15);
                            self.grid.set(x, h - 2, 2);
                        }
                        for x in 0..(dev.params[0].round() as u32).min(w) {
                            self.grid.set(x, h - 1, 3);
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

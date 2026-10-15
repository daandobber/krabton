//! Grand Piano: plays the Salamander Grand Piano (see `library`) the way
//! its SFZ does: 16 velocity layers with velocity tracking, a key struck
//! again damps its sounding string first, dampers fall on release (the
//! highest keys have none), and on release the key and damper noise and the
//! strings' resonance sound, softer the longer the key was held. The
//! sustain pedal (MIDI CC 64, or the Sustain switch) holds the notes and
//! makes its own noise. Around it: una corda, lid and tone, stereo width,
//! the player's or the audience's side, and a hall.

pub mod library;

use crate::dsp::biquad::{self, Coefs};
use crate::dsp::effects::FxCtx;
use crate::dsp::fx::hall::{self, Hall};
use crate::dsp::{db_to_gain, decay_coef};
use crate::params::{ParamSpec, Unit};
use library::{PianoLibrary, LAYER_LOVEL, OFFSETS, REGIONS, RETUNE};
use std::sync::Arc;

pub const DYNAMICS: usize = 0;
pub const SOFT: usize = 1;
pub const RELEASE: usize = 2;
pub const KEY_NOISE: usize = 3;
pub const RESONANCE: usize = 4;
pub const PEDAL_NOISE: usize = 5;
pub const SUSTAIN: usize = 6;
pub const LID: usize = 7;
pub const TONE: usize = 8;
pub const WIDTH: usize = 9;
pub const SIDE: usize = 10;
pub const ROOM: usize = 11;
pub const ROOM_SIZE: usize = 12;
pub const TUNING: usize = 13;
pub const FINE: usize = 14;
pub const TRANSPOSE: usize = 15;
pub const VOLUME: usize = 16;
pub const ACTION: usize = 17;

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::lin("Dynamics", "Touch", 0.0, 1.0, 0.73, Unit::Percent),
    ParamSpec::lin("Una Corda", "Touch", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::log("Dampers", "Touch", 0.05, 3.0, 0.6, Unit::Sec),
    ParamSpec::lin("Key Noise", "Noises", 0.0, 2.0, 1.0, Unit::Percent),
    ParamSpec::lin("Resonance", "Noises", 0.0, 2.0, 1.0, Unit::Percent),
    ParamSpec::lin("Pedal Noise", "Noises", 0.0, 2.0, 1.0, Unit::Percent),
    ParamSpec::toggle("Sustain", "Pedal", false),
    ParamSpec::lin("Lid", "Tone", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::lin("Tone", "Tone", -1.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Width", "Space", 0.0, 1.5, 1.0, Unit::Percent),
    ParamSpec::choice("Side", "Space", &["Player", "Audience"], 0),
    ParamSpec::lin("Room", "Space", 0.0, 1.0, 0.15, Unit::Percent),
    ParamSpec::lin("Room Size", "Space", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::choice("Tuning", "Tuning", &["Natural", "Retuned"], 0),
    ParamSpec::lin("Fine", "Tuning", -50.0, 50.0, 0.0, Unit::Cents),
    ParamSpec::int("Transpose", "Tuning", -24.0, 24.0, 0.0, Unit::Semi),
    ParamSpec::lin("Volume", "Output", -24.0, 12.0, 0.0, Unit::Db),
    ParamSpec::lin("Action", "Noises", 0.0, 1.0, 0.0, Unit::Percent),
];

pub fn preset_names() -> &'static [&'static str] {
    &["Concert Grand", "Studio Close", "Concert Hall", "Intimate", "Felt Piano", "Pedal Down", "Pop Piano", "Audience Seat"]
}

pub fn preset(name: &str) -> Vec<f32> {
    let mut p = crate::params::defaults(SPECS);
    let mut set = |pairs: &[(usize, f32)]| {
        for &(i, v) in pairs {
            p[i] = v;
        }
    };
    match name {
        "Studio Close" => set(&[(ROOM, 0.05), (TONE, 0.25), (WIDTH, 1.2)]),
        "Concert Hall" => set(&[(ROOM, 0.35), (ROOM_SIZE, 0.85), (WIDTH, 0.9)]),
        "Intimate" => set(&[(SOFT, 0.5), (LID, 0.35), (ROOM, 0.2), (DYNAMICS, 0.85), (KEY_NOISE, 1.3)]),
        "Felt Piano" => set(&[(SOFT, 1.0), (LID, 0.0), (TONE, -0.7), (KEY_NOISE, 1.8), (PEDAL_NOISE, 1.4), (RELEASE, 0.35), (ROOM, 0.12), (DYNAMICS, 0.9)]),
        "Pedal Down" => set(&[(SUSTAIN, 1.0), (RESONANCE, 1.5), (ROOM, 0.25), (ROOM_SIZE, 0.7)]),
        "Pop Piano" => set(&[(DYNAMICS, 0.45), (TONE, 0.4), (ROOM, 0.1), (WIDTH, 1.3)]),
        "Audience Seat" => set(&[(SIDE, 1.0), (ROOM, 0.3), (ROOM_SIZE, 0.7), (WIDTH, 0.8), (LID, 1.0)]),
        _ => {}
    }
    p
}

const MAX_VOICES: usize = 96;
const CHUNK: usize = 128;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Note,
    Noise,
}

#[derive(Clone, Copy)]
struct Voice {
    active: bool,
    kind: Kind,
    sample: u32,
    pos: f64,
    step: f64,
    gain: f32,
    env: f32,
    /// Per-sample fall once damped (1: rings on).
    fall: f32,
    /// A short fade-in when starting into a sample (0..1).
    rise: f32,
    key: u8,
    age: u64,
}

impl Voice {
    const IDLE: Self = Self { active: false, kind: Kind::Note, sample: 0, pos: 0.0, step: 1.0, gain: 0.0, env: 1.0, fall: 1.0, rise: 1.0, key: 0, age: 0 };
}

pub struct PianoState {
    lib: Option<Arc<PianoLibrary>>,
    voices: Vec<Voice>,
    sr: f32,
    counter: u64,
    clock: u64,
    /// Per key: down, when it went down (clock), its velocity (1..127).
    down: [bool; 128],
    down_at: [u64; 128],
    down_vel: [u8; 128],
    /// The pedal from MIDI, and the pedal as last applied (switch or MIDI).
    cc_pedal: bool,
    pedal: bool,
    rng: u32,
    tone: [Coefs; 2],
    tone_state: [[biquad::State; 2]; 2],
    tone_key: (f32, f32, f32),
    hall: Box<Hall>,
    hall_p: Vec<f32>,
    bus_l: Vec<f32>,
    bus_r: Vec<f32>,
}

impl PianoState {
    pub fn new(sr: f32) -> Self {
        Self {
            lib: None,
            voices: vec![Voice::IDLE; MAX_VOICES],
            sr,
            counter: 0,
            clock: 0,
            down: [false; 128],
            down_at: [0; 128],
            down_vel: [0; 128],
            cc_pedal: false,
            pedal: false,
            rng: 0x51A1_A4D2,
            tone: [Coefs::IDENTITY; 2],
            tone_state: Default::default(),
            tone_key: (f32::NAN, f32::NAN, f32::NAN),
            hall: Box::new(Hall::new(sr)),
            hall_p: crate::params::defaults(hall::SPECS),
            bus_l: vec![0.0; CHUNK],
            bus_r: vec![0.0; CHUNK],
        }
    }

    pub fn set_library(&mut self, lib: Option<Arc<PianoLibrary>>) -> Option<Arc<PianoLibrary>> {
        for v in self.voices.iter_mut() {
            v.active = false;
        }
        std::mem::replace(&mut self.lib, lib)
    }

    pub fn has_library(&self) -> bool {
        self.lib.is_some()
    }

    fn key_of(p: &[f32], note: u8) -> u8 {
        (note as i32 + p[TRANSPOSE].round() as i32).clamp(21, 108) as u8
    }

    fn region(key: u8) -> usize {
        REGIONS.iter().position(|r| (r.1..=r.2).contains(&key)).unwrap_or(0)
    }

    /// Velocity tracking as SFZ players do amp_veltrack: between full level
    /// and the velocity squared, `track` of the way.
    fn vel_gain(vel: u8, track: f32) -> f32 {
        let v = vel.max(1) as f32 / 127.0;
        (1.0 - track) + track * v * v
    }

    fn rand(&mut self) -> f32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        (self.rng >> 8) as f32 / 16_777_216.0
    }

    fn start(&mut self, kind: Kind, sample: u32, key: u8, step: f64, gain: f32) {
        self.start_at(kind, sample, key, step, gain, 0.0);
    }

    fn start_at(&mut self, kind: Kind, sample: u32, key: u8, step: f64, gain: f32, from: f64) {
        let Some(lib) = &self.lib else { return };
        if !lib.exists(sample) || gain < 1e-5 {
            return;
        }
        lib.prefetch(sample);
        self.counter += 1;
        // A free voice, or the quietest damped one, or the oldest.
        let pick = self
            .voices
            .iter()
            .position(|v| !v.active)
            .or_else(|| self.voices.iter().enumerate().filter(|(_, v)| v.fall < 1.0).min_by(|a, b| a.1.env.partial_cmp(&b.1.env).unwrap_or(std::cmp::Ordering::Equal)).map(|(i, _)| i))
            .unwrap_or_else(|| self.voices.iter().enumerate().min_by_key(|(_, v)| v.age).map(|(i, _)| i).unwrap_or(0));
        let rise = if from > 0.0 { 0.0 } else { 1.0 };
        self.voices[pick] = Voice { active: true, kind, sample, pos: from, step, gain, env: 1.0, fall: 1.0, rise, key, age: self.counter };
    }

    /// Pitch step for playing a sample recorded at `center` as `key`.
    fn step(&self, p: &[f32], region: usize, key: u8) -> f64 {
        let Some(lib) = &self.lib else { return 1.0 };
        let cents = (key as f64 - REGIONS[region].0 as f64) * 100.0 + p[FINE] as f64 + if p[TUNING] >= 0.5 { RETUNE[region] as f64 } else { 0.0 };
        2f64.powf(cents / 1200.0) * lib.sample_rate as f64 / self.sr as f64
    }

    pub fn note_on(&mut self, p: &[f32], note: u8, vel: f32) {
        if p.len() < SPECS.len() || self.lib.is_none() {
            return;
        }
        let key = Self::key_of(p, note);
        // Una corda: softer, and so a lower (darker) layer.
        let v = ((vel.clamp(0.0, 1.0) * 127.0) * (1.0 - 0.3 * p[SOFT].clamp(0.0, 1.0))).round().clamp(1.0, 127.0) as u8;
        let r = Self::region(key);
        let layer = LAYER_LOVEL.iter().rposition(|&lo| v >= lo).unwrap_or(0);
        // The string struck again: its sound so far dies away quickly.
        let choke = decay_coef(0.25, self.sr);
        for vc in self.voices.iter_mut().filter(|vc| vc.active && vc.kind == Kind::Note && vc.key == key) {
            vc.fall = vc.fall.min(choke);
        }
        let Some(lib) = &self.lib else { return };
        let sample = lib.notes[r][layer];
        // Start where the hammer hits; Action keeps (some of) the key going down before it.
        let from = OFFSETS[layer][r] as f64 * lib.sample_rate as f64 / 48000.0 * (1.0 - p.get(ACTION).copied().unwrap_or(0.0).clamp(0.0, 1.0) as f64);
        let step = self.step(p, r, key);
        let gain = Self::vel_gain(v, p[DYNAMICS].clamp(0.0, 1.0));
        self.start_at(Kind::Note, sample, key, step, gain, from);
        self.down[key as usize] = true;
        self.down_at[key as usize] = self.clock;
        self.down_vel[key as usize] = v;
    }

    pub fn note_off(&mut self, p: &[f32], note: u8) {
        if p.len() < SPECS.len() {
            return;
        }
        let key = Self::key_of(p, note);
        if !self.down[key as usize] {
            return;
        }
        self.down[key as usize] = false;
        if !self.pedal {
            self.damp(p, key);
        }
    }

    /// The damper falls on `key`: its notes die away and the release noises sound.
    fn damp(&mut self, p: &[f32], key: u8) {
        let mut sounding = false;
        // The top keys have no dampers.
        let time = if key >= 89 { 3.5 } else { p[RELEASE].max(0.02) };
        let fall = decay_coef(time, self.sr);
        for v in self.voices.iter_mut().filter(|v| v.active && v.kind == Kind::Note && v.key == key) {
            if v.fall >= 1.0 {
                sounding = true;
            }
            v.fall = v.fall.min(fall);
        }
        if !sounding || key >= 89 {
            return;
        }
        let Some(lib) = self.lib.clone() else { return };
        let held = (self.clock.saturating_sub(self.down_at[key as usize])) as f32 / self.sr;
        let v = self.down_vel[key as usize].max(1);
        let r = Self::region(key);
        // Key and damper noise: -37 dB, softer the longer the key was down.
        let hammer = db_to_gain(-37.0 - 2.0 * held) * Self::vel_gain(v, 0.82) * p[KEY_NOISE].max(0.0);
        let rel = lib.rel[(key - 21) as usize];
        self.start(Kind::Noise, rel, key, lib.sample_rate as f64 / self.sr as f64, hammer);
        // The strings' resonance as the damper lands.
        let res = p[RESONANCE].max(0.0);
        if res > 0.0 && r < 23 {
            let step = self.step(p, r, key);
            // How fast (dB per second held) the loud resonance fades, by register.
            let rt: f32 = match r {
                0..=2 => 6.0,
                3..=5 => 7.0,
                6..=9 => 8.0,
                _ => 9.0,
            };
            if v >= 45 {
                self.start(Kind::Noise, lib.harm[0][r], key, step, db_to_gain(-4.0 - rt * held) * Self::vel_gain(v, 0.92) * res);
            } else {
                self.start(Kind::Noise, lib.harm[1][r], key, step, db_to_gain(-7.0 * held) * Self::vel_gain(v, 0.92) * res);
            }
            self.start(Kind::Noise, lib.harm[2][r], key, step, db_to_gain(-2.0 * held) * Self::vel_gain(v, 0.96) * res);
        }
    }

    /// The sustain pedal from MIDI (CC 64).
    pub fn sustain(&mut self, down: bool) {
        self.cc_pedal = down;
    }

    fn set_pedal(&mut self, p: &[f32], down: bool) {
        if down == self.pedal {
            return;
        }
        self.pedal = down;
        if let Some(lib) = self.lib.clone() {
            let k = (self.rand() * 2.0) as usize % 2;
            let (sample, db) = if down { (lib.pedal_down[k], -20.0) } else { (lib.pedal_up[k], -19.0) };
            self.start(Kind::Noise, sample, 0, lib.sample_rate as f64 / self.sr as f64, db_to_gain(db) * p[PEDAL_NOISE].max(0.0));
        }
        if !down {
            // Every string not held by its key is damped.
            for key in 21..=108u8 {
                if !self.down[key as usize] && self.voices.iter().any(|v| v.active && v.kind == Kind::Note && v.key == key && v.fall >= 1.0) {
                    self.damp(p, key);
                }
            }
        }
    }

    pub fn all_off(&mut self) {
        let fall = decay_coef(0.1, self.sr);
        for v in self.voices.iter_mut().filter(|v| v.active) {
            v.fall = v.fall.min(fall);
        }
        self.down = [false; 128];
        self.cc_pedal = false;
        self.pedal = false;
    }

    /// For the display: [0] notes sounding, [1] pedal down, [2] library
    /// ready, [3..7] the keys sounding (A0 up, 22 keys per value).
    pub fn visual(&self, out: &mut [f32; crate::engine::INST_VALS]) {
        out[0] = self.voices.iter().filter(|v| v.active && v.kind == Kind::Note).count() as f32;
        out[1] = self.pedal as u8 as f32;
        out[2] = self.lib.is_some() as u8 as f32;
        let mut bits = [0u32; 4];
        for v in self.voices.iter().filter(|v| v.active && v.kind == Kind::Note && v.env > 0.01) {
            let k = v.key.saturating_sub(21) as usize;
            if k < 88 {
                bits[k / 22] |= 1 << (k % 22);
            }
        }
        for (i, b) in bits.iter().enumerate() {
            out[3 + i] = *b as f32;
        }
    }

    pub fn render(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32]) {
        if p.len() < SPECS.len() {
            return;
        }
        let want = self.cc_pedal || p[SUSTAIN] >= 0.5;
        self.set_pedal(p, want);
        let n = l.len().min(r.len());
        let mut done = 0;
        while done < n {
            let len = CHUNK.min(n - done);
            self.render_chunk(p, &mut l[done..done + len], &mut r[done..done + len]);
            done += len;
        }
    }

    fn render_chunk(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32]) {
        let len = l.len();
        self.clock += len as u64;
        let (bl, br) = (&mut self.bus_l[..len], &mut self.bus_r[..len]);
        bl.fill(0.0);
        br.fill(0.0);
        let any = self.voices.iter().any(|v| v.active);
        if let (Some(lib), true) = (&self.lib, any) {
            for v in self.voices.iter_mut().filter(|v| v.active) {
                let data = lib.frames(v.sample);
                let frames = data.len() / 2;
                for i in 0..len {
                    let idx = v.pos as usize;
                    if idx + 2 >= frames || v.env < 1e-4 {
                        v.active = false;
                        break;
                    }
                    // Cubic (Hermite) between frames.
                    let t = (v.pos - idx as f64) as f32;
                    let a = idx.saturating_sub(1);
                    let get = |k: usize, c: usize| data[k * 2 + c] as f32 * (1.0 / 32768.0);
                    let h = |c: usize| {
                        let (y0, y1, y2, y3) = (get(a, c), get(idx, c), get(idx + 1, c), get(idx + 2, c));
                        let c1 = 0.5 * (y2 - y0);
                        let c2 = y0 - 2.5 * y1 + 2.0 * y2 - 0.5 * y3;
                        let c3 = 0.5 * (y3 - y0) + 1.5 * (y1 - y2);
                        ((c3 * t + c2) * t + c1) * t + y1
                    };
                    let g = v.gain * v.env * v.rise;
                    v.rise = (v.rise + 1.0 / 96.0).min(1.0);
                    bl[i] += h(0) * g;
                    br[i] += h(1) * g;
                    v.env *= v.fall;
                    v.pos += v.step;
                }
            }
        }
        // Tone: the lid and una corda take off the top, Tone tilts.
        let key = (p[LID], p[SOFT], p[TONE]);
        if key != self.tone_key {
            self.tone_key = key;
            let sr = self.sr as f64;
            let top = -10.0 * (1.0 - p[LID].clamp(0.0, 1.0)) - 5.0 * p[SOFT].clamp(0.0, 1.0) + 5.0 * p[TONE].clamp(-1.0, 1.0);
            self.tone = [biquad::high_shelf(2500.0, sr, 0.7, top as f64), biquad::low_shelf(250.0, sr, 0.7, -2.5 * p[TONE].clamp(-1.0, 1.0) as f64)];
        }
        let width = p[WIDTH].clamp(0.0, 1.5);
        let swap = p[SIDE] >= 0.5;
        for i in 0..len {
            let (mut x, mut y) = (bl[i] as f64, br[i] as f64);
            for k in 0..2 {
                x = self.tone_state[0][k].run(&self.tone[k], x);
                y = self.tone_state[1][k].run(&self.tone[k], y);
            }
            let (m, s) = ((x + y) * 0.5, (x - y) * 0.5 * width as f64);
            let (a, b) = ((m + s) as f32, (m - s) as f32);
            (bl[i], br[i]) = if swap { (b, a) } else { (a, b) };
        }
        // The hall.
        let room = p[ROOM].clamp(0.0, 1.0);
        if room > 0.001 {
            let size = p[ROOM_SIZE].clamp(0.0, 1.0);
            let hp = &mut self.hall_p;
            hp[hall::PREDELAY] = 8.0 + 22.0 * size;
            hp[hall::SIZE] = 0.3 + 0.6 * size;
            hp[hall::DECAY] = 1.0 + 3.0 * size;
            // A still room: no modulation, which would make piano tones waver.
            hp[hall::DAMPING] = 7000.0;
            hp[hall::DIFFUSION] = 0.8;
            hp[hall::MODULATION] = 0.0;
            hp[hall::MIX] = room;
            let ctx = FxCtx { sr: self.sr, bpm: 120.0, beat: 0.0, playing: false };
            self.hall.process(hp, bl, br, &ctx, None);
        }
        // Headroom for big chords, and a soft knee above -2 dBFS for the loudest.
        let vol = db_to_gain(p[VOLUME]) * 0.5;
        let knee = |x: f32| {
            let a = x.abs();
            if a <= 0.8 {
                x
            } else {
                x.signum() * (0.8 + 0.2 * ((a - 0.8) / 0.2).tanh())
            }
        };
        for i in 0..len {
            l[i] += knee(bl[i] * vol);
            r[i] += knee(br[i] * vol);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lib() -> Arc<PianoLibrary> {
        static LIB: std::sync::OnceLock<Arc<PianoLibrary>> = std::sync::OnceLock::new();
        LIB.get_or_init(|| {
            // The libraries of earlier runs go (one still open elsewhere stays).
            let tmp = std::env::temp_dir();
            if let Ok(rd) = std::fs::read_dir(&tmp) {
                for e in rd.flatten() {
                    if e.file_name().to_string_lossy().starts_with("krabton-piano-test-") {
                        let _ = std::fs::remove_dir_all(e.path());
                    }
                }
            }
            let dir = tmp.join(format!("krabton-piano-test-{}", std::process::id()));
            library::write_test_library(&dir, 48000);
            PianoLibrary::open(&dir).unwrap()
        })
        .clone()
    }

    fn render(s: &mut PianoState, p: &[f32], secs: f32) -> Vec<f32> {
        let n = (secs * 48000.0) as usize;
        let (mut l, mut r) = (vec![0.0; n], vec![0.0; n]);
        for (a, b) in l.chunks_mut(256).zip(r.chunks_mut(256)) {
            s.render(p, a, b);
        }
        l
    }

    fn peak(v: &[f32]) -> f32 {
        v.iter().fold(0.0f32, |m, x| m.max(x.abs()))
    }

    #[test]
    fn the_library_maps_every_key_and_layer() {
        let lib = lib();
        assert!(lib.notes.iter().flatten().all(|&s| lib.exists(s)));
        assert!(lib.rel.iter().all(|&s| lib.exists(s)));
        assert!(lib.harm.iter().all(|h| h[..23].iter().all(|&s| lib.exists(s))));
        assert!(lib.pedal_down.iter().chain(&lib.pedal_up).all(|&s| lib.exists(s)));
    }

    #[test]
    fn plays_louder_layers_harder_and_damps_on_release() {
        let mut p = preset("Concert Grand");
        p[ROOM] = 0.0;
        let mut s = PianoState::new(48000.0);
        s.set_library(Some(lib()));
        s.note_on(&p, 60, 0.2);
        let soft = peak(&render(&mut s, &p, 0.2));
        s.note_off(&p, 60);
        render(&mut s, &p, 2.0);
        assert!(s.voices.iter().all(|v| !v.active), "damped and done");
        s.note_on(&p, 60, 1.0);
        let loud = peak(&render(&mut s, &p, 0.2));
        assert!(loud > soft * 3.0, "{soft} vs {loud}");
        // Releasing sounds the key noise and resonance.
        s.note_off(&p, 60);
        assert!(s.voices.iter().filter(|v| v.active && v.kind == Kind::Noise).count() >= 2);
    }

    #[test]
    fn the_pedal_holds_notes_until_it_is_lifted() {
        let mut p = preset("Concert Grand");
        p[ROOM] = 0.0;
        let mut s = PianoState::new(48000.0);
        s.set_library(Some(lib()));
        s.sustain(true);
        render(&mut s, &p, 0.01);
        s.note_on(&p, 64, 0.8);
        render(&mut s, &p, 0.1);
        s.note_off(&p, 64);
        render(&mut s, &p, 0.3);
        let held = s.voices.iter().any(|v| v.active && v.kind == Kind::Note && v.fall >= 1.0);
        assert!(held, "the pedal keeps it ringing");
        s.sustain(false);
        render(&mut s, &p, 0.01);
        assert!(s.voices.iter().filter(|v| v.active && v.kind == Kind::Note).all(|v| v.fall < 1.0), "lifting the pedal damps it");
    }

    #[test]
    fn keys_between_samples_are_pitched() {
        let mut p = preset("Concert Grand");
        p[ROOM] = 0.0;
        let mut s = PianoState::new(48000.0);
        s.set_library(Some(lib()));
        // C4 (60) is a sample, C#4 is one semitone up from it.
        s.note_on(&p, 61, 0.8);
        let v = s.voices.iter().find(|v| v.active).unwrap();
        assert!((v.step - 2f64.powf(1.0 / 12.0)).abs() < 1e-6, "{}", v.step);
        // A key struck again chokes its old sound.
        s.note_on(&p, 61, 0.8);
        assert_eq!(s.voices.iter().filter(|v| v.active && v.key == 61 && v.fall < 1.0).count(), 1);
        let out = render(&mut s, &p, 0.3);
        assert!(out.iter().all(|x| x.is_finite()) && peak(&out) > 0.01);
    }
}

#[cfg(test)]
mod real {
    use super::*;

    /// With the real library: `KRABTON_PIANO_DIR=... cargo test -p
    /// krabton-engine real_piano_levels -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn real_piano_levels() {
        let dir = std::path::PathBuf::from(std::env::var("KRABTON_PIANO_DIR").expect("KRABTON_PIANO_DIR"));
        let lib = PianoLibrary::open(&dir).unwrap();
        let p = preset("Concert Grand");
        let peak = |v: &[f32]| v.iter().fold(0.0f32, |m, x| m.max(x.abs()));
        let rms = |v: &[f32]| (v.iter().map(|x| x * x).sum::<f32>() / v.len() as f32).sqrt();
        let run = |notes: &[u8], vel: f32, pedal: bool, secs: f32| {
            let mut s = PianoState::new(48000.0);
            s.set_library(Some(lib.clone()));
            s.sustain(pedal);
            let n = (secs * 48000.0) as usize;
            let (mut l, mut r) = (vec![0.0; n], vec![0.0; n]);
            for &k in notes {
                s.note_on(&p, k, vel);
            }
            for (a, b) in l.chunks_mut(256).zip(r.chunks_mut(256)) {
                s.render(&p, a, b);
            }
            (l, r)
        };
        // What is before the hammer: the key going down.
        for (r, l) in [(13usize, 7usize), (5, 12), (20, 3)] {
            let d = lib.frames(lib.notes[r][l]);
            let off = OFFSETS[l][r] as usize;
            let level = |a: usize, b: usize| (d[a * 2..b * 2].iter().map(|&x| (x as f32 / 32768.0).powi(2)).sum::<f32>() / ((b - a) * 2) as f32).sqrt();
            let zc = |a: usize, b: usize| d[a * 2..b * 2].chunks(2).collect::<Vec<_>>().windows(2).filter(|w| (w[0][0] < 0) != (w[1][0] < 0)).count() as f32 / (b - a) as f32 * 48000.0 / 2.0;
            eprintln!("region {r} layer {l}: before the hammer rms {:.4} (~{:.0} Hz), after rms {:.4} (~{:.0} Hz)", level(0, off), zc(0, off), level(off, off + 4800), zc(off, off + 4800));
        }
        for vel in [0.1, 0.3, 0.5, 0.7, 0.9, 1.0] {
            let (l, _) = run(&[60], vel, false, 1.0);
            eprintln!("C4 vel {vel:.1}: peak {:.3} rms {:.4}", peak(&l), rms(&l));
        }
        for k in [21, 36, 48, 72, 84, 96, 108] {
            let (l, _) = run(&[k], 0.7, false, 1.0);
            eprintln!("key {k} vel 0.7: peak {:.3}", peak(&l));
        }
        let (l, r) = run(&[36, 48, 55, 60, 64, 67, 72, 76, 79, 84], 1.0, true, 3.0);
        eprintln!("ten-note fortissimo chord with pedal: peak {:.3} / {:.3}", peak(&l), peak(&r));
        // A demo to listen to.
        let mut s = PianoState::new(48000.0);
        s.set_library(Some(lib.clone()));
        let mut out = Vec::new();
        let melody: &[(u8, f32)] = &[(60, 0.5), (64, 0.55), (67, 0.6), (72, 0.75), (71, 0.6), (67, 0.5), (64, 0.45), (62, 0.5), (60, 0.4)];
        let mut buf = (vec![0.0f32; 12000], vec![0.0f32; 12000]);
        s.note_on(&p, 36, 0.5);
        s.note_on(&p, 48, 0.45);
        s.sustain(true);
        for &(k, v) in melody {
            s.note_on(&p, k, v);
            buf.0.fill(0.0);
            buf.1.fill(0.0);
            s.render(&p, &mut buf.0, &mut buf.1);
            s.note_off(&p, k);
            out.extend(buf.0.iter().zip(&buf.1).flat_map(|(a, b)| [*a, *b]));
        }
        s.note_off(&p, 36);
        s.note_off(&p, 48);
        s.sustain(false);
        for _ in 0..16 {
            buf.0.fill(0.0);
            buf.1.fill(0.0);
            s.render(&p, &mut buf.0, &mut buf.1);
            out.extend(buf.0.iter().zip(&buf.1).flat_map(|(a, b)| [*a, *b]));
        }
        if let Ok(path) = std::env::var("KRABTON_PIANO_DEMO") {
            crate::render::write_wav(std::path::Path::new(&path), 48000, &out).unwrap();
        }
        eprintln!("demo peak {:.3}", peak(&out));
    }
}

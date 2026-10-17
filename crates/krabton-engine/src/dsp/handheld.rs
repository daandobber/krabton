//! Handheld: the sound of the 8-bit handheld of 1989, made from how its chip
//! works. Two
//! pulse channels (four duties, sixteen volume steps; the first with the
//! sweep that slides its pitch), the wave channel (32 four-bit samples from
//! fifteen tables, at full, half or a quarter volume) and noise (a 15-bit or
//! a 7-bit shift register, the 7-bit one buzzing into a tone, its clock
//! chosen by shift and ratio, or following the keys). Each channel left,
//! right or both, as the chip pans; pitches from 11-bit registers, so low
//! notes stop at 64 Hz (pulses) and 32 Hz (wave); the hardware envelope
//! steps 64 times a second; vibrato and arpeggios move on the frame
//! (59.7 a second). The output's capacitor as on the first model or the
//! later ones, then a treble knob.

use super::chip::{self, Frames, HighPass, Keys, LowPass, ARP, LAYER, SPLIT};
use crate::params::{ParamSpec, Unit};

pub const MODE: usize = 0;
pub const VOICES: usize = 1;
pub const P1_LEVEL: usize = 2;
pub const P1_DUTY: usize = 3;
pub const P1_TUNE: usize = 4;
pub const P1_FINE: usize = 5;
pub const P1_SWEEP: usize = 6;
pub const P1_SHIFT: usize = 7;
pub const P1_PAN: usize = 8;
pub const P2_LEVEL: usize = 9;
pub const P2_DUTY: usize = 10;
pub const P2_TUNE: usize = 11;
pub const P2_FINE: usize = 12;
pub const P2_PAN: usize = 13;
pub const WAVE_LEVEL: usize = 14;
pub const WAVEFORM: usize = 15;
pub const WAVE_TUNE: usize = 16;
pub const WAVE_PAN: usize = 17;
pub const NOISE_LEVEL: usize = 18;
pub const NOISE_WIDTH: usize = 19;
pub const NOISE_SHIFT: usize = 20;
pub const NOISE_RATIO: usize = 21;
pub const NOISE_KEYS: usize = 22;
pub const NOISE_PAN: usize = 23;
pub const ATTACK: usize = 24;
pub const DECAY: usize = 25;
pub const RELEASE: usize = 26;
pub const VIBRATO: usize = 27;
pub const VIB_RATE: usize = 28;
pub const VIB_DELAY: usize = 29;
pub const ARP_SPEED: usize = 30;
pub const MODEL: usize = 31;
pub const TREBLE: usize = 32;
pub const MASTER: usize = 33;
pub const VOLUME: usize = 34;

pub const DUTIES: &[&str] = &["12.5%", "25%", "50%", "75%"];
pub const PANS: &[&str] = &["Both", "Left", "Right"];
pub const WAVE_LEVELS: &[&str] = &["Off", "25%", "50%", "100%"];
pub const WAVES: &[&str] = &["Sine", "Triangle", "Saw", "Square", "Pulse 25", "Pulse 12", "Organ", "Soft Bass", "Hollow", "Bell", "Buzz", "Vocal", "Steps", "Half Sine", "Grit"];
pub const WIDTHS: &[&str] = &["15-bit", "7-bit"];
pub const MODELS: &[&str] = &["First", "Later", "Raw"];

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::choice("Mode", "Play", chip::MODES, 0),
    ParamSpec::int("Voices", "Play", 1.0, 8.0, 4.0, Unit::None),
    ParamSpec::int("Pulse 1", "Pulse 1", 0.0, 15.0, 15.0, Unit::None),
    ParamSpec::choice("Duty 1", "Pulse 1", DUTIES, 2),
    ParamSpec::int("Tune 1", "Pulse 1", -24.0, 24.0, 0.0, Unit::Semi),
    ParamSpec::lin("Fine 1", "Pulse 1", -100.0, 100.0, 0.0, Unit::Cents),
    ParamSpec::int("Sweep", "Pulse 1", -7.0, 7.0, 0.0, Unit::None),
    ParamSpec::int("Sweep Shift", "Pulse 1", 0.0, 7.0, 3.0, Unit::None),
    ParamSpec::choice("Pan 1", "Pulse 1", PANS, 0),
    ParamSpec::int("Pulse 2", "Pulse 2", 0.0, 15.0, 0.0, Unit::None),
    ParamSpec::choice("Duty 2", "Pulse 2", DUTIES, 1),
    ParamSpec::int("Tune 2", "Pulse 2", -24.0, 24.0, 0.0, Unit::Semi),
    ParamSpec::lin("Fine 2", "Pulse 2", -100.0, 100.0, 8.0, Unit::Cents),
    ParamSpec::choice("Pan 2", "Pulse 2", PANS, 0),
    ParamSpec::choice("Wave", "Wave", WAVE_LEVELS, 0),
    ParamSpec::choice("Waveform", "Wave", WAVES, 0),
    ParamSpec::int("Wave Tune", "Wave", -24.0, 24.0, 0.0, Unit::Semi),
    ParamSpec::choice("Wave Pan", "Wave", PANS, 0),
    ParamSpec::int("Noise", "Noise", 0.0, 15.0, 0.0, Unit::None),
    ParamSpec::choice("Width", "Noise", WIDTHS, 0),
    ParamSpec::int("Noise Shift", "Noise", 0.0, 13.0, 3.0, Unit::None),
    ParamSpec::int("Noise Ratio", "Noise", 0.0, 7.0, 0.0, Unit::None),
    ParamSpec::toggle("Noise Keys", "Noise", true),
    ParamSpec::choice("Noise Pan", "Noise", PANS, 0),
    ParamSpec::int("Attack", "Envelope", 0.0, 7.0, 0.0, Unit::None),
    ParamSpec::int("Decay", "Envelope", 0.0, 7.0, 0.0, Unit::None),
    ParamSpec::int("Release", "Envelope", 0.0, 7.0, 1.0, Unit::None),
    ParamSpec::lin("Vibrato", "Play", 0.0, 100.0, 0.0, Unit::Cents),
    ParamSpec::lin("Vib Rate", "Play", 1.0, 15.0, 6.0, Unit::Hz),
    ParamSpec::lin("Vib Delay", "Play", 0.0, 1.0, 0.2, Unit::Sec),
    ParamSpec::int("Arp Speed", "Play", 1.0, 8.0, 2.0, Unit::None),
    ParamSpec::choice("Model", "Output", MODELS, 0),
    ParamSpec::log("Treble", "Output", 1000.0, 20000.0, 20000.0, Unit::Hz),
    ParamSpec::int("Master", "Output", 0.0, 7.0, 7.0, Unit::None),
    ParamSpec::lin("Volume", "Output", 0.0, 1.0, 0.8, Unit::Percent),
];

pub fn preset_names() -> &'static [&'static str] {
    &["Square Lead", "Thin Pulse", "Wave Bass", "Wave Organ", "Bell Wave", "Pluck", "Arpeggio", "Chord Split", "Stereo Pulses", "Sweep Up", "Noise Snare", "Noise Tone", "Echo", "Soft Pad"]
}

pub fn preset(name: &str) -> Vec<f32> {
    let mut p = crate::params::defaults(SPECS);
    let set: &[(usize, f32)] = match name {
        "Thin Pulse" => &[(P1_DUTY, 0.0), (VIBRATO, 20.0)],
        "Wave Bass" => &[(P1_LEVEL, 0.0), (WAVE_LEVEL, 3.0), (WAVEFORM, 7.0), (WAVE_TUNE, -12.0), (RELEASE, 0.0)],
        "Wave Organ" => &[(P1_LEVEL, 0.0), (WAVE_LEVEL, 3.0), (WAVEFORM, 6.0), (VIBRATO, 12.0)],
        "Bell Wave" => &[(P1_LEVEL, 0.0), (WAVE_LEVEL, 3.0), (WAVEFORM, 9.0), (RELEASE, 4.0)],
        "Pluck" => &[(P1_DUTY, 1.0), (DECAY, 2.0), (RELEASE, 2.0)],
        "Arpeggio" => &[(MODE, 2.0), (P1_DUTY, 1.0), (ARP_SPEED, 2.0), (DECAY, 6.0)],
        "Chord Split" => &[(MODE, 1.0), (P2_LEVEL, 13.0), (P2_DUTY, 2.0), (P2_FINE, 0.0), (WAVE_LEVEL, 3.0), (WAVEFORM, 1.0), (DECAY, 5.0)],
        "Stereo Pulses" => &[(P1_PAN, 1.0), (P2_LEVEL, 15.0), (P2_PAN, 2.0), (P2_FINE, 12.0), (P1_FINE, -6.0), (P2_DUTY, 1.0)],
        "Sweep Up" => &[(P1_SWEEP, 2.0), (P1_SHIFT, 3.0), (P1_DUTY, 1.0), (DECAY, 3.0)],
        "Noise Snare" => &[(P1_LEVEL, 0.0), (NOISE_LEVEL, 15.0), (NOISE_KEYS, 0.0), (NOISE_SHIFT, 1.0), (NOISE_RATIO, 1.0), (DECAY, 1.0), (RELEASE, 1.0)],
        "Noise Tone" => &[(P1_LEVEL, 0.0), (NOISE_LEVEL, 13.0), (NOISE_WIDTH, 1.0), (NOISE_KEYS, 1.0), (DECAY, 4.0)],
        "Echo" => &[(P1_DUTY, 2.0), (P2_LEVEL, 5.0), (P2_DUTY, 2.0), (P2_FINE, 0.0), (P2_PAN, 2.0), (P1_PAN, 1.0), (RELEASE, 5.0)],
        "Soft Pad" => &[(P1_LEVEL, 10.0), (P1_DUTY, 2.0), (WAVE_LEVEL, 2.0), (WAVEFORM, 0.0), (ATTACK, 4.0), (RELEASE, 6.0), (VIBRATO, 10.0), (TREBLE, 5000.0)],
        _ => &[],
    };
    for &(i, v) in set {
        p[i] = v;
    }
    p
}

/// The wave tables (32 samples, 0 to 15).
pub fn wave_table(i: usize) -> [u8; 32] {
    let mut w = [0u8; 32];
    let tau = std::f32::consts::TAU;
    let mut seed = 0x2F6Bu32;
    for (k, s) in w.iter_mut().enumerate() {
        let x = k as f32 / 32.0;
        let v: f32 = match i {
            0 => 0.5 + 0.5 * (tau * x).sin(),
            1 => 1.0 - (2.0 * x - 1.0).abs(),
            2 => x,
            3 => (x < 0.5) as u8 as f32,
            4 => (x < 0.25) as u8 as f32,
            5 => (x < 0.125) as u8 as f32,
            6 => 0.5 + 0.25 * (tau * x).sin() + 0.15 * (2.0 * tau * x).sin() + 0.1 * (3.0 * tau * x).sin(),
            7 => 0.5 + 0.32 * (tau * x).sin() + 0.12 * (2.0 * tau * x).sin() + 0.05 * (3.0 * tau * x).sin(),
            8 => 0.5 + 0.3 * (tau * x).sin() + 0.18 * (3.0 * tau * x).sin(),
            9 => 0.5 + 0.3 * (tau * x).sin() + 0.2 * (tau * 3.5 * x).sin() * (tau * x).cos(),
            10 => {
                if k % 8 == 0 {
                    1.0
                } else {
                    0.0
                }
            }
            11 => 0.5 + 0.25 * (tau * x).sin() + 0.25 * (5.0 * tau * x).sin() * (1.0 - x),
            12 => (x * 4.0).floor() / 3.0,
            13 => (tau * x * 0.5).sin(),
            _ => {
                seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12345);
                (seed >> 16) as f32 / 65535.0
            }
        };
        *s = (v.clamp(0.0, 1.0) * 15.0).round() as u8;
    }
    w
}

const CLOCK: f32 = 4_194_304.0;
const FRAME: f32 = 59.73;
const DUTY: [[u8; 8]; 4] = [[0, 0, 0, 0, 0, 0, 0, 1], [1, 0, 0, 0, 0, 0, 0, 1], [1, 0, 0, 0, 0, 1, 1, 1], [0, 1, 1, 1, 1, 1, 1, 0]];

#[derive(Clone, Copy, Default, PartialEq, Debug)]
enum Env {
    #[default]
    Idle,
    Attack,
    Hold,
    Release,
}

#[derive(Clone, Copy, Default)]
struct Ch {
    note: u8,
    vel: f32,
    gate: bool,
    age: u64,
    env: Env,
    /// The volume (0-15) and the 64 Hz steps until it moves.
    vol: u8,
    count: u8,
    target: u8,
    frames_on: u32,
    /// The frequency register (11 bits), the sweep's own, and its count.
    reg: u16,
    sweep_reg: i32,
    sweep_count: u8,
    off: bool,
    noise_cycles: f32,
    timer: f32,
    pos: u8,
    cur: f32,
    lfsr: u16,
}

impl Ch {
    fn idle(&self) -> bool {
        self.env == Env::Idle
    }

    fn start(&mut self, note: u8, vel: f32, age: u64, retrigger: bool) {
        self.note = note;
        self.vel = vel;
        self.age = age;
        self.gate = true;
        if retrigger || self.idle() {
            self.env = Env::Attack;
            self.vol = 0;
            self.count = 0;
            self.frames_on = 0;
            self.sweep_reg = -1;
            self.sweep_count = 0;
            self.off = false;
            self.lfsr = 0x7FFF;
        }
    }

    fn release(&mut self) {
        self.gate = false;
        if self.env != Env::Idle {
            self.env = Env::Release;
            self.count = 0;
        }
    }

    /// The hardware envelope, 64 times a second.
    fn env_tick(&mut self, attack: u8, decay: u8, release: u8) {
        let step = |count: &mut u8, n: u8| -> bool {
            *count += 1;
            if *count >= n {
                *count = 0;
                true
            } else {
                false
            }
        };
        match self.env {
            Env::Idle => self.vol = 0,
            Env::Attack => {
                if attack == 0 {
                    self.vol = self.target;
                } else if step(&mut self.count, attack) {
                    self.vol = (self.vol + 1).min(self.target);
                }
                if self.vol >= self.target {
                    self.env = Env::Hold;
                    self.count = 0;
                }
            }
            Env::Hold => {
                if decay > 0 && self.vol > 0 && step(&mut self.count, decay) {
                    self.vol -= 1;
                }
                self.vol = self.vol.min(self.target);
            }
            Env::Release => {
                if release == 0 || self.vol == 0 {
                    self.vol = 0;
                } else if step(&mut self.count, release) {
                    self.vol -= 1;
                }
                if self.vol == 0 {
                    self.env = Env::Idle;
                }
            }
        }
    }
}

#[derive(Clone, Copy, Default)]
struct Chip {
    ch: [Ch; 4],
    hp: [HighPass; 2],
    lp: [LowPass; 2],
}

pub struct HandheldState {
    sr: f32,
    chips: [Chip; 8],
    keys: Keys,
    frames: Frames,
    /// A note came: its frame comes now, not up to a frame later.
    pending: bool,
    env_clock: Frames,
    sweep_clock: Frames,
    waves: Vec<[u8; 32]>,
    /// The noise clocks (shift, ratio, Hz), slowest first.
    noise_rates: Vec<(u8, u8, f32)>,
    pub pitch_bend: f32,
}

fn level(p: &[f32], c: usize) -> u8 {
    let i = [P1_LEVEL, P2_LEVEL, WAVE_LEVEL, NOISE_LEVEL][c];
    p.get(i).map(|v| v.round().clamp(0.0, 15.0) as u8).unwrap_or(0)
}

fn noise_hz(shift: u8, ratio: u8) -> f32 {
    let r = if ratio == 0 { 0.5 } else { ratio as f32 };
    524_288.0 / r / 2f32.powi(shift as i32 + 1)
}

impl HandheldState {
    pub fn new(sr: f32) -> Self {
        let mut rates = Vec::new();
        for s in 0..14u8 {
            for r in 0..8u8 {
                rates.push((s, r, noise_hz(s, r)));
            }
        }
        rates.sort_by(|a, b| a.2.total_cmp(&b.2));
        Self {
            sr,
            chips: [Chip::default(); 8],
            keys: Keys::default(),
            frames: Frames::default(),
            pending: false,
            env_clock: Frames::default(),
            sweep_clock: Frames::default(),
            waves: (0..WAVES.len()).map(wave_table).collect(),
            noise_rates: rates,
            pitch_bend: 0.0,
        }
    }

    fn voices(p: &[f32]) -> usize {
        p.get(VOICES).map(|v| v.round().clamp(1.0, 8.0) as usize).unwrap_or(1)
    }

    fn target(p: &[f32], c: usize, vel: f32) -> u8 {
        if c == 2 {
            return 15;
        }
        (level(p, c) as f32 * (0.35 + 0.65 * vel)).round().clamp(0.0, 15.0) as u8
    }

    pub fn note_on(&mut self, p: &[f32], note: u8, vel: f32) {
        self.keys.press(note, vel);
        self.pending = true;
        let age = self.keys.clock;
        match chip::mode_of(p.get(MODE).copied().unwrap_or(0.0)) {
            LAYER => {
                let n = Self::voices(p);
                let chips = &self.chips;
                let k = chip::free_or_oldest(n, |_| true, |i| chips[i].ch.iter().all(|c| c.idle()), |i| chips[i].ch[0].age).unwrap_or(0);
                for (i, c) in self.chips[k].ch.iter_mut().enumerate() {
                    c.start(note, vel, age, true);
                    c.target = Self::target(p, i, vel);
                }
            }
            SPLIT => {
                let n = Self::voices(p) * 4;
                let chips = &self.chips;
                let slot = chip::free_or_oldest(n, |s| level(p, s % 4) > 0, |s| chips[s / 4].ch[s % 4].idle(), |s| chips[s / 4].ch[s % 4].age);
                if let Some(s) = slot {
                    let c = &mut self.chips[s / 4].ch[s % 4];
                    c.start(note, vel, age, true);
                    c.target = Self::target(p, s % 4, vel);
                }
            }
            _ => {
                let fresh = self.keys.held.len() == 1;
                if fresh {
                    self.keys.arp_restart();
                }
                for (i, c) in self.chips[0].ch.iter_mut().enumerate() {
                    c.start(note, vel, age, fresh);
                    c.target = Self::target(p, i, vel);
                }
            }
        }
    }

    pub fn note_off(&mut self, p: &[f32], note: u8) {
        self.keys.release(note);
        if chip::mode_of(p.get(MODE).copied().unwrap_or(0.0)) == ARP {
            if self.keys.held.is_empty() {
                self.chips[0].ch.iter_mut().for_each(Ch::release);
            }
            return;
        }
        for chip in self.chips.iter_mut() {
            for c in chip.ch.iter_mut() {
                if c.gate && c.note == note {
                    c.release();
                }
            }
        }
    }

    pub fn all_off(&mut self) {
        self.keys.clear();
        for chip in self.chips.iter_mut() {
            for c in chip.ch.iter_mut() {
                c.gate = false;
                c.env = Env::Idle;
                c.vol = 0;
            }
        }
    }

    pub fn visual(&self, out: &mut [f32; crate::engine::INST_VALS]) {
        let k = (0..8).filter(|&i| self.chips[i].ch.iter().any(|c| !c.idle())).max_by_key(|&i| self.chips[i].ch.iter().map(|c| c.age).max().unwrap_or(0)).unwrap_or(0);
        for (i, c) in self.chips[k].ch.iter().enumerate() {
            out[i * 4] = if c.idle() || c.off { 0.0 } else { c.vol as f32 / 15.0 };
            out[i * 4 + 1] = c.note as f32;
            out[i * 4 + 3] = [chip::KIND_PULSE, chip::KIND_PULSE, chip::KIND_WAVE, chip::KIND_NOISE][i];
        }
        out[32] = 4.0;
    }

    /// The driver's frame: arpeggio, vibrato, the registers.
    fn frame(&mut self, p: &[f32]) {
        let get = |i: usize, or: f32| p.get(i).copied().unwrap_or(or);
        let mode = chip::mode_of(get(MODE, 0.0));
        let arp = if mode == ARP { self.keys.arp_frame(get(ARP_SPEED, 2.0).round().clamp(1.0, 8.0) as u32) } else { None };
        let keys_noise = get(NOISE_KEYS, 1.0) >= 0.5;
        let seven = get(NOISE_WIDTH, 0.0) >= 0.5;
        let bend = self.pitch_bend;
        let fixed_noise = noise_hz(get(NOISE_SHIFT, 3.0).round().clamp(0.0, 13.0) as u8, get(NOISE_RATIO, 0.0).round().clamp(0.0, 7.0) as u8);
        for (k, chip) in self.chips.iter_mut().enumerate() {
            if mode == ARP && k > 0 {
                for c in chip.ch.iter_mut() {
                    c.env = Env::Idle;
                    c.vol = 0;
                }
                continue;
            }
            for (i, c) in chip.ch.iter_mut().enumerate() {
                if let (Some((n, v)), true) = (arp, c.gate) {
                    c.note = n;
                    c.vel = v;
                    c.target = Self::target(p, i, v);
                }
                if c.idle() {
                    continue;
                }
                c.frames_on += 1;
                let vib = chip::vibrato(c.frames_on, FRAME, get(VIBRATO, 0.0), get(VIB_RATE, 6.0), get(VIB_DELAY, 0.2));
                let tune = match i {
                    0 => get(P1_TUNE, 0.0) + get(P1_FINE, 0.0) / 100.0,
                    1 => get(P2_TUNE, 0.0) + get(P2_FINE, 0.0) / 100.0,
                    2 => get(WAVE_TUNE, 0.0),
                    _ => 0.0,
                };
                let f = chip::freq(c.note as f32 + tune + vib + bend);
                match i {
                    0 | 1 => c.reg = (2048.0 - 131_072.0 / f).round().clamp(0.0, 2047.0) as u16,
                    2 => c.reg = (2048.0 - 65_536.0 / f).round().clamp(0.0, 2047.0) as u16,
                    _ => {
                        let want = if keys_noise { f * if seven { 127.0 } else { 32.0 } } else { fixed_noise };
                        let best = self.noise_rates.iter().min_by(|a, b| (a.2.ln() - want.ln()).abs().total_cmp(&(b.2.ln() - want.ln()).abs())).map(|x| x.2).unwrap_or(fixed_noise);
                        c.noise_cycles = CLOCK / best;
                    }
                }
            }
        }
    }

    pub fn render(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32]) {
        let n = l.len().min(r.len());
        let sr = self.sr;
        let cycles = CLOCK / sr;
        let get = |i: usize, or: f32| p.get(i).copied().unwrap_or(or);
        let duty = |i: usize| get(i, 2.0).round().clamp(0.0, 3.0) as usize;
        let (d1, d2) = (duty(P1_DUTY), duty(P2_DUTY));
        let pan = |i: usize| match get(i, 0.0).round() as i32 {
            1 => (1.0, 0.0),
            2 => (0.0, 1.0),
            _ => (1.0, 1.0),
        };
        let pans = [pan(P1_PAN), pan(P2_PAN), pan(WAVE_PAN), pan(NOISE_PAN)];
        let wave_shift = [0.0f32, 0.25, 0.5, 1.0][get(WAVE_LEVEL, 0.0).round().clamp(0.0, 3.0) as usize];
        let wave = self.waves[(get(WAVEFORM, 0.0).round().max(0.0) as usize).min(WAVES.len() - 1)];
        let seven = get(NOISE_WIDTH, 0.0) >= 0.5;
        let (attack, decay, release) = (get(ATTACK, 0.0).round().clamp(0.0, 7.0) as u8, get(DECAY, 0.0).round().clamp(0.0, 7.0) as u8, get(RELEASE, 1.0).round().clamp(0.0, 7.0) as u8);
        let sweep = get(P1_SWEEP, 0.0).round() as i32;
        let sweep_shift = get(P1_SHIFT, 3.0).round().clamp(0.0, 7.0) as i32;
        let model = get(MODEL, 0.0).round() as usize;
        let k_hp = match model {
            0 => 0.999_958f32.powf(cycles),
            1 => 0.998_943f32.powf(cycles),
            _ => HighPass::coef(10.0, sr),
        };
        let k_lp = LowPass::coef(get(TREBLE, 20000.0), sr);
        let master = (get(MASTER, 7.0).round().clamp(0.0, 7.0) + 1.0) / 8.0;
        let volume = get(VOLUME, 0.8).clamp(0.0, 1.0);
        let mode = chip::mode_of(get(MODE, 0.0));
        for s in 0..n {
            if self.frames.tick(FRAME, sr) | std::mem::take(&mut self.pending) {
                self.frame(p);
            }
            let env_tick = self.env_clock.tick(64.0, sr);
            let sweep_tick = self.sweep_clock.tick(128.0, sr);
            let (mut out_l, mut out_r) = (0.0, 0.0);
            for (k, chip) in self.chips.iter_mut().enumerate() {
                if mode == ARP && k > 0 {
                    continue;
                }
                if env_tick {
                    for c in chip.ch.iter_mut() {
                        c.env_tick(attack, decay, release);
                    }
                }
                if chip.ch.iter().all(|c| c.idle()) {
                    // The capacitor lets go on its own.
                    for side in 0..2 {
                        chip.lp[side].run(chip.hp[side].run(0.0, k_hp), k_lp);
                    }
                    continue;
                }
                let mut v = [0.0f32; 4];
                for (i, c) in chip.ch.iter_mut().enumerate() {
                    if c.idle() || c.off {
                        continue;
                    }
                    match i {
                        0 | 1 => {
                            // The sweep slides the register (pulse 1 only).
                            if i == 0 && sweep != 0 && sweep_shift > 0 && sweep_tick {
                                if c.sweep_reg < 0 {
                                    c.sweep_reg = c.reg as i32;
                                }
                                c.sweep_count += 1;
                                if c.sweep_count as i32 >= sweep.abs() {
                                    c.sweep_count = 0;
                                    let d = c.sweep_reg >> sweep_shift;
                                    let next = if sweep > 0 { c.sweep_reg + d } else { c.sweep_reg - d };
                                    if next > 2047 {
                                        c.off = true;
                                    } else {
                                        c.sweep_reg = next.max(0);
                                    }
                                }
                            }
                            let reg = if i == 0 && sweep != 0 && c.sweep_reg >= 0 { c.sweep_reg as f32 } else { c.reg as f32 };
                            let seq = DUTY[if i == 0 { d1 } else { d2 }];
                            let pos = &mut c.pos;
                            v[i] = chip::run_steps(&mut c.timer, 4.0 * (2048.0 - reg), cycles, &mut c.cur, || {
                                *pos = (*pos + 1) & 7;
                                seq[*pos as usize] as f32 * 2.0 - 1.0
                            }) * c.vol as f32 / 15.0;
                        }
                        2 => {
                            // The wave's volume: full, a half or a quarter, from what the envelope leaves.
                            let lvl = wave_shift * c.vol as f32 / 15.0;
                            let shift = if lvl > 0.75 {
                                1.0
                            } else if lvl > 0.375 {
                                0.5
                            } else if lvl > 0.125 {
                                0.25
                            } else {
                                0.0
                            };
                            let pos = &mut c.pos;
                            v[i] = chip::run_steps(&mut c.timer, 2.0 * (2048.0 - c.reg as f32), cycles, &mut c.cur, || {
                                *pos = (*pos + 1) & 31;
                                wave[*pos as usize] as f32 / 7.5 - 1.0
                            }) * shift;
                        }
                        _ => {
                            let lfsr = &mut c.lfsr;
                            v[i] = chip::run_steps(&mut c.timer, c.noise_cycles, cycles, &mut c.cur, || {
                                let bit = (*lfsr ^ (*lfsr >> 1)) & 1;
                                *lfsr = (*lfsr >> 1) | (bit << 14);
                                if seven {
                                    *lfsr = (*lfsr & !0x40) | (bit << 6);
                                }
                                ((*lfsr & 1) ^ 1) as f32 * 2.0 - 1.0
                            }) * c.vol as f32 / 15.0;
                        }
                    }
                }
                let (mut cl, mut cr) = (0.0, 0.0);
                for (i, x) in v.iter().enumerate() {
                    cl += x * pans[i].0;
                    cr += x * pans[i].1;
                }
                let scale = master / 4.0;
                out_l += chip.lp[0].run(chip.hp[0].run(cl * scale, k_hp), k_lp);
                out_r += chip.lp[1].run(chip.hp[1].run(cr * scale, k_hp), k_lp);
            }
            l[s] = out_l * 0.45 * volume;
            r[s] = out_r * 0.45 * volume;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 48000.0;

    fn run(st: &mut HandheldState, p: &[f32], secs: f32) -> (Vec<f32>, Vec<f32>) {
        let n = (secs * SR) as usize;
        let (mut ol, mut or) = (Vec::with_capacity(n), Vec::with_capacity(n));
        while ol.len() < n {
            let mut l = vec![0.0; 256];
            let mut r = vec![0.0; 256];
            st.render(p, &mut l, &mut r);
            ol.extend(l);
            or.extend(r);
        }
        (ol, or)
    }

    fn rms(v: &[f32]) -> f32 {
        (v.iter().map(|x| x * x).sum::<f32>() / v.len().max(1) as f32).sqrt()
    }

    fn crossings(v: &[f32]) -> usize {
        v.windows(2).filter(|w| w[0] < 0.0 && w[1] >= 0.0).count()
    }

    #[test]
    fn a_pulse_and_the_wave_play_their_notes() {
        for (name, note, hz) in [("Square Lead", 69u8, 440.0f32), ("Wave Organ", 57, 220.0)] {
            let mut p = preset(name);
            p[MODEL] = 2.0;
            p[WAVEFORM] = 0.0;
            let mut st = HandheldState::new(SR);
            st.note_on(&p, note, 1.0);
            let (l, _) = run(&mut st, &p, 0.5);
            let f = crossings(&l[SR as usize / 4..]) as f32 * 4.0;
            assert!((f - hz).abs() < hz * 0.03, "{name}: {f}");
        }
    }

    #[test]
    fn panned_channels_stay_on_their_side() {
        let mut p = preset("Square Lead");
        p[P1_PAN] = 1.0;
        let mut st = HandheldState::new(SR);
        st.note_on(&p, 60, 1.0);
        let (l, r) = run(&mut st, &p, 0.3);
        assert!(rms(&l) > 0.02 && rms(&r) < 1e-4, "{} {}", rms(&l), rms(&r));
    }

    #[test]
    fn every_preset_sounds_and_stays_sane() {
        for name in preset_names() {
            let p = preset(name);
            let mut st = HandheldState::new(SR);
            for n in [60, 64, 67] {
                st.note_on(&p, n, 0.9);
            }
            let (l, r) = run(&mut st, &p, 0.3);
            assert!(l.iter().chain(&r).all(|x| x.is_finite() && x.abs() < 1.5), "{name}");
            assert!(rms(&l) + rms(&r) > 0.01, "{name}: {}", rms(&l));
            for n in [60, 64, 67] {
                st.note_off(&p, n);
            }
            let (l, _) = run(&mut st, &p, 1.5);
            assert!(rms(&l[l.len() - 4800..]) < 0.01, "{name} rings on");
        }
    }

    #[test]
    fn the_seven_bit_noise_is_a_tone() {
        let p = preset("Noise Tone");
        let mut st = HandheldState::new(SR);
        st.note_on(&p, 57, 1.0);
        let (l, _) = run(&mut st, &p, 0.4);
        // A loop of 127 steps: the signal repeats.
        let x = &l[9600..19200];
        let rate = CLOCK / st.chips[0].ch[3].noise_cycles;
        let period = (127.0 * SR / rate).round() as usize;
        let energy = x.iter().map(|a| a * a).sum::<f32>();
        let c = (period - 1..=period + 1).map(|lag| x.iter().zip(&x[lag..]).map(|(a, b)| a * b).sum::<f32>() / energy).fold(f32::MIN, f32::max);
        assert!(c > 0.5, "{c} at {period}");
    }

    #[test]
    fn the_wave_tables_fill_the_range() {
        for i in 0..WAVES.len() {
            let w = wave_table(i);
            assert!(w.iter().any(|&v| v >= 12), "{}", WAVES[i]);
            assert!(w.iter().all(|&v| v <= 15));
        }
    }
}

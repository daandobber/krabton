//! Micro: the sound chip of the best-selling home computer of the eighties
//! (the early chip and the later one), made from how it works. Three voices, each a 24-bit phase accumulator on the
//! 0.985 MHz clock (so its pitches are its own), with triangle, saw, pulse
//! (any width) and noise (the 23-bit register clocked sixteen times a
//! cycle), or two or three at once; hard sync and ring modulation from the
//! voice before; its envelope, the counter that climbs and falls in 256
//! steps at sixteen rates, curving down as the chip curves it. Then the
//! multimode filter (low, band and high, together or alone) that each
//! voice is routed through or not, its cutoff in eleven bits (the early
//! chip's darker and dirtier curve, or the later one's straight one), resonance in
//! sixteen steps, and the master volume. The vibrato, the pulse width
//! sweep and the arpeggio move 50 times a second, a PAL frame. Played in
//! layers, split over the three voices, or as an arpeggio (see `chip`).

use super::chip::{self, Frames, HighPass, Keys, ARP, LAYER, SPLIT};
use crate::params::{ParamSpec, Unit};

pub const MODE: usize = 0;
pub const VOICES: usize = 1;
pub const MODEL: usize = 2;
/// Each of the three voices has these, from `voice_param(v, ..)`.
pub const V_WAVE: usize = 0;
pub const V_PW: usize = 1;
pub const V_ATTACK: usize = 2;
pub const V_DECAY: usize = 3;
pub const V_SUSTAIN: usize = 4;
pub const V_RELEASE: usize = 5;
pub const V_TUNE: usize = 6;
pub const V_FINE: usize = 7;
pub const V_SYNC: usize = 8;
pub const V_RING: usize = 9;
pub const V_FILTER: usize = 10;
pub const VOICE_PARAMS: usize = 11;
pub const FILTER_MODE: usize = 36;
pub const CUTOFF: usize = 37;
pub const RESONANCE: usize = 38;
pub const ENV_CUTOFF: usize = 39;
pub const V3_OFF: usize = 40;
pub const MASTER: usize = 41;
pub const PWM: usize = 42;
pub const PWM_RATE: usize = 43;
pub const VIBRATO: usize = 44;
pub const VIB_RATE: usize = 45;
pub const VIB_DELAY: usize = 46;
pub const ARP_SPEED: usize = 47;
pub const VOLUME: usize = 48;

pub const fn voice_param(v: usize, k: usize) -> usize {
    3 + v * VOICE_PARAMS + k
}

pub const WAVES: &[&str] = &["Off", "Triangle", "Saw", "Pulse", "Noise", "Tri+Saw", "Tri+Pulse", "Saw+Pulse", "Tri+Saw+Pulse"];
pub const FILTERS: &[&str] = &["Off", "Low", "Band", "High", "Notch", "Low+Band", "Band+High", "All"];
pub const MODELS: &[&str] = &["Early", "Late"];

macro_rules! voice_specs {
    ($g:literal, $w:literal, $wave:expr, $a:expr, $d:expr, $s:expr, $r:expr, $f:literal) => {
        [
            ParamSpec::choice(concat!("Wave ", $w), $g, WAVES, $wave),
            ParamSpec::lin(concat!("Width ", $w), $g, 0.0, 1.0, 0.5, Unit::Percent),
            ParamSpec::int(concat!("Attack ", $w), $g, 0.0, 15.0, $a, Unit::None),
            ParamSpec::int(concat!("Decay ", $w), $g, 0.0, 15.0, $d, Unit::None),
            ParamSpec::int(concat!("Sustain ", $w), $g, 0.0, 15.0, $s, Unit::None),
            ParamSpec::int(concat!("Release ", $w), $g, 0.0, 15.0, $r, Unit::None),
            ParamSpec::int(concat!("Tune ", $w), $g, -24.0, 24.0, 0.0, Unit::Semi),
            ParamSpec::lin(concat!("Fine ", $w), $g, -100.0, 100.0, $f, Unit::Cents),
            ParamSpec::toggle(concat!("Sync ", $w), $g, false),
            ParamSpec::toggle(concat!("Ring ", $w), $g, false),
            ParamSpec::toggle(concat!("Filter ", $w), $g, true),
        ]
    };
}

const V1: [ParamSpec; VOICE_PARAMS] = voice_specs!("Voice 1", "1", 3, 0.0, 9.0, 10.0, 8.0, 0.0);
const V2: [ParamSpec; VOICE_PARAMS] = voice_specs!("Voice 2", "2", 0, 0.0, 9.0, 10.0, 8.0, 7.0);
const V3: [ParamSpec; VOICE_PARAMS] = voice_specs!("Voice 3", "3", 0, 0.0, 9.0, 10.0, 8.0, -7.0);

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::choice("Mode", "Play", chip::MODES, 0),
    ParamSpec::int("Voices", "Play", 1.0, 8.0, 4.0, Unit::None),
    ParamSpec::choice("Model", "Filter", MODELS, 0),
    V1[0], V1[1], V1[2], V1[3], V1[4], V1[5], V1[6], V1[7], V1[8], V1[9], V1[10],
    V2[0], V2[1], V2[2], V2[3], V2[4], V2[5], V2[6], V2[7], V2[8], V2[9], V2[10],
    V3[0], V3[1], V3[2], V3[3], V3[4], V3[5], V3[6], V3[7], V3[8], V3[9], V3[10],
    ParamSpec::choice("Filter Mode", "Filter", FILTERS, 1),
    ParamSpec::lin("Cutoff", "Filter", 0.0, 2047.0, 1200.0, Unit::None),
    ParamSpec::int("Resonance", "Filter", 0.0, 15.0, 6.0, Unit::None),
    ParamSpec::lin("Env Cutoff", "Filter", -1.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::toggle("Voice 3 Off", "Filter", false),
    ParamSpec::int("Master", "Output", 0.0, 15.0, 15.0, Unit::None),
    ParamSpec::lin("PWM", "Play", 0.0, 0.5, 0.0, Unit::Percent),
    ParamSpec::lin("PWM Rate", "Play", 0.05, 8.0, 0.6, Unit::Hz),
    ParamSpec::lin("Vibrato", "Play", 0.0, 100.0, 0.0, Unit::Cents),
    ParamSpec::lin("Vib Rate", "Play", 1.0, 15.0, 5.5, Unit::Hz),
    ParamSpec::lin("Vib Delay", "Play", 0.0, 1.0, 0.25, Unit::Sec),
    ParamSpec::int("Arp Speed", "Play", 1.0, 8.0, 1.0, Unit::None),
    ParamSpec::lin("Volume", "Output", 0.0, 1.0, 0.8, Unit::Percent),
];

pub fn preset_names() -> &'static [&'static str] {
    &["Pulse Lead", "Saw Bass", "Sync Lead", "Ring Bell", "Arpeggio", "Chord Split", "PWM Strings", "Filter Sweep", "Acid", "Noise Drum", "Wobble", "Fat Three", "Clean Pad", "Driving Bass"]
}

pub fn preset(name: &str) -> Vec<f32> {
    let mut p = crate::params::defaults(SPECS);
    let v = voice_param;
    let set: &[(usize, f32)] = match name {
        "Saw Bass" => &[(v(0, V_WAVE), 2.0), (v(0, V_DECAY), 8.0), (v(0, V_SUSTAIN), 6.0), (v(0, V_RELEASE), 4.0), (CUTOFF, 700.0), (RESONANCE, 9.0), (ENV_CUTOFF, 0.35), (v(0, V_TUNE), -12.0)],
        "Sync Lead" => &[(v(0, V_WAVE), 2.0), (v(0, V_SYNC), 1.0), (v(0, V_TUNE), 12.0), (v(2, V_WAVE), 1.0), (v(2, V_FINE), 0.0), (v(2, V_FILTER), 0.0), (V3_OFF, 1.0), (CUTOFF, 1700.0), (VIBRATO, 20.0)],
        "Ring Bell" => &[(v(0, V_WAVE), 1.0), (v(0, V_RING), 1.0), (v(0, V_DECAY), 10.0), (v(0, V_SUSTAIN), 0.0), (v(0, V_RELEASE), 10.0), (v(2, V_WAVE), 1.0), (v(2, V_TUNE), 7.0), (v(2, V_FINE), 30.0), (V3_OFF, 1.0), (FILTER_MODE, 0.0)],
        "Arpeggio" => &[(MODE, 2.0), (v(0, V_WAVE), 3.0), (v(0, V_PW), 0.3), (PWM, 0.15), (ARP_SPEED, 1.0), (CUTOFF, 1500.0)],
        "Chord Split" => &[(MODE, 1.0), (v(1, V_WAVE), 3.0), (v(2, V_WAVE), 3.0), (v(1, V_FINE), 0.0), (v(2, V_FINE), 0.0), (PWM, 0.2), (CUTOFF, 1400.0)],
        "PWM Strings" => &[(v(0, V_ATTACK), 8.0), (v(0, V_SUSTAIN), 12.0), (v(0, V_RELEASE), 9.0), (v(1, V_WAVE), 3.0), (v(1, V_ATTACK), 8.0), (v(1, V_SUSTAIN), 12.0), (v(1, V_RELEASE), 9.0), (PWM, 0.35), (PWM_RATE, 0.4), (CUTOFF, 1300.0), (VIBRATO, 10.0)],
        "Filter Sweep" => &[(v(0, V_WAVE), 2.0), (v(1, V_WAVE), 2.0), (CUTOFF, 300.0), (RESONANCE, 12.0), (ENV_CUTOFF, 0.7), (v(0, V_DECAY), 10.0), (v(0, V_SUSTAIN), 4.0), (v(1, V_DECAY), 10.0), (v(1, V_SUSTAIN), 4.0)],
        "Acid" => &[(MODEL, 1.0), (v(0, V_WAVE), 2.0), (v(0, V_DECAY), 7.0), (v(0, V_SUSTAIN), 0.0), (v(0, V_RELEASE), 3.0), (CUTOFF, 400.0), (RESONANCE, 14.0), (ENV_CUTOFF, 0.6), (v(0, V_TUNE), -12.0)],
        "Noise Drum" => &[(v(0, V_WAVE), 4.0), (v(0, V_DECAY), 6.0), (v(0, V_SUSTAIN), 0.0), (v(0, V_RELEASE), 6.0), (v(0, V_TUNE), 24.0), (CUTOFF, 1800.0), (RESONANCE, 2.0)],
        "Wobble" => &[(v(0, V_WAVE), 7.0), (v(0, V_PW), 0.6), (PWM, 0.3), (PWM_RATE, 3.0), (CUTOFF, 900.0), (RESONANCE, 10.0), (v(0, V_TUNE), -12.0)],
        "Fat Three" => &[(v(0, V_WAVE), 2.0), (v(1, V_WAVE), 2.0), (v(2, V_WAVE), 3.0), (v(2, V_TUNE), -12.0), (CUTOFF, 1500.0), (RESONANCE, 4.0)],
        "Clean Pad" => &[(MODEL, 1.0), (v(0, V_WAVE), 6.0), (v(0, V_ATTACK), 9.0), (v(0, V_RELEASE), 10.0), (v(0, V_SUSTAIN), 13.0), (v(1, V_WAVE), 5.0), (v(1, V_ATTACK), 9.0), (v(1, V_RELEASE), 10.0), (v(1, V_SUSTAIN), 13.0), (CUTOFF, 1000.0), (PWM, 0.2), (VIBRATO, 8.0)],
        "Driving Bass" => &[(v(0, V_WAVE), 3.0), (v(0, V_PW), 0.25), (v(0, V_DECAY), 7.0), (v(0, V_SUSTAIN), 8.0), (v(0, V_RELEASE), 2.0), (v(0, V_TUNE), -12.0), (PWM, 0.2), (PWM_RATE, 1.5), (FILTER_MODE, 0.0)],
        // "Pulse Lead"
        _ => &[(VIBRATO, 18.0), (PWM, 0.1)],
    };
    for &(i, x) in set {
        p[i] = x;
    }
    p
}

/// The PAL clock, and its frames.
const CLOCK: f32 = 985_248.0;
const FRAME: f32 = 50.0;
/// Cycles between the envelope's steps, for the sixteen rates.
const RATES: [f32; 16] = [9.0, 32.0, 63.0, 95.0, 149.0, 220.0, 267.0, 313.0, 392.0, 977.0, 1954.0, 3126.0, 3907.0, 11720.0, 19532.0, 31251.0];

#[derive(Clone, Copy, Default, PartialEq, Debug)]
enum Adsr {
    Attack,
    DecaySustain,
    #[default]
    Release,
}

#[derive(Clone, Copy, Default)]
struct Voice {
    note: u8,
    vel: f32,
    gate: bool,
    age: u64,
    frames_on: u32,
    // The envelope: its counter (0-255), state, and the step counters.
    env: u8,
    state: Adsr,
    rate_acc: f32,
    exp_count: u8,
    // The oscillator: its accumulator as 0 to 1, the frequency register.
    phase: f32,
    freq: u16,
    pw: f32,
    lfsr: u32,
}

impl Voice {
    fn new() -> Self {
        Self { lfsr: 0x7F_FFF8, ..Self::default() }
    }

    fn active(&self) -> bool {
        self.gate || self.env > 0
    }

    fn start(&mut self, note: u8, vel: f32, age: u64, retrigger: bool) {
        self.note = note;
        self.vel = vel;
        self.age = age;
        if retrigger || !self.gate {
            self.gate = true;
            self.state = Adsr::Attack;
            self.frames_on = 0;
        }
    }

    fn release(&mut self) {
        self.gate = false;
        self.state = Adsr::Release;
    }

    /// The decay and release fall in a curve: slower the lower they are.
    fn exp_period(&self) -> u8 {
        match self.env {
            0 => 1,
            1..=6 => 30,
            7..=14 => 16,
            15..=26 => 8,
            27..=54 => 4,
            55..=93 => 2,
            _ => 1,
        }
    }

    fn env_clock(&mut self, cycles: f32, a: usize, d: usize, s: u8, r: usize) {
        self.rate_acc += cycles;
        loop {
            let rate = RATES[match self.state {
                Adsr::Attack => a,
                Adsr::DecaySustain => d,
                Adsr::Release => r,
            }];
            if self.rate_acc < rate {
                break;
            }
            self.rate_acc -= rate;
            match self.state {
                Adsr::Attack => {
                    self.exp_count = 0;
                    self.env = self.env.saturating_add(1);
                    if self.env == 255 {
                        self.state = Adsr::DecaySustain;
                    }
                }
                Adsr::DecaySustain | Adsr::Release => {
                    self.exp_count += 1;
                    if self.exp_count >= self.exp_period() {
                        self.exp_count = 0;
                        let floor = if self.state == Adsr::DecaySustain { s } else { 0 };
                        if self.env > floor {
                            self.env -= 1;
                        }
                    }
                    if self.env == 0 {
                        self.rate_acc = 0.0;
                        break;
                    }
                }
            }
        }
    }
}

fn poly_blep(t: f32, dt: f32) -> f32 {
    if t < dt {
        let t = t / dt;
        t + t - t * t - 1.0
    } else if t > 1.0 - dt {
        let t = (t - 1.0) / dt;
        t * t + t + t + 1.0
    } else {
        0.0
    }
}

#[derive(Clone, Copy, Default)]
struct Svf {
    ic1: f32,
    ic2: f32,
}

#[derive(Clone, Copy)]
struct Chip {
    v: [Voice; 3],
    svf: Svf,
    dc: HighPass,
    pwm_phase: f32,
}

impl Default for Chip {
    fn default() -> Self {
        Self { v: [Voice::new(); 3], svf: Svf::default(), dc: HighPass::default(), pwm_phase: 0.0 }
    }
}

pub struct MicroState {
    sr: f32,
    chips: [Chip; 8],
    keys: Keys,
    frames: Frames,
    /// A note came: its frame comes now, not up to a frame later.
    pending: bool,
    pub pitch_bend: f32,
}

fn wave_of(p: &[f32], v: usize) -> u8 {
    let w = p.get(voice_param(v, V_WAVE)).map(|x| x.round().clamp(0.0, 8.0) as usize).unwrap_or(0);
    // Triangle 1, saw 2, pulse 4, noise 8.
    [0, 1, 2, 4, 8, 3, 5, 6, 7][w]
}

/// The cutoff register as Hz: the early chip's curve or the later one's line.
pub fn cutoff_hz(reg: f32, model: usize) -> f32 {
    let x = (reg / 2047.0).clamp(0.0, 1.0);
    if model == 0 {
        200.0 + 17800.0 * x.powf(2.2)
    } else {
        30.0 + 12000.0 * x
    }
}

impl MicroState {
    pub fn new(sr: f32) -> Self {
        Self { sr, chips: [Chip::default(); 8], keys: Keys::default(), frames: Frames::default(), pending: false, pitch_bend: 0.0 }
    }

    fn voices(p: &[f32]) -> usize {
        p.get(VOICES).map(|v| v.round().clamp(1.0, 8.0) as usize).unwrap_or(1)
    }

    pub fn note_on(&mut self, p: &[f32], note: u8, vel: f32) {
        self.keys.press(note, vel);
        self.pending = true;
        let age = self.keys.clock;
        match chip::mode_of(p.get(MODE).copied().unwrap_or(0.0)) {
            LAYER => {
                let n = Self::voices(p);
                let chips = &self.chips;
                let k = chip::free_or_oldest(n, |_| true, |i| chips[i].v.iter().all(|v| !v.active()), |i| chips[i].v[0].age).unwrap_or(0);
                for (i, v) in self.chips[k].v.iter_mut().enumerate() {
                    if wave_of(p, i) != 0 {
                        v.start(note, vel, age, true);
                    }
                }
            }
            SPLIT => {
                let n = Self::voices(p) * 3;
                let chips = &self.chips;
                let slot = chip::free_or_oldest(n, |s| wave_of(p, s % 3) != 0, |s| !chips[s / 3].v[s % 3].active(), |s| chips[s / 3].v[s % 3].age);
                if let Some(s) = slot {
                    self.chips[s / 3].v[s % 3].start(note, vel, age, true);
                }
            }
            _ => {
                let fresh = self.keys.held.len() == 1;
                if fresh {
                    self.keys.arp_restart();
                }
                for (i, v) in self.chips[0].v.iter_mut().enumerate() {
                    if wave_of(p, i) != 0 {
                        v.start(note, vel, age, fresh);
                    }
                }
            }
        }
    }

    pub fn note_off(&mut self, p: &[f32], note: u8) {
        self.keys.release(note);
        if chip::mode_of(p.get(MODE).copied().unwrap_or(0.0)) == ARP {
            if self.keys.held.is_empty() {
                self.chips[0].v.iter_mut().for_each(Voice::release);
            }
            return;
        }
        for chip in self.chips.iter_mut() {
            for v in chip.v.iter_mut() {
                if v.gate && v.note == note {
                    v.release();
                }
            }
        }
    }

    pub fn all_off(&mut self) {
        self.keys.clear();
        for chip in self.chips.iter_mut() {
            for v in chip.v.iter_mut() {
                v.gate = false;
                v.state = Adsr::Release;
                v.env = 0;
            }
        }
    }

    pub fn visual(&self, out: &mut [f32; crate::engine::INST_VALS]) {
        let k = (0..8).filter(|&i| self.chips[i].v.iter().any(|v| v.active())).max_by_key(|&i| self.chips[i].v.iter().map(|v| v.age).max().unwrap_or(0)).unwrap_or(0);
        for (i, v) in self.chips[k].v.iter().enumerate() {
            out[i * 4] = v.env as f32 / 255.0;
            out[i * 4 + 1] = v.note as f32;
            out[i * 4 + 2] = v.pw;
            out[i * 4 + 3] = chip::KIND_SID;
        }
        out[32] = 3.0;
    }

    /// The player's frame: arpeggio, vibrato, pulse width sweep, registers.
    fn frame(&mut self, p: &[f32]) {
        let get = |i: usize, or: f32| p.get(i).copied().unwrap_or(or);
        let mode = chip::mode_of(get(MODE, 0.0));
        let arp = if mode == ARP { self.keys.arp_frame(get(ARP_SPEED, 1.0).round().clamp(1.0, 8.0) as u32) } else { None };
        let bend = self.pitch_bend;
        let (pwm, pwm_rate) = (get(PWM, 0.0), get(PWM_RATE, 0.6));
        for chip in self.chips.iter_mut() {
            chip.pwm_phase = (chip.pwm_phase + pwm_rate / FRAME).fract();
            // A triangle sweep, as the players swept it.
            let sweep = pwm * (1.0 - 4.0 * (chip.pwm_phase - 0.5).abs());
            for (i, v) in chip.v.iter_mut().enumerate() {
                if let (Some((n, vel)), true) = (arp, v.gate) {
                    v.note = n;
                    v.vel = vel;
                }
                if !v.active() {
                    continue;
                }
                v.frames_on += 1;
                let vib = chip::vibrato(v.frames_on, FRAME, get(VIBRATO, 0.0), get(VIB_RATE, 5.5), get(VIB_DELAY, 0.25));
                let pitch = v.note as f32 + get(voice_param(i, V_TUNE), 0.0) + get(voice_param(i, V_FINE), 0.0) / 100.0 + vib + bend;
                v.freq = (chip::freq(pitch) * 16_777_216.0 / CLOCK).round().clamp(0.0, 65535.0) as u16;
                // Twelve bits of pulse width.
                v.pw = ((get(voice_param(i, V_PW), 0.5) + sweep).clamp(0.0, 1.0) * 4095.0).round() / 4096.0;
            }
        }
    }

    pub fn render(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32]) {
        let n = l.len().min(r.len());
        let sr = self.sr;
        let cycles = CLOCK / sr;
        let get = |i: usize, or: f32| p.get(i).copied().unwrap_or(or);
        let mode = chip::mode_of(get(MODE, 0.0));
        let model = get(MODEL, 0.0).round().clamp(0.0, 1.0) as usize;
        let waves = [wave_of(p, 0), wave_of(p, 1), wave_of(p, 2)];
        let rate = |i: usize, k: usize| get(voice_param(i, k), 0.0).round().clamp(0.0, 15.0) as usize;
        let adsr: [(usize, usize, u8, usize); 3] = std::array::from_fn(|i| (rate(i, V_ATTACK), rate(i, V_DECAY), rate(i, V_SUSTAIN) as u8 * 0x11, rate(i, V_RELEASE)));
        let sync: [bool; 3] = std::array::from_fn(|i| get(voice_param(i, V_SYNC), 0.0) >= 0.5);
        let ring: [bool; 3] = std::array::from_fn(|i| get(voice_param(i, V_RING), 0.0) >= 0.5);
        let routed: [bool; 3] = std::array::from_fn(|i| get(voice_param(i, V_FILTER), 1.0) >= 0.5);
        let fmode = [0u8, 1, 2, 4, 5, 3, 6, 7][get(FILTER_MODE, 1.0).round().clamp(0.0, 7.0) as usize];
        let cutoff = get(CUTOFF, 1200.0);
        let env_cut = get(ENV_CUTOFF, 0.0);
        let res = get(RESONANCE, 6.0).round().clamp(0.0, 15.0);
        let k_res = 1.0 / (0.707 * 2f32.powf(res / 15.0 * if model == 0 { 1.6 } else { 2.0 }));
        let v3_off = get(V3_OFF, 0.0) >= 0.5;
        let master = get(MASTER, 15.0).round().clamp(0.0, 15.0) / 15.0;
        let volume = get(VOLUME, 0.8).clamp(0.0, 1.0);
        let k_dc = HighPass::coef(12.0, sr);
        for s in 0..n {
            if self.frames.tick(FRAME, sr) | std::mem::take(&mut self.pending) {
                self.frame(p);
            }
            let mut out = 0.0;
            for (k, chip) in self.chips.iter_mut().enumerate() {
                if mode == ARP && k > 0 {
                    continue;
                }
                if chip.v.iter().all(|v| !v.active()) {
                    continue;
                }
                // The oscillators, each synced or rung by the voice before.
                let mut msb_prev = [false; 3];
                for (i, v) in chip.v.iter().enumerate() {
                    msb_prev[i] = v.phase >= 0.5;
                }
                let mut rose = [false; 3];
                let mut rose_at = [0.0f32; 3];
                for (i, v) in chip.v.iter_mut().enumerate() {
                    let dt = v.freq as f32 * CLOCK / 16_777_216.0 / sr;
                    let before = v.phase;
                    v.phase += dt;
                    if v.phase >= 1.0 {
                        v.phase -= 1.0;
                    }
                    // The noise register shifts on bit 19: sixteen times a cycle.
                    if waves[i] & 8 != 0 {
                        let mut steps = ((v.phase * 16.0).floor() - (before * 16.0).floor()) as i32;
                        if steps < 0 {
                            steps += 16;
                        }
                        for _ in 0..steps {
                            let bit = ((v.lfsr >> 22) ^ (v.lfsr >> 17)) & 1;
                            v.lfsr = ((v.lfsr << 1) | bit) & 0x7F_FFFF;
                        }
                    }
                    let msb = v.phase >= 0.5;
                    rose[i] = msb && !msb_prev[i];
                    if rose[i] && dt > 0.0 {
                        rose_at[i] = ((v.phase - 0.5) / dt).clamp(0.0, 1.0);
                    }
                }
                for i in 0..3 {
                    let src = (i + 2) % 3;
                    if sync[i] && rose[src] {
                        let dt = chip.v[i].freq as f32 * CLOCK / 16_777_216.0 / sr;
                        chip.v[i].phase = (rose_at[src] * dt).fract();
                    }
                }
                let msb_now: [bool; 3] = std::array::from_fn(|i| chip.v[i].phase >= 0.5);
                let mut filt = 0.0;
                let mut direct = 0.0;
                for i in 0..3 {
                    let (a, d, sus, rel) = adsr[i];
                    let v = &mut chip.v[i];
                    v.env_clock(cycles, a, d, sus, rel);
                    if waves[i] == 0 || v.env == 0 {
                        continue;
                    }
                    let dt = v.freq as f32 * CLOCK / 16_777_216.0 / sr;
                    let ph = v.phase;
                    let msb = msb_now[i] ^ (ring[i] && msb_now[(i + 2) % 3]);
                    let tri = if msb { 1.0 - (2.0 * ph).fract() } else { (2.0 * ph).fract() };
                    let saw = ph;
                    let pulse = if ph >= v.pw { 1.0 } else { 0.0 };
                    let lf = v.lfsr;
                    let bits = [20, 18, 14, 11, 9, 5, 2, 0];
                    let noise = bits.iter().enumerate().fold(0u32, |acc, (k, &b)| acc | (((lf >> b) & 1) << (7 - k))) as f32 / 255.0;
                    let w = match waves[i] {
                        1 => tri,
                        2 => saw - 0.5 * poly_blep(ph, dt),
                        4 => pulse + 0.5 * poly_blep((ph - v.pw).rem_euclid(1.0), dt) - 0.5 * poly_blep(ph, dt),
                        8 => noise,
                        // Two or three at once: the chip lets the lower bits win.
                        c => {
                            let q = |x: f32| (x.clamp(0.0, 1.0) * 4095.0) as u32;
                            let mut acc = 0xFFFu32;
                            if c & 1 != 0 {
                                acc &= q(tri);
                            }
                            if c & 2 != 0 {
                                acc &= q(saw);
                            }
                            if c & 4 != 0 {
                                acc &= q(pulse);
                            }
                            acc as f32 / 4095.0
                        }
                    };
                    let x = (w - 0.5) * 2.0 * v.env as f32 / 255.0 * (0.5 + 0.5 * v.vel);
                    if routed[i] && fmode != 0 {
                        filt += x;
                    } else if !(i == 2 && v3_off) {
                        direct += x;
                    }
                }
                // The filter: a state variable, its cutoff moved by voice 1's envelope.
                let mut y = direct;
                if fmode != 0 {
                    let reg = cutoff + env_cut * 2047.0 * chip.v[0].env as f32 / 255.0;
                    let fc = cutoff_hz(reg, model).min(sr * 0.45);
                    let g = (std::f32::consts::PI * fc / sr).tan();
                    let a1 = 1.0 / (1.0 + g * (g + k_res));
                    let (a2, a3) = (g * a1, g * g * a1);
                    // The early chip grits up when driven.
                    let v0 = if model == 0 { (filt * 0.8).tanh() / 0.8 } else { filt };
                    let f = &mut chip.svf;
                    let v3 = v0 - f.ic2;
                    let v1 = a1 * f.ic1 + a2 * v3;
                    let v2 = f.ic2 + a2 * f.ic1 + a3 * v3;
                    f.ic1 = 2.0 * v1 - f.ic1;
                    f.ic2 = 2.0 * v2 - f.ic2;
                    if f.ic1.abs() < 1e-20 {
                        f.ic1 = 0.0;
                    }
                    if f.ic2.abs() < 1e-20 {
                        f.ic2 = 0.0;
                    }
                    let (lp, bp, hp) = (v2, v1, v0 - k_res * v1 - v2);
                    if fmode & 1 != 0 {
                        y += lp;
                    }
                    if fmode & 2 != 0 {
                        y += bp;
                    }
                    if fmode & 4 != 0 {
                        y += hp;
                    }
                }
                out += chip.dc.run(y * master, k_dc);
            }
            let y = out * 0.17 * volume;
            l[s] = y;
            r[s] = y;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 48000.0;

    fn run(st: &mut MicroState, p: &[f32], secs: f32) -> Vec<f32> {
        let n = (secs * SR) as usize;
        let mut out = Vec::with_capacity(n);
        while out.len() < n {
            let mut l = vec![0.0; 256];
            let mut r = vec![0.0; 256];
            st.render(p, &mut l, &mut r);
            out.extend(l);
        }
        out
    }

    fn rms(v: &[f32]) -> f32 {
        (v.iter().map(|x| x * x).sum::<f32>() / v.len().max(1) as f32).sqrt()
    }

    fn crossings(v: &[f32]) -> usize {
        v.windows(2).filter(|w| w[0] < 0.0 && w[1] >= 0.0).count()
    }

    /// How bright a signal is: the energy of its steps against its own.
    fn roughness(v: &[f32]) -> f32 {
        let d: f32 = v.windows(2).map(|w| (w[1] - w[0]).powi(2)).sum();
        d / v.iter().map(|x| x * x).sum::<f32>().max(1e-9)
    }

    #[test]
    fn a_pulse_plays_its_note() {
        let mut p = preset("Pulse Lead");
        p[VIBRATO] = 0.0;
        p[PWM] = 0.0;
        p[FILTER_MODE] = 0.0;
        let mut st = MicroState::new(SR);
        st.note_on(&p, 69, 1.0);
        let out = run(&mut st, &p, 0.5);
        let f = crossings(&out[SR as usize / 4..]) as f32 * 4.0;
        assert!((f - 440.0).abs() < 8.0, "{f}");
    }

    #[test]
    fn the_attack_is_as_fast_as_the_chip() {
        let p = preset("Pulse Lead");
        let mut st = MicroState::new(SR);
        st.note_on(&p, 60, 1.0);
        // Attack 0: 2 ms to the top.
        run(&mut st, &p, 0.003);
        assert!(st.chips[0].v[0].env >= 250 || st.chips[0].v[0].state == Adsr::DecaySustain);
        // Decay 9 to sustain 10 (170): down, but no further.
        run(&mut st, &p, 1.5);
        assert_eq!(st.chips[0].v[0].env, 170);
        st.note_off(&p, 60);
        run(&mut st, &p, 3.0);
        assert_eq!(st.chips[0].v[0].env, 0);
    }

    #[test]
    fn the_filter_takes_the_edge_off() {
        let mut p = preset("Saw Bass");
        p[ENV_CUTOFF] = 0.0;
        p[RESONANCE] = 0.0;
        p[voice_param(0, V_SUSTAIN)] = 15.0;
        p[CUTOFF] = 150.0;
        let mut st = MicroState::new(SR);
        st.note_on(&p, 48, 1.0);
        let dark = roughness(&run(&mut st, &p, 0.3)[4800..]);
        p[FILTER_MODE] = 0.0;
        let mut st = MicroState::new(SR);
        st.note_on(&p, 48, 1.0);
        let bright = roughness(&run(&mut st, &p, 0.3)[4800..]);
        assert!(dark < bright * 0.5, "{dark} {bright}");
    }

    #[test]
    fn sync_and_ring_change_the_sound() {
        let base = preset("Ring Bell");
        let mut st = MicroState::new(SR);
        st.note_on(&base, 60, 1.0);
        let rung = run(&mut st, &base, 0.2);
        let mut plain = base.clone();
        plain[voice_param(0, V_RING)] = 0.0;
        let mut st = MicroState::new(SR);
        st.note_on(&plain, 60, 1.0);
        let not = run(&mut st, &plain, 0.2);
        let diff: f32 = rung.iter().zip(&not).map(|(a, b)| (a - b).abs()).sum();
        assert!(diff > 1.0, "{diff}");
    }

    #[test]
    fn every_preset_sounds_and_stays_sane() {
        for name in preset_names() {
            let p = preset(name);
            let mut st = MicroState::new(SR);
            for n in [48, 52, 55] {
                st.note_on(&p, n, 0.9);
            }
            let out = run(&mut st, &p, 0.4);
            assert!(out.iter().all(|x| x.is_finite() && x.abs() < 1.5), "{name}");
            assert!(rms(&out) > 0.01, "{name}: {}", rms(&out));
            for n in [48, 52, 55] {
                st.note_off(&p, n);
            }
            let tail = run(&mut st, &p, 4.0);
            assert!(rms(&tail[tail.len() - 4800..]) < 0.01, "{name} rings on");
        }
    }

    #[test]
    fn the_arpeggio_walks_the_chord_on_frames() {
        let p = preset("Arpeggio");
        let mut st = MicroState::new(SR);
        for n in [60, 63, 67] {
            st.note_on(&p, n, 1.0);
        }
        let mut seen = Vec::new();
        for _ in 0..6 {
            run(&mut st, &p, 1.0 / 50.0);
            seen.push(st.chips[0].v[0].note);
        }
        let distinct: std::collections::BTreeSet<u8> = seen.iter().copied().collect();
        assert_eq!(distinct.len(), 3, "{seen:?}");
    }
}

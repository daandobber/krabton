//! NES: the 2A03, the sound of the Nintendo Entertainment System (and the
//! Famicom), made from how the chip works. Two pulse channels (duty 12.5,
//! 25, 50 or 75 per cent, sixteen volume steps, the hardware sweep that
//! bends the pitch up or down), the triangle (32 steps, no volume at all:
//! on or off) and noise (a 15-bit shift register, long or the short metallic
//! loop, sixteen pitches). Pitches come from 11-bit timers on the 1.79 MHz
//! clock, so high notes are a little out, as on the real thing; the four
//! are mixed as the chip's resistors mix them and filtered as the console
//! does. Volume, vibrato and arpeggios move sixty times a second, as the
//! game's music driver moved them. Played in layers, split over the
//! channels, or as an arpeggio (see `chip`).

use super::chip::{self, FrameEnv, Frames, HighPass, Keys, LowPass, ARP, LAYER, SPLIT};
use crate::params::{ParamSpec, Unit};

pub const MODE: usize = 0;
pub const VOICES: usize = 1;
pub const P1_LEVEL: usize = 2;
pub const P1_DUTY: usize = 3;
pub const P1_TUNE: usize = 4;
pub const P1_FINE: usize = 5;
pub const P1_SWEEP: usize = 6;
pub const P1_SHIFT: usize = 7;
pub const P2_LEVEL: usize = 8;
pub const P2_DUTY: usize = 9;
pub const P2_TUNE: usize = 10;
pub const P2_FINE: usize = 11;
pub const P2_SWEEP: usize = 12;
pub const P2_SHIFT: usize = 13;
pub const TRI_ON: usize = 14;
pub const TRI_TUNE: usize = 15;
pub const NOISE_LEVEL: usize = 16;
pub const NOISE_SHORT: usize = 17;
pub const NOISE_TUNE: usize = 18;
pub const ATTACK: usize = 19;
pub const DECAY: usize = 20;
pub const SUSTAIN: usize = 21;
pub const RELEASE: usize = 22;
pub const VIBRATO: usize = 23;
pub const VIB_RATE: usize = 24;
pub const VIB_DELAY: usize = 25;
pub const ARP_SPEED: usize = 26;
pub const CONSOLE: usize = 27;
pub const VOLUME: usize = 28;

pub const DUTIES: &[&str] = &["12.5%", "25%", "50%", "75%"];
pub const CONSOLES: &[&str] = &["NES", "Famicom", "Raw"];

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::choice("Mode", "Play", chip::MODES, 0),
    ParamSpec::int("Voices", "Play", 1.0, 8.0, 4.0, Unit::None),
    ParamSpec::int("Pulse 1", "Pulse 1", 0.0, 15.0, 15.0, Unit::None),
    ParamSpec::choice("Duty 1", "Pulse 1", DUTIES, 2),
    ParamSpec::int("Tune 1", "Pulse 1", -24.0, 24.0, 0.0, Unit::Semi),
    ParamSpec::lin("Fine 1", "Pulse 1", -100.0, 100.0, 0.0, Unit::Cents),
    ParamSpec::int("Sweep 1", "Pulse 1", -7.0, 7.0, 0.0, Unit::None),
    ParamSpec::int("Shift 1", "Pulse 1", 0.0, 7.0, 3.0, Unit::None),
    ParamSpec::int("Pulse 2", "Pulse 2", 0.0, 15.0, 0.0, Unit::None),
    ParamSpec::choice("Duty 2", "Pulse 2", DUTIES, 1),
    ParamSpec::int("Tune 2", "Pulse 2", -24.0, 24.0, 0.0, Unit::Semi),
    ParamSpec::lin("Fine 2", "Pulse 2", -100.0, 100.0, 8.0, Unit::Cents),
    ParamSpec::int("Sweep 2", "Pulse 2", -7.0, 7.0, 0.0, Unit::None),
    ParamSpec::int("Shift 2", "Pulse 2", 0.0, 7.0, 3.0, Unit::None),
    ParamSpec::toggle("Triangle", "Triangle", false),
    ParamSpec::int("Tri Tune", "Triangle", -24.0, 24.0, -12.0, Unit::Semi),
    ParamSpec::int("Noise", "Noise", 0.0, 15.0, 0.0, Unit::None),
    ParamSpec::toggle("Short Noise", "Noise", false),
    ParamSpec::int("Noise Tune", "Noise", -8.0, 8.0, 0.0, Unit::None),
    ParamSpec::log("Attack", "Envelope", 1.0, 4000.0, 1.0, Unit::Ms),
    ParamSpec::log("Decay", "Envelope", 1.0, 4000.0, 400.0, Unit::Ms),
    ParamSpec::int("Sustain", "Envelope", 0.0, 15.0, 10.0, Unit::None),
    ParamSpec::log("Release", "Envelope", 1.0, 4000.0, 120.0, Unit::Ms),
    ParamSpec::lin("Vibrato", "Play", 0.0, 100.0, 0.0, Unit::Cents),
    ParamSpec::lin("Vib Rate", "Play", 1.0, 15.0, 6.0, Unit::Hz),
    ParamSpec::lin("Vib Delay", "Play", 0.0, 1.0, 0.2, Unit::Sec),
    ParamSpec::int("Arp Speed", "Play", 1.0, 8.0, 2.0, Unit::None),
    ParamSpec::choice("Console", "Output", CONSOLES, 0),
    ParamSpec::lin("Volume", "Output", 0.0, 1.0, 0.8, Unit::Percent),
];

pub fn preset_names() -> &'static [&'static str] {
    &["Lead", "Thin Lead", "Two Pulses", "Triangle Bass", "Arpeggio", "Chord Split", "Laser", "Coin", "Noise Hat", "Metal Noise", "Echo Lead", "Brass", "Full Chip"]
}

pub fn preset(name: &str) -> Vec<f32> {
    let mut p = crate::params::defaults(SPECS);
    let set: &[(usize, f32)] = match name {
        "Thin Lead" => &[(P1_DUTY, 0.0), (VIBRATO, 25.0), (DECAY, 600.0), (SUSTAIN, 9.0)],
        "Two Pulses" => &[(P2_LEVEL, 12.0), (P2_DUTY, 1.0), (P2_FINE, 10.0), (P1_DUTY, 2.0), (VIBRATO, 15.0)],
        "Triangle Bass" => &[(P1_LEVEL, 0.0), (TRI_ON, 1.0), (TRI_TUNE, 0.0), (RELEASE, 40.0), (CONSOLE, 1.0)],
        "Arpeggio" => &[(MODE, 2.0), (P1_DUTY, 1.0), (ARP_SPEED, 2.0), (SUSTAIN, 12.0), (DECAY, 800.0)],
        "Chord Split" => &[(MODE, 1.0), (P2_LEVEL, 15.0), (P2_DUTY, 2.0), (P2_FINE, 0.0), (TRI_ON, 1.0), (TRI_TUNE, 0.0), (SUSTAIN, 11.0)],
        "Laser" => &[(P1_DUTY, 0.0), (P1_SWEEP, 1.0), (P1_SHIFT, 2.0), (DECAY, 300.0), (SUSTAIN, 0.0), (RELEASE, 30.0)],
        "Coin" => &[(P1_DUTY, 1.0), (P1_SWEEP, -2.0), (P1_SHIFT, 4.0), (DECAY, 250.0), (SUSTAIN, 0.0)],
        "Noise Hat" => &[(P1_LEVEL, 0.0), (NOISE_LEVEL, 12.0), (NOISE_TUNE, -6.0), (DECAY, 70.0), (SUSTAIN, 0.0), (RELEASE, 30.0)],
        "Metal Noise" => &[(P1_LEVEL, 0.0), (NOISE_LEVEL, 13.0), (NOISE_SHORT, 1.0), (DECAY, 500.0), (SUSTAIN, 4.0)],
        "Echo Lead" => &[(P2_LEVEL, 5.0), (P2_DUTY, 2.0), (P2_FINE, 0.0), (VIBRATO, 20.0), (RELEASE, 600.0)],
        "Brass" => &[(P1_DUTY, 2.0), (P2_LEVEL, 10.0), (P2_DUTY, 3.0), (P2_TUNE, 12.0), (ATTACK, 60.0), (SUSTAIN, 12.0), (VIBRATO, 18.0)],
        "Full Chip" => &[(P2_LEVEL, 12.0), (P2_DUTY, 1.0), (P2_TUNE, 12.0), (TRI_ON, 1.0), (TRI_TUNE, -12.0), (NOISE_LEVEL, 4.0), (DECAY, 250.0), (SUSTAIN, 8.0)],
        _ => &[],
    };
    for &(i, v) in set {
        p[i] = v;
    }
    p
}

/// The NTSC CPU clock, and the frames of the music driver.
const CPU: f32 = 1_789_773.0;
const FRAME: f32 = 60.0;
const SEQ: [[u8; 8]; 4] = [[0, 1, 0, 0, 0, 0, 0, 0], [0, 1, 1, 0, 0, 0, 0, 0], [0, 1, 1, 1, 1, 0, 0, 0], [1, 0, 0, 1, 1, 1, 1, 1]];
const TRI: [u8; 32] = [15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];
/// The noise periods (CPU cycles), NTSC.
const NOISE_PERIODS: [u16; 16] = [4, 8, 16, 32, 64, 96, 128, 160, 202, 254, 380, 508, 762, 1016, 2034, 4068];

#[derive(Clone, Copy, Default)]
struct Ch {
    note: u8,
    vel: f32,
    gate: bool,
    age: u64,
    env: FrameEnv,
    frames_on: u32,
    /// What the music driver set this frame: the timer and the volume (0-15).
    period: u16,
    vol: u8,
    muted: bool,
    /// The sweep: how far it has moved the timer, and its divider.
    sweep: i32,
    sweep_div: u8,
    timer: f32,
    pos: u8,
    cur: f32,
    lfsr: u16,
}

impl Ch {
    fn start(&mut self, note: u8, vel: f32, age: u64, retrigger: bool) {
        self.note = note;
        self.vel = vel;
        self.age = age;
        self.gate = true;
        if self.env.idle() {
            // From silence, the triangle starts at its bottom (no click).
            self.pos = 16;
        }
        if retrigger || self.env.idle() {
            self.env.gate(true);
            self.frames_on = 0;
            self.sweep = 0;
            self.sweep_div = 0;
        }
    }
}

#[derive(Clone, Copy, Default)]
struct Chip {
    ch: [Ch; 4],
    hp1: HighPass,
    hp2: HighPass,
    lp: LowPass,
}

pub struct NesState {
    sr: f32,
    chips: [Chip; 8],
    keys: Keys,
    frames: Frames,
    /// A note came: its frame comes now, not up to a frame later.
    pending: bool,
    half: bool,
    pub pitch_bend: f32,
}

fn level(p: &[f32], c: usize) -> f32 {
    let i = [P1_LEVEL, P2_LEVEL, TRI_ON, NOISE_LEVEL][c];
    let v = p.get(i).copied().unwrap_or(0.0);
    if c == 2 {
        if v >= 0.5 {
            15.0
        } else {
            0.0
        }
    } else {
        v.round().clamp(0.0, 15.0)
    }
}

impl NesState {
    pub fn new(sr: f32) -> Self {
        let mut chips = [Chip::default(); 8];
        for c in chips.iter_mut() {
            c.ch[3].lfsr = 1;
        }
        Self { sr, chips, keys: Keys::default(), frames: Frames::default(), pending: false, half: false, pitch_bend: 0.0 }
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
                let k = chip::free_or_oldest(n, |_| true, |i| chips[i].ch.iter().all(|c| c.env.idle()), |i| chips[i].ch[0].age).unwrap_or(0);
                for c in self.chips[k].ch.iter_mut() {
                    c.start(note, vel, age, true);
                }
            }
            SPLIT => {
                let n = Self::voices(p) * 4;
                let chips = &self.chips;
                let slot = chip::free_or_oldest(n, |s| level(p, s % 4) > 0.0, |s| chips[s / 4].ch[s % 4].env.idle(), |s| chips[s / 4].ch[s % 4].age);
                if let Some(s) = slot {
                    self.chips[s / 4].ch[s % 4].start(note, vel, age, true);
                }
            }
            _ => {
                // The arpeggio: a new chord starts it over; a key added keeps it going.
                let fresh = self.keys.held.len() == 1;
                if fresh {
                    self.keys.arp_restart();
                }
                for c in self.chips[0].ch.iter_mut() {
                    c.start(note, vel, age, fresh);
                }
            }
        }
    }

    pub fn note_off(&mut self, p: &[f32], note: u8) {
        self.keys.release(note);
        match chip::mode_of(p.get(MODE).copied().unwrap_or(0.0)) {
            ARP => {
                if self.keys.held.is_empty() {
                    for c in self.chips[0].ch.iter_mut() {
                        c.gate = false;
                        c.env.gate(false);
                    }
                }
            }
            _ => {
                for chip in self.chips.iter_mut() {
                    for c in chip.ch.iter_mut() {
                        if c.gate && c.note == note {
                            c.gate = false;
                            c.env.gate(false);
                        }
                    }
                }
            }
        }
    }

    pub fn all_off(&mut self) {
        self.keys.clear();
        for chip in self.chips.iter_mut() {
            for c in chip.ch.iter_mut() {
                c.gate = false;
                c.env = FrameEnv::default();
                c.vol = 0;
            }
        }
    }

    pub fn visual(&self, out: &mut [f32; crate::engine::INST_VALS]) {
        // The chip played last (or still sounding).
        let k = (0..8).filter(|&i| self.chips[i].ch.iter().any(|c| !c.env.idle())).max_by_key(|&i| self.chips[i].ch.iter().map(|c| c.age).max().unwrap_or(0)).unwrap_or(0);
        for (i, c) in self.chips[k].ch.iter().enumerate() {
            let on = !c.env.idle() && !c.muted;
            out[i * 4] = if on { c.vol as f32 / 15.0 } else { 0.0 };
            out[i * 4 + 1] = c.note as f32;
            out[i * 4 + 3] = [chip::KIND_PULSE, chip::KIND_PULSE, chip::KIND_TRIANGLE, chip::KIND_NOISE][i];
        }
        out[32] = 4.0;
    }

    /// The driver's frame: envelopes, vibrato, arpeggio, sweeps, timers.
    fn frame(&mut self, p: &[f32]) {
        let get = |i: usize, or: f32| p.get(i).copied().unwrap_or(or);
        let mode = chip::mode_of(get(MODE, 0.0));
        let (a, d, s, r) = (get(ATTACK, 1.0) / 1000.0, get(DECAY, 400.0) / 1000.0, get(SUSTAIN, 10.0).clamp(0.0, 15.0) / 15.0, get(RELEASE, 120.0) / 1000.0);
        let (a, r) = (if a < 0.004 { 0.0 } else { a }, if r < 0.004 { 0.0 } else { r });
        let arp = if mode == ARP { self.keys.arp_frame(get(ARP_SPEED, 2.0).round().clamp(1.0, 8.0) as u32) } else { None };
        let bend = self.pitch_bend;
        // The sweep units run twice a frame (on the half frames).
        self.half = !self.half;
        for (k, chip) in self.chips.iter_mut().enumerate() {
            if mode == ARP && k > 0 {
                for c in chip.ch.iter_mut() {
                    c.env = FrameEnv::default();
                    c.vol = 0;
                }
                continue;
            }
            for (i, c) in chip.ch.iter_mut().enumerate() {
                if let (Some((n, v)), true) = (arp, c.gate) {
                    c.note = n;
                    c.vel = v;
                }
                c.env.frame(a, d, s, r, FRAME);
                if c.env.idle() {
                    c.vol = 0;
                    continue;
                }
                c.frames_on += 1;
                let vib = chip::vibrato(c.frames_on, FRAME, get(VIBRATO, 0.0), get(VIB_RATE, 6.0), get(VIB_DELAY, 0.2));
                let (tune, fine) = match i {
                    0 => (get(P1_TUNE, 0.0), get(P1_FINE, 0.0)),
                    1 => (get(P2_TUNE, 0.0), get(P2_FINE, 0.0)),
                    2 => (get(TRI_TUNE, 0.0), 0.0),
                    _ => (get(NOISE_TUNE, 0.0), 0.0),
                };
                let pitch = c.note as f32 + tune + fine / 100.0 + vib + bend;
                let lvl = level(p, i);
                // Volume: sixteen steps (the triangle has none).
                c.vol = if i == 2 {
                    if lvl > 0.0 && c.env.level > 0.0 {
                        15
                    } else {
                        0
                    }
                } else {
                    (lvl * c.env.level * (0.35 + 0.65 * c.vel)).round().clamp(0.0, 15.0) as u8
                };
                match i {
                    0 | 1 => {
                        let base = (CPU / (16.0 * chip::freq(pitch)) - 1.0).round().clamp(0.0, 2047.0) as i32;
                        let (sw, shift) = if i == 0 { (get(P1_SWEEP, 0.0), get(P1_SHIFT, 3.0)) } else { (get(P2_SWEEP, 0.0), get(P2_SHIFT, 3.0)) };
                        let (sw, shift) = (sw.round() as i32, shift.round().clamp(0.0, 7.0) as i32);
                        let mut t = base + c.sweep;
                        if sw != 0 && shift > 0 && self.half {
                            c.sweep_div += 1;
                            if c.sweep_div as i32 >= sw.abs() {
                                c.sweep_div = 0;
                                let change = t >> shift;
                                // Down in pitch (a longer period) or up; pulse 1 takes one more off.
                                let target = if sw > 0 { t + change } else { t - change - if i == 0 { 1 } else { 0 } };
                                if (8..=0x7FF).contains(&target) {
                                    c.sweep = target - base;
                                    t = target;
                                }
                            }
                        }
                        c.muted = !(8..=0x7FF).contains(&t);
                        c.period = t.clamp(0, 0x7FF) as u16;
                    }
                    2 => {
                        let t = (CPU / (32.0 * chip::freq(pitch)) - 1.0).round().clamp(0.0, 2047.0) as u16;
                        c.muted = t < 2;
                        c.period = t;
                    }
                    _ => {
                        // Sixteen pitches over six octaves of keys.
                        let idx = 15.0 - (pitch - 36.0) / 3.0;
                        c.period = idx.round().clamp(0.0, 15.0) as u16;
                        c.muted = false;
                    }
                }
            }
        }
    }

    pub fn render(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32]) {
        let n = l.len().min(r.len());
        let sr = self.sr;
        let cycles = CPU / sr;
        let duty = |i: usize| p.get(i).map(|v| (v.round().clamp(0.0, 3.0)) as usize).unwrap_or(2);
        let (d1, d2) = (duty(P1_DUTY), duty(P2_DUTY));
        let console = p.get(CONSOLE).map(|v| v.round() as usize).unwrap_or(0);
        let volume = p.get(VOLUME).copied().unwrap_or(0.8).clamp(0.0, 1.0);
        let (k90, k440, k37) = (HighPass::coef(90.0, sr), HighPass::coef(440.0, sr), HighPass::coef(37.0, sr));
        let klp = LowPass::coef(14000.0, sr);
        let k_dc = HighPass::coef(10.0, sr);
        let short = p.get(NOISE_SHORT).copied().unwrap_or(0.0) >= 0.5;
        for s in 0..n {
            if self.frames.tick(FRAME, sr) | std::mem::take(&mut self.pending) {
                self.frame(p);
            }
            let mut out = 0.0;
            for chip in self.chips.iter_mut() {
                if chip.ch.iter().all(|c| c.vol == 0 && c.env.idle()) {
                    continue;
                }
                let mut v = [0.0f32; 4];
                for (i, c) in chip.ch.iter_mut().enumerate() {
                    let silent = c.vol == 0 || c.muted;
                    match i {
                        0 | 1 => {
                            let seq = SEQ[if i == 0 { d1 } else { d2 }];
                            let vol = if silent { 0.0 } else { c.vol as f32 };
                            let period = 2.0 * (c.period as f32 + 1.0);
                            let pos = &mut c.pos;
                            v[i] = chip::run_steps(&mut c.timer, period, cycles, &mut c.cur, || {
                                *pos = (*pos + 1) & 7;
                                seq[*pos as usize] as f32
                            }) * vol;
                        }
                        2 => {
                            // Too high to hear, the triangle stays where it stopped.
                            if c.vol == 0 {
                                v[i] = 0.0;
                            } else if c.muted {
                                v[i] = TRI[c.pos as usize & 31] as f32;
                            } else {
                                let pos = &mut c.pos;
                                v[i] = chip::run_steps(&mut c.timer, c.period as f32 + 1.0, cycles, &mut c.cur, || {
                                    *pos = (*pos + 1) & 31;
                                    TRI[*pos as usize] as f32
                                });
                            }
                        }
                        _ => {
                            let vol = if silent { 0.0 } else { c.vol as f32 };
                            let period = NOISE_PERIODS[c.period as usize & 15] as f32;
                            let lfsr = &mut c.lfsr;
                            v[i] = chip::run_steps(&mut c.timer, period, cycles, &mut c.cur, || {
                                let other = if short { 6 } else { 1 };
                                let bit = (*lfsr ^ (*lfsr >> other)) & 1;
                                *lfsr = (*lfsr >> 1) | (bit << 14);
                                if *lfsr & 1 == 0 {
                                    1.0
                                } else {
                                    0.0
                                }
                            }) * vol;
                        }
                    }
                }
                // The chip's resistor mix.
                let pulses = v[0] + v[1];
                let pulse_out = if pulses > 1e-6 { 95.88 / (8128.0 / pulses + 100.0) } else { 0.0 };
                let tn = v[2] / 8227.0 + v[3] / 12241.0;
                let tnd_out = if tn > 1e-9 { 159.79 / (1.0 / tn + 100.0) } else { 0.0 };
                let x = pulse_out + tnd_out;
                let y = match console {
                    0 => chip.lp.run(chip.hp2.run(chip.hp1.run(x, k90), k440), klp),
                    1 => chip.lp.run(chip.hp1.run(x, k37), klp),
                    _ => chip.hp1.run(x, k_dc),
                };
                out += y;
            }
            let y = out * 2.4 * volume;
            l[s] = y;
            r[s] = y;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 48000.0;

    fn run(st: &mut NesState, p: &[f32], secs: f32) -> Vec<f32> {
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

    #[test]
    fn a_pulse_plays_its_note() {
        let mut p = preset("Lead");
        p[CONSOLE] = 2.0;
        let mut st = NesState::new(SR);
        st.note_on(&p, 69, 1.0);
        let out = run(&mut st, &p, 0.5);
        let f = crossings(&out[SR as usize / 4..]) as f32 * 4.0;
        assert!((f - 440.0).abs() < 10.0, "{f}");
    }

    #[test]
    fn every_preset_sounds_and_stays_sane() {
        for name in preset_names() {
            let p = preset(name);
            let mut st = NesState::new(SR);
            st.note_on(&p, 60, 1.0);
            st.note_on(&p, 64, 0.8);
            st.note_on(&p, 67, 0.8);
            let out = run(&mut st, &p, 0.3);
            assert!(out.iter().all(|x| x.is_finite() && x.abs() < 1.5), "{name}");
            assert!(rms(&out) > 0.005, "{name}: {}", rms(&out));
            st.note_off(&p, 60);
            st.note_off(&p, 64);
            st.note_off(&p, 67);
            let tail = run(&mut st, &p, 1.5);
            assert!(rms(&tail[tail.len() - 4800..]) < 0.01, "{name} rings on");
        }
    }

    #[test]
    fn the_sweep_bends_the_pitch() {
        let mut p = preset("Lead");
        p[CONSOLE] = 2.0;
        p[P1_SWEEP] = 1.0;
        p[P1_SHIFT] = 4.0;
        p[SUSTAIN] = 15.0;
        let mut st = NesState::new(SR);
        st.note_on(&p, 72, 1.0);
        let out = run(&mut st, &p, 0.6);
        let early = crossings(&out[..4800]);
        let late = crossings(&out[out.len() - 4800..]);
        // Up in period: down in pitch.
        assert!(late < early, "{early} {late}");
    }

    #[test]
    fn split_gives_each_note_a_channel() {
        let p = preset("Chord Split");
        let mut st = NesState::new(SR);
        st.note_on(&p, 60, 1.0);
        st.note_on(&p, 64, 1.0);
        st.note_on(&p, 67, 1.0);
        let notes: Vec<u8> = st.chips[0].ch[..3].iter().map(|c| c.note).collect();
        assert_eq!(notes, vec![60, 64, 67]);
    }

    #[test]
    fn the_arpeggio_walks_the_chord() {
        let p = preset("Arpeggio");
        let mut st = NesState::new(SR);
        st.note_on(&p, 60, 1.0);
        st.note_on(&p, 64, 1.0);
        st.note_on(&p, 67, 1.0);
        let mut seen = std::collections::BTreeSet::new();
        for _ in 0..40 {
            run(&mut st, &p, 1.0 / 60.0);
            seen.insert(st.chips[0].ch[0].note);
        }
        assert_eq!(seen.len(), 3, "{seen:?}");
    }

    #[test]
    fn the_triangle_has_no_volume() {
        let mut p = preset("Triangle Bass");
        p[CONSOLE] = 2.0;
        let mut st = NesState::new(SR);
        st.note_on(&p, 45, 0.2);
        let soft = rms(&run(&mut st, &p, 0.3)[4800..]);
        let mut st = NesState::new(SR);
        st.note_on(&p, 45, 1.0);
        let hard = rms(&run(&mut st, &p, 0.3)[4800..]);
        assert!((soft - hard).abs() < 0.02 * hard, "{soft} {hard}");
    }
}

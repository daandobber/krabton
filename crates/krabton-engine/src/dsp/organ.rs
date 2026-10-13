//! Organ: a tonewheel organ. Nine drawbars of sine tonewheels (the top
//! wheels fold back an octave), single-triggered percussion on the second or
//! third harmonic, key click, the scanner vibrato and chorus, a tube-style
//! overdrive and a rotating speaker. Original code.

use super::effects::FxCtx;
use super::fx::rotary::{self, Rotary};
use super::fx::DelayLine;
use super::{db_to_gain, decay_coef, fast_tanh, midi_to_freq, OnePole, Rng};
use crate::params::{ParamSpec, Unit};
use std::f32::consts::TAU;

/// Footages as multiples of the 8' fundamental.
const HARMONICS: [f32; 9] = [0.5, 1.5, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 8.0];
pub const DRAWBAR_NAMES: [&str; 9] = ["16'", "5 1/3'", "8'", "4'", "2 2/3'", "2'", "1 3/5'", "1 1/3'", "1'"];

pub const PERC: usize = 9;
pub const PERC_HARMONIC: usize = 10;
pub const PERC_DECAY: usize = 11;
pub const PERC_SOFT: usize = 12;
pub const VIBRATO: usize = 13;
pub const CLICK: usize = 14;
pub const DRIVE: usize = 15;
pub const ROTARY: usize = 16;
pub const VOLUME: usize = 17;

pub const VIBRATOS: &[&str] = &["Off", "V1", "V2", "V3", "C1", "C2", "C3"];

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::int("16'", "Drawbars", 0.0, 8.0, 8.0, Unit::None),
    ParamSpec::int("5 1/3'", "Drawbars", 0.0, 8.0, 8.0, Unit::None),
    ParamSpec::int("8'", "Drawbars", 0.0, 8.0, 8.0, Unit::None),
    ParamSpec::int("4'", "Drawbars", 0.0, 8.0, 0.0, Unit::None),
    ParamSpec::int("2 2/3'", "Drawbars", 0.0, 8.0, 0.0, Unit::None),
    ParamSpec::int("2'", "Drawbars", 0.0, 8.0, 0.0, Unit::None),
    ParamSpec::int("1 3/5'", "Drawbars", 0.0, 8.0, 0.0, Unit::None),
    ParamSpec::int("1 1/3'", "Drawbars", 0.0, 8.0, 0.0, Unit::None),
    ParamSpec::int("1'", "Drawbars", 0.0, 8.0, 0.0, Unit::None),
    ParamSpec::toggle("Percussion", "Percussion", false),
    ParamSpec::choice("Harmonic", "Percussion", &["Second", "Third"], 1),
    ParamSpec::choice("Decay", "Percussion", &["Fast", "Slow"], 0),
    ParamSpec::choice("Volume", "Percussion", &["Normal", "Soft"], 0),
    ParamSpec::choice("Vibrato", "Tone", VIBRATOS, 0),
    ParamSpec::lin("Key Click", "Tone", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::lin("Drive", "Tone", 0.0, 1.0, 0.15, Unit::Percent),
    ParamSpec::choice("Rotary", "Output", &["Off", "Slow", "Fast"], 1),
    ParamSpec::lin("Volume", "Output", -24.0, 12.0, 0.0, Unit::Db),
];

pub fn preset_names() -> &'static [&'static str] {
    &[
        "Full Organ",
        "Jazz Trio",
        "Jimmy's Blues",
        "Green Onions",
        "Gospel",
        "Rock Organ",
        "Whiter Shade",
        "Reggae Bubble",
        "Soft Flutes",
        "Ballad",
        "Theatre",
        "Cathedral",
        "Clean Bass Pedals",
    ]
}

pub fn preset(name: &str) -> Vec<f32> {
    let mut p = crate::params::defaults(SPECS);
    let mut bars = |reg: &str| {
        for (i, c) in reg.chars().take(9).enumerate() {
            p[i] = c.to_digit(10).unwrap_or(0) as f32;
        }
    };
    let extra: &[(usize, f32)] = match name {
        "Full Organ" => {
            bars("888888888");
            &[(VIBRATO, 6.0), (ROTARY, 2.0), (DRIVE, 0.3), (VOLUME, -6.0)]
        }
        "Jazz Trio" => {
            bars("888000000");
            &[(PERC, 1.0), (VIBRATO, 6.0), (CLICK, 0.4)]
        }
        "Jimmy's Blues" => {
            bars("888800000");
            &[(PERC, 1.0), (VIBRATO, 6.0), (CLICK, 0.5), (DRIVE, 0.25)]
        }
        "Green Onions" => {
            bars("886000000");
            &[(VIBRATO, 6.0), (DRIVE, 0.3), (ROTARY, 0.0)]
        }
        "Gospel" => {
            bars("888800008");
            &[(VIBRATO, 6.0), (ROTARY, 2.0), (DRIVE, 0.35), (VOLUME, -3.0)]
        }
        "Rock Organ" => {
            bars("888000000");
            &[(VIBRATO, 6.0), (ROTARY, 2.0), (DRIVE, 0.75), (CLICK, 0.6), (VOLUME, -3.0)]
        }
        "Whiter Shade" => {
            bars("688600000");
            &[(VIBRATO, 6.0), (ROTARY, 1.0), (DRIVE, 0.1)]
        }
        "Reggae Bubble" => {
            bars("008800000");
            &[(PERC, 1.0), (PERC_HARMONIC, 0.0), (CLICK, 0.6), (ROTARY, 0.0)]
        }
        "Soft Flutes" => {
            bars("008400000");
            &[(VIBRATO, 3.0), (CLICK, 0.05), (DRIVE, 0.0), (VOLUME, 3.0)]
        }
        "Ballad" => {
            bars("838000000");
            &[(VIBRATO, 6.0), (ROTARY, 1.0), (CLICK, 0.15), (DRIVE, 0.05)]
        }
        "Theatre" => {
            bars("868868868");
            &[(VIBRATO, 3.0), (ROTARY, 2.0), (VOLUME, -5.0)]
        }
        "Cathedral" => {
            bars("800888088");
            &[(VIBRATO, 0.0), (ROTARY, 0.0), (CLICK, 0.0), (DRIVE, 0.0), (VOLUME, -7.0)]
        }
        "Clean Bass Pedals" => {
            bars("880000000");
            &[(ROTARY, 0.0), (CLICK, 0.2), (DRIVE, 0.0)]
        }
        _ => &[],
    };
    for &(i, v) in extra {
        p[i] = v;
    }
    p
}

const MAX_VOICES: usize = 24;
const TABLE: usize = 2048;

#[derive(Clone, Copy, Default)]
struct Voice {
    note: u8,
    gate: bool,
    active: bool,
    phase: [f32; 9],
    amp: f32,
    perc: f32,
    perc_phase: f32,
    click: f32,
    age: u64,
}

pub struct OrganState {
    /// Pitch bend in semitones (a wheel, a controller's triggers), set by the engine.
    pub pitch_bend: f32,
    voices: [Voice; MAX_VOICES],
    sine: Vec<f32>,
    sr: f32,
    counter: u64,
    held: usize,
    rng: Rng,
    click_tone: OnePole,
    scanner: DelayLine,
    scan_phase: f32,
    rotary: Box<Rotary>,
    mono: Vec<f32>,
    r: Vec<f32>,
    /// Dry level of the tone cabinet before the rotary, for the DC blocker.
    dc: f32,
}

impl OrganState {
    pub fn new(sr: f32) -> Self {
        let sine = (0..=TABLE).map(|i| (i as f32 / TABLE as f32 * TAU).sin()).collect();
        Self {
            pitch_bend: 0.0,
            voices: [Voice::default(); MAX_VOICES],
            sine,
            sr,
            counter: 0,
            held: 0,
            rng: Rng::new(0x0E6A),
            click_tone: OnePole::default(),
            scanner: DelayLine::new((sr * 0.005) as usize + 8),
            scan_phase: 0.0,
            rotary: Box::new(Rotary::new(sr)),
            mono: vec![0.0; 4096],
            r: vec![0.0; 4096],
            dc: 0.0,
        }
    }

    pub fn note_on(&mut self, _p: &[f32], note: u8, _vel: f32) {
        // Percussion only on the first key of a phrase (single trigger).
        let perc = if self.held == 0 { 1.0 } else { 0.0 };
        self.held += 1;
        self.counter += 1;
        let pick = self
            .voices
            .iter()
            .position(|v| v.active && v.note == note)
            .or_else(|| self.voices.iter().position(|v| !v.active))
            .or_else(|| self.voices.iter().enumerate().min_by_key(|(_, v)| v.age).map(|(i, _)| i))
            .unwrap_or(0);
        let v = &mut self.voices[pick];
        if !v.active {
            v.phase = [0.0; 9];
        }
        v.note = note;
        v.gate = true;
        v.active = true;
        v.perc = perc;
        v.click = 1.0;
        v.age = self.counter;
    }

    pub fn note_off(&mut self, _p: &[f32], note: u8) {
        for v in self.voices.iter_mut().filter(|v| v.gate && v.note == note) {
            v.gate = false;
            v.click = v.click.max(0.5);
            self.held = self.held.saturating_sub(1);
        }
    }

    /// For the display: [0] horn and [1] drum position in their turn (0..1),
    /// [2] the strongest percussion and [3] the loudest voice (0..1).
    pub fn visual(&self, out: &mut [f32; crate::engine::INST_VALS]) {
        let (h, d) = self.rotary.phases();
        out[0] = h;
        out[1] = d;
        let on = || self.voices.iter().filter(|v| v.active);
        out[2] = on().map(|v| v.perc).fold(0.0, f32::max);
        out[3] = on().map(|v| v.amp).fold(0.0, f32::max);
    }

    pub fn all_off(&mut self) {
        for v in self.voices.iter_mut() {
            v.gate = false;
        }
        self.held = 0;
    }

    #[inline]
    fn sin(&self, phase: f32) -> f32 {
        let x = phase * TABLE as f32;
        let i = (x as usize).min(TABLE - 1);
        let f = x - i as f32;
        self.sine[i] + (self.sine[i + 1] - self.sine[i]) * f
    }

    pub fn render(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32]) {
        if p.len() < SPECS.len() {
            return;
        }
        let total = l.len().min(r.len());
        let chunk = self.mono.len();
        let mut done = 0;
        while done < total {
            let n = chunk.min(total - done);
            self.render_chunk(p, &mut l[done..done + n], &mut r[done..done + n]);
            done += n;
        }
    }

    fn render_chunk(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32]) {
        let n = l.len().min(r.len()).min(self.mono.len());
        let sr = self.sr;
        let perc_on = p[PERC] >= 0.5;
        // With percussion on, the 1' drawbar is taken for it, as on the B3.
        let mut levels = [0.0f32; 9];
        for (k, lv) in levels.iter_mut().enumerate() {
            let bar = p[k].round().clamp(0.0, 8.0);
            // Each drawbar step is about 3 dB.
            *lv = if bar <= 0.0 || (perc_on && k == 8) { 0.0 } else { db_to_gain(-3.0 * (8.0 - bar)) };
        }
        let perc_h = if p[PERC_HARMONIC] >= 0.5 { 3.0 } else { 2.0 };
        let perc_coef = decay_coef(if p[PERC_DECAY] >= 0.5 { 1.0 } else { 0.2 }, sr);
        let perc_gain = if p[PERC_SOFT] >= 0.5 { 0.5 } else { 1.0 };
        let click_amt = p[CLICK].clamp(0.0, 1.0);
        let click_coef = decay_coef(0.004, sr);
        let attack = 1.0 - (-1.0 / (0.002 * sr)).exp();
        let release = 1.0 - (-1.0 / (0.008 * sr)).exp();
        self.click_tone.set(3500.0, sr);
        self.mono[..n].fill(0.0);
        let mut active_any = false;
        for vi in 0..MAX_VOICES {
            if !self.voices[vi].active {
                continue;
            }
            active_any = true;
            let base = midi_to_freq(self.voices[vi].note as f32 + self.pitch_bend);
            let mut inc = [0.0f32; 9];
            for (k, f) in inc.iter_mut().enumerate() {
                let mut hz = base * HARMONICS[k];
                // The top tonewheels fold back an octave; the lowest ones up.
                while hz > 5900.0 {
                    hz *= 0.5;
                }
                while hz < 32.0 {
                    hz *= 2.0;
                }
                *f = hz / sr;
            }
            let perc_inc = (base * perc_h).min(0.45 * sr) / sr;
            for i in 0..n {
                let mut v = self.voices[vi];
                let target = if v.gate { 1.0 } else { 0.0 };
                v.amp += (target - v.amp) * if v.gate { attack } else { release };
                let mut s = 0.0;
                for k in 0..9 {
                    v.phase[k] = super::fx::wrap(v.phase[k] + inc[k]);
                    if levels[k] > 0.0 {
                        s += self.sin(v.phase[k]) * levels[k];
                    }
                }
                if perc_on && v.perc > 1e-4 {
                    v.perc_phase = super::fx::wrap(v.perc_phase + perc_inc);
                    s += self.sin(v.perc_phase) * v.perc * perc_gain * 1.2;
                    v.perc *= perc_coef;
                }
                let click = if v.click > 1e-3 { self.rng.bipolar() * v.click * click_amt } else { 0.0 };
                v.click *= click_coef;
                self.mono[i] += (s * v.amp + click) * 0.12;
                if !v.gate && v.amp < 1e-4 {
                    v.active = false;
                    v.amp = 0.0;
                }
                self.voices[vi] = v;
                if !self.voices[vi].active {
                    break;
                }
            }
        }
        if !active_any && self.dc.abs() < 1e-6 && p[ROTARY].round() as i32 == 0 {
            return;
        }
        let vib = p[VIBRATO].round().clamp(0.0, 6.0) as usize;
        let depth_ms = [0.0, 0.25, 0.5, 0.9, 0.25, 0.5, 0.9][vib];
        let chorus = vib >= 4;
        let drive = p[DRIVE].clamp(0.0, 1.0);
        let pre = 1.0 + 9.0 * drive;
        let post = 1.0 / pre.powf(0.6);
        for i in 0..n {
            let x = self.mono[i];
            let mut y = x;
            if vib > 0 {
                self.scan_phase = super::fx::wrap(self.scan_phase + 6.9 / sr);
                self.scanner.write(x);
                let d = 2.0 + depth_ms * 0.001 * sr * (1.0 + (self.scan_phase * TAU).sin());
                let wet = self.scanner.read(d);
                y = if chorus { 0.5 * (x + wet) } else { wet };
            }
            if drive > 0.0 {
                // A tube stage: a little asymmetric.
                y = fast_tanh(y * pre + 0.1 * drive) * post;
                self.dc += (y - self.dc) * 0.0005;
                y -= self.dc;
            }
            self.mono[i] = y;
        }
        let gain = db_to_gain(p[VOLUME]);
        let speed = p[ROTARY].round() as i32;
        self.r[..n].copy_from_slice(&self.mono[..n]);
        if speed > 0 {
            let mut rp = crate::params::defaults(rotary::SPECS);
            rp[rotary::SPEED] = if speed >= 2 { 2.0 } else { 1.0 };
            rp[rotary::DRIVE] = 0.0;
            let ctx = FxCtx { sr, bpm: 120.0, beat: 0.0, playing: false };
            let (a, b) = (&mut self.mono[..n], &mut self.r[..n]);
            self.rotary.process(&rp, a, b, &ctx);
        }
        for i in 0..n {
            l[i] += self.mono[i] * gain;
            r[i] += self.r[i] * gain;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_preset_sounds_and_stops() {
        for name in preset_names() {
            let p = preset(name);
            let mut o = OrganState::new(48000.0);
            for n in [48, 55, 60, 64] {
                o.note_on(&p, n, 0.8);
            }
            let (mut l, mut r) = (vec![0.0; 24000], vec![0.0; 24000]);
            for (cl, cr) in l.chunks_mut(512).zip(r.chunks_mut(512)) {
                o.render(&p, cl, cr);
            }
            let peak = l.iter().chain(&r).fold(0.0f32, |m, x| m.max(x.abs()));
            eprintln!("{name:<18} peak {peak:.3}");
            assert!(l.iter().all(|x| x.is_finite()) && peak > 0.05 && peak < 3.0, "{name}: {peak}");
            for n in [48, 55, 60, 64] {
                o.note_off(&p, n);
            }
            let (mut l, mut r) = (vec![0.0; 9600], vec![0.0; 9600]);
            o.render(&p, &mut l, &mut r);
            assert!(o.voices.iter().all(|v| !v.active), "{name}: voices stop");
        }
    }

    #[test]
    fn the_8_foot_drawbar_plays_the_note() {
        let mut p = preset("Full Organ");
        for (k, v) in p.iter_mut().enumerate().take(9) {
            *v = if k == 2 { 8.0 } else { 0.0 };
        }
        p[VIBRATO] = 0.0;
        p[ROTARY] = 0.0;
        p[DRIVE] = 0.0;
        p[CLICK] = 0.0;
        let mut o = OrganState::new(48000.0);
        o.note_on(&p, 69, 0.8);
        let (mut l, mut r) = (vec![0.0; 9600], vec![0.0; 9600]);
        o.render(&p, &mut l, &mut r);
        let crossings = l[4800..].windows(2).filter(|w| w[0] <= 0.0 && w[1] > 0.0).count();
        assert!((crossings as f32 - 44.0).abs() <= 1.0, "{crossings}");
    }
}

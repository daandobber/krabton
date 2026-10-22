//! Coast: a west-coast voice. A principal oscillator (sine, triangle, saw or
//! square) is phase modulated by a second oscillator (FM, at a ratio of the
//! note or at a frequency of its own, and AM), then folded back on itself
//! (Timbre, with Symmetry for the even harmonics), and sent through a low
//! pass gate: a filter and an amplifier opened together by a light and a
//! photocell, which open at once and close slowly, the louder the faster, so
//! a note struck rings out like a drum or a marimba. The contour that opens
//! the gate strikes it (a ping, the gate's own afterglow is the note),
//! rises and falls (Transient), or holds while the key is held (Sustained);
//! it can also open the timbre and the FM. Played by Touch it is at home.

use super::{db_to_gain, midi_to_freq, pan_gains, DcBlock, Rng};
use crate::params::{ParamSpec, Unit};
use std::f32::consts::{PI, TAU};

pub const WAVE: usize = 0;
pub const TIMBRE: usize = 1;
pub const SYMMETRY: usize = 2;
pub const RATIO: usize = 3;
pub const MOD_WAVE: usize = 4;
pub const FM: usize = 5;
pub const AM: usize = 6;
pub const KEYED: usize = 7;
pub const MOD_FREQ: usize = 8;
pub const CONTOUR: usize = 9;
pub const ATTACK: usize = 10;
pub const DECAY: usize = 11;
pub const ENV_TIMBRE: usize = 12;
pub const ENV_FM: usize = 13;
pub const GATE: usize = 14;
pub const RESPONSE: usize = 15;
pub const BRIGHT: usize = 16;
pub const NOISE: usize = 17;
pub const GLIDE: usize = 18;
pub const OCTAVE: usize = 19;
pub const VOICES: usize = 20;
pub const VELOCITY: usize = 21;
pub const WIDTH: usize = 22;
pub const VOLUME: usize = 23;

pub const WAVES: &[&str] = &["Sine", "Triangle", "Saw", "Square"];
pub const CONTOURS: &[&str] = &["Strike", "Transient", "Sustained"];
pub const GATES: &[&str] = &["Combo", "Lowpass", "Gain"];
const MAX_VOICES: usize = 8;

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::choice("Wave", "Oscillator", WAVES, 1),
    ParamSpec::lin("Timbre", "Oscillator", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::lin("Symmetry", "Oscillator", -1.0, 1.0, 0.0, Unit::None),
    ParamSpec::log("Ratio", "Modulation", 0.25, 8.0, 2.0, Unit::None),
    ParamSpec::choice("Mod Wave", "Modulation", WAVES, 0),
    ParamSpec::lin("FM", "Modulation", 0.0, 1.0, 0.2, Unit::Percent),
    ParamSpec::lin("AM", "Modulation", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::toggle("Keyed", "Modulation", true),
    ParamSpec::log("Mod Freq", "Modulation", 0.1, 2000.0, 5.0, Unit::Hz),
    ParamSpec::choice("Contour", "Contour", CONTOURS, 0),
    ParamSpec::log("Attack", "Contour", 0.001, 2.0, 0.003, Unit::Sec),
    ParamSpec::log("Decay", "Contour", 0.01, 6.0, 0.5, Unit::Sec),
    ParamSpec::lin("Env Timbre", "Contour", -1.0, 1.0, 0.35, Unit::None),
    ParamSpec::lin("Env FM", "Contour", 0.0, 1.0, 0.2, Unit::Percent),
    ParamSpec::choice("Gate", "Gate", GATES, 0),
    ParamSpec::log("Response", "Gate", 0.03, 3.0, 0.35, Unit::Sec),
    ParamSpec::lin("Brightness", "Gate", 0.0, 1.0, 0.75, Unit::Percent),
    ParamSpec::lin("Noise", "Oscillator", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::log("Glide", "Play", 0.001, 2.0, 0.001, Unit::Sec),
    ParamSpec::int("Octave", "Play", -2.0, 2.0, 0.0, Unit::None),
    ParamSpec::int("Voices", "Play", 1.0, 8.0, 6.0, Unit::None),
    ParamSpec::lin("Velocity", "Play", 0.0, 1.0, 0.7, Unit::Percent),
    ParamSpec::lin("Width", "Output", 0.0, 1.0, 0.35, Unit::Percent),
    ParamSpec::lin("Volume", "Output", -30.0, 6.0, -6.0, Unit::Db),
];

pub fn preset_names() -> &'static [&'static str] {
    &["Bongo", "Marimba", "Struck Bell", "Wood", "Folded Lead", "Clang", "Bubble", "Swell", "Low Thump", "Dust", "Ring Mod", "Glass"]
}

pub fn preset(name: &str) -> Vec<f32> {
    let mut p = crate::params::defaults(SPECS);
    let set: &[(usize, f32)] = match name {
        "Bongo" => &[(WAVE, 1.0), (TIMBRE, 0.2), (RATIO, 1.0), (FM, 0.1), (CONTOUR, 0.0), (RESPONSE, 0.25), (BRIGHT, 0.7), (ENV_TIMBRE, 0.3)],
        "Marimba" => &[(WAVE, 0.0), (TIMBRE, 0.1), (RATIO, 4.0), (FM, 0.15), (CONTOUR, 0.0), (RESPONSE, 0.6), (BRIGHT, 0.6), (ENV_FM, 0.3)],
        "Struck Bell" => &[(WAVE, 0.0), (TIMBRE, 0.25), (RATIO, 3.5), (FM, 0.35), (CONTOUR, 0.0), (RESPONSE, 2.2), (BRIGHT, 0.85), (ENV_FM, 0.5)],
        "Wood" => &[(WAVE, 1.0), (TIMBRE, 0.45), (SYMMETRY, 0.3), (RATIO, 2.0), (FM, 0.05), (CONTOUR, 0.0), (RESPONSE, 0.12), (BRIGHT, 0.55)],
        "Folded Lead" => &[(WAVE, 1.0), (TIMBRE, 0.55), (SYMMETRY, 0.2), (RATIO, 1.0), (FM, 0.1), (CONTOUR, 2.0), (ATTACK, 0.01), (DECAY, 0.4), (ENV_TIMBRE, 0.4), (GLIDE, 0.08), (VOICES, 1.0)],
        "Clang" => &[(WAVE, 1.0), (TIMBRE, 0.6), (RATIO, 1.41), (FM, 0.6), (CONTOUR, 0.0), (RESPONSE, 0.9), (BRIGHT, 0.9), (ENV_FM, 0.4)],
        "Bubble" => &[(WAVE, 0.0), (TIMBRE, 0.3), (KEYED, 0.0), (MOD_FREQ, 7.0), (FM, 0.35), (CONTOUR, 1.0), (ATTACK, 0.02), (DECAY, 0.35), (RESPONSE, 0.2)],
        "Swell" => &[(WAVE, 1.0), (TIMBRE, 0.35), (RATIO, 2.0), (FM, 0.15), (CONTOUR, 2.0), (ATTACK, 0.8), (DECAY, 1.4), (ENV_TIMBRE, 0.6), (RESPONSE, 0.6)],
        "Low Thump" => &[(WAVE, 0.0), (TIMBRE, 0.15), (RATIO, 0.5), (FM, 0.2), (CONTOUR, 0.0), (RESPONSE, 0.3), (BRIGHT, 0.45), (OCTAVE, -1.0)],
        "Dust" => &[(WAVE, 1.0), (TIMBRE, 0.4), (NOISE, 0.45), (CONTOUR, 0.0), (RESPONSE, 0.18), (BRIGHT, 0.8)],
        "Ring Mod" => &[(WAVE, 0.0), (TIMBRE, 0.2), (RATIO, 1.73), (FM, 0.0), (AM, 1.0), (CONTOUR, 1.0), (ATTACK, 0.005), (DECAY, 0.8), (RESPONSE, 0.5)],
        "Glass" => &[(WAVE, 0.0), (TIMBRE, 0.05), (RATIO, 5.0), (FM, 0.25), (CONTOUR, 0.0), (RESPONSE, 1.4), (BRIGHT, 1.0), (ENV_FM, 0.6), (WIDTH, 0.7)],
        _ => &[],
    };
    for &(i, v) in set {
        p[i] = v;
    }
    p
}

/// One period of a wave at `phase` (0..1), with its edges smoothed for `dt`.
fn wave(kind: usize, phase: f32, dt: f32) -> f32 {
    let blep = |t: f32| {
        if t < dt {
            let t = t / dt;
            t + t - t * t - 1.0
        } else if t > 1.0 - dt {
            let t = (t - 1.0) / dt;
            t * t + t + t + 1.0
        } else {
            0.0
        }
    };
    match kind {
        0 => (TAU * phase).sin(),
        1 => 1.0 - 4.0 * (phase - 0.5).abs(),
        2 => 2.0 * phase - 1.0 - blep(phase),
        _ => {
            let sq = if phase < 0.5 { 1.0 } else { -1.0 };
            sq + blep(phase) - blep((phase + 0.5).fract())
        }
    }
}

/// Folds a signal back on itself: gentle at 1, ever more folds above.
fn fold(x: f32, amount: f32, symmetry: f32) -> f32 {
    let gain = 1.0 + amount * 7.0;
    (0.5 * PI * (x * gain + symmetry * 0.8)).sin()
}

#[derive(Clone)]
struct Voice {
    note: u8,
    gate: bool,
    vel: f32,
    age: u64,
    pitch: f32,
    target: f32,
    phase: f32,
    mod_phase: f32,
    /// The contour (0..1) and where it is: rising, or not.
    env: f32,
    rising: bool,
    /// Strike: the light's flash still to come (samples).
    flash: u32,
    /// The photocell: how far the gate is open.
    cell: f32,
    lp1: f32,
    lp2: f32,
    dc: DcBlock,
    pan: f32,
}

impl Voice {
    fn new(i: usize) -> Self {
        Self { note: 0, gate: false, vel: 0.0, age: 0, pitch: 60.0, target: 60.0, phase: 0.0, mod_phase: 0.0, env: 0.0, rising: false, flash: 0, cell: 0.0, lp1: 0.0, lp2: 0.0, dc: DcBlock::default(), pan: if i % 2 == 0 { -1.0 } else { 1.0 } * (0.3 + 0.1 * (i / 2) as f32) }
    }

    fn active(&self) -> bool {
        self.gate || self.env > 1e-4 || self.cell > 1e-4 || self.flash > 0
    }
}

pub struct CoastState {
    pub pitch_bend: f32,
    sr: f32,
    voices: Vec<Voice>,
    held: Vec<(u8, f32)>,
    counter: u64,
    last_pitch: Option<f32>,
    rng: Rng,
    /// For the screen: the gate's opening and the timbre of the loudest voice.
    shown: (f32, f32),
}

impl CoastState {
    pub fn new(sr: f32) -> Self {
        Self { pitch_bend: 0.0, sr, voices: (0..MAX_VOICES).map(Voice::new).collect(), held: Vec::with_capacity(32), counter: 0, last_pitch: None, rng: Rng::new(0xC0A5_7001), shown: (0.0, 0.0) }
    }

    fn pitch_of(p: &[f32], note: u8) -> f32 {
        note as f32 + 12.0 * p[OCTAVE].round()
    }

    fn strike(&mut self, i: usize, note: u8, pitch: f32, from: Option<f32>, vel: f32) {
        self.counter += 1;
        let v = &mut self.voices[i];
        v.note = note;
        v.gate = true;
        v.vel = vel;
        v.age = self.counter;
        v.target = pitch;
        v.pitch = from.unwrap_or(pitch);
        v.rising = true;
        // The light flashes for five milliseconds.
        v.flash = (0.005 * self.sr) as u32;
    }

    pub fn note_on(&mut self, p: &[f32], note: u8, vel: f32) {
        if p.len() < SPECS.len() {
            return;
        }
        let pitch = Self::pitch_of(p, note);
        let glide = p[GLIDE] > 0.002;
        self.held.retain(|h| h.0 != note);
        self.held.push((note, vel));
        let n = (p[VOICES].round() as usize).clamp(1, MAX_VOICES);
        if n == 1 {
            let from = if glide && self.voices[0].active() { Some(self.voices[0].pitch) } else { None };
            self.strike(0, note, pitch, from, vel);
            self.last_pitch = Some(pitch);
            return;
        }
        let pool = &self.voices[..n];
        let pick = pool
            .iter()
            .position(|v| v.gate && v.note == note)
            .or_else(|| pool.iter().position(|v| !v.active()))
            .or_else(|| pool.iter().enumerate().filter(|(_, v)| !v.gate).min_by_key(|(_, v)| v.age).map(|(i, _)| i))
            .unwrap_or_else(|| pool.iter().enumerate().min_by_key(|(_, v)| v.age).map(|(i, _)| i).unwrap_or(0));
        let from = if glide { self.last_pitch } else { None };
        self.strike(pick, note, pitch, from, vel);
        self.last_pitch = Some(pitch);
    }

    pub fn note_off(&mut self, p: &[f32], note: u8) {
        self.held.retain(|h| h.0 != note);
        let n = p.get(VOICES).map(|v| (v.round() as usize).clamp(1, MAX_VOICES)).unwrap_or(1);
        if n == 1 && self.voices[0].note == note {
            // Back to a key still held, gliding there.
            if let Some(&(prev, vel)) = self.held.last() {
                let pitch = Self::pitch_of(p, prev);
                let v = &mut self.voices[0];
                v.note = prev;
                v.target = pitch;
                v.vel = vel;
                return;
            }
        }
        for v in self.voices.iter_mut().filter(|v| v.gate && v.note == note) {
            v.gate = false;
        }
    }

    pub fn all_off(&mut self) {
        self.held.clear();
        for v in self.voices.iter_mut() {
            v.gate = false;
        }
    }

    /// For the screen: voices sounding, how far the gate is open, the timbre now.
    pub fn visual(&self, out: &mut [f32; crate::engine::INST_VALS]) {
        out[0] = self.voices.iter().filter(|v| v.active()).count() as f32;
        out[1] = self.shown.0;
        out[2] = self.shown.1;
    }

    pub fn render(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32]) {
        if p.len() < SPECS.len() {
            return;
        }
        let n = l.len().min(r.len());
        let sr = self.sr;
        let pick = |i: usize, k: usize| (p[i].round().max(0.0) as usize).min(k - 1);
        let wave_kind = pick(WAVE, WAVES.len());
        let mod_kind = pick(MOD_WAVE, WAVES.len());
        let contour = pick(CONTOUR, CONTOURS.len());
        let gate_kind = pick(GATE, GATES.len());
        let keyed = p[KEYED] >= 0.5;
        let attack = (-1.0 / (p[ATTACK].max(0.0005) * sr)).exp();
        let decay = (-4.6 / (p[DECAY].max(0.005) * sr)).exp();
        // The photocell opens in a few milliseconds and lets go over Response.
        let cell_up = 1.0 - (-1.0 / (0.0015 * sr)).exp();
        let cell_down = 4.6 / (p[RESPONSE].max(0.01) * sr);
        let glide = if p[GLIDE] > 0.002 { 1.0 - (-1.0 / (p[GLIDE] * 0.3 * sr)).exp() } else { 1.0 };
        let gain = db_to_gain(p[VOLUME]) * 0.5;
        let width = p[WIDTH].clamp(0.0, 1.0);
        let vel_amt = p[VELOCITY].clamp(0.0, 1.0);
        let bright = p[BRIGHT].clamp(0.0, 1.0);
        let mut shown = (0.0f32, 0.0f32);
        for v in self.voices.iter_mut() {
            if !v.active() {
                continue;
            }
            let (pl, pr) = pan_gains(v.pan * width);
            let strength = 1.0 - vel_amt * (1.0 - v.vel.clamp(0.0, 1.0));
            for i in 0..n {
                // The contour: a flash (Strike), up and down (Transient), or up and held (Sustained).
                let target = match contour {
                    0 => {
                        if v.flash > 0 {
                            v.flash -= 1;
                            1.0
                        } else {
                            0.0
                        }
                    }
                    1 => {
                        if v.rising && v.env > 0.995 {
                            v.rising = false;
                        }
                        if v.rising {
                            1.0
                        } else {
                            0.0
                        }
                    }
                    _ => {
                        if v.gate {
                            1.0
                        } else {
                            v.rising = false;
                            0.0
                        }
                    }
                };
                v.env = if target > v.env { target + (v.env - target) * attack } else { v.env * decay };
                if contour == 0 {
                    // A strike is the flash itself: the gate's afterglow is the note.
                    v.env = target;
                }
                let light = v.env * strength;
                v.cell = if light > v.cell { v.cell + (light - v.cell) * cell_up } else { v.cell - v.cell * cell_down * (0.35 + v.cell) };
                v.pitch += (v.target - v.pitch) * glide;
                let f0 = midi_to_freq(v.pitch + self.pitch_bend);
                let fm_freq = if keyed { f0 * p[RATIO] } else { p[MOD_FREQ] };
                v.mod_phase = (v.mod_phase + fm_freq / sr).fract();
                let m = wave(mod_kind, v.mod_phase, (fm_freq / sr).min(0.5));
                let shape = (v.cell + v.env).min(1.0);
                let fm = (p[FM] + p[ENV_FM] * shape).clamp(0.0, 2.0);
                v.phase = (v.phase + f0 / sr).fract();
                let ph = (v.phase + fm * 0.75 * m).rem_euclid(1.0);
                let mut x = wave(wave_kind, ph, (f0 / sr).min(0.5));
                x *= 1.0 - p[AM] * 0.5 * (1.0 - m);
                let timbre = (p[TIMBRE] + p[ENV_TIMBRE] * shape).clamp(0.0, 1.0);
                x = fold(x, timbre, p[SYMMETRY]);
                if p[NOISE] > 0.0 {
                    x = x * (1.0 - p[NOISE]) + self.rng.bipolar() * p[NOISE];
                }
                x = v.dc.tick(x);
                // The low pass gate: the filter and the gain opened together.
                let c = v.cell.clamp(0.0, 1.0);
                let (filter_on, amp) = match gate_kind {
                    0 => (true, c * c.sqrt()),
                    1 => (true, (c * 6.0).min(1.0)),
                    _ => (false, c),
                };
                let y = if filter_on {
                    let cutoff = (40.0 * 2f32.powf(c * (2.0 + bright * 8.6))).min(sr * 0.45);
                    let a = 1.0 - (-TAU * cutoff / sr).exp();
                    v.lp1 += (x - v.lp1) * a;
                    v.lp2 += (v.lp1 - v.lp2) * a;
                    v.lp2
                } else {
                    x
                };
                let out = y * amp * gain;
                l[i] += out * pl;
                r[i] += out * pr;
                if c > shown.0 {
                    shown = (c, timbre);
                }
            }
        }
        self.shown = shown;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn play(p: &[f32], secs: f32, hold: f32) -> Vec<f32> {
        let sr = 48_000.0;
        let mut st = CoastState::new(sr);
        st.note_on(p, 57, 1.0);
        let mut out = Vec::new();
        let blocks = (secs * sr / 256.0) as usize;
        for b in 0..blocks {
            if b as f32 * 256.0 / sr >= hold && st.held.iter().any(|h| h.0 == 57) {
                st.note_off(p, 57);
            }
            let (mut l, mut r) = (vec![0.0; 256], vec![0.0; 256]);
            st.render(p, &mut l, &mut r);
            out.extend(l);
        }
        out
    }

    fn rms(x: &[f32]) -> f32 {
        (x.iter().map(|v| v * v).sum::<f32>() / x.len().max(1) as f32).sqrt()
    }

    #[test]
    fn a_strike_rings_out_and_dies_away_held_or_not() {
        let p = preset("Bongo");
        let out = play(&p, 2.0, 2.0);
        let early = rms(&out[480..4800]);
        let late = rms(&out[out.len() - 4800..]);
        assert!(early > 0.01, "it sounds: {early}");
        assert!(late < early * 0.05, "and dies away while held: {early} {late}");
        assert!(out.iter().all(|v| v.is_finite() && v.abs() < 2.0));
    }

    #[test]
    fn sustained_holds_while_the_key_is_held() {
        let mut p = preset("Folded Lead");
        p[CONTOUR] = 2.0;
        let out = play(&p, 1.5, 1.0);
        let held = rms(&out[36_000..47_000]);
        let after = rms(&out[out.len() - 4800..]);
        assert!(held > 0.01 && after < held * 0.2, "{held} {after}");
    }

    #[test]
    fn timbre_folds_in_harmonics() {
        // Zero crossings rise as the wave folds over.
        let crossings = |timbre: f32| {
            let mut p = preset("Bongo");
            (p[TIMBRE], p[ENV_TIMBRE], p[FM], p[CONTOUR], p[GATE]) = (timbre, 0.0, 0.0, 2.0, 2.0);
            let out = play(&p, 0.4, 1.0);
            out[4800..14_400].windows(2).filter(|w| w[0] <= 0.0 && w[1] > 0.0).count()
        };
        assert!(crossings(0.9) > crossings(0.0) * 2, "{} {}", crossings(0.0), crossings(0.9));
    }

    #[test]
    fn every_preset_sounds_and_stays_in_bounds() {
        for name in preset_names() {
            let out = play(&preset(name), 1.0, 0.5);
            assert!(rms(&out) > 1e-3, "{name} sounds");
            assert!(out.iter().all(|v| v.is_finite() && v.abs() < 2.0), "{name} stays in bounds");
        }
    }
}

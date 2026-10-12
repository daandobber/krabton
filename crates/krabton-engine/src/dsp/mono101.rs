//! Mono 101: a monophonic synth after the most loved small mono of the
//! early eighties. One oscillator (a saw core with a pulse from its
//! comparator and a sub oscillator from flip-flops: an octave down, two
//! octaves down, or a narrow pulse two octaves down) and noise into a
//! source mixer; a four-pole lowpass that self-oscillates; one envelope
//! for the filter and, or only the gate for, the amplifier; one LFO
//! (triangle, square, random, noise) for pitch, pulse width and filter; the
//! envelope triggered by the gate, by every key, or by the LFO; portamento
//! off, on legato, or always.
//!
//! The behaviour (trigger and gate modes, note priority, the sub modes,
//! portamento modes, a slow drift, velocity) and the presets come from HUSH
//! ONE by Charles Vestal (MIT License, Copyright (c) 2025-2026 Charles
//! Vestal, https://github.com/charlesvestal/schwung-hush1). The sound is
//! Krabton's own: band-limited oscillators (PolyBLEP steps, also on the sub
//! oscillator's flips) and a zero-delay-feedback ladder with a saturating
//! input, so it neither aliases nor needs tricks to self-oscillate.

use super::{db_to_gain, fast_tanh, Rng};
use crate::params::{ParamSpec, Unit};
use std::f32::consts::PI;

pub const LFO_RATE: usize = 0;
pub const LFO_WAVE: usize = 1;
pub const LFO_SYNC: usize = 2;
pub const RANGE: usize = 3;
pub const VCO_LFO: usize = 4;
pub const PW: usize = 5;
pub const PWM_SOURCE: usize = 6;
pub const FINE: usize = 7;
pub const PULSE: usize = 8;
pub const SAW: usize = 9;
pub const SUB: usize = 10;
pub const NOISE: usize = 11;
pub const SUB_MODE: usize = 12;
pub const CUTOFF: usize = 13;
pub const RES: usize = 14;
pub const ENV_AMT: usize = 15;
pub const VCF_LFO: usize = 16;
pub const KYBD: usize = 17;
pub const VCA_MODE: usize = 18;
pub const ATTACK: usize = 19;
pub const DECAY: usize = 20;
pub const SUSTAIN: usize = 21;
pub const RELEASE: usize = 22;
pub const TRIGGER: usize = 23;
pub const PORTA_MODE: usize = 24;
pub const PORTA_TIME: usize = 25;
pub const VELOCITY: usize = 26;
pub const VINTAGE: usize = 27;
pub const VOLUME: usize = 28;

pub const LFO_WAVES: &[&str] = &["Triangle", "Square", "Random", "Noise"];
pub const RANGES: &[&str] = &["16'", "8'", "4'", "2'"];
pub const PWM_SOURCES: &[&str] = &["LFO", "Manual", "Env"];
pub const SUB_MODES: &[&str] = &["-1 Oct", "-2 Oct", "-2 Oct Pulse"];
pub const VCA_MODES: &[&str] = &["Env", "Gate"];
pub const TRIGGERS: &[&str] = &["Gate+Trig", "Gate", "LFO"];
pub const PORTA_MODES: &[&str] = &["Off", "Auto", "On"];

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::log("Rate", "Modulator", 0.1, 30.0, 4.0, Unit::Hz),
    ParamSpec::choice("Wave", "Modulator", LFO_WAVES, 0),
    ParamSpec::toggle("Key Sync", "Modulator", false),
    ParamSpec::choice("Range", "VCO", RANGES, 1),
    ParamSpec::lin("Mod", "VCO", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Pulse Width", "VCO", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::choice("PWM", "VCO", PWM_SOURCES, 1),
    ParamSpec::lin("Fine", "VCO", -50.0, 50.0, 0.0, Unit::Cents),
    ParamSpec::lin("Pulse", "Source Mixer", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Saw", "Source Mixer", 0.0, 1.0, 0.8, Unit::Percent),
    ParamSpec::lin("Sub", "Source Mixer", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Noise", "Source Mixer", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::choice("Sub Mode", "Source Mixer", SUB_MODES, 0),
    ParamSpec::lin("Freq", "VCF", 0.0, 1.0, 0.6, Unit::Percent),
    ParamSpec::lin("Res", "VCF", 0.0, 1.0, 0.2, Unit::Percent),
    ParamSpec::lin("Env", "VCF", 0.0, 1.0, 0.35, Unit::Percent),
    ParamSpec::lin("Mod", "VCF", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Kybd", "VCF", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::choice("VCA", "VCA", VCA_MODES, 0),
    ParamSpec::log("Attack", "Env", 0.0015, 4.0, 0.002, Unit::Sec),
    ParamSpec::log("Decay", "Env", 0.002, 10.0, 0.3, Unit::Sec),
    ParamSpec::lin("Sustain", "Env", 0.0, 1.0, 0.6, Unit::Percent),
    ParamSpec::log("Release", "Env", 0.002, 10.0, 0.15, Unit::Sec),
    ParamSpec::choice("Trigger", "Env", TRIGGERS, 0),
    ParamSpec::choice("Portamento", "Portamento", PORTA_MODES, 0),
    ParamSpec::log("Time", "Portamento", 0.005, 2.0, 0.08, Unit::Sec),
    ParamSpec::lin("Velocity", "Output", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::lin("Vintage", "Output", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::lin("Volume", "Output", -24.0, 6.0, 0.0, Unit::Db),
];

/// A patch: the defaults with some knobs set.
fn patch(set: &[(usize, f32)]) -> Vec<f32> {
    let mut p = crate::params::defaults(SPECS);
    for &(k, v) in set {
        p[k] = v;
    }
    p
}

pub fn preset_names() -> &'static [&'static str] {
    &[
        "Init",
        "Classic Bass",
        "Acid Square",
        "Hollow PWM Lead",
        "Laser Zap",
        "Resonant Pluck",
        "Self-Osc Sine",
        "Noise Snare",
        "Portamento Lead",
        "Arp Pluck",
        "Random Burble",
        "Sub Bass",
        "Bleep",
        "Wobble",
        "LFO Trigger",
        "Chiptune",
    ]
}

pub fn preset(name: &str) -> Vec<f32> {
    // The first eleven after HUSH ONE's presets (with one envelope, as on the
    // original: where the amplifier held and the filter fell, the VCA runs on
    // the gate and the envelope moves the filter).
    match name {
        "Classic Bass" => patch(&[(PULSE, 0.68), (SAW, 0.12), (SUB, 1.0), (CUTOFF, 0.55), (RES, 0.16), (ENV_AMT, 0.25), (KYBD, 0.24), (VCA_MODE, 1.0), (ATTACK, 0.0015), (DECAY, 0.16), (SUSTAIN, 0.1), (RELEASE, 0.09), (TRIGGER, 1.0), (VELOCITY, 0.3), (VOLUME, -2.0)]),
        "Acid Square" => patch(&[(PULSE, 0.96), (SAW, 0.0), (SUB, 0.62), (PW, 0.15), (CUTOFF, 0.61), (RES, 0.8), (ENV_AMT, 0.38), (KYBD, 0.34), (ATTACK, 0.0015), (DECAY, 0.12), (SUSTAIN, 0.2), (RELEASE, 0.07), (PORTA_MODE, 1.0), (PORTA_TIME, 0.045), (VCF_LFO, 0.02), (VELOCITY, 0.56)]),
        "Hollow PWM Lead" => patch(&[(PULSE, 0.93), (SAW, 0.22), (SUB, 0.22), (PWM_SOURCE, 0.0), (PW, 0.72), (LFO_RATE, 3.2), (VCO_LFO, 0.04), (VCF_LFO, 0.05), (CUTOFF, 0.8), (RES, 0.31), (ENV_AMT, 0.17), (KYBD, 0.66), (ATTACK, 0.003), (DECAY, 0.15), (SUSTAIN, 0.7), (RELEASE, 0.22), (PORTA_MODE, 1.0), (PORTA_TIME, 0.014), (VELOCITY, 0.6)]),
        "Laser Zap" => patch(&[(SAW, 0.78), (PULSE, 0.18), (SUB, 0.08), (NOISE, 0.06), (PW, 0.3), (CUTOFF, 0.83), (RES, 0.5), (ENV_AMT, 0.14), (KYBD, 0.4), (LFO_RATE, 12.0), (VCO_LFO, 0.22), (ATTACK, 0.0015), (DECAY, 0.09), (SUSTAIN, 0.0), (RELEASE, 0.05), (PORTA_MODE, 2.0), (PORTA_TIME, 0.085), (VELOCITY, 0.58)]),
        "Resonant Pluck" => patch(&[(SAW, 0.48), (PULSE, 0.52), (SUB, 0.2), (PW, 0.11), (CUTOFF, 0.72), (RES, 0.75), (ENV_AMT, 0.26), (KYBD, 0.48), (ATTACK, 0.0015), (DECAY, 0.1), (SUSTAIN, 0.02), (RELEASE, 0.08), (VCF_LFO, 0.04), (LFO_RATE, 5.3), (VELOCITY, 0.64)]),
        "Self-Osc Sine" => patch(&[(SAW, 0.3), (PULSE, 0.0), (CUTOFF, 0.81), (RES, 1.0), (ENV_AMT, 0.03), (KYBD, 1.0), (ATTACK, 0.002), (DECAY, 0.16), (SUSTAIN, 0.8), (RELEASE, 0.22), (VCO_LFO, 0.03), (PORTA_MODE, 1.0), (PORTA_TIME, 0.008), (VELOCITY, 0.42)]),
        "Noise Snare" => patch(&[(NOISE, 0.96), (SAW, 0.08), (PULSE, 0.06), (PW, 0.44), (CUTOFF, 0.82), (RES, 0.38), (ENV_AMT, 0.12), (KYBD, 0.08), (ATTACK, 0.0015), (DECAY, 0.36), (SUSTAIN, 0.0), (RELEASE, 0.18), (VCF_LFO, 0.08), (LFO_RATE, 9.0), (VELOCITY, 0.72)]),
        "Portamento Lead" => patch(&[(PULSE, 0.76), (SAW, 0.36), (SUB, 0.2), (PWM_SOURCE, 0.0), (PW, 0.22), (CUTOFF, 0.79), (RES, 0.26), (ENV_AMT, 0.18), (KYBD, 0.74), (LFO_RATE, 4.6), (VCO_LFO, 0.12), (VCF_LFO, 0.06), (ATTACK, 0.002), (DECAY, 0.17), (SUSTAIN, 0.68), (RELEASE, 0.24), (PORTA_MODE, 2.0), (PORTA_TIME, 0.096), (TRIGGER, 1.0), (VELOCITY, 0.54)]),
        "Arp Pluck" => patch(&[(SAW, 0.54), (PULSE, 0.24), (SUB, 0.3), (NOISE, 0.02), (PW, 0.18), (CUTOFF, 0.76), (RES, 0.59), (ENV_AMT, 0.22), (KYBD, 0.52), (ATTACK, 0.0015), (DECAY, 0.09), (SUSTAIN, 0.0), (RELEASE, 0.04), (VCF_LFO, 0.1), (LFO_RATE, 6.4), (VELOCITY, 0.66)]),
        "Random Burble" => patch(&[(SAW, 0.42), (PULSE, 0.2), (SUB, 0.3), (NOISE, 0.18), (PWM_SOURCE, 0.0), (PW, 0.26), (CUTOFF, 0.65), (RES, 0.64), (ENV_AMT, 0.16), (KYBD, 0.4), (LFO_WAVE, 2.0), (LFO_RATE, 8.8), (VCF_LFO, 0.84), (ATTACK, 0.004), (DECAY, 0.36), (SUSTAIN, 0.62), (RELEASE, 0.4), (PORTA_MODE, 1.0), (PORTA_TIME, 0.022), (TRIGGER, 1.0)]),
        // Krabton's own.
        "Sub Bass" => patch(&[(RANGE, 0.0), (SAW, 0.2), (PULSE, 0.3), (SUB, 1.0), (CUTOFF, 0.5), (RES, 0.1), (ENV_AMT, 0.2), (KYBD, 0.3), (VCA_MODE, 1.0), (DECAY, 0.2), (SUSTAIN, 0.3), (TRIGGER, 1.0)]),
        "Bleep" => patch(&[(RANGE, 2.0), (SAW, 0.0), (PULSE, 1.0), (CUTOFF, 0.7), (RES, 0.5), (ENV_AMT, 0.3), (KYBD, 0.6), (DECAY, 0.08), (SUSTAIN, 0.0), (RELEASE, 0.05)]),
        "Wobble" => patch(&[(SAW, 0.8), (SUB, 0.5), (LFO_RATE, 2.5), (VCF_LFO, 0.6), (CUTOFF, 0.55), (RES, 0.55), (ENV_AMT, 0.0), (SUSTAIN, 1.0), (RELEASE, 0.1), (LFO_SYNC, 1.0)]),
        "LFO Trigger" => patch(&[(TRIGGER, 2.0), (LFO_WAVE, 1.0), (LFO_RATE, 8.0), (PULSE, 0.7), (SAW, 0.2), (SUB, 0.4), (CUTOFF, 0.62), (RES, 0.6), (ENV_AMT, 0.3), (DECAY, 0.06), (SUSTAIN, 0.0), (RELEASE, 0.05), (LFO_SYNC, 1.0)]),
        "Chiptune" => patch(&[(RANGE, 2.0), (SAW, 0.0), (PULSE, 1.0), (PW, 0.83), (CUTOFF, 1.0), (RES, 0.0), (ENV_AMT, 0.0), (VCA_MODE, 1.0), (VINTAGE, 0.0), (PORTA_MODE, 1.0), (PORTA_TIME, 0.02)]),
        _ => patch(&[]),
    }
}

// ---------------------------------------------------------------- the voice

#[derive(Clone, Copy, PartialEq, Debug)]
enum Stage {
    Idle,
    Attack,
    Decay,
    Release,
}

/// Steps the oscillator made between samples, band-limited (two-sample
/// PolyBLEP residuals for the previous and the current sample).
#[derive(Default)]
struct Blep {
    prev: f32,
    now: f32,
}

impl Blep {
    #[inline]
    fn step(&mut self, height: f32, x: f32) {
        let x = x.clamp(0.0, 1.0);
        self.prev += 0.5 * height * x * x;
        let y = 1.0 - x;
        self.now -= 0.5 * height * y * y;
    }
}

pub struct Mono101State {
    sr: f32,
    /// Keys held, oldest first.
    held: Vec<u8>,
    gate: bool,
    note: u8,
    velocity: f32,
    /// The pitch now (semitones) and where it is going.
    pitch: f32,
    target: f32,
    phase: f32,
    /// The flip-flops after the oscillator (counts saw cycles).
    flips: u8,
    pulse_high: bool,
    sub_now: f32,
    pending: f32,
    noise_lp: f32,
    ladder: [f32; 4],
    env: f32,
    stage: Stage,
    vca: f32,
    lfo_phase: f32,
    lfo_held: f32,
    lfo_square_high: bool,
    drift: f32,
    drift_target: f32,
    dc: f32,
    rng: Rng,
    cutoff_hz: f32,
}

impl Mono101State {
    pub fn new(sr: f32) -> Self {
        Self {
            sr: sr.max(1000.0),
            held: Vec::with_capacity(16),
            gate: false,
            note: 60,
            velocity: 1.0,
            pitch: 60.0,
            target: 60.0,
            phase: 0.0,
            flips: 0,
            pulse_high: true,
            sub_now: 1.0,
            pending: 0.0,
            noise_lp: 0.0,
            ladder: [0.0; 4],
            env: 0.0,
            stage: Stage::Idle,
            vca: 0.0,
            lfo_phase: 0.0,
            lfo_held: 0.0,
            lfo_square_high: false,
            drift: 0.0,
            drift_target: 0.0,
            dc: 0.0,
            rng: Rng::new(0x1_01_5E1D),
            cutoff_hz: 1000.0,
        }
    }

    pub fn is_active(&self) -> bool {
        self.gate || self.stage != Stage::Idle || self.vca > 1e-4
    }

    fn trigger(&mut self) {
        // From where the envelope is, so a retrigger does not click.
        self.stage = Stage::Attack;
    }

    pub fn note_on(&mut self, p: &[f32], note: u8, vel: f32) {
        let was_gate = self.gate;
        self.held.retain(|n| *n != note);
        if self.held.len() == self.held.capacity() {
            self.held.remove(0);
        }
        self.held.push(note);
        self.note = note;
        self.velocity = vel.clamp(0.0, 1.0);
        self.target = note as f32;
        // Portamento: Off jumps, Auto glides between held keys, On always glides.
        let porta = p[PORTA_MODE].round() as i32;
        if porta == 0 || (porta == 1 && !was_gate) {
            self.pitch = self.target;
        }
        if p[LFO_SYNC] >= 0.5 && !was_gate {
            self.lfo_phase = 0.0;
        }
        self.gate = true;
        match p[TRIGGER].round() as i32 {
            // Gate+Trig: every key starts the envelope again.
            0 => self.trigger(),
            // Gate: only the first key; legato notes keep it going.
            1 => {
                if !was_gate {
                    self.trigger();
                }
            }
            // LFO: the LFO triggers it while a key is down.
            _ => {}
        }
    }

    pub fn note_off(&mut self, p: &[f32], note: u8) {
        self.held.retain(|n| *n != note);
        match self.held.last().copied() {
            Some(n) => {
                // Back to the key still held (last note priority).
                self.note = n;
                self.target = n as f32;
                if p[PORTA_MODE].round() as i32 == 0 {
                    self.pitch = self.target;
                }
                if p[TRIGGER].round() as i32 == 0 {
                    self.trigger();
                }
            }
            None => {
                self.gate = false;
                if self.stage != Stage::Idle {
                    self.stage = Stage::Release;
                }
            }
        }
    }

    pub fn all_off(&mut self) {
        self.held.clear();
        self.gate = false;
        if self.stage != Stage::Idle {
            self.stage = Stage::Release;
        }
    }

    /// For the screen: the envelope, the LFO, the cutoff (0..1 of the
    /// range) and whether a key is down.
    pub fn visual(&self, out: &mut [f32; crate::engine::INST_VALS]) {
        out[0] = self.env;
        out[1] = self.lfo_held;
        out[2] = ((self.cutoff_hz / 20.0).max(1.0).log2() / 10.0).clamp(0.0, 1.0);
        out[3] = self.gate as u8 as f32;
        out[4] = self.pitch;
    }

    pub fn render(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32]) {
        if p.len() < SPECS.len() {
            return;
        }
        let n = l.len().min(r.len());
        let sr = self.sr;
        let k = |t: f32, c: f32| 1.0 - (-c / (t.max(0.0005) * sr)).exp();
        let (ka, kd, kr) = (k(p[ATTACK], 1.466), k(p[DECAY], 4.6), k(p[RELEASE], 4.6));
        let sus = p[SUSTAIN].clamp(0.0, 1.0);
        let porta_k = k(p[PORTA_TIME], 2.3);
        let octave = (p[RANGE].round().clamp(0.0, 3.0) - 1.0) * 12.0;
        let fine = p[FINE] / 100.0;
        let levels = [p[SAW], p[PULSE], p[SUB], p[NOISE]].map(|v| v.clamp(0.0, 1.0));
        let sub_mode = p[SUB_MODE].round() as u8;
        let pwm_src = p[PWM_SOURCE].round() as i32;
        let lfo_rate = p[LFO_RATE].clamp(0.05, 50.0);
        let lfo_wave = p[LFO_WAVE].round() as i32;
        let depth = |x: f32| {
            let x = x.clamp(0.0, 1.0);
            x * x * (2.0 - x)
        };
        let vco_lfo = depth(p[VCO_LFO]) * 12.0;
        let vcf_lfo = depth(p[VCF_LFO]) * 3.0;
        let env_oct = p[ENV_AMT].clamp(0.0, 1.0) * 9.0;
        let res = p[RES].clamp(0.0, 1.0);
        let kres = 4.25 * res.powf(1.2);
        let vel_s = p[VELOCITY].clamp(0.0, 1.0);
        let vel_shaped = self.velocity.powf(0.6);
        let vel_gain = 1.0 - 0.75 * vel_s * (1.0 - vel_shaped);
        let vel_oct = -1.5 * vel_s * (1.0 - vel_shaped);
        let vintage = p[VINTAGE].clamp(0.0, 1.0);
        let out_g = db_to_gain(p[VOLUME]) * vel_gain;
        let gate_mode = p[VCA_MODE] >= 0.5;
        let lfo_trig = p[TRIGGER].round() as i32 == 2;
        let declick = 1.0 - (-1.0 / (0.002 * sr)).exp();
        for i in 0..n {
            // --- LFO.
            let before = self.lfo_phase;
            self.lfo_phase = (self.lfo_phase + lfo_rate / sr).fract();
            let wrapped = self.lfo_phase < before;
            if wrapped && lfo_wave == 2 {
                self.lfo_held = self.rng.bipolar();
            }
            let lfo = match lfo_wave {
                1 => {
                    if self.lfo_phase < 0.5 {
                        1.0
                    } else {
                        -1.0
                    }
                }
                2 => self.lfo_held,
                3 => self.rng.bipolar(),
                _ => {
                    let x = self.lfo_phase;
                    if x < 0.5 {
                        4.0 * x - 1.0
                    } else {
                        3.0 - 4.0 * x
                    }
                }
            };
            if lfo_wave != 2 {
                self.lfo_held = lfo;
            }
            // LFO trigger: the envelope follows the LFO's square while a key is down.
            let high = self.lfo_phase < 0.5;
            if lfo_trig && self.gate {
                if high && !self.lfo_square_high {
                    self.stage = Stage::Attack;
                } else if !high && self.lfo_square_high && self.stage != Stage::Idle {
                    self.stage = Stage::Release;
                }
            }
            self.lfo_square_high = high;

            // --- Envelope (an RC circuit: the attack charges towards 1.3 and stops at 1).
            match self.stage {
                Stage::Attack => {
                    self.env += (1.3 - self.env) * ka;
                    if self.env >= 1.0 {
                        self.env = 1.0;
                        self.stage = Stage::Decay;
                    }
                }
                Stage::Decay => self.env += (sus - self.env) * kd,
                Stage::Release => {
                    self.env -= self.env * kr;
                    if self.env < 1e-5 {
                        self.env = 0.0;
                        self.stage = Stage::Idle;
                    }
                }
                Stage::Idle => {}
            }
            let gate_open = self.gate && (!lfo_trig || high);
            let vca_target = if gate_mode { if gate_open { 1.0 } else { 0.0 } } else { self.env };
            self.vca += (vca_target - self.vca) * declick;

            // --- Pitch: portamento, drift, LFO.
            self.pitch += (self.target - self.pitch) * porta_k;
            if self.rng.unipolar() < 0.002 {
                self.drift_target = self.rng.bipolar() * 0.12 * vintage;
            }
            self.drift += (self.drift_target - self.drift) * 0.0004;
            let semis = self.pitch + octave + fine + self.drift + lfo * vco_lfo;
            let f = 440.0 * 2f32.powf((semis - 69.0) / 12.0);
            let dt = (f / sr).min(0.45);

            // --- Oscillator: saw core, pulse comparator, sub flip-flops.
            let duty = match pwm_src {
                0 => 0.5 - 0.45 * p[PW].clamp(0.0, 1.0) * (0.5 + 0.5 * lfo),
                2 => 0.5 - 0.45 * p[PW].clamp(0.0, 1.0) * self.env,
                _ => 0.5 - 0.45 * p[PW].clamp(0.0, 1.0),
            }
            .clamp(0.04, 0.5);
            let mut bl = Blep::default();
            let a = self.phase;
            let b0 = a + dt;
            let mut b = b0;
            let since = move |e: f32| (b0 - e) / dt;
            if a < duty && b >= duty && self.pulse_high {
                bl.step(-2.0 * levels[1], since(duty));
                self.pulse_high = false;
            }
            if b >= 1.0 {
                let x = since(1.0);
                bl.step(-2.0 * levels[0], x);
                if !self.pulse_high {
                    bl.step(2.0 * levels[1], x);
                    self.pulse_high = true;
                }
                self.flips = (self.flips + 1) % 4;
                let sub = match sub_mode {
                    1 => {
                        if self.flips < 2 {
                            1.0
                        } else {
                            -1.0
                        }
                    }
                    2 => {
                        if self.flips == 0 {
                            1.0
                        } else {
                            -1.0
                        }
                    }
                    _ => {
                        if self.flips % 2 == 0 {
                            1.0
                        } else {
                            -1.0
                        }
                    }
                };
                if sub != self.sub_now {
                    bl.step((sub - self.sub_now) * levels[2], x);
                    self.sub_now = sub;
                }
                b -= 1.0;
                if b >= duty && self.pulse_high {
                    bl.step(-2.0 * levels[1], since(1.0 + duty));
                    self.pulse_high = false;
                }
            }
            self.phase = b;
            let saw = 2.0 * self.phase - 1.0;
            let pulse = if self.pulse_high { 1.0 } else { -1.0 };
            let white = self.rng.bipolar();
            self.noise_lp += 0.085 * (white - self.noise_lp);
            let noise = 0.72 * self.noise_lp * 2.5 + 0.28 * white;
            let s = levels[0] * saw + levels[1] * pulse + levels[2] * self.sub_now + levels[3] * noise;
            let osc = self.pending + bl.prev;
            self.pending = s + bl.now;
            // The mixer is pushed a little.
            let mixed = fast_tanh(osc * 0.55) * 1.2;

            // --- Filter: a four-pole ladder, zero-delay feedback, saturating input.
            let kybd = p[KYBD].clamp(0.0, 1.0) * (self.pitch - 60.0) / 12.0;
            let jitter = vintage * 0.01 * self.rng.bipolar();
            let oct = p[CUTOFF].clamp(0.0, 1.0) * 10.0 + kybd + env_oct * self.env + vcf_lfo * lfo + vel_oct + jitter;
            let fc = (20.0 * 2f32.powf(oct)).clamp(15.0, 0.45 * sr);
            self.cutoff_hz = fc;
            let g = (PI * fc / sr).tan();
            let gg = g / (1.0 + g);
            let bb = 1.0 - gg;
            let st = &mut self.ladder;
            let s_sum = (gg * (gg * (gg * st[0] + st[1]) + st[2]) + st[3]) * bb;
            let g4 = gg * gg * gg * gg;
            let x = mixed;
            let y4 = (g4 * x + s_sum) / (1.0 + kres * g4);
            let mut u = fast_tanh(x - kres * y4);
            for s in st.iter_mut() {
                let w = (u - *s) * gg;
                let yy = w + *s;
                *s = yy + w;
                u = yy;
            }
            // The resonance takes some of the bass, as on the original; part of it comes back.
            let y = u * (1.0 + 0.35 * res);
            // The coupling capacitor to the amplifier.
            self.dc += (y - self.dc) * 0.00005;
            let out = (y - self.dc) * self.vca * out_g * 0.55;
            l[i] += out;
            r[i] += out;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 48000.0;

    fn play(p: &[f32], note: u8, secs: f32, release_after: f32) -> Vec<f32> {
        let mut s = Mono101State::new(SR);
        s.note_on(p, note, 0.9);
        let mut out = Vec::new();
        let n = (secs * SR) as usize;
        let mut done = 0;
        while done < n {
            if done >= (release_after * SR) as usize && s.gate {
                s.note_off(p, note);
            }
            let len = 256.min(n - done);
            let (mut l, mut r) = (vec![0.0; len], vec![0.0; len]);
            s.render(p, &mut l, &mut r);
            assert!(l.iter().all(|v| v.is_finite() && v.abs() < 2.0));
            out.extend_from_slice(&l);
            done += len;
        }
        out
    }

    fn rms(x: &[f32]) -> f32 {
        (x.iter().map(|v| v * v).sum::<f32>() / x.len().max(1) as f32).sqrt()
    }

    fn crossings(x: &[f32]) -> f32 {
        x.windows(2).filter(|w| w[0] < 0.0 && w[1] >= 0.0).count() as f32 * SR / x.len() as f32
    }

    #[test]
    fn every_preset_plays_and_stops() {
        for name in preset_names() {
            let p = preset(name);
            assert_eq!(p.len(), SPECS.len());
            let x = play(&p, 48, 2.0, 0.6);
            assert!(rms(&x[240..4800]) > 0.01, "{name}: {}", rms(&x[240..4800]));
            let tail = rms(&x[x.len() - 4800..]);
            assert!(tail < 0.01 || *name == "Wobble", "{name} rings on: {tail}");
        }
    }

    #[test]
    fn the_sub_is_an_octave_or_two_below() {
        let base = |sub_mode: f32| {
            let p = patch(&[(SAW, 0.0), (SUB, 1.0), (SUB_MODE, sub_mode), (CUTOFF, 1.0), (ENV_AMT, 0.0), (SUSTAIN, 1.0), (VINTAGE, 0.0)]);
            crossings(&play(&p, 69, 0.5, 1.0)[4800..])
        };
        // A4 at 8': 440 Hz; the sub at 220, then 110.
        assert!((base(0.0) - 220.0).abs() < 8.0, "{}", base(0.0));
        assert!((base(1.0) - 110.0).abs() < 6.0, "{}", base(1.0));
        assert!((base(2.0) - 110.0).abs() < 6.0, "{}", base(2.0));
    }

    #[test]
    fn the_filter_self_oscillates_at_full_resonance() {
        // No oscillator at all: the filter alone sings at its cutoff.
        let p = patch(&[(SAW, 0.0), (NOISE, 0.004), (CUTOFF, 0.55), (RES, 1.0), (ENV_AMT, 0.0), (KYBD, 0.0), (SUSTAIN, 1.0), (VINTAGE, 0.0)]);
        let x = play(&p, 60, 1.5, 2.0);
        let late = &x[48000..];
        assert!(rms(late) > 0.05, "{}", rms(late));
        let f = crossings(late);
        let fc = 20.0 * 2f32.powf(5.5);
        assert!((f / fc - 1.0).abs() < 0.25, "{f} vs {fc}");
    }

    #[test]
    fn no_aliasing_at_the_top() {
        // A high saw: little energy between the harmonics (aliases) compared
        // with a naive saw; measured as how smooth the waveform's steps are.
        let p = patch(&[(RANGE, 3.0), (CUTOFF, 1.0), (ENV_AMT, 0.0), (SUSTAIN, 1.0), (VINTAGE, 0.0), (RES, 0.0)]);
        let x = play(&p, 96, 0.3, 1.0);
        let max_jump = x[4800..].windows(2).fold(0.0f32, |m, w| m.max((w[1] - w[0]).abs()));
        let peak = x[4800..].iter().fold(0.0f32, |m, v| m.max(v.abs()));
        assert!(max_jump < 1.6 * peak, "{max_jump} {peak}");
    }

    #[test]
    fn gate_mode_holds_and_env_mode_decays() {
        let env = patch(&[(DECAY, 0.1), (SUSTAIN, 0.0), (CUTOFF, 1.0), (ENV_AMT, 0.0)]);
        let gate = patch(&[(DECAY, 0.1), (SUSTAIN, 0.0), (CUTOFF, 1.0), (ENV_AMT, 0.0), (VCA_MODE, 1.0)]);
        let a = play(&env, 48, 1.0, 2.0);
        let b = play(&gate, 48, 1.0, 2.0);
        assert!(rms(&a[38400..]) < 0.2 * rms(&a[480..4800]));
        assert!(rms(&b[38400..]) > 0.6 * rms(&b[480..4800]));
    }

    #[test]
    fn portamento_auto_glides_only_between_held_keys() {
        let p = patch(&[(PORTA_MODE, 1.0), (PORTA_TIME, 0.3), (CUTOFF, 1.0), (SUSTAIN, 1.0), (VINTAGE, 0.0)]);
        let mut s = Mono101State::new(SR);
        s.note_on(&p, 48, 1.0);
        assert_eq!(s.pitch, 48.0, "the first key jumps");
        s.note_on(&p, 60, 1.0);
        let (mut l, mut r) = (vec![0.0; 2400], vec![0.0; 2400]);
        s.render(&p, &mut l, &mut r);
        assert!(s.pitch > 48.0 && s.pitch < 59.0, "{}", s.pitch);
        // Released, then a new key: no glide.
        s.note_off(&p, 60);
        s.note_off(&p, 48);
        s.note_on(&p, 36, 1.0);
        assert_eq!(s.pitch, 36.0);
    }

    #[test]
    fn gate_trigger_mode_does_not_restart_on_legato() {
        let p = patch(&[(TRIGGER, 1.0), (DECAY, 0.05), (SUSTAIN, 0.2)]);
        let mut s = Mono101State::new(SR);
        s.note_on(&p, 48, 1.0);
        let (mut l, mut r) = (vec![0.0; 9600], vec![0.0; 9600]);
        s.render(&p, &mut l, &mut r);
        s.note_on(&p, 50, 1.0);
        assert_eq!(s.stage, Stage::Decay, "legato keeps the envelope going");
        let p2 = patch(&[(TRIGGER, 0.0), (DECAY, 0.05), (SUSTAIN, 0.2)]);
        s.note_on(&p2, 52, 1.0);
        assert_eq!(s.stage, Stage::Attack, "gate+trig starts it again");
    }
}

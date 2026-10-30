//! Granulator: plays a sample as a cloud of grains. Every key held starts a
//! cloud of its own: grains of 1 ms to 2 s taken from the sample at Position,
//! sprayed about it, played at the key's pitch (with a pitch spray and jumps
//! of an octave), some of them backwards, under one of five windows, spread
//! over the stereo field. Where they come from can move by itself (Scan:
//! forward, backward, to and fro, or from Position on for as long as a key
//! is held, which stretches the sample in time), or stand still (Freeze).
//! Grains come at a rate (Density, with Jitter on when they come) or on the
//! song's beat (Sync). Each cloud rises and falls with its own envelope;
//! after the clouds a tone tilt, a smear (an echo fed back into itself) and
//! a reverb (Cloud). The grains come from a fixed pool, so a thick cloud
//! skips grains rather than allocating. Without a sample of its own it plays
//! one of four sounds made here (Source).

use super::reverb::Reverb;
use super::{db_to_gain, pan_gains, Adsr, AdsrCoefs, FilterMode, Rng, Svf};
use crate::params::{ParamSpec, Unit};
use crate::sample::{SampleData, SampleRef};
use std::sync::OnceLock;

pub const SOURCE: usize = 0;
pub const ROOT: usize = 1;
pub const POSITION: usize = 2;
pub const SPRAY: usize = 3;
pub const SIZE: usize = 4;
pub const DENSITY: usize = 5;
pub const SYNC: usize = 6;
pub const JITTER: usize = 7;
pub const SHAPE: usize = 8;
pub const REVERSE: usize = 9;
pub const SCAN: usize = 10;
pub const SPEED: usize = 11;
pub const FREEZE: usize = 12;
pub const PITCH: usize = 13;
pub const FINE: usize = 14;
pub const PITCH_SPRAY: usize = 15;
pub const OCTAVES: usize = 16;
pub const OCTAVE_WAY: usize = 17;
pub const KEYS: usize = 18;
pub const ATTACK: usize = 19;
pub const DECAY: usize = 20;
pub const SUSTAIN: usize = 21;
pub const RELEASE: usize = 22;
pub const VELOCITY: usize = 23;
pub const VEL_DENSITY: usize = 24;
pub const TONE: usize = 25;
pub const SMEAR: usize = 26;
pub const CLOUD: usize = 27;
pub const SPREAD: usize = 28;
pub const VOICES: usize = 29;
pub const VOLUME: usize = 30;

pub const SOURCES: &[&str] = &["Pad", "Choir", "Bells", "Breath"];
pub const SYNCS: &[&str] = &["Free", "1/2", "1/4", "1/8", "1/8 T", "1/16", "1/16 T", "1/32", "1/64"];
const SYNC_BEATS: [f64; 9] = [0.0, 2.0, 1.0, 0.5, 1.0 / 3.0, 0.25, 1.0 / 6.0, 0.125, 0.0625];
pub const SHAPES: &[&str] = &["Gaussian", "Hann", "Triangle", "Percussive", "Square"];
pub const SCANS: &[&str] = &["Off", "Forward", "Backward", "Ping-Pong", "Held"];
pub const OCTAVE_WAYS: &[&str] = &["Both", "Up", "Down"];

/// Grains sounding at once, over all voices.
pub const POOL: usize = 256;
const MAX_VOICES: usize = 8;
/// Samples worked on at a time (the voices' envelopes for them on the stack).
const CHUNK: usize = 32;
/// Points in a window's table.
const WINDOW: usize = 512;
/// The longest smear echo, in seconds.
const SMEAR_MAX: f32 = 1.0;
/// Grains the screen is told about.
pub const SHOWN: usize = 26;
/// Where the grains start in the screen's values (two values each).
pub const SHOWN_AT: usize = 12;

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::choice("Source", "Source", SOURCES, 0),
    ParamSpec::int("Root", "Source", 24.0, 96.0, 60.0, Unit::None),
    ParamSpec::lin("Position", "Grains", 0.0, 1.0, 0.35, Unit::Percent),
    ParamSpec::lin("Spray", "Grains", 0.0, 1.0, 0.08, Unit::Percent),
    ParamSpec::log("Size", "Grains", 0.001, 2.0, 0.15, Unit::Sec),
    ParamSpec::log("Density", "Grains", 0.5, 200.0, 18.0, Unit::Hz),
    ParamSpec::choice("Sync", "Grains", SYNCS, 0),
    ParamSpec::lin("Jitter", "Grains", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::choice("Shape", "Grains", SHAPES, 0),
    ParamSpec::lin("Reverse", "Grains", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::choice("Scan", "Scan", SCANS, 0),
    ParamSpec::log("Speed", "Scan", 0.01, 4.0, 0.1, Unit::None),
    ParamSpec::toggle("Freeze", "Scan", false),
    ParamSpec::int("Pitch", "Pitch", -24.0, 24.0, 0.0, Unit::Semi),
    ParamSpec::lin("Fine", "Pitch", -100.0, 100.0, 0.0, Unit::Cents),
    ParamSpec::lin("Pitch Spray", "Pitch", 0.0, 12.0, 0.05, Unit::None),
    ParamSpec::lin("Octaves", "Pitch", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::choice("Octave Way", "Pitch", OCTAVE_WAYS, 0),
    ParamSpec::toggle("Keys", "Pitch", true),
    ParamSpec::log("Attack", "Amp", 0.001, 10.0, 0.25, Unit::Sec),
    ParamSpec::log("Decay", "Amp", 0.01, 10.0, 1.0, Unit::Sec),
    ParamSpec::lin("Sustain", "Amp", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::log("Release", "Amp", 0.01, 20.0, 1.5, Unit::Sec),
    ParamSpec::lin("Velocity", "Amp", 0.0, 1.0, 0.6, Unit::Percent),
    ParamSpec::lin("Vel Density", "Amp", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Tone", "Space", -1.0, 1.0, 0.0, Unit::None),
    ParamSpec::lin("Smear", "Space", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Cloud", "Space", 0.0, 1.0, 0.25, Unit::Percent),
    ParamSpec::lin("Spread", "Space", 0.0, 1.0, 0.6, Unit::Percent),
    ParamSpec::int("Voices", "Output", 1.0, 8.0, 6.0, Unit::None),
    ParamSpec::lin("Volume", "Output", -30.0, 6.0, -4.0, Unit::Db),
];

pub fn preset_names() -> &'static [&'static str] {
    &["Cloud Pad", "Frozen", "Shimmer", "Glitch Stutter", "Time Stretch", "Swarm", "Reverse Rain", "Octave Spray", "Breath Drone", "Plucked Grains", "Choir Smear", "Tape Scrub"]
}

pub fn preset(name: &str) -> Vec<f32> {
    let mut p = crate::params::defaults(SPECS);
    let set: &[(usize, f32)] = match name {
        "Cloud Pad" => &[(SOURCE, 0.0), (POSITION, 0.4), (SPRAY, 0.15), (SIZE, 0.25), (DENSITY, 24.0), (SCAN, 1.0), (SPEED, 0.04), (PITCH_SPRAY, 0.08), (ATTACK, 0.8), (RELEASE, 2.5), (CLOUD, 0.35), (SPREAD, 0.7)],
        "Frozen" => &[(SOURCE, 1.0), (POSITION, 0.45), (SPRAY, 0.02), (SIZE, 0.09), (DENSITY, 40.0), (SHAPE, 1.0), (FREEZE, 1.0), (JITTER, 0.5), (ATTACK, 0.3), (RELEASE, 3.0), (SMEAR, 0.25), (CLOUD, 0.4), (SPREAD, 0.5)],
        "Shimmer" => &[(SOURCE, 2.0), (POSITION, 0.3), (SPRAY, 0.3), (SIZE, 0.18), (DENSITY, 30.0), (OCTAVES, 0.4), (OCTAVE_WAY, 1.0), (TONE, 0.15), (SMEAR, 0.35), (CLOUD, 0.55), (SPREAD, 0.9), (ATTACK, 0.4), (RELEASE, 3.0)],
        "Glitch Stutter" => &[(SOURCE, 1.0), (POSITION, 0.2), (SPRAY, 0.4), (SIZE, 0.06), (SYNC, 5.0), (JITTER, 0.0), (SHAPE, 4.0), (REVERSE, 0.3), (PITCH_SPRAY, 0.0), (OCTAVES, 0.2), (ATTACK, 0.001), (RELEASE, 0.2), (CLOUD, 0.1), (SPREAD, 0.8)],
        "Time Stretch" => &[(SOURCE, 1.0), (POSITION, 0.0), (SPRAY, 0.005), (SIZE, 0.12), (DENSITY, 30.0), (SHAPE, 1.0), (SCAN, 4.0), (SPEED, 0.25), (PITCH_SPRAY, 0.0), (ATTACK, 0.05), (RELEASE, 0.6), (CLOUD, 0.15), (SPREAD, 0.2)],
        "Swarm" => &[(SOURCE, 0.0), (POSITION, 0.5), (SPRAY, 0.5), (SIZE, 0.035), (DENSITY, 150.0), (JITTER, 1.0), (SHAPE, 2.0), (SCAN, 3.0), (SPEED, 0.1), (PITCH_SPRAY, 0.35), (CLOUD, 0.2), (SPREAD, 1.0)],
        "Reverse Rain" => &[(SOURCE, 2.0), (POSITION, 0.5), (SPRAY, 0.6), (SIZE, 0.3), (DENSITY, 9.0), (JITTER, 0.8), (SHAPE, 3.0), (REVERSE, 1.0), (PITCH_SPRAY, 0.0), (OCTAVES, 0.25), (CLOUD, 0.45), (SPREAD, 0.9), (RELEASE, 2.5)],
        "Octave Spray" => &[(SOURCE, 0.0), (POSITION, 0.3), (SPRAY, 0.1), (SIZE, 0.15), (DENSITY, 20.0), (OCTAVES, 0.6), (OCTAVE_WAY, 0.0), (CLOUD, 0.3), (SPREAD, 0.8)],
        "Breath Drone" => &[(SOURCE, 3.0), (POSITION, 0.5), (SPRAY, 0.3), (SIZE, 0.4), (DENSITY, 12.0), (SCAN, 1.0), (SPEED, 0.02), (ATTACK, 1.5), (RELEASE, 4.0), (CLOUD, 0.5), (SPREAD, 0.8), (TONE, -0.2)],
        "Plucked Grains" => &[(SOURCE, 0.0), (POSITION, 0.25), (SPRAY, 0.05), (SIZE, 0.08), (DENSITY, 14.0), (SHAPE, 3.0), (ATTACK, 0.001), (DECAY, 0.6), (SUSTAIN, 0.0), (RELEASE, 0.4), (VEL_DENSITY, 0.5), (CLOUD, 0.25)],
        "Choir Smear" => &[(SOURCE, 1.0), (POSITION, 0.6), (SPRAY, 0.2), (SIZE, 0.3), (DENSITY, 16.0), (SCAN, 3.0), (SPEED, 0.03), (PITCH_SPRAY, 0.12), (SMEAR, 0.55), (CLOUD, 0.45), (ATTACK, 0.6), (RELEASE, 3.0)],
        "Tape Scrub" => &[(SOURCE, 0.0), (POSITION, 0.1), (SPRAY, 0.01), (SIZE, 0.05), (DENSITY, 60.0), (SHAPE, 1.0), (SCAN, 3.0), (SPEED, 0.6), (PITCH_SPRAY, 0.0), (JITTER, 0.1), (CLOUD, 0.1), (SPREAD, 0.3)],
        _ => &[],
    };
    for &(i, v) in set {
        p[i] = v;
    }
    p
}

/// A grain window at `t` (0..1) of shape `shape` (see [`SHAPES`]).
pub fn window(shape: usize, t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    match shape {
        0 => {
            // A bell, its feet lowered to zero at the ends.
            let edge = (-0.5f32 * (0.5 / 0.15f32).powi(2)).exp();
            let g = (-0.5 * ((t - 0.5) / 0.15).powi(2)).exp();
            ((g - edge) / (1.0 - edge)).max(0.0)
        }
        1 => 0.5 - 0.5 * (std::f32::consts::TAU * t).cos(),
        2 => 1.0 - (2.0 * t - 1.0).abs(),
        3 => {
            // Struck: up in 3 %, then down.
            let a = 0.03;
            if t < a {
                t / a
            } else {
                (1.0 - (t - a) / (1.0 - a)).powf(2.5)
            }
        }
        _ => {
            // Square, with short ramps so it does not click.
            let ramp = 0.08;
            (t / ramp).min((1.0 - t) / ramp).min(1.0)
        }
    }
}

/// Folds a place back into 0..1, as a ball bounces between two walls.
fn fold(x: f32) -> f32 {
    let x = x.rem_euclid(2.0);
    if x > 1.0 {
        2.0 - x
    } else {
        x
    }
}

/// Where a voice's grains come from (0..1 of the sample), with the shared
/// scan at `scan` and the voice's own (Held) at `run`.
pub fn place(p: &[f32], scan: f32, run: f32) -> f32 {
    let base = p[POSITION].clamp(0.0, 1.0);
    match p[SCAN].round() as i32 {
        1 | 2 => (base + scan).rem_euclid(1.0),
        3 => fold(base + scan),
        4 => (base + run).min(1.0),
        _ => base,
    }
}

/// The sample the grains come from: its own, or the built-in sound Source
/// picks (once [`warm`] has made them; never made here, on the audio thread).
pub fn source<'a>(p: &[f32], sample: Option<&'a SampleRef>) -> Option<&'a SampleData> {
    match sample {
        Some(s) => Some(&**s),
        None => builtin(p.get(SOURCE).copied().unwrap_or(0.0).round().max(0.0) as usize).map(|s| &**s),
    }
}

static BUILTIN: OnceLock<Vec<SampleRef>> = OnceLock::new();

/// Makes the built-in sounds (once). Call it before the audio thread needs them.
pub fn warm() {
    BUILTIN.get_or_init(|| (0..SOURCES.len()).map(make_source).collect());
}

/// A built-in sound, if they have been made.
pub fn builtin(i: usize) -> Option<&'static SampleRef> {
    BUILTIN.get().and_then(|v| v.get(i.min(SOURCES.len() - 1)))
}

const SRC_RATE: u32 = 44_100;
const SRC_SECONDS: f32 = 4.0;
/// Middle C: the built-in sounds' pitch (the Root they play at as they are).
const C4: f64 = 261.625_565;

/// A sine partial added to `buf` from `start`, dying away over `decay`
/// seconds (forever when 0), turned with a rotation rather than a sine per sample.
fn partial(buf: &mut [f32], start: usize, freq: f64, amp: f32, decay: f32, phase: f64) {
    let sr = SRC_RATE as f64;
    let w = std::f64::consts::TAU * freq / sr;
    let (dc, ds) = (w.cos(), w.sin());
    let (mut c, mut s) = (phase.cos(), phase.sin());
    let fall = if decay > 0.0 { (-4.6 / (decay as f64 * sr)).exp() } else { 1.0 };
    let mut a = amp as f64;
    for x in buf.iter_mut().skip(start) {
        *x += (s * a) as f32;
        (c, s) = (c * dc - s * ds, s * dc + c * ds);
        a *= fall;
        if a < 1e-5 {
            break;
        }
    }
}

/// One of the built-in sounds, all on middle C: a pad brightening and
/// darkening again, a choir singing a, e, i, o, u, eight bells struck on a
/// pentatonic scale, and breath whistling on C.
fn make_source(which: usize) -> SampleRef {
    let n = (SRC_RATE as f32 * SRC_SECONDS) as usize;
    let sr = SRC_RATE as f32;
    let (mut l, mut r) = (vec![0.0f32; n], vec![0.0f32; n]);
    let mut rng = Rng::new(0x6A11_0000 + which as u32);
    match which {
        0 => {
            // Three detuned saws (twelve partials each), per channel; how
            // bright they are moves along the sample.
            for (ch, buf) in [&mut l, &mut r].into_iter().enumerate() {
                for (k, det) in [-0.006f64, 0.0, 0.0071].iter().enumerate() {
                    let f0 = C4 * (1.0 + det * if ch == 0 { 1.0 } else { -0.8 });
                    for h in 1..=12 {
                        let phase = rng.unipolar() as f64 * std::f64::consts::TAU;
                        let mut part = vec![0.0f32; n];
                        partial(&mut part, 0, f0 * h as f64, 0.3 / h as f32, 0.0, phase);
                        for (i, x) in part.iter().enumerate() {
                            // Brightness: dark at the ends, open in the middle.
                            let t = i as f32 / n as f32;
                            let open = 0.25 + 0.75 * (std::f32::consts::PI * t).sin();
                            let tilt = (-(h as f32 - 1.0) * (1.2 - open) * 0.6).exp();
                            buf[i] += x * tilt * if k == 1 { 1.0 } else { 0.8 };
                        }
                    }
                }
            }
        }
        1 => {
            // A pulse of breathy throat through three formants, gliding
            // through the vowels; two singers, a little apart.
            const VOWELS: [[f32; 3]; 5] = [[800.0, 1150.0, 2800.0], [400.0, 1600.0, 2700.0], [300.0, 2200.0, 3000.0], [450.0, 800.0, 2830.0], [325.0, 700.0, 2530.0]];
            const GAINS: [f32; 3] = [1.0, 0.5, 0.25];
            for (ch, buf) in [&mut l, &mut r].into_iter().enumerate() {
                let f0 = C4 as f32 * if ch == 0 { 1.0 } else { 1.004 };
                let mut ph = 0.0f32;
                let mut f = [Svf::new(), Svf::new(), Svf::new()];
                for (i, x) in buf.iter_mut().enumerate() {
                    let t = i as f32 / sr;
                    if i % 32 == 0 {
                        let at = (i as f32 / n as f32 * (VOWELS.len() - 1) as f32).min((VOWELS.len() - 1) as f32 - 1e-4);
                        let (a, u) = (at as usize, at.fract());
                        for (k, fk) in f.iter_mut().enumerate() {
                            fk.set(VOWELS[a][k] + (VOWELS[a + 1][k] - VOWELS[a][k]) * u, 0.88, sr);
                        }
                    }
                    let vib = 1.0 + 0.004 * (std::f32::consts::TAU * 5.3 * t + ch as f32).sin();
                    ph = (ph + f0 * vib / sr).fract();
                    let throat = (1.0 - 2.0 * ph) * 0.6 + rng.bipolar() * 0.08;
                    let k = 2.0 - 1.97 * 0.88;
                    *x = f.iter_mut().zip(GAINS).map(|(fk, g)| fk.process(throat, FilterMode::Band) * k * g).sum();
                }
            }
        }
        2 => {
            // Bell partials (hum, prime, tierce, quint, nominal and above),
            // each strike a note of the pentatonic scale from C.
            const STEPS: [f32; 8] = [0.0, 2.0, 4.0, 7.0, 9.0, 12.0, 7.0, 4.0];
            const PARTS: [(f64, f32, f32); 7] = [(0.5, 0.5, 3.0), (1.0, 0.8, 2.2), (1.19, 0.4, 1.6), (1.5, 0.3, 1.2), (2.0, 0.45, 1.0), (2.51, 0.2, 0.6), (3.01, 0.15, 0.4)];
            for (k, step) in STEPS.iter().enumerate() {
                let at = ((0.02 + k as f32 * 0.5) * sr) as usize;
                let f0 = C4 * 2f64.powf(*step as f64 / 12.0);
                for (ch, buf) in [&mut l, &mut r].into_iter().enumerate() {
                    for (j, &(ratio, amp, decay)) in PARTS.iter().enumerate() {
                        // A touch of beating between the two sides.
                        let beat = 1.0 + if ch == 0 { 0.0007 } else { -0.0007 } * j as f64;
                        partial(buf, at, f0 * ratio * beat, amp, decay, rng.unipolar() as f64);
                    }
                }
            }
        }
        _ => {
            // Noise through narrow filters on C and its first overtones,
            // in slow gusts.
            for buf in [&mut l, &mut r] {
                let mut f = [Svf::new(), Svf::new(), Svf::new()];
                for (k, fk) in f.iter_mut().enumerate() {
                    fk.set(C4 as f32 * (k + 1) as f32, 0.985, sr);
                }
                let mut air = Svf::new();
                air.set(1800.0, 0.2, sr);
                let k = 2.0 - 1.97 * 0.985;
                for (i, x) in buf.iter_mut().enumerate() {
                    let t = i as f32 / sr;
                    let gust = 0.6 + 0.25 * (0.7 * t).sin() + 0.15 * (1.9 * t + 1.0).sin();
                    let w = rng.bipolar();
                    let tone: f32 = f.iter_mut().zip([1.0, 0.55, 0.3]).map(|(fk, g)| fk.process(w, FilterMode::Band) * k * g * 9.0).sum();
                    *x = (tone + air.process(w, FilterMode::Band) * 0.25) * gust;
                }
            }
        }
    }
    // As loud as each other, faded in and out.
    let peak = l.iter().chain(&r).fold(0.0f32, |m, x| m.max(x.abs())).max(1e-6);
    let fade = (0.01 * sr) as usize;
    for i in 0..n {
        let g = 0.8 / peak * (i.min(n - 1 - i) as f32 / fade as f32).min(1.0);
        l[i] *= g;
        r[i] *= g;
    }
    SampleRef::new(SampleData::from_buffers(SOURCES[which], None, SRC_RATE, l, r))
}

#[derive(Clone, Copy, Default)]
struct Grain {
    on: bool,
    voice: u8,
    /// Samples to wait before it starts (in the chunk it was made in).
    delay: u32,
    /// Where it reads in the sample (frames) and how far each sample.
    pos: f64,
    step: f64,
    age: u32,
    len: u32,
    gl: f32,
    gr: f32,
    shape: u8,
    /// For the screen: its pitch (semitones from the sample's own) and how loud it is now.
    semis: f32,
    level: f32,
}

#[derive(Clone, Copy)]
struct Voice {
    note: u8,
    gate: bool,
    vel: f32,
    age: u64,
    env: Adsr,
    /// Samples until its next grain.
    wait: f64,
    /// How far its own scan has gone (Held), as a part of the sample.
    run: f32,
}

impl Voice {
    const fn new() -> Self {
        Self { note: 0, gate: false, vel: 0.0, age: 0, env: Adsr::new(), wait: 0.0, run: 0.0 }
    }

    fn active(&self) -> bool {
        self.env.is_active()
    }
}

pub struct GranulatorState {
    pub pitch_bend: f32,
    /// The song's tempo, for Sync.
    pub bpm: f32,
    sr: f32,
    voices: [Voice; MAX_VOICES],
    grains: [Grain; POOL],
    windows: [[f32; WINDOW + 1]; 5],
    counter: u64,
    rng: Rng,
    /// The shared scan, as a part of the sample (0..2 for Ping-Pong).
    scan: f32,
    /// The length of the sample last played (a new one clears the grains).
    frames: usize,
    tone: [Svf; 2],
    smear: [Vec<f32>; 2],
    smear_at: usize,
    smear_time: f32,
    smear_lp: [f32; 2],
    reverb: Reverb,
    /// Grains made so far.
    made: u64,
}

impl GranulatorState {
    pub fn new(sr: f32) -> Self {
        let mut windows = [[0.0; WINDOW + 1]; 5];
        for (s, w) in windows.iter_mut().enumerate() {
            for (i, v) in w.iter_mut().enumerate() {
                *v = window(s, i as f32 / WINDOW as f32);
            }
        }
        let smear_len = (SMEAR_MAX * sr) as usize + 4;
        Self {
            pitch_bend: 0.0,
            bpm: 120.0,
            sr,
            voices: [Voice::new(); MAX_VOICES],
            grains: [Grain::default(); POOL],
            windows,
            counter: 0,
            rng: Rng::new(0x6A11_C10D),
            scan: 0.0,
            frames: 0,
            tone: [Svf::new(), Svf::new()],
            smear: [vec![0.0; smear_len], vec![0.0; smear_len]],
            smear_at: 0,
            smear_time: 0.1 * sr,
            smear_lp: [0.0; 2],
            reverb: Reverb::new(sr),
            made: 0,
        }
    }

    pub fn note_on(&mut self, p: &[f32], note: u8, vel: f32) {
        if p.len() < SPECS.len() {
            return;
        }
        let n = (p[VOICES].round() as usize).clamp(1, MAX_VOICES);
        let pool = &self.voices[..n];
        let pick = pool
            .iter()
            .position(|v| v.gate && v.note == note)
            .or_else(|| pool.iter().position(|v| !v.active()))
            .or_else(|| pool.iter().enumerate().filter(|(_, v)| !v.gate).min_by_key(|(_, v)| v.age).map(|(i, _)| i))
            .unwrap_or_else(|| pool.iter().enumerate().min_by_key(|(_, v)| v.age).map(|(i, _)| i).unwrap_or(0));
        self.counter += 1;
        let v = &mut self.voices[pick];
        v.note = note;
        v.gate = true;
        v.vel = vel.clamp(0.0, 1.0);
        v.age = self.counter;
        v.env.trigger();
        // The first grain at once, its own scan from Position.
        v.wait = 0.0;
        v.run = 0.0;
    }

    pub fn note_off(&mut self, _p: &[f32], note: u8) {
        for v in self.voices.iter_mut().filter(|v| v.gate && v.note == note) {
            v.gate = false;
            v.env.release();
        }
    }

    pub fn all_off(&mut self) {
        for v in self.voices.iter_mut() {
            v.gate = false;
            v.env.release();
        }
    }

    /// Grains sounding now.
    pub fn grains_on(&self) -> usize {
        self.grains.iter().filter(|g| g.on).count()
    }


    /// Samples between two grains of a voice.
    fn interval(&mut self, p: &[f32], vel: f32) -> f64 {
        let sync = (p[SYNC].round().max(0.0) as usize).min(SYNC_BEATS.len() - 1);
        if sync > 0 {
            return (SYNC_BEATS[sync] * 60.0 / self.bpm.clamp(20.0, 999.0) as f64 * self.sr as f64).max(1.0);
        }
        let density = p[DENSITY].max(0.1) * (1.0 - p[VEL_DENSITY].clamp(0.0, 1.0) * (1.0 - vel)).max(0.05);
        let jitter = 1.0 + p[JITTER].clamp(0.0, 1.0) * 0.9 * self.rng.bipolar();
        (self.sr as f64 / density as f64 * jitter as f64).max(1.0)
    }

    /// Grains a second (for the level: the more overlap, the softer each).
    fn rate(&self, p: &[f32]) -> f32 {
        let sync = (p[SYNC].round().max(0.0) as usize).min(SYNC_BEATS.len() - 1);
        if sync > 0 {
            (self.bpm.clamp(20.0, 999.0) as f64 / 60.0 / SYNC_BEATS[sync]) as f32
        } else {
            p[DENSITY].max(0.1)
        }
    }

    /// Starts a grain for voice `vi`, `delay` samples into this chunk; none
    /// when the pool is full.
    fn spawn(&mut self, p: &[f32], src: &SampleData, vi: usize, delay: usize) {
        let Some(slot) = self.grains.iter().position(|g| !g.on) else { return };
        let v = self.voices[vi];
        let rng = &mut self.rng;
        let mut semis = p[PITCH].round() + p[FINE] / 100.0 + self.pitch_bend;
        if p[KEYS] >= 0.5 {
            semis += v.note as f32 - p[ROOT].round();
        }
        semis += p[PITCH_SPRAY].max(0.0) * rng.bipolar();
        if rng.unipolar() < p[OCTAVES] {
            semis += match p[OCTAVE_WAY].round() as i32 {
                1 => 12.0,
                2 => -12.0,
                _ if rng.unipolar() < 0.5 => -12.0,
                _ => 12.0,
            };
        }
        let semis = semis.clamp(-72.0, 72.0);
        let reverse = rng.unipolar() < p[REVERSE];
        let x = fold(place(p, self.scan, v.run) + p[SPRAY].clamp(0.0, 1.0) * 0.5 * self.rng.bipolar());
        let frames = src.frames();
        let last = (frames - 1) as f64;
        let len = (p[SIZE].max(0.0005) * self.sr).max(8.0) as u32;
        let speed = 2f64.powf(semis as f64 / 12.0) * src.sample_rate as f64 / self.sr as f64;
        let span = len as f64 * speed;
        // The grain reads from where it is placed on; backwards it reads the same stretch from its end.
        let mut start = x as f64 * last;
        if span < last {
            start = start.min(last - span);
        }
        let (pos, step) = if reverse { ((start + span).min(last), -speed) } else { (start, speed) };
        let overlap = self.rate(p) * len as f32 / self.sr;
        let vel_amt = p[VELOCITY].clamp(0.0, 1.0);
        let level = (1.0 - vel_amt * (1.0 - v.vel)) / overlap.max(1.0).sqrt();
        let (pl, pr) = pan_gains(p[SPREAD].clamp(0.0, 1.0) * self.rng.bipolar());
        self.grains[slot] = Grain {
            on: true,
            voice: vi as u8,
            delay: delay as u32,
            pos,
            step,
            age: 0,
            len,
            gl: level * pl,
            gr: level * pr,
            shape: (p[SHAPE].round().max(0.0) as usize).min(SHAPES.len() - 1) as u8,
            semis,
            level: 0.0,
        };
        self.made += 1;
    }

    /// For the screen: grains sounding, the shared scan's place, voices
    /// sounding, the sample's length (seconds); from 4 each voice's place
    /// (-1: silent); from [`SHOWN_AT`] up to [`SHOWN`] grains, two values
    /// each: the place in the sample plus 16 when backwards plus its level
    /// in fifteenths as whole numbers, and its pitch in semitones.
    pub fn visual(&self, out: &mut [f32; crate::engine::INST_VALS]) {
        out[0] = self.grains_on() as f32;
        out[1] = self.scan;
        out[2] = self.voices.iter().filter(|v| v.active()).count() as f32;
        out[3] = self.frames as f32;
        for (k, v) in self.voices.iter().enumerate() {
            out[4 + k] = if v.active() { v.run } else { -1.0 };
        }
        let mut k = 0;
        for g in self.grains.iter().filter(|g| g.on && g.delay == 0) {
            if k >= SHOWN {
                break;
            }
            let place = (g.pos / self.frames.max(1) as f64).clamp(0.0, 0.9999) as f32;
            let whole = (g.level.clamp(0.0, 1.0) * 15.0).round() + if g.step < 0.0 { 16.0 } else { 0.0 };
            out[SHOWN_AT + 2 * k] = whole + place;
            out[SHOWN_AT + 2 * k + 1] = g.semis;
            k += 1;
        }
        for v in out[SHOWN_AT + 2 * k..].iter_mut() {
            *v = -1.0;
        }
    }

    pub fn render(&mut self, p: &[f32], sample: Option<&SampleRef>, l: &mut [f32], r: &mut [f32]) {
        if p.len() < SPECS.len() {
            return;
        }
        let Some(src) = source(p, sample) else { return };
        let frames = src.frames();
        if frames < 2 {
            return;
        }
        if frames != self.frames {
            // Another sample: what played from the old one stops.
            for g in self.grains.iter_mut() {
                g.on = false;
            }
            self.frames = frames;
        }
        let n = l.len().min(r.len());
        let sr = self.sr;
        let coefs = AdsrCoefs::new(p[ATTACK], p[DECAY], p[SUSTAIN], p[RELEASE], sr);
        let scan_mode = p[SCAN].round() as i32;
        let frozen = p[FREEZE] >= 0.5;
        // How far the scan moves a sample: Speed 1 is as fast as the sample plays.
        let scan_step = if frozen { 0.0 } else { p[SPEED].max(0.0) * src.sample_rate as f32 / sr / frames as f32 };
        let gain = db_to_gain(p[VOLUME]) * 0.7;
        // The tone: darker to the left, thinner to the right.
        let tone = p[TONE].clamp(-1.0, 1.0);
        let tone_mode = if tone < -0.01 {
            let c = 18_000.0 * 2f32.powf(tone * 8.0);
            self.tone.iter_mut().for_each(|f| f.set(c, 0.1, sr));
            Some(FilterMode::Low)
        } else if tone > 0.01 {
            let c = 30.0 * 2f32.powf(tone * 8.0);
            self.tone.iter_mut().for_each(|f| f.set(c, 0.1, sr));
            Some(FilterMode::High)
        } else {
            None
        };
        let smear = p[SMEAR].clamp(0.0, 1.0);
        let smear_target = ((0.04 + p[SIZE] * 0.5).min(SMEAR_MAX * 0.9)) * sr;
        let smear_lp = 1.0 - (-std::f32::consts::TAU * 4000.0 / sr).exp();
        let cloud = p[CLOUD].clamp(0.0, 1.0);
        let mut done = 0;
        while done < n {
            let m = (n - done).min(CHUNK);
            match scan_mode {
                1 | 3 => self.scan = (self.scan + scan_step * m as f32).rem_euclid(2.0),
                2 => self.scan = (self.scan - scan_step * m as f32).rem_euclid(2.0),
                _ => {}
            }
            // The voices: their envelopes for this chunk, their grains made.
            let mut env = [[0.0f32; CHUNK]; MAX_VOICES];
            for vi in 0..MAX_VOICES {
                if !self.voices[vi].active() {
                    continue;
                }
                for (i, e) in env[vi][..m].iter_mut().enumerate() {
                    *e = self.voices[vi].env.next(&coefs);
                    if scan_mode == 4 {
                        let v = &mut self.voices[vi];
                        v.run = (v.run + scan_step).min(1.0);
                    }
                    self.voices[vi].wait -= 1.0;
                    if self.voices[vi].wait <= 0.0 && self.voices[vi].active() {
                        self.spawn(p, src, vi, i);
                        let vel = self.voices[vi].vel;
                        let next = self.interval(p, vel);
                        self.voices[vi].wait += next;
                    }
                }
                if !self.voices[vi].active() {
                    for g in self.grains.iter_mut().filter(|g| g.on && g.voice as usize == vi) {
                        g.on = false;
                    }
                }
            }
            // The grains.
            let (mut bl, mut br) = ([0.0f32; CHUNK], [0.0f32; CHUNK]);
            let last = frames as f64;
            for g in self.grains.iter_mut().filter(|g| g.on) {
                let e = &env[g.voice as usize % MAX_VOICES];
                let w = &self.windows[g.shape as usize % 5];
                let from = (g.delay as usize).min(m);
                g.delay = 0;
                let inv = WINDOW as f32 / g.len as f32;
                for i in from..m {
                    let t = g.age as f32 * inv;
                    let k = (t as usize).min(WINDOW - 1);
                    let win = w[k] + (w[k + 1] - w[k]) * (t - k as f32);
                    let a = g.pos.floor();
                    let j = a as usize % frames;
                    let j2 = if j + 1 >= frames { 0 } else { j + 1 };
                    let f = (g.pos - a) as f32;
                    let sl = src.left[j] + (src.left[j2] - src.left[j]) * f;
                    let sr_ = src.right[j] + (src.right[j2] - src.right[j]) * f;
                    let amp = win * e[i];
                    bl[i] += sl * amp * g.gl;
                    br[i] += sr_ * amp * g.gr;
                    g.level = amp;
                    g.pos += g.step;
                    if g.pos >= last || g.pos < 0.0 {
                        g.pos = g.pos.rem_euclid(last);
                    }
                    g.age += 1;
                    if g.age >= g.len {
                        g.on = false;
                        break;
                    }
                }
            }
            // Tone, smear, cloud.
            if let Some(mode) = tone_mode {
                for i in 0..m {
                    bl[i] = self.tone[0].process(bl[i], mode);
                    br[i] = self.tone[1].process(br[i], mode);
                }
            }
            let len = self.smear[0].len();
            for i in 0..m {
                self.smear_time += (smear_target - self.smear_time) * 0.0005;
                let back = self.smear_time.clamp(1.0, (len - 2) as f32);
                let at = self.smear_at as f32 - back + len as f32;
                let k = at as usize;
                let f = at - k as f32;
                let read = |b: &[f32]| b[k % len] + (b[(k + 1) % len] - b[k % len]) * f;
                let (dl, dr) = (read(&self.smear[0]), read(&self.smear[1]));
                self.smear_lp[0] += (dl - self.smear_lp[0]) * smear_lp;
                self.smear_lp[1] += (dr - self.smear_lp[1]) * smear_lp;
                // Each side fed from the other, so the smear wanders.
                let fb = smear * 0.82;
                self.smear[0][self.smear_at] = bl[i] + self.smear_lp[1] * fb;
                self.smear[1][self.smear_at] = br[i] + self.smear_lp[0] * fb;
                self.smear_at = (self.smear_at + 1) % len;
                bl[i] += self.smear_lp[0] * smear * 0.7;
                br[i] += self.smear_lp[1] * smear * 0.7;
            }
            if cloud > 0.001 {
                self.reverb.process(0.86, 0.45, 1.0, 15.0, 120.0, cloud * 0.6, &mut bl[..m], &mut br[..m]);
            }
            for i in 0..m {
                l[done + i] += bl[i] * gain;
                r[done + i] += br[i] * gain;
            }
            done += m;
        }
    }

    #[cfg(test)]
    fn grain_places(&self) -> Vec<(f64, f64)> {
        self.grains.iter().filter(|g| g.on).map(|g| (g.pos, g.step)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 48_000.0;

    fn sine(freq: f32, secs: f32) -> SampleRef {
        let n = (SR * secs) as usize;
        let v: Vec<f32> = (0..n).map(|i| (std::f32::consts::TAU * freq * i as f32 / SR).sin() * 0.8).collect();
        SampleRef::new(SampleData::from_buffers("sine", None, SR as u32, v.clone(), v))
    }

    /// A plain cloud: no spray, no pitch spray, no effects.
    fn plain() -> Vec<f32> {
        let mut p = crate::params::defaults(SPECS);
        for (i, v) in [(SPRAY, 0.0), (PITCH_SPRAY, 0.0), (JITTER, 0.0), (CLOUD, 0.0), (SMEAR, 0.0), (SHAPE, 1.0), (ATTACK, 0.002), (RELEASE, 0.05), (SPREAD, 0.0), (POSITION, 0.3)] {
            p[i] = v;
        }
        p
    }

    fn run(st: &mut GranulatorState, p: &[f32], s: Option<&SampleRef>, secs: f32) -> (Vec<f32>, Vec<f32>) {
        let (mut ol, mut or) = (Vec::new(), Vec::new());
        for _ in 0..(secs * SR / 256.0) as usize {
            let (mut l, mut r) = ([0.0f32; 256], [0.0f32; 256]);
            st.render(p, s, &mut l, &mut r);
            ol.extend_from_slice(&l);
            or.extend_from_slice(&r);
        }
        (ol, or)
    }

    /// The strength of `freq` in `x` (Goertzel).
    fn power(x: &[f32], freq: f32) -> f32 {
        let w = std::f32::consts::TAU * freq / SR;
        let c = 2.0 * w.cos();
        let (mut s1, mut s2) = (0.0f32, 0.0f32);
        for &v in x {
            let s = v + c * s1 - s2;
            s2 = s1;
            s1 = s;
        }
        (s1 * s1 + s2 * s2 - c * s1 * s2) / (x.len() as f32).powi(2)
    }

    /// The strength of the band 30 Hz either side of `freq` (grains spread a
    /// tone over the lines of their rate).
    fn band(x: &[f32], freq: f32) -> f32 {
        (-15..=15).map(|k| power(x, freq + 2.0 * k as f32)).sum()
    }

    fn peak(x: &[f32]) -> f32 {
        x.iter().fold(0.0f32, |m, v| m.max(v.abs()))
    }

    #[test]
    fn an_octave_up_comes_out_an_octave_up() {
        let s = sine(440.0, 1.0);
        let mut p = plain();
        p[SIZE] = 0.08;
        p[DENSITY] = 40.0;
        let mut st = GranulatorState::new(SR);
        st.note_on(&p, 72, 1.0);
        let (l, _) = run(&mut st, &p, Some(&s), 1.0);
        let x = &l[4800..];
        let (up, same) = (band(x, 880.0), band(x, 440.0));
        assert!(up > 0.0001 && up > same * 50.0, "880 Hz {up}, 440 Hz {same}");
        // And the key a fifth down a fifth down.
        let mut st = GranulatorState::new(SR);
        st.note_on(&p, 53, 1.0);
        let (l, _) = run(&mut st, &p, Some(&s), 1.0);
        let fifth = 440.0 * 2f32.powf(-7.0 / 12.0);
        assert!(band(&l[4800..], fifth) > band(&l[4800..], 440.0) * 50.0);
    }

    #[test]
    fn pitch_and_keys_off_play_the_sample_as_it_is() {
        let s = sine(440.0, 1.0);
        let mut p = plain();
        p[KEYS] = 0.0;
        p[PITCH] = 12.0;
        let mut st = GranulatorState::new(SR);
        st.note_on(&p, 40, 1.0);
        let (l, _) = run(&mut st, &p, Some(&s), 0.6);
        assert!(band(&l[4800..], 880.0) > band(&l[4800..], 440.0) * 50.0);
    }

    #[test]
    fn density_sets_how_many_grains_come_and_size_how_long_they_last() {
        let s = sine(440.0, 2.0);
        let mut p = plain();
        p[DENSITY] = 50.0;
        p[SIZE] = 0.04;
        let mut st = GranulatorState::new(SR);
        st.note_on(&p, 60, 1.0);
        let mut on = 0usize;
        let blocks = (2.0 * SR / 256.0) as usize;
        for _ in 0..blocks {
            let (mut l, mut r) = ([0.0f32; 256], [0.0f32; 256]);
            st.render(&p, Some(&s), &mut l, &mut r);
            on += st.grains_on();
        }
        let made = st.made as f32;
        assert!((made - 100.0).abs() <= 2.0, "{made} grains in two seconds at 50 a second");
        // Fifty a second of 40 ms each: two at a time.
        let mean = on as f32 / blocks as f32;
        assert!((mean - 2.0).abs() < 0.3, "{mean} at once");
        // On the beat: sixteenths at 120 are eight a second.
        p[SYNC] = 5.0;
        let mut st = GranulatorState::new(SR);
        st.bpm = 120.0;
        st.note_on(&p, 60, 1.0);
        run(&mut st, &p, Some(&s), 2.0);
        assert!((st.made as f32 - 16.0).abs() <= 1.0, "{} sixteenths in two seconds", st.made);
    }

    #[test]
    fn grains_stay_inside_the_sample_however_far_they_spray() {
        let s = sine(300.0, 0.5);
        let frames = s.frames() as f64;
        for (pos, spray, scan) in [(0.0, 1.0, 0.0), (1.0, 1.0, 1.0), (0.98, 0.5, 3.0), (0.5, 0.0, 2.0), (0.9, 1.0, 4.0)] {
            let mut p = plain();
            (p[POSITION], p[SPRAY], p[SCAN], p[SPEED], p[SIZE], p[DENSITY]) = (pos, spray, scan, 3.0, 0.3, 120.0);
            p[REVERSE] = 0.5;
            p[PITCH_SPRAY] = 12.0;
            let mut st = GranulatorState::new(SR);
            st.note_on(&p, 84, 1.0);
            for _ in 0..200 {
                let (mut l, mut r) = ([0.0f32; 256], [0.0f32; 256]);
                st.render(&p, Some(&s), &mut l, &mut r);
                for (at, _) in st.grain_places() {
                    assert!((0.0..frames).contains(&at), "a grain at {at} of {frames}");
                }
            }
        }
        // Without spray they start where Position says.
        let mut p = plain();
        p[POSITION] = 0.5;
        p[SIZE] = 0.01;
        let mut st = GranulatorState::new(SR);
        st.note_on(&p, 60, 1.0);
        let (mut l, mut r) = ([0.0f32; 16], [0.0f32; 16]);
        st.render(&p, Some(&s), &mut l, &mut r);
        let (at, _) = st.grain_places()[0];
        assert!((at - 0.5 * (frames - 1.0)).abs() < 20.0, "{at}");
    }

    #[test]
    fn reversed_grains_play_backwards() {
        // A rising ramp: forwards the grain rises, backwards it falls.
        let n = SR as usize;
        let ramp: Vec<f32> = (0..n).map(|i| i as f32 / n as f32 * 2.0 - 1.0).collect();
        let s = SampleRef::new(SampleData::from_buffers("ramp", None, SR as u32, ramp.clone(), ramp));
        for reverse in [0.0, 1.0] {
            let mut p = plain();
            (p[REVERSE], p[SHAPE], p[SIZE], p[DENSITY], p[POSITION], p[ATTACK], p[SUSTAIN]) = (reverse, 4.0, 0.2, 1.0, 0.4, 0.001, 1.0);
            let mut st = GranulatorState::new(SR);
            st.note_on(&p, 60, 1.0);
            let (mut l, mut r) = (vec![0.0f32; 9600], vec![0.0f32; 9600]);
            st.render(&p, Some(&s), &mut l, &mut r);
            assert!(st.grain_places().iter().all(|g| (g.1 < 0.0) == (reverse > 0.5)));
            // The flat middle of the window, past the attack.
            let mid = &l[3000..6000];
            let falling = mid.windows(2).filter(|w| w[1] < w[0]).count();
            if reverse > 0.5 {
                assert!(falling > mid.len() * 9 / 10, "backwards falls: {falling}");
            } else {
                assert!(falling < mid.len() / 10, "forwards rises: {falling}");
            }
        }
    }

    #[test]
    fn the_pool_fills_but_never_overflows() {
        let s = sine(220.0, 3.0);
        let mut p = plain();
        (p[DENSITY], p[SIZE], p[VOICES]) = (200.0, 2.0, 8.0);
        let mut st = GranulatorState::new(SR);
        for k in 0..8 {
            st.note_on(&p, 60 + k, 1.0);
        }
        let mut most = 0;
        for _ in 0..400 {
            let (mut l, mut r) = ([0.0f32; 256], [0.0f32; 256]);
            st.render(&p, Some(&s), &mut l, &mut r);
            assert!(l.iter().chain(&r).all(|v| v.is_finite()));
            most = most.max(st.grains_on());
            assert!(st.grains_on() <= POOL);
        }
        assert_eq!(most, POOL, "a thick cloud uses the whole pool");
        // The pool is the same array it always was: nothing was added to it.
        assert_eq!(st.grains.len(), POOL);
    }

    #[test]
    fn each_key_plays_a_cloud_of_its_own() {
        let s = sine(440.0, 1.0);
        let mut p = plain();
        p[SIZE] = 0.08;
        p[DENSITY] = 30.0;
        let mut st = GranulatorState::new(SR);
        st.note_on(&p, 60, 1.0);
        st.note_on(&p, 67, 1.0);
        let (l, _) = run(&mut st, &p, Some(&s), 0.8);
        assert_eq!(st.voices.iter().filter(|v| v.active()).count(), 2);
        let x = &l[4800..];
        let fifth = 440.0 * 2f32.powf(7.0 / 12.0);
        let (a, b, off) = (band(x, 440.0), band(x, fifth), band(x, 550.0));
        assert!(a > off * 20.0 && b > off * 20.0, "{a} {b} {off}");
        // One let go: its cloud goes, the other plays on.
        st.note_off(&p, 60);
        let (l, _) = run(&mut st, &p, Some(&s), 0.8);
        let x = &l[l.len() - 9600..];
        assert!(band(x, fifth) > band(x, 440.0) * 50.0);
    }

    #[test]
    fn silence_after_the_release() {
        let s = sine(440.0, 1.0);
        let mut p = plain();
        p[RELEASE] = 0.2;
        let mut st = GranulatorState::new(SR);
        st.note_on(&p, 60, 1.0);
        run(&mut st, &p, Some(&s), 0.5);
        st.note_off(&p, 60);
        run(&mut st, &p, Some(&s), 1.0);
        let (l, r) = run(&mut st, &p, Some(&s), 0.2);
        assert!(peak(&l) < 1e-4 && peak(&r) < 1e-4, "{} {}", peak(&l), peak(&r));
        assert_eq!(st.grains_on(), 0);
    }

    #[test]
    fn every_preset_sounds_from_its_own_source_and_dies_away() {
        warm();
        for name in preset_names() {
            let p = preset(name);
            assert_eq!(p.len(), SPECS.len());
            let mut st = GranulatorState::new(SR);
            st.note_on(&p, 60, 0.9);
            st.note_on(&p, 64, 0.7);
            let (l, r) = run(&mut st, &p, None, 1.5);
            assert!(l.iter().chain(&r).all(|v| v.is_finite()), "{name}");
            assert!(peak(&l) > 0.01, "{name} sounds: {}", peak(&l));
            assert!(peak(&l) < 4.0, "{name} is not too loud: {}", peak(&l));
            st.all_off();
            run(&mut st, &p, None, 15.0);
            let (l, _) = run(&mut st, &p, None, 0.5);
            assert!(peak(&l) < 2e-3, "{name} dies away: {}", peak(&l));
        }
    }

    #[test]
    fn extremes_stay_finite() {
        warm();
        let mut rng = Rng::new(7);
        for _ in 0..12 {
            let mut p: Vec<f32> = SPECS.iter().map(|s| s.from_norm(if rng.unipolar() < 0.5 { 0.0 } else { 1.0 })).collect();
            p[VOLUME] = 0.0;
            let mut st = GranulatorState::new(SR);
            st.pitch_bend = 2.0;
            for k in 0..10 {
                st.note_on(&p, 20 + k * 9, rng.unipolar());
            }
            let (l, r) = run(&mut st, &p, None, 0.5);
            assert!(l.iter().chain(&r).all(|v| v.is_finite() && v.abs() < 50.0));
        }
    }

    #[test]
    fn the_built_in_sounds_are_made_and_differ() {
        warm();
        for i in 0..SOURCES.len() {
            let s = builtin(i).expect("made");
            assert_eq!(s.frames(), (SRC_RATE as f32 * SRC_SECONDS) as usize);
            let pk = s.left.iter().chain(&s.right).fold(0.0f32, |m, v| m.max(v.abs()));
            assert!((pk - 0.8).abs() < 0.01, "{} peaks at {pk}", SOURCES[i]);
        }
        assert!(builtin(0).unwrap().left != builtin(1).unwrap().left);
        // The source param picks one; a sample of its own goes first.
        let mut p = plain();
        p[SOURCE] = 2.0;
        assert_eq!(source(&p, None).unwrap().name, "Bells");
        let s = sine(100.0, 0.1);
        assert_eq!(source(&p, Some(&s)).unwrap().name, "sine");
    }

    #[test]
    fn the_screen_hears_of_the_grains() {
        let s = sine(440.0, 1.0);
        let mut p = plain();
        p[REVERSE] = 1.0;
        p[POSITION] = 0.6;
        // Short grains, always one or more sounding.
        (p[SIZE], p[DENSITY]) = (0.02, 100.0);
        let mut st = GranulatorState::new(SR);
        st.note_on(&p, 60, 1.0);
        run(&mut st, &p, Some(&s), 0.3);
        let mut out = [0.0f32; crate::engine::INST_VALS];
        st.visual(&mut out);
        assert!(out[0] >= 1.0);
        let v = out[SHOWN_AT];
        assert!(v >= 16.0, "backwards");
        let place = v.fract();
        assert!((place - 0.6).abs() < 0.1, "{place}");
    }
}

//! Keytone: a toy sampling keyboard of the mid-eighties. Its sounds are
//! short 8-bit samples at about 9.4 kHz, made here once by synthesis (piano,
//! trumpet, pipe organ, a voice singing "ah", flute, brass ensemble, synth
//! drum, jazz organ and eight more in the same spirit), played back by
//! stepping through them with no interpolation and no filter, eight voices
//! summed into an 8-bit converter whose steps are held until the next: the
//! aliasing and the grain are the sound. A few envelopes, sustain on or off,
//! vibrato and portamento, as the toy had. A sample of your own goes in the
//! user slot and is read the same crude way (decimated, 8 bits).
//!
//! And the drums of its rhythm section: crunchy 8-bit samples made the same
//! way, for the Drum Rack (`KeytoneDrum`, see `drums`).

use crate::params::{ParamSpec, Unit};
use crate::sample::SampleRef;
use std::f32::consts::TAU;
use std::sync::OnceLock;

/// The rate its samples are made and kept at (Hz).
pub const SRC_RATE: f32 = 9375.0;
/// The root of every sound: one cycle in 36 samples (about 260 Hz).
const ROOT_PERIOD: usize = 36;
const ROOT_HZ: f32 = SRC_RATE / ROOT_PERIOD as f32;
pub const VOICES: usize = 8;

pub const SOUND: usize = 0;
pub const ENVELOPE: usize = 1;
pub const SUSTAIN: usize = 2;
pub const VIBRATO: usize = 3;
pub const VIB_DEPTH: usize = 4;
pub const VIB_RATE: usize = 5;
pub const PORTAMENTO: usize = 6;
pub const GLIDE: usize = 7;
pub const OCTAVE: usize = 8;
pub const TUNE: usize = 9;
pub const RATE: usize = 10;
pub const BITS: usize = 11;
pub const TONE: usize = 12;
pub const VELOCITY: usize = 13;
pub const VOLUME: usize = 14;

pub const SOUNDS: &[&str] = &[
    "Piano",
    "Trumpet",
    "Pipe Organ",
    "Human Voice",
    "Flute",
    "Brass Ensemble",
    "Synth Drum",
    "Jazz Organ",
    "Strings",
    "Vibes",
    "Glockenspiel",
    "Synth Bass",
    "Marimba",
    "Harpsichord",
    "Accordion",
    "Clarinet",
    "Sample",
];
/// The user slot: a sample of your own.
pub const SAMPLE_SLOT: usize = 16;
pub const ENVELOPES: &[&str] = &["Sound", "Organ", "Piano", "Pluck", "Swell", "Bell", "Reverse"];
pub const RATES: &[&str] = &["4.7 kHz", "9.4 kHz", "18.8 kHz", "32 kHz"];
const RATE_HZ: [f32; 4] = [4687.5, 9375.0, 18750.0, 32000.0];
pub const BIT_DEPTHS: &[&str] = &["4", "6", "8", "12"];
const BITS_N: [u32; 4] = [4, 6, 8, 12];

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::choice("Sound", "Sound", SOUNDS, 0),
    ParamSpec::choice("Envelope", "Envelope", ENVELOPES, 0),
    ParamSpec::toggle("Sustain", "Envelope", false),
    ParamSpec::toggle("Vibrato", "Effects", false),
    ParamSpec::lin("Vib Depth", "Effects", 0.0, 1.0, 0.35, Unit::Percent),
    ParamSpec::lin("Vib Rate", "Effects", 2.0, 10.0, 5.5, Unit::Hz),
    ParamSpec::toggle("Portamento", "Effects", false),
    ParamSpec::lin("Glide", "Effects", 0.02, 1.0, 0.15, Unit::Sec),
    ParamSpec::int("Octave", "Keys", -2.0, 2.0, 0.0, Unit::None),
    ParamSpec::lin("Tune", "Keys", -100.0, 100.0, 0.0, Unit::Cents),
    ParamSpec::choice("Rate", "Chip", RATES, 1),
    ParamSpec::choice("Bits", "Chip", BIT_DEPTHS, 2),
    ParamSpec::lin("Tone", "Chip", 0.0, 1.0, 0.75, Unit::Percent),
    ParamSpec::toggle("Velocity", "Keys", false),
    ParamSpec::lin("Volume", "Keys", 0.0, 1.0, 0.75, Unit::Percent),
];

pub fn preset_names() -> &'static [&'static str] {
    &[
        "Piano",
        "Trumpet",
        "Pipe Organ",
        "Human Voice",
        "Flute",
        "Brass Ensemble",
        "Synth Drum",
        "Jazz Organ",
        "Strings",
        "Vibes",
        "Glockenspiel",
        "Synth Bass",
        "Marimba",
        "Harpsichord",
        "Accordion",
        "Clarinet",
        "Dancehall Bass",
        "Bubble Organ",
        "Chipmunk Choir",
        "Crunchy Strings",
        "Clean Piano",
        "Glide Lead",
        "Toy Bells",
        "Sample",
    ]
}

pub fn preset(name: &str) -> Vec<f32> {
    let mut p = crate::params::defaults(SPECS);
    if let Some(i) = SOUNDS.iter().position(|s| *s == name) {
        p[SOUND] = i as f32;
        return p;
    }
    let set: &[(usize, f32)] = match name {
        // The sparse synth bass the riddims were built on: low, plucked, a little glide.
        "Dancehall Bass" => &[(SOUND, 11.0), (OCTAVE, -1.0), (ENVELOPE, 2.0), (PORTAMENTO, 1.0), (GLIDE, 0.05)],
        "Bubble Organ" => &[(SOUND, 7.0), (ENVELOPE, 3.0)],
        "Chipmunk Choir" => &[(SOUND, 3.0), (OCTAVE, 1.0), (VIBRATO, 1.0), (VIB_DEPTH, 0.5)],
        "Crunchy Strings" => &[(SOUND, 8.0), (RATE, 0.0), (BITS, 1.0), (SUSTAIN, 1.0)],
        "Clean Piano" => &[(SOUND, 0.0), (RATE, 3.0), (BITS, 3.0), (TONE, 1.0)],
        "Glide Lead" => &[(SOUND, 1.0), (PORTAMENTO, 1.0), (GLIDE, 0.2), (VIBRATO, 1.0), (VIB_DEPTH, 0.3)],
        "Toy Bells" => &[(SOUND, 10.0), (SUSTAIN, 1.0), (TONE, 1.0)],
        "Sample" => &[(SOUND, SAMPLE_SLOT as f32)],
        _ => &[],
    };
    for &(i, v) in set {
        p[i] = v;
    }
    p
}

// ------------------------------------------------------------ the sounds

/// A sound in its 8-bit memory: the samples, where the loop starts (it runs
/// to the end; none: it plays once), and the envelope it comes with.
pub struct Sound {
    pub data: Vec<i8>,
    pub loop_start: Option<usize>,
    pub env: usize,
    /// Semitones it plays above the key (the glockenspiel sounds high).
    pub shift: f32,
}

/// How a sound is made: partials (ratio to the root, level held, level at
/// the start, how fast it gets from one to the other), noise at the start,
/// a pitch that slides into place, the attack's length and the loop's (in
/// root cycles; a partial's ratio is rounded to fit it).
struct Recipe {
    partials: Vec<(f32, f32, f32, f32)>,
    /// Level, time constant (s) and brightness (0 dark, 1 bright).
    noise: (f32, f32, f32),
    /// Pitch off by this much at the start (a ratio, 0.1 = 10% sharp), and how fast it settles.
    scoop: (f32, f32),
    fade_in: f32,
    attack: f32,
    cycles: usize,
    /// A sound that plays once (its length in seconds) and dies away (time constant).
    once: Option<(f32, f32)>,
    env: usize,
    shift: f32,
}

impl Default for Recipe {
    fn default() -> Self {
        Self { partials: Vec::new(), noise: (0.0, 0.01, 0.5), scoop: (0.0, 0.01), fade_in: 0.002, attack: 0.25, cycles: 1, once: None, env: 1, shift: 0.0 }
    }
}

/// Three copies of a partial, one a little flat and one a little sharp
/// (`cycles` long loops beat slowly).
fn ensemble(out: &mut Vec<(f32, f32, f32, f32)>, k: f32, level: f32, start: f32, tau: f32, cycles: usize) {
    let d = 1.0 / cycles as f32;
    for r in [1.0 - d, 1.0, 1.0 + d] {
        out.push((k * r, level * 0.5, start * 0.5, tau));
    }
}

fn recipe(sound: usize) -> Recipe {
    let mut r = Recipe::default();
    let p = &mut r.partials;
    match sound {
        // Piano: bright at the hammer, darker as it rings.
        0 => {
            for k in 1..=14 {
                let kf = k as f32;
                p.push((kf, 1.0 / kf.powf(2.0), 1.0 / kf.powf(0.9), 0.3 / kf.sqrt()));
            }
            r.noise = (0.25, 0.012, 0.9);
            r.attack = 0.4;
            r.env = 2;
        }
        // Trumpet: the upper partials bloom in, the pitch comes up from below.
        1 => {
            for k in 1..=12 {
                let kf = k as f32;
                let level = 1.0 / kf.powf(0.6);
                p.push((kf, level, if k < 3 { level * 0.4 } else { 0.0 }, 0.025 * kf.sqrt()));
            }
            r.scoop = (-0.03, 0.04);
            r.noise = (0.06, 0.03, 0.6);
            r.fade_in = 0.006;
            r.attack = 0.2;
        }
        // Pipe organ: an eight-foot, its octaves and a mixture, the chiff at the start.
        2 => {
            for (k, level) in [(1.0, 1.0), (2.0, 0.55), (3.0, 0.25), (4.0, 0.3), (6.0, 0.12), (8.0, 0.15)] {
                p.push((k, level, if k == 2.0 { 1.1 } else { level }, 0.03));
            }
            r.noise = (0.3, 0.04, 0.8);
            r.fade_in = 0.01;
            r.attack = 0.15;
        }
        // A voice singing "ah": a buzz through three formants (they move with the key, as on the toy).
        3 => {
            for k in 1..=17 {
                let f = k as f32 * ROOT_HZ;
                let formant = |c: f32, b: f32, g: f32| g / (1.0 + ((f - c) / (b * 0.5)).powi(2)).sqrt();
                let level = (formant(730.0, 90.0, 1.0) + formant(1090.0, 110.0, 0.5) + formant(2440.0, 160.0, 0.25)) / (k as f32).powf(0.8);
                p.push((k as f32, level, 0.0, 0.05));
            }
            r.noise = (0.05, 0.08, 0.7);
            r.fade_in = 0.06;
            r.attack = 0.2;
            r.env = 4;
        }
        // Flute: nearly a sine, overblown a moment at the start, breath.
        4 => {
            for (k, level, start) in [(1.0, 1.0, 0.8), (2.0, 0.3, 0.7), (3.0, 0.12, 0.2), (4.0, 0.05, 0.05), (5.0, 0.03, 0.03)] {
                p.push((k, level, start, 0.04));
            }
            r.scoop = (0.01, 0.03);
            r.noise = (0.25, 0.08, 0.5);
            r.fade_in = 0.03;
            r.attack = 0.2;
        }
        // Brass ensemble: three brass voices a little apart.
        5 => {
            r.cycles = 32;
            for k in 1..=10 {
                let kf = k as f32;
                ensemble(p, kf, 1.0 / kf.powf(0.8), if k < 2 { 0.3 } else { 0.0 }, 0.05 * kf.powf(0.3), 32);
            }
            r.fade_in = 0.02;
            r.attack = 0.25;
        }
        // Synth drum: a sine falling an octave, a little noise; it plays once.
        6 => {
            p.push((1.0, 1.0, 1.0, 1.0));
            p.push((2.0, 0.15, 0.15, 1.0));
            r.scoop = (1.0, 0.04);
            r.noise = (0.3, 0.02, 0.7);
            r.once = Some((0.6, 0.16));
            r.env = 0;
        }
        // Jazz organ: sixteen, eight, five and a third, four foot, the percussion on the third harmonic, the key click.
        7 => {
            r.cycles = 2;
            for (k, level) in [(0.5, 0.6), (1.0, 1.0), (1.5, 0.45), (2.0, 0.7), (4.0, 0.3), (8.0, 0.15)] {
                p.push((k, level, level, 1.0));
            }
            p.push((3.0, 0.0, 0.6, 0.12));
            r.noise = (0.4, 0.004, 1.0);
            r.attack = 0.4;
        }
        // Strings: saws a little apart, swelling in.
        8 => {
            r.cycles = 64;
            for k in 1..=16 {
                let kf = k as f32;
                ensemble(p, kf, 0.8 / kf, 0.0, 0.08 + 0.01 * kf, 64);
            }
            r.fade_in = 0.05;
            r.attack = 0.35;
            r.env = 4;
        }
        // Vibes: the bar's partials at 1 and 4, the mallet.
        9 => {
            p.push((1.0, 1.0, 1.0, 1.0));
            p.push((4.0, 0.0, 0.5, 0.15));
            p.push((10.0, 0.0, 0.15, 0.05));
            r.noise = (0.2, 0.004, 0.6);
            r.attack = 0.5;
            r.env = 5;
        }
        // Glockenspiel: bright, out-of-tune partials (rounded to the loop), two octaves up.
        10 => {
            r.cycles = 8;
            p.push((1.0, 1.0, 1.0, 1.0));
            p.push((2.71, 0.0, 0.6, 0.25));
            p.push((5.15, 0.0, 0.3, 0.1));
            r.noise = (0.15, 0.003, 1.0);
            r.attack = 0.5;
            r.env = 5;
            r.shift = 24.0;
        }
        // Synth bass: a saw through a filter that closes after the start.
        11 => {
            for k in 1..=17 {
                let kf = k as f32;
                p.push((kf, (1.0 / kf) / (1.0 + (kf / 3.0).powi(2)), (1.0 / kf) / (1.0 + (kf / 9.0).powi(2)), 0.08));
            }
            r.attack = 0.3;
        }
        // Marimba: the fundamental and the tuned fourth harmonic, gone quickly.
        12 => {
            p.push((1.0, 1.0, 1.0, 1.0));
            p.push((4.0, 0.0, 0.45, 0.06));
            p.push((10.0, 0.0, 0.12, 0.02));
            r.noise = (0.15, 0.003, 0.4);
            r.attack = 0.3;
            r.env = 3;
        }
        // Harpsichord: plucked near the end of the string, bright and thinning.
        13 => {
            for k in 1..=17 {
                let kf = k as f32;
                let start = 1.2 * (std::f32::consts::PI * kf * 0.12).sin().abs() / kf.powf(0.4);
                p.push((kf, start * 0.4 / kf.sqrt(), start, 0.4 / kf.powf(0.3)));
            }
            r.noise = (0.2, 0.005, 1.0);
            r.attack = 0.5;
            r.env = 2;
        }
        // Accordion: two reeds, one a little sharp (the musette's beat).
        14 => {
            r.cycles = 64;
            for k in 1..=15 {
                let kf = k as f32;
                let level = 1.0 / kf.powf(0.9);
                p.push((kf, level * 0.6, level * 0.6, 1.0));
                p.push((kf * (1.0 + 1.0 / 64.0), level * 0.5, level * 0.5, 1.0));
            }
            r.fade_in = 0.03;
            r.attack = 0.1;
        }
        // Clarinet: odd partials, the upper ones coming in after the tongue.
        _ => {
            for (k, level) in [(1.0, 1.0), (2.0, 0.05), (3.0, 0.6), (4.0, 0.04), (5.0, 0.35), (7.0, 0.2), (9.0, 0.12), (11.0, 0.06)] {
                p.push((k, level, if k < 2.0 { 0.7 } else { 0.0 }, 0.03));
            }
            r.fade_in = 0.01;
            r.attack = 0.2;
        }
    }
    r
}

/// To 8 bits, the loudest sample at 127.
fn to_bytes(x: &[f32]) -> Vec<i8> {
    let peak = x.iter().fold(0.0f32, |m, v| m.max(v.abs())).max(1e-6);
    x.iter().map(|v| (v / peak * 127.0).round().clamp(-127.0, 127.0) as i8).collect()
}

/// Makes a sound by its recipe, at the toy's rate.
fn make(sound: usize) -> Sound {
    let r = recipe(sound);
    let sr = SRC_RATE;
    let loop_len = ROOT_PERIOD * r.cycles;
    let (len, loop_start) = match r.once {
        Some((secs, _)) => ((secs * sr) as usize, None),
        None => {
            // The attack in whole loops, so the loop starts in step.
            let a = ((r.attack * sr) as usize).div_ceil(loop_len) * loop_len;
            (a + loop_len, Some(a))
        }
    };
    let held_from = loop_start.unwrap_or(usize::MAX);
    let cycles = r.cycles as f32;
    // Each ratio rounded so the loop holds whole cycles of it; none above the toy's Nyquist.
    let partials: Vec<(f32, f32, f32, f32)> = r.partials.iter().map(|&(k, l, s, t)| ((k * cycles).round() / cycles, l, s, t)).filter(|p| p.0 * ROOT_HZ < sr * 0.5).collect();
    let mut phase = vec![0.0f64; partials.len()];
    let mut rng = super::Rng::new(0x5EED_0000 + sound as u32);
    let (mut lp, mut prev) = (0.0f32, 0.0f32);
    let mut out = vec![0.0f32; len];
    for (i, y) in out.iter_mut().enumerate() {
        let t = i as f32 / sr;
        let held = i >= held_from;
        let glide = if held { 1.0 } else { 1.0 + r.scoop.0 * (-t / r.scoop.1.max(1e-4)).exp() };
        let mut s = 0.0f32;
        for (k, &(ratio, level, start, tau)) in partials.iter().enumerate() {
            let a = if held { level } else { level + (start - level) * (-t / tau.max(1e-4)).exp() };
            s += a * (phase[k] as f32 * TAU).sin();
            phase[k] = (phase[k] + (ratio * glide * ROOT_HZ / sr) as f64).fract();
        }
        // Noise at the start: through a one-pole low pass (darker) or the difference (brighter).
        if !held && r.noise.0 > 0.0 {
            let n = rng.bipolar();
            lp += (0.15 + 0.85 * (1.0 - r.noise.2)) * (n - lp);
            let colored = lp * (1.0 - r.noise.2) + (n - prev) * 0.5 * r.noise.2;
            prev = n;
            s += r.noise.0 * colored * (-t / r.noise.1).exp();
        }
        if let Some((_, tc)) = r.once {
            s *= (-t / tc).exp();
        }
        if t < r.fade_in {
            s *= t / r.fade_in;
        }
        *y = s;
    }
    Sound { data: to_bytes(&out), loop_start, env: r.env, shift: r.shift }
}

static SOUND_TABLES: OnceLock<Vec<Sound>> = OnceLock::new();
static DRUM_TABLES: OnceLock<Vec<Vec<i8>>> = OnceLock::new();

/// Makes the sounds and the drums (once; never on the audio thread, which
/// stays silent until they are there).
pub fn warm() {
    SOUND_TABLES.get_or_init(|| (0..SAMPLE_SLOT).map(make).collect());
    DRUM_TABLES.get_or_init(|| (0..DRUMS.len()).map(make_drum).collect());
}

/// A sound, if it has been made.
pub fn sound(i: usize) -> Option<&'static Sound> {
    SOUND_TABLES.get().and_then(|v| v.get(i))
}

/// What a sound's loop looks like (its start, for one that plays once): for the face.
pub fn wave(i: usize) -> Option<&'static [i8]> {
    let s = sound(i)?;
    let from = s.loop_start.unwrap_or(0);
    Some(&s.data[from..(from + ROOT_PERIOD * 2).min(s.data.len())])
}

// ------------------------------------------------------------ envelopes

#[derive(Clone, Copy, PartialEq, Debug)]
enum Stage {
    Attack,
    Body,
    Release,
}

/// An envelope: attack (s), whether the body decays and how fast (time
/// constant, s; 0: held), release (time constant, s).
fn envelope(kind: usize) -> (f32, f32, f32) {
    match kind {
        2 => (0.003, 0.9, 0.25),
        3 => (0.002, 0.18, 0.08),
        4 => (0.25, 0.0, 0.35),
        5 => (0.002, 1.6, 0.9),
        6 => (0.6, 0.0, 0.015),
        _ => (0.005, 0.0, 0.05),
    }
}

#[derive(Clone, Copy)]
struct Voice {
    note: u8,
    gate: bool,
    active: bool,
    sound: usize,
    env_kind: usize,
    /// It plays to its end whatever the key does.
    once: bool,
    pos: f64,
    level: f32,
    stage: Stage,
    vel: f32,
    /// Semitones still to glide (portamento).
    glide: f32,
    /// A sample's gain (it is brought up to full scale).
    gain: f32,
    age: u64,
}

impl Voice {
    const fn new() -> Self {
        Self { note: 0, gate: false, active: false, sound: 0, env_kind: 1, once: false, pos: 0.0, level: 0.0, stage: Stage::Attack, vel: 1.0, glide: 0.0, gain: 1.0, age: 0 }
    }
}

/// To `bits` bits (signed, full scale 1).
#[inline]
pub fn quantize(x: f32, bits: u32) -> f32 {
    let steps = ((1u32 << (bits.clamp(2, 16) - 1)) - 1) as f32;
    (x.clamp(-1.0, 1.0) * steps).round() / steps
}

/// The level out of the converter, before the volume.
const OUT: f32 = 0.6;

pub struct KeytoneState {
    sr: f32,
    voices: [Voice; VOICES],
    clock: u64,
    last_note: Option<u8>,
    /// How far towards the converter's next step, and the value it holds.
    dac_phase: f64,
    dac: f32,
    /// The speaker's low pass.
    lp: f32,
    vib: f32,
    peak: f32,
    pub pitch_bend: f32,
}

impl KeytoneState {
    pub fn new(sr: f32) -> Self {
        warm();
        Self { sr, voices: [Voice::new(); VOICES], clock: 0, last_note: None, dac_phase: 0.0, dac: 0.0, lp: 0.0, vib: 0.0, peak: 0.0, pitch_bend: 0.0 }
    }

    fn pick(p: &[f32], i: usize, n: usize) -> usize {
        (p.get(i).copied().unwrap_or(0.0).round().max(0.0) as usize).min(n - 1)
    }

    pub fn note_on(&mut self, p: &[f32], sample: Option<&SampleRef>, note: u8, vel: f32) {
        if p.len() < SPECS.len() {
            return;
        }
        self.clock += 1;
        let sound = Self::pick(p, SOUND, SOUNDS.len());
        let chosen = Self::pick(p, ENVELOPE, ENVELOPES.len());
        let (own_env, once) = match sound {
            SAMPLE_SLOT => (3, true),
            s => self::sound(s).map(|s| (s.env, s.loop_start.is_none())).unwrap_or((1, false)),
        };
        // The same key again takes its own voice; else a free one, else the oldest.
        let v = (0..VOICES)
            .find(|&i| self.voices[i].active && self.voices[i].note == note)
            .or_else(|| (0..VOICES).find(|&i| !self.voices[i].active))
            .unwrap_or_else(|| (0..VOICES).min_by_key(|&i| self.voices[i].age).unwrap_or(0));
        let gain = match (sound, sample) {
            (SAMPLE_SLOT, Some(s)) => 1.0 / s.peaks.iter().fold(0.0f32, |m, (lo, hi)| m.max(lo.abs()).max(hi.abs())).max(0.05),
            _ => 1.0,
        };
        let glide = match self.last_note {
            Some(last) if p[PORTAMENTO] >= 0.5 => last as f32 - note as f32,
            _ => 0.0,
        };
        self.voices[v] = Voice {
            note,
            gate: true,
            active: true,
            sound,
            env_kind: if chosen == 0 { own_env } else { chosen },
            once: once && chosen == 0,
            pos: 0.0,
            level: 0.0,
            stage: Stage::Attack,
            vel: if p[VELOCITY] >= 0.5 { 0.25 + 0.75 * vel.clamp(0.0, 1.0) } else { 1.0 },
            glide,
            gain,
            age: self.clock,
        };
        self.last_note = Some(note);
    }

    pub fn note_off(&mut self, _p: &[f32], note: u8) {
        for v in self.voices.iter_mut().filter(|v| v.active && v.gate && v.note == note) {
            v.gate = false;
            if !v.once {
                v.stage = Stage::Release;
            }
        }
    }

    pub fn all_off(&mut self) {
        for v in self.voices.iter_mut() {
            v.active = false;
            v.gate = false;
        }
    }

    /// For the face: voices sounding, the level, and the notes they play (-1: none).
    pub fn visual(&self, out: &mut [f32; crate::engine::INST_VALS]) {
        out[0] = self.voices.iter().filter(|v| v.active).count() as f32;
        out[1] = self.peak;
        for (k, v) in self.voices.iter().enumerate() {
            out[2 + k] = if v.active { v.note as f32 } else { -1.0 };
        }
    }

    pub fn render(&mut self, p: &[f32], sample: Option<&SampleRef>, l: &mut [f32], r: &mut [f32]) {
        let n = l.len().min(r.len());
        if p.len() < SPECS.len() || SOUND_TABLES.get().is_none() {
            return;
        }
        let rate = RATE_HZ[Self::pick(p, RATE, RATES.len())];
        let bits = BITS_N[Self::pick(p, BITS, BIT_DEPTHS.len())];
        let step = rate as f64 / self.sr as f64;
        let tone = p[TONE].clamp(0.0, 1.0);
        // The speaker: 400 Hz to open (at the top, nothing in the way).
        let lp_a = if tone >= 0.999 { 1.0 } else { 1.0 - (-TAU * 400.0 * 2f32.powf(tone * 6.0) / self.sr).exp() };
        let gain = OUT * p[VOLUME].clamp(0.0, 1.0);
        let mut peak = 0.0f32;
        for i in 0..n {
            self.dac_phase += step;
            while self.dac_phase >= 1.0 {
                self.dac_phase -= 1.0;
                self.dac = self.convert(p, sample, rate, bits);
            }
            self.lp += lp_a * (self.dac - self.lp);
            let y = self.lp * gain;
            peak = peak.max(y.abs());
            l[i] += y;
            r[i] += y;
        }
        self.peak = peak;
    }

    /// One step of the converter: every voice moved on a sample, summed, in `bits`.
    fn convert(&mut self, p: &[f32], sample: Option<&SampleRef>, rate: f32, bits: u32) -> f32 {
        let base = 12.0 * p[OCTAVE].round().clamp(-2.0, 2.0) + p[TUNE] / 100.0 + self.pitch_bend;
        let vib = if p[VIBRATO] >= 0.5 {
            self.vib = (self.vib + p[VIB_RATE].clamp(0.5, 20.0) / rate).fract();
            (self.vib * TAU).sin() * p[VIB_DEPTH].clamp(0.0, 1.0)
        } else {
            0.0
        };
        let glide_k = (-1.0 / (p[GLIDE].max(0.005) * 0.4 * rate)).exp();
        let sustain = p[SUSTAIN] >= 0.5;
        let mut sum = 0.0f32;
        for v in self.voices.iter_mut().filter(|v| v.active) {
            let semis = v.note as f32 + base + v.glide + vib;
            v.glide *= glide_k;
            let ratio = 2f32.powf((semis - 60.0) / 12.0);
            let x = if v.sound == SAMPLE_SLOT {
                // A sample of your own: decimated, no filter, in 8 bits.
                match sample.filter(|s| s.frames() > 0) {
                    Some(s) => {
                        let i = v.pos as usize;
                        if i >= s.frames() {
                            v.active = false;
                            continue;
                        }
                        v.pos += (ratio * s.sample_rate as f32 / rate) as f64;
                        quantize(0.5 * (s.left[i] + s.right[i]) * v.gain, 8)
                    }
                    None => {
                        v.active = false;
                        continue;
                    }
                }
            } else {
                let Some(snd) = self::sound(v.sound) else {
                    v.active = false;
                    continue;
                };
                // Middle C sounds where the key says (the sound's root is near it).
                let hz = 261.625_56 * ratio * 2f32.powf(snd.shift / 12.0);
                let i = v.pos as usize;
                if i >= snd.data.len() {
                    match snd.loop_start {
                        Some(ls) => {
                            let len = snd.data.len() - ls;
                            v.pos = ls as f64 + (v.pos - ls as f64) % len as f64;
                        }
                        None => {
                            v.active = false;
                            continue;
                        }
                    }
                }
                let x = snd.data[(v.pos as usize).min(snd.data.len() - 1)] as f32 / 127.0;
                v.pos += (hz / ROOT_HZ * SRC_RATE / rate) as f64;
                x
            };
            // The envelope, in 32 steps as the toy's.
            let (attack, decay, release) = envelope(v.env_kind);
            let release = if sustain { (release * 6.0).max(1.5) } else { release };
            match v.stage {
                Stage::Attack => {
                    v.level += 1.0 / (attack.max(0.001) * rate);
                    if v.level >= 1.0 {
                        v.level = 1.0;
                        v.stage = Stage::Body;
                    }
                }
                Stage::Body => {
                    if decay > 0.0 {
                        v.level *= (-1.0 / (decay * rate)).exp();
                    }
                }
                Stage::Release => v.level *= (-1.0 / (release * rate)).exp(),
            }
            // A reverse swell cuts off at its top.
            if v.env_kind == 6 && v.stage == Stage::Body && !v.gate {
                v.stage = Stage::Release;
            }
            if v.level < 1e-3 && v.stage != Stage::Attack {
                v.active = false;
                continue;
            }
            let env = (v.level * 32.0).ceil() / 32.0;
            sum += x * env * v.vel;
        }
        quantize(sum * 0.4, bits)
    }
}

// ------------------------------------------------------------ the drums

/// The drums of its rhythm section, in the order of `DrumVoice::KEYTONE`.
pub const DRUMS: &[&str] = &["Bass Drum", "Snare", "Rim", "Clap", "Hi-Hat", "Open Hat", "High Tom", "Low Tom", "Cymbal", "Claves", "Cowbell", "High Conga", "Low Conga", "Maracas"];

/// Makes drum `i` at the toy's rate, from sines, squares and noise.
fn make_drum(i: usize) -> Vec<i8> {
    let sr = SRC_RATE;
    let mut rng = super::Rng::new(0xD0D0_0000 + i as u32);
    let secs = [0.32, 0.25, 0.06, 0.3, 0.08, 0.45, 0.3, 0.35, 1.1, 0.08, 0.3, 0.2, 0.25, 0.08][i.min(13)];
    let n = (secs * sr) as usize;
    let mut out = vec![0.0f32; n];
    let (mut ph, mut ph2) = (0.0f32, 0.0f32);
    let mut metal = [0.0f32; 6];
    let mut prev = 0.0f32;
    let (mut lp, mut smooth) = (0.0f32, 0.0f32);
    for (j, y) in out.iter_mut().enumerate() {
        let t = j as f32 / sr;
        let noise = rng.bipolar();
        let bright = noise - prev;
        prev = noise;
        lp += 0.3 * (noise - lp);
        let sq = |p: f32| if p < 0.5 { 1.0 } else { -1.0 };
        let adv = |p: &mut f32, f: f32| {
            *p = (*p + f / sr).fract();
            *p
        };
        let mut metal_sum = || {
            let mut s = 0.0;
            for (k, f) in [370.0, 548.0, 663.0, 941.0, 972.0, 1440.0].into_iter().enumerate() {
                metal[k] = (metal[k] + f / sr).fract();
                s += sq(metal[k]);
            }
            s / 6.0
        };
        *y = match i {
            0 => {
                let f = 55.0 + 120.0 * (-t / 0.03).exp();
                let body = (adv(&mut ph, f) * TAU).sin() * (-t / 0.11).exp();
                body + if t < 0.004 { 0.5 * noise } else { 0.0 }
            }
            1 => {
                let tone = ((adv(&mut ph, 185.0) * TAU).sin() + 0.6 * (adv(&mut ph2, 330.0) * TAU).sin()) * (-t / 0.05).exp() * 0.6;
                tone + bright * 0.5 * (-t / 0.09).exp()
            }
            2 => (0.6 * (adv(&mut ph, 1700.0) * TAU).sin() + 0.4 * sq(adv(&mut ph2, 820.0))) * (-t / 0.012).exp() + noise * 0.3 * (-t / 0.004).exp(),
            3 => {
                // Three quick claps, then the room.
                let burst = [0.0, 0.011, 0.022].iter().map(|&s| if t >= s { (-(t - s) / 0.006).exp() } else { 0.0 }).sum::<f32>() + 0.5 * (-t / 0.08).exp();
                (bright * 0.6 + lp * 0.4) * burst
            }
            4 => (bright * 0.6 + metal_sum() * 0.4) * (-t / 0.022).exp(),
            5 => (bright * 0.6 + metal_sum() * 0.4) * (-t / 0.16).exp(),
            6 | 7 => {
                let (f0, df, tc) = if i == 6 { (190.0, 50.0, 0.09) } else { (120.0, 35.0, 0.11) };
                let f = f0 + df * (-t / 0.05).exp();
                (adv(&mut ph, f) * TAU).sin() * (-t / tc).exp() + noise * 0.15 * (-t / 0.01).exp()
            }
            8 => (bright * 0.5 + metal_sum() * 0.5) * (-t / 0.4).exp(),
            9 => (adv(&mut ph, 2500.0) * TAU).sin() * (-t / 0.018).exp(),
            10 => {
                let s = 0.5 * (sq(adv(&mut ph, 562.0)) + sq(adv(&mut ph2, 845.0)));
                smooth += 0.5 * (s - smooth);
                smooth * (0.4 * (-t / 0.012).exp() + 0.6 * (-t / 0.07).exp())
            }
            11 | 12 => {
                let (f0, tc) = if i == 11 { (360.0, 0.05) } else { (250.0, 0.07) };
                (adv(&mut ph, f0 + 40.0 * (-t / 0.01).exp()) * TAU).sin() * (-t / tc).exp()
            }
            _ => bright * (t / 0.005).min(1.0) * (-t / 0.02).exp(),
        };
    }
    to_bytes(&out)
}

/// A drum of the toy, played on a pad: its sample read at the converter's
/// rate (Rate: an octave down to an octave up from the toy's) in so many
/// bits (Bits: 4 to 12, 8 in the middle), a speaker's low pass (Tone).
#[derive(Clone, Copy, Default)]
pub struct KeytoneDrum {
    drum: usize,
    pos: f64,
    inc: f64,
    phase: f64,
    step: f64,
    bits: u32,
    held: f32,
    amp: f32,
    amp_k: f32,
    lp: f32,
    lp_a: f32,
    active: bool,
}

impl KeytoneDrum {
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// `tune` a pitch ratio, `decay` 1 as made (shorter below), `rate` and `bits` 0..1.
    pub fn trigger(&mut self, drum: usize, tune: f32, decay: f32, tone: f32, rate: f32, bits: f32, sr: f32) {
        let Some(data) = DRUM_TABLES.get().and_then(|d| d.get(drum)) else {
            self.active = false;
            return;
        };
        let dac = SRC_RATE * 2f32.powf(rate.clamp(0.0, 1.0) * 2.0 - 1.0);
        let len = data.len() as f32 / SRC_RATE;
        *self = Self {
            drum,
            pos: 0.0,
            inc: (tune * SRC_RATE / dac) as f64,
            phase: 1.0,
            step: (dac / sr) as f64,
            bits: 4 + (bits.clamp(0.0, 1.0) * 8.0).round() as u32,
            held: 0.0,
            amp: 1.0,
            amp_k: if decay < 1.0 { (-1.0 / (len * decay.max(0.05) * 0.35 * sr)).exp() } else { 1.0 },
            lp: 0.0,
            lp_a: 1.0 - (-TAU * 400.0 * 2f32.powf(tone.clamp(0.0, 1.0) * 6.0) / sr).exp(),
            active: true,
        };
    }

    #[inline]
    pub fn tick(&mut self) -> f32 {
        if !self.active {
            return 0.0;
        }
        let Some(data) = DRUM_TABLES.get().and_then(|d| d.get(self.drum)) else {
            self.active = false;
            return 0.0;
        };
        self.phase += self.step;
        while self.phase >= 1.0 {
            self.phase -= 1.0;
            let i = self.pos as usize;
            if i >= data.len() {
                self.active = false;
                return 0.0;
            }
            self.held = quantize(data[i] as f32 / 127.0 * self.amp, self.bits);
            self.pos += self.inc;
        }
        self.amp *= self.amp_k;
        self.lp += self.lp_a * (self.held - self.lp);
        self.lp
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn play(p: &[f32], notes: &[u8], secs: f32) -> (Vec<f32>, KeytoneState) {
        let sr = 48000.0;
        let mut s = KeytoneState::new(sr);
        for &n in notes {
            s.note_on(p, None, n, 0.8);
        }
        let mut out = Vec::new();
        let mut l = [0.0f32; 256];
        let mut r = [0.0f32; 256];
        while (out.len() as f32) < secs * sr {
            l.fill(0.0);
            r.fill(0.0);
            s.render(p, None, &mut l, &mut r);
            out.extend_from_slice(&l);
        }
        (out, s)
    }

    /// The pitch from the rising zero crossings over the second half.
    fn pitch(x: &[f32], sr: f32) -> f32 {
        let x = &x[x.len() / 2..];
        let ups: Vec<usize> = (1..x.len()).filter(|&i| x[i - 1] <= 0.0 && x[i] > 0.0).collect();
        (ups.len() - 1) as f32 / ((ups[ups.len() - 1] - ups[0]) as f32 / sr)
    }

    #[test]
    fn it_plays_at_the_right_pitch() {
        warm();
        // The flute, nearly a sine: A4 at 440 and A3 at 220.
        let mut p = preset("Flute");
        p[TONE] = 0.4;
        for (note, hz) in [(69u8, 440.0f32), (57, 220.0)] {
            let (out, _) = play(&p, &[note], 0.6);
            let f = pitch(&out, 48000.0);
            assert!((f / hz - 1.0).abs() < 0.02, "{note}: {f} Hz");
        }
    }

    #[test]
    fn its_output_is_eight_bits_held_at_the_toys_rate() {
        warm();
        let mut p = preset("Piano");
        p[TONE] = 1.0;
        let (out, _) = play(&p, &[60, 64, 67], 0.3);
        let scale = OUT * p[VOLUME];
        // Every value on the 8-bit grid.
        for &y in &out {
            let q = y / scale * 127.0;
            assert!((q - q.round()).abs() < 1e-3, "{q}");
        }
        // Held for about five host samples (48000 / 9375) at a time.
        let changes = out.windows(2).filter(|w| w[0] != w[1]).count();
        let per = out.len() as f32 / changes.max(1) as f32;
        assert!(per > 4.5, "{per} samples a step");
        assert!(out.iter().any(|y| y.abs() > 0.05));
    }

    #[test]
    fn eight_voices_and_the_ninth_takes_the_oldest() {
        warm();
        let p = preset("Pipe Organ");
        let (_, mut s) = play(&p, &[48, 50, 52, 53, 55, 57, 59, 60], 0.05);
        let mut vals = [0.0f32; crate::engine::INST_VALS];
        s.visual(&mut vals);
        assert_eq!(vals[0], 8.0);
        s.note_on(&p, None, 72, 1.0);
        s.visual(&mut vals);
        assert_eq!(vals[0], 8.0);
        assert!(vals[2..10].contains(&72.0) && !vals[2..10].contains(&48.0));
    }

    #[test]
    fn every_preset_and_sound_plays_and_stays_finite() {
        warm();
        for name in preset_names() {
            let p = preset(name);
            assert_eq!(p.len(), SPECS.len());
            let (out, _) = play(&p, &[60, 67], 0.25);
            assert!(out.iter().all(|y| y.is_finite()), "{name}");
            if *name != "Sample" {
                assert!(out.iter().any(|y| y.abs() > 0.01), "{name} is silent");
            }
        }
        for i in 0..SAMPLE_SLOT {
            let s = sound(i).unwrap();
            assert!(s.data.iter().any(|&b| b.unsigned_abs() > 100), "{}", SOUNDS[i]);
        }
    }

    #[test]
    fn a_sample_in_the_user_slot_is_crushed_to_eight_bits() {
        warm();
        let sr = 48000.0;
        let frames = 44100;
        let left: Vec<f32> = (0..frames).map(|i| 0.3 * (i as f32 * TAU * 440.0 / 44100.0).sin()).collect();
        let sample = SampleRef::new(crate::sample::SampleData::from_buffers("sine", None, 44100, left.clone(), left));
        let mut p = preset("Sample");
        p[TONE] = 1.0;
        let mut s = KeytoneState::new(sr);
        s.note_on(&p, Some(&sample), 60, 1.0);
        let mut l = vec![0.0f32; 4800];
        let mut r = vec![0.0f32; 4800];
        s.render(&p, Some(&sample), &mut l, &mut r);
        assert!(l.iter().any(|y| y.abs() > 0.05));
        let scale = OUT * p[VOLUME];
        for &y in &l {
            let q = y / scale * 127.0;
            assert!((q - q.round()).abs() < 1e-3);
        }
    }

    #[test]
    fn portamento_glides_and_the_one_shot_ends() {
        warm();
        let mut p = preset("Synth Drum");
        let (_, s) = play(&p, &[60], 1.0);
        assert!(s.voices.iter().all(|v| !v.active), "the synth drum plays once");
        p = preset("Glide Lead");
        let mut s = KeytoneState::new(48000.0);
        s.note_on(&p, None, 60, 1.0);
        s.note_on(&p, None, 72, 1.0);
        assert!(s.voices.iter().any(|v| v.active && v.note == 72 && v.glide == -12.0));
    }

    #[test]
    fn the_drums_play_and_end() {
        warm();
        let sr = 48000.0;
        for d in 0..DRUMS.len() {
            let mut v = KeytoneDrum::default();
            v.trigger(d, 1.0, 1.0, 0.5, 0.5, 0.5, sr);
            let mut peak = 0.0f32;
            let mut n = 0;
            while v.is_active() && n < 3 * 48000 {
                let y = v.tick();
                assert!(y.is_finite());
                peak = peak.max(y.abs());
                n += 1;
            }
            assert!(!v.is_active(), "{} ends", DRUMS[d]);
            assert!(peak > 0.2, "{}: {peak}", DRUMS[d]);
        }
    }
}

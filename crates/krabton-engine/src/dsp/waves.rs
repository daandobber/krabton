//! Waves: a wavetable synth.
//!
//! Two wavetable oscillators morph through banks of eight single-cycle
//! waves (Position), each with up to seven unison voices spread in stereo
//! and a warp mode (Sync, Bend, Mirror or Quantize); a sine sub oscillator
//! and noise; a state-variable filter with its own envelope; an amp
//! envelope; and an LFO that moves Position, pitch and cutoff.
//!
//! Every wave is turned once into band-limited tables, one per octave with
//! the harmonics that would pass Nyquist removed, and the oscillators
//! crossfade between the two tables that suit their pitch, so high notes
//! do not alias.
//!
//! Waveforms from the Adventure Kid Waveforms (AKWF) by Kristoffer
//! Ekstrand, CC0. waves_data.txt lists the source file of every wave.

use super::{db_to_gain, midi_to_freq, pan_gains, Adsr, AdsrCoefs, Rng, TAU};
use crate::params::{ParamSpec, Unit};
use std::f32::consts::PI;
use std::sync::OnceLock;

/// Samples per wave in the data.
pub const WAVE_LEN: usize = 600;
/// Waves per bank.
pub const BANK_SIZE: usize = 8;
/// Waves in the data.
pub const WAVE_COUNT: usize = 128;

/// 128 waves of 600 little-endian i16 samples, in bank order.
static DATA: &[u8] = include_bytes!("waves_data.bin");

const MAX_VOICES: usize = 8;
const MAX_UNISON: usize = 7;
/// Samples per control step (pitch, position, cutoff).
const CTRL: usize = 16;
/// Host samples rendered per inner chunk.
const CHUNK: usize = 256;
/// Overall output level before the Volume knob.
const MASTER: f32 = 0.3;

/// Highest harmonic kept from a 600-sample cycle.
const TOP: usize = 299;
/// Mip levels: level `m` keeps harmonics up to `TOP >> m`.
const LEVELS: usize = 9;
/// Table length per level: many samples per kept harmonic, so the images
/// that linear interpolation leaves stay far down.
const SIZES: [usize; LEVELS] = [4096, 4096, 2048, 1024, 512, 512, 512, 512, 512];

/// Where each level starts inside one wave's block of tables. Every table
/// has one extra sample (a copy of the first) for interpolation.
const fn level_offsets() -> [usize; LEVELS + 1] {
    let mut o = [0; LEVELS + 1];
    let mut i = 0;
    while i < LEVELS {
        o[i + 1] = o[i] + SIZES[i] + 1;
        i += 1;
    }
    o
}
const OFFSETS: [usize; LEVELS + 1] = level_offsets();
/// Samples per wave across all its levels.
const STRIDE: usize = OFFSETS[LEVELS];

// Oscillator 1; oscillator 2 has the same layout starting at BANK2.
pub const BANK1: usize = 0;
pub const POS1: usize = 1;
pub const WARP1: usize = 2;
pub const WARP_AMT1: usize = 3;
pub const OCT1: usize = 4;
pub const SEMI1: usize = 5;
pub const FINE1: usize = 6;
pub const LEVEL1: usize = 7;
pub const UNISON1: usize = 8;
pub const DETUNE1: usize = 9;
pub const SPREAD1: usize = 10;
pub const BANK2: usize = 11;
pub const POS2: usize = 12;
pub const WARP2: usize = 13;
pub const WARP_AMT2: usize = 14;
pub const OCT2: usize = 15;
pub const SEMI2: usize = 16;
pub const FINE2: usize = 17;
pub const LEVEL2: usize = 18;
pub const UNISON2: usize = 19;
pub const DETUNE2: usize = 20;
pub const SPREAD2: usize = 21;
pub const SUB: usize = 22;
pub const SUB_OCT: usize = 23;
pub const NOISE: usize = 24;
pub const FILTER: usize = 25;
pub const CUTOFF: usize = 26;
pub const RESONANCE: usize = 27;
pub const ENV_AMOUNT: usize = 28;
pub const KEY_TRACK: usize = 29;
pub const F_ATTACK: usize = 30;
pub const F_DECAY: usize = 31;
pub const F_SUSTAIN: usize = 32;
pub const F_RELEASE: usize = 33;
pub const ENV_POS: usize = 34;
pub const ATTACK: usize = 35;
pub const DECAY: usize = 36;
pub const SUSTAIN: usize = 37;
pub const RELEASE: usize = 38;
pub const VELOCITY: usize = 39;
pub const LFO_WAVE: usize = 40;
pub const LFO_RATE: usize = 41;
pub const LFO_RETRIG: usize = 42;
pub const LFO_POS: usize = 43;
pub const LFO_PITCH: usize = 44;
pub const LFO_CUTOFF: usize = 45;
pub const VOICES: usize = 46;
pub const GLIDE: usize = 47;
pub const VOLUME: usize = 48;

/// The 16 banks of 8 waves, in data order.
pub const BANKS: &[&str] = &[
    "Basic",
    "Bright Saws",
    "Vocal",
    "Strings",
    "Keys",
    "Organ",
    "FM",
    "Bells",
    "Chip",
    "Guitar",
    "Reed",
    "Flute",
    "Granular",
    "Hand Drawn",
    "Birds",
    "Raw",
];
pub const WARPS: &[&str] = &["Off", "Sync", "Bend", "Mirror", "Quantize"];
pub const SUB_OCTAVES: &[&str] = &["-1 Oct", "-2 Oct"];
pub const FILTERS: &[&str] = &["Low Pass", "Band Pass", "High Pass"];
pub const LFO_WAVES: &[&str] = &["Sine", "Triangle", "Saw", "Square", "S&H"];

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::choice("Bank 1", "Osc 1", BANKS, 0),
    ParamSpec::lin("Position 1", "Osc 1", 0.0, 1.0, 3.0 / 7.0, Unit::Percent),
    ParamSpec::choice("Warp 1", "Osc 1", WARPS, 0),
    ParamSpec::lin("Warp Amt 1", "Osc 1", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::int("Octave 1", "Osc 1", -3.0, 3.0, 0.0, Unit::None),
    ParamSpec::int("Semi 1", "Osc 1", -12.0, 12.0, 0.0, Unit::Semi),
    ParamSpec::lin("Fine 1", "Osc 1", -100.0, 100.0, 0.0, Unit::Cents),
    ParamSpec::lin("Level 1", "Osc 1", 0.0, 1.0, 0.8, Unit::Percent),
    ParamSpec::int("Unison 1", "Osc 1", 1.0, 7.0, 1.0, Unit::None),
    ParamSpec::lin("Detune 1", "Osc 1", 0.0, 50.0, 12.0, Unit::Cents),
    ParamSpec::lin("Spread 1", "Osc 1", 0.0, 1.0, 0.7, Unit::Percent),
    ParamSpec::choice("Bank 2", "Osc 2", BANKS, 0),
    ParamSpec::lin("Position 2", "Osc 2", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::choice("Warp 2", "Osc 2", WARPS, 0),
    ParamSpec::lin("Warp Amt 2", "Osc 2", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::int("Octave 2", "Osc 2", -3.0, 3.0, 0.0, Unit::None),
    ParamSpec::int("Semi 2", "Osc 2", -12.0, 12.0, 0.0, Unit::Semi),
    ParamSpec::lin("Fine 2", "Osc 2", -100.0, 100.0, 0.0, Unit::Cents),
    ParamSpec::lin("Level 2", "Osc 2", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::int("Unison 2", "Osc 2", 1.0, 7.0, 1.0, Unit::None),
    ParamSpec::lin("Detune 2", "Osc 2", 0.0, 50.0, 12.0, Unit::Cents),
    ParamSpec::lin("Spread 2", "Osc 2", 0.0, 1.0, 0.7, Unit::Percent),
    ParamSpec::lin("Sub", "Sub + Noise", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::choice("Sub Octave", "Sub + Noise", SUB_OCTAVES, 0),
    ParamSpec::lin("Noise", "Sub + Noise", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::choice("Filter", "Filter", FILTERS, 0),
    ParamSpec::log("Cutoff", "Filter", 20.0, 20000.0, 20000.0, Unit::Hz),
    ParamSpec::lin("Resonance", "Filter", 0.0, 1.0, 0.1, Unit::Percent),
    ParamSpec::lin("Env Amount", "Filter", -1.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Key Track", "Filter", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::log("Flt Attack", "Filter Env", 0.001, 10.0, 0.005, Unit::Sec),
    ParamSpec::log("Flt Decay", "Filter Env", 0.005, 10.0, 0.5, Unit::Sec),
    ParamSpec::lin("Flt Sustain", "Filter Env", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::log("Flt Release", "Filter Env", 0.005, 10.0, 0.5, Unit::Sec),
    ParamSpec::lin("Env Position", "Filter Env", -1.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::log("Attack", "Amp Env", 0.001, 10.0, 0.003, Unit::Sec),
    ParamSpec::log("Decay", "Amp Env", 0.005, 10.0, 0.6, Unit::Sec),
    ParamSpec::lin("Sustain", "Amp Env", 0.0, 1.0, 0.8, Unit::Percent),
    ParamSpec::log("Release", "Amp Env", 0.005, 10.0, 0.3, Unit::Sec),
    ParamSpec::lin("Velocity", "Amp Env", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::choice("LFO Wave", "LFO", LFO_WAVES, 0),
    ParamSpec::log("LFO Rate", "LFO", 0.05, 30.0, 2.0, Unit::Hz),
    ParamSpec::toggle("LFO Retrig", "LFO", false),
    ParamSpec::lin("LFO Position", "LFO", -1.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("LFO Pitch", "LFO", 0.0, 1200.0, 0.0, Unit::Cents),
    ParamSpec::lin("LFO Cutoff", "LFO", -1.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::int("Voices", "Voice", 1.0, 8.0, 8.0, Unit::None),
    ParamSpec::log("Glide", "Voice", 0.001, 2.0, 0.001, Unit::Sec),
    ParamSpec::lin("Volume", "Output", -24.0, 6.0, 0.0, Unit::Db),
];

pub fn preset_names() -> &'static [&'static str] {
    &[
        "Init",
        "Glass Pad",
        "Vocal Sweep",
        "Chip Lead",
        "Moving Strings",
        "Bell Keys",
        "Wobble Bass",
        "Organ Morph",
        "Birdsong",
        "Granular Haze",
        "Super Saw",
        "Sync Lead",
        "Breathy Flute",
        "Drawn Pluck",
    ]
}

/// Bank index by name, as a parameter value.
fn bank(name: &str) -> f32 {
    BANKS.iter().position(|&b| b == name).unwrap_or(0) as f32
}

pub fn preset(name: &str) -> Vec<f32> {
    let mut p = crate::params::defaults(SPECS);
    let mut set = |pairs: &[(usize, f32)]| {
        for &(i, v) in pairs {
            p[i] = v;
        }
    };
    let (sync, bend, mirror, quantize) = (1.0, 2.0, 3.0, 4.0);
    let (tri, sh) = (1.0, 4.0);
    let mono = (VOICES, 1.0);
    match name {
        "Glass Pad" => set(&[(BANK1, bank("Bells")), (POS1, 0.15), (LEVEL1, 0.6), (UNISON1, 3.0), (DETUNE1, 8.0), (BANK2, bank("FM")), (POS2, 0.25), (OCT2, 1.0), (LEVEL2, 0.3), (UNISON2, 2.0), (DETUNE2, 6.0), (LFO_WAVE, tri), (LFO_RATE, 0.15), (LFO_POS, 0.25), (CUTOFF, 7000.0), (RESONANCE, 0.15), (KEY_TRACK, 0.3), (ATTACK, 0.9), (DECAY, 2.0), (SUSTAIN, 0.85), (RELEASE, 2.5), (VELOCITY, 0.3)]),
        "Vocal Sweep" => set(&[(BANK1, bank("Vocal")), (POS1, 0.0), (LEVEL1, 0.7), (UNISON1, 3.0), (DETUNE1, 10.0), (SPREAD1, 0.6), (BANK2, bank("Vocal")), (POS2, 0.2), (OCT2, -1.0), (LEVEL2, 0.4), (ENV_POS, 0.9), (F_ATTACK, 1.5), (F_DECAY, 2.5), (F_SUSTAIN, 0.4), (F_RELEASE, 1.5), (LFO_RATE, 0.3), (LFO_POS, 0.1), (CUTOFF, 9000.0), (KEY_TRACK, 0.3), (ATTACK, 0.25), (DECAY, 1.0), (SUSTAIN, 0.9), (RELEASE, 1.2)]),
        "Chip Lead" => set(&[mono, (GLIDE, 0.04), (BANK1, bank("Chip")), (POS1, 0.35), (WARP1, quantize), (WARP_AMT1, 1.0), (LEVEL1, 0.7), (BANK2, bank("Chip")), (POS2, 0.6), (FINE2, 6.0), (LEVEL2, 0.25), (LFO_RATE, 5.5), (LFO_PITCH, 25.0), (DECAY, 0.3), (SUSTAIN, 0.8), (RELEASE, 0.08)]),
        "Moving Strings" => set(&[(BANK1, bank("Strings")), (POS1, 0.3), (LEVEL1, 0.6), (UNISON1, 5.0), (DETUNE1, 16.0), (SPREAD1, 0.9), (BANK2, bank("Strings")), (POS2, 0.6), (OCT2, 1.0), (LEVEL2, 0.3), (UNISON2, 3.0), (DETUNE2, 10.0), (SPREAD2, 0.8), (LFO_WAVE, tri), (LFO_RATE, 0.12), (LFO_POS, 0.3), (CUTOFF, 6000.0), (KEY_TRACK, 0.4), (ENV_AMOUNT, 0.1), (F_ATTACK, 0.6), (ATTACK, 0.45), (DECAY, 1.0), (SUSTAIN, 0.9), (RELEASE, 1.4), (VELOCITY, 0.4)]),
        "Bell Keys" => set(&[(BANK1, bank("Bells")), (POS1, 0.1), (LEVEL1, 0.6), (BANK2, bank("Keys")), (POS2, 0.3), (LEVEL2, 0.5), (ENV_POS, 0.6), (F_DECAY, 0.8), (F_SUSTAIN, 0.0), (F_RELEASE, 1.0), (CUTOFF, 4000.0), (ENV_AMOUNT, 0.25), (KEY_TRACK, 0.6), (DECAY, 2.5), (SUSTAIN, 0.0), (RELEASE, 1.2), (VELOCITY, 0.7)]),
        "Wobble Bass" => set(&[mono, (GLIDE, 0.03), (BANK1, bank("Bright Saws")), (POS1, 0.3), (OCT1, -1.0), (LEVEL1, 0.7), (BANK2, bank("Basic")), (POS2, 4.0 / 7.0), (OCT2, -1.0), (FINE2, 7.0), (LEVEL2, 0.5), (SUB, 0.6), (LFO_RATE, 3.0), (LFO_CUTOFF, 0.5), (LFO_POS, 0.35), (CUTOFF, 400.0), (RESONANCE, 0.45), (KEY_TRACK, 0.3), (ENV_AMOUNT, 0.1), (SUSTAIN, 1.0), (RELEASE, 0.1)]),
        "Organ Morph" => set(&[(BANK1, bank("Organ")), (POS1, 0.2), (LEVEL1, 0.6), (BANK2, bank("Organ")), (POS2, 0.5), (OCT2, 1.0), (LEVEL2, 0.3), (LFO_RATE, 0.25), (LFO_POS, 0.3), (CUTOFF, 12000.0), (KEY_TRACK, 0.0), (ATTACK, 0.005), (DECAY, 0.1), (SUSTAIN, 1.0), (RELEASE, 0.08), (VELOCITY, 0.0)]),
        "Birdsong" => set(&[(VOICES, 6.0), (BANK1, bank("Birds")), (POS1, 0.4), (WARP1, sync), (WARP_AMT1, 0.2), (LEVEL1, 0.6), (BANK2, bank("Birds")), (POS2, 0.8), (OCT2, 1.0), (LEVEL2, 0.3), (LFO_WAVE, sh), (LFO_RATE, 7.0), (LFO_POS, 0.45), (CUTOFF, 9000.0), (ATTACK, 0.01), (DECAY, 0.6), (SUSTAIN, 0.5), (RELEASE, 0.5)]),
        "Granular Haze" => set(&[(BANK1, bank("Granular")), (POS1, 0.2), (LEVEL1, 0.6), (UNISON1, 7.0), (DETUNE1, 25.0), (SPREAD1, 1.0), (BANK2, bank("Granular")), (POS2, 0.6), (OCT2, -1.0), (LEVEL2, 0.35), (UNISON2, 3.0), (DETUNE2, 12.0), (NOISE, 0.06), (LFO_WAVE, tri), (LFO_RATE, 0.08), (LFO_POS, 0.45), (CUTOFF, 3500.0), (RESONANCE, 0.25), (KEY_TRACK, 0.3), (ATTACK, 1.6), (DECAY, 3.0), (SUSTAIN, 0.8), (RELEASE, 3.0), (VELOCITY, 0.3)]),
        "Super Saw" => set(&[(BANK1, bank("Bright Saws")), (POS1, 0.4), (LEVEL1, 0.7), (UNISON1, 7.0), (DETUNE1, 28.0), (SPREAD1, 1.0), (BANK2, bank("Basic")), (POS2, 3.0 / 7.0), (OCT2, -1.0), (LEVEL2, 0.35), (UNISON2, 5.0), (DETUNE2, 18.0), (SPREAD2, 0.8), (CUTOFF, 9000.0), (KEY_TRACK, 0.3), (ENV_AMOUNT, 0.1), (ATTACK, 0.008), (DECAY, 1.0), (SUSTAIN, 0.85), (RELEASE, 0.5)]),
        "Sync Lead" => set(&[mono, (GLIDE, 0.05), (BANK1, bank("Raw")), (POS1, 0.3), (WARP1, sync), (WARP_AMT1, 0.45), (LEVEL1, 0.7), (BANK2, bank("Raw")), (POS2, 0.15), (OCT2, -1.0), (FINE2, 5.0), (LEVEL2, 0.4), (LFO_RATE, 5.0), (LFO_PITCH, 15.0), (CUTOFF, 7000.0), (RESONANCE, 0.25), (ENV_AMOUNT, 0.2), (SUSTAIN, 1.0), (RELEASE, 0.15)]),
        "Breathy Flute" => set(&[(BANK1, bank("Flute")), (POS1, 0.35), (LEVEL1, 0.7), (BANK2, bank("Reed")), (POS2, 0.2), (LEVEL2, 0.2), (NOISE, 0.08), (ENV_POS, 0.3), (F_ATTACK, 0.08), (F_DECAY, 0.4), (F_SUSTAIN, 0.2), (LFO_RATE, 5.0), (LFO_PITCH, 15.0), (LFO_POS, 0.1), (CUTOFF, 6000.0), (ATTACK, 0.06), (DECAY, 0.4), (SUSTAIN, 0.85), (RELEASE, 0.25), (VELOCITY, 0.6)]),
        "Drawn Pluck" => set(&[(BANK1, bank("Hand Drawn")), (POS1, 0.5), (WARP1, mirror), (WARP_AMT1, 0.6), (LEVEL1, 0.6), (BANK2, bank("Guitar")), (POS2, 0.4), (WARP2, bend), (WARP_AMT2, 0.3), (LEVEL2, 0.4), (CUTOFF, 2000.0), (RESONANCE, 0.3), (ENV_AMOUNT, 0.5), (F_DECAY, 0.25), (F_SUSTAIN, 0.0), (DECAY, 0.6), (SUSTAIN, 0.0), (RELEASE, 0.4), (VELOCITY, 0.6)]),
        _ => {}
    }
    p
}

/// The 128 waves as decoded from the data, 600 samples each.
fn raw_waves() -> &'static [f32] {
    static RAW: OnceLock<Vec<f32>> = OnceLock::new();
    RAW.get_or_init(|| DATA.chunks_exact(2).map(|b| i16::from_le_bytes([b[0], b[1]]) as f32 / 32767.0).collect())
}

/// The (band-unlimited) wave `index` (0..128) as 600 samples, for drawing it in the UI.
pub fn wave(index: usize) -> &'static [f32] {
    let i = index.min(WAVE_COUNT - 1);
    &raw_waves()[i * WAVE_LEN..(i + 1) * WAVE_LEN]
}

/// The band-limited tables of every wave, built once.
fn mip_tables() -> &'static [f32] {
    static MIPS: OnceLock<Vec<f32>> = OnceLock::new();
    MIPS.get_or_init(build_mips)
}

fn build_mips() -> Vec<f32> {
    let raw = raw_waves();
    let mut out = vec![0.0f32; WAVE_COUNT * STRIDE];
    let step = std::f64::consts::TAU / WAVE_LEN as f64;
    let cos: Vec<f64> = (0..WAVE_LEN).map(|i| (step * i as f64).cos()).collect();
    let sin: Vec<f64> = (0..WAVE_LEN).map(|i| (step * i as f64).sin()).collect();
    let twiddles = Twiddles::new(SIZES[0]);
    let mut a = vec![0.0f64; TOP + 1];
    let mut b = vec![0.0f64; TOP + 1];
    let mut re = vec![0.0f64; SIZES[0]];
    let mut im = vec![0.0f64; SIZES[0]];
    for w in 0..WAVE_COUNT {
        let x = &raw[w * WAVE_LEN..(w + 1) * WAVE_LEN];
        // Fourier series of the cycle: the sum of a[h] cos + b[h] sin.
        for h in 1..=TOP {
            let (mut sa, mut sb, mut k) = (0.0, 0.0, 0);
            for &v in x {
                sa += v as f64 * cos[k];
                sb += v as f64 * sin[k];
                k += h;
                if k >= WAVE_LEN {
                    k -= WAVE_LEN;
                }
            }
            a[h] = sa * 2.0 / WAVE_LEN as f64;
            b[h] = sb * 2.0 / WAVE_LEN as f64;
        }
        // Each level keeps half the harmonics of the one before.
        for (m, &n) in SIZES.iter().enumerate() {
            let (re, im) = (&mut re[..n], &mut im[..n]);
            re.fill(0.0);
            im.fill(0.0);
            for h in 1..=(TOP >> m) {
                re[h] = 0.5 * a[h];
                im[h] = -0.5 * b[h];
                re[n - h] = 0.5 * a[h];
                im[n - h] = 0.5 * b[h];
            }
            twiddles.inverse_fft(re, im);
            let table = &mut out[w * STRIDE + OFFSETS[m]..][..n + 1];
            for (t, &v) in table.iter_mut().zip(re.iter()) {
                *t = v as f32;
            }
            table[n] = table[0];
        }
    }
    out
}

/// Cos/sin table for radix-2 FFTs up to a given size.
struct Twiddles {
    n: usize,
    cos: Vec<f64>,
    sin: Vec<f64>,
}

impl Twiddles {
    fn new(n: usize) -> Self {
        let step = std::f64::consts::TAU / n as f64;
        Self {
            n,
            cos: (0..n / 2).map(|k| (step * k as f64).cos()).collect(),
            sin: (0..n / 2).map(|k| (step * k as f64).sin()).collect(),
        }
    }

    /// In-place unscaled inverse DFT (positive exponent) of a power-of-two
    /// length up to `self.n`.
    fn inverse_fft(&self, re: &mut [f64], im: &mut [f64]) {
        let n = re.len();
        let bits = n.trailing_zeros();
        for i in 0..n {
            let j = i.reverse_bits() >> (usize::BITS - bits);
            if j > i {
                re.swap(i, j);
                im.swap(i, j);
            }
        }
        let mut size = 2;
        while size <= n {
            let half = size / 2;
            let stride = self.n / size;
            for start in (0..n).step_by(size) {
                for k in 0..half {
                    let (wr, wi) = (self.cos[k * stride], self.sin[k * stride]);
                    let (a, b) = (start + k, start + k + half);
                    let tr = re[b] * wr - im[b] * wi;
                    let ti = re[b] * wi + im[b] * wr;
                    re[b] = re[a] - tr;
                    im[b] = im[a] - ti;
                    re[a] += tr;
                    im[a] += ti;
                }
            }
            size *= 2;
        }
    }
}

/// What to read for one stretch of the cycle: two neighbouring waves (the
/// Position morph), each at the two mip levels around the pitch.
#[derive(Clone, Copy)]
struct Seg {
    /// Table starts: wave A rich, wave B rich, wave A poor, wave B poor.
    off: [usize; 4],
    /// Rich and poor table lengths.
    size: [f32; 2],
    last: [usize; 2],
    /// Weights in the same order as `off`.
    w: [f32; 4],
}

impl Seg {
    /// `fa` is the first of the two waves, `mix` the share of the second,
    /// `f_read` how fast (Hz) the cycle is read and `band` the highest
    /// harmonic frequency a table should hold at full weight.
    fn new(fa: usize, mix: f32, f_read: f32, band: f32, sr: f32) -> Self {
        let x = (TOP as f32 * f_read / band).max(1.0).log2();
        let rich = (x as usize).min(LEVELS - 1);
        let poor = (rich + 1).min(LEVELS - 1);
        let t = if poor > rich { (x - rich as f32).clamp(0.0, 1.0) } else { 0.0 };
        // A cycle read faster than Nyquist averages out to nothing.
        let fade = ((0.49 * sr - f_read) / (0.04 * sr)).clamp(0.0, 1.0);
        let (a, b) = (fa * STRIDE, (fa + 1) * STRIDE);
        let (ma, mb) = ((1.0 - mix) * fade, mix * fade);
        Self {
            off: [a + OFFSETS[rich], b + OFFSETS[rich], a + OFFSETS[poor], b + OFFSETS[poor]],
            size: [SIZES[rich] as f32, SIZES[poor] as f32],
            last: [SIZES[rich] - 1, SIZES[poor] - 1],
            w: [ma * (1.0 - t), mb * (1.0 - t), ma * t, mb * t],
        }
    }

    /// The morphed, band-limited wave at phase `ph` (0..=1).
    #[inline]
    fn read(&self, t: &[f32], ph: f32) -> f32 {
        let x = ph * self.size[0];
        let i = (x as usize).min(self.last[0]);
        let f = x - i as f32;
        let (a, b) = (self.off[0] + i, self.off[1] + i);
        let rich = self.w[0] * (t[a] + (t[a + 1] - t[a]) * f) + self.w[1] * (t[b] + (t[b + 1] - t[b]) * f);
        let x = ph * self.size[1];
        let i = (x as usize).min(self.last[1]);
        let f = x - i as f32;
        let (a, b) = (self.off[2] + i, self.off[3] + i);
        let poor = self.w[2] * (t[a] + (t[a + 1] - t[a]) * f) + self.w[3] * (t[b] + (t[b + 1] - t[b]) * f);
        rich + poor
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Warp {
    Off,
    /// Reads the wave faster than the cycle and restarts it, like hard
    /// sync, with a window that hides the restart.
    Sync,
    /// Squeezes the first half of the wave into less of the cycle.
    Bend,
    /// Plays the wave forward, then backward.
    Mirror,
    /// Snaps Position to whole waves.
    Quantize,
}

impl Warp {
    fn from_param(v: f32) -> Self {
        match v.round() as i32 {
            1 => Warp::Sync,
            2 => Warp::Bend,
            3 => Warp::Mirror,
            4 => Warp::Quantize,
            _ => Warp::Off,
        }
    }
}

/// Window for Sync: 1 in the middle of the cycle, falling to `1 - depth`
/// at the restart.
#[inline]
fn sync_window(q: f32, depth: f32) -> f32 {
    let u = 2.0 * q - 1.0;
    let g = 1.0 - u * u;
    1.0 - depth + depth * g * g
}

/// One oscillator's settings for a render call.
#[derive(Clone, Copy)]
struct OscCfg {
    on: bool,
    bank: usize,
    pos: f32,
    warp: Warp,
    amount: f32,
    /// Octave, semitones and fine tune, in semitones.
    transpose: f32,
    unison: usize,
    /// Frequency ratio of each unison voice and of the highest one.
    ratio: [f32; MAX_UNISON],
    max_ratio: f32,
    /// Left and right gains of each unison voice.
    gl: [f32; MAX_UNISON],
    gr: [f32; MAX_UNISON],
}

impl OscCfg {
    /// `base` is BANK1 or BANK2.
    fn new(p: &[f32], base: usize) -> Self {
        let level = p[base + LEVEL1].clamp(0.0, 1.0);
        let unison = (p[base + UNISON1].round().max(1.0) as usize).min(MAX_UNISON);
        let detune = p[base + DETUNE1].clamp(0.0, 100.0);
        let spread = p[base + SPREAD1].clamp(0.0, 1.0);
        let gain = level / (unison as f32).sqrt();
        let mut cfg = Self {
            on: level > 0.0,
            bank: (p[base + BANK1].round().max(0.0) as usize).min(BANKS.len() - 1),
            pos: p[base + POS1].clamp(0.0, 1.0),
            warp: Warp::from_param(p[base + WARP1]),
            amount: p[base + WARP_AMT1].clamp(0.0, 1.0),
            transpose: 12.0 * p[base + OCT1].round() + p[base + SEMI1].round() + p[base + FINE1] / 100.0,
            unison,
            ratio: [1.0; MAX_UNISON],
            max_ratio: 1.0,
            gl: [0.0; MAX_UNISON],
            gr: [0.0; MAX_UNISON],
        };
        for u in 0..unison {
            // Spread evenly from -1 to 1.
            let x = if unison > 1 { u as f32 / (unison - 1) as f32 * 2.0 - 1.0 } else { 0.0 };
            cfg.ratio[u] = 2f32.powf(x * detune / 1200.0);
            cfg.max_ratio = cfg.max_ratio.max(cfg.ratio[u]);
            let (l, r) = pan_gains(x * spread);
            cfg.gl[u] = l * gain;
            cfg.gr[u] = r * gain;
        }
        cfg
    }
}

/// Per-render settings, from the parameters.
struct Ctrl {
    sr: f32,
    /// Highest harmonic frequency the tables hold at full weight.
    band: f32,
    osc: [OscCfg; 2],
    sub: f32,
    sub_drop: f32,
    noise: f32,
    filter: usize,
    cutoff_note: f32,
    k: f32,
    env_amount: f32,
    key_track: f32,
    env_pos: f32,
    fenv: AdsrCoefs,
    aenv: AdsrCoefs,
    velocity: f32,
    lfo_wave: usize,
    lfo_rate: f32,
    lfo_retrig: bool,
    lfo_pos: f32,
    lfo_pitch: f32,
    lfo_cutoff: f32,
    glide: bool,
    glide_k: f32,
    pos_k: f32,
    master: f32,
}

impl Ctrl {
    fn new(p: &[f32], sr: f32) -> Self {
        let block = CTRL as f32 / sr;
        Self {
            sr,
            // Harmonics are kept up to here at full weight and fade out by
            // twice this, so what folds back lands above 18 kHz.
            band: (0.5 * (sr - 18000.0)).min(20000.0).max(0.25 * sr),
            osc: [OscCfg::new(p, BANK1), OscCfg::new(p, BANK2)],
            sub: p[SUB].clamp(0.0, 1.0),
            sub_drop: if p[SUB_OCT] >= 0.5 { 24.0 } else { 12.0 },
            noise: p[NOISE].clamp(0.0, 1.0) * 0.5,
            filter: (p[FILTER].round().max(0.0) as usize).min(FILTERS.len() - 1),
            cutoff_note: 69.0 + 12.0 * (p[CUTOFF].max(10.0) / 440.0).log2(),
            k: 2.0 - 1.9 * p[RESONANCE].clamp(0.0, 1.0),
            env_amount: p[ENV_AMOUNT].clamp(-1.0, 1.0) * 60.0,
            key_track: p[KEY_TRACK].clamp(0.0, 1.0),
            env_pos: p[ENV_POS].clamp(-1.0, 1.0),
            fenv: AdsrCoefs::new(p[F_ATTACK], p[F_DECAY], p[F_SUSTAIN], p[F_RELEASE], sr),
            aenv: AdsrCoefs::new(p[ATTACK], p[DECAY], p[SUSTAIN], p[RELEASE], sr),
            velocity: p[VELOCITY].clamp(0.0, 1.0),
            lfo_wave: (p[LFO_WAVE].round().max(0.0) as usize).min(LFO_WAVES.len() - 1),
            lfo_rate: p[LFO_RATE].clamp(0.01, 50.0),
            lfo_retrig: p[LFO_RETRIG] >= 0.5,
            lfo_pos: p[LFO_POS].clamp(-1.0, 1.0),
            lfo_pitch: p[LFO_PITCH].max(0.0) / 100.0,
            lfo_cutoff: p[LFO_CUTOFF].clamp(-1.0, 1.0) * 48.0,
            glide: p[GLIDE] > 0.0015,
            glide_k: 1.0 - (-block / p[GLIDE].max(1e-4)).exp(),
            pos_k: 1.0 - (-block / 0.005).exp(),
            master: db_to_gain(p[VOLUME]) * MASTER,
        }
    }
}

/// LFO value at phase `ph`; `hold` is the current S&H value.
#[inline]
fn lfo_shape(wave: usize, ph: f32, hold: f32) -> f32 {
    match wave {
        1 => {
            let q = ph + 0.25;
            1.0 - 4.0 * ((q - q.floor()) - 0.5).abs()
        }
        2 => 1.0 - 2.0 * ph,
        3 => {
            if ph < 0.5 {
                1.0
            } else {
                -1.0
            }
        }
        4 => hold,
        _ => (TAU * ph).sin(),
    }
}

/// Moves an LFO phase on; true when it wrapped.
#[inline]
fn lfo_advance(phase: &mut f32, inc: f32) -> bool {
    *phase += inc;
    if *phase >= 1.0 {
        *phase -= phase.floor();
        true
    } else {
        false
    }
}

#[derive(Clone, Copy)]
struct OscState {
    phase: [f32; MAX_UNISON],
    /// Smoothed position in waves (0..7).
    pos: f32,
}

#[derive(Clone)]
struct Voice {
    note: u8,
    gate: bool,
    active: bool,
    /// Just started from silence: skip smoothing on the first step.
    fresh: bool,
    age: u64,
    vel: f32,
    pitch: f32,
    target: f32,
    osc: [OscState; 2],
    sub_phase: f32,
    /// Filter coefficient and the two filter states per channel.
    g: f32,
    ic1: [f32; 2],
    ic2: [f32; 2],
    fenv: Adsr,
    aenv: Adsr,
    lfo_phase: f32,
    lfo_hold: f32,
    rng: Rng,
}

impl Voice {
    fn new(index: usize) -> Self {
        Self {
            note: 0,
            gate: false,
            active: false,
            fresh: false,
            age: 0,
            vel: 0.0,
            pitch: 60.0,
            target: 60.0,
            osc: [OscState { phase: [0.0; MAX_UNISON], pos: 0.0 }; 2],
            sub_phase: 0.0,
            g: 0.1,
            ic1: [0.0; 2],
            ic2: [0.0; 2],
            fenv: Adsr::new(),
            aenv: Adsr::new(),
            lfo_phase: 0.0,
            lfo_hold: 0.0,
            rng: Rng::new(0x3A7E_51C9 ^ (index as u32 + 1).wrapping_mul(0x9E37_79B9)),
        }
    }

    /// `from` is the pitch to glide from, if any.
    fn start(&mut self, note: u8, vel: f32, from: Option<f32>, age: u64, retrig: bool) {
        if !self.active {
            // From silence: the first unison voice starts at the top of the
            // cycle, the others anywhere so they don't comb.
            for o in self.osc.iter_mut() {
                o.phase[0] = 0.0;
                for ph in o.phase[1..].iter_mut() {
                    *ph = self.rng.unipolar();
                }
            }
            self.sub_phase = 0.0;
            self.ic1 = [0.0; 2];
            self.ic2 = [0.0; 2];
            self.fresh = true;
        }
        self.pitch = from.unwrap_or(note as f32);
        self.note = note;
        self.target = note as f32;
        self.gate = true;
        self.active = true;
        self.vel = vel;
        self.age = age;
        if retrig {
            self.lfo_phase = 0.0;
            self.lfo_hold = self.rng.bipolar();
        }
        // Envelopes restart from their current level, so no clicks.
        self.fenv.trigger();
        self.aenv.trigger();
    }

    fn release(&mut self) {
        self.gate = false;
        self.fenv.release();
        self.aenv.release();
    }
}

pub struct WavesState {
    voices: Vec<Voice>,
    held: Vec<u8>,
    last_note: Option<f32>,
    counter: u64,
    sr: f32,
    lfo_phase: f32,
    lfo_hold: f32,
    lfo_rng: Rng,
}

impl WavesState {
    pub fn new(sr: f32) -> Self {
        // Build the tables now rather than on the first note.
        let _ = mip_tables();
        Self {
            voices: (0..MAX_VOICES).map(Voice::new).collect(),
            held: Vec::with_capacity(32),
            last_note: None,
            counter: 0,
            sr: sr.max(1000.0),
            lfo_phase: 0.0,
            lfo_hold: 0.0,
            lfo_rng: Rng::new(0x57A7_E5ED),
        }
    }

    /// What the display shows, for the newest voice: [0] voices sounding,
    /// [1] and [2] where each oscillator is in its bank (0 to 7), [3] the
    /// amp envelope, [4] the filter envelope, [5] the cutoff (Hz), [6] the
    /// pitch.
    pub fn visual(&self, out: &mut [f32; crate::engine::INST_VALS]) {
        out[0] = self.voices.iter().filter(|v| v.active).count() as f32;
        if let Some(v) = self.voices.iter().filter(|v| v.active).max_by_key(|v| v.age) {
            out[1] = v.osc[0].pos;
            out[2] = v.osc[1].pos;
            out[3] = v.aenv.level;
            out[4] = v.fenv.level;
            out[5] = v.g.atan() * self.sr / std::f32::consts::PI;
            out[6] = v.pitch;
        }
    }

    pub fn note_on(&mut self, p: &[f32], note: u8, vel: f32) {
        if p.len() < SPECS.len() {
            return;
        }
        let poly = (p[VOICES].round().max(1.0) as usize).min(MAX_VOICES);
        let glide = p[GLIDE] > 0.0015;
        let retrig = p[LFO_RETRIG] >= 0.5;
        let vel = vel.clamp(0.0, 1.0);
        self.counter += 1;
        let age = self.counter;
        let from = if glide { self.last_note } else { None };
        self.last_note = Some(note as f32);
        if poly == 1 {
            // Mono: legato while a key is held, last note priority.
            let v = &mut self.voices[0];
            let legato = !self.held.is_empty() && v.gate;
            self.held.retain(|&h| h != note);
            if self.held.len() < self.held.capacity() {
                self.held.push(note);
            }
            if legato {
                v.note = note;
                v.target = note as f32;
                if !glide {
                    v.pitch = note as f32;
                }
            } else {
                let from = if glide && v.active { Some(v.pitch) } else { from };
                v.start(note, vel, from, age, retrig);
            }
            return;
        }
        let pool = &mut self.voices[..poly];
        // A key that is still held restarts its own voice.
        if let Some(v) = pool.iter_mut().find(|v| v.gate && v.note == note) {
            v.start(note, vel, from, age, retrig);
            return;
        }
        // A free voice, else the oldest released one, else the oldest.
        let pick = pool
            .iter()
            .position(|v| !v.active)
            .or_else(|| pool.iter().enumerate().filter(|(_, v)| !v.gate).min_by_key(|(_, v)| v.age).map(|(i, _)| i))
            .or_else(|| pool.iter().enumerate().min_by_key(|(_, v)| v.age).map(|(i, _)| i))
            .unwrap_or(0);
        pool[pick].start(note, vel, from, age, retrig);
    }

    pub fn note_off(&mut self, p: &[f32], note: u8) {
        if p.len() < SPECS.len() {
            return;
        }
        let poly = (p[VOICES].round().max(1.0) as usize).min(MAX_VOICES);
        if poly == 1 {
            self.held.retain(|&h| h != note);
            let glide = p[GLIDE] > 0.0015;
            let back = self.held.last().copied();
            let v = &mut self.voices[0];
            match back {
                // Fall back to the previous held key.
                Some(b) if v.gate => {
                    v.note = b;
                    v.target = b as f32;
                    if !glide {
                        v.pitch = b as f32;
                    }
                }
                _ => {
                    if v.note == note || self.held.is_empty() {
                        v.release();
                    }
                }
            }
            // Voices left over from poly playing.
            for v in self.voices[1..].iter_mut() {
                if v.gate && v.note == note {
                    v.release();
                }
            }
            return;
        }
        for v in self.voices.iter_mut() {
            if v.gate && v.note == note {
                v.release();
            }
        }
    }

    pub fn all_off(&mut self) {
        self.held.clear();
        for v in self.voices.iter_mut() {
            v.release();
        }
    }

    pub fn active_voices(&self) -> usize {
        self.voices.iter().filter(|v| v.active).count()
    }

    pub fn render(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32]) {
        if p.len() < SPECS.len() {
            return;
        }
        let n = l.len().min(r.len());
        let c = Ctrl::new(p, self.sr);
        let tables = mip_tables();
        let mut done = 0;
        while done < n {
            let len = CHUNK.min(n - done);
            // The shared LFO, one value per control step.
            let steps = len.div_ceil(CTRL);
            let mut lfo = [0.0f32; CHUNK / CTRL];
            for (s, v) in lfo.iter_mut().take(steps).enumerate() {
                *v = lfo_shape(c.lfo_wave, self.lfo_phase, self.lfo_hold);
                let step = CTRL.min(len - s * CTRL);
                if lfo_advance(&mut self.lfo_phase, c.lfo_rate * step as f32 / c.sr) {
                    self.lfo_hold = self.lfo_rng.bipolar();
                }
            }
            let (out_l, out_r) = (&mut l[done..done + len], &mut r[done..done + len]);
            for v in self.voices.iter_mut().filter(|v| v.active) {
                render_voice(v, &c, tables, &lfo[..steps], out_l, out_r);
            }
            done += len;
        }
    }
}

/// Adds one unison stack of an oscillator into `l` and `r`; `read` maps
/// the phase (0..1) to a sample.
#[inline(always)]
fn run_unison(st: &mut OscState, oc: &OscCfg, f0: f32, sr: f32, l: &mut [f32], r: &mut [f32], read: impl Fn(f32) -> f32) {
    for u in 0..oc.unison {
        let dt = (f0 * oc.ratio[u] / sr).min(0.5);
        let (gl, gr) = (oc.gl[u], oc.gr[u]);
        let mut ph = st.phase[u];
        for (a, b) in l.iter_mut().zip(r.iter_mut()) {
            ph += dt;
            if ph >= 1.0 {
                ph -= 1.0;
            }
            let s = read(ph);
            *a += s * gl;
            *b += s * gr;
        }
        st.phase[u] = ph;
    }
}

/// Renders one control step of one oscillator into `l` and `r`.
#[allow(clippy::too_many_arguments)]
fn render_osc(st: &mut OscState, oc: &OscCfg, c: &Ctrl, t: &[f32], note: f32, pos_mod: f32, fresh: bool, l: &mut [f32], r: &mut [f32]) {
    // Position in waves, smoothed so knob moves and steps don't click.
    let mut x = (oc.pos + pos_mod).clamp(0.0, 1.0) * (BANK_SIZE - 1) as f32;
    if oc.warp == Warp::Quantize {
        let i = x.floor();
        let f = ((x - i - 0.5) / (1.0 - oc.amount).max(1e-3) + 0.5).clamp(0.0, 1.0);
        x = i + f;
    }
    if fresh {
        st.pos = x;
    } else {
        st.pos += (x - st.pos) * c.pos_k;
    }
    let i = (st.pos.max(0.0) as usize).min(BANK_SIZE - 2);
    let mix = (st.pos - i as f32).clamp(0.0, 1.0);
    let fa = oc.bank * BANK_SIZE + i;
    let f0 = midi_to_freq(note + oc.transpose);
    let ftop = f0 * oc.max_ratio;
    // How fast the fastest part of the cycle may be read.
    let max_slope = (0.4 * c.sr / ftop).max(1.0);
    let seg = |slope: f32| Seg::new(fa, mix, ftop * slope, c.band, c.sr);
    let a = oc.amount;
    match oc.warp {
        Warp::Off | Warp::Quantize => {
            let s = seg(1.0);
            run_unison(st, oc, f0, c.sr, l, r, |q| s.read(t, q));
        }
        Warp::Sync => {
            let ratio = (1.0 + 7.0 * a).min(max_slope);
            let depth = (4.0 * a).min(1.0);
            let s = seg(ratio);
            run_unison(st, oc, f0, c.sr, l, r, |q| {
                let y = q * ratio;
                s.read(t, y - y.floor()) * sync_window(q, depth)
            });
        }
        Warp::Bend => {
            // The first half of the wave takes `cp` of the cycle.
            let cp = (0.5 - 0.45 * a).max(0.5 / max_slope);
            let (k0, k1) = (0.5 / cp, 0.5 / (1.0 - cp));
            let (s0, s1) = (seg(k0), seg(k1));
            run_unison(st, oc, f0, c.sr, l, r, |q| if q < cp { s0.read(t, q * k0) } else { s1.read(t, 0.5 + (q - cp) * k1) });
        }
        Warp::Mirror => {
            // Forward until `cp`, then backward to the start.
            let cp = 1.0 - 0.5 * a;
            let (k0, k1) = (1.0 / cp, 1.0 / (1.0 - cp).max(1e-4));
            let (s0, s1) = (seg(k0), seg(k1));
            run_unison(st, oc, f0, c.sr, l, r, |q| if q < cp { s0.read(t, q * k0) } else { s1.read(t, 1.0 - (q - cp) * k1) });
        }
    }
}

fn render_voice(v: &mut Voice, c: &Ctrl, t: &[f32], lfo: &[f32], out_l: &mut [f32], out_r: &mut [f32]) {
    let n = out_l.len();
    let gain = (1.0 - c.velocity + c.velocity * v.vel) * c.master;
    let k = c.k;
    let mut i = 0;
    let mut step = 0;
    while i < n && v.active {
        let len = CTRL.min(n - i);
        // --- Control rate: glide, LFO, pitch, position and cutoff.
        if c.glide {
            v.pitch += (v.target - v.pitch) * c.glide_k;
        } else {
            v.pitch = v.target;
        }
        let m = if c.lfo_retrig {
            let value = lfo_shape(c.lfo_wave, v.lfo_phase, v.lfo_hold);
            if lfo_advance(&mut v.lfo_phase, c.lfo_rate * len as f32 / c.sr) {
                v.lfo_hold = v.rng.bipolar();
            }
            value
        } else {
            lfo.get(step).copied().unwrap_or(0.0)
        };
        step += 1;
        let fenv = v.fenv.level;
        let note = v.pitch + m * c.lfo_pitch;
        let pos_mod = m * c.lfo_pos + fenv * c.env_pos;

        // --- Oscillators, sub and noise.
        let mut bl = [0.0f32; CTRL];
        let mut br = [0.0f32; CTRL];
        for (st, oc) in v.osc.iter_mut().zip(c.osc.iter()) {
            if oc.on {
                render_osc(st, oc, c, t, note, pos_mod, v.fresh, &mut bl[..len], &mut br[..len]);
            }
        }
        if c.sub > 0.0 || c.noise > 0.0 {
            let dt = (midi_to_freq(note - c.sub_drop) / c.sr).min(0.5);
            for (a, b) in bl[..len].iter_mut().zip(br[..len].iter_mut()) {
                v.sub_phase += dt;
                if v.sub_phase >= 1.0 {
                    v.sub_phase -= 1.0;
                }
                let s = c.sub * (TAU * v.sub_phase).sin() + c.noise * v.rng.bipolar();
                *a += s;
                *b += s;
            }
        }

        // --- Filter, stereo, with the cutoff ramped across the step.
        let cut = c.cutoff_note + c.env_amount * fenv + c.key_track * (v.pitch - 60.0) + c.lfo_cutoff * m;
        let g_target = (PI * midi_to_freq(cut).clamp(20.0, 0.45 * c.sr) / c.sr).tan();
        if v.fresh {
            v.g = g_target;
            v.fresh = false;
        }
        let g_step = (g_target - v.g) / len as f32;
        for j in 0..len {
            v.g += g_step;
            v.fenv.next(&c.fenv);
            let amp = v.aenv.next(&c.aenv) * gain;
            let g = v.g;
            let a1 = 1.0 / (1.0 + g * (g + k));
            let a2 = g * a1;
            let a3 = g * a2;
            let mut y = [0.0f32; 2];
            for (ch, x) in [bl[j], br[j]].into_iter().enumerate() {
                let v3 = x - v.ic2[ch];
                let v1 = a1 * v.ic1[ch] + a2 * v3;
                let v2 = v.ic2[ch] + a2 * v.ic1[ch] + a3 * v3;
                v.ic1[ch] = 2.0 * v1 - v.ic1[ch];
                v.ic2[ch] = 2.0 * v2 - v.ic2[ch];
                y[ch] = match c.filter {
                    1 => v1 * k,
                    2 => x - k * v1 - v2,
                    _ => v2,
                };
            }
            out_l[i + j] += y[0] * amp;
            out_r[i + j] += y[1] * amp;
        }
        // Guard against blow-ups from extreme modulation.
        if !(v.ic1[0].is_finite() && v.ic1[1].is_finite() && v.ic2[0].is_finite() && v.ic2[1].is_finite()) {
            v.ic1 = [0.0; 2];
            v.ic2 = [0.0; 2];
        }
        if !v.aenv.is_active() {
            v.active = false;
            v.gate = false;
        }
        i += len;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsp::fft::Fft;

    fn render(st: &mut WavesState, p: &[f32], secs: f32, sr: f32) -> (Vec<f32>, Vec<f32>) {
        let n = (secs * sr) as usize;
        let mut l = vec![0.0; n];
        let mut r = vec![0.0; n];
        for (cl, cr) in l.chunks_mut(64).zip(r.chunks_mut(64)) {
            st.render(p, cl, cr);
        }
        (l, r)
    }

    fn rms(x: &[f32]) -> f32 {
        (x.iter().map(|v| v * v).sum::<f32>() / x.len().max(1) as f32).sqrt()
    }

    fn peak(x: &[f32]) -> f32 {
        x.iter().fold(0.0f32, |m, v| m.max(v.abs()))
    }

    /// Pitch by normalized autocorrelation with parabolic refinement.
    fn pitch_hz(x: &[f32], sr: f32, lo: f32, hi: f32) -> f32 {
        let (min_lag, max_lag) = ((sr / hi) as usize, (sr / lo) as usize);
        let n = x.len() - max_lag - 2;
        let corr = |lag: usize| {
            let (mut xy, mut xx, mut yy) = (0.0f64, 0.0f64, 0.0f64);
            for i in 0..n {
                let (a, b) = (x[i] as f64, x[i + lag] as f64);
                xy += a * b;
                xx += a * a;
                yy += b * b;
            }
            xy / (xx * yy).sqrt().max(1e-20)
        };
        let s: Vec<f64> = (min_lag..=max_lag + 1).map(corr).collect();
        let best = s.iter().cloned().fold(f64::MIN, f64::max);
        let b = (1..s.len() - 1).find(|&i| s[i] >= 0.97 * best && s[i] >= s[i - 1] && s[i] >= s[i + 1]).unwrap();
        let shift = 0.5 * (s[b - 1] - s[b + 1]) / (s[b - 1] - 2.0 * s[b] + s[b + 1]);
        sr / ((min_lag + b) as f64 + shift) as f32
    }

    /// Hann-windowed spectrum (linear magnitude) of the last 8192 samples.
    fn spectrum(x: &[f32]) -> Vec<f32> {
        let mut fft = Fft::new(8192);
        let mut db = Vec::new();
        fft.spectrum_db(x, &mut db);
        db.iter().map(|d| 10f32.powf(d / 20.0)).collect()
    }

    /// Power-weighted mean frequency between 40 Hz and 20 kHz.
    fn centroid(x: &[f32], sr: f32) -> f32 {
        let s = spectrum(x);
        let hz = |k: usize| k as f32 * sr / 8192.0;
        let (mut num, mut den) = (0.0f64, 0.0f64);
        for (k, m) in s.iter().enumerate().filter(|(k, _)| (40.0..20000.0).contains(&hz(*k))) {
            let pw = (*m as f64) * (*m as f64);
            num += hz(k) as f64 * pw;
            den += pw;
        }
        (num / den.max(1e-30)) as f32
    }

    /// A plain, open-filter patch playing one oscillator.
    fn open_patch(bank_index: usize, pos: f32) -> Vec<f32> {
        let mut p = preset("Init");
        p[BANK1] = bank_index as f32;
        p[POS1] = pos;
        p[CUTOFF] = 20000.0;
        p[KEY_TRACK] = 0.0;
        p[RESONANCE] = 0.0;
        p[SUSTAIN] = 1.0;
        p
    }

    #[test]
    fn data_has_128_normalized_waves() {
        assert_eq!(DATA.len(), WAVE_COUNT * WAVE_LEN * 2);
        assert_eq!(BANKS.len() * BANK_SIZE, WAVE_COUNT);
        for i in 0..WAVE_COUNT {
            let w = wave(i);
            assert_eq!(w.len(), WAVE_LEN);
            assert!((peak(w) - 1.0).abs() < 1e-3, "wave {i} peaks at {}", peak(w));
            let mean = w.iter().sum::<f32>() / WAVE_LEN as f32;
            assert!(mean.abs() < 1e-3, "wave {i} has DC {mean}");
            assert!(rms(w) > 0.05, "wave {i} is nearly silent");
        }
    }

    #[test]
    fn mip_levels_hold_only_their_harmonics() {
        let t = mip_tables();
        assert_eq!(t.len(), WAVE_COUNT * STRIDE);
        // The Basic saw has every harmonic.
        let w = 3;
        for (m, &n) in SIZES.iter().enumerate() {
            let tab = &t[w * STRIDE + OFFSETS[m]..][..n + 1];
            assert_eq!(tab[n], tab[0]);
            let mag = |h: usize| {
                let (mut re, mut im) = (0.0f64, 0.0f64);
                for (i, &x) in tab[..n].iter().enumerate() {
                    let a = std::f64::consts::TAU * (h * i) as f64 / n as f64;
                    re += x as f64 * a.cos();
                    im += x as f64 * a.sin();
                }
                2.0 * (re * re + im * im).sqrt() / n as f64
            };
            let top = TOP >> m;
            assert!(mag(1) > 0.3, "level {m}: fundamental {}", mag(1));
            assert!(mag(top) > 5e-4, "level {m}: top harmonic {top} missing");
            for h in [top + 1, top + 2, 2 * top + 1] {
                assert!(mag(h) < 1e-5, "level {m}: harmonic {h} at {}", mag(h));
            }
        }
    }

    #[test]
    fn every_preset_sounds_and_stops() {
        let sr = 48000.0;
        for name in preset_names() {
            let p = preset(name);
            assert_eq!(p.len(), SPECS.len());
            let mut st = WavesState::new(sr);
            st.note_on(&p, 48, 0.9);
            st.note_on(&p, 55, 0.9);
            let (l, r) = render(&mut st, &p, 1.0, sr);
            let level = rms(&l).max(rms(&r));
            assert!(level > 3e-3, "{name} is silent ({level})");
            assert!(l.iter().chain(&r).all(|v| v.is_finite()), "{name} is not finite");
            let pk = peak(&l).max(peak(&r));
            assert!(pk < 2.0, "{name} peaks at {pk}");
            st.note_off(&p, 48);
            st.note_off(&p, 55);
            let mut secs = 0.0;
            while st.active_voices() > 0 && secs < 20.0 {
                render(&mut st, &p, 0.5, sr);
                secs += 0.5;
            }
            assert_eq!(st.active_voices(), 0, "{name} keeps a voice");
            let (l, _) = render(&mut st, &p, 0.05, sr);
            assert!(l.iter().all(|&v| v == 0.0), "{name} still makes sound");
        }
    }

    #[test]
    fn a4_is_440_hz() {
        for sr in [48000.0f32, 44100.0] {
            // Position 0 of Basic is a sine.
            let p = open_patch(0, 0.0);
            let mut st = WavesState::new(sr);
            st.note_on(&p, 69, 0.8);
            let (l, _) = render(&mut st, &p, 0.6, sr);
            let tail = &l[(0.1 * sr) as usize..];
            let f = pitch_hz(tail, sr, 200.0, 1000.0);
            assert!((f - 440.0).abs() < 0.5, "{sr}: autocorrelation says {f} Hz");
            // Rising zero crossings agree.
            let ups: Vec<usize> = (1..tail.len()).filter(|&i| tail[i - 1] < 0.0 && tail[i] >= 0.0).collect();
            let periods = (ups.len() - 1) as f32;
            let f = sr * periods / (ups[ups.len() - 1] - ups[0]) as f32;
            assert!((f - 440.0).abs() < 1.0, "{sr}: zero crossings say {f} Hz");
        }
    }

    #[test]
    fn high_notes_do_not_alias() {
        let sr = 48000.0;
        // A saw, a narrow pulse, the brightest saw and the harshest raw wave.
        for (b, pos) in [(0, 3.0 / 7.0), (0, 1.0), (1, 1.0), (15, 1.0)] {
            let p = open_patch(b, pos);
            let mut st = WavesState::new(sr);
            st.note_on(&p, 108, 1.0); // C8, ~4186 Hz
            let (l, _) = render(&mut st, &p, 0.5, sr);
            let s = spectrum(&l);
            let bin0 = (midi_to_freq(108.0) / sr * 8192.0).round() as usize;
            let fundamental = s[bin0 - 2..=bin0 + 2].iter().cloned().fold(0.0f32, f32::max);
            // Everything below the fundamental would be aliasing.
            let below = s[8..bin0 - 20].iter().cloned().fold(0.0f32, f32::max);
            let db = 20.0 * (below / fundamental).log10();
            assert!(db < -70.0, "bank {b} pos {pos}: aliasing at {db:.1} dB");
        }
    }

    #[test]
    fn moving_position_changes_the_spectrum() {
        let sr = 48000.0;
        let bright = |b: usize, pos: f32| {
            let p = open_patch(b, pos);
            let mut st = WavesState::new(sr);
            st.note_on(&p, 45, 0.8);
            let (l, _) = render(&mut st, &p, 0.4, sr);
            centroid(&l, sr)
        };
        // Basic goes from a sine to a narrow pulse.
        let (sine, pulse) = (bright(0, 0.0), bright(0, 1.0));
        assert!(sine < 150.0 && pulse > 4.0 * sine, "sine {sine} Hz, pulse {pulse} Hz");
        // Every bank sounds brighter at its end.
        for (b, name) in BANKS.iter().enumerate() {
            let (start, end) = (bright(b, 0.0), bright(b, 1.0));
            assert!(end > 1.3 * start, "{name}: {start} Hz to {end} Hz");
        }
    }

    #[test]
    fn eight_voice_chords_stay_under_2() {
        let sr = 48000.0;
        for name in preset_names() {
            let p = preset(name);
            let mut st = WavesState::new(sr);
            for k in [36, 43, 48, 52, 55, 59, 62, 67] {
                st.note_on(&p, k, 1.0);
            }
            let (l, r) = render(&mut st, &p, 2.0, sr);
            let pk = peak(&l).max(peak(&r));
            assert!(pk < 2.0, "{name} chord peaks at {pk}");
            assert!(pk > 0.05, "{name} chord is too quiet ({pk})");
        }
    }

    #[test]
    fn warps_change_the_sound_and_stay_bounded() {
        let sr = 48000.0;
        let off = {
            let p = open_patch(0, 3.0 / 7.0);
            let mut st = WavesState::new(sr);
            st.note_on(&p, 57, 0.8);
            render(&mut st, &p, 0.3, sr).0
        };
        for warp in 1..WARPS.len() {
            for amount in [0.0, 0.3, 0.7, 1.0] {
                for note in [36, 72, 108] {
                    let mut p = open_patch(0, 3.0 / 7.0);
                    p[WARP1] = warp as f32;
                    p[WARP_AMT1] = amount;
                    let mut st = WavesState::new(sr);
                    st.note_on(&p, note, 0.8);
                    let (l, r) = render(&mut st, &p, 0.3, sr);
                    assert!(l.iter().chain(&r).all(|v| v.is_finite()));
                    let pk = peak(&l);
                    assert!(pk < 1.5 && rms(&l) > 1e-3, "{} {amount} note {note}: peak {pk}, rms {}", WARPS[warp], rms(&l));
                }
            }
            // Away from zero every warp but Quantize changes a single wave.
            if WARPS[warp] != "Quantize" {
                let mut p = open_patch(0, 3.0 / 7.0);
                p[WARP1] = warp as f32;
                p[WARP_AMT1] = 0.6;
                let mut st = WavesState::new(sr);
                st.note_on(&p, 57, 0.8);
                let (l, _) = render(&mut st, &p, 0.3, sr);
                let diff: Vec<f32> = l.iter().zip(&off).map(|(a, b)| a - b).collect();
                assert!(rms(&diff[4800..]) > 0.1 * rms(&off[4800..]), "{} sounds like Off", WARPS[warp]);
            }
        }
    }

    #[test]
    fn quantize_snaps_position_to_whole_waves() {
        let sr = 48000.0;
        let play = |pos: f32, quantize: bool| {
            let mut p = open_patch(0, pos);
            if quantize {
                p[WARP1] = 4.0;
                p[WARP_AMT1] = 1.0;
            }
            let mut st = WavesState::new(sr);
            st.note_on(&p, 57, 0.8);
            render(&mut st, &p, 0.2, sr).0
        };
        // Just past wave 3 snaps back to wave 3.
        let snapped = play(3.3 / 7.0, true);
        let whole = play(3.0 / 7.0, false);
        let diff: Vec<f32> = snapped.iter().zip(&whole).map(|(a, b)| a - b).collect();
        assert!(rms(&diff[4800..]) < 1e-3 * rms(&whole[4800..]).max(1e-3) + 1e-4, "{}", rms(&diff[4800..]));
    }

    #[test]
    fn voices_are_limited_mono_is_legato_and_render_adds() {
        let sr = 48000.0;
        let mut p = preset("Init");
        p[VOICES] = 3.0;
        let mut st = WavesState::new(sr);
        for k in 0..6 {
            st.note_on(&p, 60 + k, 0.8);
        }
        render(&mut st, &p, 0.05, sr);
        assert_eq!(st.active_voices(), 3);

        p[VOICES] = 1.0;
        let mut st = WavesState::new(sr);
        st.note_on(&p, 60, 0.8);
        st.note_on(&p, 64, 0.8);
        render(&mut st, &p, 0.05, sr);
        assert_eq!(st.active_voices(), 1);
        // Letting go of the top key falls back to the held one.
        st.note_off(&p, 64);
        let (l, _) = render(&mut st, &p, 0.3, sr);
        assert!(rms(&l[4800..]) > 0.01);
        st.note_off(&p, 60);
        render(&mut st, &p, 2.0, sr);
        assert_eq!(st.active_voices(), 0);

        // Output is added, not overwritten; short parameter lists are ignored.
        let mut l = vec![0.5f32; 100];
        let mut r = vec![0.5f32; 100];
        st.render(&p, &mut l, &mut r);
        assert!(l.iter().chain(&r).all(|&v| v == 0.5));
        st.note_on(&p[..3], 60, 1.0);
        assert_eq!(st.active_voices(), 0);
    }

    #[test]
    #[ignore]
    fn report_cpu() {
        for name in ["Init", "Super Saw", "Granular Haze", "Drawn Pluck"] {
            let p = preset(name);
            let mut st = WavesState::new(48000.0);
            for k in 0..8 {
                st.note_on(&p, 48 + 3 * k, 0.8);
            }
            let t = std::time::Instant::now();
            let _ = render(&mut st, &p, 1.0, 48000.0);
            let el = t.elapsed().as_secs_f32();
            eprintln!("{name}: 1 s of 8 voices in {:.1} ms ({:.1}% of a core)", el * 1000.0, el * 100.0);
        }
    }
}

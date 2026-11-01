//! Princess Sounds: a library of samples in the sound of Analog Princess's
//! music (pluggnb, deep-fried beats and their sweet, airy melodies), all
//! made here by synthesis, nothing downloaded. They are made the first time
//! they are wanted and kept as WAV files (44.1 kHz, 16 bits, stereo) in
//! Documents/Krabton/Library/Princess Sounds, a folder for each kind, so
//! they are samples like any other: on the Drum Rack, the Sampler, Grit,
//! Princess Flip, Analog Princess's tracks, as audio clips.
//!
//! - Bells and plucks: FM bells (a sine phase modulated by another, its
//!   index dying faster than its level, so they go from bright to round),
//!   bells of their own partials, a music box, a kalimba; plucked strings
//!   (Karplus and Strong), plucks of detuned saws through a closing filter,
//!   marimba, a chip pluck.
//! - Leads: sines and squares that sing, with vibrato that comes in late.
//! - Chord stabs in all twelve keys (maj7, maj9, add9) and a few minor ones,
//!   an electric piano of FM with soft saws under it; airy pads of detuned
//!   saws, a choir, all with their room in them.
//! - Flutes (a sine with its breath and chiff) and vocal chops: a voice of
//!   a buzz through five formants of a vowel (ah, oh, ooh, ee).
//! - 808s: Analog Princess's own 808 (see `dsp::princess`), clean,
//!   distorted, put into another 808, rung, crushed, gliding down or up.
//! - FX: lasers and zaps, risers, reverse cymbals, a downlifter, an impact,
//!   tape stop, channel surfing, vinyl crackle, and glitch tags.
//! - Melody loops: two- and four-bar melodies over progressions of sevenths
//!   and ninths at 140 to 160 BPM in a half-time feel, named with their key
//!   and tempo (warping reads the tempo from the name), each with its tail
//!   folded into its start so it loops without a seam.
//!
//! Each sound starts and ends at silence (no click), and all are brought to
//! about the same loudness (their loudest 300 ms to -15 dBFS, the peak
//! never over -1 dBFS). The same sounds come out every time.

use crate::dsp::drums::DrumVoice;
use crate::dsp::princess::{Dist, Knobs808, Princess808, PrincessVoice};
use crate::dsp::reverb::Reverb;
use crate::dsp::{fast_tanh, midi_to_freq, osc, FilterMode, Rng, Svf, Wave, TAU};
use std::path::{Path, PathBuf};

/// The rate they are made at.
pub const RATE: u32 = 44_100;
const SR: f32 = RATE as f32;
/// Made again when this changes (a sound made better).
pub const VERSION: u32 = 1;
pub const FOLDER: &str = "Princess Sounds";

/// A chord, from its root (semitones).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Chord {
    Maj7,
    Maj9,
    Add9,
    Min7,
    Min9,
    Min11,
}

impl Chord {
    pub fn notes(self) -> &'static [i32] {
        match self {
            Chord::Maj7 => &[0, 4, 7, 11],
            Chord::Maj9 => &[0, 4, 7, 11, 14],
            Chord::Add9 => &[0, 4, 7, 14],
            Chord::Min7 => &[0, 3, 7, 10],
            Chord::Min9 => &[0, 3, 7, 10, 14],
            Chord::Min11 => &[0, 3, 7, 10, 14, 17],
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Chord::Maj7 => "maj7",
            Chord::Maj9 => "maj9",
            Chord::Add9 => "add9",
            Chord::Min7 => "m7",
            Chord::Min9 => "m9",
            Chord::Min11 => "m11",
        }
    }
}

/// What a sound is made as.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    Bell(u8),
    Pluck(u8),
    Lead(u8),
    Stab(Chord, u8),
    Pad(u8),
    Flute(u8),
    Vox(u8),
    Bass(u8),
    Fx(u8),
    Loop(u8),
}

/// A sound of the library: its folder, its name (the file's), how it is made.
#[derive(Clone, Debug, PartialEq)]
pub struct Sound {
    pub category: &'static str,
    pub name: String,
    pub kind: Kind,
}

impl Sound {
    /// Whether it is a loop (in the song's time) rather than a one-shot.
    pub fn is_loop(&self) -> bool {
        matches!(self.kind, Kind::Loop(_)) || matches!(self.kind, Kind::Fx(f) if FX[f as usize].1)
    }
}

const NOTE_NAMES: [&str; 12] = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];

const BELLS: [&str; 12] = ["Plugg Bell", "Glass Bell", "Music Box", "Celesta", "Toy Bell", "Dream Bell", "Crystal Bell", "Soft Bell", "Kalimba", "Vibe Bell", "Bell Chime", "Detuned Bell"];
const PLUCKS: [&str; 10] = ["Soft Pluck", "Bright Pluck", "Harp Pluck", "Koto Pluck", "Marimba", "Guitar Pluck", "Bubble Pluck", "Glass Pluck", "Detuned Pluck", "Chip Pluck"];
const LEADS: [&str; 8] = ["Sine Lead", "Square Lead", "Soft Square", "Whistle Lead", "Plugg Saw", "Pulse Lead", "Vibrato Sine", "Chip Lead"];
const PADS: [(&str, Chord); 6] = [("Pad maj7", Chord::Maj7), ("Pad maj9", Chord::Maj9), ("Pad add9", Chord::Add9), ("Pad m9", Chord::Min9), ("Air Pad maj9", Chord::Maj9), ("Choir Pad add9", Chord::Add9)];
const FLUTES: [&str; 5] = ["Flute", "Breathy Flute", "Pan Flute", "Vibrato Flute", "Ocarina"];
const VOX: [&str; 8] = ["Vox Ah", "Vox Oh", "Vox Ooh", "Vox Ee", "Vox Ah Low", "Vox Ah Chop", "Vox Oh Chop", "Choir Ah"];
const BASSES: [&str; 10] = ["808 Clean", "808 Distorted", "808 Into 808", "808 Ring", "808 Crunch", "808 Long", "808 Short", "808 Glide Down", "808 Glide Up", "808 Tube"];
/// The FX, and whether each is a loop (named with its tempo).
const FX: [(&str, bool); 16] = [
    ("Laser Zap", false),
    ("Laser Down", false),
    ("Laser Up", false),
    ("Zap", false),
    ("Riser Noise 150 BPM", true),
    ("Riser Tonal 150 BPM", true),
    ("Reverse Cymbal", false),
    ("Reverse Crash Long", false),
    ("Downlifter", false),
    ("Glitch Tag 1", false),
    ("Glitch Tag 2", false),
    ("Glitch Tag 3", false),
    ("Tape Stop", false),
    ("Channel Surf", false),
    ("Impact Boom", false),
    ("Vinyl Crackle 150 BPM", true),
];

/// The keys bells, leads and flutes are made in (MIDI): C4; plucks and
/// voices lower; the 808s at C1.
const BELL_NOTE: i32 = 72;
const PLUCK_NOTE: i32 = 60;
const VOX_NOTE: i32 = 67;
const BASS_NOTE: i32 = 36;

fn note_label(midi: i32) -> String {
    format!("{}{}", NOTE_NAMES[midi.rem_euclid(12) as usize], midi / 12 - 2)
}

/// A melody loop: its key (root, minor), progression, tempo, bars and what
/// plays it.
#[derive(Clone, Copy, Debug)]
struct LoopDef {
    root: i32,
    minor: bool,
    prog: usize,
    bpm: f32,
    bars: usize,
    lead: u8,
    keys: u8,
}

const LOOPS: [LoopDef; 16] = [
    LoopDef { root: 9, minor: true, prog: 0, bpm: 150.0, bars: 4, lead: 0, keys: 0 },
    LoopDef { root: 0, minor: false, prog: 1, bpm: 145.0, bars: 4, lead: 1, keys: 1 },
    LoopDef { root: 5, minor: true, prog: 2, bpm: 155.0, bars: 4, lead: 2, keys: 0 },
    LoopDef { root: 7, minor: false, prog: 3, bpm: 140.0, bars: 2, lead: 3, keys: 2 },
    LoopDef { root: 3, minor: true, prog: 0, bpm: 160.0, bars: 4, lead: 1, keys: 1 },
    LoopDef { root: 2, minor: false, prog: 1, bpm: 150.0, bars: 4, lead: 0, keys: 0 },
    LoopDef { root: 11, minor: true, prog: 2, bpm: 145.0, bars: 2, lead: 0, keys: 2 },
    LoopDef { root: 5, minor: false, prog: 3, bpm: 155.0, bars: 4, lead: 2, keys: 1 },
    LoopDef { root: 4, minor: true, prog: 0, bpm: 140.0, bars: 4, lead: 3, keys: 0 },
    LoopDef { root: 10, minor: false, prog: 1, bpm: 160.0, bars: 2, lead: 1, keys: 0 },
    LoopDef { root: 1, minor: true, prog: 2, bpm: 150.0, bars: 4, lead: 1, keys: 1 },
    LoopDef { root: 8, minor: false, prog: 3, bpm: 145.0, bars: 4, lead: 0, keys: 2 },
    LoopDef { root: 6, minor: true, prog: 3, bpm: 155.0, bars: 2, lead: 2, keys: 0 },
    LoopDef { root: 9, minor: false, prog: 0, bpm: 140.0, bars: 4, lead: 0, keys: 1 },
    LoopDef { root: 0, minor: true, prog: 1, bpm: 160.0, bars: 4, lead: 3, keys: 0 },
    LoopDef { root: 7, minor: true, prog: 2, bpm: 150.0, bars: 2, lead: 1, keys: 2 },
];

/// Progressions (each chord: semitones from the key's root, its kind),
/// the way pluggnb voices them: sevenths and ninths.
fn progression(i: usize, minor: bool) -> [(i32, Chord); 4] {
    use Chord::*;
    match (i % 4, minor) {
        (0, true) => [(0, Min9), (8, Maj7), (3, Maj7), (10, Add9)],
        (1, true) => [(0, Min9), (5, Min9), (8, Maj9), (7, Min7)],
        (2, true) => [(8, Maj7), (7, Min7), (0, Min9), (0, Min11)],
        (_, true) => [(0, Min11), (3, Maj9), (8, Maj7), (10, Add9)],
        (0, false) => [(0, Maj7), (9, Min9), (5, Maj7), (7, Add9)],
        (1, false) => [(5, Maj7), (4, Min7), (9, Min9), (0, Maj9)],
        (2, false) => [(0, Maj9), (5, Maj9), (2, Min9), (7, Add9)],
        (_, false) => [(9, Min9), (5, Maj7), (0, Add9), (7, Add9)],
    }
}

/// Every sound of the library, in order.
pub fn sounds() -> Vec<Sound> {
    let mut v = Vec::new();
    let mut put = |category: &'static str, name: String, kind: Kind| v.push(Sound { category, name, kind });
    for (k, n) in BELLS.iter().enumerate() {
        put("Bells", format!("{n} {}", note_label(BELL_NOTE)), Kind::Bell(k as u8));
    }
    for (k, n) in PLUCKS.iter().enumerate() {
        put("Plucks", format!("{n} {}", note_label(PLUCK_NOTE)), Kind::Pluck(k as u8));
    }
    for (k, n) in LEADS.iter().enumerate() {
        put("Leads", format!("{n} {}", note_label(BELL_NOTE)), Kind::Lead(k as u8));
    }
    for chord in [Chord::Maj7, Chord::Maj9, Chord::Add9] {
        for root in 0..12u8 {
            put("Chord Stabs", format!("Stab {}{}", NOTE_NAMES[root as usize], chord.name()), Kind::Stab(chord, root));
        }
    }
    for chord in [Chord::Min7, Chord::Min9, Chord::Min11] {
        put("Chord Stabs", format!("Stab C{}", chord.name()), Kind::Stab(chord, 0));
    }
    for (k, (n, _)) in PADS.iter().enumerate() {
        put("Pads", format!("{n} C3"), Kind::Pad(k as u8));
    }
    for (k, n) in FLUTES.iter().enumerate() {
        put("Flutes", format!("{n} {}", note_label(BELL_NOTE)), Kind::Flute(k as u8));
    }
    for (k, n) in VOX.iter().enumerate() {
        let note = if k == 4 { VOX_NOTE - 19 } else { VOX_NOTE };
        put("Vocal Chops", format!("{n} {}", note_label(note)), Kind::Vox(k as u8));
    }
    for (k, n) in BASSES.iter().enumerate() {
        put("808s", format!("{n} {}", note_label(BASS_NOTE)), Kind::Bass(k as u8));
    }
    for (k, (n, _)) in FX.iter().enumerate() {
        put("FX", n.to_string(), Kind::Fx(k as u8));
    }
    for (k, d) in LOOPS.iter().enumerate() {
        let key = format!("{} {}", NOTE_NAMES[d.root as usize], if d.minor { "minor" } else { "major" });
        put("Melody Loops", format!("Plugg Loop {:02} {key} {} BPM", k + 1, d.bpm as u32), Kind::Loop(k as u8));
    }
    v
}

/// The one-shots (no loops), for a place to pick one from.
pub fn one_shots() -> Vec<Sound> {
    sounds().into_iter().filter(|s| !s.is_loop()).collect()
}

// ------------------------------------------------------------ where they go

/// Where the library is kept: Documents/Krabton/Library/Princess Sounds
/// (`KRABTON_PRINCESS_SOUNDS` puts it elsewhere).
pub fn dir() -> PathBuf {
    if let Some(d) = std::env::var_os("KRABTON_PRINCESS_SOUNDS") {
        return PathBuf::from(d);
    }
    let home = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")).map(PathBuf::from).unwrap_or_default();
    home.join("Documents").join("Krabton").join("Library").join(FOLDER)
}

pub fn path_of(dir: &Path, s: &Sound) -> PathBuf {
    dir.join(s.category).join(format!("{}.wav", s.name))
}

fn version_file(dir: &Path) -> PathBuf {
    dir.join("version.txt")
}

/// Whether every sound is there, made by this version.
pub fn installed(dir: &Path) -> bool {
    std::fs::read_to_string(version_file(dir)).ok().and_then(|v| v.trim().parse::<u32>().ok()) == Some(VERSION) && sounds().iter().all(|s| path_of(dir, s).is_file())
}

/// Makes the sound (if it is not there yet, or an older version made the
/// library) and returns its file.
pub fn ensure(dir: &Path, s: &Sound) -> Result<PathBuf, String> {
    let path = path_of(dir, s);
    let current = std::fs::read_to_string(version_file(dir)).ok().and_then(|v| v.trim().parse::<u32>().ok()) == Some(VERSION);
    if !path.is_file() || !current {
        let (l, r) = render(s);
        write_wav(&path, &l, &r)?;
    }
    Ok(path)
}

/// Makes every sound not there yet (all of them for a library of an older
/// version), telling `progress` how far it is (done, of). Takes a few
/// seconds, so never on the audio or the UI thread.
pub fn install(dir: &Path, progress: &dyn Fn(usize, usize)) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let current = std::fs::read_to_string(version_file(dir)).ok().and_then(|v| v.trim().parse::<u32>().ok()) == Some(VERSION);
    let all = sounds();
    for (k, s) in all.iter().enumerate() {
        progress(k, all.len());
        let path = path_of(dir, s);
        if current && path.is_file() {
            continue;
        }
        let (l, r) = render(s);
        write_wav(&path, &l, &r)?;
    }
    std::fs::write(version_file(dir), format!("{VERSION}\n")).map_err(|e| e.to_string())?;
    let _ = std::fs::write(dir.join("README.txt"), "Princess Sounds: made by Krabton, by synthesis. Use them however you like.\n");
    progress(all.len(), all.len());
    Ok(())
}

/// 16-bit stereo, dithered.
pub fn write_wav(path: &Path, l: &[f32], r: &[f32]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    }
    let spec = hound::WavSpec { channels: 2, sample_rate: RATE, bits_per_sample: 16, sample_format: hound::SampleFormat::Int };
    // Written beside it first, so a half-made file never looks finished.
    let tmp = path.with_extension("wav.part");
    let mut w = hound::WavWriter::create(&tmp, spec).map_err(|e| format!("{}: {e}", tmp.display()))?;
    let mut rng = Rng::new(0xD17E_4011);
    for (a, b) in l.iter().zip(r) {
        for x in [*a, *b] {
            let d = (rng.unipolar() - rng.unipolar()) / 32768.0;
            let v = ((x + d).clamp(-1.0, 1.0) * 32767.0).round() as i16;
            w.write_sample(v).map_err(|e| e.to_string())?;
        }
    }
    w.finalize().map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path).map_err(|e| format!("{}: {e}", path.display()))
}

// ------------------------------------------------------------ making them

fn secs(s: f32) -> usize {
    (s * SR) as usize
}

/// A stereo buffer being made.
struct Buf {
    l: Vec<f32>,
    r: Vec<f32>,
}

impl Buf {
    fn new(len: usize) -> Self {
        Self { l: vec![0.0; len], r: vec![0.0; len] }
    }

    fn len(&self) -> usize {
        self.l.len()
    }

    /// `x` from `at`, at `gain`, panned (-1 to 1).
    fn add(&mut self, at: usize, x: &[f32], gain: f32, pan: f32) {
        let (gl, gr) = ((1.0 - pan).min(1.0) * gain, (1.0 + pan).min(1.0) * gain);
        for (i, v) in x.iter().enumerate() {
            if let (Some(a), Some(b)) = (self.l.get_mut(at + i), self.r.get_mut(at + i)) {
                *a += v * gl;
                *b += v * gr;
            }
        }
    }

    fn room(&mut self, size: f32, mix: f32) {
        let mut rv = Reverb::new(SR);
        rv.process(size, 0.5, 1.0, 15.0, 180.0, mix, &mut self.l, &mut self.r);
    }
}

/// Up over `a` seconds (a raised half sine), held, then out over the last
/// `r` seconds of `len` samples.
fn shape(x: &mut [f32], a: f32, r: f32) {
    let n = x.len();
    let (na, nr) = (secs(a).max(1), secs(r).max(1));
    for (i, v) in x.iter_mut().enumerate() {
        let up = if i < na { 0.5 - 0.5 * (std::f32::consts::PI * i as f32 / na as f32).cos() } else { 1.0 };
        let left = n - i;
        let down = if left < nr { 0.5 - 0.5 * (std::f32::consts::PI * left as f32 / nr as f32).cos() } else { 1.0 };
        *v *= up * down;
    }
}

fn exp_env(t: f32, d: f32) -> f32 {
    (-t / d.max(1e-4)).exp()
}

/// A slow vibrato that comes in after `delay` seconds (in semitones).
fn vibrato(t: f32, delay: f32, rate: f32, depth: f32) -> f32 {
    let amt = ((t - delay) / 0.4).clamp(0.0, 1.0);
    amt * depth * (TAU * rate * t).sin()
}

/// An FM bell: a sine phase modulated at `ratio` its pitch, the index
/// dying away faster than the level.
fn fm_tone(hz: f32, len: usize, ratio: f32, index: f32, index_decay: f32, decay: f32, rng: &mut Rng) -> Vec<f32> {
    let mut v = vec![0.0f32; len];
    let (mut pc, mut pm) = (rng.unipolar(), 0.0f32);
    for (i, x) in v.iter_mut().enumerate() {
        let t = i as f32 / SR;
        pm = (pm + hz * ratio / SR).fract();
        let m = (pm * TAU).sin() * index * exp_env(t, index_decay);
        pc = (pc + hz / SR).fract();
        *x = (pc * TAU + m).sin() * exp_env(t, decay);
    }
    v
}

/// A tone of its own partials (multiples of `hz`, their levels and decays).
fn partials(hz: f32, len: usize, ratios: &[f32], amps: &[f32], decays: &[f32], rng: &mut Rng) -> Vec<f32> {
    let mut v = vec![0.0f32; len];
    for ((&ratio, &amp), &d) in ratios.iter().zip(amps).zip(decays) {
        let f = hz * ratio;
        if f > SR * 0.45 {
            continue;
        }
        let mut ph = rng.unipolar();
        for (i, x) in v.iter_mut().enumerate() {
            ph = (ph + f / SR).fract();
            *x += (ph * TAU).sin() * amp * exp_env(i as f32 / SR, d);
        }
    }
    v
}

/// A plucked string (Karplus and Strong): a burst of noise round a delay of
/// one period, averaged as it goes round (`bright`: how little).
fn string(hz: f32, len: usize, bright: f32, decay: f32, rng: &mut Rng) -> Vec<f32> {
    let period = SR / hz;
    let n = period as usize + 2;
    let mut line: Vec<f32> = (0..n).map(|_| rng.bipolar()).collect();
    // The burst a little dark, as a plectrum's.
    let mut lp = 0.0;
    for x in line.iter_mut() {
        lp += (*x - lp) * (0.3 + 0.6 * bright);
        *x = lp;
    }
    let loss = (-1.0 / (decay * hz)).exp();
    let mut out = vec![0.0f32; len];
    let mut w = 0usize;
    let frac = period - period.floor();
    let mut prev = 0.0f32;
    for x in out.iter_mut() {
        let r0 = line[(w + n - period.floor() as usize) % n];
        let r1 = line[(w + n - period.floor() as usize - 1) % n];
        let y = r0 + (r1 - r0) * frac;
        let avg = y * (0.5 + 0.5 * bright) + prev * (0.5 - 0.5 * bright);
        prev = y;
        line[w % n] = avg * loss;
        w = (w + 1) % n;
        *x = y;
    }
    out
}

/// Saws (or another wave) detuned, through a low-pass that closes from
/// `open` to `close` Hz over `fall` seconds.
#[allow(clippy::too_many_arguments)]
fn filtered(hz: f32, len: usize, wave: Wave, detune: &[f32], open: f32, close: f32, fall: f32, res: f32, rng: &mut Rng) -> Vec<f32> {
    let mut v = vec![0.0f32; len];
    let mut phases: Vec<f32> = detune.iter().map(|_| rng.unipolar()).collect();
    let mut f = Svf::new();
    for (i, x) in v.iter_mut().enumerate() {
        let t = i as f32 / SR;
        let mut s = 0.0;
        for (p, &c) in phases.iter_mut().zip(detune) {
            let fr = hz * 2f32.powf(c / 1200.0);
            *p = (*p + fr / SR).fract();
            s += osc(wave, *p, fr / SR);
        }
        let cut = close + (open - close) * exp_env(t, fall);
        f.set(cut, res, SR);
        *x = f.process(s / detune.len() as f32, FilterMode::Low);
    }
    v
}

/// A buzz through the five formants of a vowel (Hz, bandwidth Hz, level):
/// a voice singing it, with its vibrato and a little breath.
fn voice(hz: f32, len: usize, formants: &[(f32, f32, f32); 5], breath: f32, vib: f32, rng: &mut Rng) -> Vec<f32> {
    let mut filt: Vec<Svf> = formants
        .iter()
        .map(|&(f, bw, _)| {
            let mut s = Svf::new();
            s.set_q(f, f / bw, SR);
            s
        })
        .collect();
    let mut v = vec![0.0f32; len];
    let mut ph = rng.unipolar();
    let mut breath_lp = 0.0f32;
    let mut glottal_lp = 0.0f32;
    for (i, x) in v.iter_mut().enumerate() {
        let t = i as f32 / SR;
        let f = hz * 2f32.powf((vibrato(t, 0.25, 5.2, vib) + 0.03 * (TAU * 0.7 * t).sin()) / 12.0);
        ph = (ph + f / SR).fract();
        // A glottal pulse: a saw rounded off.
        let saw = osc(Wave::Saw, ph, f / SR);
        glottal_lp += (saw - glottal_lp) * 0.35;
        let n = rng.bipolar();
        breath_lp += (n - breath_lp) * 0.5;
        let src = glottal_lp + breath_lp * breath;
        let mut y = 0.0;
        for (fl, &(_, _, g)) in filt.iter_mut().zip(formants) {
            y += fl.process(src, FilterMode::Band) * g;
        }
        *x = y;
    }
    v
}

/// The vowels' formants (a soprano's, but for the low ah).
const AH: [(f32, f32, f32); 5] = [(800.0, 80.0, 1.0), (1150.0, 90.0, 0.63), (2900.0, 120.0, 0.1), (3900.0, 130.0, 0.016), (4950.0, 140.0, 0.002)];
const OH: [(f32, f32, f32); 5] = [(450.0, 70.0, 1.0), (800.0, 80.0, 0.28), (2830.0, 100.0, 0.07), (3800.0, 130.0, 0.06), (4950.0, 135.0, 0.003)];
const OOH: [(f32, f32, f32); 5] = [(325.0, 50.0, 1.0), (700.0, 60.0, 0.16), (2700.0, 170.0, 0.018), (3800.0, 180.0, 0.01), (4950.0, 200.0, 0.001)];
const EE: [(f32, f32, f32); 5] = [(270.0, 60.0, 1.0), (2140.0, 90.0, 0.25), (2950.0, 100.0, 0.05), (3900.0, 120.0, 0.05), (4950.0, 120.0, 0.006)];
const AH_LOW: [(f32, f32, f32); 5] = [(700.0, 80.0, 1.0), (1220.0, 90.0, 0.5), (2600.0, 120.0, 0.25), (3200.0, 130.0, 0.12), (3700.0, 140.0, 0.06)];

/// A flute: a sine with a little of its octave and twelfth, the breath in a
/// band round its pitch, a chiff at the start, vibrato coming in.
fn flute(hz: f32, len: usize, breath: f32, vib: f32, chiff: f32, pure: bool, rng: &mut Rng) -> Vec<f32> {
    let mut v = vec![0.0f32; len];
    let mut ph = rng.unipolar();
    let mut band = Svf::new();
    band.set_q(hz * 2.0, 3.0, SR);
    let mut hp = Svf::new();
    hp.set(1500.0, 0.0, SR);
    for (i, x) in v.iter_mut().enumerate() {
        let t = i as f32 / SR;
        let f = hz * 2f32.powf(vibrato(t, 0.35, 5.0, vib) / 12.0);
        ph = (ph + f / SR).fract();
        let s = (ph * TAU).sin();
        let tone = if pure { s + 0.08 * (3.0 * ph * TAU).sin() } else { s + 0.18 * (2.0 * ph * TAU).sin() + 0.06 * (3.0 * ph * TAU).sin() };
        let n = rng.bipolar();
        let air = band.process(n, FilterMode::Band) * breath * (0.7 + 0.3 * s.abs());
        let ch = hp.process(n, FilterMode::High) * chiff * exp_env(t, 0.03);
        // It swells into the note.
        let swell = 1.0 - exp_env(t, 0.06);
        *x = tone * swell * 0.8 + air + ch;
    }
    v
}

fn bits(x: &mut [f32], b: f32, hold: usize) {
    let steps = 2f32.powf(b - 1.0);
    let mut held = 0.0;
    for (i, v) in x.iter_mut().enumerate() {
        if i % hold.max(1) == 0 {
            held = (*v * steps).round() / steps;
        }
        *v = held;
    }
}

fn bell(style: u8, hz: f32, len: usize, rng: &mut Rng) -> Buf {
    let mut b = Buf::new(len);
    let mono = |b: &mut Buf, x: Vec<f32>, g: f32| b.add(0, &x, g, 0.0);
    match style {
        0 => {
            // The plugg bell: FM at twice, a tine above, a copy a little sharp to the right.
            let a = fm_tone(hz, len, 2.0, 1.8, 0.25, 1.1, rng);
            let c = fm_tone(hz * 2f32.powf(6.0 / 1200.0), len, 2.0, 1.4, 0.2, 1.0, rng);
            let tine = partials(hz, len, &[4.0], &[0.12], &[0.08], rng);
            b.add(0, &a, 0.6, -0.25);
            b.add(0, &c, 0.5, 0.3);
            mono(&mut b, tine, 1.0);
            b.room(0.6, 0.22);
        }
        1 => {
            let x = partials(hz, len, &[1.0, 2.76, 5.4, 8.93], &[1.0, 0.5, 0.25, 0.12], &[2.0, 1.0, 0.5, 0.25], rng);
            mono(&mut b, x, 0.6);
            b.room(0.7, 0.25);
        }
        2 => {
            let x = partials(hz, len, &[1.0, 3.9, 8.1], &[1.0, 0.35, 0.12], &[1.2, 0.35, 0.12], rng);
            let click = partials(hz, len, &[11.7], &[0.15], &[0.01], rng);
            mono(&mut b, x, 0.7);
            mono(&mut b, click, 1.0);
            b.room(0.55, 0.2);
        }
        3 => {
            let x = partials(hz, len, &[1.0, 2.0, 3.0, 4.0], &[1.0, 0.4, 0.15, 0.08], &[1.4, 0.6, 0.3, 0.2], rng);
            let mut hammer: Vec<f32> = (0..len).map(|i| rng.bipolar() * exp_env(i as f32 / SR, 0.004) * 0.3).collect();
            let mut f = Svf::new();
            f.set_q(hz * 6.0, 1.0, SR);
            hammer.iter_mut().for_each(|v| *v = f.process(*v, FilterMode::Band));
            mono(&mut b, x, 0.6);
            mono(&mut b, hammer, 1.0);
            b.room(0.6, 0.2);
        }
        4 => {
            let x = fm_tone(hz, len, 3.5, 2.5, 0.08, 0.5, rng);
            mono(&mut b, x, 0.6);
            b.room(0.5, 0.2);
        }
        5 => {
            for (k, c) in [-7.0f32, 0.0, 7.0].iter().enumerate() {
                let x = fm_tone(hz * 2f32.powf(c / 1200.0), len, 2.0, 1.0, 0.5, 2.2, rng);
                b.add(0, &x, 0.35, [-0.7, 0.0, 0.7][k]);
            }
            b.room(0.88, 0.4);
        }
        6 => {
            let x = partials(hz, len, &[1.0, 2.0, 3.0, 4.2, 5.4], &[1.0, 0.6, 0.35, 0.2, 0.12], &[1.6, 1.0, 0.6, 0.4, 0.3], rng);
            let y = partials(hz * 2f32.powf(4.0 / 1200.0), len, &[1.0, 2.0, 3.0], &[0.6, 0.3, 0.2], &[1.4, 0.8, 0.5], rng);
            b.add(0, &x, 0.45, -0.3);
            b.add(0, &y, 0.45, 0.3);
            b.room(0.85, 0.35);
        }
        7 => {
            let x = fm_tone(hz, len, 1.0, 0.6, 0.6, 1.3, rng);
            mono(&mut b, x, 0.7);
            b.room(0.6, 0.18);
        }
        8 => {
            // Kalimba: a tine, its high partial short, a thump.
            let x = partials(hz, len, &[1.0, 6.3, 12.0], &[1.0, 0.25, 0.05], &[0.9, 0.06, 0.02], rng);
            let thump = partials(hz * 0.5, len, &[1.0], &[0.3], &[0.03], rng);
            mono(&mut b, x, 0.7);
            mono(&mut b, thump, 1.0);
            b.room(0.5, 0.15);
        }
        9 => {
            let mut x = partials(hz, len, &[1.0, 4.0, 10.0], &[1.0, 0.3, 0.05], &[1.8, 0.5, 0.1], rng);
            for (i, v) in x.iter_mut().enumerate() {
                *v *= 1.0 - 0.35 * (0.5 + 0.5 * (TAU * 5.0 * i as f32 / SR).sin());
            }
            mono(&mut b, x, 0.7);
            b.room(0.65, 0.22);
        }
        10 => {
            let a = fm_tone(hz, len, 2.0, 1.5, 0.3, 1.4, rng);
            let o = fm_tone(hz * 2.0, len, 2.0, 1.0, 0.2, 0.9, rng);
            b.add(0, &a, 0.5, -0.15);
            b.add(0, &o, 0.3, 0.2);
            b.room(0.75, 0.3);
        }
        _ => {
            let a = fm_tone(hz * 2f32.powf(-15.0 / 1200.0), len, 2.0, 1.6, 0.25, 1.2, rng);
            let c = fm_tone(hz * 2f32.powf(15.0 / 1200.0), len, 2.0, 1.6, 0.25, 1.2, rng);
            b.add(0, &a, 0.5, -0.8);
            b.add(0, &c, 0.5, 0.8);
            bits(&mut b.l, 10.0, 2);
            bits(&mut b.r, 10.0, 2);
            b.room(0.7, 0.25);
        }
    }
    b
}

fn pluck(style: u8, hz: f32, len: usize, rng: &mut Rng) -> Buf {
    let mut b = Buf::new(len);
    match style {
        0 => {
            let x = filtered(hz, len, Wave::Saw, &[-8.0, 8.0], 2400.0, 350.0, 0.18, 0.2, rng);
            let mut x = x;
            x.iter_mut().enumerate().for_each(|(i, v)| *v *= exp_env(i as f32 / SR, 0.5));
            b.add(0, &x, 0.8, 0.0);
            b.room(0.6, 0.2);
        }
        1 => {
            let mut x = filtered(hz, len, Wave::Square, &[-5.0, 5.0], 6500.0, 700.0, 0.12, 0.35, rng);
            x.iter_mut().enumerate().for_each(|(i, v)| *v *= exp_env(i as f32 / SR, 0.4));
            b.add(0, &x, 0.7, 0.0);
            b.room(0.55, 0.18);
        }
        2 => {
            let x = string(hz, len, 0.55, 1.2, rng);
            let o = string(hz * 2.0, len, 0.5, 0.8, rng);
            b.add(0, &x, 0.6, -0.2);
            b.add(0, &o, 0.25, 0.25);
            b.room(0.7, 0.25);
        }
        3 => {
            // Koto: bright, nasal, bending into its note.
            let mut x = string(hz * 1.012, len, 0.8, 0.9, rng);
            let mut f = Svf::new();
            f.set_q(hz * 3.0, 2.0, SR);
            x.iter_mut().for_each(|v| *v += f.process(*v, FilterMode::Band) * 0.8);
            b.add(0, &x, 0.5, 0.0);
            b.room(0.6, 0.2);
        }
        4 => {
            let x = partials(hz, len, &[1.0, 4.0, 9.9], &[1.0, 0.25, 0.06], &[0.5, 0.08, 0.02], rng);
            b.add(0, &x, 0.7, 0.0);
            b.room(0.5, 0.15);
        }
        5 => {
            let x = string(hz, len, 0.3, 1.6, rng);
            b.add(0, &x, 0.7, 0.0);
            b.room(0.55, 0.15);
        }
        6 => {
            // A bubble: a sine falling into its note, short.
            let mut x = vec![0.0f32; len];
            let mut ph = 0.0f32;
            for (i, v) in x.iter_mut().enumerate() {
                let t = i as f32 / SR;
                let f = hz * 2f32.powf(12.0 * exp_env(t, 0.012) / 12.0);
                ph = (ph + f / SR).fract();
                *v = (ph * TAU).sin() * exp_env(t, 0.22);
            }
            b.add(0, &x, 0.7, 0.0);
            b.room(0.6, 0.22);
        }
        7 => {
            let x = fm_tone(hz, len, 3.0, 2.0, 0.05, 0.35, rng);
            b.add(0, &x, 0.6, 0.0);
            b.room(0.65, 0.25);
        }
        8 => {
            let l = filtered(hz, len, Wave::Saw, &[-12.0, 3.0], 3500.0, 400.0, 0.15, 0.25, rng);
            let r = filtered(hz, len, Wave::Saw, &[-3.0, 12.0], 3500.0, 400.0, 0.15, 0.25, rng);
            let env = |x: Vec<f32>| -> Vec<f32> { x.into_iter().enumerate().map(|(i, v)| v * exp_env(i as f32 / SR, 0.45)).collect() };
            b.add(0, &env(l), 0.8, -0.9);
            b.add(0, &env(r), 0.8, 0.9);
            b.room(0.6, 0.2);
        }
        _ => {
            let mut x = vec![0.0f32; len];
            let mut ph = 0.0f32;
            for (i, v) in x.iter_mut().enumerate() {
                ph = (ph + hz / SR).fract();
                *v = if ph < 0.25 { 1.0 } else { -0.33 } * exp_env(i as f32 / SR, 0.18);
            }
            bits(&mut x, 6.0, 2);
            let mut f = Svf::new();
            f.set(7000.0, 0.0, SR);
            x.iter_mut().for_each(|v| *v = f.process(*v, FilterMode::Low));
            b.add(0, &x, 0.5, 0.0);
            b.room(0.45, 0.15);
        }
    }
    b
}

fn lead(style: u8, hz: f32, len: usize, rng: &mut Rng) -> Buf {
    let mut b = Buf::new(len);
    let hold = len - secs(0.6);
    let mut x = vec![0.0f32; len];
    let mut ph = rng.unipolar();
    let mut ph2 = rng.unipolar();
    let mut f = Svf::new();
    for (i, v) in x.iter_mut().enumerate() {
        let t = i as f32 / SR;
        let vib = match style {
            6 => vibrato(t, 0.15, 5.8, 0.5),
            3 => vibrato(t, 0.3, 5.5, 0.15) - 2.0 * exp_env(t, 0.05),
            _ => vibrato(t, 0.35, 5.5, 0.15),
        };
        let fr = hz * 2f32.powf(vib / 12.0);
        ph = (ph + fr / SR).fract();
        ph2 = (ph2 + fr * 1.004 / SR).fract();
        let dt = fr / SR;
        *v = match style {
            0 | 6 => (ph * TAU).sin() + 0.06 * (2.0 * ph * TAU).sin(),
            1 => {
                f.set(3500.0, 0.1, SR);
                f.process(osc(Wave::Square, ph, dt), FilterMode::Low) * 0.6
            }
            2 => {
                f.set(1500.0, 0.0, SR);
                f.process(osc(Wave::Square, ph, dt), FilterMode::Low) * 0.6 + 0.4 * (ph * TAU).sin()
            }
            3 => (ph * TAU).sin() + rng.bipolar() * 0.03,
            4 => {
                f.set(2500.0, 0.15, SR);
                f.process(0.5 * (osc(Wave::Saw, ph, dt) + osc(Wave::Saw, ph2, dt)), FilterMode::Low)
            }
            5 => {
                let width = 0.5 + 0.35 * (TAU * 0.8 * t).sin();
                let p = if ph < width { 1.0 } else { -1.0 };
                f.set(4000.0, 0.0, SR);
                f.process(p, FilterMode::Low) * 0.5
            }
            _ => {
                if ph < 0.125 {
                    0.7
                } else {
                    -0.1
                }
            }
        };
    }
    if style == 7 {
        bits(&mut x, 5.0, 3);
    }
    // Sung for most of it, let go at the end.
    for (i, v) in x.iter_mut().enumerate() {
        let t = i as f32 / SR;
        let rel = if i > hold { exp_env((i - hold) as f32 / SR, 0.12) } else { 1.0 };
        *v *= (t / 0.015).min(1.0) * rel;
    }
    b.add(0, &x, 0.6, 0.0);
    b.room(0.55, 0.15);
    b
}

/// An electric piano's note (FM: a tine over the body).
fn keys_note(hz: f32, len: usize, decay: f32, rng: &mut Rng) -> Vec<f32> {
    let body = fm_tone(hz, len, 1.0, 1.1, 0.35, decay, rng);
    let tine = fm_tone(hz * 1.0, len, 14.0, 0.6, 0.02, 0.06, rng);
    body.iter().zip(&tine).map(|(a, b)| a * 0.8 + b * 0.15).collect()
}

/// A soft saw pad's note (three saws detuned, low-passed).
fn pad_note(hz: f32, len: usize, cut: f32, rng: &mut Rng) -> Vec<f32> {
    filtered(hz, len, Wave::Saw, &[-11.0, 0.0, 9.0], cut, cut, 1.0, 0.1, rng)
}

/// A chord's notes from `root` (MIDI), voiced in a close spread with the
/// root an octave down under it.
fn voicing(root: i32, chord: Chord) -> Vec<i32> {
    let mut v: Vec<i32> = chord.notes().iter().map(|n| root + n).collect();
    v.insert(0, root - 12);
    v
}

fn stab(chord: Chord, pc: u8, rng: &mut Rng) -> Buf {
    let len = secs(2.6);
    let mut b = Buf::new(len);
    // Roots from A2 to G#3, so every key sits in the same place.
    let root = 57 + (pc as i32 + 3) % 12;
    let notes = voicing(root, chord);
    for (k, &n) in notes.iter().enumerate() {
        let hz = midi_to_freq(n as f32);
        let mut x = keys_note(hz, len, 1.0, rng);
        let mut pad = pad_note(hz, len, 1600.0, rng);
        shape(&mut pad, 0.02, 1.2);
        for (i, v) in x.iter_mut().enumerate() {
            *v += pad[i] * 0.25 * exp_env(i as f32 / SR, 0.5);
        }
        let pan = (k as f32 / (notes.len() - 1) as f32 - 0.5) * 0.8;
        b.add(0, &x, 0.28, pan);
    }
    b.room(0.7, 0.25);
    b
}

fn pad(style: u8, rng: &mut Rng) -> Buf {
    let len = secs(5.0);
    let mut b = Buf::new(len);
    let chord = PADS[style as usize].1;
    let notes = voicing(60, chord);
    for (k, &n) in notes.iter().enumerate() {
        let hz = midi_to_freq(n as f32);
        let pan = (k as f32 / (notes.len() - 1) as f32 - 0.5) * 1.2;
        let mut x = match style {
            4 => {
                // Air: sines and their breath, high.
                let mut x = partials(hz * 2.0, len, &[1.0, 2.0], &[0.6, 0.2], &[30.0, 30.0], rng);
                let mut band = Svf::new();
                band.set_q(hz * 4.0, 6.0, SR);
                for v in x.iter_mut() {
                    *v += band.process(rng.bipolar(), FilterMode::Band) * 0.6;
                }
                x
            }
            5 => voice(hz, len, &AH, 0.1, 0.12, rng),
            _ => pad_note(hz, len, 1400.0 + 200.0 * k as f32, rng),
        };
        shape(&mut x, 0.7, 1.4);
        b.add(0, &x, 0.25, pan);
    }
    b.room(0.9, 0.4);
    b
}

fn flute_sound(style: u8, hz: f32, len: usize, rng: &mut Rng) -> Buf {
    let mut b = Buf::new(len);
    let mut x = match style {
        0 => flute(hz, len, 0.05, 0.18, 0.15, false, rng),
        1 => flute(hz, len, 0.18, 0.15, 0.25, false, rng),
        2 => flute(hz, len, 0.25, 0.1, 0.5, true, rng),
        3 => flute(hz, len, 0.07, 0.4, 0.15, false, rng),
        _ => flute(hz, len, 0.02, 0.1, 0.05, true, rng),
    };
    shape(&mut x, 0.01, 0.35);
    b.add(0, &x, 0.6, 0.0);
    b.room(0.7, 0.22);
    b
}

fn vox(style: u8, rng: &mut Rng) -> Buf {
    let chop = matches!(style, 5 | 6);
    let len = if chop { secs(0.9) } else { secs(2.2) };
    let mut b = Buf::new(len);
    let hz = midi_to_freq(if style == 4 { (VOX_NOTE - 19) as f32 } else { VOX_NOTE as f32 });
    let (vowel, vib) = match style {
        1 | 6 => (&OH, 0.2),
        2 => (&OOH, 0.2),
        3 => (&EE, 0.2),
        4 => (&AH_LOW, 0.15),
        _ => (&AH, 0.22),
    };
    if style == 7 {
        for (k, c) in [-14.0f32, -6.0, 0.0, 5.0, 11.0, 17.0].iter().enumerate() {
            let mut x = voice(hz * 2f32.powf(c / 1200.0), len, vowel, 0.15, 0.25, rng);
            shape(&mut x, 0.25, 0.5);
            b.add(0, &x, 0.2, -0.9 + 0.36 * k as f32);
        }
        b.room(0.85, 0.35);
    } else {
        let sung = if chop { secs(0.3) } else { len - secs(0.5) };
        let mut x = voice(hz, len, vowel, 0.08, vib, rng);
        for (i, v) in x.iter_mut().enumerate() {
            let rel = if i > sung { exp_env((i - sung) as f32 / SR, if chop { 0.05 } else { 0.15 }) } else { 1.0 };
            *v *= ((i as f32 / SR) / 0.04).min(1.0) * rel;
        }
        b.add(0, &x, 0.5, 0.0);
        b.room(0.65, if chop { 0.18 } else { 0.2 });
    }
    b
}

/// Analog Princess's 808, as the machine plays it.
fn bass(style: u8, rng: &mut Rng) -> Buf {
    let len = secs(if style == 5 { 4.0 } else if style == 6 { 1.0 } else { 2.5 });
    let mut b = Buf::new(len);
    let hz = midi_to_freq(BASS_NOTE as f32);
    let mut k = Knobs808 { drive: 0.25, dist: Dist::Soft, ring: 0.0, feed: 0.0, click: 0.35, decay: 1.4, glide: 0.12, ..Knobs808::default() };
    match style {
        1 => {
            k.drive = 0.8;
            k.dist = Dist::Hard;
        }
        2 => {
            k.drive = 0.6;
            k.dist = Dist::Fold;
            k.feed = 0.6;
            k.ring = 0.25;
        }
        3 => {
            k.drive = 0.5;
            k.dist = Dist::Hard;
            k.ring = 0.75;
        }
        4 => {
            k.drive = 0.7;
            k.dist = Dist::Hard;
        }
        5 => k.decay = 3.0,
        6 => {
            k.decay = 0.35;
            k.click = 0.7;
            k.drive = 0.4;
        }
        7 | 8 => {
            k.dist = Dist::Hard;
            k.drive = 0.5;
            k.glide = 0.25;
        }
        9 => {
            k.dist = Dist::Tube;
            k.drive = 0.6;
            k.sub = 0.4;
        }
        _ => {}
    }
    let mut v = Princess808::default();
    let start = match style {
        7 => midi_to_freq((BASS_NOTE + 7) as f32),
        8 => midi_to_freq((BASS_NOTE - 7) as f32),
        _ => hz,
    };
    v.trigger(start, 1.0, false, k, SR);
    let mut x = vec![0.0f32; len];
    for (i, s) in x.iter_mut().enumerate() {
        if (style == 7 || style == 8) && i == secs(0.28) {
            v.trigger(hz, 1.0, true, k, SR);
        }
        *s = v.tick(rng);
    }
    if style == 4 {
        bits(&mut x, 6.0, 4);
    }
    shape(&mut x, 0.0005, 0.08);
    b.add(0, &x, 0.9, 0.0);
    b
}

/// One of Analog Princess's own voices, as a sample.
fn princess_voice(dv: DrumVoice, len: usize, tone: f32, a: f32, bb: f32, rng: &mut Rng) -> Vec<f32> {
    let mut v = PrincessVoice::default();
    v.trigger(dv, 1.0, 1.0, None, false, 1.0, tone, a, bb, SR);
    (0..len).map(|_| v.tick(dv, rng)).collect()
}

fn sweep(len: usize, from: f32, to: f32, curve: f32, rng: &mut Rng) -> Vec<f32> {
    let mut ph = rng.unipolar();
    (0..len)
        .map(|i| {
            let x = (i as f32 / len as f32).powf(curve);
            let f = from * (to / from).powf(x);
            ph = (ph + f / SR).fract();
            (ph * TAU).sin()
        })
        .collect()
}

/// Six squares multiplied in pairs, as the machine's hats and cymbals.
fn metal(len: usize, rng: &mut Rng) -> Vec<f32> {
    const M: [f32; 6] = [349.0, 517.5, 628.3, 888.6, 918.0, 1360.0];
    let mut ph = [0.0f32; 6];
    let mut hp = Svf::new();
    hp.set(6000.0, 0.2, SR);
    (0..len)
        .map(|_| {
            let mut sq = [0.0f32; 6];
            for (k, p) in ph.iter_mut().enumerate() {
                *p = (*p + M[k] * 2.7 / SR).fract();
                sq[k] = if *p < 0.5 { 1.0 } else { -1.0 };
            }
            let m = sq[0] * sq[1] + sq[2] * sq[3] + sq[4] * sq[5];
            hp.process(m * 0.3 + rng.bipolar() * 0.7, FilterMode::High)
        })
        .collect()
}

/// A slice of `x` repeated: glitch tags.
fn stutter(x: &[f32], slices: &[(usize, usize, f32)]) -> Vec<f32> {
    let mut out = Vec::new();
    for &(len, times, pitch) in slices {
        for _ in 0..times {
            let n = len.min(x.len());
            for i in 0..n {
                let at = (i as f32 * pitch) as usize;
                let fade = (i.min(n - i) as f32 / 40.0).min(1.0);
                out.push(x.get(at).copied().unwrap_or(0.0) * fade);
            }
        }
    }
    out
}

fn fx(style: u8, rng: &mut Rng) -> Buf {
    let beat = 60.0 / 150.0;
    match style {
        0 => {
            let len = secs(0.6);
            let mut b = Buf::new(len);
            b.add(0, &princess_voice(DrumVoice::ApLaser, len, 0.5, 0.6, 0.4, rng), 0.8, 0.0);
            b.room(0.5, 0.15);
            b
        }
        1 | 2 => {
            let len = secs(0.5);
            let mut b = Buf::new(len);
            let mut x = if style == 1 { sweep(len, 3200.0, 70.0, 0.35, rng) } else { sweep(len, 70.0, 3200.0, 2.5, rng) };
            for (i, v) in x.iter_mut().enumerate() {
                *v *= if style == 1 { exp_env(i as f32 / SR, 0.18) } else { (i as f32 / len as f32).powf(0.5) };
            }
            bits(&mut x, 7.0, 3);
            shape(&mut x, 0.001, 0.02);
            b.add(0, &x, 0.6, 0.0);
            b.room(0.55, 0.2);
            b
        }
        3 => {
            let len = secs(0.25);
            let mut b = Buf::new(len);
            let mut x = fm_tone(1600.0, len, 0.51, 6.0, 0.03, 0.06, rng);
            bits(&mut x, 6.0, 2);
            b.add(0, &x, 0.6, 0.0);
            b
        }
        4 | 5 => {
            // Two bars at 150 BPM, up into the next one.
            let len = secs(8.0 * beat);
            let mut b = Buf::new(len);
            if style == 4 {
                let mut f = Svf::new();
                let x: Vec<f32> = (0..len)
                    .map(|i| {
                        let p = i as f32 / len as f32;
                        f.set(300.0 * 30f32.powf(p), 0.6, SR);
                        f.process(rng.bipolar(), FilterMode::Band) * p * p
                    })
                    .collect();
                b.add(0, &x, 0.9, 0.0);
            } else {
                for (k, n) in [0.0f32, 7.0, 12.0].iter().enumerate() {
                    let mut x = vec![0.0f32; len];
                    let mut ph = rng.unipolar();
                    for (i, v) in x.iter_mut().enumerate() {
                        let p = i as f32 / len as f32;
                        let f = midi_to_freq(48.0 + n + 12.0 * p * p);
                        ph = (ph + f / SR).fract();
                        *v = osc(Wave::Saw, ph, f / SR) * p;
                    }
                    let mut lp = Svf::new();
                    x.iter_mut().enumerate().for_each(|(i, v)| {
                        lp.set(400.0 + 5000.0 * (i as f32 / len as f32), 0.2, SR);
                        *v = lp.process(*v, FilterMode::Low)
                    });
                    b.add(0, &x, 0.25, [-0.5, 0.5, 0.0][k]);
                }
            }
            b.room(0.8, 0.25);
            b
        }
        6 | 7 => {
            let len = secs(if style == 6 { 2.0 } else { 4.0 });
            let mut b = Buf::new(len);
            let m = metal(len, rng);
            let d = if style == 6 { 0.5 } else { 1.1 };
            // A cymbal's decay, backwards: swelling into its end.
            let x: Vec<f32> = m.iter().enumerate().map(|(i, v)| v * exp_env((len - i) as f32 / SR, d)).collect();
            b.add(0, &x, 0.6, 0.0);
            let mut s = Buf::new(len);
            s.add(0, &x, 0.6, 0.0);
            s.room(0.8, 1.0);
            // The room backwards too (it comes before the swell's end).
            s.l.reverse();
            s.r.reverse();
            let (mut rl, mut rr) = (s.l.clone(), s.r.clone());
            rl.reverse();
            rr.reverse();
            for i in 0..len {
                b.l[i] += rl[i] * 0.3;
                b.r[i] += rr[i] * 0.3;
            }
            b
        }
        8 => {
            let len = secs(2.0);
            let mut b = Buf::new(len);
            let mut f = Svf::new();
            let x: Vec<f32> = (0..len)
                .map(|i| {
                    let p = i as f32 / len as f32;
                    f.set(9000.0 * 0.02f32.powf(p), 0.5, SR);
                    f.process(rng.bipolar(), FilterMode::Band) * (1.0 - p).powf(1.5)
                })
                .collect();
            b.add(0, &x, 0.9, 0.0);
            b.room(0.8, 0.3);
            b
        }
        9 => {
            let note = bell(0, midi_to_freq(76.0), secs(0.5), rng);
            let s = secs(beat * 0.25);
            let x = stutter(&note.l, &[(s, 2, 1.0), (s / 2, 4, 1.0), (s / 4, 4, 1.5), (s / 2, 2, 0.75)]);
            let mut b = Buf::new(x.len() + secs(0.6));
            b.add(0, &x, 0.8, 0.0);
            b.room(0.6, 0.25);
            b
        }
        10 => {
            let v = vox(5, rng);
            let s = secs(beat * 0.125);
            let mut x = stutter(&v.l, &[(s * 2, 1, 1.0), (s, 4, 1.0), (s / 2, 6, 1.0), (s, 2, 0.5)]);
            bits(&mut x, 8.0, 3);
            let mut b = Buf::new(x.len() + secs(0.5));
            b.add(0, &x, 0.8, 0.0);
            b.room(0.6, 0.2);
            b
        }
        11 => {
            let surf = princess_voice(DrumVoice::ApChannelSurf, secs(0.5), 0.6, 0.8, 0.6, rng);
            let laser = princess_voice(DrumVoice::ApLaser, secs(0.4), 0.5, 0.8, 0.8, rng);
            let mut b = Buf::new(secs(1.4));
            b.add(0, &surf, 0.6, -0.3);
            b.add(secs(0.45), &laser, 0.7, 0.3);
            b.room(0.6, 0.2);
            b
        }
        12 => {
            // A chord slowing to a stop.
            let src = stab(Chord::Maj9, 9, rng);
            let len = secs(1.6);
            let mut b = Buf::new(len);
            let mut pos = 0.0f64;
            let stop = secs(0.9) as f64;
            for i in 0..len {
                let rate = if (i as f64) < stop * 0.3 { 1.0 } else { (1.0 - (i as f64 - stop * 0.3) / (stop * 0.7)).max(0.0) };
                let k = pos as usize;
                let f = (pos - k as f64) as f32;
                let at = |v: &[f32]| v.get(k).copied().unwrap_or(0.0) * (1.0 - f) + v.get(k + 1).copied().unwrap_or(0.0) * f;
                let g = (rate as f32).sqrt();
                b.l[i] = at(&src.l) * g;
                b.r[i] = at(&src.r) * g;
                pos += rate;
            }
            b
        }
        13 => {
            let len = secs(1.2);
            let mut b = Buf::new(len);
            b.add(0, &princess_voice(DrumVoice::ApChannelSurf, len, 0.6, 0.6, 0.5, rng), 0.8, 0.0);
            b
        }
        14 => {
            let len = secs(3.0);
            let mut b = Buf::new(len);
            let boom = sweep(len, 120.0, 30.0, 0.3, rng);
            let x: Vec<f32> = boom.iter().enumerate().map(|(i, v)| fast_tanh(v * 2.0 * exp_env(i as f32 / SR, 0.6)) + rng.bipolar() * 0.4 * exp_env(i as f32 / SR, 0.05)).collect();
            b.add(0, &x, 0.8, 0.0);
            b.room(0.9, 0.3);
            b
        }
        _ => {
            // Four bars of a record's crackle and hiss at 150 BPM.
            let len = secs(16.0 * beat);
            let mut b = Buf::new(len);
            let mut lp = Svf::new();
            lp.set(5000.0, 0.0, SR);
            let mut crack = 0.0f32;
            for i in 0..len {
                if rng.next_u32() % 2200 == 0 {
                    crack = rng.bipolar();
                }
                crack *= 0.6;
                let hiss = lp.process(rng.bipolar(), FilterMode::Low) * 0.05;
                b.l[i] = hiss + crack * 0.8;
                b.r[i] = hiss * 0.9 + crack * 0.6;
            }
            b
        }
    }
}

/// A two- or four-bar melody over a progression, in its key and tempo.
fn melody_loop(k: usize, rng: &mut Rng) -> Buf {
    let d = LOOPS[k];
    let beat = 60.0 / d.bpm;
    let bar = secs(4.0 * beat);
    let len = ((d.bars as f32 * 4.0 * beat) * SR).round() as usize;
    let tail = secs(2.5);
    let mut b = Buf::new(len + tail);
    let prog = progression(d.prog, d.minor);
    let scale: &[i32] = if d.minor { &[0, 2, 3, 5, 7, 8, 10] } else { &[0, 2, 4, 5, 7, 9, 11] };
    let root = 57 + (d.root + 3) % 12;
    // The chords, a bar each (a two-bar loop takes the first two).
    for bi in 0..d.bars {
        let (deg, chord) = prog[bi % 4];
        let croot = root + deg - if deg > 6 { 12 } else { 0 };
        let notes = voicing(croot, chord);
        let at = bi * bar;
        let dur = bar + secs(0.25);
        for (j, &n) in notes.iter().enumerate() {
            let hz = midi_to_freq(n as f32);
            let pan = (j as f32 / (notes.len() - 1) as f32 - 0.5) * 0.9;
            let mut x = match d.keys {
                0 => pad_note(hz, dur + secs(0.6), 1300.0, rng),
                1 => keys_note(hz, dur + secs(0.6), 1.4, rng),
                _ => fm_tone(hz, dur + secs(0.6), 1.0, 0.4, 0.8, 2.0, rng),
            };
            shape(&mut x, if d.keys == 0 { 0.12 } else { 0.004 }, 0.6);
            b.add(at, &x, if d.keys == 0 { 0.12 } else { 0.16 }, pan);
        }
    }
    // The melody: chord tones on a sparse, half-time rhythm, stepping
    // between them by the scale, resolving at the end.
    const RHYTHMS: [[u8; 8]; 6] = [[1, 0, 0, 1, 0, 0, 1, 0], [1, 0, 1, 0, 0, 0, 1, 1], [0, 0, 1, 0, 1, 0, 0, 1], [1, 0, 0, 0, 1, 0, 1, 0], [1, 1, 0, 1, 0, 0, 0, 0], [0, 1, 0, 1, 0, 1, 1, 0]];
    let mut last = root + 12 + 7;
    let mut melody: Vec<(usize, usize, i32)> = Vec::new();
    for bi in 0..d.bars {
        let (deg, chord) = prog[bi % 4];
        let tones: Vec<i32> = chord.notes().iter().map(|n| (root + deg + n).rem_euclid(12)).collect();
        let rhythm = RHYTHMS[(rng.next_u32() as usize + k) % RHYTHMS.len()];
        let slots: Vec<usize> = (0..8).filter(|&s| rhythm[s] == 1).collect();
        for (si, &s) in slots.iter().enumerate() {
            let at = bi * bar + s * bar / 8;
            let next = slots.get(si + 1).map_or(bar, |&n| n * bar / 8) + bi * bar;
            let mut dur = (next - at).min(bar / 2);
            let final_note = bi + 1 == d.bars && si + 1 == slots.len();
            // Near the last note, a chord tone mostly, a scale step now and then.
            let pick_chord = rng.unipolar() < 0.75 || final_note;
            let cands: Vec<i32> = (root + 5..root + 32)
                .filter(|n| {
                    let pc = n.rem_euclid(12);
                    if pick_chord {
                        tones.contains(&pc)
                    } else {
                        scale.contains(&(n - root).rem_euclid(12))
                    }
                })
                .collect();
            let mut best = cands.iter().copied().min_by_key(|n| ((n - last).abs() * 10) + (rng.next_u32() % 25) as i32).unwrap_or(last);
            if final_note {
                best = cands.iter().copied().filter(|n| (n - root).rem_euclid(12) == 0 || (n - root).rem_euclid(12) == 7).min_by_key(|n| (n - last).abs()).unwrap_or(best);
                dur = bar / 2;
            }
            melody.push((at, dur, best));
            last = best;
        }
    }
    for (at, dur, n) in melody {
        let hz = midi_to_freq(n as f32);
        let ring = dur + secs(0.8);
        let mut x = match d.lead {
            0 => fm_tone(hz, ring, 2.0, 1.6, 0.25, 0.9, rng),
            1 => {
                let mut x = string(hz, ring, 0.5, 1.0, rng);
                x.iter_mut().for_each(|v| *v *= 0.8);
                x
            }
            2 => flute(hz, ring, 0.06, 0.15, 0.12, false, rng),
            _ => {
                let mut ph = 0.0f32;
                (0..ring)
                    .map(|i| {
                        let t = i as f32 / SR;
                        let f = hz * 2f32.powf(vibrato(t, 0.2, 5.5, 0.12) / 12.0);
                        ph = (ph + f / SR).fract();
                        (ph * TAU).sin()
                    })
                    .collect()
            }
        };
        // Held to its length, then let go.
        for (i, v) in x.iter_mut().enumerate() {
            let rel = if i > dur { exp_env((i - dur) as f32 / SR, 0.12) } else { 1.0 };
            *v *= ((i as f32 / SR) / 0.004).min(1.0) * rel;
        }
        b.add(at, &x, 0.32, 0.1);
    }
    b.room(0.75, 0.25);
    // The tail, round into the start: it loops without a seam.
    let mut out = Buf::new(len);
    for i in 0..len + tail {
        out.l[i % len] += b.l[i];
        out.r[i % len] += b.r[i];
    }
    out
}

/// Brings a sound to its loudness: its loudest 300 ms to -15 dBFS, the
/// peak not over -1 dBFS; silence at both ends (2 ms in, 2 ms out).
fn finish(mut b: Buf) -> (Vec<f32>, Vec<f32>) {
    let n = b.len();
    for (l, r) in b.l.iter_mut().zip(b.r.iter_mut()) {
        if !l.is_finite() || !r.is_finite() {
            *l = 0.0;
            *r = 0.0;
        }
    }
    // Its offset out (a one-pole high pass at a few Hz).
    for ch in [&mut b.l, &mut b.r] {
        let mut lp = 0.0f32;
        let a = 1.0 - (-TAU * 5.0 / SR).exp();
        for v in ch.iter_mut() {
            lp += (*v - lp) * a;
            *v -= lp;
        }
    }
    let fade = secs(0.002).max(2);
    for ch in [&mut b.l, &mut b.r] {
        shape(ch, 0.002, 0.002);
        if n > 2 * fade {
            ch[0] = 0.0;
            ch[n - 1] = 0.0;
        }
    }
    let w = secs(0.3).min(n.max(1));
    let mut sum = 0.0f64;
    let mut loudest = 0.0f64;
    let e = |i: usize| (b.l[i] as f64).powi(2) + (b.r[i] as f64).powi(2);
    for i in 0..n {
        sum += e(i);
        if i >= w {
            sum -= e(i - w);
        }
        loudest = loudest.max(sum);
    }
    let rms = (loudest / (2.0 * w as f64)).sqrt() as f32;
    let peak = b.l.iter().chain(&b.r).fold(0.0f32, |m, v| m.max(v.abs()));
    if rms > 1e-6 && peak > 1e-6 {
        let g = (crate::dsp::db_to_gain(-15.0) / rms).min(crate::dsp::db_to_gain(-1.0) / peak);
        b.l.iter_mut().chain(b.r.iter_mut()).for_each(|v| *v *= g);
    }
    (b.l, b.r)
}

/// Makes a sound (44.1 kHz stereo): the same every time.
pub fn render(s: &Sound) -> (Vec<f32>, Vec<f32>) {
    let seed = s.name.bytes().fold(0x5EED_1234u32, |h, c| h.rotate_left(5) ^ c as u32).max(1);
    let mut rng = Rng::new(seed);
    let rng = &mut rng;
    let buf = match s.kind {
        Kind::Bell(k) => bell(k, midi_to_freq(BELL_NOTE as f32), secs(if k == 4 { 1.6 } else { 3.2 }), rng),
        Kind::Pluck(k) => pluck(k, midi_to_freq(PLUCK_NOTE as f32), secs(if k == 9 { 1.0 } else { 2.2 }), rng),
        Kind::Lead(k) => lead(k, midi_to_freq(BELL_NOTE as f32), secs(2.4), rng),
        Kind::Stab(c, pc) => stab(c, pc, rng),
        Kind::Pad(k) => pad(k, rng),
        Kind::Flute(k) => flute_sound(k, midi_to_freq(BELL_NOTE as f32), secs(2.4), rng),
        Kind::Vox(k) => vox(k, rng),
        Kind::Bass(k) => bass(k, rng),
        Kind::Fx(k) => fx(k, rng),
        Kind::Loop(k) => melody_loop(k as usize, rng),
    };
    finish(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(s: &Sound, l: &[f32], r: &[f32]) {
        let n = l.len();
        assert!(n > secs(0.1) && n == r.len(), "{}: {n} frames", s.name);
        assert!(l.iter().chain(r).all(|v| v.is_finite()), "{}", s.name);
        // From silence and back to it, with no step on the way.
        for ch in [l, r] {
            assert!(ch[0].abs() < 1e-3 && ch[n - 1].abs() < 1e-3, "{}: {} .. {}", s.name, ch[0], ch[n - 1]);
            // Faded in and out: the first and last samples rise from nothing
            // (a step would start at full level).
            let worst = ch[..4].iter().chain(&ch[n - 4..]).fold(0.0f32, |m, v| m.max(v.abs()));
            assert!(worst < 0.01, "{}: a step of {worst} at an end", s.name);
            let first_ms = ch[..secs(0.001)].iter().fold(0.0f32, |m, v| m.max(v.abs()));
            assert!(first_ms < 0.5, "{}: {first_ms} in the first millisecond", s.name);
        }
        let peak = l.iter().chain(r).fold(0.0f32, |m, v| m.max(v.abs()));
        assert!(peak <= crate::dsp::db_to_gain(-1.0) + 1e-4, "{}: peak {peak}", s.name);
        assert!(peak > crate::dsp::db_to_gain(-24.0), "{}: too quiet ({peak})", s.name);
    }

    #[test]
    fn a_library_of_its_own() {
        let all = sounds();
        assert!(all.len() >= 80 && all.len() <= 150, "{} sounds", all.len());
        let mut names: Vec<String> = all.iter().map(|s| format!("{}/{}", s.category, s.name)).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), all.len(), "every name once");
        // The loops carry their key and tempo in their names.
        for s in all.iter().filter(|s| matches!(s.kind, Kind::Loop(_))) {
            assert!(crate::analysis::bpm_in_name(&s.name).is_some_and(|b| (140.0..=160.0).contains(&b)), "{}", s.name);
            assert!(crate::analysis::key_in_name(&s.name).is_some(), "{}", s.name);
        }
        // Twelve keys of each stab.
        assert_eq!(all.iter().filter(|s| matches!(s.kind, Kind::Stab(Chord::Maj9, _))).count(), 12);
    }

    #[test]
    fn sounds_are_click_free_loud_alike_and_the_same_every_time() {
        // One of each kind (all of them take a while in a test build).
        let all = sounds();
        let mut seen = Vec::new();
        for s in &all {
            let kind = std::mem::discriminant(&s.kind);
            if seen.contains(&kind) {
                continue;
            }
            seen.push(kind);
            let (l, r) = render(s);
            check(s, &l, &r);
            let (l2, _) = render(s);
            assert!(l == l2, "{}: made the same again", s.name);
        }
        // And every FX and 808.
        for s in all.iter().filter(|s| matches!(s.kind, Kind::Fx(_) | Kind::Bass(_))) {
            let (l, r) = render(s);
            check(s, &l, &r);
        }
    }

    #[test]
    fn a_loop_is_as_long_as_its_bars_and_loops_without_a_seam() {
        for k in [0usize, 3] {
            let s = sounds().into_iter().find(|s| s.kind == Kind::Loop(k as u8)).unwrap();
            let (l, _) = render(&s);
            let d = LOOPS[k];
            let want = (d.bars as f32 * 4.0 * 60.0 / d.bpm * SR).round() as usize;
            assert_eq!(l.len(), want, "{}", s.name);
            // Played round, the end runs into the start.
            assert!((l[l.len() - 1] - l[0]).abs() < 1e-3);
            // A melody: the key in its name is the one heard.
            let data = crate::sample::SampleData::from_buffers(s.name.clone(), None, RATE, l.clone(), l.clone());
            let heard = crate::analysis::detect_key(&data).expect("a key");
            let named = crate::analysis::key_in_name(&s.name).unwrap();
            let same = heard.root == named.root && heard.minor == named.minor;
            let relative = heard.minor != named.minor && (heard.root as i32 - named.root as i32).rem_euclid(12) == if named.minor { 3 } else { 9 };
            assert!(same || relative, "{}: heard {}", s.name, heard.name());
        }
    }

    #[test]
    fn the_library_is_made_once_into_its_folder() {
        let dir = std::env::temp_dir().join(format!("krabton_princess_sounds_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let s = sounds().into_iter().find(|s| s.kind == Kind::Bass(0)).unwrap();
        let path = ensure(&dir, &s).unwrap();
        assert!(path.is_file() && path.starts_with(dir.join("808s")));
        let back = crate::sample::SampleData::load(&path).unwrap();
        assert_eq!(back.sample_rate, RATE);
        let (l, _) = render(&s);
        assert_eq!(back.frames(), l.len());
        assert!((back.left[l.len() / 3] - l[l.len() / 3]).abs() < 1e-3);
        assert!(!installed(&dir));
        let _ = std::fs::remove_dir_all(&dir);
    }
}

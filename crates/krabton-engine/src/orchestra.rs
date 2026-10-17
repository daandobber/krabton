//! Orchestra: the instruments of the VS Chamber Orchestra 2, Community
//! Edition, by Versilian Studios (recorded by Sam Gossner and Simon Dalzell,
//! cut by Elan Hickler; CC0, <https://github.com/sgossner/VSCO-2-CE>):
//! solo violin, violin, viola and cello sections, double bass, harp, the
//! woodwinds and the brass, timpani, mallets, an organ and an upright
//! piano, each in its articulations (sustained, plain, short, pizzicato,
//! tremolo, muted). Each instrument is downloaded the first time it is used
//! and kept as 16-bit frames in one memory-mapped file, as the grand piano is.
//!
//! The library's own SFZ files say which recording plays which keys and
//! velocities. On top of them: Dynamics, moved while a note sounds, fades
//! between the recorded dynamic layers (as a mod wheel does in orchestral
//! libraries) and the loudness with them; sustained notes loop smoothly
//! where the recording would end; Legato joins notes, the new one taking
//! over without its attack; round robins take turns; and a little Life
//! keeps repeated notes from being identical.

use crate::params::{ParamSpec, Unit, SCREEN_ONLY};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Where the samples come from: the SFZ branch, pinned.
pub const BASE_URL: &str = "https://raw.githubusercontent.com/sgossner/VSCO-2-CE/6dd651d55dde97fd4028699be9d4481f26917891/";
pub const FOLDER: &str = "VS Chamber Orchestra 2";
pub const CREDIT: &str = "VS Chamber Orchestra 2: Community Edition by Versilian Studios (recorded by Sam Gossner and Simon Dalzell, sample cutting by Elan Hickler/Soundemote), CC0 1.0: https://github.com/sgossner/VSCO-2-CE";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Family {
    Strings,
    Woodwinds,
    Brass,
    Percussion,
    Keys,
}

/// One articulation: what it is called, its SFZ file, and whether its
/// notes are held (and so may loop).
#[derive(Clone, Copy, Debug)]
pub struct Art {
    pub label: &'static str,
    pub sfz: &'static str,
    pub held: bool,
}

const fn held(label: &'static str, sfz: &'static str) -> Art {
    Art { label, sfz, held: true }
}

const fn short(label: &'static str, sfz: &'static str) -> Art {
    Art { label, sfz, held: false }
}

#[derive(Clone, Copy, Debug)]
pub struct Def {
    pub name: &'static str,
    pub family: Family,
    pub arts: &'static [Art],
    /// About how much it downloads (MB).
    pub mb: u32,
}

pub const INSTRUMENTS: &[Def] = &[
    Def { name: "Solo Violin", family: Family::Strings, mb: 136, arts: &[held("Sustain", "SViolinVib.sfz"), short("Spiccato", "SViolinSpic.sfz"), short("Pizzicato", "SViolinPizz.sfz"), held("Tremolo", "SViolinTrem.sfz")] },
    Def { name: "Violins", family: Family::Strings, mb: 100, arts: &[held("Sustain", "ViolinEnsSusVib.sfz"), short("Spiccato", "ViolinEnsSpic.sfz"), short("Pizzicato", "ViolinEnsPizz.sfz"), held("Tremolo", "ViolinEnsTrem.sfz")] },
    Def { name: "Violas", family: Family::Strings, mb: 143, arts: &[held("Sustain", "ViolaEnsSusVib.sfz"), short("Spiccato", "ViolaEnsSpic.sfz"), short("Pizzicato", "ViolaEnsPizz.sfz"), held("Tremolo", "ViolaEnsTrem.sfz")] },
    Def { name: "Cellos", family: Family::Strings, mb: 186, arts: &[held("Sustain", "CelloEnsSusVib.sfz"), short("Spiccato", "CelloEnsSpic.sfz"), short("Pizzicato", "CelloEnsPizz.sfz"), held("Tremolo", "CelloEnsTrem.sfz")] },
    Def { name: "Double Bass", family: Family::Strings, mb: 156, arts: &[held("Sustain", "ContrabassSusVB.sfz"), held("Plain", "ContrabassSusNV.sfz"), short("Spiccato", "ContrabassSpic.sfz"), short("Pizzicato", "ContrabassPizz.sfz"), held("Tremolo", "ContrabassTrem.sfz")] },
    Def { name: "Harp", family: Family::Strings, mb: 35, arts: &[short("Harp", "Harp.sfz")] },
    Def { name: "Flute", family: Family::Woodwinds, mb: 121, arts: &[held("Sustain", "FluteSusVib.sfz"), held("Plain", "FluteSusNV.sfz"), held("Expressive", "FluteExpVib.sfz"), short("Staccato", "FluteStac.sfz")] },
    Def { name: "Piccolo", family: Family::Woodwinds, mb: 13, arts: &[held("Sustain", "PiccoloSus.sfz"), short("Staccato", "PiccoloStac.sfz")] },
    Def { name: "Oboe", family: Family::Woodwinds, mb: 66, arts: &[held("Sustain", "OboeSusVib.sfz"), held("Plain", "OboeSusNV.sfz"), short("Staccato", "OboeStac.sfz")] },
    Def { name: "Clarinet", family: Family::Woodwinds, mb: 66, arts: &[held("Sustain", "ClarinetSus.sfz"), short("Staccato", "ClarinetStac.sfz")] },
    Def { name: "Bassoon", family: Family::Woodwinds, mb: 69, arts: &[held("Sustain", "BassoonVib.sfz"), held("Plain", "BassoonSus.sfz"), short("Staccato", "BassoonStac.sfz")] },
    Def { name: "French Horn", family: Family::Brass, mb: 84, arts: &[held("Sustain", "FHornSus.sfz"), short("Staccato", "FHornStac.sfz"), held("Muted", "FHornMute.sfz")] },
    Def { name: "Trumpet", family: Family::Brass, mb: 120, arts: &[held("Sustain", "TrumpetSusVib.sfz"), held("Plain", "TrumpetSus.sfz"), short("Staccato", "TrumpetStac.sfz"), held("Straight Mute", "TrumpetStraightMuteSus.sfz"), held("Harmon Mute", "TrumpetHarmonMuteSus.sfz")] },
    Def { name: "Trombone", family: Family::Brass, mb: 86, arts: &[held("Sustain", "TromboneSus.sfz"), held("Vibrato", "TromboneVib.sfz"), short("Staccato", "TromboneStac.sfz")] },
    Def { name: "Tuba", family: Family::Brass, mb: 41, arts: &[held("Sustain", "TubaSus.sfz"), short("Staccato", "TubaStac.sfz")] },
    Def { name: "Timpani", family: Family::Percussion, mb: 76, arts: &[short("Hits", "Timpani.sfz"), held("Rolls", "TimpaniRolls.sfz")] },
    Def { name: "Glockenspiel", family: Family::Percussion, mb: 7, arts: &[short("Glockenspiel", "Glockenspiel.sfz")] },
    Def { name: "Xylophone", family: Family::Percussion, mb: 5, arts: &[short("Xylophone", "Xylophone.sfz")] },
    Def { name: "Marimba", family: Family::Percussion, mb: 12, arts: &[short("Marimba", "Marimba.sfz")] },
    Def { name: "Tubular Bells", family: Family::Percussion, mb: 30, arts: &[short("Tubular Bells", "TubularBells.sfz")] },
    Def { name: "Orchestral Percussion", family: Family::Percussion, mb: 420, arts: &[short("Percussion", "GM-StylePerc.sfz")] },
    Def { name: "Church Organ", family: Family::Keys, mb: 145, arts: &[held("Full", "OrganLoud.sfz"), held("Soft", "OrganQuiet.sfz")] },
    Def { name: "Upright Piano", family: Family::Keys, mb: 160, arts: &[short("Upright", "VSUpright1.sfz")] },
];

pub fn index_of(name: &str) -> Option<usize> {
    INSTRUMENTS.iter().position(|d| d.name == name)
}

// ---------------------------------------------------------------- parameters

pub const INSTRUMENT: usize = 0;
pub const ARTICULATION: usize = 1;
pub const DYNAMICS: usize = 2;
pub const VELOCITY: usize = 3;
pub const ATTACK: usize = 4;
pub const RELEASE: usize = 5;
pub const LEGATO: usize = 6;
pub const LEGATO_TIME: usize = 7;
pub const TRANSPOSE: usize = 8;
pub const TUNE: usize = 9;
pub const LIFE: usize = 10;
pub const WIDTH: usize = 11;
pub const PAN: usize = 12;
pub const VOLUME: usize = 13;

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::int("Instrument", SCREEN_ONLY, 0.0, 63.0, 0.0, Unit::None),
    ParamSpec::int("Articulation", SCREEN_ONLY, 0.0, 7.0, 0.0, Unit::None),
    ParamSpec::lin("Dynamics", "Play", 0.0, 1.0, 0.7, Unit::Percent),
    ParamSpec::lin("Velocity", "Play", 0.0, 1.0, 0.6, Unit::Percent),
    ParamSpec::log("Attack", "Play", 1.0, 2000.0, 1.0, Unit::Ms),
    ParamSpec::lin("Release", "Play", 0.1, 3.0, 1.0, Unit::Ratio),
    ParamSpec::toggle("Legato", "Legato", false),
    ParamSpec::log("Legato Time", "Legato", 20.0, 500.0, 80.0, Unit::Ms),
    ParamSpec::int("Transpose", "Tuning", -24.0, 24.0, 0.0, Unit::Semi),
    ParamSpec::lin("Tune", "Tuning", -100.0, 100.0, 0.0, Unit::Cents),
    ParamSpec::lin("Life", "Tuning", 0.0, 1.0, 0.25, Unit::Percent),
    ParamSpec::lin("Width", "Output", 0.0, 1.5, 1.0, Unit::Percent),
    ParamSpec::lin("Pan", "Output", -1.0, 1.0, 0.0, Unit::Pan),
    ParamSpec::lin("Volume", "Output", -24.0, 6.0, 0.0, Unit::Db),
];

/// The instruments' names, in the order of `INSTRUMENTS`.
pub const NAMES: &[&str] = &["Solo Violin", "Violins", "Violas", "Cellos", "Double Bass", "Harp", "Flute", "Piccolo", "Oboe", "Clarinet", "Bassoon", "French Horn", "Trumpet", "Trombone", "Tuba", "Timpani", "Glockenspiel", "Xylophone", "Marimba", "Tubular Bells", "Orchestral Percussion", "Church Organ", "Upright Piano"];

pub fn preset_names() -> &'static [&'static str] {
    NAMES
}

pub fn preset(name: &str) -> Vec<f32> {
    let mut p = crate::params::defaults(SPECS);
    let i = index_of(name).unwrap_or(0);
    p[INSTRUMENT] = i as f32;
    let def = &INSTRUMENTS[i];
    // A little more space for sections and organ, legato for the soloists.
    match def.family {
        Family::Strings | Family::Woodwinds | Family::Brass if def.arts[0].held => {
            p[ATTACK] = 8.0;
        }
        _ => {}
    }
    if matches!(def.name, "Solo Violin" | "Flute" | "Oboe" | "Clarinet" | "Trumpet") {
        p[LEGATO] = 1.0;
    }
    p
}

// ---------------------------------------------------------------- SFZ

/// A region of an SFZ file, with what it inherits.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Region {
    pub sample: String,
    pub lokey: u8,
    pub hikey: u8,
    pub keycenter: u8,
    pub lovel: u8,
    pub hivel: u8,
    pub volume_db: f32,
    pub tune: f32,
    pub seq_len: u8,
    pub seq_pos: u8,
    pub lorand: f32,
    pub hirand: f32,
    pub attack: f32,
    pub release: f32,
}

impl Default for Region {
    fn default() -> Self {
        Self { sample: String::new(), lokey: 0, hikey: 127, keycenter: 60, lovel: 0, hivel: 127, volume_db: 0.0, tune: 0.0, seq_len: 1, seq_pos: 1, lorand: 0.0, hirand: 1.0, attack: 0.001, release: 0.3 }
    }
}

fn key(v: &str) -> Option<u8> {
    v.trim().parse::<i32>().ok().map(|k| k.clamp(0, 127) as u8).or_else(|| crate::sample_map::parse_note(v))
}

fn apply(r: &mut Region, op: &str, v: &str, default_path: &str) {
    let f = || v.trim().parse::<f32>().ok();
    match op {
        "sample" => r.sample = format!("{default_path}{}", v.trim()).replace('\\', "/"),
        "lokey" => r.lokey = key(v).unwrap_or(r.lokey),
        "hikey" => r.hikey = key(v).unwrap_or(r.hikey),
        "key" => {
            if let Some(k) = key(v) {
                r.lokey = k;
                r.hikey = k;
                r.keycenter = k;
            }
        }
        "pitch_keycenter" => r.keycenter = key(v).unwrap_or(r.keycenter),
        "lovel" => r.lovel = f().map(|x| x.clamp(0.0, 127.0) as u8).unwrap_or(r.lovel),
        "hivel" => r.hivel = f().map(|x| x.clamp(0.0, 127.0) as u8).unwrap_or(r.hivel),
        "volume" => r.volume_db = f().unwrap_or(r.volume_db),
        "tune" => r.tune = f().unwrap_or(r.tune),
        "seq_length" => r.seq_len = f().map(|x| x.clamp(1.0, 16.0) as u8).unwrap_or(1),
        "seq_position" => r.seq_pos = f().map(|x| x.clamp(1.0, 16.0) as u8).unwrap_or(1),
        "lorand" => r.lorand = f().unwrap_or(0.0),
        "hirand" => r.hirand = f().unwrap_or(1.0),
        "ampeg_attack" => r.attack = f().unwrap_or(r.attack),
        "ampeg_release" => r.release = f().unwrap_or(r.release),
        _ => {}
    }
}

/// The regions of an SFZ file (the opcodes the library uses; others are
/// passed over).
pub fn parse_sfz(text: &str) -> Vec<Region> {
    let mut out = Vec::new();
    let mut default_path = String::new();
    let mut global = Region::default();
    let mut group = Region::default();
    let mut region: Option<Region> = None;
    let mut section = "";
    let finish = |region: &mut Option<Region>, out: &mut Vec<Region>| {
        if let Some(r) = region.take() {
            if !r.sample.is_empty() {
                out.push(r);
            }
        }
    };
    for raw in text.lines() {
        let line = match raw.find("//") {
            Some(i) => &raw[..i],
            None => raw,
        };
        let mut rest = line.trim();
        while !rest.is_empty() {
            if rest.starts_with('<') {
                let end = rest.find('>').map(|i| i + 1).unwrap_or(rest.len());
                let header = &rest[1..end.saturating_sub(1)];
                rest = rest[end..].trim_start();
                finish(&mut region, &mut out);
                match header {
                    "control" => section = "control",
                    "global" => {
                        section = "global";
                        global = Region::default();
                    }
                    "group" | "master" => {
                        section = "group";
                        group = global.clone();
                    }
                    "region" => {
                        section = "region";
                        region = Some(group.clone());
                    }
                    _ => section = "",
                }
                continue;
            }
            // opcode=value, the value up to the next opcode (sample paths may hold spaces).
            let Some(eq) = rest.find('=') else { break };
            let op = rest[..eq].trim();
            let after = &rest[eq + 1..];
            let mut cut = after.len();
            for (i, _) in after.match_indices(' ') {
                let tail = after[i + 1..].trim_start();
                let looks = tail.starts_with('<') || tail.split_once('=').is_some_and(|(o, _)| !o.is_empty() && o.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'));
                if looks {
                    cut = i;
                    break;
                }
            }
            let value = after[..cut].trim();
            rest = after[cut..].trim_start();
            match section {
                "control" if op == "default_path" => default_path = value.replace('\\', "/"),
                "global" => apply(&mut global, op, value, &default_path),
                "group" => apply(&mut group, op, value, &default_path),
                "region" => {
                    if let Some(r) = region.as_mut() {
                        apply(r, op, value, &default_path);
                    }
                }
                _ => {}
            }
        }
    }
    finish(&mut region, &mut out);
    out
}

/// The URL of a file in the library (its path escaped).
pub fn url(path: &str) -> String {
    let mut s = String::from(BASE_URL);
    for c in path.chars() {
        match c {
            ' ' => s.push_str("%20"),
            '#' => s.push_str("%23"),
            c => s.push(c),
        }
    }
    s
}

// ---------------------------------------------------------------- library

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SampleEntry {
    pub offset: u64,
    pub frames: u32,
    pub rate: u32,
    /// Where a held note loops (frames), with its crossfade.
    pub loop_start: u32,
    pub loop_end: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ArtEntry {
    pub label: String,
    pub held: bool,
    pub regions: Vec<(Region, u32)>,
}

#[derive(Serialize, Deserialize)]
struct Index {
    version: u32,
    instrument: String,
    arts: Vec<ArtEntry>,
    samples: Vec<SampleEntry>,
}

pub struct Library {
    map: memmap2::Mmap,
    pub instrument: String,
    pub arts: Vec<ArtEntry>,
    pub samples: Vec<SampleEntry>,
    prefetch: crossbeam_channel::Sender<u32>,
}

impl Library {
    pub fn open(dir: &Path) -> Result<Arc<Self>, String> {
        let index: Index = serde_json::from_slice(&std::fs::read(dir.join("index.json")).map_err(|e| format!("{}: {e}", dir.display()))?).map_err(|e| e.to_string())?;
        let file = File::open(dir.join("samples.bin")).map_err(|e| e.to_string())?;
        // SAFETY: the file is Krabton's own and is not changed while mapped.
        let map = unsafe { memmap2::Mmap::map(&file) }.map_err(|e| e.to_string())?;
        let total = map.len() as u64 / 4;
        if index.samples.iter().any(|s| s.offset + s.frames as u64 > total) {
            return Err(format!("{}: samples.bin is too short", dir.display()));
        }
        // A helper pages in what notes will play, so the audio thread finds it in memory.
        let (tx, rx) = crossbeam_channel::bounded::<u32>(256);
        let spans: Vec<(u64, u32)> = index.samples.iter().map(|s| (s.offset, s.frames)).collect();
        let path = dir.join("samples.bin");
        std::thread::Builder::new()
            .name("orchestra read-ahead".into())
            .spawn(move || {
                let Ok(file) = File::open(&path) else { return };
                // SAFETY: as above.
                let Ok(map) = (unsafe { memmap2::Mmap::map(&file) }) else { return };
                let touch = |from: u64, frames: u64| {
                    let (a, b) = ((from * 4) as usize, ((from + frames) * 4).min(map.len() as u64) as usize);
                    let mut sum = 0u8;
                    let mut i = a;
                    while i < b {
                        // SAFETY: inside the mapping.
                        sum = sum.wrapping_add(unsafe { std::ptr::read_volatile(map.as_ptr().add(i)) });
                        i += 4096;
                    }
                    std::hint::black_box(sum);
                };
                for &(off, frames) in &spans {
                    touch(off, (frames as u64).min(24000));
                }
                while let Ok(s) = rx.recv() {
                    if let Some(&(off, frames)) = spans.get(s as usize) {
                        touch(off, frames as u64);
                    }
                }
            })
            .map_err(|e| e.to_string())?;
        Ok(Arc::new(Self { map, instrument: index.instrument, arts: index.arts, samples: index.samples, prefetch: tx }))
    }

    /// The interleaved stereo frames of sample `s`.
    pub fn frames(&self, s: u32) -> &[i16] {
        let Some(e) = self.samples.get(s as usize) else { return &[] };
        let bytes = &self.map[(e.offset * 4) as usize..((e.offset + e.frames as u64) * 4) as usize];
        // SAFETY: the map is page aligned and every sample starts on a frame (4 bytes).
        unsafe { std::slice::from_raw_parts(bytes.as_ptr() as *const i16, bytes.len() / 2) }
    }

    pub fn prefetch(&self, s: u32) {
        let _ = self.prefetch.try_send(s);
    }

    /// The keys an articulation plays.
    pub fn range(&self, art: usize) -> (u8, u8) {
        let Some(a) = self.arts.get(art) else { return (0, 0) };
        (a.regions.iter().map(|r| r.0.lokey).min().unwrap_or(0), a.regions.iter().map(|r| r.0.hikey).max().unwrap_or(0))
    }
}

/// Whether `dir` holds a built instrument.
pub fn installed(dir: &Path) -> bool {
    dir.join("index.json").is_file() && dir.join("samples.bin").is_file()
}

/// Decodes a WAV into 16-bit stereo frames and its rate.
fn decode(path: &Path) -> Result<(Vec<i16>, u32), String> {
    let mut r = hound::WavReader::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let spec = r.spec();
    let ch = spec.channels.max(1) as usize;
    let samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => r.samples::<f32>().map(|s| s.unwrap_or(0.0)).collect(),
        hound::SampleFormat::Int => {
            let scale = 1.0 / (1i64 << (spec.bits_per_sample.max(1) - 1)) as f32;
            r.samples::<i32>().map(|s| s.unwrap_or(0) as f32 * scale).collect()
        }
    };
    let frames = samples.len() / ch;
    let mut out = Vec::with_capacity(frames * 2);
    let mut seed = 0x1234_5678u32;
    let mut dither = || {
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (seed >> 16) as f32 / 65536.0 - 0.5
    };
    for f in 0..frames {
        let l = samples[f * ch];
        let r = if ch > 1 { samples[f * ch + 1] } else { l };
        out.push((l * 32767.0 + dither()).round().clamp(-32768.0, 32767.0) as i16);
        out.push((r * 32767.0 + dither()).round().clamp(-32768.0, 32767.0) as i16);
    }
    Ok((out, spec.sample_rate))
}

/// Where a held note loops: the steady part of the recording, between
/// points that match (a crossfade hides the join).
pub fn find_loop(frames: &[i16], rate: u32) -> (u32, u32) {
    let n = frames.len() / 2;
    if n < rate as usize {
        return (0, 0);
    }
    let mono = |i: usize| (frames[2 * i] as f32 + frames[2 * i + 1] as f32) * 0.5;
    // Where it stops being loud: the tail is not looped.
    let block = (rate / 50) as usize;
    let rms: Vec<f32> = (0..n / block).map(|b| ((0..block).map(|k| mono(b * block + k).powi(2)).sum::<f32>() / block as f32).sqrt()).collect();
    let peak = rms.iter().copied().fold(0.0, f32::max);
    let last_loud = rms.iter().rposition(|&r| r > peak * 0.35).unwrap_or(rms.len() / 2);
    let end_guess = ((last_loud * block) as f32 * 0.92) as usize;
    let start_guess = ((n as f32 * 0.3) as usize).max((rate as f32 * 0.4) as usize);
    if end_guess <= start_guess + rate as usize / 2 {
        return (0, 0);
    }
    // The end that best matches the start (correlation of a short window).
    let w = (rate / 100) as usize;
    let score = |a: usize, b: usize| {
        let (mut ab, mut aa, mut bb) = (0.0f64, 0.0f64, 0.0f64);
        for k in 0..w {
            let (x, y) = (mono(a + k) as f64, mono(b + k) as f64);
            ab += x * y;
            aa += x * x;
            bb += y * y;
        }
        ab / (aa * bb).sqrt().max(1e-9)
    };
    let search = (rate / 40) as usize;
    let mut best = (f64::MIN, end_guess);
    let lo = end_guess.saturating_sub(search);
    for b in (lo..end_guess.min(n - w - 1)).step_by(2) {
        let s = score(start_guess, b);
        if s > best.0 {
            best = (s, b);
        }
    }
    (start_guess as u32, best.1 as u32)
}

/// Builds an instrument from its downloaded files (`src`, the library's
/// folders as on GitHub) into `out` (`samples.bin`, `index.json`).
pub fn build(def: &Def, src: &Path, out: &Path, progress: &dyn Fn(usize, usize)) -> Result<(), String> {
    std::fs::create_dir_all(out).map_err(|e| e.to_string())?;
    let mut arts = Vec::new();
    let mut paths: Vec<String> = Vec::new();
    let mut by_path: HashMap<String, u32> = HashMap::new();
    let mut regions_of: Vec<Vec<(Region, String)>> = Vec::new();
    for a in def.arts {
        let text = std::fs::read_to_string(src.join(a.sfz)).map_err(|e| format!("{}: {e}", a.sfz))?;
        let regions = parse_sfz(&text);
        if regions.is_empty() {
            return Err(format!("{} has no regions", a.sfz));
        }
        for r in &regions {
            if !by_path.contains_key(&r.sample) {
                by_path.insert(r.sample.clone(), paths.len() as u32);
                paths.push(r.sample.clone());
            }
        }
        regions_of.push(regions.into_iter().map(|r| {
            let p = r.sample.clone();
            (r, p)
        }).collect());
    }
    // Which samples are held (they get loops).
    let mut held = vec![false; paths.len()];
    for (a, regions) in def.arts.iter().zip(&regions_of) {
        for (_, p) in regions {
            if a.held {
                held[by_path[p] as usize] = true;
            }
        }
    }
    let tmp = out.join("samples.bin.part");
    let mut w = BufWriter::new(File::create(&tmp).map_err(|e| e.to_string())?);
    let mut samples = Vec::with_capacity(paths.len());
    let mut offset = 0u64;
    for (i, p) in paths.iter().enumerate() {
        progress(i, paths.len());
        let (frames, rate) = decode(&src.join(p))?;
        let (loop_start, loop_end) = if held[i] { find_loop(&frames, rate) } else { (0, 0) };
        let bytes: Vec<u8> = frames.iter().flat_map(|s| s.to_le_bytes()).collect();
        w.write_all(&bytes).map_err(|e| e.to_string())?;
        let n = (frames.len() / 2) as u32;
        samples.push(SampleEntry { offset, frames: n, rate, loop_start, loop_end });
        offset += n as u64;
    }
    w.flush().map_err(|e| e.to_string())?;
    drop(w);
    for (a, regions) in def.arts.iter().zip(regions_of) {
        arts.push(ArtEntry { label: a.label.into(), held: a.held, regions: regions.into_iter().map(|(r, p)| (r, by_path[&p])).collect() });
    }
    let index = Index { version: 1, instrument: def.name.into(), arts, samples };
    std::fs::write(out.join("index.json"), serde_json::to_vec(&index).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, out.join("samples.bin")).map_err(|e| e.to_string())?;
    progress(paths.len(), paths.len());
    Ok(())
}

/// The libraries the engines play from, by instrument.
static LIBRARIES: Mutex<Vec<Option<Arc<Library>>>> = Mutex::new(Vec::new());

pub fn set_library(instrument: usize, lib: Option<Arc<Library>>) {
    if let Ok(mut g) = LIBRARIES.lock() {
        if g.len() <= instrument {
            g.resize(instrument + 1, None);
        }
        g[instrument] = lib;
    }
}

/// (Never waits: None if the list is busy.)
pub fn library(instrument: usize) -> Option<Arc<Library>> {
    LIBRARIES.try_lock().ok().and_then(|g| g.get(instrument).cloned().flatten())
}

// ---------------------------------------------------------------- player

#[derive(Clone, Copy, Default)]
struct Layer {
    sample: u32,
    pos: f64,
    inc: f64,
    gain: f32,
    /// The weight Dynamics gives it, and where it is going.
    weight: f32,
    center: f32,
}

#[derive(Clone, Copy, Default)]
struct Voice {
    note: u8,
    held: bool,
    layers: [Option<Layer>; 2],
    gate: bool,
    env: f32,
    attack: f32,
    release: f32,
    /// A legato hand-over: fading in (+) or out (-) per sample.
    fade: f32,
    fade_gain: f32,
    vel: f32,
    detune: f32,
    age: u64,
    active: bool,
}

pub struct OrchestraState {
    sr: f32,
    voices: Vec<Voice>,
    lib: Option<Arc<Library>>,
    lib_index: usize,
    clock: u64,
    seq: HashMap<(usize, u8, u8), u8>,
    rng: u32,
    held: Vec<(u8, f32)>,
    dynamics: f32,
    pub pitch_bend: f32,
}

fn get(p: &[f32], i: usize, or: f32) -> f32 {
    p.get(i).copied().unwrap_or(or)
}

impl OrchestraState {
    pub fn new(sr: f32) -> Self {
        Self { sr, voices: vec![Voice::default(); 32], lib: None, lib_index: usize::MAX, clock: 0, seq: HashMap::new(), rng: 0x9E37_79B9, held: Vec::new(), dynamics: 0.7, pitch_bend: 0.0 }
    }

    /// Whether the instrument it plays is installed.
    pub fn ready(&self) -> bool {
        self.lib.is_some()
    }

    fn library_for(&mut self, p: &[f32]) {
        let i = get(p, INSTRUMENT, 0.0).round().max(0.0) as usize;
        if i != self.lib_index || self.lib.is_none() {
            if let Some(lib) = library(i) {
                self.lib = Some(lib);
                self.lib_index = i;
            } else if i != self.lib_index {
                self.lib = None;
                self.lib_index = i;
            }
        }
    }

    fn rand(&mut self) -> f32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        (self.rng >> 8) as f32 / (1u32 << 24) as f32
    }

    pub fn note_on(&mut self, p: &[f32], note: u8, vel: f32) {
        self.library_for(p);
        let Some(lib) = self.lib.clone() else { return };
        self.held.retain(|k| k.0 != note);
        self.held.push((note, vel));
        let art = (get(p, ARTICULATION, 0.0).round().max(0.0) as usize).min(lib.arts.len().saturating_sub(1));
        let Some(a) = lib.arts.get(art) else { return };
        let key = (note as i32 + get(p, TRANSPOSE, 0.0).round() as i32).clamp(0, 127) as u8;
        // The regions for this key, by dynamic layer.
        let random = self.rand();
        let counter = *self.seq.entry((art, key, 0)).or_insert(0);
        let mut layers: Vec<&(Region, u32)> = a
            .regions
            .iter()
            .filter(|(r, _)| r.lokey <= key && key <= r.hikey)
            .filter(|(r, _)| r.seq_len <= 1 || (counter % r.seq_len) + 1 == r.seq_pos)
            .filter(|(r, _)| random >= r.lorand && random < r.hirand.max(r.lorand + 1e-6) || (r.lorand <= 0.0 && r.hirand >= 1.0))
            .collect();
        if layers.is_empty() {
            // (a round robin missing here: any of the key's)
            layers = a.regions.iter().filter(|(r, _)| r.lokey <= key && key <= r.hikey).collect();
        }
        if layers.is_empty() {
            return;
        }
        self.seq.insert((art, key, 0), counter.wrapping_add(1));
        layers.sort_by_key(|(r, _)| r.lovel);
        layers.dedup_by_key(|x| x.0.lovel);
        let life = get(p, LIFE, 0.25);
        let detune = (self.rand() - 0.5) * 8.0 * life;
        let gain_life = 1.0 + (self.rand() - 0.5) * 0.25 * life;
        let mk = |(r, s): &(Region, u32), weight: f32| {
            let e = &lib.samples[*s as usize];
            let ratio = e.rate as f64 / self.sr as f64;
            let semis = key as f64 - r.keycenter as f64 + r.tune as f64 / 100.0;
            Layer { sample: *s, pos: 0.0, inc: ratio * 2f64.powf(semis / 12.0), gain: 10f32.powf(r.volume_db / 20.0) * gain_life, weight, center: (r.lovel as f32 + r.hivel as f32) * 0.5 }
        };
        // Held notes: the two layers either side of the dynamics; short ones: the velocity's.
        let vel_amt = get(p, VELOCITY, 0.6);
        let dyn_pos = if a.held { (1.0 - vel_amt) * get(p, DYNAMICS, 0.7) + vel_amt * vel } else { vel };
        let pos = dyn_pos.clamp(0.0, 1.0) * 127.0;
        let pick = layers.iter().position(|(r, _)| pos >= r.lovel as f32 && pos <= r.hivel as f32 + 0.999).unwrap_or(if pos < layers[0].0.lovel as f32 { 0 } else { layers.len() - 1 });
        let chosen: [Option<Layer>; 2] = if a.held && layers.len() > 1 {
            let next = if pick + 1 < layers.len() { pick + 1 } else { pick - 1 };
            [Some(mk(layers[pick], 1.0)), Some(mk(layers[next], 0.0))]
        } else {
            [Some(mk(layers[pick], 1.0)), None]
        };
        for l in chosen.iter().flatten() {
            lib.prefetch(l.sample);
        }
        // Legato: the held note hands over to this one.
        let legato = get(p, LEGATO, 0.0) >= 0.5 && a.held;
        let legato_samples = get(p, LEGATO_TIME, 80.0) / 1000.0 * self.sr;
        let mut start_at = 0.0;
        if legato {
            let mut any = false;
            for v in self.voices.iter_mut().filter(|v| v.active && v.gate) {
                v.gate = false;
                v.fade = -1.0 / legato_samples.max(1.0);
                any = true;
            }
            if any {
                // Into the note past its attack.
                start_at = 0.09;
            }
        }
        self.clock += 1;
        let slot = self.voices.iter().position(|v| !v.active).unwrap_or_else(|| (0..self.voices.len()).min_by_key(|&i| self.voices[i].age).unwrap_or(0));
        let attack = (r_attack(&layers[pick].0) + get(p, ATTACK, 1.0) / 1000.0).max(0.0005);
        let release = layers[pick].0.release * get(p, RELEASE, 1.0);
        let mut v = Voice {
            note,
            held: a.held,
            layers: chosen,
            gate: true,
            env: 0.0,
            attack: 1.0 / (attack * self.sr),
            release: (-6.9 / (release.max(0.01) * self.sr)).exp(),
            fade: 0.0,
            fade_gain: 1.0,
            vel,
            detune,
            age: self.clock,
            active: true,
        };
        if start_at > 0.0 {
            for l in v.layers.iter_mut().flatten() {
                l.pos = start_at * lib.samples[l.sample as usize].rate as f64;
            }
            v.env = 1.0;
            v.fade = 1.0 / legato_samples.max(1.0);
            v.fade_gain = 0.0;
        }
        self.voices[slot] = v;
    }

    pub fn note_off(&mut self, p: &[f32], note: u8) {
        self.held.retain(|k| k.0 != note);
        let legato = get(p, LEGATO, 0.0) >= 0.5;
        // Legato back to a note still held.
        if legato {
            if let Some(&(n, vel)) = self.held.last() {
                if self.voices.iter().any(|v| v.active && v.gate && v.note == note && v.held) {
                    self.note_on(p, n, vel);
                    self.held.retain(|k| k.0 != n);
                    self.held.push((n, vel));
                    return;
                }
            }
        }
        for v in self.voices.iter_mut() {
            if v.active && v.gate && v.note == note {
                v.gate = false;
            }
        }
    }

    pub fn all_off(&mut self) {
        self.held.clear();
        for v in self.voices.iter_mut() {
            v.active = false;
        }
    }

    pub fn visual(&self, out: &mut [f32; crate::engine::INST_VALS]) {
        out[0] = if self.lib.is_some() { 1.0 } else { 0.0 };
        out[1] = self.dynamics;
        out[2] = self.voices.iter().filter(|v| v.active).count() as f32;
        if let Some(lib) = &self.lib {
            out[3] = lib.arts.len() as f32;
        }
    }

    pub fn render(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32]) {
        let n = l.len().min(r.len());
        self.library_for(p);
        let Some(lib) = self.lib.clone() else {
            l[..n].fill(0.0);
            r[..n].fill(0.0);
            return;
        };
        let target_dyn = get(p, DYNAMICS, 0.7).clamp(0.0, 1.0);
        let vel_amt = get(p, VELOCITY, 0.6);
        let width = get(p, WIDTH, 1.0);
        let (pl, pr) = crate::dsp::pan_gains(get(p, PAN, 0.0));
        let volume = 10f32.powf(get(p, VOLUME, 0.0) / 20.0) * 0.7;
        let tune = get(p, TUNE, 0.0) / 1200.0 + self.pitch_bend / 12.0;
        let bend = 2f64.powf(tune as f64);
        let xf = (0.12 * self.sr) as f64;
        l[..n].fill(0.0);
        r[..n].fill(0.0);
        // Dynamics glides (a mod wheel's speed).
        let dyn_k = 1.0 - (-1.0 / (0.03 * self.sr)).exp();
        for i in 0..n {
            self.dynamics += (target_dyn - self.dynamics) * dyn_k;
            let mut out_l = 0.0f32;
            let mut out_r = 0.0f32;
            for v in self.voices.iter_mut() {
                if !v.active {
                    continue;
                }
                // The envelope: attack, held, released; and a legato hand-over.
                if v.gate {
                    v.env = (v.env + v.attack).min(1.0);
                } else if v.fade >= 0.0 {
                    v.env *= v.release;
                }
                if v.fade != 0.0 {
                    v.fade_gain = (v.fade_gain + v.fade).clamp(0.0, 1.0);
                    if v.fade_gain <= 0.0 && v.fade < 0.0 {
                        v.active = false;
                        continue;
                    }
                    if v.fade_gain >= 1.0 && v.fade > 0.0 {
                        v.fade = 0.0;
                    }
                }
                if v.env < 1e-4 && !v.gate {
                    v.active = false;
                    continue;
                }
                // Held notes follow Dynamics: the layers crossfade, the loudness goes with them.
                let pos = if v.held { ((1.0 - vel_amt) * self.dynamics + vel_amt * v.vel).clamp(0.0, 1.0) * 127.0 } else { v.vel * 127.0 };
                if v.held {
                    if let (Some(a), Some(b)) = (v.layers[0], v.layers[1]) {
                        let t = if (b.center - a.center).abs() > 1.0 { ((pos - a.center) / (b.center - a.center)).clamp(0.0, 1.0) } else { 0.0 };
                        if let Some(x) = v.layers[0].as_mut() {
                            x.weight = (1.0 - t).sqrt();
                        }
                        if let Some(x) = v.layers[1].as_mut() {
                            x.weight = t.sqrt();
                        }
                    }
                }
                // (SFZ's velocity tracking: loudness as the square of the dynamic)
                let loud = (pos / 127.0).max(0.02).powi(2);
                let g = v.env * v.fade_gain * loud;
                let mut ended = true;
                for layer in v.layers.iter_mut().flatten() {
                    let e = &lib.samples[layer.sample as usize];
                    let frames = lib.frames(layer.sample);
                    let len = e.frames as f64;
                    let looping = v.held && e.loop_end > e.loop_start + 1000;
                    if layer.pos >= len - 1.0 && !looping {
                        continue;
                    }
                    ended = false;
                    if layer.weight <= 1e-4 {
                        layer.pos += layer.inc * bend * 2f64.powf(v.detune as f64 / 1200.0);
                        if looping && layer.pos >= e.loop_end as f64 {
                            layer.pos -= (e.loop_end - e.loop_start) as f64;
                        }
                        continue;
                    }
                    let read = |pos: f64| {
                        let i0 = pos as usize;
                        let f = (pos - i0 as f64) as f32;
                        let i1 = (i0 + 1).min(e.frames as usize - 1);
                        let a = (frames[2 * i0] as f32, frames[2 * i0 + 1] as f32);
                        let b = (frames[2 * i1] as f32, frames[2 * i1 + 1] as f32);
                        (a.0 + (b.0 - a.0) * f, a.1 + (b.1 - a.1) * f)
                    };
                    let (mut sl, mut sr_) = read(layer.pos.min(len - 1.0));
                    if looping {
                        // Near the loop's end: crossfading into its start.
                        let (ls, le) = (e.loop_start as f64, e.loop_end as f64);
                        let into = layer.pos - (le - xf);
                        if into > 0.0 {
                            let t = (into / xf).min(1.0) as f32;
                            let (al, ar) = read((layer.pos - (le - ls)).max(0.0));
                            let (wa, wb) = ((1.0 - t).sqrt(), t.sqrt());
                            sl = sl * wa + al * wb;
                            sr_ = sr_ * wa + ar * wb;
                        }
                    }
                    let k = layer.gain * layer.weight * g / 32768.0;
                    out_l += sl * k;
                    out_r += sr_ * k;
                    layer.pos += layer.inc * bend * 2f64.powf(v.detune as f64 / 1200.0);
                    if looping && layer.pos >= e.loop_end as f64 {
                        layer.pos -= (e.loop_end - e.loop_start) as f64;
                    }
                }
                if ended {
                    v.active = false;
                }
            }
            // Width, pan and level.
            let m = (out_l + out_r) * 0.5;
            let s = (out_l - out_r) * 0.5 * width;
            l[i] = (m + s) * pl * volume;
            r[i] = (m - s) * pr * volume;
        }
    }
}

fn r_attack(r: &Region) -> f32 {
    r.attack.max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SFZ: &str = "<control>\r\ndefault_path=Strings\\Solo Violin\\Arco Vib\\\r\n<global>\r\nampeg_attack=0.001\r\nampeg_release=0.8\r\n<group> //Begin Group\r\nseq_length=2\r\nseq_position=1\r\n<region>\r\nsample=LLVln_ArcoVib_A3_p.wav\r\nlokey=56\r\nhikey=58\r\npitch_keycenter=57\r\nlovel=0\r\nhivel=62\r\nvolume=21\r\n<region> sample=LLVln A#3 f.wav lokey=59 hikey=61 pitch_keycenter=A#3 lovel=63 hivel=127\r\n<group> seq_position=2 lorand=0.5 hirand=1.0\r\n<region> sample=x.wav key=60\r\n";

    #[test]
    fn the_sfz_reader_inherits_and_keeps_spaces() {
        let r = parse_sfz(SFZ);
        assert_eq!(r.len(), 3);
        assert_eq!(r[0].sample, "Strings/Solo Violin/Arco Vib/LLVln_ArcoVib_A3_p.wav");
        assert_eq!((r[0].lokey, r[0].hikey, r[0].keycenter, r[0].hivel), (56, 58, 57, 62));
        assert_eq!(r[0].volume_db, 21.0);
        assert_eq!(r[0].release, 0.8);
        assert_eq!((r[0].seq_len, r[0].seq_pos), (2, 1));
        assert_eq!(r[1].sample, "Strings/Solo Violin/Arco Vib/LLVln A#3 f.wav");
        assert_eq!(r[1].keycenter, 58);
        assert_eq!((r[2].seq_pos, r[2].lorand, r[2].lokey, r[2].hikey), (2, 0.5, 60, 60));
        assert!(url(&r[1].sample).ends_with("Arco%20Vib/LLVln%20A%233%20f.wav"));
    }

    #[test]
    fn every_instrument_has_its_articulations() {
        assert_eq!(NAMES.len(), INSTRUMENTS.len());
        for (n, d) in NAMES.iter().zip(INSTRUMENTS) {
            assert_eq!(*n, d.name);
        }
        for d in INSTRUMENTS {
            assert!(!d.arts.is_empty() && d.arts.len() <= 8, "{}", d.name);
            assert_eq!(index_of(d.name).map(|i| INSTRUMENTS[i].name), Some(d.name));
            let p = preset(d.name);
            assert_eq!(p.len(), SPECS.len());
            assert_eq!(INSTRUMENTS[p[INSTRUMENT] as usize].name, d.name);
        }
    }

    /// A tiny instrument built from generated WAVs: two layers, a held articulation.
    fn test_library(dir: &Path) -> Arc<Library> {
        let src = dir.join("src");
        std::fs::create_dir_all(src.join("S")).unwrap();
        for (name, amp) in [("p", 0.1f32), ("f", 0.8)] {
            let spec = hound::WavSpec { channels: 1, sample_rate: 48000, bits_per_sample: 16, sample_format: hound::SampleFormat::Int };
            let mut w = hound::WavWriter::create(src.join(format!("S/tone_{name}.wav")), spec).unwrap();
            for i in 0..48000 * 3 {
                let x = (std::f32::consts::TAU * 440.0 * i as f32 / 48000.0).sin() * amp;
                w.write_sample((x * 32767.0) as i16).unwrap();
            }
            w.finalize().unwrap();
        }
        std::fs::write(src.join("T.sfz"), "<control> default_path=S\\ <region> sample=tone_p.wav lokey=60 hikey=80 pitch_keycenter=69 lovel=0 hivel=63 volume=12 <region> sample=tone_f.wav lokey=60 hikey=80 pitch_keycenter=69 lovel=64 hivel=127").unwrap();
        static DEF: Def = Def { name: "Test", family: Family::Strings, mb: 1, arts: &[held("Sustain", "T.sfz")] };
        build(&DEF, &src, &dir.join("out"), &|_, _| {}).unwrap();
        Library::open(&dir.join("out")).unwrap()
    }

    #[test]
    fn it_builds_plays_at_pitch_follows_dynamics_and_loops() {
        let dir = std::env::temp_dir().join(format!("krabton-orchestra-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let lib = test_library(&dir);
        assert_eq!(lib.arts[0].regions.len(), 2);
        assert!(lib.samples[0].loop_end > lib.samples[0].loop_start, "a held sample loops");
        set_library(63, Some(lib));
        let mut p = preset("Solo Violin");
        p[INSTRUMENT] = 63.0;
        p[VELOCITY] = 0.0;
        p[LIFE] = 0.0;
        let sr = 48000.0;
        let run = |st: &mut OrchestraState, p: &[f32], secs: f32| {
            let mut out = Vec::new();
            for _ in 0..(secs * sr / 256.0) as usize {
                let (mut l, mut r) = (vec![0.0; 256], vec![0.0; 256]);
                st.render(p, &mut l, &mut r);
                out.extend(l);
            }
            out
        };
        let rms = |x: &[f32]| (x.iter().map(|v| v * v).sum::<f32>() / x.len() as f32).sqrt();
        // A above middle C plays at 440 Hz.
        p[DYNAMICS] = 1.0;
        let mut st = OrchestraState::new(sr);
        st.note_on(&p, 69, 0.8);
        let loud = run(&mut st, &p, 0.5);
        let zc = loud[12000..].windows(2).filter(|w| w[0] < 0.0 && w[1] >= 0.0).count() as f32 / ((loud.len() - 12000) as f32 / sr);
        assert!((zc - 440.0).abs() < 6.0, "{zc}");
        // Quieter dynamics: quieter.
        p[DYNAMICS] = 0.2;
        let mut st = OrchestraState::new(sr);
        st.note_on(&p, 69, 0.8);
        let soft = run(&mut st, &p, 0.5);
        assert!(rms(&soft[12000..]) < 0.5 * rms(&loud[12000..]), "{} {}", rms(&soft[12000..]), rms(&loud[12000..]));
        // Held past the recording's end: it keeps sounding (the loop).
        let mut st = OrchestraState::new(sr);
        p[DYNAMICS] = 1.0;
        st.note_on(&p, 69, 0.8);
        let long = run(&mut st, &p, 8.0);
        assert!(rms(&long[long.len() - 24000..]) > 0.2 * rms(&loud[12000..]), "held notes loop");
        assert!(long.iter().all(|x| x.is_finite() && x.abs() < 2.0));
        // Released, it stops.
        st.note_off(&p, 69);
        let tail = run(&mut st, &p, 3.0);
        assert!(rms(&tail[tail.len() - 4800..]) < 1e-3);
        set_library(63, None);
        let _ = std::fs::remove_dir_all(&dir);
    }
}

/// With a downloaded instrument (ORCHESTRA_SRC: its files; ORCHESTRA_NAME):
/// builds it and checks every articulation plays in tune.
#[cfg(test)]
mod real {
    use super::*;

    fn pitch(x: &[f32], sr: f32) -> f32 {
        // The strongest period by autocorrelation (between 40 and 2000 Hz).
        let lag_of = |lag: usize| x.iter().zip(&x[lag..]).map(|(a, b)| a * b).sum::<f32>();
        let (lo, hi) = ((sr / 2000.0) as usize, (sr / 40.0) as usize);
        let best = (lo..hi).max_by(|a, b| lag_of(*a).total_cmp(&lag_of(*b))).unwrap_or(lo);
        sr / best as f32
    }

    #[test]
    #[ignore]
    fn a_real_instrument_builds_and_plays_in_tune() {
        let Some(src) = std::env::var_os("ORCHESTRA_SRC") else { return };
        let name = std::env::var("ORCHESTRA_NAME").unwrap_or_else(|_| "Solo Violin".into());
        let i = index_of(&name).unwrap();
        let src = Path::new(&src);
        let out = src.join("built");
        let t = std::time::Instant::now();
        build(&INSTRUMENTS[i], src, &out, &|_, _| {}).unwrap();
        eprintln!("built in {:.1} s, {} MB", t.elapsed().as_secs_f32(), std::fs::metadata(out.join("samples.bin")).unwrap().len() / 1_000_000);
        let lib = Library::open(&out).unwrap();
        let loops = lib.samples.iter().filter(|s| s.loop_end > s.loop_start).count();
        eprintln!("{} samples, {loops} loop", lib.samples.len());
        set_library(i, Some(lib.clone()));
        let sr = 48000.0;
        for (a, art) in lib.arts.iter().enumerate() {
            let (lo, hi) = lib.range(a);
            let mut line = format!("{:<12} keys {lo}-{hi}:", art.label);
            for note in [lo + 2, (lo + hi) / 2, hi.saturating_sub(3)] {
                let mut p = preset(&name);
                p[ARTICULATION] = a as f32;
                p[LIFE] = 0.0;
                let mut st = OrchestraState::new(sr);
                st.note_on(&p, note, 0.8);
                let mut out = Vec::new();
                for _ in 0..(1.5 * sr / 256.0) as usize {
                    let (mut l, mut r) = (vec![0.0; 256], vec![0.0; 256]);
                    st.render(&p, &mut l, &mut r);
                    out.extend(l);
                }
                let peak = out.iter().fold(0.0f32, |m, x| m.max(x.abs()));
                let seg = &out[(0.2 * sr) as usize..(0.5 * sr) as usize];
                let f = pitch(seg, sr);
                let want = 440.0 * 2f32.powf((note as f32 - 69.0) / 12.0);
                // (in semitones from the note, octaves folded)
                let off = 12.0 * (f / want).log2();
                line += &format!(" {note}: {f:.0}Hz ({off:+.1}) peak {peak:.2};");
                assert!(out.iter().all(|x| x.is_finite()));
            }
            eprintln!("{line}");
        }
        set_library(i, None);
    }
}

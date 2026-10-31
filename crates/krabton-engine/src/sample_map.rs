//! Laying samples out over the keyboard for the Sampler: from file names
//! (`Piano C4 mf.wav`), from SFZ instruments, or by slicing one sample at
//! its hits. Only the layout lives here; the app loads the audio.

use crate::project::{DrumPad, SampleZone};
use crate::sample::SampleRef;
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

/// Where a sample goes, before its audio is loaded.
#[derive(Clone, Debug, PartialEq)]
pub struct ZoneSpec {
    pub path: PathBuf,
    pub lo_key: u8,
    pub hi_key: u8,
    pub root: u8,
    pub lo_vel: u8,
    pub hi_vel: u8,
    /// Cents.
    pub tune: f32,
    pub gain_db: f32,
    pub pan: f32,
    /// Frames; `end` 0 is the end of the sample.
    pub start: u32,
    pub end: u32,
    /// Frames, when the zone loops.
    pub loop_range: Option<(u32, u32)>,
}

impl ZoneSpec {
    pub fn new(path: PathBuf, lo_key: u8, hi_key: u8, root: u8) -> Self {
        Self { path, lo_key, hi_key, root, lo_vel: 0, hi_vel: 127, tune: 0.0, gain_db: 0.0, pan: 0.0, start: 0, end: 0, loop_range: None }
    }
}

/// A MIDI note from a name like `C4`, `c#3`, `Bb-1` (C4 = 60).
pub fn parse_note(s: &str) -> Option<u8> {
    let s = s.trim();
    let mut chars = s.chars();
    let letter = chars.next()?.to_ascii_lowercase();
    let base = match letter {
        'c' => 0,
        'd' => 2,
        'e' => 4,
        'f' => 5,
        'g' => 7,
        'a' => 9,
        'b' => 11,
        _ => return None,
    };
    let rest: String = chars.collect();
    let (acc, oct) = if let Some(r) = rest.strip_prefix('#').or_else(|| rest.strip_prefix('♯')) {
        (1, r)
    } else if let Some(r) = rest.strip_prefix('b').or_else(|| rest.strip_prefix('♭')) {
        (-1, r)
    } else {
        (0, rest.as_str())
    };
    let octave: i32 = oct.parse().ok()?;
    let n = (octave + 1) * 12 + base + acc;
    (0..=127).contains(&n).then_some(n as u8)
}

/// A key in an SFZ file: a number or a note name.
fn sfz_key(v: &str) -> Option<u8> {
    v.trim().parse::<i32>().ok().filter(|n| (0..=127).contains(n)).map(|n| n as u8).or_else(|| parse_note(v))
}

/// The words of a file name (split at anything that is not a letter, digit,
/// `#` or `-` before a digit).
fn words(stem: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let chars: Vec<char> = stem.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        // "A-1": a note in octave -1 (only right after a note letter).
        let note_like = matches!(cur.len(), 1 | 2) && cur.starts_with(|l: char| matches!(l.to_ascii_lowercase(), 'a'..='g'));
        let minus_digit = c == '-' && chars.get(i + 1).is_some_and(|d| d.is_ascii_digit()) && note_like;
        if c.is_alphanumeric() || c == '#' || minus_digit {
            cur.push(c);
        } else if !cur.is_empty() {
            out.push(std::mem::take(&mut cur));
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// The note a file name gives, if any: a note name (`C#4`) or a MIDI number
/// (`060`).
pub fn note_in_name(stem: &str) -> Option<u8> {
    let ws = words(stem);
    // Names like "C4" also hide inside words such as "PianoC4"; take the
    // last word that ends in one.
    for w in ws.iter().rev() {
        if let Some(n) = parse_note(w) {
            return Some(n);
        }
        let lower = w.to_lowercase();
        for i in (0..lower.len()).rev() {
            if lower.is_char_boundary(i) && i > 0 && w[..i].chars().last().is_some_and(|c| !c.is_ascii_digit()) {
                if let Some(n) = parse_note(&w[i..]) {
                    if w[i..].chars().next().is_some_and(|c| c.is_ascii_uppercase()) {
                        return Some(n);
                    }
                }
            }
        }
    }
    ws.iter().rev().filter_map(|w| w.parse::<u32>().ok()).find(|n| (21..=108).contains(n)).map(|n| n as u8)
}

/// How loud a file name says a layer is (higher is louder), if it says.
fn dynamic_in_name(stem: &str) -> Option<u32> {
    for w in words(stem).iter().rev() {
        let l = w.to_lowercase();
        let marks = ["ppp", "pp", "p", "mp", "mf", "f", "ff", "fff"];
        if let Some(i) = marks.iter().position(|m| *m == l) {
            return Some(i as u32 * 10);
        }
        for prefix in ["vel", "v", "l", "layer"] {
            if let Some(n) = l.strip_prefix(prefix).and_then(|r| r.parse::<u32>().ok()) {
                return Some(n);
            }
        }
    }
    None
}

/// Lays files out over the keyboard. Files with a note in their name are
/// spread around it (each covers up to halfway to its neighbours) and
/// files on the same note become velocity layers; without notes, the files
/// go one per key from C1, like a drum kit.
pub fn map_files(paths: &[PathBuf]) -> Vec<ZoneSpec> {
    let mut paths: Vec<PathBuf> = paths.to_vec();
    paths.sort();
    let stem = |p: &Path| p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    let notes: Vec<Option<u8>> = paths.iter().map(|p| note_in_name(&stem(p))).collect();
    if paths.len() == 1 {
        let root = notes[0].unwrap_or(60);
        return vec![ZoneSpec::new(paths[0].clone(), 0, 127, root)];
    }
    if notes.iter().all(|n| n.is_none()) {
        return paths.iter().enumerate().take(92).map(|(i, p)| ZoneSpec::new(p.clone(), 36 + i as u8, 36 + i as u8, 36 + i as u8)).collect();
    }
    let mut by_root: BTreeMap<u8, Vec<PathBuf>> = BTreeMap::new();
    for (p, n) in paths.iter().zip(&notes) {
        if let Some(n) = n {
            by_root.entry(*n).or_default().push(p.clone());
        }
    }
    let roots: Vec<u8> = by_root.keys().copied().collect();
    let mut out = Vec::new();
    for (i, root) in roots.iter().enumerate() {
        let lo = if i == 0 { 0 } else { (roots[i - 1] as u32 + *root as u32) / 2 + 1 } as u8;
        let hi = if i + 1 == roots.len() { 127 } else { (*root as u32 + roots[i + 1] as u32) / 2 } as u8;
        let mut layers = by_root[root].clone();
        layers.sort_by_key(|p| (dynamic_in_name(&stem(p)).unwrap_or(0), p.clone()));
        let n = layers.len() as u32;
        for (k, p) in layers.into_iter().enumerate() {
            let mut z = ZoneSpec::new(p, lo, hi, *root);
            z.lo_vel = (k as u32 * 128 / n) as u8;
            z.hi_vel = (((k as u32 + 1) * 128 / n).saturating_sub(1)).min(127) as u8;
            out.push(z);
        }
    }
    out
}

/// What an SFZ file describes.
#[derive(Clone, Debug, Default)]
pub struct SfzInstrument {
    pub zones: Vec<ZoneSpec>,
    /// Amp envelope, when the file sets one (seconds, sustain 0..1).
    pub attack: Option<f32>,
    pub decay: Option<f32>,
    pub sustain: Option<f32>,
    pub release: Option<f32>,
    /// Regions left out (release triggers, round robins after the first).
    pub skipped: usize,
}

/// Reads an SFZ file, following `#include`s and `#define`s.
pub fn load_sfz(path: &Path) -> Result<SfzInstrument, String> {
    let text = read_with_includes(path, 0)?;
    let dir = path.parent().unwrap_or(Path::new(".")).to_path_buf();
    Ok(parse_sfz(&text, &dir))
}

fn read_with_includes(path: &Path, depth: usize) -> Result<String, String> {
    if depth > 8 {
        return Err("too many nested #include".into());
    }
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let dir = path.parent().unwrap_or(Path::new("."));
    let mut out = String::new();
    for line in text.lines() {
        let t = line.trim_start();
        if let Some(rest) = t.strip_prefix("#include") {
            let name = rest.trim().trim_matches('"');
            out.push_str(&read_with_includes(&dir.join(name), depth + 1)?);
            out.push('\n');
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }
    Ok(out)
}

/// Removes `//` and `/* */` comments.
fn strip_comments(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let b = text.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'/' && i + 1 < b.len() && b[i + 1] == b'/' {
            while i < b.len() && b[i] != b'\n' {
                i += 1;
            }
        } else if b[i] == b'/' && i + 1 < b.len() && b[i + 1] == b'*' {
            i += 2;
            while i + 1 < b.len() && !(b[i] == b'*' && b[i + 1] == b'/') {
                i += 1;
            }
            i += 2;
        } else {
            let ch_len = text[i..].chars().next().map(|c| c.len_utf8()).unwrap_or(1);
            out.push_str(&text[i..i + ch_len]);
            i += ch_len;
        }
    }
    out
}

enum Token {
    Header(String),
    Opcode(String, String),
}

/// Splits SFZ text into headers and opcodes. A value runs until the next
/// opcode or header, so sample paths may contain spaces.
fn tokenize(text: &str) -> Vec<Token> {
    let mut out = Vec::new();
    for line in text.lines() {
        let mut rest = line.trim();
        while !rest.is_empty() {
            if rest.starts_with('<') {
                let end = rest.find('>').map(|e| e + 1).unwrap_or(rest.len());
                out.push(Token::Header(rest[1..end.saturating_sub(1).max(1)].trim().to_lowercase()));
                rest = rest[end..].trim_start();
                continue;
            }
            let Some(eq) = rest.find('=') else { break };
            let name = rest[..eq].trim().to_lowercase();
            let after = &rest[eq + 1..];
            // The value ends where the next `name=` or `<header>` starts.
            let mut end = after.len();
            let bytes = after.as_bytes();
            for i in 0..bytes.len() {
                if bytes[i] == b'<' && i > 0 && bytes[i - 1].is_ascii_whitespace() {
                    end = i;
                    break;
                }
                if bytes[i].is_ascii_whitespace() {
                    let tail = &after[i + 1..];
                    let word_end = tail.find(|c: char| !(c.is_ascii_alphanumeric() || c == '_')).unwrap_or(tail.len());
                    if word_end > 0 && tail[word_end..].starts_with('=') {
                        end = i;
                        break;
                    }
                }
            }
            out.push(Token::Opcode(name, after[..end].trim().to_string()));
            rest = after[end..].trim_start();
        }
    }
    out
}

pub fn parse_sfz(text: &str, dir: &Path) -> SfzInstrument {
    // #define $NAME value
    let mut defines: Vec<(String, String)> = Vec::new();
    let mut body = String::new();
    for line in strip_comments(text).lines() {
        let t = line.trim_start();
        if let Some(rest) = t.strip_prefix("#define") {
            let mut it = rest.split_whitespace();
            if let (Some(k), Some(v)) = (it.next(), it.next()) {
                defines.push((k.to_string(), v.to_string()));
            }
            continue;
        }
        body.push_str(line);
        body.push('\n');
    }
    defines.sort_by_key(|(k, _)| std::cmp::Reverse(k.len()));
    for (k, v) in &defines {
        body = body.replace(k.as_str(), v);
    }

    let mut inst = SfzInstrument::default();
    let mut control: HashMap<String, String> = HashMap::new();
    let mut global: HashMap<String, String> = HashMap::new();
    let mut master: HashMap<String, String> = HashMap::new();
    let mut group: HashMap<String, String> = HashMap::new();
    let mut region: Option<HashMap<String, String>> = None;
    let mut level = "";
    let flush = |region: &mut Option<HashMap<String, String>>, inst: &mut SfzInstrument, control: &HashMap<String, String>, layers: [&HashMap<String, String>; 3]| {
        let Some(r) = region.take() else { return };
        let mut all: HashMap<String, String> = HashMap::new();
        for l in layers {
            all.extend(l.iter().map(|(k, v)| (k.clone(), v.clone())));
        }
        all.extend(r);
        add_region(inst, control, &all, dir);
    };
    for tok in tokenize(&body) {
        match tok {
            Token::Header(h) => {
                flush(&mut region, &mut inst, &control, [&global, &master, &group]);
                match h.as_str() {
                    "control" => level = "control",
                    "global" => {
                        level = "global";
                        global.clear();
                        master.clear();
                        group.clear();
                    }
                    "master" => {
                        level = "master";
                        master.clear();
                        group.clear();
                    }
                    "group" => {
                        level = "group";
                        group.clear();
                    }
                    "region" => {
                        level = "region";
                        region = Some(HashMap::new());
                    }
                    _ => level = "",
                }
            }
            Token::Opcode(k, v) => match level {
                "control" => {
                    control.insert(k, v);
                }
                "global" => {
                    global.insert(k, v);
                }
                "master" => {
                    master.insert(k, v);
                }
                "group" => {
                    group.insert(k, v);
                }
                "region" => {
                    if let Some(r) = region.as_mut() {
                        r.insert(k, v);
                    }
                }
                _ => {}
            },
        }
    }
    flush(&mut region, &mut inst, &control, [&global, &master, &group]);
    inst
}

fn add_region(inst: &mut SfzInstrument, control: &HashMap<String, String>, op: &HashMap<String, String>, dir: &Path) {
    let get = |k: &str| op.get(k).map(|s| s.as_str());
    let num = |k: &str| get(k).and_then(|v| v.trim().parse::<f32>().ok());
    let Some(sample) = get("sample") else { return };
    if sample.starts_with('*') {
        // Built-in waveforms (*sine, *noise) are not samples.
        inst.skipped += 1;
        return;
    }
    if get("trigger").is_some_and(|t| t.eq_ignore_ascii_case("release") || t.eq_ignore_ascii_case("release_key")) {
        inst.skipped += 1;
        return;
    }
    if num("seq_position").is_some_and(|p| p > 1.0) {
        inst.skipped += 1;
        return;
    }
    let shift = control.get("note_offset").and_then(|v| v.parse::<i32>().ok()).unwrap_or(0) + 12 * control.get("octave_offset").and_then(|v| v.parse::<i32>().ok()).unwrap_or(0);
    let key = |k: &str| get(k).and_then(sfz_key).map(|n| (n as i32 + shift).clamp(0, 127) as u8);
    let (lo, hi) = match key("key") {
        Some(k) => (key("lokey").unwrap_or(k), key("hikey").unwrap_or(k)),
        None => (key("lokey").unwrap_or(0), key("hikey").unwrap_or(127)),
    };
    let root = key("pitch_keycenter").or(key("key")).unwrap_or(60);
    let default_path = control.get("default_path").map(|s| s.replace('\\', "/")).unwrap_or_default();
    let rel = format!("{default_path}{}", sample.replace('\\', "/"));
    let mut z = ZoneSpec::new(dir.join(rel), lo.min(hi), hi.max(lo), root);
    z.lo_vel = num("lovel").unwrap_or(0.0).clamp(0.0, 127.0) as u8;
    z.hi_vel = num("hivel").unwrap_or(127.0).clamp(0.0, 127.0) as u8;
    z.tune = num("tune").unwrap_or(0.0) + 100.0 * num("transpose").unwrap_or(0.0);
    z.gain_db = num("volume").unwrap_or(0.0) + num("amplitude").map(|a| 20.0 * (a / 100.0).max(1e-4).log10()).unwrap_or(0.0);
    z.pan = (num("pan").unwrap_or(0.0) / 100.0).clamp(-1.0, 1.0);
    z.start = num("offset").unwrap_or(0.0).max(0.0) as u32;
    z.end = num("end").unwrap_or(0.0).max(0.0) as u32;
    let looping = get("loop_mode").or(get("loopmode")).is_some_and(|m| m.starts_with("loop_"));
    let ls = num("loop_start").or(num("loopstart"));
    let le = num("loop_end").or(num("loopend"));
    if let (true, Some(s), Some(e)) = (looping, ls, le) {
        if e > s {
            z.loop_range = Some((s as u32, e as u32));
        }
    }
    // Drums: a zone that does not follow the keys plays at its own pitch
    // on every key.
    if num("pitch_keytrack").is_some_and(|t| t == 0.0) && z.hi_key > z.lo_key {
        for k in z.lo_key..=z.hi_key {
            let mut zk = z.clone();
            zk.lo_key = k;
            zk.hi_key = k;
            zk.root = k;
            inst.zones.push(zk);
        }
    } else {
        inst.zones.push(z);
    }
    if inst.attack.is_none() {
        inst.attack = num("ampeg_attack");
        inst.decay = num("ampeg_decay");
        inst.sustain = num("ampeg_sustain").map(|s| (s / 100.0).clamp(0.0, 1.0));
        inst.release = num("ampeg_release");
    }
}

/// Where the hits in `mono` are (frame numbers, starting with 0): rises of
/// the short-term level well above what came just before. At most `max`.
pub fn find_slices(mono: &[f32], sample_rate: u32, max: usize) -> Vec<u32> {
    let hop = (sample_rate as usize / 200).max(32); // 5 ms
    let env: Vec<f32> = mono.chunks(hop).map(|c| c.iter().map(|v| v * v).sum::<f32>() / c.len() as f32).collect();
    let peak = env.iter().fold(0.0f32, |m, v| m.max(*v));
    if peak <= 0.0 {
        return vec![0];
    }
    let floor = peak * 1e-3;
    let min_gap = (0.06 * sample_rate as f32 / hop as f32).ceil() as usize;
    let mut cuts = vec![0u32];
    let mut last = 0usize;
    for i in 2..env.len() {
        let before = env[i.saturating_sub(4)..i - 1].iter().fold(f32::MAX, |m, v| m.min(*v));
        if env[i] > floor && env[i] > 6.0 * before.max(floor * 0.1) && i - last >= min_gap {
            // Back up to where the rise starts.
            let at = (i - 1) * hop;
            if at > 0 {
                cuts.push(at as u32);
                last = i;
            }
        }
        if cuts.len() >= max {
            break;
        }
    }
    cuts
}

/// Zones with their samples loaded.
pub struct LoadedZones {
    pub zones: Vec<SampleZone>,
    /// The amp envelope an SFZ sets: attack, decay, sustain, release.
    pub envelope: [Option<f32>; 4],
    /// Files that could not be loaded.
    pub failed: Vec<String>,
    /// SFZ regions left out.
    pub skipped: usize,
}

pub fn is_audio(p: &Path) -> bool {
    crate::sample::is_audio_file(p)
}

pub fn is_sfz(p: &Path) -> bool {
    p.extension().is_some_and(|e| e.eq_ignore_ascii_case("sfz"))
}

/// The audio and SFZ files in `dir` and a few levels of subfolders.
pub fn files_in(dir: &Path) -> Vec<PathBuf> {
    fn walk(dir: &Path, depth: usize, out: &mut Vec<PathBuf>) {
        let Ok(rd) = std::fs::read_dir(dir) else { return };
        let mut entries: Vec<PathBuf> = rd.flatten().map(|e| e.path()).collect();
        entries.sort();
        for p in entries {
            if out.len() >= 4000 {
                return;
            }
            if p.is_dir() {
                if depth < 3 {
                    walk(&p, depth + 1, out);
                }
            } else if is_audio(&p) || is_sfz(&p) {
                out.push(p);
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, 0, &mut out);
    out
}

/// Loads what `paths` hold: an SFZ instrument (the first `.sfz` among them
/// or in a folder), or else the audio files laid out by name.
pub fn load_zones(paths: &[PathBuf]) -> Result<LoadedZones, String> {
    let mut files = Vec::new();
    for p in paths {
        if p.is_dir() {
            files.extend(files_in(p));
        } else {
            files.push(p.clone());
        }
    }
    let (specs, envelope, skipped) = match files.iter().find(|p| is_sfz(p)) {
        Some(sfz) => {
            let inst = load_sfz(sfz)?;
            (inst.zones, [inst.attack, inst.decay, inst.sustain, inst.release], inst.skipped)
        }
        None => {
            let audio: Vec<PathBuf> = files.into_iter().filter(|p| is_audio(p)).collect();
            if audio.is_empty() {
                return Err("No WAV, FLAC or SFZ files there".into());
            }
            (map_files(&audio), [None; 4], 0)
        }
    };
    let mut loaded: HashMap<PathBuf, Option<SampleRef>> = HashMap::new();
    let mut failed = Vec::new();
    let mut zones = Vec::new();
    for z in specs {
        let sample = loaded
            .entry(z.path.clone())
            .or_insert_with(|| match SampleRef::load(&z.path) {
                Ok(s) => Some(s),
                Err(e) => {
                    failed.push(e);
                    None
                }
            })
            .clone();
        if let Some(sample) = sample {
            zones.push(SampleZone {
                sample,
                lo_key: z.lo_key,
                hi_key: z.hi_key,
                root: z.root,
                lo_vel: z.lo_vel,
                hi_vel: z.hi_vel,
                tune: z.tune,
                gain_db: z.gain_db,
                pan: z.pan,
                start: z.start,
                end: z.end,
                loop_range: z.loop_range,
            });
        }
    }
    if zones.is_empty() {
        return Err(failed.first().cloned().unwrap_or_else(|| "Nothing to load".into()));
    }
    Ok(LoadedZones { zones, envelope, failed, skipped })
}

/// Cuts `sample` at its hits into zones, one per key from C1.
pub fn slice_zones(sample: &SampleRef, max: usize) -> Vec<SampleZone> {
    let mono: Vec<f32> = sample.left.iter().zip(&sample.right).map(|(a, b)| 0.5 * (a + b)).collect();
    let cuts = find_slices(&mono, sample.sample_rate, max.clamp(1, 92));
    cuts.iter()
        .enumerate()
        .map(|(i, &start)| {
            let key = 36 + i as u8;
            let end = cuts.get(i + 1).copied().unwrap_or(0);
            SampleZone { sample: sample.clone(), lo_key: key, hi_key: key, root: key, lo_vel: 0, hi_vel: 127, tune: 0.0, gain_db: 0.0, pan: 0.0, start, end, loop_range: None }
        })
        .collect()
}

/// Which drum a file name is, as the name of a pad of the classic kits.
fn drum_role(stem: &str) -> Option<&'static str> {
    let ws: Vec<String> = words(stem).iter().map(|w| w.to_lowercase()).collect();
    let has = |keys: &[&str]| ws.iter().any(|w| keys.iter().any(|k| w == k || (k.len() > 3 && w.contains(k))));
    Some(if has(&["kick", "bd", "bassdrum", "kik"]) {
        "Kick"
    } else if has(&["snare", "sd", "snr"]) {
        "Snare"
    } else if has(&["clap", "cp", "clp"]) {
        "Clap"
    } else if has(&["rim", "rimshot", "rs", "stick", "sidestick"]) {
        "Rim"
    } else if has(&["open", "oh", "ohh", "ohat"]) {
        "Open Hat"
    } else if has(&["hat", "hihat", "hh", "chh", "ch", "closed"]) {
        "Closed Hat"
    } else if has(&["tom", "tm", "lt", "mt", "ht", "floor"]) {
        "Tom"
    } else if has(&["cowbell", "cb", "bell", "perc", "shaker", "tamb", "tambourine", "conga", "bongo", "cymbal", "crash", "ride"]) {
        "Cowbell"
    } else {
        return None;
    })
}

/// A drum kit from sample files: each file goes to the pad its name says
/// (kick, snare, hat, ...), the rest fill the pads still free.
pub fn drum_kit_from(files: &[PathBuf]) -> (Vec<DrumPad>, Vec<String>) {
    let mut pads = crate::dsp::drums::kit("Classic 909");
    let mut audio: Vec<PathBuf> = files.iter().filter(|p| is_audio(p)).cloned().collect();
    audio.sort();
    let mut failed = Vec::new();
    let mut used = vec![false; pads.len()];
    let mut rest = Vec::new();
    for f in audio {
        let stem = f.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        match drum_role(&stem).and_then(|role| pads.iter().position(|p| p.name == role)).filter(|&i| !used[i]) {
            Some(i) => {
                used[i] = true;
                match SampleRef::load(&f) {
                    Ok(s) => {
                        pads[i].name = s.name.clone();
                        pads[i].sample = Some(s);
                    }
                    Err(e) => failed.push(e),
                }
            }
            None => rest.push(f),
        }
    }
    for f in rest {
        let Some(i) = used.iter().position(|u| !u) else { break };
        used[i] = true;
        match SampleRef::load(&f) {
            Ok(s) => {
                pads[i].name = s.name.clone();
                pads[i].sample = Some(s);
            }
            Err(e) => failed.push(e),
        }
    }
    (pads, failed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_folder_of_drum_samples_becomes_a_kit_and_an_instrument() {
        let dir = std::env::temp_dir().join(format!("krabton-kit-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let spec = hound::WavSpec { channels: 1, sample_rate: 44100, bits_per_sample: 16, sample_format: hound::SampleFormat::Int };
        for name in ["BD 808.wav", "snare tight.wav", "OH long.wav", "vox chop.wav", "Piano C4.wav", "Piano C5.wav"] {
            let mut w = hound::WavWriter::create(dir.join(name), spec).unwrap();
            for i in 0..4410 {
                w.write_sample(((i as f32 * 0.05).sin() * 20000.0) as i16).unwrap();
            }
            w.finalize().unwrap();
        }
        let files = files_in(&dir);
        assert_eq!(files.len(), 6);
        let drums: Vec<PathBuf> = files.iter().filter(|p| !p.to_string_lossy().contains("Piano")).cloned().collect();
        let (pads, failed) = drum_kit_from(&drums);
        assert!(failed.is_empty(), "{failed:?}");
        let by_note = |n: u8| pads.iter().find(|p| p.note == n).unwrap();
        assert_eq!(by_note(36).name, "BD 808");
        assert_eq!(by_note(38).name, "snare tight");
        assert_eq!(by_note(46).name, "OH long");
        assert_eq!(pads.iter().filter(|p| p.sample.is_some()).count(), 4, "the vox chop fills a free pad");
        let piano: Vec<PathBuf> = files.iter().filter(|p| p.to_string_lossy().contains("Piano")).cloned().collect();
        let lz = load_zones(&piano).unwrap();
        assert_eq!(lz.zones.iter().map(|z| (z.root, z.lo_key, z.hi_key)).collect::<Vec<_>>(), vec![(60, 0, 66), (72, 67, 127)]);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn drum_files_find_their_pads() {
        assert_eq!(drum_role("BD_808_long"), Some("Kick"));
        assert_eq!(drum_role("Snare 03"), Some("Snare"));
        assert_eq!(drum_role("hihat open"), Some("Open Hat"));
        assert_eq!(drum_role("HH closed"), Some("Closed Hat"));
        assert_eq!(drum_role("Floor Tom"), Some("Tom"));
        assert_eq!(drum_role("Vox chop"), None);
    }

    #[test]
    fn note_names() {
        assert_eq!(parse_note("C4"), Some(60));
        assert_eq!(parse_note("c#4"), Some(61));
        assert_eq!(parse_note("Bb2"), Some(46));
        assert_eq!(parse_note("A-1"), Some(9));
        assert_eq!(parse_note("H4"), None);
        assert_eq!(note_in_name("Piano C#4 mf"), Some(61));
        assert_eq!(note_in_name("cello_A2_ff_01"), Some(45));
        assert_eq!(note_in_name("PianoC4"), Some(60));
        assert_eq!(note_in_name("strings-060"), Some(60));
        assert_eq!(note_in_name("Kick 01"), None);
    }

    #[test]
    fn files_spread_over_the_keys_with_velocity_layers() {
        let p = |s: &str| PathBuf::from(format!("{s}.wav"));
        let z = map_files(&[p("Pno C3 ff"), p("Pno C3 pp"), p("Pno C4 pp"), p("Pno C4 ff"), p("Pno G4 mf")]);
        assert_eq!(z.len(), 5);
        let c3: Vec<_> = z.iter().filter(|z| z.root == 48).collect();
        assert_eq!((c3[0].lo_key, c3[0].hi_key), (0, 54));
        assert!(c3[0].path.to_string_lossy().contains("pp") && c3[0].hi_vel == 63 && c3[1].lo_vel == 64, "{c3:?}");
        let g4 = z.iter().find(|z| z.root == 67).unwrap();
        assert_eq!((g4.lo_key, g4.hi_key, g4.lo_vel, g4.hi_vel), (64, 127, 0, 127));
        // Without notes: one per key from C1.
        let kit = map_files(&[p("kick"), p("snare"), p("hat")]);
        assert_eq!(kit.iter().map(|z| (z.lo_key, z.root)).collect::<Vec<_>>(), vec![(36, 36), (37, 37), (38, 38)]);
    }

    #[test]
    fn sfz_regions_inherit_and_keep_spaces_in_paths() {
        let text = r#"
            // a comment
            <control> default_path=samples/
            #define $VEL 80
            <global> ampeg_release=0.8 volume=-3
            <group> lovel=0 hivel=$VEL
            <region> sample=Piano C4 soft.wav lokey=c4 hikey=d4 pitch_keycenter=c4
            <region> sample=Piano E4 soft.wav key=e4 tune=-10
            <group> lovel=81 hivel=127 trigger=release
            <region> sample=rel.wav key=60
            /* block
               comment */
            <group> pitch_keytrack=0 loop_mode=loop_continuous loop_start=100 loop_end=900
            <region> sample=kick.flac lokey=36 hikey=37
        "#;
        let inst = parse_sfz(text, Path::new("/lib"));
        assert_eq!(inst.zones.len(), 4, "{:?}", inst.zones);
        let z = &inst.zones[0];
        assert_eq!(z.path, Path::new("/lib").join("samples/Piano C4 soft.wav"));
        assert_eq!((z.lo_key, z.hi_key, z.root, z.lo_vel, z.hi_vel), (60, 62, 60, 0, 80));
        assert_eq!(z.gain_db, -3.0);
        assert_eq!((inst.zones[1].lo_key, inst.zones[1].root, inst.zones[1].tune), (64, 64, -10.0));
        assert_eq!(inst.skipped, 1, "the release trigger is left out");
        assert_eq!((inst.zones[2].root, inst.zones[3].root), (36, 37), "no key tracking: one zone per key");
        assert_eq!(inst.zones[2].loop_range, Some((100, 900)));
        assert_eq!(inst.release, Some(0.8));
    }

    #[test]
    fn slices_land_on_the_hits() {
        let sr = 48000;
        let mut x = vec![0.0f32; sr as usize];
        for hit in [0usize, 12000, 24000, 30000] {
            for i in 0..3000 {
                x[hit + i] = (i as f32 * 0.3).sin() * (-(i as f32) / 800.0).exp();
            }
        }
        let cuts = find_slices(&x, sr, 32);
        assert_eq!(cuts.len(), 4, "{cuts:?}");
        for (c, want) in cuts.iter().zip([0u32, 12000, 24000, 30000]) {
            assert!((*c as i64 - want as i64).abs() < 500, "{cuts:?}");
        }
    }
}

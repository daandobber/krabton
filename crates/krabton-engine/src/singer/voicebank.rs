//! UTAU voicebanks: a folder of recordings and the oto.ini files that say
//! how each is cut (`file=alias,offset,consonant,cutoff,preutterance,overlap`,
//! in milliseconds; a negative cutoff measured from the offset), read as
//! UTAU and OpenUtau read them: every oto.ini in the folder and its
//! subfolders (multi-pitch banks), in UTF-8 or Shift-JIS, prefix.map's
//! suffixes for the pitches, character.txt's name. Each recording's pitch
//! periods are found once, in the background, so the Singer can move the
//! voice to any pitch and length and keep it the singer's.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex, OnceLock, Weak};

/// One cut of a recording, as oto.ini gives it (milliseconds).
#[derive(Clone, Debug, PartialEq)]
pub struct OtoEntry {
    pub alias: String,
    pub sample: usize,
    pub offset: f32,
    pub consonant: f32,
    pub cutoff: f32,
    pub preutter: f32,
    pub overlap: f32,
}

/// A recording, mono.
pub struct Sample {
    pub name: String,
    pub sr: u32,
    pub data: Vec<i16>,
}

/// Where a recording's pitch periods are (sample frames), how long each is,
/// whether it is voiced there.
#[derive(Clone, Debug, Default)]
pub struct Marks {
    pub pos: Vec<u32>,
    pub period: Vec<u16>,
    pub voiced: Vec<bool>,
}

impl Marks {
    /// The mark nearest `at` (sample frames).
    pub fn nearest(&self, at: f64) -> Option<usize> {
        if self.pos.is_empty() {
            return None;
        }
        let at = at.max(0.0) as u32;
        let i = self.pos.partition_point(|&p| p < at);
        if i == 0 {
            Some(0)
        } else if i >= self.pos.len() {
            Some(self.pos.len() - 1)
        } else if self.pos[i] - at < at - self.pos[i - 1] {
            Some(i)
        } else {
            Some(i - 1)
        }
    }
}

pub struct VoicebankData {
    pub name: String,
    pub path: Option<PathBuf>,
    pub samples: Vec<Sample>,
    pub entries: Vec<OtoEntry>,
    pub aliases: HashMap<String, usize>,
    /// prefix.map: for MIDI notes, a prefix and a suffix to the alias.
    pub prefix_map: Vec<(u8, String, String)>,
    /// Whether its aliases run vowel to consonant ("a か", "- か").
    pub vcv: bool,
    /// What brings its singing to one loudness (voicebanks are recorded at
    /// very different levels).
    pub gain: f32,
    /// The pitch periods, once found (in the background).
    pub analysis: OnceLock<Vec<Marks>>,
    pub progress: AtomicU32,
}

/// A shared voicebank: saved as its folder; equal when it is the same one.
#[derive(Clone)]
pub struct VoicebankRef(pub Arc<VoicebankData>);

impl std::ops::Deref for VoicebankRef {
    type Target = VoicebankData;
    fn deref(&self) -> &VoicebankData {
        &self.0
    }
}

impl PartialEq for VoicebankRef {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl std::fmt::Debug for VoicebankRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Voicebank({}, {} aliases)", self.name, self.entries.len())
    }
}

/// Text as a voicebank writes it: UTF-8, or else Shift-JIS (most Japanese banks).
pub fn decode(bytes: &[u8]) -> String {
    let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF][..]).unwrap_or(bytes);
    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_string(),
        Err(_) => encoding_rs::SHIFT_JIS.decode(bytes).0.into_owned(),
    }
}

/// One oto.ini line: (file, alias, offset, consonant, cutoff, preutterance, overlap).
pub fn parse_oto_line(line: &str) -> Option<(String, String, [f32; 5])> {
    let (file, rest) = line.trim().split_once('=')?;
    let mut parts = rest.split(',');
    let alias = parts.next()?.trim().to_string();
    let mut v = [0.0f32; 5];
    for x in v.iter_mut() {
        *x = parts.next().and_then(|s| s.trim().parse().ok()).unwrap_or(0.0);
    }
    let file = file.trim().to_string();
    if file.is_empty() {
        return None;
    }
    let alias = if alias.is_empty() { Path::new(&file).file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default() } else { alias };
    Some((file, alias, v))
}

/// A note's name (C4, A#3, ...) as a MIDI note.
fn note_number(name: &str) -> Option<u8> {
    let name = name.trim();
    let (letter, rest) = name.split_at(name.char_indices().nth(1).map(|(i, _)| i)?);
    let base = match letter.to_ascii_uppercase().as_str() {
        "C" => 0,
        "D" => 2,
        "E" => 4,
        "F" => 5,
        "G" => 7,
        "A" => 9,
        "B" => 11,
        _ => return None,
    };
    let (sharp, oct) = if let Some(r) = rest.strip_prefix('#') { (1, r) } else { (0, rest) };
    let octave: i32 = oct.parse().ok()?;
    u8::try_from((octave + 1) * 12 + base + sharp).ok()
}

static OPEN: Mutex<Vec<(PathBuf, Weak<VoicebankData>)>> = Mutex::new(Vec::new());

impl VoicebankRef {
    /// Reads a voicebank folder (or shares the one open), and starts finding
    /// its pitch periods in the background.
    pub fn load(dir: &Path) -> Result<Self, String> {
        if let Some(open) = OPEN.lock().ok().and_then(|o| o.iter().find(|(p, _)| p == dir).and_then(|(_, w)| w.upgrade())) {
            return Ok(VoicebankRef(open));
        }
        let data = read(dir)?;
        let r = VoicebankRef(Arc::new(data));
        r.analyse_in_background();
        if let Ok(mut open) = OPEN.lock() {
            open.retain(|(_, w)| w.strong_count() > 0);
            open.push((dir.to_path_buf(), Arc::downgrade(&r.0)));
        }
        Ok(r)
    }

    /// A voicebank made here (tests, and the analysis done at once).
    pub fn from_parts(name: &str, samples: Vec<Sample>, entries: Vec<OtoEntry>) -> Self {
        let aliases = entries.iter().enumerate().map(|(i, e)| (e.alias.clone(), i)).collect::<HashMap<_, _>>();
        let vcv = entries.iter().any(|e| e.alias.starts_with("- "));
        let gain = leveling(&samples);
        let d = VoicebankData { name: name.into(), path: None, samples, entries, aliases, prefix_map: Vec::new(), vcv, gain, analysis: OnceLock::new(), progress: AtomicU32::new(0) };
        let marks = d.samples.iter().map(analyse).collect();
        let _ = d.analysis.set(marks);
        VoicebankRef(Arc::new(d))
    }

    /// A voicebank whose folder could not be read: silent, its path kept.
    pub fn missing(name: String, path: Option<PathBuf>) -> Self {
        let d = VoicebankData { name, path, samples: Vec::new(), entries: Vec::new(), aliases: HashMap::new(), prefix_map: Vec::new(), vcv: false, gain: 1.0, analysis: OnceLock::new(), progress: AtomicU32::new(0) };
        let _ = d.analysis.set(Vec::new());
        VoicebankRef(Arc::new(d))
    }

    pub fn is_missing(&self) -> bool {
        self.entries.is_empty()
    }

    /// How far the analysis is (0..1).
    pub fn ready(&self) -> f32 {
        if self.analysis.get().is_some() {
            1.0
        } else {
            self.progress.load(Ordering::Relaxed) as f32 / self.samples.len().max(1) as f32
        }
    }

    fn analyse_in_background(&self) {
        let d = self.0.clone();
        std::thread::spawn(move || {
            let n = d.samples.len();
            let threads = std::thread::available_parallelism().map(|t| t.get()).unwrap_or(2).clamp(1, 8);
            let mut out: Vec<Marks> = vec![Marks::default(); n];
            std::thread::scope(|s| {
                let chunk = n.div_ceil(threads).max(1);
                for (part, samples) in out.chunks_mut(chunk).zip(d.samples.chunks(chunk)) {
                    let progress = &d.progress;
                    s.spawn(move || {
                        for (m, sample) in part.iter_mut().zip(samples) {
                            *m = analyse(sample);
                            progress.fetch_add(1, Ordering::Relaxed);
                        }
                    });
                }
            });
            let _ = d.analysis.set(out);
        });
    }

    /// The alias for `lyric` at MIDI note `note`, after the vowel `prev`
    /// (None at a phrase's start): its oto entry.
    pub fn find(&self, lyric: &str, note: u8, prev: Option<char>) -> Option<&OtoEntry> {
        let (prefix, suffix) = self.prefix_map.iter().find(|(n, _, _)| *n == note).map(|(_, p, s)| (p.as_str(), s.as_str())).unwrap_or(("", ""));
        let kana = super::kana::to_kana(lyric);
        let mut tries: Vec<String> = Vec::with_capacity(8);
        for l in [kana.as_str(), lyric.trim()] {
            if self.vcv {
                match prev {
                    Some(v) => tries.push(format!("{} {l}", super::kana::vowel_name(v))),
                    None => tries.push(format!("- {l}")),
                }
                tries.push(format!("- {l}"));
            }
            tries.push(l.to_string());
        }
        for t in &tries {
            for key in [format!("{prefix}{t}{suffix}"), t.clone()] {
                if let Some(&i) = self.aliases.get(&key) {
                    return self.entries.get(i);
                }
            }
        }
        None
    }
}

/// Reads a folder: every oto.ini in it and its subfolders, their recordings.
fn read(dir: &Path) -> Result<VoicebankData, String> {
    let mut otos = Vec::new();
    if dir.join("oto.ini").is_file() {
        otos.push(dir.join("oto.ini"));
    }
    if let Ok(rd) = std::fs::read_dir(dir) {
        let mut subs: Vec<PathBuf> = rd.flatten().map(|e| e.path()).filter(|p| p.is_dir() && p.join("oto.ini").is_file()).collect();
        subs.sort();
        otos.extend(subs.into_iter().map(|p| p.join("oto.ini")));
    }
    let name_of = |d: &Path| d.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "Voicebank".into());
    if otos.is_empty() {
        return Err(format!("{}: no oto.ini in it (a UTAU voicebank's folder has one)", name_of(dir)));
    }
    let mut samples: Vec<Sample> = Vec::new();
    let mut by_path: HashMap<PathBuf, usize> = HashMap::new();
    let mut entries = Vec::new();
    for oto in &otos {
        let folder = oto.parent().unwrap_or(dir);
        let text = decode(&std::fs::read(oto).map_err(|e| format!("{}: {e}", oto.display()))?);
        for line in text.lines() {
            let Some((file, alias, v)) = parse_oto_line(line) else { continue };
            let path = folder.join(&file);
            let sample = match by_path.get(&path) {
                Some(&i) => i,
                None => match load_wav(&path) {
                    Some(s) => {
                        samples.push(s);
                        by_path.insert(path.clone(), samples.len() - 1);
                        samples.len() - 1
                    }
                    None => continue,
                },
            };
            entries.push(OtoEntry { alias, sample, offset: v[0], consonant: v[1], cutoff: v[2], preutter: v[3], overlap: v[4] });
        }
    }
    if entries.is_empty() {
        return Err(format!("{}: its oto.ini names no recordings that are there", name_of(dir)));
    }
    // The first of an alias wins (the main folder before the pitch folders).
    let mut aliases = HashMap::new();
    for (i, e) in entries.iter().enumerate() {
        aliases.entry(e.alias.clone()).or_insert(i);
    }
    let vcv = entries.iter().any(|e| e.alias.starts_with("- "));
    let mut prefix_map = Vec::new();
    if let Ok(b) = std::fs::read(dir.join("prefix.map")) {
        for line in decode(&b).lines() {
            let mut f = line.split('\t');
            if let (Some(n), Some(p), Some(s)) = (f.next(), f.next(), f.next()) {
                if let Some(n) = note_number(n) {
                    prefix_map.push((n, p.to_string(), s.trim_end().to_string()));
                }
            }
        }
    }
    let name = std::fs::read(dir.join("character.txt"))
        .ok()
        .and_then(|b| decode(&b).lines().find_map(|l| l.strip_prefix("name=").map(|n| n.trim().to_string())))
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| name_of(dir));
    let gain = leveling(&samples);
    Ok(VoicebankData { name, path: Some(dir.to_path_buf()), samples, entries, aliases, prefix_map, vcv, gain, analysis: OnceLock::new(), progress: AtomicU32::new(0) })
}

/// How much to turn a voicebank up or down: its sung parts (the louder 10 ms
/// stretches of its recordings) brought to about -12 dBFS.
fn leveling(samples: &[Sample]) -> f32 {
    let mut windows: Vec<f32> = Vec::new();
    for s in samples {
        let w = (s.sr as usize / 100).max(1);
        for chunk in s.data.chunks(w) {
            let rms = (chunk.iter().map(|&x| (x as f32 / 32768.0).powi(2)).sum::<f32>() / chunk.len() as f32).sqrt();
            if rms > 0.003 {
                windows.push(rms);
            }
        }
    }
    if windows.is_empty() {
        return 1.0;
    }
    let k = (windows.len() * 4 / 5).min(windows.len() - 1);
    let (_, loud, _) = windows.select_nth_unstable_by(k, |a, b| a.total_cmp(b));
    (0.25 / *loud).clamp(0.5, 8.0)
}

/// A recording, mono, as 16-bit.
fn load_wav(path: &Path) -> Option<Sample> {
    let mut r = hound::WavReader::open(path).ok()?;
    let spec = r.spec();
    let ch = spec.channels.max(1) as usize;
    let frames: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Int => {
            let scale = 1.0 / (1u64 << (spec.bits_per_sample.clamp(8, 32) - 1)) as f32;
            r.samples::<i32>().filter_map(Result::ok).map(|v| v as f32 * scale).collect()
        }
        hound::SampleFormat::Float => r.samples::<f32>().filter_map(Result::ok).collect(),
    };
    let data = frames.chunks(ch).map(|c| ((c.iter().sum::<f32>() / ch as f32).clamp(-1.0, 1.0) * 32767.0) as i16).collect();
    Some(Sample { name: path.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default(), sr: spec.sample_rate.max(8000), data })
}

/// Finds a recording's pitch periods: the pitch every 10 ms (normalised
/// autocorrelation on a decimated copy, 65 to 1000 Hz), then a mark on the
/// strongest peak of every period; unvoiced stretches get a mark every 5 ms.
pub fn analyse(s: &Sample) -> Marks {
    let sr = s.sr as f32;
    let x: Vec<f32> = s.data.iter().map(|v| *v as f32 / 32768.0).collect();
    let n = x.len();
    let mut marks = Marks::default();
    if n < 64 {
        return marks;
    }
    let dec = ((sr / 7000.0).round() as usize).max(1);
    let y: Vec<f32> = x.chunks(dec).map(|c| c.iter().sum::<f32>() / c.len() as f32).collect();
    let dsr = sr / dec as f32;
    let (tmin, tmax) = ((dsr / 1000.0).floor().max(2.0) as usize, (dsr / 65.0).ceil() as usize);
    let win = (0.025 * dsr) as usize;
    let hop = (0.01 * sr) as usize;
    let frames = n / hop + 1;
    // The pitch period (in sample frames) of every frame, 0 where unvoiced.
    let mut period = vec![0.0f32; frames];
    let mut rs = vec![0.0f32; tmax + 1];
    for (f, p) in period.iter_mut().enumerate() {
        let c = (f * hop) / dec;
        if c < win / 2 || c + win / 2 + tmax >= y.len() {
            continue;
        }
        let a = &y[c - win / 2..c + win / 2];
        let energy: f32 = a.iter().map(|v| v * v).sum();
        if energy / (win as f32) < 1e-6 {
            continue;
        }
        let mut best_r = 0.0f32;
        for tau in tmin..tmax {
            let b = &y[c - win / 2 + tau..c + win / 2 + tau];
            let (mut xy, mut yy) = (0.0f32, 0.0f32);
            for (u, v) in a.iter().zip(b) {
                xy += u * v;
                yy += v * v;
            }
            rs[tau] = xy / (energy * yy).sqrt().max(1e-12);
            best_r = best_r.max(rs[tau]);
        }
        // The shortest period nearly as strong as the strongest (twice the period is as alike).
        let best = (tmin + 1..tmax - 1).find(|&t| rs[t] >= 0.88 * best_r && rs[t] >= rs[t - 1] && rs[t] >= rs[t + 1]).unwrap_or(0);
        if best_r > 0.6 && best > 0 {
            *p = (best * dec) as f32;
        }
    }
    // Smooth over a frame either side: no lone octave jumps.
    let raw = period.clone();
    for f in 1..frames.saturating_sub(1) {
        let mut w = [raw[f - 1], raw[f], raw[f + 1]];
        w.sort_by(|a, b| a.total_cmp(b));
        if raw[f] > 0.0 {
            period[f] = if w[1] > 0.0 { w[1] } else { raw[f] };
        }
    }
    let unvoiced = ((0.005 * sr) as usize).max(8);
    let mut pos = 0usize;
    let mut last_voiced = false;
    while pos < n {
        let f = (pos / hop).min(frames - 1);
        let p = period[f] as usize;
        if p >= 8 {
            // The strongest peak where the next period should start.
            let (lo, hi) = if last_voiced { (pos + p - p / 4, (pos + p + p / 4).min(n)) } else { (pos, (pos + p).min(n)) };
            if lo >= n {
                break;
            }
            let m = (lo..hi).max_by(|&a, &b| x[a].total_cmp(&x[b])).unwrap_or(lo);
            let step = if marks.pos.last().is_some_and(|&l| (m as u32) > l) && last_voiced { m - *marks.pos.last().unwrap() as usize } else { p };
            marks.pos.push(m as u32);
            marks.period.push(step.clamp(8, u16::MAX as usize) as u16);
            marks.voiced.push(true);
            pos = m.max(pos + 1);
            last_voiced = true;
        } else {
            let m = pos + unvoiced;
            if m >= n {
                break;
            }
            marks.pos.push(m as u32);
            marks.period.push(unvoiced as u16);
            marks.voiced.push(false);
            pos = m;
            last_voiced = false;
        }
    }
    marks
}

impl serde::Serialize for VoicebankRef {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        #[derive(serde::Serialize)]
        struct OnDisk<'a> {
            name: &'a str,
            path: &'a Option<PathBuf>,
        }
        OnDisk { name: &self.name, path: &self.path }.serialize(s)
    }
}

impl<'de> serde::Deserialize<'de> for VoicebankRef {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(serde::Deserialize)]
        struct OnDisk {
            name: String,
            path: Option<PathBuf>,
        }
        let disk = OnDisk::deserialize(d)?;
        let Some(path) = disk.path else {
            return Ok(VoicebankRef::missing(disk.name, None));
        };
        Ok(match VoicebankRef::load(&path) {
            Ok(r) => r,
            Err(e) => {
                crate::sample::note_missing(e);
                VoicebankRef::missing(disk.name, Some(path))
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oto_lines_and_shift_jis_are_read() {
        let (file, alias, v) = parse_oto_line("_あかさ.wav=- か,1200.5,80,-300,60,20").unwrap();
        assert_eq!((file.as_str(), alias.as_str()), ("_あかさ.wav", "- か"));
        assert_eq!(v, [1200.5, 80.0, -300.0, 60.0, 20.0]);
        // No alias: the file's name.
        assert_eq!(parse_oto_line("ka.wav=,10,50,-200,30,10").unwrap().1, "ka");
        // Shift-JIS "か" is 0x82 0xA9.
        assert_eq!(decode(&[0x82, 0xA9, b'=', b'x']), "か=x");
        assert_eq!(decode("か".as_bytes()), "か");
        assert_eq!(note_number("C4"), Some(60));
        assert_eq!(note_number("A#3"), Some(58));
    }

    #[test]
    fn quiet_and_loud_voicebanks_sing_as_loud() {
        let sung = |amp: f32| {
            let data: Vec<i16> = (0..44_100).map(|i| ((i as f32 * 0.0627).sin() * amp) as i16).collect();
            let s = Sample { name: "a.wav".into(), sr: 44_100, data };
            let rms = (s.data.iter().map(|&x| (x as f32 / 32768.0).powi(2)).sum::<f32>() / s.data.len() as f32).sqrt();
            rms * leveling(&[s])
        };
        let (quiet, loud) = (sung(1500.0), sung(16_000.0));
        assert!((quiet / loud - 1.0).abs() < 0.05, "{quiet} vs {loud}");
        assert!((0.2..0.3).contains(&loud), "about -12 dBFS: {loud}");
    }

    #[test]
    fn a_voice_s_periods_are_found() {
        // Two seconds: 220 Hz with a few harmonics, then noise.
        let sr = 44_100u32;
        let mut data = Vec::new();
        for i in 0..sr {
            let t = i as f32 / sr as f32;
            let v = (std::f32::consts::TAU * 220.0 * t).sin() + 0.5 * (std::f32::consts::TAU * 440.0 * t).sin() + 0.25 * (std::f32::consts::TAU * 660.0 * t).sin();
            data.push((v * 9000.0) as i16);
        }
        let mut seed = 1u32;
        for _ in 0..sr {
            seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12345);
            data.push(((seed >> 16) as i16 >> 3) as i16);
        }
        let m = analyse(&Sample { name: "t".into(), sr, data });
        let voiced: Vec<u16> = m.period.iter().zip(&m.voiced).filter(|(_, v)| **v).map(|(p, _)| *p).skip(5).take(100).collect();
        assert!(voiced.len() > 50);
        let mean = voiced.iter().map(|p| *p as f32).sum::<f32>() / voiced.len() as f32;
        assert!((mean - 200.45).abs() < 4.0, "a 220 Hz period is about 200 frames: {mean}");
        let late = m.pos.iter().zip(&m.voiced).filter(|(p, _)| **p > sr + 4410).collect::<Vec<_>>();
        assert!(late.iter().filter(|(_, v)| **v).count() * 4 < late.len(), "the noise is mostly unvoiced");
    }
}

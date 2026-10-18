//! SF2 Player: plays the presets of SoundFont 2 banks (.sf2 files) through
//! rustysynth (MIT, Nobuaki Tanaka, <https://github.com/sinshu/rustysynth>).
//!
//! A bank is read once on the interface thread and shared (`Sf2Ref`); the
//! audio thread plays it with a synthesizer of its own that allocates
//! nothing while it plays.

use crate::params::{ParamSpec, Unit};
use rustysynth::{SoundFont, Synthesizer, SynthesizerSettings};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, Weak};

pub const VOLUME: usize = 0;
pub const PAN: usize = 1;
pub const REVERB: usize = 2;
pub const CHORUS: usize = 3;
pub const VIBRATO: usize = 4;
pub const VELOCITY: usize = 5;
pub const BEND_RANGE: usize = 6;
pub const TRANSPOSE: usize = 7;

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::lin("Volume", "Channel", -36.0, 6.0, 0.0, Unit::Db),
    ParamSpec::lin("Pan", "Channel", -1.0, 1.0, 0.0, Unit::Pan),
    ParamSpec::lin("Reverb", "Channel", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::lin("Chorus", "Channel", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Vibrato", "Play", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Velocity", "Play", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::int("Bend Range", "Play", 0.0, 24.0, 2.0, Unit::Semi),
    ParamSpec::int("Transpose", "Play", -24.0, 24.0, 0.0, Unit::Semi),
];

/// The sixteen families of General MIDI's 128 programs, eight each.
pub const GM_FAMILIES: [&str; 16] = [
    "Piano", "Chromatic Percussion", "Organ", "Guitar", "Bass", "Strings", "Ensemble", "Brass", "Reed", "Pipe", "Synth Lead", "Synth Pad", "Synth Effects", "Ethnic", "Percussive", "Sound Effects",
];

/// Banks from 128 up hold drum kits.
pub const DRUM_BANK: u16 = 128;

/// The notes of General MIDI's drum kits (bit per note), so drum
/// sequencers know what a kit plays and the key leaves it alone.
pub const GM_DRUM_NOTES: u128 = {
    let mut m = 0u128;
    let mut n = 27;
    while n <= 87 {
        m |= 1u128 << n;
        n += 1;
    }
    m
};

/// A preset of a bank: its name and where it sits (bank, program).
#[derive(Clone, Debug, PartialEq)]
pub struct Sf2Preset {
    pub name: String,
    pub bank: u16,
    pub patch: u8,
}

impl Sf2Preset {
    pub fn is_drums(&self) -> bool {
        self.bank >= DRUM_BANK
    }

    /// "000:001 Bright Piano".
    pub fn label(&self) -> String {
        format!("{:03}:{:03} {}", self.bank, self.patch + 1, self.name)
    }
}

/// A bank read from disk (or missing: no font, no presets, the path kept).
pub struct Sf2Data {
    pub name: String,
    pub path: Option<PathBuf>,
    pub font: Option<Arc<SoundFont>>,
    /// In order of bank and program.
    pub presets: Vec<Sf2Preset>,
}

impl Sf2Data {
    fn new(name: String, path: Option<PathBuf>, font: SoundFont) -> Self {
        let mut presets: Vec<Sf2Preset> = font
            .get_presets()
            .iter()
            .map(|p| Sf2Preset { name: p.get_name().trim().to_string(), bank: p.get_bank_number().clamp(0, u16::MAX as i32) as u16, patch: p.get_patch_number().clamp(0, 127) as u8 })
            .collect();
        presets.sort_by_key(|p| (p.bank, p.patch));
        presets.dedup_by_key(|p| (p.bank, p.patch));
        Self { name, path, font: Some(Arc::new(font)), presets }
    }
}

/// A shared bank. Equality is identity, so comparing projects never walks
/// the samples; it is saved as its name and path.
#[derive(Clone)]
pub struct Sf2Ref(pub Arc<Sf2Data>);

impl std::ops::Deref for Sf2Ref {
    type Target = Sf2Data;
    fn deref(&self) -> &Sf2Data {
        &self.0
    }
}

impl PartialEq for Sf2Ref {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl fmt::Debug for Sf2Ref {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Sf2({}, {} presets)", self.name, self.presets.len())
    }
}

/// The banks read, by path, while something holds them: opening the same
/// file again (another track, undo, a project) shares it.
static OPEN: Mutex<Vec<(PathBuf, Weak<Sf2Data>)>> = Mutex::new(Vec::new());

fn file_name(path: &Path) -> String {
    path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "Bank".into())
}

impl Sf2Ref {
    /// Reads a .sf2 file (or shares the one already open).
    pub fn load(path: &Path) -> Result<Self, String> {
        if let Some(open) = OPEN.lock().ok().and_then(|o| o.iter().find(|(p, _)| p == path).and_then(|(_, w)| w.upgrade())) {
            return Ok(Sf2Ref(open));
        }
        let name = file_name(path);
        let file = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let font = SoundFont::new(&mut std::io::BufReader::new(file)).map_err(|e| format!("{name}: not a SoundFont 2 bank this player reads ({e})"))?;
        let r = Sf2Ref(Arc::new(Sf2Data::new(name, Some(path.to_path_buf()), font)));
        if let Ok(mut open) = OPEN.lock() {
            open.retain(|(_, w)| w.strong_count() > 0);
            open.push((path.to_path_buf(), Arc::downgrade(&r.0)));
        }
        Ok(r)
    }

    /// A bank from its bytes (tests, and banks not on disk).
    pub fn from_bytes(name: &str, path: Option<PathBuf>, bytes: &[u8]) -> Result<Self, String> {
        let font = SoundFont::new(&mut std::io::Cursor::new(bytes)).map_err(|e| format!("{name}: not a SoundFont 2 bank ({e})"))?;
        Ok(Sf2Ref(Arc::new(Sf2Data::new(name.to_string(), path, font))))
    }

    /// A bank whose file could not be read: silent, its path kept.
    pub fn missing(name: String, path: Option<PathBuf>) -> Self {
        Sf2Ref(Arc::new(Sf2Data { name, path, font: None, presets: Vec::new() }))
    }

    pub fn is_missing(&self) -> bool {
        self.font.is_none()
    }

    pub fn preset(&self, bank: u16, patch: u8) -> Option<&Sf2Preset> {
        self.presets.iter().find(|p| p.bank == bank && p.patch == patch)
    }

    /// The preset for a General MIDI program: (bank 0 or the lowest bank
    /// that has it), or for drums the first kit.
    pub fn gm(&self, program: u8, drums: bool) -> Option<&Sf2Preset> {
        if drums {
            return self.presets.iter().find(|p| p.is_drums() && p.patch == 0).or_else(|| self.presets.iter().find(|p| p.is_drums()));
        }
        self.presets.iter().find(|p| !p.is_drums() && p.patch == program)
    }

    /// Whether it is a General MIDI set: (nearly) all 128 programs in bank 0.
    pub fn is_gm(&self) -> bool {
        self.presets.iter().filter(|p| p.bank == 0).count() >= 120
    }
}

#[derive(Serialize, Deserialize)]
struct Sf2OnDisk {
    name: String,
    path: Option<PathBuf>,
}

impl Serialize for Sf2Ref {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        Sf2OnDisk { name: self.name.clone(), path: self.path.clone() }.serialize(s)
    }
}

impl<'de> Deserialize<'de> for Sf2Ref {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let disk = Sf2OnDisk::deserialize(d)?;
        let Some(path) = disk.path else {
            return Ok(Sf2Ref::missing(disk.name, None));
        };
        Ok(match Sf2Ref::load(&path) {
            Ok(r) => r,
            Err(e) => {
                crate::sample::note_missing(e);
                Sf2Ref::missing(disk.name, Some(path))
            }
        })
    }
}

/// The player on the audio thread: a synthesizer for one bank, playing on
/// MIDI channel 1 the preset the instrument names.
pub struct Sf2State {
    font: Option<Arc<SoundFont>>,
    synth: Option<Synthesizer>,
    /// The key each played note sounds (after Transpose), -1: none.
    keys: [i16; 128],
    program: Option<(u16, u8)>,
    /// What the synthesizer was last told: volume, the controllers, the bend.
    volume: f32,
    sent: [i32; 6],
    pub pitch_bend: f32,
    pub sustain: bool,
    left: Vec<f32>,
    right: Vec<f32>,
}

const SENT_PAN: usize = 0;
const SENT_REVERB: usize = 1;
const SENT_CHORUS: usize = 2;
const SENT_VIBRATO: usize = 3;
const SENT_RANGE: usize = 4;
const SENT_BEND: usize = 5;

impl Sf2State {
    pub fn new(font: Option<&Sf2Ref>, sr: f32) -> Self {
        let font = font.and_then(|f| f.font.clone());
        let synth = font.as_ref().and_then(|f| {
            let mut settings = SynthesizerSettings::new((sr.round() as i32).clamp(16_000, 192_000));
            settings.maximum_polyphony = 128;
            Synthesizer::new(f, &settings).ok()
        });
        Self { font, synth, keys: [-1; 128], program: None, volume: -1.0, sent: [-1; 6], pitch_bend: 0.0, sustain: false, left: vec![0.0; 1024], right: vec![0.0; 1024] }
    }

    /// Whether it plays this bank (another bank needs another player).
    pub fn is_for(&self, font: Option<&Sf2Ref>) -> bool {
        match (&self.font, font.and_then(|f| f.font.as_ref())) {
            (Some(a), Some(b)) => Arc::ptr_eq(a, b),
            (None, None) => true,
            _ => false,
        }
    }

    /// Tells the synthesizer what changed: the preset, the controllers.
    fn update(&mut self, p: &[f32], bank: u16, patch: u8) {
        let Some(s) = self.synth.as_mut() else { return };
        if self.program != Some((bank, patch)) {
            s.process_midi_message(0, 0xB0, 0x00, bank as i32);
            s.process_midi_message(0, 0xC0, patch as i32, 0);
            self.program = Some((bank, patch));
        }
        let get = |i: usize| p.get(i).copied().unwrap_or(SPECS[i].default);
        let volume = 0.5 * 10f32.powf(get(VOLUME) / 20.0);
        if volume != self.volume {
            s.set_master_volume(volume);
            self.volume = volume;
        }
        let cc = |v: f32| (v * 127.0).round().clamp(0.0, 127.0) as i32;
        for (k, num, v) in [(SENT_PAN, 0x0A, cc((get(PAN) + 1.0) * 0.5)), (SENT_REVERB, 0x5B, cc(get(REVERB))), (SENT_CHORUS, 0x5D, cc(get(CHORUS))), (SENT_VIBRATO, 0x01, cc(get(VIBRATO)))] {
            if self.sent[k] != v {
                s.process_midi_message(0, 0xB0, num, v);
                self.sent[k] = v;
            }
        }
        let range = get(BEND_RANGE).round().clamp(0.0, 24.0) as i32;
        if self.sent[SENT_RANGE] != range {
            // Registered parameter 0, the bend range, then none selected.
            for (num, v) in [(0x65, 0), (0x64, 0), (0x06, range), (0x26, 0), (0x65, 127), (0x64, 127)] {
                s.process_midi_message(0, 0xB0, num, v);
            }
            self.sent[SENT_RANGE] = range;
        }
        let bend = if range == 0 { 8192 } else { (8192.0 + self.pitch_bend / range as f32 * 8192.0).round().clamp(0.0, 16383.0) as i32 };
        if self.sent[SENT_BEND] != bend {
            s.process_midi_message(0, 0xE0, bend & 0x7F, bend >> 7);
            self.sent[SENT_BEND] = bend;
        }
    }

    pub fn set_sustain(&mut self, down: bool) {
        self.sustain = down;
        if let Some(s) = self.synth.as_mut() {
            s.process_midi_message(0, 0xB0, 0x40, if down { 127 } else { 0 });
        }
    }

    pub fn note_on(&mut self, p: &[f32], bank: u16, patch: u8, pitch: u8, vel: f32) {
        self.update(p, bank, patch);
        let Some(s) = self.synth.as_mut() else { return };
        let shift = if bank >= DRUM_BANK { 0 } else { p.get(TRANSPOSE).copied().unwrap_or(0.0).round() as i32 };
        let key = (pitch as i32 + shift).clamp(0, 127);
        let sens = p.get(VELOCITY).copied().unwrap_or(1.0).clamp(0.0, 1.0);
        let vel = 1.0 - sens * (1.0 - vel.clamp(0.0, 1.0));
        let at = pitch as usize & 127;
        if self.keys[at] >= 0 {
            s.note_off(0, self.keys[at] as i32);
        }
        s.note_on(0, key, (vel * 127.0).round().clamp(1.0, 127.0) as i32);
        self.keys[at] = key as i16;
    }

    pub fn note_off(&mut self, pitch: u8) {
        let at = pitch as usize & 127;
        if let (Some(s), k) = (self.synth.as_mut(), self.keys[at]) {
            if k >= 0 {
                s.note_off(0, k as i32);
            }
        }
        self.keys[at] = -1;
    }

    pub fn all_off(&mut self) {
        if let Some(s) = self.synth.as_mut() {
            s.note_off_all(false);
        }
        self.keys = [-1; 128];
    }

    /// Adds its sound to `l` and `r`.
    pub fn render(&mut self, p: &[f32], bank: u16, patch: u8, l: &mut [f32], r: &mut [f32]) {
        self.update(p, bank, patch);
        let Some(s) = self.synth.as_mut() else { return };
        let n = l.len().min(r.len());
        let mut at = 0;
        while at < n {
            let len = (n - at).min(self.left.len());
            s.render(&mut self.left[..len], &mut self.right[..len]);
            for k in 0..len {
                l[at + k] += self.left[k];
                r[at + k] += self.right[k];
            }
            at += len;
        }
    }
}

/// A small bank, made here: one looping sine played by every preset in
/// `presets` (name, bank, program).
#[cfg(test)]
pub(crate) fn test_bank(presets: &[(&str, u16, u8)]) -> Vec<u8> {
    fn chunk(id: &[u8; 4], body: &[u8]) -> Vec<u8> {
        let mut v = id.to_vec();
        v.extend_from_slice(&(body.len() as u32).to_le_bytes());
        v.extend_from_slice(body);
        if body.len() % 2 == 1 {
            v.push(0);
        }
        v
    }
    fn list(kind: &[u8; 4], chunks: &[Vec<u8>]) -> Vec<u8> {
        let mut body = kind.to_vec();
        for c in chunks {
            body.extend_from_slice(c);
        }
        chunk(b"LIST", &body)
    }
    fn name20(s: &str) -> Vec<u8> {
        let mut v = s.as_bytes().to_vec();
        v.resize(20, 0);
        v
    }
    let u16s = |v: &[u16]| v.iter().flat_map(|x| x.to_le_bytes()).collect::<Vec<u8>>();
    // A cycle of a sine every 100 frames at 44.1 kHz, then the 46 zeros the format wants.
    let frames = 400u32;
    let mut smpl: Vec<u8> = (0..frames).flat_map(|i| (((i as f32 / 100.0 * std::f32::consts::TAU).sin() * 20000.0) as i16).to_le_bytes()).collect();
    smpl.extend(std::iter::repeat_n(0u8, 46 * 2));
    let n = presets.len() as u16;
    let mut phdr = Vec::new();
    for (i, (name, bank, patch)) in presets.iter().enumerate() {
        phdr.extend(name20(name));
        phdr.extend(u16s(&[*patch as u16, *bank, i as u16]));
        phdr.extend([0u8; 12]);
    }
    phdr.extend(name20("EOP"));
    phdr.extend(u16s(&[0, 0, n]));
    phdr.extend([0u8; 12]);
    let mut pbag = Vec::new();
    for i in 0..=n {
        pbag.extend(u16s(&[i, 0]));
    }
    // Every preset zone: instrument 0 (generator 41).
    let mut pgen = Vec::new();
    for _ in 0..n {
        pgen.extend(u16s(&[41, 0]));
    }
    pgen.extend([0u8; 4]);
    let mut inst = name20("Sine");
    inst.extend(u16s(&[0]));
    inst.extend(name20("EOI"));
    inst.extend(u16s(&[1]));
    let ibag = u16s(&[0, 0, 2, 0]);
    // Loop continuously (54 = 1), sample 0 (53, last).
    let mut igen = u16s(&[54, 1, 53, 0]);
    igen.extend([0u8; 4]);
    let mut shdr = name20("Sine");
    shdr.extend([0u32, frames, 0, frames, 44_100].iter().flat_map(|x| x.to_le_bytes()));
    shdr.extend([69u8, 0]);
    shdr.extend(u16s(&[0, 1]));
    shdr.extend(name20("EOS"));
    shdr.extend([0u8; 26]);
    let info = list(b"INFO", &[chunk(b"ifil", &u16s(&[2, 1])), chunk(b"isng", b"EMU8000\0"), chunk(b"INAM", b"Test Bank\0")]);
    let sdta = list(b"sdta", &[chunk(b"smpl", &smpl)]);
    let pdta = list(
        b"pdta",
        &[chunk(b"phdr", &phdr), chunk(b"pbag", &pbag), chunk(b"pmod", &[0u8; 10]), chunk(b"pgen", &pgen), chunk(b"inst", &inst), chunk(b"ibag", &ibag), chunk(b"imod", &[0u8; 10]), chunk(b"igen", &igen), chunk(b"shdr", &shdr)],
    );
    let mut body = b"sfbk".to_vec();
    body.extend(info);
    body.extend(sdta);
    body.extend(pdta);
    chunk(b"RIFF", &body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::defaults;

    fn bank() -> Sf2Ref {
        Sf2Ref::from_bytes("Test", None, &test_bank(&[("Sine Lead", 0, 80), ("Sine Keys", 0, 0), ("Kit", 128, 0)])).expect("reads")
    }

    fn energy(st: &mut Sf2State, p: &[f32], bank: u16, patch: u8, frames: usize) -> f32 {
        let (mut l, mut r) = (vec![0.0; frames], vec![0.0; frames]);
        st.render(p, bank, patch, &mut l, &mut r);
        l.iter().chain(&r).map(|x| x * x).sum::<f32>() / frames as f32
    }

    #[test]
    fn a_bank_lists_its_presets_in_order() {
        let b = bank();
        let labels: Vec<String> = b.presets.iter().map(|p| p.label()).collect();
        assert_eq!(labels, ["000:001 Sine Keys", "000:081 Sine Lead", "128:001 Kit"]);
        assert_eq!(b.gm(80, false).map(|p| p.name.as_str()), Some("Sine Lead"));
        assert_eq!(b.gm(5, true).map(|p| p.name.as_str()), Some("Kit"));
        assert!(!b.is_gm());
    }

    #[test]
    fn a_note_sounds_and_stops() {
        let b = bank();
        let p = defaults(SPECS);
        let mut st = Sf2State::new(Some(&b), 48_000.0);
        assert!(st.is_for(Some(&b)) && !st.is_for(None));
        assert_eq!(energy(&mut st, &p, 0, 0, 2048), 0.0, "silent before a note");
        st.note_on(&p, 0, 0, 69, 1.0);
        let on = energy(&mut st, &p, 0, 0, 4096);
        assert!(on > 1e-4, "it plays ({on})");
        st.note_off(69);
        energy(&mut st, &p, 0, 0, 48_000 * 2);
        assert!(energy(&mut st, &p, 0, 0, 4096) < on * 0.01, "and stops");
        // Quieter with the volume down.
        let mut quiet = p.clone();
        quiet[VOLUME] = -20.0;
        quiet[REVERB] = 0.0;
        let mut loud = p.clone();
        loud[REVERB] = 0.0;
        let mut a = Sf2State::new(Some(&b), 48_000.0);
        let mut c = Sf2State::new(Some(&b), 48_000.0);
        a.note_on(&loud, 0, 0, 69, 1.0);
        c.note_on(&quiet, 0, 0, 69, 1.0);
        let (la, lc) = (energy(&mut a, &loud, 0, 0, 8192), energy(&mut c, &quiet, 0, 0, 8192));
        assert!(lc < la * 0.05, "{lc} vs {la}");
    }

    #[test]
    fn without_a_bank_it_is_silent() {
        let p = defaults(SPECS);
        let mut st = Sf2State::new(None, 48_000.0);
        st.note_on(&p, 0, 0, 60, 1.0);
        assert_eq!(energy(&mut st, &p, 0, 0, 1024), 0.0);
        assert!(st.is_for(None));
    }

    #[test]
    fn a_bank_is_saved_as_its_path_and_shared_when_opened_again() {
        let dir = std::env::temp_dir().join(format!("krabton-sf2-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("Test Bank.sf2");
        std::fs::write(&path, test_bank(&[("Sine", 0, 0)])).unwrap();
        let a = Sf2Ref::load(&path).expect("loads");
        assert_eq!(a.name, "Test Bank");
        let json = serde_json::to_string(&a).unwrap();
        assert!(json.contains("Test Bank.sf2") && !json.contains("Sine"), "{json}");
        let b: Sf2Ref = serde_json::from_str(&json).unwrap();
        assert_eq!(a, b, "the bank open is shared");
        drop((a, b));
        std::fs::remove_file(&path).unwrap();
        let gone: Sf2Ref = serde_json::from_str(&json).unwrap();
        assert!(gone.is_missing() && gone.path.as_deref() == Some(path.as_path()), "a missing bank keeps its path");
        let _ = std::fs::remove_dir(&dir);
    }
}

//! Airway: a voice made by simulating a throat and a mouth, not by playing
//! recordings: lungs push air through vocal folds into an airway whose
//! shape (jaw, tongue, lips, larynx, the soft palate) makes the vowels and
//! consonants. The engine is CHIMERA by Jameson Hodge (MIT, in
//! `krabton-chimera`); Krabton plays it from the keys in three ways:
//! - Vowels: each key sings a vowel at its pitch, morphing toward a second
//!   one, the mouth moved by Jaw, Tongue, Lips, Larynx and Nasal.
//! - Words: each key sings the next syllable of the lyric (English, from
//!   the CMU pronouncing dictionary when it is there, else from the
//!   spelling), held while the key is, its consonants when it is let go.
//! - Creature: a songbird's syrinx, a seal, an alligator or an elephant,
//!   each key a call at its pitch.
//!
//! The planners allocate, so what a lyric sings is planned off the audio
//! thread (`prepare`, kept in the instrument as `PlansRef`) at middle C and
//! moved to a key's pitch as it plays. One to four voices, each a whole
//! simulated airway (they cost a lot: start with one).

use chimera::anatomy::human::{ArticKey, ArticPatch, BodyMix, NEUTRAL};
use chimera::control::score::{ControlKey, Gesture};
use chimera::engine::Engine;
use chimera::phonetics::english::EnglishG2P;
use chimera::phonetics::planner::vowel_artic;
use chimera::phonetics::sing::{plan_note_off, plan_note_on, NoteOnOptions};
use chimera::phonetics::syllables::lyrics_to_syllables;
use chimera::voice::human_voice::SourcePatch;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::params::{ParamSpec, Unit, SCREEN_ONLY};

pub const MODE: usize = 0;
pub const VOICES: usize = 1;
pub const BODY: usize = 2;
pub const VOWEL: usize = 3;
pub const MORPH_TO: usize = 4;
pub const MORPH: usize = 5;
pub const GLIDE: usize = 6;
pub const PRESSURE: usize = 7;
pub const BREATH: usize = 8;
pub const TENSION: usize = 9;
pub const FRY: usize = 10;
pub const ROUGH: usize = 11;
pub const VIBRATO: usize = 12;
pub const VIB_RATE: usize = 13;
pub const BRIGHT: usize = 14;
pub const JAW: usize = 15;
pub const TONGUE: usize = 16;
pub const LIPS: usize = 17;
pub const LARYNX: usize = 18;
pub const NASAL: usize = 19;
pub const CREATURE: usize = 20;
pub const CALL: usize = 21;
pub const LEVEL: usize = 22;

pub const MODES: &[&str] = &["Vowels", "Words", "Creature"];
pub const VOICE_COUNTS: &[&str] = &["1", "2", "4"];
const VOICE_N: [usize; 3] = [1, 2, 4];
pub const BODIES: &[&str] = &["Male", "Female", "Child"];
/// The vowels, as the engine knows them (IPA), and as they are shown.
pub const VOWEL_IPA: [&str; 12] = ["ɑ", "æ", "ɛ", "e", "i", "ɪ", "ɔ", "o", "ʊ", "u", "ʌ", "ə"];
pub const VOWEL_NAMES: &[&str] = &["ah", "a", "eh", "ay", "ee", "ih", "aw", "oh", "oo", "ooh", "uh", "er"];
pub const CREATURES: &[&str] = &["Songbird", "Seal", "Alligator", "Elephant"];
const CREATURE_IDS: [&str; 4] = ["songbird", "seal", "alligator", "elephant"];
pub const MAX_VOICES: usize = 4;
/// Where a lyric is planned (a key's pitch moves it from here).
const REF_MIDI: f64 = 60.0;

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::choice("Mode", SCREEN_ONLY, MODES, 0),
    ParamSpec::choice("Voices", SCREEN_ONLY, VOICE_COUNTS, 0),
    ParamSpec::choice("Body", SCREEN_ONLY, BODIES, 0),
    ParamSpec::choice("Vowel", "Vowels", VOWEL_NAMES, 0),
    ParamSpec::choice("Morph To", "Vowels", VOWEL_NAMES, 9),
    ParamSpec::lin("Morph", "Vowels", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Glide", "Source", 0.005, 0.5, 0.06, Unit::Sec),
    ParamSpec::lin("Pressure", "Source", 2.0, 20.0, 8.0, Unit::None),
    ParamSpec::lin("Breath", "Source", 0.0, 1.0, 0.15, Unit::Percent),
    ParamSpec::lin("Tension", "Source", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Fry", "Source", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Rough", "Source", 0.0, 1.0, 0.1, Unit::Percent),
    ParamSpec::lin("Vibrato", "Source", 0.0, 100.0, 25.0, Unit::Cents),
    ParamSpec::lin("Vib Rate", "Source", 3.0, 9.0, 5.5, Unit::Hz),
    ParamSpec::lin("Bright", "Source", 0.0, 1.0, 0.25, Unit::Percent),
    ParamSpec::lin("Jaw", "Mouth", -0.4, 0.4, 0.0, Unit::None),
    ParamSpec::lin("Tongue", "Mouth", -1.0, 1.0, 0.0, Unit::None),
    ParamSpec::lin("Lips", "Mouth", -0.5, 0.5, 0.0, Unit::None),
    ParamSpec::lin("Larynx", "Mouth", -1.0, 1.0, 0.0, Unit::None),
    ParamSpec::lin("Nasal", "Mouth", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::choice("Creature", SCREEN_ONLY, CREATURES, 0),
    ParamSpec::int("Call", SCREEN_ONLY, 1.0, 8.0, 1.0, Unit::None),
    ParamSpec::lin("Level", "Out", -24.0, 12.0, 0.0, Unit::Db),
];

const PRESETS: &[(&str, &[(usize, f32)])] = &[
    ("Choir Ah", &[(VOICES, 2.0), (VIBRATO, 30.0)]),
    ("Talking Bass", &[(MORPH, 0.0), (VOWEL, 9.0), (MORPH_TO, 0.0), (GLIDE, 0.12), (TENSION, 0.7), (FRY, 0.2), (VIBRATO, 0.0)]),
    ("Sung Words", &[(MODE, 1.0), (BODY, 1.0), (VIBRATO, 35.0)]),
    ("Whisper", &[(BREATH, 1.0), (PRESSURE, 5.0), (TENSION, 0.1), (VIBRATO, 0.0)]),
    ("Throat Creature", &[(FRY, 0.8), (ROUGH, 0.8), (LARYNX, -0.8), (TENSION, 0.9), (VOWEL, 6.0), (VIBRATO, 0.0)]),
    ("Child Ooh", &[(BODY, 2.0), (VOWEL, 9.0), (VIBRATO, 20.0)]),
    ("Nasal Drone", &[(NASAL, 0.8), (VOWEL, 11.0), (GLIDE, 0.3), (VIBRATO, 10.0)]),
    ("Songbird", &[(MODE, 2.0), (CREATURE, 0.0)]),
    ("Seal", &[(MODE, 2.0), (CREATURE, 1.0)]),
    ("Elephant", &[(MODE, 2.0), (CREATURE, 3.0)]),
];

pub fn preset_names() -> &'static [&'static str] {
    const N: [&str; 10] = ["Choir Ah", "Talking Bass", "Sung Words", "Whisper", "Throat Creature", "Child Ooh", "Nasal Drone", "Songbird", "Seal", "Elephant"];
    &N
}

pub fn preset(name: &str) -> Vec<f32> {
    let mut p = crate::params::defaults(SPECS);
    if let Some((_, set)) = PRESETS.iter().find(|(n, _)| *n == name) {
        for &(k, v) in *set {
            p[k] = v;
        }
    }
    p
}

pub fn body_of(p: &[f32]) -> BodyMix {
    match p.get(BODY).map_or(0, |v| v.round() as usize) {
        1 => BodyMix::FEMALE,
        2 => BodyMix::CHILD,
        _ => BodyMix::MALE,
    }
}

// ---------------------------------------------------------------- the lyric, planned off the audio thread

/// A syllable of the lyric: what it shows, its key press and its release, planned at middle C.
pub struct SylPlan {
    pub text: String,
    pub on: Vec<Gesture>,
    pub off: Vec<Gesture>,
}

/// What a lyric sings for a body, and the vowels' mouths for it.
pub struct Plans {
    pub text: String,
    pub body: u8,
    pub syls: Vec<SylPlan>,
    pub vowels: Vec<ArticPatch>,
}

/// Plans shared with the audio thread; equal when they are the same.
#[derive(Clone)]
pub struct PlansRef(pub Arc<Plans>);

impl std::ops::Deref for PlansRef {
    type Target = Plans;
    fn deref(&self) -> &Plans {
        &self.0
    }
}

impl PartialEq for PlansRef {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl std::fmt::Debug for PlansRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AirwayPlans({:?}, {} syllables)", self.text, self.syls.len())
    }
}

impl Default for PlansRef {
    fn default() -> Self {
        prepare("", 0)
    }
}

/// Where the pronouncing dictionary is kept: Documents/Krabton/Library/CMUdict.
pub fn dict_path() -> PathBuf {
    let home = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")).map(PathBuf::from).unwrap_or_default();
    home.join("Documents").join("Krabton").join("Library").join("CMUdict").join("cmudict.dict")
}

/// Its source: the CMU pronouncing dictionary (BSD-style), at a reviewed commit.
pub const DICT_URL: &str = "https://raw.githubusercontent.com/cmusphinx/cmudict/74790861f652b15e4ac49015a90074ad62a27690/cmudict.dict";

static G2P: Mutex<Option<Arc<EnglishG2P>>> = Mutex::new(None);

/// The English pronouncer: with the dictionary when it is on this computer (read once).
fn g2p() -> Arc<EnglishG2P> {
    let mut g = G2P.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(x) = g.as_ref() {
        return x.clone();
    }
    let mut e = EnglishG2P::new();
    if let Ok(text) = std::fs::read_to_string(dict_path()) {
        // "word PH ON ES" (alternates "word(2)", comments after #) as the engine reads it: "WORD\tPH ON ES".
        let mut out = String::with_capacity(text.len());
        for line in text.lines() {
            let line = line.split('#').next().unwrap_or("").trim();
            let Some((w, ph)) = line.split_once(' ') else { continue };
            if w.contains('(') {
                continue;
            }
            out.push_str(w);
            out.push('\t');
            out.push_str(ph.trim());
            out.push('\n');
        }
        e.load_dict(&out);
    }
    let e = Arc::new(e);
    *g = Some(e.clone());
    e
}

/// Whether the dictionary is in use.
pub fn has_dictionary() -> bool {
    g2p().is_loaded()
}

/// Reads the dictionary again (after it was downloaded).
pub fn reload_dictionary() {
    *G2P.lock().unwrap_or_else(|e| e.into_inner()) = None;
}

/// Plans lyric `text` for body `body` (0 male, 1 female, 2 child): every
/// syllable's press and release at middle C, and the vowels' mouths.
pub fn prepare(text: &str, body: u8) -> PlansRef {
    chimera::init();
    let mix = match body {
        1 => BodyMix::FEMALE,
        2 => BodyMix::CHILD,
        _ => BodyMix::MALE,
    };
    let syls = if text.trim().is_empty() { Vec::new() } else { lyrics_to_syllables(text, &*g2p()) };
    let syls = syls
        .iter()
        .map(|s| {
            let on = plan_note_on(s, REF_MIDI, &NoteOnOptions { body: mix, vibrato: 1.0, tag: 0.0, legato_from: None });
            let off = plan_note_off(s, REF_MIDI, mix);
            SylPlan { text: s.text.clone(), on, off }
        })
        .collect();
    let vowels = VOWEL_IPA.iter().map(|v| vowel_artic(v, mix).unwrap_or_default()).collect();
    PlansRef(Arc::new(Plans { text: text.to_string(), body, syls, vowels }))
}

// ---------------------------------------------------------------- playing

#[derive(Clone, Copy, Debug, Default)]
struct Voice {
    note: Option<u8>,
    /// When it was started (for stealing the oldest), its tags (Words), its syllable.
    age: u64,
    tag: f64,
    syl: usize,
    /// Samples since it went quiet (it stops being rendered a while after).
    quiet: usize,
    on: bool,
}

pub struct AirwayState {
    sr: f32,
    engines: Vec<Engine>,
    voices: [Voice; MAX_VOICES],
    /// The body and creature the engines are now (changing them builds a new airway).
    body: u8,
    creature: Option<usize>,
    mode: usize,
    next_syl: usize,
    clock: u64,
    tags: f64,
    scratch: Vec<Gesture>,
    buf: Vec<f32>,
    last: Vec<f32>,
    pub pitch_bend: f32,
    bend_was: f32,
    /// For the face: each voice's level, its note, the syllable sung last.
    meters: [f32; MAX_VOICES],
    pub sung: usize,
}

impl AirwayState {
    pub fn new(sr: f32) -> Self {
        chimera::init();
        let engines = (0..MAX_VOICES).map(|_| Engine::new(sr as f64)).collect();
        Self {
            sr,
            engines,
            voices: [Voice::default(); MAX_VOICES],
            body: 0,
            creature: None,
            mode: 0,
            next_syl: 0,
            clock: 0,
            tags: 2.0,
            scratch: Vec::with_capacity(chimera::engine::MAX_SCHEDULE),
            buf: vec![0.0; 8192],
            last: Vec::with_capacity(SPECS.len()),
            pitch_bend: 0.0,
            bend_was: 0.0,
            meters: [0.0; MAX_VOICES],
            sung: 0,
        }
    }

    fn voices_n(p: &[f32]) -> usize {
        VOICE_N[(p[VOICES].round().max(0.0) as usize).min(2)]
    }

    /// The engines as the settings want them: the body, the creature (each change builds a new airway).
    fn fit(&mut self, p: &[f32]) {
        let mode = (p[MODE].round().max(0.0) as usize).min(2);
        let body = (p[BODY].round().max(0.0) as u8).min(2);
        let creature = (mode == 2).then(|| (p[CREATURE].round().max(0.0) as usize).min(3));
        if body != self.body {
            self.body = body;
            for e in &mut self.engines {
                e.set_body(body_of(p));
            }
        }
        if creature != self.creature || mode != self.mode {
            self.creature = creature;
            for e in &mut self.engines {
                e.set_creature(creature.and_then(|c| chimera::creatures::creature_by_id(CREATURE_IDS[c])), &[]);
            }
            for v in &mut self.voices {
                v.note = None;
            }
        }
        self.mode = mode;
    }

    /// The source and the mouth from the settings, on every voice (when they changed).
    fn apply(&mut self, p: &[f32], plans: &Plans, force: bool) {
        if !force && self.last.as_slice() == p {
            return;
        }
        self.last.clear();
        self.last.extend_from_slice(&p[..p.len().min(self.last.capacity())]);
        let src = SourcePatch {
            glide: Some(p[GLIDE] as f64),
            pressure: Some(p[PRESSURE] as f64),
            breathiness: Some(p[BREATH] as f64),
            tension: Some(p[TENSION] as f64),
            fry: Some(p[FRY] as f64),
            roughness: Some(p[ROUGH] as f64),
            vibrato_depth: Some(if self.mode == 0 { p[VIBRATO] as f64 } else { 0.0 }),
            vibrato_rate: Some(p[VIB_RATE] as f64),
            brightness: Some(p[BRIGHT] as f64),
            ..Default::default()
        };
        let mut artic = ArticPatch::default();
        if self.mode == 0 && !plans.vowels.is_empty() {
            let a = plans.vowels[(p[VOWEL].round().max(0.0) as usize).min(plans.vowels.len() - 1)];
            let b = plans.vowels[(p[MORPH_TO].round().max(0.0) as usize).min(plans.vowels.len() - 1)];
            let m = p[MORPH].clamp(0.0, 1.0) as f64;
            artic = ArticPatch::blend(&[(1.0 - m, a), (m, b)]);
            let base = |a: &ArticPatch, k: ArticKey| a.get(k).unwrap_or(NEUTRAL.get(k));
            let (jaw, tb, lips) = (base(&artic, ArticKey::Jaw), base(&artic, ArticKey::TbX), base(&artic, ArticKey::LipAp));
            artic.set(ArticKey::Jaw, (jaw + p[JAW] as f64).clamp(0.0, 1.2));
            artic.set(ArticKey::TbX, tb + p[TONGUE] as f64);
            artic.set(ArticKey::LipAp, (lips + p[LIPS] as f64).max(0.0));
        }
        artic.set(ArticKey::Larynx, p[LARYNX] as f64);
        artic.set(ArticKey::Velum, -0.15 + 0.75 * p[NASAL].clamp(0.0, 1.0) as f64);
        let n = Self::voices_n(p);
        for e in self.engines.iter_mut().take(n) {
            if self.creature.is_none() {
                e.set_source(&src);
                e.set_artic(&artic, None);
            }
        }
    }

    fn hz(&self, pitch: u8) -> f64 {
        440.0 * 2f64.powf((pitch as f64 + self.pitch_bend as f64 - 69.0) / 12.0)
    }

    /// A voice for a new note: one not playing, else the oldest (one voice: always it).
    fn pick(&self, n: usize) -> usize {
        if let Some(i) = (0..n).find(|i| self.voices[*i].note.is_none()) {
            return i;
        }
        (0..n).min_by_key(|i| self.voices[*i].age).unwrap_or(0)
    }

    pub fn note_on(&mut self, p: &[f32], plans: &Plans, pitch: u8, vel: f32) {
        if p.len() < SPECS.len() {
            return;
        }
        self.fit(p);
        self.apply(p, plans, false);
        let n = Self::voices_n(p);
        let i = self.pick(n);
        self.clock += 1;
        let hz = self.hz(pitch);
        let vel = vel.clamp(0.05, 1.0) as f64;
        // A voice still singing a syllable lets it go first.
        if self.voices[i].note.is_some() && self.mode == 1 {
            let t = self.voices[i].tag;
            self.engines[i].release(t);
            self.engines[i].release(t + 1.0);
        }
        let v = &mut self.voices[i];
        v.note = Some(pitch);
        v.age = self.clock;
        v.quiet = 0;
        v.on = true;
        let e = &mut self.engines[i];
        match self.mode {
            1 if !plans.syls.is_empty() => {
                let s = self.next_syl % plans.syls.len();
                self.next_syl += 1;
                self.sung = s;
                v.syl = s;
                self.tags += 2.0;
                v.tag = self.tags;
                let ratio = hz / (440.0 * 2f64.powf((REF_MIDI - 69.0) / 12.0));
                self.scratch.clear();
                for g in plans.syls[s].on.iter().take(self.scratch.capacity()) {
                    let mut g = *g;
                    if let Some(f) = g.targets.get(ControlKey::F0) {
                        g.targets.set(ControlKey::F0, f * ratio);
                    }
                    if g.targets.has(ControlKey::VIBRATO) {
                        g.targets.set(ControlKey::VIBRATO, p[VIBRATO] as f64);
                    }
                    g.tag = g.tag.map(|t| t + v.tag);
                    self.scratch.push(g);
                }
                e.set_gate(vel);
                e.schedule(&self.scratch, 0.0, false);
            }
            2 => {
                if let Some(def) = e.creature().map(|c| c.def) {
                    let call = (p[CALL].round().max(1.0) as usize - 1).min(def.calls.len().saturating_sub(1));
                    let r = def.calls.get(call).and_then(|c| c.pitch_ref).unwrap_or(1.0);
                    e.set_gate(vel);
                    e.play_call(call, Some(r * 2f64.powf((pitch as f64 + self.pitch_bend as f64 - 60.0) / 12.0)), Some(vel));
                }
            }
            _ => {
                e.set_source(&SourcePatch { f0: Some(hz), ..Default::default() });
                e.set_gate(vel);
            }
        }
    }

    pub fn note_off(&mut self, p: &[f32], plans: &Plans, pitch: u8) {
        if p.len() < SPECS.len() {
            return;
        }
        for i in 0..MAX_VOICES {
            if self.voices[i].note != Some(pitch) {
                continue;
            }
            let v = &mut self.voices[i];
            v.note = None;
            let e = &mut self.engines[i];
            match self.mode {
                1 => {
                    e.release(v.tag);
                    e.release(v.tag + 1.0);
                    if let Some(s) = plans.syls.get(v.syl) {
                        let ratio = self_ratio(pitch, self.pitch_bend);
                        self.scratch.clear();
                        for g in s.off.iter().take(self.scratch.capacity()) {
                            let mut g = *g;
                            if let Some(f) = g.targets.get(ControlKey::F0) {
                                g.targets.set(ControlKey::F0, f * ratio);
                            }
                            self.scratch.push(g);
                        }
                        e.schedule(&self.scratch, 0.0, false);
                    }
                    e.set_gate(0.0);
                }
                2 => {
                    e.stop_call();
                    e.set_gate(0.0);
                }
                _ => e.set_gate(0.0),
            }
        }
    }

    pub fn all_off(&mut self) {
        for (v, e) in self.voices.iter_mut().zip(self.engines.iter_mut()) {
            v.note = None;
            e.set_gate(0.0);
            e.stop_speech();
            e.stop_call();
        }
    }

    pub fn visual(&self, out: &mut [f32]) {
        for (k, m) in self.meters.iter().enumerate() {
            if let Some(o) = out.get_mut(k) {
                *o = *m;
            }
        }
        if let Some(o) = out.get_mut(4) {
            *o = self.sung as f32;
        }
        for (k, v) in self.voices.iter().enumerate() {
            if let Some(o) = out.get_mut(5 + k) {
                *o = v.note.map_or(-1.0, |n| n as f32);
            }
        }
    }

    pub fn render(&mut self, p: &[f32], plans: &Plans, l: &mut [f32], r: &mut [f32]) {
        if p.len() < SPECS.len() {
            return;
        }
        self.fit(p);
        let bend = self.pitch_bend != self.bend_was;
        self.apply(p, plans, false);
        if bend && self.mode == 0 {
            self.bend_was = self.pitch_bend;
            for i in 0..MAX_VOICES {
                if let Some(n) = self.voices[i].note {
                    let hz = self.hz(n);
                    self.engines[i].set_source(&SourcePatch { f0: Some(hz), ..Default::default() });
                }
            }
        }
        let n = Self::voices_n(p);
        let level = 10f32.powf(p[LEVEL] / 20.0) * if n > 1 { 0.8 } else { 1.0 };
        let len = l.len().min(self.buf.len());
        let quiet_after = (self.sr * 1.5) as usize;
        for i in 0..n {
            let v = &mut self.voices[i];
            if !v.on {
                self.meters[i] = 0.0;
                continue;
            }
            let buf = &mut self.buf[..len];
            self.engines[i].render(buf);
            let mut peak = 0.0f32;
            // Spread over the field a little, voice by voice.
            let pan = if n == 1 { 0.0 } else { (i as f32 / (n - 1) as f32 - 0.5) * 0.6 };
            let (gl, gr) = ((1.0 - pan).min(1.0) * level, (1.0 + pan).min(1.0) * level);
            for k in 0..len {
                let s = if buf[k].is_finite() { buf[k] } else { 0.0 };
                peak = peak.max(s.abs());
                l[k] += s * gl;
                r[k] += s * gr;
            }
            self.meters[i] = self.meters[i] * 0.7 + peak * 0.3;
            if v.note.is_none() {
                v.quiet = if peak < 1e-4 { v.quiet + len } else { 0 };
                if v.quiet > quiet_after {
                    v.on = false;
                }
            }
        }
    }
}

fn self_ratio(pitch: u8, bend: f32) -> f64 {
    2f64.powf((pitch as f64 + bend as f64 - REF_MIDI) / 12.0)
}

/// Creature `k`'s definition (its calls), as the engine has it.
pub fn creature_def(k: usize) -> Option<&'static chimera::creatures::CreatureDef> {
    chimera::creatures::creature_by_id(CREATURE_IDS.get(k)?)
}

/// Makes the engine's tables now, off the audio thread.
pub fn warm() {
    chimera::init();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn play(p: &[f32], plans: &Plans, notes: &[(u8, usize, usize)], secs: f32) -> (f32, f32) {
        let sr = 48000.0;
        let mut st = AirwayState::new(sr);
        let total = (sr * secs) as usize;
        let (mut l, mut r) = (vec![0.0f32; 512], vec![0.0f32; 512]);
        let (mut peak, mut energy) = (0.0f32, 0.0f32);
        let mut at = 0;
        while at < total {
            for &(n, on, off) in notes {
                if on >= at && on < at + 512 {
                    st.note_on(p, plans, n, 0.9);
                }
                if off >= at && off < at + 512 {
                    st.note_off(p, plans, n);
                }
            }
            l.fill(0.0);
            r.fill(0.0);
            st.render(p, plans, &mut l, &mut r);
            for v in &l {
                assert!(v.is_finite());
                peak = peak.max(v.abs());
                energy += v * v;
            }
            at += 512;
        }
        (peak, (energy / total as f32).sqrt())
    }

    #[test]
    fn a_key_sings_a_vowel_and_stops() {
        let p = preset("Choir Ah");
        let plans = prepare("", 0);
        assert_eq!(plans.vowels.len(), VOWEL_IPA.len());
        assert!(plans.vowels.iter().all(|v| v.iter().count() > 0), "every vowel has a mouth");
        let (peak, rms) = play(&p, &plans, &[(57, 0, 24000)], 1.0);
        assert!(peak > 0.02 && peak < 4.0, "it sings: {peak}");
        assert!(rms > 0.005, "{rms}");
        let (silent, _) = play(&p, &plans, &[], 0.3);
        assert!(silent < 1e-3, "no key, no voice: {silent}");
    }

    #[test]
    fn words_sing_their_syllables_in_turn() {
        let mut p = preset("Sung Words");
        p[VOICES] = 0.0;
        let plans = prepare("hello world", 1);
        assert!(plans.syls.len() >= 3, "{:?}", plans.syls.iter().map(|s| s.text.clone()).collect::<Vec<_>>());
        assert!(plans.syls.iter().all(|s| !s.on.is_empty()));
        let (peak, _) = play(&p, &plans, &[(64, 0, 12000), (67, 14000, 26000), (60, 28000, 40000)], 1.2);
        assert!(peak > 0.02, "it sings the words: {peak}");
    }

    #[test]
    fn creatures_call_and_presets_fit() {
        let p = preset("Seal");
        let plans = prepare("", 0);
        let (peak, _) = play(&p, &plans, &[(60, 0, 30000)], 1.0);
        assert!(peak > 0.005, "the seal calls: {peak}");
        for n in preset_names() {
            assert_eq!(preset(n).len(), SPECS.len());
        }
    }
}

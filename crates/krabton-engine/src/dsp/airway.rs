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
use chimera::voice::human_voice::VoiceTelemetry;

/// The engine, for the face (its anatomy drawn as it moves).
pub use chimera;

/// Where the face finds what the voice does now, in its visual values:
/// the articulators (`ArticKey` order), the pitch, the lungs, voicing,
/// the nose's opening, the output, the tract's length, its areas glottis to
/// lips, the glottis' opening over two cycles, how far along the velum is.
pub const VIS_ARTIC: usize = 9;
pub const VIS_F0: usize = 26;
pub const VIS_LUNGS: usize = 27;
pub const VIS_VOICING: usize = 28;
pub const VIS_NOSE: usize = 29;
pub const VIS_OUT: usize = 30;
pub const VIS_LENGTH: usize = 31;
pub const VIS_AREAS: usize = 32;
pub const N_AREAS: usize = 16;
pub const VIS_GLOTTIS: usize = 48;
pub const N_GLOTTIS: usize = 15;
pub const VIS_VELUM_AT: usize = 63;

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
/// The tongue up or down, and the lips pushed out or drawn in (from its face's handles).
pub const TONGUE_UP: usize = 23;
pub const LIP_PUSH: usize = 24;

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
    ParamSpec::lin("Tongue Up", "Mouth", -1.5, 1.5, 0.0, Unit::None),
    ParamSpec::lin("Lip Push", "Mouth", -0.5, 0.5, 0.0, Unit::None),
];

const PRESETS: &[(&str, &[(usize, f32)])] = &[
    ("Choir Ah", &[(VOICES, 1.0), (VIBRATO, 30.0)]),
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
    /// The lyrics notes may bring (a Sing sequencer's syllables), each
    /// sung whole on its note, looked up by its text.
    pub words: Vec<SylPlan>,
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
    prepare_with(text, body, &[])
}

/// `prepare`, and every one of `words` (lyrics notes may bring) as one
/// syllable of its own: its first vowel held, the rest sung when the key is let go.
pub fn prepare_with(text: &str, body: u8, words: &[String]) -> PlansRef {
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
    let g = g2p();
    let mut keys: Vec<String> = words.iter().map(|w| chimera_key(w)).collect();
    keys.sort();
    keys.dedup();
    let words = keys
        .iter()
        .map(|w| {
            let parts = lyrics_to_syllables(w, &*g);
            // A word with no vowel to sing: kept, with nothing to sing.
            let Some(first) = parts.first() else { return SylPlan { text: w.clone(), on: Vec::new(), off: Vec::new() } };
            // One syllable of the whole word: the first's onset and vowel, then the rest as its coda.
            let mut s = first.clone();
            s.text = chimera_key(w);
            for q in &parts[1..] {
                s.coda.extend(q.onset.iter().cloned());
                s.coda.push(q.nucleus.clone());
                s.coda.extend(q.coda.iter().cloned());
            }
            let on = plan_note_on(&s, REF_MIDI, &NoteOnOptions { body: mix, vibrato: 1.0, tag: 0.0, legato_from: None });
            let off = plan_note_off(&s, REF_MIDI, mix);
            SylPlan { text: s.text.clone(), on, off }
        })
        .collect();
    PlansRef(Arc::new(Plans { text: text.to_string(), body, syls, vowels, words }))
}

/// A lyric as a note carries it (cut to what fits), to find its plan by.
pub fn chimera_key(w: &str) -> String {
    crate::midi_fx::Lyric::new(w, false).as_str().to_lowercase()
}

/// Whether `plans` already has exactly these words.
pub fn has_words(plans: &Plans, words: &[String]) -> bool {
    let mut keys: Vec<String> = words.iter().map(|w| chimera_key(w)).collect();
    keys.sort();
    keys.dedup();
    let mut have: Vec<&str> = plans.words.iter().map(|w| w.text.as_str()).collect();
    have.sort();
    have.len() == keys.len() && have.iter().zip(&keys).all(|(a, b)| *a == b.as_str())
}

// ---------------------------------------------------------------- playing

#[derive(Clone, Copy, Debug, Default)]
struct Voice {
    note: Option<u8>,
    /// When it was started (for stealing the oldest), its tags (Words), its syllable.
    age: u64,
    tag: f64,
    syl: usize,
    /// Its syllable is one of the words a note brought (not the lyric's).
    word: bool,
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
    /// The lyric the next note brings (a Sing sequencer's syllable).
    lyric: crate::midi_fx::Lyric,
    /// For the face: each voice's level, its note, the syllable sung last.
    meters: [f32; MAX_VOICES],
    pub sung: usize,
    /// What the voice shown does now (see `VIS_ARTIC` on).
    tel: VoiceTelemetry,
    shown: [f32; 55],
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
            lyric: crate::midi_fx::Lyric::new("", false),
            meters: [0.0; MAX_VOICES],
            sung: 0,
            tel: VoiceTelemetry::default(),
            shown: [0.0; 55],
        }
    }

    /// Reads what voice `i` does now into `shown` (the face's values from `VIS_ARTIC`).
    fn look(&mut self, i: usize) {
        self.engines[i].telemetry(&mut self.tel);
        let t = &self.tel;
        let s = &mut self.shown;
        for (k, a) in t.artic.iter().take(17).enumerate() {
            s[k] = *a;
        }
        s[VIS_F0 - VIS_ARTIC] = t.f0 as f32;
        s[VIS_LUNGS - VIS_ARTIC] = t.p_lung as f32;
        s[VIS_VOICING - VIS_ARTIC] = t.voicing as f32;
        s[VIS_NOSE - VIS_ARTIC] = t.velum_port as f32;
        s[VIS_OUT - VIS_ARTIC] = t.out_rms as f32;
        s[VIS_LENGTH - VIS_ARTIC] = (t.ph_len + t.or_len) as f32;
        let n = t.ph_areas.len() + t.or_areas.len();
        if n > 0 {
            for k in 0..N_AREAS {
                let j = k * n / N_AREAS;
                s[VIS_AREAS - VIS_ARTIC + k] = if j < t.ph_areas.len() { t.ph_areas[j] } else { t.or_areas.get(j - t.ph_areas.len()).copied().unwrap_or(0.0) };
            }
        }
        let g = t.glottal_area.len();
        if g > 0 {
            for k in 0..N_GLOTTIS {
                s[VIS_GLOTTIS - VIS_ARTIC + k] = t.glottal_area[k * g / N_GLOTTIS];
            }
        }
        s[VIS_VELUM_AT - VIS_ARTIC] = (t.ph_len / (t.ph_len + t.or_len).max(1e-6)) as f32;
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
        let artic = mouth(p, plans, self.mode == 0);
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

    /// The lyric the next note brings (set just before it).
    pub fn set_lyric(&mut self, l: crate::midi_fx::Lyric) {
        self.lyric = l;
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
        // A lyric the note brings, if it is one planned: sung whole (but by a creature).
        let brought = std::mem::replace(&mut self.lyric, crate::midi_fx::Lyric::new("", false));
        let word = if self.mode != 2 && !brought.as_str().is_empty() {
            let key = brought.as_str();
            plans.words.iter().position(|w| !w.on.is_empty() && w.text.len() == key.len() && w.text.eq_ignore_ascii_case(key))
        } else {
            None
        };
        // A voice still singing a syllable lets it go first.
        if self.voices[i].note.is_some() && (self.mode == 1 || self.voices[i].word) {
            let t = self.voices[i].tag;
            self.engines[i].release(t);
            self.engines[i].release(t + 1.0);
        }
        let v = &mut self.voices[i];
        v.note = Some(pitch);
        v.age = self.clock;
        v.quiet = 0;
        v.on = true;
        v.word = word.is_some();
        let e = &mut self.engines[i];
        let mode = if word.is_some() { 1 } else { self.mode };
        match mode {
            1 if word.is_some() || !plans.syls.is_empty() => {
                let (s, plan) = match word {
                    Some(w) => (w, &plans.words[w]),
                    None => {
                        let s = self.next_syl % plans.syls.len();
                        self.next_syl += 1;
                        self.sung = s;
                        (s, &plans.syls[s])
                    }
                };
                v.syl = s;
                self.tags += 2.0;
                v.tag = self.tags;
                let ratio = hz / (440.0 * 2f64.powf((REF_MIDI - 69.0) / 12.0));
                self.scratch.clear();
                for g in plan.on.iter().take(self.scratch.capacity()) {
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
            let mode = if v.word { 1 } else { self.mode };
            match mode {
                1 => {
                    e.release(v.tag);
                    e.release(v.tag + 1.0);
                    let plan = if v.word { plans.words.get(v.syl) } else { plans.syls.get(v.syl) };
                    if let Some(s) = plan {
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
        for (k, v) in self.shown.iter().enumerate() {
            if let Some(o) = out.get_mut(VIS_ARTIC + k) {
                *o = *v;
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
        // The face shows the voice started last (the human one: a creature has no mouth to show).
        if self.creature.is_none() {
            let i = (0..n).filter(|i| self.voices[*i].on).max_by_key(|i| self.voices[*i].age).unwrap_or(0);
            self.look(i);
        }
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

/// The mouth the settings ask for: the vowel (morphing to the other) with
/// the jaw, the tongue and the lips moved from it (with `vowel`; else only
/// what the words leave: the larynx and the nose).
pub fn mouth(p: &[f32], plans: &Plans, vowel: bool) -> ArticPatch {
    let get = |k: usize, d: f32| p.get(k).copied().unwrap_or(d);
    let mut artic = ArticPatch::default();
    if vowel && !plans.vowels.is_empty() {
        let a = plans.vowels[(get(VOWEL, 0.0).round().max(0.0) as usize).min(plans.vowels.len() - 1)];
        let b = plans.vowels[(get(MORPH_TO, 0.0).round().max(0.0) as usize).min(plans.vowels.len() - 1)];
        let m = get(MORPH, 0.0).clamp(0.0, 1.0) as f64;
        artic = ArticPatch::blend(&[(1.0 - m, a), (m, b)]);
        let base = |a: &ArticPatch, k: ArticKey| a.get(k).unwrap_or(NEUTRAL.get(k));
        let (jaw, tb, ty, lips, push) = (base(&artic, ArticKey::Jaw), base(&artic, ArticKey::TbX), base(&artic, ArticKey::TbY), base(&artic, ArticKey::LipAp), base(&artic, ArticKey::LipProt));
        artic.set(ArticKey::Jaw, (jaw + get(JAW, 0.0) as f64).clamp(0.0, 1.2));
        artic.set(ArticKey::TbX, tb + get(TONGUE, 0.0) as f64);
        artic.set(ArticKey::TbY, ty + get(TONGUE_UP, 0.0) as f64);
        artic.set(ArticKey::LipAp, (lips + get(LIPS, 0.0) as f64).max(0.0));
        artic.set(ArticKey::LipProt, (push + get(LIP_PUSH, 0.0) as f64).clamp(0.0, 1.0));
    }
    artic.set(ArticKey::Larynx, get(LARYNX, 0.0) as f64);
    artic.set(ArticKey::Velum, -0.15 + 0.75 * get(NASAL, 0.0).clamp(0.0, 1.0) as f64);
    artic
}

/// The mouth the settings ask for, every articulator (`ArticKey` order), for the face to draw while nothing sings.
pub fn target_artic(p: &[f32], plans: &Plans) -> [f32; 17] {
    let vowel = p.get(MODE).is_none_or(|m| m.round() as usize == 0);
    let m = mouth(p, plans, vowel).apply_to(&NEUTRAL);
    std::array::from_fn(|k| chimera::anatomy::human::ArticKey::ALL.get(k).map_or(0.0, |key| m.get(*key) as f32))
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
    fn a_note_that_brings_a_lyric_sings_it() {
        let p = preset("Choir Ah");
        let words: Vec<String> = ["sa", "ku", "ra", "hello"].iter().map(|s| s.to_string()).collect();
        let plans = prepare_with("", 0, &words);
        assert_eq!(plans.words.len(), 4);
        assert!(has_words(&plans, &words));
        assert!(!has_words(&plans, &words[..2]));
        let mut st = AirwayState::new(48000.0);
        st.set_lyric(crate::midi_fx::Lyric::new("ku", true));
        st.note_on(&p, &plans, 60, 0.9);
        assert!(st.voices[0].word, "sung as a word, though it plays vowels");
        let (mut l, mut r) = (vec![0.0f32; 512], vec![0.0f32; 512]);
        let mut peak = 0.0f32;
        for _ in 0..40 {
            l.fill(0.0);
            st.render(&p, &plans, &mut l, &mut r);
            peak = l.iter().fold(peak, |m, v| m.max(v.abs()));
        }
        assert!(peak > 0.01, "{peak}");
        st.note_off(&p, &plans, 60);
        // A lyric not planned: the vowel, as ever.
        st.set_lyric(crate::midi_fx::Lyric::new("zzz", true));
        st.note_on(&p, &plans, 62, 0.9);
        assert!(!st.voices.iter().any(|v| v.note == Some(62) && v.word));
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

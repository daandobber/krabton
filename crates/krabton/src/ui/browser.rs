//! The browser, laid out like Live's: a sidebar with Collections, Categories
//! and Places next to a list of names, where devices open up into their
//! presets and folders into their files.
//!
//! Items load onto the selected track with a double-click or Enter, or go
//! where they are dropped. With the browser focused, the arrow keys browse
//! (left and right close and open folders), 1-7 put the selected item in a
//! collection and 0 takes it out of all of them. Ctrl+F searches and
//! Ctrl+Alt+B hides the browser.

use crate::app::{App, BrowserItem};
use crate::theme::*;
use egui::{pos2, vec2, Align2, Color32, Key, Pos2, Rect, RichText, Sense, Shape, Stroke, StrokeKind, Ui};
use krabton_engine::dsp::dx7::{self, Dx7Voice};
use krabton_engine::dsp::effects::EffectKind;
use krabton_engine::dsp::{analog, braids_synth, drums, elements_synth, macro_osc, shaker_synth, synth, stk_voice, stk_mallets, stk_strings, stk_winds, organ, rings_synth};
use krabton_engine::midi_fx::MidiFxKind;
use krabton_engine::{Command, SampleRef};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

const ROW_H: f32 = 18.0;
const INDENT: f32 = 12.0;
/// Folders on disk looked into per frame while searching.
const SEARCH_DIRS: usize = 400;

/// Live's collections: Favorites and six colors.
pub const COLLECTIONS: [(&str, Color32); 7] = [
    ("Favorites", Color32::from_rgb(0xff, 0x45, 0x3a)),
    ("Orange", Color32::from_rgb(0xff, 0x9f, 0x0a)),
    ("Yellow", Color32::from_rgb(0xff, 0xd6, 0x0a)),
    ("Green", Color32::from_rgb(0x4c, 0xd9, 0x64)),
    ("Blue", Color32::from_rgb(0x4d, 0xa8, 0xff)),
    ("Purple", Color32::from_rgb(0xbf, 0x5a, 0xf2)),
    ("Gray", Color32::from_rgb(0x8e, 0x8e, 0x93)),
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Category {
    All,
    Sounds,
    Drums,
    Instruments,
    AudioEffects,
    MidiEffects,
    Plugins,
    Clips,
    Samples,
}

impl Category {
    pub const LIST: [Category; 9] = [
        Category::All,
        Category::Sounds,
        Category::Drums,
        Category::Instruments,
        Category::AudioEffects,
        Category::MidiEffects,
        Category::Plugins,
        Category::Clips,
        Category::Samples,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Category::All => "All",
            Category::Sounds => "Sounds",
            Category::Drums => "Drums",
            Category::Instruments => "Instruments",
            Category::AudioEffects => "Audio Effects",
            Category::MidiEffects => "MIDI Effects",
            Category::Plugins => "Plug-ins",
            Category::Clips => "Clips",
            Category::Samples => "Samples",
        }
    }
}

/// What the sidebar has selected.
#[derive(Clone, PartialEq, Debug)]
pub enum Section {
    Collection(usize),
    Category(Category),
    Place(PathBuf),
}

impl Section {
    fn name(&self) -> String {
        match self {
            Section::Collection(i) => COLLECTIONS[*i].0.into(),
            Section::Category(c) => c.name().into(),
            Section::Place(p) => file_name(p),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Icon {
    Preset,
    Device,
    Sample,
    Folder,
    Plugin,
    Clip,
    Bank,
    Action,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Act {
    ScanPlugins,
    AddPluginFolder,
    LoadSyx,
    LoadSyxFolder,
}

enum Kids {
    Nodes(Vec<Node>),
    /// A folder on disk, read when it is opened.
    Dir { path: PathBuf, samples_only: bool },
    /// The voices in a DX7 .syx file.
    Syx(PathBuf),
}

/// The tree the list is drawn from, built fresh for the selected section.
enum Node {
    Folder { tag: Option<String>, label: String, icon: Icon, hint: String, load: Option<BrowserItem>, kids: Kids },
    Item { tag: Option<String>, label: String, detail: String, icon: Icon, hint: String, item: BrowserItem },
    Action { label: String, hint: String, act: Act },
    Note(String),
}

impl Node {
    fn item(tag: impl Into<String>, label: impl Into<String>, icon: Icon, hint: impl Into<String>, item: BrowserItem) -> Self {
        Node::Item { tag: Some(tag.into()), label: label.into(), detail: String::new(), icon, hint: hint.into(), item }
    }

    fn folder(label: impl Into<String>, icon: Icon, kids: Vec<Node>) -> Self {
        Node::Folder { tag: None, label: label.into(), icon, hint: String::new(), load: None, kids: Kids::Nodes(kids) }
    }

    /// Dim text at the right of the row (e.g. the device of a sound).
    fn detail(mut self, d: &str) -> Self {
        if let Node::Item { detail, .. } = &mut self {
            *detail = d.into();
        }
        self
    }

    fn label(&self) -> &str {
        match self {
            Node::Folder { label, .. } | Node::Item { label, .. } | Node::Action { label, .. } | Node::Note(label) => label,
        }
    }
}

#[derive(Clone)]
enum RowKind {
    /// `auto`: opened by a search (closing it is remembered separately).
    Folder { open: bool, auto: bool, load: Option<BrowserItem>, dir: Option<PathBuf> },
    Item(BrowserItem),
    Action(Act),
    Note,
}

/// One line of the list as drawn.
#[derive(Clone)]
pub(crate) struct Row {
    depth: usize,
    id: String,
    /// The item's key in collections.
    tag: Option<String>,
    label: String,
    detail: String,
    icon: Option<Icon>,
    hint: String,
    kind: RowKind,
}

impl Row {
    fn item(&self) -> Option<&BrowserItem> {
        match &self.kind {
            RowKind::Item(i) => Some(i),
            RowKind::Folder { load, .. } => load.as_ref(),
            _ => None,
        }
    }
}

enum UiAct {
    Select(String),
    Toggle(Row),
    Sample(PathBuf),
    Load(BrowserItem),
    Run(Act),
    Tag(String, String, usize),
    Untag(String),
    AddPlace(PathBuf),
    RemovePlace(PathBuf),
    Show(Section),
}

pub struct BrowserState {
    pub visible: bool,
    pub section: Section,
    pub filter: String,
    last_filter: String,
    selected: Option<String>,
    open: HashSet<String>,
    closed_in_search: HashSet<String>,
    /// Folder listings: (path, is a folder).
    dirs: HashMap<PathBuf, Vec<(PathBuf, bool)>>,
    syx: HashMap<PathBuf, Vec<Dx7Voice>>,
    /// The rows drawn last frame, for the keys.
    rows: Vec<Row>,
    /// Keys go to the list (set by clicking in the browser).
    pub focused: bool,
    pub focus_search: bool,
    select_first: bool,
    scroll_to_selected: bool,
    scroll_top: bool,
    /// Play samples when they are selected.
    pub preview: bool,
    preview_sample: Option<SampleRef>,
    music: Option<PathBuf>,
}

impl BrowserState {
    pub fn new() -> Self {
        let music = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")).map(|h| PathBuf::from(h).join("Music")).filter(|m| m.is_dir());
        Self {
            visible: true,
            section: Section::Category(Category::Instruments),
            filter: String::new(),
            last_filter: String::new(),
            selected: None,
            open: HashSet::new(),
            closed_in_search: HashSet::new(),
            dirs: HashMap::new(),
            syx: HashMap::new(),
            rows: Vec::new(),
            focused: false,
            focus_search: false,
            select_first: false,
            scroll_to_selected: false,
            scroll_top: false,
            preview: true,
            preview_sample: None,
            music,
        }
    }
}

fn file_name(p: &Path) -> String {
    p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| p.display().to_string())
}

fn has_ext(p: &Path, ext: &str) -> bool {
    p.extension().map(|e| e.eq_ignore_ascii_case(ext)).unwrap_or(false)
}

fn dir_tag(p: &Path) -> String {
    format!("dir/{}", p.display())
}

fn file_tag(p: &Path) -> String {
    format!("file/{}", p.display())
}

/// Subfolders, WAVs and DX7 banks in `dir`: folders first, then by name.
fn list_dir(dir: &Path) -> Vec<(PathBuf, bool)> {
    let mut v = Vec::new();
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten().take(5000) {
            let path = e.path();
            let name = e.file_name().to_string_lossy().to_string();
            if name.starts_with('.') || name.starts_with('$') {
                continue;
            }
            let is_dir = path.is_dir();
            if is_dir || has_ext(&path, "wav") || has_ext(&path, "flac") || has_ext(&path, "syx") || has_ext(&path, "sfz") {
                v.push((path, is_dir));
            }
        }
    }
    v.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| file_name(&a.0).to_lowercase().cmp(&file_name(&b.0).to_lowercase())));
    v
}

/// Whether every search word is in one of `parts` (lowercase).
fn matches(tokens: &[String], parts: &[&str]) -> bool {
    tokens.iter().all(|t| parts.iter().any(|p| p.contains(t.as_str())))
}

/// The kind of sound a preset makes, from its name, as in Live's Sounds
/// (None: an init patch, not a sound). "Drums" go to the Drums category.
fn sound_type(name: &str) -> Option<&'static str> {
    let n = name.to_lowercase();
    let any = |words: &[&str]| words.iter().any(|w| n.contains(w));
    Some(if n.starts_with("init") {
        return None;
    } else if any(&["kick", "snare", "hat", "cymbal", "membrane", "wood block", "metal hit", "deep drum", "struck drum", "noise hit", "rain stick", "perc", "clap", "tom", "timpani"]) {
        "Drums"
    } else if any(&["bassoon", "trumpet", "horn", "sax", "clarinet", "oboe", "tuba", "trombone"]) {
        "Brass & Winds"
    } else if any(&["bass", "acid"]) {
        "Bass"
    } else if any(&["harpsi", "piano", "epno", "e.p", "organ", "clav", "keys", "poly"]) {
        "Keys"
    } else if any(&["string", "bowed", "cello", "violin", "harp", "sitar", "koto", "strum", "orch"]) {
        "Strings"
    } else if any(&["pad", "drone", "swarm"]) {
        "Pad"
    } else if any(&["brass", "flute", "blown", "breath", "pipe"]) {
        "Brass & Winds"
    } else if any(&["bell", "mallet", "marimba", "vibraphone", "vibe", "glock", "kalimba", "steel drum", "chime", "gong", "glass", "toy box"]) {
        "Mallets & Bells"
    } else if any(&["pluck", "guitar", "ukulele", "banjo", "mandolin", "resonator", "archtop", "twelve", "baritone", "slide"]) {
        "Plucked"
    } else if any(&["voice", "choir", "vowel", "talking", "formant"]) {
        "Voices"
    } else if any(&["lead", "saw", "arp", "sync"]) {
        "Lead"
    } else if any(&["stab"]) {
        "Stabs"
    } else if any(&["noise", "particles", "granular", "data", "zap", "tape"]) {
        "FX & Textures"
    } else {
        "Synth Misc"
    })
}

/// A built-in instrument and its presets.
struct Dev {
    name: &'static str,
    hint: &'static str,
    presets: &'static [&'static str],
    make: fn(&'static str) -> BrowserItem,
    /// The kind of sound all its presets make (else from each name).
    kind: Option<&'static str>,
}

/// The instruments with their presets, for Big Picture: (name, hint,
/// presets, what loads a preset).
pub(crate) fn instrument_catalog() -> Vec<(&'static str, &'static str, &'static [&'static str], fn(&'static str) -> BrowserItem)> {
    devices().into_iter().map(|d| (d.name, d.hint, d.presets, d.make)).collect()
}

/// What an instrument is, by its name (as the browser says it).
pub(crate) fn instrument_hint(name: &str) -> Option<&'static str> {
    devices().into_iter().find(|d| d.name == name).map(|d| d.hint)
}

fn devices() -> [Dev; 33] {
    [
        Dev {
            name: "Analog",
            hint: "Classic analog polysynth: ladder filter, sync, cross mod, PWM, drift, unison and chorus",
            presets: analog::preset_names(),
            make: BrowserItem::Analog,
            kind: None,
        },
        Dev {
            name: "Braids",
            hint: "47 shapes from Mutable Instruments Braids: analog, FM, physical models, wavetables, noise",
            presets: braids_synth::preset_names(),
            make: BrowserItem::Braids,
            kind: None,
        },
        Dev {
            name: "Elements",
            hint: "Modal synthesis from Mutable Instruments Elements: bow, blow and strike a resonator or strings",
            presets: elements_synth::preset_names(),
            make: BrowserItem::Elements,
            kind: None,
        },
        Dev {
            name: "Plaits",
            hint: "24 synthesis models from Mutable Instruments Plaits: analog, FM, speech, strings, drums",
            presets: macro_osc::preset_names(),
            make: BrowserItem::Macro,
            kind: None,
        },
        Dev {
            name: "Particles",
            hint: "Perry Cook's PhISEM shakers played with notes: maraca, cabasa, tambourine, sleigh bells, bamboo, cans, rocks; shake once, a few times or while held",
            presets: shaker_synth::preset_names(),
            make: BrowserItem::Shakers,
            kind: Some("Drums"),
        },
        Dev { name: "Synth", hint: "Two-oscillator virtual analog synth", presets: synth::preset_names(), make: BrowserItem::Synth, kind: None },
        Dev {
            name: "Modular",
            hint: "A eurorack-style modular synth: VCOs, filters, envelopes, LFOs, sequencers and more, patched with cables in a rack",
            presets: krabton_engine::modular::presets::preset_names(),
            make: BrowserItem::Modular,
            kind: None,
        },
        Dev { name: "Voice", hint: "A singing voice: 32 vowels and consonants to morph between, choirs, glide, and chant with undertones and overtones", presets: stk_voice::preset_names(), make: BrowserItem::Voice, kind: None },
        Dev { name: "Mallets", hint: "Marimba, vibraphone, agogo, woods, glass harmonica and Tibetan bowl: struck physical models from the Synthesis ToolKit", presets: stk_mallets::preset_names(), make: BrowserItem::Mallets, kind: None },
        Dev { name: "Strings", hint: "Bowed, plucked, mandolin, sitar and stiff strings: string physical models from the Synthesis ToolKit", presets: stk_strings::preset_names(), make: BrowserItem::Strings, kind: None },
        Dev { name: "Winds", hint: "Flute, clarinet, saxophone, brass, blown bottle, blow hole and whistle: blown physical models from the Synthesis ToolKit", presets: stk_winds::preset_names(), make: BrowserItem::Winds, kind: None },
        Dev { name: "Organ", hint: "Tonewheel organ: nine drawbars, percussion, key click, scanner vibrato and chorus, drive and a rotating speaker", presets: organ::preset_names(), make: BrowserItem::Organ, kind: None },
        Dev { name: "Rings", hint: "Resonator after Mutable Instruments Rings: modal, sympathetic and inharmonic strings, FM voice, chords, and the Disastrous Peace string synth", presets: rings_synth::preset_names(), make: BrowserItem::Rings, kind: None },
        Dev { name: "Grand Piano", hint: "A concert grand: the Salamander Grand Piano (a Yamaha C5 in 16 velocity layers, downloaded once), with sustain pedal, key and string noises, una corda, lid and a hall", presets: krabton_engine::piano::preset_names(), make: BrowserItem::Piano, kind: None },
        Dev { name: "Monk", hint: "A chanting monk made of FOF grains (ported from MonkSynth): vowel, head size, glide, a choir of up to ten, and an echo", presets: krabton_engine::dsp::monk::preset_names(), make: BrowserItem::Monk, kind: None },
        Dev { name: "Bassline", hint: "Acid bass: a port of Open303 by Robin Schmidt. Saw to square, the squelchy filter with its envelope, accent (velocity 100 and up) and slide (play legato). Drag the filter in the screen", presets: krabton_engine::dsp::bassline::preset_names(), make: BrowserItem::Bassline, kind: None },
        Dev { name: "Mono", hint: "The small mono of the early eighties: saw, pulse with PWM, a sub oscillator (an octave or two down) and noise into a self-oscillating four-pole filter, one envelope (or the gate for the amplifier), LFO, portamento. Behaviour and presets after HUSH ONE by Charles Vestal (MIT)", presets: krabton_engine::dsp::mono101::preset_names(), make: BrowserItem::Mono101, kind: None },
        Dev { name: "Drone", hint: "A drone that keeps moving: tanpura strings with a buzzing bridge, a PADsynth pad, swelling FM pairs and wind, all drifting slowly. Hold a key for minutes", presets: krabton_engine::dsp::drone::preset_names(), make: BrowserItem::Drone, kind: None },
        Dev { name: "Waves", hint: "Wavetable synth: two oscillators morphing through 16 banks of single-cycle waves (Adventure Kid Waveforms), unison, warps, sub, filter, envelopes and an LFO. Drag a stack to move through it", presets: krabton_engine::dsp::waves::preset_names(), make: BrowserItem::Waves, kind: None },
        Dev { name: "Guitar", hint: "A guitar and its family: steel or nylon acoustic, electric with pickups, resonator, archtop, twelve-string, baritone, bass, ukulele, banjo and mandolin. Notes go on the strings and frets a player would use, chords are strummed. Put pedals after it", presets: krabton_engine::dsp::guitar::preset_names(), make: BrowserItem::Guitar, kind: None },
        Dev { name: "Wind Chimes", hint: "A wind chime that plays by itself: the wind pushes the sail, the clapper strikes the tubes (aluminium, brass, steel, bamboo, glass or wood) and you see every strike you hear. Change the tubes, the tuning, the spacing, the clapper and the sail", presets: krabton_engine::dsp::wind_chimes::preset_names(), make: BrowserItem::WindChimes, kind: None },
        Dev { name: "Birds", hint: "Songbirds that sing by themselves: blackbird, robin, song thrush, nightingale, wren, chaffinch, great tit, skylark, sparrow, cuckoo, wood pigeon, tawny owl, loon, gull and a drumming woodpecker. Each sings its species' songs from its own place, near or far; a key makes the bird with the nearest voice sing (Follow Key puts its song on the key)", presets: krabton_engine::dsp::birds::preset_names(), make: BrowserItem::Birds, kind: None },
        Dev { name: "Nature", hint: "The sounds of outside, made from scratch: wind in gusts that whistles and rustles the leaves, rain on leaves, water, a roof or a tent, thunder after the lightning, a brook, the sea, crickets that chirp faster when it is warmer, frogs and a campfire. Play keys and the wind whistles on those notes", presets: krabton_engine::dsp::nature::preset_names(), make: BrowserItem::Nature, kind: None },
        Dev { name: "Facet", hint: "One synth, six engines, four knobs: Crease (phase distortion), Swarm (seven saws), Fold (wavefolding and FM), Grit (digital steps and noise), Partials (additive) and Wide (pulses and a wide chorus)", presets: krabton_engine::dsp::facet::preset_names(), make: BrowserItem::Facet, kind: None },
        Dev { name: "Bass Guitar", hint: "An electric bass: four wound strings (E A D G, or Drop D) heard through a pickup at the neck, in the middle or at the bridge, played with the fingers or a pick, muted with the palm. The Guitar with its bass model (also in the Guitar's folder)", presets: krabton_engine::dsp::guitar::BASS_PRESETS, make: BrowserItem::Guitar, kind: Some("Bass") },
        Dev { name: "Pan Flute", hint: "Blown pipes: pan flute (closed pipes, odd harmonics), shakuhachi, ocarina, bamboo flute and wooden flute, of bamboo, wood, clay, metal, glass, bone or plastic, with breath that rings in the pipe, a chiff at the start, scoops, vibrato that comes in after a while. Played as a mouth plays it (the lips on one pipe or two neighbours, sliding along the row) or freely", presets: krabton_engine::dsp::pan_flute::preset_names(), make: BrowserItem::PanFlute, kind: None },
        Dev { name: "Melodica", hint: "A keyboard you blow into, a real one: every key a little out of tune, a breath that is never still, tongued or swelling notes, clicking keys; free reeds sharing one breath, the plastic body, the mouthpiece or the tube; and a dub desk after it: drive, a tape echo on the beat, a spring reverb", presets: krabton_engine::dsp::melodica::preset_names(), make: BrowserItem::Melodica, kind: None },
        Dev { name: "Harmonica", hint: "A diatonic harp in any key or the song's (straight, cross harp, slant): every note in its hole, blown or drawn, missing ones bent or overblown; never blow and draw at once; the lips over more holes for chords, the hands cupped around it for a wah. Or free, as a chromatic harmonica", presets: krabton_engine::dsp::harmonica::preset_names(), make: BrowserItem::Harmonica, kind: None },
        Dev { name: "VL-Tone", hint: "The calculator with a keyboard: the Casio VL-1 of 1981, ported from the VL1 emulator. Ten sounds (piano, fantasy, violin, flute, two guitars, English horn and three Electro ones that jump an octave), its stepped envelope set with eight digits as on the real one, vibrato and tremolo, and its rhythm box (March to Beguine) on the song's tempo or its own", presets: krabton_engine::dsp::vl1::preset_names(), make: BrowserItem::VlTone, kind: None },
        Dev { name: "NES", hint: "The Nintendo's 2A03 chip: two pulse channels (four duties, the sweep that bends the pitch), the triangle that has no volume, and noise (long, or the short metallic loop), in sixteen volume steps on 11-bit timers, mixed and filtered as the console does. Layered, split over the channels, or as an arpeggio sixty frames a second", presets: krabton_engine::dsp::nes::preset_names(), make: BrowserItem::Nes, kind: None },
        Dev { name: "Game Boy", hint: "The Game Boy's sound: two pulses (one with the sweep), the wave channel with fifteen tables, and noise that buzzes into a tone in 7-bit mode; each left, right or both, the envelope stepping 64 times a second, the output of the first Game Boy or the Pocket and Color. Layered, split over the channels, or arpeggiated", presets: krabton_engine::dsp::gameboy::preset_names(), make: BrowserItem::GameBoy, kind: None },
        Dev { name: "SID", hint: "The Commodore 64's SID, the 6581 or the 8580: three voices of triangle, saw, pulse and noise (or two or three at once), hard sync and ring modulation, the envelope with its sixteen rates, and the multimode filter with resonance. Pulse width sweeps, vibrato and the C64's chord arpeggios move fifty times a second. Layered, split over the voices, or arpeggiated", presets: krabton_engine::dsp::sid::preset_names(), make: BrowserItem::Sid, kind: None },
        Dev { name: "Beads", hint: "The wavetable synth of Mutable Instruments Beads, played with keys: twelve banks of waves (sines, drawbars, voices, metal, FM and more), and grains of them seeded while you hold a key (or always, or with the song's clock), each randomized in wave, size, shape and pitch, with Beads' reverb and its tape and lo-fi qualities", presets: krabton_engine::dsp::beads_synth::preset_names(), make: BrowserItem::BeadsSynth, kind: None },
    ]
}

const FM_HINT: &str = "FM voice: double-click or drag onto a track";
const SAMPLE_HINT: &str = "Click to hear it, double-click to load it, or drag it onto a track or slot";

fn fm_voice(bank: &crate::app::FmBank, i: usize, v: &Dx7Voice) -> Node {
    Node::item(format!("inst/FM/{}/{i}", bank.name), v.name(), Icon::Preset, FM_HINT, BrowserItem::FmVoice { bank: bank.name.clone(), voice: v.0.to_vec() })
}

/// The drum instruments (808, 909, CR-78, ...), each opening into its kits.
fn drum_engine_nodes() -> Vec<Node> {
    drums::DrumEngine::ALL
        .into_iter()
        .map(|e| {
            let kids = e.kits().iter().map(|k| Node::item(format!("inst/{}/{k}", e.name()), *k, Icon::Preset, e.description(), BrowserItem::Drums(k))).collect();
            Node::Folder { tag: None, label: e.name().into(), icon: Icon::Device, hint: e.description().into(), load: Some(BrowserItem::Drums(e.kits()[0])), kids: Kids::Nodes(kids) }
        })
        .collect()
}

fn plugin_effect_node(info: &krabton_clap::PluginInfo) -> Node {
    let hint = format!("{} by {} (audio effect)\n{}\n{}", info.name, info.vendor, info.description, info.path.display());
    Node::item(format!("plugin/{}", info.id), &info.name, Icon::Plugin, hint, BrowserItem::PluginEffect(info.clone()))
}

/// An effect: with presets, a folder that loads the effect and opens into them.
fn effect_node(k: EffectKind) -> Node {
    let list = krabton_engine::presets::effect(k);
    if list.is_empty() {
        return Node::item(format!("fx/{}", k.name()), k.name(), Icon::Device, k.description(), BrowserItem::Effect(k));
    }
    let kids = list.iter().map(|p| Node::item(format!("fx/{}/{}", k.name(), p.name), p.name, Icon::Preset, k.description(), BrowserItem::EffectPreset(k, p.name))).collect();
    Node::Folder { tag: None, label: k.name().into(), icon: Icon::Device, hint: k.description().into(), load: Some(BrowserItem::Effect(k)), kids: Kids::Nodes(kids) }
}

/// A MIDI effect or sequencer, a folder of its presets when it has them.
fn midi_node(k: MidiFxKind) -> Node {
    let list = krabton_engine::presets::midi(k);
    if list.is_empty() {
        return Node::item(format!("midi/{}", k.name()), k.name(), Icon::Device, k.description(), BrowserItem::MidiFx(k));
    }
    let kids = list.iter().map(|p| Node::item(format!("midi/{}/{}", k.name(), p.name), p.name, Icon::Preset, k.description(), BrowserItem::MidiPreset(k, p.name))).collect();
    Node::Folder { tag: None, label: k.name().into(), icon: Icon::Device, hint: k.description().into(), load: Some(BrowserItem::MidiFx(k)), kids: Kids::Nodes(kids) }
}

/// The built-in effects in folders by category, as in Live.
fn effect_nodes() -> Vec<Node> {
    EffectKind::CATEGORIES
        .iter()
        .map(|cat| {
            let mut kinds: Vec<EffectKind> = EffectKind::ALL.into_iter().filter(|k| k.category() == *cat).collect();
            kinds.sort_by_key(|k| k.name());
            let kids = kinds.into_iter().map(effect_node).collect();
            Node::folder(*cat, Icon::Folder, kids)
        })
        .collect()
}

fn midi_nodes() -> Vec<Node> {
    let items = |kinds: &[MidiFxKind]| {
        let mut kinds = kinds.to_vec();
        kinds.sort_by_key(|k| k.name());
        kinds.into_iter().map(midi_node).collect::<Vec<_>>()
    };
    // Sequencers in a folder per group, then the MIDI effects.
    let groups = MidiFxKind::SEQUENCER_GROUPS.iter().map(|g| {
        let kinds: Vec<MidiFxKind> = MidiFxKind::SEQUENCERS.into_iter().filter(|k| k.group() == *g).collect();
        Node::folder(*g, Icon::Folder, items(&kinds))
    });
    let mut out = vec![Node::folder("Sequencers", Icon::Folder, groups.collect())];
    out.extend(items(&MidiFxKind::ALL));
    out
}

impl App {
    pub(crate) fn ui_browser(&mut self, ui: &mut Ui) {
        let rect = ui.max_rect();
        let (pressed, pos) = ui.input(|i| (i.pointer.any_pressed(), i.pointer.interact_pos()));
        if pressed && pos.is_some_and(|p| !rect.contains(p)) {
            self.browser.focused = false;
        }
        let frame = |fill| egui::Frame::new().fill(fill);
        egui::TopBottomPanel::top("browser_search")
            .frame(frame(PANEL).inner_margin(egui::Margin { left: 6, right: 6, top: 6, bottom: 5 }))
            .show_inside(ui, |ui| self.browser_search(ui));
        egui::TopBottomPanel::bottom("browser_preview")
            .exact_height(34.0)
            .frame(frame(PANEL).inner_margin(egui::Margin::symmetric(6, 4)))
            .show_inside(ui, |ui| self.browser_preview(ui));
        egui::SidePanel::left("browser_side")
            .resizable(true)
            .default_width(118.0)
            .width_range(84.0..=240.0)
            .frame(frame(PANEL))
            .show_inside(ui, |ui| self.browser_sidebar(ui));
        egui::CentralPanel::default().frame(frame(shade(PANEL, 0.86))).show_inside(ui, |ui| self.browser_list(ui));
    }

    fn browser_search(&mut self, ui: &mut Ui) {
        // Buttons first, from the right; the field gets what is left (its
        // width excludes its margin, so a wider one would grow the panel).
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.small_button("<").on_hover_text("Hide the browser (Ctrl+Alt+B)").clicked() {
                self.browser.visible = false;
            }
            if !self.browser.filter.is_empty() && ui.small_button("x").on_hover_text("Clear the search").clicked() {
                self.browser.filter.clear();
            }
            let edit = egui::TextEdit::singleline(&mut self.browser.filter).hint_text("Search (Ctrl+F)").desired_width((ui.available_width() - 10.0).max(40.0));
            let resp = ui.add(edit);
            if self.browser.focus_search {
                self.browser.focus_search = false;
                resp.request_focus();
            }
            if resp.has_focus() {
                // Down goes from the search field into the results.
                if ui.input(|i| i.key_pressed(Key::ArrowDown)) {
                    resp.surrender_focus();
                    self.browser.focused = true;
                    self.browser.select_first = true;
                }
                if ui.input(|i| i.key_pressed(Key::Escape)) {
                    self.browser.filter.clear();
                }
            }
        });
    }

    /// Music, the project's folder and the folders added by hand.
    fn places(&self) -> Vec<(String, PathBuf, bool)> {
        let mut v = Vec::new();
        if let Some(m) = &self.browser.music {
            v.push(("Music".to_string(), m.clone(), false));
        }
        if let Some(dir) = self.file_path.as_ref().and_then(|p| p.parent()) {
            v.push(("Current Project".to_string(), dir.to_path_buf(), false));
        }
        for p in &self.prefs.places {
            v.push((file_name(p), p.clone(), true));
        }
        v
    }

    fn browser_sidebar(&mut self, ui: &mut Ui) {
        let mut acts = Vec::new();
        egui::ScrollArea::vertical().id_salt("browser_side").auto_shrink([false, false]).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            side_heading(ui, "Collections");
            for (i, (name, color)) in COLLECTIONS.iter().enumerate() {
                let n = self.prefs.tagged.iter().filter(|t| t.collections & (1 << i) != 0).count();
                let resp = self.side_row(ui, Section::Collection(i), name, SideIcon::Dot(*color));
                resp.on_hover_text(format!("{n} item(s). Select an item and press {}, or right-click it.", i + 1));
            }
            side_heading(ui, "Categories");
            for c in Category::LIST {
                self.side_row(ui, Section::Category(c), c.name(), SideIcon::Category(c));
            }
            side_heading(ui, "Places");
            for (label, path, user) in self.places() {
                let resp = self.side_row(ui, Section::Place(path.clone()), &label, SideIcon::Folder);
                if user {
                    resp.context_menu(|ui| {
                        if ui.button("Remove from Places").clicked() {
                            acts.push(UiAct::RemovePlace(path.clone()));
                            ui.close();
                        }
                    });
                }
                resp.on_hover_text(path.display().to_string());
            }
            let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), ROW_H + 1.0), Sense::click());
            if resp.hovered() {
                ui.painter().rect_filled(rect, 0.0, LINE_SOFT);
            }
            paint_side_icon(ui.painter(), pos2(rect.left() + 13.0, rect.center().y), SideIcon::Add, TEXT_DIM);
            ui.painter().with_clip_rect(rect).text(pos2(rect.left() + 24.0, rect.center().y), Align2::LEFT_CENTER, "Add Folder...", body(), TEXT_DIM);
            if resp.on_hover_text("Add a folder to Places").clicked() {
                if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                    acts.push(UiAct::AddPlace(dir));
                }
            }
        });
        for a in acts {
            self.browser_do(a);
        }
    }

    fn side_row(&mut self, ui: &mut Ui, section: Section, label: &str, icon: SideIcon) -> egui::Response {
        let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), ROW_H + 1.0), Sense::click());
        let selected = self.browser.section == section;
        let p = ui.painter();
        if selected {
            p.rect_filled(rect, 0.0, if self.browser.focused { mix(PANEL, ACCENT, 0.32) } else { PANEL3 });
        } else if resp.hovered() {
            p.rect_filled(rect, 0.0, LINE_SOFT);
        }
        paint_side_icon(p, pos2(rect.left() + 13.0, rect.center().y), icon, if selected { TEXT } else { TEXT_DIM });
        p.with_clip_rect(rect).text(pos2(rect.left() + 24.0, rect.center().y), Align2::LEFT_CENTER, label, body(), TEXT);
        if resp.clicked() {
            if !selected {
                self.browser.section = section;
                self.browser.scroll_top = true;
            }
            self.browser.focused = true;
        }
        resp
    }

    // ------------------------------------------------------------------
    // What the list shows
    // ------------------------------------------------------------------

    fn section_nodes(&mut self, section: &Section) -> Vec<Node> {
        match section {
            Section::Category(c) => self.category_nodes(*c),
            Section::Collection(i) => self.collection_nodes(*i),
            Section::Place(p) => self.dir_nodes(p, false),
        }
    }

    fn category_nodes(&mut self, c: Category) -> Vec<Node> {
        match c {
            Category::All => Category::LIST[1..].iter().map(|&c| Node::folder(c.name(), Icon::Folder, self.category_nodes(c))).collect(),
            Category::Sounds => {
                let mut groups: BTreeMap<&str, Vec<Node>> = BTreeMap::new();
                for (kind, node) in self.sounds() {
                    if kind != "Drums" {
                        groups.entry(kind).or_default().push(node);
                    }
                }
                groups.into_iter().map(|(kind, kids)| Node::folder(kind, Icon::Folder, kids)).collect()
            }
            Category::Drums => {
                let mut hits: Vec<Node> = self.sounds().into_iter().filter(|(kind, _)| *kind == "Drums").map(|(_, n)| n).collect();
                hits.sort_by(|a, b| a.label().cmp(b.label()));
                let mut out = drum_engine_nodes();
                out.push(Node::folder("Drum Hits", Icon::Folder, hits));
                out
            }
            Category::Instruments => self.instrument_nodes(),
            Category::AudioEffects => {
                let mut out = effect_nodes();
                let fx = self.plugin_effect_nodes();
                if !fx.is_empty() {
                    out.push(Node::folder("Plug-ins", Icon::Folder, fx));
                }
                out
            }
            Category::MidiEffects => midi_nodes(),
            Category::Plugins => self.plugin_nodes(),
            Category::Clips => self.clip_nodes(),
            Category::Samples => {
                let places = self.places();
                if places.is_empty() {
                    return vec![Node::Note("Add a folder with samples to Places (bottom of the sidebar).".into())];
                }
                places
                    .into_iter()
                    .map(|(label, path, _)| Node::Folder { tag: Some(dir_tag(&path)), label, icon: Icon::Folder, hint: path.display().to_string(), load: None, kids: Kids::Dir { path, samples_only: true } })
                    .collect()
            }
        }
    }

    /// Every preset of the built-in instruments (and the first FM bank),
    /// with the kind of sound it makes.
    fn sounds(&self) -> Vec<(&'static str, Node)> {
        let mut out = Vec::new();
        for d in devices() {
            for p in d.presets {
                if let Some(kind) = sound_type(p).map(|k| d.kind.unwrap_or(k)) {
                    out.push((kind, Node::item(format!("inst/{}/{p}", d.name), *p, Icon::Preset, d.hint, (d.make)(p)).detail(d.name)));
                }
            }
        }
        if let Some(bank) = self.fm_banks.first() {
            for (i, v) in bank.voices.iter().enumerate() {
                if let Some(kind) = sound_type(&v.name()) {
                    out.push((kind, fm_voice(bank, i, v).detail("FM")));
                }
            }
        }
        out
    }

    /// A popup with every instrument and its presets (as in the browser),
    /// for `track`, at `at`, while `menu_id` is open in memory.
    pub(crate) fn instrument_popup(&mut self, ui: &Ui, track: usize, menu_id: egui::Id, at: egui::Pos2) {
        if !ui.ctx().memory(|m| m.data.get_temp::<bool>(menu_id).unwrap_or(false)) {
            return;
        }
        let mut picked: Option<BrowserItem> = None;
        let mut close = false;
        egui::Area::new(menu_id.with("area")).fixed_pos(at).order(egui::Order::Foreground).constrain(true).show(ui.ctx(), |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                ui.label(RichText::new("Instruments: pick one, or a preset from its list").small().color(TEXT_DIM));
                let mut all: Vec<(&'static str, &'static str, Vec<(String, BrowserItem)>)> = devices().into_iter().map(|d| (d.name, d.hint, d.presets.iter().map(|p| (p.to_string(), (d.make)(p))).collect())).collect();
                for e in drums::DrumEngine::ALL {
                    all.push((e.name(), e.description(), e.kits().iter().map(|k| (k.to_string(), BrowserItem::Drums(k))).collect()));
                }
                all.push(("Sampler", "Plays samples: drop a sample on it afterwards", vec![("Sampler".to_string(), BrowserItem::Sampler)]));
                all.sort_by_key(|a| a.0);
                // Columns of names; each opens its presets.
                let per = all.len().div_ceil(3);
                ui.horizontal_top(|ui| {
                    for col in all.chunks(per.max(1)) {
                        ui.vertical(|ui| {
                            ui.set_min_width(130.0);
                            for (name, hint, presets) in col {
                                if presets.len() == 1 {
                                    if ui.button(*name).on_hover_text(*hint).clicked() {
                                        picked = Some(presets[0].1.clone());
                                    }
                                } else {
                                    ui.menu_button(*name, |ui| {
                                        for (label, item) in presets {
                                            if ui.button(label).clicked() {
                                                picked = Some(item.clone());
                                            }
                                        }
                                    })
                                    .response
                                    .on_hover_text(*hint);
                                }
                            }
                        });
                    }
                });
                ui.add_space(4.0);
                if ui.button("Cancel").clicked() {
                    close = true;
                }
            });
        });
        if let Some(item) = picked {
            self.apply_item(&item, crate::app::DropTarget::Track(track));
            close = true;
        }
        if close || super::devices::popup_dismissed(ui) {
            ui.ctx().memory_mut(|m| m.data.insert_temp(menu_id, false));
        }
    }

    fn instrument_nodes(&self) -> Vec<Node> {
        let mut out: Vec<Node> = devices()
            .into_iter()
            .map(|d| {
                let kids = if d.name == "Guitar" {
                    // Its family, a folder each.
                    krabton_engine::dsp::guitar::FAMILY
                        .iter()
                        .map(|(name, presets)| {
                            let kids = presets.iter().map(|p| Node::item(format!("inst/{}/{p}", d.name), *p, Icon::Preset, d.hint, (d.make)(p))).collect();
                            Node::Folder { tag: None, label: (*name).into(), icon: Icon::Folder, hint: d.hint.into(), load: Some((d.make)(presets[0])), kids: Kids::Nodes(kids) }
                        })
                        .collect()
                } else {
                    d.presets.iter().map(|p| Node::item(format!("inst/{}/{p}", d.name), *p, Icon::Preset, d.hint, (d.make)(p))).collect()
                };
                Node::Folder { tag: None, label: d.name.into(), icon: Icon::Device, hint: d.hint.into(), load: Some((d.make)(d.presets[0])), kids: Kids::Nodes(kids) }
            })
            .collect();
        let mut fm = vec![
            Node::Action { label: "Load .syx files...".into(), hint: "Load DX7 banks or voices (Dexed / MiniDexed .syx files)".into(), act: Act::LoadSyx },
            Node::Action { label: "Load a folder of .syx files...".into(), hint: "Load every .syx in a folder, e.g. MiniDexed's sysex/voice".into(), act: Act::LoadSyxFolder },
        ];
        for (bi, bank) in self.fm_banks.iter().enumerate() {
            let voices: Vec<Node> = bank.voices.iter().enumerate().map(|(i, v)| fm_voice(bank, i, v)).collect();
            if bi == 0 {
                fm.extend(voices);
            } else {
                let hint = bank.path.as_ref().map(|p| p.display().to_string()).unwrap_or_default();
                fm.push(Node::Folder { tag: None, label: bank.name.clone(), icon: Icon::Bank, hint, load: None, kids: Kids::Nodes(voices) });
            }
        }
        let first = self.fm_banks.first().and_then(|b| b.voices.first().map(|v| BrowserItem::FmVoice { bank: b.name.clone(), voice: v.0.to_vec() }));
        out.push(Node::Folder {
            tag: None,
            label: "FM".into(),
            icon: Icon::Device,
            hint: "Six-operator FM synth that plays DX7 voices (Dexed / MiniDexed .syx banks)".into(),
            load: first,
            kids: Kids::Nodes(fm),
        });
        out.extend(drum_engine_nodes());
        out.push(Node::item("inst/Sampler", "Sampler", Icon::Device, "Plays samples: one across the keyboard, a multi-sampled instrument or SFZ, or slices of a loop", BrowserItem::Sampler));
        out.sort_by(|a, b| a.label().cmp(b.label()));
        out
    }

    fn plugin_nodes(&self) -> Vec<Node> {
        let mut out = vec![
            Node::Action {
                label: if self.plugins.scanned { "Rescan plug-ins".into() } else { "Scan for plug-ins".into() },
                hint: "Look for CLAP plugins in the standard folders".into(),
                act: Act::ScanPlugins,
            },
            Node::Action { label: "Add a plug-in folder...".into(), hint: "Add a folder with .clap files".into(), act: Act::AddPluginFolder },
        ];
        // By vendor, instruments and effects together (as in Live).
        let mut vendors: BTreeMap<String, Vec<Node>> = BTreeMap::new();
        let vendor = |info: &krabton_clap::PluginInfo| if info.vendor.trim().is_empty() { "Unknown".to_string() } else { info.vendor.clone() };
        for info in self.plugins.instruments() {
            let hint = format!("{} by {} (instrument)\n{}\n{}", info.name, info.vendor, info.description, info.path.display());
            vendors.entry(vendor(info)).or_default().push(Node::item(format!("plugin/{}", info.id), &info.name, Icon::Plugin, hint, BrowserItem::Plugin(info.clone())));
        }
        for info in self.plugins.effects() {
            vendors.entry(vendor(info)).or_default().push(plugin_effect_node(info));
        }
        if vendors.is_empty() {
            let folders: Vec<String> = self.plugins.folders().iter().map(|f| f.display().to_string()).collect();
            out.push(Node::Note(format!("No CLAP plug-ins found in {}. Install some (e.g. Surge XT, Dexed, LSP, Dragonfly Reverb, CLAP versions), or add a folder.", folders.join(", "))));
        }
        for (vendor, mut kids) in vendors {
            kids.sort_by(|a, b| a.label().to_lowercase().cmp(&b.label().to_lowercase()));
            out.push(Node::folder(vendor, Icon::Folder, kids));
        }
        if !self.plugins.scan_errors.is_empty() {
            let errors = self.plugins.scan_errors.iter().map(|e| Node::Note(e.clone())).collect();
            out.push(Node::folder(format!("{} file(s) could not be loaded", self.plugins.scan_errors.len()), Icon::Folder, errors));
        }
        out
    }

    /// The effect plug-ins, for the Audio Effects category.
    fn plugin_effect_nodes(&self) -> Vec<Node> {
        let mut v: Vec<Node> = self.plugins.effects().map(plugin_effect_node).collect();
        v.sort_by(|a, b| a.label().to_lowercase().cmp(&b.label().to_lowercase()));
        v
    }

    /// The clips in this project, per track.
    fn clip_nodes(&self) -> Vec<Node> {
        let mut out = Vec::new();
        for track in &self.project.tracks {
            let kids: Vec<Node> = track
                .slots
                .iter()
                .enumerate()
                .filter_map(|(s, c)| {
                    let c = c.as_ref()?;
                    let name = if c.name.is_empty() { "Clip".to_string() } else { c.name.clone() };
                    let item = BrowserItem::Clip { id: c.id, name: name.clone() };
                    Some(Node::Item { tag: None, label: name, detail: format!("Scene {}", s + 1), icon: Icon::Clip, hint: "Drag onto a slot or the arrangement to copy it".into(), item })
                })
                .collect();
            if !kids.is_empty() {
                out.push(Node::folder(track.name.clone(), Icon::Folder, kids));
            }
        }
        if out.is_empty() {
            out.push(Node::Note("No clips in this project yet.".into()));
        }
        out
    }

    fn collection_nodes(&mut self, c: usize) -> Vec<Node> {
        let mut tagged: Vec<_> = self.prefs.tagged.iter().filter(|t| t.collections & (1 << c) != 0).cloned().collect();
        if tagged.is_empty() {
            return vec![Node::Note(format!("Nothing in {} yet. Right-click an item, or select it and press {}.", COLLECTIONS[c].0, c + 1))];
        }
        tagged.sort_by(|a, b| a.label.to_lowercase().cmp(&b.label.to_lowercase()));
        // Everything that can be tagged and is not a file.
        let mut index = HashMap::new();
        let mut all = self.instrument_nodes();
        all.extend(effect_nodes());
        all.extend(midi_nodes());
        all.extend(self.plugin_nodes());
        fn collect(nodes: Vec<Node>, index: &mut HashMap<String, Node>) {
            for n in nodes {
                let key = match &n {
                    Node::Item { tag: Some(t), .. } => Some(t.clone()),
                    _ => None,
                };
                match (key, n) {
                    (Some(k), n) => {
                        index.entry(k).or_insert(n);
                    }
                    (None, Node::Folder { kids: Kids::Nodes(kids), .. }) => collect(kids, index),
                    _ => {}
                }
            }
        }
        collect(all, &mut index);
        tagged
            .into_iter()
            .map(|t| {
                if let Some(p) = t.key.strip_prefix("dir/").map(PathBuf::from).filter(|p| p.is_dir()) {
                    Node::Folder { tag: Some(t.key.clone()), label: t.label, icon: Icon::Folder, hint: p.display().to_string(), load: None, kids: Kids::Dir { path: p, samples_only: false } }
                } else if let Some(p) = t.key.strip_prefix("file/").map(PathBuf::from).filter(|p| p.is_file()) {
                    if has_ext(&p, "syx") {
                        Node::Folder { tag: Some(t.key.clone()), label: t.label, icon: Icon::Bank, hint: p.display().to_string(), load: None, kids: Kids::Syx(p) }
                    } else if has_ext(&p, "sfz") {
                        Node::item(t.key.clone(), t.label, Icon::Device, "SFZ instrument", BrowserItem::SamplerPatch(p))
                    } else {
                        Node::item(t.key.clone(), t.label, Icon::Sample, SAMPLE_HINT, BrowserItem::Sample(p))
                    }
                } else {
                    index.remove(&t.key).unwrap_or_else(|| Node::Note(format!("{} (not found)", t.label)))
                }
            })
            .collect()
    }

    fn dir_nodes(&mut self, dir: &Path, samples_only: bool) -> Vec<Node> {
        let entries = self.browser.dirs.entry(dir.to_path_buf()).or_insert_with(|| list_dir(dir)).clone();
        let mut out = Vec::new();
        for (path, is_dir) in entries {
            let name = file_name(&path);
            if is_dir {
                out.push(Node::Folder { tag: Some(dir_tag(&path)), label: name, icon: Icon::Folder, hint: String::new(), load: None, kids: Kids::Dir { path, samples_only } });
            } else if has_ext(&path, "wav") || has_ext(&path, "flac") {
                out.push(Node::item(file_tag(&path), name, Icon::Sample, SAMPLE_HINT, BrowserItem::Sample(path)));
            } else if has_ext(&path, "sfz") {
                out.push(Node::item(file_tag(&path), name, Icon::Device, "SFZ instrument: double-click or drag onto a track to play it with the Sampler", BrowserItem::SamplerPatch(path)));
            } else if !samples_only && has_ext(&path, "syx") {
                out.push(Node::Folder { tag: Some(file_tag(&path)), label: name, icon: Icon::Bank, hint: "DX7 bank: open it for its voices".into(), load: None, kids: Kids::Syx(path) });
            }
        }
        if out.is_empty() {
            out.push(Node::Note(if samples_only { "No samples here" } else { "Nothing Krabton can open here" }.into()));
        }
        out
    }

    fn syx_nodes(&mut self, path: &Path) -> Vec<Node> {
        let voices = self.browser.syx.entry(path.to_path_buf()).or_insert_with(|| std::fs::read(path).ok().and_then(|d| dx7::parse_sysex(&d).ok()).unwrap_or_default()).clone();
        if voices.is_empty() {
            return vec![Node::Note("Not a DX7 bank".into())];
        }
        let bank = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        voices.iter().map(|v| Node::Item { tag: None, label: v.name(), detail: String::new(), icon: Icon::Preset, hint: FM_HINT.into(), item: BrowserItem::FmVoice { bank: bank.clone(), voice: v.0.to_vec() } }).collect()
    }

    fn kids(&mut self, kids: Kids, budget: &mut usize) -> Vec<Node> {
        match kids {
            Kids::Nodes(v) => v,
            Kids::Dir { path, samples_only } => {
                *budget = budget.saturating_sub(1);
                self.dir_nodes(&path, samples_only)
            }
            Kids::Syx(path) => self.syx_nodes(&path),
        }
    }

    /// Turns the tree into the rows to draw: open folders show their
    /// contents. While searching, only matches show, with the folders
    /// they are in opened; a folder whose name matches shows everything in
    /// it (`force`).
    #[allow(clippy::too_many_arguments)]
    fn flatten(&mut self, nodes: Vec<Node>, depth: usize, parent: &str, hay: &str, tokens: &[String], force: bool, budget: &mut usize, out: &mut Vec<Row>) -> bool {
        let searching = !tokens.is_empty();
        let mut any = false;
        let mut seen: HashMap<String, usize> = HashMap::new();
        for node in nodes {
            let n = seen.entry(node.label().to_string()).or_insert(0);
            let id = if *n == 0 { format!("{parent}/{}", node.label()) } else { format!("{parent}/{}#{n}", node.label()) };
            *n += 1;
            match node {
                Node::Note(text) => {
                    if !searching {
                        out.push(Row { depth, id, tag: None, label: text, detail: String::new(), icon: None, hint: String::new(), kind: RowKind::Note });
                    }
                }
                Node::Action { label, hint, act } => {
                    if !searching {
                        out.push(Row { depth, id, tag: None, label, detail: String::new(), icon: Some(Icon::Action), hint, kind: RowKind::Action(act) });
                    }
                }
                Node::Item { tag, label, detail, icon, hint, item } => {
                    if !searching || force || matches(tokens, &[hay, &label.to_lowercase(), &detail.to_lowercase()]) {
                        out.push(Row { depth, id, tag, label, detail, icon: Some(icon), hint, kind: RowKind::Item(item) });
                        any = true;
                    }
                }
                Node::Folder { tag, label, icon, hint, load, kids } => {
                    let hay2 = format!("{hay} {}", label.to_lowercase());
                    let dir = match &kids {
                        Kids::Dir { path, .. } => Some(path.clone()),
                        _ => None,
                    };
                    let row = |open: bool, auto: bool| Row { depth, id: id.clone(), tag: tag.clone(), label: label.clone(), detail: String::new(), icon: Some(icon), hint: hint.clone(), kind: RowKind::Folder { open, auto, load: load.clone(), dir: dir.clone() } };
                    if !searching || force {
                        let open = self.browser.open.contains(&id);
                        out.push(row(open, false));
                        any = true;
                        if open {
                            let kids = self.kids(kids, budget);
                            self.flatten(kids, depth + 1, &id, &hay2, tokens, force, budget, out);
                        }
                        continue;
                    }
                    let open = !self.browser.closed_in_search.contains(&id);
                    if matches(tokens, &[&hay2]) {
                        out.push(row(open, true));
                        any = true;
                        if open {
                            let kids = self.kids(kids, budget);
                            self.flatten(kids, depth + 1, &id, &hay2, tokens, true, budget, out);
                        }
                        continue;
                    }
                    // Look inside (folders on disk only within the budget).
                    if dir.is_some() && *budget == 0 {
                        continue;
                    }
                    let kids = self.kids(kids, budget);
                    let mut sub = Vec::new();
                    if self.flatten(kids, depth + 1, &id, &hay2, tokens, false, budget, &mut sub) {
                        out.push(row(open, true));
                        any = true;
                        if open {
                            out.extend(sub);
                        }
                    }
                }
            }
        }
        any
    }

    // ------------------------------------------------------------------
    // The list
    // ------------------------------------------------------------------

    fn browser_list(&mut self, ui: &mut Ui) {
        if self.browser.filter != self.browser.last_filter {
            self.browser.last_filter = self.browser.filter.clone();
            self.browser.closed_in_search.clear();
            self.browser.scroll_top = true;
        }
        let tokens: Vec<String> = self.browser.filter.to_lowercase().split_whitespace().map(String::from).collect();
        let section = self.browser.section.clone();
        let root = format!("{section:?}");
        let nodes = self.section_nodes(&section);
        let mut rows = Vec::new();
        let mut budget = SEARCH_DIRS;
        self.flatten(nodes, 0, &root, "", &tokens, false, &mut budget, &mut rows);
        if self.browser.select_first {
            self.browser.select_first = false;
            if let Some(r) = rows.iter().find(|r| !matches!(r.kind, RowKind::Note)) {
                self.browser.selected = Some(r.id.clone());
                self.browser.scroll_to_selected = true;
            }
        }

        // Column header, as in Live.
        let (hr, _) = ui.allocate_exact_size(vec2(ui.available_width(), 20.0), Sense::hover());
        let p = ui.painter();
        p.rect_filled(hr, 0.0, PANEL2);
        p.line_segment([hr.left_bottom(), hr.right_bottom()], Stroke::new(1.0_f32, LINE));
        p.text(pos2(hr.left() + 8.0, hr.center().y), Align2::LEFT_CENTER, "Name", small(), TEXT_DIM);

        let mut acts = Vec::new();
        let mut area = egui::ScrollArea::vertical().id_salt(("browser_list", &root)).auto_shrink([false, false]);
        if self.browser.scroll_top {
            self.browser.scroll_top = false;
            area = area.vertical_scroll_offset(0.0);
        }
        area.show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            for row in &rows {
                self.browser_row(ui, row, &mut acts);
            }
            if rows.is_empty() {
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    ui.label(RichText::new(format!("No results in {}", section.name())).small().color(TEXT_DIM));
                });
                if section != Section::Category(Category::All) {
                    ui.horizontal(|ui| {
                        ui.add_space(8.0);
                        if ui.small_button("Search everything").clicked() {
                            acts.push(UiAct::Show(Section::Category(Category::All)));
                        }
                    });
                }
            }
            // The empty space below the rows also focuses the browser.
            let rest = ui.available_rect_before_wrap();
            if rest.height() > 0.0 && ui.interact(rest, ui.id().with("browser_rest"), Sense::click()).clicked() {
                self.browser.focused = true;
            }
        });
        self.browser.scroll_to_selected = false;
        self.browser.rows = rows;
        for a in acts {
            self.browser_do(a);
        }
    }

    fn browser_row(&self, ui: &mut Ui, row: &Row, acts: &mut Vec<UiAct>) {
        let x0 = 6.0 + row.depth as f32 * INDENT;
        if let RowKind::Note = row.kind {
            ui.horizontal(|ui| {
                ui.add_space(x0 + 10.0);
                ui.add(egui::Label::new(RichText::new(&row.label).small().color(TEXT_DIM)).wrap());
            });
            ui.add_space(3.0);
            return;
        }
        let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), ROW_H), Sense::hover());
        let selected = self.browser.selected.as_deref() == Some(row.id.as_str());
        if selected && self.browser.scroll_to_selected {
            ui.scroll_to_rect(rect, None);
        }
        if !ui.is_rect_visible(rect) {
            return;
        }
        let resp = ui.interact(rect, ui.id().with(("browser_row", &row.id)), Sense::click_and_drag());
        let p = ui.painter_at(rect);
        if selected {
            p.rect_filled(rect, 0.0, if self.browser.focused { mix(PANEL, ACCENT, 0.32) } else { PANEL3 });
        } else if resp.hovered() {
            p.rect_filled(rect, 0.0, LINE_SOFT);
        }
        let cy = rect.center().y;
        let x = rect.left() + x0;
        if let RowKind::Folder { open, .. } = row.kind {
            paint_triangle(&p, pos2(x + 4.0, cy), open, if selected { TEXT } else { TEXT_DIM });
        }
        if let Some(icon) = row.icon {
            paint_icon(&p, pos2(x + 16.0, cy), icon, if icon == Icon::Action { ACCENT } else { TEXT_DIM });
        }
        let tx = x + 25.0;
        let mut right = rect.right() - 6.0;
        if !row.detail.is_empty() {
            let r = p.text(pos2(right, cy), Align2::RIGHT_CENTER, &row.detail, small(), TEXT_DIM);
            right = r.left() - 8.0;
        }
        if let Some(tag) = &row.tag {
            let bits = self.prefs.collections(tag);
            for (i, (_, color)) in COLLECTIONS.iter().enumerate().rev() {
                if bits & (1 << i) != 0 {
                    p.circle_filled(pos2(right - 3.0, cy), 3.0, *color);
                    right -= 8.0;
                }
            }
        }
        let color = if let RowKind::Action(_) = row.kind { ACCENT } else { TEXT };
        p.with_clip_rect(Rect::from_min_max(pos2(tx, rect.top()), pos2(right.max(tx), rect.bottom()))).text(pos2(tx, cy), Align2::LEFT_CENTER, &row.label, body(), color);

        let item = row.item().cloned();
        if let Some(it) = &item {
            resp.dnd_set_drag_payload(it.clone());
        }
        if resp.clicked() {
            acts.push(UiAct::Select(row.id.clone()));
            match &row.kind {
                RowKind::Folder { .. } => acts.push(UiAct::Toggle(row.clone())),
                RowKind::Item(BrowserItem::Sample(path)) => acts.push(UiAct::Sample(path.clone())),
                RowKind::Action(a) => acts.push(UiAct::Run(*a)),
                _ => {}
            }
        }
        // egui counts two quick clicks anywhere as a double-click; only one
        // on the row the first click selected loads.
        if resp.double_clicked() && selected {
            if let Some(it) = &item {
                acts.push(UiAct::Load(it.clone()));
            }
        }
        let dir = match &row.kind {
            RowKind::Folder { dir, .. } => dir.clone(),
            _ => None,
        };
        if item.is_some() || row.tag.is_some() || dir.is_some() {
            resp.context_menu(|ui| {
                if let Some(it) = &item {
                    if ui.button("Load on the selected track").clicked() {
                        acts.push(UiAct::Load(it.clone()));
                        ui.close();
                    }
                }
                if let Some(tag) = &row.tag {
                    ui.separator();
                    let bits = self.prefs.collections(tag);
                    for (i, (name, color)) in COLLECTIONS.iter().enumerate() {
                        let mut on = bits & (1 << i) != 0;
                        if ui.checkbox(&mut on, RichText::new(format!("{name}  ({})", i + 1)).color(*color)).clicked() {
                            acts.push(UiAct::Tag(tag.clone(), row.label.clone(), i));
                        }
                    }
                    if bits != 0 && ui.button("Remove from all collections  (0)").clicked() {
                        acts.push(UiAct::Untag(tag.clone()));
                        ui.close();
                    }
                }
                if let Some(d) = &dir {
                    ui.separator();
                    if ui.button("Add to Places").clicked() {
                        acts.push(UiAct::AddPlace(d.clone()));
                        ui.close();
                    }
                }
            });
        }
        if !row.hint.is_empty() {
            resp.on_hover_text(&row.hint);
        }
    }

    fn browser_preview(&mut self, ui: &mut Ui) {
        ui.horizontal_centered(|ui| {
            let on = self.browser.preview;
            if ui.selectable_label(on, "Preview").on_hover_text("Play samples when they are selected").clicked() {
                self.browser.preview = !on;
                if on {
                    self.cmd(Command::Preview(None));
                }
            }
            let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), ui.available_height().max(10.0)), Sense::click());
            let p = ui.painter_at(r);
            p.rect_filled(r, 2.0, shade(PANEL, 0.8));
            match &self.browser.preview_sample {
                Some(s) => {
                    let peaks = &s.0.peaks;
                    if !peaks.is_empty() {
                        let cols = r.width().max(1.0) as usize;
                        let mid = r.center().y;
                        let half = r.height() * 0.45;
                        for c in 0..cols {
                            let a = c * peaks.len() / cols;
                            let b = ((c + 1) * peaks.len() / cols).max(a + 1).min(peaks.len());
                            let (lo, hi) = peaks[a..b].iter().fold((0.0f32, 0.0f32), |(lo, hi), &(l, h)| (lo.min(l), hi.max(h)));
                            let x = r.left() + c as f32 + 0.5;
                            p.line_segment([pos2(x, mid - hi.clamp(-1.0, 1.0) * half), pos2(x, mid - lo.clamp(-1.0, 1.0) * half - 0.5)], Stroke::new(1.0_f32, mix(TEXT_DIM, ACCENT, 0.5)));
                        }
                    }
                    p.text(r.left_top() + vec2(4.0, 2.0), Align2::LEFT_TOP, &s.0.name, small(), TEXT);
                    if resp.on_hover_text("Click to play it again").clicked() {
                        let s = s.clone();
                        self.cmd(Command::Preview(Some(s)));
                    }
                }
                None => {
                    p.text(r.center(), Align2::CENTER_CENTER, "Select a sample to hear it", small(), TEXT_DIM);
                }
            }
        });
    }

    // ------------------------------------------------------------------
    // Doing things
    // ------------------------------------------------------------------

    fn browser_do(&mut self, a: UiAct) {
        match a {
            UiAct::Select(id) => {
                self.browser.selected = Some(id);
                self.browser.preview_sample = None;
                self.browser.focused = true;
            }
            UiAct::Toggle(row) => self.browser_toggle(&row),
            UiAct::Sample(path) => self.browser_sample(&path),
            UiAct::Load(item) => {
                let target = self.default_target();
                self.apply_item(&item, target);
            }
            UiAct::Run(act) => self.browser_run(act),
            UiAct::Tag(key, label, c) => self.prefs.toggle_collection(&key, &label, c),
            UiAct::Untag(key) => self.prefs.clear_collections(&key),
            UiAct::AddPlace(dir) => {
                if !self.prefs.places.contains(&dir) {
                    self.prefs.places.push(dir.clone());
                    self.prefs.save();
                }
                self.browser.section = Section::Place(dir);
                self.browser.scroll_top = true;
            }
            UiAct::RemovePlace(dir) => {
                self.prefs.places.retain(|p| *p != dir);
                self.prefs.save();
                if self.browser.section == Section::Place(dir) {
                    self.browser.section = Section::Category(Category::Samples);
                }
            }
            UiAct::Show(section) => {
                self.browser.section = section;
                self.browser.scroll_top = true;
            }
        }
    }

    fn browser_toggle(&mut self, row: &Row) {
        let RowKind::Folder { open, auto, dir, .. } = &row.kind else { return };
        if *auto {
            if *open {
                self.browser.closed_in_search.insert(row.id.clone());
            } else {
                self.browser.closed_in_search.remove(&row.id);
            }
        } else if *open {
            self.browser.open.remove(&row.id);
        } else {
            self.browser.open.insert(row.id.clone());
            // Read the folder again: it may have changed on disk.
            if let Some(d) = dir {
                self.browser.dirs.remove(d);
            }
        }
    }

    /// A sample was selected: show it, and play it when previewing.
    fn browser_sample(&mut self, path: &Path) {
        if let Some(s) = self.load_sample(path) {
            if self.browser.preview {
                self.cmd(Command::Preview(Some(s.clone())));
            }
            self.browser.preview_sample = Some(s);
        }
    }

    fn browser_run(&mut self, act: Act) {
        match act {
            Act::ScanPlugins => {
                self.plugins.scan();
                let (n, fx) = (self.plugins.instruments().count(), self.plugins.effects().count());
                self.toast(format!("Found {n} CLAP instrument(s) and {fx} effect(s)"));
            }
            Act::AddPluginFolder => {
                if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                    if !self.prefs.plugin_folders.contains(&dir) {
                        self.prefs.plugin_folders.push(dir.clone());
                        self.prefs.save();
                    }
                    self.plugins.add_folder(dir);
                }
            }
            Act::LoadSyx => {
                if let Some(files) = rfd::FileDialog::new().add_filter("DX7 sysex", &["syx", "SYX"]).pick_files() {
                    let mut ok = 0;
                    for f in files {
                        match self.load_fm_bank(&f) {
                            Ok(_) => ok += 1,
                            Err(e) => self.toast(e),
                        }
                    }
                    self.prefs.save();
                    if ok > 0 {
                        self.toast(format!("Loaded {ok} bank(s)"));
                    }
                }
            }
            Act::LoadSyxFolder => {
                if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                    let n = self.load_fm_folder(&dir);
                    self.toast(format!("Loaded {n} bank(s)"));
                }
            }
        }
    }

    /// Keys for the focused browser (rows from the last frame). Returns
    /// whether the key was used.
    pub(crate) fn browser_key(&mut self, key: Key) -> bool {
        if !self.browser.focused || !self.browser.visible {
            return false;
        }
        let rows = self.browser.rows.clone();
        let cur = self.browser.selected.as_ref().and_then(|id| rows.iter().position(|r| &r.id == id));
        let pickable = |i: &usize| !matches!(rows[*i].kind, RowKind::Note);
        let select = |app: &mut App, i: usize| {
            app.browser.selected = Some(rows[i].id.clone());
            app.browser.scroll_to_selected = true;
            app.browser.preview_sample = None;
            if let RowKind::Item(BrowserItem::Sample(p)) = &rows[i].kind {
                app.browser_sample(p);
            }
        };
        let number = |k: Key| [Key::Num0, Key::Num1, Key::Num2, Key::Num3, Key::Num4, Key::Num5, Key::Num6, Key::Num7].iter().position(|n| *n == k);
        match key {
            Key::ArrowDown => {
                if let Some(i) = (cur.map(|c| c + 1).unwrap_or(0)..rows.len()).find(pickable) {
                    select(self, i);
                }
            }
            Key::ArrowUp => {
                if let Some(i) = (0..cur.unwrap_or(rows.len())).rev().find(pickable) {
                    select(self, i);
                }
            }
            Key::ArrowRight => {
                if let Some(c) = cur {
                    if let RowKind::Folder { open, .. } = rows[c].kind {
                        if !open {
                            self.browser_toggle(&rows[c]);
                        } else if rows.get(c + 1).is_some_and(|r| r.depth > rows[c].depth) {
                            select(self, c + 1);
                        }
                    }
                }
            }
            Key::ArrowLeft => {
                if let Some(c) = cur {
                    match rows[c].kind {
                        RowKind::Folder { open: true, .. } => self.browser_toggle(&rows[c]),
                        _ => {
                            if let Some(i) = (0..c).rev().find(|&i| rows[i].depth < rows[c].depth) {
                                select(self, i);
                            }
                        }
                    }
                }
            }
            Key::Enter => {
                if let Some(c) = cur {
                    match (&rows[c].kind, rows[c].item()) {
                        (_, Some(item)) => {
                            let item = item.clone();
                            self.browser_do(UiAct::Load(item));
                        }
                        (RowKind::Folder { .. }, None) => self.browser_toggle(&rows[c]),
                        (RowKind::Action(a), _) => self.browser_run(*a),
                        _ => {}
                    }
                }
            }
            Key::Escape => self.browser.focused = false,
            k => {
                let Some(n) = number(k) else { return false };
                if let Some(row) = cur.map(|c| &rows[c]) {
                    if let Some(tag) = &row.tag {
                        if n == 0 {
                            self.prefs.clear_collections(tag);
                        } else {
                            self.prefs.toggle_collection(tag, &row.label, n - 1);
                        }
                    }
                }
            }
        }
        true
    }
}

fn side_heading(ui: &mut Ui, text: &str) {
    ui.add_space(8.0);
    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 16.0), Sense::hover());
    ui.painter().text(pos2(r.left() + 7.0, r.center().y), Align2::LEFT_CENTER, text, small(), TEXT_DIM);
}

#[derive(Clone, Copy)]
enum SideIcon {
    Dot(Color32),
    Category(Category),
    Folder,
    Add,
}

fn paint_side_icon(p: &egui::Painter, c: Pos2, icon: SideIcon, col: Color32) {
    let s = Stroke::new(1.2_f32, col);
    match icon {
        SideIcon::Dot(color) => {
            p.circle_filled(c, 4.0, color);
        }
        SideIcon::Folder => paint_icon(p, c, Icon::Folder, col),
        SideIcon::Add => {
            p.line_segment([c - vec2(4.0, 0.0), c + vec2(4.0, 0.0)], s);
            p.line_segment([c - vec2(0.0, 4.0), c + vec2(0.0, 4.0)], s);
        }
        SideIcon::Category(cat) => match cat {
            Category::All => {
                for dy in [-3.0, 0.0, 3.0] {
                    p.line_segment([c + vec2(-4.5, dy), c + vec2(4.5, dy)], s);
                }
            }
            Category::Sounds => {
                // A speaker.
                p.rect_filled(Rect::from_min_max(c + vec2(-4.5, -1.8), c + vec2(-2.0, 1.8)), 0.0, col);
                p.add(Shape::convex_polygon(vec![c + vec2(-2.0, -1.8), c + vec2(1.5, -4.5), c + vec2(1.5, 4.5), c + vec2(-2.0, 1.8)], col, Stroke::NONE));
                p.line_segment([c + vec2(3.5, -2.5), c + vec2(3.5, 2.5)], s);
            }
            Category::Drums => {
                p.circle_stroke(c, 4.2, s);
                p.circle_filled(c, 1.3, col);
            }
            Category::Instruments => {
                let r = Rect::from_center_size(c, vec2(10.0, 7.0));
                p.rect_stroke(r, 1.0, s, StrokeKind::Inside);
                for dx in [-1.7, 1.7] {
                    p.line_segment([pos2(c.x + dx, r.top()), pos2(c.x + dx, c.y + 1.0)], s);
                }
            }
            Category::AudioEffects => {
                let pts: Vec<Pos2> = (0..=10).map(|i| c + vec2(i as f32 - 5.0, -3.5 * (i as f32 / 10.0 * std::f32::consts::TAU).sin())).collect();
                p.add(Shape::line(pts, s));
            }
            Category::MidiEffects => {
                for (i, dy) in [2.5, -0.5, -3.5].iter().enumerate() {
                    p.rect_filled(Rect::from_center_size(c + vec2(-3.5 + i as f32 * 3.5, *dy), vec2(3.0, 2.0)), 0.0, col);
                }
            }
            Category::Plugins => paint_icon(p, c, Icon::Plugin, col),
            Category::Clips => paint_icon(p, c, Icon::Clip, col),
            Category::Samples => paint_icon(p, c, Icon::Sample, col),
        },
    }
}

fn paint_triangle(p: &egui::Painter, c: Pos2, open: bool, col: Color32) {
    let pts = if open { vec![c + vec2(-3.5, -2.0), c + vec2(3.5, -2.0), c + vec2(0.0, 2.5)] } else { vec![c + vec2(-2.0, -3.5), c + vec2(2.5, 0.0), c + vec2(-2.0, 3.5)] };
    p.add(Shape::convex_polygon(pts, col, Stroke::NONE));
}

fn paint_icon(p: &egui::Painter, c: Pos2, icon: Icon, col: Color32) {
    let s = Stroke::new(1.0_f32, col);
    match icon {
        Icon::Preset => {
            p.rect_stroke(Rect::from_center_size(c, vec2(7.0, 7.0)), 1.5, s, StrokeKind::Inside);
        }
        Icon::Device => {
            p.rect_filled(Rect::from_center_size(c, vec2(7.0, 7.0)), 1.5, col);
        }
        Icon::Sample => {
            for (i, h) in [2.0, 5.0, 8.0, 4.0, 6.0, 2.5].iter().enumerate() {
                let x = c.x - 4.5 + i as f32 * 1.8;
                p.line_segment([pos2(x, c.y - h / 2.0), pos2(x, c.y + h / 2.0)], s);
            }
        }
        Icon::Folder => {
            let r = Rect::from_min_max(c + vec2(-5.0, -2.5), c + vec2(5.0, 4.0));
            p.rect_stroke(r, 1.0, s, StrokeKind::Inside);
            p.line_segment([c + vec2(-5.0, -2.5), c + vec2(-4.0, -4.0)], s);
            p.line_segment([c + vec2(-4.0, -4.0), c + vec2(-0.5, -4.0)], s);
            p.line_segment([c + vec2(-0.5, -4.0), c + vec2(0.5, -2.5)], s);
        }
        Icon::Plugin => {
            p.rect_stroke(Rect::from_min_max(c + vec2(-3.5, -1.5), c + vec2(3.5, 4.0)), 1.0, s, StrokeKind::Inside);
            p.line_segment([c + vec2(-1.5, -1.5), c + vec2(-1.5, -4.5)], s);
            p.line_segment([c + vec2(1.5, -1.5), c + vec2(1.5, -4.5)], s);
        }
        Icon::Clip => {
            p.rect_filled(Rect::from_center_size(c, vec2(10.0, 6.0)), 1.5, col);
        }
        Icon::Bank => {
            p.rect_stroke(Rect::from_min_max(c + vec2(-4.5, -2.0), c + vec2(2.5, 4.5)), 1.0, s, StrokeKind::Inside);
            p.line_segment([c + vec2(-2.5, -4.0), c + vec2(4.5, -4.0)], s);
            p.line_segment([c + vec2(4.5, -4.0), c + vec2(4.5, 2.5)], s);
        }
        Icon::Action => {
            p.line_segment([c - vec2(3.5, 0.0), c + vec2(3.5, 0.0)], Stroke::new(1.4_f32, col));
            p.line_segment([c - vec2(0.0, 3.5), c + vec2(0.0, 3.5)], Stroke::new(1.4_f32, col));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_get_a_kind_of_sound() {
        assert_eq!(sound_type("Init"), None);
        assert_eq!(sound_type("Moog Bass"), Some("Bass"));
        assert_eq!(sound_type("Juno Strings"), Some("Strings"));
        assert_eq!(sound_type("Saw Swarm"), Some("Pad"));
        assert_eq!(sound_type("Steel Drum"), Some("Mallets & Bells"));
        assert_eq!(sound_type("Kick"), Some("Drums"));
        assert_eq!(sound_type("Harmonic Organ"), Some("Keys"));
        assert_eq!(sound_type("KRAB EPNO"), Some("Keys"));
        assert_eq!(sound_type("Rave Stab"), Some("Stabs"));
        assert_eq!(sound_type("Twin Peaks"), Some("Synth Misc"));
        // Every built-in preset finds a place.
        for d in devices() {
            for p in d.presets {
                let _ = sound_type(p);
            }
        }
    }

    #[test]
    fn search_words_can_be_anywhere() {
        let t = |s: &str| s.split_whitespace().map(String::from).collect::<Vec<_>>();
        assert!(matches(&t("analog bass"), &[" instruments analog", "moog bass", ""]));
        assert!(!matches(&t("analog pad"), &[" instruments analog", "moog bass", ""]));
        assert!(matches(&[], &["anything"]));
    }
}

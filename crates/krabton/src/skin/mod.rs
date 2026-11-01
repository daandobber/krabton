//! Character: every machine looks like itself. A skin gives a device its
//! panel (colours, material, wooden cheeks, screws, stripes), its logo in
//! its own lettering, the ink of its labels, the kind of knobs it has and
//! which knobs are its big ones. Options > Plain devices turns it all off
//! for the flat look.

mod catalog;
mod paint;

pub use catalog::{for_effect, for_instrument, for_midi};
pub use paint::{group_backdrop, knob_face, logo, panel, CHEEK, STRIPE};

use crate::theme::{AccentScope, KNOB, PANEL3, TEXT, TEXT_DIM};
use egui::{Color32, FontFamily, FontId};
use std::cell::Cell;

/// The lettering of a logo (or of a skin's labels).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Font {
    /// Krabton's own.
    Ui,
    /// Wide and technical, studio gear (Michroma).
    Wide,
    /// Rounded retro-future (Audiowide).
    Future,
    /// Bold blocks, drum machines (Bungee).
    Block,
    /// Neon tubes (Monoton).
    Neon,
    /// Space age (Orbitron).
    Space,
    /// Eight-bit (Press Start 2P).
    Pixel,
    /// Seventies (Righteous).
    Retro,
    /// Sporty italic-ish (Russo One).
    Sport,
    /// A dot-matrix display (VT323).
    Lcd,
    /// A script, chrome on old instruments (Yellowtail).
    Script,
    /// A grand serif (Abril Fatface).
    Serif,
    /// Tall condensed capitals (Bebas Neue).
    Tall,
}

/// The fonts in the binary: their family names and data.
const FONTS: &[(&str, &[u8])] = &[
    ("k-wide", include_bytes!("../../assets/fonts/Michroma-Regular.ttf")),
    ("k-future", include_bytes!("../../assets/fonts/Audiowide-Regular.ttf")),
    ("k-block", include_bytes!("../../assets/fonts/Bungee-Regular.ttf")),
    ("k-neon", include_bytes!("../../assets/fonts/Monoton-Regular.ttf")),
    ("k-space", include_bytes!("../../assets/fonts/Orbitron-Variable.ttf")),
    ("k-pixel", include_bytes!("../../assets/fonts/PressStart2P-Regular.ttf")),
    ("k-retro", include_bytes!("../../assets/fonts/Righteous-Regular.ttf")),
    ("k-sport", include_bytes!("../../assets/fonts/RussoOne-Regular.ttf")),
    ("k-lcd", include_bytes!("../../assets/fonts/VT323-Regular.ttf")),
    ("k-script", include_bytes!("../../assets/fonts/Yellowtail-Regular.ttf")),
    ("k-serif", include_bytes!("../../assets/fonts/AbrilFatface-Regular.ttf")),
    ("k-tall", include_bytes!("../../assets/fonts/BebasNeue-Regular.ttf")),
];

impl Font {
    fn family(self) -> FontFamily {
        let name = match self {
            Font::Ui => return FontFamily::Proportional,
            Font::Wide => "k-wide",
            Font::Future => "k-future",
            Font::Block => "k-block",
            Font::Neon => "k-neon",
            Font::Space => "k-space",
            Font::Pixel => "k-pixel",
            Font::Retro => "k-retro",
            Font::Sport => "k-sport",
            Font::Lcd => "k-lcd",
            Font::Script => "k-script",
            Font::Serif => "k-serif",
            Font::Tall => "k-tall",
        };
        FontFamily::Name(name.into())
    }

    /// The font at about the visual size `size` (the lettering differ in
    /// how big they look at the same size).
    pub fn id(self, size: f32) -> FontId {
        let k = match self {
            Font::Pixel => 0.62,
            Font::Wide => 0.8,
            Font::Lcd => 1.3,
            Font::Tall => 1.2,
            Font::Script => 1.15,
            Font::Block => 0.9,
            Font::Space => 0.95,
            _ => 1.0,
        };
        FontId::new((size * k).round(), self.family())
    }
}

/// A Japanese font on this computer (for lyrics in kana and voicebanks' names).
fn system_cjk_font() -> Option<Vec<u8>> {
    const CANDIDATES: &[&str] = &[
        "C:/Windows/Fonts/YuGothM.ttc",
        "C:/Windows/Fonts/YuGothR.ttc",
        "C:/Windows/Fonts/meiryo.ttc",
        "C:/Windows/Fonts/msgothic.ttc",
        "/System/Library/Fonts/ヒラギノ角ゴシック W3.ttc",
        "/System/Library/Fonts/Hiragino Sans GB.ttc",
        "/Library/Fonts/Arial Unicode.ttf",
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/google-noto-cjk/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/truetype/droid/DroidSansFallbackFull.ttf",
    ];
    CANDIDATES.iter().find_map(|p| std::fs::read(p).ok())
}

/// Adds the logo fonts (each falls back on Krabton's own for what it lacks).
pub fn install_fonts(ctx: &egui::Context) {
    let mut defs = egui::FontDefinitions::default();
    // Kana and kanji from the system's Japanese font, behind everything else.
    if let Some(cjk) = system_cjk_font() {
        defs.font_data.insert("cjk".to_owned(), std::sync::Arc::new(egui::FontData::from_owned(cjk)));
        for fam in [FontFamily::Proportional, FontFamily::Monospace] {
            defs.families.entry(fam).or_default().push("cjk".to_owned());
        }
    }
    let fallback = defs.families.get(&FontFamily::Proportional).cloned().unwrap_or_default();
    for (name, data) in FONTS {
        defs.font_data.insert((*name).to_owned(), std::sync::Arc::new(egui::FontData::from_static(data)));
        let mut list = vec![(*name).to_owned()];
        list.extend(fallback.iter().cloned());
        defs.families.insert(FontFamily::Name((*name).into()), list);
    }
    ctx.set_fonts(defs);
}

/// What the panel is made of.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Surface {
    /// Painted, top colour to bottom colour.
    Flat,
    /// Brushed metal: fine horizontal lines.
    Brushed,
    /// Textured plastic or hammertone.
    Grain,
    /// Carbon or ribbed black.
    Carbon,
    /// Wood, its grain running across.
    Wood,
    /// Velvet: darker towards the edges.
    Velvet,
    /// A screen's scanlines.
    Scan,
    /// A sunburst: the top colour in the middle, the bottom colour at the rim.
    Burst,
    /// Lacquer: a gloss across the top.
    Gloss,
    /// A sky at night: specks of stars, a few sparkles, a glow low down.
    Night,
}

/// The knobs.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Knob {
    /// Krabton's ring.
    Ring,
    /// Machined metal, knurled at the edge.
    Chrome,
    /// A black skirt with a scale and a metal top, as on old monosynths.
    Vintage,
    /// A dark knob with a coloured cap.
    Cap,
    /// A glowing arc.
    Neon,
    /// A chicken head pointer.
    Chicken,
    /// A small eurorack knob.
    Euro,
    /// Blocks of light, eight-bit.
    Pixel,
    /// A rubber knob in a ring of LEDs.
    Led,
}

/// How a device's groups of knobs are set apart.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Groups {
    /// Just their names.
    Plain,
    /// Each on a darker (or lighter) inset.
    Inset,
    /// Each under a bar with its name.
    Header,
    /// A line around each, the name in its top edge.
    Outline,
}

/// How a logo is written.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Case {
    AsIs,
    Upper,
    Lower,
}

/// A machine's look.
#[derive(Clone, Copy, Debug)]
pub struct Skin {
    /// The panel: top and bottom colours and what it is made of.
    pub top: Color32,
    pub bottom: Color32,
    pub surface: Surface,
    /// Wooden cheeks left and right, in this wood.
    pub cheeks: Option<Color32>,
    pub screws: bool,
    /// Lines of colour across the top of the panel.
    pub stripes: &'static [Color32],
    /// Labels, and values and other small print.
    pub ink: Color32,
    pub dim: Color32,
    /// Insets: choice boxes, switches, tabs; and the ink on them.
    pub well: Color32,
    pub well_ink: Color32,
    /// The machine's colour (knob pointers, arcs, its screens).
    pub accent: Color32,
    pub knob: Knob,
    /// The knob's body or cap.
    pub cap: Color32,
    pub logo: Font,
    pub logo_size: f32,
    pub logo_ink: Color32,
    pub logo_case: Case,
    /// A plate behind the logo.
    pub plate: Option<Color32>,
    pub groups: Groups,
    /// Labels in capitals.
    pub caps: bool,
    /// The knobs drawn big (by name), where there is room.
    pub heroes: &'static [&'static str],
    /// Drum pads, a colour per column (as the step buttons of drum machines).
    pub pads: &'static [Color32],
}

impl Skin {
    /// Whether the knob of parameter `name` is a big one.
    pub fn is_hero(&self, name: &str) -> bool {
        self.heroes.contains(&name)
    }
}

thread_local! {
    static ENABLED: Cell<bool> = const { Cell::new(true) };
    static SLIDERS: Cell<bool> = const { Cell::new(false) };
    static CURRENT: Cell<Option<Skin>> = const { Cell::new(None) };
    static TRACK: Cell<Option<Color32>> = const { Cell::new(None) };
}

/// Character on (the default) or the plain look.
pub fn set_enabled(on: bool) {
    ENABLED.with(|c| c.set(on));
}

pub fn enabled() -> bool {
    ENABLED.with(|c| c.get())
}

/// Sliders instead of knobs where a panel has both (the Touch view, Coast).
pub fn set_sliders(on: bool) {
    SLIDERS.with(|c| c.set(on));
}

pub fn sliders() -> bool {
    SLIDERS.with(|c| c.get())
}

/// The skin of the device being drawn, if it has one.
pub fn current() -> Option<Skin> {
    CURRENT.with(|c| c.get())
}

/// The colour of the track whose device is being drawn in its own skin.
pub fn track_colour() -> Option<Color32> {
    TRACK.with(|c| c.get())
}

/// While it lives, devices are drawn in `skin` (nothing changes with the
/// plain look), and the accent is the skin's.
pub struct Scope {
    prev: Option<Skin>,
    prev_track: Option<Color32>,
    _accent: Option<AccentScope>,
}

impl Scope {
    pub fn new(skin: Option<Skin>) -> Self {
        let skin = skin.filter(|_| enabled());
        let track = skin.map(|_| crate::theme::accent_now());
        let prev = CURRENT.with(|c| c.replace(skin));
        let prev_track = TRACK.with(|c| c.replace(track));
        Self { prev, prev_track, _accent: skin.map(|s| AccentScope::exact(s.accent)) }
    }
}

impl Drop for Scope {
    fn drop(&mut self) {
        let (prev, prev_track) = (self.prev, self.prev_track);
        CURRENT.with(|c| c.set(prev));
        TRACK.with(|c| c.set(prev_track));
    }
}

/// The ink of labels on the device being drawn.
pub fn ink() -> Color32 {
    current().map(|s| s.ink).unwrap_or(TEXT)
}

/// The ink of small print on the device being drawn.
pub fn ink_dim() -> Color32 {
    current().map(|s| s.dim).unwrap_or(TEXT_DIM)
}

/// Insets (choice boxes, switches) on the device being drawn, and their ink.
pub fn well() -> (Color32, Color32) {
    current().map(|s| (s.well, s.well_ink)).unwrap_or((KNOB, TEXT))
}

/// A switch that is off on the device being drawn.
pub fn switch_off() -> Color32 {
    current().map(|s| s.well).unwrap_or(PANEL3)
}

/// A label as the skin writes it.
pub fn label_text(s: &str) -> String {
    match current() {
        Some(k) if k.caps => s.to_uppercase(),
        _ => s.to_owned(),
    }
}

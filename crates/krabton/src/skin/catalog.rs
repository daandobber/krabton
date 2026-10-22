//! Every machine's look.

use super::{Case, Font, Groups, Knob, Skin, Surface};
use egui::Color32;
use krabton_engine::dsp::effects::EffectKind;
use krabton_engine::midi_fx::MidiFxKind;

const fn hex(v: u32) -> Color32 {
    Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8)
}

/// Dark graphite, the start of most.
const BASE: Skin = Skin {
    top: hex(0x2c2e33),
    bottom: hex(0x1f2125),
    surface: Surface::Grain,
    cheeks: None,
    screws: false,
    stripes: &[],
    ink: hex(0xe6e6e8),
    dim: hex(0x9c9ea4),
    well: hex(0x131417),
    well_ink: hex(0xe6e6e8),
    accent: hex(0xffa733),
    knob: Knob::Cap,
    cap: hex(0xd8d8da),
    logo: Font::Tall,
    logo_size: 20.0,
    logo_ink: hex(0xf2f2f2),
    logo_case: Case::AsIs,
    plate: None,
    groups: Groups::Plain,
    caps: false,
    heroes: &[],
    pads: &[],
};

/// A eurorack module: brushed aluminium, black print, white knobs.
const MODULE: Skin = Skin {
    top: hex(0xd3d6d8),
    bottom: hex(0xb3b7ba),
    surface: Surface::Brushed,
    screws: true,
    ink: hex(0x17191b),
    dim: hex(0x50545a),
    well: hex(0x2a2d31),
    well_ink: hex(0xeeeeee),
    knob: Knob::Euro,
    cap: hex(0xf4f4f2),
    logo: Font::Ui,
    logo_size: 21.0,
    logo_ink: hex(0x17191b),
    logo_case: Case::Lower,
    ..BASE
};

/// A studio rack unit: black brushed metal, screws in its ears.
const RACK: Skin = Skin {
    top: hex(0x26282c),
    bottom: hex(0x17181b),
    surface: Surface::Brushed,
    screws: true,
    knob: Knob::Chrome,
    cap: hex(0xe2e4e6),
    logo: Font::Wide,
    logo_size: 19.0,
    logo_case: Case::Upper,
    caps: true,
    groups: Groups::Outline,
    ..BASE
};

/// Silver plastic of the eighties' boxes.
const SILVER: Skin = Skin {
    top: hex(0xc9cacb),
    bottom: hex(0xa7a8aa),
    surface: Surface::Grain,
    ink: hex(0x121212),
    dim: hex(0x45474a),
    well: hex(0x26272a),
    well_ink: hex(0xeeeeee),
    knob: Knob::Vintage,
    cap: hex(0x1b1b1c),
    logo: Font::Block,
    logo_size: 18.0,
    logo_ink: hex(0x141414),
    caps: true,
    groups: Groups::Outline,
    ..BASE
};

const ANALOG: Skin = Skin {
    top: hex(0x1d1b19),
    bottom: hex(0x11100f),
    surface: Surface::Brushed,
    cheeks: Some(hex(0x6a3a1f)),
    ink: hex(0xeee2c4),
    dim: hex(0xa99d84),
    well: hex(0x0b0a09),
    well_ink: hex(0xeee2c4),
    accent: hex(0xf2a33a),
    knob: Knob::Vintage,
    cap: hex(0x121212),
    logo: Font::Wide,
    logo_size: 19.0,
    logo_ink: hex(0xeee2c4),
    logo_case: Case::Upper,
    groups: Groups::Outline,
    caps: true,
    heroes: &["Cutoff", "Resonance"],
    ..BASE
};

const SYNTH: Skin = Skin {
    top: hex(0x2b3644),
    bottom: hex(0x1b222c),
    ink: hex(0xe0e9f2),
    dim: hex(0x91a2b4),
    well: hex(0x111821),
    accent: hex(0x4fd1ff),
    cap: hex(0xdfe3e8),
    logo: Font::Retro,
    logo_size: 21.0,
    logo_ink: hex(0x8fe3ff),
    groups: Groups::Inset,
    heroes: &["Cutoff", "Resonance"],
    ..BASE
};

const FM: Skin = Skin {
    top: hex(0x3a2e29),
    bottom: hex(0x241c18),
    surface: Surface::Flat,
    ink: hex(0xf0e6dc),
    dim: hex(0xb0a196),
    well: hex(0x17110e),
    accent: hex(0x46c9ae),
    knob: Knob::Cap,
    cap: hex(0x8a2d36),
    logo: Font::Wide,
    logo_size: 19.0,
    logo_ink: hex(0xf0e6dc),
    logo_case: Case::Upper,
    stripes: &[hex(0x8a2d36), hex(0x46c9ae)],
    groups: Groups::Header,
    heroes: &["Brightness", "Cutoff"],
    ..BASE
};

const SAMPLER: Skin = Skin {
    top: hex(0x3b3d42),
    bottom: hex(0x2a2c30),
    accent: hex(0xff4141),
    cap: hex(0x1d1d1f),
    logo: Font::Tall,
    logo_size: 24.0,
    logo_case: Case::Upper,
    groups: Groups::Inset,
    heroes: &["Cutoff", "Resonance"],
    ..BASE
};

/// A street organ's painted front: deep red, gold lines, cream lettering.
const STREET_ORGAN: Skin = Skin {
    top: hex(0x8a1e26),
    bottom: hex(0x561017),
    surface: Surface::Velvet,
    cheeks: Some(hex(0xb48a3c)),
    stripes: &[hex(0xe8c35a), hex(0x2f6b4f)],
    ink: hex(0xf6e7c8),
    dim: hex(0xd9c49a),
    well: hex(0x3a0a0e),
    well_ink: hex(0xf6e7c8),
    accent: hex(0xf2c14e),
    knob: Knob::Cap,
    cap: hex(0xe0b04a),
    logo: Font::Script,
    logo_size: 25.0,
    logo_ink: hex(0xffe08a),
    groups: Groups::Outline,
    ..BASE
};

/// A rack sound module: dark grey with its ears' screws, a green display.
const SF2: Skin = Skin {
    top: hex(0x3a3d43),
    bottom: hex(0x24262a),
    surface: Surface::Grain,
    screws: true,
    stripes: &[hex(0x2fb5c8)],
    accent: hex(0xa6f04d),
    knob: Knob::Cap,
    cap: hex(0x2fb5c8),
    logo: Font::Wide,
    logo_size: 17.0,
    logo_case: Case::Upper,
    groups: Groups::Outline,
    ..BASE
};

/// A vocal booth: deep indigo lacquer, a pink stripe and pink caps, a cyan display.
const SINGER: Skin = Skin {
    top: hex(0x2c2752),
    bottom: hex(0x181530),
    surface: Surface::Gloss,
    stripes: &[hex(0xff7aa8), hex(0x5ad1d6)],
    ink: hex(0xf1eefa),
    dim: hex(0xb3acd0),
    well: hex(0x0f0d1f),
    well_ink: hex(0xf1eefa),
    accent: hex(0x7ff0e6),
    knob: Knob::Cap,
    cap: hex(0xff7aa8),
    logo: Font::Future,
    logo_size: 20.0,
    logo_ink: hex(0xffd3e3),
    groups: Groups::Outline,
    heroes: &["Vibrato"],
    ..BASE
};

/// A red desktop synth: dark red lacquer, black knobs, a pale blue display.
const HYPER: Skin = Skin {
    top: hex(0x8c1219),
    bottom: hex(0x4a080d),
    surface: Surface::Gloss,
    stripes: &[hex(0xd02a33)],
    ink: hex(0xf3e9e9),
    dim: hex(0xd7b9b9),
    well: hex(0x1a0507),
    well_ink: hex(0xf3e9e9),
    accent: hex(0x8fd8ff),
    knob: Knob::Cap,
    cap: hex(0x1b1b1d),
    logo: Font::Future,
    logo_size: 20.0,
    logo_ink: hex(0xffffff),
    logo_case: Case::Upper,
    groups: Groups::Outline,
    heroes: &["Cutoff"],
    ..BASE
};

const PARTICLES: Skin = Skin {
    top: hex(0xc9a56c),
    bottom: hex(0xa4814b),
    surface: Surface::Wood,
    ink: hex(0x2a1a0c),
    dim: hex(0x45331c),
    well: hex(0x3a2614),
    well_ink: hex(0xf3e6cc),
    accent: hex(0xc4471d),
    knob: Knob::Cap,
    cap: hex(0x3a2614),
    logo: Font::Script,
    logo_size: 24.0,
    logo_ink: hex(0x3a2210),
    ..BASE
};

const VOICE: Skin = Skin {
    top: hex(0x3b1e4b),
    bottom: hex(0x1f0d29),
    surface: Surface::Velvet,
    ink: hex(0xf1dcaa),
    dim: hex(0xb69c72),
    well: hex(0x170920),
    well_ink: hex(0xf1dcaa),
    accent: hex(0xf2c35a),
    knob: Knob::Chrome,
    cap: hex(0xe6c068),
    logo: Font::Serif,
    logo_size: 21.0,
    logo_ink: hex(0xf2c35a),
    groups: Groups::Outline,
    heroes: &["Vowel", "Morph"],
    ..BASE
};

const MALLETS: Skin = Skin {
    top: hex(0x5b2f1f),
    bottom: hex(0x3b1b10),
    surface: Surface::Wood,
    ink: hex(0xf4e3c6),
    dim: hex(0xc2a888),
    well: hex(0x200e07),
    well_ink: hex(0xf4e3c6),
    accent: hex(0xffb347),
    cap: hex(0xd9c49a),
    logo: Font::Serif,
    logo_size: 21.0,
    logo_ink: hex(0xffd79a),
    ..BASE
};

const STRINGS: Skin = Skin {
    top: hex(0x9a4a18),
    bottom: hex(0x2e1206),
    surface: Surface::Burst,
    ink: hex(0xf8e8cc),
    dim: hex(0xd8bf9c),
    well: hex(0x1e0b03),
    well_ink: hex(0xf8e8cc),
    accent: hex(0xf3b65a),
    knob: Knob::Chrome,
    cap: hex(0xe8c27a),
    logo: Font::Script,
    logo_size: 25.0,
    logo_ink: hex(0xfbe3b0),
    ..BASE
};

const WINDS: Skin = Skin {
    top: hex(0xddb95e),
    bottom: hex(0xa47e2c),
    surface: Surface::Brushed,
    ink: hex(0x2a1c04),
    dim: hex(0x48360c),
    well: hex(0x3a2a08),
    well_ink: hex(0xf6e2a8),
    accent: hex(0x8a2a10),
    knob: Knob::Chrome,
    cap: hex(0xf4dc94),
    logo: Font::Serif,
    logo_size: 21.0,
    logo_ink: hex(0x3a2606),
    ..BASE
};

const ORGAN: Skin = Skin {
    top: hex(0x2c1c13),
    bottom: hex(0x1a0f09),
    surface: Surface::Wood,
    cheeks: Some(hex(0x7a4423)),
    ink: hex(0xf2e6cc),
    dim: hex(0xbfae90),
    well: hex(0x0f0805),
    well_ink: hex(0xf2e6cc),
    accent: hex(0xe0553a),
    knob: Knob::Chicken,
    cap: hex(0x161210),
    logo: Font::Script,
    logo_size: 25.0,
    logo_ink: hex(0xf5e2b6),
    ..BASE
};

const MONK: Skin = Skin {
    top: hex(0x4b4640),
    bottom: hex(0x2f2b27),
    ink: hex(0xece4d2),
    dim: hex(0xb2a894),
    well: hex(0x1c1916),
    accent: hex(0xe0aa45),
    knob: Knob::Vintage,
    cap: hex(0x1d1a17),
    logo: Font::Serif,
    logo_size: 21.0,
    logo_ink: hex(0xe8c27a),
    groups: Groups::Outline,
    heroes: &["Vowel", "Head"],
    ..BASE
};

const PIANO: Skin = Skin {
    top: hex(0x18181b),
    bottom: hex(0x050506),
    surface: Surface::Gloss,
    ink: hex(0xebe5d8),
    dim: hex(0x9e978b),
    well: hex(0x000000),
    well_ink: hex(0xebe5d8),
    accent: hex(0xdcb86c),
    knob: Knob::Chrome,
    cap: hex(0xe9d9b4),
    logo: Font::Serif,
    logo_size: 21.0,
    logo_ink: hex(0xdcb86c),
    ..BASE
};

const BASSLINE: Skin = Skin { accent: hex(0xe8541c), knob: Knob::Euro, cap: hex(0x1c1c1d), heroes: &["Cutoff", "Resonance"], surface: Surface::Brushed, ..SILVER };

const MONO: Skin = Skin {
    top: hex(0x2d2f33),
    bottom: hex(0x1d1e21),
    accent: hex(0xff5a43),
    stripes: &[hex(0xff5a43), hex(0xf6a623), hex(0x4da3ff)],
    cap: hex(0x1a1a1c),
    knob: Knob::Cap,
    logo: Font::Wide,
    logo_size: 19.0,
    logo_case: Case::Upper,
    caps: true,
    heroes: &["Freq", "Res"],
    ..BASE
};

const DRONE: Skin = Skin {
    top: hex(0x241d4d),
    bottom: hex(0x0b0920),
    surface: Surface::Velvet,
    ink: hex(0xdad2ff),
    dim: hex(0x9a90c8),
    well: hex(0x0a0818),
    well_ink: hex(0xdad2ff),
    accent: hex(0xa585ff),
    knob: Knob::Neon,
    logo: Font::Neon,
    logo_size: 22.0,
    logo_ink: hex(0xc8b6ff),
    ..BASE
};

const WAVES: Skin = Skin {
    top: hex(0x0f2b34),
    bottom: hex(0x07151a),
    surface: Surface::Scan,
    ink: hex(0xccfbff),
    dim: hex(0x7fb6bd),
    well: hex(0x041013),
    well_ink: hex(0xccfbff),
    accent: hex(0x2fe6d6),
    knob: Knob::Neon,
    logo: Font::Future,
    logo_size: 20.0,
    logo_ink: hex(0x2fe6d6),
    heroes: &["Position 1", "Position 2", "Cutoff"],
    ..BASE
};

const GUITAR: Skin = Skin {
    top: hex(0xe08a2e),
    bottom: hex(0x2c0d03),
    surface: Surface::Burst,
    ink: hex(0xfff3de),
    dim: hex(0xf0d2a8),
    well: hex(0x1e0802),
    well_ink: hex(0xfff3de),
    accent: hex(0xffd27a),
    knob: Knob::Cap,
    cap: hex(0xf2e8cf),
    logo: Font::Script,
    logo_size: 25.0,
    logo_ink: hex(0xfff3de),
    ..BASE
};

const BASS_GUITAR: Skin = Skin {
    top: hex(0x231e1b),
    bottom: hex(0x0f0d0c),
    ink: hex(0xf2e2ca),
    dim: hex(0xb4a28a),
    well: hex(0x070605),
    well_ink: hex(0xf2e2ca),
    accent: hex(0xff9b3d),
    knob: Knob::Chrome,
    cap: hex(0xdfe1e3),
    logo: Font::Script,
    logo_size: 25.0,
    logo_ink: hex(0xf2e2ca),
    ..BASE
};

const WIND_CHIMES: Skin = Skin {
    top: hex(0x4782ba),
    bottom: hex(0x1d3a5f),
    surface: Surface::Flat,
    ink: hex(0xf4fbff),
    dim: hex(0xbcd4ea),
    well: hex(0x112640),
    well_ink: hex(0xf4fbff),
    accent: hex(0xffe08a),
    knob: Knob::Chrome,
    cap: hex(0xeef3f7),
    logo: Font::Script,
    logo_size: 25.0,
    logo_ink: hex(0xffffff),
    ..BASE
};

const BIRDS: Skin = Skin {
    top: hex(0x325d36),
    bottom: hex(0x15301b),
    ink: hex(0xebf6dd),
    dim: hex(0xaecb9c),
    well: hex(0x0b1d0f),
    well_ink: hex(0xebf6dd),
    accent: hex(0xffc94a),
    cap: hex(0xe2ecc8),
    logo: Font::Script,
    logo_size: 25.0,
    logo_ink: hex(0xffe9a8),
    ..BASE
};

const NATURE: Skin = Skin {
    top: hex(0x434e2c),
    bottom: hex(0x222a18),
    ink: hex(0xeff0d9),
    dim: hex(0xb3b59a),
    well: hex(0x13180c),
    well_ink: hex(0xeff0d9),
    accent: hex(0x9ad65e),
    cap: hex(0x7a5a34),
    logo: Font::Script,
    logo_size: 25.0,
    logo_ink: hex(0xdff2b8),
    ..BASE
};

const FACET: Skin = Skin {
    top: hex(0xf3f0eb),
    bottom: hex(0xdfdad1),
    surface: Surface::Flat,
    ink: hex(0x24232b),
    dim: hex(0x6b6970),
    well: hex(0x2c2b33),
    well_ink: hex(0xf3f0eb),
    accent: hex(0xff5a5f),
    knob: Knob::Led,
    cap: hex(0xfdfdfd),
    logo: Font::Future,
    logo_size: 19.0,
    logo_ink: hex(0x24232b),
    logo_case: Case::Lower,
    ..BASE
};

/// Coast: a pale blue-grey painted panel between walnut cheeks, dark ink,
/// orange caps, the groups outlined.
const COAST: Skin = Skin {
    top: hex(0xd7e3e6),
    bottom: hex(0xbfd0d4),
    surface: Surface::Flat,
    cheeks: Some(hex(0x6b4426)),
    screws: true,
    ink: hex(0x1d2a33),
    dim: hex(0x51616b),
    well: hex(0x1d2a33),
    well_ink: hex(0xe8f0f2),
    accent: hex(0xe8642c),
    knob: Knob::Cap,
    cap: hex(0x23292e),
    logo: Font::Future,
    logo_size: 20.0,
    logo_ink: hex(0x1d2a33),
    logo_case: Case::Lower,
    groups: Groups::Outline,
    caps: true,
    heroes: &["Timbre", "Response"],
    ..BASE
};

/// Crimson: deep red textured plastic, light ink, amber displays, dark caps,
/// each group under a bar.
const CRIMSON: Skin = Skin {
    top: hex(0x9e1622),
    bottom: hex(0x5a0912),
    surface: Surface::Grain,
    ink: hex(0xf6ecec),
    dim: hex(0xdcbcbc),
    well: hex(0x170406),
    well_ink: hex(0xffd9a0),
    accent: hex(0xffaa3c),
    knob: Knob::Cap,
    cap: hex(0x26262a),
    logo: Font::Wide,
    logo_size: 19.0,
    logo_ink: hex(0xffffff),
    logo_case: Case::Upper,
    groups: Groups::Header,
    heroes: &["Cutoff"],
    ..BASE
};

/// Silicon: dark anthracite as the big desktop virtual analogs, light print,
/// red for what is lit, a blue backlit display, black caps under bars.
const SILICON: Skin = Skin {
    top: hex(0x2c2d31),
    bottom: hex(0x17181b),
    surface: Surface::Grain,
    ink: hex(0xe9e9ec),
    dim: hex(0xa2a4ab),
    well: hex(0x0a1824),
    well_ink: hex(0x9fdcff),
    accent: hex(0xe5372b),
    knob: Knob::Cap,
    cap: hex(0x0e0e10),
    logo: Font::Wide,
    logo_size: 19.0,
    logo_ink: hex(0xffffff),
    logo_case: Case::Upper,
    groups: Groups::Header,
    heroes: &["Cutoff"],
    ..BASE
};

const PAN_FLUTE: Skin = Skin {
    top: hex(0xbd9d59),
    bottom: hex(0x8a6e35),
    surface: Surface::Wood,
    ink: hex(0x241a08),
    dim: hex(0x3e3010),
    well: hex(0x33270e),
    well_ink: hex(0xf3e6c4),
    accent: hex(0x2e7a3a),
    cap: hex(0x3a2a12),
    logo: Font::Script,
    logo_size: 25.0,
    logo_ink: hex(0x2c1e08),
    ..BASE
};

const MELODICA: Skin = Skin {
    top: hex(0xcc2a22),
    bottom: hex(0x8a130e),
    ink: hex(0xfff3ea),
    dim: hex(0xf3c2ba),
    well: hex(0x4a0905),
    well_ink: hex(0xfff3ea),
    accent: hex(0xffe066),
    cap: hex(0xf7f7f5),
    logo: Font::Script,
    logo_size: 25.0,
    logo_ink: hex(0xffffff),
    ..BASE
};

const HARMONICA: Skin = Skin {
    top: hex(0xdadde1),
    bottom: hex(0x9da2a8),
    surface: Surface::Brushed,
    ink: hex(0x14171b),
    dim: hex(0x474c53),
    well: hex(0x23272d),
    well_ink: hex(0xeef0f2),
    accent: hex(0x1f5fc4),
    knob: Knob::Chrome,
    cap: hex(0xf2f4f6),
    logo: Font::Script,
    logo_size: 25.0,
    logo_ink: hex(0x1b3a86),
    ..BASE
};

const CALCULATOR: Skin = Skin {
    top: hex(0x1d1e20),
    bottom: hex(0x101112),
    ink: hex(0xdadada),
    dim: hex(0x8e9094),
    well: hex(0x050505),
    accent: hex(0xff8a1f),
    knob: Knob::Pixel,
    cap: hex(0x3a3b3e),
    logo: Font::Lcd,
    logo_size: 20.0,
    logo_ink: hex(0x1d2616),
    plate: Some(hex(0x9fae86)),
    stripes: &[hex(0xff8a1f), hex(0x3db0e8)],
    ..BASE
};

const CONSOLE: Skin = Skin {
    top: hex(0xcbcbc8),
    bottom: hex(0xa8a8a5),
    ink: hex(0x1a1a1a),
    dim: hex(0x4c4c4c),
    well: hex(0x222222),
    well_ink: hex(0xeeeeee),
    accent: hex(0xd8262c),
    knob: Knob::Cap,
    cap: hex(0xc8232a),
    logo: Font::Pixel,
    logo_size: 19.0,
    logo_ink: hex(0xe0282f),
    logo_case: Case::Upper,
    plate: Some(hex(0x1b1b1b)),
    stripes: &[hex(0x1b1b1b)],
    caps: true,
    ..BASE
};

const HANDHELD: Skin = Skin {
    top: hex(0xcbc7bf),
    bottom: hex(0xb1aca2),
    ink: hex(0x2b2a6a),
    dim: hex(0x5a586f),
    well: hex(0x3a3a46),
    well_ink: hex(0xe8e6f0),
    accent: hex(0x9c2a6d),
    knob: Knob::Cap,
    cap: hex(0xa3306f),
    logo: Font::Pixel,
    logo_size: 18.0,
    logo_ink: hex(0x2b2a6a),
    logo_case: Case::Upper,
    stripes: &[hex(0x8b1e4b), hex(0x2b2a6a)],
    caps: true,
    ..BASE
};

const MICRO: Skin = Skin {
    top: hex(0x8f7d5e),
    bottom: hex(0x6b5b42),
    ink: hex(0xf6eedb),
    dim: hex(0xd8cbb0),
    well: hex(0x40318d),
    well_ink: hex(0xa59cff),
    accent: hex(0xa59cff),
    knob: Knob::Pixel,
    cap: hex(0x3d3024),
    logo: Font::Pixel,
    logo_size: 18.0,
    logo_ink: hex(0xa59cff),
    logo_case: Case::Upper,
    plate: Some(hex(0x40318d)),
    stripes: &[hex(0xd1342f), hex(0xf28a2e), hex(0xf7d23a), hex(0x3fa34d), hex(0x3c5bd8)],
    caps: true,
    heroes: &["Cutoff", "Resonance"],
    ..BASE
};

const WARP: Skin = Skin {
    top: hex(0x2b2c30),
    bottom: hex(0x18191c),
    surface: Surface::Carbon,
    accent: hex(0x37c9b8),
    knob: Knob::Led,
    cap: hex(0x3a3c40),
    logo: Font::Wide,
    logo_size: 19.0,
    logo_case: Case::Upper,
    logo_ink: hex(0xe6e6e6),
    stripes: &[hex(0x37c9b8), hex(0xd2508f)],
    ..BASE
};

const HYBRID: Skin = Skin {
    top: hex(0x2c323b),
    bottom: hex(0x1a1e24),
    surface: Surface::Brushed,
    accent: hex(0xff7a2f),
    knob: Knob::Chrome,
    cap: hex(0xdfe2e6),
    logo: Font::Sport,
    logo_size: 20.0,
    logo_ink: hex(0xff9a5c),
    logo_case: Case::Upper,
    groups: Groups::Inset,
    heroes: &["Cutoff 1", "Reso 1"],
    ..BASE
};

const CHIP_FM: Skin = Skin {
    top: hex(0x1a1a1b),
    bottom: hex(0x0a0a0a),
    surface: Surface::Carbon,
    ink: hex(0xe8e8e8),
    dim: hex(0x9a9a9a),
    well: hex(0x000000),
    accent: hex(0xe63a2e),
    knob: Knob::Cap,
    cap: hex(0x2c2c2e),
    logo: Font::Pixel,
    logo_size: 18.0,
    logo_ink: hex(0xeac43c),
    logo_case: Case::Upper,
    stripes: &[hex(0xe63a2e)],
    ..BASE
};

const V2: Skin = Skin {
    top: hex(0x0c0e19),
    bottom: hex(0x05060b),
    surface: Surface::Scan,
    ink: hex(0xd6e4ff),
    dim: hex(0x8090b4),
    well: hex(0x020308),
    well_ink: hex(0xd6e4ff),
    accent: hex(0x39f3ff),
    knob: Knob::Neon,
    logo: Font::Space,
    logo_size: 22.0,
    logo_ink: hex(0x39f3ff),
    stripes: &[hex(0xff2bd6), hex(0x39f3ff)],
    ..BASE
};

const OIDOS: Skin = Skin {
    top: hex(0x111a38),
    bottom: hex(0x02030a),
    surface: Surface::Velvet,
    ink: hex(0xd8e2ff),
    dim: hex(0x8a98c4),
    well: hex(0x02030a),
    well_ink: hex(0xd8e2ff),
    accent: hex(0x8ab4ff),
    knob: Knob::Neon,
    logo: Font::Space,
    logo_size: 22.0,
    logo_ink: hex(0xbcd0ff),
    heroes: &["Harmonicity", "Sharpness"],
    ..BASE
};

const QUARTET: Skin = Skin {
    top: hex(0xf1efe9),
    bottom: hex(0xdedad2),
    surface: Surface::Flat,
    ink: hex(0x2a2a30),
    dim: hex(0x6c6b70),
    well: hex(0x2e2e35),
    well_ink: hex(0xf1efe9),
    accent: hex(0xe8553a),
    knob: Knob::Cap,
    cap: hex(0xfafafa),
    logo: Font::Sport,
    logo_size: 20.0,
    logo_ink: hex(0x2a2a30),
    logo_case: Case::Upper,
    stripes: &[hex(0xe8553a), hex(0xf2a03d), hex(0x3d7fe8)],
    ..BASE
};

const ORCHESTRA: Skin = Skin {
    top: hex(0x5e1019),
    bottom: hex(0x2a050a),
    surface: Surface::Velvet,
    ink: hex(0xf4e0b2),
    dim: hex(0xc4a87c),
    well: hex(0x1c0306),
    well_ink: hex(0xf4e0b2),
    accent: hex(0xe8c068),
    knob: Knob::Chrome,
    cap: hex(0xecc878),
    logo: Font::Serif,
    logo_size: 21.0,
    logo_ink: hex(0xe8c068),
    groups: Groups::Outline,
    heroes: &["Dynamics"],
    ..BASE
};

const MODULAR: Skin = Skin {
    top: hex(0x1d1d20),
    bottom: hex(0x121214),
    surface: Surface::Brushed,
    screws: true,
    accent: hex(0xff6a3d),
    knob: Knob::Euro,
    cap: hex(0x202022),
    logo: Font::Wide,
    logo_size: 19.0,
    logo_case: Case::Upper,
    ..BASE
};

// Drum machines.

const M808: Skin = Skin {
    top: hex(0x3a3632),
    bottom: hex(0x23201d),
    stripes: &[hex(0xe8471f), hex(0xf28c28), hex(0xf6c445), hex(0xf1e8cf)],
    ink: hex(0xf1e8cf),
    dim: hex(0xb4ab94),
    accent: hex(0xf28c28),
    cap: hex(0xe6dfcc),
    logo: Font::Block,
    logo_size: 20.0,
    logo_ink: hex(0xf1e8cf),
    caps: true,
    pads: &[hex(0xe8471f), hex(0xf28c28), hex(0xf6c445), hex(0xf1e8cf)],
    ..BASE
};

const M909: Skin = Skin { accent: hex(0xf26b1d), stripes: &[hex(0xf26b1d)], knob: Knob::Euro, cap: hex(0x222224), pads: &[hex(0xe4e4e1)], ..SILVER };
const M606: Skin = Skin { accent: hex(0xd83a2a), pads: &[hex(0x2a2a2c)], knob: Knob::Euro, cap: hex(0x1a1a1a), surface: Surface::Brushed, logo: Font::Wide, logo_size: 19.0, ..SILVER };

const CR78: Skin = Skin {
    top: hex(0xdccfb1),
    bottom: hex(0xc0b391),
    cheeks: Some(hex(0x6b3f22)),
    ink: hex(0x2a2016),
    dim: hex(0x483a2a),
    well: hex(0x3a2c1e),
    well_ink: hex(0xf3ead6),
    accent: hex(0xc8541f),
    knob: Knob::Chrome,
    cap: hex(0xe6e8ea),
    logo: Font::Retro,
    logo_size: 21.0,
    logo_ink: hex(0x3a2a1a),
    pads: &[hex(0x5a3a24), hex(0x5a3a24), hex(0xe2d6b8), hex(0xe2d6b8)],
    ..BASE
};

const ACOUSTIC: Skin = Skin {
    top: hex(0x6e3c20),
    bottom: hex(0x3e1f0e),
    surface: Surface::Wood,
    ink: hex(0xf6e6cc),
    dim: hex(0xc8ae8c),
    well: hex(0x1e0e05),
    well_ink: hex(0xf6e6cc),
    accent: hex(0xffc56a),
    knob: Knob::Chrome,
    cap: hex(0xe8eaec),
    logo: Font::Script,
    logo_size: 25.0,
    logo_ink: hex(0xffe3b0),
    ..BASE
};

const RETRO: Skin = Skin {
    top: hex(0x27222f),
    bottom: hex(0x15121b),
    surface: Surface::Scan,
    stripes: &[hex(0xff5f5f), hex(0xffb84d), hex(0xffe14d), hex(0x5fd17a), hex(0x4da3ff)],
    accent: hex(0xff5fa2),
    cap: hex(0xff8fbf),
    logo: Font::Retro,
    logo_size: 21.0,
    logo_ink: hex(0xffd36b),
    pads: &[hex(0xff5f5f), hex(0xffb84d), hex(0x5fd17a), hex(0x4da3ff)],
    ..BASE
};

const CHIP_DRUMS: Skin = Skin {
    top: hex(0x231b3b),
    bottom: hex(0x110d21),
    surface: Surface::Scan,
    accent: hex(0x7cff6b),
    knob: Knob::Pixel,
    cap: hex(0x3c3360),
    logo: Font::Pixel,
    logo_size: 18.0,
    logo_ink: hex(0x7cff6b),
    logo_case: Case::Upper,
    ..BASE
};

const DRUM_RACK: Skin = Skin { accent: hex(0xffa733), knob: Knob::Led, cap: hex(0x45484e), logo_size: 22.0, logo_case: Case::Upper, ..BASE };

// Effects.

const REVERB: Skin = Skin {
    top: hex(0x1a2c4d),
    bottom: hex(0x0b1528),
    surface: Surface::Velvet,
    ink: hex(0xdce9ff),
    dim: hex(0x93a7c8),
    well: hex(0x060d1a),
    well_ink: hex(0xdce9ff),
    accent: hex(0x86b8ff),
    cap: hex(0xd4dcea),
    logo: Font::Sport,
    logo_size: 18.0,
    logo_ink: hex(0xbcd6ff),
    heroes: &["Decay", "Size"],
    ..BASE
};

const DELAY: Skin = Skin {
    top: hex(0x2f4a3a),
    bottom: hex(0x1a2c22),
    ink: hex(0xf1ebd6),
    dim: hex(0xb4b29a),
    well: hex(0x0e1a13),
    well_ink: hex(0xf1ebd6),
    accent: hex(0xf2d16b),
    knob: Knob::Chicken,
    cap: hex(0x141414),
    logo: Font::Retro,
    logo_size: 20.0,
    logo_ink: hex(0xf2d16b),
    heroes: &["Time", "Feedback"],
    ..BASE
};

const MODULATION: Skin = Skin {
    top: hex(0x3c2558),
    bottom: hex(0x211436),
    ink: hex(0xf1e7ff),
    dim: hex(0xb8a6d4),
    well: hex(0x150b24),
    well_ink: hex(0xf1e7ff),
    accent: hex(0xcaa0ff),
    cap: hex(0xece4f8),
    logo: Font::Retro,
    logo_size: 20.0,
    logo_ink: hex(0xdcc4ff),
    heroes: &["Rate", "Depth"],
    ..BASE
};

const DISTORTION: Skin = Skin {
    top: hex(0x4c1515),
    bottom: hex(0x260808),
    surface: Surface::Carbon,
    ink: hex(0xffe8e2),
    dim: hex(0xd4a49a),
    well: hex(0x170303),
    well_ink: hex(0xffe8e2),
    accent: hex(0xff5f3d),
    knob: Knob::Chrome,
    cap: hex(0xe2e4e6),
    logo: Font::Block,
    logo_size: 18.0,
    logo_ink: hex(0xff8a6b),
    heroes: &["Drive"],
    ..BASE
};

const DYNAMICS: Skin = Skin { accent: hex(0x7fe0a0), heroes: &["Threshold", "Ratio"], ..RACK };
const EQ: Skin = Skin { top: hex(0x50687c), bottom: hex(0x364a5b), surface: Surface::Grain, ink: hex(0xf2f4f6), dim: hex(0xc2ccd6), well: hex(0x1d2833), accent: hex(0xffd36b), knob: Knob::Chicken, cap: hex(0x151515), heroes: &["Cutoff", "Resonance"], ..RACK };

const PEDALS: Skin = Skin {
    top: hex(0x2a2d31),
    bottom: hex(0x1b1d20),
    screws: true,
    accent: hex(0xffb347),
    knob: Knob::Chrome,
    logo: Font::Sport,
    logo_size: 18.0,
    logo_case: Case::Upper,
    ..BASE
};

const AIRWINDOWS: Skin = Skin {
    top: hex(0xebe9e3),
    bottom: hex(0xd8d5cd),
    surface: Surface::Flat,
    ink: hex(0x26262a),
    dim: hex(0x67676c),
    well: hex(0x2b2b30),
    well_ink: hex(0xebe9e3),
    accent: hex(0x2b6cb0),
    knob: Knob::Euro,
    cap: hex(0x2b2b2e),
    logo: Font::Ui,
    logo_size: 19.0,
    logo_ink: hex(0x26262a),
    ..BASE
};

const UTILITY: Skin = Skin { accent: hex(0x9fd8ff), knob: Knob::Ring, logo_case: Case::Upper, ..BASE };

const VA_FILTER: Skin = Skin { heroes: &["Cutoff", "Resonance"], ..ANALOG };

/// A sequencer's or a MIDI effect's look.
const SEQUENCER: Skin = Skin { accent: hex(0xff4f4f), knob: Knob::Led, cap: hex(0x3a3c41), logo_case: Case::Upper, ..BASE };
const MIDI_FX: Skin = Skin { accent: hex(0x8fe388), knob: Knob::Led, cap: hex(0x3a3c41), logo_case: Case::Upper, ..BASE };

/// The look of an instrument, by its name (as its title shows it).
pub fn for_instrument(name: &str) -> Option<Skin> {
    Some(match name {
        "Analog" => ANALOG,
        "Synth" => SYNTH,
        "FM" => FM,
        "Sampler" => SAMPLER,
        "SF2 Player" => SF2,
        "Singer" => SINGER,
        "Hyper" => HYPER,
        "Street Organ" => STREET_ORGAN,
        "Plaits" => Skin { accent: hex(0x2fa84f), heroes: &["Harmonics", "Timbre", "Morph"], ..MODULE },
        "Braids" => Skin { accent: hex(0xe04534), heroes: &["Timbre", "Color"], ..MODULE },
        "Elements" => Skin { accent: hex(0x5a6df0), heroes: &["Geometry", "Brightness"], ..MODULE },
        "Rings" => Skin { accent: hex(0xd49a1e), heroes: &["Structure", "Brightness"], ..MODULE },
        "Beads" => Skin { accent: hex(0xc83cd0), heroes: &["Size", "Density"], ..MODULE },
        "Particles" | "Shakers" => PARTICLES,
        "Modular" => MODULAR,
        "Voice" => VOICE,
        "Mallets" => MALLETS,
        "Strings" => STRINGS,
        "Winds" => WINDS,
        "Organ" => ORGAN,
        "Monk" => MONK,
        "Grand Piano" => PIANO,
        "Bassline" => BASSLINE,
        "Mono" => MONO,
        "Drone" => DRONE,
        "Waves" => WAVES,
        "Guitar" => GUITAR,
        "Bass Guitar" => BASS_GUITAR,
        "Wind Chimes" => WIND_CHIMES,
        "Birds" => BIRDS,
        "Nature" => NATURE,
        "Facet" => FACET,
        "Coast" => COAST,
        "Crimson" => CRIMSON,
        "Silicon" => SILICON,
        "Pan Flute" => PAN_FLUTE,
        "Melodica" => MELODICA,
        "Harmonica" => HARMONICA,
        "Calculator" => CALCULATOR,
        "Console" => CONSOLE,
        "Handheld" => HANDHELD,
        "Micro" => MICRO,
        "Warp" => WARP,
        "Hybrid" => HYBRID,
        "Chip FM" => CHIP_FM,
        "V2" => V2,
        "Oidos" => OIDOS,
        "Quartet" => QUARTET,
        "Orchestra" | "Soloists" => ORCHESTRA,
        // Drum machines.
        "808" => M808,
        "909" => M909,
        "606" => M606,
        "CR-78" => CR78,
        "Mutable Drums" => Skin { accent: hex(0x2fa84f), ..MODULE },
        "Resonant Drums" => Skin { top: hex(0x1c1c1f), bottom: hex(0x111113), ink: hex(0xe6e6e8), dim: hex(0x9c9ea4), well: hex(0x08080a), logo_ink: hex(0xe6e6e8), accent: hex(0x3ec6ff), cap: hex(0x2a2a2d), ..MODULE },
        "FM Drums" => Skin { top: hex(0x1e2e33), bottom: hex(0x111b1e), stripes: &[], ..FM },
        "Acoustic Drums" => ACOUSTIC,
        "Real Drums" => Skin { top: hex(0xa8743c), bottom: hex(0x6e451c), ..ACOUSTIC },
        "Retro Machines" => RETRO,
        "Macro Drums" => Skin { top: hex(0x1f3a2e), bottom: hex(0x12241c), accent: hex(0x5fe3a1), knob: Knob::Led, cap: hex(0x2c4a3c), logo: Font::Sport, ..BASE },
        "Chip Drums" => CHIP_DRUMS,
        "Sampled Machines" => SAMPLER,
        "Drum Rack" => DRUM_RACK,
        _ => return None,
    })
}

/// The look of an audio effect.
pub fn for_effect(kind: EffectKind) -> Option<Skin> {
    use EffectKind::*;
    Some(match kind {
        Plugin | Scope => return None,
        VaFilter => VA_FILTER,
        Warps => Skin { accent: hex(0x2fa84f), heroes: &["Timbre"], ..MODULE },
        Clouds => Skin { accent: hex(0x3d8be0), heroes: &["Position", "Size"], ..MODULE },
        Beads => Skin { accent: hex(0xc83cd0), ..MODULE },
        Rings => Skin { accent: hex(0xd49a1e), heroes: &["Structure"], ..MODULE },
        _ => match kind.category() {
            "Reverb & Resonance" => REVERB,
            "Delay & Loop" => DELAY,
            "Modulation" => MODULATION,
            "Distortion" => DISTORTION,
            "Dynamics" => DYNAMICS,
            "EQ & Filters" => EQ,
            "Pedals" => PEDALS,
            "Airwindows" => AIRWINDOWS,
            _ => UTILITY,
        },
    })
}

/// The look of a MIDI effect or sequencer.
pub fn for_midi(kind: MidiFxKind) -> Option<Skin> {
    Some(if kind.is_sequencer() { SEQUENCER } else { MIDI_FX })
}

#[cfg(test)]
mod tests {
    use super::*;
    use krabton_engine::dsp::drums::DrumEngine;

    fn lum(c: Color32) -> f32 {
        0.299 * c.r() as f32 + 0.587 * c.g() as f32 + 0.114 * c.b() as f32
    }

    /// Every look there is, by what wears it.
    fn every_skin() -> Vec<(String, Skin)> {
        let mut all: Vec<(String, Skin)> = crate::ui::browser::device_names().iter().filter_map(|n| for_instrument(n).map(|s| (n.to_string(), s))).collect();
        all.extend(DrumEngine::ALL.iter().filter_map(|e| for_instrument(e.name()).map(|s| (e.name().to_string(), s))));
        all.extend(EffectKind::ALL.iter().filter_map(|k| for_effect(*k).map(|s| (k.name().to_string(), s))));
        all.extend(MidiFxKind::ALL.iter().filter_map(|k| for_midi(*k).map(|s| (k.name().to_string(), s))));
        all
    }

    #[test]
    fn every_instrument_and_drum_machine_has_its_own_look() {
        for name in crate::ui::browser::device_names() {
            assert!(for_instrument(name).is_some(), "{name} has no look");
        }
        for e in DrumEngine::ALL {
            assert!(for_instrument(e.name()).is_some(), "{} has no look", e.name());
        }
        assert!(for_instrument("CLAP Plugin").is_none());
        assert!(for_effect(EffectKind::Plugin).is_none());
    }

    #[test]
    fn the_print_reads_on_every_panel() {
        for (name, s) in every_skin() {
            let panel = lum(crate::theme::mix(s.top, s.bottom, 0.5));
            assert!((lum(s.ink) - panel).abs() > 100.0, "{name}: labels on the panel");
            assert!((lum(s.dim) - panel).abs() > 55.0, "{name}: small print on the panel");
            assert!((lum(s.well_ink) - lum(s.well)).abs() > 100.0, "{name}: print on the insets");
            if let Some(plate) = s.plate {
                assert!((lum(s.logo_ink) - lum(plate)).abs() > 60.0, "{name}: logo on its plate");
            }
        }
    }
}

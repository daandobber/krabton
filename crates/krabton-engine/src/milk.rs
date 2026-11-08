//! Milk: a feedback visualiser in the manner of the classic music
//! visualisers. Every frame the last one is pulled through a field of
//! motion (closer in or further, turned, warped, rippled, moved,
//! stretched) and fades a little; over it go the sound's wave, a shape
//! (which can hold the last frame in it: pictures inside pictures),
//! borders and motion dots; then an echo, gamma, solarize, invert and a
//! hue turn. Some fifty values make a picture; a preset sets them, each
//! moving with the bass, the mids, the top, the beat or a slow swing, and
//! one preset melts into the next. What is saved with a Look: the preset,
//! how it changes to the next, and the hand's tweaks on top of it.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A value of the picture (by its place: `MilkParam::index`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum MilkParam {
    Zoom,
    ZoomRad,
    Rot,
    RotRad,
    Warp,
    WarpSpeed,
    WarpScale,
    Ripple,
    RippleFreq,
    Cx,
    Cy,
    Dx,
    Dy,
    Sx,
    Sy,
    Decay,
    WaveMode,
    WaveScale,
    WaveAlpha,
    WaveHue,
    WaveSat,
    WaveX,
    WaveY,
    WaveThick,
    ShapeSides,
    ShapeRad,
    ShapeAng,
    ShapeSpin,
    ShapeX,
    ShapeY,
    ShapeAlpha,
    ShapeTex,
    ShapeHue,
    OuterBorder,
    InnerBorder,
    BorderAlpha,
    BorderHue,
    MvX,
    MvY,
    MvAlpha,
    DarkenCenter,
    Gamma,
    EchoZoom,
    EchoAlpha,
    EchoOrient,
    Brighten,
    Darken,
    Solarize,
    Invert,
    Hue,
}

pub const MILK_N: usize = 50;

/// What a value is: its range, where it rests, whether it is one of a few (a mode, a count, on or off), what it is called.
#[derive(Clone, Copy, Debug)]
pub struct MilkDef {
    pub name: &'static str,
    pub min: f32,
    pub max: f32,
    pub default: f32,
    pub whole: bool,
}

const fn d(name: &'static str, min: f32, max: f32, default: f32) -> MilkDef {
    MilkDef { name, min, max, default, whole: false }
}

const fn w(name: &'static str, min: f32, max: f32, default: f32) -> MilkDef {
    MilkDef { name, min, max, default, whole: true }
}

impl MilkParam {
    pub const ALL: [MilkParam; MILK_N] = [
        MilkParam::Zoom,
        MilkParam::ZoomRad,
        MilkParam::Rot,
        MilkParam::RotRad,
        MilkParam::Warp,
        MilkParam::WarpSpeed,
        MilkParam::WarpScale,
        MilkParam::Ripple,
        MilkParam::RippleFreq,
        MilkParam::Cx,
        MilkParam::Cy,
        MilkParam::Dx,
        MilkParam::Dy,
        MilkParam::Sx,
        MilkParam::Sy,
        MilkParam::Decay,
        MilkParam::WaveMode,
        MilkParam::WaveScale,
        MilkParam::WaveAlpha,
        MilkParam::WaveHue,
        MilkParam::WaveSat,
        MilkParam::WaveX,
        MilkParam::WaveY,
        MilkParam::WaveThick,
        MilkParam::ShapeSides,
        MilkParam::ShapeRad,
        MilkParam::ShapeAng,
        MilkParam::ShapeSpin,
        MilkParam::ShapeX,
        MilkParam::ShapeY,
        MilkParam::ShapeAlpha,
        MilkParam::ShapeTex,
        MilkParam::ShapeHue,
        MilkParam::OuterBorder,
        MilkParam::InnerBorder,
        MilkParam::BorderAlpha,
        MilkParam::BorderHue,
        MilkParam::MvX,
        MilkParam::MvY,
        MilkParam::MvAlpha,
        MilkParam::DarkenCenter,
        MilkParam::Gamma,
        MilkParam::EchoZoom,
        MilkParam::EchoAlpha,
        MilkParam::EchoOrient,
        MilkParam::Brighten,
        MilkParam::Darken,
        MilkParam::Solarize,
        MilkParam::Invert,
        MilkParam::Hue,
    ];

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn def(self) -> MilkDef {
        use MilkParam as M;
        match self {
            M::Zoom => d("Zoom", 0.85, 1.15, 1.0),
            M::ZoomRad => d("Zoom Edge", -0.2, 0.2, 0.0),
            M::Rot => d("Rotate", -0.12, 0.12, 0.0),
            M::RotRad => d("Rotate Edge", -0.12, 0.12, 0.0),
            M::Warp => d("Warp", 0.0, 2.5, 0.0),
            M::WarpSpeed => d("Warp Speed", 0.0, 4.0, 1.0),
            M::WarpScale => d("Warp Scale", 0.2, 4.0, 1.0),
            M::Ripple => d("Ripple", 0.0, 1.5, 0.0),
            M::RippleFreq => d("Ripple Rings", 0.2, 4.0, 1.0),
            M::Cx => d("Centre X", 0.0, 1.0, 0.5),
            M::Cy => d("Centre Y", 0.0, 1.0, 0.5),
            M::Dx => d("Move X", -0.03, 0.03, 0.0),
            M::Dy => d("Move Y", -0.03, 0.03, 0.0),
            M::Sx => d("Stretch X", 0.9, 1.1, 1.0),
            M::Sy => d("Stretch Y", 0.9, 1.1, 1.0),
            M::Decay => d("Decay", 0.8, 1.0, 0.98),
            M::WaveMode => w("Wave Shape", 0.0, 5.0, 0.0),
            M::WaveScale => d("Wave Size", 0.0, 3.0, 1.0),
            M::WaveAlpha => d("Wave", 0.0, 1.0, 0.8),
            M::WaveHue => d("Wave Hue", 0.0, 1.0, 0.0),
            M::WaveSat => d("Wave Colour", 0.0, 1.0, 0.0),
            M::WaveX => d("Wave X", 0.0, 1.0, 0.5),
            M::WaveY => d("Wave Y", 0.0, 1.0, 0.5),
            M::WaveThick => d("Wave Thickness", 1.0, 4.0, 1.5),
            M::ShapeSides => w("Shape Sides", 3.0, 12.0, 4.0),
            M::ShapeRad => d("Shape Size", 0.0, 1.0, 0.2),
            M::ShapeAng => d("Shape Angle", 0.0, 6.2832, 0.0),
            M::ShapeSpin => d("Shape Spin", -2.0, 2.0, 0.0),
            M::ShapeX => d("Shape X", 0.0, 1.0, 0.5),
            M::ShapeY => d("Shape Y", 0.0, 1.0, 0.5),
            M::ShapeAlpha => d("Shape", 0.0, 1.0, 0.0),
            M::ShapeTex => w("Shape Holds", 0.0, 1.0, 0.0),
            M::ShapeHue => d("Shape Hue", 0.0, 1.0, 0.0),
            M::OuterBorder => d("Outer Border", 0.0, 0.1, 0.0),
            M::InnerBorder => d("Inner Border", 0.0, 0.1, 0.0),
            M::BorderAlpha => d("Borders", 0.0, 1.0, 0.0),
            M::BorderHue => d("Border Hue", 0.0, 1.0, 0.0),
            M::MvX => w("Dots Across", 0.0, 48.0, 0.0),
            M::MvY => w("Dots Down", 0.0, 36.0, 0.0),
            M::MvAlpha => d("Dots", 0.0, 1.0, 0.0),
            M::DarkenCenter => d("Dark Centre", 0.0, 1.0, 0.0),
            M::Gamma => d("Gamma", 0.5, 3.0, 1.0),
            M::EchoZoom => d("Echo Zoom", 0.5, 2.0, 1.0),
            M::EchoAlpha => d("Echo", 0.0, 1.0, 0.0),
            M::EchoOrient => w("Echo Flip", 0.0, 3.0, 0.0),
            M::Brighten => d("Brighten", 0.0, 1.0, 0.0),
            M::Darken => d("Darken", 0.0, 1.0, 0.0),
            M::Solarize => d("Solarize", 0.0, 1.0, 0.0),
            M::Invert => d("Invert", 0.0, 1.0, 0.0),
            M::Hue => d("Hue", 0.0, 1.0, 0.0),
        }
    }

    /// Its name in the shader (`ZOOM`, `WAVE_MODE`, ...).
    pub fn glsl(self) -> String {
        let mut out = String::new();
        for (i, c) in format!("{self:?}").chars().enumerate() {
            if c.is_uppercase() && i > 0 {
                out.push('_');
            }
            out.push(c.to_ascii_uppercase());
        }
        out
    }
}

/// What moves a value.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MilkSrc {
    Bass,
    Mid,
    Treb,
    /// The same, slower (the attack of the music, not each hit).
    BassAtt,
    MidAtt,
    TrebAtt,
    /// A pulse on every beat, every bar.
    Beat,
    Bar,
    /// A slow swing: sin or cos of the time times this.
    Sin(f32),
    Cos(f32),
}

/// What the music is now, for a preset.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MilkIn {
    pub bass: f32,
    pub mid: f32,
    pub treb: f32,
    pub bass_att: f32,
    pub mid_att: f32,
    pub treb_att: f32,
    pub beat: f32,
    pub bar: f32,
    pub time: f32,
}

impl MilkIn {
    fn of(&self, s: MilkSrc) -> f32 {
        match s {
            MilkSrc::Bass => self.bass,
            MilkSrc::Mid => self.mid,
            MilkSrc::Treb => self.treb,
            MilkSrc::BassAtt => self.bass_att,
            MilkSrc::MidAtt => self.mid_att,
            MilkSrc::TrebAtt => self.treb_att,
            MilkSrc::Beat => self.beat,
            MilkSrc::Bar => self.bar,
            MilkSrc::Sin(r) => (self.time * r).sin(),
            MilkSrc::Cos(r) => (self.time * r).cos(),
        }
    }
}

type Term = (MilkSrc, f32);

/// A preset: the values it sets (the rest rest), each with what moves it.
pub struct MilkPreset {
    pub name: &'static str,
    pub set: Vec<(MilkParam, f32, Vec<Term>)>,
}

fn p(name: &'static str, set: Vec<(MilkParam, f32, Vec<Term>)>) -> MilkPreset {
    MilkPreset { name, set }
}

/// Krabton's presets.
pub fn presets() -> &'static [MilkPreset] {
    static P: std::sync::OnceLock<Vec<MilkPreset>> = std::sync::OnceLock::new();
    P.get_or_init(|| {
        use MilkParam as M;
        use MilkSrc as S;
        let v = |param, base, terms: &[Term]| (param, base, terms.to_vec());
        vec![
            p("Drain", vec![v(M::Zoom, 0.96, &[(S::Bass, -0.03)]), v(M::Rot, 0.02, &[(S::Sin(0.3), 0.02)]), v(M::Decay, 0.97, &[]), v(M::WaveMode, 1.0, &[]), v(M::WaveHue, 0.55, &[]), v(M::WaveSat, 0.6, &[]), v(M::DarkenCenter, 0.3, &[])]),
            p("Bloom", vec![v(M::Zoom, 1.03, &[(S::BassAtt, 0.04)]), v(M::Warp, 0.6, &[(S::Mid, 0.6)]), v(M::WaveHue, 0.85, &[(S::Sin(0.1), 0.1)]), v(M::WaveSat, 0.7, &[]), v(M::EchoAlpha, 0.4, &[]), v(M::EchoZoom, 1.2, &[]), v(M::Gamma, 1.4, &[]), v(M::Decay, 0.96, &[])]),
            p("Spill", vec![v(M::ShapeSides, 6.0, &[]), v(M::ShapeRad, 0.35, &[(S::Bass, 0.08)]), v(M::ShapeSpin, 0.3, &[]), v(M::ShapeAlpha, 0.9, &[]), v(M::ShapeTex, 1.0, &[]), v(M::Zoom, 1.01, &[]), v(M::Rot, 0.01, &[]), v(M::Decay, 0.96, &[]), v(M::WaveAlpha, 0.6, &[])]),
            p("Ink Ripple", vec![v(M::Ripple, 0.6, &[(S::Bass, 0.6)]), v(M::RippleFreq, 1.5, &[]), v(M::Decay, 0.985, &[]), v(M::WaveMode, 3.0, &[]), v(M::Solarize, 0.3, &[]), v(M::Zoom, 1.0, &[(S::Beat, 0.01)])]),
            p("Orbitals", vec![v(M::WaveMode, 2.0, &[]), v(M::WaveScale, 1.4, &[(S::Mid, 0.4)]), v(M::Rot, 0.0, &[(S::Sin(0.2), 0.04)]), v(M::Zoom, 0.99, &[]), v(M::Decay, 0.95, &[]), v(M::EchoAlpha, 0.5, &[]), v(M::EchoOrient, 1.0, &[]), v(M::WaveHue, 0.15, &[(S::Sin(0.07), 0.15)]), v(M::WaveSat, 0.9, &[])]),
            p("Furnace", vec![v(M::Zoom, 1.02, &[(S::Bass, 0.05)]), v(M::Dy, -0.004, &[]), v(M::Warp, 0.4, &[]), v(M::Decay, 0.96, &[]), v(M::WaveHue, 0.05, &[]), v(M::WaveSat, 1.0, &[]), v(M::Gamma, 1.6, &[]), v(M::Hue, 0.0, &[(S::Sin(0.05), 0.04)])]),
            p("Tunnel Vision", vec![v(M::Zoom, 0.94, &[(S::Bass, -0.04)]), v(M::ZoomRad, 0.08, &[]), v(M::Rot, 0.015, &[]), v(M::MvX, 24.0, &[]), v(M::MvY, 18.0, &[]), v(M::MvAlpha, 0.3, &[(S::Treb, 0.4)]), v(M::WaveMode, 1.0, &[]), v(M::DarkenCenter, 0.6, &[])]),
            p("Spin Cycle", vec![v(M::Rot, 0.0, &[(S::Sin(0.15), 0.08), (S::Treb, 0.03)]), v(M::ShapeTex, 1.0, &[]), v(M::ShapeSides, 4.0, &[]), v(M::ShapeRad, 0.5, &[]), v(M::ShapeAlpha, 0.6, &[]), v(M::Decay, 0.97, &[])]),
            p("Static Bloom", vec![v(M::Warp, 1.5, &[(S::Treb, 1.0)]), v(M::WarpSpeed, 2.0, &[]), v(M::WarpScale, 2.5, &[]), v(M::Decay, 0.93, &[]), v(M::WaveMode, 3.0, &[]), v(M::Solarize, 0.5, &[])]),
            p("Mirror Lake", vec![v(M::EchoAlpha, 0.5, &[]), v(M::EchoOrient, 2.0, &[]), v(M::Zoom, 1.01, &[]), v(M::Dy, 0.003, &[]), v(M::WaveY, 0.6, &[]), v(M::Decay, 0.97, &[]), v(M::WaveHue, 0.6, &[]), v(M::WaveSat, 0.5, &[])]),
            p("Pulse Grid", vec![v(M::MvX, 32.0, &[]), v(M::MvY, 24.0, &[]), v(M::MvAlpha, 0.6, &[]), v(M::Zoom, 1.0, &[(S::Beat, 0.06)]), v(M::Decay, 0.9, &[]), v(M::OuterBorder, 0.02, &[]), v(M::BorderAlpha, 0.0, &[(S::Beat, 0.8)]), v(M::WaveMode, 4.0, &[])]),
            p("Liquid Neon", vec![v(M::Warp, 0.8, &[]), v(M::WarpScale, 0.6, &[]), v(M::Zoom, 1.015, &[]), v(M::Rot, -0.01, &[]), v(M::WaveMode, 5.0, &[]), v(M::WaveHue, 0.8, &[(S::Sin(0.1), 0.2)]), v(M::WaveSat, 1.0, &[]), v(M::Gamma, 1.3, &[]), v(M::Decay, 0.975, &[])]),
            p("Stargate", vec![v(M::Zoom, 1.06, &[(S::BassAtt, 0.06)]), v(M::Decay, 0.92, &[]), v(M::WaveMode, 5.0, &[]), v(M::MvX, 20.0, &[]), v(M::MvY, 12.0, &[]), v(M::MvAlpha, 0.4, &[]), v(M::WaveHue, 0.5, &[]), v(M::WaveSat, 0.4, &[])]),
            p("Smoke Signal", vec![v(M::Dy, -0.01, &[]), v(M::Warp, 0.5, &[]), v(M::Decay, 0.985, &[]), v(M::Sx, 1.01, &[]), v(M::WaveAlpha, 0.5, &[]), v(M::WaveThick, 2.0, &[]), v(M::Darken, 0.4, &[]), v(M::Gamma, 1.2, &[])]),
            p("Shatter", vec![v(M::ShapeSides, 3.0, &[]), v(M::ShapeTex, 1.0, &[]), v(M::ShapeRad, 0.45, &[]), v(M::ShapeSpin, 1.2, &[(S::Bass, 1.0)]), v(M::ShapeAlpha, 0.8, &[]), v(M::Zoom, 0.98, &[]), v(M::EchoAlpha, 0.3, &[]), v(M::EchoOrient, 3.0, &[]), v(M::Decay, 0.95, &[])]),
            p("Heartbeat", vec![v(M::Zoom, 1.0, &[(S::Beat, 0.1), (S::Bass, 0.05)]), v(M::Decay, 0.9, &[]), v(M::WaveMode, 1.0, &[]), v(M::WaveScale, 1.0, &[(S::Bass, 1.0)]), v(M::InnerBorder, 0.03, &[]), v(M::BorderAlpha, 0.0, &[(S::Beat, 0.6)]), v(M::BorderHue, 0.95, &[]), v(M::WaveHue, 0.97, &[]), v(M::WaveSat, 0.8, &[])]),
            p("Aurora", vec![v(M::Dy, 0.002, &[]), v(M::Warp, 0.3, &[]), v(M::Ripple, 0.2, &[]), v(M::Decay, 0.99, &[]), v(M::WaveHue, 0.4, &[(S::Sin(0.05), 0.3)]), v(M::WaveSat, 0.8, &[]), v(M::WaveAlpha, 0.6, &[]), v(M::Gamma, 1.8, &[]), v(M::Hue, 0.0, &[(S::Sin(0.03), 0.05)])]),
            p("Vortex", vec![v(M::Rot, 0.06, &[(S::Bass, 0.04)]), v(M::RotRad, -0.08, &[]), v(M::Zoom, 0.97, &[]), v(M::ZoomRad, -0.05, &[]), v(M::Decay, 0.96, &[]), v(M::WaveMode, 1.0, &[]), v(M::WaveHue, 0.7, &[]), v(M::WaveSat, 0.6, &[])]),
            p("Glass Bead", vec![v(M::ShapeSides, 12.0, &[]), v(M::ShapeTex, 1.0, &[]), v(M::ShapeRad, 0.3, &[(S::Mid, 0.2)]), v(M::ShapeX, 0.5, &[(S::Sin(0.4), 0.2)]), v(M::ShapeY, 0.5, &[(S::Cos(0.3), 0.2)]), v(M::ShapeAlpha, 1.0, &[]), v(M::Decay, 0.94, &[]), v(M::Zoom, 1.02, &[])]),
            p("Acid Rain", vec![v(M::Dy, 0.015, &[(S::Treb, 0.01)]), v(M::MvX, 40.0, &[]), v(M::MvY, 30.0, &[]), v(M::MvAlpha, 0.5, &[]), v(M::WaveMode, 3.0, &[]), v(M::WaveHue, 0.3, &[]), v(M::WaveSat, 1.0, &[]), v(M::Decay, 0.92, &[]), v(M::Solarize, 0.4, &[])]),
            p("Deep Field", vec![v(M::Zoom, 1.004, &[]), v(M::Decay, 0.995, &[]), v(M::WaveMode, 2.0, &[]), v(M::WaveScale, 0.8, &[]), v(M::WaveAlpha, 0.4, &[]), v(M::EchoAlpha, 0.6, &[]), v(M::EchoZoom, 0.8, &[]), v(M::DarkenCenter, 0.4, &[])]),
            p("Cathedral", vec![v(M::EchoAlpha, 0.5, &[]), v(M::EchoOrient, 1.0, &[]), v(M::Invert, 0.0, &[(S::Bar, 1.0)]), v(M::ShapeSides, 4.0, &[]), v(M::ShapeAlpha, 0.5, &[(S::Beat, 0.5)]), v(M::ShapeRad, 0.6, &[]), v(M::Zoom, 1.03, &[]), v(M::Decay, 0.9, &[])]),
            p("Dissolve", vec![v(M::Warp, 2.0, &[]), v(M::WarpSpeed, 0.5, &[]), v(M::Decay, 0.97, &[]), v(M::WaveMode, 4.0, &[]), v(M::WaveAlpha, 0.7, &[]), v(M::Brighten, 0.5, &[])]),
            p("Event Horizon", vec![v(M::Zoom, 0.9, &[(S::Bass, -0.05)]), v(M::ZoomRad, 0.15, &[]), v(M::Rot, 0.03, &[]), v(M::DarkenCenter, 1.0, &[]), v(M::WaveMode, 1.0, &[]), v(M::WaveScale, 0.6, &[]), v(M::WaveHue, 0.08, &[]), v(M::WaveSat, 1.0, &[]), v(M::Decay, 0.97, &[]), v(M::Gamma, 1.5, &[])]),
        ]
    })
}

/// The values at rest.
pub fn defaults() -> [f32; MILK_N] {
    MilkParam::ALL.map(|m| m.def().default)
}

/// The values of preset `p` with the music `input`, in their ranges.
pub fn eval(p: &MilkPreset, input: &MilkIn) -> [f32; MILK_N] {
    let mut out = defaults();
    for (m, base, terms) in &p.set {
        out[m.index()] = base + terms.iter().map(|(s, a)| input.of(*s) * a).sum::<f32>();
    }
    clamp(&mut out);
    out
}

fn clamp(v: &mut [f32; MILK_N]) {
    for (m, x) in MilkParam::ALL.iter().zip(v.iter_mut()) {
        let d = m.def();
        *x = if x.is_finite() { x.clamp(d.min, d.max) } else { d.default };
        if d.whole {
            *x = x.round();
        }
    }
}

/// Between `a` and `b` (at `t`): the values on the way, the modes and counts `b`'s from half way.
pub fn blend(a: &[f32; MILK_N], b: &[f32; MILK_N], t: f32) -> [f32; MILK_N] {
    let t = t.clamp(0.0, 1.0);
    let mut out = *b;
    for (k, m) in MilkParam::ALL.iter().enumerate() {
        out[k] = if m.def().whole {
            if t < 0.5 {
                a[k]
            } else {
                b[k]
            }
        } else {
            a[k] + (b[k] - a[k]) * t
        };
    }
    out
}

/// The hand on top of a preset: a tweak `t` (0..1, 0.5: none) moves value `m` up to half its range either way.
pub fn tweak(v: &mut [f32; MILK_N], tweaks: &BTreeMap<MilkParam, f32>) {
    for (m, t) in tweaks {
        let d = m.def();
        v[m.index()] += (t.clamp(0.0, 1.0) - 0.5) * (d.max - d.min);
    }
    clamp(v);
}

/// How often a Look's Milk goes on to another preset by itself.
pub const AUTO: [(&str, u32); 4] = [("Off", 0), ("4 bars", 4), ("8 bars", 8), ("16 bars", 16)];
/// How long one preset takes to melt into the next (beats).
pub const BLENDS: [(f32, &str); 5] = [(0.0, "Cut"), (1.0, "1 beat"), (2.0, "2 beats"), (4.0, "4 beats"), (8.0, "8 beats")];

/// A Look's Milk: its preset, the hand's tweaks, how it changes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MilkLook {
    pub preset: String,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub tweaks: BTreeMap<MilkParam, f32>,
    /// Every so many bars another preset (see `AUTO`), the next or one by chance.
    pub auto: u32,
    pub random: bool,
    pub blend: f32,
}

impl Default for MilkLook {
    fn default() -> Self {
        Self { preset: presets()[0].name.to_string(), tweaks: BTreeMap::new(), auto: 0, random: false, blend: 2.0 }
    }
}

impl MilkLook {
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }

    /// Its preset's place (the first when it is not there).
    pub fn place(&self) -> usize {
        place_of(&self.preset)
    }
}

/// A preset's place by its name (the first when there is none).
pub fn place_of(name: &str) -> usize {
    presets().iter().position(|p| p.name == name).unwrap_or(0)
}

/// Milk playing: the preset, the one it melts from, the music slowed, the bars counted.
#[derive(Clone, Debug, Default)]
pub struct MilkRun {
    /// The preset now (by place), and what the Look asked for last (a change there takes over).
    pub now: usize,
    asked: Option<usize>,
    from: Option<([f32; MILK_N], f32)>,
    last: Option<[f32; MILK_N]>,
    att: [f32; 3],
    bars: u32,
}

impl MilkRun {
    /// One frame: `dt` seconds on, the music `input` (its `_att`s are made
    /// here), a bar begun or not, the beats a second (for the blend),
    /// `pick` for a preset by chance. The values to draw with.
    pub fn frame(&mut self, look: &MilkLook, mut input: MilkIn, dt: f32, bar_hit: bool, beats_per_sec: f32, mut pick: impl FnMut(usize) -> usize) -> [f32; MILK_N] {
        let n = presets().len();
        let k = 0.5f32.powf(dt / 0.4);
        for (a, v) in self.att.iter_mut().zip([input.bass, input.mid, input.treb]) {
            *a = *a * k + v * (1.0 - k);
        }
        (input.bass_att, input.mid_att, input.treb_att) = (self.att[0], self.att[1], self.att[2]);
        let asked = look.place();
        let mut go = None;
        if self.asked != Some(asked) {
            go = Some(asked);
            self.asked = Some(asked);
            self.bars = 0;
        } else if bar_hit {
            self.bars += 1;
            if look.auto > 0 && self.bars % look.auto == 0 {
                go = Some(if look.random { pick(n) } else { (self.now + 1) % n });
            }
        }
        if let Some(g) = go {
            if g != self.now || self.last.is_none() {
                if let Some(last) = self.last.filter(|_| look.blend > 0.0) {
                    self.from = Some((last, 0.0));
                }
                self.now = g.min(n - 1);
            }
        }
        let target = eval(&presets()[self.now], &input);
        let mut out = match &mut self.from {
            Some((a, t)) => {
                *t += dt * beats_per_sec / look.blend.max(0.01);
                let s = t.clamp(0.0, 1.0);
                let s = s * s * (3.0 - 2.0 * s);
                let v = blend(a, &target, s);
                if *t >= 1.0 {
                    self.from = None;
                }
                v
            }
            None => target,
        };
        self.last = Some(out);
        tweak(&mut out, &look.tweaks);
        out
    }

    pub fn name(&self) -> &'static str {
        presets()[self.now.min(presets().len() - 1)].name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_value_has_its_place_and_a_name_for_the_shader() {
        for (i, m) in MilkParam::ALL.iter().enumerate() {
            assert_eq!(m.index(), i);
            let d = m.def();
            assert!(d.min <= d.default && d.default <= d.max, "{m:?}");
        }
        assert_eq!(MilkParam::WaveMode.glsl(), "WAVE_MODE");
        assert_eq!(MilkParam::Zoom.glsl(), "ZOOM");
    }

    #[test]
    fn presets_are_many_named_once_and_in_their_ranges() {
        let ps = presets();
        assert!(ps.len() >= 24);
        for (i, p) in ps.iter().enumerate() {
            assert!(ps.iter().skip(i + 1).all(|q| q.name != p.name), "{} twice", p.name);
            let loud = MilkIn { bass: 1.0, mid: 1.0, treb: 1.0, beat: 1.0, bar: 1.0, time: 3.0, ..Default::default() };
            for v in [eval(p, &MilkIn::default()), eval(p, &loud)] {
                for (m, x) in MilkParam::ALL.iter().zip(v) {
                    let d = m.def();
                    assert!(x >= d.min && x <= d.max, "{}: {m:?} {x}", p.name);
                }
            }
        }
    }

    #[test]
    fn the_music_moves_a_preset() {
        let p = &presets()[place_of("Heartbeat")];
        let quiet = eval(p, &MilkIn::default());
        let kick = eval(p, &MilkIn { bass: 1.0, beat: 1.0, ..Default::default() });
        assert!(kick[MilkParam::Zoom.index()] > quiet[MilkParam::Zoom.index()] + 0.1);
        assert_eq!(quiet[MilkParam::WaveMode.index()], 1.0, "a circle");
    }

    #[test]
    fn one_preset_melts_into_the_next_and_the_hand_moves_it() {
        let mut run = MilkRun::default();
        let mut look = MilkLook { preset: "Drain".into(), blend: 2.0, ..Default::default() };
        let quiet = MilkIn::default();
        let a = run.frame(&look, quiet, 0.02, false, 2.0, |_| 0);
        look.preset = "Furnace".into();
        let b = eval(&presets()[place_of("Furnace")], &quiet);
        let first = run.frame(&look, quiet, 0.02, false, 2.0, |_| 0);
        let z = MilkParam::Zoom.index();
        assert!((first[z] - a[z]).abs() < 0.01, "starts where it was");
        let mut v = first;
        for _ in 0..60 {
            v = run.frame(&look, quiet, 0.02, false, 2.0, |_| 0);
        }
        assert!((v[z] - b[z]).abs() < 1e-5, "a second at 2 beats a second: there");
        assert_eq!(run.name(), "Furnace");
        look.tweaks.insert(MilkParam::Zoom, 1.0);
        let pushed = run.frame(&look, quiet, 0.02, false, 2.0, |_| 0);
        assert!((pushed[z] - (b[z] + 0.15).min(1.15)).abs() < 1e-4, "a tweak all the way up: half the range more");
    }

    #[test]
    fn it_goes_on_by_itself_every_so_many_bars() {
        let mut run = MilkRun::default();
        let look = MilkLook { auto: 4, blend: 0.0, ..Default::default() };
        let q = MilkIn::default();
        run.frame(&look, q, 0.02, false, 2.0, |_| 0);
        assert_eq!(run.now, 0);
        for bar in 1..=8 {
            run.frame(&look, q, 0.02, true, 2.0, |_| 0);
            assert_eq!(run.now, bar / 4, "bar {bar}");
        }
        let random = MilkLook { auto: 4, random: true, blend: 0.0, ..Default::default() };
        for _ in 0..4 {
            run.frame(&random, q, 0.02, true, 2.0, |_| 7);
        }
        assert_eq!(run.now, 7, "by chance");
    }

    #[test]
    fn a_looks_milk_survives_a_save() {
        let mut m = MilkLook { preset: "Vortex".into(), auto: 8, random: true, blend: 4.0, ..Default::default() };
        m.tweaks.insert(MilkParam::Decay, 0.7);
        let back: MilkLook = serde_json::from_str(&serde_json::to_string(&m).unwrap()).unwrap();
        assert_eq!(back, m);
        assert_eq!(place_of("Nope"), 0);
    }
}

//! Inferno's face: a house on fire. The whole panel burns behind the grid
//! (see `inferno_fire`), a charred wall and black beams in front of the
//! blaze: a step that is on is a window with the fire behind it, one that is
//! off is boarded up. A step that plays makes its column erupt (an accent
//! hotter, ratchets spit sparks, an octave jump flares), and Burn takes the
//! whole panel. Sparks and embers are a fixed pool of particles kept in the
//! face's memory (nothing allocated for them as they burn). Flames: Calm
//! burns low, Off not at all. Without the shader, tongues from that same pool.
//!
//! The same face in the device chain (one row of values at a time) and
//! filling the window (every row, with its settings beside it). Clicks
//! for what has steps (click: the next, right-click: the one before), drags
//! for what is continuous, dropdowns for long lists.

use super::seq_view::Live;
use crate::app::App;
use crate::theme::*;
use crate::widgets::*;
use egui::epaint::Mesh;
use egui::{pos2, vec2, Align2, Color32, Context, FontId, Id, Painter, Pos2, Rect, RichText, Sense, Shape, Stroke, Ui, UiBuilder};
use krabton_engine::note_name;
use krabton_engine::params::ParamSpec;
use krabton_engine::sequencers::chance::CONDITION_TAGS;
use krabton_engine::sequencers::inferno::{self as inf, Step, PAGE, PATTERNS, STEPS};

// ---------------------------------------------------------------- the palette

const fn hex(v: u32) -> Color32 {
    Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8)
}

pub(crate) const CHARCOAL: Color32 = hex(0x150c09);
const SOOT: Color32 = hex(0x0a0504);
const ASH: Color32 = hex(0x3b2a24);
const CINDER: Color32 = hex(0x5a2e1e);
const BLOOD: Color32 = hex(0xa3180c);
pub(crate) const EMBER: Color32 = hex(0xff4d12);
const FLAME: Color32 = hex(0xff8a1f);
pub(crate) const GOLD: Color32 = hex(0xffc94a);
const WHITE_HOT: Color32 = hex(0xfff1c8);
const SMOKE: Color32 = hex(0x6b625e);
/// Plates to read on, burnt red: black over the yellow of the fire turns olive.
const SCORCH: Color32 = hex(0x2c0500);

fn alpha(c: Color32, a: f32) -> Color32 {
    let a = a.clamp(0.0, 1.0);
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), (a * 255.0) as u8)
}

/// Fire's colour at a heat (0: a dull coal, 1: white hot).
fn heat_color(h: f32) -> Color32 {
    let h = h.clamp(0.0, 1.0);
    let stops = [(0.0, CINDER), (0.3, BLOOD), (0.55, EMBER), (0.75, FLAME), (0.9, GOLD), (1.0, WHITE_HOT)];
    for w in stops.windows(2) {
        if h <= w[1].0 {
            return mix(w[0].1, w[1].1, (h - w[0].0) / (w[1].0 - w[0].0));
        }
    }
    WHITE_HOT
}

/// A rectangle from one colour at its top to another at its bottom.
fn gradient(p: &Painter, r: Rect, top: Color32, bottom: Color32) {
    let mut m = Mesh::default();
    m.colored_vertex(r.left_top(), top);
    m.colored_vertex(r.right_top(), top);
    m.colored_vertex(r.right_bottom(), bottom);
    m.colored_vertex(r.left_bottom(), bottom);
    m.add_triangle(0, 1, 2);
    m.add_triangle(0, 2, 3);
    p.add(Shape::mesh(m));
}


/// Its lettering, in its own look (the logo fonts come with the skins).
fn logo_font(size: f32) -> FontId {
    if crate::skin::current().is_some() {
        crate::skin::Font::Metal.id(size)
    } else {
        FontId::proportional(size)
    }
}

/// What it writes on the fire: square and hard.
fn hard_font(size: f32) -> FontId {
    if crate::skin::current().is_some() {
        crate::skin::Font::Sport.id(size)
    } else {
        FontId::proportional(size)
    }
}

/// Text that stands out on fire: a black edge all round.
fn inked(p: &Painter, pos: Pos2, anchor: Align2, s: &str, font: FontId, c: Color32) {
    for (dx, dy) in [(-1.0, 0.0), (1.0, 0.0), (0.0, -1.0), (0.0, 1.0), (-1.0, -1.0), (1.0, 1.0), (1.0, -1.0), (-1.0, 1.0)] {
        p.text(pos + vec2(dx, dy), anchor, s, font.clone(), Color32::BLACK);
    }
    p.text(pos, anchor, s, font, c);
}

/// The logo: a flame job along the letters, yellow into red into pink, on a
/// black edge, like the name on a hot rod.
fn flame_logo(p: &Painter, left_center: Pos2, s: &str, size: f32, flick: f32) {
    let font = logo_font(size);
    let galley = p.layout_no_wrap(s.to_string(), font.clone(), Color32::WHITE);
    let w = galley.size().x.max(1.0);
    let at = left_center - vec2(0.0, galley.size().y * 0.5);
    for (dx, dy) in [(-2.0, 0.0), (2.0, 0.0), (0.0, -2.0), (0.0, 2.0), (-1.5, -1.5), (1.5, 1.5), (1.5, -1.5), (-1.5, 1.5)] {
        p.text(left_center + vec2(dx, dy), Align2::LEFT_CENTER, s, font.clone(), Color32::BLACK);
    }
    // Letter by letter, each its colour along the name.
    let mut x = at.x;
    for (k, ch) in s.chars().enumerate() {
        let t = (x - at.x + 3.0) / w;
        let c = if t < 0.45 { mix(hex(0xfff6b0), hex(0xff7a00), t / 0.45) } else { mix(hex(0xff7a00), hex(0xff1f4a), (t - 0.45) / 0.55) };
        let c = mix(c, hex(0xffffff), (flick * 0.4 + 0.1 * ((k as f32 * 1.7).sin())).clamp(0.0, 0.35));
        let one = p.layout_no_wrap(ch.to_string(), font.clone(), c);
        let advance = one.size().x;
        p.galley(pos2(x, at.y), one, c);
        x += advance;
    }
}

/// A plate with its corners cut, hard as a blade.
fn cut_plate(p: &Painter, r: Rect, cut: f32, fill: Color32, line: Stroke) {
    let c = cut.min(r.width() * 0.3).min(r.height() * 0.3);
    let pts = vec![pos2(r.left() + c, r.top()), r.right_top(), pos2(r.right(), r.bottom() - c), pos2(r.right() - c, r.bottom()), r.left_bottom(), pos2(r.left(), r.top() + c)];
    p.add(Shape::convex_polygon(pts, fill, line));
}

// ---------------------------------------------------------------- the flames

/// How many particles burn at most.
pub(crate) const POOL: usize = 320;
const FLAME_P: u8 = 0;
const EMBER_P: u8 = 1;
const SPARK_P: u8 = 2;
const SMOKE_P: u8 = 3;

#[derive(Clone, Copy, Debug, Default)]
struct Particle {
    /// Where (from the face's top left) and how fast (points a second).
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    /// Seconds left of it and how long it had.
    life: f32,
    max: f32,
    size: f32,
    kind: u8,
    heat: f32,
    seed: f32,
}

/// The fire on a face: its particles, a fixed pool used round and round.
#[derive(Clone, Debug)]
pub(crate) struct Fire {
    p: [Particle; POOL],
    /// The next slot to burn in (the oldest goes first when all burn).
    next: usize,
    /// The engine's count of steps struck, as far as the flames have seen it.
    hits: u32,
    rng: u32,
    /// Seconds burning, how much the panel flares (Burn), and what to emit
    /// over time (fractions carried to the next frame).
    time: f64,
    flare: f32,
    carry: f32,
    ember_carry: f32,
    /// How each step still glows from its last hit (0..1).
    glow: [f32; STEPS],
}

impl Default for Fire {
    fn default() -> Self {
        Self { p: [Particle::default(); POOL], next: 0, hits: u32::MAX, rng: 0x5EED_F12E, time: 0.0, flare: 0.0, carry: 0.0, ember_carry: 0.0, glow: [0.0; STEPS] }
    }
}

impl Fire {
    fn unit(&mut self) -> f32 {
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.rng = x;
        (x >> 8) as f32 / 16_777_216.0
    }

    #[allow(clippy::too_many_arguments)]
    fn spawn(&mut self, kind: u8, x: f32, y: f32, vx: f32, vy: f32, life: f32, size: f32, heat: f32) {
        let seed = self.unit() * 100.0;
        self.p[self.next] = Particle { x, y, vx, vy, life, max: life, size, kind, heat, seed };
        self.next = (self.next + 1) % POOL;
    }

    /// How many burn now.
    #[cfg(test)]
    pub(crate) fn alive(&self) -> usize {
        self.p.iter().filter(|p| p.life > 0.0).count()
    }

    /// A burst of flame at `at` (heat 0..1), with embers and some smoke.
    fn burst(&mut self, at: Pos2, width: f32, heat: f32, count: usize, calm: bool) {
        let n = if calm { count.div_ceil(2) } else { count };
        for _ in 0..n {
            let x = at.x + (self.unit() - 0.5) * width;
            let (vx, vy) = ((self.unit() - 0.5) * 18.0, -70.0 - 110.0 * self.unit() * (0.5 + heat));
            let life = 0.4 + 0.5 * self.unit();
            let size = width * (0.2 + 0.2 * self.unit()) * (0.7 + 0.6 * heat);
            self.spawn(FLAME_P, x, at.y, vx, vy, life, size, heat);
        }
        for _ in 0..(n / 3).max(1) {
            let (vx, vy) = ((self.unit() - 0.5) * 40.0, -50.0 - 60.0 * self.unit());
            let life = 0.9 + 0.9 * self.unit();
            let (x, size) = (at.x + (self.unit() - 0.5) * width, 1.2 + self.unit() * 1.3);
            self.spawn(EMBER_P, x, at.y, vx, vy, life, size, heat);
        }
        if !calm && self.unit() < 0.5 {
            let (life, vx) = (1.0 + self.unit(), (self.unit() - 0.5) * 8.0);
            self.spawn(SMOKE_P, at.x, at.y - width * 0.6, vx, -18.0, life, width * 0.25, 0.0);
        }
    }

    /// Sparks spat out (ratchets).
    fn sparks(&mut self, at: Pos2, count: usize) {
        for _ in 0..count {
            let a = std::f32::consts::PI * (1.05 + 0.9 * self.unit());
            let v = 90.0 + 120.0 * self.unit();
            let life = 0.25 + 0.3 * self.unit();
            self.spawn(SPARK_P, at.x, at.y, a.cos() * v, a.sin() * v, life, 1.0, 1.0);
        }
    }

    /// Moves everything on by `dt` seconds.
    fn update(&mut self, dt: f32) {
        self.time += dt as f64;
        self.flare = (self.flare - dt * 1.5).max(0.0);
        for g in self.glow.iter_mut() {
            *g = (*g - dt * 2.5).max(0.0);
        }
        let t = self.time as f32;
        for p in self.p.iter_mut().filter(|p| p.life > 0.0) {
            p.life -= dt;
            match p.kind {
                SPARK_P => p.vy += 260.0 * dt,
                FLAME_P => {
                    // Flicker sideways, slow down as it cools.
                    p.vx += (t * 23.0 + p.seed).sin() * 120.0 * dt;
                    p.vy *= 1.0 - 0.6 * dt;
                }
                SMOKE_P => p.size += 10.0 * dt,
                _ => p.vx += (t * 5.0 + p.seed).sin() * 30.0 * dt,
            }
            p.x += p.vx * dt;
            p.y += p.vy * dt;
        }
    }

    /// The shader burns the flames when it can; then only sparks and embers fly here.
    fn draw(&self, p: &Painter, origin: Pos2, blazing: bool) {
        let t = self.time as f32;
        for kind in [SMOKE_P, FLAME_P, EMBER_P, SPARK_P] {
            if kind == FLAME_P {
                if !blazing {
                    self.draw_flames(p, origin);
                }
                continue;
            }
            for q in self.p.iter().filter(|q| q.life > 0.0 && q.kind == kind) {
                let f = (q.life / q.max).clamp(0.0, 1.0);
                let at = origin + vec2(q.x, q.y);
                match kind {
                    SMOKE_P => {
                        p.circle_filled(at, q.size, alpha(SMOKE, 0.16 * f));
                    }
                    EMBER_P => {
                        let tw = 0.6 + 0.4 * (t * 17.0 + q.seed).sin();
                        p.circle_filled(at, q.size, alpha(heat_color(0.7 + 0.3 * f), f * tw));
                    }
                    _ => {
                        let tail = at - vec2(q.vx, q.vy) * 0.03;
                        p.line_segment([tail, at], Stroke::new(1.2_f32, alpha(heat_color(0.85 + 0.15 * f), f)));
                    }
                }
            }
        }
    }

    /// The flames, as tongues in one mesh: a wide red one with a yellow one
    /// inside, hot at the root, wavering, fading at the tip, adding light
    /// to what is under them.
    fn draw_flames(&self, p: &Painter, origin: Pos2) {
        let t = self.time as f32;
        let mut m = Mesh::default();
        for q in self.p.iter().filter(|q| q.life > 0.0 && q.kind == FLAME_P) {
            let f = (q.life / q.max).clamp(0.0, 1.0);
            let at = origin + vec2(q.x, q.y);
            let flick = 0.85 + 0.15 * (t * 31.0 + q.seed).sin();
            let w = q.size * (0.35 + 0.65 * f) * flick;
            let l = q.size * 3.0 * (0.4 + 0.6 * f) * flick;
            let sway = (t * 13.0 + q.seed).sin() * w * 0.9;
            let heat = (q.heat * 0.35 + f * 0.7).min(1.0);
            tongue(&mut m, at, w, l, sway, heat_color(heat * 0.7), 0.55 * f);
            tongue(&mut m, at + vec2(0.0, -w * 0.1), w * 0.5, l * 0.6, sway * 0.6, heat_color((heat + 0.25).min(1.0)), 0.9 * f);
        }
        if !m.is_empty() {
            p.add(Shape::mesh(m));
        }
    }
}

/// Fire's colour adding to what is under it (premultiplied, hardly any alpha).
fn light(c: Color32, a: f32) -> Color32 {
    let a = a.clamp(0.0, 1.0);
    let k = |v: u8| (v as f32 * a) as u8;
    Color32::from_rgba_premultiplied(k(c.r()), k(c.g()), k(c.b()), (a * 60.0) as u8)
}

/// A tongue of flame on `m`: its root at `at`, `w` wide, `l` tall, its tip
/// swayed sideways.
fn tongue(m: &mut Mesh, at: Pos2, w: f32, l: f32, sway: f32, color: Color32, a: f32) {
    let i = m.vertices.len() as u32;
    let core = light(color, a);
    let edge = light(color, a * 0.45);
    m.colored_vertex(at + vec2(sway * 0.15, -l * 0.25), core);
    m.colored_vertex(at + vec2(0.0, w * 0.55), edge);
    m.colored_vertex(at + vec2(w * 0.8, w * 0.1), edge);
    m.colored_vertex(at + vec2(w * 0.75 + sway * 0.3, -l * 0.35), edge);
    m.colored_vertex(at + vec2(sway, -l), light(color, 0.0));
    m.colored_vertex(at + vec2(-w * 0.75 + sway * 0.3, -l * 0.35), edge);
    m.colored_vertex(at + vec2(-w * 0.8, w * 0.1), edge);
    for k in 1..=6 {
        m.add_triangle(i, i + k, i + k % 6 + 1);
    }
}

fn fire_id(id: Id) -> Id {
    id.with("fire")
}

/// The face's fire, taken out of its memory (put back with `put_fire`).
fn take_fire(ctx: &Context, id: Id) -> Fire {
    ctx.data_mut(|d| std::mem::take(d.get_temp_mut_or_default::<Fire>(fire_id(id))))
}

fn put_fire(ctx: &Context, id: Id, fire: Fire) {
    ctx.data_mut(|d| *d.get_temp_mut_or_default::<Fire>(fire_id(id)) = fire);
}

// ---------------------------------------------------------------- the rows

/// The rows of values a step has, below its notes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Lane {
    Velocity,
    Gate,
    Ratchet,
    Curve,
    Chance,
    Condition,
    Nudge,
    Wob,
    Mod,
    Glide,
}

const LANES: [Lane; 10] = [Lane::Velocity, Lane::Gate, Lane::Ratchet, Lane::Curve, Lane::Chance, Lane::Condition, Lane::Nudge, Lane::Wob, Lane::Mod, Lane::Glide];

impl Lane {
    fn name(self) -> &'static str {
        match self {
            Lane::Velocity => "Velocity",
            Lane::Gate => "Gate",
            Lane::Ratchet => "Ratchet",
            Lane::Curve => "Curve",
            Lane::Chance => "Chance",
            Lane::Condition => "Cond",
            Lane::Nudge => "Nudge",
            Lane::Wob => "Wob",
            Lane::Mod => "Mod",
            Lane::Glide => "Glide",
        }
    }

    fn tip(self) -> &'static str {
        match self {
            Lane::Velocity => "How hard each step plays. Drag across the steps",
            Lane::Gate => "How long each note is, in eighths of a step (up to two steps). Drag across the steps",
            Lane::Ratchet => "Hits in the step (1 to 8). Click: one more, right-click: one less",
            Lane::Curve => "How the ratchets go: even, speeding up, slowing down, fading, swelling. Click: the next, right-click: the one before",
            Lane::Chance => "The chance the step plays. Drag across the steps",
            Lane::Condition => "When it plays: always, 1:2 (the first of every two times round), fill, first, pre (the last condition came true)... Click: the next, right-click: the one before",
            Lane::Nudge => "Earlier (down) or later (up) than its step. Drag across the steps",
            Lane::Wob => "The rate the mod wheel wobbles at while the step sounds (to the instrument's knob it is patched to). Click: the next, right-click: the one before",
            Lane::Mod => "How far the wob goes (or, without a wob rate, where the mod wheel stays). Drag across the steps",
            Lane::Glide => "How long a Bend glide out of this step takes (of Glide Time). Drag across the steps",
        }
    }

    /// Continuous: (min, max) to drag between.
    fn range(self) -> Option<(f32, f32)> {
        match self {
            Lane::Velocity => Some((1.0, 127.0)),
            Lane::Gate => Some((1.0, 16.0)),
            Lane::Chance => Some((0.0, 100.0)),
            Lane::Nudge => Some((-45.0, 45.0)),
            Lane::Mod => Some((0.0, 127.0)),
            Lane::Glide => Some((0.0, 200.0)),
            _ => None,
        }
    }

    fn get(self, s: &Step) -> f32 {
        match self {
            Lane::Velocity => s.velocity as f32,
            Lane::Gate => s.gate as f32,
            Lane::Ratchet => s.ratchet as f32,
            Lane::Curve => s.curve as f32,
            Lane::Chance => s.chance as f32,
            Lane::Condition => s.condition as f32,
            Lane::Nudge => s.nudge as f32,
            Lane::Wob => s.wob as f32,
            Lane::Mod => s.modv as f32,
            Lane::Glide => s.glide as f32,
        }
    }

    fn set(self, s: &mut Step, v: f32) {
        let v = v.round();
        match self {
            Lane::Velocity => s.velocity = v.clamp(1.0, 127.0) as u8,
            Lane::Gate => s.gate = v.clamp(1.0, 16.0) as u8,
            Lane::Ratchet => s.ratchet = v.clamp(1.0, 8.0) as u8,
            Lane::Curve => s.curve = v.clamp(0.0, inf::CURVES.len() as f32 - 1.0) as u8,
            Lane::Chance => s.chance = v.clamp(0.0, 100.0) as u8,
            Lane::Condition => s.condition = v.clamp(0.0, CONDITION_TAGS.len() as f32 - 1.0) as u8,
            Lane::Nudge => s.nudge = v.clamp(-45.0, 45.0) as i8,
            Lane::Wob => s.wob = v.clamp(0.0, inf::WOBS.len() as f32 - 1.0) as u8,
            Lane::Mod => s.modv = v.clamp(0.0, 127.0) as u8,
            Lane::Glide => s.glide = v.clamp(0.0, 200.0) as u8,
        }
    }

    /// Discrete: how many values (click goes round them).
    fn count(self) -> usize {
        match self {
            Lane::Ratchet => 8,
            Lane::Curve => inf::CURVES.len(),
            Lane::Condition => CONDITION_TAGS.len(),
            Lane::Wob => inf::WOBS.len(),
            _ => 0,
        }
    }

    fn shown(self, s: &Step) -> String {
        match self {
            Lane::Ratchet => format!("x{}", s.ratchet),
            Lane::Curve => ["=", ">>", "<<", "\\", "/"][(s.curve as usize).min(4)].to_string(),
            Lane::Condition => CONDITION_TAGS[(s.condition as usize).min(CONDITION_TAGS.len() - 1)].to_string(),
            Lane::Wob => if s.wob == 0 { String::new() } else { inf::WOBS[s.wob as usize].to_string() },
            _ => String::new(),
        }
    }
}

/// The switches a step has: accent, slide, tie, octave jump, fifth, lock, mute.
const FLAGS: [(&str, &str); 7] = [
    ("ACC", "Accent: harder, and an accent for a synth that has them (velocity 100 and up)"),
    ("SLIDE", "Slide into the next step: the note stays down until the next starts (Legato: a mono synth glides; Bend: the next bends from it)"),
    ("TIE", "Tie: the note before goes on through this step, without a new attack"),
    ("8VA", "An octave up on top of its octave"),
    ("5TH", "A fifth up"),
    ("LOCK", "Locked: Evolve and Mutate leave it as it is"),
    ("MUTE", "Muted: silent, but kept"),
];

fn flag(s: &Step, f: usize) -> bool {
    [s.accent, s.slide, s.tie, s.jump, s.fifth, s.lock, s.mute][f]
}

fn flip(s: &mut Step, f: usize) {
    match f {
        0 => s.accent = !s.accent,
        1 => s.slide = !s.slide,
        2 => s.tie = !s.tie,
        3 => s.jump = !s.jump,
        4 => s.fifth = !s.fifth,
        5 => s.lock = !s.lock,
        _ => s.mute = !s.mute,
    }
}

// ---------------------------------------------------------------- what the app does for it

/// What the face asks of the app.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Act {
    /// A new line into a pattern (for a song part: on its chords).
    Ignite { pattern: usize, part: Option<usize> },
    /// The wob to an instrument knob (or off it).
    PatchWob(Option<u16>),
    /// The instrument its genre pairs with.
    LoadPair,
    /// The Band window.
    BandWindow,
}

/// What the face knows of the app around it.
#[derive(Clone, Debug, Default)]
pub(crate) struct Around {
    /// The song's parts (when the band plays parts and it is in it).
    pub parts: Vec<String>,
    /// The instrument's knobs the wob can go to, and the one it goes to.
    pub knobs: Vec<(u16, String)>,
    pub patched: Option<u16>,
    /// The instrument on the track.
    pub instrument: String,
    /// The band's chord now (with the part), for the big face's Band section.
    pub chord: Option<String>,
}

// ---------------------------------------------------------------- the face

/// What the face keeps between frames: the page shown, whether it follows
/// the playhead, the row of values shown in the chain.
#[derive(Clone, Copy, Debug)]
struct View {
    page: usize,
    follow: bool,
    lane: usize,
}

impl Default for View {
    fn default() -> Self {
        Self { page: 0, follow: true, lane: 0 }
    }
}

/// The pattern as Evolve has made it, and what from.
#[derive(Clone)]
struct EvolveShown((usize, u32, u32, u32, usize, [Step; STEPS]), [Step; STEPS]);

/// The clipboard for a pattern (copy and paste).
#[derive(Clone, Default)]
struct Clip(Option<(Vec<Step>, usize)>);

fn seed_now() -> u32 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.subsec_nanos() ^ d.as_secs() as u32).unwrap_or(7)
}

fn pick(p: &[f32], i: usize, n: usize) -> usize {
    (p.get(i).copied().unwrap_or(0.0).round().max(0.0) as usize).min(n.saturating_sub(1))
}

/// A cell that is clicked through values: Some(+1) for a click, Some(-1)
/// for a right-click.
fn clicks(resp: &egui::Response) -> Option<i32> {
    if resp.clicked() {
        Some(1)
    } else if resp.secondary_clicked() {
        Some(-1)
    } else {
        None
    }
}

/// A fire button: lit when on.
fn fire_button(ui: &mut Ui, r: Rect, id: Id, label: &str, on: bool, hot: f32) -> egui::Response {
    let resp = ui.interact(r, id, Sense::click());
    let p = ui.painter();
    let cut = (r.height() * 0.28).min(6.0);
    if on {
        cut_plate(p, r, cut, heat_color(0.82 + 0.18 * hot), Stroke::new(1.0_f32, WHITE_HOT));
    } else {
        cut_plate(p, r, cut, if resp.hovered() { hex(0x3a0a04) } else { hex(0x0a0404) }, Stroke::new(1.0_f32, if resp.hovered() { EMBER } else { hex(0xb3200c) }));
    }
    let font = hard_font((r.height() * 0.5).clamp(9.0, 14.0));
    let clip = p.with_clip_rect(r);
    if on {
        clip.text(r.center(), Align2::CENTER_CENTER, label, font, Color32::BLACK);
    } else {
        inked(&clip, r.center(), Align2::CENTER_CENTER, label, font, hex(0xffd9a0));
    }
    resp
}

/// A dropdown for a choice parameter.
fn dropdown(ui: &mut Ui, r: Rect, id: Id, spec: &ParamSpec, value: &mut f32) -> bool {
    let krabton_engine::params::ParamKind::Choice(opts) = spec.kind else { return false };
    let cur = (value.round().max(0.0) as usize).min(opts.len() - 1);
    let mut changed = false;
    let mut child = ui.new_child(UiBuilder::new().max_rect(r));
    egui::ComboBox::from_id_salt(id).width(r.width() - 6.0).height(420.0).selected_text(RichText::new(opts[cur]).small().color(GOLD)).show_ui(&mut child, |ui| {
        for (i, o) in opts.iter().enumerate() {
            if ui.selectable_label(i == cur, *o).clicked() && i != cur {
                *value = i as f32;
                changed = true;
            }
        }
    });
    changed
}

/// Draws Inferno's face. Returns whether params or data changed, and what
/// it asks of the app.
#[allow(clippy::too_many_arguments)]
pub(crate) fn face(ui: &mut Ui, id: Id, params: &mut [f32], data: &mut Vec<u8>, live: Live, height: f32, big: bool, around: &Around) -> (bool, Vec<Act>) {
    let mut changed = false;
    let mut acts = Vec::new();
    if !inf::is_current(data) {
        *data = inf::inferno_default();
        changed = true;
    }
    let state = live.vis.map(|v| inf::Live::unpack(&v)).filter(|l| l.playing && live.playing);
    let view_id = id.with("view");
    let mut view: View = ui.ctx().data(|d| d.get_temp(view_id)).unwrap_or_default();
    let pat = pick(params, inf::PATTERN, PATTERNS);
    let len = inf::length(data, pat);
    let flames = pick(params, inf::FLAMES, inf::FLAME_MODES.len());
    let calm = flames == 1;
    let burn = params[inf::BURN] >= 0.5;
    // Following the playhead: the page it is on (of the pattern shown).
    if let (Some(s), true) = (state, view.follow) {
        if s.pattern == pat {
            view.page = s.step / PAGE;
        }
    }
    let pages = len.div_ceil(PAGE).max(1);
    view.page = view.page.min(pages.max(view.page + 1) - 1).min(STEPS / PAGE - 1);

    // Its size: the chain's height, a page of sixteen steps.
    let room = ui.available_width();
    let side = if !big { 0.0 } else if room < 900.0 { 240.0 } else { 300.0 };
    let label_w = if big { 74.0 } else { 46.0 };
    let width = if big { (room - side - 12.0).max(420.0) } else { 600.0 };
    // Narrow, its top row takes two.
    let two_rows = big && width < 780.0;
    // Filling the window its settings go beside it (the device's frame goes round both).
    let (whole, _) = ui.allocate_exact_size(vec2(width + side, height), Sense::hover());
    let rect = Rect::from_min_size(whole.min, vec2(width, height));
    // In the chain the fire goes on behind the device's other columns too (as wide as they were last frame).
    let reach: f32 = ui.ctx().data(|d| d.get_temp(id.with("reach"))).unwrap_or(whole.right());
    let whole = if big { whole } else { Rect::from_min_max(whole.min, pos2(reach.max(whole.right()), whole.bottom())) };
    let painter = ui.painter().clone();
    let p = painter.with_clip_rect(rect.expand(2.0));
    // The panel: charcoal, glowing from below with the heat.
    let heat = state.map(|s| s.heat).unwrap_or(0.0).max(if burn { 1.0 } else { 0.0 });
    let mut fire = take_fire(ui.ctx(), id);
    if burn {
        fire.flare = 1.0;
    }
    let glow = (0.25 + 0.5 * heat + 0.4 * fire.flare).min(1.0);
    // The blaze, under everything: idle it smoulders, playing it roars, Burn takes it all.
    let first_step = view.page * PAGE;
    let blazing = flames < 2 && super::inferno_fire::works();
    let blaze = {
        let grid_from = (6.0 + label_w) / whole.width().max(1.0);
        let grid_to = (rect.width() - 6.0) / whole.width().max(1.0);
        let mut cols = [0.0; super::inferno_fire::COLUMNS];
        for (c, v) in cols.iter_mut().enumerate() {
            *v = fire.glow[(first_step + c).min(STEPS - 1)];
        }
        let idle = if calm { 0.12 } else { 0.3 };
        let playing = if state.is_some() { if calm { 0.2 } else { 0.42 } } else { 0.0 };
        let level = if burn { 1.15 } else { idle + playing + heat * if calm { 0.1 } else { 0.25 } };
        super::inferno_fire::Blaze { time: fire.time as f32, heat: level, grid: [grid_from, grid_to], cols, frame: ui.ctx().cumulative_pass_nr() }
    };
    if blazing {
        super::inferno_fire::back(&painter.with_clip_rect(whole), whole, blaze);
    } else {
        rect_fill(&p, rect, 0.0, SOOT);
        gradient(&p, rect.shrink(1.0), CHARCOAL, mix(CHARCOAL, BLOOD, 0.35 * glow));
    }
    gradient(&p, Rect::from_min_max(pos2(rect.left() + 1.0, rect.bottom() - rect.height() * 0.3), rect.right_bottom() - vec2(1.0, 1.0)), alpha(EMBER, 0.0), alpha(EMBER, 0.22 * glow));
    if !big && whole.right() > rect.right() {
        rect_fill(&painter.with_clip_rect(whole), Rect::from_min_max(pos2(rect.right() + 2.0, rect.top()), whole.right_bottom()), 0.0, alpha(SCORCH, 0.8));
    }
    // A black band to read the top row on.
    let top_band = Rect::from_min_size(rect.min, vec2(rect.width(), if two_rows { 70.0 } else if big { 40.0 } else { 30.0 }));
    gradient(&p, top_band, alpha(Color32::BLACK, 0.86), alpha(Color32::BLACK, 0.55));

    // The top row: the name, the patterns, the pages, the length, and the fire.
    let head_h = if big { 30.0 } else { 22.0 };
    let head = Rect::from_min_size(rect.min + vec2(6.0, 4.0), vec2(rect.width() - 12.0, head_h));
    // The patterns' row: beside the name, or under it.
    let row_top = if two_rows { head.bottom() + 3.0 } else { head.top() + 1.0 };
    let mut x = head.left();
    let bw = if two_rows { 22.0 } else if big { 26.0 } else { 19.0 };
    for (i, letter) in inf::LETTERS.iter().enumerate() {
        let r = Rect::from_min_size(pos2(x, row_top), vec2(bw - 2.0, head_h - 2.0));
        let playing_here = state.is_some_and(|s| s.pattern == i);
        let filled = inf::pattern(data, i).iter().any(|s| s.on);
        let resp = fire_button(ui, r, id.with(("pat", i)), letter, i == pat, if playing_here { 1.0 } else { 0.0 });
        if !filled && i != pat {
            p.text(r.center_bottom() - vec2(0.0, 3.0), Align2::CENTER_CENTER, ".", small(), ASH);
        }
        if playing_here {
            p.line_segment([r.left_bottom() + vec2(2.0, 1.0), r.right_bottom() + vec2(-2.0, 1.0)], Stroke::new(2.0_f32, GOLD));
        }
        let resp = resp.on_hover_text(format!("Pattern {letter}: click to play and edit it (right-click: copy, paste, clear, into the chain)"));
        if resp.clicked() && i != pat {
            params[inf::PATTERN] = i as f32;
            changed = true;
        }
        resp.context_menu(|ui| {
            if ui.button("Copy").clicked() {
                let steps = inf::pattern(data, i).to_vec();
                let n = inf::length(data, i);
                ui.ctx().data_mut(|d| d.insert_temp(Id::new("inferno_clip"), Clip(Some((steps, n)))));
                ui.close();
            }
            let clip: Clip = ui.ctx().data(|d| d.get_temp(Id::new("inferno_clip"))).unwrap_or_default();
            if ui.add_enabled(clip.0.is_some(), egui::Button::new("Paste")).clicked() {
                if let Some((steps, n)) = clip.0 {
                    let mut arr = [Step::REST; STEPS];
                    for (a, s) in arr.iter_mut().zip(steps) {
                        *a = s;
                    }
                    inf::set_pattern(data, i, &arr);
                    inf::set_length(data, i, n);
                    changed = true;
                }
                ui.close();
            }
            if ui.button("Clear").clicked() {
                inf::set_pattern(data, i, &[Step::REST; STEPS]);
                changed = true;
                ui.close();
            }
            if ui.button("Add to the chain").clicked() {
                let (list, n) = inf::chain(data);
                let mut v = list[..n].to_vec();
                if v.len() < inf::CHAIN_MAX {
                    v.push(i as u8);
                }
                inf::set_chain(data, &v);
                params[inf::CHAIN] = 1.0;
                changed = true;
                ui.close();
            }
        });
        x += bw;
    }
    x += 8.0;
    for pg in 0..STEPS / PAGE {
        let r = Rect::from_min_size(pos2(x, row_top), vec2(bw - 2.0, head_h - 2.0));
        let inside = pg < pages;
        let resp = ui.interact(r, id.with(("page", pg)), Sense::click());
        let lit = pg == view.page;
        cut_plate(&p, r, 4.0, if lit { EMBER } else if inside { hex(0x2a0a04) } else { hex(0x060303) }, Stroke::new(1.0_f32, if inside { EMBER } else { ASH }));
        p.text(r.center(), Align2::CENTER_CENTER, format!("{}", pg + 1), small(), if lit { SOOT } else if inside { GOLD } else { ASH });
        if state.is_some_and(|s| s.pattern == pat && s.step / PAGE == pg) {
            p.circle_filled(r.right_top() + vec2(-3.0, 3.0), 2.0, GOLD);
        }
        if resp.on_hover_text("This page of sixteen steps (the pattern plays the pages up to its length)").clicked() {
            view.page = pg;
            view.follow = false;
        }
        x += bw;
    }
    x += 6.0;
    let lr = Rect::from_min_size(pos2(x, row_top), vec2(if big { 64.0 } else { 50.0 }, head_h - 2.0));
    let resp = fire_button(ui, lr, id.with("len"), &format!("Len {len}"), false, 0.0).on_hover_text("How many steps the pattern plays (1 to 64). Click: one more, right-click: one less; with Shift by four");
    if let Some(d) = clicks(&resp) {
        let by = if ui.input(|i| i.modifiers.shift) { 4 } else { 1 };
        inf::set_length(data, pat, (len as i32 + d * by).clamp(1, STEPS as i32) as usize);
        changed = true;
    }
    x = lr.right() + 6.0;
    let fr = Rect::from_min_size(pos2(x, row_top), vec2(if big { 58.0 } else { 44.0 }, head_h - 2.0));
    if fire_button(ui, fr, id.with("follow"), "Follow", view.follow, 0.0).on_hover_text("The page follows the playhead").clicked() {
        view.follow = !view.follow;
    }
    // On the right: Burn and Fill (held).
    let mut rx = head.right();
    for (pi, label, tip) in [(inf::FILL, "FILL", "Hold for a fill: rolls and rising octaves in the bar's last beat, and the steps with a Fill condition. Right-click: it stays on"), (inf::BURN, "BURN", "Hold to burn: more ratchets, accents and octave jumps while it is down. Right-click: it stays on")] {
        let w = if big { 64.0 } else { 46.0 };
        let r = Rect::from_min_size(pos2(rx - w, head.top() + 1.0), vec2(w, head_h - 2.0));
        let resp = fire_button(ui, r, id.with(label), label, params[pi] >= 0.5, heat).on_hover_text(tip);
        // Down while it is held; a right-click leaves it on (or takes it off).
        let down = resp.is_pointer_button_down_on() && ui.input(|i| i.pointer.primary_down());
        let held_id = id.with((label, "held"));
        if down != ui.ctx().data(|d| d.get_temp::<bool>(held_id)).unwrap_or(false) {
            ui.ctx().data_mut(|d| d.insert_temp(held_id, down));
            params[pi] = down as u8 as f32;
            changed = true;
        }
        if resp.secondary_clicked() {
            params[pi] = if params[pi] >= 0.5 { 0.0 } else { 1.0 };
            changed = true;
        }
        rx -= w + 4.0;
    }

    // The grid.
    let lanes: Vec<Lane> = if big { LANES.to_vec() } else { vec![LANES[view.lane.min(LANES.len() - 1)]] };
    let flags: usize = if big { FLAGS.len() } else { 5 };
    let tools_h = if big { 0.0 } else { 20.0 };
    let top = head.bottom() + 6.0 + if two_rows { head_h } else { 0.0 };
    let bottom = rect.bottom() - 6.0 - tools_h;
    let avail = (bottom - top).max(60.0);
    // Shares of the height: the steps, notes, octaves, the switches, the values.
    let weights = [3.2f32, 1.3, 1.0];
    let flag_w = 0.75;
    let lane_w = if big { 1.35 } else { 2.0 };
    let total = weights.iter().sum::<f32>() + flag_w * flags as f32 + lane_w * lanes.len() as f32;
    let unit = avail / total;
    let grid_l = rect.left() + 6.0 + label_w;
    let cw = (rect.right() - 6.0 - grid_l) / PAGE as f32;
    let cell = |i: usize, y: f32, h: f32| Rect::from_min_size(pos2(grid_l + i as f32 * cw, y), vec2(cw, h));
    let base = params[inf::BASE].round() as i32;
    let chromatic = pick(params, inf::NOTES, 2) == 1;
    let first = view.page * PAGE;
    // Evolve shows the pattern as it has become while it plays.
    let evolving = state.filter(|s| s.generation > 0 && s.pattern == pat && pick(params, inf::EVOLVE, inf::EVOLVES.len()) > 0);
    let written = inf::pattern(data, pat);
    let shown: [Step; STEPS] = match evolving {
        Some(s) => {
            // Worked out again only when it has changed.
            let key = (pat, s.generation, params[inf::EVOLVE_AMOUNT].to_bits(), params[inf::SEED].to_bits(), live.scale.len(), written);
            let cache_id = id.with("evolved");
            let kept: Option<EvolveShown> = ui.ctx().data(|d| d.get_temp(cache_id));
            match kept.filter(|k| k.0 == key) {
                Some(k) => k.1,
                None => {
                    let steps = inf::evolved(data, pat, s.generation, params[inf::EVOLVE_AMOUNT], params[inf::SEED].round().max(0.0) as u32, live.scale.len());
                    ui.ctx().data_mut(|d| d.insert_temp(cache_id, EvolveShown(key, steps)));
                    steps
                }
            }
        }
        None => written,
    };
    let label = |p: &Painter, y: f32, h: f32, s: &str| {
        let clip = p.with_clip_rect(Rect::from_min_size(pos2(rect.left() + 4.0, y), vec2(label_w - 4.0, h)));
        inked(&clip, pos2(rect.left() + 8.0, y + h * 0.5), Align2::LEFT_CENTER, s, hard_font((h * 0.55).clamp(8.0, 12.0)), hex(0xffe9c0));
    };
    let mut y = top;
    gradient(&p, Rect::from_min_max(pos2(rect.left(), top), pos2(grid_l, bottom + tools_h)), alpha(SCORCH, 0.8), alpha(SCORCH, 0.88));
    // Steps: on or off (click), muted (right-click); flames on what plays.
    let steps_h = unit * weights[0];
    let steps_row = Rect::from_min_size(pos2(grid_l, y), vec2(cw * PAGE as f32, steps_h));
    label(&p, y, steps_h, "STEP");
    let mut cells = [Rect::NOTHING; PAGE];
    for c in 0..PAGE {
        let i = first + c;
        let s = shown[i];
        let r = cell(c, y, steps_h).shrink(2.0);
        cells[c] = r;
        let inside = i < len;
        let playing_now = state.is_some_and(|st| st.pattern == pat && st.step == i);
        let resp = ui.interact(r, id.with(("step", i)), Sense::click());
        // Beats marked: every fourth step's cell a little lighter.
        let beat_mark = c % 4 == 0;
        let frame_c = if playing_now { WHITE_HOT } else if s.on && inside { hex(0xff3a08) } else { hex(0x4a120a) };
        if !inside {
            // Past its end: bricked up.
            rect_fill(&p, r, 0.0, alpha(Color32::BLACK, 0.93));
        } else if s.on && !s.mute {
            let v = s.velocity as f32 / 127.0;
            if blazing {
                // A window with the fire behind it, blowing out when it plays.
                let gain = 0.5 + 0.35 * v + if s.accent { 0.2 } else { 0.0 } + if playing_now { 0.5 } else { 0.0 } + 0.6 * fire.glow[i];
                super::inferno_fire::window(&painter.with_clip_rect(rect), r, whole, gain, blaze);
            } else {
                let hot = 0.2 + 0.22 * v + if s.accent { 0.16 } else { 0.0 } + if playing_now { 0.25 } else { 0.0 } + 0.35 * fire.glow[i];
                gradient(&p, r, heat_color(hot), heat_color(hot * 0.5));
            }
            if s.tie {
                // A tie: a beam from the window before.
                p.rect_filled(Rect::from_min_size(pos2(r.left() - 5.0, r.center().y - 4.0), vec2(10.0, 8.0)), 0.0, Color32::BLACK);
            }
        } else if s.on {
            // Muted: burnt out.
            rect_fill(&p, r, 0.0, alpha(Color32::BLACK, 0.9));
            p.line_segment([r.left_top() + vec2(4.0, 4.0), r.right_bottom() - vec2(4.0, 4.0)], Stroke::new(2.0_f32, hex(0xb3200c)));
            p.line_segment([r.right_top() + vec2(-4.0, 4.0), r.left_bottom() + vec2(4.0, -4.0)], Stroke::new(2.0_f32, hex(0xb3200c)));
        } else {
            // Off: boarded up, the fire only just through the cracks.
            rect_fill(&p, r, 0.0, alpha(Color32::BLACK, if beat_mark { 0.78 } else { 0.86 }));
            let board = Stroke::new(2.0_f32, hex(0x1e0d09));
            p.line_segment([pos2(r.left() + 1.0, r.top() + r.height() * 0.3), pos2(r.right() - 1.0, r.top() + r.height() * 0.22)], board);
            p.line_segment([pos2(r.left() + 1.0, r.top() + r.height() * 0.72), pos2(r.right() - 1.0, r.top() + r.height() * 0.8)], board);
        }
        p.rect_stroke(r, 0.0, Stroke::new(3.0_f32, Color32::BLACK), egui::StrokeKind::Outside);
        p.rect_stroke(r, 0.0, Stroke::new(if playing_now { 2.0_f32 } else { 1.0 }, frame_c), egui::StrokeKind::Inside);
        if s.on {
            let pitch = inf::pitch_of(&s, base, chromatic, &live.scale).clamp(0, 127) as u8;
            let name = if s.tie { "~".to_string() } else { note_name(pitch) };
            let ink = if s.mute { SMOKE } else { hex(0xffffff) };
            inked(&p.with_clip_rect(r), r.center() + vec2(0.0, r.height() * 0.12), Align2::CENTER_CENTER, &name, hard_font((r.height() * 0.32).clamp(9.0, 19.0)), ink);
            if s.lock {
                let lk = Rect::from_min_size(r.left_top() + vec2(3.0, 3.0), vec2(6.0, 5.0));
                p.rect_filled(lk, 1.0, GOLD);
                p.circle_stroke(lk.center_top(), 2.2, Stroke::new(1.0_f32, GOLD));
            }
            if s.ratchet > 1 {
                inked(&p, r.right_top() + vec2(-3.0, 3.0), Align2::RIGHT_TOP, &format!("x{}", s.ratchet), hard_font(10.0), hex(0xffffff));
            }
        }
        if evolving.is_some() && shown[i] != written[i] {
            p.circle_filled(r.center_top() + vec2(0.0, 3.0), 2.0, GOLD);
        }
        if s.slide && s.on && inside {
            // A slide: a curl of fire towards the next.
            let a = r.right_center() + vec2(-3.0, -r.height() * 0.2);
            p.line_segment([a, a + vec2(6.0, -3.0)], Stroke::new(2.0_f32, GOLD));
        }
        let resp = resp.on_hover_text(format!("Step {}: click on or off, right-click to mute it", i + 1));
        if resp.clicked() {
            let mut st = written[i];
            st.on = !st.on;
            if st.on && st.note == 0 && i > 0 && written[i - 1].on {
                st.note = written[i - 1].note;
            }
            inf::set_step(data, pat, i, st);
            changed = true;
        }
        if resp.secondary_clicked() {
            let mut st = written[i];
            st.mute = !st.mute;
            inf::set_step(data, pat, i, st);
            changed = true;
        }
    }
    y += steps_h;
    // Notes and octaves: click the next, right-click the one before.
    for (row, (name, h)) in [("NOTE", unit * weights[1]), ("OCT", unit * weights[2])].into_iter().enumerate() {
        label(&p, y, h, name);
        for c in 0..PAGE {
            let i = first + c;
            let s = written[i];
            let r = cell(c, y, h).shrink(1.5);
            rect_fill(&p, r, 0.0, alpha(SCORCH, if s.on { 0.9 } else { 0.72 }));
            let text = if row == 0 {
                if chromatic { format!("+{}", s.note) } else { format!("{}", s.note + 1) }
            } else {
                match s.octave {
                    0 => "0".into(),
                    o => format!("{o:+}"),
                }
            };
            inked(&p, r.center(), Align2::CENTER_CENTER, &text, hard_font((h * 0.55).clamp(8.0, 13.0)), if s.on { hex(0xffd23a) } else { SMOKE });
            let resp = ui.interact(r, id.with(("nr", row, i)), Sense::click());
            let tip = if row == 0 { "Its note: a degree of the key above the base note (Chromatic: semitones). Click: up, right-click: down" } else { "Its octave (-2 to +2). Click: up, right-click: down" };
            if let Some(d) = clicks(&resp.on_hover_text(tip)) {
                let mut st = s;
                if row == 0 {
                    st.note = (st.note as i32 + d).clamp(0, inf::NOTE_MAX as i32) as u8;
                } else {
                    st.octave = (st.octave as i32 + d).clamp(-2, 2) as i8;
                }
                st.on = true;
                inf::set_step(data, pat, i, st);
                changed = true;
            }
        }
        y += h;
    }
    // The switches.
    let fh = unit * flag_w;
    for (f, (name, tip)) in FLAGS.iter().enumerate().take(flags) {
        label(&p, y, fh, name);
        for c in 0..PAGE {
            let i = first + c;
            let s = written[i];
            let r = cell(c, y, fh).shrink2(vec2(2.0, 1.0));
            let lit = flag(&s, f);
            if lit {
                gradient(&p, r, hex(0xffe14a), hex(0xff2a08));
                p.rect_stroke(r, 0.0, Stroke::new(1.0_f32, Color32::BLACK), egui::StrokeKind::Outside);
            } else {
                rect_fill(&p, r, 0.0, alpha(SCORCH, 0.72));
            }
            let resp = ui.interact(r, id.with(("flag", f, i)), Sense::click());
            if c == 0 {
                resp.clone().on_hover_text(*tip);
            }
            if resp.clicked() || resp.secondary_clicked() {
                let mut st = s;
                flip(&mut st, f);
                inf::set_step(data, pat, i, st);
                changed = true;
            }
        }
        y += fh;
    }
    // The values: bars to drag, or steps to click.
    for (li, &lane) in lanes.iter().enumerate() {
        let h = unit * lane_w;
        let row = Rect::from_min_size(pos2(grid_l, y + 1.0), vec2(cw * PAGE as f32, h - 2.0));
        let lab = Rect::from_min_size(pos2(rect.left() + 4.0, y), vec2(label_w - 4.0, h));
        if big {
            label(&p, y, h, &lane.name().to_uppercase());
        } else {
            // In the chain one row at a time: click its name for the next.
            let resp = ui.interact(lab, id.with("lane_pick"), Sense::click());
            cut_plate(&p, lab.shrink(1.0), 4.0, if resp.hovered() { hex(0x3a0a04) } else { Color32::BLACK }, Stroke::new(1.0_f32, hex(0xb3200c)));
            p.with_clip_rect(lab).text(lab.center(), Align2::CENTER_CENTER, lane.name(), hard_font(9.0), hex(0xffd23a));
            if let Some(d) = clicks(&resp.on_hover_text("The row of values shown. Click: the next, right-click: the one before")) {
                view.lane = ((view.lane as i32 + d).rem_euclid(LANES.len() as i32)) as usize;
            }
        }
        let resp = ui.interact(row, id.with(("lane", li)), if lane.range().is_some() { Sense::click_and_drag() } else { Sense::click() });
        let resp = resp.on_hover_text(lane.tip());
        for c in 0..PAGE {
            let i = first + c;
            let s = written[i];
            let r = cell(c, y + 1.0, h - 2.0).shrink2(vec2(2.0, 1.0));
            rect_fill(&p, r, 0.0, alpha(SCORCH, 0.8));
            match lane.range() {
                Some((lo, hi)) => {
                    let v = lane.get(&s);
                    let f = (v - lo) / (hi - lo);
                    let (from, to) = if lo < 0.0 {
                        let mid = r.center().y;
                        (mid, r.bottom() - f * r.height())
                    } else {
                        (r.bottom(), r.bottom() - f * r.height())
                    };
                    let bar = Rect::from_min_max(pos2(r.left() + 2.0, from.min(to)), pos2(r.right() - 2.0, from.max(to)));
                    if s.on {
                        gradient(&p, bar, mix(hex(0xff5c00), hex(0xfff0a0), f), hex(0xc80a10));
                    } else {
                        rect_fill(&p, bar, 0.0, alpha(ASH, 0.8));
                    }
                }
                None => {
                    let t = lane.shown(&s);
                    let lit = lane.get(&s) > if lane == Lane::Ratchet { 1.0 } else { 0.0 };
                    if lit {
                        gradient(&p, r, hex(0xff5c00), hex(0xc80a10));
                    }
                    inked(&p.with_clip_rect(r), r.center(), Align2::CENTER_CENTER, &t, hard_font((h * 0.4).clamp(8.0, 12.0)), if lit { hex(0xffffff) } else { SMOKE });
                }
            }
        }
        if let Some(pt) = resp.interact_pointer_pos() {
            let c = ((pt.x - grid_l) / cw).floor();
            if (0.0..PAGE as f32).contains(&c) {
                let i = first + c as usize;
                let mut st = written[i];
                match lane.range() {
                    Some((lo, hi)) if resp.is_pointer_button_down_on() || resp.clicked() => {
                        let f = ((row.bottom() - pt.y) / row.height()).clamp(0.0, 1.0);
                        let v = lo + f * (hi - lo);
                        if (lane.get(&st) - v.round()).abs() >= 0.5 {
                            lane.set(&mut st, v);
                            inf::set_step(data, pat, i, st);
                            changed = true;
                        }
                    }
                    None => {
                        if let Some(d) = clicks(&resp) {
                            let n = lane.count() as i32;
                            let base = if lane == Lane::Ratchet { 1 } else { 0 };
                            let v = (lane.get(&st) as i32 - base + d).rem_euclid(n) + base;
                            lane.set(&mut st, v as f32);
                            inf::set_step(data, pat, i, st);
                            changed = true;
                        }
                    }
                    _ => {}
                }
            }
        }
        y += h;
    }
    let grid = Rect::from_min_max(pos2(grid_l, top), pos2(grid_l + cw * PAGE as f32, y));

    // The playhead: a burning line.
    if let Some(s) = state.filter(|s| s.pattern == pat && s.step / PAGE == view.page) {
        let x = grid_l + (s.step % PAGE) as f32 * cw + cw * 0.5;
        let col = Rect::from_min_max(pos2(x - cw * 0.5, top), pos2(x + cw * 0.5, grid.bottom()));
        gradient(&p, col, alpha(EMBER, 0.12), alpha(EMBER, 0.4));
        p.line_segment([pos2(x, steps_row.top()), pos2(x, grid.bottom())], Stroke::new(5.0_f32, alpha(EMBER, 0.55)));
        p.line_segment([pos2(x, steps_row.top()), pos2(x, grid.bottom())], Stroke::new(2.0_f32, WHITE_HOT));
    }

    // The tools under the grid (in the chain; big, they are beside it).
    if !big {
        let ty = rect.bottom() - 6.0 - tools_h + 2.0;
        rect_fill(&p, Rect::from_min_max(pos2(rect.left(), ty - 2.0), rect.right_bottom()), 0.0, alpha(Color32::BLACK, 0.82));
        changed |= tools(ui, id, params, data, live, pat, pos2(rect.left() + 6.0, ty), rect.right() - 6.0, tools_h - 2.0, &mut acts);
    }

    // The fire: new flames for what struck since the last frame.
    let dt = ui.input(|i| i.stable_dt).clamp(0.0, 0.05);
    let origin = rect.min;
    if flames < 2 {
        if let Some(s) = state {
            let shows = s.pattern == pat && s.hit_step / PAGE == view.page;
            if fire.hits == u32::MAX {
                fire.hits = s.hits;
            }
            let fresh = s.hits.wrapping_sub(fire.hits).min(4);
            if fresh > 0 && shows {
                let c = s.hit_step % PAGE;
                let r = cells[c];
                let st = shown[s.hit_step.min(STEPS - 1)];
                let h: f32 = if s.hit_accent { 1.0 } else { 0.6 } + if s.hit_jump { 0.2 } else { 0.0 };
                // From its foot, up through it and out of the top.
                let at = r.center_bottom() - origin.to_vec2() - vec2(0.0, r.height() * 0.1);
                fire.glow[s.hit_step.min(STEPS - 1)] = 1.0;
                fire.burst(pos2(at.x, at.y), r.width() * 0.9, h.min(1.0), if s.hit_accent { 16 } else { 10 } + if s.hit_jump { 8 } else { 0 }, calm);
                if st.ratchet > 1 {
                    fire.sparks(pos2(at.x, at.y), (st.ratchet as usize * 3).min(20) / if calm { 2 } else { 1 });
                }
                // A slide or a tie: a trail of fire from the step before.
                if let Some(prev) = s.prev_hit.filter(|_| s.hit_tie || shown[s.prev_hit.unwrap_or(0).min(STEPS - 1)].slide) {
                    if prev / PAGE == view.page {
                        let a = cells[prev % PAGE].center() - origin.to_vec2();
                        let b = r.center() - origin.to_vec2();
                        let n = if calm { 4 } else { 9 };
                        for k in 0..n {
                            let t = k as f32 / n as f32;
                            let q = a + (b - a) * t;
                            fire.spawn(FLAME_P, q.x, q.y - 4.0 * (t * std::f32::consts::PI).sin(), 0.0, -30.0, 0.35 + 0.2 * t, r.width() * 0.25, 0.8);
                        }
                    }
                }
            }
            fire.hits = s.hits;
            // What sounds keeps burning; the playhead smoulders; Burn sets it all alight.
            fire.carry += dt * if calm { 8.0 } else { 20.0 };
            while fire.carry >= 1.0 {
                fire.carry -= 1.0;
                if s.sounding.is_some() && s.pattern == pat && s.hit_step / PAGE == view.page {
                    let r = cells[s.hit_step % PAGE];
                    let x = r.left() - origin.x + fire.unit() * r.width();
                    fire.spawn(FLAME_P, x, r.top() - origin.y + r.height() * 0.2, 0.0, -35.0, 0.3, r.width() * 0.22, 0.5);
                }
                if s.pattern == pat && s.step / PAGE == view.page && fire.unit() < 0.5 {
                    let x = grid_l + (s.step % PAGE) as f32 * cw + cw * 0.5 - origin.x;
                    let yy = top - origin.y + fire.unit() * (grid.height());
                    let vx = (fire.unit() - 0.5) * 20.0;
                    fire.spawn(EMBER_P, x, yy, vx, -40.0, 0.8, 1.3, 0.8);
                }
                if heat > 0.5 || fire.flare > 0.1 {
                    // The whole panel: flames from anywhere, most from low down.
                    for _ in 0..if calm { 1 } else { 3 } {
                        let (ux, uy) = (fire.unit(), fire.unit());
                        let (life, h) = (0.5 + 0.4 * fire.unit(), heat.max(fire.flare));
                        fire.spawn(FLAME_P, grid_l - origin.x + ux * grid.width(), grid.bottom() - origin.y - uy * uy * grid.height(), 0.0, -90.0 - 110.0 * heat, life, cw * 0.32, h);
                    }
                }
            }
        } else {
            fire.hits = u32::MAX;
            if burn {
                fire.carry += dt * 20.0;
                while fire.carry >= 1.0 {
                    fire.carry -= 1.0;
                    let x = grid_l - origin.x + fire.unit() * grid.width();
                    fire.spawn(FLAME_P, x, grid.bottom() - origin.y, 0.0, -80.0, 0.6, cw * 0.35, 1.0);
                }
            }
        }
        // Embers and sparks going up from the blaze, always; more the harder it burns.
        fire.ember_carry += dt * (if calm { 10.0 } else { 26.0 } + 90.0 * blaze.heat * if calm { 0.3 } else { 1.0 });
        while fire.ember_carry >= 1.0 {
            fire.ember_carry -= 1.0;
            let (ux, uy, up) = (fire.unit(), fire.unit(), fire.unit());
            let (x, y) = (ux * whole.width(), whole.height() - uy * uy * whole.height() * 0.5);
            let vx = (fire.unit() - 0.5) * 70.0;
            if up < 0.25 {
                let boost = fire.unit();
                fire.spawn(SPARK_P, x, y, vx * 2.0, -170.0 - 260.0 * boost, 0.45 + 0.5 * uy, 1.0, 1.0);
            } else {
                fire.spawn(EMBER_P, x, y, vx, -60.0 - 120.0 * up, 0.9 + 1.2 * uy, 1.0 + 1.5 * up, 0.8);
            }
        }
        fire.update(dt);
        fire.draw(&painter.with_clip_rect(whole), origin, blazing);
        ui.ctx().request_repaint();
    }
    put_fire(ui.ctx(), id, fire);

    if big {
        // Its settings beside the grid.
        let side_rect = Rect::from_min_size(pos2(rect.right() + 8.0, rect.top()), vec2(side - 8.0, height));
        rect_fill(&painter, side_rect, 0.0, alpha(SCORCH, 0.84));
        let mut child = ui.new_child(UiBuilder::new().max_rect(side_rect).layout(egui::Layout::top_down(egui::Align::Min)));
        egui::ScrollArea::vertical().id_salt(id.with("side")).max_height(height).show(&mut child, |ui| {
            ui.set_width(side - 20.0);
            changed |= settings(ui, id, params, data, live, pat, around, &mut acts);
        });
    }
    if blazing && !calm {
        super::inferno_fire::front(&ui.painter().with_clip_rect(whole), whole, 0.6, blaze);
    }
    ui.ctx().data_mut(|d| d.insert_temp(view_id, view));
    (changed, acts)
}

/// The pattern tools: Ignite and the genre, Mutate, Rotate, Reverse,
/// Invert, Humanize, Clear.
#[allow(clippy::too_many_arguments)]
fn tools(ui: &mut Ui, id: Id, params: &mut [f32], data: &mut Vec<u8>, live: Live, pat: usize, at: Pos2, right: f32, h: f32, acts: &mut Vec<Act>) -> bool {
    let mut changed = false;
    let specs = inf::SPECS;
    let x = at.x;
    let gr = Rect::from_min_size(pos2(x, at.y), vec2(110.0, h));
    if dropdown(ui, gr, id.with("genre"), &specs[inf::GENRE], &mut params[inf::GENRE]) {
        changed = true;
        apply_feel(params);
    }
    let ir = Rect::from_min_size(pos2(gr.right() + 4.0, at.y), vec2(62.0, h));
    if fire_button(ui, ir, id.with("ignite"), "IGNITE", false, 0.6).on_hover_text("Write a new line in the genre (with its Density, Complexity, Melody; on the band's chords when it is in the band)").clicked() {
        acts.push(Act::Ignite { pattern: pat, part: None });
    }
    changed |= pattern_tools(ui, id, params, data, live, pat, pos2(ir.right() + 4.0, at.y), right, h);
    changed
}

/// Mutate, Rotate, Reverse, Invert, Humanize and Clear, in a row.
#[allow(clippy::too_many_arguments)]
fn pattern_tools(ui: &mut Ui, id: Id, params: &[f32], data: &mut Vec<u8>, live: Live, pat: usize, at: Pos2, right: f32, h: f32) -> bool {
    let mut changed = false;
    let mut x = at.x;
    let len = inf::length(data, pat);
    let mut steps = inf::pattern(data, pat);
    let n = live.scale.len();
    let buttons: [(&str, &str); 7] = [
        ("Mutate", "A few small changes (not to the locked steps)"),
        ("<", "Everything a step earlier"),
        (">", "Everything a step later"),
        ("Rev", "Backwards"),
        ("Inv", "Upside down, round the root"),
        ("Hum", "A little looser: velocities and timing"),
        ("Clear", "Every step off"),
    ];
    // Mutate and Clear have longer names.
    let share: [f32; 7] = [2.0, 0.8, 0.8, 1.0, 1.0, 1.0, 1.4];
    let unit = ((right - x - 3.0 * buttons.len() as f32) / share.iter().sum::<f32>()).clamp(20.0, 34.0);
    for (k, (label, tip)) in buttons.iter().enumerate() {
        let bw = unit * share[k];
        let r = Rect::from_min_size(pos2(x, at.y), vec2(bw, h));
        if fire_button(ui, r, id.with(("tool", k)), label, false, 0.0).on_hover_text(*tip).clicked() {
            match k {
                0 => inf::mutate(&mut steps, len, params[inf::EVOLVE_AMOUNT].max(0.15), seed_now(), n),
                1 => inf::rotate(&mut steps, len, false),
                2 => inf::rotate(&mut steps, len, true),
                3 => inf::reverse(&mut steps, len),
                4 => inf::invert(&mut steps, len, n),
                5 => inf::humanize(&mut steps, len, 0.6, seed_now()),
                _ => steps = [Step::REST; STEPS],
            }
            inf::set_pattern(data, pat, &steps);
            changed = true;
        }
        x += bw + 3.0;
    }
    changed
}

/// Picking a genre sets its feel too: division, groove, swing, glide, base note.
fn apply_feel(params: &mut [f32]) {
    let f = inf::feel(pick(params, inf::GENRE, inf::GENRES.len()));
    params[inf::DIVISION] = f.division as f32;
    params[inf::GROOVE] = f.groove as f32;
    params[inf::SWING] = f.swing;
    params[inf::GLIDE] = f.glide as f32;
    params[inf::BASE] = f.base;
}

/// The settings beside the big grid.
#[allow(clippy::too_many_arguments)]
fn settings(ui: &mut Ui, id: Id, params: &mut [f32], data: &mut Vec<u8>, live: Live, pat: usize, around: &Around, acts: &mut Vec<Act>) -> bool {
    let mut changed = false;
    let specs = inf::SPECS;
    let w = ui.available_width();
    let heading = |ui: &mut Ui, s: &str| {
        ui.add_space(4.0);
        let (r, _) = ui.allocate_exact_size(vec2(w, 16.0), Sense::hover());
        flame_logo(ui.painter(), r.left_center(), s, 17.0, 0.0);
        ui.painter().line_segment([r.left_bottom(), r.right_bottom()], Stroke::new(2.0_f32, hex(0xeb0d08)));
    };
    // A row of dropdowns, then a row of faders.
    let drops = |ui: &mut Ui, list: &[usize], params: &mut [f32]| -> bool {
        let mut c = false;
        let n = list.len().max(1) as f32;
        let (r, _) = ui.allocate_exact_size(vec2(w, 34.0), Sense::hover());
        for (k, &i) in list.iter().enumerate() {
            let cw = w / n;
            let cell = Rect::from_min_size(pos2(r.left() + k as f32 * cw, r.top()), vec2(cw - 4.0, 34.0));
            ui.painter().text(cell.left_top(), Align2::LEFT_TOP, specs[i].name, FontId::proportional(10.0), hex(0xe0a070));
            c |= dropdown(ui, Rect::from_min_max(cell.left_top() + vec2(0.0, 13.0), cell.right_bottom()), id.with(("dd", i)), &specs[i], &mut params[i]);
        }
        c
    };
    let faders = |ui: &mut Ui, list: &[usize], params: &mut [f32]| -> bool {
        let mut c = false;
        let n = list.len().max(1) as f32;
        let (r, _) = ui.allocate_exact_size(vec2(w, 78.0), Sense::hover());
        for (k, &i) in list.iter().enumerate() {
            let cw = (w / n).min(64.0);
            let cell = Rect::from_min_size(pos2(r.left() + k as f32 * cw, r.top()), vec2(cw - 4.0, 78.0));
            c |= param_slider(ui, cell, id.with(("fd", i)), &specs[i], &mut params[i], EMBER).0;
        }
        c
    };
    heading(ui, "IGNITE");
    let g = params[inf::GENRE];
    if drops(ui, &[inf::GENRE, inf::KICK], params) {
        changed = true;
        if params[inf::GENRE] != g {
            apply_feel(params);
        }
    }
    changed |= faders(ui, &[inf::DENSITY, inf::COMPLEXITY, inf::MELODY], params);
    let (r, _) = ui.allocate_exact_size(vec2(w, 26.0), Sense::hover());
    let ir = Rect::from_min_size(r.min, vec2(w * 0.4, 24.0));
    if fire_button(ui, ir, id.with("ig_big"), "IGNITE", false, 0.7).on_hover_text("Write a new line into this pattern").clicked() {
        acts.push(Act::Ignite { pattern: pat, part: None });
    }
    let sr = Rect::from_min_size(pos2(ir.right() + 6.0, r.top()), vec2(w * 0.3, 24.0));
    let seed = params[inf::SEED].round() as i32;
    let resp = fire_button(ui, sr, id.with("seed"), &format!("Seed {seed}"), false, 0.0).on_hover_text("Ignite's seed: the same seed writes the same line. Click: the next, right-click: the one before");
    if let Some(d) = clicks(&resp) {
        params[inf::SEED] = (seed + d).clamp(0, 9999) as f32;
        changed = true;
    }
    let dr = Rect::from_min_size(pos2(sr.right() + 6.0, r.top()), vec2(w - (sr.right() - r.left()) - 6.0, 24.0));
    if fire_button(ui, dr, id.with("dice"), "Dice", false, 0.0).on_hover_text("A new seed, and Ignite").clicked() {
        params[inf::SEED] = (seed_now() % 10000) as f32;
        changed = true;
        acts.push(Act::Ignite { pattern: pat, part: None });
    }
    let (r, _) = ui.allocate_exact_size(vec2(w, 22.0), Sense::hover());
    changed |= pattern_tools(ui, id.with("t2"), params, data, live, pat, r.min, r.right(), 20.0);
    let (pr, _) = ui.allocate_exact_size(vec2(w, 22.0), Sense::hover());
    let (inst, preset) = inf::feel(pick(params, inf::GENRE, inf::GENRES.len())).pair;
    ui.painter().text(pr.left_center(), Align2::LEFT_CENTER, format!("Pairs with {inst} · {preset}"), small(), hex(0xe0a070));
    let lr = Rect::from_min_size(pos2(pr.right() - 50.0, pr.top()), vec2(50.0, 20.0));
    if fire_button(ui, lr, id.with("pair"), "Load", false, 0.0).on_hover_text(format!("Put {inst} with its preset {preset} on the track (in place of {})", if around.instrument.is_empty() { "nothing" } else { &around.instrument })).clicked() {
        acts.push(Act::LoadPair);
    }

    heading(ui, "CLOCK");
    changed |= drops(ui, &[inf::DIVISION, inf::DIRECTION, inf::GROOVE], params);
    changed |= faders(ui, &[inf::SWING, inf::GATE, inf::ACCENT, inf::GLIDE_TIME], params);
    heading(ui, "NOTES & GLIDE");
    changed |= drops(ui, &[inf::NOTES, inf::GLIDE], params);
    let (r, _) = ui.allocate_exact_size(vec2(w, 24.0), Sense::hover());
    let base = params[inf::BASE].round() as i32;
    let br = Rect::from_min_size(r.min, vec2(w * 0.5, 22.0));
    let resp = fire_button(ui, br, id.with("base"), &format!("Base {}", note_name(base.clamp(0, 127) as u8)), false, 0.0).on_hover_text("The base note: the first degree of its notes. Click: a semitone up, right-click: down; with Shift an octave");
    if let Some(d) = clicks(&resp) {
        let by = if ui.input(|i| i.modifiers.shift) { 12 } else { 1 };
        params[inf::BASE] = (base + d * by).clamp(24, 72) as f32;
        changed = true;
    }
    heading(ui, "WOB");
    changed |= drops(ui, &[inf::WOB_OUT, inf::WOB_SHAPE], params);
    changed |= faders(ui, &[inf::WOB_DEPTH], params);
    {
        let (r, _) = ui.allocate_exact_size(vec2(w, 34.0), Sense::hover());
        ui.painter().text(r.left_top(), Align2::LEFT_TOP, "Wob to the instrument's", FontId::proportional(10.0), hex(0xe0a070));
        let cur = around.patched.and_then(|k| around.knobs.iter().find(|(i, _)| *i == k)).map(|(_, n)| n.as_str()).unwrap_or("Not patched");
        let mut child = ui.new_child(UiBuilder::new().max_rect(Rect::from_min_max(r.left_top() + vec2(0.0, 13.0), r.right_bottom())));
        egui::ComboBox::from_id_salt(id.with("wob_knob")).width(w - 8.0).height(360.0).selected_text(RichText::new(cur).small().color(GOLD)).show_ui(&mut child, |ui| {
            if ui.selectable_label(around.patched.is_none(), "Not patched").clicked() {
                acts.push(Act::PatchWob(None));
            }
            for (k, name) in &around.knobs {
                if ui.selectable_label(around.patched == Some(*k), name).clicked() {
                    acts.push(Act::PatchWob(Some(*k)));
                }
            }
        });
    }
    heading(ui, "EVOLVE");
    changed |= drops(ui, &[inf::EVOLVE], params);
    changed |= faders(ui, &[inf::EVOLVE_AMOUNT], params);

    heading(ui, "LANES");
    for (lane, name) in inf::LANES.iter().enumerate() {
        let (r, _) = ui.allocate_exact_size(vec2(w, 20.0), Sense::hover());
        ui.painter().text(r.left_center(), Align2::LEFT_CENTER, *name, small(), hex(0xe0a070));
        let n = inf::lane_length(data, pat, lane);
        let br = Rect::from_min_size(pos2(r.left() + 70.0, r.top()), vec2(80.0, 18.0));
        let own = n != inf::length(data, pat) || lane == 0;
        let resp = fire_button(ui, br, id.with(("lanelen", lane)), &format!("{n} steps"), own && lane > 0, 0.0).on_hover_text("How long this lane runs before it starts again (its own length: a polymeter against the notes). Click: one more, right-click: one less");
        if let Some(d) = clicks(&resp) {
            inf::set_lane_length(data, pat, lane, (n as i32 + d).clamp(1, STEPS as i32) as usize);
            changed = true;
        }
    }

    heading(ui, "CHAIN");
    let (list, n) = inf::chain(data);
    let (r, _) = ui.allocate_exact_size(vec2(w, 24.0), Sense::hover());
    let on = params[inf::CHAIN] >= 0.5;
    if fire_button(ui, Rect::from_min_size(r.min, vec2(52.0, 22.0)), id.with("chain_on"), "Chain", on, 0.0).on_hover_text("Play the chain: each pattern once, one after another").clicked() {
        params[inf::CHAIN] = if on { 0.0 } else { 1.0 };
        changed = true;
    }
    let mut v = list[..n].to_vec();
    let mut cx = r.left() + 58.0;
    let mut edit = false;
    for k in 0..n {
        let cr = Rect::from_min_size(pos2(cx, r.top()), vec2(20.0, 22.0));
        let resp = fire_button(ui, cr, id.with(("chain", k)), inf::LETTERS[v[k] as usize], false, 0.0).on_hover_text("Click: the next pattern, right-click: the one before; middle-click: out of the chain");
        if let Some(d) = clicks(&resp) {
            v[k] = ((v[k] as i32 + d).rem_euclid(PATTERNS as i32)) as u8;
            edit = true;
        }
        if resp.middle_clicked() {
            v.remove(k);
            edit = true;
            break;
        }
        cx += 22.0;
    }
    if n < inf::CHAIN_MAX && fire_button(ui, Rect::from_min_size(pos2(cx, r.top()), vec2(20.0, 22.0)), id.with("chain_add"), "+", false, 0.0).on_hover_text("Add the pattern shown").clicked() {
        v.push(pat as u8);
        edit = true;
    }
    if edit {
        inf::set_chain(data, &v);
        changed = true;
    }

    // The band: only its switch while it is not in it.
    heading(ui, "BAND");
    let in_band = params[inf::BAND] >= 0.5;
    let (r, _) = ui.allocate_exact_size(vec2(w, 26.0), Sense::hover());
    let sw = Rect::from_min_size(r.min, vec2(w * 0.45, 24.0));
    if fire_button(ui, sw, id.with("band"), if in_band { "In the band" } else { "Band off" }, in_band, 0.0).on_hover_text("In the band: its notes follow the band's chords, and it plays the song's parts as they say (a pattern per part, energy, dynamics, fill, build, break). Off: it plays as written").clicked() {
        params[inf::BAND] = if in_band { 0.0 } else { 1.0 };
        changed = true;
    }
    if in_band {
        let pr = Rect::from_min_size(pos2(sw.right() + 6.0, r.top()), vec2(w - sw.width() - 6.0, 24.0));
        if fire_button(ui, pr, id.with("band_win"), "Parts...", false, 0.0).on_hover_text("The Band window: the song's parts, how the band plays each, the form and who plays where").clicked() {
            acts.push(Act::BandWindow);
        }
        changed |= drops(ui, &[inf::BAND + 1], params);
        if let Some(chord) = &around.chord {
            let (r, _) = ui.allocate_exact_size(vec2(w, 18.0), Sense::hover());
            ui.painter().text(r.left_center(), Align2::LEFT_CENTER, chord, small(), GOLD);
        }
    }
    if !around.parts.is_empty() {
        heading(ui, "SONG PARTS");
        for (part, name) in around.parts.iter().enumerate() {
            let (r, _) = ui.allocate_exact_size(vec2(w, 22.0), Sense::hover());
            ui.painter().with_clip_rect(r).text(r.left_center(), Align2::LEFT_CENTER, name, small(), hex(0xe0a070));
            let cur = inf::part_pattern(data, part);
            let label = cur.map(|p| inf::LETTERS[p]).unwrap_or("-");
            let pr = Rect::from_min_size(pos2(r.left() + 110.0, r.top()), vec2(40.0, 20.0));
            let resp = fire_button(ui, pr, id.with(("part", part)), label, cur.is_some(), 0.0).on_hover_text("The pattern this part plays (-: the one picked). Click: the next, right-click: the one before");
            if let Some(d) = clicks(&resp) {
                let k = cur.map(|p| p as i32 + 1).unwrap_or(0);
                let next = (k + d).rem_euclid(PATTERNS as i32 + 1);
                inf::set_part_pattern(data, part, (next > 0).then(|| next as usize - 1));
                changed = true;
            }
            let ir = Rect::from_min_size(pos2(pr.right() + 6.0, r.top()), vec2(70.0, 20.0));
            if fire_button(ui, ir, id.with(("part_ig", part)), "Ignite", false, 0.5).on_hover_text("Write a new line for this part, on its chords, into its pattern").clicked() {
                acts.push(Act::Ignite { pattern: cur.unwrap_or(pat), part: Some(part) });
            }
        }
    }
    heading(ui, "FLAMES");
    changed |= drops(ui, &[inf::FLAMES], params);
    changed
}

/// Ignite without the app (the band's chords and the kick unknown).
pub(crate) fn ignite_plain(params: &[f32], data: &mut Vec<u8>, pattern: usize, scale: &krabton_engine::Scale) {
    let g = inf::Ignite::from_params(params, scale);
    let (steps, len) = inf::ignite(&g);
    inf::set_pattern(data, pattern, &steps);
    inf::set_length(data, pattern, len);
}

// ---------------------------------------------------------------- the app's side

impl App {
    /// What the face knows of the track it is on.
    pub(crate) fn inferno_around(&self, track: usize, params: &[f32]) -> Around {
        let Some(tr) = self.project.tracks.get(track) else { return Around::default() };
        let in_band = params.get(inf::BAND).is_some_and(|v| *v >= 0.5);
        let parts = if in_band { self.project.band.parts.iter().map(|p| p.name.clone()).collect() } else { Vec::new() };
        let mut knobs = Vec::new();
        if let Some(specs) = tr.instrument.as_ref().and_then(krabton_engine::modulation::instrument_specs) {
            for (i, s) in specs.iter().enumerate() {
                if matches!(s.kind, krabton_engine::params::ParamKind::Linear | krabton_engine::params::ParamKind::Log) && s.shown() {
                    knobs.push((i as u16, format!("{} · {}", s.group, s.name)));
                }
            }
        }
        let patched = self.inferno_wob_route(track).map(|(_, p)| p);
        let st = self.band_status();
        let chord = (in_band && st.on && st.degree > 0).then(|| {
            let part = self.project.band.parts.get(st.part as usize).filter(|_| st.parts);
            let key = part.map(|p| p.play.key(self.project.scale)).unwrap_or(self.project.scale);
            let name = krabton_engine::classic_seq::chord_name(&key, st.degree, st.quality);
            match part {
                Some(p) => format!("{} · {name}", p.name),
                None => format!("Chord {name}"),
            }
        });
        Around { parts, knobs, patched, instrument: tr.instrument.as_ref().map(|i| i.kind_name().to_string()).unwrap_or_default(), chord }
    }

    /// The track's Mod Wheel modulator and the instrument knob it is on (the wob's cable).
    fn inferno_wob_route(&self, track: usize) -> Option<(u64, u16)> {
        use krabton_engine::modulation::{ModKind, ModOwner, ModTarget};
        let id = self.project.tracks.get(track)?.id;
        let m = self.project.modulators.iter().find(|m| m.kind == ModKind::ModWheel && m.track == Some(id))?;
        self.project.mod_routes.iter().filter(|r| r.source == m.id).find_map(|r| match r.target {
            ModTarget::Param { owner: ModOwner::Instrument(t), param } if t == id => Some((r.id, param)),
            _ => None,
        })
    }

    /// Where the kick is on the first drum track (a bit per sixteenth of its first bar).
    fn inferno_kick(&self, except: usize) -> Option<u16> {
        for (ti, tr) in self.project.tracks.iter().enumerate() {
            let kit = tr.instrument.as_ref().map(|i| i.drum_notes()).unwrap_or(0);
            if ti == except || kit == 0 {
                continue;
            }
            let mut mask = 0u16;
            let mut add = |start: f64, pitch: u8| {
                if matches!(pitch, 35 | 36) && (0.0..4.0).contains(&start) {
                    mask |= 1 << ((start * 4.0).round() as usize).min(15);
                }
            };
            if let Some(dev) = tr.midi_devices.iter().find(|d| d.enabled && d.kind.is_sequencer()) {
                for n in krabton_engine::midi_fx::render_notes(dev.kind, &dev.params, &dev.data, 4.0, self.project.bpm, self.project.scale, self.project.scale_mode, kit) {
                    add(n.start, n.pitch);
                }
            } else if let Some(krabton_engine::project::ClipContent::Midi { notes }) = tr.slots.iter().flatten().next().map(|c| &c.content) {
                for n in notes {
                    add(n.start, n.pitch);
                }
            }
            if mask != 0 {
                return Some(mask);
            }
        }
        None
    }

    /// Does what an Inferno's face asks.
    pub(crate) fn inferno_act(&mut self, track: usize, dev_id: u64, act: Act) {
        use krabton_engine::modulation::{ModKind, ModOwner, ModTarget};
        let Some(tr) = self.project.tracks.get(track) else { return };
        let Some(d) = tr.midi_devices.iter().position(|d| d.id == dev_id) else { return };
        let tid = tr.id;
        match act {
            Act::BandWindow => self.band.window = true,
            Act::Ignite { pattern, part } => {
                let dev = &self.project.tracks[track].midi_devices[d];
                let params = dev.params.clone();
                let mut g = inf::Ignite::from_params(&params, &self.project.scale);
                if g.kick_mode > 0 {
                    g.kick = self.inferno_kick(track);
                }
                // In the band: the chords of the part (or the band's one progression).
                if params.get(inf::BAND).is_some_and(|v| *v >= 0.5) {
                    let rate = inf::step_beats(&params);
                    let shared = match part {
                        Some(i) => self.project.band.parts.get(i).map(|p| p.part()),
                        None => self.band_first_part(),
                    };
                    g.chords = shared.and_then(|s| inf::chords_on_steps(&s, rate));
                }
                if part.is_some() {
                    g.seed = g.seed.wrapping_add(part.unwrap_or(0) as u32 * 101);
                }
                let (steps, len) = inf::ignite(&g);
                let dev = &mut self.project.tracks[track].midi_devices[d];
                inf::set_pattern(&mut dev.data, pattern, &steps);
                inf::set_length(&mut dev.data, pattern, len);
                if let Some(i) = part {
                    inf::set_part_pattern(&mut dev.data, i, Some(pattern));
                }
                // A new seed for the next time.
                dev.params[inf::SEED] = ((params[inf::SEED].round() as u32 + 1) % 10000) as f32;
            }
            Act::PatchWob(param) => {
                if let Some((route, _)) = self.inferno_wob_route(track) {
                    self.project.mod_routes.retain(|r| r.id != route);
                }
                if let Some(param) = param {
                    let mid = match self.project.modulators.iter().find(|m| m.kind == ModKind::ModWheel && m.track == Some(tid)) {
                        Some(m) => m.id,
                        None => {
                            if self.project.modulators.len() >= krabton_engine::modulation::MAX_MODS {
                                self.toast(format!("At most {} modulators: take one away in the Mod tab", krabton_engine::modulation::MAX_MODS));
                                return;
                            }
                            let mut m = self.project.make_modulator(ModKind::ModWheel);
                            m.track = Some(tid);
                            m.name = format!("Inferno Wob ({})", self.project.tracks[track].name);
                            let id = m.id;
                            self.project.modulators.push(m);
                            id
                        }
                    };
                    let target = ModTarget::Param { owner: ModOwner::Instrument(tid), param };
                    self.project.add_route(mid, target, 0.6);
                    self.toast(format!("Wob -> {}", self.project.target_label(target).1));
                }
            }
            Act::LoadPair => {
                let genre = pick(&self.project.tracks[track].midi_devices[d].params, inf::GENRE, inf::GENRES.len());
                let (inst, preset) = inf::feel(genre).pair;
                let instrument = match inst {
                    "Bassline" => krabton_engine::Instrument::bassline(preset),
                    "Analog" => krabton_engine::Instrument::analog(preset),
                    "Crimson" => krabton_engine::Instrument::crimson(preset),
                    "Silicon" => krabton_engine::Instrument::silicon(preset),
                    "Mono" => krabton_engine::Instrument::mono101(preset),
                    _ => krabton_engine::Instrument::synth(preset),
                };
                self.project.tracks[track].instrument = Some(instrument);
                self.toast(format!("{inst} · {preset} on the track"));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_flames_stay_in_their_pool() {
        let mut f = Fire::default();
        for k in 0..400 {
            f.burst(pos2(10.0 + k as f32, 50.0), 20.0, 1.0, 12, false);
            f.sparks(pos2(10.0, 50.0), 20);
            f.update(1.0 / 60.0);
            assert!(f.alive() <= POOL);
        }
        assert!(f.alive() > POOL / 2, "it burns");
        // Left alone it burns out.
        for _ in 0..300 {
            f.update(1.0 / 60.0);
        }
        assert_eq!(f.alive(), 0);
        assert_eq!(f.p.len(), POOL);
    }

    #[test]
    fn a_tongue_of_flame_is_seven_points_and_its_tip_fades() {
        let mut m = Mesh::default();
        tongue(&mut m, pos2(10.0, 50.0), 8.0, 30.0, 2.0, GOLD, 1.0);
        assert_eq!(m.vertices.len(), 7);
        assert_eq!(m.indices.len(), 18);
        assert!(m.indices.iter().all(|i| (*i as usize) < 7));
        assert_eq!(m.vertices[4].color.a(), 0, "its tip");
        assert!(m.vertices[4].pos.y < 25.0, "it rises");
    }

    #[test]
    fn heat_runs_from_coal_to_white() {
        assert_eq!(heat_color(0.0), CINDER);
        assert_eq!(heat_color(1.0), WHITE_HOT);
        let lum = |c: Color32| c.r() as u32 + c.g() as u32 + c.b() as u32;
        assert!((0..10).all(|i| lum(heat_color(i as f32 / 10.0)) <= lum(heat_color((i + 1) as f32 / 10.0))));
    }

    /// Runs the face a few frames; positions relative to its top left.
    fn drive(params: &mut Vec<f32>, data: &mut Vec<u8>, big: bool, frames: &[Vec<(Pos2, Option<(bool, egui::PointerButton)>)>], vis: Option<[u64; 4]>) -> Vec<Act> {
        use egui::{Event, Modifiers, RawInput};
        let ctx = egui::Context::default();
        let mut origin = Pos2::ZERO;
        let mut acts = Vec::new();
        for (i, evs) in std::iter::once(&Vec::new()).chain(frames.iter()).enumerate() {
            let mut input = RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1400.0, 900.0))), ..Default::default() };
            for &(p, press) in evs {
                let pos = origin + p.to_vec2();
                input.events.push(Event::PointerMoved(pos));
                if let Some((pressed, button)) = press {
                    input.events.push(Event::PointerButton { pos, button, pressed, modifiers: Modifiers::NONE });
                }
            }
            let _ = ctx.run(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    if i == 0 {
                        origin = ui.cursor().min;
                    }
                    let live = Live { vis, playing: vis.is_some(), scale: krabton_engine::Scale::default(), kit: 0, bpm: 120.0 };
                    let (_, a) = face(ui, Id::new("f"), params, data, live, if big { 600.0 } else { 210.0 }, big, &Around::default());
                    acts.extend(a);
                });
            });
        }
        acts
    }

    #[test]
    fn a_click_lights_a_step_and_a_right_click_steps_a_note_back() {
        use egui::PointerButton::{Primary, Secondary};
        let mut p = krabton_engine::params::defaults(inf::SPECS);
        let mut d = inf::empty();
        // The first step cell sits just right of the labels, under the top row.
        let at = pos2(46.0 + 6.0 + 10.0, 40.0);
        let click = |b| vec![vec![(at, Some((true, b)))], vec![(at, Some((false, b)))]];
        drive(&mut p, &mut d, false, &click(Primary), None);
        assert!(inf::step(&d, 0, 0).on);
        // Its note row: click up twice, right-click down once.
        let mut s = inf::step(&d, 0, 0);
        s.note = 3;
        inf::set_step(&mut d, 0, 0, s);
        let note_at = pos2(at.x, 80.0);
        let frames: Vec<_> = [Secondary].iter().flat_map(|b| click_at(note_at, *b)).collect();
        drive(&mut p, &mut d, false, &frames, None);
        assert_eq!(inf::step(&d, 0, 0).note, 2, "right-click goes back");
    }

    fn click_at(at: Pos2, b: egui::PointerButton) -> Vec<Vec<(Pos2, Option<(bool, egui::PointerButton)>)>> {
        vec![vec![(at, Some((true, b)))], vec![(at, Some((false, b)))]]
    }

    #[test]
    fn it_draws_playing_and_big_and_burns_while_burn_is_held() {
        let mut p = krabton_engine::params::defaults(inf::SPECS);
        let mut d = inf::inferno_default();
        let live = |hits: u64| [0x40 | 1 << 63 | 200 << 24, 3 | (hits << 32) | 0xff << 16, 500, 0];
        let frames: Vec<Vec<(Pos2, Option<(bool, egui::PointerButton)>)>> = (0..6).map(|_| vec![(pos2(5.0, 5.0), None)]).collect();
        drive(&mut p, &mut d, true, &frames, Some(live(4)));
        drive(&mut p, &mut d, false, &frames, Some(live(9)));
        // Burn: down while the button is held.
        let burn_at = pos2(594.0 - 46.0 - 4.0 - 23.0, 14.0);
        let held = vec![vec![(burn_at, Some((true, egui::PointerButton::Primary)))], vec![(burn_at, None)]];
        drive(&mut p, &mut d, false, &held, None);
        assert_eq!(p[inf::BURN], 1.0, "held down");
        let mut p = krabton_engine::params::defaults(inf::SPECS);
        let released = vec![vec![(burn_at, Some((true, egui::PointerButton::Primary)))], vec![(burn_at, Some((false, egui::PointerButton::Primary)))], vec![(burn_at, None)]];
        drive(&mut p, &mut d, false, &released, None);
        assert_eq!(p[inf::BURN], 0.0, "let go");
        // A right-click leaves it on, and something else can hold it (a script).
        let right = vec![vec![(burn_at, Some((true, egui::PointerButton::Secondary)))], vec![(burn_at, Some((false, egui::PointerButton::Secondary)))], vec![(burn_at, None)]];
        drive(&mut p, &mut d, false, &right, None);
        assert_eq!(p[inf::BURN], 1.0, "latched");
        drive(&mut p, &mut d, false, &frames, None);
        assert_eq!(p[inf::BURN], 1.0, "stays");
    }
}

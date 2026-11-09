//! Drawing the Video on the graphics card. Once a frame, one shader puts
//! every effect on the picture into the *stage* (a picture of its own, 1280
//! by 720, which also keeps the last frame for Trails and Echo); every
//! place the Video shows (its window, the screen) then draws the stage. It
//! runs in egui's OpenGL painter, through a paint callback. While the Video
//! is recorded, the stage is read back after it is drawn. The Visuals (see
//! `visuals`) are drawn first, into a picture of their own the effects
//! take with the film's. An export with Visuals in it has its frames drawn
//! here too (`GpuFrames`), at its own size.

use eframe::egui_glow;
use eframe::glow::{self, HasContext};
use super::visuals::{self, Scope, SCOPE_W};
use krabton_engine::milk::{self, MILK_N};
use krabton_engine::video::{self as kv, Effect, FxSetting};

/// Milk's place among the Visuals' scenes.
pub const MILK_SCENE: usize = 10;
use std::sync::{Arc, Condvar, Mutex};

pub const STAGE_W: i32 = 1280;
pub const STAGE_H: i32 = 720;

/// How a picture sits on the stage (see `video::ClipFrame`): filling it or
/// fitting in it, how see-through, how big, where, turned, and cropped.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Framing {
    pub fill: bool,
    pub opacity: f32,
    pub zoom: f32,
    pub pos: [f32; 2],
    /// Turns (1: all the way round).
    pub turn: f32,
    /// Cut off the left, top, right and bottom (fractions of the picture).
    pub crop: [f32; 4],
}

impl Default for Framing {
    fn default() -> Self {
        Self { fill: true, opacity: 1.0, zoom: 1.0, pos: [0.0; 2], turn: 0.0, crop: [0.0; 4] }
    }
}

/// Everything the effect shader needs for one frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Params {
    /// The picture's size in pixels (0: no picture), and how it sits.
    pub video_size: [f32; 2],
    pub frame: Framing,
    /// How much of each effect (0..1, by `Effect::index`; Strobe is its light now).
    pub fx: [f32; Effect::COUNT],
    /// Each effect's own settings (see `Effect::params`).
    pub p: [[f32; 4]; Effect::COUNT],
    /// Turned this far (turns), and shaken to here (fractions of the stage).
    pub spin: f32,
    pub shake: [f32; 2],
    /// Change when the glitch's strips and the grain move.
    pub seed: f32,
    pub grain: f32,
    /// The colours of Tint, Flash, Strobe and Edges (0..1).
    pub tint: [f32; 3],
    pub flash: [f32; 3],
    pub strobe: [f32; 3],
    pub edges: [f32; 3],
    /// The clip it is changing from: how (0 not changing, 1 Fade, 2 Flash,
    /// 3 Glitch), how far (0: all of it, 1: all of the new one), its
    /// picture's size (0: none yet, black) and how it sits.
    pub transition: f32,
    pub mix: f32,
    pub old_size: [f32; 2],
    pub old_frame: Framing,
    /// The Visuals: their own time (seconds, as fast as their Speed and
    /// Rush say), their colour, and what they hear of the music.
    pub vtime: f32,
    pub vcol: [f32; 3],
    pub scope: Scope,
    /// The Wave's colour, and Colour Map's (for Duotone).
    pub wave: [f32; 3],
    pub mapc: [f32; 3],
    /// Milk's values (see `krabton_engine::milk`), and how long this frame is (seconds).
    pub milk: [f32; MILK_N],
    pub vdt: f32,
    /// While Milk changes preset: the last one's values, how far (1: done), by which pattern.
    pub milk_old: [f32; MILK_N],
    pub milk_mix: f32,
    pub milk_style: f32,
    /// While a Look with other Visuals comes in: the scene it comes from, how far (1: done), by which pattern.
    pub vfrom: f32,
    pub vmix: f32,
    pub vstyle: f32,
    /// The Visuals scene's own settings as they are now (see
    /// `kv::scene_params`), and the scene's it comes from (while a Look
    /// with other Visuals comes in).
    pub sp: [f32; kv::SCENE_P],
    pub sp_from: [f32; kv::SCENE_P],
    /// The stage's size in pixels, as the set's shape has it.
    pub stage: [i32; 2],
}

impl Default for Params {
    fn default() -> Self {
        Self {
            video_size: [0.0; 2],
            frame: Framing::default(),
            fx: [0.0; Effect::COUNT],
            p: Self::plain_settings(),
            spin: 0.0,
            shake: [0.0; 2],
            seed: 0.0,
            grain: 0.0,
            tint: [1.0, 0.25, 0.63],
            flash: [1.0; 3],
            strobe: [1.0; 3],
            edges: [1.0; 3],
            transition: 0.0,
            mix: 1.0,
            old_size: [0.0; 2],
            old_frame: Framing::default(),
            vtime: 0.0,
            vcol: [1.0; 3],
            scope: Scope::default(),
            wave: [1.0; 3],
            mapc: [1.0, 0.55, 0.16],
            milk: milk::defaults(),
            vdt: 1.0 / 60.0,
            milk_old: milk::defaults(),
            milk_mix: 1.0,
            milk_style: 0.0,
            vfrom: 0.0,
            vmix: 1.0,
            vstyle: 0.0,
            sp: kv::scene_defaults(0),
            sp_from: kv::scene_defaults(0),
            stage: [STAGE_W, STAGE_H],
        }
    }
}

impl Params {
    /// Every effect's settings as they start.
    pub fn plain_settings() -> [[f32; 4]; Effect::COUNT] {
        let mut p = [[0.0; 4]; Effect::COUNT];
        for e in Effect::ALL {
            p[e.index()] = FxSetting::default().params4(e);
        }
        p
    }

    #[cfg(test)]
    pub fn get(&self, e: Effect) -> f32 {
        self.fx[e.index()]
    }

    /// Effect `e`'s setting `k`.
    #[cfg(test)]
    pub fn param(&self, e: Effect, k: usize) -> f32 {
        self.p[e.index()][k]
    }

    /// Between `a` (at 0) and `b` (at 1), as a Look morphs into the next:
    /// every amount, setting and colour on the way; what is chosen (a
    /// mirror's axis, Tint's blend) and the pictures are `b`'s from the start.
    pub fn morph(a: &Params, b: &Params, t: f32) -> Params {
        let t = t.clamp(0.0, 1.0);
        let l = |x: f32, y: f32| x + (y - x) * t;
        let l3 = |x: [f32; 3], y: [f32; 3]| [l(x[0], y[0]), l(x[1], y[1]), l(x[2], y[2])];
        let mut out = *b;
        for e in Effect::ALL {
            let i = e.index();
            out.fx[i] = l(a.fx[i], b.fx[i]);
            for (k, def) in e.params().iter().enumerate() {
                if def.choices.is_empty() {
                    out.p[i][k] = l(a.p[i][k], b.p[i][k]);
                }
            }
        }
        out.spin = l(a.spin, b.spin);
        out.shake = [l(a.shake[0], b.shake[0]), l(a.shake[1], b.shake[1])];
        out.tint = l3(a.tint, b.tint);
        out.flash = l3(a.flash, b.flash);
        out.strobe = l3(a.strobe, b.strobe);
        out.edges = l3(a.edges, b.edges);
        out.vcol = l3(a.vcol, b.vcol);
        out.wave = l3(a.wave, b.wave);
        out.mapc = l3(a.mapc, b.mapc);
        out.milk = milk::blend(&a.milk, &b.milk, t);
        // Visuals going away keep their scene, blend and colours as they fade.
        let v = Effect::Visuals.index();
        if b.fx[v] < 0.001 && a.fx[v] > 0.001 {
            out.p[v] = a.p[v];
            out.sp = a.sp;
        } else if a.fx[v] > 0.001 && (a.p[v][0] - b.p[v][0]).abs() > 0.5 {
            // From one scene to another: both drawn, the new coming in, each with its own settings.
            out.vfrom = a.p[v][0];
            out.vmix = t;
            out.sp_from = a.sp;
        } else {
            // The same scene: its settings on the way (a choice is the new one's).
            let defs = kv::scene_params(b.p[v][0].round().max(0.0) as usize);
            for (k, o) in out.sp.iter_mut().enumerate() {
                if defs.get(k).is_some_and(|d| d.choices.is_empty()) {
                    *o = l(a.sp[k], b.sp[k]);
                }
            }
            out.sp_from = out.sp;
        }
        out
    }

    /// Whether the Visuals draw Milk.
    pub fn milk_on(&self) -> bool {
        let milk = MILK_SCENE as f32;
        self.visuals_on() && ((self.p[Effect::Visuals.index()][0] - milk).abs() < 0.5 || (self.vmix < 0.999 && (self.vfrom - milk).abs() < 0.5))
    }

    /// Whether the Visuals show.
    pub fn visuals_on(&self) -> bool {
        self.fx[Effect::Visuals.index()] > 0.001
    }

    /// Whether only the graphics card draws it (the Visuals, the Feedback,
    /// the Wave): an export of it is drawn there too.
    pub fn needs_card(&self) -> bool {
        self.visuals_on() || self.transition > 3.5 || Effect::ALL.iter().any(|e| e.card_only() && self.fx[e.index()] > 0.001)
    }
}

/// The shader's names for the effects' amounts and settings, by their
/// place: `A_ZOOM` how much Zoom, `P_ZOOM` its settings (a vec4).
pub(crate) fn defines() -> String {
    let mut s = String::new();
    for e in Effect::ALL {
        let i = e.index();
        let name = format!("{e:?}").to_uppercase();
        s.push_str(&format!("#define A_{name} u_fx[{}].{}\n#define P_{name} u_p[{i}]\n", i / 4, ["x", "y", "z", "w"][i % 4]));
    }
    s.push_str(&format!("#define FX_VEC4 {}\n#define FX_N {}\n", Effect::COUNT.div_ceil(4), Effect::COUNT));
    s
}

/// The amounts packed four to a vec4, for `u_fx`.
pub fn packed_amounts(fx: &[f32; Effect::COUNT]) -> Vec<f32> {
    let mut v = fx.to_vec();
    v.resize(Effect::COUNT.div_ceil(4) * 4, 0.0);
    v
}

#[derive(Default)]
struct State {
    gl: Option<Gl>,
    /// It could not be made (old OpenGL): the window shows the plain picture.
    failed: Option<String>,
    /// The frame the stage was last drawn for.
    drawn: u64,
    /// The stage read back after it is drawn (RGBA, bottom row first), and
    /// whether it is wanted (while recording).
    want_pixels: bool,
    pixels: Option<(u64, Vec<u8>)>,
}

const VERTEX: &str = "
I vec2 a_pos;
O vec2 v_uv;
void main() {
    // 0,0 at the top left, as egui's pictures are.
    v_uv = vec2(a_pos.x * 0.5 + 0.5, 0.5 - a_pos.y * 0.5);
    gl_Position = vec4(a_pos, 0.0, 1.0);
}
";

/// The effects, in the order they are put on (see `export::draw`, which
/// draws the same on the processor). `A_*` and `P_*` are made by `defines`.
const EFFECTS: &str = "
uniform sampler2D u_video;
uniform sampler2D u_prev;
uniform sampler2D u_old;
uniform vec2 u_stage;
uniform vec2 u_video_size;
uniform float u_has_video;
uniform vec2 u_old_size;
uniform float u_old_has;
uniform vec4 u_nf;
uniform vec4 u_nc;
uniform vec2 u_nm;
uniform vec4 u_of;
uniform vec4 u_oc;
uniform vec2 u_om;
uniform vec4 u_fx[FX_VEC4];
uniform vec4 u_p[FX_N];
uniform float u_spin;
uniform vec2 u_shake;
uniform float u_seed;
uniform float u_grain;
uniform vec3 u_tint;
uniform vec3 u_flash;
uniform vec3 u_strobe;
uniform vec3 u_edges;
uniform float u_transition;
uniform float u_mix;
uniform sampler2D u_gen;
uniform sampler2D u_scope;
uniform float u_vtime;
uniform vec3 u_wave_col;
uniform vec3 u_map_col;
I vec2 v_uv;

float hash(float n) { return fract(sin(n) * 43758.5453); }

float luma(vec3 c) { return dot(c, vec3(0.299, 0.587, 0.114)); }

vec2 turn2(vec2 p, float a) { return vec2(cos(a) * p.x - sin(a) * p.y, sin(a) * p.x + cos(a) * p.y); }

// Where a picture `size` big is at a point of the stage (0..1 both ways):
// filling the stage (cut at the sides) or fitting in it (black bars).
vec2 place(vec2 st, vec2 size, float fill) {
    float sa = u_stage.x / u_stage.y;
    float va = size.x / max(size.y, 1.0);
    vec2 k = vec2(1.0);
    if ((fill > 0.5) == (va > sa)) { k.x = sa / va; } else { k.y = va / sa; }
    return (st - 0.5) * k + 0.5;
}

// The stage's point `st` as the picture sits (moved, turned, made bigger).
vec2 framed(vec2 st, vec4 f) {
    float aspect = u_stage.x / u_stage.y;
    vec2 q = st - 0.5 - f.xy;
    q.x *= aspect;
    q = turn2(q, -f.w * 6.2831853) / max(f.z, 0.01);
    q.x /= aspect;
    return q + 0.5;
}

float inside(vec2 uv, vec4 crop) {
    return step(crop.x, uv.x) * step(uv.x, 1.0 - crop.z) * step(crop.y, uv.y) * step(uv.y, 1.0 - crop.w);
}

vec3 pick_new(vec2 st) {
    vec2 uv = place(framed(st, u_nf), u_video_size, u_nm.y);
    return texture2D(u_video, clamp(uv, 0.0, 1.0)).rgb * inside(uv, u_nc) * u_has_video * u_nm.x;
}

vec3 pick_old(vec2 st) {
    vec2 uv = place(framed(st, u_of), u_old_size, u_om.y);
    return texture2D(u_old, clamp(uv, 0.0, 1.0)).rgb * inside(uv, u_oc) * u_old_has * u_om.x;
}

// The picture at a point of the stage: the clip that plays, or while it
// changes, the two together.
vec3 pick_film(vec2 st) {
    if (u_transition < 0.5) { return pick_new(st); }
    float m = clamp(u_mix, 0.0, 1.0);
    if (u_transition < 1.5) { return mix(pick_old(st), pick_new(st), m); }
    float peak = 1.0 - abs(m * 2.0 - 1.0);
    if (u_transition < 2.5) {
        vec3 c = m < 0.5 ? pick_old(st) : pick_new(st);
        return mix(c, vec3(1.0), peak);
    }
    // By a pattern: out from the middle, in from the edges, a wipe, a spiral, patches, blinds, rings.
    if (u_transition > 3.5 && u_transition < 10.5) {
        float w = tr_weight(st, u_stage.x / u_stage.y, m, u_transition - 3.0);
        return mix(pick_old(st), pick_new(st), w);
    }
    // Zoom: into the last, closer and closer, and out of it on the next.
    if (u_transition > 10.5 && u_transition < 11.5) {
        float z = 1.0 + peak * peak * 7.0;
        vec2 q = (st - 0.5) / z + 0.5;
        vec3 c = m < 0.5 ? pick_old(q) : pick_new(q);
        return c * (1.0 - peak * 0.35);
    }
    // Push: the next comes in from the right and pushes the last off to the left.
    if (u_transition > 11.5) {
        float x = st.x + m;
        return x < 1.0 ? pick_old(vec2(x, st.y)) : pick_new(vec2(x - 1.0, st.y));
    }
    // Glitch: strips of the new clip, more and more, pushed about.
    float band = floor(st.y * 24.0 + hash(u_seed) * 3.0);
    vec2 q = st + vec2((hash(band * 1.31 + u_seed) - 0.5) * 0.3 * peak, 0.0);
    return hash(band * 7.13 + floor(u_seed)) < m ? pick_new(q) : pick_old(q);
}

vec3 gen_at(vec2 st) { return texture2D(u_gen, vec2(st.x, 1.0 - st.y)).rgb; }

// The film with the Visuals on it, as their Blend says.
vec3 pick(vec2 st) {
    float a = A_VISUALS;
    if (a < 0.001) { return pick_film(st); }
    float mode = P_VISUALS.y;
    if (mode > 5.5) {
        // Bends the film: pushed along the Visuals' slopes.
        float e = 2.0 / u_stage.y;
        vec2 slope = vec2(luma(gen_at(st + vec2(e, 0.0))) - luma(gen_at(st - vec2(e, 0.0))), luma(gen_at(st + vec2(0.0, e))) - luma(gen_at(st - vec2(0.0, e))));
        return pick_film(st + slope * 0.15 * a);
    }
    vec3 g = gen_at(st);
    vec3 f = pick_film(st);
    if (mode < 0.5) { return mix(f, g, a); }
    if (mode < 1.5) { return f + g * a; }
    if (mode < 2.5) { return mix(f, 1.0 - (1.0 - f) * (1.0 - g), a); }
    if (mode < 3.5) { return mix(f, f * g, a); }
    if (mode < 4.5) { return mix(f, abs(f - g), a); }
    return mix(f, f * smoothstep(0.03, 0.4, luma(g)), a);
}

// A point around the middle (x in the stage's proportion), with red and blue pulled apart.
vec3 at(vec2 p) {
    float aspect = u_stage.x / u_stage.y;
    vec2 st = vec2(p.x / aspect, p.y) + 0.5;
    float off = A_SPLIT * P_SPLIT.x;
    if (off < 0.0005) { return pick(st); }
    vec2 d = vec2(cos(P_SPLIT.y * 6.2831853), sin(P_SPLIT.y * 6.2831853)) * off;
    return vec3(pick(st + d).r, pick(st).g, pick(st - d).b);
}

vec3 hue_turn(vec3 c, float a) {
    vec3 k = vec3(0.57735);
    float ca = cos(a);
    return c * ca + cross(k, c) * sin(a) + k * dot(k, c) * (1.0 - ca);
}

void main() {
    float aspect = u_stage.x / u_stage.y;
    vec2 st = v_uv;
    // Scanlines: the glass bulges.
    float off_glass = 1.0;
    if (A_SCANLINES > 0.001 && P_SCANLINES.y > 0.001) {
        vec2 c = st - 0.5;
        c *= 1.0 + dot(c, c) * P_SCANLINES.y * A_SCANLINES;
        st = c + 0.5;
        off_glass = mix(1.0, inside(st, vec4(0.0)), A_SCANLINES);
    }
    // Glitch: strips pushed sideways, different every time it changes.
    if (A_GLITCH > 0.001) {
        float band = floor(st.y * P_GLITCH.x + hash(u_seed) * 5.0);
        if (hash(band * 3.17 + u_seed * 11.0) < A_GLITCH * 0.7) {
            st.x += (hash(band * 1.71 + u_seed) - 0.5) * P_GLITCH.y * A_GLITCH;
        }
    }
    // Pixels: blocks up to its size in stage pixels.
    if (A_PIXELS > 0.001) {
        float px = mix(1.0, P_PIXELS.x, A_PIXELS * A_PIXELS);
        st = (floor(st * u_stage / px) + 0.5) * px / u_stage;
    }
    vec2 p = st - 0.5;
    p.x *= aspect;
    vec2 centre = vec2(P_ZOOM.y * aspect, P_ZOOM.z);
    p = (p - centre) / (1.0 + pow(A_ZOOM, P_ZOOM.w) * P_ZOOM.x) + centre;
    p = turn2(p, u_spin * 6.2831853);
    p -= u_shake * vec2(aspect, 1.0);
    // Tile: repeated across and down (every other mirrored).
    if (A_TILE > 0.001) {
        float n = 1.0 + floor(A_TILE * (P_TILE.x - 1.0) + 0.5);
        vec2 q = (p / vec2(aspect, 1.0) + 0.5) * n;
        vec2 cell = floor(q);
        vec2 f = fract(q);
        if (P_TILE.y > 0.5) {
            if (mod(cell.x, 2.0) > 0.5) { f.x = 1.0 - f.x; }
            if (mod(cell.y, 2.0) > 0.5) { f.y = 1.0 - f.y; }
        }
        p = (f - 0.5) * vec2(aspect, 1.0);
    }
    // Checker: the squares that are the other colour of the board, by chance as many as it is up.
    float chk = 0.0;
    if (A_CHECKER > 0.001) {
        float n = max(P_CHECKER.x, 1.0);
        vec2 g = (p + vec2(u_vtime * P_CHECKER.z * 0.1, 0.0)) * n;
        vec2 cell = floor(g);
        float swap = P_CHECKER.w > 0.001 ? floor(u_vtime * P_CHECKER.w) : 0.0;
        float odd = mod(cell.x + cell.y + swap, 2.0);
        if (odd > 0.5 && hash(cell.x * 7.13 + cell.y * 3.71) < A_CHECKER * 1.05) {
            chk = 1.0;
            vec2 f = fract(g) - 0.5;
            float style = P_CHECKER.y;
            if (style < 0.5) { f.x = -f.x; }
            else if (style > 2.5 && style < 3.5) { f = fract(f + 0.5) - 0.5; }
            else if (style > 4.5) { f = vec2(-f.y, f.x); }
            p = (cell + 0.5 + f) / n - vec2(u_vtime * P_CHECKER.z * 0.1, 0.0);
        }
    }
    // Puzzle: squares shuffled about, again at its rate.
    if (A_PUZZLE > 0.001) {
        float n = max(P_PUZZLE.x, 1.0);
        vec2 q = (p / vec2(aspect, 1.0) + 0.5) * n;
        vec2 cell = floor(q);
        float t = floor(u_vtime * P_PUZZLE.y);
        if (hash(cell.x * 5.3 + cell.y * 9.1 + t * 1.7) < A_PUZZLE) {
            vec2 to = floor(vec2(hash(cell.x + cell.y * 13.0 + t * 3.1), hash(cell.y + cell.x * 7.0 + t * 5.3)) * n);
            q = to + fract(q);
            p = (q / n - 0.5) * vec2(aspect, 1.0);
        }
    }
    // Shred: strips pushed apart, across or down.
    if (A_SHRED > 0.001) {
        float n = max(P_SHRED.x, 1.0);
        if (P_SHRED.y < 0.5) {
            float s = floor((p.y + 0.5) * n);
            p.x += (hash(s * 1.37) - 0.5) * P_SHRED.z * A_SHRED * aspect * (0.6 + 0.4 * sin(u_vtime * P_SHRED.w * 3.0 + s));
        } else {
            float s = floor((p.x / aspect + 0.5) * n);
            p.y += (hash(s * 1.37) - 0.5) * P_SHRED.z * A_SHRED * (0.6 + 0.4 * sin(u_vtime * P_SHRED.w * 3.0 + s));
        }
    }
    // Melt: dripping down in streaks.
    if (A_MELT > 0.001) {
        float col = floor((p.x / aspect + 0.5) * P_MELT.y);
        float drip = hash(col * 2.31) * (0.6 + 0.4 * sin(u_vtime * P_MELT.z * 2.0 + col * 0.7));
        p.y += drip * P_MELT.x * A_MELT * (0.5 + p.y);
    }
    // Swirl: turned round the middle, more near it.
    if (A_SWIRL > 0.001) {
        float r = length(p);
        float k = smoothstep(P_SWIRL.y, 0.0, r);
        p = turn2(p, A_SWIRL * P_SWIRL.x * 6.2831853 * k * k);
    }
    // Fisheye: bulged out (or pinched in).
    if (A_FISHEYE > 0.001) {
        float k = A_FISHEYE * P_FISHEYE.x;
        float r2 = dot(p, p);
        p = p / (1.0 + k * (1.2 - 2.4 * r2));
    }
    // Ripple: rings running out from the middle.
    if (A_RIPPLE > 0.001) {
        float r = length(p);
        if (r > 0.0001) {
            p += p / r * sin(r * P_RIPPLE.x * 12.0 - u_vtime * P_RIPPLE.y * 6.0) * 0.015 * A_RIPPLE;
        }
    }
    // VHS: the lines wobbling, a band of noise rolling down, pushing them aside.
    if (A_VHS > 0.001) {
        float y = 0.5 - p.y;
        float band = smoothstep(0.06, 0.0, abs(fract(u_vtime * 0.12) - y)) * P_VHS.y;
        p.x += (sin(y * 90.0 + u_vtime * 7.0) * 0.0015 + sin(y * 9.0 + u_vtime * 1.3) * 0.004) * P_VHS.x * A_VHS * aspect + band * 0.03 * A_VHS * (hash(floor(y * 200.0) + floor(u_vtime * 30.0)) - 0.5);
    }
    vec3 c = at(p);
    // Zoom Blur: streaks out from its point.
    if (A_ZOOMBLUR > 0.001) {
        vec2 zc = vec2(P_ZOOMBLUR.y * aspect, P_ZOOMBLUR.z);
        vec3 sum = c;
        for (int k = 1; k < 8; k++) {
            sum += at(mix(p, zc, float(k) / 8.0 * P_ZOOMBLUR.x * A_ZOOMBLUR * 0.5));
        }
        c = sum / 8.0;
    }
    // Blur: the picture round each point.
    if (A_BLUR > 0.001) {
        float r = A_BLUR * P_BLUR.x * 0.02;
        vec3 sum = c;
        for (int k = 0; k < 8; k++) {
            float a = float(k) * 0.7853982;
            sum += at(p + vec2(cos(a), sin(a)) * r);
        }
        c = sum / 9.0;
    }
    // Mirror: one half onto the other (or four ways).
    if (A_MIRROR > 0.001) {
        vec2 q = p;
        float axis = P_MIRROR.x;
        if (axis < 0.5) { q.x = -abs(q.x); }
        else if (axis < 1.5) { q.x = abs(q.x); }
        else if (axis < 2.5) { q.y = -abs(q.y); }
        else if (axis < 3.5) { q.y = abs(q.y); }
        else { q = -abs(q); }
        c = mix(c, at(q), A_MIRROR);
    }
    // Checker: its squares in other colours.
    if (chk > 0.5) {
        float style = P_CHECKER.y;
        if (style > 0.5 && style < 1.5) { c = 1.0 - c; }
        else if (style > 1.5 && style < 2.5) { c = hue_turn(c, 3.1415926); }
        else if (style > 3.5 && style < 4.5) { c *= 0.0; }
    }
    // Prism: triangles' kaleidoscope, all over, turning.
    if (A_PRISM > 0.001) {
        vec2 q = turn2(p, u_vtime * P_PRISM.y) * P_PRISM.x;
        vec2 s = vec2(1.0, 1.7320508);
        vec2 a = mod(q, s) - s * 0.5;
        vec2 b = mod(q - s * 0.5, s) - s * 0.5;
        vec2 h = dot(a, a) < dot(b, b) ? a : b;
        float seg = 1.0471976;
        float ang = mod(atan(h.y, h.x), seg);
        ang = abs(ang - seg * 0.5);
        c = mix(c, at(vec2(cos(ang), sin(ang)) * length(h) * 0.9), A_PRISM);
    }
    // Polar: wrapped round, a little planet or a tunnel.
    if (A_POLAR > 0.001) {
        float r = length(p) / max(P_POLAR.x, 0.05);
        float ang = atan(p.y, p.x) / 6.2831853 + 0.5 + u_vtime * P_POLAR.y * 0.05;
        float y = P_POLAR.z > 0.5 ? r - 0.5 : 0.5 - r;
        c = mix(c, at(vec2((fract(ang) - 0.5) * aspect, clamp(y, -0.5, 0.5))), A_POLAR);
    }
    // Droste: inside itself, zooming on.
    if (A_DROSTE > 0.001) {
        float sc = max(P_DROSTE.x, 1.1);
        float t = fract(u_vtime * P_DROSTE.y * 0.25);
        vec2 q = p * pow(sc, -t);
        float l = log(max(length(q), 1e-4)) / log(sc);
        float lvl = floor(l + 1.0);
        q *= pow(sc, -lvl) * sc;
        q = turn2(q, lvl * P_DROSTE.z * 1.5707963);
        c = mix(c, at(q * 0.95), A_DROSTE);
    }
    // Pop Art: copies side by side, each in flat colours of its own.
    if (A_POPART > 0.001) {
        float n = floor(max(P_POPART.x, 1.0) + 0.5);
        vec2 q = (p / vec2(aspect, 1.0) + 0.5) * n;
        vec2 cell = floor(q);
        vec3 src = at((fract(q) - 0.5) * vec2(aspect, 1.0));
        float steps = max(P_POPART.y, 2.0);
        float l = floor(luma(src) * steps) / (steps - 1.0);
        float hue = fract(P_POPART.z + (cell.x + cell.y * n) / (n * n));
        vec3 ink = 0.5 + 0.5 * cos(6.2831853 * (hue + vec3(0.0, 0.33, 0.67)));
        vec3 paper = 0.5 + 0.5 * cos(6.2831853 * (hue + 0.5 + vec3(0.0, 0.33, 0.67)));
        c = mix(c, mix(ink * 0.25, paper, l), A_POPART);
    }
    // Kaleido: mirrored wedges.
    if (A_KALEIDO > 0.001) {
        float seg = 6.2831853 / max(P_KALEIDO.x, 1.0);
        float ang = mod(atan(p.y, p.x) + P_KALEIDO.y * 6.2831853, seg);
        ang = abs(ang - seg * 0.5);
        c = mix(c, at(vec2(cos(ang), sin(ang)) * length(p)), A_KALEIDO);
    }
    // Edges: the outlines, in their colour.
    if (A_EDGES > 0.001) {
        float d = P_EDGES.x / u_stage.y;
        float gx = luma(at(p + vec2(d, 0.0))) - luma(at(p - vec2(d, 0.0)));
        float gy = luma(at(p + vec2(0.0, d))) - luma(at(p - vec2(0.0, d)));
        vec3 lines = u_edges * clamp(length(vec2(gx, gy)) * 4.0, 0.0, 1.0);
        c = mix(c, P_EDGES.y < 0.5 ? lines : max(c, lines), A_EDGES);
    }
    if (A_HUE > 0.001) {
        c = clamp(hue_turn(c, A_HUE * P_HUE.x * 6.2831853), 0.0, 1.0);
        c = clamp(mix(vec3(luma(c)), c, 1.0 + (P_HUE.y - 1.0) * A_HUE), 0.0, 1.0);
    }
    if (A_GRADE > 0.001) {
        c = (c - 0.5) * (1.0 + (P_GRADE.y - 1.0) * A_GRADE) + 0.5 + P_GRADE.x * A_GRADE;
        c = clamp(mix(vec3(luma(c)), c, 1.0 + (P_GRADE.z - 1.0) * A_GRADE), 0.0, 1.0);
    }
    if (A_POSTER > 0.001) {
        float levels = mix(24.0, P_POSTER.x, A_POSTER);
        c = floor(c * levels + 0.5) / levels;
    }
    // Tint, blended as it says.
    float blend = P_TINT.x;
    vec3 tinted = blend < 0.5 ? u_tint * (0.2 + luma(c) * 1.3) : blend < 1.5 ? c * u_tint : blend < 2.5 ? 1.0 - (1.0 - c) * (1.0 - u_tint) : c + u_tint;
    c = mix(c, tinted, A_TINT);
    c = mix(c, 1.0 - c, A_INVERT);
    c = mix(c, u_flash, A_FLASH * P_FLASH.x);
    // VHS: the colours bleeding sideways, noise over it.
    if (A_VHS > 0.001) {
        float b = P_VHS.z * 0.012 * A_VHS;
        vec3 bled = vec3(at(p + vec2(b, 0.0)).r, c.g, at(p - vec2(b * 0.6, 0.0)).b);
        c = mix(c, bled, A_VHS);
        float y = v_uv.y;
        float band = smoothstep(0.06, 0.0, abs(fract(u_vtime * 0.12) - y));
        float n = hash(floor(v_uv.x * u_stage.x * 0.5) * 3.1 + floor(y * u_stage.y * 0.5) * 7.7 + floor(u_vtime * 30.0) * 13.0);
        c += (n - 0.5) * P_VHS.y * A_VHS * (0.12 + band * 0.6);
        c = mix(c, c * vec3(1.05, 0.95, 1.08), A_VHS * 0.5);
    }
    // Smear: the bright parts dragged along (as pixels sorted).
    if (A_SMEAR > 0.001) {
        float a = P_SMEAR.z * 6.2831853;
        vec2 dir = vec2(sin(a) * aspect, cos(a)) * P_SMEAR.y * A_SMEAR / 10.0;
        for (int k = 1; k <= 10; k++) {
            vec3 s = at(p + dir * float(k));
            if (luma(s) > P_SMEAR.x && luma(s) > luma(c)) {
                c = mix(c, s, 1.0 - float(k) / 11.0);
            }
        }
    }
    // Bloom: the bright parts glowing over the rest.
    if (A_BLOOM > 0.001) {
        vec3 glow = vec3(0.0);
        float reach = 0.01 + 0.05 * P_BLOOM.y;
        for (int k = 0; k < 12; k++) {
            float a = float(k) * 0.5235988;
            float d = reach * (0.35 + 0.65 * fract(float(k) * 0.618));
            vec3 s = at(p + vec2(cos(a), sin(a)) * d);
            glow += max(s - P_BLOOM.x, 0.0);
        }
        c += glow / 12.0 * 2.5 * A_BLOOM;
    }
    // Colour Map: its light as other colours.
    if (A_MAP > 0.001) {
        float l = clamp(luma(c), 0.0, 1.0);
        float m = P_MAP.x;
        vec3 mapped;
        if (m < 0.5) {
            mapped = clamp(vec3(1.5 * l - 0.2, sin(l * 3.14159) * 1.1 - 0.1, 0.9 - 1.6 * l) + vec3(0.0, 0.0, smoothstep(0.85, 1.0, l)), 0.0, 1.0);
            mapped = mix(vec3(0.05, 0.0, 0.25), mapped, smoothstep(0.0, 0.2, l));
        } else if (m < 1.5) {
            mapped = vec3(0.15, 1.0, 0.25) * (0.15 + 1.1 * l) + (hash(v_uv.x * 731.0 + v_uv.y * 977.0 + u_grain) - 0.5) * 0.12;
        } else if (m < 2.5) {
            mapped = vec3(1.0, 0.85, 0.62) * l + vec3(0.08, 0.04, 0.0);
        } else if (m < 3.5) {
            mapped = mix(vec3(0.05, 0.05, 0.12), u_map_col, l);
        } else if (m < 4.5) {
            mapped = vec3(0.0, 1.0, 0.35) * l * (0.6 + 0.4 * step(0.5, fract(v_uv.y * u_stage.y / 3.0)));
        } else {
            mapped = vec3(1.0 - l * 0.2, 0.25 + 0.6 * (1.0 - l), 0.6 + 0.4 * l) * (0.4 + 0.8 * luma(vec3(c.g, c.g, c.r)));
        }
        c = mix(c, mapped, A_MAP);
    }
    // Halftone: in dots, as printed.
    if (A_HALFTONE > 0.001) {
        float size = P_HALFTONE.x / u_stage.y;
        vec2 q = turn2(p, P_HALFTONE.y * 6.2831853) / size;
        vec2 cell = floor(q) + 0.5;
        vec3 ink = at(turn2(cell * size, -P_HALFTONE.y * 6.2831853));
        float l = luma(ink);
        float ink_mode = P_HALFTONE.z;
        float r = sqrt(ink_mode > 1.5 ? l : (ink_mode > 0.5 ? 1.0 - l : l)) * 0.7;
        float dot_ = smoothstep(r + 0.06, r - 0.06, length(q - cell));
        vec3 h;
        if (ink_mode < 0.5) { h = ink * dot_ * 1.2; }
        else if (ink_mode < 1.5) { h = vec3(1.0 - dot_) * vec3(0.98, 0.96, 0.9); }
        else { h = vec3(dot_); }
        c = mix(c, h, A_HALFTONE);
    }
    // ASCII: in signs, by how light each cell is.
    if (A_ASCII > 0.001) {
        float size = P_ASCII.x / u_stage.y;
        vec2 q = (p + vec2(aspect, 1.0) * 0.5) / size;
        vec2 cell = floor(q);
        vec2 f = fract(q) - 0.5;
        vec3 ink = at((cell + 0.5) * size - vec2(aspect, 1.0) * 0.5);
        float l = luma(ink);
        float k = floor(clamp(l, 0.0, 0.999) * 10.0);
        float w = 0.09;
        float on = 0.0;
        vec2 a = abs(f);
        if (k < 0.5) { on = 0.0; }
        else if (k < 1.5) { on = step(length(f - vec2(0.0, 0.3)), 0.08); }
        else if (k < 2.5) { on = step(a.y, w) * step(a.x, 0.3); }
        else if (k < 3.5) { on = max(step(a.y, w) * step(a.x, 0.32), step(a.x, w) * step(a.y, 0.32)); }
        else if (k < 4.5) { on = step(abs(f.x - f.y), w * 1.4) * step(a.x, 0.32) + step(abs(f.x + f.y), w * 1.4) * step(a.x, 0.32); }
        else if (k < 5.5) { on = step(abs(length(f) - 0.28), w); }
        else if (k < 6.5) { on = max(step(abs(a.x - 0.16), w) * step(a.y, 0.38), step(abs(a.y - 0.14), w) * step(a.x, 0.38)); }
        else if (k < 7.5) { on = max(step(abs(max(a.x, a.y) - 0.3), w), step(length(f), 0.1)); }
        else if (k < 8.5) { on = step(length(f), 0.32); }
        else { on = step(max(a.x, a.y), 0.38); }
        on = clamp(on, 0.0, 1.0);
        vec3 ink_c = P_ASCII.y < 0.5 ? ink * 1.4 : (P_ASCII.y < 1.5 ? vec3(0.2, 1.0, 0.35) : vec3(1.0));
        c = mix(c, ink_c * on, A_ASCII);
    }
    // Dither: few shades, dithered with a pattern of dots.
    if (A_DITHER > 0.001) {
        vec2 px = floor(v_uv * u_stage / max(P_DITHER.y, 1.0));
        vec2 a = mod(px, 2.0);
        vec2 b = mod(floor(px / 2.0), 2.0);
        float bayer = (4.0 * mod(a.x * 2.0 + a.y * 3.0, 4.0) + mod(b.x * 2.0 + b.y * 3.0, 4.0)) / 16.0;
        float levels = max(P_DITHER.x - 1.0, 1.0);
        vec3 d = floor(clamp(c, 0.0, 1.0) * levels + bayer) / levels;
        c = mix(c, d, A_DITHER);
    }
    // Vignette: darker towards the corners.
    if (A_VIGNETTE > 0.001) {
        float r = length((v_uv - 0.5) * 2.0) * 0.70710678;
        float inner = 1.0 - P_VIGNETTE.x;
        c *= 1.0 - smoothstep(inner - P_VIGNETTE.y * 0.5, inner + P_VIGNETTE.y * 0.5, r) * A_VIGNETTE;
    }
    // Scanlines: the lines, and black off the glass.
    if (A_SCANLINES > 0.001) {
        float s = 0.5 + 0.5 * cos(st.y * P_SCANLINES.x * 6.2831853);
        c *= (1.0 - A_SCANLINES * 0.6 * (1.0 - s)) * off_glass;
    }
    if (A_GRAIN > 0.001) {
        vec2 g = floor(v_uv * u_stage / max(P_GRAIN.x, 1.0));
        c += (hash(g.x * 12.9898 + g.y * 78.233 + u_grain) - 0.5) * A_GRAIN * 0.5;
    }
    c *= 1.0 - A_GATE;
    c = mix(c, u_strobe, A_STROBE);
    // Trails: the last frame, drawn a little closer, fading under this one.
    if (A_TRAILS > 0.001) {
        vec2 q = (v_uv - 0.5) * P_TRAILS.x + 0.5;
        vec3 prev = texture2D(u_prev, vec2(q.x, 1.0 - q.y)).rgb;
        c = max(c, mix(c, prev, A_TRAILS * P_TRAILS.y));
    }
    // Echo: the last frame moved a little, under this one.
    if (A_ECHO > 0.001) {
        vec2 q = v_uv - P_ECHO.xy;
        vec3 prev = texture2D(u_prev, vec2(q.x, 1.0 - q.y)).rgb * inside(q, vec4(0.0));
        c = mix(c, prev, A_ECHO * P_ECHO.z * 0.85);
    }
    // Datamosh: blocks of the last frame left behind, drifting.
    if (A_DATAMOSH > 0.001) {
        vec2 n = vec2(P_DATAMOSH.x, floor(P_DATAMOSH.x / aspect + 0.5));
        vec2 blk = floor(v_uv * n);
        float h = hash(blk.x * 17.13 + blk.y * 3.71 + floor(u_vtime * 8.0) * 0.37);
        if (h < A_DATAMOSH * 0.85) {
            vec2 drift = (vec2(hash(blk.x + blk.y * 9.1), hash(blk.y + blk.x * 5.3)) - 0.5) * P_DATAMOSH.y * 0.02;
            vec2 q = v_uv + drift;
            c = texture2D(u_prev, vec2(q.x, 1.0 - q.y)).rgb;
        }
    }
    // Wave: the sound's wave over it (and, with Feedback, carried on in it).
    if (A_WAVE > 0.001) {
        float px = 1.0 / u_stage.y;
        vec2 q = vec2((v_uv.x - 0.5) * aspect, 0.5 - v_uv.y - P_WAVE.w);
        float amp = 0.05 + 0.35 * P_WAVE.y;
        float th = P_WAVE.z * px * 0.5;
        float d = 1.0;
        if (P_WAVE.x < 0.5) {
            d = abs(q.y - (texture2D(u_scope, vec2(v_uv.x, 0.125)).r * 2.0 - 1.0) * amp);
        } else if (P_WAVE.x < 1.5) {
            float a = atan(q.y, q.x) / 6.2831853 + 0.5;
            float rr = 0.08 + 0.3 * P_WAVE.y + (texture2D(u_scope, vec2(a, 0.125)).r * 2.0 - 1.0) * amp * 0.35;
            d = abs(length(q) - rr);
        } else if (P_WAVE.x < 2.5) {
            float xi = (floor(v_uv.x * 64.0) + 0.5) / 64.0;
            float w = (texture2D(u_scope, vec2(xi, 0.125)).r * 2.0 - 1.0) * amp;
            d = max(length(vec2((v_uv.x - xi) * aspect, q.y - w)) - th * 2.0, 0.0);
        } else {
            float sx = abs(v_uv.x - 0.5) * 2.0;
            float s = texture2D(u_scope, vec2(sx * (127.0 / 128.0) + 0.5 / 128.0, 0.375)).r;
            d = (abs(q.y) < s * amp * 1.5 && fract(sx * 48.0) > 0.25) ? 0.0 : 1.0;
        }
        float line = smoothstep(th + px, max(th - px * 0.5, 0.0), d);
        c = mix(c, u_wave_col, line * A_WAVE);
    }
    // Feedback: the last frame closer in, turned and stirred, fading under this one.
    if (A_FEEDBACK > 0.001) {
        vec2 q = (v_uv - 0.5) * vec2(aspect, 1.0);
        q = turn2(q, -P_FEEDBACK.y * 6.2831853 * A_FEEDBACK) / mix(1.0, P_FEEDBACK.x, A_FEEDBACK);
        float stir = P_FEEDBACK.z * 0.012 * A_FEEDBACK;
        q += stir * vec2(sin(q.y * 7.0 + u_vtime * 1.3), cos(q.x * 7.0 - u_vtime * 1.1));
        q = q / vec2(aspect, 1.0) + 0.5;
        vec3 prev = texture2D(u_prev, vec2(q.x, 1.0 - q.y)).rgb * inside(q, vec4(0.0));
        c = max(c, prev * P_FEEDBACK.w * A_FEEDBACK);
    }
    gl_FragColor = vec4(clamp(c, 0.0, 1.0), 1.0);
}
";

const SHOW: &str = "
uniform sampler2D u_stage_tex;
I vec2 v_uv;
void main() {
    gl_FragColor = vec4(texture2D(u_stage_tex, vec2(v_uv.x, 1.0 - v_uv.y)).rgb, 1.0);
}
";

/// The OpenGL side: made the first time it draws.
struct Gl {
    fx: glow::Program,
    show: glow::Program,
    quad: glow::Buffer,
    vao: Option<glow::VertexArray>,
    fbo: glow::Framebuffer,
    /// The stage, twice: the one drawn last and the one drawn into.
    tex: [glow::Texture; 2],
    cur: usize,
    /// Its size (the stage's, or an export's).
    w: i32,
    h: i32,
    /// The Visuals: their shader (made the first time they show; an error
    /// if it could not be), their picture, and the music they are shown.
    gen: Option<Result<glow::Program, String>>,
    gen_tex: glow::Texture,
    scope_tex: glow::Texture,
    hist_tex: glow::Texture,
    /// Milk: its shader, and its feedback (the frame drawn last and the one drawn into).
    milk: Option<Result<glow::Program, String>>,
    milk_tex: [glow::Texture; 2],
    milk_cur: usize,
}

/// The stage, shared by every place that shows it.
#[derive(Clone, Default)]
pub struct Stage {
    state: Arc<Mutex<State>>,
}

impl Stage {
    /// Why the shaders could not be made, if they could not.
    pub fn failed(&self) -> Option<String> {
        self.state.lock().ok().and_then(|s| s.failed.clone())
    }

    /// Whether the stage is read back after it is drawn (while recording).
    pub fn want_pixels(&self, on: bool) {
        if let Ok(mut s) = self.state.lock() {
            s.want_pixels = on;
            if !on {
                s.pixels = None;
            }
        }
    }

    /// The stage last read back (RGBA, at the shape's stage size, bottom row
    /// first) and the frame it was drawn for, once.
    pub fn take_pixels(&self) -> Option<(u64, Vec<u8>)> {
        self.state.lock().ok().and_then(|mut s| s.pixels.take())
    }

    /// Draws the stage into `rect` (the effects are worked out the first
    /// time it is drawn in frame `frame`), from the picture in `video` (and
    /// `old`, the clip it is changing from).
    pub fn paint(&self, painter: &egui::Painter, rect: egui::Rect, video: Option<egui::TextureId>, old: Option<egui::TextureId>, params: Params, frame: u64) {
        let state = self.state.clone();
        let cb = egui_glow::CallbackFn::new(move |info, p| {
            let Ok(mut st) = state.lock() else { return };
            if st.failed.is_some() {
                return;
            }
            let gl = p.gl();
            // Another shape: the stage made again at its size.
            let want = (params.stage[0].max(16), params.stage[1].max(16));
            if st.gl.as_ref().is_some_and(|g| (g.w, g.h) != want) {
                st.gl = None;
            }
            if st.gl.is_none() {
                // SAFETY: on the painter's thread with its context current, as egui_glow calls back.
                match unsafe { Gl::sized(gl, want.0, want.1) } {
                    Ok(g) => st.gl = Some(g),
                    Err(e) => {
                        eprintln!("video: {e}");
                        st.failed = Some(e);
                        return;
                    }
                }
            }
            let fresh = st.drawn != frame;
            st.drawn = frame;
            let want = st.want_pixels;
            let Some(g) = st.gl.as_mut() else { return };
            let video_tex = video.and_then(|id| p.texture(id));
            let old_tex = old.and_then(|id| p.texture(id));
            let mut read = None;
            // SAFETY: as above; every object used was made on this context.
            unsafe {
                if fresh {
                    g.draw_effects(gl, video_tex, old_tex, &params);
                    if want {
                        // The stage just drawn, still bound to the frame buffer.
                        let mut px = vec![0u8; (g.w * g.h * 4) as usize];
                        gl.read_pixels(0, 0, g.w, g.h, glow::RGBA, glow::UNSIGNED_BYTE, glow::PixelPackData::Slice(Some(&mut px)));
                        read = Some(px);
                    }
                    gl.bind_framebuffer(glow::FRAMEBUFFER, p.intermediate_fbo());
                }
                let vp = info.viewport_in_pixels();
                gl.viewport(vp.left_px, vp.from_bottom_px, vp.width_px, vp.height_px);
                let clip = info.clip_rect_in_pixels();
                gl.enable(glow::SCISSOR_TEST);
                gl.scissor(clip.left_px, clip.from_bottom_px, clip.width_px, clip.height_px);
                g.draw_stage(gl);
            }
            if let Some(px) = read {
                st.pixels = Some((frame, px));
            }
        });
        painter.add(egui::PaintCallback { rect, callback: Arc::new(cb) });
    }
}

/// The start of each shader: its version, and words that work on old and
/// new OpenGL (and OpenGL ES) alike.
fn header(gl: &glow::Context, fragment: bool) -> String {
    let v = egui_glow::ShaderVersion::get(gl);
    let new = v.is_new_shader_interface();
    let mut s = format!("{}\n", v.version_declaration());
    s.push_str("#ifdef GL_ES\n#if defined(GL_FRAGMENT_PRECISION_HIGH) && GL_FRAGMENT_PRECISION_HIGH == 1\nprecision highp float;\n#else\nprecision mediump float;\n#endif\n#endif\n");
    match (new, fragment) {
        (true, false) => s.push_str("#define I in\n#define O out\n"),
        (false, false) => s.push_str("#define I attribute\n#define O varying\n"),
        (true, true) => s.push_str("#define I in\nout vec4 f_color;\n#define gl_FragColor f_color\n#define texture2D texture\n"),
        (false, true) => s.push_str("#define I varying\n"),
    }
    s
}

unsafe fn compile(gl: &glow::Context, kind: u32, src: &str) -> Result<glow::Shader, String> {
    let sh = gl.create_shader(kind)?;
    gl.shader_source(sh, src);
    gl.compile_shader(sh);
    if !gl.get_shader_compile_status(sh) {
        let log = gl.get_shader_info_log(sh);
        gl.delete_shader(sh);
        return Err(format!("the video shader did not compile: {log}"));
    }
    Ok(sh)
}

pub(crate) unsafe fn program(gl: &glow::Context, frag: &str) -> Result<glow::Program, String> {
    let vs = compile(gl, glow::VERTEX_SHADER, &format!("{}{VERTEX}", header(gl, false)))?;
    let fs = compile(gl, glow::FRAGMENT_SHADER, &format!("{}{frag}", header(gl, true)))?;
    let prog = gl.create_program()?;
    gl.attach_shader(prog, vs);
    gl.attach_shader(prog, fs);
    gl.bind_attrib_location(prog, 0, "a_pos");
    gl.link_program(prog);
    gl.detach_shader(prog, vs);
    gl.detach_shader(prog, fs);
    gl.delete_shader(vs);
    gl.delete_shader(fs);
    if !gl.get_program_link_status(prog) {
        let log = gl.get_program_info_log(prog);
        gl.delete_program(prog);
        return Err(format!("the video shader did not link: {log}"));
    }
    Ok(prog)
}

/// Milk's values for a shader: the preset's, the last one's, how far the change is, its pattern.
unsafe fn milk_uniforms(gl: &glow::Context, prog: glow::Program, p: &Params) {
    let u = |name: &str| gl.get_uniform_location(prog, name);
    gl.uniform_4_f32_slice(u("u_m[0]").as_ref(), &visuals::milk_packed(&p.milk));
    gl.uniform_4_f32_slice(u("u_o[0]").as_ref(), &visuals::milk_packed(&p.milk_old));
    gl.uniform_1_f32(u("u_mmix").as_ref(), p.milk_mix);
    gl.uniform_1_f32(u("u_mstyle").as_ref(), p.milk_style);
}

/// A picture to draw into (or from), black, `w` by `h`.
unsafe fn new_texture(gl: &glow::Context, w: i32, h: i32) -> Result<glow::Texture, String> {
    let t = gl.create_texture()?;
    gl.bind_texture(glow::TEXTURE_2D, Some(t));
    gl.tex_image_2d(glow::TEXTURE_2D, 0, glow::RGBA as i32, w, h, 0, glow::RGBA, glow::UNSIGNED_BYTE, glow::PixelUnpackData::Slice(None));
    gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::LINEAR as i32);
    gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::LINEAR as i32);
    gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE as i32);
    gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE as i32);
    gl.bind_texture(glow::TEXTURE_2D, None);
    Ok(t)
}

impl Gl {
    unsafe fn sized(gl: &glow::Context, w: i32, h: i32) -> Result<Self, String> {
        let fx = program(gl, &format!("{}{}{EFFECTS}", defines(), visuals::PATTERNS))?;
        let show = program(gl, SHOW)?;
        let quad = gl.create_buffer()?;
        let corners: [f32; 8] = [-1.0, -1.0, 1.0, -1.0, -1.0, 1.0, 1.0, 1.0];
        let bytes: Vec<u8> = corners.iter().flat_map(|v| v.to_ne_bytes()).collect();
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(quad));
        gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, &bytes, glow::STATIC_DRAW);
        // Newer OpenGL (and the core profile on macOS) draws through a vertex array.
        let vao = if egui_glow::ShaderVersion::get(gl).is_new_shader_interface() {
            let vao = gl.create_vertex_array()?;
            gl.bind_vertex_array(Some(vao));
            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, 8, 0);
            gl.bind_vertex_array(None);
            Some(vao)
        } else {
            None
        };
        gl.bind_buffer(glow::ARRAY_BUFFER, None);
        let fbo = gl.create_framebuffer()?;
        let mut tex = [gl.create_texture()?, gl.create_texture()?];
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(fbo));
        for t in &mut tex {
            gl.bind_texture(glow::TEXTURE_2D, Some(*t));
            gl.tex_image_2d(glow::TEXTURE_2D, 0, glow::RGBA as i32, w, h, 0, glow::RGBA, glow::UNSIGNED_BYTE, glow::PixelUnpackData::Slice(None));
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::LINEAR as i32);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::LINEAR as i32);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE as i32);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE as i32);
            gl.framebuffer_texture_2d(glow::FRAMEBUFFER, glow::COLOR_ATTACHMENT0, glow::TEXTURE_2D, Some(*t), 0);
            if gl.check_framebuffer_status(glow::FRAMEBUFFER) != glow::FRAMEBUFFER_COMPLETE {
                gl.bind_framebuffer(glow::FRAMEBUFFER, None);
                return Err("the video stage could not be made".into());
            }
            gl.disable(glow::SCISSOR_TEST);
            gl.clear_color(0.0, 0.0, 0.0, 1.0);
            gl.clear(glow::COLOR_BUFFER_BIT);
        }
        gl.bind_texture(glow::TEXTURE_2D, None);
        gl.bind_framebuffer(glow::FRAMEBUFFER, None);
        let gen_tex = new_texture(gl, w, h)?;
        let scope_tex = new_texture(gl, SCOPE_W as i32, 4)?;
        let hist_tex = new_texture(gl, SCOPE_W as i32, visuals::HIST as i32)?;
        let milk_tex = [new_texture(gl, w, h)?, new_texture(gl, w, h)?];
        Ok(Self { fx, show, quad, vao, fbo, tex, cur: 0, w, h, gen: None, gen_tex, scope_tex, hist_tex, milk: None, milk_tex, milk_cur: 0 })
    }

    /// Milk's feedback, one frame on (made the first time; false when its shader could not be).
    unsafe fn draw_milk(&mut self, gl: &glow::Context, p: &Params) -> bool {
        if self.milk.is_none() {
            let made = program(gl, &format!("{}{}", visuals::milk_defines(), visuals::MILK_SHADER));
            if let Err(e) = &made {
                eprintln!("video: Milk: {e}");
            }
            self.milk = Some(made);
        }
        let Some(Ok(prog)) = self.milk else { return false };
        let next = 1 - self.milk_cur;
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(self.fbo));
        gl.framebuffer_texture_2d(glow::FRAMEBUFFER, glow::COLOR_ATTACHMENT0, glow::TEXTURE_2D, Some(self.milk_tex[next]), 0);
        gl.viewport(0, 0, self.w, self.h);
        gl.disable(glow::SCISSOR_TEST);
        gl.disable(glow::BLEND);
        gl.use_program(Some(prog));
        gl.active_texture(glow::TEXTURE1);
        gl.bind_texture(glow::TEXTURE_2D, Some(self.scope_tex));
        gl.active_texture(glow::TEXTURE0);
        gl.bind_texture(glow::TEXTURE_2D, Some(self.milk_tex[self.milk_cur]));
        let u = |name: &str| gl.get_uniform_location(prog, name);
        gl.uniform_1_i32(u("u_prev").as_ref(), 0);
        gl.uniform_1_i32(u("u_scope").as_ref(), 1);
        milk_uniforms(gl, prog, p);
        gl.uniform_2_f32(u("u_stage").as_ref(), self.w as f32, self.h as f32);
        gl.uniform_1_f32(u("u_time").as_ref(), p.vtime);
        gl.uniform_1_f32(u("u_k").as_ref(), (p.vdt * 60.0).clamp(0.25, 4.0));
        self.quad(gl);
        gl.active_texture(glow::TEXTURE1);
        gl.bind_texture(glow::TEXTURE_2D, None);
        gl.active_texture(glow::TEXTURE0);
        gl.bind_texture(glow::TEXTURE_2D, None);
        self.milk_cur = next;
        true
    }

    /// The Visuals, into their picture (made the first time; nothing when their shader could not be made).
    unsafe fn draw_visuals(&mut self, gl: &glow::Context, p: &Params) -> bool {
        if self.gen.is_none() {
            let made = program(gl, &format!("{}{}{}", defines(), visuals::milk_defines(), visuals::SHADER));
            if let Err(e) = &made {
                eprintln!("video: the Visuals: {e}");
            }
            self.gen = Some(made);
        }
        let Some(Ok(prog)) = self.gen else { return false };
        // Milk first: its feedback goes on a frame.
        let milk = p.milk_on() && self.draw_milk(gl, p);
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(self.fbo));
        gl.framebuffer_texture_2d(glow::FRAMEBUFFER, glow::COLOR_ATTACHMENT0, glow::TEXTURE_2D, Some(self.gen_tex), 0);
        gl.viewport(0, 0, self.w, self.h);
        gl.disable(glow::SCISSOR_TEST);
        gl.disable(glow::BLEND);
        gl.use_program(Some(prog));
        gl.active_texture(glow::TEXTURE0);
        gl.bind_texture(glow::TEXTURE_2D, Some(self.scope_tex));
        let u = |name: &str| gl.get_uniform_location(prog, name);
        gl.uniform_1_i32(u("u_scope").as_ref(), 0);
        gl.active_texture(glow::TEXTURE1);
        gl.bind_texture(glow::TEXTURE_2D, Some(self.milk_tex[self.milk_cur]));
        gl.active_texture(glow::TEXTURE0);
        gl.uniform_1_i32(u("u_milk").as_ref(), 1);
        // The spectrum's history.
        gl.active_texture(glow::TEXTURE2);
        gl.bind_texture(glow::TEXTURE_2D, Some(self.hist_tex));
        gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
        gl.tex_sub_image_2d(glow::TEXTURE_2D, 0, 0, 0, SCOPE_W as i32, visuals::HIST as i32, glow::RGBA, glow::UNSIGNED_BYTE, glow::PixelUnpackData::Slice(Some(&p.scope.hist_rgba())));
        gl.active_texture(glow::TEXTURE0);
        gl.uniform_1_i32(u("u_hist").as_ref(), 2);
        milk_uniforms(gl, prog, p);
        gl.uniform_1_f32(u("u_vfrom").as_ref(), p.vfrom);
        gl.uniform_1_f32(u("u_vmix").as_ref(), p.vmix);
        gl.uniform_1_f32(u("u_vstyle").as_ref(), p.vstyle);
        gl.uniform_3_f32(u("u_sa").as_ref(), p.sp[0], p.sp[1], p.sp[2]);
        gl.uniform_3_f32(u("u_sb").as_ref(), p.sp[3], p.sp[4], p.sp[5]);
        gl.uniform_3_f32(u("u_fa").as_ref(), p.sp_from[0], p.sp_from[1], p.sp_from[2]);
        gl.uniform_3_f32(u("u_fb").as_ref(), p.sp_from[3], p.sp_from[4], p.sp_from[5]);
        let _ = milk;
        gl.uniform_2_f32(u("u_stage").as_ref(), self.w as f32, self.h as f32);
        gl.uniform_4_f32_slice(u("u_fx[0]").as_ref(), &packed_amounts(&p.fx));
        let settings: Vec<f32> = p.p.iter().flatten().copied().collect();
        gl.uniform_4_f32_slice(u("u_p[0]").as_ref(), &settings);
        gl.uniform_1_f32(u("u_time").as_ref(), p.vtime);
        gl.uniform_1_f32(u("u_seed").as_ref(), p.seed);
        gl.uniform_3_f32(u("u_vcol").as_ref(), p.vcol[0], p.vcol[1], p.vcol[2]);
        self.quad(gl);
        gl.active_texture(glow::TEXTURE2);
        gl.bind_texture(glow::TEXTURE_2D, None);
        gl.active_texture(glow::TEXTURE1);
        gl.bind_texture(glow::TEXTURE_2D, None);
        gl.active_texture(glow::TEXTURE0);
        gl.bind_texture(glow::TEXTURE_2D, None);
        true
    }

    unsafe fn quad(&self, gl: &glow::Context) {
        match self.vao {
            Some(vao) => {
                gl.bind_vertex_array(Some(vao));
                gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);
                gl.bind_vertex_array(None);
            }
            None => {
                gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.quad));
                gl.enable_vertex_attrib_array(0);
                gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, 8, 0);
                gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);
                gl.disable_vertex_attrib_array(0);
                gl.bind_buffer(glow::ARRAY_BUFFER, None);
            }
        }
    }

    /// The effects, from the picture and the last stage into the other stage.
    unsafe fn draw_effects(&mut self, gl: &glow::Context, video: Option<glow::Texture>, old: Option<glow::Texture>, p: &Params) {
        // What the music is now (for the Visuals and the Wave).
        gl.bind_texture(glow::TEXTURE_2D, Some(self.scope_tex));
        gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
        gl.tex_sub_image_2d(glow::TEXTURE_2D, 0, 0, 0, SCOPE_W as i32, 4, glow::RGBA, glow::UNSIGNED_BYTE, glow::PixelUnpackData::Slice(Some(&p.scope.rgba())));
        gl.bind_texture(glow::TEXTURE_2D, None);
        // The Visuals first (without them, or their shader, the effects see none).
        let mut p = *p;
        if p.visuals_on() && !self.draw_visuals(gl, &p) {
            p.fx[Effect::Visuals.index()] = 0.0;
        }
        let p = &p;
        let next = 1 - self.cur;
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(self.fbo));
        gl.framebuffer_texture_2d(glow::FRAMEBUFFER, glow::COLOR_ATTACHMENT0, glow::TEXTURE_2D, Some(self.tex[next]), 0);
        gl.viewport(0, 0, self.w, self.h);
        gl.disable(glow::SCISSOR_TEST);
        gl.disable(glow::BLEND);
        gl.use_program(Some(self.fx));
        gl.active_texture(glow::TEXTURE1);
        gl.bind_texture(glow::TEXTURE_2D, Some(self.tex[self.cur]));
        gl.active_texture(glow::TEXTURE2);
        gl.bind_texture(glow::TEXTURE_2D, Some(old.unwrap_or(self.tex[self.cur])));
        gl.active_texture(glow::TEXTURE3);
        gl.bind_texture(glow::TEXTURE_2D, Some(self.gen_tex));
        gl.active_texture(glow::TEXTURE4);
        gl.bind_texture(glow::TEXTURE_2D, Some(self.scope_tex));
        gl.active_texture(glow::TEXTURE0);
        // Without a picture the shader reads nothing (it is multiplied by 0).
        gl.bind_texture(glow::TEXTURE_2D, Some(video.unwrap_or(self.tex[self.cur])));
        let u = |name: &str| gl.get_uniform_location(self.fx, name);
        gl.uniform_1_i32(u("u_video").as_ref(), 0);
        gl.uniform_1_i32(u("u_prev").as_ref(), 1);
        gl.uniform_1_i32(u("u_old").as_ref(), 2);
        gl.uniform_1_i32(u("u_gen").as_ref(), 3);
        gl.uniform_1_i32(u("u_scope").as_ref(), 4);
        gl.uniform_1_f32(u("u_vtime").as_ref(), p.vtime);
        gl.uniform_3_f32(u("u_wave_col").as_ref(), p.wave[0], p.wave[1], p.wave[2]);
        gl.uniform_3_f32(u("u_map_col").as_ref(), p.mapc[0], p.mapc[1], p.mapc[2]);
        gl.uniform_2_f32(u("u_old_size").as_ref(), p.old_size[0].max(1.0), p.old_size[1].max(1.0));
        gl.uniform_1_f32(u("u_old_has").as_ref(), if old.is_some() && p.old_size[0] > 0.0 { 1.0 } else { 0.0 });
        gl.uniform_2_f32(u("u_stage").as_ref(), self.w as f32, self.h as f32);
        gl.uniform_2_f32(u("u_video_size").as_ref(), p.video_size[0].max(1.0), p.video_size[1].max(1.0));
        let has = video.is_some() && p.video_size[0] > 0.0;
        gl.uniform_1_f32(u("u_has_video").as_ref(), if has { 1.0 } else { 0.0 });
        // How each picture sits: moved and turned, its crop, how see-through and whether it fills.
        for (f, place, crop, m) in [(&p.frame, "u_nf", "u_nc", "u_nm"), (&p.old_frame, "u_of", "u_oc", "u_om")] {
            gl.uniform_4_f32(u(place).as_ref(), f.pos[0], f.pos[1], f.zoom, f.turn);
            gl.uniform_4_f32(u(crop).as_ref(), f.crop[0], f.crop[1], f.crop[2], f.crop[3]);
            gl.uniform_2_f32(u(m).as_ref(), f.opacity, if f.fill { 1.0 } else { 0.0 });
        }
        gl.uniform_4_f32_slice(u("u_fx[0]").as_ref(), &packed_amounts(&p.fx));
        let settings: Vec<f32> = p.p.iter().flatten().copied().collect();
        gl.uniform_4_f32_slice(u("u_p[0]").as_ref(), &settings);
        for (name, v) in [("u_spin", p.spin), ("u_seed", p.seed), ("u_grain", p.grain), ("u_transition", p.transition), ("u_mix", p.mix)] {
            gl.uniform_1_f32(u(name).as_ref(), v);
        }
        gl.uniform_2_f32(u("u_shake").as_ref(), p.shake[0], p.shake[1]);
        for (name, c) in [("u_tint", p.tint), ("u_flash", p.flash), ("u_strobe", p.strobe), ("u_edges", p.edges)] {
            gl.uniform_3_f32(u(name).as_ref(), c[0], c[1], c[2]);
        }
        self.quad(gl);
        gl.active_texture(glow::TEXTURE4);
        gl.bind_texture(glow::TEXTURE_2D, None);
        gl.active_texture(glow::TEXTURE3);
        gl.bind_texture(glow::TEXTURE_2D, None);
        gl.active_texture(glow::TEXTURE2);
        gl.bind_texture(glow::TEXTURE_2D, None);
        gl.active_texture(glow::TEXTURE1);
        gl.bind_texture(glow::TEXTURE_2D, None);
        gl.active_texture(glow::TEXTURE0);
        self.cur = next;
    }

    /// The stage, into the viewport set (egui's place for the callback).
    unsafe fn draw_stage(&self, gl: &glow::Context) {
        gl.disable(glow::BLEND);
        gl.use_program(Some(self.show));
        gl.active_texture(glow::TEXTURE0);
        gl.bind_texture(glow::TEXTURE_2D, Some(self.tex[self.cur]));
        gl.uniform_1_i32(gl.get_uniform_location(self.show, "u_stage_tex").as_ref(), 0);
        self.quad(gl);
        gl.bind_texture(glow::TEXTURE_2D, None);
    }
}

// ----------------------------------------------------------------------
// An export's frames on the graphics card
// ----------------------------------------------------------------------

/// A picture of an export's (RGB, top row first).
#[derive(Clone, Debug, Default)]
pub struct Rgb {
    pub w: usize,
    pub h: usize,
    pub px: Vec<u8>,
}

/// A frame an export asks for: its size, the pictures, the effects; and
/// whether it is the film's first (the trails start black).
pub struct Ask {
    pub w: usize,
    pub h: usize,
    pub new: Option<Arc<Rgb>>,
    pub old: Option<Arc<Rgb>>,
    pub params: Params,
    pub first: bool,
}

#[derive(Default)]
struct Frames {
    ask: Option<Ask>,
    done: Option<Result<Vec<u8>, String>>,
    gl: Option<(Gl, [glow::Texture; 2])>,
    failed: Option<String>,
}

/// Frames of an export drawn on the graphics card (an export with Visuals
/// in it: they are drawn nowhere else). The export's thread asks
/// (`draw`), the window's painter draws (`paint`, while the export goes on).
#[derive(Clone, Default)]
pub struct GpuFrames {
    inner: Arc<(Mutex<Frames>, Condvar)>,
}

impl GpuFrames {
    /// Frame `ask` drawn (RGB, top row first), waiting for the window to draw
    /// it; an error when it cannot be, or `stop` says to stop.
    pub fn draw(&self, ask: Ask, stop: &std::sync::atomic::AtomicBool) -> Result<Vec<u8>, String> {
        let (lock, cv) = &*self.inner;
        let mut f = lock.lock().map_err(|_| "lost")?;
        if let Some(e) = &f.failed {
            return Err(e.clone());
        }
        f.ask = Some(ask);
        f.done = None;
        cv.notify_all();
        let mut waited = std::time::Duration::ZERO;
        loop {
            if let Some(done) = f.done.take() {
                return done;
            }
            if stop.load(std::sync::atomic::Ordering::Relaxed) {
                f.ask = None;
                return Err("Export cancelled".into());
            }
            if waited > std::time::Duration::from_secs(60) {
                f.ask = None;
                return Err("The graphics card drew nothing for a minute: keep Krabton's window open (not minimised) while a film with Visuals is exported".into());
            }
            let step = std::time::Duration::from_millis(50);
            f = cv.wait_timeout(f, step).map_err(|_| "lost")?.0;
            waited += step;
        }
    }

    /// While an export goes on: the window's painter draws the frames asked
    /// for (a few each time it paints).
    pub fn paint(&self, painter: &egui::Painter) {
        let inner = self.inner.clone();
        let cb = egui_glow::CallbackFn::new(move |_info, p| {
            let (lock, cv) = &*inner;
            let Ok(mut f) = lock.lock() else { return };
            let gl = p.gl();
            let start = std::time::Instant::now();
            // A few frames a paint, while it is quick.
            while let Some(ask) = f.ask.take() {
                // SAFETY: on the painter's thread with its context current, as egui_glow calls back.
                let done = unsafe { draw_frame(gl, &mut f.gl, &ask) };
                if let Err(e) = &done {
                    f.failed = Some(e.clone());
                }
                f.done = Some(done);
                cv.notify_all();
                // The export's thread asks for the next (while there is time this paint).
                let Some(left) = std::time::Duration::from_millis(12).checked_sub(start.elapsed()) else { break };
                let Ok((g, _)) = cv.wait_timeout_while(f, left, |f| f.ask.is_none() && f.failed.is_none()) else { return };
                f = g;
            }
            // SAFETY: as above.
            unsafe { gl.bind_framebuffer(glow::FRAMEBUFFER, p.intermediate_fbo()) };
        });
        let at = painter.clip_rect().left_top();
        painter.add(egui::PaintCallback { rect: egui::Rect::from_min_size(at, egui::vec2(1.0, 1.0)), callback: Arc::new(cb) });
    }
}

/// Draws one frame of an export (its pipeline made, at its size, the first time).
unsafe fn draw_frame(gl: &glow::Context, slot: &mut Option<(Gl, [glow::Texture; 2])>, ask: &Ask) -> Result<Vec<u8>, String> {
    let (w, h) = (ask.w as i32, ask.h as i32);
    if slot.as_ref().is_none_or(|s| s.0.w != w || s.0.h != h) {
        *slot = Some((Gl::sized(gl, w, h)?, [new_texture(gl, 1, 1)?, new_texture(gl, 1, 1)?]));
    }
    let Some((g, pics)) = slot.as_mut() else { return Err("no graphics".into()) };
    if ask.first {
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(g.fbo));
        for t in g.tex.into_iter().chain(g.milk_tex) {
            gl.framebuffer_texture_2d(glow::FRAMEBUFFER, glow::COLOR_ATTACHMENT0, glow::TEXTURE_2D, Some(t), 0);
            gl.disable(glow::SCISSOR_TEST);
            gl.clear_color(0.0, 0.0, 0.0, 1.0);
            gl.clear(glow::COLOR_BUFFER_BIT);
        }
    }
    // The film's pictures onto the card.
    let up = |k: usize, pic: &Option<Arc<Rgb>>| -> Option<glow::Texture> {
        let pic = pic.as_ref().filter(|p| p.w > 0 && p.h > 0 && p.px.len() >= p.w * p.h * 3)?;
        gl.bind_texture(glow::TEXTURE_2D, Some(pics[k]));
        gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
        gl.tex_image_2d(glow::TEXTURE_2D, 0, glow::RGB as i32, pic.w as i32, pic.h as i32, 0, glow::RGB, glow::UNSIGNED_BYTE, glow::PixelUnpackData::Slice(Some(&pic.px)));
        gl.bind_texture(glow::TEXTURE_2D, None);
        Some(pics[k])
    };
    let new = up(0, &ask.new);
    let old = up(1, &ask.old);
    g.draw_effects(gl, new, old, &ask.params);
    let mut rgba = vec![0u8; ask.w * ask.h * 4];
    gl.pixel_store_i32(glow::PACK_ALIGNMENT, 1);
    gl.read_pixels(0, 0, w, h, glow::RGBA, glow::UNSIGNED_BYTE, glow::PixelPackData::Slice(Some(&mut rgba)));
    Ok(rgb_top_first(&rgba, ask.w, ask.h))
}

/// RGBA read back (bottom row first) as RGB, top row first.
pub fn rgb_top_first(rgba: &[u8], w: usize, h: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(w * h * 3);
    for y in (0..h).rev() {
        for px in rgba[y * w * 4..(y + 1) * w * 4].chunks_exact(4) {
            out.extend_from_slice(&px[..3]);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_frame_read_back_is_turned_the_right_way_up() {
        // Two rows read back: the bottom one (blue) first.
        let rgba = [0, 0, 255, 255, 0, 0, 255, 255, 255, 0, 0, 255, 255, 0, 0, 255];
        assert_eq!(rgb_top_first(&rgba, 2, 2), [255, 0, 0, 255, 0, 0, 0, 0, 255, 0, 0, 255]);
    }

    #[test]
    fn the_visuals_morph_their_colour_and_show_when_up() {
        let mut a = Params::default();
        let b = Params { vcol: [0.0, 0.0, 1.0], ..Default::default() };
        a.vcol = [1.0, 0.0, 0.0];
        assert_eq!(Params::morph(&a, &b, 0.5).vcol, [0.5, 0.0, 0.5]);
        assert!(!a.visuals_on());
        a.fx[Effect::Visuals.index()] = 0.5;
        assert!(a.visuals_on());
        assert!(defines().contains("#define A_VISUALS") && defines().contains("#define P_FRACTURE"));
    }
}

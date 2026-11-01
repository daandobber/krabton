//! Drawing the Video on the graphics card. Once a frame, one shader puts
//! every effect on the picture into the *stage* (a picture of its own, 1280
//! by 720, which also keeps the last frame for Trails and Echo); every
//! place the Video shows (its window, the screen) then draws the stage. It
//! runs in egui's OpenGL painter, through a paint callback. While the Video
//! is recorded, the stage is read back after it is drawn.

use eframe::egui_glow;
use eframe::glow::{self, HasContext};
use krabton_engine::video::{Effect, FxSetting};
use std::sync::{Arc, Mutex};

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

    pub fn get(&self, e: Effect) -> f32 {
        self.fx[e.index()]
    }

    /// Effect `e`'s setting `k`.
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
        out
    }
}

/// The shader's names for the effects' amounts and settings, by their
/// place: `A_ZOOM` how much Zoom, `P_ZOOM` its settings (a vec4).
fn defines() -> String {
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
vec3 pick(vec2 st) {
    if (u_transition < 0.5) { return pick_new(st); }
    float m = clamp(u_mix, 0.0, 1.0);
    if (u_transition < 1.5) { return mix(pick_old(st), pick_new(st), m); }
    float peak = 1.0 - abs(m * 2.0 - 1.0);
    if (u_transition < 2.5) {
        vec3 c = m < 0.5 ? pick_old(st) : pick_new(st);
        return mix(c, vec3(1.0), peak);
    }
    // Glitch: strips of the new clip, more and more, pushed about.
    float band = floor(st.y * 24.0 + hash(u_seed) * 3.0);
    vec2 q = st + vec2((hash(band * 1.31 + u_seed) - 0.5) * 0.3 * peak, 0.0);
    return hash(band * 7.13 + floor(u_seed)) < m ? pick_new(q) : pick_old(q);
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

    /// The stage last read back (RGBA, `STAGE_W` by `STAGE_H`, bottom row
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
            if st.gl.is_none() {
                // SAFETY: on the painter's thread with its context current, as egui_glow calls back.
                match unsafe { Gl::new(gl) } {
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
                        let mut px = vec![0u8; (STAGE_W * STAGE_H * 4) as usize];
                        gl.read_pixels(0, 0, STAGE_W, STAGE_H, glow::RGBA, glow::UNSIGNED_BYTE, glow::PixelPackData::Slice(Some(&mut px)));
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

unsafe fn program(gl: &glow::Context, frag: &str) -> Result<glow::Program, String> {
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

impl Gl {
    unsafe fn new(gl: &glow::Context) -> Result<Self, String> {
        let fx = program(gl, &format!("{}{EFFECTS}", defines()))?;
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
            gl.tex_image_2d(glow::TEXTURE_2D, 0, glow::RGBA as i32, STAGE_W, STAGE_H, 0, glow::RGBA, glow::UNSIGNED_BYTE, glow::PixelUnpackData::Slice(None));
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
        Ok(Self { fx, show, quad, vao, fbo, tex, cur: 0 })
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
        let next = 1 - self.cur;
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(self.fbo));
        gl.framebuffer_texture_2d(glow::FRAMEBUFFER, glow::COLOR_ATTACHMENT0, glow::TEXTURE_2D, Some(self.tex[next]), 0);
        gl.viewport(0, 0, STAGE_W, STAGE_H);
        gl.disable(glow::SCISSOR_TEST);
        gl.disable(glow::BLEND);
        gl.use_program(Some(self.fx));
        gl.active_texture(glow::TEXTURE1);
        gl.bind_texture(glow::TEXTURE_2D, Some(self.tex[self.cur]));
        gl.active_texture(glow::TEXTURE2);
        gl.bind_texture(glow::TEXTURE_2D, Some(old.unwrap_or(self.tex[self.cur])));
        gl.active_texture(glow::TEXTURE0);
        // Without a picture the shader reads nothing (it is multiplied by 0).
        gl.bind_texture(glow::TEXTURE_2D, Some(video.unwrap_or(self.tex[self.cur])));
        let u = |name: &str| gl.get_uniform_location(self.fx, name);
        gl.uniform_1_i32(u("u_video").as_ref(), 0);
        gl.uniform_1_i32(u("u_prev").as_ref(), 1);
        gl.uniform_1_i32(u("u_old").as_ref(), 2);
        gl.uniform_2_f32(u("u_old_size").as_ref(), p.old_size[0].max(1.0), p.old_size[1].max(1.0));
        gl.uniform_1_f32(u("u_old_has").as_ref(), if old.is_some() && p.old_size[0] > 0.0 { 1.0 } else { 0.0 });
        gl.uniform_2_f32(u("u_stage").as_ref(), STAGE_W as f32, STAGE_H as f32);
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

//! The blaze behind and before Inferno's face, on the graphics card. Once a
//! frame one shader burns a small picture of the fire (tongues of a hot-rod
//! flame job: white-hot and yellow at the root, through orange and red to a
//! pink tip, with black licks through them; a column erupts where a step
//! plays). The face then shows that picture three ways: *back*, the whole
//! panel, a charred wall with a burning house's beams in front of the fire
//! and smoke rolling over it; *window*, a step that is on, a hole in the wall
//! with the fire raging behind it; and *front*, the flames licking up over
//! the controls at the bottom. The picture is small, so it costs the same on
//! any size of window.

use eframe::egui_glow;
use eframe::glow::{self, HasContext};
use egui::{Painter, Rect};
use std::sync::{Arc, Mutex, OnceLock};

const W: i32 = 512;
const H: i32 = 320;
/// Columns of the grid that can erupt.
pub const COLUMNS: usize = 16;

const BURN: &str = "
uniform float u_t;
uniform float u_aspect;
uniform float u_heat;
uniform float u_cols[16];
uniform vec2 u_grid;
I vec2 v_uv;
float hash(vec2 p) {
    p = fract(p * vec2(123.34, 456.21));
    p += dot(p, p + 45.32);
    return fract(p.x * p.y);
}
float noise(vec2 p) {
    vec2 i = floor(p);
    vec2 f = fract(p);
    f = f * f * (3.0 - 2.0 * f);
    return mix(mix(hash(i), hash(i + vec2(1.0, 0.0)), f.x), mix(hash(i + vec2(0.0, 1.0)), hash(i + vec2(1.0, 1.0)), f.x), f.y);
}
float fbm(vec2 p) {
    float a = 0.5;
    float s = 0.0;
    for (int k = 0; k < 4; k++) {
        s += a * noise(p);
        p = p * 2.03 + vec2(17.0, 9.0);
        a *= 0.5;
    }
    return s;
}
// One sheet of flame: how far inside it this point is (0 at the edge).
float sheet(vec2 q, float t, float scale, float speed, float reach) {
    float bend = fbm(vec2(q.x * 1.7 + t * 0.2, q.y * 1.3 - t * 0.9));
    float x = q.x + (bend - 0.5) * 0.55 * (q.y + 0.25);
    // Long upright tongues: noise stretched high, rushing up.
    float n = fbm(vec2(x * scale, q.y * 0.9 - t * speed));
    float sharp = 1.0 - abs(2.0 * noise(vec2(x * scale * 1.6, q.y * 0.55 - t * speed * 1.3)) - 1.0);
    float tongues = smoothstep(0.3, 0.72, n) * 0.85 + sharp * sharp * sharp * 0.42;
    return tongues * reach - q.y;
}
void main() {
    vec2 q = vec2(v_uv.x * u_aspect, 1.0 - v_uv.y);
    float t = u_t;
    // Where a step plays its column erupts.
    float erupt = 0.0;
    float g = (v_uv.x - u_grid.x) / max(u_grid.y - u_grid.x, 0.001) * 16.0;
    for (int i = 0; i < 16; i++) {
        float d = g - (float(i) + 0.5);
        erupt += u_cols[i] * exp(-d * d * 0.9);
    }
    float reach = (0.34 + 1.0 * u_heat) * (1.0 + 0.8 * min(erupt, 1.5));
    float rear = sheet(q + vec2(3.1, 0.0), t * 0.8, 2.2, 0.9, reach * 1.2);
    float mid = sheet(q + vec2(7.7, 0.0), t, 3.1, 1.25, reach);
    float lead = sheet(q, t * 1.15, 4.3, 1.7, reach * 0.72);
    // The flame job's colours, by how deep inside: pink tip, red, orange, yellow, white.
    float depth = max(max(rear * 0.42, mid * 0.75), lead * 1.1);
    vec3 pink = vec3(1.0, 0.2, 0.34);
    vec3 red = vec3(0.92, 0.05, 0.03);
    vec3 orange = vec3(1.0, 0.36, 0.0);
    vec3 yellow = vec3(1.0, 0.78, 0.05);
    vec3 white = vec3(1.0, 0.97, 0.72);
    vec3 c = mix(pink, red, smoothstep(0.0, 0.045, depth));
    c = mix(c, orange, smoothstep(0.05, 0.17, depth));
    c = mix(c, yellow, smoothstep(0.17, 0.36, depth));
    c = mix(c, white, smoothstep(0.62, 1.0, depth) * 0.75);
    float a = smoothstep(0.0, 0.012, depth);
    // Black licks through it, sharp at both ends.
    float lx = q.x + (fbm(vec2(q.x * 2.0, q.y * 1.5 - t * 1.1)) - 0.5) * 0.5;
    float lick = noise(vec2(lx * 7.5, q.y * 1.15 - t * 1.5));
    float cut = smoothstep(0.66, 0.7, lick) * smoothstep(0.02, 0.1, depth);
    a *= 1.0 - cut;
    gl_FragColor = vec4(c * a, a);
}
";

const SHOW: &str = "
uniform sampler2D u_fire;
uniform vec4 u_panel;
uniform float u_mode;
uniform float u_gain;
uniform float u_t;
uniform float u_heat;
float hash(vec2 p) {
    p = fract(p * vec2(123.34, 456.21));
    p += dot(p, p + 45.32);
    return fract(p.x * p.y);
}
float noise(vec2 p) {
    vec2 i = floor(p);
    vec2 f = fract(p);
    f = f * f * (3.0 - 2.0 * f);
    return mix(mix(hash(i), hash(i + vec2(1.0, 0.0)), f.x), mix(hash(i + vec2(0.0, 1.0)), hash(i + vec2(1.0, 1.0)), f.x), f.y);
}
float beam(vec2 p, vec2 a, vec2 b, float w) {
    vec2 ba = b - a;
    float h = clamp(dot(p - a, ba) / dot(ba, ba), 0.0, 1.0);
    // Charred: its edge bites in and out.
    float bite = noise(p * 42.0) * 0.005;
    return length(p - a - ba * h) - w - bite;
}
// The burnt-out house: posts, rafters, a broken floor, a beam coming down.
float house(vec2 p, float aspect) {
    float x0 = aspect * 0.08;
    float x1 = aspect * 0.92;
    float mid = aspect * 0.5;
    float eave = 0.58;
    float apex = 0.97;
    float d = beam(p, vec2(x0, 0.0), vec2(x0, eave), 0.016);
    d = min(d, beam(p, vec2(x1, 0.0), vec2(x1, eave), 0.016));
    d = min(d, beam(p, vec2(aspect * 0.36, 0.0), vec2(aspect * 0.36, 0.44), 0.012));
    d = min(d, beam(p, vec2(aspect * 0.67, 0.0), vec2(aspect * 0.67, 0.5), 0.012));
    d = min(d, beam(p, vec2(x0 - 0.05, eave - 0.05), vec2(mid, apex), 0.018));
    d = min(d, beam(p, vec2(mid + aspect * 0.13, apex - 0.1), vec2(x1 + 0.05, eave - 0.05), 0.018));
    d = min(d, beam(p, vec2(aspect * 0.42, 0.72), vec2(aspect * 0.6, 0.12), 0.016));
    return d;
}
void main() {
    vec2 uv = (gl_FragCoord.xy - u_panel.xy) / u_panel.zw;
    float aspect = u_panel.z / u_panel.w;
    if (u_mode < 0.5) {
        // Back: the wall, the fire, the house in front of it, the smoke over it.
        vec2 p = vec2(uv.x * aspect, uv.y);
        vec4 f = texture2D(u_fire, uv);
        // Black gloss, charred into cracks that glow where it is hot.
        float crack = 1.0 - abs(2.0 * noise(p * vec2(9.0, 7.0) + vec2(3.0, 0.0)) - 1.0);
        float glow = smoothstep(0.93, 0.995, crack) * (0.25 + 0.75 * u_heat) * (1.0 - uv.y * 0.6);
        vec3 wall = vec3(0.012, 0.008, 0.008) + vec3(0.9, 0.16, 0.02) * glow;
        // A light from the fire on the gloss.
        wall += vec3(0.1, 0.022, 0.006) * (1.0 - uv.y) * (0.5 + u_heat);
        vec3 c = wall * (1.0 - f.a) + f.rgb;
        float d = house(p, aspect);
        float solid = 1.0 - smoothstep(0.0, 0.004, d);
        float rim = (1.0 - smoothstep(0.0, 0.016, abs(d))) * f.a;
        c = mix(c, vec3(0.008, 0.005, 0.005), solid);
        c += vec3(1.0, 0.3, 0.03) * rim * 0.55;
        // Smoke rolling across the top, lit from under.
        float s = noise(vec2(p.x * 2.1 - u_t * 0.14, uv.y * 3.0 - u_t * 0.3));
        s = s * 0.6 + 0.4 * noise(vec2(p.x * 5.0 + u_t * 0.1, uv.y * 7.0 - u_t * 0.5));
        float over = smoothstep(0.52, 0.98, uv.y + (s - 0.5) * 0.45);
        c = mix(c, vec3(0.03, 0.022, 0.02) + vec3(0.12, 0.03, 0.0) * s * (1.0 - uv.y), over * 0.88);
        // The edges burnt dark.
        vec2 e = min(uv, 1.0 - uv) * vec2(aspect, 1.0);
        c *= 0.45 + 0.55 * smoothstep(0.0, 0.05, min(e.x, e.y));
        gl_FragColor = vec4(c, 1.0);
    } else if (u_mode < 1.5) {
        // A window: the root of the fire behind it, however low the fire is.
        vec2 q = vec2(uv.x, uv.y * 0.5);
        vec4 f = texture2D(u_fire, q);
        vec3 c = vec3(0.45, 0.03, 0.0) * (1.0 - f.a) + f.rgb;
        gl_FragColor = vec4(min(c * u_gain, vec3(1.0)), 1.0);
    } else {
        // In front: the tongues over the controls, low down, adding light.
        vec4 f = texture2D(u_fire, uv);
        float m = (1.0 - smoothstep(0.0, 0.1 + 0.14 * u_heat, uv.y)) * u_gain;
        gl_FragColor = vec4(f.rgb * m, f.a * m * 0.5);
    }
}
";

struct Gl {
    burn: glow::Program,
    show: glow::Program,
    quad: glow::Buffer,
    vao: Option<glow::VertexArray>,
    fbo: glow::Framebuffer,
    tex: glow::Texture,
}

#[derive(Default)]
struct State {
    gl: Option<Gl>,
    failed: bool,
    burnt: u64,
}

fn state() -> &'static Arc<Mutex<State>> {
    static STATE: OnceLock<Arc<Mutex<State>>> = OnceLock::new();
    STATE.get_or_init(Default::default)
}

/// Can the fire be drawn here? (Otherwise the face draws its own, smaller one.)
pub fn works() -> bool {
    state().lock().map(|s| !s.failed).unwrap_or(false)
}

/// How the fire burns this frame.
#[derive(Clone, Copy)]
pub struct Blaze {
    pub time: f32,
    pub heat: f32,
    /// The grid's left and right, as parts of the panel's width.
    pub grid: [f32; 2],
    /// How much each column of the grid erupts.
    pub cols: [f32; COLUMNS],
    pub frame: u64,
}

#[derive(Clone, Copy, PartialEq)]
enum Pass {
    Back,
    Window,
    Front,
}

impl Gl {
    unsafe fn new(gl: &glow::Context) -> Result<Self, String> {
        let burn = crate::video::stage::program(gl, BURN)?;
        let show = crate::video::stage::program(gl, SHOW)?;
        let quad = gl.create_buffer()?;
        let corners: [f32; 8] = [-1.0, -1.0, 1.0, -1.0, -1.0, 1.0, 1.0, 1.0];
        let bytes: Vec<u8> = corners.iter().flat_map(|v| v.to_ne_bytes()).collect();
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(quad));
        gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, &bytes, glow::STATIC_DRAW);
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
        let tex = gl.create_texture()?;
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(fbo));
        gl.bind_texture(glow::TEXTURE_2D, Some(tex));
        gl.tex_image_2d(glow::TEXTURE_2D, 0, glow::RGBA as i32, W, H, 0, glow::RGBA, glow::UNSIGNED_BYTE, glow::PixelUnpackData::Slice(None));
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::LINEAR as i32);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::LINEAR as i32);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE as i32);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE as i32);
        gl.framebuffer_texture_2d(glow::FRAMEBUFFER, glow::COLOR_ATTACHMENT0, glow::TEXTURE_2D, Some(tex), 0);
        let complete = gl.check_framebuffer_status(glow::FRAMEBUFFER) == glow::FRAMEBUFFER_COMPLETE;
        gl.bind_texture(glow::TEXTURE_2D, None);
        gl.bind_framebuffer(glow::FRAMEBUFFER, None);
        if !complete {
            return Err("the fire's picture could not be made".into());
        }
        Ok(Self { burn, show, quad, vao, fbo, tex })
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

    /// This frame's fire into its picture.
    unsafe fn burn(&self, gl: &glow::Context, b: &Blaze, aspect: f32) {
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(self.fbo));
        gl.viewport(0, 0, W, H);
        gl.disable(glow::SCISSOR_TEST);
        gl.disable(glow::BLEND);
        gl.use_program(Some(self.burn));
        let at = |name: &str| gl.get_uniform_location(self.burn, name);
        gl.uniform_1_f32(at("u_t").as_ref(), b.time % 1000.0);
        gl.uniform_1_f32(at("u_aspect").as_ref(), aspect);
        gl.uniform_1_f32(at("u_heat").as_ref(), b.heat);
        gl.uniform_2_f32(at("u_grid").as_ref(), b.grid[0], b.grid[1]);
        gl.uniform_1_f32_slice(at("u_cols").as_ref(), &b.cols);
        self.quad(gl);
    }

    unsafe fn show(&self, gl: &glow::Context, pass: Pass, panel: [f32; 4], gain: f32, b: &Blaze) {
        match pass {
            Pass::Front => {
                gl.enable(glow::BLEND);
                gl.blend_func(glow::ONE, glow::ONE_MINUS_SRC_ALPHA);
            }
            _ => gl.disable(glow::BLEND),
        }
        gl.use_program(Some(self.show));
        let at = |name: &str| gl.get_uniform_location(self.show, name);
        gl.active_texture(glow::TEXTURE0);
        gl.bind_texture(glow::TEXTURE_2D, Some(self.tex));
        gl.uniform_1_i32(at("u_fire").as_ref(), 0);
        gl.uniform_4_f32(at("u_panel").as_ref(), panel[0], panel[1], panel[2], panel[3]);
        gl.uniform_1_f32(at("u_mode").as_ref(), pass as u8 as f32);
        gl.uniform_1_f32(at("u_gain").as_ref(), gain);
        gl.uniform_1_f32(at("u_t").as_ref(), b.time % 1000.0);
        gl.uniform_1_f32(at("u_heat").as_ref(), b.heat);
        self.quad(gl);
        gl.bind_texture(glow::TEXTURE_2D, None);
    }
}

fn paint(painter: &Painter, rect: Rect, panel: Rect, pass: Pass, gain: f32, blaze: Blaze) {
    let shared = state().clone();
    let cb = egui_glow::CallbackFn::new(move |info, p| {
        let Ok(mut st) = shared.lock() else { return };
        if st.failed {
            return;
        }
        let gl = p.gl();
        if st.gl.is_none() {
            // SAFETY: on the painter's thread with its context current, as egui_glow calls back.
            match unsafe { Gl::new(gl) } {
                Ok(g) => st.gl = Some(g),
                Err(e) => {
                    eprintln!("inferno: {e}");
                    st.failed = true;
                    return;
                }
            }
        }
        let fresh = st.burnt != blaze.frame;
        st.burnt = blaze.frame;
        let Some(g) = st.gl.as_ref() else { return };
        let ppp = info.pixels_per_point;
        let screen_h = info.screen_size_px[1] as f32;
        // The panel in the frame buffer's pixels, from its bottom left.
        let at = [panel.left() * ppp, screen_h - panel.bottom() * ppp, panel.width() * ppp, panel.height() * ppp];
        // SAFETY: as above; every object used was made on this context.
        unsafe {
            if fresh {
                g.burn(gl, &blaze, panel.width() / panel.height().max(1.0));
                gl.bind_framebuffer(glow::FRAMEBUFFER, p.intermediate_fbo());
            }
            let vp = info.viewport_in_pixels();
            gl.viewport(vp.left_px, vp.from_bottom_px, vp.width_px, vp.height_px);
            let clip = info.clip_rect_in_pixels();
            gl.enable(glow::SCISSOR_TEST);
            gl.scissor(clip.left_px, clip.from_bottom_px, clip.width_px, clip.height_px);
            g.show(gl, pass, at, gain, &blaze);
        }
    });
    painter.add(egui::PaintCallback { rect, callback: Arc::new(cb) });
}

/// The whole panel on fire: first, under everything on it.
pub fn back(painter: &Painter, panel: Rect, blaze: Blaze) {
    paint(painter, panel, panel, Pass::Back, 1.0, blaze);
}

/// A hole in the wall at `rect`, the fire behind it; `gain` how fierce.
pub fn window(painter: &Painter, rect: Rect, panel: Rect, gain: f32, blaze: Blaze) {
    paint(painter, rect, panel, Pass::Window, gain, blaze);
}

/// The flames over the controls: last, over everything on the panel.
pub fn front(painter: &Painter, panel: Rect, gain: f32, blaze: Blaze) {
    paint(painter, panel, panel, Pass::Front, gain, blaze);
}

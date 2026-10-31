//! Drawing the Video on the graphics card. Once a frame, one shader puts
//! every effect on the picture into the *stage* (a picture of its own, 1280
//! by 720, which also keeps the last frame for Trails); every place the
//! Video shows (its window, the screen) then draws the stage. It runs in
//! egui's OpenGL painter, through a paint callback.

use eframe::egui_glow;
use eframe::glow::{self, HasContext};
use std::sync::{Arc, Mutex};

pub const STAGE_W: i32 = 1280;
pub const STAGE_H: i32 = 720;

/// Everything the effect shader needs for one frame (amounts 0..1).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Params {
    /// The picture's size in pixels (0: no picture).
    pub video_size: [f32; 2],
    pub fill: bool,
    pub zoom: f32,
    /// Turns (1: all the way round).
    pub spin: f32,
    /// Where the picture is shaken to (fractions of the stage).
    pub shake: [f32; 2],
    pub mirror: f32,
    pub kaleido: f32,
    pub pixels: f32,
    pub split: f32,
    pub glitch: f32,
    /// Changes every frame, so the glitch's strips move.
    pub seed: f32,
    /// Turns round the colour circle.
    pub hue: f32,
    pub poster: f32,
    pub flash: f32,
    pub invert: f32,
    pub strobe: f32,
    pub gate: f32,
    pub trails: f32,
    /// A colour (0..1) and how much of it.
    pub tint: [f32; 4],
}

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

#[derive(Default)]
struct State {
    gl: Option<Gl>,
    /// It could not be made (old OpenGL): the window shows the plain picture.
    failed: Option<String>,
    /// The frame the stage was last drawn for.
    drawn: u64,
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

    /// Draws the stage into `rect` (the effects are worked out the first
    /// time it is drawn in frame `frame`), from the picture in `video`.
    pub fn paint(&self, painter: &egui::Painter, rect: egui::Rect, video: Option<egui::TextureId>, params: Params, frame: u64) {
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
            let Some(g) = st.gl.as_mut() else { return };
            let video_tex = video.and_then(|id| p.texture(id));
            // SAFETY: as above; every object used was made on this context.
            unsafe {
                if fresh {
                    g.draw_effects(gl, video_tex, &params);
                    gl.bind_framebuffer(glow::FRAMEBUFFER, p.intermediate_fbo());
                }
                let vp = info.viewport_in_pixels();
                gl.viewport(vp.left_px, vp.from_bottom_px, vp.width_px, vp.height_px);
                let clip = info.clip_rect_in_pixels();
                gl.enable(glow::SCISSOR_TEST);
                gl.scissor(clip.left_px, clip.from_bottom_px, clip.width_px, clip.height_px);
                g.draw_stage(gl);
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

const VERTEX: &str = "
I vec2 a_pos;
O vec2 v_uv;
void main() {
    // 0,0 at the top left, as egui's pictures are.
    v_uv = vec2(a_pos.x * 0.5 + 0.5, 0.5 - a_pos.y * 0.5);
    gl_Position = vec4(a_pos, 0.0, 1.0);
}
";

const EFFECTS: &str = "
uniform sampler2D u_video;
uniform sampler2D u_prev;
uniform vec2 u_stage;
uniform vec2 u_video_size;
uniform float u_has_video;
uniform float u_fill;
uniform float u_zoom;
uniform float u_spin;
uniform vec2 u_shake;
uniform float u_mirror;
uniform float u_kaleido;
uniform float u_pixels;
uniform float u_split;
uniform float u_glitch;
uniform float u_seed;
uniform float u_hue;
uniform float u_poster;
uniform float u_flash;
uniform float u_invert;
uniform float u_strobe;
uniform float u_gate;
uniform float u_trails;
uniform vec4 u_tint;
I vec2 v_uv;

float hash(float n) { return fract(sin(n) * 43758.5453); }

// The picture at a point of the stage (0..1 both ways): filling the stage
// (cut at the sides) or fitting in it (black bars).
vec3 pick(vec2 st) {
    float sa = u_stage.x / u_stage.y;
    float va = u_video_size.x / max(u_video_size.y, 1.0);
    vec2 k = vec2(1.0);
    if ((u_fill > 0.5) == (va > sa)) { k.x = sa / va; } else { k.y = va / sa; }
    vec2 uv = (st - 0.5) * k + 0.5;
    float inside = step(0.0, uv.x) * step(uv.x, 1.0) * step(0.0, uv.y) * step(uv.y, 1.0);
    return texture2D(u_video, clamp(uv, 0.0, 1.0)).rgb * inside * u_has_video;
}

// A point around the middle (x in the stage's proportion), with red and blue pulled apart.
vec3 at(vec2 p) {
    float aspect = u_stage.x / u_stage.y;
    vec2 st = vec2(p.x / aspect, p.y) + 0.5;
    float off = u_split * 0.03;
    if (off < 0.0005) { return pick(st); }
    return vec3(pick(st + vec2(off, 0.0)).r, pick(st).g, pick(st - vec2(off, 0.0)).b);
}

vec3 hue_turn(vec3 c, float a) {
    vec3 k = vec3(0.57735);
    float ca = cos(a);
    return c * ca + cross(k, c) * sin(a) + k * dot(k, c) * (1.0 - ca);
}

void main() {
    vec2 st = v_uv;
    // Glitch: strips pushed sideways, different every frame.
    if (u_glitch > 0.001) {
        float band = floor(st.y * 18.0 + hash(u_seed) * 5.0);
        if (hash(band * 3.17 + u_seed * 11.0) < u_glitch * 0.7) {
            st.x += (hash(band * 1.71 + u_seed) - 0.5) * 0.4 * u_glitch;
        }
    }
    // Pixels: blocks up to 40 stage pixels.
    if (u_pixels > 0.001) {
        float px = mix(1.0, 40.0, u_pixels * u_pixels);
        st = (floor(st * u_stage / px) + 0.5) * px / u_stage;
    }
    float aspect = u_stage.x / u_stage.y;
    vec2 p = st - 0.5;
    p.x *= aspect;
    p /= 1.0 + u_zoom * 1.5;
    float a = u_spin * 6.2831853;
    p = vec2(cos(a) * p.x - sin(a) * p.y, sin(a) * p.x + cos(a) * p.y);
    p -= u_shake * vec2(aspect, 1.0);
    vec3 c = at(p);
    // Mirror: the left half onto the right.
    if (u_mirror > 0.001) { c = mix(c, at(vec2(-abs(p.x), p.y)), u_mirror); }
    // Kaleido: six mirrored wedges.
    if (u_kaleido > 0.001) {
        float seg = 6.2831853 / 6.0;
        float ang = mod(atan(p.y, p.x), seg);
        ang = abs(ang - seg * 0.5);
        c = mix(c, at(vec2(cos(ang), sin(ang)) * length(p)), u_kaleido);
    }
    if (u_hue > 0.001) { c = clamp(hue_turn(c, u_hue * 6.2831853), 0.0, 1.0); }
    if (u_poster > 0.001) {
        float levels = mix(24.0, 2.0, u_poster);
        c = floor(c * levels + 0.5) / levels;
    }
    float luma = dot(c, vec3(0.299, 0.587, 0.114));
    c = mix(c, u_tint.rgb * (0.2 + luma * 1.3), u_tint.a);
    c = mix(c, 1.0 - c, u_invert);
    c = mix(c, vec3(1.0), u_flash * 0.85);
    c *= 1.0 - u_gate;
    c = mix(c, vec3(1.0), u_strobe);
    // Trails: the last frame, drawn a little closer, fading under this one.
    if (u_trails > 0.001) {
        vec2 q = (v_uv - 0.5) * 0.985 + 0.5;
        vec3 prev = texture2D(u_prev, vec2(q.x, 1.0 - q.y)).rgb;
        c = max(c, mix(c, prev, u_trails * 0.9));
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
        let fx = program(gl, EFFECTS)?;
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
    unsafe fn draw_effects(&mut self, gl: &glow::Context, video: Option<glow::Texture>, p: &Params) {
        let next = 1 - self.cur;
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(self.fbo));
        gl.framebuffer_texture_2d(glow::FRAMEBUFFER, glow::COLOR_ATTACHMENT0, glow::TEXTURE_2D, Some(self.tex[next]), 0);
        gl.viewport(0, 0, STAGE_W, STAGE_H);
        gl.disable(glow::SCISSOR_TEST);
        gl.disable(glow::BLEND);
        gl.use_program(Some(self.fx));
        gl.active_texture(glow::TEXTURE1);
        gl.bind_texture(glow::TEXTURE_2D, Some(self.tex[self.cur]));
        gl.active_texture(glow::TEXTURE0);
        // Without a picture the shader reads nothing (it is multiplied by 0).
        gl.bind_texture(glow::TEXTURE_2D, Some(video.unwrap_or(self.tex[self.cur])));
        let u = |name: &str| gl.get_uniform_location(self.fx, name);
        gl.uniform_1_i32(u("u_video").as_ref(), 0);
        gl.uniform_1_i32(u("u_prev").as_ref(), 1);
        gl.uniform_2_f32(u("u_stage").as_ref(), STAGE_W as f32, STAGE_H as f32);
        gl.uniform_2_f32(u("u_video_size").as_ref(), p.video_size[0].max(1.0), p.video_size[1].max(1.0));
        let has = video.is_some() && p.video_size[0] > 0.0;
        gl.uniform_1_f32(u("u_has_video").as_ref(), if has { 1.0 } else { 0.0 });
        gl.uniform_1_f32(u("u_fill").as_ref(), if p.fill { 1.0 } else { 0.0 });
        for (name, v) in [
            ("u_zoom", p.zoom),
            ("u_spin", p.spin),
            ("u_mirror", p.mirror),
            ("u_kaleido", p.kaleido),
            ("u_pixels", p.pixels),
            ("u_split", p.split),
            ("u_glitch", p.glitch),
            ("u_seed", p.seed),
            ("u_hue", p.hue),
            ("u_poster", p.poster),
            ("u_flash", p.flash),
            ("u_invert", p.invert),
            ("u_strobe", p.strobe),
            ("u_gate", p.gate),
            ("u_trails", p.trails),
        ] {
            gl.uniform_1_f32(u(name).as_ref(), v);
        }
        gl.uniform_2_f32(u("u_shake").as_ref(), p.shake[0], p.shake[1]);
        gl.uniform_4_f32(u("u_tint").as_ref(), p.tint[0], p.tint[1], p.tint[2], p.tint[3]);
        self.quad(gl);
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

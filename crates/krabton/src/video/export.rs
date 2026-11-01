//! Export Video: the set's Video as a film (MP4) with the song's sound, as
//! it would have played. The song is rendered offline as Export Audio does,
//! stopping at every frame of the film to let the Video's reckoning (the
//! same `Director` the window uses) hear it, follow its notes and beat, and
//! say which clip shows where and how the links move it. Then every frame
//! is drawn by a copy of the stage's shader that runs on the processor, and
//! ffmpeg makes the film of the frames and the sound.

use super::director::{Asks, Director, Shot};
use super::media::Media;
use super::stage::Params;
use krabton_engine::project::Project;
use krabton_engine::render::{self, RenderProgress, RenderSource};
use krabton_engine::video::VideoSet;
use std::ffi::OsString;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// The film's sizes on offer (16:9).
pub const SIZES: [(u32, u32); 3] = [(854, 480), (1280, 720), (1920, 1080)];
/// Frames a second on offer.
pub const RATES: [u32; 4] = [24, 25, 30, 60];

/// How the film is made.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Opts {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
}

impl Default for Opts {
    fn default() -> Self {
        Self { width: 1280, height: 720, fps: 30 }
    }
}

/// How many frames a song of `samples` sample frames at `sample_rate` makes at `fps`.
pub fn frame_count(samples: usize, sample_rate: u32, fps: u32) -> usize {
    (samples as f64 * fps as f64 / sample_rate.max(1) as f64).floor() as usize
}

/// The sound under frame `k` (its first sample frame and the one after its
/// last): each frame as long as its share, so a long song does not drift.
/// (`render::render_watched` stops at the same places.)
#[cfg(test)]
pub fn frame_samples(k: usize, sample_rate: u32, fps: u32) -> (usize, usize) {
    let every = sample_rate as f64 / fps.max(1) as f64;
    ((k as f64 * every).round() as usize, ((k + 1) as f64 * every).round() as usize)
}

/// The arguments that make ffmpeg a film of raw RGB frames on its input
/// (`width` by `height`, `fps` a second) and the sound in `wav`: H.264
/// (or `encoder`) in yuv420p for every player, AAC sound, ready to stream.
pub fn encode_args(width: u32, height: u32, fps: u32, wav: &Path, out: &Path, encoder: &str) -> Vec<OsString> {
    let mut a: Vec<OsString> = Vec::new();
    let mut push = |s: &str| a.push(s.into());
    for s in ["-hide_banner", "-loglevel", "error", "-nostdin", "-y", "-f", "rawvideo", "-pix_fmt", "rgb24"] {
        push(s);
    }
    push("-s");
    push(&format!("{width}x{height}"));
    push("-r");
    push(&fps.to_string());
    push("-i");
    push("-");
    a.push("-i".into());
    a.push(wav.as_os_str().to_owned());
    let mut push = |s: &str| a.push(s.into());
    for s in ["-map", "0:v", "-map", "1:a", "-c:v", encoder] {
        push(s);
    }
    if encoder == "libx264" {
        for s in ["-preset", "medium", "-crf", "18"] {
            push(s);
        }
    } else {
        for s in ["-q:v", "3"] {
            push(s);
        }
    }
    for s in ["-pix_fmt", "yuv420p", "-c:a", "aac", "-b:a", "256k", "-movflags", "+faststart"] {
        push(s);
    }
    a.push(out.as_os_str().to_owned());
    a
}

/// The film encoder this ffmpeg has: H.264 when it can, else MPEG-4.
pub fn encoder(exe: &Path) -> &'static str {
    let listed = super::ffmpeg::command(exe).args(["-hide_banner", "-encoders"]).stdin(std::process::Stdio::null()).stderr(std::process::Stdio::null()).output();
    match listed {
        Ok(o) if String::from_utf8_lossy(&o.stdout).contains("libx264") => "libx264",
        _ => "mpeg4",
    }
}

// ----------------------------------------------------------------------
// The stage on the processor
// ----------------------------------------------------------------------

/// A picture to draw from (RGB, top row first).
pub struct Pic {
    pub w: usize,
    pub h: usize,
    pub rgb: Vec<u8>,
}

impl Pic {
    pub fn from_image(img: &egui::ColorImage) -> Self {
        let [w, h] = img.size;
        Self { w, h, rgb: img.pixels.iter().flat_map(|c| [c.r(), c.g(), c.b()]).collect() }
    }

    /// The colour at (u, v) (0..1 both ways, from the top left), blended
    /// between its four nearest pixels as the graphics card does.
    fn sample(&self, u: f32, v: f32) -> [f32; 3] {
        if self.w == 0 || self.h == 0 {
            return [0.0; 3];
        }
        let x = (u * self.w as f32 - 0.5).clamp(0.0, (self.w - 1) as f32);
        let y = (v * self.h as f32 - 0.5).clamp(0.0, (self.h - 1) as f32);
        let (x0, y0) = (x.floor() as usize, y.floor() as usize);
        let (x1, y1) = ((x0 + 1).min(self.w - 1), (y0 + 1).min(self.h - 1));
        let (fx, fy) = (x - x0 as f32, y - y0 as f32);
        let px = |x: usize, y: usize, c: usize| self.rgb[(y * self.w + x) * 3 + c] as f32 / 255.0;
        let mut out = [0.0; 3];
        for (c, o) in out.iter_mut().enumerate() {
            let top = px(x0, y0, c) * (1.0 - fx) + px(x1, y0, c) * fx;
            let bottom = px(x0, y1, c) * (1.0 - fx) + px(x1, y1, c) * fx;
            *o = top * (1.0 - fy) + bottom * fy;
        }
        out
    }
}

fn fract(x: f32) -> f32 {
    x - x.floor()
}

fn hash(n: f32) -> f32 {
    fract(n.sin() * 43758.5453)
}

fn mix3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t]
}

fn luma(c: [f32; 3]) -> f32 {
    0.299 * c[0] + 0.587 * c[1] + 0.114 * c[2]
}

fn turn2(p: [f32; 2], a: f32) -> [f32; 2] {
    let (s, c) = a.sin_cos();
    [c * p[0] - s * p[1], s * p[0] + c * p[1]]
}

fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn inside(uv: [f32; 2], crop: [f32; 4]) -> bool {
    uv[0] >= crop[0] && uv[0] <= 1.0 - crop[2] && uv[1] >= crop[1] && uv[1] <= 1.0 - crop[3]
}

/// One frame drawn as the stage's shader draws it (see `stage::EFFECTS`,
/// effect for effect in the same order): `new` the clip that plays, `old`
/// the one it is changing from, `prev` the frame drawn before (for Trails
/// and Echo), into `out` (RGB, `w` by `h`).
pub fn draw(out: &mut [u8], prev: &[u8], w: usize, h: usize, new: Option<&Pic>, old: Option<&Pic>, p: &Params) {
    use krabton_engine::video::Effect as E;
    let stage = [w as f32, h as f32];
    let aspect = stage[0] / stage[1];
    // The shader's pixels are the stage's 1280 by 720.
    let unit = [super::stage::STAGE_W as f32, super::stage::STAGE_H as f32];
    let scale = w as f32 / unit[0];
    let a = |e: E| p.fx[e.index()];
    let q = |e: E| p.p[e.index()];
    let place = |st: [f32; 2], size: [f32; 2], fill: bool| {
        let va = size[0] / size[1].max(1.0);
        let mut k = [1.0, 1.0];
        if fill == (va > aspect) {
            k[0] = aspect / va;
        } else {
            k[1] = va / aspect;
        }
        [(st[0] - 0.5) * k[0] + 0.5, (st[1] - 0.5) * k[1] + 0.5]
    };
    let framed = |st: [f32; 2], f: &super::stage::Framing| {
        let mut v = [(st[0] - 0.5 - f.pos[0]) * aspect, st[1] - 0.5 - f.pos[1]];
        v = turn2(v, -f.turn * std::f32::consts::TAU);
        let z = f.zoom.max(0.01);
        [v[0] / z / aspect + 0.5, v[1] / z + 0.5]
    };
    let one = |pic: Option<&Pic>, size: [f32; 2], f: &super::stage::Framing, st: [f32; 2]| -> [f32; 3] {
        let Some(pic) = pic.filter(|_| size[0] > 0.0) else { return [0.0; 3] };
        let uv = place(framed(st, f), size, f.fill);
        if !inside(uv, f.crop) {
            return [0.0; 3];
        }
        pic.sample(uv[0], uv[1]).map(|v| v * f.opacity)
    };
    let pick = |st: [f32; 2]| -> [f32; 3] {
        let fresh = |st| one(new, p.video_size, &p.frame, st);
        let stale = |st| one(old, p.old_size, &p.old_frame, st);
        let m = p.mix.clamp(0.0, 1.0);
        if p.transition < 0.5 {
            return fresh(st);
        }
        if p.transition < 1.5 {
            return mix3(stale(st), fresh(st), m);
        }
        let peak = 1.0 - (m * 2.0 - 1.0).abs();
        if p.transition < 2.5 {
            let c = if m < 0.5 { stale(st) } else { fresh(st) };
            return mix3(c, [1.0; 3], peak);
        }
        let band = (st[1] * 24.0 + hash(p.seed) * 3.0).floor();
        let q = [st[0] + (hash(band * 1.31 + p.seed) - 0.5) * 0.3 * peak, st[1]];
        if hash(band * 7.13 + p.seed.floor()) < m {
            fresh(q)
        } else {
            stale(q)
        }
    };
    let at = |v: [f32; 2]| -> [f32; 3] {
        let st = [v[0] / aspect + 0.5, v[1] + 0.5];
        let off = a(E::Split) * q(E::Split)[0];
        if off < 0.0005 {
            return pick(st);
        }
        let ang = q(E::Split)[1] * std::f32::consts::TAU;
        let d = [ang.cos() * off, ang.sin() * off];
        [pick([st[0] + d[0], st[1] + d[1]])[0], pick(st)[1], pick([st[0] - d[0], st[1] - d[1]])[2]]
    };
    let prev_at = |uv: [f32; 2]| -> Option<[f32; 3]> {
        if prev.len() != w * h * 3 {
            return None;
        }
        let (px, py) = (((uv[0].clamp(0.0, 1.0) * w as f32) as usize).min(w - 1), ((uv[1].clamp(0.0, 1.0) * h as f32) as usize).min(h - 1));
        let o = (py * w + px) * 3;
        Some([prev[o] as f32 / 255.0, prev[o + 1] as f32 / 255.0, prev[o + 2] as f32 / 255.0])
    };
    let row = |y: usize, line: &mut [u8]| {
        for x in 0..w {
            let uv = [(x as f32 + 0.5) / stage[0], (y as f32 + 0.5) / stage[1]];
            let mut st = uv;
            // Scanlines: the glass bulges.
            let mut off_glass = 1.0;
            if a(E::Scanlines) > 0.001 && q(E::Scanlines)[1] > 0.001 {
                let c = [st[0] - 0.5, st[1] - 0.5];
                let k = 1.0 + (c[0] * c[0] + c[1] * c[1]) * q(E::Scanlines)[1] * a(E::Scanlines);
                st = [c[0] * k + 0.5, c[1] * k + 0.5];
                off_glass = 1.0 + (if inside(st, [0.0; 4]) { 1.0 } else { 0.0 } - 1.0) * a(E::Scanlines);
            }
            if a(E::Glitch) > 0.001 {
                let band = (st[1] * q(E::Glitch)[0] + hash(p.seed) * 5.0).floor();
                if hash(band * 3.17 + p.seed * 11.0) < a(E::Glitch) * 0.7 {
                    st[0] += (hash(band * 1.71 + p.seed) - 0.5) * q(E::Glitch)[1] * a(E::Glitch);
                }
            }
            if a(E::Pixels) > 0.001 {
                let px = (1.0 + (q(E::Pixels)[0] - 1.0) * a(E::Pixels) * a(E::Pixels)) * scale;
                st = [((st[0] * stage[0] / px).floor() + 0.5) * px / stage[0], ((st[1] * stage[1] / px).floor() + 0.5) * px / stage[1]];
            }
            let mut v = [(st[0] - 0.5) * aspect, st[1] - 0.5];
            let zq = q(E::Zoom);
            let centre = [zq[1] * aspect, zq[2]];
            let z = 1.0 + a(E::Zoom).powf(zq[3]) * zq[0];
            v = [(v[0] - centre[0]) / z + centre[0], (v[1] - centre[1]) / z + centre[1]];
            v = turn2(v, p.spin * std::f32::consts::TAU);
            v = [v[0] - p.shake[0] * aspect, v[1] - p.shake[1]];
            let mut c = at(v);
            if a(E::ZoomBlur) > 0.001 {
                let zb = q(E::ZoomBlur);
                let zc = [zb[1] * aspect, zb[2]];
                let mut sum = c;
                for k in 1..8 {
                    let t = k as f32 / 8.0 * zb[0] * a(E::ZoomBlur) * 0.5;
                    let s = at([v[0] + (zc[0] - v[0]) * t, v[1] + (zc[1] - v[1]) * t]);
                    sum = [sum[0] + s[0], sum[1] + s[1], sum[2] + s[2]];
                }
                c = sum.map(|s| s / 8.0);
            }
            if a(E::Blur) > 0.001 {
                let r = a(E::Blur) * q(E::Blur)[0] * 0.02;
                let mut sum = c;
                for k in 0..8 {
                    let an = k as f32 * std::f32::consts::FRAC_PI_4;
                    let s = at([v[0] + an.cos() * r, v[1] + an.sin() * r]);
                    sum = [sum[0] + s[0], sum[1] + s[1], sum[2] + s[2]];
                }
                c = sum.map(|s| s / 9.0);
            }
            if a(E::Mirror) > 0.001 {
                let axis = q(E::Mirror)[0];
                let m = if axis < 0.5 {
                    [-v[0].abs(), v[1]]
                } else if axis < 1.5 {
                    [v[0].abs(), v[1]]
                } else if axis < 2.5 {
                    [v[0], -v[1].abs()]
                } else if axis < 3.5 {
                    [v[0], v[1].abs()]
                } else {
                    [-v[0].abs(), -v[1].abs()]
                };
                c = mix3(c, at(m), a(E::Mirror));
            }
            if a(E::Kaleido) > 0.001 {
                let seg = std::f32::consts::TAU / q(E::Kaleido)[0].max(1.0);
                let ang = (v[1].atan2(v[0]) + q(E::Kaleido)[1] * std::f32::consts::TAU).rem_euclid(seg);
                let ang = (ang - seg * 0.5).abs();
                let r = (v[0] * v[0] + v[1] * v[1]).sqrt();
                c = mix3(c, at([ang.cos() * r, ang.sin() * r]), a(E::Kaleido));
            }
            if a(E::Edges) > 0.001 {
                let d = q(E::Edges)[0] / unit[1];
                let gx = luma(at([v[0] + d, v[1]])) - luma(at([v[0] - d, v[1]]));
                let gy = luma(at([v[0], v[1] + d])) - luma(at([v[0], v[1] - d]));
                let e = ((gx * gx + gy * gy).sqrt() * 4.0).clamp(0.0, 1.0);
                let lines = p.edges.map(|k| k * e);
                let over = if q(E::Edges)[1] < 0.5 { lines } else { [0, 1, 2].map(|i| c[i].max(lines[i])) };
                c = mix3(c, over, a(E::Edges));
            }
            if a(E::Hue) > 0.001 {
                let (s, co) = (a(E::Hue) * q(E::Hue)[0] * std::f32::consts::TAU).sin_cos();
                let k = 0.57735f32;
                let dot = k * (c[0] + c[1] + c[2]);
                let cross = [k * (c[2] - c[1]), k * (c[0] - c[2]), k * (c[1] - c[0])];
                c = [0, 1, 2].map(|i| (c[i] * co + cross[i] * s + k * dot * (1.0 - co)).clamp(0.0, 1.0));
                let l = luma(c);
                let sat = 1.0 + (q(E::Hue)[1] - 1.0) * a(E::Hue);
                c = c.map(|v| (l + (v - l) * sat).clamp(0.0, 1.0));
            }
            if a(E::Grade) > 0.001 {
                let g = q(E::Grade);
                let con = 1.0 + (g[1] - 1.0) * a(E::Grade);
                c = c.map(|v| (v - 0.5) * con + 0.5 + g[0] * a(E::Grade));
                let l = luma(c);
                let sat = 1.0 + (g[2] - 1.0) * a(E::Grade);
                c = c.map(|v| (l + (v - l) * sat).clamp(0.0, 1.0));
            }
            if a(E::Poster) > 0.001 {
                let levels = 24.0 + (q(E::Poster)[0] - 24.0) * a(E::Poster);
                c = c.map(|v| (v * levels + 0.5).floor() / levels);
            }
            let blend = q(E::Tint)[0];
            let l = luma(c);
            let tinted = if blend < 0.5 {
                p.tint.map(|t| t * (0.2 + l * 1.3))
            } else if blend < 1.5 {
                [0, 1, 2].map(|i| c[i] * p.tint[i])
            } else if blend < 2.5 {
                [0, 1, 2].map(|i| 1.0 - (1.0 - c[i]) * (1.0 - p.tint[i]))
            } else {
                [0, 1, 2].map(|i| c[i] + p.tint[i])
            };
            c = mix3(c, tinted, a(E::Tint));
            c = mix3(c, c.map(|v| 1.0 - v), a(E::Invert));
            c = mix3(c, p.flash, a(E::Flash) * q(E::Flash)[0]);
            if a(E::Vignette) > 0.001 {
                let d = [(uv[0] - 0.5) * 2.0, (uv[1] - 0.5) * 2.0];
                let r = (d[0] * d[0] + d[1] * d[1]).sqrt() * std::f32::consts::FRAC_1_SQRT_2;
                let inner = 1.0 - q(E::Vignette)[0];
                let soft = q(E::Vignette)[1] * 0.5;
                let k = 1.0 - smoothstep(inner - soft, inner + soft, r) * a(E::Vignette);
                c = c.map(|v| v * k);
            }
            if a(E::Scanlines) > 0.001 {
                let s = 0.5 + 0.5 * (st[1] * q(E::Scanlines)[0] * std::f32::consts::TAU).cos();
                let k = (1.0 - a(E::Scanlines) * 0.6 * (1.0 - s)) * off_glass;
                c = c.map(|v| v * k);
            }
            if a(E::Grain) > 0.001 {
                let size = q(E::Grain)[0].max(1.0);
                let g = [(uv[0] * unit[0] / size).floor(), (uv[1] * unit[1] / size).floor()];
                let n = (hash(g[0] * 12.9898 + g[1] * 78.233 + p.grain) - 0.5) * a(E::Grain) * 0.5;
                c = c.map(|v| v + n);
            }
            c = c.map(|v| v * (1.0 - a(E::Gate)));
            c = mix3(c, p.strobe, a(E::Strobe));
            if a(E::Trails) > 0.001 {
                let t = q(E::Trails);
                if let Some(was) = prev_at([(uv[0] - 0.5) * t[0] + 0.5, (uv[1] - 0.5) * t[0] + 0.5]) {
                    let blend = mix3(c, was, a(E::Trails) * t[1]);
                    c = [0, 1, 2].map(|i| c[i].max(blend[i]));
                }
            }
            if a(E::Echo) > 0.001 {
                let t = q(E::Echo);
                let at_q = [uv[0] - t[0], uv[1] - t[1]];
                if let Some(was) = prev_at(at_q) {
                    let was = if inside(at_q, [0.0; 4]) { was } else { [0.0; 3] };
                    c = mix3(c, was, a(E::Echo) * t[2] * 0.85);
                }
            }
            for (i, v) in c.iter().enumerate() {
                line[x * 3 + i] = (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
            }
        }
    };
    // The rows shared out over the processor's cores.
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get()).clamp(1, 16);
    let rows_each = h.div_ceil(threads).max(1);
    std::thread::scope(|s| {
        for (k, chunk) in out.chunks_mut(rows_each * w * 3).enumerate() {
            let row = &row;
            s.spawn(move || {
                for (j, line) in chunk.chunks_mut(w * 3).enumerate() {
                    row(k * rows_each + j, line);
                }
            });
        }
    });
}

// ----------------------------------------------------------------------
// The job
// ----------------------------------------------------------------------

/// How the export goes.
#[derive(Clone, Debug, PartialEq)]
pub enum State {
    /// Waiting for the films to be read.
    Reading,
    /// The song (0..1).
    Sound(f32),
    /// The frames (0..1).
    Picture(f32),
    Done(PathBuf),
    Failed(String),
}

impl State {
    /// How far all of it is (0..1).
    pub fn fraction(&self) -> f32 {
        match self {
            State::Reading => 0.0,
            State::Sound(f) => 0.3 * f,
            State::Picture(f) => 0.3 + 0.7 * f,
            _ => 1.0,
        }
    }
}

/// What one frame of the film shows.
#[derive(Clone, Copy, Debug)]
pub struct Planned {
    pub shot: Shot,
    pub params: Params,
}

/// An export going on.
pub struct Job {
    pub state: Arc<Mutex<State>>,
    pub cancel: Arc<AtomicBool>,
    pub handle: Option<std::thread::JoinHandle<Vec<Box<dyn krabton_engine::plugin::AudioProcessor>>>>,
    /// Plugin instances used by the render; dropped on the UI thread afterwards.
    pub plugin_instances: Vec<krabton_clap::PluginInstance>,
}

impl Job {
    pub fn state(&self) -> State {
        self.state.lock().map(|s| s.clone()).unwrap_or(State::Failed("lost".into()))
    }
}

/// Everything the job needs.
pub struct Work {
    pub project: Project,
    pub source: RenderSource,
    pub sample_rate: u32,
    pub opts: Opts,
    pub out: PathBuf,
    pub ffmpeg: PathBuf,
    /// Each clip of the bank, read (or being read).
    pub media: Vec<Option<Arc<Media>>>,
    pub processors: Vec<(u64, Box<dyn krabton_engine::plugin::AudioProcessor>)>,
}

/// Plays the Video along with the song (rendered offline): a frame's
/// shot and look for every `sample_rate / fps` samples, and the sound.
/// None when cancelled.
pub fn plan(work_project: &Project, source: RenderSource, sample_rate: u32, fps: u32, media: &[Option<Arc<Media>>], processors: Vec<(u64, Box<dyn krabton_engine::plugin::AudioProcessor>)>, progress: &RenderProgress) -> (Option<(Vec<Planned>, Vec<f32>)>, Vec<Box<dyn krabton_engine::plugin::AudioProcessor>>) {
    let set: VideoSet = work_project.video.clone();
    let durations: Vec<f32> = media.iter().map(|m| m.as_ref().map_or(0.0, |m| m.info().duration)).collect();
    let sizes: Vec<Option<[f32; 2]>> = media.iter().map(|m| m.as_ref().map(|m| m.info()).filter(|i| i.width > 0).map(|i| [i.width as f32, i.height as f32])).collect();
    let mut dir = Director::default();
    dir.switch.clip = set.current.min(set.clips.len().saturating_sub(1));
    let mut frames = Vec::new();
    let dt = 1.0 / fps.max(1) as f32;
    let (audio, left) = render::render_watched(work_project, source, sample_rate, work_project.beats_per_bar_f(), Some(progress), processors, sample_rate as f64 / fps.max(1) as f64, |status, _| {
        let asks = Asks { dt, durations: &durations, ..Default::default() };
        let shot = dir.frame(&set, work_project, status, &asks);
        let size = |i: usize| sizes.get(i).copied().flatten();
        let params = super::frame_params(&set, work_project, &mut dir, &shot, (size(shot.clip), shot.from.and_then(|f| size(f.0))));
        frames.push(Planned { shot, params });
    });
    // A frame for every whole frame's share of the sound.
    (
        audio.map(|a| {
            frames.truncate(frame_count(a.len() / 2, sample_rate, fps));
            (frames, a)
        }),
        left,
    )
}

/// Starts making the film in the background.
pub fn start(work: Work, plugin_instances: Vec<krabton_clap::PluginInstance>) -> Job {
    let state = Arc::new(Mutex::new(State::Reading));
    let cancel = Arc::new(AtomicBool::new(false));
    let (st, stop) = (state.clone(), cancel.clone());
    let handle = std::thread::spawn(move || {
        let (result, left) = run(work, &st, &stop);
        if let Ok(mut s) = st.lock() {
            *s = match result {
                Ok(p) => State::Done(p),
                Err(e) => State::Failed(e),
            };
        }
        left
    });
    Job { state, cancel, handle: Some(handle), plugin_instances }
}

fn set_state(st: &Mutex<State>, s: State) {
    if let Ok(mut g) = st.lock() {
        *g = s;
    }
}

type Leftovers = Vec<Box<dyn krabton_engine::plugin::AudioProcessor>>;

fn run(work: Work, st: &Arc<Mutex<State>>, stop: &Arc<AtomicBool>) -> (Result<PathBuf, String>, Leftovers) {
    let Work { project, source, sample_rate, opts, out, ffmpeg, media, processors } = work;
    // The films, all of them read.
    while media.iter().flatten().any(|m| !m.is_done()) {
        if stop.load(Ordering::Relaxed) {
            return (Err("Export cancelled".into()), Vec::new());
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    // The song, and the Video along with it.
    let progress = Arc::new(RenderProgress::default());
    let watch = {
        let (progress, st, stop) = (progress.clone(), st.clone(), stop.clone());
        let done = Arc::new(AtomicBool::new(false));
        let d = done.clone();
        let t = std::thread::spawn(move || {
            while !d.load(Ordering::Relaxed) {
                if stop.load(Ordering::Relaxed) {
                    progress.cancel.store(true, Ordering::Relaxed);
                }
                set_state(&st, State::Sound(progress.fraction()));
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        });
        (done, t)
    };
    let (planned, left) = plan(&project, source, sample_rate, opts.fps, &media, processors, &progress);
    watch.0.store(true, Ordering::Relaxed);
    let _ = watch.1.join();
    let Some((frames, audio)) = planned else { return (Err("Export cancelled".into()), left) };
    let result = encode(&frames, &audio, sample_rate, opts, &out, &ffmpeg, &media, st, stop);
    (result.map(|_| out), left)
}

#[allow(clippy::too_many_arguments)]
fn encode(frames: &[Planned], audio: &[f32], sample_rate: u32, opts: Opts, out: &Path, ffmpeg: &Path, media: &[Option<Arc<Media>>], st: &Mutex<State>, stop: &AtomicBool) -> Result<(), String> {
    let tmp = std::env::temp_dir().join(format!("krabton-video-export-{}-{}", std::process::id(), out.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default()));
    std::fs::create_dir_all(&tmp).map_err(|e| format!("{}: {e}", tmp.display()))?;
    let wav = tmp.join("sound.wav");
    let log = tmp.join("ffmpeg.log");
    let result = (|| {
        render::write_wav(&wav, sample_rate, audio)?;
        let enc = encoder(ffmpeg);
        let log_file = std::fs::File::create(&log).map_err(|e| e.to_string())?;
        let mut child = super::ffmpeg::command(ffmpeg)
            .args(encode_args(opts.width, opts.height, opts.fps, &wav, out, enc))
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(log_file)
            .spawn()
            .map_err(|e| format!("ffmpeg did not run: {e}"))?;
        let (w, h) = (opts.width as usize, opts.height as usize);
        let mut px = vec![0u8; w * h * 3];
        let mut prev = vec![0u8; w * h * 3];
        // The last frame unpacked of each clip (most frames show the same one again).
        let mut cache: Vec<Option<(usize, Pic)>> = (0..media.len()).map(|_| None).collect();
        let mut written = Ok(());
        {
            let mut stdin = child.stdin.take().ok_or("ffmpeg takes no frames")?;
            for (k, f) in frames.iter().enumerate() {
                if stop.load(Ordering::Relaxed) {
                    written = Err("Export cancelled".to_string());
                    break;
                }
                for (clip, at) in [Some((f.shot.clip, f.shot.at)), f.shot.from.map(|(c, a, _)| (c, a))].into_iter().flatten() {
                    let Some(m) = media.get(clip).cloned().flatten() else { continue };
                    let Some(i) = m.frame_at(at) else { continue };
                    if cache[clip].as_ref().map(|c| c.0) != Some(i) {
                        cache[clip] = m.image(i).map(|img| (i, Pic::from_image(&img)));
                    }
                }
                let pic = |c: usize| cache.get(c).and_then(|p| p.as_ref()).map(|p| &p.1);
                std::mem::swap(&mut px, &mut prev);
                draw(&mut px, &prev, w, h, pic(f.shot.clip), f.shot.from.and_then(|(c, _, _)| pic(c)), &f.params);
                if let Err(e) = stdin.write_all(&px) {
                    written = Err(format!("ffmpeg stopped taking frames: {e}"));
                    break;
                }
                if k % 10 == 0 {
                    set_state(st, State::Picture(k as f32 / frames.len().max(1) as f32));
                }
            }
        }
        let status = child.wait().map_err(|e| e.to_string())?;
        written?;
        if !status.success() {
            let why = std::fs::read_to_string(&log).unwrap_or_default();
            let line = why.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("ffmpeg failed").to_string();
            return Err(format!("ffmpeg could not make the film: {line}"));
        }
        Ok(())
    })();
    let _ = std::fs::remove_dir_all(&tmp);
    if result.is_err() {
        let _ = std::fs::remove_file(out);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::super::stage::Framing;
    use super::*;
    use krabton_engine::video::Effect;

    #[test]
    fn every_frame_has_its_share_of_the_sound_without_drifting() {
        // Three minutes at 48 kHz, 30 a second: 5400 frames of 1600 samples.
        assert_eq!(frame_count(48_000 * 180, 48_000, 30), 5400);
        assert_eq!(frame_samples(0, 48_000, 30), (0, 1600));
        // 44.1 kHz at 30: 1470 each; at 24 (1837.5) they take turns, 1838 and 1837.
        assert_eq!(frame_samples(3, 44_100, 30), (4410, 5880));
        let lens: Vec<usize> = (0..4).map(|k| frame_samples(k, 44_100, 24)).map(|(a, b)| b - a).collect();
        assert_eq!(lens.iter().sum::<usize>(), 7350);
        assert!(lens.iter().all(|l| *l == 1837 || *l == 1838));
        // After an hour of frames, still where the sound is.
        let k = 24 * 3600;
        assert_eq!(frame_samples(k, 44_100, 24).0, 44_100 * 3600);
        // 29.97 is not on offer, but a rate that does not divide still adds up.
        let total: usize = (0..1000).map(|k| frame_samples(k, 48_000, 60)).map(|(a, b)| b - a).sum();
        assert_eq!(total, 800_000);
    }

    #[test]
    fn the_encode_command_takes_raw_frames_and_the_sound() {
        let args: Vec<String> = encode_args(1280, 720, 30, Path::new("C:/tmp/sound.wav"), Path::new("C:/out/my song.mp4"), "libx264").into_iter().map(|a| a.to_string_lossy().into_owned()).collect();
        let at = |s: &str| args.iter().position(|a| a == s).unwrap_or_else(|| panic!("{s} in {args:?}"));
        assert_eq!(args[at("-f") + 1], "rawvideo");
        assert_eq!(args[at("-pix_fmt") + 1], "rgb24", "the input's pixels come first");
        assert_eq!(args[at("-s") + 1], "1280x720");
        assert_eq!(args[at("-r") + 1], "30");
        // The frames from the pipe, then the sound.
        let inputs: Vec<&String> = args.iter().enumerate().filter(|(i, _)| *i > 0 && args[i - 1] == "-i").map(|(_, a)| a).collect();
        assert_eq!(inputs, ["-", "C:/tmp/sound.wav"]);
        assert_eq!(args[at("-c:v") + 1], "libx264");
        assert_eq!(args[at("-c:a") + 1], "aac");
        assert!(args.windows(2).any(|w| w == ["-pix_fmt", "yuv420p"]), "for every player");
        assert_eq!(args.last().map(String::as_str), Some("C:/out/my song.mp4"));
        // Without H.264: MPEG-4 at a fixed quality.
        let plain: Vec<String> = encode_args(854, 480, 25, Path::new("a.wav"), Path::new("b.mp4"), "mpeg4").into_iter().map(|a| a.to_string_lossy().into_owned()).collect();
        assert!(plain.windows(2).any(|w| w == ["-c:v", "mpeg4"]) && plain.windows(2).any(|w| w == ["-q:v", "3"]));
        assert!(!plain.contains(&"-crf".to_string()));
    }

    fn pic(w: usize, h: usize, rgb: [u8; 3]) -> Pic {
        Pic { w, h, rgb: (0..w * h).flat_map(|_| rgb).collect() }
    }

    #[test]
    fn the_processor_draws_what_the_stage_draws() {
        let (w, h) = (64, 36);
        let mut out = vec![0u8; w * h * 3];
        let red = pic(16, 9, [255, 0, 0]);
        let p = Params { video_size: [16.0, 9.0], ..Default::default() };
        draw(&mut out, &[], w, h, Some(&red), None, &p);
        assert!(out.chunks(3).all(|c| c == [255, 0, 0]), "a 16:9 picture fills a 16:9 film");
        // Fit, a square picture: black bars at the sides.
        let sq = pic(8, 8, [0, 255, 0]);
        draw(&mut out, &[], w, h, Some(&sq), None, &Params { video_size: [8.0, 8.0], frame: Framing { fill: false, ..Default::default() }, ..Default::default() });
        assert_eq!(&out[(18 * w) * 3..(18 * w) * 3 + 3], &[0, 0, 0], "the left edge is a bar");
        assert_eq!(&out[(18 * w + 32) * 3..(18 * w + 32) * 3 + 3], &[0, 255, 0]);
        // Invert, and a full flash to (nearly) white.
        draw(&mut out, &[], w, h, Some(&red), None, &with(p, &[(Effect::Invert, 1.0)]));
        assert_eq!(&out[..3], &[0, 255, 255]);
        draw(&mut out, &[], w, h, Some(&red), None, &with(p, &[(Effect::Gate, 1.0)]));
        assert!(out.iter().all(|v| *v == 0), "gated shut: dark");
        // A fade halfway between the clip it leaves and the next.
        let blue = pic(16, 9, [0, 0, 255]);
        draw(&mut out, &[], w, h, Some(&red), Some(&blue), &Params { transition: 1.0, mix: 0.5, old_size: [16.0, 9.0], ..p });
        let c = &out[(18 * w + 30) * 3..(18 * w + 30) * 3 + 3];
        assert!((c[0] as i32 - 128).abs() < 3 && c[1] == 0 && (c[2] as i32 - 128).abs() < 3, "{c:?}");
        // Through white: all white halfway.
        draw(&mut out, &[], w, h, Some(&red), Some(&blue), &Params { transition: 2.0, mix: 0.5, old_size: [16.0, 9.0], ..p });
        assert!(out.iter().all(|v| *v == 255));
        // Nothing to show: black.
        draw(&mut out, &[], w, h, None, None, &p);
        assert!(out.iter().all(|v| *v == 0));
    }

    /// `p` with these effects this much.
    fn with(p: Params, fx: &[(Effect, f32)]) -> Params {
        let mut p = p;
        for (e, v) in fx {
            p.fx[e.index()] = *v;
        }
        p
    }

    fn at(out: &[u8], w: usize, x: usize, y: usize) -> [u8; 3] {
        let o = (y * w + x) * 3;
        [out[o], out[o + 1], out[o + 2]]
    }

    /// A picture `top` above and `bottom` below its middle.
    fn halves(w: usize, h: usize, top: [u8; 3], bottom: [u8; 3]) -> Pic {
        Pic { w, h, rgb: (0..w * h).flat_map(|i| if i / w < h / 2 { top } else { bottom }).collect() }
    }

    #[test]
    fn the_new_effects_and_settings_draw_what_they_say() {
        let (w, h) = (64, 36);
        let mut out = vec![0u8; w * h * 3];
        let p = Params { video_size: [16.0, 9.0], ..Default::default() };
        let near = |c: [u8; 3], e: [u8; 3]| c.iter().zip(e).all(|(a, b)| (*a as i32 - b as i32).abs() <= 2);
        // Grade: a quarter brighter (and the contrast it starts with) on grey.
        let grey = pic(16, 9, [128, 128, 128]);
        let mut g = with(p, &[(Effect::Grade, 1.0)]);
        g.p[Effect::Grade.index()][0] = 0.25;
        draw(&mut out, &[], w, h, Some(&grey), None, &g);
        assert!(near(at(&out, w, 30, 18), [192, 192, 192]), "{:?}", at(&out, w, 30, 18));
        // Vignette: the corners dark, the middle as it was.
        draw(&mut out, &[], w, h, Some(&grey), None, &with(p, &[(Effect::Vignette, 1.0)]));
        assert!(at(&out, w, 0, 0)[0] < 40 && near(at(&out, w, 32, 18), [128, 128, 128]));
        // Mirror, top onto bottom: red over blue becomes red all over.
        let rb = halves(16, 9, [255, 0, 0], [0, 0, 255]);
        let mut m = with(p, &[(Effect::Mirror, 1.0)]);
        m.p[Effect::Mirror.index()][0] = 2.0;
        draw(&mut out, &[], w, h, Some(&rb), None, &m);
        assert_eq!((at(&out, w, 10, 2), at(&out, w, 10, 33)), ([255, 0, 0], [255, 0, 0]));
        // Tint, multiplied by green: white turns green.
        let white = pic(16, 9, [255, 255, 255]);
        let mut t = with(p, &[(Effect::Tint, 1.0)]);
        t.p[Effect::Tint.index()][0] = 1.0;
        t.tint = [0.0, 1.0, 0.0];
        draw(&mut out, &[], w, h, Some(&white), None, &t);
        assert_eq!(at(&out, w, 20, 20), [0, 255, 0]);
        // Edges over black: none in a flat picture, a white line where red meets blue.
        draw(&mut out, &[], w, h, Some(&white), None, &with(p, &[(Effect::Edges, 1.0)]));
        assert_eq!(at(&out, w, 20, 10), [0, 0, 0]);
        let bw = halves(16, 9, [255, 255, 255], [0, 0, 0]);
        let mut edges = with(p, &[(Effect::Edges, 1.0)]);
        edges.p[Effect::Edges.index()][0] = 4.0;
        draw(&mut out, &[], w, h, Some(&bw), None, &edges);
        let line = (0..h).map(|y| at(&out, w, 20, y)[0]).max().unwrap();
        assert!(line > 60 && at(&out, w, 20, 2) == [0, 0, 0] && at(&out, w, 20, 33) == [0, 0, 0], "{line}");
        // Strobe in a colour of its own.
        let mut s = with(p, &[(Effect::Strobe, 1.0)]);
        s.strobe = [1.0, 0.0, 0.0];
        draw(&mut out, &[], w, h, Some(&grey), None, &s);
        assert!(out.chunks(3).all(|c| c == [255, 0, 0]));
        // Echo: the last frame (white) under this one (black), moved a little.
        let black = pic(16, 9, [0, 0, 0]);
        let prev = vec![255u8; w * h * 3];
        let mut e = with(p, &[(Effect::Echo, 1.0)]);
        e.p[Effect::Echo.index()] = [0.0, 0.0, 1.0, 0.0];
        draw(&mut out, &prev, w, h, Some(&black), None, &e);
        assert!(near(at(&out, w, 30, 18), [217, 217, 217]), "{:?}", at(&out, w, 30, 18));
        // The clip's frame: cropped on the left half, half see-through.
        let framed = Params { frame: Framing { crop: [0.5, 0.0, 0.0, 0.0], opacity: 0.5, ..Default::default() }, ..p };
        draw(&mut out, &[], w, h, Some(&white), None, &framed);
        assert_eq!((at(&out, w, 10, 18), at(&out, w, 50, 18)), ([0, 0, 0], [128, 128, 128]));
        // Grain, scanlines and blur stay in their range and change a flat picture only a little (blur not at all).
        draw(&mut out, &[], w, h, Some(&grey), None, &with(p, &[(Effect::Blur, 1.0), (Effect::ZoomBlur, 1.0)]));
        assert!(near(at(&out, w, 32, 18), [128, 128, 128]));
        draw(&mut out, &[], w, h, Some(&grey), None, &with(p, &[(Effect::Scanlines, 1.0), (Effect::Grain, 1.0)]));
        assert!(out.iter().any(|v| *v != 128));
    }

    #[test]
    fn a_look_morphs_its_amounts_and_settings_and_switches_its_choices_at_the_start() {
        let mut a = with(Params::default(), &[(Effect::Zoom, 0.2)]);
        let mut b = with(Params::default(), &[(Effect::Zoom, 1.0), (Effect::Hue, 0.5)]);
        a.p[Effect::Pixels.index()][0] = 10.0;
        b.p[Effect::Pixels.index()][0] = 30.0;
        a.p[Effect::Mirror.index()][0] = 0.0;
        b.p[Effect::Mirror.index()][0] = 4.0;
        a.tint = [0.0; 3];
        b.tint = [1.0; 3];
        let m = Params::morph(&a, &b, 0.25);
        assert!((m.get(Effect::Zoom) - 0.4).abs() < 1e-6);
        assert!((m.get(Effect::Hue) - 0.125).abs() < 1e-6);
        assert!((m.param(Effect::Pixels, 0) - 15.0).abs() < 1e-5);
        assert_eq!(m.param(Effect::Mirror, 0), 4.0, "the axis is the new Look's at once");
        assert!((m.tint[0] - 0.25).abs() < 1e-6);
        assert_eq!(Params::morph(&a, &b, 1.0), b);
    }

    /// The stage's settings as they were before the effects had settings of their own (0.93).
    #[derive(Clone, Copy, Default)]
    struct OldParams {
        video_size: [f32; 2],
        fill: bool,
        zoom: f32,
        spin: f32,
        shake: [f32; 2],
        mirror: f32,
        kaleido: f32,
        pixels: f32,
        split: f32,
        glitch: f32,
        seed: f32,
        hue: f32,
        poster: f32,
        flash: f32,
        invert: f32,
        strobe: f32,
        gate: f32,
        trails: f32,
        tint: [f32; 4],
        transition: f32,
        mix: f32,
        old_size: [f32; 2],
    }

    /// The same frame in today's settings: every effect as it starts, its amount as before.
    fn now_params(o: &OldParams) -> Params {
        use krabton_engine::video::Effect as E;
        let mut p = Params { video_size: o.video_size, frame: Framing { fill: o.fill, ..Default::default() }, old_frame: Framing { fill: o.fill, ..Default::default() }, spin: o.spin, shake: o.shake, seed: o.seed, tint: [o.tint[0], o.tint[1], o.tint[2]], transition: o.transition, mix: o.mix, old_size: o.old_size, ..Default::default() };
        for (e, v) in [(E::Zoom, o.zoom), (E::Mirror, o.mirror), (E::Kaleido, o.kaleido), (E::Pixels, o.pixels), (E::Split, o.split), (E::Glitch, o.glitch), (E::Hue, o.hue), (E::Poster, o.poster), (E::Flash, o.flash), (E::Invert, o.invert), (E::Strobe, o.strobe), (E::Gate, o.gate), (E::Trails, o.trails), (E::Tint, o.tint[3])] {
            p.fx[e.index()] = v;
        }
        p
    }

    /// A picture with something in every pixel: red across, green down, blue in a checker.
    fn gradient(w: usize, h: usize) -> Pic {
        Pic { w, h, rgb: (0..w * h).flat_map(|i| [(i % w * 255 / w) as u8, (i / w * 255 / h) as u8, if (i % w / 4 + i / w / 4) % 2 == 0 { 200 } else { 30 }]).collect() }
    }

    #[test]
    fn a_set_from_before_the_settings_looks_exactly_as_it_did() {
        let (w, h) = (96, 54);
        let pic = gradient(40, 30);
        let other = gradient(20, 20);
        let base = OldParams { video_size: [40.0, 30.0], fill: true, mix: 1.0, old_size: [20.0, 20.0], seed: 37.3, ..Default::default() };
        let mut prev = vec![0u8; w * h * 3];
        for (k, b) in prev.iter_mut().enumerate() {
            *b = (k * 7 % 251) as u8;
        }
        let cases = [
            OldParams { zoom: 0.6, spin: 0.13, shake: [0.02, -0.01], ..base },
            OldParams { mirror: 0.7, kaleido: 0.4, ..base },
            OldParams { pixels: 0.5, split: 0.8, glitch: 0.9, ..base },
            OldParams { hue: 0.3, poster: 0.6, flash: 0.4, ..base },
            OldParams { invert: 0.5, strobe: 0.3, gate: 0.2, trails: 0.7, tint: [0.2, 0.9, 0.4, 0.6], ..base },
            OldParams { fill: false, transition: 1.0, mix: 0.4, ..base },
            OldParams { transition: 3.0, mix: 0.6, zoom: 0.2, ..base },
        ];
        for (i, o) in cases.iter().enumerate() {
            let (mut was, mut now) = (vec![0u8; w * h * 3], vec![0u8; w * h * 3]);
            old_draw(&mut was, &prev, w, h, Some(&pic), Some(&other), o);
            draw(&mut now, &prev, w, h, Some(&pic), Some(&other), &now_params(o));
            let worst = was.iter().zip(&now).map(|(a, b)| (*a as i32 - *b as i32).abs()).max().unwrap();
            assert!(worst <= 1, "case {i}: off by {worst}");
        }
    }

    /// The processor's drawing as it was before the effects had settings (0.93), to hold the new one to.
    fn old_draw(out: &mut [u8], prev: &[u8], w: usize, h: usize, new: Option<&Pic>, old: Option<&Pic>, p: &OldParams) {
        let stage = [w as f32, h as f32];
        let aspect = stage[0] / stage[1];
        // The shader's pixels are the stage's 1280 wide.
        let scale = w as f32 / super::super::stage::STAGE_W as f32;
        let place = |st: [f32; 2], size: [f32; 2]| {
            let va = size[0] / size[1].max(1.0);
            let mut k = [1.0, 1.0];
            if p.fill == (va > aspect) {
                k[0] = aspect / va;
            } else {
                k[1] = va / aspect;
            }
            [(st[0] - 0.5) * k[0] + 0.5, (st[1] - 0.5) * k[1] + 0.5]
        };
        let one = |pic: Option<&Pic>, size: [f32; 2], st: [f32; 2]| -> [f32; 3] {
            let Some(pic) = pic.filter(|_| size[0] > 0.0) else { return [0.0; 3] };
            let uv = place(st, size);
            if !(0.0..=1.0).contains(&uv[0]) || !(0.0..=1.0).contains(&uv[1]) {
                return [0.0; 3];
            }
            pic.sample(uv[0], uv[1])
        };
        let pick = |st: [f32; 2]| -> [f32; 3] {
            let fresh = |st| one(new, p.video_size, st);
            let stale = |st| one(old, p.old_size, st);
            let m = p.mix.clamp(0.0, 1.0);
            if p.transition < 0.5 {
                return fresh(st);
            }
            if p.transition < 1.5 {
                return mix3(stale(st), fresh(st), m);
            }
            let peak = 1.0 - (m * 2.0 - 1.0).abs();
            if p.transition < 2.5 {
                let c = if m < 0.5 { stale(st) } else { fresh(st) };
                return mix3(c, [1.0; 3], peak);
            }
            let band = (st[1] * 24.0 + hash(p.seed) * 3.0).floor();
            let q = [st[0] + (hash(band * 1.31 + p.seed) - 0.5) * 0.3 * peak, st[1]];
            if hash(band * 7.13 + p.seed.floor()) < m {
                fresh(q)
            } else {
                stale(q)
            }
        };
        let at = |q: [f32; 2]| -> [f32; 3] {
            let st = [q[0] / aspect + 0.5, q[1] + 0.5];
            let off = p.split * 0.03;
            if off < 0.0005 {
                return pick(st);
            }
            [pick([st[0] + off, st[1]])[0], pick(st)[1], pick([st[0] - off, st[1]])[2]]
        };
        let row = |y: usize, line: &mut [u8]| {
            for x in 0..w {
                let uv = [(x as f32 + 0.5) / stage[0], (y as f32 + 0.5) / stage[1]];
                let mut st = uv;
                if p.glitch > 0.001 {
                    let band = (st[1] * 18.0 + hash(p.seed) * 5.0).floor();
                    if hash(band * 3.17 + p.seed * 11.0) < p.glitch * 0.7 {
                        st[0] += (hash(band * 1.71 + p.seed) - 0.5) * 0.4 * p.glitch;
                    }
                }
                if p.pixels > 0.001 {
                    let px = (1.0 + 39.0 * p.pixels * p.pixels) * scale;
                    st = [((st[0] * stage[0] / px).floor() + 0.5) * px / stage[0], ((st[1] * stage[1] / px).floor() + 0.5) * px / stage[1]];
                }
                let mut q = [(st[0] - 0.5) * aspect, st[1] - 0.5];
                let z = 1.0 + p.zoom * 1.5;
                q = [q[0] / z, q[1] / z];
                let a = p.spin * std::f32::consts::TAU;
                q = [a.cos() * q[0] - a.sin() * q[1], a.sin() * q[0] + a.cos() * q[1]];
                q = [q[0] - p.shake[0] * aspect, q[1] - p.shake[1]];
                let mut c = at(q);
                if p.mirror > 0.001 {
                    c = mix3(c, at([-q[0].abs(), q[1]]), p.mirror);
                }
                if p.kaleido > 0.001 {
                    let seg = std::f32::consts::TAU / 6.0;
                    let ang = (q[1].atan2(q[0])).rem_euclid(seg);
                    let ang = (ang - seg * 0.5).abs();
                    let r = (q[0] * q[0] + q[1] * q[1]).sqrt();
                    c = mix3(c, at([ang.cos() * r, ang.sin() * r]), p.kaleido);
                }
                if p.hue > 0.001 {
                    let (s, co) = (p.hue * std::f32::consts::TAU).sin_cos();
                    let k = 0.57735f32;
                    let dot = k * (c[0] + c[1] + c[2]);
                    let cross = [k * (c[2] - c[1]), k * (c[0] - c[2]), k * (c[1] - c[0])];
                    c = [0, 1, 2].map(|i| (c[i] * co + cross[i] * s + k * dot * (1.0 - co)).clamp(0.0, 1.0));
                }
                if p.poster > 0.001 {
                    let levels = 24.0 + (2.0 - 24.0) * p.poster;
                    c = c.map(|v| (v * levels + 0.5).floor() / levels);
                }
                let luma = 0.299 * c[0] + 0.587 * c[1] + 0.114 * c[2];
                c = mix3(c, [0, 1, 2].map(|i| p.tint[i] * (0.2 + luma * 1.3)), p.tint[3]);
                c = mix3(c, c.map(|v| 1.0 - v), p.invert);
                c = mix3(c, [1.0; 3], p.flash * 0.85);
                c = c.map(|v| v * (1.0 - p.gate));
                c = mix3(c, [1.0; 3], p.strobe);
                if p.trails > 0.001 && prev.len() == w * h * 3 {
                    let qx = ((uv[0] - 0.5) * 0.985 + 0.5).clamp(0.0, 1.0);
                    let qy = ((uv[1] - 0.5) * 0.985 + 0.5).clamp(0.0, 1.0);
                    let (px, py) = (((qx * w as f32) as usize).min(w - 1), ((qy * h as f32) as usize).min(h - 1));
                    let o = (py * w + px) * 3;
                    let was = [prev[o] as f32 / 255.0, prev[o + 1] as f32 / 255.0, prev[o + 2] as f32 / 255.0];
                    let blend = mix3(c, was, p.trails * 0.9);
                    c = [0, 1, 2].map(|i| c[i].max(blend[i]));
                }
                for (i, v) in c.iter().enumerate() {
                    line[x * 3 + i] = (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
                }
            }
        };
        // The rows shared out over the processor's cores.
        let threads = std::thread::available_parallelism().map_or(4, |n| n.get()).clamp(1, 16);
        let rows_each = h.div_ceil(threads).max(1);
        std::thread::scope(|s| {
            for (k, chunk) in out.chunks_mut(rows_each * w * 3).enumerate() {
                let row = &row;
                s.spawn(move || {
                    for (j, line) in chunk.chunks_mut(w * 3).enumerate() {
                        row(k * rows_each + j, line);
                    }
                });
            }
        });
    }


    /// A short real export with the ffmpeg of `KRABTON_FFMPEG` (skipped without one).
    #[test]
    fn a_short_song_becomes_a_film() {
        let Some(exe) = std::env::var_os("KRABTON_FFMPEG").map(PathBuf::from).filter(|p| p.is_file()) else { return };
        let dir = std::env::temp_dir().join(format!("krabton-export-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        // A picture in the Video, moved by the demo's kicks.
        let still = dir.join("dot.png");
        image::RgbaImage::from_pixel(32, 18, image::Rgba([200, 40, 40, 255])).save(&still).unwrap();
        let mut project = krabton_engine::demo::demo_project();
        project.video.clips.push(krabton_engine::video::VideoClip::new(still.clone()));
        let m = Media::open(&still);
        let out = dir.join("song.mp4");
        let job = start(
            Work { project, source: RenderSource::Scene { scene: 3, beats: 4.0 }, sample_rate: 48_000, opts: Opts { width: 320, height: 180, fps: 12 }, out: out.clone(), ffmpeg: exe.clone(), media: vec![Some(m)], processors: Vec::new() },
            Vec::new(),
        );
        let mut job = job;
        let _ = job.handle.take().unwrap().join();
        assert_eq!(job.state(), State::Done(out.clone()));
        // ffmpeg reads it back: a film of that size with sound.
        let probe = super::super::ffmpeg::command(&exe).args(super::super::ffmpeg::probe_args(&out)).stderr(std::process::Stdio::piped()).output().unwrap();
        let text = String::from_utf8_lossy(&probe.stderr);
        let p = super::super::ffmpeg::parse_probe(&text).expect("a film");
        assert_eq!((p.width, p.height), (320, 180));
        assert!(text.contains("Audio: aac"), "{text}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}

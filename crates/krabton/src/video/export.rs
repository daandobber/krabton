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

/// One frame drawn as the stage's shader draws it (see `stage::EFFECTS`):
/// `new` the clip that plays, `old` the one it is changing from, `prev`
/// the frame drawn before (for Trails), into `out` (RGB, `w` by `h`).
pub fn draw(out: &mut [u8], prev: &[u8], w: usize, h: usize, new: Option<&Pic>, old: Option<&Pic>, p: &Params) {
    let stage = [w as f32, h as f32];
    let aspect = stage[0] / stage[1];
    // The shader's pixels are the stage's 1280 wide.
    let scale = w as f32 / super::stage::STAGE_W as f32;
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
        let look = dir.look;
        let size = |i: usize| sizes.get(i).copied().flatten();
        let params = super::stage_params(&set, work_project, &look, &shot, (size(shot.clip), shot.from.and_then(|f| size(f.0))), &mut dir.dice);
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
    use super::*;

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
        let p = Params { video_size: [16.0, 9.0], fill: true, ..Default::default() };
        draw(&mut out, &[], w, h, Some(&red), None, &p);
        assert!(out.chunks(3).all(|c| c == [255, 0, 0]), "a 16:9 picture fills a 16:9 film");
        // Fit, a square picture: black bars at the sides.
        let sq = pic(8, 8, [0, 255, 0]);
        draw(&mut out, &[], w, h, Some(&sq), None, &Params { video_size: [8.0, 8.0], fill: false, ..Default::default() });
        assert_eq!(&out[(18 * w) * 3..(18 * w) * 3 + 3], &[0, 0, 0], "the left edge is a bar");
        assert_eq!(&out[(18 * w + 32) * 3..(18 * w + 32) * 3 + 3], &[0, 255, 0]);
        // Invert, and a full flash to (nearly) white.
        draw(&mut out, &[], w, h, Some(&red), None, &Params { invert: 1.0, ..p });
        assert_eq!(&out[..3], &[0, 255, 255]);
        draw(&mut out, &[], w, h, Some(&red), None, &Params { gate: 1.0, ..p });
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

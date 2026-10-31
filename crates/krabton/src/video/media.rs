//! The clips of the Video's bank, read into memory in the background:
//! pictures (PNG, JPEG) and animated GIFs by Krabton itself, films through
//! ffmpeg (see `ffmpeg`). A film can be shown while it is still being read:
//! its frames come in one by one.

use super::ffmpeg;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};

/// How much memory one clip may take (a film's frames are JPEGs, so a
/// whole song fits at its own frame rate).
pub const BUDGET: usize = 384 << 20;
pub const FILM_BUDGET: usize = 640 << 20;
/// Pictures larger than this (either way) are made smaller.
const MAX_PICTURE: u32 = 1920;
/// GIF frames wider than this are made smaller.
const MAX_GIF: u32 = 960;

/// What a clip is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Picture,
    Gif,
    Film,
}

/// The pixels of a frame: RGBA, or a JPEG as films come (about an eighth
/// of that; unpacked as it is shown).
pub enum Pixels {
    Rgba(Vec<u8>),
    Jpeg(Vec<u8>),
}

pub struct Frame {
    /// When it starts (seconds).
    pub t: f32,
    pub px: Pixels,
}

/// What is known of a clip so far.
#[derive(Clone, Debug, Default)]
pub struct Info {
    pub width: u32,
    pub height: u32,
    /// Seconds (0 for a picture).
    pub duration: f32,
    /// Frames it will have when read.
    pub expected: usize,
    /// A line about it (size, frames a second, memory).
    pub note: String,
    pub error: Option<String>,
    /// A film, and no ffmpeg to read it.
    pub needs_ffmpeg: bool,
}

pub struct Media {
    pub path: PathBuf,
    pub kind: Kind,
    info: Mutex<Info>,
    frames: RwLock<Vec<Frame>>,
    done: AtomicBool,
    cancel: AtomicBool,
}

/// What a file is, by its first bytes, else by its name.
pub fn kind_of(path: &Path, head: &[u8]) -> Kind {
    if head.starts_with(b"GIF8") {
        return Kind::Gif;
    }
    if head.starts_with(b"\x89PNG") || head.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return Kind::Picture;
    }
    let ext = path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    match ext.as_str() {
        "gif" => Kind::Gif,
        "png" | "jpg" | "jpeg" => Kind::Picture,
        _ => Kind::Film,
    }
}

/// The files the Video takes (for the file dialog).
pub const EXTENSIONS: [&str; 16] = ["mp4", "mov", "m4v", "webm", "mkv", "avi", "wmv", "mpg", "mpeg", "ogv", "flv", "3gp", "gif", "png", "jpg", "jpeg"];

/// Whether a dropped file is one the Video takes.
pub fn takes(path: &Path) -> bool {
    path.extension().map(|e| e.to_string_lossy().to_lowercase()).is_some_and(|e| EXTENSIONS.contains(&e.as_str()))
}

impl Media {
    /// Starts reading a clip; it fills in as it is read.
    pub fn open(path: &Path) -> Arc<Media> {
        let head = std::fs::File::open(path)
            .and_then(|mut f| {
                let mut b = [0u8; 8];
                let n = std::io::Read::read(&mut f, &mut b)?;
                Ok(b[..n].to_vec())
            })
            .unwrap_or_default();
        let m = Arc::new(Media { path: path.to_path_buf(), kind: kind_of(path, &head), info: Mutex::new(Info::default()), frames: RwLock::new(Vec::new()), done: AtomicBool::new(false), cancel: AtomicBool::new(false) });
        let work = m.clone();
        std::thread::spawn(move || {
            let result = match work.kind {
                Kind::Picture => work.read_picture(),
                Kind::Gif => work.read_gif(),
                Kind::Film => work.read_film(),
            };
            if let Err(e) = result {
                if let Ok(mut i) = work.info.lock() {
                    i.error = Some(e);
                }
            }
            work.done.store(true, Ordering::Release);
        });
        m
    }

    /// A clip made in memory (for the tests).
    #[cfg(test)]
    pub fn from_frames(path: &Path, kind: Kind, width: u32, height: u32, frames: Vec<Frame>, duration: f32) -> Arc<Media> {
        let info = Info { width, height, duration, expected: frames.len(), ..Default::default() };
        Arc::new(Media { path: path.to_path_buf(), kind, info: Mutex::new(info), frames: RwLock::new(frames), done: AtomicBool::new(true), cancel: AtomicBool::new(false) })
    }

    pub fn info(&self) -> Info {
        self.info.lock().map(|i| i.clone()).unwrap_or_default()
    }

    /// Read all of it (or given up).
    pub fn is_done(&self) -> bool {
        self.done.load(Ordering::Acquire)
    }

    /// Stops reading (it was taken out of the bank).
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Release);
    }

    pub fn frame_count(&self) -> usize {
        self.frames.read().map(|f| f.len()).unwrap_or(0)
    }

    /// The frame that shows at `t` seconds (of those read so far).
    pub fn frame_at(&self, t: f64) -> Option<usize> {
        let f = self.frames.read().ok()?;
        if f.is_empty() {
            return None;
        }
        let i = f.partition_point(|fr| fr.t as f64 <= t);
        Some(i.saturating_sub(1).min(f.len() - 1))
    }

    /// A frame as an egui picture.
    pub fn image(&self, index: usize) -> Option<egui::ColorImage> {
        let f = self.frames.read().ok()?;
        let fr = f.get(index)?;
        let i = self.info();
        let (w, h) = (i.width as usize, i.height as usize);
        Some(match &fr.px {
            Pixels::Rgba(px) => egui::ColorImage::from_rgba_unmultiplied([w, h], px),
            Pixels::Jpeg(data) => {
                let rgb = image::load_from_memory_with_format(data, image::ImageFormat::Jpeg).ok()?.to_rgb8();
                egui::ColorImage::from_rgb([rgb.width() as usize, rgb.height() as usize], rgb.as_raw())
            }
        })
    }

    /// A small picture of it (its first frame), `width` wide.
    pub fn thumbnail(&self, width: usize) -> Option<egui::ColorImage> {
        let full = self.image(0)?;
        let [w, h] = full.size;
        let tw = width.min(w).max(1);
        let th = (h * tw / w.max(1)).max(1);
        let pixels = (0..th).flat_map(|y| (0..tw).map(move |x| (x, y))).map(|(x, y)| full.pixels[(y * h / th) * w + x * w / tw]).collect();
        Some(egui::ColorImage::new([tw, th], pixels))
    }

    fn set_info(&self, f: impl FnOnce(&mut Info)) {
        if let Ok(mut i) = self.info.lock() {
            f(&mut i);
        }
    }

    fn push(&self, frame: Frame) {
        if let Ok(mut f) = self.frames.write() {
            f.push(frame);
        }
    }

    fn read_picture(&self) -> Result<(), String> {
        let data = std::fs::read(&self.path).map_err(|e| format!("{}: {e}", self.path.display()))?;
        let img = image::load_from_memory(&data).map_err(|e| format!("{}: {e}", file_name(&self.path)))?;
        let img = if img.width() > MAX_PICTURE || img.height() > MAX_PICTURE { img.thumbnail(MAX_PICTURE, MAX_PICTURE) } else { img };
        let rgba = img.to_rgba8();
        let (w, h) = rgba.dimensions();
        self.set_info(|i| {
            (i.width, i.height, i.expected) = (w, h, 1);
            i.note = format!("{w}×{h} picture");
        });
        self.push(Frame { t: 0.0, px: Pixels::Rgba(rgba.into_raw()) });
        Ok(())
    }

    fn read_gif(&self) -> Result<(), String> {
        let data = std::fs::read(&self.path).map_err(|e| format!("{}: {e}", self.path.display()))?;
        let g = super::gif::decode(&data, BUDGET * 4).map_err(|e| format!("{}: {e}", file_name(&self.path)))?;
        // Big ones are made smaller (each frame is a whole picture).
        let (w, h) = if g.width > MAX_GIF { (MAX_GIF, (g.height * MAX_GIF / g.width).max(1)) } else { (g.width, g.height) };
        let n = g.frames.len();
        let mut used = 0;
        let mut kept = 0;
        for (t, px) in g.frames {
            let px = if (w, h) == (g.width, g.height) {
                px
            } else {
                let full = image::RgbaImage::from_raw(g.width, g.height, px).ok_or("a broken GIF frame")?;
                image::imageops::resize(&full, w, h, image::imageops::FilterType::Triangle).into_raw()
            };
            used += px.len();
            if used > BUDGET && kept > 0 {
                break;
            }
            self.push(Frame { t, px: Pixels::Rgba(px) });
            kept += 1;
        }
        self.set_info(|i| {
            (i.width, i.height, i.expected, i.duration) = (w, h, kept, if kept > 1 { g.duration } else { 0.0 });
            i.note = if kept > 1 { format!("{w}×{h} GIF, {kept} frames, {:.1} s", g.duration) } else { format!("{w}×{h} GIF") };
            if kept < n {
                i.note.push_str(&format!(" (the first {kept} of {n} frames)"));
            }
        });
        Ok(())
    }

    fn read_film(&self) -> Result<(), String> {
        let Some(exe) = ffmpeg::find() else {
            self.set_info(|i| i.needs_ffmpeg = true);
            return Err("Films need ffmpeg".into());
        };
        let out = ffmpeg::command(&exe).args(ffmpeg::probe_args(&self.path)).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::piped()).output().map_err(|e| format!("ffmpeg did not run: {e}"))?;
        let text = String::from_utf8_lossy(&out.stderr);
        let probe = ffmpeg::parse_probe(&text).ok_or_else(|| format!("{}: ffmpeg finds no film in it", file_name(&self.path)))?;
        let plan = ffmpeg::plan(&probe, FILM_BUDGET);
        let (w, h) = (plan.width, plan.height);
        let expected = plan.frames();
        let note = |mb: usize| {
            let mut n = format!("{w}×{h}, {:.0} frames a second, {mb} MB", plan.fps);
            if plan.cut {
                n.push_str(&format!(" (the first {:.0} s)", plan.seconds));
            }
            n
        };
        self.set_info(|i| {
            (i.width, i.height, i.expected, i.duration) = (w, h, expected, plan.seconds as f32);
            i.note = note((expected * plan.frame_bytes()) >> 20);
        });
        let mut child = ffmpeg::command(&exe).args(ffmpeg::decode_args(&self.path, &plan)).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::null()).spawn().map_err(|e| format!("ffmpeg did not run: {e}"))?;
        let stdout = child.stdout.take().ok_or("ffmpeg gave no output")?;
        let mut k = 0usize;
        let mut used = 0usize;
        let read = ffmpeg::read_jpegs(stdout, |px| {
            used += px.len();
            self.push(Frame { t: (k as f64 / plan.fps) as f32, px: Pixels::Jpeg(px) });
            k += 1;
            // Past the budget after all (busier pictures than guessed): the rest is left out.
            k < expected && used < FILM_BUDGET && !self.cancel.load(Ordering::Acquire)
        });
        let _ = child.kill();
        let _ = child.wait();
        let n = read.map_err(|e| e.to_string())?;
        if n == 0 {
            return Err(format!("{}: ffmpeg could not read it", file_name(&self.path)));
        }
        // It may be a little shorter than it said.
        let length = (n as f64 / plan.fps) as f32;
        self.set_info(|i| {
            i.expected = n;
            i.duration = length;
            i.note = note(used >> 20);
        });
        Ok(())
    }
}

fn file_name(p: &Path) -> String {
    p.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn files_are_known_by_their_first_bytes_then_their_name() {
        assert_eq!(kind_of(Path::new("a.mp4"), b"GIF89a.."), Kind::Gif);
        assert_eq!(kind_of(Path::new("photo"), b"\x89PNG\r\n\x1a\n"), Kind::Picture);
        assert_eq!(kind_of(Path::new("x.dat"), &[0xFF, 0xD8, 0xFF, 0xE0]), Kind::Picture);
        assert_eq!(kind_of(Path::new("clip.MOV"), b"\0\0\0 ftyp"), Kind::Film);
        assert_eq!(kind_of(Path::new("cat.GIF"), b""), Kind::Gif);
        assert!(takes(Path::new("C:/x/Film.MP4")));
        assert!(takes(Path::new("loop.gif")));
        assert!(!takes(Path::new("song.wav")));
        assert!(!takes(Path::new("noext")));
    }

    #[test]
    fn frames_are_found_by_time_and_a_gif_reads_from_disk() {
        let frames = (0..4).map(|k| Frame { t: k as f32 * 0.25, px: Pixels::Rgba(vec![k as u8 * 60, 0, 0, 255]) }).collect();
        let m = Media::from_frames(Path::new("m.gif"), Kind::Gif, 1, 1, frames, 1.0);
        assert_eq!(m.frame_at(0.0), Some(0));
        assert_eq!(m.frame_at(0.3), Some(1));
        assert_eq!(m.frame_at(0.75), Some(3));
        assert_eq!(m.frame_at(9.0), Some(3));
        assert_eq!(m.image(2).map(|i| i.pixels[0].r()), Some(120));
        assert_eq!(m.thumbnail(8).map(|t| t.size), Some([1, 1]));

        let dir = std::env::temp_dir().join(format!("krabton-video-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("two.gif");
        std::fs::write(&path, super::super::gif::tests::make_gif(4, 2, &[(vec![1u8; 8], 50, 1, None), (vec![2u8; 8], 50, 1, None)])).unwrap();
        let m = Media::open(&path);
        let start = std::time::Instant::now();
        while !m.is_done() && start.elapsed().as_secs() < 10 {
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let info = m.info();
        assert_eq!(info.error, None);
        assert_eq!((info.width, info.height, m.frame_count()), (4, 2, 2));
        assert!((info.duration - 1.0).abs() < 1e-6);
        assert_eq!(m.frame_at(0.6), Some(1));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_png_reads_and_a_missing_file_says_why() {
        let dir = std::env::temp_dir().join(format!("krabton-video-png-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("dot.png");
        image::RgbaImage::from_pixel(3, 2, image::Rgba([10, 200, 30, 255])).save(&path).unwrap();
        let wait = |m: &Arc<Media>| {
            let start = std::time::Instant::now();
            while !m.is_done() && start.elapsed().as_secs() < 10 {
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
        };
        let m = Media::open(&path);
        wait(&m);
        assert_eq!(m.kind, Kind::Picture);
        assert_eq!(m.image(0).map(|i| (i.size, i.pixels[0])), Some(([3, 2], egui::Color32::from_rgb(10, 200, 30))));
        let gone = Media::open(&dir.join("nothing.png"));
        wait(&gone);
        assert!(gone.info().error.is_some());
        let _ = std::fs::remove_dir_all(&dir);
    }
}

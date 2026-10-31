//! Films for the Video, through ffmpeg: Krabton has no video decoder of its
//! own, so it runs the `ffmpeg` program, which reads nearly every film
//! there is (`krabton_engine::ffmpeg` finds it: next to Krabton, in
//! Krabton's library, Documents/Krabton/Library/ffmpeg, where the Video can
//! fetch it, and on the PATH). A film is read whole, made smaller (640
//! pixels wide at most) and kept in memory as raw frames, so that it can
//! jump anywhere at once.

use std::ffi::OsString;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

// Where ffmpeg is: in the engine, so that anything in Krabton can use it.
pub use krabton_engine::ffmpeg::{command, exe_name, find, forget, library_dir, works};

/// What ffmpeg says about a film.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Probe {
    pub width: u32,
    pub height: u32,
    pub fps: f64,
    /// Seconds (None for a stream that does not say).
    pub duration: Option<f64>,
}

/// The arguments that make ffmpeg describe a file (it says it on stderr,
/// and fails for want of an output, which is fine).
pub fn probe_args(path: &Path) -> Vec<OsString> {
    vec!["-hide_banner".into(), "-nostdin".into(), "-i".into(), path.as_os_str().to_owned()]
}

/// Reads what ffmpeg writes about its input: the length from `Duration:`,
/// the size and frame rate from the first video stream. A film turned on
/// its side (a phone's) comes out turned upright, so the sizes swap.
pub fn parse_probe(text: &str) -> Option<Probe> {
    let duration = text.lines().find_map(|l| {
        let rest = l.trim().strip_prefix("Duration:")?;
        let stamp = rest.split(',').next()?.trim();
        let mut parts = stamp.split(':');
        let (h, m, s) = (parts.next()?.parse::<f64>().ok()?, parts.next()?.parse::<f64>().ok()?, parts.next()?.parse::<f64>().ok()?);
        Some(h * 3600.0 + m * 60.0 + s)
    });
    let video = text.lines().find(|l| l.contains("Stream #") && l.contains("Video:"))?;
    let after = &video[video.find("Video:")? + 6..];
    // The size: the first "WxH" that is a size (not "0x31637661", a tag).
    let (mut width, mut height) = (0, 0);
    for word in after.split(|c: char| c == ',' || c == ' ' || c == '[' || c == ']') {
        if let Some((a, b)) = word.split_once('x') {
            if let (Ok(w), Ok(h)) = (a.parse::<u32>(), b.parse::<u32>()) {
                if (1..=16384).contains(&w) && (1..=16384).contains(&h) {
                    (width, height) = (w, h);
                    break;
                }
            }
        }
    }
    if width == 0 {
        return None;
    }
    let rate_before = |unit: &str| {
        let words: Vec<&str> = after.split(',').map(str::trim).collect();
        words.iter().find_map(|w| w.strip_suffix(unit).and_then(|n| parse_rate(n.trim())))
    };
    let fps = rate_before(" fps").or_else(|| rate_before(" tbr")).filter(|f| *f > 0.0 && f.is_finite()).unwrap_or(25.0);
    // Turned a quarter: "rotation of -90.00 degrees" (newer), "rotate : 90" (older).
    let quarter = text.lines().any(|l| {
        let l = l.trim();
        let deg = if let Some(r) = l.strip_prefix("displaymatrix: rotation of ") {
            r.split_whitespace().next().and_then(|d| d.parse::<f64>().ok())
        } else if let Some(r) = l.strip_prefix("rotate") {
            r.trim_start().strip_prefix(':').and_then(|d| d.trim().parse::<f64>().ok())
        } else {
            None
        };
        deg.is_some_and(|d| (d.abs() - 90.0).abs() < 1.0 || (d.abs() - 270.0).abs() < 1.0)
    });
    if quarter {
        std::mem::swap(&mut width, &mut height);
    }
    Some(Probe { width, height, fps, duration })
}

/// "29.97", "30k" (30000), "24000/1001".
fn parse_rate(s: &str) -> Option<f64> {
    if let Some((a, b)) = s.split_once('/') {
        return Some(a.parse::<f64>().ok()? / b.parse::<f64>().ok()?.max(1e-9));
    }
    match s.strip_suffix('k') {
        Some(k) => k.parse::<f64>().ok().map(|v| v * 1000.0),
        None => s.parse().ok(),
    }
}

/// How a film is read into memory.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Plan {
    pub width: u32,
    pub height: u32,
    pub fps: f64,
    /// How much of it (seconds).
    pub seconds: f64,
    /// It is longer than that: only its start is kept.
    pub cut: bool,
}

impl Plan {
    /// Bytes of one frame (Y, then U and V at a quarter each).
    pub fn frame_bytes(&self) -> usize {
        yuv_bytes(self.width, self.height)
    }

    pub fn frames(&self) -> usize {
        (self.seconds * self.fps).ceil().max(1.0) as usize
    }
}

/// Bytes of a YUV 4:2:0 frame (both sides even).
pub fn yuv_bytes(w: u32, h: u32) -> usize {
    let (w, h) = (w as usize, h as usize);
    w * h + 2 * (w / 2) * (h / 2)
}

/// Sizes even (as 4:2:0 wants) and no wider than `max_w`, keeping its shape.
fn fit_width(p: &Probe, max_w: u32) -> (u32, u32) {
    let w = p.width.min(max_w).max(2);
    let h = (p.height as f64 * w as f64 / p.width.max(1) as f64).round().max(2.0) as u32;
    (w & !1, h & !1)
}

/// How to fit a film into `budget` bytes: 640 pixels wide at up to 30
/// frames a second; a long one gets fewer frames (down to 12 a second),
/// then fewer pixels (down to 320 wide), then fewer frames still (down to
/// 8), and only then is it cut short.
pub fn plan(p: &Probe, budget: usize) -> Plan {
    let length = p.duration.filter(|d| *d > 0.0).unwrap_or(60.0);
    let fits = |w: u32, h: u32, fps: f64| (length * fps).ceil() * yuv_bytes(w, h) as f64 <= budget as f64;
    let src_fps = p.fps.clamp(1.0, 60.0);
    let mut fps = src_fps.min(30.0);
    let (mut w, mut h) = fit_width(p, 640);
    let frames_for = |w: u32, h: u32| budget as f64 / yuv_bytes(w, h) as f64;
    if !fits(w, h, fps) {
        fps = (frames_for(w, h) / length).max(12.0f64.min(src_fps)).min(fps);
    }
    if !fits(w, h, fps) {
        for max_w in [480, 400, 320] {
            (w, h) = fit_width(p, max_w);
            if fits(w, h, fps) {
                break;
            }
        }
    }
    if !fits(w, h, fps) {
        fps = (frames_for(w, h) / length).max(8.0f64.min(src_fps)).min(fps);
    }
    let seconds = if fits(w, h, fps) { length } else { (frames_for(w, h) / fps).floor().max(1.0) };
    Plan { width: w, height: h, fps, seconds, cut: seconds < length - 1e-6 }
}

/// The arguments that make ffmpeg write the film as raw frames on its
/// output: sound left out, at the plan's rate and size, in 4:2:0 YUV with
/// the BT.601 colours and range, as long as the plan says.
pub fn decode_args(path: &Path, plan: &Plan) -> Vec<OsString> {
    let mut a: Vec<OsString> = ["-hide_banner", "-loglevel", "error", "-nostdin", "-i"].iter().map(OsString::from).collect();
    a.push(path.as_os_str().to_owned());
    a.push("-an".into());
    a.push("-sn".into());
    a.push("-t".into());
    a.push(format!("{:.3}", plan.seconds).into());
    a.push("-vf".into());
    a.push(format!("fps={:.4},scale={}:{}:flags=bilinear:out_color_matrix=bt601:out_range=tv", plan.fps, plan.width, plan.height).into());
    for s in ["-pix_fmt", "yuv420p", "-f", "rawvideo", "-"] {
        a.push(s.into());
    }
    a
}

/// Reads whole frames of `frame_bytes` from `input` and hands each to
/// `each` (false stops). A part frame at the end is dropped. Returns how
/// many were read.
pub fn read_frames(mut input: impl Read, frame_bytes: usize, mut each: impl FnMut(Vec<u8>) -> bool) -> std::io::Result<usize> {
    let mut n = 0;
    if frame_bytes == 0 {
        return Ok(0);
    }
    loop {
        let mut buf = vec![0u8; frame_bytes];
        let mut got = 0;
        while got < frame_bytes {
            match input.read(&mut buf[got..]) {
                Ok(0) => return Ok(n),
                Ok(k) => got += k,
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
                Err(e) => return Err(e),
            }
        }
        n += 1;
        if !each(buf) {
            return Ok(n);
        }
    }
}

/// Where a static ffmpeg for this computer can be fetched (a zip), and
/// about how big it is. Windows: gyan.dev's essentials build; macOS and
/// Linux: Martin Riedl's builds (Intel/AMD and ARM).
pub fn download() -> Option<(&'static str, u64)> {
    if cfg!(windows) && cfg!(target_arch = "x86_64") {
        Some(("https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip", 115_000_000))
    } else if cfg!(target_os = "macos") && cfg!(target_arch = "aarch64") {
        Some(("https://ffmpeg.martin-riedl.de/redirect/latest/macos/arm64/release/ffmpeg.zip", 28_000_000))
    } else if cfg!(target_os = "macos") && cfg!(target_arch = "x86_64") {
        Some(("https://ffmpeg.martin-riedl.de/redirect/latest/macos/amd64/release/ffmpeg.zip", 30_000_000))
    } else if cfg!(target_os = "linux") && cfg!(target_arch = "x86_64") {
        Some(("https://ffmpeg.martin-riedl.de/redirect/latest/linux/amd64/release/ffmpeg.zip", 30_000_000))
    } else if cfg!(target_os = "linux") && cfg!(target_arch = "aarch64") {
        Some(("https://ffmpeg.martin-riedl.de/redirect/latest/linux/arm64/release/ffmpeg.zip", 28_000_000))
    } else {
        None
    }
}

/// How to get ffmpeg by hand on this system.
pub fn install_hint() -> &'static str {
    if cfg!(windows) {
        "or install it yourself (winget install ffmpeg) and start Krabton again"
    } else if cfg!(target_os = "macos") {
        "or install it yourself (brew install ffmpeg)"
    } else {
        "or install it yourself (sudo apt install ffmpeg, sudo pacman -S ffmpeg, sudo dnf install ffmpeg)"
    }
}

/// How fetching ffmpeg goes.
#[derive(Clone, Debug, PartialEq)]
pub enum Fetch {
    Downloading { bytes: u64 },
    Unpacking,
    Ready(PathBuf),
    Failed(String),
}

/// Starts fetching ffmpeg in the background (see `download`).
pub fn start_fetch() -> Arc<Mutex<Fetch>> {
    let state = Arc::new(Mutex::new(Fetch::Downloading { bytes: 0 }));
    let s = state.clone();
    std::thread::spawn(move || {
        let result = fetch(&s);
        if result.is_ok() {
            forget();
        }
        if let Ok(mut st) = s.lock() {
            *st = match result {
                Ok(p) => Fetch::Ready(p),
                Err(e) => Fetch::Failed(e),
            };
        }
    });
    state
}

fn fetch(state: &Arc<Mutex<Fetch>>) -> Result<PathBuf, String> {
    let (url, _) = download().ok_or("there is no ffmpeg to fetch for this computer")?;
    let dir = library_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let zip = dir.join("ffmpeg.zip.part");
    let mut child = crate::piano_lib::curl()
        .args(["-sSfL", "--retry", "3", "--retry-delay", "2", "-o"])
        .arg(&zip)
        .arg(url)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("curl could not run ({e}); it comes with Windows 10 and later, macOS and most Linux systems"))?;
    let status = loop {
        if let Some(st) = child.try_wait().map_err(|e| e.to_string())? {
            break st;
        }
        let bytes = std::fs::metadata(&zip).map(|m| m.len()).unwrap_or(0);
        if let Ok(mut s) = state.lock() {
            *s = Fetch::Downloading { bytes };
        }
        std::thread::sleep(std::time::Duration::from_millis(250));
    };
    if !status.success() {
        let _ = std::fs::remove_file(&zip);
        return Err("downloading ffmpeg failed".into());
    }
    if let Ok(mut s) = state.lock() {
        *s = Fetch::Unpacking;
    }
    // Only the program (a zip also holds ffplay, ffprobe and documents).
    let unpacked = crate::singer_lib::unzip_some(&zip, &dir, |name| is_ffmpeg_entry(name));
    let _ = std::fs::remove_file(&zip);
    unpacked.map_err(|e| format!("ffmpeg arrived but could not be unpacked: {e}"))?;
    let exe = [dir.join(exe_name()), dir.join("bin").join(exe_name())].into_iter().find(|p| p.is_file()).ok_or("the download held no ffmpeg")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&exe, std::fs::Permissions::from_mode(0o755));
    }
    if !works(&exe) {
        return Err("the ffmpeg that came does not run on this computer".into());
    }
    Ok(exe)
}

/// Whether a file in the zip is the program itself.
fn is_ffmpeg_entry(name: &str) -> bool {
    let file = name.rsplit('/').next().unwrap_or(name);
    file.eq_ignore_ascii_case(exe_name())
}

#[cfg(test)]
mod tests {
    use super::*;

    const PHONE: &str = "Input #0, mov,mp4,m4a,3gp,3g2,mj2, from 'clip.mp4':
  Metadata:
    major_brand     : isom
  Duration: 00:01:02.50, start: 0.000000, bitrate: 8123 kb/s
  Stream #0:0[0x1](und): Video: h264 (High) (avc1 / 0x31637661), yuv420p(tv, bt709, progressive), 1920x1080 [SAR 1:1 DAR 16:9], 7990 kb/s, 29.97 fps, 29.97 tbr, 90k tbn (default)
      Metadata:
        handler_name    : VideoHandle
      Side data:
        displaymatrix: rotation of -90.00 degrees
  Stream #0:1[0x2](und): Audio: aac (LC) (mp4a / 0x6134706D), 48000 Hz, stereo, fltp, 128 kb/s (default)
At least one output file must be specified";

    #[test]
    fn ffmpeg_says_how_long_how_big_and_how_fast() {
        let p = parse_probe(PHONE).expect("a film");
        // Turned on its side: upright it is tall.
        assert_eq!((p.width, p.height), (1080, 1920));
        assert!((p.fps - 29.97).abs() < 1e-9);
        assert!((p.duration.unwrap() - 62.5).abs() < 1e-9);
        let webm = "Input #0, matroska,webm, from 'a.webm':\n  Duration: N/A, start: 0.000000, bitrate: N/A\n  Stream #0:0: Video: vp9 (Profile 0), yuv420p(tv), 640x360, SAR 1:1 DAR 16:9, 24000/1001 tbr, 1k tbn (default)\n";
        let p = parse_probe(webm).unwrap();
        assert_eq!((p.width, p.height, p.duration), (640, 360, None));
        assert!((p.fps - 23.976).abs() < 1e-3);
        let old_rotate = "  Duration: 00:00:05.00, start: 0.0\n  Stream #0:0: Video: h264, yuv420p, 1280x720, 30 fps, 30 tbr\n    Metadata:\n      rotate          : 90\n";
        assert_eq!(parse_probe(old_rotate).map(|p| (p.width, p.height)), Some((720, 1280)));
        // Only sound, or nothing: no film.
        assert!(parse_probe("  Duration: 00:03:00.00\n  Stream #0:0: Audio: mp3, 44100 Hz, stereo\n").is_none());
        assert!(parse_probe("clip.mp4: No such file or directory").is_none());
    }

    #[test]
    fn a_short_film_keeps_its_frames_and_a_long_one_is_fitted_into_memory() {
        let short = Probe { width: 1920, height: 1080, fps: 29.97, duration: Some(10.0) };
        let p = plan(&short, 384 << 20);
        assert_eq!((p.width, p.height), (640, 360));
        assert!((p.fps - 29.97).abs() < 1e-9);
        assert_eq!((p.seconds, p.cut), (10.0, false));
        assert_eq!(p.frame_bytes(), 640 * 360 * 3 / 2);
        // Four minutes: fewer frames, then fewer pixels, all of it kept.
        let long = Probe { duration: Some(240.0), ..short };
        let p = plan(&long, 384 << 20);
        assert!(!p.cut);
        assert!(p.fps >= 8.0 && p.width <= 640 && p.width >= 320, "{p:?}");
        assert!(p.frames() * p.frame_bytes() <= 384 << 20);
        // An hour: only its start.
        let film = Probe { duration: Some(3600.0), ..short };
        let p = plan(&film, 384 << 20);
        assert!(p.cut && p.seconds < 3600.0 && p.seconds > 60.0, "{p:?}");
        assert_eq!(p.width, 320);
        assert!(p.frames() * p.frame_bytes() <= (384 << 20) + p.frame_bytes());
        // Small and slow films stay as they are; sizes are even.
        let tiny = Probe { width: 321, height: 241, fps: 10.0, duration: Some(3.0) };
        let p = plan(&tiny, 384 << 20);
        assert_eq!((p.width, p.height, p.fps), (320, 240, 10.0));
    }

    #[test]
    fn the_decode_command_asks_for_raw_frames_of_the_plan() {
        let plan = Plan { width: 640, height: 360, fps: 25.0, seconds: 12.5, cut: false };
        let args: Vec<String> = decode_args(Path::new("my film.mov"), &plan).into_iter().map(|a| a.to_string_lossy().into_owned()).collect();
        let at = |s: &str| args.iter().position(|a| a == s).unwrap_or_else(|| panic!("{s} in {args:?}"));
        assert_eq!(args[at("-i") + 1], "my film.mov");
        assert_eq!(args[at("-t") + 1], "12.500");
        assert_eq!(args[at("-vf") + 1], "fps=25.0000,scale=640:360:flags=bilinear:out_color_matrix=bt601:out_range=tv");
        assert_eq!(args[at("-pix_fmt") + 1], "yuv420p");
        assert_eq!(args[at("-f") + 1], "rawvideo");
        assert!(args.contains(&"-an".to_string()));
        assert_eq!(args.last().map(String::as_str), Some("-"));
        let probe: Vec<String> = probe_args(Path::new("x.mp4")).into_iter().map(|a| a.to_string_lossy().into_owned()).collect();
        assert_eq!(probe, vec!["-hide_banner", "-nostdin", "-i", "x.mp4"]);
    }

    /// Hands out its bytes a few at a time, as a pipe does.
    struct Trickle(Vec<u8>, usize);

    impl Read for Trickle {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            let n = buf.len().min(7).min(self.0.len() - self.1);
            buf[..n].copy_from_slice(&self.0[self.1..self.1 + n]);
            self.1 += n;
            Ok(n)
        }
    }

    #[test]
    fn frames_are_read_whole_from_a_trickling_pipe() {
        let data: Vec<u8> = (0..50u8).collect();
        let mut frames = Vec::new();
        let n = read_frames(Trickle(data.clone(), 0), 12, |f| {
            frames.push(f);
            true
        })
        .unwrap();
        // Four whole frames; the last two bytes are a part frame.
        assert_eq!(n, 4);
        assert_eq!(frames[3], data[36..48].to_vec());
        // Stopping early.
        let n = read_frames(Trickle(data, 0), 12, |_| false).unwrap();
        assert_eq!(n, 1);
    }

    #[test]
    fn only_the_program_is_taken_out_of_the_zip() {
        let exe = exe_name();
        assert!(is_ffmpeg_entry(&format!("bin/{exe}")));
        assert!(is_ffmpeg_entry(exe));
        assert!(!is_ffmpeg_entry("bin/ffprobe.exe"));
        assert!(!is_ffmpeg_entry("doc/ffmpeg.html"));
        assert!(library_dir().ends_with("ffmpeg"));
    }
}

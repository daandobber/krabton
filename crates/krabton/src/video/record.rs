//! Record Video: what the Video shows and the master's sound, recorded
//! live as it is played, into a film. The stage is read back from the
//! graphics card after it is drawn (`stage::Stage::take_pixels`) and the
//! master's sound taken from the engine's tap (`EngineStatus::record_master`,
//! written by the audio thread without locks). A thread of its own hands
//! the frames to one ffmpeg (a film without sound, at a steady rate: a
//! frame missed is the last one again, so the picture keeps time) and
//! writes the sound to a file beside it; stopping puts the two together.
//! Picture and sound start at the same moment and each keeps its own
//! clock (the frames by the time since, the sound by its samples), so they
//! stay together however the window's frames come.

use super::stage::{STAGE_H, STAGE_W};
use krabton_engine::engine::{EngineStatus, RECORD_LEN};
use std::ffi::OsString;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender, TrySendError};
use std::time::Instant;

/// Frame `k` of the film for the writer (RGBA, the stage's size, bottom
/// row first; empty: only how far the film goes).
struct Piece(u64, Vec<u8>);

/// A recording going on.
pub struct Recorder {
    tx: Option<SyncSender<Piece>>,
    worker: Option<std::thread::JoinHandle<Result<(), String>>>,
    /// The sound goes to a writer of its own, never waiting.
    sound_tx: Option<std::sync::mpsc::Sender<Vec<f32>>>,
    sound_worker: Option<std::thread::JoinHandle<Result<(), String>>>,
    started: Instant,
    fps: u32,
    /// The next frame due, and the sound read so far (the tap's count).
    next_frame: u64,
    read_to: u64,
    pub out: PathBuf,
    tmp: PathBuf,
    sample_rate: u32,
    ffmpeg: PathBuf,
    scratch: (Vec<f32>, Vec<f32>),
}

/// How many frames of a steady `fps` are due `secs` after the start.
pub fn frames_due(secs: f64, fps: u32) -> u64 {
    (secs.max(0.0) * fps as f64).floor() as u64 + 1
}

/// The arguments for the film without sound: raw RGBA frames of the stage
/// on its input, turned the right way up and made `width` by `height`.
pub fn picture_args(fps: u32, width: u32, height: u32, out: &Path) -> Vec<OsString> {
    let mut a: Vec<OsString> = Vec::new();
    for s in ["-hide_banner", "-loglevel", "error", "-nostdin", "-y", "-f", "rawvideo", "-pix_fmt", "rgba"] {
        a.push(s.into());
    }
    a.push("-s".into());
    a.push(format!("{STAGE_W}x{STAGE_H}").into());
    a.push("-r".into());
    a.push(fps.to_string().into());
    for s in ["-i", "-", "-vf"] {
        a.push(s.into());
    }
    a.push(format!("vflip,scale={width}:{height}").into());
    for s in ["-c:v", "libx264", "-preset", "veryfast", "-crf", "20", "-pix_fmt", "yuv420p"] {
        a.push(s.into());
    }
    a.push(out.as_os_str().to_owned());
    a
}

/// The arguments that put the film and the sound (raw 32-bit float stereo
/// at `sample_rate`) together, the film as it is, the sound in AAC.
pub fn mux_args(film: &Path, sound: &Path, sample_rate: u32, out: &Path) -> Vec<OsString> {
    let mut a: Vec<OsString> = Vec::new();
    for s in ["-hide_banner", "-loglevel", "error", "-nostdin", "-y", "-i"] {
        a.push(s.into());
    }
    a.push(film.as_os_str().to_owned());
    for s in ["-f", "f32le", "-ar"] {
        a.push(s.into());
    }
    a.push(sample_rate.to_string().into());
    for s in ["-ac", "2", "-i"] {
        a.push(s.into());
    }
    a.push(sound.as_os_str().to_owned());
    for s in ["-map", "0:v", "-map", "1:a", "-c:v", "copy", "-c:a", "aac", "-b:a", "256k", "-movflags", "+faststart"] {
        a.push(s.into());
    }
    a.push(out.as_os_str().to_owned());
    a
}

/// Where recordings go: Documents/Krabton/Recorded.
pub fn recorded_dir() -> PathBuf {
    let lib = super::ffmpeg::library_dir();
    lib.parent().and_then(|p| p.parent()).map_or_else(|| PathBuf::from("Recorded"), |k| k.join("Recorded"))
}

/// A name for a recording of `set` that is not taken in `dir`.
pub fn free_name(dir: &Path, set: &str) -> PathBuf {
    let base = if set.trim().is_empty() { "Krabton" } else { set.trim() };
    (1..).map(|k| dir.join(format!("{base} recording {k}.mp4"))).find(|p| !p.exists()).unwrap_or_else(|| dir.join("recording.mp4"))
}

impl Recorder {
    /// Starts recording into `out` (`width` by `height`, `fps` a second);
    /// the engine's tap is switched on.
    pub fn start(status: &EngineStatus, ffmpeg: &Path, out: PathBuf, width: u32, height: u32, fps: u32) -> Result<Self, String> {
        if let Some(dir) = out.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        let tmp = std::env::temp_dir().join(format!("krabton-record-{}-{}", std::process::id(), out.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default()));
        std::fs::create_dir_all(&tmp).map_err(|e| format!("{}: {e}", tmp.display()))?;
        let film = tmp.join("picture.mp4");
        let mut child = super::ffmpeg::command(ffmpeg)
            .args(picture_args(fps, width, height, &film))
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| format!("ffmpeg did not run: {e}"))?;
        let stdin = child.stdin.take().ok_or("ffmpeg takes no frames")?;
        let sound = std::fs::File::create(tmp.join("sound.raw")).map_err(|e| e.to_string())?;
        let (tx, rx) = sync_channel::<Piece>(8);
        let worker = std::thread::spawn(move || write_frames(rx, stdin, child));
        let (sound_tx, sound_rx) = std::sync::mpsc::channel::<Vec<f32>>();
        let sound_worker = std::thread::spawn(move || write_sound(sound_rx, sound));
        status.recording.store(true, Ordering::Relaxed);
        let read_to = status.record_master.written();
        Ok(Self {
            tx: Some(tx),
            worker: Some(worker),
            sound_tx: Some(sound_tx),
            sound_worker: Some(sound_worker),
            started: Instant::now(),
            fps: fps.max(1),
            next_frame: 0,
            read_to,
            out,
            tmp,
            sample_rate: status.sample_rate.load() as u32,
            ffmpeg: ffmpeg.to_path_buf(),
            scratch: (Vec::new(), Vec::new()),
        })
    }

    /// How long it has recorded (seconds) and how much it wrote so far (bytes).
    pub fn progress(&self) -> (f64, u64) {
        let size = |f: &str| std::fs::metadata(self.tmp.join(f)).map_or(0, |m| m.len());
        (self.started.elapsed().as_secs_f64(), size("picture.mp4") + size("sound.raw"))
    }

    /// One frame of the window: the sound that came, and the stage last
    /// drawn as every frame now due (none when it is not time yet).
    pub fn frame(&mut self, status: &EngineStatus, pixels: Option<Vec<u8>>) {
        // The sound: everything since the last read (silence for what the tap no longer holds).
        let end = status.record_master.written();
        let new = end.saturating_sub(self.read_to);
        if new > 0 {
            let lost = new.saturating_sub(RECORD_LEN as u64 / 2) as usize;
            let n = (new as usize - lost).min(RECORD_LEN / 2);
            status.record_master.read(n, &mut self.scratch.0, &mut self.scratch.1);
            let mut pcm = vec![0.0f32; lost * 2];
            pcm.extend(self.scratch.0.iter().zip(&self.scratch.1).flat_map(|(l, r)| [*l, *r]));
            self.read_to = end;
            if let Some(stx) = &self.sound_tx {
                let _ = stx.send(pcm);
            }
        }
        let Some(tx) = &self.tx else { return };
        // The picture: the frame due now (the writer fills in any it missed).
        let due = frames_due(self.started.elapsed().as_secs_f64(), self.fps);
        if let (Some(px), true) = (pixels, due > self.next_frame) {
            let k = due - 1;
            match tx.try_send(Piece(k, px)) {
                Ok(()) | Err(TrySendError::Full(_)) => self.next_frame = due,
                Err(TrySendError::Disconnected(_)) => self.tx = None,
            }
        }
    }

    /// Stops: the frames and the sound are finished, then put together in
    /// the background. The film's path, or why it failed, comes from the handle.
    pub fn stop(mut self, status: &EngineStatus) -> std::thread::JoinHandle<Result<PathBuf, String>> {
        // The last sound, then the tap off.
        self.frame(status, None);
        status.recording.store(false, Ordering::Relaxed);
        let secs = self.started.elapsed().as_secs_f64();
        let frames = frames_due(secs, self.fps);
        let tx = self.tx.take();
        let worker = self.worker.take();
        drop(self.sound_tx.take());
        let sound_worker = self.sound_worker.take();
        let (tmp, out, ffmpeg, sr) = (self.tmp.clone(), self.out.clone(), self.ffmpeg.clone(), self.sample_rate);
        std::thread::spawn(move || {
            if let Some(tx) = &tx {
                // Up to the last frame due, so the picture lasts as long as the sound.
                let _ = tx.send(Piece(frames.saturating_sub(1), Vec::new()));
            }
            drop(tx);
            let join = |w: Option<std::thread::JoinHandle<Result<(), String>>>| w.map_or(Err("lost".to_string()), |w| w.join().unwrap_or(Err("the recorder stopped".into())));
            let wrote = join(worker).and(join(sound_worker));
            let result = wrote.and_then(|_| {
                let st = super::ffmpeg::command(&ffmpeg).args(mux_args(&tmp.join("picture.mp4"), &tmp.join("sound.raw"), sr, &out)).stdin(std::process::Stdio::null()).output().map_err(|e| format!("ffmpeg did not run: {e}"))?;
                if st.status.success() {
                    Ok(out.clone())
                } else {
                    Err(format!("ffmpeg could not make the film: {}", String::from_utf8_lossy(&st.stderr).lines().last().unwrap_or("")))
                }
            });
            let _ = std::fs::remove_dir_all(&tmp);
            result
        })
    }
}

/// The sound's writer: raw 32-bit float stereo, as it comes.
fn write_sound(rx: std::sync::mpsc::Receiver<Vec<f32>>, file: std::fs::File) -> Result<(), String> {
    let mut out = std::io::BufWriter::new(file);
    for pcm in rx {
        let raw: Vec<u8> = pcm.iter().flat_map(|v| v.to_le_bytes()).collect();
        out.write_all(&raw).map_err(|e| e.to_string())?;
    }
    out.flush().map_err(|e| e.to_string())
}

/// The frames' writer: to ffmpeg in order (a missed one is the one before
/// again; an empty one only says how far the film goes).
fn write_frames(rx: Receiver<Piece>, mut stdin: std::process::ChildStdin, mut child: std::process::Child) -> Result<(), String> {
    let mut last: Option<Vec<u8>> = None;
    let mut next = 0u64;
    let mut failed = None;
    for Piece(k, px) in rx {
        let px = if px.is_empty() { last.clone() } else { Some(px) };
        let Some(px) = px else { continue };
        while next <= k && failed.is_none() {
            // Frames missed: the one before (or this one) again.
            let f = if next < k { last.as_ref().unwrap_or(&px) } else { &px };
            if let Err(e) = stdin.write_all(f) {
                failed = Some(format!("ffmpeg stopped taking frames: {e}"));
            }
            next += 1;
        }
        last = Some(px);
    }
    drop(stdin);
    let st = child.wait().map_err(|e| e.to_string())?;
    if let Some(e) = failed {
        return Err(e);
    }
    if !st.success() {
        return Err("ffmpeg could not make the picture".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frames_keep_a_steady_rate_whatever_the_window_does() {
        // At once: the first frame; a third of a second at 30: eleven.
        assert_eq!(frames_due(0.0, 30), 1);
        assert_eq!(frames_due(1.0 / 3.0 + 1e-9, 30), 11);
        // A minute at 25: as many frames as the sound lasts, one more begun.
        assert_eq!(frames_due(60.0, 25), 1501);
        // The sound of a minute at 48 kHz and the picture end together (within a frame).
        let sound_secs = (48_000 * 60) as f64 / 48_000.0;
        let picture_secs = (frames_due(60.0, 25) - 1) as f64 / 25.0;
        assert!((sound_secs - picture_secs).abs() <= 1.0 / 25.0);
    }

    #[test]
    fn the_picture_and_the_sound_are_put_together_as_they_are() {
        let pic: Vec<String> = picture_args(30, 1280, 720, Path::new("C:/tmp/picture.mp4")).into_iter().map(|a| a.to_string_lossy().into_owned()).collect();
        let at = |args: &[String], s: &str| args.iter().position(|a| a == s).unwrap();
        assert_eq!(pic[at(&pic, "-pix_fmt") + 1], "rgba");
        assert_eq!(pic[at(&pic, "-s") + 1], format!("{STAGE_W}x{STAGE_H}"));
        assert_eq!(pic[at(&pic, "-r") + 1], "30");
        assert_eq!(pic[at(&pic, "-vf") + 1], "vflip,scale=1280:720", "the stage is read bottom row first");
        let mux: Vec<String> = mux_args(Path::new("a/picture.mp4"), Path::new("a/sound.raw"), 44_100, Path::new("b/my set.mp4")).into_iter().map(|a| a.to_string_lossy().into_owned()).collect();
        let inputs: Vec<&String> = mux.iter().enumerate().filter(|(i, _)| *i > 0 && mux[i - 1] == "-i").map(|(_, a)| a).collect();
        assert_eq!(inputs, ["a/picture.mp4", "a/sound.raw"]);
        assert_eq!(mux[at(&mux, "-ar") + 1], "44100");
        assert!(mux.windows(2).any(|w| w == ["-c:v", "copy"]) && mux.windows(2).any(|w| w == ["-f", "f32le"]));
        assert_eq!(mux.last().map(String::as_str), Some("b/my set.mp4"));
        // Names that are free.
        let dir = std::env::temp_dir().join(format!("krabton-rec-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let first = free_name(&dir, "Song");
        assert!(first.ends_with("Song recording 1.mp4"));
        std::fs::write(&first, b"x").unwrap();
        assert!(free_name(&dir, "Song").ends_with("Song recording 2.mp4"));
        let _ = std::fs::remove_dir_all(&dir);
        assert!(recorded_dir().ends_with("Recorded"));
    }
}

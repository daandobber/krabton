//! Offline (faster than real-time) rendering and WAV export.

use crate::engine::{AtomicF32, Command, Engine};
use crate::plugin::AudioProcessor;
use crate::project::Project;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Default)]
pub struct RenderProgress {
    pub fraction: AtomicF32Wrapper,
    pub cancel: AtomicBool,
}

/// `AtomicF32` with a `Default`.
pub struct AtomicF32Wrapper(pub AtomicF32);

impl Default for AtomicF32Wrapper {
    fn default() -> Self {
        Self(AtomicF32::new(0.0))
    }
}

impl RenderProgress {
    pub fn fraction(&self) -> f32 {
        self.fraction.0.load()
    }
}

pub enum RenderSource {
    /// Render the arrangement between two beat positions.
    Arrangement { start: f64, end: f64 },
    /// Launch a scene and render a number of beats of it.
    Scene { scene: usize, beats: f64 },
}

/// Renders interleaved stereo audio. Returns `None` when cancelled.
pub fn render(project: &Project, source: RenderSource, sample_rate: u32, tail_beats: f64, progress: Option<&RenderProgress>) -> Option<Vec<f32>> {
    render_with_plugins(project, source, sample_rate, tail_beats, progress, Vec::new()).0
}

/// Like [`render`], with plugin processors for `Instrument::Plugin` tracks
/// (keyed by instance id). The processors are returned, stopped, so the
/// caller can drop them on the UI thread.
pub fn render_with_plugins(
    project: &Project,
    source: RenderSource,
    sample_rate: u32,
    tail_beats: f64,
    progress: Option<&RenderProgress>,
    processors: Vec<(u64, Box<dyn AudioProcessor>)>,
) -> (Option<Vec<f32>>, Vec<Box<dyn AudioProcessor>>) {
    render_watched(project, source, sample_rate, tail_beats, progress, processors, f64::INFINITY, |_, _| {})
}

/// Like [`render_with_plugins`], stopping every `every` frames (it may be a
/// fraction: the stops fall on the nearest frame, without drifting) to hand
/// `watch` the engine's status and the stop's number, as the Video reads
/// them while it plays (where the song is, the notes struck, the sound it
/// listens to). The first stop is after the first `every` frames.
#[allow(clippy::too_many_arguments)]
pub fn render_watched(
    project: &Project,
    source: RenderSource,
    sample_rate: u32,
    tail_beats: f64,
    progress: Option<&RenderProgress>,
    processors: Vec<(u64, Box<dyn AudioProcessor>)>,
    every: f64,
    mut watch: impl FnMut(&crate::engine::EngineStatus, usize),
) -> (Option<Vec<f32>>, Vec<Box<dyn AudioProcessor>>) {
    let mut p = project.clone();
    p.loop_enabled = false;
    p.metronome = false;
    let bpm = p.bpm;
    let (mut engine, handle) = Engine::new(sample_rate as f32, Arc::new(p));
    for (instance, processor) in processors {
        handle.send(Command::AttachProcessor { instance, processor });
    }
    let beats = match source {
        RenderSource::Arrangement { start, end } => {
            handle.send(Command::SetPosition(start));
            handle.send(Command::Play);
            (end - start).max(0.0)
        }
        RenderSource::Scene { scene, beats } => {
            handle.send(Command::LaunchScene(scene));
            beats
        }
    };
    let play_frames = (beats * 60.0 / bpm * sample_rate as f64) as usize;
    let tail_frames = (tail_beats * 60.0 / bpm * sample_rate as f64) as usize;
    let total = play_frames + tail_frames;
    let mut out = Vec::with_capacity(total * 2);
    let mut buf = vec![0.0f32; 2048];
    let mut done = 0;
    let mut stopped = false;
    let status = handle.status.clone();
    // The next stop for `watch`.
    let mut stop = 0usize;
    let stop_at = |k: usize| if every.is_finite() && every > 0.0 { ((k + 1) as f64 * every).round() as usize } else { usize::MAX };
    while done < total {
        if let Some(pr) = progress {
            if pr.cancel.load(Ordering::Relaxed) {
                return (None, engine.take_processors());
            }
            pr.fraction.0.store(done as f32 / total.max(1) as f32);
        }
        if !stopped && done >= play_frames {
            // Let reverbs and delays ring out without new notes.
            handle.send(Command::StopAllClips);
            handle.send(Command::Stop);
            stopped = true;
        }
        let n = (total - done).min(1024).min(if stopped { usize::MAX } else { play_frames - done }).min(stop_at(stop).saturating_sub(done)).max(1);
        let chunk = &mut buf[..n * 2];
        engine.process(chunk);
        out.extend_from_slice(chunk);
        done += n;
        handle.collect_garbage();
        while done >= stop_at(stop) {
            watch(&status, stop);
            stop += 1;
        }
    }
    if let Some(pr) = progress {
        pr.fraction.0.store(1.0);
    }
    (Some(out), engine.take_processors())
}

/// The audio of a clip as it plays at `bpm`, at the sample's own rate
/// (interleaved stereo): `length` beats from `offset` into its playing (its
/// loop `loop_len` beats), warped, with its gain, pan and fades.
pub fn clip_audio(a: &crate::project::AudioClip, loop_len: f64, offset: f64, length: f64, bpm: f64) -> (u32, Vec<f32>) {
    let sr = a.sample.sample_rate;
    let spb = sr as f64 * 60.0 / bpm.max(1.0);
    let n = (length.max(0.0) * spb).round() as usize;
    let (mut l, mut r) = (vec![0.0f32; n], vec![0.0f32; n]);
    let mut st = crate::warp::Stretch::default();
    let at = crate::warp::Place { origin: -offset, region: (0.0, length.max(0.0)), fades: (a.fade_in, if a.looping { 0.0 } else { a.fade_out }), key: 1 };
    let mut i = 0;
    while i < n {
        let seg = (n - i).min(4096);
        crate::warp::render(a, loop_len, at, i as f64 / spb, spb, sr as f32, seg, &mut l[i..], &mut r[i..], &mut st);
        i += seg;
    }
    (sr, l.iter().zip(&r).flat_map(|(x, y)| [*x, *y]).collect())
}

/// Writes interleaved stereo float samples as a 24-bit WAV file.
pub fn write_wav(path: &Path, sample_rate: u32, interleaved: &[f32]) -> Result<(), String> {
    let spec = hound::WavSpec { channels: 2, sample_rate, bits_per_sample: 24, sample_format: hound::SampleFormat::Int };
    let mut w = hound::WavWriter::create(path, spec).map_err(|e| format!("{}: {e}", path.display()))?;
    const MAX: f32 = 8_388_607.0;
    for &s in interleaved {
        w.write_sample((s.clamp(-1.0, 1.0) * MAX) as i32).map_err(|e| e.to_string())?;
    }
    w.finalize().map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clip_audio_is_the_clip_as_it_plays() {
        use crate::project::AudioClip;
        use crate::sample::{SampleData, SampleRef};
        // One second of a ramp at 120 BPM: two beats.
        let ramp: Vec<f32> = (0..1000).map(|i| i as f32 / 1000.0).collect();
        let a = AudioClip::new(SampleRef(std::sync::Arc::new(SampleData::from_buffers("ramp", None, 1000, ramp.clone(), ramp))), 120.0);
        let (sr, out) = clip_audio(&a, 2.0, 0.0, 2.0, 120.0);
        assert_eq!(sr, 1000);
        assert_eq!(out.len(), 2000);
        assert!((out[2 * 500] - 0.5).abs() < 1e-3);
        // From beat 1 for two beats: the second half, then the loop again.
        let (_, out) = clip_audio(&a, 2.0, 1.0, 2.0, 120.0);
        assert!((out[2 * 100] - 0.6).abs() < 1e-3);
        assert!((out[2 * 600] - 0.1).abs() < 1e-3);
    }

    #[test]
    fn renders_demo_scene() {
        let p = crate::demo::demo_project();
        let out = render(&p, RenderSource::Scene { scene: 3, beats: 4.0 }, 44100, 1.0, None).unwrap();
        let expected = ((5.0 * 60.0 / p.bpm) * 44100.0) as usize * 2;
        assert!((out.len() as i64 - expected as i64).abs() < 8, "{} vs {}", out.len(), expected);
        let peak = out.iter().fold(0.0f32, |m, v| m.max(v.abs()));
        assert!(peak > 0.1, "peak {peak}");
    }

    #[test]
    fn a_watched_render_stops_on_every_video_frame() {
        let p = crate::demo::demo_project();
        // 30 frames a second at 44.1 kHz: 1470 samples a frame.
        let mut stops = Vec::new();
        let (out, _) = render_watched(&p, RenderSource::Scene { scene: 3, beats: 4.0 }, 44100, 0.0, None, Vec::new(), 44100.0 / 30.0, |st, k| {
            stops.push((k, st.song_pos.load(), st.playing.load(std::sync::atomic::Ordering::Relaxed)));
        });
        let frames = out.unwrap().len() / 2;
        // Four beats at the demo's tempo, a stop each 1470 samples, numbered in order.
        assert_eq!(stops.len(), frames / 1470);
        assert!(stops.iter().enumerate().all(|(i, s)| s.0 == i));
        // The song moves on between stops: a frame of it each time.
        let beats_per_frame = p.bpm / 60.0 / 30.0;
        let step = stops[10].1 - stops[9].1;
        assert!((step - beats_per_frame).abs() < 1e-3, "{step} vs {beats_per_frame}");
        assert!(stops[5].2, "it plays");
        // The same audio as without the stops.
        let plain = render(&p, RenderSource::Scene { scene: 3, beats: 4.0 }, 44100, 0.0, None).unwrap();
        assert_eq!(plain.len(), frames * 2);
    }

    #[test]
    fn wav_roundtrip() {
        let dir = std::env::temp_dir().join(format!("krabton-wav-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("t.wav");
        let data: Vec<f32> = (0..2000).map(|i| ((i as f32) * 0.01).sin() * 0.5).collect();
        write_wav(&path, 48000, &data).unwrap();
        let s = crate::sample::SampleData::from_wav(&path).unwrap();
        assert_eq!(s.frames(), 1000);
        assert!((s.left[10] - data[20]).abs() < 1e-4);
        let _ = std::fs::remove_dir_all(&dir);
    }
}

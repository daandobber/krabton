//! Convert to MIDI, as in Live: an audio clip heard as notes on a new MIDI
//! track (its melody, its chords or its drums), the new clip at the same
//! place and as long as the audio's loop, its notes on the clip's beats
//! through its warp markers.
//!
//! Melody follows the pitch of one voice (after de Cheveigné and Kawahara's
//! YIN, 2002, and Mauch and Dixon's pYIN, 2014): each frame's difference
//! function (by FFT), its dips as candidate periods, each as likely as the
//! share of thresholds (a Beta(2, 18) spread of them) under which YIN would
//! pick it, then the likeliest path through them and silence (a jump of an
//! octave costs, so the octave errors of a frame or two do not stay). The
//! path is cut into notes where the pitch settles on another semitone (a
//! glide goes to the note it leaves, up to halfway; a vibrato of half a
//! semitone either way stays one note), where it goes silent, and where
//! the sound strikes again; each note's pitch is the middle of its frames,
//! its velocity how loud it is.
//!
//! Harmony cuts the sound at its hits and its beats and finds the notes of
//! each piece in its spectrum (after Klapuri's iterative estimation, 2006):
//! the pitch whose harmonics sum the loudest, its partials then taken out
//! (as much as a smooth spectrum of it would have, so a partial it shares
//! with another note keeps the rest), again until what is left is weak. A
//! chord of three to five notes, the bass among them; pieces with the same
//! chord in a row are one.
//!
//! Drums takes the hits (see `analysis`) and hears what each adds below
//! 150 Hz (a kick), in the middle (the noise of a snare) and in the highs
//! alone (a hat; open when its highs ring on), on the notes of a General
//! MIDI kit, which Krabton's kits use too.

use crate::analysis;
use crate::dsp::fft::Fft;
use crate::project::{ArrangementClip, AudioClip, Clip, ClipContent, Instrument, Note, Project};
use crate::sample::SampleData;
use crate::slicing::{SliceClip, SlicePlace};
use crate::warp::WarpMode;

/// What a clip is heard as.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Convert {
    Melody,
    Harmony,
    Drums,
}

impl Convert {
    pub const ALL: [Convert; 3] = [Convert::Melody, Convert::Harmony, Convert::Drums];

    pub fn name(self) -> &'static str {
        match self {
            Convert::Melody => "Melody",
            Convert::Harmony => "Harmony",
            Convert::Drums => "Drums",
        }
    }

    /// The instrument of the new track: a lead for a melody, keys for
    /// chords, a drum machine for drums.
    pub fn instrument(self) -> Instrument {
        match self {
            Convert::Melody => Instrument::analog("Mono Lead"),
            Convert::Harmony => Instrument::analog("Poly Keys"),
            Convert::Drums => Instrument::drums("909 Classic"),
        }
    }
}

/// The notes of a General MIDI kit (and of Krabton's kits).
pub const KICK: u8 = 36;
pub const SNARE: u8 = 38;
pub const CLOSED_HAT: u8 = 42;
pub const OPEN_HAT: u8 = 46;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ConvertOptions {
    pub kind: Convert,
    /// The notes' starts and ends onto this grid (beats), or where they are.
    pub quantize: Option<f64>,
}

/// A note heard: its pitch, where it starts and ends (seconds of what was
/// heard) and how loud it is (0..1).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Heard {
    pub pitch: u8,
    pub start: f64,
    pub end: f64,
    pub velocity: f32,
}

/// The clip beats that are heard: its loop when it loops (`len` beats from
/// the loop's start), else from its Start to its End marker.
fn clip_beats(a: &AudioClip, len: f64) -> (f64, f64) {
    if a.looping {
        (a.loop_start, a.loop_start + len.max(0.0))
    } else {
        (a.start, a.end_beat().max(a.start))
    }
}

/// How far (semitones) the clip plays its sound away from as recorded:
/// Transpose and Detune, or in Re-Pitch the tempo it is played at.
fn pitch_shift(a: &AudioClip, b0: f64, b1: f64, project_bpm: f64) -> f64 {
    if a.warp && a.mode == WarpMode::RePitch {
        let spb = if b1 - b0 > 1e-6 { (a.secs_at(b1) - a.secs_at(b0)) / (b1 - b0) } else { 60.0 / a.bpm.max(1.0) };
        12.0 * (spb / (60.0 / project_bpm.max(1.0))).log2()
    } else {
        a.transpose as f64 + a.detune as f64 / 100.0
    }
}

/// The notes of audio clip `a` (a clip `len` beats long, in a song at
/// `project_bpm` with `beats_per_bar`), as `opts` hears them, in beats of
/// the new clip, and that clip's length. `progress` hears how far it is
/// (0..1). Takes a while on a long clip: off the UI's thread.
pub fn convert(a: &AudioClip, len: f64, opts: &ConvertOptions, project_bpm: f64, progress: &mut dyn FnMut(f32)) -> Result<(Vec<Note>, f64), String> {
    let (b0, b1) = clip_beats(a, len);
    if b1 - b0 < 1e-6 {
        return Err("The clip has no sound to hear".into());
    }
    let src: &SampleData = if a.reverse { a.sample.reversed() } else { &a.sample };
    let sr = src.sample_rate.max(1);
    let frames = src.frames();
    let at = |b: f64| ((a.secs_at(b) * sr as f64).round().max(0.0) as usize).min(frames);
    let (f0, f1) = (at(b0), at(b1));
    if f1 <= f0 + sr as usize / 50 {
        return Err("The clip has no sound to hear".into());
    }
    let x: Vec<f32> = (f0..f1).map(|i| 0.5 * (src.left[i] + src.right[i])).collect();
    if x.iter().fold(0.0f32, |m, v| m.max(v.abs())) < 1e-4 {
        return Err("The clip is silent".into());
    }
    let secs0 = f0 as f64 / sr as f64;
    let beat_of = |t: f64| a.beat_at(secs0 + t) - b0;
    let heard = match opts.kind {
        Convert::Melody => melody(&x, sr, pitch_shift(a, b0, b1, project_bpm), progress),
        Convert::Harmony => {
            // The beats cut the chords too (as the clip's own beats).
            let beats: Vec<f64> = (1..).map(|k| b0 + k as f64).take_while(|&b| b < b1).map(|b| a.secs_at(b) - secs0).collect();
            harmony(&x, sr, &beats, pitch_shift(a, b0, b1, project_bpm), progress)
        }
        Convert::Drums => drums(&x, sr, progress),
    };
    let length = b1 - b0;
    let mut notes = Vec::with_capacity(heard.len());
    for h in heard {
        let (mut s, mut e) = (beat_of(h.start), beat_of(h.end));
        if let Some(q) = opts.quantize.filter(|q| *q > 0.0) {
            s = (s / q).round() * q;
            e = ((e / q).round() * q).max(s + q);
        }
        if s < -1e-9 || s >= length - 1e-9 {
            continue;
        }
        let s = s.max(0.0);
        let e = e.min(length);
        if e - s < 1e-3 {
            continue;
        }
        notes.push(Note::new(h.pitch, s, e - s, h.velocity));
    }
    if opts.quantize.is_some() {
        crate::project::tidy_notes(&mut notes);
    }
    notes.sort_by(|a, b| a.start.total_cmp(&b.start).then(a.pitch.cmp(&b.pitch)));
    progress(1.0);
    if notes.is_empty() {
        return Err(format!("No {} heard in the clip", match opts.kind {
            Convert::Melody => "melody",
            Convert::Harmony => "chords",
            Convert::Drums => "drums",
        }));
    }
    Ok((notes, length))
}

/// What converting made.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Converted {
    /// The new track.
    pub track: usize,
    pub notes: usize,
}

impl Project {
    /// The audio clip of `track` at `clip`, as it is now (for hearing it
    /// off this thread): the clip and where its MIDI clip would go.
    pub fn convert_source(&self, track: usize, clip: SliceClip) -> Result<(Clip, SlicePlace), String> {
        let t = self.tracks.get(track).ok_or("There is no such track")?;
        let (source, place) = match clip {
            SliceClip::Slot(s) => (t.slots.get(s).and_then(|c| c.clone()), SlicePlace::Slot(s)),
            SliceClip::Arrangement(id) => match t.arrangement.iter().find(|a| a.id == id) {
                Some(ac) => (Some(ac.clip.clone()), SlicePlace::Arrangement { start: ac.start, length: ac.length, offset: ac.offset }),
                None => (None, SlicePlace::Slot(0)),
            },
        };
        let source = source.ok_or("There is no clip there")?;
        if source.is_midi() {
            return Err("Only audio clips can be converted to MIDI".into());
        }
        Ok((source, place))
    }

    /// Convert to New MIDI Track: `notes` (what [`convert`] heard in
    /// `source`, a clip `length` beats long) on a new MIDI track at `at`
    /// with the instrument for `kind`, in a clip at `place` (the source's
    /// scene, or its place on the arrangement).
    pub fn add_converted_track(&mut self, at: usize, source: &Clip, place: SlicePlace, kind: Convert, notes: Vec<Note>, length: f64) -> Converted {
        let name = format!("{} {}", source.name, kind.name());
        let t = self.insert_midi_track(at, name, Some(kind.instrument()));
        self.tracks[t].color = source.color;
        let count = notes.len();
        let mut clip = Clip::new_midi(self.alloc_id(), source.name.clone(), source.color, length.max(1.0 / 16.0));
        clip.content = ClipContent::Midi { notes };
        match place {
            SlicePlace::Slot(s) => {
                while s >= self.scenes.len() {
                    let n = self.scenes.len();
                    self.add_scene(n);
                }
                self.tracks[t].slots[s] = Some(clip);
            }
            SlicePlace::Arrangement { start, length, offset } => {
                let id = self.alloc_id();
                self.tracks[t].arrangement.push(ArrangementClip { offset, ..ArrangementClip::new(id, start, length, clip) });
            }
        }
        Converted { track: t, notes: count }
    }

    /// Both at once (here, on this thread): the clip of `track` at `clip`
    /// heard as `opts` says, onto a new track right after it.
    pub fn convert_to_midi_track(&mut self, track: usize, clip: SliceClip, opts: &ConvertOptions) -> Result<Converted, String> {
        let (source, place) = self.convert_source(track, clip)?;
        let ClipContent::Audio(a) = &source.content else { return Err("Only audio clips can be converted to MIDI".into()) };
        let (notes, length) = convert(a, source.length, opts, self.bpm, &mut |_| {})?;
        Ok(self.add_converted_track(track + 1, &source, place, opts.kind, notes, length))
    }
}

// ---------------------------------------------------------------------------
// Signal helpers

/// A complex FFT of `n` points (a power of two), in doubles.
struct Cfft {
    n: usize,
    rev: Vec<usize>,
    cos: Vec<f64>,
    sin: Vec<f64>,
}

impl Cfft {
    fn new(n: usize) -> Self {
        assert!(n.is_power_of_two() && n >= 4);
        let bits = n.trailing_zeros();
        let rev = (0..n).map(|i| i.reverse_bits() >> (usize::BITS - bits)).collect();
        let tau = std::f64::consts::TAU;
        let cos = (0..n / 2).map(|k| (tau * k as f64 / n as f64).cos()).collect();
        let sin = (0..n / 2).map(|k| -(tau * k as f64 / n as f64).sin()).collect();
        Self { n, rev, cos, sin }
    }

    /// Forward, in place.
    fn run(&self, re: &mut [f64], im: &mut [f64]) {
        let n = self.n;
        for i in 0..n {
            let j = self.rev[i];
            if i < j {
                re.swap(i, j);
                im.swap(i, j);
            }
        }
        let mut size = 2;
        while size <= n {
            let half = size / 2;
            let step = n / size;
            for start in (0..n).step_by(size) {
                for k in 0..half {
                    let (wr, wi) = (self.cos[k * step], self.sin[k * step]);
                    let (a, b) = (start + k, start + k + half);
                    let tr = re[b] * wr - im[b] * wi;
                    let ti = re[b] * wi + im[b] * wr;
                    re[b] = re[a] - tr;
                    im[b] = im[a] - ti;
                    re[a] += tr;
                    im[a] += ti;
                }
            }
            size *= 2;
        }
    }
}

/// `x` at about `target` Hz or a little more (every so many samples, after
/// a low-pass), and its rate.
fn decimate(x: &[f32], sr: u32, target: f64) -> (Vec<f32>, f64) {
    let factor = ((sr as f64 / target).floor() as usize).max(1);
    if factor == 1 {
        return (x.to_vec(), sr as f64);
    }
    // A windowed sinc a little under the new Nyquist.
    let fc = 0.45 / factor as f64;
    let m = 8 * factor;
    let taps: Vec<f64> = (0..=2 * m)
        .map(|k| {
            let t = k as f64 - m as f64;
            let sinc = if t == 0.0 { 2.0 * fc } else { (std::f64::consts::TAU * fc * t).sin() / (std::f64::consts::PI * t) };
            let w = 0.42 - 0.5 * (std::f64::consts::TAU * k as f64 / (2 * m) as f64).cos() + 0.08 * (2.0 * std::f64::consts::TAU * k as f64 / (2 * m) as f64).cos();
            sinc * w
        })
        .collect();
    let sum: f64 = taps.iter().sum();
    let out = (0..x.len() / factor)
        .map(|i| {
            let c = (i * factor) as isize;
            let mut acc = 0.0f64;
            for (k, h) in taps.iter().enumerate() {
                let j = c + k as isize - m as isize;
                if j >= 0 && (j as usize) < x.len() {
                    acc += h * x[j as usize] as f64;
                }
            }
            (acc / sum) as f32
        })
        .collect();
    (out, sr as f64 / factor as f64)
}

fn db(x: f64) -> f64 {
    10.0 * x.max(1e-20).log10()
}

fn midi_of(hz: f64) -> f64 {
    69.0 + 12.0 * (hz / 440.0).log2()
}

fn hz_of(midi: f64) -> f64 {
    440.0 * 2f64.powf((midi - 69.0) / 12.0)
}

/// The hits of `x` (frames, with how strong each is), as slicing finds them.
fn hits_of(x: &[f32], sr: u32) -> (SampleData, Vec<analysis::Hit>) {
    let s = SampleData::from_buffers("heard", None, sr, x.to_vec(), x.to_vec());
    let hits = analysis::spectral_flux(&s).hits(&s, 0.5);
    (s, hits)
}

// ---------------------------------------------------------------------------
// Melody

/// The pitch of one frame: what the path through the candidates chose.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PitchFrame {
    /// Seconds from the start.
    pub time: f64,
    /// Hz, or None where nothing sounds with a pitch.
    pub hz: Option<f64>,
    /// How loud, in dB (of full scale).
    pub level: f64,
}

/// The lowest and highest pitches a melody can have (Hz).
const MIN_HZ: f64 = 50.0;
const MAX_HZ: f64 = 2100.0;

/// The Beta(2, 18) spread of YIN's threshold: how much of it lies under `x`.
fn threshold_cdf(x: f64) -> f64 {
    let b = 18.0;
    let x = x.clamp(0.0, 1.0);
    1.0 - (1.0 - x).powf(b) * (1.0 + b * x)
}

/// The pitch of `x` every 5 ms (see the module's notes).
pub fn pitch_track(x: &[f32], sr: u32) -> Vec<PitchFrame> {
    let (y, fs) = decimate(x, sr, 20_000.0);
    let tau_min = ((fs / MAX_HZ).floor() as usize).max(2);
    let tau_max = (fs / MIN_HZ).ceil() as usize;
    let w = (fs * 0.025).round() as usize;
    let n = (w + tau_max + 1).next_power_of_two();
    let hop = ((fs * 0.005).round() as usize).max(1);
    let fft = Cfft::new(n);
    let count = y.len() / hop + 1;
    let seg_len = w + tau_max;
    let (mut re, mut im) = (vec![0.0f64; n], vec![0.0f64; n]);
    let (mut pr, mut pi) = (vec![0.0f64; n], vec![0.0f64; n]);
    let mut seg = vec![0.0f64; seg_len];
    let mut sq = vec![0.0f64; seg_len + 1];
    let mut d = vec![0.0f64; tau_max + 2];
    // Each frame's candidates: (Hz, how likely), and how loud it is.
    let mut cands: Vec<Vec<(f64, f64)>> = Vec::with_capacity(count);
    let mut levels = Vec::with_capacity(count);
    for t in 0..count {
        let c = (t * hop) as isize;
        let from = c - (seg_len / 2) as isize;
        for (j, v) in seg.iter_mut().enumerate() {
            let i = from + j as isize;
            *v = if i >= 0 && (i as usize) < y.len() { y[i as usize] as f64 } else { 0.0 };
        }
        sq[0] = 0.0;
        for j in 0..seg_len {
            sq[j + 1] = sq[j] + seg[j] * seg[j];
        }
        let e0 = sq[w];
        // The level: of the window in the middle of the frame.
        let mid = (seg_len - w) / 2;
        levels.push(db((sq[mid + w] - sq[mid]) / w as f64));
        if e0 < 1e-10 * w as f64 {
            cands.push(Vec::new());
            continue;
        }
        // The cross-correlation of the window with the frame, through one
        // FFT of both (the window real, the frame imaginary).
        for j in 0..n {
            re[j] = if j < w { seg[j] } else { 0.0 };
            im[j] = if j < seg_len { seg[j] } else { 0.0 };
        }
        fft.run(&mut re, &mut im);
        for k in 0..n {
            let m = (n - k) % n;
            let (zr, zi) = (re[k], im[k]);
            let (wr, wi) = (re[m], -im[m]);
            let (ar, ai) = (0.5 * (zr + wr), 0.5 * (zi + wi));
            let (br, bi) = (0.5 * (zi - wi), -0.5 * (zr - wr));
            // conj(A) * B, conjugated again for the inverse.
            pr[k] = ar * br + ai * bi;
            pi[k] = -(ar * bi - ai * br);
        }
        fft.run(&mut pr, &mut pi);
        // The difference function and its cumulative mean normalised form.
        d[0] = 1.0;
        let mut sum = 0.0f64;
        for tau in 1..=tau_max {
            let r = pr[tau] / n as f64;
            let et = sq[tau + w] - sq[tau];
            let diff = (e0 + et - 2.0 * r).max(0.0);
            sum += diff;
            d[tau] = if sum > 0.0 { diff * tau as f64 / sum } else { 1.0 };
        }
        // The dips, in order, each as likely as the thresholds that pick it.
        let mut list = Vec::new();
        let mut lowest_before = 1.0f64;
        for tau in tau_min.max(2)..tau_max {
            if d[tau] < d[tau - 1] && d[tau] <= d[tau + 1] {
                let v = d[tau];
                if v < lowest_before {
                    let p = threshold_cdf(lowest_before) - threshold_cdf(v);
                    if p > 1e-4 {
                        // Between samples, by the parabola through it.
                        let (a, b, cc) = (d[tau - 1], v, d[tau + 1]);
                        let den = a - 2.0 * b + cc;
                        let off = if den > 1e-12 { (0.5 * (a - cc) / den).clamp(-0.5, 0.5) } else { 0.0 };
                        list.push((fs / (tau as f64 + off), p));
                    }
                    lowest_before = v;
                }
            }
        }
        list.sort_by(|a, b| b.1.total_cmp(&a.1));
        list.truncate(6);
        cands.push(list);
    }
    // Quiet frames have no pitch.
    let loudest = levels.iter().copied().fold(f64::MIN, f64::max);
    for (c, l) in cands.iter_mut().zip(&levels) {
        if *l < loudest - 45.0 || *l < -75.0 {
            c.clear();
        }
    }
    let path = viterbi(&cands);
    (0..count).map(|t| PitchFrame { time: (t * hop) as f64 / fs, hz: path[t], level: levels[t] }).collect()
}

/// The likeliest path through each frame's candidates or silence: a step
/// costs a little per semitone, starting or stopping a little more.
fn viterbi(cands: &[Vec<(f64, f64)>]) -> Vec<Option<f64>> {
    const PER_SEMITONE: f64 = 0.4;
    const SWITCH: f64 = 1.5;
    const EPS: f64 = 1e-3;
    let t_len = cands.len();
    if t_len == 0 {
        return Vec::new();
    }
    // State 0 is silence; 1.. the frame's candidates.
    let emit = |c: &[(f64, f64)]| -> Vec<f64> {
        let voiced: f64 = c.iter().map(|x| x.1).sum::<f64>().min(1.0);
        let mut v = vec![-(1.0 - voiced + EPS).ln()];
        v.extend(c.iter().map(|x| -(x.1 + EPS).ln()));
        v
    };
    let mut cost = emit(&cands[0]);
    let mut back: Vec<Vec<usize>> = Vec::with_capacity(t_len);
    back.push(vec![0; cost.len()]);
    for t in 1..t_len {
        let e = emit(&cands[t]);
        let mut next = vec![f64::MAX; e.len()];
        let mut from = vec![0usize; e.len()];
        for (j, ej) in e.iter().enumerate() {
            for (i, ci) in cost.iter().enumerate() {
                let step = match (i, j) {
                    (0, 0) => 0.0,
                    (0, _) | (_, 0) => SWITCH,
                    _ => PER_SEMITONE * (midi_of(cands[t - 1][i - 1].0) - midi_of(cands[t][j - 1].0)).abs(),
                };
                let c = ci + step;
                if c < next[j] {
                    next[j] = c;
                    from[j] = i;
                }
            }
            next[j] += ej;
        }
        cost = next;
        back.push(from);
    }
    let mut s = (0..cost.len()).min_by(|&a, &b| cost[a].total_cmp(&cost[b])).unwrap_or(0);
    let mut out = vec![None; t_len];
    for t in (0..t_len).rev() {
        out[t] = (s > 0).then(|| cands[t][s - 1].0);
        s = back[t][s];
    }
    out
}

/// The shortest note a melody keeps (seconds).
const MIN_NOTE: f64 = 0.06;

/// The notes of the melody in `x` (see the module's notes), `shift`
/// semitones away from as it sounds there.
pub fn melody(x: &[f32], sr: u32, shift: f64, progress: &mut dyn FnMut(f32)) -> Vec<Heard> {
    let frames = pitch_track(x, sr);
    progress(0.6);
    if frames.len() < 3 {
        return Vec::new();
    }
    let hop = frames[1].time - frames[0].time;
    // Semitones, smoothed a little (a median of five: one wrong frame goes).
    let raw: Vec<Option<f64>> = frames.iter().map(|f| f.hz.map(midi_of)).collect();
    let pitch: Vec<Option<f64>> = (0..raw.len())
        .map(|t| {
            raw[t]?;
            let mut v: Vec<f64> = raw[t.saturating_sub(2)..(t + 3).min(raw.len())].iter().flatten().copied().collect();
            v.sort_by(f64::total_cmp);
            Some(v[v.len() / 2])
        })
        .collect();
    // Where the sound strikes again (louder by a few dB): a new note there.
    let (s, hits) = hits_of(x, sr);
    let strikes: Vec<usize> = hits.iter().filter(|h| analysis::level_jump(&s, h.frame) > 0.1).map(|h| ((h.frame as f64 / sr as f64) / hop).round() as usize).collect();
    progress(0.8);
    let settle = ((0.04 / hop).round() as usize).max(2);
    let mut notes: Vec<(usize, usize)> = Vec::new();
    let mut t = 0;
    while t < pitch.len() {
        if pitch[t].is_none() {
            t += 1;
            continue;
        }
        // A run of frames with a pitch, cut into notes.
        let start = t;
        while t < pitch.len() && pitch[t].is_some() {
            t += 1;
        }
        let run: Vec<f64> = pitch[start..t].iter().flatten().copied().collect();
        let mut cuts = vec![0usize];
        let mut centre = median(&run[..settle.min(run.len())]);
        let mut i = 1;
        while i < run.len() {
            let away = (run[i] - centre.round()).abs() > 0.75;
            // A strike well into the note.
            let struck = strikes.iter().any(|&f| f == start + i) && i - cuts.last().copied().unwrap_or(0) > settle * 2;
            if away && i + settle <= run.len() {
                let next = &run[i..i + settle];
                let (lo, hi) = next.iter().fold((f64::MAX, f64::MIN), |(a, b), &v| (a.min(v), b.max(v)));
                let there = median(next);
                if hi - lo < 0.8 && (there - centre.round()).abs() > 0.75 {
                    // Settled on another note: the cut where it passed halfway.
                    let half = 0.5 * (centre + there);
                    let up = there > centre;
                    let last = cuts.last().copied().unwrap_or(0);
                    let mut c = i;
                    while c > last + 1 && (if up { run[c - 1] > half } else { run[c - 1] < half }) {
                        c -= 1;
                    }
                    cuts.push(c);
                    centre = there;
                    i += settle;
                    continue;
                }
            }
            if struck && !away {
                cuts.push(i);
            }
            i += 1;
        }
        cuts.push(run.len());
        for w in cuts.windows(2) {
            if w[1] > w[0] {
                notes.push((start + w[0], start + w[1]));
            }
        }
    }
    let loudest = frames.iter().map(|f| f.level).fold(f64::MIN, f64::max);
    let mut out: Vec<Heard> = Vec::new();
    for (a, b) in notes {
        if (b - a) as f64 * hop < MIN_NOTE {
            continue;
        }
        // The pitch of its middle (the ends may glide).
        let trim = (b - a) / 5;
        let mid: Vec<f64> = pitch[a + trim..b - trim].iter().flatten().copied().collect();
        let p = (median(&mid) + shift).round();
        if !(0.0..=127.0).contains(&p) {
            continue;
        }
        let level = frames[a..b].iter().map(|f| f.level).fold(f64::MIN, f64::max) - loudest;
        out.push(Heard { pitch: p as u8, start: frames[a].time, end: frames[b - 1].time + hop, velocity: velocity_of(level) });
    }
    out
}

/// A velocity for a level `db` below the loudest of the clip.
fn velocity_of(db: f64) -> f32 {
    (0.3 + 0.7 * (1.0 + db / 36.0)).clamp(0.2, 1.0) as f32
}

fn median(v: &[f64]) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    let mut s = v.to_vec();
    s.sort_by(f64::total_cmp);
    s[s.len() / 2]
}

// ---------------------------------------------------------------------------
// Harmony

/// The lowest and highest notes a chord is heard with.
const LOW_NOTE: u8 = 28;
const HIGH_NOTE: u8 = 96;
/// Harmonics a note's loudness is summed over.
const HARMONICS: usize = 10;
/// At most this many notes in a chord.
const MAX_CHORD: usize = 5;

/// A peak of a spectrum: Hz, and how loud (compressed).
#[derive(Clone, Copy, Debug)]
struct Peak {
    hz: f64,
    amp: f64,
}

/// The peaks of the average spectrum of `x[a..b]` (at `fs`).
fn peaks(x: &[f32], fs: f64, a: usize, b: usize, ffts: &mut [(usize, Fft)]) -> Vec<Peak> {
    let len = b.saturating_sub(a);
    // The longest window that fits (for the lows), 2048 at least.
    let size = ffts.iter().map(|f| f.0).filter(|&n| n <= len.max(2048)).max().unwrap_or(2048);
    let Some((_, fft)) = ffts.iter_mut().find(|f| f.0 == size) else { return Vec::new() };
    let mut starts = Vec::new();
    let mut s = a;
    while s + size <= b {
        starts.push(s);
        s += size / 2;
    }
    if starts.is_empty() {
        starts.push(if a + size <= x.len() { a } else { x.len().saturating_sub(size) });
    }
    let mut avg = vec![0.0f64; size / 2];
    let mut mags = Vec::new();
    let mut buf = vec![0.0f32; size];
    for &s in &starts {
        for (j, v) in buf.iter_mut().enumerate() {
            *v = x.get(s + j).copied().unwrap_or(0.0);
        }
        fft.magnitudes(&buf, &mut mags);
        for (o, m) in avg.iter_mut().zip(&mags) {
            *o += *m as f64 / starts.len() as f64;
        }
    }
    let most = avg.iter().copied().fold(0.0f64, f64::max);
    if most <= 1e-9 {
        return Vec::new();
    }
    let bin = fs / size as f64;
    let mut out = Vec::new();
    for k in 2..avg.len() - 1 {
        let m = avg[k];
        if m >= avg[k - 1] && m > avg[k + 1] && m > most * 0.003 {
            // Between bins, by the parabola through the log levels.
            let (l, c, r) = (avg[k - 1].max(1e-12).ln(), m.ln(), avg[k + 1].max(1e-12).ln());
            let den = l - 2.0 * c + r;
            let off = if den < -1e-12 { (0.5 * (l - r) / den).clamp(-0.5, 0.5) } else { 0.0 };
            let peak = (c - 0.25 * (l - r) * off).exp();
            out.push(Peak { hz: (k as f64 + off) * bin, amp: (peak / most).powf(0.6) });
        }
    }
    out
}

/// The notes of a chord in `peaks` (MIDI, with how strongly each is
/// heard), the strongest first.
fn chord_notes(peaks: &[Peak], fs: f64) -> Vec<(u8, f64)> {
    if peaks.is_empty() {
        return Vec::new();
    }
    let mut left: Vec<f64> = peaks.iter().map(|p| p.amp).collect();
    let top = (fs * 0.45).min(6000.0);
    // The peak nearest `hz` within a quarter tone.
    let near = |hz: f64| -> Option<usize> {
        let i = peaks.partition_point(|p| p.hz < hz);
        [i.wrapping_sub(1), i].iter().filter_map(|&j| peaks.get(j).map(|p| (j, (p.hz - hz).abs()))).filter(|(_, dd)| *dd < hz * 0.029).min_by(|a, b| a.1.total_cmp(&b.1)).map(|x| x.0)
    };
    let weight = |h: usize| 0.85f64.powi(h as i32 - 1);
    let mut picked: Vec<(u8, f64)> = Vec::new();
    let mut first = 0.0f64;
    while picked.len() < MAX_CHORD + 1 {
        let mut best: Option<(u8, f64, Vec<Option<usize>>)> = None;
        for p in LOW_NOTE..=HIGH_NOTE {
            if picked.iter().any(|(q, _)| q.abs_diff(p) <= 1) {
                continue;
            }
            let f0 = hz_of(p as f64);
            let parts: Vec<Option<usize>> = (1..=HARMONICS).map(|h| if f0 * h as f64 > top { None } else { near(f0 * h as f64) }).collect();
            // A note sounds its own pitch.
            let Some(Some(fund)) = parts.first() else { continue };
            if left[*fund] < 0.05 {
                continue;
            }
            let s: f64 = parts.iter().enumerate().filter_map(|(h, j)| j.map(|j| weight(h + 1) * left[j])).sum();
            if best.as_ref().is_none_or(|b| s > b.1) {
                best = Some((p, s, parts));
            }
        }
        let Some((p, s, parts)) = best else { break };
        if picked.is_empty() {
            first = s;
        } else if s < first * 0.3 {
            break;
        }
        picked.push((p, s));
        // Its partials out, as much as a smooth spectrum of it would have
        // (a partial it shares with another note keeps the rest).
        let amps: Vec<f64> = parts.iter().map(|j| j.map_or(0.0, |j| left[j])).collect();
        for (h, j) in parts.iter().enumerate() {
            let Some(j) = *j else { continue };
            let lo = if h > 0 { amps[h - 1] } else { amps[h] };
            let hi = amps.get(h + 1).copied().unwrap_or(amps[h]);
            let smooth = amps[h].min((lo + amps[h] + hi) / 3.0);
            left[j] = (left[j] - smooth).max(0.0);
        }
    }
    picked
}

/// The chord shapes a chord is heard as (semitones above its root).
const SHAPES: [&[u8]; 10] = [&[0, 4, 7], &[0, 3, 7], &[0, 3, 6], &[0, 4, 8], &[0, 2, 7], &[0, 5, 7], &[0, 4, 7, 10], &[0, 4, 7, 11], &[0, 3, 7, 10], &[0, 3, 6, 10]];

/// The notes of `picked` that make the chord: the bass, and the others
/// whose notes are of the chord shape that fits them best (a stray note
/// goes), the strongest few.
fn as_chord(picked: &[(u8, f64)]) -> Vec<u8> {
    if picked.is_empty() {
        return Vec::new();
    }
    let mut chroma = [0.0f64; 12];
    for &(p, s) in picked {
        chroma[p as usize % 12] += s;
    }
    let total: f64 = chroma.iter().sum();
    let mut best: Option<(f64, usize, &[u8])> = None;
    for root in 0..12 {
        for shape in SHAPES {
            let has = |pc: usize| shape.iter().any(|&i| (root + i as usize) % 12 == pc);
            let inside: f64 = (0..12).filter(|&pc| has(pc)).map(|pc| chroma[pc]).sum();
            let missing = shape.iter().filter(|&&i| chroma[(root + i as usize) % 12] <= 0.0).count();
            let score = inside - (total - inside) - 0.2 * total * missing as f64 - 0.02 * total * shape.len() as f64;
            if best.is_none_or(|b| score > b.0) {
                best = Some((score, root, shape));
            }
        }
    }
    let bass = picked.iter().map(|p| p.0).min().unwrap_or(0);
    let mut keep: Vec<(u8, f64)> = match best {
        // A shape that covers at least three of the notes heard sorts out strays.
        Some((_, root, shape)) if picked.iter().filter(|(p, _)| shape.iter().any(|&i| (root + i as usize) % 12 == *p as usize % 12)).count() >= 3 => {
            picked.iter().copied().filter(|(p, _)| *p == bass || shape.iter().any(|&i| (root + i as usize) % 12 == *p as usize % 12)).collect()
        }
        _ => picked.to_vec(),
    };
    // The bass, and the strongest of the rest.
    keep.sort_by(|a, b| (b.0 == bass).cmp(&(a.0 == bass)).then(b.1.total_cmp(&a.1)));
    keep.truncate(MAX_CHORD);
    let mut notes: Vec<u8> = keep.iter().map(|k| k.0).collect();
    notes.sort_unstable();
    notes
}

/// The chords of `x` (see the module's notes), cut at its hits and at
/// `beats` (seconds), `shift` semitones away from as they sound there.
pub fn harmony(x: &[f32], sr: u32, beats: &[f64], shift: f64, progress: &mut dyn FnMut(f32)) -> Vec<Heard> {
    let (y, fs) = decimate(x, sr, 20_000.0);
    let (_, hits) = hits_of(x, sr);
    let dur = x.len() as f64 / sr as f64;
    let mut cuts: Vec<f64> = vec![0.0];
    cuts.extend(hits.iter().map(|h| h.frame as f64 / sr as f64));
    cuts.extend(beats.iter().copied());
    cuts.sort_by(f64::total_cmp);
    let mut bounds: Vec<f64> = Vec::new();
    for c in cuts {
        if c < dur - 0.1 && bounds.last().is_none_or(|&l| c - l >= 0.1) {
            bounds.push(c);
        }
    }
    bounds.push(dur);
    let mut ffts: Vec<(usize, Fft)> = [2048, 4096, 8192].iter().map(|&n| (n, Fft::new(n))).collect();
    let rms = |a: usize, b: usize| -> f64 { db(y[a.min(y.len())..b.min(y.len())].iter().map(|v| (*v as f64).powi(2)).sum::<f64>() / (b.saturating_sub(a)).max(1) as f64) };
    // Each piece: its notes, from the end of its attack.
    let skip = (0.03 * fs) as usize;
    let pieces: Vec<(f64, f64, Vec<u8>, f64)> = bounds
        .windows(2)
        .enumerate()
        .map(|(i, w)| {
            progress(0.1 + 0.8 * i as f32 / bounds.len() as f32);
            let (a, b) = ((w[0] * fs) as usize, (w[1] * fs) as usize);
            let a2 = (a + skip).min(b);
            let pk = peaks(&y, fs, a2, b, &mut ffts);
            let notes = as_chord(&chord_notes(&pk, fs));
            (w[0], w[1], notes, rms(a2, b))
        })
        .collect();
    let loudest = pieces.iter().map(|p| p.3).fold(f64::MIN, f64::max);
    let mut out: Vec<Heard> = Vec::new();
    let mut last: Option<(Vec<u8>, usize)> = None;
    for (a, b, notes, level) in pieces {
        if level < loudest - 40.0 || notes.is_empty() {
            last = None;
            continue;
        }
        let notes: Vec<u8> = notes.iter().filter_map(|&n| u8::try_from((n as f64 + shift).round() as i64).ok()).filter(|&n| n <= 127).collect();
        // The same chord on: longer.
        if let Some((prev, from)) = &last {
            if *prev == notes {
                for h in &mut out[*from..] {
                    h.end = b;
                }
                continue;
            }
        }
        let from = out.len();
        let vel = velocity_of(level - loudest);
        out.extend(notes.iter().map(|&p| Heard { pitch: p, start: a, end: b, velocity: vel }));
        last = Some((notes, from));
    }
    out
}

// ---------------------------------------------------------------------------
// Drums

/// A drum heard at a hit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DrumClass {
    Kick,
    Snare,
    ClosedHat,
    OpenHat,
}

impl DrumClass {
    pub fn note(self) -> u8 {
        match self {
            DrumClass::Kick => KICK,
            DrumClass::Snare => SNARE,
            DrumClass::ClosedHat => CLOSED_HAT,
            DrumClass::OpenHat => OPEN_HAT,
        }
    }
}

/// What a hit adds: in the lows (below 150 Hz), the body (to 600 Hz), the
/// middle (to 4 kHz) and the highs, and how long its highs ring (seconds).
#[derive(Clone, Copy, Debug, Default)]
struct HitSound {
    low: f64,
    body: f64,
    mid: f64,
    high: f64,
    ring: f64,
}

/// A hit's highs ring on this long and more: an open hat.
const OPEN_RING: f64 = 0.09;

/// The drums of `x`: each hit's kick, snare or hat (some at once), on the
/// notes of a General MIDI kit (see the module's notes).
pub fn drums(x: &[f32], sr: u32, progress: &mut dyn FnMut(f32)) -> Vec<Heard> {
    let (_, hits) = hits_of(x, sr);
    progress(0.4);
    if hits.is_empty() {
        return Vec::new();
    }
    let size = ((sr as f64 * 0.043) as usize).next_power_of_two();
    let mut fft = Fft::new(size);
    let bin = sr as f64 / size as f64;
    let band = |hz: f64| ((hz / bin).round() as usize).min(size / 2);
    let (l0, l1, b1, m1) = (band(30.0), band(150.0), band(600.0), band(4000.0));
    let mut mags = Vec::new();
    let mut buf = vec![0.0f32; size];
    let mut energy = |from: isize, fft: &mut Fft| -> [f64; 4] {
        for (j, v) in buf.iter_mut().enumerate() {
            let i = from + j as isize;
            *v = if i >= 0 && (i as usize) < x.len() { x[i as usize] } else { 0.0 };
        }
        fft.magnitudes(&buf, &mut mags);
        let sum = |a: usize, b: usize| mags[a..b.max(a)].iter().map(|m| (*m as f64).powi(2)).sum::<f64>();
        [sum(l0, l1), sum(l1, b1), sum(b1, m1), sum(m1, mags.len())]
    };
    // The highs' ring: how long until their energy (in 5 ms blocks) has
    // fallen 20 dB from its most, before the next hit.
    let block = (sr as usize / 200).max(1);
    let ring = |a: usize, b: usize| -> f64 {
        let hp: Vec<f64> = (a..b.min(x.len())).map(|i| if i > 0 { (x[i] - x[i - 1]) as f64 } else { 0.0 }).collect();
        let blocks: Vec<f64> = hp.chunks(block).map(|c| c.iter().map(|v| v * v).sum::<f64>()).collect();
        let Some((peak_at, &peak)) = blocks.iter().enumerate().max_by(|a, b| a.1.total_cmp(b.1)) else { return 0.0 };
        let gone = blocks[peak_at..].iter().position(|&e| e < peak * 0.01).map_or(blocks.len(), |k| peak_at + k);
        (gone - peak_at) as f64 * block as f64 / sr as f64
    };
    let sounds: Vec<HitSound> = hits
        .iter()
        .enumerate()
        .map(|(k, h)| {
            let f = h.frame as isize;
            // The window after the hit has it a quarter in (where the window is half up).
            let after = energy(f - (size / 4) as isize, &mut fft);
            let before = energy(f - (size + size / 4) as isize, &mut fft);
            let rise = |i: usize| (after[i] - before[i]).max(0.0);
            let next = hits.get(k + 1).map_or(x.len(), |n| n.frame);
            let till = next.min(h.frame + sr as usize / 2);
            HitSound { low: rise(0), body: rise(1), mid: rise(2), high: rise(3), ring: ring(h.frame, till) }
        })
        .collect();
    progress(0.8);
    let max_low = sounds.iter().map(|s| s.low).fold(0.0, f64::max);
    let max_noise = sounds.iter().map(|s| s.mid + s.high).fold(0.0, f64::max);
    let level = |v: f64, most: f64| -> f32 { if most > 0.0 { velocity_of(db(v / most).max(-60.0)) } else { 0.8 } };
    let mut out = Vec::new();
    let sixteenth = 0.1;
    for (h, s) in hits.iter().zip(&sounds) {
        let total = s.low + s.body + s.mid + s.high;
        if total <= 0.0 {
            continue;
        }
        let noise = s.mid + s.high;
        let mut classes: Vec<(DrumClass, f32)> = Vec::new();
        if s.low / total >= 0.35 || (s.low >= 0.2 * max_low && s.low / total >= 0.1) {
            classes.push((DrumClass::Kick, level(s.low, max_low)));
        }
        if noise >= 0.2 * max_noise || noise / total >= 0.3 {
            let hat = if s.ring >= OPEN_RING { DrumClass::OpenHat } else { DrumClass::ClosedHat };
            let class = if s.mid / noise.max(1e-20) >= 0.06 { DrumClass::Snare } else { hat };
            classes.push((class, level(noise, max_noise)));
        }
        if classes.is_empty() {
            // Neither plainly: by where most of it is.
            let class = if s.low + s.body >= noise { if s.low >= s.body { DrumClass::Kick } else { DrumClass::Snare } } else { DrumClass::ClosedHat };
            classes.push((class, 0.5));
        }
        let t = h.frame as f64 / sr as f64;
        for (c, v) in classes {
            out.push(Heard { pitch: c.note(), start: t, end: t + sixteenth, velocity: v });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::tests::drum;
    use crate::project::TrackKind;
    use crate::sample::SampleRef;
    use std::f64::consts::TAU;

    /// A tone of harmonics `amps` (the first the fundamental) following
    /// `hz` (seconds to Hz), `secs` long, faded in and out over 5 ms.
    fn tone(sr: u32, secs: f64, amps: &[f64], hz: impl Fn(f64) -> f64) -> Vec<f32> {
        let n = (secs * sr as f64) as usize;
        let mut phase = 0.0f64;
        let fade = 0.005 * sr as f64;
        (0..n)
            .map(|i| {
                let t = i as f64 / sr as f64;
                phase += TAU * hz(t) / sr as f64;
                let env = (i as f64 / fade).min(1.0).min((n - i) as f64 / fade);
                let v: f64 = amps.iter().enumerate().map(|(h, a)| a * ((h + 1) as f64 * phase).sin()).sum();
                (0.3 * env * v) as f32
            })
            .collect()
    }

    fn notes_at(heard: &[Heard]) -> Vec<(u8, f64, f64)> {
        heard.iter().map(|h| (h.pitch, h.start, h.end)).collect()
    }

    /// Notes (MIDI, seconds) one after the other, each a harmonic tone,
    /// with a little silence between.
    fn tune(sr: u32, notes: &[(u8, f64)], amps: &[f64]) -> (Vec<f32>, Vec<(u8, f64, f64)>) {
        let mut x = Vec::new();
        let mut want = Vec::new();
        for &(p, len) in notes {
            let t0 = x.len() as f64 / sr as f64;
            x.extend(tone(sr, len - 0.03, amps, |_| hz_of(p as f64)));
            x.extend(std::iter::repeat_n(0.0, (0.03 * sr as f64) as usize));
            want.push((p, t0, t0 + len - 0.03));
        }
        (x, want)
    }

    fn check(heard: &[Heard], want: &[(u8, f64, f64)], tol: f64, what: &str) {
        assert_eq!(heard.len(), want.len(), "{what}: heard {:?}", notes_at(heard));
        for (h, w) in heard.iter().zip(want) {
            assert_eq!(h.pitch, w.0, "{what}: heard {:?}", notes_at(heard));
            assert!((h.start - w.1).abs() < tol && (h.end - w.2).abs() < tol, "{what}: {:?} for {w:?}", (h.pitch, h.start, h.end));
        }
    }

    #[test]
    fn a_tune_of_tones_is_heard_note_for_note() {
        let notes = [(60, 0.4), (62, 0.4), (64, 0.2), (67, 0.6), (55, 0.4), (72, 0.3), (40, 0.5), (84, 0.3), (45, 0.4)];
        for sr in [44_100, 48_000] {
            // A pure sine and a bright tone.
            for amps in [&[1.0][..], &[1.0, 0.6, 0.4, 0.3, 0.2, 0.1][..]] {
                let (x, want) = tune(sr, &notes, amps);
                let heard = melody(&x, sr, 0.0, &mut |_| {});
                check(&heard, &want, 0.03, &format!("{sr} Hz, {} harmonics", amps.len()));
            }
        }
    }

    #[test]
    fn a_weak_fundamental_is_still_its_note_not_the_octave() {
        // Most of the sound in the second harmonic, or no fundamental at all.
        for amps in [&[0.25, 1.0, 0.5, 0.3][..], &[0.0, 1.0, 0.8, 0.6, 0.4][..], &[1.0, 0.0, 1.0, 0.0, 0.5][..]] {
            let (x, want) = tune(48_000, &[(57, 0.5), (50, 0.5), (64, 0.5)], amps);
            let heard = melody(&x, 48_000, 0.0, &mut |_| {});
            check(&heard, &want, 0.03, &format!("{amps:?}"));
        }
    }

    #[test]
    fn vibrato_stays_a_note_and_a_glide_goes_halfway() {
        let sr = 48_000;
        // Half a second of A4 with a vibrato of half a semitone at 5.5 Hz.
        let x = tone(sr, 0.8, &[1.0, 0.5, 0.3], |t| hz_of(69.0 + 0.5 * (TAU * 5.5 * t).sin()));
        let heard = melody(&x, sr, 0.0, &mut |_| {});
        check(&heard, &[(69, 0.0, 0.8)], 0.03, "vibrato");
        // C4 for 0.4 s, gliding up to E4 over 0.1 s, held 0.4 s: the cut in the glide's middle.
        let x = tone(sr, 0.9, &[1.0, 0.5], |t| hz_of(if t < 0.4 { 60.0 } else if t < 0.5 { 60.0 + 4.0 * (t - 0.4) / 0.1 } else { 64.0 }));
        let heard = melody(&x, sr, 0.0, &mut |_| {});
        check(&heard, &[(60, 0.0, 0.45), (64, 0.45, 0.9)], 0.04, "glide");
    }

    #[test]
    fn a_struck_note_again_is_two_notes_and_loudness_is_velocity() {
        let sr = 48_000;
        // E4 struck twice without a gap (a pluck dying away), then softer.
        let pluck = |secs: f64, amp: f64| -> Vec<f32> { tone(sr, secs, &[1.0, 0.5, 0.25], |_| hz_of(64.0)).iter().enumerate().map(|(i, v)| (*v as f64 * amp * (-(i as f64) / sr as f64 / 0.15).exp()) as f32).collect() };
        let mut x = pluck(0.4, 1.0);
        x.extend(pluck(0.4, 1.0));
        x.extend(pluck(0.4, 0.2));
        let heard = melody(&x, sr, 0.0, &mut |_| {});
        assert_eq!(heard.iter().map(|h| h.pitch).collect::<Vec<_>>(), vec![64, 64, 64], "{:?}", notes_at(&heard));
        assert!((heard[1].start - 0.4).abs() < 0.03 && (heard[2].start - 0.8).abs() < 0.03, "{:?}", notes_at(&heard));
        assert!(heard[2].velocity < heard[1].velocity - 0.2, "{} {}", heard[1].velocity, heard[2].velocity);
        // Transposed: the notes it plays.
        let up = melody(&x, sr, 2.0, &mut |_| {});
        assert!(up.iter().all(|h| h.pitch == 66));
    }

    /// Chords of harmonic tones (each note's harmonics falling as 1/h),
    /// `secs` each.
    fn chords(sr: u32, chords: &[&[u8]], secs: f64) -> Vec<f32> {
        let mut x = vec![0.0f32; (chords.len() as f64 * secs * sr as f64) as usize];
        let amps: Vec<f64> = (1..=8).map(|h| 1.0 / h as f64).collect();
        for (k, c) in chords.iter().enumerate() {
            let at = (k as f64 * secs * sr as f64) as usize;
            for &p in c.iter() {
                for (i, v) in tone(sr, secs, &amps, |_| hz_of(p as f64)).iter().enumerate() {
                    if let Some(o) = x.get_mut(at + i) {
                        *o += *v * 0.5;
                    }
                }
            }
        }
        x
    }

    #[test]
    fn chords_are_heard_with_their_notes_and_bass() {
        let sr = 48_000;
        let progression: [&[u8]; 6] = [
            &[48, 60, 64, 67],     // C major, C in the bass
            &[45, 57, 60, 64],     // A minor
            &[43, 59, 62, 65],     // G7 (B D F over G)
            &[53, 57, 60, 64],     // F major 7
            &[50, 57, 60, 65],     // D minor 7
            &[40, 55, 59, 62, 64], // E minor 7 (G B D E over E)
        ];
        let x = chords(sr, &progression, 1.0);
        let beats: Vec<f64> = (1..12).map(|k| k as f64 * 0.5).collect();
        let heard = harmony(&x, sr, &beats, 0.0, &mut |_| {});
        for (k, want) in progression.iter().enumerate() {
            let t = k as f64 + 0.5;
            let mut got: Vec<u8> = heard.iter().filter(|h| h.start <= t && h.end > t).map(|h| h.pitch).collect();
            got.sort_unstable();
            assert!((3..=5).contains(&got.len()), "chord {k}: {got:?}");
            let pcs = |v: &[u8]| {
                let mut p: Vec<u8> = v.iter().map(|n| n % 12).collect();
                p.sort_unstable();
                p.dedup();
                p
            };
            assert_eq!(pcs(&got), pcs(want), "chord {k}: heard {got:?}, played {want:?}");
            assert_eq!(got[0], want[0], "chord {k}'s bass: {got:?}");
            // One chord over its two beats.
            let starts: Vec<f64> = heard.iter().filter(|h| h.start <= t && h.end > t).map(|h| h.start).collect();
            assert!(starts.iter().all(|s| (s - k as f64).abs() < 0.05), "chord {k} starts {starts:?}");
        }
    }

    #[test]
    fn triads_and_sevenths_in_the_middle_of_the_keyboard() {
        let sr = 44_100;
        for chord in [&[60u8, 64, 67][..], &[62, 65, 69], &[64, 67, 71, 74], &[57, 61, 64, 67], &[59, 62, 65, 69], &[65, 69, 72, 76]] {
            let x = chords(sr, &[chord], 1.5);
            let heard = harmony(&x, sr, &[], 0.0, &mut |_| {});
            let got: Vec<u8> = heard.iter().map(|h| h.pitch % 12).collect();
            let mut want: Vec<u8> = chord.iter().map(|p| p % 12).collect();
            let mut g = got.clone();
            g.sort_unstable();
            g.dedup();
            want.sort_unstable();
            assert_eq!(g, want, "{chord:?}: heard {:?}", heard.iter().map(|h| h.pitch).collect::<Vec<_>>());
        }
    }

    /// A drum loop at 120 BPM over two bars (a sixteenth is 0.125 s): the
    /// steps of each drum, an open hat ringing long.
    fn beat(sr: u32, kit: &[(char, &[usize])]) -> Vec<f32> {
        let step = sr as usize / 8;
        let mut x = vec![0.0f32; step * 32];
        for (k, &(kind, steps)) in kit.iter().enumerate() {
            for &s in steps {
                let sound: Vec<f32> = if kind == 'o' {
                    // An open hat: bright noise ringing 0.15 s.
                    drum('h', sr, 7 + s as u32).iter().enumerate().map(|(i, v)| v * (i as f32 / sr as f32 / 0.025).exp() * (-(i as f32) / sr as f32 / 0.15).exp()).collect()
                } else {
                    drum(kind, sr, 1 + k as u32 * 977 + s as u32 * 31)
                };
                for (i, v) in sound.iter().enumerate() {
                    if let Some(o) = x.get_mut(s * step + i) {
                        *o += v;
                    }
                }
            }
        }
        x
    }

    #[test]
    fn a_drum_loop_is_heard_as_kicks_snares_and_hats() {
        let kit: &[(char, &[usize])] = &[('k', &[0, 6, 10, 16, 22, 25]), ('s', &[4, 12, 20, 28]), ('h', &[2, 8, 14, 18, 24]), ('o', &[30]), ('q', &[11, 27])];
        for sr in [44_100, 48_000] {
            let x = beat(sr, kit);
            let heard = drums(&x, sr, &mut |_| {});
            let step = 0.125;
            let mut got: Vec<(u8, usize)> = heard.iter().map(|h| (h.pitch, (h.start / step).round() as usize)).collect();
            got.sort_unstable();
            let mut want: Vec<(u8, usize)> = Vec::new();
            for &(kind, steps) in kit {
                let note = match kind {
                    'k' => KICK,
                    's' => SNARE,
                    'o' => OPEN_HAT,
                    _ => CLOSED_HAT,
                };
                want.extend(steps.iter().map(|&s| (note, s)));
            }
            want.sort_unstable();
            assert_eq!(got, want, "{sr} Hz");
            for h in &heard {
                let s = (h.start / step).round();
                assert!((h.start - s * step).abs() < 0.004, "{h:?} on its step");
            }
            // The soft hats softer than the loud ones.
            let vel = |s: usize| heard.iter().find(|h| (h.start / step).round() as usize == s).unwrap().velocity;
            assert!(vel(11) < vel(8) - 0.1, "{} {}", vel(11), vel(8));
        }
    }

    #[test]
    fn kicks_and_hats_at_once_are_both_and_a_bass_is_no_kick() {
        let sr = 48_000;
        let kit: &[(char, &[usize])] = &[('k', &[0, 8, 16, 24]), ('h', &[0, 4, 8, 12, 16, 20, 24, 28])];
        let mut x = beat(sr, kit);
        // A bass held under it.
        for (i, v) in x.iter_mut().enumerate() {
            *v += 0.3 * (TAU * 55.0 * i as f64 / sr as f64).sin() as f32;
        }
        let heard = drums(&x, sr, &mut |_| {});
        let count = |p: u8| heard.iter().filter(|h| h.pitch == p).count();
        assert_eq!((count(KICK), count(CLOSED_HAT), count(SNARE), count(OPEN_HAT)), (4, 8, 0, 0), "{:?}", notes_at(&heard));
    }

    /// An audio track whose clip (scene 1) is `x`, a loop of `beats` at 120 BPM.
    fn project_with(x: Vec<f32>, sr: u32, beats: f64) -> Project {
        let mut p = Project::blank();
        let t = p.add_audio_track();
        let id = p.alloc_id();
        let s = SampleRef::new(SampleData::from_buffers("Tune", None, sr, x.clone(), x));
        let mut c = Clip::new_audio(id, s, [10, 200, 90], 120.0);
        if let ClipContent::Audio(a) = &mut c.content {
            a.bpm = 120.0;
            a.mode = WarpMode::Beats;
            a.markers.clear();
            a.start = 0.0;
            a.loop_start = 0.0;
            a.looping = true;
        }
        c.length = beats;
        p.tracks[t].slots[0] = Some(c);
        p
    }

    #[test]
    fn a_clip_converts_onto_a_new_track_in_its_beats() {
        let sr = 48_000;
        // Quarter notes at 120 BPM (half a second each): C D E G.
        let (x, _) = tune(sr, &[(60, 0.5), (62, 0.5), (64, 0.5), (67, 0.5)], &[1.0, 0.5]);
        let mut p = project_with(x, sr, 4.0);
        let done = p.convert_to_midi_track(0, SliceClip::Slot(0), &ConvertOptions { kind: Convert::Melody, quantize: Some(0.25) }).unwrap();
        assert_eq!((done.track, done.notes), (1, 4));
        let t = &p.tracks[1];
        assert_eq!((t.kind, t.name.as_str()), (TrackKind::Midi, "Tune Melody"));
        assert!(matches!(t.instrument, Some(Instrument::Analog { .. })));
        let clip = t.slots[0].as_ref().unwrap();
        assert_eq!(clip.length, 4.0);
        let notes: Vec<(u8, f64, f64)> = clip.notes().iter().map(|n| (n.pitch, n.start, n.length)).collect();
        assert_eq!(notes, vec![(60, 0.0, 1.0), (62, 1.0, 1.0), (64, 2.0, 1.0), (67, 3.0, 1.0)]);
        // Unquantized, on the beats within a little; a warp marker moves them with it.
        let ClipContent::Audio(a) = &mut p.tracks[0].slots[0].as_mut().unwrap().content else { panic!() };
        a.markers = vec![(0.0, 0.0), (2.0, 1.0), (4.0, 2.0)];
        a.bpm = 60.0;
        let done = p.convert_to_midi_track(0, SliceClip::Slot(0), &ConvertOptions { kind: Convert::Melody, quantize: None }).unwrap();
        let notes = p.tracks[done.track].slots[0].as_ref().unwrap().notes().to_vec();
        for (k, n) in notes.iter().enumerate() {
            assert!((n.start - k as f64).abs() < 0.04, "{notes:?}");
        }
        // Drums: on the 909, the kicks on C1.
        let sr = 44_100;
        let mut p = project_with(beat(sr, &[('k', &[0, 8, 16, 24]), ('s', &[4, 12, 20, 28])]), sr, 8.0);
        let done = p.convert_to_midi_track(0, SliceClip::Slot(0), &ConvertOptions { kind: Convert::Drums, quantize: Some(0.25) }).unwrap();
        let t = &p.tracks[done.track];
        assert!(matches!(&t.instrument, Some(Instrument::Drums { kit, .. }) if kit == "909 Classic"));
        let notes = t.slots[0].as_ref().unwrap().notes();
        let kicks: Vec<f64> = notes.iter().filter(|n| n.pitch == KICK).map(|n| n.start).collect();
        let snares: Vec<f64> = notes.iter().filter(|n| n.pitch == SNARE).map(|n| n.start).collect();
        assert_eq!((kicks, snares), (vec![0.0, 2.0, 4.0, 6.0], vec![1.0, 3.0, 5.0, 7.0]));
        // A MIDI clip is no audio to hear.
        p.tracks[1].slots[0] = Some(Clip::new_midi(99, "x", [0, 0, 0], 4.0));
        assert!(p.convert_to_midi_track(1, SliceClip::Slot(0), &ConvertOptions { kind: Convert::Drums, quantize: None }).is_err());
    }

    #[test]
    fn the_fft_cross_correlation_matches_the_sum() {
        let fft = Cfft::new(16);
        let a: Vec<f64> = (0..6).map(|i| (i as f64 * 0.7).sin()).collect();
        let b: Vec<f64> = (0..12).map(|i| (i as f64 * 0.3).cos()).collect();
        let (mut re, mut im) = (vec![0.0; 16], vec![0.0; 16]);
        re[..6].copy_from_slice(&a);
        im[..12].copy_from_slice(&b);
        fft.run(&mut re, &mut im);
        let (mut pr, mut pi) = (vec![0.0; 16], vec![0.0; 16]);
        for k in 0..16 {
            let m = (16 - k) % 16;
            let (zr, zi, wr, wi) = (re[k], im[k], re[m], -im[m]);
            let (ar, ai) = (0.5 * (zr + wr), 0.5 * (zi + wi));
            let (br, bi) = (0.5 * (zi - wi), -0.5 * (zr - wr));
            pr[k] = ar * br + ai * bi;
            pi[k] = -(ar * bi - ai * br);
        }
        fft.run(&mut pr, &mut pi);
        for tau in 0..6 {
            let want: f64 = (0..6).map(|j| a[j] * b[j + tau]).sum();
            assert!((pr[tau] / 16.0 - want).abs() < 1e-9, "{tau}: {} {want}", pr[tau] / 16.0);
        }
    }
}

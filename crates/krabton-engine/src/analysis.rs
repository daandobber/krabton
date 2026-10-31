//! Listening to samples: where the hits in a sound are (its transients),
//! for slicing a loop.
//!
//! The onset detector is the usual one for drums (after Dixon, "Onset
//! Detection Revisited", 2006, and Böck and Widmer's SuperFlux, 2013): the
//! spectrum in short frames, in bands of a sixth of an octave and on a
//! logarithmic scale of level, so a soft hat counts as much as a loud kick;
//! how much each band rises from a frame to the one half a window before
//! (the spectral flux; a rise against the loudest of the band and its
//! neighbours there, so a sliding pitch or a held bass is no hit); the
//! peaks of that which stand above its running mean by a threshold the
//! sensitivity sets, at least `MIN_GAP` apart. Each hit is then put on the
//! sample where the sound's energy (its highs a little more) jumps the most.
//!
//! Beside it, the tempo and first downbeat of a recording, for warping
//! (Live's auto-warp): a second onset curve with the lows counted twice
//! (kicks and snares make the beat more than hats do), its
//! autocorrelation weighted towards 120 BPM as people tap, then a line
//! through the hits that fall on that beat pins it down to a small fraction
//! of a percent. [`analyze`] finds both once per sample (cached on it).

use crate::dsp::fft::Fft;
use crate::sample::SampleData;

/// Hits closer together than this (seconds) are one hit (a flam, the
/// bursts of a clap).
pub const MIN_GAP: f64 = 0.03;

/// The levels below about -40 dB of the loudest count for little.
const COMPRESS: f32 = 100.0;

/// A hit: the frame of the sample where it starts, and how much it stands
/// out (0..1: the strongest of the sample is 1): how sudden it is and how
/// much louder it makes the sound, half each (so a kick under a bass,
/// which changes few bands, counts as much as it is heard).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hit {
    pub frame: usize,
    pub strength: f32,
}

/// How much new sound each moment of a sample brings (its spectral flux):
/// one value per `hop` frames, for the window centred there, the most 1.
#[derive(Clone, Debug)]
pub struct Flux {
    pub hop: usize,
    /// Frames of each window.
    pub size: usize,
    pub sample_rate: u32,
    pub values: Vec<f32>,
}

/// Where the hits in `sample` start (frames), as [`Flux::hits`] finds them
/// with `sensitivity` (0..1, the 0 to 100 % of the slider: more finds the
/// softer hits too).
pub fn transients(sample: &SampleData, sensitivity: f32) -> Vec<usize> {
    spectral_flux(sample).hits(sample, sensitivity).iter().map(|h| h.frame).collect()
}

/// What [`analyze`] found in a sample.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Analysis {
    /// Where the hits start, in frames, in order.
    pub transients: Vec<usize>,
    /// How strong each hit is (0..1), beside `transients`.
    pub strengths: Vec<f32>,
    /// Its tempo, when it has a steady beat.
    pub tempo: Option<Tempo>,
}

/// A steady beat.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tempo {
    pub bpm: f64,
    /// The first beat, in seconds.
    pub beat: f64,
    /// The first downbeat (a bar's one, in 4/4), in seconds.
    pub downbeat: f64,
    /// How clear the beat is, 0..1.
    pub confidence: f32,
}

/// The onset curve: a value per frame of `hop` samples.
pub struct Onsets {
    /// Spectral flux over the whole spectrum, its loudest frame at 1.
    pub flux: Vec<f32>,
    /// The same below 200 Hz (kicks), its loudest frame at 1.
    pub low: Vec<f32>,
    pub hop: usize,
    pub sample_rate: f64,
}

impl Onsets {
    /// Frames per second.
    pub fn rate(&self) -> f64 {
        self.sample_rate / self.hop as f64
    }

    /// The flux at `t` seconds, between frames.
    fn at(curve: &[f32], rate: f64, t: f64) -> f32 {
        let x = t * rate;
        if x < 0.0 {
            return 0.0;
        }
        let i = x as usize;
        let f = (x - i as f64) as f32;
        match (curve.get(i), curve.get(i + 1)) {
            (Some(a), Some(b)) => a + (b - a) * f,
            (Some(a), None) => *a,
            _ => 0.0,
        }
    }
}

/// How readily warping finds hits (the slicer's slider at 50 %).
pub const WARP_SENSITIVITY: f32 = 0.5;

/// The transients (as [`transients`] finds them at `WARP_SENSITIVITY`) and
/// the tempo of a sample. [`SampleData::analysis`] keeps it once made.
pub fn analyze(s: &SampleData) -> Analysis {
    let hits = spectral_flux(s).hits(s, WARP_SENSITIVITY);
    let transients: Vec<usize> = hits.iter().map(|h| h.frame).collect();
    let strengths: Vec<f32> = hits.iter().map(|h| h.strength).collect();
    let tempo = tempo_from(s, &onsets(s), &transients, &strengths);
    Analysis { transients, strengths, tempo }
}

/// The tempo and first downbeat of a sample, if it has a steady beat.
pub fn detect_tempo(s: &SampleData) -> Option<Tempo> {
    s.analysis().tempo
}

fn mono(s: &SampleData, i: usize) -> f32 {
    match (s.left.get(i), s.right.get(i)) {
        (Some(l), Some(r)) => (l + r) * 0.5,
        _ => 0.0,
    }
}

/// The beat curve of a sample (for its tempo): frames of about 23 ms, every
/// 11.6 ms, the flux of the whole spectrum in dB and of its lows apart.
pub fn onsets(s: &SampleData) -> Onsets {
    let sr = s.sample_rate as f64;
    let hop = if sr > 50_000.0 { 1024 } else { 512 };
    let n = hop * 2;
    let frames = s.frames().div_ceil(hop) + 1;
    let mut fft = Fft::new(n);
    let top = ((16_000.0 * n as f64 / sr) as usize).clamp(8, n / 2);
    let low_top = ((200.0 * n as f64 / sr) as usize).clamp(2, top);
    let mut buf = vec![0.0f32; n];
    let mut spec = Vec::with_capacity(n / 2);
    let mut last = vec![-70.0f32; n / 2];
    let mut flux = Vec::with_capacity(frames);
    let mut low = Vec::with_capacity(frames);
    for t in 0..frames {
        // Frame t is centred on sample t * hop.
        let from = (t * hop) as isize - hop as isize;
        for (j, b) in buf.iter_mut().enumerate() {
            let i = from + j as isize;
            *b = if i < 0 { 0.0 } else { mono(s, i as usize) };
        }
        fft.spectrum_db(&buf, &mut spec);
        let (mut f, mut fl) = (0.0f32, 0.0f32);
        for k in 1..top {
            let v = spec[k].max(-70.0);
            let d = (v - last[k]).max(0.0);
            f += d;
            if k < low_top {
                fl += d;
            }
            last[k] = v;
        }
        flux.push(f / top as f32);
        low.push(fl / low_top as f32);
    }
    for curve in [&mut flux, &mut low] {
        let m = curve.iter().fold(0.0f32, |m, &x| m.max(x));
        if m > 1e-9 {
            for x in curve.iter_mut() {
                *x /= m;
            }
        }
    }
    Onsets { flux, low, hop, sample_rate: sr }
}

/// The mean of `x` over `w` frames each side of every frame.
fn local_mean(x: &[f32], w: usize) -> Vec<f32> {
    let mut sum = Vec::with_capacity(x.len() + 1);
    sum.push(0.0f64);
    for &v in x {
        sum.push(sum.last().unwrap_or(&0.0) + v as f64);
    }
    (0..x.len())
        .map(|t| {
            let (lo, hi) = (t.saturating_sub(w), (t + w + 1).min(x.len()));
            ((sum[hi] - sum[lo]) / (hi - lo) as f64) as f32
        })
        .collect()
}

/// The tempo: the autocorrelation of the onset curve (weighted towards 120
/// BPM, as people tap), then a fit of the transients to that beat.
fn tempo_from(s: &SampleData, on: &Onsets, transients: &[usize], strengths: &[f32]) -> Option<Tempo> {
    let rate = on.rate();
    let n = on.flux.len();
    if transients.len() < 6 || n < (rate * 2.0) as usize {
        return None;
    }
    // The curve (with the lows counted twice: kicks and snares make the
    // beat more than hats do) less its local mean, so only the hits count.
    let both: Vec<f32> = on.flux.iter().zip(&on.low).map(|(f, l)| f + l).collect();
    let mean = local_mean(&both, 8);
    let peaks: Vec<f32> = both.iter().zip(&mean).map(|(f, m)| (f - m).max(0.0)).collect();
    // Smoothed a little, so beats that fall between frames still line up.
    let e: Vec<f32> = (0..n)
        .map(|t| {
            let w = [1.0f32, 2.0, 3.0, 2.0, 1.0];
            (0..5).map(|j| peaks.get((t + j).wrapping_sub(2)).map_or(0.0, |v| v * w[j])).sum::<f32>() / 9.0
        })
        .collect();
    let ac = |lag: f64| -> f64 {
        let l = lag as usize;
        let fr = lag - l as f64;
        let mut acc = 0.0f64;
        for t in 0..n.saturating_sub(l + 1) {
            let b = e[t + l] as f64 * (1.0 - fr) + e[t + l + 1] as f64 * fr;
            acc += e[t] as f64 * b;
        }
        acc / n.saturating_sub(l).max(1) as f64
    };
    let lag_of = |bpm: f64| rate * 60.0 / bpm;
    let (lo_lag, hi_lag) = (lag_of(200.0).floor() as usize, lag_of(60.0).ceil() as usize);
    let raw: Vec<f64> = (0..=hi_lag * 4 + 2).map(|l| if l >= lo_lag / 2 { ac(l as f64) } else { 0.0 }).collect();
    let acl = |lag: f64| -> f64 {
        let l = lag as usize;
        let fr = lag - l as f64;
        match (raw.get(l), raw.get(l + 1)) {
            (Some(a), Some(b)) => a * (1.0 - fr) + b * fr,
            _ => 0.0,
        }
    };
    let mut scores = Vec::new();
    let mut best = (0.0f64, f64::MIN);
    for l in lo_lag.max(2)..=hi_lag {
        let lag = l as f64;
        let bpm = rate * 60.0 / lag;
        let prior = (-0.5 * ((bpm / 120.0).log2() / 0.8).powi(2)).exp();
        let comb = acl(lag) + 0.5 * acl(2.0 * lag) + 0.25 * acl(3.0 * lag) + 0.25 * acl(4.0 * lag);
        let sc = comb * prior;
        scores.push(sc);
        if sc > best.1 {
            best = (lag, sc);
        }
    }
    if best.1 <= 0.0 {
        return None;
    }
    // Between lags, by the parabola through the best and its neighbours.
    let l = best.0;
    let (a, b, c) = (acl(l - 1.0), acl(l), acl(l + 1.0));
    let d = a - 2.0 * b + c;
    let lag = if d < 0.0 { l + (0.5 * (a - c) / d).clamp(-0.5, 0.5) } else { l };
    let mean = scores.iter().sum::<f64>() / scores.len() as f64;
    let confidence = (1.0 - mean / best.1).clamp(0.0, 1.0) as f32;

    // The transients pin it down: each strong one goes to its nearest beat,
    // and a line through them gives the beat's length and where it falls,
    // over more and more of the file.
    let sr = s.sample_rate as f64;
    // Each hit weighs what the curve (lows counted twice) says at it.
    let hits: Vec<(f64, f64)> = transients
        .iter()
        .zip(strengths)
        .filter(|(_, &w)| w > 0.15)
        .map(|(&t, _)| {
            let secs = t as f64 / sr;
            let w = (secs * rate).round() as usize;
            (secs, e[w.saturating_sub(2)..(w + 3).min(n)].iter().fold(0.0f32, |m, &x| m.max(x)) as f64)
        })
        .filter(|h| h.1 > 0.0)
        .collect();
    if hits.len() < 4 {
        return None;
    }
    let mut period = lag / rate;
    let first = hits[0].0;
    // Where the beats fall, from all the early hits: on the beat the strong
    // ones agree on (not on the off-beats of the hats).
    let (mut cs, mut sn) = (0.0f64, 0.0f64);
    for &(t, w) in hits.iter().filter(|h| h.0 <= first + 8.0 * period) {
        let a = std::f64::consts::TAU * t / period;
        cs += w * a.cos();
        sn += w * a.sin();
    }
    let mut phase = sn.atan2(cs) / std::f64::consts::TAU * period;
    phase += ((first - phase) / period).round() * period;
    let mut span = 8.0 * period;
    let end = hits.last().map_or(first, |h| h.0);
    loop {
        for _ in 0..3 {
            let (mut sw, mut sn, mut st, mut snn, mut snt) = (0.0, 0.0, 0.0, 0.0, 0.0);
            for &(t, w) in hits.iter().filter(|h| h.0 <= first + span) {
                let k = ((t - phase) / period).round();
                let r = t - phase - k * period;
                if r.abs() > 0.12 * period {
                    continue;
                }
                sw += w;
                sn += w * k;
                st += w * t;
                snn += w * k * k;
                snt += w * k * t;
            }
            let det = sw * snn - sn * sn;
            if sw <= 0.0 || det.abs() < 1e-9 {
                break;
            }
            let p = (sw * snt - sn * st) / det;
            if !(p > 0.0) || (p / period - 1.0).abs() > 0.08 {
                break;
            }
            period = p;
            phase = (st - p * sn) / sw;
        }
        if first + span >= end {
            break;
        }
        span *= 2.0;
    }
    let bpm = 60.0 / period;
    if !(30.0..=400.0).contains(&bpm) {
        return None;
    }
    // The first beat (one a hair before the start is the start).
    let beat = (phase - ((phase + 0.05 * period) / period).floor() * period).max(0.0);
    let n0 = ((beat - phase) / period).round() as i64;
    // The downbeat: of the four beats of a bar, the one with the most kick
    // (the first hit's beat when it is close).
    let dur = s.duration();
    let mut bar = [0.0f64; 4];
    let mut k = 0i64;
    loop {
        let t = beat + k as f64 * period;
        if t > dur {
            break;
        }
        // The curve's peak for a hit comes a frame or so after it.
        let peak = |c: &[f32]| (-1..=2).map(|j| Onsets::at(c, rate, t + j as f64 / rate)).fold(0.0f32, f32::max) as f64;
        let v = peak(&on.low) * 2.0 + peak(&on.flux);
        bar[(n0 + k).rem_euclid(4) as usize] += v;
        k += 1;
    }
    let first_beat = (((first - phase) / period).round() as i64).rem_euclid(4) as usize;
    // A recording usually starts on its downbeat: the first hit's beat wins
    // unless another has clearly more kick.
    let best = (0..4).max_by(|&a, &b| bar[a].total_cmp(&bar[b])).unwrap_or(0);
    let down = if bar[first_beat] > bar[best] * 0.85 { first_beat } else { best };
    let downbeat = beat + ((down as i64 - n0).rem_euclid(4)) as f64 * period;
    Some(Tempo { bpm, beat, downbeat: downbeat.max(0.0), confidence })
}

/// The bands of a window of `size` frames at `sr`: ranges of FFT bins of a
/// sixth of an octave each from 30 Hz up to 16 kHz (at least a bin each).
fn bands(size: usize, sr: u32) -> Vec<(usize, usize)> {
    let bin = |hz: f64| ((hz * size as f64 / sr as f64).round() as usize).clamp(1, size / 2);
    let top = bin(16_000.0f64.min(sr as f64 * 0.5));
    let mut out = Vec::new();
    let mut lo = bin(30.0);
    let mut f = 30.0f64;
    while lo < top {
        f *= 2f64.powf(1.0 / 6.0);
        let hi = bin(f).max(lo + 1).min(top);
        if hi > lo {
            out.push((lo, hi));
            lo = hi;
        }
    }
    out
}

/// The spectral flux of a sample (see the module's notes).
pub fn spectral_flux(sample: &SampleData) -> Flux {
    let sr = sample.sample_rate.max(1);
    // About 21 ms windows, every quarter of one.
    let size = if sr <= 50_000 {
        1024
    } else if sr <= 100_000 {
        2048
    } else {
        4096
    };
    let hop = size / 4;
    let n = sample.frames();
    let count = n / hop + 1;
    let peak = sample.left.iter().chain(&sample.right).fold(0.0f32, |m, v| m.max(v.abs()));
    if n == 0 || peak < 1e-5 {
        return Flux { hop, size, sample_rate: sr, values: vec![0.0; count] };
    }
    let bands = bands(size, sr);
    let mut fft = Fft::new(size);
    let mut buf = vec![0.0f32; size];
    let mut mags = Vec::with_capacity(size / 2);
    // The bands of the two frames before: the flux compares with the one
    // two hops back (half a window, so the windows hardly share).
    let mut back: [Vec<f32>; 2] = [vec![0.0; bands.len()], vec![0.0; bands.len()]];
    let mut cur = vec![0.0f32; bands.len()];
    let scale = 1.0 / peak;
    let mut values = Vec::with_capacity(count);
    for j in 0..count {
        if j * hop + size / 2 > n {
            // A window running off the end: the sound stopping there is no hit.
            values.push(0.0);
            continue;
        }
        let start = (j * hop) as isize - (size / 2) as isize;
        for (i, b) in buf.iter_mut().enumerate() {
            let k = start + i as isize;
            *b = if k >= 0 && (k as usize) < n { 0.5 * (sample.left[k as usize] + sample.right[k as usize]) * scale } else { 0.0 };
        }
        fft.magnitudes(&buf, &mut mags);
        for (v, &(a, b)) in cur.iter_mut().zip(&bands) {
            let e: f32 = mags[a..b].iter().sum();
            *v = (1.0 + COMPRESS * e).log10();
        }
        let prev = &back[0];
        let last = cur.len() - 1;
        let mut flux = 0.0f32;
        for (k, &v) in cur.iter().enumerate() {
            let was = prev[k.saturating_sub(1)..=(k + 1).min(last)].iter().fold(0.0f32, |m, x| m.max(*x));
            flux += (v - was).max(0.0);
        }
        values.push(flux);
        back.swap(0, 1);
        back[1].copy_from_slice(&cur);
    }
    let most = values.iter().fold(0.0f32, |m, v| m.max(*v));
    if most > 0.0 {
        for v in &mut values {
            *v /= most;
        }
    }
    Flux { hop, size, sample_rate: sr, values }
}

/// How much louder a sample gets at `frame`: the level of the 20 ms after
/// it over that of the 20 ms before, from 0 (no louder) to 1 (30 dB or
/// more, or out of silence).
pub fn level_jump(sample: &SampleData, frame: usize) -> f32 {
    let n = sample.frames();
    let w = (sample.sample_rate as usize / 50).max(1);
    let energy = |a: usize, b: usize| (a.min(n)..b.min(n)).map(|i| (0.5 * (sample.left[i] + sample.right[i])).powi(2) as f64).sum::<f64>();
    let after = energy(frame, frame + w);
    let before = energy(frame.saturating_sub(w), frame);
    let eps = 1e-9 * w as f64;
    let db = 10.0 * ((after + eps) / (before + eps)).log10();
    (db as f32 / 30.0).clamp(0.0, 1.0)
}

impl Flux {
    /// So many hops (at least one) in `secs`.
    fn hops(&self, secs: f64) -> usize {
        ((secs * self.sample_rate as f64 / self.hop as f64).round() as usize).max(1)
    }

    /// The hits of `sample` (the one this was made of), in order: peaks of
    /// the flux standing out of their surroundings by as much as
    /// `sensitivity` (0..1) asks, each put on its sample.
    pub fn hits(&self, sample: &SampleData, sensitivity: f32) -> Vec<Hit> {
        let s = sensitivity.clamp(0.0, 1.0);
        let v = &self.values;
        let (pre_max, post_max) = (self.hops(0.03), self.hops(0.03));
        let (pre_avg, post_avg) = (self.hops(0.1), self.hops(0.07));
        // How far above its surroundings a peak must be, and above nothing:
        // at 0 % only the plainest hits, at 100 % nearly every bump.
        let delta = 0.02 + 0.5 * (1.0 - s).powi(2);
        let floor = 0.01 + 0.3 * (1.0 - s).powi(2);
        let mut peaks = Vec::new();
        for (j, &x) in v.iter().enumerate() {
            if x < floor {
                continue;
            }
            let (lo, hi) = (j.saturating_sub(pre_max), (j + post_max + 1).min(v.len()));
            // A local maximum (on a flat top, its first frame).
            if v[lo..j].iter().any(|&y| y >= x) || v[j + 1..hi].iter().any(|&y| y > x) {
                continue;
            }
            let (alo, ahi) = (j.saturating_sub(pre_avg), (j + post_avg + 1).min(v.len()));
            let mean = v[alo..ahi].iter().sum::<f32>() / (ahi - alo) as f32;
            if x >= mean + delta {
                peaks.push((j, x));
            }
        }
        let gap = (MIN_GAP * self.sample_rate as f64) as usize;
        let mut hits: Vec<Hit> = Vec::with_capacity(peaks.len());
        for (j, x) in peaks {
            let after = hits.last().map_or(0, |h| h.frame + gap / 2);
            let frame = self.refine(sample, j * self.hop, after);
            match hits.last_mut() {
                Some(last) if frame < last.frame + gap => {
                    // Too close to the one before: the stronger stays.
                    if x > last.strength {
                        *last = Hit { frame, strength: x };
                    }
                }
                _ => hits.push(Hit { frame, strength: x }),
            }
        }
        for h in &mut hits {
            h.strength = 0.5 * h.strength + 0.5 * level_jump(sample, h.frame);
        }
        let most = hits.iter().fold(0.0f32, |m, h| m.max(h.strength));
        if most > 0.0 {
            for h in &mut hits {
                h.strength /= most;
            }
        }
        hits
    }

    /// Where the hit the flux saw at `centre` starts: where the energy of
    /// the next 4 ms most outweighs that of the 4 ms before (of the sound's
    /// changes, which favour its highs over a held bass). Not before `after`.
    fn refine(&self, sample: &SampleData, centre: usize, after: usize) -> usize {
        let n = sample.frames();
        let w = ((self.sample_rate as usize) / 250).max(8);
        let a = centre.saturating_sub(self.size / 2 + self.hop).max(after).min(n.saturating_sub(1));
        let b = (centre + self.size / 2).min(n).max(a + 1);
        // Running sums of the energy of the sound's changes from a - w to b + w.
        let lo = a.saturating_sub(w);
        let hi = (b + w).min(n);
        let mono = |i: usize| 0.5 * (sample.left[i] + sample.right[i]);
        let mut sums = Vec::with_capacity(hi - lo + 1);
        sums.push(0.0f64);
        let mut acc = 0.0f64;
        for i in lo..hi {
            let d = mono(i) - if i > 0 { mono(i - 1) } else { 0.0 };
            acc += (d * d) as f64;
            sums.push(acc);
        }
        let energy = |from: usize, to: usize| sums[to.clamp(lo, hi) - lo] - sums[from.clamp(lo, hi) - lo];
        let eps = acc / (hi - lo).max(1) as f64 * w as f64 * 1e-3 + 1e-12;
        let mut best = (a, f64::MIN);
        for p in a..b {
            let before = if p >= w { energy(p - w, p) } else { energy(0, p) };
            let score = (energy(p, p + w) + eps) / (before + eps);
            if score > best.1 {
                best = (p, score);
            }
        }
        best.0
    }

    /// How strong a hit starting at `frame` would be: the most the flux
    /// rises around there (for choosing which grid lines to keep).
    pub fn strength_near(&self, frame: usize) -> f32 {
        let a = frame.saturating_sub(self.size / 4) / self.hop;
        let b = ((frame + self.size / 2) / self.hop + 1).min(self.values.len());
        self.values.get(a..b).map_or(0.0, |v| v.iter().fold(0.0f32, |m, x| m.max(*x)))
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::f32::consts::TAU;

    /// A small noise source for the test drums.
    struct Noise(u32);
    impl Noise {
        fn next(&mut self) -> f32 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 17;
            self.0 ^= self.0 << 5;
            self.0 as f32 / u32::MAX as f32 * 2.0 - 1.0
        }
    }

    /// A kick (a sine falling from 150 to 50 Hz), a snare (a tone and
    /// noise), a closed hat (bright noise) or a soft hat, `secs` long.
    pub(crate) fn drum(kind: char, sr: u32, seed: u32) -> Vec<f32> {
        let mut noise = Noise(seed | 1);
        let n = (sr as f32 * 0.3) as usize;
        let mut out = Vec::with_capacity(n);
        let (mut phase, mut last) = (0.0f32, 0.0f32);
        for i in 0..n {
            let t = i as f32 / sr as f32;
            let x = match kind {
                'k' => {
                    phase += TAU * (50.0 + 100.0 * (-t / 0.03).exp()) / sr as f32;
                    0.9 * phase.sin() * (-t / 0.12).exp()
                }
                's' => 0.35 * (TAU * 185.0 * t).sin() * (-t / 0.05).exp() + 0.45 * noise.next() * (-t / 0.07).exp(),
                _ => {
                    // Noise, its lows taken out (a difference), dying fast.
                    let w = noise.next();
                    let hp = w - last;
                    last = w;
                    let level = if kind == 'h' { 0.3 } else { 0.08 };
                    level * hp * (-t / 0.025).exp()
                }
            };
            out.push(x);
        }
        out
    }

    /// Two bars of drums at 120 BPM (a sixteenth is 0.125 s): `pattern`
    /// gives each drum's steps. Returns the loop and where its hits are.
    pub(crate) fn drum_loop(sr: u32, pattern: &[(char, &[usize])], bass: bool) -> (SampleData, Vec<usize>) {
        let step = sr as usize / 8;
        let n = step * 32;
        let mut x = vec![0.0f32; n];
        let mut at = Vec::new();
        for (k, &(kind, steps)) in pattern.iter().enumerate() {
            for (m, &s) in steps.iter().enumerate() {
                let start = s * step;
                for (i, v) in drum(kind, sr, 1 + k as u32 * 977 + m as u32 * 31).iter().enumerate() {
                    if start + i < n {
                        x[start + i] += v;
                    }
                }
                at.push(start);
            }
        }
        if bass {
            // A held bass under it all, changing its note with the bars
            // (where a kick is) and wobbling a little.
            let mut phase = 0.0f32;
            for (i, v) in x.iter_mut().enumerate() {
                let t = i as f32 / sr as f32;
                let hz = if i < n / 2 { 55.0 } else { 73.4 } * (1.0 + 0.004 * (TAU * 5.0 * t).sin());
                phase += TAU * hz / sr as f32;
                *v += 0.35 * phase.sin() + 0.1 * (2.0 * phase).sin();
            }
        }
        at.sort_unstable();
        at.dedup();
        let right = x.clone();
        (SampleData::from_buffers("loop", None, sr, x, right), at)
    }

    pub(crate) const BEAT: &[(char, &[usize])] = &[
        ('k', &[0, 6, 10, 16, 22, 25]),
        ('s', &[4, 12, 20, 28]),
        ('h', &[2, 8, 14, 18, 24, 30]),
        ('q', &[11, 27]),
    ];

    fn check(hits: &[usize], want: &[usize], sr: u32, tol_ms: f32, what: &str) {
        assert_eq!(hits.len(), want.len(), "{what}: found {:?}, wanted {want:?}", hits.iter().map(|h| *h as f32 / sr as f32).collect::<Vec<_>>());
        for (h, w) in hits.iter().zip(want) {
            let ms = (*h as f32 - *w as f32) * 1000.0 / sr as f32;
            assert!(ms.abs() <= tol_ms, "{what}: a hit at {w} found at {h} ({ms:.2} ms off)");
        }
    }

    #[test]
    fn the_hits_of_a_drum_loop_are_found_where_they_are() {
        for sr in [44_100, 48_000, 96_000] {
            for bass in [false, true] {
                let (s, want) = drum_loop(sr, BEAT, bass);
                let hits = transients(&s, 0.5);
                check(&hits, &want, sr, 3.0, &format!("{sr} Hz, bass {bass}"));
            }
        }
    }

    #[test]
    fn sensitivity_finds_fewer_or_more() {
        let sr = 48_000;
        let (s, want) = drum_loop(sr, BEAT, true);
        let flux = spectral_flux(&s);
        let count = |x: f32| flux.hits(&s, x).len();
        // Turned down, the soft hats go; turned up, nothing that is there is lost.
        assert!(count(0.0) < want.len(), "{}", count(0.0));
        assert!(count(0.0) <= count(0.3) && count(0.3) <= count(0.5) && count(0.5) <= count(0.8));
        // At 0 % the snares, which stand out the most, still are hits.
        let strong = flux.hits(&s, 0.0);
        let step = sr as usize / 8;
        for k in [4, 12, 20, 28] {
            assert!(strong.iter().any(|h| h.frame.abs_diff(k * step) < step / 8), "the snare of step {k} stays: {strong:?}");
        }
        // The soft hats are the weakest hits.
        let all = flux.hits(&s, 0.5);
        let weakest: Vec<usize> = {
            let mut v = all.clone();
            v.sort_by(|a, b| a.strength.total_cmp(&b.strength));
            v.iter().take(2).map(|h| (h.frame + step / 2) / step).collect()
        };
        assert!(weakest.contains(&11) && weakest.contains(&27), "{all:?}");
        assert!(count(1.0) >= want.len());
    }

    #[test]
    fn silence_and_a_held_tone_have_no_hits_but_their_start() {
        let sr = 48_000;
        let quiet = SampleData::from_buffers("quiet", None, sr, vec![0.0; sr as usize], vec![0.0; sr as usize]);
        assert!(transients(&quiet, 1.0).is_empty());
        // A pad fading in over half a second and held: one hit at most, at its start.
        let tone: Vec<f32> = (0..sr as usize * 2).map(|i| (i as f32 / (sr as f32 * 0.5)).min(1.0) * 0.5 * (TAU * 220.0 * i as f32 / sr as f32).sin()).collect();
        let s = SampleData::from_buffers("tone", None, sr, tone.clone(), tone);
        let hits = transients(&s, 0.5);
        assert!(hits.len() <= 1 && hits.iter().all(|&h| h < sr as usize / 2), "{hits:?}");
    }

    #[test]
    fn a_strong_hit_is_strong_near_it_and_weak_between_hits() {
        let sr = 48_000;
        let (s, _) = drum_loop(sr, &[('k', &[0, 8, 16, 24])], false);
        let flux = spectral_flux(&s);
        let step = sr as usize / 8;
        assert!(flux.strength_near(8 * step) > 0.5);
        assert!(flux.strength_near(4 * step) < 0.1, "{}", flux.strength_near(4 * step));
    }
}

#[cfg(test)]
mod tempo_tests {
    use super::*;

    /// A drum loop: a kick on 1 and 3, a snare on 2 and 4, hats on the
    /// eighths, for `bars` bars at `bpm`, starting `lead` seconds in.
    fn drum_loop(bpm: f64, bars: usize, sr: u32, lead: f64) -> SampleData {
        let spb = 60.0 / bpm;
        let len = ((lead + bars as f64 * 4.0 * spb + 0.5) * sr as f64) as usize;
        let mut x = vec![0.0f32; len];
        let mut seed = 0x1234_5678u32;
        let mut noise = move || {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            (seed as f32 / u32::MAX as f32) * 2.0 - 1.0
        };
        let fs = sr as f64;
        for b in 0..bars * 8 {
            let t0 = lead + b as f64 * spb * 0.5;
            let at = (t0 * fs) as usize;
            let beat = b / 2;
            let on_beat = b % 2 == 0;
            for i in 0..(0.25 * fs) as usize {
                let t = i as f64 / fs;
                let mut v = 0.0f64;
                if on_beat && beat % 2 == 0 {
                    // Kick: a falling sine.
                    let f = 50.0 + 100.0 * (-t * 30.0).exp();
                    v += 0.9 * (std::f64::consts::TAU * f * t).sin() * (-t * 9.0).exp();
                }
                if on_beat && beat % 2 == 1 {
                    v += 0.5 * noise() as f64 * (-t * 22.0).exp() + 0.3 * (std::f64::consts::TAU * 190.0 * t).sin() * (-t * 18.0).exp();
                }
                v += 0.15 * noise() as f64 * (-t * 80.0).exp();
                if let Some(s) = x.get_mut(at + i) {
                    *s += v as f32;
                }
            }
        }
        SampleData::from_buffers("loop", None, sr, x.clone(), x)
    }

    #[test]
    fn the_analysis_keeps_the_hits_of_a_loop() {
        let s = drum_loop(120.0, 2, 44100, 0.1);
        let t = &s.analysis().transients;
        // 16 eighths in two bars, each within two milliseconds.
        assert_eq!(t.len(), 16, "{t:?}");
        for (k, &f) in t.iter().enumerate() {
            let want = ((0.1 + k as f64 * 0.25) * 44100.0) as i64;
            assert!((f as i64 - want).abs() < 90, "hit {k} at {f}, wanted {want}");
        }
    }

    #[test]
    fn tempo_of_drum_loops() {
        for (bpm, lead) in [(90.0, 0.0), (100.0, 0.31), (120.0, 0.05), (128.0, 0.0), (140.0, 0.2), (87.5, 0.0)] {
            let s = drum_loop(bpm, 8, 44100, lead);
            let t = detect_tempo(&s).unwrap_or_else(|| panic!("no tempo at {bpm}"));
            assert!((t.bpm / bpm - 1.0).abs() < 0.002, "{bpm}: found {:.3}", t.bpm);
            assert!((t.downbeat - lead).abs() < 0.01, "{bpm}: downbeat {:.3}, wanted {lead}", t.downbeat);
        }
    }

    #[test]
    fn no_tempo_in_a_drone() {
        let x: Vec<f32> = (0..44100 * 4).map(|i| (i as f32 * 0.03).sin() * 0.3).collect();
        let s = SampleData::from_buffers("drone", None, 44100, x.clone(), x);
        assert!(detect_tempo(&s).is_none());
    }
}

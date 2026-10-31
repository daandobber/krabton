//! What is in a recording: where its hits start (transients), its tempo and
//! its first downbeat, found the way Live's auto-warp finds them on import.
//!
//! Everything starts from an onset curve: the spectrum in short frames and
//! how much louder each frame got than the one before (spectral flux).
//! Peaks of that curve are the transients, placed to the sample on the
//! attack; its autocorrelation gives the beat, which the transients then
//! pin down to a small fraction of a percent.

use crate::dsp::fft::Fft;
use crate::sample::SampleData;

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

/// Transients and tempo of a sample.
pub fn analyze(s: &SampleData) -> Analysis {
    let on = onsets(s);
    let (transients, strengths) = pick_transients(s, &on);
    let tempo = tempo_from(s, &on, &transients, &strengths);
    Analysis { transients, strengths, tempo }
}

/// Where the hits of a sample start, in frames (Live's transient markers;
/// what Beats warping and slicing cut at).
pub fn transients(s: &SampleData) -> Vec<usize> {
    s.analysis().transients.clone()
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

/// The onset curve of a sample: frames of about 23 ms, every 11.6 ms.
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

/// Peaks of the onset curve above its surroundings, each placed on the
/// sample where its attack starts.
fn pick_transients(s: &SampleData, on: &Onsets) -> (Vec<usize>, Vec<f32>) {
    let f = &on.flux;
    let n = f.len();
    // The level around each frame (a third of a second), for a threshold that follows the music.
    let around = local_mean(f, 14);
    let min_gap = (0.045 * on.rate()).round().max(1.0) as usize;
    let mut picked: Vec<(usize, f32)> = Vec::new();
    for t in 0..n {
        let v = f[t];
        let thr = around[t] * 1.5 + 0.06;
        if v < thr {
            continue;
        }
        let lo = t.saturating_sub(2);
        let hi = (t + 3).min(n);
        if f[lo..hi].iter().any(|&x| x > v) || (t > 0 && f[t - 1] == v) {
            continue;
        }
        match picked.last_mut() {
            Some(last) if t - last.0 < min_gap => {
                if v > last.1 {
                    *last = (t, v);
                }
            }
            _ => picked.push((t, v)),
        }
    }
    let mut out: Vec<usize> = Vec::with_capacity(picked.len());
    let mut strengths = Vec::with_capacity(picked.len());
    for (t, v) in picked {
        let at = attack_start(s, t * on.hop, on.hop);
        if out.last().is_some_and(|&p| at <= p + 16) {
            continue;
        }
        out.push(at);
        strengths.push(v.min(1.0));
    }
    (out, strengths)
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

/// The sample where the attack near `centre` starts: the block of 32
/// samples that rises most over the loudest of the 24 blocks before it (so
/// a low note still ringing does not count), then the first sample in it
/// that gets a third of the way up from that level.
fn attack_start(s: &SampleData, centre: usize, hop: usize) -> usize {
    const B: usize = 32;
    const BACK: usize = 24;
    let from = centre.saturating_sub(2 * hop);
    let to = (centre + hop).min(s.frames());
    if to <= from + B {
        return from;
    }
    let base = from.saturating_sub(BACK * B);
    let blocks = (to - base) / B;
    let first = (from - base) / B;
    let level = |b: usize| -> f32 { (0..B).map(|j| mono(s, base + b * B + j).abs()).fold(0.0, f32::max) };
    let lv: Vec<f32> = (0..blocks).map(level).collect();
    let before = |b: usize| lv[b.saturating_sub(BACK)..b].iter().fold(0.0f32, |m, &x| m.max(x));
    let mut best = (first, f32::MIN);
    for b in first..blocks {
        let rise = lv[b] - before(b);
        if rise > best.1 {
            best = (b, rise);
        }
    }
    let b = best.0;
    let floor = before(b);
    let peak = lv[b..(b + 4).min(blocks)].iter().fold(0.0f32, |m, &x| m.max(x));
    let thr = floor + (peak - floor) * 0.33;
    let start = base + b.saturating_sub(1) * B;
    let end = base + (b + 1) * B;
    let mut at = start;
    for i in start..end {
        if mono(s, i).abs() >= thr {
            at = i;
            break;
        }
    }
    at.saturating_sub(8)
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
        acc / (n - l).max(1) as f64
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

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A drum loop: a kick on 1 and 3, a snare on 2 and 4, hats on the
    /// eighths, for `bars` bars at `bpm`, starting `lead` seconds in.
    pub(crate) fn drum_loop(bpm: f64, bars: usize, sr: u32, lead: f64) -> SampleData {
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
    fn transients_land_on_the_hits() {
        let s = drum_loop(120.0, 2, 44100, 0.1);
        let t = transients(&s);
        // 16 eighths in two bars.
        assert_eq!(t.len(), 16, "{t:?}");
        for (k, &f) in t.iter().enumerate() {
            let want = ((0.1 + k as f64 * 0.25) * 44100.0) as i64;
            assert!((f as i64 - want).abs() < 40, "hit {k} at {f}, wanted {want}");
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

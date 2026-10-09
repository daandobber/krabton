//! Lens Limiter: a lookahead brickwall limiter for mixes and masters, with
//! true-peak detection, release styles and an EBU R128 loudness meter.
//!
//! The gain path: the gain each sample needs (ceiling / peak) goes through a
//! sliding minimum over the lookahead window and then a moving average of
//! the same length, so the gain is already down when a peak arrives and
//! ramps there smoothly. Release only slows the way back up, so the ceiling
//! holds. True peak looks between the samples with a 4x interpolator.

use super::biquad::{Coefs, State};
use crate::params::{ParamSpec, Unit};
use crate::scope::DeviceTaps;

pub const STYLE: usize = 0;
pub const GAIN: usize = 1;
pub const CEILING: usize = 2;
pub const LOOKAHEAD: usize = 3;
pub const RELEASE: usize = 4;
pub const LINK: usize = 5;
pub const TRUE_PEAK: usize = 6;
pub const UNITY: usize = 7;
pub const RESET: usize = 8;

pub const STYLES: &[&str] = &["Transparent", "Punchy", "Dynamic", "Aggressive", "Allround", "Bus", "Safe", "Modern"];

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::choice("Style", "Limiter", STYLES, 7),
    ParamSpec::lin("Gain", "Limiter", 0.0, 30.0, 0.0, Unit::Db),
    ParamSpec::lin("Ceiling", "Limiter", -30.0, 0.0, -1.0, Unit::Db),
    ParamSpec::log("Lookahead", "Timing", 0.1, 5.0, 1.5, Unit::Ms),
    ParamSpec::log("Release", "Timing", 1.0, 1000.0, 100.0, Unit::Ms),
    ParamSpec::lin("Channel Link", "Timing", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::toggle("True Peak", "Output", true),
    ParamSpec::toggle("Unity Gain", "Output", false),
    ParamSpec::int("Reset Meter", "Output", 0.0, 1_000_000.0, 0.0, Unit::None),
];

/// Meter values published for the display.
pub const M_GR: usize = 0;
pub const M_MOMENTARY: usize = 1;
pub const M_SHORT: usize = 2;
pub const M_INTEGRATED: usize = 3;
pub const M_TRUE_PEAK: usize = 4;
pub const M_IN: usize = 5;
pub const M_OUT: usize = 6;
pub const M_LATENCY: usize = 7;

/// (release multiplier, adaptive release, soft clip before limiting)
const STYLE_TABLE: [(f32, bool, f32); 8] = [
    (2.0, true, 0.0),  // Transparent: slow, adaptive
    (0.5, false, 0.0), // Punchy: quick recovery keeps the attacks
    (1.0, true, 0.0),  // Dynamic: fast after short peaks, slow on long ones
    (0.25, false, 0.5), // Aggressive: very fast, with some clipping
    (1.0, false, 0.0), // Allround
    (1.5, false, 0.0), // Bus
    (3.0, false, 0.0), // Safe: slow and clean
    (1.0, true, 0.2),  // Modern: adaptive, a touch of clipping
];

const MAX_WINDOW: usize = 1024; // 5 ms at 192 kHz
const RING: usize = 2048;
const TP_TAPS: usize = 8;

/// 4x interpolation kernels for the three points between samples.
fn tp_kernels() -> [[f32; TP_TAPS]; 3] {
    let mut k = [[0.0f32; TP_TAPS]; 3];
    for (ph, row) in k.iter_mut().enumerate() {
        let frac = (ph + 1) as f64 / 4.0;
        let mut sum = 0.0;
        for (i, v) in row.iter_mut().enumerate() {
            // Taps at -3..=4 around the point frac after sample 0.
            let t = i as f64 - 3.0 - frac;
            let sinc = if t.abs() < 1e-9 { 1.0 } else { (std::f64::consts::PI * t).sin() / (std::f64::consts::PI * t) };
            let win = 0.5 + 0.5 * (std::f64::consts::PI * t / 4.5).cos();
            *v = (sinc * win) as f32;
            sum += sinc * win;
        }
        for v in row.iter_mut() {
            *v /= sum as f32;
        }
    }
    k
}

/// EBU R128 K-weighting (ITU-R BS.1770) for any sample rate.
fn k_weighting(sr: f64) -> (Coefs, Coefs) {
    let pi = std::f64::consts::PI;
    let (f0, g, q) = (1681.974450955533, 3.999843853973347, 0.7071752369554196);
    let k = (pi * f0 / sr).tan();
    let vh = 10f64.powf(g / 20.0);
    let vb = vh.powf(0.4996667741545416);
    let a0 = 1.0 + k / q + k * k;
    let shelf = Coefs { b0: (vh + vb * k / q + k * k) / a0, b1: 2.0 * (k * k - vh) / a0, b2: (vh - vb * k / q + k * k) / a0, a1: 2.0 * (k * k - 1.0) / a0, a2: (1.0 - k / q + k * k) / a0 };
    let (f0, q) = (38.13547087602444, 0.5003270373238773);
    let k = (pi * f0 / sr).tan();
    let a0 = 1.0 + k / q + k * k;
    let hp = Coefs { b0: 1.0, b1: -2.0, b2: 1.0, a1: 2.0 * (k * k - 1.0) / a0, a2: (1.0 - k / q + k * k) / a0 };
    (shelf, hp)
}

/// Loudness per EBU R128: momentary (400 ms), short-term (3 s) and
/// integrated with the absolute (-70 LUFS) and relative (-10 LU) gates.
struct Loudness {
    k: (Coefs, Coefs),
    st: [[State; 2]; 2],
    /// Energy of the current 100 ms block, and the last 30 blocks.
    acc: f64,
    acc_n: usize,
    block_len: usize,
    blocks: [f64; 30],
    nblocks: usize,
    /// Histogram of 400 ms block loudness, 0.1 LU bins from -70 LUFS.
    hist: Vec<u32>,
    momentary: f32,
    short: f32,
    integrated: f32,
}

const HIST_BINS: usize = 760;

fn lufs(energy: f64) -> f32 {
    if energy <= 1e-12 {
        -120.0
    } else {
        (-0.691 + 10.0 * energy.log10()) as f32
    }
}

impl Loudness {
    fn new(sr: f32) -> Self {
        Self {
            k: k_weighting(sr as f64),
            st: [[State::default(); 2]; 2],
            acc: 0.0,
            acc_n: 0,
            block_len: (sr as usize / 10).max(1),
            blocks: [0.0; 30],
            nblocks: 0,
            hist: vec![0; HIST_BINS],
            momentary: -120.0,
            short: -120.0,
            integrated: -120.0,
        }
    }

    fn reset(&mut self) {
        let sr_block = self.block_len;
        let k = self.k;
        *self = Loudness { k, block_len: sr_block, ..Loudness::new(1.0) };
        self.block_len = sr_block;
    }

    #[inline]
    fn push(&mut self, l: f32, r: f32) {
        let mut e = 0.0;
        for (c, x) in [l, r].into_iter().enumerate() {
            let y = self.st[c][0].run(&self.k.0, x as f64);
            let y = self.st[c][1].run(&self.k.1, y);
            e += y * y;
        }
        self.acc += e;
        self.acc_n += 1;
        if self.acc_n >= self.block_len {
            self.blocks.copy_within(0..29, 1);
            self.blocks[0] = self.acc / self.acc_n as f64;
            self.nblocks += 1;
            self.acc = 0.0;
            self.acc_n = 0;
            let mean = |n: usize| self.blocks[..n.min(self.nblocks)].iter().sum::<f64>() / n.min(self.nblocks).max(1) as f64;
            let m = mean(4);
            self.momentary = lufs(m);
            self.short = lufs(mean(30));
            if self.nblocks >= 4 && self.momentary > -70.0 {
                let bin = (((self.momentary + 70.0) * 10.0) as usize).min(HIST_BINS - 1);
                self.hist[bin] += 1;
                self.integrated = self.gated();
            }
        }
    }

    fn gated(&self) -> f32 {
        let energy = |bin: usize| 10f64.powf(((bin as f64 + 0.5) / 10.0 - 70.0 + 0.691) / 10.0);
        let (mut sum, mut n) = (0.0, 0u64);
        for (b, &c) in self.hist.iter().enumerate() {
            sum += energy(b) * c as f64;
            n += c as u64;
        }
        if n == 0 {
            return -120.0;
        }
        let relative = lufs(sum / n as f64) - 10.0;
        let first = (((relative + 70.0) * 10.0).max(0.0) as usize).min(HIST_BINS);
        let (mut sum, mut n) = (0.0, 0u64);
        for (b, &c) in self.hist.iter().enumerate().skip(first) {
            sum += energy(b) * c as f64;
            n += c as u64;
        }
        if n == 0 {
            -120.0
        } else {
            lufs(sum / n as f64)
        }
    }
}

pub struct LensLimiterState {
    kern: [[f32; TP_TAPS]; 3],
    /// Recent input per channel, for the true-peak interpolator.
    hist: [[f32; TP_TAPS]; 2],
    /// Delayed audio.
    ring: [Vec<f32>; 2],
    pos: usize,
    /// Sliding minimum of the required gain (monotonic deque of (index,
    /// value)) and its moving average.
    deque: Vec<(u64, f32)>,
    dq_head: usize,
    dq_len: usize,
    avg: Vec<f32>,
    avg_sum: f64,
    count: u64,
    window: usize,
    gain: [f32; 2],
    memory: f32,
    loud: Loudness,
    true_peak: f32,
    reset_seen: f32,
    pub taps: DeviceTaps,
}

impl LensLimiterState {
    pub fn new(sr: f32) -> Self {
        Self {
            kern: tp_kernels(),
            hist: [[0.0; TP_TAPS]; 2],
            ring: [vec![0.0; RING], vec![0.0; RING]],
            pos: 0,
            deque: vec![(0, 1.0); RING],
            dq_head: 0,
            dq_len: 0,
            avg: vec![1.0; RING],
            avg_sum: 0.0,
            count: 0,
            window: 0,
            gain: [1.0; 2],
            memory: 0.0,
            loud: Loudness::new(sr),
            true_peak: -120.0,
            reset_seen: 0.0,
            taps: DeviceTaps::full(),
        }
    }

    /// Latency in samples at these settings.
    pub fn latency(p: &[f32], sr: f32) -> usize {
        Self::window_for(p, sr) + if p.get(TRUE_PEAK).copied().unwrap_or(1.0) >= 0.5 { 4 } else { 0 }
    }

    fn window_for(p: &[f32], sr: f32) -> usize {
        ((p.get(LOOKAHEAD).copied().unwrap_or(1.5).clamp(0.1, 5.0) * 0.001 * sr) as usize).clamp(1, MAX_WINDOW)
    }

    /// Sliding minimum: push value `v` for sample `idx`, drop what left the
    /// window, return the minimum of the last `w` values.
    #[inline]
    fn slide_min(&mut self, idx: u64, v: f32, w: usize) -> f32 {
        while self.dq_len > 0 {
            let back = (self.dq_head + self.dq_len - 1) % RING;
            if self.deque[back].1 >= v {
                self.dq_len -= 1;
            } else {
                break;
            }
        }
        let at = (self.dq_head + self.dq_len) % RING;
        self.deque[at] = (idx, v);
        self.dq_len += 1;
        while self.deque[self.dq_head].0 + w as u64 <= idx {
            self.dq_head = (self.dq_head + 1) % RING;
            self.dq_len -= 1;
        }
        self.deque[self.dq_head].1
    }

    pub fn process(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32], sr: f32) {
        if p.len() < SPECS.len() {
            return;
        }
        let n = l.len().min(r.len());
        if p[RESET] != self.reset_seen {
            self.reset_seen = p[RESET];
            self.loud.reset();
            self.true_peak = -120.0;
        }
        let (rel_mul, adaptive, clip) = STYLE_TABLE[(p[STYLE].round().max(0.0) as usize).min(STYLE_TABLE.len() - 1)];
        let gain_in = 10f32.powf(p[GAIN].clamp(0.0, 30.0) / 20.0);
        let ceiling = 10f32.powf(p[CEILING].clamp(-30.0, 0.0) / 20.0);
        let w = Self::window_for(p, sr);
        if w != self.window {
            // New window: start the averages over.
            self.window = w;
            self.avg_sum = w as f64;
            self.avg.iter_mut().for_each(|v| *v = 1.0);
            self.dq_len = 0;
        }
        let tp = p[TRUE_PEAK] >= 0.5;
        let tp_delay = if tp { 4 } else { 0 };
        let delay = w - 1 + tp_delay;
        let link = p[LINK].clamp(0.0, 1.0);
        let coef = |ms: f32| 1.0 - (-1.0 / (ms.max(0.01) * 0.001 * sr)).exp();
        let base_rel = p[RELEASE].clamp(1.0, 1000.0) * rel_mul;
        let mem_k = coef(800.0);
        let unity = p[UNITY] >= 0.5;
        let (mut gr_min, mut in_pk, mut out_pk) = (0.0f32, 0.0f32, 0.0f32);
        for i in 0..n {
            let mut x = [l[i] * gain_in, r[i] * gain_in];
            in_pk = in_pk.max(x[0].abs()).max(x[1].abs());
            if clip > 0.0 {
                // A little soft clipping above the ceiling before limiting.
                for v in x.iter_mut() {
                    let t = *v / ceiling;
                    if t.abs() > 1.0 - clip * 0.5 {
                        *v = ceiling * (t + (t.tanh() - t) * clip);
                    }
                }
            }
            // Peak per channel, between the samples too.
            let mut pk = [0.0f32; 2];
            for c in 0..2 {
                let h = &mut self.hist[c];
                h.copy_within(1.., 0);
                h[TP_TAPS - 1] = x[c];
                // The sample `tp_delay` back, and the points after it.
                let mut m = h[TP_TAPS - 1 - tp_delay].abs();
                if tp {
                    for k in &self.kern {
                        let v: f32 = k.iter().zip(h.iter()).map(|(a, b)| a * b).sum();
                        m = m.max(v.abs());
                    }
                }
                pk[c] = m;
            }
            let linked = pk[0].max(pk[1]);
            let need = |pk: f32| if pk > ceiling { ceiling / pk } else { 1.0 };
            // One gain path for the linked part; channels get their own on top.
            let idx = self.count;
            self.count += 1;
            let req = need(linked).min(1.0);
            let mn = self.slide_min(idx, req, w);
            let slot = (idx % RING as u64) as usize;
            self.avg_sum += mn as f64 - self.avg[(slot + RING - w) % RING] as f64;
            self.avg[slot] = mn;
            let smooth = (self.avg_sum / w as f64).min(1.0) as f32;
            // Release: adaptive styles recover fast after short peaks.
            let depth = (1.0 - self.gain[0].min(self.gain[1])).min(1.0);
            self.memory += (depth - self.memory) * mem_k;
            let rel_ms = if adaptive { base_rel * (0.3 + 2.7 * (self.memory * 4.0).min(1.0)) } else { base_rel };
            let rel = coef(rel_ms);
            for c in 0..2 {
                // Unlinked: each channel only follows its own need.
                let own = need(pk[c]);
                let target = smooth.max(own.min(smooth + (1.0 - smooth) * (1.0 - link)));
                let target = if link >= 0.999 { smooth } else { target.min(1.0) };
                let g = &mut self.gain[c];
                *g = if target < *g { target } else { *g + (target - *g) * rel };
            }
            // The delayed audio meets its gain; a final clip guards the ceiling.
            let read = (self.pos + RING - delay) % RING;
            let mut y = [0.0f32; 2];
            for c in 0..2 {
                self.ring[c][self.pos] = x[c];
                let d = self.ring[c][read];
                y[c] = (d * self.gain[c]).clamp(-ceiling, ceiling);
            }
            self.pos = (self.pos + 1) % RING;
            gr_min = gr_min.min(20.0 * self.gain[0].min(self.gain[1]).max(1e-6).log10());
            out_pk = out_pk.max(y[0].abs()).max(y[1].abs());
            self.loud.push(y[0], y[1]);
            if unity {
                y[0] /= gain_in;
                y[1] /= gain_in;
            }
            l[i] = y[0];
            r[i] = y[1];
        }
        let db = |v: f32| 20.0 * v.max(1e-9).log10();
        self.true_peak = self.true_peak.max(db(out_pk));
        if let Some(m) = &mut self.taps.meters {
            m.values[M_GR] = gr_min;
            m.values[M_MOMENTARY] = self.loud.momentary;
            m.values[M_SHORT] = self.loud.short;
            m.values[M_INTEGRATED] = self.loud.integrated;
            m.values[M_TRUE_PEAK] = self.true_peak;
            m.values[M_IN] = db(in_pk);
            m.values[M_OUT] = db(out_pk);
            m.values[M_LATENCY] = delay as f32;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    fn run(p: &[f32], x: &[f32], sr: f32) -> (Vec<f32>, LensLimiterState) {
        let mut st = LensLimiterState::new(sr);
        let mut l = x.to_vec();
        let mut r = x.to_vec();
        for (cl, cr) in l.chunks_mut(256).zip(r.chunks_mut(256)) {
            st.process(p, cl, cr, sr);
        }
        (l, st)
    }

    fn defaults() -> Vec<f32> {
        crate::params::defaults(SPECS)
    }

    #[test]
    fn no_sample_goes_over_the_ceiling_in_any_style() {
        let sr = 48000.0;
        // Drums-like: bursts of a loud tone with quieter parts, ending loud.
        let x: Vec<f32> = (0..45600).map(|i| {
            let env = if (i / 2400) % 3 == 0 { 1.0 } else { 0.25 };
            env * 1.8 * (2.0 * PI * 110.0 * i as f32 / sr).sin()
        }).collect();
        let mut p = defaults();
        p[CEILING] = -1.0;
        let ceiling = 10f32.powf(-1.0 / 20.0);
        for style in 0..STYLES.len() {
            p[STYLE] = style as f32;
            let (y, st) = run(&p, &x, sr);
            let peak = y.iter().fold(0.0f32, |m, v| m.max(v.abs()));
            assert!(peak <= ceiling + 1e-6, "{}: {peak}", STYLES[style]);
            assert!(peak > ceiling * 0.95, "{}: pushes up to the ceiling ({peak})", STYLES[style]);
            assert!(st.taps.meters.as_ref().unwrap().values[M_GR] < -3.0);
        }
    }

    #[test]
    fn the_gain_ramps_down_before_a_peak() {
        let sr = 48000.0;
        let mut x = vec![0.1f32; 4800];
        x[3000] = 2.0; // one hot sample
        let mut p = defaults();
        p[LOOKAHEAD] = 2.0;
        p[TRUE_PEAK] = 0.0;
        let (y, _) = run(&p, &x, sr);
        let w = (0.002 * sr) as usize;
        let d = w - 1;
        // The spike lands at 3000 + latency, at the ceiling; the samples just
        // before it are already turned down, smoothly.
        let ceiling = 10f32.powf(-1.0 / 20.0);
        assert!((y[3000 + d] - ceiling).abs() < 1e-3, "{}", y[3000 + d]);
        let before = y[3000 + d - w / 2];
        assert!(before < 0.1 && before > 0.03, "half way down the ramp: {before}");
        assert!((y[3000 + d - w - 10] - 0.1).abs() < 1e-4, "untouched before the window");
    }

    #[test]
    fn true_peak_catches_peaks_between_samples() {
        let sr = 48000.0;
        // A tone at fs/4 with a 45° phase: samples at 0.707, the wave at 1.
        let x: Vec<f32> = (0..48000).map(|i| 1.2 * (2.0 * PI * (i as f32 / 4.0) + PI / 4.0).sin()).collect();
        let mut p = defaults();
        p[CEILING] = 0.0;
        p[TRUE_PEAK] = 0.0;
        let (y, _) = run(&p, &x, sr);
        let sample_peak = y[24000..].iter().fold(0.0f32, |m, v| m.max(v.abs()));
        // Sample peaks pass at 0.85 (under 1): the real wave is at 1.2.
        assert!(sample_peak > 0.8);
        p[TRUE_PEAK] = 1.0;
        let (y, _) = run(&p, &x, sr);
        let tp = y[24000..].iter().fold(0.0f32, |m, v| m.max(v.abs()));
        assert!(tp < sample_peak * 0.9, "true peak limiting turns it down further: {tp}");
    }

    #[test]
    fn loudness_of_a_reference_tone() {
        // EBU: a 1 kHz sine at -23 dBFS on both channels reads -20.3 LUFS
        // (K-weighting adds about +0.7 dB at 1 kHz, two channels add 3 dB).
        let sr = 48000.0;
        let amp = 10f32.powf(-23.0 / 20.0) * 2f32.sqrt();
        let x: Vec<f32> = (0..sr as usize * 5).map(|i| amp * (2.0 * PI * 1000.0 * i as f32 / sr).sin()).collect();
        let mut p = defaults();
        p[CEILING] = 0.0;
        let (_, st) = run(&p, &x, sr);
        let m = st.taps.meters.as_ref().unwrap().values;
        for (name, v) in [("momentary", m[M_MOMENTARY]), ("short", m[M_SHORT]), ("integrated", m[M_INTEGRATED])] {
            assert!((v + 20.0).abs() < 0.5, "{name}: {v} LUFS");
        }
    }
}

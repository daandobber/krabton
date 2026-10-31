//! A small radix-2 FFT for spectrum analyzers, and a plan of one that can
//! be done a step at a time (for the Stretch's big windows).

use std::f32::consts::PI;

pub struct Fft {
    n: usize,
    cos: Vec<f32>,
    sin: Vec<f32>,
    rev: Vec<usize>,
    window: Vec<f32>,
    re: Vec<f32>,
    im: Vec<f32>,
}

impl Fft {
    /// `n` must be a power of two.
    pub fn new(n: usize) -> Self {
        assert!(n.is_power_of_two() && n >= 4);
        let bits = n.trailing_zeros();
        let rev = (0..n).map(|i| i.reverse_bits() >> (usize::BITS - bits)).collect();
        let cos = (0..n / 2).map(|k| (2.0 * PI * k as f32 / n as f32).cos()).collect();
        let sin = (0..n / 2).map(|k| -(2.0 * PI * k as f32 / n as f32).sin()).collect();
        let window = (0..n).map(|i| 0.5 - 0.5 * (2.0 * PI * i as f32 / n as f32).cos()).collect();
        Self { n, cos, sin, rev, window, re: vec![0.0; n], im: vec![0.0; n] }
    }

    pub fn len(&self) -> usize {
        self.n
    }

    pub fn is_empty(&self) -> bool {
        self.n == 0
    }

    fn transform(&mut self) {
        let n = self.n;
        let mut size = 2;
        while size <= n {
            let half = size / 2;
            let step = n / size;
            for start in (0..n).step_by(size) {
                for k in 0..half {
                    let (wr, wi) = (self.cos[k * step], self.sin[k * step]);
                    let (a, b) = (start + k, start + k + half);
                    let tr = self.re[b] * wr - self.im[b] * wi;
                    let ti = self.re[b] * wi + self.im[b] * wr;
                    self.re[b] = self.re[a] - tr;
                    self.im[b] = self.im[a] - ti;
                    self.re[a] += tr;
                    self.im[a] += ti;
                }
            }
            size *= 2;
        }
    }

    /// Level per bin in dB (n/2 bins, bin k at k * sr / n Hz) of the last
    /// `n` samples of `x`, Hann windowed, scaled so a full-scale sine reads
    /// 0 dB at its frequency.
    pub fn spectrum_db(&mut self, x: &[f32], out: &mut Vec<f32>) {
        self.magnitudes(x, out);
        for m in out.iter_mut() {
            *m = 20.0 * m.max(1e-10).log10();
        }
    }

    /// The same as [`Fft::spectrum_db`], as plain levels (a full-scale sine
    /// reads 1 at its frequency).
    pub fn magnitudes(&mut self, x: &[f32], out: &mut Vec<f32>) {
        let n = self.n;
        let x = &x[x.len().saturating_sub(n)..];
        for i in 0..n {
            let v = x.get(i).copied().unwrap_or(0.0) * self.window[i];
            self.re[self.rev[i]] = v;
            self.im[self.rev[i]] = 0.0;
        }
        self.transform();
        let scale = 4.0 / n as f32;
        out.clear();
        out.extend((0..n / 2).map(|k| (self.re[k] * self.re[k] + self.im[k] * self.im[k]).sqrt() * scale));
    }
}

/// A complex FFT of one size that can be done a step at a time (the bit
/// reversal, then one pass per doubling), so a big transform can be spread
/// over several audio blocks. Forward only; for the inverse transform
/// conjugate before and after (and divide by the size).
pub struct Plan {
    n: usize,
    cos: Vec<f32>,
    sin: Vec<f32>,
    rev: Vec<u32>,
}

impl Plan {
    /// `n` must be a power of two.
    pub fn new(n: usize) -> Self {
        assert!(n.is_power_of_two() && n >= 4);
        let bits = n.trailing_zeros();
        let rev = (0..n).map(|i| (i.reverse_bits() >> (usize::BITS - bits)) as u32).collect();
        let cos = (0..n / 2).map(|k| (2.0 * std::f64::consts::PI * k as f64 / n as f64).cos() as f32).collect();
        let sin = (0..n / 2).map(|k| -(2.0 * std::f64::consts::PI * k as f64 / n as f64).sin() as f32).collect();
        Self { n, cos, sin, rev }
    }

    pub fn len(&self) -> usize {
        self.n
    }

    pub fn is_empty(&self) -> bool {
        self.n == 0
    }

    /// The passes after the bit reversal.
    pub fn passes(&self) -> usize {
        self.n.trailing_zeros() as usize
    }

    /// Puts the values in bit-reversed order (the first step).
    pub fn reorder(&self, re: &mut [f32], im: &mut [f32]) {
        for i in 0..self.n {
            let j = self.rev[i] as usize;
            if j > i {
                re.swap(i, j);
                im.swap(i, j);
            }
        }
    }

    /// Pass `k` (0..passes): the butterflies of size 2^(k+1).
    pub fn pass(&self, k: usize, re: &mut [f32], im: &mut [f32]) {
        let n = self.n;
        let size = 2 << k;
        let half = size / 2;
        let step = n / size;
        for start in (0..n).step_by(size) {
            for j in 0..half {
                let (wr, wi) = (self.cos[j * step], self.sin[j * step]);
                let (a, b) = (start + j, start + j + half);
                let tr = re[b] * wr - im[b] * wi;
                let ti = re[b] * wi + im[b] * wr;
                re[b] = re[a] - tr;
                im[b] = im[a] - ti;
                re[a] += tr;
                im[a] += ti;
            }
        }
    }

    /// The whole transform at once.
    pub fn forward(&self, re: &mut [f32], im: &mut [f32]) {
        self.reorder(re, im);
        for k in 0..self.passes() {
            self.pass(k, re, im);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_plan_matches_the_analyzer_and_goes_back() {
        let n = 1024;
        let x: Vec<f32> = (0..n).map(|i| ((i * 7 % 13) as f32 - 6.0) / 6.0).collect();
        let plan = Plan::new(n);
        let mut re = x.clone();
        let mut im = vec![0.0; n];
        plan.forward(&mut re, &mut im);
        // A plain DFT of bin 5.
        let (mut sr, mut si) = (0.0f64, 0.0f64);
        for (i, v) in x.iter().enumerate() {
            let a = -2.0 * std::f64::consts::PI * 5.0 * i as f64 / n as f64;
            sr += *v as f64 * a.cos();
            si += *v as f64 * a.sin();
        }
        assert!((re[5] as f64 - sr).abs() < 1e-2 && (im[5] as f64 - si).abs() < 1e-2);
        // Inverse: conjugate, forward, conjugate, divide.
        for v in im.iter_mut() {
            *v = -*v;
        }
        plan.forward(&mut re, &mut im);
        for (a, b) in re.iter().zip(&x) {
            assert!((a / n as f32 - b).abs() < 1e-4);
        }
    }

    #[test]
    fn a_sine_shows_up_in_its_bin_at_its_level() {
        let n = 4096;
        let sr = 48000.0;
        let bin = 100;
        let f = bin as f32 * sr / n as f32;
        let x: Vec<f32> = (0..n).map(|i| 0.5 * (2.0 * PI * f * i as f32 / sr).sin()).collect();
        let mut fft = Fft::new(n);
        let mut out = Vec::new();
        fft.spectrum_db(&x, &mut out);
        let peak = (0..out.len()).max_by(|&a, &b| out[a].partial_cmp(&out[b]).unwrap()).unwrap();
        assert_eq!(peak, bin);
        assert!((out[bin] - 20.0 * 0.5f32.log10()).abs() < 0.1, "{}", out[bin]);
        assert!(out[bin + 20] < -80.0, "the window keeps leakage low: {}", out[bin + 20]);
    }
}

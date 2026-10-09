//! A small radix-2 FFT for spectrum analyzers.

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
        out.extend((0..n / 2).map(|k| {
            let m = (self.re[k] * self.re[k] + self.im[k] * self.im[k]).sqrt() * scale;
            20.0 * m.max(1e-10).log10()
        }));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

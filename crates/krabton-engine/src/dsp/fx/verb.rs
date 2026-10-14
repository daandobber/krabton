//! Building blocks of the big reverbs (Sky, Massive, Smear): delay lines
//! of a power-of-two size read with interpolation, all-passes on them, and
//! the lossless matrices that mix a feedback delay network.

/// A delay line of a power-of-two size: written a sample at a time, read
/// at any (fractional) delay.
pub struct Line {
    buf: Vec<f32>,
    mask: usize,
    w: usize,
}

impl Line {
    /// Room for at least `len` samples of delay.
    pub fn new(len: usize) -> Self {
        let size = (len + 4).next_power_of_two();
        Self { buf: vec![0.0; size], mask: size - 1, w: 0 }
    }

    /// The longest delay it can read.
    pub fn max_delay(&self) -> f32 {
        (self.mask - 3) as f32
    }

    #[inline]
    pub fn write(&mut self, x: f32) {
        self.w = self.w.wrapping_add(1);
        self.buf[self.w & self.mask] = x;
    }

    /// The sample written `d` samples ago (whole samples, at least 1).
    #[inline]
    pub fn tap(&self, d: usize) -> f32 {
        self.buf[self.w.wrapping_sub(d) & self.mask]
    }

    /// Read `d` samples back with linear interpolation (for long, slowly
    /// modulated lines).
    #[inline]
    pub fn read(&self, d: f32) -> f32 {
        let d = d.clamp(1.0, self.max_delay());
        let i = d as usize;
        let f = d - i as f32;
        let a = self.tap(i);
        let b = self.tap(i + 1);
        a + (b - a) * f
    }

    /// Read with cubic (Hermite) interpolation: cleaner when the delay moves fast.
    #[inline]
    pub fn read_cubic(&self, d: f32) -> f32 {
        let d = d.clamp(2.0, self.max_delay() - 1.0);
        let i = d as usize;
        let f = d - i as f32;
        let (xm1, x0, x1, x2) = (self.tap(i - 1), self.tap(i), self.tap(i + 1), self.tap(i + 2));
        let c1 = 0.5 * (x1 - xm1);
        let c2 = xm1 - 2.5 * x0 + 2.0 * x1 - 0.5 * x2;
        let c3 = 0.5 * (x2 - xm1) + 1.5 * (x0 - x1);
        ((c3 * f + c2) * f + c1) * f + x0
    }

    pub fn clear(&mut self) {
        self.buf.fill(0.0);
    }
}

/// A Schroeder all-pass on a line, its delay free to move.
pub struct Ap {
    line: Line,
}

impl Ap {
    pub fn new(len: usize) -> Self {
        Self { line: Line::new(len + 2) }
    }

    #[inline]
    pub fn tick(&mut self, x: f32, delay: f32, g: f32) -> f32 {
        let d = self.line.read(delay);
        let v = x + g * d;
        self.line.write(v);
        d - g * v
    }

    pub fn clear(&mut self) {
        self.line.clear();
    }
}

/// The Hadamard matrix (normalized, so it keeps energy) on 4, 8 or 16 values.
#[inline]
pub fn hadamard(x: &mut [f32]) {
    let n = x.len();
    debug_assert!(n.is_power_of_two());
    let mut h = 1;
    while h < n {
        let mut i = 0;
        while i < n {
            for j in i..i + h {
                let (a, b) = (x[j], x[j + h]);
                x[j] = a + b;
                x[j + h] = a - b;
            }
            i += 2 * h;
        }
        h *= 2;
    }
    let s = 1.0 / (n as f32).sqrt();
    for v in x.iter_mut() {
        *v *= s;
    }
}

/// The Householder reflection I - 2/N: every line gets a little of all the others.
#[inline]
pub fn householder(x: &mut [f32]) {
    let k = 2.0 / x.len() as f32 * x.iter().sum::<f32>();
    for v in x.iter_mut() {
        *v -= k;
    }
}

/// A gentle limit for what goes round a feedback loop.
#[inline]
pub fn soft(x: f32) -> f32 {
    if x.abs() < 1.0 {
        x
    } else {
        x.signum() * (1.0 + (x.abs() - 1.0).tanh() * 0.5)
    }
}

/// Lengths spread between `lo` and `hi` (geometrically, nudged off any
/// common ratio), as many as `out` holds.
pub fn spread(lo: f32, hi: f32, out: &mut [f32]) {
    let n = out.len();
    const NUDGE: [f32; 16] = [0.0, 0.031, -0.017, 0.043, -0.029, 0.011, 0.037, -0.041, 0.023, -0.007, 0.047, -0.033, 0.019, -0.023, 0.029, 0.0];
    for (k, o) in out.iter_mut().enumerate() {
        let t = if n > 1 { k as f32 / (n - 1) as f32 } else { 0.0 };
        *o = lo * (hi / lo).powf(t) * (1.0 + NUDGE[k % 16]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hadamard_keeps_energy() {
        let mut x = [0.3, -1.0, 0.5, 0.25, 0.0, 0.7, -0.2, 0.1];
        let e0: f32 = x.iter().map(|v| v * v).sum();
        hadamard(&mut x);
        let e1: f32 = x.iter().map(|v| v * v).sum();
        assert!((e0 - e1).abs() < 1e-5, "{e0} {e1}");
        let mut y = x;
        householder(&mut y);
        let e2: f32 = y.iter().map(|v| v * v).sum();
        assert!((e0 - e2).abs() < 1e-5);
    }

    #[test]
    fn a_line_delays() {
        let mut l = Line::new(100);
        for i in 0..50 {
            l.write(i as f32);
        }
        assert_eq!(l.tap(1), 48.0);
        assert!((l.read(10.5) - 38.5).abs() < 1e-5);
        assert!((l.read_cubic(10.0) - 39.0).abs() < 1e-4);
    }
}

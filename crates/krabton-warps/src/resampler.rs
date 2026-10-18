//! Windowed-sinc sample rate conversion between the host rate and the
//! module's rate (Krabton's own, as in krabton-clouds; not part of Warps). Positions are kept as exact
//! fractions of the two (integer) rates, so the two directions never drift
//! apart.

/// Interpolator taps and phases.
pub const TAPS: usize = 64;
pub const HALF: usize = TAPS / 2;
const PHASES: usize = 128;

fn bessel_i0(x: f64) -> f64 {
    let mut sum = 1.0;
    let mut term = 1.0;
    for k in 1..50 {
        term *= x / (2.0 * k as f64);
        sum += term * term;
    }
    sum
}

/// Kaiser-windowed sinc kernels, one row of `TAPS` per phase (plus a last
/// row for interpolation), each summing to 1. `cutoff` is relative to the
/// input Nyquist frequency.
fn kernel(cutoff: f64) -> Box<[f32]> {
    const BETA: f64 = 8.0;
    let mut k = vec![0.0f32; (PHASES + 1) * TAPS];
    for p in 0..=PHASES {
        let frac = p as f64 / PHASES as f64;
        let mut vals = [0.0f64; TAPS];
        let mut sum = 0.0;
        for (i, v) in vals.iter_mut().enumerate() {
            let d = (HALF as f64 - 1.0 - i as f64) + frac;
            let x = d * cutoff;
            let sinc = if x.abs() < 1e-9 { 1.0 } else { (std::f64::consts::PI * x).sin() / (std::f64::consts::PI * x) };
            let t = (d / HALF as f64).clamp(-1.0, 1.0);
            *v = sinc * bessel_i0(BETA * (1.0 - t * t).sqrt()) / bessel_i0(BETA);
            sum += *v;
        }
        for (r, v) in k[p * TAPS..(p + 1) * TAPS].iter_mut().zip(vals) {
            *r = (v / sum) as f32;
        }
    }
    k.into_boxed_slice()
}

/// Stereo resampler. The output position runs between the two middle taps
/// of the history, `frac / denominator` of the way.
pub struct Resampler {
    kernel: Box<[f32]>,
    hist_l: [f32; 2 * TAPS],
    hist_r: [f32; 2 * TAPS],
    hist_pos: usize,
    /// Position of the next output, in 1/denominator input samples.
    frac: u64,
    /// Input samples per output sample, times `denominator`.
    step: u64,
    denominator: u64,
}

impl Resampler {
    pub fn new(input_rate: u32, output_rate: u32) -> Self {
        let cutoff = 0.9 * (output_rate as f64 / input_rate as f64).min(1.0);
        Self {
            kernel: kernel(cutoff),
            hist_l: [0.0; 2 * TAPS],
            hist_r: [0.0; 2 * TAPS],
            hist_pos: 0,
            frac: 0,
            step: input_rate as u64,
            denominator: output_rate as u64,
        }
    }

    #[inline]
    pub fn push(&mut self, x: [f32; 2]) {
        self.hist_l[self.hist_pos] = x[0];
        self.hist_l[self.hist_pos + TAPS] = x[0];
        self.hist_r[self.hist_pos] = x[1];
        self.hist_r[self.hist_pos + TAPS] = x[1];
        self.hist_pos = (self.hist_pos + 1) % TAPS;
    }

    #[inline]
    fn interpolate(&self) -> [f32; 2] {
        let pos = self.frac as f32 / self.denominator as f32 * PHASES as f32;
        let row = (pos as usize).min(PHASES - 1);
        let t = pos - row as f32;
        let k0 = &self.kernel[row * TAPS..(row + 1) * TAPS];
        let k1 = &self.kernel[(row + 1) * TAPS..(row + 2) * TAPS];
        let hl = &self.hist_l[self.hist_pos..self.hist_pos + TAPS];
        let hr = &self.hist_r[self.hist_pos..self.hist_pos + TAPS];
        let (mut yl, mut yr) = (0.0, 0.0);
        for j in 0..TAPS {
            let c = k0[j] + (k1[j] - k0[j]) * t;
            yl += c * hl[j];
            yr += c * hr[j];
        }
        [yl, yr]
    }

    /// Push-driven use: after each `push`, call this until it returns
    /// `None`; it yields the outputs that are now due.
    #[inline]
    pub fn next_output(&mut self) -> Option<[f32; 2]> {
        if self.frac < self.denominator {
            let y = self.interpolate();
            self.frac += self.step;
            Some(y)
        } else {
            self.frac -= self.denominator;
            None
        }
    }

    /// Pull-driven use: returns one output and how many inputs to `push`
    /// before the next one.
    #[inline]
    pub fn output_and_advance(&mut self) -> ([f32; 2], usize) {
        let y = self.interpolate();
        self.frac += self.step;
        let mut needed = 0;
        while self.frac >= self.denominator {
            self.frac -= self.denominator;
            needed += 1;
        }
        (y, needed)
    }
}

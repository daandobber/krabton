//! Runs a 48 kHz, 24-sample block process at any host rate: the mono input
//! is resampled to 48 kHz, cut in blocks, and the stereo output resampled
//! back, both with Kaiser-windowed sinc interpolators (as in Krabton's
//! Elements instrument). At 48 kHz the samples only pass through the block
//! buffer (23 samples of latency).
//!
//! Krabton code, MIT license.

use crate::dsp::{BLOCK_SIZE, SAMPLE_RATE};

/// Taps of the interpolators when not decimating; more when decimating.
const BASE_TAPS: usize = 64;
const MAX_TAPS: usize = 512;
/// Kernel phases (linearly interpolated).
const PHASES: usize = 64;

fn bessel_i0(x: f64) -> f64 {
    let mut sum = 1.0;
    let mut term = 1.0;
    for k in 1..60 {
        term *= x / (2.0 * k as f64);
        sum += term * term;
    }
    sum
}

/// A Kaiser-windowed sinc interpolator: `PHASES + 1` rows of `taps`
/// coefficients, each summing to 1.
struct Kernel {
    taps: usize,
    table: Vec<f32>,
}

impl Kernel {
    /// `ratio` is output rate / input rate.
    fn new(ratio: f64) -> Self {
        const BETA: f64 = 8.0;
        let cutoff = 0.9 * ratio.min(1.0);
        let taps = ((BASE_TAPS as f64 / ratio.min(1.0)).ceil() as usize).next_multiple_of(2).clamp(BASE_TAPS, MAX_TAPS);
        let half = taps / 2;
        let mut table = vec![0.0f32; (PHASES + 1) * taps];
        let mut vals = vec![0.0f64; taps];
        for p in 0..=PHASES {
            let frac = p as f64 / PHASES as f64;
            let mut sum = 0.0;
            for (i, v) in vals.iter_mut().enumerate() {
                let d = (half as f64 - 1.0 - i as f64) + frac;
                let x = d * cutoff;
                let sinc =
                    if x.abs() < 1e-9 { 1.0 } else { (std::f64::consts::PI * x).sin() / (std::f64::consts::PI * x) };
                let t = (d / half as f64).clamp(-1.0, 1.0);
                *v = sinc * bessel_i0(BETA * (1.0 - t * t).sqrt()) / bessel_i0(BETA);
                sum += *v;
            }
            for (r, v) in table[p * taps..(p + 1) * taps].iter_mut().zip(&vals) {
                *r = (v / sum) as f32;
            }
        }
        Self { taps, table }
    }

    /// The signal at `frac` (0..1) between the two middle samples of
    /// `history` (oldest first, `taps` long).
    #[inline]
    fn at(&self, history: &[f32], frac: f32) -> f32 {
        let pos = frac.clamp(0.0, 1.0) * PHASES as f32;
        let row = (pos as usize).min(PHASES - 1);
        let t = pos - row as f32;
        let k0 = &self.table[row * self.taps..(row + 1) * self.taps];
        let k1 = &self.table[(row + 1) * self.taps..(row + 2) * self.taps];
        let mut y = 0.0;
        for j in 0..self.taps {
            y += (k0[j] + (k1[j] - k0[j]) * t) * history[j];
        }
        y
    }
}

/// A ring of the last `taps` samples, stored twice so that the window is
/// always contiguous.
struct History {
    data: Vec<f32>,
    pos: usize,
    taps: usize,
}

impl History {
    fn new(taps: usize) -> Self {
        Self { data: vec![0.0; 2 * taps], pos: 0, taps }
    }

    #[inline]
    fn push(&mut self, x: f32) {
        self.data[self.pos] = x;
        self.data[self.pos + self.taps] = x;
        self.pos = (self.pos + 1) % self.taps;
    }

    #[inline]
    fn window(&self) -> &[f32] {
        &self.data[self.pos..self.pos + self.taps]
    }
}

pub struct BlockAdapter {
    direct: bool,
    // Host to 48 kHz.
    in_kernel: Kernel,
    in_history: History,
    /// Position of the next 48 kHz sample after the middle of the history,
    /// in host samples.
    in_pos: f32,
    in_step: f32,
    /// Host samples since the last non-zero input.
    in_quiet: usize,
    // Blocks.
    block_in: [f32; BLOCK_SIZE],
    block_l: [f32; BLOCK_SIZE],
    block_r: [f32; BLOCK_SIZE],
    block_fill: usize,
    // 48 kHz output queue.
    fifo_l: Vec<f32>,
    fifo_r: Vec<f32>,
    fifo_head: usize,
    fifo_len: usize,
    // 48 kHz to host.
    out_kernel: Kernel,
    out_history_l: History,
    out_history_r: History,
    out_frac: f32,
    out_step: f32,
    /// Output samples queued at the start.
    priming: usize,
}

impl BlockAdapter {
    pub fn new(host_rate: f32) -> Self {
        let direct = host_rate == SAMPLE_RATE;
        let s = SAMPLE_RATE as f64 / host_rate as f64;
        let in_kernel = Kernel::new(if direct { 1.0 } else { s });
        let out_kernel = Kernel::new(if direct { 1.0 } else { 1.0 / s });
        let priming = if direct { BLOCK_SIZE - 1 } else { BLOCK_SIZE + s.ceil() as usize + 1 };
        let capacity = priming + 4 * BLOCK_SIZE + 2 * s.ceil() as usize;
        let mut a = Self {
            direct,
            in_history: History::new(in_kernel.taps),
            in_kernel,
            in_pos: 1.0,
            in_step: (1.0 / s) as f32,
            in_quiet: usize::MAX / 2,
            block_in: [0.0; BLOCK_SIZE],
            block_l: [0.0; BLOCK_SIZE],
            block_r: [0.0; BLOCK_SIZE],
            block_fill: 0,
            fifo_l: vec![0.0; capacity],
            fifo_r: vec![0.0; capacity],
            fifo_head: 0,
            fifo_len: 0,
            out_history_l: History::new(out_kernel.taps),
            out_history_r: History::new(out_kernel.taps),
            out_kernel,
            out_frac: 0.0,
            out_step: s as f32,
            priming,
        };
        a.fifo_len = a.priming;
        a
    }

    /// Latency of the audio path, in host samples.
    pub fn latency(&self) -> f32 {
        if self.direct {
            self.priming as f32
        } else {
            let half_in = (self.in_kernel.taps / 2) as f32;
            let half_out = (self.out_kernel.taps / 2) as f32;
            half_in + (half_out + 1.0 + self.priming as f32) / self.out_step
        }
    }

    #[inline]
    fn push_internal(&mut self, x: f32, render: &mut impl FnMut(&[f32; BLOCK_SIZE], &mut [f32; BLOCK_SIZE], &mut [f32; BLOCK_SIZE])) {
        self.block_in[self.block_fill] = x;
        self.block_fill += 1;
        if self.block_fill == BLOCK_SIZE {
            self.block_fill = 0;
            render(&self.block_in, &mut self.block_l, &mut self.block_r);
            let cap = self.fifo_l.len();
            for i in 0..BLOCK_SIZE {
                if self.fifo_len == cap {
                    // Cannot happen with the priming above; drop the oldest.
                    self.fifo_head = (self.fifo_head + 1) % cap;
                    self.fifo_len -= 1;
                }
                let w = (self.fifo_head + self.fifo_len) % cap;
                self.fifo_l[w] = self.block_l[i];
                self.fifo_r[w] = self.block_r[i];
                self.fifo_len += 1;
            }
        }
    }

    #[inline]
    fn pop_internal(&mut self) -> (f32, f32) {
        if self.fifo_len == 0 {
            return (0.0, 0.0);
        }
        let v = (self.fifo_l[self.fifo_head], self.fifo_r[self.fifo_head]);
        self.fifo_head = (self.fifo_head + 1) % self.fifo_l.len();
        self.fifo_len -= 1;
        v
    }

    /// Runs `render` on 48 kHz blocks as host samples come and go.
    pub fn process(
        &mut self,
        input: Option<&[f32]>,
        left: &mut [f32],
        right: &mut [f32],
        mut render: impl FnMut(&[f32; BLOCK_SIZE], &mut [f32; BLOCK_SIZE], &mut [f32; BLOCK_SIZE]),
    ) {
        let n = left.len().min(right.len());
        for i in 0..n {
            let x = input.and_then(|s| s.get(i).copied()).unwrap_or(0.0);
            let x = if x.is_finite() { x.clamp(-64.0, 64.0) } else { 0.0 };
            if self.direct {
                self.push_internal(x, &mut render);
                let (l, r) = self.pop_internal();
                left[i] = l;
                right[i] = r;
                continue;
            }

            // Host to 48 kHz.
            self.in_history.push(x);
            self.in_quiet = if x != 0.0 { 0 } else { self.in_quiet.saturating_add(1) };
            self.in_pos -= 1.0;
            while self.in_pos < 1.0 {
                let y = if self.in_quiet > self.in_kernel.taps {
                    0.0
                } else {
                    self.in_kernel.at(self.in_history.window(), self.in_pos)
                };
                self.push_internal(y, &mut render);
                self.in_pos += self.in_step;
            }

            // 48 kHz to host.
            left[i] = self.out_kernel.at(self.out_history_l.window(), self.out_frac);
            right[i] = self.out_kernel.at(self.out_history_r.window(), self.out_frac);
            self.out_frac += self.out_step;
            while self.out_frac >= 1.0 {
                self.out_frac -= 1.0;
                let (l, r) = self.pop_internal();
                self.out_history_l.push(l);
                self.out_history_r.push(r);
            }
        }
    }
}

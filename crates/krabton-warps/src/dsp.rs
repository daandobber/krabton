//! The pieces of stmlib Warps uses (`stmlib/dsp/dsp.h`, `filter.h`,
//! `parameter_interpolator.h`), and its polyphase sample rate converters
//! (`warps/dsp/sample_rate_converter.h`).
//!
//! Copyright 2014 Emilie Gillet, MIT license (see the crate's `lib.rs`).

/// A linear interpolation into `table` at `index * size`.
#[inline]
pub fn interpolate(table: &[f32], index: f32, size: f32) -> f32 {
    let index = (index * size).clamp(0.0, (table.len() - 2) as f32);
    let i = index as usize;
    let f = index - i as f32;
    table[i] + (table[i + 1] - table[i]) * f
}

/// The same, around the middle element `center` (for tables of a bipolar input).
#[inline]
pub fn interpolate_centered(table: &[f32], center: usize, index: f32, size: f32) -> f32 {
    let pos = (center as f32 + index * size).clamp(0.0, (table.len() - 2) as f32);
    let i = pos as usize;
    let f = pos - i as f32;
    table[i] + (table[i + 1] - table[i]) * f
}

#[inline]
pub fn soft_limit(x: f32) -> f32 {
    x * (27.0 + x * x) / (27.0 + 9.0 * x * x)
}

#[inline]
pub fn soft_clip(x: f32) -> f32 {
    if x < -3.0 {
        -1.0
    } else if x > 3.0 {
        1.0
    } else {
        soft_limit(x)
    }
}

#[inline]
pub fn clip16(x: i32) -> i16 {
    x.clamp(-32768, 32767) as i16
}

#[inline]
pub fn semitones_to_ratio(semitones: f32) -> f32 {
    2f32.powf(semitones / 12.0)
}

/// stmlib's ParameterInterpolator: from the stored value to a new one over
/// a block (the first step already moved), the stored value updated.
pub struct Interp {
    value: f32,
    increment: f32,
}

impl Interp {
    pub fn new(state: &mut f32, new_value: f32, size: usize) -> Self {
        let value = *state;
        let increment = (new_value - value) / size.max(1) as f32;
        *state = new_value;
        Self { value, increment }
    }

    #[inline]
    pub fn next(&mut self) -> f32 {
        self.value += self.increment;
        self.value
    }
}

/// stmlib's Svf (low pass only here), with an accurate frequency.
#[derive(Clone, Copy, Default)]
pub struct Svf {
    g: f32,
    r: f32,
    h: f32,
    state_1: f32,
    state_2: f32,
}

impl Svf {
    pub fn set_f_q(&mut self, f: f32, resonance: f32) {
        self.g = (std::f32::consts::PI * f.min(0.497)).tan();
        self.r = 1.0 / resonance;
        self.h = 1.0 / (1.0 + self.r * self.g + self.g * self.g);
    }

    pub fn low_pass(&mut self, buf: &mut [f32]) {
        for x in buf.iter_mut() {
            let hp = (*x - self.r * self.state_1 - self.g * self.state_1 - self.state_2) * self.h;
            let bp = self.g * hp + self.state_1;
            self.state_1 = self.g * hp + bp;
            let lp = self.g * bp + self.state_2;
            self.state_2 = self.g * bp + lp;
            *x = lp;
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CrossMode {
    LowPass,
    BandPassNormalized,
    HighPass,
}

/// stmlib's CrossoverSvf: two cascaded passes of a state-variable filter.
#[derive(Clone, Copy, Default)]
pub struct CrossoverSvf {
    f: f32,
    fq: f32,
    lp: [f32; 2],
    bp: [f32; 2],
    x: [f32; 2],
}

impl CrossoverSvf {
    pub fn set_f_fq(&mut self, f: f32, fq: f32) {
        self.f = f;
        self.fq = fq;
    }

    /// One pass (0 or 1) over `buf`, in place.
    pub fn process(&mut self, mode: CrossMode, pass: usize, input: &[f32], out: &mut [f32]) {
        let (f, fq) = (self.f, self.fq);
        let (mut lp, mut bp, mut x) = (self.lp[pass], self.bp[pass], self.x[pass]);
        for (o, &i) in out.iter_mut().zip(input.iter()) {
            lp += f * bp;
            bp += -fq * bp - f * lp + i;
            if mode == CrossMode::BandPassNormalized {
                bp += x;
            }
            x = i;
            *o = match mode {
                CrossMode::LowPass => lp * f,
                CrossMode::BandPassNormalized => bp * fq,
                CrossMode::HighPass => x - lp * f - bp * fq,
            };
        }
        self.lp[pass] = lp;
        self.bp[pass] = bp;
        self.x[pass] = x;
    }
}

/// The symmetric kernel from its first half.
fn full_kernel(half: &[f32]) -> Vec<f32> {
    let mut h = half.to_vec();
    h.extend(half.iter().rev());
    h
}

/// Upsampling by `ratio` through a polyphase FIR.
pub struct Up {
    ratio: usize,
    h: Vec<f32>,
    /// The last N inputs, newest first.
    x: Vec<f32>,
}

impl Up {
    pub fn new(ratio: usize, half: &[f32]) -> Self {
        let h = full_kernel(half);
        let n = h.len() / ratio;
        Self { ratio, h, x: vec![0.0; n] }
    }

    pub fn delay(&self) -> usize {
        self.h.len() / self.ratio / 2
    }

    pub fn process(&mut self, input: &[f32], out: &mut [f32]) {
        let n = self.x.len();
        let mut o = 0;
        for &v in input {
            self.x.copy_within(0..n - 1, 1);
            self.x[0] = v;
            for j in 0..self.ratio {
                let mut acc = 0.0;
                for i in 0..n {
                    acc += self.x[i] * self.h[j + i * self.ratio];
                }
                out[o] = acc;
                o += 1;
            }
        }
    }
}

/// Downsampling by `ratio` through an FIR (the input a multiple of `ratio`).
pub struct Down {
    ratio: usize,
    h: Vec<f32>,
    hist: Vec<f32>,
    pos: usize,
}

impl Down {
    pub fn new(ratio: usize, half: &[f32]) -> Self {
        let h = full_kernel(half);
        let n = h.len();
        Self { ratio, h, hist: vec![0.0; 2 * n], pos: 0 }
    }

    pub fn delay(&self) -> usize {
        self.h.len() / 2
    }

    pub fn process(&mut self, input: &[f32], out: &mut [f32]) {
        let n = self.h.len();
        let mut o = 0;
        for chunk in input.chunks_exact(self.ratio) {
            for &v in chunk {
                // (twice, so the window never wraps)
                self.hist[self.pos] = v;
                self.hist[self.pos + n] = v;
                self.pos = (self.pos + 1) % n;
            }
            // Newest first: positions pos-1, pos-2, ...
            let base = self.pos + n - 1;
            let mut acc = 0.0;
            for i in 0..n {
                acc += self.hist[base - i] * self.h[i];
            }
            out[o] = acc;
            o += 1;
        }
    }
}

/// stmlib's random words (a linear congruential generator).
pub struct Random(u32);

impl Random {
    pub fn new() -> Self {
        Self(0x21)
    }

    pub fn word(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.0
    }
}

impl Default for Random {
    fn default() -> Self {
        Self::new()
    }
}

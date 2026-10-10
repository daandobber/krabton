//! Real FFT, after Laurent de Soras' FFTReal (`stmlib/fft/shy_fft.h`, with
//! the `RotationPhasor` used by Clouds).
//!
//! For a block of N real samples the spectrum is stored as N floats: the
//! real parts of bins 0..=N/2 in `[0..=N/2]`, the imaginary parts of bins
//! 1..N/2 in `[N/2 + 1..N]`. The inverse transform is not normalized (it
//! returns N times the input).
//!
//! Copyright 2014 Emilie Gillet, MIT license. Rust port for Krabton.

const SQRT_2_DIV_2: f32 = 0.707_106_77;

/// Roots of unity by successive rotations, as in the module.
#[derive(Clone, Copy, Default)]
struct RotationPhasor {
    cos: f32,
    sin: f32,
    real: f32,
    imag: f32,
}

impl RotationPhasor {
    #[inline]
    fn start(&mut self, lut: &[f32], pass: usize) {
        let index = (pass - 3) << 1;
        self.cos = lut[index];
        self.real = lut[index];
        self.sin = lut[index + 1];
        self.imag = lut[index + 1];
    }

    #[inline]
    fn rotate(&mut self) {
        let temp = self.cos * self.real - self.sin * self.imag;
        self.sin = self.cos * self.imag + self.sin * self.real;
        self.cos = temp;
    }
}

pub struct ShyFft {
    num_passes: usize,
    size: usize,
    sin_cos_lut: [f32; 32],
}

impl ShyFft {
    /// `size` must be a power of two between 8 and 65536.
    pub fn new(size: usize) -> Self {
        let num_passes = size.trailing_zeros() as usize;
        assert!(size.is_power_of_two() && (3..=16).contains(&num_passes));
        let mut sin_cos_lut = [0.0; 32];
        for pass in 3..num_passes {
            let index = (pass - 3) << 1;
            let angle = std::f32::consts::PI / (1u32 << pass) as f32;
            sin_cos_lut[index] = angle.cos();
            sin_cos_lut[index + 1] = angle.sin();
        }
        Self { num_passes, size, sin_cos_lut }
    }

    pub fn size(&self) -> usize {
        self.size
    }

    #[inline]
    fn bit_reverse(&self, i: usize) -> usize {
        let lo = (i & 0xff) as u8;
        let hi = (i >> 8) as u8;
        (((lo.reverse_bits() as usize) << 8) | hi.reverse_bits() as usize) >> (16 - self.num_passes)
    }

    /// Forward transform; `input` is used as a workspace and lost.
    pub fn direct(&self, input: &mut [f32], output: &mut [f32]) {
        let size = self.size;
        let input = &mut input[..size];
        let output = &mut output[..size];
        let quarter = size >> 2;

        // First and second pass.
        for i in (0..size).step_by(4) {
            let r0 = self.bit_reverse(i);
            let (r1, r2, r3) = (r0 + 2 * quarter, r0 + quarter, r0 + 3 * quarter);
            let s = &*input;
            let d = &mut output[i..i + 4];
            d[1] = s[r0] - s[r1];
            d[3] = s[r2] - s[r3];
            let a = s[r0] + s[r1];
            let b = s[r2] + s[r3];
            d[0] = a + b;
            d[2] = a - b;
        }

        // Third pass.
        {
            let s = &*output;
            let d = &mut *input;
            for i in (0..size).step_by(8) {
                d[i] = s[i] + s[i + 4];
                d[i + 4] = s[i] - s[i + 4];
                d[i + 2] = s[i + 2];
                d[i + 6] = s[i + 6];

                let v = (s[i + 5] - s[i + 7]) * SQRT_2_DIV_2;
                d[i + 1] = s[i + 1] + v;
                d[i + 3] = s[i + 1] - v;

                let v = (s[i + 5] + s[i + 7]) * SQRT_2_DIV_2;
                d[i + 5] = v + s[i + 3];
                d[i + 7] = v - s[i + 3];
            }
        }

        // Remaining passes, flipping source and destination each time.
        let mut d_is_output = false;
        let mut phasor = RotationPhasor::default();
        for pass in 3..self.num_passes {
            d_is_output = !d_is_output;
            let (s, d): (&[f32], &mut [f32]) = if d_is_output { (&*input, &mut *output) } else { (&*output, &mut *input) };
            let n = 1usize << pass;
            let n_2 = n >> 1;
            for i in (0..size).step_by(n << 1) {
                let s1r = i;
                let s2r = s1r + n;
                let dr = i;
                let di = dr + n;
                d[dr] = s[s1r] + s[s2r];
                d[di] = s[s1r] - s[s2r];
                d[dr + n_2] = s[s1r + n_2];
                d[di + n_2] = s[s2r + n_2];
                let s1i = s1r + n_2;
                let s2i = s1i + n;
                phasor.start(&self.sin_cos_lut, pass);
                for j in 1..n_2 {
                    let c = phasor.cos;
                    let sn = phasor.sin;

                    let v = s[s2r + j] * c - s[s2i + j] * sn;
                    d[dr + j] = s[s1r + j] + v;
                    d[di - j] = s[s1r + j] - v;

                    let v = s[s2r + j] * sn + s[s2i + j] * c;
                    d[di + j] = v + s[s1i + j];
                    d[di + n - j] = v - s[s1i + j];
                    phasor.rotate();
                }
            }
        }

        if !d_is_output {
            output.copy_from_slice(input);
        }
    }

    /// Inverse transform (scaled by N); `input` is used as a workspace and
    /// lost.
    pub fn inverse(&self, input: &mut [f32], output: &mut [f32]) {
        let size = self.size;
        let input = &mut input[..size];
        let output = &mut output[..size];
        let quarter = size >> 2;

        // Remaining passes, from the last one down.
        let mut s_is_input = true;
        let mut phasor = RotationPhasor::default();
        for pass in (3..self.num_passes).rev() {
            let (s, d): (&[f32], &mut [f32]) = if s_is_input { (&*input, &mut *output) } else { (&*output, &mut *input) };
            let n = 1usize << pass;
            let n_2 = n >> 1;
            for i in (0..size).step_by(n << 1) {
                let sr = i;
                let si = sr + n;
                let d1r = i;
                let d2r = d1r + n;
                d[d1r] = s[sr] + s[si];
                d[d2r] = s[sr] - s[si];
                d[d1r + n_2] = s[sr + n_2] * 2.0;
                d[d2r + n_2] = s[si + n_2] * 2.0;
                let d1i = d1r + n_2;
                let d2i = d1i + n;
                phasor.start(&self.sin_cos_lut, pass);
                for j in 1..n_2 {
                    d[d1r + j] = s[sr + j] + s[si - j];
                    d[d1i + j] = s[si + j] - s[si + n - j];

                    let c = phasor.cos;
                    let sn = phasor.sin;
                    let vr = s[sr + j] - s[si - j];
                    let vi = s[si + j] + s[si + n - j];

                    d[d2r + j] = vr * c + vi * sn;
                    d[d2i + j] = vi * c - vr * sn;
                    phasor.rotate();
                }
            }
            s_is_input = !s_is_input;
        }

        // The data must be in `output` for the third pass.
        if s_is_input {
            output.copy_from_slice(input);
        }

        {
            let s = &*output;
            let d = &mut *input;
            for i in (0..size).step_by(8) {
                d[i] = s[i] + s[i + 4];
                d[i + 4] = s[i] - s[i + 4];
                d[i + 2] = s[i + 2] * 2.0;
                d[i + 6] = s[i + 6] * 2.0;
                d[i + 1] = s[i + 1] + s[i + 3];
                d[i + 3] = s[i + 5] - s[i + 7];
                let vr = s[i + 1] - s[i + 3];
                let vi = s[i + 5] + s[i + 7];
                d[i + 5] = (vr + vi) * SQRT_2_DIV_2;
                d[i + 7] = (vi - vr) * SQRT_2_DIV_2;
            }
        }

        // First and second pass.
        for i in (0..size).step_by(4) {
            let r0 = self.bit_reverse(i);
            let (r1, r2, r3) = (r0 + 2 * quarter, r0 + quarter, r0 + 3 * quarter);
            let s = &input[i..i + 4];
            let b_0 = s[0] + s[2];
            let b_2 = s[0] - s[2];
            let b_1 = s[1] * 2.0;
            let b_3 = s[3] * 2.0;
            output[r0] = b_0 + b_1;
            output[r1] = b_0 - b_1;
            output[r2] = b_2 + b_3;
            output[r3] = b_2 - b_3;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_a_dft_and_inverts() {
        let n = 64;
        let fft = ShyFft::new(n);
        let x: Vec<f32> = (0..n).map(|i| ((i * 7 + 3) % 11) as f32 - 5.0 + (i as f32 * 0.37).sin()).collect();
        let mut input = x.clone();
        let mut spectrum = vec![0.0; n];
        fft.direct(&mut input, &mut spectrum);
        for k in 0..=n / 2 {
            let (mut re, mut im) = (0.0f64, 0.0f64);
            for (t, &v) in x.iter().enumerate() {
                let a = -2.0 * std::f64::consts::PI * (k * t) as f64 / n as f64;
                re += v as f64 * a.cos();
                im += v as f64 * a.sin();
            }
            assert!((spectrum[k] as f64 - re).abs() < 1e-3, "re {k}");
            if k > 0 && k < n / 2 {
                // FFTReal stores the negated imaginary part.
                assert!((spectrum[n / 2 + k] as f64 + im).abs() < 1e-3, "im {k}");
            }
        }
        let mut back = vec![0.0; n];
        fft.inverse(&mut spectrum, &mut back);
        for (a, b) in x.iter().zip(&back) {
            assert!((a * n as f32 - b).abs() < 1e-2);
        }
    }
}

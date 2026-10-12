//! The recording buffer (`beads/dsp/delay_line.h`): 16-bit, 12-bit or
//! 8-bit µ-law samples, mono or stereo, written backwards so that a delay
//! of d reads the sample written d steps ago; a short tail of recent
//! input is kept while frozen and crossfaded back in when the freeze ends.
//!
//! The module packs 12-bit samples in bitfields and shares one memory
//! between its quality settings; here a single buffer of 16-bit words is
//! used for every setting, holding the same values.
//!
//! Copyright 2017 Emilie Gillet, MIT license. Rust port for Krabton.

use crate::util::{clip16, integral_fractional, linear_to_mu_law, mu_law_to_linear, smooth_step};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Resolution {
    Bit8,
    Bit12,
    Bit16,
}

const INTERPOLATION_TAIL: usize = 4;
pub const FREEZE_TAIL_SIZE: usize = 256;

pub struct DelayLine {
    buf: Vec<i16>,
    resolution: Resolution,
    channels: usize,
    max_delay: usize,
    freeze_tail: Vec<[f32; 2]>,
    write_ptr: i32,
    tail_write_ptr: i32,
    tail_xfade_duration: i32,
    written_since_last_frozen: i32,
    duration: i32,
    frozen: bool,
}

impl DelayLine {
    /// A buffer for up to `capacity` samples (frames times channels).
    pub fn new(capacity: usize) -> Self {
        Self {
            buf: vec![0; capacity + 2 * INTERPOLATION_TAIL],
            resolution: Resolution::Bit16,
            channels: 2,
            max_delay: 4,
            freeze_tail: vec![[0.0; 2]; FREEZE_TAIL_SIZE],
            write_ptr: 0,
            tail_write_ptr: 0,
            tail_xfade_duration: 0,
            written_since_last_frozen: 0,
            duration: 0,
            frozen: false,
        }
    }

    /// Sets the format and length (in frames), and clears it.
    pub fn configure(&mut self, resolution: Resolution, channels: usize, max_delay: usize) {
        let need = (max_delay + INTERPOLATION_TAIL) * channels;
        assert!(need <= self.buf.len(), "delay line too small");
        self.resolution = resolution;
        self.channels = channels;
        self.max_delay = max_delay;
        self.reset();
    }

    pub fn reset(&mut self) {
        self.write_ptr = 0;
        self.tail_write_ptr = 0;
        self.tail_xfade_duration = 0;
        self.frozen = false;
        self.written_since_last_frozen = 0;
        self.duration = self.max_delay as i32;
        // Silence in every format (µ-law's zero is 255).
        let zero = if self.resolution == Resolution::Bit8 { 255 } else { 0 };
        self.buf.fill(zero);
        self.freeze_tail.fill([0.0; 2]);
    }

    pub fn max_delay(&self) -> i32 {
        self.max_delay as i32
    }

    pub fn channels(&self) -> usize {
        self.channels
    }

    pub fn frozen(&self) -> bool {
        self.frozen
    }

    pub fn duration(&self) -> i32 {
        self.duration
    }

    pub fn remaining(&self) -> i32 {
        self.max_delay as i32 - self.written_since_last_frozen
    }

    pub fn trim(&mut self) {
        self.tail_write_ptr = 0;
        self.duration = 0;
    }

    #[inline]
    fn store(&mut self, i: usize, s: [f32; 2]) {
        let c = self.channels;
        let base = i * c;
        if c == 1 {
            let lr = clip16(((s[0] + s[1]) * 16384.0) as i32) as i16;
            self.buf[base] = match self.resolution {
                Resolution::Bit16 => lr,
                Resolution::Bit12 => lr >> 4,
                Resolution::Bit8 => linear_to_mu_law(lr) as i16,
            };
        } else {
            for (k, x) in s.iter().enumerate() {
                let v = clip16((x * 32768.0) as i32) as i16;
                self.buf[base + k] = match self.resolution {
                    Resolution::Bit16 => v,
                    Resolution::Bit12 => v >> 4,
                    Resolution::Bit8 => linear_to_mu_law(v) as i16,
                };
            }
        }
    }

    #[inline]
    fn decode(&self, v: i16) -> f32 {
        match self.resolution {
            Resolution::Bit16 => v as f32 / 32768.0,
            Resolution::Bit12 => v as f32 / 2048.0,
            Resolution::Bit8 => mu_law_to_linear(v as u8),
        }
    }

    #[inline]
    fn load(&self, i: usize) -> [f32; 2] {
        let c = self.channels;
        if c == 1 {
            let v = self.decode(self.buf[i]);
            [v, v]
        } else {
            [self.decode(self.buf[i * 2]), self.decode(self.buf[i * 2 + 1])]
        }
    }

    #[inline]
    pub fn write(&mut self, s: [f32; 2]) {
        let wp = self.write_ptr as usize;
        self.store(wp, s);
        if wp < INTERPOLATION_TAIL {
            self.store(wp + self.max_delay, s);
        }
        if self.write_ptr == 0 {
            self.write_ptr = self.max_delay as i32 - 1;
        } else {
            self.write_ptr -= 1;
        }
        self.written_since_last_frozen += 1;
        if self.written_since_last_frozen >= self.max_delay as i32 {
            self.written_since_last_frozen = 0;
        }
        self.duration += 1;
        if self.duration >= self.max_delay as i32 {
            self.duration = self.max_delay as i32;
        }
    }

    /// Writes, unless frozen: then the input goes to the tail, to be faded
    /// back in when the freeze ends.
    pub fn write_frozen(&mut self, mut s: [f32; 2], frozen: bool) {
        if frozen {
            if !self.frozen {
                self.tail_write_ptr = 0;
            }
            if (self.tail_write_ptr as usize) < FREEZE_TAIL_SIZE {
                self.freeze_tail[self.tail_write_ptr as usize] = s;
                self.tail_write_ptr += 1;
            }
            self.written_since_last_frozen = 0;
        } else {
            if self.frozen && self.tail_write_ptr != 0 {
                self.tail_xfade_duration = self.tail_write_ptr;
            }
            if self.tail_write_ptr != 0 {
                let pos = (self.tail_xfade_duration - self.tail_write_ptr).clamp(0, FREEZE_TAIL_SIZE as i32 - 1) as usize;
                let t = self.tail_write_ptr as f32 / self.tail_xfade_duration.max(1) as f32;
                let gain = smooth_step(t);
                s[0] += (self.freeze_tail[pos][0] - s[0]) * gain;
                s[1] += (self.freeze_tail[pos][1] - s[1]) * gain;
                self.tail_write_ptr -= 1;
            }
            self.write(s);
        }
        self.frozen = frozen;
    }

    /// Hermite-interpolated read, `delay_fractional` towards older samples.
    #[inline]
    pub fn read_hermite(&self, delay_integral: i32, delay_fractional: f32) -> [f32; 2] {
        let m = self.max_delay as i64;
        let d = delay_integral as i64 + m - 1;
        let i = ((self.write_ptr as i64 + d).rem_euclid(m)) as usize;
        let x0 = self.load(i);
        let x1 = self.load(i + 1);
        let x2 = self.load(i + 2);
        let x3 = self.load(i + 3);
        let t = delay_fractional;
        let mut out = [0.0f32; 2];
        for k in 0..2 {
            let c = (x2[k] - x0[k]) * 0.5;
            let v = x1[k] - x2[k];
            let w = c + v;
            let a = w + v + (x3[k] - x1[k]) * 0.5;
            let b_neg = w + a;
            out[k] = (((a * t) - b_neg) * t + c) * t + x1[k];
        }
        out
    }

    /// The same with a 22.10 fixed-point delay.
    #[inline]
    pub fn read_hermite_fixed(&self, delay: i32) -> [f32; 2] {
        self.read_hermite(delay >> 10, (delay & 1023) as f32 / 1024.0)
    }

    #[inline]
    pub fn read_hermite_float(&self, delay: f32) -> [f32; 2] {
        let (i, f) = integral_fractional(delay);
        self.read_hermite(i, f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_delay_reads_what_was_written_that_long_ago() {
        for (res, ch) in [(Resolution::Bit16, 2), (Resolution::Bit12, 2), (Resolution::Bit16, 1), (Resolution::Bit8, 2)] {
            let mut d = DelayLine::new(4096);
            d.configure(res, ch, 1000);
            for i in 0..600 {
                let v = (i as f32 * 0.01).sin() * 0.5;
                d.write([v, v]);
            }
            // Delay 1 is the last sample written.
            let last = (599.0f32 * 0.01).sin() * 0.5;
            let got = d.read_hermite(1, 0.0);
            assert!((got[0] - last).abs() < 0.02, "{res:?} {ch}: {} vs {last}", got[0]);
            let older = (500.0f32 * 0.01).sin() * 0.5;
            let got = d.read_hermite(100, 0.0);
            assert!((got[0] - older).abs() < 0.02, "{res:?} {ch}: {} vs {older}", got[0]);
        }
    }
}

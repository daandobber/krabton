//! The recording buffer: a circular buffer of 16-bit or 8-bit mu-law
//! samples with a short tail for the interpolators, and a crossfade when
//! recording resumes after a freeze (`clouds/dsp/audio_buffer.h`,
//! `clouds/dsp/mu_law.h`, `clouds/dsp/mu_law.cc`).
//!
//! Copyright 2014 Emilie Gillet, MIT license. Rust port for Krabton.

use crate::dsp::clip16;
use crate::resources::LUT_ULAW;

pub const CROSS_FADE_SIZE: i32 = 256;
pub const INTERPOLATION_TAIL: i32 = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Interpolation {
    Zoh,
    Linear,
    Hermite,
}

#[inline]
pub fn mu_law_to_lin(u_val: u8) -> i16 {
    LUT_ULAW[u_val as usize]
}

#[inline]
pub fn lin_to_mu_law(pcm_val: i16) -> u8 {
    let mut pcm_val = pcm_val >> 2;
    let mask: u8;
    if pcm_val < 0 {
        pcm_val = -pcm_val;
        mask = 0x7f;
    } else {
        mask = 0xff;
    }
    if pcm_val > 8159 {
        pcm_val = 8159;
    }
    pcm_val += 0x84 >> 2;
    let seg: i16 = if pcm_val <= 0x3f {
        0
    } else if pcm_val <= 0x7f {
        1
    } else if pcm_val <= 0xff {
        2
    } else if pcm_val <= 0x1ff {
        3
    } else if pcm_val <= 0x3ff {
        4
    } else if pcm_val <= 0x7ff {
        5
    } else if pcm_val <= 0xfff {
        6
    } else if pcm_val <= 0x1fff {
        7
    } else {
        8
    };
    if seg >= 8 {
        0x7f ^ mask
    } else {
        let uval = ((seg << 4) | ((pcm_val >> (seg + 1)) & 0x0f)) as u8;
        uval ^ mask
    }
}

/// How samples are stored in the recording buffer.
pub trait Resolution: Copy + Default + Send + 'static {
    /// The value of an empty buffer.
    const SILENCE: Self;
    /// Multiplies the raw values to get floats.
    const SCALE: f32;
    const IS_16_BIT: bool;
    fn raw(self) -> f32;
    /// `Write(float)`.
    fn encode(value: f32) -> Self;
    /// Used by the fast write path (16-bit only), which scales by 32767.
    fn from_i16(value: i16) -> Self;
}

impl Resolution for i16 {
    const SILENCE: Self = 0;
    const SCALE: f32 = 1.0 / 32768.0;
    const IS_16_BIT: bool = true;
    #[inline]
    fn raw(self) -> f32 {
        self as f32
    }
    #[inline]
    fn encode(value: f32) -> Self {
        clip16((value * 32768.0) as i32) as i16
    }
    #[inline]
    fn from_i16(value: i16) -> Self {
        value
    }
}

/// An 8-bit mu-law sample.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MuLaw(pub u8);

impl Resolution for MuLaw {
    const SILENCE: Self = MuLaw(127);
    const SCALE: f32 = 1.0 / 32768.0;
    const IS_16_BIT: bool = false;
    #[inline]
    fn raw(self) -> f32 {
        mu_law_to_lin(self.0) as f32
    }
    #[inline]
    fn encode(value: f32) -> Self {
        MuLaw(lin_to_mu_law(clip16((value * 32768.0) as i32) as i16))
    }
    #[inline]
    fn from_i16(value: i16) -> Self {
        MuLaw(lin_to_mu_law(value))
    }
}

pub struct AudioBuffer<R: Resolution> {
    /// Preallocated memory; only the first `size + INTERPOLATION_TAIL`
    /// samples are used.
    data: Box<[R]>,
    size: i32,
    write_head: i32,
    /// Recording continues here while frozen, to crossfade with when
    /// recording resumes. One extra slot: the C++ code reads one sample past
    /// the end (with a gain of zero).
    tail: [i16; CROSS_FADE_SIZE as usize + 1],
    crossfade_counter: i32,
}

impl<R: Resolution> AudioBuffer<R> {
    /// Allocates room for up to `capacity` samples (tail included).
    pub fn new(capacity: usize) -> Self {
        let capacity = capacity.max(INTERPOLATION_TAIL as usize + 16);
        Self {
            data: vec![R::SILENCE; capacity].into_boxed_slice(),
            size: capacity as i32 - INTERPOLATION_TAIL,
            write_head: 0,
            tail: [0; CROSS_FADE_SIZE as usize + 1],
            crossfade_counter: 0,
        }
    }

    /// `Init`: uses `size` samples of the memory (tail included) and clears
    /// them.
    pub fn init(&mut self, size: usize) {
        let size = size.clamp(INTERPOLATION_TAIL as usize + 16, self.data.len());
        self.size = size as i32 - INTERPOLATION_TAIL;
        self.write_head = 0;
        self.crossfade_counter = 0;
        self.data[..size].fill(R::SILENCE);
    }

    #[inline]
    pub fn size(&self) -> i32 {
        self.size
    }

    #[inline]
    pub fn head(&self) -> i32 {
        self.write_head
    }

    #[inline]
    fn write(&mut self, value: f32) {
        let head = self.write_head as usize;
        self.data[head] = R::encode(value);
        if self.write_head < INTERPOLATION_TAIL {
            self.data[head + self.size as usize] = self.data[head];
        }
        self.write_head += 1;
        if self.write_head >= self.size {
            self.write_head = 0;
        }
    }

    /// Records `frames[..][channel]`, or keeps a tail for the crossfade when
    /// `write` is false (frozen).
    pub fn write_fade(&mut self, frames: &[[f32; 2]], channel: usize, write: bool) {
        let size = frames.len() as i32;
        if !write {
            if self.crossfade_counter < CROSS_FADE_SIZE {
                for frame in frames {
                    if self.crossfade_counter < CROSS_FADE_SIZE {
                        self.tail[self.crossfade_counter as usize] = clip16((frame[channel] * 32767.0) as i32) as i16;
                        self.crossfade_counter += 1;
                    }
                }
            }
        } else if self.crossfade_counter == 0
            && R::IS_16_BIT
            && self.write_head >= INTERPOLATION_TAIL
            && self.write_head < self.size - size
        {
            // Fast write routine for the most common case.
            for frame in frames {
                self.data[self.write_head as usize] = R::from_i16(clip16((frame[channel] * 32767.0) as i32) as i16);
                self.write_head += 1;
            }
        } else {
            for frame in frames {
                let mut sample = frame[channel];
                if self.crossfade_counter > 0 {
                    self.crossfade_counter -= 1;
                    let tail_sample = self.tail[(CROSS_FADE_SIZE - self.crossfade_counter) as usize] as f32;
                    let gain = self.crossfade_counter as f32 * (1.0 / CROSS_FADE_SIZE as f32);
                    sample += (tail_sample / 32768.0 - sample) * gain;
                }
                self.write(sample);
            }
        }
    }

    /// Brings an index into the buffer. The module wraps once; the second
    /// wrap only guards against reads the C++ code would make out of bounds.
    #[inline]
    fn wrap(&self, integral: i32) -> usize {
        let mut i = integral;
        if i >= self.size {
            i -= self.size;
        }
        if i < 0 || i >= self.size {
            i = i.rem_euclid(self.size);
        }
        i as usize
    }

    #[inline]
    pub fn read(&self, method: Interpolation, integral: i32, fractional: u16) -> f32 {
        match method {
            Interpolation::Zoh => self.read_zoh(integral),
            Interpolation::Linear => self.read_linear(integral, fractional),
            Interpolation::Hermite => self.read_hermite(integral, fractional),
        }
    }

    #[inline]
    pub fn read_zoh(&self, integral: i32) -> f32 {
        let i = self.wrap(integral);
        self.data[i].raw() * R::SCALE
    }

    #[inline]
    pub fn read_linear(&self, integral: i32, fractional: u16) -> f32 {
        let i = self.wrap(integral);
        let t = fractional as f32 / 65536.0;
        let x0 = self.data[i].raw();
        let x1 = self.data[i + 1].raw();
        (x0 + (x1 - x0) * t) * R::SCALE
    }

    #[inline]
    pub fn read_hermite(&self, integral: i32, fractional: u16) -> f32 {
        let i = self.wrap(integral);
        let t = fractional as f32 / 65536.0;
        let xm1 = self.data[i].raw();
        let x0 = self.data[i + 1].raw();
        let x1 = self.data[i + 2].raw();
        let x2 = self.data[i + 3].raw();
        // Laurent de Soras's Hermite interpolator.
        let c = (x1 - xm1) * 0.5;
        let v = x0 - x1;
        let w = c + v;
        let a = w + v + (x2 - x0) * 0.5;
        let b_neg = w + a;
        ((((a * t) - b_neg) * t + c) * t + x0) * R::SCALE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mu_law_round_trip() {
        for x in [-32768i32, -20000, -1000, -10, 0, 10, 1000, 20000, 32767] {
            let y = mu_law_to_lin(lin_to_mu_law(x as i16)) as i32;
            let tolerance = (x.abs() / 16).max(16);
            assert!((x - y).abs() <= tolerance, "{x} -> {y}");
        }
        assert_eq!(mu_law_to_lin(MuLaw::SILENCE.0), 0);
    }
}

//! Short-time Fourier transform with overlap-add resynthesis
//! (`clouds/dsp/pvoc/stft.h`, `clouds/dsp/pvoc/stft.cc`).
//!
//! Copyright 2014 Emilie Gillet, MIT license. Rust port for Krabton.

use super::fft::ShyFft;
use super::frame_transformation::FrameTransformation;
use crate::dsp::{clip16, Random};
use crate::parameters::Parameters;
use crate::resources::LUT_SINE_WINDOW_4096;

pub struct Stft {
    fft_size: usize,
    hop_size: usize,
    buffer_size: usize,
    window_stride: usize,
    analysis: Box<[i16]>,
    synthesis: Box<[i16]>,
    buffer_ptr: usize,
    process_ptr: usize,
    block_size: usize,
    ready: usize,
    done: usize,
}

impl Stft {
    /// Allocates for frames of up to `max_fft_size` samples.
    pub fn new(max_fft_size: usize) -> Self {
        let max_buffer = max_fft_size + max_fft_size / 2;
        Self {
            fft_size: max_fft_size,
            hop_size: max_fft_size / 4,
            buffer_size: max_fft_size + max_fft_size / 4,
            window_stride: 1,
            analysis: vec![0; max_buffer].into_boxed_slice(),
            synthesis: vec![0; max_buffer].into_boxed_slice(),
            buffer_ptr: 0,
            process_ptr: 0,
            block_size: 0,
            ready: 0,
            done: 0,
        }
    }

    pub fn init(&mut self, fft_size: usize, hop_size: usize) {
        let fft_size = fft_size.min(self.analysis.len() * 2 / 3).max(8);
        self.fft_size = fft_size;
        self.hop_size = hop_size.clamp(1, fft_size);
        self.buffer_size = self.fft_size + self.hop_size;
        self.window_stride = LUT_SINE_WINDOW_4096.len() / fft_size;
        self.reset();
    }

    pub fn reset(&mut self) {
        self.buffer_ptr = 0;
        self.process_ptr = (2 * self.hop_size) % self.buffer_size;
        self.block_size = 0;
        self.analysis[..self.buffer_size].fill(0);
        self.synthesis[..self.buffer_size].fill(0);
        self.ready = 0;
        self.done = 0;
    }

    /// Writes channel `channel` of `input` into the analysis buffer and
    /// reads the resynthesized signal into the same channel of `output`.
    pub fn process(&mut self, input: &[[f32; 2]], output: &mut [[f32; 2]], channel: usize) {
        let mut offset = 0;
        let mut size = input.len().min(output.len());
        while size > 0 {
            let processed = size.min(self.hop_size - self.block_size);
            for i in 0..processed {
                let sample = (input[offset + i][channel] * 32768.0) as i32;
                self.analysis[self.buffer_ptr + i] = clip16(sample) as i16;
                output[offset + i][channel] = self.synthesis[self.buffer_ptr + i] as f32 / 16384.0;
            }
            offset += processed;
            self.block_size += processed;
            size -= processed;
            self.buffer_ptr += processed;
            if self.buffer_ptr >= self.buffer_size {
                self.buffer_ptr -= self.buffer_size;
            }
            if self.block_size >= self.hop_size {
                self.block_size -= self.hop_size;
                self.ready = self.ready.wrapping_add(1);
            }
        }
    }

    #[inline]
    pub fn pending(&self) -> bool {
        self.ready != self.done
    }

    /// Analyses, transforms and resynthesizes one frame, if one is ready
    /// (done by the module outside of the audio interrupt).
    pub fn buffer(
        &mut self,
        fft: &ShyFft,
        fft_buffer: &mut [f32],
        ifft_buffer: &mut [f32],
        modifier: &mut FrameTransformation,
        parameters: &Parameters,
        rng: &mut Random,
    ) {
        if self.ready == self.done {
            return;
        }
        let n = self.fft_size;

        // Copy block to FFT buffer and apply window.
        let mut source_ptr = self.process_ptr;
        let mut w = 0;
        for x in fft_buffer[..n].iter_mut() {
            *x = LUT_SINE_WINDOW_4096[w] * self.analysis[source_ptr] as f32;
            source_ptr += 1;
            if source_ptr >= self.buffer_size {
                source_ptr -= self.buffer_size;
            }
            w += self.window_stride;
        }

        // Compute FFT. fft_in is lost.
        fft.direct(fft_buffer, ifft_buffer);
        // Process in the frequency domain.
        modifier.process(parameters, ifft_buffer, fft_buffer, rng);
        // Compute IFFT. ifft_in is lost.
        fft.inverse(fft_buffer, ifft_buffer);

        let mut destination_ptr = self.process_ptr;
        let inverse_window_size = 1.0 / ((n * n / self.hop_size) >> 1) as f32;
        let mut w = 0;
        for i in 0..n {
            let s = ifft_buffer[i] * LUT_SINE_WINDOW_4096[w] * inverse_window_size;
            let mut x = s as i32;
            if i < n - self.hop_size {
                // Overlap-add.
                x = x.saturating_add(self.synthesis[destination_ptr] as i32);
            }
            self.synthesis[destination_ptr] = clip16(x) as i16;
            destination_ptr += 1;
            if destination_ptr >= self.buffer_size {
                destination_ptr -= self.buffer_size;
            }
            w += self.window_stride;
        }

        self.done = self.done.wrapping_add(1);
        self.process_ptr += self.hop_size;
        if self.process_ptr >= self.buffer_size {
            self.process_ptr -= self.buffer_size;
        }
    }
}

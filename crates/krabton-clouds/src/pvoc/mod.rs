//! The phase vocoder of the spectral mode (`clouds/dsp/pvoc/*`).
//!
//! Copyright 2014 Emilie Gillet, MIT license. Rust port for Krabton.

pub mod fft;
pub mod frame_transformation;
pub mod stft;

use crate::dsp::Random;
use crate::parameters::Parameters;
use fft::ShyFft;
use frame_transformation::FrameTransformation;
use stft::Stft;

pub const MAX_FFT_SIZE: usize = 4096;
const HOP_RATIO: usize = 4;

pub struct PhaseVocoder {
    fft: ShyFft,
    fft_buffer: Box<[f32]>,
    ifft_buffer: Box<[f32]>,
    stft: [Stft; 2],
    frame_transformation: [FrameTransformation; 2],
    num_channels: usize,
}

impl Default for PhaseVocoder {
    fn default() -> Self {
        Self {
            fft: ShyFft::new(MAX_FFT_SIZE),
            fft_buffer: vec![0.0; MAX_FFT_SIZE].into_boxed_slice(),
            ifft_buffer: vec![0.0; MAX_FFT_SIZE].into_boxed_slice(),
            stft: [Stft::new(MAX_FFT_SIZE), Stft::new(MAX_FFT_SIZE)],
            frame_transformation: [FrameTransformation::new(MAX_FFT_SIZE), FrameTransformation::new(MAX_FFT_SIZE)],
            num_channels: 2,
        }
    }
}

impl PhaseVocoder {
    /// The module sizes the number of spectral "textures" from the memory
    /// left after the FFT and analysis/synthesis buffers: 3 per channel in
    /// stereo, 7 in mono (one of them holds the phases).
    pub fn init(&mut self, num_channels: usize) {
        self.num_channels = num_channels.clamp(1, 2);
        let num_textures = if self.num_channels == 1 { 7 } else { 3 };
        for i in 0..self.num_channels {
            self.stft[i].init(MAX_FFT_SIZE, MAX_FFT_SIZE / HOP_RATIO);
        }
        for i in 0..self.num_channels {
            self.frame_transformation[i].init(MAX_FFT_SIZE, num_textures);
        }
        self.fft_buffer.fill(0.0);
        self.ifft_buffer.fill(0.0);
    }

    pub fn process(&mut self, input: &[[f32; 2]], output: &mut [[f32; 2]]) {
        for i in 0..self.num_channels {
            self.stft[i].process(input, output, i);
        }
    }

    /// Runs the pending frames (the module does this in its main loop).
    pub fn buffer(&mut self, parameters: &Parameters, rng: &mut Random) {
        for i in 0..self.num_channels {
            // One frame per hop; more than one could only pile up if this
            // was not called for a whole hop.
            for _ in 0..4 {
                if !self.stft[i].pending() {
                    break;
                }
                self.stft[i].buffer(
                    &self.fft,
                    &mut self.fft_buffer,
                    &mut self.ifft_buffer,
                    &mut self.frame_transformation[i],
                    parameters,
                    rng,
                );
            }
        }
    }
}

//! The 2x decimator and interpolator of the 16 kHz ("lo-fi") qualities
//! (`clouds/dsp/sample_rate_converter.h`).
//!
//! Copyright 2014 Emilie Gillet, MIT license. Rust port for Krabton.

use crate::resources::SRC_FILTER_1X_2_45;

const FILTER_SIZE: usize = 45;

/// Polyphase FIR rate converter: `ratio` -2 decimates, +2 interpolates.
pub struct SampleRateConverter {
    ratio: i32,
    history: [[f32; 2]; FILTER_SIZE * 2],
    history_ptr: i32,
}

impl SampleRateConverter {
    pub fn new(ratio: i32) -> Self {
        let mut s = Self { ratio, history: [[0.0; 2]; FILTER_SIZE * 2], history_ptr: 0 };
        s.init();
        s
    }

    pub fn init(&mut self) {
        self.history = [[0.0; 2]; FILTER_SIZE * 2];
        self.history_ptr = FILTER_SIZE as i32 - 1;
    }

    pub fn process(&mut self, input: &[[f32; 2]], output: &mut [[f32; 2]]) {
        let ratio = self.ratio;
        let scale = if ratio < 0 { 1.0 } else { ratio as f32 };
        let consumed = if ratio < 0 { (-ratio) as usize } else { 1 };
        let produced = if ratio > 0 { ratio as usize } else { 1 };
        let mut history_ptr = self.history_ptr;
        let mut out_index = 0;
        for chunk in input.chunks_exact(consumed) {
            for &frame in chunk {
                self.history[history_ptr as usize + FILTER_SIZE] = frame;
                self.history[history_ptr as usize] = frame;
                history_ptr -= 1;
                if history_ptr < 0 {
                    history_ptr += FILTER_SIZE as i32;
                }
            }
            for i in 0..produced {
                let (mut y_l, mut y_r) = (0.0, 0.0);
                let mut x = history_ptr as usize + 1;
                let mut j = i;
                while j < FILTER_SIZE {
                    let h = SRC_FILTER_1X_2_45[j];
                    y_l += self.history[x][0] * h;
                    y_r += self.history[x][1] * h;
                    x += 1;
                    j += produced;
                }
                if let Some(out) = output.get_mut(out_index) {
                    *out = [y_l * scale, y_r * scale];
                }
                out_index += 1;
            }
        }
        self.history_ptr = history_ptr;
    }
}

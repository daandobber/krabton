//! Output limiter (`rings/dsp/limiter.h`).
//!
//! Copyright 2015 Emilie Gillet, MIT license. Rust port for Krabton.

use crate::dsp::{slope, soft_limit};

#[derive(Clone, Copy, Debug)]
pub struct Limiter {
    peak: f32,
}

impl Default for Limiter {
    fn default() -> Self {
        Self { peak: 0.5 }
    }
}

impl Limiter {
    pub fn process(&mut self, l: &mut [f32], r: &mut [f32], pre_gain: f32) {
        for (l, r) in l.iter_mut().zip(r.iter_mut()) {
            let l_pre = *l * pre_gain;
            let r_pre = *r * pre_gain;

            let l_peak = l_pre.abs();
            let r_peak = r_pre.abs();
            let s_peak = (r_pre - l_pre).abs();

            let peak = l_peak.max(r_peak).max(s_peak);
            slope(&mut self.peak, peak, 0.05, 0.00002);

            // Clamp to 8Vpp, clipping softly towards 10Vpp.
            let gain = if self.peak <= 1.0 { 1.0 } else { 1.0 / self.peak };
            *l = soft_limit(l_pre * gain * 0.8);
            *r = soft_limit(r_pre * gain * 0.8);
        }
    }
}

//! Density2: density/saturation with a highpass, run on the signal and on a
//! half-sample interpolation of it (`Density2Proc.cpp`).
//!
//! Copyright (c) Chris Johnson (airwindows), MIT license. Rust port for Krabton.

use crate::dsp::{dither32, noise_floor};
use crate::{EffectInfo, Kernel, ParamInfo};

pub(crate) const INFO: EffectInfo = EffectInfo {
    id: "Density2",
    name: "Density2",
    category: "Saturation",
    description: "Adds density and drive when turned up, or thins the sound out when turned below its starting point.",
    params: &[
        ParamInfo { name: "Density", default: 0.2 },
        ParamInfo { name: "Highpass", default: 0.0 },
        ParamInfo { name: "Output", default: 1.0 },
        ParamInfo { name: "Dry/Wet", default: 1.0 },
    ],
};

const ATA: f64 = 0.915965594177219015;

#[derive(Clone, Copy, Default)]
struct Channel {
    last3: f64,
    last2: f64,
    last1: f64,
    ata_a: f64,
    ata_b: f64,
    ata_c: f64,
    last_diff: f64,
    iir_a: f64,
    iir_b: f64,
}

pub(crate) struct Density2 {
    overallscale: f64,
    ch: [Channel; 2],
    fpd_l: u32,
    fpd_r: u32,
}

impl Kernel for Density2 {
    const NUM_PARAMS: usize = 4;

    fn new(sample_rate: f64) -> Self {
        Self { overallscale: sample_rate / 44100.0, ch: [Channel::default(); 2], fpd_l: crate::dsp::seed(), fpd_r: crate::dsp::seed() }
    }

    fn reset(&mut self) {
        self.ch = [Channel::default(); 2];
    }

    fn process(&mut self, p: &[f64], left: &mut [f32], right: &mut [f32]) {
        let mut density = (p[0] * 5.0) - 1.0;
        let mut out = density.abs();
        while out > 1.0 {
            out -= 1.0;
        }
        density *= density.abs();
        let iir_amount = p[1].powf(3.0) / self.overallscale;
        let output = p[2];
        let wet = p[3];

        for (out_l, out_r) in left.iter_mut().zip(right.iter_mut()) {
            let mut input = [noise_floor(*out_l as f64, self.fpd_l), noise_floor(*out_r as f64, self.fpd_r)];
            for c in 0..2 {
                let ch = &mut self.ch[c];
                let dry = input[c];
                let mut x = input[c];

                let mut halfway = (x + ch.last1 + ((-ch.last2 + ch.last3) * 0.0414213562373095048801688)) / 2.0;
                let half_dry = halfway;
                ch.last3 = ch.last2;
                ch.last2 = ch.last1;
                ch.last1 = x;

                ch.iir_b = (ch.iir_b * (1.0 - iir_amount)) + (halfway * iir_amount);
                halfway -= ch.iir_b; //highpass section
                halfway = densify(halfway, density, out);

                ch.ata_c = halfway - half_dry;
                ch.ata_a *= ATA;
                ch.ata_b *= ATA;
                ch.ata_b += ch.ata_c;
                ch.ata_a -= ch.ata_c;
                ch.ata_c = ch.ata_b;
                let half_diff = ch.ata_c * ATA;

                ch.iir_a = (ch.iir_a * (1.0 - iir_amount)) + (x * iir_amount);
                x -= ch.iir_a; //highpass section
                x = densify(x, density, out);

                ch.ata_c = x - dry;
                ch.ata_a *= ATA;
                ch.ata_b *= ATA;
                ch.ata_a += ch.ata_c;
                ch.ata_b -= ch.ata_c;
                ch.ata_c = ch.ata_a;
                let diff = ch.ata_c * ATA;

                x = dry + ((diff + half_diff + ch.last_diff) / 1.187);
                ch.last_diff = diff / 2.0;
                x *= output;
                input[c] = (dry * (1.0 - wet)) + (x * wet);
            }
            *out_l = dither32(input[0], &mut self.fpd_l) as f32;
            *out_r = dither32(input[1], &mut self.fpd_r) as f32;
        }
    }
}

/// The repeated sine stages plus the final boosted/starved blend.
#[inline]
fn densify(mut x: f64, density: f64, out: f64) -> f64 {
    let mut count = density;
    while count > 1.0 {
        let mut b = x.abs() * 1.57079633;
        if b > 1.57079633 {
            b = 1.57079633;
        }
        b = b.sin();
        x = if x > 0.0 { b } else { -b };
        count -= 1.0;
    }
    let mut b = x.abs() * 1.57079633;
    if b > 1.57079633 {
        b = 1.57079633;
    }
    if density > 0.0 {
        b = b.sin();
    } else {
        b = 1.0 - b.cos(); //produce either boosted or starved version
    }
    if x > 0.0 {
        (x * (1.0 - out)) + (b * out)
    } else {
        (x * (1.0 - out)) - (b * out) //blend according to density control
    }
}

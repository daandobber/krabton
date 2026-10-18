//! Ensemble: a chorus of many swirling voices (`EnsembleProc.cpp`).
//!
//! Copyright (c) Chris Johnson (airwindows), MIT license. Rust port for Krabton.

use crate::dsp::{dither32, noise_floor};
use crate::{EffectInfo, Kernel, ParamInfo};

pub(crate) const INFO: EffectInfo = EffectInfo {
    id: "Ensemble",
    name: "Ensemble",
    category: "Other",
    description: "Stacks up to 48 slowly drifting chorus voices into a thick, lush ensemble sound.",
    params: &[
        ParamInfo { name: "Ensemble", default: 0.5 },
        ParamInfo { name: "Fullness", default: 0.0 },
        ParamInfo { name: "Brighten", default: 1.0 },
        ParamInfo { name: "Dry/Wet", default: 1.0 },
    ],
};

/// `totalsamples`: the double buffer, two copies of 32767 samples. Its size
/// does not depend on the sample rate (the taps reach at most ~1900 samples
/// past the write position, at 22.05 kHz and Fullness 0).
const TOTAL: usize = 65540;
/// The write position counts down from here to 1.
const HALF: usize = 32767;
/// `floor((A*46.0)+2.9)` at A = 1.
const MAX_TAPS: usize = 48;
const PI: f64 = 3.141592653589793238;

/// The "air" filter that adds back the highs the interpolation loses.
#[derive(Clone, Copy, Default)]
struct Air {
    prev: f64,
    even: f64,
    odd: f64,
    factor: f64,
}

impl Air {
    #[inline]
    fn tick(&mut self, x: f64, flip: bool) -> f64 {
        self.factor = self.prev - x;
        if flip {
            self.even += self.factor;
            self.odd -= self.factor;
            self.factor = self.even;
        } else {
            self.odd += self.factor;
            self.even -= self.factor;
            self.factor = self.odd;
        }
        self.odd = (self.odd - ((self.odd - self.even) / 256.0)) / 1.0001;
        self.even = (self.even - ((self.even - self.odd) / 256.0)) / 1.0001;
        self.prev = x;
        self.factor
    }
}

pub(crate) struct Ensemble {
    overallscale: f64,
    /// `dL`, `dR`: `float` in the original, so stored as `f32` here too.
    d: [Box<[f32]>; 2],
    sweep: [f64; MAX_TAPS + 1],
    gcount: usize,
    air: [Air; 2],
    fp_flip: bool,
    fpd_l: u32,
    fpd_r: u32,
}

impl Kernel for Ensemble {
    const NUM_PARAMS: usize = 4;

    fn new(sample_rate: f64) -> Self {
        let mut s = Self {
            overallscale: sample_rate / 44100.0,
            d: [vec![0.0; TOTAL].into_boxed_slice(), vec![0.0; TOTAL].into_boxed_slice()],
            sweep: [PI / 2.0; MAX_TAPS + 1],
            gcount: 0,
            air: [Air::default(); 2],
            fp_flip: true,
            fpd_l: crate::dsp::seed(),
            fpd_r: crate::dsp::seed(),
        };
        s.reset();
        s
    }

    fn reset(&mut self) {
        self.d[0].fill(0.0);
        self.d[1].fill(0.0);
        self.sweep = [PI / 2.0; MAX_TAPS + 1];
        self.gcount = 0;
        self.air = [Air::default(); 2];
        self.fp_flip = true;
    }

    fn process(&mut self, p: &[f64], left: &mut [f32], right: &mut [f32]) {
        let overallscale = self.overallscale;
        // `B/12` is float math in the original (B is a float, 12 an int)
        let mut spd = (0.4 + (p[1] as f32 / 12.0) as f64).powf(10.0);
        spd *= overallscale;
        let depth = 0.002 / spd;
        let tupi = PI * 2.0;
        let taps = ((p[0] * 46.0) + 2.9).floor();
        let brighten = p[2];
        let wet = p[3];
        let hapi = PI / taps;
        let n = (taps as usize).clamp(1, MAX_TAPS);

        let mut start = [0.0f64; MAX_TAPS + 1];
        let mut sinoffset = [0.0f64; MAX_TAPS + 1];
        let mut speed = [0.0f64; MAX_TAPS + 1];
        for count in 1..=n {
            start[count] = depth * count as f64;
            sinoffset[count] = hapi * (count - 1) as f64;
            speed[count] = spd / (1.0 + (count as f64 / taps));
        }
        //that's for speeding up things in the sample-processing area

        for (out_l, out_r) in left.iter_mut().zip(right.iter_mut()) {
            let mut input_l = noise_floor(*out_l as f64, self.fpd_l);
            let mut input_r = noise_floor(*out_r as f64, self.fpd_r);
            let dry_l = input_l;
            let dry_r = input_r;

            input_l += self.air[0].tick(input_l, self.fp_flip) * brighten;
            input_r += self.air[1].tick(input_r, self.fp_flip) * brighten;
            //air, compensates for loss of highs in flanger's interpolation
            self.fp_flip = !self.fp_flip;

            if self.gcount < 1 || self.gcount > HALF {
                self.gcount = HALF;
            }
            let gcount = self.gcount;
            let [dl, dr] = &mut self.d;
            dl[gcount] = input_l as f32;
            dl[gcount + HALF] = dl[gcount];
            dr[gcount] = input_r as f32;
            dr[gcount + HALF] = dr[gcount];
            let mut temp_l = input_l;
            let mut temp_r = input_r;
            //double buffer

            for ensemble in 1..=n {
                let offset = start[ensemble] + (depth * (self.sweep[ensemble] + sinoffset[ensemble]).sin());
                let floffset = offset - offset.floor();
                let count = (gcount + offset.floor() as usize).min(TOTAL - 3);

                temp_l += dl[count] as f64 * (1.0 - floffset); //less as value moves away from .0
                temp_l += dl[count + 1] as f64; //we can assume always using this in one way or another?
                temp_l += dl[count + 2] as f64 * floffset; //greater as value moves away from .0
                temp_l -= interpolation_hack(&dl[count..count + 3]);

                temp_r += dr[count] as f64 * (1.0 - floffset);
                temp_r += dr[count + 1] as f64;
                temp_r += dr[count + 2] as f64 * floffset;
                temp_r -= interpolation_hack(&dr[count..count + 3]);

                self.sweep[ensemble] += speed[ensemble];
                if self.sweep[ensemble] > tupi {
                    self.sweep[ensemble] -= tupi;
                }
            }
            self.gcount -= 1;
            //still scrolling through the samples, remember

            input_l = temp_l / (4.0 * taps.sqrt());
            input_r = temp_r / (4.0 * taps.sqrt());

            if wet != 1.0 {
                input_l = (input_l * wet) + (dry_l * (1.0 - wet));
                input_r = (input_r * wet) + (dry_r * (1.0 - wet));
            }

            *out_l = dither32(input_l, &mut self.fpd_l) as f32;
            *out_r = dither32(input_r, &mut self.fpd_r) as f32;
        }
    }
}

/// `((d[c]-d[c+1])-(d[c+1]-d[c+2]))/50`, in `float` like the original
/// ("interpolation hacks 'r us").
#[inline]
fn interpolation_hack(d: &[f32]) -> f64 {
    (((d[0] - d[1]) - (d[1] - d[2])) / 50.0) as f64
}

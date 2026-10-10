//! IronOxide5: tape emulation with tape speed, flutter and noise
//! (`IronOxide5Proc.cpp`).
//!
//! Copyright (c) Chris Johnson (airwindows), MIT license. Rust port for Krabton.

use crate::dsp::{dither32, noise_floor, unit};
use crate::{EffectInfo, Kernel, ParamInfo};

pub(crate) const INFO: EffectInfo = EffectInfo {
    id: "IronOxide5",
    name: "IronOxide5",
    category: "Tape",
    description: "Classic, punchy tape saturation with separate high and low tape speeds, flutter and tape noise.",
    params: &[
        ParamInfo { name: "Input Trim", default: 0.5 },
        ParamInfo { name: "Tape High", default: 0.562341325190349 },
        ParamInfo { name: "Tape Low", default: 0.562341325190349 },
        ParamInfo { name: "Flutter", default: 0.5 },
        ParamInfo { name: "Noise", default: 0.5 },
        ParamInfo { name: "Output Trim", default: 0.5 },
        ParamInfo { name: "Inv/Dry/Wet", default: 1.0 },
    ],
};

/// The taps of the slow (tape head) average, after `count += 3`, in the
/// order the original adds them; a `None` is a `temp /= 2`.
const TAPS: [Option<usize>; 36] = [
    Some(127), Some(113), Some(109), Some(107), Some(103), Some(101), Some(97), Some(89), Some(83), None,
    Some(79), Some(73), Some(71), Some(67), Some(61), Some(59), Some(53), Some(47), Some(43), Some(41), Some(37), Some(31), Some(29), None,
    Some(23), Some(19), Some(17), Some(13), Some(11), None,
    Some(7), Some(5), Some(3), None,
    Some(2), Some(1),
];

#[derive(Clone, Copy, Default)]
struct Channel {
    /// iirSampleA/B: the low-leaning highpass, alternating with `flip`.
    iir: [f64; 2],
    fast_iir: [f64; 2],
    slow_iir: [f64; 2],
    prev_input: f64,
}

pub(crate) struct IronOxide5 {
    overallscale: f64,
    d: [[f64; 264]; 2],
    fl: [[f64; 100]; 2],
    gcount: i64,
    fstoredcount: i64,
    flip: bool,
    rateof: f64,
    sweep: f64,
    nextmax: f64,
    ch: [Channel; 2],
    fpd_l: u32,
    fpd_r: u32,
}

impl Kernel for IronOxide5 {
    const NUM_PARAMS: usize = 7;

    fn new(sample_rate: f64) -> Self {
        let mut s = Self {
            overallscale: sample_rate / 44100.0,
            d: [[0.0; 264]; 2],
            fl: [[0.0; 100]; 2],
            gcount: 0,
            fstoredcount: 0,
            flip: false,
            rateof: 0.5,
            sweep: 0.0,
            nextmax: 0.5,
            ch: [Channel::default(); 2],
            fpd_l: crate::dsp::seed(),
            fpd_r: crate::dsp::seed(),
        };
        s.reset();
        s
    }

    fn reset(&mut self) {
        self.d = [[0.0; 264]; 2];
        self.fl = [[0.0; 100]; 2];
        self.gcount = 0;
        self.fstoredcount = 0;
        self.flip = false;
        self.rateof = 0.5;
        self.sweep = 0.0;
        self.nextmax = 0.5;
        self.ch = [Channel::default(); 2];
    }

    fn process(&mut self, p: &[f64], left: &mut [f32], right: &mut [f32]) {
        let overallscale = self.overallscale;
        let inputgain = 10.0f64.powf(((p[0] * 36.0) - 18.0) / 20.0);
        let outputgain = 10.0f64.powf(((p[5] * 36.0) - 18.0) / 20.0);
        let mut ips = (((p[1] * p[1]) * (p[1] * p[1]) * 148.5) + 1.5) * 1.1;
        if !(1.0..=200.0).contains(&ips) {
            ips = 33.0;
        }
        let temp_randy = 0.04 + (0.11 / ips.sqrt());
        let mut lps = (((p[2] * p[2]) * (p[2] * p[2]) * 148.5) + 1.5) * 1.1;
        if !(1.0..=200.0).contains(&lps) {
            lps = 33.0;
        }
        let mut iir_amount = lps / 430.0; //for low leaning
        let mut fast_taper = ips / 15.0;
        let mut slow_taper = 2.0 / (lps * lps);
        let mut lowspeedscale = 5.0 / ips;
        let depth = p[3].powf(2.0) * overallscale;
        let fluttertrim = 0.00581 / overallscale;
        let sweeptrim = (0.0005 * depth) / overallscale;
        let tupi = 3.141592653589793238 * 2.0;
        let newrate = 0.006 / overallscale;
        let oldrate = 1.0 - newrate;
        if overallscale == 0.0 {
            fast_taper += 1.0;
            slow_taper += 1.0;
        } else {
            iir_amount /= overallscale;
            lowspeedscale *= overallscale;
            fast_taper = 1.0 + (fast_taper / overallscale);
            slow_taper = 1.0 + (slow_taper / overallscale);
        }
        let noise = p[4] * 0.5;
        let invdrywet = (p[6] * 2.0) - 1.0;
        let mut dry_gain = 1.0;
        if invdrywet > 0.0 {
            dry_gain -= invdrywet;
        }

        for (out_l, out_r) in left.iter_mut().zip(right.iter_mut()) {
            let mut input = [noise_floor(*out_l as f64, self.fpd_l), noise_floor(*out_r as f64, self.fpd_r)];
            let mut dry = input;

            let flutterrandy = unit(self.fpd_l);
            if self.fstoredcount < 0 || self.fstoredcount > 30 {
                self.fstoredcount = 30;
            }
            let fstored = self.fstoredcount as usize;
            let offset = (1.0 + self.sweep.sin()) * depth;
            let flooroffset = offset - offset.floor();
            // stays far inside the buffer up to the highest sample rate
            let flutcount = (fstored + offset.floor() as usize).min(98);
            for c in 0..2 {
                self.fl[c][fstored + 31] = input[c];
                self.fl[c][fstored] = input[c];
                input[c] = (self.fl[c][flutcount] * (1.0 - flooroffset)) + (self.fl[c][flutcount + 1] * flooroffset);
            }
            self.rateof = (self.nextmax * newrate) + (self.rateof * oldrate);
            self.sweep += self.rateof * fluttertrim;
            self.sweep += self.sweep * sweeptrim;
            if self.sweep >= tupi {
                self.sweep = 0.0;
                self.nextmax = 0.02 + (flutterrandy * 0.98);
            }
            self.fstoredcount -= 1;

            let set = if self.flip { 0 } else { 1 };
            for c in 0..2 {
                let ch = &mut self.ch[c];
                ch.iir[set] = (ch.iir[set] * (1.0 - iir_amount)) + (input[c] * iir_amount);
                input[c] -= ch.iir[set];
                input[c] *= inputgain;
                input[c] = bridge(input[c]);
            }

            if self.gcount < 0 || self.gcount > 131 {
                self.gcount = 131;
            }
            let count = self.gcount as usize;
            for c in 0..2 {
                let d = &mut self.d[c];
                d[count + 131] = input[c];
                d[count] = input[c];
                let ch = &mut self.ch[c];
                ch.fast_iir[set] /= fast_taper;
                ch.slow_iir[set] /= slow_taper;
                ch.fast_iir[set] += d[count];
                let base = count + 3;
                let mut temp = 0.0;
                let mut first = true;
                for tap in TAPS {
                    match tap {
                        Some(t) if first => {
                            temp = d[base + t];
                            first = false;
                        }
                        Some(t) => temp += d[base + t],
                        None => temp /= 2.0,
                    }
                }
                ch.slow_iir[set] += temp / 128.0;
                input[c] = ch.fast_iir[set] - (ch.slow_iir[set] / slow_taper);
                input[c] /= fast_taper;
                input[c] /= lowspeedscale;
                input[c] = bridge(input[c]);
            }

            let randy = (0.55 + temp_randy + (unit(self.fpd_r) * temp_randy)) * noise; //0 to 2
            for c in 0..2 {
                let ch = &mut self.ch[c];
                input[c] *= 1.0 - randy;
                input[c] += ch.prev_input * randy;
                ch.prev_input = dry[c];
            }
            self.flip = !self.flip;

            for c in 0..2 {
                if outputgain != 1.0 {
                    input[c] *= outputgain;
                }
                if invdrywet != 1.0 {
                    input[c] *= invdrywet;
                }
                if dry_gain != 1.0 {
                    dry[c] *= dry_gain;
                }
                if dry[c].abs() > 0.0 {
                    input[c] += dry[c];
                }
            }

            *out_l = dither32(input[0], &mut self.fpd_l) as f32;
            *out_r = dither32(input[1], &mut self.fpd_r) as f32;
        }
    }
}

/// `bridgerectifier = sin(min(fabs(x), pi/2))` with the sign put back.
#[inline]
fn bridge(x: f64) -> f64 {
    let mut b = x.abs();
    if b > 1.57079633 {
        b = 1.57079633;
    }
    b = b.sin();
    if x > 0.0 {
        b
    } else {
        -b
    }
}

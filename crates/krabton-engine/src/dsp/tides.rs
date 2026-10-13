//! Tides: a tidal modulator, after Mutable Instruments Tides (2018). A
//! slope that rises and falls (AD: once per trigger; Loop: over and over;
//! AR: up while a note is held, down when it is let go) with Shape bending
//! its curves, Slope moving its peak, Smoothness rounding it off (to the
//! left) or folding it (to the right), and four outputs whose relation Shift
//! sets: Gates (the slope, a plain ramp and end-of-attack and
//! end-of-release pulses), Amplitude (the slope moving from output to
//! output), Phase (four phases of it) and Frequency (four at ratios of each
//! other, from subharmonics to chords, always in step).
//!
//! The slope generator is ported from Tides' firmware, at control rate:
//!
//! Copyright 2017 Emilie Gillet.
//!
//! Permission is hereby granted, free of charge, to any person obtaining a
//! copy of this software and associated documentation files (the
//! "Software"), to deal in the Software without restriction, including
//! without limitation the rights to use, copy, modify, merge, publish,
//! distribute, sublicense, and/or sell copies of the Software, and to permit
//! persons to whom the Software is furnished to do so, subject to the
//! following conditions:
//!
//! The above copyright notice and this permission notice shall be included
//! in all copies or substantial portions of the Software.
//!
//! THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS
//! OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
//! MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN
//! NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM,
//! DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR
//! OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE
//! USE OR OTHER DEALINGS IN THE SOFTWARE.

use std::f64::consts::PI;

pub const RAMP_MODES: &[&str] = &["AD", "Loop", "AR"];
pub const OUTPUT_MODES: &[&str] = &["Gates", "Amplitude", "Phase", "Frequency"];
pub const AD: usize = 0;
pub const LOOP: usize = 1;
pub const AR: usize = 2;
pub const GATES: usize = 0;
pub const AMPLITUDE: usize = 1;
pub const PHASE: usize = 2;
pub const FREQUENCY: usize = 3;
pub const CHANNELS: usize = 4;

// ---------------------------------------------------------------- tables

const WS: usize = 1024;
const TABLE: usize = WS + 1;

struct Tables {
    /// Five audio-rate and seven control-rate curves, 1025 values each.
    wavetable: Vec<i16>,
    bipolar_fold: Vec<f32>,
    unipolar_fold: Vec<f32>,
}

fn tables() -> &'static Tables {
    static T: std::sync::OnceLock<Tables> = std::sync::OnceLock::new();
    T.get_or_init(|| {
        let q = |v: f64| (v * 32767.0).round() as i16;
        let mut wavetable = Vec::with_capacity(12 * TABLE);
        let x: Vec<f64> = (0..WS).map(|i| i as f64 / WS as f64).collect();
        let tan_scale = 8.0f64.atan();
        let curve = |f: &dyn Fn(f64) -> f64| -> Vec<f64> { x.iter().map(|&x| f(x)).collect() };
        let inverse_tan = curve(&|x| if x == 0.0 { 0.0 } else { ((tan_scale * (1.0 - 2.0 * x)).tan() / 8.0).acos() / PI });
        let inverse_sin = curve(&|x| (1.0 - 2.0 * x).acos() / PI);
        let linear = curve(&|x| x);
        let sin = curve(&|x| 0.5 - 0.5 * (x * PI).cos());
        let bump = curve(&|x| {
            let fade = (4.0 - 4.0 * x).min(1.0);
            (1.0 - (PI * x * 1.5).cos()) * (1.0 - (PI * fade).cos()) / 4.5
        });
        for t in [&inverse_tan, &inverse_sin, &linear, &sin, &bump] {
            wavetable.extend(t.iter().map(|&v| q(v)));
            wavetable.push(q(t[WS - 1]));
        }
        // Control rate: a rise, the top, and the fall (mirrored, or 1 - x).
        let h = WS / 2;
        let x: Vec<f64> = (0..h).map(|i| i as f64 / h as f64).collect();
        let curve = |f: &dyn Fn(f64) -> f64| -> Vec<f64> { x.iter().map(|&x| f(x)).collect() };
        let expo_raw = curve(&|x| 1.0 - (-5.0 * x).exp());
        let expo_max = expo_raw.iter().cloned().fold(f64::MIN, f64::max);
        let expo: Vec<f64> = expo_raw.iter().map(|v| v / expo_max).collect();
        let log = curve(&|x| (1.0 - x * expo_max).ln() / -5.0);
        let linear = curve(&|x| x);
        let sin = curve(&|x| (1.0 - (PI * x).cos()) / 2.0);
        let inverse_sin = curve(&|x| (1.0 - 2.0 * x).acos() / PI);
        let mut flip = |t: &[f64], mirrored: bool| {
            wavetable.extend(t.iter().map(|&v| q(v)));
            wavetable.push(q(1.0));
            if mirrored {
                wavetable.extend(t.iter().rev().map(|&v| q(v)));
            } else {
                wavetable.extend(t.iter().map(|&v| q(1.0 - v)));
            }
        };
        flip(&log, false);
        flip(&log, true);
        flip(&inverse_sin, false);
        flip(&linear, false);
        flip(&sin, false);
        flip(&expo, true);
        flip(&expo, false);
        debug_assert_eq!(wavetable.len(), 12 * TABLE);

        let n = WS + 4;
        let mut xb: Vec<f64> = (0..n).map(|i| i as f64 / (WS as f64 / 2.0) - 1.0).collect();
        xb[n - 1] = xb[n - 2];
        let bf: Vec<f64> = xb
            .iter()
            .map(|&x| {
                let window = (-x * x * 4.0).exp().powi(2);
                (8.0 * PI * x).sin() * window + (3.0 * x).atan() * (1.0 - window)
            })
            .collect();
        let m = bf.iter().fold(0.0f64, |a, v| a.max(v.abs()));
        let bipolar_fold = bf.iter().map(|v| (v / m) as f32).collect();
        let mut xu: Vec<f64> = (0..n).map(|i| i as f64 / WS as f64).collect();
        xu[n - 1] = xu[n - 3];
        xu[n - 2] = xu[n - 3];
        let uf: Vec<f64> = xu
            .iter()
            .map(|&x| {
                let window = (-x * x * 4.0).exp().powi(2);
                (0.38 * (16.0 * PI * x).sin() + 4.0 * x) * window + (4.0 * x).atan() * (1.0 - window)
            })
            .collect();
        let m = uf.iter().fold(0.0f64, |a, v| a.max(v.abs()));
        let unipolar_fold = uf.iter().map(|v| (v / m) as f32).collect();
        Tables { wavetable, bipolar_fold, unipolar_fold }
    })
}

fn interpolate(t: &[f32], x: f32, size: f32) -> f32 {
    let xi = (x * size).max(0.0);
    let i = (xi as usize).min(t.len() - 2);
    let f = xi - i as f32;
    t[i] + (t[i + 1] - t[i]) * f
}

// ---------------------------------------------------------------- ratios

/// The Frequency mode's ratios (and how many master cycles each takes to
/// come round again), by Shift.
const RATIOS: [[(f32, i32); CHANNELS]; 21] = [
    [(1.0, 1), (0.5, 2), (0.25, 4), (0.125, 8)],
    [(1.0, 1), (0.5, 2), (0.333_333_33, 3), (0.2, 5)],
    [(1.0, 1), (0.5, 2), (0.333_333_33, 3), (0.25, 4)],
    [(1.0, 1), (0.666_666_66, 3), (0.5, 2), (0.25, 4)],
    [(1.0, 1), (0.666_666_66, 3), (0.5, 2), (0.333_333_33, 3)],
    [(1.0, 1), (0.75, 4), (0.666_666_66, 3), (0.5, 2)],
    [(1.0, 1), (0.8, 5), (0.666_666_66, 3), (0.5, 2)],
    [(1.0, 1), (0.8, 5), (0.75, 3), (0.5, 2)],
    [(1.0, 1), (0.8, 5), (0.75, 4), (0.666_666_66, 3)],
    [(1.0, 1), (0.909_090_9, 11), (0.857_142_86, 7), (0.8, 5)],
    [(1.0, 1), (1.0, 1), (1.0, 1), (1.0, 1)],
    [(1.0, 1), (1.090_909_1, 11), (1.142_857_1, 7), (1.2, 5)],
    [(1.0, 1), (1.25, 4), (1.333_333_3, 3), (1.5, 2)],
    [(1.0, 1), (1.25, 4), (1.333_333_3, 3), (2.0, 2)],
    [(1.0, 1), (1.25, 4), (1.5, 3), (2.0, 2)],
    [(1.0, 1), (1.333_333_3, 3), (1.5, 2), (2.0, 1)],
    [(1.0, 1), (1.5, 2), (2.0, 1), (3.0, 1)],
    [(1.0, 1), (1.5, 2), (2.0, 1), (4.0, 1)],
    [(1.0, 1), (2.0, 1), (3.0, 1), (4.0, 1)],
    [(1.0, 1), (2.0, 1), (3.0, 1), (5.0, 1)],
    [(1.0, 1), (2.0, 1), (4.0, 1), (8.0, 1)],
];

/// The ratios Shift picks (for the screen).
pub fn ratios(shift: f32) -> [f32; CHANNELS] {
    let i = ((shift.clamp(0.0, 1.0) * 21.0) as usize).min(20);
    RATIOS[i].map(|r| r.0)
}

// ---------------------------------------------------------------- state

/// The knobs.
#[derive(Clone, Copy, Debug)]
pub struct Settings {
    pub ramp_mode: usize,
    pub output_mode: usize,
    pub shape: f32,
    pub slope: f32,
    pub smoothness: f32,
    pub shift: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct Tides {
    master_phase: f32,
    wrap: [i32; CHANNELS],
    phase: [f32; CHANNELS],
    freq: [f32; CHANNELS],
    ratio: [(f32, i32); CHANNELS],
    ratio_index: i32,
    prev_shift: [f32; CHANNELS],
    ws_prev_in: [f32; CHANNELS],
    ws_prev_out: [f32; CHANNELS],
    ws_break: [f32; CHANNELS],
    lp1: [f32; CHANNELS],
    lp2: [f32; CHANNELS],
    /// The four outputs: -1..1 looping, 0..1 otherwise.
    pub out: [f32; CHANNELS],
}

impl Default for Tides {
    fn default() -> Self {
        Self {
            master_phase: 0.0,
            wrap: [0; CHANNELS],
            // Finished: an envelope waits for its first trigger.
            phase: [1.0; CHANNELS],
            freq: [0.0; CHANNELS],
            ratio: [(1.0, 1); CHANNELS],
            ratio_index: 10,
            prev_shift: [0.0; CHANNELS],
            ws_prev_in: [0.0; CHANNELS],
            ws_prev_out: [0.0; CHANNELS],
            ws_break: [0.0; CHANNELS],
            lp1: [0.0; CHANNELS],
            lp2: [0.0; CHANNELS],
            out: [0.0; CHANNELS],
        }
    }
}

/// How much of a curve survives at a frequency (the shapes and folds are
/// tamed as the slope gets fast).
fn tame(f0: f32, harmonics: f32, order: f32) -> f32 {
    let f0 = f0 * harmonics;
    let max_f = 0.5 / order;
    let a = (1.0 - (f0 - max_f) / (0.5 - max_f)).clamp(0.0, 1.0);
    a * a * a
}

impl Tides {
    fn skewed_ramp(&mut self, j: usize, mut phase: f32, shift: f32, mut frequency: f32, pw: f32) -> f32 {
        if shift != 0.0 {
            phase += shift;
            frequency += shift - self.prev_shift[j];
            self.prev_shift[j] = shift;
            if phase >= 1.0 {
                phase -= 1.0;
            } else if phase < 0.0 {
                phase += 1.0;
            }
        }
        let lo = frequency.abs() * 2.0;
        let pw = pw.clamp(lo.min(0.5), (1.0 - lo).max(0.5));
        let up = 0.5 / pw;
        let down = 0.5 / (1.0 - pw);
        if phase < pw {
            phase * up
        } else {
            (phase - pw) * down + 0.5
        }
    }

    fn slope_of(&mut self, mode: usize, j: usize, phase: f32, shift: f32, frequency: f32, pw: f32) -> f32 {
        match mode {
            AR => phase,
            AD => self.skewed_ramp(j, phase, 0.0, frequency, pw),
            _ => self.skewed_ramp(j, phase, shift, frequency, pw),
        }
    }

    fn shape(&mut self, mode: usize, j: usize, input: f32, table: usize, fraction: f32) -> f32 {
        let w = &tables().wavetable;
        let idx = 1024.0 * input;
        let i = (idx as i32 & 1023) as usize;
        let f = idx - idx.floor();
        let base = table * TABLE;
        let at = |k: usize| w[(base + k).min(w.len() - 1)] as f32 / 32768.0;
        let x = at(i) + (at(i + 1) - at(i)) * f;
        let y = at(i + TABLE) + (at(i + TABLE + 1) - at(i + TABLE)) * f;
        let mut output = x + (y - x) * fraction;
        if mode != AR {
            return output;
        }
        let prev = self.ws_prev_in[j];
        if prev <= 0.5 && input > 0.5 || prev > 0.5 && input < 0.5 {
            self.ws_break[j] = self.ws_prev_out[j];
        } else if input == 1.0 {
            self.ws_break[j] = 1.0;
        } else if input == 0.5 {
            self.ws_break[j] = 0.0;
        }
        let b = self.ws_break[j];
        output = if input <= 0.5 { b + (1.0 - b) * output } else { b * output };
        self.ws_prev_in[j] = input;
        self.ws_prev_out[j] = output;
        output
    }

    fn fold(mode: usize, unipolar: f32, amount: f32) -> f32 {
        let t = tables();
        if mode == LOOP {
            let bipolar = 2.0 * unipolar - 1.0;
            let folded = if amount > 0.0 { interpolate(&t.bipolar_fold, 0.5 + bipolar * (0.03 + 0.46 * amount), 1024.0) } else { 0.0 };
            bipolar + (folded - bipolar) * amount
        } else {
            let folded = if amount > 0.0 { interpolate(&t.unipolar_fold, unipolar * amount, 1024.0) } else { 0.0 };
            unipolar + (folded - unipolar) * amount
        }
    }

    /// Advances one control block of `frames` samples. `f0`: cycles per
    /// sample; `ramp`: the song's phase in the cycle (Loop synced to it);
    /// `rising`/`high`: the gate (a note started, notes are held).
    #[allow(clippy::too_many_arguments)]
    pub fn process(&mut self, s: &Settings, f0: f32, frames: usize, ramp: Option<f32>, rising: bool, high: bool) -> [f32; CHANNELS] {
        let mode = s.ramp_mode.min(2);
        let output_mode = s.output_mode.min(3);
        let n = frames.max(1) as f32;
        let frequency = f0.clamp(0.0, 0.25);
        let mut pw = s.slope.clamp(0.0, 1.0);
        if pw < 0.5 {
            // A more interesting range of attack times.
            pw = 0.5 + 0.6 * (pw - 0.5) / ((pw - 0.5).abs() + 0.1);
        }
        let slope = 3.0 + (pw - 0.5).abs() * 5.0;
        let shape_amount = (s.shape - 0.5).abs() * 2.0;
        let attenuation = tame(frequency, slope, 16.0);
        let shape = 0.5 + (s.shape.clamp(0.0, 1.0) - 0.5) * attenuation;
        let mut smoothness = s.smoothness.clamp(0.0, 1.0);
        if smoothness > 0.5 {
            smoothness = 0.5 + (smoothness - 0.5) * tame(frequency, slope * (3.0 + shape_amount * attenuation * 5.0), 12.0);
        }
        let shift = 2.0 * s.shift.clamp(0.0, 1.0) - 1.0;
        let shape_index = shape * 5.9999 + 5.0;
        let (shape_table, shape_frac) = (shape_index as usize, shape_index.fract());
        let fold_amount = (2.0 * (smoothness - 0.5)).max(0.0);
        let next_ratio = if output_mode == FREQUENCY {
            // stmlib's hysteresis quantizer over 21 ratio sets.
            let v = s.shift.clamp(0.0, 1.0) * 21.0 - 0.5;
            let sign = if v > self.ratio_index as f32 { -1.0 } else { 1.0 };
            self.ratio_index = ((v + sign * 0.05 + 0.5) as i32).clamp(0, 20);
            RATIOS[self.ratio_index as usize]
        } else {
            [(1.0, 1); CHANNELS]
        };
        let step = shift / (CHANNELS - 1) as f32;
        let partial_step = shift / CHANNELS as f32;
        let pw_increment = (if shift > 0.0 { 1.0 - pw } else { pw }) * step;
        let per_channel_pw: [f32; CHANNELS] = std::array::from_fn(|j| pw + pw_increment * j as f32);

        // ---------------------------------------------------------------- the ramps
        let channels = if output_mode == FREQUENCY || (output_mode == PHASE && mode == AR) { CHANNELS } else { 1 };
        match mode {
            AD => {
                if rising {
                    self.phase[..channels].fill(0.0);
                }
                for i in 0..channels {
                    self.freq[i] = (frequency * next_ratio[i].0).min(0.25);
                    self.phase[i] = (self.phase[i] + self.freq[i] * n).min(1.0);
                }
            }
            AR => {
                for i in 0..channels {
                    self.freq[i] = if output_mode == PHASE { frequency } else { (frequency * next_ratio[i].0).min(0.25) };
                }
                let clip_at = if high { 0.5 } else { 1.0 };
                for i in 0..channels {
                    if self.phase[i] < 0.5 && !high {
                        self.phase[i] = 0.5;
                    } else if self.phase[i] > 0.5 && high {
                        self.phase[i] = 0.0;
                    }
                    let this_pw = if output_mode == FREQUENCY { pw } else { per_channel_pw[i] };
                    let slope = if self.phase[i] < 0.5 { 0.5 / (1.0e-6 + this_pw) } else { 0.5 / (1.0 + 1.0e-6 - this_pw) };
                    self.phase[i] = (self.phase[i] + self.freq[i] * slope * n).min(clip_at);
                }
            }
            _ => {
                let mut wrapped = false;
                match ramp {
                    Some(r) => {
                        for i in 0..channels {
                            self.freq[i] = (frequency * self.ratio[i].0).min(0.25);
                        }
                        if r < self.master_phase {
                            wrapped = true;
                        }
                        self.master_phase = r;
                    }
                    None => {
                        let mut reset = false;
                        if rising {
                            self.master_phase = 0.0;
                            self.ratio[..channels].copy_from_slice(&next_ratio[..channels]);
                            self.wrap[..channels].fill(0);
                            reset = true;
                        }
                        for i in 0..channels {
                            self.freq[i] = (frequency * self.ratio[i].0).min(0.25);
                        }
                        if !reset {
                            self.master_phase += frequency * n;
                        }
                        if self.master_phase >= 1.0 {
                            self.master_phase -= self.master_phase.floor();
                            wrapped = true;
                        }
                    }
                }
                if wrapped {
                    for i in 0..channels {
                        self.wrap[i] += 1;
                        if self.wrap[i] >= self.ratio[i].1 {
                            self.ratio[i] = next_ratio[i];
                            self.wrap[i] = 0;
                        }
                    }
                }
                for i in 0..channels {
                    let p = (self.master_phase + self.wrap[i] as f32) * self.ratio[i].0;
                    self.phase[i] = p - p.floor();
                }
            }
        }

        // ---------------------------------------------------------------- the outputs
        let scale = |u: f32| if mode == LOOP { 2.0 * u - 1.0 } else { u };
        let mut out = [0.0f32; CHANNELS];
        match output_mode {
            GATES => {
                let (phase, f) = (self.phase[0], self.freq[0]);
                let raw = self.slope_of(mode, 0, phase, 0.0, f, pw);
                let slope = self.shape(mode, 0, raw, shape_table, shape_frac);
                out[0] = Self::fold(mode, slope, fold_amount) * shift;
                let plain = self.shape(mode, 1, raw, 8, 0.0);
                out[1] = scale(plain);
                out[2] = match mode {
                    AR => (phase >= 0.5) as u8 as f32,
                    _ => (phase >= pw) as u8 as f32,
                };
                out[3] = match mode {
                    LOOP => (phase < (96.0 * f).min(0.5)) as u8 as f32,
                    _ => (phase >= 1.0) as u8 as f32,
                };
            }
            AMPLITUDE => {
                let (phase, f) = (self.phase[0], self.freq[0]);
                let raw = self.slope_of(mode, 0, phase, 0.0, f, pw);
                let shaped = self.shape(mode, 0, raw, shape_table, shape_frac);
                let slope = Self::fold(mode, shaped, fold_amount) * if shift < 0.0 { -1.0 } else { 1.0 };
                let index = (shift * 5.1).abs();
                for (j, o) in out.iter_mut().enumerate() {
                    let gain = (1.0 - ((j + 1) as f32 - index).abs()).max(0.0);
                    *o = slope * gain;
                }
            }
            PHASE => {
                let mut phase_shift = 0.0;
                for j in 0..CHANNELS {
                    let source = if mode == AR { j } else { 0 };
                    let this_pw = if mode == AD { per_channel_pw[j] } else { pw };
                    let raw = self.slope_of(mode, j, self.phase[source], phase_shift, self.freq[source], this_pw);
                    let shaped = self.shape(mode, j, raw, shape_table, shape_frac);
                    out[j] = Self::fold(mode, shaped, fold_amount);
                    phase_shift -= partial_step;
                }
            }
            _ => {
                for j in 0..CHANNELS {
                    let raw = self.slope_of(mode, j, self.phase[j], 0.0, self.freq[j], pw);
                    let shaped = self.shape(mode, j, raw, shape_table, shape_frac);
                    out[j] = Self::fold(mode, shaped, fold_amount);
                }
            }
        }

        // ---------------------------------------------------------------- smoothness
        if smoothness < 0.5 {
            let mut ratio = smoothness * 2.0;
            ratio *= ratio;
            ratio *= ratio;
            let last = if output_mode == GATES { 1 } else { CHANNELS };
            for i in 0..last {
                let source = if output_mode == FREQUENCY { i } else { 0 };
                let mut f = self.freq[source] * 0.5;
                f += (1.0 - f) * ratio;
                // The firmware's one-pole runs per sample; this runs per block.
                let c = 1.0 - (1.0 - f.clamp(0.0, 1.0)).powf(n);
                self.lp1[i] += (out[i] - self.lp1[i]) * c;
                self.lp2[i] += (self.lp1[i] - self.lp2[i]) * c;
                out[i] = self.lp2[i];
            }
        }
        self.out = out;
        out
    }
}

/// One cycle of the four outputs at these settings, `n` points (for the
/// screen): a loop's second cycle, an AD after a trigger, an AR held for
/// the first half.
pub fn preview(s: &Settings, n: usize) -> Vec<[f32; CHANNELS]> {
    let mut t = Tides::default();
    let f0 = 1.0 / n as f32;
    let mut out = Vec::with_capacity(n);
    let cycles = if s.ramp_mode == LOOP { 2 } else { 1 };
    for c in 0..cycles {
        for k in 0..n {
            let (rising, high) = match s.ramp_mode {
                AD => (c == 0 && k == 0, false),
                AR => (k == 0, k < n / 2),
                _ => (c == 0 && k == 0, false),
            };
            let o = t.process(s, f0, 1, None, rising, high);
            if c == cycles - 1 {
                out.push(o);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(ramp_mode: usize, output_mode: usize) -> Settings {
        Settings { ramp_mode, output_mode, shape: 0.5, slope: 0.5, smoothness: 0.5, shift: 0.5 }
    }

    #[test]
    fn the_tables_are_curves() {
        let t = tables();
        assert_eq!(t.wavetable.len(), 12 * TABLE);
        // The control-rate linear curve: up to the top at the middle, down again.
        let lin = &t.wavetable[8 * TABLE..9 * TABLE];
        assert_eq!(lin[0], 0);
        assert_eq!(lin[512], 32767);
        assert!(lin[1024] < 100);
        assert!((t.bipolar_fold[512].abs()) < 0.02);
        assert!(t.unipolar_fold.iter().all(|v| v.abs() <= 1.0001));
    }

    #[test]
    fn a_loop_is_a_triangle_at_noon_and_bends_with_shape() {
        let s = settings(LOOP, PHASE);
        let cycle = preview(&s, 64);
        let v: Vec<f32> = cycle.iter().map(|o| o[0]).collect();
        let (lo, hi) = v.iter().fold((1.0f32, -1.0f32), |(a, b), x| (a.min(*x), b.max(*x)));
        assert!(lo < -0.95 && hi > 0.95, "{lo} {hi}");
        // At noon a triangle: the peak in the middle.
        let peak = v.iter().enumerate().max_by(|a, b| a.1.total_cmp(b.1)).unwrap().0;
        assert!((28..=36).contains(&peak), "{peak}");
        // Slope moves the peak.
        let early = preview(&Settings { slope: 0.1, ..s }, 64);
        let peak = early.iter().enumerate().max_by(|a, b| a.1[0].total_cmp(&b.1[0])).unwrap().0;
        assert!(peak < 16, "{peak}");
        // Shape bends it: a different curve with the same ends.
        let bent = preview(&Settings { shape: 0.1, ..s }, 64);
        assert!((bent[16][0] - cycle[16][0]).abs() > 0.1);
    }

    #[test]
    fn ad_rises_falls_and_stays_down() {
        let s = settings(AD, PHASE);
        let mut t = Tides::default();
        let f0 = 1.0 / (48_000.0 * 0.5);
        let mut vals = Vec::new();
        for k in 0..800 {
            vals.push(t.process(&s, f0, 64, None, k == 0, k == 0)[0]);
        }
        let peak = vals.iter().cloned().fold(0.0f32, f32::max);
        assert!(peak > 0.95);
        // Half a second of 64-frame blocks: done after about 375.
        assert!(vals[700].abs() < 0.01);
        assert!(vals[100] > 0.3);
    }

    #[test]
    fn frequency_mode_runs_at_ratios() {
        let mut s = settings(LOOP, FREQUENCY);
        s.shift = 0.9; // ratios 1, 2, 3, 4
        assert_eq!(ratios(s.shift), [1.0, 2.0, 3.0, 4.0]);
        let mut t = Tides::default();
        let mut crossings = [0usize; CHANNELS];
        let mut last = [0.0f32; CHANNELS];
        // Four cycles of the master at 64 steps each.
        for k in 0..256 {
            let o = t.process(&s, 1.0 / 64.0, 1, None, k == 0, false);
            for j in 0..CHANNELS {
                if last[j] < 0.0 && o[j] >= 0.0 {
                    crossings[j] += 1;
                }
                last[j] = o[j];
            }
        }
        assert!((3..=5).contains(&crossings[0]), "{crossings:?}");
        assert!((15..=17).contains(&crossings[3]), "{crossings:?}");
    }

    #[test]
    fn synced_it_follows_the_songs_phase() {
        let s = settings(LOOP, PHASE);
        let mut t = Tides::default();
        let a = t.process(&s, 1.0 / 96_000.0, 64, Some(0.25), false, false)[0];
        let b = t.process(&s, 1.0 / 96_000.0, 64, Some(0.75), false, false)[0];
        assert!((a - 0.0).abs() < 0.05 && (b - 0.0).abs() < 0.05, "{a} {b}");
        let top = t.process(&s, 1.0 / 96_000.0, 64, Some(0.5), false, false)[0];
        assert!(top > 0.95);
    }
}

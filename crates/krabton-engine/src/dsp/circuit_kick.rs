//! Circuit kick: a port of ChowKick by Jatin Chowdhury (BSD 3-Clause; the
//! notice is below). A rectangular trigger pulse goes through the pulse
//! shaper of the 808's bass drum, modelled as a wave digital filter (the
//! source, R162, R163, C40 and the 1N4148 diode D53, its reflection solved
//! with the Wright omega function), then into a resonator: a biquad with
//! feedback around a band pass whose states pass through soft clippers, so
//! that it rings tighter (Tight) or bends its pitch (Bounce) as it is
//! driven, then a little noise riding on it, a tone low-pass and a DC block.
//!
//! Copyright (c) 2020, jatinchowdhury18. All rights reserved.
//! Redistribution and use in source and binary forms, with or without
//! modification, are permitted provided that the following conditions are
//! met: 1. Redistributions of source code must retain the above copyright
//! notice, this list of conditions and the following disclaimer.
//! 2. Redistributions in binary form must reproduce the above copyright
//! notice, this list of conditions and the following disclaimer in the
//! documentation and/or other materials provided with the distribution.
//! 3. Neither the name of the copyright holder nor the names of its
//! contributors may be used to endorse or promote products derived from this
//! software without specific prior written permission.
//! THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS
//! IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO,
//! THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR
//! PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR
//! CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL,
//! EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO,
//! PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR
//! PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF
//! LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING
//! NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS
//! SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

use std::f32::consts::TAU;

/// The Wright omega function, w + ln(w) = x.
fn wright_omega(x: f32) -> f32 {
    let x = x as f64;
    // A start close enough (e^x below 1, x - ln x above), then Newton.
    let mut w = if x < 1.0 { x.exp() } else { x - x.ln() };
    w = w.max(1e-30);
    for _ in 0..5 {
        w -= (w + w.ln() - x) / (1.0 + 1.0 / w);
        w = w.max(1e-30);
    }
    w as f32
}

/// The 808's pulse shaper as a wave digital filter.
#[derive(Clone, Copy, Default)]
struct PulseShaper {
    // Port resistances.
    r_c: f32,
    r162: f32,
    r163: f32,
    r_s1: f32,
    r_p2: f32,
    // C40 (alpha transform).
    c_b: f32,
    c_z: f32,
    b_coef: f32,
    a_coef: f32,
    // Waves going up, kept for coming back down.
    b_vs: f32,
    b_p1: f32,
    b_s1: f32,
    b_p2: f32,
    out: f32,
}

const VS_R: f32 = 1.0e-9;
const DIODE_IS: f32 = 2.52e-9;
const DIODE_VT: f32 = 25.85e-3;

impl PulseShaper {
    fn new(sr: f32, sustain: f32, decay: f32) -> Self {
        let alpha = 0.029f32;
        let c = 0.015e-6f32;
        let r_c = 1.0 / ((1.0 + alpha) * c * sr);
        let r163 = 5000.0 + (1.0 - sustain.clamp(0.0, 1.0).powf(0.05)) * (500_000.0 - 5000.0);
        let r162 = 500.0 + decay.clamp(0.0, 1.0).powi(2) * (100_000.0 - 500.0);
        let r_p1 = 1.0 / (1.0 / r_c + 1.0 / r163);
        let r_s1 = VS_R + r_p1;
        let r_p2 = 1.0 / (1.0 / r162 + 1.0 / r_s1);
        Self { r_c, r162, r163, r_s1, r_p2, b_coef: (1.0 - alpha) / 2.0, a_coef: (1.0 + alpha) / 2.0, ..Self::default() }
    }

    fn process(&mut self, x: f32) -> f32 {
        // Up the tree.
        self.b_vs = x;
        self.c_b = self.b_coef * self.c_b + self.a_coef * self.c_z;
        let g1 = (1.0 / self.r_c) / (1.0 / self.r_c + 1.0 / self.r163);
        // P1: C40 parallel R163 (whose reflection is 0).
        self.b_p1 = 0.0 - g1 * (0.0 - self.c_b);
        // S1: the source in series with P1.
        self.b_s1 = -(self.b_vs + self.b_p1);
        // P2: R162 inverted, parallel S1.
        let g_i = (1.0 / self.r162) / (1.0 / self.r162 + 1.0 / self.r_s1);
        self.b_p2 = self.b_s1 - g_i * (self.b_s1 - 0.0);
        // The diode at the root.
        let a = self.b_p2;
        let r_is = self.r_p2 * DIODE_IS;
        let b_d = a + 2.0 * r_is - 2.0 * DIODE_VT * wright_omega((r_is / DIODE_VT).ln() + (a + r_is) / DIODE_VT);
        // And down again.
        let b2 = b_d + self.b_p2 - self.b_s1;
        let b_i1 = b2 + (self.b_s1 - 0.0);
        let a_r162 = -b_i1;
        let port1 = VS_R / self.r_s1;
        let b1 = self.b_vs - port1 * (b2 + self.b_vs + self.b_p1);
        let x_p1 = -(b2 + b1);
        let b2p = x_p1 + self.b_p1 - 0.0;
        let a_c = b2p + (0.0 - self.c_b);
        self.c_z = a_c;
        // The voltage across R162.
        self.out = a_r162 * 0.5;
        if !self.out.is_finite() {
            *self = Self { c_b: 0.0, c_z: 0.0, ..*self };
            self.out = 0.0;
        }
        self.out
    }
}

/// The settings of a kick.
#[derive(Clone, Copy, Debug)]
pub struct KickKnobs {
    /// Resonance frequency (Hz).
    pub freq: f32,
    /// 0..1: how long it rings (the resonator's damping, inverted).
    pub ring: f32,
    pub q: f32,
    pub tight: f32,
    pub bounce: f32,
    /// Output tone (Hz).
    pub tone: f32,
    pub noise: f32,
    pub width_ms: f32,
    pub pulse_sustain: f32,
    pub pulse_decay: f32,
}

impl Default for KickKnobs {
    fn default() -> Self {
        Self { freq: 60.0, ring: 0.5, q: 0.5, tight: 0.5, bounce: 0.0, tone: 800.0, noise: 0.0, width_ms: 1.0, pulse_sustain: 0.5, pulse_decay: 0.5 }
    }
}

#[derive(Clone, Copy, Default)]
pub struct CircuitKick {
    sr: f32,
    shaper: PulseShaper,
    pulse_left: u32,
    pulse_amp: f32,
    b: [f32; 3],
    a: [f32; 3],
    z: [f32; 3],
    d: [f32; 3],
    noise_e: f32,
    noise_amt: f32,
    noise_lp: f32,
    noise_k: f32,
    rng: u32,
    out_b: f32,
    out_a1: f32,
    out_z: f32,
    dc_x: f32,
    dc_y: f32,
    dc_k: f32,
    quiet: u32,
    age: u32,
    active: bool,
}

impl CircuitKick {
    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn trigger(&mut self, k: KickKnobs, vel: f32, sr: f32) {
        let sr = sr.max(1000.0);
        let fresh = !self.active || self.sr != sr;
        self.sr = sr;
        self.shaper = PulseShaper::new(sr, k.pulse_sustain, k.pulse_decay);
        self.pulse_left = (sr * k.width_ms.clamp(0.025, 2.5) / 1000.0) as u32;
        self.pulse_amp = 0.1 + 1.8 * vel.clamp(0.0, 1.0);
        // The resonator.
        let g = 0.0001 * (0.5f32 / 0.0001).powf((1.0 - k.ring).clamp(0.0, 1.0));
        let wc = k.freq.clamp(20.0, sr * 0.45) * TAU / sr;
        let (ws, wcos) = wc.sin_cos();
        let alpha = ws / (2.0 * k.q.clamp(0.1, 2.0));
        let a0 = (g + 1.0) + alpha * g;
        self.b = [(alpha + 1.0) / a0, -2.0 * wcos / a0, (1.0 - alpha) / a0];
        self.a = [1.0, -2.0 * wcos * (g + 1.0) / a0, ((g + 1.0) - alpha * g) / a0];
        let (t, bo) = (k.tight.clamp(0.0, 1.0), k.bounce.clamp(0.0, 1.0));
        self.d = [4.9 * t.powi(4) + 0.1, 4.9 * t.powi(6) + 0.1, 4.75 * bo.powi(3) + 0.25];
        if fresh {
            self.z = [0.0; 3];
            self.out_z = 0.0;
            self.dc_x = 0.0;
            self.dc_y = 0.0;
        }
        self.noise_amt = k.noise.clamp(0.0, 1.0).powi(2);
        self.noise_e = 2.0 * (1.0 - 0.5f32).powf(2.5) + 1.0;
        self.noise_k = 1.0 - (-TAU * 2000.0 / sr).exp();
        // The output low-pass and its makeup gain.
        let tone = k.tone.clamp(300.0, 7000.0);
        let tone_norm = (tone / 300.0).ln() / (7000.0f32 / 300.0).ln();
        let gain_db = 14.0 * bo.powf(2.5) + (tone_norm - 0.5) * -6.0 + 3.5;
        let gain = 10f32.powf(gain_db / 20.0);
        let c = 1.0 / (TAU * tone / sr / 2.0).tan();
        self.out_b = gain / (c + 1.0);
        self.out_a1 = (1.0 - c) / (c + 1.0);
        self.dc_k = 1.0 - TAU * 10.0 / sr;
        self.quiet = 0;
        self.age = 0;
        self.active = true;
    }

    fn drive(x: f32, d: f32) -> f32 {
        (x * d).tanh() / d
    }

    pub fn tick(&mut self) -> f32 {
        if !self.active {
            return 0.0;
        }
        let pulse = if self.pulse_left > 0 {
            self.pulse_left -= 1;
            self.pulse_amp
        } else {
            0.0
        };
        let x = self.shaper.process(pulse);
        // The resonator (its basic mode).
        let [b0, b1, b2] = self.b;
        let [_, a1, a2] = self.a;
        let [d1, d2, d3] = self.d;
        let y = self.z[1] + x * b0;
        let yd = Self::drive(y, d3);
        self.z[1] = Self::drive(self.z[2] + x * b1 - yd * a1, d1);
        self.z[2] = Self::drive(x * b2 - yd * a2, d2);
        let mut y = y;
        if self.noise_amt > 0.0 {
            self.rng = self.rng.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let n = (self.rng >> 8) as f32 / 8_388_608.0 - 1.0;
            self.noise_lp += (n - self.noise_lp) * self.noise_k;
            y += y.abs().powf(self.noise_e) * self.noise_lp * self.noise_amt;
        }
        // Tone, then the DC block.
        let o = self.out_b * y + self.out_z;
        self.out_z = self.out_b * y - self.out_a1 * o;
        let h = o - self.dc_x + self.dc_k * self.dc_y;
        self.dc_x = o;
        self.dc_y = if h.abs() < 1e-20 { 0.0 } else { h };
        self.age += 1;
        if h.abs() < 3e-4 && self.age as f32 > 0.02 * self.sr {
            self.quiet += 1;
        } else {
            self.quiet = 0;
        }
        if self.quiet as f32 > 0.08 * self.sr || !h.is_finite() {
            self.active = false;
            return 0.0;
        }
        h
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hit(k: KickKnobs, vel: f32, secs: f32) -> Vec<f32> {
        let mut v = CircuitKick::default();
        v.trigger(k, vel, 48000.0);
        (0..(secs * 48000.0) as usize).map(|_| v.tick()).collect()
    }

    #[test]
    fn wright_omega_solves_its_equation() {
        for x in [-10.0f32, -3.0, -1.0, 0.0, 1.0, 5.0, 20.0] {
            let w = wright_omega(x);
            assert!((w + w.ln() - x).abs() < 1e-3, "{x}: {w}");
        }
    }

    #[test]
    fn it_rings_at_its_frequency_and_dies_away() {
        let out = hit(KickKnobs::default(), 1.0, 2.0);
        let peak = out.iter().fold(0.0f32, |m, x| m.max(x.abs()));
        assert!(peak > 0.05 && peak < 3.0, "{peak}");
        assert!(out.iter().all(|x| x.is_finite()));
        // It starts high and falls towards its frequency as it dies away
        // (the clippers in the resonator retune it as it gets quieter).
        let zc: Vec<usize> = (1..out.len()).filter(|&i| out[i - 1] < 0.0 && out[i] >= 0.0).collect();
        let early = 48000.0 / (zc[2] - zc[1]) as f32;
        let late = 48000.0 / (zc[zc.len().min(40) - 1] - zc[zc.len().min(40) - 2]) as f32;
        assert!(early > 2.0 * late, "{early} {late}");
        assert!((55.0..180.0).contains(&late), "{late}");
        let mut v = CircuitKick::default();
        v.trigger(KickKnobs::default(), 1.0, 48000.0);
        for _ in 0..48000 * 4 {
            v.tick();
        }
        assert!(!v.is_active());
    }

    #[test]
    fn ring_and_velocity_matter() {
        let rms = |x: &[f32]| (x.iter().map(|v| v * v).sum::<f32>() / x.len() as f32).sqrt();
        let short = hit(KickKnobs { ring: 0.2, ..KickKnobs::default() }, 1.0, 1.0);
        let long = hit(KickKnobs { ring: 0.8, ..KickKnobs::default() }, 1.0, 1.0);
        assert!(rms(&long[12000..]) > 2.0 * rms(&short[12000..]), "{} {}", rms(&long[12000..]), rms(&short[12000..]));
        let soft = hit(KickKnobs::default(), 0.2, 0.3);
        let hard = hit(KickKnobs::default(), 1.0, 0.3);
        // (velocity drives the diode and the clippers: louder, and harder)
        assert!(rms(&hard) > rms(&soft), "{} {}", rms(&hard), rms(&soft));
    }
}


//! Massive: echoes and reverb as one huge thing. Eight delay lines feed each
//! other; Delay sets how long they are (or the song's tempo does), Warp
//! spreads them apart (turn it while it rings and the echoes bend),
//! Feedback how long it all goes on (all the way up: for ever), Density how
//! soon the echoes blur into a wash. Every mode wires the lines its own way:
//!
//! - *Nebula*: echoes that bloom into a soft wash.
//! - *Comet*: one echo after the other, each a little more smeared.
//! - *Pulsar*: echoes in rhythm, at ratios of the delay (1, 3/2, 2, 3).
//! - *Quasar*: dense from the start, a vast reverb.
//! - *Horizon*: twice as long and slow to build: an endless space.
//! - *Void*: dark and bottomless.
//! - *Drift*: echoes that wander in pitch, chorused.
//! - *Halo*: every pass a little of it an octave up.
//!
//! Low Cut and High Cut work inside the loop, so each pass gets thinner or
//! darker; Mod Rate and Mod Depth move the lines.

use super::verb::{hadamard, householder, soft, Ap, Line};
use super::PitchShifter;
use crate::dsp::effects::FxCtx;
use crate::params::{ParamSpec, Unit};
use std::f32::consts::TAU;

pub const MODE: usize = 0;
pub const DELAY: usize = 1;
pub const SYNC: usize = 2;
pub const WARP: usize = 3;
pub const FEEDBACK: usize = 4;
pub const DENSITY: usize = 5;
pub const LOW_CUT: usize = 6;
pub const HIGH_CUT: usize = 7;
pub const MOD_RATE: usize = 8;
pub const MOD_DEPTH: usize = 9;
pub const WIDTH: usize = 10;
pub const MIX: usize = 11;

pub const MODES: &[&str] = &["Nebula", "Comet", "Pulsar", "Quasar", "Horizon", "Void", "Drift", "Halo"];
pub const SYNCS: &[&str] = &["Off", "1/32", "1/16", "1/16 D", "1/8 T", "1/8", "1/8 D", "1/4 T", "1/4", "1/4 D", "1/2", "1 Bar"];
const SYNC_BEATS: [f32; 12] = [0.0, 0.125, 0.25, 0.375, 1.0 / 3.0, 0.5, 0.75, 2.0 / 3.0, 1.0, 1.5, 2.0, 4.0];

const NEBULA: usize = 0;
const COMET: usize = 1;
const PULSAR: usize = 2;
const QUASAR: usize = 3;
const HORIZON: usize = 4;
const VOID: usize = 5;
const DRIFT: usize = 6;
const HALO: usize = 7;

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::choice("Mode", "Massive", MODES, 0),
    ParamSpec::log("Delay", "Massive", 10.0, 2000.0, 300.0, Unit::Ms),
    ParamSpec::choice("Sync", "Massive", SYNCS, 0),
    ParamSpec::lin("Warp", "Massive", 0.0, 1.0, 0.4, Unit::Percent),
    ParamSpec::lin("Feedback", "Massive", 0.0, 1.0, 0.75, Unit::Percent),
    ParamSpec::lin("Density", "Massive", 0.0, 1.0, 0.6, Unit::Percent),
    ParamSpec::log("Low Cut", "Tone", 10.0, 2000.0, 40.0, Unit::Hz),
    ParamSpec::log("High Cut", "Tone", 500.0, 20000.0, 9000.0, Unit::Hz),
    ParamSpec::log("Mod Rate", "Motion", 0.01, 10.0, 0.3, Unit::Hz),
    ParamSpec::lin("Mod Depth", "Motion", 0.0, 1.0, 0.35, Unit::Percent),
    ParamSpec::lin("Width", "Output", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::lin("Mix", "Output", 0.0, 1.0, 0.5, Unit::Percent),
];

const N: usize = 8;
const MAX_DELAY_S: f32 = 2.0;
/// The longest a line gets (Horizon doubles the delay, the modes stretch
/// it; beyond this it stays here).
const MAX_LINE_S: f32 = 4.0;

/// Each mode's line lengths (relative to Delay) at Warp 0 and at Warp 1.
fn ratios(mode: usize, warp: f32) -> [f32; N] {
    let lerp = |a: [f32; N], b: [f32; N]| std::array::from_fn(|k| a[k] + (b[k] - a[k]) * warp);
    match mode {
        COMET => lerp([1.0, 1.003, 1.007, 1.011, 0.997, 0.993, 1.013, 0.989], [1.0, 1.05, 1.11, 1.17, 0.95, 0.9, 1.23, 0.85]),
        PULSAR => lerp([1.0, 1.0, 1.5, 1.5, 2.0, 2.0, 0.75, 0.75], [1.0, 1.01, 1.5, 1.52, 2.0, 2.03, 0.75, 3.0]),
        QUASAR | VOID => lerp([0.31, 0.37, 0.43, 0.53, 0.61, 0.71, 0.83, 1.0], [0.11, 0.19, 0.29, 0.41, 0.53, 0.67, 0.83, 1.0]),
        HORIZON => lerp([1.0, 1.13, 1.27, 1.39, 1.53, 1.67, 1.81, 2.0], [0.6, 0.8, 1.0, 1.25, 1.5, 1.75, 2.0, 2.0]),
        _ => lerp([1.0, 1.02, 1.05, 1.08, 0.97, 0.94, 1.11, 0.91], [0.5, 0.62, 0.75, 0.89, 1.05, 1.22, 1.4, 1.6]),
    }
}

pub struct Massive {
    sr: f32,
    lines: Vec<Line>,
    loop_ap: Vec<Ap>,
    input_ap: [[Ap; 4]; 2],
    lens: [f32; N],
    lp: [f32; N],
    hp: [f32; N],
    phase: [f32; N],
    wander: [f32; N],
    rng: u32,
    shift: PitchShifter,
    halo_lp: f32,
    last: [f32; N],
    started: bool,
}

impl Massive {
    pub fn new(sr: f32) -> Self {
        let len = (MAX_LINE_S * sr) as usize;
        Self {
            sr,
            lines: (0..N).map(|_| Line::new(len)).collect(),
            loop_ap: (0..N).map(|_| Ap::new((0.06 * sr) as usize)).collect(),
            input_ap: std::array::from_fn(|_| std::array::from_fn(|_| Ap::new((0.12 * sr) as usize))),
            lens: [0.0; N],
            lp: [0.0; N],
            hp: [0.0; N],
            phase: std::array::from_fn(|k| k as f32 * 0.137),
            wander: [0.0; N],
            rng: 0x3a55_1e7e,
            shift: PitchShifter::new(sr, 0.2),
            halo_lp: 0.0,
            last: [0.0; N],
            started: false,
        }
    }

    fn random(&mut self) -> f32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        (self.rng >> 8) as f32 / (1u32 << 24) as f32 * 2.0 - 1.0
    }

    pub fn process(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32], ctx: &FxCtx) {
        let n = l.len().min(r.len());
        let sr = self.sr;
        let mode = (p[MODE].round().max(0.0) as usize).min(MODES.len() - 1);
        let sync = (p[SYNC].round().max(0.0) as usize).min(SYNCS.len() - 1);
        let delay_s = if sync > 0 { SYNC_BEATS[sync] * 60.0 / ctx.bpm.max(20.0) } else { p[DELAY] * 0.001 };
        let delay = delay_s.clamp(0.005, MAX_DELAY_S) * sr * if mode == HORIZON { 2.0 } else { 1.0 };
        let warp = p[WARP].clamp(0.0, 1.0);
        let fb = p[FEEDBACK].clamp(0.0, 1.0);
        // All the way up holds it; below, a gentle curve so the top of the
        // knob is where the long tails are.
        let g = if fb >= 0.995 { 1.0 } else { 1.0 - (1.0 - fb).powf(1.6) };
        let density = p[DENSITY].clamp(0.0, 1.0) * if matches!(mode, QUASAR | VOID) { 1.0 } else { 0.85 };
        let pole = |hz: f32| 1.0 - (-TAU * hz.clamp(10.0, 0.45 * sr) / sr).exp();
        let high = pole(p[HIGH_CUT] * if mode == VOID { 0.35 } else { 1.0 });
        let low = pole(p[LOW_CUT]);
        let rate = p[MOD_RATE].clamp(0.001, 20.0);
        let mut depth = p[MOD_DEPTH].clamp(0.0, 1.0) * (0.004 * sr).min(0.08 * delay);
        if mode == DRIFT {
            depth *= 2.5;
        }
        let width = p[WIDTH].clamp(0.0, 1.0);
        let mix = p[MIX].clamp(0.0, 1.0);
        let want = ratios(mode, warp).map(|q| (q * delay).min(self.lines[0].max_delay() - depth - 4.0));
        if !self.started {
            self.lens = want;
            self.started = true;
        }
        // Lengths glide to where they are going: a turn of Delay or Warp bends the echoes.
        let glide = 1.0 - (-1.0 / (0.12 * sr)).exp();
        let ap_g = 0.15 + 0.55 * density;
        let in_g = 0.35 + 0.35 * density;
        let window = 0.08 * sr;
        for k in 0..N {
            if mode == DRIFT && self.random() > 0.9 {
                self.wander[k] = (self.wander[k] + 0.3 * self.random()).clamp(-1.0, 1.0);
            }
        }
        for i in 0..n {
            let (xl, xr) = (l[i], r[i]);
            // Smear the input as much as Density says (dry at none, so the
            // first echo comes on time).
            let mut inp = [xl, xr];
            for c in 0..2 {
                let s = if c == 1 { 1.09 } else { 1.0 };
                let scale = (delay / sr).clamp(0.02, 0.4) / 0.4;
                let mut v = inp[c];
                for (k, ap) in self.input_ap[c].iter_mut().enumerate() {
                    v = ap.tick(v, [3.1, 5.3, 8.9, 14.1][k] * 0.001 * sr * s * (0.4 + 0.6 * scale) * if matches!(mode, QUASAR | VOID | HORIZON) { 3.0 } else { 1.0 }, in_g);
                }
                inp[c] += density.min(1.0) * (v - inp[c]);
            }
            let mut halo = 0.0;
            if mode == HALO {
                let mono = self.last.iter().sum::<f32>() * 0.18;
                let up = self.shift.tick(mono, 2.0, window);
                self.halo_lp += 0.2 * (up - self.halo_lp);
                halo = 0.3 * self.halo_lp;
            }
            let mut x = [0.0f32; N];
            for k in 0..N {
                self.lens[k] += (want[k] - self.lens[k]) * glide;
                self.phase[k] += rate * (1.0 + 0.13 * k as f32) / sr;
                if self.phase[k] >= 1.0 {
                    self.phase[k] -= 1.0;
                }
                let m = depth * ((TAU * self.phase[k]).sin() + if mode == DRIFT { self.wander[k] } else { 0.0 });
                // The line and its all-pass together are as long as asked.
                let ap_len = ((5.0 + 3.7 * k as f32) * 0.001 * sr).min(0.25 * self.lens[k]);
                let mut v = self.lines[k].read_cubic((self.lens[k] - ap_len).max(2.0) + depth * 2.0 + m);
                v = self.loop_ap[k].tick(v, ap_len, ap_g);
                // Inside the loop: darker and thinner each pass.
                self.lp[k] += high * (v - self.lp[k]);
                self.hp[k] += low * (self.lp[k] - self.hp[k]);
                x[k] = self.lp[k] - self.hp[k];
            }
            self.last = x;
            match mode {
                // A chain: each line feeds the next, turned a little
                // towards the line across (a rotation keeps the energy).
                COMET | PULSAR => {
                    let y = x;
                    for k in 0..N {
                        x[k] = y[(k + N - 1) % N];
                    }
                    let (s, c) = 0.3f32.sin_cos();
                    for k in 0..N / 2 {
                        let (a, b) = (x[k], x[k + N / 2]);
                        x[k] = c * a - s * b;
                        x[k + N / 2] = s * a + c * b;
                    }
                }
                NEBULA | DRIFT => householder(&mut x),
                _ => hadamard(&mut x),
            }
            for k in 0..N {
                let feed = if k % 2 == 0 { inp[0] } else { inp[1] };
                let feed = if matches!(mode, COMET | PULSAR) && k > 1 { 0.0 } else { feed };
                self.lines[k].write(soft(g * x[k] + 0.6 * feed + halo));
            }
            let (mut ol, mut or) = (0.0f32, 0.0f32);
            for k in 0..N / 2 {
                ol += self.last[2 * k];
                or += self.last[2 * k + 1];
            }
            ol *= 0.55;
            or *= 0.55;
            let mid = 0.5 * (ol + or);
            let side = 0.5 * (ol - or) * width;
            l[i] = xl * (1.0 - mix) + (mid + side) * mix;
            r[i] = xr * (1.0 - mix) + (mid - side) * mix;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_util::*;
    use super::*;

    fn run(mode: usize, secs: f32, f: impl Fn(&mut Vec<f32>)) -> Vec<f32> {
        let mut p = crate::params::defaults(SPECS);
        p[MODE] = mode as f32;
        p[MIX] = 1.0;
        f(&mut p);
        let mut m = Massive::new(48000.0);
        let n = (secs * 48000.0) as usize;
        let mut l = vec![0.0; n];
        for v in l[..480].iter_mut() {
            *v = 0.5;
        }
        let mut r = l.clone();
        for (cl, cr) in l.chunks_mut(256).zip(r.chunks_mut(256)) {
            m.process(&p, cl, cr, &ctx());
        }
        l
    }

    #[test]
    fn every_mode_sounds_and_stays_bounded() {
        for mode in 0..MODES.len() {
            let l = run(mode, 6.0, |p| p[FEEDBACK] = 0.8);
            assert!(l.iter().all(|v| v.is_finite() && v.abs() < 4.0), "{}", MODES[mode]);
            assert!(rms(&l[4800..96000]) > 1e-3, "{}", MODES[mode]);
        }
    }

    #[test]
    fn full_feedback_holds_and_none_dies_at_once() {
        let held = run(QUASAR, 8.0, |p| {
            p[FEEDBACK] = 1.0;
            p[HIGH_CUT] = 20000.0;
            p[LOW_CUT] = 10.0;
        });
        assert!(rms(&held[7 * 48000..]) > 0.2 * rms(&held[48000..2 * 48000]));
        let dry = run(COMET, 3.0, |p| p[FEEDBACK] = 0.0);
        assert!(rms(&dry[2 * 48000..]) < 1e-4);
    }

    #[test]
    fn comet_echoes_at_the_delay() {
        let l = run(COMET, 1.5, |p| {
            p[DELAY] = 250.0;
            p[WARP] = 0.0;
            p[DENSITY] = 0.0;
            p[MOD_DEPTH] = 0.0;
            p[FEEDBACK] = 0.5;
        });
        // The first echo: about 250 ms after the click.
        let peak = (6000..18000).max_by(|&a, &b| l[a].abs().total_cmp(&l[b].abs())).unwrap();
        assert!((peak as f32 / 48.0 - 250.0).abs() < 15.0, "first echo at {} ms", peak as f32 / 48.0);
    }
}

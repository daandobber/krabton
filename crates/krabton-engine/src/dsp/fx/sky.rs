//! Sky: a big, thick reverb with eight types on one sixteen-line feedback
//! delay network. Every line is modulated, has an all-pass of its own for
//! density, and loses its highs and lows at the rates Decay, Low and High
//! set; a Hadamard matrix mixes them. Around that core each type does its
//! own thing, with Character as its own knob:
//!
//! - *Room*: short lines, early reflections (Character: how many).
//! - *Plate*: dense at once and bright (Character: density).
//! - *Hall*: long lines, early reflections (Character: how many).
//! - *Cloud*: the input smeared through long all-passes first, huge
//!   (Character: the smear).
//! - *Bloom*: a feedback diffuser in front, so the reverb swells into being
//!   after each sound (Character: how long it blooms).
//! - *Chorale*: the tail sings through three vowel formants (Character:
//!   the vowel, from a to u).
//! - *Swell*: every new sound fades into the reverb, as if the volume
//!   pedal were rolled up (Character: how slowly).
//! - *Magneto*: a four-head tape echo feeding the reverb (Character: the
//!   time between the heads).
//!
//! Shimmer puts an octave up in the loop; Freeze holds the tail.

use super::verb::{hadamard, soft, spread, Ap, Line};
use super::{Follower, PitchShifter};
use crate::dsp::effects::FxCtx;
use crate::dsp::{OnePole, Svf};
use crate::params::{ParamSpec, Unit};
use std::f32::consts::TAU;

pub const TYPE: usize = 0;
pub const DECAY: usize = 1;
pub const PREDELAY: usize = 2;
pub const SIZE: usize = 3;
pub const CHARACTER: usize = 4;
pub const LOW: usize = 5;
pub const HIGH: usize = 6;
pub const MODULATION: usize = 7;
pub const SHIMMER: usize = 8;
pub const LOW_CUT: usize = 9;
pub const FREEZE: usize = 10;
pub const WIDTH: usize = 11;
pub const MIX: usize = 12;

pub const TYPES: &[&str] = &["Room", "Plate", "Hall", "Cloud", "Bloom", "Chorale", "Swell", "Magneto"];
const ROOM: usize = 0;
const PLATE: usize = 1;
const HALL: usize = 2;
const CLOUD: usize = 3;
const BLOOM: usize = 4;
const CHORALE: usize = 5;
const SWELL: usize = 6;
const MAGNETO: usize = 7;

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::choice("Type", "Sky", TYPES, 2),
    ParamSpec::log("Decay", "Sky", 0.3, 60.0, 5.0, Unit::Sec),
    ParamSpec::lin("Pre-Delay", "Sky", 0.0, 500.0, 25.0, Unit::Ms),
    ParamSpec::lin("Size", "Sky", 0.0, 1.0, 0.6, Unit::Percent),
    ParamSpec::lin("Character", "Sky", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Low", "Tone", 0.25, 2.0, 1.1, Unit::None),
    ParamSpec::log("High", "Tone", 800.0, 20000.0, 7000.0, Unit::Hz),
    ParamSpec::lin("Modulation", "Sky", 0.0, 1.0, 0.45, Unit::Percent),
    ParamSpec::lin("Shimmer", "Sky", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::log("Low Cut", "Tone", 20.0, 1000.0, 60.0, Unit::Hz),
    ParamSpec::toggle("Freeze", "Sky", false),
    ParamSpec::lin("Width", "Output", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::lin("Mix", "Output", 0.0, 1.0, 0.35, Unit::Percent),
];

const N: usize = 16;
/// How big each type's lines are, and how strongly its all-passes diffuse.
const TYPE_SIZE: [f32; 8] = [0.35, 0.6, 1.0, 1.5, 1.0, 1.0, 1.0, 0.9];
const TYPE_DIFFUSE: [f32; 8] = [0.55, 0.72, 0.62, 0.7, 0.65, 0.62, 0.62, 0.6];
/// Vowel formants (Hz) for Chorale: a, e, i, o, u.
const VOWELS: [[f32; 3]; 5] = [[800.0, 1150.0, 2900.0], [400.0, 1600.0, 2700.0], [300.0, 2200.0, 3000.0], [450.0, 800.0, 2830.0], [325.0, 700.0, 2530.0]];

pub struct Sky {
    sr: f32,
    pre: [Line; 2],
    input_ap: [[Ap; 4]; 2],
    smear: [[Ap; 4]; 2],
    bloom: [Line; 2],
    bloom_ap: [[Ap; 3]; 2],
    tape: [Line; 2],
    lines: Vec<Line>,
    loop_ap: Vec<Ap>,
    low: [f32; N],
    damp: [f32; N],
    phase: [f32; N],
    drift: [f32; N],
    drift_to: [f32; N],
    rng: u32,
    shift: PitchShifter,
    shimmer_lp: f32,
    last: [f32; N],
    formants: [[Svf; 3]; 2],
    early: Line,
    swell: Follower,
    swell_fast: Follower,
    swell_gain: f32,
    cut: [OnePole; 2],
}

impl Sky {
    pub fn new(sr: f32) -> Self {
        let ms = |m: f32| (m * 0.001 * sr) as usize + 8;
        let mut base = [0.0f32; N];
        spread(31.0, 157.0, &mut base);
        Self {
            sr,
            pre: [Line::new(ms(510.0)), Line::new(ms(510.0))],
            input_ap: std::array::from_fn(|_| std::array::from_fn(|_| Ap::new(ms(30.0)))),
            smear: std::array::from_fn(|_| std::array::from_fn(|_| Ap::new(ms(120.0)))),
            bloom: [Line::new(ms(200.0)), Line::new(ms(200.0))],
            bloom_ap: std::array::from_fn(|_| std::array::from_fn(|_| Ap::new(ms(60.0)))),
            tape: [Line::new(ms(1700.0)), Line::new(ms(1700.0))],
            // The longest type at the largest size, and room for the modulation.
            lines: base.iter().map(|&m| Line::new(ms(m * 1.5 * 1.6 + 8.0))).collect(),
            loop_ap: (0..N).map(|_| Ap::new(ms(20.0))).collect(),
            low: [0.0; N],
            damp: [0.0; N],
            phase: std::array::from_fn(|k| k as f32 / N as f32),
            drift: [0.0; N],
            drift_to: [0.0; N],
            rng: 0x5c1e_ba5e,
            shift: PitchShifter::new(sr, 0.2),
            shimmer_lp: 0.0,
            last: [0.0; N],
            formants: std::array::from_fn(|_| std::array::from_fn(|_| Svf::new())),
            early: Line::new(ms(90.0)),
            swell: Follower::default(),
            swell_fast: Follower::default(),
            swell_gain: 1.0,
            cut: [OnePole::default(); 2],
        }
    }

    fn random(&mut self) -> f32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        (self.rng >> 8) as f32 / (1u32 << 24) as f32 * 2.0 - 1.0
    }

    pub fn process(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32], _ctx: &FxCtx) {
        let n = l.len().min(r.len());
        let sr = self.sr;
        let ty = (p[TYPE].round().max(0.0) as usize).min(TYPES.len() - 1);
        let ch = p[CHARACTER].clamp(0.0, 1.0);
        let freeze = p[FREEZE] >= 0.5;
        let t60 = p[DECAY].clamp(0.1, 120.0);
        let low_t60 = t60 * p[LOW].clamp(0.1, 4.0);
        let pole = |hz: f32| 1.0 - (-TAU * hz.clamp(20.0, 0.45 * sr) / sr).exp();
        let damp = if freeze { 1.0 } else { pole(p[HIGH]) };
        let split = pole(250.0);
        let size = TYPE_SIZE[ty] * (0.4 + 1.2 * p[SIZE].clamp(0.0, 1.0));
        let depth = p[MODULATION].clamp(0.0, 1.0) * 0.0018 * sr;
        let pre = (p[PREDELAY].clamp(0.0, 500.0) * 0.001 * sr).max(1.0);
        let width = p[WIDTH].clamp(0.0, 1.0);
        let mix = p[MIX].clamp(0.0, 1.0);
        let shimmer = p[SHIMMER].clamp(0.0, 1.0);
        let mut diffuse = TYPE_DIFFUSE[ty];
        if ty == PLATE {
            diffuse = 0.55 + 0.25 * ch;
        }
        for c in self.cut.iter_mut() {
            c.set(p[LOW_CUT].clamp(10.0, 2000.0), sr);
        }
        let mut base = [0.0f32; N];
        spread(31.0, 157.0, &mut base);
        let mut lens = [0.0f32; N];
        let mut gains = [(1.0f32, 1.0f32); N];
        let mut ap_len = [0.0f32; N];
        for k in 0..N {
            lens[k] = base[k] * 0.001 * sr * size;
            ap_len[k] = (3.1 + 0.53 * k as f32) * 0.001 * sr * (0.5 + 0.5 * size.min(1.5));
            // Gain per pass for a fall of 60 dB in T60, lows and highs apart
            // (the line and its all-pass together).
            let pass = lens[k] + ap_len[k];
            gains[k] = if freeze { (1.0, 1.0) } else { (10f32.powf(-3.0 * pass / (low_t60 * sr)), 10f32.powf(-3.0 * pass / (t60 * sr))) };
        }
        // Chorale: the vowel, from a over e, i and o to u.
        if ty == CHORALE {
            let v = ch * 3.999;
            let (i, f) = (v as usize, v.fract());
            for c in 0..2 {
                for k in 0..3 {
                    let hz = VOWELS[i][k] + (VOWELS[i + 1][k] - VOWELS[i][k]) * f;
                    self.formants[c][k].set_q(hz * if c == 1 { 1.02 } else { 1.0 }, 7.0, sr);
                }
            }
        }
        self.swell.set(0.02, 0.4, sr);
        self.swell_fast.set(0.001, 0.05, sr);
        let swell_rate = 1.0 / ((0.15 + 2.5 * ch) * sr);
        let smear_g = 0.35 + 0.4 * ch;
        let bloom_fb = 0.45 + 0.45 * ch;
        let bloom_len = (40.0 + 100.0 * ch) * 0.001 * sr;
        let head = (60.0 + 340.0 * ch) * 0.001 * sr;
        let early_amount = if matches!(ty, ROOM | HALL) { 0.2 + 0.6 * ch } else { 0.0 };
        let in_diffuse = 0.5 + 0.25 * diffuse;
        let window = 0.08 * sr;
        let shimmer_tone = pole(5000.0);
        // A slow random wander on every line on top of its sine.
        for k in 0..N {
            if self.random() > 0.97 {
                self.drift_to[k] = self.random();
            }
        }
        for i in 0..n {
            let (xl, xr) = (l[i], r[i]);
            let mut inp = [0.0f32; 2];
            for c in 0..2 {
                let x = if c == 0 { xl } else { xr };
                self.pre[c].write(x);
                inp[c] = if freeze { 0.0 } else { self.pre[c].read(pre) };
            }
            // Swell: every attack starts the fade-in again.
            if ty == SWELL {
                let m = 0.5 * (inp[0].abs() + inp[1].abs());
                let slow = self.swell.tick(m);
                let fast = self.swell_fast.tick(m);
                if fast > 2.0 * slow + 1e-3 {
                    self.swell_gain = 0.0;
                }
                self.swell_gain = (self.swell_gain + swell_rate).min(1.0);
                let g = self.swell_gain * self.swell_gain;
                inp[0] *= g;
                inp[1] *= g;
            }
            // Magneto: four heads, the echoes into the reverb.
            if ty == MAGNETO {
                for c in 0..2 {
                    self.tape[c].write(inp[c]);
                    let mut e = 0.0;
                    for h in 1..=4 {
                        e += self.tape[c].read(head * h as f32 * if c == 1 { 1.01 } else { 1.0 }) * (1.05 - 0.2 * h as f32);
                    }
                    inp[c] = 0.4 * inp[c] + 0.5 * e;
                }
            }
            // Diffusion, the type's own in front.
            for c in 0..2 {
                let mut v = inp[c];
                let s = if c == 1 { 1.11 } else { 1.0 };
                for (k, ap) in self.input_ap[c].iter_mut().enumerate() {
                    v = ap.tick(v, [4.7, 3.6, 12.7, 9.3][k] * 0.001 * sr * s, in_diffuse);
                }
                if ty == CLOUD {
                    for (k, ap) in self.smear[c].iter_mut().enumerate() {
                        v = ap.tick(v, [41.0, 59.0, 83.0, 107.0][k] * 0.001 * sr * s, smear_g);
                    }
                }
                if ty == BLOOM {
                    // A loop of all-passes: the density grows with each pass.
                    let back = self.bloom[c].read(bloom_len * s);
                    let mut b = v + bloom_fb * back;
                    for (k, ap) in self.bloom_ap[c].iter_mut().enumerate() {
                        b = ap.tick(b, [13.3, 23.9, 37.1][k] * 0.001 * sr * s, 0.6);
                    }
                    self.bloom[c].write(soft(b));
                    v = 0.25 * v + 0.75 * back;
                }
                inp[c] = v;
            }
            // Shimmer: the tail an octave up, back into the loop.
            let mut sh = 0.0;
            if shimmer > 0.0 {
                let mono = (self.last.iter().step_by(2).sum::<f32>()) * 0.25;
                let up = self.shift.tick(mono, 2.0, window);
                self.shimmer_lp += shimmer_tone * (up - self.shimmer_lp);
                sh = self.shimmer_lp * shimmer * 0.5;
            }
            // The network.
            let mut x = [0.0f32; N];
            for k in 0..N {
                self.phase[k] += (0.07 + 0.061 * k as f32) / sr;
                if self.phase[k] >= 1.0 {
                    self.phase[k] -= 1.0;
                }
                self.drift[k] += (self.drift_to[k] - self.drift[k]) * 2e-5;
                let m = depth * (0.6 * (TAU * self.phase[k]).sin() + 0.4 * self.drift[k]);
                let mut v = self.lines[k].read(lens[k] + depth * 1.2 + m);
                v = self.loop_ap[k].tick(v, ap_len[k], diffuse * 0.75);
                // Lows and highs lose at their own rates, then the damping.
                self.low[k] += split * (v - self.low[k]);
                let (gl, gh) = gains[k];
                let shaped = self.low[k] * gl + (v - self.low[k]) * gh;
                self.damp[k] += damp * (shaped - self.damp[k]);
                x[k] = self.damp[k];
            }
            self.last = x;
            hadamard(&mut x);
            for k in 0..N {
                let feed = if k % 2 == 0 { inp[0] } else { inp[1] } * if k % 4 < 2 { 1.0 } else { -1.0 };
                self.lines[k].write(soft(x[k] + 0.35 * feed + sh));
            }
            // Out: half the lines each side.
            let (mut ol, mut or) = (0.0f32, 0.0f32);
            for k in 0..N / 2 {
                let s = if k % 2 == 0 { 1.0 } else { -1.0 };
                ol += s * self.last[2 * k];
                or += s * self.last[2 * k + 1];
            }
            ol *= 0.5;
            or *= 0.5;
            if early_amount > 0.0 {
                let e = 0.5 * (inp[0] + inp[1]);
                self.early.write(e);
                let room = TYPE_SIZE[ty] * (0.5 + p[SIZE]);
                let taps = [(7.1, 0.7, -0.6), (11.3, -0.55, 0.65), (17.9, 0.5, 0.4), (23.3, -0.4, -0.45), (31.7, 0.35, 0.3), (41.9, -0.25, 0.3)];
                for (t, gl, gr) in taps {
                    let v = self.early.read(t * room * 0.001 * sr);
                    ol += early_amount * gl * v;
                    or += early_amount * gr * v;
                }
            }
            if ty == CHORALE {
                let mut fl = 0.0;
                let mut fr = 0.0;
                for k in 0..3 {
                    let w = [1.0, 0.7, 0.45][k];
                    fl += w * self.formants[0][k].tick(ol).band;
                    fr += w * self.formants[1][k].tick(or).band;
                }
                ol = 0.4 * ol + 1.4 * fl;
                or = 0.4 * or + 1.4 * fr;
            }
            ol = self.cut[0].hp(ol);
            or = self.cut[1].hp(or);
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

    fn impulse_response(ty: usize, secs: f32, f: impl Fn(&mut Vec<f32>)) -> (Vec<f32>, Vec<f32>) {
        let mut p = crate::params::defaults(SPECS);
        p[TYPE] = ty as f32;
        p[MIX] = 1.0;
        p[PREDELAY] = 0.0;
        f(&mut p);
        let mut sky = Sky::new(48000.0);
        let n = (secs * 48000.0) as usize;
        let mut l = vec![0.0; n];
        let mut r = vec![0.0; n];
        for k in 0..2400 {
            l[k] = if k < 48 { 0.5 } else { 0.0 };
            r[k] = l[k];
        }
        for (cl, cr) in l.chunks_mut(256).zip(r.chunks_mut(256)) {
            sky.process(&p, cl, cr, &ctx());
        }
        (l, r)
    }

    #[test]
    fn every_type_rings_and_dies_away() {
        // (Swell swallows a lone click: it is tried with a sound that lasts.)
        for ty in (0..TYPES.len()).filter(|&t| t != SWELL) {
            let (l, r) = impulse_response(ty, 6.0, |p| p[DECAY] = 1.5);
            assert!(l.iter().chain(&r).all(|v| v.is_finite()), "{}", TYPES[ty]);
            let early = rms(&l[4800..48000]);
            let late = rms(&l[5 * 48000..]);
            assert!(early > 1e-3, "{}: {early}", TYPES[ty]);
            assert!(late < early * 0.05, "{}: {early} then {late}", TYPES[ty]);
            // Wide: the two sides differ.
            let diff = rms(&l[4800..48000].iter().zip(&r[4800..48000]).map(|(a, b)| a - b).collect::<Vec<_>>());
            assert!(diff > early * 0.2, "{} is not stereo", TYPES[ty]);
        }
    }

    #[test]
    fn a_long_decay_rings_long_and_freeze_holds() {
        let (l, _) = impulse_response(HALL, 8.0, |p| p[DECAY] = 20.0);
        assert!(rms(&l[7 * 48000..]) > 0.1 * rms(&l[48000..2 * 48000]));
        // A short decay, frozen after half a second: it keeps ringing.
        let mut p = crate::params::defaults(SPECS);
        p[MIX] = 1.0;
        p[DECAY] = 1.0;
        let mut sky = Sky::new(48000.0);
        let mut l = tone(220.0, 4.0, 0.3);
        let mut r = l.clone();
        for (k, (cl, cr)) in l.chunks_mut(256).zip(r.chunks_mut(256)).enumerate() {
            p[FREEZE] = if k * 256 > 24000 { 1.0 } else { 0.0 };
            sky.process(&p, cl, cr, &ctx());
        }
        assert!(rms(&l[3 * 48000..]) > 0.3 * rms(&l[12000..24000]), "{} {}", rms(&l[3 * 48000..]), rms(&l[12000..24000]));
    }

    #[test]
    fn shimmer_stays_stable() {
        let (l, _) = impulse_response(CLOUD, 10.0, |p| {
            p[DECAY] = 30.0;
            p[SHIMMER] = 1.0;
        });
        assert!(l.iter().all(|v| v.is_finite() && v.abs() < 4.0));
    }

    #[test]
    fn swell_fades_in() {
        let mut p = crate::params::defaults(SPECS);
        p[TYPE] = SWELL as f32;
        p[MIX] = 1.0;
        p[PREDELAY] = 0.0;
        p[CHARACTER] = 0.6;
        let mut sky = Sky::new(48000.0);
        let mut l = vec![0.0; 2 * 48000];
        let src = tone(220.0, 1.5, 0.3);
        l[4800..4800 + src.len()].copy_from_slice(&src);
        let mut r = l.clone();
        for (cl, cr) in l.chunks_mut(256).zip(r.chunks_mut(256)) {
            sky.process(&p, cl, cr, &ctx());
        }
        // Quiet just after the sound starts, full a second later.
        let (soon, later) = (rms(&l[4800..9600]), rms(&l[48000..52800]));
        assert!(soon < 0.3 * later, "{soon} {later}");
    }
}

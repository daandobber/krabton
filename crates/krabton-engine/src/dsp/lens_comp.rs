//! Lens Comp: a compressor with character styles, soft knee, range,
//! lookahead, hold, a sidechain high-pass, stereo link, dry/wet and auto
//! gain. It publishes its gain reduction for the level display.
//!
//! Styles change the detector and the timing:
//! - Clean: feed-forward peak detection, plain attack and release.
//! - Classic: feedback (it listens to its own output), like old hardware.
//! - Opto: release that slows down the longer and deeper it compresses.
//! - Vocal: RMS detection and a wide knee, for an even voice.
//! - Mastering: slow RMS detection, gentle and transparent.
//! - Bus: feedback with a softer attack, the "glue" of a mix bus.
//! - Punch: a slow attack that lets transients through, quick release.
//! - Pumping: a release that falls linearly in dB, so you hear it breathe.

use super::biquad::{self, Coefs, State};
use crate::params::{ParamSpec, Unit};
use crate::scope::DeviceTaps;

pub const STYLE: usize = 0;
pub const THRESHOLD: usize = 1;
pub const RATIO: usize = 2;
pub const KNEE: usize = 3;
pub const RANGE: usize = 4;
pub const ATTACK: usize = 5;
pub const RELEASE: usize = 6;
pub const AUTO_RELEASE: usize = 7;
pub const LOOKAHEAD: usize = 8;
pub const HOLD: usize = 9;
pub const SC_HPF: usize = 10;
pub const LINK: usize = 11;
pub const MIX: usize = 12;
pub const OUTPUT: usize = 13;
pub const AUTO_GAIN: usize = 14;

pub const STYLES: &[&str] = &["Clean", "Classic", "Opto", "Vocal", "Mastering", "Bus", "Punch", "Pumping"];

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::choice("Style", "Character", STYLES, 0),
    ParamSpec::lin("Threshold", "Dynamics", -60.0, 0.0, -18.0, Unit::Db),
    ParamSpec::log("Ratio", "Dynamics", 1.0, 100.0, 4.0, Unit::Ratio),
    ParamSpec::lin("Knee", "Dynamics", 0.0, 72.0, 6.0, Unit::Db),
    ParamSpec::lin("Range", "Dynamics", 0.0, 60.0, 60.0, Unit::Db),
    ParamSpec::log("Attack", "Timing", 0.005, 250.0, 10.0, Unit::Ms),
    ParamSpec::log("Release", "Timing", 5.0, 2500.0, 150.0, Unit::Ms),
    ParamSpec::toggle("Auto Release", "Timing", false),
    ParamSpec::lin("Lookahead", "Timing", 0.0, 20.0, 0.0, Unit::Ms),
    ParamSpec::lin("Hold", "Timing", 0.0, 500.0, 0.0, Unit::Ms),
    ParamSpec::log("SC High-Pass", "Sidechain", 20.0, 2000.0, 20.0, Unit::Hz),
    ParamSpec::lin("Stereo Link", "Sidechain", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::lin("Mix", "Output", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::lin("Output", "Output", -36.0, 36.0, 0.0, Unit::Db),
    ParamSpec::toggle("Auto Gain", "Output", false),
];

/// Meter values published for the display.
pub const M_GR: usize = 0;
pub const M_IN: usize = 1;
pub const M_OUT: usize = 2;
pub const M_MAKEUP: usize = 3;

#[derive(Clone, Copy)]
struct Style {
    feedback: bool,
    /// RMS window in ms; 0 = peak.
    rms_ms: f32,
    attack_mul: f32,
    release_mul: f32,
    min_knee: f32,
    program_release: bool,
    linear_release: bool,
}

const STYLE_TABLE: [Style; 8] = [
    Style { feedback: false, rms_ms: 0.0, attack_mul: 1.0, release_mul: 1.0, min_knee: 0.0, program_release: false, linear_release: false },
    Style { feedback: true, rms_ms: 0.0, attack_mul: 1.0, release_mul: 1.0, min_knee: 3.0, program_release: false, linear_release: false },
    Style { feedback: false, rms_ms: 2.0, attack_mul: 2.0, release_mul: 1.0, min_knee: 12.0, program_release: true, linear_release: false },
    Style { feedback: false, rms_ms: 5.0, attack_mul: 1.0, release_mul: 1.0, min_knee: 18.0, program_release: true, linear_release: false },
    Style { feedback: false, rms_ms: 20.0, attack_mul: 1.5, release_mul: 1.5, min_knee: 6.0, program_release: false, linear_release: false },
    Style { feedback: true, rms_ms: 0.0, attack_mul: 1.5, release_mul: 1.0, min_knee: 6.0, program_release: false, linear_release: false },
    Style { feedback: false, rms_ms: 0.0, attack_mul: 3.0, release_mul: 0.6, min_knee: 3.0, program_release: false, linear_release: false },
    Style { feedback: false, rms_ms: 0.0, attack_mul: 1.0, release_mul: 1.0, min_knee: 0.0, program_release: false, linear_release: true },
];

/// Gain change in dB (≤ 0) for a level `x` dB.
pub fn gain_computer(x: f32, threshold: f32, ratio: f32, knee: f32) -> f32 {
    let slope = 1.0 / ratio.max(1.0) - 1.0;
    let over = x - threshold;
    if knee > 0.0 && 2.0 * over.abs() <= knee {
        slope * (over + knee * 0.5).powi(2) / (2.0 * knee)
    } else if over > 0.0 {
        slope * over
    } else {
        0.0
    }
}

const MAX_LOOKAHEAD: usize = 4096;

pub struct LensCompState {
    sc_hp: [State; 2],
    sc_coefs: Coefs,
    sc_freq: f32,
    rms: [f32; 2],
    /// Smoothed gain change in dB per channel (≤ 0).
    gr: [f32; 2],
    hold: [u32; 2],
    /// How long and deep compression has been going (for program release).
    memory: f32,
    last_out: [f32; 2],
    delay: [Vec<f32>; 2],
    pos: usize,
    makeup: f32,
    pub taps: DeviceTaps,
}

impl Default for LensCompState {
    fn default() -> Self {
        Self {
            sc_hp: [State::default(); 2],
            sc_coefs: Coefs::IDENTITY,
            sc_freq: 0.0,
            rms: [0.0; 2],
            gr: [0.0; 2],
            hold: [0; 2],
            memory: 0.0,
            last_out: [0.0; 2],
            delay: [vec![0.0; MAX_LOOKAHEAD], vec![0.0; MAX_LOOKAHEAD]],
            pos: 0,
            makeup: 1.0,
            taps: DeviceTaps::full(),
        }
    }
}

#[inline]
fn db(x: f32) -> f32 {
    20.0 * x.max(1e-9).log10()
}

impl LensCompState {
    pub fn process(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32], sr: f32) {
        if p.len() < SPECS.len() {
            return;
        }
        let n = l.len().min(r.len());
        let style = STYLE_TABLE[(p[STYLE].round().max(0.0) as usize).min(STYLE_TABLE.len() - 1)];
        let threshold = p[THRESHOLD];
        let ratio = p[RATIO].max(1.0);
        let knee = p[KNEE].max(style.min_knee);
        let range = p[RANGE].max(0.0);
        let coef = |ms: f32| 1.0 - (-1.0 / (ms.max(0.001) * 0.001 * sr)).exp();
        let att = coef(p[ATTACK] * style.attack_mul);
        let release_ms = p[RELEASE] * style.release_mul;
        let auto_release = p[AUTO_RELEASE] >= 0.5 || style.program_release;
        let hold = (p[HOLD].max(0.0) * 0.001 * sr) as u32;
        let la = ((p[LOOKAHEAD].clamp(0.0, 20.0) * 0.001 * sr) as usize).min(MAX_LOOKAHEAD - 1);
        let link = p[LINK].clamp(0.0, 1.0);
        let mix = p[MIX].clamp(0.0, 1.0);
        let rms_k = if style.rms_ms > 0.0 { coef(style.rms_ms) } else { 1.0 };
        // Auto gain: make up half of the reduction a full-scale signal gets.
        let auto = if p[AUTO_GAIN] >= 0.5 { -0.5 * gain_computer(0.0, threshold, ratio, knee).max(-range) } else { 0.0 };
        let makeup_target = 10f32.powf((p[OUTPUT] + auto) / 20.0);
        let hp = p[SC_HPF];
        if (hp - self.sc_freq).abs() > 0.01 {
            self.sc_freq = hp;
            self.sc_coefs = biquad::highpass(hp as f64, sr as f64, 0.7071);
        }
        let use_hp = hp > 21.0;
        let mem_k = coef(1000.0);
        let (mut in_pk, mut out_pk, mut gr_pk) = (0.0f32, 0.0f32, 0.0f32);
        let makeup_step = (makeup_target - self.makeup) / n.max(1) as f32;
        for i in 0..n {
            let x = [l[i], r[i]];
            in_pk = in_pk.max(x[0].abs()).max(x[1].abs());
            // Detector input: the signal, or what came out last time.
            let mut lvl = [0.0f32; 2];
            for c in 0..2 {
                let mut s = if style.feedback { self.last_out[c] } else { x[c] };
                if use_hp {
                    s = self.sc_hp[c].run(&self.sc_coefs, s as f64) as f32;
                }
                lvl[c] = if style.rms_ms > 0.0 {
                    self.rms[c] += (s * s - self.rms[c]) * rms_k;
                    db((2.0 * self.rms[c]).sqrt())
                } else {
                    db(s.abs())
                };
            }
            let linked = lvl[0].max(lvl[1]);
            // Program-dependent release: up to 4x slower after long, deep work.
            let rel_ms = if auto_release { release_ms * (0.4 + 3.6 * self.memory) } else { release_ms };
            let rel = coef(rel_ms);
            for c in 0..2 {
                let level = lvl[c] + (linked - lvl[c]) * link;
                let target = gain_computer(level, threshold, ratio, knee).max(-range);
                let g = &mut self.gr[c];
                if target < *g {
                    *g += (target - *g) * att;
                    self.hold[c] = hold;
                } else if self.hold[c] > 0 {
                    self.hold[c] -= 1;
                } else if style.linear_release {
                    // Falls back by the whole range over the release time.
                    *g = (*g + range.max(6.0) / (rel_ms * 0.001 * sr)).min(target);
                } else {
                    *g += (target - *g) * rel;
                }
            }
            let depth = (-(self.gr[0].min(self.gr[1])) / 12.0).min(1.0);
            self.memory += (depth - self.memory) * mem_k;
            self.makeup += makeup_step;
            for c in 0..2 {
                // Lookahead: the gain meets the audio `la` samples later.
                let buf = &mut self.delay[c];
                buf[self.pos] = x[c];
                let dry = buf[(self.pos + MAX_LOOKAHEAD - la) % MAX_LOOKAHEAD];
                let wet = dry * 10f32.powf(self.gr[c] / 20.0);
                let y = (dry + (wet - dry) * mix) * self.makeup;
                self.last_out[c] = wet;
                if c == 0 {
                    l[i] = y;
                } else {
                    r[i] = y;
                }
                out_pk = out_pk.max(y.abs());
            }
            self.pos = (self.pos + 1) % MAX_LOOKAHEAD;
            gr_pk = gr_pk.min(self.gr[0].min(self.gr[1]));
        }
        self.makeup = makeup_target;
        if let Some(m) = &mut self.taps.meters {
            m.values[M_GR] = gr_pk;
            m.values[M_IN] = db(in_pk);
            m.values[M_OUT] = db(out_pk);
            m.values[M_MAKEUP] = db(makeup_target);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    fn run(p: &[f32], amp: f32, secs: f32) -> (f32, LensCompState) {
        let sr = 48000.0;
        let n = (secs * sr) as usize;
        let mut st = LensCompState::default();
        let mut l: Vec<f32> = (0..n).map(|i| amp * (2.0 * PI * 200.0 * i as f32 / sr).sin()).collect();
        let mut r = l.clone();
        for (cl, cr) in l.chunks_mut(256).zip(r.chunks_mut(256)) {
            st.process(p, cl, cr, sr);
        }
        let out = l[n * 3 / 4..].iter().fold(0.0f32, |m, s| m.max(s.abs()));
        (db(out), st)
    }

    #[test]
    fn the_static_curve_matches_threshold_and_ratio() {
        assert_eq!(gain_computer(-30.0, -20.0, 4.0, 0.0), 0.0);
        assert!((gain_computer(-10.0, -20.0, 4.0, 0.0) + 7.5).abs() < 1e-4);
        // A soft knee meets the hard curve outside the knee and is smooth inside.
        assert!((gain_computer(-14.0, -20.0, 4.0, 6.0) - gain_computer(-14.0, -20.0, 4.0, 0.0)).abs() < 1e-4);
        assert!(gain_computer(-20.0, -20.0, 4.0, 6.0) < 0.0);
    }

    #[test]
    fn every_style_compresses_a_loud_tone_to_the_curve() {
        let mut p = crate::params::defaults(SPECS);
        p[THRESHOLD] = -20.0;
        p[RATIO] = 4.0;
        p[KNEE] = 0.0;
        for style in 0..STYLES.len() {
            p[STYLE] = style as f32;
            let (out, st) = run(&p, 0.5, 2.0); // -6 dBFS peaks
            // A peak detector reaches the static curve: -20 + 14/4 = -16.5.
            // RMS styles measure a bit lower, so they compress a bit less.
            if std::env::var("COMP_REPORT").is_ok() {
                eprintln!("{:<10} out {out:6.1} dB", STYLES[style]);
            }
            assert!(out < -9.0 && out > -19.0, "{}: {out:.1} dB", STYLES[style]);
            let gr = st.taps.meters.as_ref().unwrap().values[M_GR];
            assert!(gr < -3.0, "{}: shows {gr} dB of reduction", STYLES[style]);
        }
        // A quiet tone passes untouched.
        p[STYLE] = 0.0;
        let (out, _) = run(&p, 0.05, 1.0);
        assert!((out - db(0.05)).abs() < 0.1);
    }

    #[test]
    fn range_mix_and_auto_gain() {
        let mut p = crate::params::defaults(SPECS);
        p[THRESHOLD] = -40.0;
        p[RATIO] = 100.0;
        p[RANGE] = 6.0;
        let (out, _) = run(&p, 0.5, 1.0);
        assert!((out - (db(0.5) - 6.0)).abs() < 0.3, "range limits the reduction: {out:.2}");
        p[RANGE] = 60.0;
        p[MIX] = 0.0;
        let (out, _) = run(&p, 0.5, 1.0);
        assert!((out - db(0.5)).abs() < 0.1, "mix 0 is dry");
        p[MIX] = 1.0;
        p[THRESHOLD] = -20.0;
        p[RATIO] = 4.0;
        let (plain, _) = run(&p, 0.5, 1.0);
        p[AUTO_GAIN] = 1.0;
        let (auto, _) = run(&p, 0.5, 1.0);
        assert!(auto > plain + 3.0, "auto gain makes up: {plain:.1} -> {auto:.1}");
    }

    #[test]
    fn lookahead_catches_a_transient() {
        let sr = 48000.0;
        let mut p = crate::params::defaults(SPECS);
        p[THRESHOLD] = -20.0;
        p[RATIO] = 100.0;
        p[ATTACK] = 1.0;
        let hit = |p: &[f32]| {
            let mut st = LensCompState::default();
            let n = 9600;
            let mut l: Vec<f32> = (0..n).map(|i| if i >= 4800 { 0.8 * (2.0 * PI * 100.0 * i as f32 / sr).sin() } else { 0.0 }).collect();
            let mut r = l.clone();
            for (cl, cr) in l.chunks_mut(64).zip(r.chunks_mut(64)) {
                st.process(p, cl, cr, sr);
            }
            l.iter().fold(0.0f32, |m, s| m.max(s.abs()))
        };
        let plain = hit(&p);
        p[LOOKAHEAD] = 5.0;
        let ahead = hit(&p);
        assert!(ahead < plain * 0.7, "lookahead lowers the first peak: {plain:.2} -> {ahead:.2}");
    }
}

//! Tune: pitch correction for one voice. It hears the note (the YIN method,
//! on a copy of the input at a quarter of the rate) and moves it to the nearest
//! note of a key: the song's, or one of its own. With Speed at its lowest the
//! voice jumps from note to note, the hard robot sound; slower, it is steered
//! gently. Shift moves it by semitones as well.
//!
//! The pitch is moved by two read heads running through a delay line, fading
//! into each other, with their span tied to the period of the note so the
//! fades fall alike in every period. No correction, no fade and no comb: the
//! heads settle on the shortest delay.

use crate::dsp::effects::FxCtx;
use crate::params::{ParamSpec, Unit};
use crate::scale::{Scale, ScaleKind};
use std::f32::consts::PI;

pub const SPEED: usize = 0;
pub const AMOUNT: usize = 1;
pub const KEY: usize = 2;
pub const SCALE: usize = 3;
pub const SHIFT: usize = 4;
pub const MIX: usize = 5;

pub const KEYS: &[&str] = &["Song", "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
pub const SCALES: &[&str] = &["Song", "Chromatic", "Major", "Minor", "Major Pentatonic", "Minor Pentatonic", "Blues"];

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::log("Speed", "Tune", 0.001, 0.5, 0.001, Unit::Sec),
    ParamSpec::lin("Amount", "Tune", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::choice("Key", "Key", KEYS, 0),
    ParamSpec::choice("Scale", "Key", SCALES, 0),
    ParamSpec::lin("Shift", "Output", -24.0, 24.0, 0.0, Unit::Semi),
    ParamSpec::lin("Mix", "Output", 0.0, 1.0, 1.0, Unit::Percent),
];

const DECIMATE: usize = 4;
/// Samples at the lower rate: what is compared, and what it is compared with.
const WINDOW: usize = 400;
const MAX_LAG: usize = 220;
const DETECT: usize = WINDOW + MAX_LAG + 1;
/// Lower-rate samples between looks at the pitch.
const HOP: usize = 64;
const DELAY: usize = 1 << 14;

pub struct Tune {
    sr: f32,
    det: [f32; DETECT],
    det_at: usize,
    since: usize,
    acc: f32,
    acc_n: usize,
    /// Scratch for YIN (kept here: nothing allocated while it plays).
    cmnd: [f32; MAX_LAG + 1],
    flat: [f32; DETECT],
    /// The note heard (semitones, MIDI numbering), when one is.
    heard: Option<f32>,
    target: Option<i32>,
    /// The correction now, in semitones.
    now: f32,
    period: f32,
    buf: [Vec<f32>; 2],
    at: usize,
    phase: f32,
    span: f32,
}

/// The nearest note in a key, staying on the last one unless clearly past it.
fn nearest(heard: f32, scale: &Scale, last: Option<i32>) -> i32 {
    let mut best = heard.round() as i32;
    let mut gap = f32::MAX;
    let centre = heard.round() as i32;
    for p in centre - 7..=centre + 7 {
        if (0..=127).contains(&p) && scale.contains(p as u8) {
            let d = (p as f32 - heard).abs();
            if d < gap {
                gap = d;
                best = p;
            }
        }
    }
    if let Some(l) = last {
        // A little stickiness, or a note on the edge flips to and fro.
        if l != best && (0..=127).contains(&l) && scale.contains(l as u8) && (l as f32 - heard).abs() < gap + 0.12 {
            return l;
        }
    }
    best
}

impl Tune {
    pub fn new(sr: f32) -> Self {
        let sr = sr.max(1000.0);
        Self {
            sr,
            det: [0.0; DETECT],
            det_at: 0,
            since: 0,
            acc: 0.0,
            acc_n: 0,
            cmnd: [0.0; MAX_LAG + 1],
            flat: [0.0; DETECT],
            heard: None,
            target: None,
            now: 0.0,
            period: sr / 220.0,
            buf: [vec![0.0; DELAY], vec![0.0; DELAY]],
            at: 0,
            phase: 0.0,
            span: 0.02 * sr,
        }
    }

    /// The YIN difference function on the last stretch at the lower rate.
    fn listen(&mut self) {
        let low_sr = self.sr / DECIMATE as f32;
        for i in 0..DETECT {
            self.flat[i] = self.det[(self.det_at + i) % DETECT];
        }
        let x = &self.flat;
        let energy: f32 = x[..WINDOW].iter().map(|v| v * v).sum::<f32>() / WINDOW as f32;
        if energy < 1e-6 {
            self.heard = None;
            return;
        }
        let min_lag = ((low_sr / 1000.0) as usize).max(2);
        let max_lag = ((low_sr / 55.0) as usize).min(MAX_LAG);
        let mut sum = 0.0f32;
        self.cmnd[0] = 1.0;
        for lag in 1..=max_lag {
            let mut d = 0.0f32;
            for j in 0..WINDOW {
                let e = x[j] - x[j + lag];
                d += e * e;
            }
            sum += d;
            self.cmnd[lag] = if sum > 0.0 { d * lag as f32 / sum } else { 1.0 };
        }
        // The first dip under the threshold, down to its bottom.
        let mut lag = min_lag;
        let mut found = None;
        while lag < max_lag {
            if self.cmnd[lag] < 0.15 {
                while lag + 1 < max_lag && self.cmnd[lag + 1] < self.cmnd[lag] {
                    lag += 1;
                }
                found = Some(lag);
                break;
            }
            lag += 1;
        }
        let Some(lag) = found else {
            self.heard = None;
            return;
        };
        // Between the samples.
        let (a, b, c) = (self.cmnd[lag - 1], self.cmnd[lag], self.cmnd[(lag + 1).min(MAX_LAG)]);
        let bend = a - 2.0 * b + c;
        let exact = lag as f32 + if bend.abs() > 1e-9 { 0.5 * (a - c) / bend } else { 0.0 };
        let hz = low_sr / exact;
        self.period = self.sr / hz;
        self.heard = Some(69.0 + 12.0 * (hz / 440.0).log2());
    }

    pub fn process(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32], ctx: &FxCtx) {
        if p.len() < SPECS.len() {
            return;
        }
        let root = match p[KEY].round() as i32 {
            k if k >= 1 => (k - 1) as u8,
            _ => ctx.scale.root,
        };
        let kind = match p[SCALE].round() as i32 {
            1 => ScaleKind::Chromatic,
            2 => ScaleKind::Major,
            3 => ScaleKind::Minor,
            4 => ScaleKind::MajorPentatonic,
            5 => ScaleKind::MinorPentatonic,
            6 => ScaleKind::Blues,
            _ => ctx.scale.kind,
        };
        let scale = Scale::new(root, kind);
        let amount = p[AMOUNT].clamp(0.0, 1.0);
        let shift = p[SHIFT];
        let mix = p[MIX].clamp(0.0, 1.0);
        let glide = 1.0 - (-2.2 / (p[SPEED].max(0.0005) * self.sr)).exp();
        let n = l.len().min(r.len());
        for i in 0..n {
            let (in_l, in_r) = (l[i], r[i]);
            // Listening, at the lower rate.
            self.acc += 0.5 * (in_l + in_r);
            self.acc_n += 1;
            if self.acc_n == DECIMATE {
                self.det[self.det_at] = self.acc / DECIMATE as f32;
                self.det_at = (self.det_at + 1) % DETECT;
                self.acc = 0.0;
                self.acc_n = 0;
                self.since += 1;
                if self.since >= HOP {
                    self.since = 0;
                    self.listen();
                    match self.heard {
                        Some(h) => self.target = Some(nearest(h, &scale, self.target)),
                        None => self.target = None,
                    }
                }
            }
            let want = match (self.heard, self.target) {
                (Some(h), Some(t)) => (t as f32 - h) * amount,
                // Between notes it does not move what it cannot hear.
                _ => self.now * 0.999,
            };
            self.now += (want - self.now) * glide;
            let semis = self.now + shift;
            let ratio = 2f32.powf(semis / 12.0);

            self.buf[0][self.at] = in_l;
            self.buf[1][self.at] = in_r;
            // The heads: the phase turns as fast as the pitch differs.
            if (ratio - 1.0).abs() < 2e-4 {
                // Nothing to do: settle on the shortest delay, one clean head.
                self.phase -= self.phase.min(0.0004);
            } else {
                self.phase += (1.0 - ratio) / self.span;
            }
            if self.phase >= 1.0 || self.phase < 0.0 {
                self.phase = self.phase.rem_euclid(1.0);
                // A span of some periods, so the fades fall alike in each.
                let periods = (0.018 * self.sr / self.period).ceil().max(2.0);
                self.span = (periods * self.period).clamp(0.012 * self.sr, 0.06 * self.sr);
            }
            let other = (self.phase + 0.5).fract();
            let g1 = (PI * self.phase).cos().powi(2);
            let g2 = 1.0 - g1;
            let read = |buf: &[f32], at: usize, delay: f32| {
                let pos = at as f32 + DELAY as f32 - delay;
                let whole = pos.floor();
                let f = pos - whole;
                let k = whole as usize;
                let a = buf[k % DELAY];
                let b = buf[(k + 1) % DELAY];
                a + (b - a) * f
            };
            let (d1, d2) = (self.phase * self.span + 1.0, other * self.span + 1.0);
            let wet_l = g1 * read(&self.buf[0], self.at, d1) + g2 * read(&self.buf[0], self.at, d2);
            let wet_r = g1 * read(&self.buf[1], self.at, d1) + g2 * read(&self.buf[1], self.at, d2);
            self.at = (self.at + 1) % DELAY;
            l[i] = in_l + (wet_l - in_l) * mix;
            r[i] = in_r + (wet_r - in_r) * mix;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 48000.0;

    fn ctx(scale: Scale) -> FxCtx {
        FxCtx { sr: SR, bpm: 120.0, beat: 0.0, playing: true, scale, note: None }
    }

    fn params(set: &[(usize, f32)]) -> Vec<f32> {
        let mut p = crate::params::defaults(SPECS);
        for &(k, v) in set {
            p[k] = v;
        }
        p
    }

    /// A voice-like tone: a note with its harmonics, for a second.
    fn tone(hz: f32) -> Vec<f32> {
        (0..SR as usize)
            .map(|i| {
                let t = i as f32 / SR;
                let w = 2.0 * PI * hz * t;
                0.3 * (w.sin() + 0.5 * (2.0 * w).sin() + 0.25 * (3.0 * w).sin())
            })
            .collect()
    }

    fn run(mut x: Vec<f32>, p: &[f32], scale: Scale) -> Vec<f32> {
        let mut t = Tune::new(SR);
        let mut r = x.clone();
        for (a, b) in x.chunks_mut(256).zip(r.chunks_mut(256)) {
            t.process(p, a, b, &ctx(scale));
        }
        x
    }

    /// The strongest frequency near `around`, in the last half.
    fn pitch(x: &[f32], around: f32) -> f32 {
        let s = &x[x.len() / 2..];
        let mut best = (0.0, 0.0);
        let mut f = around * 0.8;
        while f < around * 1.25 {
            let (mut re, mut im) = (0.0f32, 0.0f32);
            for (i, v) in s.iter().enumerate() {
                let w = 2.0 * PI * f * i as f32 / SR;
                re += v * w.cos();
                im += v * w.sin();
            }
            let m = re * re + im * im;
            if m > best.0 {
                best = (m, f);
            }
            f += 0.5;
        }
        best.1
    }

    #[test]
    fn a_flat_note_comes_up_to_the_note() {
        // 430 Hz is 40 cents under A.
        let out = run(tone(430.0), &params(&[(SCALE, 1.0)]), Scale::new(0, ScaleKind::Major));
        let hz = pitch(&out, 440.0);
        assert!((hz - 440.0).abs() < 2.5, "{hz}");
    }

    #[test]
    fn a_note_outside_the_key_goes_to_the_nearest_in_it() {
        // F#4 in C major is a whole step between F and G: it lands on one.
        let out = run(tone(372.0), &params(&[]), Scale::new(0, ScaleKind::Major));
        let hz = pitch(&out, 372.0);
        let (to_f, to_g) = ((hz - 349.2).abs(), (hz - 392.0).abs());
        assert!(to_f.min(to_g) < 3.0, "{hz}");
        // Its own key, not the song's.
        let out = run(tone(372.0), &params(&[(KEY, 7.0), (SCALE, 2.0)]), Scale::new(0, ScaleKind::Major));
        let hz = pitch(&out, 372.0);
        assert!((hz - 370.0).abs() < 3.0, "F# major has that note: {hz}");
    }

    #[test]
    fn shift_moves_it_by_semitones() {
        let out = run(tone(440.0), &params(&[(SCALE, 1.0), (SHIFT, -12.0)]), Scale::new(0, ScaleKind::Major));
        let hz = pitch(&out, 220.0);
        assert!((hz - 220.0).abs() < 2.0, "{hz}");
    }

    #[test]
    fn an_amount_of_nothing_and_a_mix_of_nothing_leave_it_be() {
        let out = run(tone(430.0), &params(&[(SCALE, 1.0), (AMOUNT, 0.0)]), Scale::new(0, ScaleKind::Major));
        assert!((pitch(&out, 430.0) - 430.0).abs() < 2.0);
        let x = tone(430.0);
        let out = run(x.clone(), &params(&[(MIX, 0.0)]), Scale::new(0, ScaleKind::Major));
        assert_eq!(out, x);
    }

    #[test]
    fn in_tune_it_is_a_clean_short_delay_without_a_comb() {
        let x = tone(440.0);
        let out = run(x.clone(), &params(&[(SCALE, 1.0)]), Scale::new(0, ScaleKind::Major));
        let rms = |s: &[f32]| (s.iter().map(|v| v * v).sum::<f32>() / s.len() as f32).sqrt();
        let (a, b) = (rms(&x[24000..]), rms(&out[24000..]));
        assert!((b / a - 1.0).abs() < 0.08, "{a} {b}");
    }

    #[test]
    fn silence_and_noise_stay_finite() {
        let mut rng = crate::dsp::Rng::new(9);
        let noise: Vec<f32> = (0..24000).map(|_| rng.bipolar() * 0.5).collect();
        let out = run(noise, &params(&[]), Scale::new(0, ScaleKind::Major));
        assert!(out.iter().all(|v| v.is_finite() && v.abs() < 3.0));
        let out = run(vec![0.0; 24000], &params(&[]), Scale::new(0, ScaleKind::Major));
        assert!(out.iter().all(|v| v.abs() < 1e-6));
    }

    #[test]
    fn the_nearest_note_sticks_a_little() {
        let c = Scale::new(0, ScaleKind::Major);
        assert_eq!(nearest(60.2, &c, None), 60);
        assert_eq!(nearest(61.6, &c, None), 62);
        assert_eq!(nearest(61.02, &c, Some(60)), 60);
        assert_eq!(nearest(61.5, &c, Some(60)), 62);
    }
}

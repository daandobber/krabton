//! Vocoder: what goes in speaks, and a chord in the song's key sings it. Twenty
//! bands listen to the track and open twenty bands on a stack of saws; the
//! hissing top of a voice comes through as noise, so its s and t can still be
//! told apart. Which chord is a matter of the *Degree*, which can be moved by
//! hand or by a modulator like any other knob.

use super::super::{db_to_gain, fast_tanh, Rng, Svf};
use crate::dsp::effects::FxCtx;
use crate::params::{ParamSpec, Unit};

pub const DEGREE: usize = 0;
pub const OCTAVE: usize = 1;
pub const CHORD: usize = 2;
pub const WAVE: usize = 3;
pub const FORMANT: usize = 4;
pub const ATTACK: usize = 5;
pub const RELEASE: usize = 6;
pub const HISS: usize = 7;
pub const DETUNE: usize = 8;
pub const MIX: usize = 9;
pub const OUTPUT: usize = 10;

pub const CHORDS: &[&str] = &["One Note", "Fifth", "Triad", "Seventh", "Wide"];
pub const WAVES: &[&str] = &["Saw", "Square", "Thin"];

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::int("Degree", "Carrier", 1.0, 7.0, 1.0, Unit::None),
    ParamSpec::int("Octave", "Carrier", -2.0, 2.0, 0.0, Unit::None),
    ParamSpec::choice("Chord", "Carrier", CHORDS, 2),
    ParamSpec::choice("Wave", "Carrier", WAVES, 0),
    ParamSpec::lin("Formant", "Voice", -12.0, 12.0, 0.0, Unit::Semi),
    ParamSpec::log("Attack", "Voice", 0.5, 100.0, 4.0, Unit::Ms),
    ParamSpec::log("Release", "Voice", 5.0, 800.0, 60.0, Unit::Ms),
    ParamSpec::lin("Hiss", "Voice", 0.0, 1.0, 0.4, Unit::Percent),
    ParamSpec::lin("Detune", "Carrier", 0.0, 30.0, 7.0, Unit::Cents),
    ParamSpec::lin("Mix", "Output", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::lin("Output", "Output", -24.0, 12.0, 0.0, Unit::Db),
];

const BANDS: usize = 20;
const NOTES: usize = 4;
const Q: f32 = 5.0;
/// How many of the top bands steer the hiss.
const TOP: usize = 4;

#[derive(Clone, Copy)]
struct Band {
    /// Two in a row: they should not hear each other's band.
    hear: [Svf; 2],
    sing: [[Svf; 2]; 2],
    env: f32,
    hz: f32,
}

pub struct Vocoder {
    sr: f32,
    bands: [Band; BANDS],
    formant: f32,
    phase: [[f32; 2]; NOTES],
    rng: Rng,
    hiss_high: Svf,
}

fn band_hz(k: usize) -> f32 {
    90.0 * (9000.0f32 / 90.0).powf(k as f32 / (BANDS - 1) as f32)
}

/// The steps of the key the notes of a chord are on.
fn chord_steps(chord: usize) -> &'static [i32] {
    match chord {
        0 => &[0],
        1 => &[0, 4],
        2 => &[0, 2, 4],
        3 => &[0, 2, 4, 6],
        _ => &[0, 4, 7, 9],
    }
}

/// A band-limited step in a saw's corner.
#[inline]
fn blep(phase: f32, dt: f32) -> f32 {
    if phase < dt {
        let t = phase / dt;
        t + t - t * t - 1.0
    } else if phase > 1.0 - dt {
        let t = (phase - 1.0) / dt;
        t * t + t + t + 1.0
    } else {
        0.0
    }
}

impl Vocoder {
    pub fn new(sr: f32) -> Self {
        let mut bands = [Band { hear: [Svf::new(); 2], sing: [[Svf::new(); 2]; 2], env: 0.0, hz: 0.0 }; BANDS];
        for (k, b) in bands.iter_mut().enumerate() {
            b.hz = band_hz(k).min(sr * 0.45);
            for f in b.hear.iter_mut() {
                f.set_q(b.hz, Q, sr);
            }
            for f in b.sing.iter_mut().flatten() {
                f.set_q(b.hz, Q, sr);
            }
        }
        let mut hiss_high = Svf::new();
        hiss_high.set_q(5000.0_f32.min(sr * 0.45), 0.7, sr);
        Self { sr, bands, formant: 0.0, phase: [[0.0, 0.31]; NOTES], rng: Rng::new(0xC0DE), hiss_high }
    }

    pub fn process(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32], ctx: &FxCtx) {
        let sr = self.sr;
        let n = l.len().min(r.len());
        let formant = p[FORMANT];
        if (formant - self.formant).abs() > 0.01 {
            self.formant = formant;
            let ratio = 2f32.powf(formant / 12.0);
            for b in self.bands.iter_mut() {
                let hz = (b.hz * ratio).clamp(30.0, sr * 0.45);
                for f in b.sing.iter_mut().flatten() {
                    f.set_q(hz, Q, sr);
                }
            }
        }
        let attack = 1.0 - (-1.0 / (p[ATTACK].max(0.1) * 0.001 * sr)).exp();
        let release = 1.0 - (-1.0 / (p[RELEASE].max(1.0) * 0.001 * sr)).exp();
        let hiss = p[HISS].clamp(0.0, 1.0);
        let mix = p[MIX].clamp(0.0, 1.0);
        let output = db_to_gain(p[OUTPUT]);
        let wave = p[WAVE].round() as i32;
        // The notes of the chord on this step of the song's key.
        let steps = chord_steps((p[CHORD].round().max(0.0) as usize).min(CHORDS.len() - 1));
        let (home, _) = ctx.scale.locate(48 + 12 * p[OCTAVE].round() as i32);
        let root = home + p[DEGREE].round() as i32 - 1;
        let mut incs = [[0.0f32; 2]; NOTES];
        let cents = p[DETUNE] / 1200.0;
        for (k, inc) in incs.iter_mut().enumerate().take(steps.len()) {
            let hz = crate::dsp::midi_to_freq(ctx.scale.pitch_at(root + steps[k]) as f32);
            *inc = [hz * 2f32.powf(-cents) / sr, hz * 2f32.powf(cents) / sr];
        }
        let count = steps.len();
        let scale = 1.0 / (count as f32).sqrt();
        for i in 0..n {
            let (dry_l, dry_r) = (l[i], r[i]);
            let voice = 0.5 * (dry_l + dry_r);
            let mut carrier = [0.0f32; 2];
            for k in 0..count {
                for (side, c) in carrier.iter_mut().enumerate() {
                    let dt = incs[k][side].min(0.45);
                    let ph = (self.phase[k][side] + dt).fract();
                    self.phase[k][side] = ph;
                    let saw = 2.0 * ph - 1.0 - blep(ph, dt);
                    *c += match wave {
                        1 => {
                            let other = (ph + 0.5).fract();
                            saw - (2.0 * other - 1.0 - blep(other, dt))
                        }
                        2 => {
                            let other = (ph + 0.12).fract();
                            0.7 * (saw - (2.0 * other - 1.0 - blep(other, dt)))
                        }
                        _ => saw,
                    };
                }
            }
            let noise = self.hiss_high.tick(self.rng.bipolar()).high;
            let (mut out_l, mut out_r, mut top) = (0.0f32, 0.0f32, 0.0f32);
            for (k, b) in self.bands.iter_mut().enumerate() {
                let once = b.hear[0].tick(voice).band / Q;
                let heard = (b.hear[1].tick(once).band / Q).abs();
                b.env += (heard - b.env) * if heard > b.env { attack } else { release };
                let [left, right] = &mut b.sing;
                let once_l = left[0].tick(carrier[0] * scale).band / Q;
                let once_r = right[0].tick(carrier[1] * scale).band / Q;
                let a = left[1].tick(once_l).band / Q;
                let c = right[1].tick(once_r).band / Q;
                out_l += a * b.env;
                out_r += c * b.env;
                if k >= BANDS - TOP {
                    top += b.env;
                }
            }
            let s = noise * top * hiss * 2.5;
            let wet_l = fast_tanh((out_l * 60.0 + s) * output);
            let wet_r = fast_tanh((out_r * 60.0 + s) * output);
            l[i] = dry_l + (wet_l - dry_l) * mix;
            r[i] = dry_r + (wet_r - dry_r) * mix;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scale::{Scale, ScaleKind};

    const SR: f32 = 48000.0;

    fn ctx() -> FxCtx {
        FxCtx { sr: SR, bpm: 120.0, beat: 0.0, playing: true, scale: Scale::new(0, ScaleKind::Major) }
    }

    fn at(x: &[f32], hz: f32) -> f32 {
        let (mut re, mut im) = (0.0f32, 0.0f32);
        for (i, v) in x.iter().enumerate() {
            let a = std::f32::consts::TAU * hz * i as f32 / SR;
            re += v * a.cos();
            im += v * a.sin();
        }
        (re * re + im * im).sqrt() / x.len() as f32
    }

    fn rms(x: &[f32]) -> f32 {
        (x.iter().map(|v| v * v).sum::<f32>() / x.len().max(1) as f32).sqrt()
    }

    fn run(p: &[f32], make: impl Fn(usize) -> f32, seconds: f32) -> Vec<f32> {
        let n = (seconds * SR) as usize;
        let mut l: Vec<f32> = (0..n).map(&make).collect();
        let mut r = l.clone();
        let mut v = Vocoder::new(SR);
        for (a, b) in l.chunks_mut(256).zip(r.chunks_mut(256)) {
            v.process(p, a, b, &ctx());
        }
        l
    }

    fn noise(i: usize) -> f32 {
        // The same each time.
        let x = (i as u32).wrapping_mul(2654435761) ^ (i as u32 >> 3).wrapping_mul(40503);
        x as f32 / u32::MAX as f32 * 0.8 - 0.4
    }

    #[test]
    fn no_voice_no_sound() {
        let x = run(&crate::params::defaults(SPECS), |_| 0.0, 0.5);
        assert!(x.iter().all(|v| v.abs() < 1e-6));
    }

    #[test]
    fn it_sings_the_chords_pitch_not_the_voices() {
        // One note: C3 in C major, whatever goes in.
        let mut p = crate::params::defaults(SPECS);
        p[CHORD] = 0.0;
        p[DETUNE] = 0.0;
        p[HISS] = 0.0;
        let x = run(&p, noise, 1.0);
        let c = crate::dsp::midi_to_freq(48.0);
        let wet = &x[9600..];
        assert!(rms(wet) > 0.03, "{}", rms(wet));
        for h in [3.0, 4.0, 5.0] {
            assert!(at(wet, c * h) > 2.5 * at(wet, c * (h + 0.5)), "harmonic {h}: {} {}", at(wet, c * h), at(wet, c * (h + 0.5)));
        }
    }

    #[test]
    fn the_degree_moves_it_up_the_key() {
        let mut p = crate::params::defaults(SPECS);
        p[CHORD] = 0.0;
        p[DETUNE] = 0.0;
        p[HISS] = 0.0;
        p[DEGREE] = 5.0;
        let x = run(&p, noise, 1.0);
        let g = crate::dsp::midi_to_freq(55.0);
        assert!(at(&x[9600..], g * 3.0) > 2.5 * at(&x[9600..], g * 3.5));
    }

    #[test]
    fn it_opens_where_the_voice_is() {
        let tone = |hz: f32| move |i: usize| (std::f32::consts::TAU * hz * i as f32 / SR).sin() * 0.5;
        let mut p = crate::params::defaults(SPECS);
        p[HISS] = 0.0;
        let low = run(&p, tone(300.0), 0.6);
        let high = run(&p, tone(3000.0), 0.6);
        // A sound's brightness: how much changes from one frame to the next.
        let edge = |a: &[f32]| rms(&a.windows(2).map(|w| w[1] - w[0]).collect::<Vec<_>>()) / rms(a).max(1e-9);
        assert!(edge(&high[9600..]) > edge(&low[9600..]) * 2.0, "{} {}", edge(&low[9600..]), edge(&high[9600..]));
    }

    #[test]
    fn hiss_lets_the_top_through() {
        let hf = |i: usize| (std::f32::consts::TAU * 8000.0 * i as f32 / SR).sin() * 0.5;
        let mut p = crate::params::defaults(SPECS);
        p[HISS] = 0.0;
        let without = rms(&run(&p, hf, 0.5)[4800..]);
        p[HISS] = 1.0;
        let with = rms(&run(&p, hf, 0.5)[4800..]);
        assert!(with > without * 1.5, "{without} {with}");
    }

    #[test]
    fn it_stays_in_bounds() {
        let mut p = crate::params::defaults(SPECS);
        p[OUTPUT] = 12.0;
        p[CHORD] = 4.0;
        let x = run(&p, |i| noise(i) * 2.5, 1.0);
        assert!(x.iter().all(|v| v.is_finite() && v.abs() <= 1.05), "{}", x.iter().fold(0.0f32, |m, v| m.max(v.abs())));
    }
}

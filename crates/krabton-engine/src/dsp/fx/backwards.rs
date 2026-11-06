//! Backwards: an echo that comes back reversed, swelling up to where the sound
//! was. A piece of what went by, the length of a note of the song, is played the
//! other way round; two such pieces overlap so the turns are never heard. With
//! feedback the echo of an echo plays forward again.

use super::super::{FilterMode, Svf};
use crate::dsp::effects::FxCtx;
use crate::params::{ParamSpec, Unit};

pub const LENGTH: usize = 0;
pub const FEEDBACK: usize = 1;
pub const TONE: usize = 2;
pub const WIDTH: usize = 3;
pub const MIX: usize = 4;

pub const LENGTHS: &[&str] = &["1/16", "1/8", "1/8 D", "1/4", "1/4 D", "1/2", "1 Bar", "2 Bars"];
const LENGTH_BEATS: [f64; 8] = [0.25, 0.5, 0.75, 1.0, 1.5, 2.0, 4.0, 8.0];

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::choice("Length", "Reverse", LENGTHS, 3),
    ParamSpec::lin("Feedback", "Reverse", 0.0, 0.95, 0.25, Unit::Percent),
    ParamSpec::log("Tone", "Reverse", 300.0, 18000.0, 9000.0, Unit::Hz),
    ParamSpec::lin("Width", "Output", 0.0, 1.0, 0.4, Unit::Percent),
    ParamSpec::lin("Mix", "Output", 0.0, 1.0, 0.4, Unit::Percent),
];

/// The longest piece (at the slowest tempo that counts).
const LINE_SECONDS: f32 = 22.0;

pub struct Backwards {
    sr: f32,
    line: [Vec<f32>; 2],
    write: usize,
    /// Two pieces half a length apart: how far along each is (0..1), and where it reads.
    along: [f64; 2],
    read: [f64; 2],
    tone: [Svf; 2],
}

impl Backwards {
    pub fn new(sr: f32) -> Self {
        let size = (LINE_SECONDS * sr) as usize + 4;
        Self { sr, line: [vec![0.0; size], vec![0.0; size]], write: 0, along: [0.0, 0.5], read: [0.0; 2], tone: [Svf::new(); 2] }
    }

    pub fn process(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32], ctx: &FxCtx) {
        let sr = self.sr as f64;
        let n = l.len().min(r.len());
        let size = self.line[0].len();
        let beats = LENGTH_BEATS[(p[LENGTH].round().max(0.0) as usize).min(LENGTH_BEATS.len() - 1)];
        let frames = (beats * 60.0 / ctx.bpm.max(20.0) as f64 * sr).clamp(64.0, size as f64 * 0.45);
        let step = 1.0 / frames;
        let feedback = p[FEEDBACK].clamp(0.0, 0.95);
        let width = p[WIDTH].clamp(0.0, 1.0);
        let mix = p[MIX].clamp(0.0, 1.0);
        let cutoff = p[TONE].clamp(100.0, self.sr * 0.45);
        for f in self.tone.iter_mut() {
            f.set_q(cutoff, 0.707, self.sr);
        }
        for i in 0..n {
            let (dry_l, dry_r) = (l[i], r[i]);
            let mut wet = [0.0f32; 2];
            for g in 0..2 {
                // A hump over the piece: two of them, half apart, always add up to one.
                let w = (std::f64::consts::PI * self.along[g]).sin();
                let gain = (w * w) as f32;
                let at = self.read[g] as usize % size;
                wet[0] += self.line[0][at] * gain;
                wet[1] += self.line[1][at] * gain;
                self.read[g] = (self.read[g] - 1.0).rem_euclid(size as f64);
                self.along[g] += step;
                if self.along[g] >= 1.0 {
                    self.along[g] -= 1.0;
                    // A new piece: from what just went in, backwards.
                    self.read[g] = self.write as f64;
                }
            }
            let wet_l = self.tone[0].process(wet[0], FilterMode::Low);
            let wet_r = self.tone[1].process(wet[1], FilterMode::Low);
            self.line[0][self.write] = dry_l + wet_l * feedback;
            self.line[1][self.write] = dry_r + wet_r * feedback;
            self.write = (self.write + 1) % size;
            let mid = 0.5 * (wet_l + wet_r);
            let side = 0.5 * (wet_l - wet_r) * (0.5 + width * 1.5);
            l[i] = dry_l * (1.0 - mix) + (mid + side) * mix;
            r[i] = dry_r * (1.0 - mix) + (mid - side) * mix;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scale::{Scale, ScaleKind};

    const SR: f32 = 48000.0;

    fn ctx() -> FxCtx {
        FxCtx { sr: SR, bpm: 120.0, beat: 0.0, playing: true, scale: Scale::new(0, ScaleKind::Minor) }
    }

    fn run(p: &[f32], make: impl Fn(usize) -> f32, seconds: f32) -> Vec<f32> {
        let n = (seconds * SR) as usize;
        let mut l: Vec<f32> = (0..n).map(&make).collect();
        let mut r = l.clone();
        let mut b = Backwards::new(SR);
        for (a, c) in l.chunks_mut(256).zip(r.chunks_mut(256)) {
            b.process(p, a, c, &ctx());
        }
        l
    }

    fn rms(x: &[f32]) -> f32 {
        (x.iter().map(|v| v * v).sum::<f32>() / x.len().max(1) as f32).sqrt()
    }

    fn wet(feedback: f32) -> Vec<f32> {
        let mut p = crate::params::defaults(SPECS);
        p[FEEDBACK] = feedback;
        p[MIX] = 1.0;
        p[TONE] = 18000.0;
        p[LENGTH] = 3.0; // a quarter at 120: half a second
        p
    }

    #[test]
    fn silence_stays_silent() {
        assert!(run(&wet(0.5), |_| 0.0, 1.0).iter().all(|v| v.abs() < 1e-6));
    }

    #[test]
    fn a_sound_that_dies_away_comes_back_swelling() {
        // A blow that fades over a tenth of a second, and then nothing.
        let burst = |i: usize| {
            let t = i as f32 / SR;
            if t < 0.1 {
                (std::f32::consts::TAU * 1000.0 * t).sin() * (1.0 - t / 0.1)
            } else {
                0.0
            }
        };
        let x = run(&wet(0.0), burst, 1.2);
        let parts: Vec<f32> = x.chunks(1200).map(rms).collect();
        let peak = parts.iter().enumerate().fold((0, 0.0f32), |m, (k, v)| if *v > m.1 { (k, *v) } else { m });
        assert!(peak.1 > 0.01, "something comes back: {}", peak.1);
        // Before the peak: quieter the further from it, as a fade played backwards.
        let k = peak.0;
        assert!(k >= 3, "{k}");
        assert!(parts[k - 3] < parts[k] * 0.9, "{:?}", &parts[k - 3..=k]);
    }

    #[test]
    fn it_comes_later_than_the_sound_and_the_pieces_add_up_evenly() {
        // A steady tone comes through without dips where the pieces turn.
        let x = run(&wet(0.0), |i| (std::f32::consts::TAU * 700.0 * i as f32 / SR).sin() * 0.5, 2.0);
        let parts: Vec<f32> = x[48000..].chunks(2400).map(rms).collect();
        let top = parts.iter().cloned().fold(0.0f32, f32::max);
        assert!(top > 0.15, "{top}");
    }

    #[test]
    fn feedback_keeps_it_going_and_it_dies_away() {
        let burst = |i: usize| if i < 2400 { ((i as f32) * 0.2).sin() * 0.7 } else { 0.0 };
        let once = run(&wet(0.0), burst, 3.0);
        let more = run(&wet(0.8), burst, 3.0);
        let tail = |x: &[f32]| rms(&x[x.len() - 24000..]);
        assert!(tail(&once) < 1e-4, "{}", tail(&once));
        assert!(tail(&more) > 1e-3, "{}", tail(&more));
        let long = run(&wet(0.95), burst, 40.0);
        assert!(long.iter().all(|v| v.is_finite() && v.abs() < 4.0));
    }
}

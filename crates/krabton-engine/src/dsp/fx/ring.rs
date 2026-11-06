//! Ring: the sound multiplied by a tone. The notes go, the sum and the difference
//! stay: bells out of pads, robots out of voices and a slow tremolo at the low
//! end of the dial. *Key* tunes the tone to a note of the song's key, so the
//! metal is at least the right metal.

use super::super::fast_tanh;
use crate::dsp::effects::FxCtx;
use crate::params::{ParamSpec, Unit};

pub const FREQUENCY: usize = 0;
pub const IN_KEY: usize = 1;
pub const DEGREE: usize = 2;
pub const OCTAVE: usize = 3;
pub const WAVE: usize = 4;
pub const SPREAD: usize = 5;
pub const DRIVE: usize = 6;
pub const MIX: usize = 7;

pub const WAVES: &[&str] = &["Sine", "Triangle", "Square"];

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::log("Frequency", "Tone", 1.0, 5000.0, 220.0, Unit::Hz),
    ParamSpec::toggle("Key", "Tone", false),
    ParamSpec::int("Degree", "Tone", 1.0, 7.0, 1.0, Unit::None),
    ParamSpec::int("Octave", "Tone", -2.0, 3.0, 1.0, Unit::None),
    ParamSpec::choice("Wave", "Tone", WAVES, 0),
    ParamSpec::lin("Spread", "Tone", 0.0, 50.0, 0.0, Unit::Hz),
    ParamSpec::lin("Drive", "Output", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Mix", "Output", 0.0, 1.0, 1.0, Unit::Percent),
];

pub struct Ring {
    sr: f32,
    phase: [f32; 2],
}

impl Ring {
    pub fn new(sr: f32) -> Self {
        Self { sr, phase: [0.0; 2] }
    }

    pub fn process(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32], ctx: &FxCtx) {
        let n = l.len().min(r.len());
        let hz = if p[IN_KEY] >= 0.5 {
            let (home, _) = ctx.scale.locate(48 + 12 * p[OCTAVE].round() as i32);
            crate::dsp::midi_to_freq(ctx.scale.pitch_at(home + p[DEGREE].round() as i32 - 1) as f32)
        } else {
            p[FREQUENCY]
        };
        let spread = p[SPREAD];
        let inc = [(hz - spread * 0.5).max(0.0) / self.sr, (hz + spread * 0.5) / self.sr];
        let wave = p[WAVE].round() as i32;
        let drive = 1.0 + p[DRIVE].clamp(0.0, 1.0) * 6.0;
        let mix = p[MIX].clamp(0.0, 1.0);
        let shape = |ph: f32| match wave {
            1 => 1.0 - 4.0 * (ph - 0.5).abs(),
            2 => {
                if ph < 0.5 {
                    1.0
                } else {
                    -1.0
                }
            }
            _ => (ph * std::f32::consts::TAU).sin(),
        };
        for i in 0..n {
            self.phase[0] = (self.phase[0] + inc[0]).fract();
            self.phase[1] = (self.phase[1] + inc[1]).fract();
            let (a, b) = (l[i], r[i]);
            let (x, y) = if drive > 1.0 { (fast_tanh(a * drive), fast_tanh(b * drive)) } else { (a, b) };
            l[i] = a + (x * shape(self.phase[0]) - a) * mix;
            r[i] = b + (y * shape(self.phase[1]) - b) * mix;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scale::{Scale, ScaleKind};

    const SR: f32 = 48000.0;

    fn run(p: &[f32], input_hz: f32) -> (Vec<f32>, Vec<f32>) {
        let n = 48000;
        let mut l: Vec<f32> = (0..n).map(|i| (std::f32::consts::TAU * input_hz * i as f32 / SR).sin() * 0.5).collect();
        let mut r = l.clone();
        let ctx = FxCtx { sr: SR, bpm: 120.0, beat: 0.0, playing: true, scale: Scale::new(0, ScaleKind::Major) };
        let mut g = Ring::new(SR);
        for (a, b) in l.chunks_mut(256).zip(r.chunks_mut(256)) {
            g.process(p, a, b, &ctx);
        }
        (l, r)
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

    #[test]
    fn the_note_goes_and_the_sum_and_difference_stay() {
        let mut p = crate::params::defaults(SPECS);
        p[FREQUENCY] = 300.0;
        let (x, _) = run(&p, 1000.0);
        assert!(at(&x, 700.0) > 0.1, "{}", at(&x, 700.0));
        assert!(at(&x, 1300.0) > 0.1, "{}", at(&x, 1300.0));
        assert!(at(&x, 1000.0) < 0.01, "{}", at(&x, 1000.0));
    }

    #[test]
    fn mix_brings_back_the_note() {
        let mut p = crate::params::defaults(SPECS);
        p[MIX] = 0.0;
        let (x, _) = run(&p, 1000.0);
        assert!(at(&x, 1000.0) > 0.2);
    }

    #[test]
    fn in_key_it_takes_its_note_from_the_song() {
        let mut p = crate::params::defaults(SPECS);
        p[IN_KEY] = 1.0;
        p[DEGREE] = 5.0;
        p[OCTAVE] = 1.0;
        // G4 in C major.
        let (x, _) = run(&p, 1000.0);
        let g = crate::dsp::midi_to_freq(67.0);
        assert!(at(&x, 1000.0 + g) > 0.1, "{g}: {}", at(&x, 1000.0 + g));
    }

    #[test]
    fn spread_makes_the_sides_differ() {
        let mut p = crate::params::defaults(SPECS);
        p[SPREAD] = 20.0;
        let (a, b) = run(&p, 500.0);
        assert!(a.iter().zip(&b).any(|(x, y)| (x - y).abs() > 0.05));
    }
}

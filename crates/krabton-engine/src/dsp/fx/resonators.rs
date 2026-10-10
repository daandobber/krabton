//! Resonators: five tuned comb filters on a note and a chord, ringing with
//! whatever comes in (drums turn into chords), spread across the stereo
//! field.

use super::DelayLine;
use crate::dsp::effects::FxCtx;
use crate::dsp::{fast_tanh, midi_to_freq, OnePole};
use crate::params::{ParamSpec, Unit};

pub const NOTE: usize = 0;
pub const CHORD: usize = 1;
pub const DECAY: usize = 2;
pub const COLOR: usize = 3;
pub const WIDTH: usize = 4;
pub const MIX: usize = 5;

pub const CHORDS: &[&str] = &["Unison", "Octaves", "Fifths", "Major", "Minor", "Sus4", "Major 7", "Minor 7", "Cluster"];
const INTERVALS: [[f32; 5]; 9] = [
    [0.0, 0.0, 0.0, 0.0, 0.0],
    [0.0, 12.0, -12.0, 24.0, 0.0],
    [0.0, 7.0, 12.0, 19.0, -5.0],
    [0.0, 4.0, 7.0, 12.0, 16.0],
    [0.0, 3.0, 7.0, 12.0, 15.0],
    [0.0, 5.0, 7.0, 12.0, 17.0],
    [0.0, 4.0, 7.0, 11.0, 14.0],
    [0.0, 3.0, 7.0, 10.0, 14.0],
    [0.0, 1.0, 2.0, 3.0, 5.0],
];
const PANS: [f32; 5] = [0.0, -0.8, 0.8, -0.4, 0.4];

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::int("Note", "Tuning", 24.0, 96.0, 48.0, Unit::None),
    ParamSpec::choice("Chord", "Tuning", CHORDS, 3),
    ParamSpec::log("Decay", "Sound", 0.05, 10.0, 1.5, Unit::Sec),
    ParamSpec::log("Color", "Sound", 400.0, 18000.0, 6000.0, Unit::Hz),
    ParamSpec::lin("Width", "Output", 0.0, 1.0, 0.6, Unit::Percent),
    ParamSpec::lin("Mix", "Output", 0.0, 1.0, 0.5, Unit::Percent),
];

pub struct Resonators {
    lines: Vec<DelayLine>,
    damp: [OnePole; 5],
}

impl Resonators {
    pub fn new(sr: f32) -> Self {
        // Long enough for the lowest note an octave below the lowest setting.
        let len = (sr / midi_to_freq(11.0)) as usize + 8;
        Self { lines: (0..5).map(|_| DelayLine::new(len)).collect(), damp: [OnePole::default(); 5] }
    }

    pub fn process(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32], ctx: &FxCtx) {
        let n = l.len().min(r.len());
        let sr = ctx.sr;
        let note = p[NOTE].round().clamp(12.0, 108.0);
        let chord = &INTERVALS[(p[CHORD].round().max(0.0) as usize).min(INTERVALS.len() - 1)];
        let decay = p[DECAY].clamp(0.01, 30.0);
        for d in self.damp.iter_mut() {
            d.set(p[COLOR].clamp(100.0, 0.45 * sr), sr);
        }
        let width = p[WIDTH].clamp(0.0, 1.0);
        let mix = p[MIX].clamp(0.0, 1.0);
        let mut setup = [(0.0f32, 0.0f32, 1.0f32, 1.0f32); 5];
        for (k, s) in setup.iter_mut().enumerate() {
            let f = midi_to_freq(note + chord[k]);
            let len = (sr / f).clamp(2.0, (self.lines[k].len() - 4) as f32);
            // Feedback for a 60 dB fall in `decay` seconds.
            let g = 10f32.powf(-3.0 * len / (sr * decay));
            let pan = PANS[k] * width;
            *s = (len, g, (1.0 - pan).min(1.0), (1.0 + pan).min(1.0));
        }
        for i in 0..n {
            let (xl, xr) = (l[i], r[i]);
            let x = 0.5 * (xl + xr) * 0.2;
            let (mut wl, mut wr) = (0.0, 0.0);
            for k in 0..5 {
                let (len, g, gl, gr) = setup[k];
                let y = self.lines[k].read(len);
                let fbk = self.damp[k].lp(y) * g;
                let w = fast_tanh(x + fbk);
                self.lines[k].write(if w.is_finite() { w } else { 0.0 });
                wl += y * gl;
                wr += y * gr;
            }
            l[i] = xl * (1.0 - mix) + wl * 0.5 * mix;
            r[i] = xr * (1.0 - mix) + wr * 0.5 * mix;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_util::*;
    use super::*;

    #[test]
    fn a_click_rings_at_the_note() {
        let mut rs = Resonators::new(48000.0);
        let mut p = crate::params::defaults(SPECS);
        p[CHORD] = 0.0;
        p[NOTE] = 57.0;
        p[MIX] = 1.0;
        let mut l = vec![0.0f32; 24000];
        l[0] = 1.0;
        let mut r = l.clone();
        rs.process(&p, &mut l, &mut r, &ctx());
        let got = strongest(&l[2400..], 180.0, 260.0, 1.0);
        assert!((got - 220.0).abs() <= 3.0, "{got}");
        assert!(peak(&l) < 2.0 && rms(&l[12000..]) > 1e-4);
    }
}

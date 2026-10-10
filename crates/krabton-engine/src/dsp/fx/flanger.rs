//! Flanger: a short delay swept by an LFO and fed back, mixed with the dry
//! signal, with a stereo offset.

use super::{sine, DelayLine, Lfo, LFO_SYNC};
use crate::dsp::effects::FxCtx;
use crate::params::{ParamSpec, Unit};

pub const RATE: usize = 0;
pub const SYNC: usize = 1;
pub const DEPTH: usize = 2;
pub const DELAY: usize = 3;
pub const FEEDBACK: usize = 4;
pub const STEREO: usize = 5;
pub const MIX: usize = 6;

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::log("Rate", "LFO", 0.02, 10.0, 0.25, Unit::Hz),
    ParamSpec::choice("Sync", "LFO", LFO_SYNC, 0),
    ParamSpec::lin("Depth", "LFO", 0.0, 1.0, 0.7, Unit::Percent),
    ParamSpec::log("Delay", "Flanger", 0.3, 15.0, 3.0, Unit::Ms),
    ParamSpec::lin("Feedback", "Flanger", -0.95, 0.95, 0.6, Unit::Percent),
    ParamSpec::lin("Stereo", "Output", 0.0, 1.0, 0.25, Unit::Percent),
    ParamSpec::lin("Mix", "Output", 0.0, 1.0, 0.5, Unit::Percent),
];

pub struct Flanger {
    lfo: Lfo,
    lines: [DelayLine; 2],
    last: [f32; 2],
    phases: Vec<f32>,
}

impl Flanger {
    pub fn new(sr: f32) -> Self {
        let len = (sr * 0.04) as usize + 8;
        Self { lfo: Lfo::default(), lines: [DelayLine::new(len), DelayLine::new(len)], last: [0.0; 2], phases: vec![0.0; 4096] }
    }

    pub fn process(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32], ctx: &FxCtx) {
        let n = l.len().min(r.len()).min(self.phases.len());
        let depth = p[DEPTH].clamp(0.0, 1.0);
        let base = p[DELAY].clamp(0.1, 20.0) * 0.001 * ctx.sr;
        let (fb, mix) = (p[FEEDBACK].clamp(-0.95, 0.95), p[MIX].clamp(0.0, 1.0));
        let offset = 0.5 * p[STEREO].clamp(0.0, 1.0);
        self.lfo.run(p[RATE], p[SYNC], ctx, &mut self.phases[..n]);
        for i in 0..n {
            for ch in 0..2 {
                let ph = self.phases[i] + if ch == 1 { offset } else { 0.0 };
                // Sweeps from the set delay down to a twentieth of it.
                let d = (base * (1.0 - 0.95 * depth * (0.5 + 0.5 * sine(ph)))).max(1.0);
                let x = if ch == 0 { l[i] } else { r[i] };
                let wet = self.lines[ch].read(d);
                let mut w = x + fb * wet;
                if !w.is_finite() {
                    w = 0.0;
                    self.lines[ch].clear();
                }
                self.lines[ch].write(crate::dsp::fast_tanh(w));
                self.last[ch] = wet;
                let out = x * (1.0 - 0.5 * mix) + wet * 0.5 * mix;
                if ch == 0 {
                    l[i] = out;
                } else {
                    r[i] = out;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_util::*;
    use super::*;

    #[test]
    fn it_flanges_and_stays_bounded() {
        let mut f = Flanger::new(48000.0);
        let mut p = crate::params::defaults(SPECS);
        p[FEEDBACK] = 0.95;
        p[MIX] = 1.0;
        let mut l = tone(300.0, 1.0, 0.8);
        let mut r = l.clone();
        for (cl, cr) in l.chunks_mut(256).zip(r.chunks_mut(256)) {
            f.process(&p, cl, cr, &ctx());
        }
        assert!(l.iter().chain(&r).all(|x| x.is_finite()));
        assert!(peak(&l) < 4.0 && rms(&l) > 0.05);
    }
}

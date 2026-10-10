//! Spring: a spring reverb tank. Each spring is a loop of a delay and a long
//! chain of all-pass filters, which spreads the frequencies out in time:
//! the chirp and boing of a guitar amp or a dub mixer's spring. Two springs
//! per side, a drip that shakes the delay, and the tank's limited tone.

use super::DelayLine;
use crate::dsp::effects::FxCtx;
use crate::dsp::{fast_tanh, OnePole, Rng};
use crate::params::{ParamSpec, Unit};

pub const DECAY: usize = 0;
pub const TENSION: usize = 1;
pub const LENGTH: usize = 2;
pub const TONE: usize = 3;
pub const DRIP: usize = 4;
pub const DRIVE: usize = 5;
pub const MIX: usize = 6;

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::lin("Decay", "Spring", 0.0, 1.0, 0.6, Unit::Percent),
    ParamSpec::lin("Tension", "Spring", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Length", "Spring", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::log("Tone", "Tone", 1000.0, 10000.0, 4000.0, Unit::Hz),
    ParamSpec::lin("Drip", "Tone", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::lin("Drive", "Tone", 0.0, 1.0, 0.1, Unit::Percent),
    ParamSpec::lin("Mix", "Output", 0.0, 1.0, 0.35, Unit::Percent),
];

const STAGES: usize = 36;
/// Loop delays (ms at length 0.5) of the four springs.
const DELAYS: [f32; 4] = [37.0, 45.5, 41.3, 52.1];

struct Spring {
    line: DelayLine,
    /// First-order all-pass states (input and output).
    x1: [f32; STAGES],
    y1: [f32; STAGES],
    tone: OnePole,
    dc: OnePole,
}

impl Spring {
    fn new(sr: f32) -> Self {
        Self { line: DelayLine::new((sr * 0.13) as usize), x1: [0.0; STAGES], y1: [0.0; STAGES], tone: OnePole::default(), dc: OnePole::default() }
    }
}

pub struct SpringReverb {
    springs: [Spring; 4],
    rng: Rng,
    drip: f32,
    drip_lp: OnePole,
}

impl SpringReverb {
    pub fn new(sr: f32) -> Self {
        Self { springs: std::array::from_fn(|_| Spring::new(sr)), rng: Rng::new(0x5_9819), drip: 0.0, drip_lp: OnePole::default() }
    }

    pub fn process(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32], ctx: &FxCtx) {
        let n = l.len().min(r.len());
        let sr = ctx.sr;
        let fb = 0.5 + 0.47 * p[DECAY].clamp(0.0, 1.0);
        // More tension: a stronger chirp.
        let a = -(0.45 + 0.4 * p[TENSION].clamp(0.0, 1.0));
        let length = 0.5 + p[LENGTH].clamp(0.0, 1.0);
        let drip = p[DRIP].clamp(0.0, 1.0);
        let drive = 1.0 + 4.0 * p[DRIVE].clamp(0.0, 1.0);
        let mix = p[MIX].clamp(0.0, 1.0);
        for s in self.springs.iter_mut() {
            s.tone.set(p[TONE].clamp(200.0, 0.45 * sr), sr);
            s.dc.set(60.0, sr);
        }
        self.drip_lp.set(8.0, sr);
        for i in 0..n {
            let (xl, xr) = (l[i], r[i]);
            // Drips: the springs are knocked about, now and then.
            if self.rng.unipolar() < drip * 6.0 / sr {
                self.drip = self.rng.bipolar();
            }
            self.drip *= 0.9995;
            let wobble = self.drip_lp.lp(self.drip) * drip * 0.004 * sr;
            let mut out = [0.0f32; 2];
            for (k, s) in self.springs.iter_mut().enumerate() {
                let input = if k % 2 == 0 { xl } else { xr };
                let d = (DELAYS[k] * 0.001 * sr * length + wobble * if k % 2 == 0 { 1.0 } else { -0.7 }).max(4.0);
                let back = s.line.read(d);
                // The dispersion: a long chain of first-order all-passes.
                let mut v = fast_tanh((input + back * fb) * drive) / drive;
                for j in 0..STAGES {
                    let y = a * v + s.x1[j] - a * s.y1[j];
                    s.x1[j] = v;
                    s.y1[j] = y;
                    v = y;
                }
                v = s.tone.lp(v);
                v -= s.dc.lp(v);
                if !v.is_finite() {
                    v = 0.0;
                    s.x1 = [0.0; STAGES];
                    s.y1 = [0.0; STAGES];
                    s.line.clear();
                }
                s.line.write(v);
                out[k % 2] += v;
            }
            l[i] = xl * (1.0 - mix) + out[0] * 0.7 * mix;
            r[i] = xr * (1.0 - mix) + out[1] * 0.7 * mix;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_util::*;
    use super::*;

    #[test]
    fn a_click_boings_and_dies_away() {
        let mut s = SpringReverb::new(48000.0);
        let mut p = crate::params::defaults(SPECS);
        p[MIX] = 1.0;
        let mut l = vec![0.0f32; 48000 * 6];
        l[10] = 1.0;
        let mut r = l.clone();
        for (cl, cr) in l.chunks_mut(256).zip(r.chunks_mut(256)) {
            s.process(&p, cl, cr, &ctx());
        }
        assert!(l.iter().all(|x| x.is_finite()) && peak(&l) < 2.0 && peak(&l) > 0.01);
        assert!(rms(&l[2400..24000]) > 1e-3);
        assert!(peak(&l[48000 * 5..]) < 1e-3, "{}", peak(&l[48000 * 5..]));
    }
}

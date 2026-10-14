//! Lattice: eight short delays feeding each other through a mixing matrix
//! (a feedback delay network). Spaced unevenly they blur into a room; spaced
//! evenly (Shape) they ring like a metal box. Size sets the delays, Feedback
//! how long it rings, Tone how dark, and turned all the way up the network
//! holds what is in it (Freeze).

use super::DelayLine;
use crate::dsp::effects::FxCtx;
use crate::dsp::OnePole;
use crate::params::{ParamSpec, Unit};

pub const SIZE: usize = 0;
pub const FEEDBACK: usize = 1;
pub const TONE: usize = 2;
pub const SHAPE: usize = 3;
pub const MIX: usize = 4;

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::log("Size", "Lattice", 0.002, 0.08, 0.02, Unit::Sec),
    ParamSpec::lin("Feedback", "Lattice", 0.0, 1.0, 0.75, Unit::Percent),
    ParamSpec::lin("Tone", "Lattice", 0.0, 1.0, 0.6, Unit::Percent),
    ParamSpec::lin("Shape", "Lattice", 0.0, 1.0, 0.2, Unit::Percent),
    ParamSpec::lin("Mix", "Output", 0.0, 1.0, 0.4, Unit::Percent),
];

const LINES: usize = 8;
/// Uneven spacing: lengths with no common measure.
const UNEVEN: [f32; LINES] = [1.0, 1.137, 1.273, 1.419, 1.583, 1.771, 1.997, 2.239];

pub struct Lattice {
    lines: Vec<DelayLine>,
    damp: [OnePole; LINES],
    sr: f32,
}

impl Lattice {
    pub fn new(sr: f32) -> Self {
        let len = (sr * 0.08 * 2.3) as usize + 16;
        Self { lines: (0..LINES).map(|_| DelayLine::new(len)).collect(), damp: [OnePole::default(); LINES], sr }
    }

    pub fn process(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32], _ctx: &FxCtx) {
        let n = l.len().min(r.len());
        let sr = self.sr;
        let size = p[SIZE].clamp(0.002, 0.08) * sr;
        let shape = p[SHAPE].clamp(0.0, 1.0);
        let lengths: [f32; LINES] = std::array::from_fn(|k| size * (UNEVEN[k] * (1.0 - shape) + (1.0 + k as f32 * 0.0001) * shape * 1.6));
        let freeze = p[FEEDBACK] >= 0.995;
        let g = if freeze { 1.0 } else { p[FEEDBACK].clamp(0.0, 1.0) * 0.97 };
        let cutoff = 800.0 + 17000.0 * p[TONE].clamp(0.0, 1.0).powi(2);
        for d in self.damp.iter_mut() {
            d.set(if freeze { 20000.0 } else { cutoff }, sr);
        }
        let mix = p[MIX].clamp(0.0, 1.0);
        let input = if freeze { 0.0 } else { 1.0 };
        let scale = 1.0 / (LINES as f32).sqrt();
        for i in 0..n {
            let mut v: [f32; LINES] = std::array::from_fn(|k| self.damp[k].lp(self.lines[k].read(lengths[k])));
            let (outl, outr) = (v.iter().step_by(2).sum::<f32>() * 0.35, v.iter().skip(1).step_by(2).sum::<f32>() * 0.35);
            // Mix them through a Hadamard matrix (in place, normalized).
            let mut h = 1;
            while h < LINES {
                for a in (0..LINES).step_by(h * 2) {
                    for b in a..a + h {
                        let (x, y) = (v[b], v[b + h]);
                        v[b] = x + y;
                        v[b + h] = x - y;
                    }
                }
                h *= 2;
            }
            let (x, y) = (l[i], r[i]);
            for (k, line) in self.lines.iter_mut().enumerate() {
                let into = if k % 2 == 0 { x } else { y };
                line.write(v[k] * scale * g + into * input * 0.5);
            }
            l[i] = x * (1.0 - mix) + outl * mix;
            r[i] = y * (1.0 - mix) + outr * mix;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_rings_on_and_stays_finite() {
        let sr = 48000.0;
        let mut f = Lattice::new(sr);
        let mut p = crate::params::defaults(SPECS);
        p[MIX] = 1.0;
        p[FEEDBACK] = 0.9;
        let ctx = FxCtx { sr, bpm: 120.0, beat: 0.0, playing: false };
        let mut l = vec![0.0; 48000];
        let mut r = vec![0.0; 48000];
        l[0] = 1.0;
        r[0] = 1.0;
        f.process(&p, &mut l, &mut r, &ctx);
        let rms = |x: &[f32]| (x.iter().map(|v| v * v).sum::<f32>() / x.len() as f32).sqrt();
        assert!(rms(&l[4800..9600]) > 1e-3, "rings: {}", rms(&l[4800..9600]));
        assert!(rms(&l[40000..]) < rms(&l[4800..9600]), "and dies away");
        assert!(l.iter().chain(&r).all(|x| x.is_finite() && x.abs() < 4.0));
        // Frozen it holds what it has.
        p[FEEDBACK] = 1.0;
        let mut l2 = vec![0.0; 48000];
        let mut r2 = vec![0.0; 48000];
        f.process(&p, &mut l2, &mut r2, &ctx);
        assert!(rms(&l2[40000..]) > rms(&l2[0..8000]) * 0.5, "frozen: {} vs {}", rms(&l2[40000..]), rms(&l2[0..8000]));
    }
}

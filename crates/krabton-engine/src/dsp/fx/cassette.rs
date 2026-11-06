//! Cassette: a tape that has been in a car all summer. The pitch sways slowly
//! and shivers quickly, the tape saturates, the top goes, the low end swells at
//! the head's bump, it hisses, and now and then the sound dips where the oxide
//! has worn.

use super::super::{fast_tanh, FilterMode, Rng, Svf};
use crate::dsp::effects::FxCtx;
use crate::params::{ParamSpec, Unit};

pub const WOW: usize = 0;
pub const FLUTTER: usize = 1;
pub const SATURATION: usize = 2;
pub const TONE: usize = 3;
pub const BUMP: usize = 4;
pub const HISS: usize = 5;
pub const DROPOUTS: usize = 6;
pub const MIX: usize = 7;

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::lin("Wow", "Motor", 0.0, 1.0, 0.25, Unit::Percent),
    ParamSpec::lin("Flutter", "Motor", 0.0, 1.0, 0.2, Unit::Percent),
    ParamSpec::lin("Saturation", "Tape", 0.0, 1.0, 0.35, Unit::Percent),
    ParamSpec::log("Tone", "Tape", 1000.0, 18000.0, 7500.0, Unit::Hz),
    ParamSpec::lin("Bump", "Tape", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::lin("Hiss", "Wear", 0.0, 1.0, 0.15, Unit::Percent),
    ParamSpec::lin("Dropouts", "Wear", 0.0, 1.0, 0.1, Unit::Percent),
    ParamSpec::lin("Mix", "Output", 0.0, 1.0, 1.0, Unit::Percent),
];

const LINE_SECONDS: f32 = 0.08;
/// Where the head sits when nothing sways, in seconds.
const CENTRE: f32 = 0.022;

pub struct Cassette {
    sr: f32,
    line: [Vec<f32>; 2],
    write: usize,
    wow: f32,
    flutter: f32,
    /// Slow chance: where the speed is wandering to.
    drift: f32,
    drift_goal: f32,
    drift_in: u32,
    rng: Rng,
    /// The dip the sound is in, how deep it wants to be, and for how much longer.
    dip: f32,
    dip_goal: f32,
    dip_in: u32,
    tone: [Svf; 2],
    bump: [Svf; 2],
    hiss_high: [Svf; 2],
}

impl Cassette {
    pub fn new(sr: f32) -> Self {
        let size = (LINE_SECONDS * sr) as usize + 4;
        let mut bump = [Svf::new(); 2];
        let mut hiss_high = [Svf::new(); 2];
        for f in bump.iter_mut() {
            f.set_q(85.0, 1.2, sr);
        }
        for f in hiss_high.iter_mut() {
            f.set_q(2500.0_f32.min(sr * 0.45), 0.7, sr);
        }
        Self { sr, line: [vec![0.0; size], vec![0.0; size]], write: 0, wow: 0.0, flutter: 0.0, drift: 0.0, drift_goal: 0.0, drift_in: 0, rng: Rng::new(0xCA55), dip: 1.0, dip_goal: 1.0, dip_in: 0, tone: [Svf::new(); 2], bump, hiss_high }
    }

    /// How far off the centre the head is now (seconds), from the two motors.
    fn sway(&mut self, wow: f32, flutter: f32) -> f32 {
        let sr = self.sr;
        let tau = std::f32::consts::TAU;
        self.wow = (self.wow + 0.55 / sr).fract();
        self.flutter = (self.flutter + 7.3 / sr).fract();
        if self.drift_in == 0 {
            self.drift_goal = self.rng.bipolar();
            self.drift_in = (sr * (0.4 + self.rng.unipolar() * 1.5)) as u32;
        }
        self.drift_in -= 1;
        self.drift += (self.drift_goal - self.drift) * (1.5 / sr);
        wow * 0.008 * (0.7 * (self.wow * tau).sin() + 0.3 * self.drift) + flutter * 0.0006 * ((self.flutter * tau).sin() + 0.4 * (self.flutter * 2.7 * tau).sin())
    }

    pub fn process(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32], _ctx: &FxCtx) {
        let sr = self.sr;
        let n = l.len().min(r.len());
        let (wow, flutter) = (p[WOW].clamp(0.0, 1.0), p[FLUTTER].clamp(0.0, 1.0));
        let sat = p[SATURATION].clamp(0.0, 1.0);
        let drive = 1.0 + sat * 5.0;
        let comp = 1.0 / fast_tanh(drive).max(0.1) / (1.0 + sat);
        let bump = p[BUMP].clamp(0.0, 1.0) * 0.9;
        let hiss = p[HISS].clamp(0.0, 1.0);
        let hiss_level = hiss * hiss * 0.05;
        let drops = p[DROPOUTS].clamp(0.0, 1.0);
        let mix = p[MIX].clamp(0.0, 1.0);
        let cutoff = p[TONE].clamp(200.0, sr * 0.45);
        for f in self.tone.iter_mut() {
            f.set_q(cutoff, 0.6, sr);
        }
        let dip_k = 1.0 - (-1.0 / (0.006 * sr)).exp();
        let size = self.line[0].len();
        for i in 0..n {
            let delay = (CENTRE + self.sway(wow, flutter)) * sr;
            // Now and then the oxide has gone: a short dip.
            if self.dip_in > 0 {
                self.dip_in -= 1;
                if self.dip_in == 0 {
                    self.dip_goal = 1.0;
                }
            } else if drops > 0.0 && self.rng.unipolar() < drops * drops * 3.0 / sr {
                self.dip_goal = 0.15 + 0.5 * self.rng.unipolar();
                self.dip_in = (sr * (0.03 + 0.12 * self.rng.unipolar())) as u32;
            }
            self.dip += (self.dip_goal - self.dip) * dip_k;
            let dry = [l[i], r[i]];
            let mut out = [0.0f32; 2];
            for side in 0..2 {
                self.line[side][self.write] = dry[side];
                let at = self.write as f32 + size as f32 - delay;
                let i0 = at.floor() as usize % size;
                let f = at.fract();
                let line = &self.line[side];
                let x = line[i0] + (line[(i0 + 1) % size] - line[i0]) * f;
                let warm = x + bump * self.bump[side].tick(x).band;
                let y = fast_tanh(warm * drive) * comp;
                let toned = self.tone[side].process(y, FilterMode::Low);
                let h = self.hiss_high[side].tick(self.rng.bipolar()).high * hiss_level;
                out[side] = toned * self.dip + h;
            }
            self.write = (self.write + 1) % size;
            l[i] = dry[0] + (out[0] - dry[0]) * mix;
            r[i] = dry[1] + (out[1] - dry[1]) * mix;
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
        let mut c = Cassette::new(SR);
        for (a, b) in l.chunks_mut(256).zip(r.chunks_mut(256)) {
            c.process(p, a, b, &ctx());
        }
        l
    }

    fn rms(x: &[f32]) -> f32 {
        (x.iter().map(|v| v * v).sum::<f32>() / x.len().max(1) as f32).sqrt()
    }

    fn clean() -> Vec<f32> {
        let mut p = crate::params::defaults(SPECS);
        p[WOW] = 0.0;
        p[FLUTTER] = 0.0;
        p[HISS] = 0.0;
        p[DROPOUTS] = 0.0;
        p
    }

    fn tone(hz: f32) -> impl Fn(usize) -> f32 {
        move |i| (std::f32::consts::TAU * hz * i as f32 / SR).sin() * 0.4
    }

    /// How much the time between rising crossings varies, as a share of it.
    fn wobble(x: &[f32]) -> f32 {
        let ups: Vec<usize> = (1..x.len()).filter(|&i| x[i - 1] < 0.0 && x[i] >= 0.0).collect();
        let over = 20;
        let spans: Vec<f32> = ups.windows(over + 1).map(|w| (w[over] - w[0]) as f32).collect();
        let mean = spans.iter().sum::<f32>() / spans.len() as f32;
        spans.iter().fold(0.0f32, |m, s| m.max((s - mean).abs())) / mean
    }

    #[test]
    fn silence_stays_silent_without_hiss() {
        let x = run(&clean(), |_| 0.0, 0.5);
        assert!(x.iter().all(|v| v.abs() < 1e-6));
    }

    #[test]
    fn the_hiss_is_there_when_nothing_plays() {
        let mut p = clean();
        p[HISS] = 0.6;
        let x = run(&p, |_| 0.0, 0.5);
        assert!(rms(&x) > 1e-3, "{}", rms(&x));
    }

    #[test]
    fn the_motors_make_the_pitch_sway() {
        let steady = wobble(&run(&clean(), tone(1000.0), 2.0)[4800..]);
        let mut p = clean();
        p[WOW] = 1.0;
        let wow = wobble(&run(&p, tone(1000.0), 2.0)[4800..]);
        let mut p = clean();
        p[FLUTTER] = 1.0;
        let flutter = wobble(&run(&p, tone(1000.0), 2.0)[4800..]);
        assert!(steady < 0.005, "{steady}");
        assert!(wow > steady * 3.0 && wow > 0.01, "{steady} {wow}");
        assert!(flutter > steady * 3.0, "{steady} {flutter}");
    }

    #[test]
    fn the_top_goes() {
        let mut p = clean();
        p[TONE] = 1500.0;
        let low = rms(&run(&p, tone(300.0), 0.5)[4800..]);
        let high = rms(&run(&p, tone(8000.0), 0.5)[4800..]);
        assert!(high < low * 0.2, "{low} {high}");
    }

    #[test]
    fn dropouts_dip_the_sound() {
        let mut p = clean();
        p[DROPOUTS] = 1.0;
        let x = run(&p, tone(500.0), 6.0);
        let chunks: Vec<f32> = x[4800..].chunks(1200).map(rms).collect();
        let top = chunks.iter().cloned().fold(0.0f32, f32::max);
        let low = chunks.iter().cloned().fold(1.0f32, f32::min);
        assert!(low < top * 0.75, "{low} {top}");
        let steady = run(&clean(), tone(500.0), 2.0);
        let chunks: Vec<f32> = steady[4800..].chunks(1200).map(rms).collect();
        let top = chunks.iter().cloned().fold(0.0f32, f32::max);
        let low = chunks.iter().cloned().fold(1.0f32, f32::min);
        assert!(low > top * 0.97, "{low} {top}");
    }

    #[test]
    fn it_stays_in_bounds_when_driven_hard() {
        let mut p = crate::params::defaults(SPECS);
        p[SATURATION] = 1.0;
        p[BUMP] = 1.0;
        let x = run(&p, |i| ((i as f32) * 0.01).sin() * 3.0, 1.0);
        assert!(x.iter().all(|v| v.is_finite() && v.abs() < 1.6));
    }
}

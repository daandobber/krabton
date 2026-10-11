//! A wind chime, simulated. Seen from above, the tubes hang in a ring
//! around the clapper; each is a pendulum on its string, and so is the
//! clapper, with a sail below it that catches the wind. The wind blows from
//! a direction that wanders, in gusts, with some turbulence. When the
//! clapper reaches a tube they collide (momentum and a bounce), and that
//! collision is the strike you hear, as hard as they met. Both the Chimes
//! sequencer and the Wind Chimes instrument run this, and their screens
//! draw its state, so what you see is what you hear.
//!
//! Units: the ring of tubes has radius 1; time in seconds.

pub const MAX_TUBES: usize = 8;
/// Radius of a tube (seen from above).
pub const TUBE_R: f32 = 0.09;
/// The simulation step.
const STEP: f32 = 0.001;

/// The chime's build and the weather.
#[derive(Clone, Copy, Debug)]
pub struct Shape {
    pub tubes: usize,
    /// 0..1: how far the clapper has to swing to reach a tube.
    pub spacing: f32,
    /// 0..1: the clapper's weight.
    pub clapper: f32,
    /// 0..1: the size of the sail.
    pub sail: f32,
    /// 0..1.5: how hard the wind blows.
    pub wind: f32,
    /// 0..1: how much it comes in gusts.
    pub gusts: f32,
    /// Each tube's length relative to the longest (0..1].
    pub lengths: [f32; MAX_TUBES],
}

impl Shape {
    /// The clapper's radius: what is left of the ring after the gap.
    pub fn clapper_r(&self) -> f32 {
        1.0 - TUBE_R - self.gap()
    }

    pub fn gap(&self) -> f32 {
        0.05 + 0.45 * self.spacing.clamp(0.0, 1.0)
    }

    /// Where tube `i` hangs (seen from above, at rest).
    pub fn rest(&self, i: usize) -> [f32; 2] {
        let a = std::f32::consts::TAU * i as f32 / self.tubes.max(1) as f32 - std::f32::consts::FRAC_PI_2 + 0.3;
        [a.cos(), a.sin()]
    }
}

pub struct ChimeSim {
    /// The clapper's place and speed (from the centre).
    pub clapper: [f32; 2],
    cv: [f32; 2],
    /// Each tube's swing away from where it hangs, and its speed.
    pub tube: [[f32; 2]; MAX_TUBES],
    tv: [[f32; 2]; MAX_TUBES],
    /// The wind now: strength and direction (radians).
    pub wind: f32,
    pub dir: f32,
    gust: f32,
    dir_drift: f32,
    turb: [f32; 2],
    /// Seconds since each tube was struck (a tube rests against the
    /// clapper without ringing again and again).
    since: [f32; MAX_TUBES],
    rng: u32,
}

impl Default for ChimeSim {
    fn default() -> Self {
        Self::new(0x51_3E_C4_1A)
    }
}

impl ChimeSim {
    pub fn new(seed: u32) -> Self {
        Self { clapper: [0.0; 2], cv: [0.0; 2], tube: [[0.0; 2]; MAX_TUBES], tv: [[0.0; 2]; MAX_TUBES], wind: 0.0, dir: 0.4, gust: 0.0, dir_drift: 0.0, turb: [0.0; 2], since: [1.0; MAX_TUBES], rng: seed | 1 }
    }

    fn noise(&mut self) -> f32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        (self.rng >> 8) as f32 / 8_388_608.0 - 1.0
    }

    /// A hand on tube `i` (a played note): it swings out and rings.
    pub fn nudge(&mut self, s: &Shape, i: usize, speed: f32) {
        if i < s.tubes {
            let r = s.rest(i);
            self.tv[i][0] += r[0] * speed * 0.6;
            self.tv[i][1] += r[1] * speed * 0.6;
            self.since[i] = 0.0;
        }
    }

    /// Runs `secs`; `strike(tube, speed, at)` for every collision, `at` in
    /// seconds from the start of this run. `blowing` false lets the wind die.
    pub fn run(&mut self, s: &Shape, secs: f32, blowing: bool, strike: &mut dyn FnMut(usize, f32, f32)) {
        let n = s.tubes.clamp(1, MAX_TUBES);
        let steps = (secs / STEP).round().max(1.0) as usize;
        let dt = secs / steps as f32;
        let mc = 0.6 + 1.6 * s.clapper.clamp(0.0, 1.0);
        let sail = 0.35 + 1.3 * s.sail.clamp(0.0, 1.0);
        let reach = s.clapper_r() + TUBE_R;
        // Wind changes over seconds: gusts and direction as slow random walks.
        let gust_k = (dt / 2.5).min(1.0);
        let dir_k = (dt / 6.0).min(1.0);
        let turb_k = (dt / 0.3).min(1.0);
        let sq = dt.sqrt();
        for step in 0..steps {
            self.gust += -gust_k * self.gust + 1.1 * sq * self.noise();
            self.dir_drift += -dir_k * self.dir_drift + 0.35 * sq * self.noise();
            let target = if blowing { (s.wind * (1.0 + s.gusts * 1.2 * self.gust)).clamp(0.0, 2.0) } else { 0.0 };
            self.wind += (target - self.wind) * (dt / 0.4).min(1.0);
            self.dir = 0.4 + self.dir_drift;
            for k in 0..2 {
                let r = self.noise();
                self.turb[k] += -turb_k * self.turb[k] + 2.0 * sq * r;
            }
            let (dc, ds) = (self.dir.cos(), self.dir.sin());
            let push = 4.5 * self.wind * sail;
            let rough = 7.0 * self.wind * sail;
            // The clapper: a pendulum of about 0.55 Hz with the sail in the wind.
            let w2 = (std::f32::consts::TAU * 0.55).powi(2);
            for k in 0..2 {
                let f = push * [dc, ds][k] + rough * self.turb[k];
                let a = -w2 * self.clapper[k] - 0.7 * self.cv[k] + f / mc;
                self.cv[k] += a * dt;
            }
            for k in 0..2 {
                self.clapper[k] += self.cv[k] * dt;
            }
            // The tubes: shorter ones swing faster, and catch a little wind.
            for i in 0..n {
                let len = s.lengths[i].clamp(0.2, 1.0);
                let w2 = (std::f32::consts::TAU * 0.75 / len.sqrt()).powi(2);
                let mt = 0.5 + 0.7 * len;
                for k in 0..2 {
                    let f = 0.35 * push * [dc, ds][k] * len + 0.5 * rough * self.turb[k];
                    let a = -w2 * self.tube[i][k] - 0.5 * self.tv[i][k] + f / mt;
                    self.tv[i][k] += a * dt;
                }
                for k in 0..2 {
                    self.tube[i][k] += self.tv[i][k] * dt;
                }
                self.since[i] += dt;
                // The clapper and this tube meet?
                let r = s.rest(i);
                let rel = [self.clapper[0] - (r[0] + self.tube[i][0]), self.clapper[1] - (r[1] + self.tube[i][1])];
                let dist = (rel[0] * rel[0] + rel[1] * rel[1]).sqrt().max(1e-6);
                if dist < reach {
                    let nrm = [rel[0] / dist, rel[1] / dist];
                    let vn = (self.cv[0] - self.tv[i][0]) * nrm[0] + (self.cv[1] - self.tv[i][1]) * nrm[1];
                    if vn < 0.0 {
                        // A bounce, with momentum shared by their weights.
                        let j = -(1.0 + 0.5) * vn / (1.0 / mc + 1.0 / mt);
                        for k in 0..2 {
                            self.cv[k] += j * nrm[k] / mc;
                            self.tv[i][k] -= j * nrm[k] / mt;
                        }
                        let speed = -vn;
                        if speed > 0.03 && self.since[i] > 0.05 {
                            self.since[i] = 0.0;
                            strike(i, speed, step as f32 * dt);
                        }
                    }
                    // Not inside each other.
                    let over = reach - dist;
                    for k in 0..2 {
                        self.clapper[k] += nrm[k] * over * mt / (mc + mt);
                        self.tube[i][k] -= nrm[k] * over * mc / (mc + mt);
                    }
                }
            }
            // Strings are only so long.
            let lim = |v: &mut [f32; 2], max: f32| {
                let d = (v[0] * v[0] + v[1] * v[1]).sqrt();
                if d > max {
                    v[0] *= max / d;
                    v[1] *= max / d;
                }
            };
            lim(&mut self.clapper, 1.6);
            for t in self.tube.iter_mut().take(n) {
                lim(t, 0.6);
            }
        }
    }
}

/// Tube lengths for frequencies `f` (Hz): a free bar sounds at a pitch
/// that goes with one over its length squared, so the length goes with one
/// over the square root of the frequency. The longest is 1.
pub fn lengths_for(f: &[f32]) -> [f32; MAX_TUBES] {
    let lowest = f.iter().copied().fold(f32::MAX, f32::min).max(1.0);
    std::array::from_fn(|i| f.get(i).map(|&x| (lowest / x.max(1.0)).sqrt()).unwrap_or(1.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shape(wind: f32) -> Shape {
        Shape { tubes: 6, spacing: 0.5, clapper: 0.5, sail: 0.5, wind, gusts: 0.6, lengths: lengths_for(&[440.0, 494.0, 554.0, 659.0, 740.0, 880.0]) }
    }

    fn strikes(s: &Shape, secs: f32) -> Vec<(usize, f32)> {
        let mut sim = ChimeSim::new(7);
        let mut out = Vec::new();
        sim.run(s, secs, true, &mut |i, v, _| out.push((i, v)));
        out
    }

    #[test]
    #[ignore]
    fn report_rates() {
        for w in [0.1f32, 0.25, 0.45, 0.7, 1.0, 1.5] {
            let v = strikes(&shape(w), 60.0);
            let mean = v.iter().map(|x| x.1).sum::<f32>() / v.len().max(1) as f32;
            let max = v.iter().map(|x| x.1).fold(0.0f32, f32::max);
            eprintln!("wind {w}: {} strikes a minute, mean speed {mean:.2}, max {max:.2}", v.len());
        }
    }

    #[test]
    fn no_wind_no_chimes() {
        assert!(strikes(&shape(0.0), 20.0).is_empty());
    }

    #[test]
    fn more_wind_more_chimes_and_harder() {
        let soft = strikes(&shape(0.25), 60.0);
        let hard = strikes(&shape(0.9), 60.0);
        assert!(hard.len() > soft.len(), "{} vs {}", soft.len(), hard.len());
        let mean = |v: &[(usize, f32)]| v.iter().map(|x| x.1).sum::<f32>() / v.len().max(1) as f32;
        assert!(mean(&hard) > mean(&soft));
        // The wind comes from one side, but over a minute several tubes ring.
        let mut rung: Vec<usize> = hard.iter().map(|x| x.0).collect();
        rung.sort();
        rung.dedup();
        assert!(rung.len() >= 3, "{rung:?}");
    }

    #[test]
    fn a_wider_gap_rings_less() {
        let mut close = shape(0.6);
        close.spacing = 0.1;
        let mut far = shape(0.6);
        far.spacing = 0.9;
        assert!(strikes(&close, 60.0).len() > strikes(&far, 60.0).len());
    }

    #[test]
    fn the_clapper_never_passes_through_a_tube() {
        let s = shape(1.5);
        let mut sim = ChimeSim::new(3);
        for _ in 0..2000 {
            sim.run(&s, 0.01, true, &mut |_, _, _| {});
            for i in 0..s.tubes {
                let r = s.rest(i);
                let d = ((sim.clapper[0] - r[0] - sim.tube[i][0]).powi(2) + (sim.clapper[1] - r[1] - sim.tube[i][1]).powi(2)).sqrt();
                assert!(d > s.clapper_r() + TUBE_R - 0.05, "{d}");
            }
        }
    }
}

//! Gate: lets sound through above a threshold and turns it down below it,
//! with hysteresis (Return), attack, hold, release and a floor.

use super::Follower;
use crate::dsp::effects::FxCtx;
use crate::dsp::{db_to_gain, gain_to_db};
use crate::params::{ParamSpec, Unit};

pub const THRESHOLD: usize = 0;
pub const RETURN: usize = 1;
pub const ATTACK: usize = 2;
pub const HOLD: usize = 3;
pub const RELEASE: usize = 4;
pub const FLOOR: usize = 5;

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::lin("Threshold", "Gate", -80.0, 0.0, -40.0, Unit::Db),
    ParamSpec::lin("Return", "Gate", 0.0, 24.0, 3.0, Unit::Db),
    ParamSpec::log("Attack", "Envelope", 0.05, 100.0, 0.5, Unit::Ms),
    ParamSpec::log("Hold", "Envelope", 1.0, 1000.0, 20.0, Unit::Ms),
    ParamSpec::log("Release", "Envelope", 1.0, 3000.0, 100.0, Unit::Ms),
    ParamSpec::lin("Floor", "Gate", -80.0, 0.0, -80.0, Unit::Db),
];

#[derive(Default)]
pub struct Gate {
    detector: Follower,
    open: bool,
    hold_left: f32,
    gain: f32,
}

impl Gate {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn process(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32], ctx: &FxCtx) {
        let n = l.len().min(r.len());
        let thr = p[THRESHOLD];
        let close_at = thr - p[RETURN].max(0.0);
        let floor = if p[FLOOR] <= -79.9 { 0.0 } else { db_to_gain(p[FLOOR]) };
        let att = 1.0 - (-1.0 / (p[ATTACK].max(0.01) * 0.001 * ctx.sr)).exp();
        let rel = 1.0 - (-1.0 / (p[RELEASE].max(0.1) * 0.001 * ctx.sr)).exp();
        let hold = p[HOLD].max(0.0) * 0.001 * ctx.sr;
        self.detector.set(0.0002, 0.03, ctx.sr);
        for i in 0..n {
            let level_db = gain_to_db(self.detector.tick(l[i].abs().max(r[i].abs())));
            if level_db > thr {
                self.open = true;
                self.hold_left = hold;
            } else if self.open && level_db < close_at {
                if self.hold_left > 0.0 {
                    self.hold_left -= 1.0;
                } else {
                    self.open = false;
                }
            }
            let (target, c) = if self.open { (1.0, att) } else { (floor, rel) };
            self.gain += (target - self.gain) * c;
            l[i] *= self.gain;
            r[i] *= self.gain;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_util::*;
    use super::*;

    #[test]
    fn quiet_sound_is_shut_out_and_loud_gets_through() {
        let mut g = Gate::new();
        let p = crate::params::defaults(SPECS);
        let mut quiet = tone(200.0, 0.5, 0.001);
        let mut q2 = quiet.clone();
        g.process(&p, &mut quiet, &mut q2, &ctx());
        assert!(peak(&quiet[12000..]) < 1e-5);
        let mut loud = tone(200.0, 0.5, 0.5);
        let mut l2 = loud.clone();
        g.process(&p, &mut loud, &mut l2, &ctx());
        assert!(peak(&loud[4800..]) > 0.45);
    }
}

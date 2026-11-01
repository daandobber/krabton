//! Fryer: Analog Princess's FRY section as an effect, for any track or
//! sample (see `dsp::fry`). What comes in goes into an 808's distortion and
//! that into another (*808s*: one, two or three deep; *Heat*, *Distortion*),
//! through a ring modulator (*Ring*, free in Hz or in the song's time), a
//! crusher (*Bits*, *Downsample*), with the subliminal shadow under it (a
//! quiet copy backwards and pitched down, low-passed: *Subliminal*, *Sub
//! Pitch*, *Sub Reverse*, *Sub Tone*), then the time effects (a *Stutter*,
//! a *Tape Stop*, *Channel Surf*: now and then, or while held) and a clip
//! into the *Ceiling* pushed by *Clip*, behind a limiter that never lets a
//! peak over it. *Deep Fry* turns most of it up at once.

use crate::dsp::effects::FxCtx;
use crate::dsp::fry::{now_and_then, stutter_slice, Bus, BusKnobs, Cascade, Shadow};
use crate::dsp::princess::Dist;
use crate::dsp::{db_to_gain, Rng};
use crate::params::{ParamSpec, Unit};

pub const DEEP_FRY: usize = 0;
pub const HEAT: usize = 1;
pub const STAGES: usize = 2;
pub const DIST: usize = 3;
pub const RING: usize = 4;
pub const RING_SYNC: usize = 5;
pub const RING_HZ: usize = 6;
pub const CLIP: usize = 7;
pub const CLIP_MODE: usize = 8;
pub const CEILING: usize = 9;
pub const BITS: usize = 10;
pub const DOWN: usize = 11;
pub const SUBLIMINAL: usize = 12;
pub const SUB_PITCH: usize = 13;
pub const SUB_REVERSE: usize = 14;
pub const SUB_TONE: usize = 15;
pub const SURF: usize = 16;
pub const SURF_HOLD: usize = 17;
pub const STUTTER: usize = 18;
pub const STUTTER_RATE: usize = 19;
pub const STUTTER_HOLD: usize = 20;
pub const TAPE: usize = 21;
pub const TAPE_TIME: usize = 22;
pub const TAPE_HOLD: usize = 23;
pub const MIX: usize = 24;
pub const OUTPUT: usize = 25;

pub const STAGE_NAMES: &[&str] = &["One 808", "808 Into 808", "Three Deep"];
/// The ring in the song's time: so many times a bar (Free: Ring Hz).
pub const RING_SYNCS: &[&str] = &["Free", "1/16", "1/32", "1/64", "1/128", "1/256"];
const RING_PER_BAR: [f32; 6] = [0.0, 16.0, 32.0, 64.0, 128.0, 256.0];
/// How far down the shadow is (semitones).
pub const SUB_PITCHES: &[&str] = &["-5", "-7", "-12", "-24"];
const SUB_SEMIS: [f32; 4] = [5.0, 7.0, 12.0, 24.0];

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::lin("Deep Fry", "Fry", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Heat", "808", 0.0, 1.0, 0.35, Unit::Percent),
    ParamSpec::choice("808s", "808", STAGE_NAMES, 1),
    ParamSpec::choice("Distortion", "808", crate::dsp::princess_machine::DISTS, 1),
    ParamSpec::lin("Ring", "Ring", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::choice("Ring Sync", "Ring", RING_SYNCS, 0),
    ParamSpec::log("Ring Hz", "Ring", 30.0, 3000.0, 180.0, Unit::Hz),
    ParamSpec::lin("Clip", "Clip", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::choice("Clip Mode", "Clip", &["Soft", "Hard"], 1),
    ParamSpec::lin("Ceiling", "Clip", -12.0, 0.0, -1.0, Unit::Db),
    ParamSpec::int("Bits", "Crush", 2.0, 16.0, 16.0, Unit::None),
    ParamSpec::log("Downsample", "Crush", 1.0, 32.0, 1.0, Unit::None),
    ParamSpec::lin("Subliminal", "Subliminal", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::choice("Sub Pitch", "Subliminal", SUB_PITCHES, 2),
    ParamSpec::toggle("Sub Reverse", "Subliminal", true),
    ParamSpec::log("Sub Tone", "Subliminal", 150.0, 4000.0, 700.0, Unit::Hz),
    ParamSpec::lin("Channel Surf", "Time", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::toggle("Surf Hold", "Time", false),
    ParamSpec::lin("Stutter", "Time", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::choice("Stutter Rate", "Time", crate::dsp::princess_machine::STUTTER_RATES, 1),
    ParamSpec::toggle("Stutter Hold", "Time", false),
    ParamSpec::lin("Tape Stop", "Time", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::choice("Tape Time", "Time", crate::dsp::princess_machine::TAPE_TIMES, 2),
    ParamSpec::toggle("Tape Hold", "Time", false),
    ParamSpec::lin("Mix", "Output", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::lin("Output", "Output", -24.0, 6.0, 0.0, Unit::Db),
];

/// How long the shadow's grains are (s).
const SHADOW_GRAIN: f32 = 0.32;
/// How loud the shadow is at Subliminal 100 %, under the sound.
const SHADOW_GAIN: f32 = 0.5;

pub struct Fryer {
    sr: f32,
    bus: Box<Bus>,
    cascade: Cascade,
    shadow: Box<Shadow>,
    rng: Rng,
    /// The beat it keeps (the song's while it plays, its own after) and
    /// the last sixteenth it threw its dice on.
    beat: f64,
    last_step: i64,
    tone: f32,
}

impl Fryer {
    pub fn new(sr: f32) -> Self {
        Self { sr, bus: Box::new(Bus::new(sr)), cascade: Cascade::new(sr), shadow: Box::new(Shadow::new(sr)), rng: Rng::new(0xF2_7E25), beat: 0.0, last_step: i64::MIN, tone: 0.0 }
    }

    /// Whether a stutter, a tape stop or a channel flip is going, and how
    /// much the limiter takes.
    pub fn lights(&self) -> [f32; 4] {
        self.bus.lights()
    }

    pub fn process(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32], ctx: &FxCtx) {
        if p.len() < SPECS.len() {
            return;
        }
        let n = l.len().min(r.len());
        let sr = self.sr;
        let get = |i: usize| p[i];
        let df = get(DEEP_FRY).clamp(0.0, 1.0);
        let heat = get(HEAT).clamp(0.0, 1.0);
        let heat = heat + df * 0.6 * (1.0 - heat);
        let stages = (get(STAGES).round().max(0.0) as usize).min(2) + 1;
        let dist = Dist::from_index(get(DIST));
        let bits = (get(BITS).round() - df * 8.0).clamp(2.0, 16.0);
        let down = get(DOWN).clamp(1.0, 32.0) * (1.0 + df * 5.0);
        let ring = get(RING).clamp(0.0, 1.0);
        let sync = (get(RING_SYNC).round().max(0.0) as usize).min(RING_PER_BAR.len() - 1);
        let ring_hz = if sync == 0 { get(RING_HZ).clamp(1.0, 3000.0) } else { ctx.bpm.max(1.0) / 60.0 / 4.0 * RING_PER_BAR[sync] };
        let clip = get(CLIP).clamp(0.0, 1.0);
        let clip = clip + df * 0.6 * (1.0 - clip);
        let spb = 60.0 / ctx.bpm.max(1.0) * sr;
        let bk = BusKnobs {
            drive: df * 0.4,
            steps: 2f32.powf(bits - 1.0),
            rate: 1.0 / down,
            crush: bits < 16.0 || down > 1.001,
            ring: ring + df * 0.25 * (1.0 - ring),
            ring_inc: ring_hz.min(sr * 0.45) / sr,
            push: 1.0 + 5.0 * clip,
            hard: get(CLIP_MODE) >= 0.5,
            ceiling: db_to_gain(get(CEILING).clamp(-12.0, 0.0)),
            volume: 1.0,
            lim_rel: 1.0 - (-1.0 / (0.08 * sr)).exp(),
            stutter_hold: get(STUTTER_HOLD) >= 0.5,
            stutter_slice: (stutter_slice(get(STUTTER_RATE), &mut self.rng) * spb) as u32,
            tape_hold: get(TAPE_HOLD) >= 0.5,
            tape_len: ([0.25, 0.5, 1.0, 2.0][(get(TAPE_TIME).round().max(0.0) as usize).min(3)] * spb) as u32,
            surf_hold: get(SURF_HOLD) >= 0.5,
        };
        self.bus.holds(&bk);
        let sub = (get(SUBLIMINAL) + df * 0.2).clamp(0.0, 1.0) * SHADOW_GAIN;
        let semis = SUB_SEMIS[(get(SUB_PITCH).round().max(0.0) as usize).min(SUB_SEMIS.len() - 1)];
        let reverse = get(SUB_REVERSE) >= 0.5;
        let tone = get(SUB_TONE).clamp(100.0, 8000.0);
        if tone != self.tone {
            self.tone = tone;
            self.shadow.set_tone(tone);
        }
        let amounts = [(get(STUTTER) + df * 0.15).clamp(0.0, 1.0), (get(TAPE) + df * 0.05).clamp(0.0, 1.0), (get(SURF) + df * 0.12).clamp(0.0, 1.0)];
        let mix = get(MIX).clamp(0.0, 1.0);
        let out = db_to_gain(get(OUTPUT).clamp(-24.0, 6.0));
        if ctx.playing {
            self.beat = ctx.beat;
        }
        let bps = ctx.bpm.max(1.0) as f64 / 60.0 / sr as f64;
        for i in 0..n {
            // The dice, on the sixteenths.
            let step = (self.beat * 4.0).floor() as i64;
            if step != self.last_step {
                if self.last_step != i64::MIN {
                    if let Some(fx) = now_and_then(&mut self.rng, step.rem_euclid(16), amounts, get(TAPE_TIME), get(STUTTER_RATE), spb) {
                        self.bus.start(fx);
                    }
                }
                self.last_step = step;
            }
            self.beat += bps;
            let x = [l[i], r[i]];
            let hot = if heat > 0.001 { self.cascade.tick(x, heat, stages, dist) } else { x };
            let mut y = self.bus.fry(hot, &bk);
            let s = self.shadow.tick((x[0] + x[1]) * 0.5, semis, reverse, SHADOW_GRAIN);
            if sub > 0.0 {
                y = [y[0] + s * sub, y[1] + s * sub];
            }
            let y = self.bus.tick(y, &bk);
            l[i] = (x[0] * (1.0 - mix) + y[0] * mix) * out;
            r[i] = (x[1] * (1.0 - mix) + y[1] * mix) * out;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsp::TAU;

    const SR: f32 = 48_000.0;

    fn ctx() -> FxCtx {
        FxCtx { sr: SR, bpm: 120.0, beat: 0.0, playing: false }
    }

    fn params(set: &[(usize, f32)]) -> Vec<f32> {
        let mut p = crate::params::defaults(SPECS);
        p[HEAT] = 0.0;
        p[CLIP] = 0.0;
        p[CEILING] = 0.0;
        for &(i, v) in set {
            p[i] = v;
        }
        p
    }

    fn sine(hz: f32, amp: f32, secs: f32) -> Vec<f32> {
        (0..(secs * SR) as usize).map(|i| amp * (TAU * hz * i as f32 / SR).sin()).collect()
    }

    fn run(f: &mut Fryer, p: &[f32], x: &[f32]) -> Vec<f32> {
        let mut out = Vec::with_capacity(x.len());
        for c in x.chunks(256) {
            let (mut l, mut r) = (c.to_vec(), c.to_vec());
            f.process(p, &mut l, &mut r, &ctx());
            out.extend_from_slice(&l);
        }
        out
    }

    /// The strength of `hz` in `x` (Goertzel), as an amplitude.
    fn level(x: &[f32], hz: f32) -> f32 {
        let w = TAU * hz / SR;
        let c = 2.0 * w.cos();
        let (mut s1, mut s2) = (0.0f32, 0.0f32);
        for &v in x {
            let s = v + c * s1 - s2;
            s2 = s1;
            s1 = s;
        }
        (s1 * s1 + s2 * s2 - c * s1 * s2).max(0.0).sqrt() * 2.0 / x.len() as f32
    }

    fn peak(x: &[f32]) -> f32 {
        x.iter().fold(0.0f32, |m, v| m.max(v.abs()))
    }

    #[test]
    fn the_ceiling_holds_however_hot_it_goes() {
        for (mode, df) in [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)] {
            let p = params(&[(CEILING, -6.0), (CLIP, 1.0), (CLIP_MODE, mode), (HEAT, 1.0), (DEEP_FRY, df), (SUBLIMINAL, 1.0)]);
            let mut f = Fryer::new(SR);
            let x = sine(80.0, 3.0, 1.0);
            let y = run(&mut f, &p, &x);
            assert!(y.iter().all(|v| v.is_finite()));
            assert!(peak(&y) <= db_to_gain(-6.0) + 1e-5, "mode {mode} fry {df}: {}", peak(&y));
            assert!(peak(&y[24_000..]) > db_to_gain(-6.0) * 0.8, "pushed into it");
        }
    }

    #[test]
    fn the_ring_puts_sidebands_either_side() {
        let p = params(&[(RING, 1.0), (RING_HZ, 150.0)]);
        let mut f = Fryer::new(SR);
        let x = sine(1000.0, 0.3, 1.0);
        let y = run(&mut f, &p, &x);
        let w = &y[4800..];
        let (lo, hi, mid) = (level(w, 850.0), level(w, 1150.0), level(w, 1000.0));
        assert!(lo > 0.1 && hi > 0.1, "sidebands at 850 and 1150 Hz: {lo} {hi}");
        assert!(mid < 0.02, "the carrier mostly gone: {mid}");
        // In the song's time: 1/64 at 120 BPM is 32 Hz.
        let p = params(&[(RING, 1.0), (RING_SYNC, 3.0)]);
        let mut f = Fryer::new(SR);
        let y = run(&mut f, &p, &x);
        assert!(level(&y[4800..], 1032.0) > 0.1 && level(&y[4800..], 968.0) > 0.1);
    }

    #[test]
    fn the_808s_cascade_grinds_harder() {
        let x = sine(110.0, 0.2, 0.5);
        let harm = |stages: f32| {
            let p = params(&[(HEAT, 0.25), (STAGES, stages)]);
            let mut f = Fryer::new(SR);
            let y = run(&mut f, &p, &x);
            let w = &y[4800..];
            (level(w, 330.0) + level(w, 550.0)) / level(w, 110.0).max(1e-6)
        };
        let (one, two) = (harm(0.0), harm(1.0));
        assert!(one > 0.1, "an 808's distortion: {one}");
        assert!(two > one * 1.1, "808 into 808 is harder: {two} against {one}");
    }

    #[test]
    fn the_subliminal_shadow_is_quiet_low_and_under_it() {
        let x = sine(440.0, 0.5, 2.0);
        let mut f = Fryer::new(SR);
        let dry = run(&mut f, &params(&[]), &x);
        let p = params(&[(SUBLIMINAL, 1.0), (SUB_PITCH, 2.0), (SUB_TONE, 4000.0)]);
        let mut f = Fryer::new(SR);
        let wet = run(&mut f, &p, &x);
        let w = 24_000..96_000;
        let shadow: Vec<f32> = wet[w.clone()].iter().zip(&dry[w]).map(|(a, b)| a - b).collect();
        // An octave down, at about half the level, the sound itself untouched.
        let down = level(&shadow, 220.0);
        assert!(down > 0.08 && down < 0.35, "an octave down under it: {down}");
        assert!(level(&shadow, 440.0) < 0.05, "not the sound again: {}", level(&shadow, 440.0));
        // Off: nothing under it.
        let mut f = Fryer::new(SR);
        let again = run(&mut f, &params(&[(SUBLIMINAL, 0.0)]), &x);
        assert!(again.iter().zip(&dry).all(|(a, b)| (a - b).abs() < 1e-6));
        // Low-passed: a high sound's shadow is much weaker.
        let hi = sine(3000.0, 0.5, 2.0);
        let p = params(&[(SUBLIMINAL, 1.0), (SUB_PITCH, 2.0), (SUB_TONE, 300.0)]);
        let mut f = Fryer::new(SR);
        let wet = run(&mut f, &p, &hi);
        let mut f = Fryer::new(SR);
        let dry = run(&mut f, &params(&[]), &hi);
        let shadow: Vec<f32> = wet[24_000..96_000].iter().zip(&dry[24_000..96_000]).map(|(a, b)| a - b).collect();
        assert!(level(&shadow, 1500.0) < 0.01, "{}", level(&shadow, 1500.0));
    }

    #[test]
    fn held_buttons_stutter_and_stop_the_tape() {
        let x = sine(330.0, 0.4, 2.0);
        let mut f = Fryer::new(SR);
        let dry = run(&mut f, &params(&[]), &x);
        let mut f = Fryer::new(SR);
        let y = run(&mut f, &params(&[(TAPE_HOLD, 1.0), (TAPE_TIME, 0.0)]), &x);
        let tail = &y[48_000..];
        assert!(tail.iter().map(|v| v * v).sum::<f32>() / (tail.len() as f32) < 1e-6, "stopped to nothing");
        let mut f = Fryer::new(SR);
        let y = run(&mut f, &params(&[(STUTTER_HOLD, 1.0)]), &x);
        assert!(y.iter().all(|v| v.is_finite()));
        assert_eq!(dry.len(), y.len());
    }
}

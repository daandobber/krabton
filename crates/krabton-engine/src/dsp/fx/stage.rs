//! Stage: puts a track somewhere in a room it shares with the other tracks
//! that have one, as heard from where the listener sits.
//!
//! Where the sound stands (across, and how deep into the room) gives the
//! direct sound its direction (panned, the near ear a little earlier, on
//! headphones the far ear in the head's shadow) and its distance (quieter
//! and duller further away, and later if you like). The walls, floor and
//! ceiling send back reflections from the sound's mirror images (first and
//! second order), each from its own direction, as late and as dull as its
//! way is long. Behind them the room's tail, as long as its size and walls
//! make it (Sabine), as loud against the direct sound as the room makes it:
//! further away, more room and less sound.
//!
//! The room (size, walls, reverb, the listener, ...) is the same for every
//! Stage in a set: the app keeps them equal.

use super::DelayLine;
use crate::dsp::effects::FxCtx;
use crate::params::{ParamSpec, Unit};
use std::f32::consts::FRAC_PI_4;

pub const ACROSS: usize = 0;
pub const DEPTH: usize = 1;
pub const SPREAD: usize = 2;
pub const SIZE: usize = 3;
pub const WALLS: usize = 4;
pub const TAIL: usize = 5;
pub const LISTENER: usize = 6;
pub const FALLOFF: usize = 7;
pub const AIR: usize = 8;
pub const FLIGHT: usize = 9;
pub const EARS: usize = 10;
pub const LEVEL: usize = 11;
/// The room's parameters: the same in every Stage of a set.
pub const ROOM: [usize; 8] = [SIZE, WALLS, TAIL, LISTENER, FALLOFF, AIR, FLIGHT, EARS];
pub const EARS_NAMES: &[&str] = &["Speakers", "Headphones"];

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::lin("Across", "Place", -1.0, 1.0, 0.0, Unit::Pan),
    ParamSpec::lin("Depth", "Place", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::lin("Spread", "Place", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::int("Room m", "Room", 4.0, 60.0, 16.0, Unit::None),
    ParamSpec::lin("Walls", "Room", 0.0, 1.0, 0.45, Unit::Percent),
    ParamSpec::lin("Reverb", "Room", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Listener", "Room", 0.0, 1.0, 0.8, Unit::Percent),
    ParamSpec::lin("Falloff", "Room", 0.0, 1.0, 0.7, Unit::Percent),
    ParamSpec::lin("Air", "Room", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::toggle("Delay", "Room", false),
    ParamSpec::choice("Ears", "Room", EARS_NAMES, 0),
    ParamSpec::lin("Level", "Place", -12.0, 12.0, 0.0, Unit::Db),
];

const C: f32 = 343.0;
/// Ears and sound above the floor (m).
const EAR_HEIGHT: f32 = 1.6;
const SOURCE_HEIGHT: f32 = 1.4;
/// The direct sound from the left and right of the source, then six first
/// and four second order reflections of its middle.
const TAPS: usize = 12;
const LINES: usize = 8;
const FDN_MS: [f32; LINES] = [29.7, 37.1, 41.1, 43.7, 53.3, 59.9, 67.1, 73.3];

/// The room's shape: width, depth, height (m).
pub fn room(p: &[f32]) -> (f32, f32, f32) {
    let w = p[SIZE].clamp(4.0, 60.0);
    (w, w * 1.4, (w * 0.4).clamp(2.6, 18.0))
}

/// Where the source stands and where the listener sits (x across, y from
/// the front wall, in metres).
pub fn places(p: &[f32]) -> ((f32, f32), (f32, f32)) {
    let (w, d, _) = room(p);
    let sx = p[ACROSS].clamp(-1.0, 1.0) * (w * 0.5 - 0.4);
    let sy = 0.3 + p[DEPTH].clamp(0.0, 1.0) * (d - 0.6);
    let ly = 0.3 + p[LISTENER].clamp(0.0, 1.0) * (d - 0.6);
    ((sx, sy), (0.0, ly))
}

/// How far from the listener a sound keeps its level as it comes in (m):
/// the middle of the stage, a third into the room. Nearer is louder.
pub fn reference(p: &[f32]) -> f32 {
    let (_, d, _) = room(p);
    let ((_, _), (lx, ly)) = places(p);
    let sy = 0.3 + 0.3 * (d - 0.6);
    ((lx * lx) + (sy - ly).powi(2) + (SOURCE_HEIGHT - EAR_HEIGHT).powi(2)).sqrt().max(1.0)
}

/// How far the source is from the listener (m).
pub fn distance(p: &[f32]) -> f32 {
    let ((sx, sy), (lx, ly)) = places(p);
    ((sx - lx).powi(2) + (sy - ly).powi(2) + (SOURCE_HEIGHT - EAR_HEIGHT).powi(2)).sqrt()
}

/// The room's reverberation time (s), after Sabine.
pub fn rt60(p: &[f32]) -> f32 {
    let (w, d, h) = room(p);
    let v = w * d * h;
    let s = 2.0 * (w * d + w * h + d * h);
    let alpha = absorption(p);
    (0.161 * v / (s * alpha)).clamp(0.15, 9.0)
}

fn absorption(p: &[f32]) -> f32 {
    0.06 + 0.8 * p[WALLS].clamp(0.0, 1.0)
}

/// Where a tap comes from, its gain at both ears, delay at both ears (s)
/// and how far up its sound reaches at both ears (Hz).
#[derive(Clone, Copy, Default)]
struct Target {
    gain: [f32; 2],
    delay: [f32; 2],
    cutoff: [f32; 2],
}

#[derive(Clone, Copy, Default)]
struct Tap {
    gain: [f32; 2],
    delay: [f32; 2],
    coef: [f32; 2],
    z: [f32; 2],
}

pub struct Stage {
    sr: f32,
    /// The left and right input, and their middle.
    lines: [DelayLine; 3],
    taps: [Tap; TAPS],
    ready: bool,
    fdn: [DelayLine; LINES],
    damp: [f32; LINES],
    tail_level: f32,
    /// Where the tail reads its input (glides as the sound moves).
    pre: f32,
}

impl Stage {
    pub fn new(sr: f32) -> Self {
        let len = (0.75 * sr) as usize + 8;
        let fdn_len = (FDN_MS[LINES - 1] * 0.001 * 3.2 * sr) as usize + 8;
        Self {
            sr,
            lines: [DelayLine::new(len), DelayLine::new(len), DelayLine::new(len)],
            taps: [Tap::default(); TAPS],
            ready: false,
            fdn: std::array::from_fn(|_| DelayLine::new(fdn_len)),
            damp: [0.0; LINES],
            tail_level: 0.0,
            pre: -1.0,
        }
    }

    pub fn clear(&mut self) {
        for l in self.lines.iter_mut().chain(self.fdn.iter_mut()) {
            l.clear();
        }
        self.damp = [0.0; LINES];
        for t in self.taps.iter_mut() {
            t.z = [0.0; 2];
        }
    }

    /// Where every tap should be now.
    fn targets(p: &[f32]) -> ([Target; TAPS], f32) {
        let (w, d, h) = room(p);
        let ((sx, sy), (lx, ly)) = places(p);
        let walls = p[WALLS].clamp(0.0, 1.0);
        let falloff = p[FALLOFF].clamp(0.0, 1.0);
        let air = p[AIR].clamp(0.0, 1.0);
        let phones = p[EARS].round() >= 1.0;
        let reflect = (1.0 - absorption(p)).max(0.0).sqrt();
        let half = p[SPREAD].clamp(0.0, 1.0) * (w * 0.12).min(3.0);
        let (sz, lz) = (SOURCE_HEIGHT, EAR_HEIGHT);
        // (x, y, z, order, from: 0 left, 1 right, 2 middle)
        let mut images = [(0.0f32, 0.0f32, 0.0f32, 0u8, 0u8); TAPS];
        images[0] = (sx - half, sy, sz, 0, 0);
        images[1] = (sx + half, sy, sz, 0, 1);
        let firsts = [(-w - sx, sy, sz), (w - sx, sy, sz), (sx, -sy, sz), (sx, 2.0 * d - sy, sz), (sx, sy, -sz), (sx, sy, 2.0 * h - sz)];
        for (k, &(x, y, z)) in firsts.iter().enumerate() {
            images[2 + k] = (x, y, z, 1, 2);
        }
        let seconds = [(-w - sx, -sy), (w - sx, -sy), (-w - sx, 2.0 * d - sy), (w - sx, 2.0 * d - sy)];
        for (k, &(x, y)) in seconds.iter().enumerate() {
            images[8 + k] = (x, y, sz, 2, 2);
        }
        let direct = ((sx - lx).powi(2) + (sy - ly).powi(2) + (sz - lz).powi(2)).sqrt();
        let reference = reference(p);
        let level = |r: f32| (reference / r.max(0.5)).powf(falloff);
        let mut out = [Target::default(); TAPS];
        for (t, &(x, y, z, order, _)) in out.iter_mut().zip(images.iter()) {
            let (vx, vy, vz) = (x - lx, y - ly, z - lz);
            let r = (vx * vx + vy * vy + vz * vz).sqrt().max(0.3);
            // The listener faces the front wall (y = 0).
            let az = vx.atan2(-vy);
            let side = az.sin();
            let behind = az.cos() < 0.0;
            let mut g = level(r) * reflect.powi(order as i32);
            if order == 0 {
                g *= if half > 0.0 { std::f32::consts::FRAC_1_SQRT_2 } else { 0.5 };
            } else {
                g *= 0.85;
            }
            let wall_cut = 16000.0 * (1.0 - 0.75 * walls).powi(order as i32);
            let air_cut = 20000.0 / (1.0 + air * r / 12.0);
            let mut cut = wall_cut.min(air_cut).max(900.0);
            if behind {
                cut *= 0.7;
            }
            let t0 = r / C;
            if phones {
                // The head: a lag of up to 0.66 ms, the far ear in its shadow.
                let lateral = side.clamp(-1.0, 1.0).asin();
                let itd = 0.0875 / C * (lateral + lateral.sin());
                let far = 1.0 - 0.45 * side.abs();
                let shadow = (cut * (1.0 - 0.8 * side.abs())).max(900.0);
                if side >= 0.0 {
                    t.gain = [g * far, g];
                    t.delay = [t0 + itd.abs(), t0];
                    t.cutoff = [shadow, cut];
                } else {
                    t.gain = [g, g * far];
                    t.delay = [t0, t0 + itd.abs()];
                    t.cutoff = [cut, shadow];
                }
            } else {
                // Speakers: panned at equal power, the near side a little earlier.
                let a = (side + 1.0) * FRAC_PI_4;
                t.gain = [g * a.cos(), g * a.sin()];
                let itd = 0.00025 * side;
                t.delay = [t0 + itd.max(0.0), t0 + (-itd).max(0.0)];
                t.cutoff = [cut, cut];
            }
        }
        (out, direct)
    }

    pub fn process(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32], ctx: &FxCtx) {
        let n = l.len().min(r.len());
        if p.len() < SPECS.len() {
            return;
        }
        let sr = ctx.sr.max(1.0);
        self.sr = sr;
        let (targets, direct) = Self::targets(p);
        // Without the delay the direct sound arrives at once, the rest as much later as its way is longer.
        // (the earliest arrival at either ear: all the others keep their lead on it)
        let first = targets.iter().flat_map(|t| t.delay).fold(f32::MAX, f32::min);
        let offset = if p[FLIGHT] >= 0.5 { 0.0 } else { first.min(direct / C) };
        let max = (self.lines[0].len() - 4) as f32;
        let coef = |hz: f32| 1.0 - (-std::f32::consts::TAU * hz.min(sr * 0.45) / sr).exp();
        let mut want = [Tap::default(); TAPS];
        for (w, t) in want.iter_mut().zip(targets.iter()) {
            for e in 0..2 {
                w.gain[e] = t.gain[e];
                w.delay[e] = (((t.delay[e] - offset) * sr).max(0.0) + 1.0).min(max);
                w.coef[e] = coef(t.cutoff[e]);
            }
        }
        if !self.ready {
            self.ready = true;
            for (cur, w) in self.taps.iter_mut().zip(want.iter()) {
                cur.gain = w.gain;
                cur.delay = w.delay;
                cur.coef = w.coef;
            }
        }
        // The tail: Sabine's time; as loud as the direct sound at the critical distance.
        let (w, d, h) = room(p);
        let t60 = rt60(p);
        let v = w * d * h;
        let critical = (0.057 * (v / t60).sqrt()).max(0.3);
        let falloff = p[FALLOFF].clamp(0.0, 1.0);
        let tail_want = 2.0 * p[TAIL].clamp(0.0, 1.0) * (reference(p) / critical.max(0.5)).powf(falloff);
        let scale = (w / 16.0).powf(0.6).clamp(0.45, 3.0);
        let mut lens = [0.0f32; LINES];
        let mut fb = [0.0f32; LINES];
        let mut norm = 0.0;
        for k in 0..LINES {
            lens[k] = (FDN_MS[k] * 0.001 * scale * sr).min((self.fdn[k].len() - 4) as f32);
            fb[k] = 10f32.powf(-3.0 * lens[k] / (t60 * sr));
            norm += 1.0 - fb[k] * fb[k];
        }
        // Equal power for any length of tail.
        let norm = (norm / LINES as f32).sqrt();
        let damp = coef(9000.0 * (1.0 - 0.7 * p[WALLS].clamp(0.0, 1.0)) + 1500.0);
        // The tail starts with the first reflections (a mean free path after the direct sound).
        let s = 2.0 * (w * d + w * h + d * h);
        let pre = ((4.0 * v / s) / C + direct / C - offset).max(0.0) * sr + 1.0;
        let pre = pre.min(max);
        if self.pre < 0.0 {
            self.pre = pre;
        }
        let level = 10f32.powf(p[LEVEL].clamp(-24.0, 24.0) / 20.0);
        let k_smooth = 1.0 - (-1.0 / (0.04 * sr)).exp();
        for i in 0..n {
            let (xl, xr) = (l[i], r[i]);
            self.lines[0].write(xl);
            self.lines[1].write(xr);
            self.lines[2].write(0.5 * (xl + xr));
            let mut out = [0.0f32; 2];
            for (j, (tap, w)) in self.taps.iter_mut().zip(want.iter()).enumerate() {
                let src = match j {
                    0 => 0,
                    1 => 1,
                    _ => 2,
                };
                for e in 0..2 {
                    tap.gain[e] += k_smooth * (w.gain[e] - tap.gain[e]);
                    tap.delay[e] += k_smooth * (w.delay[e] - tap.delay[e]);
                    tap.coef[e] += k_smooth * (w.coef[e] - tap.coef[e]);
                    let x = self.lines[src].read(tap.delay[e]);
                    tap.z[e] += tap.coef[e] * (x - tap.z[e]);
                    out[e] += tap.z[e] * tap.gain[e];
                }
            }
            // The tail (Householder feedback).
            self.tail_level += k_smooth * (tail_want - self.tail_level);
            self.pre += k_smooth * (pre - self.pre);
            let input = self.lines[2].read(self.pre);
            let mut y = [0.0f32; LINES];
            let mut sum = 0.0;
            for k in 0..LINES {
                let v = self.fdn[k].read(lens[k]);
                self.damp[k] += damp * (v - self.damp[k]);
                y[k] = self.damp[k] * fb[k];
                sum += y[k];
            }
            let hh = sum * (2.0 / LINES as f32);
            for k in 0..LINES {
                let feed = y[k] - hh + input * if k % 2 == 0 { 0.5 } else { -0.5 };
                self.fdn[k].write(if feed.is_finite() { feed } else { 0.0 });
            }
            let tl = (y[0] - y[2] + y[4] - y[6]) * 0.5;
            let tr = (y[1] - y[3] + y[5] - y[7]) * 0.5;
            let g = self.tail_level * norm * 1.6;
            l[i] = (out[0] + tl * g) * level;
            r[i] = (out[1] + tr * g) * level;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_util::*;
    use super::*;
    use crate::params::defaults;

    fn run(p: &[f32], input: &[f32]) -> (Vec<f32>, Vec<f32>) {
        let mut s = Stage::new(48000.0);
        let mut l = input.to_vec();
        let mut r = input.to_vec();
        for (cl, cr) in l.chunks_mut(256).zip(r.chunks_mut(256)) {
            s.process(p, cl, cr, &ctx());
        }
        (l, r)
    }

    fn click(len: usize) -> Vec<f32> {
        let mut v = vec![0.0; len];
        v[100] = 1.0;
        v
    }

    fn energy(v: &[f32]) -> f32 {
        v.iter().map(|x| x * x).sum()
    }

    #[test]
    fn a_sound_on_the_left_comes_from_the_left() {
        let mut p = defaults(SPECS);
        p[ACROSS] = -0.9;
        let (l, r) = run(&p, &click(4800));
        // The direct sound: the first few milliseconds.
        assert!(energy(&l[..400]) > 3.0 * energy(&r[..400]), "{} {}", energy(&l[..400]), energy(&r[..400]));
        p[ACROSS] = 0.9;
        let (l, r) = run(&p, &click(4800));
        assert!(energy(&r[..400]) > 3.0 * energy(&l[..400]));
        // On headphones the near ear hears it first.
        p[EARS] = 1.0;
        let (l, r) = run(&p, &click(4800));
        let first = |v: &[f32]| v.iter().position(|x| x.abs() > 0.02).unwrap();
        assert!(first(&r) < first(&l), "{} {}", first(&r), first(&l));
    }

    #[test]
    fn further_away_is_quieter_and_more_room() {
        let near = {
            let mut p = defaults(SPECS);
            p[DEPTH] = 0.75;
            p
        };
        let far = {
            let mut p = defaults(SPECS);
            p[DEPTH] = 0.0;
            p
        };
        assert!(distance(&far) > 2.0 * distance(&near), "{} {}", distance(&far), distance(&near));
        let (nl, _) = run(&near, &click(48000));
        let (fl, _) = run(&far, &click(48000));
        let direct = |v: &[f32]| energy(&v[..300]);
        let room = |v: &[f32]| energy(&v[2400..]);
        assert!(direct(&nl) > 2.0 * direct(&fl));
        // The direct sound against the room: much less of it far away.
        assert!(direct(&nl) / room(&nl) > 2.0 * direct(&fl) / room(&fl), "{} {}", direct(&nl) / room(&nl), direct(&fl) / room(&fl));
    }

    #[test]
    fn with_the_delay_a_far_sound_arrives_late() {
        let mut p = defaults(SPECS);
        p[FLIGHT] = 1.0;
        p[DEPTH] = 0.0;
        p[SIZE] = 30.0;
        let (l, _) = run(&p, &click(9600));
        let first = l.iter().position(|x| x.abs() > 1e-3).unwrap();
        let expect = 100.0 + distance(&p) / C * 48000.0;
        assert!((first as f32 - expect).abs() < 30.0, "{first} {expect}");
        p[FLIGHT] = 0.0;
        let (l, _) = run(&p, &click(9600));
        assert!(l.iter().position(|x| x.abs() > 1e-3).unwrap() < 110);
    }

    #[test]
    fn a_bigger_softer_room_and_a_smaller_harder_one() {
        let mut big = defaults(SPECS);
        big[SIZE] = 40.0;
        big[WALLS] = 0.1;
        let mut small = defaults(SPECS);
        small[SIZE] = 6.0;
        small[WALLS] = 0.8;
        assert!(rt60(&big) > 4.0 * rt60(&small), "{} {}", rt60(&big), rt60(&small));
        let (bl, _) = run(&big, &click(96000));
        let (sl, _) = run(&small, &click(96000));
        // A second later the big one still rings.
        assert!(energy(&bl[48000..]) > 20.0 * energy(&sl[48000..]));
        assert!(bl.iter().chain(sl.iter()).all(|x| x.is_finite() && x.abs() < 4.0));
    }

    #[test]
    fn in_the_middle_of_the_stage_it_keeps_about_its_level() {
        let p = defaults(SPECS);
        let input = tone(330.0, 1.0, 0.3);
        let (l, r) = run(&p, &input);
        let out = (rms(&l[24000..]) + rms(&r[24000..])) * 0.5;
        let db = 20.0 * (out / rms(&input[24000..])).log10();
        assert!(db.abs() < 4.0, "{db} dB");
    }

    #[test]
    fn moving_the_sound_glides_without_clicks() {
        let mut s = Stage::new(48000.0);
        let mut p = defaults(SPECS);
        let tone = tone(220.0, 1.0, 0.5);
        let mut out = Vec::new();
        for (b, chunk) in tone.chunks(256).enumerate() {
            if b == 60 {
                p[ACROSS] = 1.0;
                p[DEPTH] = 0.9;
            }
            let mut l = chunk.to_vec();
            let mut r = chunk.to_vec();
            s.process(&p, &mut l, &mut r, &ctx());
            out.extend(l);
        }
        // A tone bends smoothly (its second difference stays tiny); a click would not.
        let bend = out.windows(3).skip(4800).map(|w| (w[0] - 2.0 * w[1] + w[2]).abs()).fold(0.0f32, f32::max);
        assert!(bend < 0.02, "{bend}");
    }
}

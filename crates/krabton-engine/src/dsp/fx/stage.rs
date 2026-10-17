//! Stage: a sound somewhere in a room, as heard from where the listener
//! sits. The same processing places the tracks on the project's Map (see
//! `MapRoom` and `Place`), where the room is shared and off unless asked.
//!
//! Where the sound stands gives the direct sound its direction (panned like
//! a 3D panner: well off to one side is all the way there; the near ear a
//! little earlier; on headphones the far ear in the head's shadow) and its
//! distance (quieter, duller and narrower further away, and later if you
//! like). A stereo sound keeps its width: in the middle of the stage at full
//! width it comes out as it went in. Which way it faces matters as with an
//! instrument: turned away it is quieter and duller, and there is more room
//! in it. The walls, floor and ceiling send back reflections from the
//! sound's mirror images (first and second order), each from its own
//! direction, as late and dull as its way is long, as loud as the sound
//! sends that way; behind them the room's tail (Sabine), as loud against
//! the direct sound as the room makes it.

use super::DelayLine;
use crate::dsp::effects::FxCtx;
use crate::params::{ParamSpec, Unit};
use std::f32::consts::{FRAC_PI_4, TAU};

pub const ACROSS: usize = 0;
pub const DEPTH: usize = 1;
pub const WIDTH: usize = 2;
pub const SIZE: usize = 3;
pub const WALLS: usize = 4;
pub const TAIL: usize = 5;
pub const LISTENER: usize = 6;
pub const FALLOFF: usize = 7;
pub const AIR: usize = 8;
pub const FLIGHT: usize = 9;
pub const EARS: usize = 10;
pub const LEVEL: usize = 11;
pub const FACING: usize = 12;
pub const BEAM: usize = 13;
pub const ROOM_AMOUNT: usize = 14;
pub const EARS_NAMES: &[&str] = &["Speakers", "Headphones"];

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::lin("Across", "Place", -1.0, 1.0, 0.0, Unit::Pan),
    ParamSpec::lin("Depth", "Place", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::lin("Width", "Place", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::int("Room m", "Room", 4.0, 60.0, 16.0, Unit::None),
    ParamSpec::lin("Walls", "Room", 0.0, 1.0, 0.45, Unit::Percent),
    ParamSpec::lin("Reverb", "Room", 0.0, 1.0, 0.35, Unit::Percent),
    ParamSpec::lin("Listener", "Room", 0.0, 1.0, 0.6, Unit::Percent),
    ParamSpec::lin("Falloff", "Room", 0.0, 1.0, 0.7, Unit::Percent),
    ParamSpec::lin("Air", "Room", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::toggle("Delay", "Room", false),
    ParamSpec::choice("Ears", "Room", EARS_NAMES, 0),
    ParamSpec::lin("Level", "Place", -12.0, 12.0, 0.0, Unit::Db),
    ParamSpec::lin("Facing", "Place", -180.0, 180.0, 0.0, Unit::None),
    ParamSpec::lin("Beam", "Place", 0.0, 1.0, 0.4, Unit::Percent),
    ParamSpec::lin("Room", "Room", 0.0, 1.0, 0.7, Unit::Percent),
];

const C: f32 = 343.0;
/// Ears and sound above the floor (m).
const EAR_HEIGHT: f32 = 1.6;
const SOURCE_HEIGHT: f32 = 1.4;
/// So far to one side (degrees) is all the way to that side.
const FULL_SIDE: f32 = 55.0;
/// The direct sound of the left and right input, then six first and four
/// second order reflections of their middle.
const TAPS: usize = 12;
const LINES: usize = 8;
const FDN_MS: [f32; LINES] = [29.7, 37.1, 41.1, 43.7, 53.3, 59.9, 67.1, 73.3];

/// The room's shape: width, depth, height (m).
pub fn room(p: &[f32]) -> (f32, f32, f32) {
    let w = p[SIZE].clamp(4.0, 60.0);
    (w, w * 1.4, (w * 0.4).clamp(2.6, 18.0))
}

/// Where the source stands and where the listener sits (x across, to the
/// listener's right; y from the front wall towards the back, in metres).
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
    (0.161 * v / (s * absorption(p))).clamp(0.15, 9.0)
}

fn absorption(p: &[f32]) -> f32 {
    0.06 + 0.8 * p[WALLS].clamp(0.0, 1.0)
}

/// Which way `facing` (degrees: 0 towards the back of the room, where the
/// listener is; 90 to the listener's right) points, as a vector (x, y).
pub fn facing_vector(facing: f32) -> (f32, f32) {
    let a = facing.to_radians();
    (a.sin(), a.cos())
}

/// The facing (degrees) from `from` towards `to` (both x, y in metres).
pub fn facing_towards(from: (f32, f32), to: (f32, f32)) -> f32 {
    (to.0 - from.0).atan2(to.1 - from.1).to_degrees()
}

/// Where the source is heard from: -1 all the way left to 1 all the way right.
pub fn direction(p: &[f32]) -> f32 {
    let ((sx, sy), (lx, ly)) = places(p);
    let az = (sx - lx).atan2(-(sy - ly));
    pan_of(az.sin().clamp(-1.0, 1.0).asin())
}

/// Across (-1 to 1) for a source on the line where it keeps its level, to
/// be heard from `pan` (-1 left to 1 right), as near as the room allows.
pub fn across_for(p: &[f32], pan: f32) -> f32 {
    let (w, d, _) = room(p);
    let ((_, _), (_, ly)) = places(p);
    let ahead = (ly - (0.3 + 0.3 * (d - 0.6))).max(0.5);
    let x = ahead * (pan.clamp(-1.0, 1.0) * FULL_SIDE).to_radians().tan();
    (x / (w * 0.5 - 0.4)).clamp(-1.0, 1.0)
}

/// Where a lateral angle (radians, + right) pans: -1 left to 1 right.
fn pan_of(lateral: f32) -> f32 {
    (lateral.to_degrees() / FULL_SIDE).clamp(-1.0, 1.0)
}

/// Equal-power gains for a pan.
fn pan_gains(p: f32) -> (f32, f32) {
    let a = (p.clamp(-1.0, 1.0) + 1.0) * FRAC_PI_4;
    (a.cos(), a.sin())
}

/// What a tap does at both ears: gain, delay (s) and how far up its sound
/// reaches (Hz).
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
            lines: [DelayLine::new(len), DelayLine::new(len), DelayLine::new(len)],
            taps: [Tap::default(); TAPS],
            ready: false,
            fdn: std::array::from_fn(|_| DelayLine::new(fdn_len)),
            damp: [0.0; LINES],
            tail_level: 0.0,
            pre: -1.0,
        }
    }

    /// Forgets what it heard (and starts in its next place from nothing).
    pub fn clear(&mut self) {
        for l in self.lines.iter_mut().chain(self.fdn.iter_mut()) {
            l.clear();
        }
        self.damp = [0.0; LINES];
        self.ready = false;
        self.tail_level = 0.0;
        self.pre = -1.0;
        for t in self.taps.iter_mut() {
            t.z = [0.0; 2];
        }
    }

    /// Where every tap should be now, and the direct sound's distance.
    fn targets(p: &[f32]) -> ([Target; TAPS], f32) {
        let (w, d, h) = room(p);
        let ((sx, sy), (lx, ly)) = places(p);
        let walls = p[WALLS].clamp(0.0, 1.0);
        let falloff = p[FALLOFF].clamp(0.0, 1.0);
        let air = p[AIR].clamp(0.0, 1.0);
        let phones = p[EARS].round() >= 1.0;
        let reflect = (1.0 - absorption(p)).max(0.0).sqrt();
        let room_amount = p[ROOM_AMOUNT].clamp(0.0, 1.0);
        let beam = p[BEAM].clamp(0.0, 1.0);
        let face = facing_vector(p[FACING]);
        let (sz, lz) = (SOURCE_HEIGHT, EAR_HEIGHT);
        let reference = reference(p);
        let level = |r: f32| (reference / r.max(0.5)).powf(falloff);
        // How loud and how bright the sound leaves towards (ex, ey).
        let aim = |ex: f32, ey: f32| {
            let n = (ex * ex + ey * ey).sqrt().max(1e-6);
            let cos = (face.0 * ex + face.1 * ey) / n;
            let off = beam * (1.0 - cos) * 0.5;
            (1.0 - 0.85 * off, 1.0 - 0.8 * off)
        };
        let air_cut = |r: f32| 20000.0 / (1.0 + air * (r - 1.0).max(0.0) / 15.0);
        let mut out = [Target::default(); TAPS];

        // The direct sound: its middle's direction, its width kept.
        let (vx, vy, vz) = (sx - lx, sy - ly, sz - lz);
        let r = (vx * vx + vy * vy + vz * vz).sqrt().max(0.3);
        let az = vx.atan2(-vy);
        let lateral = az.sin().clamp(-1.0, 1.0).asin();
        let behind = az.cos() < 0.0;
        let centre = pan_of(lateral);
        let width = p[WIDTH].clamp(0.0, 1.0) * (reference / r).clamp(0.2, 1.0) * (1.0 - 0.5 * centre.abs());
        let (dir_gain, dir_cut) = aim(lx - sx, ly - sy);
        let g = level(r) * dir_gain;
        let mut cut = (air_cut(r) * dir_cut).max(600.0);
        if behind {
            cut *= 0.7;
        }
        let t0 = r / C;
        for (k, t) in out.iter_mut().take(2).enumerate() {
            let pan = (centre + if k == 0 { -width } else { width }).clamp(-1.0, 1.0);
            if phones {
                let lat = (pan * FULL_SIDE).to_radians();
                let itd = 0.0875 / C * (lat + lat.sin());
                let far = 1.0 - 0.45 * lat.sin().abs();
                let shadow = (cut * (1.0 - 0.8 * lat.sin().abs())).max(600.0);
                if lat >= 0.0 {
                    t.gain = [g * far, g];
                    t.delay = [t0 + itd.abs(), t0];
                    t.cutoff = [shadow, cut];
                } else {
                    t.gain = [g, g * far];
                    t.delay = [t0, t0 + itd.abs()];
                    t.cutoff = [cut, shadow];
                }
            } else {
                let (gl, gr) = pan_gains(pan);
                t.gain = [g * gl, g * gr];
                let itd = 0.00025 * (pan * FULL_SIDE).to_radians().sin();
                t.delay = [t0 + itd.max(0.0), t0 + (-itd).max(0.0)];
                t.cutoff = [cut, cut];
            }
        }

        // The reflections: (image x, y, z, order, which axes its walls turned).
        let firsts = [(-w - sx, sy, sz, (true, false)), (w - sx, sy, sz, (true, false)), (sx, -sy, sz, (false, true)), (sx, 2.0 * d - sy, sz, (false, true)), (sx, sy, -sz, (false, false)), (sx, sy, 2.0 * h - sz, (false, false))];
        let seconds = [(-w - sx, -sy), (w - sx, -sy), (-w - sx, 2.0 * d - sy), (w - sx, 2.0 * d - sy)];
        let mut images = [(0.0f32, 0.0f32, 0.0f32, 1u8, (false, false)); TAPS - 2];
        for (k, &(x, y, z, m)) in firsts.iter().enumerate() {
            images[k] = (x, y, z, 1, m);
        }
        for (k, &(x, y)) in seconds.iter().enumerate() {
            images[6 + k] = (x, y, sz, 2, (true, true));
        }
        for (t, &(x, y, z, order, (mx, my))) in out.iter_mut().skip(2).zip(images.iter()) {
            let (vx, vy, vz) = (x - lx, y - ly, z - lz);
            let r = (vx * vx + vy * vy + vz * vz).sqrt().max(0.3);
            let az = vx.atan2(-vy);
            let lateral = az.sin().clamp(-1.0, 1.0).asin();
            // The way it left the source: towards the wall (the image's way, turned back).
            let (ex, ey) = (if mx { vx } else { -vx }, if my { vy } else { -vy });
            let (ag, ac) = aim(ex, ey);
            let g = room_amount * level(r) * reflect.powi(order as i32) * 0.85 * ag;
            let wall_cut = 16000.0 * (1.0 - 0.75 * walls).powi(order as i32);
            let mut cut = (wall_cut.min(air_cut(r)) * ac).max(600.0);
            if az.cos() < 0.0 {
                cut *= 0.7;
            }
            let (gl, gr) = pan_gains(pan_of(lateral));
            let itd = if phones { 0.0875 / C * (lateral + lateral.sin()) } else { 0.00025 * lateral.sin() };
            let t0 = r / C;
            t.gain = [g * gl, g * gr];
            t.delay = [t0 + itd.max(0.0), t0 + (-itd).max(0.0)];
            t.cutoff = [cut, cut];
        }
        (out, r)
    }

    pub fn process(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32], ctx: &FxCtx) {
        let n = l.len().min(r.len());
        if p.len() < SPECS.len() {
            return;
        }
        let sr = ctx.sr.max(1.0);
        let (targets, direct) = Self::targets(p);
        // Without the delay the direct sound arrives at once (at its earlier
        // ear), all the rest as much later as their way is longer.
        let first = targets[..2].iter().flat_map(|t| t.delay).fold(f32::MAX, f32::min);
        let offset = if p[FLIGHT] >= 0.5 { 0.0 } else { first };
        let max = (self.lines[0].len() - 4) as f32;
        let coef = |hz: f32| 1.0 - (-TAU * hz.min(sr * 0.45) / sr).exp();
        let mut want = [Tap::default(); TAPS];
        for (wt, t) in want.iter_mut().zip(targets.iter()) {
            for e in 0..2 {
                wt.gain[e] = t.gain[e];
                wt.delay[e] = (((t.delay[e] - offset) * sr).max(0.0) + 1.0).min(max);
                wt.coef[e] = coef(t.cutoff[e]);
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
        // The tail: Sabine's time, as loud as the direct sound at the critical distance.
        let (w, d, h) = room(p);
        let t60 = rt60(p);
        let v = w * d * h;
        let critical = (0.057 * (v / t60).sqrt()).max(0.3);
        let falloff = p[FALLOFF].clamp(0.0, 1.0);
        let room_amount = p[ROOM_AMOUNT].clamp(0.0, 1.0);
        let tail_want = 2.0 * room_amount * p[TAIL].clamp(0.0, 1.0) * (reference(p) / critical.max(0.5)).powf(falloff);
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
        let pre = (((4.0 * v / s) / C + direct / C - offset).max(0.0) * sr + 1.0).min(max);
        if self.pre < 0.0 {
            self.pre = pre;
        }
        let level = 10f32.powf(p[LEVEL].clamp(-24.0, 24.0) / 20.0);
        let k_smooth = 1.0 - (-1.0 / (0.04 * sr)).exp();
        // With no room at all (the Map, as it starts) the reflections and the tail rest.
        let quiet = room_amount <= 0.0 && self.tail_level < 1e-5;
        let taps = if quiet { 2 } else { TAPS };
        for i in 0..n {
            let (xl, xr) = (l[i], r[i]);
            self.lines[0].write(xl);
            self.lines[1].write(xr);
            self.lines[2].write(0.5 * (xl + xr));
            let mut out = [0.0f32; 2];
            for (j, (tap, w)) in self.taps.iter_mut().zip(want.iter()).enumerate().take(taps) {
                let src = j.min(2);
                for e in 0..2 {
                    tap.gain[e] += k_smooth * (w.gain[e] - tap.gain[e]);
                    tap.delay[e] += k_smooth * (w.delay[e] - tap.delay[e]);
                    tap.coef[e] += k_smooth * (w.coef[e] - tap.coef[e]);
                    let x = self.lines[src].read(tap.delay[e]);
                    tap.z[e] += tap.coef[e] * (x - tap.z[e]);
                    out[e] += tap.z[e] * tap.gain[e];
                }
            }
            if !quiet {
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
                out[0] += tl * g;
                out[1] += tr * g;
            }
            l[i] = out[0] * level;
            r[i] = out[1] * level;
        }
    }
}

/// The Map's room, the same for every track on it. With `room` at 0 (as it
/// starts) there is no room at all: a track on the map is only placed.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct MapRoom {
    pub room: f32,
    pub size: f32,
    pub walls: f32,
    pub reverb: f32,
    pub listener: f32,
    pub falloff: f32,
    pub air: f32,
    pub delay: bool,
    pub headphones: bool,
}

impl Default for MapRoom {
    fn default() -> Self {
        Self { room: 0.0, size: 16.0, walls: 0.45, reverb: 0.35, listener: 0.6, falloff: 0.7, air: 0.5, delay: false, headphones: false }
    }
}

/// Where a track stands on the Map, how wide it is and which way it faces
/// (degrees: 0 towards the listener's side of the room, 90 to the right).
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Place {
    pub across: f32,
    pub depth: f32,
    pub width: f32,
    pub facing: f32,
    pub beam: f32,
}

impl Default for Place {
    fn default() -> Self {
        Self { across: 0.0, depth: 0.3, width: 1.0, facing: 0.0, beam: 0.4 }
    }
}

/// A Stage's parameters for a track on the Map.
pub fn map_params(room: &MapRoom, place: &Place) -> [f32; 15] {
    let mut p = [0.0f32; 15];
    for (v, s) in p.iter_mut().zip(SPECS) {
        *v = s.default;
    }
    p[ACROSS] = place.across;
    p[DEPTH] = place.depth;
    p[WIDTH] = place.width;
    p[FACING] = place.facing;
    p[BEAM] = place.beam;
    p[SIZE] = room.size;
    p[WALLS] = room.walls;
    p[TAIL] = room.reverb;
    p[LISTENER] = room.listener;
    p[FALLOFF] = room.falloff;
    p[AIR] = room.air;
    p[FLIGHT] = if room.delay { 1.0 } else { 0.0 };
    p[EARS] = if room.headphones { 1.0 } else { 0.0 };
    p[ROOM_AMOUNT] = room.room;
    p[LEVEL] = 0.0;
    p
}

#[cfg(test)]
mod tests {
    use super::super::test_util::*;
    use super::*;
    use crate::params::defaults;

    fn run(p: &[f32], l_in: &[f32], r_in: &[f32]) -> (Vec<f32>, Vec<f32>) {
        let mut s = Stage::new(48000.0);
        let mut l = l_in.to_vec();
        let mut r = r_in.to_vec();
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

    fn on_map(place: Place, room: f32) -> Vec<f32> {
        let room = MapRoom { room, ..MapRoom::default() };
        map_params(&room, &place).to_vec()
    }

    #[test]
    fn on_the_map_in_the_middle_of_the_stage_it_sounds_as_it_did() {
        // No room, the place it starts at: the left stays left, the right right, the level kept.
        let p = on_map(Place::default(), 0.0);
        let left = tone(330.0, 0.5, 0.3);
        let right = tone(550.0, 0.5, 0.3);
        let (l, r) = run(&p, &left, &right);
        let skip = 4800;
        let db = |a: &[f32], b: &[f32]| 20.0 * (rms(&a[skip..]) / rms(&b[skip..])).log10();
        assert!(db(&l, &left).abs() < 1.0, "{}", db(&l, &left));
        assert!(db(&r, &right).abs() < 1.0, "{}", db(&r, &right));
        // None of the right in the left.
        let at = |v: &[f32], f: f32| {
            let w = f * TAU / 48000.0;
            let (s, c) = v.iter().enumerate().fold((0.0f32, 0.0f32), |(s, c), (i, x)| (s + x * (w * i as f32).sin(), c + x * (w * i as f32).cos()));
            (s * s + c * c).sqrt()
        };
        assert!(at(&l[skip..], 550.0) < 0.02 * at(&l[skip..], 330.0), "{} {}", at(&l[skip..], 550.0), at(&l[skip..], 330.0));
    }

    #[test]
    fn to_one_side_is_all_the_way_there() {
        let p = on_map(Place { across: 0.6, width: 0.0, ..Place::default() }, 0.0);
        let (l, r) = run(&p, &click(4800), &click(4800));
        assert!(energy(&r) > 8.0 * energy(&l), "{} {}", energy(&r), energy(&l));
        let p = on_map(Place { across: -1.0, width: 0.0, ..Place::default() }, 0.0);
        let (l, r) = run(&p, &click(4800), &click(4800));
        assert!(energy(&l) > 30.0 * energy(&r), "{} {}", energy(&l), energy(&r));
        // On headphones the near ear first.
        let mut p = on_map(Place { across: 0.8, width: 0.0, ..Place::default() }, 0.0);
        p[EARS] = 1.0;
        let (l, r) = run(&p, &click(4800), &click(4800));
        let first = |v: &[f32]| v.iter().position(|x| x.abs() > 0.02).unwrap();
        assert!(first(&r) < first(&l), "{} {}", first(&r), first(&l));
    }

    #[test]
    fn turned_away_it_is_quieter_duller_and_roomier() {
        let toward = on_map(Place::default(), 1.0);
        let away = on_map(Place { facing: 180.0, beam: 1.0, ..Place::default() }, 1.0);
        let (tl, _) = run(&toward, &click(48000), &click(48000));
        let (al, _) = run(&away, &click(48000), &click(48000));
        let direct = |v: &[f32]| energy(&v[..300]);
        let room = |v: &[f32]| energy(&v[2400..]);
        assert!(direct(&tl) > 3.0 * direct(&al), "{} {}", direct(&tl), direct(&al));
        assert!(direct(&tl) / room(&tl) > 2.0 * direct(&al) / room(&al));
    }

    #[test]
    fn further_away_is_quieter_and_with_a_room_more_room() {
        let near = on_map(Place { depth: 0.55, ..Place::default() }, 1.0);
        let far = on_map(Place { depth: 0.0, ..Place::default() }, 1.0);
        assert!(distance(&far) > 2.0 * distance(&near), "{} {}", distance(&far), distance(&near));
        let (nl, _) = run(&near, &click(48000), &click(48000));
        let (fl, _) = run(&far, &click(48000), &click(48000));
        let direct = |v: &[f32]| energy(&v[..300]);
        let room = |v: &[f32]| energy(&v[2400..]);
        assert!(direct(&nl) > 2.0 * direct(&fl));
        assert!(direct(&nl) / room(&nl) > 2.0 * direct(&fl) / room(&fl), "{} {}", direct(&nl) / room(&nl), direct(&fl) / room(&fl));
        // Without a room: no tail at all.
        let dry = on_map(Place { depth: 0.0, ..Place::default() }, 0.0);
        let (dl, _) = run(&dry, &click(48000), &click(48000));
        assert!(room(&dl) < 1e-9, "{}", room(&dl));
    }

    #[test]
    fn with_the_delay_a_far_sound_arrives_late() {
        let mut p = defaults(SPECS);
        p[FLIGHT] = 1.0;
        p[DEPTH] = 0.0;
        p[SIZE] = 30.0;
        p[WIDTH] = 0.0;
        let (l, _) = run(&p, &click(9600), &click(9600));
        let first = l.iter().position(|x| x.abs() > 1e-3).unwrap();
        let expect = 100.0 + distance(&p) / C * 48000.0;
        assert!((first as f32 - expect).abs() < 30.0, "{first} {expect}");
        p[FLIGHT] = 0.0;
        let (l, _) = run(&p, &click(9600), &click(9600));
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
        let (bl, _) = run(&big, &click(96000), &click(96000));
        let (sl, _) = run(&small, &click(96000), &click(96000));
        assert!(energy(&bl[48000..]) > 20.0 * energy(&sl[48000..]));
        assert!(bl.iter().chain(sl.iter()).all(|x| x.is_finite() && x.abs() < 4.0));
    }

    #[test]
    fn moving_and_turning_glide_without_clicks() {
        let mut s = Stage::new(48000.0);
        let mut p = defaults(SPECS);
        let tone = tone(220.0, 1.0, 0.5);
        let mut out = Vec::new();
        for (b, chunk) in tone.chunks(256).enumerate() {
            if b == 60 {
                p[ACROSS] = 1.0;
                p[DEPTH] = 0.9;
                p[FACING] = 120.0;
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

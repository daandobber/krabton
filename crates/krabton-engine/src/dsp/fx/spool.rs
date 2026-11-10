//! Spool: one sound on a tape and four heads playing it, each looping a
//! stretch of its own. Every head has its window (where, how long), its
//! speed, forwards or backwards, round or back and forth (ping-pong), its
//! own pitch, level and pan; together they layer a single sample into
//! drones, ambient beds, tape-style gestures and glitches that stay loose
//! and off the grid.
//!
//! The heads move as tape does: when they start and stop (with the song,
//! by themselves or with the notes) they wind up and down in pitch, as
//! long as *Inertia* says, and played backwards they sound as a tape turned
//! over. *Stretch* plays them as grains instead, so speed and pitch part:
//! a head slowed to nearly nothing freezes into a drone at its pitch,
//! *Grain* sets how long each grain is and *Spray* how far they scatter.
//! *Fade In* and *Fade Out* shape every pass of a window, from a click-free
//! loop to swells.
//!
//! The sound is on the same tape as the Looper's ([`LoopMemory`], made by
//! the UI, saved next to the set): dropped in, or captured from what comes
//! into the device (*Capture* on records, off keeps it).

use crate::dsp::db_to_gain;
use crate::dsp::effects::FxCtx;
use crate::dsp::fx::looper::{LoopMemory, LoopState};
use crate::params::{ParamSpec, Unit};
use std::sync::atomic::Ordering;
use std::sync::Arc;

const RELAXED: Ordering = Ordering::Relaxed;

pub const CAPTURE: usize = 0;
pub const PLAYBACK: usize = 1;
pub const TRANSPOSE: usize = 2;
pub const TONE: usize = 3;
pub const FADE_IN: usize = 4;
pub const FADE_OUT: usize = 5;
pub const SPRAY: usize = 6;
pub const GRAIN: usize = 7;
pub const STRETCH: usize = 8;
pub const INERTIA: usize = 9;
pub const INPUT: usize = 10;
pub const LEVEL: usize = 11;
/// Where the heads' settings start, and how many each has.
pub const HEAD0: usize = 12;
pub const HEAD_P: usize = 9;
pub const HEADS: usize = 4;
// A head's settings, from its first.
pub const H_ON: usize = 0;
pub const H_START: usize = 1;
pub const H_LENGTH: usize = 2;
pub const H_SPEED: usize = 3;
pub const H_REVERSE: usize = 4;
pub const H_PINGPONG: usize = 5;
pub const H_LEVEL: usize = 6;
pub const H_PAN: usize = 7;
pub const H_PITCH: usize = 8;

/// Head `h`'s setting `k`.
pub const fn head(h: usize, k: usize) -> usize {
    HEAD0 + h * HEAD_P + k
}

pub const PLAYBACKS: &[&str] = &["Transport", "Endless", "MIDI"];
pub const TRANSPORT: usize = 0;
pub const ENDLESS: usize = 1;
pub const MIDI: usize = 2;

macro_rules! head_specs {
    ($g:literal, $on:literal, $start:literal, $len:literal, $speed:literal, $rev:literal, $pp:literal, $lvl:literal, $pan:literal, $pitch:literal, $s:expr, $l:expr, $r:expr, $p:expr) => {
        [
            ParamSpec::toggle($on, $g, true),
            ParamSpec::lin($start, $g, 0.0, 1.0, $s, Unit::Percent),
            ParamSpec::log($len, $g, 0.002, 1.0, $l, Unit::Percent),
            ParamSpec::log($speed, $g, 0.05, 4.0, 1.0, Unit::Ratio),
            ParamSpec::toggle($rev, $g, $r),
            ParamSpec::toggle($pp, $g, false),
            ParamSpec::lin($lvl, $g, 0.0, 1.0, 0.7, Unit::Percent),
            ParamSpec::lin($pan, $g, -1.0, 1.0, $p, Unit::Pan),
            ParamSpec::int($pitch, $g, -24.0, 24.0, 0.0, Unit::Semi),
        ]
    };
}

const H1: [ParamSpec; HEAD_P] = head_specs!("Head 1", "1 On", "1 Start", "1 Length", "1 Speed", "1 Reverse", "1 Ping-Pong", "1 Level", "1 Pan", "1 Pitch", 0.05, 0.12, false, -0.4);
const H2: [ParamSpec; HEAD_P] = head_specs!("Head 2", "2 On", "2 Start", "2 Length", "2 Speed", "2 Reverse", "2 Ping-Pong", "2 Level", "2 Pan", "2 Pitch", 0.3, 0.08, false, 0.4);
const H3: [ParamSpec; HEAD_P] = head_specs!("Head 3", "3 On", "3 Start", "3 Length", "3 Speed", "3 Reverse", "3 Ping-Pong", "3 Level", "3 Pan", "3 Pitch", 0.5, 0.16, true, -0.15);
const H4: [ParamSpec; HEAD_P] = head_specs!("Head 4", "4 On", "4 Start", "4 Length", "4 Speed", "4 Reverse", "4 Ping-Pong", "4 Level", "4 Pan", "4 Pitch", 0.75, 0.2, false, 0.15);

const fn all_specs() -> [ParamSpec; HEAD0 + HEADS * HEAD_P] {
    let global = [
        ParamSpec::toggle("Capture", "Tape", false),
        ParamSpec::choice("Playback", "Tape", PLAYBACKS, ENDLESS),
        ParamSpec::int("Transpose", "Tape", -24.0, 24.0, 0.0, Unit::Semi),
        ParamSpec::lin("Tone", "Shape", -1.0, 1.0, 0.0, Unit::Percent),
        ParamSpec::lin("Fade In", "Shape", 0.0, 0.5, 0.05, Unit::Percent),
        ParamSpec::lin("Fade Out", "Shape", 0.0, 0.5, 0.05, Unit::Percent),
        ParamSpec::lin("Spray", "Grains", 0.0, 1.0, 0.0, Unit::Percent),
        ParamSpec::log("Grain", "Grains", 10.0, 500.0, 120.0, Unit::Ms),
        ParamSpec::toggle("Stretch", "Grains", false),
        ParamSpec::lin("Inertia", "Tape", 0.0, 1.0, 0.35, Unit::Percent),
        ParamSpec::lin("Input", "Mix", 0.0, 1.0, 1.0, Unit::Percent),
        ParamSpec::lin("Level", "Mix", -36.0, 6.0, 0.0, Unit::Db),
    ];
    let mut out = [ParamSpec::toggle("", "", false); HEAD0 + HEADS * HEAD_P];
    let mut i = 0;
    while i < HEAD0 {
        out[i] = global[i];
        i += 1;
    }
    let mut k = 0;
    while k < HEAD_P {
        out[HEAD0 + k] = H1[k];
        out[HEAD0 + HEAD_P + k] = H2[k];
        out[HEAD0 + 2 * HEAD_P + k] = H3[k];
        out[HEAD0 + 3 * HEAD_P + k] = H4[k];
        k += 1;
    }
    out
}

static ALL: [ParamSpec; HEAD0 + HEADS * HEAD_P] = all_specs();
pub static SPECS: &[ParamSpec] = &ALL;

/// How many grains a head plays at once (each half over the next).
const GRAINS: usize = 3;

#[derive(Clone, Copy, Default)]
struct Grain {
    on: bool,
    pos: f64,
    age: f32,
    len: f32,
    rate: f64,
}

#[derive(Clone, Copy)]
struct Head {
    /// Where it is on the tape (frames).
    pos: f64,
    /// Back and forth: +1 or -1.
    dir: f64,
    /// How fast the tape runs under it now, 0 (still) to 1 (as set).
    run: f32,
    grains: [Grain; GRAINS],
    next_grain: f32,
    /// Its window last time, to place it again when that moves.
    window: (f64, f64),
    /// How loud it was in the last block (for the face).
    peak: f32,
}

impl Default for Head {
    fn default() -> Self {
        Self { pos: 0.0, dir: 1.0, run: 0.0, grains: [Grain::default(); GRAINS], next_grain: 0.0, window: (0.0, 0.0), peak: 0.0 }
    }
}

/// Spool on the audio thread.
pub struct Spool {
    mem: Option<Arc<LoopMemory>>,
    /// A tape replaced, waiting to be handed away (never dropped here).
    retired: Option<Arc<LoopMemory>>,
    heads: [Head; HEADS],
    /// Capturing: frames written so far (None: not capturing).
    rec: Option<usize>,
    /// Whether the heads were still (to start them from their windows' starts).
    still: bool,
    note: Option<u8>,
    tone: [f32; 2],
    dice: u32,
}

impl Default for Spool {
    fn default() -> Self {
        Self::new()
    }
}

#[inline]
fn hermite(y0: f32, y1: f32, y2: f32, y3: f32, t: f32) -> f32 {
    let c1 = 0.5 * (y2 - y0);
    let c2 = y0 - 2.5 * y1 + 2.0 * y2 - 0.5 * y3;
    let c3 = 0.5 * (y3 - y0) + 1.5 * (y1 - y2);
    ((c3 * t + c2) * t + c1) * t + y1
}

/// The tape at `pos` (between its frames), inside `0..len`.
#[inline]
fn read(m: &LoopMemory, len: usize, pos: f64) -> (f32, f32) {
    let i = pos.floor();
    let t = (pos - i) as f32;
    let i = i as isize;
    let at = |k: isize| m.read(k.clamp(0, len as isize - 1) as usize);
    let (a, b, c, d) = (at(i - 1), at(i), at(i + 1), at(i + 2));
    (hermite(a.0, b.0, c.0, d.0, t), hermite(a.1, b.1, c.1, d.1, t))
}

/// `pos` brought back into the window `a..b` (round, or back and forth).
#[inline]
fn wrap(pos: f64, dir: &mut f64, a: f64, b: f64, pingpong: bool) -> f64 {
    let w = (b - a).max(1.0);
    if pos >= a && pos < b {
        return pos;
    }
    if pingpong {
        let mut p = pos;
        for _ in 0..4 {
            if p >= b {
                p = 2.0 * b - p - 1e-6;
                *dir = -*dir;
            } else if p < a {
                p = 2.0 * a - p;
                *dir = -*dir;
            } else {
                break;
            }
        }
        p.clamp(a, b - 1e-6)
    } else {
        a + (pos - a).rem_euclid(w)
    }
}

/// How loud a pass of a window is at `phase` (0 its start, 1 its end, as it plays).
#[inline]
fn fade(phase: f64, fade_in: f32, fade_out: f32) -> f32 {
    let x = phase.clamp(0.0, 1.0) as f32;
    let a = if fade_in > 1e-4 { (x / fade_in).min(1.0) } else { 1.0 };
    let b = if fade_out > 1e-4 { ((1.0 - x) / fade_out).min(1.0) } else { 1.0 };
    let s = |v: f32| v * v * (3.0 - 2.0 * v);
    s(a) * s(b)
}

impl Spool {
    pub fn new() -> Self {
        Self { mem: None, retired: None, heads: [Head::default(); HEADS], rec: None, still: true, note: None, tone: [0.0; 2], dice: 0x2545_f491 }
    }

    /// The tape it plays, as its device carries it.
    pub fn attach(&mut self, mem: Option<&Arc<LoopMemory>>) {
        let same = match (&self.mem, mem) {
            (Some(a), Some(b)) => Arc::ptr_eq(a, b),
            (None, None) => true,
            _ => false,
        };
        if !same {
            if let Some(old) = std::mem::replace(&mut self.mem, mem.cloned()) {
                self.retired = Some(old);
            }
            self.rec = None;
            self.still = true;
        }
    }

    pub fn take_retired(&mut self) -> Option<Arc<LoopMemory>> {
        self.retired.take()
    }

    /// A head's window on a tape of `len` frames: from, to.
    fn window(p: &[f32], h: usize, len: usize) -> (f64, f64) {
        let len = len as f64;
        let w = (p[head(h, H_LENGTH)].clamp(0.0, 1.0) as f64 * len).clamp(64.0f64.min(len), len);
        let a = (p[head(h, H_START)].clamp(0.0, 1.0) as f64 * len).min(len - w).max(0.0);
        (a, a + w)
    }

    /// Every head back to where its window starts (its end, played backwards).
    fn restart(&mut self, p: &[f32], len: usize) {
        for (h, hd) in self.heads.iter_mut().enumerate() {
            let (a, b) = Self::window(p, h, len);
            hd.dir = 1.0;
            hd.pos = if p[head(h, H_REVERSE)] >= 0.5 { b - 1.0 } else { a };
            hd.window = (a, b);
            hd.grains = [Grain::default(); GRAINS];
            hd.next_grain = 0.0;
        }
    }

    pub fn process(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32], ctx: &FxCtx) {
        let n = l.len().min(r.len());
        let Some(m) = self.mem.clone() else { return };
        if p.len() < SPECS.len() {
            return;
        }
        // Capturing what comes in.
        let capture = p[CAPTURE] >= 0.5;
        match (capture, self.rec) {
            (true, None) => {
                self.rec = Some(0);
                m.state.store(LoopState::Recording as u8, RELAXED);
                m.len.store(0, RELAXED);
                m.loop_bpm.store(0.0);
            }
            (false, Some(done)) => {
                self.rec = None;
                m.len.store(done, RELAXED);
                m.state.store(if done > 0 { LoopState::Stopped } else { LoopState::Empty } as u8, RELAXED);
                m.layers.store((done > 0) as u32, RELAXED);
                m.generation.fetch_add(1, RELAXED);
                self.still = true;
                self.restart(p, done.max(1));
            }
            _ => {}
        }
        let dry = p[INPUT].clamp(0.0, 1.0);
        if let Some(at) = self.rec {
            let cap = m.capacity();
            let k = n.min(cap - at.min(cap));
            for i in 0..k {
                m.put(at + i, l[i], r[i]);
            }
            let now = at + k;
            self.rec = Some(now);
            m.len.store(now, RELAXED);
            for i in 0..n {
                l[i] *= dry;
                r[i] *= dry;
            }
            for (k, hd) in self.heads.iter_mut().enumerate() {
                hd.peak = 0.0;
                m.heads[k].store(-1.0);
            }
            return;
        }
        let len = m.len.load(RELAXED).min(m.capacity());
        if len < 64 || m.state() == LoopState::Recording {
            for i in 0..n {
                l[i] *= dry;
                r[i] *= dry;
            }
            for a in &m.heads {
                a.store(-1.0);
            }
            return;
        }
        // How the heads run: with the song, always, or while a note is held.
        let mode = (p[PLAYBACK].round().max(0.0) as usize).min(PLAYBACKS.len() - 1);
        let target = match mode {
            TRANSPORT => ctx.playing,
            MIDI => ctx.note.is_some(),
            _ => true,
        };
        // A new note: the heads from their starts (not when one is only let go of).
        if mode == MIDI && ctx.note.is_some() && ctx.note != self.note {
            self.restart(p, len);
        }
        self.note = ctx.note;
        if target && self.still {
            self.restart(p, len);
            self.still = false;
        }
        let inertia = p[INERTIA].clamp(0.0, 1.0);
        let ramp = 1.0 / ((0.01 + 3.0 * inertia * inertia) * ctx.sr.max(1.0));
        let key = p[TRANSPOSE] + if mode == MIDI { ctx.note.map_or(0.0, |k| k as f32 - 60.0) } else { 0.0 };
        let stretch = p[STRETCH] >= 0.5;
        let grain_len = (p[GRAIN].clamp(10.0, 500.0) * 0.001 * ctx.sr).max(32.0);
        let spray = p[SPRAY].clamp(0.0, 1.0);
        let (fi, fo) = (p[FADE_IN].clamp(0.0, 0.5), p[FADE_OUT].clamp(0.0, 0.5));
        let rate_fix = m.rate() as f64 / ctx.sr.max(1.0) as f64;
        // The tone: darker one way, thinner the other.
        let tone = p[TONE].clamp(-1.0, 1.0);
        let fc = if tone < 0.0 { 20000.0 * (200.0f32 / 20000.0).powf(-tone) } else { 20.0 * (4000.0f32 / 20.0).powf(tone) };
        let g = 1.0 - (-std::f32::consts::TAU * fc.min(ctx.sr * 0.45) / ctx.sr.max(1.0)).exp();
        let level = db_to_gain(p[LEVEL]);
        let mut all_still = true;
        for i in 0..n {
            let (mut ol, mut or) = (0.0f32, 0.0f32);
            for h in 0..HEADS {
                let mut hd = self.heads[h];
                // The tape winds up and down.
                let want = if target { 1.0 } else { 0.0 };
                hd.run = if hd.run < want { (hd.run + ramp).min(want) } else { (hd.run - ramp).max(want) };
                let on = p[head(h, H_ON)] >= 0.5;
                if !on || hd.run <= 0.0 {
                    if i == n - 1 {
                        hd.peak *= 0.5;
                    }
                    self.heads[h] = hd;
                    continue;
                }
                all_still = false;
                let (a, b) = Self::window(p, h, len);
                if (a, b) != hd.window {
                    hd.window = (a, b);
                }
                let pp = p[head(h, H_PINGPONG)] >= 0.5;
                hd.pos = wrap(hd.pos, &mut hd.dir, a, b, pp);
                let back = if p[head(h, H_REVERSE)] >= 0.5 { -1.0 } else { 1.0 };
                let sign = back * hd.dir;
                let speed = p[head(h, H_SPEED)].clamp(0.05, 4.0) as f64 * rate_fix;
                let pitch = 2f64.powf((key + p[head(h, H_PITCH)]) as f64 / 12.0);
                let w = b - a;
                let phase = |pos: f64| if sign > 0.0 { (pos - a) / w } else { (b - pos) / w };
                let (sl, sr) = if stretch {
                    // Grains at its pitch, the head moving through at its speed.
                    hd.next_grain -= 1.0;
                    if hd.next_grain <= 0.0 {
                        hd.next_grain = grain_len * 0.5;
                        if let Some(gr) = hd.grains.iter_mut().find(|g| !g.on) {
                            let off = (self_random(&mut self.dice) as f64 - 0.5) * spray as f64 * w;
                            let mut d = 1.0;
                            *gr = Grain { on: true, pos: wrap(hd.pos + off, &mut d, a, b, false), age: 0.0, len: grain_len, rate: pitch * sign * rate_fix };
                        }
                    }
                    let (mut gl, mut grr) = (0.0, 0.0);
                    let tape = hd.run as f64;
                    for gr in hd.grains.iter_mut().filter(|g| g.on) {
                        let x = gr.age / gr.len;
                        let win = 0.5 - 0.5 * (std::f32::consts::TAU * x).cos();
                        let mut d = 1.0;
                        gr.pos = wrap(gr.pos, &mut d, a, b, false);
                        let (vl, vr) = read(&m, len, gr.pos);
                        gl += vl * win;
                        grr += vr * win;
                        gr.pos += gr.rate * tape;
                        gr.age += 1.0;
                        if gr.age >= gr.len {
                            gr.on = false;
                        }
                    }
                    hd.pos += speed * sign * tape;
                    (gl, grr)
                } else {
                    let v = read(&m, len, hd.pos);
                    hd.pos += speed * pitch * sign * hd.run as f64;
                    v
                };
                let shape = fade(phase(hd.pos), fi, fo) * hd.run.min(1.0).sqrt();
                let lvl = p[head(h, H_LEVEL)].clamp(0.0, 1.0) * shape;
                let pan = p[head(h, H_PAN)].clamp(-1.0, 1.0);
                let ang = (pan + 1.0) * std::f32::consts::FRAC_PI_4;
                let (pl, pr) = (ang.cos() * std::f32::consts::SQRT_2, ang.sin() * std::f32::consts::SQRT_2);
                let (hl, hr) = (sl * lvl * pl.min(1.0), sr * lvl * pr.min(1.0));
                hd.peak = hd.peak.max(hl.abs().max(hr.abs()));
                ol += hl;
                or += hr;
                self.heads[h] = hd;
            }
            // The tone, on the heads.
            self.tone[0] += g * (ol - self.tone[0]);
            self.tone[1] += g * (or - self.tone[1]);
            let (tl, tr) = if tone < 0.0 { (self.tone[0], self.tone[1]) } else if tone > 0.0 { (ol - self.tone[0], or - self.tone[1]) } else { (ol, or) };
            l[i] = l[i] * dry + tl * level;
            r[i] = r[i] * dry + tr * level;
        }
        if all_still && !target {
            self.still = true;
        }
        // Where the heads are, for the face.
        for (k, hd) in self.heads.iter_mut().enumerate() {
            let on = p[head(k, H_ON)] >= 0.5 && hd.run > 0.0;
            m.heads[k].store(if on { (hd.pos / len as f64) as f32 } else { -1.0 });
            m.head_peaks[k].store(hd.peak);
            hd.peak *= 0.8;
        }
        m.pos.store(self.heads[0].pos);
    }

    /// Where head `k` is now (frames), for tests.
    pub fn head_pos(&self, k: usize) -> f64 {
        self.heads[k].pos
    }
}

fn self_random(d: &mut u32) -> f32 {
    let mut x = *d;
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    *d = x;
    (x >> 8) as f32 / (1u32 << 24) as f32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::defaults;

    const SR: f32 = 48000.0;

    fn ctx(playing: bool, note: Option<u8>) -> FxCtx {
        FxCtx { sr: SR, bpm: 120.0, beat: 0.0, playing, scale: Default::default(), note }
    }

    fn tape(seconds: f32) -> Arc<LoopMemory> {
        let m = LoopMemory::new(SR, 10.0);
        let n = (seconds * SR) as usize;
        let l: Vec<f32> = (0..n).map(|i| (i as f32 * 220.0 * std::f32::consts::TAU / SR).sin() * 0.5).collect();
        m.load(&l, &l, 0.0);
        Arc::new(m)
    }

    fn run(s: &mut Spool, p: &[f32], blocks: usize, c: &FxCtx) -> (Vec<f32>, Vec<f32>) {
        let (mut ol, mut or) = (Vec::new(), Vec::new());
        for _ in 0..blocks {
            let mut l = [0.0f32; 256];
            let mut r = [0.0f32; 256];
            s.process(p, &mut l, &mut r, c);
            ol.extend_from_slice(&l);
            or.extend_from_slice(&r);
        }
        (ol, or)
    }

    fn rms(v: &[f32]) -> f32 {
        (v.iter().map(|x| x * x).sum::<f32>() / v.len().max(1) as f32).sqrt()
    }

    #[test]
    fn four_heads_loop_their_own_windows() {
        let m = tape(2.0);
        let mut s = Spool::new();
        s.attach(Some(&m));
        let mut p = defaults(SPECS);
        p[INPUT] = 0.0;
        let (l, _) = run(&mut s, &p, 400, &ctx(false, None));
        assert!(rms(&l[l.len() / 2..]) > 0.05, "it plays by itself (Endless)");
        let len = 2.0 * SR as f64;
        for h in 0..HEADS {
            let (a, b) = Spool::window(&p, h, len as usize);
            let at = s.head_pos(h);
            assert!(at >= a - 1.0 && at <= b + 1.0, "head {h} at {at} inside {a}..{b}");
        }
        assert!(l.iter().all(|v| v.is_finite() && v.abs() < 4.0));
    }

    #[test]
    fn with_the_transport_it_winds_down_and_up_as_tape() {
        let m = tape(1.0);
        let mut s = Spool::new();
        s.attach(Some(&m));
        let mut p = defaults(SPECS);
        p[INPUT] = 0.0;
        p[PLAYBACK] = TRANSPORT as f32;
        let (still, _) = run(&mut s, &p, 40, &ctx(false, None));
        assert!(rms(&still) < 1e-6, "silent while the song is stopped");
        let (playing, _) = run(&mut s, &p, 400, &ctx(true, None));
        assert!(rms(&playing[playing.len() / 2..]) > 0.05);
        // Stopped: it slows down for a while, then is silent.
        let (stopping, _) = run(&mut s, &p, 40, &ctx(false, None));
        assert!(rms(&stopping) > 0.001, "a tape stop, not a cut");
        let (after, _) = run(&mut s, &p, 800, &ctx(false, None));
        assert!(rms(&after[after.len() - 2048..]) < 1e-4);
    }

    #[test]
    fn notes_play_it_in_midi_mode_and_transpose_it() {
        let m = tape(1.0);
        let mut s = Spool::new();
        s.attach(Some(&m));
        let mut p = defaults(SPECS);
        p[INPUT] = 0.0;
        p[PLAYBACK] = MIDI as f32;
        p[INERTIA] = 0.0;
        p[head(0, H_START)] = 0.0;
        p[head(0, H_LENGTH)] = 1.0;
        let (none, _) = run(&mut s, &p, 20, &ctx(true, None));
        assert!(rms(&none) < 1e-6);
        let before = s.head_pos(0);
        run(&mut s, &p, 20, &ctx(true, Some(72)));
        let up = s.head_pos(0) - before;
        let mut s2 = Spool::new();
        s2.attach(Some(&m));
        let b2 = s2.head_pos(0);
        run(&mut s2, &p, 20, &ctx(true, Some(60)));
        let plain = s2.head_pos(0) - b2;
        assert!(up.abs() > plain.abs() * 1.5, "an octave up runs twice as fast: {up} {plain}");
    }

    #[test]
    fn stretch_keeps_the_pitch_while_the_head_crawls() {
        let m = tape(2.0);
        let mut s = Spool::new();
        s.attach(Some(&m));
        let mut p = defaults(SPECS);
        p[INPUT] = 0.0;
        p[STRETCH] = 1.0;
        for h in 0..HEADS {
            p[head(h, H_SPEED)] = 0.05;
            p[head(h, H_ON)] = (h == 0) as u8 as f32;
        }
        let (l, _) = run(&mut s, &p, 600, &ctx(false, None));
        let tail = &l[l.len() / 2..];
        assert!(rms(tail) > 0.05, "a drone, not silence");
        // Still at 220 Hz: count the crossings.
        let crossings = tail.windows(2).filter(|w| w[0] < 0.0 && w[1] >= 0.0).count() as f32;
        let hz = crossings / (tail.len() as f32 / SR);
        assert!((hz - 220.0).abs() < 40.0, "{hz} Hz");
    }

    #[test]
    fn capture_records_what_comes_in() {
        let m = Arc::new(LoopMemory::new(SR, 10.0));
        let mut s = Spool::new();
        s.attach(Some(&m));
        let mut p = defaults(SPECS);
        p[CAPTURE] = 1.0;
        for _ in 0..200 {
            let mut l = [0.3f32; 256];
            let mut r = [0.3f32; 256];
            s.process(&p, &mut l, &mut r, &ctx(false, None));
        }
        p[CAPTURE] = 0.0;
        let mut l = [0.0f32; 256];
        let mut r = [0.0f32; 256];
        s.process(&p, &mut l, &mut r, &ctx(false, None));
        assert_eq!(m.len.load(RELAXED), 200 * 256);
        assert_eq!(m.state(), LoopState::Stopped);
        assert!((m.read(1000).0 - 0.3).abs() < 1e-6);
        assert!(!m.frames().0.is_empty(), "kept for the set");
    }
}

//! Open Air: puts a sound outside. Outside there are no walls around you,
//! so the sound is mostly the direct sound and what bounces back from the
//! few things there are: the ground (grass swallows it, water throws it
//! back), the trunks of a forest (many small reflections, scattered in time
//! and direction), the walls of a canyon (echoes going back and forth), the
//! far shore of a lake, or the close rock of a cave (dense and dark, the one
//! place that rings for long). Far away, a sound is softer, duller and
//! wetter.
//!
//! Around it, if you like, the place's own sounds: birds, a dawn chorus,
//! an evening, the night, the wind, rain, water; made by the Birds and
//! Nature instruments, and quieter while you play (Duck). Original code.

use crate::dsp::birds::{self, BirdsState};
use crate::dsp::effects::FxCtx;
use crate::dsp::fx::hall::{self, Hall};
use crate::dsp::nature::{self, NatureState};
use crate::dsp::{db_to_gain, pan_gains, OnePole, Rng};
use crate::params::{ParamSpec, Unit};
use crate::scope::{DeviceTaps, MeterTap, ScopeTap};

pub const PLACE: usize = 0;
pub const DISTANCE: usize = 1;
pub const SIZE: usize = 2;
pub const DECAY: usize = 3;
pub const WIDTH: usize = 4;
pub const MIX: usize = 5;
pub const AMBIENCE: usize = 6;
pub const AMBIENCE_LEVEL: usize = 7;
pub const DUCK: usize = 8;

pub const PLACES: &[&str] = &["Forest", "Meadow", "Canyon", "Lake", "Cave"];
pub const AMBIENCES: &[&str] = &["None", "Birds", "Dawn", "Evening", "Night", "Wind", "Rain", "Water"];

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::choice("Place", "Place", PLACES, 0),
    ParamSpec::lin("Distance", "Place", 0.0, 1.0, 0.25, Unit::Percent),
    ParamSpec::lin("Size", "Place", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Decay", "Place", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Width", "Place", 0.0, 1.0, 0.8, Unit::Percent),
    ParamSpec::lin("Mix", "Place", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::choice("Ambience", "Around", AMBIENCES, 0),
    ParamSpec::lin("Ambience Level", "Around", -36.0, 6.0, -6.0, Unit::Db),
    ParamSpec::lin("Duck", "Around", 0.0, 1.0, 0.5, Unit::Percent),
];

/// One reflection: how long after the direct sound (samples), how loud on
/// each side, and how dull (0 bright .. 2 dull).
#[derive(Clone, Copy)]
struct Tap {
    d: usize,
    gl: f32,
    gr: f32,
    group: usize,
}

const MAX_TAPS: usize = 96;
const BUF: usize = 1 << 17; // about 2.7 s at 48 kHz

/// The tail per place: level, decay (s) at Decay 0 and the more at 1,
/// room size, damping (Hz), pre-delay (ms).
struct Tail {
    level: f32,
    t60: (f32, f32),
    size: f32,
    damping: f32,
    predelay: f32,
    /// The groups' brightness (Hz): early, middle, late.
    tone: [f32; 3],
}

const TAILS: [Tail; 5] = [
    Tail { level: 0.32, t60: (0.7, 2.0), size: 0.5, damping: 5500.0, predelay: 15.0, tone: [11000.0, 6500.0, 3500.0] },
    Tail { level: 0.07, t60: (0.4, 0.9), size: 0.4, damping: 5000.0, predelay: 30.0, tone: [12000.0, 5000.0, 2500.0] },
    Tail { level: 0.22, t60: (1.2, 3.5), size: 0.8, damping: 4500.0, predelay: 70.0, tone: [9000.0, 5000.0, 3000.0] },
    Tail { level: 0.25, t60: (1.0, 2.8), size: 0.7, damping: 7000.0, predelay: 40.0, tone: [12000.0, 7000.0, 4000.0] },
    Tail { level: 0.6, t60: (2.0, 6.5), size: 0.75, damping: 3200.0, predelay: 8.0, tone: [7000.0, 4000.0, 2200.0] },
];

pub struct OpenAir {
    sr: f32,
    buf: [Vec<f32>; 2],
    pos: usize,
    refl: Vec<Tap>,
    /// The params the taps were made for.
    key: [f32; 4],
    groups: [[OnePole; 3]; 2],
    direct: [OnePole; 2],
    hall: Box<Hall>,
    hall_p: Vec<f32>,
    nature: Box<NatureState>,
    birds: Box<BirdsState>,
    nat_p: Vec<f32>,
    bird_p: Vec<f32>,
    amb_key: (usize, usize),
    env: f32,
    duck: f32,
    tl: Vec<f32>,
    tr: Vec<f32>,
    al: Vec<f32>,
    ar: Vec<f32>,
    /// For the screen: the input's level, the ambience's, the ducking.
    pub meter: [f32; 3],
    /// What the screen shows: the output, and (meters) the levels, the
    /// reflections and tail, the weather and the birds.
    pub taps: DeviceTaps,
    vis: [f32; crate::engine::INST_VALS],
    early_level: f32,
    tail_level: f32,
}

impl OpenAir {
    pub fn new(sr: f32) -> Self {
        let sr = sr.max(1000.0);
        Self {
            sr,
            buf: [vec![0.0; BUF], vec![0.0; BUF]],
            pos: 0,
            refl: Vec::with_capacity(MAX_TAPS),
            key: [f32::NAN; 4],
            groups: [[OnePole::default(); 3]; 2],
            direct: [OnePole::default(); 2],
            hall: Box::new(Hall::new(sr)),
            hall_p: crate::params::defaults(hall::SPECS),
            nature: Box::new(NatureState::new(sr)),
            birds: Box::new(BirdsState::new(sr)),
            nat_p: crate::params::defaults(nature::SPECS),
            bird_p: crate::params::defaults(birds::SPECS),
            amb_key: (usize::MAX, usize::MAX),
            env: 0.0,
            duck: 1.0,
            tl: vec![0.0; 512],
            tr: vec![0.0; 512],
            al: vec![0.0; 512],
            ar: vec![0.0; 512],
            meter: [0.0; 3],
            taps: DeviceTaps { pre: None, post: ScopeTap::default(), meters: Some(MeterTap::default()) },
            vis: [0.0; crate::engine::INST_VALS],
            early_level: 0.0,
            tail_level: 0.0,
        }
    }

    /// The reflections of a place, its size and the distance.
    fn make_taps(&mut self, place: usize, size: f32, distance: f32) {
        let key = [place as f32, (size * 50.0).round(), (distance * 50.0).round(), self.sr];
        if key == self.key {
            return;
        }
        self.key = key;
        let ms = |t: f32| ((t * 0.001 * self.sr) as usize).clamp(1, BUF - 2);
        let s = 0.5 + size;
        let mut rng = Rng::new(0x0FE7_A1A5 ^ place as u32);
        let mut taps = Vec::with_capacity(MAX_TAPS);
        let add = |taps: &mut Vec<Tap>, t: f32, g: f32, pan: f32, group: usize| {
            if taps.len() < MAX_TAPS {
                let (gl, gr) = pan_gains(pan);
                taps.push(Tap { d: ms(t), gl: g * gl, gr: g * gr, group });
            }
        };
        // The ground: further away, the bounce comes sooner after the sound.
        let ground = 1.2 + 6.0 * (1.0 - distance);
        let ground_gain = [0.3, 0.35, 0.3, 0.75, 0.0][place];
        if ground_gain > 0.0 {
            add(&mut taps, ground, ground_gain, 0.0, if place == 3 { 0 } else { 1 });
        }
        match place {
            // Forest: many trunks, reflections scattered in time and space.
            0 => {
                let n = 56;
                let span = 260.0 * s;
                for k in 0..n {
                    let u = (k as f32 + rng.unipolar()) / n as f32;
                    let t = 8.0 + span * u.powf(1.6);
                    let g = 0.55 * (-t / (110.0 * s)).exp() * (0.5 + 0.5 * rng.unipolar());
                    let group = if t < 60.0 { 0 } else if t < 150.0 { 1 } else { 2 };
                    add(&mut taps, t, g, rng.bipolar() * 0.9, group);
                }
            }
            // Meadow: open, only a treeline far off.
            1 => {
                for k in 0..3 {
                    let t = (140.0 + 120.0 * k as f32) * s + 20.0 * rng.unipolar();
                    add(&mut taps, t, 0.16 / (1.0 + k as f32), rng.bipolar() * 0.6, 2);
                }
            }
            // Canyon: echoes going back and forth between two walls, each a
            // little smeared and darker.
            2 => {
                let (a, b) = (180.0 * s, 260.0 * s);
                let mut echoes: Vec<(f32, f32, f32)> = Vec::new();
                for i in 0..5 {
                    for j in 0..5 {
                        if i + j == 0 || i + j > 5 {
                            continue;
                        }
                        let t = i as f32 * a + j as f32 * b;
                        let g = 0.55 * 0.62f32.powi(i + j - 1);
                        let pan = (j as f32 - i as f32) / (i + j) as f32 * 0.8;
                        echoes.push((t, g, pan));
                    }
                }
                echoes.sort_by(|x, y| x.0.total_cmp(&y.0));
                for (t, g, pan) in echoes {
                    if t > 2500.0 {
                        break;
                    }
                    let group = if t < 400.0 { 0 } else if t < 900.0 { 1 } else { 2 };
                    add(&mut taps, t, g * 0.6, pan, group);
                    for _ in 0..3 {
                        add(&mut taps, t + 2.0 + 10.0 * rng.unipolar(), g * 0.15, pan + 0.2 * rng.bipolar(), group.min(2));
                    }
                }
            }
            // Lake: the water throws it back, the far shore answers.
            3 => {
                let shore = 450.0 * s;
                for k in 0..5 {
                    add(&mut taps, shore + 25.0 * k as f32 + 15.0 * rng.unipolar(), 0.2 * (1.0 - 0.15 * k as f32), rng.bipolar() * 0.7, 2);
                }
                for k in 0..6 {
                    add(&mut taps, 20.0 + 30.0 * k as f32 * s + 10.0 * rng.unipolar(), 0.08, rng.bipolar(), 1);
                }
            }
            // Cave: close rock all around, dense.
            _ => {
                for k in 0..40 {
                    let t = 4.0 + 90.0 * s * (k as f32 + rng.unipolar()) / 40.0;
                    let g = 0.45 * (-t / (60.0 * s)).exp() * (0.6 + 0.4 * rng.unipolar());
                    add(&mut taps, t, g, rng.bipolar(), if t < 30.0 { 0 } else { 1 });
                }
            }
        }
        // The same amount of reflected sound for any size: per place how much
        // comes back (the ground included).
        let target = [0.3, 0.12, 0.35, 0.6, 0.45][place];
        let e: f32 = taps.iter().map(|t| 0.5 * (t.gl * t.gl + t.gr * t.gr)).sum();
        if e > 1e-9 {
            let k = (target / e).sqrt();
            for t in taps.iter_mut() {
                t.gl *= k;
                t.gr *= k;
            }
        }
        self.refl = taps;
    }

    /// The place's sounds for this ambience.
    fn make_ambience(&mut self, amb: usize, place: usize) {
        if self.amb_key == (amb, place) {
            return;
        }
        self.amb_key = (amb, place);
        let mut n = crate::params::defaults(nature::SPECS);
        for i in [nature::WIND, nature::WHISTLE, nature::LEAVES] {
            n[i] = 0.0;
        }
        let mut b = crate::params::defaults(birds::SPECS);
        for i in [birds::NUMBER_1, birds::NUMBER_2, birds::NUMBER_3] {
            b[i] = 0.0;
        }
        let birds_from = |name: &str, b: &mut Vec<f32>| {
            *b = birds::preset(name);
        };
        let forest = place == 0;
        match amb {
            1 => {
                birds_from(["Woodland", "Meadow", "Moorland", "Lake at Dusk", "Garden Morning"][place], &mut b);
                n[nature::WIND] = 0.12;
                n[nature::LEAVES] = if forest { 0.25 } else { 0.05 };
            }
            2 => {
                birds_from(if place == 3 { "Lake at Dusk" } else { "Dawn Chorus" }, &mut b);
                n[nature::WIND] = 0.08;
                n[nature::STREAM] = if place <= 1 { 0.2 } else { 0.0 };
            }
            3 => {
                birds_from(if forest { "Lone Blackbird" } else { "Summer Evening" }, &mut b);
                n[nature::CRICKETS] = 0.3;
                n[nature::WARMTH] = 0.6;
                n[nature::WIND] = 0.08;
            }
            4 => {
                n = nature::preset("Summer Night");
                b[birds::BIRD_1] = 11.0;
                b[birds::NUMBER_1] = 1.0;
                b[birds::BUSY] = 0.4;
            }
            5 => {
                n[nature::WIND] = 0.6;
                n[nature::GUSTS] = 0.7;
                n[nature::WHISTLE] = if place == 2 { 0.45 } else { 0.1 };
                n[nature::LEAVES] = if forest { 0.4 } else { 0.0 };
            }
            6 => {
                n[nature::RAIN] = 0.4;
                n[nature::RAIN_ON] = if place == 3 { 1.0 } else { 0.0 };
                n[nature::WIND] = 0.15;
                n[nature::THUNDER] = 0.1;
            }
            7 => {
                if place == 3 {
                    n[nature::SEA] = 0.25;
                    n[nature::WAVES] = 0.1;
                } else {
                    n[nature::STREAM] = 0.55;
                }
                n[nature::WIND] = 0.08;
            }
            _ => {}
        }
        // In a cave: drips, and the birds far off at the mouth.
        if place == 4 && amb > 0 {
            n[nature::RAIN] = n[nature::RAIN].max(0.08);
            n[nature::RAIN_ON] = 1.0;
            b[birds::DISTANCE] = 1.0;
        }
        n[nature::PLAYS] = 0.0;
        b[birds::SINGS] = 0.0;
        // The place's own room is added after; keep theirs small.
        n[nature::SPACE] = 0.1;
        b[birds::SPACE] = b[birds::SPACE].min(0.35);
        self.nat_p = n;
        self.bird_p = b;
    }

    pub fn process(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32], ctx: &FxCtx) {
        let n = l.len().min(r.len());
        if p.len() < SPECS.len() {
            return;
        }
        let sr = ctx.sr;
        let place = (p[PLACE].round().max(0.0) as usize).min(PLACES.len() - 1);
        let k = |i: usize| p[i].clamp(0.0, 1.0);
        let (distance, size, decay, width, mix) = (k(DISTANCE), k(SIZE), k(DECAY), k(WIDTH), k(MIX));
        let amb = (p[AMBIENCE].round().max(0.0) as usize).min(AMBIENCES.len() - 1);
        let amb_gain = db_to_gain(p[AMBIENCE_LEVEL]);
        let duck_amount = k(DUCK);
        self.make_taps(place, size, distance);
        let tail = &TAILS[place];
        for c in 0..2 {
            for g in 0..3 {
                self.groups[c][g].set(tail.tone[g] * (1.0 - 0.45 * distance), sr);
            }
            // Far away: the air takes the highs.
            self.direct[c].set(2500.0 + 17500.0 * (1.0 - distance).powi(2), sr);
        }
        let direct_gain = 1.0 / (1.0 + 2.5 * distance);
        let early_gain = 0.8 + 0.4 * distance;
        let hp = &mut self.hall_p;
        hp[hall::PREDELAY] = tail.predelay;
        hp[hall::SIZE] = (tail.size + 0.3 * (size - 0.5)).clamp(0.1, 1.0);
        hp[hall::DECAY] = tail.t60.0 + (tail.t60.1 - tail.t60.0) * decay;
        hp[hall::LOW_DECAY] = if place == 4 { 1.3 } else { 0.8 };
        hp[hall::DAMPING] = tail.damping * (1.0 - 0.35 * distance);
        hp[hall::DIFFUSION] = if place == 2 { 0.5 } else { 0.8 };
        hp[hall::MODULATION] = 0.2;
        hp[hall::FREEZE] = 0.0;
        hp[hall::WIDTH] = 1.0;
        hp[hall::MIX] = 1.0;
        if amb > 0 {
            self.make_ambience(amb, place);
        }
        let mut done = 0;
        while done < n {
            let len = 512.min(n - done);
            let (xl, xr) = (&mut l[done..done + len], &mut r[done..done + len]);
            // The input's loudness, for ducking the ambience.
            let mut peak = 0.0f32;
            for j in 0..len {
                peak = peak.max(xl[j].abs().max(xr[j].abs()));
            }
            let rel = (-(len as f32) / (0.6 * sr)).exp();
            self.env = if peak > self.env { peak } else { self.env * rel };
            for j in 0..len {
                // Direct sound, early reflections.
                let (dl, dr) = (xl[j], xr[j]);
                let mono = 0.5 * (dl + dr);
                self.pos = (self.pos + 1) & (BUF - 1);
                self.buf[0][self.pos] = dl;
                self.buf[1][self.pos] = dr;
                let mut g = [[0.0f32; 3]; 2];
                for t in &self.refl {
                    let at = (self.pos + BUF - t.d) & (BUF - 1);
                    let v = 0.5 * (self.buf[0][at] + self.buf[1][at]);
                    g[0][t.group] += v * t.gl;
                    g[1][t.group] += v * t.gr;
                }
                let mut early = [0.0f32; 2];
                for c in 0..2 {
                    for k in 0..3 {
                        early[c] += self.groups[c][k].lp(g[c][k]);
                    }
                }
                let direct = [self.direct[0].lp(dl) * direct_gain, self.direct[1].lp(dr) * direct_gain];
                self.tl[j] = (early[0] * 0.6 + mono * 0.25 * (0.4 + distance)) * tail.level;
                self.tr[j] = (early[1] * 0.6 + mono * 0.25 * (0.4 + distance)) * tail.level;
                // Width on the direct and early sound.
                let (ol, or) = (direct[0] + early[0] * early_gain, direct[1] + early[1] * early_gain);
                self.early_level = (self.early_level * 0.9995).max((early[0] + early[1]).abs() * 0.5);
                let (m, s) = (0.5 * (ol + or), 0.5 * (ol - or) * width);
                self.al[j] = m + s;
                self.ar[j] = m - s;
            }
            let ctx_h = FxCtx { sr, bpm: ctx.bpm, beat: ctx.beat, playing: ctx.playing };
            self.hall.process(&self.hall_p, &mut self.tl[..len], &mut self.tr[..len], &ctx_h, None);
            let tail_peak = self.tl[..len].iter().fold(0.0f32, |m, v| m.max(v.abs()));
            self.tail_level = (self.tail_level * 0.95).max(tail_peak * 2.0);
            for j in 0..len {
                let wl = self.al[j] + self.tl[j] * 2.0;
                let wr = self.ar[j] + self.tr[j] * 2.0;
                xl[j] = xl[j] * (1.0 - mix) + wl * mix;
                xr[j] = xr[j] * (1.0 - mix) + wr * mix;
            }
            // The place's own sounds, quieter while you play.
            if amb > 0 {
                let target = 1.0 - duck_amount * 0.9 * (self.env * 6.0).min(1.0);
                let k = 1.0 - (-(len as f32) / (0.25 * sr)).exp();
                self.duck += (target - self.duck) * k;
                self.al[..len].fill(0.0);
                self.ar[..len].fill(0.0);
                self.nature.playing = ctx.playing;
                self.birds.playing = ctx.playing;
                self.nature.render(&self.nat_p, &mut self.al[..len], &mut self.ar[..len]);
                self.birds.render(&self.bird_p, &mut self.al[..len], &mut self.ar[..len]);
                let g = amb_gain * self.duck;
                let mut ap = 0.0f32;
                for j in 0..len {
                    xl[j] += self.al[j] * g;
                    xr[j] += self.ar[j] * g;
                    ap = ap.max(self.al[j].abs());
                }
                self.meter[1] = (self.meter[1] * 0.9).max(ap * g);
            } else {
                self.meter[1] *= 0.9;
                self.duck = 1.0;
            }
            self.meter[0] = self.env;
            self.meter[2] = self.duck;
            done += len;
        }
        // For the screen.
        if let Some(m) = &mut self.taps.meters {
            let v = &mut m.values;
            v[0] = self.meter[0];
            v[1] = self.meter[1];
            v[2] = self.meter[2];
            v[3] = self.tail_level;
            v[4] = self.early_level;
            v[31] = if ctx.playing && amb > 0 { 1.0 } else { 0.0 };
            if amb > 0 {
                self.nature.visual(&mut self.vis);
                // Wind, rain, lightning, crickets, brook, frogs, waves,
                // whistling, leaves.
                for (k, src) in [1usize, 5, 6, 13, 8, 14, 9, 10, 3, 4].into_iter().enumerate() {
                    v[5 + k] = self.vis[src];
                }
                self.birds.visual(&mut self.vis);
                // Eight birds: across and how loud they sing.
                for b in 0..8 {
                    let o = 4 + 5 * b;
                    let here = self.vis[o + 4] >= 0.0;
                    v[15 + 2 * b] = self.vis[o];
                    v[16 + 2 * b] = if here { self.vis[o + 2].max(0.001) } else { 0.0 };
                }
            } else {
                v[5..31].fill(0.0);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 48000.0;

    fn ctx(playing: bool) -> FxCtx {
        FxCtx { sr: SR, bpm: 120.0, beat: 0.0, playing }
    }

    /// A click, then the response for `secs`.
    fn response(p: &[f32], secs: f32) -> (Vec<f32>, Vec<f32>) {
        let mut fx = OpenAir::new(SR);
        let n = (secs * SR) as usize;
        let (mut l, mut r) = (vec![0.0f32; n], vec![0.0f32; n]);
        l[0] = 1.0;
        r[0] = 1.0;
        for (a, b) in l.chunks_mut(256).zip(r.chunks_mut(256)) {
            fx.process(p, a, b, &ctx(false));
        }
        (l, r)
    }

    fn energy(x: &[f32]) -> f32 {
        x.iter().map(|v| v * v).sum()
    }

    fn place(i: usize) -> Vec<f32> {
        let mut p = crate::params::defaults(SPECS);
        p[PLACE] = i as f32;
        p[MIX] = 1.0;
        p
    }

    #[test]
    fn every_place_answers_and_stays_bounded() {
        for i in 0..PLACES.len() {
            let (l, r) = response(&place(i), 4.0);
            assert!(l.iter().chain(&r).all(|v| v.is_finite() && v.abs() < 2.0), "{}", PLACES[i]);
            let late = energy(&l[(0.05 * SR) as usize..]);
            assert!(late > 1e-5, "{}: {late}", PLACES[i]);
        }
    }

    #[test]
    fn a_cave_rings_far_longer_than_a_meadow() {
        let tail = |i: usize| {
            let (l, _) = response(&place(i), 3.0);
            energy(&l[(1.0 * SR) as usize..])
        };
        assert!(tail(4) > 20.0 * tail(1), "cave {} meadow {}", tail(4), tail(1));
    }

    #[test]
    fn a_canyon_echoes_back() {
        // The first wall is 180 ms off at size 0.5 (times 1.0).
        let (l, r) = response(&place(2), 1.0);
        let at = |t: f32| {
            let i = (t * SR) as usize;
            energy(&l[i..i + 960]) + energy(&r[i..i + 960])
        };
        assert!(at(0.18) > 10.0 * at(0.13), "{} vs {}", at(0.18), at(0.13));
    }

    #[test]
    fn far_away_is_softer_and_duller() {
        let direct = |d: f32| {
            let mut p = place(1);
            p[DISTANCE] = d;
            let (l, _) = response(&p, 0.3);
            // The first millisecond: the direct sound alone.
            energy(&l[..48])
        };
        assert!(direct(0.0) > 4.0 * direct(1.0), "{} {}", direct(0.0), direct(1.0));
    }

    #[test]
    fn the_ambience_plays_with_the_song_and_ducks_under_the_sound() {
        let mut p = place(0);
        p[AMBIENCE] = 7.0; // water: a brook
        p[MIX] = 0.0;
        p[DUCK] = 1.0;
        let mut fx = OpenAir::new(SR);
        let run = |fx: &mut OpenAir, secs: f32, playing: bool, input: f32| {
            let n = (secs * SR) as usize;
            let (mut l, mut r) = (vec![input; n], vec![input; n]);
            for (a, b) in l.chunks_mut(256).zip(r.chunks_mut(256)) {
                fx.process(&p, a, b, &ctx(playing));
            }
            l.iter().map(|v| (v - input).abs()).sum::<f32>() / n as f32
        };
        assert!(run(&mut fx, 1.0, false, 0.0) < 1e-6, "silent while stopped");
        let free = run(&mut fx, 3.0, true, 0.0);
        assert!(free > 1e-3, "{free}");
        // A loud, steady input: the brook goes down (the input itself is
        // taken out of the measure).
        let _ = run(&mut fx, 1.0, true, 0.5);
        let ducked = run(&mut fx, 2.0, true, 0.5);
        assert!(ducked < 0.4 * free, "{ducked} vs {free}");
    }
}

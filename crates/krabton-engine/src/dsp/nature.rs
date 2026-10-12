//! Nature: the sounds of outside, made from scratch. Wind that comes in
//! gusts (a slow and a quick restless part, a little different left and
//! right), whistling where it meets an edge and rustling the leaves; rain,
//! drop by drop, on leaves, water (each drop that traps air rings as a
//! bubble that rises in pitch), a roof or a tent, with thunder that rolls in
//! after the lightning, later the further away it is; a brook of bubbles
//! and gurgling; the sea, wave after wave building, breaking and washing
//! out; crickets that chirp faster when it is warmer (as real ones do);
//! frogs; a campfire that crackles.
//!
//! Play keys: the wind whistles on those notes. It plays while the song
//! plays, always, or while a key is held (Plays). Original code.

use super::effects::FxCtx;
use super::fx::hall::{self, Hall};
use super::{db_to_gain, midi_to_freq, pan_gains, OnePole, Rng, Svf};
use crate::params::{ParamSpec, Unit};
use std::f32::consts::{PI, TAU};

pub const PLAYS: usize = 0;
pub const WIND: usize = 1;
pub const GUSTS: usize = 2;
pub const WHISTLE: usize = 3;
pub const LEAVES: usize = 4;
pub const RAIN: usize = 5;
pub const RAIN_ON: usize = 6;
pub const THUNDER: usize = 7;
pub const STREAM: usize = 8;
pub const SEA: usize = 9;
pub const WAVES: usize = 10;
pub const CRICKETS: usize = 11;
pub const WARMTH: usize = 12;
pub const FROGS: usize = 13;
pub const FIRE: usize = 14;
pub const WIDTH: usize = 15;
pub const SPACE: usize = 16;
pub const VOLUME: usize = 17;

pub const PLAY_MODES: &[&str] = &["While Playing", "Always", "While a Key Is Held"];
pub const SURFACES: &[&str] = &["Leaves", "Water", "Roof", "Tent"];

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::choice("Plays", "Wind", PLAY_MODES, 0),
    ParamSpec::lin("Wind", "Wind", 0.0, 1.0, 0.45, Unit::Percent),
    ParamSpec::lin("Gusts", "Wind", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Whistle", "Wind", 0.0, 1.0, 0.15, Unit::Percent),
    ParamSpec::lin("Leaves", "Wind", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::lin("Rain", "Rain", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::choice("Rain On", "Rain", SURFACES, 0),
    ParamSpec::lin("Thunder", "Rain", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Stream", "Water", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Sea", "Water", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Waves", "Water", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Crickets", "Night", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Warmth", "Night", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Frogs", "Night", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Fire", "Night", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Width", "Sound", 0.0, 1.0, 0.8, Unit::Percent),
    ParamSpec::lin("Space", "Sound", 0.0, 1.0, 0.15, Unit::Percent),
    ParamSpec::lin("Volume", "Sound", -24.0, 12.0, 0.0, Unit::Db),
];

pub fn preset_names() -> &'static [&'static str] {
    &["Breeze", "Windy Hill", "Singing Wind", "Gentle Rain", "Rain on the Roof", "Tent in the Rain", "Storm", "Brook", "Ocean Shore", "Summer Night", "Pond at Night", "Campfire"]
}

pub fn preset(name: &str) -> Vec<f32> {
    let mut p = crate::params::defaults(SPECS);
    let set: &[(usize, f32)] = match name {
        "Breeze" => &[(WIND, 0.35), (GUSTS, 0.5), (WHISTLE, 0.05), (LEAVES, 0.45)],
        "Windy Hill" => &[(WIND, 0.75), (GUSTS, 0.85), (WHISTLE, 0.45), (LEAVES, 0.2)],
        "Singing Wind" => &[(WIND, 0.5), (GUSTS, 0.6), (WHISTLE, 0.85), (LEAVES, 0.1), (SPACE, 0.4)],
        "Gentle Rain" => &[(WIND, 0.15), (LEAVES, 0.15), (RAIN, 0.4), (RAIN_ON, 0.0)],
        "Rain on the Roof" => &[(WIND, 0.12), (LEAVES, 0.0), (WHISTLE, 0.0), (RAIN, 0.6), (RAIN_ON, 2.0), (THUNDER, 0.15)],
        "Tent in the Rain" => &[(WIND, 0.2), (LEAVES, 0.1), (WHISTLE, 0.0), (RAIN, 0.55), (RAIN_ON, 3.0), (THUNDER, 0.25)],
        "Storm" => &[(WIND, 0.9), (GUSTS, 1.0), (WHISTLE, 0.35), (LEAVES, 0.5), (RAIN, 0.85), (RAIN_ON, 0.0), (THUNDER, 0.75), (SPACE, 0.25)],
        "Brook" => &[(WIND, 0.12), (LEAVES, 0.25), (WHISTLE, 0.0), (STREAM, 0.65)],
        "Ocean Shore" => &[(WIND, 0.35), (GUSTS, 0.4), (WHISTLE, 0.05), (LEAVES, 0.0), (SEA, 0.8), (WAVES, 0.6)],
        "Summer Night" => &[(WIND, 0.08), (LEAVES, 0.15), (WHISTLE, 0.0), (CRICKETS, 0.6), (WARMTH, 0.75), (FROGS, 0.25)],
        "Pond at Night" => &[(WIND, 0.06), (LEAVES, 0.05), (WHISTLE, 0.0), (FROGS, 0.7), (CRICKETS, 0.3), (WARMTH, 0.5), (STREAM, 0.12)],
        "Campfire" => &[(WIND, 0.12), (LEAVES, 0.1), (WHISTLE, 0.0), (FIRE, 0.7), (CRICKETS, 0.35), (WARMTH, 0.6)],
        _ => &[],
    };
    for &(i, v) in set {
        p[i] = v;
    }
    p
}

/// A restless value (Ornstein-Uhlenbeck): it wanders around zero, `rate`
/// how quickly, `size` how far.
#[derive(Clone, Copy, Default)]
struct Wander {
    x: f32,
}

impl Wander {
    fn step(&mut self, rng: &mut Rng, rate: f32, size: f32, dt: f32) -> f32 {
        let g = rng.bipolar() + rng.bipolar() + rng.bipolar();
        self.x += -rate * self.x * dt + size * (2.0 * rate * dt).sqrt() * g;
        self.x
    }
}

/// A short sound: a tone that may glide (a bubble rising) and a burst of
/// noise through a band. Raindrops, bubbles, crackles.
#[derive(Clone, Copy, Default)]
struct Grain {
    live: bool,
    f: f32,
    glide: f32,
    phase: f32,
    tone: f32,
    tone_k: f32,
    noise: f32,
    noise_k: f32,
    band: Svf,
    norm: f32,
    pl: f32,
    pr: f32,
}

const GRAINS: usize = 64;
const WHISTLES: usize = 6;
const CRICKET_N: usize = 6;
const FROG_N: usize = 5;

#[derive(Clone, Copy, Default)]
struct Cricket {
    f: f32,
    pan: f32,
    gain: f32,
    /// Where in its chirp it is (0..1 of a chirp period) and its own pace.
    chirp: f32,
    pace: f32,
    phase: f32,
    trill: bool,
    env: f32,
}

#[derive(Clone, Copy, Default)]
struct Frog {
    peeper: bool,
    pan: f32,
    gain: f32,
    pulse: f32,
    pulse_hz: f32,
    f1: Svf,
    f2: Svf,
    form: f32,
    /// Seconds into its call; the call's bursts; until the next call.
    t: f32,
    bursts: u32,
    wait: f32,
    phase: f32,
    env: f32,
}

#[derive(Clone, Copy, Default)]
struct Wave {
    t: f32,
    period: f32,
    size: f32,
    lp1: Svf,
    lp2: Svf,
    pan: f32,
}

/// A thunder: the flash first, the sound `delay` seconds later; its
/// rolls (start, level, decay).
#[derive(Clone, Copy, Default)]
struct Thunder {
    live: bool,
    t: f32,
    delay: f32,
    dist: f32,
    rolls: [(f32, f32, f32); 8],
    lp1: OnePole,
    lp2: OnePole,
    crack: f32,
    crack_hp: OnePole,
    pan: f32,
}

#[derive(Clone, Copy, Default)]
struct Whistle {
    note: u8,
    gate: bool,
    env: f32,
    band: Svf,
}

pub struct NatureState {
    /// The song plays (set by the engine every block).
    pub playing: bool,
    sr: f32,
    rng: Rng,
    held: usize,
    keys: [Whistle; WHISTLES],
    auto: [Svf; 2],
    slow: Wander,
    fast_l: Wander,
    fast_r: Wander,
    dir: Wander,
    speed: [f32; 2],
    body: [Svf; 2],
    rumble: [OnePole; 2],
    leaf_band: [Svf; 2],
    leaf_env: [f32; 2],
    rain_bed: [Svf; 2],
    stream_band: Svf,
    stream_wander: Wander,
    stream_lp: OnePole,
    grains: Vec<Grain>,
    waves: [Wave; 2],
    surf: [OnePole; 2],
    fizz: [Svf; 2],
    fizz_env: [f32; 2],
    crickets: [Cricket; CRICKET_N],
    frogs: [Frog; FROG_N],
    flick: Wander,
    fire_hiss: Svf,
    fire_roar: [OnePole; 2],
    thunder: [Thunder; 2],
    flash: f32,
    hall: Box<Hall>,
    hall_p: Vec<f32>,
    bl: Vec<f32>,
    br: Vec<f32>,
    wl: Vec<f32>,
    wr: Vec<f32>,
    /// For the screen.
    vis: [f32; 32],
}

const CHUNK: usize = 64;

impl NatureState {
    pub fn new(sr: f32) -> Self {
        let sr = sr.max(1000.0);
        let mut rng = Rng::new(0x0A7E_5EED);
        let crickets = std::array::from_fn(|i| Cricket {
            f: 3800.0 * (5200.0f32 / 3800.0).powf(rng.unipolar()),
            pan: rng.bipolar() * 0.9,
            gain: 0.35 + 0.65 * rng.unipolar(),
            chirp: rng.unipolar(),
            pace: 0.9 + 0.2 * rng.unipolar(),
            phase: 0.0,
            trill: i == CRICKET_N - 1,
            env: 0.0,
        });
        let frogs = std::array::from_fn(|i| {
            let peeper = i % 3 == 2;
            let mut f = Frog { peeper, pan: rng.bipolar() * 0.8, gain: 0.4 + 0.6 * rng.unipolar(), pulse_hz: 60.0 + 60.0 * rng.unipolar(), wait: 0.5 + 3.0 * rng.unipolar(), ..Default::default() };
            f.form = 400.0 + 500.0 * rng.unipolar();
            f.f1.set_q(f.form, 6.0, sr);
            f.f2.set_q(f.form * 2.3, 8.0, sr);
            f
        });
        let waves = std::array::from_fn(|i| Wave { t: i as f32 * 4.5, period: 9.0, size: 0.8, pan: if i == 0 { -0.45 } else { 0.45 }, ..Default::default() });
        Self {
            playing: false,
            sr,
            rng,
            held: 0,
            keys: [Whistle::default(); WHISTLES],
            auto: [Svf::new(); 2],
            slow: Wander::default(),
            fast_l: Wander::default(),
            fast_r: Wander::default(),
            dir: Wander::default(),
            speed: [0.0; 2],
            body: [Svf::new(); 2],
            rumble: [OnePole::default(); 2],
            leaf_band: [Svf::new(); 2],
            leaf_env: [0.0; 2],
            rain_bed: [Svf::new(); 2],
            stream_band: Svf::new(),
            stream_wander: Wander::default(),
            stream_lp: OnePole::default(),
            grains: vec![Grain::default(); GRAINS],
            waves,
            surf: [OnePole::default(); 2],
            fizz: [Svf::new(); 2],
            fizz_env: [0.0; 2],
            crickets,
            frogs,
            flick: Wander::default(),
            fire_hiss: Svf::new(),
            fire_roar: [OnePole::default(); 2],
            thunder: [Thunder::default(); 2],
            flash: 0.0,
            hall: Box::new(Hall::new(sr)),
            hall_p: crate::params::defaults(hall::SPECS),
            bl: vec![0.0; CHUNK],
            br: vec![0.0; CHUNK],
            wl: vec![0.0; CHUNK],
            wr: vec![0.0; CHUNK],
            vis: [0.0; 32],
        }
    }

    pub fn note_on(&mut self, _p: &[f32], note: u8, _vel: f32) {
        self.held += 1;
        // A free whistle, or the quietest one.
        let i = (0..WHISTLES).find(|&i| !self.keys[i].gate && self.keys[i].env < 1e-3).unwrap_or_else(|| (0..WHISTLES).min_by(|&a, &b| self.keys[a].env.total_cmp(&self.keys[b].env)).unwrap_or(0));
        let w = &mut self.keys[i];
        if w.note != note {
            w.band.reset();
        }
        w.note = note;
        w.gate = true;
    }

    pub fn note_off(&mut self, _p: &[f32], note: u8) {
        self.held = self.held.saturating_sub(1);
        for w in self.keys.iter_mut().filter(|w| w.note == note) {
            w.gate = false;
        }
    }

    pub fn all_off(&mut self) {
        self.held = 0;
        for w in self.keys.iter_mut() {
            w.gate = false;
        }
    }

    /// The screen: [0] on, [1] wind, [2] its direction, [3] whistling, [4]
    /// leaves, [5] rain, [6] lightning, [7] thunder, [8] brook, [9..13] two
    /// waves (how far along, how big), [13] crickets, [14] frogs, [15] fire,
    /// [16..22] each cricket, [22..27] each frog.
    pub fn visual(&self, out: &mut [f32; crate::engine::INST_VALS]) {
        out[..32].copy_from_slice(&self.vis);
        out[0] = 1.0;
    }

    fn spawn(&mut self, g: Grain) {
        let i = match self.grains.iter().position(|g| !g.live) {
            Some(i) => i,
            None => (0..GRAINS).min_by(|&a, &b| (self.grains[a].tone + self.grains[a].noise).total_cmp(&(self.grains[b].tone + self.grains[b].noise))).unwrap_or(0),
        };
        self.grains[i] = g;
    }

    /// A grain: a tone (Hz, gliding by `glide` octaves a second, level,
    /// seconds to die away) and noise (level, seconds, band Hz and Q).
    #[allow(clippy::too_many_arguments)]
    fn grain(&mut self, f: f32, glide: f32, tone: f32, tone_s: f32, noise: f32, noise_s: f32, band: f32, q: f32, pan: f32) {
        let sr = self.sr;
        let mut b = Svf::new();
        b.set_q(band.clamp(40.0, 0.45 * sr), q, sr);
        let (pl, pr) = pan_gains(pan);
        let g = Grain {
            live: true,
            f,
            glide: (glide * std::f32::consts::LN_2 / sr).exp(),
            phase: 0.0,
            tone,
            tone_k: super::decay_coef(tone_s, sr),
            noise,
            noise_k: super::decay_coef(noise_s, sr),
            band: b,
            norm: (sr / (PI * band.max(40.0) / q)).sqrt() * 0.35 / q,
            pl,
            pr,
        };
        self.spawn(g);
    }

    fn u(&mut self) -> f32 {
        self.rng.unipolar()
    }

    fn range(&mut self, a: f32, b: f32) -> f32 {
        a + (b - a) * self.rng.unipolar()
    }

    fn pitch(&mut self, a: f32, b: f32) -> f32 {
        a * (b / a).powf(self.rng.unipolar())
    }

    /// A raindrop on this surface.
    fn drop(&mut self, surface: usize, pan: f32) {
        let a = self.u().powf(1.5);
        match surface {
            // Leaves: a crisp tick.
            0 => {
                let band = self.pitch(2500.0, 7000.0);
                let ring = self.pitch(3000.0, 6000.0);
                let life = self.range(0.002, 0.005);
                self.grain(ring, 0.0, 0.08 * a, 0.004, 0.9 * a, life, band, 3.0, pan);
            }
            // Water: a splash, and often a bubble that rings and rises.
            1 => {
                let band = self.pitch(1500.0, 5000.0);
                let bubble = self.u() < 0.35;
                let f = self.pitch(1200.0, 4500.0);
                let ring = if bubble { 0.5 * a } else { 0.0 };
                let life = self.range(0.006, 0.025);
                self.grain(f, 1.0 / life * 0.7, ring, life, 0.35 * a, 0.002, band, 1.5, pan);
            }
            // Roof: a dull tap, now and then a tinkle.
            2 => {
                let band = self.pitch(600.0, 2000.0);
                let tink = self.u() < 0.1;
                let f = self.pitch(3000.0, 6000.0);
                let life = self.range(0.008, 0.02);
                self.grain(f, 0.0, if tink { 0.15 * a } else { 0.0 }, 0.03, 0.9 * a, life, band, 2.0, pan);
            }
            // Tent: a soft thump of the cloth.
            _ => {
                let f = self.pitch(120.0, 260.0);
                let life = self.range(0.025, 0.045);
                self.grain(f, -1.0, 0.35 * a, life, 0.3 * a, 0.01, 700.0, 0.7, pan);
            }
        }
    }

    fn start_thunder(&mut self) {
        let Some(i) = (0..2).find(|&i| !self.thunder[i].live) else { return };
        let dist = self.u();
        let mut rolls = [(0.0f32, 0.0f32, 1.0f32); 8];
        let n = 4 + (self.rng.next_u32() % 5) as usize;
        let len = self.range(3.0, 8.0);
        for (k, roll) in rolls.iter_mut().enumerate().take(n) {
            let start = if k == 0 { 0.0 } else { self.u() * len * 0.6 };
            *roll = (start, self.range(0.4, 1.0), self.range(0.5, 2.0) * (1.0 + dist));
        }
        let pan = self.rng.bipolar() * 0.6;
        let sr = self.sr;
        let t = &mut self.thunder[i];
        *t = Thunder { live: true, t: 0.0, delay: 0.3 + 4.0 * dist, dist, rolls, crack: if dist < 0.4 { (0.4 - dist) * 2.5 } else { 0.0 }, pan, ..Default::default() };
        t.lp1.set(70.0 + 260.0 * (1.0 - dist), sr);
        t.lp2.set(90.0 + 300.0 * (1.0 - dist), sr);
        t.crack_hp.set(900.0, sr);
        self.flash = 1.0;
    }

    pub fn render(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32]) {
        if p.len() < SPECS.len() {
            return;
        }
        let plays = match (p[PLAYS].round().max(0.0) as usize).min(PLAY_MODES.len() - 1) {
            0 => self.playing,
            1 => true,
            _ => self.held > 0,
        };
        // Everything fades in and out rather than stopping dead.
        let n = l.len().min(r.len());
        let sr = self.sr;
        let k = |i: usize| p[i].clamp(0.0, 1.0);
        let (wind, gusts, whistle, leaves) = (k(WIND), k(GUSTS), k(WHISTLE), k(LEAVES));
        let (rain, thunder, stream, sea, waves) = (k(RAIN), k(THUNDER), k(STREAM), k(SEA), k(WAVES));
        let (crickets, warmth, frogs, fire) = (k(CRICKETS), k(WARMTH), k(FROGS), k(FIRE));
        let surface = (p[RAIN_ON].round().max(0.0) as usize).min(SURFACES.len() - 1);
        let (width, space) = (k(WIDTH), k(SPACE));
        let gain = db_to_gain(p[VOLUME]);
        let mut done = 0;
        while done < n {
            let len = CHUNK.min(n - done);
            let dt = len as f32 / sr;
            let on = if plays { 1.0 } else { 0.0 };
            // ---- the wind's speed, left and right
            let slow = self.slow.step(&mut self.rng, 0.25, 0.4, dt);
            let fl = self.fast_l.step(&mut self.rng, 2.5, 0.25, dt);
            let fr = self.fast_r.step(&mut self.rng, 2.5, 0.25, dt);
            let dir = self.dir.step(&mut self.rng, 0.05, 0.6, dt).clamp(-1.0, 1.0);
            let target = |f: f32| (wind * (1.0 + gusts * (1.5 * slow + 0.9 * f))).max(0.0) * on;
            let (tl, tr) = (target(fl), target(fr));
            let s_start = self.speed;
            let a = 1.0 - (-dt * 6.0).exp();
            self.speed[0] += a * (tl - self.speed[0]);
            self.speed[1] += a * (tr - self.speed[1]);
            let sp = 0.5 * (self.speed[0] + self.speed[1]);
            for c in 0..2 {
                let s = self.speed[c].min(2.0);
                self.body[c].set_q(150.0 + 600.0 * s, 1.4, sr);
                self.rumble[c].set(90.0, sr);
                self.leaf_band[c].set_q(4200.0, 0.8, sr);
            }
            // The whistles: on the keys held, or where the wind takes them.
            let drive = (sp * 1.4).min(1.2);
            let mut whistle_level = 0.0f32;
            for w in self.keys.iter_mut() {
                if w.env > 1e-4 || w.gate {
                    let f = midi_to_freq(w.note as f32) * (1.0 + 0.006 * fl);
                    w.band.set_q(f, 70.0, sr);
                }
            }
            let auto_f = 350.0 + 900.0 * sp.min(1.5);
            self.auto[0].set_q(auto_f, 45.0, sr);
            self.auto[1].set_q(auto_f * 1.59, 45.0, sr);
            let auto_amp = whistle * ((sp - 0.3) * 1.5).clamp(0.0, 1.0).powf(1.5) * if self.keys.iter().any(|w| w.gate) { 0.3 } else { 1.0 };
            // ---- rain, thunder, brook and fire throw grains
            let drops = if rain > 0.0 && plays { (20.0 + 900.0 * rain * rain) * dt } else { 0.0 };
            let mut count = drops.floor() as u32 + u32::from(self.u() < drops.fract());
            while count > 0 {
                let pan = self.rng.bipolar();
                self.drop(surface, pan);
                count -= 1;
            }
            if thunder > 0.0 && plays && self.u() < thunder * thunder * 0.08 * dt {
                self.start_thunder();
            }
            let bubbles = if stream > 0.0 && plays { (40.0 + 250.0 * stream) * dt } else { 0.0 };
            let mut count = bubbles.floor() as u32 + u32::from(self.u() < bubbles.fract());
            while count > 0 {
                let f = self.pitch(350.0, 2200.0);
                let life = self.range(0.008, 0.03);
                let a = self.u().powi(2) * 0.5 * stream;
                let pan = self.rng.bipolar() * 0.6;
                let glide = self.range(0.4, 1.1) / life;
                self.grain(f, glide, a, life, 0.0, 0.001, 1000.0, 1.0, pan);
                count -= 1;
            }
            let flick = self.flick.step(&mut self.rng, 1.5, 0.5, dt);
            let crackles = if fire > 0.0 && plays { fire * (5.0 + 40.0 * (0.5 + flick).clamp(0.0, 1.5)) * dt } else { 0.0 };
            let mut count = crackles.floor() as u32 + u32::from(self.u() < crackles.fract());
            while count > 0 {
                let a = self.u().powi(3) * fire * 0.45;
                let band = self.pitch(1500.0, 6000.0);
                let pan = self.rng.bipolar() * 0.3;
                let life = self.range(0.0004, 0.003);
                self.grain(1000.0, 0.0, 0.0, 0.001, a, life, band, 1.5, pan);
                count -= 1;
            }
            self.fire_hiss.set_q(3500.0, 0.8, sr);
            for c in 0..2 {
                self.fire_roar[c].set(120.0, sr);
                self.surf[c].set(150.0, sr);
            }
            let bed_f = [5000.0, 3000.0, 1500.0, 800.0][surface];
            for c in 0..2 {
                self.rain_bed[c].set_q(bed_f, 0.6, sr);
                self.fizz[c].set_q(5000.0, 0.7, sr);
            }
            let sw = self.stream_wander.step(&mut self.rng, 6.0, 0.5, dt);
            self.stream_band.set_q(300.0 + 600.0 * (0.5 + sw).clamp(0.0, 1.2), 3.0, sr);
            self.stream_lp.set(200.0, sr);
            // ---- the sea's waves
            let mut wave_amp = [0.0f32; 2];
            let mut wave_fizz = [0.0f32; 2];
            for (i, wv) in self.waves.iter_mut().enumerate() {
                wv.t += dt;
                if wv.t >= wv.period {
                    wv.t -= wv.period;
                    wv.period = (6.0 + 7.0 * waves) * (0.8 + 0.4 * self.rng.unipolar());
                    wv.size = (0.6 + 0.4 * self.rng.unipolar()) * (0.5 + 0.5 * waves);
                }
                let u = wv.t / wv.period.max(0.1);
                let (a, cut) = if u < 0.45 {
                    let x = u / 0.45;
                    (0.15 + 0.45 * x * x, 250.0 + 800.0 * x)
                } else if u < 0.52 {
                    let x = (u - 0.45) / 0.07;
                    (0.6 + 0.4 * x, 1050.0 + 2200.0 * x)
                } else {
                    let x = u - 0.52;
                    ((-x * 4.5).exp(), (3250.0 * (-x * 5.0).exp()).max(250.0))
                };
                wave_amp[i] = a * wv.size;
                wave_fizz[i] = if u > 0.5 { (-(u - 0.5) * 4.0).exp() * wv.size } else { 0.0 };
                wv.lp1.set_q(cut, 0.6, sr);
                wv.lp2.set_q(cut * 1.3, 0.6, sr);
            }
            // ---- crickets: chirps per second from the warmth (about 7 per
            // minute more for every degree: Dolbear's law).
            let temp = 10.0 + 22.0 * warmth;
            let chirps = ((7.0 * temp - 30.0) / 60.0).max(0.3);
            let pulse_hz = 25.0 + temp;
            // ---- frogs call now and then
            for fr in self.frogs.iter_mut() {
                if fr.bursts == 0 {
                    fr.wait -= dt;
                    if fr.wait <= 0.0 && frogs > 0.0 && plays {
                        fr.bursts = if fr.peeper { 1 } else { 2 + self.rng.next_u32() % 2 };
                        fr.t = 0.0;
                        fr.wait = (1.5 + 3.5 * self.rng.unipolar()) / (0.4 + frogs);
                    }
                }
            }
            // ---- render the chunk
            let (bl, br) = (&mut self.bl[..len], &mut self.br[..len]);
            bl.fill(0.0);
            br.fill(0.0);
            let mut leaf_act = 0.0f32;
            let mut cricket_act = [0.0f32; CRICKET_N];
            let mut frog_act = [0.0f32; FROG_N];
            for j in 0..len {
                let x = j as f32 / len as f32;
                let s = [s_start[0] + (self.speed[0] - s_start[0]) * x, s_start[1] + (self.speed[1] - s_start[1]) * x];
                let mut out = [0.0f32; 2];
                // Wind body and its low rumble.
                for c in 0..2 {
                    let sc = s[c].min(1.6);
                    if sc > 1e-4 {
                        let nz = self.rng.bipolar();
                        // A low-pass with a hump that moves with the speed:
                        // the wind's whoosh, dark as wind is.
                        out[c] += self.body[c].tick(nz).low * 0.8 * sc.powf(1.3);
                        out[c] += self.rumble[c].lp(self.rng.bipolar()) * 1.4 * sc * sc;
                    }
                }
                // Leaves: little rustles, more and harder in a gust.
                if leaves > 0.0 {
                    for c in 0..2 {
                        let sc = s[c].min(1.5);
                        let rate = leaves * (30.0 * on + 1800.0 * sc * sc);
                        if self.rng.unipolar() < rate / sr {
                            // Many leaves at once: each a little softer.
                            let a = self.rng.unipolar();
                            self.leaf_env[c] = (self.leaf_env[c] + a * a * 0.8 * (300.0 / (rate + 300.0)).sqrt()).min(1.0);
                        }
                        let y = self.leaf_band[c].tick(self.rng.bipolar()).band * self.leaf_env[c];
                        self.leaf_env[c] *= 0.993;
                        out[c] += y * 0.35;
                        out[c] += self.rng.bipolar() * 0.012 * leaves * sc * sc;
                        leaf_act = leaf_act.max(self.leaf_env[c]);
                    }
                }
                // Whistles.
                let mut wsum = 0.0;
                for w in self.keys.iter_mut() {
                    let target = if w.gate { 1.0 } else { 0.0 };
                    let rate = if w.gate { 4.0 } else { 1.5 };
                    w.env += (target - w.env) * (rate / sr);
                    if w.env > 1e-4 {
                        let f = midi_to_freq(w.note as f32);
                        let norm = 0.3 / (0.577 * (PI * (f / 70.0) / sr).sqrt());
                        wsum += w.band.tick(self.rng.bipolar()).band / 70.0 * norm * w.env * whistle.max(0.05) * (0.3 + 0.7 * drive.min(1.0));
                    }
                }
                if auto_amp > 1e-4 {
                    let f = 350.0 + 900.0 * (0.5 * (s[0] + s[1])).min(1.5);
                    let norm = 0.3 / (0.577 * (PI * (f / 45.0) / sr).sqrt());
                    wsum += (self.auto[0].tick(self.rng.bipolar()).band + 0.6 * self.auto[1].tick(self.rng.bipolar()).band) / 45.0 * norm * auto_amp;
                }
                whistle_level = whistle_level.max(wsum.abs());
                out[0] += wsum * (0.6 + 0.4 * dir);
                out[1] += wsum * (0.6 - 0.4 * dir);
                // Rain's hiss under the drops.
                if rain > 0.0 {
                    let bed = 0.25 * rain.powf(1.5) * on;
                    for (c, o) in out.iter_mut().enumerate() {
                        *o += self.rain_bed[c].tick(self.rng.bipolar()).band * bed;
                    }
                }
                // The brook's gurgle.
                if stream > 0.0 {
                    let g = self.stream_band.tick(self.rng.bipolar()).band * 0.22 * stream * on;
                    let low = self.stream_lp.lp(self.rng.bipolar()) * 1.0 * stream * on;
                    out[0] += g + low;
                    out[1] += g * 0.8 + low;
                }
                // The sea.
                if sea > 0.0 {
                    for (i, wv) in self.waves.iter_mut().enumerate() {
                        let nz = self.rng.bipolar();
                        let y = wv.lp2.tick(wv.lp1.tick(nz).low).low * wave_amp[i] * sea * 0.65 * on;
                        let (pl, pr) = pan_gains(wv.pan);
                        out[0] += y * pl;
                        out[1] += y * pr;
                        // The fizz of the wash: bubbles bursting.
                        if wave_fizz[i] > 0.01 && self.rng.unipolar() < wave_fizz[i] * 900.0 / sr {
                            self.fizz_env[i] += self.rng.unipolar() * 0.3;
                        }
                        let f = self.fizz[i].tick(self.rng.bipolar()).band * self.fizz_env[i] * sea * on;
                        self.fizz_env[i] *= 0.99;
                        out[0] += f * pl;
                        out[1] += f * pr;
                    }
                    for (c, o) in out.iter_mut().enumerate() {
                        *o += self.surf[c].lp(self.rng.bipolar()) * 1.2 * sea * on;
                    }
                }
                // Crickets.
                if crickets > 0.0 {
                    for (i, cr) in self.crickets.iter_mut().enumerate() {
                        let period = 1.0 / (chirps * cr.pace);
                        cr.chirp = (cr.chirp + 1.0 / (period * sr)).fract();
                        let t = cr.chirp * period;
                        let pulses = if cr.trill { 1.0e9 } else { 3.0 };
                        let pr_hz = if cr.trill { pulse_hz * 1.8 } else { pulse_hz };
                        let gate = if cr.trill { (0.5 + 0.5 * (TAU * cr.chirp).cos()).powi(2) } else { 1.0 };
                        let target = if t * pr_hz < pulses && on > 0.0 {
                            let ph = (t * pr_hz).fract();
                            if ph < 0.6 {
                                (PI * ph / 0.6).sin().powi(2) * gate
                            } else {
                                0.0
                            }
                        } else {
                            0.0
                        };
                        cr.env += (target - cr.env) * 0.2;
                        cr.phase = (cr.phase + cr.f / sr).fract();
                        let (s1, c1) = (TAU * cr.phase).sin_cos();
                        let y = (s1 + 0.15 * 2.0 * s1 * c1) * cr.env * cr.gain * crickets * 0.16;
                        let (pl, pr) = pan_gains(cr.pan);
                        out[0] += y * pl;
                        out[1] += y * pr;
                        cricket_act[i] = cricket_act[i].max(cr.env);
                    }
                }
                // Frogs.
                if frogs > 0.0 {
                    for (i, fr) in self.frogs.iter_mut().enumerate() {
                        if fr.bursts == 0 && fr.env < 1e-4 {
                            continue;
                        }
                        let (burst, gap) = if fr.peeper { (0.12, 0.0) } else { (0.09, 0.06) };
                        let k = (fr.t / (burst + gap)) as u32;
                        let inb = fr.bursts > 0 && k < fr.bursts && fr.t - k as f32 * (burst + gap) < burst;
                        let target = if inb { 1.0 } else { 0.0 };
                        fr.env += (target - fr.env) * 0.004;
                        fr.t += 1.0 / sr;
                        if fr.bursts > 0 && fr.t > fr.bursts as f32 * (burst + gap) + 0.05 {
                            fr.bursts = 0;
                        }
                        let y = if fr.peeper {
                            let u = (fr.t / burst).min(1.0);
                            fr.phase = (fr.phase + (2600.0 + 400.0 * u) / sr).fract();
                            (TAU * fr.phase).sin() * 0.5
                        } else {
                            fr.pulse += fr.pulse_hz / sr;
                            let imp = if fr.pulse >= 1.0 {
                                fr.pulse -= 1.0;
                                1.0
                            } else {
                                0.0
                            };
                            (fr.f1.tick(imp).band + 0.5 * fr.f2.tick(imp).band) * 1.2
                        };
                        let y = y * fr.env * fr.gain * frogs * 0.6;
                        let (pl, pr) = pan_gains(fr.pan);
                        out[0] += y * pl;
                        out[1] += y * pr;
                        frog_act[i] = frog_act[i].max(fr.env);
                    }
                }
                // Fire: its roar and hiss (the crackles are grains).
                if fire > 0.0 {
                    let f = (0.6 + 0.4 * flick).clamp(0.2, 1.4) * fire * on;
                    // A fire is quiet: a soft roar, a little hiss.
                    let hiss = self.fire_hiss.tick(self.rng.bipolar()).band * 0.02 * f;
                    out[0] += hiss + self.fire_roar[0].lp(self.rng.bipolar()) * 0.6 * f;
                    out[1] += hiss + self.fire_roar[1].lp(self.rng.bipolar()) * 0.6 * f;
                }
                // Grains: drops, bubbles, crackles.
                for g in self.grains.iter_mut().filter(|g| g.live) {
                    let mut y = 0.0;
                    if g.tone > 1e-4 {
                        g.phase = (g.phase + g.f / sr).fract();
                        y += (TAU * g.phase).sin() * g.tone;
                        g.tone *= g.tone_k;
                        g.f = (g.f * g.glide).min(0.45 * sr);
                    }
                    if g.noise > 1e-4 {
                        y += g.band.tick(self.rng.bipolar()).band * g.norm * g.noise;
                        g.noise *= g.noise_k;
                    }
                    g.live = g.tone > 1e-4 || g.noise > 1e-4;
                    out[0] += y * g.pl * 0.5;
                    out[1] += y * g.pr * 0.5;
                }
                // Thunder.
                for th in self.thunder.iter_mut().filter(|t| t.live) {
                    th.t += 1.0 / sr;
                    let t = th.t - th.delay;
                    if t < 0.0 {
                        continue;
                    }
                    let mut env = 0.0;
                    for &(start, level, decay) in th.rolls.iter() {
                        if level > 0.0 && t >= start {
                            let x = t - start;
                            env += level * (1.0 - (-x / 0.06).exp()) * (-x / decay).exp();
                        }
                    }
                    let nz = self.rng.bipolar();
                    let y = th.lp2.lp(th.lp1.lp(nz)) * 6.0 * env.min(1.6) * (1.0 - 0.6 * th.dist) * thunder.max(0.3);
                    let crack = if th.crack > 0.0 { th.crack_hp.hp(self.rng.bipolar()) * th.crack * (-t / 0.12).exp() * 0.6 } else { 0.0 };
                    let (pl, pr) = pan_gains(th.pan);
                    out[0] += (y + crack) * pl;
                    out[1] += (y + crack) * pr;
                    if t > 1.0 && env < 1e-3 {
                        th.live = false;
                    }
                }
                bl[j] = out[0];
                br[j] = out[1];
            }
            // Width, room.
            for j in 0..len {
                let (m, s) = (0.5 * (bl[j] + br[j]), 0.5 * (bl[j] - br[j]) * width);
                bl[j] = m + s;
                br[j] = m - s;
            }
            if space > 0.001 {
                self.wl[..len].copy_from_slice(bl);
                self.wr[..len].copy_from_slice(br);
                let hp = &mut self.hall_p;
                hp[hall::PREDELAY] = 20.0;
                hp[hall::SIZE] = 0.5 + 0.5 * space;
                hp[hall::DECAY] = 1.2 + 4.0 * space;
                hp[hall::LOW_DECAY] = 0.8;
                hp[hall::DAMPING] = 6000.0;
                hp[hall::DIFFUSION] = 0.8;
                hp[hall::MODULATION] = 0.15;
                hp[hall::FREEZE] = 0.0;
                hp[hall::WIDTH] = 1.0;
                hp[hall::MIX] = 1.0;
                let ctx = FxCtx { sr, bpm: 120.0, beat: 0.0, playing: false };
                self.hall.process(&self.hall_p, &mut self.wl[..len], &mut self.wr[..len], &ctx, None);
                for j in 0..len {
                    bl[j] += self.wl[j] * space * 0.8;
                    br[j] += self.wr[j] * space * 0.8;
                }
            }
            for j in 0..len {
                l[done + j] += soft(bl[j] * gain);
                r[done + j] += soft(br[j] * gain);
            }
            // ---- for the screen
            self.flash *= (-dt * 5.0).exp();
            let rumble = self.thunder.iter().filter(|t| t.live && t.t > t.delay).map(|t| 1.0 - 0.6 * t.dist).fold(0.0, f32::max);
            let v = &mut self.vis;
            v[1] = sp;
            v[2] = dir;
            v[3] = (v[3] * 0.8).max(whistle_level * 3.0).min(1.0);
            v[4] = (v[4] * 0.85).max(leaf_act * 2.0).min(1.0);
            v[5] = if plays { rain } else { v[5] * 0.95 };
            v[6] = self.flash;
            v[7] = rumble * thunder;
            v[8] = if plays { stream } else { v[8] * 0.95 };
            v[9] = self.waves[0].t / self.waves[0].period.max(0.1);
            v[10] = self.waves[0].size * sea;
            v[11] = self.waves[1].t / self.waves[1].period.max(0.1);
            v[12] = self.waves[1].size * sea;
            v[13] = cricket_act.iter().copied().fold(0.0, f32::max) * crickets;
            v[14] = frog_act.iter().copied().fold(0.0, f32::max) * frogs;
            v[15] = (0.6 + 0.4 * flick).clamp(0.0, 1.5) * fire * on;
            for i in 0..CRICKET_N {
                v[16 + i] = cricket_act[i] * crickets;
            }
            for i in 0..FROG_N {
                v[22 + i] = frog_act[i] * frogs;
            }
            done += len;
        }
    }
}

/// Untouched up to 0.6, then bending softly towards 1: a storm stays loud
/// without clipping.
fn soft(x: f32) -> f32 {
    let a = x.abs();
    if a < 0.6 {
        x
    } else {
        x.signum() * (0.6 + 0.4 * super::fast_tanh((a - 0.6) / 0.4))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 48000.0;

    fn run(st: &mut NatureState, p: &[f32], secs: f32) -> (Vec<f32>, Vec<f32>) {
        let n = (secs * SR) as usize;
        let (mut l, mut r) = (vec![0.0; n], vec![0.0; n]);
        for (a, b) in l.chunks_mut(256).zip(r.chunks_mut(256)) {
            st.render(p, a, b);
        }
        (l, r)
    }

    fn rms(x: &[f32]) -> f32 {
        (x.iter().map(|v| v * v).sum::<f32>() / x.len().max(1) as f32).sqrt()
    }

    fn mag(x: &[f32], hz: f32) -> f32 {
        let w = TAU as f64 * hz as f64 / SR as f64;
        let n = x.len() as f64;
        let (mut re, mut im) = (0.0f64, 0.0f64);
        for (i, &s) in x.iter().enumerate() {
            let win = 0.5 - 0.5 * (TAU as f64 * i as f64 / n).cos();
            re += s as f64 * win * (w * i as f64).cos();
            im += s as f64 * win * (w * i as f64).sin();
        }
        ((re * re + im * im).sqrt() / n) as f32
    }

    #[test]
    fn every_preset_sounds_and_stays_bounded() {
        for name in preset_names() {
            let mut p = preset(name);
            p[PLAYS] = 1.0;
            let mut st = NatureState::new(SR);
            let (l, r) = run(&mut st, &p, 12.0);
            let peak = l.iter().chain(&r).fold(0.0f32, |m, v| m.max(v.abs()));
            assert!(l.iter().chain(&r).all(|v| v.is_finite()), "{name}");
            assert!(peak > 0.01 && peak < 1.2, "{name}: {peak}");
            if std::env::var("NATURE_REPORT").is_ok() {
                eprintln!("{name}: peak {peak:.3} rms {:.4}", rms(&l));
            }
        }
    }

    #[test]
    fn every_layer_alone_makes_sound() {
        for layer in [WIND, RAIN, STREAM, SEA, CRICKETS, FROGS, FIRE] {
            let mut p = crate::params::defaults(SPECS);
            p[PLAYS] = 1.0;
            p[WIND] = 0.0;
            p[LEAVES] = 0.0;
            p[WHISTLE] = 0.0;
            p[SPACE] = 0.0;
            p[layer] = 0.7;
            let mut st = NatureState::new(SR);
            let (l, _) = run(&mut st, &p, 8.0);
            let level = rms(&l);
            if std::env::var("NATURE_REPORT").is_ok() {
                let peak = l.iter().fold(0.0f32, |m, v| m.max(v.abs()));
                eprintln!("layer {layer}: rms {level:.4} peak {peak:.3}");
            }
            assert!(level > 0.002 && level < 0.5, "layer {layer}: {level}");
        }
    }

    #[test]
    fn the_wind_whistles_on_the_key_held() {
        let mut p = preset("Singing Wind");
        p[PLAYS] = 2.0;
        p[SPACE] = 0.0;
        p[LEAVES] = 0.0;
        let mut st = NatureState::new(SR);
        st.note_on(&p, 69, 0.8);
        let (l, _) = run(&mut st, &p, 4.0);
        let tail = &l[(2.0 * SR) as usize..];
        assert!(mag(tail, 440.0) > 4.0 * mag(tail, 520.0), "{} vs {}", mag(tail, 440.0), mag(tail, 520.0));
    }

    #[test]
    fn crickets_chirp_faster_when_it_is_warmer() {
        let chirps = |warmth: f32| {
            let mut p = crate::params::defaults(SPECS);
            p[PLAYS] = 1.0;
            p[WIND] = 0.0;
            p[LEAVES] = 0.0;
            p[CRICKETS] = 1.0;
            p[WARMTH] = warmth;
            p[SPACE] = 0.0;
            let mut st = NatureState::new(SR);
            st.crickets.iter_mut().skip(1).for_each(|c| c.gain = 0.0);
            let (l, _) = run(&mut st, &p, 10.0);
            // Count the chirps: the envelope rising after a quiet while.
            let mut env = 0.0f32;
            let (mut count, mut quiet) = (0, 0usize);
            for v in l {
                env = (env * 0.999).max(v.abs());
                if env < 0.002 {
                    quiet += 1;
                } else {
                    if quiet > 4000 {
                        count += 1;
                    }
                    quiet = 0;
                }
            }
            count
        };
        let (cold, warm) = (chirps(0.0), chirps(1.0));
        assert!(warm > cold + 8, "cold {cold} warm {warm}");
    }

    #[test]
    fn lightning_comes_before_its_thunder() {
        let mut p = crate::params::defaults(SPECS);
        p[PLAYS] = 1.0;
        p[WIND] = 0.0;
        p[LEAVES] = 0.0;
        p[THUNDER] = 1.0;
        p[SPACE] = 0.0;
        let mut st = NatureState::new(SR);
        st.start_thunder();
        st.thunder[0].dist = 0.8;
        st.thunder[0].delay = 2.0;
        st.thunder[0].crack = 0.0;
        let mut v = [0.0f32; crate::engine::INST_VALS];
        let (l, _) = run(&mut st, &p, 0.5);
        st.visual(&mut v);
        assert!(rms(&l) < 1e-3, "no sound yet");
        let (l, _) = run(&mut st, &p, 3.0);
        assert!(rms(&l) > 0.005, "{}", rms(&l));
    }
}

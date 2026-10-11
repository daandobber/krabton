//! Shakers: Perry Cook's PhISEM (Physically Informed Stochastic Event
//! Modeling). Many small objects (beans, beads, jingles, rocks) collide at
//! random while the shaking energy dies away; every collision adds a burst
//! of noise that rings a few resonances. Written for Krabton from Cook,
//! "Physically Informed Sonic Modeling (PhISM): Synthesis of Percussive
//! Sounds", Computer Music Journal 21:3 (1997); the instrument settings
//! follow the Shakers of his Synthesis ToolKit (STK).

use super::Rng;

/// The models' numbers are given at this rate (as in the original papers)
/// and converted to the running rate.
const REF_RATE: f32 = 22050.0;

/// One shaker: how many objects, how fast the shaking and each collision
/// die away, and the resonances (frequency in Hz, pole radius, gain).
pub struct ShakerModel {
    pub name: &'static str,
    /// Chance of a collision per sample, out of 1024, at the reference rate.
    pub objects: f32,
    pub system_decay: f32,
    pub sound_decay: f32,
    pub resonances: &'static [(f32, f32, f32)],
    /// Random detuning of the resonances on every collision (0 = none).
    pub vary: f32,
    /// Output trim so the models play at similar levels.
    pub level: f32,
}

pub const MODELS: [ShakerModel; 11] = [
    ShakerModel { name: "Maraca", objects: 25.0, system_decay: 0.999, sound_decay: 0.95, resonances: &[(3200.0, 0.96, 1.0)], vary: 0.0, level: 10.3 },
    ShakerModel { name: "Cabasa", objects: 512.0, system_decay: 0.997, sound_decay: 0.96, resonances: &[(3000.0, 0.7, 1.0)], vary: 0.0, level: 3.7 },
    ShakerModel { name: "Sekere", objects: 64.0, system_decay: 0.999, sound_decay: 0.96, resonances: &[(5500.0, 0.6, 1.0)], vary: 0.0, level: 2.95 },
    ShakerModel {
        name: "Tambourine",
        objects: 32.0,
        system_decay: 0.9985,
        sound_decay: 0.95,
        resonances: &[(2300.0, 0.96, 0.1), (5600.0, 0.995, 0.8), (8100.0, 0.995, 1.0)],
        vary: 0.05,
        level: 31.6,
    },
    ShakerModel {
        name: "Sleigh Bells",
        objects: 32.0,
        system_decay: 0.9994,
        sound_decay: 0.97,
        resonances: &[(2500.0, 0.999, 1.0), (5300.0, 0.999, 1.0), (6500.0, 0.999, 1.0), (8300.0, 0.999, 1.0), (9800.0, 0.999, 1.0)],
        vary: 0.03,
        level: 25.0,
    },
    ShakerModel {
        name: "Bamboo",
        objects: 1.25,
        system_decay: 0.9999,
        sound_decay: 0.95,
        resonances: &[(2800.0, 0.995, 1.0), (2240.0, 0.995, 1.0), (3360.0, 0.995, 1.0)],
        vary: 0.2,
        level: 7.9,
    },
    ShakerModel { name: "Sandpaper", objects: 128.0, system_decay: 0.999, sound_decay: 0.999, resonances: &[(4500.0, 0.6, 1.0)], vary: 0.0, level: 0.165 },
    ShakerModel {
        name: "Can",
        objects: 1.0,
        system_decay: 0.999,
        sound_decay: 0.97,
        resonances: &[(370.0, 0.99, 1.0), (1025.0, 0.992, 1.8), (1424.0, 0.992, 1.8), (2149.0, 0.992, 1.8), (3596.0, 0.992, 1.8)],
        vary: 0.0,
        level: 3.27,
    },
    ShakerModel { name: "Crunch", objects: 7.0, system_decay: 0.99806, sound_decay: 0.95, resonances: &[(800.0, 0.95, 1.0)], vary: 0.0, level: 12.2 },
    ShakerModel { name: "Rocks", objects: 23.0, system_decay: 0.9998, sound_decay: 0.98, resonances: &[(6460.0, 0.932, 1.0)], vary: 0.11, level: 2.78 },
    ShakerModel { name: "Gravel", objects: 1600.0, system_decay: 0.9998, sound_decay: 0.95, resonances: &[(9000.0, 0.843, 1.0)], vary: 0.18, level: 3.1 },
];

/// A two-pole resonance with zeros at DC and Nyquist, unity gain at its
/// centre.
#[derive(Clone, Copy, Default)]
struct Reson {
    a1: f32,
    a2: f32,
    b0: f32,
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
    freq: f32,
    radius: f32,
    gain: f32,
}

impl Reson {
    fn tune(&mut self, freq: f32, sr: f32) {
        let w = std::f32::consts::TAU * freq.clamp(20.0, 0.45 * sr) / sr;
        let r = self.radius;
        self.a1 = -2.0 * r * w.cos();
        self.a2 = r * r;
        self.b0 = 0.5 - 0.5 * r * r;
    }

    #[inline]
    fn tick(&mut self, x: f32) -> f32 {
        let y = self.b0 * (x - self.x2) - self.a1 * self.y1 - self.a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = x;
        self.y2 = self.y1;
        self.y1 = y;
        y
    }
}

/// How a shaker is played: everything but the model.
#[derive(Clone, Copy, Debug)]
pub struct ShakeSettings {
    /// The resonances' frequencies are multiplied by this.
    pub tune: f32,
    /// How long each shake lasts (1 = as modeled).
    pub decay: f32,
    /// How sharply the resonances ring (0.5 = as modeled).
    pub tone: f32,
    /// How many objects (0.5 = as modeled, 0 a quarter, 1 four times as many).
    pub objects: f32,
    /// How long each collision rings (0.5 = as modeled).
    pub grain: f32,
    /// How much each collision detunes the resonances (0.5 = as modeled).
    pub scatter: f32,
    /// Shakes per hit, `rate` per second.
    pub shakes: u32,
    pub rate: f32,
    /// Keep shaking at `rate` until released.
    pub hold: bool,
    /// Randomness of the shake timing (0..1).
    pub humanize: f32,
    /// Seconds over which a shake's energy comes in (0: at once).
    pub swell: f32,
    /// How far the collisions wander across the stereo field (0..1).
    pub spread: f32,
    /// Each next shake softer (up to 1) or louder (down to -1); 0: all alike.
    pub fade: f32,
}

impl Default for ShakeSettings {
    fn default() -> Self {
        Self { tune: 1.0, decay: 1.0, tone: 0.5, objects: 0.5, grain: 0.5, scatter: 0.5, shakes: 1, rate: 8.0, hold: false, humanize: 0.0, swell: 0.0, spread: 0.0, fade: 0.0 }
    }
}

/// A playing shaker.
#[derive(Clone, Copy, Default)]
pub struct Shaker {
    energy: f32,
    /// Energy still coming in (a swelling shake).
    incoming: f32,
    /// Share of `incoming` that arrives per sample (1: all at once).
    swell_coef: f32,
    level: f32,
    system_decay: f32,
    sound_decay: f32,
    /// Collision chance per sample (0..1).
    chance: f32,
    collision_gain: f32,
    vary: f32,
    trim: f32,
    sr: f32,
    res: [Reson; 5],
    /// The right channel's resonances (when spread).
    res_r: [Reson; 5],
    n: usize,
    spread: f32,
    gain_l: f32,
    gain_r: f32,
    /// Energy the next shake adds, at most `shake_cap` (the velocity).
    shake_energy: f32,
    shake_cap: f32,
    /// `shake_energy` is multiplied by this after every shake.
    fade: f32,
    shakes_left: u32,
    holding: bool,
    /// Samples to the next shake.
    countdown: f32,
    /// Shakes and collisions so far (wrapping), for displays.
    pub shakes: u32,
    pub hits: u32,
    interval: f32,
    humanize: f32,
}

/// Shaking energy never goes over this, however often it is shaken (as in STK).
const MAX_ENERGY: f32 = 1.0;

impl Shaker {
    /// Shakes model `m` once (drum pads). `tune` scales the resonances,
    /// `decay` how long the shaking lasts (1 = as modeled), `tone` how
    /// sharply they ring (0.5 = as modeled) and `objects` how many there are
    /// (0.5 = as modeled, 0 a quarter, 1 four times as many).
    pub fn trigger(&mut self, m: &ShakerModel, vel: f32, tune: f32, decay: f32, tone: f32, objects: f32, sr: f32) {
        self.start(m, vel, &ShakeSettings { tune, decay, tone, objects, ..Default::default() }, sr);
    }

    /// Starts shaking model `m` as `s` says.
    pub fn start(&mut self, m: &ShakerModel, vel: f32, s: &ShakeSettings, sr: f32) {
        let k = REF_RATE / sr;
        let per_sample = |d: f32, stretch: f32| d.powf(k / stretch.max(0.01));
        self.system_decay = per_sample(m.system_decay, s.decay);
        self.sound_decay = per_sample(m.sound_decay, 4f32.powf((s.grain - 0.5) * 2.0));
        let n_objects = m.objects * 4f32.powf((s.objects - 0.5) * 2.0);
        self.chance = (n_objects / 1024.0 * k).min(1.0);
        // Fewer objects hit harder (Cook's gain rule).
        self.collision_gain = (n_objects + 1.0).ln() / 4f32.ln() * 40.0 / n_objects.max(0.5);
        let scatter = s.scatter.clamp(0.0, 1.0);
        self.vary = m.vary * (2.0 * scatter).min(1.0) + (scatter - 0.5).max(0.0) * 0.4;
        self.trim = m.level;
        self.sr = sr;
        self.n = m.resonances.len().min(5);
        let sharp = 2f32.powf((0.5 - s.tone) * 2.0);
        for (r, &(freq, radius, gain)) in self.res.iter_mut().zip(m.resonances) {
            r.radius = (1.0 - (1.0 - radius) * sharp).clamp(0.3, 0.9995);
            r.freq = freq * s.tune;
            r.gain = gain;
            r.tune(r.freq, sr);
        }
        self.res_r = self.res;
        self.spread = s.spread.clamp(0.0, 1.0);
        self.gain_l = 1.0;
        self.gain_r = 1.0;
        let fade = s.fade.clamp(-1.0, 1.0);
        self.shake_cap = vel.clamp(0.0, 1.0);
        // Growing shakes start soft.
        self.shake_energy = self.shake_cap * (1.0 + 0.8 * fade.min(0.0));
        self.fade = 1.0 - 0.3 * fade;
        self.shakes_left = s.shakes.max(1) - 1;
        self.holding = s.hold;
        self.interval = sr / s.rate.max(0.1);
        self.countdown = self.interval;
        self.humanize = s.humanize.clamp(0.0, 1.0);
        self.swell_coef = if s.swell > 1e-3 { 1.0 - (-1.0 / (s.swell * sr)).exp() } else { 1.0 };
        self.shake();
    }

    /// One shake: its energy comes in, at once with a first collision, or
    /// swelling.
    /// How agitated the shaker is (0..1).
    pub fn energy(&self) -> f32 {
        self.energy
    }

    fn shake(&mut self) {
        self.shakes = self.shakes.wrapping_add(1);
        if self.swell_coef < 1.0 {
            self.incoming += self.shake_energy;
        } else {
            self.energy = (self.energy + self.shake_energy).min(MAX_ENERGY);
            // A shake starts with a collision, however few objects there are.
            self.level += self.collision_gain * self.energy;
        }
    }

    /// The note was let go: no more shakes (what is shaking dies away).
    pub fn release(&mut self) {
        self.holding = false;
        self.shakes_left = 0;
    }

    pub fn is_done(&self) -> bool {
        !self.holding && self.shakes_left == 0 && self.incoming < 1e-4 && self.energy < 1e-4 && self.level < 1e-4
    }

    /// The next sample, left and right.
    #[inline]
    pub fn tick(&mut self, rng: &mut Rng) -> (f32, f32) {
        if self.shakes_left > 0 || self.holding {
            self.countdown -= 1.0;
            if self.countdown <= 0.0 {
                self.shakes_left = self.shakes_left.saturating_sub(1);
                self.countdown += self.interval * (1.0 + 0.6 * self.humanize * rng.bipolar());
                self.shake_energy = (self.shake_energy * self.fade).min(self.shake_cap);
                self.shake();
            }
        }
        if self.incoming > 0.0 {
            let d = self.incoming * self.swell_coef;
            self.energy = (self.energy + d).min(MAX_ENERGY);
            self.incoming = if self.incoming - d < 1e-6 { 0.0 } else { self.incoming - d };
        }
        self.energy *= self.system_decay;
        if rng.unipolar() < self.chance {
            self.hits = self.hits.wrapping_add(1);
            self.level += self.collision_gain * self.energy;
            if self.spread > 0.0 {
                let pan = self.spread * rng.bipolar();
                self.gain_l = 1.0 - pan.max(0.0);
                self.gain_r = 1.0 + pan.min(0.0);
            }
            if self.vary > 0.0 {
                for i in 0..self.n {
                    let f = self.res[i].freq * (1.0 + self.vary * rng.bipolar());
                    self.res[i].tune(f, self.sr);
                    let (a1, a2, b0) = (self.res[i].a1, self.res[i].a2, self.res[i].b0);
                    let rr = &mut self.res_r[i];
                    (rr.a1, rr.a2, rr.b0) = (a1, a2, b0);
                }
            }
        }
        let input = self.level * rng.bipolar();
        self.level *= self.sound_decay;
        let scale = self.trim * 0.05;
        if self.spread <= 0.0 {
            let mut out = 0.0;
            for r in self.res[..self.n].iter_mut() {
                out += r.tick(input) * r.gain;
            }
            return (out * scale, out * scale);
        }
        let (mut l, mut r) = (0.0, 0.0);
        for i in 0..self.n {
            l += self.res[i].tick(input * self.gain_l) * self.res[i].gain;
            r += self.res_r[i].tick(input * self.gain_r) * self.res_r[i].gain;
        }
        (l * scale, r * scale)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(m: &ShakerModel, secs: f32) -> Vec<f32> {
        let sr = 48000.0;
        let mut s = Shaker::default();
        let mut rng = Rng::new(12345);
        s.trigger(m, 1.0, 1.0, 1.0, 0.5, 0.5, sr);
        (0..(secs * sr) as usize).map(|_| s.tick(&mut rng).0).collect()
    }

    #[test]
    fn every_shaker_rattles_and_stops() {
        for m in &MODELS {
            let out = render(m, 6.0);
            let peak = out.iter().fold(0.0f32, |a, v| a.max(v.abs()));
            let rms = (out[..4800].iter().map(|v| v * v).sum::<f32>() / 4800.0).sqrt();
            eprintln!("{:<13} peak {peak:.3} rms(100 ms) {rms:.4}", m.name);
            if std::env::var_os("SHAKER_CAL").is_some() {
                continue;
            }
            assert!(peak > 0.05 && peak < 2.5, "{}: peak {peak}", m.name);
            assert!(out.iter().all(|v| v.is_finite()), "{}", m.name);
            let tail = out[out.len() - 4800..].iter().fold(0.0f32, |a, v| a.max(v.abs()));
            assert!(tail < 0.01, "{}: still sounding after 6 s ({tail})", m.name);
        }
    }

    #[test]
    fn more_objects_make_a_denser_sound() {
        let sr = 48000.0;
        let crossings = |objects: f32| {
            let mut s = Shaker::default();
            let mut rng = Rng::new(7);
            s.trigger(&MODELS[0], 1.0, 1.0, 1.0, 0.5, objects, sr);
            let mut hits = 0;
            for _ in 0..4800 {
                let before = s.level;
                s.tick(&mut rng);
                // The level only ever falls, except on a collision.
                if s.level > before {
                    hits += 1;
                }
            }
            hits
        };
        assert!(crossings(1.0) > crossings(0.0) * 4, "{} vs {}", crossings(1.0), crossings(0.0));
    }

    /// The loudest sample from `from` seconds to the end of `secs`, and
    /// whether the shaker had stopped.
    fn tail(s: &ShakeSettings, secs: f32, from: f32, release_at: Option<f32>) -> (f32, bool) {
        let sr = 48000.0;
        let mut sh = Shaker::default();
        let mut rng = Rng::new(9);
        sh.start(&MODELS[0], 1.0, s, sr);
        let mut peak = 0.0f32;
        for i in 0..(secs * sr) as usize {
            if release_at.is_some_and(|t| i == (t * sr) as usize) {
                sh.release();
            }
            let (l, r) = sh.tick(&mut rng);
            if i >= (from * sr) as usize {
                peak = peak.max(l.abs().max(r.abs()));
            }
        }
        (peak, sh.is_done())
    }

    #[test]
    fn more_shakes_and_holding_keep_it_going() {
        let one = ShakeSettings::default();
        let four = ShakeSettings { shakes: 4, rate: 4.0, ..Default::default() };
        let held = ShakeSettings { hold: true, rate: 6.0, ..Default::default() };
        // Around 0.8 s one shake is long gone; the fourth of four is sounding.
        assert!(tail(&one, 0.9, 0.8, None).0 < 0.01);
        assert!(tail(&four, 0.9, 0.8, None).0 > 0.05);
        // Held, it keeps going; let go, it stops.
        assert!(tail(&held, 3.0, 2.8, None).0 > 0.05);
        let (peak, done) = tail(&held, 6.0, 5.8, Some(1.0));
        assert!(peak < 0.01 && done, "{peak} {done}");
    }

    #[test]
    fn a_swell_comes_in_slowly() {
        let at_once = tail(&ShakeSettings::default(), 0.03, 0.0, None).0;
        let swelling = tail(&ShakeSettings { swell: 0.5, ..Default::default() }, 0.03, 0.0, None).0;
        assert!(swelling < at_once * 0.5, "{swelling} vs {at_once}");
    }

    #[test]
    fn spread_puts_collisions_apart() {
        let sr = 48000.0;
        let mut sh = Shaker::default();
        let mut rng = Rng::new(4);
        sh.start(&MODELS[1], 1.0, &ShakeSettings { spread: 1.0, ..Default::default() }, sr);
        let (mut diff, mut sum) = (0.0f32, 0.0f32);
        for _ in 0..24000 {
            let (l, r) = sh.tick(&mut rng);
            diff += (l - r).abs();
            sum += l.abs() + r.abs();
        }
        assert!(diff > 0.2 * sum, "left and right differ: {diff} of {sum}");
    }

}

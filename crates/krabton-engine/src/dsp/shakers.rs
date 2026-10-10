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

/// A playing shaker.
#[derive(Clone, Copy, Default)]
pub struct Shaker {
    energy: f32,
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
    n: usize,
}

impl Shaker {
    /// Shakes model `m` once. `tune` scales the resonances, `decay` how
    /// long the shaking lasts (1 = as modeled), `tone` how sharply they
    /// ring (0.5 = as modeled) and `objects` how many there are (0.5 = as
    /// modeled, 0 a quarter, 1 four times as many).
    pub fn trigger(&mut self, m: &ShakerModel, vel: f32, tune: f32, decay: f32, tone: f32, objects: f32, sr: f32) {
        let k = REF_RATE / sr;
        let per_sample = |d: f32, stretch: f32| d.powf(k / stretch.max(0.01));
        self.system_decay = per_sample(m.system_decay, decay);
        self.sound_decay = per_sample(m.sound_decay, 1.0);
        let n_objects = m.objects * 4f32.powf((objects - 0.5) * 2.0);
        self.chance = (n_objects / 1024.0 * k).min(1.0);
        // Fewer objects hit harder (Cook's gain rule).
        self.collision_gain = (n_objects + 1.0).ln() / 4f32.ln() * 40.0 / n_objects.max(0.5);
        self.vary = m.vary;
        self.trim = m.level;
        self.sr = sr;
        self.n = m.resonances.len().min(5);
        let sharp = 2f32.powf((0.5 - tone) * 2.0);
        for (r, &(freq, radius, gain)) in self.res.iter_mut().zip(m.resonances) {
            r.radius = (1.0 - (1.0 - radius) * sharp).clamp(0.3, 0.9995);
            r.freq = freq * tune;
            r.gain = gain;
            r.tune(r.freq, sr);
        }
        self.energy += vel.clamp(0.0, 1.0);
        // A hit starts with a collision, however few objects there are.
        self.level = self.collision_gain * self.energy;
    }

    pub fn is_done(&self) -> bool {
        self.energy < 1e-5 && self.level < 1e-5
    }

    #[inline]
    pub fn tick(&mut self, rng: &mut Rng) -> f32 {
        self.energy *= self.system_decay;
        if rng.unipolar() < self.chance {
            self.level += self.collision_gain * self.energy;
            if self.vary > 0.0 {
                for r in self.res[..self.n].iter_mut() {
                    let f = r.freq * (1.0 + self.vary * rng.bipolar());
                    r.tune(f, self.sr);
                }
            }
        }
        let input = self.level * rng.bipolar();
        self.level *= self.sound_decay;
        let mut out = 0.0;
        for r in self.res[..self.n].iter_mut() {
            out += r.tick(input) * r.gain;
        }
        out * self.trim * 0.05
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
        (0..(secs * sr) as usize).map(|_| s.tick(&mut rng)).collect()
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
}

//! Gabber: the hardcore kick, tuned, as breakcore, gabber, speedcore and
//! terror stand on it. A sine body whose pitch falls from high (Punch) to
//! the note played over Sweep, with a click on its front; it holds (Hold)
//! and dies away (Tail), long for hardstyle, short for speedcore. Then the
//! chain that makes it what it is: Drive into an overdrive that leans one
//! way (Lean: the honk of a kick pushed through a mixer), a lift of the
//! middle (Honk, at Honk Hz), a hard clip, a wavefold (Fold), fewer bits
//! and a lower rate (Crush), a tone and the level. Every key plays it at its
//! pitch (speedcore's basslines), the tail always on the key; held, it rolls
//! (Roll: 1/8 to 1/64 of the song's beat) and rises or falls so many
//! semitones a hit (Rise). Two kicks at once at most (the old one fading as
//! the next comes, as one voice on a mixer would).
//!
//! `preview` draws one kick as it sounds now, for the face.

use crate::params::{ParamSpec, Unit};

pub const PUNCH: usize = 0;
pub const SWEEP: usize = 1;
pub const CLICK: usize = 2;
pub const HOLD: usize = 3;
pub const TAIL: usize = 4;
pub const DRIVE: usize = 5;
pub const LEAN: usize = 6;
pub const HONK: usize = 7;
pub const HONK_HZ: usize = 8;
pub const CLIP: usize = 9;
pub const FOLD: usize = 10;
pub const CRUSH: usize = 11;
pub const TONE: usize = 12;
pub const ROLL: usize = 13;
pub const RISE: usize = 14;
pub const VELOCITY: usize = 15;
pub const LEVEL: usize = 16;

pub const ROLLS: &[&str] = &["Off", "1/8", "1/16", "1/32", "1/64"];
const ROLL_BEATS: [f64; 5] = [0.0, 0.5, 0.25, 0.125, 0.0625];

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::lin("Punch", "Body", 0.0, 1.0, 0.6, Unit::Percent),
    ParamSpec::log("Sweep", "Body", 0.005, 0.2, 0.045, Unit::Sec),
    ParamSpec::lin("Click", "Body", 0.0, 1.0, 0.4, Unit::Percent),
    ParamSpec::log("Hold", "Body", 0.001, 0.3, 0.04, Unit::Sec),
    ParamSpec::log("Tail", "Body", 0.03, 2.0, 0.35, Unit::Sec),
    ParamSpec::lin("Drive", "Distortion", 0.0, 48.0, 24.0, Unit::Db),
    ParamSpec::lin("Lean", "Distortion", 0.0, 1.0, 0.35, Unit::Percent),
    ParamSpec::lin("Honk", "Distortion", 0.0, 18.0, 8.0, Unit::Db),
    ParamSpec::log("Honk Hz", "Distortion", 300.0, 4000.0, 1100.0, Unit::Hz),
    ParamSpec::lin("Clip", "Distortion", 0.0, 1.0, 0.7, Unit::Percent),
    ParamSpec::lin("Fold", "Distortion", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Crush", "Distortion", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::log("Tone", "Out", 400.0, 18000.0, 9000.0, Unit::Hz),
    ParamSpec::choice("Roll", "Roll", ROLLS, 0),
    ParamSpec::lin("Rise", "Roll", -12.0, 12.0, 0.0, Unit::Semi),
    ParamSpec::lin("Velocity", "Out", 0.0, 1.0, 0.4, Unit::Percent),
    ParamSpec::lin("Level", "Out", -24.0, 6.0, -6.0, Unit::Db),
];

const PRESETS: &[(&str, &[(usize, f32)])] = &[
    ("Gabber", &[]),
    ("Hardstyle", &[(PUNCH, 0.75), (SWEEP, 0.07), (HOLD, 0.08), (TAIL, 0.9), (DRIVE, 20.0), (LEAN, 0.2), (HONK, 12.0), (HONK_HZ, 800.0), (CLIP, 0.5), (TONE, 7000.0)]),
    ("Speedcore", &[(PUNCH, 0.5), (SWEEP, 0.02), (HOLD, 0.01), (TAIL, 0.08), (DRIVE, 36.0), (CLIP, 1.0), (CLICK, 0.7), (TONE, 12000.0)]),
    ("Terror", &[(PUNCH, 0.8), (SWEEP, 0.03), (HOLD, 0.02), (TAIL, 0.18), (DRIVE, 42.0), (FOLD, 0.6), (CRUSH, 0.35), (CLIP, 0.9), (HONK, 4.0)]),
    ("Industrial", &[(PUNCH, 0.4), (SWEEP, 0.06), (TAIL, 0.5), (DRIVE, 30.0), (LEAN, 0.7), (FOLD, 0.3), (CRUSH, 0.55), (TONE, 5000.0), (CLICK, 0.8)]),
    ("Distorted 909", &[(PUNCH, 0.55), (SWEEP, 0.035), (HOLD, 0.03), (TAIL, 0.28), (DRIVE, 16.0), (LEAN, 0.5), (HONK, 5.0), (CLIP, 0.4)]),
    ("Breakcore Thump", &[(PUNCH, 0.65), (SWEEP, 0.04), (HOLD, 0.02), (TAIL, 0.22), (DRIVE, 28.0), (HONK, 9.0), (HONK_HZ, 1500.0), (CLIP, 0.85), (CRUSH, 0.15), (ROLL, 2.0)]),
];

pub fn preset_names() -> &'static [&'static str] {
    const N: [&str; 7] = ["Gabber", "Hardstyle", "Speedcore", "Terror", "Industrial", "Distorted 909", "Breakcore Thump"];
    &N
}

pub fn preset(name: &str) -> Vec<f32> {
    let mut p = crate::params::defaults(SPECS);
    if let Some((_, set)) = PRESETS.iter().find(|(n, _)| *n == name) {
        for &(k, v) in *set {
            p[k] = v;
        }
    }
    p
}

/// One kick sounding.
#[derive(Clone, Copy, Debug, Default)]
struct Kick {
    on: bool,
    /// Samples since it struck, its note's frequency, its loudness, its fade (being taken over).
    t: u32,
    hz: f32,
    vel: f32,
    phase: f32,
    fade: f32,
    noise: u32,
}

/// The chain after the body: overdrive, the honk, the clip, the fold, the crush, the tone.
#[derive(Clone, Copy, Debug, Default)]
struct Chain {
    // The honk's peak filter (a biquad) and the tone's low pass.
    b: [f32; 5],
    z: [f32; 2],
    lp: f32,
    hold: f32,
    count: f32,
    dc: [f32; 2],
}

impl Chain {
    fn set(&mut self, p: &[f32], sr: f32) {
        // A peak at Honk Hz, Honk dB, Q about 1.
        let f = p[HONK_HZ].clamp(100.0, sr * 0.45);
        let a = 10f32.powf(p[HONK] / 40.0);
        let w = std::f32::consts::TAU * f / sr;
        let alpha = w.sin() / 2.0;
        let (b0, b1, b2, a0, a1, a2) = (1.0 + alpha * a, -2.0 * w.cos(), 1.0 - alpha * a, 1.0 + alpha / a, -2.0 * w.cos(), 1.0 - alpha / a);
        self.b = [b0 / a0, b1 / a0, b2 / a0, a1 / a0, a2 / a0];
    }

    fn run(&mut self, x: f32, p: &[f32], k_lp: f32, dc: f32) -> f32 {
        let drive = 10f32.powf(p[DRIVE] / 20.0);
        // Overdrive, leaning one way.
        let lean = p[LEAN] * 0.6;
        let mut y = ((x * drive + lean).tanh() - lean.tanh()) / (1.0 - lean.tanh().abs() * 0.5);
        // The honk.
        let o = self.b[0] * y + self.z[0];
        self.z[0] = self.b[1] * y - self.b[3] * o + self.z[1];
        self.z[1] = self.b[2] * y - self.b[4] * o;
        y = o;
        // The hard clip: from soft (0) to a wall (1).
        let c = p[CLIP].clamp(0.0, 1.0);
        let soft = (y * 1.5).tanh();
        let hard = y.clamp(-0.85, 0.85) / 0.85;
        y = soft + (hard - soft) * c;
        // The fold.
        let f = p[FOLD].clamp(0.0, 1.0);
        if f > 0.001 {
            let g = 1.0 + f * 4.0;
            y = (y * g * std::f32::consts::FRAC_PI_2).sin() * (1.0 - f * 0.3) + y * (1.0 - f) * 0.3;
        }
        // The crush: fewer bits, a lower rate.
        let cr = p[CRUSH].clamp(0.0, 1.0);
        if cr > 0.001 {
            let steps = 2f32.powf(12.0 - cr * 9.0);
            self.count += 1.0;
            if self.count >= 1.0 + cr * 12.0 {
                self.count = 0.0;
                self.hold = (y * steps).round() / steps;
            }
            y = self.hold;
        }
        // Tone, and no offset left (the lean puts one in).
        self.lp += (y - self.lp) * k_lp;
        let out = self.lp - self.dc[0] + dc * self.dc[1];
        self.dc = [self.lp, out];
        out
    }
}

/// Gabber playing.
pub struct GabberState {
    sr: f32,
    kicks: [Kick; 2],
    chain: Chain,
    /// The keys held (newest last), for the rolls; the song's tempo and place.
    held: Vec<(u8, f32)>,
    bpm: f64,
    beat: f64,
    playing: bool,
    roll_step: Option<i64>,
    rolled: i32,
    pub pitch_bend: f32,
    noise: u32,
    /// For the face: how hard it is driven now (0..1).
    pub meter: f32,
}

impl GabberState {
    pub fn new(sr: f32) -> Self {
        Self { sr, kicks: [Kick::default(); 2], chain: Chain::default(), held: Vec::with_capacity(16), bpm: 120.0, beat: 0.0, playing: false, roll_step: None, rolled: 0, pitch_bend: 0.0, noise: 0x1234_5678, meter: 0.0 }
    }

    pub fn set_transport(&mut self, bpm: f64, beat: f64, playing: bool) {
        self.bpm = bpm.max(1.0);
        self.beat = beat;
        self.playing = playing;
    }

    fn strike(&mut self, pitch: u8, vel: f32, p: &[f32]) {
        let v = 1.0 - p[VELOCITY].clamp(0.0, 1.0) * (1.0 - vel.clamp(0.0, 1.0));
        let hz = 440.0 * 2f32.powf((pitch as f32 + self.pitch_bend + self.rolled as f32 * p[RISE] - 69.0) / 12.0);
        // The one sounding fades out fast; the new one takes the other place.
        let free = if self.kicks[0].on && self.kicks[0].fade >= 1.0 { 1 } else { 0 };
        for (k, kick) in self.kicks.iter_mut().enumerate() {
            if k != free && kick.on {
                kick.fade = kick.fade.min(0.999);
            }
        }
        self.noise = self.noise.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.kicks[free] = Kick { on: true, t: 0, hz, vel: v, phase: 0.0, fade: 1.0, noise: self.noise };
    }

    pub fn note_on(&mut self, p: &[f32], pitch: u8, vel: f32) {
        if p.len() < SPECS.len() {
            return;
        }
        self.held.retain(|h| h.0 != pitch);
        if self.held.len() < self.held.capacity() {
            self.held.push((pitch, vel));
        }
        self.rolled = 0;
        self.roll_step = None;
        self.strike(pitch, vel, p);
    }

    pub fn note_off(&mut self, pitch: u8) {
        self.held.retain(|h| h.0 != pitch);
        self.rolled = 0;
    }

    pub fn all_off(&mut self) {
        self.held.clear();
        for k in &mut self.kicks {
            k.fade = k.fade.min(0.999);
        }
    }

    pub fn visual(&self, out: &mut [f32]) {
        if let Some(o) = out.get_mut(0) {
            *o = self.meter;
        }
    }

    pub fn render(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32]) {
        if p.len() < SPECS.len() {
            return;
        }
        let sr = self.sr;
        self.chain.set(p, sr);
        let k_lp = 1.0 - (-std::f32::consts::TAU * p[TONE].clamp(100.0, 20000.0) / sr).exp();
        // The offset taken out below 5 Hz (higher, a low kick's square would sag).
        let dc = 1.0 - std::f32::consts::TAU * 5.0 / sr;
        let level = 10f32.powf(p[LEVEL] / 20.0);
        let roll = ROLL_BEATS[(p[ROLL].round().max(0.0) as usize).min(ROLL_BEATS.len() - 1)];
        let beat_per_sample = self.bpm / 60.0 / sr as f64;
        let mut peak = 0.0f32;
        for i in 0..l.len() {
            // Held, it rolls on the song's grid.
            if roll > 0.0 && self.playing {
                if let Some(&(pitch, vel)) = self.held.last() {
                    let step = ((self.beat + i as f64 * beat_per_sample) / roll).floor() as i64;
                    if self.roll_step.is_some_and(|s| s != step) {
                        self.rolled += 1;
                        self.strike(pitch, vel, p);
                    }
                    self.roll_step = Some(step);
                }
            }
            let mut x = 0.0f32;
            for k in self.kicks.iter_mut().filter(|k| k.on) {
                x += kick_sample(k, p, sr);
            }
            let y = self.chain.run(x, p, k_lp, dc) * level;
            peak = peak.max(y.abs());
            l[i] += y;
            r[i] += y;
        }
        self.beat += l.len() as f64 * beat_per_sample;
        self.meter = self.meter * 0.8 + peak.min(1.5) * 0.2;
    }
}

/// The body of one kick, a sample: the pitch falling from Punch to the note
/// over Sweep, the click, the hold and the tail (and the fade when taken over).
fn kick_sample(k: &mut Kick, p: &[f32], sr: f32) -> f32 {
    let t = k.t as f32 / sr;
    let sweep = p[SWEEP].max(0.001);
    let punch = p[PUNCH].clamp(0.0, 1.0) * 5.0;
    let f = k.hz * 2f32.powf(punch * (-t / sweep).exp());
    k.phase = (k.phase + f / sr).fract();
    let body = (k.phase * std::f32::consts::TAU).sin();
    let hold = p[HOLD].max(0.0);
    let env = if t < hold { 1.0 } else { (-(t - hold) / p[TAIL].max(0.01) * 4.0).exp() };
    // The click: a few milliseconds of noise and a step.
    k.noise = k.noise.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    let n = (k.noise >> 9) as f32 / 4_194_304.0 - 1.0;
    let click = p[CLICK] * (n * 0.6 + 0.8) * (-t * 900.0).exp();
    // Taken over: gone within a few milliseconds.
    if k.fade < 1.0 {
        k.fade -= 1.0 / (0.004 * sr);
    }
    let a = k.fade.max(0.0);
    k.t += 1;
    if a <= 0.0 || env < 1e-4 {
        k.on = false;
    }
    (body * env + click) * k.vel * a
}

/// One kick as it sounds with `p` at note `pitch`: `n` points over `secs` (the peak of each stretch).
pub fn preview(p: &[f32], pitch: u8, secs: f32, n: usize) -> Vec<f32> {
    if p.len() < SPECS.len() || n == 0 {
        return vec![0.0; n];
    }
    let sr = 22050.0;
    let mut st = GabberState::new(sr);
    st.note_on(p, pitch, 1.0);
    let total = (secs * sr) as usize;
    let mut buf = vec![0.0f32; total];
    let mut r = vec![0.0f32; total];
    st.render(p, &mut buf, &mut r);
    (0..n)
        .map(|k| {
            let a = k * total / n;
            let b = ((k + 1) * total / n).max(a + 1).min(total);
            buf[a..b].iter().fold(0.0f32, |m, v| if v.abs() > m.abs() { *v } else { m })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kick(p: &[f32], pitch: u8, secs: f32) -> Vec<f32> {
        let mut st = GabberState::new(48000.0);
        st.note_on(p, pitch, 1.0);
        let mut l = vec![0.0f32; (48000.0 * secs) as usize];
        let mut r = l.clone();
        st.render(p, &mut l, &mut r);
        l
    }

    #[test]
    fn it_kicks_loud_and_dies_away() {
        let p = preset("Gabber");
        let k = kick(&p, 36, 1.5);
        let peak = k.iter().fold(0.0f32, |m, v| m.max(v.abs()));
        assert!(peak > 0.2 && peak < 1.5, "{peak}");
        let end = k[k.len() - 2000..].iter().fold(0.0f32, |m, v| m.max(v.abs()));
        assert!(end < 0.01, "it dies away: {end}");
        assert!(k.iter().all(|v| v.is_finite()));
    }

    #[test]
    fn the_tail_is_on_the_key() {
        let mut p = preset("Hardstyle");
        p[DRIVE] = 0.0;
        p[CLIP] = 0.0;
        p[HONK] = 0.0;
        // Zero crossings in the tail, a key an octave up: twice as many.
        let zc = |k: &[f32]| k[12000..24000].windows(2).filter(|w| w[0] < 0.0 && w[1] >= 0.0).count() as f32;
        let (a, b) = (zc(&kick(&p, 36, 0.6)), zc(&kick(&p, 48, 0.6)));
        assert!((b / a - 2.0).abs() < 0.2, "{a} {b}");
        // C2 is 65 Hz: about 16 crossings in a quarter of a second.
        assert!((a - 16.0).abs() < 3.0, "{a}");
    }

    #[test]
    fn drive_squares_it_up_and_rolls_roll() {
        let clean = {
            let mut p = preset("Gabber");
            p[DRIVE] = 0.0;
            p[CLIP] = 0.0;
            p[HONK] = 0.0;
            kick(&p, 36, 0.3)
        };
        let hot = kick(&preset("Gabber"), 36, 0.3);
        // Driven, it is squared up: its loudness nearer its peak.
        let full = |k: &[f32]| {
            let w = &k[200..6000];
            let peak = w.iter().fold(0.0f32, |m, v| m.max(v.abs()));
            (w.iter().map(|v| v * v).sum::<f32>() / w.len() as f32).sqrt() / peak.max(1e-6)
        };
        assert!(full(&hot) > full(&clean) * 1.2, "{} {}", full(&hot), full(&clean));
        // Held with a roll of sixteenths at 180: a hit every 83 ms.
        let mut p = preset("Gabber");
        p[ROLL] = 2.0;
        p[TAIL] = 0.03;
        p[HOLD] = 0.005;
        let mut st = GabberState::new(48000.0);
        st.set_transport(180.0, 0.0, true);
        st.note_on(&p, 36, 1.0);
        let (mut l, mut r) = (vec![0.0f32; 480], vec![0.0f32; 480]);
        for _ in 0..100 {
            l.fill(0.0);
            st.render(&p, &mut l, &mut r);
        }
        assert!((10..=13).contains(&st.rolled), "a second of sixteenths at 180: {}", st.rolled);
        st.note_off(36);
        assert_eq!(st.rolled, 0);
        for n in preset_names() {
            assert_eq!(preset(n).len(), SPECS.len());
        }
        assert_eq!(preview(&preset("Gabber"), 36, 0.5, 200).len(), 200);
    }
}

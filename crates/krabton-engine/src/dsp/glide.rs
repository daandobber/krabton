//! Glide: the long bass of trap and pluggnb, one sine that booms and slides. A
//! new note while another is held does not strike again: the pitch slides there,
//! and back when it is let go. It falls into its note from above as it starts,
//! is driven until it is heard on small speakers, and rings out as long as it is
//! told to.

use super::{fast_tanh, FilterMode, Rng, Svf};
use crate::params::{ParamSpec, Unit};

pub const DECAY: usize = 0;
pub const GLIDE: usize = 1;
pub const DROP: usize = 2;
pub const DROP_TIME: usize = 3;
pub const CLICK: usize = 4;
pub const DRIVE: usize = 5;
pub const OVERTONE: usize = 6;
pub const TONE: usize = 7;
pub const RELEASE: usize = 8;
pub const TUNE: usize = 9;
pub const VOLUME: usize = 10;

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::log("Decay", "Boom", 0.1, 8.0, 2.0, Unit::Sec),
    ParamSpec::lin("Glide", "Boom", 0.0, 800.0, 120.0, Unit::Ms),
    ParamSpec::lin("Drop", "Boom", 0.0, 24.0, 5.0, Unit::Semi),
    ParamSpec::log("Drop Time", "Boom", 5.0, 200.0, 40.0, Unit::Ms),
    ParamSpec::lin("Click", "Boom", 0.0, 1.0, 0.25, Unit::Percent),
    ParamSpec::lin("Drive", "Tone", 0.0, 1.0, 0.25, Unit::Percent),
    ParamSpec::lin("Overtone", "Tone", 0.0, 1.0, 0.1, Unit::Percent),
    ParamSpec::log("Tone", "Tone", 100.0, 12000.0, 6000.0, Unit::Hz),
    ParamSpec::log("Release", "Output", 5.0, 800.0, 60.0, Unit::Ms),
    ParamSpec::int("Tune", "Output", -24.0, 24.0, 0.0, Unit::Semi),
    ParamSpec::lin("Volume", "Output", 0.0, 1.0, 0.8, Unit::Percent),
];

pub fn preset_names() -> &'static [&'static str] {
    &["Long Boom", "Clean Sub", "Long Slide", "Short Punch", "Distorted", "Blown Speaker", "Soft Pillow", "Knock"]
}

pub fn preset(name: &str) -> Vec<f32> {
    let mut p = crate::params::defaults(SPECS);
    let set: &[(usize, f32)] = match name {
        "Clean Sub" => &[(DRIVE, 0.0), (OVERTONE, 0.0), (CLICK, 0.0), (DROP, 0.0), (DECAY, 4.0)],
        "Long Slide" => &[(GLIDE, 400.0), (DECAY, 5.0), (DROP, 2.0)],
        "Short Punch" => &[(DECAY, 0.5), (DROP, 12.0), (DROP_TIME, 30.0), (CLICK, 0.5), (GLIDE, 40.0)],
        "Distorted" => &[(DRIVE, 0.7), (OVERTONE, 0.3), (TONE, 8000.0)],
        "Blown Speaker" => &[(DRIVE, 1.0), (OVERTONE, 0.6), (TONE, 3500.0), (DECAY, 3.0)],
        "Soft Pillow" => &[(DRIVE, 0.1), (CLICK, 0.0), (TONE, 400.0), (DROP, 0.0), (DECAY, 3.0)],
        "Knock" => &[(DECAY, 0.25), (DROP, 24.0), (DROP_TIME, 18.0), (CLICK, 0.8), (DRIVE, 0.5)],
        _ => &[],
    };
    for &(i, v) in set {
        p[i] = v;
    }
    p
}

pub struct GlideState {
    sr: f32,
    pub pitch_bend: f32,
    /// The keys held, newest last.
    held: [u8; 8],
    count: usize,
    /// Where the pitch is and where it is going (in notes).
    now: f32,
    goal: f32,
    /// How far above the note it still is as it falls into it (semitones).
    fall: f32,
    phase: f32,
    /// The ring, falling from 1, and what the sound does to get there smoothly.
    ring: f32,
    level: f32,
    /// Let go: on the way out.
    gate: bool,
    velocity: f32,
    click: f32,
    rng: Rng,
    tone: Svf,
}

impl GlideState {
    pub fn new(sr: f32) -> Self {
        Self { sr, pitch_bend: 0.0, held: [0; 8], count: 0, now: 36.0, goal: 36.0, fall: 0.0, phase: 0.0, ring: 0.0, level: 0.0, gate: false, velocity: 1.0, click: 0.0, rng: Rng::new(0x51DE), tone: Svf::new() }
    }

    pub fn note_on(&mut self, p: &[f32], note: u8, vel: f32) {
        let get = |i: usize| p.get(i).copied().unwrap_or(SPECS[i].default);
        let held = self.count > 0;
        // Taken from the list if it was in it, and put on top.
        if let Some(k) = self.held[..self.count].iter().position(|n| *n == note) {
            self.held.copy_within(k + 1..self.count, k);
            self.count -= 1;
        }
        if self.count == self.held.len() {
            self.held.copy_within(1.., 0);
            self.count -= 1;
        }
        self.held[self.count] = note;
        self.count += 1;
        self.goal = note as f32;
        if held {
            // Sliding: the same breath goes on.
            return;
        }
        if get(GLIDE) <= 0.0 || self.ring < 0.01 {
            self.now = self.goal;
        }
        self.fall = get(DROP);
        self.ring = 1.0;
        self.gate = true;
        self.velocity = vel.clamp(0.0, 1.0);
        self.click = 1.0;
    }

    pub fn note_off(&mut self, _p: &[f32], note: u8) {
        if let Some(k) = self.held[..self.count].iter().position(|n| *n == note) {
            self.held.copy_within(k + 1..self.count, k);
            self.count -= 1;
        }
        if self.count > 0 {
            self.goal = self.held[self.count - 1] as f32;
        } else {
            self.gate = false;
        }
    }

    pub fn all_off(&mut self) {
        self.count = 0;
        self.gate = false;
    }

    pub fn render(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32]) {
        let get = |i: usize| p.get(i).copied().unwrap_or(SPECS[i].default);
        let sr = self.sr;
        let n = l.len().min(r.len());
        l[..n].fill(0.0);
        r[..n].fill(0.0);
        // -60 dB in the decay's time.
        let ring_k = (-6.9 / (get(DECAY).max(0.01) * sr)).exp();
        let out_k = (-6.9 / (get(RELEASE).max(1.0) * 0.001 * sr)).exp();
        let glide = get(GLIDE);
        let glide_k = if glide <= 0.0 { 0.0 } else { (-1.0 / (glide * 0.001 * 0.35 * sr)).exp() };
        let fall_k = (-1.0 / (get(DROP_TIME).max(1.0) * 0.001 * sr)).exp();
        let smooth = 1.0 - (-1.0 / (0.0015 * sr)).exp();
        let click_k = (-1.0 / (0.0015 * sr)).exp();
        let click_level = get(CLICK) * 0.5;
        let drive = 1.0 + get(DRIVE) * 12.0;
        let comp = 1.0 / fast_tanh(drive).max(0.1);
        let over = get(OVERTONE);
        let tune = get(TUNE);
        let volume = get(VOLUME);
        self.tone.set_q(get(TONE).min(sr * 0.45), 0.707, sr);
        for i in 0..n {
            if self.ring < 1e-5 && self.level < 1e-5 && self.click < 1e-4 {
                self.level = 0.0;
                continue;
            }
            self.now = self.goal + (self.now - self.goal) * glide_k;
            self.fall *= fall_k;
            self.ring *= if self.gate { ring_k } else { out_k * ring_k };
            self.level += (self.ring - self.level) * smooth;
            let note = self.now + self.fall + tune + self.pitch_bend;
            let hz = 440.0 * 2f32.powf((note - 69.0) / 12.0);
            self.phase = (self.phase + hz / sr).fract();
            let w = self.phase * std::f32::consts::TAU;
            let x = (w.sin() + over * 0.5 * (2.0 * w).sin() + over * 0.25 * (3.0 * w).sin()) * self.level;
            let y = fast_tanh(x * drive) * comp;
            self.click *= click_k;
            let burst = self.rng.bipolar() * self.click * click_level;
            let s = self.tone.process(y + burst, FilterMode::Low) * self.velocity * volume;
            l[i] += s;
            r[i] += s;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 48000.0;

    fn play(s: &mut GlideState, p: &[f32], seconds: f32) -> Vec<f32> {
        let n = (seconds * SR) as usize;
        let (mut l, mut r) = (vec![0.0; n], vec![0.0; n]);
        for (a, b) in l.chunks_mut(256).zip(r.chunks_mut(256)) {
            s.render(p, a, b);
        }
        l
    }

    /// The pitch of a stretch, from its rising zero crossings.
    fn pitch(x: &[f32]) -> f32 {
        let ups: Vec<usize> = (1..x.len()).filter(|&i| x[i - 1] < 0.0 && x[i] >= 0.0).collect();
        (ups.len() - 1) as f32 * SR / (ups[ups.len() - 1] - ups[0]) as f32
    }

    fn quiet(mut p: Vec<f32>) -> Vec<f32> {
        p[DRIVE] = 0.0;
        p[OVERTONE] = 0.0;
        p[CLICK] = 0.0;
        p[DROP] = 0.0;
        p[DECAY] = 8.0;
        p
    }

    #[test]
    fn it_plays_the_note_it_was_given() {
        let p = quiet(crate::params::defaults(SPECS));
        let mut s = GlideState::new(SR);
        s.note_on(&p, 33, 1.0);
        let x = play(&mut s, &p, 1.0);
        let f = pitch(&x[24000..]);
        assert!((f - 55.0).abs() < 0.5, "{f}");
    }

    #[test]
    fn a_new_note_held_over_slides_and_does_not_strike_again() {
        let mut p = quiet(crate::params::defaults(SPECS));
        p[GLIDE] = 100.0;
        let mut s = GlideState::new(SR);
        s.note_on(&p, 33, 1.0);
        play(&mut s, &p, 1.0);
        let ring = s.ring;
        s.note_on(&p, 45, 1.0);
        let x = play(&mut s, &p, 0.03);
        assert!(s.ring < ring, "no new blow");
        // Not there yet...
        assert!(s.now > 33.5 && s.now < 44.5, "{}", s.now);
        assert!(x.iter().all(|v| v.abs() < 1.2));
        // ...then it is, and back when let go.
        let x = play(&mut s, &p, 0.6);
        let f = pitch(&x[x.len() - 12000..]);
        assert!((f - 110.0).abs() < 1.5, "{f}");
        s.note_off(&p, 45);
        let x = play(&mut s, &p, 0.6);
        let f = pitch(&x[x.len() - 12000..]);
        assert!((f - 55.0).abs() < 1.0, "{f}");
    }

    #[test]
    fn it_falls_into_its_note() {
        let mut p = quiet(crate::params::defaults(SPECS));
        p[DROP] = 12.0;
        p[DROP_TIME] = 50.0;
        let mut s = GlideState::new(SR);
        s.note_on(&p, 45, 1.0);
        let x = play(&mut s, &p, 0.6);
        let (start, end) = (pitch(&x[..1600]), pitch(&x[x.len() - 12000..]));
        assert!(start > end * 1.3, "{start} {end}");
    }

    #[test]
    fn decay_says_how_long_it_rings_and_letting_go_ends_it() {
        let mut p = crate::params::defaults(SPECS);
        p[DECAY] = 0.5;
        p[CLICK] = 0.0;
        let mut s = GlideState::new(SR);
        s.note_on(&p, 36, 1.0);
        let x = play(&mut s, &p, 1.2);
        let peak = |a: &[f32]| a.iter().fold(0.0f32, |m, v| m.max(v.abs()));
        assert!(peak(&x[..4800]) > 0.3);
        assert!(peak(&x[x.len() - 4800..]) < 0.01, "{}", peak(&x[x.len() - 4800..]));
        p[DECAY] = 8.0;
        s.note_on(&p, 36, 1.0);
        play(&mut s, &p, 0.2);
        s.note_off(&p, 36);
        let x = play(&mut s, &p, 0.5);
        assert!(peak(&x[x.len() - 2400..]) < 0.002, "{}", peak(&x[x.len() - 2400..]));
    }

    #[test]
    fn every_preset_makes_a_clean_loud_enough_sound() {
        for name in preset_names() {
            let p = preset(name);
            assert_eq!(p.len(), SPECS.len());
            let mut s = GlideState::new(SR);
            s.note_on(&p, 36, 1.0);
            let x = play(&mut s, &p, 0.5);
            let top = x.iter().fold(0.0f32, |m, v| m.max(v.abs()));
            assert!(top > 0.2 && top < 1.3 && x.iter().all(|v| v.is_finite()), "{name}: {top}");
        }
    }
}

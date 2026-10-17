//! What the sound chips share: how a keyboard plays a chip that was never
//! made for one. Layer: every note gets a chip of its own and all its
//! channels sound it (as many chips as Voices). Split: the notes are spread
//! over the channels, one note a channel, as a real tune on one chip.
//! Arpeggio: the held notes take turns, a frame or a few each, on one chip,
//! the chord as the old computers and consoles played it. Pitch, volume and the
//! arpeggio move only on frames (50 or 60 a second), as the music drivers
//! of those machines moved them.

pub const MODES: &[&str] = &["Layer", "Split", "Arpeggio"];
pub const LAYER: usize = 0;
pub const SPLIT: usize = 1;
pub const ARP: usize = 2;

/// What a screen is told about each channel: level (0 to 1), note, shape
/// (duty, wave or pulse width) and kind, four values a channel.
pub const VIS_CHANNELS: usize = 8;
pub const KIND_PULSE: f32 = 1.0;
pub const KIND_TRIANGLE: f32 = 2.0;
pub const KIND_NOISE: f32 = 3.0;
pub const KIND_WAVE: f32 = 4.0;
pub const KIND_SID: f32 = 5.0;

pub fn mode_of(v: f32) -> usize {
    (v.round().max(0.0) as usize).min(MODES.len() - 1)
}

pub fn freq(note: f32) -> f32 {
    440.0 * 2f32.powf((note - 69.0) / 12.0)
}

/// The keys held, in the order they were played.
#[derive(Default)]
pub struct Keys {
    pub held: Vec<(u8, f32)>,
    /// The arpeggio: which of the held notes, and frames left on it.
    pub arp_pos: usize,
    arp_left: u32,
    pub clock: u64,
}

impl Keys {
    pub fn press(&mut self, note: u8, vel: f32) {
        self.held.retain(|k| k.0 != note);
        self.held.push((note, vel));
        self.clock += 1;
    }

    pub fn release(&mut self, note: u8) {
        self.held.retain(|k| k.0 != note);
    }

    pub fn clear(&mut self) {
        self.held.clear();
        self.arp_pos = 0;
        self.arp_left = 0;
    }

    /// One frame of the arpeggio: the note it is on (lowest first, up).
    pub fn arp_frame(&mut self, speed: u32) -> Option<(u8, f32)> {
        if self.held.is_empty() {
            self.arp_pos = 0;
            self.arp_left = 0;
            return None;
        }
        let mut notes = self.held.clone();
        notes.sort_by_key(|k| k.0);
        if self.arp_left == 0 {
            self.arp_pos = self.arp_pos.wrapping_add(1) % notes.len();
            self.arp_left = speed.max(1);
        }
        self.arp_left -= 1;
        Some(notes[self.arp_pos % notes.len()])
    }

    /// Starts the arpeggio over (a new chord).
    pub fn arp_restart(&mut self) {
        self.arp_pos = usize::MAX;
        self.arp_left = 0;
    }
}

/// The free slot, else the one that started longest ago.
pub fn free_or_oldest(n: usize, usable: impl Fn(usize) -> bool, idle: impl Fn(usize) -> bool, age: impl Fn(usize) -> u64) -> Option<usize> {
    let ok: Vec<usize> = (0..n).filter(|&i| usable(i)).collect();
    ok.iter().copied().find(|&i| idle(i)).or_else(|| ok.iter().copied().min_by_key(|&i| age(i)))
}

/// Counts frames: true on the sample a frame starts.
#[derive(Default, Clone, Copy)]
pub struct Frames {
    acc: f32,
}

impl Frames {
    pub fn tick(&mut self, rate: f32, sr: f32) -> bool {
        self.acc += rate / sr;
        if self.acc >= 1.0 {
            self.acc -= 1.0;
            true
        } else {
            false
        }
    }
}

/// A software envelope moved on frames, as a tracker's volume column:
/// up over the attack, down over the decay to the sustain, down over the
/// release. 0 to 1; the chip rounds it to its own steps.
#[derive(Default, Clone, Copy, PartialEq, Debug)]
pub enum Stage {
    #[default]
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

#[derive(Default, Clone, Copy)]
pub struct FrameEnv {
    pub level: f32,
    pub stage: Stage,
}

impl FrameEnv {
    pub fn gate(&mut self, on: bool) {
        if on {
            self.stage = Stage::Attack;
        } else if self.stage != Stage::Idle {
            self.stage = Stage::Release;
        }
    }

    /// One frame; times in seconds, frames a second.
    pub fn frame(&mut self, a: f32, d: f32, s: f32, r: f32, rate: f32) {
        let step = |secs: f32| 1.0 / (secs * rate).max(1.0);
        match self.stage {
            Stage::Idle => self.level = 0.0,
            Stage::Attack => {
                self.level = if a <= 0.0 { 1.0 } else { (self.level + step(a)).min(1.0) };
                if self.level >= 1.0 {
                    self.stage = Stage::Decay;
                }
            }
            Stage::Decay => {
                self.level = if d <= 0.0 { s } else { (self.level - step(d) * (1.0 - s).max(0.05)).max(s) };
                if self.level <= s {
                    self.stage = Stage::Sustain;
                }
            }
            Stage::Sustain => self.level = s,
            Stage::Release => {
                self.level = if r <= 0.0 { 0.0 } else { (self.level - step(r)).max(0.0) };
                if self.level <= 0.0 {
                    self.stage = Stage::Idle;
                }
            }
        }
    }

    pub fn idle(&self) -> bool {
        self.stage == Stage::Idle
    }
}

/// A frame-stepped vibrato (in semitones), after a delay.
pub fn vibrato(frames_on: u32, rate_frames: f32, depth_cents: f32, rate_hz: f32, delay_s: f32) -> f32 {
    let t = frames_on as f32 / rate_frames;
    if depth_cents <= 0.0 || t < delay_s {
        return 0.0;
    }
    let fade = ((t - delay_s) / 0.25).min(1.0);
    depth_cents / 100.0 * fade * (std::f32::consts::TAU * rate_hz * (t - delay_s)).sin()
}

/// One-pole filters for the consoles' outputs.
#[derive(Default, Clone, Copy)]
pub struct HighPass {
    x: f32,
    y: f32,
}

impl HighPass {
    pub fn run(&mut self, x: f32, k: f32) -> f32 {
        let y = k * (self.y + x - self.x);
        self.x = x;
        self.y = if y.abs() < 1e-20 { 0.0 } else { y };
        self.y
    }

    pub fn coef(hz: f32, sr: f32) -> f32 {
        let rc = 1.0 / (std::f32::consts::TAU * hz);
        rc / (rc + 1.0 / sr)
    }
}

#[derive(Default, Clone, Copy)]
pub struct LowPass {
    y: f32,
}

impl LowPass {
    pub fn run(&mut self, x: f32, k: f32) -> f32 {
        self.y += k * (x - self.y);
        if self.y.abs() < 1e-20 {
            self.y = 0.0;
        }
        self.y
    }

    pub fn coef(hz: f32, sr: f32) -> f32 {
        1.0 - (-std::f32::consts::TAU * hz.min(sr * 0.49) / sr).exp()
    }
}

/// A step that a chip's channel takes every `period` clock cycles, its
/// value averaged over each sample (a chip runs at a megahertz or more;
/// the average keeps most of the aliasing out).
pub fn run_steps(timer: &mut f32, period: f32, cycles: f32, cur: &mut f32, mut step: impl FnMut() -> f32) -> f32 {
    let mut left = cycles;
    let mut sum = 0.0;
    let period = period.max(1.0);
    while *timer < left {
        sum += *cur * *timer;
        left -= *timer;
        *cur = step();
        *timer = period;
    }
    sum += *cur * left;
    *timer -= left;
    sum / cycles
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_arpeggio_runs_up_the_chord() {
        let mut k = Keys::default();
        k.press(64, 1.0);
        k.press(60, 1.0);
        k.press(67, 1.0);
        k.arp_restart();
        let seen: Vec<u8> = (0..6).map(|_| k.arp_frame(1).unwrap().0).collect();
        assert_eq!(seen, vec![60, 64, 67, 60, 64, 67]);
        let slow: Vec<u8> = (0..4).map(|_| k.arp_frame(2).unwrap().0).collect();
        assert_eq!(slow[0], slow[1]);
    }

    #[test]
    fn steps_average_over_a_sample() {
        // A square of period 2 cycles (value flips each cycle), 10 cycles a sample.
        let (mut timer, mut cur) = (1.0, 1.0);
        let high = std::cell::Cell::new(true);
        let avg = run_steps(&mut timer, 1.0, 10.0, &mut cur, || {
            high.set(!high.get());
            if high.get() { 1.0 } else { 0.0 }
        });
        assert!((avg - 0.5).abs() < 0.11, "{avg}");
    }

    #[test]
    fn the_envelope_goes_round() {
        let mut e = FrameEnv::default();
        e.gate(true);
        for _ in 0..30 {
            e.frame(0.1, 0.2, 0.5, 0.3, 60.0);
        }
        assert!((e.level - 0.5).abs() < 1e-3, "{}", e.level);
        e.gate(false);
        for _ in 0..30 {
            e.frame(0.1, 0.2, 0.5, 0.3, 60.0);
        }
        assert!(e.idle());
    }
}

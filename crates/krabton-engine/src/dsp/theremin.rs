//! Theremin: played without touching it, after the classic RCA and the later
//! kit theremins. Two radio oscillators far above hearing, one fixed and one
//! pulled by the hand's distance to the pitch rod, are mixed and detected:
//! what is heard is their difference, the beat. This is modelled as it
//! happens, worked out at the beat instead of at radio frequency: an envelope
//! detector on two sines of amplitude 1 and A gives sqrt(1 + A² + 2A cos θ),
//! θ the phase between them; a small A is a sine, a large one a rectified,
//! reedy wave. The two oscillators also pull on each other (coupling), so θ
//! runs fast and slow in turn (Adler's equation, solved exactly, so the pitch
//! stays true) and the wave leans over: the nasal voice of the kit
//! theremins. Waveform turns both up together, run four times oversampled;
//! a tone filter (Brightness) and a formant follow.
//!
//! It is played as a theremin is: one voice, always gliding from note to
//! note (Glide), swelling in and out as the hand leaves and nears the volume
//! loop (Attack, Release), with the player's vibrato (after a Delay, a little
//! Irregular) and a hand that is never quite still (Wobble). Keys, MIDI and
//! clips play it; the hand on its face plays it too (`hand`), the pitch
//! anywhere between notes, or pulled onto the song's key by Snap.

use super::biquad::{self, Cascade};
use super::{db_to_gain, midi_to_freq, Rng};
use crate::params::{ParamSpec, Unit};
use std::f32::consts::{PI, TAU};

pub const GLIDE: usize = 0;
pub const OCTAVE: usize = 1;
pub const RANGE: usize = 2;
pub const SNAP: usize = 3;
pub const VELOCITY: usize = 4;
pub const WAVEFORM: usize = 5;
pub const BRIGHTNESS: usize = 6;
pub const FORMANT: usize = 7;
pub const VIBRATO: usize = 8;
pub const RATE: usize = 9;
pub const DELAY: usize = 10;
pub const IRREGULAR: usize = 11;
pub const WOBBLE: usize = 12;
pub const ATTACK: usize = 13;
pub const RELEASE: usize = 14;
pub const VOLUME: usize = 15;

/// Oversampling of the oscillators.
const OS: usize = 4;
/// Below this the level is silence.
const SILENT: f32 = 1e-4;
/// The hand is gone when nothing is heard of it for this long (seconds).
const HAND_TIMEOUT: f32 = 0.5;
/// Where the formant sits (Hz).
const FORMANT_HZ: f64 = 1100.0;

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::log("Glide", "Play", 0.001, 2.0, 0.12, Unit::Sec),
    ParamSpec::int("Octave", "Play", -2.0, 2.0, 0.0, Unit::None),
    ParamSpec::int("Range", "Play", 2.0, 4.0, 3.0, Unit::None),
    ParamSpec::lin("Snap", "Play", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Velocity", "Play", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::lin("Waveform", "Tone", 0.0, 1.0, 0.35, Unit::Percent),
    ParamSpec::lin("Brightness", "Tone", 0.0, 1.0, 0.6, Unit::Percent),
    ParamSpec::lin("Formant", "Tone", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::lin("Vibrato", "Hand", 0.0, 100.0, 20.0, Unit::Cents),
    ParamSpec::lin("Rate", "Hand", 3.0, 9.0, 5.8, Unit::Hz),
    ParamSpec::lin("Delay", "Hand", 0.0, 2.0, 0.35, Unit::Sec),
    ParamSpec::lin("Irregular", "Hand", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::lin("Wobble", "Hand", 0.0, 60.0, 6.0, Unit::Cents),
    ParamSpec::log("Attack", "Volume", 0.005, 3.0, 0.25, Unit::Sec),
    ParamSpec::log("Release", "Volume", 0.01, 5.0, 0.6, Unit::Sec),
    ParamSpec::lin("Volume", "Volume", -30.0, 6.0, -6.0, Unit::Db),
];

pub fn preset_names() -> &'static [&'static str] {
    &["Classic", "Nasal", "Spooky", "Whistle", "Cello", "Soprano", "Wobbly", "In Tune", "Bright Lead", "Deep"]
}

pub fn preset(name: &str) -> Vec<f32> {
    let mut p = crate::params::defaults(SPECS);
    let set: &[(usize, f32)] = match name {
        // A concert theremin: nearly a sine, singing, the vibrato the player's.
        "Classic" => &[(WAVEFORM, 0.15), (BRIGHTNESS, 0.55), (FORMANT, 0.2), (GLIDE, 0.1), (VIBRATO, 22.0), (RATE, 6.2), (DELAY, 0.3), (IRREGULAR, 0.25), (WOBBLE, 4.0), (ATTACK, 0.18), (RELEASE, 0.5)],
        // The wooden kit box: leaning over, reedy, through its formant.
        "Nasal" => &[(WAVEFORM, 0.7), (BRIGHTNESS, 0.5), (FORMANT, 0.65), (GLIDE, 0.09), (VIBRATO, 18.0), (RATE, 5.6), (DELAY, 0.4), (ATTACK, 0.12), (RELEASE, 0.4)],
        // The flying saucer of the fifties' films: wide, slow, wavering.
        "Spooky" => &[(WAVEFORM, 0.3), (BRIGHTNESS, 0.5), (FORMANT, 0.25), (GLIDE, 0.4), (OCTAVE, 1.0), (VIBRATO, 65.0), (RATE, 6.8), (DELAY, 0.0), (IRREGULAR, 0.5), (WOBBLE, 25.0), (ATTACK, 0.5), (RELEASE, 1.4)],
        "Whistle" => &[(WAVEFORM, 0.0), (BRIGHTNESS, 0.4), (FORMANT, 0.0), (GLIDE, 0.06), (OCTAVE, 1.0), (VIBRATO, 14.0), (RATE, 5.4), (DELAY, 0.25), (WOBBLE, 3.0), (ATTACK, 0.08), (RELEASE, 0.3)],
        "Cello" => &[(WAVEFORM, 0.6), (BRIGHTNESS, 0.42), (FORMANT, 0.55), (GLIDE, 0.08), (OCTAVE, -1.0), (VIBRATO, 18.0), (RATE, 5.2), (DELAY, 0.45), (IRREGULAR, 0.35), (ATTACK, 0.14), (RELEASE, 0.5)],
        "Soprano" => &[(WAVEFORM, 0.45), (BRIGHTNESS, 0.5), (FORMANT, 0.85), (GLIDE, 0.11), (VIBRATO, 30.0), (RATE, 5.9), (DELAY, 0.5), (IRREGULAR, 0.4), (ATTACK, 0.2), (RELEASE, 0.6)],
        "Wobbly" => &[(WAVEFORM, 0.4), (GLIDE, 0.25), (VIBRATO, 35.0), (RATE, 4.6), (DELAY, 0.0), (IRREGULAR, 0.9), (WOBBLE, 45.0), (ATTACK, 0.3), (RELEASE, 0.9)],
        // The hand pulled onto the key's notes, gliding between them.
        "In Tune" => &[(WAVEFORM, 0.3), (SNAP, 0.9), (GLIDE, 0.08), (VIBRATO, 20.0), (DELAY, 0.3), (WOBBLE, 2.0), (ATTACK, 0.1), (RELEASE, 0.4)],
        "Bright Lead" => &[(WAVEFORM, 1.0), (BRIGHTNESS, 0.85), (FORMANT, 0.4), (GLIDE, 0.05), (VIBRATO, 15.0), (DELAY, 0.6), (WOBBLE, 2.0), (ATTACK, 0.02), (RELEASE, 0.25), (VOLUME, -9.0)],
        "Deep" => &[(WAVEFORM, 0.5), (BRIGHTNESS, 0.35), (FORMANT, 0.3), (GLIDE, 0.15), (OCTAVE, -1.0), (VIBRATO, 12.0), (RATE, 4.8), (DELAY, 0.6), (ATTACK, 0.3), (RELEASE, 1.0)],
        _ => &[],
    };
    for &(i, v) in set {
        p[i] = v;
    }
    p
}

/// Coupling and detector depth for a Waveform (0..1).
fn coupling(waveform: f32) -> (f32, f32) {
    let w = waveform.clamp(0.0, 1.0);
    (0.82 * w, 0.18 + 0.78 * w)
}

/// The detected beat at half its phase `h` (0..PI): the pulled phase between
/// the oscillators from the even one (coupling `c`, `s` = sqrt(1 - c²)),
/// through the envelope detector (depth `a`), about -1..1.
#[inline]
fn detect(h: f32, c: f32, s: f32, a: f32) -> f32 {
    let (sin, cos) = h.sin_cos();
    // tan(θ/2) = c + s·tan(h): the exact solution of the pulled phase.
    let (x, y) = (cos, s * sin + c * cos);
    let cos_t = (x * x - y * y) / (x * x + y * y);
    ((1.0 + a * a + 2.0 * a * cos_t).max(0.0).sqrt() - 1.0) / a
}

/// One period of the wave a Waveform makes, at `phase` (0..1), for screens.
pub fn wave_at(waveform: f32, phase: f32) -> f32 {
    let (c, a) = coupling(waveform);
    detect(PI * phase.rem_euclid(1.0), c, (1.0 - c * c).sqrt(), a)
}

/// Pulls a pitch toward the nearest note of `key` (a bit per pitch class)
/// by `amount`: 0 leaves it, 1 lands on the note.
pub fn snap(pitch: f32, key: u16, amount: f32) -> f32 {
    let amount = amount.clamp(0.0, 1.0);
    if amount <= 0.0 || key & 0xFFF == 0 || !pitch.is_finite() {
        return pitch;
    }
    let base = pitch.round() as i32;
    let mut best = base as f32;
    let mut dist = f32::MAX;
    for k in -6..=6 {
        let n = base + k;
        if key >> n.rem_euclid(12) & 1 == 1 && (n as f32 - pitch).abs() < dist {
            dist = (n as f32 - pitch).abs();
            best = n as f32;
        }
    }
    pitch + (best - pitch) * amount
}

/// A value wandering smoothly between random points (-1..1).
#[derive(Clone, Copy, Default)]
struct Wander {
    from: f32,
    to: f32,
    t: f32,
}

impl Wander {
    /// `step`: how far to the next point each sample (1 / samples between points).
    #[inline]
    fn tick(&mut self, rng: &mut Rng, step: f32) -> f32 {
        self.t += step;
        if self.t >= 1.0 {
            self.t -= 1.0;
            self.from = self.to;
            self.to = rng.bipolar();
        }
        let s = self.t * self.t * (3.0 - 2.0 * self.t);
        self.from + (self.to - self.from) * s
    }
}

pub struct ThereminState {
    pub pitch_bend: f32,
    /// The song's key, a bit per pitch class (for Snap).
    pub key: u16,
    sr: f32,
    /// Keys held, the latest last.
    held: Vec<(u8, f32)>,
    note: u8,
    gate: bool,
    vel: f32,
    /// The hand: its pitch (MIDI) and its height (0..1), and how long since it was last heard of.
    hand: Option<(f32, f32)>,
    hand_age: usize,
    /// Where the vibrato's delay last started from (the hand's pitch).
    hand_anchor: f32,
    /// Where the pitch glides to and where it is (MIDI notes).
    target: f32,
    pitch: f32,
    /// The volume (0..1; heard as its square).
    level: f32,
    /// Seconds since the last new note, for the vibrato's delay.
    since: f32,
    vib_amt: f32,
    vib_phase: f32,
    vib_rate: Wander,
    vib_depth: Wander,
    wobble: Wander,
    drift: Wander,
    rng: Rng,
    /// The even phase of the beat (0..1).
    phase: f32,
    decimate: Cascade,
    dec: [biquad::State; 3],
    tone: biquad::State,
    formant: biquad::State,
    /// Takes out the detector's steady part (low enough to keep the lowest notes).
    dc: biquad::State,
    /// For the screen: the pitch heard (MIDI), the volume heard, the pitch before the player's hand.
    shown: (f32, f32, f32),
}

impl ThereminState {
    pub fn new(sr: f32) -> Self {
        let sr = sr.max(8000.0);
        Self {
            pitch_bend: 0.0,
            key: 0xFFF,
            sr,
            held: Vec::with_capacity(16),
            note: 0,
            gate: false,
            vel: 0.0,
            hand: None,
            hand_age: 0,
            hand_anchor: 0.0,
            target: 60.0,
            pitch: 60.0,
            level: 0.0,
            since: 0.0,
            vib_amt: 0.0,
            vib_phase: 0.0,
            vib_rate: Wander::default(),
            vib_depth: Wander::default(),
            wobble: Wander::default(),
            drift: Wander::default(),
            rng: Rng::new(0x7E2E_3141),
            phase: 0.0,
            decimate: biquad::butterworth(6, false, 0.3 * sr as f64, (OS as f32 * sr) as f64, std::f64::consts::FRAC_1_SQRT_2),
            dec: [biquad::State::default(); 3],
            tone: biquad::State::default(),
            formant: biquad::State::default(),
            dc: biquad::State::default(),
            shown: (0.0, 0.0, 0.0),
        }
    }

    fn sounding(&self) -> bool {
        self.gate || self.hand.is_some() || self.level > SILENT
    }

    /// Starts a new pitch: from silence it starts there, while sounding it glides.
    fn go_to(&mut self, pitch: f32) {
        if !self.sounding() {
            self.pitch = pitch;
            self.vib_amt = 0.0;
        }
        self.target = pitch;
        self.since = 0.0;
    }

    pub fn note_on(&mut self, p: &[f32], note: u8, vel: f32) {
        if p.len() < SPECS.len() {
            return;
        }
        self.held.retain(|h| h.0 != note);
        if self.held.len() == self.held.capacity() {
            self.held.remove(0);
        }
        self.held.push((note, vel));
        let pitch = note as f32 + 12.0 * p[OCTAVE].round();
        if self.hand.is_none() {
            self.go_to(pitch);
        } else {
            self.target = pitch;
        }
        self.note = note;
        self.gate = true;
        self.vel = vel;
    }

    pub fn note_off(&mut self, p: &[f32], note: u8) {
        self.held.retain(|h| h.0 != note);
        if self.note != note {
            return;
        }
        // Back to a key still held, gliding there.
        match self.held.last() {
            Some(&(prev, vel)) => {
                self.note = prev;
                self.vel = vel;
                self.target = prev as f32 + 12.0 * p.get(OCTAVE).map_or(0.0, |o| o.round());
                if self.hand.is_none() {
                    self.since = 0.0;
                }
            }
            None => self.gate = false,
        }
    }

    pub fn all_off(&mut self) {
        self.held.clear();
        self.gate = false;
        self.hand = None;
    }

    /// The player's hand over it: a pitch (a MIDI note, between notes too)
    /// and how far it is from the volume loop (0: at it, silent; 1: loud),
    /// or None when it is taken away. It holds until the next word of it,
    /// for half a second.
    pub fn hand(&mut self, at: Option<(f32, f32)>) {
        match at {
            Some((pitch, height)) if pitch.is_finite() && height.is_finite() => {
                if self.hand.is_none() {
                    self.go_to(pitch);
                    self.hand_anchor = pitch;
                }
                // A hand moving to another note starts the vibrato's delay again.
                if (pitch - self.hand_anchor).abs() > 0.7 {
                    self.hand_anchor = pitch;
                    self.since = 0.0;
                }
                self.hand = Some((pitch, height.clamp(0.0, 1.0)));
                self.hand_age = 0;
            }
            _ => {
                if self.hand.take().is_some() {
                    if self.gate {
                        // Back to the key held.
                        self.since = 0.0;
                    } else {
                        // It fades where the hand left it.
                        self.target = self.pitch;
                    }
                }
            }
        }
    }

    /// For the screen: sounding (0 or 1), the pitch heard (MIDI), its volume
    /// (0..1), the pitch before vibrato and wobble, and whether a hand plays it.
    pub fn visual(&self, out: &mut [f32; crate::engine::INST_VALS]) {
        out[0] = if self.sounding() { 1.0 } else { 0.0 };
        out[1] = self.shown.0;
        out[2] = self.shown.1;
        out[3] = self.shown.2;
        out[4] = if self.hand.is_some() { 1.0 } else { 0.0 };
    }

    pub fn render(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32]) {
        if p.len() < SPECS.len() {
            return;
        }
        let n = l.len().min(r.len());
        let sr = self.sr;
        if self.hand.is_some() {
            self.hand_age += n;
            if self.hand_age as f32 > HAND_TIMEOUT * sr {
                self.hand(None);
            }
        }
        if !self.sounding() {
            self.level = 0.0;
            self.shown.1 = 0.0;
            return;
        }
        let snap_amt = p[SNAP].clamp(0.0, 1.0);
        let glide = p[GLIDE].max(0.001);
        // The keys glide over Glide; the hand follows within a few
        // milliseconds, and with Snap glides from note to note.
        let tau = if self.hand.is_some() { 0.012 + snap_amt * glide * 0.3 } else { glide * 0.3 };
        let glide_k = 1.0 - (-1.0 / (tau * sr)).exp();
        let rise = (-4.6 / (p[ATTACK].max(0.001) * sr)).exp();
        let fall = (-4.6 / (p[RELEASE].max(0.001) * sr)).exp();
        let hand_k = (-4.6 / (0.03 * sr)).exp();
        let vel_amt = p[VELOCITY].clamp(0.0, 1.0);
        let gain = db_to_gain(p[VOLUME]) * 0.7;
        let (c0, a0) = coupling(p[WAVEFORM]);
        let depth = p[VIBRATO].max(0.0) / 100.0;
        let rate = p[RATE].clamp(0.5, 20.0);
        let delay = p[DELAY].max(0.0);
        let irregular = p[IRREGULAR].clamp(0.0, 1.0);
        let wobble = p[WOBBLE].max(0.0) / 100.0;
        let fade_in = 1.0 - (-1.0 / (0.35 * sr)).exp();
        let fade_out = 1.0 - (-1.0 / (0.06 * sr)).exp();
        // The tone filter and the formant, as set.
        let bright = p[BRIGHTNESS].clamp(0.0, 1.0);
        let cut = (300.0 * 2f32.powf(bright * 5.6)).min(sr * 0.45);
        let tone = biquad::lowpass(cut as f64, sr as f64, 0.65);
        let formant = biquad::bell(FORMANT_HZ, sr as f64, 1.3, (p[FORMANT].clamp(0.0, 1.0) * 10.0) as f64);
        let dc = biquad::highpass1(8.0, sr as f64);
        let dt = 1.0 / sr;
        let mut shown = self.shown;
        for i in 0..n {
            // Where to: the hand (pulled onto the key), or the keys.
            let (target, want) = match self.hand {
                Some((pitch, height)) => (snap(pitch, self.key, snap_amt), height),
                None if self.gate => (self.target, 1.0 - vel_amt * (1.0 - self.vel.clamp(0.0, 1.0))),
                None => (self.target, 0.0),
            };
            self.pitch += (target - self.pitch) * glide_k;
            let k = if self.hand.is_some() {
                hand_k
            } else if want > self.level {
                rise
            } else {
                fall
            };
            self.level = want + (self.level - want) * k;
            // The player's vibrato after its delay, never quite even; the hand's wobble.
            self.since += dt;
            let on = if self.since >= delay { 1.0 } else { 0.0 };
            self.vib_amt += (on - self.vib_amt) * if on > self.vib_amt { fade_in } else { fade_out };
            let jr = self.vib_rate.tick(&mut self.rng, 1.3 * dt);
            let jd = self.vib_depth.tick(&mut self.rng, 0.9 * dt);
            self.vib_phase = (self.vib_phase + rate * (1.0 + 0.14 * irregular * jr) * dt).fract();
            let vib = (TAU * self.vib_phase).sin() * depth * self.vib_amt * (1.0 + 0.4 * irregular * jd);
            let wob = wobble * (0.6 * self.wobble.tick(&mut self.rng, 2.2 * dt) + 0.4 * self.drift.tick(&mut self.rng, 0.3 * dt));
            let heard = self.pitch + self.pitch_bend + vib + wob;
            let f = midi_to_freq(heard).min(sr * 0.45);
            // High notes get fewer harmonics, as the circuit's own bandwidth gives them.
            let high = ((f - 900.0) / 3000.0).clamp(0.0, 0.7);
            let c = c0 * (1.0 - high);
            let a = a0 - (a0 - 0.18) * high;
            let s = (1.0 - c * c).sqrt();
            let step = f / (sr * OS as f32);
            let mut y = 0.0;
            for _ in 0..OS {
                self.phase += step;
                if self.phase >= 1.0 {
                    self.phase -= 1.0;
                }
                let mut v = detect(PI * self.phase, c, s, a) as f64;
                for (st, co) in self.dec.iter_mut().zip(&self.decimate.sections[..self.decimate.n]) {
                    v = st.run(co, v);
                }
                y = v;
            }
            let x = self.dc.run(&dc, self.formant.run(&formant, self.tone.run(&tone, y))) as f32;
            let out = x * self.level * self.level * gain;
            l[i] += out;
            r[i] += out;
            shown = (heard, self.level * self.level, self.pitch);
        }
        self.shown = shown;
        if !self.sounding() {
            self.level = 0.0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 48_000.0;

    /// Renders `secs`, calling `at` before each block of 64 with the time.
    fn run(st: &mut ThereminState, p: &[f32], secs: f32, mut at: impl FnMut(&mut ThereminState, f32)) -> Vec<f32> {
        let mut out = Vec::new();
        let blocks = (secs * SR / 64.0) as usize;
        for b in 0..blocks {
            at(st, b as f32 * 64.0 / SR);
            let (mut l, mut r) = (vec![0.0; 64], vec![0.0; 64]);
            st.render(p, &mut l, &mut r);
            out.extend(l);
        }
        out
    }

    /// Still: no vibrato, no wobble.
    fn still(name: &str) -> Vec<f32> {
        let mut p = preset(name);
        (p[VIBRATO], p[WOBBLE], p[IRREGULAR]) = (0.0, 0.0, 0.0);
        p
    }

    fn rms(x: &[f32]) -> f32 {
        (x.iter().map(|v| v * v).sum::<f32>() / x.len().max(1) as f32).sqrt()
    }

    /// `x` through a Hann window.
    fn hann(x: &[f32]) -> Vec<f32> {
        let n = x.len() as f32;
        x.iter().enumerate().map(|(k, v)| v * (0.5 - 0.5 * (TAU * k as f32 / n).cos())).collect()
    }

    /// Strength of `f` in a windowed `x` (Goertzel).
    fn goertzel(x: &[f32], f: f32) -> f32 {
        let c = 2.0 * (TAU as f64 * f as f64 / SR as f64).cos();
        let (mut s1, mut s2) = (0.0f64, 0.0f64);
        for &v in x {
            let s = v as f64 + c * s1 - s2;
            s2 = s1;
            s1 = s;
        }
        (s1 * s1 + s2 * s2 - c * s1 * s2) as f32
    }

    /// Strength of `f` in `x`.
    fn power(x: &[f32], f: f32) -> f32 {
        goertzel(&hann(x), f)
    }

    /// The strongest frequency between `lo` and `hi`, to a tenth of a hertz.
    fn peak(x: &[f32], lo: f32, hi: f32) -> f32 {
        let x = hann(x);
        let mut best = (lo, 0.0);
        let mut f = lo;
        while f <= hi {
            let pw = goertzel(&x, f);
            if pw > best.1 {
                best = (f, pw);
            }
            f += 0.1;
        }
        best.0
    }

    #[test]
    fn it_plays_in_tune_whatever_the_waveform() {
        for w in [0.0, 0.5, 1.0] {
            let mut p = still("Classic");
            p[WAVEFORM] = w;
            let mut st = ThereminState::new(SR);
            st.note_on(&p, 69, 1.0);
            let out = run(&mut st, &p, 1.5, |_, _| {});
            let f = peak(&out[24_000..72_000], 430.0, 450.0);
            assert!((f - 440.0).abs() < 0.3, "waveform {w}: {f} Hz");
        }
        // An octave up is an octave up.
        let mut p = still("Classic");
        p[OCTAVE] = 1.0;
        let mut st = ThereminState::new(SR);
        st.note_on(&p, 57, 1.0);
        let out = run(&mut st, &p, 1.5, |_, _| {});
        assert!((peak(&out[24_000..72_000], 430.0, 450.0) - 440.0).abs() < 0.3);
    }

    #[test]
    fn a_new_note_glides_there_without_starting_again() {
        let p = still("Classic");
        let mut st = ThereminState::new(SR);
        st.note_on(&p, 57, 1.0);
        let mut pitches = Vec::new();
        let mut levels = Vec::new();
        let out = run(&mut st, &p, 1.2, |st, t| {
            if (t - 0.6).abs() < 0.5 / 750.0 {
                st.note_on(&p, 69, 1.0);
            }
            pitches.push(st.pitch);
            levels.push(st.level);
        });
        let from = (0.6 * 750.0) as usize;
        let glide = &pitches[from..];
        // Up all the way, through the notes between, and there in the end.
        assert!(glide.windows(2).all(|w| w[1] >= w[0] - 1e-4), "it only goes up");
        assert!(glide.iter().any(|&p| p > 61.0 && p < 65.0), "through the notes between");
        assert!((glide[glide.len() - 1] - 69.0).abs() < 0.05, "and there: {}", glide[glide.len() - 1]);
        // The volume does not dip: no new attack, no click.
        let before = levels[from - 1];
        assert!(levels[from..].iter().all(|&l| l > before * 0.98), "the volume holds");
        let win = |t: f32| rms(&out[(t * SR) as usize..(t * SR) as usize + 960]);
        assert!(win(0.61) > win(0.55) * 0.8 && win(0.61) < win(0.55) * 1.25, "{} {}", win(0.55), win(0.61));
        let jump = out.windows(2).map(|w| (w[1] - w[0]).abs()).fold(0.0f32, f32::max);
        assert!(jump < 0.1, "no click: {jump}");
    }

    #[test]
    fn the_vibrato_has_its_depth_and_rate_after_its_delay() {
        let mut p = still("Classic");
        (p[VIBRATO], p[RATE], p[DELAY]) = (50.0, 6.0, 1.0);
        let mut st = ThereminState::new(SR);
        st.note_on(&p, 69, 1.0);
        let mut heard = Vec::new();
        run(&mut st, &p, 4.0, |st, _| heard.push(st.shown.0 - 69.0));
        // 750 blocks a second.
        let early = &heard[75..700];
        assert!(early.iter().all(|d| d.abs() < 0.02), "not before the delay");
        let late = &heard[2250..3000];
        let (lo, hi) = late.iter().fold((f32::MAX, f32::MIN), |(a, b), &d| (a.min(d), b.max(d)));
        assert!(((hi - lo) / 2.0 - 0.5).abs() < 0.05, "half a semitone each way: {lo} {hi}");
        let ups = late.windows(2).filter(|w| w[0] < 0.0 && w[1] >= 0.0).count();
        assert!((5..=7).contains(&ups), "six times a second: {ups}");
    }

    #[test]
    fn it_swells_in_over_attack_and_fades_to_silence_over_release() {
        let mut p = still("Classic");
        (p[ATTACK], p[RELEASE]) = (0.5, 0.5);
        let mut st = ThereminState::new(SR);
        st.note_on(&p, 69, 1.0);
        let out = run(&mut st, &p, 3.0, |st, t| {
            if (t - 1.0).abs() < 0.5 / 750.0 {
                st.note_off(&p, 69);
            }
        });
        let win = |t: f32| rms(&out[(t * SR) as usize..(t * SR) as usize + 2400]);
        assert!(win(0.0) < win(0.3) * 0.5 && win(0.3) < win(0.8), "it swells: {} {} {}", win(0.0), win(0.3), win(0.8));
        assert!(win(1.1) < win(0.9) * 0.8 && win(1.1) > win(0.9) * 0.05, "it fades: {} {}", win(0.9), win(1.1));
        assert!(out[(2.2 * SR) as usize..].iter().all(|v| v.abs() < 1e-5), "and is silent");
        assert!(!st.sounding());
    }

    #[test]
    fn snap_pulls_the_hand_onto_the_notes_of_the_key() {
        let major = crate::Scale::new(0, crate::ScaleKind::Major).mask();
        let settle = |amount: f32, key: u16| {
            let mut p = still("Classic");
            p[SNAP] = amount;
            let mut st = ThereminState::new(SR);
            st.key = key;
            run(&mut st, &p, 0.6, |st, _| st.hand(Some((61.4, 0.8))));
            st.pitch
        };
        assert!((settle(0.0, major) - 61.4).abs() < 0.01, "free");
        assert!((settle(1.0, major) - 62.0).abs() < 0.01, "on D, the nearest note of C major");
        assert!((settle(0.5, major) - 61.7).abs() < 0.01, "halfway there");
        assert!((settle(1.0, 0xFFF) - 61.0).abs() < 0.01, "on C sharp with every note");
    }

    #[test]
    fn the_hand_plays_between_the_notes_and_lets_go() {
        let p = still("Classic");
        let mut st = ThereminState::new(SR);
        let out = run(&mut st, &p, 3.0, |st, t| {
            if t < 1.0 {
                // A slow slide up a fifth, the hand high.
                st.hand(Some((60.0 + 7.0 * t, 0.9)));
            }
        });
        assert!(rms(&out[24_000..36_000]) > 0.05, "it sounds");
        // Nothing heard of the hand: it is gone, and the sound fades.
        assert!(st.hand.is_none() && !st.sounding());
        let jump = out.windows(2).map(|w| (w[1] - w[0]).abs()).fold(0.0f32, f32::max);
        assert!(jump < 0.1, "a smooth slide: {jump}");
    }

    #[test]
    fn waveform_brings_in_harmonics_and_brightness_takes_them_out() {
        let harmonics = |w: f32, b: f32| {
            let mut p = still("Classic");
            (p[WAVEFORM], p[BRIGHTNESS], p[FORMANT]) = (w, b, 0.0);
            let mut st = ThereminState::new(SR);
            st.note_on(&p, 57, 1.0);
            let out = run(&mut st, &p, 1.0, |_, _| {});
            let x = &out[24_000..48_000];
            (2..=8).map(|k| power(x, 220.0 * k as f32)).sum::<f32>() / power(x, 220.0)
        };
        let pure = harmonics(0.0, 1.0);
        let reedy = harmonics(1.0, 1.0);
        assert!(pure < 0.02, "nearly a sine: {pure}");
        assert!(reedy > pure * 10.0, "reedy: {pure} {reedy}");
        assert!(harmonics(1.0, 0.0) < reedy * 0.5, "darker: {}", harmonics(1.0, 0.0));
    }

    #[test]
    fn pulling_leans_the_wave_over() {
        // Unpulled the beat is even (the same either side of its middle);
        // pulled, it runs fast and slow in turn.
        let lean = |w: f32| (1..50).map(|k| (wave_at(w, k as f32 / 100.0) - wave_at(w, 1.0 - k as f32 / 100.0)).abs()).fold(0.0f32, f32::max);
        assert!(lean(0.0) < 1e-4, "{}", lean(0.0));
        assert!(lean(1.0) > 0.3, "{}", lean(1.0));
    }

    #[test]
    fn every_preset_sounds_and_stays_in_bounds() {
        for name in preset_names() {
            let p = preset(name);
            let mut st = ThereminState::new(SR);
            st.pitch_bend = 0.5;
            let out = run(&mut st, &p, 2.0, |st, t| {
                if t == 0.0 {
                    st.note_on(&p, 48, 0.8);
                }
                if (t - 0.5).abs() < 0.5 / 750.0 {
                    st.note_on(&p, 84, 1.0);
                }
                if (0.8..1.2).contains(&t) {
                    st.hand(Some((40.0 + 50.0 * t, t - 0.7)));
                }
            });
            assert!(rms(&out) > 1e-3, "{name} sounds");
            assert!(out.iter().all(|v| v.is_finite() && v.abs() < 2.0), "{name} stays in bounds");
        }
    }
}

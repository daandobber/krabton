//! Analog Princess: deep-fried pluggnb drums, synthesized (no samples). The
//! sound of the strange corners of pluggnb and its channels: an 808 put into
//! another 808, clipped to the ceiling as in 2015, crispy ring-modulated
//! hats, and the channel surfing and subliminal layers of that sound design.
//!
//! - The 808s are sines that drop into their note at the hit. A second 808 (a
//!   fifth up, with its own drop and a shorter decay) ring-modulates the
//!   first (Fried 808, Ring), or the first one's output pushes the second's
//!   pitch around (808 Into 808, Feed, then folded back); the Crunch 808 is
//!   crushed to fewer bits at a lower rate (Fry). Tone drives them: a soft
//!   saturation into a hard clip at the ceiling. A new note glides from the
//!   pitch still ringing (Glide); while another of its keys is held it slides
//!   there without a new hit, as pluggnb 808s slide when their notes overlap.
//! - The kick is a sine that falls from high (Punch) into a hard clip (Clip);
//!   the snare two pitched sines and noise, with a clap's bursts layered on
//!   (Clap) and a short bright room (Room); the clap's bursts are
//!   ring-modulated by two tones (Ring) and clipped.
//! - The hats are six squares multiplied in pairs (a ring-modulated metal),
//!   mixed with noise (Ring), high-passed and clipped; harder hits are
//!   brighter and longer. The Hat Roll rolls in the song's time: sixteenths,
//!   triplets, thirty-seconds, or ratcheting up (Roll), for a beat.
//! - Laser: a sine swept down from far up (Sweep), crushed (Fry). Bit Perc: a
//!   rimshot of two pings and a click at fewer bits (Bits) and a lower rate
//!   (Rate).
//! - Channel Surf: bursts of a TV or radio flipping through channels, each a
//!   few tens of milliseconds and cut off abruptly: static in some band, a
//!   test tone, a tuning whistle, mains hum (Surf: how fast it flips; Tones:
//!   how many of them carry a tone rather than static).
//! - Subliminal: a shadow, mostly felt. A detuned low drone that swells up as
//!   a reversed hit does and stops (Swell), and a pitched-down whisper of
//!   noise through two formants (Whisper).
//! - Plugg Bell: the sweet bell that pluggnb drums sit with; a sine phase
//!   modulated at twice its pitch (Shine) and a tine, a second one detuned
//!   with a slow vibrato (Detune).

use super::drums::DrumVoice;
use super::*;

/// An 808's note at Tune 0 (Hz): MIDI note 31's.
pub const BASE_808: f32 = 49.0;
/// Where the clippers stop.
const CEILING: f32 = 0.9;
/// Six squares for the hats (Hz), multiplied in pairs.
const METAL: [f32; 6] = [349.0, 517.5, 628.3, 888.6, 918.0, 1360.0];
/// The room's taps (samples at 48 kHz) and the longest it can hold.
const ROOM_TAPS: [usize; 4] = [431, 613, 797, 1019];
const ROOM_LEN: usize = 2048;

/// An Analog Princess voice while it sounds.
pub struct PrincessVoice {
    sr: f32,
    t: u32,
    vel: f32,
    tone: f32,
    a: f32,
    b: f32,
    active: bool,
    /// Samples it has been silent.
    quiet: u32,
    phase: [f32; 6],
    /// Envelopes and their per-sample decays (what each is depends on the voice).
    env: [f32; 4],
    coef: [f32; 4],
    /// Subliminal: the swell still rising, and the decay after it.
    rising: bool,
    fall: f32,
    /// The 808's pitch (log2 of Hz), where it glides to, the glide's step a
    /// sample and the samples it has left.
    pitch: f32,
    target: f32,
    glide_step: f32,
    glide_left: u32,
    /// Base frequencies (Hz).
    freq: [f32; 2],
    /// Drops in pitch at the hit (fractions above the note) and their decays.
    bend: [f32; 2],
    bend_coef: [f32; 2],
    filt: [Svf; 4],
    lp: OnePole,
    hp: OnePole,
    /// The crusher: its steps, its rate (a fraction of the sample rate) and
    /// the sample held between its samples.
    steps: f32,
    rate: f32,
    held: f32,
    hold_ph: f32,
    /// Hat Roll: samples to the next hit, the gap after it, how the gap
    /// shrinks, the hits left of all of them.
    next_hit: u32,
    gap: f32,
    gap_mul: f32,
    hits_left: u32,
    hits: u32,
    hit_level: f32,
    /// Channel Surf: samples left of this channel and the hold before the
    /// end; what the channel is, its tone's pitch and its level.
    chan_left: u32,
    hold: u32,
    chan_kind: u8,
    chan_hz: f32,
    chan_sweep: f32,
    chan_level: f32,
    /// The snare's room.
    room: [f32; ROOM_LEN],
    room_pos: usize,
    taps: [usize; 4],
    /// The song's tempo (the Hat Roll keeps it).
    pub bpm: f32,
    /// How far the drop in pitch at the hit goes and how long it takes, as
    /// multiples of the voice's own (1: as designed; the machine sets them).
    pub pitch_amt: f32,
    pub pitch_time: f32,
}

impl Default for PrincessVoice {
    fn default() -> Self {
        Self {
            sr: 48000.0,
            t: 0,
            vel: 1.0,
            tone: 0.5,
            a: 0.5,
            b: 0.5,
            active: false,
            quiet: 0,
            phase: [0.0; 6],
            env: [0.0; 4],
            coef: [0.0; 4],
            rising: false,
            fall: 0.0,
            pitch: BASE_808.log2(),
            target: BASE_808.log2(),
            glide_step: 0.0,
            glide_left: 0,
            freq: [0.0; 2],
            bend: [0.0; 2],
            bend_coef: [0.0; 2],
            filt: [Svf::new(); 4],
            lp: OnePole::default(),
            hp: OnePole::default(),
            steps: 32768.0,
            rate: 1.0,
            held: 0.0,
            hold_ph: 0.0,
            next_hit: 0,
            gap: 0.0,
            gap_mul: 1.0,
            hits_left: 0,
            hits: 0,
            hit_level: 1.0,
            chan_left: 0,
            hold: 0,
            chan_kind: 0,
            chan_hz: 440.0,
            chan_sweep: 1.0,
            chan_level: 1.0,
            room: [0.0; ROOM_LEN],
            room_pos: 0,
            taps: ROOM_TAPS,
            bpm: 120.0,
            pitch_amt: 1.0,
            pitch_time: 1.0,
        }
    }
}

#[inline]
fn sine(p: f32) -> f32 {
    (p * TAU).sin()
}

#[inline]
fn square(p: f32) -> f32 {
    if p < 0.5 {
        1.0
    } else {
        -1.0
    }
}

/// The chain of a 2015 808: a soft saturation into a hard clip at the
/// ceiling, both harder as `drive` (0..1) goes up.
#[inline]
pub fn fry(x: f32, drive: f32) -> f32 {
    let soft = fast_tanh(x * (1.0 + 2.0 * drive));
    (soft * (1.0 + 11.0 * drive * drive)).clamp(-CEILING, CEILING)
}

/// A triangle fold: past ±1 the wave turns back.
#[inline]
fn fold(x: f32) -> f32 {
    let t = (x + 1.0) * 0.25;
    1.0 - 4.0 * (t - t.floor() - 0.5).abs()
}

/// How long a glide takes (s) at Glide `a`; none at 0.
pub fn glide_secs(a: f32) -> f32 {
    if a < 0.01 {
        0.0
    } else {
        0.02 + 0.6 * a * a
    }
}

/// The 808s, which take notes.
pub fn is_808(voice: DrumVoice) -> bool {
    matches!(voice, DrumVoice::ApFried808 | DrumVoice::ApDouble808 | DrumVoice::ApCrunch808)
}

impl PrincessVoice {
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Advances oscillator `k` at `f` Hz and returns its phase.
    #[inline]
    fn adv(&mut self, k: usize, f: f32) -> f32 {
        let p = self.phase[k] + f / self.sr;
        self.phase[k] = p - p.floor();
        self.phase[k]
    }

    /// The crusher: fewer bits, held at a lower rate.
    #[inline]
    fn crush(&mut self, x: f32) -> f32 {
        self.hold_ph += self.rate;
        if self.hold_ph >= 1.0 {
            self.hold_ph -= 1.0;
            self.held = (x * self.steps).round() / self.steps;
        }
        self.held
    }

    /// Sets the crusher to `bits` at `rate` (a fraction of the sample rate).
    fn set_crush(&mut self, bits: f32, rate: f32) {
        self.steps = 2f32.powf(bits.clamp(1.0, 16.0) - 1.0);
        self.rate = rate.clamp(0.01, 1.0);
        self.held = 0.0;
        self.hold_ph = 0.0;
    }

    /// A short, bright room: one line read at four taps and fed back.
    #[inline]
    fn room(&mut self, x: f32) -> f32 {
        let mut wet = 0.0;
        for &tap in &self.taps {
            wet += self.room[(self.room_pos + ROOM_LEN - tap) % ROOM_LEN];
        }
        wet *= 0.25;
        self.room[self.room_pos] = x + wet * 0.62;
        self.room_pos = (self.room_pos + 1) % ROOM_LEN;
        self.hp.hp(wet)
    }

    /// A clap's bursts, `n` of them `gap` seconds apart, then its tail (env 2).
    #[inline]
    fn clap_env(&self, ts: f32, n: f32, gap: f32) -> f32 {
        let burst = if ts < gap * n { (-(ts % gap) / 0.0022).exp() } else { 0.0 };
        burst.max(self.env[2] * 0.6)
    }

    /// Flips to another channel.
    fn new_channel(&mut self, rng: &mut Rng) {
        let sr = self.sr;
        // Faster with Surf: 20 to 140 ms, down to a few at the top.
        let secs = (0.02 + 0.12 * rng.unipolar()) * (1.5 - 1.25 * self.a);
        self.chan_left = (secs.max(0.008) * sr) as u32;
        // Tones: a test tone or a whistle; else static, now and then hum.
        self.chan_kind = if rng.unipolar() < 0.15 + 0.7 * self.b {
            if rng.unipolar() < 0.5 {
                1
            } else {
                2
            }
        } else if self.chan_kind != 3 && rng.unipolar() < 0.2 {
            3
        } else {
            0
        };
        // Static in a band somewhere between 250 Hz and 9 kHz, as loud wherever it is.
        let fc = 250.0 * 36f32.powf(rng.unipolar());
        let q = 0.8 + 2.5 * rng.unipolar();
        self.filt[0].set_q(fc, q, sr);
        let width = (fc / q / (0.5 * sr)).max(1e-4);
        self.chan_level = (0.45 / width.sqrt()).min(5.0) / q;
        self.chan_hz = 180.0 * 12f32.powf(rng.unipolar());
        // A tuning whistle slides up or down an octave or two a second.
        self.chan_sweep = 2f32.powf((rng.bipolar() * 2.5) / sr);
        self.filt[1].set_q(self.chan_hz * 1.3, 1.0, sr);
    }

    /// Hits `voice`. `tune` is a frequency ratio; `key_hz` the key's pitch
    /// when an 808 is played from the keys; `legato` that another of its keys
    /// is held (it slides there without a new hit). `decay` stretches the
    /// decay (1 = as designed), `tone`, `a` and `b` are the pad's knobs.
    #[allow(clippy::too_many_arguments)]
    pub fn trigger(&mut self, voice: DrumVoice, vel: f32, tune: f32, key_hz: Option<f32>, legato: bool, decay: f32, tone: f32, a: f32, b: f32, sr: f32) {
        self.hit(voice, vel, tune, key_hz, legato, decay, tone, a, b, sr);
        // A new hit (not a slide): its drop in pitch as the machine shapes it.
        if self.t == 0 && (self.pitch_amt != 1.0 || self.pitch_time != 1.0) {
            let time = self.pitch_time.max(0.05);
            for k in 0..2 {
                self.bend[k] *= self.pitch_amt.max(0.0);
                self.bend_coef[k] = self.bend_coef[k].powf(1.0 / time);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn hit(&mut self, voice: DrumVoice, vel: f32, tune: f32, key_hz: Option<f32>, legato: bool, decay: f32, tone: f32, a: f32, b: f32, sr: f32) {
        use DrumVoice::*;
        let ringing = self.active && self.sr == sr;
        self.sr = sr;
        self.vel = vel.clamp(0.0, 1.0);
        self.tone = tone.clamp(0.0, 1.0);
        self.a = a.clamp(0.0, 1.0);
        self.b = b.clamp(0.0, 1.0);
        self.active = true;
        self.quiet = 0;
        self.rising = false;
        self.hits_left = 0;
        let d = decay.max(0.05);
        let dc = |s: f32| decay_coef(s, sr);
        if is_808(voice) {
            let to = (key_hz.unwrap_or(BASE_808) * tune).max(1.0).log2();
            let glide = glide_secs(self.a);
            if ringing && glide > 0.0 {
                let n = (glide * sr).max(1.0);
                self.glide_step = (to - self.pitch) / n;
                self.glide_left = n as u32;
            } else {
                self.pitch = to;
                self.glide_left = 0;
            }
            self.target = to;
            if legato && ringing {
                // A slide: no new hit, it only comes up a little.
                self.env[0] = self.env[0].max(0.6);
                self.env[1] = self.env[1].max(0.4);
                return;
            }
            self.t = 0;
            if !ringing {
                self.phase = [0.0; 6];
            }
            self.env = [1.0, 1.0, 1.0, 0.0];
            self.coef = [dc(1.4 * d), dc(0.6 * d), dc(0.004), 0.0];
            // Both drop into their notes, the second more slowly.
            self.bend = [1.2, 0.6];
            self.bend_coef = [dc(0.03), dc(0.06)];
            self.lp.set(1500.0 + 4000.0 * self.tone, sr);
            self.lp.z = 0.0;
            if voice == ApCrunch808 {
                self.set_crush(16.0 - 13.0 * self.b, 2f32.powf(-5.0 * self.b));
            }
            return;
        }
        self.t = 0;
        self.phase = [0.0, 0.13, 0.31, 0.47, 0.61, 0.83];
        self.env = [1.0; 4];
        self.coef = [0.0; 4];
        self.bend = [0.0; 2];
        self.bend_coef = [0.0; 2];
        self.hits = 0;
        self.hit_level = 1.0;
        for f in &mut self.filt {
            f.reset();
        }
        self.lp.z = 0.0;
        self.hp.z = 0.0;
        self.set_crush(16.0, 1.0);
        let tone = self.tone;
        match voice {
            ApKick => {
                self.freq = [52.0 * tune, 0.0];
                // Punch: how far up it starts.
                self.bend = [1.0 + 5.0 * self.a, 0.0];
                self.bend_coef = [dc(0.045), 0.0];
                self.coef = [dc(0.5 * d), 0.0, dc(0.004), 0.0];
                self.filt[0].set_q(1500.0 + 7000.0 * tone, 0.7, sr);
            }
            ApSnare => {
                self.freq = [210.0 * tune, 345.0 * tune];
                self.bend = [0.35, 0.0];
                self.bend_coef = [dc(0.02), 0.0];
                // The noise, the body, the clap's tail.
                self.coef = [dc(0.22 * d), dc(0.12 * d), dc(0.18 * d), 0.0];
                self.filt[0].set_q(1800.0 + 4000.0 * tone, 0.7, sr);
                self.filt[1].set_q(1300.0 * tune, 1.6, sr);
                self.room.fill(0.0);
                self.room_pos = 0;
                let k = sr / 48000.0;
                for (t, &base) in self.taps.iter_mut().zip(ROOM_TAPS.iter()) {
                    *t = ((base as f32 * k) as usize).clamp(1, ROOM_LEN - 1);
                }
                self.hp.set(500.0, sr);
            }
            ApClap => {
                self.freq = [560.0 * tune, 873.0 * tune];
                self.coef = [0.0, 0.0, dc(0.3 * d), 0.0];
                self.filt[0].set_q(1150.0 * tune * (0.8 + 0.5 * tone), 1.8, sr);
            }
            ApHat | ApOpenHat | ApHatRoll => {
                // Harder hits brighter and longer.
                let len = match voice {
                    ApOpenHat => 0.5,
                    ApHatRoll => 0.04,
                    _ => 0.06,
                } * d
                    * (0.6 + 0.6 * self.vel);
                let bright = (5500.0 + 4000.0 * tone) * (0.75 + 0.45 * self.vel);
                self.filt[0].set_q(bright * tune.sqrt(), 0.8, sr);
                self.filt[1].set_q(9500.0 * tune.sqrt(), 1.4, sr);
                self.coef = [dc(len), 0.0, 0.0, 0.0];
                // (the metal's tuning, as a ratio)
                self.freq = [tune, 0.0];
                if voice == ApHatRoll {
                    // Roll: sixteenths, triplets, thirty-seconds, or ratcheting up, for a beat.
                    let beat = 60.0 / self.bpm.clamp(20.0, 400.0);
                    let (gap, mul) = match (self.b * 3.999) as u32 {
                        0 => (beat / 4.0, 1.0),
                        1 => (beat / 6.0, 1.0),
                        2 => (beat / 8.0, 1.0),
                        _ => (beat / 4.0, 0.72),
                    };
                    let mut n = 0;
                    let (mut at, mut g) = (0.0, gap);
                    while at + 0.5 * g < beat * d.min(4.0) && n < 64 {
                        at += g;
                        g = (g * mul).max(beat / 16.0);
                        n += 1;
                    }
                    self.hits = n;
                    self.hits_left = n.saturating_sub(1);
                    self.gap = gap * sr;
                    self.gap_mul = mul;
                    self.next_hit = self.gap as u32;
                    self.hit_level = 0.55;
                }
            }
            ApLaser => {
                self.freq = [95.0 * tune, 0.0];
                // Sweep: from further up, more slowly.
                self.bend = [8.0 + 40.0 * self.a, 0.0];
                self.bend_coef = [dc(0.04 + 0.25 * self.a), 0.0];
                self.coef = [dc(0.3 * d), 0.0, 0.0, 0.0];
                self.set_crush(14.0 - 11.0 * self.b, 2f32.powf(-4.0 * self.b));
            }
            ApBitPerc => {
                self.freq = [1650.0 * tune, 830.0 * tune];
                self.coef = [dc(0.07 * d), dc(0.035 * d), dc(0.003), 0.0];
                self.filt[0].set_q(4000.0, 0.7, sr);
                self.set_crush(10.0 - 8.0 * self.a, 2f32.powf(-4.0 * self.b));
            }
            ApChannelSurf => {
                // On for a while, then cut.
                self.hold = (0.7 * d * sr) as u32;
                self.coef = [1.0, 0.0, 0.0, 0.0];
                self.chan_left = 0;
                self.chan_kind = 0;
                self.lp.set(2500.0 + 14000.0 * tone, sr);
                self.hp.set(120.0, sr);
            }
            ApSubliminal => {
                self.freq = [55.0 * tune, 55.0 * tune * 1.013];
                // Swell: rises as a reversed hit, then stops.
                let swell = (0.12 + 1.1 * self.a) * d;
                self.env = [0.02, 0.0, 0.0, 0.0];
                self.coef = [(1.0f32 / 0.02).powf(1.0 / (swell * sr)), 0.0, 0.0, 0.0];
                self.rising = true;
                self.fall = dc(0.12 * d);
                self.filt[0].set_q(220.0 + 700.0 * tone, 0.7, sr);
                // The whisper, pitched down: two low formants.
                self.filt[1].set_q(520.0 * tune.sqrt(), 3.0, sr);
                self.filt[2].set_q(980.0 * tune.sqrt(), 3.0, sr);
            }
            ApBell => {
                self.freq = [523.25 * tune, 0.0];
                // Its ring, its shine, its tine.
                self.coef = [dc(1.1 * d), dc(0.35 * d), dc(0.04), 0.0];
            }
            _ => self.active = false,
        }
    }

    #[inline]
    pub fn tick(&mut self, voice: DrumVoice, rng: &mut Rng) -> f32 {
        use DrumVoice::*;
        let sr = self.sr;
        let ts = self.t as f32 / sr;
        let (tone, a, b) = (self.tone, self.a, self.b);
        let y = match voice {
            ApFried808 | ApDouble808 | ApCrunch808 => {
                if self.glide_left > 0 {
                    self.pitch += self.glide_step;
                    self.glide_left -= 1;
                    if self.glide_left == 0 {
                        self.pitch = self.target;
                    }
                }
                let f = self.pitch.exp2();
                let s1 = sine(self.adv(0, f * (1.0 + self.bend[0])));
                let click = self.lp.lp(rng.bipolar()) * self.env[2] * (0.2 + 0.6 * tone);
                match voice {
                    ApFried808 => {
                        // Ring: the second 808, a fifth up, multiplies the first.
                        let s2 = sine(self.adv(1, f * 1.5 * (1.0 + self.bend[1]))) * self.env[1];
                        let x = s1 * (1.0 - 0.6 * b) + s1 * s2 * 1.6 * b;
                        fry(x * self.env[0] + click, tone)
                    }
                    ApDouble808 => {
                        // Feed: the first 808's output pushes the second's pitch around.
                        let out1 = s1 * self.env[0];
                        let f2 = f * 1.5 * (1.0 + self.bend[1]) * (1.0 + 3.0 * b * out1);
                        let s2 = sine(self.adv(1, f2)) * self.env[1];
                        let x = out1 * (1.0 - 0.5 * b) + s2 * 0.6 * b + out1 * s2 * b + click;
                        fast_tanh(fold(x * (1.0 + 5.0 * tone)) * 1.3) * CEILING
                    }
                    _ => {
                        let x = fry(s1 * self.env[0] + click, tone);
                        self.crush(x)
                    }
                }
            }
            ApKick => {
                let f = self.freq[0] * (1.0 + self.bend[0]);
                let body = sine(self.adv(0, f)) * self.env[0];
                let click = self.filt[0].process(rng.bipolar(), FilterMode::Low) * self.env[2] * (0.4 + 0.8 * tone);
                fry(body * 1.2 + click, b)
            }
            ApSnare => {
                let bendf = 1.0 + self.bend[0];
                let (f0, f1) = (self.freq[0] * bendf, self.freq[1] * bendf);
                let body = (sine(self.adv(0, f0)) * 0.6 + sine(self.adv(1, f1)) * 0.4) * self.env[1];
                let n = rng.bipolar();
                let snare = self.filt[0].process(n, FilterMode::High) * self.env[0];
                let clap = self.filt[1].process(n, FilterMode::Band) * self.clap_env(ts, 3.0, 0.009);
                let dry = body * 0.75 + snare * 0.8 + clap * a * 2.2;
                let wet = self.room(dry);
                fast_tanh((dry + wet * b * 2.0) * 1.3) * 0.95
            }
            ApClap => {
                let n = self.filt[0].process(rng.bipolar(), FilterMode::Band) * self.clap_env(ts, 4.0, 0.0085) * 2.4;
                let (f0, f1) = (self.freq[0], self.freq[1]);
                let carrier = sine(self.adv(0, f0)) * 0.6 + square(self.adv(1, f1)) * 0.4;
                let x = n * (1.0 - a) + n * carrier * a * 1.8;
                fry(x, 0.15 + 0.85 * b)
            }
            ApHat | ApOpenHat | ApHatRoll => {
                if self.hits_left > 0 {
                    if self.next_hit == 0 {
                        // The next hit of the roll, a little louder each time.
                        self.hits_left -= 1;
                        let k = (self.hits - self.hits_left) as f32 / self.hits.max(1) as f32;
                        self.hit_level = 0.55 + 0.45 * k;
                        self.env[0] = 1.0;
                        self.gap = (self.gap * self.gap_mul).max(60.0 / self.bpm.clamp(20.0, 400.0) / 16.0 * sr);
                        self.next_hit = self.gap as u32;
                    } else {
                        self.next_hit -= 1;
                    }
                }
                let tune = self.freq[0];
                let mut m = 0.0;
                for k in 0..3 {
                    let p = self.adv(2 * k, METAL[2 * k] * tune);
                    let q = self.adv(2 * k + 1, METAL[2 * k + 1] * tune);
                    m += square(p) * square(q);
                }
                let x = m / 3.0 * a + rng.bipolar() * (1.0 - a);
                let y = self.filt[0].process(x, FilterMode::High) + self.filt[1].process(x, FilterMode::Band) * 0.4;
                let clip = if voice == ApHatRoll { 0.5 } else { b };
                fast_tanh(y * self.env[0] * self.hit_level * (2.0 + 6.0 * clip)) * 0.85
            }
            ApLaser => {
                let f = self.freq[0] * (1.0 + self.bend[0]);
                let p = self.adv(0, f);
                let x = (sine(p) * 0.8 + square(p) * 0.2) * self.env[0];
                let x = fast_tanh(x * 1.6);
                self.crush(x)
            }
            ApBitPerc => {
                let (f0, f1) = (self.freq[0], self.freq[1]);
                let ping = sine(self.adv(0, f0)) * self.env[0] * 0.7 + sine(self.adv(1, f1)) * self.env[1] * 0.6;
                let click = self.filt[0].process(rng.bipolar(), FilterMode::High) * self.env[2] * (0.5 + tone);
                let x = fast_tanh((ping + click) * 1.5);
                self.crush(x)
            }
            ApChannelSurf => {
                if self.chan_left == 0 {
                    self.new_channel(rng);
                }
                self.chan_left -= 1;
                if self.hold > 0 {
                    self.hold -= 1;
                    if self.hold == 0 {
                        self.coef[0] = decay_coef(0.03, sr);
                    }
                }
                let n = rng.bipolar();
                let x = match self.chan_kind {
                    // Static.
                    0 => self.filt[0].process(n, FilterMode::Band) * self.chan_level,
                    // A test tone, wobbling, under a little hiss.
                    1 => {
                        let hz = self.chan_hz;
                        sine(self.adv(0, hz)) * (0.6 + 0.4 * sine(self.adv(1, 7.0))) * 0.8 + n * 0.12
                    }
                    // A tuning whistle sliding, in hiss.
                    2 => {
                        self.chan_hz = (self.chan_hz * self.chan_sweep).clamp(100.0, 6000.0);
                        let hz = self.chan_hz;
                        let w = sine(self.adv(0, hz)) * 0.6;
                        w + self.filt[1].process(n, FilterMode::Band) * 0.5
                    }
                    // Mains hum and crackle.
                    _ => {
                        let hum = square(self.adv(0, 60.0)) * 0.3 + sine(self.adv(1, 180.0)) * 0.3;
                        let crackle = if rng.unipolar() < 0.003 { rng.bipolar() * 2.0 } else { 0.0 };
                        hum + crackle
                    }
                };
                let y = self.lp.lp(x);
                fast_tanh(self.hp.hp(y) * 1.4) * self.env[0]
            }
            ApSubliminal => {
                if self.rising && self.env[0] >= 1.0 {
                    self.env[0] = 1.0;
                    self.coef[0] = self.fall;
                    self.rising = false;
                }
                let (f0, f1) = (self.freq[0], self.freq[1]);
                let s = sine(self.adv(0, f0)) + sine(self.adv(1, f1)) + 0.6 * sine(self.adv(2, f0 * 0.5));
                let drone = self.filt[0].process(fast_tanh(s * 0.9), FilterMode::Low);
                let n = rng.bipolar();
                let syllables = 0.55 + 0.45 * sine(self.adv(3, 4.7));
                let whisper = (self.filt[1].process(n, FilterMode::Band) + self.filt[2].process(n, FilterMode::Band) * 0.7) * syllables * 1.6;
                fast_tanh((drone * (1.0 - b) + whisper * b) * self.env[0] * 1.1) * 0.9
            }
            ApBell => {
                let f = self.freq[0] * (1.0 + 0.004 * b * sine(self.adv(4, 5.5)));
                let attack = (ts / 0.003).min(1.0);
                let m = sine(self.adv(1, f * 2.0)) * (0.3 + 2.7 * a) * self.env[1];
                let c1 = (self.adv(0, f) * TAU + m).sin();
                let c2 = (self.adv(3, f * (1.0 + 0.012 * b)) * TAU + m).sin();
                let tine = sine(self.adv(2, f * 7.03)) * self.env[2] * 0.3 * (0.4 + a);
                let x = (c1 * (1.0 - 0.45 * b) + c2 * 0.45 * b) * self.env[0] * attack + tine;
                fast_tanh(x * 1.2) * 0.85
            }
            _ => 0.0,
        };
        for (e, c) in self.env.iter_mut().zip(self.coef) {
            *e *= c;
        }
        self.bend[0] *= self.bend_coef[0];
        self.bend[1] *= self.bend_coef[1];
        self.t = self.t.saturating_add(1);
        // Done once it has been quiet for a while (the room and the clippers
        // ring on under the envelopes); a roll waits for its last hit.
        if self.t as f32 > 0.02 * sr && y.abs() < 2e-4 && self.hits_left == 0 && !self.rising {
            self.quiet += 1;
        } else {
            self.quiet = 0;
        }
        if self.quiet as f32 > 0.05 * sr || !y.is_finite() {
            self.active = false;
        }
        y
    }
}

/// How the machine's 808 is distorted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dist {
    /// A round saturation, never quite square.
    Soft,
    /// Soft into a hard clip at the ceiling (the 2015 808).
    Hard,
    /// Folded back on itself past the ceiling (808 into 808).
    Fold,
    /// Lopsided, as a valve: the even harmonics come up.
    Tube,
}

impl Dist {
    pub fn from_index(i: f32) -> Self {
        match i.round() as i32 {
            0 => Dist::Soft,
            2 => Dist::Fold,
            3 => Dist::Tube,
            _ => Dist::Hard,
        }
    }
}

/// The machine's 808 distortions: `x` driven by `drive` (0..1) into one of
/// them, never over the ceiling. `dc` keeps the Tube's offset (it follows
/// the sound at 12 Hz).
#[inline]
pub fn shape(x: f32, drive: f32, dist: Dist, dc: &mut OnePole) -> f32 {
    match dist {
        Dist::Soft => fast_tanh(x * (1.0 + 4.0 * drive)) * CEILING,
        Dist::Hard => fry(x, drive),
        Dist::Fold => fast_tanh(fold(x * (1.0 + 5.0 * drive)) * 1.3) * CEILING,
        Dist::Tube => {
            let t = x * (1.0 + 4.0 * drive);
            let y = fast_tanh(t + 0.45 * (0.3 + drive) * t * t);
            // (the lopsided part's offset, taken out)
            ((y - dc.lp(y)) * CEILING).clamp(-CEILING, CEILING)
        }
    }
}

/// The knobs of the machine's 808 at a hit.
#[derive(Clone, Copy, Debug)]
pub struct Knobs808 {
    /// How hard it is driven into its distortion (0..1).
    pub drive: f32,
    pub dist: Dist,
    /// The second 808 multiplying the first (0..1), and how far the first
    /// pushes the second one's pitch around (Feed, 0..1).
    pub ring: f32,
    pub feed: f32,
    /// The second 808's pitch above the first (semitones) and its decay (a
    /// multiple of the first's).
    pub ring_semis: f32,
    pub ring_decay: f32,
    /// The decay (a multiple of the designed one).
    pub decay: f32,
    /// How long a slide takes (s).
    pub glide: f32,
    /// A sine an octave down (0..1) and the click at the hit (0..1).
    pub sub: f32,
    pub click: f32,
    /// The drop in pitch at the hit: how far and how long (multiples).
    pub pitch_amt: f32,
    pub pitch_time: f32,
}

impl Default for Knobs808 {
    fn default() -> Self {
        Self { drive: 0.5, dist: Dist::Hard, ring: 0.3, feed: 0.0, ring_semis: 7.0, ring_decay: 1.0, decay: 1.0, glide: 0.12, sub: 0.0, click: 0.5, pitch_amt: 1.0, pitch_time: 1.0 }
    }
}

/// The machine's 808: one voice with all of Analog Princess's 808s in it.
/// A sine dropping into its note, a second 808 above it that rings it
/// (Ring) or is pushed around by it (Feed), a sub an octave down, a click,
/// then one of four distortions. A slide glides to the next note without a
/// new hit.
pub struct Princess808 {
    sr: f32,
    active: bool,
    quiet: u32,
    t: u32,
    phase: [f32; 3],
    /// The first 808, the second, the click.
    env: [f32; 3],
    coef: [f32; 3],
    bend: [f32; 2],
    bend_coef: [f32; 2],
    pitch: f32,
    target: f32,
    glide_step: f32,
    glide_left: u32,
    lp: OnePole,
    dc: OnePole,
    k: Knobs808,
    ratio: f32,
}

impl Default for Princess808 {
    fn default() -> Self {
        Self {
            sr: 48000.0,
            active: false,
            quiet: 0,
            t: 0,
            phase: [0.0; 3],
            env: [0.0; 3],
            coef: [0.0; 3],
            bend: [0.0; 2],
            bend_coef: [0.0; 2],
            pitch: BASE_808.log2(),
            target: BASE_808.log2(),
            glide_step: 0.0,
            glide_left: 0,
            lp: OnePole::default(),
            dc: OnePole::default(),
            k: Knobs808::default(),
            ratio: 1.5,
        }
    }
}

impl Princess808 {
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Its pitch now (Hz).
    pub fn hz(&self) -> f32 {
        self.pitch.exp2()
    }

    /// Plays `hz`. With `slide` (and while it still rings) it glides there
    /// in `k.glide` without a new hit.
    pub fn trigger(&mut self, hz: f32, vel: f32, slide: bool, k: Knobs808, sr: f32) {
        let ringing = self.active && self.sr == sr;
        self.sr = sr;
        self.k = k;
        self.ratio = 2f32.powf(k.ring_semis / 12.0);
        let to = hz.max(1.0).log2();
        self.target = to;
        if slide && ringing {
            let n = (k.glide.max(0.005) * sr).max(1.0);
            self.glide_step = (to - self.pitch) / n;
            self.glide_left = n as u32;
            // No new hit: it only comes up a little.
            self.env[0] = self.env[0].max(0.6 * vel.clamp(0.3, 1.0));
            self.env[1] = self.env[1].max(0.4);
            return;
        }
        self.pitch = to;
        self.glide_left = 0;
        self.active = true;
        self.quiet = 0;
        self.t = 0;
        if !ringing {
            self.phase = [0.0; 3];
        }
        let d = k.decay.max(0.05);
        let dc = |s: f32| decay_coef(s, sr);
        self.env = [1.0, 1.0, 1.0];
        self.coef = [dc(1.4 * d), dc(0.6 * d * k.ring_decay.max(0.05)), dc(0.004)];
        // Both drop into their notes, the second more slowly.
        let time = k.pitch_time.max(0.05);
        self.bend = [1.2 * k.pitch_amt.max(0.0), 0.6 * k.pitch_amt.max(0.0)];
        self.bend_coef = [dc(0.03 * time), dc(0.06 * time)];
        self.lp.set(1500.0 + 4000.0 * k.drive, sr);
        self.lp.z = 0.0;
        self.dc.set(12.0, sr);
    }

    #[inline]
    fn adv(&mut self, i: usize, f: f32) -> f32 {
        let p = self.phase[i] + f / self.sr;
        self.phase[i] = p - p.floor();
        self.phase[i]
    }

    #[inline]
    pub fn tick(&mut self, rng: &mut Rng) -> f32 {
        if !self.active {
            return 0.0;
        }
        let k = self.k;
        if self.glide_left > 0 {
            self.pitch += self.glide_step;
            self.glide_left -= 1;
            if self.glide_left == 0 {
                self.pitch = self.target;
            }
        }
        let f = self.pitch.exp2();
        let out1 = sine(self.adv(0, f * (1.0 + self.bend[0]))) * self.env[0];
        // Feed: the first 808 pushes the second's pitch around.
        let f2 = f * self.ratio * (1.0 + self.bend[1]) * (1.0 + 3.0 * k.feed * out1);
        let s2 = sine(self.adv(1, f2)) * self.env[1];
        let sub = sine(self.adv(2, f * 0.5)) * self.env[0] * k.sub * 0.8;
        let click = self.lp.lp(rng.bipolar()) * self.env[2] * k.click * 1.2;
        let x = out1 * (1.0 - 0.6 * k.ring - 0.3 * k.feed) + out1 * s2 * 1.6 * k.ring + s2 * 0.6 * k.feed * self.env[0] + sub + click;
        let y = shape(x, k.drive, k.dist, &mut self.dc);
        for (e, c) in self.env.iter_mut().zip(self.coef) {
            *e *= c;
        }
        self.bend[0] *= self.bend_coef[0];
        self.bend[1] *= self.bend_coef[1];
        self.t = self.t.saturating_add(1);
        if self.t as f32 > 0.02 * self.sr && y.abs() < 2e-4 {
            self.quiet += 1;
        } else {
            self.quiet = 0;
        }
        if self.quiet as f32 > 0.05 * self.sr || !y.is_finite() {
            self.active = false;
            return 0.0;
        }
        y
    }

    pub fn stop(&mut self) {
        self.active = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 48000.0;

    /// Plays `voice` with its knobs (`tone`, `a`, `b`) for `secs`.
    fn play(voice: DrumVoice, tone: f32, a: f32, b: f32, secs: f32) -> (Vec<f32>, bool) {
        let mut v = PrincessVoice::default();
        let mut rng = Rng::new(7);
        v.trigger(voice, 1.0, 1.0, None, false, 1.0, tone, a, b, SR);
        let x = (0..(secs * SR) as usize).map(|_| v.tick(voice, &mut rng)).collect();
        (x, v.is_active())
    }

    fn at(x: &[f32], a: f32, b: f32) -> &[f32] {
        &x[(a * SR) as usize..((b * SR) as usize).min(x.len())]
    }

    fn peak(x: &[f32]) -> f32 {
        x.iter().fold(0.0f32, |m, v| m.max(v.abs()))
    }

    fn rms(x: &[f32]) -> f32 {
        (x.iter().map(|v| v * v).sum::<f32>() / x.len().max(1) as f32).sqrt()
    }

    /// Frequency from upward zero crossings.
    fn pitch(x: &[f32]) -> f32 {
        let ups: Vec<f32> = x.windows(2).enumerate().filter(|(_, w)| w[0] <= 0.0 && w[1] > 0.0).map(|(i, w)| i as f32 + w[0] / (w[0] - w[1])).collect();
        if ups.len() < 2 {
            return 0.0;
        }
        (ups.len() - 1) as f32 * SR / (ups[ups.len() - 1] - ups[0])
    }

    /// Magnitude at `hz` (Goertzel, through a Hann window), over `x`.
    fn magnitude(x: &[f32], hz: f32) -> f32 {
        let w = TAU * hz / SR;
        let n = x.len().max(2) as f32 - 1.0;
        let (mut s1, mut s2) = (0.0f32, 0.0f32);
        for (i, &v) in x.iter().enumerate() {
            let v = v * (0.5 - 0.5 * (TAU * i as f32 / n).cos());
            let s0 = v + 2.0 * w.cos() * s1 - s2;
            s2 = s1;
            s1 = s0;
        }
        (s1 * s1 + s2 * s2 - 2.0 * w.cos() * s1 * s2).sqrt() / x.len() as f32
    }

    /// Brightness (0..1): the energy a first difference passes, against the whole.
    fn bright(x: &[f32]) -> f32 {
        let d: f32 = x.windows(2).map(|w| (w[1] - w[0]).powi(2)).sum();
        let e: f32 = x.iter().map(|v| v * v).sum();
        d / (4.0 * e.max(1e-12))
    }

    const ALL: [DrumVoice; 14] = [
        DrumVoice::ApFried808,
        DrumVoice::ApDouble808,
        DrumVoice::ApCrunch808,
        DrumVoice::ApKick,
        DrumVoice::ApSnare,
        DrumVoice::ApClap,
        DrumVoice::ApHat,
        DrumVoice::ApOpenHat,
        DrumVoice::ApHatRoll,
        DrumVoice::ApLaser,
        DrumVoice::ApBitPerc,
        DrumVoice::ApChannelSurf,
        DrumVoice::ApSubliminal,
        DrumVoice::ApBell,
    ];

    #[test]
    fn every_voice_plays_and_ends() {
        for voice in ALL {
            for (tone, a, b) in [(0.5, 0.5, 0.5), (0.0, 0.0, 0.0), (1.0, 1.0, 1.0)] {
                let (x, still) = play(voice, tone, a, b, 10.0);
                let p = peak(&x);
                assert!(x.iter().all(|s| s.is_finite()), "{voice:?}");
                assert!(p > 0.2 && p <= 1.0, "{voice:?} at {tone} {a} {b}: peak {p}");
                assert!(!still, "{voice:?} at {tone} {a} {b} ends");
            }
        }
    }

    #[test]
    fn the_808_glides_to_a_new_note_as_long_as_glide_says() {
        // From MIDI note 24 (32.7 Hz) to 31 (49 Hz) while it rings: halfway there in the middle of the glide, there after it.
        let (c1, g1) = (midi_to_freq(24.0), midi_to_freq(31.0));
        for glide in [0.6f32, 1.0] {
            let mut v = PrincessVoice::default();
            let mut rng = Rng::new(3);
            v.trigger(DrumVoice::ApFried808, 1.0, 1.0, Some(c1), false, 2.0, 0.0, glide, 0.0, SR);
            let x: Vec<f32> = (0..(0.4 * SR) as usize).map(|_| v.tick(DrumVoice::ApFried808, &mut rng)).collect();
            let before = pitch(at(&x, 0.15, 0.4));
            assert!((before / c1 - 1.0).abs() < 0.03, "plays 32.7 Hz: {before} Hz");
            v.trigger(DrumVoice::ApFried808, 1.0, 1.0, Some(g1), true, 2.0, 0.0, glide, 0.0, SR);
            let secs = glide_secs(glide);
            let y: Vec<f32> = (0..((secs + 0.4) * SR) as usize).map(|_| v.tick(DrumVoice::ApFried808, &mut rng)).collect();
            let mid = pitch(at(&y, 0.25 * secs, 0.75 * secs));
            let after = pitch(at(&y, secs + 0.05, secs + 0.4));
            assert!(mid > c1 * 1.12 && mid < g1 * 0.92, "glide {glide}: {mid} Hz on the way from {c1} to {g1}");
            assert!((after / g1 - 1.0).abs() < 0.03, "glide {glide}: arrives at {after} Hz");
            // A slide is no new hit: it stays as loud as it was.
            assert!(peak(&y) < peak(&x) * 1.05);
        }
        // Without Glide it is there at once.
        let mut v = PrincessVoice::default();
        let mut rng = Rng::new(3);
        v.trigger(DrumVoice::ApFried808, 1.0, 1.0, Some(c1), false, 2.0, 0.0, 0.0, 0.0, SR);
        for _ in 0..(0.3 * SR) as usize {
            v.tick(DrumVoice::ApFried808, &mut rng);
        }
        v.trigger(DrumVoice::ApFried808, 1.0, 1.0, Some(g1), true, 2.0, 0.0, 0.0, 0.0, SR);
        let y: Vec<f32> = (0..(0.2 * SR) as usize).map(|_| v.tick(DrumVoice::ApFried808, &mut rng)).collect();
        assert!((pitch(at(&y, 0.0, 0.2)) / g1 - 1.0).abs() < 0.03);
    }

    #[test]
    fn the_second_808_rings_sidebands_into_the_first() {
        // Clean (Tone 0): the first at 49 Hz, the second at 73.5; their sum, 122.5 Hz, is no harmonic of either.
        let (dry, _) = play(DrumVoice::ApFried808, 0.0, 0.0, 0.0, 0.4);
        let (ring, _) = play(DrumVoice::ApFried808, 0.0, 0.0, 1.0, 0.4);
        let (d, r) = (at(&dry, 0.08, 0.38), at(&ring, 0.08, 0.38));
        let side = |x: &[f32]| magnitude(x, BASE_808 * 2.5);
        assert!(side(r) > 10.0 * side(d).max(1e-5), "sum {} with Ring, {} without", side(r), side(d));
        assert!(magnitude(r, BASE_808 * 0.5) > 5.0 * magnitude(d, BASE_808 * 0.5).max(1e-5), "and the difference");
        // 808 into 808: the second one's pitch pushed around adds partials too.
        let (fed, _) = play(DrumVoice::ApDouble808, 0.0, 0.0, 1.0, 0.4);
        let (plain, _) = play(DrumVoice::ApDouble808, 0.0, 0.0, 0.0, 0.4);
        assert!(bright(at(&fed, 0.08, 0.38)) > 2.0 * bright(at(&plain, 0.08, 0.38)));
    }

    #[test]
    fn tone_clips_the_808_to_the_ceiling() {
        let (clean, _) = play(DrumVoice::ApFried808, 0.0, 0.0, 0.0, 0.5);
        let (fried, _) = play(DrumVoice::ApFried808, 1.0, 0.0, 0.0, 0.5);
        let (c, f) = (at(&clean, 0.1, 0.4), at(&fried, 0.1, 0.4));
        assert!(peak(&fried) <= CEILING + 1e-6, "capped at the ceiling: {}", peak(&fried));
        // Square: the peak is the level (crest near 1), and the odd harmonics come up.
        assert!(peak(f) / rms(f) < 1.15, "crest {}", peak(f) / rms(f));
        assert!(peak(c) / rms(c) > 1.3, "the clean one is a sine: crest {}", peak(c) / rms(c));
        let third = |x: &[f32]| magnitude(x, BASE_808 * 3.0) / magnitude(x, BASE_808);
        assert!(third(f) > 0.2 && third(f) > 5.0 * third(c), "third harmonic {} against {}", third(f), third(c));
        // The kick's Clip too.
        let (k0, _) = play(DrumVoice::ApKick, 0.5, 0.5, 0.0, 0.3);
        let (k1, _) = play(DrumVoice::ApKick, 0.5, 0.5, 1.0, 0.3);
        assert!(peak(&k1) <= CEILING + 1e-6);
        assert!(rms(at(&k1, 0.05, 0.2)) > 1.5 * rms(at(&k0, 0.05, 0.2)), "louder for as loud a peak");
    }

    #[test]
    fn the_crunch_808_falls_on_fewer_steps() {
        let mut v = PrincessVoice::default();
        let mut rng = Rng::new(1);
        v.trigger(DrumVoice::ApCrunch808, 1.0, 1.0, None, false, 1.0, 0.3, 0.0, 1.0, SR);
        let x: Vec<f32> = (0..4800).map(|_| v.tick(DrumVoice::ApCrunch808, &mut rng)).collect();
        // Three bits: four steps a side.
        let mut levels: Vec<i32> = x.iter().map(|s| (s * 1000.0).round() as i32).collect();
        levels.sort();
        levels.dedup();
        assert!(levels.len() <= 9, "{levels:?}");
    }

    #[test]
    fn channel_surf_flips_between_channels() {
        // Brightness per 10 ms jumps as it flips from one channel to another.
        let (x, _) = play(DrumVoice::ApChannelSurf, 0.7, 0.5, 0.5, 0.7);
        let w = (0.01 * SR) as usize;
        let b: Vec<f32> = x.chunks(w).take(65).map(bright).collect();
        let jumps = b.windows(2).filter(|p| p[1] > p[0] * 2.5 || p[0] > p[1] * 2.5).count();
        assert!(jumps >= 3, "{jumps} jumps in {b:?}");
        let max = b.iter().cloned().fold(0.0, f32::max);
        let min = b.iter().cloned().fold(1.0, f32::min);
        assert!(max > 8.0 * min, "channels from {min} to {max}");
        // Surf flips faster.
        let count = |a: f32| {
            let mut v = PrincessVoice::default();
            let mut rng = Rng::new(9);
            v.trigger(DrumVoice::ApChannelSurf, 1.0, 1.0, None, false, 1.0, 0.7, a, 0.5, SR);
            let mut flips = 0;
            for _ in 0..(0.6 * SR) as usize {
                if v.chan_left == 0 {
                    flips += 1;
                }
                v.tick(DrumVoice::ApChannelSurf, &mut rng);
            }
            flips
        };
        assert!(count(1.0) > 2 * count(0.0), "{} vs {}", count(1.0), count(0.0));
    }

    #[test]
    fn the_hat_roll_keeps_the_songs_time() {
        // Sixteenths at 120 BPM: four hits 125 ms apart in a beat; thirty-seconds: eight.
        for (b, hits) in [(0.1, 4), (0.6, 8)] {
            let mut v = PrincessVoice::default();
            v.bpm = 120.0;
            let mut rng = Rng::new(2);
            v.trigger(DrumVoice::ApHatRoll, 1.0, 1.0, None, false, 1.0, 0.5, 0.5, b, SR);
            let x: Vec<f32> = (0..(0.6 * SR) as usize).map(|_| v.tick(DrumVoice::ApHatRoll, &mut rng)).collect();
            let w = (0.005 * SR) as usize;
            let env: Vec<f32> = x.chunks(w).map(peak).collect();
            let onsets = env.windows(2).filter(|p| p[1] > p[0] * 3.0 && p[1] > 0.05).count() + 1;
            assert_eq!(onsets, hits, "Roll {b}: {env:?}");
        }
    }

    #[test]
    fn the_subliminal_swells_up_and_a_hat_is_brighter_hit_hard() {
        let (x, _) = play(DrumVoice::ApSubliminal, 0.5, 0.5, 0.0, 1.2);
        assert!(rms(at(&x, 0.5, 0.6)) > 4.0 * rms(at(&x, 0.0, 0.1)), "it swells");
        let hat = |vel: f32| {
            let mut v = PrincessVoice::default();
            let mut rng = Rng::new(4);
            v.trigger(DrumVoice::ApHat, vel, 1.0, None, false, 1.0, 0.5, 0.5, 0.5, SR);
            (0..2400).map(|_| v.tick(DrumVoice::ApHat, &mut rng)).collect::<Vec<f32>>()
        };
        let (soft, hard) = (hat(0.2), hat(1.0));
        assert!(rms(&hard[1200..]) > 1.5 * rms(&soft[1200..]), "a hard hit is longer");
    }

    #[test]
    fn the_machines_808_slides_and_distorts_four_ways() {
        let (c1, g1) = (midi_to_freq(24.0), midi_to_freq(31.0));
        let mut v = Princess808::default();
        let mut rng = Rng::new(5);
        let k = Knobs808 { drive: 0.0, ring: 0.0, click: 0.0, decay: 2.0, glide: 0.2, ..Knobs808::default() };
        v.trigger(c1, 1.0, false, k, SR);
        let x: Vec<f32> = (0..(0.4 * SR) as usize).map(|_| v.tick(&mut rng)).collect();
        assert!((pitch(at(&x, 0.15, 0.4)) / c1 - 1.0).abs() < 0.03);
        v.trigger(g1, 1.0, true, k, SR);
        let y: Vec<f32> = (0..(0.6 * SR) as usize).map(|_| v.tick(&mut rng)).collect();
        let mid = pitch(at(&y, 0.05, 0.15));
        assert!(mid > c1 * 1.12 && mid < g1 * 0.92, "on its way: {mid}");
        assert!((pitch(at(&y, 0.25, 0.55)) / g1 - 1.0).abs() < 0.03);
        assert!(peak(&y) < peak(&x) * 1.05, "a slide is no new hit");
        // Every distortion stays under the ceiling and brings up harmonics.
        for dist in [Dist::Soft, Dist::Hard, Dist::Fold, Dist::Tube] {
            let mut v = Princess808::default();
            v.trigger(BASE_808, 1.0, false, Knobs808 { drive: 1.0, dist, ring: 0.0, click: 0.0, ..Knobs808::default() }, SR);
            let x: Vec<f32> = (0..(0.5 * SR) as usize).map(|_| v.tick(&mut rng)).collect();
            assert!(x.iter().all(|s| s.is_finite()));
            assert!(peak(&x) <= CEILING + 1e-4, "{dist:?}: {}", peak(&x));
            let w = at(&x, 0.1, 0.4);
            let partials = (magnitude(w, BASE_808 * 2.0) + magnitude(w, BASE_808 * 3.0)) / magnitude(w, BASE_808);
            assert!(partials > 0.15, "{dist:?}: {partials}");
        }
    }

    #[test]
    fn the_machine_shapes_the_drop_in_pitch() {
        // The kick's drop, twice as far: higher at the start, the same after.
        let run = |amt: f32| {
            let mut v = PrincessVoice::default();
            v.pitch_amt = amt;
            let mut rng = Rng::new(2);
            v.trigger(DrumVoice::ApKick, 1.0, 1.0, None, false, 1.0, 0.0, 0.5, 0.0, SR);
            (0..(0.3 * SR) as usize).map(|_| v.tick(DrumVoice::ApKick, &mut rng)).collect::<Vec<f32>>()
        };
        let (plain, far) = (run(1.0), run(2.0));
        assert!(pitch(at(&far, 0.0, 0.02)) > 1.3 * pitch(at(&plain, 0.0, 0.02)));
        assert!((pitch(at(&far, 0.2, 0.3)) / pitch(at(&plain, 0.2, 0.3)) - 1.0).abs() < 0.05);
    }

    /// Levels of every voice (cargo test -- --ignored --nocapture princess_levels).
    #[test]
    #[ignore]
    fn princess_levels() {
        for voice in ALL {
            let (x, _) = play(voice, 0.5, 0.5, 0.5, 2.0);
            println!("{voice:?}: peak {:.3}, rms(0..0.1) {:.3}", peak(&x), rms(at(&x, 0.0, 0.1)));
        }
    }
}

//! Chip drums: drums as the old consoles, handhelds and home computers made
//! them, each from its chip's own parts. Console: the triangle that has no
//! volume, swept down a frame at a time, long and short (metallic) noise,
//! and drums through the 1-bit delta sample channel (its 7-bit counter
//! stepping up or down two at a time: the crunch). Handheld: the 4-bit wave
//! channel, 15- and 7-bit noise, the envelope stepping 64 times a second.
//! Micro: the SID-style kits of sidacid by NorfolkHacker (MIT, notice below:
//! naive triangles, pulse clicks, ring modulation, an 8-bit LFSR noise, a
//! squelchy state variable filter). Arcade: two-operator FM with feedback and
//! the PSG's noise, through a 9-bit output. Beeper: one bit, a speaker on or
//! off, its loudness only in how often it is on. Woodgrain: the polynomial
//! counters of the woodgrain console's sound chip, pitch in 32 steps of a
//! divider, volume in 16 steps a frame. Every drum is a recipe of those
//! parts; Tune, Decay, Tone and the two macros (Sweep, Noise) bend them.
//!
//! The Micro recipes follow sidacid (<https://github.com/NorfolkHacker/sidacid>),
//! MIT License, Copyright (c) 2026 NorfolkHacker: Permission is hereby
//! granted, free of charge, to any person obtaining a copy of this software
//! and associated documentation files (the "Software"), to deal in the
//! Software without restriction, including without limitation the rights to
//! use, copy, modify, merge, publish, distribute, sublicense, and/or sell
//! copies of the Software, and to permit persons to whom the Software is
//! furnished to do so, subject to the following conditions: The above
//! copyright notice and this permission notice shall be included in all
//! copies or substantial portions of the Software. THE SOFTWARE IS PROVIDED
//! "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED.

use std::f32::consts::{PI, TAU};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Chip {
    Console,
    Handheld,
    Micro,
    Arcade,
    Beeper,
    Woodgrain,
}

impl Chip {
    /// Frames a second its music drivers stepped volume and pitch at (0: smooth).
    fn frame_rate(self) -> f32 {
        match self {
            Chip::Console | Chip::Woodgrain => 60.0,
            Chip::Handheld => 64.0,
            _ => 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Wave {
    None,
    /// The console's 32-step triangle (no volume: on or off).
    NesTri,
    Pulse(f32),
    /// SID-style naive triangle.
    Tri,
    Sine,
    /// A 4-bit, 32-step sine (the handheld's wave channel).
    Wave4,
    /// The woodgrain chip's tones: a counter of this many steps per cycle.
    Tia(TiaShape),
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum TiaShape {
    /// A square: two steps.
    Square,
    /// The "buzz": a 31-step pattern.
    Div31,
    /// The 4-bit polynomial (15 steps).
    Poly4,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Noise {
    None,
    /// The console's noise: long (15-bit) or short (93 steps, metallic), at a clock (Hz).
    NesLong(f32),
    NesShort(f32),
    /// The handheld's: 15- or 7-bit.
    Gb15(f32),
    Gb7(f32),
    /// sidacid's 16-bit Galois LFSR, its low byte the sample.
    Sid(f32),
    /// The PSG's 16-bit noise.
    Psg(f32),
    /// The woodgrain chip's 9-bit polynomial.
    Tia9(f32),
    /// One random bit.
    Bit(f32),
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Filter {
    None,
    Hp(f32, f32),
    Bp(f32, f32),
}

#[derive(Clone, Copy, Debug)]
struct Recipe {
    chip: Chip,
    wave: Wave,
    f_start: f32,
    f_end: f32,
    /// Pitch time constant (s).
    sweep: f32,
    tone_level: f32,
    tone_decay: f32,
    /// A pulse click before the tone (s), as sidacid's 909s.
    click: f32,
    /// A second oscillator: ring modulating (true) or added, at this ratio.
    osc2: Option<(f32, bool)>,
    /// FM: modulator ratio, index, how fast the index falls (s), feedback.
    fm: Option<(f32, f32, f32, f32)>,
    noise: Noise,
    noise_level: f32,
    noise_decay: f32,
    /// Claps: bursts and their spacing (s).
    bursts: u8,
    spacing: f32,
    filter: Filter,
    /// Through the 1-bit delta sample channel.
    dpcm: bool,
    one_bit: bool,
    level: f32,
}

const BASE: Recipe = Recipe {
    chip: Chip::Micro,
    wave: Wave::None,
    f_start: 100.0,
    f_end: 100.0,
    sweep: 0.03,
    tone_level: 1.0,
    tone_decay: 0.2,
    click: 0.0,
    osc2: None,
    fm: None,
    noise: Noise::None,
    noise_level: 0.0,
    noise_decay: 0.1,
    bursts: 0,
    spacing: 0.01,
    filter: Filter::None,
    dpcm: false,
    one_bit: false,
    level: 1.0,
};

/// The drums.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    ConsoleKick,
    ConsoleSampleKick,
    ConsoleSnare,
    ConsoleSampleSnare,
    ConsoleHat,
    ConsoleOpenHat,
    ConsoleTom,
    ConsoleCrash,
    HandheldKick,
    HandheldSnare,
    HandheldHat,
    HandheldOpenHat,
    HandheldTom,
    HandheldClap,
    MicroKick,
    MicroSnare,
    MicroTom,
    MicroClap,
    MicroRim,
    MicroHat,
    MicroOpenHat,
    MicroPunchKick,
    MicroPunchSnare,
    MicroPunchClap,
    MicroPunchRim,
    MicroPunchHat,
    MicroPunchOpenHat,
    MicroPunchTom,
    ArcadeKick,
    ArcadeSnare,
    ArcadeTom,
    ArcadeHat,
    ArcadeOpenHat,
    ArcadeCowbell,
    BeeperKick,
    BeeperSnare,
    BeeperHat,
    BeeperTom,
    WoodKick,
    WoodSnare,
    WoodHat,
    WoodTom,
    WoodZap,
    WoodBoom,
}

const NES_CPU: f32 = 1_789_773.0;
const NES_NOISE: [f32; 16] = [4.0, 8.0, 16.0, 32.0, 64.0, 96.0, 128.0, 160.0, 202.0, 254.0, 380.0, 508.0, 762.0, 1016.0, 2034.0, 4068.0];
const TIA_CLOCK: f32 = 31_400.0;

fn nes_noise(i: usize) -> f32 {
    NES_CPU / NES_NOISE[i]
}

fn recipe(kind: Kind) -> Recipe {
    use Kind::*;
    let console = Recipe { chip: Chip::Console, ..BASE };
    let handheld = Recipe { chip: Chip::Handheld, ..BASE };
    let micro = BASE;
    let arcade = Recipe { chip: Chip::Arcade, ..BASE };
    let beeper = Recipe { chip: Chip::Beeper, one_bit: true, ..BASE };
    let wood = Recipe { chip: Chip::Woodgrain, ..BASE };
    match kind {
        ConsoleKick => Recipe { wave: Wave::NesTri, f_start: 260.0, f_end: 45.0, sweep: 0.03, tone_decay: 0.22, noise: Noise::NesLong(nes_noise(12)), noise_level: 0.5, noise_decay: 0.035, ..console },
        ConsoleSampleKick => Recipe { wave: Wave::Sine, f_start: 190.0, f_end: 50.0, sweep: 0.035, tone_decay: 0.35, dpcm: true, level: 1.1, ..console },
        ConsoleSnare => Recipe { wave: Wave::NesTri, f_start: 240.0, f_end: 160.0, sweep: 0.04, tone_decay: 0.08, noise: Noise::NesLong(nes_noise(5)), noise_level: 0.9, noise_decay: 0.25, ..console },
        ConsoleSampleSnare => Recipe { wave: Wave::Sine, f_start: 220.0, f_end: 175.0, sweep: 0.02, tone_decay: 0.1, tone_level: 0.7, noise: Noise::Bit(20000.0), noise_level: 0.7, noise_decay: 0.18, filter: Filter::Bp(2800.0, 0.2), dpcm: true, level: 1.8, ..console },
        ConsoleHat => Recipe { noise: Noise::NesShort(nes_noise(2)), noise_level: 0.7, noise_decay: 0.05, ..console },
        ConsoleOpenHat => Recipe { noise: Noise::NesShort(nes_noise(3)), noise_level: 0.6, noise_decay: 0.4, ..console },
        ConsoleTom => Recipe { wave: Wave::NesTri, f_start: 230.0, f_end: 110.0, sweep: 0.08, tone_decay: 0.3, noise: Noise::NesLong(nes_noise(10)), noise_level: 0.25, noise_decay: 0.03, ..console },
        ConsoleCrash => Recipe { noise: Noise::NesLong(nes_noise(3)), noise_level: 0.6, noise_decay: 1.3, ..console },
        HandheldKick => Recipe { wave: Wave::Wave4, f_start: 220.0, f_end: 48.0, sweep: 0.03, tone_decay: 0.25, noise: Noise::Gb15(6000.0), noise_level: 0.35, noise_decay: 0.02, ..handheld },
        HandheldSnare => Recipe { wave: Wave::Pulse(0.5), f_start: 230.0, f_end: 180.0, sweep: 0.02, tone_level: 0.45, tone_decay: 0.08, noise: Noise::Gb15(65536.0), noise_level: 0.9, noise_decay: 0.22, ..handheld },
        HandheldHat => Recipe { noise: Noise::Gb7(262_144.0), noise_level: 0.55, noise_decay: 0.04, ..handheld },
        HandheldOpenHat => Recipe { noise: Noise::Gb15(131_072.0), noise_level: 0.5, noise_decay: 0.35, ..handheld },
        HandheldTom => Recipe { wave: Wave::Wave4, f_start: 270.0, f_end: 140.0, sweep: 0.06, tone_decay: 0.3, ..handheld },
        HandheldClap => Recipe { noise: Noise::Gb15(40000.0), noise_level: 0.9, noise_decay: 0.18, bursts: 3, spacing: 0.012, filter: Filter::Bp(1500.0, 0.3), ..handheld },
        // sidacid's kit A: the 808 made of SID parts.
        MicroKick => Recipe { wave: Wave::Tri, f_start: 150.0, f_end: 50.0, sweep: 0.030, tone_decay: 0.75, ..micro },
        MicroSnare => Recipe { wave: Wave::Tri, f_start: 330.0, f_end: 180.0, sweep: 0.010, tone_decay: 0.17, tone_level: 0.56, osc2: Some((1.48, false)), noise: Noise::Sid(0.0), noise_level: 0.7, noise_decay: 0.225, filter: Filter::Hp(1500.0, 0.2), level: 0.9, ..micro },
        MicroTom => Recipe { wave: Wave::Tri, f_start: 181.0, f_end: 110.0, sweep: 0.05, tone_decay: 0.5, tone_level: 0.9, noise: Noise::Sid(0.0), noise_level: 0.05, noise_decay: 0.05, filter: Filter::Hp(3000.0, 0.0), ..micro },
        MicroClap => Recipe { noise: Noise::Sid(0.0), noise_level: 0.9, noise_decay: 0.3, bursts: 3, spacing: 0.011, filter: Filter::Bp(1750.0, 0.3), level: 1.3, ..micro },
        MicroRim => Recipe { wave: Wave::Pulse(0.5), f_start: 1700.0, f_end: 1700.0, tone_decay: 0.035, tone_level: 0.65, level: 0.7, ..micro },
        MicroHat => Recipe { noise: Noise::Sid(0.0), noise_level: 0.7, noise_decay: 0.07, filter: Filter::Hp(8500.0, 0.1), level: 1.4, ..micro },
        MicroOpenHat => Recipe { noise: Noise::Sid(0.0), noise_level: 0.7, noise_decay: 0.65, filter: Filter::Hp(8500.0, 0.1), level: 1.2, ..micro },
        // Kit B: the 909.
        MicroPunchKick => Recipe { wave: Wave::Tri, f_start: 250.0, f_end: 50.0, sweep: 0.012, tone_decay: 0.45, click: 0.004, ..micro },
        MicroPunchSnare => Recipe { wave: Wave::Tri, f_start: 260.0, f_end: 190.0, sweep: 0.015, tone_decay: 0.11, tone_level: 0.7, noise: Noise::Sid(0.0), noise_level: 0.8, noise_decay: 0.255, filter: Filter::Hp(800.0, 0.0), level: 0.9, ..micro },
        MicroPunchClap => Recipe { noise: Noise::Sid(0.0), noise_level: 0.9, noise_decay: 0.23, bursts: 4, spacing: 0.008, filter: Filter::Bp(1700.0, 0.4), level: 1.3, ..micro },
        MicroPunchRim => Recipe { wave: Wave::Tri, f_start: 1100.0, f_end: 1100.0, tone_decay: 0.03, tone_level: 0.7, osc2: Some((1.62, true)), ..micro },
        MicroPunchHat => Recipe { wave: Wave::Tri, f_start: 3000.0, f_end: 3000.0, tone_level: 0.35, tone_decay: 0.09, osc2: Some((std::f32::consts::SQRT_2, true)), noise: Noise::Sid(0.0), noise_level: 0.7, noise_decay: 0.09, filter: Filter::Hp(9000.0, 0.1), level: 1.2, ..micro },
        MicroPunchOpenHat => Recipe { wave: Wave::Tri, f_start: 3000.0, f_end: 3000.0, tone_level: 0.35, tone_decay: 0.55, osc2: Some((std::f32::consts::SQRT_2, true)), noise: Noise::Sid(0.0), noise_level: 0.7, noise_decay: 0.55, filter: Filter::Hp(9000.0, 0.1), level: 1.1, ..micro },
        MicroPunchTom => Recipe { wave: Wave::Tri, f_start: 210.0, f_end: 100.0, sweep: 0.06, tone_decay: 0.4, tone_level: 0.9, click: 0.003, ..micro },
        ArcadeKick => Recipe { wave: Wave::Sine, f_start: 170.0, f_end: 45.0, sweep: 0.025, tone_decay: 0.4, fm: Some((1.0, 1.6, 0.025, 0.35)), ..arcade },
        ArcadeSnare => Recipe { wave: Wave::Sine, f_start: 210.0, f_end: 160.0, sweep: 0.03, tone_decay: 0.12, tone_level: 0.7, fm: Some((1.41, 3.0, 0.06, 0.2)), noise: Noise::Psg(55930.0), noise_level: 0.7, noise_decay: 0.2, ..arcade },
        ArcadeTom => Recipe { wave: Wave::Sine, f_start: 230.0, f_end: 120.0, sweep: 0.05, tone_decay: 0.32, fm: Some((1.0, 1.0, 0.05, 0.2)), ..arcade },
        ArcadeHat => Recipe { wave: Wave::Sine, f_start: 3200.0, f_end: 3200.0, tone_level: 0.3, tone_decay: 0.04, fm: Some((1.39, 4.0, 0.04, 0.0)), noise: Noise::Psg(111_860.0), noise_level: 0.6, noise_decay: 0.04, filter: Filter::Hp(6000.0, 0.0), level: 1.2, ..arcade },
        ArcadeOpenHat => Recipe { wave: Wave::Sine, f_start: 3200.0, f_end: 3200.0, tone_level: 0.3, tone_decay: 0.4, fm: Some((1.39, 4.0, 0.4, 0.0)), noise: Noise::Psg(111_860.0), noise_level: 0.55, noise_decay: 0.4, filter: Filter::Hp(6000.0, 0.0), level: 1.1, ..arcade },
        ArcadeCowbell => Recipe { wave: Wave::Sine, f_start: 800.0, f_end: 800.0, tone_decay: 0.3, tone_level: 0.8, fm: Some((1.43, 1.5, 0.3, 0.1)), ..arcade },
        BeeperKick => Recipe { wave: Wave::Pulse(0.5), f_start: 320.0, f_end: 40.0, sweep: 0.04, tone_decay: 0.16, level: 0.6, ..beeper },
        BeeperSnare => Recipe { wave: Wave::Pulse(0.5), f_start: 210.0, f_end: 180.0, tone_level: 0.5, tone_decay: 0.04, noise: Noise::Bit(22050.0), noise_level: 1.0, noise_decay: 0.15, level: 0.6, ..beeper },
        BeeperHat => Recipe { noise: Noise::Bit(44100.0), noise_level: 1.0, noise_decay: 0.03, level: 0.5, ..beeper },
        BeeperTom => Recipe { wave: Wave::Pulse(0.5), f_start: 250.0, f_end: 110.0, sweep: 0.06, tone_decay: 0.22, level: 0.6, ..beeper },
        WoodKick => Recipe { wave: Wave::Tia(TiaShape::Div31), f_start: 150.0, f_end: 50.0, sweep: 0.04, tone_decay: 0.25, ..wood },
        WoodSnare => Recipe { noise: Noise::Tia9(TIA_CLOCK / 9.0), noise_level: 0.9, noise_decay: 0.2, wave: Wave::Tia(TiaShape::Square), f_start: 220.0, f_end: 180.0, tone_level: 0.4, tone_decay: 0.06, ..wood },
        WoodHat => Recipe { noise: Noise::Tia9(TIA_CLOCK), noise_level: 0.7, noise_decay: 0.04, ..wood },
        WoodTom => Recipe { wave: Wave::Tia(TiaShape::Square), f_start: 200.0, f_end: 100.0, sweep: 0.07, tone_decay: 0.3, ..wood },
        WoodZap => Recipe { wave: Wave::Tia(TiaShape::Poly4), f_start: 900.0, f_end: 90.0, sweep: 0.12, tone_decay: 0.35, level: 0.8, ..wood },
        WoodBoom => Recipe { noise: Noise::Tia9(TIA_CLOCK / 32.0), noise_level: 1.0, noise_decay: 1.2, ..wood },
    }
}

/// The pad's knobs.
#[derive(Clone, Copy, Debug)]
pub struct ChipKnobs {
    /// Pitch, as a ratio.
    pub tune: f32,
    /// Length, as a ratio (1: as designed).
    pub decay: f32,
    /// 0..1: noise brighter or darker.
    pub tone: f32,
    /// 0..1: more or less pitch sweep (0.5: as designed).
    pub sweep: f32,
    /// 0..1: more or less noise (0.5: as designed).
    pub noise: f32,
}

#[derive(Clone, Copy)]
pub struct ChipVoice {
    r: Recipe,
    sr: f32,
    t: f32,
    vel: f32,
    f_start: f32,
    f_end: f32,
    tone_decay: f32,
    noise_decay: f32,
    noise_rate: f32,
    noise_amt: f32,
    filter: Filter,
    ph: [f32; 3],
    lfsr: u32,
    noise_acc: f32,
    noise_out: f32,
    fb: [f32; 2],
    svf: [f32; 2],
    frame_t: f32,
    frame_tone: f32,
    frame_noise: f32,
    frame_f: f32,
    dpcm_acc: f32,
    dpcm_counter: i32,
    dpcm_out: f32,
    dc_x: f32,
    dc_y: f32,
    rng: u32,
    active: bool,
}

impl Default for ChipVoice {
    fn default() -> Self {
        Self {
            r: BASE,
            sr: 48000.0,
            t: 0.0,
            vel: 1.0,
            f_start: 100.0,
            f_end: 100.0,
            tone_decay: 0.2,
            noise_decay: 0.1,
            noise_rate: 0.0,
            noise_amt: 0.0,
            filter: Filter::None,
            ph: [0.0; 3],
            lfsr: 1,
            noise_acc: 0.0,
            noise_out: 0.0,
            fb: [0.0; 2],
            svf: [0.0; 2],
            frame_t: 0.0,
            frame_tone: 1.0,
            frame_noise: 1.0,
            frame_f: 100.0,
            dpcm_acc: 0.0,
            dpcm_counter: 64,
            dpcm_out: 0.0,
            dc_x: 0.0,
            dc_y: 0.0,
            rng: 0x1F2E_3D4C,
            active: false,
        }
    }
}

/// How far down an exponential decay over `secs` has come at `t` (-60 dB at `secs`).
fn decay(t: f32, secs: f32) -> f32 {
    (-6.9 * t / secs.max(0.001)).exp()
}

impl ChipVoice {
    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn trigger(&mut self, kind: Kind, k: ChipKnobs, vel: f32, sr: f32) {
        let r = recipe(kind);
        let seed = self.rng;
        *self = Self { r, sr: sr.max(1000.0), vel: vel.clamp(0.0, 1.0), rng: seed.wrapping_mul(747_796_405).wrapping_add(2_891_336_453), ..Self::default() };
        // Sweep: the start further from the end (or closer).
        let ratio = (r.f_start / r.f_end).max(1e-3);
        let sweep = ratio.powf((2.0 * k.sweep).clamp(0.0, 2.0));
        self.f_end = r.f_end * k.tune;
        self.f_start = self.f_end * sweep;
        self.tone_decay = r.tone_decay * k.decay.max(0.05);
        self.noise_decay = r.noise_decay * k.decay.max(0.05);
        let bright = 2f32.powf((k.tone - 0.5) * 2.0);
        self.noise_rate = match r.noise {
            Noise::None => 0.0,
            Noise::Sid(_) => self.sr * bright,
            Noise::NesLong(f) | Noise::NesShort(f) | Noise::Gb15(f) | Noise::Gb7(f) | Noise::Psg(f) | Noise::Tia9(f) | Noise::Bit(f) => f * bright,
        };
        self.noise_amt = r.noise_level * (2.0 * k.noise).clamp(0.0, 2.0);
        self.filter = match r.filter {
            Filter::Hp(f, q) => Filter::Hp(f * bright, q),
            Filter::Bp(f, q) => Filter::Bp(f * bright, q),
            Filter::None => Filter::None,
        };
        self.lfsr = match r.noise {
            Noise::Sid(_) => 0xACE1,
            _ => 0x7FFF,
        };
        self.frame_f = self.f_start;
        self.active = true;
    }

    fn clock_noise(&mut self) -> f32 {
        match self.r.noise {
            Noise::NesLong(_) | Noise::Gb15(_) => {
                let bit = (self.lfsr ^ (self.lfsr >> 1)) & 1;
                self.lfsr = (self.lfsr >> 1) | (bit << 14);
                if self.lfsr & 1 == 0 {
                    1.0
                } else {
                    -1.0
                }
            }
            Noise::NesShort(_) => {
                let bit = (self.lfsr ^ (self.lfsr >> 6)) & 1;
                self.lfsr = (self.lfsr >> 1) | (bit << 14);
                if self.lfsr & 1 == 0 {
                    1.0
                } else {
                    -1.0
                }
            }
            Noise::Gb7(_) => {
                let bit = (self.lfsr ^ (self.lfsr >> 1)) & 1;
                self.lfsr = ((self.lfsr >> 1) | (bit << 14)) & !0x40 | (bit << 6);
                if self.lfsr & 1 == 0 {
                    1.0
                } else {
                    -1.0
                }
            }
            Noise::Sid(_) => {
                let lsb = self.lfsr & 1;
                self.lfsr >>= 1;
                if lsb != 0 {
                    self.lfsr ^= 0xB400;
                }
                (self.lfsr & 0xFF) as f32 / 127.5 - 1.0
            }
            Noise::Psg(_) => {
                let bit = (self.lfsr ^ (self.lfsr >> 3)) & 1;
                self.lfsr = (self.lfsr >> 1) | (bit << 15);
                if self.lfsr & 1 != 0 {
                    1.0
                } else {
                    -1.0
                }
            }
            Noise::Tia9(_) => {
                // x^9 + x^5 + 1
                let bit = ((self.lfsr >> 8) ^ (self.lfsr >> 4)) & 1;
                self.lfsr = ((self.lfsr << 1) | bit) & 0x1FF;
                if self.lfsr == 0 {
                    self.lfsr = 1;
                }
                if self.lfsr & 1 != 0 {
                    1.0
                } else {
                    -1.0
                }
            }
            Noise::Bit(_) => {
                self.rng ^= self.rng << 13;
                self.rng ^= self.rng >> 17;
                self.rng ^= self.rng << 5;
                if self.rng & 1 != 0 {
                    1.0
                } else {
                    -1.0
                }
            }
            Noise::None => 0.0,
        }
    }

    fn noise(&mut self) -> f32 {
        if self.noise_rate <= 0.0 {
            return 0.0;
        }
        // Clocked at its rate, held between (as the chips hold it); several
        // clocks a sample averaged.
        self.noise_acc += self.noise_rate / self.sr;
        if self.noise_acc < 1.0 {
            return self.noise_out;
        }
        let n = (self.noise_acc as u32).min(64);
        self.noise_acc -= self.noise_acc.floor();
        let mut sum = 0.0;
        for _ in 0..n {
            sum += self.clock_noise();
        }
        self.noise_out = sum / n as f32;
        self.noise_out
    }

    fn osc(&mut self, f: f32) -> f32 {
        let r = self.r;
        let dt = (f / self.sr).min(0.49);
        let ph = self.ph[0];
        let y = match r.wave {
            Wave::None => 0.0,
            Wave::NesTri => {
                let step = (ph * 32.0) as i32;
                let v = if step < 16 { 15 - step } else { step - 16 };
                v as f32 / 7.5 - 1.0
            }
            Wave::Pulse(w) => {
                if ph < w {
                    1.0
                } else {
                    -1.0
                }
            }
            Wave::Tri => {
                let msb = ph >= 0.5;
                let ring = match r.osc2 {
                    Some((_, true)) => self.ph[1] >= 0.5,
                    _ => false,
                };
                let x = (2.0 * ph).fract();
                let t = if msb ^ ring { 1.0 - x } else { x };
                2.0 * t - 1.0
            }
            Wave::Sine => {
                if let Some((ratio, index, fall, feedback)) = r.fm {
                    // Two operators: the modulator's index falls away.
                    let env = decay(self.t, fall);
                    let m = (TAU * self.ph[1] + feedback * (self.fb[0] + self.fb[1]) * 0.5).sin();
                    self.fb = [self.fb[1], m];
                    self.ph[1] = (self.ph[1] + dt * ratio).fract();
                    (TAU * ph + index * env * m).sin()
                } else {
                    (TAU * ph).sin()
                }
            }
            Wave::Wave4 => {
                let step = (ph * 32.0) as usize;
                let v = ((TAU * (step as f32 + 0.5) / 32.0).sin() * 7.5 + 7.5).round();
                v / 7.5 - 1.0
            }
            Wave::Tia(shape) => {
                // A divider of a 31.4 kHz clock: the pitch in its steps.
                let steps = match shape {
                    TiaShape::Square => 2.0,
                    TiaShape::Div31 => 31.0,
                    TiaShape::Poly4 => 15.0,
                };
                let i = (ph * steps) as usize;
                let bit = match shape {
                    TiaShape::Square => i < 1,
                    TiaShape::Div31 => i < 13,
                    TiaShape::Poly4 => (0b000_1001_1010_1111u16 >> (i % 15)) & 1 != 0,
                };
                if bit {
                    1.0
                } else {
                    -1.0
                }
            }
        };
        self.ph[0] += dt;
        if self.ph[0] >= 1.0 {
            self.ph[0] -= 1.0;
        }
        if let Some((ratio, ring)) = r.osc2 {
            if r.fm.is_none() {
                self.ph[1] = (self.ph[1] + dt * ratio).fract();
                if !ring {
                    let x = (2.0 * self.ph[1]).fract();
                    let t2 = if self.ph[1] >= 0.5 { 1.0 - x } else { x };
                    return 0.5 * (y + 2.0 * t2 - 1.0);
                }
            }
        }
        y
    }

    pub fn tick(&mut self) -> f32 {
        if !self.active {
            return 0.0;
        }
        let r = self.r;
        let sr = self.sr;
        let t = self.t;
        let fr = r.chip.frame_rate();
        // Pitch and the envelopes: smooth, or stepped a frame at a time.
        let (f, tone_env, noise_env) = if fr > 0.0 {
            if t >= self.frame_t {
                self.frame_t = t + 1.0 / fr;
                let q = |x: f32| (x * 15.0).round() / 15.0;
                self.frame_f = self.f_end + (self.f_start - self.f_end) * (-t / r.sweep.max(1e-4)).exp();
                // The triangle has no volume: on until it is cut.
                self.frame_tone = if r.wave == Wave::NesTri { (t < self.tone_decay * 0.4) as u8 as f32 } else { q(decay(t, self.tone_decay)) };
                self.frame_noise = q(decay(t, self.noise_decay));
                if let Wave::Tia(shape) = r.wave {
                    // The woodgrain chip's pitch: its divider's 32 steps.
                    let steps = match shape {
                        TiaShape::Square => 2.0,
                        TiaShape::Div31 => 31.0,
                        TiaShape::Poly4 => 15.0,
                    };
                    let div = (TIA_CLOCK / (self.frame_f * steps) - 1.0).round().clamp(0.0, 31.0);
                    self.frame_f = TIA_CLOCK / (div + 1.0) / steps;
                }
            }
            (self.frame_f, self.frame_tone, self.frame_noise)
        } else {
            (self.f_end + (self.f_start - self.f_end) * (-t / r.sweep.max(1e-4)).exp(), decay(t, self.tone_decay), decay(t, self.noise_decay))
        };
        let mut y = 0.0;
        if r.wave != Wave::None {
            let tone = if t < r.click {
                // A pulse click first.
                if (t * 2000.0).fract() < 0.5 {
                    1.0
                } else {
                    -1.0
                }
            } else {
                self.osc(f)
            };
            y += tone * r.tone_level * tone_env;
        }
        if r.noise != Noise::None {
            let n = self.noise();
            let mut env = noise_env;
            if r.bursts > 0 {
                let burst_end = r.bursts as f32 * r.spacing;
                env = if t < burst_end { (-(t % r.spacing) / (r.spacing / 3.0)).exp() } else { decay(t - burst_end, self.noise_decay) };
            }
            let mut nf = n * env * self.noise_amt;
            if self.filter != Filter::None {
                let (fc, q, hp) = match self.filter {
                    Filter::Hp(f, q) => (f, q, true),
                    Filter::Bp(f, q) => (f, q, false),
                    Filter::None => (1000.0, 0.0, false),
                };
                let g = (PI * fc.min(sr * 0.45) / sr).tan();
                let k = 1.414 - 1.394 * q;
                let a1 = 1.0 / (1.0 + g * (g + k));
                let v3 = nf - self.svf[1];
                let v1 = a1 * self.svf[0] + g * a1 * v3;
                let v2 = self.svf[1] + g * v1;
                self.svf[0] = (2.0 * v1 - self.svf[0]).clamp(-3.0, 3.0);
                self.svf[1] = 2.0 * v2 - self.svf[1];
                nf = if hp { nf - k * v1 - v2 } else { v1 };
            }
            y += nf;
        }
        let mut out = y * r.level * (0.35 + 0.65 * self.vel) * 0.6;
        if r.dpcm {
            // The delta sample channel at its fastest (33 kHz): a 7-bit
            // counter that can only step two up or two down.
            self.dpcm_acc += 33_144.0 / sr;
            while self.dpcm_acc >= 1.0 {
                self.dpcm_acc -= 1.0;
                let target = (out * 100.0 + 64.0).round() as i32;
                if target > self.dpcm_counter && self.dpcm_counter <= 125 {
                    self.dpcm_counter += 2;
                } else if target < self.dpcm_counter && self.dpcm_counter >= 2 {
                    self.dpcm_counter -= 2;
                }
                self.dpcm_out = (self.dpcm_counter - 64) as f32 / 100.0;
            }
            out = self.dpcm_out;
        }
        if r.one_bit {
            // One bit: the louder, the more often the speaker is on.
            let env = tone_env.max(noise_env * self.noise_amt.min(1.0));
            self.rng ^= self.rng << 13;
            self.rng ^= self.rng >> 17;
            self.rng ^= self.rng << 5;
            let gate = ((self.rng >> 8) as f32 / 16_777_216.0) < env.powf(0.6);
            out = if gate && y > 0.0 {
                r.level * 0.6
            } else if gate {
                -r.level * 0.6
            } else {
                0.0
            };
        }
        if r.chip == Chip::Arcade {
            // The 9-bit output.
            out = (out * 256.0).round() / 256.0;
        }
        // DC away.
        let k = 1.0 - TAU * 15.0 / sr;
        let h = out - self.dc_x + k * self.dc_y;
        self.dc_x = out;
        self.dc_y = h;
        self.t += 1.0 / sr;
        let longest = self.tone_decay.max(if r.bursts > 0 { self.noise_decay + r.bursts as f32 * r.spacing } else { self.noise_decay });
        if self.t > longest * 1.05 + 0.02 {
            self.active = false;
        }
        h
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [Kind; 44] = [
        Kind::ConsoleKick,
        Kind::ConsoleSampleKick,
        Kind::ConsoleSnare,
        Kind::ConsoleSampleSnare,
        Kind::ConsoleHat,
        Kind::ConsoleOpenHat,
        Kind::ConsoleTom,
        Kind::ConsoleCrash,
        Kind::HandheldKick,
        Kind::HandheldSnare,
        Kind::HandheldHat,
        Kind::HandheldOpenHat,
        Kind::HandheldTom,
        Kind::HandheldClap,
        Kind::MicroKick,
        Kind::MicroSnare,
        Kind::MicroTom,
        Kind::MicroClap,
        Kind::MicroRim,
        Kind::MicroHat,
        Kind::MicroOpenHat,
        Kind::MicroPunchKick,
        Kind::MicroPunchSnare,
        Kind::MicroPunchClap,
        Kind::MicroPunchRim,
        Kind::MicroPunchHat,
        Kind::MicroPunchOpenHat,
        Kind::MicroPunchTom,
        Kind::ArcadeKick,
        Kind::ArcadeSnare,
        Kind::ArcadeTom,
        Kind::ArcadeHat,
        Kind::ArcadeOpenHat,
        Kind::ArcadeCowbell,
        Kind::BeeperKick,
        Kind::BeeperSnare,
        Kind::BeeperHat,
        Kind::BeeperTom,
        Kind::WoodKick,
        Kind::WoodSnare,
        Kind::WoodHat,
        Kind::WoodTom,
        Kind::WoodZap,
        Kind::WoodBoom,
    ];

    fn knobs() -> ChipKnobs {
        ChipKnobs { tune: 1.0, decay: 1.0, tone: 0.5, sweep: 0.5, noise: 0.5 }
    }

    #[test]
    fn every_drum_sounds_and_ends() {
        for kind in ALL {
            let mut v = ChipVoice::default();
            v.trigger(kind, knobs(), 1.0, 44100.0);
            let out: Vec<f32> = (0..44100 * 3).map(|_| v.tick()).collect();
            let head = &out[..2205];
            let rms = (head.iter().map(|x| x * x).sum::<f32>() / head.len() as f32).sqrt();
            assert!(rms > 0.02, "{kind:?}: {rms}");
            assert!(out.iter().all(|x| x.is_finite() && x.abs() < 1.5), "{kind:?} {}", out.iter().fold(0.0f32, |m, x| m.max(x.abs())));
            assert!(!v.is_active(), "{kind:?} still on");
        }
    }

    #[test]
    fn the_sample_channel_only_steps_by_two() {
        let mut v = ChipVoice::default();
        v.trigger(Kind::ConsoleSampleKick, knobs(), 1.0, 33_144.0);
        let mut last = v.dpcm_counter;
        for _ in 0..5000 {
            v.tick();
            assert!((v.dpcm_counter - last).abs() <= 2);
            last = v.dpcm_counter;
        }
    }

    #[test]
    fn the_woodgrain_pitch_sits_on_its_divider() {
        let mut v = ChipVoice::default();
        v.trigger(Kind::WoodTom, knobs(), 1.0, 48000.0);
        for _ in 0..4800 {
            v.tick();
            let div = TIA_CLOCK / (v.frame_f * 2.0) - 1.0;
            assert!((div - div.round()).abs() < 1e-3, "{div}");
        }
    }

    #[test]
    fn the_console_steps_its_volume_in_sixteen() {
        let mut v = ChipVoice::default();
        v.trigger(Kind::ConsoleSnare, knobs(), 1.0, 48000.0);
        for _ in 0..9600 {
            v.tick();
            assert!((v.frame_noise * 15.0 - (v.frame_noise * 15.0).round()).abs() < 1e-4);
        }
    }
}

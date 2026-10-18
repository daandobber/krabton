//! Chip FM: the four-operator FM sound chips of the 16-bit consoles and the
//! arcade boards, played as an instrument. The chips themselves are a Rust
//! port of their emulation in ymfm by Aaron Giles
//! (<https://github.com/aaronsgiles/ymfm>): the log-sine and exponent
//! tables taken from the dies, the envelope generator with its rate tables
//! and the attack's glitches, detune, the LFO, the eight algorithms, the
//! operator feedback, the SSG envelopes; "Console" is the six-channel chip
//! of the 16-bit consoles with its 9-bit DAC and the step it makes around
//! zero (the "ladder"), "Arcade" the eight-channel arcade chip with its LFO
//! waveforms, second detune and floating-point DAC. Each runs at its own
//! rate and is resampled.
//!
//! Krabton plays it like a synth: one patch on every channel, a note on
//! the channel free longest, velocity on the operators that sound, a
//! pitch drop for drums, fine tune and pitch bend written to its
//! frequency registers as a driver would.
//!
//! ymfm's license:
//!
//! BSD 3-Clause License. Copyright (c) 2021, Aaron Giles. All rights reserved.
//!
//! Redistribution and use in source and binary forms, with or without
//! modification, are permitted provided that the following conditions are met:
//!
//! 1. Redistributions of source code must retain the above copyright notice,
//!    this list of conditions and the following disclaimer.
//! 2. Redistributions in binary form must reproduce the above copyright
//!    notice, this list of conditions and the following disclaimer in the
//!    documentation and/or other materials provided with the distribution.
//! 3. Neither the name of the copyright holder nor the names of its
//!    contributors may be used to endorse or promote products derived from
//!    this software without specific prior written permission.
//!
//! THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
//! AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
//! IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
//! ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE
//! LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR
//! CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF
//! SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS
//! INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN
//! CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE)
//! ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE
//! POSSIBILITY OF SUCH DAMAGE.

use crate::params::{ParamSpec, Unit};

// ---------------------------------------------------------------- parameters

pub const CHIP: usize = 0;
pub const ALGORITHM: usize = 1;
pub const FEEDBACK: usize = 2;
pub const LFO_RATE: usize = 3;
pub const LFO_WAVE: usize = 4;
pub const AMS: usize = 5;
pub const PMS: usize = 6;
pub const AM_DEPTH: usize = 7;
pub const PM_DEPTH: usize = 8;
/// Operator `k`'s (0 to 3, in the algorithm's order) parameter.
pub const fn op(k: usize, i: usize) -> usize {
    9 + k * 12 + i
}
pub const AR: usize = 0;
pub const DR: usize = 1;
pub const SR: usize = 2;
pub const RR: usize = 3;
pub const SL: usize = 4;
pub const TL: usize = 5;
pub const KS: usize = 6;
pub const MUL: usize = 7;
pub const DT: usize = 8;
pub const DT2: usize = 9;
pub const AM: usize = 10;
pub const SSG: usize = 11;
pub const TRANSPOSE: usize = 57;
pub const FINE: usize = 58;
pub const VELOCITY: usize = 59;
pub const DROP: usize = 60;
pub const DROP_TIME: usize = 61;
pub const PAN: usize = 62;
pub const VOLUME: usize = 63;

pub const CHIPS: &[&str] = &["Console", "Arcade"];
pub const LFO_WAVES: &[&str] = &["Saw", "Square", "Triangle", "Noise"];
pub const SSG_MODES: &[&str] = &["Off", "Repeat", "Once Low", "Alternate", "Once High", "Inv Repeat", "Inv Once Low", "Inv Alternate", "Inv Once High"];
pub const OFF_ON: &[&str] = &["Off", "On"];
pub const PANS: &[&str] = &["Both", "Left", "Right"];

macro_rules! op_specs {
    ($g:literal, $n:literal) => {
        [
            ParamSpec::int(concat!("Op", $n, " Attack"), $g, 0.0, 31.0, 31.0, Unit::None),
            ParamSpec::int(concat!("Op", $n, " Decay"), $g, 0.0, 31.0, 10.0, Unit::None),
            ParamSpec::int(concat!("Op", $n, " Sustain Rate"), $g, 0.0, 31.0, 0.0, Unit::None),
            ParamSpec::int(concat!("Op", $n, " Release"), $g, 0.0, 15.0, 7.0, Unit::None),
            ParamSpec::int(concat!("Op", $n, " Sustain Level"), $g, 0.0, 15.0, 2.0, Unit::None),
            ParamSpec::int(concat!("Op", $n, " Level"), $g, 0.0, 127.0, 127.0, Unit::None),
            ParamSpec::int(concat!("Op", $n, " Key Scale"), $g, 0.0, 3.0, 0.0, Unit::None),
            ParamSpec::int(concat!("Op", $n, " Multiple"), $g, 0.0, 15.0, 1.0, Unit::None),
            ParamSpec::int(concat!("Op", $n, " Detune"), $g, -3.0, 3.0, 0.0, Unit::None),
            ParamSpec::int(concat!("Op", $n, " Detune 2"), $g, 0.0, 3.0, 0.0, Unit::None),
            ParamSpec::choice(concat!("Op", $n, " AM"), $g, OFF_ON, 0),
            ParamSpec::choice(concat!("Op", $n, " SSG-EG"), $g, SSG_MODES, 0),
        ]
    };
}

const OP1: [ParamSpec; 12] = op_specs!("Op 1", "1");
const OP2: [ParamSpec; 12] = op_specs!("Op 2", "2");
const OP3: [ParamSpec; 12] = op_specs!("Op 3", "3");
const OP4: [ParamSpec; 12] = op_specs!("Op 4", "4");

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::choice("Chip", "Chip", CHIPS, 0),
    ParamSpec::int("Algorithm", "Chip", 0.0, 7.0, 4.0, Unit::None),
    ParamSpec::int("Feedback", "Chip", 0.0, 7.0, 0.0, Unit::None),
    ParamSpec::int("LFO Rate", "LFO", 0.0, 8.0, 0.0, Unit::None),
    ParamSpec::choice("LFO Wave", "LFO", LFO_WAVES, 2),
    ParamSpec::int("AM Sens", "LFO", 0.0, 3.0, 0.0, Unit::None),
    ParamSpec::int("PM Sens", "LFO", 0.0, 7.0, 0.0, Unit::None),
    ParamSpec::int("AM Depth", "LFO", 0.0, 127.0, 64.0, Unit::None),
    ParamSpec::int("PM Depth", "LFO", 0.0, 127.0, 64.0, Unit::None),
    OP1[0], OP1[1], OP1[2], OP1[3], OP1[4], OP1[5], OP1[6], OP1[7], OP1[8], OP1[9], OP1[10], OP1[11],
    OP2[0], OP2[1], OP2[2], OP2[3], OP2[4], OP2[5], OP2[6], OP2[7], OP2[8], OP2[9], OP2[10], OP2[11],
    OP3[0], OP3[1], OP3[2], OP3[3], OP3[4], OP3[5], OP3[6], OP3[7], OP3[8], OP3[9], OP3[10], OP3[11],
    OP4[0], OP4[1], OP4[2], OP4[3], OP4[4], OP4[5], OP4[6], OP4[7], OP4[8], OP4[9], OP4[10], OP4[11],
    ParamSpec::int("Transpose", "Play", -24.0, 24.0, 0.0, Unit::Semi),
    ParamSpec::lin("Fine", "Play", -100.0, 100.0, 0.0, Unit::Cents),
    ParamSpec::lin("Velocity", "Play", 0.0, 1.0, 0.6, Unit::Percent),
    ParamSpec::lin("Pitch Drop", "Play", 0.0, 48.0, 0.0, Unit::Semi),
    ParamSpec::log("Drop Time", "Play", 5.0, 1000.0, 60.0, Unit::Ms),
    ParamSpec::choice("Pan", "Play", PANS, 0),
    ParamSpec::lin("Volume", "Play", -24.0, 6.0, 0.0, Unit::Db),
];

pub fn preset_names() -> &'static [&'static str] {
    &["Init", "FM Bass", "Slap Bass", "E.Piano", "Brass", "Bell", "Lead", "Strings", "Organ", "Clav", "Kick", "Snare", "Tom", "Metal Pulse", "Arcade Brass", "Arcade Bass", "Arcade Strings", "Arcade Lead"]
}

/// Operator settings: AR, DR, SR, RR, SL, TL, KS, MUL, DT.
type Op = [f32; 9];

pub fn preset(name: &str) -> Vec<f32> {
    let mut p = crate::params::defaults(SPECS);
    // Init: one sine on the fourth operator.
    for k in 0..4 {
        p[op(k, TL)] = if k == 3 { 0.0 } else { 127.0 };
    }
    let (chip, alg, fb, lfo, pms, ams, ops, extra): (f32, f32, f32, f32, f32, f32, [Op; 4], Vec<(usize, f32)>) = match name {
        "FM Bass" => (0.0, 4.0, 5.0, 0.0, 0.0, 0.0, [[31.0, 14.0, 2.0, 7.0, 3.0, 24.0, 1.0, 1.0, 0.0], [31.0, 10.0, 3.0, 8.0, 2.0, 4.0, 1.0, 1.0, 0.0], [31.0, 16.0, 0.0, 7.0, 5.0, 40.0, 1.0, 3.0, 0.0], [31.0, 12.0, 2.0, 8.0, 3.0, 12.0, 1.0, 1.0, 1.0]], vec![(TRANSPOSE, -12.0)]),
        "Slap Bass" => (0.0, 0.0, 6.0, 0.0, 0.0, 0.0, [[31.0, 18.0, 0.0, 9.0, 8.0, 28.0, 2.0, 1.0, 0.0], [31.0, 16.0, 4.0, 9.0, 6.0, 36.0, 1.0, 3.0, 0.0], [31.0, 14.0, 2.0, 9.0, 4.0, 30.0, 1.0, 1.0, 0.0], [31.0, 11.0, 3.0, 9.0, 3.0, 2.0, 1.0, 1.0, 0.0]], vec![(TRANSPOSE, -12.0)]),
        "E.Piano" => (0.0, 4.0, 0.0, 0.0, 0.0, 0.0, [[31.0, 18.0, 0.0, 6.0, 15.0, 40.0, 1.0, 14.0, 0.0], [31.0, 8.0, 2.0, 6.0, 4.0, 6.0, 1.0, 1.0, 0.0], [31.0, 10.0, 2.0, 6.0, 6.0, 30.0, 1.0, 1.0, 1.0], [31.0, 6.0, 2.0, 6.0, 3.0, 8.0, 1.0, 1.0, -1.0]], vec![(VELOCITY, 0.9)]),
        "Brass" => (0.0, 4.0, 4.0, 4.0, 2.0, 0.0, [[18.0, 6.0, 0.0, 7.0, 1.0, 18.0, 0.0, 1.0, 0.0], [20.0, 4.0, 0.0, 7.0, 1.0, 2.0, 0.0, 1.0, 1.0], [16.0, 6.0, 0.0, 7.0, 1.0, 22.0, 0.0, 1.0, -1.0], [20.0, 4.0, 0.0, 7.0, 1.0, 6.0, 0.0, 1.0, 2.0]], vec![]),
        "Bell" => (0.0, 4.0, 0.0, 0.0, 0.0, 0.0, [[31.0, 8.0, 4.0, 4.0, 15.0, 28.0, 1.0, 7.0, 2.0], [31.0, 6.0, 4.0, 4.0, 15.0, 0.0, 1.0, 1.0, 0.0], [31.0, 10.0, 4.0, 4.0, 15.0, 34.0, 1.0, 3.0, -3.0], [31.0, 7.0, 4.0, 4.0, 15.0, 8.0, 1.0, 2.0, 1.0]], vec![]),
        "Lead" => (0.0, 4.0, 7.0, 5.0, 3.0, 0.0, [[31.0, 4.0, 0.0, 7.0, 1.0, 26.0, 0.0, 1.0, 0.0], [31.0, 2.0, 0.0, 7.0, 1.0, 0.0, 0.0, 1.0, 0.0], [31.0, 4.0, 0.0, 7.0, 1.0, 30.0, 0.0, 2.0, 3.0], [31.0, 2.0, 0.0, 7.0, 1.0, 6.0, 0.0, 1.0, -2.0]], vec![]),
        "Strings" => (0.0, 5.0, 6.0, 4.0, 2.0, 0.0, [[14.0, 0.0, 0.0, 5.0, 0.0, 32.0, 0.0, 1.0, 0.0], [14.0, 0.0, 0.0, 5.0, 0.0, 8.0, 0.0, 1.0, 1.0], [13.0, 0.0, 0.0, 5.0, 0.0, 12.0, 0.0, 2.0, -1.0], [14.0, 0.0, 0.0, 5.0, 0.0, 10.0, 0.0, 1.0, 3.0]], vec![]),
        "Organ" => (0.0, 7.0, 3.0, 0.0, 0.0, 0.0, [[31.0, 0.0, 0.0, 9.0, 0.0, 10.0, 0.0, 1.0, 0.0], [31.0, 0.0, 0.0, 9.0, 0.0, 14.0, 0.0, 2.0, 0.0], [31.0, 0.0, 0.0, 9.0, 0.0, 18.0, 0.0, 4.0, 0.0], [31.0, 0.0, 0.0, 9.0, 0.0, 24.0, 0.0, 8.0, 0.0]], vec![]),
        "Clav" => (0.0, 4.0, 2.0, 0.0, 0.0, 0.0, [[31.0, 16.0, 4.0, 10.0, 6.0, 22.0, 2.0, 3.0, 0.0], [31.0, 12.0, 4.0, 10.0, 6.0, 4.0, 1.0, 1.0, 0.0], [31.0, 18.0, 4.0, 10.0, 8.0, 34.0, 2.0, 5.0, 0.0], [31.0, 14.0, 4.0, 10.0, 6.0, 10.0, 1.0, 2.0, 0.0]], vec![(VELOCITY, 0.9)]),
        "Kick" => (0.0, 4.0, 0.0, 0.0, 0.0, 0.0, [[31.0, 20.0, 0.0, 12.0, 15.0, 30.0, 0.0, 1.0, 0.0], [31.0, 15.0, 0.0, 12.0, 15.0, 0.0, 0.0, 1.0, 0.0], [31.0, 0.0, 0.0, 15.0, 15.0, 127.0, 0.0, 1.0, 0.0], [31.0, 0.0, 0.0, 15.0, 15.0, 127.0, 0.0, 1.0, 0.0]], vec![(DROP, 24.0), (DROP_TIME, 60.0), (TRANSPOSE, -12.0)]),
        "Snare" => (0.0, 7.0, 7.0, 0.0, 0.0, 0.0, [[31.0, 16.0, 0.0, 12.0, 15.0, 4.0, 0.0, 15.0, 0.0], [31.0, 15.0, 0.0, 12.0, 15.0, 6.0, 0.0, 1.0, 0.0], [31.0, 0.0, 0.0, 15.0, 15.0, 127.0, 0.0, 1.0, 0.0], [31.0, 0.0, 0.0, 15.0, 15.0, 127.0, 0.0, 1.0, 0.0]], vec![(DROP, 12.0), (DROP_TIME, 40.0)]),
        "Tom" => (0.0, 4.0, 3.0, 0.0, 0.0, 0.0, [[31.0, 14.0, 0.0, 10.0, 15.0, 34.0, 0.0, 1.0, 0.0], [31.0, 12.0, 0.0, 10.0, 15.0, 0.0, 0.0, 1.0, 0.0], [31.0, 0.0, 0.0, 15.0, 15.0, 127.0, 0.0, 1.0, 0.0], [31.0, 0.0, 0.0, 15.0, 15.0, 127.0, 0.0, 1.0, 0.0]], vec![(DROP, 12.0), (DROP_TIME, 150.0)]),
        "Metal Pulse" => (0.0, 4.0, 4.0, 0.0, 0.0, 0.0, [[31.0, 20.0, 0.0, 7.0, 4.0, 20.0, 0.0, 5.0, 1.0], [31.0, 6.0, 0.0, 7.0, 2.0, 2.0, 0.0, 1.0, 0.0], [31.0, 22.0, 0.0, 7.0, 6.0, 24.0, 0.0, 7.0, -2.0], [31.0, 6.0, 0.0, 7.0, 2.0, 8.0, 0.0, 2.0, 0.0]], vec![(op(0, SSG), 1.0), (op(2, SSG), 3.0)]),
        "Arcade Brass" => (1.0, 4.0, 4.0, 5.0, 2.0, 0.0, [[18.0, 6.0, 0.0, 7.0, 1.0, 18.0, 0.0, 1.0, 0.0], [20.0, 4.0, 0.0, 7.0, 1.0, 2.0, 0.0, 1.0, 1.0], [16.0, 6.0, 0.0, 7.0, 1.0, 22.0, 0.0, 1.0, -1.0], [20.0, 4.0, 0.0, 7.0, 1.0, 6.0, 0.0, 1.0, 2.0]], vec![(PM_DEPTH, 30.0)]),
        "Arcade Bass" => (1.0, 0.0, 6.0, 0.0, 0.0, 0.0, [[31.0, 16.0, 0.0, 9.0, 6.0, 26.0, 2.0, 1.0, 0.0], [31.0, 14.0, 2.0, 9.0, 5.0, 34.0, 1.0, 2.0, 0.0], [31.0, 12.0, 2.0, 9.0, 4.0, 30.0, 1.0, 1.0, 0.0], [31.0, 10.0, 3.0, 9.0, 3.0, 0.0, 1.0, 1.0, 0.0]], vec![(TRANSPOSE, -12.0), (op(1, DT2), 1.0)]),
        "Arcade Strings" => (1.0, 5.0, 6.0, 5.0, 3.0, 0.0, [[14.0, 0.0, 0.0, 5.0, 0.0, 32.0, 0.0, 1.0, 0.0], [14.0, 0.0, 0.0, 5.0, 0.0, 8.0, 0.0, 1.0, 1.0], [13.0, 0.0, 0.0, 5.0, 0.0, 12.0, 0.0, 2.0, -1.0], [14.0, 0.0, 0.0, 5.0, 0.0, 10.0, 0.0, 1.0, 3.0]], vec![(PM_DEPTH, 20.0), (LFO_WAVE, 2.0)]),
        "Arcade Lead" => (1.0, 4.0, 7.0, 6.0, 3.0, 1.0, [[31.0, 4.0, 0.0, 7.0, 1.0, 24.0, 0.0, 1.0, 0.0], [31.0, 2.0, 0.0, 7.0, 1.0, 0.0, 0.0, 1.0, 0.0], [31.0, 4.0, 0.0, 7.0, 1.0, 28.0, 0.0, 3.0, 0.0], [31.0, 2.0, 0.0, 7.0, 1.0, 6.0, 0.0, 1.0, 0.0]], vec![(PM_DEPTH, 40.0), (op(2, DT2), 2.0)]),
        _ => return p,
    };
    p[CHIP] = chip;
    p[ALGORITHM] = alg;
    p[FEEDBACK] = fb;
    p[LFO_RATE] = lfo;
    p[PMS] = pms;
    p[AMS] = ams;
    for (k, o) in ops.iter().enumerate() {
        for (i, &v) in [AR, DR, SR, RR, SL, TL, KS, MUL, DT].iter().zip(o.iter()) {
            p[op(k, *i)] = v;
        }
    }
    for (i, v) in extra {
        p[i] = v;
    }
    p
}

fn get(p: &[f32], i: usize) -> f32 {
    p.get(i).copied().unwrap_or(SPECS[i].default)
}

fn geti(p: &[f32], i: usize) -> u32 {
    get(p, i).round().clamp(SPECS[i].min, SPECS[i].max) as i32 as u32
}

// ---------------------------------------------------------------- tables

fn bitfield(value: u32, start: u32, length: u32) -> u32 {
    (value >> start) & ((1 << length) - 1)
}

/// The absolute value of the sine of a 10-bit phase, as a 4.8 attenuation
/// (the OPN die's table of a quarter wave).
fn abs_sin_attenuation(input: u32) -> u32 {
    const T: [u16; 256] = [
        0x859, 0x6c3, 0x607, 0x58b, 0x52e, 0x4e4, 0x4a6, 0x471, 0x443, 0x41a, 0x3f5, 0x3d3, 0x3b5, 0x398, 0x37e, 0x365, 0x34e, 0x339, 0x324, 0x311, 0x2ff, 0x2ed, 0x2dc, 0x2cd, 0x2bd, 0x2af, 0x2a0, 0x293, 0x286, 0x279, 0x26d, 0x261,
        0x256, 0x24b, 0x240, 0x236, 0x22c, 0x222, 0x218, 0x20f, 0x206, 0x1fd, 0x1f5, 0x1ec, 0x1e4, 0x1dc, 0x1d4, 0x1cd, 0x1c5, 0x1be, 0x1b7, 0x1b0, 0x1a9, 0x1a2, 0x19b, 0x195, 0x18f, 0x188, 0x182, 0x17c, 0x177, 0x171, 0x16b, 0x166,
        0x160, 0x15b, 0x155, 0x150, 0x14b, 0x146, 0x141, 0x13c, 0x137, 0x133, 0x12e, 0x129, 0x125, 0x121, 0x11c, 0x118, 0x114, 0x10f, 0x10b, 0x107, 0x103, 0x0ff, 0x0fb, 0x0f8, 0x0f4, 0x0f0, 0x0ec, 0x0e9, 0x0e5, 0x0e2, 0x0de, 0x0db,
        0x0d7, 0x0d4, 0x0d1, 0x0cd, 0x0ca, 0x0c7, 0x0c4, 0x0c1, 0x0be, 0x0bb, 0x0b8, 0x0b5, 0x0b2, 0x0af, 0x0ac, 0x0a9, 0x0a7, 0x0a4, 0x0a1, 0x09f, 0x09c, 0x099, 0x097, 0x094, 0x092, 0x08f, 0x08d, 0x08a, 0x088, 0x086, 0x083, 0x081,
        0x07f, 0x07d, 0x07a, 0x078, 0x076, 0x074, 0x072, 0x070, 0x06e, 0x06c, 0x06a, 0x068, 0x066, 0x064, 0x062, 0x060, 0x05e, 0x05c, 0x05b, 0x059, 0x057, 0x055, 0x053, 0x052, 0x050, 0x04e, 0x04d, 0x04b, 0x04a, 0x048, 0x046, 0x045,
        0x043, 0x042, 0x040, 0x03f, 0x03e, 0x03c, 0x03b, 0x039, 0x038, 0x037, 0x035, 0x034, 0x033, 0x031, 0x030, 0x02f, 0x02e, 0x02d, 0x02b, 0x02a, 0x029, 0x028, 0x027, 0x026, 0x025, 0x024, 0x023, 0x022, 0x021, 0x020, 0x01f, 0x01e,
        0x01d, 0x01c, 0x01b, 0x01a, 0x019, 0x018, 0x017, 0x017, 0x016, 0x015, 0x014, 0x014, 0x013, 0x012, 0x011, 0x011, 0x010, 0x00f, 0x00f, 0x00e, 0x00d, 0x00d, 0x00c, 0x00c, 0x00b, 0x00a, 0x00a, 0x009, 0x009, 0x008, 0x008, 0x007,
        0x007, 0x007, 0x006, 0x006, 0x005, 0x005, 0x005, 0x004, 0x004, 0x004, 0x003, 0x003, 0x003, 0x002, 0x002, 0x002, 0x002, 0x001, 0x001, 0x001, 0x001, 0x001, 0x001, 0x001, 0x000, 0x000, 0x000, 0x000, 0x000, 0x000, 0x000, 0x000,
    ];
    let input = if bitfield(input, 8, 1) != 0 { !input } else { input };
    T[(input & 0xff) as usize] as u32
}

/// A 5.8 logarithmic attenuation as a 13-bit linear volume (the die's power table).
fn attenuation_to_volume(input: u32) -> u32 {
    const M: [u16; 256] = [
        0x3fa, 0x3f5, 0x3ef, 0x3ea, 0x3e4, 0x3df, 0x3da, 0x3d4, 0x3cf, 0x3c9, 0x3c4, 0x3bf, 0x3b9, 0x3b4, 0x3ae, 0x3a9, 0x3a4, 0x39f, 0x399, 0x394, 0x38f, 0x38a, 0x384, 0x37f, 0x37a, 0x375, 0x370, 0x36a, 0x365, 0x360, 0x35b, 0x356,
        0x351, 0x34c, 0x347, 0x342, 0x33d, 0x338, 0x333, 0x32e, 0x329, 0x324, 0x31f, 0x31a, 0x315, 0x310, 0x30b, 0x306, 0x302, 0x2fd, 0x2f8, 0x2f3, 0x2ee, 0x2e9, 0x2e5, 0x2e0, 0x2db, 0x2d6, 0x2d2, 0x2cd, 0x2c8, 0x2c4, 0x2bf, 0x2ba,
        0x2b5, 0x2b1, 0x2ac, 0x2a8, 0x2a3, 0x29e, 0x29a, 0x295, 0x291, 0x28c, 0x288, 0x283, 0x27f, 0x27a, 0x276, 0x271, 0x26d, 0x268, 0x264, 0x25f, 0x25b, 0x257, 0x252, 0x24e, 0x249, 0x245, 0x241, 0x23c, 0x238, 0x234, 0x230, 0x22b,
        0x227, 0x223, 0x21e, 0x21a, 0x216, 0x212, 0x20e, 0x209, 0x205, 0x201, 0x1fd, 0x1f9, 0x1f5, 0x1f0, 0x1ec, 0x1e8, 0x1e4, 0x1e0, 0x1dc, 0x1d8, 0x1d4, 0x1d0, 0x1cc, 0x1c8, 0x1c4, 0x1c0, 0x1bc, 0x1b8, 0x1b4, 0x1b0, 0x1ac, 0x1a8,
        0x1a4, 0x1a0, 0x19c, 0x199, 0x195, 0x191, 0x18d, 0x189, 0x185, 0x181, 0x17e, 0x17a, 0x176, 0x172, 0x16f, 0x16b, 0x167, 0x163, 0x160, 0x15c, 0x158, 0x154, 0x151, 0x14d, 0x149, 0x146, 0x142, 0x13e, 0x13b, 0x137, 0x134, 0x130,
        0x12c, 0x129, 0x125, 0x122, 0x11e, 0x11b, 0x117, 0x114, 0x110, 0x10c, 0x109, 0x106, 0x102, 0x0ff, 0x0fb, 0x0f8, 0x0f4, 0x0f1, 0x0ed, 0x0ea, 0x0e7, 0x0e3, 0x0e0, 0x0dc, 0x0d9, 0x0d6, 0x0d2, 0x0cf, 0x0cc, 0x0c8, 0x0c5, 0x0c2,
        0x0be, 0x0bb, 0x0b8, 0x0b5, 0x0b1, 0x0ae, 0x0ab, 0x0a8, 0x0a4, 0x0a1, 0x09e, 0x09b, 0x098, 0x094, 0x091, 0x08e, 0x08b, 0x088, 0x085, 0x082, 0x07e, 0x07b, 0x078, 0x075, 0x072, 0x06f, 0x06c, 0x069, 0x066, 0x063, 0x060, 0x05d,
        0x05a, 0x057, 0x054, 0x051, 0x04e, 0x04b, 0x048, 0x045, 0x042, 0x03f, 0x03c, 0x039, 0x036, 0x033, 0x030, 0x02d, 0x02a, 0x028, 0x025, 0x022, 0x01f, 0x01c, 0x019, 0x016, 0x014, 0x011, 0x00e, 0x00b, 0x008, 0x006, 0x003, 0x000,
    ];
    let shift = input >> 8;
    if shift >= 32 {
        return 0;
    }
    (((M[(input & 0xff) as usize] as u32) | 0x400) << 2) >> shift
}

/// The 4-bit envelope increment for a 6-bit rate at a 3-bit step.
fn attenuation_increment(rate: u32, index: u32) -> u32 {
    const T: [u32; 64] = [
        0x00000000, 0x00000000, 0x10101010, 0x10101010, 0x10101010, 0x10101010, 0x11101110, 0x11101110, 0x10101010, 0x10111010, 0x11101110, 0x11111110, 0x10101010, 0x10111010, 0x11101110, 0x11111110, 0x10101010, 0x10111010, 0x11101110, 0x11111110, 0x10101010, 0x10111010, 0x11101110, 0x11111110, 0x10101010, 0x10111010, 0x11101110, 0x11111110,
        0x10101010, 0x10111010, 0x11101110, 0x11111110, 0x10101010, 0x10111010, 0x11101110, 0x11111110, 0x10101010, 0x10111010, 0x11101110, 0x11111110, 0x10101010, 0x10111010, 0x11101110, 0x11111110, 0x10101010, 0x10111010, 0x11101110, 0x11111110, 0x11111111, 0x21112111, 0x21212121, 0x22212221, 0x22222222, 0x42224222,
        0x42424242, 0x44424442, 0x44444444, 0x84448444, 0x84848484, 0x88848884, 0x88888888, 0x88888888, 0x88888888, 0x88888888,
    ];
    bitfield(T[rate as usize], 4 * index, 4)
}

fn detune_adjustment(detune: u32, keycode: u32) -> i32 {
    const T: [[u8; 4]; 32] = [
        [0, 0, 1, 2], [0, 0, 1, 2], [0, 0, 1, 2], [0, 0, 1, 2], [0, 1, 2, 2], [0, 1, 2, 3], [0, 1, 2, 3], [0, 1, 2, 3], [0, 1, 2, 4], [0, 1, 3, 4], [0, 1, 3, 4], [0, 1, 3, 5], [0, 2, 4, 5], [0, 2, 4, 6], [0, 2, 4, 6], [0, 2, 5, 7],
        [0, 2, 5, 8], [0, 3, 6, 8], [0, 3, 6, 9], [0, 3, 7, 10], [0, 4, 8, 11], [0, 4, 8, 12], [0, 4, 9, 13], [0, 5, 10, 14], [0, 5, 11, 16], [0, 6, 12, 17], [0, 6, 13, 19], [0, 7, 14, 20], [0, 8, 16, 22], [0, 8, 16, 22], [0, 8, 16, 22], [0, 8, 16, 22],
    ];
    let r = T[keycode as usize & 31][(detune & 3) as usize] as i32;
    if bitfield(detune, 2, 1) != 0 {
        -r
    } else {
        r
    }
}

/// The arcade chip's phase steps: 12 notes of 64 steps from C#, as
/// measured on the chip by David Viens (used in ymfm with his permission).
const OPM_PHASE_STEP: [u32; 768] = [
    41568, 41600, 41632, 41664, 41696, 41728, 41760, 41792, 41856, 41888, 41920, 41952, 42016, 42048, 42080, 42112,
    42176, 42208, 42240, 42272, 42304, 42336, 42368, 42400, 42464, 42496, 42528, 42560, 42624, 42656, 42688, 42720,
    42784, 42816, 42848, 42880, 42912, 42944, 42976, 43008, 43072, 43104, 43136, 43168, 43232, 43264, 43296, 43328,
    43392, 43424, 43456, 43488, 43552, 43584, 43616, 43648, 43712, 43744, 43776, 43808, 43872, 43904, 43936, 43968,
    44032, 44064, 44096, 44128, 44192, 44224, 44256, 44288, 44352, 44384, 44416, 44448, 44512, 44544, 44576, 44608,
    44672, 44704, 44736, 44768, 44832, 44864, 44896, 44928, 44992, 45024, 45056, 45088, 45152, 45184, 45216, 45248,
    45312, 45344, 45376, 45408, 45472, 45504, 45536, 45568, 45632, 45664, 45728, 45760, 45792, 45824, 45888, 45920,
    45984, 46016, 46048, 46080, 46144, 46176, 46208, 46240, 46304, 46336, 46368, 46400, 46464, 46496, 46528, 46560,
    46656, 46688, 46720, 46752, 46816, 46848, 46880, 46912, 46976, 47008, 47072, 47104, 47136, 47168, 47232, 47264,
    47328, 47360, 47392, 47424, 47488, 47520, 47552, 47584, 47648, 47680, 47744, 47776, 47808, 47840, 47904, 47936,
    48032, 48064, 48096, 48128, 48192, 48224, 48288, 48320, 48384, 48416, 48448, 48480, 48544, 48576, 48640, 48672,
    48736, 48768, 48800, 48832, 48896, 48928, 48992, 49024, 49088, 49120, 49152, 49184, 49248, 49280, 49344, 49376,
    49440, 49472, 49504, 49536, 49600, 49632, 49696, 49728, 49792, 49824, 49856, 49888, 49952, 49984, 50048, 50080,
    50144, 50176, 50208, 50240, 50304, 50336, 50400, 50432, 50496, 50528, 50560, 50592, 50656, 50688, 50752, 50784,
    50880, 50912, 50944, 50976, 51040, 51072, 51136, 51168, 51232, 51264, 51328, 51360, 51424, 51456, 51488, 51520,
    51616, 51648, 51680, 51712, 51776, 51808, 51872, 51904, 51968, 52000, 52064, 52096, 52160, 52192, 52224, 52256,
    52384, 52416, 52448, 52480, 52544, 52576, 52640, 52672, 52736, 52768, 52832, 52864, 52928, 52960, 52992, 53024,
    53120, 53152, 53216, 53248, 53312, 53344, 53408, 53440, 53504, 53536, 53600, 53632, 53696, 53728, 53792, 53824,
    53920, 53952, 54016, 54048, 54112, 54144, 54208, 54240, 54304, 54336, 54400, 54432, 54496, 54528, 54592, 54624,
    54688, 54720, 54784, 54816, 54880, 54912, 54976, 55008, 55072, 55104, 55168, 55200, 55264, 55296, 55360, 55392,
    55488, 55520, 55584, 55616, 55680, 55712, 55776, 55808, 55872, 55936, 55968, 56032, 56064, 56128, 56160, 56224,
    56288, 56320, 56384, 56416, 56480, 56512, 56576, 56608, 56672, 56736, 56768, 56832, 56864, 56928, 56960, 57024,
    57120, 57152, 57216, 57248, 57312, 57376, 57408, 57472, 57536, 57568, 57632, 57664, 57728, 57792, 57824, 57888,
    57952, 57984, 58048, 58080, 58144, 58208, 58240, 58304, 58368, 58400, 58464, 58496, 58560, 58624, 58656, 58720,
    58784, 58816, 58880, 58912, 58976, 59040, 59072, 59136, 59200, 59232, 59296, 59328, 59392, 59456, 59488, 59552,
    59648, 59680, 59744, 59776, 59840, 59904, 59936, 60000, 60064, 60128, 60160, 60224, 60288, 60320, 60384, 60416,
    60512, 60544, 60608, 60640, 60704, 60768, 60800, 60864, 60928, 60992, 61024, 61088, 61152, 61184, 61248, 61280,
    61376, 61408, 61472, 61536, 61600, 61632, 61696, 61760, 61824, 61856, 61920, 61984, 62048, 62080, 62144, 62208,
    62272, 62304, 62368, 62432, 62496, 62528, 62592, 62656, 62720, 62752, 62816, 62880, 62944, 62976, 63040, 63104,
    63200, 63232, 63296, 63360, 63424, 63456, 63520, 63584, 63648, 63680, 63744, 63808, 63872, 63904, 63968, 64032,
    64096, 64128, 64192, 64256, 64320, 64352, 64416, 64480, 64544, 64608, 64672, 64704, 64768, 64832, 64896, 64928,
    65024, 65056, 65120, 65184, 65248, 65312, 65376, 65408, 65504, 65536, 65600, 65664, 65728, 65792, 65856, 65888,
    65984, 66016, 66080, 66144, 66208, 66272, 66336, 66368, 66464, 66496, 66560, 66624, 66688, 66752, 66816, 66848,
    66944, 66976, 67040, 67104, 67168, 67232, 67296, 67328, 67424, 67456, 67520, 67584, 67648, 67712, 67776, 67808,
    67904, 67936, 68000, 68064, 68128, 68192, 68256, 68288, 68384, 68448, 68512, 68544, 68640, 68672, 68736, 68800,
    68896, 68928, 68992, 69056, 69120, 69184, 69248, 69280, 69376, 69440, 69504, 69536, 69632, 69664, 69728, 69792,
    69920, 69952, 70016, 70080, 70144, 70208, 70272, 70304, 70400, 70464, 70528, 70560, 70656, 70688, 70752, 70816,
    70912, 70976, 71040, 71104, 71136, 71232, 71264, 71360, 71424, 71488, 71552, 71616, 71648, 71744, 71776, 71872,
    71968, 72032, 72096, 72160, 72192, 72288, 72320, 72416, 72480, 72544, 72608, 72672, 72704, 72800, 72832, 72928,
    72992, 73056, 73120, 73184, 73216, 73312, 73344, 73440, 73504, 73568, 73632, 73696, 73728, 73824, 73856, 73952,
    74080, 74144, 74208, 74272, 74304, 74400, 74432, 74528, 74592, 74656, 74720, 74784, 74816, 74912, 74944, 75040,
    75136, 75200, 75264, 75328, 75360, 75456, 75488, 75584, 75648, 75712, 75776, 75840, 75872, 75968, 76000, 76096,
    76224, 76288, 76352, 76416, 76448, 76544, 76576, 76672, 76736, 76800, 76864, 76928, 77024, 77120, 77152, 77248,
    77344, 77408, 77472, 77536, 77568, 77664, 77696, 77792, 77856, 77920, 77984, 78048, 78144, 78240, 78272, 78368,
    78464, 78528, 78592, 78656, 78688, 78784, 78816, 78912, 78976, 79040, 79104, 79168, 79264, 79360, 79392, 79488,
    79616, 79680, 79744, 79808, 79840, 79936, 79968, 80064, 80128, 80192, 80256, 80320, 80416, 80512, 80544, 80640,
    80768, 80832, 80896, 80960, 80992, 81088, 81120, 81216, 81280, 81344, 81408, 81472, 81568, 81664, 81696, 81792,
    81952, 82016, 82080, 82144, 82176, 82272, 82304, 82400, 82464, 82528, 82592, 82656, 82752, 82848, 82880, 82976,
];

/// The arcade chip's phase step for a block/keycode/fraction, after a delta.
fn opm_key_code_to_phase_step(block_freq: u32, delta: i32) -> u32 {
    let table = &OPM_PHASE_STEP;
    let mut block = bitfield(block_freq, 10, 3) as i32;
    let adjusted = bitfield(block_freq, 6, 4) as i32 - bitfield(block_freq, 8, 2) as i32;
    let mut eff = (adjusted << 6) | bitfield(block_freq, 0, 6) as i32;
    eff += delta;
    if !(0..768).contains(&eff) {
        if eff < 0 {
            eff += 768;
            if block == 0 {
                return table[0] >> 7;
            }
            block -= 1;
        } else {
            eff -= 768;
            if eff >= 768 {
                block += 1;
                eff -= 768;
            }
            if block >= 7 {
                return table[767];
            }
            block += 1;
        }
    }
    table[eff as usize] >> (block ^ 7)
}

fn opn_lfo_pm_phase_adjustment(fnum_bits: u32, pm_sensitivity: u32, lfo_raw_pm: i32) -> i32 {
    const SHIFTS: [[u8; 8]; 8] = [
        [0x77, 0x77, 0x77, 0x77, 0x77, 0x77, 0x77, 0x77],
        [0x77, 0x77, 0x77, 0x77, 0x72, 0x72, 0x72, 0x72],
        [0x77, 0x77, 0x77, 0x72, 0x72, 0x72, 0x17, 0x17],
        [0x77, 0x77, 0x72, 0x72, 0x17, 0x17, 0x12, 0x12],
        [0x77, 0x77, 0x72, 0x17, 0x17, 0x17, 0x12, 0x07],
        [0x77, 0x77, 0x17, 0x12, 0x07, 0x07, 0x02, 0x01],
        [0x77, 0x77, 0x17, 0x12, 0x07, 0x07, 0x02, 0x01],
        [0x77, 0x77, 0x17, 0x12, 0x07, 0x07, 0x02, 0x01],
    ];
    let abs_pm = lfo_raw_pm.unsigned_abs();
    let shifts = SHIFTS[pm_sensitivity as usize & 7][bitfield(abs_pm, 0, 3) as usize] as u32;
    let mut adjust = ((fnum_bits >> bitfield(shifts, 0, 4)) + (fnum_bits >> bitfield(shifts, 4, 4))) as i32;
    if pm_sensitivity > 5 {
        adjust <<= pm_sensitivity - 5;
    }
    adjust >>= 2;
    if lfo_raw_pm < 0 {
        -adjust
    } else {
        adjust
    }
}

/// The arcade chip's DAC: 10-bit mantissa, 3-bit exponent.
fn roundtrip_fp(value: i32) -> i32 {
    if value < -32768 {
        return -32768;
    }
    if value > 32767 {
        return 32767;
    }
    let scan = value ^ (value >> 31);
    let exponent = (7 - ((scan << 17) as u32).leading_zeros() as i32).max(1) - 1;
    let mask = (1 << exponent) - 1;
    value & !mask
}

// ---------------------------------------------------------------- the chips

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Chip {
    /// The 16-bit consoles' six-channel chip.
    Console,
    /// The arcade boards' eight-channel chip.
    Arcade,
}

impl Chip {
    pub fn channels(self) -> usize {
        match self {
            Chip::Console => 6,
            Chip::Arcade => 8,
        }
    }

    /// Its sample rate at the usual clock.
    pub fn rate(self) -> f64 {
        match self {
            Chip::Console => 7_670_453.0 / 144.0,
            Chip::Arcade => 3_579_545.0 / 64.0,
        }
    }

    fn channel_offset(self, ch: usize) -> u32 {
        match self {
            Chip::Console => (ch % 3) as u32 + 0x100 * (ch / 3) as u32,
            Chip::Arcade => ch as u32,
        }
    }

    fn operator_offset(self, opnum: usize) -> u32 {
        match self {
            Chip::Console => ((opnum % 12) + (opnum % 12) / 3) as u32 + 0x100 * (opnum / 12) as u32,
            Chip::Arcade => opnum as u32,
        }
    }

    /// The operators of channel `ch`, in the algorithm's order.
    fn operator_map(self, ch: usize) -> [usize; 4] {
        match self {
            Chip::Console => {
                let (c, hi) = (ch % 3, (ch / 3) * 12);
                [hi + c, hi + c + 6, hi + c + 3, hi + c + 9]
            }
            Chip::Arcade => [ch, ch + 16, ch + 8, ch + 24],
        }
    }
}

const EG_DEPRESS: usize = 0;
const EG_ATTACK: usize = 1;
const EG_DECAY: usize = 2;
const EG_SUSTAIN: usize = 3;
const EG_RELEASE: usize = 4;
const EG_STATES: usize = 6;
const EG_QUIET: u32 = 0x380;
const PHASE_STEP_DYNAMIC: u32 = 1;
const WAVEFORM_LENGTH: u32 = 0x400;

/// What is worked out for an operator before it is clocked.
#[derive(Clone, Copy, Default)]
struct Cache {
    phase_step: u32,
    total_level: u32,
    block_freq: u32,
    detune: i32,
    multiple: u32,
    eg_sustain: u32,
    eg_rate: [u32; EG_STATES],
}

/// The register array and what the chip computes from it.
struct Regs {
    chip: Chip,
    data: Vec<u8>,
    waveform: Vec<u16>,
    lfo_counter: u32,
    lfo_am: u32,
    noise_lfsr: u32,
    noise_counter: u32,
    noise_state: u32,
    lfo_waveform: [[i16; 256]; 4],
}

impl Regs {
    fn new(chip: Chip) -> Self {
        let waveform = (0..WAVEFORM_LENGTH).map(|i| (abs_sin_attenuation(i) | (bitfield(i, 9, 1) << 15)) as u16).collect();
        let mut lfo_waveform = [[0i16; 256]; 4];
        for index in 0..256u32 {
            let am = (index ^ 0xff) as u8;
            let pm = index as u8;
            lfo_waveform[0][index as usize] = (am as u16 | ((pm as u16) << 8)) as i16;
            let am = if bitfield(index, 7, 1) != 0 { 0u8 } else { 0xff };
            let pm = am ^ 0x80;
            lfo_waveform[1][index as usize] = (am as u16 | ((pm as u16) << 8)) as i16;
            let am = if bitfield(index, 7, 1) != 0 { (index << 1) as u8 } else { ((index ^ 0xff) << 1) as u8 };
            let pm = if bitfield(index, 6, 1) != 0 { am } else { !am };
            lfo_waveform[2][index as usize] = (am as u16 | ((pm as u16) << 8)) as i16;
        }
        let mut r = Self { chip, data: vec![0; 0x200], waveform, lfo_counter: 0, lfo_am: 0, noise_lfsr: 1, noise_counter: 0, noise_state: 0, lfo_waveform };
        r.reset();
        r
    }

    fn reset(&mut self) {
        self.data.fill(0);
        match self.chip {
            Chip::Console => {
                for i in [0xb4, 0xb5, 0xb6, 0x1b4, 0x1b5, 0x1b6] {
                    self.data[i] = 0xc0;
                }
            }
            Chip::Arcade => {
                for i in 0x20..0x28 {
                    self.data[i] = 0xc0;
                }
            }
        }
    }

    fn byte(&self, offset: u32, start: u32, count: u32, extra: u32) -> u32 {
        bitfield(self.data[(offset + extra) as usize & 0x1ff] as u32, start, count)
    }

    fn word(&self, o1: u32, s1: u32, c1: u32, o2: u32, s2: u32, c2: u32, extra: u32) -> u32 {
        (self.byte(o1, s1, c1, extra) << c2) | self.byte(o2, s2, c2, extra)
    }

    // The registers, by chip.
    fn lfo_enable(&self) -> u32 {
        match self.chip {
            Chip::Console => self.byte(0x22, 3, 1, 0),
            Chip::Arcade => 1,
        }
    }
    fn ch_block_freq(&self, c: u32) -> u32 {
        match self.chip {
            Chip::Console => self.word(0xa4, 0, 6, 0xa0, 0, 8, c),
            Chip::Arcade => self.word(0x28, 0, 7, 0x30, 2, 6, c),
        }
    }
    fn ch_feedback(&self, c: u32) -> u32 {
        match self.chip {
            Chip::Console => self.byte(0xb0, 3, 3, c),
            Chip::Arcade => self.byte(0x20, 3, 3, c),
        }
    }
    fn ch_algorithm(&self, c: u32) -> u32 {
        match self.chip {
            Chip::Console => self.byte(0xb0, 0, 3, c),
            Chip::Arcade => self.byte(0x20, 0, 3, c),
        }
    }
    fn ch_output_0(&self, c: u32) -> u32 {
        match self.chip {
            Chip::Console => self.byte(0xb4, 7, 1, c),
            Chip::Arcade => self.byte(0x20, 6, 1, c),
        }
    }
    fn ch_output_1(&self, c: u32) -> u32 {
        match self.chip {
            Chip::Console => self.byte(0xb4, 6, 1, c),
            Chip::Arcade => self.byte(0x20, 7, 1, c),
        }
    }
    fn ch_lfo_am_sens(&self, c: u32) -> u32 {
        match self.chip {
            Chip::Console => self.byte(0xb4, 4, 2, c),
            Chip::Arcade => self.byte(0x38, 0, 2, c),
        }
    }
    fn ch_lfo_pm_sens(&self, c: u32) -> u32 {
        match self.chip {
            Chip::Console => self.byte(0xb4, 0, 3, c),
            Chip::Arcade => self.byte(0x38, 4, 3, c),
        }
    }
    fn op_base(&self, console: u32, arcade: u32) -> u32 {
        match self.chip {
            Chip::Console => console,
            Chip::Arcade => arcade,
        }
    }
    fn op_detune(&self, o: u32) -> u32 {
        self.byte(self.op_base(0x30, 0x40), 4, 3, o)
    }
    fn op_multiple(&self, o: u32) -> u32 {
        self.byte(self.op_base(0x30, 0x40), 0, 4, o)
    }
    fn op_total_level(&self, o: u32) -> u32 {
        self.byte(self.op_base(0x40, 0x60), 0, 7, o)
    }
    fn op_ksr(&self, o: u32) -> u32 {
        self.byte(self.op_base(0x50, 0x80), 6, 2, o)
    }
    fn op_attack_rate(&self, o: u32) -> u32 {
        self.byte(self.op_base(0x50, 0x80), 0, 5, o)
    }
    fn op_decay_rate(&self, o: u32) -> u32 {
        self.byte(self.op_base(0x60, 0xa0), 0, 5, o)
    }
    fn op_lfo_am_enable(&self, o: u32) -> u32 {
        self.byte(self.op_base(0x60, 0xa0), 7, 1, o)
    }
    fn op_sustain_rate(&self, o: u32) -> u32 {
        self.byte(self.op_base(0x70, 0xc0), 0, 5, o)
    }
    fn op_detune2(&self, o: u32) -> u32 {
        match self.chip {
            Chip::Console => 0,
            Chip::Arcade => self.byte(0xc0, 6, 2, o),
        }
    }
    fn op_sustain_level(&self, o: u32) -> u32 {
        self.byte(self.op_base(0x80, 0xe0), 4, 4, o)
    }
    fn op_release_rate(&self, o: u32) -> u32 {
        self.byte(self.op_base(0x80, 0xe0), 0, 4, o)
    }
    fn op_ssg_eg_enable(&self, o: u32) -> u32 {
        match self.chip {
            Chip::Console => self.byte(0x90, 3, 1, o),
            Chip::Arcade => 0,
        }
    }
    fn op_ssg_eg_mode(&self, o: u32) -> u32 {
        match self.chip {
            Chip::Console => self.byte(0x90, 0, 3, o),
            Chip::Arcade => 0,
        }
    }
    fn noise_enable(&self) -> u32 {
        match self.chip {
            Chip::Console => 0,
            Chip::Arcade => self.byte(0x0f, 7, 1, 0),
        }
    }

    /// A register write; the key-on registers say which channel and operators.
    fn write(&mut self, index: u32, data: u8) -> Option<(usize, u32)> {
        match self.chip {
            Chip::Console => {
                if (index & 0xf0) == 0xa0 {
                    if bitfield(index, 0, 2) == 3 {
                        return None;
                    }
                    let latch = (0xb8 | bitfield(index, 3, 1)) as usize;
                    if bitfield(index, 2, 1) != 0 {
                        self.data[latch] = data & 0x3f;
                    } else {
                        self.data[index as usize] = data;
                        self.data[(index | 4) as usize] = self.data[latch];
                    }
                    return None;
                }
                if (index & 0xf8) == 0xb8 {
                    return None;
                }
                self.data[index as usize] = data;
                if index == 0x28 {
                    let mut ch = bitfield(data as u32, 0, 2);
                    if ch == 3 {
                        return None;
                    }
                    ch += bitfield(data as u32, 2, 1) * 3;
                    return Some((ch as usize, bitfield(data as u32, 4, 4)));
                }
                None
            }
            Chip::Arcade => {
                if index == 0x19 {
                    self.data[(index + bitfield(data as u32, 7, 1)) as usize] = data;
                } else if index != 0x1a {
                    self.data[index as usize] = data;
                }
                if index == 0x08 {
                    return Some((bitfield(data as u32, 0, 3) as usize, bitfield(data as u32, 3, 4)));
                }
                None
            }
        }
    }

    /// The noise and the LFO, a sample on; returns the LFO's raw PM.
    fn clock_noise_and_lfo(&mut self) -> i32 {
        match self.chip {
            Chip::Console => {
                if self.lfo_enable() == 0 {
                    self.lfo_counter = 0;
                    self.lfo_am = 0x3f;
                    return 0;
                }
                const MAX: [u32; 8] = [109, 78, 72, 68, 63, 45, 9, 6];
                let sub = self.lfo_counter as u8 as u32;
                self.lfo_counter = self.lfo_counter.wrapping_add(1);
                if sub >= MAX[self.byte(0x22, 0, 3, 0) as usize] {
                    self.lfo_counter = self.lfo_counter.wrapping_add(0x101 - sub);
                }
                self.lfo_am = bitfield(self.lfo_counter, 8, 6);
                if bitfield(self.lfo_counter, 14, 1) == 0 {
                    self.lfo_am ^= 0x3f;
                }
                let mut pm = bitfield(self.lfo_counter, 10, 3) as i32;
                if bitfield(self.lfo_counter, 13, 1) != 0 {
                    pm ^= 7;
                }
                if bitfield(self.lfo_counter, 14, 1) != 0 {
                    -pm
                } else {
                    pm
                }
            }
            Chip::Arcade => {
                let freq = self.byte(0x0f, 0, 5, 0) ^ 0x1f;
                for _ in 0..2 {
                    self.noise_lfsr <<= 1;
                    self.noise_lfsr |= bitfield(self.noise_lfsr, 17, 1) ^ bitfield(self.noise_lfsr, 14, 1) ^ 1;
                    let c = self.noise_counter;
                    self.noise_counter += 1;
                    if c >= freq {
                        self.noise_counter = 0;
                        self.noise_state = bitfield(self.noise_lfsr, 17, 1);
                    }
                }
                let rate = self.byte(0x18, 0, 8, 0);
                self.lfo_counter = self.lfo_counter.wrapping_add((0x10 | bitfield(rate, 0, 4)) << bitfield(rate, 4, 4));
                if self.byte(0x01, 1, 1, 0) != 0 {
                    self.lfo_counter = 0;
                }
                let lfo = bitfield(self.lfo_counter, 22, 8) as usize;
                let noise = bitfield(self.noise_lfsr, 17, 8);
                self.lfo_waveform[3][(lfo + 1) & 0xff] = (noise | (noise << 8)) as i16;
                let ampm = self.lfo_waveform[self.byte(0x1b, 0, 2, 0) as usize][lfo] as i32;
                self.lfo_am = (((ampm & 0xff) as u32) * self.byte(0x19, 0, 7, 0)) >> 7;
                ((ampm >> 8) * self.byte(0x1a, 0, 7, 0) as i32) >> 7
            }
        }
    }

    fn lfo_am_offset(&self, c: u32) -> u32 {
        match self.chip {
            Chip::Console => {
                let shift = (1 << (self.ch_lfo_am_sens(c) ^ 3)) - 1;
                (self.lfo_am << 1) >> shift
            }
            Chip::Arcade => {
                let s = self.ch_lfo_am_sens(c);
                if s == 0 {
                    0
                } else {
                    self.lfo_am << (s - 1)
                }
            }
        }
    }

    fn cache_operator_data(&self, c: u32, o: u32, cache: &mut Cache) {
        let block_freq = self.ch_block_freq(c);
        cache.block_freq = block_freq;
        let keycode = match self.chip {
            Chip::Console => (bitfield(block_freq, 10, 4) << 1) | bitfield(0xfe80, bitfield(block_freq, 7, 4), 1),
            Chip::Arcade => bitfield(block_freq, 8, 5),
        };
        cache.detune = detune_adjustment(self.op_detune(o), keycode);
        cache.multiple = (self.op_multiple(o) * 2).max(1);
        let pm_active = match self.chip {
            Chip::Console => self.lfo_enable() != 0 && self.ch_lfo_pm_sens(c) != 0,
            Chip::Arcade => self.byte(0x1a, 0, 7, 0) != 0 && self.ch_lfo_pm_sens(c) != 0,
        };
        cache.phase_step = if pm_active { PHASE_STEP_DYNAMIC } else { self.compute_phase_step(c, o, cache, 0) };
        cache.total_level = self.op_total_level(o) << 3;
        let mut sl = self.op_sustain_level(o);
        sl |= (sl + 1) & 0x10;
        cache.eg_sustain = sl << 5;
        let ksrval = keycode >> (self.op_ksr(o) ^ 3);
        let eff = |raw: u32| if raw == 0 { 0 } else { (raw + ksrval).min(63) };
        cache.eg_rate[EG_ATTACK] = eff(self.op_attack_rate(o) * 2);
        cache.eg_rate[EG_DECAY] = eff(self.op_decay_rate(o) * 2);
        cache.eg_rate[EG_SUSTAIN] = eff(self.op_sustain_rate(o) * 2);
        cache.eg_rate[EG_RELEASE] = eff(self.op_release_rate(o) * 4 + 2);
        cache.eg_rate[EG_DEPRESS] = 0x3f;
    }

    fn compute_phase_step(&self, c: u32, o: u32, cache: &Cache, lfo_raw_pm: i32) -> u32 {
        match self.chip {
            Chip::Console => {
                let mut fnum = bitfield(cache.block_freq, 0, 11) << 1;
                let pms = self.ch_lfo_pm_sens(c);
                if pms != 0 {
                    fnum = (fnum as i32 + opn_lfo_pm_phase_adjustment(bitfield(cache.block_freq, 4, 7), pms, lfo_raw_pm)) as u32 & 0xfff;
                }
                let block = bitfield(cache.block_freq, 11, 3);
                let mut step = (fnum << block) >> 2;
                step = (step as i32 + cache.detune) as u32 & 0x1ffff;
                (step * cache.multiple) >> 1
            }
            Chip::Arcade => {
                const DT2: [i32; 4] = [0, (600 * 64 + 50) / 100, (781 * 64 + 50) / 100, (950 * 64 + 50) / 100];
                let mut delta = DT2[self.op_detune2(o) as usize];
                let pms = self.ch_lfo_pm_sens(c);
                if pms != 0 {
                    if pms < 6 {
                        delta += lfo_raw_pm >> (6 - pms);
                    } else {
                        delta += ((lfo_raw_pm as u32) << (pms - 5)) as i32;
                    }
                }
                let step = (opm_key_code_to_phase_step(cache.block_freq, delta) as i32 + cache.detune) as u32;
                (step * cache.multiple) >> 1
            }
        }
    }
}

#[derive(Clone, Copy, Default)]
struct Operator {
    choffs: u32,
    opoffs: u32,
    phase: u32,
    env_attenuation: u32,
    env_state: usize,
    ssg_inverted: u32,
    key_state: u32,
    keyon_live: u32,
    cache: Cache,
}

impl Operator {
    fn reset(&mut self) {
        self.phase = 0;
        self.env_attenuation = 0x3ff;
        self.env_state = EG_RELEASE;
        self.ssg_inverted = 0;
        self.key_state = 0;
        self.keyon_live = 0;
    }

    fn start_attack(&mut self, regs: &Regs, is_restart: bool) {
        if self.env_state == EG_ATTACK {
            return;
        }
        self.env_state = EG_ATTACK;
        if !is_restart {
            self.ssg_inverted = regs.op_ssg_eg_enable(self.opoffs) & bitfield(regs.op_ssg_eg_mode(self.opoffs), 2, 1);
            self.phase = 0;
        }
        if self.cache.eg_rate[EG_ATTACK] >= 62 {
            self.env_attenuation = 0;
        }
    }

    fn start_release(&mut self) {
        if self.env_state >= EG_RELEASE {
            return;
        }
        self.env_state = EG_RELEASE;
        if self.ssg_inverted != 0 {
            self.env_attenuation = (0x200u32.wrapping_sub(self.env_attenuation)) & 0x3ff;
            self.ssg_inverted = 0;
        }
    }

    fn clock_keystate(&mut self, regs: &Regs, keystate: u32) {
        if (keystate ^ self.key_state) != 0 {
            self.key_state = keystate;
            if keystate != 0 {
                self.start_attack(regs, false);
            } else {
                self.start_release();
            }
        }
    }

    fn prepare(&mut self, regs: &Regs) -> bool {
        regs.cache_operator_data(self.choffs, self.opoffs, &mut self.cache);
        self.clock_keystate(regs, (self.keyon_live != 0) as u32);
        self.env_state != EG_RELEASE || self.env_attenuation < EG_QUIET
    }

    fn clock_ssg_eg_state(&mut self, regs: &Regs) {
        if bitfield(self.env_attenuation, 9, 1) == 0 {
            return;
        }
        let mode = regs.op_ssg_eg_mode(self.opoffs);
        if bitfield(mode, 0, 1) != 0 {
            self.ssg_inverted = bitfield(mode, 2, 1) ^ bitfield(mode, 1, 1);
            if self.env_state != EG_ATTACK {
                self.env_attenuation = if self.ssg_inverted != 0 { 0x200 } else { 0x3ff };
            }
        } else {
            self.ssg_inverted ^= bitfield(mode, 1, 1);
            if self.env_state == EG_DECAY || self.env_state == EG_SUSTAIN {
                self.start_attack(regs, true);
            }
            if bitfield(mode, 1, 1) == 0 {
                self.phase = 0;
            }
        }
        if self.env_state == EG_RELEASE {
            self.env_attenuation = 0x3ff;
        }
    }

    fn clock_envelope(&mut self, regs: &Regs, env_counter: u32) {
        if self.env_state == EG_ATTACK && self.env_attenuation == 0 {
            self.env_state = EG_DECAY;
        }
        if self.env_state == EG_DECAY && self.env_attenuation >= self.cache.eg_sustain {
            self.env_state = EG_SUSTAIN;
        }
        let rate = self.cache.eg_rate[self.env_state];
        let rate_shift = rate >> 2;
        let counter = env_counter << rate_shift;
        if bitfield(counter, 0, 11) != 0 {
            return;
        }
        let relevant = bitfield(counter, if rate_shift <= 11 { 11 } else { rate_shift }, 3);
        let increment = attenuation_increment(rate, relevant);
        if self.env_state == EG_ATTACK {
            if rate < 62 {
                // (C's ~ of the 16-bit value is negative: the attenuation falls)
                let delta = (-(self.env_attenuation as i32 + 1) * increment as i32) >> 4;
                self.env_attenuation = (self.env_attenuation as i32 + delta).max(0) as u32;
            }
        } else {
            if regs.op_ssg_eg_enable(self.opoffs) == 0 {
                self.env_attenuation += increment;
            } else if self.env_attenuation < 0x200 {
                self.env_attenuation += 4 * increment;
            }
            if self.env_attenuation >= 0x400 {
                self.env_attenuation = 0x3ff;
            }
        }
    }

    fn clock(&mut self, regs: &Regs, env_counter: u32, lfo_raw_pm: i32) {
        if regs.op_ssg_eg_enable(self.opoffs) != 0 {
            self.clock_ssg_eg_state(regs);
        } else {
            self.ssg_inverted = 0;
        }
        if bitfield(env_counter, 0, 2) == 0 {
            self.clock_envelope(regs, env_counter >> 2);
        }
        let mut step = self.cache.phase_step;
        if step == PHASE_STEP_DYNAMIC {
            step = regs.compute_phase_step(self.choffs, self.opoffs, &self.cache, lfo_raw_pm);
        }
        self.phase = self.phase.wrapping_add(step);
    }

    fn phase(&self) -> u32 {
        self.phase >> 10
    }

    fn envelope_attenuation(&self, regs: &Regs, am_offset: u32) -> u32 {
        let mut result = self.env_attenuation;
        if self.ssg_inverted != 0 {
            result = (0x200u32.wrapping_sub(result)) & 0x3ff;
        }
        if regs.op_lfo_am_enable(self.opoffs) != 0 {
            result += am_offset;
        }
        result += self.cache.total_level;
        result.min(0x3ff)
    }

    fn compute_volume(&self, regs: &Regs, phase: u32, am_offset: u32) -> i32 {
        if self.env_attenuation > EG_QUIET {
            return 0;
        }
        let sin = regs.waveform[(phase & (WAVEFORM_LENGTH - 1)) as usize] as u32;
        let env = self.envelope_attenuation(regs, am_offset) << 2;
        let result = attenuation_to_volume((sin & 0x7fff) + env) as i32;
        if bitfield(sin, 15, 1) != 0 {
            -result
        } else {
            result
        }
    }

    fn compute_noise_volume(&self, regs: &Regs, am_offset: u32) -> i32 {
        let result = ((self.envelope_attenuation(regs, am_offset) ^ 0x3ff) << 1) as i32;
        if bitfield(regs.noise_state, 0, 1) != 0 {
            -result
        } else {
            result
        }
    }
}

#[derive(Clone, Copy, Default)]
struct Channel {
    choffs: u32,
    feedback: [i16; 2],
    feedback_in: i16,
    ops: [usize; 4],
}

const ALGORITHMS: [u16; 8] = {
    const fn alg(op2in: u16, op3in: u16, op4in: u16, op1out: u16, op2out: u16, op3out: u16) -> u16 {
        op2in | (op3in << 1) | (op4in << 4) | (op1out << 7) | (op2out << 8) | (op3out << 9)
    }
    [alg(1, 2, 3, 0, 0, 0), alg(0, 5, 3, 0, 0, 0), alg(0, 2, 6, 0, 0, 0), alg(1, 0, 7, 0, 0, 0), alg(1, 0, 3, 0, 1, 0), alg(1, 1, 1, 0, 1, 1), alg(1, 0, 0, 0, 1, 1), alg(0, 0, 0, 1, 1, 1)]
};

/// One chip: its registers, channels and operators.
struct Engine {
    regs: Regs,
    env_counter: u32,
    channels: Vec<Channel>,
    ops: Vec<Operator>,
    active: u32,
    modified: bool,
    prepare_count: u32,
}

impl Engine {
    fn new(chip: Chip) -> Self {
        let regs = Regs::new(chip);
        let nch = chip.channels();
        let mut ops: Vec<Operator> = (0..nch * 4).map(|i| Operator { opoffs: chip.operator_offset(i), ..Default::default() }).collect();
        let channels: Vec<Channel> = (0..nch)
            .map(|c| {
                let map = chip.operator_map(c);
                for &o in &map {
                    ops[o].choffs = chip.channel_offset(c);
                }
                Channel { choffs: chip.channel_offset(c), feedback: [0; 2], feedback_in: 0, ops: map }
            })
            .collect();
        let mut e = Self { regs, env_counter: 0, channels, ops, active: 0, modified: true, prepare_count: 0 };
        for o in e.ops.iter_mut() {
            o.reset();
        }
        e
    }

    fn write(&mut self, reg: u32, data: u8) {
        self.modified = true;
        if let Some((ch, mask)) = self.regs.write(reg, data) {
            if ch < self.channels.len() {
                let map = self.channels[ch].ops;
                for (k, &o) in map.iter().enumerate() {
                    self.ops[o].keyon_live = bitfield(mask, k as u32, 1);
                }
            }
        }
    }

    /// Channel `ch`'s operators worked out now (a key-on or -off takes effect).
    fn prepare_channel(&mut self, ch: usize) {
        let map = self.channels[ch].ops;
        let mut active = false;
        for &o in &map {
            active |= self.ops[o].prepare(&self.regs);
        }
        if active {
            self.active |= 1 << ch;
        } else {
            self.active &= !(1 << ch);
        }
    }

    fn clock(&mut self) {
        if self.modified || self.prepare_count >= 4096 {
            for ch in 0..self.channels.len() {
                self.prepare_channel(ch);
            }
            self.modified = false;
            self.prepare_count = 0;
        } else {
            self.prepare_count += 1;
        }
        // (the envelope clock is a third of the sample rate)
        self.env_counter = self.env_counter.wrapping_add(1);
        if bitfield(self.env_counter, 0, 2) == 3 {
            self.env_counter = self.env_counter.wrapping_add(1);
        }
        let pm = self.regs.clock_noise_and_lfo();
        let ec = self.env_counter;
        for c in self.channels.iter_mut() {
            c.feedback[0] = c.feedback[1];
            c.feedback[1] = c.feedback_in;
            for &o in &c.ops {
                self.ops[o].clock(&self.regs, ec, pm);
            }
        }
    }

    /// Channel `ch`'s output (left, right), its sum shifted down by
    /// `rshift` and clipped to `clipmax`.
    fn output_channel(&mut self, ch: usize, rshift: u32, clipmax: i32) -> (i32, i32) {
        if self.active & (1 << ch) == 0 {
            return (0, 0);
        }
        let regs = &self.regs;
        let c = &mut self.channels[ch];
        let op = |k: usize| &self.ops[c.ops[k]];
        let am = regs.lfo_am_offset(c.choffs);
        let fb = regs.ch_feedback(c.choffs);
        let mut opmod = 0i32;
        if fb != 0 {
            opmod = (c.feedback[0] as i32 + c.feedback[1] as i32) >> (10 - fb);
        }
        let op1 = op(0).compute_volume(regs, (op(0).phase() as i32 + opmod) as u32, am);
        c.feedback_in = op1 as i16;
        let alg = ALGORITHMS[regs.ch_algorithm(c.choffs) as usize & 7] as u32;
        let mut opout = [0i32; 8];
        opout[1] = op1;
        let m = opout[bitfield(alg, 0, 1) as usize] >> 1;
        opout[2] = op(1).compute_volume(regs, (op(1).phase() as i32 + m) as u32, am);
        opout[5] = opout[1] + opout[2];
        let m = opout[bitfield(alg, 1, 3) as usize] >> 1;
        opout[3] = op(2).compute_volume(regs, (op(2).phase() as i32 + m) as u32, am);
        opout[6] = opout[1] + opout[3];
        opout[7] = opout[2] + opout[3];
        let mut result = if regs.noise_enable() != 0 && c.choffs == 7 {
            op(3).compute_noise_volume(regs, am)
        } else {
            let m = opout[bitfield(alg, 4, 3) as usize] >> 1;
            op(3).compute_volume(regs, (op(3).phase() as i32 + m) as u32, am)
        };
        result >>= rshift;
        let clipmin = -clipmax - 1;
        if bitfield(alg, 7, 1) != 0 {
            result = (result + (opout[1] >> rshift)).clamp(clipmin, clipmax);
        }
        if bitfield(alg, 8, 1) != 0 {
            result = (result + (opout[2] >> rshift)).clamp(clipmin, clipmax);
        }
        if bitfield(alg, 9, 1) != 0 {
            result = (result + (opout[3] >> rshift)).clamp(clipmin, clipmax);
        }
        let l = if regs.ch_output_0(c.choffs) != 0 { result } else { 0 };
        let r = if regs.ch_output_1(c.choffs) != 0 { result } else { 0 };
        (l, r)
    }

    /// A sample, as the chip's output stage makes it (16-bit range).
    fn generate(&mut self) -> (i32, i32) {
        self.clock();
        match self.regs.chip {
            Chip::Console => {
                // 9-bit per channel, the DAC's step around zero, multiplexed (averaged).
                let dac = |v: i32| if v < 0 { v - 3 } else { v + 4 };
                let (mut l, mut r) = (0, 0);
                for ch in 0..6 {
                    let (a, b) = self.output_channel(ch, 5, 256);
                    l += dac(a);
                    r += dac(b);
                }
                ((l * 128) * 64 / (6 * 65), (r * 128) * 64 / (6 * 65))
            }
            Chip::Arcade => {
                let (mut l, mut r) = (0, 0);
                for ch in 0..8 {
                    let (a, b) = self.output_channel(ch, 0, 32767);
                    l += a;
                    r += b;
                }
                (roundtrip_fp(l), roundtrip_fp(r))
            }
        }
    }
}

// ---------------------------------------------------------------- the instrument

#[derive(Clone, Copy, Default)]
struct Voice {
    note: u8,
    vel: f32,
    gate: bool,
    age: u64,
    /// Seconds since the note started (for the pitch drop).
    t: f32,
}

pub struct ChipFmState {
    sr: f32,
    chip: Chip,
    engine: Engine,
    voices: Vec<Voice>,
    clock: u64,
    /// The last patch written (params of the patch only), to write it again when it changes.
    written: Vec<f32>,
    /// Resampling: the chip's last four samples and where between them the output is.
    hist: [(f32, f32); 4],
    frac: f64,
    step: f64,
    /// The output capacitors: the DC of the console chip's DAC taken out
    /// (input and output of a high pass a channel).
    dc: [(f32, f32); 2],
    pub pitch_bend: f32,
}

impl ChipFmState {
    pub fn new(sr: f32) -> Self {
        let chip = Chip::Console;
        Self { sr, chip, engine: Engine::new(chip), voices: vec![Voice::default(); chip.channels()], clock: 0, written: Vec::new(), hist: [(0.0, 0.0); 4], frac: 0.0, step: chip.rate() / sr as f64, dc: [(0.0, 0.0); 2], pitch_bend: 0.0 }
    }

    fn set_chip(&mut self, chip: Chip) {
        if chip != self.chip {
            self.chip = chip;
            self.engine = Engine::new(chip);
            self.voices = vec![Voice::default(); chip.channels()];
            self.written.clear();
            self.step = chip.rate() / self.sr as f64;
        }
    }

    /// The register offset of channel `ch`'s operator `k` (algorithm order).
    fn op_reg(&self, ch: usize, k: usize) -> u32 {
        let opnum = self.chip.operator_map(ch)[k];
        self.chip.operator_offset(opnum)
    }

    /// Which operators sound (and so take velocity) in algorithm `alg`.
    fn carriers(alg: u32) -> [bool; 4] {
        match alg {
            0..=3 => [false, false, false, true],
            4 => [false, true, false, true],
            5 | 6 => [false, true, true, true],
            _ => [true, true, true, true],
        }
    }

    /// The patch into every channel's registers (velocity aside).
    fn write_patch(&mut self, p: &[f32]) {
        let chip = self.chip;
        let alg = geti(p, ALGORITHM);
        let fb = geti(p, FEEDBACK);
        let lfo = geti(p, LFO_RATE);
        let pms = geti(p, PMS);
        let ams = geti(p, AMS);
        let pan: u32 = match geti(p, PAN) {
            1 => 0x2,
            2 => 0x1,
            _ => 0x3,
        };
        match chip {
            Chip::Console => {
                self.engine.write(0x22, if lfo == 0 { 0 } else { 0x08 | (lfo - 1) as u8 });
            }
            Chip::Arcade => {
                const RATES: [u8; 9] = [0, 0x70, 0x90, 0xa8, 0xb8, 0xc4, 0xd0, 0xdc, 0xe8];
                self.engine.write(0x18, RATES[lfo as usize]);
                self.engine.write(0x19, geti(p, AM_DEPTH) as u8 & 0x7f);
                self.engine.write(0x19, 0x80 | (if lfo == 0 { 0 } else { geti(p, PM_DEPTH) as u8 & 0x7f }));
                self.engine.write(0x1b, geti(p, LFO_WAVE) as u8 & 3);
            }
        }
        for ch in 0..chip.channels() {
            let c = chip.channel_offset(ch);
            match chip {
                Chip::Console => {
                    self.engine.write(0xb0 + c, ((fb << 3) | alg) as u8);
                    self.engine.write(0xb4 + c, ((pan << 6) | (ams << 4) | pms) as u8);
                }
                Chip::Arcade => {
                    // (left is bit 6, right bit 7)
                    let lr = ((pan & 2) >> 1) | ((pan & 1) << 1);
                    self.engine.write(0x20 + c, ((lr << 6) | (fb << 3) | alg) as u8);
                    self.engine.write(0x38 + c, ((pms << 4) | ams) as u8);
                }
            }
            for k in 0..4 {
                let o = self.op_reg(ch, k);
                let dt = get(p, op(k, DT)).round() as i32;
                let dt = if dt < 0 { (4 + (-dt).min(3)) as u32 } else { dt.min(3) as u32 };
                let mul = geti(p, op(k, MUL));
                let ks = geti(p, op(k, KS));
                let am = geti(p, op(k, AM));
                let (ar, dr, sr, rr, sl) = (geti(p, op(k, AR)), geti(p, op(k, DR)), geti(p, op(k, SR)), geti(p, op(k, RR)), geti(p, op(k, SL)));
                match chip {
                    Chip::Console => {
                        self.engine.write(0x30 + o, ((dt << 4) | mul) as u8);
                        self.engine.write(0x50 + o, ((ks << 6) | ar) as u8);
                        self.engine.write(0x60 + o, ((am << 7) | dr) as u8);
                        self.engine.write(0x70 + o, sr as u8);
                        self.engine.write(0x80 + o, ((sl << 4) | rr) as u8);
                        let ssg = geti(p, op(k, SSG));
                        self.engine.write(0x90 + o, if ssg == 0 { 0 } else { (0x08 | (ssg - 1)) as u8 });
                    }
                    Chip::Arcade => {
                        let dt2 = geti(p, op(k, DT2));
                        self.engine.write(0x40 + o, ((dt << 4) | mul) as u8);
                        self.engine.write(0x80 + o, ((ks << 6) | ar) as u8);
                        self.engine.write(0xa0 + o, ((am << 7) | dr) as u8);
                        self.engine.write(0xc0 + o, ((dt2 << 6) | sr) as u8);
                        self.engine.write(0xe0 + o, ((sl << 4) | rr) as u8);
                    }
                }
            }
        }
        // The levels, with each channel's note's velocity.
        for ch in 0..chip.channels() {
            self.write_levels(p, ch);
        }
    }

    /// Channel `ch`'s total levels: the patch's, the sounding operators
    /// quieter for a softer note.
    fn write_levels(&mut self, p: &[f32], ch: usize) {
        let alg = geti(p, ALGORITHM);
        let carriers = Self::carriers(alg);
        let vel_amt = get(p, VELOCITY).clamp(0.0, 1.0);
        let vel = self.voices.get(ch).map(|v| v.vel).unwrap_or(1.0);
        // (a softer note: up to 24 dB quieter, in the chip's 0.75 dB steps)
        let soften = ((1.0 - vel) * vel_amt * 32.0).round() as u32;
        for k in 0..4 {
            let o = self.op_reg(ch, k);
            let tl = (geti(p, op(k, TL)) + if carriers[k] { soften } else { 0 }).min(127);
            let reg = match self.chip {
                Chip::Console => 0x40,
                Chip::Arcade => 0x60,
            };
            self.engine.write(reg + o, tl as u8);
        }
    }

    /// Channel `ch` set to sound `pitch` (a MIDI note, fractional).
    fn write_pitch(&mut self, ch: usize, pitch: f32) {
        let c = self.chip.channel_offset(ch);
        match self.chip {
            Chip::Console => {
                let freq = 440.0 * 2f64.powf((pitch as f64 - 69.0) / 12.0);
                let rate = self.chip.rate();
                let mut block = 0u32;
                let mut fnum = freq * (1u64 << 20) as f64 / rate * 2.0;
                while fnum > 2047.0 && block < 7 {
                    block += 1;
                    fnum /= 2.0;
                }
                let fnum = (fnum.round() as u32).min(2047);
                self.engine.write(0xa4 + c, ((block << 3) | (fnum >> 8)) as u8);
                self.engine.write(0xa0 + c, (fnum & 0xff) as u8);
            }
            Chip::Arcade => {
                // Octaves begin at C#; the codes skip every fourth value; 64 fractions a semitone.
                const CODES: [u32; 12] = [0, 1, 2, 4, 5, 6, 8, 9, 10, 12, 13, 14];
                let pitch = pitch.clamp(13.0, 125.0) - 1.0;
                let semis = pitch.floor();
                let kf = ((pitch - semis) * 64.0).floor() as u32;
                let n = semis as i32;
                let octave = (n / 12 - 1).clamp(0, 7) as u32;
                let kc = (octave << 4) | CODES[(n % 12) as usize];
                self.engine.write(0x28 + c, kc as u8);
                self.engine.write(0x30 + c, (kf << 2) as u8);
            }
        }
    }

    fn key(&mut self, ch: usize, on: bool) {
        let mask = if on { 0xf } else { 0 };
        match self.chip {
            Chip::Console => {
                let code = (ch % 3) as u32 | (((ch / 3) as u32) << 2);
                self.engine.write(0x28, ((mask << 4) | code) as u8);
            }
            Chip::Arcade => self.engine.write(0x08, ((mask << 3) | ch as u32) as u8),
        }
        self.engine.prepare_channel(ch);
    }

    fn sync(&mut self, p: &[f32]) {
        let chip = if geti(p, CHIP) == 1 { Chip::Arcade } else { Chip::Console };
        self.set_chip(chip);
        let patch = &p[..TRANSPOSE.min(p.len())];
        let pan = get(p, PAN);
        let vel = get(p, VELOCITY);
        if self.written.len() != patch.len() + 2 || self.written[..patch.len()] != *patch || self.written[patch.len()] != pan || self.written[patch.len() + 1] != vel {
            self.write_patch(p);
            self.written = patch.to_vec();
            self.written.push(pan);
            self.written.push(vel);
        }
    }

    fn pitch_of(&self, p: &[f32], v: &Voice) -> f32 {
        let drop = get(p, DROP) * (-v.t / (get(p, DROP_TIME).max(1.0) / 1000.0)).exp();
        v.note as f32 + get(p, TRANSPOSE).round() + get(p, FINE) / 100.0 + self.pitch_bend + drop
    }

    pub fn note_on(&mut self, p: &[f32], note: u8, vel: f32) {
        self.sync(p);
        self.clock += 1;
        let n = self.voices.len();
        // The channel let go of longest ago (else the oldest).
        let ch = (0..n).filter(|&i| !self.voices[i].gate).min_by_key(|&i| self.voices[i].age).unwrap_or_else(|| (0..n).min_by_key(|&i| self.voices[i].age).unwrap_or(0));
        self.key(ch, false);
        self.voices[ch] = Voice { note, vel, gate: true, age: self.clock, t: 0.0 };
        self.write_levels(p, ch);
        let v = self.voices[ch];
        let pitch = self.pitch_of(p, &v);
        self.write_pitch(ch, pitch);
        self.key(ch, true);
    }

    pub fn note_off(&mut self, _p: &[f32], note: u8) {
        for ch in 0..self.voices.len() {
            if self.voices[ch].gate && self.voices[ch].note == note {
                self.voices[ch].gate = false;
                self.key(ch, false);
            }
        }
    }

    pub fn all_off(&mut self) {
        let chip = self.chip;
        self.engine = Engine::new(chip);
        self.voices = vec![Voice::default(); chip.channels()];
        self.written.clear();
    }

    pub fn visual(&self, out: &mut [f32; crate::engine::INST_VALS]) {
        out[0] = self.engine.active.count_ones() as f32;
        // Each channel's operators' levels (0 to 1), four at a time.
        for (ch, c) in self.engine.channels.iter().enumerate().take(8) {
            for k in 0..4 {
                let o = &self.engine.ops[c.ops[k]];
                let lvl = 1.0 - (o.env_attenuation.min(0x3ff) as f32 / 1023.0);
                out[8 + ch * 4 + k] = if self.engine.active & (1 << ch) != 0 { lvl } else { 0.0 };
            }
        }
        if let Some(v) = self.voices.iter().filter(|v| v.gate).max_by_key(|v| v.age) {
            out[1] = v.note as f32;
            out[6] = 1.0;
        }
    }

    pub fn render(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32]) {
        self.sync(p);
        let n = l.len().min(r.len());
        // Pitch drops and bends: the frequencies written again once a block.
        let dt = n as f32 / self.sr;
        let moving = get(p, DROP) > 0.0 || self.pitch_bend != 0.0;
        for ch in 0..self.voices.len() {
            let v = self.voices[ch];
            if v.age == 0 {
                continue;
            }
            if moving {
                let pitch = self.pitch_of(p, &v);
                self.write_pitch(ch, pitch);
            }
            self.voices[ch].t += dt;
        }
        let gain = 10f32.powf(get(p, VOLUME) / 20.0)
            * match self.chip {
                Chip::Console => 3.0,
                Chip::Arcade => 2.0,
            }
            / 32768.0;
        // (a high pass at 5 Hz)
        let dc_r = 1.0 - std::f32::consts::TAU * 5.0 / self.sr;
        for i in 0..n {
            self.frac += self.step;
            while self.frac >= 1.0 {
                self.frac -= 1.0;
                let (a, b) = self.engine.generate();
                self.hist = [self.hist[1], self.hist[2], self.hist[3], (a as f32, b as f32)];
            }
            // Cubic (Hermite) between the middle two of the last four.
            let t = self.frac as f32;
            let herm = |y0: f32, y1: f32, y2: f32, y3: f32| {
                let c1 = 0.5 * (y2 - y0);
                let c2 = y0 - 2.5 * y1 + 2.0 * y2 - 0.5 * y3;
                let c3 = 0.5 * (y3 - y0) + 1.5 * (y1 - y2);
                ((c3 * t + c2) * t + c1) * t + y1
            };
            let h = &self.hist;
            let out = [herm(h[0].0, h[1].0, h[2].0, h[3].0) * gain, herm(h[0].1, h[1].1, h[2].1, h[3].1) * gain];
            for (k, x) in out.into_iter().enumerate() {
                let (x1, y1) = self.dc[k];
                let y = x - x1 + dc_r * y1;
                self.dc[k] = (x, y);
                if k == 0 {
                    l[i] = y;
                } else {
                    r[i] = y;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(p: &[f32], note: u8, secs: f32, off_at: f32) -> (Vec<f32>, Vec<f32>) {
        let sr = 48000.0;
        let mut st = ChipFmState::new(sr);
        st.note_on(p, note, 0.9);
        let (mut ol, mut or) = (Vec::new(), Vec::new());
        for b in 0..(secs * sr / 256.0) as usize {
            if b == (off_at * sr / 256.0) as usize {
                st.note_off(p, note);
            }
            let (mut l, mut r) = (vec![0.0; 256], vec![0.0; 256]);
            st.render(p, &mut l, &mut r);
            ol.extend(l);
            or.extend(r);
        }
        (ol, or)
    }

    fn rms(x: &[f32]) -> f32 {
        (x.iter().map(|v| v * v).sum::<f32>() / x.len().max(1) as f32).sqrt()
    }

    fn period(x: &[f32], lo: usize, hi: usize) -> usize {
        let lag_of = |lag: usize| x.iter().zip(&x[lag..]).map(|(a, b)| a * b).sum::<f32>();
        (lo..hi).max_by(|a, b| lag_of(*a).total_cmp(&lag_of(*b))).unwrap()
    }

    #[test]
    fn the_tables_line_up() {
        assert_eq!(SPECS.len(), VOLUME + 1);
        assert_eq!(SPECS[op(3, SSG)].name, "Op4 SSG-EG");
        assert_eq!(SPECS[TRANSPOSE].name, "Transpose");
        assert_eq!((OPM_PHASE_STEP[0], OPM_PHASE_STEP[767]), (41568, 82976));
        // The DAC keeps 10 bits with the sign.
        assert_eq!(roundtrip_fp(100), 100);
        assert_eq!(roundtrip_fp(12345), 12345 & !0x1f);
    }

    #[test]
    fn both_chips_play_in_tune() {
        for chip in [0.0, 1.0] {
            let mut p = preset("Init");
            p[CHIP] = chip;
            let (l, _) = run(&p, 69, 1.0, 1.0);
            let x = &l[4800..28800];
            let sr = 48000.0;
            let lag = period(x, 90, 130);
            let f = sr / lag as f32;
            assert!((f - 440.0).abs() < 5.0, "chip {chip}: {f}");
            assert!(rms(x) > 0.05, "chip {chip}: {}", rms(x));
        }
    }

    #[test]
    fn every_preset_sounds_and_stays_in_range() {
        for name in preset_names() {
            let p = preset(name);
            assert_eq!(p.len(), SPECS.len());
            let (l, r) = run(&p, if matches!(*name, "Kick" | "Snare" | "Tom") { 48 } else { 60 }, 2.0, 1.0);
            let level = rms(&l[480..24000]).max(rms(&r[480..24000])).max(rms(&l[24000..48000]));
            let peak = l.iter().chain(&r).fold(0.0f32, |m, x| m.max(x.abs()));
            eprintln!("{name:<14} level {level:.3} peak {peak:.3}");
            assert!(l.iter().chain(&r).all(|x| x.is_finite()), "{name}");
            assert!(level > 0.01, "{name}: {level}");
            assert!(peak < 1.5, "{name}: {peak}");
        }
    }

    #[test]
    fn released_notes_fade_and_channels_are_shared() {
        let p = preset("E.Piano");
        let sr = 48000.0;
        let mut st = ChipFmState::new(sr);
        for k in 0..10 {
            st.note_on(&p, 48 + k * 2, 0.8);
        }
        for k in 0..10 {
            st.note_off(&p, 48 + k * 2);
        }
        let mut tail = Vec::new();
        for _ in 0..(4.0 * sr / 256.0) as usize {
            let (mut l, mut r) = (vec![0.0; 256], vec![0.0; 256]);
            st.render(&p, &mut l, &mut r);
            tail.extend(l);
        }
        assert!(rms(&tail[tail.len() - 4800..]) < 1e-3);
        // Velocity: a soft note is quieter.
        let (loud, _) = run(&p, 60, 0.5, 0.5);
        let mut q = p.clone();
        q[VELOCITY] = 1.0;
        let mut st = ChipFmState::new(sr);
        st.note_on(&q, 60, 0.2);
        let mut soft = Vec::new();
        for _ in 0..90 {
            let (mut l, mut r) = (vec![0.0; 256], vec![0.0; 256]);
            st.render(&q, &mut l, &mut r);
            soft.extend(l);
        }
        assert!(rms(&soft[2400..]) < 0.5 * rms(&loud[2400..]));
    }
}

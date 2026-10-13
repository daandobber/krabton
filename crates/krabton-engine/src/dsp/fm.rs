//! FM: a 6-operator FM engine that plays DX7 voices, with
//! MiniDexed-style extras (unison, filter, transpose, mono/glide).
//! An original implementation based on the published DX7 behaviour.

use super::dx7::{self, ALGORITHMS};
use super::*;
use crate::params::{ParamSpec, Unit};
use std::sync::OnceLock;

pub const VOLUME: usize = 0;
pub const CUTOFF: usize = 1;
pub const RESO: usize = 2;
pub const BRIGHTNESS: usize = 3;
pub const ATTACK: usize = 4;
pub const RELEASE: usize = 5;
pub const UNISON: usize = 6;
pub const DETUNE: usize = 7;
pub const SPREAD: usize = 8;
pub const TRANSPOSE: usize = 9;
pub const MODE: usize = 10;
pub const GLIDE: usize = 11;

const UNISON_OPTS: &[&str] = &["1", "2", "3", "4"];
const MODES: &[&str] = &["Poly", "Mono"];

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::lin("Volume", "Global", -36.0, 6.0, -3.0, Unit::Db),
    ParamSpec::log("Cutoff", "Tone", 20.0, 20000.0, 20000.0, Unit::Hz),
    ParamSpec::lin("Resonance", "Tone", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Brightness", "Tone", -1.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Attack", "Shape", -1.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Release", "Shape", -1.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::choice("Unison", "Unison", UNISON_OPTS, 0),
    ParamSpec::lin("Detune", "Unison", 0.0, 50.0, 12.0, Unit::Cents),
    ParamSpec::lin("Spread", "Unison", 0.0, 1.0, 0.7, Unit::Percent),
    ParamSpec::int("Transpose", "Global", -24.0, 24.0, 0.0, Unit::Semi),
    ParamSpec::choice("Voices", "Global", MODES, 0),
    ParamSpec::log("Glide", "Global", 0.001, 1.0, 0.001, Unit::Sec),
];

const MAX_VOICES: usize = 16;
const CHUNK: usize = 16;
const SINE_BITS: usize = 12;
const SINE_SIZE: usize = 1 << SINE_BITS;
/// Phase deviation (in cycles) per unit of operator output. Operator
/// output reaches about 2.2 at full level, a modulation index near 4.4 pi.
const MOD_SCALE: f32 = 1.0;
/// Levels are in units of 0.75 dB; this is where an operator falls silent
/// (EG level + output level below it are inaudible).
const LEVEL_FLOOR: f32 = 133.0;
/// Rising segments first jump to this level (about -50 dB).
const ATTACK_JUMP: f32 = 53.6;

fn sine_table() -> &'static [f32] {
    static T: OnceLock<Vec<f32>> = OnceLock::new();
    T.get_or_init(|| (0..=SINE_SIZE).map(|i| (i as f32 / SINE_SIZE as f32 * TAU).sin()).collect())
}

#[inline]
fn sine(t: &[f32], phase: f32) -> f32 {
    let p = phase - phase.floor();
    let x = p * SINE_SIZE as f32;
    let i = (x as usize).min(SINE_SIZE - 1);
    let f = x - i as f32;
    t[i] + (t[i + 1] - t[i]) * f
}

/// DX7 output/EG level (0..99) to the internal 0..127 log scale.
fn scale_level(l: u8) -> f32 {
    const LOW: [u8; 20] = [0, 5, 9, 13, 17, 20, 23, 25, 27, 29, 31, 33, 35, 37, 39, 41, 42, 43, 45, 46];
    if l < 20 {
        LOW[l as usize] as f32
    } else {
        (l.min(99) + 28) as f32
    }
}

/// EG speed in level units per second for a rate (0..99 plus scaling).
/// The DX7 doubles its envelope speed every ~6 rate steps.
fn eg_speed(rate: f32) -> f32 {
    let q = ((rate.clamp(0.0, 127.0) * 41.0 / 64.0) as i32).min(63);
    (4 + (q & 3)) as f32 * 2f32.powi(q >> 2) * 0.0841
}

/// Pitch EG speed (pitch level units per second).
fn decay_speed(rate: f32) -> f32 {
    0.217 * 2f32.powf(rate.clamp(0.0, 127.0) / 6.24)
}

/// Operator level (units of 0.75 dB above silence) to a linear amplitude.
#[inline]
fn level_gain(level: f32) -> f32 {
    if level <= 0.5 {
        0.0
    } else {
        2f32.powf(level / 8.0 - 14.0)
    }
}

/// Velocity sensitivity offset in level units (like the DX7's velocity curve).
fn velocity_offset(kvs: u8, vel: f32) -> f32 {
    let data = 254.0 * vel.clamp(0.0, 1.0).powf(0.3);
    kvs as f32 * (data - 239.0) / 8.0
}

fn pitch_eg_semis(level: f32) -> f32 {
    let x = (level - 50.0) / 49.0;
    42.0 * x * x * x + 6.0 * x
}

fn lfo_hz(speed: u8) -> f32 {
    let s = speed as f32;
    if s <= 66.0 {
        0.062 + 0.15 * s
    } else {
        10.0 * 4.9f32.powf((s - 66.0) / 33.0)
    }
}

const PMS_SEMIS: [f32; 8] = [0.0, 0.6, 1.0, 1.6, 2.6, 4.2, 7.0, 12.0];
const AMS_DEPTH: [f32; 4] = [0.0, 0.33, 0.66, 1.0];

/// Keyboard level scaling offset in level units (negative attenuates).
fn kbd_scaling(note: i32, bp: u8, ld: u8, rd: u8, lc: u8, rc: u8) -> f32 {
    // Exponential curve shape per group of three semitones.
    const EXP: [f32; 33] = [
        0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 11.0, 14.0, 16.0, 19.0, 23.0, 27.0, 33.0, 39.0, 47.0, 56.0, 66.0, 80.0, 94.0, 110.0, 126.0,
        142.0, 158.0, 174.0, 190.0, 206.0, 222.0, 238.0, 250.0,
    ];
    let bp_note = bp as i32 + 21;
    let (dist, depth, curve) = if note < bp_note { ((bp_note - note) as f32 / 3.0, ld, lc) } else { ((note - bp_note) as f32 / 3.0, rd, rc) };
    if depth == 0 || dist <= 0.0 {
        return 0.0;
    }
    let mag = match curve {
        1 | 2 => EXP[(dist as usize).min(32)] * depth as f32 * 0.01,
        _ => dist * depth as f32 * 0.0803,
    }
    .min(127.0);
    if curve <= 1 {
        -mag
    } else {
        mag
    }
}

#[derive(Clone, Copy, Default)]
struct Op {
    on: bool,
    phase: f32,
    ratio: f32,
    fixed_hz: f32,
    level: f32,
    stage: u8,
    targets: [f32; 4],
    rates: [f32; 4],
    out_level: f32,
    ams: f32,
    gain: f32,
}

impl Op {
    /// Advances the envelope by `dt` seconds. `level` and `targets` are the
    /// combined EG + output level; rising segments speed up at low levels.
    fn eg(&mut self, dt: f32) {
        let s = self.stage as usize;
        if s >= 4 {
            return;
        }
        let target = self.targets[s];
        let speed = self.rates[s];
        if self.level < target {
            if self.level < ATTACK_JUMP {
                self.level = ATTACK_JUMP.min(target);
            }
            self.level += speed * (17.0 - self.level / 8.0).max(1.0) * dt;
            if self.level >= target {
                self.level = target;
                self.next_stage();
            }
        } else {
            self.level -= speed * dt;
            if self.level <= target {
                self.level = target;
                self.next_stage();
            }
        }
    }

    fn next_stage(&mut self) {
        self.stage = match self.stage {
            0 => 1,
            1 => 2,
            2 => 4, // hold at L3 until key off
            _ => 5, // release finished
        };
    }
}

#[derive(Clone, Copy)]
struct Voice {
    active: bool,
    released: bool,
    note: u8,
    age: u64,
    ops: [Op; 6],
    pitch_level: f32,
    pitch_stage: u8,
    pitch_targets: [f32; 4],
    pitch_rates: [f32; 4],
    /// Note (with transpose and unison detune) and its glide position.
    target_note: f32,
    cur_note: f32,
    pan: (f32, f32),
    fb: [f32; 2],
}

impl Default for Voice {
    fn default() -> Self {
        Self {
            active: false,
            released: false,
            note: 0,
            age: 0,
            ops: [Op::default(); 6],
            pitch_level: 50.0,
            pitch_stage: 4,
            pitch_targets: [50.0; 4],
            pitch_rates: [99.0; 4],
            target_note: 60.0,
            cur_note: 60.0,
            pan: (1.0, 1.0),
            fb: [0.0; 2],
        }
    }
}

impl Voice {
    fn release(&mut self) {
        if self.active && !self.released {
            self.released = true;
            for op in self.ops.iter_mut() {
                op.stage = 3;
            }
            self.pitch_stage = 3;
        }
    }

    fn pitch_eg(&mut self, dt: f32) {
        let s = self.pitch_stage as usize;
        if s >= 4 {
            return;
        }
        let target = self.pitch_targets[s];
        let speed = decay_speed(self.pitch_rates[s]) * 0.8;
        if (self.pitch_level - target).abs() <= speed * dt {
            self.pitch_level = target;
            self.pitch_stage = match s {
                0 => 1,
                1 => 2,
                _ => 4,
            };
        } else if self.pitch_level < target {
            self.pitch_level += speed * dt;
        } else {
            self.pitch_level -= speed * dt;
        }
    }
}

pub struct FmState {
    /// Pitch bend in semitones (a wheel, a controller's triggers), set by the engine.
    pub pitch_bend: f32,
    voices: [Voice; MAX_VOICES],
    counter: u64,
    lfo_phase: f32,
    lfo_time: f32,
    sh_value: f32,
    rng: Rng,
    filter: [Svf; 2],
    held: Vec<u8>,
}

impl Default for FmState {
    fn default() -> Self {
        Self::new()
    }
}

impl FmState {
    pub fn new() -> Self {
        sine_table();
        Self {
            pitch_bend: 0.0,
            voices: [Voice::default(); MAX_VOICES],
            counter: 0,
            lfo_phase: 0.0,
            lfo_time: 0.0,
            sh_value: 0.0,
            rng: Rng::new(0xDEC0_DE42),
            filter: [Svf::new(); 2],
            held: Vec::with_capacity(16),
        }
    }

    pub fn active_voices(&self) -> usize {
        self.voices.iter().filter(|v| v.active).count()
    }

    fn pick_voice(&self) -> usize {
        if let Some(i) = self.voices.iter().position(|v| !v.active) {
            return i;
        }
        if let Some((i, _)) = self.voices.iter().enumerate().filter(|(_, v)| v.released).min_by_key(|(_, v)| v.age) {
            return i;
        }
        self.voices.iter().enumerate().min_by_key(|(_, v)| v.age).map(|(i, _)| i).unwrap_or(0)
    }

    /// Starts a note. `voice` is a 155-byte DX7 voice, `op_mask` the
    /// enabled operators (bit 0 = OP1).
    pub fn note_on(&mut self, p: &[f32], voice: &[u8], op_mask: u8, note: u8, vel: f32) {
        if voice.len() < dx7::VOICE_BYTES {
            return;
        }
        let mono = p[MODE] >= 0.5;
        let unison = (p[UNISON].round() as usize + 1).clamp(1, 4);
        let glide = p[GLIDE] > 0.0015;
        self.counter += 1;
        if voice[dx7::LFKS] != 0 || self.active_voices() == 0 {
            self.lfo_phase = 0.0;
            self.lfo_time = 0.0;
        }
        if mono {
            self.held.retain(|&n| n != note);
            if self.held.len() < self.held.capacity() {
                self.held.push(note);
            }
        }
        let legato = mono && self.voices.iter().any(|v| v.active && !v.released);
        let prev_note = self.voices.iter().filter(|v| v.active).max_by_key(|v| v.age).map(|v| v.cur_note);
        for u in 0..unison {
            let (detune, pan) = if unison == 1 {
                (0.0, 0.0)
            } else {
                let x = u as f32 / (unison - 1) as f32 * 2.0 - 1.0;
                (x * p[DETUNE] / 100.0, x * p[SPREAD])
            };
            let idx = if mono { u } else { self.pick_voice() };
            let counter = self.counter;
            let v = &mut self.voices[idx];
            let target = note as f32 + (voice[dx7::TRNP] as f32 - 24.0) + p[TRANSPOSE].round() + detune;
            if mono && legato && v.active {
                v.note = note;
                v.target_note = target;
                if !glide {
                    v.cur_note = target;
                }
                v.age = counter;
                continue;
            }
            *v = Voice::default();
            let (pl, pr) = pan_gains(pan);
            v.pan = (pl, pr);
            v.active = true;
            v.note = note;
            v.age = counter;
            v.target_note = target;
            v.cur_note = if glide { prev_note.unwrap_or(target) } else { target };
            setup_voice(v, p, voice, op_mask, note, vel);
        }
    }

    pub fn note_off(&mut self, p: &[f32], note: u8) {
        if p[MODE] >= 0.5 {
            self.held.retain(|&n| n != note);
            if let Some(&back) = self.held.last() {
                // Fall back to the previously held note (legato).
                for v in self.voices.iter_mut().filter(|v| v.active && !v.released) {
                    let delta = back as f32 - v.note as f32;
                    v.note = back;
                    v.target_note += delta;
                    if p[GLIDE] <= 0.0015 {
                        v.cur_note = v.target_note;
                    }
                }
                return;
            }
            for v in self.voices.iter_mut() {
                v.release();
            }
            return;
        }
        for v in self.voices.iter_mut() {
            if v.note == note {
                v.release();
            }
        }
    }

    pub fn all_off(&mut self) {
        self.held.clear();
        for v in self.voices.iter_mut() {
            v.release();
        }
    }

    pub fn render(&mut self, p: &[f32], voice: &[u8], l: &mut [f32], r: &mut [f32], sr: f32) {
        let n = l.len().min(r.len());
        if n == 0 || voice.len() < dx7::VOICE_BYTES {
            return;
        }
        let tab = sine_table();
        let alg = &ALGORITHMS[(voice[dx7::ALG] as usize).min(31)];
        let mut mod_idx = [[0usize; 3]; 6];
        let mut mod_n = [0usize; 6];
        for k in 0..6 {
            for j in 0..6 {
                if alg.mods[k] & (1 << j) != 0 && mod_n[k] < 3 {
                    mod_idx[k][mod_n[k]] = j;
                    mod_n[k] += 1;
                }
            }
        }
        let fb = voice[dx7::FB];
        let fb_amt = if fb == 0 { 0.0 } else { 0.255 * 2f32.powi(fb as i32 - 7) };
        let carriers = alg.carriers;
        let out_gain = db_to_gain(p[VOLUME]) * 0.045;
        // LFO
        let lfo_rate = lfo_hz(voice[dx7::LFS]);
        let lfo_delay = (voice[dx7::LFD] as f32 / 99.0).powi(2) * 5.0;
        let pmd = voice[dx7::LPMD] as f32 / 99.0 * PMS_SEMIS[(voice[dx7::LPMS] as usize).min(7)];
        let amd = voice[dx7::LAMD] as f32 / 99.0;
        let wave = voice[dx7::LFW];
        let glide_coef = if p[GLIDE] > 0.0015 { decay_coef(p[GLIDE], sr / CHUNK as f32) } else { 0.0 };
        let use_filter = p[CUTOFF] < 19_500.0;
        if use_filter {
            for f in &mut self.filter {
                f.set(p[CUTOFF], p[RESO], sr);
            }
        }

        let mut start = 0;
        while start < n {
            let end = (start + CHUNK).min(n);
            let len = end - start;
            let dt = len as f32 / sr;
            // Global LFO value for this chunk.
            let ph = self.lfo_phase;
            let prev_ph = ph;
            self.lfo_phase = wrap_phase(self.lfo_phase + lfo_rate * dt);
            if self.lfo_phase < prev_ph && wave == 5 {
                self.sh_value = self.rng.bipolar();
            }
            let lfo = match wave {
                0 => 1.0 - 4.0 * (ph - 0.5).abs(),
                1 => 1.0 - 2.0 * ph,
                2 => 2.0 * ph - 1.0,
                3 => {
                    if ph < 0.5 {
                        1.0
                    } else {
                        -1.0
                    }
                }
                5 => self.sh_value,
                _ => (ph * TAU).sin(),
            };
            self.lfo_time += dt;
            let fade = if lfo_delay <= 0.0 {
                1.0
            } else if self.lfo_time < lfo_delay {
                0.0
            } else {
                ((self.lfo_time - lfo_delay) / (lfo_delay * 0.5 + 0.01)).min(1.0)
            };
            let lfo = lfo * fade;
            let lfo_uni = (1.0 - lfo) * 0.5 * fade;
            let pitch_lfo = lfo * pmd;

            for v in self.voices.iter_mut().filter(|v| v.active) {
                v.pitch_eg(dt);
                v.cur_note = v.target_note + (v.cur_note - v.target_note) * glide_coef;
                let note = v.cur_note + pitch_eg_semis(v.pitch_level) + pitch_lfo + self.pitch_bend;
                let base_hz = midi_to_freq(note);
                let mut inc = [0.0f32; 6];
                let mut g0 = [0.0f32; 6];
                let mut dg = [0.0f32; 6];
                let mut audible = false;
                for (k, op) in v.ops.iter_mut().enumerate() {
                    if !op.on {
                        continue;
                    }
                    op.eg(dt);
                    let hz = if op.fixed_hz > 0.0 { op.fixed_hz } else { base_hz * op.ratio };
                    inc[k] = (hz / sr).min(0.49);
                    let am = op.ams * amd * lfo_uni * 60.0;
                    let target = level_gain(op.level - am);
                    g0[k] = op.gain;
                    dg[k] = (target - op.gain) / len as f32;
                    op.gain = target;
                    let rising = op.stage == 3 && op.targets[3] > op.level;
                    if carriers & (1 << k) != 0 && (op.level > 16.0 || rising) {
                        audible = true;
                    }
                }
                let (pl, pr) = v.pan;
                for i in 0..len {
                    let mut outs = [0.0f32; 6];
                    let fbv = (v.fb[0] + v.fb[1]) * 0.5 * fb_amt;
                    let mut sum = 0.0;
                    for k in (0..6).rev() {
                        let op = &mut v.ops[k];
                        if !op.on {
                            continue;
                        }
                        let mut m = 0.0;
                        for j in 0..mod_n[k] {
                            m += outs[mod_idx[k][j]];
                        }
                        let mut phase_mod = m * MOD_SCALE;
                        if k == alg.fb_to {
                            phase_mod += fbv;
                        }
                        let g = g0[k] + dg[k] * i as f32;
                        let y = sine(tab, op.phase + phase_mod) * g;
                        outs[k] = y;
                        op.phase += inc[k];
                        if op.phase >= 1.0 {
                            op.phase -= 1.0;
                        }
                        if carriers & (1 << k) != 0 {
                            sum += y;
                        }
                    }
                    v.fb[1] = v.fb[0];
                    v.fb[0] = outs[alg.fb_from];
                    let s = sum * out_gain;
                    l[start + i] += s * pl;
                    r[start + i] += s * pr;
                }
                if v.released && !audible {
                    v.active = false;
                }
            }
            start = end;
        }
        if use_filter {
            for i in 0..n {
                l[i] = self.filter[0].process(l[i], FilterMode::Low);
                r[i] = self.filter[1].process(r[i], FilterMode::Low);
            }
        }
    }
}

/// Note-on setup: frequencies, envelope targets and level offsets.
fn setup_voice(v: &mut Voice, p: &[f32], voice: &[u8], op_mask: u8, note: u8, vel: f32) {
    let alg = &ALGORITHMS[(voice[dx7::ALG] as usize).min(31)];
    let brightness = p[BRIGHTNESS] * 20.0;
    let attack = p[ATTACK] * 40.0;
    let release = p[RELEASE] * 40.0;
    for k in 0..6 {
        let at = |param: usize| voice[dx7::Dx7Voice::op_offset(k, param)];
        let op = &mut v.ops[k];
        *op = Op::default();
        op.on = op_mask & (1 << k) != 0;
        let coarse = at(dx7::PC) as f32;
        let fine = at(dx7::PF) as f32;
        if at(dx7::PM) == 1 {
            op.fixed_hz = 10f32.powf((at(dx7::PC) & 3) as f32 + fine / 100.0);
        } else {
            let ratio = if coarse == 0.0 { 0.5 } else { coarse };
            let detune = (at(dx7::PD) as f32 - 7.0) * 1.5;
            op.ratio = ratio * (1.0 + fine / 100.0) * 2f32.powf(detune / 1200.0);
        }
        let carrier = alg.carriers & (1 << k) != 0;
        let kvs = at(dx7::KVS);
        let vel_offset = velocity_offset(kvs, vel);
        let ks = kbd_scaling(note as i32, at(dx7::BP), at(dx7::LD), at(dx7::RD), at(dx7::LC), at(dx7::RC));
        let bright = if carrier { 0.0 } else { brightness };
        op.out_level = (scale_level(at(dx7::OL)) + vel_offset + ks + bright).clamp(0.0, 127.0);
        // Rate scaling: higher notes get faster envelopes (in rate units).
        let rate_add = at(dx7::RS) as f32 * (note as f32 / 3.0 - 7.0).max(0.0) * 0.195;
        for s in 0..4 {
            op.targets[s] = (scale_level(at(dx7::L1 + s)) + op.out_level - LEVEL_FLOOR).max(0.0);
            let mut rate = at(dx7::R1 + s) as f32 + rate_add;
            if carrier && s == 0 {
                rate += attack;
            }
            if s == 3 {
                rate += release;
            }
            op.rates[s] = eg_speed(rate);
        }
        op.level = 0.0;
        op.stage = 0;
        op.ams = AMS_DEPTH[(at(dx7::AMS) as usize).min(3)];
        op.phase = 0.0;
    }
    for s in 0..4 {
        v.pitch_targets[s] = voice[dx7::PL1 + s] as f32;
        v.pitch_rates[s] = voice[dx7::PR1 + s] as f32;
    }
    v.pitch_level = voice[dx7::PL1 + 3] as f32;
    v.pitch_stage = 0;
    let _ = p;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::defaults;

    fn render_note(voice: &dx7::Dx7Voice, note: u8, hold: f32, total: f32) -> (f32, Vec<f32>) {
        let p = defaults(SPECS);
        let mut s = FmState::new();
        s.note_on(&p, &voice.0, 0x3f, note, 0.8);
        let sr = 48000.0;
        let mut out = Vec::new();
        let block = 256;
        let mut t = 0.0;
        let mut released = false;
        while t < total {
            if !released && t >= hold {
                s.note_off(&p, note);
                released = true;
            }
            let mut l = vec![0.0; block];
            let mut r = vec![0.0; block];
            s.render(&p, &voice.0, &mut l, &mut r, sr);
            out.extend_from_slice(&l);
            t += block as f32 / sr;
        }
        (out.iter().fold(0.0f32, |m, v| m.max(v.abs())), out)
    }

    #[test]
    fn init_voice_is_a_sine_at_the_right_pitch() {
        let v = dx7::init_voice();
        let (peak, out) = render_note(&v, 69, 0.5, 0.5);
        assert!(peak > 0.05 && peak < 0.5, "peak {peak}");
        // Count zero crossings over the sustained part: 440 Hz.
        let seg = &out[4800..4800 + 48000 / 10];
        let crossings = seg.windows(2).filter(|w| w[0] <= 0.0 && w[1] > 0.0).count();
        assert!((43..=45).contains(&crossings), "crossings {crossings}");
    }

    #[test]
    fn factory_voices_sound_and_end() {
        for v in dx7::factory_bank() {
            let (peak, out) = render_note(&v, 60, 2.0, 16.0);
            assert!(peak > 0.02, "{} too quiet: {peak}", v.name());
            assert!(peak < 1.5, "{} too loud: {peak}", v.name());
            let tail = out[out.len() - 2048..].iter().fold(0.0f32, |m, x| m.max(x.abs()));
            assert!(tail < 0.01, "{} does not release: {tail}", v.name());
            assert!(out.iter().all(|x| x.is_finite()));
        }
    }

    #[test]
    fn voices_free_up_after_release() {
        let v = dx7::factory_bank()[0].clone();
        let p = defaults(SPECS);
        let mut s = FmState::new();
        for n in 48..72 {
            s.note_on(&p, &v.0, 0x3f, n, 0.7);
        }
        assert_eq!(s.active_voices(), MAX_VOICES);
        s.all_off();
        let mut l = vec![0.0; 48000];
        let mut r = vec![0.0; 48000];
        for _ in 0..8 {
            s.render(&p, &v.0, &mut l, &mut r, 48000.0);
        }
        assert_eq!(s.active_voices(), 0);
    }

    #[test]
    fn unison_uses_more_voices() {
        let v = dx7::init_voice();
        let mut p = defaults(SPECS);
        p[UNISON] = 3.0;
        let mut s = FmState::new();
        s.note_on(&p, &v.0, 0x3f, 60, 0.8);
        assert_eq!(s.active_voices(), 4);
    }
}

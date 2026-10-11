//! Synth: a two-oscillator virtual analog polysynth.

use super::*;
use crate::params::{ParamSpec, Unit};

pub const OSC1_WAVE: usize = 0;
pub const OSC2_WAVE: usize = 1;
pub const OSC2_SEMI: usize = 2;
pub const OSC2_FINE: usize = 3;
pub const OSC_MIX: usize = 4;
pub const SUB: usize = 5;
pub const NOISE: usize = 6;
pub const UNISON: usize = 7;
pub const FILTER_MODE: usize = 8;
pub const CUTOFF: usize = 9;
pub const RESO: usize = 10;
pub const FENV_AMT: usize = 11;
pub const KEYTRACK: usize = 12;
pub const DRIVE: usize = 13;
pub const AMP_A: usize = 14;
pub const AMP_D: usize = 15;
pub const AMP_S: usize = 16;
pub const AMP_R: usize = 17;
pub const FLT_A: usize = 18;
pub const FLT_D: usize = 19;
pub const FLT_S: usize = 20;
pub const FLT_R: usize = 21;
pub const LFO_RATE: usize = 22;
pub const LFO_CUTOFF: usize = 23;
pub const LFO_PITCH: usize = 24;
pub const VOICE_MODE: usize = 25;
pub const GLIDE: usize = 26;
pub const VOLUME: usize = 27;

const WAVES: &[&str] = &["Saw", "Square", "Triangle", "Sine"];
const FMODES: &[&str] = &["Lowpass", "Highpass", "Bandpass", "Notch"];
const VMODES: &[&str] = &["Poly", "Mono"];

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::choice("Osc 1", "Oscillators", WAVES, 0),
    ParamSpec::choice("Osc 2", "Oscillators", WAVES, 1),
    ParamSpec::int("Osc 2 Pitch", "Oscillators", -24.0, 24.0, 0.0, Unit::Semi),
    ParamSpec::lin("Osc 2 Fine", "Oscillators", -50.0, 50.0, 7.0, Unit::Cents),
    ParamSpec::lin("Osc Mix", "Oscillators", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Sub", "Oscillators", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Noise", "Oscillators", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Unison", "Oscillators", 0.0, 50.0, 0.0, Unit::Cents),
    ParamSpec::choice("Type", "Filter", FMODES, 0),
    ParamSpec::log("Cutoff", "Filter", 20.0, 20000.0, 2000.0, Unit::Hz),
    ParamSpec::lin("Resonance", "Filter", 0.0, 1.0, 0.2, Unit::Percent),
    ParamSpec::lin("Env Amt", "Filter", -1.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::lin("Key Track", "Filter", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Drive", "Filter", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::log("Attack", "Amp Env", 0.001, 5.0, 0.003, Unit::Sec),
    ParamSpec::log("Decay", "Amp Env", 0.005, 10.0, 0.4, Unit::Sec),
    ParamSpec::lin("Sustain", "Amp Env", 0.0, 1.0, 0.7, Unit::Percent),
    ParamSpec::log("Release", "Amp Env", 0.005, 10.0, 0.25, Unit::Sec),
    ParamSpec::log("Attack", "Filter Env", 0.001, 5.0, 0.002, Unit::Sec),
    ParamSpec::log("Decay", "Filter Env", 0.005, 10.0, 0.4, Unit::Sec),
    ParamSpec::lin("Sustain", "Filter Env", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::log("Release", "Filter Env", 0.005, 10.0, 0.3, Unit::Sec),
    ParamSpec::log("Rate", "LFO", 0.05, 20.0, 4.0, Unit::Hz),
    ParamSpec::lin("To Cutoff", "LFO", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("To Pitch", "LFO", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::choice("Voices", "Global", VMODES, 0),
    ParamSpec::log("Glide", "Global", 0.001, 1.0, 0.001, Unit::Sec),
    ParamSpec::lin("Volume", "Global", -36.0, 6.0, -6.0, Unit::Db),
];

pub fn preset_names() -> &'static [&'static str] {
    &["Init", "Fat Bass", "Acid", "Sub Bass", "Soft Pad", "Pluck", "Lead", "Keys"]
}

pub fn preset(name: &str) -> Vec<f32> {
    let mut p = crate::params::defaults(SPECS);
    let mut set = |pairs: &[(usize, f32)]| {
        for &(i, v) in pairs {
            p[i] = v;
        }
    };
    match name {
        "Fat Bass" => set(&[
            (OSC1_WAVE, 0.0),
            (OSC2_WAVE, 1.0),
            (OSC2_SEMI, -12.0),
            (OSC2_FINE, 5.0),
            (OSC_MIX, 0.4),
            (SUB, 0.5),
            (CUTOFF, 300.0),
            (RESO, 0.3),
            (FENV_AMT, 0.45),
            (KEYTRACK, 0.3),
            (DRIVE, 0.3),
            (AMP_A, 0.002),
            (AMP_D, 0.3),
            (AMP_S, 0.8),
            (AMP_R, 0.08),
            (FLT_A, 0.001),
            (FLT_D, 0.25),
            (FLT_S, 0.1),
            (FLT_R, 0.1),
            (VOICE_MODE, 1.0),
            (GLIDE, 0.03),
            (VOLUME, -8.0),
        ]),
        "Acid" => set(&[
            (OSC1_WAVE, 0.0),
            (OSC_MIX, 0.0),
            (CUTOFF, 380.0),
            (RESO, 0.85),
            (FENV_AMT, 0.6),
            (KEYTRACK, 0.4),
            (DRIVE, 0.5),
            (AMP_A, 0.002),
            (AMP_D, 0.4),
            (AMP_S, 0.8),
            (AMP_R, 0.05),
            (FLT_D, 0.18),
            (FLT_S, 0.0),
            (VOICE_MODE, 1.0),
            (GLIDE, 0.06),
            (VOLUME, -10.0),
        ]),
        "Sub Bass" => set(&[
            (OSC1_WAVE, 3.0),
            (OSC2_WAVE, 2.0),
            (OSC_MIX, 0.15),
            (CUTOFF, 900.0),
            (RESO, 0.0),
            (FENV_AMT, 0.0),
            (AMP_A, 0.003),
            (AMP_D, 0.5),
            (AMP_S, 1.0),
            (AMP_R, 0.08),
            (VOICE_MODE, 1.0),
            (VOLUME, -4.0),
        ]),
        "Soft Pad" => set(&[
            (OSC1_WAVE, 0.0),
            (OSC2_WAVE, 0.0),
            (OSC2_FINE, 9.0),
            (OSC_MIX, 0.5),
            (UNISON, 18.0),
            (CUTOFF, 1400.0),
            (RESO, 0.15),
            (FENV_AMT, 0.25),
            (AMP_A, 0.8),
            (AMP_D, 1.5),
            (AMP_S, 0.8),
            (AMP_R, 1.8),
            (FLT_A, 1.2),
            (FLT_D, 2.0),
            (FLT_S, 0.5),
            (FLT_R, 1.5),
            (LFO_RATE, 0.3),
            (LFO_CUTOFF, 0.25),
            (VOLUME, -14.0),
        ]),
        "Pluck" => set(&[
            (OSC1_WAVE, 0.0),
            (OSC2_WAVE, 1.0),
            (OSC2_SEMI, 12.0),
            (OSC_MIX, 0.3),
            (UNISON, 8.0),
            (CUTOFF, 900.0),
            (RESO, 0.3),
            (FENV_AMT, 0.55),
            (AMP_A, 0.001),
            (AMP_D, 0.35),
            (AMP_S, 0.0),
            (AMP_R, 0.3),
            (FLT_A, 0.001),
            (FLT_D, 0.2),
            (FLT_S, 0.0),
            (FLT_R, 0.2),
            (VOLUME, -10.0),
        ]),
        "Lead" => set(&[
            (OSC1_WAVE, 0.0),
            (OSC2_WAVE, 0.0),
            (OSC2_FINE, 12.0),
            (OSC_MIX, 0.5),
            (UNISON, 10.0),
            (CUTOFF, 3500.0),
            (RESO, 0.25),
            (FENV_AMT, 0.2),
            (AMP_A, 0.01),
            (AMP_D, 0.2),
            (AMP_S, 0.8),
            (AMP_R, 0.25),
            (LFO_RATE, 5.5),
            (LFO_PITCH, 0.1),
            (VOICE_MODE, 1.0),
            (GLIDE, 0.05),
            (VOLUME, -12.0),
        ]),
        "Keys" => set(&[
            (OSC1_WAVE, 2.0),
            (OSC2_WAVE, 3.0),
            (OSC2_SEMI, 12.0),
            (OSC2_FINE, 0.0),
            (OSC_MIX, 0.35),
            (CUTOFF, 2500.0),
            (FENV_AMT, 0.3),
            (AMP_A, 0.002),
            (AMP_D, 1.2),
            (AMP_S, 0.3),
            (AMP_R, 0.4),
            (FLT_D, 0.6),
            (FLT_S, 0.2),
            (VOLUME, -10.0),
        ]),
        _ => {}
    }
    p
}

const MAX_VOICES: usize = 16;
const CHUNK: usize = 16;

#[derive(Clone, Copy)]
struct Voice {
    active: bool,
    released: bool,
    note: u8,
    vel: f32,
    age: u64,
    /// osc1 L, osc1 R, osc2 L, osc2 R
    phase: [f32; 4],
    sub_phase: f32,
    freq: f32,
    target: f32,
    amp: Adsr,
    fenv: Adsr,
    filt: [Svf; 2],
    rng: Rng,
    /// The filter's cutoff at the last control step (Hz), for displays.
    fc: f32,
}

impl Voice {
    fn new(seed: u32) -> Self {
        Self {
            active: false,
            released: false,
            note: 0,
            vel: 0.0,
            age: 0,
            phase: [0.0; 4],
            sub_phase: 0.0,
            freq: 440.0,
            target: 440.0,
            amp: Adsr::new(),
            fenv: Adsr::new(),
            filt: [Svf::new(); 2],
            rng: Rng::new(seed),
            fc: 1000.0,
        }
    }
}

pub struct SynthState {
    voices: [Voice; MAX_VOICES],
    lfo_phase: f32,
    counter: u64,
    last_freq: f32,
    held: [u8; 16],
    held_len: usize,
}

impl Default for SynthState {
    fn default() -> Self {
        Self::new()
    }
}

impl SynthState {
    pub fn new() -> Self {
        let mut voices = [Voice::new(1); MAX_VOICES];
        for (i, v) in voices.iter_mut().enumerate() {
            v.rng = Rng::new(0x1234_5678 ^ (i as u32 * 7919 + 1));
        }
        Self { voices, lfo_phase: 0.0, counter: 0, last_freq: 220.0, held: [0; 16], held_len: 0 }
    }

    /// For the display, laid out as `AnalogState::visual`.
    pub fn visual(&self, out: &mut [f32; crate::engine::INST_VALS]) {
        // The newest voices first, without allocating (this runs on the audio thread).
        let mut newest: [Option<&Voice>; 6] = [None; 6];
        let mut below = u64::MAX;
        for slot in newest.iter_mut() {
            *slot = self.voices.iter().filter(|v| v.active && v.age < below).max_by_key(|v| v.age);
            match slot {
                Some(v) => below = v.age,
                None => break,
            }
        }
        let on = || newest.iter().map_while(|v| *v);
        let stage = |s: Stage| match s {
            Stage::Idle => 0.0,
            Stage::Attack => 1.0,
            Stage::Decay | Stage::Sustain => 2.0,
            Stage::Release => 3.0,
        };
        out[0] = on().count() as f32;
        for (k, v) in on().enumerate() {
            out[1 + k * 4] = v.amp.level;
            out[2 + k * 4] = stage(v.amp.stage);
            out[3 + k * 4] = v.fenv.level;
            out[4 + k * 4] = stage(v.fenv.stage);
        }
        if let Some(v) = newest[0] {
            out[25] = v.fc;
            out[26] = v.note as f32;
        }
    }

    pub fn active_voices(&self) -> usize {
        self.voices.iter().filter(|v| v.active).count()
    }

    pub fn note_on(&mut self, p: &[f32], note: u8, vel: f32) {
        let mono = p[VOICE_MODE] >= 0.5;
        let glide = p[GLIDE] > 0.0015;
        let target = midi_to_freq(note as f32);
        self.counter += 1;
        if mono {
            if self.held_len < self.held.len() {
                self.held[self.held_len] = note;
                self.held_len += 1;
            }
            let last = self.last_freq;
            let v = &mut self.voices[0];
            let legato = v.active && !v.released;
            if !v.active {
                v.freq = if glide { last } else { target };
            } else if !glide {
                v.freq = target;
            }
            v.target = target;
            v.note = note;
            if !legato {
                v.amp.trigger();
                v.fenv.trigger();
                v.vel = vel;
            }
            v.active = true;
            v.released = false;
            v.age = self.counter;
        } else {
            let idx = self.pick_voice();
            let last = self.last_freq;
            let counter = self.counter;
            let v = &mut self.voices[idx];
            if !v.active {
                for f in &mut v.filt {
                    f.reset();
                }
                let r = v.rng.next_u32();
                v.phase = [0.0, (r & 0xff) as f32 / 256.0, 0.25, ((r >> 8) & 0xff) as f32 / 256.0];
                v.sub_phase = 0.0;
            }
            v.freq = if glide { last } else { target };
            v.target = target;
            v.note = note;
            v.vel = vel;
            v.amp.trigger();
            v.fenv.trigger();
            v.active = true;
            v.released = false;
            v.age = counter;
        }
        self.last_freq = target;
    }

    fn pick_voice(&self) -> usize {
        if let Some(i) = self.voices.iter().position(|v| !v.active) {
            return i;
        }
        let released = self
            .voices
            .iter()
            .enumerate()
            .filter(|(_, v)| v.released)
            .min_by(|a, b| a.1.amp.level.total_cmp(&b.1.amp.level))
            .map(|(i, _)| i);
        if let Some(i) = released {
            return i;
        }
        self.voices.iter().enumerate().min_by_key(|(_, v)| v.age).map(|(i, _)| i).unwrap_or(0)
    }

    pub fn note_off(&mut self, p: &[f32], note: u8) {
        let mono = p[VOICE_MODE] >= 0.5;
        if mono {
            if let Some(pos) = self.held[..self.held_len].iter().rposition(|&n| n == note) {
                self.held.copy_within(pos + 1..self.held_len, pos);
                self.held_len -= 1;
            }
            let v = &mut self.voices[0];
            if v.note == note && v.active {
                if self.held_len > 0 {
                    let back = self.held[self.held_len - 1];
                    v.note = back;
                    v.target = midi_to_freq(back as f32);
                    if p[GLIDE] <= 0.0015 {
                        v.freq = v.target;
                    }
                    self.last_freq = v.target;
                } else {
                    v.amp.release();
                    v.fenv.release();
                    v.released = true;
                }
            }
        }
        for v in self.voices.iter_mut() {
            if v.active && !v.released && v.note == note && !mono {
                v.amp.release();
                v.fenv.release();
                v.released = true;
            }
        }
    }

    pub fn all_off(&mut self) {
        self.held_len = 0;
        for v in self.voices.iter_mut() {
            if v.active {
                v.amp.release();
                v.fenv.release();
                v.released = true;
            }
        }
    }

    pub fn render(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32], sr: f32) {
        let n = l.len().min(r.len());
        if n == 0 {
            return;
        }
        let w1 = Wave::from_index(p[OSC1_WAVE]);
        let w2 = Wave::from_index(p[OSC2_WAVE]);
        let ratio2 = 2f32.powf((p[OSC2_SEMI] + p[OSC2_FINE] / 100.0) / 12.0);
        let uni = p[UNISON] / 100.0 * 0.5;
        let uni_l = 2f32.powf(-uni / 12.0);
        let uni_r = 2f32.powf(uni / 12.0);
        let mix = p[OSC_MIX];
        let sub = p[SUB];
        let noise = p[NOISE] * 0.6;
        let mode = FilterMode::from_index(p[FILTER_MODE]);
        let cutoff = p[CUTOFF];
        let res = p[RESO];
        let env_amt = p[FENV_AMT] * 7.0;
        let keytrack = p[KEYTRACK];
        let drive = p[DRIVE];
        let drive_in = 1.0 + drive * 5.0;
        let drive_out = (1.0 + drive) * 0.7;
        let amp_c = AdsrCoefs::new(p[AMP_A], p[AMP_D], p[AMP_S], p[AMP_R], sr);
        let fenv_c = AdsrCoefs::new(p[FLT_A], p[FLT_D], p[FLT_S], p[FLT_R], sr / CHUNK as f32);
        let lfo_rate = p[LFO_RATE];
        let lfo_cut = p[LFO_CUTOFF] * 4.0;
        let lfo_pitch = p[LFO_PITCH] * 2.0;
        let vol = db_to_gain(p[VOLUME]) * 0.6;
        let glide_coef = if p[GLIDE] > 0.0015 { decay_coef(p[GLIDE], sr / CHUNK as f32) } else { 0.0 };

        let mut start = 0;
        while start < n {
            let end = (start + CHUNK).min(n);
            let len = end - start;
            let lfo = (self.lfo_phase * TAU).sin();
            self.lfo_phase = wrap_phase(self.lfo_phase + lfo_rate * len as f32 / sr);
            let pitch_mod = if lfo_pitch > 0.0 { 2f32.powf(lfo * lfo_pitch / 12.0) } else { 1.0 };

            for v in self.voices.iter_mut().filter(|v| v.active) {
                let fe = v.fenv.next(&fenv_c);
                v.freq = v.target + (v.freq - v.target) * glide_coef;
                let f = v.freq * pitch_mod;
                let oct = fe * env_amt + lfo * lfo_cut + keytrack * (v.note as f32 - 60.0) / 12.0;
                let fc = cutoff * 2f32.powf(oct);
                v.fc = fc;
                v.filt[0].set(fc, res, sr);
                v.filt[1].set(fc, res, sr);
                let base = f / sr;
                let dt = [base * uni_l, base * uni_r, base * ratio2 * uni_l, base * ratio2 * uni_r];
                let dt = dt.map(|d| d.min(0.45));
                let dt_sub = base * 0.5;
                let velg = 0.35 + 0.65 * v.vel;

                for i in start..end {
                    let a = v.amp.next(&amp_c) * velg * vol;
                    let nz = if noise > 0.0 { v.rng.bipolar() * noise } else { 0.0 };
                    let s_sub = if sub > 0.0 { (v.sub_phase * TAU).sin() * sub } else { 0.0 };
                    let mut xl = osc(w1, v.phase[0], dt[0]) * (1.0 - mix) + osc(w2, v.phase[2], dt[2]) * mix + s_sub + nz;
                    let mut xr = osc(w1, v.phase[1], dt[1]) * (1.0 - mix) + osc(w2, v.phase[3], dt[3]) * mix + s_sub + nz;
                    if drive > 0.001 {
                        xl = fast_tanh(xl * drive_in) * drive_out;
                        xr = fast_tanh(xr * drive_in) * drive_out;
                    }
                    let yl = v.filt[0].process(xl, mode);
                    let yr = v.filt[1].process(xr, mode);
                    l[i] += yl * a;
                    r[i] += yr * a;
                    for k in 0..4 {
                        v.phase[k] = wrap_phase(v.phase[k] + dt[k]);
                    }
                    v.sub_phase = wrap_phase(v.sub_phase + dt_sub);
                }
                if !v.amp.is_active() {
                    v.active = false;
                    v.released = false;
                }
            }
            start = end;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_match_spec_len() {
        for name in preset_names() {
            assert_eq!(preset(name).len(), SPECS.len(), "{name}");
        }
    }

    #[test]
    fn synth_makes_sound_and_stops() {
        let p = preset("Pluck");
        let mut s = SynthState::new();
        s.note_on(&p, 60, 1.0);
        let mut l = vec![0.0; 4800];
        let mut r = vec![0.0; 4800];
        s.render(&p, &mut l, &mut r, 48000.0);
        let peak = l.iter().fold(0.0f32, |m, v| m.max(v.abs()));
        assert!(peak > 0.01, "peak {peak}");
        assert!(peak < 2.0);
        s.note_off(&p, 60);
        for _ in 0..40 {
            l.fill(0.0);
            r.fill(0.0);
            s.render(&p, &mut l, &mut r, 48000.0);
        }
        assert_eq!(s.active_voices(), 0);
    }

    #[test]
    fn mono_mode_uses_one_voice() {
        let p = preset("Fat Bass");
        let mut s = SynthState::new();
        s.note_on(&p, 40, 1.0);
        s.note_on(&p, 43, 1.0);
        assert_eq!(s.active_voices(), 1);
        s.note_off(&p, 43);
        assert_eq!(s.voices[0].note, 40);
    }
}

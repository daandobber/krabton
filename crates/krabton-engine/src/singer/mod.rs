//! Singer: sings the lyrics of its notes with UTAU voicebanks, rebuilt for
//! Krabton after how UTAU and OpenUtau sing (OpenUtau: MIT,
//! <https://github.com/openutau/OpenUtau>). A note's lyric picks a sample
//! from the voicebank (its alias in kana, from romaji too; in a VCV bank
//! after the vowel before it, "a か", or "- か" at a phrase's start), cut as
//! the oto.ini says: the consonant played as it was, the vowel stretched for
//! as long as the note is held. The voice is moved to the note's pitch with
//! PSOLA (each pitch period of the recording laid down again at the new
//! pitch), so it stays the singer's; Gender moves its formants. The
//! consonant comes before the beat so the vowel lands on it: clip notes
//! reach the Singer a little early (see `LEAD_SECS`). Notes glide into each
//! other (Portamento), get vibrato after a while, cross-fade over the
//! overlap the voicebank sets. Without a voicebank (or while one is being
//! analysed) it sings with a voice of its own: vowels from formants.

pub mod kana;
pub mod voicebank;

pub use voicebank::VoicebankRef;

use crate::dsp::{db_to_gain, midi_to_freq, Rng, Svf};
use crate::midi_fx::Lyric;
use crate::params::{ParamSpec, Unit};
use std::f32::consts::TAU;

/// How far ahead the engine sends clip notes to the Singer (seconds), so a
/// consonant this long can start before the beat.
pub const LEAD_SECS: f64 = 0.25;

pub const VOLUME: usize = 0;
pub const GENDER: usize = 1;
pub const BREATH: usize = 2;
pub const VIBRATO: usize = 3;
pub const VIB_RATE: usize = 4;
pub const VIB_DELAY: usize = 5;
pub const PORTAMENTO: usize = 6;
pub const CONSONANT: usize = 7;
pub const RELEASE: usize = 8;
pub const TRANSPOSE: usize = 9;
pub const DEFAULT: usize = 10;

/// What a note without a lyric sings.
pub const DEFAULTS: &[&str] = &["a", "la", "u", "n", "o"];

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::lin("Volume", "Output", -30.0, 6.0, -3.0, Unit::Db),
    ParamSpec::lin("Gender", "Voice", -1.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Breath", "Voice", 0.0, 1.0, 0.1, Unit::Percent),
    ParamSpec::lin("Vibrato", "Vibrato", 0.0, 100.0, 25.0, Unit::Cents),
    ParamSpec::lin("Vib Rate", "Vibrato", 3.0, 9.0, 5.5, Unit::Hz),
    ParamSpec::lin("Vib Delay", "Vibrato", 0.0, 1.5, 0.35, Unit::Sec),
    ParamSpec::lin("Portamento", "Voice", 0.0, 0.3, 0.06, Unit::Sec),
    ParamSpec::log("Consonant", "Voice", 0.5, 2.0, 1.0, Unit::Ratio),
    ParamSpec::log("Release", "Output", 0.02, 0.6, 0.08, Unit::Sec),
    ParamSpec::int("Transpose", "Output", -24.0, 24.0, 0.0, Unit::Semi),
    ParamSpec::choice("Default", "Output", DEFAULTS, 0),
];

const VOICES: usize = 3;
const RING: usize = 8192;
/// How far ahead (output frames) a pitch period is laid down: more than half
/// the longest grain, so each grain is laid whole, its rise and its fall.
const LOOK: f64 = 1024.0;

/// Formants of the vowels (Hz): a, i, u, e, o, n.
fn formants(v: char) -> [(f32, f32); 3] {
    match v {
        'i' => [(300.0, 60.0), (2250.0, 90.0), (3000.0, 120.0)],
        'u' => [(330.0, 60.0), (1200.0, 90.0), (2400.0, 120.0)],
        'e' => [(480.0, 70.0), (1900.0, 90.0), (2550.0, 120.0)],
        'o' => [(520.0, 70.0), (880.0, 80.0), (2500.0, 120.0)],
        'n' => [(260.0, 60.0), (1600.0, 150.0), (2600.0, 200.0)],
        _ => [(760.0, 80.0), (1220.0, 90.0), (2600.0, 120.0)],
    }
}

/// The sound before a vowel, for the voice of its own: (noise colour Hz,
/// how long in seconds, voiced as a nasal).
fn consonant_of(lyric: &str) -> Option<(f32, f32, bool)> {
    let l = lyric.trim().to_lowercase();
    let first = l.chars().next()?;
    Some(match first {
        's' | 'z' | 'c' | 'j' => (6000.0, 0.09, false),
        'f' | 'h' => (2500.0, 0.07, false),
        't' | 'd' => (4000.0, 0.025, false),
        'k' | 'g' => (2200.0, 0.03, false),
        'p' | 'b' => (900.0, 0.02, false),
        'm' | 'n' if l.len() > 1 => (300.0, 0.06, true),
        _ => return None,
    })
}

/// What a voice plays: a voicebank's cut (sample frames), or the voice of its own.
#[derive(Clone, Copy)]
enum Source {
    Bank { sample: usize, seg_start: f64, cons_end: f64, seg_end: f64, loop_lo: f64, loop_hi: f64, overlap: f64 },
    Own { vowel: char, cons: Option<(f32, f32, bool)> },
}

#[derive(Clone)]
struct Voice {
    active: bool,
    gate: bool,
    note: u8,
    source: Source,
    /// Output frames since the note came, before it sounds, and before its
    /// release (a note that came early is let go as late).
    t: f64,
    wait: f64,
    release_in: f64,
    early: bool,
    released: bool,
    /// Where in the recording it is (sample frames) and which way the vowel is read.
    a_pos: f64,
    dir: f64,
    next_mark: f64,
    ring: Vec<f32>,
    pitch_from: f32,
    pitch_to: f32,
    vel: f32,
    amp: f32,
    /// Fading out (per output frame) once the next note sounds.
    fade: f32,
    /// The voice of its own: the glottal phase and the formants.
    phase: f32,
    filters: [Svf; 3],
    noise: Svf,
    /// What it sings, for the screen: the voicebank's sample, or the lyric.
    sung: Lyric,
}

impl Voice {
    fn new() -> Self {
        Self {
            active: false,
            gate: false,
            note: 0,
            source: Source::Own { vowel: 'a', cons: None },
            t: 0.0,
            wait: 0.0,
            release_in: 0.0,
            early: false,
            released: false,
            a_pos: 0.0,
            dir: 1.0,
            next_mark: 0.0,
            ring: vec![0.0; RING],
            pitch_from: 60.0,
            pitch_to: 60.0,
            vel: 1.0,
            amp: 0.0,
            fade: 0.0,
            phase: 0.0,
            filters: [Svf::new(); 3],
            noise: Svf::new(),
            sung: Lyric::default(),
        }
    }
}

pub struct SingerState {
    pub pitch_bend: f32,
    sr: f32,
    voices: Vec<Voice>,
    next: Lyric,
    prev_vowel: Option<char>,
    /// Output frames since the last voice went quiet (a phrase starts after a rest).
    quiet: f64,
    last_pitch: Option<f32>,
    rng: Rng,
    breath_bp: Svf,
    /// For the screen: the note sounding, how loud, whether from the voicebank,
    /// and what it sings.
    shown: (f32, f32, f32),
    shown_sung: Lyric,
}

impl SingerState {
    pub fn new(sr: f32) -> Self {
        Self { pitch_bend: 0.0, sr: sr.max(1000.0), voices: (0..VOICES).map(|_| Voice::new()).collect(), next: Lyric::default(), prev_vowel: None, quiet: 1e9, last_pitch: None, rng: Rng::new(0x51_6E_6E), breath_bp: Svf::new(), shown: (0.0, 0.0, 0.0), shown_sung: Lyric::default() }
    }

    /// The lyric of the next note (the engine sets it from the note).
    pub fn set_lyric(&mut self, lyric: Lyric) {
        self.next = lyric;
    }

    pub fn note_on(&mut self, p: &[f32], bank: Option<&VoicebankRef>, note: u8, vel: f32) {
        if p.len() < SPECS.len() {
            return;
        }
        let lyric_buf = std::mem::take(&mut self.next);
        let default = DEFAULTS[(p[DEFAULT].round().max(0.0) as usize).min(DEFAULTS.len() - 1)];
        let lyric = if lyric_buf.is_empty() { default } else { lyric_buf.as_str() };
        let early = lyric_buf.early;
        let key = (note as i32 + p[TRANSPOSE].round() as i32).clamp(0, 127) as u8;
        let sr = self.sr as f64;
        let lead = if early { LEAD_SECS * sr } else { 0.0 };
        // A phrase starts after a rest; within one the voice glides from the note before.
        let phrase_start = self.quiet > 0.15 * sr && !self.voices.iter().any(|v| v.active && !v.released);
        let prev = if phrase_start { None } else { self.prev_vowel };
        let glide_from = if phrase_start { key as f32 } else { self.last_pitch.unwrap_or(key as f32) };
        let analysis = bank.and_then(|b| b.analysis.get().map(|a| (b, a)));
        let source = match analysis.and_then(|(b, a)| b.find(lyric, key, prev).map(|e| (b, a, e))) {
            Some((b, a, e)) if a.get(e.sample).is_some_and(|m| !m.pos.is_empty()) => {
                let s = &b.samples[e.sample];
                let ms = s.sr as f64 / 1000.0;
                let len = s.data.len() as f64;
                let seg_start = (e.offset as f64 * ms).clamp(0.0, len);
                let seg_end = if e.cutoff < 0.0 { seg_start - e.cutoff as f64 * ms } else { len - e.cutoff as f64 * ms }.clamp(seg_start + 1.0, len);
                let cons = if e.consonant > 0.0 { e.consonant as f64 } else { e.preutter.max(0.0) as f64 } * ms;
                let cons_end = (seg_start + cons).min(seg_end);
                let vowel = (seg_end - cons_end).max(1.0);
                Source::Bank { sample: e.sample, seg_start, cons_end, seg_end, loop_lo: cons_end + 0.25 * vowel, loop_hi: seg_end - 0.2 * vowel, overlap: (e.overlap.max(0.0) as f64 / 1000.0 * sr).max(0.012 * sr) }
            }
            _ => Source::Own { vowel: kana::vowel_of(&kana::to_kana(lyric)).or_else(|| kana::vowel_of(lyric)).unwrap_or('a'), cons: consonant_of(lyric) },
        };
        // Where it starts: the consonant before the beat (as long as the note came early enough).
        let cons_speed = p[CONSONANT].clamp(0.25, 4.0) as f64;
        let (wait, a_pos, sung) = match source {
            Source::Bank { sample, seg_start, .. } => {
                let e = analysis.and_then(|(b, _)| b.find(lyric, key, prev)).cloned();
                let sung = Lyric::new(e.as_ref().map_or(lyric, |e| e.alias.as_str()), false);
                let bank_sr = bank.map(|b| b.samples[sample].sr as f64).unwrap_or(44_100.0);
                let pre = e.map(|e| e.preutter.max(0.0) as f64 / 1000.0).unwrap_or(0.05) / cons_speed;
                let before = lead / sr - pre;
                if before >= 0.0 {
                    (before * sr, seg_start, sung)
                } else {
                    (0.0, seg_start + (-before) * bank_sr * cons_speed, sung)
                }
            }
            Source::Own { cons, .. } => {
                let c = cons.map(|c| c.1 as f64).unwrap_or(0.0);
                ((lead - c * sr).max(0.0), 0.0, Lyric::new(lyric, false))
            }
        };
        self.prev_vowel = kana::vowel_of(&kana::to_kana(lyric)).or_else(|| kana::vowel_of(lyric));
        self.last_pitch = Some(key as f32);
        // A free voice, or the quietest.
        let i = self.voices.iter().position(|v| !v.active).unwrap_or_else(|| self.voices.iter().enumerate().min_by(|a, b| a.1.amp.total_cmp(&b.1.amp)).map(|(i, _)| i).unwrap_or(0));
        let v = &mut self.voices[i];
        v.active = true;
        v.gate = true;
        v.note = note;
        v.source = source;
        v.t = 0.0;
        v.wait = wait;
        v.release_in = 0.0;
        v.early = early;
        v.released = false;
        v.a_pos = a_pos;
        v.dir = 1.0;
        v.next_mark = wait;
        v.ring.iter_mut().for_each(|x| *x = 0.0);
        v.pitch_from = glide_from;
        v.pitch_to = key as f32;
        v.vel = 0.6 + 0.4 * vel.clamp(0.0, 1.0);
        v.amp = 0.0;
        v.fade = 0.0;
        v.phase = 0.0;
        v.sung = sung;
        if let Source::Own { vowel, .. } = source {
            for (f, (hz, bw)) in v.filters.iter_mut().zip(formants(vowel)) {
                f.reset();
                f.set_q(hz, hz / bw, self.sr);
            }
        }
    }

    pub fn note_off(&mut self, p: &[f32], note: u8) {
        let _ = p;
        let lead = LEAD_SECS * self.sr as f64;
        for v in self.voices.iter_mut().filter(|v| v.active && v.gate && v.note == note) {
            v.gate = false;
            // A note that came early is let go as much later.
            v.release_in = if v.early { lead } else { 0.0 };
        }
    }

    pub fn all_off(&mut self) {
        for v in self.voices.iter_mut() {
            v.gate = false;
            v.release_in = 0.0;
        }
    }

    /// The screen: [0] on, [1] the note sounding, [2] how loud, [3] 1 when
    /// from the voicebank, [4] how many bytes of what it sings, [5..] them.
    pub fn visual(&self, out: &mut [f32; crate::engine::INST_VALS]) {
        out[0] = 1.0;
        out[1] = self.shown.0;
        out[2] = self.shown.1;
        out[3] = self.shown.2;
        let sung = self.shown_sung.as_str().as_bytes();
        out[4] = sung.len() as f32;
        for (o, b) in out[5..].iter_mut().zip(sung) {
            *o = *b as f32;
        }
    }

    pub fn render(&mut self, p: &[f32], bank: Option<&VoicebankRef>, l: &mut [f32], r: &mut [f32]) {
        if p.len() < SPECS.len() {
            return;
        }
        let sr = self.sr;
        let gain = db_to_gain(p[VOLUME]);
        let gender = 2f32.powf(p[GENDER].clamp(-1.0, 1.0) * 0.35);
        let breath = p[BREATH].clamp(0.0, 1.0);
        let vib_depth = p[VIBRATO].clamp(0.0, 200.0) / 100.0;
        let vib_rate = p[VIB_RATE].clamp(0.5, 12.0);
        let vib_delay = p[VIB_DELAY].clamp(0.0, 4.0) as f64 * sr as f64;
        let porta = (p[PORTAMENTO].clamp(0.0, 1.0) as f64 * sr as f64).max(1.0);
        let release = (p[RELEASE].clamp(0.005, 2.0) * sr).max(1.0);
        let cons_speed = p[CONSONANT].clamp(0.25, 4.0) as f64;
        let analysis = bank.and_then(|b| b.analysis.get().map(|a| (b, a)));
        self.breath_bp.set_q(3000.0, 0.8, sr);
        let n = l.len().min(r.len());
        let mut shown = (0.0f32, 0.0f32, 0.0f32);
        let mut sung = Lyric::default();
        for j in 0..n {
            let mut out = 0.0f32;
            let mut sounding = false;
            // A voice that begins to sound fades the ones before it out over its overlap.
            let mut starts: Option<(usize, f64)> = None;
            for (vi, v) in self.voices.iter_mut().enumerate() {
                if !v.active {
                    continue;
                }
                if v.t < v.wait {
                    v.t += 1.0;
                    if v.t >= v.wait {
                        let ovl = match v.source {
                            Source::Bank { overlap, .. } => overlap,
                            Source::Own { .. } => 0.03 * sr as f64,
                        };
                        starts = Some((vi, ovl));
                    }
                    continue;
                }
                // Let go: after the delay a note that came early waits.
                if !v.gate && !v.released {
                    v.release_in -= 1.0;
                    if v.release_in <= 0.0 {
                        v.released = true;
                    }
                }
                let st = v.t - v.wait;
                // Pitch: the glide from the note before, the vibrato after a while.
                let glide = (st / porta).min(1.0) as f32;
                let mut pitch = v.pitch_from + (v.pitch_to - v.pitch_from) * (glide * glide * (3.0 - 2.0 * glide));
                if st > vib_delay && vib_depth > 0.0 {
                    let ramp = (((st - vib_delay) / (0.25 * sr as f64)).min(1.0)) as f32;
                    pitch += vib_depth * ramp * (TAU * vib_rate * (st as f32 / sr)).sin();
                }
                pitch += self.pitch_bend;
                let f0 = midi_to_freq(pitch);
                // How loud: up quickly, down over the release, out when faded.
                let target = if v.released { 0.0 } else { 1.0 };
                let rate = if v.released { 1.0 / release } else { 1.0 / (0.008 * sr) };
                v.amp += (target - v.amp).clamp(-rate, rate);
                if v.fade > 0.0 {
                    v.amp = (v.amp - v.fade).max(0.0);
                }
                let y = match v.source {
                    Source::Bank { sample, cons_end, seg_end, loop_lo, loop_hi, .. } => {
                        let Some((b, a)) = analysis else {
                            v.active = false;
                            continue;
                        };
                        let (Some(s), Some(m)) = (b.samples.get(sample), a.get(sample)) else {
                            v.active = false;
                            continue;
                        };
                        let rate = s.sr as f64 / sr as f64;
                        let level_in = b.gain / 32768.0;
                        // Through the consonant as recorded, then the vowel, back and forth while held.
                        if v.a_pos < cons_end {
                            v.a_pos += rate * cons_speed;
                        } else if !v.released && loop_hi > loop_lo + 64.0 {
                            v.a_pos += rate * 0.7 * v.dir;
                            if v.a_pos >= loop_hi {
                                v.dir = -1.0;
                            } else if v.a_pos <= loop_lo {
                                v.dir = 1.0;
                            }
                        } else {
                            v.a_pos += rate;
                        }
                        if v.a_pos >= seg_end {
                            v.released = true;
                            v.fade = v.fade.max(1.0 / (0.01 * sr));
                        }
                        // The next pitch periods, laid down at the note's pitch ahead of time.
                        let t_abs = v.t;
                        while v.next_mark <= t_abs + LOOK {
                            let Some(mi) = m.nearest(v.a_pos) else { break };
                            let pa = m.period[mi].max(8) as f64;
                            let pm = m.pos[mi] as f64;
                            let half = pa / rate / gender as f64;
                            let period_out = if m.voiced[mi] { sr as f64 / f0 as f64 } else { pa / rate };
                            let level = if m.voiced[mi] { (period_out / (pa / rate)).clamp(0.35, 1.6) as f32 } else { 1.0 };
                            let start = v.next_mark - half;
                            let len = (2.0 * half) as usize;
                            for k in 0..len.min(RING - 1) {
                                let at = start + k as f64;
                                if at < t_abs {
                                    continue;
                                }
                                let src = pm - pa + k as f64 * rate * gender as f64;
                                if src < 0.0 || src >= (s.data.len() - 1) as f64 {
                                    continue;
                                }
                                let i0 = src as usize;
                                let fr = (src - i0 as f64) as f32;
                                let x = s.data[i0] as f32 + (s.data[i0 + 1] as f32 - s.data[i0] as f32) * fr;
                                let w = 0.5 - 0.5 * (TAU * k as f32 / len as f32).cos();
                                v.ring[(at as usize) % RING] += x * level_in * w * level;
                            }
                            v.next_mark += period_out.max(8.0);
                        }
                        let idx = (t_abs as usize) % RING;
                        let y = v.ring[idx];
                        v.ring[idx] = 0.0;
                        shown.2 = 1.0;
                        y
                    }
                    Source::Own { cons, .. } => {
                        // A glottal pulse (a soft saw) through the vowel's formants.
                        v.phase = (v.phase + f0 / sr).fract();
                        let src = (1.0 - 2.0 * v.phase) * 0.6;
                        let mut y = 0.0;
                        for f in v.filters.iter_mut() {
                            y += f.tick(src).band;
                        }
                        let cons_frames = cons.map(|c| c.1 * sr).unwrap_or(0.0) as f64;
                        if let Some((hz, _, nasal)) = cons {
                            if st < cons_frames {
                                let k = 1.0 - (st / cons_frames) as f32;
                                if nasal {
                                    y = y * 0.3 + (TAU * f0 / sr * st as f32).sin() * 0.25 * k;
                                } else {
                                    v.noise.set_q(hz, 1.2, sr);
                                    y = y * (1.0 - k) + v.noise.tick(self.rng.bipolar()).band * 1.6 * k;
                                }
                            }
                        }
                        y * 0.5
                    }
                };
                let breathy = if breath > 0.0 { self.breath_bp.tick(self.rng.bipolar()).band * breath * 0.08 * v.amp } else { 0.0 };
                out += (y * (1.0 - 0.3 * breath) + breathy) * v.amp * v.vel;
                v.t += 1.0;
                if v.released && v.amp <= 0.0001 {
                    v.active = false;
                }
                if v.active && v.amp > shown.1 {
                    shown.0 = v.pitch_to;
                    shown.1 = v.amp;
                    sung = v.sung;
                }
                sounding |= v.active;
            }
            if let Some((who, ovl)) = starts {
                let fade = 1.0 / ovl.max(1.0) as f32;
                for (vi, v) in self.voices.iter_mut().enumerate() {
                    if vi != who && v.active && v.t >= v.wait {
                        v.fade = v.fade.max(fade);
                        v.gate = false;
                        v.released = true;
                    }
                }
            }
            if sounding {
                self.quiet = 0.0;
            } else {
                self.quiet += 1.0;
            }
            let y = out * gain;
            l[j] += y;
            r[j] += y;
        }
        self.shown = shown;
        self.shown_sung = sung;
    }
}

/// What a Singer's screen values say it sings.
pub fn sung_of(vals: &[f32]) -> String {
    let n = vals.get(4).map_or(0, |n| (*n as usize).min(22));
    let bytes: Vec<u8> = vals.iter().skip(5).take(n).map(|b| *b as u8).collect();
    String::from_utf8(bytes).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::voicebank::{OtoEntry, Sample};
    use super::*;
    use crate::params::defaults;

    const SR: f32 = 48_000.0;

    fn run(st: &mut SingerState, p: &[f32], bank: Option<&VoicebankRef>, secs: f32) -> Vec<f32> {
        let n = (secs * SR) as usize;
        let (mut l, mut r) = (vec![0.0; n], vec![0.0; n]);
        for (a, b) in l.chunks_mut(256).zip(r.chunks_mut(256)) {
            st.render(p, bank, a, b);
        }
        l
    }

    fn mag(x: &[f32], hz: f32) -> f32 {
        let (mut re, mut im) = (0.0f32, 0.0f32);
        for (i, v) in x.iter().enumerate() {
            let a = TAU * hz * i as f32 / SR;
            re += v * a.cos();
            im += v * a.sin();
        }
        (re * re + im * im).sqrt() / x.len() as f32
    }

    /// A voicebank made here: "a" sung at 220 Hz with a vowel-like spectrum.
    fn bank() -> VoicebankRef {
        let sr = 44_100u32;
        let mut data = Vec::new();
        for i in 0..(sr * 2) {
            let t = i as f32 / sr as f32;
            let v: f32 = (1..12).map(|h| (TAU * 220.0 * h as f32 * t).sin() / h as f32 * if h == 3 || h == 4 { 2.0 } else { 1.0 }).sum();
            data.push((v * 4000.0) as i16);
        }
        let a = OtoEntry { alias: "あ".into(), sample: 0, offset: 50.0, consonant: 100.0, cutoff: -1800.0, preutter: 60.0, overlap: 20.0 };
        VoicebankRef::from_parts("Test", vec![Sample { name: "a.wav".into(), sr, data }], vec![a])
    }

    #[test]
    fn it_sings_the_note_s_pitch_from_the_voicebank() {
        let b = bank();
        let mut p = defaults(SPECS);
        p[VIBRATO] = 0.0;
        p[BREATH] = 0.0;
        let mut st = SingerState::new(SR);
        st.set_lyric(Lyric::new("a", false));
        st.note_on(&p, Some(&b), 69, 1.0);
        let x = run(&mut st, &p, Some(&b), 1.0);
        let tail = &x[24_000..];
        assert!(st.shown.2 == 1.0, "it sang from the voicebank");
        // Recorded at 220 Hz, sung at A4: 440 Hz, not 220.
        assert!(mag(tail, 440.0) > 3.0 * mag(tail, 220.0), "{} vs {}", mag(tail, 440.0), mag(tail, 220.0));
        st.note_off(&p, 69);
        run(&mut st, &p, Some(&b), 0.5);
        assert!(run(&mut st, &p, Some(&b), 0.2).iter().all(|v| v.abs() < 1e-3), "and stops");
    }

    /// The biggest step from one sample to the next, against how loud it is.
    fn roughness(x: &[f32]) -> f32 {
        let rms = (x.iter().map(|v| v * v).sum::<f32>() / x.len() as f32).sqrt().max(1e-9);
        x.windows(2).fold(0.0f32, |m, w| m.max((w[1] - w[0]).abs())) / rms
    }

    #[test]
    fn its_grains_join_without_clicks() {
        let b = bank();
        let src: Vec<f32> = b.samples[0].data[4410..80_000].iter().map(|v| *v as f32).collect();
        let smooth = roughness(&src);
        let mut p = defaults(SPECS);
        p[VIBRATO] = 0.0;
        p[BREATH] = 0.0;
        // As recorded, lower, higher: every pitch period laid down whole.
        for note in [57u8, 52, 64] {
            let mut st = SingerState::new(SR);
            st.set_lyric(Lyric::new("a", false));
            st.note_on(&p, Some(&b), note, 1.0);
            let x = run(&mut st, &p, Some(&b), 1.5);
            let r = roughness(&x[12_000..]);
            assert!(r < 2.0 * smooth, "note {note}: steps {r:.2} against the recording's {smooth:.2}");
        }
    }

    #[test]
    fn a_note_that_comes_early_puts_its_vowel_on_the_beat() {
        let b = bank();
        let mut p = defaults(SPECS);
        p[VIBRATO] = 0.0;
        let mut st = SingerState::new(SR);
        st.set_lyric(Lyric::new("a", true));
        st.note_on(&p, Some(&b), 69, 1.0);
        let x = run(&mut st, &p, Some(&b), 0.5);
        // Silent until the consonant starts (the lead less the preutterance), sounding by the beat.
        let quiet = (LEAD_SECS as f32 - 0.06 - 0.02) * SR;
        assert!(x[..quiet as usize].iter().all(|v| v.abs() < 1e-4), "waits for its consonant's start");
        let beat = (LEAD_SECS as f32 * SR) as usize;
        assert!(x[beat..beat + 2400].iter().any(|v| v.abs() > 0.01), "sounds on the beat");
    }

    #[test]
    fn without_a_voicebank_it_sings_its_own_vowels() {
        let mut p = defaults(SPECS);
        p[VIBRATO] = 0.0;
        let mut st = SingerState::new(SR);
        st.set_lyric(Lyric::new("ka", false));
        st.note_on(&p, None, 57, 1.0);
        let x = run(&mut st, &p, None, 0.6);
        assert!(x.iter().all(|v| v.is_finite()) && x[12_000..].iter().map(|v| v * v).sum::<f32>() > 1.0, "it sings");
        assert!(mag(&x[12_000..], 220.0) > mag(&x[12_000..], 233.08) * 3.0, "at its pitch");
    }
}

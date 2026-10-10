//! Shakers: the PhISEM shakers of `shakers` as an instrument played with
//! notes. The key tunes the resonances, a note can shake once, a few times
//! or for as long as it is held, and a second shaker can be layered on top.

use super::biquad::{self, Coefs};
use super::shakers::{ShakeSettings, Shaker, MODELS};
use super::{db_to_gain, decay_coef, fast_tanh, Rng};
use crate::params::{ParamSpec, Unit};

pub const MAX_VOICES: usize = 8;
const CHUNK: usize = 256;

pub const MODEL: usize = 0;
pub const OBJECTS: usize = 1;
pub const LENGTH: usize = 2;
pub const GRAIN: usize = 3;
pub const RING: usize = 4;
pub const SCATTER: usize = 5;
pub const TRANSPOSE: usize = 6;
pub const KEY_TRACK: usize = 7;
pub const SHAKES: usize = 8;
pub const RATE: usize = 9;
pub const HOLD: usize = 10;
pub const HUMANIZE: usize = 11;
pub const SWELL: usize = 12;
pub const FADE: usize = 13;
pub const LAYER: usize = 14;
pub const LAYER_MIX: usize = 15;
pub const LAYER_TUNE: usize = 16;
pub const BRIGHT: usize = 17;
pub const LOW_CUT: usize = 18;
pub const DRIVE: usize = 19;
pub const SPREAD: usize = 20;
pub const VELOCITY: usize = 21;
pub const DAMP: usize = 22;
pub const VOICES: usize = 23;
pub const VOLUME: usize = 24;

/// In the order of `shakers::MODELS`.
pub const MODEL_NAMES: &[&str] = &["Maraca", "Cabasa", "Sekere", "Tambourine", "Sleigh Bells", "Bamboo", "Sandpaper", "Can", "Crunch", "Rocks", "Gravel"];
pub const LAYERS: &[&str] = &["Off", "Maraca", "Cabasa", "Sekere", "Tambourine", "Sleigh Bells", "Bamboo", "Sandpaper", "Can", "Crunch", "Rocks", "Gravel"];

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::choice("Model", "Shaker", MODEL_NAMES, 0),
    ParamSpec::lin("Objects", "Shaker", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Length", "Shaker", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Grain", "Shaker", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Ring", "Shaker", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Scatter", "Shaker", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::int("Transpose", "Pitch", -24.0, 24.0, 0.0, Unit::Semi),
    ParamSpec::lin("Key Track", "Pitch", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::int("Shakes", "Motion", 1.0, 16.0, 1.0, Unit::None),
    ParamSpec::log("Rate", "Motion", 0.5, 30.0, 8.0, Unit::Hz),
    ParamSpec::toggle("Hold", "Motion", false),
    ParamSpec::lin("Humanize", "Motion", 0.0, 1.0, 0.2, Unit::Percent),
    ParamSpec::log("Swell", "Motion", 0.001, 4.0, 0.001, Unit::Sec),
    ParamSpec::lin("Fade", "Motion", -1.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::choice("Layer", "Layer", LAYERS, 0),
    ParamSpec::lin("Layer Mix", "Layer", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::int("Layer Tune", "Layer", -24.0, 24.0, 0.0, Unit::Semi),
    ParamSpec::log("Brightness", "Tone", 200.0, 20000.0, 20000.0, Unit::Hz),
    ParamSpec::log("Low Cut", "Tone", 20.0, 2000.0, 20.0, Unit::Hz),
    ParamSpec::lin("Drive", "Tone", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Spread", "Tone", 0.0, 1.0, 0.4, Unit::Percent),
    ParamSpec::lin("Velocity", "Output", 0.0, 1.0, 0.7, Unit::Percent),
    ParamSpec::lin("Damp", "Output", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::int("Voices", "Output", 1.0, 8.0, 8.0, Unit::None),
    ParamSpec::lin("Volume", "Output", -24.0, 12.0, 0.0, Unit::Db),
];

pub fn preset_names() -> &'static [&'static str] {
    &[
        "Init",
        "Maraca",
        "Maraca Roll",
        "Cabasa",
        "Sekere Groove",
        "Tambourine",
        "Tambourine Roll",
        "Jingle Shaker",
        "Sleigh Bells",
        "Bell Tree",
        "Bamboo Chimes",
        "Rain Stick",
        "Sandpaper",
        "Tin Can",
        "Popcorn",
        "Footsteps",
        "Gravel Walk",
        "Shaker Pad",
        "Ghost Rattle",
        "Lo-Fi Shaker",
    ]
}

pub fn preset(name: &str) -> Vec<f32> {
    let mut p = crate::params::defaults(SPECS);
    let mut set = |pairs: &[(usize, f32)]| {
        for &(i, v) in pairs {
            p[i] = v;
        }
    };
    let model = |name: &str| MODEL_NAMES.iter().position(|m| *m == name).unwrap_or(0) as f32;
    let layer = |name: &str| LAYERS.iter().position(|m| *m == name).unwrap_or(0) as f32;
    let hold = (HOLD, 1.0);
    match name {
        "Maraca" => set(&[(KEY_TRACK, 0.3), (SPREAD, 0.3)]),
        "Maraca Roll" => set(&[hold, (RATE, 9.0), (HUMANIZE, 0.25), (KEY_TRACK, 0.3), (SPREAD, 0.5), (LENGTH, 0.4)]),
        "Cabasa" => set(&[(MODEL, model("Cabasa")), (KEY_TRACK, 0.3), (SPREAD, 0.3), (VOLUME, 2.0)]),
        "Sekere Groove" => set(&[(MODEL, model("Sekere")), (SHAKES, 2.0), (RATE, 7.0), (FADE, 0.5), (KEY_TRACK, 0.3), (SPREAD, 0.4)]),
        "Tambourine" => set(&[(MODEL, model("Tambourine")), (KEY_TRACK, 0.3), (SPREAD, 0.4), (VOLUME, 6.0)]),
        "Tambourine Roll" => set(&[(MODEL, model("Tambourine")), hold, (RATE, 12.0), (HUMANIZE, 0.2), (LENGTH, 0.35), (KEY_TRACK, 0.3), (SPREAD, 0.5)]),
        "Jingle Shaker" => set(&[(LAYER, layer("Tambourine")), (LAYER_MIX, 0.35), (KEY_TRACK, 0.3), (SPREAD, 0.5), (VOLUME, -5.0)]),
        "Sleigh Bells" => set(&[(MODEL, model("Sleigh Bells")), hold, (RATE, 5.0), (HUMANIZE, 0.3), (SPREAD, 0.7), (KEY_TRACK, 0.5)]),
        "Bell Tree" => set(&[(MODEL, model("Sleigh Bells")), (SHAKES, 10.0), (RATE, 16.0), (FADE, 0.3), (HUMANIZE, 0.15), (RING, 0.75), (SPREAD, 1.0)]),
        "Bamboo Chimes" => set(&[(MODEL, model("Bamboo")), (LENGTH, 0.65), (RING, 0.6), (SCATTER, 0.6), (SPREAD, 0.9)]),
        "Rain Stick" => set(&[(MODEL, model("Rocks")), (OBJECTS, 0.8), (LENGTH, 0.8), (SWELL, 0.8), (SCATTER, 0.7), (GRAIN, 0.35), (SPREAD, 0.9), (KEY_TRACK, 0.5), (TRANSPOSE, 5.0), (BRIGHT, 12000.0), (VOLUME, 10.0)]),
        "Sandpaper" => set(&[(MODEL, model("Sandpaper")), hold, (RATE, 2.5), (SWELL, 0.12), (HUMANIZE, 0.2), (KEY_TRACK, 0.2), (SPREAD, 0.2), (VOLUME, 8.0)]),
        "Tin Can" => set(&[(MODEL, model("Can")), (RING, 0.6), (SPREAD, 0.3)]),
        "Popcorn" => set(&[(MODEL, model("Bamboo")), (OBJECTS, 0.25), (GRAIN, 0.15), (RING, 0.2), (SCATTER, 1.0), (LENGTH, 0.6), (TRANSPOSE, 12.0), (SPREAD, 1.0)]),
        "Footsteps" => set(&[(MODEL, model("Crunch")), hold, (RATE, 1.8), (HUMANIZE, 0.15), (KEY_TRACK, 0.2), (BRIGHT, 6000.0), (SPREAD, 0.3), (VOLUME, -3.0)]),
        "Gravel Walk" => set(&[(MODEL, model("Gravel")), hold, (RATE, 1.6), (SWELL, 0.06), (HUMANIZE, 0.2), (LENGTH, 0.35), (KEY_TRACK, 0.2), (SPREAD, 0.4), (VOLUME, 8.0)]),
        "Shaker Pad" => set(&[(MODEL, model("Cabasa")), hold, (RATE, 14.0), (SWELL, 1.5), (HUMANIZE, 0.4), (LAYER, layer("Sekere")), (LAYER_MIX, 0.4), (LAYER_TUNE, 7.0), (SPREAD, 1.0), (BRIGHT, 7000.0), (DAMP, 0.4), (VOLUME, 10.0)]),
        "Ghost Rattle" => set(&[(MODEL, model("Rocks")), hold, (RATE, 3.0), (HUMANIZE, 1.0), (SWELL, 0.4), (SCATTER, 0.9), (SPREAD, 1.0), (LOW_CUT, 300.0), (BRIGHT, 4000.0), (KEY_TRACK, 0.5), (VOLUME, 9.0)]),
        "Lo-Fi Shaker" => set(&[(DRIVE, 0.6), (BRIGHT, 3500.0), (LOW_CUT, 400.0), (KEY_TRACK, 0.3), (SPREAD, 0.0), (VOLUME, 3.0)]),
        _ => {}
    }
    p
}

#[derive(Clone, Copy)]
struct Voice {
    main: Shaker,
    layer: Shaker,
    layered: bool,
    rng: Rng,
    note: u8,
    gate: bool,
    active: bool,
    age: u64,
    amp: f32,
    /// Per-sample fall of `amp` once released (1: rings out).
    damp: f32,
}

impl Voice {
    fn new(i: usize) -> Self {
        Self { main: Shaker::default(), layer: Shaker::default(), layered: false, rng: Rng::new(0x5A4B_E000 + i as u32 * 7919), note: 0, gate: false, active: false, age: 0, amp: 1.0, damp: 1.0 }
    }

    fn is_done(&self) -> bool {
        (self.main.is_done() && (!self.layered || self.layer.is_done())) || self.amp < 1e-4
    }
}

pub struct ShakerSynthState {
    voices: [Voice; MAX_VOICES],
    counter: u64,
    sr: f32,
    buf_l: [f32; CHUNK],
    buf_r: [f32; CHUNK],
    lp: [biquad::State; 2],
    hp: [biquad::State; 2],
}

/// How the parameters play a shaker (the model aside).
fn settings(p: &[f32], tune: f32) -> ShakeSettings {
    ShakeSettings {
        tune,
        decay: 8f32.powf((p[LENGTH] - 0.5) * 2.0),
        tone: p[RING],
        objects: p[OBJECTS],
        grain: p[GRAIN],
        scatter: p[SCATTER],
        shakes: p[SHAKES].round().clamp(1.0, 16.0) as u32,
        rate: p[RATE],
        hold: p[HOLD] >= 0.5,
        humanize: p[HUMANIZE],
        swell: if p[SWELL] > 0.0015 { p[SWELL] } else { 0.0 },
        spread: p[SPREAD],
        fade: p[FADE],
    }
}

impl ShakerSynthState {
    pub fn new(sr: f32) -> Self {
        Self {
            voices: std::array::from_fn(Voice::new),
            counter: 0,
            sr,
            buf_l: [0.0; CHUNK],
            buf_r: [0.0; CHUNK],
            lp: Default::default(),
            hp: Default::default(),
        }
    }

    pub fn note_on(&mut self, p: &[f32], note: u8, vel: f32) {
        if p.len() < SPECS.len() {
            return;
        }
        let poly = (p[VOICES].round() as usize).clamp(1, MAX_VOICES);
        self.counter += 1;
        let pool = &mut self.voices[..poly];
        // A held key shakes its own shaker again; otherwise a free voice,
        // the oldest released one, or the oldest.
        let pick = pool
            .iter()
            .position(|v| v.active && v.gate && v.note == note)
            .or_else(|| pool.iter().position(|v| !v.active))
            .or_else(|| pool.iter().enumerate().filter(|(_, v)| !v.gate).min_by_key(|(_, v)| v.age).map(|(i, _)| i))
            .unwrap_or_else(|| pool.iter().enumerate().min_by_key(|(_, v)| v.age).map(|(i, _)| i).unwrap_or(0));
        let v = &mut pool[pick];
        let energy = 1.0 - p[VELOCITY].clamp(0.0, 1.0) * (1.0 - vel.clamp(0.0, 1.0));
        let tune = 2f32.powf(((note as f32 - 60.0) * p[KEY_TRACK] + p[TRANSPOSE].round()) / 12.0);
        let s = settings(p, tune);
        let m = &MODELS[(p[MODEL].round().max(0.0) as usize).min(MODELS.len() - 1)];
        if !v.active || v.note != note {
            v.main = Shaker::default();
            v.layer = Shaker::default();
        }
        v.main.start(m, energy, &s, self.sr);
        let layer = p[LAYER].round().max(0.0) as usize;
        v.layered = layer > 0;
        if v.layered {
            let lm = &MODELS[(layer - 1).min(MODELS.len() - 1)];
            let ls = ShakeSettings { tune: tune * 2f32.powf(p[LAYER_TUNE].round() / 12.0), ..s };
            v.layer.start(lm, energy, &ls, self.sr);
        }
        v.note = note;
        v.gate = true;
        v.active = true;
        v.age = self.counter;
        v.amp = 1.0;
        v.damp = 1.0;
    }

    pub fn note_off(&mut self, p: &[f32], note: u8) {
        if p.len() < SPECS.len() {
            return;
        }
        let damp = p[DAMP].clamp(0.0, 1.0);
        // Damped, a released note fades in 3 s down to 10 ms.
        let fall = if damp > 0.001 { decay_coef(0.01 + 3.0 * (1.0 - damp).powi(2), self.sr) } else { 1.0 };
        for v in self.voices.iter_mut().filter(|v| v.gate && v.note == note) {
            v.gate = false;
            v.main.release();
            v.layer.release();
            v.damp = fall;
        }
    }

    pub fn all_off(&mut self) {
        for v in self.voices.iter_mut() {
            v.gate = false;
            v.main.release();
            v.layer.release();
        }
    }

    pub fn active_voices(&self) -> usize {
        self.voices.iter().filter(|v| v.active).count()
    }

    pub fn render(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32]) {
        if p.len() < SPECS.len() {
            return;
        }
        if !self.voices.iter().any(|v| v.active) {
            for s in self.lp.iter_mut().chain(self.hp.iter_mut()) {
                s.reset();
            }
            return;
        }
        let n = l.len().min(r.len());
        let master = db_to_gain(p[VOLUME]);
        let mix = p[LAYER_MIX].clamp(0.0, 1.0);
        let (gain_main, gain_layer) = ((2.0 * (1.0 - mix)).min(1.0), (2.0 * mix).min(1.0));
        let sr = self.sr as f64;
        let bright = p[BRIGHT].min(0.45 * self.sr);
        let lp: Option<Coefs> = (bright < 19000.0).then(|| biquad::lowpass(bright as f64, sr, 0.707));
        let hp: Option<Coefs> = (p[LOW_CUT] > 21.0).then(|| biquad::highpass(p[LOW_CUT] as f64, sr, 0.707));
        let drive = p[DRIVE].clamp(0.0, 1.0);
        let push = 1.0 + 7.0 * drive;
        let makeup = 1.0 / push.sqrt();
        let mut done = 0;
        while done < n {
            let len = CHUNK.min(n - done);
            let (bl, br) = (&mut self.buf_l[..len], &mut self.buf_r[..len]);
            bl.fill(0.0);
            br.fill(0.0);
            for v in self.voices.iter_mut().filter(|v| v.active) {
                for i in 0..len {
                    let (mut yl, mut yr) = v.main.tick(&mut v.rng);
                    if v.layered {
                        let (a, b) = v.layer.tick(&mut v.rng);
                        yl = yl * gain_main + a * gain_layer;
                        yr = yr * gain_main + b * gain_layer;
                    }
                    if !v.gate {
                        v.amp *= v.damp;
                    }
                    bl[i] += yl * v.amp;
                    br[i] += yr * v.amp;
                }
                if v.is_done() {
                    v.active = false;
                }
            }
            for (ch, buf) in [&mut *bl, &mut *br].into_iter().enumerate() {
                for x in buf.iter_mut() {
                    let mut y = *x;
                    if drive > 0.0 {
                        y = fast_tanh(y * push) * makeup;
                    }
                    if let Some(c) = &hp {
                        y = self.hp[ch].run(c, y as f64) as f32;
                    }
                    if let Some(c) = &lp {
                        y = self.lp[ch].run(c, y as f64) as f32;
                    }
                    *x = y * master;
                }
            }
            for i in 0..len {
                l[done + i] += bl[i];
                r[done + i] += br[i];
            }
            done += len;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn play(name: &str, note: u8, hold_secs: f32, secs: f32) -> (Vec<f32>, Vec<f32>, ShakerSynthState) {
        let sr = 48000.0;
        let p = preset(name);
        let mut s = ShakerSynthState::new(sr);
        s.note_on(&p, note, 0.9);
        let n = (secs * sr) as usize;
        let (mut l, mut r) = (vec![0.0; n], vec![0.0; n]);
        let off = ((hold_secs * sr) as usize).min(n);
        s.render(&p, &mut l[..off], &mut r[..off]);
        s.note_off(&p, note);
        s.render(&p, &mut l[off..], &mut r[off..]);
        (l, r, s)
    }

    fn peak(v: &[f32]) -> f32 {
        v.iter().fold(0.0f32, |a, x| a.max(x.abs()))
    }

    #[test]
    fn every_preset_sounds_and_stops() {
        for name in preset_names() {
            let (l, r, s) = play(name, 60, 1.0, 12.0);
            let pk = peak(&l).max(peak(&r));
            eprintln!("{name:<16} peak {pk:.3}");
            assert!(l.iter().chain(&r).all(|x| x.is_finite()), "{name}");
            if std::env::var_os("SHAKER_CAL").is_some() {
                continue;
            }
            assert!(pk > 0.05 && pk < 3.0, "{name}: peak {pk}");
            let tail = peak(&l[l.len() - 4800..]).max(peak(&r[r.len() - 4800..]));
            assert!(tail < 0.01, "{name}: still sounding ({tail})");
            assert_eq!(s.active_voices(), 0, "{name}");
        }
    }

    #[test]
    fn a_held_note_keeps_shaking() {
        let (l, _, _) = play("Maraca Roll", 60, 2.0, 2.5);
        // Still going after 1.8 s while held.
        assert!(peak(&l[(1.7 * 48000.0) as usize..(2.0 * 48000.0) as usize]) > 0.05);
        let (l, _, _) = play("Maraca", 60, 2.0, 2.5);
        assert!(peak(&l[(1.7 * 48000.0) as usize..(2.0 * 48000.0) as usize]) < 0.01);
    }

    #[test]
    fn higher_notes_ring_higher() {
        // Zero crossings of a tuned shaker, an octave apart.
        let crossings = |note: u8| {
            let (l, _, _) = play("Tin Can", note, 0.5, 0.5);
            l.windows(2).filter(|w| (w[0] < 0.0) != (w[1] < 0.0)).count()
        };
        let (lo, hi) = (crossings(48), crossings(60));
        assert!(hi as f32 > lo as f32 * 1.5, "{lo} vs {hi}");
    }

    #[test]
    fn damp_cuts_a_released_note() {
        let mut p = preset("Bamboo Chimes");
        let sr = 48000.0;
        let ring = |p: &[f32]| {
            let mut s = ShakerSynthState::new(sr);
            s.note_on(p, 60, 1.0);
            let (mut l, mut r) = (vec![0.0; 4800], vec![0.0; 4800]);
            s.render(p, &mut l, &mut r);
            s.note_off(p, 60);
            let (mut l, mut r) = (vec![0.0; 24000], vec![0.0; 24000]);
            s.render(p, &mut l, &mut r);
            peak(&l[19200..])
        };
        let free = ring(&p);
        p[DAMP] = 1.0;
        let damped = ring(&p);
        assert!(damped < free * 0.1, "{damped} vs {free}");
    }
}

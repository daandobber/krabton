//! What every module does, one sample at a time, in volts.

use super::ModuleKind;
use crate::dsp::effects::FxCtx;
use crate::dsp::fx::plate::{self, Plate};
use crate::dsp::fx::DelayLine;
use crate::dsp::{fast_tanh, pan_gains, Rng, Svf};
use mi_plaits_dsp::drums::analog_bass_drum::AnalogBassDrum;
use mi_plaits_dsp::drums::analog_snare_drum::AnalogSnareDrum;
use mi_plaits_dsp::drums::hihat::{Hihat, NoiseType, VcaType};
use mi_plaits_dsp::voice::{Modulations, Patch as PlaitsPatch, Voice as PlaitsVoice};
use std::f32::consts::{PI, TAU};

/// Blocks of the oscillators and processors that work in blocks.
const PLAITS_BLOCK: usize = 12;
const BRAIDS_OUT: usize = krabton_braids::BLOCK_SIZE / 2;
const ADAPT: usize = 32;
const FIR: usize = crate::dsp::braids_synth::FIR_TAPS;

/// Above this a gate or trigger is high.
const HIGH: f32 = 1.0;
/// 0 V is C4.
pub const C4: f32 = 261.625_58;
/// Length of trigger pulses.
const TRIG_S: f32 = 0.002;

/// The played note of a voice, and the song, for one sample.
#[derive(Clone, Copy, Debug, Default)]
pub struct SampleCtx {
    pub sr: f32,
    pub bpm: f32,
    pub beat: f64,
    pub playing: bool,
    /// The note in volts (1 V/oct, 0 V = C4).
    pub pitch: f32,
    pub gate: bool,
    pub vel: f32,
    /// A trigger is due (a new note).
    pub trig: bool,
    pub macros: [f32; 8],
}

fn rising(prev: &mut f32, now: f32) -> bool {
    let r = *prev < HIGH && now >= HIGH;
    *prev = now;
    r
}

#[inline]
fn blep(t: f32, dt: f32) -> f32 {
    if t < dt {
        let t = t / dt;
        t + t - t * t - 1.0
    } else if t > 1.0 - dt {
        let t = (t - 1.0) / dt;
        t * t + t + t + 1.0
    } else {
        0.0
    }
}

const SCALE_STEPS: [&[i32]; 8] = [
    &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
    &[0, 2, 4, 5, 7, 9, 11],
    &[0, 2, 3, 5, 7, 8, 10],
    &[0, 2, 3, 5, 7, 9, 10],
    &[0, 2, 4, 7, 9],
    &[0, 3, 5, 7, 10],
    &[0, 3, 5, 6, 7, 10],
    &[0, 2, 4, 6, 8, 10],
];

/// The state of one module in one voice.
pub enum State {
    None,
    Midi { pitch: f32, trig: u32 },
    Clock { free_beat: f64 },
    Vco { phase: f32, sync: f32 },
    Lfo { phase: f32, reset: f32, held: f32, rng: Rng },
    Noise { rng: Rng, pink: [f32; 7], drift: f32, target: f32, phase: f32 },
    Vcf { svf: Svf },
    Ladder { s: [f32; 4] },
    Adsr { stage: u8, level: f32, gate: f32, retrig: f32, eoc: u32 },
    Ad { stage: u8, t: f32, level: f32, prev: f32, trig: f32 },
    SampleHold { held: f32, out: f32, trig: f32, rng: Rng },
    Slew { out: f32 },
    Quantizer { note: i32, trig: u32 },
    Sequencer { step: Option<usize>, clock: f32, reset: f32, since: u32, period: u32, gate: u32, trig: u32 },
    Delay { line: DelayLine, lp: f32 },
    Reverb(Box<Plate>, Vec<f32>),
    Divider { clock: f32, reset: f32, count: u32, open: bool },
    Chance { trig: f32, to_a: bool, rng: Rng },
    MacroVco(Box<MacroVcoState>),
    ShapeVco(Box<ShapeVcoState>),
    Resonator(Box<ResonatorState>),
    Texture(Box<TextureState>),
    BassDrum { d: Box<AnalogBassDrum>, trig: f32 },
    SnareDrum { d: Box<AnalogSnareDrum>, trig: f32 },
    HiHat { d: Box<Hihat>, trig: f32 },
    Function { stage: u8, pos: f32, trig: f32, eoc: u32 },
    Random(Box<RandomState>),
    Switch { step: usize, clock: f32, reset: f32 },
    Follower { env: f32 },
    Burst { left: u32, timer: f32, trig: f32, pulse: u32, eoc: u32 },
    Euclid { step: Option<usize>, clock: f32, reset: f32 },
    ShiftReg { reg: u16, clock: f32, pulse: u32, rng: Rng },
    Pulse { prev: f32, delay: u32, gate: u32 },
    Pluck { line: DelayLine, last: f32, lp: f32, trig: f32, excite: u32, rng: Rng },
    Formant { f: [Svf; 3] },
    TrigSeq { step: Option<usize>, clock: f32, reset: f32, trig: u32 },
    Crusher { held: f32, phase: f32 },
}

pub struct MacroVcoState {
    voice: PlaitsVoice<'static>,
    out: [f32; PLAITS_BLOCK],
    aux: [f32; PLAITS_BLOCK],
    pos: usize,
}

pub struct ShapeVcoState {
    osc: krabton_braids::MacroOscillator,
    out: [f32; BRAIDS_OUT],
    pos: usize,
    fir: [f32; FIR],
    hist: [f32; FIR],
    hist_pos: usize,
    strike: f32,
    rate_offset: i32,
}

pub struct ResonatorState {
    rings: krabton_rings::Rings,
    input: [f32; ADAPT],
    l: [f32; ADAPT],
    r: [f32; ADAPT],
    pos: usize,
    strum: f32,
    pending: bool,
}

pub struct TextureState {
    clouds: krabton_clouds::Clouds,
    in_l: [f32; ADAPT],
    in_r: [f32; ADAPT],
    l: [f32; ADAPT],
    r: [f32; ADAPT],
    pos: usize,
}

pub struct RandomState {
    rng: Rng,
    phase: f32,
    clock: f32,
    /// Remembered draws (deja vu): gate coin, X1, X2.
    history: [[f32; 3]; 16],
    idx: usize,
    gate_on: bool,
    t1: bool,
    period: u32,
    since: u32,
    x: [f32; 2],
    target: [f32; 2],
}

/// Formant frequencies of A, E, I, O, U.
const VOWELS: [[f32; 3]; 5] = [[800.0, 1150.0, 2900.0], [350.0, 2000.0, 2800.0], [270.0, 2140.0, 2950.0], [450.0, 800.0, 2830.0], [325.0, 700.0, 2700.0]];

impl State {
    pub fn new(kind: ModuleKind, sr: f32, seed: u32) -> Self {
        use ModuleKind::*;
        match kind {
            Midi => State::Midi { pitch: 0.0, trig: 0 },
            Clock => State::Clock { free_beat: 0.0 },
            Vco => State::Vco { phase: 0.0, sync: 0.0 },
            Lfo => State::Lfo { phase: 0.0, reset: 0.0, held: 0.0, rng: Rng::new(seed ^ 0x1F0) },
            Noise => State::Noise { rng: Rng::new(seed ^ 0x401), pink: [0.0; 7], drift: 0.0, target: 0.0, phase: 0.0 },
            Vcf => State::Vcf { svf: Svf::new() },
            Ladder => State::Ladder { s: [0.0; 4] },
            Adsr => State::Adsr { stage: 0, level: 0.0, gate: 0.0, retrig: 0.0, eoc: 0 },
            Ad => State::Ad { stage: 0, t: 0.0, level: 0.0, prev: 0.0, trig: 0.0 },
            SampleHold => State::SampleHold { held: 0.0, out: 0.0, trig: 0.0, rng: Rng::new(seed ^ 0x5A4) },
            Slew => State::Slew { out: 0.0 },
            Quantizer => State::Quantizer { note: i32::MIN, trig: 0 },
            Sequencer => State::Sequencer { step: None, clock: 0.0, reset: 0.0, since: 0, period: (sr * 0.125) as u32, gate: 0, trig: 0 },
            Delay => State::Delay { line: DelayLine::new((sr * 2.1) as usize), lp: 0.0 },
            Reverb => State::Reverb(Box::new(Plate::new(sr)), crate::params::defaults(plate::SPECS)),
            Divider => State::Divider { clock: 0.0, reset: 0.0, count: 0, open: false },
            Chance => State::Chance { trig: 0.0, to_a: true, rng: Rng::new(seed ^ 0xC4A) },
            MacroVco => {
                let mut voice = PlaitsVoice::new(PLAITS_BLOCK, sr);
                voice.init();
                State::MacroVco(Box::new(MacroVcoState { voice, out: [0.0; PLAITS_BLOCK], aux: [0.0; PLAITS_BLOCK], pos: PLAITS_BLOCK }))
            }
            ShapeVco => State::ShapeVco(Box::new(ShapeVcoState {
                osc: krabton_braids::MacroOscillator::new(),
                out: [0.0; BRAIDS_OUT],
                pos: BRAIDS_OUT,
                fir: crate::dsp::braids_synth::halfband(),
                hist: [0.0; FIR],
                hist_pos: 0,
                strike: 0.0,
                rate_offset: (12.0 * (krabton_braids::SAMPLE_RATE / (2.0 * sr)).log2() * 128.0).round() as i32,
            })),
            Resonator => State::Resonator(Box::new(ResonatorState { rings: krabton_rings::Rings::new(sr), input: [0.0; ADAPT], l: [0.0; ADAPT], r: [0.0; ADAPT], pos: 0, strum: 0.0, pending: false })),
            Texture => State::Texture(Box::new(TextureState { clouds: krabton_clouds::Clouds::new(sr), in_l: [0.0; ADAPT], in_r: [0.0; ADAPT], l: [0.0; ADAPT], r: [0.0; ADAPT], pos: 0 })),
            BassDrum => {
                let mut d = Box::new(AnalogBassDrum::new());
                d.init(sr);
                State::BassDrum { d, trig: 0.0 }
            }
            SnareDrum => {
                let mut d = Box::new(AnalogSnareDrum::new());
                d.init(sr);
                State::SnareDrum { d, trig: 0.0 }
            }
            HiHat => {
                let mut d = Box::new(Hihat::new());
                d.init(sr);
                State::HiHat { d, trig: 0.0 }
            }
            Function => State::Function { stage: 0, pos: 0.0, trig: 0.0, eoc: 0 },
            RandomGen => State::Random(Box::new(RandomState {
                rng: Rng::new(seed ^ 0xA4B1),
                phase: 0.0,
                clock: 0.0,
                history: [[0.5; 3]; 16],
                idx: 0,
                gate_on: false,
                t1: true,
                period: (sr * 0.25) as u32,
                since: 0,
                x: [0.0; 2],
                target: [0.0; 2],
            })),
            Switch => State::Switch { step: 0, clock: 0.0, reset: 0.0 },
            Follower => State::Follower { env: 0.0 },
            Burst => State::Burst { left: 0, timer: 0.0, trig: 0.0, pulse: 0, eoc: 0 },
            Euclid => State::Euclid { step: None, clock: 0.0, reset: 0.0 },
            ShiftReg => State::ShiftReg { reg: 0b1011_0010_0110_1001, clock: 0.0, pulse: 0, rng: Rng::new(seed ^ 0x5E6) },
            Pulse => State::Pulse { prev: 0.0, delay: 0, gate: 0 },
            Pluck => State::Pluck { line: DelayLine::new((sr / 16.0) as usize + 8), last: 0.0, lp: 0.0, trig: 0.0, excite: 0, rng: Rng::new(seed ^ 0x9C1) },
            Formant => State::Formant { f: [Svf::new(), Svf::new(), Svf::new()] },
            TrigSeq => State::TrigSeq { step: None, clock: 0.0, reset: 0.0, trig: 0 },
            Crusher => State::Crusher { held: 0.0, phase: 1.0 },
            Out | Macros | Vca | Mixer | Atten | Folder | RingMod | QuadVca | Logic | MetaMod | Octave | StereoMixer => State::None,
        }
    }

    /// One sample: reads `inp` (volts; `connected` has a bit per patched
    /// input), writes `out`.
    #[allow(clippy::too_many_arguments)]
    pub fn tick(&mut self, kind: ModuleKind, p: &[f32], inp: &[f32], connected: u32, out: &mut [f32], c: &SampleCtx) {
        let sr = c.sr;
        let patched = |i: usize| connected & (1 << i) != 0;
        let trig_len = (TRIG_S * sr) as u32;
        match (kind, self) {
            (ModuleKind::Midi, State::Midi { pitch, trig }) => {
                let target = c.pitch + p[1].round();
                let glide = p[0];
                if glide <= 0.0015 {
                    *pitch = target;
                } else {
                    *pitch += (target - *pitch) * (1.0 - (-1.0 / (glide * sr)).exp());
                }
                if c.trig {
                    *trig = trig_len;
                }
                out[0] = *pitch;
                out[1] = if c.gate { 10.0 } else { 0.0 };
                out[2] = c.vel * 10.0;
                out[3] = if *trig > 0 { 10.0 } else { 0.0 };
                *trig = trig.saturating_sub(1);
            }
            (ModuleKind::Macros, _) => {
                for (o, m) in out.iter_mut().zip(c.macros) {
                    *o = m.clamp(0.0, 1.0) * 10.0;
                }
            }
            (ModuleKind::Clock, State::Clock { free_beat }) => {
                const BEATS: [f64; 6] = [1.0, 0.5, 1.0 / 3.0, 0.25, 1.0 / 6.0, 0.125];
                let len = BEATS[(p[0].round().max(0.0) as usize).min(5)];
                *free_beat += c.bpm.max(1.0) as f64 / 60.0 / sr as f64;
                let beat = if c.playing { c.beat } else { *free_beat };
                let step = (beat / len).floor();
                let mut f = (beat / len - step) as f32;
                // Swing: odd steps come later.
                if step as i64 % 2 == 1 {
                    f -= p[2];
                }
                let gate = (0.0..p[1]).contains(&f);
                let s = step as i64;
                out[0] = if gate { 10.0 } else { 0.0 };
                out[1] = if s.rem_euclid(2) == 0 && f < 1.0 && f >= 0.0 { 10.0 } else { 0.0 };
                out[2] = if s.rem_euclid(4) < 2 { 10.0 } else { 0.0 };
                let bar = beat.rem_euclid(4.0);
                out[3] = if bar < (TRIG_S * c.bpm / 60.0) as f64 * 2.0 { 10.0 } else { 0.0 };
            }
            (ModuleKind::Vco, State::Vco { phase, sync }) => {
                let v = p[0].round() + (p[1].round() + p[2] * 0.01) / 12.0 + if patched(0) { inp[0] } else { 0.0 };
                let f = C4 * 2f32.powf(v.clamp(-10.0, 10.0));
                // Linear (through-zero) FM: 5 V at full FM swings by the frequency.
                let f = f * (1.0 + p[4] * inp[1] / 5.0);
                let dt = (f / sr).clamp(-0.45, 0.45);
                if rising(sync, inp[3]) {
                    *phase = 0.0;
                }
                *phase += dt;
                *phase -= phase.floor();
                let ph = *phase;
                let adt = dt.abs().max(1e-6);
                let pw = (p[3] + p[5] * inp[2] / 10.0).clamp(0.05, 0.95);
                out[0] = 5.0 * (TAU * ph).sin();
                out[1] = 5.0 * (1.0 - 4.0 * (ph - 0.5).abs());
                out[2] = 5.0 * (2.0 * ph - 1.0 - blep(ph, adt));
                let mut sq = if ph < pw { 1.0 } else { -1.0 };
                sq += blep(ph, adt);
                let ph2 = (ph - pw + 1.0).fract();
                sq -= blep(ph2, adt);
                out[3] = 5.0 * sq;
            }
            (ModuleKind::Lfo, State::Lfo { phase, reset, held, rng }) => {
                let f = p[0] * 2f32.powf(inp[0].clamp(-8.0, 8.0));
                if rising(reset, inp[1]) {
                    *phase = 0.0;
                }
                *phase += f / sr;
                if *phase >= 1.0 {
                    *phase -= phase.floor();
                    *held = rng.bipolar();
                }
                let ph = *phase;
                let off = p[1];
                out[0] = 5.0 * (TAU * ph).sin() + off;
                out[1] = 5.0 * (1.0 - 4.0 * (ph - 0.5).abs()) + off;
                out[2] = 5.0 * (2.0 * ph - 1.0) + off;
                out[3] = if ph < 0.5 { 5.0 } else { -5.0 } + off;
                out[4] = 5.0 * *held + off;
            }
            (ModuleKind::Noise, State::Noise { rng, pink, drift, target, phase }) => {
                let w = rng.bipolar();
                // Paul Kellet's pink filter.
                pink[0] = 0.99886 * pink[0] + w * 0.0555179;
                pink[1] = 0.99332 * pink[1] + w * 0.0750759;
                pink[2] = 0.96900 * pink[2] + w * 0.1538520;
                pink[3] = 0.86650 * pink[3] + w * 0.3104856;
                pink[4] = 0.55000 * pink[4] + w * 0.5329522;
                pink[5] = -0.7616 * pink[5] - w * 0.0168980;
                let pk = pink.iter().sum::<f32>() + w * 0.5362;
                pink[6] = w * 0.115926;
                *phase += p[0] / sr;
                if *phase >= 1.0 {
                    *phase -= 1.0;
                    *target = rng.bipolar() * 5.0;
                }
                *drift += (*target - *drift) * (1.0 - (-TAU * p[0] / sr).exp());
                out[0] = w * 5.0;
                out[1] = pk * 0.55;
                out[2] = *drift;
            }
            (ModuleKind::Vcf, State::Vcf { svf }) => {
                let fc = p[0] * 2f32.powf((inp[1] * p[2]).clamp(-10.0, 10.0));
                let res = (p[1] + inp[2] / 10.0).clamp(0.0, 1.0);
                svf.set(fc.clamp(10.0, 0.45 * sr), res * 0.98, sr);
                let drive = 1.0 + 4.0 * p[3];
                let x = fast_tanh(inp[0] / 5.0 * drive) / drive.sqrt();
                let o = svf.tick(x);
                out[0] = o.low * 5.0;
                out[1] = o.band * 5.0;
                out[2] = o.high * 5.0;
            }
            (ModuleKind::Ladder, State::Ladder { s }) => {
                let fc = (p[0] * 2f32.powf((inp[1] * p[2]).clamp(-10.0, 10.0))).clamp(10.0, 0.42 * sr);
                let g = 1.0 - (-TAU * fc / sr).exp();
                let k = 4.0 * (p[1] + inp[2] / 10.0).clamp(0.0, 1.05);
                let drive = 1.0 + 5.0 * p[3];
                let x = fast_tanh((inp[0] / 5.0 * drive - k * s[3]).clamp(-8.0, 8.0));
                s[0] += g * (x - s[0]);
                s[1] += g * (s[0] - s[1]);
                s[2] += g * (s[1] - s[2]);
                s[3] += g * (s[2] - s[3]);
                if !s[3].is_finite() {
                    *s = [0.0; 4];
                }
                out[0] = s[3] * 5.0 * (1.0 + 0.5 * k) / drive.sqrt();
            }
            (ModuleKind::Vca, _) => {
                let mut g = (p[0] + if patched(1) { inp[1] / 10.0 } else { 0.0 }).clamp(0.0, 1.5);
                if p[1] >= 0.5 {
                    g *= g;
                }
                out[0] = inp[0] * g;
            }
            (ModuleKind::Adsr, State::Adsr { stage, level, gate, retrig, eoc }) => {
                let g = inp[0] >= HIGH;
                let started = rising(gate, inp[0]) | rising(retrig, inp[1]);
                if started {
                    *stage = 1;
                }
                if !g && *stage != 0 && *stage != 4 {
                    *stage = 4;
                }
                let coef = |t: f32| 1.0 - (-1.0 / (t.max(0.0005) * sr)).exp();
                match *stage {
                    1 => {
                        // Charging towards a level past the top, as analog ones do.
                        *level += (1.25 - *level) * coef(p[0] * 0.6);
                        if *level >= 1.0 {
                            *level = 1.0;
                            *stage = 2;
                        }
                    }
                    2 | 3 => *level += (p[2] - *level) * coef(p[1] * 0.25),
                    4 => {
                        *level += (0.0 - *level) * coef(p[3] * 0.25);
                        if *level < 1e-4 {
                            *level = 0.0;
                            *stage = 0;
                            *eoc = trig_len;
                        }
                    }
                    _ => {}
                }
                out[0] = *level * 10.0;
                out[1] = 10.0 - *level * 10.0;
                out[2] = if *eoc > 0 { 10.0 } else { 0.0 };
                *eoc = eoc.saturating_sub(1);
            }
            (ModuleKind::Ad, State::Ad { stage, t, level, prev, trig }) => {
                if rising(trig, inp[0]) {
                    *stage = 1;
                    *t = 0.0;
                    *prev = *level;
                }
                match *stage {
                    1 => {
                        *t += 1.0 / (p[0].max(0.0001) * sr);
                        *level = *prev + (1.0 - *prev) * t.min(1.0);
                        if *t >= 1.0 {
                            *stage = 2;
                            *t = 0.0;
                        }
                    }
                    2 => {
                        *t += 1.0 / (p[1].max(0.001) * sr);
                        let u = t.min(1.0);
                        // Curve: from a straight line to a fast exponential fall.
                        let lin = 1.0 - u;
                        let exp = (-6.0 * u).exp() * (1.0 - u);
                        *level = lin + (exp - lin) * p[2];
                        if *t >= 1.0 {
                            *stage = 0;
                            *level = 0.0;
                        }
                    }
                    _ => {}
                }
                out[0] = *level * 10.0;
            }
            (ModuleKind::Mixer, _) => {
                let s = (0..4).map(|i| inp[i] * p[i]).sum::<f32>();
                out[0] = s * p[4];
            }
            (ModuleKind::Atten, _) => {
                let in2 = if patched(1) { inp[1] } else { inp[0] };
                out[0] = inp[0] * p[0] + p[1];
                out[1] = in2 * p[2] + p[3];
            }
            (ModuleKind::SampleHold, State::SampleHold { held, out: o, trig, rng }) => {
                if rising(trig, inp[1]) {
                    *held = if patched(0) { inp[0] } else { rng.bipolar() * 5.0 };
                }
                *o += (*held - *o) * (1.0 - (-1.0 / (p[0].max(0.0001) * sr)).exp());
                out[0] = *o;
            }
            (ModuleKind::Slew, State::Slew { out: o }) => {
                let t = if inp[0] > *o { p[0] } else { p[1] };
                *o += (inp[0] - *o) * (1.0 - (-1.0 / (t.max(0.0001) * sr)).exp());
                out[0] = *o;
            }
            (ModuleKind::Quantizer, State::Quantizer { note, trig }) => {
                let steps = SCALE_STEPS[(p[0].round().max(0.0) as usize).min(SCALE_STEPS.len() - 1)];
                let root = p[1].round() as i32;
                let semis = (inp[0] * 12.0).round() as i32 - root;
                let oct = semis.div_euclid(12);
                let within = semis.rem_euclid(12);
                // The nearest scale note (ties go down).
                let degree = steps.iter().copied().min_by_key(|s| ((s - within).abs(), *s)).unwrap_or(0);
                let n = oct * 12 + degree + root + p[2].round() as i32;
                if n != *note {
                    *note = n;
                    *trig = trig_len;
                }
                out[0] = n as f32 / 12.0;
                out[1] = if *trig > 0 { 10.0 } else { 0.0 };
                *trig = trig.saturating_sub(1);
            }
            (ModuleKind::Sequencer, State::Sequencer { step, clock, reset, since, period, gate, trig }) => {
                let steps = (p[8].round() as usize).clamp(1, 8);
                if rising(reset, inp[1]) {
                    *step = None;
                }
                *since = since.saturating_add(1);
                if rising(clock, inp[0]) {
                    *period = (*since).clamp(16, (sr * 8.0) as u32);
                    *since = 0;
                    *step = Some(step.map(|s| (s + 1) % steps).unwrap_or(0));
                    *gate = ((*period as f32) * p[9]) as u32;
                    *trig = trig_len;
                }
                let s = step.unwrap_or(0).min(steps - 1);
                out[0] = p[s];
                out[1] = if *gate > 0 { 10.0 } else { 0.0 };
                out[2] = if *trig > 0 { 10.0 } else { 0.0 };
                *gate = gate.saturating_sub(1);
                *trig = trig.saturating_sub(1);
            }
            (ModuleKind::Folder, _) => {
                let amount = 1.0 + 8.0 * (p[0] + p[2] * inp[1] / 10.0).clamp(0.0, 1.5);
                let x = inp[0] / 5.0 * amount + p[1];
                out[0] = 5.0 * (x * PI * 0.5).sin();
            }
            (ModuleKind::RingMod, _) => {
                let rm = inp[0] * inp[1] / 5.0;
                out[0] = inp[0] * (1.0 - p[0]) + rm * p[0];
            }
            (ModuleKind::Delay, State::Delay { line, lp }) => {
                let t = (p[0] * 0.001 * 2f32.powf(inp[1].clamp(-4.0, 4.0))).clamp(0.0005, 2.0);
                let y = line.read(t * sr);
                *lp += (y - *lp) * (1.0 - (-TAU * p[2] / sr).exp());
                let w = inp[0] + *lp * p[1];
                line.write(fast_tanh(w / 8.0) * 8.0);
                out[0] = inp[0] * (1.0 - p[3]) + *lp * p[3];
            }
            (ModuleKind::Reverb, State::Reverb(plate, params)) => {
                params[plate::SIZE] = p[0];
                params[plate::DECAY] = p[1];
                params[plate::MIX] = p[2];
                let ctx = FxCtx { sr, bpm: c.bpm, beat: c.beat, playing: c.playing };
                let (mut l, mut r) = ([inp[0]], [inp[0]]);
                plate.process(params, &mut l, &mut r, &ctx);
                out[0] = l[0];
                out[1] = r[0];
            }
            (ModuleKind::Divider, State::Divider { clock, reset, count, open }) => {
                if rising(reset, inp[1]) {
                    *count = 0;
                }
                if rising(clock, inp[0]) {
                    *open = *count % (p[0].round().max(1.0) as u32) == 0;
                    *count += 1;
                }
                out[0] = if *open && inp[0] >= HIGH { 10.0 } else { 0.0 };
            }
            (ModuleKind::Chance, State::Chance { trig, to_a, rng }) => {
                if rising(trig, inp[0]) {
                    *to_a = rng.unipolar() < p[0];
                }
                let high = if inp[0] >= HIGH { inp[0] } else { 0.0 };
                out[0] = if *to_a { high } else { 0.0 };
                out[1] = if *to_a { 0.0 } else { high };
            }
            (ModuleKind::MacroVco, State::MacroVco(s)) => {
                if s.pos >= PLAITS_BLOCK {
                    let patch = PlaitsPatch {
                        note: 60.0 + p[1].round() * 12.0,
                        harmonics: p[2],
                        timbre: p[3],
                        morph: p[4],
                        frequency_modulation_amount: 0.0,
                        timbre_modulation_amount: if patched(3) { 1.0 } else { 0.0 },
                        morph_modulation_amount: if patched(4) { 1.0 } else { 0.0 },
                        engine: (p[0].round().max(0.0) as usize).min(crate::dsp::macro_osc::MODELS.len() - 1),
                        decay: p[5],
                        lpg_colour: p[6],
                    };
                    let m = Modulations {
                        note: (inp[0] * 12.0).clamp(-96.0, 96.0),
                        harmonics: (inp[2] / 5.0).clamp(-1.0, 1.0),
                        timbre: (inp[3] / 5.0).clamp(-1.0, 1.0),
                        morph: (inp[4] / 5.0).clamp(-1.0, 1.0),
                        trigger: if inp[1] >= HIGH { 1.0 } else { 0.0 },
                        trigger_patched: patched(1),
                        level: (inp[5] / 10.0).clamp(0.0, 1.0),
                        level_patched: patched(5),
                        timbre_patched: patched(3),
                        morph_patched: patched(4),
                        ..Default::default()
                    };
                    s.voice.render(&patch, &m, &mut s.out, &mut s.aux);
                    s.pos = 0;
                }
                let (a, b) = (s.out[s.pos], s.aux[s.pos]);
                out[0] = if a.is_finite() { a * 5.0 } else { 0.0 };
                out[1] = if b.is_finite() { b * 5.0 } else { 0.0 };
                s.pos += 1;
            }
            (ModuleKind::ShapeVco, State::ShapeVco(s)) => {
                if rising(&mut s.strike, inp[3]) {
                    s.osc.strike();
                }
                if s.pos >= BRAIDS_OUT {
                    let semis = 60.0 + p[1].round() * 12.0 + inp[0] * 12.0;
                    let pitch = ((semis * 128.0) as i32 + s.rate_offset).clamp(0, 16383) as i16;
                    let knob = |v: f32, cv: f32| ((v + cv / 10.0).clamp(0.0, 1.0) * 32767.0) as i16;
                    s.osc.set_shape((p[0].round().max(0.0) as usize).min(crate::dsp::braids_synth::SHAPE_LIST.len() - 1));
                    s.osc.set_pitch(pitch);
                    s.osc.set_parameters(knob(p[2], inp[1]), knob(p[3], inp[2]));
                    let mut raw = [0i16; krabton_braids::BLOCK_SIZE];
                    s.osc.render(&[0u8; krabton_braids::BLOCK_SIZE], &mut raw);
                    for j in 0..BRAIDS_OUT {
                        for v in &raw[j * 2..j * 2 + 2] {
                            s.hist[s.hist_pos] = *v as f32 / 32768.0;
                            s.hist_pos = (s.hist_pos + 1) % FIR;
                        }
                        let mut acc = 0.0;
                        for (t, c) in s.fir.iter().enumerate() {
                            acc += c * s.hist[(s.hist_pos + t) % FIR];
                        }
                        s.out[j] = acc;
                    }
                    s.pos = 0;
                }
                out[0] = s.out[s.pos] * 5.0;
                s.pos += 1;
            }
            (ModuleKind::Resonator, State::Resonator(s)) => {
                if rising(&mut s.strum, inp[2]) {
                    s.pending = true;
                }
                s.input[s.pos] = inp[0] / 5.0;
                out[0] = s.l[s.pos] * 5.0;
                out[1] = s.r[s.pos] * 5.0;
                s.pos += 1;
                if s.pos >= ADAPT {
                    s.pos = 0;
                    if std::mem::take(&mut s.pending) {
                        s.rings.note_on(p[1].round() + inp[1] * 12.0, 0.8);
                    }
                    let model = match p[0].round() as i32 {
                        1 => krabton_rings::Model::SympatheticString,
                        2 => krabton_rings::Model::InharmonicString,
                        3 => krabton_rings::Model::FmVoice,
                        4 => krabton_rings::Model::SympatheticStringQuantized,
                        5 => krabton_rings::Model::StringAndReverb,
                        _ => krabton_rings::Model::Modal,
                    };
                    let knob = |v: f32, cv: f32| (v + cv / 10.0).clamp(0.0, 1.0);
                    let prm = krabton_rings::Params {
                        model,
                        polyphony: 1,
                        easter_egg: false,
                        egg_fx: krabton_rings::EggFx::Formant,
                        structure: knob(p[2], inp[3]),
                        brightness: knob(p[3], inp[4]),
                        damping: knob(p[4], inp[5]),
                        position: knob(p[5], inp[6]),
                        chord: None,
                    };
                    let input = if patched(0) { Some(&s.input[..]) } else { None };
                    s.rings.process(&prm, input, &mut s.l, &mut s.r);
                }
            }
            (ModuleKind::Texture, State::Texture(s)) => {
                s.in_l[s.pos] = inp[0] / 5.0;
                s.in_r[s.pos] = if patched(1) { inp[1] } else { inp[0] } / 5.0;
                out[0] = s.l[s.pos] * 5.0;
                out[1] = s.r[s.pos] * 5.0;
                s.pos += 1;
                if s.pos >= ADAPT {
                    s.pos = 0;
                    let mode = match p[0].round() as i32 {
                        1 => krabton_clouds::PlaybackMode::Stretch,
                        2 => krabton_clouds::PlaybackMode::LoopingDelay,
                        3 => krabton_clouds::PlaybackMode::Spectral,
                        _ => krabton_clouds::PlaybackMode::Granular,
                    };
                    let knob = |v: f32, cv: f32| (v + cv / 10.0).clamp(0.0, 1.0);
                    let prm = krabton_clouds::Params {
                        mode,
                        quality: 0,
                        position: knob(p[1], inp[4]),
                        size: knob(p[2], inp[5]),
                        pitch: (p[3].round() + inp[6] * 12.0).clamp(-48.0, 48.0),
                        density: knob(p[4], inp[7]),
                        texture: p[5],
                        dry_wet: p[6],
                        stereo_spread: p[7],
                        feedback: p[8],
                        reverb: p[9],
                        freeze: inp[2] >= HIGH,
                        trigger: inp[3] >= HIGH,
                    };
                    s.l = s.in_l;
                    s.r = s.in_r;
                    s.clouds.process(&prm, &mut s.l, &mut s.r);
                }
            }
            (ModuleKind::BassDrum, State::BassDrum { d, trig }) => {
                let hit = rising(trig, inp[0]);
                let accent = if patched(1) { (inp[1] / 10.0).clamp(0.0, 1.0) } else { 0.8 };
                let f0 = (p[0] * 2f32.powf(inp[2].clamp(-4.0, 4.0)) / sr).clamp(1e-5, 0.4);
                let a = p[3];
                let mut o = [0.0f32];
                d.render(false, hit, accent, f0, p[1], p[2], (a * 4.0).min(1.0), (a * 4.0 - 1.0).clamp(0.0, 1.0), &mut o);
                out[0] = o[0] * 5.0;
            }
            (ModuleKind::SnareDrum, State::SnareDrum { d, trig }) => {
                let hit = rising(trig, inp[0]);
                let accent = if patched(1) { (inp[1] / 10.0).clamp(0.0, 1.0) } else { 0.8 };
                let f0 = (p[0] * 2f32.powf(inp[2].clamp(-4.0, 4.0)) / sr).clamp(1e-5, 0.4);
                let mut o = [0.0f32];
                d.render(false, hit, accent, f0, p[1], p[2], p[3], &mut o);
                out[0] = o[0] * 5.0;
            }
            (ModuleKind::HiHat, State::HiHat { d, trig }) => {
                let hit = rising(trig, inp[0]);
                let accent = if patched(1) { (inp[1] / 10.0).clamp(0.0, 1.0) } else { 0.8 };
                let f0 = (p[0] * 2f32.powf(inp[2].clamp(-4.0, 4.0)) / sr).clamp(1e-5, 0.4);
                let (mut t1, mut t2, mut o) = ([0.0f32], [0.0f32], [0.0f32]);
                d.render(false, hit, accent, f0, p[1], p[2], p[3], &mut t1, &mut t2, &mut o, NoiseType::Square, VcaType::Swing, true, false);
                out[0] = o[0] * 5.0;
            }
            (ModuleKind::Function, State::Function { stage, pos, trig, eoc }) => {
                let cycle = p[3] >= 0.5 || inp[3] >= HIGH;
                if rising(trig, inp[0]) || (cycle && *stage == 0) {
                    *stage = 1;
                }
                let rise = (p[0] * 2f32.powf(-inp[1].clamp(-8.0, 8.0))).max(0.0002);
                let fall = (p[1] * 2f32.powf(-inp[2].clamp(-8.0, 8.0))).max(0.0002);
                match *stage {
                    1 => {
                        *pos += 1.0 / (rise * sr);
                        if *pos >= 1.0 {
                            *pos = 1.0;
                            *stage = 2;
                        }
                    }
                    2 => {
                        *pos -= 1.0 / (fall * sr);
                        if *pos <= 0.0 {
                            *pos = 0.0;
                            *stage = 0;
                            *eoc = trig_len;
                        }
                    }
                    _ => {}
                }
                // Shape: below 0 logarithmic, above 0 exponential.
                let k = 1.0 + 4.0 * p[2].abs();
                let y = if p[2] >= 0.0 { pos.powf(k) } else { 1.0 - (1.0 - *pos).powf(k) };
                out[0] = y * 10.0;
                out[1] = if *stage == 2 { 10.0 } else { 0.0 };
                out[2] = if *eoc > 0 { 10.0 } else { 0.0 };
                *eoc = eoc.saturating_sub(1);
            }
            (ModuleKind::RandomGen, State::Random(s)) => {
                // A tick: from the clock input, or the own clock with jitter.
                let tick = if patched(0) {
                    rising(&mut s.clock, inp[0])
                } else {
                    s.phase += p[0] / sr;
                    if s.phase >= 1.0 {
                        s.phase -= 1.0;
                        s.phase -= p[2] * 0.5 * s.rng.unipolar();
                        true
                    } else {
                        false
                    }
                };
                s.since = s.since.saturating_add(1);
                if tick {
                    s.period = s.since.clamp(32, (sr * 8.0) as u32);
                    s.since = 0;
                    let len = (p[6].round() as usize).clamp(1, 16);
                    s.idx = (s.idx + 1) % len;
                    // Deja vu: play back a remembered draw, or make a new one.
                    if s.rng.unipolar() >= p[5] {
                        s.history[s.idx] = [s.rng.unipolar(), s.rng.unipolar(), s.rng.unipolar()];
                    }
                    let [coin, a, b] = s.history[s.idx];
                    s.t1 = coin < p[1];
                    s.gate_on = true;
                    for (k, r) in [a, b].into_iter().enumerate() {
                        let mut v = (2.0 * r - 1.0) * p[3];
                        if p[4] > 0.5 {
                            v = (v * 12.0).round() / 12.0;
                        }
                        s.target[k] = v;
                    }
                }
                if s.gate_on && s.since > s.period / 2 {
                    s.gate_on = false;
                }
                // Steps below half glide between values.
                let glide = (0.5 - p[4]).max(0.0) * 2.0 * s.period as f32;
                let c = if glide > 1.0 { 1.0 - (-1.0 / glide).exp() } else { 1.0 };
                for k in 0..2 {
                    s.x[k] += (s.target[k] - s.x[k]) * c;
                }
                out[0] = if s.gate_on && s.t1 { 10.0 } else { 0.0 };
                out[1] = if s.gate_on && !s.t1 { 10.0 } else { 0.0 };
                out[2] = s.x[0];
                out[3] = s.x[1];
            }
            (ModuleKind::QuadVca, _) => {
                let mut mix = 0.0;
                for k in 0..4 {
                    let mut g = (p[k] + if patched(4 + k) { inp[4 + k] / 10.0 } else { 0.0 }).clamp(0.0, 1.5);
                    g = g.powf(1.0 + 2.0 * p[4]);
                    out[k] = inp[k] * g;
                    mix += out[k];
                }
                out[4] = mix;
            }
            (ModuleKind::Logic, _) => {
                out[0] = inp[0].max(inp[1]);
                out[1] = inp[0].min(inp[1]);
                out[2] = inp[0].abs();
                out[3] = if inp[0] > inp[1] { 10.0 } else { 0.0 };
            }
            (ModuleKind::Switch, State::Switch { step, clock, reset }) => {
                let steps = (p[0].round() as usize).clamp(1, 4);
                if rising(reset, inp[1]) {
                    *step = 0;
                }
                if rising(clock, inp[0]) {
                    *step = (*step + 1) % steps;
                }
                out[0] = inp[2 + (*step).min(3)];
            }
            (ModuleKind::MetaMod, _) => {
                let a = (inp[0] * p[2] / 5.0).clamp(-2.0, 2.0);
                let b = (inp[1] * p[3] / 5.0).clamp(-2.0, 2.0);
                let t = (p[1] + inp[3] / 10.0).clamp(0.0, 1.0);
                let algo = (p[0] + inp[2] / 10.0).clamp(0.0, 1.0) * 5.0;
                let k = (algo.floor() as usize).min(4);
                let f = algo - k as f32;
                let relu = |x: f32| x.max(0.0);
                let run = |n: usize, t: f32| -> f32 {
                    match n {
                        0 => a * (1.0 - t) + b * t,
                        1 => ((a + b) * (1.0 + 4.0 * t) * PI * 0.5).sin(),
                        // A diode ring modulator.
                        2 => fast_tanh((relu(a + b).powi(2) - relu(a - b).powi(2)) * (1.0 + 3.0 * t)),
                        3 => (a * b * (1.0 + 3.0 * t)).clamp(-1.0, 1.0),
                        4 => {
                            let ia = (a.clamp(-1.0, 1.0) * 32767.0) as i16;
                            let ib = (b.clamp(-1.0, 1.0) * 32767.0) as i16;
                            let x = (ia ^ ib) as f32 / 32768.0;
                            x * t + a * b * (1.0 - t)
                        }
                        _ => {
                            let c = if a.abs() > b.abs() { a } else { b };
                            c * (1.0 - t) + if a > b { 1.0 } else { -1.0 } * t
                        }
                    }
                };
                out[0] = (run(k, t) * (1.0 - f) + run(k + 1, t) * f) * 5.0;
                out[1] = (run(k, 1.0 - t) * (1.0 - f) + run(k + 1, 1.0 - t) * f) * 5.0;
            }
            (ModuleKind::Follower, State::Follower { env }) => {
                let x = (inp[0].abs() / 5.0 * p[0] * 10.0).min(10.0);
                let c = if x > *env { 1.0 - (-1.0 / (p[1].max(0.0001) * sr)).exp() } else { 1.0 - (-1.0 / (p[2].max(0.001) * sr)).exp() };
                *env += (x - *env) * c;
                out[0] = *env;
                out[1] = if *env > p[3] { 10.0 } else { 0.0 };
            }
            (ModuleKind::Octave, _) => {
                out[0] = inp[0] + p[0].round() + p[1].round() / 12.0;
            }
            (ModuleKind::Burst, State::Burst { left, timer, trig, pulse, eoc }) => {
                if rising(trig, inp[0]) {
                    *left = p[0].round().clamp(1.0, 16.0) as u32;
                    *timer = 0.0;
                }
                if *left > 0 {
                    *timer -= 1.0;
                    if *timer <= 0.0 {
                        *pulse = trig_len.max(4);
                        *left -= 1;
                        *timer += sr / (p[1] * 2f32.powf(inp[1].clamp(-4.0, 4.0))).max(0.5);
                        if *left == 0 {
                            *eoc = trig_len;
                        }
                    }
                }
                out[0] = if *pulse > 0 { 10.0 } else { 0.0 };
                out[1] = if *eoc > 0 && *pulse == 0 { 10.0 } else { 0.0 };
                *pulse = pulse.saturating_sub(1);
                if *pulse == 0 {
                    *eoc = eoc.saturating_sub(1);
                }
            }
            (ModuleKind::Euclid, State::Euclid { step, clock, reset }) => {
                let steps = (p[0].round() as usize).clamp(1, 32);
                let pulses = (p[1].round() as usize).min(steps);
                let rot = p[2].round() as usize;
                if rising(reset, inp[1]) {
                    *step = None;
                }
                if rising(clock, inp[0]) {
                    *step = Some(step.map(|s| (s + 1) % steps).unwrap_or(0));
                }
                let s = step.unwrap_or(0) % steps;
                let hit = step.is_some() && ((s + rot) * pulses) % steps < pulses;
                let high = inp[0] >= HIGH;
                out[0] = if hit && high { 10.0 } else { 0.0 };
                out[1] = if !hit && high && step.is_some() { 10.0 } else { 0.0 };
            }
            (ModuleKind::ShiftReg, State::ShiftReg { reg, clock, pulse, rng }) => {
                let len = (p[1].round() as u32).clamp(2, 16);
                if rising(clock, inp[0]) {
                    let mut bit = (*reg >> (len - 1)) & 1;
                    if rng.unipolar() < (p[0] + inp[1] / 10.0).clamp(0.0, 1.0) {
                        bit ^= 1;
                    }
                    let mask = if len >= 16 { 0xFFFF } else { (1u16 << len) - 1 };
                    *reg = ((*reg << 1) | bit) & mask;
                    if *reg & 1 == 1 {
                        *pulse = trig_len;
                    }
                }
                out[0] = (*reg & 0xFF) as f32 / 255.0 * p[2];
                out[1] = if *reg & 1 == 1 && inp[0] >= HIGH { 10.0 } else { 0.0 };
                out[2] = if *pulse > 0 { 10.0 } else { 0.0 };
                *pulse = pulse.saturating_sub(1);
            }
            (ModuleKind::StereoMixer, _) => {
                let (mut l, mut r) = (0.0, 0.0);
                for k in 0..4 {
                    let (gl, gr) = pan_gains(p[4 + k]);
                    l += inp[k] * p[k] * gl;
                    r += inp[k] * p[k] * gr;
                }
                out[0] = l * p[8];
                out[1] = r * p[8];
            }
            (ModuleKind::Pulse, State::Pulse { prev, delay, gate }) => {
                if rising(prev, inp[0]) {
                    *delay = (p[1] * sr) as u32 + 1;
                }
                if *delay > 0 {
                    *delay -= 1;
                    if *delay == 0 {
                        *gate = (p[0] * sr).max(1.0) as u32;
                    }
                }
                out[0] = if *gate > 0 { 10.0 } else { 0.0 };
                *gate = gate.saturating_sub(1);
            }
            (ModuleKind::Pluck, State::Pluck { line, last, lp, trig, excite, rng }) => {
                let f = (C4 * 2f32.powf((inp[0] + p[3].round()).clamp(-6.0, 6.0))).clamp(20.0, 0.25 * sr);
                let d = (sr / f).min((line.len() - 4) as f32);
                if rising(trig, inp[1]) {
                    *excite = d as u32;
                }
                // The pluck: a burst of noise, darker with less brightness.
                let mut x = inp[2] / 5.0;
                if *excite > 0 {
                    *excite -= 1;
                    let n = rng.bipolar();
                    *lp += (n - *lp) * (0.05 + 0.95 * p[2]);
                    x += *lp;
                }
                let y = line.read(d);
                let g = 0.9 + 0.0999 * p[0];
                let damp = 0.5 + 0.5 * (1.0 - p[1]);
                let v = x + g * (y * damp + *last * (1.0 - damp));
                *last = y;
                line.write(if v.is_finite() { v.clamp(-4.0, 4.0) } else { 0.0 });
                out[0] = y * 5.0;
            }
            (ModuleKind::Formant, State::Formant { f }) => {
                let pos = ((p[0] + inp[1] / 10.0).clamp(0.0, 1.0)) * 4.0;
                let k = (pos.floor() as usize).min(3);
                let t = pos - k as f32;
                let shift = 2f32.powf((p[1].round() / 12.0 + inp[2]).clamp(-3.0, 3.0));
                let q = 2.0 + 18.0 * p[2];
                let x = inp[0] / 5.0;
                let mut y = 0.0;
                for (i, filt) in f.iter_mut().enumerate() {
                    let hz = (VOWELS[k][i] * (1.0 - t) + VOWELS[k + 1][i] * t) * shift;
                    filt.set_q(hz.clamp(40.0, 0.45 * sr), q, sr);
                    y += filt.tick(x).band * [1.0, 0.6, 0.35][i];
                }
                out[0] = (x * (1.0 - p[3]) + y * p[3] * 1.5) * 5.0;
            }
            (ModuleKind::TrigSeq, State::TrigSeq { step, clock, reset, trig }) => {
                if rising(reset, inp[1]) {
                    *step = None;
                }
                if rising(clock, inp[0]) {
                    let s = step.map(|s| (s + 1) % 16).unwrap_or(0);
                    *step = Some(s);
                    if p[s] >= 0.5 {
                        *trig = trig_len;
                    }
                }
                let on = step.map(|s| p[s] >= 0.5).unwrap_or(false);
                out[0] = if on && inp[0] >= HIGH { 10.0 } else { 0.0 };
                out[1] = if *trig > 0 { 10.0 } else { 0.0 };
                *trig = trig.saturating_sub(1);
            }
            (ModuleKind::Crusher, State::Crusher { held, phase }) => {
                *phase += (p[1] * 2f32.powf(inp[1].clamp(-6.0, 6.0))) / sr;
                if *phase >= 1.0 {
                    *phase -= phase.floor();
                    let steps = 2f32.powf(p[0].round().clamp(1.0, 16.0) - 1.0);
                    *held = (inp[0] / 5.0 * steps).round() / steps * 5.0;
                }
                out[0] = inp[0] * (1.0 - p[2]) + *held * p[2];
            }
            _ => {}
        }
    }
}

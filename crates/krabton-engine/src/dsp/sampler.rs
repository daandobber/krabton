//! Sampler: plays samples across the keyboard. Either one sample on every
//! key (from its root key), or zones: samples over ranges of keys and
//! velocities (multi-sampled instruments, SFZ files, slices of a loop).
//! In One-Shot mode a note plays its sample to the end, for drums.
//!
//! Start, End and the loop points are fractions of the part of the sample a
//! zone plays (the whole sample, or a slice), so they work for every zone
//! at once and can be modulated. A loop plays forward (with an optional
//! crossfade) or back and forth.

use super::*;
use crate::params::{ParamSpec, Unit};
use crate::project::SampleZone;
use crate::sample::SampleRef;

pub const ROOT: usize = 0;
pub const TRANSPOSE: usize = 1;
pub const FINE: usize = 2;
pub const START: usize = 3;
pub const LOOP: usize = 4;
pub const ATTACK: usize = 5;
pub const DECAY: usize = 6;
pub const SUSTAIN: usize = 7;
pub const RELEASE: usize = 8;
pub const CUTOFF: usize = 9;
pub const RESO: usize = 10;
pub const VEL_SENS: usize = 11;
pub const VOLUME: usize = 12;
pub const MODE: usize = 13;
pub const END: usize = 14;
pub const LOOP_START: usize = 15;
pub const LOOP_END: usize = 16;
pub const FADE: usize = 17;

pub const MODES: &[&str] = &["Classic", "One-Shot"];
/// How a note loops (the Loop parameter).
pub const LOOP_MODES: &[&str] = &["Off", "Forward", "Ping-Pong"];

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::int("Root Key", "Sample", 0.0, 127.0, 60.0, Unit::None),
    ParamSpec::int("Transpose", "Sample", -48.0, 48.0, 0.0, Unit::Semi),
    ParamSpec::lin("Fine", "Sample", -50.0, 50.0, 0.0, Unit::Cents),
    ParamSpec::lin("Start", "Sample", 0.0, 0.99, 0.0, Unit::Percent),
    ParamSpec::choice("Loop", "Loop", LOOP_MODES, 0),
    ParamSpec::log("Attack", "Envelope", 0.001, 5.0, 0.001, Unit::Sec),
    ParamSpec::log("Decay", "Envelope", 0.005, 10.0, 1.0, Unit::Sec),
    ParamSpec::lin("Sustain", "Envelope", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::log("Release", "Envelope", 0.005, 10.0, 0.1, Unit::Sec),
    ParamSpec::log("Cutoff", "Filter", 20.0, 20000.0, 20000.0, Unit::Hz),
    ParamSpec::lin("Resonance", "Filter", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Vel > Vol", "Global", 0.0, 1.0, 0.6, Unit::Percent),
    ParamSpec::lin("Volume", "Global", -36.0, 6.0, -3.0, Unit::Db),
    ParamSpec::choice("Mode", "Sample", MODES, 0),
    ParamSpec::lin("End", "Sample", 0.01, 1.0, 1.0, Unit::Percent),
    ParamSpec::lin("Loop Start", "Loop", 0.0, 0.99, 0.0, Unit::Percent),
    ParamSpec::lin("Loop End", "Loop", 0.01, 1.0, 1.0, Unit::Percent),
    ParamSpec::lin("Loop Fade", "Loop", 0.0, 0.5, 0.0, Unit::Percent),
];

const MAX_VOICES: usize = 32;
/// Most zones one note starts (velocity layers sounding together).
const MAX_LAYERS: usize = 4;
/// `Voice::zone` of the single sample (no zones).
const SINGLE: usize = usize::MAX;

#[derive(Clone, Copy)]
struct Voice {
    active: bool,
    released: bool,
    note: u8,
    vel: f32,
    age: u64,
    pos: f64,
    /// Playing forward (1) or backward (-1, ping-pong loops).
    dir: f64,
    zone: usize,
    amp: Adsr,
    filt: [Svf; 2],
}

/// What a voice plays: a sample between two frames, maybe looping.
struct Play<'a> {
    sample: &'a SampleRef,
    root: f32,
    cents: f32,
    gain: f32,
    pan: f32,
    start: f64,
    end: f64,
    looped: Option<(f64, f64)>,
    ping_pong: bool,
    /// Crossfade length before the loop end (frames, forward loops).
    fade: f64,
}

fn param(p: &[f32], i: usize) -> f32 {
    p.get(i).copied().unwrap_or(SPECS[i].default)
}

/// Where a part of a sample from `r0` to `r1` (frames) plays with
/// parameters `p`: start, end, and the loop (from `own_loop` when the zone
/// has one, else the loop parameters).
#[allow(clippy::type_complexity)]
pub fn bounds(p: &[f32], r0: f64, r1: f64, own_loop: Option<(u32, u32)>) -> (f64, f64, Option<(f64, f64)>) {
    let len = (r1 - r0).max(1.0);
    let start = (r0 + param(p, START) as f64 * len).floor();
    let end = (r0 + param(p, END) as f64 * len).clamp(start + 16.0, r1.max(start + 16.0));
    let mode = param(p, LOOP).round() as i32;
    if mode == 0 || one_shot(p) {
        return (start, end, None);
    }
    let (a, b) = match own_loop {
        Some((a, b)) if b > a => (a as f64, b as f64),
        _ => (r0 + param(p, LOOP_START) as f64 * len, r0 + param(p, LOOP_END) as f64 * len),
    };
    let a = a.clamp(start, (end - 16.0).max(start));
    let b = b.clamp(a + 16.0, end.max(a + 16.0));
    (start, end, Some((a, b)))
}

fn one_shot(p: &[f32]) -> bool {
    p.get(MODE).is_some_and(|m| *m >= 0.5)
}

/// The part of the sample a zone plays (frames).
pub fn zone_region(z: &SampleZone) -> (f64, f64) {
    let frames = (z.sample.frames() as f64 - 1.0).max(1.0);
    let end = if z.end > 0 { (z.end as f64).min(frames) } else { frames };
    ((z.start as f64).min(end - 1.0).max(0.0), end)
}

/// What zone `zone` (or the single sample) plays with parameters `p`.
fn play<'a>(p: &[f32], sample: Option<&'a SampleRef>, zones: &'a [SampleZone], zone: usize) -> Option<Play<'a>> {
    let ping_pong = param(p, LOOP).round() as i32 == 2;
    let finish = |pl: Play<'a>| {
        let fade = match pl.looped {
            Some((a, b)) if !ping_pong => (param(p, FADE) as f64 * (b - a)).min(a).min((b - a) * 0.5).max(0.0),
            _ => 0.0,
        };
        Play { ping_pong, fade, ..pl }
    };
    if zone == SINGLE {
        let s = sample?;
        let (start, end, looped) = bounds(p, 0.0, (s.frames() as f64 - 1.0).max(1.0), None);
        return Some(finish(Play { sample: s, root: p[ROOT], cents: 0.0, gain: 1.0, pan: 0.0, start, end, looped, ping_pong, fade: 0.0 }));
    }
    let z = zones.get(zone)?;
    let (r0, r1) = zone_region(z);
    let (start, end, looped) = bounds(p, r0, r1, z.loop_range);
    Some(finish(Play { sample: &z.sample, root: z.root as f32, cents: z.tune, gain: db_to_gain(z.gain_db), pan: z.pan, start, end, looped, ping_pong, fade: 0.0 }))
}

pub struct SamplerState {
    /// Pitch bend in semitones (a wheel, a controller's triggers), set by the engine.
    pub pitch_bend: f32,
    voices: [Voice; MAX_VOICES],
    counter: u64,
}

impl Default for SamplerState {
    fn default() -> Self {
        Self::new()
    }
}

impl SamplerState {
    pub fn new() -> Self {
        let v = Voice {
            active: false,
            released: false,
            note: 0,
            vel: 0.0,
            age: 0,
            pos: 0.0,
            dir: 1.0,
            zone: SINGLE,
            amp: Adsr::new(),
            filt: [Svf::new(); 2],
        };
        Self {
            pitch_bend: 0.0, voices: [v; MAX_VOICES], counter: 0 }
    }

    pub fn note_on(&mut self, p: &[f32], sample: Option<&SampleRef>, zones: &[SampleZone], note: u8, vel: f32) {
        if zones.is_empty() {
            if sample.is_some() {
                self.start(p, sample, zones, SINGLE, note, vel);
            }
            return;
        }
        let v127 = (vel * 127.0).round().clamp(1.0, 127.0) as u8;
        let mut started = 0;
        for i in 0..zones.len() {
            let z = &zones[i];
            if (z.lo_key..=z.hi_key).contains(&note) && (z.lo_vel..=z.hi_vel).contains(&v127) {
                self.start(p, sample, zones, i, note, vel);
                started += 1;
                if started == MAX_LAYERS {
                    break;
                }
            }
        }
    }

    fn start(&mut self, p: &[f32], sample: Option<&SampleRef>, zones: &[SampleZone], zone: usize, note: u8, vel: f32) {
        let Some(pl) = play(p, sample, zones, zone) else { return };
        self.counter += 1;
        let idx = self
            .voices
            .iter()
            .position(|v| !v.active)
            .unwrap_or_else(|| self.voices.iter().enumerate().min_by_key(|(_, v)| v.age).map(|(i, _)| i).unwrap_or(0));
        let v = &mut self.voices[idx];
        v.active = true;
        v.released = false;
        v.note = note;
        v.vel = vel;
        v.age = self.counter;
        v.zone = zone;
        v.pos = pl.start;
        v.dir = 1.0;
        v.amp.reset();
        v.amp.trigger();
        for f in &mut v.filt {
            f.reset();
        }
    }

    pub fn note_off(&mut self, p: &[f32], note: u8) {
        if one_shot(p) {
            return;
        }
        for v in self.voices.iter_mut() {
            if v.active && !v.released && v.note == note {
                v.amp.release();
                v.released = true;
            }
        }
    }

    pub fn all_off(&mut self) {
        for v in self.voices.iter_mut() {
            v.amp.release();
            v.released = true;
        }
    }

    pub fn render(&mut self, p: &[f32], sample: Option<&SampleRef>, zones: &[SampleZone], l: &mut [f32], r: &mut [f32], sr: f32) {
        let n = l.len().min(r.len());
        let shot = one_shot(p);
        // One-Shot holds the level after the attack until the sample ends.
        let amp_c = if shot { AdsrCoefs::new(p[ATTACK], p[DECAY], 1.0, p[RELEASE], sr) } else { AdsrCoefs::new(p[ATTACK], p[DECAY], p[SUSTAIN], p[RELEASE], sr) };
        let filter_on = p[CUTOFF] < 19_500.0;
        let vol = db_to_gain(p[VOLUME]);
        let vel_sens = p[VEL_SENS];
        for v in self.voices.iter_mut().filter(|v| v.active) {
            let Some(pl) = play(p, sample, zones, v.zone) else {
                v.active = false;
                continue;
            };
            let s = pl.sample;
            let semis = v.note as f32 - pl.root + p[TRANSPOSE] + (p[FINE] + pl.cents) / 100.0 + self.pitch_bend;
            let rate = 2f64.powf(semis as f64 / 12.0) * s.sample_rate as f64 / sr as f64;
            if filter_on {
                v.filt[0].set(p[CUTOFF], p[RESO], sr);
                v.filt[1].set(p[CUTOFF], p[RESO], sr);
            }
            let g = vol * pl.gain * (1.0 - vel_sens + vel_sens * v.vel);
            let (gl, gr) = pan_gains(pl.pan);
            for i in 0..n {
                match pl.looped {
                    Some((a, b)) if pl.ping_pong => {
                        if v.dir > 0.0 && v.pos >= b {
                            v.dir = -1.0;
                            v.pos = (b - (v.pos - b)).max(a);
                        } else if v.dir < 0.0 && v.pos <= a {
                            v.dir = 1.0;
                            v.pos = (a + (a - v.pos)).min(b);
                        }
                    }
                    Some((a, b)) if v.pos >= b => v.pos = a + (v.pos - a) % (b - a).max(1.0),
                    None if v.pos >= pl.end - 1.0 => {
                        v.active = false;
                        break;
                    }
                    _ => {}
                }
                let (mut a, mut b) = s.read(v.pos);
                // Crossfade into the loop start before reaching the end.
                if let (Some((la, lb)), true) = (pl.looped, pl.fade > 0.0) {
                    if v.pos > lb - pl.fade {
                        let t = ((v.pos - (lb - pl.fade)) / pl.fade) as f32;
                        let (ea, eb) = s.read(v.pos - (lb - la));
                        a = a * (1.0 - t) + ea * t;
                        b = b * (1.0 - t) + eb * t;
                    }
                }
                if filter_on {
                    a = v.filt[0].process(a, FilterMode::Low);
                    b = v.filt[1].process(b, FilterMode::Low);
                }
                let e = v.amp.next(&amp_c) * g;
                l[i] += a * e * gl;
                r[i] += b * e * gr;
                v.pos += rate * v.dir;
                if !v.amp.is_active() {
                    v.active = false;
                    break;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::defaults;
    use crate::sample::SampleData;

    /// A sample of `freq` Hz at 48 kHz.
    fn tone(name: &str, freq: f32, secs: f32) -> SampleRef {
        let n = (secs * 48000.0) as usize;
        let x: Vec<f32> = (0..n).map(|i| (TAU * freq * i as f32 / 48000.0).sin() * 0.5).collect();
        SampleRef::new(SampleData::from_buffers(name, None, 48000, x.clone(), x))
    }

    fn zone(s: &SampleRef, lo: u8, hi: u8, root: u8) -> SampleZone {
        SampleZone { sample: s.clone(), lo_key: lo, hi_key: hi, root, lo_vel: 0, hi_vel: 127, tune: 0.0, gain_db: 0.0, pan: 0.0, start: 0, end: 0, loop_range: None }
    }

    fn crossings(x: &[f32]) -> usize {
        x.windows(2).filter(|w| w[0] <= 0.0 && w[1] > 0.0).count()
    }

    fn play_note(st: &mut SamplerState, p: &[f32], zones: &[SampleZone], note: u8, vel: f32, frames: usize) -> Vec<f32> {
        st.note_on(p, None, zones, note, vel);
        let (mut l, mut r) = (vec![0.0; frames], vec![0.0; frames]);
        st.render(p, None, zones, &mut l, &mut r, 48000.0);
        l
    }

    #[test]
    fn a_note_plays_the_zone_it_falls_in_at_the_right_pitch() {
        let p = defaults(SPECS);
        let low = tone("low", 220.0, 1.0);
        let high = tone("high", 880.0, 1.0);
        let zones = vec![zone(&low, 0, 63, 57), zone(&high, 64, 127, 81)];
        let mut st = SamplerState::new();
        // Half a second of each (higher notes use the sample up sooner).
        // A3 on the low zone plays it as recorded: 220 Hz.
        let x = play_note(&mut st, &p, &zones, 57, 1.0, 24000);
        assert!((crossings(&x) as i32 - 110).abs() <= 2, "{}", crossings(&x));
        // B3 (two semitones up) on the low zone: 247 Hz.
        let mut st = SamplerState::new();
        let x = play_note(&mut st, &p, &zones, 59, 1.0, 24000);
        assert!((crossings(&x) as i32 - 123).abs() <= 2, "{}", crossings(&x));
        // A5 on the high zone: 880 Hz.
        let mut st = SamplerState::new();
        let x = play_note(&mut st, &p, &zones, 81, 1.0, 24000);
        assert!((crossings(&x) as i32 - 440).abs() <= 2, "{}", crossings(&x));
    }

    #[test]
    fn velocity_picks_the_layer() {
        let p = defaults(SPECS);
        let soft = tone("soft", 300.0, 0.5);
        let loud = tone("loud", 600.0, 0.5);
        let mut a = zone(&soft, 0, 127, 60);
        a.hi_vel = 63;
        let mut b = zone(&loud, 0, 127, 60);
        b.lo_vel = 64;
        let zones = vec![a, b];
        let x = play_note(&mut SamplerState::new(), &p, &zones, 60, 0.3, 24000);
        assert!((crossings(&x) as i32 - 150).abs() <= 3, "soft layer: {}", crossings(&x));
        let x = play_note(&mut SamplerState::new(), &p, &zones, 60, 0.9, 24000);
        assert!((crossings(&x) as i32 - 300).abs() <= 3, "loud layer: {}", crossings(&x));
    }

    #[test]
    fn one_shot_ignores_note_off_and_slices_stop_at_their_end() {
        let mut p = defaults(SPECS);
        p[MODE] = 1.0;
        let s = tone("loop", 100.0, 1.0);
        let mut z = zone(&s, 36, 36, 36);
        z.start = 12000;
        z.end = 24000;
        let zones = vec![z];
        let mut st = SamplerState::new();
        st.note_on(&p, None, &zones, 36, 1.0);
        st.note_off(&p, 36);
        let (mut l, mut r) = (vec![0.0; 18000], vec![0.0; 18000]);
        st.render(&p, None, &zones, &mut l, &mut r, 48000.0);
        let rms = |x: &[f32]| (x.iter().map(|v| v * v).sum::<f32>() / x.len() as f32).sqrt();
        assert!(rms(&l[1000..11000]) > 0.1, "still playing after note-off");
        assert!(rms(&l[12500..18000]) < 1e-4, "stops at the slice end");
    }

    #[test]
    fn loops_keep_playing_forward_or_back_and_forth() {
        let s = tone("short", 500.0, 0.25);
        for mode in [1.0, 2.0] {
            let mut p = defaults(SPECS);
            p[LOOP] = mode;
            p[LOOP_START] = 0.25;
            p[LOOP_END] = 0.75;
            p[FADE] = if mode == 1.0 { 0.2 } else { 0.0 };
            let mut st = SamplerState::new();
            st.note_on(&p, Some(&s), &[], 60, 1.0);
            // Four times as long as the sample.
            let (mut l, mut r) = (vec![0.0; 48000], vec![0.0; 48000]);
            st.render(&p, Some(&s), &[], &mut l, &mut r, 48000.0);
            let tail = &l[40000..];
            let rms = (tail.iter().map(|v| v * v).sum::<f32>() / tail.len() as f32).sqrt();
            assert!(rms > 0.1, "mode {mode}: still sounding ({rms})");
            assert!(l.windows(2).all(|w| (w[1] - w[0]).abs() < 0.2), "mode {mode}: no clicks");
        }
    }

    #[test]
    fn start_and_end_are_parts_of_the_sample() {
        let mut p = defaults(SPECS);
        p[START] = 0.5;
        p[END] = 0.75;
        let (start, end, looped) = bounds(&p, 1000.0, 3000.0, None);
        assert_eq!((start, end, looped), (2000.0, 2500.0, None));
        p[LOOP] = 1.0;
        p[LOOP_START] = 0.0;
        p[LOOP_END] = 1.0;
        // The loop stays inside start and end.
        assert_eq!(bounds(&p, 1000.0, 3000.0, None).2, Some((2000.0, 2500.0)));
        // A zone's own loop (from an SFZ) wins.
        assert_eq!(bounds(&p, 1000.0, 3000.0, Some((2100, 2400))).2, Some((2100.0, 2400.0)));
    }

    #[test]
    fn the_single_sample_still_plays_without_zones() {
        let p = defaults(SPECS);
        let s = tone("one", 440.0, 1.0);
        let mut st = SamplerState::new();
        st.note_on(&p, Some(&s), &[], 60, 1.0);
        let (mut l, mut r) = (vec![0.0; 48000], vec![0.0; 48000]);
        st.render(&p, Some(&s), &[], &mut l, &mut r, 48000.0);
        assert!((crossings(&l) as i32 - 440).abs() <= 3, "{}", crossings(&l));
    }
}

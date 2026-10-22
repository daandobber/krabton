//! Percussion: a percussion ensemble. Pick a style (son montuno,
//! guaguancó, bossa nova, samba, baião, cumbia, bembé in 6/8) and its
//! players lock in around the timeline (the clave or the bell): shakers,
//! congas with their open, muted and low tones, bongos, a low drum, a
//! güiro... Density brings them in one by one, the lead drum improvises
//! (licks every bar as Variation rises, a call at the end of every four
//! bars, and the bongo player leaves the martillo for it), the others
//! now and then drop a note or add a ghost one, and Humanize loosens them.
//!
//! It plays the percussion of the kit on the track: a drum the kit lacks
//! takes the nearest one it has (congas become toms, the claves a rim).

use super::SeqState;
use crate::midi_fx::{MidiCtx, MidiEvent};
use crate::params::{ParamSpec, Unit};

pub const STYLE: usize = 0;
pub const DENSITY: usize = 1;
pub const VARIATION: usize = 2;
pub const HUMANIZE: usize = 3;
pub const SWING: usize = 4;
pub const VELOCITY: usize = 5;
pub const LEAD: usize = 6;
pub const SPEED: usize = 7;

pub const SPEEDS: &[&str] = &["Half Time", "Normal", "Double Time"];

/// The instruments, with the notes each would like (General MIDI first),
/// for its first, second and third sound.
pub struct Instrument {
    pub name: &'static str,
    pub notes: [&'static [u8]; 3],
}

pub const INSTRUMENTS: &[Instrument] = &[
    Instrument { name: "Claves", notes: [&[75, 37, 76, 56], &[76, 37], &[75]] },
    Instrument { name: "Cowbell", notes: [&[56, 67, 37], &[56, 68, 37], &[56]] },
    Instrument { name: "Agogo", notes: [&[67, 56, 37], &[68, 56, 37], &[67]] },
    Instrument { name: "Shaker", notes: [&[70, 82, 69, 54, 42], &[69, 70, 42], &[70]] },
    Instrument { name: "Congas", notes: [&[63, 50, 48, 47], &[62, 63, 48, 37], &[64, 45, 43, 41]] },
    Instrument { name: "Bongos", notes: [&[60, 63, 50, 48], &[61, 62, 64, 47, 45], &[60]] },
    Instrument { name: "Low Drum", notes: [&[41, 43, 45, 36], &[41, 43, 45, 36], &[43, 41, 36]] },
    Instrument { name: "Guiro", notes: [&[74, 73, 69, 70, 42], &[73, 70, 42], &[74]] },
    Instrument { name: "Timbales", notes: [&[65, 50, 37], &[66, 47, 45], &[65]] },
    Instrument { name: "Triangle", notes: [&[81, 80, 53, 46, 42], &[80, 42], &[81]] },
    Instrument { name: "Caixa", notes: [&[38, 40, 37], &[37, 38], &[38]] },
];
const CLAVES: usize = 0;
const COWBELL: usize = 1;
const AGOGO: usize = 2;
const SHAKER: usize = 3;
const CONGAS: usize = 4;
const BONGOS: usize = 5;
const LOW: usize = 6;
const GUIRO: usize = 7;
const TIMBALES: usize = 8;
const TRIANGLE: usize = 9;
const CAIXA: usize = 10;

/// A style: steps per bar (16, or 12 for 6/8), its parts (the timeline
/// first, then in the order Density brings them in) and its lead: the
/// instrument, its quiet pattern, and the part it takes over when it
/// improvises.
pub struct Style {
    pub name: &'static str,
    pub steps: usize,
    pub parts: &'static [(usize, &'static str)],
    pub lead: (usize, &'static str, Option<usize>),
}

// Notation: x accent, o hit, - ghost (first sound); s accent, m hit (second
// sound); t the third sound; . rest. One or two bars.
pub const STYLES: &[Style] = &[
    Style {
        name: "Son Montuno",
        steps: 16,
        parts: &[
            (CLAVES, "x.....x.....x... ....x...x......."),
            (LOW, "......o.....x... ......o.....x..."),
            (CONGAS, "-.-.s.-.-.-.o.t. -.-.s.-.-.-.o.t."),
            (SHAKER, "x-o-x-o-x-o-x-o- x-o-x-o-x-o-x-o-"),
            (BONGOS, "x.o.o.m.x.o.o.m. x.o.o.m.x.o.o.m."),
            (COWBELL, "x...o.o.x...o.o. x...o.o.x...o.o."),
            (GUIRO, "o.mmo.mmo.mmo.mm o.mmo.mmo.mmo.mm"),
        ],
        lead: (BONGOS, "................ ................", Some(4)),
    },
    Style {
        name: "Guaguanco",
        steps: 16,
        parts: &[
            (CLAVES, "x.....x.......x. ....x...x......."),
            (TIMBALES, "x.x.xx.x.x.xx.x. x.x.xx.x.x.xx.x."),
            (CONGAS, "-.-.m.o.o.-.t.t. -.-.s.o.o.-.t..."),
            (SHAKER, "o-o-o-o-o-o-o-o- o-o-o-o-o-o-o-o-"),
            (LOW, "............o... ............o..."),
        ],
        lead: (CONGAS, "..........s..s.. ....s.....o.o...", None),
    },
    Style {
        name: "Bossa Nova",
        steps: 16,
        parts: &[
            (CLAVES, "x.....x.....x... ....x.....x....."),
            (SHAKER, "o-o-o-o-o-o-o-o- o-o-o-o-o-o-o-o-"),
            (LOW, "x..o....x..o.... x..o....x..o...."),
            (CONGAS, "....m.o.....m.o. ....m.o.....m.o."),
            (TRIANGLE, "m.m.o.m.m.m.o.m. m.m.o.m.m.m.o.m."),
        ],
        lead: (BONGOS, "................ ................", None),
    },
    Style {
        name: "Samba",
        steps: 16,
        parts: &[
            (LOW, "-...x...-...x..."),
            (TIMBALES, "x.x..x.x..x.x..."),
            (SHAKER, "o-oxo-oxo-oxo-ox"),
            (AGOGO, "x.m.x.m.xm.m.m.."),
            (CAIXA, "x-o-x-oox-o-x-oo"),
            (CONGAS, "..t...t...t...t."),
        ],
        lead: (TIMBALES, "..............s.", None),
    },
    Style {
        name: "Baiao",
        steps: 16,
        parts: &[
            (LOW, "x..o-...x..o-..."),
            (TRIANGLE, "mmommmommmommmom"),
            (CONGAS, "....o.o.....o.o."),
            (SHAKER, "o-o-o-o-o-o-o-o-"),
            (AGOGO, "x.....m.x...m..."),
        ],
        lead: (CONGAS, "................", None),
    },
    Style {
        name: "Cumbia",
        steps: 16,
        parts: &[
            (GUIRO, "o.mmo.mmo.mmo.mm"),
            (LOW, "x...-...x...-.o."),
            (CONGAS, "..o...o...o...o."),
            (SHAKER, "x-o-x-o-x-o-x-o-"),
            (COWBELL, "o.......o......."),
        ],
        lead: (CONGAS, "......o.o.....s.", None),
    },
    Style {
        name: "Bembe 6/8",
        steps: 12,
        parts: &[
            (COWBELL, "x.x.xx.x.x.x"),
            (SHAKER, "x-ox-ox-ox-o"),
            (LOW, "x.....x....."),
            (CONGAS, "m.oo.mm.oo.m"),
            (CLAVES, "x..x..x..x.."),
        ],
        lead: (CONGAS, "......s..s..", None),
    },
];

pub const STYLE_NAMES: &[&str] = &["Son Montuno", "Guaguanco", "Bossa Nova", "Samba", "Baiao", "Cumbia", "Bembe 6/8"];

/// What the lead plays when it improvises: a bar's licks, and the calls
/// that end a phrase.
const LICKS_16: &[&str] = &["o.o.o.....s.s...", "....s..s..o.o.o.", "s.....o.o.o.s...", "..o.o.s...o.o...", "o.o.s.s.o.o.s.s.", "......o.oo.os...", "s..s..s...o.o.o.", "o..o..o.o..o..s."];
const CALLS_16: &[&str] = &["o.o.o.o.s.s.s.s.", "s.s.o.o.s.s.o.oo", "x..x..x...s.s.ss"];
const LICKS_12: &[&str] = &["o.o..s.o.o..", "s..s..o.o.o.", "..o.o.s..s..", "o.oo.os.....", "s.s.s.o.o.o."];
const CALLS_12: &[&str] = &["o.o.o.s.s.s.", "s.ssos.ssos."];

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::choice("Style", "Ensemble", STYLE_NAMES, 0),
    ParamSpec::lin("Density", "Ensemble", 0.0, 1.0, 0.75, Unit::Percent),
    ParamSpec::lin("Variation", "Ensemble", 0.0, 1.0, 0.4, Unit::Percent),
    ParamSpec::lin("Humanize", "Feel", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::lin("Swing", "Feel", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Velocity", "Feel", 0.05, 1.0, 0.8, Unit::Percent),
    ParamSpec::toggle("Lead", "Ensemble", true),
    ParamSpec::choice("Speed", "Feel", SPEEDS, 1),
    ParamSpec::toggle("Transpose", "Keys", false),
    ParamSpec::toggle("Hold", "Keys", false),
    ParamSpec::lin("Vel Low", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Vel High", "Dynamics", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::lin("Vel Random", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Vel Accent", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
];

pub fn style(p: &[f32]) -> &'static Style {
    &STYLES[(p[STYLE].round().max(0.0) as usize).min(STYLES.len() - 1)]
}

/// Beats per step: sixteenths, or triplet eighths in 6/8 (twice as long at
/// half time, half as long at double time).
pub fn step_beats(p: &[f32]) -> f64 {
    let speed = [2.0, 1.0, 0.5][(p.get(SPEED).copied().unwrap_or(1.0).round().max(0.0) as usize).min(2)];
    4.0 / style(p).steps as f64 * speed
}

/// A pattern's symbols without the spaces.
pub fn symbols(s: &str) -> impl Iterator<Item = char> + '_ {
    s.chars().filter(|c| !c.is_whitespace())
}

/// The symbol at step `i` (wrapping round the pattern's bars).
pub fn symbol(s: &str, i: usize) -> char {
    let n = symbols(s).count().max(1);
    symbols(s).nth(i % n).unwrap_or('.')
}

/// Sound (0, 1, 2) and loudness of a symbol.
fn sound(c: char) -> Option<(usize, f32)> {
    match c {
        'x' => Some((0, 1.0)),
        'o' => Some((0, 0.75)),
        '-' => Some((0, 0.4)),
        's' => Some((1, 1.0)),
        'm' => Some((1, 0.7)),
        't' => Some((2, 0.85)),
        _ => None,
    }
}

/// The note an instrument's sound plays in a kit (0: no kit, General MIDI).
pub fn note_for(instrument: usize, sound: usize, kit: u128) -> u8 {
    let c = INSTRUMENTS[instrument.min(INSTRUMENTS.len() - 1)].notes[sound.min(2)];
    if kit == 0 {
        return c[0];
    }
    // The kit's own, else the nearest it has.
    c.iter().copied().find(|n| kit >> n & 1 == 1).or_else(|| (35u8..=81).filter(|n| kit >> n & 1 == 1).min_by_key(|n| (*n as i32 - c[0] as i32).abs())).unwrap_or(c[0])
}

/// How many of a style's parts play at a density (the timeline always).
pub fn parts_playing(s: &Style, density: f32) -> usize {
    1 + (density.clamp(0.0, 1.0) * (s.parts.len() - 1) as f32).round() as usize
}

#[derive(Clone, Debug, Default)]
pub struct PercussionState {
    bar: i64,
    /// The lead's pattern this bar (None: its quiet one).
    lick: Option<&'static str>,
    pub call: bool,
}

impl SeqState {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn percussion(&mut self, k: i64, p: &[f32], at: u64, step_frames: f64, out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        let st = style(p);
        let n = st.steps as i64;
        let bar = k.div_euclid(n);
        let i = k.rem_euclid(n) as usize;
        let variation = p[VARIATION].clamp(0.0, 1.0);
        let human = p[HUMANIZE].clamp(0.0, 1.0);
        let lead_on = p[LEAD] >= 0.5;
        if bar != self.perc.bar || i == 0 {
            // A new bar: does the lead improvise?
            self.perc.bar = bar;
            let phrase_end = bar.rem_euclid(4) == 3;
            let (licks, calls) = if st.steps == 12 { (LICKS_12, CALLS_12) } else { (LICKS_16, CALLS_16) };
            self.perc.call = false;
            self.perc.lick = None;
            if lead_on && phrase_end && self.unit() < variation * 1.3 {
                self.perc.call = true;
                self.perc.lick = Some(calls[(self.unit() * calls.len() as f32) as usize % calls.len()]);
            } else if lead_on && self.unit() < variation {
                self.perc.lick = Some(licks[(self.unit() * licks.len() as f32) as usize % licks.len()]);
            }
        }
        let playing = parts_playing(st, p[DENSITY]);
        let replaced = if self.perc.lick.is_some() { st.lead.2 } else { None };
        let swing = if st.steps == 16 && k % 2 == 1 { p[SWING].clamp(0.0, 1.0) as f64 * 0.33 * step_frames } else { 0.0 };
        let vel = p[VELOCITY].clamp(0.05, 1.0);
        let mut fired = 0u64;
        // The parts, then the lead (as part 7).
        for part in 0..=st.parts.len() {
            let (inst, pat, is_lead) = if part == st.parts.len() {
                if !lead_on {
                    continue;
                }
                (st.lead.0, self.perc.lick.unwrap_or(st.lead.1), true)
            } else {
                if part >= playing || replaced == Some(part) {
                    continue;
                }
                (st.parts[part].0, st.parts[part].1, false)
            };
            let c = if is_lead { symbol(pat, i) } else { symbol(pat, bar.rem_euclid(2) as usize * st.steps + i) };
            let hit = match sound(c) {
                // Now and then a player leaves a note out...
                Some(_) if !is_lead && part > 0 && self.unit() < variation * 0.08 => None,
                Some(h) => Some(h),
                // ... or slips a ghost note in (shakers and drums).
                None if !is_lead && matches!(inst, SHAKER | CONGAS | BONGOS) && self.unit() < variation * 0.05 => Some((0, 0.35)),
                None => None,
            };
            let Some((snd, level)) = hit else { continue };
            fired |= 1 << part.min(7);
            let delay = swing + self.unit() as f64 * human as f64 * 0.12 * step_frames;
            let v = vel * level * (1.0 + (self.unit() - 0.5) * 0.3 * human);
            let note = note_for(inst, snd, ctx.drum_notes);
            self.play(ctx, at + delay as u64, note as i32, v.clamp(0.05, 1.0), step_frames * 0.8, out);
        }
        let lick_mask = self.perc.lick.map(|l| symbols(l).enumerate().filter(|(_, c)| sound(*c).is_some()).fold(0u64, |m, (j, _)| m | 1 << j)).unwrap_or(0);
        self.visual[0] = i as u64 | (st.steps as u64) << 8 | (bar as u64 & 0xff) << 16;
        self.visual[1] = fired | (self.perc.lick.is_some() as u64) << 8 | (self.perc.call as u64) << 9;
        self.visual[2] = lick_mask;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::midi_fx::MidiFxKind;
    use crate::params::defaults;
    use crate::scale::{Scale, ScaleKind};

    fn run(p: &[f32], bars: usize) -> Vec<(f64, u8)> {
        let mut s = SeqState::new();
        let mut notes = Vec::new();
        let block = 512;
        let mut frame = 0;
        while frame < bars * 4 * 24_000 {
            let mut out = Vec::new();
            let ctx = MidiCtx { now: frame as u64, len: block, samples_per_beat: 24_000.0, scale: Scale::new(0, ScaleKind::Major), scale_on: false, beat: frame as f64 / 24_000.0, playing: true, sr: 48_000.0, drum_notes: 0, lead: 0.0, touch: [0.0; 4] };
            s.process(MidiFxKind::Percussion, p, &[], &mut out, &ctx);
            for e in out.iter().filter(|e| e.on) {
                notes.push(((frame + e.offset as usize) as f64 / 24_000.0, e.pitch));
            }
            frame += block;
        }
        notes
    }

    #[test]
    fn every_style_is_whole_bars() {
        for s in STYLES {
            for (inst, pat) in s.parts.iter().map(|(i, p)| (*i, *p)).chain(std::iter::once((s.lead.0, s.lead.1))) {
                let n = symbols(pat).count();
                assert!(n == s.steps || n == 2 * s.steps, "{} {}: {n}", s.name, INSTRUMENTS[inst].name);
                assert!(symbols(pat).all(|c| ".xo-smt".contains(c)), "{}", s.name);
            }
        }
        assert_eq!(STYLE_NAMES.len(), STYLES.len());
        for (s, n) in STYLES.iter().zip(STYLE_NAMES) {
            assert_eq!(s.name, *n);
        }
        for l in LICKS_16.iter().chain(CALLS_16) {
            assert_eq!(symbols(l).count(), 16);
        }
        for l in LICKS_12.iter().chain(CALLS_12) {
            assert_eq!(symbols(l).count(), 12);
        }
    }

    #[test]
    fn the_clave_keeps_time() {
        let mut p = defaults(SPECS);
        p[VARIATION] = 0.0;
        p[HUMANIZE] = 0.0;
        let notes = run(&p, 2);
        let clave: Vec<f64> = notes.iter().filter(|n| n.1 == 75).map(|n| n.0).collect();
        // Son clave 3-2: 1, 2&, 4 | 2, 3.
        let want = [0.0, 1.5, 3.0, 5.0, 6.0];
        assert!(clave.len() == want.len() && clave.iter().zip(want).all(|(a, b)| (a - b).abs() < 1e-3), "{clave:?}");
    }

    #[test]
    fn density_brings_players_in() {
        let mut p = defaults(SPECS);
        p[VARIATION] = 0.0;
        p[DENSITY] = 0.0;
        p[LEAD] = 0.0;
        let few: std::collections::BTreeSet<u8> = run(&p, 2).iter().map(|n| n.1).collect();
        assert_eq!(few.into_iter().collect::<Vec<_>>(), vec![75]);
        p[DENSITY] = 1.0;
        let all: std::collections::BTreeSet<u8> = run(&p, 2).iter().map(|n| n.1).collect();
        assert!(all.len() >= 8, "{all:?}");
    }

    #[test]
    fn six_eight_runs_in_triplets_and_the_lead_improvises() {
        let mut p = defaults(SPECS);
        p[STYLE] = 6.0;
        p[VARIATION] = 1.0;
        p[HUMANIZE] = 0.0;
        let notes = run(&p, 8);
        let bell: Vec<f64> = notes.iter().filter(|n| n.1 == 56).map(|n| n.0).take(7).collect();
        let third = 1.0 / 3.0;
        let want: Vec<f64> = [0, 2, 4, 5, 7, 9, 11].iter().map(|&s| s as f64 * third).collect();
        assert!(bell.iter().zip(&want).all(|(a, b)| (a - b).abs() < 1e-3), "{bell:?}");
        // Improvising, the lead (open and slapped conga) plays far more than its quiet part.
        let congas = |notes: &[(f64, u8)]| notes.iter().filter(|n| n.1 == 62 || n.1 == 63).count();
        p[VARIATION] = 0.0;
        let quiet = run(&p, 8);
        assert!(congas(&notes) > congas(&quiet) + 16, "{} {}", congas(&notes), congas(&quiet));
    }

    #[test]
    fn a_kit_without_congas_plays_toms() {
        let kit: u128 = [36u8, 38, 42, 45, 47, 48, 50, 37, 56].iter().fold(0, |m, n| m | 1 << n);
        assert_eq!(note_for(CONGAS, 0, kit), 50);
        assert_eq!(note_for(CONGAS, 2, kit), 45);
        assert_eq!(note_for(CLAVES, 0, kit), 37);
    }
}

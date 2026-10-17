//! The demo set that opens on first launch, so there is something to jam with.

use crate::dsp::effects::EffectKind;
use crate::project::*;

const KICK: u8 = 36;
const RIM: u8 = 37;
const CLAP: u8 = 39;
const CHAT: u8 = 42;
const TOM: u8 = 45;
const OHAT: u8 = 46;
const BELL: u8 = 56;

/// A minor - F - C - G, one chord per bar.
const ROOTS: [u8; 4] = [33, 29, 36, 31];
const TRIADS: [[u8; 3]; 4] = [[57, 60, 64], [53, 57, 60], [55, 60, 64], [55, 59, 62]];
const SEVENTHS: [u8; 4] = [67, 64, 71, 65];

fn n(pitch: u8, start: f64, length: f64, vel: f32) -> Note {
    Note::new(pitch, start, length, vel)
}

fn set_clip(p: &mut Project, ti: usize, si: usize, name: &str, length: f64, notes: Vec<Note>) {
    let id = p.alloc_id();
    let color = p.tracks[ti].color;
    let mut c = Clip::new_midi(id, name, color, length);
    if let Some(v) = c.notes_mut() {
        *v = notes;
    }
    p.tracks[ti].slots[si] = Some(c);
}

fn every(step: f64, len: f64, mut f: impl FnMut(f64) -> Option<Note>) -> Vec<Note> {
    let mut out = Vec::new();
    let mut t = 0.0;
    while t < len - 1e-9 {
        if let Some(n) = f(t) {
            out.push(n);
        }
        t += step;
    }
    out
}

fn is(t: f64, x: f64) -> bool {
    (t - x).abs() < 1e-6
}

pub fn demo_project() -> Project {
    let mut p = Project::blank();
    p.name = "Krabton Demo".into();
    p.bpm = 124.0;
    p.scale = crate::scale::Scale::new(9, crate::scale::ScaleKind::Minor);
    for (i, name) in ["Intro", "Groove", "Break", "Drop"].iter().enumerate() {
        p.scenes[i].name = name.to_string();
    }

    // --- Drums -----------------------------------------------------------
    let drums = p.add_midi_track(Some(Instrument::drums("Classic 909")));
    p.tracks[drums].color = PALETTE[1];
    let mut comp = p.make_device(EffectKind::Compressor);
    comp.params[0] = -16.0;
    comp.params[1] = 3.0;
    comp.params[2] = 8.0;
    comp.params[4] = 3.0;
    p.tracks[drums].devices.push(comp);
    p.tracks[drums].mixer.sends = vec![0.08, 0.0];

    let intro = every(0.5, 4.0, |t| if t.fract() == 0.0 { Some(n(KICK, t, 0.25, 1.0)) } else { Some(n(CHAT, t, 0.2, 0.55)) });
    set_clip(&mut p, drums, 0, "Intro Beat", 4.0, intro);

    let mut groove = every(0.25, 4.0, |t| {
        let f = t.fract();
        if is(f, 0.5) {
            Some(n(OHAT, t, 0.25, 0.6))
        } else if is(f, 0.0) {
            None
        } else {
            Some(n(CHAT, t, 0.1, if is(f, 0.25) { 0.35 } else { 0.5 }))
        }
    });
    groove.extend(every(1.0, 4.0, |t| Some(n(KICK, t, 0.25, 1.0))));
    groove.push(n(CLAP, 1.0, 0.25, 0.9));
    groove.push(n(CLAP, 3.0, 0.25, 0.9));
    set_clip(&mut p, drums, 1, "Groove", 4.0, groove);

    let mut brk = every(0.25, 8.0, |t| Some(n(CHAT, t, 0.1, if is(t.fract(), 0.0) { 0.5 } else { 0.3 })));
    for b in [1.0, 3.0, 5.0, 7.0] {
        brk.push(n(CLAP, b, 0.25, 0.85));
    }
    for (i, t) in [6.5, 6.75, 7.0, 7.25, 7.5, 7.75].iter().enumerate() {
        brk.push(n(TOM, *t, 0.25, 0.6 + i as f32 * 0.07));
    }
    set_clip(&mut p, drums, 2, "Break", 8.0, brk);

    let mut drop = every(0.25, 4.0, |t| {
        let f = t.fract();
        if is(f, 0.5) {
            Some(n(OHAT, t, 0.25, 0.7))
        } else if is(f, 0.0) {
            None
        } else {
            Some(n(CHAT, t, 0.1, 0.45))
        }
    });
    drop.extend(every(1.0, 4.0, |t| Some(n(KICK, t, 0.25, 1.0))));
    drop.extend([n(CLAP, 1.0, 0.25, 1.0), n(CLAP, 3.0, 0.25, 1.0)]);
    drop.extend([n(RIM, 0.75, 0.1, 0.7), n(RIM, 2.25, 0.1, 0.6), n(RIM, 3.75, 0.1, 0.7)]);
    drop.extend([n(BELL, 1.75, 0.1, 0.5), n(BELL, 3.25, 0.1, 0.45)]);
    set_clip(&mut p, drums, 3, "Drop", 4.0, drop);

    // --- Bass ------------------------------------------------------------
    let bass = p.add_midi_track(Some(Instrument::synth("Fat Bass")));
    p.tracks[bass].name = "Bass".into();
    p.tracks[bass].color = PALETTE[9];
    let mut sat = p.make_device(EffectKind::Saturator);
    sat.params[0] = 4.0;
    sat.params[4] = 0.6;
    p.tracks[bass].devices.push(sat);
    p.tracks[bass].mixer.volume_db = -2.0;

    let mut groove = Vec::new();
    for (bar, &root) in ROOTS.iter().enumerate() {
        let o = bar as f64 * 4.0;
        for beat in 0..4 {
            groove.push(n(root, o + beat as f64 + 0.5, 0.35, 0.9));
        }
        groove.push(n(root + 12, o + 3.75, 0.2, 0.7));
    }
    set_clip(&mut p, bass, 1, "Offbeat Bass", 16.0, groove);

    let long = ROOTS.iter().enumerate().map(|(bar, &r)| n(r, bar as f64 * 4.0, 3.9, 0.8)).collect();
    set_clip(&mut p, bass, 2, "Long Bass", 16.0, long);

    let mut roll = Vec::new();
    for (bar, &root) in ROOTS.iter().enumerate() {
        let o = bar as f64 * 4.0;
        for beat in 0..4 {
            for sub in [0.25, 0.5, 0.75] {
                let up = beat == 3 && is(sub, 0.75);
                roll.push(n(if up { root + 12 } else { root }, o + beat as f64 + sub, 0.2, if is(sub, 0.25) { 0.7 } else { 0.9 }));
            }
        }
    }
    set_clip(&mut p, bass, 3, "Rolling Bass", 16.0, roll);

    // --- Chords ----------------------------------------------------------
    let chords = p.add_midi_track(Some(Instrument::synth("Soft Pad")));
    p.tracks[chords].name = "Chords".into();
    p.tracks[chords].color = PALETTE[11];
    p.tracks[chords].mixer.volume_db = -3.0;
    p.tracks[chords].mixer.sends = vec![0.55, 0.0];
    let triads: Vec<Note> = TRIADS
        .iter()
        .enumerate()
        .flat_map(|(bar, ch)| ch.iter().map(move |&k| n(k, bar as f64 * 4.0, 3.95, 0.7)))
        .collect();
    let mut sevenths = triads.clone();
    for (bar, &k) in SEVENTHS.iter().enumerate() {
        sevenths.push(n(k, bar as f64 * 4.0, 3.95, 0.6));
    }
    set_clip(&mut p, chords, 0, "Pads", 16.0, triads.clone());
    set_clip(&mut p, chords, 1, "Pads", 16.0, triads.clone());
    set_clip(&mut p, chords, 2, "Pads 7th", 16.0, sevenths);
    set_clip(&mut p, chords, 3, "Pads", 16.0, triads);

    // --- Arp -------------------------------------------------------------
    let arp = p.add_midi_track(Some(Instrument::synth("Pluck")));
    p.tracks[arp].name = "Arp".into();
    p.tracks[arp].color = PALETTE[6];
    p.tracks[arp].mixer.volume_db = -4.0;
    p.tracks[arp].mixer.sends = vec![0.35, 0.45];
    let pattern = [0usize, 1, 2, 3, 2, 1, 0, 1, 2, 3, 2, 1, 0, 1, 2, 3];
    let make_arp = |vel: f32| {
        let mut v = Vec::new();
        for (bar, ch) in TRIADS.iter().enumerate() {
            let tones = [ch[0] + 12, ch[1] + 12, ch[2] + 12, ch[0] + 24];
            for (i, &idx) in pattern.iter().enumerate() {
                let accent = if i % 4 == 0 { 1.0 } else { 0.75 };
                v.push(n(tones[idx], bar as f64 * 4.0 + i as f64 * 0.25, 0.2, vel * accent));
            }
        }
        v
    };
    set_clip(&mut p, arp, 0, "Arp Soft", 16.0, make_arp(0.55));
    set_clip(&mut p, arp, 2, "Arp", 16.0, make_arp(0.75));
    set_clip(&mut p, arp, 3, "Arp Loud", 16.0, make_arp(0.95));

    // --- FM Keys (FM, DX7-style electric piano) -------------------------
    let keys = p.add_midi_track(Some(Instrument::fm(&crate::dsp::dx7::factory_bank()[0], "Krab Factory")));
    p.tracks[keys].name = "FM Keys".into();
    p.tracks[keys].color = PALETTE[3];
    p.tracks[keys].mixer.volume_db = -7.0;
    p.tracks[keys].mixer.sends = vec![0.3, 0.2];
    let mut stabs = Vec::new();
    for (bar, ch) in TRIADS.iter().enumerate() {
        for t in [1.5, 3.5] {
            for &k in ch {
                stabs.push(n(k + 12, bar as f64 * 4.0 + t, 0.35, 0.75));
            }
        }
    }
    let ep_chords: Vec<Note> = TRIADS
        .iter()
        .enumerate()
        .flat_map(|(bar, ch)| ch.iter().map(move |&k| n(k, bar as f64 * 4.0, 3.5, 0.6)))
        .collect();
    set_clip(&mut p, keys, 1, "EP Stabs", 16.0, stabs.clone());
    set_clip(&mut p, keys, 2, "EP Chords", 16.0, ep_chords);
    set_clip(&mut p, keys, 3, "EP Stabs", 16.0, stabs);

    p.loop_end = 16.0;
    p
}

/// A new age ambient set, "Forest Morning": a brook and birds, wind chimes
/// and the wind, a drone, a pan flute and a glass bowl. Brook, birds and
/// chimes play by themselves while the song plays; the scenes bring in the
/// drone, the bowl and the flute.
pub fn ambient_project() -> Project {
    let mut p = Project::blank();
    p.name = "Forest Morning".into();
    p.bpm = 72.0;
    p.scale = crate::scale::Scale::new(9, crate::scale::ScaleKind::Minor);
    for (i, name) in ["Dawn", "Morning", "Flute", "Dusk"].iter().enumerate() {
        p.scenes[i].name = name.to_string();
    }

    // --- Outside: they need no clips --------------------------------------
    let brook = p.add_midi_track(Some(Instrument::nature("Brook")));
    p.tracks[brook].name = "Brook".into();
    p.tracks[brook].color = PALETTE[9];
    p.tracks[brook].mixer.volume_db = -4.0;
    let birds = p.add_midi_track(Some(Instrument::birds("Garden Morning")));
    p.tracks[birds].name = "Birds".into();
    p.tracks[birds].color = PALETTE[6];
    p.tracks[birds].mixer.volume_db = -6.0;
    let chimes = p.add_midi_track(Some(Instrument::wind_chimes("Bamboo Grove")));
    p.tracks[chimes].name = "Chimes".into();
    p.tracks[chimes].color = PALETTE[3];
    p.tracks[chimes].mixer.volume_db = -10.0;

    // --- The drone: A, and F at dusk ----------------------------------------
    let drone = p.add_midi_track(Some(Instrument::drone("Shimmer Fields")));
    p.tracks[drone].name = "Drone".into();
    p.tracks[drone].color = PALETTE[11];
    p.tracks[drone].mixer.volume_db = -9.0;
    p.tracks[drone].mixer.sends = vec![0.3, 0.0];
    let held = |keys: &[u8]| keys.iter().map(|&k| n(k, 0.0, 15.9, 0.7)).collect::<Vec<_>>();
    set_clip(&mut p, drone, 0, "A", 16.0, held(&[45, 57]));
    set_clip(&mut p, drone, 1, "A", 16.0, held(&[45, 57, 64]));
    set_clip(&mut p, drone, 2, "A", 16.0, held(&[45, 57, 64]));
    set_clip(&mut p, drone, 3, "F", 16.0, held(&[41, 53, 60]));

    // --- A glass bowl now and then ------------------------------------------
    let bowl = p.add_midi_track(Some(Instrument::rings("Glass Bowl")));
    p.tracks[bowl].name = "Bowl".into();
    p.tracks[bowl].color = PALETTE[1];
    p.tracks[bowl].mixer.volume_db = -8.0;
    p.tracks[bowl].mixer.sends = vec![0.45, 0.0];
    set_clip(&mut p, bowl, 1, "Bowl", 16.0, vec![n(69, 0.0, 2.0, 0.6), n(76, 8.0, 2.0, 0.5)]);
    set_clip(&mut p, bowl, 2, "Bowl", 16.0, vec![n(69, 0.0, 2.0, 0.6)]);
    set_clip(&mut p, bowl, 3, "Bowl", 16.0, vec![n(65, 0.0, 2.0, 0.55), n(72, 8.0, 2.0, 0.45)]);

    // --- The pan flute: eight bars in A minor pentatonic ---------------------
    let flute = p.add_midi_track(Some(Instrument::pan_flute("Romanian Pan Flute")));
    p.tracks[flute].name = "Pan Flute".into();
    p.tracks[flute].color = PALETTE[0];
    p.tracks[flute].mixer.volume_db = -5.0;
    p.tracks[flute].mixer.sends = vec![0.1, 0.0];
    // Played over a lake: the water and the far shore answer.
    let mut air = p.make_device(EffectKind::OpenAir);
    air.params[crate::dsp::fx::open_air::PLACE] = 3.0;
    air.params[crate::dsp::fx::open_air::DISTANCE] = 0.35;
    air.params[crate::dsp::fx::open_air::MIX] = 0.6;
    p.tracks[flute].devices.push(air);
    let tune: &[(u8, f64, f64)] = &[
        (76, 0.0, 1.5),
        (74, 1.5, 0.5),
        (72, 2.0, 2.0),
        (69, 4.0, 3.0),
        (72, 8.0, 1.0),
        (74, 9.0, 1.0),
        (76, 10.0, 2.0),
        (79, 12.0, 1.5),
        (76, 13.5, 0.5),
        (74, 14.0, 2.0),
        (76, 16.0, 3.0),
        (74, 19.0, 0.5),
        (72, 19.5, 0.5),
        (69, 20.0, 2.0),
        (67, 22.0, 2.0),
        (69, 24.0, 1.0),
        (72, 25.0, 1.0),
        (74, 26.0, 1.0),
        (76, 27.0, 1.0),
        (69, 28.0, 3.5),
    ];
    let melody: Vec<Note> = tune.iter().map(|&(k, t, l)| n(k, t, l * 0.95, if l >= 2.0 { 0.8 } else { 0.7 })).collect();
    set_clip(&mut p, flute, 2, "Melody", 32.0, melody.clone());
    let low: Vec<Note> = melody.iter().map(|m| Note { pitch: m.pitch - 12, velocity: m.velocity * 0.85, ..m.clone() }).collect();
    set_clip(&mut p, flute, 3, "Melody Low", 32.0, low);

    p.loop_end = 32.0;
    p
}

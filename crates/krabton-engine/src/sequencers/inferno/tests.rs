use super::*;
use crate::band::{BandNow, PartPlay, Prog};
use crate::midi_fx::{MidiFxKind, MidiFxState};
use crate::params::defaults;
use crate::scale::ScaleKind;

const SR: f64 = 48_000.0;
const K: MidiFxKind = MidiFxKind::Inferno;

/// What a run played: (frame, on, pitch, velocity), and the wob and bend
/// after each block (frame at its end).
#[derive(Default)]
struct Run {
    evs: Vec<(u64, bool, u8, f32)>,
    outs: Vec<(u64, Option<f32>, f32)>,
}

impl Run {
    fn ons(&self) -> Vec<(u64, u8)> {
        self.evs.iter().filter(|e| e.1).map(|e| (e.0, e.2)).collect()
    }
}

fn a_minor() -> Scale {
    Scale::new(9, ScaleKind::Minor)
}

/// Runs `beats` beats at `bpm` in blocks of `block` frames. `band` gives
/// the band for a block's beat; `each` can change things before a block
/// (its index, the beat, the params, the data) and say whether it plays.
#[allow(clippy::too_many_arguments)]
fn run_full(p: &[f32], data: &[u8], bpm: f64, beats: f64, block: usize, scale: Scale, band: &dyn Fn(f64) -> BandNow, each: &mut dyn FnMut(u64, f64, &mut Vec<f32>, &mut Vec<u8>) -> bool, input: &dyn Fn(u64) -> Vec<MidiEvent>) -> Run {
    let spb = SR * 60.0 / bpm;
    let mut st = MidiFxState::new(K);
    let mut run = Run::default();
    let total = (beats * spb) as u64;
    let mut p = p.to_vec();
    let mut data = data.to_vec();
    let mut frame = 0u64;
    let mut b = 0u64;
    let mut beat = 0.0;
    while frame < total {
        let playing = each(b, beat, &mut p, &mut data);
        let len = block.min((total - frame) as usize);
        let mut bn = band(beat);
        bn.beat = beat;
        let ctx = MidiCtx { now: frame, len, samples_per_beat: spb, scale, scale_on: true, beat, playing, sr: SR as f32, drum_notes: 0, lead: 0.0, touch: [0.0; 5], band: bn };
        let mut out = Vec::new();
        st.process(K, &p, &data, &input(b), &mut out, &ctx);
        out.sort_by_key(|e| (e.offset, e.on));
        for e in &out {
            run.evs.push((frame + e.offset as u64, e.on, e.pitch, e.vel));
        }
        let (w, bend) = st.inferno_out(&p).unwrap();
        run.outs.push((frame + len as u64, w, bend));
        frame += len as u64;
        beat += len as f64 / spb;
        b += 1;
    }
    // Then the transport stops.
    let ctx = MidiCtx { now: frame, len: block, samples_per_beat: spb, scale, scale_on: true, beat, playing: false, sr: SR as f32, drum_notes: 0, lead: 0.0, touch: [0.0; 5], band: BandNow::default() };
    let mut out = Vec::new();
    st.process(K, &p, &data, &[], &mut out, &ctx);
    run.evs.extend(out.iter().map(|e| (frame + e.offset as u64, e.on, e.pitch, e.vel)));
    run
}

fn run(p: &[f32], data: &[u8], bpm: f64, beats: f64) -> Run {
    run_full(p, data, bpm, beats, 256, a_minor(), &|_| BandNow::default(), &mut |_, _, _, _| true, &|_| Vec::new())
}

/// A pattern A from `steps` (index, step), `len` long.
fn pat(steps: &[(usize, Step)], len: usize) -> Vec<u8> {
    let mut d = empty();
    for &(i, s) in steps {
        set_step(&mut d, 0, i, s);
    }
    set_length(&mut d, 0, len);
    d
}

fn all_on(len: usize) -> Vec<u8> {
    pat(&(0..len).map(|i| (i, Step::note(0))).collect::<Vec<_>>(), len)
}

/// Every note-on ends, and never more than `most` sound at once.
fn balanced(run: &Run, most: usize) {
    let mut count = [0i32; 128];
    let mut max = 0;
    for e in &run.evs {
        let c = &mut count[e.2 as usize];
        *c += if e.1 { 1 } else { -1 };
        assert!(*c >= 0, "an off without its on at {}: {}", e.0, e.2);
        max = max.max(count.iter().filter(|c| **c > 0).count());
    }
    assert!(count.iter().all(|c| *c == 0), "notes left hanging: {:?}", count.iter().enumerate().filter(|c| *c.1 != 0).collect::<Vec<_>>());
    assert!(max <= most, "{max} notes at once");
}

#[test]
fn steps_land_on_their_grid_at_every_tempo_and_division() {
    for (bpm, div) in [(90.0, 2), (128.0, 2), (174.0, 2), (120.0, 0), (120.0, 1), (140.0, 3), (100.0, 4), (110.0, 5)] {
        let mut p = defaults(SPECS);
        p[DIVISION] = div as f32;
        let r = run(&p, &all_on(16), bpm, 4.0);
        let spb = SR * 60.0 / bpm;
        let step = DIVISION_BEATS[div] * spb;
        let ons = r.ons();
        let n = (4.0 / DIVISION_BEATS[div]).round() as usize;
        assert_eq!(ons.len(), n, "{bpm} BPM, {}", DIVISIONS[div]);
        for (k, on) in ons.iter().enumerate() {
            assert!((on.0 as f64 - k as f64 * step).abs() <= 1.0, "{bpm} BPM {}: step {k} at {} not {}", DIVISIONS[div], on.0, k as f64 * step);
        }
        // Each note lasts its gate (half a step).
        let offs: Vec<u64> = r.evs.iter().filter(|e| !e.1).map(|e| e.0).collect();
        assert!((offs[0] as f64 - step * 0.5).abs() <= 2.0, "{} vs {}", offs[0], step * 0.5);
    }
}

#[test]
fn swing_grooves_and_nudges_move_the_steps() {
    let spb = SR * 60.0 / 120.0;
    let step = 0.25 * spb;
    let mut p = defaults(SPECS);
    p[SWING] = 0.5;
    let ons = run(&p, &all_on(16), 120.0, 1.0).ons();
    // Swing half way: every second sixteenth a quarter of a step late.
    assert!((ons[1].0 as f64 - 1.25 * step).abs() <= 1.0, "{:?}", ons);
    assert!((ons[2].0 as f64 - 2.0 * step).abs() <= 1.0);
    p[GROOVE] = 2.0; // MPC 58
    let ons = run(&p, &all_on(16), 120.0, 1.0).ons();
    assert!((ons[1].0 as f64 - 1.16 * step).abs() <= 1.0, "{:?}", ons);
    // A nudge earlier plays before its step (the step after is planned ahead).
    let mut d = all_on(16);
    set_step(&mut d, 0, 4, Step { nudge: -20, ..Step::note(0) });
    let ons = run(&defaults(SPECS), &d, 120.0, 2.0).ons();
    assert!((ons[4].0 as f64 - 3.8 * step).abs() <= 1.0, "{:?}", ons);
    assert!((ons[5].0 as f64 - 5.0 * step).abs() <= 1.0);
}

#[test]
fn slides_overlap_for_a_mono_synth_and_bend_with_bend() {
    let d = pat(&[(0, Step { slide: true, ..Step::note(0) }), (1, Step::note(2))], 4);
    let p = defaults(SPECS);
    let r = run(&p, &d, 120.0, 1.0);
    let spb = SR * 60.0 / 120.0;
    let step = (0.25 * spb) as u64;
    let on1 = r.evs.iter().find(|e| e.1 && e.0 >= step - 1).expect("second note");
    let off0 = r.evs.iter().find(|e| !e.1 && e.2 != on1.2).expect("first note ends");
    assert!(off0.0 > on1.0, "legato: the first ends after the second starts ({} vs {})", off0.0, on1.0);
    assert!(off0.0 - on1.0 < step / 4, "a little overlap only");
    balanced(&r, 2);
    // Bend: the first ends as the second starts, the second bends from the first.
    let mut p = defaults(SPECS);
    p[GLIDE] = 1.0;
    let r = run(&p, &d, 120.0, 1.0);
    let on1 = r.evs.iter().find(|e| e.1 && e.0 >= step - 1).unwrap();
    let off0 = r.evs.iter().find(|e| !e.1 && e.2 != on1.2).unwrap();
    assert_eq!(off0.0, on1.0);
    let i = r.evs.iter().position(|e| e == off0).unwrap();
    let j = r.evs.iter().position(|e| e == on1).unwrap();
    assert!(i < j, "the note-off comes first");
    let bent: Vec<f32> = r.outs.iter().filter(|o| o.0 > on1.0).map(|o| o.2).collect();
    // C up to E in A minor: from four semitones below, home within the glide.
    assert!(bent[0] < -0.5 && bent[0] >= -4.0, "{:?}", &bent[..4]);
    assert!(bent.windows(2).all(|w| w[1] >= w[0] - 1e-6));
    assert_eq!(*bent.last().unwrap(), 0.0);
    balanced(&r, 2);
}

#[test]
fn ties_hold_and_the_same_note_slides_without_an_attack() {
    let d = pat(&[(0, Step::note(0)), (1, Step { tie: true, ..Step::note(0) }), (2, Step { tie: true, ..Step::note(0) }), (4, Step { slide: true, ..Step::note(3) }), (5, Step::note(3))], 8);
    let r = run(&defaults(SPECS), &d, 120.0, 2.0);
    let ons = r.ons();
    assert_eq!(ons.len(), 2, "{ons:?}");
    let step = 0.25 * SR * 60.0 / 120.0;
    // The tied note ends in its third step, after half of it.
    let off = r.evs.iter().find(|e| !e.1).unwrap();
    assert!((off.0 as f64 - 2.5 * step).abs() <= 2.0, "{} vs {}", off.0, 2.5 * step);
    balanced(&r, 2);
}

#[test]
fn ratchets_roll_and_curve() {
    let d = pat(&[(0, Step { ratchet: 4, ..Step::note(0) })], 4);
    let ons = run(&defaults(SPECS), &d, 120.0, 1.0).ons();
    let step = 0.25 * SR * 60.0 / 120.0;
    assert_eq!(ons.len(), 4);
    for (i, o) in ons.iter().enumerate() {
        assert!((o.0 as f64 - i as f64 * step / 4.0).abs() <= 1.0, "{ons:?}");
    }
    let d = pat(&[(0, Step { ratchet: 6, curve: 1, ..Step::note(0) })], 4);
    let r = run(&defaults(SPECS), &d, 120.0, 1.0);
    let ons = r.ons();
    assert_eq!(ons.len(), 6);
    let gaps: Vec<u64> = ons.windows(2).map(|w| w[1].0 - w[0].0).collect();
    assert!(gaps.windows(2).all(|g| g[1] <= g[0]), "speeding up: {gaps:?}");
    balanced(&r, 1);
}

#[test]
fn chance_and_conditions_are_the_same_with_the_same_seed() {
    let d = pat(&(0..16).map(|i| (i, Step { chance: 50, ..Step::note((i % 5) as u8) })).collect::<Vec<_>>(), 16);
    let mut p = defaults(SPECS);
    let a = run(&p, &d, 120.0, 16.0).ons();
    let b = run(&p, &d, 120.0, 16.0).ons();
    assert_eq!(a, b);
    assert!(a.len() > 16 && a.len() < 48, "about half: {}", a.len());
    p[SEED] = 2.0;
    assert_ne!(run(&p, &d, 120.0, 16.0).ons(), a);
    // 1:2 plays the first time round of every two; Fill only in a fill.
    let d = pat(&[(0, Step { condition: 1, ..Step::note(0) }), (8, Step { condition: 10, ..Step::note(4) })], 16);
    let p = defaults(SPECS);
    let ons = run(&p, &d, 120.0, 16.0).ons();
    let bars: Vec<u64> = ons.iter().map(|o| o.0 / (4 * 24_000)).collect();
    assert_eq!(bars, vec![0, 2]);
    let mut p = defaults(SPECS);
    p[FILL] = 1.0;
    let ons = run(&p, &d, 120.0, 4.0).ons();
    assert!(ons.iter().any(|o| o.1 == pitch_of(&Step::note(4), 36, false, &a_minor()) as u8), "the fill's note: {ons:?}");
}

#[test]
fn no_note_is_left_hanging_whatever_happens() {
    // Busy lines in every genre, patterns switched and rewritten while
    // they play, Burn and Fill, a loop back, odd block sizes, then a stop.
    for genre in 0..GENRES.len() {
        let mut d = empty();
        for pat in 0..3 {
            let g = Ignite { genre, density: 0.9, complexity: 1.0, melody: 0.8, seed: 7 + pat as u32, scale_len: 7, chromatic: false, chords: None, kick: None, kick_mode: 0 };
            let (steps, len) = ignite(&g);
            set_pattern(&mut d, pat, &steps);
            set_length(&mut d, pat, len);
        }
        let mut p = defaults(SPECS);
        p[GLIDE] = (genre % 2) as f32;
        p[GATE] = 1.5;
        p[DIRECTION] = (genre % 5) as f32;
        let mut stopped = false;
        let r = run_full(
            &p,
            &d,
            100.0 + genre as f64 * 4.0,
            64.0,
            97 + genre * 13,
            a_minor(),
            &|_| BandNow::default(),
            &mut |b, beat, p, d| {
                p[PATTERN] = ((beat / 12.0) as usize % 3) as f32;
                p[BURN] = ((beat / 6.0) as usize % 3 == 1) as u8 as f32;
                p[FILL] = ((beat / 10.0) as usize % 4 == 3) as u8 as f32;
                if b % 400 == 399 {
                    let (s, _) = ignite(&Ignite { genre: (genre + 3) % GENRES.len(), density: 0.8, complexity: 0.9, melody: 0.5, seed: b as u32, scale_len: 7, chromatic: false, chords: None, kick: None, kick_mode: 0 });
                    set_pattern(d, 1, &s);
                }
                if beat > 60.0 {
                    stopped = true;
                }
                !stopped
            },
            &|_| Vec::new(),
        );
        assert!(r.ons().len() > 40, "{}: {}", GENRES[genre], r.ons().len());
        balanced(&r, 6);
    }
}

#[test]
fn a_jump_in_the_song_lets_go_cleanly() {
    let d = pat(&[(0, Step { slide: true, ..Step::note(0) }), (1, Step { tie: true, slide: true, ..Step::note(0) }), (2, Step { ratchet: 3, ..Step::note(2) }), (3, Step { gate: 16, ..Step::note(4) })], 4);
    let p = defaults(SPECS);
    let spb = SR * 60.0 / 120.0;
    let mut st = MidiFxState::new(K);
    let mut evs = Vec::new();
    let mut frame = 0u64;
    // Two beats, then back to beat 0 (a loop), twice; then stop.
    for round in 0..3 {
        let mut beat = 0.0;
        while beat < 2.0 {
            let ctx = MidiCtx { now: frame, len: 256, samples_per_beat: spb, scale: a_minor(), scale_on: true, beat, playing: round < 2, sr: SR as f32, drum_notes: 0, lead: 0.0, touch: [0.0; 5], band: BandNow::default() };
            let mut out = Vec::new();
            st.process(K, &p, &d, &[], &mut out, &ctx);
            out.sort_by_key(|e| (e.offset, e.on));
            evs.extend(out.iter().map(|e| (frame + e.offset as u64, e.on, e.pitch, e.vel)));
            frame += 256;
            beat += 256.0 / spb;
        }
    }
    balanced(&Run { evs, outs: Vec::new() }, 3);
}

/// The pitches of a pattern's notes as it would play them (A minor, base C2).
fn in_key(steps: &[Step], len: usize) -> bool {
    steps[..len].iter().filter(|s| s.sounds()).all(|s| a_minor().contains(pitch_of(s, 36, false, &a_minor()).clamp(0, 127) as u8))
}

#[test]
fn ignite_writes_each_genre_in_key_with_its_shape() {
    for genre in 0..GENRES.len() {
        let mut notes = 0;
        let mut sparse = 0;
        for seed in 1..9 {
            let g = |density: f32| Ignite { genre, density, complexity: 0.4, melody: 0.5, seed, scale_len: 7, chromatic: false, chords: None, kick: None, kick_mode: 0 };
            let (steps, len) = ignite(&g(0.6));
            assert!(len == 16 || len == 32, "{}", GENRES[genre]);
            assert!(in_key(&steps, len), "{}", GENRES[genre]);
            let attacks = steps[..len].iter().filter(|s| s.on && !s.tie).count();
            assert!(attacks >= 2 && attacks <= len, "{}: {attacks} notes in {len}", GENRES[genre]);
            assert!(steps[len..].iter().all(|s| !s.on));
            notes += attacks;
            let (low, _) = ignite(&g(0.1));
            sparse += low[..len].iter().filter(|s| s.on && !s.tie).count();
            // The same seed writes the same line.
            assert_eq!(ignite(&g(0.6)).0, steps);
        }
        assert!(sparse < notes, "{}: denser with Density", GENRES[genre]);
    }
    let shape = |genre: usize| {
        let g = Ignite { genre, density: 0.7, complexity: 0.3, melody: 0.3, seed: 3, scale_len: 7, chromatic: false, chords: None, kick: None, kick_mode: 0 };
        ignite(&g)
    };
    // Psytrance stays off the kick; disco and synthwave jump octaves on the off-beats.
    let (psy, len) = shape(15);
    assert!(psy[..len].iter().enumerate().all(|(i, s)| !s.on || i % 4 != 0));
    for genre in [11, 13] {
        let (s, len) = shape(genre);
        assert!(s[..len].iter().enumerate().any(|(i, s)| s.on && s.jump && i % 4 == 2), "{}", GENRES[genre]);
    }
    // Dubstep wobs and holds; trap holds and glides.
    let (dub, len) = shape(4);
    assert!(dub[..len].iter().any(|s| s.wob > 0) && dub[..len].iter().any(|s| s.tie));
    let (trap, len) = shape(8);
    assert!(trap[..len].iter().any(|s| s.tie));
    // Avoiding the kick: nothing on a four-on-the-floor kick.
    let g = Ignite { genre: 0, density: 1.0, complexity: 0.0, melody: 0.3, seed: 5, scale_len: 7, chromatic: false, chords: None, kick: Some(0x1111), kick_mode: 1 };
    let (s, len) = ignite(&g);
    assert!(s[..len].iter().enumerate().all(|(i, s)| !(s.on && !s.tie) || i % 4 != 0));
}

#[test]
fn ignite_on_the_bands_chords_writes_their_notes() {
    let mut chords = [(1u8, 0u8); STEPS];
    for c in chords.iter_mut().skip(8) {
        *c = (5, 4); // a power chord: root and fifth only
    }
    let g = Ignite { genre: 10, density: 0.9, complexity: 0.2, melody: 1.0, seed: 4, scale_len: 7, chromatic: false, chords: Some(chords), kick: None, kick_mode: 0 };
    let (s, len) = ignite(&g);
    // Counted from the chord's root: a power chord has only roots, fifths, octaves.
    for (i, st) in s[..len.min(16)].iter().enumerate().filter(|(i, s)| *i >= 8 && s.on && !s.tie) {
        assert!(matches!(st.note, 0 | 4 | 7), "step {i}: {}", st.note);
    }
}

#[test]
fn mutate_and_evolve_leave_the_locked_steps() {
    let (mut steps, len) = ignite(&Ignite { genre: 0, density: 0.7, complexity: 0.4, melody: 0.5, seed: 9, scale_len: 7, chromatic: false, chords: None, kick: None, kick_mode: 0 });
    for s in steps.iter_mut().take(8) {
        s.lock = true;
    }
    let before = steps;
    mutate(&mut steps, len, 1.0, 3, 7);
    assert_eq!(steps[..8], before[..8]);
    assert_ne!(steps, before);
    // Evolve, as it plays: bar 0 as written, later bars changed, the locked steps kept.
    let mut d = empty();
    let line: Vec<(usize, Step)> = (0..16).map(|i| (i, Step { lock: i < 4, ..Step::note([0u8, 2, 4, 3][i % 4]) })).collect();
    for &(i, s) in &line {
        set_step(&mut d, 0, i, s);
    }
    let mut p = defaults(SPECS);
    p[EVOLVE] = 1.0;
    p[EVOLVE_AMOUNT] = 1.0;
    let ons = run(&p, &d, 120.0, 16.0).ons();
    let bar = |b: u64| -> Vec<(u64, u8)> { ons.iter().filter(|o| o.0 / 96_000 == b).map(|o| (o.0 % 96_000, o.1)).collect() };
    let plain = run(&defaults(SPECS), &d, 120.0, 4.0).ons();
    assert_eq!(bar(0), plain);
    assert!((1..4).any(|b| bar(b) != bar(0)), "it evolves");
    for b in 0..4 {
        let first: Vec<_> = bar(b).into_iter().filter(|o| o.0 < 4 * 6000).collect();
        assert_eq!(first, plain.iter().copied().filter(|o| o.0 < 4 * 6000).collect::<Vec<_>>(), "bar {b}: the locked steps");
    }
    // What the face shows is what it plays.
    assert_eq!(evolved(&d, 0, 2, 1.0, 1, 7), evolved(&d, 0, 2, 1.0, 1, 7));
}

fn band_on(chords: &[(u8, u8)], key: Scale) -> BandNow {
    let mut c = [(0u8, 0u8); 8];
    c[..chords.len()].copy_from_slice(chords);
    let prog = Prog { length: 2, count: chords.len() as u8, chords: c };
    BandNow { on: true, key, prog, next: prog, from: 0.0, until: f64::INFINITY, ..BandNow::default() }
}

#[test]
fn it_follows_the_bands_chords_and_the_key() {
    // A on every step (base A2), the band on A minor's i and v (a bar each).
    let d = all_on(16);
    let mut p = defaults(SPECS);
    p[BASE] = 45.0;
    let key = a_minor();
    let band = |_: f64| band_on(&[(1, 0), (5, 0)], key);
    let follow = |mode: f32, on: bool| {
        let mut p = p.clone();
        p[BAND] = on as u8 as f32;
        p[BAND + 1] = mode;
        run_full(&p, &d, 120.0, 8.0, 256, key, &band, &mut |_, _, _, _| true, &|_| Vec::new()).ons()
    };
    let at = |ons: &[(u64, u8)], beat: f64| ons.iter().find(|o| (o.0 as f64 - beat * 24_000.0).abs() < 2.0).map(|o| o.1);
    // Root: on the v the line moves to E, down a fourth.
    let root = follow(0.0, true);
    assert_eq!(at(&root, 0.0), Some(45));
    assert_eq!(at(&root, 4.0), Some(40));
    // Chord: on the chord's notes (E G B on the v).
    let mut p2 = p.clone();
    p2[BAND] = 1.0;
    p2[BAND + 1] = 1.0;
    let d2 = pat(&(0..16).map(|i| (i, Step::note((i % 3) as u8))).collect::<Vec<_>>(), 16);
    let chord = run_full(&p2, &d2, 120.0, 8.0, 256, key, &band, &mut |_, _, _, _| true, &|_| Vec::new()).ons();
    assert!(chord.iter().filter(|o| o.0 >= 96_000).all(|o| [4, 7, 11].contains(&(o.1 % 12))), "{chord:?}");
    // The link off: as written.
    let off = follow(0.0, false);
    assert!(off.iter().all(|o| o.1 == 45));
    // The key: base C3 in C minor plays C, moved with the held key (Transpose).
    let mut p = defaults(SPECS);
    p[BASE] = 48.0;
    let c_minor = Scale::new(0, ScaleKind::Minor);
    let ons = run_full(&p, &d, 120.0, 1.0, 256, c_minor, &|_| BandNow::default(), &mut |_, _, _, _| true, &|_| Vec::new()).ons();
    assert!(ons.iter().all(|o| o.1 == 48));
    p[26] = 1.0; // Transpose
    let held = run_full(&p, &d, 120.0, 2.0, 256, c_minor, &|_| BandNow::default(), &mut |_, _, _, _| true, &|b| if b == 0 { vec![MidiEvent::on(0, 51, 0.8)] } else { Vec::new() }).ons();
    assert!(held.iter().all(|o| o.1 == 51), "a scale step up (Eb): {held:?}");
}

/// A band of two parts (beats 0-8 and 8-16), each with `play`.
fn parts(beat: f64, play: [PartPlay; 2]) -> BandNow {
    let mut b = band_on(&[(1, 0)], a_minor());
    b.parts = true;
    let part = ((beat / 8.0).floor() as i64).rem_euclid(2) as u8;
    let from = (beat / 8.0).floor() * 8.0;
    b.part = part;
    b.next_part = 1 - part;
    b.from = from;
    b.until = from + 8.0;
    b.prog.count = 2;
    b.next = b.prog;
    b.play = play[part as usize];
    b.next_play = play[1 - part as usize];
    b.ends = true;
    b
}

#[test]
fn each_song_part_plays_its_own_pattern_from_its_first_step() {
    let mut d = empty();
    for i in 0..16 {
        set_step(&mut d, 0, i, Step::note(0));
        set_step(&mut d, 1, i, Step::note(4));
    }
    set_part_pattern(&mut d, 0, Some(0));
    set_part_pattern(&mut d, 1, Some(1));
    let mut p = defaults(SPECS);
    p[BAND] = 1.0;
    let band = |beat: f64| parts(beat, [PartPlay::default(); 2]);
    let ons = run_full(&p, &d, 120.0, 16.0, 200, a_minor(), &band, &mut |_, _, _, _| true, &|_| Vec::new()).ons();
    let a = pitch_of(&Step::note(0), 36, false, &a_minor()) as u8;
    let b = pitch_of(&Step::note(4), 36, false, &a_minor()) as u8;
    let switch = 8 * 24_000;
    assert!(ons.iter().filter(|o| o.0 < switch).all(|o| o.1 == a));
    assert!(ons.iter().filter(|o| o.0 >= switch).all(|o| o.1 == b));
    let first = ons.iter().find(|o| o.1 == b).unwrap().0;
    assert!(first.abs_diff(switch) <= 1, "exactly where the part changes: {first}");
}

#[test]
fn the_parts_energy_fill_build_and_break_play_hotter_or_quieter() {
    let d = pat(&(0..16).map(|i| (i, Step { accent: i == 0, ..Step::note(0) })).collect::<Vec<_>>(), 16);
    let mut p = defaults(SPECS);
    p[BAND] = 1.0;
    let count = |play: PartPlay, from: f64, to: f64| {
        let band = move |beat: f64| parts(beat, [play; 2]);
        let ons = run_full(&p, &d, 120.0, 8.0, 256, a_minor(), &band, &mut |_, _, _, _| true, &|_| Vec::new()).ons();
        ons.iter().filter(|o| o.0 as f64 >= from * 24_000.0 && (o.0 as f64) < to * 24_000.0).count()
    };
    let mid = count(PartPlay::default(), 0.0, 8.0);
    assert_eq!(mid, 32, "as written");
    let hot = count(PartPlay { energy: 1.0, ..PartPlay::default() }, 0.0, 8.0);
    let cold = count(PartPlay { energy: 0.0, ..PartPlay::default() }, 0.0, 8.0);
    assert!(hot > mid && cold < mid && cold >= 2, "{cold} {mid} {hot}");
    // A fill rolls in the part's last beat; a build heats its last bar.
    let fill = count(PartPlay { fill: true, ..PartPlay::default() }, 7.0, 8.0);
    assert!(fill > count(PartPlay::default(), 7.0, 8.0));
    let build = count(PartPlay { build: 1, ..PartPlay::default() }, 4.0, 8.0);
    assert!(build > count(PartPlay::default(), 4.0, 8.0));
    // A break: nothing in the last beat.
    assert_eq!(count(PartPlay { brk: 1, ..PartPlay::default() }, 7.0, 8.0), 0);
    assert!(count(PartPlay { brk: 1, ..PartPlay::default() }, 0.0, 7.0) > 0);
}

#[test]
fn the_wob_wobbles_the_mod_wheel_while_its_note_sounds() {
    let d = pat(&[(0, Step { wob: 3, modv: 127, gate: 16, slide: true, ..Step::note(0) }), (1, Step { tie: true, wob: 3, slide: true, ..Step::note(0) }), (2, Step { tie: true, wob: 3, slide: true, ..Step::note(0) }), (3, Step { tie: true, wob: 3, ..Step::note(0) })], 16);
    let r = run(&defaults(SPECS), &d, 120.0, 1.0);
    let w: Vec<f32> = r.outs.iter().map(|o| o.1.unwrap()).collect();
    let (lo, hi) = w.iter().fold((1.0f32, 0.0f32), |(a, b), v| (a.min(*v), b.max(*v)));
    assert!(lo < 0.1 && hi > 0.9, "a wave over the beat: {lo} {hi}");
    let mut p = defaults(SPECS);
    p[WOB_OUT] = 0.0;
    assert!(run(&p, &d, 120.0, 1.0).outs.iter().all(|o| o.1.is_none()));
}

#[test]
fn presets_cover_the_genres_and_play() {
    assert!(PRESETS.len() >= 30, "{}", PRESETS.len());
    let specs = K.specs();
    for g in GENRES {
        assert!(PRESETS.iter().any(|pr| pr.set.iter().any(|(n, v)| *n == "Genre" && matches!(v, V::C(x) if x == g))), "{g}");
    }
    for pr in PRESETS {
        let data = (pr.data.unwrap())();
        assert!(is_current(&data), "{}", pr.name);
        let p = crate::presets::apply(specs, pr);
        let r = run(&p, &data, 120.0, 8.0);
        assert!(r.ons().len() >= 3, "{}", pr.name);
        balanced(&r, 6);
    }
    // Their pairs are presets the instruments have.
    for g in 0..GENRES.len() {
        let (inst, preset) = feel(g).pair;
        let names: &[&str] = match inst {
            "Bassline" => crate::dsp::bassline::preset_names(),
            "Analog" => crate::dsp::analog::preset_names(),
            "Crimson" => crate::dsp::crimson::preset_names(),
            "Silicon" => crate::dsp::silicon::preset_names(),
            "Synth" => crate::dsp::synth::preset_names(),
            "Mono" => crate::dsp::mono101::preset_names(),
            _ => panic!("{inst}"),
        };
        assert!(names.contains(&preset), "{inst} has no {preset}");
    }
}

#[test]
fn old_and_odd_data_play_safely_and_it_renders_to_a_clip() {
    for data in [Vec::new(), vec![1, 2, 3], vec![0xff; DATA_BYTES]] {
        let r = run(&defaults(SPECS), &data, 120.0, 4.0);
        balanced(&r, 4);
    }
    let notes = crate::midi_fx::render_notes(K, &defaults(SPECS), &K.default_data(), 16.0, 128.0, a_minor(), true, 0);
    assert!(notes.len() > 8);
    assert!(notes.iter().all(|n| a_minor().contains(n.pitch)));
    // The face's word comes back as it went.
    let mut st = InfernoState { now: (5, 2), generation: 3, heat_now: 0.5, sounding: Some(40), hits: 9, last_hit: 7 | (TRIG_ACCENT as u64) << 8, prev_hit: 6, wob_out: 0.25, bend_out: -2.0, ..InfernoState::default() };
    let live = Live::unpack(&st.visual(true));
    assert_eq!((live.step, live.pattern, live.generation, live.sounding, live.hit_step, live.prev_hit, live.hits), (5, 2, 3, Some(40), 7, Some(6), 9));
    assert!(live.hit_accent && !live.hit_slide && live.playing);
    assert!((live.wob - 0.25).abs() < 1e-3 && (live.bend + 2.0).abs() < 0.02);
    st.sounding = None;
    assert_eq!(Live::unpack(&st.visual(false)).sounding, None);
}

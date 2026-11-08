//! The Video's view, laid out as a VJ desk. Along the top the deck: the
//! clips of the bank (click one to play it, drag to put it elsewhere) and
//! the Look pads, each under a line saying when a click starts it and how
//! the change goes. In the middle the picture with its timeline under it
//! (the whole film as a strip of its frames, the parts that play lit: drag
//! to keep a part, its edges to trim it, Mark In and Mark Out while it
//! plays) and what the music does. On the right a side with four tabs:
//! *Clip* (how the clip edited plays), *Look* (the effects of the Look
//! edited, each a card with the links that move it), *Triggers* (what
//! changes the clips, the Looks, the slices and the parts) and *Output*
//! (the screen for a projector, Record Video and Export Video).

use crate::app::{App, ExportSource, View};
use crate::theme::*;
use crate::video::{export, ffmpeg, media, strip, ytdl, VideoTab};
use crate::widgets::*;
use super::vj;
use egui::{pos2, vec2, Align2, Color32, Context, Id, Key, Rect, RichText, Sense, Stroke, Ui, UiBuilder};
use krabton_engine::milk::{self, MilkParam};
use krabton_engine::video::{self as kv, Effect, HitTrigger, Launch, PlayMode, Source, Transition, VideoLink};
use std::path::PathBuf;

/// A choice that changes with a click (lit when `on`).
fn choice(ui: &mut Ui, label: &str, on: bool, tip: &str) -> egui::Response {
    let b = egui::Button::new(RichText::new(label).color(if on { TEXT_DARK } else { TEXT })).fill(if on { ACCENT } else { PANEL3 });
    ui.add(b).on_hover_text(tip)
}

/// A list to pick one of `options` from (for choices with more than a
/// few; lengths and counts stay buttons to click). Whether it changed.
fn dropdown<T: PartialEq + Clone>(ui: &mut Ui, id: impl std::hash::Hash, width: f32, value: &mut T, options: &[(T, String)], tip: &str) -> bool {
    vj::pick(ui, id, width, value, options, tip)
}

/// A track to pick (None first, named `none`). Whether it changed.
fn track_dropdown(ui: &mut Ui, id: impl std::hash::Hash, value: &mut Option<u64>, tracks: &[(u64, String)], none: &str, tip: &str) -> bool {
    let mut options: Vec<(Option<u64>, String)> = vec![(None, none.to_string())];
    options.extend(tracks.iter().map(|(id, n)| (Some(*id), n.clone())));
    if value.is_some_and(|v| !tracks.iter().any(|t| t.0 == v)) {
        options.push((*value, "(a track gone)".to_string()));
    }
    dropdown(ui, id, 96.0, value, &options, tip)
}

/// A Look to pick (None first, named `none`).
fn look_dropdown(ui: &mut Ui, id: impl std::hash::Hash, value: &mut Option<usize>, names: &[String], none: &str, tip: &str) -> bool {
    let mut options: Vec<(Option<usize>, String)> = vec![(None, none.to_string())];
    options.extend(names.iter().enumerate().map(|(k, n)| (Some(k), n.clone())));
    dropdown(ui, id, 110.0, value, &options, tip)
}

/// A Look's name on its pad and in the lists (its number when it has none).
fn look_label(k: usize, l: &kv::VideoLook) -> String {
    if l.name.trim().is_empty() {
        format!("Look {}", k + 1)
    } else {
        l.name.clone()
    }
}

/// `s` cut to `n` characters (with an ellipsis).
fn short(s: &str, n: usize) -> String {
    if s.chars().count() > n {
        format!("{}…", s.chars().take(n.saturating_sub(1)).collect::<String>())
    } else {
        s.to_string()
    }
}

fn launch_name(l: Launch) -> &'static str {
    match l {
        Launch::Now => "Now",
        Launch::Beat => "On the beat",
        Launch::Bar => "On the bar",
    }
}

/// The Looks renumbered (one put in or taken out): Look `i` is now `place(i)`
/// wherever a clip, a part or the sequence names it.
fn renumber_looks(set: &mut kv::VideoSet, place: impl Fn(usize) -> usize) {
    let n = set.looks.len();
    let fix = |l: &mut Option<usize>| *l = l.map(&place).filter(|k| *k < n);
    for c in &mut set.clips {
        fix(&mut c.look);
        for l in &mut c.part_looks {
            fix(l);
        }
    }
    for s in &mut set.look_seq.steps {
        *s = s.map(|k| place(k as usize) as u8).filter(|k| (*k as usize) < n);
    }
}

/// A length in beats for the hit triggers' gap and grid (0: none).
fn beats_name(b: f32) -> String {
    match b {
        b if b <= 0.0 => "Off".to_string(),
        b if b < 1.0 => format!("1/{}", (4.0 / b).round()),
        b => format!("{b}"),
    }
}

/// How wide the labels on the left of the side's rows are, so the
/// controls after them line up.
const LABEL_W: f32 = 84.0;

/// A row of the side: its label in the dim colour on the left, the
/// controls after it.
fn row<R>(ui: &mut Ui, label: &str, add: impl FnOnce(&mut Ui) -> R) -> R {
    ui.horizontal(|ui| {
        let (r, _) = ui.allocate_exact_size(vec2(LABEL_W, 20.0), Sense::hover());
        ui.painter().with_clip_rect(r).text(pos2(r.left(), r.center().y), Align2::LEFT_CENTER, label, small(), TEXT_DIM);
        add(ui)
    })
    .inner
}

/// A short line saying what something means, in the dim colour, under
/// the controls of a row (where they start).
fn hint(_ui: &mut Ui, _s: &str) {}

/// A short line saying what something means, in the dim colour.
fn note(ui: &mut Ui, s: &str) {
    ui.add(egui::Label::new(RichText::new(s).small().color(TEXT_DIM)).wrap());
}

/// A heading for a part of the side: small capitals, a line under it.
fn section(ui: &mut Ui, title: &str) {
    ui.add_space(12.0);
    ui.label(RichText::new(title.to_uppercase()).small().strong().color(TEXT_DIM));
    let r = ui.available_rect_before_wrap();
    ui.painter().line_segment([pos2(r.left(), r.top() + 1.0), pos2(r.right(), r.top() + 1.0)], Stroke::new(1.0_f32, LINE_SOFT));
    ui.add_space(5.0);
}

/// The name of a group of the deck, in small capitals.
fn deck_title(ui: &mut Ui, title: &str) {
    ui.label(RichText::new(title).small().strong().color(TEXT));
    ui.add_space(4.0);
}

/// A small on/off switch. Clicked: it turns.
fn switch(ui: &mut Ui, on: &mut bool, tip: &str) -> egui::Response {
    let (r, mut resp) = ui.allocate_exact_size(vec2(28.0, 16.0), Sense::click());
    let p = ui.painter();
    let track = Rect::from_center_size(r.center(), vec2(26.0, 14.0));
    p.rect_filled(track, 7.0, if *on { ACCENT } else { PANEL3 });
    let x = if *on { track.right() - 7.0 } else { track.left() + 7.0 };
    p.circle_filled(pos2(x, track.center().y), 5.0, if *on { TEXT_DARK } else { TEXT_DIM });
    if resp.clicked() {
        *on = !*on;
        resp.mark_changed();
    }
    resp.on_hover_text(tip)
}

/// A length to click through (right-click goes back), one of `lengths`
/// (in beats, with its name). Whether it changed.
fn length_button(ui: &mut Ui, value: &mut f32, lengths: &[(f32, &str)], tip: &str) -> bool {
    let k = lengths.iter().position(|l| (l.0 - *value).abs() < 1e-3);
    let label = k.map_or_else(|| format!("{value} beats"), |k| lengths[k].1.to_string());
    let r = ui.add(egui::Button::new(RichText::new(label).color(TEXT)).fill(PANEL3).min_size(vec2(62.0, 0.0))).on_hover_text(format!("{tip}. Click: longer, right-click: shorter"));
    let n = lengths.len();
    let to = if r.clicked() {
        Some(k.map_or(0, |k| (k + 1) % n))
    } else if r.secondary_clicked() {
        Some(k.map_or(0, |k| (k + n - 1) % n))
    } else {
        None
    };
    match to {
        Some(t) => {
            *value = lengths[t].0;
            true
        }
        None => false,
    }
}

/// The lengths of a change of clip, in beats.
const CHANGE_LENGTHS: [(f32, &str); 5] = [(0.25, "1/4 beat"), (0.5, "1/2 beat"), (1.0, "1 beat"), (2.0, "2 beats"), (4.0, "4 beats")];
/// The lengths of a change of Look (Cut: at once).
const MORPH_LENGTHS: [(f32, &str); 6] = [(0.0, "Cut"), (0.25, "1/4 beat"), (0.5, "1/2 beat"), (1.0, "1 beat"), (2.0, "2 beats"), (4.0, "4 beats")];

/// The name of a track (`none` for no track, the master or every one).
fn track_name(tracks: &[(u64, String)], id: Option<u64>, none: &str) -> String {
    match id {
        None => none.to_string(),
        Some(id) => tracks.iter().find(|t| t.0 == id).map_or("a track gone".to_string(), |t| t.1.clone()),
    }
}

/// What hits drive, said in a few words: "kicks in Drums".
fn hits_said(t: &HitTrigger, tracks: &[(u64, String)]) -> String {
    let band = match t.band {
        kv::HitBand::Any => "hits".to_string(),
        b => b.name().to_lowercase(),
    };
    format!("{band} in {}", track_name(tracks, t.track, "the master"))
}

/// Over the clips: when a click starts one and how it changes to the next.
fn clip_head(ui: &mut Ui, set: &mut kv::VideoSet) {
    ui.spacing_mut().item_spacing.x = 6.0;
    deck_title(ui, "CLIPS");
    ui.label(RichText::new("Start").small().color(TEXT_DIM));
    let launches: Vec<(Launch, String)> = Launch::ALL.iter().map(|l| (*l, launch_name(*l).to_string())).collect();
    let tip = set.launch.about();
    dropdown(ui, "video_launch", 84.0, &mut set.launch, &launches, tip);
    ui.label(RichText::new("Change").small().color(TEXT_DIM));
    let ts: Vec<(Transition, String)> = Transition::ALL.iter().map(|t| (*t, t.name().to_string())).collect();
    let tip = set.transition.about();
    dropdown(ui, "video_transition", 64.0, &mut set.transition, &ts, tip);
    if set.transition != Transition::Cut {
        length_button(ui, &mut set.transition_beats, &CHANGE_LENGTHS, "How long the change takes, in beats of the song");
    }
}

/// Over the Look pads: when a click starts one and how long it morphs.
fn look_head(ui: &mut Ui, set: &mut kv::VideoSet) {
    ui.spacing_mut().item_spacing.x = 6.0;
    deck_title(ui, "LOOKS");
    ui.label(RichText::new("Start").small().color(TEXT_DIM));
    let launches: Vec<(Launch, String)> = Launch::ALL.iter().map(|l| (*l, launch_name(*l).to_string())).collect();
    dropdown(ui, "video_look_launch", 84.0, &mut set.look_launch, &launches, "When a Look pad clicked takes over (while the song plays)");
    ui.label(RichText::new("Morph").small().color(TEXT_DIM)).on_hover_text("How long a change of Look takes");
    length_button(ui, &mut set.morph_beats, &MORPH_LENGTHS, "How long a change of Look takes: its amounts, settings and colours move over to the next (Cut: at once)");
    if set.morph_beats > 0.0 {
        let ways: Vec<(Transition, String)> = Transition::ALL.iter().filter(|t| **t == Transition::Fade || t.pattern() > 0.0).map(|t| (*t, t.name().to_string())).collect();
        vj::chip(ui, "video_look_change", 74.0, &mut set.look_change, &ways, "How a Look with other Visuals comes in over its morph: everywhere at once (Fade), or out from the middle, in from the edges, a wipe, a spiral, patches, blinds, rings");
    }
}

/// What hears the hits and what a hit does: the sound, the band, how
/// sensitive, the least gap and the grid (`what`: what it moves, for the
/// list of what a hit does).
fn hit_editor(ui: &mut Ui, id: impl std::hash::Hash + Copy, t: &mut HitTrigger, tracks: &[(u64, String)], what: &str) {
    row(ui, "Hits in", |ui| {
        track_dropdown(ui, (id, "track"), &mut t.track, tracks, "Master", "The sound heard: the whole song, or one track (a drum machine's own track hears only its drums)");
        let bands: Vec<(kv::HitBand, String)> = kv::HitBand::ALL.iter().map(|b| (*b, b.name().to_string())).collect();
        dropdown(ui, (id, "band"), 74.0, &mut t.band, &bands, "Which hits: kicks (the low end), snares (the middle), hats (the top) or any");
    });
    row(ui, "Each hit", |ui| {
        let tos: Vec<(kv::StepTo, String)> = kv::StepTo::ALL.iter().map(|s| (*s, format!("{} {what}", s.name()))).collect();
        let tip = t.to.about();
        dropdown(ui, (id, "to"), 130.0, &mut t.to, &tos, tip);
    });
    row(ui, "Sensitivity", |ui| {
        ui.spacing_mut().slider_width = (ui.available_width() - 48.0).clamp(40.0, 170.0);
        vj::bar(ui, &mut t.sensitivity, 0.0, 1.0, 0.5, "", vj::pct, 0.0, None, "More: quieter hits count too");
        ui.label(RichText::new(format!("{:.0} %", t.sensitivity * 100.0)).small().color(TEXT_DIM));
    });
    row(ui, "Gap", |ui| {
        ui.spacing_mut().item_spacing.x = 3.0;
        for b in kv::HIT_LENGTHS.iter().skip(1).take(4) {
            if choice(ui, &beats_name(*b), (t.gap - b).abs() < 1e-3, "The least time between two hits (in beats): one sooner is not heard").clicked() {
                t.gap = *b;
            }
        }
    });
    row(ui, "On the grid", |ui| {
        ui.spacing_mut().item_spacing.x = 3.0;
        for b in [0.0, 0.25, 1.0] {
            if choice(ui, &beats_name(b), (t.quantize - b).abs() < 1e-3, "A hit waits for the next step of this grid (Off: at once, as heard)").clicked() {
                t.quantize = b;
            }
        }
    });
}

/// A row that switches hits on and off for something, and their editor
/// under it while they are on.
fn hits_row(ui: &mut Ui, id: impl std::hash::Hash + Copy, t: &mut Option<HitTrigger>, tracks: &[(u64, String)], what: &str, tip: &str) {
    row(ui, "Hits", |ui| {
        let mut on = t.is_some();
        if switch(ui, &mut on, tip).changed() || on != t.is_some() {
            *t = on.then(HitTrigger::default);
        }
        let said = t.as_ref().map_or("off".to_string(), |t| hits_said(t, tracks));
        ui.label(RichText::new(said).small().color(TEXT_DIM));
    });
    if let Some(t) = t {
        hit_editor(ui, id, t, tracks, what);
        ui.add_space(4.0);
    }
}

/// How a link answers its source: where the effect rests, how fast it
/// rises and falls, its curve, threshold and invert, and an LFO's or a
/// band's own settings.
fn link_response(ui: &mut Ui, i: usize, l: &mut VideoLink) {
    let fader = |ui: &mut Ui, label: &str, v: &mut f32, range: std::ops::RangeInclusive<f32>, suffix: &str, tip: &str| {
        let d = range.start().max(0.0);
        vj::bar(ui, v, *range.start(), *range.end(), if label == "Curve" { 1.0 } else { d }, label, |v| format!("{v:.2}{suffix}"), 0.0, None, tip);
    };
    if !l.effect.on_hits() {
        fader(ui, "At rest", &mut l.low, 0.0..=1.0, "", "Where the effect is with the source at nothing (the amount is where it is at full; above the amount turns it round)");
    }
    fader(ui, "Attack", &mut l.response.attack, 0.0..=2.0, " s", "How long it takes to rise (0: at once)");
    fader(ui, "Release", &mut l.response.release, 0.0..=2.0, " s", "How long it takes to fall back (0: at once)");
    fader(ui, "Curve", &mut l.response.curve, 0.25..=4.0, "", "Above 1 it holds back and comes late and sudden; below 1 early and soft");
    fader(ui, "Threshold", &mut l.response.threshold, 0.0..=0.95, "", "It only moves above this");
    row(ui, "", |ui| ui.checkbox(&mut l.response.invert, RichText::new("Invert").small()).on_hover_text("Upside down: the effect is up when the source is down"));
    match l.source {
        Source::Lfo => {
            row(ui, "Shape", |ui| {
                let shapes: Vec<(kv::LfoShape, String)> = kv::LfoShape::ALL.iter().map(|s| (*s, s.name().to_string())).collect();
                dropdown(ui, ("video_lfo_shape", i), 90.0, &mut l.lfo.shape, &shapes, "The LFO's shape");
            });
            row(ui, "A cycle", |ui| {
                let names: Vec<String> = kv::LFO_BEATS.iter().map(|b| if *b < 1.0 { format!("1/{} beat", (1.0 / b).round()) } else if *b == 1.0 { "1 beat".to_string() } else { format!("{b} beats") }).collect();
                let lengths: Vec<(f32, &str)> = kv::LFO_BEATS.iter().zip(&names).map(|(b, n)| (*b, n.as_str())).collect();
                length_button(ui, &mut l.lfo.beats, &lengths, "A cycle every so many beats of the song");
            });
            fader(ui, "Phase", &mut l.lfo.phase, 0.0..=1.0, "", "Where in its cycle it is on the song's first beat");
        }
        Source::Band => {
            row(ui, "Band", |ui| {
                ui.add(egui::DragValue::new(&mut l.band.0).range(20.0..=20000.0).speed(5.0).suffix(" Hz")).on_hover_text("From");
                ui.label(RichText::new("to").small().color(TEXT_DIM));
                ui.add(egui::DragValue::new(&mut l.band.1).range(20.0..=20000.0).speed(20.0).suffix(" Hz")).on_hover_text("To");
            });
        }
        _ => {}
    }
}

/// The Look's faders, in groups: (name, colour, effects).
const GROUPS: [(&str, [u8; 3], &[Effect]); 7] = [
    ("VISUALS", [0xa0, 0x78, 0xff], &[Effect::Visuals, Effect::Burst, Effect::Warp, Effect::Density, Effect::Rush, Effect::Glow, Effect::Orbit, Effect::Fracture]),
    ("MOTION", [0x40, 0xc8, 0xdc], &[Effect::Zoom, Effect::Shake, Effect::Spin, Effect::ZoomBlur, Effect::Feedback]),
    ("COLOUR", [0xff, 0x9a, 0x40], &[Effect::Hue, Effect::Tint, Effect::Grade, Effect::Poster, Effect::Invert, Effect::Edges]),
    ("SHAPE", [0x5c, 0xd0, 0x7a], &[Effect::Mirror, Effect::Kaleido, Effect::Pixels, Effect::Split, Effect::Glitch, Effect::Blur]),
    ("LIGHT", [0xf0, 0xd0, 0x40], &[Effect::Flash, Effect::Strobe, Effect::Gate, Effect::Vignette, Effect::Scanlines, Effect::Grain]),
    ("TIME", [0x60, 0x90, 0xff], &[Effect::Trails, Effect::Echo, Effect::Wave]),
    ("FILM", [0xff, 0x60, 0xa0], &[Effect::Speed, Effect::Freeze, Effect::Jump, Effect::Stutter, Effect::Scratch, Effect::NextClip]),
];

/// Milk's faders: (value, its name on the fader).
const MILK_MOTION: [(MilkParam, &str); 10] = [(MilkParam::Zoom, "Zoom"), (MilkParam::Rot, "Rotate"), (MilkParam::Warp, "Warp"), (MilkParam::Ripple, "Ripple"), (MilkParam::Decay, "Decay"), (MilkParam::Dx, "Move X"), (MilkParam::Dy, "Move Y"), (MilkParam::Sx, "Stretch"), (MilkParam::Mirror, "Mirror"), (MilkParam::Tile, "Tiles")];
const MILK_COLOUR: [(MilkParam, &str); 8] = [(MilkParam::Gamma, "Gamma"), (MilkParam::Solarize, "Solarize"), (MilkParam::Invert, "Invert"), (MilkParam::Hue, "Hue"), (MilkParam::HueDrift, "Hue Drift"), (MilkParam::Chroma, "Chroma"), (MilkParam::Blur, "Blur"), (MilkParam::DarkenCenter, "Dark Ctr")];
const MILK_DRAW: [(MilkParam, &str); 8] = [(MilkParam::WaveAlpha, "Wave"), (MilkParam::WaveScale, "Wave Size"), (MilkParam::ShapeAlpha, "Shape"), (MilkParam::ShapeCount, "Shapes"), (MilkParam::ShapeRing, "Ring"), (MilkParam::EchoAlpha, "Echo"), (MilkParam::EchoZoom, "Echo Zoom"), (MilkParam::Sparkle, "Sparkle")];

/// Milk's panel: its preset (and the one playing), how it goes on to the
/// next, and faders that move the preset's values (in the middle: as it is).
fn milk_panel(ui: &mut Ui, m: &mut milk::MilkLook, now: Option<(&str, [f32; milk::MILK_N])>) {
    let col = Color32::from_rgb(0xe8, 0xe0, 0xd0);
    egui::Frame::new().fill(mix(PANEL2, col, 0.05)).stroke(Stroke::new(1.0_f32, mix(LINE_SOFT, col, 0.3))).corner_radius(5).inner_margin(egui::Margin::symmetric(6, 5)).show(ui, |ui| {
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 4.0;
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                ui.label(RichText::new("MILK").small().strong().color(col));
                ui.add_space(4.0);
                let n = milk::presets().len();
                let mut k = m.place();
                if ui.small_button("<").on_hover_text("The preset before").clicked() {
                    k = (k + n - 1) % n;
                }
                let opts: Vec<(usize, String)> = milk::presets().iter().enumerate().map(|(i, p)| (i, p.name.to_string())).collect();
                vj::chip(ui, "video_milk_preset", 120.0, &mut k, &opts, "Milk's preset: what moves how, and what is drawn");
                if ui.small_button(">").on_hover_text("The next preset").clicked() {
                    k = (k + 1) % n;
                }
                if k != m.place() {
                    m.preset = milk::presets()[k].name.to_string();
                }
                if let Some((name, _)) = now.filter(|(name, _)| *name != m.preset) {
                    ui.label(RichText::new(format!("now {name}")).small().color(TEXT_DIM)).on_hover_text("The preset it went on to by itself");
                }
                ui.add_space(8.0);
                ui.label(RichText::new("Next").small().color(TEXT_DIM));
                let autos: Vec<(u32, String)> = milk::AUTO.iter().map(|(n, b)| (*b, n.to_string())).collect();
                vj::segmented(ui, &mut m.auto, &autos, "Every so many bars it goes on to another preset by itself (while the song plays)");
                if m.auto > 0 && choice(ui, "Random", m.random, "The next by chance, rather than in turn").clicked() {
                    m.random = !m.random;
                }
                if m.auto == 1 {
                    let secs: Vec<(f32, &str)> = milk::SECONDS.to_vec();
                    length_button(ui, &mut m.seconds, &secs, "About how long between presets (a little sooner or later each time)");
                }
                if choice(ui, "Cuts", m.cuts, "A big hit in the bass cuts to another preset at once").clicked() {
                    m.cuts = !m.cuts;
                }
            });
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                ui.add_space(36.0);
                ui.label(RichText::new("Change").small().color(TEXT_DIM));
                let styles: Vec<(u8, String)> = milk::STYLES.iter().enumerate().map(|(i, n)| (i as u8, n.to_string())).collect();
                vj::chip(ui, "video_milk_style", 86.0, &mut m.style, &styles, "How one preset gives way to the next: its values melting everywhere at once, or the new one coming in out from the middle, in from the edges, as a wipe, a spiral, a plasma, blinds or rings (Random: one of them each time); the last one goes on moving while it does");
                ui.label(RichText::new("over").small().color(TEXT_DIM));
                let blends: Vec<(f32, &str)> = milk::BLENDS.to_vec();
                length_button(ui, &mut m.blend, &blends, "How long one preset takes to give way to the next");
            });
            for (row_name, faders) in [("MOTION", &MILK_MOTION[..]), ("COLOUR", &MILK_COLOUR[..]), ("DRAW", &MILK_DRAW[..])] {
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    let (r, _) = ui.allocate_exact_size(vec2(40.0, 96.0), Sense::hover());
                    ui.painter().text(pos2(r.left(), r.top() + 6.0), Align2::LEFT_CENTER, row_name, small(), TEXT_DIM);
                    for (param, name) in faders {
                        let d = param.def();
                        let mut t = m.tweaks.get(param).copied().unwrap_or(0.5);
                        let live = now.map_or(t, |(_, v)| (v[param.index()] - d.min) / (d.max - d.min));
                        ui.vertical(|ui| {
                            ui.set_width(46.0);
                            ui.vertical_centered(|ui| {
                                let tip = format!("{}: moves the preset's value either way (the middle: as the preset has it)", d.name);
                                let rest = (t - 0.5).abs() < 1e-3;
                                if vj::fader(ui, &mut t, live, 0.5, vec2(28.0, 80.0), col, rest, &tip).changed() {
                                    if (t - 0.5).abs() < 1e-3 {
                                        m.tweaks.remove(param);
                                    } else {
                                        m.tweaks.insert(*param, t);
                                    }
                                }
                                ui.add(egui::Label::new(RichText::new(*name).small().color(TEXT)).truncate());
                            });
                        });
                    }
                    if !m.tweaks.is_empty() && ui.small_button("Reset").on_hover_text("The preset as it is").clicked() {
                        m.tweaks.clear();
                    }
                });
            }
        });
    });
}

/// The Visuals' blend and colours, in their group's head.
fn visuals_head(ui: &mut Ui, look: &mut kv::VideoLook) {
    let e = Effect::Visuals;
    let mut s = look.setting(e);
    ui.add_space(6.0);
    let mut b = s.param(e, 1).round() as usize;
    let opts: Vec<(usize, String)> = kv::VISUAL_BLENDS.iter().enumerate().map(|(i, n)| (i, n.to_string())).collect();
    if vj::chip(ui, "video_vis_blend", 132.0, &mut b, &opts, "How the Visuals go on the film (with no film: Instead, Over or Screen show them on black)") {
        s.set_param(e, 1, b as f32);
    }
    let own = s.color.or(e.color()).unwrap_or([0xff; 3]);
    let pal = s.param(e, 2).round() as usize;
    ui.spacing_mut().item_spacing.x = 2.0;
    for (k, name) in kv::VISUAL_PALETTES.iter().enumerate() {
        let (r, resp) = ui.allocate_exact_size(vec2(20.0, vj::H), Sense::click());
        vj::swatch(ui.painter(), r.shrink(2.0), k, Color32::from_rgb(own[0], own[1], own[2]));
        if k == pal {
            ui.painter().rect_stroke(r, 3.0, Stroke::new(1.5_f32, TEXT), egui::StrokeKind::Inside);
        }
        if resp.on_hover_text(format!("Colours: {name}")).clicked() {
            s.set_param(e, 2, k as f32);
        }
    }
    if pal == 0 {
        let mut c = own;
        if ui.color_edit_button_srgb(&mut c).on_hover_text("Their own colour").changed() {
            s.color = Some(c);
        }
    }
    if s != look.setting(e) {
        *look.setting_mut(e) = s;
    }
}

/// An effect's fader, its name and what moves it. Its name clicked: where
/// to show its settings (the place under it).
fn fader_column(ui: &mut Ui, look: &mut kv::VideoLook, e: Effect, live: Option<&kv::Look>, col: Color32) -> Option<[f32; 2]> {
    let mut s = look.setting(e);
    let links: Vec<Source> = look.links.iter().filter(|l| l.effect == e).map(|l| l.source).collect();
    let used = s.base > 0.0 || !links.is_empty();
    let dim = s.off || !used;
    let now = live.map_or(s.base, |l| l.get(e));
    let mut opened = None;
    ui.vertical(|ui| {
        ui.set_width(46.0);
        ui.vertical_centered(|ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            let tip = if e.on_hits() { format!("{}: how often it happens on its own (a link makes it happen on hits)", e.name()) } else { format!("{}: how much of it there always is (what moves it adds to it)", e.name()) };
            vj::fader(ui, &mut s.base, now, 0.0, vec2(28.0, 96.0), col, dim, &tip);
            let name = ui.add(egui::Label::new(RichText::new(e.name()).small().color(if dim { TEXT_DIM } else { TEXT })).truncate().sense(Sense::click()));
            let tag = match links.as_slice() {
                [] => "+".to_string(),
                [one] => one.name().to_string(),
                [one, more @ ..] => format!("{}+{}", one.name(), more.len()),
            };
            let chip = ui.add(egui::Button::new(RichText::new(tag).small().color(if links.is_empty() { TEXT_DIM } else { col })).fill(PANEL3).corner_radius(8.0).min_size(vec2(42.0, 16.0)));
            let name = name.on_hover_text(format!("{}\nClick: its settings and what moves it", e.about()));
            let chip = chip.on_hover_text("What in the music moves it: click to link it, or to change its links");
            if name.clicked() || chip.clicked() {
                opened = Some([name.rect.left() - 4.0, chip.rect.bottom() + 4.0]);
            }
        });
    });
    if s != look.setting(e) {
        if s == kv::FxSetting::default() {
            look.fx.remove(&e);
        } else {
            *look.setting_mut(e) = s;
        }
    }
    opened
}

/// An effect's settings and what moves it (its links: the source, what it
/// hears, how much, how it answers). Whether to close.
fn effect_detail(ui: &mut Ui, look: &mut kv::VideoLook, e: Effect, readings: &[f32], tracks: &[(u64, String)], add: &mut Option<Source>) -> bool {
    let mut close = false;
    let mut s = look.setting(e);
    ui.spacing_mut().item_spacing.y = 4.0;
    ui.horizontal(|ui| {
        let mut on = !s.off;
        switch(ui, &mut on, "Off: the Look without it, its settings and links kept");
        s.off = !on;
        ui.label(RichText::new(e.name()).strong().color(TEXT)).on_hover_text(e.about());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.add(egui::Button::new(RichText::new("×").color(TEXT_DIM)).frame(false)).on_hover_text("Close (Esc)").clicked() {
                close = true;
            }
            if ui.small_button("Reset").on_hover_text("Its settings and colour as they start (its amount and links stay)").clicked() {
                s.params.clear();
                s.color = None;
            }
        });
    });
    let tip = if e.on_hits() { "How often it happens on its own" } else { "How much of it there always is" };
    vj::bar(ui, &mut s.base, 0.0, 1.0, 0.0, if e.on_hits() { "On its own" } else { "Always" }, vj::pct, 0.0, None, tip);
    for (k, def) in e.params().iter().enumerate() {
        let mut v = s.param(e, k);
        let changed = if def.choices.is_empty() {
            let span = def.max - def.min;
            let dec = if span > 20.0 { 0 } else if span < 0.25 { 3 } else { 2 };
            vj::bar(ui, &mut v, def.min, def.max, def.default, def.name, |v| format!("{v:.dec$}"), 0.0, None, def.about).changed()
        } else {
            row(ui, def.name, |ui| {
                let mut i = v.round() as usize;
                let opts: Vec<(usize, String)> = def.choices.iter().enumerate().map(|(i, n)| (i, n.to_string())).collect();
                let c = if def.length {
                    vj::segmented(ui, &mut i, &opts, def.about)
                } else {
                    vj::pick(ui, ("video_fx_choice", e.index(), k), 150.0, &mut i, &opts, def.about)
                };
                v = i as f32;
                c
            })
        };
        if changed {
            s.set_param(e, k, v);
        }
    }
    if e.has_color() && e != Effect::Visuals {
        row(ui, "Colour", |ui| {
            let own = s.color.is_some();
            let label = if e == Effect::Tint { "The track's" } else { "White" };
            if choice(ui, label, !own, if e == Effect::Tint { "In the colour of the track its link hears" } else { "Its own colour: white" }).clicked() {
                s.color = None;
            }
            let mut c = s.color.or(e.color()).unwrap_or([0xff, 0x40, 0xa0]);
            if ui.color_edit_button_srgb(&mut c).on_hover_text("A colour of its own").changed() || (own && s.color != Some(c)) {
                s.color = Some(c);
            }
        });
    }
    // What moves it.
    ui.add_space(4.0);
    ui.label(RichText::new("MOVED BY").small().color(TEXT_DIM));
    let sources: Vec<(Source, String)> = Source::ALL.iter().map(|s| (*s, s.name().to_string())).collect();
    let mut remove = None;
    for (i, l) in look.links.iter_mut().enumerate().filter(|(_, l)| l.effect == e) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 3.0;
            let about = l.source.about();
            vj::chip(ui, ("video_link_src", i), 58.0, &mut l.source, &sources, about);
            if l.source.has_track() {
                let none = if l.source.listens() { "Master" } else { "All tracks" };
                let mut options: Vec<(Option<u64>, String)> = vec![(None, none.to_string())];
                options.extend(tracks.iter().map(|(id, n)| (Some(*id), n.clone())));
                vj::chip(ui, ("video_link_from", i), 76.0, &mut l.from, &options, if l.source.listens() { "The sound it listens to: the master (the whole song) or a track" } else { "Whose notes: every track's, or one's" });
            }
            let more_id = Id::new(("video_link_more", i));
            let mut more = ui.ctx().data(|d| d.get_temp::<bool>(more_id)).unwrap_or(false);
            let w = ui.available_width() - 34.0;
            let tip = if l.effect.on_hits() { "How often a hit does it" } else { "How much at full" };
            vj::bar(ui, &mut l.amount, 0.0, 1.0, 0.5, "", vj::pct, w, readings.get(i).copied(), tip);
            if ui.add(egui::Button::new(RichText::new("...").small().color(if more { ACCENT } else { TEXT_DIM })).frame(false)).on_hover_text("How it answers: where it rests, attack, release, curve, threshold").clicked() {
                more = !more;
                ui.ctx().data_mut(|d| d.insert_temp(more_id, more));
            }
            if ui.add(egui::Button::new(RichText::new("x").small().color(TEXT_DIM)).frame(false)).on_hover_text("Take this link away").clicked() {
                remove = Some(i);
            }
        });
        if ui.ctx().data(|d| d.get_temp::<bool>(Id::new(("video_link_more", i)))).unwrap_or(false) {
            egui::Frame::new().fill(PANEL2).corner_radius(3).inner_margin(egui::Margin::same(4)).show(ui, |ui| link_response(ui, i, l));
        }
    }
    if let Some(i) = remove {
        look.links.remove(i);
    }
    if look.links.len() < kv::MAX_LINKS {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(3.0, 3.0);
            ui.label(RichText::new("+").small().color(TEXT_DIM));
            for src in Source::ALL {
                if vj::word_tile(ui, src.name(), true, src.about()).clicked() {
                    *add = Some(src);
                }
            }
        });
    }
    if s != look.setting(e) {
        if s == kv::FxSetting::default() {
            look.fx.remove(&e);
        } else {
            *look.setting_mut(e) = s;
        }
    }
    close
}

/// A row of steps, each one of `n` (shown by `name`) or none (`none`):
/// click for the next, right-click to pick; - and + make it shorter or
/// longer. The step `now` is lit.
#[allow(clippy::too_many_arguments)]
fn steps_row(ui: &mut Ui, id: impl std::hash::Hash + Copy, steps: &mut Vec<Option<u8>>, n: usize, now: Option<usize>, none: &str, name: impl Fn(usize) -> String, tip: &str) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(3.0, 3.0);
        for (k, s) in steps.iter_mut().enumerate() {
            let label = s.map_or(none.to_string(), |v| name(v as usize));
            let lit = now == Some(k);
            let b = egui::Button::new(RichText::new(label).small().color(if lit { TEXT_DARK } else if s.is_some() { TEXT } else { TEXT_DIM })).fill(if lit { ACCENT } else { PANEL3 }).min_size(vec2(28.0, 22.0));
            let r = ui.add(b).on_hover_text(format!("Step {}. {tip}", k + 1));
            if r.clicked() {
                *s = match *s {
                    None => Some(0),
                    Some(v) if (v as usize) + 1 < n => Some(v + 1),
                    Some(_) => None,
                };
            }
            r.context_menu(|ui| {
                if ui.selectable_label(s.is_none(), none).clicked() {
                    *s = None;
                    ui.close();
                }
                for v in 0..n.min(64) {
                    if ui.selectable_label(*s == Some(v as u8), name(v)).clicked() {
                        *s = Some(v as u8);
                        ui.close();
                    }
                }
            });
        }
        if steps.len() > 1 && ui.small_button("-").on_hover_text("One step fewer").clicked() {
            steps.pop();
        }
        if steps.len() < kv::MAX_STEPS && ui.small_button("+").on_hover_text("One step more").clicked() {
            let next = steps.last().copied().flatten().map(|v| ((v as usize + 1) % n.max(1)) as u8);
            steps.push(next);
        }
    });
    let _ = id;
}

/// The sixteen-by-nine rect in `r`, in its middle.
fn sixteen_nine(r: Rect) -> Rect {
    let w = r.width().min(r.height() * 16.0 / 9.0);
    Rect::from_center_size(r.center(), vec2(w, w * 9.0 / 16.0))
}

/// m:ss.s
fn clock(secs: f64) -> String {
    let s = secs.max(0.0);
    format!("{}:{:04.1}", (s / 60.0).floor() as u64, s % 60.0)
}

/// The note that picks clip or slice `k` (C1 the first).
fn note_name(k: usize) -> String {
    const NAMES: [&str; 12] = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
    let p = 36 + k;
    format!("{}{}", NAMES[p % 12], p as i32 / 12 - 2)
}

/// A step for the ruler's marks that leaves about `min_px` between them.
fn ruler_step(px_per_sec: f32, min_px: f32) -> f32 {
    const STEPS: [f32; 14] = [0.1, 0.2, 0.5, 1.0, 2.0, 5.0, 10.0, 15.0, 30.0, 60.0, 120.0, 300.0, 600.0, 1800.0];
    STEPS.iter().copied().find(|s| s * px_per_sec >= min_px).unwrap_or(3600.0)
}

/// How near a part's edge (pixels) a press takes the edge.
const EDGE_ZONE: f32 = 7.0;
/// How far outside a part its edge can still be taken.
const EDGE_OUTSIDE: f32 = 4.0;

/// What a press on a part does: trims its left or right edge, or moves it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PartGrab {
    Left,
    Right,
    Move,
}

/// What a press at `x` does to a part drawn from `xa` to `xb` (pixels on
/// the timeline, whatever its zoom): near an edge it takes the edge,
/// inside it the part. A part narrower than three grab zones is split in
/// thirds (the left third its start, the right third its end), so even a
/// sliver can be trimmed and moved. Just outside an edge still takes it.
/// None: nowhere near.
pub(crate) fn part_grab(x: f32, xa: f32, xb: f32) -> Option<PartGrab> {
    let (xa, xb) = (xa.min(xb), xa.max(xb));
    if x < xa - EDGE_OUTSIDE || x > xb + EDGE_OUTSIDE {
        return None;
    }
    if x < xa {
        return Some(PartGrab::Left);
    }
    if x > xb {
        return Some(PartGrab::Right);
    }
    let zone = if xb - xa < 3.0 * EDGE_ZONE { (xb - xa) / 3.0 } else { EDGE_ZONE };
    Some(if x - xa <= zone {
        PartGrab::Left
    } else if xb - x <= zone {
        PartGrab::Right
    } else {
        PartGrab::Move
    })
}

/// The part a press at `x` takes and how, of parts drawn at `xs` (pixels,
/// in order): the part it is inside first (where two touch, the left one's
/// end), else an edge just outside one.
pub(crate) fn hit_parts(x: f32, xs: &[(f32, f32)]) -> Option<(usize, PartGrab)> {
    if let Some(k) = xs.iter().position(|&(a, b)| x >= a && x <= b) {
        return part_grab(x, xs[k].0, xs[k].1).map(|g| (k, g));
    }
    // Outside every part: the nearest edge within reach.
    xs.iter()
        .enumerate()
        .filter_map(|(k, &(a, b))| part_grab(x, a, b).map(|g| (k, g, (x - a).abs().min((x - b).abs()))))
        .min_by(|p, q| p.2.total_cmp(&q.2))
        .map(|(k, g, _)| (k, g))
}

/// What a drag on the timeline does, kept while it lasts.
#[derive(Clone, Copy, Debug)]
enum TimeDrag {
    /// A new part from where it started (seconds), between `lo` and `hi`.
    New { from: f32, lo: f32, hi: f32 },
    /// Part `k` moved, held `grab` seconds from its start.
    Move { k: usize, grab: f32, lo: f32, hi: f32 },
    /// Part `k`'s start (or end) moved, held `off` seconds from the edge.
    Edge { k: usize, end: bool, off: f32, lo: f32, hi: f32 },
}

impl App {
    /// Whether the Video's view is up.
    pub(crate) fn video_shown(&self) -> bool {
        self.view == View::Video
    }

    /// To the Video's view, remembering the view before it to go back to.
    pub(crate) fn open_video(&mut self) {
        if self.view != View::Video {
            self.video_back = self.view;
            self.view = View::Video;
        }
    }

    /// Back from the Video to the view before it.
    pub(crate) fn close_video(&mut self) {
        if self.view == View::Video {
            self.view = if self.video_back == View::Video { View::Studio } else { self.video_back };
        }
    }

    /// What the Video does wherever it shows: what was fetched or made
    /// comes in, its screen and the box for Export Video. (The view itself
    /// is `ui_video_view`; a frame of the Video is `video_frame`.)
    pub(crate) fn ui_video(&mut self, ctx: &Context) {
        if !self.video_shown() {
            self.video.window_rect = None;
        }
        self.poll_ffmpeg_fetch();
        self.poll_link_grab(ctx);
        self.poll_video_export(ctx);
        if self.video.screen_open {
            self.video_screen(ctx);
        }
        if self.video.export_open {
            self.video_export_box(ctx);
        }
    }

    fn poll_ffmpeg_fetch(&mut self) {
        let state = self.video.fetch.as_ref().and_then(|f| f.lock().ok().map(|s| s.clone()));
        match state {
            Some(ffmpeg::Fetch::Ready(_)) => {
                self.video.fetch = None;
                self.video.reload_films();
                self.toast("ffmpeg is here: films play now");
            }
            Some(ffmpeg::Fetch::Failed(e)) => {
                self.video.fetch = None;
                self.toast(e);
            }
            _ => {}
        }
    }

    /// A film from a link arrived: into the bank, its sound onto a new
    /// audio track when asked.
    fn poll_link_grab(&mut self, ctx: &Context) {
        let state = self.video.grab.as_ref().and_then(|g| g.lock().ok().map(|s| s.clone()));
        match state {
            Some(ytdl::Grab::Ready(path)) => {
                self.video.grab = None;
                let title = ytdl::title_of(&path);
                self.toast(format!("{title} is in the Video (and in Documents/Krabton/Films)"));
                let before = self.project.video.clips.len();
                self.video_add(std::slice::from_ref(&path));
                if let Some(c) = self.project.video.clips.get_mut(before) {
                    c.name = title;
                }
                if self.video.link_sound {
                    self.import_files(vec![path], crate::import::FileTarget::NewTrack { beat: None });
                }
            }
            Some(ytdl::Grab::Failed(e)) => {
                self.video.grab = None;
                self.toast(e);
            }
            Some(_) => ctx.request_repaint_after(std::time::Duration::from_millis(250)),
            None => {}
        }
    }

    /// Export Video finished (or failed): says so.
    fn poll_video_export(&mut self, ctx: &Context) {
        let Some(job) = &mut self.video.export else { return };
        // The graphics card draws the frames with Visuals in them, a few each time the window paints.
        job.gpu.paint(&ctx.layer_painter(egui::LayerId::background()));
        ctx.request_repaint();
        if !job.handle.as_ref().is_some_and(|h| h.is_finished()) {
            return;
        }
        // Plugin processors come back here so they are dropped on the UI thread.
        let _leftovers = job.handle.take().map(|h| h.join());
        let state = job.state();
        job.plugin_instances.clear();
        self.video.export = None;
        // Said on the console too, for scripts.
        match state {
            export::State::Done(p) => {
                println!("video export: done {}", p.display());
                self.toast(format!("Exported {}", p.display()));
            }
            export::State::Failed(e) => {
                println!("video export: {e}");
                self.toast(e);
            }
            _ => {}
        }
    }

    /// Fetches the film behind the link typed.
    pub(crate) fn video_fetch_link(&mut self) {
        let link = self.video.link.trim().to_string();
        if !ytdl::is_link(&link) {
            self.toast("That is not a link (it starts with https://)");
            return;
        }
        self.video.grab = Some(ytdl::start(link));
        self.video.link_open = false;
        self.video.link.clear();
    }

    /// The Video's view: the deck along the top (the clips over the
    /// picture, the Looks over the side), the picture with its timeline
    /// and what the music does in the middle, the side with its tabs on
    /// the right. Films and pictures dropped on it come into its bank.
    pub(crate) fn ui_video_view(&mut self, ui: &mut Ui) {
        let all = ui.available_rect_before_wrap();
        ui.allocate_rect(all, Sense::hover());
        let here = crate::import::file_drop_pos(ui.ctx()).is_none_or(|p| all.contains(p));
        let dropped: Vec<PathBuf> = ui.ctx().input(|i| i.raw.dropped_files.iter().filter_map(|f| f.path.clone()).filter(|p| media::takes(p)).collect());
        if here && !dropped.is_empty() {
            self.video_add(&dropped);
            crate::import::take_file_drop(ui.ctx());
        }
        let full = all.shrink(8.0);
        let gap = 8.0;
        // On the right the monitor: the picture, the meters, the scenes. The deck over the rest.
        let mon_w = (full.width() * 0.36).clamp(320.0, 520.0);
        let deck_h = (full.height() * 0.16).clamp(88.0, 112.0);
        let monitor = Rect::from_min_max(pos2(full.right() - mon_w, full.top()), full.max);
        let deck = Rect::from_min_size(full.min, vec2(full.width() - mon_w - gap, deck_h));
        let main = Rect::from_min_max(pos2(full.left(), deck.bottom() + gap), pos2(monitor.left() - gap, full.bottom()));
        // The timeline's keys work while the pointer is on the picture or the controls.
        self.video.window_rect = Some(main.union(monitor));
        self.video_deck(ui, deck, (deck.width() * 0.42).clamp(200.0, 360.0), gap);
        self.video_monitor(ui, monitor);
        self.video_main(ui, main);
    }

    /// The monitor: the picture (double-click: full screen), the meters, and the scenes of the Visuals.
    fn video_monitor(&mut self, ui: &mut Ui, r: Rect) {
        let pic = Rect::from_min_size(r.min, vec2(r.width(), r.width() * 9.0 / 16.0));
        let p = ui.painter().clone();
        p.rect_filled(pic, 3.0, Color32::BLACK);
        let set = self.project.video.clone();
        let m = self.video.playing_media(&set);
        let info = m.as_ref().map(|m| m.info()).unwrap_or_default();
        self.draw_stage(&p, pic);
        let pr = ui.interact(pic, Id::new("video_pic"), Sense::click());
        if pr.double_clicked() {
            self.video.screen_open = true;
            self.video.fullscreen = true;
        }
        pr.context_menu(|ui| {
            if ui.button("Screen (a window of its own)").clicked() {
                self.video.screen_open = true;
                ui.close();
            }
            if ui.button("Full screen").clicked() {
                self.video.screen_open = true;
                self.video.fullscreen = true;
                ui.close();
            }
        });
        // What to do when there is nothing to see.
        let mut msg_ui = ui.new_child(UiBuilder::new().max_rect(pic.shrink(10.0)).layout(egui::Layout::top_down(egui::Align::Center)));
        if set.clips.is_empty() && !self.video.has_picture() {
            self.video_welcome(&mut msg_ui, pic.height());
        } else if info.needs_ffmpeg {
            self.ffmpeg_offer(&mut msg_ui, pic.height());
        } else if let Some(e) = &info.error {
            msg_ui.add_space(pic.height() * 0.35);
            msg_ui.label(RichText::new(e).color(REC));
        }
        let ears = Rect::from_min_size(pos2(r.left(), pic.bottom() + 6.0), vec2(r.width(), 32.0));
        self.video_ears(ui, ears);
        let scenes = Rect::from_min_max(pos2(r.left(), ears.bottom() + 6.0), r.max);
        self.video_scenes(ui, scenes);
    }

    /// The Visuals' scenes as a list (as a visualiser has them): click one
    /// and the Look edited draws it; *Film only* turns them off.
    fn video_scenes(&mut self, ui: &mut Ui, r: Rect) {
        rect_fill(ui.painter(), r, 4.0, PANEL);
        let mut set = self.project.video.clone();
        let look = set.edited_look_mut();
        let v = look.setting(Effect::Visuals);
        let on = !v.off && (v.base > 0.0 || look.links.iter().any(|l| l.effect == Effect::Visuals));
        let now = if on { Some(v.param(Effect::Visuals, 0).round() as usize) } else { None };
        let mut ui = ui.new_child(UiBuilder::new().max_rect(r.shrink(6.0)));
        ui.label(RichText::new("SCENES").small().strong().color(TEXT_DIM));
        ui.add_space(2.0);
        let mut pick: Option<Option<usize>> = None;
        egui::ScrollArea::vertical().id_salt("video_scenes").auto_shrink([false, false]).show(&mut ui, |ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            let w = ui.available_width() - 8.0;
            for k in std::iter::once(None).chain((0..kv::VISUAL_SCENES.len()).map(Some)) {
                let (row_r, resp) = ui.allocate_exact_size(vec2(w, 34.0), Sense::click());
                let lit = now == k;
                let p = ui.painter();
                p.rect_filled(row_r, 3.0, if lit { ACCENT.gamma_multiply(0.25) } else if resp.hovered() { PANEL3 } else { PANEL2 });
                let icon = Rect::from_min_size(row_r.min + vec2(4.0, 3.0), vec2(64.0, 28.0));
                p.rect_filled(icon, 2.0, Color32::BLACK);
                let name = match k {
                    Some(k) => {
                        vj::scene_icon(&p.with_clip_rect(icon), icon.shrink(3.0), k, if lit { ACCENT } else { TEXT });
                        kv::VISUAL_SCENES[k]
                    }
                    None => "Film only",
                };
                p.text(pos2(icon.right() + 10.0, row_r.center().y), Align2::LEFT_CENTER, name, body(), if lit { TEXT } else { TEXT_DIM });
                let tip = match k {
                    Some(k) => kv::preset(kv::VISUAL_SCENES[k]).map_or("", |p| p.about),
                    None => "No Visuals: the film as the Look has it",
                };
                if resp.on_hover_text(tip).clicked() {
                    pick = Some(k);
                }
            }
        });
        if let Some(k) = pick {
            let s = look.setting_mut(Effect::Visuals);
            match k {
                Some(k) => {
                    s.set_param(Effect::Visuals, 0, k as f32);
                    s.off = false;
                    if s.base < 0.01 {
                        s.base = 1.0;
                    }
                }
                None => s.base = 0.0,
            }
            if *s == kv::FxSetting::default() {
                look.fx.remove(&Effect::Visuals);
            }
        }
        if set != self.project.video {
            self.project.video = set;
        }
    }

    /// The controls: the tabs, and what the one picked holds (the Look's
    /// faders; the clip with its timeline; the triggers; the output).
    fn video_main(&mut self, ui: &mut Ui, r: Rect) {
        rect_fill(ui.painter(), r, 4.0, PANEL);
        let tabs = Rect::from_min_size(r.min + vec2(6.0, 6.0), vec2((r.width() - 12.0).min(520.0), 24.0));
        let n = VideoTab::ALL.len() as f32;
        let w = (tabs.width() - 4.0 * (n - 1.0)) / n;
        for (k, t) in VideoTab::ALL.iter().enumerate() {
            let b = Rect::from_min_size(pos2(tabs.left() + k as f32 * (w + 4.0), tabs.top()), vec2(w, tabs.height()));
            if button(ui, b, Id::new(("video_tab", k)), t.name(), self.video.tab == *t, ACCENT).on_hover_text(t.about()).clicked() {
                self.video.tab = *t;
            }
        }
        let tab = self.video.tab;
        let mut body = Rect::from_min_max(pos2(r.left() + 8.0, tabs.bottom() + 8.0), pos2(r.right() - 4.0, r.bottom() - 4.0));
        // The clip's timeline over its settings.
        if tab == VideoTab::Clip && !self.project.video.clips.is_empty() {
            let (bar_h, tl_h, hint_h) = (28.0, 76.0, 16.0);
            let bar = Rect::from_min_size(body.min, vec2(body.width() - 6.0, bar_h));
            let tl = Rect::from_min_size(pos2(bar.left(), bar.bottom() + 4.0), vec2(bar.width(), tl_h));
            let line = Rect::from_min_size(pos2(bar.left(), tl.bottom() + 2.0), vec2(bar.width(), hint_h));
            let mut bar_ui = ui.new_child(UiBuilder::new().max_rect(bar).layout(egui::Layout::left_to_right(egui::Align::Center)));
            self.video_transport(&mut bar_ui);
            self.video_timeline(ui, tl);
            let said = self.video_timeline_hint();
            ui.painter().with_clip_rect(line).text(pos2(line.left() + 2.0, line.center().y), Align2::LEFT_CENTER, said, small(), TEXT_DIM);
            body.min.y = line.bottom() + 6.0;
        }
        let mut body_ui = ui.new_child(UiBuilder::new().max_rect(body));
        let mut area = egui::ScrollArea::vertical().id_salt(("video_side", tab.name())).auto_shrink([false, false]);
        if let Some(y) = self.video.side_scroll.take() {
            area = area.vertical_scroll_offset(y);
        }
        area.show(&mut body_ui, |ui| {
            ui.set_max_width(ui.available_width() - 10.0);
            let mut set = self.project.video.clone();
            let tracks: Vec<(u64, String)> = self.project.tracks.iter().map(|t| (t.id, t.name.clone())).collect();
            match tab {
                VideoTab::Clip => self.video_tab_clip(ui, &mut set, &tracks),
                VideoTab::Look => self.video_controls(ui, &mut set, &tracks),
                VideoTab::Triggers => self.video_tab_triggers(ui, &mut set, &tracks),
                VideoTab::Output => self.video_tab_output(ui, &mut set),
            }
            if set != self.project.video {
                self.project.video = set;
            }
            ui.add_space(12.0);
        });
    }

    /// The Look as a desk of faders: every effect always there, in groups;
    /// up from nothing it is in the Look. Under each fader what moves it;
    /// click its name for its settings and its links.
    fn video_controls(&mut self, ui: &mut Ui, set: &mut kv::VideoSet, tracks: &[(u64, String)]) {
        let n_looks = set.looks.len().max(1);
        let k = set.look.min(n_looks - 1);
        ui.add_space(2.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            let (b, _) = ui.allocate_exact_size(vec2(22.0, vj::H), Sense::hover());
            rect_fill(ui.painter(), b, 3.0, PANEL3);
            ui.painter().text(b.center(), Align2::CENTER_CENTER, (k + 1).to_string(), small(), TEXT);
            let look = set.edited_look_mut();
            ui.add(egui::TextEdit::singleline(&mut look.name).desired_width(140.0).hint_text(format!("Look {}", k + 1)))
                .on_hover_text("The Look edited (click a Look pad in the deck to edit another)");
            vj::bar(ui, &mut set.amount, 0.0, 1.0, 1.0, "Music", vj::pct, 170.0, None, "How much the music moves the picture: every link of every Look at once (0: the clip as it is, with what the effects always have)");
            let presets = ui.add(egui::Button::new(RichText::new("Presets").small().color(TEXT)).fill(PANEL3).corner_radius(4.0).min_size(vec2(0.0, vj::H))).on_hover_text("Fill this Look from a preset: Visuals, or effects for the film");
            let mut pick: Option<&str> = None;
            egui::Popup::menu(&presets).show(|ui| {
                ui.label(RichText::new("VISUALS").small().color(TEXT_DIM));
                egui::Grid::new("video_presets_vis").spacing([3.0, 3.0]).show(ui, |ui| {
                    for (k, name) in kv::VISUAL_SCENES.iter().enumerate() {
                        let about = kv::preset(name).map_or("", |p| p.about);
                        if vj::tile(ui, vec2(78.0, 50.0), false, name, about, |p, r, c| vj::scene_icon(p, r, k, c)).clicked() {
                            pick = Some(name);
                            ui.close();
                        }
                        if k % 5 == 4 {
                            ui.end_row();
                        }
                    }
                });
                ui.add_space(4.0);
                ui.label(RichText::new("THE FILM").small().color(TEXT_DIM));
                ui.horizontal_wrapped(|ui| {
                    ui.set_max_width(420.0);
                    for name in kv::PRESET_NAMES {
                        if let Some(pr) = kv::preset(name).filter(|p| p.fx.is_empty()) {
                            if vj::word_tile(ui, name, true, pr.about).clicked() {
                                pick = Some(name);
                                ui.close();
                            }
                        }
                    }
                });
            });
            if let Some(pr) = pick.and_then(kv::preset) {
                set.apply_preset(&pr);
                let mut links = std::mem::take(&mut set.edited_look_mut().links);
                self.video_fit_links(&mut links);
                set.edited_look_mut().links = links;
            }
        });
        ui.add_space(8.0);
        // What the music does to the Look now (when it is the one playing).
        let live = (self.video.dir.look_playing() == k).then_some(self.video.dir.look);
        let open_id = Id::new("video_fx_open");
        let mut open: Option<(usize, [f32; 2])> = ui.ctx().data(|d| d.get_temp(open_id)).flatten();
        let look = set.edited_look_mut();
        // Milk drawing: its presets and its faders first.
        let v = look.setting(Effect::Visuals);
        if v.base > 0.0 && !v.off && v.param(Effect::Visuals, 0).round() as usize == crate::video::stage::MILK_SCENE {
            let now = live.is_some().then(|| (self.video.dir.milk.name(), self.video.dir.milk_frame.now));
            milk_panel(ui, &mut look.milk, now);
            ui.add_space(8.0);
        }
        // The groups in rows, as many to a row as fit.
        let room = ui.available_width();
        let width = |effects: &[Effect]| {
            let faders = effects.len() + usize::from(effects.contains(&Effect::Visuals));
            let head = if effects.contains(&Effect::Visuals) { 400.0 } else { 0.0 };
            (faders as f32 * 48.0).max(head) + 14.0
        };
        let mut rows: Vec<Vec<usize>> = vec![Vec::new()];
        let mut used = 0.0;
        for (g, (_, _, effects)) in GROUPS.iter().enumerate() {
            let w = width(effects);
            if used + w > room && !rows.last().is_some_and(|r| r.is_empty()) {
                rows.push(Vec::new());
                used = 0.0;
            }
            if let Some(r) = rows.last_mut() {
                r.push(g);
            }
            used += w + 8.0;
        }
        for row_groups in rows {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
                for (title, rgb, effects) in row_groups.iter().map(|g| GROUPS[*g]) {
                    let col = Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
                    egui::Frame::new().fill(mix(PANEL2, col, 0.07)).stroke(Stroke::new(1.0_f32, mix(LINE_SOFT, col, 0.25))).corner_radius(5).inner_margin(egui::Margin::symmetric(6, 5)).show(ui, |ui| {
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 3.0;
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(title).small().strong().color(col));
                                if effects.contains(&Effect::Visuals) {
                                    visuals_head(ui, look);
                                }
                            });
                            ui.horizontal_top(|ui| {
                                ui.spacing_mut().item_spacing.x = 2.0;
                                for e in effects.iter() {
                                    if let Some(at) = fader_column(ui, look, *e, live.as_ref(), col) {
                                        open = if open.is_some_and(|o| o.0 == e.index()) { None } else { Some((e.index(), at)) };
                                    }
                                    // The Visuals' Speed, a fader of its own.
                                    if *e == Effect::Visuals {
                                        let s = look.setting_mut(Effect::Visuals);
                                        let mut v = s.param(Effect::Visuals, 3) / 4.0;
                                        ui.vertical(|ui| {
                                            ui.set_width(46.0);
                                            ui.vertical_centered(|ui| {
                                                let now = v;
                                                if vj::fader(ui, &mut v, now, 0.25, vec2(28.0, 96.0), col, false, "How fast the Visuals move by themselves (Rush makes them faster)").changed() {
                                                    s.set_param(Effect::Visuals, 3, v * 4.0);
                                                }
                                                ui.label(RichText::new("Speed").small().color(TEXT));
                                            });
                                        });
                                        if *s == kv::FxSetting::default() {
                                            look.fx.remove(&Effect::Visuals);
                                        }
                                    }
                                }
                            });
                        });
                    });
                }
            });
            ui.add_space(8.0);
        }
        // The settings and links of the effect picked, floating by its fader.
        if let Some((i, at)) = open {
            match Effect::ALL.iter().copied().find(|e| e.index() == i) {
                Some(e) => {
                    let mut close = false;
                    let readings = self.video.dir.readings.clone();
                    let mut add: Option<Source> = None;
                    egui::Area::new(Id::new("video_fx_panel")).order(egui::Order::Middle).fixed_pos(pos2(at[0], at[1])).show(ui.ctx(), |ui| {
                        egui::Frame::new().fill(PANEL).stroke(Stroke::new(1.0_f32, LINE)).corner_radius(6).inner_margin(egui::Margin::same(8)).shadow(egui::Shadow { offset: [0, 4], blur: 12, spread: 0, color: Color32::from_black_alpha(120) }).show(ui, |ui| {
                            ui.set_width(300.0);
                            close = effect_detail(ui, look, e, &readings, tracks, &mut add);
                        });
                    });
                    if let Some(src) = add {
                        let mut link = [VideoLink::new(src, e, 0.5)];
                        self.video_fit_links(&mut link);
                        look.links.extend(link);
                    }
                    if close || ui.input(|i| i.key_pressed(Key::Escape)) {
                        open = None;
                    }
                }
                None => open = None,
            }
        }
        ui.ctx().data_mut(|d| d.insert_temp(open_id, open));
    }

    // ------------------------------------------------------------------
    // The deck
    // ------------------------------------------------------------------

    /// The deck: the clips of the bank on the left and the Look pads on
    /// the right (as wide as the side under them), each under a line
    /// saying when a click starts it and how the change goes.
    fn video_deck(&mut self, ui: &mut Ui, r: Rect, looks_w: f32, gap: f32) {
        let clips = Rect::from_min_max(r.min, pos2(r.right() - looks_w - gap, r.bottom()));
        let looks = Rect::from_min_max(pos2(r.right() - looks_w, r.top()), r.max);
        let head_h = 26.0;
        for g in [clips, looks] {
            rect_fill(ui.painter(), g, 4.0, PANEL);
        }
        let head = |g: Rect| Rect::from_min_size(g.min + vec2(10.0, 2.0), vec2(g.width() - 20.0, head_h));
        let pads = |g: Rect| Rect::from_min_max(pos2(g.left() + 8.0, g.top() + head_h + 4.0), pos2(g.right() - 8.0, g.bottom() - 8.0));
        let mut set = self.project.video.clone();
        let mut h = ui.new_child(UiBuilder::new().max_rect(head(clips)).layout(egui::Layout::left_to_right(egui::Align::Center)));
        clip_head(&mut h, &mut set);
        let mut h = ui.new_child(UiBuilder::new().max_rect(head(looks)).layout(egui::Layout::left_to_right(egui::Align::Center)));
        look_head(&mut h, &mut set);
        if set != self.project.video {
            self.project.video = set;
        }
        self.video_bank(ui, pads(clips));
        let mut set = self.project.video.clone();
        self.video_look_pads(ui, pads(looks), &mut set);
        if set != self.project.video {
            self.project.video = set;
        }
    }

    /// The clips, as pads: click one to play it (and edit it), drag it to
    /// put it elsewhere, right-click for more. The last pad adds a file or
    /// a film from a link.
    fn video_bank(&mut self, ui: &mut Ui, r: Rect) {
        let clips = self.project.video.clips.clone();
        let room = clips.len() < kv::MAX_CLIPS;
        let slots = (clips.len() + usize::from(room)).max(1);
        let gap = 8.0;
        let ph = r.height();
        let pw = ((r.width() - gap * (slots as f32 - 1.0)) / slots as f32).min(ph * 16.0 / 9.0).max(40.0);
        let pad_at = |i: usize| Rect::from_min_size(pos2(r.left() + i as f32 * (pw + gap), r.top()), vec2(pw, ph));
        let playing = self.video.playing();
        let pending = self.video.dir.switch.pending.map(|p| p.0);
        let edited = self.project.video.current;
        let notes_pick = self.project.video.clip_notes.is_some();
        let drag_id = Id::new("video_pad_drag");
        let dragging: Option<usize> = ui.ctx().data(|d| d.get_temp(drag_id));
        let mut launch = None;
        let mut remove = None;
        let mut replace = None;
        let mut slice = None;
        let mut duplicate = None;
        let mut moved = None;
        for (i, c) in clips.iter().enumerate() {
            let pad = pad_at(i);
            let resp = ui.interact(pad, Id::new(("video_pad", i)), Sense::click_and_drag());
            let p = ui.painter();
            rect_fill(p, pad, 4.0, if resp.hovered() { PANEL3 } else { PANEL2 });
            let m = self.video.media.get(&c.path).cloned();
            let info = m.as_ref().map(|m| m.info()).unwrap_or_default();
            if let Some(t) = self.video.thumbs.get(&c.path) {
                let [tw, th] = t.size();
                // Filling the pad (what sticks out cut off), as a clip in a VJ program.
                let area = pad.shrink(1.0);
                let (pa, ta) = (area.width() / area.height(), tw as f32 / th.max(1) as f32);
                let uv = if ta > pa {
                    let u = pa / ta;
                    Rect::from_min_max(pos2((1.0 - u) * 0.5, 0.0), pos2((1.0 + u) * 0.5, 1.0))
                } else {
                    let v = ta / pa;
                    Rect::from_min_max(pos2(0.0, (1.0 - v) * 0.5), pos2(1.0, (1.0 + v) * 0.5))
                };
                p.image(t.id(), area, uv, Color32::WHITE);
            }
            // Its number (the scene that picks it) or note, and its name on a dark band.
            let tag = if notes_pick { note_name(i) } else { (i + 1).to_string() };
            let g = p.text(pad.min + vec2(5.0, 4.0), Align2::LEFT_TOP, &tag, small(), TEXT);
            p.rect_filled(g.expand(2.0), 2.0, Color32::from_black_alpha(170));
            p.text(pad.min + vec2(5.0, 4.0), Align2::LEFT_TOP, &tag, small(), TEXT);
            let band = Rect::from_min_max(pos2(pad.left(), pad.bottom() - 16.0), pad.max);
            p.rect_filled(band, egui::CornerRadius { nw: 0, ne: 0, sw: 4, se: 4 }, Color32::from_black_alpha(170));
            p.with_clip_rect(band.shrink2(vec2(4.0, 0.0))).text(pos2(band.left() + 5.0, band.center().y), Align2::LEFT_CENTER, &c.name, small(), TEXT);
            let status = if info.needs_ffmpeg {
                Some(("needs ffmpeg", ACCENT))
            } else if info.error.is_some() {
                Some(("cannot read", REC))
            } else {
                None
            };
            if let Some((s, col)) = status {
                p.text(pos2(pad.center().x, pad.top() + 28.0), Align2::CENTER_CENTER, s, small(), col);
            }
            // In a corner: cut by Slice to Drum Rack (its cues), or the parts it keeps.
            let corner = match (&c.cues, c.is_cut()) {
                (Some(q), _) => Some(format!("{} cues", q.len())),
                (None, true) => Some(format!("{} part{}", c.parts.len(), if c.parts.len() == 1 { "" } else { "s" })),
                _ => None,
            };
            if let Some(s) = corner {
                let at = pos2(pad.right() - 5.0, pad.top() + 4.0);
                let g = p.text(at, Align2::RIGHT_TOP, &s, small(), TEXT);
                p.rect_filled(g.expand(2.0), 2.0, Color32::from_black_alpha(170));
                p.text(at, Align2::RIGHT_TOP, &s, small(), TEXT);
            }
            // Still being read: how far.
            if let Some(m) = &m {
                if !m.is_done() && info.expected > 0 {
                    let f = (m.frame_count() as f32 / info.expected as f32).clamp(0.0, 1.0);
                    let bar = Rect::from_min_size(pos2(pad.left() + 4.0, band.top() - 5.0), vec2((pad.width() - 8.0) * f, 3.0));
                    p.rect_filled(bar, 1.0, ACCENT);
                } else if !m.is_done() {
                    p.text(pos2(pad.center().x, pad.top() + 28.0), Align2::CENTER_CENTER, "reading...", small(), TEXT_DIM);
                }
            }
            // Playing: lit; waiting for its beat: blinking; the one edited: underlined.
            if playing == i {
                rect_line(p, pad, 4.0, 2.0, ACCENT);
            } else if pending == Some(i) {
                let blink = (ui.input(|inp| inp.time) * 4.0) as i64 % 2 == 0;
                rect_line(p, pad, 4.0, 2.0, if blink { ACCENT } else { LINE });
                ui.ctx().request_repaint_after(std::time::Duration::from_millis(120));
            } else {
                rect_line(p, pad, 4.0, 1.0, LINE);
            }
            if edited == i {
                p.rect_filled(Rect::from_min_size(pos2(pad.left() + 6.0, pad.bottom() + 2.0), vec2(pad.width() - 12.0, 2.0)), 1.0, SELECT);
            }
            let about = match (&info.error, info.note.is_empty()) {
                (Some(e), _) => format!("{}\n{e}", c.path.display()),
                (None, false) => format!("{}\n{}", c.path.display(), info.note),
                (None, true) => c.path.display().to_string(),
            };
            let film = kv::is_film(&c.path);
            let resp = resp.on_hover_text(format!("{about}\nClick: play it and edit it (underlined) · drag: move it · right-click: more"));
            if resp.clicked() {
                launch = Some(i);
            }
            if resp.drag_started() {
                ui.ctx().data_mut(|d| d.insert_temp(drag_id, i));
            }
            if resp.drag_stopped() {
                if let Some(pt) = ui.ctx().pointer_latest_pos() {
                    let to = (((pt.x - r.left()) / (pw + gap)).floor().max(0.0) as usize).min(clips.len() - 1);
                    if to != i {
                        moved = Some((i, to));
                    }
                }
                ui.ctx().data_mut(|d| d.remove::<usize>(drag_id));
            }
            resp.context_menu(|ui| {
                if film && ui.button("Slice to Drum Rack...").on_hover_text("Its sound cut at its hits or on the beat onto the pads of a new Drum Rack track, and its picture cut at the same places: the track's notes play both, in sync").clicked() {
                    slice = Some(c.path.clone());
                    ui.close();
                }
                if ui.add_enabled(clips.len() < kv::MAX_CLIPS, egui::Button::new("Duplicate")).on_hover_text("The same film again, next to it: other parts, another way of playing").clicked() {
                    duplicate = Some(i);
                    ui.close();
                }
                if ui.button("Replace...").clicked() {
                    replace = Some(i);
                    ui.close();
                }
                if ui.button("Remove").clicked() {
                    remove = Some(i);
                    ui.close();
                }
            });
        }
        // Where a dragged pad goes.
        if let (Some(from), Some(pt)) = (dragging, ui.ctx().pointer_latest_pos().filter(|_| ui.ctx().dragged_id().is_some())) {
            let to = (((pt.x - r.left()) / (pw + gap)).floor().max(0.0) as usize).min(clips.len().saturating_sub(1));
            if to != from {
                let x = if to > from { pad_at(to).right() + gap * 0.5 } else { pad_at(to).left() - gap * 0.5 };
                ui.painter().line_segment([pos2(x, r.top()), pos2(x, r.top() + ph)], Stroke::new(3.0_f32, ACCENT));
            }
        }
        // The last pad: a file, or a film from a link (or how its film is coming).
        if room {
            let pad = pad_at(clips.len());
            let half = (pad.height() - 4.0) * 0.5;
            let file = Rect::from_min_size(pad.min, vec2(pad.width(), half));
            let link = Rect::from_min_size(pos2(pad.left(), pad.bottom() - half), vec2(pad.width(), half));
            let resp = ui.interact(file, Id::new("video_pad_add"), Sense::click());
            let p = ui.painter();
            let hot = resp.hovered();
            rect_line(p, file, 4.0, 1.0, if hot { ACCENT } else { LINE });
            p.with_clip_rect(file).text(file.center(), Align2::CENTER_CENTER, "+ Add file", body(), if hot { TEXT } else { TEXT_DIM });
            if resp.on_hover_text("Films (with ffmpeg), pictures (PNG, JPEG) or animated GIFs. Or drop them on this view").clicked() {
                if let Some(paths) = rfd::FileDialog::new().set_title("Films, pictures or GIFs for the Video").add_filter("Films, pictures and GIFs", &media::EXTENSIONS).pick_files() {
                    self.video_add(&paths);
                }
            }
            let grab = self.video.grab.as_ref().and_then(|g| g.lock().ok().map(|s| s.clone()));
            let resp = ui.interact(link, Id::new("video_pad_link"), Sense::click());
            let p = ui.painter();
            let busy = grab.is_some();
            let hot = resp.hovered() || self.video.link_open || busy;
            rect_line(p, link, 4.0, 1.0, if hot { ACCENT } else { LINE });
            match &grab {
                Some(g) => {
                    let (what, f) = match g {
                        ytdl::Grab::Tool => ("Getting yt-dlp...".to_string(), 0.0),
                        ytdl::Grab::Film(f) if *f >= 0.999 => ("Almost there...".to_string(), 1.0),
                        ytdl::Grab::Film(f) => (format!("Fetching {:.0} %", f * 100.0), *f),
                        _ => (String::new(), 1.0),
                    };
                    p.with_clip_rect(link).text(link.center() - vec2(0.0, 3.0), Align2::CENTER_CENTER, what, small(), TEXT);
                    let bar = Rect::from_min_size(pos2(link.left() + 6.0, link.bottom() - 6.0), vec2((link.width() - 12.0) * f, 2.0));
                    p.rect_filled(bar, 1.0, ACCENT);
                }
                None => {
                    p.with_clip_rect(link).text(link.center(), Align2::CENTER_CENTER, "+ From a link", body(), if resp.hovered() { TEXT } else { TEXT_DIM });
                }
            }
            let tip = if busy { "A film is being fetched from a link" } else { "A film from a link (YouTube and the many sites yt-dlp knows): paste it, and it comes into the bank" };
            if resp.on_hover_text(tip).clicked() && !busy {
                self.video.link_open = !self.video.link_open;
            }
            if self.video.link_open && !busy {
                self.video_link_box(ui.ctx(), pos2(pad.left(), pad.bottom() + 6.0));
            }
        } else if self.video.link_open {
            self.video.link_open = false;
        }
        if let Some(i) = launch {
            self.video_launch(i);
        }
        if let Some((from, to)) = moved {
            self.video_move_clip(from, to);
        }
        if let Some(path) = slice {
            self.open_film_slice_dialog(&path);
        }
        if let Some(i) = duplicate {
            let set = &mut self.project.video;
            if set.clips.len() < kv::MAX_CLIPS {
                let mut c = set.clips[i].clone();
                c.name = format!("{} 2", c.name);
                set.clips.insert(i + 1, c);
                self.video_pick(i + 1);
            }
        }
        if let Some(i) = replace {
            if let Some(path) = rfd::FileDialog::new().set_title("A film, picture or GIF instead").add_filter("Films, pictures and GIFs", &media::EXTENSIONS).pick_file() {
                if let Some(c) = self.project.video.clips.get_mut(i) {
                    *c = kv::VideoClip { play: c.play.clone(), ..kv::VideoClip::new(path) };
                    self.video_pick(i);
                }
            }
        }
        if let Some(i) = remove {
            let set = &mut self.project.video;
            set.clips.remove(i);
            set.current = set.current.min(set.clips.len().saturating_sub(1));
            let c = set.current;
            self.video.play_now(c);
        }
    }

    /// The Look pads, two rows of four: click one to change to it (and
    /// edit it in the Look tab), right-click for more; the last pad makes a
    /// new Look. A pad says its effects under its name.
    fn video_look_pads(&mut self, ui: &mut Ui, r: Rect, set: &mut kv::VideoSet) {
        let playing = self.video.dir.look_playing();
        let pending = self.video.dir.pads.pending.map(|p| p.0);
        let own = self.video.dir.pads.clip;
        let n = set.looks.len();
        let slots = (n + 1).min(kv::MAX_LOOKS);
        let (cols, gap) = (4, 6.0);
        let w = (r.width() - gap * (cols as f32 - 1.0)) / cols as f32;
        let h = ((r.height() - gap) / 2.0).max(20.0);
        let mut launch = None;
        let mut act: Option<(usize, &str)> = None;
        for k in 0..slots {
            let pad = Rect::from_min_size(pos2(r.left() + (k % cols) as f32 * (w + gap), r.top() + (k / cols) as f32 * (h + gap)), vec2(w, h));
            let resp = ui.interact(pad, Id::new(("video_look_pad", k)), Sense::click());
            let p = ui.painter();
            if k == n {
                rect_line(p, pad, 4.0, 1.0, if resp.hovered() { ACCENT } else { LINE });
                p.text(pad.center(), Align2::CENTER_CENTER, "+ Look", body(), if resp.hovered() { TEXT } else { TEXT_DIM });
                if resp.on_hover_text("A new Look: a copy of the one edited, to change").clicked() {
                    act = Some((k, "new"));
                }
                continue;
            }
            let lit = playing == k;
            rect_fill(p, pad, 4.0, if lit { ACCENT } else if resp.hovered() { PANEL3 } else { PANEL2 });
            let (col, dim) = if lit { (TEXT_DARK, TEXT_DARK) } else { (TEXT, TEXT_DIM) };
            let pc = p.with_clip_rect(pad.shrink(3.0));
            let two = h >= 32.0;
            let name_y = if two { pad.top() + h * 0.36 } else { pad.center().y };
            pc.text(pos2(pad.left() + 8.0, name_y), Align2::LEFT_CENTER, look_label(k, &set.looks[k]), body(), col);
            pc.text(pos2(pad.right() - 5.0, pad.top() + 3.0), Align2::RIGHT_TOP, (k + 1).to_string(), egui::FontId::proportional(9.0), dim);
            if two {
                let fx: Vec<&str> = set.looks[k].effects().iter().map(|e| e.name()).collect();
                let said = if fx.is_empty() { "no effects".to_string() } else { fx.join(" · ") };
                pc.text(pos2(pad.left() + 8.0, pad.top() + h * 0.74), Align2::LEFT_CENTER, said, small(), dim);
            }
            if pending == Some(k) {
                let blink = (ui.input(|inp| inp.time) * 4.0) as i64 % 2 == 0;
                rect_line(p, pad, 4.0, 2.0, if blink { ACCENT } else { LINE });
                ui.ctx().request_repaint_after(std::time::Duration::from_millis(120));
            } else if own == k && !lit {
                rect_line(p, pad, 4.0, 1.5, ACCENT.gamma_multiply(0.6));
            }
            if set.look == k {
                p.rect_filled(Rect::from_min_size(pos2(pad.left() + 6.0, pad.bottom() + 1.0), vec2(pad.width() - 12.0, 2.0)), 1.0, SELECT);
            }
            let resp = resp.on_hover_text("Click: this Look now (or on its beat), edited in the Look tab (underlined) · lit while it plays · right-click: more");
            if resp.clicked() {
                launch = Some(k);
            }
            resp.context_menu(|ui| {
                ui.add(egui::TextEdit::singleline(&mut set.looks[k].name).hint_text("Its name").desired_width(140.0));
                if ui.add_enabled(n < kv::MAX_LOOKS, egui::Button::new("Duplicate")).clicked() {
                    act = Some((k, "dup"));
                    ui.close();
                }
                if ui.add_enabled(n > 1, egui::Button::new("Delete")).clicked() {
                    act = Some((k, "del"));
                    ui.close();
                }
            });
        }
        match act {
            Some((_, "new")) if n < kv::MAX_LOOKS => {
                let mut l = set.edited_look().clone();
                l.name = format!("Look {}", n + 1);
                set.looks.push(l);
                set.look = n;
            }
            Some((k, "dup")) if n < kv::MAX_LOOKS => {
                let mut l = set.looks[k].clone();
                l.name = format!("{} 2", l.name);
                set.looks.insert(k + 1, l);
                renumber_looks(set, |i| if i > k { i + 1 } else { i });
                set.look = k + 1;
            }
            Some((k, "del")) if n > 1 => {
                set.looks.remove(k);
                renumber_looks(set, |i| if i > k { i - 1 } else { i.min(n - 2) });
                set.look = set.look.min(set.looks.len() - 1);
            }
            _ => {}
        }
        if let Some(k) = launch {
            set.look = k;
            let st = self.status();
            let (beat, playing) = (st.song_pos.load(), st.playing.load(std::sync::atomic::Ordering::Relaxed));
            self.video.dir.launch_look(set, k, beat, playing, self.project.beats_per_bar);
        }
    }

    /// Puts clip `from` of the bank at `to` (the others move up or down);
    /// the clip edited and the one playing stay themselves.
    pub(crate) fn video_move_clip(&mut self, from: usize, to: usize) {
        let set = &mut self.project.video;
        if from >= set.clips.len() || to >= set.clips.len() || from == to {
            return;
        }
        let c = set.clips.remove(from);
        set.clips.insert(to, c);
        let place = |i: usize| {
            if i == from {
                to
            } else if from < i && i <= to {
                i - 1
            } else if to <= i && i < from {
                i + 1
            } else {
                i
            }
        };
        set.current = place(set.current);
        self.video.dir.renumber(place);
        self.video.editing = Some(set.current);
    }

    /// The box to paste a link in, under its pad.
    fn video_link_box(&mut self, ctx: &Context, at: egui::Pos2) {
        let mut get = false;
        let mut close = false;
        egui::Area::new(Id::new("video_link_box")).order(egui::Order::Foreground).fixed_pos(at).show(ctx, |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.set_width(420.0);
                ui.label(RichText::new("A film from a link").strong().color(TEXT));
                let edit = ui.add(egui::TextEdit::singleline(&mut self.video.link).hint_text("https://www.youtube.com/watch?v=...").desired_width(f32::INFINITY));
                edit.request_focus();
                if edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    get = true;
                }
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    close = true;
                }
                ui.checkbox(&mut self.video.link_sound, "Its sound on a new audio track too");
                ui.horizontal(|ui| {
                    if ui.add_enabled(ytdl::is_link(&self.video.link), egui::Button::new("Get it")).clicked() {
                        get = true;
                    }
                    if ui.button("Cancel").clicked() {
                        close = true;
                    }
                    if ui.button("Paste").on_hover_text("The link on the clipboard").clicked() {
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::RequestPaste);
                    }
                    if ui.button("Open folder").on_hover_text(format!("Where the films go: {}", ytdl::films_dir().display())).clicked() {
                        ytdl::open_films_dir();
                    }
                });
                ui.label(RichText::new("Fetched with yt-dlp (it comes the first time), up to 720p, into Documents/Krabton/Films. Only take what you may use").small().color(TEXT_DIM));
            });
        });
        if get {
            self.video_fetch_link();
        } else if close {
            self.video.link_open = false;
        }
    }

    // ------------------------------------------------------------------
    // The picture and its timeline
    // ------------------------------------------------------------------

    /// What the mouse does on the timeline now, said under it.
    fn video_timeline_hint(&self) -> String {
        let Some((_, clip, dur, _)) = self.video_edited() else { return String::new() };
        if dur <= 0.0 {
            String::new()
        } else if clip.is_cued() {
            "Cut by Slice to Drum Rack: the whole film plays, by its cues (the lines). Click to put the film there".to_string()
        } else if clip.parts.is_empty() {
            "The whole film plays. Drag on it to keep a part · click: put the film there · wheel: zoom".to_string()
        } else {
            "Drag a gap: another part · drag a part: move it · its edges: trim · Shift-click: pick more · right-click: more · double-click a gap: whole film".to_string()
        }
    }

    /// Before there is a clip: how to start, in the picture.
    fn video_welcome(&mut self, ui: &mut Ui, h: f32) {
        ui.add_space((h * 0.12).max(4.0));
        ui.label(RichText::new("Bring in a film, a picture or a GIF").color(TEXT).size(18.0));
        ui.add_space(2.0);
        ui.label(RichText::new("It plays along with the music: the kicks, the bass and the beat move it").color(TEXT_DIM));
        ui.add_space(12.0);
        let open = egui::Button::new(RichText::new("+ Add file...").color(TEXT_DARK).size(14.0)).fill(ACCENT).min_size(vec2(220.0, 30.0));
        if ui.add(open).on_hover_text("Films (MP4, MOV, WebM and more), pictures (PNG, JPEG) or animated GIFs").clicked() {
            if let Some(paths) = rfd::FileDialog::new().set_title("Films, pictures or GIFs for the Video").add_filter("Films, pictures and GIFs", &media::EXTENSIONS).pick_files() {
                self.video_add(&paths);
            }
        }
        ui.add_space(4.0);
        let link = egui::Button::new(RichText::new("+ From a link (YouTube and more)...").size(14.0)).fill(PANEL3).min_size(vec2(220.0, 30.0));
        if ui.add(link).on_hover_text("Paste a link: the film comes into the bank (fetched with yt-dlp, up to 720p)").clicked() {
            self.video.link_open = true;
        }
        ui.add_space(4.0);
        let vis = egui::Button::new(RichText::new("Visuals: no film needed").size(14.0)).fill(PANEL3).min_size(vec2(220.0, 30.0));
        if ui.add(vis).on_hover_text("Krabton draws the pictures itself, moving with the music: ridges, a lattice, a swarm, data, a tunnel, the sound's wave, contours, shards, the notes, strings. Pick another in the Look tab's presets").clicked() {
            self.video_visuals_preset("Lattice");
        }
        ui.add_space(6.0);
        ui.label(RichText::new("or drop files anywhere on this view").small().color(TEXT_DIM));
        if ffmpeg::find().is_none() {
            ui.add_space(12.0);
            ui.label(RichText::new("Films need ffmpeg, a free program. Pictures and GIFs play without it.").small().color(TEXT_DIM));
            ui.add_space(4.0);
            self.ffmpeg_fetch_row(ui);
        }
    }

    /// The Look edited becomes preset `name` (its links fitted to the tracks).
    pub(crate) fn video_visuals_preset(&mut self, name: &str) {
        if let Some(pr) = kv::preset(name) {
            self.video_preset(&pr);
        }
    }

    /// The film edited: its length, whether it is the one that plays.
    fn video_edited(&self) -> Option<(usize, kv::VideoClip, f32, bool)> {
        let set = &self.project.video;
        let i = set.current;
        let c = set.clips.get(i)?.clone();
        let dur = self.video.media.get(&c.path).map_or(0.0, |m| m.info().duration);
        Some((i, c, dur, self.video.playing() == i))
    }

    /// The film's frames a second (for a frame step).
    fn video_fps(&self, i: usize) -> f64 {
        let info = self.project.video.clips.get(i).and_then(|c| self.video.media.get(&c.path)).map(|m| m.info()).unwrap_or_default();
        if info.duration > 0.0 && info.expected > 0 {
            info.expected as f64 / info.duration as f64
        } else {
            25.0
        }
    }

    /// The buttons over the timeline: pause, a frame back and on, where it
    /// is, Mark In and Out, split and delete, the zoom.
    fn video_transport(&mut self, ui: &mut Ui) {
        let Some((_, clip, dur, live)) = self.video_edited() else { return };
        ui.spacing_mut().item_spacing.x = 4.0;
        let cued = clip.is_cued();
        let paused = self.video.paused;
        if choice(ui, if paused { "Play" } else { "Pause" }, paused, "Stops the film where it is, to find a place (K). The music goes on").clicked() {
            self.video_key_action(VideoAct::Pause);
        }
        if ui.small_button("<").on_hover_text("A frame back (,)").clicked() {
            self.video_key_action(VideoAct::Step(-1));
        }
        if ui.small_button(">").on_hover_text("A frame on (.)").clicked() {
            self.video_key_action(VideoAct::Step(1));
        }
        let shown = if live { format!("{} / {}", clock(self.video.at), clock(dur as f64)) } else { format!("- / {}", clock(dur as f64)) };
        ui.label(RichText::new(shown).font(mono()).color(if live { TEXT } else { TEXT_DIM })).on_hover_text(if live { "Where the film is, of how long it is" } else { "Another clip plays now: click this one's timeline or its pad to play it" });
        ui.separator();
        let can_mark = live && dur > 0.0 && !cued;
        let marking = self.video.mark_in.is_some();
        if ui.add_enabled(can_mark, egui::Button::new(RichText::new("Mark In").color(if marking { TEXT_DARK } else { TEXT })).fill(if marking { ACCENT } else { PANEL3 })).on_hover_text("A part starts here (I). Until Mark Out the film plays whole, so what was skipped can be marked too").clicked() {
            self.video_key_action(VideoAct::MarkIn);
        }
        if ui.add_enabled(can_mark, egui::Button::new("Mark Out")).on_hover_text("The part ends here (O): from Mark In, or the end of the part the film is in").clicked() {
            self.video_key_action(VideoAct::MarkOut);
        }
        if ui.add_enabled(live && !cued && clip.part_at(self.video.at as f32).is_some(), egui::Button::new("Split")).on_hover_text("Cuts the part the film is in, here (S)").clicked() {
            self.video_key_action(VideoAct::Split);
        }
        let picked = !self.video.picked.is_empty();
        if ui.add_enabled(picked && !cued, egui::Button::new("Delete")).on_hover_text("Takes the parts picked away (Delete): that bit of the film is skipped").clicked() {
            self.video_key_action(VideoAct::Delete);
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.small_button("+").on_hover_text("Closer in (the wheel over the timeline)").clicked() {
                self.video_zoom(dur, 0.5, None);
            }
            if ui.small_button("All").on_hover_text("The whole film on the timeline").clicked() {
                self.video.view = (0.0, 0.0);
            }
            if ui.small_button("-").on_hover_text("Further out").clicked() {
                self.video_zoom(dur, 2.0, None);
            }
        });
    }

    /// The timeline seen `by` times as long, round `at` (seconds; None: its middle).
    fn video_zoom(&mut self, dur: f32, by: f32, at: Option<f32>) {
        if dur <= 0.0 {
            return;
        }
        let (start, len) = if self.video.view.1 <= 0.0 { (0.0, dur) } else { self.video.view };
        let at = at.unwrap_or(start + len * 0.5);
        let new_len = (len * by).clamp((dur / 400.0).max(0.5).min(dur), dur);
        let f = (at - start) / len.max(1e-6);
        let new_start = (at - f * new_len).clamp(0.0, (dur - new_len).max(0.0));
        self.video.view = if new_len >= dur - 1e-3 { (0.0, 0.0) } else { (new_start, new_len) };
    }

    /// The timeline of the clip edited: its whole film as a strip of
    /// frames (zoom with the wheel), the parts that play lit, the gaps
    /// dark. Drag on it to keep a part, drag a part to move it, its edges
    /// to trim it; click to put the film there (and pick the part, Shift
    /// for more); right-click a part for more; double-click a gap for the
    /// whole film again. The ruler on top moves the film when dragged.
    fn video_timeline(&mut self, ui: &mut Ui, r: Rect) {
        let Some((i, clip, dur, live)) = self.video_edited() else { return };
        let p = ui.painter().clone();
        rect_fill(&p, r, 3.0, PANEL2);
        let m = self.video.media.get(&clip.path).cloned();
        if dur <= 0.0 {
            let what = if m.as_ref().is_some_and(|m| !m.is_done()) { "Reading the film..." } else { "A picture has no timeline: it shows as it is" };
            p.text(r.center(), Align2::CENTER_CENTER, what, small(), TEXT_DIM);
            return;
        }
        let ruler = Rect::from_min_size(r.min + vec2(4.0, 2.0), vec2(r.width() - 8.0, 14.0));
        let strip_r = Rect::from_min_size(pos2(ruler.left(), ruler.bottom() + 2.0), vec2(ruler.width(), r.height() - 30.0));
        let scroll_r = Rect::from_min_size(pos2(ruler.left(), strip_r.bottom() + 3.0), vec2(ruler.width(), 6.0));
        let (vs, vl) = if self.video.view.1 <= 0.0 { (0.0, dur) } else { (self.video.view.0.clamp(0.0, dur), self.video.view.1.min(dur)) };
        let x_of = |t: f32| strip_r.left() + (t - vs) / vl * strip_r.width();
        let t_of = |x: f32| (vs + (x - strip_r.left()) / strip_r.width() * vl).clamp(0.0, dur);
        let cued = clip.is_cued();

        // The ruler.
        let step = ruler_step(strip_r.width() / vl, 70.0);
        let mut t = (vs / step).floor() * step;
        let pc = p.with_clip_rect(r);
        while t <= vs + vl {
            let x = x_of(t);
            pc.line_segment([pos2(x, ruler.bottom() - 4.0), pos2(x, ruler.bottom())], Stroke::new(1.0_f32, TEXT_DIM));
            pc.text(pos2(x + 3.0, ruler.center().y - 1.0), Align2::LEFT_CENTER, clock(t as f64), small(), TEXT_DIM);
            t += step;
        }

        // The film's frames.
        let ps = p.with_clip_rect(strip_r);
        rect_fill(&ps, strip_r, 2.0, Color32::BLACK);
        if let Some(m) = &m {
            let info = m.info();
            let aspect = if info.height > 0 { info.width as f32 / info.height as f32 } else { 16.0 / 9.0 };
            let tile_w = (strip_r.height() * aspect).clamp(24.0, 200.0);
            let strip = self.video.strips.entry(clip.path.clone()).or_insert_with(|| strip::Strip::new(m.clone()));
            let tiles = (strip_r.width() / tile_w).ceil() as usize;
            for k in 0..tiles {
                let tile = Rect::from_min_size(pos2(strip_r.left() + k as f32 * tile_w, strip_r.top()), vec2(tile_w, strip_r.height()));
                let mid = t_of(tile.center().x.min(strip_r.right()));
                let Some(f) = m.frame_at(mid as f64) else { continue };
                match strip.get(f) {
                    Some(tex) => {
                        ps.image(tex.id(), tile.shrink2(vec2(0.5, 0.0)), Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
                    }
                    None => rect_fill(&ps, tile.shrink(1.0), 0.0, PANEL3),
                }
            }
            strip.frame(ui.ctx());
        }

        // The gaps dark, the parts lit, their numbers (or notes).
        let parts = if cued { Vec::new() } else { clip.parts.clone() };
        let spans = clip.spans(dur);
        if !parts.is_empty() {
            let mut from = 0.0;
            for &(a, b) in spans.iter().chain(std::iter::once(&(dur, dur))) {
                if a > from {
                    ps.rect_filled(Rect::from_min_max(pos2(x_of(from), strip_r.top()), pos2(x_of(a), strip_r.bottom())), 0.0, Color32::from_black_alpha(185));
                }
                from = b;
            }
        }
        let seq_on = clip.play.seq.on && spans.len() > 1;
        let by_part = clip.play.mode == PlayMode::Slices && clip.play.part_slices && spans.len() > 1 && !seq_on;
        let by_note = seq_on && clip.play.seq.order == kv::PartOrder::Notes;
        // The part that plays now, with a part sequence.
        let playing_part = if live && seq_on { self.video.dir.playhead.part } else { None };
        let solo = self.video.solo;
        let looks = self.project.video.looks.clone();
        for (k, &(a, b)) in parts.iter().enumerate() {
            let pr = Rect::from_min_max(pos2(x_of(a), strip_r.top()), pos2(x_of(b.min(dur)), strip_r.bottom()));
            let picked = self.video.picked.contains(&k);
            if picked {
                ps.rect_filled(pr, 0.0, ACCENT.gamma_multiply(0.18));
            }
            if playing_part == Some(k) {
                ps.rect_filled(pr, 0.0, Color32::WHITE.gamma_multiply(0.12));
                rect_line(&ps, pr.shrink(2.0), 2.0, 2.0, Color32::WHITE);
            }
            rect_line(&ps, pr, 2.0, if picked { 2.5 } else { 1.5 }, if picked { SELECT } else { ACCENT });
            let label = if by_part { note_name(k) } else if by_note { note_name(k + 24) } else { (k + 1).to_string() };
            let label = if solo == Some(k) { format!("{label} alone") } else { label };
            // Its own Look, by name.
            let label = match clip.part_look(k).and_then(|l| looks.get(l).map(|look| look_label(l, look))) {
                Some(name) => format!("{label} · {}", short(&name, 10)),
                None => label,
            };
            let g = ps.text(pr.min + vec2(4.0, 3.0), Align2::LEFT_TOP, &label, small(), TEXT_DARK);
            ps.rect_filled(g.expand(2.0), 2.0, if picked { SELECT } else { ACCENT });
            ps.text(pr.min + vec2(4.0, 3.0), Align2::LEFT_TOP, &label, small(), TEXT_DARK);
        }
        // Where the slices are cut (Slices), or the cues.
        if let Some(q) = clip.cues.as_ref().filter(|_| cued) {
            for t in &q.times {
                let x = x_of(*t as f32);
                ps.line_segment([pos2(x, strip_r.top()), pos2(x, strip_r.bottom())], Stroke::new(1.0_f32, ACCENT));
            }
        } else if clip.play.mode == PlayMode::Slices && !by_part {
            let n = clip.slice_count(&spans) as usize;
            for k in 1..n.min(64) {
                let x = x_of(kv::film_time(&spans, clip.slice_bounds(&spans, k).0) as f32);
                ps.line_segment([pos2(x, strip_r.top() + strip_r.height() * 0.6), pos2(x, strip_r.bottom())], Stroke::new(1.0_f32, TEXT.gamma_multiply(0.7)));
            }
        }
        // Mark In, and a part being dragged out.
        if let Some(a) = self.video.mark_in {
            let x = x_of(a);
            pc.line_segment([pos2(x, ruler.top()), pos2(x, strip_r.bottom())], Stroke::new(2.0_f32, ACCENT));
            let g = pc.text(pos2(x + 3.0, strip_r.bottom() - 3.0), Align2::LEFT_BOTTOM, "In", small(), TEXT_DARK);
            pc.rect_filled(g.expand(1.0), 2.0, ACCENT);
            pc.text(pos2(x + 3.0, strip_r.bottom() - 3.0), Align2::LEFT_BOTTOM, "In", small(), TEXT_DARK);
        }
        let drag_id = Id::new("video_timeline_drag");
        let drag: Option<TimeDrag> = ui.ctx().data(|d| d.get_temp(drag_id));
        // The playhead (of the film that plays, when it is this one).
        if live {
            let x = x_of(self.video.at as f32);
            if (strip_r.left() - 1.0..=strip_r.right() + 1.0).contains(&x) {
                pc.line_segment([pos2(x, ruler.top()), pos2(x, strip_r.bottom())], Stroke::new(1.5_f32, Color32::WHITE));
                pc.add(egui::Shape::convex_polygon(vec![pos2(x - 5.0, ruler.top()), pos2(x + 5.0, ruler.top()), pos2(x, ruler.top() + 7.0)], Color32::WHITE, Stroke::NONE));
            }
        }

        // The scroll bar, when zoomed in.
        if vl < dur - 1e-3 {
            rect_fill(&p, scroll_r, 3.0, PANEL3);
            let thumb = Rect::from_min_max(pos2(scroll_r.left() + vs / dur * scroll_r.width(), scroll_r.top()), pos2(scroll_r.left() + (vs + vl) / dur * scroll_r.width(), scroll_r.bottom()));
            let sr = ui.interact(scroll_r, Id::new("video_tl_scroll"), Sense::click_and_drag());
            rect_fill(&p, thumb, 3.0, if sr.dragged() || sr.hovered() { TEXT } else { TEXT_DIM });
            if sr.dragged() || sr.clicked() {
                if let Some(pt) = sr.interact_pointer_pos() {
                    let c = (pt.x - scroll_r.left()) / scroll_r.width() * dur;
                    self.video.view.0 = (c - vl * 0.5).clamp(0.0, dur - vl);
                }
            }
        }

        // The wheel: closer in or further out round the pointer; sideways moves along.
        let over = ui.rect_contains_pointer(r);
        if over {
            let (scroll, at) = ui.input(|inp| (inp.smooth_scroll_delta, inp.pointer.hover_pos()));
            if scroll.y.abs() > 0.1 {
                self.video_zoom(dur, (-scroll.y * 0.004).exp(), at.map(|pt| t_of(pt.x)));
            }
            if scroll.x.abs() > 0.1 && self.video.view.1 > 0.0 {
                let (s, l) = self.video.view;
                self.video.view.0 = (s - scroll.x / strip_r.width() * l).clamp(0.0, dur - l);
            }
        }

        // The ruler: drag (or click) to put the film there.
        let rr = ui.interact(ruler.expand2(vec2(0.0, 2.0)), Id::new("video_tl_ruler"), Sense::click_and_drag()).on_hover_text("Drag to move the film");
        if rr.clicked() || rr.dragged() {
            if let Some(pt) = rr.interact_pointer_pos() {
                self.video_seek_to(i, t_of(pt.x) as f64, live);
            }
        }

        let set_parts = |app: &mut App, parts: Vec<(f32, f32)>| {
            if let Some(c) = app.project.video.clips.get_mut(i) {
                if parts.is_empty() {
                    c.part_looks.clear();
                }
                c.parts = parts;
            }
        };
        let resp = ui.interact(strip_r, Id::new("video_tl_strip"), Sense::click_and_drag());
        if cued {
            if resp.clicked() {
                if let Some(pt) = resp.interact_pointer_pos() {
                    self.video_seek_to(i, t_of(pt.x) as f64, live);
                }
            }
            return;
        }
        // The parts where they are drawn: a press near an edge trims it, inside moves it.
        let xs: Vec<(f32, f32)> = parts.iter().map(|&(a, b)| (x_of(a), x_of(b.min(dur)))).collect();
        let hit_part = |x: f32| -> Option<(usize, PartGrab)> { hit_parts(x, &xs) };
        // The pointer's shape says what a drag does, and the edge it takes lights up.
        let held: Option<TimeDrag> = drag.filter(|_| resp.dragged());
        let lit = match held {
            Some(TimeDrag::Edge { k, end, .. }) => Some((k, if end { PartGrab::Right } else { PartGrab::Left })),
            Some(TimeDrag::Move { k, .. }) => Some((k, PartGrab::Move)),
            _ => resp.hover_pos().and_then(|pt| hit_part(pt.x)),
        };
        match lit.map(|h| h.1) {
            Some(PartGrab::Left | PartGrab::Right) => ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal),
            Some(PartGrab::Move) if held.is_some() => ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing),
            Some(PartGrab::Move) => ui.ctx().set_cursor_icon(egui::CursorIcon::Grab),
            None => {}
        }
        // The handles on every part's edges (the one under the pointer brighter).
        for (k, &(xa, xb)) in xs.iter().enumerate() {
            let h = strip_r.height() * 0.5;
            let y = strip_r.center().y;
            let w = (xb - xa).max(0.0);
            for (edge, x) in [(PartGrab::Left, xa + 3.0_f32.min(w * 0.25)), (PartGrab::Right, xb - 3.0_f32.min(w * 0.25))] {
                let on = lit == Some((k, edge));
                let col = if on { Color32::WHITE } else { ACCENT };
                ps.rect_filled(Rect::from_center_size(pos2(x, y), vec2(if on { 4.0 } else { 3.0 }, h)), 1.5, col);
            }
        }
        let bounds = |k: Option<usize>, t: f32| -> (f32, f32) {
            // The room between the neighbours of part `k` (or of the gap at `t`).
            let lo = parts.iter().enumerate().filter(|(j, p)| Some(*j) != k && p.1 <= t + 1e-4).map(|(_, p)| p.1).fold(0.0, f32::max);
            let hi = parts.iter().enumerate().filter(|(j, p)| Some(*j) != k && p.0 >= t - 1e-4).map(|(_, p)| p.0).fold(dur, f32::min);
            (lo, hi.max(lo))
        };
        if resp.drag_started() {
            // Where the button went down, not where the pointer is now: egui
            // only calls it a drag once it has moved a few pixels, by when an
            // edge grabbed is already behind it (that moved the part instead).
            let origin = ui.input(|inp| inp.pointer.press_origin()).or(resp.interact_pointer_pos());
            if let Some(pt) = origin {
                let t = t_of(pt.x);
                let d = match hit_part(pt.x) {
                    Some((k, edge @ (PartGrab::Left | PartGrab::Right))) => {
                        let (a, b) = parts[k];
                        let (lo, hi) = bounds(Some(k), (a + b) * 0.5);
                        let end = edge == PartGrab::Right;
                        TimeDrag::Edge { k, end, off: t - if end { b.min(dur) } else { a }, lo, hi }
                    }
                    Some((k, PartGrab::Move)) => {
                        let (a, b) = parts[k];
                        let (lo, hi) = bounds(Some(k), (a + b) * 0.5);
                        self.video.picked = vec![k];
                        TimeDrag::Move { k, grab: t - a, lo, hi }
                    }
                    None => {
                        let (lo, hi) = bounds(None, t);
                        TimeDrag::New { from: t, lo, hi }
                    }
                };
                ui.ctx().data_mut(|dd| dd.insert_temp(drag_id, d));
            }
        }
        if let (true, Some(d), Some(pt)) = (resp.dragged(), drag, resp.interact_pointer_pos()) {
            let t = t_of(pt.x);
            match d {
                TimeDrag::New { from, lo, hi } => {
                    let (a, b) = (from.min(t).clamp(lo, hi), from.max(t).clamp(lo, hi));
                    let nr = Rect::from_min_max(pos2(x_of(a), strip_r.top()), pos2(x_of(b), strip_r.bottom()));
                    ps.rect_filled(nr, 0.0, ACCENT.gamma_multiply(0.25));
                    rect_line(&ps, nr, 2.0, 1.5, ACCENT);
                    ps.text(nr.center_bottom() - vec2(0.0, 3.0), Align2::CENTER_BOTTOM, format!("{:.1} s", b - a), small(), TEXT);
                }
                TimeDrag::Move { k, grab, lo, hi } => {
                    let mut ps2 = parts.clone();
                    let len = ps2[k].1 - ps2[k].0;
                    let a = (t - grab).clamp(lo, (hi - len).max(lo));
                    ps2[k] = (a, a + len);
                    set_parts(self, ps2);
                }
                TimeDrag::Edge { k, end, off, lo, hi } => {
                    let mut ps2 = parts.clone();
                    let (a, b) = ps2[k];
                    let t = t - off;
                    ps2[k] = if end { (a, t.clamp(a + kv::MIN_PART, hi.max(a + kv::MIN_PART))) } else { (t.clamp(lo, (b - kv::MIN_PART).max(lo)), b) };
                    set_parts(self, ps2);
                }
            }
            ui.ctx().request_repaint();
        }
        if resp.drag_stopped() {
            if let (Some(TimeDrag::New { from, lo, hi }), Some(pt)) = (drag, resp.interact_pointer_pos()) {
                let t = t_of(pt.x);
                let (a, b) = (from.min(t).clamp(lo, hi), from.max(t).clamp(lo, hi));
                if b - a >= kv::MIN_PART {
                    if let Some(c) = self.project.video.clips.get_mut(i) {
                        c.add_part(a, b);
                        self.video.picked = c.part_at((a + b) * 0.5).into_iter().collect();
                    }
                    self.video.solo = None;
                }
            }
            ui.ctx().data_mut(|dd| dd.remove::<TimeDrag>(drag_id));
        }
        if resp.clicked() {
            if let Some(pt) = resp.interact_pointer_pos() {
                let shift = ui.input(|inp| inp.modifiers.shift);
                match hit_part(pt.x).map(|h| h.0) {
                    Some(k) if shift => {
                        if let Some(at) = self.video.picked.iter().position(|j| *j == k) {
                            self.video.picked.remove(at);
                        } else {
                            self.video.picked.push(k);
                        }
                    }
                    Some(k) => self.video.picked = vec![k],
                    None if !shift => self.video.picked.clear(),
                    None => {}
                }
                self.video_seek_to(i, t_of(pt.x) as f64, live);
            }
        }
        if resp.double_clicked() {
            if let Some(pt) = resp.interact_pointer_pos() {
                if hit_part(pt.x).is_none() {
                    set_parts(self, Vec::new());
                    self.video.picked.clear();
                    self.video.solo = None;
                }
            }
        }
        let menu_id = Id::new("video_tl_menu_part");
        if resp.secondary_clicked() {
            let k = resp.interact_pointer_pos().and_then(|pt| hit_part(pt.x)).map(|h| h.0);
            ui.ctx().data_mut(|dd| dd.insert_temp(menu_id, k));
            if let Some(k) = k {
                if !self.video.picked.contains(&k) {
                    self.video.picked = vec![k];
                }
            }
        }
        // What the mouse does here is said under the timeline (a tip would cover the meters).
        let menu_k: Option<usize> = ui.ctx().data(|dd| dd.get_temp(menu_id)).flatten();
        let here = self.video.at as f32;
        let mut act = None;
        resp.context_menu(|ui| {
            if let Some(k) = menu_k.filter(|k| *k < parts.len()) {
                let (a, b) = parts[k];
                ui.label(RichText::new(format!("Part {}: {} to {}", k + 1, clock(a as f64), clock(b.min(dur) as f64))).small().color(TEXT_DIM));
                if ui.button("Delete").clicked() {
                    act = Some(VideoAct::Delete);
                    ui.close();
                }
                if ui.add_enabled(live && here > a && here < b, egui::Button::new("Split at the playhead")).clicked() {
                    act = Some(VideoAct::Split);
                    ui.close();
                }
                let alone = self.video.solo == Some(k);
                if ui.button(if alone { "Play all parts again" } else { "Play only this" }).on_hover_text("Round and round in this part alone, to look at it (not saved)").clicked() {
                    self.video.solo = if alone { None } else { Some(k) };
                    if !live {
                        self.video_pick(i);
                    }
                    ui.close();
                }
                ui.menu_button("Look", |ui| {
                    let names: Vec<String> = self.project.video.looks.iter().enumerate().map(|(j, l)| look_label(j, l)).collect();
                    let now = clip.part_look(k);
                    if ui.selectable_label(now.is_none(), "The clip's").on_hover_text("No Look of its own: the clip's (or the set's) plays").clicked() {
                        if let Some(c) = self.project.video.clips.get_mut(i) {
                            c.set_part_look(k, None);
                        }
                        ui.close();
                    }
                    for (j, n) in names.iter().enumerate() {
                        if ui.selectable_label(now == Some(j), n).on_hover_text("This part plays with this Look, whatever the clip's").clicked() {
                            if let Some(c) = self.project.video.clips.get_mut(i) {
                                c.set_part_look(k, Some(j));
                            }
                            ui.close();
                        }
                    }
                });
                if ui.button("Keep only this").on_hover_text("The other parts go").clicked() {
                    let look = clip.part_look(k);
                    set_parts(self, vec![(a, b)]);
                    if let Some(c) = self.project.video.clips.get_mut(i) {
                        c.part_looks.clear();
                        c.set_part_look(0, look);
                    }
                    self.video.picked = vec![0];
                    self.video.solo = None;
                    ui.close();
                }
                ui.separator();
            }
            if ui.add_enabled(!parts.is_empty(), egui::Button::new("Whole film")).on_hover_text("No parts: all of the film plays").clicked() {
                set_parts(self, Vec::new());
                self.video.picked.clear();
                self.video.solo = None;
                ui.close();
            }
        });
        if let Some(a) = act {
            self.video_key_action(a);
        }
    }

    /// Puts the film of clip `i` at `t` seconds (it plays at once when it did not).
    fn video_seek_to(&mut self, i: usize, t: f64, live: bool) {
        if !live {
            self.video_pick(i);
        }
        self.video.seek = Some(t);
    }

    /// What the timeline's keys and buttons do.
    pub(crate) fn video_key_action(&mut self, act: VideoAct) {
        let Some((i, clip, dur, live)) = self.video_edited() else { return };
        let here = self.video.at as f32;
        match act {
            VideoAct::Pause => self.video.paused = !self.video.paused,
            VideoAct::Step(n) => {
                if live {
                    self.video.paused = true;
                    let step = 1.0 / self.video_fps(i);
                    self.video.seek = Some((self.video.at + n as f64 * step).clamp(0.0, dur as f64));
                }
            }
            VideoAct::MarkIn if live && !clip.is_cued() => self.video.mark_in = Some(here),
            VideoAct::MarkOut if live && !clip.is_cued() => {
                let c = &mut self.project.video.clips[i];
                match self.video.mark_in.take() {
                    Some(a) if (here - a).abs() >= kv::MIN_PART => {
                        c.add_part(a.min(here), a.max(here));
                        self.video.picked = c.part_at((a + here) * 0.5).into_iter().collect();
                    }
                    Some(_) => self.toast("Too short: a part lasts a tenth of a second at least"),
                    None => match c.part_at(here) {
                        // Inside a part: it ends here.
                        Some(k) if here - c.parts[k].0 >= kv::MIN_PART => {
                            c.parts[k].1 = here;
                            self.video.picked = vec![k];
                        }
                        Some(_) => {}
                        // In a gap (or the whole film): from the end of the part before.
                        None => {
                            let from = c.parts.iter().map(|p| p.1).filter(|e| *e <= here).fold(0.0, f32::max);
                            if here - from >= kv::MIN_PART {
                                c.add_part(from, here);
                                self.video.picked = c.part_at(here - kv::MIN_PART * 0.5).into_iter().collect();
                            }
                        }
                    },
                }
                self.video.solo = None;
            }
            VideoAct::Split if live => {
                if let Some(c) = self.project.video.clips.get_mut(i) {
                    if c.split_part(here) {
                        self.video.picked = c.part_at(here).into_iter().collect();
                        self.video.solo = None;
                    }
                }
            }
            VideoAct::Delete => {
                if let Some(c) = self.project.video.clips.get_mut(i) {
                    let mut picked = std::mem::take(&mut self.video.picked);
                    picked.sort_unstable();
                    for k in picked.into_iter().rev() {
                        if k < c.parts.len() {
                            c.remove_part(k);
                        }
                    }
                    self.video.solo = None;
                }
            }
            _ => {}
        }
    }

    /// The Video's keys, while the pointer is on its picture or timeline
    /// (elsewhere the letters still play notes): I and O mark a part, S
    /// splits, Delete takes the parts picked away, K pauses, comma and
    /// period step a frame. Whether it took the key.
    pub(crate) fn video_key(&mut self, ctx: &Context, key: Key, m: egui::Modifiers) -> bool {
        let over = self.video_shown() && self.video.window_rect.is_some_and(|r| ctx.input(|i| i.pointer.hover_pos()).is_some_and(|p| r.contains(p)));
        if !over || !m.is_none() || self.project.video.clips.is_empty() {
            return false;
        }
        let act = match key {
            Key::I => VideoAct::MarkIn,
            Key::O => VideoAct::MarkOut,
            Key::S => VideoAct::Split,
            Key::K => VideoAct::Pause,
            Key::Comma => VideoAct::Step(-1),
            Key::Period => VideoAct::Step(1),
            Key::Delete | Key::Backspace if !self.video.picked.is_empty() => VideoAct::Delete,
            Key::Escape if self.video.mark_in.is_some() => {
                self.video.mark_in = None;
                return true;
            }
            _ => return false,
        };
        self.video_key_action(act);
        true
    }

    /// The stage in `rect` (or, without shaders, the plain picture).
    fn draw_stage(&self, p: &egui::Painter, rect: Rect) {
        if self.video.stage.failed().is_some() {
            if let (Some(t), true) = (&self.video.texture, self.video.params.video_size[0] > 0.0) {
                let [w, h] = t.size();
                let s = (rect.width() / w as f32).min(rect.height() / h as f32);
                p.image(t.id(), Rect::from_center_size(rect.center(), vec2(w as f32 * s, h as f32 * s)), Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
            }
            return;
        }
        let tex = self.video.texture.as_ref().map(|t| t.id()).filter(|_| self.video.params.video_size[0] > 0.0);
        let old = self.video.old_texture.as_ref().map(|t| t.id()).filter(|_| self.video.params.old_size[0] > 0.0);
        self.video.stage.paint(p, rect, tex, old, self.video.params, self.frame);
    }

    /// Films need ffmpeg: what it is, a button to fetch it, how far that is.
    fn ffmpeg_offer(&mut self, ui: &mut Ui, h: f32) {
        ui.add_space((h * 0.18).max(4.0));
        ui.label(RichText::new("This clip is a film, and films need ffmpeg").color(TEXT).size(15.0));
        ui.add_space(4.0);
        ui.label(RichText::new("ffmpeg is a free program that reads nearly every film. Krabton can fetch it into its library for you.").color(TEXT_DIM));
        ui.add_space(8.0);
        self.ffmpeg_fetch_row(ui);
        ui.add_space(6.0);
        ui.label(RichText::new("Pictures and GIFs play without it.").small().color(TEXT_DIM));
    }

    /// ffmpeg's button: how big it is, how far it is fetched (or how to
    /// install it where Krabton cannot fetch it), and to look for it again.
    fn ffmpeg_fetch_row(&mut self, ui: &mut Ui) {
        let state = self.video.fetch.as_ref().and_then(|f| f.lock().ok().map(|s| s.clone()));
        match (state, ffmpeg::download()) {
            (Some(ffmpeg::Fetch::Downloading { bytes }), Some((_, total))) => {
                ui.add(egui::ProgressBar::new((bytes as f32 / total as f32).min(1.0)).desired_width(280.0).text(format!("Fetching ffmpeg: {} of about {} MB", bytes >> 20, total >> 20)));
            }
            (Some(ffmpeg::Fetch::Unpacking), _) => {
                ui.label(RichText::new("Unpacking ffmpeg...").color(TEXT));
            }
            (_, Some((_, total))) => {
                if ui.add(egui::Button::new(RichText::new(format!("Get ffmpeg ({} MB)", total / 1_000_000)).color(TEXT_DARK)).fill(ACCENT)).on_hover_text(format!("Downloads a ready-made ffmpeg into {}", ffmpeg::library_dir().display())).clicked() {
                    self.video.fetch = Some(ffmpeg::start_fetch());
                }
                ui.add_space(4.0);
                ui.label(RichText::new(ffmpeg::install_hint()).small().color(TEXT_DIM));
            }
            (_, None) => {
                ui.label(RichText::new(format!("Install it ({}), then start Krabton again", ffmpeg::install_hint().trim_start_matches("or install it yourself ").trim_matches(['(', ')']))).color(TEXT));
            }
        }
        if ui.small_button("Look again").on_hover_text("Installed it just now? Krabton looks for it again").clicked() {
            ffmpeg::forget();
            self.video.reload_films();
        }
    }

    /// What the music does now, as small meters (the master's and the song's).
    fn video_ears(&self, ui: &mut Ui, r: Rect) {
        let p = ui.painter();
        rect_fill(p, r, 4.0, PANEL2);
        let sources = [Source::Kick, Source::Hits, Source::Bass, Source::Mids, Source::Highs, Source::Level, Source::Notes, Source::Beat, Source::Bar];
        let w = (r.width() - 16.0) / sources.len() as f32;
        for (k, s) in sources.iter().enumerate() {
            let cell = Rect::from_min_size(pos2(r.left() + 8.0 + k as f32 * w, r.top() + 5.0), vec2(w - 6.0, r.height() - 10.0));
            let v = self.video.dir.master_value(*s).clamp(0.0, 1.0);
            let bar = Rect::from_min_max(pos2(cell.left(), cell.bottom() - 5.0), pos2(cell.right(), cell.bottom()));
            p.rect_filled(bar, 2.0, PANEL3);
            p.rect_filled(Rect::from_min_size(bar.min, vec2(bar.width() * v, bar.height())), 2.0, mix(shade(ACCENT, 0.6), ACCENT, v));
            p.text(pos2(cell.left(), cell.top() + 5.0), Align2::LEFT_CENTER, s.name(), small(), if v > 0.5 { TEXT } else { TEXT_DIM });
            let _ = ui.interact(cell, Id::new(("video_ear", k)), Sense::hover()).on_hover_text(s.about());
        }
    }

    // ------------------------------------------------------------------
    // The side
    // ------------------------------------------------------------------

    /// The Clip tab: the clip edited, how it plays, its parts one at a
    /// time, its Look and how it sits on the screen.
    fn video_tab_clip(&mut self, ui: &mut Ui, set: &mut kv::VideoSet, tracks: &[(u64, String)]) {
        let edited = set.current.min(set.clips.len().saturating_sub(1));
        let look_names: Vec<String> = set.looks.iter().enumerate().map(|(k, l)| look_label(k, l)).collect();
        let part_now = (self.video.playing() == edited).then_some(self.video.dir.playhead.part).flatten();
        let song = {
            let st = self.status();
            (st.playing.load(std::sync::atomic::Ordering::Relaxed), st.song_pos.load(), self.project.beats_per_bar)
        };
        // The step of a sequence the song is on (while it plays).
        let step_now = |every: kv::Every| -> Option<i64> { song.0.then(|| every.step(song.1, song.2)) };
        let Some(clip) = set.clips.get_mut(edited) else {
            ui.add_space(8.0);
            note(ui, "No clip yet. Add a film, a picture or a GIF with + Add file in the deck (or drop one on this view): how it plays is set here.");
            return;
        };
        let mut go_triggers = false;
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            let (b, _) = ui.allocate_exact_size(vec2(22.0, 20.0), Sense::hover());
            rect_fill(ui.painter(), b, 3.0, PANEL3);
            ui.painter().text(b.center(), Align2::CENTER_CENTER, (edited + 1).to_string(), small(), TEXT);
            ui.add(egui::TextEdit::singleline(&mut clip.name).desired_width(ui.available_width()).hint_text("Its name"));
        });
        let dur = self.video.media.get(&clip.path).map_or(0.0, |m| m.info().duration);
        let cued = clip.is_cued();
        let parts_n = clip.spans(dur).len();
        if dur > 0.0 {
            let what = if cued {
                format!("{} long, cut in {} cues by Slice to Drum Rack", clock(dur as f64), clip.cues.as_ref().map_or(0, |c| c.len()))
            } else if clip.is_cut() {
                format!("{} of {} plays: {} part{} back to back", clock(clip.joined_len(dur) as f64), clock(dur as f64), clip.parts.len(), if clip.parts.len() == 1 { "" } else { "s" })
            } else {
                format!("{} long: all of it plays (drag on the timeline to keep parts)", clock(dur as f64))
            };
            note(ui, &what);
        } else {
            note(ui, "A picture: it shows as it is");
        }

        section(ui, "How it plays");
        row(ui, "Mode", |ui| {
            let modes: Vec<(PlayMode, String)> = PlayMode::ALL.iter().map(|m| (*m, m.name().to_string())).collect();
            let tip = clip.play.mode.about();
            dropdown(ui, ("video_mode", edited), 70.0, &mut clip.play.mode, &modes, tip);
            if !(cued && clip.play.mode == PlayMode::Slices) {
                let mut dir = match (clip.play.reverse, clip.play.bounce) {
                    (_, true) => 2,
                    (true, false) => 1,
                    _ => 0,
                };
                let free = clip.play.mode != PlayMode::Slices;
                let dirs: Vec<(usize, String)> = [(0, "Forward"), (1, "Reverse"), (2, "Ping-pong")].iter().filter(|(k, _)| *k < 2 || free).map(|(k, s)| (*k, s.to_string())).collect();
                if dropdown(ui, ("video_dir", edited), 82.0, &mut dir, &dirs, "Which way it plays: forwards, backwards, or there and back (Loop and Tempo)") {
                    clip.play.reverse = dir == 1;
                    clip.play.bounce = dir == 2;
                }
            }
            if choice(ui, "Freeze", clip.play.freeze, "It stands still on one frame: where Start puts it, or the slice a note picks").clicked() {
                clip.play.freeze = !clip.play.freeze;
            }
        });
        let seq_on = clip.play.seq.on && parts_n > 1 && !cued;
        let what = if seq_on { "the part that plays" } else if clip.is_cut() { "its parts back to back" } else { "the whole clip" };
        let plain = match clip.play.mode {
            PlayMode::Loop => format!("Round and round on its own, at its speed ({what})"),
            PlayMode::Tempo => format!("In time with the song: {what} in {} bar{}, following where the song is", clip.play.bars, if clip.play.bars == 1 { "" } else { "s" }),
            PlayMode::Slices if cued => "Each note shows the film from its cue, as filmed, for as long as its pad's sound plays".to_string(),
            PlayMode::Slices if seq_on => format!("Cut in {} slices of the part that plays: the notes (or hits) pick one", clip.play.slices),
            PlayMode::Slices if clip.play.part_slices && parts_n > 1 => format!("Notes play its parts, like pads: C1 the first part, C#1 the second ({parts_n} parts)"),
            PlayMode::Slices => format!("Cut in {} slices that notes play, like pads: C1 the first, C#1 the second...", clip.play.slices),
        };
        hint(ui, &plain);
        match clip.play.mode {
            PlayMode::Tempo => {
                row(ui, "Length", |ui| {
                    ui.spacing_mut().item_spacing.x = 3.0;
                    for b in [1, 2, 4, 8, 16] {
                        if choice(ui, &b.to_string(), clip.play.bars == b, "How many bars it takes, in time with the song").clicked() {
                            clip.play.bars = b;
                        }
                    }
                    ui.label(RichText::new("bars").small().color(TEXT_DIM));
                });
            }
            // Cues play as filmed, as their sound does.
            PlayMode::Slices if cued => {}
            PlayMode::Loop | PlayMode::Slices => {
                vj::bar(ui, &mut clip.play.speed, 0.0, 4.0, 1.0, "Speed", |v| format!("{v:.3}×"), 0.0, None, "How fast it plays (1: as filmed)");
            }
        }
        if clip.play.mode != PlayMode::Slices && dur > 0.0 {
            let len = clip.joined_len(dur).max(0.1);
            vj::bar(ui, &mut clip.play.offset, 0.0, len, 0.0, "Start", |v| format!("{v:.2} s"), 0.0, None, "Where in its parts it starts (and where Freeze stands)");
        }
        let mut even = false;
        if let (true, Some(cues)) = (cued && clip.play.mode == PlayMode::Slices, clip.cues.as_mut()) {
            // Cut where Slice to Drum Rack cut its sound.
            row(ui, "Cues", |ui| {
                ui.label(RichText::new(format!("{}", cues.len())).color(TEXT));
                let (name, tip) = if cues.gate { ("Gate", "Gate: the picture stands still where its note ends, as the pad's sound stops. Click for One-Shot") } else { ("One-Shot", "One-Shot: each piece plays on to its end, as the pad's sound does. Click for Gate") };
                if choice(ui, name, false, tip).clicked() {
                    cues.gate = !cues.gate;
                }
                if choice(ui, "Even", false, "Forget the cues: back to equal slices (and parts can be kept again)").clicked() {
                    even = true;
                }
            });
        } else if clip.play.mode == PlayMode::Slices {
            row(ui, "Slices", |ui| {
                ui.spacing_mut().item_spacing.x = 3.0;
                let by_part = clip.play.part_slices && parts_n > 1 && !seq_on;
                for n in [4, 8, 16, 32] {
                    if choice(ui, &n.to_string(), !by_part && clip.play.slices == n, "In how many equal slices it is cut (of its parts back to back, or of the part that plays)").clicked() {
                        clip.play.slices = n;
                        clip.play.part_slices = false;
                    }
                }
                let tip = if parts_n > 1 { "One slice for each part: the notes play the parts" } else { "One slice for each part: keep a few parts on the timeline first" };
                if ui.add_enabled(parts_n > 1 && !seq_on, egui::Button::new(RichText::new("Parts").color(if by_part { TEXT_DARK } else { TEXT })).fill(if by_part { ACCENT } else { PANEL3 })).on_hover_text(tip).on_disabled_hover_text(tip).clicked() {
                    clip.play.part_slices = !clip.play.part_slices;
                }
            });
        } else {
            row(ui, "Jump cuts", |ui| {
                ui.spacing_mut().item_spacing.x = 3.0;
                for n in [4, 8, 16, 32] {
                    if choice(ui, &n.to_string(), clip.play.slices == n, "Where a Jump effect can cut to: the clip (or the part that plays) in this many equal slices").clicked() {
                        clip.play.slices = n;
                    }
                }
                ui.label(RichText::new("slices").small().color(TEXT_DIM));
            });
        }
        if clip.play.mode == PlayMode::Slices {
            let by = match (&clip.cues, &clip.play.slice_hits) {
                (Some(q), _) if cued => format!("the notes of {}", track_name(tracks, q.track, "any track")),
                (_, Some(t)) => hits_said(t, tracks),
                _ => format!("the notes of {}", track_name(tracks, clip.play.slice_track, "any track")),
            };
            row(ui, "Played by", |ui| {
                ui.label(RichText::new(by).color(TEXT));
                if ui.small_button("Change...").on_hover_text("In the Triggers tab").clicked() {
                    go_triggers = true;
                }
            });
        }
        if even {
            clip.cues = None;
        }
        row(ui, "Look", |ui| {
            look_dropdown(ui, ("video_clip_look", edited), &mut clip.look, &look_names, "Same as the deck", "The Look this clip plays with (a part's own wins over it). Same as the deck: whatever the Look pads, the sequence, the notes or the scenes say");
        });
        hint(ui, "A part can have a Look of its own: right-click it on the timeline");

        // Its parts one at a time, changing on the beat.
        if !cued {
            section(ui, "One part at a time");
            let seq = &mut clip.play.seq;
            row(ui, "Parts", |ui| {
                let tip = if parts_n > 1 { "Plays one part at a time, changing on the beat as the order says" } else { "Keep two parts or more on the timeline first" };
                let mut on = seq.on;
                ui.add_enabled_ui(parts_n > 1, |ui| switch(ui, &mut on, tip));
                seq.on = on;
                let said = if parts_n < 2 { "keep two parts or more first".to_string() } else if seq.on { "one at a time".to_string() } else { "back to back".to_string() };
                ui.label(RichText::new(said).small().color(TEXT_DIM));
            });
            if seq.on && parts_n > 1 {
                row(ui, "Order", |ui| {
                    let orders: Vec<(kv::PartOrder, String)> = kv::PartOrder::ALL.iter().map(|o| (*o, o.name().to_string())).collect();
                    let tip = seq.order.about();
                    dropdown(ui, ("video_seq_order", edited), 96.0, &mut seq.order, &orders, tip);
                    if seq.order.on_steps() {
                        ui.label(RichText::new("every").small().color(TEXT_DIM));
                        if choice(ui, seq.every.name(), false, "How often it may change part, on the song's beat. Click for the next").clicked() {
                            seq.every = seq.every.next();
                        }
                    }
                });
                hint(ui, seq.order.about());
                match seq.order {
                    kv::PartOrder::Steps => {
                        let now = step_now(seq.every).map(|s| s.rem_euclid(seq.steps.len().max(1) as i64) as usize);
                        ui.horizontal_top(|ui| {
                            let (r, _) = ui.allocate_exact_size(vec2(LABEL_W, 22.0), Sense::hover());
                            ui.painter().text(pos2(r.left(), r.center().y), Align2::LEFT_CENTER, "Steps", small(), TEXT_DIM);
                            ui.vertical(|ui| steps_row(ui, ("video_seq_steps", edited), &mut seq.steps, parts_n.max(1), now, "Hold", |k| (k + 1).to_string(), "Each step a part (click for the next part, right-click to pick, or Hold: the part before stays)"));
                        });
                    }
                    kv::PartOrder::Notes | kv::PartOrder::Hits => {
                        let by = if seq.order == kv::PartOrder::Notes { format!("the notes of {}", track_name(tracks, seq.track, "any track")) } else { hits_said(&seq.hits, tracks) };
                        row(ui, "Played by", |ui| {
                            ui.label(RichText::new(by).color(TEXT));
                            if ui.small_button("Change...").on_hover_text("In the Triggers tab").clicked() {
                                go_triggers = true;
                            }
                        });
                    }
                    _ => {}
                }
                let k = part_now.map_or("-".to_string(), |k| (k + 1).to_string());
                hint(ui, &format!("Part {k} of {parts_n} plays now (lit on the timeline)"));
            }
        }

        // How it sits on the screen.
        section(ui, "On the screen");
        let frame = &mut clip.frame;
        row(ui, "Fit", |ui| {
            let fits = vec![(None, "Same as Output".to_string()), (Some(kv::Fit::Fill), "Fill".to_string()), (Some(kv::Fit::Fit), "Fit".to_string())];
            dropdown(ui, ("video_clip_fit", edited), 112.0, &mut frame.fit, &fits, "Fill: the whole screen, cutting off what sticks out. Fit: the whole picture, with black bars");
            if ui.add_enabled(!frame.is_plain(), egui::Button::new("Reset")).on_hover_text("As it was: the whole picture, in the middle").clicked() {
                *frame = kv::ClipFrame::default();
            }
        });
        let open_id = Id::new(("video_frame_more", edited));
        let mut more = ui.ctx().data(|d| d.get_temp::<bool>(open_id)).unwrap_or(!frame.is_plain());
        let fader = |ui: &mut Ui, label: &str, v: &mut f32, range: std::ops::RangeInclusive<f32>, tip: &str| {
            let d = match label {
                "Opacity" | "Zoom" => 1.0,
                _ => 0.0,
            };
            vj::bar(ui, v, *range.start(), *range.end(), d, label, |v| format!("{v:.2}"), 0.0, None, tip);
        };
        fader(ui, "Opacity", &mut frame.opacity, 0.0..=1.0, "How see-through (0: not there, 1: all of it)");
        fader(ui, "Zoom", &mut frame.zoom, 0.25..=4.0, "How big (1: as it fits)");
        row(ui, "", |ui| {
            if ui.add(egui::Button::new(RichText::new(if more { "Fewer" } else { "Where, turn, crop..." }).small()).fill(PANEL2)).on_hover_text("Where it is on the screen, turned, and cut off at its sides").clicked() {
                more = !more;
            }
        });
        if more {
            fader(ui, "Across", &mut frame.x, -1.0..=1.0, "Moved right (or left), in screens");
            fader(ui, "Down", &mut frame.y, -1.0..=1.0, "Moved down (or up), in screens");
            fader(ui, "Turn", &mut frame.turn, -0.5..=0.5, "Turned (0.25: a quarter of the way round)");
            for (k, side) in ["Crop left", "Crop top", "Crop right", "Crop bottom"].iter().enumerate() {
                fader(ui, side, &mut frame.crop[k], 0.0..=0.45, "Cut off this side (of the picture)");
            }
        }
        ui.ctx().data_mut(|d| d.insert_temp(open_id, more));
        if go_triggers {
            self.video.tab = VideoTab::Triggers;
        }
    }

    /// The Look tab: the Look edited, its name, a preset, how much the
    /// music moves it, and its effects, each a card with its settings and
    /// the links that move it.
    /// The Triggers tab: what changes the clips and the Looks, and what
    /// plays the slices and the parts of the clip edited.
    fn video_tab_triggers(&mut self, ui: &mut Ui, set: &mut kv::VideoSet, tracks: &[(u64, String)]) {
        let song = {
            let st = self.status();
            (st.playing.load(std::sync::atomic::Ordering::Relaxed), st.song_pos.load(), self.project.beats_per_bar)
        };
        ui.add_space(6.0);
        note(ui, "What changes the picture as the music plays. A click on a pad in the deck always does too.");

        section(ui, "Clips change with");
        row(ui, "Scenes", |ui| {
            switch(ui, &mut set.follow_scenes, "The scene that plays picks the clip: scene 1 the first clip (its number on its pad), scene 2 the second, and on");
            ui.label(RichText::new("scene 1 plays clip 1, and on").small().color(TEXT_DIM));
        });
        row(ui, "Notes of", |ui| {
            track_dropdown(ui, "video_clip_notes", &mut set.clip_notes, tracks, "No track", "The notes of a track pick the clip: C1 the first (its note on its pad), C#1 the second, and on");
            ui.label(RichText::new("C1 clip 1, C#1 clip 2").small().color(TEXT_DIM));
        });
        hits_row(ui, "video_clip_hits", &mut set.clip_hits, tracks, "clip", "Hits in a sound change the clip (for sequencers and drum machines that send no notes)");
        let how = match (set.follow_scenes, set.clip_notes.is_some(), set.clip_hits.is_some()) {
            (false, false, false) => "Now: only clicks in the deck (and a Next Clip effect)",
            (true, false, false) => "Now: the scenes, clicks and a Next Clip effect",
            (false, true, false) => "Now: that track's notes, clicks and a Next Clip effect",
            (_, _, true) => "Now: the hits (and whatever else is on), clicks and a Next Clip effect",
            (true, true, false) => "Now: the scenes, that track's notes, clicks and a Next Clip effect",
        };
        hint(ui, how);

        section(ui, "Looks change with");
        row(ui, "Scenes", |ui| {
            switch(ui, &mut set.look_scenes, "The scene that plays picks the Look: scene 1 the first pad, and on");
            ui.label(RichText::new("scene 1 plays Look 1, and on").small().color(TEXT_DIM));
        });
        row(ui, "Notes of", |ui| {
            track_dropdown(ui, "video_look_notes", &mut set.look_notes, tracks, "No track", "The notes of a track pick the Look: C1 the first pad, C#1 the second (a track of its own keeps them apart from the clips' notes)");
            ui.label(RichText::new("C1 Look 1, C#1 Look 2").small().color(TEXT_DIM));
        });
        hits_row(ui, "video_look_hits", &mut set.look_hits, tracks, "Look", "Hits in a sound change the Look");
        let n_looks = set.looks.len().max(1);
        let look_names: Vec<String> = set.looks.iter().enumerate().map(|(k, l)| look_label(k, l)).collect();
        let seq = &mut set.look_seq;
        row(ui, "Sequence", |ui| {
            switch(ui, &mut seq.on, "Looks in turn on the song's beat: a step every so often, each step a Look (or -: the deck's own)");
            if seq.on {
                ui.label(RichText::new("a step every").small().color(TEXT_DIM));
                if choice(ui, seq.every.name(), false, "How long each step lasts, on the song's beat. Click for the next").clicked() {
                    seq.every = seq.every.next();
                }
            } else {
                ui.label(RichText::new("off").small().color(TEXT_DIM));
            }
        });
        if seq.on {
            let now = song.0.then(|| seq.every.step(song.1, song.2)).map(|s| s.rem_euclid(seq.steps.len().max(1) as i64) as usize);
            ui.horizontal_top(|ui| {
                ui.add_space(LABEL_W);
                ui.vertical(|ui| steps_row(ui, "video_look_steps", &mut seq.steps, n_looks, now, "-", |k| look_names.get(k).map_or((k + 1).to_string(), |n| short(n, 5)), "Each step a Look (click for the next, right-click to pick; -: the deck's own)"));
            });
        }
        hint(ui, "Which Look plays: a part's own, else its clip's, else the sequence's step, else the deck's (the pad clicked, or the scenes, notes or hits)");

        // The clip edited: what plays its slices and its parts.
        let edited = set.current.min(set.clips.len().saturating_sub(1));
        let Some(clip) = set.clips.get_mut(edited) else { return };
        let dur = self.video.media.get(&clip.path).map_or(0.0, |m| m.info().duration);
        let parts_n = clip.spans(dur).len();
        let cued = clip.is_cued();
        section(ui, &format!("Clip {}: its slices", edited + 1));
        if clip.play.mode != PlayMode::Slices {
            hint(ui, &format!("It plays in {}: choose Slices in the Clip tab for notes or hits to cut it up", clip.play.mode.name()));
        } else if let (true, Some(cues)) = (cued, clip.cues.as_mut()) {
            row(ui, "Notes of", |ui| {
                track_dropdown(ui, ("video_cue_track", edited), &mut cues.track, tracks, "Any track", "Whose notes show the cues (the track it was sliced to)");
            });
            hint(ui, "Each note shows the film from its cue, as the Drum Rack plays its sound");
        } else {
            row(ui, "Played by", |ui| {
                let mut by = clip.play.slice_hits.is_some() as usize;
                if dropdown(ui, ("video_slice_by", edited), 96.0, &mut by, &[(0, "Notes".to_string()), (1, "Hits".to_string())], "Notes: a track's notes pick the slices, like pads. Hits: kicks, snares or hats in a sound move to another slice (for sequencers and drum machines that send no notes)") {
                    clip.play.slice_hits = (by == 1).then(HitTrigger::default);
                }
                if clip.play.slice_hits.is_none() {
                    track_dropdown(ui, ("video_slice_track", edited), &mut clip.play.slice_track, tracks, "Any track", "Whose notes pick the slices");
                }
            });
            match &mut clip.play.slice_hits {
                Some(t) => hit_editor(ui, ("video_slice_hits", edited), t, tracks, "slice"),
                None => hint(ui, "C1 the first slice, C#1 the second, and on"),
            }
        }
        if !cued {
            section(ui, &format!("Clip {}: its parts", edited + 1));
            let seq = &mut clip.play.seq;
            match (seq.on && parts_n > 1, seq.order) {
                (false, _) => hint(ui, "They play back to back. Switch on One part at a time in the Clip tab for them to change on the beat, by notes or by hits"),
                (true, kv::PartOrder::Notes) => {
                    row(ui, "Notes of", |ui| {
                        track_dropdown(ui, ("video_seq_track", edited), &mut seq.track, tracks, "Any track", "Whose notes pick the part: C3 the first, C#3 the second (below C3 they still play slices)");
                    });
                    hint(ui, "C3 the first part, C#3 the second (above the slices, so one track can play both)");
                }
                (true, kv::PartOrder::Hits) => hit_editor(ui, ("video_seq_hits", edited), &mut seq.hits, tracks, "part"),
                (true, o) => {
                    row(ui, "Played by", |ui| {
                        let mut by = 0usize;
                        let opts = [(0, o.name().to_string()), (1, "Notes".to_string()), (2, "Hits".to_string())];
                        if dropdown(ui, ("video_seq_by", edited), 96.0, &mut by, &opts, "How the parts change: in an order of their own (set in the Clip tab), by a track's notes or by hits") {
                            seq.order = if by == 1 { kv::PartOrder::Notes } else { kv::PartOrder::Hits };
                        }
                    });
                    hint(ui, &format!("{} every {}", o.about(), seq.every.name().to_lowercase()));
                }
            }
        }
    }

    /// The Output tab: the screen for a projector, how the picture fills
    /// it, Record Video, Export Video, the size of a film, ffmpeg and the keys.
    fn video_tab_output(&mut self, ui: &mut Ui, set: &mut kv::VideoSet) {
        section(ui, "Screen");
        let screen = self.video.screen_open;
        ui.horizontal(|ui| {
            let w = (ui.available_width() - 6.0) * 0.5;
            if ui.add_sized([w, 28.0], egui::Button::new(RichText::new(if screen { "Close the screen" } else { "Screen" }).color(if screen { TEXT_DARK } else { TEXT })).fill(if screen { ACCENT } else { PANEL3 })).on_hover_text("The picture alone in a window of its own (click again to close it)").clicked() {
                self.video.screen_open = !screen;
                self.video.fullscreen = false;
            }
            if ui.add_sized([w, 28.0], egui::Button::new("Full screen")).on_hover_text("The picture filling the screen Krabton is on, at once (Esc or a double-click leaves)").clicked() {
                self.video.screen_open = true;
                self.video.fullscreen = true;
            }
        });
        note(ui, "Drag the screen's window to a projector or a second screen and double-click it to fill that screen (double-click, F11 or Esc to leave). Double-click the picture here for the same.");
        ui.add_space(4.0);
        row(ui, "Picture", |ui| {
            let fits = vec![(kv::Fit::Fill, "Fill".to_string()), (kv::Fit::Fit, "Fit".to_string())];
            dropdown(ui, "video_fit", 70.0, &mut set.fit, &fits, "Fill: the whole screen, cutting off what sticks out. Fit: the whole picture, with black bars (a clip can have its own)");
            ui.label(RichText::new(if set.fit == kv::Fit::Fill { "no bars, the edges cut" } else { "all of it, with bars" }).small().color(TEXT_DIM));
        });

        section(ui, "Record");
        ui.horizontal(|ui| {
            let on = self.video.rec.is_some();
            let label = if on { "Stop recording" } else { "Record Video" };
            let b = egui::Button::new(RichText::new(label).color(TEXT)).fill(if on { REC } else { PANEL3 });
            if ui.add_sized([150.0, 28.0], b).on_hover_text("Records the picture and the master's sound as you play. Click again to stop").clicked() {
                if on {
                    self.video_record_stop();
                } else {
                    self.video_record_start(None);
                }
            }
            if let Some(r) = &self.video.rec {
                let (secs, bytes) = r.progress();
                ui.label(RichText::new(format!("REC {}  {:.1} MB", clock(secs), bytes as f64 / 1e6)).color(REC).font(mono()));
                ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));
            }
        });
        note(ui, "What shows and the master's sound, live, into a film (MP4) in Documents/Krabton/Recorded");

        section(ui, "Export");
        match &self.video.export {
            Some(job) => {
                let st = job.state();
                let what = match st {
                    export::State::Reading => "Reading the films...",
                    export::State::Sound(_) => "Playing the song...",
                    _ => "Drawing the frames...",
                };
                ui.add(egui::ProgressBar::new(st.fraction()).text(what));
                if ui.button("Cancel").clicked() {
                    job.cancel.store(true, std::sync::atomic::Ordering::Relaxed);
                }
            }
            None => {
                if ui.add_enabled(!self.project.video.clips.is_empty(), egui::Button::new("Export Video...").min_size(vec2(150.0, 28.0))).on_hover_text("A film (MP4) of the song with its sound and the Video as it plays to it").on_disabled_hover_text("Add a clip first").clicked() {
                    self.video.export_open = true;
                }
            }
        }
        note(ui, "A film of the arrangement or a scene with the song's sound: the clips, their changes, the Looks and every link, made in the background");
        ui.add_space(4.0);
        let o = &mut self.video.export_opts;
        row(ui, "Size", |ui| {
            let mut size = (o.width, o.height);
            let sizes: Vec<((u32, u32), String)> = export::SIZES.iter().map(|&(w, h)| ((w, h), format!("{h}p ({w} by {h})"))).collect();
            if dropdown(ui, "video_out_size", 130.0, &mut size, &sizes, "How big a film is (Record Video and Export Video)") {
                (o.width, o.height) = size;
            }
        });
        row(ui, "Frames", |ui| {
            let rates: Vec<(u32, String)> = export::RATES.iter().map(|f| (*f, f.to_string())).collect();
            dropdown(ui, "video_out_fps", 50.0, &mut o.fps, &rates, "How smooth: 24 or 25 as cinema, 30 as most video, 60 for fast effects");
            ui.label(RichText::new("a second").small().color(TEXT_DIM));
        });

        section(ui, "ffmpeg");
        match ffmpeg::find() {
            Some(p) => note(ui, &format!("Films play, record and export with {}", p.display())),
            None => {
                note(ui, "Films need ffmpeg, a free program that reads nearly every film. Pictures and GIFs play without it.");
                ui.add_space(4.0);
                self.ffmpeg_fetch_row(ui);
            }
        }

        section(ui, "Keys");
        note(ui, "With the pointer on the picture or the timeline:");
        for (k, what) in [("I", "Mark In: a part starts here"), ("O", "Mark Out: the part ends here"), ("S", "Split the part the film is in"), ("Delete", "Take the parts picked away"), ("K", "Pause (the music goes on)"), (", and .", "A frame back, a frame on"), ("Esc", "Forget Mark In")] {
            row(ui, k, |ui| {
                ui.label(RichText::new(what).small().color(TEXT));
            });
        }
    }

    /// The box for Export Video: what of the song, how big, how smooth.
    fn video_export_box(&mut self, ctx: &Context) {
        let mut open = true;
        let mut go = false;
        let has_arr = self.project.arrangement_end() > 0.0;
        egui::Window::new("Export Video").collapsible(false).resizable(false).open(&mut open).show(ctx, |ui| {
            ui.radio_value(&mut self.export.source, ExportSource::Arrangement, "Arrangement").on_hover_text("The whole arrangement");
            if !has_arr {
                ui.label(RichText::new("   (the arrangement is empty)").small().color(TEXT_DIM));
            }
            ui.radio_value(&mut self.export.source, ExportSource::Scene, "Scene");
            ui.add_enabled_ui(self.export.source == ExportSource::Scene, |ui| {
                ui.horizontal(|ui| {
                    let name = self.project.scenes.get(self.export.scene).map(|s| s.name.clone()).unwrap_or_default();
                    egui::ComboBox::from_id_salt("video_export_scene").selected_text(name).show_ui(ui, |ui| {
                        for (i, s) in self.project.scenes.iter().enumerate() {
                            ui.selectable_value(&mut self.export.scene, i, &s.name);
                        }
                    });
                    ui.add(egui::DragValue::new(&mut self.export.bars).range(1..=512).suffix(" bars"));
                });
            });
            ui.horizontal(|ui| {
                ui.label("Size");
                let o = &mut self.video.export_opts;
                let mut size = (o.width, o.height);
                let sizes: Vec<((u32, u32), String)> = export::SIZES.iter().map(|&(w, h)| ((w, h), format!("{h}p ({w} by {h})"))).collect();
                if dropdown(ui, "video_export_size", 130.0, &mut size, &sizes, "How big the film is (Record Video too)") {
                    (o.width, o.height) = size;
                }
                ui.label("Frames a second");
                let rates: Vec<(u32, String)> = export::RATES.iter().map(|f| (*f, f.to_string())).collect();
                dropdown(ui, "video_export_fps", 50.0, &mut o.fps, &rates, "How smooth: 24 or 25 as cinema, 30 as most video, 60 for fast effects");
            });
            ui.label(RichText::new("H.264 and AAC in an MP4, with a bar of tail. The films are as the Video holds them (854 wide at most). Made in the background, faster or slower than real time.").small().color(TEXT_DIM));
            if ffmpeg::find().is_none() {
                ui.label(RichText::new("Needs ffmpeg: the Video's Get ffmpeg fetches it").color(REC));
            }
            ui.add_space(4.0);
            if ui.add_enabled(ffmpeg::find().is_some() && self.video.export.is_none(), egui::Button::new(RichText::new("Export...").strong())).clicked() {
                go = true;
            }
        });
        if go {
            let name = format!("{}.mp4", self.project.name);
            if let Some(path) = rfd::FileDialog::new().add_filter("MP4 film", &["mp4"]).set_file_name(name).save_file() {
                self.start_video_export(path);
            }
        }
        if !open || go {
            self.video.export_open = false;
        }
    }

    /// Starts Export Video into `path` (see `video::export`).
    pub(crate) fn start_video_export(&mut self, path: PathBuf) {
        if self.video.export.is_some() {
            return;
        }
        let Some(exe) = ffmpeg::find() else {
            self.toast("Export Video needs ffmpeg: the Video's Get ffmpeg fetches it");
            return;
        };
        let project = self.project.clone();
        let bar = project.beats_per_bar_f();
        let source = match self.export.source {
            ExportSource::Arrangement => {
                let end = project.arrangement_end();
                if end <= 0.0 {
                    self.toast("The arrangement is empty. Use 'Scenes to Arrangement' or export a scene.");
                    return;
                }
                krabton_engine::render::RenderSource::Arrangement { start: 0.0, end }
            }
            ExportSource::Scene => krabton_engine::render::RenderSource::Scene { scene: self.export.scene, beats: self.export.bars as f64 * bar },
        };
        let media = project.video.clips.iter().map(|c| Some(self.video.media.entry(c.path.clone()).or_insert_with(|| media::Media::open(&c.path)).clone())).collect();
        let sr = 48_000;
        let (plugin_instances, processors, errors) = self.plugins.export_instances(&project, sr as f64);
        if let Some(e) = errors.first() {
            self.toast(format!("Exporting without a plugin: {e}"));
        }
        let work = export::Work { project, source, sample_rate: sr, opts: self.video.export_opts, out: path, ffmpeg: exe, media, processors };
        self.video.export = Some(export::start(work, plugin_instances));
    }

    /// The picture alone, in a window of its own (for a second screen).
    fn video_screen(&mut self, ctx: &Context) {
        let id = egui::ViewportId::from_hash_of("krabton_video_screen");
        let builder = egui::ViewportBuilder::default().with_title("Krabton Video").with_inner_size([960.0, 540.0]).with_min_inner_size([160.0, 90.0]);
        let mut close = false;
        ctx.show_viewport_immediate(id, builder, |ctx, class| {
            let draw = |ui: &mut Ui, app: &mut App| {
                let r = ui.max_rect();
                ui.painter().rect_filled(r, 0.0, Color32::BLACK);
                let pic = sixteen_nine(r);
                app.draw_stage(ui.painter(), pic);
                let resp = ui.interact(r, Id::new("video_screen_area"), Sense::click());
                // A hint while the mouse is over it (out of the picture when it is still).
                if resp.hovered() && ui.input(|i| i.pointer.time_since_last_movement() < 2.0) {
                    let hint = if app.video.fullscreen { "Double-click or Esc: back to a window" } else { "Double-click: fill the screen" };
                    let g = ui.painter().text(r.center_bottom() - vec2(0.0, 20.0), Align2::CENTER_CENTER, hint, body(), TEXT);
                    ui.painter().rect_filled(g.expand(6.0), 4.0, Color32::from_black_alpha(160));
                    ui.painter().text(r.center_bottom() - vec2(0.0, 20.0), Align2::CENTER_CENTER, hint, body(), TEXT);
                }
                resp.double_clicked()
            };
            let toggle = if class == egui::ViewportClass::Embedded {
                // No windows of its own here: inside the main one.
                let mut open = true;
                let t = egui::Window::new("Video screen").open(&mut open).default_size([480.0, 270.0]).show(ctx, |ui| draw(ui, self)).and_then(|r| r.inner).unwrap_or(false);
                close |= !open;
                t
            } else {
                egui::CentralPanel::default().frame(egui::Frame::new().fill(Color32::BLACK)).show(ctx, |ui| draw(ui, self)).inner
            };
            let esc = ctx.input(|i| i.key_pressed(egui::Key::Escape));
            let f11 = ctx.input(|i| i.key_pressed(egui::Key::F11));
            if toggle || f11 || (esc && self.video.fullscreen) {
                self.video.fullscreen = !self.video.fullscreen;
            }
            // Full screen as asked (here, or by the window's Full screen).
            if self.video.full_sent != Some(self.video.fullscreen) {
                ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(self.video.fullscreen));
                self.video.full_sent = Some(self.video.fullscreen);
            }
            if ctx.input(|i| i.viewport().close_requested()) {
                close = true;
            }
        });
        if close {
            self.video.screen_open = false;
            self.video.fullscreen = false;
            self.video.full_sent = None;
        }
    }
}

impl App {
    /// The Video's script actions for its timeline and clips (see
    /// `run_script`). Whether `name` was one of them.
    ///
    /// `videopart=FROM,TO` keeps that part too (seconds) and `videounpart=N`
    /// takes part N away, `videoparts` prints them (`videoparts=0`: the whole
    /// film), `videosplit[=T]` splits at T (or the playhead), `videoseek=T`
    /// puts the film there, `videomarkin[=T]` and `videomarkout[=T]` mark (at
    /// T, or where the film is), `videopause[=0]`, `videosolo=N` plays part N
    /// alone (`videosolo` all again), `videopartslices[=0]`, `videopick=N`
    /// edits and plays clip N at once and `videoclip=N` clicks it (with the
    /// launch and transition), `videomove=FROM,TO`, `videodup=N`,
    /// `videolaunch=N` (0 now, 1 beat, 2 bar), `videotransition=NAME[,BEATS]`,
    /// `videonotes=TRACK` (`videonotes` off), `videozoom=FROM,LENGTH`
    /// (`videozoom` all), `videofull` (the screen, filling it) and
    /// `videoexport=PATH[,HEIGHT,FPS]` (Export Video of the export's source).
    pub(crate) fn video_script(&mut self, name: &str, arg: &str) -> bool {
        let f: Vec<f32> = arg.split(',').filter_map(|v| v.trim().parse().ok()).collect();
        let n = f.first().map(|v| *v as usize);
        let i = self.project.video.current;
        match name {
            "videopart" => {
                if let (Some(c), [a, b, ..]) = (self.project.video.clips.get_mut(i), f.as_slice()) {
                    c.add_part(*a, *b);
                }
            }
            "videounpart" => {
                self.video.picked = n.into_iter().collect();
                self.video_key_action(VideoAct::Delete);
            }
            "videoparts" => match (arg.trim(), self.project.video.clips.get_mut(i)) {
                ("0", Some(c)) => c.parts.clear(),
                (_, Some(c)) => {
                    let dur = self.video.media.get(&c.path).map_or(0.0, |m| m.info().duration);
                    println!("video parts of clip {i}: {:?} ({:.2} s of {:.2} s)", c.parts, c.joined_len(dur), dur);
                }
                _ => {}
            },
            "videosplit" => {
                if let Some(t) = f.first() {
                    self.video.at = *t as f64;
                }
                self.video_key_action(VideoAct::Split);
            }
            "videoseek" => self.video.seek = f.first().map(|t| *t as f64),
            "videomarkin" | "videomarkout" => {
                if let Some(t) = f.first() {
                    self.video.at = *t as f64;
                }
                self.video_key_action(if name == "videomarkin" { VideoAct::MarkIn } else { VideoAct::MarkOut });
            }
            "videopause" => self.video.paused = arg.trim() != "0",
            "videosolo" => self.video.solo = n,
            "videopartslices" => {
                if let Some(c) = self.project.video.clips.get_mut(i) {
                    c.play.part_slices = arg.trim() != "0";
                }
            }
            "videopick" => self.video_pick(n.unwrap_or(0)),
            "videoclip" => self.video_launch(n.unwrap_or(0)),
            "videomove" => {
                if let [a, b, ..] = f.as_slice() {
                    self.video_move_clip(*a as usize, *b as usize);
                }
            }
            "videodup" => {
                let k = n.unwrap_or(i);
                let set = &mut self.project.video;
                if k < set.clips.len() && set.clips.len() < kv::MAX_CLIPS {
                    let c = set.clips[k].clone();
                    set.clips.insert(k + 1, c);
                }
            }
            "videolaunch" => self.project.video.launch = Launch::ALL[n.unwrap_or(0).min(Launch::ALL.len() - 1)],
            "videotransition" => {
                let mut words = arg.split(',');
                let t = words.next().map(str::trim).unwrap_or("");
                if let Some(t) = Transition::ALL.into_iter().find(|x| x.name().eq_ignore_ascii_case(t)) {
                    self.project.video.transition = t;
                }
                if let Some(b) = words.next().and_then(|b| b.trim().parse::<f32>().ok()) {
                    self.project.video.transition_beats = b;
                }
            }
            "videonotes" => self.project.video.clip_notes = n.and_then(|t| self.project.tracks.get(t)).map(|t| t.id),
            "videozoom" => {
                self.video.view = match f.as_slice() {
                    [a, b, ..] => (*a, *b),
                    _ => (0.0, 0.0),
                }
            }
            "videofull" => {
                self.video.screen_open = true;
                self.video.fullscreen = true;
            }
            // The part sequence: videoseq=ON[,EVERY,ORDER] (every: beat, bar, 2, 4; order by
            // its name), videosteps=1,2,-,3 (parts from 1, - holds).
            "videoseq" => {
                let mut words = arg.split(',').map(str::trim);
                if let Some(c) = self.project.video.clips.get_mut(i) {
                    c.play.seq.on = words.next() != Some("0");
                    if let Some(e) = words.next() {
                        c.play.seq.every = match e {
                            "beat" => kv::Every::Beat,
                            "2" => kv::Every::TwoBars,
                            "4" => kv::Every::FourBars,
                            _ => kv::Every::Bar,
                        };
                    }
                    if let Some(o) = words.next().and_then(|o| kv::PartOrder::ALL.into_iter().find(|x| x.name().to_lowercase().starts_with(&o.to_lowercase()))) {
                        c.play.seq.order = o;
                    }
                }
            }
            "videosteps" | "videolookseq" => {
                let steps: Vec<Option<u8>> = arg.split(',').map(|w| w.trim().parse::<u8>().ok().map(|v| v.saturating_sub(1))).collect();
                if name == "videosteps" {
                    if let Some(c) = self.project.video.clips.get_mut(i) {
                        c.play.seq.steps = steps;
                    }
                } else {
                    self.project.video.look_seq.on = true;
                    self.project.video.look_seq.steps = steps;
                }
            }
            // Looks: videolookadd[=NAME] (a copy of the one edited), videolook=N (its pad
            // clicked), videolookfx=EFFECT,ALWAYS, videoparam=EFFECT,K,VALUE, videomorph=BEATS,
            // videocliplook=N (- none), videopartlook=PART,N, videolinkadd=SOURCE,EFFECT,AMOUNT.
            "videolookadd" => {
                let set = &mut self.project.video;
                if set.looks.len() < kv::MAX_LOOKS {
                    let mut l = set.edited_look().clone();
                    l.name = if arg.trim().is_empty() { format!("Look {}", set.looks.len() + 1) } else { arg.trim().to_string() };
                    set.looks.push(l);
                    set.look = set.looks.len() - 1;
                }
            }
            "videolook" => self.video_launch_look(n.unwrap_or(0)),
            "videolookfx" | "videoparam" => {
                let words: Vec<&str> = arg.split(',').map(str::trim).collect();
                let effect = words.first().and_then(|w| Effect::ALL.into_iter().find(|e| e.name().eq_ignore_ascii_case(w) || format!("{e:?}").eq_ignore_ascii_case(w)));
                let nums: Vec<f32> = words.iter().skip(1).filter_map(|w| w.parse().ok()).collect();
                if let Some(e) = effect {
                    let s = self.project.video.edited_look_mut().setting_mut(e);
                    match (name, nums.as_slice()) {
                        ("videolookfx", [v, ..]) => s.base = v.clamp(0.0, 1.0),
                        ("videoparam", [k, v, ..]) => s.set_param(e, *k as usize, *v),
                        _ => {}
                    }
                }
            }
            "videomorph" => self.project.video.morph_beats = f.first().copied().unwrap_or(0.0),
            "videocliplook" => {
                if let Some(c) = self.project.video.clips.get_mut(i) {
                    c.look = n;
                }
            }
            "videopartlook" => {
                if let (Some(c), [k, l, ..]) = (self.project.video.clips.get_mut(i), f.as_slice()) {
                    c.set_part_look(*k as usize, Some(*l as usize));
                }
            }
            "videolinkadd" => {
                let words: Vec<&str> = arg.split(',').map(str::trim).collect();
                let source = words.first().and_then(|w| Source::ALL.into_iter().find(|s| s.name().eq_ignore_ascii_case(w)));
                let effect = words.get(1).and_then(|w| Effect::ALL.into_iter().find(|e| e.name().eq_ignore_ascii_case(w)));
                let amount = words.get(2).and_then(|w| w.parse().ok()).unwrap_or(0.5);
                if let (Some(s), Some(e)) = (source, effect) {
                    self.project.video.edited_look_mut().links.push(VideoLink::new(s, e, amount));
                }
            }
            // Hits heard: videohits=WHAT[,BAND] (what: slices, parts, clips, looks; band:
            // kick, snare, hats, any), videohits=off.
            "videohits" => {
                let words: Vec<&str> = arg.split(',').map(str::trim).collect();
                let band = match words.get(1).copied().unwrap_or("kick") {
                    "snare" => kv::HitBand::Snare,
                    "hats" => kv::HitBand::Hats,
                    "any" => kv::HitBand::Any,
                    _ => kv::HitBand::Kick,
                };
                let t = HitTrigger { band, ..Default::default() };
                let set = &mut self.project.video;
                match words.first().copied().unwrap_or("") {
                    "slices" => {
                        if let Some(c) = set.clips.get_mut(i) {
                            c.play.slice_hits = Some(t);
                        }
                    }
                    "parts" => {
                        if let Some(c) = set.clips.get_mut(i) {
                            (c.play.seq.on, c.play.seq.order, c.play.seq.hits) = (true, kv::PartOrder::Hits, t);
                        }
                    }
                    "clips" => set.clip_hits = Some(t),
                    "looks" => set.look_hits = Some(t),
                    _ => {
                        set.clip_hits = None;
                        set.look_hits = None;
                    }
                }
            }
            // videoframe=OPACITY,ZOOM,X,Y,TURN: how the clip edited sits on the screen.
            "videoframe" => {
                if let Some(c) = self.project.video.clips.get_mut(i) {
                    let fr = &mut c.frame;
                    for (k, v) in f.iter().enumerate() {
                        match k {
                            0 => fr.opacity = *v,
                            1 => fr.zoom = *v,
                            2 => fr.x = *v,
                            3 => fr.y = *v,
                            _ => fr.turn = *v,
                        }
                    }
                }
            }
            // videorec[=PATH]: Record Video starts; videorecstop stops it.
            "videorec" => self.video_record_start((!arg.trim().is_empty()).then(|| PathBuf::from(arg.trim()))),
            "videorecstop" => self.video_record_stop(),
            // videoscroll=Y: the side panel scrolled down Y points.
            "videoscroll" => self.video.side_scroll = f.first().copied(),
            // videonow: prints the Look and the part that play (to check by script).
            "videonow" => {
                let d = &self.video.dir;
                println!("video now: clip {}, look {} (own {}), part {:?}, morph {:?}", d.switch.clip, d.look_playing(), d.pads.clip, d.playhead.part, d.look_from.as_ref().map(|f| (f.0, f.2)));
            }
            "videoexport" => {
                let mut words = arg.split(',');
                let path = PathBuf::from(words.next().unwrap_or("").trim());
                let rest: Vec<u32> = words.filter_map(|w| w.trim().parse().ok()).collect();
                if let Some(&(w, h)) = rest.first().and_then(|h| export::SIZES.iter().find(|s| s.1 == *h)) {
                    self.video.export_opts.width = w;
                    self.video.export_opts.height = h;
                }
                if let Some(fps) = rest.get(1) {
                    self.video.export_opts.fps = *fps;
                }
                self.start_video_export(path);
            }
            _ => return false,
        }
        true
    }
}

/// What the timeline's keys and buttons do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum VideoAct {
    Pause,
    /// A frame on (or back).
    Step(i32),
    MarkIn,
    MarkOut,
    Split,
    Delete,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_press_near_a_parts_edge_trims_it_and_inside_moves_it() {
        // A part 100 px wide from 200: 7 px at each end take the edges.
        assert_eq!(part_grab(201.0, 200.0, 300.0), Some(PartGrab::Left));
        assert_eq!(part_grab(206.5, 200.0, 300.0), Some(PartGrab::Left));
        assert_eq!(part_grab(250.0, 200.0, 300.0), Some(PartGrab::Move));
        assert_eq!(part_grab(294.0, 200.0, 300.0), Some(PartGrab::Right));
        // Just outside still takes the edge; further off, nothing.
        assert_eq!(part_grab(197.0, 200.0, 300.0), Some(PartGrab::Left));
        assert_eq!(part_grab(303.5, 200.0, 300.0), Some(PartGrab::Right));
        assert_eq!(part_grab(190.0, 200.0, 300.0), None);
        // A narrow part (12 px, less than three zones): in thirds, so it can be trimmed and moved.
        assert_eq!(part_grab(101.0, 100.0, 112.0), Some(PartGrab::Left));
        assert_eq!(part_grab(106.0, 100.0, 112.0), Some(PartGrab::Move));
        assert_eq!(part_grab(111.0, 100.0, 112.0), Some(PartGrab::Right));
        // The same seconds zoomed in ten times: the zone stays 7 pixels.
        assert_eq!(part_grab(2006.0, 2000.0, 3000.0), Some(PartGrab::Left));
        assert_eq!(part_grab(2010.0, 2000.0, 3000.0), Some(PartGrab::Move));
    }

    #[test]
    fn touching_parts_give_the_left_ones_end_and_a_gap_near_an_edge_takes_it() {
        let xs = [(100.0, 200.0), (200.0, 300.0), (400.0, 404.0)];
        assert_eq!(hit_parts(200.0, &xs), Some((0, PartGrab::Right)));
        assert_eq!(hit_parts(203.0, &xs), Some((1, PartGrab::Left)));
        assert_eq!(hit_parts(150.0, &xs), Some((0, PartGrab::Move)));
        // In the gap, a few pixels from the next part's start.
        assert_eq!(hit_parts(397.0, &xs), Some((2, PartGrab::Left)));
        assert_eq!(hit_parts(350.0, &xs), None);
        // A sliver of 4 px still has a middle to move it by.
        assert_eq!(hit_parts(402.0, &xs), Some((2, PartGrab::Move)));
    }
}

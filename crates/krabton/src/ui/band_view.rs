//! The band in the editors: a Band section on every device that can be in
//! it (its switch; with it on, how it follows, the band's chord and part
//! now with the part's energy, and the way to the Band window), and the
//! Band window, where the song is arranged in parts: the part playing and
//! the next part, the parts as cards, the part picked with its chords and
//! how the band plays it (energy, dynamics, key, swing, feel and how it
//! ends), the form with the one playing lit and how far through it is, and
//! who plays in which part.

use crate::app::App;
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, Context, FontId, Id, Rect, RichText, Sense, Stroke, Ui};
use krabton_engine::band::{self as kb, BandPart, BandStatus, Cue, FormStep, PartPlay, Role, FEELS, FOLLOWS, LETTERS, MAX_FORM, MAX_PARTS, MODES, NONE, PART_NAMES};
use krabton_engine::classic_seq::{chord_name, CHORD_LENGTHS, PROG_SLOTS, QUALITIES};
use krabton_engine::midi_fx::MidiFxKind;
use krabton_engine::params::{ParamSpec, Unit};
use krabton_engine::sequencers::players::band_param;
use krabton_engine::sequencers::strum::chord_letter_name;
use krabton_engine::Scale;

/// A colour per part, A to F.
const PART_COLORS: [Color32; MAX_PARTS] = [
    Color32::from_rgb(0x5b, 0x8d, 0xd9),
    Color32::from_rgb(0x4c, 0xb8, 0x7a),
    Color32::from_rgb(0xe0, 0x8a, 0x3c),
    Color32::from_rgb(0xb0, 0x6c, 0xd8),
    Color32::from_rgb(0xd8, 0x5a, 0x7a),
    Color32::from_rgb(0x3c, 0xb4, 0xc0),
];

pub(crate) fn part_color(i: usize) -> Color32 {
    PART_COLORS[i % MAX_PARTS]
}

/// A part's knobs (see `PartPlay`).
static ENERGY: ParamSpec = ParamSpec::lin("Energy", "Part", 0.0, 1.0, 0.5, Unit::Percent);
static DYNAMICS: ParamSpec = ParamSpec::lin("Dynamics", "Part", -12.0, 6.0, 0.0, Unit::Db);
static TRANSPOSE: ParamSpec = ParamSpec::int("Transpose", "Part", -12.0, 12.0, 0.0, Unit::Semi);
static SWING: ParamSpec = ParamSpec::lin("Swing", "Part", 0.0, 1.0, 0.0, Unit::Percent);

/// What a device's Band section shows of the band.
pub(crate) struct BandLive {
    pub st: BandStatus,
    /// The parts' names (none: one progression) and how often the form plays the one now.
    pub names: Vec<String>,
    pub times: Option<u8>,
    /// The key the part playing is in.
    pub scale: Scale,
    /// The part playing's energy (none without parts).
    pub energy: Option<f32>,
}

impl App {
    pub(crate) fn band_live(&self) -> BandLive {
        let st = self.band_status();
        let times = self.project.band.form.get(st.step as usize).filter(|_| st.step != NONE).map(|f| f.times.max(1));
        let part = self.project.band.parts.get(st.part as usize).filter(|_| st.parts);
        BandLive {
            st,
            names: self.project.band.parts.iter().map(|p| p.name.clone()).collect(),
            times,
            scale: part.map(|p| p.play.key(self.project.scale)).unwrap_or(self.project.scale),
            energy: part.map(|p| p.play.energy),
        }
    }
}

/// Whether a kind can be in the band (a chord player, one that follows it, or drums).
pub(crate) fn in_band_kind(kind: MidiFxKind) -> bool {
    kb::switch_param(kind).is_some()
}

/// The band's chord now: its numeral and its name ("vi", "Am").
fn chord_now(live: &BandLive) -> Option<(String, String)> {
    let st = &live.st;
    (st.on && st.slot != NONE && st.degree > 0).then(|| (chord_name(&live.scale, st.degree, st.quality), chord_letter_name(&live.scale, st.degree, st.quality)))
}

/// The part now, and which time round ("Chorus 2/2"), or the one progression.
fn part_now(live: &BandLive) -> String {
    let st = &live.st;
    match live.names.get(st.part as usize).filter(|_| st.parts) {
        Some(name) => match live.times {
            Some(t) if t > 1 => format!("{name} {}/{t}", st.pass as u32 + 1),
            _ => name.clone(),
        },
        None => "One progression".into(),
    }
}

/// A device's Band section. With Band off only its switch is there. Returns
/// (changed, open the Band window, a cue for the band).
pub(crate) fn band_section(ui: &mut Ui, id: Id, kind: MidiFxKind, params: &mut [f32], live: &BandLive, accent: Color32, height: f32) -> (bool, bool, Option<Option<Cue>>) {
    let w = 112.0;
    let (rect, _) = ui.allocate_exact_size(vec2(w, height.max(60.0)), Sense::hover());
    let p = ui.painter().clone();
    let follows = kb::follow_param(kind);
    let Some(bi) = kb::switch_param(kind) else { return (false, false, None) };
    let mut changed = false;
    let mut open = false;
    let mut cue = None;
    text(&p, pos2(rect.left() + 5.0, rect.top() + 6.0), Align2::LEFT_CENTER, "Band", small(), shade(accent, 0.85));
    p.line_segment([pos2(rect.left(), rect.top() + 2.0), pos2(rect.left(), rect.bottom() - 2.0)], egui::Stroke::new(1.0_f32, LINE_SOFT));
    let x = rect.left() + 6.0;
    let bw = w - 10.0;
    let mut y = rect.top() + 14.0;
    let on = params.get(bi).is_some_and(|v| *v >= 0.5);
    let sw = Rect::from_min_size(pos2(x, y), vec2(bw, 28.0));
    let tip = if follows.is_some() {
        "In the band: its notes follow the band's chords, and it plays the parts as they say (energy, dynamics, key, who plays where). Off: it plays as written"
    } else if band_param(kind).is_some() {
        "In the band: it plays the band's chords, the same as every chord player in it; change them here and the others follow"
    } else {
        "In the band: it plays the parts as they say (energy, dynamics, fills, builds and breaks, who plays where); its notes stay drums"
    };
    if button(ui, sw, id.with("on"), if on { "In the band" } else { "Band off" }, on, accent).on_hover_text(tip).clicked() {
        if let Some(v) = params.get_mut(bi) {
            *v = if on { 0.0 } else { 1.0 };
            changed = true;
        }
    }
    if !on {
        return (changed, false, None);
    }
    y += 32.0;
    if let Some(fi) = follows.map(|b| b + 1) {
        let n = FOLLOWS.len();
        let mode = params.get(fi).map(|v| (v.round().max(0.0) as usize).min(n - 1)).unwrap_or(0);
        let r = Rect::from_min_size(pos2(x, y), vec2(bw, 22.0));
        let resp = button(ui, r, id.with("follow"), &format!("Follow: {}", FOLLOWS[mode]), false, accent);
        let resp = resp.on_hover_text("How it follows: Root moves its notes with the chord's root, Chord also onto the chord's notes, Bass onto the chord's root and fifth. Click for the next, right-click for the previous");
        let step = if resp.clicked() { Some(1) } else if resp.secondary_clicked() { Some(n - 1) } else { None };
        if let (Some(s), Some(v)) = (step, params.get_mut(fi)) {
            *v = ((mode + s) % n) as f32;
            changed = true;
        }
        y += 26.0;
    }
    // The band's chord now.
    let box_h = (rect.bottom() - y - 52.0).clamp(30.0, 56.0);
    let cr = Rect::from_min_size(pos2(x, y), vec2(bw, box_h));
    rect_fill(&p, cr, 4.0, PANEL3);
    match chord_now(live) {
        Some((numeral, name)) => {
            text(&p, pos2(cr.center().x, cr.center().y - 6.0), Align2::CENTER_CENTER, &numeral, FontId::proportional(20.0), TEXT);
            text(&p, pos2(cr.center().x, cr.bottom() - 8.0), Align2::CENTER_CENTER, &name, small(), TEXT_DIM);
        }
        None => {
            let what = if live.st.on { "-" } else { "No band yet" };
            text(&p, cr.center(), Align2::CENTER_CENTER, what, small(), TEXT_DIM);
        }
    }
    y += box_h + 4.0;
    // The part now: a click goes on to the next part; along its bottom the part's energy.
    let pr = Rect::from_min_size(pos2(x, y), vec2(bw, 22.0));
    let cued = live.st.cued != NONE;
    let label = match live.names.get(live.st.cued as usize).filter(|_| cued) {
        Some(next) => format!("> {next}"),
        None => part_now(live),
    };
    let lit = cued && super::session::blink();
    let resp = button(ui, pr, id.with("part"), &label, lit, live.names.get(live.st.part as usize).map(|_| part_color(live.st.part as usize)).unwrap_or(accent));
    if let Some(e) = live.energy {
        let bar = Rect::from_min_size(pos2(pr.left() + 2.0, pr.bottom() - 3.0), vec2((bw - 4.0) * e.clamp(0.0, 1.0), 2.0));
        ui.painter().rect_filled(bar, 1.0, part_color(live.st.part as usize));
    }
    if live.names.is_empty() {
        resp.on_hover_text("The band plays one progression; give it parts (Verse, Chorus...) in the Band window");
    } else {
        let energy = live.energy.map(|e| format!(" (the line along its bottom: its energy, {:.0}%)", e * 100.0)).unwrap_or_default();
        let resp = resp.on_hover_text(format!("The part playing{energy}. Click: the next part when this one ends; right-click: not after all"));
        if resp.clicked() {
            cue = Some(Some(Cue::Next));
        }
        if resp.secondary_clicked() {
            cue = Some(None);
        }
    }
    y += 26.0;
    let wr = Rect::from_min_size(pos2(x, y), vec2(bw, 20.0));
    if button(ui, wr, id.with("window"), "Parts...", false, accent).on_hover_text("The Band window: the song's parts, how the band plays each, the form and who plays where").clicked() {
        open = true;
    }
    (changed, open, cue)
}

/// Common progressions to start a part from.
const PROGRESSIONS: &[(&str, &[(u8, u8)])] = &[
    ("I - V - vi - IV", &[(1, 0), (5, 0), (6, 0), (4, 0)]),
    ("vi - IV - I - V", &[(6, 0), (4, 0), (1, 0), (5, 0)]),
    ("I - vi - IV - V", &[(1, 0), (6, 0), (4, 0), (5, 0)]),
    ("ii - V - I", &[(2, 1), (5, 1), (1, 1), (1, 1)]),
    ("IV - V - iii - vi", &[(4, 0), (5, 0), (3, 0), (6, 0)]),
    ("i - VI - III - VII", &[(1, 0), (6, 0), (3, 0), (7, 0)]),
    ("Andalusian", &[(1, 0), (7, 0), (6, 0), (5, 0)]),
    ("Blues", &[(1, 1), (1, 1), (4, 1), (1, 1), (5, 1), (4, 1), (1, 1), (5, 1)]),
    ("Bridge IV - V", &[(4, 0), (4, 0), (5, 0), (5, 0)]),
];

/// The chords of a part in short, in its key: "I V vi IV".
fn summary(scale: &Scale, part: &BandPart) -> String {
    let key = part.play.key(*scale);
    part.chords.iter().take(part.count as usize).map(|c| chord_name(&key, c.0, c.1)).collect::<Vec<_>>().join(" ")
}

/// A word for an energy.
fn energy_word(e: f32) -> &'static str {
    match e {
        e if e < 0.15 => "bare",
        e if e < 0.35 => "sparse",
        e if e < 0.45 => "lighter",
        e if e <= 0.55 => "as written",
        e if e < 0.8 => "fuller",
        _ => "full out",
    }
}

/// How the band plays a part, in a few words: "fuller, +3 dB, up 1, fill".
fn play_words(play: &PartPlay) -> String {
    let mut w: Vec<String> = Vec::new();
    if (play.energy - 0.5).abs() > 0.05 {
        w.push(energy_word(play.energy).into());
    }
    if play.dynamics.abs() >= 0.5 {
        w.push(format!("{:+.0} dB", play.dynamics));
    }
    match play.transpose {
        0 => {}
        t if t > 0 => w.push(format!("up {t}")),
        t => w.push(format!("down {}", -t)),
    }
    if play.mode > 0 {
        w.push(MODES[play.mode as usize % MODES.len()].to_lowercase());
    }
    if play.swing > 0.0 {
        w.push(format!("swing {:.0}%", play.swing * 100.0));
    }
    if play.feel > 0 {
        w.push(FEELS[play.feel as usize % FEELS.len()].to_lowercase());
    }
    if play.fill {
        w.push("fill".into());
    }
    if play.build > 0 {
        w.push("build".into());
    }
    if play.brk > 0 {
        w.push("break".into());
    }
    if w.is_empty() {
        "as written".into()
    } else {
        w.join(", ")
    }
}

/// A big flat button that lights up; click and right-click.
fn pad(ui: &mut Ui, r: Rect, id: Id, label: &str, fill: Color32, tip: &str) -> egui::Response {
    let resp = ui.interact(r, id, Sense::click());
    let p = ui.painter();
    let fill = if resp.hovered() { mix(fill, Color32::WHITE, 0.08) } else { fill };
    p.rect_filled(r.translate(vec2(0.0, 2.0)), 4.0, Color32::from_black_alpha(90));
    rect_fill(p, r, 4.0, fill);
    p.with_clip_rect(r).text(r.center(), Align2::CENTER_CENTER, label, FontId::proportional(13.0), text_on(fill));
    resp.on_hover_text(tip)
}

/// A big knob for one of a part's settings: its name above, what it is now
/// below (and a word for it). True when it changed.
#[allow(clippy::too_many_arguments)]
fn part_knob(ui: &mut Ui, cell: Rect, id: Id, spec: &ParamSpec, value: &mut f32, bipolar: bool, shown: &str, word: &str, tip: &str, accent: Color32) -> bool {
    let p = ui.painter().clone();
    text(&p, pos2(cell.center().x, cell.top() + 7.0), Align2::CENTER_CENTER, spec.name, small(), TEXT_DIM);
    let k = Rect::from_center_size(pos2(cell.center().x, cell.top() + 50.0), vec2(64.0, 64.0));
    let steps = if spec.is_discrete() { (spec.max - spec.min).round().max(1.0) } else { 0.0 };
    let speed = if steps > 0.0 { (167.0 / (steps * 12.0)).max(1.0) } else { 1.0 };
    let (resp, new) = knob_with_speed(ui, k, id, spec.to_norm(*value), spec.to_norm(spec.default), bipolar, accent, speed);
    text(&p, pos2(cell.center().x, cell.bottom() - 22.0), Align2::CENTER_CENTER, shown, body(), TEXT);
    text(&p, pos2(cell.center().x, cell.bottom() - 8.0), Align2::CENTER_CENTER, word, small(), TEXT_DIM);
    resp.on_hover_text(format!("{tip}\nDrag up or down (Shift: fine); double-click: back to {}", spec.format(spec.default)));
    match new.map(|n| spec.from_norm(n)) {
        Some(v) if v != *value => {
            *value = v;
            true
        }
        _ => false,
    }
}

/// A choice as a big button with its name above: click for the next, right-click for the previous.
#[allow(clippy::too_many_arguments)]
fn choice(ui: &mut Ui, cell: Rect, id: Id, name: &str, options: &[&str], value: &mut u8, tip: &str, color: Color32) -> bool {
    let p = ui.painter().clone();
    text(&p, pos2(cell.left() + 2.0, cell.top() + 6.0), Align2::LEFT_CENTER, name, small(), TEXT_DIM);
    let n = options.len().max(1) as u8;
    let v = *value % n;
    let r = Rect::from_min_max(pos2(cell.left(), cell.top() + 14.0), cell.max);
    let fill = if v > 0 { shade(color, 0.85) } else { PANEL3 };
    let resp = pad(ui, r, id, options[v as usize], fill, &format!("{tip}\nClick: the next, right-click: the previous"));
    let to = if resp.clicked() {
        (v + 1) % n
    } else if resp.secondary_clicked() {
        (v + n - 1) % n
    } else {
        return false;
    };
    *value = to;
    true
}

/// A heading for a part of the window, and what it is in a few words.
fn heading(ui: &mut Ui, title: &str, hint: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(title).strong().color(TEXT_DIM));
        ui.add_space(6.0);
        ui.label(RichText::new(hint).small().color(TEXT_DIM));
    });
}

/// A member of the band: its device, its track and what it plays as.
struct Member {
    id: u64,
    name: &'static str,
    track: String,
    color: Color32,
    role: Role,
}

impl Member {
    fn plays_as(&self) -> String {
        match self.role {
            Role::Chords => "chords".into(),
            Role::Follows(m) => format!("follows: {}", FOLLOWS[(m as usize).min(FOLLOWS.len() - 1)]),
            Role::Drums => "drums".into(),
        }
    }
}

impl App {
    /// The Band window (from a device's Band section or the Studio's bar).
    pub(crate) fn ui_band_window(&mut self, ctx: &Context) {
        if !self.band.window {
            return;
        }
        let (open, _) = super::windows::tool_window(ctx, "band", "Band", vec2(940.0, 680.0), |ui| self.band_body(ui));
        if !open {
            self.band.window = false;
        }
    }

    /// Who is in the band, in the order of the tracks.
    fn band_members(&self) -> Vec<Member> {
        let mut v = Vec::new();
        for tr in &self.project.tracks {
            for dev in &tr.midi_devices {
                if let Some(role) = kb::role(dev.kind, &dev.params, false) {
                    v.push(Member { id: dev.id, name: dev.kind.name(), track: tr.name.clone(), color: rgb(tr.color), role });
                }
            }
        }
        v
    }

    fn band_body(&mut self, ui: &mut Ui) {
        let st = self.band_status();
        let scale = self.project.scale;
        let accent = accent_now();
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            if self.project.band.parts.is_empty() {
                ui.label(RichText::new("The band plays one progression: the one its chord players share.").color(TEXT));
                ui.label(RichText::new("Arrange it in parts (a Verse, a Chorus, a Bridge...): each has chords of its own and says how the band plays it (fuller or sparser, softer or harder, in which key, with a fill or a break at its end) and who plays in it. A form plays them in turn, and everyone moves on together.").small().color(TEXT_DIM));
                ui.add_space(8.0);
                let (r, _) = ui.allocate_exact_size(vec2(260.0, 44.0), Sense::hover());
                if pad(ui, r, Id::new("band_make"), "Make parts: Verse and Chorus", accent, "The band's chords become the Verse (with a fill at its end), a fuller Chorus to change; the form Verse twice, Chorus twice").clicked() {
                    self.make_band_parts();
                }
                ui.add_space(10.0);
                let members = self.band_members();
                let text = if members.is_empty() { "Nobody in the band yet: switch Band on in a chord player, a sequencer or a drummer.".to_string() } else { format!("In the band: {}", members.iter().map(|m| format!("{} ({}, {})", m.name, m.track, m.plays_as())).collect::<Vec<_>>().join(", ")) };
                ui.label(RichText::new(text).small().color(TEXT_DIM));
                return;
            }
            self.band_now_row(ui, st, accent);
            ui.add_space(10.0);
            heading(ui, "PARTS", "Click a part to edit it below; its play button goes there when the part playing ends");
            self.band_cards(ui, st, &scale, accent);
            ui.add_space(8.0);
            self.band_part_panel(ui, st, &scale, accent);
            ui.add_space(10.0);
            heading(ui, "FORM", "The parts in the order they play, each so many times, then round again");
            self.band_form_row(ui, st, accent);
            ui.add_space(10.0);
            heading(ui, "WHO PLAYS WHERE", "Click a square: that member sits that part out, or plays in it again");
            self.band_matrix(ui, st);
        });
    }

    /// The parts from the band's progression: a Verse that fills into a
    /// fuller Chorus, the form.
    pub(crate) fn make_band_parts(&mut self) {
        let part = self.band_first_part().unwrap_or_else(|| BandPart::from_part("", &[2, 4, 1, 0, 5, 0, 6, 0, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0]).part());
        let mut verse = BandPart::from_part(PART_NAMES[0], &part);
        verse.play.fill = true;
        let mut chorus = BandPart::from_part(PART_NAMES[1], &part);
        chorus.count = 4;
        chorus.chords[..4].copy_from_slice(&[(4, 0), (5, 0), (1, 0), (6, 0)]);
        chorus.play.energy = 0.7;
        chorus.play.fill = true;
        self.project.band.parts = vec![verse, chorus];
        self.project.band.form = vec![FormStep { part: 0, times: 2 }, FormStep { part: 1, times: 2 }];
        self.band.editing = 0;
    }

    /// What the band's chord players share now (the first one's).
    fn band_first_part(&self) -> Option<Vec<u8>> {
        self.project.tracks.iter().flat_map(|t| &t.midi_devices).find(|d| band_param(d.kind).and_then(|b| d.params.get(b)).is_some_and(|v| *v >= 0.5)).map(|d| krabton_engine::sequencers::players::band_part(&d.params, &d.data))
    }

    /// The part playing (how far through it, its chord), the next part, and how a cue waits.
    fn band_now_row(&mut self, ui: &mut Ui, st: BandStatus, accent: Color32) {
        let (row, _) = ui.allocate_exact_size(vec2(ui.available_width().max(600.0), 54.0), Sense::hover());
        let p = ui.painter().clone();
        let parts = &self.project.band.parts;
        let now = (st.part as usize).min(parts.len() - 1);
        let nr = Rect::from_min_size(row.min, vec2(300.0, 54.0));
        rect_fill(&p, nr, 4.0, PANEL3);
        rect_fill(&p, Rect::from_min_size(nr.min, vec2(8.0, 54.0)), 2.0, part_color(now));
        let live = BandLive { st, names: parts.iter().map(|p| p.name.clone()).collect(), times: self.project.band.form.get(st.step as usize).filter(|_| st.step != NONE).map(|f| f.times.max(1)), scale: parts[now].play.key(self.project.scale), energy: None };
        text(&p, pos2(nr.left() + 16.0, nr.top() + 14.0), Align2::LEFT_CENTER, format!("{}  {}", LETTERS[now], part_now(&live)), FontId::proportional(15.0), TEXT);
        let off_form = if st.step == NONE && !self.project.band.form.is_empty() { "  (off the form)" } else { "" };
        p.with_clip_rect(nr.shrink(2.0)).text(pos2(nr.left() + 16.0, nr.top() + 33.0), Align2::LEFT_CENTER, format!("{}{off_form}", summary(&self.project.scale, &parts[now])), small(), TEXT_DIM);
        if let Some((numeral, name)) = chord_now(&live) {
            text(&p, pos2(nr.right() - 10.0, nr.center().y - 3.0), Align2::RIGHT_CENTER, format!("{numeral}  {name}"), FontId::proportional(18.0), accent);
        }
        // How far through the part.
        let track = Rect::from_min_size(pos2(nr.left() + 16.0, nr.bottom() - 9.0), vec2(nr.width() - 26.0, 4.0));
        rect_fill(&p, track, 2.0, PANEL);
        if st.parts {
            rect_fill(&p, Rect::from_min_size(track.min, vec2(track.width() * st.through as f32 / 255.0, 4.0)), 2.0, part_color(now));
        }
        // The next part: when this one ends (or on the bar).
        let cued = st.cued != NONE;
        let nx = Rect::from_min_size(pos2(nr.right() + 10.0, row.top()), vec2(210.0, 54.0));
        let label = match parts.get(st.cued as usize).filter(|_| cued) {
            Some(next) => format!("Next: {}  (waiting)", next.name),
            None => "Next part  >".to_string(),
        };
        let fill = if cued && super::session::blink() { part_color(st.cued as usize) } else { shade(accent, 0.75) };
        let resp = pad(ui, nx, Id::new("band_next"), &label, fill, "Everyone on to the next part when this one ends (or on the bar). Right-click: not after all");
        if resp.clicked() {
            self.cmd(krabton_engine::Command::BandCue(Some(Cue::Next)));
        }
        if resp.secondary_clicked() {
            self.cmd(krabton_engine::Command::BandCue(None));
        }
        let br = Rect::from_min_size(pos2(nx.right() + 10.0, row.top() + 12.0), vec2(96.0, 30.0));
        let on_bar = self.project.band.cue_on_bar;
        if button(ui, br, Id::new("band_on_bar"), if on_bar { "On the bar" } else { "At its end" }, on_bar, accent).on_hover_text("When the next part (or a part picked) starts: at the end of the part playing, or on the next bar").clicked() {
            self.project.band.cue_on_bar = !on_bar;
        }
        let xr = Rect::from_min_size(pos2(br.right() + 10.0, row.top() + 12.0), vec2(110.0, 30.0));
        if button(ui, xr, Id::new("band_no_parts"), "One progression", false, accent).on_hover_text("No parts: the band plays one progression again (the part playing). Ctrl+Z brings them back").clicked() {
            self.project.band = Default::default();
        }
    }

    /// The parts as cards: click one to edit it, its play button to go to
    /// it; the line along the bottom is its energy.
    fn band_cards(&mut self, ui: &mut Ui, st: BandStatus, scale: &Scale, accent: Color32) {
        let n = self.project.band.parts.len();
        let (cw, ch) = (130.0, 76.0);
        let (row, _) = ui.allocate_exact_size(vec2((n.min(MAX_PARTS - 1) + 1) as f32 * (cw + 8.0), ch), Sense::hover());
        let mut add = false;
        for i in 0..n {
            let r = Rect::from_min_size(pos2(row.left() + i as f32 * (cw + 8.0), row.top()), vec2(cw, ch));
            let part = &self.project.band.parts[i];
            let playing = st.parts && st.part as usize == i;
            let cued = st.cued as usize == i && super::session::blink();
            let editing = self.band.editing == i;
            let col = part_color(i);
            let resp = ui.interact(r, Id::new(("band_card", i)), Sense::click());
            let p = ui.painter().clone();
            let fill = if playing || cued { shade(col, 0.7) } else if resp.hovered() { shade(PANEL3, 1.2) } else { PANEL3 };
            p.rect_filled(r.translate(vec2(0.0, 2.0)), 4.0, Color32::from_black_alpha(90));
            rect_fill(&p, r, 4.0, fill);
            if editing {
                rect_line(&p, r, 4.0, 2.0, accent);
            }
            let ink = text_on(fill);
            text(&p, pos2(r.left() + 8.0, r.top() + 15.0), Align2::LEFT_CENTER, LETTERS[i], crate::skin::Font::Block.id(19.0), if playing || cued { ink } else { col });
            p.with_clip_rect(r.shrink(2.0)).text(pos2(r.left() + 28.0, r.top() + 15.0), Align2::LEFT_CENTER, &part.name, FontId::proportional(13.0), ink);
            p.with_clip_rect(r.shrink(2.0)).text(pos2(r.left() + 8.0, r.top() + 36.0), Align2::LEFT_CENTER, summary(scale, part), small(), ink);
            text(&p, pos2(r.left() + 8.0, r.bottom() - 16.0), Align2::LEFT_CENTER, format!("{} x {}", part.count, CHORD_LENGTHS[part.length.min(3) as usize]), small(), if playing { ink } else { TEXT_DIM });
            // Its energy, and how far through it while it plays.
            let e = Rect::from_min_size(pos2(r.left() + 6.0, r.bottom() - 5.0), vec2((r.width() - 44.0) * part.play.energy.clamp(0.0, 1.0), 3.0));
            p.rect_filled(e, 1.0, if playing { ink } else { col });
            if playing {
                let x = r.left() + 4.0 + (r.width() - 8.0) * st.through as f32 / 255.0;
                p.line_segment([pos2(x, r.top() + 2.0), pos2(x, r.top() + 6.0)], Stroke::new(2.0_f32, ink));
            }
            if resp.on_hover_text(format!("{}: {}. Click to edit it", part.name, play_words(&part.play))).clicked() {
                self.band.editing = i;
            }
            let go = Rect::from_min_size(pos2(r.right() - 32.0, r.bottom() - 30.0), vec2(28.0, 24.0));
            let gresp = ui.interact(go, Id::new(("band_go", i)), Sense::click());
            rect_fill(ui.painter(), go, 3.0, if gresp.hovered() { accent } else { shade(PANEL2, 0.9) });
            play_icon(ui.painter(), go.center(), 9.0, if gresp.hovered() { TEXT_DARK } else { TEXT });
            if gresp.on_hover_text("Everyone to this part when the one playing ends (or on the bar); stopped, the song starts with it").clicked() {
                self.cmd(krabton_engine::Command::BandCue(Some(Cue::Part(i as u8))));
            }
        }
        if n < MAX_PARTS {
            let r = Rect::from_min_size(pos2(row.left() + n as f32 * (cw + 8.0), row.top()), vec2(cw * 0.6, ch));
            if pad(ui, r, Id::new("band_add_part"), "+ Part", PANEL2, "A new part, a copy of the one you edit to change").clicked() {
                add = true;
            }
        }
        if add {
            let from = self.project.band.parts.get(self.band.editing).cloned();
            let mut part = from.unwrap_or_else(|| BandPart::from_part("", &[2, 4, 1, 0, 5, 0, 6, 0, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0]));
            part.name = PART_NAMES[n % MAX_PARTS].to_string();
            self.project.band.parts.push(part);
            self.band.editing = n;
        }
    }

    /// The part picked, in one panel: its name and chords, and how the band plays it.
    fn band_part_panel(&mut self, ui: &mut Ui, st: BandStatus, scale: &Scale, accent: Color32) {
        let n = self.project.band.parts.len();
        let e = self.band.editing.min(n - 1);
        self.band.editing = e;
        egui::Frame::new().fill(PANEL2).corner_radius(6).inner_margin(egui::Margin::same(10)).stroke(Stroke::new(1.0_f32, shade(part_color(e), 0.7))).show(ui, |ui| {
            self.band_part_editor(ui, st, scale, accent);
            if let Some(i) = self.project.band.parts.len().checked_sub(1).map(|last| self.band.editing.min(last)) {
                ui.add_space(10.0);
                self.band_part_play(ui, i, scale);
            }
        });
    }

    /// The part being edited: its name, how long its chords last, how many,
    /// and the chords (click: the next, right-click: the previous).
    fn band_part_editor(&mut self, ui: &mut Ui, st: BandStatus, scale: &Scale, accent: Color32) {
        let n = self.project.band.parts.len();
        let e = self.band.editing.min(n - 1);
        let mut delete = false;
        let mut duplicate = false;
        let key = self.project.band.parts[e].play.key(*scale);
        ui.horizontal(|ui| {
            ui.label(RichText::new(LETTERS[e]).strong().size(18.0).color(part_color(e)));
            let part = &mut self.project.band.parts[e];
            ui.add(egui::TextEdit::singleline(&mut part.name).desired_width(120.0)).on_hover_text("The part's name");
            let len = part.length.min(3) as usize;
            let resp = ui.add(egui::Button::new(RichText::new(format!("Each chord {}", CHORD_LENGTHS[len])).color(TEXT)).fill(PANEL3)).on_hover_text("How long each chord lasts: click for longer, right-click for shorter");
            if resp.clicked() {
                part.length = ((len + 1) % 4) as u8;
            }
            if resp.secondary_clicked() {
                part.length = ((len + 3) % 4) as u8;
            }
            let count = part.count.clamp(1, PROG_SLOTS as u8);
            let resp = ui.add(egui::Button::new(RichText::new(format!("{count} chords")).color(TEXT)).fill(PANEL3)).on_hover_text("How many chords: click for one more, right-click for one fewer");
            if resp.clicked() {
                part.count = (count % PROG_SLOTS as u8) + 1;
            }
            if resp.secondary_clicked() {
                part.count = if count <= 1 { PROG_SLOTS as u8 } else { count - 1 };
            }
            ui.menu_button(RichText::new("Progressions").color(TEXT), |ui| {
                for (name, chords) in PROGRESSIONS {
                    if ui.button(*name).clicked() {
                        part.count = chords.len() as u8;
                        part.chords[..chords.len()].copy_from_slice(chords);
                        ui.close();
                    }
                }
            });
            if n < MAX_PARTS && ui.add(egui::Button::new(RichText::new("Duplicate").color(TEXT)).fill(PANEL3)).on_hover_text("A copy of this part, to change").clicked() {
                duplicate = true;
            }
            if ui.add(egui::Button::new(RichText::new("Delete").color(TEXT)).fill(PANEL3)).on_hover_text("This part goes, and its steps in the form").clicked() {
                delete = true;
            }
            ui.add_space(8.0);
            ui.label(RichText::new(format!("in {}", key.name())).color(TEXT_DIM)).on_hover_text("The key this part plays in (the song's key, or as its Key and Transpose make it)");
        });
        ui.add_space(4.0);
        let (cw, ch) = (78.0, 62.0);
        let (row, _) = ui.allocate_exact_size(vec2(PROG_SLOTS as f32 * (cw + 6.0), ch), Sense::hover());
        let p = ui.painter().clone();
        let part = &mut self.project.band.parts[e];
        for slot in 0..PROG_SLOTS {
            let r = Rect::from_min_size(pos2(row.left() + slot as f32 * (cw + 6.0), row.top()), vec2(cw, ch));
            let (degree, quality) = part.chords[slot];
            let used = slot < part.count as usize;
            let playing = st.parts && st.part as usize == e && st.slot as usize == slot;
            rect_fill(&p, r, 4.0, if playing { shade(accent, 0.55) } else if used { PANEL3 } else { PANEL });
            if playing {
                rect_line(&p, r, 4.0, 1.5, accent);
            }
            text(&p, pos2(r.left() + 4.0, r.top() + 7.0), Align2::LEFT_CENTER, (slot + 1).to_string(), small(), TEXT_DIM);
            let top = Rect::from_min_size(r.min, vec2(cw, ch - 20.0));
            text(&p, top.center() + vec2(0.0, 2.0), Align2::CENTER_CENTER, chord_name(&key, degree, quality), FontId::proportional(18.0), if used { TEXT } else { TEXT_DIM });
            let resp = ui.interact(top, Id::new(("band_deg", e, slot)), Sense::click());
            let letter = if degree == 0 { "rest".to_string() } else { chord_letter_name(&key, degree, quality) };
            let resp = resp.on_hover_text(format!("{letter} · click: next chord, right-click: previous"));
            if resp.clicked() {
                part.chords[slot].0 = (degree + 1) % 8;
            }
            if resp.secondary_clicked() {
                part.chords[slot].0 = (degree + 7) % 8;
            }
            let q = Rect::from_min_size(pos2(r.left() + 3.0, r.bottom() - 18.0), vec2(cw - 6.0, 15.0));
            let nq = QUALITIES.len() as u8;
            let resp = button(ui, q, Id::new(("band_q", e, slot)), QUALITIES[(quality % nq) as usize], false, accent).on_hover_text("Chord type: click for the next, right-click for the previous");
            if resp.clicked() {
                part.chords[slot].1 = (quality + 1) % nq;
            }
            if resp.secondary_clicked() {
                part.chords[slot].1 = (quality + nq - 1) % nq;
            }
        }
        if duplicate {
            let mut copy = self.project.band.parts[e].clone();
            copy.name = format!("{} 2", copy.name);
            self.project.band.parts.push(copy);
            self.band.editing = n;
        }
        if delete {
            self.delete_band_part(e);
        }
    }

    /// How the band plays part `e`: four knobs (energy, dynamics,
    /// transpose, swing), its key and feel, and how it ends.
    fn band_part_play(&mut self, ui: &mut Ui, e: usize, scale: &Scale) {
        let col = part_color(e);
        let mut play = self.project.band.parts[e].play;
        ui.label(RichText::new(format!("HOW THE BAND PLAYS {}", self.project.band.parts[e].name.to_uppercase())).small().strong().color(TEXT_DIM));
        let (row, _) = ui.allocate_exact_size(vec2(ui.available_width().max(860.0), 112.0), Sense::hover());
        let p = ui.painter().clone();
        let (kw, kh) = (88.0, 112.0);
        let cell = |i: usize| Rect::from_min_size(pos2(row.left() + i as f32 * (kw + 4.0), row.top()), vec2(kw, kh));
        let mut changed = false;
        let shown = format!("{:.0}%", play.energy * 100.0);
        let word = energy_word(play.energy);
        changed |= part_knob(ui, cell(0), Id::new(("band_energy", e)), &ENERGY, &mut play.energy, true, &shown, word, "Energy: how full the band plays this part. 50%: as written. Lower: fewer notes (the off-beats go first, the bar's one stays) and the drummers play simpler and softer. Higher: the strong notes an octave up too, and busier drums", col);
        let dyn_word = if play.dynamics <= -0.5 { "softer" } else if play.dynamics >= 0.5 { "harder" } else { "as written" };
        let shown = format!("{:+.0} dB", play.dynamics);
        changed |= part_knob(ui, cell(1), Id::new(("band_dynamics", e)), &DYNAMICS, &mut play.dynamics, false, &shown, dyn_word, "Dynamics: how hard the band plays this part, every note softer or harder (-12 dB: half as hard)", col);
        let mut t = play.transpose as f32;
        let t_word = match play.transpose {
            0 => "same key",
            1 | -1 => "semitone",
            _ => "semitones",
        };
        if part_knob(ui, cell(2), Id::new(("band_transpose", e)), &TRANSPOSE, &mut t, true, &format!("{:+}", play.transpose), t_word, "Transpose: the whole part up or down in semitones, the chords and everyone following them (the last chorus a half step up). Drums stay as they are", col) {
            play.transpose = t.round().clamp(-12.0, 12.0) as i8;
            changed = true;
        }
        let swing_shown = if play.swing > 0.0 { format!("{:.0}%", play.swing * 100.0) } else { "Own".into() };
        let word = if play.swing > 0.0 { "for everyone" } else { "each their own" };
        changed |= part_knob(ui, cell(3), Id::new(("band_swing", e)), &SWING, &mut play.swing, false, &swing_shown, word, "Swing: one swing for everyone in the band in this part (all the way down: each keeps its own)", col);
        // The choices: the key and the feel, then how the part ends.
        let x0 = cell(4).left() + 12.0;
        p.line_segment([pos2(x0 - 8.0, row.top() + 4.0), pos2(x0 - 8.0, row.bottom() - 4.0)], Stroke::new(1.0_f32, LINE_SOFT));
        let (cw, chh) = (118.0, 50.0);
        let at = |c: usize, r: usize| Rect::from_min_size(pos2(x0 + c as f32 * (cw + 8.0), row.top() + r as f32 * (chh + 8.0)), vec2(cw, chh));
        let tip = format!("The part's key: the song's, its parallel (C major becomes C minor) or its relative (C major becomes A minor: the same notes, the chords count from A). Now: {}", play.key(*scale).name());
        changed |= choice(ui, at(0, 0), Id::new(("band_mode", e)), "Key", MODES, &mut play.mode, &tip, col);
        changed |= choice(ui, at(1, 0), Id::new(("band_feel", e)), "Feel", FEELS, &mut play.feel, "Half Time: the followers and the drums play their steps twice as long; Double Time: half as long. The chord players keep the chords in time", col);
        let mut fill = play.fill as u8;
        if choice(ui, at(0, 1), Id::new(("band_fill", e)), "At its end", &["No fill", "Fill"], &mut fill, "Fill: the drummers (Drummer, Home Rhythms, Chance, Drum Map) play a fill in the part's last bar, with a crash on the next part's one. It comes where the part ends (its last time round), not between its repeats", col) {
            play.fill = fill == 1;
            changed = true;
        }
        let builds = ["No build", "Build 1 bar", "Build 2 bars"];
        changed |= choice(ui, at(1, 1), Id::new(("band_build", e)), "", &builds, &mut play.build, "Build: the part's last bar or two start softer and sparser and grow louder and fuller into the next part", col);
        let breaks = ["No break", "Break: last beat", "Break: last bar"];
        changed |= choice(ui, at(2, 1), Id::new(("band_break", e)), "", &breaks, &mut play.brk, "Break: everyone stops for the part's last beat or bar, and comes back on the next part's one", col);
        // In words, and back to as written.
        let words = Rect::from_min_size(pos2(at(2, 0).left(), row.top() + 16.0), vec2(cw + 40.0, chh - 16.0));
        text(&p, pos2(words.left() + 2.0, row.top() + 6.0), Align2::LEFT_CENTER, "In short", small(), TEXT_DIM);
        let galley = p.layout(play_words(&play), body(), TEXT, words.width());
        p.with_clip_rect(words.expand(2.0)).galley(words.min + vec2(2.0, 0.0), galley, TEXT);
        if !play.is_default() {
            let r = Rect::from_min_size(pos2(at(2, 0).right() + 48.0, row.top() + 14.0), vec2(70.0, 36.0));
            if pad(ui, r, Id::new(("band_play_reset", e)), "As written", PANEL3, "Every setting of this part back to as written (who plays in it stays)").clicked() {
                play = PartPlay::default();
                changed = true;
            }
        }
        if changed {
            self.project.band.parts[e].play = play;
        }
    }

    /// Takes part `i` out (and its steps in the form); the last one leaves one progression.
    fn delete_band_part(&mut self, i: usize) {
        let band = &mut self.project.band;
        if band.parts.len() <= 1 {
            *band = Default::default();
            return;
        }
        band.parts.remove(i);
        band.form.retain(|f| f.part as usize != i);
        for f in band.form.iter_mut() {
            if f.part as usize > i {
                f.part -= 1;
            }
        }
        self.band.editing = self.band.editing.min(band.parts.len() - 1);
    }

    /// The form: a block per step (its part and how many times), the one
    /// playing lit with how far through it is. The part: click for the
    /// next, right-click for the previous; the times: - and + (or click and
    /// right-click); x takes the step out, + adds one.
    fn band_form_row(&mut self, ui: &mut Ui, st: BandStatus, accent: Color32) {
        let n = self.project.band.parts.len();
        let steps = self.project.band.form.len();
        let (bw, bh) = (90.0, 66.0);
        let (row, _) = ui.allocate_exact_size(vec2((steps + 1) as f32 * (bw + 6.0), bh), Sense::hover());
        let mut remove = None;
        for i in 0..steps {
            let r = Rect::from_min_size(pos2(row.left() + i as f32 * (bw + 6.0), row.top()), vec2(bw, bh));
            let FormStep { part, times } = self.project.band.form[i];
            let part = (part as usize).min(n - 1);
            let playing = st.parts && st.step as usize == i;
            let col = part_color(part);
            let p = ui.painter().clone();
            let fill = if playing { col } else { shade(col, 0.45) };
            p.rect_filled(r.translate(vec2(0.0, 2.0)), 4.0, Color32::from_black_alpha(90));
            rect_fill(&p, r, 4.0, fill);
            if playing {
                rect_line(&p, r, 4.0, 2.0, accent);
            }
            let ink = text_on(fill);
            let top = Rect::from_min_size(r.min, vec2(bw, bh * 0.5));
            text(&p, pos2(top.left() + 8.0, top.center().y), Align2::LEFT_CENTER, LETTERS[part], crate::skin::Font::Block.id(18.0), ink);
            p.with_clip_rect(top.shrink(2.0)).text(pos2(top.left() + 24.0, top.center().y), Align2::LEFT_CENTER, &self.project.band.parts[part].name, small(), ink);
            let resp = ui.interact(top, Id::new(("band_form_part", i)), Sense::click()).on_hover_text("The part: click for the next, right-click for the previous");
            if resp.clicked() {
                self.project.band.form[i].part = ((part + 1) % n) as u8;
            }
            if resp.secondary_clicked() {
                self.project.band.form[i].part = ((part + n - 1) % n) as u8;
            }
            // How many times: - and + on either side.
            let bottom = Rect::from_min_max(pos2(r.left(), top.bottom()), pos2(r.right(), r.bottom() - 8.0));
            let label = if playing && times > 1 { format!("{}/{times}", st.pass as u32 + 1) } else { format!("x{times}") };
            text(&p, bottom.center(), Align2::CENTER_CENTER, label, FontId::proportional(15.0), ink);
            let less = Rect::from_min_size(bottom.min, vec2(24.0, bottom.height()));
            let more = Rect::from_min_size(pos2(bottom.right() - 24.0, bottom.top()), vec2(24.0, bottom.height()));
            let mid = Rect::from_min_max(pos2(less.right(), bottom.top()), pos2(more.left(), bottom.bottom()));
            let lresp = ui.interact(less, Id::new(("band_form_less", i)), Sense::click());
            let mresp = ui.interact(more, Id::new(("band_form_more", i)), Sense::click());
            text(&p, less.center(), Align2::CENTER_CENTER, "-", FontId::proportional(16.0), if lresp.hovered() { ink } else { shade(ink, 0.7) });
            text(&p, more.center(), Align2::CENTER_CENTER, "+", FontId::proportional(16.0), if mresp.hovered() { ink } else { shade(ink, 0.7) });
            let cresp = ui.interact(mid, Id::new(("band_form_times", i)), Sense::click()).on_hover_text("How many times it plays: click or + for more, right-click or - for fewer");
            if lresp.on_hover_text("Once fewer").clicked() || cresp.secondary_clicked() {
                self.project.band.form[i].times = times.saturating_sub(1).max(1);
            }
            if mresp.on_hover_text("Once more").clicked() || cresp.clicked() {
                self.project.band.form[i].times = (times % 16) + 1;
            }
            // How far through the part it is.
            let track = Rect::from_min_size(pos2(r.left() + 5.0, r.bottom() - 7.0), vec2(bw - 10.0, 3.0));
            if playing {
                p.rect_filled(track, 1.0, shade(ink, 0.35));
                p.rect_filled(Rect::from_min_size(track.min, vec2(track.width() * st.through as f32 / 255.0, 3.0)), 1.0, ink);
            }
            let x = Rect::from_min_size(pos2(r.right() - 16.0, r.top() + 2.0), vec2(14.0, 14.0));
            let xresp = ui.interact(x, Id::new(("band_form_x", i)), Sense::click());
            if xresp.hovered() {
                text(ui.painter(), x.center(), Align2::CENTER_CENTER, "x", small(), REC);
            }
            if xresp.on_hover_text("Take this step out").clicked() {
                remove = Some(i);
            }
        }
        if steps < MAX_FORM {
            let r = Rect::from_min_size(pos2(row.left() + steps as f32 * (bw + 6.0), row.top()), vec2(bw * 0.5, bh));
            if pad(ui, r, Id::new("band_form_add"), "+", PANEL2, "One more step: the part after the last one, once").clicked() {
                let next = self.project.band.form.last().map(|f| (f.part as usize + 1) % n).unwrap_or(0);
                self.project.band.form.push(FormStep { part: next as u8, times: 1 });
            }
        }
        if let Some(i) = remove {
            self.project.band.form.remove(i);
        }
        if self.project.band.form.is_empty() {
            ui.label(RichText::new("No form: the part picked plays round and round; Next part goes on to the next one.").small().color(TEXT_DIM));
        }
    }

    /// Who plays where: a row per member, a column per part; a lit square
    /// plays, a dark one sits the part out.
    fn band_matrix(&mut self, ui: &mut Ui, st: BandStatus) {
        let members = self.band_members();
        if members.is_empty() {
            ui.label(RichText::new("Nobody in the band yet: switch Band on in a chord player, a sequencer or a drummer.").small().color(TEXT_DIM));
            return;
        }
        let n = self.project.band.parts.len();
        let (name_w, cw, rh, head) = (250.0, 92.0, 30.0, 22.0);
        let (r, _) = ui.allocate_exact_size(vec2(name_w + n as f32 * (cw + 4.0), head + members.len() as f32 * (rh + 4.0)), Sense::hover());
        let p = ui.painter().clone();
        let col_rect = |i: usize, top: f32, h: f32| Rect::from_min_size(pos2(r.left() + name_w + i as f32 * (cw + 4.0), top), vec2(cw, h));
        for (i, part) in self.project.band.parts.iter().enumerate() {
            let hr = col_rect(i, r.top(), head - 4.0);
            let playing = st.parts && st.part as usize == i;
            if playing {
                rect_fill(&p, hr, 3.0, shade(part_color(i), 0.5));
            }
            p.with_clip_rect(hr).text(pos2(hr.left() + 6.0, hr.center().y), Align2::LEFT_CENTER, format!("{}  {}", LETTERS[i], part.name), small(), if playing { TEXT } else { part_color(i) });
        }
        let mut toggle = None;
        for (m, member) in members.iter().enumerate() {
            let top = r.top() + head + m as f32 * (rh + 4.0);
            let nr = Rect::from_min_size(pos2(r.left(), top), vec2(name_w - 8.0, rh));
            rect_fill(&p, nr, 3.0, PANEL2);
            rect_fill(&p, Rect::from_min_size(nr.min, vec2(5.0, rh)), 2.0, member.color);
            let clip = p.with_clip_rect(nr.shrink(1.0));
            clip.text(pos2(nr.left() + 12.0, nr.center().y), Align2::LEFT_CENTER, member.name, body(), TEXT);
            let w = p.layout_no_wrap(member.name.to_string(), body(), TEXT).size().x;
            clip.text(pos2(nr.left() + 18.0 + w, nr.center().y), Align2::LEFT_CENTER, format!("{} · {}", member.track, member.plays_as()), small(), TEXT_DIM);
            for (i, part) in self.project.band.parts.iter().enumerate() {
                let cr = col_rect(i, top, rh);
                let plays = !part.out.contains(&member.id);
                let resp = ui.interact(cr, Id::new(("band_seat", member.id, i)), Sense::click());
                let fill = if plays { shade(part_color(i), if resp.hovered() { 1.1 } else { 0.85 }) } else if resp.hovered() { PANEL3 } else { PANEL };
                rect_fill(&p, cr, 3.0, fill);
                if st.parts && st.part as usize == i {
                    rect_line(&p, cr, 3.0, 1.5, TEXT);
                }
                text(&p, cr.center(), Align2::CENTER_CENTER, if plays { "plays" } else { "sits out" }, small(), if plays { text_on(fill) } else { TEXT_DIM });
                let tip = if plays { format!("{} plays in {}. Click: it sits this part out (it stops cleanly, and is back for the next part it plays in)", member.name, part.name) } else { format!("{} sits {} out. Click: it plays in it again", member.name, part.name) };
                if resp.on_hover_text(tip).clicked() {
                    toggle = Some((i, member.id));
                }
            }
        }
        if let Some((i, id)) = toggle {
            let out = &mut self.project.band.parts[i].out;
            match out.iter().position(|x| *x == id) {
                Some(k) => {
                    out.remove(k);
                }
                None => out.push(id),
            }
        }
    }
}

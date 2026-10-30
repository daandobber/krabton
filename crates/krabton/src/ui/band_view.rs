//! The band in the editors: a Band section on every device that can be in
//! it (its switch; with it on, how it follows, the band's chord and part
//! now, and the way to the Band window), and the Band window: the song's
//! parts as cards (click one to edit its chords, its play button to go to
//! it), the part's chords, the form as a row of blocks with the one playing
//! lit, and the next part.

use crate::app::App;
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, Context, FontId, Id, Rect, RichText, Sense, Ui};
use krabton_engine::band::{self as kb, BandPart, BandStatus, Cue, FormStep, FOLLOWS, LETTERS, MAX_FORM, MAX_PARTS, NONE, PART_NAMES};
use krabton_engine::classic_seq::{chord_name, CHORD_LENGTHS, PROG_SLOTS, QUALITIES};
use krabton_engine::midi_fx::MidiFxKind;
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

/// What a device's Band section shows of the band.
pub(crate) struct BandLive {
    pub st: BandStatus,
    /// The parts' names (none: one progression) and how often the form plays the one now.
    pub names: Vec<String>,
    pub times: Option<u8>,
    pub scale: Scale,
}

impl App {
    pub(crate) fn band_live(&self) -> BandLive {
        let st = self.band_status();
        let times = self.project.band.form.get(st.step as usize).filter(|_| st.step != NONE).map(|f| f.times.max(1));
        BandLive { st, names: self.project.band.parts.iter().map(|p| p.name.clone()).collect(), times, scale: self.project.scale }
    }
}

/// Whether a kind can be in the band (a chord player, or one that follows it).
pub(crate) fn in_band_kind(kind: MidiFxKind) -> bool {
    band_param(kind).is_some() || kb::follow_param(kind).is_some()
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
    let Some(bi) = follows.or(band_param(kind)) else { return (false, false, None) };
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
    let tip = if follows.is_some() { "In the band: its notes follow the band's chords (off: it plays as written)" } else { "In the band: it plays the band's chords, the same as every chord player in it; change them here and the others follow" };
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
    // The part now: a click goes on to the next part.
    let pr = Rect::from_min_size(pos2(x, y), vec2(bw, 22.0));
    let cued = live.st.cued != NONE;
    let label = match live.names.get(live.st.cued as usize).filter(|_| cued) {
        Some(next) => format!("> {next}"),
        None => part_now(live),
    };
    let lit = cued && super::session::blink();
    let resp = button(ui, pr, id.with("part"), &label, lit, live.names.get(live.st.part as usize).map(|_| part_color(live.st.part as usize)).unwrap_or(accent));
    if live.names.is_empty() {
        resp.on_hover_text("The band plays one progression; give it parts (Verse, Chorus...) in the Band window");
    } else {
        let resp = resp.on_hover_text("The part playing. Click: the next part when this one ends; right-click: not after all");
        if resp.clicked() {
            cue = Some(Some(Cue::Next));
        }
        if resp.secondary_clicked() {
            cue = Some(None);
        }
    }
    y += 26.0;
    let wr = Rect::from_min_size(pos2(x, y), vec2(bw, 20.0));
    if button(ui, wr, id.with("window"), "Parts...", false, accent).on_hover_text("The Band window: the song's parts, their chords and the form").clicked() {
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

/// The chords of a part in short: "I V vi IV".
fn summary(scale: &Scale, part: &BandPart) -> String {
    part.chords.iter().take(part.count as usize).map(|c| chord_name(scale, c.0, c.1)).collect::<Vec<_>>().join(" ")
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

impl App {
    /// The Band window (from a device's Band section or the Studio's bar).
    pub(crate) fn ui_band_window(&mut self, ctx: &Context) {
        if !self.band.window {
            return;
        }
        let (open, _) = super::windows::tool_window(ctx, "band", "Band", vec2(780.0, 470.0), |ui| self.band_body(ui));
        if !open {
            self.band.window = false;
        }
    }

    /// Who is in the band: "Piano Player, Step (Root)...".
    fn band_members(&self) -> Vec<String> {
        let mut v = Vec::new();
        for tr in &self.project.tracks {
            for dev in &tr.midi_devices {
                if band_param(dev.kind).and_then(|b| dev.params.get(b)).is_some_and(|x| *x >= 0.5) {
                    v.push(format!("{} ({})", dev.kind.name(), tr.name));
                } else if let Some(m) = kb::follow_mode(dev.kind, &dev.params) {
                    v.push(format!("{} ({}, {})", dev.kind.name(), tr.name, FOLLOWS[m as usize]));
                }
            }
        }
        v
    }

    fn band_body(&mut self, ui: &mut Ui) {
        let st = self.band_status();
        let scale = self.project.scale;
        let accent = accent_now();
        let members = self.band_members();
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            if self.project.band.parts.is_empty() {
                ui.label(RichText::new("The band plays one progression: the one its chord players share.").color(TEXT));
                ui.label(RichText::new("Give it parts (a Verse, a Chorus, a Bridge...), each with chords of its own, and a form that plays them in turn: everyone in the band moves on together.").small().color(TEXT_DIM));
                ui.add_space(8.0);
                let (r, _) = ui.allocate_exact_size(vec2(240.0, 40.0), Sense::hover());
                if pad(ui, r, Id::new("band_make"), "Make parts: Verse and Chorus", accent, "The band's chords become the Verse; a Chorus to change; the form Verse twice, Chorus twice").clicked() {
                    self.make_band_parts();
                }
                ui.add_space(10.0);
                self.band_members_line(ui, &members);
                return;
            }
            self.band_now_row(ui, st, accent);
            ui.add_space(8.0);
            self.band_cards(ui, st, &scale, accent);
            ui.add_space(8.0);
            self.band_part_editor(ui, st, &scale, accent);
            ui.add_space(10.0);
            self.band_form_row(ui, st, accent);
            ui.add_space(10.0);
            self.band_members_line(ui, &members);
        });
    }

    fn band_members_line(&self, ui: &mut Ui, members: &[String]) {
        let text = if members.is_empty() { "Nobody in the band yet: switch Band on in a chord player or a sequencer.".to_string() } else { format!("In the band: {}", members.join(", ")) };
        ui.label(RichText::new(text).small().color(TEXT_DIM));
    }

    /// The parts from the band's progression: a Verse, a Chorus, the form.
    pub(crate) fn make_band_parts(&mut self) {
        let part = self.band_first_part().unwrap_or_else(|| BandPart::from_part("", &[2, 4, 1, 0, 5, 0, 6, 0, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0]).part());
        let verse = BandPart::from_part(PART_NAMES[0], &part);
        let mut chorus = BandPart::from_part(PART_NAMES[1], &part);
        chorus.count = 4;
        chorus.chords[..4].copy_from_slice(&[(4, 0), (5, 0), (1, 0), (6, 0)]);
        self.project.band.parts = vec![verse, chorus];
        self.project.band.form = vec![FormStep { part: 0, times: 2 }, FormStep { part: 1, times: 2 }];
        self.band.editing = 0;
    }

    /// What the band's chord players share now (the first one's).
    fn band_first_part(&self) -> Option<Vec<u8>> {
        self.project.tracks.iter().flat_map(|t| &t.midi_devices).find(|d| band_param(d.kind).and_then(|b| d.params.get(b)).is_some_and(|v| *v >= 0.5)).map(|d| krabton_engine::sequencers::players::band_part(&d.params, &d.data))
    }

    /// The part playing and its chord, the next part, and how a cue waits.
    fn band_now_row(&mut self, ui: &mut Ui, st: BandStatus, accent: Color32) {
        let (row, _) = ui.allocate_exact_size(vec2(ui.available_width().max(600.0), 46.0), Sense::hover());
        let p = ui.painter().clone();
        let parts = &self.project.band.parts;
        let now = (st.part as usize).min(parts.len() - 1);
        let nr = Rect::from_min_size(row.min, vec2(250.0, 46.0));
        rect_fill(&p, nr, 4.0, PANEL3);
        rect_fill(&p, Rect::from_min_size(nr.min, vec2(8.0, 46.0)), 2.0, part_color(now));
        let live = BandLive { st, names: parts.iter().map(|p| p.name.clone()).collect(), times: self.project.band.form.get(st.step as usize).filter(|_| st.step != NONE).map(|f| f.times.max(1)), scale: self.project.scale };
        text(&p, pos2(nr.left() + 16.0, nr.top() + 14.0), Align2::LEFT_CENTER, format!("{}  {}", LETTERS[now], part_now(&live)), FontId::proportional(15.0), TEXT);
        let off_form = if st.step == NONE && !self.project.band.form.is_empty() { "  (off the form)" } else { "" };
        text(&p, pos2(nr.left() + 16.0, nr.top() + 33.0), Align2::LEFT_CENTER, format!("{}{off_form}", summary(&live.scale, &parts[now])), small(), TEXT_DIM);
        if let Some((numeral, name)) = chord_now(&live) {
            text(&p, pos2(nr.right() - 10.0, nr.center().y), Align2::RIGHT_CENTER, format!("{numeral}  {name}"), FontId::proportional(18.0), accent);
        }
        // The next part: when this one ends (or on the bar).
        let cued = st.cued != NONE;
        let nx = Rect::from_min_size(pos2(nr.right() + 10.0, row.top()), vec2(170.0, 46.0));
        let label = match parts.get(st.cued as usize).filter(|_| cued) {
            Some(next) => format!("Next: {}", next.name),
            None => "Next part >".to_string(),
        };
        let fill = if cued && super::session::blink() { part_color(st.cued as usize) } else { PANEL3 };
        let resp = pad(ui, nx, Id::new("band_next"), &label, fill, "Everyone on to the next part when this one ends (or on the bar). Right-click: not after all");
        if resp.clicked() {
            self.cmd(krabton_engine::Command::BandCue(Some(Cue::Next)));
        }
        if resp.secondary_clicked() {
            self.cmd(krabton_engine::Command::BandCue(None));
        }
        let br = Rect::from_min_size(pos2(nx.right() + 10.0, row.top() + 8.0), vec2(96.0, 30.0));
        let on_bar = self.project.band.cue_on_bar;
        if button(ui, br, Id::new("band_on_bar"), if on_bar { "On the bar" } else { "At its end" }, on_bar, accent).on_hover_text("When a part picked starts: at the end of the part playing, or on the next bar").clicked() {
            self.project.band.cue_on_bar = !on_bar;
        }
        let xr = Rect::from_min_size(pos2(br.right() + 10.0, row.top() + 8.0), vec2(110.0, 30.0));
        if button(ui, xr, Id::new("band_no_parts"), "One progression", false, accent).on_hover_text("No parts: the band plays one progression again (the part playing). Ctrl+Z brings them back").clicked() {
            self.project.band = Default::default();
        }
    }

    /// The parts as cards: click one to edit it, its play button to go to it.
    fn band_cards(&mut self, ui: &mut Ui, st: BandStatus, scale: &Scale, accent: Color32) {
        let n = self.project.band.parts.len();
        let (cw, ch) = (118.0, 72.0);
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
            p.with_clip_rect(r.shrink(2.0)).text(pos2(r.left() + 8.0, r.top() + 38.0), Align2::LEFT_CENTER, summary(scale, part), small(), ink);
            text(&p, pos2(r.left() + 8.0, r.bottom() - 12.0), Align2::LEFT_CENTER, format!("{} x {}", part.count, CHORD_LENGTHS[part.length.min(3) as usize]), small(), if playing { ink } else { TEXT_DIM });
            if resp.on_hover_text(format!("{}: click to edit its chords", part.name)).clicked() {
                self.band.editing = i;
            }
            let go = Rect::from_min_size(pos2(r.right() - 30.0, r.bottom() - 26.0), vec2(26.0, 22.0));
            let gresp = ui.interact(go, Id::new(("band_go", i)), Sense::click());
            rect_fill(ui.painter(), go, 3.0, if gresp.hovered() { accent } else { shade(PANEL2, 0.9) });
            play_icon(ui.painter(), go.center(), 9.0, if gresp.hovered() { TEXT_DARK } else { TEXT });
            if gresp.on_hover_text("Everyone to this part when the one playing ends (or on the bar); stopped, the song starts with it").clicked() {
                self.cmd(krabton_engine::Command::BandCue(Some(Cue::Part(i as u8))));
            }
        }
        if n < MAX_PARTS {
            let r = Rect::from_min_size(pos2(row.left() + n as f32 * (cw + 8.0), row.top()), vec2(cw * 0.6, ch));
            if pad(ui, r, Id::new("band_add_part"), "+ Part", PANEL2, "A new part, from the chords of the one you edit").clicked() {
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

    /// The part being edited: its name, how long its chords last, how many,
    /// and the chords (click: the next, right-click: the previous).
    fn band_part_editor(&mut self, ui: &mut Ui, st: BandStatus, scale: &Scale, accent: Color32) {
        let n = self.project.band.parts.len();
        let e = self.band.editing.min(n - 1);
        self.band.editing = e;
        let mut delete = false;
        let mut duplicate = false;
        ui.horizontal(|ui| {
            ui.label(RichText::new(LETTERS[e]).strong().color(part_color(e)));
            let part = &mut self.project.band.parts[e];
            ui.add(egui::TextEdit::singleline(&mut part.name).desired_width(110.0)).on_hover_text("The part's name");
            let len = part.length.min(3) as usize;
            let resp = ui.add(egui::Button::new(RichText::new(format!("Each {}", CHORD_LENGTHS[len])).color(TEXT)).fill(PANEL3)).on_hover_text("How long each chord lasts: click for longer, right-click for shorter");
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
            rect_fill(&p, r, 4.0, if playing { shade(accent, 0.55) } else if used { PANEL3 } else { PANEL2 });
            if playing {
                rect_line(&p, r, 4.0, 1.5, accent);
            }
            text(&p, pos2(r.left() + 4.0, r.top() + 7.0), Align2::LEFT_CENTER, (slot + 1).to_string(), small(), TEXT_DIM);
            let top = Rect::from_min_size(r.min, vec2(cw, ch - 20.0));
            text(&p, top.center() + vec2(0.0, 2.0), Align2::CENTER_CENTER, chord_name(scale, degree, quality), FontId::proportional(18.0), if used { TEXT } else { TEXT_DIM });
            let resp = ui.interact(top, Id::new(("band_deg", e, slot)), Sense::click());
            let letter = if degree == 0 { "rest".to_string() } else { chord_letter_name(scale, degree, quality) };
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
    /// playing lit. The part: click for the next, right-click for the
    /// previous; the times the same; x takes the step out, + adds one.
    fn band_form_row(&mut self, ui: &mut Ui, st: BandStatus, accent: Color32) {
        ui.label(RichText::new("FORM").strong().color(TEXT_DIM)).on_hover_text("The parts in the order they play, each so many times, then round again");
        let n = self.project.band.parts.len();
        let steps = self.project.band.form.len();
        let (bw, bh) = (72.0, 58.0);
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
            let top = Rect::from_min_size(r.min, vec2(bw, bh * 0.6));
            text(&p, pos2(top.left() + 8.0, top.center().y), Align2::LEFT_CENTER, LETTERS[part], crate::skin::Font::Block.id(18.0), ink);
            p.with_clip_rect(top.shrink(2.0)).text(pos2(top.left() + 24.0, top.center().y), Align2::LEFT_CENTER, &self.project.band.parts[part].name, small(), ink);
            let resp = ui.interact(top, Id::new(("band_form_part", i)), Sense::click()).on_hover_text("The part: click for the next, right-click for the previous");
            if resp.clicked() {
                self.project.band.form[i].part = ((part + 1) % n) as u8;
            }
            if resp.secondary_clicked() {
                self.project.band.form[i].part = ((part + n - 1) % n) as u8;
            }
            let bottom = Rect::from_min_max(pos2(r.left(), top.bottom()), r.max);
            let label = if playing && times > 1 { format!("{}/{times}", st.pass as u32 + 1) } else { format!("x{times}") };
            text(&p, bottom.center(), Align2::CENTER_CENTER, label, FontId::proportional(14.0), ink);
            let resp = ui.interact(bottom, Id::new(("band_form_times", i)), Sense::click()).on_hover_text("How many times: click for more, right-click for fewer");
            if resp.clicked() {
                self.project.band.form[i].times = (times % 16) + 1;
            }
            if resp.secondary_clicked() {
                self.project.band.form[i].times = times.saturating_sub(1).max(1);
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
            let r = Rect::from_min_size(pos2(row.left() + steps as f32 * (bw + 6.0), row.top()), vec2(bw * 0.6, bh));
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
}

//! Airway's face, as CHIMERA's own lab draws a voice: neon lines on black,
//! panels in thin blue frames. Along the top how it plays (Vowels, Words,
//! Creature), the body and the voices. Then the vocal tract seen from the
//! side as the engine shapes it now (the walls, the tongue in green, the
//! lips in pink, the cross-sections in orange; drag in it to move the
//! tongue and the jaw), the tube glottis to lips, the vocal folds, the
//! spectrum with its pitch and resonances, the vowel chart (click a vowel
//! to sing it) and the voice's knobs. For Words the lyric and its
//! syllables; for a creature its calls.

use super::inst_screen::InstLive;
use super::modulation::ModView;
use egui::{pos2, vec2, Align2, Color32, FontId, Id, Pos2, Rect, RichText, Sense, Shape, Stroke, Ui};
use krabton_engine::dsp::airway::{self as aw, chimera, PlansRef};
use std::sync::{Arc, Mutex};

const BG: Color32 = Color32::from_rgb(0x02, 0x05, 0x0b);
const FRAME: Color32 = Color32::from_rgb(0x1b, 0x3a, 0x5c);
const CYAN: Color32 = Color32::from_rgb(0x4f, 0xd8, 0xff);
const DEEP: Color32 = Color32::from_rgb(0x1f, 0x6f, 0xc8);
const LABEL: Color32 = Color32::from_rgb(0x8f, 0xb8, 0xd8);
const ORANGE: Color32 = Color32::from_rgb(0xff, 0xb3, 0x40);
const GREEN: Color32 = Color32::from_rgb(0x55, 0xff, 0x99);
const PINK: Color32 = Color32::from_rgb(0xff, 0x5f, 0xd0);
const RED: Color32 = Color32::from_rgb(0xff, 0x6a, 0x4f);
const WHITE: Color32 = Color32::from_rgb(0xe8, 0xf4, 0xff);

/// The dictionary's download, when one is going.
type Fetch = Arc<Mutex<Option<Result<(), String>>>>;

fn small() -> FontId {
    FontId::monospace(10.0)
}

/// A neon line: a soft glow under a sharp core.
fn neon(p: &egui::Painter, pts: Vec<Pos2>, c: Color32, w: f32) {
    if pts.len() < 2 {
        return;
    }
    p.add(Shape::line(pts.clone(), Stroke::new(w * 4.0, c.gamma_multiply(0.08))));
    p.add(Shape::line(pts.clone(), Stroke::new(w * 2.2, c.gamma_multiply(0.22))));
    p.add(Shape::line(pts, Stroke::new(w, c)));
}

/// A panel: its frame and its title (and a word after it), and the room inside.
fn panel_frame(p: &egui::Painter, r: Rect, title: &str, sub: &str) -> Rect {
    p.rect_filled(r, 2.0, BG);
    p.rect_stroke(r, 2.0, Stroke::new(1.0_f32, FRAME), egui::StrokeKind::Inside);
    let t = p.text(r.left_top() + vec2(8.0, 8.0), Align2::LEFT_TOP, title, FontId::monospace(11.0), WHITE);
    p.text(pos2(t.right() + 10.0, t.center().y), Align2::LEFT_CENTER, sub, small(), LABEL);
    Rect::from_min_max(pos2(r.left() + 6.0, r.top() + 24.0), pos2(r.right() - 6.0, r.bottom() - 6.0))
}

/// A tab along the top: framed when it is the one.
fn tab(ui: &mut Ui, on: bool, name: &str, tip: &str) -> bool {
    let galley = ui.painter().layout_no_wrap(name.to_string(), FontId::monospace(12.0), WHITE);
    let (r, resp) = ui.allocate_exact_size(vec2(galley.size().x + 18.0, 22.0), Sense::click());
    let p = ui.painter();
    if on {
        p.rect_filled(r, 2.0, Color32::from_rgb(0x08, 0x1c, 0x30));
        p.rect_stroke(r, 2.0, Stroke::new(1.0_f32, CYAN), egui::StrokeKind::Inside);
    } else if resp.hovered() {
        p.rect_stroke(r, 2.0, Stroke::new(1.0_f32, FRAME), egui::StrokeKind::Inside);
    }
    p.text(r.center(), Align2::CENTER_CENTER, name, FontId::monospace(12.0), if on { WHITE } else { LABEL });
    resp.on_hover_text(tip).clicked()
}

/// A neon knob as the lab has them: a ring, a needle, the value large under it in its colour.
#[allow(clippy::too_many_arguments)]
fn knob(ui: &mut Ui, id: Id, at: Pos2, radius: f32, v: &mut f32, spec: &krabton_engine::params::ParamSpec, label: &str, shown: String, col: Color32) -> bool {
    let r = Rect::from_center_size(at, vec2(radius * 2.0 + 8.0, radius * 2.0 + 8.0));
    let resp = ui.interact(r, id, Sense::click_and_drag());
    let mut changed = false;
    let span = spec.max - spec.min;
    if resp.dragged() {
        let d = -resp.drag_delta().y + resp.drag_delta().x * 0.5;
        let fine = if ui.input(|i| i.modifiers.shift) { 0.2 } else { 1.0 };
        *v = (*v + d * span / 220.0 * fine).clamp(spec.min, spec.max);
        if spec.kind == krabton_engine::params::ParamKind::Int {
            *v = v.round();
        }
        changed = true;
    }
    if resp.double_clicked() {
        *v = spec.default;
        changed = true;
    }
    let p = ui.painter();
    let norm = ((*v - spec.min) / span.max(1e-6)).clamp(0.0, 1.0);
    let a0 = std::f32::consts::PI * 0.75;
    let sweep = std::f32::consts::PI * 1.5;
    let arc = |t0: f32, t1: f32, rad: f32| -> Vec<Pos2> {
        (0..=32).map(|k| a0 + sweep * (t0 + (t1 - t0) * k as f32 / 32.0)).map(|a| at + vec2(a.cos(), a.sin()) * rad).collect()
    };
    p.circle_filled(at, radius, Color32::from_rgb(0x04, 0x0a, 0x14));
    p.add(Shape::line(arc(0.0, 1.0, radius), Stroke::new(2.0_f32, col.gamma_multiply(0.18))));
    // Ticks round it.
    for k in 0..=10 {
        let a = a0 + sweep * k as f32 / 10.0;
        let d = vec2(a.cos(), a.sin());
        p.line_segment([at + d * (radius + 3.0), at + d * (radius + 6.0)], Stroke::new(1.0_f32, col.gamma_multiply(0.45)));
    }
    neon(p, arc(0.0, norm.max(0.002), radius), col, 2.0);
    let a = a0 + sweep * norm;
    neon(p, vec![at + vec2(a.cos(), a.sin()) * (radius * 0.25), at + vec2(a.cos(), a.sin()) * (radius * 0.8)], WHITE, 1.6);
    p.text(pos2(at.x, at.y - radius - if radius > 10.0 { 11.0 } else { 8.0 }), Align2::CENTER_CENTER, label, FontId::monospace(if radius > 10.0 { 10.0 } else { 8.5 }), col);
    let big = if radius > 17.0 { 16.0 } else if radius > 10.0 { 11.0 } else { 9.0 };
    p.text(pos2(at.x, at.y + radius + big * 0.75), Align2::CENTER_CENTER, shown, FontId::monospace(big), col);
    if resp.hovered() || resp.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
    }
    resp.on_hover_text(format!("{}: drag up or down (Shift: finer), double-click: back to where it starts", spec.name));
    changed
}

/// The vowels where they sit on the chart (an adult voice's first two
/// resonances, Hz), by the instrument's own vowel order.
const VOWEL_F: [(f32, f32); 12] = [(730.0, 1090.0), (660.0, 1720.0), (530.0, 1840.0), (400.0, 2100.0), (270.0, 2290.0), (390.0, 1990.0), (570.0, 840.0), (450.0, 830.0), (440.0, 1020.0), (300.0, 870.0), (640.0, 1190.0), (500.0, 1500.0)];

/// The spectrum of `wave` (dB, at `n` frequencies from 60 Hz to 6 kHz, logarithmically).
fn spectrum(wave: &[f32], sr: f32, n: usize) -> Vec<(f32, f32)> {
    let len = 2048.min(wave.len());
    if len < 256 {
        return Vec::new();
    }
    let w = &wave[wave.len() - len..];
    (0..n)
        .map(|k| {
            let f = 60.0 * (6000.0f32 / 60.0).powf(k as f32 / (n - 1) as f32);
            let step = std::f32::consts::TAU * f / sr;
            let (mut re, mut im) = (0.0f32, 0.0f32);
            for (i, x) in w.iter().enumerate().step_by(2) {
                let hann = 0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / len as f32).cos();
                let a = step * i as f32;
                re += x * hann * a.cos();
                im += x * hann * a.sin();
            }
            let mag = (re * re + im * im).sqrt() / (len as f32 * 0.25);
            (f, 20.0 * mag.max(1e-6).log10())
        })
        .collect()
}

/// The three strongest resonances above the pitch (their frequencies, low to high).
fn formants(spec: &[(f32, f32)], f0: f32) -> Vec<f32> {
    let mut peaks: Vec<(f32, f32)> = spec.windows(3).filter(|w| w[1].1 > w[0].1 && w[1].1 > w[2].1 && w[1].0 > (f0 * 1.6).max(220.0)).map(|w| w[1]).collect();
    peaks.sort_by(|a, b| b.1.total_cmp(&a.1));
    let mut f: Vec<f32> = peaks.into_iter().take(3).map(|p| p.0).collect();
    f.sort_by(|a, b| a.total_cmp(b));
    f
}

thread_local! {
    /// The anatomy of the body drawn last (made once a body).
    static TRACT: std::cell::RefCell<Option<(u8, chimera::anatomy::human::HumanVocalTract)>> = const { std::cell::RefCell::new(None) };
}

/// The vocal tract seen from the side, as the articulators `artic` shape it, fitted into `r`.
fn tract(p: &egui::Painter, r: Rect, body: u8, artic: &[f32]) {
    use chimera::anatomy::human::{BodyMix, HumanArtic, HumanVocalTract};
    let a: Vec<f64> = artic.iter().map(|v| *v as f64).collect();
    let frame = TRACT.with(|t| {
        let mut t = t.borrow_mut();
        if t.as_ref().is_none_or(|(b, _)| *b != body) {
            let mix = match body {
                1 => BodyMix::FEMALE,
                2 => BodyMix::CHILD,
                _ => BodyMix::MALE,
            };
            *t = Some((body, HumanVocalTract::new(mix.body())));
        }
        let art = if a.iter().all(|v| *v == 0.0) { chimera::anatomy::human::NEUTRAL } else { HumanArtic::from_array(&a) };
        t.as_mut().map(|(_, tr)| tr.compute_new(&art))
    });
    let Some(f) = frame else { return };
    // Its extent, to fit it (x forward, y up).
    let all = f.outer.iter().chain(f.inner.iter()).chain(f.nasal_cavity.iter()).chain(f.upper_lip.iter()).chain(f.lower_lip.iter());
    let (mut x0, mut x1, mut y0, mut y1) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
    for q in all {
        x0 = x0.min(q.x);
        x1 = x1.max(q.x);
        y0 = y0.min(q.y);
        y1 = y1.max(q.y);
    }
    if x1 <= x0 || y1 <= y0 {
        return;
    }
    let s = ((r.width() as f64 - 16.0) / (x1 - x0)).min((r.height() as f64 - 16.0) / (y1 - y0));
    let cx = r.center().x as f64 - (x0 + x1) * 0.5 * s;
    let cy = r.center().y as f64 + (y0 + y1) * 0.5 * s;
    let to = |q: &chimera::anatomy::geom::Vec2| pos2((cx + q.x * s) as f32, (cy - q.y * s) as f32);
    let line = |pts: &[chimera::anatomy::geom::Vec2]| pts.iter().map(to).collect::<Vec<_>>();
    // The cross-sections first, orange, under the walls.
    for g in f.grid.iter().step_by(1) {
        p.line_segment([to(&g.inner), to(&g.outer)], Stroke::new(1.0_f32, ORANGE.gamma_multiply(0.55)));
    }
    neon(p, line(&f.nasal_cavity), DEEP, 1.2);
    neon(p, line(&f.outer), CYAN, 1.4);
    neon(p, line(&f.inner), CYAN, 1.4);
    neon(p, line(&f.velum), WHITE, 1.2);
    neon(p, line(&f.epiglottis), CYAN, 1.0);
    neon(p, line(&f.jaw_bone), DEEP, 1.0);
    neon(p, line(&f.upper_teeth), WHITE, 1.2);
    neon(p, line(&f.lower_teeth), WHITE, 1.2);
    // The tongue, in green, with spokes from its centre as the lab draws its muscle.
    let tongue = line(&f.tongue);
    let c = to(&f.tongue_center);
    for q in tongue.iter().step_by(3) {
        p.line_segment([c, *q], Stroke::new(1.0_f32, GREEN.gamma_multiply(0.35)));
    }
    neon(p, tongue, GREEN, 1.6);
    neon(p, line(&f.upper_lip), PINK, 1.6);
    neon(p, line(&f.lower_lip), PINK, 1.6);
    // The handles: the tongue's body, the jaw, the lips.
    for (q, col) in [(to(&f.tongue_center), GREEN), (to(&f.tip), RED)] {
        p.circle_stroke(q, 7.0, Stroke::new(1.2_f32, col));
        p.line_segment([q - vec2(10.0, 0.0), q + vec2(10.0, 0.0)], Stroke::new(1.0_f32, col));
        p.line_segment([q - vec2(0.0, 10.0), q + vec2(0.0, 10.0)], Stroke::new(1.0_f32, col));
    }
    let g = to(&f.glottis);
    p.circle_filled(g, 3.0, WHITE);
}

/// Airway's panel. Returns a message for the player.
#[allow(clippy::too_many_arguments)]
pub(crate) fn panel(ui: &mut Ui, id: Id, preset: &mut String, params: &mut Vec<f32>, text: &mut String, plans: &mut PlansRef, live: &InstLive, height: f32, _mv: Option<&mut ModView>) -> Option<String> {
    if params.len() < aw::SPECS.len() {
        return None;
    }
    let mut said = None;
    let body_was = params[aw::BODY].round() as u8;
    let mode = (params[aw::MODE].round().max(0.0) as usize).min(2);
    let words_of = |plans: &PlansRef| plans.words.iter().map(|w| w.text.clone()).collect::<Vec<_>>();
    // Along the top.
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        egui::ComboBox::from_id_salt(id.with("preset")).width(120.0).selected_text(RichText::new(preset.as_str()).monospace()).show_ui(ui, |ui| {
            for name in aw::preset_names() {
                if ui.selectable_label(preset == name, *name).clicked() {
                    *preset = name.to_string();
                    *params = aw::preset(name);
                }
            }
        });
        ui.add_space(8.0);
        let tips = ["Each key sings a vowel at its pitch, the mouth set by the knobs", "Each key sings the next syllable of the lyric", "Each key a creature's call at its pitch"];
        for (k, name) in ["VOWELS", "WORDS", "CREATURE"].iter().enumerate() {
            if tab(ui, mode == k, name, tips[k]) {
                params[aw::MODE] = k as f32;
            }
        }
        ui.add_space(12.0);
        for (k, name) in ["MALE", "FEMALE", "CHILD"].iter().enumerate() {
            if tab(ui, params[aw::BODY].round() as usize == k, name, "The body: its size, its airway, its pitch (a change builds a new airway)") {
                params[aw::BODY] = k as f32;
            }
        }
        ui.add_space(12.0);
        ui.label(RichText::new("VOICES").monospace().color(LABEL));
        for (k, name) in aw::VOICE_COUNTS.iter().enumerate() {
            if tab(ui, params[aw::VOICES].round() as usize == k, name, "How many keys sing at once: each voice is a whole simulated airway and costs much (start with one)") {
                params[aw::VOICES] = k as f32;
            }
        }
        ui.add_space(12.0);
        let load = live.vals[aw::VIS_OUT];
        ui.label(RichText::new(format!("OUT {:>5.1} dB", 20.0 * load.max(1e-5).log10())).monospace().color(LABEL));
    });
    // Words: the lyric, its syllables, the dictionary.
    if mode == 1 {
        ui.horizontal(|ui| {
            ui.label(RichText::new("LYRIC").monospace().color(LABEL));
            let edit = ui.add(egui::TextEdit::singleline(text).desired_width(360.0).font(FontId::monospace(12.0)).hint_text("words to sing, a syllable a key (ti-ger splits by hand, - holds a vowel on)"));
            if edit.lost_focus() || (edit.changed() && !edit.has_focus()) {
                *plans = aw::prepare_with(text, params[aw::BODY].round() as u8, &words_of(plans));
            }
            let key = id.with("dict_fetch");
            let fetch: Option<Fetch> = ui.data(|d| d.get_temp(key));
            match fetch.as_ref().and_then(|f| f.lock().ok().and_then(|g| g.clone())) {
                Some(Ok(())) => {
                    ui.data_mut(|d| d.remove::<Fetch>(key));
                    aw::reload_dictionary();
                    *plans = aw::prepare_with(text, params[aw::BODY].round() as u8, &words_of(plans));
                    said = Some("The pronouncing dictionary is in: the lyric is sung as English is spoken".to_string());
                }
                Some(Err(e)) => {
                    ui.data_mut(|d| d.remove::<Fetch>(key));
                    said = Some(format!("Could not get the dictionary: {e}"));
                }
                None if fetch.is_some() => {
                    ui.spinner();
                    ui.ctx().request_repaint_after(std::time::Duration::from_millis(200));
                }
                None => {
                    if !aw::has_dictionary() && ui.button(RichText::new("GET DICTIONARY").monospace()).on_hover_text("The CMU pronouncing dictionary (3.6 MB, BSD licence) into Documents/Krabton/Library/CMUdict: words sung as English speaks them, not guessed from their spelling").clicked() {
                        let f: Fetch = Arc::new(Mutex::new(None));
                        let g = f.clone();
                        std::thread::spawn(move || {
                            let to = aw::dict_path();
                            let r = to.parent().map_or(Ok(()), std::fs::create_dir_all).map_err(|e| e.to_string()).and_then(|_| crate::piano_lib::fetch(aw::DICT_URL, &to));
                            if let Ok(mut x) = g.lock() {
                                *x = Some(r);
                            }
                        });
                        ui.data_mut(|d| d.insert_temp(key, f));
                    }
                }
            }
            let sung = live.vals[4].round() as usize;
            let singing = live.vals[5..9].iter().any(|v| *v >= 0.0);
            ui.add_space(8.0);
            ui.spacing_mut().item_spacing.x = 6.0;
            for (k, s) in plans.syls.iter().enumerate().take(48) {
                let on = k == sung && singing;
                ui.label(RichText::new(&s.text).monospace().color(if on { GREEN } else { LABEL }).background_color(if on { Color32::from_rgb(0x08, 0x2a, 0x18) } else { Color32::TRANSPARENT }));
            }
        });
    }
    let accent_line = ui.min_rect().bottom();
    let _ = accent_line;
    let h = (height - if mode == 1 { 52.0 } else { 28.0 }).max(180.0);
    let (rect, _) = ui.allocate_exact_size(vec2((ui.available_width() - 14.0).max(700.0), h), Sense::hover());
    let total = rect.width();
    // With little room the tube and the folds give theirs to the knobs.
    let wide = total >= 1080.0;
    let voice_w = if wide { (total * 0.27).max(320.0) } else { (total * 0.36).max(250.0) };
    let p = ui.painter_at(rect);
    let vals = &live.vals;
    let artic = &vals[aw::VIS_ARTIC..aw::VIS_ARTIC + 17];
    let f0 = vals[aw::VIS_F0];
    let gap = 6.0;

    // The vocal tract (or the creature).
    let tw = (h * 1.05).min(total * if wide { 0.25 } else { 0.3 });
    let tr = Rect::from_min_size(rect.min, vec2(tw, h));
    if mode == 2 {
        let inner = panel_frame(&p, tr, "BESTIARY", aw::CREATURES[(params[aw::CREATURE].round().max(0.0) as usize).min(3)]);
        let mut y = inner.top() + 4.0;
        for (k, name) in aw::CREATURES.iter().enumerate() {
            let r = Rect::from_min_size(pos2(inner.left(), y), vec2(inner.width() * 0.45, 24.0));
            let on = params[aw::CREATURE].round() as usize == k;
            let resp = ui.interact(r, id.with(("creature", k)), Sense::click());
            p.rect_stroke(r, 2.0, Stroke::new(1.0_f32, if on { CYAN } else { FRAME }), egui::StrokeKind::Inside);
            p.text(r.left_center() + vec2(8.0, 0.0), Align2::LEFT_CENTER, name.to_uppercase(), FontId::monospace(12.0), if on { WHITE } else { LABEL });
            if resp.clicked() {
                params[aw::CREATURE] = k as f32;
            }
            y += 28.0;
        }
        let c = (params[aw::CREATURE].round().max(0.0) as usize).min(3);
        if let Some(def) = aw::creature_def(c) {
            let mut y = inner.top() + 4.0;
            let x = inner.left() + inner.width() * 0.5;
            p.text(pos2(x, y + 6.0), Align2::LEFT_CENTER, "CALLS", small(), LABEL);
            y += 16.0;
            for (k, call) in def.calls.iter().enumerate().take(8) {
                let r = Rect::from_min_size(pos2(x, y), vec2(inner.right() - x, 22.0));
                let on = params[aw::CALL].round() as usize == k + 1;
                let resp = ui.interact(r, id.with(("call", k)), Sense::click());
                p.rect_stroke(r, 2.0, Stroke::new(1.0_f32, if on { PINK } else { FRAME }), egui::StrokeKind::Inside);
                p.text(r.left_center() + vec2(6.0, 0.0), Align2::LEFT_CENTER, call.name.to_uppercase(), FontId::monospace(11.0), if on { WHITE } else { LABEL });
                if resp.on_hover_text(call.hint).clicked() {
                    params[aw::CALL] = (k + 1) as f32;
                }
                y += 26.0;
            }
            p.text(pos2(inner.left(), inner.bottom() - 8.0), Align2::LEFT_CENTER, def.blurb, small(), LABEL);
        }
    } else {
        let body = params[aw::BODY].round() as u8;
        let inner = panel_frame(&p, tr, "VOCAL TRACT", ["ADULT MALE", "ADULT FEMALE", "CHILD"][body.min(2) as usize]);
        tract(&p, Rect::from_min_max(inner.min, pos2(inner.right(), inner.bottom() - 12.0)), body, artic);
        // Its numbers, along the bottom.
        let mut x = inner.left();
        for (name, v, unit) in [("F0", f0, "HZ"), ("LUNGS", vals[aw::VIS_LUNGS], "CMH2O"), ("TRACT", vals[aw::VIS_LENGTH], "CM")] {
            let r = p.text(pos2(x, inner.bottom()), Align2::LEFT_BOTTOM, format!("{name} {v:.1} {unit}"), small(), LABEL);
            x = r.right() + 10.0;
        }
        // Drag in it: the tongue forward and back, the jaw open and shut.
        let resp = ui.interact(inner, id.with("tract_drag"), Sense::drag());
        if resp.dragged() {
            let d = resp.drag_delta();
            params[aw::TONGUE] = (params[aw::TONGUE] + d.x * 0.01).clamp(-1.0, 1.0);
            params[aw::JAW] = (params[aw::JAW] + d.y * 0.004).clamp(-0.4, 0.4);
        }
        resp.on_hover_text("Drag: the tongue forward and back, the jaw open and shut");
    }

    // The tube and the vocal folds.
    let cw = if wide { ((total - tw - voice_w - 3.0 * gap) * 0.5).max(180.0) } else { 0.0 };
    let x1 = tr.right() + gap;
    if wide {
    let tube = Rect::from_min_size(pos2(x1, rect.top()), vec2(cw, h * 0.45));
    let inner = panel_frame(&p, tube, "TUBE", "GLOTTIS TO LIPS");
    let areas = &vals[aw::VIS_AREAS..aw::VIS_AREAS + aw::N_AREAS];
    let amax = areas.iter().fold(1.0f32, |m, a| m.max(*a));
    let mid = inner.center().y;
    let half = inner.height() * 0.4;
    let pts: Vec<(f32, f32)> = (0..aw::N_AREAS).map(|k| (inner.left() + 4.0 + (inner.width() - 8.0) * k as f32 / (aw::N_AREAS - 1) as f32, (areas[k] / amax).sqrt() * half)).collect();
    // The tube as stripes, as the lab draws it.
    for w in pts.windows(2) {
        for s in 0..6 {
            let t = s as f32 / 6.0;
            let x = w[0].0 + (w[1].0 - w[0].0) * t;
            let r = w[0].1 + (w[1].1 - w[0].1) * t;
            p.line_segment([pos2(x, mid - r), pos2(x, mid + r)], Stroke::new(1.0_f32, WHITE.gamma_multiply(0.35)));
        }
    }
    neon(&p, pts.iter().map(|(x, r)| pos2(*x, mid - r)).collect(), WHITE, 1.2);
    neon(&p, pts.iter().map(|(x, r)| pos2(*x, mid + r)).collect(), WHITE, 1.2);
    let vx = inner.left() + inner.width() * vals[aw::VIS_VELUM_AT].clamp(0.0, 1.0);
    let nose = vals[aw::VIS_NOSE];
    p.text(pos2(vx, inner.top() + 4.0), Align2::CENTER_TOP, if nose > 0.05 { "VELUM OPEN" } else { "VELUM SHUT" }, small(), LABEL);
    for (x, name) in [(inner.left(), "GLOTTIS"), (vx, "VELUM"), (inner.right(), "TEETH")] {
        let anchor = if x == inner.left() { Align2::LEFT_BOTTOM } else if x == inner.right() { Align2::RIGHT_BOTTOM } else { Align2::CENTER_BOTTOM };
        p.text(pos2(x, inner.bottom()), anchor, name, small(), LABEL);
    }

    let folds = Rect::from_min_max(pos2(x1, tube.bottom() + gap), pos2(x1 + cw, rect.bottom()));
    let inner = panel_frame(&p, folds, "VOCAL FOLDS", "SLOWED DOWN");
    let g = &vals[aw::VIS_GLOTTIS..aw::VIS_GLOTTIS + aw::N_GLOTTIS];
    let gmax = g.iter().fold(1e-6f32, |m, a| m.max(*a));
    let rad = (inner.height() * 0.42).min(inner.width() * 0.2);
    let c = pos2(inner.left() + rad + 6.0, inner.center().y);
    p.circle_stroke(c, rad, Stroke::new(1.2_f32, DEEP));
    // The folds, opening as the glottis does now (two cycles, slowed).
    let t = ui.input(|i| i.time) as f32;
    let phase = ((t * 1.3).fract() * aw::N_GLOTTIS as f32) as usize % aw::N_GLOTTIS;
    let open = (g[phase] / gmax).clamp(0.0, 1.0) * vals[aw::VIS_VOICING].clamp(0.0, 1.0);
    for side in [-1.0f32, 1.0] {
        let pts: Vec<Pos2> = (0..=16).map(|k| k as f32 / 16.0).map(|u| pos2(c.x + side * open * rad * 0.35 * (std::f32::consts::PI * u).sin(), c.y - rad * 0.75 + u * rad * 1.5)).collect();
        neon(&p, pts, WHITE, 1.4);
    }
    let wx = c.x + rad + 14.0;
    let ww = inner.right() - wx;
    let wave_pts: Vec<Pos2> = (0..aw::N_GLOTTIS).map(|k| pos2(wx + ww * k as f32 / (aw::N_GLOTTIS - 1) as f32, inner.center().y - 4.0 - (g[k] / gmax) * inner.height() * 0.35)).collect();
    neon(&p, wave_pts, ORANGE, 1.4);
    p.text(pos2(wx, inner.bottom()), Align2::LEFT_BOTTOM, format!("OPEN {:.0}%", vals[aw::VIS_VOICING].clamp(0.0, 1.0) * 100.0), small(), LABEL);
    p.text(pos2(inner.right(), inner.bottom()), Align2::RIGHT_BOTTOM, format!("{:.0} HZ", f0), small(), LABEL);
    }

    // The spectrum and the vowels.
    let x2 = if wide { x1 + cw + gap } else { x1 };
    let sw = if wide { cw } else { total - tw - voice_w - 2.0 * gap };
    let sp = Rect::from_min_size(pos2(x2, rect.top()), vec2(sw, h * 0.5));
    let inner = panel_frame(&p, sp, "SPECTRUM", "RESONANCES");
    let spec = spectrum(&live.wave, 48000.0, 120);
    let fx = |f: f32| inner.left() + inner.width() * ((f / 60.0).ln() / (6000.0f32 / 60.0).ln()).clamp(0.0, 1.0);
    let fy = |db: f32| inner.bottom() - 10.0 - inner.height() * ((db + 90.0) / 80.0).clamp(0.0, 1.0);
    for f in [100.0, 200.0, 500.0, 1000.0, 2000.0, 5000.0] {
        let x = fx(f);
        p.line_segment([pos2(x, inner.top()), pos2(x, inner.bottom() - 10.0)], Stroke::new(1.0_f32, FRAME.gamma_multiply(0.6)));
        p.text(pos2(x, inner.bottom()), Align2::CENTER_BOTTOM, if f >= 1000.0 { format!("{}K", f / 1000.0) } else { format!("{f}") }, small(), LABEL);
    }
    let mut found = Vec::new();
    if !spec.is_empty() {
        for (f, db) in &spec {
            p.line_segment([pos2(fx(*f), inner.bottom() - 10.0), pos2(fx(*f), fy(*db))], Stroke::new(1.0_f32, ORANGE.gamma_multiply(0.25)));
        }
        neon(&p, spec.iter().map(|(f, db)| pos2(fx(*f), fy(*db))).collect(), ORANGE, 1.4);
        found = formants(&spec, f0);
        for (k, f) in found.iter().enumerate() {
            let x = fx(*f);
            p.text(pos2(x, inner.top() + 2.0), Align2::CENTER_TOP, format!("F{}", k + 1), small(), ORANGE);
        }
    }
    if f0 > 40.0 {
        let x = fx(f0);
        p.line_segment([pos2(x, inner.top() + 12.0), pos2(x, inner.bottom() - 10.0)], Stroke::new(1.0_f32, GREEN.gamma_multiply(0.7)));
        p.text(pos2(x + 3.0, inner.bottom() - 14.0), Align2::LEFT_BOTTOM, format!("F0 {:.0}", f0), small(), GREEN);
    }

    let vw = Rect::from_min_max(pos2(x2, sp.bottom() + gap), pos2(x2 + sw, rect.bottom()));
    let inner = panel_frame(&p, vw, "VOWELS", "F1 / F2");
    // F2 across (front on the left), F1 down.
    let vx = |f2: f32| inner.left() + inner.width() * (1.0 - ((f2 - 700.0) / 1800.0).clamp(0.0, 1.0));
    let vy = |f1: f32| inner.top() + inner.height() * ((f1 - 230.0) / 560.0).clamp(0.0, 1.0);
    let quad = vec![pos2(vx(2300.0), vy(260.0)), pos2(vx(800.0), vy(280.0)), pos2(vx(1000.0), vy(760.0)), pos2(vx(1750.0), vy(700.0)), pos2(vx(2300.0), vy(260.0))];
    neon(&p, quad, DEEP, 1.0);
    let now = (params[aw::VOWEL].round().max(0.0) as usize).min(11);
    for (k, (f1, f2)) in VOWEL_F.iter().enumerate() {
        let at = pos2(vx(*f2), vy(*f1));
        let r = Rect::from_center_size(at, vec2(22.0, 18.0));
        let resp = ui.interact(r, id.with(("vowel", k)), Sense::click());
        let on = k == now && mode == 0;
        p.text(at, Align2::CENTER_CENTER, aw::VOWEL_IPA[k], FontId::proportional(if on { 17.0 } else { 14.0 }), if on { GREEN } else if resp.hovered() { WHITE } else { LABEL });
        if resp.on_hover_text(format!("Sing \"{}\"", aw::VOWEL_NAMES[k])).clicked() {
            params[aw::VOWEL] = k as f32;
            params[aw::MORPH] = 0.0;
            params[aw::MODE] = 0.0;
        }
    }
    // Where the voice is now, from what it sounds like.
    if found.len() >= 2 && vals[aw::VIS_VOICING] > 0.2 {
        let at = pos2(vx(found[1]), vy(found[0]));
        p.circle_stroke(at, 7.0, Stroke::new(1.4_f32, GREEN));
        p.line_segment([at - vec2(11.0, 0.0), at + vec2(11.0, 0.0)], Stroke::new(1.0_f32, GREEN));
        p.line_segment([at - vec2(0.0, 11.0), at + vec2(0.0, 11.0)], Stroke::new(1.0_f32, GREEN));
    }

    // The voice: the knobs.
    let x3 = x2 + sw + gap;
    let vr = Rect::from_min_max(pos2(x3, rect.top()), pos2(rect.right(), rect.bottom()));
    let inner = panel_frame(&p, vr, "VOICE", "SOURCE · MOUTH");
    let fmt = |k: usize, v: f32| -> String {
        match k {
            aw::PRESSURE => format!("{v:.1}"),
            aw::VIBRATO => format!("{v:.0}"),
            aw::VIB_RATE => format!("{v:.1}"),
            aw::GLIDE => format!("{:.0}", v * 1000.0),
            aw::LEVEL => format!("{v:+.0}"),
            aw::JAW | aw::TONGUE | aw::LIPS | aw::LARYNX => format!("{v:+.2}"),
            _ => format!("{:.0}", v * 100.0),
        }
    };
    let big: [(usize, &str, Color32); 4] = [(aw::PRESSURE, "BREATH", RED), (aw::TENSION, "PRESSED", PINK), (aw::BREATH, "AIRY", CYAN), (aw::VIBRATO, "VIBRATO", WHITE)];
    let small_k: [(usize, &str, Color32); 12] = [
        (aw::FRY, "FRY", ORANGE),
        (aw::ROUGH, "ROUGH", ORANGE),
        (aw::BRIGHT, "BRIGHT", CYAN),
        (aw::VIB_RATE, "VIB HZ", WHITE),
        (aw::GLIDE, "GLIDE MS", GREEN),
        (aw::LEVEL, "LEVEL", WHITE),
        (aw::JAW, "JAW", RED),
        (aw::TONGUE, "TONGUE", GREEN),
        (aw::LIPS, "LIPS", PINK),
        (aw::LARYNX, "LARYNX", CYAN),
        (aw::NASAL, "NASAL", DEEP),
        (aw::MORPH, "MORPH", GREEN),
    ];
    let colw = inner.width() / 4.0;
    let big_r = (colw * 0.26).min(inner.height() * 0.1).clamp(11.0, 24.0);
    let y_big = inner.top() + 14.0 + big_r;
    for (k, (pi, label, col)) in big.iter().enumerate() {
        let at = pos2(inner.left() + colw * (k as f32 + 0.5), y_big);
        let mut v = params[*pi];
        let shown = fmt(*pi, v);
        if knob(ui, id.with(("knob", *pi)), at, big_r, &mut v, &aw::SPECS[*pi], label, shown, *col) {
            params[*pi] = v;
        }
    }
    let small_w = inner.width() / 6.0;
    let top = y_big + big_r + 26.0;
    let row_h = (inner.bottom() - top) / 2.0;
    let small_r = (row_h * 0.5 - 13.0).min(small_w * 0.28).clamp(7.0, 16.0);
    for (k, (pi, label, col)) in small_k.iter().enumerate() {
        let at = pos2(inner.left() + small_w * ((k % 6) as f32 + 0.5), top + row_h * ((k / 6) as f32 + 0.5));
        let mut v = params[*pi];
        let shown = fmt(*pi, v);
        if knob(ui, id.with(("knob", *pi)), at, small_r, &mut v, &aw::SPECS[*pi], label, shown, *col) {
            params[*pi] = v;
        }
    }
    // The vowel it morphs to, by the morph knob.
    let _ = aw::MORPH_TO;

    if params[aw::BODY].round() as u8 != body_was {
        *plans = aw::prepare_with(text, params[aw::BODY].round() as u8, &words_of(plans));
    }
    if live.vals[..4].iter().any(|v| *v > 0.0) || vals[aw::VIS_VOICING] > 0.01 {
        ui.ctx().request_repaint();
    }
    said
}

impl crate::app::App {
    /// Every Airway on a track with a Sing sequencer has its words planned
    /// (each syllable Sing sends, sung whole on its note).
    pub(crate) fn sync_airways(&mut self) {
        use krabton_engine::midi_fx::MidiFxKind;
        use krabton_engine::project::Instrument;
        for t in &mut self.project.tracks {
            let Some(Instrument::Airway { params, text, plans, .. }) = &mut t.instrument else { continue };
            let mut words: Vec<String> = Vec::new();
            for d in t.midi_devices.iter().filter(|d| d.kind == MidiFxKind::Sing) {
                for w in krabton_engine::sequencers::sing::words(&d.data) {
                    words.extend(w);
                }
            }
            if !aw::has_words(plans, &words) {
                *plans = aw::prepare_with(text, params[aw::BODY].round() as u8, &words);
            }
        }
    }
}

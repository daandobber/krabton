//! The Look as a patch, as a modular synth has it: the sources (the film,
//! the Visuals) on the left, the effects the Look uses joined by a cable in
//! the order they work on the picture, to the Output; the music's
//! modulators (a kick, the bass, an LFO...) as round modules under them,
//! each with a cable to the effects it moves and a small knob on the cable
//! for how much. Only what the Look uses is there: *+ Add* puts more in,
//! a module's x, a right-click on it, Delete or its panel's Remove take it out; drag from a modulator to an
//! effect to make it move that too; click a module for its settings.

use super::video_view::{effect_detail, milk_panel, scene_detail, track_name, visuals_head, GROUPS};

/// The module a link goes to: the Visuals' own, or the effect's.
fn target(e: Effect) -> Node {
    if e == Effect::Visuals {
        Node::Visuals
    } else {
        Node::Fx(e)
    }
}
use super::vj;
use crate::app::App;
use crate::theme::*;
use crate::video::VideoTab;
use crate::widgets::*;
use egui::epaint::CubicBezierShape;
use egui::{pos2, vec2, Align2, Color32, Id, Key, Pos2, Rect, RichText, Sense, Stroke, Ui, Vec2};
use krabton_engine::video::{self as kv, Effect, Source, VideoLink};

/// The order the effects work on the picture: those on the film's place,
/// those on the Visuals, then the picture's (as the stage's shader has them).
pub(crate) const FILM_ORDER: [Effect; 6] = [Effect::Speed, Effect::Freeze, Effect::Jump, Effect::Stutter, Effect::Scratch, Effect::NextClip];
pub(crate) const VIS_ORDER: [Effect; 11] = [Effect::Burst, Effect::Warp, Effect::Density, Effect::Rush, Effect::Glow, Effect::Orbit, Effect::Fracture, Effect::Depth, Effect::Detail, Effect::Thickness, Effect::Scatter];
pub(crate) const MAIN_ORDER: [Effect; 46] = [
    Effect::Ripple,
    Effect::Fisheye,
    Effect::Swirl,
    Effect::Melt,
    Effect::Shred,
    Effect::Puzzle,
    Effect::Checker,
    Effect::Tile,
    Effect::Shake,
    Effect::Spin,
    Effect::Zoom,
    Effect::Pixels,
    Effect::Glitch,
    Effect::Split,
    Effect::ZoomBlur,
    Effect::Blur,
    Effect::Mirror,
    Effect::Kaleido,
    Effect::Prism,
    Effect::Polar,
    Effect::Droste,
    Effect::PopArt,
    Effect::Edges,
    Effect::Hue,
    Effect::Grade,
    Effect::Poster,
    Effect::Tint,
    Effect::Invert,
    Effect::Smear,
    Effect::Bloom,
    Effect::Map,
    Effect::Halftone,
    Effect::Ascii,
    Effect::Dither,
    Effect::Flash,
    Effect::Vhs,
    Effect::Vignette,
    Effect::Scanlines,
    Effect::Grain,
    Effect::Gate,
    Effect::Strobe,
    Effect::Trails,
    Effect::Echo,
    Effect::Datamosh,
    Effect::Wave,
    Effect::Feedback,
];

/// The picture's effects in the order this Look has them work: MAIN_ORDER,
/// with the shape and the picture stages as the Look puts them.
pub(crate) fn main_order(look: &kv::VideoLook) -> Vec<Effect> {
    let mut stages: Vec<std::vec::IntoIter<Effect>> = kv::STAGES.iter().map(|s| look.stage_order(s).into_iter()).collect();
    MAIN_ORDER.iter().map(|e| kv::stage_index(*e).and_then(|k| stages[k].next()).unwrap_or(*e)).collect()
}

/// Whether `a` and `b` are in the same stage (so one can take the other's place).
fn same_stage(a: Effect, b: Effect) -> bool {
    kv::stage_index(a).is_some() && kv::stage_index(a) == kv::stage_index(b)
}

/// A module of the patch.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Node {
    Film,
    Visuals,
    Mix,
    Fx(Effect),
    /// A modulator: what it hears, and of which track.
    Mod(Source, Option<u64>),
    Output,
}

const EFF: Vec2 = vec2(124.0, 58.0);
const SRC: Vec2 = vec2(150.0, 66.0);
const MOD_R: f32 = 24.0;
const GAP: f32 = 36.0;

/// The colour of a module (by the desk's groups; a modulator by what it hears).
fn colour_of(n: Node) -> Color32 {
    match n {
        Node::Fx(e) => GROUPS.iter().find(|g| g.2.contains(&e)).map_or(ACCENT, |g| Color32::from_rgb(g.1[0], g.1[1], g.1[2])),
        Node::Mod(s, _) => {
            let k = Source::ALL.iter().position(|x| *x == s).unwrap_or(0);
            egui::ecolor::Hsva::new((k as f32 * 0.137 + 0.55).fract(), 0.55, 0.95, 1.0).into()
        }
        Node::Visuals => Color32::from_rgb(0xa0, 0x78, 0xff),
        Node::Film => Color32::from_rgb(0xff, 0x60, 0xa0),
        _ => TEXT,
    }
}

/// Where the modules sit: the sources and theirs in rows, then the
/// picture's effects in rows to the Output, the modulators under them.
pub(crate) fn layout(film: bool, visuals: bool, used: &[Effect], order: &[Effect], mods: &[(Node, Vec<Effect>)], width: f32) -> Vec<(Node, Rect)> {
    let mut out: Vec<(Node, Rect)> = Vec::new();
    let mut y = 12.0;
    let mut sources = 0;
    let row_h = SRC.y + 28.0;
    for (on, node, order) in [(film, Node::Film, &FILM_ORDER[..]), (visuals, Node::Visuals, &VIS_ORDER[..])] {
        if !on {
            continue;
        }
        sources += 1;
        let mut x = 12.0;
        out.push((node, Rect::from_min_size(pos2(x, y), SRC)));
        x += SRC.x + GAP;
        for e in order.iter().filter(|e| used.contains(e)) {
            // On in rows (under the source's own, not under the source).
            if x + EFF.x > width - 12.0 {
                x = 12.0 + SRC.x + GAP;
                y += row_h;
            }
            out.push((Node::Fx(*e), Rect::from_min_size(pos2(x, y + (SRC.y - EFF.y) / 2.0), EFF)));
            x += EFF.x + GAP;
        }
        y += row_h;
    }
    // The picture's effects (after the mix of the two sources), row after row, to the Output.
    let mut x = 12.0;
    y += 8.0;
    let mut chain: Vec<Node> = Vec::new();
    if sources == 2 {
        chain.push(Node::Mix);
    }
    chain.extend(order.iter().filter(|e| used.contains(e)).map(|e| Node::Fx(*e)));
    chain.push(Node::Output);
    for n in chain {
        let size = match n {
            Node::Output => SRC,
            Node::Mix => vec2(96.0, 50.0),
            _ => EFF,
        };
        if x + size.x > width - 12.0 && x > 12.0 {
            x = 12.0;
            y += row_h;
        }
        out.push((n, Rect::from_min_size(pos2(x, y + (SRC.y - size.y) / 2.0), size)));
        x += size.x + GAP;
    }
    // The modulators, each under the effects it moves.
    let my = y + SRC.y + 56.0;
    let mut placed: Vec<(Node, f32)> = mods
        .iter()
        .map(|(m, targets)| {
            let xs: Vec<f32> = targets.iter().filter_map(|e| out.iter().find(|(n, _)| *n == target(*e)).map(|(_, r)| r.center().x)).collect();
            (*m, if xs.is_empty() { 40.0 } else { xs.iter().sum::<f32>() / xs.len() as f32 })
        })
        .collect();
    placed.sort_by(|a, b| a.1.total_cmp(&b.1));
    let mut next = 12.0 + MOD_R;
    let mut row = 0.0;
    for (m, want) in placed {
        let mut cx = want.max(next);
        if cx > width - MOD_R - 12.0 {
            row += 1.0;
            cx = 12.0 + MOD_R;
        }
        out.push((m, Rect::from_center_size(pos2(cx, my + row * (MOD_R * 2.0 + 30.0)), vec2(MOD_R * 2.0, MOD_R * 2.0))));
        next = cx + MOD_R * 2.0 + 22.0;
    }
    out
}

/// A signal cable from module `a` to module `b`: along the row, or down and round to the next.
fn join(p: &egui::Painter, a: Rect, b: Rect, stroke: Stroke) {
    if b.left() < a.right() - 4.0 && b.top() > a.center().y {
        cable(p, pos2(a.center().x, a.bottom()), pos2(b.center().x, b.top()), true, stroke);
    } else {
        cable(p, pos2(a.right(), a.center().y), pos2(b.left(), b.center().y), false, stroke);
    }
}

/// A modulator's cable: up out of its head, into the foot of what it moves.
fn hang(p: &egui::Painter, a: Pos2, b: Pos2, stroke: Stroke) -> CubicBezierShape {
    let d = ((b - a).length() * 0.35).clamp(20.0, 90.0);
    let shape = CubicBezierShape::from_points_stroke([a, a - vec2(0.0, d), b + vec2(0.0, d), b], false, Color32::TRANSPARENT, stroke);
    p.add(shape.clone());
    shape
}

fn cable(p: &egui::Painter, a: Pos2, b: Pos2, down: bool, stroke: Stroke) -> CubicBezierShape {
    let d = ((b - a).length() * 0.4).clamp(20.0, 120.0);
    // Down from a module's foot into the next one's head, or out of its side into the next's.
    let (c1, c2) = if down { (a + vec2(0.0, d), b - vec2(0.0, d)) } else { (a + vec2(d, 0.0), b - vec2(d, 0.0)) };
    let shape = CubicBezierShape::from_points_stroke([a, c1, c2, b], false, Color32::TRANSPARENT, stroke);
    p.add(shape.clone());
    shape
}

impl App {
    /// The Look as a patch (see the module's words).
    pub(crate) fn video_patch(&mut self, ui: &mut Ui, set: &mut kv::VideoSet, tracks: &[(u64, String)]) {
        let n_looks = set.looks.len().max(1);
        let k = set.look.min(n_looks - 1);
        let film = !set.clips.is_empty();
        let live = (self.video.dir.look_playing() == k).then_some(self.video.dir.look);
        let readings = self.video.dir.readings.clone();
        let milk_now = live.is_some().then(|| (self.video.dir.milk.name(), self.video.dir.milk_frame.now));
        let sel_id = Id::new("video_patch_sel");
        let mut selected: Option<Node> = ui.ctx().data(|d| d.get_temp(sel_id)).flatten();
        let drag_id = Id::new("video_patch_drag");
        let mut dragging: Option<Node> = ui.ctx().data(|d| d.get_temp(drag_id)).flatten();
        let mut add_link: Option<(Source, Option<u64>, Effect)> = None;
        let mut remove: Option<Node> = None;
        let mut add_fx: Option<Effect> = None;
        let mut go_clip = false;
        let mut reorder: Option<(Effect, Effect)> = None;
        let mut dice_all: Option<u32> = None;

        // The head: the Look's name, how much the music moves it, Add, Presets.
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            let look = set.edited_look_mut();
            ui.add(egui::TextEdit::singleline(&mut look.name).desired_width(120.0).hint_text(format!("Look {}", k + 1)));
            let add = ui.add(egui::Button::new(RichText::new("+ Add").color(TEXT_DARK)).fill(ACCENT).corner_radius(4.0).min_size(vec2(0.0, vj::H))).on_hover_text("Put an effect or the Visuals in this Look");
            let used = look.effects();
            egui::Popup::menu(&add).show(|ui| {
                ui.set_max_width(560.0);
                ui.label(RichText::new("SOURCES").small().color(TEXT_DIM));
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = vec2(3.0, 3.0);
                    if vj::word_tile(ui, "Visuals", !used.contains(&Effect::Visuals), "Pictures Krabton draws itself (pick the scene in its module or in the list on the right)").clicked() {
                        add_fx = Some(Effect::Visuals);
                        ui.close();
                    }
                });
                for (title, _, effects) in GROUPS.iter() {
                    ui.label(RichText::new(*title).small().color(TEXT_DIM));
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = vec2(3.0, 3.0);
                        for e in effects.iter().filter(|e| **e != Effect::Visuals) {
                            if vj::word_tile(ui, e.name(), !used.contains(e), e.about()).clicked() {
                                add_fx = Some(*e);
                                ui.close();
                            }
                        }
                    });
                }
            });
            let shuffle = ui
                .add(egui::Button::new(RichText::new("Shuffle").color(TEXT)).corner_radius(4.0).min_size(vec2(0.0, vj::H)))
                .on_hover_text("The effects in another order by chance: the same effects make another picture.\nDrag a module onto another of its kind to take its place; right-click: back to the order they start in");
            if shuffle.clicked() {
                let seed = (ui.input(|i| i.time) * 1000.0) as u32 ^ (k as u32).wrapping_mul(0x9e37_79b9);
                look.shuffle_order(seed);
            }
            shuffle.context_menu(|ui| {
                if ui.button("Order as they start").clicked() {
                    look.reset_order();
                    ui.close();
                }
            });
            let dice = ui
                .add(egui::Button::new(RichText::new("Dice").color(TEXT_DARK)).fill(Color32::from_rgb(0xff, 0x4f, 0xa3)).corner_radius(4.0).min_size(vec2(0.0, vj::H)))
                .on_hover_text("A wild Look by chance: effects that bend, fold, colour and light it, at settings by chance, each moved by something in the music, in an order by chance (the Visuals and the film's stay). Again for another; Ctrl+Z for the one before. Right-click: every Look at once");
            if dice.clicked() {
                let seed = (ui.input(|i| i.time) * 1000.0) as u32 ^ (k as u32 + 1).wrapping_mul(0x2545_f491);
                look.dice(seed);
                selected = None;
            }
            dice.context_menu(|ui| {
                if ui.button("Dice every Look").on_hover_text("Every Look of the set by chance, each its own (with AUTO by the Looks: a new picture every change)").clicked() {
                    dice_all = Some((ui.input(|i| i.time) * 1000.0) as u32);
                    ui.close();
                }
            });
            // The order changing by itself.
            ui.label(RichText::new("ORDER").small().strong().color(if look.reorder.is_some() { ACCENT } else { TEXT_DIM }));
            let mut how: u8 = match (look.reorder.is_some(), look.reorder_turn) {
                (false, _) => 0,
                (true, false) => 1,
                (true, true) => 2,
            };
            let hows = [(0u8, "Fixed".to_string()), (1, "Shuffle".to_string()), (2, "Turn".to_string())];
            if vj::segmented(ui, &mut how, &hows, "The order changing by itself while the song plays: Fixed (as it is set), Shuffle (another by chance every time) or Turn (each moving on a place, round again)") {
                look.reorder = if how == 0 { None } else { Some(look.reorder.unwrap_or(kv::Every::Bar)) };
                look.reorder_turn = how == 2;
            }
            if let Some(mut every) = look.reorder {
                let evs: Vec<(kv::Every, String)> = kv::Every::ALL.iter().map(|e| (*e, e.name().to_string())).collect();
                if vj::chip(ui, "video_reorder_every", 62.0, &mut every, &evs, "How often: a sixteenth, an eighth, a beat, a bar, two bars or four") {
                    look.reorder = Some(every);
                }
            }
            vj::bar(ui, &mut set.amount, 0.0, 1.0, 1.0, "Music", vj::pct, 150.0, None, "How much the music moves the picture: every cable of every Look at once");
        });
        if let Some(seed) = dice_all {
            for (i, l) in set.looks.iter_mut().enumerate() {
                l.dice(seed ^ (i as u32 + 1).wrapping_mul(0x9e37_79b9));
            }
            selected = None;
        }
        ui.add_space(6.0);

        let look = set.edited_look_mut();
        let used = look.effects();
        let visuals = used.contains(&Effect::Visuals);
        // The modulators: what the links hear (and of which track), and what they move.
        let mut mods: Vec<(Node, Vec<Effect>)> = Vec::new();
        for l in &look.links {
            let n = Node::Mod(l.source, l.from);
            match mods.iter_mut().find(|m| m.0 == n) {
                Some(m) => m.1.push(l.effect),
                None => mods.push((n, vec![l.effect])),
            }
        }
        let width = ui.available_width();
        let order = main_order(look);
        let placed = layout(film, visuals, &used, &order, &mods, width);
        let height = placed.iter().map(|(_, r)| r.bottom()).fold(0.0, f32::max) + 24.0;
        let (canvas, bg) = ui.allocate_exact_size(vec2(width, height.max(200.0)), Sense::click());
        let origin = canvas.min.to_vec2();
        let rect_of = |n: Node| placed.iter().find(|(m, _)| *m == n).map(|(_, r)| r.translate(origin));
        let p = ui.painter().with_clip_rect(canvas);
        rect_fill(&p, canvas, 4.0, Color32::from_rgb(0x15, 0x16, 0x1c));
        // A grid of dots, as a patch panel.
        let mut gy = canvas.top() + 12.0;
        while gy < canvas.bottom() {
            let mut gx = canvas.left() + 12.0;
            while gx < canvas.right() {
                p.circle_filled(pos2(gx, gy), 0.8, Color32::from_white_alpha(14));
                gx += 24.0;
            }
            gy += 24.0;
        }

        // The signal: the sources (and theirs) to the mix, the picture's effects to the Output.
        let signal = Stroke::new(3.0_f32, ACCENT.gamma_multiply(0.8));
        let out_of = |r: Rect| pos2(r.right(), r.center().y);
        let in_of = |r: Rect| pos2(r.left(), r.center().y);
        let mut source_ends: Vec<Rect> = Vec::new();
        for (on, src, order) in [(film, Node::Film, &FILM_ORDER[..]), (visuals, Node::Visuals, &VIS_ORDER[..])] {
            if !on {
                continue;
            }
            let mut last = rect_of(src);
            for e in order.iter().filter(|e| used.contains(e)) {
                if let (Some(a), Some(b)) = (last, rect_of(Node::Fx(*e))) {
                    join(&p, a, b, signal);
                    last = Some(b);
                }
            }
            source_ends.extend(last);
        }
        let mixer = rect_of(Node::Mix);
        if let Some(m) = mixer {
            for r in &source_ends {
                join(&p, *r, m, signal);
            }
        }
        let mut last = mixer.or(source_ends.first().copied());
        for n in order.iter().filter(|e| used.contains(e)).map(|e| Node::Fx(*e)).chain(std::iter::once(Node::Output)) {
            if let Some(b) = rect_of(n) {
                if let Some(a) = last {
                    join(&p, a, b, signal);
                }
                last = Some(b);
            }
        }

        // The modulators' cables, each with a knob for how much.
        let pointer = ui.input(|i| i.pointer.hover_pos());
        let scene_now = look.setting(Effect::Visuals).param(Effect::Visuals, 0).round().max(0.0) as usize;
        for (i, l) in look.links.iter_mut().enumerate() {
            let (Some(a), Some(b)) = (rect_of(Node::Mod(l.source, l.from)), rect_of(target(l.effect))) else { continue };
            let col = colour_of(Node::Mod(l.source, l.from));
            let from = pos2(a.center().x, a.top());
            let to = pos2(b.center().x, b.bottom());
            let live_v = readings.get(i).copied().unwrap_or(0.0).clamp(0.0, 1.0);
            let shape = hang(&p, from, to, Stroke::new(1.5 + 2.0 * live_v, col.gamma_multiply(0.45 + 0.5 * live_v)));
            let mid = shape.sample(0.5);
            // The knob: drag up or down for how much.
            let kr = Rect::from_center_size(mid, vec2(18.0, 18.0));
            let resp = ui.interact(kr, Id::new(("video_patch_knob", i)), Sense::click_and_drag());
            if resp.dragged() {
                l.amount = (l.amount - resp.drag_delta().y * 0.005).clamp(0.0, 1.0);
            }
            if resp.double_clicked() {
                l.amount = 0.5;
            }
            p.circle_filled(mid, 8.0, Color32::from_rgb(0x20, 0x22, 0x2a));
            let arc: Vec<Pos2> = (0..=16).map(|s| {
                let t = -2.3 + 4.6 * l.amount * s as f32 / 16.0;
                mid + vec2(t.sin(), -t.cos()) * 7.0
            }).collect();
            p.add(egui::Shape::line(arc, Stroke::new(2.0_f32, col)));
            if resp.hovered() || resp.dragged() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
            }
            let how = if l.effect.on_hits() { "how often a hit does it" } else { "how much at full" };
            let what = match l.param {
                Some(k) => format!("the Visuals' {}", kv::scene_params(scene_now).get(k as usize).map_or("setting", |d| d.name)),
                None => l.effect.name().to_string(),
            };
            let resp = resp.on_hover_text(format!("{} moves {}: {} {:.0} %. Drag up or down, double-click: half. Right-click: take it away", l.source.name(), what, how, l.amount * 100.0));
            if resp.secondary_clicked() {
                // This cable alone (taken away below).
                ui.ctx().data_mut(|d| d.insert_temp(Id::new("video_patch_unlink"), i));
            }
        }
        // A cable being drawn from a modulator.
        if let (Some(Node::Mod(s, f)), Some(pt)) = (dragging, pointer) {
            if let Some(a) = rect_of(Node::Mod(s, f)) {
                hang(&p, pos2(a.center().x, a.top()), pt, Stroke::new(2.0_f32, colour_of(Node::Mod(s, f))));
            }
        }

        // The modules.
        let fx_live = |e: Effect, look: &kv::VideoLook| live.map_or(look.setting(e).base, |l| l.get(e));
        for (n, r) in placed.iter().map(|(n, r)| (*n, r.translate(origin))) {
            let col = colour_of(n);
            let picked = selected == Some(n);
            let resp = ui.interact(r, Id::new(("video_patch_node", format!("{n:?}"))), Sense::click_and_drag());
            match n {
                Node::Mod(s, from) => {
                    let first = look.links.iter().position(|l| l.source == s && l.from == from);
                    let v = first.and_then(|i| readings.get(i).copied()).unwrap_or(0.0).clamp(0.0, 1.0);
                    p.circle_filled(r.center(), MOD_R, Color32::from_rgb(0x20, 0x22, 0x2a));
                    p.circle_filled(r.center(), MOD_R * (0.25 + 0.6 * v), col.gamma_multiply(0.35 + 0.5 * v));
                    p.circle_stroke(r.center(), MOD_R, Stroke::new(if picked { 2.5 } else { 1.5_f32 }, col));
                    text(&p, r.center(), Align2::CENTER_CENTER, s.name(), small(), TEXT);
                    if s.has_track() {
                        let who = track_name(tracks, from, if s.listens() { "master" } else { "all" });
                        text(&p, pos2(r.center().x, r.bottom() + 8.0), Align2::CENTER_CENTER, who, small(), TEXT_DIM);
                    }
                    if resp.drag_started() {
                        dragging = Some(n);
                    }
                    resp.clone().on_hover_text(format!("{}: {}\nDrag to an effect to move it too; click for its settings", s.name(), s.about()));
                }
                _ => {
                    let fill = if picked { mix(PANEL2, col, 0.18) } else { mix(PANEL2, col, 0.08) };
                    p.rect_filled(r, 6.0, fill);
                    p.rect_stroke(r, 6.0, Stroke::new(if picked { 2.0 } else { 1.0_f32 }, col.gamma_multiply(if picked { 1.0 } else { 0.6 })), egui::StrokeKind::Inside);
                    // Its ports.
                    if n != Node::Film && n != Node::Visuals {
                        p.circle_filled(in_of(r), 4.0, col);
                    }
                    if n != Node::Output {
                        p.circle_filled(out_of(r), 4.0, col);
                    }
                    let title = match n {
                        Node::Film => "FILM".to_string(),
                        Node::Visuals => "VISUALS".to_string(),
                        Node::Mix => "MIX".to_string(),
                        Node::Output => "OUTPUT".to_string(),
                        Node::Fx(e) => e.name().to_uppercase(),
                        Node::Mod(..) => String::new(),
                    };
                    text(&p, pos2(r.left() + 8.0, r.top() + 11.0), Align2::LEFT_CENTER, title, small(), col);
                    let body = Rect::from_min_max(pos2(r.left() + 8.0, r.top() + 22.0), pos2(r.right() - 8.0, r.bottom() - 6.0));
                    match n {
                        Node::Film => {
                            let name = set_clip_name(film, &self.project.video);
                            text(&p, body.left_center(), Align2::LEFT_CENTER, name, body_font(), TEXT);
                            resp.clone().on_hover_text("The clips of the deck: click to edit the one that plays (the Clip tab)");
                            if resp.clicked() {
                                go_clip = true;
                            }
                        }
                        Node::Visuals => {
                            let scene = look.setting(Effect::Visuals).param(Effect::Visuals, 0).round() as usize;
                            let icon = Rect::from_min_size(body.min, vec2(46.0, body.height()));
                            p.rect_filled(icon, 2.0, Color32::BLACK);
                            vj::scene_icon(&p.with_clip_rect(icon), icon.shrink(2.0), scene, col);
                            text(&p, pos2(icon.right() + 8.0, body.center().y), Align2::LEFT_CENTER, kv::VISUAL_SCENES.get(scene).copied().unwrap_or(""), body_font(), TEXT);
                        }
                        Node::Mix => {
                            let b = look.setting(Effect::Visuals).param(Effect::Visuals, 1).round() as usize;
                            text(&p, body.left_center(), Align2::LEFT_CENTER, kv::VISUAL_BLENDS.get(b).copied().unwrap_or(""), small(), TEXT_DIM);
                        }
                        Node::Output => {
                            text(&p, body.left_center(), Align2::LEFT_CENTER, "to the screen", small(), TEXT_DIM);
                        }
                        Node::Fx(e) => {
                            // How much of it there is now (and always).
                            let s = look.setting(e);
                            let bar = Rect::from_min_max(pos2(body.left(), body.center().y + 2.0), pos2(body.right(), body.center().y + 8.0));
                            p.rect_filled(bar, 2.0, PANEL3);
                            let now = fx_live(e, look).clamp(0.0, 1.0);
                            p.rect_filled(Rect::from_min_size(bar.min, vec2(bar.width() * now, bar.height())), 2.0, col.gamma_multiply(if s.off { 0.3 } else { 0.85 }));
                            p.line_segment([pos2(bar.left() + bar.width() * s.base, bar.top() - 3.0), pos2(bar.left() + bar.width() * s.base, bar.bottom() + 3.0)], Stroke::new(1.5_f32, TEXT));
                            if s.off {
                                text(&p, pos2(body.right(), r.top() + 11.0), Align2::RIGHT_CENTER, "off", small(), TEXT_DIM);
                            }
                            // A module of its stage dropped on it: they change places.
                            if resp.drag_started() && kv::stage_of(e).is_some() {
                                dragging = Some(n);
                            }
                            if let Some(Node::Fx(d)) = dragging {
                                if d != e && same_stage(d, e) && pointer.is_some_and(|pt| r.contains(pt)) {
                                    p.rect_stroke(r.expand(3.0), 7.0, Stroke::new(2.0_f32, colour_of(Node::Fx(d))), egui::StrokeKind::Outside);
                                    if ui.input(|i| i.pointer.any_released()) {
                                        reorder = Some((d, e));
                                    }
                                }
                            }
                            // A cable dropped on it: it is moved by that modulator too.
                            if let Some(Node::Mod(src, from)) = dragging {
                                if pointer.is_some_and(|pt| r.contains(pt)) {
                                    p.rect_stroke(r.expand(3.0), 7.0, Stroke::new(2.0_f32, colour_of(Node::Mod(src, from))), egui::StrokeKind::Outside);
                                    if ui.input(|i| i.pointer.any_released()) {
                                        add_link = Some((src, from, e));
                                    }
                                }
                            }
                            let tip = if kv::stage_of(e).is_some() { "Click: its settings and what moves it. Drag onto another of its kind: they change places" } else { "Click: its settings and what moves it" };
                            resp.clone().on_hover_text(format!("{}\n{tip}", e.about()));
                        }
                        Node::Mod(..) => {}
                    }
                    // Taken out with its x (shown while the pointer is anywhere on it, the x included).
                    if matches!(n, Node::Fx(_) | Node::Visuals) && (pointer.is_some_and(|pt| r.contains(pt)) || picked) {
                        let x = Rect::from_center_size(pos2(r.right() - 9.0, r.top() + 10.0), vec2(14.0, 14.0));
                        let xr = ui.interact(x, Id::new(("video_patch_x", format!("{n:?}"))), Sense::click());
                        text(&p, x.center(), Align2::CENTER_CENTER, "x", small(), if xr.hovered() { TEXT } else { TEXT_DIM });
                        if xr.on_hover_text("Take it out of the Look (and its cables)").clicked() {
                            remove = Some(n);
                        }
                    }
                }
            }
            // Right-click: take it out.
            if matches!(n, Node::Fx(_) | Node::Visuals | Node::Mod(..)) {
                resp.context_menu(|ui| {
                    if ui.button("Remove").on_hover_text("Take it out of the Look, with its cables").clicked() {
                        remove = Some(n);
                        ui.close();
                    }
                });
            }
            if resp.clicked() && n != Node::Film {
                selected = if selected == Some(n) { None } else { Some(n) };
            }
        }
        // A module being moved: it goes with the pointer.
        if let (Some(Node::Fx(d)), Some(pt)) = (dragging, pointer) {
            if let Some(r) = rect_of(Node::Fx(d)) {
                let ghost = Rect::from_center_size(pt, r.size());
                let col = colour_of(Node::Fx(d));
                p.rect_filled(ghost, 6.0, mix(PANEL2, col, 0.25).gamma_multiply(0.85));
                p.rect_stroke(ghost, 6.0, Stroke::new(1.5_f32, col), egui::StrokeKind::Inside);
                text(&p, ghost.center(), Align2::CENTER_CENTER, d.name().to_uppercase(), small(), col);
                ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
            }
        }
        if let Some((d, onto)) = reorder {
            if let Some(stage) = kv::stage_of(d) {
                let to = look.stage_order(stage).iter().position(|x| *x == onto).unwrap_or(0);
                look.move_effect(d, to);
            }
        }
        if ui.input(|i| i.pointer.any_released()) {
            dragging = None;
        }
        if bg.clicked() {
            selected = None;
        }
        // Delete (or Backspace) takes the module picked out, unless a text is being typed.
        if selected.is_some() && !ui.ctx().wants_keyboard_input() && ui.input(|i| i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace)) {
            remove = selected;
        }
        if used.is_empty() {
            text(&p, canvas.center() - vec2(0.0, 0.0), Align2::CENTER_CENTER, "Nothing in this Look yet: + Add an effect or the Visuals (or pick a scene on the right)", small(), TEXT_DIM);
        }

        // The settings of the module picked, floating by it.
        if let Some(n) = selected {
            if let Some(r) = rect_of(n) {
                let at = pos2((r.left()).min(canvas.right() - 320.0), r.bottom() + 8.0);
                let mut add_src: Option<Source> = None;
                let mut close = false;
                egui::Area::new(Id::new("video_patch_panel")).order(egui::Order::Middle).fixed_pos(at).show(ui.ctx(), |ui| {
                    egui::Frame::new().fill(PANEL).stroke(Stroke::new(1.0_f32, LINE)).corner_radius(6).inner_margin(egui::Margin::same(8)).shadow(egui::Shadow { offset: [0, 4], blur: 12, spread: 0, color: Color32::from_black_alpha(120) }).show(ui, |ui| {
                        ui.set_width(300.0);
                        match n {
                            Node::Fx(e) => {
                                close = effect_detail(ui, look, e, &readings, tracks, &mut add_src);
                                ui.add_space(4.0);
                                ui.horizontal(|ui| {
                                    if ui.small_button("Remove").on_hover_text("Take it out of the Look, with its cables (or Delete)").clicked() {
                                        remove = Some(n);
                                    }
                                    if kv::stage_of(e).is_some() {
                                        ui.add_space(8.0);
                                        if ui.small_button("< Earlier").on_hover_text("Let it work on the picture before the one left of it").clicked() {
                                            look.nudge_effect(e, -1);
                                        }
                                        if ui.small_button("Later >").on_hover_text("Let it work on the picture after the one right of it").clicked() {
                                            look.nudge_effect(e, 1);
                                        }
                                    }
                                });
                            }
                            Node::Visuals | Node::Mix => {
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new("Visuals").strong().color(TEXT));
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        if ui.add(egui::Button::new(RichText::new("x").color(TEXT_DIM)).frame(false)).clicked() {
                                            close = true;
                                        }
                                    });
                                });
                                let scene = look.setting(Effect::Visuals).param(Effect::Visuals, 0).round() as usize;
                                egui::Grid::new("video_patch_scenes").spacing([3.0, 3.0]).show(ui, |ui| {
                                    for (k, name) in kv::VISUAL_SCENES.iter().enumerate() {
                                        if vj::tile(ui, vec2(56.0, 42.0), k == scene, name, "A scene for the Visuals", |p, r, c| vj::scene_icon(p, r, k, c)).clicked() {
                                            look.setting_mut(Effect::Visuals).set_param(Effect::Visuals, 0, k as f32);
                                        }
                                        if k % 5 == 4 {
                                            ui.end_row();
                                        }
                                    }
                                });
                                ui.horizontal(|ui| visuals_head(ui, look));
                                let s = look.setting_mut(Effect::Visuals);
                                let mut v = s.param(Effect::Visuals, 3);
                                if vj::bar(ui, &mut v, 0.0, 4.0, 1.0, "Speed", |v| format!("{v:.2}"), 0.0, None, "How fast they move by themselves (Rush makes them faster)").changed() {
                                    s.set_param(Effect::Visuals, 3, v);
                                }
                                vj::bar(ui, &mut s.base, 0.0, 1.0, 1.0, "Amount", vj::pct, 0.0, None, "How much of the Visuals there are, with the film");
                                if scene == crate::video::stage::MILK_SCENE {
                                    milk_panel(ui, &mut look.milk, milk_now);
                                }
                                scene_detail(ui, look, &readings, tracks);
                                ui.add_space(4.0);
                                if ui.small_button("Remove").on_hover_text("Take the Visuals out of the Look, with their effects and cables (or Delete)").clicked() {
                                    remove = Some(Node::Visuals);
                                }
                            }
                            Node::Mod(s, from) => {
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new(s.name()).strong().color(colour_of(n)));
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        if ui.add(egui::Button::new(RichText::new("x").color(TEXT_DIM)).frame(false)).clicked() {
                                            close = true;
                                        }
                                        if ui.small_button("Remove").on_hover_text("Take it and its cables away").clicked() {
                                            remove = Some(n);
                                        }
                                    });
                                });
                                ui.label(RichText::new(s.about()).small().color(TEXT_DIM));
                                // What it hears and of which track, for every cable it has.
                                let mut src = s;
                                let mut who = from;
                                let sources: Vec<(Source, String)> = Source::ALL.iter().map(|s| (*s, s.name().to_string())).collect();
                                ui.horizontal(|ui| {
                                    vj::chip(ui, "video_patch_mod_src", 90.0, &mut src, &sources, "What in the music it follows");
                                    if src.has_track() {
                                        let none = if src.listens() { "Master" } else { "All tracks" };
                                        let mut options: Vec<(Option<u64>, String)> = vec![(None, none.to_string())];
                                        options.extend(tracks.iter().map(|(id, n)| (Some(*id), n.clone())));
                                        vj::chip(ui, "video_patch_mod_from", 110.0, &mut who, &options, "The sound it listens to, or whose notes");
                                    }
                                });
                                if src != s || who != from {
                                    for l in look.links.iter_mut().filter(|l| l.source == s && l.from == from) {
                                        l.source = src;
                                        l.from = who;
                                    }
                                    selected = Some(Node::Mod(src, who));
                                }
                                ui.label(RichText::new("Its cables: the knob on each says how much; click the effect for how it answers").small().color(TEXT_DIM));
                            }
                            _ => close = true,
                        }
                    });
                });
                if let (Some(src), Node::Fx(e)) = (add_src, n) {
                    add_link = Some((src, None, e));
                }
                if close || ui.input(|i| i.key_pressed(Key::Escape)) {
                    selected = None;
                }
            }
        }

        // What was asked.
        let unlink: Option<usize> = ui.ctx().data_mut(|d| d.remove_temp(Id::new("video_patch_unlink")));
        if let Some(i) = unlink {
            if i < look.links.len() {
                look.links.remove(i);
            }
        }
        match remove {
            Some(Node::Fx(e)) => {
                look.fx.remove(&e);
                look.links.retain(|l| l.effect != e);
                selected = None;
            }
            Some(Node::Visuals) => {
                look.fx.remove(&Effect::Visuals);
                look.links.retain(|l| !l.effect.is_visual());
                for e in VIS_ORDER {
                    look.fx.remove(&e);
                }
                selected = None;
            }
            Some(Node::Mod(s, from)) => {
                look.links.retain(|l| !(l.source == s && l.from == from));
                selected = None;
            }
            _ => {}
        }
        if let Some((src, from, e)) = add_link {
            if !look.links.iter().any(|l| l.source == src && l.from == from && l.effect == e) && look.links.len() < kv::MAX_LINKS {
                let mut link = [VideoLink { from, ..VideoLink::new(src, e, 0.5) }];
                if from.is_none() {
                    self.video_fit_links(&mut link);
                }
                look.links.extend(link);
            }
            dragging = None;
        }
        if let Some(e) = add_fx {
            add_effect(look, e);
            selected = Some(if e == Effect::Visuals { Node::Visuals } else { Node::Fx(e) });
        }
        if go_clip {
            self.video.tab = VideoTab::Clip;
        }
        ui.ctx().data_mut(|d| {
            d.insert_temp(sel_id, selected);
            d.insert_temp(drag_id, dragging);
        });
    }
}

/// The name shown on the Film module.
fn set_clip_name(film: bool, set: &kv::VideoSet) -> String {
    if !film {
        return "no clip".into();
    }
    set.clips.get(set.current).map_or_else(|| "clip".to_string(), |c| if c.name.is_empty() { c.path.file_stem().map_or("clip".into(), |s| s.to_string_lossy().to_string()) } else { c.name.clone() })
}

fn body_font() -> egui::FontId {
    egui::FontId::proportional(13.0)
}

/// Effect `e` into the Look: the Visuals full, the rest at a fair amount (those of the film moved by the kick).
fn add_effect(look: &mut kv::VideoLook, e: Effect) {
    if look.effects().contains(&e) {
        return;
    }
    if e == Effect::Visuals {
        look.setting_mut(e).base = 1.0;
    } else if e.moves_film() {
        if look.links.len() < kv::MAX_LINKS {
            look.links.push(VideoLink::new(Source::Kick, e, 0.5));
        }
    } else {
        look.setting_mut(e).base = if e.is_visual() { 0.3 } else { 0.5 };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_effect_has_its_place_in_the_patch_once() {
        let all: Vec<Effect> = FILM_ORDER.iter().chain(VIS_ORDER.iter()).chain(MAIN_ORDER.iter()).copied().chain(std::iter::once(Effect::Visuals)).collect();
        assert_eq!(all.len(), Effect::COUNT);
        for e in Effect::ALL {
            assert_eq!(all.iter().filter(|x| **x == e).count(), 1, "{e:?}");
        }
    }

    #[test]
    fn the_modules_are_laid_out_in_their_order_and_the_modulators_under_them() {
        let used = [Effect::Burst, Effect::Zoom, Effect::Hue, Effect::Visuals];
        let mods = [(Node::Mod(Source::Kick, None), vec![Effect::Zoom])];
        let placed = layout(true, true, &used, &MAIN_ORDER, &mods, 900.0);
        let at = |n: Node| placed.iter().find(|(m, _)| *m == n).map(|(_, r)| *r).unwrap();
        assert!(at(Node::Visuals).right() < at(Node::Fx(Effect::Burst)).left(), "a Visuals' knob after the Visuals");
        assert!(at(Node::Fx(Effect::Zoom)).right() < at(Node::Fx(Effect::Hue)).left(), "Zoom before Hue, as the picture has them");
        assert!(at(Node::Fx(Effect::Hue)).right() < at(Node::Output).left());
        assert!(at(Node::Mix).top() > at(Node::Visuals).bottom(), "the mix after the sources, first of the picture's");
        assert!(at(Node::Mod(Source::Kick, None)).top() > at(Node::Fx(Effect::Zoom)).bottom(), "the kick under what it moves");
        assert!((at(Node::Mod(Source::Kick, None)).center().x - at(Node::Fx(Effect::Zoom)).center().x).abs() < 1.0);
        // A narrow patch goes on in rows.
        let many: Vec<Effect> = MAIN_ORDER[..10].to_vec();
        let placed = layout(false, false, &many, &MAIN_ORDER, &[], 400.0);
        let ys: std::collections::BTreeSet<i32> = placed.iter().map(|(_, r)| r.top() as i32).collect();
        assert!(ys.len() > 2, "rows: {ys:?}");
    }
}

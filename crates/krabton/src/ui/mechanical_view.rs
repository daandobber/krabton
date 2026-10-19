//! The mechanical sequencers' screens: the Organ Grinder's book (folded
//! cardboard punched with the holes it plays, sliding past the reading
//! bar) and the Music Box's cylinder (its pins turning towards the comb).
//! Both draw the bars the engine makes from the same parameters, so the
//! holes and pins are the notes to come.

use super::seq_view::Live;
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, Rect, Sense, Stroke, Ui};
use krabton_engine::sequencers::mechanical::{self as mech, Hole, Lane};

fn choice(params: &[f32], i: usize, n: usize) -> usize {
    (params[i].round().max(0.0) as usize).min(n - 1)
}

/// Where the engine is: (bar, tick in it), or the start when stopped.
fn now(live: &Live, tpb: u16) -> (i64, f32) {
    match live.on() {
        Some(v) => {
            let k = v[3] as i64;
            (k.div_euclid(tpb as i64), k.rem_euclid(tpb as i64) as f32)
        }
        None => (0, 0.0),
    }
}

/// The book: lanes for the drums, the bass, the accompaniment and the tune
/// (counter melody paler), a fold at every bar, the reading bar a quarter in.
pub(crate) fn organ(ui: &mut Ui, params: &[f32], live: Live, accent: Color32, height: f32) -> bool {
    let (rect, _) = ui.allocate_exact_size(vec2(300.0, height), Sense::hover());
    let p = ui.painter().clone();
    let card = Color32::from_rgb(0xd9, 0xc3, 0x93);
    let ink = Color32::from_rgb(0x3a, 0x2a, 0x18);
    rect_fill(&p, rect, 3.0, KNOB);
    let book = Rect::from_min_max(pos2(rect.left() + 6.0, rect.top() + 6.0), pos2(rect.right() - 6.0, rect.bottom() - 20.0));
    p.rect_filled(book, 2.0, card);
    let tpb = mech::organ_ticks_per_bar(params);
    let (bar, tick) = now(&live, tpb);
    // Three bars in view: a little of the one going, two to come.
    let px = book.width() / (tpb as f32 * 2.6);
    let read_x = book.left() + book.width() * 0.22;
    let x_of = |b: i64, t: f32| read_x + ((b - bar) as f32 * tpb as f32 + t - tick) * px;
    // The lanes, low to high: drums, bass, accompaniment, tune.
    let lanes = [(Lane::Drum, 0.0, 0.14), (Lane::Bass, 0.14, 0.32), (Lane::Accomp, 0.32, 0.55), (Lane::Melody, 0.55, 1.0)];
    let band = |lane: Lane| lanes.iter().find(|l| l.0 == if lane == Lane::Counter { Lane::Melody } else { lane }).map(|l| (l.1, l.2)).unwrap_or((0.0, 1.0));
    for (_, lo, _) in lanes.iter().skip(1) {
        let y = book.bottom() - lo * book.height();
        p.line_segment([pos2(book.left(), y), pos2(book.right(), y)], Stroke::new(1.0_f32, ink.gamma_multiply(0.25)));
    }
    let clip = p.with_clip_rect(book);
    let mut holes: Vec<Hole> = Vec::with_capacity(160);
    for b in bar..bar + 3 {
        let fx = x_of(b, 0.0);
        // The fold between pages.
        clip.line_segment([pos2(fx, book.top()), pos2(fx, book.bottom())], Stroke::new(1.0_f32, ink.gamma_multiply(0.4)));
        mech::organ_bar(params, b, &mut holes);
        for h in &holes {
            let (lo, hi) = band(h.lane);
            let (dlo, dhi) = match h.lane {
                Lane::Drum => (23.0, 30.0),
                Lane::Bass => (-5.0, 8.0),
                Lane::Accomp => (-1.0, 11.0),
                _ => (-8.0, 13.0),
            };
            let f = ((h.deg as f32 - dlo) / (dhi - dlo)).clamp(0.0, 1.0);
            let y = book.bottom() - (lo + (hi - lo) * (0.1 + 0.8 * f)) * book.height();
            let x0 = x_of(b, h.at as f32);
            let x1 = x_of(b, (h.at + h.len) as f32) - 1.0;
            let played = x0 <= read_x && live.on().is_some();
            let c = if played && x1 >= read_x { accent } else if h.lane == Lane::Counter { ink.gamma_multiply(0.5) } else { ink };
            clip.rect_filled(Rect::from_min_max(pos2(x0, y - 2.0), pos2(x1.max(x0 + 2.0), y + 2.0)), 2.0, c);
        }
    }
    // The reading bar.
    p.line_segment([pos2(read_x, book.top() - 2.0), pos2(read_x, book.bottom() + 2.0)], Stroke::new(2.0_f32, accent));
    let style = mech::ORGAN_STYLES[choice(params, mech::OG_STYLE, mech::ORGAN_STYLES.len())];
    let tune = params[mech::OG_TUNE].round() as i32;
    text(&p, pos2(rect.left() + 8.0, rect.bottom() - 4.0), Align2::LEFT_BOTTOM, &format!("{style} · book {tune}"), small(), TEXT_DIM);
    text(&p, pos2(rect.right() - 8.0, rect.bottom() - 4.0), Align2::RIGHT_BOTTOM, &format!("bar {}", bar.rem_euclid(32) + 1), small(), TEXT_DIM);
    if live.on().is_some() {
        ui.ctx().request_repaint();
    }
    false
}

/// The cylinder: its pins coming round towards the comb along the bottom,
/// a tooth for each note the comb has, lit as it is plucked.
pub(crate) fn music_box(ui: &mut Ui, params: &[f32], live: Live, accent: Color32, height: f32) -> bool {
    let (rect, _) = ui.allocate_exact_size(vec2(280.0, height), Sense::hover());
    let p = ui.painter().clone();
    rect_fill(&p, rect, 3.0, KNOB);
    let brass = Color32::from_rgb(0xc9, 0xa2, 0x4c);
    let steel = Color32::from_rgb(0xc8, 0xcc, 0xd2);
    let comb_h = 18.0;
    let cyl = Rect::from_min_max(pos2(rect.left() + 10.0, rect.top() + 8.0), pos2(rect.right() - 10.0, rect.bottom() - 22.0 - comb_h));
    // The cylinder, rounder at its edges.
    for i in 0..6 {
        let f = i as f32 / 6.0;
        let band = Rect::from_min_max(pos2(cyl.left(), cyl.top() + f * cyl.height()), pos2(cyl.right(), cyl.top() + (f + 1.0 / 6.0) * cyl.height()));
        let light = 1.0 - (f + 1.0 / 12.0 - 0.45).abs() * 1.4;
        p.rect_filled(band, 0.0, mix(Color32::from_rgb(0x5a, 0x45, 0x1c), brass, light.clamp(0.2, 1.0)));
    }
    let comb = choice(params, mech::MB_COMB, mech::BOX_COMBS.len());
    let (lo, hi) = [(-7, 10), (-11, 17), (-14, 21)][comb];
    let teeth = (hi - lo + 1) as usize;
    let tw = cyl.width() / teeth as f32;
    let tpb = mech::box_ticks_per_bar(params);
    let (bar, tick) = now(&live, tpb);
    // Two bars of pins above the comb, coming down.
    let py = cyl.height() / (tpb as f32 * 2.0);
    let mut holes: Vec<Hole> = Vec::with_capacity(160);
    let mut plucked = vec![0.0f32; teeth];
    let clip = p.with_clip_rect(cyl);
    for b in bar..bar + 3 {
        mech::box_bar(params, b, &mut holes);
        for h in &holes {
            let col = (h.deg - lo).clamp(0, teeth as i32 - 1) as usize;
            let dt = (b - bar) as f32 * tpb as f32 + h.at as f32 - tick;
            if live.on().is_some() && (-2.0..0.5).contains(&dt) {
                plucked[col] = plucked[col].max(1.0 - (-dt) / 2.0);
            }
            let y = cyl.bottom() - dt * py;
            let x = cyl.left() + (col as f32 + 0.5) * tw;
            let r = if h.lane == Lane::Melody { 2.2 } else { 1.7 };
            clip.circle_filled(pos2(x, y), r, if dt < 0.5 && live.on().is_some() { accent } else { Color32::from_rgb(0x2a, 0x22, 0x14) });
        }
    }
    // The comb: a tooth for every note, longer for the low ones.
    let top = cyl.bottom() + 2.0;
    for (i, &pl) in plucked.iter().enumerate() {
        let x = cyl.left() + (i as f32 + 0.5) * tw;
        let len = comb_h * (1.0 - 0.45 * i as f32 / teeth as f32);
        let c = mix(steel, accent, pl.clamp(0.0, 1.0));
        p.line_segment([pos2(x, top), pos2(x, top + len)], Stroke::new((tw * 0.55).clamp(1.0, 5.0), c));
    }
    let metre = mech::BOX_METRES[choice(params, mech::MB_METRE, mech::BOX_METRES.len())];
    let tune = params[mech::MB_TUNE].round() as i32;
    let wind = params[mech::MB_WIND];
    text(&p, pos2(rect.left() + 8.0, rect.bottom() - 4.0), Align2::LEFT_BOTTOM, &format!("{metre} · tune {tune} · {}", mech::BOX_COMBS[comb]), small(), TEXT_DIM);
    // How far the spring has run down this turn.
    if wind > 0.0 {
        let x = (bar.rem_euclid(16) as f32 + tick / tpb as f32) / 16.0;
        let spring = Rect::from_min_size(pos2(rect.right() - 58.0, rect.bottom() - 13.0), vec2(48.0, 6.0));
        p.rect_filled(spring, 3.0, PANEL3);
        p.rect_filled(Rect::from_min_size(spring.min, vec2(spring.width() * (1.0 - x * wind), spring.height())), 3.0, accent.gamma_multiply(0.8));
    }
    if live.on().is_some() {
        ui.ctx().request_repaint();
    }
    false
}

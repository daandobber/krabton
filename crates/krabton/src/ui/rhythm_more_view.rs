//! More rhythm generators' faces. Chance: the grid, every step filled as far
//! as its chance, its condition written on it and its ratchet as dots; a
//! skipped step lights red. Bits: the formula's byte step by step, each
//! drum's bit in its colour. Percussion: the players of the style with
//! what they play this bar, the lead's licks lit as it improvises.

use super::seq_view::Live;
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, Id, Rect, RichText, Sense, Stroke, Ui};
use krabton_engine::classic_seq::gm_drum_name;
use krabton_engine::sequencers::bits as bt;
use krabton_engine::sequencers::chance as ch;
use krabton_engine::sequencers::percussion as pc;

/// Mouse-wheel steps over a spot (20 points a step).
fn wheel_steps(ui: &Ui, id: Id) -> f32 {
    let scroll = ui.input(|i| i.smooth_scroll_delta.y);
    if scroll.abs() <= 0.5 {
        return 0.0;
    }
    let acc = ui.ctx().memory(|m| m.data.get_temp::<f32>(id).unwrap_or(0.0)) + scroll;
    let n = (acc / 20.0).trunc();
    ui.ctx().memory_mut(|m| m.data.insert_temp(id, acc - n * 20.0));
    n
}

// ---------------------------------------------------------------- chance

pub(crate) fn chance(ui: &mut Ui, id: Id, params: &mut [f32], data: &mut Vec<u8>, live: Live, accent: Color32, height: f32) -> bool {
    let mut changed = false;
    if !ch::is_current(data) {
        *data = ch::chance_default();
        changed = true;
    }
    let label_w = 78.0;
    let cw = 16.0;
    let foot = 22.0;
    let rh = ((height - foot - 2.0) / ch::LANES as f32).clamp(10.0, 20.0);
    let w = label_w + ch::STEPS as f32 * cw;
    let (rect, _) = ui.allocate_exact_size(vec2(w, height), Sense::hover());
    let p = ui.painter().clone();
    let vis = live.on();
    let grid = Rect::from_min_size(pos2(rect.left() + label_w, rect.top()), vec2(ch::STEPS as f32 * cw, rh * ch::LANES as f32));
    rect_fill(&p, grid, 3.0, KNOB);
    let tag_font = egui::FontId::proportional(8.5);
    for lane in 0..ch::LANES {
        let y = grid.top() + lane as f32 * rh;
        let len = ch::length(data, lane);
        let note = ch::note(data, lane);
        let pos = vis.map(|v| ((v[0] >> (4 * lane)) & 0xf) as usize);
        let fired = vis.is_some_and(|v| (v[0] >> (32 + lane)) & 1 == 1);
        let skipped = vis.is_some_and(|v| (v[0] >> (40 + lane)) & 1 == 1);
        // The lane: its drum (scroll) and its length (right-click).
        let lr = Rect::from_min_size(pos2(rect.left(), y), vec2(label_w - 4.0, rh));
        text(&p, pos2(lr.left() + 2.0, lr.center().y), Align2::LEFT_CENTER, gm_drum_name(note), small(), if fired { accent } else { TEXT });
        text(&p, pos2(lr.right(), lr.center().y), Align2::RIGHT_CENTER, format!("{len}"), small(), if len < ch::STEPS { accent } else { TEXT_DIM });
        let lresp = ui.interact(lr, id.with(("lane", lane)), Sense::click());
        if lresp.hovered() {
            let n = wheel_steps(ui, id.with(("scroll", lane)));
            if n != 0.0 {
                ch::set_note(data, lane, (note as f32 + n).clamp(0.0, 127.0) as u8);
                changed = true;
            }
        }
        lresp.context_menu(|ui| {
            ui.label(RichText::new("Steps in this lane").small().color(TEXT_DIM));
            egui::Grid::new(id.with(("lengths", lane))).show(ui, |ui| {
                for n in 1..=ch::STEPS {
                    if ui.selectable_label(n == len, format!("{n}")).clicked() {
                        ch::set_length(data, lane, n);
                        changed = true;
                        ui.close();
                    }
                    if n % 8 == 0 {
                        ui.end_row();
                    }
                }
            });
        });
        lresp.on_hover_text("Scroll: this lane's drum. Right-click: how many steps it has (lanes of their own lengths drift against each other)");
        for i in 0..ch::STEPS {
            let c = Rect::from_min_size(pos2(grid.left() + i as f32 * cw + 1.0, y + 1.0), vec2(cw - 2.0, rh - 2.0));
            let s = ch::step(data, lane, i);
            let active = i < len;
            let base = match s.level {
                2 => mix(accent, Color32::WHITE, 0.4),
                1 => accent,
                _ if i % 4 == 0 => shade(PANEL3, 0.8),
                _ => PANEL2,
            };
            let base = if active { base } else { shade(base, 0.3) };
            if s.level > 0 && s.chance < 100 {
                // The chance: how much of the step is filled.
                rect_fill(&p, c, 2.0, shade(base, 0.32));
                let h = c.height() * s.chance as f32 / 100.0;
                rect_fill(&p, Rect::from_min_max(pos2(c.left(), c.bottom() - h), c.max), 2.0, base);
            } else {
                rect_fill(&p, c, 2.0, base);
            }
            if s.level > 0 && s.condition > 0 {
                text(&p, c.center(), Align2::CENTER_CENTER, ch::CONDITION_TAGS[s.condition as usize], tag_font.clone(), TEXT_DARK);
            }
            for r in 1..s.ratchet {
                p.circle_filled(pos2(c.left() + 3.0 + (r - 1) as f32 * 3.5, c.top() + 2.5), 1.2, TEXT_DARK);
            }
            if pos == Some(i) {
                let col = if s.level == 0 {
                    LINE
                } else if fired {
                    Color32::WHITE
                } else if skipped {
                    REC
                } else {
                    TEXT_DIM
                };
                rect_line(&p, c, 2.0, 1.0, col);
            }
        }
    }
    let resp = ui.interact(grid, id.with("cells"), Sense::click_and_drag());
    let cell_at = |pt: egui::Pos2| -> Option<(usize, usize)> {
        let i = ((pt.x - grid.left()) / cw).floor();
        let lane = ((pt.y - grid.top()) / rh).floor();
        ((0.0..ch::STEPS as f32).contains(&i) && (0.0..ch::LANES as f32).contains(&lane)).then_some((lane as usize, i as usize))
    };
    let primary = ui.input(|inp| inp.pointer.primary_down());
    if let Some((lane, i)) = resp.interact_pointer_pos().filter(|_| resp.is_pointer_button_down_on() && primary).and_then(cell_at) {
        // Press cycles off > hit > accent; dragging paints that value.
        let paint_id = id.with("paint");
        if ui.input(|inp| inp.pointer.primary_pressed()) {
            let next = (ch::step(data, lane, i).level + 1) % 3;
            ui.ctx().memory_mut(|m| m.data.insert_temp(paint_id, next));
        }
        if let Some(level) = ui.ctx().memory(|m| m.data.get_temp::<u8>(paint_id)) {
            let s = ch::step(data, lane, i);
            if s.level != level {
                ch::set_step(data, lane, i, ch::Step { level, ..s });
                changed = true;
            }
        }
    }
    if let Some((lane, i)) = resp.hover_pos().and_then(cell_at) {
        let n = wheel_steps(ui, id.with("chance_scroll"));
        if n != 0.0 {
            let s = ch::step(data, lane, i);
            ch::set_step(data, lane, i, ch::Step { chance: (s.chance as f32 + n * 10.0).clamp(0.0, 100.0) as u8, ..s });
            changed = true;
        }
        if resp.secondary_clicked() {
            ui.ctx().memory_mut(|m| m.data.insert_temp(id.with("menu_cell"), (lane, i)));
        }
    }
    let menu_cell = ui.ctx().memory(|m| m.data.get_temp::<(usize, usize)>(id.with("menu_cell")));
    resp.context_menu(|ui| {
        let Some((lane, i)) = menu_cell else { return };
        let mut s = ch::step(data, lane, i);
        let before = s;
        ui.label(RichText::new(format!("{} · step {}", gm_drum_name(ch::note(data, lane)), i + 1)).small().color(TEXT_DIM));
        ui.horizontal(|ui| {
            for (l, name) in ["Off", "Hit", "Accent"].iter().enumerate() {
                if ui.selectable_label(s.level == l as u8, *name).clicked() {
                    s.level = l as u8;
                }
            }
        });
        let mut chance = s.chance as f32;
        if ui.add(egui::Slider::new(&mut chance, 0.0..=100.0).step_by(5.0).suffix(" %").text("Chance")).changed() {
            s.chance = chance as u8;
        }
        ui.horizontal(|ui| {
            ui.label("Ratchet");
            for r in 1..=4u8 {
                if ui.selectable_label(s.ratchet == r, format!("{r}")).clicked() {
                    s.ratchet = r;
                }
            }
        });
        ui.separator();
        ui.label(RichText::new("Plays").small().color(TEXT_DIM));
        egui::Grid::new(id.with("conditions")).show(ui, |ui| {
            for (c, name) in ch::CONDITIONS.iter().enumerate() {
                if ui.selectable_label(s.condition == c as u8, *name).on_hover_text(condition_help(c)).clicked() {
                    s.condition = c as u8;
                }
                if c == 0 || c % 4 == 1 {
                    ui.end_row();
                }
            }
        });
        if s != before {
            // Setting a chance or condition on an empty step makes it a hit.
            if s.level == 0 && before.level == 0 {
                s.level = 1;
            }
            ch::set_step(data, lane, i, s);
            changed = true;
        }
    });
    resp.on_hover_text("Click: off > hit > accent (drag to paint). Scroll: the step's chance. Right-click: chance, condition and ratchet");
    // The footer: patterns and the fill.
    let f = Rect::from_min_size(pos2(rect.left(), rect.bottom() - foot + 2.0), vec2(w, foot - 4.0));
    let pat = button(ui, Rect::from_min_size(f.min, vec2(80.0, f.height())), id.with("pat"), "Patterns", false, accent);
    let mut picked = None;
    egui::Popup::menu(&pat).show(|ui| {
        for name in ch::PATTERNS {
            if ui.button(*name).clicked() {
                picked = Some(*name);
                ui.close();
            }
        }
    });
    if let Some(name) = picked {
        *data = ch::pattern(name);
        changed = true;
    }
    let fill_now = vis.is_some_and(|v| (v[0] >> 48) & 1 == 1);
    let fill_on = params[ch::FILL] >= 0.5;
    if button(ui, Rect::from_min_size(f.min + vec2(86.0, 0.0), vec2(56.0, f.height())), id.with("fill"), "Fill", fill_on || fill_now, accent).on_hover_text("Play the fill now (steps on Fill play, Not Fill rest); lit while a fill plays").clicked() {
        params[ch::FILL] = if fill_on { 0.0 } else { 1.0 };
        changed = true;
    }
    changed
}

fn condition_help(c: usize) -> &'static str {
    match c {
        0 => "Every time round",
        1..=9 => "A:B: the A-th of every B times round (1:2 the first of every two, 4:4 the fourth of every four)",
        10 => "Only in a fill",
        11 => "Only when it is not a fill",
        12 => "Only the first time round after you press play",
        13 => "Every time but the first",
        14 => "If the last condition in this lane came true",
        15 => "If the last condition in this lane did not",
        16 => "If the last condition in the lane above came true",
        _ => "If the last condition in the lane above did not",
    }
}

// ---------------------------------------------------------------- bits

pub(crate) fn bits(ui: &mut Ui, id: Id, params: &mut [f32], live: Live, accent: Color32, height: f32) -> bool {
    let mut changed = false;
    let cols = 32usize;
    let cw = 9.0;
    let label_w = 64.0;
    let top_h = 18.0;
    let foot = 22.0;
    let rh = ((height - top_h - foot - 2.0) / 8.0).clamp(8.0, 16.0);
    let w = cols as f32 * cw + label_w;
    let (rect, _) = ui.allocate_exact_size(vec2(w, height), Sense::hover());
    let p = ui.painter().clone();
    let colors = [ACCENT, SELECT, PLAY, Color32::from_rgb(0xe0, 0x84, 0xff)];
    let f = (params[bt::FORMULA].round().max(0.0) as usize).min(bt::FORMULAS.len() - 1);
    let (a, b) = (params[bt::A].round() as u32, params[bt::B].round() as u32);
    let formula = bt::FORMULAS[f].replace('a', &a.to_string()).replace('b', &b.to_string());
    text(&p, rect.left_top() + vec2(2.0, 1.0), Align2::LEFT_TOP, formula, egui::FontId::monospace(11.0), accent);
    let vis = live.on();
    let rate = krabton_engine::sequencers::rate_beats(params[bt::RATE]);
    let per_bar = (4.0 / rate).round() as i64;
    // The window of 32 steps the counter is in.
    let t_now = vis.map(|v| v[0] as u32);
    let start = match t_now {
        Some(t) => t - t % cols as u32,
        None => bt::counter(0, params, per_bar),
    };
    let grid = Rect::from_min_size(pos2(rect.left(), rect.top() + top_h), vec2(cols as f32 * cw, 8.0 * rh));
    rect_fill(&p, grid, 3.0, KNOB);
    let bit_of = |lane: usize| params[bt::BIT + lane].round().clamp(0.0, 7.0) as usize;
    for c in 0..cols {
        let t = start.wrapping_add(c as u32);
        let byte = bt::byte(f, t, a, b);
        let x = grid.left() + c as f32 * cw;
        if t_now == Some(t) {
            p.rect_filled(Rect::from_min_size(pos2(x, grid.top()), vec2(cw, grid.height())), 0.0, PANEL3);
        }
        for bit in 0..8usize {
            let y = grid.bottom() - (bit + 1) as f32 * rh;
            let on = byte >> bit & 1 == 1;
            let lane = (0..bt::LANES).find(|&l| bit_of(l) == bit);
            let cell = Rect::from_min_size(pos2(x + 1.0, y + 1.0), vec2(cw - 2.0, rh - 2.0));
            let col = match (on, lane) {
                (true, Some(l)) if bt::hit(params, t, l).is_some() => colors[l],
                (true, Some(l)) => shade(colors[l], 0.45),
                (true, None) => shade(TEXT_DIM, 0.6),
                (false, Some(l)) => shade(colors[l], 0.12),
                (false, None) => PANEL2,
            };
            p.rect_filled(cell, 1.0, col);
        }
        if c % 4 == 0 {
            p.line_segment([pos2(x, grid.top()), pos2(x, grid.bottom())], Stroke::new(1.0_f32, if c % 16 == 0 { LINE } else { LINE_SOFT }));
        }
    }
    // Which drum a click gives a row of bits.
    let sel_id = id.with("lane");
    let mut sel = ui.ctx().memory(|m| m.data.get_temp::<usize>(sel_id).unwrap_or(0)).min(bt::LANES - 1);
    for bit in 0..8usize {
        let y = grid.bottom() - (bit + 1) as f32 * rh + rh * 0.5;
        let lanes: Vec<usize> = (0..bt::LANES).filter(|&l| bit_of(l) == bit).collect();
        let label = lanes.iter().map(|&l| gm_drum_name(params[bt::NOTE + l].round().clamp(0.0, 127.0) as u8)).collect::<Vec<_>>().join(" ");
        let col = lanes.first().map(|&l| colors[l]).unwrap_or(TEXT_DIM);
        text(&p, pos2(grid.right() + 4.0, y), Align2::LEFT_CENTER, if label.is_empty() { format!("bit {bit}") } else { label }, small(), col);
    }
    let resp = ui.interact(grid, id.with("grid"), Sense::click());
    if let Some(pt) = resp.interact_pointer_pos().filter(|_| resp.clicked()) {
        let bit = ((grid.bottom() - pt.y) / rh).floor().clamp(0.0, 7.0);
        if params[bt::BIT + sel] != bit {
            params[bt::BIT + sel] = bit;
            changed = true;
        }
    }
    resp.on_hover_text("The formula's byte, step by step (bit 7 on top). Each drum plays where its bit is on (Level) or turns on (Edge). Click a row to give it to the selected drum");
    let f_rect = Rect::from_min_size(pos2(rect.left(), rect.bottom() - foot + 2.0), vec2(w, foot - 4.0));
    for l in 0..bt::LANES {
        let r = Rect::from_min_size(f_rect.min + vec2(l as f32 * 64.0, 0.0), vec2(60.0, f_rect.height()));
        let name = gm_drum_name(params[bt::NOTE + l].round().clamp(0.0, 127.0) as u8);
        let fired = vis.is_some_and(|v| (v[0] >> (32 + l)) & 1 == 1);
        if button(ui, r, id.with(("sel", l)), name, sel == l || fired, colors[l]).on_hover_text("Select this drum, then click a row of bits for it").clicked() {
            sel = l;
        }
    }
    ui.ctx().memory_mut(|m| m.data.insert_temp(sel_id, sel));
    changed
}

// ---------------------------------------------------------------- percussion

pub(crate) fn percussion(ui: &mut Ui, id: Id, params: &mut [f32], live: Live, accent: Color32, height: f32) -> bool {
    let mut changed = false;
    let st = pc::style(params);
    let label_w = 96.0;
    let cw = if st.steps == 12 { 20.0 } else { 15.0 };
    let rows = st.parts.len() + 1;
    let top_h = 18.0;
    let rh = ((height - top_h - 2.0) / rows as f32).clamp(10.0, 18.0);
    let w = label_w + st.steps as f32 * cw;
    let (rect, _) = ui.allocate_exact_size(vec2(w, height), Sense::hover());
    let p = ui.painter().clone();
    let vis = live.on();
    let step = vis.map(|v| (v[0] & 0xff) as usize);
    let bar = vis.map(|v| ((v[0] >> 16) & 0xff) as usize).unwrap_or(0);
    let fired = vis.map(|v| v[1] & 0xff).unwrap_or(0);
    let improvising = vis.is_some_and(|v| (v[1] >> 8) & 1 == 1);
    let calling = vis.is_some_and(|v| (v[1] >> 9) & 1 == 1);
    let playing = pc::parts_playing(st, params[pc::DENSITY]);
    text(&p, rect.left_top() + vec2(2.0, 1.0), Align2::LEFT_TOP, st.name, small(), accent);
    if st.steps == 12 {
        text(&p, rect.right_top() + vec2(-2.0, 1.0), Align2::RIGHT_TOP, "6/8", small(), TEXT_DIM);
    }
    let grid = Rect::from_min_size(pos2(rect.left() + label_w, rect.top() + top_h), vec2(st.steps as f32 * cw, rh * rows as f32));
    rect_fill(&p, grid, 3.0, KNOB);
    for row in 0..rows {
        let y = grid.top() + row as f32 * rh;
        let is_lead = row == st.parts.len();
        let (inst, pat) = if is_lead { (st.lead.0, st.lead.1) } else { st.parts[row] };
        let on = if is_lead { params[pc::LEAD] >= 0.5 } else { row < playing && !(improvising && st.lead.2 == Some(row)) };
        let name = match (is_lead, calling) {
            (true, true) => "Lead · call".to_string(),
            (true, false) => format!("Lead · {}", pc::INSTRUMENTS[inst].name),
            _ => pc::INSTRUMENTS[inst].name.to_string(),
        };
        let hot = fired >> row.min(7) & 1 == 1;
        text(&p, pos2(rect.left() + 2.0, y + rh * 0.5), Align2::LEFT_CENTER, name, small(), if hot { accent } else if on { TEXT } else { TEXT_DIM });
        let lick = vis.map(|v| v[2]).filter(|_| is_lead && improvising);
        for i in 0..st.steps {
            let c = Rect::from_min_size(pos2(grid.left() + i as f32 * cw + 1.0, y + 1.0), vec2(cw - 2.0, rh - 2.0));
            let sym = if is_lead { pc::symbol(pat, i) } else { pc::symbol(pat, (bar % 2) * st.steps + i) };
            let level = match lick {
                Some(m) => (m >> i & 1 == 1) as u8 as f32 * 0.9,
                None => match sym {
                    'x' | 's' => 1.0,
                    'o' | 'm' | 't' => 0.7,
                    '-' => 0.35,
                    _ => 0.0,
                },
            };
            let beat = if st.steps == 12 { i % 3 == 0 } else { i % 4 == 0 };
            let base = if beat { shade(PANEL3, 0.8) } else { PANEL2 };
            let col = if level > 0.0 {
                let c = match sym {
                    _ if lick.is_some() => accent,
                    'm' | 's' => SELECT,
                    't' => PLAY,
                    _ => accent,
                };
                mix(base, c, if on { level } else { level * 0.3 })
            } else {
                base
            };
            rect_fill(&p, c, 2.0, col);
            if step == Some(i) {
                rect_line(&p, c, 2.0, 1.0, if level > 0.0 && on { Color32::WHITE } else { LINE });
            }
        }
        // Click a player to bring the ensemble up to them.
        let lr = Rect::from_min_size(pos2(rect.left(), y), vec2(label_w - 2.0, rh));
        let resp = ui.interact(lr, id.with(("row", row)), Sense::click());
        if resp.clicked() {
            if is_lead {
                params[pc::LEAD] = if params[pc::LEAD] >= 0.5 { 0.0 } else { 1.0 };
            } else {
                params[pc::DENSITY] = if st.parts.len() > 1 { row as f32 / (st.parts.len() - 1) as f32 } else { 1.0 };
            }
            changed = true;
        }
        resp.on_hover_text(if is_lead { "The lead improvises as Variation says (a call every fourth bar). Click: lead on or off" } else { "Click: the ensemble up to this player (Density)" });
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::Pos2;
    use krabton_engine::params::defaults;

    #[test]
    fn the_screens_draw() {
        let ctx = egui::Context::default();
        let live = Live { vis: Some([0x0001_0f0f_0123_4567, 0x1ff, 0x0f0f, 0]), playing: true, scale: krabton_engine::Scale::new(0, krabton_engine::ScaleKind::Major), kit: 0 };
        let mut chance_p = defaults(ch::SPECS);
        let mut chance_d = Vec::new();
        let mut bits_p = defaults(bt::SPECS);
        let mut perc_p = defaults(pc::SPECS);
        for style in 0..pc::STYLES.len() {
            perc_p[pc::STYLE] = style as f32;
            let _ = ctx.run(egui::RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1400.0, 800.0))), ..Default::default() }, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    chance(ui, Id::new("c"), &mut chance_p, &mut chance_d, live, ACCENT, 200.0);
                    bits(ui, Id::new("d"), &mut bits_p, live, ACCENT, 200.0);
                    percussion(ui, Id::new("e"), &mut perc_p, live, ACCENT, 200.0);
                });
            });
        }
        assert!(ch::is_current(&chance_d));
    }
}

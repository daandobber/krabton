//! The command palette (Ctrl+P): type a few letters of anything Krabton can do
//! or add, Enter does it. Letters match in order, not necessarily together, so
//! "mnc" finds Munich; a match at the start of a word counts for more.

use crate::app::{App, BrowserItem, DropTarget, TrackSel};
use crate::theme::*;
use egui::{Context, Key, RichText};
use krabton_engine::dsp::effects::EffectKind;
use krabton_engine::midi_fx::MidiFxKind;

#[derive(Clone)]
enum Run {
    Do(fn(&mut App)),
    Add(BrowserItem),
}

#[derive(Clone)]
struct Entry {
    what: &'static str,
    name: String,
    run: Run,
}

/// What the palette remembers between frames.
#[derive(Default)]
pub struct Palette {
    pub open: bool,
    query: String,
    at: usize,
    list: Vec<Entry>,
    focused: bool,
}

/// How well `query` fits `name` (letters in order), or nothing.
pub fn score(query: &str, name: &str) -> Option<i32> {
    let (q, n) = (query.to_lowercase(), name.to_lowercase());
    let chars: Vec<char> = n.chars().collect();
    let mut at = 0;
    let mut total = 0;
    let mut last: Option<usize> = None;
    for c in q.chars().filter(|c| !c.is_whitespace()) {
        let k = (at..chars.len()).find(|&i| chars[i] == c)?;
        // At the start of a word, right after the last letter, early in the name.
        if k == 0 || !chars[k - 1].is_alphanumeric() {
            total += 10;
        }
        if last.is_some_and(|l| l + 1 == k) {
            total += 6;
        }
        total -= (k as i32).min(8) / 4;
        last = Some(k);
        at = k + 1;
    }
    // Shorter names first, among equals.
    Some(total * 4 - chars.len() as i32 / 8)
}

fn actions() -> Vec<Entry> {
    fn e(name: &str, f: fn(&mut App)) -> Entry {
        Entry { what: "Do", name: name.to_string(), run: Run::Do(f) }
    }
    vec![
        e("Save", |a| a.save(false)),
        e("Save As", |a| a.save(true)),
        e("Group Tracks", |a| a.group_selected()),
        e("Ungroup Tracks", |a| a.ungroup_selected()),
        e("Save as Default Set", |a| a.save_default_set()),
        e("New Set", |a| a.new_project()),
        e("Open Set", |a| a.open()),
        e("Undo", |a| a.undo()),
        e("Redo", |a| a.redo()),
        e("Play or Stop", |a| a.toggle_play()),
        e("Export Audio", |a| a.export.open = true),
        e("Import Audio", |a| a.import_audio_dialog()),
        e("Import MIDI File", |a| a.import_midi_dialog()),
        e("Packs", |a| a.packs.open = true),
        e("Check for Updates", |a| {
            if a.update.found().is_none() {
                a.update.check_now();
            }
            a.update.open = true;
        }),
        e("Audio & MIDI Settings", |a| a.show_settings = true),
        e("MIDI Learn", |a| a.toggle_midi_learn()),
        e("Video", |a| a.open_video()),
        e("Map", |a| a.map_window = true),
        e("Load Demo Set", |a| a.load_demo()),
        e("Load Gallop Set", |a| a.load_gallop()),
        e("Load Ambient Set", |a| a.load_ambient()),
    ]
}

fn everything() -> Vec<Entry> {
    let mut v = actions();
    for k in EffectKind::ALL {
        v.push(Entry { what: "Effect", name: k.name().to_string(), run: Run::Add(BrowserItem::Effect(k)) });
    }
    for k in MidiFxKind::SEQUENCERS.into_iter().chain(MidiFxKind::ALL) {
        if v.iter().any(|e| e.name == k.name()) {
            continue;
        }
        let what = if k.is_sequencer() { "Sequencer" } else { "MIDI Effect" };
        v.push(Entry { what, name: k.name().to_string(), run: Run::Add(BrowserItem::MidiFx(k)) });
    }
    for (name, item) in crate::ui::browser::first_presets() {
        v.push(Entry { what: "Instrument", name: name.to_string(), run: Run::Add(item) });
    }
    v
}

impl App {
    pub(crate) fn ui_palette(&mut self, ctx: &Context) {
        if !self.palette.open {
            return;
        }
        if self.palette.list.is_empty() {
            self.palette.list = everything();
        }
        let query = self.palette.query.clone();
        let mut hits: Vec<(i32, usize)> = if query.trim().is_empty() {
            (0..self.palette.list.len().min(14)).map(|i| (0, i)).collect()
        } else {
            self.palette.list.iter().enumerate().filter_map(|(i, e)| score(&query, &e.name).map(|s| (s, i))).collect()
        };
        hits.sort_by_key(|h| -h.0);
        hits.truncate(14);
        let (down, up, enter, esc) = ctx.input(|i| (i.key_pressed(Key::ArrowDown), i.key_pressed(Key::ArrowUp), i.key_pressed(Key::Enter), i.key_pressed(Key::Escape)));
        if down {
            self.palette.at = (self.palette.at + 1).min(hits.len().saturating_sub(1));
        }
        if up {
            self.palette.at = self.palette.at.saturating_sub(1);
        }
        self.palette.at = self.palette.at.min(hits.len().saturating_sub(1));
        let mut chosen = enter.then(|| hits.get(self.palette.at).map(|h| h.1)).flatten();
        let screen = ctx.screen_rect();
        egui::Window::new("Palette")
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .fixed_size(egui::vec2(460.0, 0.0))
            .fixed_pos(egui::pos2(screen.center().x - 230.0, screen.top() + 70.0))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                let edit = ui.add(egui::TextEdit::singleline(&mut self.palette.query).desired_width(f32::INFINITY).hint_text("Type anything: Save, Reverb, Munich, Inferno...").frame(false));
                if !self.palette.focused {
                    edit.request_focus();
                    self.palette.focused = true;
                }
                if edit.changed() {
                    self.palette.at = 0;
                }
                ui.separator();
                if hits.is_empty() {
                    ui.label(RichText::new("Nothing by that name").color(TEXT_DIM));
                }
                for (row, (_, i)) in hits.iter().enumerate() {
                    let e = &self.palette.list[*i];
                    let on = row == self.palette.at;
                    let resp = ui
                        .horizontal(|ui| {
                            let r = ui.selectable_label(on, RichText::new(&e.name));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| ui.label(RichText::new(e.what).small().color(TEXT_DIM)));
                            r
                        })
                        .inner;
                    if resp.clicked() {
                        chosen = Some(*i);
                    }
                }
            });
        if esc {
            self.close_palette();
        }
        if let Some(i) = chosen {
            let run = self.palette.list[i].run.clone();
            self.close_palette();
            match run {
                Run::Do(f) => f(self),
                Run::Add(item) => {
                    // On the selected track when it fits there, else on a new one.
                    let target = match (&item, self.sel_track) {
                        (BrowserItem::Effect(_), sel) => DropTarget::Chain(sel),
                        (BrowserItem::MidiFx(_), TrackSel::Track(t)) => DropTarget::Track(t),
                        _ => DropTarget::NewTrack,
                    };
                    self.apply_item(&item, target);
                }
            }
        }
    }

    pub(crate) fn toggle_palette(&mut self) {
        if self.palette.open {
            self.close_palette();
        } else {
            self.palette.open = true;
            self.palette.focused = false;
            self.palette.query.clear();
            self.palette.at = 0;
        }
    }

    fn close_palette(&mut self) {
        self.palette.open = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn letters_in_order_find_a_name_and_word_starts_count() {
        assert!(score("mnc", "Munich").is_some());
        assert!(score("xyz", "Munich").is_none());
        assert!(score("cnm", "Munich").is_none());
        // The start of a word beats the same letters inside one.
        assert!(score("s", "Save").unwrap() > score("s", "Bass Synth").unwrap());
        assert!(score("mi", "MIDI Learn").unwrap() > score("mi", "Ambient Mix").unwrap());
        assert_eq!(score("", "Anything").map(|_| ()), Some(()));
    }

    #[test]
    fn everything_is_on_the_list() {
        let all = everything();
        for name in ["Save", "MIDI Learn", "Tune", "Inferno", "Munich"] {
            assert!(all.iter().any(|e| e.name == name), "{name}");
        }
    }
}

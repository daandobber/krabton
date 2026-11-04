//! Mastering beside the Map: how loud the song comes out, how loud it should
//! be, and a click to get there. It works on the master's own chain: a gentle
//! compressor and the Lens Limiter (whose loudness meter it reads), which stay
//! there to change by hand like any other device.

use crate::app::App;
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, Id, Rect, RichText, Sense, Stroke, Ui};
use krabton_engine::dsp::effects::EffectKind;
use krabton_engine::dsp::lens_limiter as lim;

/// How loud a song should end up, and how the limiter gets it there.
pub(crate) struct Target {
    pub name: &'static str,
    pub lufs: f32,
    pub ceiling: f32,
    /// See `lens_limiter::STYLES`.
    pub style: f32,
    pub about: &'static str,
}

pub(crate) const TARGETS: [Target; 6] = [
    Target { name: "Streaming", lufs: -14.0, ceiling: -1.0, style: 0.0, about: "Where the streaming services turn a song to: louder gains nothing there" },
    Target { name: "Soft", lufs: -16.0, ceiling: -1.0, style: 6.0, about: "Quiet and open, for songs that breathe" },
    Target { name: "Balanced", lufs: -12.0, ceiling: -1.0, style: 4.0, about: "A bit louder, still with its dynamics" },
    Target { name: "Loud", lufs: -9.0, ceiling: -0.5, style: 7.0, about: "As loud as most pop and hip-hop" },
    Target { name: "Club", lufs: -7.5, ceiling: -0.3, style: 1.0, about: "For the club: loud, and the kick keeps its punch" },
    Target { name: "Deep Fried", lufs: -5.0, ceiling: -0.1, style: 3.0, about: "Clipped to the ceiling on purpose" },
];

/// The limiter's gain that brings a song measured at `measured` to `target`.
pub(crate) fn gain_for(gain_now: f32, measured: f32, target: f32) -> f32 {
    (gain_now + target - measured).clamp(0.0, 30.0)
}

impl App {
    /// The master's last Lens Limiter: where its chain ends up.
    fn master_limiter(&self) -> Option<usize> {
        self.project.master.devices.iter().rposition(|d| d.kind == EffectKind::LensLimiter)
    }

    /// Puts a gentle compressor and a limiter on the master (what is not there
    /// yet), set to the target.
    pub(crate) fn add_mastering(&mut self) {
        let has = |app: &App, kind| app.project.master.devices.iter().any(|d| d.kind == kind);
        let at = self.master_limiter().unwrap_or(self.project.master.devices.len());
        if !has(self, EffectKind::LensComp) {
            let mut comp = self.project.make_device(EffectKind::LensComp);
            if let Some(p) = krabton_engine::presets::effect(EffectKind::LensComp).iter().find(|p| p.name == "Mastering") {
                comp.params = krabton_engine::presets::apply(EffectKind::LensComp.specs(), p);
            }
            self.project.master.devices.insert(at, comp);
        }
        if !has(self, EffectKind::LensLimiter) {
            let limiter = self.project.make_device(EffectKind::LensLimiter);
            self.project.master.devices.push(limiter);
        }
        self.set_master_target(self.project.master.target as usize);
        self.toast("Mastering on the master: play the song, then click Reach target");
    }

    /// A target chosen: the limiter takes its style and ceiling.
    pub(crate) fn set_master_target(&mut self, k: usize) {
        let k = k.min(TARGETS.len() - 1);
        self.project.master.target = k as u8;
        if let Some(i) = self.master_limiter() {
            let p = &mut self.project.master.devices[i].params;
            p[lim::STYLE] = TARGETS[k].style;
            p[lim::CEILING] = TARGETS[k].ceiling;
        }
    }

    /// What the master's limiter measures now.
    fn master_meters(&self) -> Option<[f32; krabton_engine::scope::METER_VALUES]> {
        let dev = self.project.master.devices.get(self.master_limiter()?)?;
        dev.enabled.then(|| self.status().meters.read(dev.id)).flatten()
    }

    /// Turns the limiter's gain so that the song comes out at the target.
    pub(crate) fn reach_master_target(&mut self) {
        let target = TARGETS[(self.project.master.target as usize).min(TARGETS.len() - 1)].lufs;
        let measured = self.master_meters().map(|m| m[lim::M_INTEGRATED]).filter(|&v| v > -70.0);
        let (Some(i), Some(measured)) = (self.master_limiter(), measured) else {
            self.toast("Play the song first: it measures while it plays");
            return;
        };
        let p = &mut self.project.master.devices[i].params;
        let gain = gain_for(p[lim::GAIN], measured, target);
        let short = gain_now_short(p[lim::GAIN], measured, target);
        p[lim::GAIN] = (gain * 10.0).round() / 10.0;
        // Measured anew from here.
        p[lim::RESET] += 1.0;
        let gain = p[lim::GAIN];
        self.toast(if short { format!("Limiter at 0 dB and still too loud for {target} LUFS: turn the tracks down") } else { format!("Limiter gain {gain:.1} dB: play on, it measures again") });
    }

    /// Hears the song without the master's chain, or with it again.
    fn toggle_master_before(&mut self) {
        if self.master_before.is_empty() {
            self.master_before = self.project.master.devices.iter().filter(|d| d.enabled).map(|d| d.id).collect();
            for d in self.project.master.devices.iter_mut() {
                d.enabled = false;
            }
        } else {
            for d in self.project.master.devices.iter_mut() {
                d.enabled = d.enabled || self.master_before.contains(&d.id);
            }
            self.master_before.clear();
        }
    }

    /// The master's strip beside the Map's mixer.
    pub(crate) fn map_master(&mut self, ui: &mut Ui, h: f32) {
        egui::Frame::new().fill(PANEL2).corner_radius(4.0).inner_margin(6).show(ui, |ui| {
            ui.set_height(h - 14.0);
            ui.horizontal_top(|ui| {
                self.map_master_fader(ui, h - 14.0);
                ui.vertical(|ui| self.map_master_loudness(ui));
            });
        });
    }

    fn map_master_fader(&mut self, ui: &mut Ui, h: f32) {
        ui.vertical(|ui| {
            ui.set_width(44.0);
            ui.label(RichText::new("Master").small().color(TEXT));
            let (r, _) = ui.allocate_exact_size(vec2(44.0, (h - 40.0).max(40.0)), Sense::hover());
            let f = Rect::from_min_max(r.min, pos2(r.left() + 26.0, r.bottom()));
            let mut db = self.project.master.volume_db;
            if fader(ui, f, Id::new("map_master_fader"), &mut db, accent_now()) {
                self.project.master.volume_db = db;
            }
            let m = Rect::from_min_max(pos2(r.right() - 12.0, r.top()), r.max);
            rect_fill(ui.painter(), m, 2.0, BG);
            for (k, v) in [self.master_meter.0, self.master_meter.1].iter().enumerate() {
                let level = 20.0 * v.max(1e-6).log10();
                let frac = ((level + 60.0) / 66.0).clamp(0.0, 1.0);
                let x = m.left() + k as f32 * 6.0;
                let col = Rect::from_min_max(pos2(x, m.bottom() - frac * m.height()), pos2(x + 5.0, m.bottom()));
                rect_fill(ui.painter(), col, 1.0, if level > -3.0 { REC } else { Color32::from_gray(190) });
            }
            ui.label(RichText::new(format!("{:+.1}", self.project.master.volume_db)).small().color(TEXT_DIM));
        });
    }

    fn map_master_loudness(&mut self, ui: &mut Ui) {
        let k = (self.project.master.target as usize).min(TARGETS.len() - 1);
        let target = &TARGETS[k];
        ui.horizontal(|ui| {
            ui.label(RichText::new("Target").small().color(TEXT_DIM));
            let mut choice = k;
            egui::ComboBox::from_id_salt("map_master_target").width(92.0).selected_text(target.name).show_ui(ui, |ui| {
                for (j, t) in TARGETS.iter().enumerate() {
                    ui.selectable_value(&mut choice, j, t.name).on_hover_text(t.about);
                }
            });
            ui.label(RichText::new(format!("{:.1} LUFS", target.lufs)).small().color(TEXT_DIM));
            if choice != k {
                self.set_master_target(choice);
            }
        });
        let meters = self.master_meters();
        let short = meters.map(|m| m[lim::M_SHORT]).filter(|&v| v > -70.0);
        // How loud now, against the target.
        let (bar, _) = ui.allocate_exact_size(vec2(ui.available_width().min(210.0), 12.0), Sense::hover());
        rect_fill(ui.painter(), bar, 2.0, BG);
        let x_of = |lufs: f32| bar.left() + ((lufs + 30.0) / 30.0).clamp(0.0, 1.0) * bar.width();
        if let Some(v) = short {
            let off = (v - TARGETS[k].lufs).abs();
            let colour = if off < 1.0 { PLAY } else if v > TARGETS[k].lufs { REC } else { Color32::from_gray(190) };
            rect_fill(ui.painter(), Rect::from_min_max(bar.min, pos2(x_of(v), bar.bottom())), 2.0, colour);
        }
        let tx = x_of(TARGETS[k].lufs);
        ui.painter().line_segment([pos2(tx, bar.top() - 2.0), pos2(tx, bar.bottom() + 2.0)], Stroke::new(2.0_f32, accent_now()));
        ui.painter().text(bar.right_center() + vec2(-4.0, 0.0), Align2::RIGHT_CENTER, short.map_or("no reading".to_string(), |v| format!("{v:.1}")), small(), TEXT);
        match meters {
            Some(m) => {
                let show = |v: f32| if v <= -70.0 { "-".to_string() } else { format!("{v:.1}") };
                ui.label(RichText::new(format!("Song {} LUFS · Peak {} dB · Limiting {:.1} dB", show(m[lim::M_INTEGRATED]), show(m[lim::M_TRUE_PEAK]), m[lim::M_GR].abs())).small().color(TEXT_DIM));
            }
            None if !self.master_before.is_empty() => {
                ui.label(RichText::new("Hearing it without the master's chain").small().color(TEXT_DIM));
            }
            None => {
                ui.label(RichText::new("It measures with a limiter on the master").small().color(TEXT_DIM));
            }
        }
        ui.horizontal(|ui| {
            if self.master_limiter().is_none() {
                if ui.small_button("Add mastering").on_hover_text("Puts a gentle compressor and a limiter on the master, set to the target").clicked() {
                    self.add_mastering();
                }
            } else {
                if ui.small_button("Reach target").on_hover_text("Turns the limiter so that the song comes out at the target: play it for a while first").clicked() {
                    self.reach_master_target();
                }
                let before = !self.master_before.is_empty();
                if ui.selectable_label(before, "Before").on_hover_text("The song without the master's chain, to compare. It is quieter then, which does not make it worse").clicked() {
                    self.toggle_master_before();
                }
            }
        });
    }
}

/// Whether even no gain at all is too loud for the target.
fn gain_now_short(gain_now: f32, measured: f32, target: f32) -> bool {
    gain_now + target - measured < 0.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_gain_brings_the_song_to_the_target() {
        // At -20 with 2 dB of gain, to -14: 6 dB more.
        assert_eq!(gain_for(2.0, -20.0, -14.0), 8.0);
        // Too loud: less, down to none.
        assert_eq!(gain_for(4.0, -12.0, -14.0), 2.0);
        assert_eq!(gain_for(1.0, -8.0, -14.0), 0.0);
        assert!(gain_now_short(1.0, -8.0, -14.0));
        assert!(!gain_now_short(4.0, -12.0, -14.0));
        assert_eq!(gain_for(0.0, -60.0, -5.0), 30.0, "no further than the limiter goes");
    }

    #[test]
    fn every_target_can_be_set_on_the_limiter() {
        for t in &TARGETS {
            assert!((lim::SPECS[lim::CEILING].min..=lim::SPECS[lim::CEILING].max).contains(&t.ceiling), "{}", t.name);
            assert!((t.style as usize) < lim::STYLES.len(), "{}", t.name);
            assert!(t.lufs > -30.0 && t.lufs < 0.0, "{}", t.name);
        }
    }
}

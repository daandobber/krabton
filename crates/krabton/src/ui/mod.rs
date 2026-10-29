//! All views. Each submodule adds `impl App` methods for its part of the UI.

pub mod ambient_view;
pub mod arrangement;
pub mod automation;
pub mod beads_view;
pub mod browser;
pub mod chime_view;
pub mod chords_view;
pub mod classic_view;
pub mod detail;
pub mod devices;
pub mod drummer_view;
pub mod dialogs;
pub mod engine_screens;
pub mod envelope_view;
pub mod fm_panel;
pub mod glitch_view;
pub mod grains_view;
pub mod inst_screen;
pub mod io_section;
pub mod key_drawer;
pub mod lens;
pub mod grid_panel;
pub mod modulation;
pub mod mono101_view;
pub mod composers_view;
pub mod masters_view;
pub mod mechanical_view;
pub mod minimal_view;
pub mod nature_view;
pub mod nature_seq_view;
pub mod systems_view;
pub mod world_view;
pub mod reed_view;
pub mod chip_view;
pub mod warp_view;
pub mod orchestra_view;
pub mod filter_view;
pub mod chipfm_view;
pub mod v2_view;
pub mod oidos_view;
pub mod open_air_view;
pub mod pedal_view;
pub mod faces;
pub mod param_tabs;
pub mod piano_roll;
pub mod rack;
pub mod rack_view;
pub mod rhythm_more_view;
pub mod played_view;
pub mod players_view;
pub mod rhythm_view;
pub mod scope;
pub mod seq_more_view;
pub mod sing_view;
pub mod touch_view;
pub mod seq_view;
pub mod session;
pub mod settings;
pub mod map_view;
pub mod windows;
pub mod transport;

use crate::app::{App, ArrClipDrag, BrowserItem, ClipSel, SlotDrag, View};
use crate::theme::*;
use egui::{Align2, Context, DragAndDrop, Id, LayerId, Order};

impl App {
    /// Shows what is being dragged next to the mouse pointer.
    pub(crate) fn draw_drag_ghost(&self, ctx: &Context) {
        let label = if let Some(item) = DragAndDrop::payload::<BrowserItem>(ctx) {
            item.label()
        } else if let (Some(d), View::Session) = (DragAndDrop::payload::<ArrClipDrag>(ctx), self.view) {
            let name = self.project.tracks.iter().flat_map(|t| &t.arrangement).find(|a| a.id == d.id).map(|a| a.clip.name.clone()).unwrap_or_default();
            format!("{name}: drop on a slot")
        } else if let Some(chip) = DragAndDrop::payload::<modulation::ModChip>(ctx) {
            let name = self.project.modulators.iter().find(|m| m.id == chip.0).map(|m| m.name.clone()).unwrap_or_default();
            format!("{name}: drop on a knob")
        } else if let Some(d) = DragAndDrop::payload::<SlotDrag>(ctx) {
            let name = self.clip_ref(ClipSel::Slot { track: d.track, scene: d.scene }).map(|c| c.name.clone()).unwrap_or_default();
            let copy = ctx.input(|i| i.modifiers.ctrl || i.modifiers.command);
            format!("{name}{}", if copy { "  (copy)" } else { "" })
        } else {
            return;
        };
        let Some(pos) = ctx.input(|i| i.pointer.hover_pos()) else { return };
        let p = ctx.layer_painter(LayerId::new(Order::Tooltip, Id::new("drag_ghost")));
        let galley_pos = pos + egui::vec2(14.0, 10.0);
        let r = p.text(galley_pos, Align2::LEFT_TOP, &label, small(), TEXT_DARK);
        p.rect_filled(r.expand(4.0), 3.0, ACCENT);
        p.text(galley_pos, Align2::LEFT_TOP, &label, small(), TEXT_DARK);
    }
}

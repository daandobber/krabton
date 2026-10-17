//! Tool windows (the Map, the Settings, the History, the Glitch Sequencer):
//! a title bar of their own with minimize, maximize and close (a double
//! click on it maximizes too). A minimized one waits in the dock, in the
//! status bar, until clicked.

use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Context, Id, Rect, RichText, Sense, Stroke, Ui};

#[derive(Clone, Copy, Default)]
struct Flags {
    max: bool,
    min: bool,
}

/// The minimized windows: (key, title, frame it was last asked for).
#[derive(Clone, Default)]
struct Dock(Vec<(String, String, u64)>);

fn flags_id(key: &str) -> Id {
    Id::new(("tool_window", key))
}

fn dock_id() -> Id {
    Id::new("tool_dock")
}

enum Icon {
    Min,
    Max,
    Restore,
    Close,
}

/// A small button with a drawn icon; true when clicked.
fn icon_button(ui: &mut Ui, rect: Rect, id: Id, icon: Icon, tip: &str) -> bool {
    let resp = ui.interact(rect, id, Sense::click());
    let p = ui.painter();
    if resp.hovered() {
        rect_fill(p, rect, 3.0, if matches!(icon, Icon::Close) { REC } else { PANEL3 });
    }
    let c = if resp.hovered() { TEXT } else { TEXT_DIM };
    let s = Stroke::new(1.4_f32, c);
    let r = Rect::from_center_size(rect.center(), vec2(9.0, 9.0));
    match icon {
        Icon::Min => {
            p.line_segment([pos2(r.left(), r.bottom()), pos2(r.right(), r.bottom())], s);
        }
        Icon::Max => {
            p.rect_stroke(r, 1.0, s, egui::StrokeKind::Middle);
        }
        Icon::Restore => {
            let back = r.translate(vec2(2.0, -2.0)).shrink(1.0);
            p.rect_stroke(back, 1.0, s, egui::StrokeKind::Middle);
            p.rect_filled(r.translate(vec2(-1.0, 1.0)).shrink(1.0), 1.0, if resp.hovered() { PANEL3 } else { PANEL2 });
            p.rect_stroke(r.translate(vec2(-1.0, 1.0)).shrink(1.0), 1.0, s, egui::StrokeKind::Middle);
        }
        Icon::Close => {
            p.line_segment([r.left_top(), r.right_bottom()], s);
            p.line_segment([r.right_top(), r.left_bottom()], s);
        }
    }
    resp.on_hover_text(tip).clicked()
}

/// Shows a tool window with `body` in it. Returns false once closed, and
/// the body's result when it was drawn (not while minimized).
pub(crate) fn tool_window<R>(ctx: &Context, key: &str, title: &str, default: egui::Vec2, body: impl FnOnce(&mut Ui) -> R) -> (bool, Option<R>) {
    let id = flags_id(key);
    let mut f: Flags = ctx.data(|d| d.get_temp(id)).unwrap_or_default();
    if f.min {
        let frame = ctx.cumulative_pass_nr();
        ctx.data_mut(|d| {
            let dock = d.get_temp_mut_or_default::<Dock>(dock_id());
            dock.0.retain(|e| e.0 != key);
            dock.0.push((key.to_string(), title.to_string(), frame));
        });
        return (true, None);
    }
    let mut keep = true;
    let mut out = None;
    // Maximized and not are two windows, each keeping its own place and size.
    let win = egui::Window::new(title).id(id.with(if f.max { "max" } else { "normal" })).title_bar(false).collapsible(false).constrain(true);
    let win = if f.max { win.fixed_rect(ctx.available_rect().shrink(4.0)).resizable(false) } else { win.default_size(default).resizable(true) };
    win.show(ctx, |ui| {
        let (bar, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 24.0), Sense::click());
        text(ui.painter(), pos2(bar.left() + 4.0, bar.center().y), Align2::LEFT_CENTER, title, egui::FontId::proportional(14.0), TEXT);
        let b = |i: f32| Rect::from_center_size(pos2(bar.right() - 12.0 - i * 24.0, bar.center().y), vec2(22.0, 20.0));
        if icon_button(ui, b(0.0), id.with("close"), Icon::Close, "Close") {
            keep = false;
        }
        let (icon, tip) = if f.max { (Icon::Restore, "Back to its size") } else { (Icon::Max, "Fill the window") };
        if icon_button(ui, b(1.0), id.with("max"), icon, tip) || resp.double_clicked() {
            f.max = !f.max;
        }
        if icon_button(ui, b(2.0), id.with("min"), Icon::Min, "Minimize: it waits in the status bar") {
            f.min = true;
        }
        ui.painter().line_segment([pos2(bar.left(), bar.bottom() + 2.0), pos2(bar.right(), bar.bottom() + 2.0)], Stroke::new(1.0_f32, LINE_SOFT));
        ui.add_space(6.0);
        out = Some(body(ui));
    });
    if !keep {
        f = Flags::default();
    }
    ctx.data_mut(|d| d.insert_temp(id, f));
    (keep, out)
}

/// The minimized windows as buttons (in the status bar): a click brings one back.
pub(crate) fn dock(ui: &mut Ui) {
    let now = ui.ctx().cumulative_pass_nr();
    let entries: Vec<(String, String)> = ui.ctx().data(|d| d.get_temp::<Dock>(dock_id())).unwrap_or_default().0.into_iter().filter(|e| e.2 + 2 >= now).map(|e| (e.0, e.1)).collect();
    for (key, title) in entries {
        if ui.add(egui::Button::new(RichText::new(format!("^ {title}")).small().color(TEXT)).fill(PANEL3)).on_hover_text("Bring it back").clicked() {
            let id = flags_id(&key);
            ui.ctx().data_mut(|d| {
                let mut f: Flags = d.get_temp(id).unwrap_or_default();
                f.min = false;
                d.insert_temp(id, f);
                if let Some(dock) = d.get_temp_mut_or_default::<Dock>(dock_id()).0.iter().position(|e| e.0 == key) {
                    d.get_temp_mut_or_default::<Dock>(dock_id()).0.remove(dock);
                }
            });
        }
    }
}

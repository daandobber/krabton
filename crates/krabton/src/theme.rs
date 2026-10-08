//! Colors and global egui styling. Dark, flat, orange accents.

use egui::{Color32, FontId, TextStyle};

pub const BG: Color32 = Color32::from_rgb(0x18, 0x18, 0x1a);
pub const PANEL: Color32 = Color32::from_rgb(0x24, 0x24, 0x27);
pub const PANEL2: Color32 = Color32::from_rgb(0x2d, 0x2d, 0x31);
pub const PANEL3: Color32 = Color32::from_rgb(0x38, 0x38, 0x3d);
pub const SLOT: Color32 = Color32::from_rgb(0x30, 0x30, 0x34);
pub const LINE: Color32 = Color32::from_rgb(0x42, 0x42, 0x47);
pub const LINE_SOFT: Color32 = Color32::from_rgb(0x2f, 0x2f, 0x33);
pub const TEXT: Color32 = Color32::from_rgb(0xd8, 0xd8, 0xda);
pub const TEXT_DIM: Color32 = Color32::from_rgb(0x8c, 0x8c, 0x92);
pub const TEXT_DARK: Color32 = Color32::from_rgb(0x16, 0x16, 0x18);
pub const ACCENT: Color32 = Color32::from_rgb(0xff, 0xa7, 0x33);
pub const PLAY: Color32 = Color32::from_rgb(0x4c, 0xd9, 0x64);
pub const REC: Color32 = Color32::from_rgb(0xff, 0x45, 0x3a);
pub const SELECT: Color32 = Color32::from_rgb(0x9f, 0xd8, 0xff);
pub const SOLO: Color32 = Color32::from_rgb(0x4d, 0xa8, 0xff);
pub const KNOB: Color32 = Color32::from_rgb(0x1f, 0x1f, 0x22);

pub fn rgb(c: [u8; 3]) -> Color32 {
    Color32::from_rgb(c[0], c[1], c[2])
}

/// Scales brightness (0..1 darkens).
pub fn shade(c: Color32, f: f32) -> Color32 {
    let m = |v: u8| ((v as f32 * f).clamp(0.0, 255.0)) as u8;
    Color32::from_rgb(m(c.r()), m(c.g()), m(c.b()))
}

pub fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).clamp(0.0, 255.0) as u8;
    Color32::from_rgb(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()))
}

/// Readable text color on top of `c`.
pub fn text_on(c: Color32) -> Color32 {
    let lum = 0.299 * c.r() as f32 + 0.587 * c.g() as f32 + 0.114 * c.b() as f32;
    if lum > 140.0 {
        TEXT_DARK
    } else {
        TEXT
    }
}

pub fn small() -> FontId {
    FontId::proportional(10.5)
}

pub fn body() -> FontId {
    FontId::proportional(12.0)
}

pub fn mono() -> FontId {
    FontId::monospace(12.0)
}

pub fn apply(ctx: &egui::Context) {
    let mut v = egui::Visuals::dark();
    v.panel_fill = PANEL;
    v.window_fill = PANEL2;
    v.faint_bg_color = PANEL2;
    v.extreme_bg_color = BG;
    v.override_text_color = Some(TEXT);
    v.selection.bg_fill = shade(ACCENT, 0.55);
    v.selection.stroke = egui::Stroke::new(1.0_f32, ACCENT);
    v.hyperlink_color = ACCENT;
    v.widgets.noninteractive.bg_fill = PANEL;
    v.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0_f32, LINE_SOFT);
    v.widgets.inactive.bg_fill = PANEL3;
    v.widgets.inactive.weak_bg_fill = PANEL3;
    v.widgets.hovered.bg_fill = shade(PANEL3, 1.25);
    v.widgets.hovered.weak_bg_fill = shade(PANEL3, 1.25);
    v.widgets.active.bg_fill = shade(ACCENT, 0.7);
    v.widgets.active.weak_bg_fill = shade(ACCENT, 0.7);
    v.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0_f32, TEXT);
    v.widgets.inactive.fg_stroke = egui::Stroke::new(1.0_f32, TEXT);
    v.widgets.hovered.fg_stroke = egui::Stroke::new(1.5_f32, Color32::WHITE);
    v.widgets.active.fg_stroke = egui::Stroke::new(2.0_f32, Color32::WHITE);
    v.widgets.open.fg_stroke = egui::Stroke::new(1.0_f32, Color32::WHITE);
    // Always dark, whatever the OS theme says.
    ctx.set_theme(egui::ThemePreference::Dark);
    ctx.set_visuals_of(egui::Theme::Dark, v.clone());
    ctx.set_visuals_of(egui::Theme::Light, v);
    ctx.all_styles_mut(|s| {
        s.spacing.item_spacing = egui::vec2(6.0, 4.0);
        s.spacing.button_padding = egui::vec2(6.0, 2.0);
        s.spacing.interact_size.y = 20.0;
        s.text_styles.insert(TextStyle::Body, FontId::proportional(12.5));
        s.text_styles.insert(TextStyle::Button, FontId::proportional(12.5));
        s.text_styles.insert(TextStyle::Small, FontId::proportional(10.5));
        s.text_styles.insert(TextStyle::Heading, FontId::proportional(15.0));
        s.text_styles.insert(TextStyle::Monospace, FontId::monospace(12.0));
    });
}

//! Krabton: an open-source, Live-inspired DAW written in Rust.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod audio;
mod editing;
mod grid;
mod history;
mod midi;
mod plugins;
mod prefs;
mod pulse;
mod recording;
mod theme;
mod ui;
mod widgets;

fn main() -> eframe::Result<()> {
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("Krabton")
        .with_inner_size([1440.0, 900.0])
        .with_min_inner_size([960.0, 620.0])
        .with_maximized(true);
    if std::env::var_os("KRABTON_TEST_WINDOW").is_some() {
        // For automated checks: a small window that does not take focus.
        viewport = viewport.with_maximized(false).with_inner_size([1000.0, 640.0]).with_position([20.0, 20.0]).with_active(false);
    }
    let options = eframe::NativeOptions { viewport, ..Default::default() };
    eframe::run_native("Krabton", options, Box::new(|cc| Ok(Box::new(app::App::new(cc)))))
}

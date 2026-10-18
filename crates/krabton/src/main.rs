//! Krabton: an open-source, Live-inspired DAW written in Rust.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod audio;
mod band;
mod big;
mod editing;
mod grid;
mod history;
mod midi;
mod mp3;
mod drum_lib;
mod piano_lib;
mod sf2_lib;
mod orchestra_lib;
mod plugins;
mod prefs;
mod pulse;
mod recording;
mod shot;
mod skin;
mod tape;
mod theme;
mod ui;
mod widgets;

fn main() -> eframe::Result<()> {
    // The crab, for the title bar and the task bar.
    let icon = egui::IconData { rgba: include_bytes!("../assets/icon-64.rgba").to_vec(), width: 64, height: 64 };
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("Krabton")
        .with_icon(std::sync::Arc::new(icon))
        .with_inner_size([1440.0, 900.0])
        // Small enough for handhelds (a DevTerm is 1280 x 480).
        .with_min_inner_size([640.0, 400.0])
        .with_maximized(true);
    if std::env::var_os("KRABTON_TEST_WINDOW").is_some() {
        // For automated checks: a small window that does not take focus.
        // KRABTON_TEST_SIZE=1280x480 tries another screen (a handheld's).
        let size = std::env::var("KRABTON_TEST_SIZE").ok().and_then(|s| s.split_once('x').and_then(|(w, h)| Some([w.trim().parse::<f32>().ok()?, h.trim().parse::<f32>().ok()?]))).unwrap_or([1000.0, 640.0]);
        viewport = viewport.with_maximized(false).with_inner_size(size).with_min_inner_size([320.0, 240.0]).with_position([20.0, 20.0]).with_active(false);
    }
    let options = eframe::NativeOptions { viewport, ..Default::default() };
    eframe::run_native("Krabton", options, Box::new(|cc| Ok(Box::new(app::App::new(cc)))))
}

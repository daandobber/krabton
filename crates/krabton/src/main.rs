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
mod midi_out;
mod move_ctl;
mod mp3;
mod drum_lib;
mod firmware_lib;
mod piano_lib;
mod sf2_lib;
mod singer_lib;
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
mod video;
mod widgets;

/// Writes every panic down (Krabton has no console on Windows): when, in
/// which thread, what and where, and the call stack, appended to crash.log
/// beside the settings. The default message is still printed too.
fn log_panics() {
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        default(info);
        let Some(path) = prefs::Prefs::crash_log() else { return };
        let what = info.payload().downcast_ref::<&str>().map(|s| s.to_string()).or_else(|| info.payload().downcast_ref::<String>().cloned()).unwrap_or_default();
        let at = info.location().map(|l| format!("{}:{}", l.file(), l.line())).unwrap_or_default();
        let when = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        let thread = std::thread::current().name().unwrap_or("unnamed").to_string();
        let text = format!("Krabton {} ({}), {when} s after 1970, thread {thread}: {what}\n  at {at}\n{}\n\n", env!("CARGO_PKG_VERSION"), std::env::consts::OS, std::backtrace::Backtrace::force_capture());
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
            use std::io::Write;
            let _ = f.write_all(text.as_bytes());
        }
    }));
}

fn main() -> eframe::Result<()> {
    log_panics();
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

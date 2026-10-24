#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod audio;
mod ui;
mod app;

use app::AuditionApp;
use eframe::egui::vec2;

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size(vec2(1500.0, 850.0))
            .with_min_inner_size(vec2(900.0, 550.0))
            .with_title("Audition-RS — 파형 오디오 편집기 (Waveform Audio Editor)"),
        ..Default::default()
    };

    eframe::run_native(
        "Audition-RS",
        native_options,
        Box::new(|cc| Ok(Box::new(AuditionApp::new(cc)))),
    )
}

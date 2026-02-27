mod app;
mod config;

use eframe::egui;

use crate::app::PomodoroApp;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([320.0, 400.0])
            .with_title("Otamot"),
        ..Default::default()
    };

    eframe::run_native(
        "Otamot",
        options,
        Box::new(|cc| Ok(Box::new(PomodoroApp::new(cc)))),
    )
}

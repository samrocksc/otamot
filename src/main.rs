// Hide console window on Windows when launching the GUI
#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use eframe::egui;
use eframe::egui::IconData;

// Import from local app module
mod app;

fn load_icon() -> Option<IconData> {
    // Load the icon from assets/icon.png (embedded in binary)
    let icon_bytes = include_bytes!("../assets/icon.png");

    // Decode PNG using the image crate if available, otherwise use a simple approach
    // For now, we'll use eframe's built-in RGBA loading
    let image = image::load_from_memory(icon_bytes).ok()?;
    let image = image.resize(64, 64, image::imageops::FilterType::Lanczos3);
    let rgba = image.to_rgba8();
    let (width, height) = rgba.dimensions();

    Some(IconData {
        rgba: rgba.into_raw(),
        width,
        height,
    })
}

fn main() -> eframe::Result<()> {
    // Load icon
    let icon = load_icon();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([320.0, 400.0])
            .with_title("Otamot")
            .with_icon(icon.unwrap_or_default()),
        ..Default::default()
    };

    eframe::run_native(
        "Otamot",
        options,
        Box::new(|cc| Ok(Box::new(app::PomodoroApp::new(cc)))),
    )
}

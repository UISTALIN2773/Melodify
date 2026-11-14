#![windows_subsystem = "windows"]

mod app;
mod audio;
mod innertube;
mod models;
mod streamer;

use app::MelodyfyApp;
use eframe::egui::Vec2;

fn main() -> eframe::Result<()> {
    // Robust crash protection & logging
    std::panic::set_hook(Box::new(|info| {
        let err_text = format!("Melodyfy Panic Report:\n{:?}\n", info);
        let _ = std::fs::write("melodyfy_error.log", err_text);
    }));

    let is_mobile = std::env::args().any(|arg| arg == "--mobile" || arg == "-m")
        || std::env::current_exe()
            .map(|p| p.to_string_lossy().to_lowercase().contains("mobile"))
            .unwrap_or(false);

    let (initial_size, title) = if is_mobile {
        (
            Vec2::new(390.0, 820.0),
            "📱 Melodyfy Mobile — Spotify Mobile Rust",
        )
    } else {
        (
            Vec2::new(1150.0, 740.0),
            "🎧 Melodyfy Rust — Reproductor Libre, Rápido y Sin Anuncios",
        )
    };

    let native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title(title)
            .with_inner_size(initial_size)
            .with_min_inner_size(Vec2::new(320.0, 480.0))
            .with_active(true),
        ..Default::default()
    };

    eframe::run_native(
        "Melodyfy",
        native_options,
        Box::new(move |cc| {
            // Load Windows native Segoe UI Emoji font as fallback to eliminate ToFU (□)
            let mut fonts = eframe::egui::FontDefinitions::default();
            if let Ok(font_bytes) = std::fs::read("C:\\Windows\\Fonts\\seguiemj.ttf") {
                fonts.font_data.insert(
                    "seguiemj".to_owned(),
                    std::sync::Arc::new(eframe::egui::FontData::from_owned(font_bytes)),
                );
                fonts
                    .families
                    .entry(eframe::egui::FontFamily::Proportional)
                    .or_default()
                    .push("seguiemj".to_owned());
                fonts
                    .families
                    .entry(eframe::egui::FontFamily::Monospace)
                    .or_default()
                    .push("seguiemj".to_owned());
            }
            cc.egui_ctx.set_fonts(fonts);

            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(MelodyfyApp::new(cc, is_mobile)))
        }),
    )
}

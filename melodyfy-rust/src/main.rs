#![windows_subsystem = "windows"]

mod app;
mod audio;
mod innertube;
mod lyrics;
mod models;
mod streamer;

use app::MelodyfyApp;
use eframe::egui::{IconData, Vec2};
use std::sync::Arc;

pub const APP_LOGO_BYTES: &[u8] = include_bytes!("../assets/logo.jpg");
pub const APP_LOGO_URI: &str = "bytes://melodyfy_logo.jpg";

fn load_app_icon() -> Option<Arc<IconData>> {
    let img = image::load_from_memory(APP_LOGO_BYTES).ok()?.into_rgba8();
    let (width, height) = img.dimensions();
    Some(Arc::new(IconData {
        rgba: img.into_raw(),
        width,
        height,
    }))
}

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

    let mut viewport = eframe::egui::ViewportBuilder::default()
        .with_title(title)
        .with_inner_size(initial_size)
        .with_min_inner_size(Vec2::new(320.0, 480.0))
        .with_active(true);

    if let Some(icon) = load_app_icon() {
        viewport = viewport.with_icon(icon);
    }

    let native_options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "Melodyfy",
        native_options,
        Box::new(move |cc| {
            // Load Windows native fonts as fallback to eliminate ToFU (□) for Korean / CJK / Emojis
            let mut fonts = eframe::egui::FontDefinitions::default();

            // 1. Korean / Asian Hangul (Malgun Gothic)
            if let Ok(font_bytes) = std::fs::read("C:\\Windows\\Fonts\\malgun.ttf") {
                fonts.font_data.insert(
                    "malgun".to_owned(),
                    std::sync::Arc::new(eframe::egui::FontData::from_owned(font_bytes)),
                );
                fonts
                    .families
                    .entry(eframe::egui::FontFamily::Proportional)
                    .or_default()
                    .push("malgun".to_owned());
                fonts
                    .families
                    .entry(eframe::egui::FontFamily::Monospace)
                    .or_default()
                    .push("malgun".to_owned());
            }

            // 2. Windows native Segoe UI Emoji
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
            cc.egui_ctx.include_bytes(APP_LOGO_URI, APP_LOGO_BYTES);
            Ok(Box::new(MelodyfyApp::new(cc, is_mobile)))
        }),
    )
}

#![windows_subsystem = "windows"]

mod app;
mod core;
mod models;
mod ui;

use crate::app::InjectorApp;
use eframe::{icon_data, NativeOptions};
use std::sync::Arc;

fn main() -> eframe::Result<()> {
    let icon = icon_data::from_png_bytes(include_bytes!("../assets/icon.png"))
        .expect("Failed to load window icon from assets/icon.png");

    let native_options = NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_min_inner_size([600.0, 400.0])
            .with_icon(Arc::new(icon)),
        ..Default::default()
    };

    eframe::run_native(
        "Fluffy Injector",
        native_options,
        Box::new(|cc| Box::new(InjectorApp::new(cc))),
    )
}
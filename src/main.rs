#![windows_subsystem = "windows"]

mod app;
mod core;
mod models;
mod ui;

use crate::app::InjectorApp;
use crate::models::config::{Config, DEFAULT_WINDOW_SIZE, MIN_WINDOW_SIZE};
use eframe::{icon_data, NativeOptions};
use std::sync::Arc;

fn main() -> eframe::Result<()> {
    let icon = icon_data::from_png_bytes(include_bytes!("../assets/icon.png"))
        .expect("Failed to load window icon from assets/icon.png");
    let config = Config::load().unwrap_or_default();

    let mut viewport = eframe::egui::ViewportBuilder::default()
        .with_inner_size(config.saved_window_size().unwrap_or(DEFAULT_WINDOW_SIZE))
        .with_min_inner_size(MIN_WINDOW_SIZE)
        .with_icon(Arc::new(icon));
    if let Some(position) = config.saved_window_position() {
        viewport = viewport.with_position(position);
    }

    let native_options = NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "Fluffy Injector",
        native_options,
        Box::new(move |cc| Box::new(InjectorApp::new(cc, config))),
    )
}

#![windows_subsystem = "windows"]
#![feature(random)]

mod app;
mod core;
mod models;
mod ui;

use crate::app::InjectorApp;
use crate::models::{APP_NAME, DEFAULT_WINDOW_SIZE, MIN_WINDOW_SIZE};
use eframe::{icon_data, NativeOptions};
use std::sync::Arc;

fn main() -> eframe::Result<()> {
    let icon = icon_data::from_png_bytes(include_bytes!("../assets/icon.png"))
        .expect("Failed to load window icon from assets/icon.png");
    let viewport = eframe::egui::ViewportBuilder::default()
        .with_inner_size(DEFAULT_WINDOW_SIZE)
        .with_min_inner_size(MIN_WINDOW_SIZE)
        .with_icon(Arc::new(icon));

    let native_options = NativeOptions {
        viewport,
        persist_window: true,
        centered: true,
        ..Default::default()
    };

    eframe::run_native(
        APP_NAME,
        native_options,
        Box::new(|cc| Ok(Box::new(InjectorApp::new(cc)))),
    )
}

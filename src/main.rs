#![windows_subsystem = "console"]
#![feature(random)]

mod app;
mod cli;
mod core;
mod models;
mod ui;

use crate::app::InjectorApp;
use crate::models::{APP_NAME, DEFAULT_WINDOW_SIZE, MIN_WINDOW_SIZE};
use eframe::{icon_data, NativeOptions};
use std::sync::Arc;
use windows::Win32::System::Console::{FreeConsole, GetConsoleWindow};
use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_HIDE};

fn main() -> eframe::Result<()> {
    if let Some(code) = cli::run() {
        std::process::exit(code);
    }
    detach_console();
    run_gui()
}

fn detach_console() {
    let hwnd = unsafe { GetConsoleWindow() };
    if hwnd.is_invalid() {
        return;
    }
    let hidden = unsafe { ShowWindow(hwnd, SW_HIDE) }.as_bool();
    let detached = unsafe { FreeConsole() }.is_ok();
    let _detached = hidden || detached;
}

fn run_gui() -> eframe::Result<()> {
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

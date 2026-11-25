#![windows_subsystem = "windows"]
#![feature(random)]

mod app;
mod cli;
mod core;
mod models;
mod settings;
mod ui;

use crate::app::InjectorApp;
use crate::models::{Config, ToastLevel, APP_NAME, DEFAULT_WINDOW_SIZE, MIN_WINDOW_SIZE};
use taut::{Window, WindowPlacement};

fn main() {
    if let Some(code) = cli::run() {
        std::process::exit(code);
    }
    if taut::single_instance("fluffy-injector") {
        run_gui();
    }
}

fn run_gui() {
    let (config, warning, settings_writable) = match Config::load() {
        Ok((config, warning)) => (config, warning, true),
        Err(error) => (
            Config::default(),
            Some(format!(
                "{error:#}. Settings will not be saved this session."
            )),
            false,
        ),
    };
    let placement = config
        .window
        .filter(|place| {
            [place.width, place.height]
                .into_iter()
                .all(|size| size.is_finite() && size > 0.0)
        })
        .map(|place| WindowPlacement {
            position: place.position,
            width: place.width.max(MIN_WINDOW_SIZE[0]),
            height: place.height.max(MIN_WINDOW_SIZE[1]),
            maximized: place.maximized,
        });
    // With no explicit icons, taut uses the multi-size icon embedded by build.rs.
    let window = Window {
        title: APP_NAME,
        width: DEFAULT_WINDOW_SIZE[0],
        height: DEFAULT_WINDOW_SIZE[1],
        min_width: MIN_WINDOW_SIZE[0],
        min_height: MIN_WINDOW_SIZE[1],
        screen: Some(0),
        ..Default::default()
    }
    .with_placement(placement);
    let mut app = InjectorApp::new(config);
    app.settings_writable = settings_writable;
    if let Some(warning) = warning {
        app.add_toast(ToastLevel::Error, warning);
    }
    taut::run(window, move |ui| app.describe(ui));
}

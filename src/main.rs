mod modules;
use eframe::{NativeOptions, icon_data};
use modules::app::InjectorApp;
use std::sync::Arc;

fn main() -> eframe::Result<()> {
    let d = icon_data::from_png_bytes(include_bytes!("../assets/icon.png"))
        .expect("The icon data must be valid");
    let mut native_options = NativeOptions::default();
    native_options.viewport.icon = Some(Arc::new(d));

    eframe::run_native(
        "Fluffy Injector",
        native_options,
        Box::new(|_cc| Box::new(InjectorApp::default())),
    )
}

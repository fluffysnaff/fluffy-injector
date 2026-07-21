use crate::app::InjectorApp;
use crate::models::config::Dll;
use crate::models::process::ProcessInfo;
use crate::models::toast::{Toast, ToastLevel};
use eframe::egui::{self, Color32, Frame, Margin, RichText, TextureHandle, Vec2};

pub(crate) fn show(ui: &mut egui::Ui, app: &mut InjectorApp) -> bool {
    ui.ctx().set_visuals(egui::Visuals::dark());
    draw_top_bar(ui, app);
    let changed = draw_central_panel(ui, app);
    draw_toasts(ui.ctx(), app);
    changed
}

fn selected_process_label(app: &InjectorApp) -> String {
    match app.selected_process_info() {
        Some(process) => format!("Selected Process: {} ({})", process.name, process.pid),
        None => app
            .config
            .last_selected_app
            .as_ref()
            .map(|name| format!("Waiting for Process: {name}"))
            .unwrap_or_else(|| "Selected Process: None".to_string()),
    }
}

fn draw_top_bar(ui: &mut egui::Ui, app: &InjectorApp) {
    let process_label = selected_process_label(app);
    let dll_count = app.selected_dlls().count();
    let frame = Frame::default()
        .fill(Color32::from_rgb(30, 30, 30))
        .stroke(egui::Stroke::new(1.0, Color32::from_gray(80)))
        .inner_margin(Margin::same(8));
    egui::Panel::top("selected_info_panel")
        .frame(frame)
        .show(ui, |ui| {
            ui.horizontal(|ui| draw_selected_summary(ui, &process_label, dll_count));
        });
}

fn draw_selected_summary(ui: &mut egui::Ui, process_label: &str, dll_count: usize) {
    ui.label(
        RichText::new(process_label)
            .size(16.0)
            .color(Color32::WHITE),
    );
    ui.add_space(20.0);
    ui.separator();
    ui.add_space(20.0);
    ui.label(
        RichText::new(format!("Selected DLLs: {dll_count}"))
            .size(16.0)
            .color(Color32::LIGHT_BLUE),
    );
}

fn draw_central_panel(ui: &mut egui::Ui, app: &mut InjectorApp) -> bool {
    let mut changed = false;
    let frame = Frame::default()
        .fill(Color32::from_rgb(20, 20, 20))
        .inner_margin(Margin::same(12));
    egui::CentralPanel::default()
        .frame(frame)
        .show(ui, |ui| changed |= draw_columns(ui, app));
    changed
}

fn draw_columns(ui: &mut egui::Ui, app: &mut InjectorApp) -> bool {
    let mut changed = false;
    let width = ui.available_width();
    let height = ui.available_height();
    ui.horizontal(|ui| {
        ui.allocate_ui(Vec2::new(width * 0.4, height), |ui| {
            changed |= draw_process_panel(ui, app);
        });
        ui.allocate_ui(Vec2::new(width * 0.6, height), |ui| {
            changed |= draw_dll_panel(ui, app);
        });
    });
    changed
}

fn draw_process_panel(ui: &mut egui::Ui, app: &mut InjectorApp) -> bool {
    let mut changed = false;
    ui.vertical(|ui| {
        ui.label(RichText::new("🔍 Search Process").size(14.0).strong());
        ui.text_edit_singleline(&mut app.process_search);
        ui.add_space(5.0);
        ui.separator();
        ui.add_space(10.0);
        egui::ScrollArea::vertical()
            .id_salt("process_list")
            .show(ui, |ui| changed |= draw_process_list(ui, app));
    });
    changed
}

fn draw_process_list(ui: &mut egui::Ui, app: &mut InjectorApp) -> bool {
    if app.is_loading_processes && app.processes.is_empty() {
        ui.add(egui::Spinner::new());
        return false;
    }

    let search = app.process_search.to_lowercase();
    let mut selection = None;
    for process in &app.processes {
        if !search.is_empty() && !process.name.to_lowercase().contains(&search) {
            continue;
        }
        let selected = app.selected_process == Some(process.pid);
        if draw_process_row(ui, process, app.icon_cache.get(&process.pid), selected) {
            selection = Some((process.pid, process.name.clone()));
        }
    }
    let Some((pid, name)) = selection else {
        return false;
    };
    app.selected_process = Some(pid);
    app.config.last_selected_app = Some(name);
    true
}

fn draw_process_row(
    ui: &mut egui::Ui,
    process: &ProcessInfo,
    texture: Option<&TextureHandle>,
    selected: bool,
) -> bool {
    ui.horizontal(|ui| {
        match texture {
            Some(texture) => ui.image((texture.id(), Vec2::new(16.0, 16.0))),
            None => ui.label("❔"),
        };
        let label = RichText::new(format!("{} ({})", process.name, process.pid));
        ui.selectable_label(selected, label).clicked()
    })
    .inner
}

fn draw_dll_panel(ui: &mut egui::Ui, app: &mut InjectorApp) -> bool {
    let mut changed = false;
    ui.vertical(|ui| {
        draw_dll_header(ui);
        egui::ScrollArea::vertical()
            .id_salt("dll_list")
            .show(ui, |ui| {
                changed |= draw_dll_list(ui, &mut app.config.dlls, app.default_dll_texture.id());
            });
        ui.add_space(20.0);
        ui.separator();
        ui.add_space(10.0);
        changed |= draw_dll_settings(ui, app);
        ui.add_space(10.0);
        changed |= draw_dll_actions(ui, app);
        ui.add_space(10.0);
    });
    changed
}

fn draw_dll_header(ui: &mut egui::Ui) {
    ui.label(
        RichText::new("📂 DLLs")
            .size(18.0)
            .strong()
            .color(Color32::WHITE),
    );
    ui.add_space(5.0);
    ui.separator();
    ui.add_space(5.0);
}

fn draw_dll_settings(ui: &mut egui::Ui, app: &mut InjectorApp) -> bool {
    let copy_changed = ui
        .checkbox(&mut app.config.copy_dll_on_inject, "Copy on inject")
        .on_hover_text("Keeps the original DLL free for rebuilding.")
        .changed();
    let random_changed = ui
        .add_enabled(
            app.config.copy_dll_on_inject,
            egui::Checkbox::new(&mut app.config.randomize_dll_name, "Random name"),
        )
        .on_hover_text("Gives the copied DLL a random name.")
        .changed();
    copy_changed || random_changed
}

fn draw_dll_actions(ui: &mut egui::Ui, app: &mut InjectorApp) -> bool {
    let mut changed = false;
    let size = Vec2::new(100.0, 35.0);
    let selected_count = app.selected_dlls().count();
    let inject_enabled = app.selected_process.is_some() && selected_count > 0 && !app.is_injecting;
    let inject_label = if app.is_injecting {
        "Injecting..."
    } else {
        "🚀 Inject"
    };
    ui.horizontal(|ui| {
        if button_clicked(ui, true, "➕ Add DLL", size) {
            changed |= add_dll(app);
        }
        if button_clicked(ui, inject_enabled, inject_label, size) {
            app.start_injection(ui.ctx());
        }
        if button_clicked(ui, selected_count > 0, "❌ Remove", size) {
            let removed = remove_selected_dlls(app);
            app.add_toast(ToastLevel::Info, format!("Removed {removed} DLL(s)."));
            changed = true;
        }
    });
    changed
}

fn button_clicked(ui: &mut egui::Ui, enabled: bool, label: &str, size: Vec2) -> bool {
    ui.add_enabled(enabled, egui::Button::new(label).min_size(size))
        .clicked()
}

fn draw_dll_list(ui: &mut egui::Ui, dlls: &mut [Dll], texture: egui::TextureId) -> bool {
    let mut changed = false;
    for dll in dlls {
        changed |= ui
            .horizontal(|ui| {
                ui.image((texture, Vec2::new(16.0, 16.0)));
                let name = std::path::Path::new(&dll.path)
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy();
                ui.checkbox(&mut dll.selected, name).changed()
            })
            .inner;
    }
    changed
}

fn add_dll(app: &mut InjectorApp) -> bool {
    let Some(path) = crate::core::dll_selector::select_dll() else {
        return false;
    };
    if app.config.dlls.iter().any(|dll| dll.path == path) {
        app.add_toast(ToastLevel::Warning, "DLL is already in the list.");
        return false;
    }
    app.config.dlls.push(Dll {
        path,
        selected: false,
    });
    true
}

fn remove_selected_dlls(app: &mut InjectorApp) -> usize {
    let previous_len = app.config.dlls.len();
    app.config.dlls.retain(|dll| !dll.selected);
    previous_len - app.config.dlls.len()
}

fn draw_toasts(ctx: &egui::Context, app: &mut InjectorApp) {
    app.toasts.retain(|toast| toast.is_alive());
    egui::Area::new("toasts".into())
        .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-8.0, -8.0))
        .show(ctx, |ui| {
            for toast in &app.toasts {
                draw_toast(ui, toast);
                ui.add_space(5.0);
            }
        });
}

fn draw_toast(ui: &mut egui::Ui, toast: &Toast) {
    let (icon, color) = match toast.level {
        ToastLevel::Info => ("ℹ", Color32::from_gray(180)),
        ToastLevel::Success => ("✅", Color32::GREEN),
        ToastLevel::Warning => ("⚠", Color32::YELLOW),
        ToastLevel::Error => ("❌", Color32::RED),
    };
    egui::Frame::popup(ui.style()).show(ui, |ui| {
        ui.set_min_width(200.0);
        ui.label(RichText::new(format!("{icon} {}", toast.message)).color(color));
    });
}

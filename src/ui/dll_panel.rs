use crate::app::InjectorApp;
use crate::models::config::Dll;
use crate::models::toast::ToastLevel;
use crate::ui::{rule_separator, SURFACE};
use eframe::egui::{self, Color32, Frame, Margin, RichText, Vec2};

pub(super) fn show(ui: &mut egui::Ui, app: &mut InjectorApp) -> bool {
    let mut changed = false;
    egui::Panel::bottom("dll_footer")
        .resizable(false)
        .show_separator_line(false)
        .frame(
            Frame::NONE
                .fill(SURFACE)
                .inner_margin(Margin {
                    left: 0,
                    right: 0,
                    top: 10,
                    bottom: 4,
                }),
        )
        .show(ui, |ui| changed |= draw_footer(ui, app));
    ui.label(RichText::new("DLLs").size(15.0).strong());
    ui.add_space(8.0);
    let list_size = ui.available_size();
    egui::ScrollArea::vertical()
        .id_salt("dll_list")
        .auto_shrink([false, false])
        .max_width(list_size.x)
        .max_height(list_size.y)
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            changed |= draw_dll_list(ui, &mut app.config.dlls, app.default_dll_texture.id());
        });
    changed
}

fn draw_footer(ui: &mut egui::Ui, app: &mut InjectorApp) -> bool {
    let mut changed = false;
    rule_separator(ui);
    ui.add_space(10.0);
    changed |= draw_settings(ui, app);
    ui.add_space(10.0);
    changed |= draw_actions(ui, app);
    changed
}

fn draw_settings(ui: &mut egui::Ui, app: &mut InjectorApp) -> bool {
    ui.horizontal(|ui| {
        let copy_changed = ui
            .checkbox(&mut app.config.copy_dll_on_inject, "Copy on inject")
            .on_hover_text("Keeps the original DLL free for rebuilding.")
            .changed();
        ui.add_space(16.0);
        let random_changed = ui
            .add_enabled(
                app.config.copy_dll_on_inject,
                egui::Checkbox::new(&mut app.config.randomize_dll_name, "Random name"),
            )
            .on_hover_text("Gives the copied DLL a random name.")
            .changed();
        copy_changed || random_changed
    })
    .inner
}

fn draw_actions(ui: &mut egui::Ui, app: &mut InjectorApp) -> bool {
    let mut changed = false;
    let selected_count = app.selected_dlls().count();
    let inject_enabled = app.selected_process.is_some() && selected_count > 0 && !app.is_injecting;
    let inject_label = if app.is_injecting {
        "Injecting…"
    } else {
        "Inject"
    };
    let size = Vec2::new(action_button_width(ui), 24.0);
    ui.horizontal(|ui| {
        if action_button(ui, true, "Add DLL", size) {
            changed |= add_dll(app);
        }
        if action_button(ui, inject_enabled, inject_label, size) {
            app.start_injection(ui.ctx());
        }
        if action_button(ui, selected_count > 0, "Remove", size) {
            let removed = remove_selected_dlls(app);
            app.add_toast(ToastLevel::Info, format!("Removed {removed} DLL(s)."));
            changed = true;
        }
    });
    changed
}

fn action_button_width(ui: &egui::Ui) -> f32 {
    let spacing = ui.spacing().item_spacing.x;
    ((ui.available_width() - spacing * 2.0) / 3.0)
        .floor()
        .clamp(49.0, 250.0)
}

fn action_button(ui: &mut egui::Ui, enabled: bool, label: &str, size: Vec2) -> bool {
    ui.add_enabled(enabled, egui::Button::new(label).min_size(size))
        .clicked()
}

fn draw_dll_list(ui: &mut egui::Ui, dlls: &mut [Dll], texture: egui::TextureId) -> bool {
    if dlls.is_empty() {
        ui.colored_label(Color32::DARK_GRAY, "No DLLs added yet.");
        return false;
    }
    let mut changed = false;
    for dll in dlls {
        changed |= draw_dll_row(ui, dll, texture);
    }
    changed
}

fn draw_dll_row(ui: &mut egui::Ui, dll: &mut Dll, texture: egui::TextureId) -> bool {
    ui.horizontal(|ui| {
        ui.add(egui::Image::new((texture, Vec2::splat(16.0))));
        let name = std::path::Path::new(&dll.path)
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| dll.path.clone());
        ui.checkbox(&mut dll.selected, name).changed()
    })
    .inner
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

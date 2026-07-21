use crate::app::InjectorApp;
use crate::models::toast::ToastLevel;
use eframe::egui::{self, Color32, Frame, Margin, RichText, Vec2};

pub(crate) fn show(ui: &mut egui::Ui, app: &mut InjectorApp) -> bool {
    ui.ctx().set_visuals(egui::Visuals::dark());
    draw_top_bar(ui, app);
    let changed = draw_central_panel(ui, app);
    draw_toasts(ui.ctx(), app);
    changed
}

fn draw_top_bar(ui: &mut egui::Ui, app: &InjectorApp) {
    egui::Panel::top("selected_info_panel")
        .frame(
            Frame::default()
                .fill(Color32::from_rgb(30, 30, 30))
                .stroke(egui::Stroke::new(1.0_f32, Color32::from_gray(80)))
                .inner_margin(Margin::same(8)),
        )
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let process_label = match app.selected_process_info() {
                    Some(process) => {
                        format!("Selected Process: {} ({})", process.name, process.pid)
                    }
                    None => app
                        .config
                        .last_selected_app
                        .as_ref()
                        .map(|name| format!("Waiting for Process: {}", name))
                        .unwrap_or_else(|| "Selected Process: None".to_string()),
                };
                ui.label(
                    RichText::new(process_label)
                        .size(16.0)
                        .color(Color32::WHITE),
                );
                ui.add_space(20.0);
                ui.separator();
                ui.add_space(20.0);
                let dll_label = format!("Selected DLLs: {}", app.selected_dlls().count());
                ui.label(
                    RichText::new(dll_label)
                        .size(16.0)
                        .color(Color32::LIGHT_BLUE),
                );
            });
        });
}

fn draw_central_panel(ui: &mut egui::Ui, app: &mut InjectorApp) -> bool {
    let mut changed = false;
    egui::CentralPanel::default()
        .frame(
            Frame::default()
                .fill(Color32::from_rgb(20, 20, 20))
                .inner_margin(Margin::same(12)),
        )
        .show(ui, |ui| {
            let total_width = ui.available_width();
            let total_height = ui.available_height();
            let left_width = total_width * 0.4;
            let right_width = total_width * 0.6;

            ui.horizontal(|ui| {
                ui.allocate_ui(Vec2::new(left_width, total_height), |ui| {
                    changed |= draw_process_panel(ui, app);
                });
                ui.allocate_ui(Vec2::new(right_width, total_height), |ui| {
                    changed |= draw_dll_panel(ui, app);
                });
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
            .show(ui, |ui| {
                if app.is_loading_processes && app.processes.is_empty() {
                    ui.add(egui::Spinner::new());
                } else {
                    let search_lower = app.process_search.to_lowercase();
                    for proc in &app.processes {
                        if !app.process_search.is_empty()
                            && !proc.name.to_lowercase().contains(&search_lower)
                        {
                            continue;
                        }
                        ui.horizontal(|ui| {
                            if let Some(texture) = app.icon_cache.get(&proc.pid) {
                                ui.image((texture.id(), Vec2::new(16.0, 16.0)));
                            } else {
                                ui.label("❔");
                            }
                            let is_selected = app.selected_process == Some(proc.pid);
                            let label = format!("{} ({})", proc.name, proc.pid);
                            if ui
                                .selectable_label(is_selected, RichText::new(label))
                                .clicked()
                            {
                                app.selected_process = Some(proc.pid);
                                app.config.last_selected_app = Some(proc.name.clone());
                                changed = true;
                            }
                        });
                    }
                }
            });
    });
    changed
}

fn draw_dll_panel(ui: &mut egui::Ui, app: &mut InjectorApp) -> bool {
    let mut changed = false;
    ui.vertical(|ui| {
        ui.label(
            RichText::new("📂 DLLs")
                .size(18.0)
                .strong()
                .color(Color32::WHITE),
        );
        ui.add_space(5.0);
        ui.separator();
        ui.add_space(5.0);

        egui::ScrollArea::vertical()
            .id_salt("dll_list")
            .show(ui, |ui| {
                for dll in &mut app.dlls {
                    let file_name = std::path::Path::new(&dll.path)
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy();

                    ui.horizontal(|ui| {
                        ui.image((app.default_dll_texture.id(), Vec2::new(16.0, 16.0)));

                        if ui.checkbox(&mut dll.selected, file_name).changed() {
                            changed = true;
                        }
                    });
                }
            });

        ui.add_space(20.0);
        ui.separator();
        ui.add_space(10.0);

        if ui
            .checkbox(&mut app.config.copy_dll_on_inject, "Copy on inject")
            .on_hover_text("Keeps the original DLL free for rebuilding.")
            .changed()
        {
            changed = true;
        }
        let random_name_enabled = app.config.copy_dll_on_inject;
        if ui
            .add_enabled(
                random_name_enabled,
                egui::Checkbox::new(&mut app.config.randomize_dll_name, "Random name"),
            )
            .on_hover_text("Gives the copied DLL a random name.")
            .changed()
        {
            changed = true;
        }
        ui.add_space(10.0);

        ui.horizontal(|ui| {
            let button_size = Vec2::new(100.0, 35.0);
            if ui
                .add(egui::Button::new("➕ Add DLL").min_size(button_size))
                .clicked()
            {
                if let Some(path) = crate::core::dll_selector::select_dll() {
                    if !app.dlls.iter().any(|dll| dll.path == path) {
                        app.dlls.push(crate::app::Dll {
                            path,
                            selected: false,
                        });
                        changed = true;
                    } else {
                        app.add_toast(ToastLevel::Warning, "DLL is already in the list.");
                    }
                }
            }
            let selected_count = app.selected_dlls().count();
            let inject_enabled =
                app.selected_process.is_some() && selected_count > 0 && !app.is_injecting;
            let inject_label = if app.is_injecting {
                "Injecting..."
            } else {
                "🚀 Inject"
            };
            if ui
                .add_enabled(
                    inject_enabled,
                    egui::Button::new(inject_label).min_size(button_size),
                )
                .clicked()
            {
                app.start_injection(ui.ctx());
            }
            if ui
                .add_enabled(
                    selected_count > 0,
                    egui::Button::new("❌ Remove").min_size(button_size),
                )
                .clicked()
            {
                let previous_len = app.dlls.len();
                app.dlls.retain(|dll| !dll.selected);
                let removed = previous_len - app.dlls.len();
                changed = true;
                app.add_toast(ToastLevel::Info, format!("Removed {} DLL(s).", removed));
            }
        });
        ui.add_space(10.0);
    });
    changed
}

fn draw_toasts(ctx: &egui::Context, app: &mut InjectorApp) {
    app.toasts.retain(|toast| toast.is_alive());
    egui::Area::new("toasts".into())
        .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-8.0, -8.0))
        .show(ctx, |ui| {
            for toast in &app.toasts {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.set_min_width(200.0);
                    let (icon, color) = match toast.level {
                        ToastLevel::Info => ("ℹ", Color32::from_gray(180)),
                        ToastLevel::Success => ("✅", Color32::GREEN),
                        ToastLevel::Warning => ("⚠", Color32::YELLOW),
                        ToastLevel::Error => ("❌", Color32::RED),
                    };
                    let text = RichText::new(format!("{} {}", icon, toast.message)).color(color);
                    ui.label(text);
                });
                ui.add_space(5.0);
            }
        });
}

use crate::app::InjectorApp;
use crate::core::injector;
use crate::models::toast::ToastLevel;
use eframe::egui::{self, Color32, Frame, Margin, RichText, Vec2};

pub fn show(ctx: &egui::Context, app: &mut InjectorApp) {
    ctx.set_visuals(egui::Visuals::dark());
    draw_top_bar(ctx, app);
    draw_central_panel(ctx, app);
    draw_toasts(ctx, app);
}

fn draw_top_bar(ctx: &egui::Context, app: &InjectorApp) {
    egui::TopBottomPanel::top("selected_info_panel")
        .frame(
            Frame::default()
                .fill(Color32::from_rgb(30, 30, 30))
                .stroke(egui::Stroke::new(1.0, Color32::from_gray(80)))
                .inner_margin(Margin::same(8.0)),
        )
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                let process_label = match app.selected_process_name() {
                    Some(name) => format!("Selected Process: {} ({})", name, app.selected_process.unwrap()),
                    None => "Selected Process: None".to_string(),
                };
                ui.label(RichText::new(process_label).size(16.0).color(Color32::WHITE));
                ui.add_space(20.0);
                ui.separator();
                ui.add_space(20.0);
                let dll_label = match app.dll_manager.selected_path() {
                    Some(path) => format!("Selected DLL: {}", std::path::Path::new(&path).file_name().unwrap_or_default().to_string_lossy()),
                    None => "Selected DLL: None".to_string(),
                };
                ui.label(RichText::new(dll_label).size(16.0).color(Color32::LIGHT_BLUE));
            });
        });
}

fn draw_central_panel(ctx: &egui::Context, app: &mut InjectorApp) {
    egui::CentralPanel::default()
        .frame(
            Frame::default()
                .fill(Color32::from_rgb(20, 20, 20))
                .inner_margin(Margin::same(12.0)),
        )
        .show(ctx, |ui| {
            let total_width = ui.available_width();
            let total_height = ui.available_height();
            let left_width = total_width * 0.4;
            let right_width = total_width * 0.6;

            ui.horizontal(|ui| {
                ui.allocate_ui(Vec2::new(left_width, total_height), |ui| {
                    draw_process_panel(ui, app);
                });
                ui.allocate_ui(Vec2::new(right_width, total_height), |ui| {
                    draw_dll_panel(ui, app);
                });
            });
        });
}

fn draw_process_panel(ui: &mut egui::Ui, app: &mut InjectorApp) {
    let mut errors_to_toast: Vec<String> = Vec::new();
    ui.vertical(|ui| {
        ui.add_space(10.0);
        ui.label(RichText::new("🔍 Search Process").size(14.0).strong());
        ui.text_edit_singleline(&mut app.process_search);
        ui.add_space(5.0);
        ui.separator();
        ui.add_space(5.0);
        ui.horizontal(|ui| {
            if ui.button("🔄 Refresh").clicked() {
                app.refresh_processes();
            }
            ui.checkbox(&mut app.auto_refresh, "Auto");
        });
        ui.add_space(10.0);
        egui::ScrollArea::vertical().id_source("process_list").show(ui, |ui| {
            if app.is_loading_processes && app.processes.is_empty() {
                ui.add(egui::Spinner::new());
            } else {
                let search_lower = app.process_search.to_lowercase();
                for proc in &app.processes {
                    if !app.process_search.is_empty() && !proc.name.to_lowercase().contains(&search_lower) {
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
                        if ui.selectable_label(is_selected, RichText::new(label)).clicked() {
                            app.selected_process = Some(proc.pid);
                            app.config.last_selected_app = Some(proc.name.clone());
                            if let Err(e) = app.config.save() {
                                errors_to_toast.push(format!("Failed to save config: {}", e));
                            }
                        }
                    });
                }
            }
        });
    });
    for error_msg in errors_to_toast {
        app.add_toast(ToastLevel::Error, error_msg);
    }
}

fn draw_dll_panel(ui: &mut egui::Ui, app: &mut InjectorApp) {
    ui.vertical(|ui| {
        ui.add_space(10.0);
        ui.label(RichText::new("📂 DLLs").size(18.0).strong().color(Color32::WHITE));
        ui.add_space(5.0);
        ui.separator();
        ui.add_space(5.0);

        egui::ScrollArea::vertical().id_source("dll_list").show(ui, |ui| {
            for i in 0..app.dll_manager.get_dlls().len() {
                let is_selected = app.dll_manager.selected_dll() == Some(i);
                let file_name = std::path::Path::new(&app.dll_manager.get_dlls()[i])
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                
                ui.horizontal(|ui| {
                    if let Some(tex) = &app.default_dll_texture {
                        ui.image((tex.id(), Vec2::new(16.0, 16.0)));
                    } else {
                        ui.label("❔");
                    }
                    
                    if ui.selectable_label(is_selected, file_name).clicked() {
                        app.dll_manager.select(Some(i));
                    }
                });
            }
        });
        
        ui.add_space(20.0);
        ui.separator();
        ui.add_space(10.0);

        ui.horizontal(|ui| {
            let button_size = Vec2::new(100.0, 35.0);
            if ui.add(egui::Button::new("➕ Add DLL").min_size(button_size)).clicked() {
                if let Some(path) = crate::core::dll_selector::select_dll() {
                    if !app.dll_manager.get_dlls().contains(&path) {
                        app.dll_manager.add(path.clone());
                        app.config.dlls.push(path);
                        let _ = app.config.save();
                    } else {
                        app.add_toast(ToastLevel::Warning, "DLL is already in the list.");
                    }
                }
            }
            let inject_enabled = app.selected_process.is_some() && app.dll_manager.selected_dll().is_some();
            if ui.add_enabled(inject_enabled, egui::Button::new("🚀 Inject DLL").min_size(button_size)).clicked() {
                if let (Some(pid), Some(dll_path)) = (app.selected_process, app.dll_manager.selected_path()) {
                    match injector::inject_dll(pid, &dll_path) {
                        Ok(_) => app.add_toast(ToastLevel::Success, "Injection successful!"),
                        Err(e) => app.add_toast(ToastLevel::Error, format!("Injection failed: {}", e)),
                    }
                }
            }
            let remove_enabled = app.dll_manager.selected_dll().is_some();
            if ui.add_enabled(remove_enabled, egui::Button::new("❌ Remove File").min_size(button_size)).clicked() {
                if let Some(selected_index) = app.dll_manager.selected_dll() {
                    app.dll_manager.remove(selected_index);
                    app.config.dlls.remove(selected_index);
                    let _ = app.config.save();
                    app.add_toast(ToastLevel::Info, "DLL removed.");
                }
            }
        });
        ui.add_space(10.0);
    });
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
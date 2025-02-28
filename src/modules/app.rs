use crate::modules::config::Config;
use crate::modules::dll::{DLLManager, select_dll};
use crate::modules::icon;
use crate::modules::injector::inject_dll;
use crate::modules::process::{ProcessInfo, get_processes};
use eframe::egui::{self, Color32, Frame, Margin, RichText, Stroke, Vec2, Visuals};
use std::collections::HashMap;

pub struct InjectorApp {
    processes: Vec<ProcessInfo>,
    selected_process: Option<u32>,
    dll_manager: DLLManager,
    injection_message: Option<String>,
    process_search: String,
    config: Config,
    icon_cache: HashMap<u32, egui::TextureHandle>,
}

impl InjectorApp {
    fn selected_process_name(&self) -> Option<&str> {
        self.selected_process.and_then(|pid| {
            self.processes
                .iter()
                .find(|p| p.pid == pid)
                .map(|p| p.name.as_str())
        })
    }
}

impl Default for InjectorApp {
    fn default() -> Self {
        let config = Config::load();
        let processes = get_processes();
        let selected_process = if let Some(ref last_app) = config.last_selected_app {
            processes
                .iter()
                .find(|p| p.name == *last_app)
                .map(|p| p.pid)
        } else {
            None
        };

        let mut dll_manager = DLLManager::new();
        for dll in &config.dlls {
            dll_manager.add(dll.clone());
        }

        Self {
            processes,
            selected_process,
            dll_manager,
            injection_message: None,
            process_search: String::new(),
            config,
            icon_cache: HashMap::new(),
        }
    }
}

impl eframe::App for InjectorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.set_visuals(Visuals::dark());

        egui::TopBottomPanel::top("selected_info_panel")
            .frame(
                Frame::default()
                    .fill(Color32::from_rgb(30, 30, 30))
                    .rounding(5.0)
                    .stroke(Stroke::new(1.0, Color32::from_gray(80)))
                    .inner_margin(Margin::same(8.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let process_label = match self.selected_process {
                        Some(pid) => {
                            if let Some(name) = self.selected_process_name() {
                                format!("Selected Process: {name} ({pid})")
                            } else {
                                format!("Selected Process: PID {pid}")
                            }
                        }
                        None => "Selected Process: None".to_string(),
                    };

                    let dll_label = match self.dll_manager.selected_dll() {
                        Some(idx) => format!("Selected DLL: {}", self.dll_manager.get_dlls()[idx]),
                        None => "Selected DLL: None".to_string(),
                    };

                    ui.label(
                        RichText::new(process_label)
                            .size(16.0)
                            .color(Color32::WHITE),
                    );
                    ui.add_space(20.0);
                    ui.separator();
                    ui.add_space(20.0);
                    ui.label(
                        RichText::new(dll_label)
                            .size(16.0)
                            .color(Color32::LIGHT_BLUE),
                    );
                });
            });

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
                    // Left Panel: Processes
                    ui.allocate_ui(Vec2::new(left_width, total_height), |ui| {
                        ui.vertical(|ui| {
                            ui.add_space(10.0);
                            ui.label(RichText::new("🔍 Search Process").size(14.0).strong());
                            ui.text_edit_singleline(&mut self.process_search);
                            ui.add_space(5.0);
                            ui.separator();
                            ui.add_space(5.0);

                            if ui.button("🔄 Refresh").clicked() {
                                self.processes = crate::modules::process::get_processes();
                                if let Some(ref last_app) = self.config.last_selected_app {
                                    if let Some(proc) =
                                        self.processes.iter().find(|p| p.name == *last_app)
                                    {
                                        self.selected_process = Some(proc.pid);
                                    }
                                }
                            }

                            ui.add_space(10.0);

                            egui::ScrollArea::vertical()
                                .id_source("process_list")
                                .show(ui, |ui| {
                                    let search_lower = self.process_search.to_lowercase();
                                    for proc in &self.processes {
                                        if !self.process_search.is_empty()
                                            && !proc.name.to_lowercase().contains(&search_lower)
                                        {
                                            continue;
                                        }
                                        ui.horizontal(|ui| {
                                            // Render the icon if available.
                                            if let Some(tex) = self.icon_cache.get(&proc.pid) {
                                                ui.allocate_ui(Vec2::new(16.0, 16.0), |ui| {
                                                    ui.add(egui::Image::new(tex));
                                                });
                                            } else if let Some(tex) =
                                                crate::modules::icon::load_exe_icon(ctx, &proc.exe)
                                            {
                                                self.icon_cache.insert(proc.pid, tex.clone());
                                                ui.allocate_ui(Vec2::new(16.0, 16.0), |ui| {
                                                    ui.add(egui::Image::new(&tex));
                                                });
                                            } else {
                                                ui.label("❔");
                                            }
                                            if ui
                                                .selectable_label(
                                                    Some(proc.pid) == self.selected_process,
                                                    format!("{} ({})", proc.name, proc.pid),
                                                )
                                                .clicked()
                                            {
                                                self.selected_process = Some(proc.pid);
                                                self.config.last_selected_app =
                                                    Some(proc.name.clone());
                                                let _ = self.config.save();
                                            }
                                        });
                                    }
                                });
                        });
                    });

                    // Right Panel: DLL list & Buttons
                    ui.allocate_ui(Vec2::new(right_width, total_height), |ui| {
                        ui.vertical(|ui| {
                            ui.add_space(10.0);
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
                                .id_source("dll_list")
                                .show(ui, |ui| {
                                    let dlls = self.dll_manager.get_dlls();
                                    let mut selected_idx = self.dll_manager.selected_dll();

                                    for (i, dll) in dlls.iter().enumerate() {
                                        if ui
                                            .selectable_label(Some(i) == selected_idx, dll)
                                            .clicked()
                                        {
                                            selected_idx = Some(i);
                                        }
                                    }
                                    if let Some(i) = selected_idx {
                                        self.dll_manager.select(i);
                                    }
                                });

                            ui.add_space(20.0);
                            ui.separator();
                            ui.add_space(10.0);

                            ui.horizontal(|ui| {
                                if ui
                                    .add(
                                        egui::Button::new("➕ Add DLL")
                                            .fill(Color32::from_rgb(50, 50, 50))
                                            .rounding(8.0)
                                            .min_size(Vec2::new(100.0, 35.0)),
                                    )
                                    .clicked()
                                {
                                    if let Some(path) = crate::modules::dll::select_dll() {
                                        self.dll_manager.add(path.clone());
                                        self.config.dlls.push(path);
                                        let _ = self.config.save();
                                    }
                                }

                                if ui
                                    .add(
                                        egui::Button::new("🚀 Inject DLL")
                                            .fill(Color32::from_rgb(50, 50, 50))
                                            .rounding(8.0)
                                            .min_size(Vec2::new(100.0, 35.0)),
                                    )
                                    .clicked()
                                {
                                    if let Some(pid) = self.selected_process {
                                        if let Some(dll_path) = self.dll_manager.selected_path() {
                                            let success = inject_dll(pid, &dll_path);
                                            self.injection_message = Some(if success {
                                                "✅ Injection successful!".to_string()
                                            } else {
                                                "❌ Injection failed.".to_string()
                                            });
                                        } else {
                                            self.injection_message =
                                                Some("❌ Select a DLL first.".to_string());
                                        }
                                    } else {
                                        self.injection_message =
                                            Some("❌ Select a process first.".to_string());
                                    }
                                }
                            });

                            ui.add_space(10.0);

                            if let Some(ref msg) = self.injection_message {
                                ui.label(RichText::new(msg).size(16.0).color(
                                    if msg.contains("✅") {
                                        Color32::GREEN
                                    } else {
                                        Color32::RED
                                    },
                                ));
                            }
                        });
                    });
                });
            });
    }
}

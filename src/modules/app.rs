use crate::modules::dll::{DLLManager, select_dll};
use crate::modules::injector::inject_dll;
use crate::modules::process::get_processes;
use eframe::egui::{self, Color32, Frame, Margin, RichText, Stroke, Vec2, Visuals};

pub struct InjectorApp {
    processes: Vec<(String, u32)>,
    selected_process: Option<u32>,
    dll_manager: DLLManager,
    injection_message: Option<String>,
    process_search: String,
}

impl InjectorApp {
    fn selected_process_name(&self) -> Option<&str> {
        self.selected_process.and_then(|pid| {
            self.processes
                .iter()
                .find(|(_, p)| *p == pid)
                .map(|(name, _)| name.as_str())
        })
    }
}

impl Default for InjectorApp {
    fn default() -> Self {
        Self {
            processes: get_processes(),
            selected_process: None,
            dll_manager: DLLManager::new(),
            injection_message: None,
            process_search: String::new(),
        }
    }
}

impl eframe::App for InjectorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 🔥 Apply a custom dark mode theme
        ctx.set_visuals(Visuals::dark());

        // --- 🌟 Top Panel (Better Spacing & Padding) ---
        egui::TopBottomPanel::top("selected_info_panel")
            .frame(
                Frame::default()
                    .fill(Color32::from_rgb(30, 30, 30)) // Dark header
                    .rounding(5.0) // Rounded corners
                    .stroke(Stroke::new(1.0, Color32::from_gray(80))) // Subtle outline
                    .inner_margin(Margin::same(8.0)), // Adds padding inside the panel
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

                    // 🔹 Styled text with spacing
                    ui.label(
                        RichText::new(process_label)
                            .size(16.0)
                            .color(Color32::WHITE),
                    );
                    ui.add_space(20.0); // Adds space between labels
                    ui.separator();
                    ui.add_space(20.0);
                    ui.label(
                        RichText::new(dll_label)
                            .size(16.0)
                            .color(Color32::LIGHT_BLUE),
                    );
                });
            });

        // --- 🌟 Central Panel with Padding ---
        egui::CentralPanel::default()
            .frame(
                Frame::default()
                    .fill(Color32::from_rgb(20, 20, 20)) // Background color
                    .inner_margin(Margin::same(12.0)), // Adds padding inside the panel
            )
            .show(ctx, |ui| {
                let total_width = ui.available_width();
                let total_height = ui.available_height();
                let left_width = total_width * 0.4;
                let right_width = total_width * 0.6;

                ui.horizontal(|ui| {
                    // --- 🌟 Left Panel: Processes ---
                    ui.allocate_ui(Vec2::new(left_width, total_height), |ui| {
                        ui.vertical(|ui| {
                            ui.add_space(10.0);
                            ui.label(RichText::new("🔍 Search Process").size(14.0).strong());
                            ui.text_edit_singleline(&mut self.process_search);
                            ui.add_space(5.0);
                            ui.separator();
                            ui.add_space(5.0);

                            // 🔄 Refresh Button: Reloads process list
                            if ui.button("🔄 Refresh").clicked() {
                                self.processes = get_processes(); // Fetch updated processes
                            }

                            ui.add_space(10.0);

                            egui::ScrollArea::vertical()
                                .id_source("process_list")
                                .show(ui, |ui| {
                                    let search_lower = self.process_search.to_lowercase();
                                    for (name, pid) in &self.processes {
                                        if !self.process_search.is_empty()
                                            && !name.to_lowercase().contains(&search_lower)
                                        {
                                            continue;
                                        }
                                        if ui
                                            .selectable_label(
                                                Some(*pid) == self.selected_process,
                                                format!("{name} ({pid})"),
                                            )
                                            .clicked()
                                        {
                                            self.selected_process = Some(*pid);
                                        }
                                    }
                                });
                        });
                    });

                    // --- 🌟 Right Panel: DLL List & Buttons ---
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

                            // --- 🌟 Styled Buttons with Padding ---
                            ui.horizontal(|ui| {
                                if ui
                                    .add(
                                        egui::Button::new("➕ Add DLL")
                                            .fill(Color32::from_rgb(50, 50, 50)) // Button background color
                                            .rounding(8.0) // Rounded corners
                                            .min_size(Vec2::new(100.0, 35.0)), // Make buttons wider
                                    )
                                    .clicked()
                                {
                                    if let Some(path) = select_dll() {
                                        self.dll_manager.add(path);
                                    }
                                }

                                if ui
                                    .add(
                                        egui::Button::new("🚀 Inject DLL")
                                            .fill(Color32::from_rgb(50, 50, 50)) // Button background color
                                            .rounding(8.0) // Rounded corners
                                            .min_size(Vec2::new(100.0, 35.0)), // Make buttons wider
                                    )
                                    .clicked()
                                {
                                    if let Some(pid) = self.selected_process {
                                        if let Some(dll_path) = self.dll_manager.selected_path() {
                                            let success = inject_dll(pid, &dll_path);
                                            self.injection_message = Some(
                                                if success {
                                                    "✅ Injection successful!"
                                                } else {
                                                    "❌ Injection failed."
                                                }
                                                .to_string(),
                                            );
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

                            // --- 🌟 Animated Injection Status ---
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

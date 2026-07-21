use crate::core::{icon_loader, process_scanner};
use crate::models::config::Config;
use crate::models::dll_manager::DLLManager;
use crate::models::process::ProcessInfo;
use crate::models::toast::{Toast, ToastLevel};
use crate::ui;
use eframe::egui::{ColorImage, Context, TextureHandle};
use std::collections::{HashMap, HashSet};
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{Duration, Instant};

const WINDOW_SAVE_DELAY: Duration = Duration::from_millis(500);

pub enum BackgroundMessage {
    Processes(Vec<ProcessInfo>),
    Icon((u32, ColorImage)),
}

pub struct InjectorApp {
    // Core State
    pub processes: Vec<ProcessInfo>,
    pub selected_process: Option<u32>,
    pub dll_manager: DLLManager,
    pub config: Config,
    pub is_loading_processes: bool,
    pub icon_cache: HashMap<u32, TextureHandle>,
    pub default_dll_texture: Option<TextureHandle>,
    pub toasts: Vec<Toast>,

    // UI-specific state moved here
    pub process_search: String,

    // Private fields
    background_rx: Receiver<BackgroundMessage>,
    icon_tx: Sender<(u32, std::path::PathBuf)>,
    window_save_at: Option<Instant>,
}

impl InjectorApp {
    pub fn new(cc: &eframe::CreationContext<'_>, config: Config) -> Self {
        let mut dll_manager = DLLManager::new();
        let selected_dlls: HashSet<&str> =
            config.selected_dlls.iter().map(String::as_str).collect();
        for dll in &config.dlls {
            let index = dll_manager.get_dlls().len();
            dll_manager.add(dll.clone());
            dll_manager.set_selected(index, selected_dlls.contains(dll.as_str()));
        }

        let (background_tx, background_rx) = mpsc::channel();
        let (icon_tx, icon_rx) = mpsc::channel();

        let process_tx = background_tx.clone();
        let ctx_clone = cc.egui_ctx.clone();
        std::thread::spawn(move || {
            process_scanner::scan_loop(process_tx, ctx_clone);
        });

        let icon_bgtx = background_tx;
        let ctx_clone = cc.egui_ctx.clone();
        std::thread::spawn(move || {
            icon_loader::load_loop(icon_rx, icon_bgtx, ctx_clone);
        });

        let mut app = Self {
            processes: Vec::new(),
            selected_process: None,
            dll_manager,
            config,
            background_rx,
            icon_tx,
            window_save_at: None,
            is_loading_processes: true,
            icon_cache: HashMap::new(),
            default_dll_texture: None,
            toasts: Vec::new(),
            process_search: String::new(),
        };

        app.load_default_dll_icon(&cc.egui_ctx);
        app
    }

    fn handle_background_updates(&mut self, ctx: &Context) {
        while let Ok(message) = self.background_rx.try_recv() {
            match message {
                BackgroundMessage::Processes(procs) => {
                    self.update_processes(procs);
                }
                BackgroundMessage::Icon((pid, color_image)) => {
                    let texture =
                        ctx.load_texture(format!("icon_{}", pid), color_image, Default::default());
                    self.icon_cache.insert(pid, texture);
                }
            }
        }
    }

    fn request_missing_icons(&self) {
        for proc in &self.processes {
            if !proc.exe.as_os_str().is_empty() && !self.icon_cache.contains_key(&proc.pid) {
                let _ = self.icon_tx.send((proc.pid, proc.exe.clone()));
            }
        }
    }

    fn update_processes(&mut self, processes: Vec<ProcessInfo>) {
        let live_pids: HashSet<u32> = processes.iter().map(|process| process.pid).collect();
        self.icon_cache.retain(|pid, _| live_pids.contains(pid));
        self.selected_process = resolve_process_selection(
            &processes,
            self.selected_process,
            self.config.last_selected_app.as_deref(),
        );
        self.processes = processes;
        self.is_loading_processes = false;
        self.request_missing_icons();
    }

    fn track_window_geometry(&mut self, ctx: &Context) {
        let geometry = ctx.input(|input| {
            let viewport = input.viewport();
            if viewport.minimized == Some(true)
                || viewport.maximized == Some(true)
                || viewport.fullscreen == Some(true)
            {
                return None;
            }

            let position = viewport.outer_rect?.min;
            let size = viewport.inner_rect?.size();
            Some((
                [position.x.round(), position.y.round()],
                [size.x.round(), size.y.round()],
            ))
        });
        let now = Instant::now();

        if let Some((position, size)) = geometry {
            if self.config.window_position != Some(position)
                || self.config.window_size != Some(size)
            {
                self.config.window_position = Some(position);
                self.config.window_size = Some(size);
                self.window_save_at = Some(now + WINDOW_SAVE_DELAY);
            }
        }

        if let Some(save_at) = self.window_save_at {
            if now >= save_at {
                self.window_save_at = None;
                if let Err(error) = self.config.save() {
                    self.add_toast(
                        ToastLevel::Error,
                        format!("Failed to save window position: {}", error),
                    );
                }
            } else {
                ctx.request_repaint_after(save_at.saturating_duration_since(now));
            }
        }
    }

    pub fn selected_process_name(&self) -> Option<&str> {
        self.selected_process.and_then(|pid| {
            self.processes
                .iter()
                .find(|p| p.pid == pid)
                .map(|p| p.name.as_str())
        })
    }

    pub fn add_toast(&mut self, level: ToastLevel, message: impl Into<String>) {
        self.toasts.push(Toast::new(level, message));
    }

    fn load_default_dll_icon(&mut self, ctx: &Context) {
        let image_bytes = include_bytes!("../assets/dll_icon.png");
        if let Ok(dyn_img) = image::load_from_memory(image_bytes) {
            let resized = dyn_img.resize_exact(16, 16, image::imageops::FilterType::Lanczos3);
            let rgba = resized.to_rgba8();
            let color_image = ColorImage::from_rgba_unmultiplied([16, 16], rgba.as_raw());
            self.default_dll_texture =
                Some(ctx.load_texture("dll_default", color_image, Default::default()));
        }
    }
}

impl eframe::App for InjectorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_background_updates(ctx);
        self.track_window_geometry(ctx);
        ui::show(ctx, self);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        let _ = self.config.save();
    }
}

fn resolve_process_selection(
    processes: &[ProcessInfo],
    selected_pid: Option<u32>,
    selected_name: Option<&str>,
) -> Option<u32> {
    if let Some(pid) = selected_pid {
        let still_running = processes.iter().any(|process| {
            process.pid == pid
                && selected_name
                    .map(|name| process.name.eq_ignore_ascii_case(name))
                    .unwrap_or(true)
        });
        if still_running {
            return Some(pid);
        }
    }

    selected_name.and_then(|name| {
        processes
            .iter()
            .find(|process| process.name.eq_ignore_ascii_case(name))
            .map(|process| process.pid)
    })
}

#[cfg(test)]
mod tests {
    use super::{resolve_process_selection, ProcessInfo};
    use std::path::PathBuf;

    fn process(name: &str, pid: u32) -> ProcessInfo {
        ProcessInfo {
            name: name.into(),
            pid,
            exe: PathBuf::new(),
        }
    }

    #[test]
    fn keeps_live_pid_and_reacquires_terminated_process_by_name() {
        let processes = [process("game.exe", 10), process("game.exe", 20)];

        assert_eq!(
            resolve_process_selection(&processes, Some(20), Some("game.exe")),
            Some(20)
        );
        assert_eq!(
            resolve_process_selection(&processes, Some(99), Some("GAME.EXE")),
            Some(10)
        );
        assert_eq!(
            resolve_process_selection(&processes, Some(99), Some("missing.exe")),
            None
        );
    }
}

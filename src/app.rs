use crate::core::{icon_loader, injector, process_scanner};
use crate::models::config::Config;
use crate::models::process::ProcessInfo;
use crate::models::toast::{Toast, ToastLevel};
use eframe::egui::{self, ColorImage, Context, TextureHandle};
use std::collections::{HashMap, HashSet};
use std::sync::mpsc::{self, Receiver, Sender};

pub(crate) enum BackgroundMessage {
    Processes(Vec<ProcessInfo>),
    Icon((u32, ColorImage)),
    Injection { total: usize, failures: Vec<String> },
    Error(String),
}

pub(crate) struct Dll {
    pub path: String,
    pub selected: bool,
}

pub(crate) struct InjectorApp {
    // Core State
    pub processes: Vec<ProcessInfo>,
    pub selected_process: Option<u32>,
    pub dlls: Vec<Dll>,
    pub config: Config,
    pub is_loading_processes: bool,
    pub icon_cache: HashMap<u32, TextureHandle>,
    pub default_dll_texture: TextureHandle,
    pub toasts: Vec<Toast>,
    pub is_injecting: bool,

    // UI-specific state moved here
    pub process_search: String,

    // Private fields
    background_rx: Receiver<BackgroundMessage>,
    background_tx: Sender<BackgroundMessage>,
    icon_tx: Sender<(u32, std::path::PathBuf)>,
}

impl InjectorApp {
    pub(crate) fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let (config, config_error) = match Config::load(cc.storage) {
            Ok(config) => (config, None),
            Err(error) => (Config::default(), Some(error)),
        };
        let dlls = config
            .dlls
            .iter()
            .cloned()
            .map(|path| Dll {
                selected: config.selected_dlls.contains(&path),
                path,
            })
            .collect();

        let (background_tx, background_rx) = mpsc::channel();
        let (icon_tx, icon_rx) = mpsc::channel();

        let process_tx = background_tx.clone();
        let ctx_clone = cc.egui_ctx.clone();
        std::thread::spawn(move || process_scanner::scan_loop(process_tx, ctx_clone));

        let icon_bgtx = background_tx.clone();
        let ctx_clone = cc.egui_ctx.clone();
        std::thread::spawn(move || icon_loader::load_loop(icon_rx, icon_bgtx, ctx_clone));

        let mut app = Self {
            processes: Vec::new(),
            selected_process: None,
            dlls,
            config,
            background_rx,
            background_tx,
            icon_tx,
            is_loading_processes: true,
            icon_cache: HashMap::new(),
            default_dll_texture: load_default_dll_icon(&cc.egui_ctx),
            toasts: Vec::new(),
            is_injecting: false,
            process_search: String::new(),
        };
        if let Some(error) = config_error {
            app.add_toast(
                ToastLevel::Error,
                format!("Failed to load settings: {error}"),
            );
        }
        if cc.storage.is_none() {
            app.add_toast(ToastLevel::Error, "AppData settings are unavailable.");
        }
        app
    }

    fn handle_background_updates(&mut self, ctx: &Context) {
        while let Ok(message) = self.background_rx.try_recv() {
            match message {
                BackgroundMessage::Processes(procs) => self.update_processes(procs),
                BackgroundMessage::Icon((pid, color_image)) => {
                    let texture =
                        ctx.load_texture(format!("icon_{pid}"), color_image, Default::default());
                    self.icon_cache.insert(pid, texture);
                }
                BackgroundMessage::Injection { total, failures } => {
                    self.finish_injection(total, failures)
                }
                BackgroundMessage::Error(error) => self.add_toast(ToastLevel::Error, error),
            }
        }
    }

    fn request_missing_icons(&self) {
        for proc in &self.processes {
            if !proc.exe.as_os_str().is_empty()
                && !self.icon_cache.contains_key(&proc.pid)
                && self.icon_tx.send((proc.pid, proc.exe.clone())).is_err()
            {
                break;
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

    pub(crate) fn selected_process_info(&self) -> Option<&ProcessInfo> {
        self.selected_process
            .and_then(|pid| self.processes.iter().find(|process| process.pid == pid))
    }

    pub(crate) fn add_toast(&mut self, level: ToastLevel, message: impl Into<String>) {
        self.toasts.push(Toast::new(level, message));
    }

    fn persist_config(&mut self, storage: &mut dyn eframe::Storage) {
        self.config.dlls = self.dlls.iter().map(|dll| dll.path.clone()).collect();
        self.config.selected_dlls = self
            .dlls
            .iter()
            .filter(|dll| dll.selected)
            .map(|dll| dll.path.clone())
            .collect();
        if let Err(error) = self.config.save(storage) {
            self.add_toast(
                ToastLevel::Error,
                format!("Failed to save settings: {error}"),
            );
        }
    }

    pub(crate) fn selected_dlls(&self) -> impl Iterator<Item = &str> {
        self.dlls
            .iter()
            .filter_map(|dll| dll.selected.then_some(dll.path.as_str()))
    }

    pub(crate) fn start_injection(&mut self, ctx: &Context) {
        let Some(pid) = self.selected_process else {
            return;
        };
        let dlls: Vec<String> = self.selected_dlls().map(str::to_owned).collect();
        if dlls.is_empty() || self.is_injecting {
            return;
        }

        self.is_injecting = true;
        let copy = self.config.copy_dll_on_inject;
        let randomize = self.config.randomize_dll_name;
        let tx = self.background_tx.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let total = dlls.len();
            let failures = dlls
                .iter()
                .filter_map(|path| {
                    injector::inject_dll(pid, path, copy, randomize)
                        .err()
                        .map(|error| format!("{}: {error}", dll_name(path)))
                })
                .collect();
            if tx
                .send(BackgroundMessage::Injection { total, failures })
                .is_err()
            {
                return;
            }
            ctx.request_repaint();
        });
    }

    fn finish_injection(&mut self, total: usize, failures: Vec<String>) {
        self.is_injecting = false;
        if failures.is_empty() {
            self.add_toast(ToastLevel::Success, format!("Injected {total} DLL(s)."));
            return;
        }
        self.add_toast(
            ToastLevel::Error,
            format!(
                "Injected {}/{}. Failed: {}",
                total - failures.len(),
                total,
                failures.join("; ")
            ),
        );
    }
}

fn dll_name(path: &str) -> String {
    std::path::Path::new(path)
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned()
}

fn load_default_dll_icon(ctx: &Context) -> TextureHandle {
    let image = image::load_from_memory(include_bytes!("../assets/dll_icon.png"))
        .expect("embedded DLL icon must be valid")
        .resize_exact(16, 16, image::imageops::FilterType::Lanczos3)
        .to_rgba8();
    ctx.load_texture(
        "dll_default",
        ColorImage::from_rgba_unmultiplied([16, 16], image.as_raw()),
        Default::default(),
    )
}

impl eframe::App for InjectorApp {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        self.handle_background_updates(ui.ctx());
        let changed = crate::ui::show(ui, self);
        if let (true, Some(storage)) = (changed, frame.storage_mut()) {
            self.persist_config(storage);
        }
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        self.persist_config(storage);
    }

    fn persist_egui_memory(&self) -> bool {
        false
    }
}

fn resolve_process_selection(
    processes: &[ProcessInfo],
    selected_pid: Option<u32>,
    selected_name: Option<&str>,
) -> Option<u32> {
    selected_pid
        .filter(|pid| {
            processes.iter().any(|process| {
                process.pid == *pid
                    && selected_name.is_none_or(|name| process.name.eq_ignore_ascii_case(name))
            })
        })
        .or_else(|| {
            selected_name.and_then(|name| {
                processes
                    .iter()
                    .find(|process| process.name.eq_ignore_ascii_case(name))
                    .map(|process| process.pid)
            })
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

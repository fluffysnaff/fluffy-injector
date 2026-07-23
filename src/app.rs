use crate::core::{icon_loader, injector, process_scanner};
use crate::models::{Config, ProcessInfo, Toast, ToastLevel};
use eframe::egui::{self, ColorImage, Context, TextureHandle};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::mpsc::{self, Receiver, Sender};

pub(crate) enum BackgroundMessage {
    Processes(Vec<ProcessInfo>),
    Icon((u32, ColorImage)),
    Injection(usize, Vec<String>),
    Error(String),
}

pub(crate) struct InjectorApp {
    pub processes: Vec<ProcessInfo>,
    pub selected_process: Option<u32>,
    pub config: Config,
    pub is_loading_processes: bool,
    pub icon_cache: HashMap<u32, TextureHandle>,
    pub default_dll_texture: TextureHandle,
    pub toasts: Vec<Toast>,
    pub is_injecting: bool,
    pub process_search: String,
    pub process_search_lower: String,
    background_rx: Receiver<BackgroundMessage>,
    background_tx: Sender<BackgroundMessage>,
    icon_tx: Sender<(u32, std::path::PathBuf)>,
}

impl InjectorApp {
    pub(crate) fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let config = Config::load(cc.storage);
        let (background_tx, background_rx) = mpsc::channel();
        let (icon_tx, icon_rx) = mpsc::channel();
        spawn_workers(&cc.egui_ctx, &background_tx, icon_rx);
        crate::ui::apply_theme(&cc.egui_ctx);
        Self {
            processes: Vec::new(),
            selected_process: None,
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
            process_search_lower: String::new(),
        }
    }

    fn update_icon(&mut self, ctx: &Context, (pid, image): (u32, ColorImage)) {
        let texture = ctx.load_texture(format!("icon_{pid}"), image, Default::default());
        self.icon_cache.insert(pid, texture);
    }

    fn update_processes(&mut self, mut processes: Vec<ProcessInfo>) {
        let live: HashSet<u32> = processes.iter().map(|p| p.pid).collect();
        self.icon_cache.retain(|pid, _| live.contains(pid));
        order_by_favorite(&mut processes, &self.config);
        let last = self
            .config
            .last_selected_app
            .as_deref()
            .filter(|name| !self.config.is_blocked(name));
        self.selected_process = resolve_selection(&processes, self.selected_process, last);
        self.processes = processes;
        self.is_loading_processes = false;
        request_missing_icons(&self.processes, &self.icon_cache, &self.icon_tx);
    }

    pub(crate) fn order_processes_by_favorite(&mut self) {
        order_by_favorite(&mut self.processes, &self.config);
    }

    pub(crate) fn sync_process_search_lower(&mut self) {
        self.process_search_lower = self.process_search.to_ascii_lowercase();
    }

    pub(crate) fn selected_process_info(&self) -> Option<&ProcessInfo> {
        self.selected_process
            .and_then(|pid| self.processes.iter().find(|p| p.pid == pid))
    }

    pub(crate) fn add_toast(&mut self, level: ToastLevel, message: impl Into<String>) {
        self.toasts.push(Toast::new(level, message));
    }

    fn persist_config(&mut self, storage: &mut dyn eframe::Storage) {
        if let Err(error) = self.config.save(storage) {
            self.add_toast(ToastLevel::Error, format!("Failed to save settings: {error}"));
        }
    }

    pub(crate) fn selected_dlls(&self) -> impl Iterator<Item = &str> {
        self.config
            .dlls
            .iter()
            .filter_map(|dll| dll.selected.then_some(dll.path.as_str()))
    }

    pub(crate) fn start_injection(&mut self, ctx: &Context) {
        let dlls: Vec<String> = self.selected_dlls().map(str::to_owned).collect();
        self.start_injection_of(ctx, dlls);
    }

    pub(crate) fn start_injection_of(&mut self, ctx: &Context, dlls: Vec<String>) {
        let Some(pid) = self.selected_process else {
            return;
        };
        if dlls.is_empty() || self.is_injecting {
            return;
        }
        self.is_injecting = true;
        let copy = self.config.copy_dll_on_inject;
        let randomize = self.config.randomize_dll_name;
        let tx = self.background_tx.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let message = inject_dlls(pid, &dlls, copy, randomize);
            if tx.send(message).is_ok() {
                ctx.request_repaint();
            }
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

fn spawn_workers(
    ctx: &Context,
    background_tx: &Sender<BackgroundMessage>,
    icon_rx: Receiver<(u32, std::path::PathBuf)>,
) {
    let process_tx = background_tx.clone();
    let process_ctx = ctx.clone();
    std::thread::spawn(move || process_scanner::scan_loop(process_tx, process_ctx));
    let icon_tx = background_tx.clone();
    let icon_ctx = ctx.clone();
    std::thread::spawn(move || icon_loader::load_loop(icon_rx, icon_tx, icon_ctx));
}

fn handle_background_message(app: &mut InjectorApp, ctx: &Context, message: BackgroundMessage) {
    match message {
        BackgroundMessage::Processes(processes) => app.update_processes(processes),
        BackgroundMessage::Icon(icon) => app.update_icon(ctx, icon),
        BackgroundMessage::Injection(total, failures) => app.finish_injection(total, failures),
        BackgroundMessage::Error(error) => app.add_toast(ToastLevel::Error, error),
    }
}

fn request_missing_icons(
    processes: &[ProcessInfo],
    icon_cache: &HashMap<u32, TextureHandle>,
    icon_tx: &Sender<(u32, std::path::PathBuf)>,
) {
    for process in processes {
        let missing = !process.exe.as_os_str().is_empty() && !icon_cache.contains_key(&process.pid);
        if missing && icon_tx.send((process.pid, process.exe.clone())).is_err() {
            break;
        }
    }
}

fn inject_dlls(pid: u32, dlls: &[String], copy: bool, randomize: bool) -> BackgroundMessage {
    let failures = dlls
        .iter()
        .filter_map(|path| {
            injector::inject_dll(pid, path, copy, randomize).err().map(|error| {
                let name = Path::new(path)
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy();
                format!("{name}: {error}")
            })
        })
        .collect();
    BackgroundMessage::Injection(dlls.len(), failures)
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
        while let Ok(message) = self.background_rx.try_recv() {
            handle_background_message(self, ui.ctx(), message);
        }
        let changed = crate::ui::show(ui, self);
        if let (true, Some(storage)) = (changed, frame.storage_mut()) {
            self.persist_config(storage);
        }
    }

    /// Persist on change via [`Self::ui`] and frequent autosave (window placement).
    /// Do not rely on graceful shutdown — End Task never reaches close handlers.
    fn save(&mut self, _storage: &mut dyn eframe::Storage) {}

    fn auto_save_interval(&self) -> std::time::Duration {
        // Frequent enough for End Task; not every frame (avoids saving mid-DPI restore).
        std::time::Duration::from_millis(500)
    }

    fn persist_egui_memory(&self) -> bool {
        false
    }
}

fn order_by_favorite(processes: &mut [ProcessInfo], config: &Config) {
    if config.favorites.is_empty() {
        return;
    }
    processes.sort_by_key(|process| !config.is_favorite(&process.name));
}

fn resolve_selection(
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

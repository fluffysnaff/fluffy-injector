use crate::core::{icon_loader, process_scanner};
use crate::models::config::Config;
use crate::models::dll_manager::DLLManager;
use crate::models::process::ProcessInfo;
use crate::models::toast::{Toast, ToastLevel};
use crate::ui;
use eframe::egui::{ColorImage, Context, TextureHandle};
use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{Duration, Instant};

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
    pub auto_refresh: bool,
    pub icon_cache: HashMap<u32, TextureHandle>,
    pub default_dll_texture: Option<TextureHandle>,
    pub toasts: Vec<Toast>,
    
    // UI-specific state moved here
    pub process_search: String,

    // Private fields
    background_rx: Receiver<BackgroundMessage>,
    icon_tx: Sender<(u32, std::path::PathBuf)>,
    last_refresh: Instant,
}

impl InjectorApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let config = Config::load().unwrap_or_default();
        let mut dll_manager = DLLManager::new();
        for dll in &config.dlls {
            dll_manager.add(dll.clone());
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
            is_loading_processes: true,
            auto_refresh: true,
            last_refresh: Instant::now() - Duration::from_secs(10),
            icon_cache: HashMap::new(),
            default_dll_texture: None,
            toasts: Vec::new(),
            process_search: String::new(),
        };

        app.load_default_dll_icon(&cc.egui_ctx);
        app.try_select_last_app();
        app
    }

    fn handle_background_updates(&mut self, ctx: &Context) {
        while let Ok(message) = self.background_rx.try_recv() {
            match message {
                BackgroundMessage::Processes(procs) => {
                    self.processes = procs;
                    self.is_loading_processes = false;
                    self.try_select_last_app();
                    self.request_missing_icons();
                }
                BackgroundMessage::Icon((pid, color_image)) => {
                    let texture =
                        ctx.load_texture(format!("icon_{}", pid), color_image, Default::default());
                    self.icon_cache.insert(pid, texture);
                }
            }
        }
    }

    pub fn refresh_processes(&mut self) {
        if !self.is_loading_processes {
            self.is_loading_processes = true;
            self.last_refresh = Instant::now();
        }
    }

    fn request_missing_icons(&self) {
        for proc in &self.processes {
            if !proc.exe.as_os_str().is_empty() && !self.icon_cache.contains_key(&proc.pid) {
                let _ = self.icon_tx.send((proc.pid, proc.exe.clone()));
            }
        }
    }
    
    fn try_select_last_app(&mut self) {
        if let Some(last_app_name) = &self.config.last_selected_app {
            if let Some(proc) = self.processes.iter().find(|p| &p.name == last_app_name) {
                self.selected_process = Some(proc.pid);
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

        if self.auto_refresh
            && !self.is_loading_processes
            && self.last_refresh.elapsed() > Duration::from_secs(5)
        {
            self.last_refresh = Instant::now();
        }
        
        ui::show(ctx, self);
    }
}
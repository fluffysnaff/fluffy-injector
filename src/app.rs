use crate::core::{
    icon_loader::{self, IconData, IconRequest},
    injector, process_scanner,
};
use crate::models::{Config, Dll, Notice, ProcessInfo, StoredPlacement, ToastLevel};
use crate::settings::SettingsWriter;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread;
use std::time::{Duration, Instant};
use taut::{FileDialog, Picture, Theme, Ui, Waker};

const BACKGROUND_BATCH: usize = 16;
const MAX_ICON_REQUESTS: usize = 24;
const WORKER_CHECK: Duration = Duration::from_millis(150);

pub(crate) enum BackgroundMessage {
    Processes(Vec<ProcessInfo>),
    ScanError(String),
    SettingsSaved(u64, Result<(), String>),
}

enum IconState {
    Pending,
    Ready(Option<Picture>),
}

impl IconState {
    fn picture(&self) -> Option<Picture> {
        match self {
            Self::Ready(picture) => *picture,
            Self::Pending => None,
        }
    }
}

struct InjectionResult {
    total: usize,
    failures: Vec<String>,
    cleanup_warnings: Vec<String>,
}

pub(crate) struct InjectorApp {
    pub processes: Vec<ProcessInfo>,
    pub process_rows: Vec<(usize, bool)>,
    pub selected_process: Option<u32>,
    pub config: Config,
    pub is_loading_processes: bool,
    pub scan_failed: bool,
    icon_cache: HashMap<PathBuf, IconState>,
    dll_icon: Option<IconState>,
    pub process_search: String,
    pub dll_menu: Option<usize>,
    pub settings_writable: bool,
    theme: Theme,
    notices: Vec<Notice>,
    next_notice: u64,
    settings: Option<SettingsWriter>,
    close_after_save: Option<Instant>,
    files: FileDialog,
    background_rx: Receiver<BackgroundMessage>,
    background_tx: Sender<BackgroundMessage>,
    icon_worker: Option<icon_loader::Worker>,
    scan_stop: Option<Sender<()>>,
    injection: Option<Receiver<InjectionResult>>,
}

impl InjectorApp {
    pub(crate) fn new(config: Config) -> Self {
        let (background_tx, background_rx) = mpsc::channel();
        let mut files = FileDialog::native();
        files.only(&[".dll"]);

        Self {
            processes: Vec::new(),
            process_rows: Vec::new(),
            selected_process: None,
            config,
            is_loading_processes: true,
            scan_failed: false,
            icon_cache: HashMap::new(),
            dll_icon: None,
            process_search: String::new(),
            dll_menu: None,
            settings_writable: true,
            theme: crate::ui::theme(),
            notices: Vec::new(),
            next_notice: 0,
            settings: None,
            close_after_save: None,
            files,
            background_rx,
            background_tx,
            icon_worker: None,
            scan_stop: None,
            injection: None,
        }
    }

    pub(crate) fn describe(&mut self, ui: &mut Ui) {
        ui.theme = self.theme;
        let held_close = self.hold_close_for_save(ui);
        if self.scan_stop.is_none() {
            let (stop_tx, stop_rx) = mpsc::channel();
            self.scan_stop = Some(stop_tx);
            let process_tx = self.background_tx.clone();
            thread::spawn(move || process_scanner::scan_loop(process_tx, stop_rx));
            self.icon_worker = Some(icon_loader::Worker::spawn());
        }
        let mut received = 0;
        while received < BACKGROUND_BATCH {
            let Ok(message) = self.background_rx.try_recv() else {
                break;
            };
            received += 1;
            self.apply_background_message(ui, message);
        }
        if received == BACKGROUND_BATCH {
            ui.wake_after(Duration::ZERO);
        } else if self
            .settings
            .as_mut()
            .is_some_and(SettingsWriter::worker_stopped)
        {
            self.save_finished(Some("The settings writer stopped unexpectedly.".into()));
        }
        self.poll_icons(ui);
        self.poll_injection(ui);
        if self.request_icons() {
            ui.wake_after(WORKER_CHECK);
        }

        let mut changed = crate::ui::show(ui, self);
        if !self.files.shown && ui.popup_anchor().is_none() && ui.keys("Ctrl+O") {
            self.pick_dll();
        }
        if let Some(path) = ui.file_dialog(&mut self.files) {
            if self.add_dll(path) {
                changed = true;
                ui.rerun();
            }
        }
        if let Some(window) = ui.window_placement() {
            let placement = StoredPlacement {
                position: window.position,
                width: window.width,
                height: window.height,
                maximized: window.maximized,
            };
            if self.config.window != Some(placement) {
                self.config.window = Some(placement);
                changed = true;
            }
        }
        if changed && self.settings_writable {
            if !self
                .settings
                .get_or_insert_with(|| SettingsWriter::new(self.background_tx.clone()))
                .submit(&self.config)
            {
                self.save_finished(Some("The settings writer stopped unexpectedly.".into()));
            }
        }
        if self.settings.as_ref().is_some_and(SettingsWriter::saving) {
            ui.wake_after(WORKER_CHECK);
        }
        self.handle_close(ui, held_close);
        let now = Instant::now();
        self.notices.retain(|notice| notice.expires > now);
        crate::ui::show_notices(ui, &self.notices, now);
    }

    fn apply_background_message(&mut self, ui: &mut Ui, message: BackgroundMessage) {
        match message {
            BackgroundMessage::Processes(processes) => self.update_processes(ui, processes),
            BackgroundMessage::ScanError(error) => {
                self.is_loading_processes = false;
                self.scan_failed = true;
                self.add_toast(ToastLevel::Error, error);
            }
            BackgroundMessage::SettingsSaved(revision, result) => {
                if self
                    .settings
                    .as_mut()
                    .is_some_and(|writer| writer.acknowledge(revision, result.is_ok()))
                {
                    self.save_finished(
                        result
                            .err()
                            .map(|error| format!("Failed to save settings: {error}")),
                    );
                }
            }
        }
    }

    fn save_finished(&mut self, error: Option<String>) {
        if let Some(error) = error {
            self.close_after_save = None;
            self.add_toast(ToastLevel::Error, error);
        } else if self.close_after_save.is_some() {
            self.add_toast(ToastLevel::Success, "Settings saved.");
            self.close_after_save = Some(Instant::now() + Duration::from_secs(1));
        }
    }

    fn hold_close_for_save(&mut self, ui: &mut Ui) -> bool {
        if ui.close_requested() && self.settings.as_ref().is_some_and(SettingsWriter::saving) {
            ui.cancel_close("Saving settings");
            self.close_after_save = Some(Instant::now());
            return true;
        }
        false
    }

    fn handle_close(&mut self, ui: &mut Ui, held_close: bool) {
        if !held_close && ui.close_requested() {
            self.hold_close_for_save(ui);
        } else if let Some(until) = self.close_after_save {
            if self.settings.as_ref().is_some_and(SettingsWriter::unsaved) {
                return;
            }
            let left = until.saturating_duration_since(Instant::now());
            if left.is_zero() {
                ui.quit();
            } else {
                ui.wake_after(left);
            }
        }
    }

    fn update_processes(&mut self, ui: &mut Ui, processes: Vec<ProcessInfo>) {
        let live: HashSet<&Path> = processes.iter().map(|p| p.exe.as_path()).collect();
        self.icon_cache.retain(|path, state| {
            // Keep one request per path even if it disappears before the reply.
            let keep = matches!(state, IconState::Pending) || live.contains(path.as_path());
            if let (false, IconState::Ready(Some(picture))) = (keep, state) {
                ui.drop_picture(*picture);
            }
            keep
        });
        self.processes = processes;
        self.rebuild_process_rows();
        self.resolve_selection();
        self.is_loading_processes = false;
        self.scan_failed = false;
    }

    fn poll_icons(&mut self, ui: &mut Ui) {
        for _ in 0..BACKGROUND_BATCH {
            let Some(worker) = &self.icon_worker else {
                return;
            };
            match worker.results.try_recv() {
                Ok((request, data)) => self.apply_icon(ui, request, data),
                Err(TryRecvError::Empty) => return,
                Err(TryRecvError::Disconnected) => {
                    self.icon_worker = None;
                    self.icon_cache
                        .retain(|_, state| matches!(state, IconState::Ready(_)));
                    self.add_toast(ToastLevel::Warning, "The icon worker stopped unexpectedly.");
                    return;
                }
            }
        }
        ui.wake_after(Duration::ZERO);
    }

    fn apply_icon(&mut self, ui: &mut Ui, request: IconRequest, data: IconData) {
        let ready = || IconState::Ready(data.map(|(width, pixels)| ui.add_picture(width, pixels)));
        match request {
            IconRequest::Dll => self.dll_icon = Some(ready()),
            IconRequest::Process(path) => {
                if self.processes.iter().any(|process| process.exe == path) {
                    self.icon_cache.insert(path, ready());
                } else {
                    self.icon_cache.remove(&path);
                }
            }
        }
    }

    fn request_icons(&mut self) -> bool {
        let Some(worker) = &self.icon_worker else {
            return false;
        };
        let mut pending = self
            .icon_cache
            .values()
            .chain(self.dll_icon.iter())
            .filter(|state| matches!(state, IconState::Pending))
            .count();
        if self.dll_icon.is_none() && !self.config.dlls.is_empty() && pending < MAX_ICON_REQUESTS {
            if worker.requests.send(IconRequest::Dll).is_err() {
                return true;
            }
            self.dll_icon = Some(IconState::Pending);
            pending += 1;
        }
        for process in &self.processes {
            if pending == MAX_ICON_REQUESTS {
                break;
            }
            if process.exe.as_os_str().is_empty() || self.icon_cache.contains_key(&process.exe) {
                continue;
            }
            if worker
                .requests
                .send(IconRequest::Process(process.exe.clone()))
                .is_err()
            {
                return true;
            }
            self.icon_cache
                .insert(process.exe.clone(), IconState::Pending);
            pending += 1;
        }
        pending != 0
    }

    pub(crate) fn process_icon(&self, path: &Path) -> Option<Picture> {
        self.icon_cache.get(path).and_then(IconState::picture)
    }

    pub(crate) fn dll_icon(&self) -> Option<Picture> {
        self.dll_icon.as_ref().and_then(IconState::picture)
    }

    pub(crate) fn rebuild_process_rows(&mut self) {
        let query = self.process_search.as_bytes();
        self.process_rows.clear();
        self.process_rows
            .extend(
                self.processes
                    .iter()
                    .enumerate()
                    .filter_map(|(index, process)| {
                        let matches = query.is_empty()
                            || process
                                .name
                                .as_bytes()
                                .windows(query.len())
                                .any(|part| part.eq_ignore_ascii_case(query));
                        (matches && !self.config.is_blocked(&process.name))
                            .then(|| (index, self.config.is_favorite(&process.name)))
                    }),
            );
        self.process_rows.sort_by_key(|&(_, favorite)| !favorite);
    }

    pub(crate) fn resolve_selection(&mut self) {
        self.selected_process = self.remembered_matches().and_then(|mut matches| {
            if let Some(pid) = self
                .selected_process
                .filter(|&pid| matches.clone().any(|process| process.pid == pid))
            {
                return Some(pid);
            }
            let first = matches.next()?;
            matches.next().is_none().then_some(first.pid)
        });
    }

    pub(crate) fn multiple_matches(&self) -> bool {
        self.selected_process.is_none()
            && self
                .remembered_matches()
                .is_some_and(|mut matches| matches.nth(1).is_some())
    }

    fn remembered_matches(&self) -> Option<impl Iterator<Item = &ProcessInfo> + Clone> {
        let name = self
            .config
            .last_selected_app
            .as_deref()
            .filter(|name| !self.config.is_blocked(name))?;
        Some(
            self.processes
                .iter()
                .filter(move |process| process.name.eq_ignore_ascii_case(name)),
        )
    }

    pub(crate) fn selected_row(&self) -> Option<usize> {
        let pid = self.selected_process?;
        self.process_rows
            .iter()
            .position(|&(index, _)| self.processes[index].pid == pid)
    }

    pub(crate) fn selected_process_info(&self) -> Option<&ProcessInfo> {
        self.selected_process
            .and_then(|pid| self.processes.iter().find(|p| p.pid == pid))
    }

    pub(crate) fn add_toast(&mut self, level: ToastLevel, message: impl Into<String>) {
        self.notices.push(Notice {
            id: self.next_notice,
            level,
            message: message.into(),
            expires: Instant::now() + Duration::from_secs(3),
        });
        self.next_notice += 1;
    }

    pub(crate) fn pick_dll(&mut self) {
        let folder = if self.files.folder.is_dir() {
            self.files.folder.clone()
        } else {
            std::env::current_dir().unwrap_or_default()
        };
        self.files.pick(folder);
    }

    pub(crate) fn add_dll(&mut self, path: PathBuf) -> bool {
        match self.validate_dll(&path) {
            Ok(path) => self.config.dlls.push(Dll {
                path,
                selected: false,
            }),
            Err(error) => {
                self.add_toast(ToastLevel::Warning, format!("{}: {error}", path.display()));
                return false;
            }
        }
        true
    }

    fn validate_dll(&self, path: &Path) -> Result<String, &'static str> {
        if !path.is_file() {
            return Err("File not found.");
        }
        if !path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("dll"))
        {
            return Err("Choose a .dll file.");
        }
        let identity = path
            .canonicalize()
            .map_err(|_| "Could not resolve file path.")?;
        if self.config.dlls.iter().any(|dll| {
            Path::new(&dll.path)
                .canonicalize()
                .is_ok_and(|stored| stored == identity)
        }) {
            return Err("DLL is already in the list.");
        }
        let stored = if path.is_absolute() { path } else { &identity };
        stored
            .to_str()
            .map(str::to_owned)
            .ok_or("File path is not valid Unicode.")
    }

    pub(crate) fn selected_dlls(&self) -> impl Iterator<Item = &str> {
        self.config
            .dlls
            .iter()
            .filter_map(|dll| dll.selected.then_some(dll.path.as_str()))
    }

    pub(crate) fn start_injection(&mut self, dlls: Vec<String>) {
        let Some(pid) = self.selected_process.filter(|_| self.can_inject()) else {
            return;
        };
        if dlls.is_empty() {
            return;
        }
        let copy = self.config.copy_dll_on_inject;
        let randomize = self.config.randomize_dll_name;
        let (tx, rx) = mpsc::channel();
        self.injection = Some(rx);
        thread::spawn(move || {
            let _ = tx.send(inject_dlls(pid, &dlls, copy, randomize));
            Waker.wake();
        });
    }

    pub(crate) fn is_injecting(&self) -> bool {
        self.injection.is_some()
    }

    pub(crate) fn can_inject(&self) -> bool {
        !self.is_loading_processes
            && !self.scan_failed
            && !self.is_injecting()
            && self.selected_process.is_some()
    }

    fn poll_injection(&mut self, ui: &mut Ui) {
        let Some(rx) = &self.injection else { return };
        match rx.try_recv() {
            Ok(result) => self.finish_injection(result),
            Err(TryRecvError::Disconnected) => {
                self.add_toast(
                    ToastLevel::Error,
                    "The injection worker stopped unexpectedly.",
                );
            }
            Err(TryRecvError::Empty) => {
                ui.wake_after(WORKER_CHECK);
                return;
            }
        }
        self.injection = None;
    }

    fn finish_injection(&mut self, result: InjectionResult) {
        let InjectionResult {
            total,
            failures,
            cleanup_warnings,
        } = result;
        if !cleanup_warnings.is_empty() {
            self.add_toast(ToastLevel::Warning, cleanup_warnings.join("; "));
        }
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

fn inject_dlls(pid: u32, dlls: &[String], copy: bool, randomize: bool) -> InjectionResult {
    let mut failures = Vec::new();
    let mut cleanup_warnings = Vec::new();
    for path in dlls {
        let (result, warning) = injector::inject_dll(pid, path, copy, randomize);
        cleanup_warnings.extend(warning);
        if let Err(error) = result {
            let name = Path::new(path)
                .file_name()
                .unwrap_or_default()
                .to_string_lossy();
            failures.push(format!("{name}: {error}"));
        }
    }
    InjectionResult {
        total: dlls.len(),
        failures,
        cleanup_warnings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use taut::testing::Trial;

    #[test]
    fn in_flight_icon_is_not_requeued_and_absent_result_is_discarded() {
        let mut trial = Trial::new(InjectorApp::new(Config::default()), |_, _| {});
        let (app, ui) = (&mut trial.app, &mut trial.ui);
        let (requests, rx) = mpsc::channel();
        let (tx, results) = mpsc::channel();
        app.icon_worker = Some(icon_loader::Worker { requests, results });
        let path = PathBuf::from("test.exe");
        let rows = vec![ProcessInfo {
            name: "test.exe".into(),
            pid: 1,
            exe: path.clone(),
        }];
        app.update_processes(ui, rows.clone());
        assert!(app.request_icons());
        let request = rx.try_recv().unwrap();

        app.update_processes(ui, vec![]);
        app.update_processes(ui, rows.clone());
        assert!(app.request_icons());
        assert!(matches!(rx.try_recv(), Err(TryRecvError::Empty)));

        app.update_processes(ui, vec![]);
        tx.send((request, Some((1, vec![0])))).unwrap();
        app.poll_icons(ui);
        assert!(!app.icon_cache.contains_key(&path));
        app.update_processes(ui, rows);
        assert!(app.request_icons());
        tx.send((rx.try_recv().unwrap(), None)).unwrap();
        app.poll_icons(ui);
        assert!(!app.request_icons(), "a live path caches a negative result");
    }
}

use anyhow::{Context, Result};
use eframe::{Storage, APP_KEY};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::path::PathBuf;
use std::time::{Duration, Instant};

pub(crate) const DEFAULT_WINDOW_SIZE: [f32; 2] = [680.0, 450.0];
pub(crate) const MIN_WINDOW_SIZE: [f32; 2] = [420.0, 300.0];
pub(crate) const APP_NAME: &str = "Fluffy Injector";
pub(crate) const DEFAULT_SPLIT_RATIO: f32 = 0.42;

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Config {
    pub dlls: Vec<Dll>,
    pub last_selected_app: Option<String>,
    #[serde(default)]
    pub copy_dll_on_inject: bool,
    #[serde(default)]
    pub randomize_dll_name: bool,
    /// Fraction of the body width used by the process list (0..=1).
    #[serde(default = "default_split_ratio")]
    pub split_ratio: f32,
    #[serde(default)]
    pub favorites: Vec<String>,
    #[serde(default)]
    pub blocked: Vec<String>,
}

fn default_split_ratio() -> f32 {
    DEFAULT_SPLIT_RATIO
}

impl Default for Config {
    fn default() -> Self {
        Self {
            dlls: Vec::new(),
            last_selected_app: None,
            copy_dll_on_inject: false,
            randomize_dll_name: false,
            split_ratio: DEFAULT_SPLIT_RATIO,
            favorites: Vec::new(),
            blocked: Vec::new(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Dll {
    pub path: String,
    pub selected: bool,
}

impl Config {
    pub(crate) fn load(storage: Option<&dyn Storage>) -> Self {
        let mut config: Self = storage
            .and_then(|storage| storage.get_string(APP_KEY))
            .and_then(|data| ron::from_str(&data).ok())
            .unwrap_or_default();
        sort_names_ascii(&mut config.blocked);
        sort_names_ascii(&mut config.favorites);
        config
    }

    pub(crate) fn save(&self, storage: &mut dyn Storage) -> Result<()> {
        let data = ron::to_string(self).context("Failed to serialize settings")?;
        storage.set_string(APP_KEY, data);
        storage.flush();
        Ok(())
    }

    pub(crate) fn is_favorite(&self, name: &str) -> bool {
        contains_sorted(&self.favorites, name)
    }

    pub(crate) fn is_blocked(&self, name: &str) -> bool {
        contains_sorted(&self.blocked, name)
    }

    pub(crate) fn toggle_favorite(&mut self, name: &str) {
        if remove_sorted(&mut self.favorites, name) {
            return;
        }
        insert_sorted(&mut self.favorites, name.to_owned());
    }

    pub(crate) fn block_process(&mut self, name: &str) {
        remove_sorted(&mut self.favorites, name);
        insert_sorted(&mut self.blocked, name.to_owned());
    }

    pub(crate) fn unblock_at(&mut self, index: usize) {
        if index < self.blocked.len() {
            self.blocked.remove(index);
        }
    }
}

fn cmp_names_ascii(left: &str, right: &str) -> Ordering {
    left.bytes()
        .map(|byte| byte.to_ascii_lowercase())
        .cmp(right.bytes().map(|byte| byte.to_ascii_lowercase()))
}

fn sort_names_ascii(names: &mut [String]) {
    names.sort_unstable_by(|left, right| cmp_names_ascii(left, right));
}

fn contains_sorted(names: &[String], name: &str) -> bool {
    names.binary_search_by(|entry| cmp_names_ascii(entry, name)).is_ok()
}

fn insert_sorted(names: &mut Vec<String>, name: String) {
    if let Err(index) = names.binary_search_by(|entry| cmp_names_ascii(entry, &name)) {
        names.insert(index, name);
    }
}

fn remove_sorted(names: &mut Vec<String>, name: &str) -> bool {
    match names.binary_search_by(|entry| cmp_names_ascii(entry, name)) {
        Ok(index) => {
            names.remove(index);
            true
        }
        Err(_) => false,
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct ProcessInfo {
    pub name: String,
    pub pid: u32,
    pub exe: PathBuf,
}

pub(crate) enum ToastLevel {
    Info,
    Success,
    Warning,
    Error,
}

pub(crate) struct Toast {
    pub level: ToastLevel,
    pub message: String,
    created_at: Instant,
}

impl Toast {
    pub(crate) fn new(level: ToastLevel, message: impl Into<String>) -> Self {
        Self {
            level,
            message: message.into(),
            created_at: Instant::now(),
        }
    }

    pub(crate) fn is_alive(&self) -> bool {
        self.created_at.elapsed() < Duration::from_secs(3)
    }
}

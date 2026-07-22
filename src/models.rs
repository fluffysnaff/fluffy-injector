use anyhow::{Context, Result};
use eframe::{Storage, APP_KEY};
use serde::{Deserialize, Serialize};
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
        storage
            .and_then(|storage| storage.get_string(APP_KEY))
            .and_then(|data| ron::from_str(&data).ok())
            .unwrap_or_default()
    }

    pub(crate) fn save(&self, storage: &mut dyn Storage) -> Result<()> {
        let data = ron::to_string(self).context("Failed to serialize settings")?;
        storage.set_string(APP_KEY, data);
        storage.flush();
        Ok(())
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

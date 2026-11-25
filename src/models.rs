use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;
use taut::storage;

pub(crate) const DEFAULT_WINDOW_SIZE: [f32; 2] = [680.0, 450.0];
pub(crate) const MIN_WINDOW_SIZE: [f32; 2] = [420.0, 300.0];
pub(crate) const APP_NAME: &str = "Fluffy Injector";
pub(crate) const DEFAULT_SPLIT_RATIO: f32 = 0.42;

#[derive(Clone, Debug, Serialize, Deserialize)]
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
    pub window: Option<StoredPlacement>,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct StoredPlacement {
    pub position: (i32, i32),
    pub width: f32,
    pub height: f32,
    pub maximized: bool,
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
            window: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Dll {
    pub path: String,
    pub selected: bool,
}

impl Config {
    pub(crate) fn load() -> Result<(Self, Option<String>)> {
        let path = config_path().context("Could not locate the settings folder")?;
        let legacy = storage::folder(APP_NAME).map(|folder| folder.join("data").join("app.ron"));
        Self::load_from(&path, legacy.as_deref())
    }

    fn load_from(path: &Path, legacy: Option<&Path>) -> Result<(Self, Option<String>)> {
        let (mut config, imported) = match fs::read(path) {
            Ok(data) => {
                let parsed = ron::de::from_bytes(&data).with_context(|| {
                    let backup = path.with_extension("ron.bak");
                    let detail = match File::create_new(&backup)
                        .and_then(|mut file| file.write_all(&data))
                    {
                        Ok(()) => format!("copy at {}", backup.display()),
                        Err(error) => format!("backup unavailable: {error}"),
                    };
                    format!("Invalid settings ({detail})")
                })?;
                (parsed, false)
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                match legacy.map(fs::read_to_string) {
                    Some(Ok(data)) => (decode_legacy(&data)?, true),
                    Some(Err(error)) if error.kind() != io::ErrorKind::NotFound => {
                        return Err(error).context("Could not read legacy settings");
                    }
                    _ => return Ok((Self::default(), None)),
                }
            }
            Err(error) => return Err(error).context("Could not read settings"),
        };
        sort_names(&mut config.blocked);
        sort_names(&mut config.favorites);
        let warning = if imported {
            config
                .save_to(path)
                .err()
                .map(|error| format!("Could not save imported settings: {error:#}"))
        } else {
            None
        };
        Ok((config, warning))
    }

    pub(crate) fn save(&self) -> Result<()> {
        let path = config_path().context("Could not locate the settings folder")?;
        self.save_to(&path)
    }

    fn save_to(&self, path: &Path) -> Result<()> {
        let data = ron::to_string(self).context("Failed to serialize settings")?;
        storage::write(path, &data).context("Failed to write settings")
    }

    pub(crate) fn is_favorite(&self, name: &str) -> bool {
        name_index(&self.favorites, name).is_some()
    }

    pub(crate) fn is_blocked(&self, name: &str) -> bool {
        name_index(&self.blocked, name).is_some()
    }

    pub(crate) fn toggle_favorite(&mut self, name: &str) {
        if self.is_blocked(name) || remove_name(&mut self.favorites, name) {
            return;
        }
        insert_name(&mut self.favorites, name);
    }

    pub(crate) fn block_process(&mut self, name: &str) {
        remove_name(&mut self.favorites, name);
        if !self.is_blocked(name) {
            insert_name(&mut self.blocked, name);
        }
    }

    pub(crate) fn unblock_at(&mut self, index: usize) {
        if index < self.blocked.len() {
            self.blocked.remove(index);
        }
    }
}

fn decode_legacy(data: &str) -> Result<Config> {
    let entries: HashMap<String, String> =
        ron::from_str(data).context("Invalid legacy settings map")?;
    let app = entries
        .get("app")
        .context("Legacy settings have no app entry")?;
    ron::from_str(app).context("Invalid legacy app settings")
}

fn config_path() -> Option<PathBuf> {
    Some(storage::folder("fluffy-injector")?.join("config.ron"))
}

fn name_index(names: &[String], name: &str) -> Option<usize> {
    names
        .iter()
        .position(|entry| entry.eq_ignore_ascii_case(name))
}

fn insert_name(names: &mut Vec<String>, name: &str) {
    names.push(name.to_owned());
    sort_names(names);
}

fn remove_name(names: &mut Vec<String>, name: &str) -> bool {
    let Some(index) = name_index(names, name) else {
        return false;
    };
    names.remove(index);
    true
}

fn sort_names(names: &mut [String]) {
    names.sort_unstable_by(|left, right| {
        left.bytes()
            .map(|byte| byte.to_ascii_lowercase())
            .cmp(right.bytes().map(|byte| byte.to_ascii_lowercase()))
    });
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

pub(crate) struct Notice {
    pub id: u64,
    pub level: ToastLevel,
    pub message: String,
    pub expires: Instant,
}

#[cfg(test)]
mod tests {
    use super::*;

    const LEGACY: &str = r#"{
        "egui": "()",
        "app": "(dlls:[(path:\"C:\\\\Mods\\\\example.dll\",selected:true)],last_selected_app:Some(\"Game.exe\"),favorites:[\"Game.exe\"])"
    }"#;

    struct ConfigFiles(PathBuf);

    impl ConfigFiles {
        fn new() -> Self {
            let dir = std::env::temp_dir().join(format!(
                "fluffy-settings-test-{}-{}",
                std::process::id(),
                std::random::random::<u64>(..)
            ));
            fs::create_dir(&dir).unwrap();
            Self(dir)
        }
    }

    impl Drop for ConfigFiles {
        fn drop(&mut self) {
            for name in ["app.ron", "config.ron", "config.ron.bak"] {
                let _ = fs::remove_file(self.0.join(name));
            }
            let _ = fs::remove_dir(&self.0);
        }
    }

    #[test]
    fn imports_legacy_storage_map_without_modifying_it() {
        let dir = ConfigFiles::new();
        let settings = dir.0.join("config.ron");
        let legacy = dir.0.join("app.ron");
        fs::write(&legacy, LEGACY).unwrap();
        let (imported, warning) = Config::load_from(&settings, Some(&legacy)).unwrap();
        assert!(warning.is_none());
        assert_eq!(imported.last_selected_app.as_deref(), Some("Game.exe"));
        assert_eq!(imported.dlls[0].path, r"C:\Mods\example.dll");
        assert!(imported.dlls[0].selected);
        assert!(imported.is_favorite("game.exe"));
        assert_eq!(imported.window, None);
        assert_eq!(fs::read_to_string(&legacy).unwrap(), LEGACY);
        let saved: Config = ron::de::from_bytes(&fs::read(settings).unwrap()).unwrap();
        assert_eq!(saved.last_selected_app, imported.last_selected_app);
    }

    #[test]
    fn backs_up_malformed_settings_without_falling_back_to_legacy() {
        let dir = ConfigFiles::new();
        let settings = dir.0.join("config.ron");
        let legacy = dir.0.join("app.ron");
        let bad = b"invalid RON \xff";
        fs::write(&settings, bad).unwrap();
        fs::write(&legacy, LEGACY).unwrap();
        assert!(Config::load_from(&settings, Some(&legacy)).is_err());
        assert_eq!(fs::read(&settings).unwrap(), bad);
        assert_eq!(fs::read(settings.with_extension("ron.bak")).unwrap(), bad);
    }

    #[test]
    fn block_and_favorite_transitions_are_case_insensitive() {
        let mut config = Config::default();
        let steps: [(fn(&mut Config), bool, bool); 8] = [
            (|c| c.toggle_favorite("Game.exe"), true, false),
            (|c| c.toggle_favorite("GAME.EXE"), false, false),
            (|c| c.toggle_favorite("game.exe"), true, false),
            (|c| c.block_process("GAME.exe"), false, true),
            (|c| c.block_process("game.EXE"), false, true),
            (|c| c.toggle_favorite("GAME.EXE"), false, true),
            (|c| c.unblock_at(0), false, false),
            (|c| c.toggle_favorite("Game.exe"), true, false),
        ];
        for (index, (apply, favorite, blocked)) in steps.into_iter().enumerate() {
            apply(&mut config);
            assert_eq!(config.is_favorite("gAmE.eXe"), favorite, "step {index}");
            assert_eq!(config.is_blocked("gAmE.eXe"), blocked, "step {index}");
            assert_eq!(
                config.favorites.len(),
                usize::from(favorite),
                "step {index}"
            );
            assert_eq!(config.blocked.len(), usize::from(blocked), "step {index}");
        }
    }
}

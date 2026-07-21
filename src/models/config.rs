use anyhow::{Context, Result};
use eframe::{Storage, APP_KEY};
use serde::{Deserialize, Serialize};

pub(crate) const DEFAULT_WINDOW_SIZE: [f32; 2] = [800.0, 600.0];
pub(crate) const MIN_WINDOW_SIZE: [f32; 2] = [600.0, 400.0];

#[derive(Debug, Default, Serialize, Deserialize)]
pub(crate) struct Config {
    pub dlls: Vec<Dll>,
    pub last_selected_app: Option<String>,
    #[serde(default)]
    pub copy_dll_on_inject: bool,
    #[serde(default)]
    pub randomize_dll_name: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Dll {
    pub path: String,
    pub selected: bool,
}

impl Config {
    pub(crate) fn load(storage: Option<&dyn Storage>) -> Result<Self> {
        let Some(data) = storage.and_then(|storage| storage.get_string(APP_KEY)) else {
            return Ok(Self::default());
        };
        ron::from_str(&data).context("Failed to parse persisted settings")
    }

    pub(crate) fn save(&self, storage: &mut dyn Storage) -> Result<()> {
        let data = ron::to_string(self).context("Failed to serialize settings")?;
        storage.set_string(APP_KEY, data);
        Ok(())
    }
}

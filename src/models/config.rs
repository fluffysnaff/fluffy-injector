use anyhow::{Context, Result};
use eframe::{Storage, APP_KEY};
use serde::{Deserialize, Serialize};

pub(crate) const DEFAULT_WINDOW_SIZE: [f32; 2] = [800.0, 600.0];
pub(crate) const MIN_WINDOW_SIZE: [f32; 2] = [600.0, 400.0];

#[derive(Serialize, Deserialize, Default)]
pub(crate) struct Config {
    pub dlls: Vec<String>,
    pub last_selected_app: Option<String>,
    #[serde(default)]
    pub selected_dlls: Vec<String>,
    #[serde(default)]
    pub copy_dll_on_inject: bool,
    #[serde(default)]
    pub randomize_dll_name: bool,
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

#[cfg(test)]
mod tests {
    use super::Config;

    #[test]
    fn missing_new_settings_use_defaults() {
        let config: Config = ron::from_str("(dlls:[],last_selected_app:None)").unwrap();
        assert!(config.selected_dlls.is_empty());
        assert!(!config.copy_dll_on_inject);
        assert!(!config.randomize_dll_name);
    }

    #[test]
    fn persisted_settings_round_trip() {
        let config = Config {
            selected_dlls: vec!["a.dll".into(), "b.dll".into()],
            randomize_dll_name: true,
            ..Default::default()
        };
        let data = ron::to_string(&config).unwrap();
        let restored: Config = ron::from_str(&data).unwrap();

        assert_eq!(restored.selected_dlls, config.selected_dlls);
        assert_eq!(restored.randomize_dll_name, config.randomize_dll_name);
    }
}

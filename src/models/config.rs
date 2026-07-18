use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Serialize, Deserialize, Default)]
pub struct Config {
    pub dlls: Vec<String>,
    pub last_selected_app: Option<String>,
    #[serde(default)]
    pub selected_dlls: Vec<String>,
    #[serde(default)]
    pub copy_dll_on_inject: bool,
}

impl Config {
    pub fn load() -> Result<Self> {
        let path = "config.json";
        if Path::new(path).exists() {
            let data = std::fs::read_to_string(path).context("Failed to read config file")?;
            serde_json::from_str(&data).context("Failed to parse config JSON")
        } else {
            Ok(Config::default())
        }
    }

    pub fn save(&self) -> Result<()> {
        let data = serde_json::to_string_pretty(self).context("Failed to serialize config")?;
        std::fs::write("config.json", data).context("Failed to write config file")
    }
}

#[cfg(test)]
mod tests {
    use super::Config;

    #[test]
    fn missing_new_settings_use_defaults() {
        let config: Config =
            serde_json::from_str(r#"{"dlls":[],"last_selected_app":null}"#).unwrap();
        assert!(config.selected_dlls.is_empty());
        assert!(!config.copy_dll_on_inject);
    }

    #[test]
    fn selected_dlls_round_trip() {
        let config = Config {
            selected_dlls: vec!["a.dll".into(), "b.dll".into()],
            ..Default::default()
        };
        let json = serde_json::to_string(&config).unwrap();
        let restored: Config = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.selected_dlls, config.selected_dlls);
    }
}

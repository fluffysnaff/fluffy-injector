use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

pub const DEFAULT_WINDOW_SIZE: [f32; 2] = [800.0, 600.0];
pub const MIN_WINDOW_SIZE: [f32; 2] = [600.0, 400.0];

#[derive(Serialize, Deserialize, Default)]
pub struct Config {
    pub dlls: Vec<String>,
    pub last_selected_app: Option<String>,
    #[serde(default)]
    pub selected_dlls: Vec<String>,
    #[serde(default)]
    pub copy_dll_on_inject: bool,
    #[serde(default)]
    pub window_position: Option<[f32; 2]>,
    #[serde(default)]
    pub window_size: Option<[f32; 2]>,
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

    pub fn saved_window_position(&self) -> Option<[f32; 2]> {
        self.window_position
            .filter(|position| position.iter().all(|value| value.is_finite()))
    }

    pub fn saved_window_size(&self) -> Option<[f32; 2]> {
        self.window_size.filter(|size| {
            size[0].is_finite()
                && size[1].is_finite()
                && size[0] >= MIN_WINDOW_SIZE[0]
                && size[1] >= MIN_WINDOW_SIZE[1]
        })
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
        assert!(config.window_position.is_none());
        assert!(config.window_size.is_none());
    }

    #[test]
    fn persisted_settings_round_trip() {
        let config = Config {
            selected_dlls: vec!["a.dll".into(), "b.dll".into()],
            window_position: Some([120.0, 80.0]),
            window_size: Some([900.0, 700.0]),
            ..Default::default()
        };
        let json = serde_json::to_string(&config).unwrap();
        let restored: Config = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.selected_dlls, config.selected_dlls);
        assert_eq!(restored.window_position, config.window_position);
        assert_eq!(restored.window_size, config.window_size);
    }

    #[test]
    fn rejects_invalid_window_geometry() {
        let config = Config {
            window_position: Some([f32::NAN, 10.0]),
            window_size: Some([100.0, 100.0]),
            ..Default::default()
        };

        assert!(config.saved_window_position().is_none());
        assert!(config.saved_window_size().is_none());
    }
}

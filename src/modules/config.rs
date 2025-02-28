use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::Path;

#[derive(Serialize, Deserialize, Default)]
pub struct Config {
    pub dlls: Vec<String>,
    pub last_selected_app: Option<String>,
}

impl Config {
    pub fn load() -> Self {
        let path = "config.json";
        if Path::new(path).exists() {
            let data = fs::read_to_string(path).unwrap_or_else(|_| "".to_string());
            if let Ok(config) = serde_json::from_str::<Config>(&data) {
                return config;
            }
        }
        Config::default()
    }

    pub fn save(&self) -> io::Result<()> {
        let data = serde_json::to_string_pretty(self)?;
        fs::write("config.json", data)
    }
}

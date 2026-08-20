use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::sync::watch;

const SETTINGS_FILE: &str = "settings.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub max_heart_rate: u16,
    pub allowed_devices: String,
    pub server_port: u16,
    pub auto_start: bool,
    pub minimize_to_tray: bool,
    #[serde(default)]
    pub language: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            max_heart_rate: 190,
            allowed_devices: "band,amazfit,watch,mi".into(),
            server_port: 3030,
            auto_start: false,
            minimize_to_tray: true,
            language: "zh".into(),
        }
    }
}

impl Config {
    fn settings_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("band-heart-rate")
            .join(SETTINGS_FILE)
    }

    pub fn load() -> Self {
        let path = Self::settings_path();
        match std::fs::read_to_string(&path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::settings_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }
}

pub struct ConfigManager {
    pub tx: watch::Sender<Config>,
    pub rx: watch::Receiver<Config>,
}

impl ConfigManager {
    pub fn new() -> Self {
        let config = Config::load();
        let (tx, rx) = watch::channel(config);
        Self { tx, rx }
    }
}

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct AppConfig {
    pub base_url: String,
    pub auth_token: String,
    pub upload_folder: String,
    pub auto_copy_format: String,
    pub show_floating: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            base_url: String::new(),
            auth_token: String::new(),
            upload_folder: String::new(),
            auto_copy_format: "raw".to_string(),
            show_floating: true,
        }
    }
}

fn config_path() -> PathBuf {
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("satellite");
    fs::create_dir_all(&config_dir).ok();
    config_dir.join("config.json")
}

pub fn load_config() -> AppConfig {
    let path = config_path();
    match fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => AppConfig::default(),
    }
}

pub fn store_config(config: &AppConfig) -> Result<(), String> {
    let path = config_path();
    let json = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| format!("Failed to write config: {}", e))
}

#[tauri::command]
pub fn get_config() -> AppConfig {
    load_config()
}

#[tauri::command]
pub fn save_config(
    base_url: String,
    auth_token: String,
    upload_folder: String,
    auto_copy_format: String,
    show_floating: bool,
) -> Result<(), String> {
    let config = AppConfig {
        base_url,
        auth_token,
        upload_folder,
        auto_copy_format,
        show_floating,
    };
    store_config(&config)
}

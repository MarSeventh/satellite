use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct AppConfig {
    pub base_url: String,
    pub auth_token: String,
    pub upload_folder: String,
    pub upload_channel: String,
    pub channel_name: String,
    pub auto_copy_format: String,
    pub show_floating: bool,
    /// "none" | "system" | "custom"
    pub proxy_mode: String,
    /// Custom HTTP proxy URL, e.g. http://127.0.0.1:7890
    pub proxy_url: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            base_url: String::new(),
            auth_token: String::new(),
            upload_folder: String::new(),
            upload_channel: String::new(),
            channel_name: String::new(),
            auto_copy_format: "raw".to_string(),
            show_floating: true,
            proxy_mode: "none".to_string(),
            proxy_url: String::new(),
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

/// Build a reqwest::Client respecting the proxy settings in config.
pub fn build_http_client(config: &AppConfig) -> Result<reqwest::Client, String> {
    let mut builder = reqwest::Client::builder();

    match config.proxy_mode.as_str() {
        "system" => {
            // reqwest default behavior uses system proxy, so just don't call no_proxy
        }
        "custom" => {
            if !config.proxy_url.is_empty() {
                let proxy = reqwest::Proxy::all(&config.proxy_url)
                    .map_err(|e| format!("Invalid proxy URL: {}", e))?;
                builder = builder.proxy(proxy);
            }
        }
        _ => {
            // "none" — disable all proxies
            builder = builder.no_proxy();
        }
    }

    builder.build().map_err(|e| format!("Failed to build HTTP client: {}", e))
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
    upload_channel: String,
    channel_name: String,
    auto_copy_format: String,
    show_floating: bool,
    proxy_mode: String,
    proxy_url: String,
) -> Result<(), String> {
    let config = AppConfig {
        base_url,
        auth_token,
        upload_folder,
        upload_channel,
        channel_name,
        auto_copy_format,
        show_floating,
        proxy_mode,
        proxy_url,
    };
    store_config(&config)
}

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub watch_folder: Option<String>,
    pub watch_folder_enabled: bool,
    pub last_export_dir: Option<String>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            watch_folder: None,
            watch_folder_enabled: false,
            last_export_dir: None,
        }
    }
}

pub fn settings_path() -> Result<PathBuf, String> {
    let base = std::env::var("APPDATA").map_err(|err| format!("APPDATA is unavailable: {err}"))?;
    Ok(PathBuf::from(base).join("Cutdown").join("settings.json"))
}

pub fn load_settings() -> AppSettings {
    let path = match settings_path() {
        Ok(path) => path,
        Err(_) => return AppSettings::default(),
    };

    if !path.exists() {
        return AppSettings::default();
    }

    let raw = match fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(_) => return AppSettings::default(),
    };

    serde_json::from_str(&raw).unwrap_or_default()
}

pub fn save_settings(settings: &AppSettings) -> Result<(), String> {
    let path = settings_path()?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("Failed to create settings directory: {err}"))?;
    }

    let raw = serde_json::to_string_pretty(settings)
        .map_err(|err| format!("Failed to serialize settings: {err}"))?;

    fs::write(&path, raw).map_err(|err| format!("Failed to write settings: {err}"))
}

#[tauri::command]
pub fn get_settings() -> AppSettings {
    load_settings()
}

pub fn update_watch_folder_settings(path: Option<String>, enabled: bool) -> Result<AppSettings, String> {
    let mut settings = load_settings();
    settings.watch_folder = path.filter(|value| !value.trim().is_empty());
    settings.watch_folder_enabled = enabled && settings.watch_folder.is_some();
    save_settings(&settings)?;
    Ok(settings.clone())
}

#[tauri::command]
pub fn set_last_export_dir(path: String) -> Result<(), String> {
    let mut settings = load_settings();
    settings.last_export_dir = Some(path);
    save_settings(&settings)
}

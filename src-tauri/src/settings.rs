use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub watch_folder: Option<String>,
    pub watch_folder_enabled: bool,
    pub last_export_dir: Option<String>,
    pub default_export_dir: Option<String>,
    pub last_preset_id: String,
    pub prefer_gpu_encoding: bool,
    pub run_at_startup: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            watch_folder: None,
            watch_folder_enabled: false,
            last_export_dir: None,
            default_export_dir: None,
            last_preset_id: crate::presets::PRESET_LOSSLESS.to_string(),
            prefer_gpu_encoding: true,
            run_at_startup: false,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSettingsParams {
    pub watch_folder: Option<String>,
    pub watch_folder_enabled: bool,
    pub default_export_dir: Option<String>,
    pub last_preset_id: Option<String>,
    pub prefer_gpu_encoding: bool,
    pub run_at_startup: bool,
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
    settings.last_export_dir = Some(path.clone());
    if settings.default_export_dir.is_none() {
        settings.default_export_dir = Some(path);
    }
    save_settings(&settings)
}

#[tauri::command]
pub fn set_last_preset_id(preset_id: String) -> Result<(), String> {
    let mut settings = load_settings();
    settings.last_preset_id = preset_id;
    save_settings(&settings)
}

#[tauri::command]
pub fn update_settings(params: UpdateSettingsParams) -> Result<AppSettings, String> {
    let mut settings = load_settings();
    settings.watch_folder = params
        .watch_folder
        .filter(|value| !value.trim().is_empty());
    settings.watch_folder_enabled =
        params.watch_folder_enabled && settings.watch_folder.is_some();
    settings.default_export_dir = params
        .default_export_dir
        .filter(|value| !value.trim().is_empty());
    settings.prefer_gpu_encoding = params.prefer_gpu_encoding;
    settings.run_at_startup = params.run_at_startup;

    if let Some(preset_id) = params.last_preset_id.filter(|value| !value.trim().is_empty()) {
        settings.last_preset_id = preset_id;
    }

    crate::windows_integration::set_run_at_startup(params.run_at_startup)?;
    save_settings(&settings)?;
    Ok(settings)
}

use crate::presets::{self, normalize_custom_export_presets};
use crate::upload_providers::{
    migrate_upload_providers, normalize_settings_providers, UploadProvider,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomExportPreset {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub lossless: bool,
    #[serde(default = "default_custom_mode")]
    pub mode: String,
    pub video_bitrate_kbps: Option<u64>,
    pub audio_bitrate_kbps: Option<u64>,
    pub crf: Option<u32>,
    pub max_width: Option<u32>,
    pub max_height: Option<u32>,
    pub target_bytes: Option<u64>,
    pub encoder_speed: Option<String>,
}

fn default_custom_mode() -> String {
    "bitrate".to_string()
}

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
    #[serde(default)]
    pub start_minimized_to_tray: bool,
    #[serde(default)]
    pub catbox_user_hash: Option<String>,
    #[serde(default)]
    pub catbox_api_url: Option<String>,
    #[serde(default)]
    pub recent_sources: Vec<String>,
    #[serde(default)]
    pub upload_providers: Vec<UploadProvider>,
    #[serde(default)]
    pub default_upload_provider_id: Option<String>,
    #[serde(default)]
    pub custom_export_presets: Vec<CustomExportPreset>,
    #[serde(default)]
    pub obs_websocket_host: Option<String>,
    #[serde(default)]
    pub obs_websocket_port: Option<u16>,
    #[serde(default)]
    pub obs_websocket_password: Option<String>,
}

const MAX_RECENT_SOURCES: usize = 10;

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
            start_minimized_to_tray: false,
            catbox_user_hash: None,
            catbox_api_url: None,
            recent_sources: Vec::new(),
            upload_providers: Vec::new(),
            default_upload_provider_id: None,
            custom_export_presets: Vec::new(),
            obs_websocket_host: Some("127.0.0.1".to_string()),
            obs_websocket_port: Some(4455),
            obs_websocket_password: None,
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
    pub catbox_user_hash: Option<String>,
    pub catbox_api_url: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveEditorSettingsParams {
    pub watch_folder: Option<String>,
    pub watch_folder_enabled: bool,
    pub default_export_dir: Option<String>,
    pub last_preset_id: Option<String>,
    pub prefer_gpu_encoding: bool,
    pub run_at_startup: bool,
    pub start_minimized_to_tray: bool,
    pub providers: Vec<UploadProvider>,
    pub default_upload_provider_id: Option<String>,
    pub custom_export_presets: Vec<CustomExportPreset>,
    pub obs_websocket_host: Option<String>,
    pub obs_websocket_port: Option<u16>,
    pub obs_websocket_password: Option<String>,
}

pub fn apply_editor_settings(params: SaveEditorSettingsParams) -> Result<AppSettings, String> {
    let mut settings = load_settings();
    settings.watch_folder = params.watch_folder.filter(|value| !value.trim().is_empty());
    settings.watch_folder_enabled = params.watch_folder_enabled && settings.watch_folder.is_some();
    settings.default_export_dir = params
        .default_export_dir
        .filter(|value| !value.trim().is_empty());
    settings.prefer_gpu_encoding = params.prefer_gpu_encoding;
    settings.run_at_startup = params.run_at_startup;
    settings.start_minimized_to_tray = params.start_minimized_to_tray;
    settings.upload_providers = params.providers;
    settings.default_upload_provider_id = params
        .default_upload_provider_id
        .filter(|value| !value.trim().is_empty());
    settings.custom_export_presets = params.custom_export_presets;
    settings.obs_websocket_host = params
        .obs_websocket_host
        .filter(|value| !value.trim().is_empty());
    settings.obs_websocket_port = params.obs_websocket_port;
    settings.obs_websocket_password = params
        .obs_websocket_password
        .filter(|value| !value.trim().is_empty());

    if let Some(preset_id) = params
        .last_preset_id
        .filter(|value| !value.trim().is_empty())
    {
        settings.last_preset_id = preset_id;
    }

    normalize_custom_export_presets(&mut settings.custom_export_presets)?;

    if !presets::all_preset_infos()
        .iter()
        .any(|preset| preset.id == settings.last_preset_id)
    {
        settings.last_preset_id = presets::PRESET_LOSSLESS.to_string();
    }

    normalize_settings_providers(
        &mut settings.upload_providers,
        &mut settings.default_upload_provider_id,
    )?;

    crate::windows_integration::set_run_at_startup(params.run_at_startup)?;
    save_settings(&settings)?;
    Ok(settings)
}

pub fn settings_path() -> Result<PathBuf, String> {
    let base = std::env::var("APPDATA").map_err(|err| format!("APPDATA is unavailable: {err}"))?;
    Ok(PathBuf::from(base).join("Cutdown").join("settings.json"))
}

fn sanitize_secret_key_part(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

fn provider_secret_key(provider_id: &str, field: &str) -> String {
    format!(
        "upload/{}/{}",
        sanitize_secret_key_part(provider_id),
        sanitize_secret_key_part(field)
    )
}

fn read_secret_into_option(key: &str, value: &mut Option<String>) {
    if value
        .as_ref()
        .is_some_and(|stored| !stored.trim().is_empty())
    {
        return;
    }

    if let Ok(Some(secret)) = crate::secret_store::read_secret(key) {
        if !secret.trim().is_empty() {
            *value = Some(secret);
        }
    }
}

fn persist_option_secret(key: &str, value: &mut Option<String>) -> Result<(), String> {
    match value
        .as_ref()
        .map(|stored| stored.trim())
        .filter(|stored| !stored.is_empty())
    {
        Some(secret) => {
            crate::secret_store::write_secret(key, secret)?;
            *value = None;
        }
        None => {
            crate::secret_store::delete_secret(key)?;
        }
    }
    Ok(())
}

fn read_secret_into_config(provider_id: &str, config: &mut Value, json_key: &str) {
    let has_value = config
        .get(json_key)
        .and_then(Value::as_str)
        .is_some_and(|value| !value.trim().is_empty());
    if has_value {
        return;
    }

    if let Ok(Some(secret)) =
        crate::secret_store::read_secret(&provider_secret_key(provider_id, json_key))
    {
        if !secret.trim().is_empty() {
            config[json_key] = Value::String(secret);
        }
    }
}

fn persist_config_secret(
    provider_id: &str,
    config: &mut Value,
    json_key: &str,
    scrubbed_value: Value,
) -> Result<(), String> {
    let key = provider_secret_key(provider_id, json_key);
    let secret = config
        .get(json_key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);

    if let Some(secret) = secret {
        crate::secret_store::write_secret(&key, &secret)?;
        config[json_key] = scrubbed_value;
    } else {
        crate::secret_store::delete_secret(&key)?;
    }

    Ok(())
}

fn hydrate_settings_secrets(settings: &mut AppSettings) {
    read_secret_into_option("legacy/catboxUserHash", &mut settings.catbox_user_hash);
    read_secret_into_option(
        "obs/websocketPassword",
        &mut settings.obs_websocket_password,
    );

    for provider in &mut settings.upload_providers {
        match provider.kind.as_str() {
            crate::upload_providers::KIND_CATBOX => {
                read_secret_into_config(&provider.id, &mut provider.config, "userHash");
            }
            crate::upload_providers::KIND_FILEGARDEN => {
                read_secret_into_config(&provider.id, &mut provider.config, "password");
                read_secret_into_config(&provider.id, &mut provider.config, "sessionCookie");
                read_secret_into_config(&provider.id, &mut provider.config, "authToken");
            }
            _ => {}
        }
    }
}

fn persist_settings_secrets(settings: &mut AppSettings) -> Result<(), String> {
    persist_option_secret("legacy/catboxUserHash", &mut settings.catbox_user_hash)?;
    persist_option_secret(
        "obs/websocketPassword",
        &mut settings.obs_websocket_password,
    )?;

    for provider in &mut settings.upload_providers {
        match provider.kind.as_str() {
            crate::upload_providers::KIND_CATBOX => {
                persist_config_secret(&provider.id, &mut provider.config, "userHash", Value::Null)?;
            }
            crate::upload_providers::KIND_FILEGARDEN => {
                persist_config_secret(
                    &provider.id,
                    &mut provider.config,
                    "password",
                    Value::String(String::new()),
                )?;
                persist_config_secret(
                    &provider.id,
                    &mut provider.config,
                    "sessionCookie",
                    Value::Null,
                )?;
                persist_config_secret(
                    &provider.id,
                    &mut provider.config,
                    "authToken",
                    Value::Null,
                )?;
            }
            _ => {}
        }
    }

    Ok(())
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

    let mut settings: AppSettings = serde_json::from_str(&raw).unwrap_or_default();
    migrate_upload_providers(
        &mut settings.upload_providers,
        &mut settings.default_upload_provider_id,
        &settings.catbox_user_hash,
        &settings.catbox_api_url,
    );
    hydrate_settings_secrets(&mut settings);
    if !settings.upload_providers.is_empty() {
        let previous_default = settings.default_upload_provider_id.clone();
        let _ = normalize_settings_providers(
            &mut settings.upload_providers,
            &mut settings.default_upload_provider_id,
        );
        if settings.default_upload_provider_id != previous_default {
            let _ = save_settings(&settings);
        }
    }
    settings
}

pub fn save_settings(settings: &AppSettings) -> Result<(), String> {
    let path = settings_path()?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("Failed to create settings directory: {err}"))?;
    }

    let mut serialized_settings = settings.clone();
    persist_settings_secrets(&mut serialized_settings)?;

    let raw = serde_json::to_string_pretty(&serialized_settings)
        .map_err(|err| format!("Failed to serialize settings: {err}"))?;

    fs::write(&path, raw).map_err(|err| format!("Failed to write settings: {err}"))
}

#[tauri::command]
pub fn get_settings() -> AppSettings {
    load_settings()
}

#[tauri::command]
pub fn push_recent_source(path: String) -> Result<Vec<String>, String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Ok(load_settings().recent_sources);
    }

    let mut settings = load_settings();
    settings.recent_sources.retain(|entry| entry != trimmed);
    settings.recent_sources.insert(0, trimmed.to_string());
    settings.recent_sources.truncate(MAX_RECENT_SOURCES);
    save_settings(&settings)?;
    Ok(settings.recent_sources.clone())
}

pub fn update_watch_folder_settings(
    path: Option<String>,
    enabled: bool,
) -> Result<AppSettings, String> {
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
    settings.watch_folder = params.watch_folder.filter(|value| !value.trim().is_empty());
    settings.watch_folder_enabled = params.watch_folder_enabled && settings.watch_folder.is_some();
    settings.default_export_dir = params
        .default_export_dir
        .filter(|value| !value.trim().is_empty());
    settings.prefer_gpu_encoding = params.prefer_gpu_encoding;
    settings.run_at_startup = params.run_at_startup;
    settings.catbox_user_hash = params
        .catbox_user_hash
        .filter(|value| !value.trim().is_empty());
    settings.catbox_api_url = params
        .catbox_api_url
        .filter(|value| !value.trim().is_empty());

    if let Some(preset_id) = params
        .last_preset_id
        .filter(|value| !value.trim().is_empty())
    {
        settings.last_preset_id = preset_id;
    }

    crate::windows_integration::set_run_at_startup(params.run_at_startup)?;
    save_settings(&settings)?;
    Ok(settings)
}

mod client;
mod providers;
mod response;

use crate::settings::{load_settings, save_settings};
use crate::upload_providers::{
    parse_catbox_config, parse_filegarden_config, parse_http_multipart_config, provider_summary,
    UploadProvider, UploadProviderSummary, KIND_CATBOX, KIND_FILEGARDEN, KIND_HTTP_MULTIPART,
};
use arboard::Clipboard;
use std::path::{Path, PathBuf};

#[tauri::command]
pub async fn upload_file(file_path: String, provider_id: Option<String>) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || upload_file_blocking(file_path, provider_id))
        .await
        .map_err(|err| format!("Upload worker failed: {err}"))?
}

fn upload_file_blocking(file_path: String, provider_id: Option<String>) -> Result<String, String> {
    let path = PathBuf::from(&file_path);
    if !path.is_file() {
        return Err("Upload file does not exist.".to_string());
    }

    let mut settings = load_settings();
    let provider_id = resolve_provider_id(&settings, provider_id.as_deref())?;
    let provider = settings
        .upload_providers
        .iter()
        .find(|entry| entry.id == provider_id)
        .ok_or_else(|| format!("Upload provider \"{provider_id}\" was not found."))?
        .clone();

    let url = upload_with_provider(&path, &provider, &mut settings)?;

    save_settings(&settings)?;
    Ok(url)
}

#[tauri::command]
pub fn list_upload_providers() -> Vec<UploadProviderSummary> {
    let settings = load_settings();
    settings
        .upload_providers
        .iter()
        .map(|provider| provider_summary(provider, &settings.default_upload_provider_id))
        .collect()
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveUploadProvidersParams {
    pub providers: Vec<UploadProvider>,
    pub default_upload_provider_id: Option<String>,
}

#[tauri::command]
pub fn save_upload_providers(
    params: SaveUploadProvidersParams,
) -> Result<Vec<UploadProviderSummary>, String> {
    let mut settings = load_settings();
    settings.upload_providers = params.providers;
    settings.default_upload_provider_id = params
        .default_upload_provider_id
        .filter(|value| !value.trim().is_empty());

    crate::upload_providers::normalize_settings_providers(
        &mut settings.upload_providers,
        &mut settings.default_upload_provider_id,
    )?;

    save_settings(&settings)?;

    Ok(settings
        .upload_providers
        .iter()
        .map(|provider| provider_summary(provider, &settings.default_upload_provider_id))
        .collect())
}

#[tauri::command]
pub fn get_upload_providers_for_editor() -> UploadProvidersEditorState {
    let settings = load_settings();
    UploadProvidersEditorState {
        providers: settings.upload_providers,
        default_upload_provider_id: settings.default_upload_provider_id,
    }
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadProvidersEditorState {
    pub providers: Vec<UploadProvider>,
    pub default_upload_provider_id: Option<String>,
}

#[tauri::command]
pub fn copy_text_to_clipboard(text: String) -> Result<(), String> {
    let mut clipboard =
        Clipboard::new().map_err(|err| format!("Failed to open clipboard: {err}"))?;
    clipboard
        .set_text(text)
        .map_err(|err| format!("Failed to copy to clipboard: {err}"))
}

fn resolve_provider_id(
    settings: &crate::settings::AppSettings,
    provider_id: Option<&str>,
) -> Result<String, String> {
    if let Some(id) = provider_id.filter(|value| !value.trim().is_empty()) {
        if let Some(provider) = settings
            .upload_providers
            .iter()
            .find(|provider| provider.id == id && provider.enabled)
        {
            return Ok(provider.id.clone());
        }
    }

    if let Some(default_id) = settings.default_upload_provider_id.as_ref() {
        if settings
            .upload_providers
            .iter()
            .any(|provider| provider.id == *default_id && provider.enabled)
        {
            return Ok(default_id.clone());
        }
    }

    settings
        .upload_providers
        .iter()
        .find(|provider| provider.enabled)
        .map(|provider| provider.id.clone())
        .ok_or_else(|| "No enabled upload providers are configured.".to_string())
}

fn upload_with_provider(
    path: &Path,
    provider: &UploadProvider,
    settings: &mut crate::settings::AppSettings,
) -> Result<String, String> {
    if !provider.enabled {
        return Err(format!(
            "Upload provider \"{}\" is disabled.",
            provider.name
        ));
    }

    match provider.kind.as_str() {
        KIND_CATBOX => {
            let config = parse_catbox_config(&provider.config)?;
            providers::catbox::upload(path, &config)
        }
        KIND_FILEGARDEN => {
            let mut config = parse_filegarden_config(&provider.config)?;
            let url = providers::filegarden::upload(path, &mut config)?;
            if let Some(stored) = settings
                .upload_providers
                .iter_mut()
                .find(|entry| entry.id == provider.id)
            {
                stored.config = crate::upload_providers::filegarden_config_to_value(&config);
            }
            Ok(url)
        }
        KIND_HTTP_MULTIPART => {
            let config = parse_http_multipart_config(&provider.config)?;
            providers::http_multipart::upload(path, &config)
        }
        other => Err(format!("Unknown upload provider kind: {other}")),
    }
}

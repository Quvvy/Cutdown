use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;

pub const KIND_CATBOX: &str = "catbox";
pub const KIND_FILEGARDEN: &str = "filegarden";
pub const KIND_HTTP_MULTIPART: &str = "http_multipart";

pub const DEFAULT_CATBOX_ID: &str = "catbox";
pub const DEFAULT_FILEGARDEN_ID: &str = "filegarden";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadProvider {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub kind: String,
    pub config: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadProviderSummary {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub kind: String,
    pub is_default: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatboxConfig {
    #[serde(default = "default_catbox_api_url")]
    pub api_url: String,
    pub user_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FilegardenConfig {
    #[serde(default = "default_filegarden_api_base")]
    pub api_base: String,
    pub email: String,
    pub password: String,
    pub totp: Option<String>,
    #[serde(default)]
    pub session_cookie: Option<String>,
    #[serde(default)]
    pub auth_user_id: Option<String>,
    #[serde(default)]
    pub auth_token: Option<String>,
    #[serde(default)]
    pub upload_url: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpMultipartConfig {
    pub url: String,
    #[serde(default = "default_file_field")]
    pub file_field: String,
    #[serde(default)]
    pub extra_fields: HashMap<String, String>,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    #[serde(default = "default_response_mode")]
    pub response_mode: String,
    pub response_json_path: Option<String>,
}

fn default_catbox_api_url() -> String {
    "https://catbox.moe/user/api.php".to_string()
}

fn default_filegarden_api_base() -> String {
    "https://api.filegarden.com".to_string()
}

fn default_file_field() -> String {
    "file".to_string()
}

fn default_response_mode() -> String {
    "plain_url".to_string()
}

pub fn default_catbox_provider() -> UploadProvider {
    UploadProvider {
        id: DEFAULT_CATBOX_ID.to_string(),
        name: "Catbox".to_string(),
        enabled: true,
        kind: KIND_CATBOX.to_string(),
        config: json!({
            "apiUrl": default_catbox_api_url(),
            "userHash": null
        }),
    }
}

pub fn default_filegarden_provider() -> UploadProvider {
    UploadProvider {
        id: DEFAULT_FILEGARDEN_ID.to_string(),
        name: "File Garden".to_string(),
        enabled: true,
        kind: KIND_FILEGARDEN.to_string(),
        config: json!({
            "apiBase": default_filegarden_api_base(),
            "email": "",
            "password": "",
            "totp": null,
            "sessionCookie": null,
            "authUserId": null,
            "authToken": null,
            "uploadUrl": null
        }),
    }
}

pub fn migrate_upload_providers(
    providers: &mut Vec<UploadProvider>,
    default_id: &mut Option<String>,
    catbox_user_hash: &Option<String>,
    catbox_api_url: &Option<String>,
) {
    if !providers.is_empty() {
        return;
    }

    let has_legacy = catbox_user_hash
        .as_ref()
        .is_some_and(|v| !v.trim().is_empty())
        || catbox_api_url
            .as_ref()
            .is_some_and(|v| !v.trim().is_empty());

    if has_legacy {
        let mut provider = default_catbox_provider();
        if let Some(url) = catbox_api_url.as_ref().filter(|v| !v.trim().is_empty()) {
            provider.config["apiUrl"] = json!(url.trim());
        }
        if let Some(hash) = catbox_user_hash.as_ref().filter(|v| !v.trim().is_empty()) {
            provider.config["userHash"] = json!(hash.trim());
        }
        providers.push(provider);
    } else {
        providers.push(default_filegarden_provider());
    }

    if default_id.is_none() {
        *default_id = providers.first().map(|provider| provider.id.clone());
    }
}

pub fn normalize_settings_providers(
    providers: &mut [UploadProvider],
    default_id: &mut Option<String>,
) -> Result<(), String> {
    if providers.is_empty() {
        return Err("Add at least one upload target in Settings.".to_string());
    }

    let mut seen = std::collections::HashSet::new();
    for provider in providers.iter() {
        let id = provider.id.trim();
        if id.is_empty() {
            return Err("Each upload provider needs a non-empty id.".to_string());
        }
        if !seen.insert(id.to_string()) {
            return Err(format!("Duplicate upload provider id: {id}"));
        }
    }

    for provider in providers.iter_mut() {
        provider.id = provider.id.trim().to_string();
        provider.name = provider.name.trim().to_string();
        if provider.name.is_empty() {
            provider.name = provider.id.clone();
        }
        provider.kind = provider.kind.trim().to_lowercase();
    }

    let first_enabled_id = || {
        providers
            .iter()
            .find(|provider| provider.enabled)
            .map(|provider| provider.id.clone())
            .or_else(|| providers.first().map(|provider| provider.id.clone()))
    };

    if let Some(default) = default_id.clone() {
        let default_valid = providers
            .iter()
            .any(|provider| provider.id == default && provider.enabled);
        if !default_valid {
            *default_id = first_enabled_id();
        }
    } else {
        *default_id = first_enabled_id();
    }

    Ok(())
}

pub fn provider_summary(
    provider: &UploadProvider,
    default_id: &Option<String>,
) -> UploadProviderSummary {
    UploadProviderSummary {
        id: provider.id.clone(),
        name: provider.name.clone(),
        enabled: provider.enabled,
        kind: provider.kind.clone(),
        is_default: default_id.as_ref() == Some(&provider.id),
    }
}

pub fn parse_catbox_config(config: &Value) -> Result<CatboxConfig, String> {
    serde_json::from_value(config.clone())
        .map_err(|err| format!("Invalid Catbox provider config: {err}"))
}

pub fn parse_filegarden_config(config: &Value) -> Result<FilegardenConfig, String> {
    serde_json::from_value(config.clone())
        .map_err(|err| format!("Invalid File Garden provider config: {err}"))
}

pub fn parse_http_multipart_config(config: &Value) -> Result<HttpMultipartConfig, String> {
    serde_json::from_value(config.clone())
        .map_err(|err| format!("Invalid custom upload provider config: {err}"))
}

pub fn filegarden_config_to_value(config: &FilegardenConfig) -> Value {
    serde_json::to_value(config).unwrap_or_else(|_| json!({}))
}

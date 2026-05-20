use crate::upload::client::build_client;
use crate::upload::response::{extract_share_url, mime_for_path};
use crate::upload_providers::HttpMultipartConfig;
use reqwest::blocking::multipart;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use std::path::Path;

pub fn upload(path: &Path, config: &HttpMultipartConfig) -> Result<String, String> {
    let url = config.url.trim();
    if url.is_empty() {
        return Err("Custom upload URL is required.".to_string());
    }
    if !url.starts_with("https://") && !url.starts_with("http://") {
        return Err("Custom upload URL must start with http:// or https://.".to_string());
    }

    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("clip.mp4")
        .to_string();

    let bytes = std::fs::read(path).map_err(|err| format!("Failed to read upload file: {err}"))?;
    let file_field = if config.file_field.trim().is_empty() {
        "file"
    } else {
        config.file_field.trim()
    };

    let part = multipart::Part::bytes(bytes)
        .file_name(file_name)
        .mime_str(mime_for_path(path))
        .map_err(|err| format!("Failed to prepare upload payload: {err}"))?;

    let mut form = multipart::Form::new().part(file_field.to_string(), part);
    for (key, value) in &config.extra_fields {
        if !key.trim().is_empty() {
            form = form.text(key.clone(), value.clone());
        }
    }

    let mut headers = HeaderMap::new();
    for (key, value) in &config.headers {
        let name = HeaderName::from_bytes(key.as_bytes())
            .map_err(|_| format!("Invalid header name: {key}"))?;
        let header_value = HeaderValue::from_str(value)
            .map_err(|_| format!("Invalid header value for {key}"))?;
        headers.insert(name, header_value);
    }

    let response = build_client()?
        .post(url)
        .headers(headers)
        .multipart(form)
        .send()
        .map_err(|err| format!("Custom upload request failed: {err}"))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        return Err(if body.trim().is_empty() {
            format!("Custom upload failed with status {status}.")
        } else {
            format!("Custom upload failed ({status}): {}", body.trim())
        });
    }

    let body = response
        .text()
        .map_err(|err| format!("Failed to read custom upload response: {err}"))?;

    extract_share_url(
        &body,
        config.response_mode.as_str(),
        config.response_json_path.as_deref(),
    )
}

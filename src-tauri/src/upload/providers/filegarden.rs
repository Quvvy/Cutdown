use crate::upload::client::build_client;
use crate::upload::response::{extract_share_url, mime_for_path};
use crate::upload_providers::FilegardenConfig;
use base64::engine::general_purpose::{
    STANDARD as BASE64_STANDARD, URL_SAFE_NO_PAD as BASE64_URL_SAFE,
};
use base64::Engine;
use reqwest::blocking::multipart;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use serde_json::{json, Value};
use std::fs::File;
use std::path::Path;

const DEFAULT_API_BASE: &str = "https://api.filegarden.com";
const CONTENT_ORIGIN: &str = "https://file.garden";
const DEPRECATED_V0_UPLOAD_SUFFIX: &str = "/api/v0/files";

pub fn upload(path: &Path, config: &mut FilegardenConfig) -> Result<String, String> {
    sanitize_config(config);
    let client = build_client()?;

    if auth_credentials(config).is_err() {
        sign_in(&client, config)?;
    }

    if let Some(upload_url) = config
        .upload_url
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        return upload_custom_multipart(path, &client, config, upload_url.trim());
    }

    match upload_via_pipe(path, &client, config) {
        Ok(url) => Ok(url),
        Err(first_error) => {
            sign_in(&client, config)?;
            upload_via_pipe(path, &client, config).map_err(|second_error| {
                format!("{second_error} (after re-authentication; first attempt: {first_error})")
            })
        }
    }
}

fn upload_via_pipe(
    path: &Path,
    client: &reqwest::blocking::Client,
    config: &FilegardenConfig,
) -> Result<String, String> {
    let (user_id, token) = auth_credentials(config)?;
    let api_base = normalize_api_base(&config.api_base);
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("clip.mp4")
        .to_string();

    // Match the File Garden web UI: encodeURI(JSON.stringify({ parent, name })).
    let x_data_json = json!({
        "parent": null,
        "name": file_name
    })
    .to_string();
    let x_data_header = encode_uri(&x_data_json);

    let response = client
        .post(format!("{api_base}/users/{user_id}/pipe"))
        .header(AUTHORIZATION, basic_auth_header(&user_id, &token))
        .header(CONTENT_TYPE, "application/octet-stream")
        .header("X-Data", x_data_header)
        .body(File::open(path).map_err(|err| format!("Failed to read upload file: {err}"))?)
        .send()
        .map_err(|err| format!("File Garden upload request failed: {err}"))?;

    let status = response.status();
    let body = response
        .text()
        .map_err(|err| format!("Failed to read File Garden upload response: {err}"))?;

    if status == reqwest::StatusCode::UNAUTHORIZED {
        return Err("File Garden session expired.".to_string());
    }

    if !status.is_success() {
        return Err(parse_legacy_error(&body, status.as_u16()));
    }

    share_url_from_pipe_response(&body, &user_id)
}

fn upload_custom_multipart(
    path: &Path,
    client: &reqwest::blocking::Client,
    config: &FilegardenConfig,
    upload_url: &str,
) -> Result<String, String> {
    let (user_id, token) = auth_credentials(config)?;
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("clip.mp4")
        .to_string();

    let file = File::open(path).map_err(|err| format!("Failed to read upload file: {err}"))?;
    let length = file
        .metadata()
        .map_err(|err| format!("Failed to inspect upload file: {err}"))?
        .len();
    let part = multipart::Part::reader_with_length(file, length)
        .file_name(file_name)
        .mime_str(mime_for_path(path))
        .map_err(|err| format!("Failed to prepare upload payload: {err}"))?;

    let form = multipart::Form::new().part("file", part);

    let response = client
        .post(upload_url)
        .header(AUTHORIZATION, basic_auth_header(&user_id, &token))
        .header("Accept", "application/json")
        .multipart(form)
        .send()
        .map_err(|err| format!("File Garden upload request failed: {err}"))?;

    let status = response.status();
    let body = response
        .text()
        .map_err(|err| format!("Failed to read File Garden upload response: {err}"))?;

    if status == reqwest::StatusCode::UNAUTHORIZED {
        return Err("File Garden session expired.".to_string());
    }

    if !status.is_success() {
        return Err(parse_legacy_error(&body, status.as_u16()));
    }

    extract_share_url(&body, "json_path", Some("url"))
        .or_else(|_| extract_share_url(&body, "json_path", Some("publicUrl")))
}

pub fn sign_in(
    client: &reqwest::blocking::Client,
    config: &mut FilegardenConfig,
) -> Result<(), String> {
    let email = config.email.trim();
    let password = config.password.trim();
    if email.is_empty() || password.is_empty() {
        return Err("File Garden email and password are required in Settings.".to_string());
    }

    if config
        .totp
        .as_ref()
        .is_some_and(|value| !value.trim().is_empty())
    {
        return Err(
            "File Garden's current API does not support TOTP through Cutdown. \
            Sign in on filegarden.com with Google or Discord, or use Catbox / a custom upload server."
                .to_string(),
        );
    }

    let api_base = normalize_api_base(&config.api_base);
    let connection = format!("password {}", BASE64_STANDARD.encode(password.as_bytes()));
    let payload = json!({
        "email": email,
        "connection": connection
    });

    let response = client
        .post(format!("{api_base}/token"))
        .header(CONTENT_TYPE, "application/json")
        .header("Accept", "application/json")
        .header("User-Agent", "Cutdown/0.1")
        .json(&payload)
        .send()
        .map_err(|err| format!("File Garden sign-in request failed: {err}"))?;

    let status = response.status();
    let body = response
        .text()
        .map_err(|err| format!("Failed to read File Garden sign-in response: {err}"))?;

    if status.is_redirection() {
        return Err("File Garden API base URL is wrong (got a redirect). \
            Set API base to https://api.filegarden.com in Settings."
            .to_string());
    }

    if !status.is_success() {
        return Err(parse_legacy_sign_in_error(&body, status.as_u16()));
    }

    let value: Value = serde_json::from_str(&body)
        .map_err(|_| "File Garden sign-in returned an unexpected response.".to_string())?;

    let user_id = value
        .get("id")
        .and_then(|entry| entry.as_str())
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .ok_or_else(|| "File Garden sign-in succeeded but no user id was returned.".to_string())?
        .to_string();

    let token = value
        .get("token")
        .and_then(|entry| entry.as_str())
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .ok_or_else(|| "File Garden sign-in succeeded but no token was returned.".to_string())?
        .to_string();

    config.auth_user_id = Some(user_id);
    config.auth_token = Some(token);
    config.session_cookie = None;

    Ok(())
}

fn auth_credentials(config: &FilegardenConfig) -> Result<(String, String), String> {
    match (
        config.auth_user_id.as_deref(),
        config.auth_token.as_deref(),
    ) {
        (Some(user_id), Some(token)) if !user_id.trim().is_empty() && !token.trim().is_empty() => {
            Ok((user_id.trim().to_string(), token.trim().to_string()))
        }
        _ => Err(
            "File Garden is not signed in. Save Settings after entering your email and password, then try Upload again."
                .to_string(),
        ),
    }
}

fn basic_auth_header(user_id: &str, token: &str) -> String {
    format!(
        "Basic {}",
        BASE64_STANDARD.encode(format!("{user_id}:{token}"))
    )
}

fn share_url_from_pipe_response(body: &str, user_id: &str) -> Result<String, String> {
    let value: Value = serde_json::from_str(body)
        .map_err(|_| "File Garden upload returned an unexpected response.".to_string())?;

    let path = value
        .get("path")
        .and_then(|entry| entry.as_str())
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .ok_or_else(|| "File Garden upload succeeded but no file path was returned.".to_string())?;

    Ok(format!(
        "{}/{}/{}",
        CONTENT_ORIGIN,
        public_user_id(user_id),
        encode_for_pipe(path)
    ))
}

fn public_user_id(user_id: &str) -> String {
    if user_id.len() == 24 && user_id.chars().all(|ch| ch.is_ascii_hexdigit()) {
        if let Ok(bytes) = hex_to_bytes(user_id) {
            return BASE64_URL_SAFE.encode(bytes);
        }
    }

    user_id.to_string()
}

fn hex_to_bytes(hex: &str) -> Result<Vec<u8>, String> {
    (0..hex.len())
        .step_by(2)
        .map(|index| {
            u8::from_str_radix(&hex[index..index + 2], 16)
                .map_err(|_| "Invalid File Garden user id.".to_string())
        })
        .collect()
}

fn encode_for_pipe(path: &str) -> String {
    path.split('/')
        .map(encode_uri_component_segment)
        .collect::<Vec<_>>()
        .join("/")
}

/// JavaScript `encodeURI` (used for the `X-Data` header).
fn encode_uri(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    for byte in input.bytes() {
        if is_encode_uri_unescaped(byte) {
            output.push(byte as char);
        } else {
            output.push('%');
            output.push(hex_digit(byte >> 4));
            output.push(hex_digit(byte & 0x0f));
        }
    }
    output
}

/// JavaScript `encodeURIComponent`, then restore `/` and `@` for pipe paths.
fn encode_uri_component_segment(segment: &str) -> String {
    let encoded = encode_uri_component(segment);
    encoded.replace("%2F", "/").replace("%40", "@")
}

/// JavaScript `encodeURIComponent`.
fn encode_uri_component(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    for byte in input.bytes() {
        if byte.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&byte) {
            output.push(byte as char);
        } else {
            output.push('%');
            output.push(hex_digit(byte >> 4));
            output.push(hex_digit(byte & 0x0f));
        }
    }
    output
}

fn is_encode_uri_unescaped(byte: u8) -> bool {
    byte.is_ascii_alphanumeric()
        || matches!(
            byte,
            b';' | b','
                | b'/'
                | b'?'
                | b':'
                | b'@'
                | b'&'
                | b'='
                | b'+'
                | b'$'
                | b'-'
                | b'_'
                | b'.'
                | b'!'
                | b'~'
                | b'*'
                | b'\''
                | b'('
                | b')'
                | b'#'
        )
}

fn hex_digit(value: u8) -> char {
    char::from_digit(u32::from(value), 16)
        .unwrap()
        .to_ascii_uppercase()
}

fn normalize_api_base(api_base: &str) -> String {
    let trimmed = api_base.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return DEFAULT_API_BASE.to_string();
    }

    if trimmed == "https://filegarden.com" || trimmed == "http://filegarden.com" {
        return DEFAULT_API_BASE.to_string();
    }

    if trimmed.starts_with("https://www.filegarden.com")
        || trimmed.starts_with("http://www.filegarden.com")
    {
        return DEFAULT_API_BASE.to_string();
    }

    trimmed.to_string()
}

fn sanitize_config(config: &mut FilegardenConfig) {
    config.api_base = normalize_api_base(&config.api_base);
    if config
        .upload_url
        .as_ref()
        .is_some_and(|value| value.contains(DEPRECATED_V0_UPLOAD_SUFFIX))
    {
        config.upload_url = None;
    }
}

fn parse_legacy_error(body: &str, status: u16) -> String {
    if let Ok(value) = serde_json::from_str::<Value>(body) {
        if let Some(message) = value.get("error").and_then(|entry| entry.as_str()) {
            return format!("File Garden upload failed ({status}): {message}");
        }
    }

    if body.trim().is_empty() {
        format!("File Garden upload failed with status {status}.")
    } else {
        format!("File Garden upload failed ({status}): {}", body.trim())
    }
}

fn parse_legacy_sign_in_error(body: &str, status: u16) -> String {
    if let Ok(value) = serde_json::from_str::<Value>(body) {
        if value.get("unverified").and_then(|entry| entry.as_bool()) == Some(true) {
            return "File Garden email is not verified. Check your inbox for the verification link."
                .to_string();
        }
        if let Some(message) = value.get("error").and_then(|entry| entry.as_str()) {
            if message.contains("password") {
                return "File Garden email or password is incorrect.".to_string();
            }
            return format!("File Garden sign-in failed ({status}): {message}");
        }
    }

    if body.trim().is_empty() {
        format!("File Garden sign-in failed with status {status}.")
    } else {
        format!("File Garden sign-in failed ({status}): {}", body.trim())
    }
}

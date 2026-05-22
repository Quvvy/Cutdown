use crate::upload::client::build_client;
use crate::upload::response::{extract_share_url, mime_for_path};
use crate::upload_providers::CatboxConfig;
use reqwest::blocking::multipart;
use std::fs::File;
use std::path::Path;

pub fn upload(path: &Path, config: &CatboxConfig) -> Result<String, String> {
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

    let mut form = multipart::Form::new()
        .text("reqtype", "fileupload")
        .part("fileToUpload", part);

    if let Some(userhash) = config
        .user_hash
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        form = form.text("userhash", userhash.trim().to_string());
    }

    let response = build_client()?
        .post(config.api_url.trim())
        .multipart(form)
        .send()
        .map_err(|err| format!("Catbox upload request failed: {err}"))?;

    if !response.status().is_success() {
        return Err(format!(
            "Catbox upload failed with status {}.",
            response.status()
        ));
    }

    let body = response
        .text()
        .map_err(|err| format!("Failed to read Catbox response: {err}"))?;

    extract_share_url(&body, "plain_url", None).map_err(|_| {
        if body.trim().is_empty() {
            "Catbox returned an empty response.".to_string()
        } else {
            format!("Catbox upload failed: {}", body.trim())
        }
    })
}

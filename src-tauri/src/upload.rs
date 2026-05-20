use crate::settings::load_settings;
use reqwest::blocking::multipart;
use std::fs;
use std::path::PathBuf;

#[tauri::command]
pub fn upload_to_catbox(file_path: String) -> Result<String, String> {
    let path = PathBuf::from(&file_path);
    if !path.is_file() {
        return Err("Upload file does not exist.".to_string());
    }

    let settings = load_settings();
    let api_url = settings
        .catbox_api_url
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "https://catbox.moe/user/api.php".to_string());

    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("clip.mp4")
        .to_string();

    let bytes = fs::read(&path).map_err(|err| format!("Failed to read upload file: {err}"))?;
    let part = multipart::Part::bytes(bytes)
        .file_name(file_name)
        .mime_str("video/mp4")
        .map_err(|err| format!("Failed to prepare upload payload: {err}"))?;

    let mut form = multipart::Form::new()
        .text("reqtype", "fileupload")
        .part("fileToUpload", part);

    if let Some(userhash) = settings
        .catbox_user_hash
        .filter(|value| !value.trim().is_empty())
    {
        form = form.text("userhash", userhash);
    }

    let response = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .map_err(|err| format!("Failed to create HTTP client: {err}"))?
        .post(&api_url)
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
        .map_err(|err| format!("Failed to read Catbox response: {err}"))?
        .trim()
        .to_string();

    if body.starts_with("http://") || body.starts_with("https://") {
        return Ok(body);
    }

    Err(if body.is_empty() {
        "Catbox returned an empty response.".to_string()
    } else {
        format!("Catbox upload failed: {body}")
    })
}

#[tauri::command]
pub fn copy_text_to_clipboard(text: String) -> Result<(), String> {
    let mut clipboard =
        arboard::Clipboard::new().map_err(|err| format!("Failed to open clipboard: {err}"))?;
    clipboard
        .set_text(text)
        .map_err(|err| format!("Failed to copy to clipboard: {err}"))
}

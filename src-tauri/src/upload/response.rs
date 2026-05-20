use serde_json::Value;

pub fn extract_share_url(body: &str, response_mode: &str, json_path: Option<&str>) -> Result<String, String> {
    let trimmed = body.trim();
    if trimmed.is_empty() {
        return Err("Upload host returned an empty response.".to_string());
    }

    if response_mode == "json_path" || trimmed.starts_with('{') || trimmed.starts_with('[') {
        let value: Value = serde_json::from_str(trimmed)
            .map_err(|err| format!("Upload response was not valid JSON: {err}"))?;
        let path = json_path.unwrap_or("url");
        if let Some(url) = read_json_path(&value, path) {
            if url.starts_with("http://") || url.starts_with("https://") {
                return Ok(url);
            }
        }
        if let Some(url) = find_url_in_json(&value) {
            return Ok(url);
        }
        return Err(format!(
            "Could not find a URL at JSON path \"{path}\" in the upload response."
        ));
    }

    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        return Ok(trimmed.to_string());
    }

    Err(format!("Upload host returned an unexpected response: {trimmed}"))
}

fn read_json_path(value: &Value, path: &str) -> Option<String> {
    let mut current = value;
    for segment in path.split('.') {
        if segment.is_empty() {
            continue;
        }
        current = current.get(segment)?;
    }
    current.as_str().map(str::to_string)
}

fn find_url_in_json(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => {
            if text.starts_with("http://") || text.starts_with("https://") {
                Some(text.clone())
            } else {
                None
            }
        }
        Value::Array(items) => items.iter().find_map(find_url_in_json),
        Value::Object(map) => {
            for key in ["url", "link", "publicUrl", "public_url", "directLink", "direct_link"] {
                if let Some(Value::String(text)) = map.get(key) {
                    if text.starts_with("http://") || text.starts_with("https://") {
                        return Some(text.clone());
                    }
                }
            }
            map.values().find_map(find_url_in_json)
        }
        _ => None,
    }
}

pub fn mime_for_path(path: &std::path::Path) -> &'static str {
    match path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "mkv" => "video/x-matroska",
        "mov" => "video/quicktime",
        "avi" => "video/x-msvideo",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        _ => "application/octet-stream",
    }
}

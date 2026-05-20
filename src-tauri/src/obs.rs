use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

const VIDEO_EXTENSIONS: &[&str] = &["mp4", "mkv", "mov", "avi", "webm", "ts", "flv", "m4v"];

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LatestReplayResult {
    pub path: Option<String>,
    pub message: String,
}

fn is_video_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            VIDEO_EXTENSIONS
                .iter()
                .any(|candidate| ext.eq_ignore_ascii_case(candidate))
        })
        .unwrap_or(false)
}

fn modified_time(path: &Path) -> Option<SystemTime> {
    fs::metadata(path).ok()?.modified().ok()
}

pub fn find_latest_video_in_directory(dir: &str) -> Result<Option<PathBuf>, String> {
    let root = PathBuf::from(dir.trim());
    if !root.is_dir() {
        return Err("Folder does not exist.".to_string());
    }

    let mut newest: Option<(SystemTime, PathBuf)> = None;

    for entry in fs::read_dir(&root).map_err(|err| format!("Failed to read folder: {err}"))? {
        let entry = entry.map_err(|err| format!("Failed to read folder entry: {err}"))?;
        let path = entry.path();
        if !path.is_file() || !is_video_file(&path) {
            continue;
        }

        let Some(modified) = modified_time(&path) else {
            continue;
        };

        if newest
            .as_ref()
            .map(|(time, _)| modified > *time)
            .unwrap_or(true)
        {
            newest = Some((modified, path));
        }
    }

    Ok(newest.map(|(_, path)| path))
}

#[tauri::command]
pub fn find_latest_replay_in_folder(folder: String) -> Result<LatestReplayResult, String> {
    let latest = find_latest_video_in_directory(&folder)?;
    Ok(LatestReplayResult {
        path: latest
            .as_ref()
            .map(|path| path.to_string_lossy().to_string()),
        message: latest
            .as_ref()
            .map(|path| format!("Latest replay: {}", path.display()))
            .unwrap_or_else(|| "No video files found in folder.".to_string()),
    })
}

#[derive(Debug, Deserialize)]
struct ObsHello {
    #[serde(default)]
    d: ObsHelloData,
}

#[derive(Debug, Deserialize, Default)]
struct ObsHelloData {
    #[serde(default)]
    authentication: Option<ObsAuthChallenge>,
}

#[derive(Debug, Deserialize)]
struct ObsAuthChallenge {
    challenge: String,
    salt: String,
}

#[derive(Debug, Deserialize)]
struct ObsResponseEnvelope {
    #[serde(default)]
    d: ObsResponseData,
}

#[derive(Debug, Deserialize, Default)]
struct ObsResponseData {
    #[serde(default)]
    request_status: Option<ObsRequestStatus>,
}

#[derive(Debug, Deserialize, Default)]
struct ObsRequestStatus {
    #[serde(default)]
    code: i64,
    #[serde(default)]
    comment: Option<String>,
}

fn obs_auth_response(password: &str, challenge: &ObsAuthChallenge) -> String {
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    use sha2::{Digest, Sha256};

    let secret = format!(
        "{}{}",
        password,
        STANDARD.encode(Sha256::digest(password.as_bytes()))
    );
    let secret_hash = Sha256::digest(secret.as_bytes());
    let challenge_hash = Sha256::digest(
        format!(
            "{}{}",
            STANDARD.encode(secret_hash),
            challenge.challenge
        )
        .as_bytes(),
    );
    let auth_hash = Sha256::digest(
        format!(
            "{}{}",
            STANDARD.encode(challenge_hash),
            challenge.salt
        )
        .as_bytes(),
    );
    STANDARD.encode(auth_hash)
}

fn send_obs_request(
    host: &str,
    port: u16,
    password: Option<&str>,
    request_type: &str,
) -> Result<(), String> {
    use tungstenite::connect;
    use tungstenite::Message;

    let url = format!("ws://{host}:{port}");
    let (mut socket, _) = connect(&url).map_err(|err| format!("OBS WebSocket connect failed: {err}"))?;

    let hello_raw = match socket.read() {
        Ok(Message::Text(text)) => text,
        Ok(_) => return Err("Unexpected OBS WebSocket handshake message.".to_string()),
        Err(err) => return Err(format!("OBS WebSocket read failed: {err}")),
    };

    let hello: ObsHello =
        serde_json::from_str(&hello_raw).map_err(|err| format!("Invalid OBS hello payload: {err}"))?;

    let auth = if let (Some(password), Some(challenge)) = (password.filter(|p| !p.is_empty()), hello.d.authentication) {
        Some(obs_auth_response(password, &challenge))
    } else {
        None
    };

    let identify = serde_json::json!({
        "op": 1,
        "d": {
            "rpcVersion": 1,
            "eventSubscriptions": 0,
            "authentication": auth,
        }
    });
    socket
        .send(Message::Text(identify.to_string()))
        .map_err(|err| format!("OBS identify failed: {err}"))?;

    if let Ok(Message::Text(text)) = socket.read() {
        let _: serde_json::Value = serde_json::from_str(&text).unwrap_or_default();
    }

    let request = serde_json::json!({
        "op": 6,
        "d": {
            "requestType": request_type,
            "requestId": "cutdown-obs",
        }
    });
    socket
        .send(Message::Text(request.to_string()))
        .map_err(|err| format!("OBS request failed: {err}"))?;

    let response_raw = match socket.read() {
        Ok(Message::Text(text)) => text,
        Ok(_) => return Err("Unexpected OBS response.".to_string()),
        Err(err) => return Err(format!("OBS response read failed: {err}")),
    };

    let response: ObsResponseEnvelope = serde_json::from_str(&response_raw)
        .map_err(|err| format!("Invalid OBS response: {err}"))?;

    if response.d.request_status.as_ref().map(|s| s.code).unwrap_or(100) != 100 {
        return Err(response
            .d
            .request_status
            .and_then(|status| status.comment)
            .unwrap_or_else(|| format!("OBS request {request_type} failed.")));
    }

    Ok(())
}

#[tauri::command]
pub fn save_obs_replay_buffer(
    host: Option<String>,
    port: Option<u16>,
    password: Option<String>,
) -> Result<(), String> {
    let host = host
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "127.0.0.1".to_string());
    let port = port.unwrap_or(4455);
    let password = password.filter(|value| !value.trim().is_empty());

    send_obs_request(&host, port, password.as_deref(), "SaveReplayBuffer")
}

#[tauri::command]
pub fn open_watch_folder_in_explorer(path: String) -> Result<(), String> {
    crate::ffmpeg::reveal_in_explorer(path)
}

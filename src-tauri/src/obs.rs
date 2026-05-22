use serde::Serialize;
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

fn file_added_time(path: &Path) -> Option<SystemTime> {
    let meta = fs::metadata(path).ok()?;
    meta.created().ok().or_else(|| meta.modified().ok())
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

        let Some(added) = file_added_time(&path) else {
            continue;
        };

        if newest
            .as_ref()
            .map(|(time, _)| added > *time)
            .unwrap_or(true)
        {
            newest = Some((added, path));
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

#[tauri::command]
pub fn open_watch_folder_in_explorer(path: String) -> Result<(), String> {
    crate::ffmpeg::reveal_in_explorer(path)
}

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const MAX_ENTRIES: usize = 50;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipHistoryEntry {
    pub output_path: String,
    pub source_path: Option<String>,
    pub preset_id: String,
    pub exported_at: String,
    pub file_size: u64,
    pub duration: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub share_url: Option<String>,
}

pub fn history_path() -> Result<PathBuf, String> {
    let base = std::env::var("APPDATA").map_err(|err| format!("APPDATA is unavailable: {err}"))?;
    Ok(PathBuf::from(base).join("Cutdown").join("history.json"))
}

fn load_entries() -> Vec<ClipHistoryEntry> {
    let path = match history_path() {
        Ok(path) => path,
        Err(_) => return Vec::new(),
    };

    if !path.exists() {
        return Vec::new();
    }

    let raw = match fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(_) => return Vec::new(),
    };

    serde_json::from_str(&raw).unwrap_or_default()
}

fn save_entries(entries: &[ClipHistoryEntry]) -> Result<(), String> {
    let path = history_path()?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("Failed to create history directory: {err}"))?;
    }

    let raw = serde_json::to_string_pretty(entries)
        .map_err(|err| format!("Failed to serialize clip history: {err}"))?;

    fs::write(&path, raw).map_err(|err| format!("Failed to write clip history: {err}"))
}

pub fn append_entry(entry: ClipHistoryEntry) -> Result<Vec<ClipHistoryEntry>, String> {
    let mut entries = load_entries();
    entries.retain(|existing| existing.output_path != entry.output_path);
    entries.insert(0, entry);

    if entries.len() > MAX_ENTRIES {
        entries.truncate(MAX_ENTRIES);
    }

    save_entries(&entries)?;
    Ok(entries)
}

#[tauri::command]
pub fn list_clip_history() -> Vec<ClipHistoryEntry> {
    load_entries()
}

#[tauri::command]
pub fn clear_clip_history() -> Result<(), String> {
    save_entries(&[])
}

#[tauri::command]
pub fn remove_clip_history_entry(output_path: String) -> Result<Vec<ClipHistoryEntry>, String> {
    let mut entries = load_entries();
    entries.retain(|entry| entry.output_path != output_path);
    save_entries(&entries)?;
    Ok(entries)
}

#[tauri::command]
pub fn update_clip_history_share_url(
    output_path: String,
    share_url: String,
) -> Result<Vec<ClipHistoryEntry>, String> {
    let mut entries = load_entries();
    let mut found = false;

    for entry in entries.iter_mut() {
        if entry.output_path == output_path {
            entry.share_url = Some(share_url);
            found = true;
            break;
        }
    }

    if !found {
        return Err(format!("History entry not found for {output_path}"));
    }

    save_entries(&entries)?;
    Ok(entries)
}

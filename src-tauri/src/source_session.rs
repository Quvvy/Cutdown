use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedSegment {
    pub id: String,
    pub source_start: f64,
    pub source_end: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedBookmark {
    pub id: String,
    pub time: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NormalizedCropRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceSession {
    pub source_path: String,
    pub segments: Vec<SavedSegment>,
    pub selected_segment_id: Option<String>,
    pub range_start: Option<f64>,
    pub range_end: Option<f64>,
    pub crop_enabled: bool,
    pub crop_rect: NormalizedCropRect,
    pub clip_volume: f64,
    pub current_time: Option<f64>,
    #[serde(default)]
    pub bookmarks: Vec<SavedBookmark>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct SessionStore {
    #[serde(default)]
    sessions: HashMap<String, SourceSession>,
}

const MAX_SESSIONS: usize = 32;

fn sessions_path() -> Result<PathBuf, String> {
    let base = std::env::var("APPDATA").map_err(|err| format!("APPDATA is unavailable: {err}"))?;
    Ok(PathBuf::from(base).join("Cutdown").join("sessions.json"))
}

fn session_key(path: &str) -> String {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let candidate = Path::new(trimmed);
    if let Ok(canonical) = candidate.canonicalize() {
        return canonical.to_string_lossy().to_string();
    }

    trimmed.replace('/', "\\")
}

fn load_store() -> SessionStore {
    let path = match sessions_path() {
        Ok(path) => path,
        Err(_) => return SessionStore::default(),
    };

    if !path.exists() {
        return SessionStore::default();
    }

    let raw = match fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(_) => return SessionStore::default(),
    };

    serde_json::from_str(&raw).unwrap_or_default()
}

fn save_store(store: &SessionStore) -> Result<(), String> {
    let path = sessions_path()?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("Failed to create sessions directory: {err}"))?;
    }

    let raw = serde_json::to_string_pretty(store)
        .map_err(|err| format!("Failed to serialize sessions: {err}"))?;

    fs::write(&path, raw).map_err(|err| format!("Failed to write sessions: {err}"))
}

fn sanitize_session(session: SourceSession, duration: Option<f64>) -> Option<SourceSession> {
    if session.segments.is_empty() {
        return None;
    }

    let mut segments = session.segments;
    if let Some(duration) = duration {
        segments.retain(|segment| {
            segment.source_start < duration
                && segment.source_end > segment.source_start + 0.01
                && segment.source_end <= duration + 0.05
        });
        for segment in &mut segments {
            segment.source_start = segment.source_start.clamp(0.0, duration);
            segment.source_end = segment.source_end.clamp(segment.source_start + 0.01, duration);
        }
    }

    if segments.is_empty() {
        return None;
    }

    let selected_segment_id = session
        .selected_segment_id
        .filter(|id| segments.iter().any(|segment| &segment.id == id));

    let (range_start, range_end) = match (session.range_start, session.range_end) {
        (Some(start), Some(end)) if end > start => (Some(start), Some(end)),
        _ => (None, None),
    };

    let mut bookmarks = session.bookmarks;
    if let Some(duration) = duration {
        bookmarks.retain(|bookmark| bookmark.time >= 0.0 && bookmark.time <= duration + 0.05);
    }
    bookmarks.sort_by(|left, right| {
        left.time
            .partial_cmp(&right.time)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    Some(SourceSession {
        source_path: session.source_path,
        segments,
        selected_segment_id,
        range_start,
        range_end,
        crop_enabled: session.crop_enabled,
        crop_rect: session.crop_rect,
        clip_volume: session.clip_volume.clamp(0.0, 1.0),
        current_time: session.current_time.map(|value| value.max(0.0)),
        bookmarks,
    })
}

#[tauri::command]
pub fn get_source_session(path: String, duration: Option<f64>) -> Result<Option<SourceSession>, String> {
    let key = session_key(&path);
    if key.is_empty() {
        return Ok(None);
    }

    let store = load_store();
    let session = store.sessions.get(&key).cloned();
    Ok(session.and_then(|entry| sanitize_session(entry, duration)))
}

#[tauri::command]
pub fn save_source_session(session: SourceSession, duration: Option<f64>) -> Result<(), String> {
    let key = session_key(&session.source_path);
    if key.is_empty() {
        return Ok(());
    }

    let Some(session) = sanitize_session(session, duration) else {
        return Ok(());
    };

    let mut store = load_store();
    store.sessions.insert(key, session);

    if store.sessions.len() > MAX_SESSIONS {
        let overflow = store.sessions.len() - MAX_SESSIONS;
        let keys: Vec<String> = store.sessions.keys().take(overflow).cloned().collect();
        for stale in keys {
            store.sessions.remove(&stale);
        }
    }

    save_store(&store)
}

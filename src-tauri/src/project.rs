use crate::source_session::{NormalizedCropRect, SavedSegment};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

pub const PROJECT_VERSION: u32 = 1;
pub const PROJECT_EXTENSION: &str = "cutdown";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CutdownProject {
    pub version: u32,
    pub source_path: String,
    pub segments: Vec<SavedSegment>,
    pub selected_segment_id: Option<String>,
    pub range_start: Option<f64>,
    pub range_end: Option<f64>,
    pub crop_enabled: bool,
    pub crop_rect: NormalizedCropRect,
    pub clip_volume: f64,
    pub current_time: Option<f64>,
    pub export_preset_id: Option<String>,
    pub accurate_trim: bool,
    pub strip_audio: bool,
}

#[tauri::command]
pub fn save_project_file(path: String, project: CutdownProject) -> Result<(), String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("Project path is required.".to_string());
    }

    let mut payload = project;
    payload.version = PROJECT_VERSION;

    let raw = serde_json::to_string_pretty(&payload)
        .map_err(|err| format!("Failed to serialize project: {err}"))?;

    if let Some(parent) = Path::new(trimmed).parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .map_err(|err| format!("Failed to create project directory: {err}"))?;
        }
    }

    fs::write(trimmed, raw).map_err(|err| format!("Failed to write project file: {err}"))
}

#[tauri::command]
pub fn load_project_file(path: String) -> Result<CutdownProject, String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("Project path is required.".to_string());
    }

    let file = Path::new(trimmed);
    if !file.exists() {
        return Err("Project file does not exist.".to_string());
    }

    let raw =
        fs::read_to_string(file).map_err(|err| format!("Failed to read project file: {err}"))?;

    let mut project: CutdownProject =
        serde_json::from_str(&raw).map_err(|err| format!("Invalid project file: {err}"))?;

    if project.version == 0 {
        project.version = PROJECT_VERSION;
    }

    if project.version > PROJECT_VERSION {
        return Err("This project file was created with a newer version of Cutdown.".to_string());
    }

    Ok(project)
}

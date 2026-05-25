use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

pub const PROJECT_VERSION: u32 = 1;
pub const PROJECT_EXTENSION: &str = "cutdown";

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
    #[serde(default)]
    pub bookmarks: Vec<SavedBookmark>,
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn sample_project() -> CutdownProject {
        CutdownProject {
            version: 0,
            source_path: "E:\\clips\\sample.mp4".to_string(),
            segments: vec![SavedSegment {
                id: "segment-1".to_string(),
                source_start: 1.0,
                source_end: 4.0,
            }],
            selected_segment_id: Some("segment-1".to_string()),
            range_start: Some(1.0),
            range_end: Some(4.0),
            crop_enabled: false,
            crop_rect: NormalizedCropRect {
                x: 0.0,
                y: 0.0,
                width: 1.0,
                height: 1.0,
            },
            clip_volume: 1.0,
            current_time: Some(1.5),
            bookmarks: vec![SavedBookmark {
                id: "bookmark-1".to_string(),
                time: 2.0,
                label: Some("Nice cut".to_string()),
            }],
            export_preset_id: Some("lossless-trim".to_string()),
            accurate_trim: false,
            strip_audio: false,
        }
    }

    fn temp_project_path(name: &str) -> String {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        std::env::temp_dir()
            .join(format!("cutdown-{name}-{unique}.cutdown"))
            .to_string_lossy()
            .to_string()
    }

    #[test]
    fn saves_and_loads_project_bookmarks() {
        let path = temp_project_path("bookmarks");
        save_project_file(path.clone(), sample_project()).expect("project should save");

        let loaded = load_project_file(path.clone()).expect("project should load");
        fs::remove_file(path).ok();

        assert_eq!(loaded.version, PROJECT_VERSION);
        assert_eq!(loaded.bookmarks.len(), 1);
        assert_eq!(loaded.bookmarks[0].id, "bookmark-1");
        assert_eq!(loaded.bookmarks[0].time, 2.0);
        assert_eq!(loaded.bookmarks[0].label.as_deref(), Some("Nice cut"));
    }

    #[test]
    fn loads_legacy_project_without_bookmarks() {
        let path = temp_project_path("legacy");
        fs::write(
            &path,
            r#"{
  "version": 1,
  "sourcePath": "E:\\clips\\sample.mp4",
  "segments": [{"id": "segment-1", "sourceStart": 1.0, "sourceEnd": 4.0}],
  "selectedSegmentId": "segment-1",
  "rangeStart": 1.0,
  "rangeEnd": 4.0,
  "cropEnabled": false,
  "cropRect": {"x": 0.0, "y": 0.0, "width": 1.0, "height": 1.0},
  "clipVolume": 1.0,
  "currentTime": 1.5,
  "exportPresetId": "lossless-trim",
  "accurateTrim": false,
  "stripAudio": false
}"#,
        )
        .expect("legacy project should write");

        let loaded = load_project_file(path.clone()).expect("legacy project should load");
        fs::remove_file(path).ok();

        assert!(loaded.bookmarks.is_empty());
    }
}

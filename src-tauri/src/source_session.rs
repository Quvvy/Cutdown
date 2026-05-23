use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedSegment {
    pub id: String,
    pub source_start: f64,
    pub source_end: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NormalizedCropRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

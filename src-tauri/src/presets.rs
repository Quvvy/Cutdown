use crate::settings::{load_settings, CustomExportPreset};
// CustomExportPreset lives in settings.rs; conversion helpers below.
use serde::Serialize;

pub const PRESET_LOSSLESS: &str = "lossless-trim";
pub const PRESET_DISCORD: &str = "discord";
pub const PRESET_ARCHIVE: &str = "archive";
pub const PRESET_TWITTER: &str = "twitter";

pub const DISCORD_TARGET_BYTES: u64 = 9 * 1024 * 1024;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub lossless: bool,
    pub requires_gpu: bool,
    #[serde(default)]
    pub custom: bool,
    #[serde(default)]
    pub target_bytes: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct EncodeProfile {
    pub lossless: bool,
    pub video_codec: String,
    pub video_args: Vec<String>,
    pub audio_codec: String,
    pub audio_args: Vec<String>,
    pub scale_filter: Option<String>,
    pub target_bytes: Option<u64>,
}

#[tauri::command]
pub fn list_presets() -> Vec<PresetInfo> {
    all_preset_infos()
}

pub fn all_preset_infos() -> Vec<PresetInfo> {
    let mut presets = builtin_presets();
    for custom in &load_settings().custom_export_presets {
        presets.push(custom.to_preset_info());
    }
    presets
}

pub fn resolve_preset_id(preset_id: Option<String>) -> String {
    preset_id
        .filter(|id| !id.trim().is_empty())
        .unwrap_or_else(|| PRESET_LOSSLESS.to_string())
}

pub fn resolve_encode_profile(
    preset_id: &str,
    prefer_gpu: bool,
    gpu_encoders: &[String],
    total_duration_secs: f64,
    video_width: u32,
    video_height: u32,
) -> Result<EncodeProfile, String> {
    if preset_id == PRESET_LOSSLESS {
        return Ok(EncodeProfile {
            lossless: true,
            video_codec: "copy".to_string(),
            video_args: Vec::new(),
            audio_codec: "copy".to_string(),
            audio_args: Vec::new(),
            scale_filter: None,
            target_bytes: None,
        });
    }

    if let Some(custom) = find_custom_preset(preset_id) {
        return custom_to_profile(
            &custom,
            prefer_gpu,
            gpu_encoders,
            total_duration_secs,
            video_width,
            video_height,
        );
    }

    let video_codec = pick_video_encoder(prefer_gpu, gpu_encoders);

    let profile = match preset_id {
        PRESET_DISCORD => {
            let audio_kbps = 128u64;
            let video_kbps = discord_video_bitrate_kbps(total_duration_secs, audio_kbps);
            EncodeProfile {
                lossless: false,
                video_codec: video_codec.clone(),
                video_args: encoder_args(&video_codec, video_kbps, "fast"),
                audio_codec: "aac".to_string(),
                audio_args: vec!["-b:a".to_string(), format!("{audio_kbps}k")],
                scale_filter: optional_scale_filter(1280, 720, video_width, video_height),
                target_bytes: Some(DISCORD_TARGET_BYTES),
            }
        }
        PRESET_ARCHIVE => {
            let archive_codec = "libx264".to_string();
            EncodeProfile {
                lossless: false,
                video_codec: archive_codec,
                video_args: vec![
                    "-crf".to_string(),
                    "18".to_string(),
                    "-preset".to_string(),
                    "medium".to_string(),
                ],
                audio_codec: "aac".to_string(),
                audio_args: vec!["-b:a".to_string(), "192k".to_string()],
                scale_filter: None,
                target_bytes: None,
            }
        }
        PRESET_TWITTER => {
            let video_kbps = 2500u64;
            EncodeProfile {
                lossless: false,
                video_codec: video_codec.clone(),
                video_args: encoder_args(&video_codec, video_kbps, "fast"),
                audio_codec: "aac".to_string(),
                audio_args: vec!["-b:a".to_string(), "128k".to_string()],
                scale_filter: optional_scale_filter(1280, 720, video_width, video_height),
                target_bytes: None,
            }
        }
        _ => return Err(format!("Unknown export preset: {preset_id}")),
    };

    Ok(profile)
}

pub fn normalize_custom_export_presets(presets: &mut [CustomExportPreset]) -> Result<(), String> {
    let mut seen = std::collections::HashSet::new();

    for preset in presets.iter_mut() {
        preset.id = preset.id.trim().to_string();
        preset.name = preset.name.trim().to_string();
        preset.description = preset.description.trim().to_string();

        if preset.id.is_empty() {
            return Err("Each custom export preset needs an id.".to_string());
        }

        if preset.name.is_empty() {
            return Err("Each custom export preset needs a name.".to_string());
        }

        if is_builtin_preset_id(&preset.id) {
            return Err(format!(
                "Custom preset id \"{}\" conflicts with a built-in preset.",
                preset.id
            ));
        }

        if !seen.insert(preset.id.clone()) {
            return Err(format!("Duplicate custom export preset id: {}", preset.id));
        }

        if preset.lossless {
            continue;
        }

        match preset.mode.as_str() {
            "bitrate" => {
                if preset.video_bitrate_kbps.unwrap_or(0) == 0 {
                    return Err(format!(
                        "Preset \"{}\" needs a video bitrate (kbps) for bitrate mode.",
                        preset.name
                    ));
                }
            }
            "crf" => {
                if preset.crf.is_none() {
                    return Err(format!(
                        "Preset \"{}\" needs a CRF value for quality mode.",
                        preset.name
                    ));
                }
            }
            "target_size" => {
                if preset.target_bytes.unwrap_or(0) == 0 {
                    return Err(format!(
                        "Preset \"{}\" needs a target file size for target-size mode.",
                        preset.name
                    ));
                }
            }
            other => {
                return Err(format!(
                    "Preset \"{}\" has unknown mode \"{other}\".",
                    preset.name
                ));
            }
        }

        if preset.audio_bitrate_kbps.is_none() {
            preset.audio_bitrate_kbps = Some(128);
        }
    }

    Ok(())
}

fn find_custom_preset(preset_id: &str) -> Option<CustomExportPreset> {
    load_settings()
        .custom_export_presets
        .into_iter()
        .find(|preset| preset.id == preset_id)
}

fn custom_to_profile(
    preset: &CustomExportPreset,
    prefer_gpu: bool,
    gpu_encoders: &[String],
    total_duration_secs: f64,
    video_width: u32,
    video_height: u32,
) -> Result<EncodeProfile, String> {
    if preset.lossless {
        return Ok(EncodeProfile {
            lossless: true,
            video_codec: "copy".to_string(),
            video_args: Vec::new(),
            audio_codec: "copy".to_string(),
            audio_args: Vec::new(),
            scale_filter: None,
            target_bytes: None,
        });
    }

    let video_codec = pick_video_encoder(prefer_gpu, gpu_encoders);
    let encoder_speed = preset
        .encoder_speed
        .as_deref()
        .filter(|value| !value.is_empty())
        .unwrap_or("fast");
    let audio_kbps = preset.audio_bitrate_kbps.unwrap_or(128);
    let scale_filter = optional_scale_from_preset(preset, video_width, video_height);

    let profile = match preset.mode.as_str() {
        "bitrate" => {
            let video_kbps = preset.video_bitrate_kbps.unwrap_or(2500);
            EncodeProfile {
                lossless: false,
                video_codec: video_codec.clone(),
                video_args: encoder_args(&video_codec, video_kbps, encoder_speed),
                audio_codec: "aac".to_string(),
                audio_args: vec!["-b:a".to_string(), format!("{audio_kbps}k")],
                scale_filter,
                target_bytes: None,
            }
        }
        "crf" => {
            let crf = preset.crf.unwrap_or(20);
            EncodeProfile {
                lossless: false,
                video_codec: "libx264".to_string(),
                video_args: vec![
                    "-crf".to_string(),
                    crf.to_string(),
                    "-preset".to_string(),
                    encoder_speed.to_string(),
                ],
                audio_codec: "aac".to_string(),
                audio_args: vec!["-b:a".to_string(), format!("{audio_kbps}k")],
                scale_filter,
                target_bytes: None,
            }
        }
        "target_size" => {
            let target_bytes = preset.target_bytes.unwrap_or(DISCORD_TARGET_BYTES);
            let video_kbps = discord_video_bitrate_kbps(total_duration_secs, audio_kbps);
            EncodeProfile {
                lossless: false,
                video_codec: video_codec.clone(),
                video_args: encoder_args(&video_codec, video_kbps, encoder_speed),
                audio_codec: "aac".to_string(),
                audio_args: vec!["-b:a".to_string(), format!("{audio_kbps}k")],
                scale_filter,
                target_bytes: Some(target_bytes),
            }
        }
        other => return Err(format!("Unknown custom preset mode: {other}")),
    };

    Ok(profile)
}

fn optional_scale_from_preset(
    preset: &CustomExportPreset,
    video_width: u32,
    video_height: u32,
) -> Option<String> {
    match (preset.max_width, preset.max_height) {
        (Some(width), Some(height)) => {
            optional_scale_filter(width, height, video_width, video_height)
        }
        _ => None,
    }
}

fn is_builtin_preset_id(id: &str) -> bool {
    matches!(
        id,
        PRESET_LOSSLESS | PRESET_DISCORD | PRESET_ARCHIVE | PRESET_TWITTER
    )
}

pub fn libx264_fallback_profile(profile: &EncodeProfile) -> EncodeProfile {
    let mut fallback = profile.clone();
    if fallback.video_codec == "libx264" {
        return fallback;
    }

    fallback.video_codec = "libx264".to_string();
    if profile.video_args.iter().any(|arg| arg == "-crf") {
        return fallback;
    }

    let kbps = profile
        .video_args
        .windows(2)
        .find(|pair| pair[0] == "-b:v")
        .and_then(|pair| pair[1].trim_end_matches('k').parse::<u64>().ok())
        .unwrap_or(2500);
    fallback.video_args = encoder_args("libx264", kbps, "fast");
    fallback
}

fn pick_video_encoder(prefer_gpu: bool, gpu_encoders: &[String]) -> String {
    if !prefer_gpu {
        return "libx264".to_string();
    }

    for candidate in ["h264_nvenc", "h264_amf", "h264_qsv"] {
        if gpu_encoders.iter().any(|encoder| encoder == candidate) {
            return candidate.to_string();
        }
    }

    "libx264".to_string()
}

fn encoder_args(codec: &str, video_kbps: u64, preset: &str) -> Vec<String> {
    if codec == "libx264" {
        return vec![
            "-b:v".to_string(),
            format!("{video_kbps}k"),
            "-preset".to_string(),
            preset.to_string(),
            "-maxrate".to_string(),
            format!("{}k", video_kbps + 256),
            "-bufsize".to_string(),
            format!("{}k", video_kbps * 2),
        ];
    }

    vec![
        "-b:v".to_string(),
        format!("{video_kbps}k"),
        "-preset".to_string(),
        if codec.contains("nvenc") {
            "p4".to_string()
        } else {
            preset.to_string()
        },
    ]
}

fn discord_video_bitrate_kbps(duration_secs: f64, audio_kbps: u64) -> u64 {
    let duration_secs = duration_secs.max(1.0);
    let total_kbps = ((DISCORD_TARGET_BYTES as f64 * 8.0) / duration_secs / 1000.0) as u64;
    total_kbps.saturating_sub(audio_kbps).clamp(400, 8000)
}

pub fn scale_filter_for_max(max_w: u32, max_h: u32, width: u32, height: u32) -> String {
    if width <= max_w && height <= max_h {
        return String::new();
    }

    format!("scale='min({max_w},iw)':'min({max_h},ih)':force_original_aspect_ratio=decrease:force_divisible_by=2,format=yuv420p")
}

fn optional_scale_filter(max_w: u32, max_h: u32, width: u32, height: u32) -> Option<String> {
    let filter = scale_filter_for_max(max_w, max_h, width, height);
    if filter.is_empty() {
        None
    } else {
        Some(filter)
    }
}

pub fn apply_bitrate_scale(profile: &mut EncodeProfile, factor: f64) {
    if profile.lossless {
        return;
    }

    let mut next_args = Vec::new();
    let mut index = 0;

    while index < profile.video_args.len() {
        let key = &profile.video_args[index];
        if key == "-b:v" {
            let kbps = profile
                .video_args
                .get(index + 1)
                .and_then(|value| value.trim_end_matches('k').parse::<f64>().ok())
                .unwrap_or(2000.0);
            next_args.push("-b:v".to_string());
            next_args.push(format!("{}k", (kbps * factor).max(300.0) as u64));
            index += 2;
            continue;
        }

        next_args.push(key.clone());
        index += 1;
    }

    profile.video_args = next_args;
}

pub fn builtin_presets() -> Vec<PresetInfo> {
    vec![
        PresetInfo {
            id: PRESET_LOSSLESS.to_string(),
            name: "Lossless Trim".to_string(),
            description: "Fast stream-copy trim with no re-encode.".to_string(),
            lossless: true,
            requires_gpu: false,
            custom: false,
            target_bytes: None,
        },
        PresetInfo {
            id: PRESET_DISCORD.to_string(),
            name: "Discord".to_string(),
            description: "H.264/AAC sized for ~9 MB uploads.".to_string(),
            lossless: false,
            requires_gpu: false,
            custom: false,
            target_bytes: Some(DISCORD_TARGET_BYTES),
        },
        PresetInfo {
            id: PRESET_ARCHIVE.to_string(),
            name: "Archive".to_string(),
            description: "High-quality H.264 re-encode for keeping clips.".to_string(),
            lossless: false,
            requires_gpu: false,
            custom: false,
            target_bytes: None,
        },
        PresetInfo {
            id: PRESET_TWITTER.to_string(),
            name: "Twitter / X".to_string(),
            description: "720p H.264 with platform-friendly limits.".to_string(),
            lossless: false,
            requires_gpu: false,
            custom: false,
            target_bytes: None,
        },
    ]
}

impl CustomExportPreset {
    pub fn to_preset_info(&self) -> PresetInfo {
        PresetInfo {
            id: self.id.clone(),
            name: self.name.clone(),
            description: if self.description.is_empty() {
                custom_preset_summary(self)
            } else {
                self.description.clone()
            },
            lossless: self.lossless,
            requires_gpu: !self.lossless,
            custom: true,
            target_bytes: if self.mode == "target_size" {
                self.target_bytes
            } else {
                None
            },
        }
    }
}

fn custom_preset_summary(preset: &CustomExportPreset) -> String {
    if preset.lossless {
        return "Custom lossless stream-copy.".to_string();
    }

    match preset.mode.as_str() {
        "bitrate" => format!(
            "Custom H.264/AAC at {} kbps video.",
            preset.video_bitrate_kbps.unwrap_or(2500)
        ),
        "crf" => format!("Custom H.264/AAC at CRF {}.", preset.crf.unwrap_or(20)),
        "target_size" => {
            let megabytes =
                preset.target_bytes.unwrap_or(DISCORD_TARGET_BYTES) as f64 / (1024.0 * 1024.0);
            format!("Custom H.264/AAC targeting about {megabytes:.0} MB.")
        }
        _ => "Custom export preset.".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discord_scale_filter_uses_yuv420p_and_even_dimensions() {
        let filter = scale_filter_for_max(1280, 720, 1920, 1080);
        assert!(filter.contains("format=yuv420p"));
        assert!(filter.contains("force_divisible_by=2"));
    }

    #[test]
    fn libx264_fallback_replaces_gpu_codec() {
        let profile = EncodeProfile {
            lossless: false,
            video_codec: "h264_nvenc".to_string(),
            video_args: vec![
                "-b:v".to_string(),
                "2000k".to_string(),
                "-preset".to_string(),
                "p4".to_string(),
            ],
            audio_codec: "aac".to_string(),
            audio_args: vec![],
            scale_filter: None,
            target_bytes: None,
        };

        let fallback = libx264_fallback_profile(&profile);
        assert_eq!(fallback.video_codec, "libx264");
        assert!(fallback.video_args.contains(&"-b:v".to_string()));
        assert!(fallback.video_args.contains(&"2000k".to_string()));
    }
}

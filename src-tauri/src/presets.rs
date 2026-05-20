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
    builtin_presets()
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

    format!("scale='min({max_w},iw)':'min({max_h},ih)':force_original_aspect_ratio=decrease")
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
        },
        PresetInfo {
            id: PRESET_DISCORD.to_string(),
            name: "Discord".to_string(),
            description: "H.264/AAC sized for ~9 MB uploads.".to_string(),
            lossless: false,
            requires_gpu: false,
        },
        PresetInfo {
            id: PRESET_ARCHIVE.to_string(),
            name: "Archive".to_string(),
            description: "High-quality H.264 re-encode for keeping clips.".to_string(),
            lossless: false,
            requires_gpu: false,
        },
        PresetInfo {
            id: PRESET_TWITTER.to_string(),
            name: "Twitter / X".to_string(),
            description: "720p H.264 with platform-friendly limits.".to_string(),
            lossless: false,
            requires_gpu: false,
        },
    ]
}

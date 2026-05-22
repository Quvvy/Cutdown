use crate::clip_history::{append_entry, ClipHistoryEntry};
use crate::command_util::command;
use crate::ffmpeg_install;
use crate::presets::{
    self, apply_bitrate_scale, resolve_encode_profile, resolve_preset_id, EncodeProfile,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::Emitter;

static EXPORT_CANCELLED: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoMetadata {
    pub duration: f64,
    pub fps: f64,
    pub codec: String,
    pub width: u32,
    pub height: u32,
    pub file_size: u64,
    pub audio_codec: Option<String>,
    pub audio_channels: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportParams {
    input_path: String,
    output_path: String,
    segments: Vec<ExportSegment>,
    audio_mode: Option<AudioMode>,
    preset_id: Option<String>,
    prefer_gpu: Option<bool>,
    source_path: Option<String>,
    crop: Option<CropRect>,
    /// Playback/export gain multiplier (0.0–2.0, default 1.0).
    volume: Option<f64>,
    /// Re-encode segment boundaries for frame-accurate cuts (disables stream-copy).
    accurate_trim: Option<bool>,
    /// Fade in duration in seconds at the start of the exported output.
    fade_in_seconds: Option<f64>,
    /// Fade out duration in seconds at the end of the exported output.
    fade_out_seconds: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CropRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportSegment {
    start: f64,
    end: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportResult {
    output_path: String,
    file_size: u64,
    duration: f64,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AudioMode {
    Preserve,
    Strip,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportProgress {
    stage: String,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    percent: Option<f64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FfmpegCheckResult {
    available: bool,
    ffmpeg_path: String,
    ffprobe_path: String,
    source: String,
    message: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewParams {
    input_path: String,
    force_proxy: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewResult {
    preview_path: String,
    strategy: String,
}

#[tauri::command]
pub fn probe_video(path: String) -> Result<VideoMetadata, String> {
    let input = PathBuf::from(path);
    if !input.exists() {
        return Err("Input video does not exist.".to_string());
    }

    let output = command(ffprobe_path())
        .args([
            "-v",
            "error",
            "-print_format",
            "json",
            "-show_format",
            "-show_streams",
        ])
        .arg(&input)
        .output()
        .map_err(|err| format!("Failed to run ffprobe: {err}"))?;

    if !output.status.success() {
        return Err(command_error("ffprobe", &output.stderr));
    }

    let json: Value = serde_json::from_slice(&output.stdout)
        .map_err(|err| format!("Failed to parse ffprobe output: {err}"))?;
    let streams = json
        .get("streams")
        .and_then(Value::as_array)
        .ok_or_else(|| "ffprobe did not return stream metadata.".to_string())?;
    let video_stream = streams
        .iter()
        .find(|stream| stream.get("codec_type").and_then(Value::as_str) == Some("video"))
        .ok_or_else(|| "No video stream was found.".to_string())?;
    let audio_codec = streams
        .iter()
        .find(|stream| stream.get("codec_type").and_then(Value::as_str) == Some("audio"))
        .and_then(|stream| stream.get("codec_name"))
        .and_then(Value::as_str)
        .map(ToString::to_string);
    let audio_channels = streams
        .iter()
        .find(|stream| stream.get("codec_type").and_then(Value::as_str) == Some("audio"))
        .and_then(|stream| stream.get("channels"))
        .and_then(Value::as_u64);

    let duration = json
        .get("format")
        .and_then(|format| format.get("duration"))
        .and_then(parse_json_f64)
        .or_else(|| video_stream.get("duration").and_then(parse_json_f64))
        .unwrap_or(0.0);
    let fps = video_stream
        .get("avg_frame_rate")
        .and_then(Value::as_str)
        .and_then(parse_rate)
        .unwrap_or(0.0);
    let file_size = std::fs::metadata(&input)
        .map_err(|err| format!("Failed to read input metadata: {err}"))?
        .len();

    Ok(VideoMetadata {
        duration,
        fps,
        codec: video_stream
            .get("codec_name")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string(),
        width: video_stream
            .get("width")
            .and_then(Value::as_u64)
            .unwrap_or(0) as u32,
        height: video_stream
            .get("height")
            .and_then(Value::as_u64)
            .unwrap_or(0) as u32,
        file_size,
        audio_codec,
        audio_channels,
    })
}

#[tauri::command]
pub async fn export_clip(
    app: tauri::AppHandle,
    params: ExportParams,
) -> Result<ExportResult, String> {
    tauri::async_runtime::spawn_blocking(move || export_clip_blocking(app, params))
        .await
        .map_err(|err| format!("Export worker failed: {err}"))?
}

fn export_clip_blocking(
    app: tauri::AppHandle,
    params: ExportParams,
) -> Result<ExportResult, String> {
    EXPORT_CANCELLED.store(false, Ordering::SeqCst);
    let input = PathBuf::from(&params.input_path);
    let output = PathBuf::from(&params.output_path);
    let audio_mode = params.audio_mode.unwrap_or(AudioMode::Preserve);
    let preset_id = resolve_preset_id(params.preset_id);
    let prefer_gpu = params.prefer_gpu.unwrap_or(true);

    if !input.exists() {
        return Err("Input video does not exist.".to_string());
    }

    let segments = valid_segments(params.segments)?;
    let duration = segments
        .iter()
        .map(|segment| segment.end - segment.start)
        .sum::<f64>();

    let metadata = probe_video(params.input_path.clone())?;
    let gpu_encoders = crate::encoder_detect::detect_gpu_encoders();
    let mut profile = resolve_encode_profile(
        &preset_id,
        prefer_gpu,
        &gpu_encoders,
        duration,
        metadata.width,
        metadata.height,
    )?;

    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("Failed to create output directory: {err}"))?;
    }

    let preset_name = presets::all_preset_infos()
        .into_iter()
        .find(|preset| preset.id == preset_id)
        .map(|preset| preset.name)
        .unwrap_or_else(|| preset_id.clone());

    emit_export_progress(
        &app,
        "starting",
        &format!("Preparing export with {preset_name}."),
        None,
    );

    let crop = normalize_crop(params.crop.as_ref(), metadata.width, metadata.height);
    let volume = normalize_volume(params.volume);
    let accurate_trim = params.accurate_trim.unwrap_or(false);
    let fade_in = normalize_fade(params.fade_in_seconds);
    let fade_out = normalize_fade(params.fade_out_seconds);
    let export_duration = duration;
    let use_lossless = profile.lossless
        && crop.is_none()
        && !volume_is_adjusted(volume)
        && !accurate_trim
        && fade_in <= 0.0
        && fade_out <= 0.0;

    if use_lossless {
        if segments.len() == 1 {
            let segment = &segments[0];
            let segment_duration = segment.end - segment.start;
            emit_export_progress(&app, "segment", "Exporting single segment.", Some(0.0));
            export_segment(
                Some(&app),
                &input,
                &output,
                segment.start,
                segment_duration,
                audio_mode,
                volume,
                fade_in,
                fade_out,
                export_duration,
            )
            .map_err(|err| format!("Single-segment export failed: {err}"))?;
        } else {
            export_multi_segment_lossless(&app, &input, &output, &segments, audio_mode, volume)
                .map_err(|err| format!("Multi-segment export failed: {err}"))?;
        }
    } else {
        if profile.lossless {
            profile = lossless_crop_profile();
        }
        export_with_reencode(
            &app,
            &input,
            &output,
            &segments,
            audio_mode,
            &profile,
            crop.as_ref(),
            volume,
            fade_in,
            fade_out,
            export_duration,
            accurate_trim,
        )
        .map_err(|err| format!("Re-encode export failed: {err}"))?;
    }

    let mut file_size = fs::metadata(&output)
        .map_err(|err| format!("Failed to read exported file metadata: {err}"))?
        .len();

    if let Some(target_bytes) = profile.target_bytes {
        let mut attempts = 0;
        while file_size > target_bytes && attempts < 2 {
            attempts += 1;
            apply_bitrate_scale(&mut profile, 0.82);
            emit_export_progress(
                &app,
                "retry",
                &format!("Output exceeded target size, retrying ({attempts}/2)."),
                Some(5.0),
            );
            export_with_reencode(
                &app,
                &input,
                &output,
                &segments,
                audio_mode,
                &profile,
                crop.as_ref(),
                volume,
                fade_in,
                fade_out,
                export_duration,
                accurate_trim,
            )?;
            file_size = fs::metadata(&output)
                .map_err(|err| format!("Failed to read exported file metadata: {err}"))?
                .len();
        }
    }

    emit_export_progress(&app, "complete", "Export complete.", Some(100.0));

    let output_path = output.to_string_lossy().to_string();
    let _ = append_entry(ClipHistoryEntry {
        output_path: output_path.clone(),
        source_path: params.source_path.filter(|value| !value.trim().is_empty()),
        preset_id: preset_id.clone(),
        exported_at: Utc::now().to_rfc3339(),
        file_size,
        duration,
        share_url: None,
    });

    Ok(ExportResult {
        output_path,
        file_size,
        duration,
    })
}

#[tauri::command]
pub fn cancel_export() {
    EXPORT_CANCELLED.store(true, Ordering::SeqCst);
}

#[allow(clippy::too_many_arguments)]
fn export_with_reencode(
    app: &tauri::AppHandle,
    input: &Path,
    output: &Path,
    segments: &[ExportSegment],
    audio_mode: AudioMode,
    profile: &EncodeProfile,
    crop: Option<&CropRect>,
    volume: f64,
    fade_in: f64,
    fade_out: f64,
    output_duration: f64,
    accurate_trim: bool,
) -> Result<(), String> {
    if segments.len() == 1 {
        let segment = &segments[0];
        export_reencoded_segment(
            Some(app),
            input,
            output,
            segment.start,
            segment.end - segment.start,
            profile,
            audio_mode,
            crop,
            volume,
            fade_in,
            fade_out,
            segment.end - segment.start,
            accurate_trim,
        )
    } else {
        export_multi_segment_reencode(
            app,
            input,
            output,
            segments,
            profile,
            audio_mode,
            crop,
            volume,
            accurate_trim,
        )?;
        if fade_in > 0.0 || fade_out > 0.0 {
            apply_output_audio_fades(
                output,
                audio_mode,
                volume,
                fade_in,
                fade_out,
                output_duration,
            )?;
        }
        Ok(())
    }
}

fn export_multi_segment_lossless(
    app: &tauri::AppHandle,
    input: &Path,
    output: &Path,
    segments: &[ExportSegment],
    audio_mode: AudioMode,
    volume: f64,
) -> Result<(), String> {
    export_multi_segment(app, input, output, segments, audio_mode, volume)
}

#[allow(clippy::too_many_arguments)]
fn export_multi_segment_reencode(
    app: &tauri::AppHandle,
    input: &Path,
    output: &Path,
    segments: &[ExportSegment],
    profile: &EncodeProfile,
    audio_mode: AudioMode,
    crop: Option<&CropRect>,
    volume: f64,
    accurate_trim: bool,
) -> Result<(), String> {
    let temp_dir = std::env::temp_dir().join(format!(
        "cutdown-reencode-{}-{}",
        std::process::id(),
        current_timestamp_millis()
    ));
    fs::create_dir_all(&temp_dir)
        .map_err(|err| format!("Failed to create temp directory: {err}"))?;

    let result = (|| {
        let mut segment_paths = Vec::with_capacity(segments.len());
        let total_segments = segments.len() as f64;

        for (index, segment) in segments.iter().enumerate() {
            let segment_duration = segment.end - segment.start;
            let base_percent = (index as f64 / total_segments) * 90.0;
            emit_export_progress(
                app,
                "segment",
                &format!("Encoding segment {} of {}.", index + 1, segments.len()),
                Some(base_percent),
            );
            let segment_path = temp_dir.join(format!("segment-{index:03}.mp4"));
            export_reencoded_segment(
                Some(app),
                input,
                &segment_path,
                segment.start,
                segment_duration,
                profile,
                audio_mode,
                crop,
                volume,
                0.0,
                0.0,
                segment_duration,
                accurate_trim,
            )?;
            segment_paths.push(segment_path);
        }

        emit_export_progress(app, "concat", "Joining encoded segments.", Some(92.0));
        let concat_list = temp_dir.join("segments.txt");
        fs::write(&concat_list, concat_file_contents(&segment_paths))
            .map_err(|err| format!("Failed to write concat list: {err}"))?;

        let mut command = command(ffmpeg_path());
        command
            .arg("-y")
            .args(["-f", "concat", "-safe", "0"])
            .arg("-i")
            .arg(&concat_list)
            .args(["-c", "copy"])
            .arg(output);

        run_command_with_progress(
            Some(app),
            command,
            "concat",
            None,
            "Joining encoded segments.",
        )
    })();

    let _ = fs::remove_dir_all(&temp_dir);

    result
}

#[allow(clippy::too_many_arguments)]
fn export_reencoded_segment(
    app: Option<&tauri::AppHandle>,
    input: &Path,
    output: &Path,
    start: f64,
    duration: f64,
    profile: &EncodeProfile,
    audio_mode: AudioMode,
    crop: Option<&CropRect>,
    volume: f64,
    fade_in: f64,
    fade_out: f64,
    output_duration: f64,
    accurate_trim: bool,
) -> Result<(), String> {
    if duration <= 0.0 {
        return Err("Segment end must be after segment start.".to_string());
    }

    let mut command = command(ffmpeg_path());
    command.arg("-y");
    if accurate_trim {
        command
            .arg("-i")
            .arg(input)
            .arg("-ss")
            .arg(format_seconds(start))
            .arg("-t")
            .arg(format_seconds(duration));
    } else {
        command
            .arg("-ss")
            .arg(format_seconds(start))
            .arg("-i")
            .arg(input)
            .arg("-t")
            .arg(format_seconds(duration));
    }

    if let Some(filter) = build_video_filter(profile, crop) {
        command.args(["-vf", &filter]);
    }

    command.arg("-c:v").arg(&profile.video_codec);
    for arg in &profile.video_args {
        command.arg(arg);
    }

    append_audio_output(
        &mut command,
        audio_mode,
        volume,
        Some(profile),
        fade_in,
        fade_out,
        output_duration,
    );

    command.args(["-movflags", "+faststart"]).arg(output);

    run_command_with_progress(app, command, "encode", Some(duration), "Encoding segment.")
}

fn valid_segments(segments: Vec<ExportSegment>) -> Result<Vec<ExportSegment>, String> {
    let segments = segments
        .into_iter()
        .filter(|segment| segment.end > segment.start)
        .collect::<Vec<_>>();

    if segments.is_empty() {
        return Err("At least one segment is required for export.".to_string());
    }

    Ok(segments)
}

fn export_multi_segment(
    app: &tauri::AppHandle,
    input: &Path,
    output: &Path,
    segments: &[ExportSegment],
    audio_mode: AudioMode,
    volume: f64,
) -> Result<(), String> {
    let temp_dir = std::env::temp_dir().join(format!(
        "cutdown-{}-{}",
        std::process::id(),
        current_timestamp_millis()
    ));
    fs::create_dir_all(&temp_dir)
        .map_err(|err| format!("Failed to create temp directory: {err}"))?;

    let result = (|| {
        let extension = output
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("mp4");
        let mut segment_paths = Vec::with_capacity(segments.len());

        let total_segments = segments.len() as f64;

        for (index, segment) in segments.iter().enumerate() {
            let segment_duration = segment.end - segment.start;
            let base_percent = (index as f64 / total_segments) * 90.0;
            emit_export_progress(
                app,
                "segment",
                &format!("Exporting segment {} of {}.", index + 1, segments.len()),
                Some(base_percent),
            );
            let segment_path = temp_dir.join(format!("segment-{index:03}.{extension}"));
            export_segment(
                Some(app),
                input,
                &segment_path,
                segment.start,
                segment_duration,
                audio_mode,
                volume,
                0.0,
                0.0,
                segment_duration,
            )?;
            segment_paths.push(segment_path);
        }

        emit_export_progress(app, "concat", "Joining exported segments.", Some(92.0));
        let concat_list = temp_dir.join("segments.txt");
        fs::write(&concat_list, concat_file_contents(&segment_paths))
            .map_err(|err| format!("Failed to write concat list: {err}"))?;

        let mut command = command(ffmpeg_path());
        command
            .arg("-y")
            .args(["-f", "concat", "-safe", "0"])
            .arg("-i")
            .arg(&concat_list)
            .args(["-c", "copy"])
            .arg(output);

        run_command_with_progress(
            Some(app),
            command,
            "concat",
            None,
            "Joining exported segments.",
        )?;

        Ok(())
    })();

    let cleanup = fs::remove_dir_all(&temp_dir);
    if result.is_ok() {
        cleanup.map_err(|err| format!("Failed to remove temp files: {err}"))?;
    } else {
        let _ = cleanup;
    }

    result
}

#[allow(clippy::too_many_arguments)]
fn export_segment(
    app: Option<&tauri::AppHandle>,
    input: &Path,
    output: &Path,
    start: f64,
    duration: f64,
    audio_mode: AudioMode,
    volume: f64,
    fade_in: f64,
    fade_out: f64,
    output_duration: f64,
) -> Result<(), String> {
    if duration <= 0.0 {
        return Err("Segment end must be after segment start.".to_string());
    }

    let mut command = command(ffmpeg_path());
    command
        .arg("-y")
        .arg("-ss")
        .arg(format_seconds(start))
        .arg("-i")
        .arg(input)
        .arg("-t")
        .arg(format_seconds(duration));

    if (volume_is_adjusted(volume) || fade_in > 0.0 || fade_out > 0.0)
        && !matches!(audio_mode, AudioMode::Strip)
    {
        command.arg("-c:v").arg("copy");
        append_audio_output(
            &mut command,
            audio_mode,
            volume,
            None,
            fade_in,
            fade_out,
            output_duration,
        );
    } else {
        command.args(["-c", "copy", "-avoid_negative_ts", "make_zero"]);
        if matches!(audio_mode, AudioMode::Strip) {
            command.arg("-an");
        }
    }

    command.arg(output);

    run_command_with_progress(
        app,
        command,
        "segment",
        Some(duration),
        "Exporting segment.",
    )
}

#[tauri::command]
pub fn reveal_in_explorer(path: String) -> Result<(), String> {
    let path = PathBuf::from(path);
    let target = if path.is_dir() {
        path
    } else {
        path.parent()
            .map(Path::to_path_buf)
            .ok_or_else(|| "Could not resolve output directory.".to_string())?
    };

    command("explorer")
        .arg(target)
        .spawn()
        .map_err(|err| format!("Failed to open Explorer: {err}"))?;

    Ok(())
}

#[tauri::command]
pub async fn prepare_preview(
    app: tauri::AppHandle,
    params: PreviewParams,
) -> Result<PreviewResult, String> {
    tauri::async_runtime::spawn_blocking(move || prepare_preview_blocking(app, params))
        .await
        .map_err(|err| format!("Preview worker failed: {err}"))?
}

fn prepare_preview_blocking(
    app: tauri::AppHandle,
    params: PreviewParams,
) -> Result<PreviewResult, String> {
    let input = PathBuf::from(params.input_path);

    if !input.exists() {
        return Err("Input video does not exist.".to_string());
    }

    let temp_dir = preview_temp_dir();
    fs::create_dir_all(&temp_dir)
        .map_err(|err| format!("Failed to create preview temp directory: {err}"))?;

    if !params.force_proxy {
        let remux_path = temp_dir.join(format!(
            "preview-remux-{}-{}.mp4",
            std::process::id(),
            current_timestamp_millis()
        ));

        match remux_preview(&input, &remux_path) {
            Ok(()) => {
                return Ok(PreviewResult {
                    preview_path: remux_path.to_string_lossy().to_string(),
                    strategy: "Preview remux".to_string(),
                });
            }
            Err(err) => {
                let _ = fs::remove_file(&remux_path);
                eprintln!("preview remux failed, falling back to proxy: {err}");
            }
        }
    }

    let proxy_path = temp_dir.join(format!(
        "preview-proxy-{}-{}.mp4",
        std::process::id(),
        current_timestamp_millis()
    ));
    proxy_preview(&input, &proxy_path, Some(&app))?;

    Ok(PreviewResult {
        preview_path: proxy_path.to_string_lossy().to_string(),
        strategy: "Preview proxy".to_string(),
    })
}

#[tauri::command]
pub fn cleanup_preview(path: String) -> Result<(), String> {
    let path = PathBuf::from(path);

    if path.exists() {
        ensure_preview_path(&path)?;
        fs::remove_file(&path).map_err(|err| format!("Failed to clean up preview file: {err}"))?;
    }

    Ok(())
}

fn preview_temp_dir() -> PathBuf {
    std::env::temp_dir().join("cutdown-preview")
}

fn ensure_preview_path(path: &Path) -> Result<(), String> {
    let preview_root = preview_temp_dir()
        .canonicalize()
        .map_err(|err| format!("Failed to resolve preview temp directory: {err}"))?;
    let target = path
        .canonicalize()
        .map_err(|err| format!("Failed to resolve preview file: {err}"))?;

    if !target.starts_with(&preview_root) {
        return Err("Refusing to clean up a file outside Cutdown's preview directory.".to_string());
    }

    if !target.is_file() {
        return Err("Preview cleanup target is not a file.".to_string());
    }

    Ok(())
}

fn remux_preview(input: &Path, output: &Path) -> Result<(), String> {
    let mut command = command(ffmpeg_path());
    command
        .arg("-y")
        .arg("-i")
        .arg(input)
        .args([
            "-map",
            "0:v:0",
            "-map",
            "0:a?",
            "-c",
            "copy",
            "-movflags",
            "+faststart",
        ])
        .arg(output);

    run_command_with_progress(None, command, "preview", None, "Preparing preview remux.")
}

fn proxy_preview(
    input: &Path,
    output: &Path,
    app: Option<&tauri::AppHandle>,
) -> Result<(), String> {
    let duration = probe_duration_seconds(input).ok();
    let mut command = command(ffmpeg_path());
    command
        .arg("-y")
        .arg("-i")
        .arg(input)
        .args([
            "-map",
            "0:v:0",
            "-map",
            "0:a?",
            "-c:v",
            "libx264",
            "-preset",
            "veryfast",
            "-crf",
            "23",
            "-c:a",
            "aac",
            "-b:a",
            "160k",
            "-movflags",
            "+faststart",
        ])
        .arg(output);

    run_command_with_progress(
        app,
        command,
        "preview",
        duration,
        "Generating preview proxy.",
    )
}

pub fn ffmpeg_is_available() -> bool {
    check_ffmpeg().available
}

#[tauri::command]
pub fn check_ffmpeg() -> FfmpegCheckResult {
    let ffmpeg = resolve_binary("ffmpeg.exe", "ffmpeg");
    let ffprobe = resolve_binary("ffprobe.exe", "ffprobe");

    let available = ffmpeg.available && ffprobe.available;
    let message = if available {
        format!(
            "Using {} ffmpeg and {} ffprobe.",
            ffmpeg.source, ffprobe.source
        )
    } else {
        "ffmpeg is not installed. Re-run the Cutdown installer (requires internet), use Download ffmpeg in the app, install ffmpeg on PATH, or run npm run prepare:ffmpeg for development.".to_string()
    };

    FfmpegCheckResult {
        available,
        ffmpeg_path: ffmpeg.path,
        ffprobe_path: ffprobe.path,
        source: if ffmpeg.source == ffprobe.source {
            ffmpeg.source
        } else {
            format!("{} / {}", ffmpeg.source, ffprobe.source)
        },
        message,
    }
}

struct ResolvedBinary {
    path: String,
    source: String,
    available: bool,
}

fn resolve_binary(bundled_name: &str, path_name: &str) -> ResolvedBinary {
    if let Some(path) = bundled_binary(bundled_name) {
        return ResolvedBinary {
            path: path.to_string_lossy().to_string(),
            source: binary_source(&path),
            available: true,
        };
    }

    let path = PathBuf::from(path_name);
    let available = command(&path)
        .arg("-version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false);

    ResolvedBinary {
        path: path.to_string_lossy().to_string(),
        source: "PATH".to_string(),
        available,
    }
}

fn probe_duration_seconds(input: &Path) -> Result<f64, String> {
    let output = command(ffprobe_path())
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
        ])
        .arg(input)
        .output()
        .map_err(|err| format!("Failed to run ffprobe: {err}"))?;

    if !output.status.success() {
        return Err(command_error("ffprobe", &output.stderr));
    }

    let raw = String::from_utf8_lossy(&output.stdout);
    raw.trim()
        .parse::<f64>()
        .map_err(|err| format!("Failed to parse ffprobe duration: {err}"))
}

fn run_command_with_progress(
    app: Option<&tauri::AppHandle>,
    mut command: Command,
    stage: &str,
    total_duration: Option<f64>,
    message: &str,
) -> Result<(), String> {
    command.stderr(Stdio::piped()).stdout(Stdio::null());

    let mut child = command
        .spawn()
        .map_err(|err| format!("Failed to start ffmpeg: {err}"))?;

    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "ffmpeg did not provide stderr output.".to_string())?;

    let reader = BufReader::new(stderr);
    let mut parsed_duration = total_duration;
    let mut last_percent = -1.0;

    for line in reader.lines() {
        let line = line.map_err(|err| format!("Failed to read ffmpeg output: {err}"))?;

        if EXPORT_CANCELLED.load(Ordering::SeqCst) {
            let _ = child.kill();
            let _ = child.wait();
            EXPORT_CANCELLED.store(false, Ordering::SeqCst);
            return Err("Export was cancelled.".to_string());
        }

        if parsed_duration.is_none() {
            parsed_duration = parse_duration_line(&line);
        }

        if let (Some(total), Some(current)) = (parsed_duration, parse_time_line(&line)) {
            if total > 0.0 {
                let percent = ((current / total) * 100.0).clamp(0.0, 99.0);
                if let Some(app) = app.as_ref() {
                    if (percent - last_percent).abs() >= 0.5 {
                        last_percent = percent;
                        emit_export_progress(app, stage, message, Some(percent));
                    }
                }
            }
        }
    }

    let status = child
        .wait()
        .map_err(|err| format!("Failed to wait for ffmpeg: {err}"))?;

    if !status.success() {
        return Err(format!("{message} ffmpeg exited with status {status}."));
    }

    if let Some(app) = app.as_ref() {
        emit_export_progress(app, stage, message, Some(100.0));
    }

    Ok(())
}

fn parse_duration_line(line: &str) -> Option<f64> {
    let marker = "Duration:";
    let index = line.find(marker)?;
    let rest = line[index + marker.len()..].trim();
    let time = rest.split(',').next()?.trim();
    parse_ffmpeg_timestamp(time)
}

fn parse_time_line(line: &str) -> Option<f64> {
    let marker = "time=";
    let index = line.find(marker)?;
    let rest = &line[index + marker.len()..];
    let time = rest.split_whitespace().next()?;
    parse_ffmpeg_timestamp(time)
}

fn parse_ffmpeg_timestamp(raw: &str) -> Option<f64> {
    let parts: Vec<&str> = raw.trim().split(':').collect();
    match parts.len() {
        3 => {
            let hours: f64 = parts[0].parse().ok()?;
            let minutes: f64 = parts[1].parse().ok()?;
            let seconds: f64 = parts[2].parse().ok()?;
            Some(hours * 3600.0 + minutes * 60.0 + seconds)
        }
        2 => {
            let minutes: f64 = parts[0].parse().ok()?;
            let seconds: f64 = parts[1].parse().ok()?;
            Some(minutes * 60.0 + seconds)
        }
        _ => None,
    }
}

fn normalize_crop(
    crop: Option<&CropRect>,
    video_width: u32,
    video_height: u32,
) -> Option<CropRect> {
    let crop = crop?;
    if crop.width < 2 || crop.height < 2 || video_width < 2 || video_height < 2 {
        return None;
    }

    let even = |value: u32| -> u32 { value - (value % 2) };
    let max_w = even(video_width);
    let max_h = even(video_height);
    let width = even(crop.width.min(max_w));
    let height = even(crop.height.min(max_h));
    let x = even(crop.x.min(max_w.saturating_sub(width)));
    let y = even(crop.y.min(max_h.saturating_sub(height)));

    if width < 2 || height < 2 {
        return None;
    }

    Some(CropRect {
        x,
        y,
        width,
        height,
    })
}

fn build_video_filter(profile: &EncodeProfile, crop: Option<&CropRect>) -> Option<String> {
    let mut parts = Vec::new();

    if let Some(crop) = crop {
        parts.push(format!(
            "crop={}:{}:{}:{}",
            crop.width, crop.height, crop.x, crop.y
        ));
    }

    if let Some(scale) = &profile.scale_filter {
        if !scale.is_empty() {
            parts.push(scale.clone());
        }
    }

    if parts.is_empty() {
        None
    } else {
        Some(parts.join(","))
    }
}

fn normalize_volume(volume: Option<f64>) -> f64 {
    match volume {
        Some(value) if value.is_finite() => value.clamp(0.0, 2.0),
        _ => 1.0,
    }
}

fn volume_is_adjusted(volume: f64) -> bool {
    (volume - 1.0).abs() > 0.001
}

fn normalize_fade(seconds: Option<f64>) -> f64 {
    match seconds {
        Some(value) if value.is_finite() => value.clamp(0.0, 30.0),
        _ => 0.0,
    }
}

fn audio_filter_chain(
    volume: f64,
    fade_in: f64,
    fade_out: f64,
    output_duration: f64,
) -> Option<String> {
    let mut parts = Vec::new();

    if volume_is_adjusted(volume) {
        parts.push(format!("volume={volume:.4}"));
    }

    if fade_in > 0.0 {
        parts.push(format!("afade=t=in:st=0:d={fade_in:.3}"));
    }

    if fade_out > 0.0 && output_duration > fade_out {
        parts.push(format!(
            "afade=t=out:st={:.3}:d={fade_out:.3}",
            (output_duration - fade_out).max(0.0)
        ));
    }

    if parts.is_empty() {
        None
    } else {
        Some(parts.join(","))
    }
}

fn append_audio_output(
    command: &mut Command,
    audio_mode: AudioMode,
    volume: f64,
    profile: Option<&EncodeProfile>,
    fade_in: f64,
    fade_out: f64,
    output_duration: f64,
) {
    if matches!(audio_mode, AudioMode::Strip) {
        command.arg("-an");
        return;
    }

    if let Some(filter) = audio_filter_chain(volume, fade_in, fade_out, output_duration) {
        command.args(["-af", &filter]);
    }

    if let Some(profile) = profile {
        command.arg("-c:a").arg(&profile.audio_codec);
        for arg in &profile.audio_args {
            command.arg(arg);
        }
        return;
    }

    command.args(["-c:a", "aac", "-b:a", "192k"]);
}

fn apply_output_audio_fades(
    output: &Path,
    audio_mode: AudioMode,
    volume: f64,
    fade_in: f64,
    fade_out: f64,
    output_duration: f64,
) -> Result<(), String> {
    if matches!(audio_mode, AudioMode::Strip) {
        return Ok(());
    }

    let Some(filter) = audio_filter_chain(volume, fade_in, fade_out, output_duration) else {
        return Ok(());
    };

    let temp_output = output.with_extension("fade-tmp.mp4");
    let mut command = command(ffmpeg_path());
    command
        .arg("-y")
        .arg("-i")
        .arg(output)
        .args(["-af", &filter])
        .arg("-c:v")
        .arg("copy")
        .args(["-c:a", "aac", "-b:a", "192k"])
        .arg(&temp_output);

    run_command_with_progress(None, command, "fade", None, "Applying audio fades.")?;

    fs::rename(&temp_output, output)
        .map_err(|err| format!("Failed to replace output after audio fade: {err}"))
}

fn lossless_crop_profile() -> EncodeProfile {
    EncodeProfile {
        lossless: false,
        video_codec: "libx264".to_string(),
        video_args: vec![
            "-crf".to_string(),
            "16".to_string(),
            "-preset".to_string(),
            "fast".to_string(),
        ],
        audio_codec: "aac".to_string(),
        audio_args: vec!["-b:a".to_string(), "192k".to_string()],
        scale_filter: None,
        target_bytes: None,
    }
}

fn emit_export_progress(app: &tauri::AppHandle, stage: &str, message: &str, percent: Option<f64>) {
    let _ = app.emit(
        "export_progress",
        ExportProgress {
            stage: stage.to_string(),
            message: message.to_string(),
            percent,
        },
    );
}

fn concat_file_contents(paths: &[PathBuf]) -> String {
    paths
        .iter()
        .map(|path| {
            let normalized = path
                .to_string_lossy()
                .replace('\\', "/")
                .replace('\'', "'\\''");
            format!("file '{normalized}'")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn current_timestamp_millis() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}

pub(crate) fn ffmpeg_path() -> PathBuf {
    locate_binary("ffmpeg.exe").unwrap_or_else(|| PathBuf::from("ffmpeg"))
}

pub(crate) fn ffprobe_path() -> PathBuf {
    locate_binary("ffprobe.exe").unwrap_or_else(|| PathBuf::from("ffprobe"))
}

fn locate_binary(name: &str) -> Option<PathBuf> {
    let user_path = ffmpeg_install::user_ffmpeg_path(name);
    if user_path.exists() {
        return Some(user_path);
    }

    let dev_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("public")
        .join("ffmpeg")
        .join(name);

    if dev_path.exists() {
        return Some(dev_path);
    }

    None
}

fn bundled_binary(name: &str) -> Option<PathBuf> {
    locate_binary(name)
}

fn binary_source(path: &Path) -> String {
    let user_dir = ffmpeg_install::user_ffmpeg_dir();
    if path.starts_with(&user_dir) {
        return "installed".to_string();
    }

    if path
        .components()
        .any(|component| component.as_os_str() == "public")
    {
        return "development".to_string();
    }

    "path".to_string()
}

fn parse_json_f64(value: &Value) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_str().and_then(|raw| raw.parse::<f64>().ok()))
}

fn parse_rate(rate: &str) -> Option<f64> {
    let (numerator, denominator) = rate.split_once('/')?;
    let numerator = numerator.parse::<f64>().ok()?;
    let denominator = denominator.parse::<f64>().ok()?;

    if denominator == 0.0 {
        return None;
    }

    Some(numerator / denominator)
}

fn format_seconds(seconds: f64) -> String {
    format!("{:.3}", seconds.max(0.0))
}

fn command_error(command: &str, stderr: &[u8]) -> String {
    let stderr = String::from_utf8_lossy(stderr);
    let detail = stderr.trim();

    if detail.is_empty() {
        format!("{command} failed without an error message.")
    } else {
        format!("{command} failed: {detail}")
    }
}

fn waveform_sample_rate(duration: f64, bucket_count: usize) -> u32 {
    if !duration.is_finite() || duration <= 0.0 {
        return 4000;
    }

    // Enough samples per bucket for a usable shape; cap total decode for long clips.
    const MIN_SAMPLES_PER_BUCKET: f64 = 4.0;
    const MAX_DECODE_SAMPLES: f64 = 80_000.0;

    let min_rate = (MIN_SAMPLES_PER_BUCKET * bucket_count as f64 / duration).ceil();
    let cap_rate = (MAX_DECODE_SAMPLES / duration).ceil();
    min_rate.min(cap_rate).clamp(50.0, 4000.0) as u32
}

/// Downsampled peak envelope (0.0–1.0) for timeline waveform display.
#[tauri::command]
pub async fn extract_waveform(
    path: String,
    bucket_count: Option<u32>,
    has_audio: Option<bool>,
    duration: Option<f64>,
) -> Result<Vec<f32>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        extract_waveform_blocking(path, bucket_count, has_audio, duration)
    })
    .await
    .map_err(|err| format!("Waveform worker failed: {err}"))?
}

fn extract_waveform_blocking(
    path: String,
    bucket_count: Option<u32>,
    has_audio: Option<bool>,
    duration: Option<f64>,
) -> Result<Vec<f32>, String> {
    let input = PathBuf::from(path.trim());
    if !input.exists() {
        return Err("Input video does not exist.".to_string());
    }

    match has_audio {
        Some(false) => return Ok(Vec::new()),
        None => {
            let metadata = probe_video(input.to_string_lossy().to_string())?;
            if metadata.audio_codec.is_none() {
                return Ok(Vec::new());
            }
        }
        Some(true) => {}
    }

    let bucket_count = bucket_count.unwrap_or(2000).clamp(64, 8000) as usize;
    let sample_rate = waveform_sample_rate(duration.unwrap_or(0.0), bucket_count);
    let child = command(ffmpeg_path())
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-i",
            input.to_string_lossy().as_ref(),
            "-vn",
            "-ac",
            "1",
            "-ar",
            &sample_rate.to_string(),
            "-f",
            "f32le",
            "pipe:1",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| format!("Failed to run ffmpeg for waveform: {err}"))?;

    let output = child
        .wait_with_output()
        .map_err(|err| format!("Failed to read ffmpeg waveform output: {err}"))?;

    if !output.status.success() {
        return Err(command_error("ffmpeg", &output.stderr));
    }

    let raw = output.stdout;
    if raw.len() < 4 {
        return Ok(Vec::new());
    }

    let sample_count = raw.len() / 4;
    let mut peaks = vec![0.0f32; bucket_count];
    let samples_per_bucket = (sample_count as f64 / bucket_count as f64).max(1.0);

    for (bucket, peak) in peaks.iter_mut().enumerate().take(bucket_count) {
        let start = (bucket as f64 * samples_per_bucket) as usize;
        let end = (((bucket + 1) as f64) * samples_per_bucket) as usize;
        let end = end.min(sample_count);
        let mut max_peak = 0.0f32;

        for sample_index in start..end {
            let offset = sample_index * 4;
            if offset + 4 > raw.len() {
                break;
            }
            let bytes: [u8; 4] = raw[offset..offset + 4]
                .try_into()
                .map_err(|_| "Invalid waveform sample.".to_string())?;
            let value = f32::from_le_bytes(bytes).abs();
            if value.is_finite() && value > max_peak {
                max_peak = value;
            }
        }

        *peak = max_peak;
    }

    let normalize = peaks.iter().copied().fold(0.0f32, f32::max);
    if normalize > 0.0 {
        for peak in &mut peaks {
            *peak = (*peak / normalize).clamp(0.0, 1.0);
        }
    }

    Ok(peaks)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ffmpeg_timestamps() {
        assert_eq!(parse_ffmpeg_timestamp("00:01:02.500"), Some(62.5));
        assert_eq!(parse_ffmpeg_timestamp("01:02.250"), Some(62.25));
        assert_eq!(parse_ffmpeg_timestamp("not-time"), None);
    }

    #[test]
    fn parses_duration_and_progress_lines() {
        assert_eq!(
            parse_duration_line("Duration: 00:02:03.00, start: 0.000000"),
            Some(123.0)
        );
        assert_eq!(
            parse_time_line("frame=1 fps=0.0 q=-1.0 time=00:00:05.50 bitrate=N/A"),
            Some(5.5)
        );
    }

    #[test]
    fn normalizes_crop_to_video_bounds_and_even_dimensions() {
        let crop = CropRect {
            x: 1919,
            y: 1079,
            width: 99,
            height: 99,
        };

        let normalized = normalize_crop(Some(&crop), 1920, 1080).expect("crop should clamp");

        assert_eq!(normalized.x, 1822);
        assert_eq!(normalized.y, 982);
        assert_eq!(normalized.width, 98);
        assert_eq!(normalized.height, 98);
    }

    #[test]
    fn rejects_invalid_export_segments() {
        let segments = vec![
            ExportSegment {
                start: 5.0,
                end: 5.0,
            },
            ExportSegment {
                start: 9.0,
                end: 2.0,
            },
        ];

        assert!(valid_segments(segments).is_err());
    }

    #[test]
    fn keeps_valid_export_segments() {
        let segments = vec![
            ExportSegment {
                start: 0.0,
                end: 1.5,
            },
            ExportSegment {
                start: 4.0,
                end: 7.0,
            },
        ];

        let valid = valid_segments(segments).expect("segments should be valid");

        assert_eq!(valid.len(), 2);
        assert_eq!(valid[0].start, 0.0);
        assert_eq!(valid[1].end, 7.0);
    }

    #[test]
    fn clamps_waveform_sample_rate() {
        assert_eq!(waveform_sample_rate(0.0, 2000), 4000);
        assert_eq!(waveform_sample_rate(10_000.0, 2000), 50);
        assert_eq!(waveform_sample_rate(0.01, 8000), 4000);
    }
}

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tauri::Emitter;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoMetadata {
    duration: f64,
    fps: f64,
    codec: String,
    width: u32,
    height: u32,
    file_size: u64,
    audio_codec: Option<String>,
    audio_channels: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportParams {
    input_path: String,
    output_path: String,
    segments: Vec<ExportSegment>,
    audio_mode: Option<AudioMode>,
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

    let output = Command::new(ffprobe_path())
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
pub fn export_clip(app: tauri::AppHandle, params: ExportParams) -> Result<ExportResult, String> {
    let input = PathBuf::from(&params.input_path);
    let output = PathBuf::from(&params.output_path);
    let audio_mode = params.audio_mode.unwrap_or(AudioMode::Preserve);

    if !input.exists() {
        return Err("Input video does not exist.".to_string());
    }

    let segments = valid_segments(params.segments)?;
    let duration = segments
        .iter()
        .map(|segment| segment.end - segment.start)
        .sum::<f64>();

    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("Failed to create output directory: {err}"))?;
    }

    emit_export_progress(&app, "starting", "Preparing export.", None);

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
        )
        .map_err(|err| format!("Single-segment export failed: {err}"))?;
    } else {
        export_multi_segment(&app, &input, &output, &segments, audio_mode)
            .map_err(|err| format!("Multi-segment export failed: {err}"))?;
    }

    let file_size = fs::metadata(&output)
        .map_err(|err| format!("Failed to read exported file metadata: {err}"))?
        .len();

    emit_export_progress(&app, "complete", "Export complete.", Some(100.0));

    Ok(ExportResult {
        output_path: output.to_string_lossy().to_string(),
        file_size,
        duration,
    })
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
) -> Result<(), String> {
    let temp_dir = std::env::temp_dir().join(format!(
        "cutdown-{}-{}",
        std::process::id(),
        current_timestamp_millis()
    ));
    fs::create_dir_all(&temp_dir).map_err(|err| format!("Failed to create temp directory: {err}"))?;

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
            )?;
            segment_paths.push(segment_path);
        }

        emit_export_progress(app, "concat", "Joining exported segments.", Some(92.0));
        let concat_list = temp_dir.join("segments.txt");
        fs::write(&concat_list, concat_file_contents(&segment_paths))
            .map_err(|err| format!("Failed to write concat list: {err}"))?;

        let mut command = Command::new(ffmpeg_path());
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

fn export_segment(
    app: Option<&tauri::AppHandle>,
    input: &Path,
    output: &Path,
    start: f64,
    duration: f64,
    audio_mode: AudioMode,
) -> Result<(), String> {
    if duration <= 0.0 {
        return Err("Segment end must be after segment start.".to_string());
    }

    let mut command = Command::new(ffmpeg_path());
    command
        .arg("-y")
        .arg("-ss")
        .arg(format_seconds(start))
        .arg("-i")
        .arg(input)
        .arg("-t")
        .arg(format_seconds(duration))
        .args(["-c", "copy", "-avoid_negative_ts", "make_zero"]);

    if matches!(audio_mode, AudioMode::Strip) {
        command.arg("-an");
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

    Command::new("explorer")
        .arg(target)
        .spawn()
        .map_err(|err| format!("Failed to open Explorer: {err}"))?;

    Ok(())
}

#[tauri::command]
pub fn prepare_preview(app: tauri::AppHandle, params: PreviewParams) -> Result<PreviewResult, String> {
    let input = PathBuf::from(params.input_path);

    if !input.exists() {
        return Err("Input video does not exist.".to_string());
    }

    let temp_dir = std::env::temp_dir().join("cutdown-preview");
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
        fs::remove_file(&path).map_err(|err| format!("Failed to clean up preview file: {err}"))?;
    }

    Ok(())
}

fn remux_preview(input: &Path, output: &Path) -> Result<(), String> {
    let mut command = Command::new(ffmpeg_path());
    command
        .arg("-y")
        .arg("-i")
        .arg(input)
        .args(["-map", "0:v:0", "-map", "0:a?", "-c", "copy", "-movflags", "+faststart"])
        .arg(output);

    run_command_with_progress(None, command, "preview", None, "Preparing preview remux.")
}

fn proxy_preview(
    input: &Path,
    output: &Path,
    app: Option<&tauri::AppHandle>,
) -> Result<(), String> {
    let duration = probe_duration_seconds(input).ok();
    let mut command = Command::new(ffmpeg_path());
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
        "ffmpeg or ffprobe is missing. Install ffmpeg on PATH or run npm run prepare:ffmpeg.".to_string()
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
            source: "bundled".to_string(),
            available: true,
        };
    }

    let path = PathBuf::from(path_name);
    let available = Command::new(&path)
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
    let output = Command::new(ffprobe_path())
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

        if parsed_duration.is_none() {
            parsed_duration = parse_duration_line(&line);
        }

        if let (Some(total), Some(current)) = (parsed_duration, parse_time_line(&line)) {
            if total > 0.0 {
                let percent = ((current / total) * 100.0).clamp(0.0, 99.0);
                if let Some(app) = app.as_ref() {
                    if (percent - last_percent).abs() >= 0.5 {
                        last_percent = percent;
                        emit_export_progress(
                            app,
                            stage,
                            message,
                            Some(percent),
                        );
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

fn emit_export_progress(
    app: &tauri::AppHandle,
    stage: &str,
    message: &str,
    percent: Option<f64>,
) {
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
            let normalized = path.to_string_lossy().replace('\\', "/").replace('\'', "'\\''");
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

fn ffmpeg_path() -> PathBuf {
    bundled_binary("ffmpeg.exe").unwrap_or_else(|| PathBuf::from("ffmpeg"))
}

fn ffprobe_path() -> PathBuf {
    bundled_binary("ffprobe.exe").unwrap_or_else(|| PathBuf::from("ffprobe"))
}

fn bundled_binary(name: &str) -> Option<PathBuf> {
    let dev_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("public")
        .join("ffmpeg")
        .join(name);

    if dev_path.exists() {
        return Some(dev_path);
    }

    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf))?;

    [
        exe_dir.join("ffmpeg").join(name),
        exe_dir.join("resources").join("ffmpeg").join(name),
        exe_dir.join("..").join("Resources").join("ffmpeg").join(name),
    ]
    .into_iter()
    .find(|path| path.exists())
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

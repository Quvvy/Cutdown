use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;

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
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportParams {
    input_path: String,
    output_path: String,
    in_point: f64,
    out_point: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportResult {
    output_path: String,
    file_size: u64,
    duration: f64,
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
    })
}

#[tauri::command]
pub fn export_clip(params: ExportParams) -> Result<ExportResult, String> {
    let input = PathBuf::from(&params.input_path);
    let output = PathBuf::from(&params.output_path);
    let duration = params.out_point - params.in_point;

    if !input.exists() {
        return Err("Input video does not exist.".to_string());
    }

    if duration <= 0.0 {
        return Err("Out point must be after in point.".to_string());
    }

    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| format!("Failed to create output directory: {err}"))?;
    }

    let command_output = Command::new(ffmpeg_path())
        .arg("-y")
        .arg("-ss")
        .arg(format_seconds(params.in_point))
        .arg("-i")
        .arg(&input)
        .arg("-t")
        .arg(format_seconds(duration))
        .args(["-c", "copy", "-avoid_negative_ts", "make_zero"])
        .arg(&output)
        .output()
        .map_err(|err| format!("Failed to run ffmpeg: {err}"))?;

    if !command_output.status.success() {
        return Err(command_error("ffmpeg", &command_output.stderr));
    }

    let file_size = std::fs::metadata(&output)
        .map_err(|err| format!("Failed to read exported file metadata: {err}"))?
        .len();

    Ok(ExportResult {
        output_path: output.to_string_lossy().to_string(),
        file_size,
        duration,
    })
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

    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf))
        .map(|dir| dir.join("ffmpeg").join(name))
        .filter(|path| path.exists())
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

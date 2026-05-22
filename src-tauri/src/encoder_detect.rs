use crate::command_util::command;
use crate::ffmpeg;

#[tauri::command]
pub fn detect_gpu_encoders() -> Vec<String> {
    let output = command(ffmpeg::ffmpeg_path())
        .args(["-hide_banner", "-encoders"])
        .output();

    let Ok(output) = output else {
        return Vec::new();
    };

    let combined = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    ["h264_nvenc", "h264_amf", "h264_qsv"]
        .iter()
        .filter(|encoder| combined.contains(**encoder))
        .map(|encoder| encoder.to_string())
        .collect()
}

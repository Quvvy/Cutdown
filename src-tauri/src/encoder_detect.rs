use std::path::{Path, PathBuf};
use std::process::Command;

#[tauri::command]
pub fn detect_gpu_encoders() -> Vec<String> {
    let output = Command::new(ffmpeg_path())
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

fn ffmpeg_path() -> PathBuf {
    bundled_binary("ffmpeg.exe").unwrap_or_else(|| PathBuf::from("ffmpeg"))
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

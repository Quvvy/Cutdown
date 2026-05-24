use crate::command_util::command;
use serde::Serialize;
use std::fs::{self, File};
use std::io::{copy, Read, Write};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter};

/// Rolling "latest release" essentials package (Windows x64) from gyan.dev.
const FFMPEG_PACKAGE_URL: &str = "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FfmpegInstallProgress {
    pub stage: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub percent: Option<f64>,
}

pub fn user_ffmpeg_dir() -> PathBuf {
    #[cfg(windows)]
    {
        if let Ok(root) = std::env::var("LOCALAPPDATA") {
            return PathBuf::from(root).join("Cutdown").join("ffmpeg");
        }
    }

    dirs_fallback().join("ffmpeg")
}

pub fn install_log_path() -> PathBuf {
    #[cfg(windows)]
    {
        if let Ok(root) = std::env::var("LOCALAPPDATA") {
            return PathBuf::from(root)
                .join("Cutdown")
                .join("install-ffmpeg.log");
        }
    }

    dirs_fallback().join("install-ffmpeg.log")
}

fn dirs_fallback() -> PathBuf {
    std::env::temp_dir().join("Cutdown")
}

pub fn user_ffmpeg_path(name: &str) -> PathBuf {
    user_ffmpeg_dir().join(name)
}

/// Used by the NSIS installer and `cutdown.exe --install-dependencies`.
pub fn run_headless_install() -> Result<(), String> {
    if crate::ffmpeg::ffmpeg_is_available() {
        log_install("ffmpeg already available; skipping download.")?;
        return Ok(());
    }

    log_install("Starting ffmpeg install from installer…")?;
    let result = run_install(None);
    match &result {
        Ok(()) => log_install("ffmpeg install finished successfully.")?,
        Err(err) => log_install(&format!("ffmpeg install failed: {err}"))?,
    }
    result
}

#[tauri::command]
pub async fn install_ffmpeg(app: AppHandle) -> Result<crate::ffmpeg::FfmpegCheckResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        run_install(Some(&app))?;
        Ok(crate::ffmpeg::check_ffmpeg())
    })
    .await
    .map_err(|err| format!("ffmpeg install worker failed: {err}"))?
}

pub fn run_install(app: Option<&AppHandle>) -> Result<(), String> {
    #[cfg(not(windows))]
    {
        let _ = app;
        return Err(
            "Automatic ffmpeg install is only supported on Windows. Install ffmpeg and add it to PATH."
                .to_string(),
        );
    }

    #[cfg(windows)]
    {
        install_ffmpeg_windows(app)
    }
}

#[cfg(windows)]
fn install_ffmpeg_windows(app: Option<&AppHandle>) -> Result<(), String> {
    emit(
        app,
        "download",
        "Downloading latest ffmpeg essentials build (~80 MB)…",
        Some(0.0),
    );

    let install_dir = user_ffmpeg_dir();
    fs::create_dir_all(&install_dir)
        .map_err(|err| format!("Failed to create ffmpeg install folder: {err}"))?;

    let temp_root = std::env::temp_dir().join(format!("cutdown-ffmpeg-{}", temp_stamp()));
    fs::create_dir_all(&temp_root).map_err(|err| format!("Failed to create temp folder: {err}"))?;
    let zip_path = temp_root.join("ffmpeg-package.zip");
    let extract_dir = temp_root.join("extract");

    let install_result: Result<(), String> = (|| {
        download_package(app, FFMPEG_PACKAGE_URL, &zip_path)?;
        emit(app, "extract", "Extracting ffmpeg and ffprobe…", Some(92.0));
        extract_binaries(&zip_path, &extract_dir, &install_dir)?;
        verify_install(&install_dir)?;
        write_install_note(&install_dir)?;
        Ok(())
    })();

    let _ = fs::remove_dir_all(&temp_root);

    install_result?;

    emit(
        app,
        "complete",
        "Installed latest ffmpeg essentials (Windows x64).",
        Some(100.0),
    );
    Ok(())
}

#[cfg(windows)]
fn download_package(app: Option<&AppHandle>, url: &str, destination: &Path) -> Result<(), String> {
    let client = reqwest::blocking::Client::builder()
        .user_agent("Cutdown/0.2.1")
        .build()
        .map_err(|err| format!("Failed to create HTTP client: {err}"))?;

    let response = client
        .get(url)
        .send()
        .map_err(|err| format!("Failed to download ffmpeg package: {err}"))?;

    if !response.status().is_success() {
        return Err(format!(
            "Failed to download ffmpeg package (HTTP {}).",
            response.status()
        ));
    }

    let total = response.content_length().unwrap_or(0);
    let mut source = response;
    let mut file = File::create(destination)
        .map_err(|err| format!("Failed to create download file: {err}"))?;
    let mut downloaded = 0u64;
    let mut buffer = [0u8; 64 * 1024];

    loop {
        let read = source
            .read(&mut buffer)
            .map_err(|err| format!("Failed while downloading ffmpeg: {err}"))?;
        if read == 0 {
            break;
        }

        file.write_all(&buffer[..read])
            .map_err(|err| format!("Failed to write ffmpeg download: {err}"))?;
        downloaded += read as u64;

        if total > 0 {
            let percent = (downloaded as f64 / total as f64) * 88.0;
            emit(
                app,
                "download",
                &format!(
                    "Downloading ffmpeg… {} / {} MB",
                    downloaded / 1_000_000,
                    total / 1_000_000
                ),
                Some(percent),
            );
        }
    }

    Ok(())
}

#[cfg(windows)]
fn extract_binaries(zip_path: &Path, extract_dir: &Path, install_dir: &Path) -> Result<(), String> {
    fs::create_dir_all(extract_dir)
        .map_err(|err| format!("Failed to create extract folder: {err}"))?;

    let archive_file =
        File::open(zip_path).map_err(|err| format!("Failed to open downloaded package: {err}"))?;
    let mut archive = zip::ZipArchive::new(archive_file)
        .map_err(|err| format!("Invalid ffmpeg package: {err}"))?;

    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|err| format!("Failed to read package entry: {err}"))?;
        let Some(file_name) = entry.enclosed_name().map(|path| path.to_path_buf()) else {
            continue;
        };

        let normalized = file_name.to_string_lossy().replace('\\', "/");
        let target_name = if normalized.ends_with("/bin/ffmpeg.exe") {
            Some("ffmpeg.exe")
        } else if normalized.ends_with("/bin/ffprobe.exe") {
            Some("ffprobe.exe")
        } else {
            None
        };

        let Some(target_name) = target_name else {
            continue;
        };

        let output_path = install_dir.join(target_name);
        let mut output = File::create(&output_path)
            .map_err(|err| format!("Failed to create {target_name}: {err}"))?;
        copy(&mut entry, &mut output)
            .map_err(|err| format!("Failed to extract {target_name}: {err}"))?;
    }

    if !install_dir.join("ffmpeg.exe").exists() || !install_dir.join("ffprobe.exe").exists() {
        return Err("Downloaded package did not contain ffmpeg.exe and ffprobe.exe.".to_string());
    }

    Ok(())
}

#[cfg(windows)]
fn verify_install(install_dir: &Path) -> Result<(), String> {
    for name in ["ffmpeg.exe", "ffprobe.exe"] {
        let path = install_dir.join(name);
        let output = command(&path)
            .arg("-version")
            .output()
            .map_err(|err| format!("Failed to run {name}: {err}"))?;

        if !output.status.success() {
            return Err(format!(
                "{name} failed verification: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }
    }

    Ok(())
}

#[cfg(windows)]
fn write_install_note(install_dir: &Path) -> Result<(), String> {
    let ffmpeg_version = command(install_dir.join("ffmpeg.exe"))
        .arg("-version")
        .output()
        .ok()
        .map(|output| {
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .next()
                .unwrap_or("")
                .to_string()
        })
        .unwrap_or_else(|| "unknown".to_string());

    let note = format!(
        "Cutdown installed ffmpeg essentials (latest release build)\nSource: {FFMPEG_PACKAGE_URL}\n{ffmpeg_version}\n",
    );
    fs::write(install_dir.join("INSTALL.txt"), note)
        .map_err(|err| format!("Failed to write install note: {err}"))?;
    Ok(())
}

fn emit(app: Option<&AppHandle>, stage: &str, message: &str, percent: Option<f64>) {
    if let Some(app) = app {
        let _ = app.emit(
            "ffmpeg_install_progress",
            FfmpegInstallProgress {
                stage: stage.to_string(),
                message: message.to_string(),
                percent,
            },
        );
    }
}

fn log_install(message: &str) -> Result<(), String> {
    let path = install_log_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("Failed to create install log folder: {err}"))?;
    }

    let line = format!(
        "[{}] {message}\n",
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
    );

    use std::io::Write;
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|err| format!("Failed to open install log: {err}"))?;
    file.write_all(line.as_bytes())
        .map_err(|err| format!("Failed to write install log: {err}"))?;
    Ok(())
}

fn temp_stamp() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}

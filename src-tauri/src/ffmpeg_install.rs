use crate::command_util::command;
use serde::Serialize;
use std::fs::{self, File};
use std::io::{copy, Read, Write};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter};

/// Pinned essentials build (Windows x64). ~80 MB download.
const FFMPEG_PACKAGE_URL: &str =
    "https://www.gyan.dev/ffmpeg/builds/packages/ffmpeg-7.1.1-essentials_build.zip";
const FFMPEG_PACKAGE_LABEL: &str = "ffmpeg 7.1.1 essentials (Windows x64)";

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

fn dirs_fallback() -> PathBuf {
    std::env::temp_dir().join("Cutdown")
}

pub fn user_ffmpeg_path(name: &str) -> PathBuf {
    user_ffmpeg_dir().join(name)
}

#[tauri::command]
pub async fn install_ffmpeg(app: AppHandle) -> Result<crate::ffmpeg::FfmpegCheckResult, String> {
    tauri::async_runtime::spawn_blocking(move || install_ffmpeg_blocking(app))
        .await
        .map_err(|err| format!("ffmpeg install worker failed: {err}"))?
}

fn install_ffmpeg_blocking(app: AppHandle) -> Result<crate::ffmpeg::FfmpegCheckResult, String> {
    run_install(&app)?;
    Ok(crate::ffmpeg::check_ffmpeg())
}

pub fn run_install(app: &AppHandle) -> Result<(), String> {
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
fn install_ffmpeg_windows(app: &AppHandle) -> Result<(), String> {
    emit(
        app,
        "download",
        "Downloading ffmpeg (~80 MB). This only happens once.",
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
        &format!("Installed {FFMPEG_PACKAGE_LABEL}."),
        Some(100.0),
    );
    Ok(())
}

#[cfg(windows)]
fn download_package(app: &AppHandle, url: &str, destination: &Path) -> Result<(), String> {
    let client = reqwest::blocking::Client::builder()
        .user_agent("Cutdown/0.2.0")
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
    let note = format!("Cutdown installed {FFMPEG_PACKAGE_LABEL}\nSource: {FFMPEG_PACKAGE_URL}\n",);
    fs::write(install_dir.join("INSTALL.txt"), note)
        .map_err(|err| format!("Failed to write install note: {err}"))?;
    Ok(())
}

fn emit(app: &AppHandle, stage: &str, message: &str, percent: Option<f64>) {
    let _ = app.emit(
        "ffmpeg_install_progress",
        FfmpegInstallProgress {
            stage: stage.to_string(),
            message: message.to_string(),
            percent,
        },
    );
}

fn temp_stamp() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}

use crate::settings::load_settings;
use notify::Watcher;
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;

const VIDEO_EXTENSIONS: &[&str] = &["mp4", "mkv", "mov", "webm", "ts", "avi", "flv"];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchFolderClip {
    pub path: String,
}

struct WatchFolderState {
    watcher: Option<notify::RecommendedWatcher>,
    recent: HashMap<String, Instant>,
}

impl WatchFolderState {
    fn new() -> Self {
        Self {
            watcher: None,
            recent: HashMap::new(),
        }
    }
}

pub fn manage_state(app: &tauri::App) -> Result<(), String> {
    app.manage(Mutex::new(WatchFolderState::new()));
    // Never block app startup on folder I/O — at login a watch path on H: or a network
    // drive can stall `is_dir()` long enough to freeze the window (white screen).
    spawn_restart_watcher(app.handle().clone());
    Ok(())
}

/// Starts or restarts the watcher on a background thread with retries (login / drive spin-up).
pub fn spawn_restart_watcher(app: AppHandle) {
    std::thread::spawn(move || {
        const MAX_ATTEMPTS: u32 = 12;
        const RETRY_DELAY: Duration = Duration::from_secs(5);

        for attempt in 0..MAX_ATTEMPTS {
            if attempt > 0 {
                std::thread::sleep(RETRY_DELAY);
            }
            match restart_watcher(app.clone()) {
                Ok(()) => return,
                Err(err) => eprintln!(
                    "watch folder watcher (attempt {}/{MAX_ATTEMPTS}): {err}",
                    attempt + 1
                ),
            }
        }
    });
}

pub fn restart_watcher(app: AppHandle) -> Result<(), String> {
    let settings = load_settings();
    let state = app.state::<Mutex<WatchFolderState>>();

    let mut guard = state
        .lock()
        .map_err(|_| "Watch folder state lock poisoned.".to_string())?;

    guard.watcher = None;

    if !settings.watch_folder_enabled {
        return Ok(());
    }

    let folder = settings
        .watch_folder
        .as_ref()
        .ok_or_else(|| "Watch folder is not configured.".to_string())?;
    let folder_path = PathBuf::from(folder);

    watch_folder_accessible(&folder_path)?;

    let app_handle = app.clone();
    let watcher = notify::recommended_watcher(move |result| {
        if let Ok(event) = result {
            handle_notify_event(&app_handle, event);
        }
    })
    .map_err(|err| format!("Failed to start folder watcher: {err}"))?;

    let mut watcher = watcher;
    watcher
        .watch(&folder_path, notify::RecursiveMode::NonRecursive)
        .map_err(|err| format!("Failed to watch folder: {err}"))?;

    guard.watcher = Some(watcher);
    Ok(())
}

fn watch_folder_accessible(folder_path: &Path) -> Result<(), String> {
    if !path_root_is_available(folder_path) {
        return Err(format!(
            "Watch folder drive is not available yet: {}",
            folder_path.display()
        ));
    }

    if !folder_path.is_dir() {
        return Err(format!(
            "Watch folder does not exist: {}",
            folder_path.display()
        ));
    }

    Ok(())
}

/// Returns false when a Windows drive letter is not mounted yet (avoids blocking I/O).
fn path_root_is_available(path: &Path) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Storage::FileSystem::GetDriveTypeW;

        let Some(text) = path.to_str() else {
            return true;
        };

        let prefix = if text.len() >= 2 && text.as_bytes()[1] == b':' {
            format!("{}\\", &text[..2])
        } else {
            return true;
        };

        let wide: Vec<u16> = std::ffi::OsStr::new(&prefix)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let drive_type = unsafe { GetDriveTypeW(wide.as_ptr()) };
        // DRIVE_UNKNOWN (0) and DRIVE_NO_ROOT_DIR (1) — drive letter not ready at login.
        return drive_type > 1;
    }

    #[cfg(not(windows))]
    {
        let _ = path;
        true
    }
}

fn handle_notify_event(app: &AppHandle, event: notify::Event) {
    let relevant = matches!(
        event.kind,
        notify::EventKind::Create(_) | notify::EventKind::Modify(_)
    );

    if !relevant {
        return;
    }

    for path in event.paths {
        if !is_candidate_video(&path) {
            continue;
        }

        let app_handle = app.clone();
        std::thread::spawn(move || {
            if let Some(stable_path) = wait_for_stable_file(&path) {
                announce_clip(app_handle, stable_path);
            }
        });
    }
}

fn announce_clip(app: AppHandle, path: PathBuf) {
    let path_key = path.to_string_lossy().to_string();

    {
        let state = app.state::<Mutex<WatchFolderState>>();
        let Ok(mut guard) = state.lock() else {
            return;
        };

        let now = Instant::now();
        if let Some(last) = guard.recent.get(&path_key) {
            if now.duration_since(*last) < Duration::from_secs(2) {
                return;
            }
        }

        guard.recent.insert(path_key.clone(), now);
        guard
            .recent
            .retain(|_, seen| now.duration_since(*seen) < Duration::from_secs(30));
    }

    let payload = WatchFolderClip {
        path: path_key.clone(),
    };

    let _ = app.emit("watch_folder_clip", payload);

    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("New clip");

    let _ = app
        .notification()
        .builder()
        .title("Cutdown")
        .body(format!("New clip ready: {file_name}"))
        .show();

    if let Err(err) = crate::show_editor_window(&app) {
        eprintln!("failed to show editor for watch-folder clip: {err}");
    }
}

fn is_candidate_video(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }

    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_lowercase();

    if file_name.ends_with(".tmp") || file_name.ends_with(".part") {
        return false;
    }

    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| VIDEO_EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

fn wait_for_stable_file(path: &Path) -> Option<PathBuf> {
    std::thread::sleep(Duration::from_millis(500));

    let mut last_size = None;

    for _ in 0..8 {
        if !path.exists() || !path.is_file() {
            std::thread::sleep(Duration::from_millis(250));
            continue;
        }

        let size = std::fs::metadata(path).ok()?.len();
        if let Some(previous) = last_size {
            if previous == size && size > 0 {
                return Some(path.to_path_buf());
            }
        }

        last_size = Some(size);
        std::thread::sleep(Duration::from_millis(250));
    }

    None
}

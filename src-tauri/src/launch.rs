use std::path::PathBuf;
use std::sync::Mutex;

pub struct LaunchState {
    pub pending_file: Mutex<Option<String>>,
}

impl LaunchState {
    pub fn new() -> Self {
        Self {
            pending_file: Mutex::new(parse_launch_file()),
        }
    }
}

fn parse_launch_file() -> Option<String> {
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg.starts_with('-') {
            continue;
        }

        let path = PathBuf::from(&arg);
        if path.is_file() && is_video_extension(path.extension()?.to_str()?) {
            return Some(path.to_string_lossy().to_string());
        }
    }

    None
}

fn is_video_extension(ext: &str) -> bool {
    matches!(
        ext.to_ascii_lowercase().as_str(),
        "mp4" | "mkv" | "mov" | "webm" | "ts" | "avi" | "flv"
    )
}

#[tauri::command]
pub fn get_launch_path(state: tauri::State<'_, LaunchState>) -> Option<String> {
    state
        .pending_file
        .lock()
        .ok()
        .and_then(|mut value| value.take())
}

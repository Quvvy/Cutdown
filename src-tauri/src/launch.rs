use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

const VIDEO_EXTENSIONS: &[&str] = &["mp4", "mkv", "mov", "webm", "ts", "avi", "flv", "m4v"];
const PROJECT_EXTENSION: &str = "cutdown";
pub const FROM_STARTUP_FLAG: &str = "--from-startup";

pub struct LaunchState {
    pending_files: Mutex<VecDeque<String>>,
}

impl LaunchState {
    pub fn new() -> Self {
        let mut pending_files = VecDeque::new();
        if let Some(path) = parse_open_path_from_args(std::env::args().skip(1)) {
            pending_files.push_back(path);
        }
        Self {
            pending_files: Mutex::new(pending_files),
        }
    }

    pub fn enqueue(&self, path: String) {
        let trimmed = path.trim();
        if trimmed.is_empty() {
            return;
        }

        if let Ok(mut queue) = self.pending_files.lock() {
            if queue.back().map(String::as_str) != Some(trimmed) {
                queue.push_back(trimmed.to_string());
            }
        }
    }

    pub fn take_next(&self) -> Option<String> {
        self.pending_files.lock().ok().and_then(|mut queue| queue.pop_front())
    }

    pub fn has_pending(&self) -> bool {
        self.pending_files
            .lock()
            .ok()
            .is_some_and(|queue| !queue.is_empty())
    }
}

pub fn launched_from_startup() -> bool {
    std::env::args().any(|arg| arg == FROM_STARTUP_FLAG)
}

pub fn parse_open_path_from_args<I, S>(args: I) -> Option<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    args.into_iter()
        .map(|arg| arg.as_ref().trim().to_string())
        .filter(|arg| !arg.is_empty())
        .find_map(|arg| normalize_open_candidate(&arg))
}

fn normalize_open_candidate(arg: &str) -> Option<String> {
    let trimmed = strip_wrapping_quotes(arg);
    if trimmed.is_empty() || trimmed.starts_with('-') {
        return None;
    }

    let without_url = decode_file_url(&trimmed).unwrap_or_else(|| trimmed.to_string());
    let path = strip_extended_path_prefix(&without_url);

    if looks_like_app_binary(&path) {
        return None;
    }

    if !is_supported_user_path(&path) {
        return None;
    }

    Some(path)
}

fn strip_wrapping_quotes(value: &str) -> &str {
    let trimmed = value.trim();
    if trimmed.len() >= 2 {
        let bytes = trimmed.as_bytes();
        if (bytes[0] == b'"' && bytes[trimmed.len() - 1] == b'"')
            || (bytes[0] == b'\'' && bytes[trimmed.len() - 1] == b'\'')
        {
            return &trimmed[1..trimmed.len() - 1];
        }
    }
    trimmed
}

fn decode_file_url(value: &str) -> Option<String> {
    let lower = value.to_ascii_lowercase();
    if !lower.starts_with("file:") {
        return None;
    }

    let rest = value
        .get(5..)
        .unwrap_or_default()
        .trim_start_matches('/')
        .trim_start_matches('\\');

    let decoded = percent_decode(rest);
    if decoded.len() >= 2 && decoded.as_bytes()[1] == b':' {
        return Some(decoded.replace('/', "\\"));
    }

    if decoded.starts_with('\\') || decoded.starts_with('/') {
        return Some(decoded);
    }

    Some(format!("/{decoded}"))
}

fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            if let Ok(byte) = u8::from_str_radix(
                std::str::from_utf8(&bytes[index + 1..index + 3]).unwrap_or(""),
                16,
            ) {
                output.push(byte);
                index += 3;
                continue;
            }
        }
        output.push(bytes[index]);
        index += 1;
    }
    String::from_utf8_lossy(&output).into_owned()
}

fn strip_extended_path_prefix(path: &str) -> String {
    let trimmed = path.trim();
    if let Some(rest) = trimmed.strip_prefix(r"\\?\") {
        rest.to_string()
    } else {
        trimmed.to_string()
    }
}

fn looks_like_app_binary(path: &str) -> bool {
    Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.eq_ignore_ascii_case("cutdown.exe") || name.eq_ignore_ascii_case("cutdown"))
        .unwrap_or(false)
}

pub fn is_supported_user_path(path: &str) -> bool {
    extension_of(path)
        .map(|ext| is_video_extension(&ext) || ext == PROJECT_EXTENSION)
        .unwrap_or(false)
}

pub fn is_project_path(path: &str) -> bool {
    extension_of(path).is_some_and(|ext| ext == PROJECT_EXTENSION)
}

fn is_video_extension(ext: &str) -> bool {
    VIDEO_EXTENSIONS.iter().any(|candidate| *candidate == ext)
}

fn extension_of(path: &str) -> Option<String> {
    PathBuf::from(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.trim().to_ascii_lowercase())
        .filter(|ext| !ext.is_empty())
}

#[tauri::command]
pub fn get_launch_path(state: tauri::State<'_, LaunchState>) -> Option<String> {
    state.take_next()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_quoted_windows_video_path() {
        assert_eq!(
            parse_open_path_from_args([r#""C:\Clips\match.mp4""#]),
            Some(r"C:\Clips\match.mp4".to_string())
        );
    }

    #[test]
    fn parses_project_and_skips_flags() {
        let parsed = parse_open_path_from_args([
            FROM_STARTUP_FLAG,
            "--install-dependencies",
            r"D:\Projects\edit.cutdown",
        ]);
        assert_eq!(parsed, Some(r"D:\Projects\edit.cutdown".to_string()));
        assert!(is_project_path(r"D:\Projects\edit.cutdown"));
    }

    #[test]
    fn ignores_the_app_binary_and_unknown_extensions() {
        assert_eq!(
            parse_open_path_from_args([r"C:\Apps\Cutdown\Cutdown.exe"]),
            None
        );
        assert_eq!(parse_open_path_from_args(["C:\\notes.txt"]), None);
    }

    #[test]
    fn parses_file_urls() {
        assert_eq!(
            parse_open_path_from_args(["file:///C:/Clips/highlight%20reel.mkv"]),
            Some(r"C:\Clips\highlight reel.mkv".to_string())
        );
    }

    #[test]
    fn queue_dedupes_consecutive_paths() {
        let state = LaunchState {
            pending_files: Mutex::new(VecDeque::new()),
        };
        state.enqueue("C:\\a.mp4".into());
        state.enqueue("C:\\a.mp4".into());
        state.enqueue("C:\\b.mp4".into());
        assert_eq!(state.take_next(), Some("C:\\a.mp4".into()));
        assert_eq!(state.take_next(), Some("C:\\b.mp4".into()));
        assert_eq!(state.take_next(), None);
    }
}

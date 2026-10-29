use std::path::PathBuf;

/// Settings, history and other small per-user files.
/// Windows: `%APPDATA%\Cutdown`. Linux: `$XDG_CONFIG_HOME/cutdown` (usually `~/.config/cutdown`).
pub fn config_dir() -> Result<PathBuf, String> {
    #[cfg(windows)]
    {
        let base =
            std::env::var("APPDATA").map_err(|err| format!("APPDATA is unavailable: {err}"))?;
        Ok(PathBuf::from(base).join("Cutdown"))
    }

    #[cfg(not(windows))]
    {
        Ok(xdg_config_home()?.join("cutdown"))
    }
}

/// Downloaded tools and logs.
/// Windows: `%LOCALAPPDATA%\Cutdown`. Linux: `$XDG_DATA_HOME/cutdown` (usually `~/.local/share/cutdown`).
pub fn data_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        std::env::var_os("LOCALAPPDATA").map(|root| PathBuf::from(root).join("Cutdown"))
    }

    #[cfg(not(windows))]
    {
        xdg_dir("XDG_DATA_HOME", &[".local", "share"])
            .ok()
            .map(|root| root.join("cutdown"))
    }
}

#[cfg(not(windows))]
pub fn xdg_config_home() -> Result<PathBuf, String> {
    xdg_dir("XDG_CONFIG_HOME", &[".config"])
}

#[cfg(not(windows))]
fn xdg_dir(var: &str, home_fallback: &[&str]) -> Result<PathBuf, String> {
    if let Some(value) = std::env::var_os(var).filter(|value| !value.is_empty()) {
        let path = PathBuf::from(value);
        // The XDG spec says relative values are invalid and must be ignored.
        if path.is_absolute() {
            return Ok(path);
        }
    }

    let home = std::env::var_os("HOME")
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("Neither {var} nor HOME is set"))?;
    Ok(home_fallback
        .iter()
        .fold(PathBuf::from(home), |path, part| path.join(part)))
}

/// File names for the ffmpeg tools as they appear on disk.
pub const FFMPEG_BINARY: &str = if cfg!(windows) { "ffmpeg.exe" } else { "ffmpeg" };
pub const FFPROBE_BINARY: &str = if cfg!(windows) { "ffprobe.exe" } else { "ffprobe" };

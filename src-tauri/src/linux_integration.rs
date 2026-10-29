use crate::launch::FROM_STARTUP_FLAG;
use std::path::{Path, PathBuf};

/// XDG autostart entry, honoured by KDE Plasma, GNOME, Xfce, Cinnamon and friends.
const AUTOSTART_FILE_NAME: &str = "cutdown.desktop";

#[tauri::command]
pub fn set_run_at_startup(enabled: bool) -> Result<(), String> {
    if enabled {
        let exe = startup_exe()?;
        write_autostart_entry(&exe)
    } else {
        delete_autostart_entry()
    }
}

/// Keeps the autostart entry aligned with the saved preference on every launch, and
/// re-points it if the app moved (new AppImage location, .deb reinstall).
pub fn sync_run_at_startup(enabled: bool) {
    if enabled {
        if let Err(err) = set_run_at_startup(true) {
            eprintln!("failed to enable run-at-startup: {err}");
            if std::env::current_exe()
                .ok()
                .as_deref()
                .is_some_and(is_build_tree_exe)
            {
                let _ = delete_autostart_entry();
            }
        }
        return;
    }

    if let Err(err) = set_run_at_startup(false) {
        eprintln!("failed to disable run-at-startup: {err}");
    }
}

fn autostart_path() -> Result<PathBuf, String> {
    Ok(crate::app_paths::xdg_config_home()?
        .join("autostart")
        .join(AUTOSTART_FILE_NAME))
}

fn startup_exe() -> Result<PathBuf, String> {
    // An AppImage runs from a temporary mount; $APPIMAGE is the file the user actually has.
    if let Some(appimage) = std::env::var_os("APPIMAGE").filter(|value| !value.is_empty()) {
        return Ok(PathBuf::from(appimage));
    }

    let current =
        std::env::current_exe().map_err(|err| format!("Failed to resolve app path: {err}"))?;
    if is_build_tree_exe(&current) {
        return Err(
            "Run at startup requires the installed Cutdown app. Install the .deb or use the AppImage, then enable this setting from there (not npm run tauri dev).".to_string(),
        );
    }

    Ok(current)
}

fn write_autostart_entry(exe: &Path) -> Result<(), String> {
    let path = autostart_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| format!("Failed to create autostart folder: {err}"))?;
    }

    std::fs::write(&path, autostart_entry(exe))
        .map_err(|err| format!("Failed to write {}: {err}", path.display()))
}

fn delete_autostart_entry() -> Result<(), String> {
    let path = autostart_path()?;
    match std::fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(format!(
            "Failed to disable run at startup ({}): {err}",
            path.display()
        )),
    }
}

fn autostart_entry(exe: &Path) -> String {
    format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name=Cutdown\n\
         Comment=Start Cutdown in the tray\n\
         Exec={} {FROM_STARTUP_FLAG}\n\
         Icon=cutdown\n\
         Terminal=false\n\
         X-GNOME-Autostart-enabled=true\n",
        quote_exec_arg(&exe.to_string_lossy())
    )
}

/// Quotes one Exec= argument per the Desktop Entry spec. Inside quotes, `"`, `` ` ``, `$`
/// and `\` are backslash-escaped, and the backslash itself is escaped again because Exec
/// is a string value.
fn quote_exec_arg(arg: &str) -> String {
    let mut quoted = String::with_capacity(arg.len() + 2);
    quoted.push('"');
    for ch in arg.chars() {
        match ch {
            '"' | '`' | '$' => {
                quoted.push_str("\\\\");
                quoted.push(ch);
            }
            '\\' => quoted.push_str("\\\\\\\\"),
            '%' => quoted.push_str("%%"),
            _ => quoted.push(ch),
        }
    }
    quoted.push('"');
    quoted
}

fn is_build_tree_exe(path: &Path) -> bool {
    let normalized = path.to_string_lossy();
    normalized.contains("/target/debug/") || normalized.contains("/target/release/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_dev_build_exe() {
        assert!(is_build_tree_exe(Path::new(
            "/home/me/Cutdown/target/debug/cutdown"
        )));
        assert!(is_build_tree_exe(Path::new(
            "/home/me/Cutdown/target/release/cutdown"
        )));
        assert!(!is_build_tree_exe(Path::new("/usr/bin/cutdown")));
    }

    #[test]
    fn autostart_entry_quotes_exec() {
        let entry = autostart_entry(Path::new("/home/me/Apps/Cutdown 0.5.AppImage"));
        assert!(entry.contains("Exec=\"/home/me/Apps/Cutdown 0.5.AppImage\" --from-startup\n"));
    }

    #[test]
    fn exec_quoting_escapes_reserved_characters() {
        assert_eq!(quote_exec_arg("/a$b"), "\"/a\\\\$b\"");
        assert_eq!(quote_exec_arg("/100%"), "\"/100%%\"");
    }
}

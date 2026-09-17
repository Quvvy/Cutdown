use crate::command_util::command;
use crate::launch::FROM_STARTUP_FLAG;
use std::path::{Path, PathBuf};

const RUN_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
const RUN_VALUE: &str = "Cutdown";
const UNINSTALL_KEY: &str =
    r"HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\Cutdown";
const UNINSTALL_KEY_MACHINE: &str =
    r"HKLM\Software\Microsoft\Windows\CurrentVersion\Uninstall\Cutdown";
const MANUFACTURER_PRODUCT_KEY: &str = r"HKCU\Software\Cutdown\Cutdown";
const MAIN_BINARY_NAME: &str = "Cutdown.exe";

#[tauri::command]
pub fn set_run_at_startup(enabled: bool) -> Result<(), String> {
    if enabled {
        let exe = startup_registry_exe()?;
        write_startup_entry(&exe)
    } else {
        delete_startup_entry()
    }
}

/// Keeps the HKCU Run key aligned with the saved preference on every launch.
/// Disabled settings delete leftover installer/dev entries so the app cannot keep
/// starting with Windows after the user turned the option off.
pub fn sync_run_at_startup(enabled: bool) {
    if enabled {
        if let Err(err) = set_run_at_startup(true) {
            eprintln!("failed to enable run-at-startup: {err}");
            if std::env::current_exe()
                .ok()
                .as_deref()
                .is_some_and(is_build_tree_exe)
            {
                let _ = delete_startup_entry();
            }
        }
        return;
    }

    if let Err(err) = set_run_at_startup(false) {
        eprintln!("failed to disable run-at-startup: {err}");
    }
}

fn startup_registry_exe() -> Result<PathBuf, String> {
    if let Some(installed) = resolve_installed_exe() {
        return Ok(installed);
    }

    let current =
        std::env::current_exe().map_err(|err| format!("Failed to resolve app path: {err}"))?;
    if is_build_tree_exe(&current) {
        return Err(
            "Run at startup requires the installed Cutdown app. Install Cutdown, then enable this setting from the installed app (not npm run tauri dev).".to_string(),
        );
    }

    Ok(current)
}

fn resolve_installed_exe() -> Option<PathBuf> {
    for candidate in candidate_install_paths() {
        if candidate.is_file() {
            return Some(candidate);
        }
    }

    for key in [
        UNINSTALL_KEY,
        UNINSTALL_KEY_MACHINE,
        MANUFACTURER_PRODUCT_KEY,
    ] {
        if let Some(dir) = read_registry_install_dir(key) {
            let exe = dir.join(MAIN_BINARY_NAME);
            if exe.is_file() {
                return Some(exe);
            }
        }
    }

    None
}

fn candidate_install_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
        let root = PathBuf::from(local_app_data);
        paths.push(root.join("Cutdown").join(MAIN_BINARY_NAME));
        paths.push(root.join("Programs").join("Cutdown").join(MAIN_BINARY_NAME));
    }

    if let Ok(program_files) = std::env::var("ProgramFiles") {
        paths.push(
            PathBuf::from(program_files)
                .join("Cutdown")
                .join(MAIN_BINARY_NAME),
        );
    }

    if let Ok(program_files_x86) = std::env::var("ProgramFiles(x86)") {
        paths.push(
            PathBuf::from(program_files_x86)
                .join("Cutdown")
                .join(MAIN_BINARY_NAME),
        );
    }

    paths
}

fn read_registry_install_dir(key: &str) -> Option<PathBuf> {
    let output = if key.ends_with("Uninstall\\Cutdown") {
        command("reg")
            .args(["query", key, "/v", "InstallLocation"])
            .output()
            .ok()?
    } else {
        command("reg").args(["query", key, "/ve"]).output().ok()?
    };

    if !output.status.success() {
        return None;
    }

    let text = String::from_utf8_lossy(&output.stdout);
    parse_reg_value(&text).map(PathBuf::from)
}

fn read_startup_registry_exe() -> Option<PathBuf> {
    let output = command("reg")
        .args(["query", RUN_KEY, "/v", RUN_VALUE])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let text = String::from_utf8_lossy(&output.stdout);
    parse_startup_exe(&parse_reg_value(&text)?).map(PathBuf::from)
}

/// Extracts the executable from a Run-key command that may include quotes and `--from-startup`.
pub fn parse_startup_exe(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    if trimmed.starts_with('"') {
        let rest = &trimmed[1..];
        let end = rest.find('"')?;
        let exe = rest[..end].replace("\"\"", "\"");
        return if exe.is_empty() { None } else { Some(exe) };
    }

    let exe = trimmed
        .split_once(" --")
        .map(|(head, _)| head)
        .unwrap_or(trimmed)
        .trim();
    if exe.is_empty() {
        None
    } else {
        Some(exe.to_string())
    }
}

fn parse_reg_value(text: &str) -> Option<String> {
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("HKEY_") {
            continue;
        }

        let mut parts = trimmed.split_whitespace();
        let _name = parts.next()?;
        let kind = parts.next()?;
        if kind != "REG_SZ" && kind != "REG_EXPAND_SZ" {
            continue;
        }

        let value = parts.collect::<Vec<_>>().join(" ");
        return Some(unquote_reg_value(&value));
    }

    None
}

fn unquote_reg_value(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.len() >= 2 && trimmed.starts_with('"') && trimmed.ends_with('"') {
        trimmed[1..trimmed.len() - 1].replace("\"\"", "\"")
    } else {
        trimmed.to_string()
    }
}

fn write_startup_entry(exe: &Path) -> Result<(), String> {
    let command = format!("\"{}\" {FROM_STARTUP_FLAG}", exe.to_string_lossy());
    run_reg(&[
        "add",
        RUN_KEY,
        "/v",
        RUN_VALUE,
        "/t",
        "REG_SZ",
        "/d",
        &command,
        "/f",
    ])
}

fn delete_startup_entry() -> Result<(), String> {
    let output = command("reg")
        .args(["delete", RUN_KEY, "/v", RUN_VALUE, "/f"])
        .output()
        .map_err(|err| format!("Failed to run reg.exe: {err}"))?;

    if output.status.success() || read_startup_registry_exe().is_none() {
        return Ok(());
    }

    let message = reg_message(&output);
    if registry_value_missing(&message) {
        return Ok(());
    }

    Err(format!(
        "Failed to disable run at startup (reg.exe exit {}): {message}",
        output.status
    ))
}

fn run_reg(args: &[&str]) -> Result<(), String> {
    let output = command("reg")
        .args(args)
        .output()
        .map_err(|err| format!("Failed to run reg.exe: {err}"))?;

    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "reg.exe exited with status {}: {}",
            output.status,
            reg_message(&output)
        ))
    }
}

fn reg_message(output: &std::process::Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    format!("{stdout}{stderr}").trim().to_string()
}

fn registry_value_missing(message: &str) -> bool {
    message.contains("unable to find the specified registry key or value")
        || message.contains("cannot find the file specified")
}

fn is_build_tree_exe(path: &Path) -> bool {
    let normalized = path
        .to_string_lossy()
        .replace('/', "\\")
        .to_ascii_lowercase();

    normalized.contains("\\target\\debug\\")
        || normalized.contains("\\target\\release\\")
        || (normalized.contains("\\src-tauri\\") && normalized.ends_with("\\cutdown.exe"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_build_tree_executables() {
        assert!(is_build_tree_exe(Path::new(
            r"E:\cursor\Cutdown\src-tauri\target\debug\cutdown.exe"
        )));
        assert!(is_build_tree_exe(Path::new(
            r"E:\cursor\Cutdown\src-tauri\target\release\cutdown.exe"
        )));
        assert!(!is_build_tree_exe(Path::new(
            r"C:\Users\me\AppData\Local\Cutdown\Cutdown.exe"
        )));
    }

    #[test]
    fn parses_reg_sz_values() {
        let sample = r#"
HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Run
    Cutdown    REG_SZ    "C:\Users\me\AppData\Local\Cutdown\Cutdown.exe"
"#;
        assert_eq!(
            parse_reg_value(sample),
            Some(r"C:\Users\me\AppData\Local\Cutdown\Cutdown.exe".to_string())
        );
    }

    #[test]
    fn unquotes_registry_strings() {
        assert_eq!(
            unquote_reg_value(r#""C:\Apps\Cutdown\Cutdown.exe""#),
            r"C:\Apps\Cutdown\Cutdown.exe"
        );
    }

    #[test]
    fn parses_startup_commands_with_and_without_flags() {
        assert_eq!(
            parse_startup_exe(r#""C:\Users\me\AppData\Local\Cutdown\Cutdown.exe" --from-startup"#),
            Some(r"C:\Users\me\AppData\Local\Cutdown\Cutdown.exe".to_string())
        );
        assert_eq!(
            parse_startup_exe(r"C:\Apps\Cutdown\Cutdown.exe"),
            Some(r"C:\Apps\Cutdown\Cutdown.exe".to_string())
        );
    }

    #[test]
    fn parse_reg_value_then_startup_exe_handles_command_args() {
        let sample = r#"
HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Run
    Cutdown    REG_SZ    "C:\Users\me\AppData\Local\Cutdown\Cutdown.exe" --from-startup
"#;
        let value = parse_reg_value(sample).expect("reg value");
        assert_eq!(
            parse_startup_exe(&value),
            Some(r"C:\Users\me\AppData\Local\Cutdown\Cutdown.exe".to_string())
        );
    }
}

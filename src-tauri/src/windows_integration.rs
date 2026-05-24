use crate::command_util::command;
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

/// Keeps the Run registry entry aligned with the installed app when startup is enabled.
pub fn ensure_run_at_startup_on_launch(enabled: bool) {
    if !enabled {
        return;
    }

    let current_exe = std::env::current_exe().ok();
    let registry_exe = read_startup_registry_exe();

    if let Ok(target) = startup_registry_exe() {
        if registry_exe.as_ref() != Some(&target) {
            if let Err(err) = write_startup_entry(&target) {
                eprintln!("failed to sync run-at-startup registry entry: {err}");
            }
        }
        return;
    }

    if let (Some(current), Some(reg)) = (current_exe.as_deref(), registry_exe.as_deref()) {
        if is_build_tree_exe(current) && paths_refer_to_same_file(current, reg) {
            if let Err(err) = delete_startup_entry() {
                eprintln!("failed to remove dev run-at-startup entry: {err}");
            }
        }
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
    parse_reg_value(&text).map(PathBuf::from)
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
    let quoted = format!("\"{}\"", exe.to_string_lossy());
    run_reg(&[
        "add",
        RUN_KEY,
        "/v",
        RUN_VALUE,
        "/t",
        "REG_SZ",
        "/d",
        &quoted,
        "/f",
    ])
}

fn delete_startup_entry() -> Result<(), String> {
    let output = command("reg")
        .args(["delete", RUN_KEY, "/v", RUN_VALUE, "/f"])
        .output()
        .map_err(|err| format!("Failed to run reg.exe: {err}"))?;

    if output.status.success() {
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

fn paths_refer_to_same_file(left: &Path, right: &Path) -> bool {
    if left == right {
        return true;
    }

    match (left.canonicalize(), right.canonicalize()) {
        (Ok(left), Ok(right)) => left == right,
        _ => false,
    }
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
}

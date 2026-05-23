use crate::command_util::command;

const RUN_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
const RUN_VALUE: &str = "Cutdown";

#[tauri::command]
pub fn set_run_at_startup(enabled: bool) -> Result<(), String> {
    if enabled {
        enable_run_at_startup()
    } else {
        delete_startup_entry()
    }
}

#[cfg(debug_assertions)]
fn enable_run_at_startup() -> Result<(), String> {
    Err(
        "Run at startup is only available in release builds. Install Cutdown from the release installer, then enable this option in Settings.".to_string(),
    )
}

#[cfg(not(debug_assertions))]
fn enable_run_at_startup() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|err| format!("Failed to resolve app path: {err}"))?;
    let quoted = format!("\"{}\"", exe.to_string_lossy());
    run_reg(&[
        "add", RUN_KEY, "/v", RUN_VALUE, "/t", "REG_SZ", "/d", &quoted, "/f",
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

#[cfg(not(debug_assertions))]
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

use std::process::Command;

const RUN_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
const RUN_VALUE: &str = "Cutdown";

#[tauri::command]
pub fn set_run_at_startup(enabled: bool) -> Result<(), String> {
    if enabled {
        let exe = std::env::current_exe().map_err(|err| format!("Failed to resolve app path: {err}"))?;
        let quoted = format!("\"{}\"", exe.to_string_lossy());
        run_reg(&["add", RUN_KEY, "/v", RUN_VALUE, "/t", "REG_SZ", "/d", &quoted, "/f"])
    } else {
        run_reg(&["delete", RUN_KEY, "/v", RUN_VALUE, "/f"])
    }
}

fn run_reg(args: &[&str]) -> Result<(), String> {
    let status = Command::new("reg")
        .args(args)
        .status()
        .map_err(|err| format!("Failed to run reg.exe: {err}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("reg.exe exited with status {status}"))
    }
}

use std::ffi::OsStr;
use std::process::Command;

/// Spawn a child process without flashing a console window on Windows.
pub fn command<S: AsRef<OsStr>>(program: S) -> Command {
    let mut cmd = Command::new(program);
    hide_console_window(&mut cmd);
    cmd
}

fn hide_console_window(cmd: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    #[cfg(not(windows))]
    {
        let _ = cmd;
    }
}

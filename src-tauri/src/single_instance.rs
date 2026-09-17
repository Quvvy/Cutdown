use crate::launch::LaunchState;
use crate::show_editor_window;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

const FOCUS_TOKEN: &str = "*focus*";

pub enum Handshake {
    Forwarded,
    Primary(TcpListener),
}

/// If another Cutdown is already listening, send this process's open-path (if any) and
/// request focus, then the caller should exit. Otherwise bind a local listener.
pub fn handshake() -> Handshake {
    if let Some(port) = read_published_port() {
        if forward_to_existing(port) {
            return Handshake::Forwarded;
        }
    }

    match TcpListener::bind("127.0.0.1:0") {
        Ok(listener) => {
            let _ = listener.set_nonblocking(true);
            if let Ok(addr) = listener.local_addr() {
                let _ = write_published_port(addr.port());
            }
            Handshake::Primary(listener)
        }
        Err(err) => {
            eprintln!("single-instance bind failed, continuing as primary: {err}");
            Handshake::Primary(listener_fallback())
        }
    }
}

fn listener_fallback() -> TcpListener {
    TcpListener::bind("127.0.0.1:0").expect("Cutdown could not bind a local single-instance port")
}

pub fn spawn_listener(app: AppHandle, listener: TcpListener) {
    std::thread::Builder::new()
        .name("cutdown-single-instance".into())
        .spawn(move || {
            loop {
                match listener.accept() {
                    Ok((stream, _)) => handle_client(&app, stream),
                    Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(80));
                    }
                    Err(err) => {
                        eprintln!("single-instance accept failed: {err}");
                        std::thread::sleep(Duration::from_millis(250));
                    }
                }
            }
        })
        .expect("single-instance listener thread");
}

fn handle_client(app: &AppHandle, stream: TcpStream) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    if reader.read_line(&mut line).is_err() {
        return;
    }

    let payload = line.trim();
    if !payload.is_empty() && payload != FOCUS_TOKEN {
        if let Some(path) = crate::launch::parse_open_path_from_args([payload]) {
            app.state::<LaunchState>().enqueue(path);
            let _ = app.emit("second-instance", ());
        }
    }

    if let Err(err) = show_editor_window(app) {
        eprintln!("failed to focus existing Cutdown window: {err}");
    }
}

fn forward_to_existing(port: u16) -> bool {
    let mut stream = match TcpStream::connect_timeout(
        &std::net::SocketAddr::from(([127, 0, 0, 1], port)),
        Duration::from_millis(400),
    ) {
        Ok(stream) => stream,
        Err(_) => return false,
    };

    let _ = stream.set_write_timeout(Some(Duration::from_millis(400)));
    let payload = crate::launch::parse_open_path_from_args(std::env::args().skip(1))
        .unwrap_or_else(|| FOCUS_TOKEN.to_string());
    if writeln!(stream, "{payload}").is_err() {
        return false;
    }
    let _ = stream.flush();
    true
}

fn port_file_path() -> Option<PathBuf> {
    let base = std::env::var_os("APPDATA")
        .or_else(|| std::env::var_os("XDG_RUNTIME_DIR"))
        .or_else(|| std::env::var_os("TMPDIR"))
        .or_else(|| std::env::var_os("TEMP"))
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    Some(base.join("Cutdown").join("instance.port"))
}

fn read_published_port() -> Option<u16> {
    let path = port_file_path()?;
    let raw = std::fs::read_to_string(path).ok()?;
    raw.trim().parse().ok()
}

fn write_published_port(port: u16) -> std::io::Result<()> {
    let path = port_file_path().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::NotFound, "instance port path unavailable")
    })?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, port.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn focus_token_is_not_a_media_path() {
        assert!(crate::launch::parse_open_path_from_args([FOCUS_TOKEN]).is_none());
    }
}

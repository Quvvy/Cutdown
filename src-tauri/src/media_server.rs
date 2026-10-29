//! Serves clips to the preview `<video>` over `http://127.0.0.1` on Linux.
//!
//! WebKitGTK hands media URLs to GStreamer, which can only fetch http(s), blob and file
//! URLs, so the `asset://` protocol that WebView2 plays from on Windows can't feed a
//! `<video>` there. Every URL carries a random per-launch token and only video files are
//! served, so web pages open in a browser can't use the port to read the user's files.

use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::time::Duration;

const MAX_HEADER_BYTES: usize = 16 * 1024;

/// Present only when the server is running (Linux). `None` means "use asset://".
pub struct MediaServerState(pub Option<String>);

#[tauri::command]
pub fn media_base_url(state: tauri::State<'_, MediaServerState>) -> Option<String> {
    state.0.clone()
}

/// Binds a loopback port and serves on a background thread. Returns the base URL that
/// file paths (percent-encoded as one segment) are appended to.
#[cfg_attr(windows, allow(dead_code))]
pub fn start() -> std::io::Result<String> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    let token = random_token();
    let prefix = format!("/{token}/");

    std::thread::Builder::new()
        .name("cutdown-media-server".into())
        .spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else {
                    continue;
                };
                let prefix = prefix.clone();
                let _ = std::thread::Builder::new()
                    .name("cutdown-media-request".into())
                    .spawn(move || {
                        // Players drop connections mid-body on every seek; that's not an error.
                        let _ = handle_connection(stream, &prefix);
                    });
            }
        })?;

    Ok(format!("http://127.0.0.1:{port}/{token}/"))
}

fn random_token() -> String {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};

    // RandomState is seeded from the OS RNG; two instances give 128 unpredictable bits.
    (0..2)
        .map(|_| {
            let mut hasher = RandomState::new().build_hasher();
            hasher.write_u128(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|duration| duration.as_nanos())
                    .unwrap_or(0),
            );
            format!("{:016x}", hasher.finish())
        })
        .collect()
}

#[derive(Debug, PartialEq)]
struct Request {
    method: String,
    target: String,
    range: Option<String>,
}

fn handle_connection(mut stream: TcpStream, prefix: &str) -> std::io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(10)))?;
    let request = match read_request(&stream)? {
        Some(request) => request,
        None => return respond_empty(&mut stream, "400 Bad Request", &[]),
    };

    if request.method != "GET" && request.method != "HEAD" {
        return respond_empty(&mut stream, "405 Method Not Allowed", &[("Allow", "GET, HEAD")]);
    }

    let Some(path) = resolve_path(&request.target, prefix) else {
        return respond_empty(&mut stream, "404 Not Found", &[]);
    };

    let mut file = match File::open(&path) {
        Ok(file) => file,
        Err(_) => return respond_empty(&mut stream, "404 Not Found", &[]),
    };
    let len = file.metadata()?.len();
    let content_type = content_type_for(&path);

    let (status, start, end) = match request.range.as_deref() {
        None => ("200 OK", 0, len.saturating_sub(1)),
        Some(header) => match parse_range(header, len) {
            Some((start, end)) => ("206 Partial Content", start, end),
            None => {
                let unsatisfied = format!("bytes */{len}");
                return respond_empty(
                    &mut stream,
                    "416 Range Not Satisfiable",
                    &[("Content-Range", &unsatisfied)],
                );
            }
        },
    };
    let body_len = if len == 0 { 0 } else { end - start + 1 };

    let mut head = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {body_len}\r\nAccept-Ranges: bytes\r\nCache-Control: no-store\r\nConnection: close\r\n"
    );
    if status.starts_with("206") {
        head.push_str(&format!("Content-Range: bytes {start}-{end}/{len}\r\n"));
    }
    head.push_str("\r\n");
    stream.write_all(head.as_bytes())?;

    if request.method == "GET" && body_len > 0 {
        file.seek(SeekFrom::Start(start))?;
        std::io::copy(&mut file.take(body_len), &mut stream)?;
    }
    stream.flush()
}

fn read_request(stream: &TcpStream) -> std::io::Result<Option<Request>> {
    let mut reader = BufReader::new(stream);
    let mut request_line = String::new();
    reader.read_line(&mut request_line)?;

    let mut parts = request_line.split_whitespace();
    let (Some(method), Some(target)) = (parts.next(), parts.next()) else {
        return Ok(None);
    };

    let mut range = None;
    let mut header_bytes = request_line.len();
    loop {
        let mut line = String::new();
        let read = reader.read_line(&mut line)?;
        header_bytes += read;
        if read == 0 || header_bytes > MAX_HEADER_BYTES {
            return Ok(None);
        }
        let line = line.trim_end();
        if line.is_empty() {
            break;
        }
        if let Some((name, value)) = line.split_once(':') {
            if name.trim().eq_ignore_ascii_case("range") {
                range = Some(value.trim().to_string());
            }
        }
    }

    Ok(Some(Request {
        method: method.to_string(),
        target: target.to_string(),
        range,
    }))
}

/// `/<token>/<percent-encoded absolute path>` → that path, if it's a video file.
fn resolve_path(target: &str, prefix: &str) -> Option<PathBuf> {
    let encoded = target.strip_prefix(prefix)?;
    let encoded = encoded.split(['?', '#']).next().unwrap_or_default();
    let path = PathBuf::from(percent_decode(encoded)?);

    if !path.is_absolute() || !is_media_file(&path) {
        return None;
    }
    Some(path)
}

fn percent_decode(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let hex = std::str::from_utf8(bytes.get(index + 1..index + 3)?).ok()?;
            output.push(u8::from_str_radix(hex, 16).ok()?);
            index += 3;
        } else {
            output.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(output).ok()
}

fn extension(path: &Path) -> String {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default()
}

/// Same list as the asset protocol scope in tauri.conf.json.
fn is_media_file(path: &Path) -> bool {
    matches!(
        extension(path).as_str(),
        "mp4" | "mkv" | "mov" | "webm" | "ts" | "avi" | "flv" | "m4v"
    )
}

fn content_type_for(path: &Path) -> &'static str {
    match extension(path).as_str() {
        "mp4" | "m4v" => "video/mp4",
        "mkv" => "video/x-matroska",
        "mov" => "video/quicktime",
        "webm" => "video/webm",
        "ts" => "video/mp2t",
        "avi" => "video/x-msvideo",
        "flv" => "video/x-flv",
        _ => "application/octet-stream",
    }
}

/// Single `bytes=` ranges only (all a media player sends). Returns inclusive bounds.
fn parse_range(header: &str, len: u64) -> Option<(u64, u64)> {
    let spec = header.trim().strip_prefix("bytes=")?;
    if spec.contains(',') || len == 0 {
        return None;
    }
    let (start, end) = spec.split_once('-')?;
    let (start, end) = (start.trim(), end.trim());

    if start.is_empty() {
        // Suffix range: the last N bytes.
        let suffix: u64 = end.parse().ok()?;
        if suffix == 0 {
            return None;
        }
        return Some((len.saturating_sub(suffix), len - 1));
    }

    let start: u64 = start.parse().ok()?;
    if start >= len {
        return None;
    }
    let end = if end.is_empty() {
        len - 1
    } else {
        end.parse::<u64>().ok()?.min(len - 1)
    };
    (start <= end).then_some((start, end))
}

fn respond_empty(
    stream: &mut TcpStream,
    status: &str,
    headers: &[(&str, &str)],
) -> std::io::Result<()> {
    let mut head = format!("HTTP/1.1 {status}\r\nContent-Length: 0\r\nConnection: close\r\n");
    for (name, value) in headers {
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    head.push_str("\r\n");
    stream.write_all(head.as_bytes())?;
    stream.flush()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_byte_ranges() {
        assert_eq!(parse_range("bytes=0-", 100), Some((0, 99)));
        assert_eq!(parse_range("bytes=10-19", 100), Some((10, 19)));
        assert_eq!(parse_range("bytes=90-500", 100), Some((90, 99)));
        assert_eq!(parse_range("bytes=-10", 100), Some((90, 99)));
        assert_eq!(parse_range("bytes=100-", 100), None);
        assert_eq!(parse_range("bytes=0-1,5-6", 100), None);
        assert_eq!(parse_range("items=0-1", 100), None);
    }

    #[test]
    fn resolves_only_tokenised_video_paths() {
        let prefix = "/abc/";
        assert_eq!(
            resolve_path("/abc/%2Fhome%2Fme%2Fmy%20clip%231.mp4", prefix),
            Some(PathBuf::from("/home/me/my clip#1.mp4"))
        );
        assert_eq!(resolve_path("/wrong/%2Fhome%2Fme%2Fa.mp4", prefix), None);
        assert_eq!(resolve_path("/abc/%2Fetc%2Fpasswd", prefix), None);
        assert_eq!(resolve_path("/abc/relative.mp4", prefix), None);
        assert_eq!(resolve_path("/abc/%2Fbad%ZZ.mp4", prefix), None);
    }

    #[test]
    fn serves_ranges_over_http() {
        let dir = std::env::temp_dir().join(format!("cutdown-media-test-{}", random_token()));
        std::fs::create_dir_all(&dir).unwrap();
        let clip = dir.join("clip.mp4");
        std::fs::write(&clip, b"0123456789").unwrap();

        let base = start().unwrap();
        let url = format!("{base}{}", clip.to_string_lossy().replace('/', "%2F"));
        let (host_port, path) = url
            .strip_prefix("http://")
            .and_then(|rest| rest.split_once('/'))
            .unwrap();

        let mut stream = TcpStream::connect(host_port).unwrap();
        write!(stream, "GET /{path} HTTP/1.1\r\nHost: x\r\nRange: bytes=2-5\r\n\r\n").unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();

        assert!(response.starts_with("HTTP/1.1 206"), "{response}");
        assert!(response.contains("Content-Range: bytes 2-5/10"));
        assert!(response.contains("Content-Type: video/mp4"));
        assert!(response.ends_with("\r\n\r\n2345"));

        let _ = std::fs::remove_dir_all(dir);
    }
}

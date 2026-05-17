# Cutdown

Cutdown is a lightweight Windows-first desktop video clipping tool for OBS replay buffer workflows. The goal is to make trimming, compressing, cropping, exporting, and sharing gameplay clips fast enough that the app can stay out of the way until it is needed.

The project uses Tauri v2, Svelte, TypeScript, Rust, plain CSS, and ffmpeg as the only video processing dependency.

## Current MVP

The first milestone is a working local editor and lossless export baseline:

- Open a local video file from the editor.
- Probe clip metadata through `ffprobe`.
- Preview video with an HTML `video` element.
- Set in/out trim points with timeline handles.
- Use keyboard shortcuts for `I`, `O`, `Space`, frame stepping, and 5-second stepping.
- Export a lossless stream-copy trim through `ffmpeg`.
- Use a Windows tray menu for `Open Editor` and `Quit`.

## Requirements

- Windows 10/11 x64.
- Node.js with npm.
- Rust toolchain with Cargo.
- Microsoft Edge WebView2 Runtime.
- Visual Studio Build Tools with C++ workload.
- ffmpeg and ffprobe.

For development, ffmpeg can be available on `PATH`. For release packaging, static Windows `ffmpeg.exe` and `ffprobe.exe` should be bundled in:

```text
public/ffmpeg/
```

## Development

Install dependencies:

```powershell
npm install
```

Run validation:

```powershell
npm run check
cargo check --manifest-path src-tauri/Cargo.toml
```

Run the desktop app:

```powershell
npm run tauri dev
```

Build the app and installer:

```powershell
npm run tauri -- build
```

## Roadmap

### Milestone 1: MVP Editor

Status: in progress.

- Tauri v2 + Svelte + TypeScript scaffold.
- Tray menu and editor window management.
- Editor screen with video preview, timeline, trim controls, and export modal.
- Rust commands for `probe_video`, `export_clip`, and placeholder GPU detection.
- Lossless stream-copy export with ffmpeg.
- Basic Windows icon and NSIS build path.

### Milestone 2: ffmpeg Bundling and Export Hardening

- Bundle static Windows x64 `ffmpeg.exe` and `ffprobe.exe`.
- Add clear missing-binary errors and setup diagnostics.
- Parse ffmpeg stderr for progress events and ETA.
- Emit `export_progress` events to the frontend.
- Add open-in-explorer and success notification actions.
- Add safer output naming and overwrite handling.
- Add runtime tests with sample mp4, mkv, and mov clips.

### Milestone 3: Presets and Compression

- Implement built-in presets: Discord, Lossless Trim, Archive, and Twitter/X.
- Persist presets to `%APPDATA%/Cutdown/presets.json`.
- Detect available GPU encoders: `h264_nvenc`, `h264_amf`, and `h264_qsv`.
- Prefer GPU encoding when available, with fallback to `libx264`.
- Implement target-size encoding for Discord-style 10MB exports.
- Add two-pass bitrate calculation and retry handling for oversized output.

### Milestone 4: Crop

- Add crop mode to the video preview.
- Support draggable and resizable crop selection.
- Snap near common aspect ratios: 16:9, 9:16, 4:3, and 1:1.
- Pass crop settings to ffmpeg as a `crop=w:h:x:y` filter.
- Automatically re-encode when crop is enabled.

### Milestone 5: Watch Folder Workflow

- Add settings for the OBS replay buffer watch folder.
- Use Rust `notify` to monitor new video files.
- Show Windows toast notifications for new clips.
- Open the editor with the selected clip from a notification.
- Keep tray behavior fast and quiet on startup.

### Milestone 6: Upload and Sharing

- Add upload host adapter modules.
- Implement Catbox.moe anonymous upload first.
- Add Streamable with credentials stored through the OS keychain.
- Investigate FileGarden API support and either implement or document as TODO.
- Add upload selection to export flow and copy-link success actions.

### Milestone 7: Clip History

- Store the last 20 exported clips in `%APPDATA%/Cutdown/history.json`.
- Generate and persist thumbnails at export time.
- Add a collapsible history drawer.
- Support reopen-in-editor and copy-share-link actions.

### Milestone 8: Settings and Windows Integration

- Add minimal settings for watch folder, export folder, default preset, upload host, startup behavior, GPU override, notifications, and upload credentials.
- Add run-on-startup support through `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`.
- Register Windows Open With support for `.mp4`, `.mkv`, `.mov`, `.avi`, `.webm`, `.ts`, and `.flv`.
- Document registry keys for manual add/remove.

### Milestone 9: Performance Audit

- Measure cold launch to tray icon.
- Measure editor open latency from tray, notification, and Open With entry.
- Measure preview readiness for large clips.
- Profile thumbnail generation and ensure it remains async.
- Measure idle RAM and final app binary size.

## Project Structure

```text
src-tauri/
  src/
    main.rs
    ffmpeg.rs
    encoder_detect.rs
src/
  App.svelte
  components/
  stores/
  lib/
public/
  ffmpeg/
```

See `PROGRESS.md` for the current implementation status and validation history.

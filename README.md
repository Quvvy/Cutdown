# Cutdown

Cutdown is a lightweight Windows-first desktop video clipping tool for OBS replay buffer workflows. The goal is to make trimming, compressing, cropping, exporting, and sharing gameplay clips fast enough that the app can stay out of the way until it is needed.

The project uses Tauri v2, Svelte, TypeScript, Rust, plain CSS, and ffmpeg as the only video processing dependency.

## Current MVP

The current milestone is a working local multi-cut editor with compression presets:

- Open a local video file from the editor, watch-folder toast, or **Open With** on a video file.
- Probe clip metadata through `ffprobe`.
- Preview video with an HTML `video` element (native, remux, or proxy fallback).
- Split the clip into multiple kept segments.
- Select/delete unwanted segments.
- Set an I/O range on the source timeline and use it for editing and export.
- Undo/redo segment edits.
- Export with **Lossless Trim** (stream-copy) or compressed presets: **Discord**, **Archive**, **Twitter/X**.
- Export kept segments as a sequence, or export the I/O range as a single trim.
- Watch an OBS replay folder and get a toast when a new clip appears.
- Use the system tray: close (X) minimizes to tray; left-click the tray icon or choose **Open Editor** to restore; **Quit** exits.
- Review recent exports in **History**, upload to Catbox, and copy share links.
- Crop the preview (16:9, 9:16, or free) before export.
- Adjust clip volume for preview and export.

Current shortcuts:

- `S`: split at playhead.
- `I` / `O`: set range in/out points.
- `Delete` / `Backspace`: delete selected segment.
- `Ctrl+Z`: undo segment edit.
- `Ctrl+Y` / `Ctrl+Shift+Z`: redo segment edit.
- `L`: toggle preview loop inside the I/O range.
- `Z`: zoom timeline to the I/O range.
- `Space`: play/pause.
- `Left` / `Right`: step by frame.
- `Shift+Left` / `Shift+Right`: step by 5 seconds.
- `Escape`: close Export, Settings, or History panels.

Range actions are also available from the transport bar, timeline context menu, and export modal (sequence vs I/O range).

Known limitations:

- Stream-copy cuts are fast and lossless, but not always frame-perfect because keyframes matter.
- Volume can be adjusted for preview and export; fade/strip/waveform editing is not implemented yet.
- Preview support is currently limited by WebView2/HTML video decoding. ffmpeg may support files that the preview cannot play until proxy/remux preview support is added.

## Requirements

- Windows 10/11 x64.
- Node.js with npm.
- Rust toolchain with Cargo.
- Microsoft Edge WebView2 Runtime.
- Visual Studio Build Tools with C++ workload.
- ffmpeg and ffprobe.

For development, ffmpeg can be available on `PATH`. For release packaging, static Windows `ffmpeg.exe` and `ffprobe.exe` are packaged from:

```text
public/ffmpeg/
```

Populate that folder from the locally installed ffmpeg build:

```powershell
npm run prepare:ffmpeg
```

The app checks ffmpeg availability on startup (see **Settings** in the toolbar). If bundled binaries are missing, install ffmpeg on PATH or run `npm run prepare:ffmpeg` before building.

## Development

Install dependencies:

```powershell
npm install
```

Run validation:

```powershell
npm run check
npm run validate:release
cargo check --manifest-path src-tauri/Cargo.toml
```

Run the desktop app:

```powershell
npm run tauri dev
```

Build the app and installer:

```powershell
npm run prepare:ffmpeg
npm run tauri -- build
```

## Runtime testing

See [docs/TESTING.md](docs/TESTING.md) for the manual validation matrix (probe, preview, edit, export, watch folder).

## Export presets

| Preset | Use |
|--------|-----|
| Lossless Trim | Fast stream-copy, no quality loss (default) |
| Discord | H.264/AAC sized for ~9 MB uploads |
| Archive | High-quality H.264 for keeping clips |
| Twitter / X | 720p-friendly H.264 export |

Enable **Prefer GPU encoding** in Settings when NVENC/AMF/QSV is available.

## Watch folder (OBS replay buffer)

1. Open **Settings** in the toolbar.
2. Choose your OBS replay buffer output folder.
3. Enable watch-folder notifications and save.
4. When a new video file appears, Cutdown shows a Windows toast and can open the clip in the editor.

## Roadmap

### Milestone 1: Multi-Cut MVP Editor

Status: complete.

### Milestone 2: ffmpeg Bundling and Export Hardening

Status: mostly complete.

- Bundle static Windows x64 `ffmpeg.exe` and `ffprobe.exe`.
- `check_ffmpeg` diagnostics and setup guidance.
- Parse ffmpeg stderr for progress events (percent).
- Emit `export_progress` events to the frontend.
- Open-in-explorer, success notification, overwrite confirm, last export folder.
- Preview remux/proxy when native WebView2 playback fails.

### Milestone 3: Reliable Preview

Status: first implementation complete.

### Milestone 4: Presets and Compression

Status: v1 complete (built-in presets, GPU detection, Discord size targeting).

### Milestone 5: Crop

Status: session crop v1 complete (preview overlay + ffmpeg export).

### Milestone 6: Watch Folder Workflow

Status: MVP complete.

### Milestone 9: Windows Integration

Status: partial (Open With associations, launch path, default export folder, run at startup).

### Milestone 7–10

Catbox upload, clip history, performance baseline, volume control, and UI polish are documented in [PROGRESS.md](PROGRESS.md).

### Milestone 11–18 (planned)

See [PROGRESS.md](PROGRESS.md) for the full backlog: trim accuracy, audio editing, timeline workflow, session save, export/presets v2, preview improvements, OBS integration, and cross-platform release hardening.

## Project Structure

```text
src-tauri/
  src/
    main.rs
    ffmpeg.rs
    presets.rs
    settings.rs
    watch_folder.rs
    launch.rs
    windows_integration.rs
    encoder_detect.rs
src/
  App.svelte
  components/
  stores/
  lib/
docs/
  TESTING.md
public/
  ffmpeg/
```

See `PROGRESS.md` for the current implementation status and validation history.

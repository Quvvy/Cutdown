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
- Export with **Lossless Trim** (stream-copy) or compressed presets: **Discord**, **Archive**, **Twitter/X**, plus **custom presets** you define in Settings.
- Export kept segments as a sequence, or export the I/O range as a single trim.
- Watch an OBS replay folder and get a toast when a new clip appears.
- Use the system tray: close (X) minimizes to tray; left-click the tray icon or choose **Open Editor** to restore; **Quit** exits.
- Review recent exports in **History**, upload to **Catbox**, **File Garden**, or a **custom HTTP server**, and copy share links.
- Crop the preview (16:9, 9:16, or free) before export.
- Adjust clip volume for preview and export, or **strip audio** on export.
- **Fit**, zoom, and pan the preview; save a **`.cutdown` project** to keep cuts, range, crop, and markers.

### Upload targets

Configure providers in **Settings → Upload targets**:

- **Catbox** — anonymous or registered uploads via the Catbox API.
- **File Garden** — sign in with email and password via `api.filegarden.com`; session is stored locally.
- **Custom HTTP** — POST multipart to your own media server (URL, file field name, optional `Authorization` header, plain-text or JSON URL response).

Use the **Upload** menu in the bottom bar (after export) or in clip history to pick a target. Known upload and OBS secrets are stored in Windows Credential Manager; non-secret settings live in `%APPDATA%/Cutdown/settings.json`.

Current shortcuts:

- `S`: split at playhead.
- `I` / `O`: set range in/out points.
- `Delete` / `Backspace`: delete selected segment.
- `Ctrl+Z`: undo segment edit.
- `Ctrl+Y` / `Ctrl+Shift+Z`: redo segment edit.
- `Shift+L`: toggle preview loop inside the I/O range.
- `Z`: zoom timeline to the I/O range.
- `Space`: play/pause.
- `Left` / `Right`: step by frame.
- `Shift+Left` / `Shift+Right`: step by 5 seconds.
- `J` / `K` / `L`: step back 1s, pause, step forward 1s.
- `[` / `]`: snap playhead to range in / out.
- `Ctrl+D`: duplicate selected segment.
- Drag a selected segment's left or right edge to trim or extend the cut.
- `Escape`: close open panels, modals, and confirmation dialogs.

Range actions are also available from the transport bar, timeline context menu, and export modal (sequence vs I/O range).

Known limitations:

- Stream-copy cuts are fast and lossless, but not always frame-perfect because keyframes matter.
- Volume can be adjusted for preview and export; audio waveform on the timeline; fade in/out on export.
- Preview uses WebView2/HTML video with seek-at-cut transitions. Brief A/V blips between reordered segments are possible; remux/proxy fallbacks apply when native decode fails.
- Reopening a raw video starts a fresh edit; use **Save project** (`.cutdown`) to restore your work.

## Requirements

- Windows 10/11 x64.
- Node.js with npm.
- Rust toolchain with Cargo.
- Microsoft Edge WebView2 Runtime.
- Visual Studio Build Tools with C++ workload.
- ffmpeg and ffprobe.

For development, ffmpeg can be on `PATH`, or copied into `public/ffmpeg/` via `npm run prepare:ffmpeg`.

**Release installers do not bundle ffmpeg.** The NSIS installer downloads the latest Windows essentials build during setup (~80 MB, one-time install to `%LOCALAPPDATA%\Cutdown\ffmpeg`). If that step fails, use **Install ffmpeg** in the app banner or install ffmpeg on PATH yourself.

## Development

Install dependencies:

```powershell
npm install
```

Run validation:

```powershell
npm run check
npm test
npm run validate:release
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

App icons (window, taskbar, tray, installer, favicon) are generated from `branding/app-icon.png`. After changing the logo, run:

```powershell
npm run icons
```

## Runtime testing

See [docs/TESTING.md](docs/TESTING.md) for the manual validation matrix (probe, preview, edit, export, watch folder), and [docs/RELEASE.md](docs/RELEASE.md) for the release checklist.

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

### Milestone 2: ffmpeg Installation and Export Hardening

Status: mostly complete.

- Install a pinned Windows x64 ffmpeg/ffprobe build from the in-app banner, with PATH and development-folder fallbacks.
- `check_ffmpeg` diagnostics and setup guidance.
- Parse ffmpeg stderr for progress events (percent).
- Emit `export_progress` events to the frontend.
- Open-in-explorer, success notification, overwrite confirm, last export folder.
- Preview remux/proxy when native WebView2 playback fails.

### Milestone 3: Reliable Preview

Status: complete (native / remux / proxy fallbacks; segment-aware playback driver).

### Milestone 4: Presets and Compression

Status: v1 complete (built-in presets, GPU detection, Discord size targeting).

### Milestone 5: Crop

Status: complete (preview overlay + ffmpeg export).

### Milestone 6: Watch Folder Workflow

Status: MVP complete.

### Milestone 9: Windows Integration

Status: complete (Open With, launch path, default export folder, run at startup, tray).

### Milestone 7–10

Catbox upload, clip history, performance baseline, volume control, and UI polish are documented in [PROGRESS.md](PROGRESS.md).

### Milestone 11–18

See [PROGRESS.md](PROGRESS.md) for backlog status. Timeline workflow (M13) and preview/input v2 (M16) are complete. **v0.3** shipped project-only persistence, timeline refactor, and in-app updates (from v0.2.4). OBS WebSocket (M17) is deferred.

## Project Structure

```text
src-tauri/
  src/
    main.rs
    command_util.rs
    ffmpeg.rs
    ffmpeg_install.rs
    presets.rs
    settings.rs
    upload/
    upload_providers.rs
    clip_history.rs
    project.rs
    obs.rs
    watch_folder.rs
    launch.rs
    windows_integration.rs
    encoder_detect.rs
src/
  App.svelte
  components/
  stores/
  lib/
    timelineEditing.ts
    timelineTrackSizing.ts
    projectFile.ts
    segmentBounds.ts
docs/
  TESTING.md
public/
  ffmpeg/
```

See `PROGRESS.md` for the current implementation status and validation history.

# Cutdown Progress

This file tracks implementation progress against the original Cutdown product prompt.

## Current Status

The project is a multi-cut MVP with I/O range editing, export hardening, and an OBS watch-folder workflow. The app scaffolds, type-checks, compiles, and builds with Tauri on Windows.

## Completed

- Initialized a Tauri v2, Svelte, TypeScript, and Rust project.
- Added plain CSS styling with a dark utility-focused interface.
- Added a Windows tray menu with `Open Editor` and `Quit`.
- Added a single-screen editor layout with consolidated toolbar (no faux menu bar).
- Added HTML `video` preview with current timestamp overlay and I/O loop playback.
- Added a segment-based timeline with top-ruler scrubbing and I/O range markers.
- Added split/delete segment editing and I/O range actions (split I/O, keep, trim outside, zoom, export range).
- Removed segment reordering and preview-selected-segment / fake audio metering UI.
- Added undo/redo for segment edits.
- Added keyboard shortcuts for editing, range, loop, and zoom.
- Added an export modal with sequence vs I/O range modes.
- Added Rust `probe_video`, `export_clip`, `check_ffmpeg`, preview remux/proxy, and `reveal_in_explorer`.
- Added stage-based `export_progress` events with optional ffmpeg percent parsing.
- Added bundled ffmpeg path resolution and startup diagnostics.
- Added export UX: last output folder memory, overwrite confirm, success notification.
- Added settings persistence in `%APPDATA%/Cutdown/settings.json`.
- Added watch-folder monitoring with Windows toast + auto-open in editor.
- Added `docs/TESTING.md` runtime validation matrix.

## Validation

Last checked:

- `npm run check` passes with 0 errors and 0 warnings.
- `cargo check` passes.
- Manual runtime matrix documented in [docs/TESTING.md](docs/TESTING.md).

Previously validated:

- `npm run build` passes.
- `npm run tauri -- build` passes and produces a Windows NSIS bundle.

## Known Issues and Risks

- ffmpeg and ffprobe must be on PATH for dev, or copied via `npm run prepare:ffmpeg` for bundled release builds.
- The multi-cut MVP still needs broader runtime testing with real sample clips and audio variations (see test matrix).
- `npm audit --omit=dev` reports a moderate Svelte advisory. npm's automated fix requires a breaking Svelte 5 upgrade, so it is deferred.
- Stream-copy cuts are fast and lossless, but may not be frame-perfect because keyframes matter.
- Preview support depends on WebView2/HTML video support. ffmpeg can support more formats than the preview can play.
- Watch-folder notifications require Windows notification permission.

## Next Up

1. Complete manual runtime matrix on real OBS/replay-buffer clips.
2. Compression presets (Discord, lossless trim, GPU encode).
3. Crop mode and Windows Open With integration.

## Roadmap Status

### Milestone 1: Multi-Cut MVP Editor

Status: complete.

### Milestone 2: Export, Audio, and ffmpeg Hardening

Status: mostly complete.

Remaining:

- Broader runtime testing with real clips.
- Optional: frame-accurate re-encode trim mode.

### Milestone 3: Reliable Preview

Status: first implementation complete.

### Milestone 4: Presets and Compression

Status: not started.

### Milestone 5: Crop

Status: not started.

### Milestone 6: Watch Folder Workflow

Status: MVP complete.

Implemented:

- Watch folder setting and enable toggle.
- `notify`-based folder monitoring with debounce and extension filter.
- Windows toast on new clips.
- Open clip in editor from watch event (with unsaved-edit confirm).

### Milestone 7: Upload and Sharing

Status: not started.

### Milestone 8: Clip History

Status: not started.

### Milestone 9: Settings and Windows Integration

Status: partial (minimal settings panel only).

### Milestone 10: Performance Audit

Status: not started.

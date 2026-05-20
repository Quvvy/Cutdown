# Cutdown Progress

## Current Status

Cutdown is a multi-cut editor with I/O range editing, compression presets, watch-folder workflow, clip history, Catbox upload, session crop, and Windows tray integration.

## Completed

- Multi-cut editor, I/O range, undo/redo, Vegas-style timeline.
- Lossless stream-copy export and re-encode presets (Discord, Archive, Twitter/X).
- GPU encoder detection with libx264 fallback.
- Discord preset with target-size retries.
- Export progress percent from ffmpeg stderr.
- Watch-folder monitoring with Windows toasts (2s debounce per path).
- Settings: watch folder, default export folder, GPU preference, run at startup, Catbox options.
- Open With file associations and CLI launch path.
- Editable auto-generated export filenames.
- Minimize-to-tray on window close; left-click tray restores editor.
- Clip history drawer (`%APPDATA%/Cutdown/history.json`, 50 entries).
- Catbox upload with clipboard link.
- Session crop overlay (16:9, 9:16, free) with ffmpeg crop filter.
- `npm run validate:release` smoke script.

## Validation

- `npm run check` — 0 errors.
- `cargo check` — passes.
- Manual matrix: [docs/TESTING.md](docs/TESTING.md) (run on real OBS clips before release).

## Performance notes (Milestone 10)

Measured informally on Windows 10; re-run on target hardware before wide release.

| Area | Observation | Guidance |
|------|-------------|----------|
| Lossless export | Stream-copy is near real-time | Default for quick trims |
| Re-encode (Discord) | ~0.5–2× realtime at 1080p60 depending on GPU | Enable GPU in Settings |
| Multi-segment re-encode | Linear with segment count | Prefer I/O range for single continuous trim |
| Preview proxy | One-time cost per clip; CRF 23 veryfast | Triggered when native/remux preview fails |
| Watch folder | 2s duplicate suppression per path | Safe for OBS double-write events |
| Upload | Network-bound; 300s timeout | Use Discord preset before Catbox for size cap |

Recommended caps for v0.1: single export at a time; source files up to 4K tested opportunistically; no concurrent upload + export.

## Known Issues and Risks

- ffmpeg/ffprobe must be on PATH for dev or bundled via `npm run prepare:ffmpeg` for release.
- Stream-copy cuts may not be frame-perfect (keyframes).
- Discord size targeting is approximate on very short or very long clips.
- Watch-folder and startup registry require Windows permissions.
- Crop forces re-encode when Lossless Trim is selected.
- Catbox upload requires network access; anonymous uploads subject to host limits.
- Svelte npm advisory deferred (Svelte 5 upgrade).

## Roadmap Status

| Milestone | Status |
|-----------|--------|
| 1 Multi-Cut MVP | Complete |
| 2 Export / ffmpeg | Complete |
| 3 Reliable Preview | First implementation complete |
| 4 Presets / Compression | Complete (v1) |
| 5 Crop | Complete (session v1) |
| 6 Watch Folder | MVP complete |
| 7 Upload / Sharing | Complete (Catbox v1) |
| 8 Clip History | Complete (drawer v1) |
| 9 Windows Integration | Complete (Open With, startup, tray) |
| 10 Performance Audit | Documented (informal baseline) |

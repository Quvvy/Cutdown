# Cutdown Progress

## Current Status

Cutdown is a multi-cut editor with I/O range editing, compression presets, watch-folder workflow, and partial Windows integration. The app type-checks, compiles, and builds with Tauri on Windows.

## Completed

- Multi-cut editor, I/O range, undo/redo, Vegas-style timeline.
- Lossless stream-copy export and re-encode presets (Discord, Archive, Twitter/X).
- GPU encoder detection (`h264_nvenc`, `h264_amf`, `h264_qsv`) with libx264 fallback.
- Discord preset with target-size retries.
- Export progress percent from ffmpeg stderr.
- Watch-folder monitoring with Windows toasts.
- Settings: watch folder, default export folder, GPU preference, run at startup.
- Open With file associations (installer) and CLI launch path.
- `npm run validate:release` smoke script.

## Validation

- `npm run check` — 0 errors.
- `cargo check` — passes.
- Manual matrix: [docs/TESTING.md](docs/TESTING.md) (run on real clips before release).

## Known Issues and Risks

- ffmpeg/ffprobe must be on PATH for dev or bundled via `npm run prepare:ffmpeg` for release.
- Stream-copy cuts may not be frame-perfect (keyframes).
- Discord size targeting is approximate on very short or very long clips.
- Watch-folder and startup registry require Windows permissions.
- Svelte npm advisory deferred (Svelte 5 upgrade).

## Next Up

1. Execute full manual test matrix on OBS/replay-buffer clips.
2. Clip history drawer and upload hosts (Catbox).
3. Crop mode.
4. Performance audit before wide release.

## Roadmap Status

| Milestone | Status |
|-----------|--------|
| 1 Multi-Cut MVP | Complete |
| 2 Export / ffmpeg | Mostly complete |
| 3 Reliable Preview | First implementation complete |
| 4 Presets / Compression | Complete (v1) |
| 5 Crop | Not started |
| 6 Watch Folder | MVP complete |
| 7 Upload / Sharing | Not started |
| 8 Clip History | Not started |
| 9 Windows Integration | Partial (Open With, startup, default folders) |
| 10 Performance Audit | Not started |

# Cutdown Progress

## Current Status

Cutdown is a multi-cut editor with I/O range editing, compression presets, watch-folder workflow, clip history, Catbox upload, session crop, clip volume, custom toolbar icons, and Windows tray integration.

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
- Clip volume slider (preview + export via ffmpeg `volume` filter).
- Custom Fluent toolbar/timeline icons and styled range sliders.
- Timeline zoom-to-range scroll fix.
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
| 11 Trim Accuracy | Planned |
| 12 Audio Editing | Planned |
| 13 Timeline Workflow | Planned |
| 14 Session / Project Save | Planned |
| 15 Export & Presets v2 | Planned |
| 16 Preview & Input v2 | Planned |
| 17 OBS & Tray Workflow | Planned |
| 18 Platform & Release | Planned |

## Upcoming milestones (detail)

### Milestone 11: Trim accuracy

- Optional snap-to-keyframe for stream-copy cuts.
- Optional “accurate cut” mode (re-encode segment boundaries) when frame-perfect in/out matters.
- Document trade-offs in UI (speed vs accuracy).

### Milestone 12: Audio editing

- Strip/mute audio from export UI (backend already supports `AudioMode::Strip`).
- Fade in/out on I/O range.
- Optional waveform on the audio track lane.

### Milestone 13: Timeline workflow

- Drag-reorder segments; duplicate segment.
- J/K/L shuttle scrubbing; snap playhead to I/O markers.
- In-app keyboard shortcut reference.

### Milestone 14: Session / project save

- Persist crop, volume, I/O, and segment list per source file.
- Optional `.cutdown` project file or “restore last session” when reopening a clip.

### Milestone 15: Export & presets v2

- User-defined export presets (CRF, resolution cap, bitrate).
- Export queue (no concurrent export + upload).
- Batch export from history or “one file per segment.”
- Additional upload targets (e.g. Discord webhook) alongside Catbox.
- Video-copy + audio-filter-only path when only volume changes on lossless preset.

### Milestone 16: Preview & input v2

- Drag-and-drop file onto the editor window.
- Recent source files list (complement export history).
- Proactive proxy for heavy codecs or large files.
- Playback speed (0.5× / 1× / 2×).

### Milestone 17: OBS & tray workflow

- OBS WebSocket: load latest replay or tie into replay-buffer naming.
- Tray actions: open watch folder, export-complete notification while minimized.

### Milestone 18: Platform & release

- macOS / Linux ports (tray, file associations, shell integration per OS).
- Tauri auto-updater and signed release CI.
- Rust integration tests for probe + single-segment export.
- Svelte 5 migration (deferred npm advisory).

## Suggested v0.2 priority

| Tier | Focus |
|------|--------|
| **High** | Milestone 11 (trim accuracy), 16 drag-and-drop + recent sources, 13 shortcut cheatsheet |
| **Medium** | 15 custom presets + export queue, 12 strip audio / waveform, 14 session save |
| **Longer** | 17 OBS WebSocket, 18 cross-platform + auto-update + automated export tests |

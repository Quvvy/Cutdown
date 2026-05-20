# Cutdown Progress

## Current Status

Cutdown is a multi-cut editor with I/O range editing, built-in and custom compression presets, watch-folder workflow, clip history, pluggable upload targets (Catbox, File Garden via `api.filegarden.com`, custom HTTP), per-source session restore, session crop, clip volume, preview fit/zoom/pan, custom toolbar icons, and Windows tray integration.

## Completed

- Multi-cut editor, I/O range, undo/redo, Vegas-style timeline.
- Lossless stream-copy export and re-encode presets (Discord, Archive, Twitter/X).
- GPU encoder detection with libx264 fallback.
- Discord preset with target-size retries.
- Export progress percent from ffmpeg stderr.
- Watch-folder monitoring with Windows toasts (2s debounce per path).
- Settings: watch folder, default export folder, GPU preference, run at startup, upload targets.
- Open With file associations and CLI launch path.
- Editable auto-generated export filenames.
- Minimize-to-tray on window close; left-click tray restores editor.
- Clip history drawer (`%APPDATA%/Cutdown/history.json`, 50 entries).
- Pluggable upload providers: Catbox, File Garden (legacy API at `api.filegarden.com`, email/password), custom HTTP multipart (self-hosted).
- Custom export presets (CRF, bitrate, target size, lossless) in Settings and Export modal.
- Preview fit-to-panel, zoom/pan, and workspace splitter resize.
- Session crop overlay (16:9, 9:16, free) with ffmpeg crop filter.
- Clip volume slider (preview + export via ffmpeg `volume` filter).
- Custom Fluent toolbar/timeline icons and styled range sliders.
- Timeline zoom-to-range scroll fix.
- Keyboard shortcuts modal (`?`).
- Drag-and-drop open; recent source list in settings.
- Accurate trim export option.
- Per-source session restore (segments, I/O, crop, volume).
- `.cutdown` project save/open; export batch per segment; queued upload after export.
- Audio fade in/out on export; trim quality hints in export modal.
- OBS WebSocket SaveReplayBuffer; latest replay from watch folder; tray Open Watch Folder.
- Preview playback speed (0.5×–2×) and on-demand proxy preview.
- Strip audio on export; J/K/L scrub; snap to I/O; segment duplicate and drag-reorder.
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
| 7 Upload / Sharing | Complete (v2: Catbox, File Garden, custom) |
| 8 Clip History | Complete (drawer v1) |
| 9 Windows Integration | Complete (Open With, startup, tray) |
| 10 Performance Audit | Documented (informal baseline) |
| 11 Trim Accuracy | Partial (accurate trim + export trim hints) |
| 12 Audio Editing | Partial (strip audio, I/O fades on export) |
| 13 Timeline Workflow | Partial (shortcuts modal, J/K/L, snap I/O, reorder/duplicate) |
| 14 Session / Project Save | Partial (AppData session + `.cutdown` project files) |
| 15 Export & Presets v2 | Partial (custom presets, batch export, queued upload) |
| 16 Preview & Input v2 | Partial (drag-drop, recent, fit/zoom/pan, speed, proxy button) |
| 17 OBS & Tray Workflow | Partial (WebSocket save replay, watch folder tray, latest replay) |
| 18 Platform & Release | Planned |

## Upcoming milestones (detail)

### Milestone 11: Trim accuracy

- Optional “accurate cut” mode (re-encode segment boundaries) when frame-perfect in/out matters.
- Document trade-offs in UI (speed vs accuracy).

### Milestone 12: Audio editing

- Strip/mute audio from export UI (done).
- Fade in/out on I/O range.
- Optional waveform on the audio track lane.

### Milestone 13: Timeline workflow

- Drag-reorder segments; duplicate segment (done).
- J/K/L shuttle scrubbing; snap playhead to I/O markers (done).
- In-app keyboard shortcut reference (done via `?` modal).

### Milestone 14: Session / project save

- Per-source session in `%APPDATA%/Cutdown/sessions.json` (done).
- Optional `.cutdown` project file for sharing projects.

### Milestone 15: Export & presets v2

- User-defined export presets (done).
- Export queue (no concurrent export + upload).
- Batch export from history or “one file per segment.”
- Additional built-in upload targets (e.g. Discord webhook).
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

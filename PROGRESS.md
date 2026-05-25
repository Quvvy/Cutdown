# Cutdown Progress

## Current Status

Cutdown is a multi-cut editor with I/O range editing, built-in and custom compression presets, watch-folder workflow, clip history, pluggable upload targets (Catbox, File Garden via `api.filegarden.com`, custom HTTP), `.cutdown` project files, crop, clip volume, preview fit/zoom/pan, custom toolbar icons, in-app updates, and Windows tray integration.

**v0.3.0** refactors timeline editing into tested frontend modules, uses a flexible timeline grid layout, and saves editor state through `.cutdown` projects instead of automatic per-source session files.

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
- Crop overlay (16:9, 9:16, free) with ffmpeg crop filter.
- Clip volume slider (preview + export via ffmpeg `volume` filter).
- Custom Fluent toolbar/timeline icons and styled range sliders.
- Timeline zoom-to-range scroll fix.
- Keyboard shortcuts modal (`?`).
- Drag-and-drop open; recent sources toolbar menu.
- Accurate trim export option.
- `.cutdown` project save/open (segments, I/O, crop, volume, bookmarks, export options).
- Export batch per segment; queued upload after export.
- Audio fade in/out on export; trim quality hints in export modal.
- Latest replay from watch folder; tray Open Watch Folder; tray minimize hint in Settings.
- Preview playback speed (0.5×–2×) and on-demand proxy preview.
- Strip audio on export; J/K/L scrub; snap to I/O; segment duplicate and drag-reorder.
- Segment edge drag to trim or extend cuts.
- Segment-aware preview driver (`sequencePlayback.ts`) for reordered timelines.
- Keep-only I/O range; `npm run validate:release` smoke script.
- GitHub Actions CI (Windows): typecheck, frontend tests, Rust check/test, Clippy when available.
- **v0.2.4:** In-app updater (GitHub releases + signed NSIS), Settings check-for-updates, run-at-startup targets installed app.
- **v0.3.0:** Timeline editing extracted to `timelineEditing.ts`, track sizing to `timelineTrackSizing.ts`, project payloads to `projectFile.ts`; removed `%APPDATA%` per-source auto-session; timeline tracks fill pane with proportional grid rows; expanded unit tests.

## Validation

- `npm run check` — 0 errors.
- `cargo check` — passes.
- `npm test` — frontend unit tests (timeline mapping, editing, track sizing, project payloads, segment bounds).
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

Recommended caps: single export at a time; source files up to 4K tested opportunistically; no concurrent upload + export.

## Known Issues and Risks

- ffmpeg/ffprobe: on PATH or `public/ffmpeg/` for dev; release installer downloads latest essentials build to `%LOCALAPPDATA%\Cutdown\ffmpeg` (not bundled in the installer package). In-app install is a fallback.
- Stream-copy cuts may not be frame-perfect (keyframes).
- Preview uses a single HTML `<video>` element with seek-at-cut transitions; brief A/V blips at segment boundaries are possible.
- Discord size targeting is approximate on very short or very long clips.
- Watch-folder and startup registry require Windows permissions.
- Crop forces re-encode when Lossless Trim is selected.
- Catbox upload requires network access; anonymous uploads subject to host limits.
- Reopening a raw video file starts a fresh edit; use **Save project** (`.cutdown`) to keep cuts and markers.
- Svelte npm advisory deferred (Svelte 5 upgrade).

## Roadmap Status

| Milestone | Status |
|-----------|--------|
| 1 Multi-Cut MVP | Complete |
| 2 Export / ffmpeg | Complete |
| 3 Reliable Preview | Complete (native / remux / proxy + segment playback driver) |
| 4 Presets / Compression | Complete (v1) |
| 5 Crop | Complete |
| 6 Watch Folder | MVP complete |
| 7 Upload / Sharing | Complete (v2: Catbox, File Garden, custom) |
| 8 Clip History | Complete (drawer v1) |
| 9 Windows Integration | Complete (Open With, startup, tray) |
| 10 Performance Audit | Documented (informal baseline) |
| 11 Trim Accuracy | Partial (accurate trim + export hints; UI copy polish optional) |
| 12 Audio Editing | Partial (strip audio, fades, waveform lane; advanced audio optional) |
| 13 Timeline Workflow | Complete (reorder, duplicate, edge trim, J/K/L, snap I/O, shortcuts, keep range) |
| 14 Project Save | Complete (`.cutdown` projects; no auto AppData session) |
| 15 Export & Presets v2 | Partial (custom presets, batch, queued upload; queue hardening optional) |
| 16 Preview & Input v2 | Complete (drag-drop, recent menu, fit/zoom/pan, speed, proxy) |
| 17 OBS & Tray Workflow | Partial (watch folder, latest replay from folder; WebSocket deferred) |
| 18 Platform & Release | Partial (signed Windows installer + in-app updater; cross-platform deferred) |

## v0.3 shipped

- Timeline editing logic moved to tested frontend modules.
- Project-only persistence (`.cutdown`); per-source auto-restore removed.
- Timeline track layout uses proportional grid rows that fill the editor pane.
- In-app updater from v0.2.4 remains the update path for installed builds.

## Backlog (optional)

### Milestone 11: Trim accuracy

- Clearer in-app copy for accurate trim vs lossless speed trade-offs.

### Milestone 15: Export & presets v2

- Stricter export queue (no concurrent export + upload).
- Discord webhook upload target.
- Video-copy path when only volume changes on lossless preset.

### Milestone 17: OBS & tray

- OBS WebSocket: load latest replay by API.
- Export-complete notification while minimized.

### Milestone 18: Platform & release (longer term)

- macOS / Linux ports.
- Svelte/npm advisory migration.

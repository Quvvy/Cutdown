# Cutdown Progress

## Current Status

Cutdown is a multi-cut editor with I/O range editing, built-in and custom compression presets, watch-folder workflow, clip history, pluggable upload targets (Catbox, File Garden via `api.filegarden.com`, custom HTTP), per-source session restore, session crop, clip volume, preview fit/zoom/pan, custom toolbar icons, and Windows tray integration.

**Timeline and segment preview are frozen for v0.2.x.** Further timeline work is limited to user-reported regressions only (no architectural churn). See [src/lib/sequencePlayback.ts](src/lib/sequencePlayback.ts).

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
- Drag-and-drop open; recent sources toolbar menu.
- Accurate trim export option.
- Per-source session restore (segments, I/O, crop, volume).
- `.cutdown` project save/open; export batch per segment; queued upload after export.
- Audio fade in/out on export; trim quality hints in export modal.
- Latest replay from watch folder; tray Open Watch Folder; tray minimize hint in Settings.
- Preview playback speed (0.5×–2×) and on-demand proxy preview.
- Strip audio on export; J/K/L scrub; snap to I/O; segment duplicate and drag-reorder.
- Segment-aware preview driver (`sequencePlayback.ts`) for reordered timelines.
- Keep-only I/O range; `npm run validate:release` smoke script.
- GitHub Actions CI (Windows): typecheck, frontend tests, Rust check/test, Clippy when available.

## Validation

- `npm run check` — 0 errors.
- `cargo check` — passes.
- `npm test` — frontend unit tests (timeline mapping, sequence playback).
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

Recommended caps for v0.2: single export at a time; source files up to 4K tested opportunistically; no concurrent upload + export.

## Known Issues and Risks

- ffmpeg/ffprobe: on PATH or `public/ffmpeg/` for dev; release uses in-app download or PATH (not bundled in installer).
- Stream-copy cuts may not be frame-perfect (keyframes).
- Preview uses a single HTML `<video>` element with seek-at-cut transitions; brief A/V blips at segment boundaries are possible (acceptable for v0.2).
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
| 3 Reliable Preview | Complete (native / remux / proxy + segment playback driver) |
| 4 Presets / Compression | Complete (v1) |
| 5 Crop | Complete (session v1) |
| 6 Watch Folder | MVP complete |
| 7 Upload / Sharing | Complete (v2: Catbox, File Garden, custom) |
| 8 Clip History | Complete (drawer v1) |
| 9 Windows Integration | Complete (Open With, startup, tray) |
| 10 Performance Audit | Documented (informal baseline) |
| 11 Trim Accuracy | Partial (accurate trim + export hints; UI copy polish optional) |
| 12 Audio Editing | Partial (strip audio, fades, waveform lane; advanced audio optional) |
| 13 Timeline Workflow | Complete (reorder, duplicate, J/K/L, snap I/O, shortcuts, keep range) |
| 14 Session / Project Save | Complete (AppData session + `.cutdown` projects) |
| 15 Export & Presets v2 | Partial (custom presets, batch, queued upload; queue hardening optional) |
| 16 Preview & Input v2 | Complete (drag-drop, recent menu, fit/zoom/pan, speed, proxy) |
| 17 OBS & Tray Workflow | Partial (watch folder, latest replay from folder; WebSocket deferred) |
| 18 Platform & Release | Partial (v0.2 Windows installer; CI added; auto-updater / cross-platform deferred) |

## v0.3 focus (chosen)

**Primary: Milestone 18 — release hardening**

- Keep GitHub Actions CI green on `main`.
- Rust integration tests for probe + single-segment export.
- Signed installer / Tauri auto-updater (when ready for distribution).

**Deferred: Milestone 17 — OBS WebSocket**

- Latest replay via folder scan remains the supported workflow until WebSocket is scoped for a later release.

## Backlog (optional, not blocking v0.2.x)

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

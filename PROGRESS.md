# Progress notes

Internal scratch pad for what’s done, what’s flaky, and what’s next. Not release notes—see GitHub releases for those.

## Now (v0.3.0)

Editor state lives in `.cutdown` project files. We dropped the old `%APPDATA%` auto-session per source path—reopen a video without a project and you get one full-length segment again.

Timeline code got pulled into `timelineEditing.ts`, `timelineTrackSizing.ts`, `projectFile.ts`, `segmentBounds.ts` with tests. Track rows use proportional grid sizing so the timeline fills the pane when you resize the window.

Still on Windows only. Updater (signed NSIS + `latest.json`) landed in 0.2.4.

## Works

Multi-cut timeline, I/O range tools, undo/redo, segment reorder, edge trim/extend, bookmarks, crop, volume, fades, strip audio, waveform lane, fit/zoom preview, proxy preview, watch-folder toasts, Open With, tray, clip history, Catbox / File Garden / custom HTTP upload, custom export presets, GPU encoders, export progress %, `.cutdown` save/load with relink, CI on Windows (`validate:release`).

## Before you tag a release

```powershell
npm run validate:release
```

Then walk [docs/TESTING.md](docs/TESTING.md) on real clips—especially installed NSIS, not just `tauri dev`. Timeline layout has bitten us on WebView2 before when we changed grid sizing; always click around the timeline after UI changes.

## Rough performance (informal, Win10)

| Thing | Ballpark |
|-------|----------|
| Lossless export | ~realtime |
| Discord re-encode 1080p60 | 0.5–2× realtime with GPU |
| Many segments re-encoded | scales with segment count; one I/O export is simpler |
| Proxy build | one-time per clip when preview falls back |

One export at a time is the happy path. Don’t run a big upload and export together.

## Known rough edges

- ffmpeg: installer pulls essentials; dev can use PATH or `public/ffmpeg/`; in-app download is backup.
- Preview = one `<video>` element; segment jumps aren’t seamless.
- Lossless + crop = re-encode.
- File Garden uses the legacy API host; credentials in Credential Manager.
- npm audit on Svelte stack—not addressed yet.
- OBS WebSocket “latest clip” isn’t built; folder scan + watch folder is the supported path.

## Milestone map (abbreviated)

| # | Topic | State |
|---|--------|--------|
| 1–6 | Core editor, export, preview, presets, crop, watch folder | done |
| 7–9 | Upload, history, Windows integration | done |
| 10 | Perf notes | documented only |
| 11–12 | Trim copy, fancier audio | partial |
| 13–14 | Timeline workflow, projects | done (projects replaced auto-session) |
| 15 | Export queue polish | partial |
| 16 | Preview UX | done |
| 17 | OBS WebSocket | deferred |
| 18 | Signed releases / updater | done for Windows; other OSes not started |

## Maybe later

- OBS WebSocket instead of folder guessing
- Export-finished toast while minimized
- macOS/Linux
- Tighter export queue (no overlap with upload)
- Discord webhook target

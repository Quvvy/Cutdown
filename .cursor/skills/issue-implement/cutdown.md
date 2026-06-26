# Cutdown — implementer reference

## Stack

Tauri 2, Svelte 5, Rust, ffmpeg. Windows-first desktop app.

## Validation

Default gate: `npm run validate:release` (check, test, build, cargo check/test, optional clippy).

## Persistence

- Editor state is saved in **`.cutdown` project files** only.
- Reopening a raw video without a project starts a **fresh** full-length segment.
- Do not reintroduce per-source `%APPDATA%` auto-session unless the issue explicitly asks for it.

## Timeline and UI

- Timeline logic lives in `src/lib/timelineEditing.ts`, `timelineTrackSizing.ts`, `segmentBounds.ts`.
- Timeline uses **proportional grid rows** (`fr`) — `tauri dev` in Chrome is not enough to sign off layout changes.
- If you touch `Timeline.svelte` or timeline CSS in `src/styles.css`, say in the PR that the reviewer should verify **installed NSIS / WebView2** (see docs/TESTING.md).

## Export / ffmpeg

- Release builds do not bundle ffmpeg; installer downloads to `%LOCALAPPDATA%\Cutdown\ffmpeg`.
- Stream-copy export is keyframe-aligned; accurate trim and crop may force re-encode.

## Useful paths

| Topic | Path |
|-------|------|
| Main UI | `src/App.svelte` |
| Timeline | `src/components/Timeline.svelte` |
| Export | `src-tauri/src/ffmpeg.rs` |
| Settings | `src-tauri/src/settings.rs` |
| Projects | `src/lib/projectFile.ts`, `src-tauri/src/project.rs` |
| Manual QA | `docs/TESTING.md` |
| Release | `docs/RELEASE.md` |

## PR checklist snippet

When touching timeline, export, or Windows integration, include in PR test plan:

- [ ] `npm run validate:release`
- [ ] Manual row(s) from docs/TESTING.md for affected area
- [ ] NSIS install smoke if timeline/CSS changed

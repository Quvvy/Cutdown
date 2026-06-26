# Review checklist (by area)

Use the rows that match files changed in the PR. Full matrix: [docs/TESTING.md](../../../docs/TESTING.md).

## Always

- [ ] Diff scope matches linked issue
- [ ] `npm run validate:release` called out or CI green
- [ ] No secrets, debug logs, or release binaries committed
- [ ] Commit message and PR description make sense

## `src/lib/*` (timeline, project, mapping)

- [ ] Unit tests added/updated if behavior changed
- [ ] Edge cases: empty segments, min duration, overlap (segment bounds)

## `src/components/Timeline.svelte`, `src/styles.css` (timeline)

- [ ] Layout/zoom/resize still plausible
- [ ] PR notes installed NSIS/WebView2 smoke — dev-only check is insufficient

## `src/App.svelte`

- [ ] No accidental scope creep; editor state still project-based

## `src-tauri/src/ffmpeg.rs`, export paths

- [ ] Export still valid for segment list / I/O range
- [ ] Error messages still user-facing strings

## `src-tauri/src/windows_integration.rs`, tray, startup

- [ ] Startup registry targets installed app path, not dev `target/debug`

## `docs/*`

- [ ] Accurate vs actual behavior (especially persistence: projects not auto-session)

## Windows / installer

- [ ] If `tauri.conf.json` or bundle config changed, RELEASE.md implications noted

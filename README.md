# Cutdown

Windows desktop app for chopping up and compressing videos. Open a file, mark what you want to keep, export, optionally upload a link.

Stack: Tauri 2, Svelte, Rust, ffmpeg. Setup downloads ffmpeg to `%LOCALAPPDATA%\Cutdown\ffmpeg` (needs internet once).

## What it does

**Editing.** Split at the playhead (`S`), delete segments, drag segments to reorder, drag the edges of a selected segment to trim or extend. Set In/Out (`I` / `O`) for a working range—keep only that span, trim everything else to it, or export the range as one file. Undo/redo. Bookmarks on the timeline (`M`). Save your work as a `.cutdown` project file; opening the same MP4 again without a project starts clean.

**Preview.** WebView2 video with remux/proxy fallbacks when the codec is awkward. Fit, zoom, pan. Playback speed and optional proxy for heavy files. Reordered segments play in sequence order; you might hear a tiny blip at joins—that’s the tradeoff for a simple preview path.

**Export.** Lossless Trim (stream copy, fast) or re-encode presets (Discord ~9 MB, Archive, Twitter/X). Custom presets in Settings. Per-segment batch export, I/O range export, crop, volume, fade in/out, strip audio, “accurate trim” when you care about frame boundaries more than speed. GPU encoding when available.

**Around the editor.** Watch folder + toast when OBS drops a new file. Recent sources list. Clip history with reveal/copy path and upload. Tray: X minimizes, tray icon brings the window back. Run at startup (points at the installed exe, not a dev build). In-app updates from GitHub on 0.2.4+.

**Uploads** (Settings → Upload targets): Catbox, File Garden (`api.filegarden.com`), or your own multipart HTTP endpoint. Secrets in Windows Credential Manager; other settings in `%APPDATA%\Cutdown\settings.json`.

Press `?` in the app for the full shortcut list.

## Caveats worth knowing

- Stream-copy export is fast but keyframe-aligned—not always frame-perfect. Use accurate trim or a re-encode preset when that matters.
- Crop on a lossless preset still re-encodes.
- Discord size targeting is a best effort; very short or very long clips can miss the target.
- Projects are how you save edits. Raw file reopen ≠ resume.

## Building from source

Needs Windows 10/11 x64, Node, Rust, WebView2, VS C++ build tools, and ffmpeg on PATH (or `npm run prepare:ffmpeg` for `public/ffmpeg/`).

```powershell
npm install
npm run tauri dev          # dev
npm run validate:release   # check + test + build smoke
npm run tauri -- build     # installer (unsigned unless you set signing env—see docs/RELEASE.md)
npm run icons              # after changing branding/app-icon.png
```

Manual QA checklist: [docs/TESTING.md](docs/TESTING.md). Shipping a build: [docs/RELEASE.md](docs/RELEASE.md).

## OBS watch folder

Settings → pick the replay output folder → enable notifications. New file → toast; open from there or use **Latest replay** if you wired that up.

## Repo layout

```
src-tauri/src/   Rust: ffmpeg, export, settings, upload, project I/O, watch folder, Windows hooks
src/             Svelte UI; lib/ has timeline editing, mapping, project payload helpers
docs/            TESTING.md, RELEASE.md
public/ffmpeg/   optional dev binaries (not in git)
```

Shipped version and known quirks: [PROGRESS.md](PROGRESS.md).

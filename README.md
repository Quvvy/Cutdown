# Cutdown

Desktop app for Windows and Linux for chopping up and compressing videos. Open a file, mark what you want to keep, export, optionally upload a link.

Stack: Tauri 2, Svelte, Rust, ffmpeg. On Windows, setup downloads ffmpeg to `%LOCALAPPDATA%\Cutdown\ffmpeg` (needs internet once). On Linux, Cutdown uses the system ffmpeg (the .deb pulls it in).

## What it does

**Opening.** Right-click a video in Explorer → **Clip with Cutdown** (Windows 11: under *Show more options*), drag it onto the window, or File → Open (Ctrl+O). It opens playing, with the timeline fitted to the clip, frames along the video track and the waveform under it. Portrait phone video, variable-frame-rate recordings and multi-hour files are handled.

**Editing.** Cut at the playhead (`X` or `S`), delete segments, drag segments to reorder, drag the edges of a selected segment to trim or extend. Set In/Out (`I` / `O`) for a working range—keep only that span, trim everything else to it, or export the range as one file. Undo/redo. Bookmarks on the timeline (`M`). Range, snap and marker tools live in the timeline's right-click menu. Save your work as a `.cutdown` project file; opening the same MP4 again without a project starts clean.

**Preview.** WebView2 video with remux/proxy fallbacks when the codec is awkward. HEVC, AV1, and 10-bit files get a playable H.264 proxy automatically instead of a black frame. Fit, zoom, pan — Fit to window also runs when you resize the app window, unless you have zoomed or panned away. Playback speed and optional proxy for heavy files. Reordered segments play in sequence order; you might hear a tiny blip at joins—that’s the tradeoff for a simple preview path.

**Export.** Ctrl+E, Enter: pick Original quality (instant, lossless), Discord (under 20 MB) or Twitter/X, and the kept parts between In and Out are saved next to the original as `name-0m14s.mp4`. **Copy file** then puts the file on the clipboard to paste into Discord. More options: Lossless Trim (stream copy, fast) or re-encode presets (Discord under 20 MB, Archive, Twitter/X). **Audio-only** export with built-in presets (WAV, MP3 128k/192k, OGG). Custom presets in Settings. Per-segment batch export, I/O range export, crop, volume, fade in/out, strip audio, “accurate trim” when you care about frame boundaries more than speed. GPU encoding when available. Output filename extension updates automatically when switching video vs audio.

**Around the editor.** Watch folder + toast when OBS drops a new file. Recent sources list. Clip history with reveal/copy path and upload. Tray: closing the window can hide to tray (Settings); the tray icon brings the window back. Only one instance runs — opening a clip or the app shortcut focuses it. Run at startup (points at the installed exe, not a dev build) follows Settings and is applied as soon as you toggle it. In-app updates from GitHub on 0.2.4+.

**Uploads** (Settings → Upload targets): Catbox, File Garden (`api.filegarden.com`), or your own multipart HTTP endpoint. Secrets in Windows Credential Manager (Linux: the Secret Service keyring — KWallet, GNOME Keyring — or a private `secrets.json` if no keyring is running); other settings in `%APPDATA%\Cutdown\settings.json` (Linux: `~/.config/cutdown/settings.json`).

Press `?` in the app for the full shortcut list.

## Linux

Builds as a `.deb` (Debian, Ubuntu, Mint, Pop!_OS…) and an AppImage (anything else). Differences from Windows:

- **ffmpeg** comes from your package manager, not an in-app download.
- **Opening:** right-click a video in Dolphin / Files → *Open With* → Cutdown (the .deb registers the video types and `.cutdown` projects).
- **Preview** plays through WebKitGTK + GStreamer. If a format shows a black frame, install `gstreamer1.0-libav gstreamer1.0-plugins-good gstreamer1.0-plugins-bad`; the H.264 proxy fallback still kicks in for HEVC/AV1/10-bit. The AppImage bundles its own GStreamer.
- **Tray:** needs a StatusNotifier/AppIndicator tray (KDE Plasma has one; GNOME needs the AppIndicator extension). Restore from the tray icon's menu → *Open Editor*.
- **Run at startup** writes `~/.config/autostart/cutdown.desktop`.
- **Watch-folder alerts** are desktop notifications with *Open* / *Not now*.
- **NVIDIA:** Cutdown sets `WEBKIT_DISABLE_DMABUF_RENDERER=1` on the proprietary driver to avoid a blank window. Set it yourself to `0` to opt out.
- **Updates:** no in-app updater; install the newer .deb / AppImage from GitHub releases.

## Caveats worth knowing

- Stream-copy export is fast but keyframe-aligned—not always frame-perfect. Use accurate trim or a re-encode preset when that matters.
- Crop on a lossless preset still re-encodes.
- Discord size targeting is a best effort; very short or very long clips can miss the target.
- Projects are how you save edits. Raw file reopen ≠ resume.

## Building from source

Needs Windows 10/11 x64, Node, Rust, WebView2, VS C++ build tools, and ffmpeg on PATH (or `npm run prepare:ffmpeg` for `public/ffmpeg/`).

**On Linux** (Ubuntu/Debian names; Fedora/Arch: see the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/#linux)):

```bash
sudo apt install build-essential curl file pkg-config libwebkit2gtk-4.1-dev libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev patchelf ffmpeg nodejs npm cargo rustc
npm install
npm run tauri dev               # dev
npm run validate:release:linux  # check + test + build smoke
npm run build:linux             # .deb + AppImage in target/release/bundle/
sudo apt install ./target/release/bundle/deb/Cutdown_*_amd64.deb
```

Linux bundle settings (targets, MIME types, deb dependencies) live in `src-tauri/tauri.linux.conf.json`, which Tauri merges over `tauri.conf.json` on Linux.

```powershell
npm install
npm run tauri dev          # dev
npm run dev                # UI only, in a normal browser (see below)
npm run validate:release   # check + test + build smoke
npm run tauri -- build     # installer (unsigned unless you set signing env—see docs/RELEASE.md)
npm run icons              # after changing branding/app-icon.png
```

**UI in a browser.** `npm run dev` and open http://127.0.0.1:1420. Without the desktop app, a dev-only stand-in (`src/dev/tauriStub.ts` + `dev/viteDevBackend.ts`) answers the backend calls: the folder button asks for a video path, and probing, the waveform and playback use your real ffmpeg/ffprobe. Export, upload, projects and tray need `npm run tauri dev`. None of this is in production builds. Restart `npm run dev` after editing `dev/viteDevBackend.ts`.

Manual QA checklist: [docs/TESTING.md](docs/TESTING.md). Shipping a build: [docs/RELEASE.md](docs/RELEASE.md). Agent workflow (implement + review issues): [docs/AGENT-WORKFLOW.md](docs/AGENT-WORKFLOW.md).

## OBS watch folder

Settings → pick the replay output folder → enable notifications. New file → toast; open from there or use **Latest replay** if you wired that up.

## Repo layout

```
src-tauri/src/   Rust: ffmpeg, export, settings, upload, project I/O, watch folder, Windows hooks
src/             Svelte UI; lib/ has timeline editing, mapping, project payload helpers
docs/            TESTING.md, RELEASE.md, AGENT-WORKFLOW.md
AGENTS.md        implement + review agent prompts
.cursor/skills/  issue-implement, issue-review
public/ffmpeg/   optional dev binaries (not in git)
```

Shipped version and known quirks: [PROGRESS.md](PROGRESS.md).

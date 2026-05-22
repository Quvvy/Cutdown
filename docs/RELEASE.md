# Release Checklist

Use this checklist for Windows installer releases.

## Version Updates

Keep these versions in sync:

- `package.json`
- `src-tauri/Cargo.toml`
- `src-tauri/tauri.conf.json`

Update `PROGRESS.md` and README feature notes when release behavior changes.

## Validation

Run:

```powershell
npm ci
npm run validate:release
```

The validation script runs frontend typecheck, frontend unit tests, frontend build, Rust check, Rust tests, and Clippy when installed.

Then complete the manual matrix in `docs/TESTING.md`, with special attention to:

- Installer post-install step downloads ffmpeg (requires internet); fallback **Install ffmpeg** banner if download failed.
- Open With / file association launch.
- Probe, native preview, remux preview, and proxy preview.
- Split, I/O range export, per-segment batch export, crop, volume, fade, and audio strip.
- Upload to configured targets and copy share URL.
- Save/load `.cutdown` project files and relink missing sources.
- Watch-folder toast and tray restore/quit behavior.

## Build

Build the installer:

```powershell
npm run tauri -- build
```

Release installers do not bundle ffmpeg. The installer runs `Cutdown.exe --install-dependencies` after setup to download the latest essentials build from gyan.dev. Log: `%LOCALAPPDATA%\Cutdown\install-ffmpeg.log`.

## Smoke Test

On a clean Windows profile or VM:

- Install from the NSIS installer with internet access and verify `%LOCALAPPDATA%\Cutdown\ffmpeg\ffmpeg.exe` exists afterward.
- On a machine where installer download was skipped, start the app with no ffmpeg on PATH and verify the **Install ffmpeg** banner works.
- Open a short MP4, split once, export with Lossless Trim, and reveal the output folder.
- Reopen the app and verify recent source/session restore behavior.

## Publish

- Create or update release notes with user-visible changes, fixes, and known limitations.
- Attach the NSIS installer artifact.
- Tag the release after the installer has passed the smoke test.

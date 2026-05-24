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

For in-app updates, sign release builds with the updater private key (stored locally in `.tauri/cutdown.key`, not committed):

```powershell
$env:TAURI_SIGNING_PRIVATE_KEY = Get-Content -Raw .tauri\cutdown.key
npm run tauri -- build
```

The build emits the NSIS installer plus a `.sig` file when `createUpdaterArtifacts` is enabled in `src-tauri/tauri.conf.json`.

Release installers do not bundle ffmpeg. The installer runs `Cutdown.exe --install-dependencies` after setup to download the latest essentials build from gyan.dev. Log: `%LOCALAPPDATA%\Cutdown\install-ffmpeg.log`.

## In-app updater

The app checks GitHub for updates on startup (after a short delay) and via **Settings → Check for updates**.

After a signed build, generate `latest.json` for the release:

```powershell
.\scripts\generate-latest-json.ps1 `
  -Version 0.2.4 `
  -InstallerPath release\Cutdown_0.2.4_x64-setup.exe `
  -SignaturePath release\Cutdown_0.2.4_x64-setup.exe.sig `
  -Notes "Release notes here"
```

Upload these assets to the GitHub release (same tag as the version, e.g. `v0.2.4`):

- `Cutdown_*_x64-setup.exe` — NSIS installer (required for updates; do not use MSI)
- `Cutdown_*_x64-setup.exe.sig` — signature from the signed build
- `latest.json` — update manifest (must be named exactly `latest.json` for the `/releases/latest/download/` URL)

The updater endpoint in `tauri.conf.json` points at `https://github.com/Quvvy/Cutdown/releases/latest/download/latest.json`. The public signing key is embedded in the app; keep the private key secret and use it only when building release installers.

## Smoke Test

On a clean Windows profile or VM:

- Install from the NSIS installer with internet access and verify `%LOCALAPPDATA%\Cutdown\ffmpeg\ffmpeg.exe` exists afterward.
- On a machine where installer download was skipped, start the app with no ffmpeg on PATH and verify the **Install ffmpeg** banner works.
- Open a short MP4, split once, export with Lossless Trim, and reveal the output folder.
- Reopen the app and verify recent source/session restore behavior.

## Publish

- Create or update release notes with user-visible changes, fixes, and known limitations.
- Attach the NSIS installer artifact, its `.sig` file, and `latest.json` (see **In-app updater** above).
- Tag the release after the installer has passed the smoke test.

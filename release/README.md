# Release artifacts

Installer builds are written here locally and are not committed to git.

## v0.3.0

- **File:** `Cutdown_0.3.0_x64-setup.exe`
- **Build:** `npm run validate:release` then signed `npm run tauri -- build` (see [docs/RELEASE.md](../docs/RELEASE.md))
- **Notes:** Project-only persistence (`.cutdown`); timeline refactor; in-app updater from v0.2.4+.

## v0.2.4

- **File:** `Cutdown_0.2.4_x64-setup.exe`
- **Notes:** In-app updater, segment edge drag, startup fixes.

## v0.2.1

- **File:** `Cutdown_0.2.1_x64-setup.exe`
- **Notes:** NSIS installer downloads latest ffmpeg essentials during setup (internet required). WebView2 runtime must be installed separately.

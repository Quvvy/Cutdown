# Release artifacts

Installer builds are written here locally and are not committed to git.

## v0.2.1

- **File:** `Cutdown_0.2.1_x64-setup.exe`
- **Build:** `npm run validate:release` then `npm run tauri -- build`
- **Notes:** NSIS installer downloads latest ffmpeg essentials during setup (internet required). WebView2 runtime must be installed separately.

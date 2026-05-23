# Release artifacts

Installer builds are written here locally and are not committed to git.

## v0.2.3

- **File:** `Cutdown_0.2.3_x64-setup.exe`
- **Build:** `npm run validate:release` then `npm run tauri -- build`
- **Notes:** Fixes timeline layout (video/audio tracks fill the pane; drag resize works; no blank gap under tracks).

## v0.2.2

- **File:** `Cutdown_0.2.2_x64-setup.exe`
- **Build:** `npm run validate:release` then `npm run tauri -- build`
- **Notes:** Fixes startup hang when the watch-folder drive is not ready at login; optional start minimized to tray; fresh timeline when reopening videos (edits only via `.cutdown` projects); checkbox layout and Shift+X to quit.

## v0.2.1

- **File:** `Cutdown_0.2.1_x64-setup.exe`
- **Build:** `npm run validate:release` then `npm run tauri -- build`
- **Notes:** NSIS installer downloads latest ffmpeg essentials during setup (internet required). WebView2 runtime must be installed separately.

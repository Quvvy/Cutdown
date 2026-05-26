# Shipping a Windows build

## 1. Bump version

Same number in all three:

- `package.json`
- `src-tauri/Cargo.toml`
- `src-tauri/tauri.conf.json`

Skim README / PROGRESS if behavior changed.

## 2. Automated checks

```powershell
npm ci
npm run validate:release
```

That’s typecheck, frontend tests, Vite build, `cargo check`, `cargo test`, Clippy if installed. Not a substitute for the manual pass in TESTING.md.

## 3. Build installer

Unsigned (local only):

```powershell
npm run tauri -- build
```

Signed + updater artifacts (private key in `.tauri/cutdown.key`, gitignored):

```powershell
$env:TAURI_SIGNING_PRIVATE_KEY = Get-Content -Raw .tauri\cutdown.key
npm run tauri -- build
```

Output under `src-tauri/target/release/bundle/nsis/` (or the cargo target dir your machine uses). Copy into `release/` if you keep installers there.

ffmpeg is **not** inside the installer package. Post-install runs `Cutdown.exe --install-dependencies` to fetch essentials. Log: `%LOCALAPPDATA%\Cutdown\install-ffmpeg.log`.

## 4. Updater manifest

After a signed build:

```powershell
.\scripts\generate-latest-json.ps1 `
  -Version 0.3.0 `
  -InstallerPath release\Cutdown_0.3.0_x64-setup.exe `
  -SignaturePath release\Cutdown_0.3.0_x64-setup.exe.sig `
  -Notes "Short plain-text release note"
```

Upload to the GitHub release tag (e.g. `v0.3.0`):

| File | Why |
|------|-----|
| `Cutdown_*_x64-setup.exe` | NSIS only (not MSI) |
| `Cutdown_*_x64-setup.exe.sig` | Updater signature |
| `latest.json` | Exact name; app hits `/releases/latest/download/latest.json` |

Pubkey is in `tauri.conf.json`. Don’t commit the private key.

## 5. Quick install smoke

On a VM or spare profile if you can:

1. Install with internet → `%LOCALAPPDATA%\Cutdown\ffmpeg\ffmpeg.exe` exists.
2. Open MP4 → split → Lossless export → reveal folder.
3. Save `.cutdown`, reopen → segments/range/markers back.
4. Reopen same MP4 without project → fresh single segment.
5. Tray: close window, restore from icon, quit from menu.
6. If you have an older build installed: Settings → check for updates.

## 6. Publish

Write release notes for humans. Attach the three updater files. Tag after smoke passes.

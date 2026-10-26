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

## 3. Build, sign, and write latest.json

Run in a normal terminal (signing asks for the key password):

```powershell
.\scripts\build-release.ps1 -Notes "Short plain-text release note"
```

It runs `validate:release`, builds the NSIS installer **unsigned**, signs it as a separate step with `.tauri\cutdown.key`, refuses to continue if the signature's key ID doesn't match the `pubkey` in `tauri.conf.json`, then copies the installer + `.sig` into `release/` and writes `release/latest.json`. `-SkipValidate` / `-SkipBuild` re-run later steps (for example, to re-sign).

If there's no console to answer the password prompt it stops immediately instead of hanging.

ffmpeg is **not** inside the installer package. Post-install runs `Cutdown.exe --install-dependencies` to fetch essentials. Log: `%LOCALAPPDATA%\Cutdown\install-ffmpeg.log`.

### Signing key

| File (`.tauri/`, gitignored) | What it is |
|------|-----|
| `cutdown.key` | The updater key every shipped build trusts (key ID `99-39-06-CC-EF-F9-F0-19`). Never replace it. |
| `cutdown.key.pub` | Its public key, same as `plugins.updater.pubkey`. |
| `cutdown-unused-FAC75342.key` | A key generated in June 2026 that no release trusts. Don't sign with it. |

Losing `cutdown.key`, or its password, means installed copies can't auto-update. Keep a copy and the password somewhere off this machine.

To set or change its password without changing the key (needs `winget install jedisct1.minisign`):

```powershell
.\scripts\protect-signing-key.ps1
```

Switching to a different key takes two releases: first ship a build signed with the old key whose `pubkey` is the new one, then sign later releases with the new key.

## 4. Publish files

Upload to the GitHub release tag (e.g. `v0.4.0`):

| File | Why |
|------|-----|
| `Cutdown_*_x64-setup.exe` | NSIS only (not MSI) |
| `Cutdown_*_x64-setup.exe.sig` | Updater signature |
| `latest.json` | Exact name; app hits `/releases/latest/download/latest.json` |

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

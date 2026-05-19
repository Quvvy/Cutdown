# Cutdown Runtime Test Matrix

Use this checklist when validating a build before release or after editor/export changes.

## Prerequisites

- Windows 10/11 x64
- `ffmpeg` and `ffprobe` on PATH, or run `npm run prepare:ffmpeg` before `npm run tauri dev`
- Sample clips covering common OBS/replay-buffer cases

## Automated smoke (developer)

```powershell
npm run check
cargo check --manifest-path src-tauri/Cargo.toml
# Optional: invoke check_ffmpeg from the running app (Settings → status)
```

## Manual matrix

| Area | Case | Pass |
|------|------|------|
| Probe | MP4 H.264 + AAC | |
| Probe | MKV H.265 + Opus | |
| Probe | MOV | |
| Probe | No audio track | |
| Preview | Native WebView2 playback | |
| Preview | Remux fallback (container-only issue) | |
| Preview | Proxy fallback (codec issue) | |
| Edit | Split at playhead (`S`) | |
| Edit | Multi-segment delete | |
| Edit | I/O keep / trim outside / split I/O | |
| Edit | Undo / redo | |
| Export | Single segment stream-copy | |
| Export | Multi-segment concat | |
| Export | I/O range mode | |
| Export | Audio preserved (listen + ffprobe) | |
| Export | Progress bar shows percent on long proxy export | |
| Watch | New file in watch folder → toast → opens in editor | |

## Known limitations (not failures)

- Stream-copy trims may not be frame-perfect (keyframe boundaries).
- Preview may require remux/proxy for some codecs; export still uses the source file.

Log new failures in [PROGRESS.md](../PROGRESS.md) under **Known Issues**.

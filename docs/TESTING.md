# Cutdown Runtime Test Matrix

Use this checklist when validating a build before release or after editor/export changes.

## Prerequisites

- Windows 10/11 x64
- `ffmpeg` and `ffprobe` on PATH, or run `npm run prepare:ffmpeg` before `npm run tauri dev`
- Sample clips covering common OBS/replay-buffer cases

## Automated smoke (developer)

```powershell
npm run validate:release
```

This runs `npm run check`, `cargo check`, and reports bundled ffmpeg status.

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
| Export | Single segment stream-copy (Lossless Trim) | |
| Export | Multi-segment concat (Lossless Trim) | |
| Export | I/O range mode (Lossless Trim) | |
| Export | Discord preset under ~9 MB on 30–60s clip | |
| Export | Archive preset plays back correctly | |
| Export | Twitter preset at 720p max | |
| Export | Progress bar shows percent on re-encode | |
| Export | GPU encoder used when enabled (check Settings) | |
| Audio | Preserved on stream-copy export | |
| Watch | New file in watch folder → toast → opens in editor | |
| Windows | Open With launches Cutdown with file loaded | |
| Windows | Default export folder used in save dialog | |
| Windows | Run at startup toggle (optional) | |

## Known limitations (not failures)

- Stream-copy trims may not be frame-perfect (keyframe boundaries).
- Preview may require remux/proxy for some codecs; export still uses the source file.
- Discord size targeting is single-pass with up to two bitrate retries, not a full two-pass encode.

Log new failures in [PROGRESS.md](../PROGRESS.md) under **Known Issues**.

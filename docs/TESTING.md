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

This runs `npm run check`, `cargo check`, and reports bundled ffmpeg status. Run this before every release candidate; it satisfies the tooling portion of pre-release validation. Complete the manual matrix below on real OBS clips before tagging a release.

### Pre-release checklist (automated)

| Check | Command / tool | Pass |
|-------|----------------|------|
| TypeScript / Svelte | `npm run check` (via validate:release) | |
| Rust compile | `cargo check` (via validate:release) | |
| ffmpeg on PATH or bundled | validate:release output | |

Release installer build:

```powershell
npm run prepare:ffmpeg
npm run tauri -- build
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
| Preview | Fit preview to panel shows full frame above timeline | |
| Preview | Fit preview still correct after resizing workspace splitter | |
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
| Export | Default `*-cutdown.mp4` filename on Export open | |
| Export | Custom filename + choose folder (no full save path required) | |
| Export | Crop overlay 16:9 / 9:16 / free + cropped export matches preview | |
| Audio | Preserved on stream-copy export | |
| Watch | New file in watch folder → toast → opens in editor | |
| Windows | Open With launches Cutdown with file loaded | |
| Windows | Default export folder used in save dialog | |
| Windows | Run at startup toggle (optional) | |
| Tray | Close (X) hides window; app stays in tray | |
| Tray | Left-click tray icon restores main window | |
| Tray | Tray menu Open Editor / Quit works | |
| History | Export adds row; Reveal / Open / Copy path work | |
| History | Clear history removes all rows | |
| Upload | Catbox upload copies HTTPS link to clipboard | |
| Upload | File Garden sign-in + upload copies HTTPS link (account required) | |
| Upload | Custom HTTP multipart upload returns share URL | |
| Upload | Upload menu lists enabled providers; default provider used | |
| Settings | Add/edit/remove upload targets; set default provider | |
| UI | Escape closes Export / Settings / History | |
| UI | Backdrop click closes modals | |
| UI | `?` opens keyboard shortcuts modal | |
| UI | Drop video file onto window opens clip | |
| UI | Recent menu lists and opens prior sources | |
| Export | Accurate trim re-encodes boundaries on Lossless preset | |
| Export | Custom preset (bitrate / CRF / target size / lossless) from Settings | |
| Export | Strip audio exports video-only file | |
| Edit | J / K / L shuttle scrub (step back, pause, step forward) | |
| Edit | `[` / `]` snap playhead to I/O range markers | |
| Edit | Timeline toolbar: Split, In, Out, Clear range, Add marker, Snap In/Out, Split I/O, Delete | |
| Edit | `M` adds timeline bookmark; click bookmark seeks; right-click removes | |
| Edit | `,` / `.` previous / next marker; double-click or menu edits label | |
| Edit | `Del` deletes selected marker; `Shift + M` removes nearest at playhead | |
| Edit | Bookmarks persist when reopening same source / `.cutdown` project | |
| Edit | Audio track shows waveform envelope when clip has audio | |
| Edit | Timeline toolbar controls disabled without an open clip | |
| Edit | Ctrl+D duplicates selected segment | |
| Edit | Drag segment on timeline to reorder | |
| Session | Reopen same source restores segments, I/O, crop, volume, bookmarks | |
| Export | Batch per-segment export writes one file per kept segment | |
| Export | Queue upload after export runs uploads sequentially | |
| Export | Audio fade in/out on I/O or sequence export | |
| Project | Save / open `.cutdown` project restores editor state | |
| OBS | Latest replay opens newest file in watch folder | |
| OBS | Tray menu Open Watch Folder opens Explorer | |
| Preview | Playback speed 0.5× / 1× / 2× | |
| Preview | Proxy preview button builds proxy for heavy codecs | |
| Export | Trim hint explains stream-copy vs re-encode blockers | |

## Known limitations (not failures)

- Stream-copy trims may not be frame-perfect (keyframe boundaries).
- Preview may require remux/proxy for some codecs; export still uses the source file.
- Discord size targeting is single-pass with up to two bitrate retries, not a full two-pass encode.
- Crop with Lossless Trim preset forces a high-quality H.264 re-encode.

Log new failures in [PROGRESS.md](../PROGRESS.md) under **Known Issues**.

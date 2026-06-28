# Testing

Run this before tagging a release or after touching export, preview, timeline layout, or Windows integration.

## Setup

- Windows 10/11 x64
- ffmpeg: PATH, `public/ffmpeg/` (`npm run prepare:ffmpeg`), or post-install / in-app download
- A few real clips—H.264 MP4 from OBS, something HEVC, one with no audio if you have it

## Machines

`npm run validate:release` covers tooling. **Also** install the NSIS build and click through the app. WebView2 in the packaged app has behaved differently from Chrome in dev (timeline sizing especially).

```powershell
npm run validate:release
npm run tauri -- build   # or signed build per RELEASE.md
```

| Automated | |
|-----------|---|
| `npm run check` | |
| `npm test` | |
| `npm run build` | |
| `cargo check` / `cargo test` | |
| ffmpeg found (script output) | |

## Manual matrix

Check Pass when done. Skip rows that don’t apply.

| Area | Case | ✓ |
|------|------|---|
| **Probe** | MP4 H.264 + AAC | |
| | MKV HEVC | |
| | MOV | |
| | No audio | |
| **Preview** | Native playback | |
| | Remux fallback | |
| | Proxy fallback | |
| | Fit + resize workspace splitter | |
| | Speed 0.5× / 2× | |
| **Edit** | Split `S` | |
| | Delete segment (keep ≥1) | |
| | Undo / redo | |
| | I/O in/out, keep range, trim outside, split I/O | |
| | Duplicate `Ctrl+D`, reorder drag | |
| | Edge drag trim/extend | |
| | Bookmarks add/seek/edit/remove | |
| | `J` `K` `L`, `[` `]` snap to I/O | |
| | Waveform visible when audio present | |
| | Toolbar disabled with no clip open | |
| | Split outside segment → toast | |
| | Raw reopen = fresh; `.cutdown` = restore | |
| **Export** | Lossless single + multi-segment | |
| | I/O range export | |
| | Discord ~9 MB (30–60s) | |
| | Archive, Twitter presets | |
| | Progress % on re-encode | |
| | GPU when enabled in Settings | |
| | Crop matches preview | |
| | Strip audio, fades | |
| | Accurate trim | |
| | Custom preset from Settings | |
| | Batch per segment | |
| | Queue upload after export | |
| | Audio-only: MP3 192 default, WAV, OGG | |
| | Audio preset tab; `.mp4` → `.mp3` on switch | |
| | MP3 128 vs 192 bitrate | |
| | Post-export footer icons (copy, folder, upload) | |
| **Windows** | Open With | |
| | Default export folder | |
| | Run at startup (installed build) | |
| **Tray** | X hides; icon restores; Quit exits | |
| **Watch** | New file → toast → open | |
| **History** | Row after export; reveal/copy/clear confirm | |
| **Upload** | Catbox link | |
| | File Garden (signed in) | |
| | Custom HTTP | |
| **UI** | `?` help, Esc closes panels | |
| | Drop file to open | |
| | Recent sources | |
| | ffmpeg banner + in-app install | |
| | Relink missing project source | |
| **Updater** | Check for updates (older install) | |

## Not bugs (usually)

- Lossless cut lands on keyframes.
- Preview hiccup between non-adjacent source regions in a reordered timeline.
- Discord preset misses size on edge-case durations.
- Lossless + crop re-encodes.

New real bugs → note in PROGRESS.md under known issues.

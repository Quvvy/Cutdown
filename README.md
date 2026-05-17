# Cutdown

Fast desktop video trimming MVP built with Tauri v2, Svelte, TypeScript, and Rust.

## MVP Features

- Open local video files from the editor.
- Probe clip metadata through `ffprobe`.
- Preview video with an HTML `video` element.
- Set in/out points with timeline handles or keyboard shortcuts.
- Export a lossless stream-copy trim through `ffmpeg`.
- Use a Windows tray menu for `Open Editor` and `Quit`.

## Local Setup

Install the JavaScript and Rust toolchains, then install dependencies:

```powershell
npm install
```

Place Windows x64 `ffmpeg.exe` and `ffprobe.exe` in:

```text
public/ffmpeg/
```

Run the desktop app:

```powershell
npm run tauri dev
```

The backend falls back to `ffmpeg` and `ffprobe` from `PATH` if bundled binaries are not present.

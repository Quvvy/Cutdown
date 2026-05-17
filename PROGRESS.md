# Cutdown Progress

This file tracks implementation progress against the original Cutdown product prompt.

## Current Status

The project is at MVP baseline. The app scaffolds, type-checks, compiles, and builds with Tauri on Windows.

## Completed

- Initialized a Tauri v2, Svelte, TypeScript, and Rust project.
- Added plain CSS styling with a dark utility-focused interface.
- Added a Windows tray menu with `Open Editor` and `Quit`.
- Added a single-screen editor layout.
- Added HTML `video` preview with current timestamp overlay.
- Added timeline range controls for in/out trim points and seeking.
- Added keyboard shortcuts for `I`, `O`, `Space`, frame stepping, and 5-second stepping.
- Added an export modal for selecting the output path.
- Added Svelte editor state for current file, video metadata, trim points, and export status.
- Added Rust `probe_video` command using `ffprobe`.
- Added Rust `export_clip` command using ffmpeg stream copy.
- Added placeholder `detect_gpu_encoders` command.
- Added a required Tauri icon asset.
- Installed and validated local prerequisites: Node/npm, Rust/Cargo, WebView2, Visual Studio Build Tools, ffmpeg, and ffprobe.

## Validation

Last checked after restarting Cursor:

- `node --version` resolves.
- `npm --version` resolves.
- `rustc --version` resolves.
- `cargo --version` resolves.
- `ffmpeg` and `ffprobe` resolve.
- `npm run check` passes with 0 errors and 0 warnings.
- `cargo check` passes.

Previously validated:

- `npm run build` passes.
- `npm run tauri -- build` passes and produces a Windows NSIS bundle.

## Known Issues and Risks

- ffmpeg and ffprobe are installed on the system, but release bundling still needs static binaries in `public/ffmpeg/`.
- The MVP has not yet been runtime-tested with real sample clips in the UI.
- `npm audit --omit=dev` reports a moderate Svelte advisory. npm's automated fix requires a breaking Svelte 5 upgrade, so it is deferred until a planned framework update.
- The repo has no initial commit yet.
- Most files are currently untracked.

## Next Up

1. Bundle static Windows x64 `ffmpeg.exe` and `ffprobe.exe`.
2. Runtime-test probe and lossless export with mp4, mkv, and mov clips.
3. Add ffmpeg progress event parsing.
4. Improve export completion UX with open-in-explorer and toast actions.
5. Implement real GPU encoder detection.

## Roadmap Status

### Milestone 1: MVP Editor

Status: mostly complete.

Remaining:

- Runtime-test with real clips.
- Polish output path handling.
- Add progress events instead of one-shot export status.

### Milestone 2: ffmpeg Bundling and Export Hardening

Status: not started.

Planned:

- Bundle static Windows ffmpeg binaries.
- Add diagnostics for missing binaries.
- Parse ffmpeg progress.
- Add safer overwrite and output folder behavior.

### Milestone 3: Presets and Compression

Status: not started.

Planned:

- Add Discord, Lossless Trim, Archive, and Twitter/X presets.
- Add custom preset persistence.
- Add size-targeted two-pass encoding.
- Add GPU encoder preference and fallback.

### Milestone 4: Crop

Status: not started.

Planned:

- Add crop overlay.
- Add aspect ratio snapping.
- Add ffmpeg crop filter wiring.
- Force re-encode when crop is active.

### Milestone 5: Watch Folder Workflow

Status: not started.

Planned:

- Add watch folder setting.
- Monitor with Rust `notify`.
- Show Windows toast notifications for new clips.
- Open clicked clips directly in the editor.

### Milestone 6: Upload and Sharing

Status: not started.

Planned:

- Add upload host trait/module structure.
- Implement Catbox.moe.
- Implement Streamable with secure credential storage.
- Investigate FileGarden.

### Milestone 7: Clip History

Status: not started.

Planned:

- Store last 20 exports.
- Generate thumbnails.
- Add collapsible history drawer.
- Support reopen and copy-link actions.

### Milestone 8: Settings and Windows Integration

Status: not started.

Planned:

- Add minimal settings panel.
- Add startup registration.
- Add Open With/file association support.
- Document registry keys.

### Milestone 9: Performance Audit

Status: not started.

Planned:

- Measure startup and editor-open times.
- Measure preview readiness.
- Measure idle RAM.
- Measure final app and ffmpeg bundle sizes.

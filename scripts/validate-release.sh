#!/usr/bin/env bash
# Linux counterpart of validate-release.ps1: typecheck, tests, builds, clippy.
set -euo pipefail
cd "$(dirname "$0")/.."

run() {
  echo "Running $*..."
  "$@"
}

run npm run check
run npm test
run npm run build
run cargo check --manifest-path src-tauri/Cargo.toml
run cargo test --manifest-path src-tauri/Cargo.toml

if cargo clippy --version >/dev/null 2>&1; then
  run cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
else
  echo "cargo-clippy is not installed; skipping clippy."
fi

if command -v ffmpeg >/dev/null && command -v ffprobe >/dev/null; then
  echo "ffmpeg and ffprobe are on PATH."
else
  echo "ffmpeg/ffprobe are not on PATH. Install them: sudo apt install ffmpeg" >&2
fi

echo "Validation script finished. Complete the manual matrix in docs/TESTING.md."

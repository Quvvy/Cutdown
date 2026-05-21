$ErrorActionPreference = "Stop"
$projectRoot = Split-Path -Parent $PSScriptRoot

Set-Location $projectRoot

Write-Host "Running npm run check..."
npm run check

Write-Host "Running cargo check..."
cargo check --manifest-path src-tauri/Cargo.toml

if (Get-Command ffmpeg -ErrorAction SilentlyContinue) {
  Write-Host "ffmpeg is available on PATH."
} else {
  Write-Warning "ffmpeg is not on PATH. Run npm run prepare:ffmpeg before release builds."
}

$ffmpegBundled = Join-Path $projectRoot "public\ffmpeg\ffmpeg.exe"
if (Test-Path $ffmpegBundled) {
  Write-Host "Bundled ffmpeg found at $ffmpegBundled"
} else {
  Write-Warning "Bundled ffmpeg missing. Run: npm run prepare:ffmpeg"
}

$appIcon = Join-Path $projectRoot "branding\app-icon.png"
$bundleIcon = Join-Path $projectRoot "src-tauri\icons\icon.ico"
if ((Test-Path $appIcon) -and (Test-Path $bundleIcon)) {
  Write-Host "App icons found (source + bundle ICO)."
} else {
  Write-Warning "App icons missing. Run: npm run icons"
}

Write-Host "Validation script finished. Complete the manual matrix in docs/TESTING.md."

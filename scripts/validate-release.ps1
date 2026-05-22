$ErrorActionPreference = "Stop"
$projectRoot = Split-Path -Parent $PSScriptRoot

Set-Location $projectRoot

Write-Host "Running npm run check..."
npm run check

Write-Host "Running npm test..."
npm test

Write-Host "Running npm run build..."
npm run build

Write-Host "Running cargo check..."
cargo check --manifest-path src-tauri/Cargo.toml

Write-Host "Running cargo test..."
cargo test --manifest-path src-tauri/Cargo.toml

if (Get-Command cargo-clippy -ErrorAction SilentlyContinue) {
  Write-Host "Running cargo clippy..."
  cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
} else {
  Write-Host "cargo-clippy is not installed; skipping clippy."
}

if (Get-Command ffmpeg -ErrorAction SilentlyContinue) {
  Write-Host "ffmpeg is available on PATH (development / optional)."
} else {
  Write-Host "ffmpeg is not on PATH. Release builds no longer bundle ffmpeg; users download it from the app banner."
}

$ffmpegDev = Join-Path $projectRoot "public\ffmpeg\ffmpeg.exe"
if (Test-Path $ffmpegDev) {
  Write-Host "Development ffmpeg found at $ffmpegDev"
} else {
  Write-Host "No public/ffmpeg copy (optional for dev). Use PATH or npm run prepare:ffmpeg."
}

$appIcon = Join-Path $projectRoot "branding\app-icon.png"
$bundleIcon = Join-Path $projectRoot "src-tauri\icons\icon.ico"
if ((Test-Path $appIcon) -and (Test-Path $bundleIcon)) {
  Write-Host "App icons found (source + bundle ICO)."
} else {
  Write-Warning "App icons missing. Run: npm run icons"
}

Write-Host "Validation script finished. Complete the manual matrix in docs/TESTING.md."

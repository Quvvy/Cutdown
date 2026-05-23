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

$ffmpegReady = $false
if (Get-Command ffmpeg -ErrorAction SilentlyContinue) {
  if (Get-Command ffprobe -ErrorAction SilentlyContinue) {
    $ffmpegReady = $true
  }
}
$ffmpegDevDir = Join-Path $projectRoot "public\ffmpeg"
if (-not $ffmpegReady -and (Test-Path (Join-Path $ffmpegDevDir "ffmpeg.exe")) -and (Test-Path (Join-Path $ffmpegDevDir "ffprobe.exe"))) {
  $env:PATH = "$ffmpegDevDir;$env:PATH"
  $ffmpegReady = $true
  Write-Host "Using development ffmpeg from public/ffmpeg for integration tests."
}

if ($ffmpegReady) {
  Write-Host "Running ffmpeg integration test (integration_probe_and_lossless_export)..."
  cargo test --manifest-path src-tauri/Cargo.toml integration_probe_and_lossless_export -- --nocapture
} else {
  Write-Warning "ffmpeg/ffprobe not available; integration_probe_and_lossless_export will be skipped in cargo test."
}

if (Get-Command cargo-clippy -ErrorAction SilentlyContinue) {
  Write-Host "Running cargo clippy..."
  cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
  if ($LASTEXITCODE -ne 0) {
    throw "cargo clippy failed with exit code $LASTEXITCODE"
  }
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

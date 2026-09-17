$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $true
$projectRoot = Split-Path -Parent $PSScriptRoot

Set-Location $projectRoot

function Invoke-Checked {
  param(
    [Parameter(Mandatory = $true)]
    [scriptblock]$Command,
    [string]$Label
  )

  Write-Host $Label
  & $Command
  if ($LASTEXITCODE -ne 0) {
    throw "$Label failed with exit code $LASTEXITCODE"
  }
}

Invoke-Checked { npm run check } "Running npm run check..."
Invoke-Checked { npm test } "Running npm test..."
Invoke-Checked { npm run build } "Running npm run build..."
Invoke-Checked { cargo check --manifest-path src-tauri/Cargo.toml } "Running cargo check..."
Invoke-Checked { cargo test --manifest-path src-tauri/Cargo.toml } "Running cargo test..."

if (Get-Command cargo-clippy -ErrorAction SilentlyContinue) {
  Invoke-Checked {
    cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
  } "Running cargo clippy..."
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

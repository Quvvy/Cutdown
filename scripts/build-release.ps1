param(
  [Parameter(Mandatory = $true)]
  [string]$Notes,

  [switch]$SkipValidate,
  [switch]$SkipBuild
)

# Build the NSIS installer unsigned, then sign it as a separate step, check the
# signature against the app's updater pubkey, and write release\latest.json.
# Run in an interactive terminal: signing asks for the key password.

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "signing.ps1")
Set-Location $script:RepoRoot

$keyPath = ".tauri\cutdown.key"
if (-not (Test-Path $keyPath)) {
  throw "Signing key not found: $keyPath"
}

# Signing prompts for the password. With no console to answer it, the prompt
# waits forever, so stop now instead of after a full build.
if ([Console]::IsInputRedirected -and -not $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD) {
  throw "No interactive console for the key password prompt. Run this in a terminal, or set TAURI_SIGNING_PRIVATE_KEY_PASSWORD."
}

$version = (Get-Content "src-tauri\tauri.conf.json" -Raw | ConvertFrom-Json).version
$installerName = "Cutdown_${version}_x64-setup.exe"
Write-Host "Releasing $version"

if (-not $SkipValidate) {
  npm run validate:release
  if ($LASTEXITCODE -ne 0) { throw "validate:release failed" }
}

if (-not $SkipBuild) {
  # Updater artifacts off: tauri build would otherwise sign inline and prompt mid-build.
  Remove-Item Env:TAURI_SIGNING_PRIVATE_KEY -ErrorAction SilentlyContinue
  $overrides = Join-Path $env:TEMP "cutdown-build-overrides.json"
  [IO.File]::WriteAllText($overrides, '{"bundle":{"createUpdaterArtifacts":false}}')
  npm run tauri -- build --config $overrides
  if ($LASTEXITCODE -ne 0) { throw "tauri build failed" }
}

$installer = @("target\release\bundle\nsis\$installerName", "src-tauri\target\release\bundle\nsis\$installerName") |
  Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $installer) {
  throw "Installer not found: $installerName"
}

npx tauri signer sign -f $keyPath $installer
if ($LASTEXITCODE -ne 0) { throw "Signing failed" }
Assert-SignatureMatchesApp "$installer.sig"

Copy-Item $installer, "$installer.sig" release\ -Force
& (Join-Path $PSScriptRoot "generate-latest-json.ps1") `
  -Version $version `
  -InstallerPath "release\$installerName" `
  -SignaturePath "release\$installerName.sig" `
  -Notes $Notes `
  -OutPath "release\latest.json"

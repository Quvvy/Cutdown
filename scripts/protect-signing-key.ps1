# Set or change the password on .tauri\cutdown.key without changing the key itself,
# so installed copies keep trusting updates signed with it.
#
# Tauri's signer has no change-password command; minisign does. Tauri key files are
# base64-wrapped minisign keys, so unwrap, run `minisign -C`, and wrap again.
# Needs minisign on PATH: winget install jedisct1.minisign
# Run in an interactive terminal: minisign asks for the current and new passwords.

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "signing.ps1")

$keyPath = Join-Path $script:RepoRoot ".tauri\cutdown.key"
if (-not (Test-Path $keyPath)) {
  throw "Signing key not found: $keyPath"
}
if (-not (Get-Command minisign -ErrorAction SilentlyContinue)) {
  throw "minisign not found. Install it with: winget install jedisct1.minisign"
}

$backup = "$keyPath.bak-$(Get-Date -Format yyyyMMdd-HHmmss)"
Copy-Item $keyPath $backup
Write-Host "Backed up current key to $backup"

$plain = Join-Path $env:TEMP "cutdown-minisign-$([Guid]::NewGuid()).key"
try {
  $text = [Text.Encoding]::UTF8.GetString([Convert]::FromBase64String((Get-Content $keyPath -Raw).Trim()))
  [IO.File]::WriteAllText($plain, $text.TrimEnd() + "`n")

  Write-Host "Current password: press Enter if the key has none."
  minisign -C -s $plain
  if ($LASTEXITCODE -ne 0) { throw "minisign -C failed; key unchanged" }

  $wrapped = [Convert]::ToBase64String([IO.File]::ReadAllBytes($plain))
  [IO.File]::WriteAllText($keyPath, $wrapped)
} finally {
  Remove-Item $plain -Force -ErrorAction SilentlyContinue
}

Write-Host "Password updated. Test it before deleting the backup:"
Write-Host "  npx tauri signer sign -f .tauri\cutdown.key release\<installer>.exe"
Write-Host "Then store the password and a copy of the key somewhere off this machine."

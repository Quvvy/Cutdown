# Shared helpers for updater signing. Dot-source from release scripts.

$script:RepoRoot = Split-Path -Parent $PSScriptRoot

# Key ID (8 bytes, hex) from a Tauri pubkey or .sig string.
# Both are base64 of a minisign file whose second line is base64 of: 2-byte algorithm + 8-byte key ID + ...
function Get-MinisignKeyId([string]$TauriBase64) {
  $text = [Text.Encoding]::UTF8.GetString([Convert]::FromBase64String($TauriBase64.Trim()))
  $payload = [Convert]::FromBase64String(($text -split "`n")[1].Trim())
  return [BitConverter]::ToString($payload[2..9])
}

function Get-AppUpdaterPubkey {
  $conf = Get-Content (Join-Path $script:RepoRoot "src-tauri\tauri.conf.json") -Raw | ConvertFrom-Json
  return $conf.plugins.updater.pubkey
}

# Throws unless the .sig was made by the key the shipped app trusts.
function Assert-SignatureMatchesApp([string]$SignaturePath) {
  $expected = Get-MinisignKeyId (Get-AppUpdaterPubkey)
  $actual = Get-MinisignKeyId (Get-Content -LiteralPath $SignaturePath -Raw)
  if ($actual -ne $expected) {
    throw "Signature key $actual does not match the app's updater pubkey $expected. Installed copies would reject this update. Re-sign with .tauri\cutdown.key."
  }
  Write-Host "Signature key matches app pubkey ($expected)."
}

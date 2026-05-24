param(
  [Parameter(Mandatory = $true)]
  [string]$Version,

  [Parameter(Mandatory = $true)]
  [string]$InstallerPath,

  [Parameter(Mandatory = $true)]
  [string]$SignaturePath,

  [string]$Notes = "",
  [string]$TagPrefix = "v",
  [string]$Repo = "Quvvy/Cutdown",
  [string]$OutPath = ""
)

$ErrorActionPreference = "Stop"

if (-not (Test-Path $InstallerPath)) {
  throw "Installer not found: $InstallerPath"
}
if (-not (Test-Path $SignaturePath)) {
  throw "Signature file not found: $SignaturePath"
}

$installerName = Split-Path -Leaf $InstallerPath
$tag = "$TagPrefix$Version"
$downloadUrl = "https://github.com/$Repo/releases/download/$tag/$installerName"
$signature = (Get-Content -LiteralPath $SignaturePath -Raw).Trim()

$manifest = [ordered]@{
  version  = $Version
  notes    = $Notes
  pub_date = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
  platforms = [ordered]@{
    "windows-x86_64" = [ordered]@{
      signature = $signature
      url       = $downloadUrl
    }
  }
}

if (-not $OutPath) {
  $OutPath = Join-Path (Split-Path -Parent $InstallerPath) "latest.json"
}

$json = $manifest | ConvertTo-Json -Depth 6
[System.IO.File]::WriteAllText($OutPath, $json, [System.Text.UTF8Encoding]::new($false))
Write-Host "Wrote $OutPath"
Write-Host "Upload $OutPath and $installerName to GitHub release $tag"

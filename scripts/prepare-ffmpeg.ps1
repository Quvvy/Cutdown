$ErrorActionPreference = "Stop"

$projectRoot = Split-Path -Parent $PSScriptRoot
$targetDir = Join-Path $projectRoot "public\ffmpeg"
New-Item -ItemType Directory -Force $targetDir | Out-Null

$binaries = @("ffmpeg.exe", "ffprobe.exe")

foreach ($binary in $binaries) {
  $command = Get-Command $binary -ErrorAction Stop
  $target = Join-Path $targetDir $binary
  Copy-Item -Force $command.Source $target
  Write-Host "Copied $binary to $target"
}

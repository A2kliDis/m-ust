#!/usr/bin/env pwsh
# One-line install for m-ust (Windows):
#   irm https://raw.githubusercontent.com/A2kliDis/m-ust/master/install.ps1 | iex
$ErrorActionPreference = "Stop"
$Repo = "A2kliDis/m-ust"
$Name = "m-ust"
$BinDir = Join-Path $env:USERPROFILE ".cargo\bin"

if (-not (Test-Path $BinDir)) { New-Item -ItemType Directory -Path $BinDir | Out-Null }
$rel = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest" -TimeoutSec 20
$asset = @($rel.assets | Where-Object { $_.name -like "*windows*.zip" }) | Select-Object -First 1
if (-not $asset) { throw "release $($rel.tag_name) has no Windows asset" }
$tmp = Join-Path ([IO.Path]::GetTempPath()) "$Name-install"
if (Test-Path $tmp) { Remove-Item -Recurse -Force $tmp }
New-Item -ItemType Directory -Path $tmp | Out-Null
$zip = Join-Path $tmp "m-ust.zip"
Invoke-WebRequest -Uri $asset.browser_download_url -OutFile $zip -TimeoutSec 120
Expand-Archive -Path $zip -DestinationPath $tmp -Force
$exe = Get-ChildItem -Path $tmp -Filter "$Name.exe" -Recurse | Select-Object -First 1
if (-not $exe) { throw "archive contains no $Name.exe" }
Copy-Item $exe.FullName (Join-Path $BinDir "$Name.exe") -Force
Remove-Item -Recurse -Force $tmp
Write-Host "$Name $($rel.tag_name) installed to $BinDir"
& (Join-Path $BinDir "$Name.exe") --version

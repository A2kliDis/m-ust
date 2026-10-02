#!/usr/bin/env pwsh
# One-line install for m-ust (Windows):
#   irm https://raw.githubusercontent.com/A2kliDis/m-ust/main/install.ps1 | iex
#
# Tries a prebuilt binary from the latest GitHub Release first,
# falls back to `cargo install --git` (needs Rust + repo access).
$ErrorActionPreference = "Stop"
$Repo = "A2kliDis/m-ust"
$Name = "m-ust"
$BinDir = Join-Path $env:USERPROFILE ".cargo\bin"

function Install-ViaCargo {
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        throw "cargo not found. Install Rust from https://rustup.rs/ first."
    }
    & cargo install --git "https://github.com/$Repo" --bin $Name --locked
}

$installed = $false
try {
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
    $installed = $true
    Write-Host "$Name $($rel.tag_name) installed to $BinDir"
}
catch {
    Write-Warning "Prebuilt download failed: $($_.Exception.Message)"
}

if (-not $installed) {
    Write-Host "Falling back to 'cargo install --git'..."
    Install-ViaCargo
}

& (Join-Path $BinDir "$Name.exe") --version

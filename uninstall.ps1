#!/usr/bin/env pwsh
# Uninstall m-ust (Windows):
#   irm https://raw.githubusercontent.com/A2kliDis/m-ust/main/uninstall.ps1 | iex
# Add -Purge to also delete config + history:
#   irm https://raw.githubusercontent.com/A2kliDis/m-ust/main/uninstall.ps1 | iex -Purge
param([switch]$Purge)
$ErrorActionPreference = "Stop"
$Name = "m-ust"
$Exe = Join-Path (Join-Path $env:USERPROFILE ".cargo\bin") "$Name.exe"

if (Get-Process $Name -ErrorAction SilentlyContinue) {
    throw "$Name is running. Quit it (q) first, then rerun this script."
}
if (Test-Path $Exe) {
    Remove-Item $Exe -Force
    Write-Host "Removed $Exe"
} else {
    Write-Host "$Name is not installed."
}

$DataDir = Join-Path $env:APPDATA $Name
if ($Purge) {
    if (Test-Path $DataDir) {
        Remove-Item -Recurse -Force $DataDir
        Write-Host "Purged $DataDir (config + history)."
    }
} else {
    Write-Host "Kept $DataDir (config + history). Rerun with -Purge to delete it."
}

#!/usr/bin/env pwsh
# Uninstall m-ust (Windows):
#   irm https://raw.githubusercontent.com/A2kliDis/m-ust/master/uninstall.ps1 | iex
# Add -Purge to also delete config + history:
#   $s = irm https://raw.githubusercontent.com/A2kliDis/m-ust/master/uninstall.ps1; & ([scriptblock]::Create($s)) -Purge
param([switch]$Purge)
$ErrorActionPreference = "Stop"
$Name = "m-ust"
$InstallDir = Join-Path $env:LOCALAPPDATA $Name
$LegacyExe = Join-Path (Join-Path $env:USERPROFILE ".cargo\bin") "$Name.exe"

if (Get-Process $Name -ErrorAction SilentlyContinue) {
    throw "$Name is running. Quit it (q) first, then rerun this script."
}
$removed = $false
foreach ($exe in @((Join-Path $InstallDir "$Name.exe"), $LegacyExe)) {
    if (Test-Path $exe) {
        Remove-Item $exe -Force
        Write-Host "Removed $exe"
        $removed = $true
    }
}
if (-not $removed) { Write-Host "$Name is not installed." }

# Drop the install dir from user PATH (harmless if absent)
$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
$parts = @($userPath -split ";" | Where-Object { $_ -ne "" -and $_ -ne $InstallDir })
if ($parts.Count -ne (($userPath -split ";").Count)) {
    [Environment]::SetEnvironmentVariable("Path", ($parts -join ";"), "User")
    Write-Host "Removed $InstallDir from user PATH."
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

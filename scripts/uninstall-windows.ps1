# Uninstall Windows keys-layer (binary + logon task). Keeps config by default.
#
#   .\scripts\uninstall-windows.ps1
#   .\scripts\uninstall-windows.ps1 -PurgeConfig

[CmdletBinding()]
param(
    [switch]$PurgeConfig,
    [switch]$Help
)

$ErrorActionPreference = "Stop"

if ($Help) {
    Write-Host @"
Uninstall Windows keys-layer (binary + logon task). Keeps config by default.

  .\scripts\uninstall-windows.ps1
  .\scripts\uninstall-windows.ps1 -PurgeConfig
"@
    exit 0
}

if ($env:OS -ne "Windows_NT") {
    Write-Error "Windows-only"
    exit 1
}

$BinDir = Join-Path $env:LOCALAPPDATA "keys-layer"
$BinPath = Join-Path $BinDir "keys-layer.exe"
$ConfigDir = Join-Path $env:APPDATA "keys-layer"
$ConfigPath = Join-Path $ConfigDir "config.toml"
$TaskName = "keys-layer"

Write-Host "==> stopping keys-layer"
Stop-ScheduledTask -TaskName $TaskName -ErrorAction SilentlyContinue
Get-Process -Name "keys-layer" -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Milliseconds 300

Write-Host "==> removing logon task"
Unregister-ScheduledTask -TaskName $TaskName -Confirm:$false -ErrorAction SilentlyContinue

Write-Host "==> removing binary"
Remove-Item -Force -ErrorAction SilentlyContinue $BinPath
if (Test-Path $BinDir) {
    $left = Get-ChildItem $BinDir -Force -ErrorAction SilentlyContinue
    if (-not $left) {
        Remove-Item -Force $BinDir -ErrorAction SilentlyContinue
    }
}

if ($PurgeConfig) {
    Write-Host "==> removing config"
    Remove-Item -Force -ErrorAction SilentlyContinue $ConfigPath
    if (Test-Path $ConfigDir) {
        $left = Get-ChildItem $ConfigDir -Force -ErrorAction SilentlyContinue
        if (-not $left) {
            Remove-Item -Force $ConfigDir -ErrorAction SilentlyContinue
        }
    }
} else {
    Write-Host "==> keeping config at $ConfigPath"
}

Write-Host "==> done"

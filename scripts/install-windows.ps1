# Install keys-layer on Windows: build, config, optional logon autostart.
#
# Usage (from repo root, PowerShell):
#   .\scripts\install-windows.ps1              # binary + config + autostart + start now
#   .\scripts\install-windows.ps1 -NoAutostart # binary + config only
#   .\scripts\install-windows.ps1 -SkipBuild   # use existing target\release\keys-layer.exe
#
# See windows.md

[CmdletBinding()]
param(
    [switch]$NoAutostart,
    [switch]$SkipBuild,
    [switch]$Help
)

$ErrorActionPreference = "Stop"

if ($Help) {
    Write-Host @"
Install keys-layer on Windows: build, config, optional logon autostart.

  .\scripts\install-windows.ps1              # binary + config + autostart + start now
  .\scripts\install-windows.ps1 -NoAutostart # binary + config only
  .\scripts\install-windows.ps1 -SkipBuild   # use existing target\release\keys-layer.exe
"@
    exit 0
}

if ($env:OS -ne "Windows_NT") {
    Write-Error "This installer is Windows-only."
    exit 1
}

$Root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
Set-Location $Root

$BinDir = Join-Path $env:LOCALAPPDATA "keys-layer"
$BinPath = Join-Path $BinDir "keys-layer.exe"
$ConfigDir = Join-Path $env:APPDATA "keys-layer"
$ConfigPath = Join-Path $ConfigDir "config.toml"
$ExampleConfig = Join-Path $Root "config.example.toml"
$BuiltExe = Join-Path $Root "target\release\keys-layer.exe"
$TaskName = "keys-layer"

Write-Host "==> keys-layer Windows install"
Write-Host "    binary: $BinPath"
Write-Host "    config: $ConfigPath"
Write-Host ""

if (-not $SkipBuild) {
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        Write-Error "cargo not found. Install Rust from https://rustup.rs"
        exit 1
    }
    Write-Host "==> building release binary"
    cargo build --release -p keys-layer
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}

if (-not (Test-Path $BuiltExe)) {
    Write-Error "missing $BuiltExe — build first or omit -SkipBuild"
    exit 1
}

New-Item -ItemType Directory -Force -Path $BinDir | Out-Null

# Stop running instance so the exe can be replaced.
Get-Process -Name "keys-layer" -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Milliseconds 300

Copy-Item -Force $BuiltExe $BinPath
Write-Host "==> installed $BinPath"

New-Item -ItemType Directory -Force -Path $ConfigDir | Out-Null
if (Test-Path $ConfigPath) {
    Write-Host "==> keeping existing config"
} else {
    if (-not (Test-Path $ExampleConfig)) {
        Write-Error "missing $ExampleConfig"
        exit 1
    }
    Copy-Item $ExampleConfig $ConfigPath
    Write-Host "==> created $ConfigPath"
}

if (-not $NoAutostart) {
    Write-Host "==> registering logon task '$TaskName'"
    $action = New-ScheduledTaskAction -Execute $BinPath
    $trigger = New-ScheduledTaskTrigger -AtLogOn -User $env:USERNAME
    $settings = New-ScheduledTaskSettingsSet `
        -AllowStartIfOnBatteries `
        -DontStopIfGoingOnBatteries `
        -ExecutionTimeLimit ([TimeSpan]::Zero)
    Register-ScheduledTask `
        -TaskName $TaskName `
        -Action $action `
        -Trigger $trigger `
        -Settings $settings `
        -RunLevel Limited `
        -Force | Out-Null

    Write-Host "==> starting keys-layer"
    Start-ScheduledTask -TaskName $TaskName
} else {
    Write-Host "==> skipping autostart (-NoAutostart)"
    Write-Host "    start manually: & `"$BinPath`""
}

Write-Host ""
$LogPath = Join-Path $BinDir "keys-layer.log"
Write-Host "==> done"
Write-Host "    Edit config:  $ConfigPath  (hot-reloads on save)"
Write-Host "    Log file:     $LogPath"
Write-Host "    Uninstall:    .\scripts\uninstall-windows.ps1"
if (-not $NoAutostart) {
    Write-Host "    Stop:         Stop-ScheduledTask -TaskName $TaskName"
}

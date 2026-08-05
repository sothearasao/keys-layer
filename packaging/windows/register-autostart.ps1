# Register logon autostart + seed config for keys-layer (used by Inno Setup / CI installer).
#
#   .\register-autostart.ps1 -ExePath "...\keys-layer.exe" -ExampleConfig "...\config.example.toml"
#   .\register-autostart.ps1 -ExePath "..." -ExampleConfig "..." -NoAutostart

[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$ExePath,

    [string]$ExampleConfig = "",

    [string]$TaskName = "keys-layer",

    [switch]$NoAutostart
)

$ErrorActionPreference = "Stop"

if (-not (Test-Path -LiteralPath $ExePath)) {
    Write-Error "exe not found: $ExePath"
    exit 1
}

$ExePath = (Resolve-Path -LiteralPath $ExePath).Path
$ConfigDir = Join-Path $env:APPDATA "keys-layer"
$ConfigPath = Join-Path $ConfigDir "config.toml"

New-Item -ItemType Directory -Force -Path $ConfigDir | Out-Null
if (-not (Test-Path -LiteralPath $ConfigPath)) {
    if ($ExampleConfig -and (Test-Path -LiteralPath $ExampleConfig)) {
        Copy-Item -LiteralPath $ExampleConfig -Destination $ConfigPath
        Write-Host "created $ConfigPath"
    } else {
        Write-Warning "no config yet at $ConfigPath (and no example to copy)"
    }
} else {
    Write-Host "keeping existing config $ConfigPath"
}

if ($NoAutostart) {
    Write-Host "skipping autostart (-NoAutostart)"
    exit 0
}

Get-Process -Name "keys-layer" -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Milliseconds 300

$action = New-ScheduledTaskAction -Execute $ExePath
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

Start-ScheduledTask -TaskName $TaskName
Write-Host "registered and started task '$TaskName'"

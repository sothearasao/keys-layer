# Stop keys-layer and remove the logon Scheduled Task (used by Inno Setup uninstall).

[CmdletBinding()]
param(
    [string]$TaskName = "keys-layer"
)

$ErrorActionPreference = "SilentlyContinue"

Stop-ScheduledTask -TaskName $TaskName -ErrorAction SilentlyContinue
Get-Process -Name "keys-layer" -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Milliseconds 300
Unregister-ScheduledTask -TaskName $TaskName -Confirm:$false -ErrorAction SilentlyContinue

Write-Host "removed task '$TaskName' (config under %APPDATA%\keys-layer kept)"

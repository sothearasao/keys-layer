# Build a Windows setup EXE with Inno Setup (after a release cargo build).
#
# Usage (from repo root, on Windows):
#   .\scripts\build-windows-installer.ps1
#   .\scripts\build-windows-installer.ps1 -Version 0.1.3
#   .\scripts\build-windows-installer.ps1 -SkipBuild
#
# Requires: Rust (unless -SkipBuild), Inno Setup 6 (choco install innosetup)

[CmdletBinding()]
param(
    [string]$Version = "",
    [switch]$SkipBuild,
    [switch]$Help
)

$ErrorActionPreference = "Stop"

if ($Help) {
    Write-Host @"
Build keys-layer Windows installer (Inno Setup).

  .\scripts\build-windows-installer.ps1
  .\scripts\build-windows-installer.ps1 -Version 0.1.3
  .\scripts\build-windows-installer.ps1 -SkipBuild
"@
    exit 0
}

if ($env:OS -ne "Windows_NT") {
    Write-Error "Windows-only"
    exit 1
}

$Root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
Set-Location $Root

if (-not $Version) {
    $cargo = Get-Content (Join-Path $Root "Cargo.toml") -Raw
    if ($cargo -match 'version\s*=\s*"([^"]+)"') {
        $Version = $Matches[1]
    } else {
        $Version = "0.0.0"
    }
}

if (-not $SkipBuild) {
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        Write-Error "cargo not found"
        exit 1
    }
    Write-Host "==> cargo build --release -p keys-layer"
    cargo build --release -p keys-layer
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}

$Exe = Join-Path $Root "target\release\keys-layer.exe"
if (-not (Test-Path $Exe)) {
    Write-Error "missing $Exe"
    exit 1
}

$Iscc = $null
foreach ($candidate in @(
        "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe",
        "${env:ProgramFiles}\Inno Setup 6\ISCC.exe",
        "C:\Program Files (x86)\Inno Setup 6\ISCC.exe"
    )) {
    if (Test-Path $candidate) {
        $Iscc = $candidate
        break
    }
}
if (-not $Iscc) {
    $cmd = Get-Command iscc -ErrorAction SilentlyContinue
    if ($cmd) { $Iscc = $cmd.Source }
}
if (-not $Iscc) {
    Write-Error "Inno Setup 6 not found. Install with: choco install innosetup -y"
    exit 1
}

$Iss = Join-Path $Root "packaging\windows\keys-layer.iss"
$OutDir = Join-Path $Root "dist\windows"
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

Write-Host "==> ISCC /DMyAppVersion=$Version"
& $Iscc "/DMyAppVersion=$Version" $Iss
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

# Portable zip alongside the setup EXE
$PortableName = "keys-layer-$Version-windows-x64"
$Stage = Join-Path $OutDir $PortableName
Remove-Item -Recurse -Force -ErrorAction SilentlyContinue $Stage
New-Item -ItemType Directory -Force -Path $Stage | Out-Null
Copy-Item $Exe (Join-Path $Stage "keys-layer.exe")
Copy-Item (Join-Path $Root "config.example.toml") (Join-Path $Stage "config.example.toml")
Copy-Item (Join-Path $Root "packaging\windows\register-autostart.ps1") (Join-Path $Stage "register-autostart.ps1")
Copy-Item (Join-Path $Root "windows.md") (Join-Path $Stage "README-windows.md") -ErrorAction SilentlyContinue

$Zip = Join-Path $OutDir "$PortableName.zip"
Remove-Item -Force -ErrorAction SilentlyContinue $Zip
Compress-Archive -Path $Stage -DestinationPath $Zip -Force
Remove-Item -Recurse -Force $Stage

Write-Host ""
Write-Host "==> artifacts in $OutDir"
Get-ChildItem $OutDir | ForEach-Object { Write-Host "    $($_.Name)" }

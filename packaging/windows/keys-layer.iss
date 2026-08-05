; Inno Setup script for keys-layer (per-user, no admin).
;
; Build (from repo root, after cargo build --release -p keys-layer):
;   .\scripts\build-windows-installer.ps1
; Or:
;   iscc /DMyAppVersion=0.1.3 packaging\windows\keys-layer.iss
;
; See windows.md

#ifndef MyAppVersion
  #define MyAppVersion "0.1.0"
#endif

#define MyAppName "keys-layer"
#define MyAppPublisher "sothearasao"
#define MyAppURL "https://github.com/sothearasao/keys-layer"
#define MyAppExeName "keys-layer.exe"

[Setup]
AppId={{8F3C2A1B-6D4E-4F9A-9B21-E7C5A0D1F8B2}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppVerName={#MyAppName} {#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}/releases
DefaultDirName={localappdata}\keys-layer
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir=..\..\dist\windows
OutputBaseFilename=keys-layer-{#MyAppVersion}-windows-x64-setup
SetupIconFile=
Compression=lzma
SolidCompression=yes
WizardStyle=modern
UninstallDisplayIcon={app}\{#MyAppExeName}
InfoBeforeFile=
CloseApplications=force
RestartApplications=no
; Unsigned builds trip SmartScreen — expected until codesigning is added.

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "autostart"; Description: "Start keys-layer automatically at logon"; GroupDescription: "Autostart:"; Flags: checkedonce
Name: "desktopicon"; Description: "Create a desktop shortcut"; GroupDescription: "Additional icons:"; Flags: unchecked

[Files]
Source: "..\..\target\release\keys-layer.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\config.example.toml"; DestDir: "{app}"; DestName: "config.example.toml"; Flags: ignoreversion
Source: "register-autostart.ps1"; DestDir: "{app}"; Flags: ignoreversion
Source: "unregister-autostart.ps1"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"
Name: "{group}\Edit config folder"; Filename: "{userappdata}\keys-layer"
Name: "{group}\Uninstall {#MyAppName}"; Filename: "{uninstallexe}"
Name: "{userdesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Tasks: desktopicon

[Run]
; Seed config + Scheduled Task when autostart is selected.
Filename: "{sys}\WindowsPowerShell\v1.0\powershell.exe"; \
  Parameters: "-NoProfile -ExecutionPolicy Bypass -File ""{app}\register-autostart.ps1"" -ExePath ""{app}\{#MyAppExeName}"" -ExampleConfig ""{app}\config.example.toml"""; \
  StatusMsg: "Registering logon autostart..."; \
  Flags: runhidden waituntilterminated; \
  Tasks: autostart
; Seed config only when autostart was unchecked.
Filename: "{sys}\WindowsPowerShell\v1.0\powershell.exe"; \
  Parameters: "-NoProfile -ExecutionPolicy Bypass -File ""{app}\register-autostart.ps1"" -ExePath ""{app}\{#MyAppExeName}"" -ExampleConfig ""{app}\config.example.toml"" -NoAutostart"; \
  StatusMsg: "Creating config if missing..."; \
  Flags: runhidden waituntilterminated; \
  Tasks: not autostart
Filename: "{app}\{#MyAppExeName}"; \
  Description: "Launch keys-layer now"; \
  Flags: nowait postinstall skipifsilent unchecked; \
  Tasks: not autostart

[UninstallRun]
Filename: "{sys}\WindowsPowerShell\v1.0\powershell.exe"; \
  Parameters: "-NoProfile -ExecutionPolicy Bypass -File ""{app}\unregister-autostart.ps1"""; \
  Flags: runhidden waituntilterminated; \
  RunOnceId: "UnregisterKeysLayer"

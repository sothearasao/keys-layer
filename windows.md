# Windows

keys-layer on Windows uses a **low-level keyboard hook** (`WH_KEYBOARD_LL`) to intercept keys and **`SendInput`** to emit remaps. Same TOML config and engine as macOS/Linux.

No third-party driver install is required for this backend (same idea as Kanata’s default Windows build).

Related: [configuration](./configuration.md) · [linux.md](./linux.md)

---

## Build & run

On a Windows machine (PowerShell or cmd):

```powershell
git clone https://github.com/sothearasao/keys-layer.git
cd keys-layer
cargo build --release -p keys-layer

mkdir $env:APPDATA\keys-layer
copy config.example.toml $env:APPDATA\keys-layer\config.toml

.\target\release\keys-layer.exe
```

Or pass a config path:

```powershell
.\target\release\keys-layer.exe C:\path\to\config.toml
```

Default config search order:

1. `%APPDATA%\keys-layer\config.toml`
2. `%USERPROFILE%\.config\keys-layer\config.toml`

---

## Permissions / UAC

- Run as a normal interactive user (desktop session).
- Remaps may **not** apply inside some elevated admin windows or anti-cheat games (LLHOOK limitation).
- For those cases, a future **Interception driver** backend would be needed (not shipped yet).

---

## Config notes (Windows)

| Setting | Behavior |
|---------|----------|
| Hold layers / chords / sequences | Same as other platforms |
| Config hot-reload | Save the TOML file while running |
| `settings.devices` | **Ignored** (hook is system-wide) — warning printed at start |
| `f_row_media_devices` | If **non-empty**, enable F7–F12 → media/volume for all keyboards. F1–F6 stay F-keys (no brightness VKs). Set `[]` for raw F-keys |

```toml
[settings]
f_row_media_devices = ["on"]   # any non-empty list enables media F-row
# f_row_media_devices = []     # disable
```

---

## Autostart (optional)

Task Scheduler → create a task at logon:

- Program: `C:\path\to\keys-layer.exe`
- Arguments: (optional) path to `config.toml`
- Run only when user is logged on

Or drop a shortcut in the Startup folder.

---

## Troubleshooting

| Symptom | Fix |
|---------|-----|
| Hook install fails | Don’t run as a service/Session 0; use an interactive desktop |
| Remaps work in Notepad but not in an admin app | Expected for LLHOOK — run that app non-elevated, or wait for Interception backend |
| Keys echo / stuck | Restart keys-layer; avoid a second remapper (AutoHotkey, PowerToys Keyboard Manager) |
| Want only one keyboard remapped | Not supported on LLHOOK; use Linux/macOS device filter, or future Interception |

---

## vs other platforms

| | macOS | Linux | Windows (this) |
|--|--------|--------|----------------|
| Capture | Karabiner DriverKit | evdev grab | `WH_KEYBOARD_LL` |
| Emit | VirtualHID | uinput | `SendInput` |
| Extra driver | Karabiner VirtualHID | none (`uinput`) | none |
| Per-device filter | yes | yes | no |
| Elevated apps | works (root daemon) | works | often no |

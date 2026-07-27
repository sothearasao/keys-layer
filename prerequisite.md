# Prerequisites (macOS)

Do these **before** installing `keys-layer`.

Related: [install](./install.md) · [installation](./installation.md) · [autostart](./autostart.md) · [uninstall](./uninstall.md) · [configuration](./configuration.md) · [README](./README.md)

---

## What you need

| Item | Why |
|------|-----|
| macOS 11 (Big Sur) or newer | DriverKit VirtualHID |
| [Rust](https://rustup.rs) | Build from source |
| Xcode Command Line Tools | Compile the DriverKit client (`xcode-select --install`) |
| Karabiner VirtualHIDDevice | Seizes keyboards and injects remapped keys |

`keys-layer` must run as **root** (Karabiner IPC is root-only).

---

## 1. Install the VirtualHID driver

Pick **one** option.

### Option A — Karabiner-Elements (easiest)

1. Install [Karabiner-Elements](https://karabiner-elements.pqrs.org/).
2. Open it once so it activates the DriverKit extension.
3. Enable the extension:

   **System Settings → General → Login Items & Extensions → Driver Extensions**  
   → turn on `org.pqrs.Karabiner-DriverKit-VirtualHIDDevice`

KE will keep the VirtualHID **daemon** running for you.

### Option B — Standalone driver pkg

1. Download a release:  
   https://github.com/pqrs-org/Karabiner-DriverKit-VirtualHIDDevice/releases

   > This project uses `karabiner-driverkit` **0.3.x** (matches Karabiner-Elements ~6.x).  
   > Do **not** use crate **0.4.x** unless you install VirtualHIDDevice **v8.0.0**.

2. Activate:

```bash
sudo /Applications/.Karabiner-VirtualHIDDevice-Manager.app/Contents/MacOS/Karabiner-VirtualHIDDevice-Manager forceActivate
```

3. Enable the Driver Extension (same System Settings path as Option A).

4. Start the daemon (if KE is not managing it):

```bash
sudo "/Library/Application Support/org.pqrs/Karabiner-DriverKit-VirtualHIDDevice/Applications/Karabiner-VirtualHIDDevice-Daemon.app/Contents/MacOS/Karabiner-VirtualHIDDevice-Daemon" &
```

### Check the driver

```bash
ps aux | grep -i VirtualHIDDevice-Daemon | grep -v grep
systemextensionsctl list | grep -i pqrs
```

You should see the daemon process and an activated pqrs extension.

---

## 2. Privacy permissions

After you install `keys-layer` (see [installation](./installation.md) — usually `./scripts/install.sh`), add the **real binary** — not Terminal — to both lists:

**Path:** `/usr/local/bin/keys-layer`  
(older installs may use `~/.cargo/bin/keys-layer` — prefer the path `install.sh` printed)

1. **System Settings → Privacy & Security → Accessibility**
2. **System Settings → Privacy & Security → Input Monitoring**

Enable the toggle for `keys-layer`.

> After every reinstall that replaces the binary, macOS may treat it as new.  
> Remove it from both lists, add it again, or toggle off/on — then restart keys-layer.

If the log shows `IOHIDDeviceOpen error: … not permitted`, Input Monitoring is missing or stale.

---

## 3. Quit Karabiner-Elements remapping

Only **one** app can exclusively grab the keyboard.

You need the VirtualHID **daemon**, but you must **not** run Karabiner-Elements’ remapper at the same time.

If KE is remapping, keys-layer typically shows:

- `IOHIDDeviceOpen error: … not permitted` / exclusive access  
- or `DriverKit virtual keyboard not ready`

Stop KE’s grabber:

```bash
osascript -e 'quit app "Karabiner-Elements"'

launchctl bootout gui/$(id -u)/org.pqrs.service.agent.Karabiner-Core-Service-rev2 2>/dev/null
launchctl bootout gui/$(id -u)/org.pqrs.service.agent.karabiner_console_user_server 2>/dev/null
launchctl bootout gui/$(id -u)/org.pqrs.service.agent.Karabiner-Menu 2>/dev/null

# should print nothing:
ps aux | grep -i Karabiner-Core-Service | grep -v grep
```

Keep **Karabiner-VirtualHIDDevice-Daemon** running.

---

## Ready?

When the driver is active, permissions are granted, and KE remapping is quit, continue with **[installation.md](./installation.md)**.

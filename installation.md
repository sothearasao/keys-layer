# Installation (macOS)

`keys-layer` remaps keys through **Karabiner-DriverKit-VirtualHIDDevice**. That means you need the driver + daemon, privacy permissions, and to run the binary as **root**.

Related docs: [README](./README.md) · [configuration](./configuration.md) · [config.example.toml](./config.example.toml)

## Requirements

- macOS 11 (Big Sur) or newer
- Rust toolchain (to build from source): https://rustup.rs
- Xcode Command Line Tools (`xcode-select --install`) — needed to compile the DriverKit client
- Karabiner VirtualHIDDevice driver (via **Karabiner-Elements**, or the standalone pkg)

## 1. Install the VirtualHID driver

### Option A — Karabiner-Elements (simplest)

1. Install [Karabiner-Elements](https://karabiner-elements.pqrs.org/).
2. Open it once so it activates the DriverKit extension.
3. Confirm the extension is on:

   **System Settings → General → Login Items & Extensions → Driver Extensions**  
   Enable `org.pqrs.Karabiner-DriverKit-VirtualHIDDevice`.

KE will manage the VirtualHID **daemon** for you.

### Option B — Standalone driver pkg

Download a release from:

https://github.com/pqrs-org/Karabiner-DriverKit-VirtualHIDDevice/releases

> **Version note:** this project uses `karabiner-driverkit` **0.3.x**, which matches Karabiner-Elements’ bundled daemon (~6.x).  
> Do **not** use crate **0.4.x** unless you install standalone VirtualHIDDevice **v8.0.0** (protocol 7).

Activate:

```bash
sudo /Applications/.Karabiner-VirtualHIDDevice-Manager.app/Contents/MacOS/Karabiner-VirtualHIDDevice-Manager forceActivate
```

Enable the Driver Extension in System Settings (same path as above).

Start the daemon (if not using Karabiner-Elements):

```bash
sudo "/Library/Application Support/org.pqrs/Karabiner-DriverKit-VirtualHIDDevice/Applications/Karabiner-VirtualHIDDevice-Daemon.app/Contents/MacOS/Karabiner-VirtualHIDDevice-Daemon" &
```

Verify:

```bash
ps aux | grep -i VirtualHIDDevice-Daemon | grep -v grep
systemextensionsctl list | grep -i pqrs
```

## 2. Build and install `keys-layer`

From the repo:

```bash
cd /path/to/keys-layer
cargo install --path crates/keys-layer --force
```

Binary location:

```text
~/.cargo/bin/keys-layer
```

Ensure `~/.cargo/bin` is on your `PATH`.

### Config file

```bash
mkdir -p ~/.config/keys-layer
cp config.example.toml ~/.config/keys-layer/config.toml
# edit ~/.config/keys-layer/config.toml
```

If you run `keys-layer` with no arguments, it loads:

```text
~/.config/keys-layer/config.toml
```

## 3. Grant privacy permissions

Add **`~/.cargo/bin/keys-layer`** (the real binary, not Terminal) to:

1. **System Settings → Privacy & Security → Accessibility**
2. **System Settings → Privacy & Security → Input Monitoring**

After `cargo install --force`, macOS may treat the binary as new — toggle the permission off/on or re-add the binary.

## 4. Quit Karabiner-Elements remapping (important)

Only **one** process can exclusively grab the keyboard.

If Karabiner-Elements is remapping, `keys-layer` will fail with:

- `IOHIDDeviceOpen error: … exclusive access and device already open`
- sometimes `DriverKit virtual keyboard not ready (sink disconnected)`

Quit KE before starting keys-layer:

```bash
osascript -e 'quit app "Karabiner-Elements"'

# stop agents from respawning this login session
launchctl bootout gui/$(id -u)/org.pqrs.service.agent.Karabiner-Core-Service-rev2 2>/dev/null
launchctl bootout gui/$(id -u)/org.pqrs.service.agent.karabiner_console_user_server 2>/dev/null
launchctl bootout gui/$(id -u)/org.pqrs.service.agent.Karabiner-Menu 2>/dev/null

# confirm Core-Service is gone
ps aux | grep -i Karabiner-Core-Service | grep -v grep
```

Keep the **VirtualHIDDevice-Daemon** running (that one is required).

## 5. Run

Must use **sudo** (Karabiner IPC under `…/tmp/rootonly/` is root-only):

```bash
sudo keys-layer
# or
sudo keys-layer ~/.config/keys-layer/config.toml
```

Success looks like:

```text
keys-layer (DriverKit) running — /Users/…/.config/keys-layer/config.toml
Hold F / Caps for layers. Requires sudo + Karabiner VirtualHIDDevice.
Ctrl-C to quit.
```

## 6. Optional — start at login (LaunchDaemon)

Create `/Library/LaunchDaemons/local.keys-layer.plist` (edit paths to match your machine):

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>local.keys-layer</string>
  <key>ProgramArguments</key>
  <array>
    <string>/Users/YOUR_USER/.cargo/bin/keys-layer</string>
    <string>/Users/YOUR_USER/.config/keys-layer/config.toml</string>
  </array>
  <key>RunAtLoad</key>
  <true/>
  <key>KeepAlive</key>
  <true/>
  <key>StandardOutPath</key>
  <string>/var/log/keys-layer.log</string>
  <key>StandardErrorPath</key>
  <string>/var/log/keys-layer.log</string>
</dict>
</plist>
```

Install:

```bash
sudo cp local.keys-layer.plist /Library/LaunchDaemons/local.keys-layer.plist
sudo chown root:wheel /Library/LaunchDaemons/local.keys-layer.plist
sudo launchctl bootstrap system /Library/LaunchDaemons/local.keys-layer.plist
```

Reload after config changes:

```bash
sudo launchctl kickstart -k system/local.keys-layer
```

Uninstall:

```bash
sudo launchctl bootout system/local.keys-layer
sudo rm /Library/LaunchDaemons/local.keys-layer.plist
```

## Troubleshooting

### `connect_failed asio.system:2`

| Cause | Fix |
|-------|-----|
| Not running as root | Use `sudo keys-layer` |
| Daemon not running | Start VirtualHIDDevice-Daemon (or open Karabiner-Elements once) |
| Wrong driverkit crate vs daemon | Use `karabiner-driverkit` **0.3.x** with KE; **0.4.x** needs VHID **v8.0.0** |

### `exclusive access and device already open`

Karabiner-Elements (or another remapper) already grabbed the keyboard. Quit KE / kill Core-Service (see step 4).

### `DriverKit virtual keyboard not ready (sink disconnected)`

Daemon not ready or client couldn’t connect. Start/restart the daemon, use `sudo`, confirm driver extension is activated.

### `grab failed`

Re-add the **current** `keys-layer` binary to Accessibility + Input Monitoring (path-pinned grants break after reinstall).

### Caps Lock still toggles / layer sticks

Prefer `native = "disable"` on Caps. DriverKit gives real press/release; if something still feels sticky, confirm no second remapper is running.

### After `cargo install --force` nothing works

Re-grant Accessibility / Input Monitoring for the new binary, then:

```bash
sudo keys-layer
```

## Uninstall

```bash
# stop LaunchDaemon if you installed one
sudo launchctl bootout system/local.keys-layer 2>/dev/null
sudo rm -f /Library/LaunchDaemons/local.keys-layer.plist

# remove binary + config
rm -f ~/.cargo/bin/keys-layer
rm -rf ~/.config/keys-layer
```

Removing Karabiner-Elements / the DriverKit extension is optional and separate (use KE’s uninstaller or the VirtualHIDDevice manager `deactivate`).

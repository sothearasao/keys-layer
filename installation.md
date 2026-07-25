# Installation (macOS)

Install and run `keys-layer`.

**Before this page:** complete **[prerequisite.md](./prerequisite.md)** (driver, permissions, quit Karabiner remapping).

Also: [autostart](./autostart.md) · [uninstall](./uninstall.md) · [configuration](./configuration.md) · [README](./README.md)

---

## Quick path

```text
prerequisites → cargo install → config → sudo keys-layer
                 (optional) autostart.md for login
```

---

## 1. Build and install the binary

From the repo:

```bash
cd /path/to/keys-layer
cargo install --path crates/keys-layer --force
```

Installs to:

```text
~/.cargo/bin/keys-layer
```

Ensure `~/.cargo/bin` is on your `PATH`.

Then re-check **Accessibility** and **Input Monitoring** for that binary ([prerequisite.md](./prerequisite.md#2-privacy-permissions)).

---

## 2. Create a config

```bash
mkdir -p ~/.config/keys-layer
cp config.example.toml ~/.config/keys-layer/config.toml
```

Edit `~/.config/keys-layer/config.toml` as you like. Details: [configuration.md](./configuration.md).

With no arguments, `keys-layer` loads that default path.

---

## 3. Run (foreground)

Must use **sudo**:

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

If remaps do nothing, check the terminal (or `/var/log/keys-layer.log` if using autostart) for `not permitted` / exclusive access — see [Troubleshooting](#troubleshooting).

---

## 4. Optional — start at login

See **[autostart.md](./autostart.md)** (LaunchDaemon install, reload, stop).

---

## Troubleshooting

### Remaps don’t work but the process is “running”

Almost always: keyboards were **not seized**.

```bash
# LaunchDaemon log:
tail -40 /var/log/keys-layer.log
```

| Log line | Fix |
|----------|-----|
| `IOHIDDeviceOpen … not permitted` | Re-add binary to **Input Monitoring** (+ Accessibility); quit KE remapping ([prerequisite](./prerequisite.md)) |
| `exclusive access` / device already open | Quit Karabiner-Elements Core-Service ([prerequisite](./prerequisite.md#3-quit-karabiner-elements-remapping)) |
| `connect_failed asio.system:2` | Run as root / start VirtualHID daemon / check driverkit version |

### `connect_failed asio.system:2`

| Cause | Fix |
|-------|-----|
| Not root | `sudo keys-layer` (LaunchDaemon already runs as root) |
| Daemon not running | Start VirtualHIDDevice-Daemon or open KE once |
| Wrong crate vs daemon | `karabiner-driverkit` **0.3.x** with KE; **0.4.x** needs VHID **v8.0.0** |

### `DriverKit virtual keyboard not ready`

Daemon not ready or client couldn’t connect. Restart the daemon, use `sudo`, confirm the driver extension is activated.

### Caps Lock still toggles

Use `native = "disable"` on Caps in config. Confirm no second remapper is running.

### After `cargo install --force` nothing works

Re-grant Accessibility + Input Monitoring for `~/.cargo/bin/keys-layer`, then:

```bash
sudo keys-layer
# or, if using autostart:
sudo launchctl kickstart -k system/local.keys-layer
```

LaunchDaemon bootstrap / plist errors: [autostart.md](./autostart.md#autostart-troubleshooting).

---

## Next

- Autostart at login: [autostart.md](./autostart.md)  
- Config: [configuration.md](./configuration.md)  
- Remove everything: [uninstall.md](./uninstall.md)

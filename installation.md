# Installation (macOS)

Full install notes and troubleshooting.

**Quickstart:** **[quickstart.md](./quickstart.md)** · **[Homebrew](./homebrew.md)**  
**Before / driver detail:** [prerequisite.md](./prerequisite.md) — VirtualHID + Privacy permissions are **permanent** requirements on macOS.

Also: [autostart](./autostart.md) · [uninstall](./uninstall.md) · [configuration](./configuration.md) · [README](./README.md)

---

## Quick path (recommended)

From the repo:

```bash
./scripts/install.sh
```

That one command:

1. Builds a release binary → `/usr/local/bin/keys-layer`
2. Creates `~/.config/keys-layer/config.toml` if missing (from `config.example.toml`)
3. Installs the LaunchDaemon (`local.keys-layer`) so it starts at login
4. Tries to disable **Karabiner-Core-Service** (keeps the VirtualHID daemon)

Binary + config only (no daemon):

```bash
./scripts/install.sh --no-daemon
sudo /usr/local/bin/keys-layer ~/.config/keys-layer/config.toml
```

### After `install.sh` — still do these once

macOS will not automate them:

1. **Driver extension** — System Settings → General → Login Items & Extensions → Driver Extensions → enable VirtualHID  
2. **Privacy** — Accessibility **and** Input Monitoring for `/usr/local/bin/keys-layer`  
   (remove any old `~/.cargo/bin/keys-layer` entries)  
3. Confirm VirtualHID **daemon** is running; leave Karabiner-Elements remapping quit  

Verify:

```bash
tail -20 /var/log/keys-layer.log
# expect: keys-layer (DriverKit) running …
# no: IOHIDDeviceOpen … not permitted
```

### After every rebuild (`./scripts/install.sh`)

Config edits hot-reload — **no** rebuild needed for TOML changes.

Replacing the binary often makes macOS drop Input Monitoring even though the path is unchanged. If remaps stop or the log shows `not permitted`:

1. Re-toggle or remove/+ again **Accessibility** and **Input Monitoring** for `/usr/local/bin/keys-layer`  
2. Then:

```bash
sudo launchctl kickstart -k system/local.keys-layer
```

Or just run `./scripts/install.sh` again after fixing Privacy (it kickstarts the daemon).

Details: [configuration.md](./configuration.md#hot-reload).

---

## Manual install (optional)

If you prefer not to use the script:

```bash
cargo build --release -p keys-layer
sudo cp target/release/keys-layer /usr/local/bin/keys-layer

mkdir -p ~/.config/keys-layer
cp -n config.example.toml ~/.config/keys-layer/config.toml

# LaunchDaemon: see autostart.md
```

Or `cargo install --path crates/keys-layer --force` (binary under `~/.cargo/bin/`) — then point the plist at that path yourself.

---

## Troubleshooting

### Remaps don’t work but the process is “running”

Almost always: keyboards were **not seized**.

```bash
tail -40 /var/log/keys-layer.log
```

| Log line | Fix |
|----------|-----|
| `IOHIDDeviceOpen … not permitted` | Re-add **`/usr/local/bin/keys-layer`** to Input Monitoring (+ Accessibility); quit KE remapping ([prerequisite](./prerequisite.md)) |
| `exclusive access` / device already open | Quit Karabiner-Elements Core-Service ([prerequisite](./prerequisite.md#3-quit-karabiner-elements-remapping)) |
| `connect_failed asio.system:2` | Run as root / start VirtualHID daemon / check driverkit version |

### `connect_failed asio.system:2`

| Cause | Fix |
|-------|-----|
| Not root | LaunchDaemon runs as root; foreground needs `sudo` |
| Daemon not running | Start VirtualHIDDevice-Daemon or open KE once |
| Wrong crate vs daemon | `karabiner-driverkit` **0.3.x** with KE; **0.4.x** needs VHID **v8.0.0** |

### `DriverKit virtual keyboard not ready`

Daemon not ready or client couldn’t connect. Restart the daemon, confirm the driver extension is activated.

### Caps Lock still toggles

Use `native = "disable"` on Caps in config. Confirm no second remapper is running.

### After reinstall nothing works

Replacing `/usr/local/bin/keys-layer` often invalidates TCC even at the same path. Re-toggle or remove/+ **Accessibility** and **Input Monitoring** for that binary, then:

```bash
sudo launchctl kickstart -k system/local.keys-layer
```

LaunchDaemon bootstrap / plist errors: [autostart.md](./autostart.md#autostart-troubleshooting).

---

## Next

- Config: [configuration.md](./configuration.md)  
- Autostart details: [autostart.md](./autostart.md)  
- Remove everything: `./scripts/uninstall.sh` or [uninstall.md](./uninstall.md)

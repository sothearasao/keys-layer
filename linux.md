# Linux

keys-layer on Linux grabs physical keyboards with **evdev** (`EVIOCGRAB`) and emits remapped keys through a **uinput** virtual keyboard. The same TOML config and engine as macOS.

Related: [quickstart](./quickstart.md) · [configuration](./configuration.md)

---

## Quick install

```bash
./scripts/install-linux.sh --user-systemd
# log out/in if you were added to group `input`
```

This builds a release binary into `~/.local/bin/keys-layer`, creates config if missing, installs a **udev** rule for `/dev/uinput`, and enables a **systemd --user** service.

Other options:

```bash
./scripts/install-linux.sh                 # binary + config + udev only
./scripts/install-linux.sh --system-systemd
./scripts/uninstall-linux.sh
./scripts/uninstall-linux.sh --purge-config
```

---

## Requirements

| Need | Why |
|------|-----|
| Read `/dev/input/event*` | See keyboards |
| `EVIOCGRAB` | Exclusive grab (`input` group or root) |
| Write `/dev/uinput` | Virtual keyboard (udev rule ships with the installer) |

```bash
sudo usermod -aG input "$USER"   # then re-login
sudo modprobe uinput             # some distros
```

---

## Manual run

```bash
cargo build --release -p keys-layer
mkdir -p ~/.config/keys-layer
cp -n config.example.toml ~/.config/keys-layer/config.toml
./target/release/keys-layer
```

---

## Features (Linux)

| Feature | Behavior |
|---------|----------|
| Hold layers / chords / sequences | Same engine as macOS |
| Config hot-reload | Save TOML or `kill -HUP <pid>` |
| **Hot-plug** | New matching keyboards seized within ~2s |
| **F-row media** | Devices in `f_row_media_devices` map F1–F12 → brightness / media / volume (F3/F4 stay F-keys). Hold `KEY_FN` for real F-keys when the board sends it. Default example list is `["Apple Internal"]` — set your laptop name substring, or `[]` to disable |
| **Caps Lock LED** | Toggled on the physical keyboard via `EV_LED` when Caps is emitted |

```toml
[settings]
devices = []                          # all keyboards
f_row_media_devices = ["AT Translated"]  # example laptop controller name
# f_row_media_devices = []            # raw F-keys everywhere
```

---

## Autostart

Prefer the installer flags. Manual user unit template: [`packaging/keys-layer.user.service.in`](./packaging/keys-layer.user.service.in).

```bash
systemctl --user status keys-layer
journalctl --user -u keys-layer -f
```

---

## Troubleshooting

| Symptom | Fix |
|---------|-----|
| `open /dev/uinput failed` | `modprobe uinput`; re-run installer udev; re-login for `input` group |
| `no keyboards grabbed` | `ls -l /dev/input/event*`; check group; `devices = []` |
| Keys dead after start | Check journal; unplug/replug; ensure only one remapper grabs |
| Hot-plug ignored | Name must match `devices` (if set); wait ~2s; look for `hotplug:` in logs |
| Want real F-keys on laptop | `f_row_media_devices = []` |
| Caps LED dark | Board may not support `LED_CAPSL` over the grabbed node — key still toggles in software |

---

## vs macOS

| | macOS | Linux |
|--|--------|--------|
| Capture | Karabiner DriverKit | evdev grab |
| Emit | VirtualHID | uinput |
| Privileges | root + Accessibility / Input Monitoring | `input` group + uinput |
| F-row media | Fn/Globe + System Settings | `KEY_FN` when present; else media-by-default for listed devices |
| Caps LED | IOHID | `EV_LED` on physical device |
| Install | Homebrew / `install.sh` | `install-linux.sh` + systemd |

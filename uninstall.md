# Uninstall (macOS)

Remove `keys-layer` from your Mac. You do **not** need to reboot.

Related: [prerequisite](./prerequisite.md) · [installation](./installation.md) · [autostart](./autostart.md) · [README](./README.md)

---

## Script (recommended)

```bash
./scripts/uninstall.sh           # stop daemon, remove binary; keep config
./scripts/uninstall.sh --purge   # also delete ~/.config/keys-layer
```

Removes:

- LaunchDaemon `local.keys-layer`
- `/usr/local/bin/keys-layer` (and `~/.cargo/bin/keys-layer` if present)

Does **not** remove Karabiner VirtualHID (shared with other tools).

---

## Manual steps

### 1. Stop the LaunchDaemon

```bash
sudo launchctl bootout system/local.keys-layer 2>/dev/null
sudo rm -f /Library/LaunchDaemons/local.keys-layer.plist
```

### 2. Remove the binary (and optionally config)

```bash
sudo rm -f /usr/local/bin/keys-layer
rm -f ~/.cargo/bin/keys-layer
# optional:
rm -rf ~/.config/keys-layer
```

Optional — remove privacy entries:

**System Settings → Privacy & Security → Accessibility** and **Input Monitoring**  
→ select `keys-layer` → **−**

### 3. Optional — remove Karabiner VirtualHID

Only if you no longer need it for anything else (Karabiner-Elements, kanata, etc.).

- **Karabiner-Elements:** use its uninstaller from the app menu.  
- **Standalone VirtualHIDDevice:**

```bash
sudo /Applications/.Karabiner-VirtualHIDDevice-Manager.app/Contents/MacOS/Karabiner-VirtualHIDDevice-Manager deactivate
```

---

## Reinstall later

1. [prerequisite.md](./prerequisite.md)  
2. `./scripts/install.sh`

# Uninstall (macOS)

Remove `keys-layer` from your Mac. You do **not** need to reboot.

Related: [prerequisite](./prerequisite.md) · [installation](./installation.md) · [autostart](./autostart.md) · [README](./README.md)

---

## 1. Stop the LaunchDaemon (if you installed one)

```bash
sudo launchctl bootout system/local.keys-layer 2>/dev/null
sudo rm -f /Library/LaunchDaemons/local.keys-layer.plist
```

Confirm it is gone:

```bash
sudo launchctl print system/local.keys-layer 2>&1 | head -5
# expect an error / not found
```

If you were running it in a terminal instead, press **Ctrl-C**.

---

## 2. Remove the binary and config

```bash
rm -f ~/.cargo/bin/keys-layer
rm -rf ~/.config/keys-layer
```

Optional — remove privacy entries:

**System Settings → Privacy & Security → Accessibility** and **Input Monitoring**  
→ select `keys-layer` → **−**

---

## 3. Optional — remove Karabiner VirtualHID

Only if you no longer need it for anything else (Karabiner-Elements, kanata, etc.).

- **Karabiner-Elements:** use its uninstaller / “Uninstall Karabiner-Elements” from the app menu, or remove the app and follow KE’s docs.  
- **Standalone VirtualHIDDevice:**

```bash
sudo /Applications/.Karabiner-VirtualHIDDevice-Manager.app/Contents/MacOS/Karabiner-VirtualHIDDevice-Manager deactivate
```

Then remove the manager app / pkg leftovers if you want a full cleanup.

Leaving the driver installed is fine if you might reinstall `keys-layer` later.

---

## Reinstall later

1. [prerequisite.md](./prerequisite.md)  
2. [installation.md](./installation.md)

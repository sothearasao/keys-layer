# Menu bar toggle (macOS)

Remapping is **off by default** (does not start at login/reboot). Use **Keys Layer.app** in `/Applications` to turn it on or off. While On, it **stays on across sleep/wake** (recovery retries; LaunchDaemon KeepAlive restarts if the process exits).

Related: [homebrew](./homebrew.md) · [installation](./installation.md) · [prerequisite](./prerequisite.md)

---

## Install (Homebrew — simplest)

```bash
brew install --HEAD sothearasao/keys-layer/keys-layer
```

That puts **Keys Layer** in **/Applications**. Then:

1. Open **Keys Layer** from Applications / Spotlight  
2. **Open Privacy Settings…** (or grant manually) for  
   `$(brew --prefix)/opt/keys-layer/bin/keys-layer`  
   — Accessibility **and** Input Monitoring (**not** the menu app)  
3. **Turn Remapping On** (password once)

First On creates config + LaunchDaemon. If Privacy was granted while remapping was already running, the app **restarts remapping automatically** (or use **Fix Privacy / Restart Remapping**).

You do **not** need `keys-layer-setup` for the normal path.

### From source

```bash
./scripts/install.sh   # → /Applications/Keys Layer.app
open "/Applications/Keys Layer.app"
```

---

## Use

| Item | Action |
|------|--------|
| **Turn Remapping On** | Starts remapping (admin once; first time seeds config + plist) |
| **Turn Remapping Off** | Stops remapping |
| **Keyboards** | Panel with switches for each board. **On** = remapped input. **Off** = keyboard muted completely (no keys reach macOS). Writes `disabled_devices` in config. **Scan** refreshes the live list. |
| **Launch Menu at Login** | Menu icon at login only — remaps stay off until you turn them on |
| **Open Config** / **Open Log** | `~/.config/keys-layer/config.toml` / `/var/log/keys-layer.log` |

Disabling a keyboard writes `settings.devices` and restarts remapping so that board is released. Enabling adds it back to the allowlist.

```bash
keys-layer-ctl status
sudo keys-layer-ctl on
sudo keys-layer-ctl off
sudo keys-layer-ctl devices
sudo keys-layer-ctl device-disable "Apple Internal Keyboard / Trackpad"
sudo keys-layer-ctl device-enable "Apple Internal"
```

---

## Notes

- Keep VirtualHID driver + daemon healthy ([prerequisite](./prerequisite.md)).
- Emergency: **Turn Remapping Off** or `keys-layer-emergency-stop`.

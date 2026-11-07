# Menu bar toggle (macOS)

Remapping is **owned by the menu app**. It is **off by default** (does not start at login/reboot). **Turn On** starts remapping; **Quit & Stop Remapping** (or closing the menu) stops it — remapping does not keep running in the background after quit. Sleep/wake recovery stays in-process while the menu is open.

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
| **Turn On** | Starts remapping (admin once; first time seeds config + plist) |
| **Turn Off** | Stops remapping |
| **Quit & Stop Remapping** | Quits the menu **and** stops the remapper (heartbeat) |
| **Connected** | Lists **currently plugged-in** keyboards only (not your config allowlist). Toggle **Off** to mute that board completely. Writes `disabled_devices` — **no admin password**. `settings.devices` in config still controls which boards get remapped. |
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

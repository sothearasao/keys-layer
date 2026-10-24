# Homebrew (macOS)

Install `keys-layer` with Homebrew. The menu bar app lands in **`/Applications`** like a normal Mac app.

Related: [quickstart](./quickstart.md) · [prerequisite](./prerequisite.md) · [menubar](./menubar.md) · [installation](./installation.md)

---

## Install

```bash
brew tap sothearasao/keys-layer https://github.com/sothearasao/keys-layer.git
brew trust sothearasao/keys-layer
brew install --HEAD sothearasao/keys-layer/keys-layer
```

Homebrew builds the remapper + **Keys Layer.app**, then copies the app to:

`/Applications/Keys Layer.app`

Stable (versioned) installs are also available once the formula is bumped; prefer `--HEAD` for the latest menu-bar flow.

---

## First-time use (simple)

1. **VirtualHID** — enable the DriverKit extension ([prerequisite.md](./prerequisite.md)).  
   If needed: `./scripts/setup-virtualhid.sh` from a clone, or Karabiner-Elements once.
2. **Privacy** — Accessibility **and** Input Monitoring for:
   `$(brew --prefix)/opt/keys-layer/bin/keys-layer`
3. Open **Keys Layer** from **/Applications** (or Spotlight).
4. Menu bar → **Turn Remapping On** (macOS password once).  
   First On creates `~/.config/keys-layer/config.toml` and the LaunchDaemon plist.

Remapping stays **off** across reboot until you turn it on again. Optional: **Launch Menu at Login** keeps the icon only.

CLI:

```bash
keys-layer-ctl status
sudo keys-layer-ctl on
sudo keys-layer-ctl off
```

---

## Upgrade

```bash
brew upgrade --fetch-HEAD keys-layer
# App in /Applications is refreshed on install/upgrade via post_install
```

Re-check Input Monitoring if the log shows `not permitted` after an upgrade.

---

## Troubleshooting

| Symptom | Fix |
|---------|-----|
| `unable to run rust-objcopy` during `brew install` | Fixed in the formula (`CARGO_PROFILE_RELEASE_STRIP=none`). Update the tap / reinstall `--HEAD`. Or: `export CARGO_PROFILE_RELEASE_STRIP=none` and use `./scripts/install.sh`. |
| `A full installation of Xcode.app is required` | Fixed: formula no longer requires Xcode.app. Command Line Tools + `swift` are enough for the menu bar. `brew update` then `brew install --HEAD …` again. |
| `sandbox-exec: sandbox_apply: Operation not permitted` while building menu bar | Fixed: menu bar builds with `swiftc` (no SwiftPM). `brew update` then `brew install --HEAD …` again. |
| Key left of `1` types `§`/`±` instead of `` ` ``/`~` | Fixed in **v0.1.6+** (`iso_grave_swap` off by default). `brew update && brew upgrade keys-layer`, then toggle Off/On. If you still need the old swap: `iso_grave_swap = true` in config. |
| Keyboard freezes every few seconds after macOS upgrade; log shows `connect_failed asio.system:2` | VirtualHID **daemon** not running. `keys-layer-emergency-stop`, then `./scripts/setup-virtualhid.sh --no-pkg`, re-enable Driver Extensions, Turn Remapping On. |
| `keys-layer-setup: command not found` | Install failed — fix the build first, then `brew install …` again |
| Cursor stuck / keyboard dies with BT mouse | Set `devices = ["Apple Internal"]` (see [configuration.md](./configuration.md#listing-devices)); upgrade past the seize filter fix |
| Keyboard dead after sleep / remapper wedged | **Emergency (mouse OK):** `keys-layer-emergency-stop` or menu **Turn Remapping Off**. |

### Keyboard dead right now

```bash
sudo launchctl bootout system/local.keys-layer
sudo pkill -9 keys-layer
```

---

## Uninstall

```bash
keys-layer-emergency-stop 2>/dev/null || true
brew uninstall keys-layer
rm -rf "/Applications/Keys Layer.app"
# optional: brew untap sothearasao/keys-layer
```

Homebrew removes the Cellar copy; delete `/Applications/Keys Layer.app` yourself (or leave it — it will call missing brew paths).

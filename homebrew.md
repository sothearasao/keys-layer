# Homebrew (macOS)

Install `keys-layer` with Homebrew, then finish VirtualHID + Privacy + daemon setup.

Related: [quickstart](./quickstart.md) · [prerequisite](./prerequisite.md) · [installation](./installation.md)

---

## Install

Homebrew 6+ requires trusting third-party taps (custom remotes must trust the **whole** tap):

```bash
brew tap sothearasao/keys-layer https://github.com/sothearasao/keys-layer.git
brew trust sothearasao/keys-layer
brew install sothearasao/keys-layer/keys-layer
keys-layer-setup
```

Stable installs **v0.1.6** (includes the §/± grave-key fix). For the latest `main` branch instead:

```bash
brew install --HEAD sothearasao/keys-layer/keys-layer
```

After install, see notes with:

```bash
brew info sothearasao/keys-layer/keys-layer
```

---

## Finish setup

1. **VirtualHID** — enable the DriverKit extension ([prerequisite.md](./prerequisite.md)).
2. **Config + LaunchDaemon plist** (remapping stays **off** until you turn it on):

```bash
keys-layer-setup
```

3. **Privacy** — Accessibility + Input Monitoring for:
   `$(brew --prefix)/opt/keys-layer/bin/keys-layer`  
   (also shown under **Caveats** in `brew info sothearasao/keys-layer/keys-layer`)

4. **Menu bar** — only way remapping starts (not at login). Open the app, then **Turn Remapping On** (password once; applies TCC to a new process):

```bash
keys-layer-menu
# or: open "$(brew --prefix)/opt/keys-layer/Keys Layer.app"
# CLI: keys-layer-ctl on|off|status
```

See [menubar.md](./menubar.md).

Config only (no daemon):

```bash
keys-layer-setup --no-daemon
sudo "$(brew --prefix)/opt/keys-layer/bin/keys-layer"
```

Verify:

```bash
tail -20 /var/log/keys-layer.log
```

---

## Upgrade

```bash
brew upgrade keys-layer                # stable
# or: brew upgrade --fetch-HEAD keys-layer   # tracking main
keys-layer-setup                       # refresh LaunchDaemon paths if needed
```

Re-check Input Monitoring if the log shows `not permitted` after an upgrade.

---

## Troubleshooting

| Symptom | Fix |
|---------|-----|
| `unable to run rust-objcopy` during `brew install` | Fixed in the formula (`CARGO_PROFILE_RELEASE_STRIP=none`). Update the tap / reinstall `--HEAD`. Or: `export CARGO_PROFILE_RELEASE_STRIP=none` and use `./scripts/install.sh`. |
| `A full installation of Xcode.app is required` | Fixed: formula no longer requires Xcode.app. Command Line Tools + `swift` are enough for the menu bar. `brew update` then `brew install --HEAD …` again. |
| Key left of `1` types `§`/`±` instead of `` ` ``/`~` | Fixed in **v0.1.6+** (`iso_grave_swap` off by default). `brew update && brew upgrade keys-layer`, then `sudo launchctl kickstart -k system/local.keys-layer`. If you still need the old swap: `iso_grave_swap = true` in config. |
| Keyboard freezes every few seconds after macOS upgrade; log shows `connect_failed asio.system:2` | VirtualHID **daemon** not running. `keys-layer-emergency-stop`, then `./scripts/setup-virtualhid.sh --no-pkg`, re-enable Driver Extensions, `keys-layer-setup`. |
| `keys-layer-setup: command not found` | Install failed — fix the build first, then `brew install …` again |
| Cursor stuck / keyboard dies with BT mouse | Set `devices = ["Apple Internal"]` (see [configuration.md](./configuration.md#listing-devices)); upgrade past the seize filter fix |
| Keyboard dead after sleep / remapper wedged | **Emergency (mouse OK):** `keys-layer-emergency-stop` or `sudo launchctl bootout system/local.keys-layer && sudo pkill -9 keys-layer`. Then upgrade `--HEAD` and `keys-layer-setup` when you want remaps back. |

### Keyboard dead right now

```bash
# mouse still works — open Terminal and run:
sudo launchctl bootout system/local.keys-layer
sudo pkill -9 keys-layer
```

That **stops remaps** and returns the keyboard to macOS. Leave it off until you upgrade. To start again later:

```bash
sudo launchctl bootstrap system /Library/LaunchDaemons/local.keys-layer.plist
sudo launchctl kickstart -k system/local.keys-layer
```

---

## Uninstall

```bash
keys-layer-setup --no-daemon 2>/dev/null || true
# stop daemon if you installed it:
sudo launchctl bootout system/local.keys-layer 2>/dev/null
sudo rm -f /Library/LaunchDaemons/local.keys-layer.plist

brew uninstall keys-layer
# optional: brew untap sothearasao/keys-layer
```

Or use [`./scripts/uninstall.sh`](./scripts/uninstall.sh) if you also installed from source earlier.

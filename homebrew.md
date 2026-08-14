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

Stable installs **v0.1.3** from the GitHub tag. For the latest `main` branch instead:

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
2. **Config + LaunchDaemon:**

```bash
keys-layer-setup
```

3. **Privacy** — Accessibility + Input Monitoring for:
   `$(brew --prefix)/opt/keys-layer/bin/keys-layer`  
   (also shown under **Caveats** in `brew info sothearasao/keys-layer/keys-layer`)

4. **Restart the daemon** — TCC only applies on a new process; the first start usually ran before you toggled Privacy:

```bash
sudo launchctl kickstart -k system/local.keys-layer
```

You do **not** need to re-run `keys-layer-setup` for that — setup only helps because it kickstarts at the end.

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
| `keys-layer-setup: command not found` | Install failed — fix the build first, then `brew install …` again |
| Cursor stuck / keyboard dies with BT mouse | Set `devices = ["Apple Internal"]` (see [configuration.md](./configuration.md#listing-devices)); upgrade past the seize filter fix |

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

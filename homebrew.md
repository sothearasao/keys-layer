# Homebrew (macOS)

Install `keys-layer` with Homebrew, then finish VirtualHID + Privacy + daemon setup.

Related: [quickstart](./quickstart.md) · [prerequisite](./prerequisite.md) · [installation](./installation.md)

---

## Install

Homebrew 6+ requires trusting third-party taps (custom remotes must trust the **whole** tap):

```bash
brew tap sothearasao/keys-layer https://github.com/sothearasao/keys-layer.git
brew trust sothearasao/keys-layer
brew install --HEAD sothearasao/keys-layer/keys-layer
```

`--HEAD` builds from the `main` branch (no release tag required yet).

After install, see notes with:

```bash
brew info sothearasao/keys-layer/keys-layer
```

After you publish a GitHub release and fill `url` / `sha256` in [`Formula/keys-layer.rb`](./Formula/keys-layer.rb), users can drop `--HEAD`:

```bash
brew install sothearasao/keys-layer/keys-layer
```

---

## Finish setup

1. **VirtualHID** — enable the DriverKit extension ([prerequisite.md](./prerequisite.md)).
2. **Privacy** — Accessibility + Input Monitoring for:
   `$(brew --prefix)/opt/keys-layer/bin/keys-layer`  
   (also shown under **Caveats** in `brew info sothearasao/keys-layer/keys-layer`)
3. **Config + LaunchDaemon:**

```bash
keys-layer-setup
```

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
brew upgrade --fetch-HEAD keys-layer   # HEAD install
# or: brew upgrade keys-layer          # after a stable formula exists
keys-layer-setup                       # refresh LaunchDaemon paths if needed
```

Re-check Input Monitoring if the log shows `not permitted` after an upgrade.

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

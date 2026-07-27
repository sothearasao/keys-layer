# Autostart (macOS LaunchDaemon)

Run `keys-layer` automatically at login (as root), without keeping a terminal open.

**Recommended:** `./scripts/install.sh` installs the daemon for you.

**Before this:** [prerequisite.md](./prerequisite.md) and a working binary ([installation.md](./installation.md)).

Also: [uninstall](./uninstall.md) · [README](./README.md)

---

## Via install script

```bash
./scripts/install.sh
```

Generates a plist from [`packaging/local.keys-layer.plist.in`](./packaging/local.keys-layer.plist.in) with your absolute paths:

- Binary: `/usr/local/bin/keys-layer`
- Config: `/Users/YOU/.config/keys-layer/config.toml`

---

## Manual install

Copy the template and replace placeholders (or edit after `sed`):

```bash
BIN=/usr/local/bin/keys-layer
CFG="$HOME/.config/keys-layer/config.toml"

sed \
  -e "s|__KEYS_LAYER_BIN__|${BIN}|g" \
  -e "s|__KEYS_LAYER_CONFIG__|${CFG}|g" \
  packaging/local.keys-layer.plist.in > /tmp/local.keys-layer.plist

plutil -lint /tmp/local.keys-layer.plist
# expect: OK

sudo cp /tmp/local.keys-layer.plist /Library/LaunchDaemons/local.keys-layer.plist
sudo chown root:wheel /Library/LaunchDaemons/local.keys-layer.plist
sudo chmod 644 /Library/LaunchDaemons/local.keys-layer.plist

sudo launchctl bootout system/local.keys-layer 2>/dev/null
sudo launchctl bootstrap system /Library/LaunchDaemons/local.keys-layer.plist
sudo launchctl kickstart -k system/local.keys-layer
```

The file must end with a full closing tag `</plist>` (a truncated `</plist` causes `Bootstrap failed: 5: Input/output error`).

---

## Verify

```bash
sudo launchctl print system/local.keys-layer | head -40
tail -f /var/log/keys-layer.log
```

You should see:

```text
keys-layer (DriverKit) running — …
```

There should be **no** `IOHIDDeviceOpen … not permitted` lines. If remaps fail, fix permissions / quit KE remapping — see [prerequisite.md](./prerequisite.md) and [installation troubleshooting](./installation.md#troubleshooting).

---

## Reload (no reboot)

**Config** hot-reloads on save (or `sudo kill -HUP $(pgrep -f keys-layer)`). Check `/var/log/keys-layer.log` for `config reloaded OK` / `FAILED`.

After **reinstalling the binary**:

```bash
# Re-grant Accessibility + Input Monitoring if the binary path changed, then:
sudo launchctl kickstart -k system/local.keys-layer
```

Full stop/start:

```bash
sudo launchctl bootout system/local.keys-layer 2>/dev/null
sudo launchctl bootstrap system /Library/LaunchDaemons/local.keys-layer.plist
sudo launchctl kickstart -k system/local.keys-layer
```

---

## Stop / remove autostart only

Keeps the binary and config; only disables login start:

```bash
sudo launchctl bootout system/local.keys-layer
sudo rm -f /Library/LaunchDaemons/local.keys-layer.plist
```

Full removal: `./scripts/uninstall.sh` or [uninstall.md](./uninstall.md).

---

## Autostart troubleshooting

### `Bootstrap failed: 5: Input/output error`

Bad or incomplete plist.

```bash
plutil -lint /Library/LaunchDaemons/local.keys-layer.plist
```

| Result | Fix |
|--------|-----|
| EOF / parse error | Fix XML (`</plist>`); regenerate via `./scripts/install.sh` |
| `OK` but bootstrap still fails | `bootout` then `bootstrap` again; confirm `Label` is `local.keys-layer` |

Also check: absolute paths, files exist, owner `root:wheel`, mode `644`.

### Job is running but remaps don’t work

```bash
tail -40 /var/log/keys-layer.log
```

Usually Input Monitoring or Karabiner-Elements still grabbing — [prerequisite.md](./prerequisite.md).

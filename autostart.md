# Autostart (macOS LaunchDaemon)

Run `keys-layer` automatically at login (as root), without keeping a terminal open.

**Before this:** [prerequisite.md](./prerequisite.md) and [installation.md](./installation.md) (binary + config working with `sudo keys-layer`).

Also: [uninstall](./uninstall.md) · [README](./README.md)

---

## Template

Repo file: [`packaging/local.keys-layer.plist`](./packaging/local.keys-layer.plist)

### Edit paths

Use **absolute** paths — no `~`:

```xml
<string>/Users/YOUR_USER/.cargo/bin/keys-layer</string>
<string>/Users/YOUR_USER/.config/keys-layer/config.toml</string>
```

The file must end with a full closing tag:

```xml
</dict>
</plist>
```

A truncated `</plist` (missing `>`) causes `Bootstrap failed: 5: Input/output error`.

---

## Install and start

```bash
plutil -lint packaging/local.keys-layer.plist
# expect: OK

sudo cp packaging/local.keys-layer.plist /Library/LaunchDaemons/local.keys-layer.plist
sudo chown root:wheel /Library/LaunchDaemons/local.keys-layer.plist
sudo chmod 644 /Library/LaunchDaemons/local.keys-layer.plist

sudo launchctl bootout system/local.keys-layer 2>/dev/null
sudo launchctl bootstrap system /Library/LaunchDaemons/local.keys-layer.plist
sudo launchctl kickstart -k system/local.keys-layer
```

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

After editing config or `cargo install --force`:

```bash
# Re-grant Accessibility + Input Monitoring if you reinstalled the binary, then:
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

Full removal (binary + config too): [uninstall.md](./uninstall.md).

---

## Autostart troubleshooting

### `Bootstrap failed: 5: Input/output error`

Bad or incomplete plist.

```bash
plutil -lint /Library/LaunchDaemons/local.keys-layer.plist
```

| Result | Fix |
|--------|-----|
| EOF / parse error | Fix XML (`</plist>`); re-copy from `packaging/local.keys-layer.plist` |
| `OK` but bootstrap still fails | `bootout` then `bootstrap` again; confirm `Label` is `local.keys-layer` |

Also check: absolute paths, files exist, owner `root:wheel`, mode `644`.

### Job is running but remaps don’t work

```bash
tail -40 /var/log/keys-layer.log
```

Usually Input Monitoring or Karabiner-Elements still grabbing — [prerequisite.md](./prerequisite.md).

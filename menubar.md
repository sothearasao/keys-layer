# Menu bar toggle (macOS)

Remapping is **off by default**. Use the menu bar (or `keys-layer-ctl`) as the only way to turn it on or off. It does **not** start at login.

Related: [homebrew](./homebrew.md) · [installation](./installation.md) · [prerequisite](./prerequisite.md)

---

## Install

Run setup once (installs the LaunchDaemon **plist**, leaves remapping **off**):

```bash
keys-layer-setup
# optional: keys-layer-setup --start   # turn remapping on immediately
```

**Homebrew**

```bash
keys-layer-menu
# app: $(brew --prefix)/opt/keys-layer/Keys Layer.app
```

**From source**

```bash
./scripts/install.sh
open "/Applications/Keys Layer.app"
```

Enable **Launch Menu at Login** in the menu if you want the icon every boot (still does **not** start remapping).

---

## Use

| Item | Action |
|------|--------|
| **Turn Remapping On** | Starts remapping (admin password once) |
| **Turn Remapping Off** | Stops remapping; keyboard back to macOS |
| **Launch Menu at Login** | Menu icon at login only — remaps stay off until you turn them on |
| **Open Config** / **Open Log** | Config / `/var/log/keys-layer.log` |

Filled keyboard = **on**; outline = **off**. After reboot, remapping stays **off** until you turn it on again.

```bash
keys-layer-ctl status
sudo keys-layer-ctl on
sudo keys-layer-ctl off
```

---

## Notes

- The plist at `/Library/LaunchDaemons/local.keys-layer.plist` must stay installed — the menu uses it. It is **not** an autostart; `RunAtLoad` / `KeepAlive` are off.
- On/off needs **admin** (Karabiner IPC is root-only).
- Emergency: **Turn Remapping Off** or `keys-layer-emergency-stop`.

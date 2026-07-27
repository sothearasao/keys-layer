# Quick install

Minimal setup for **macOS**. Details: [prerequisite](./prerequisite.md) · [installation](./installation.md) · [configuration](./configuration.md)

---

## 1. Driver (once)

Install [Karabiner-Elements](https://karabiner-elements.pqrs.org/) (or the standalone VirtualHID pkg), then enable:

**System Settings → General → Login Items & Extensions → Driver Extensions**  
→ `org.pqrs.Karabiner-DriverKit-VirtualHIDDevice`

Quit Karabiner-Elements remapping (menu bar). Keep the VirtualHID **daemon** running.

---

## 2. Install keys-layer

```bash
cd /path/to/keys-layer
./scripts/install.sh
```

---

## 3. Permissions (once)

**System Settings → Privacy & Security** — add and enable:

`/usr/local/bin/keys-layer`

- Accessibility  
- Input Monitoring  

---

## 4. Check

```bash
tail -20 /var/log/keys-layer.log
```

You want `keys-layer (DriverKit) running` and **no** `not permitted`.

Edit config: `~/.config/keys-layer/config.toml` — **hot-reloads on save** (watch the log for `config reloaded OK` or `FAILED`).  
Rebuild binary: `./scripts/install.sh` or `sudo launchctl kickstart -k system/local.keys-layer`

---

## Undo

```bash
./scripts/uninstall.sh           # keep config
./scripts/uninstall.sh --purge   # delete config too
```

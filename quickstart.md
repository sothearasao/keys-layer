# Quick install

Minimal setup for **macOS**. Details: [prerequisite](./prerequisite.md) · [installation](./installation.md) · [homebrew](./homebrew.md) · [configuration](./configuration.md)

> **Always required:** Karabiner VirtualHID (driver + daemon) and Privacy permissions. That is by design on macOS — see [prerequisite.md](./prerequisite.md).
>
> **F-row:** On Apple keyboards, F1–F2 / F5–F12 act as brightness, backlight, media, and volume. **F3/F4 stay F-keys** (no Mission Control / Spotlight). Full table: [configuration.md](./configuration.md#f1f12-behavior-macos).

---

## Option A — Homebrew

```bash
brew tap sothearasao/keys-layer https://github.com/sothearasao/keys-layer.git
brew install --HEAD keys-layer
keys-layer-setup
```

Grant **Accessibility** + **Input Monitoring** to the path shown by `brew caveats keys-layer`.

More: [homebrew.md](./homebrew.md).

---

## Option B — From source

### 1. Driver (once)

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
Rebuild binary: `./scripts/install.sh` — if remaps die afterward, re-toggle Input Monitoring / Accessibility for `/usr/local/bin/keys-layer`, then `sudo launchctl kickstart -k system/local.keys-layer`.

---

## Undo

```bash
./scripts/uninstall.sh           # keep config
./scripts/uninstall.sh --purge   # delete config too
```

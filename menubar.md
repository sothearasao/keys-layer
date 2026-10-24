# Menu bar toggle (macOS)

Remapping is **off by default**. Use **Keys Layer.app** in `/Applications` to turn it on or off. It does **not** start at login.

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
| **Turn Remapping On** | Starts remapping (admin once; first time seeds config + plist) |
| **Turn Remapping Off** | Stops remapping |
| **Launch Menu at Login** | Menu icon at login only — remaps stay off until you turn them on |
| **Open Config** / **Open Log** | `~/.config/keys-layer/config.toml` / `/var/log/keys-layer.log` |

```bash
keys-layer-ctl status
sudo keys-layer-ctl on
sudo keys-layer-ctl off
```

---

## Notes

- Keep VirtualHID driver + daemon healthy ([prerequisite](./prerequisite.md)).
- Emergency: **Turn Remapping Off** or `keys-layer-emergency-stop`.

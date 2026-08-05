# keys-layer

A simple hold-to-layer keyboard remapper for **macOS**, **Linux**, and **Windows**, written in Rust.

Inspired by [Kanata](https://github.com/jtroo/kanata), but focused on a small feature set and a plain **TOML** config.

## Features

- **Momentary layers** — hold a key past `hold_ms` to activate a layer; release to leave
- **Tap vs hold** — quick tap can emit a different key (or nothing)
- **Holdable remaps** — layer keys like `j = "delete"` tap once or repeat while held
- **Chords / sequences** — e.g. `k = ["left_alt", "delete"]` (Option+Delete) or `{ sequence = ["a", "b"] }`
- **Config hot-reload** — save `config.toml` while running; success/failure is logged (no restart)
- **Per-layer / per-key timing** — `hold_ms` on settings, layer, or key
- **`native = "disable"`** — suppress physical key behavior (useful for Caps Lock)
- **macOS** — [Karabiner-DriverKit-VirtualHIDDevice](https://github.com/pqrs-org/Karabiner-DriverKit-VirtualHIDDevice)
- **Linux** — evdev exclusive grab + uinput ([linux.md](./linux.md))
- **Windows** — low-level keyboard hook + SendInput ([windows.md](./windows.md))
- **Mac F-row media** (Apple Internal by default) — brightness, volume, playback, etc.; see [F1–F12](./configuration.md#f1f12-behavior-macos)

### F1–F12 (short)

On Apple Internal (configurable): F1–F2 brightness, F5–F6 keyboard light, F7–F9 media, F10–F12 mute/volume. **F3/F4 stay as F-keys** (no Mission Control / Spotlight via VirtualHID). Other boards keep real F1–F12. Details: [configuration.md](./configuration.md#f1f12-behavior-macos).

## Quick start

See **[quickstart.md](./quickstart.md)** or install with **[Homebrew](./homebrew.md)**:

```bash
brew tap sothearasao/keys-layer https://github.com/sothearasao/keys-layer.git
brew trust sothearasao/keys-layer
brew install sothearasao/keys-layer/keys-layer
keys-layer-setup
# grant Accessibility + Input Monitoring to:
#   $(brew --prefix)/opt/keys-layer/bin/keys-layer
```

From source:

```bash
./scripts/install.sh
# grant Accessibility + Input Monitoring to /usr/local/bin/keys-layer
tail -20 /var/log/keys-layer.log
```

| Doc | When |
|-----|------|
| [quickstart.md](./quickstart.md) | First-time quickstart (from source) |
| [homebrew.md](./homebrew.md) | `brew install` (macOS) |
| [linux.md](./linux.md) | Linux: `./scripts/install-linux.sh` |
| [windows.md](./windows.md) | Windows: `.\scripts\install-windows.ps1` |
| [prerequisite.md](./prerequisite.md) | Driver / permissions (macOS; required forever) |
| [installation.md](./installation.md) | Full install + troubleshooting (macOS) |
| [configuration.md](./configuration.md) | TOML reference |
| [uninstall.md](./uninstall.md) | Remove |

Default config path (when no argument is given):

```text
~/.config/keys-layer/config.toml
```

## Example config

```toml
[settings]
hold_ms = 200          # global fallback

[layer.base]
hold_ms = 150          # default for hold-keys on this layer
f = { tap = "f", hold = "mod_f" }
caps = { hold = "mod_caps", native = "disable", hold_ms = 100 }

[layer.mod_f]
j = { key = "delete" }              # Delete
k = ["left_alt", "delete"]          # Option + Delete

[layer.mod_caps]
h = "left"
j = "down"
k = "up"
l = "right"
```

Copy [`config.example.toml`](./config.example.toml) to get started.

Full syntax: **[configuration.md](./configuration.md)**.

### Config rules (short)

| Binding | Meaning |
|---------|---------|
| `j = "delete"` | Remap (holdable / repeats) |
| `j = { key = "delete" }` | Same as above |
| `k = ["left_alt", "delete"]` | Chord (Option+Delete); hold repeats last key |
| `m = { sequence = ["a", "b"] }` | Tap each key in order on press |
| `f = { tap = "f", hold = "mod_f" }` | Tap → `f`; hold → layer `mod_f` |
| `caps = { hold = "mod_caps", native = "disable" }` | Hold-only layer; never fire native Caps Lock |

**`hold_ms` priority:** per-key → per-layer → `[settings]`.

## Project layout

```text
crates/
  keys-layer-core/   # TOML config + layer / tap-hold engine (platform-free)
  keys-layer/        # macOS DriverKit CLI
```

## Development

```bash
cargo test -p keys-layer-core
cargo build -p keys-layer
cargo install --path crates/keys-layer --force
```

## Important notes

### Permanent macOS requirements (by design)

macOS does not allow a normal app to seize the keyboard. Like [Kanata](https://github.com/jtroo/kanata), keys-layer uses **Karabiner’s DriverKit VirtualHIDDevice**:

| Requirement | Stays after install? |
|-------------|----------------------|
| VirtualHID **driver extension** enabled | Yes — needed every boot |
| VirtualHID **daemon** running | Yes — usually via Karabiner-Elements or the standalone daemon |
| **Accessibility** + **Input Monitoring** for `/usr/local/bin/keys-layer` | Yes — re-check after reinstalling the binary |
| Run as **root** (LaunchDaemon or `sudo`) | Yes — Karabiner IPC is root-only |

You do **not** need Karabiner-Elements’ own remapping (Core-Service). Use the VirtualHID stack only; quit KE remapping so it does not fight keys-layer.

These are not temporary setup steps you can remove later — they are how keyboard remapping works on modern macOS without a custom Apple-signed dext.

### Linux

See **[linux.md](./linux.md)** — needs access to `/dev/input` (grab) and `/dev/uinput` (emit). No Karabiner equivalent.

### Other

- Do **not** run Karabiner-Elements remapping at the same time on macOS — only one process can grab the keyboard.
- Uses `karabiner-driverkit` **0.3.x** (compatible with Karabiner-Elements’ bundled VirtualHID daemon ~6.x). Version **0.4.x** needs standalone VirtualHIDDevice v8.0.0 and will fail with `connect_failed asio.system:2` against KE’s daemon.

## License

MIT

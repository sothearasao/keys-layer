# keys-layer

A simple hold-to-layer keyboard remapper for **macOS**, written in Rust.

Inspired by [Kanata](https://github.com/jtroo/kanata), but focused on a small feature set and a plain **TOML** config.

## Features

- **Momentary layers** — hold a key past `hold_ms` to activate a layer; release to leave
- **Tap vs hold** — quick tap can emit a different key (or nothing)
- **Holdable remaps** — layer keys like `j = "delete"` tap once or repeat while held
- **Per-layer / per-key timing** — `hold_ms` on settings, layer, or key
- **`native = "disable"`** — suppress physical key behavior (useful for Caps Lock)
- **DriverKit backend** — uses [Karabiner-DriverKit-VirtualHIDDevice](https://github.com/pqrs-org/Karabiner-DriverKit-VirtualHIDDevice) (same approach as Kanata)

## Quick start

See **[install.md](./install.md)** (short checklist).

```bash
./scripts/install.sh
# grant Accessibility + Input Monitoring to /usr/local/bin/keys-layer
tail -20 /var/log/keys-layer.log
```

| Doc | When |
|-----|------|
| [install.md](./install.md) | First-time quickstart |
| [prerequisite.md](./prerequisite.md) | Driver / permissions deep dive |
| [installation.md](./installation.md) | Full install + troubleshooting |
| [configuration.md](./configuration.md) | TOML reference |
| [uninstall.md](./uninstall.md) | Remove (`./scripts/uninstall.sh`) |

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
j = { key = "delete" } # tap = one delete; hold = repeating delete

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

- Must run as **root** (`sudo`) — Karabiner VirtualHID IPC is root-only.
- Do **not** run Karabiner-Elements remapping at the same time — only one process can grab the keyboard.
- Uses `karabiner-driverkit` **0.3.x** (compatible with Karabiner-Elements’ bundled VirtualHID daemon ~6.x). Version **0.4.x** needs standalone VirtualHIDDevice v8.0.0 and will fail with `connect_failed asio.system:2` against KE’s daemon.

## License

MIT

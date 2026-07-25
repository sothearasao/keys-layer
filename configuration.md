# Configuration

How to write and load a `keys-layer` config.

Related docs: [README](./README.md) · [prerequisite](./prerequisite.md) · [installation](./installation.md) · [autostart](./autostart.md) · [uninstall](./uninstall.md) · [config.example.toml](./config.example.toml)

## Where the config lives

Default path (used when you run `sudo keys-layer` with no arguments):

```text
~/.config/keys-layer/config.toml
```

Create it from the example:

```bash
mkdir -p ~/.config/keys-layer
cp config.example.toml ~/.config/keys-layer/config.toml
```

Or pass a path explicitly:

```bash
sudo keys-layer /path/to/config.toml
```

Format is **TOML**. After editing, restart `keys-layer` (Ctrl-C, then `sudo keys-layer` again). There is no hot-reload yet.

---

## Overview

A config has:

1. Optional `[settings]` — global defaults  
2. One or more `[layer.<name>]` tables — keymaps  
3. The layer named `base` (or `settings.base_layer`) is active at startup  

Hold a key past `hold_ms` → enter another layer (momentary). Release → return.

---

## `[settings]`

```toml
[settings]
hold_ms = 200
base_layer = "base"
# devices = ["Moonlander"]           # optional: seize only this board
f_row_media_devices = ["Apple Internal"]  # Fn-aware F-row media (default)
```

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `hold_ms` | integer (ms) | `200` | Global hold delay when a layer/key does not set its own |
| `base_layer` | string | `"base"` | Layer active when the program starts (must exist) |
| `devices` | string array | `[]` (all keyboards) | Product-name substrings to seize; empty = all |
| `f_row_media_devices` | string array | `["Apple Internal"]` | Only these keyboards get Mac-style F1–F12 ↔ media (Fn/Globe + System Settings). Other boards keep real F-keys. Set `[]` to disable. |

On matching devices, F1–F12 follow macOS: media by default, real F-keys while holding **Fn** / **Globe** (inverted if “Use F1, F2… as standard function keys” is on). F3/F4 stay as F-keys (no stable VirtualHID Mission Control / Spotlight).

---

## Layers — `[layer.<name>]`

```toml
[layer.base]
hold_ms = 150
f = { tap = "f", hold = "mod_f" }
caps = { hold = "mod_caps", native = "disable", hold_ms = 100 }
```

| Entry | Meaning |
|-------|---------|
| `hold_ms = N` | Default hold delay for hold-keys **on this layer** |
| `key = …` | Binding for that physical key (see below) |

`hold_ms` on a layer is **not** a keyboard key; it is reserved metadata.

### Hold delay priority

When a hold-to-layer key is pressed, delay is chosen as:

1. **Per-key** `hold_ms` on that binding  
2. Else **layer** `hold_ms`  
3. Else **`[settings].hold_ms`**

---

## Key bindings

### 1. Simple remap (holdable)

Emit another key. Tap once for one press; keep holding for OS-style repeat.

```toml
[layer.mod_f]
j = "delete"
# equivalent:
j = { key = "delete" }
j = { tap = "delete" }
```

| Event | Output |
|-------|--------|
| Press | target key down |
| Hold (repeat) | repeating target |
| Release | target key up |

### 2. Hold-to-layer

```toml
[layer.base]
f = { tap = "f", hold = "mod_f" }
f = { tap = "f", hold = "mod_f", hold_ms = 180 }
caps = { hold = "mod_caps", native = "disable" }
caps = { tap = "escape", hold = "mod_caps", native = "disable", hold_ms = 100 }
```

| Field | Required | Description |
|-------|----------|-------------|
| `hold` | **yes** | Name of the layer to activate (must be defined) |
| `tap` | no | Key sent on quick release before `hold_ms` |
| `hold_ms` | no | Delay override for this key only |
| `native` | no | `"enable"` (default) or `"disable"` |

#### Momentary hold

- Hold past `hold_ms` → push that layer  
- Release the hold key → leave the layer  
- No permanent layer switch in v1  

#### Fast typing

If another key is pressed **before** `hold_ms`, the pending hold key is resolved as a **tap** first (so rolls like `fe` stay in order instead of becoming `ef`).

#### `native = "disable"`

Never fire the physical key. Use this for **Caps Lock** so macOS does not toggle Caps Lock / the LED.

| Config | Quick press | Long hold |
|--------|-------------|-----------|
| `{ hold = "mod_caps", native = "disable" }` | nothing | `mod_caps` |
| `{ tap = "escape", hold = "mod_caps", native = "disable" }` | Escape | `mod_caps` |

---

## Complete example

```toml
[settings]
hold_ms = 200

[layer.base]
hold_ms = 150
f = { tap = "f", hold = "mod_f" }
caps = { hold = "mod_caps", native = "disable", hold_ms = 100 }

[layer.mod_f]
j = { key = "delete" }

[layer.mod_caps]
h = "left"
j = "down"
k = "up"
l = "right"
```

Behavior:

- Tap `f` → types `f`  
- Hold `f` ≥ 150ms → `mod_f`; then `j` → Delete (hold `j` to repeat Delete)  
- Hold Caps ≥ 100ms → vim arrows on `hjkl`; release Caps to leave  
- Caps Lock itself never toggles (`native = "disable"`)

Same file: [`config.example.toml`](./config.example.toml).

---

## Key names

Names are case-insensitive. `-` and `_` are normalized.

### Aliases

| You can write | Stored as |
|---------------|-----------|
| `caps`, `capslock`, `cap_lock` | `caps_lock` |
| `esc` | `escape` |
| `backspace`, `bksp` | `delete` (Mac backspace) |
| `del`, `fwd_delete`, `forward_del` | `forward_delete` |
| `return` | `enter` |
| `cmd`, `meta`, `win`, `lcmd` | `left_meta` |
| `rcmd`, `right_cmd` | `right_meta` |
| `alt`, `option`, `lalt` | `left_alt` |
| `ralt`, `right_option` | `right_alt` |
| `ctrl`, `lctrl` | `left_control` |
| `rctrl` | `right_control` |
| `shift`, `lshift` | `left_shift` |
| `rshift` | `right_shift` |

### Common keys

- Letters: `a`–`z`  
- Digits: `0`–`9`  
- Arrows: `left`, `right`, `up`, `down`  
- Nav: `home`, `end`, `page_up`, `page_down`  
- Mods: `left_shift`, `left_control`, `left_alt`, `left_meta`, and `right_*`  
- Function: `f1`–`f12`  
- Other: `space`, `tab`, `enter`, `escape`, `delete`, `forward_delete`  
- Punctuation names: `minus`, `equal`, `left_bracket`, `right_bracket`, `backslash`, `semicolon`, `quote`, `grave`, `comma`, `period`, `slash`

Full DriverKit HID map: `crates/keys-layer/src/macos/hid_usage.rs`.

---

## Validation errors

| Message (typical) | Cause |
|-------------------|--------|
| `base layer "…" not found` | `settings.base_layer` does not match any `[layer.*]` |
| `hold target layer "…" does not exist` | `hold = "…"` points at a missing layer |
| `need key/tap … or hold = "layer"` | Table binding is empty / incomplete |
| `native = "disable" only applies with hold` | `native` used on a plain remap |

---

## Tips

- Prefer **Caps** (`native = "disable"`) as a layer key; use a letter like `f` only if you accept tap/hold timing tradeoffs.  
- Shorter `hold_ms` (e.g. `80–120`) feels snappier for Caps; slightly longer (`150–200`) is safer for home-row letters.  
- Layer remaps (`j = "delete"`) already support **hold-to-repeat**; you do not need a special “hold delete” binding.  
- Keep one config file under version control or back it up; the binary does not write the config.

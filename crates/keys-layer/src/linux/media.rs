//! Linux F-row → multimedia keys (for `f_row_media_devices`).
//!
//! Unlike macOS (Apple Fn / Globe + System Settings), Linux uses `KEY_FN` when
//! the board sends it. Default (no Fn held): media mapping. Hold Fn: real F-keys.
//! F3/F4 stay as F-keys (same as macOS VirtualHID limits).

use std::sync::atomic::{AtomicBool, Ordering};

use evdev::KeyCode;

static FN_DOWN: AtomicBool = AtomicBool::new(false);

pub fn note_fn_key(code: KeyCode, value: i32) -> bool {
    if code != KeyCode::KEY_FN && code != KeyCode::KEY_FN_ESC {
        return false;
    }
    FN_DOWN.store(value != 0, Ordering::Relaxed);
    true
}

pub fn want_media_for_f_row() -> bool {
    !FN_DOWN.load(Ordering::Relaxed)
}

/// Map F1–F12 → Linux multimedia key. `None` = keep real F-key.
pub fn media_keycode_for_f(code: KeyCode) -> Option<KeyCode> {
    Some(match code {
        KeyCode::KEY_F1 => KeyCode::KEY_BRIGHTNESSDOWN,
        KeyCode::KEY_F2 => KeyCode::KEY_BRIGHTNESSUP,
        // F3 / F4 — no stable desktop equivalents we control → keep F-keys
        KeyCode::KEY_F5 => KeyCode::KEY_KBDILLUMDOWN,
        KeyCode::KEY_F6 => KeyCode::KEY_KBDILLUMUP,
        KeyCode::KEY_F7 => KeyCode::KEY_PREVIOUSSONG,
        KeyCode::KEY_F8 => KeyCode::KEY_PLAYPAUSE,
        KeyCode::KEY_F9 => KeyCode::KEY_NEXTSONG,
        KeyCode::KEY_F10 => KeyCode::KEY_MUTE,
        KeyCode::KEY_F11 => KeyCode::KEY_VOLUMEDOWN,
        KeyCode::KEY_F12 => KeyCode::KEY_VOLUMEUP,
        _ => return None,
    })
}

pub fn is_function_row(code: KeyCode) -> bool {
    matches!(
        code,
        KeyCode::KEY_F1
            | KeyCode::KEY_F2
            | KeyCode::KEY_F3
            | KeyCode::KEY_F4
            | KeyCode::KEY_F5
            | KeyCode::KEY_F6
            | KeyCode::KEY_F7
            | KeyCode::KEY_F8
            | KeyCode::KEY_F9
            | KeyCode::KEY_F10
            | KeyCode::KEY_F11
            | KeyCode::KEY_F12
    )
}

//! Mac F-row → media, gated by Fn/Globe + `com.apple.keyboard.fnState`.
//!
//! Only used for devices listed in `f_row_media_devices` (default: Apple Internal)
//! so other boards (e.g. Moonlander) keep real F1–F12.

use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use super::hid_usage::{
    PAGE_APPLE_TOP_CASE, PAGE_APPLE_VENDOR_KEYBOARD, PAGE_CONSUMER, USAGE_APPLE_FN,
    USAGE_APPLE_GLOBE,
};

static FN_DOWN: AtomicBool = AtomicBool::new(false);
/// `true` when System Settings → “Use F1, F2, … as standard function keys”.
static FN_STATE_STANDARD: AtomicBool = AtomicBool::new(false);
static FN_STATE_CHECKED_AT: AtomicU64 = AtomicU64::new(0);

/// Track Apple Fn (top-case) and Globe (vendor keyboard) for F-row mode.
pub fn note_modifier_event(page: u32, code: u32, value: u64) {
    let down = value != 0;
    if (page == PAGE_APPLE_TOP_CASE && code == USAGE_APPLE_FN)
        || (page == PAGE_APPLE_VENDOR_KEYBOARD && code == USAGE_APPLE_GLOBE)
    {
        FN_DOWN.store(down, Ordering::Relaxed);
    }
}

fn refresh_fn_state_preference() {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let last = FN_STATE_CHECKED_AT.load(Ordering::Relaxed);
    if now.saturating_sub(last) < 3 {
        return;
    }
    FN_STATE_CHECKED_AT.store(now, Ordering::Relaxed);

    let out = Command::new("defaults")
        .args(["read", "-g", "com.apple.keyboard.fnState"])
        .output();
    let standard = out
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| s.trim().parse::<i32>().ok())
        .map(|v| v != 0)
        .unwrap_or(false);
    FN_STATE_STANDARD.store(standard, Ordering::Relaxed);
}

/// Whether an F1–F12 press should become a media key (vs a real function key).
pub fn want_media_for_f_row() -> bool {
    refresh_fn_state_preference();
    let fn_down = FN_DOWN.load(Ordering::Relaxed);
    let standard = FN_STATE_STANDARD.load(Ordering::Relaxed);
    if standard {
        fn_down
    } else {
        !fn_down
    }
}

/// Map keyboard-page F1–F12 usage → (page, code) media HID for VirtualHID.
/// F3/F4 have no stable VirtualHID equivalent → `None` (emit real F-key).
pub fn media_hid_for_f_usage(usage: u32) -> Option<(u32, u32)> {
    Some(match usage {
        0x3A => (PAGE_APPLE_TOP_CASE, 0x05), // brightness down
        0x3B => (PAGE_APPLE_TOP_CASE, 0x04), // brightness up
        0x3E => (PAGE_APPLE_TOP_CASE, 0x09), // illum down
        0x3F => (PAGE_APPLE_TOP_CASE, 0x08), // illum up
        0x40 => (PAGE_CONSUMER, 0xB6),       // prev track
        0x41 => (PAGE_CONSUMER, 0xCD),       // play/pause
        0x42 => (PAGE_CONSUMER, 0xB5),       // next track
        0x43 => (PAGE_CONSUMER, 0xE2),       // mute
        0x44 => (PAGE_CONSUMER, 0xEA),       // volume down
        0x45 => (PAGE_CONSUMER, 0xE9),       // volume up
        _ => return None,
    })
}

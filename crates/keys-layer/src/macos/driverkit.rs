//! Karabiner DriverKit VirtualHIDDevice backend.
//!
//! Seizes physical keyboards via the Karabiner dext and emits remapped keys
//! through a virtual HID keyboard. Requires:
//! - Karabiner-DriverKit-VirtualHIDDevice installed & activated
//! - VirtualHIDDevice daemon running
//! - keys-layer running as root (`sudo`)
//! - Input Monitoring + Accessibility for the binary

use std::collections::HashSet;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use karabiner_driverkit::{
    driver_activated, fetch_devices, grab, is_sink_ready, register_device, release, send_key,
    wait_key, DKEvent,
};
use keys_layer_core::{load_config, Engine, InputEvent, KeyName, OutputEvent};

use super::caps_lock;
use super::hid_usage::{key_name_to_usage, usage_to_key_name, PAGE_KEYBOARD};
use super::media_keys;

const VALUE_RELEASE: u64 = 0;
const VALUE_PRESS: u64 = 1;
const VALUE_REPEAT: u64 = 2;

/// Run the DriverKit remapper until the process exits (or grab fails).
pub fn run(config_path: &Path) -> Result<(), String> {
    ensure_root()?;

    if !driver_activated() {
        return Err(
            "Karabiner-DriverKit-VirtualHIDDevice is not activated.\n\
             Install the driver pkg, then run:\n\
             sudo /Applications/.Karabiner-VirtualHIDDevice-Manager.app/Contents/MacOS/Karabiner-VirtualHIDDevice-Manager forceActivate\n\
             Enable it under System Settings → General → Login Items & Extensions → Driver Extensions.\n\
             Start the daemon (or install Karabiner-Elements which manages it)."
                .into(),
        );
    }

    let config = load_config(config_path).map_err(|e| e.to_string())?;
    let device_patterns = config.settings.devices.clone();
    let f_row_media_hashes = device_hashes_matching(&config.settings.f_row_media_devices);
    if !f_row_media_hashes.is_empty() {
        eprintln!(
            "F-row media (Fn/Globe) enabled for {} device(s)",
            f_row_media_hashes.len()
        );
    }
    let engine = Arc::new(Mutex::new(Engine::new(config)));
    let started = Instant::now();

    caps_lock::force_caps_lock_off();

    let seized = seize_devices(&device_patterns)?;
    eprintln!("seized keyboards: {}", seized.join(", "));

    if !grab() {
        return Err(
            "grab failed — grant Input Monitoring + Accessibility to this binary, \
             run as root, and ensure no other remapper has an exclusive grab."
                .into(),
        );
    }

    struct Guard;
    impl Drop for Guard {
        fn drop(&mut self) {
            release();
        }
    }
    let _guard = Guard;

    if !wait_for_sink(Duration::from_secs(10)) {
        return Err(
            "DriverKit virtual keyboard not ready (sink disconnected).\n\
             Start Karabiner-VirtualHIDDevice-Daemon and retry."
                .into(),
        );
    }

    let engine_tick = Arc::clone(&engine);
    let started_tick = started;
    thread::spawn(move || loop {
        thread::sleep(Duration::from_millis(10));
        let now_ms = started_tick.elapsed().as_millis() as u64;
        let outputs = {
            let mut eng = engine_tick.lock().expect("engine lock");
            eng.tick(now_ms)
        };
        emit_outputs(&outputs);
    });

    eprintln!(
        "keys-layer (DriverKit) running — {}\n\
         Hold F / Caps for layers. Requires sudo + Karabiner VirtualHIDDevice.\n\
         Ctrl-C to quit.",
        config_path.display()
    );

    loop {
        let mut event = DKEvent {
            value: 0,
            page: 0,
            code: 0,
            device_hash: 0,
        };
        let got = wait_key(&mut event);
        if got == 0 {
            return Err("input pipe closed (devices released)".into());
        }

        // Track Fn / Globe before forwarding non-keyboard pages.
        if event.page != PAGE_KEYBOARD {
            media_keys::note_modifier_event(event.page, event.code, event.value);
            emit_hid(event.page, event.code, event.value);
            continue;
        }
        if event.code == 0xffff_ffff || event.code == 0x1 {
            continue;
        }
        if event.value > VALUE_REPEAT {
            continue;
        }

        // F1–F12: on configured devices, apply Mac Fn/Globe media behavior.
        // Other keyboards keep real F-keys.
        if is_function_row(event.code) {
            let media_device = f_row_media_hashes.contains(&event.device_hash);
            if media_device && media_keys::want_media_for_f_row() {
                if let Some((page, code)) = media_keys::media_hid_for_f_usage(event.code) {
                    emit_hid(page, code, event.value);
                    continue;
                }
            }
            emit_hid(event.page, event.code, event.value);
            continue;
        }

        let Some(key) = usage_to_key_name(event.code) else {
            passthrough(&mut event);
            continue;
        };

        let now_ms = started.elapsed().as_millis() as u64;
        let input = match event.value {
            VALUE_RELEASE => InputEvent::KeyUp(key.clone()),
            VALUE_PRESS | VALUE_REPEAT => InputEvent::KeyDown(key.clone()),
            _ => continue,
        };

        // Single lock: decide intercept + handle without racing the tick thread.
        let outputs = {
            let mut eng = engine.lock().expect("engine lock");
            if !(eng.should_intercept(&key) || eng.is_native_disabled(&key)) {
                None
            } else {
                if key.as_str() == "caps_lock" {
                    caps_lock::force_caps_lock_off();
                }
                Some(eng.handle(input, now_ms))
            }
        };

        match outputs {
            None => passthrough(&mut event),
            Some(outputs) => emit_outputs(&outputs),
        }
    }
}

fn ensure_root() -> Result<(), String> {
    let uid = unsafe { libc::geteuid() };
    if uid != 0 {
        return Err(
            "DriverKit backend must run as root (Karabiner IPC is root-only).\n\
             Try: sudo keys-layer\n\
             or:  sudo keys-layer ~/.config/keys-layer/config.toml"
                .into(),
        );
    }
    Ok(())
}

fn wait_for_sink(timeout: Duration) -> bool {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if is_sink_ready() {
            return true;
        }
        thread::sleep(Duration::from_millis(100));
    }
    is_sink_ready()
}

/// Hashes of connected keyboards whose product name matches any pattern.
fn device_hashes_matching(patterns: &[String]) -> HashSet<u64> {
    if patterns.is_empty() {
        return HashSet::new();
    }
    let available = fetch_devices();
    let mut out = HashSet::new();
    for device in &available {
        let name = device.product_key.to_lowercase();
        if patterns
            .iter()
            .any(|p| name.contains(&p.to_lowercase()))
        {
            out.insert(device.hash);
        }
    }
    out
}

/// Register keyboards to seize.
/// Empty `patterns` → all keyboards. Otherwise match product-name substrings.
fn seize_devices(patterns: &[String]) -> Result<Vec<String>, String> {
    if patterns.is_empty() {
        if !register_device("") {
            return Err(
                "failed to register keyboards with Karabiner DriverKit \
                 (is the VirtualHIDDevice daemon running?)"
                    .into(),
            );
        }
        return Ok(vec!["(all keyboards)".into()]);
    }

    let available = fetch_devices();
    if available.is_empty() {
        return Err(
            "no HID keyboards found (is the VirtualHIDDevice daemon running?)".into(),
        );
    }

    let mut seized = Vec::new();
    for pattern in patterns {
        let pat = pattern.to_lowercase();
        for device in available
            .iter()
            .filter(|d| d.product_key.to_lowercase().contains(&pat))
        {
            let hash_id = format!("0x{:X}", device.hash);
            let ok = register_device(&hash_id) || register_device(&device.product_key);
            if ok && !seized.iter().any(|s| s == &device.product_key) {
                seized.push(device.product_key.clone());
            }
        }
    }

    if seized.is_empty() {
        let names: Vec<_> = available.iter().map(|d| d.product_key.as_str()).collect();
        return Err(format!(
            "no keyboards matched devices = {patterns:?}\n\
             Connected: {}\n\
             Tip: use a product-name substring, e.g. devices = [\"Moonlander\"]",
            names.join(", ")
        ));
    }

    Ok(seized)
}

fn passthrough(event: &mut DKEvent) {
    if event.page == PAGE_KEYBOARD {
        event.code = fix_iso_virtual_usage(event.code);
    }
    emit_hid(event.page, event.code, event.value);
}

fn emit_hid(page: u32, code: u32, value: u64) {
    let mut value = value;
    if value == VALUE_REPEAT {
        value = VALUE_PRESS;
    }
    let mut event = DKEvent {
        value,
        page,
        code,
        device_hash: 0,
    };
    let rc = send_key(&mut event);
    if rc == 2 {
        eprintln!("warning: virtual keyboard sink not ready (event dropped)");
    } else if rc == 1 {
        eprintln!("warning: unrecognized HID page={page:#x} code={code:#x}");
    }
}

fn emit_outputs(outputs: &[OutputEvent]) {
    for out in outputs {
        let (name, value) = match out {
            OutputEvent::KeyDown(k) | OutputEvent::KeyRepeat(k) => (k, VALUE_PRESS),
            OutputEvent::KeyUp(k) => (k, VALUE_RELEASE),
        };

        if name.as_str() == "caps_lock" {
            if value == VALUE_PRESS {
                let on = caps_lock::get_caps_lock_state().unwrap_or(false);
                let _ = caps_lock::set_caps_lock_state(!on);
            }
            continue;
        }

        let Some(code) = key_name_to_usage(name) else {
            eprintln!("warning: unknown output key: {name}");
            continue;
        };

        let mut event = DKEvent {
            value,
            page: PAGE_KEYBOARD,
            code: fix_iso_virtual_usage(code),
            device_hash: 0,
        };

        let rc = send_key(&mut event);
        if rc == 2 {
            eprintln!("warning: virtual keyboard sink not ready");
        } else if rc == 1 {
            eprintln!("warning: unrecognized HID usage for {name}");
        }
    }
}

fn is_function_row(code: u32) -> bool {
    (0x3A..=0x45).contains(&code)
}

/// VirtualHID often reports as ISO; swap grave ↔ non_us_backslash so ANSI
/// boards type `/~ instead of §/±.
fn fix_iso_virtual_usage(code: u32) -> u32 {
    match code {
        0x35 => 0x64,
        0x64 => 0x35,
        other => other,
    }
}

#[allow(dead_code)]
fn _touch_keyname(_: &KeyName) {}

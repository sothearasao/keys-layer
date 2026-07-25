//! Karabiner DriverKit VirtualHIDDevice backend.
//!
//! Seizes physical keyboards via the Karabiner dext and emits remapped keys
//! through a virtual HID keyboard. Requires:
//! - Karabiner-DriverKit-VirtualHIDDevice installed & activated
//! - VirtualHIDDevice daemon running
//! - keys-layer running as root (`sudo`)
//! - Input Monitoring + Accessibility for the binary

use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use karabiner_driverkit::{
    driver_activated, grab, is_sink_ready, register_device, release, send_key, wait_key, DKEvent,
};
use keys_layer_core::{load_config, Engine, InputEvent, KeyName, OutputEvent};

use super::caps_lock;
use super::hid_usage::{key_name_to_usage, usage_to_key_name, PAGE_KEYBOARD};

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
    let engine = Arc::new(Mutex::new(Engine::new(config)));
    let started = Instant::now();

    caps_lock::force_caps_lock_off();

    if !register_device("") {
        return Err(
            "failed to register keyboards with Karabiner DriverKit \
             (is the VirtualHIDDevice daemon running?)"
                .into(),
        );
    }

    if !grab() {
        return Err(
            "grab failed — grant Input Monitoring + Accessibility to this binary, \
             run as root, and ensure no other remapper has an exclusive grab."
                .into(),
        );
    }

    // Ensure we release devices on exit.
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

    // Hold-timer tick thread.
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

        if event.page != PAGE_KEYBOARD {
            continue;
        }
        // Skip noise values / reserved codes (same idea as kanata).
        if event.code == 0xffff_ffff || event.code == 0x1 {
            continue;
        }
        if event.value > VALUE_REPEAT {
            continue;
        }

        let Some(key) = usage_to_key_name(event.code) else {
            // Unknown key — pass through unchanged.
            passthrough(&mut event);
            continue;
        };

        let now_ms = started.elapsed().as_millis() as u64;
        let intercept = {
            let eng = engine.lock().expect("engine lock");
            eng.should_intercept(&key) || eng.is_native_disabled(&key)
        };

        if !intercept {
            passthrough(&mut event);
            continue;
        }

        // Caps Lock: never emit native; clear lock state.
        if key.as_str() == "caps_lock" {
            caps_lock::force_caps_lock_off();
        }

        let input = match event.value {
            VALUE_RELEASE => InputEvent::KeyUp(key),
            VALUE_PRESS | VALUE_REPEAT => InputEvent::KeyDown(key),
            _ => continue,
        };

        let outputs = {
            let mut eng = engine.lock().expect("engine lock");
            eng.handle(input, now_ms)
        };
        emit_outputs(&outputs);
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

fn passthrough(event: &mut DKEvent) {
    let rc = send_key(event);
    if rc == 2 {
        eprintln!("warning: virtual keyboard sink not ready (passthrough dropped)");
    }
}

fn emit_outputs(outputs: &[OutputEvent]) {
    for out in outputs {
        let (name, value) = match out {
            OutputEvent::KeyDown(k) | OutputEvent::KeyRepeat(k) => (k, VALUE_PRESS),
            OutputEvent::KeyUp(k) => (k, VALUE_RELEASE),
        };

        // Caps Lock LED/state via IOKit (virtual HID has no physical LED).
        if name.as_str() == "caps_lock" {
            if value == VALUE_PRESS {
                // Toggle physical caps if someone remaps TO caps — rare.
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
            code,
            device_hash: 0,
        };
        // For repeats, prefer value=2 when supported.
        if matches!(out, OutputEvent::KeyRepeat(_)) {
            event.value = VALUE_REPEAT;
        }

        let rc = send_key(&mut event);
        if rc == 2 {
            eprintln!("warning: virtual keyboard sink not ready");
        } else if rc == 1 {
            eprintln!("warning: unrecognized HID usage for {name}");
        }
    }
}

// Silence unused import if KeyName only used via methods in some builds.
#[allow(dead_code)]
fn _touch_keyname(_: &KeyName) {}

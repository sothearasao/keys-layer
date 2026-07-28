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
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use karabiner_driverkit::{
    driver_activated, fetch_devices, grab, is_sink_ready, register_device, regrab_input,
    release, release_input_only, send_key, wait_key, DKEvent,
};
use keys_layer_core::{load_config, Engine, InputEvent, KeyName, OutputEvent};

use super::caps_lock;
use super::hid_usage::{key_name_to_usage, usage_to_key_name, PAGE_KEYBOARD};
use super::media_keys;
use super::reload::{self, ReloadHandles};

const VALUE_RELEASE: u64 = 0;
const VALUE_PRESS: u64 = 1;
const VALUE_REPEAT: u64 = 2;

/// Set by the hotplug thread before `release_input_only`; main loop regrabs instead of exiting.
static HOTPLUG_RESEIZE: AtomicBool = AtomicBool::new(false);

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
    let f_row_media_hashes = Arc::new(Mutex::new(device_hashes_matching(
        &config.settings.f_row_media_devices,
    )));
    {
        let n = f_row_media_hashes.lock().expect("media lock").len();
        if n > 0 {
            eprintln!("F-row media (Fn/Globe) enabled for {n} device(s)");
        }
    }
    let devices = Arc::new(Mutex::new(device_patterns.clone()));
    let f_row_media_devices = Arc::new(Mutex::new(config.settings.f_row_media_devices.clone()));
    let suppress_native_caps = config.is_native_disabled(&KeyName::new("caps_lock"));
    let engine = Arc::new(Mutex::new(Engine::new(config)));
    let started = Instant::now();

    // Only clear Caps Lock when config asks to suppress it; otherwise leave state alone.
    if suppress_native_caps {
        caps_lock::force_caps_lock_off();
    }

    let (seized, initial_hashes) = seize_devices(&device_patterns)?;
    eprintln!("seized keyboards: {}", seized.join(", "));
    let seized_hashes = Arc::new(Mutex::new(initial_hashes));

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

    start_hotplug_watcher(
        Arc::clone(&devices),
        Arc::clone(&f_row_media_devices),
        Arc::clone(&f_row_media_hashes),
        Arc::clone(&seized_hashes),
    );

    reload::start(
        config_path.to_path_buf(),
        Arc::clone(&engine),
        ReloadHandles {
            f_row_media_hashes: Arc::clone(&f_row_media_hashes),
            f_row_media_devices: Arc::clone(&f_row_media_devices),
            devices: Arc::clone(&devices),
        },
    );

    eprintln!(
        "keys-layer (DriverKit) running — {}\n\
         Hold F / Caps for layers. Requires sudo + Karabiner VirtualHIDDevice.\n\
         Config hot-reloads on save (or SIGHUP). New keyboards are seized automatically.\n\
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
            if HOTPLUG_RESEIZE.swap(false, Ordering::SeqCst) {
                if regrab_input() {
                    eprintln!("hotplug: reseized keyboards");
                    continue;
                }
                return Err(
                    "hotplug regrab failed — restart keys-layer \
                     (sudo launchctl kickstart -k system/local.keys-layer)"
                        .into(),
                );
            }
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
            let media_device = f_row_media_hashes
                .lock()
                .expect("media lock")
                .contains(&event.device_hash);
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
                // Physical Caps is seized — block OS toggle only when native=disable.
                // With native=enable, a quick tap emits caps_lock (IOHID toggle below).
                if key.as_str() == "caps_lock" && eng.is_native_disabled(&key) {
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
/// Empty `patterns` → no matches (used for `f_row_media_devices = []`).
pub(super) fn device_hashes_matching(patterns: &[String]) -> HashSet<u64> {
    if patterns.is_empty() {
        return HashSet::new();
    }
    matching_devices(patterns)
        .into_iter()
        .map(|d| d.hash)
        .collect()
}

fn is_virtual_hid_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.contains("karabiner") || lower.contains("virtualhid")
}

fn matching_devices(patterns: &[String]) -> Vec<karabiner_driverkit::DeviceData> {
    let available = fetch_devices()
        .into_iter()
        .filter(|d| !is_virtual_hid_name(&d.product_key))
        .collect::<Vec<_>>();
    if patterns.is_empty() {
        return available;
    }
    available
        .into_iter()
        .filter(|d| {
            let name = d.product_key.to_lowercase();
            patterns
                .iter()
                .any(|p| name.contains(&p.to_lowercase()))
        })
        .collect()
}

/// Register keyboards to seize.
/// Empty `patterns` → all keyboards. Otherwise match product-name substrings.
/// Returns (human-readable names, device hashes registered).
fn seize_devices(patterns: &[String]) -> Result<(Vec<String>, HashSet<u64>), String> {
    let matched = matching_devices(patterns);
    if matched.is_empty() {
        let available = fetch_devices()
            .into_iter()
            .filter(|d| !is_virtual_hid_name(&d.product_key))
            .collect::<Vec<_>>();
        if available.is_empty() {
            return Err(
                "no HID keyboards found (is the VirtualHIDDevice daemon running?)".into(),
            );
        }
        if !patterns.is_empty() {
            let names: Vec<_> = available.iter().map(|d| d.product_key.as_str()).collect();
            return Err(format!(
                "no keyboards matched devices = {patterns:?}\n\
                 Connected: {}\n\
                 Tip: use a product-name substring, e.g. devices = [\"Moonlander\"]",
                names.join(", ")
            ));
        }
    }

    if patterns.is_empty() {
        if !register_device("") {
            return Err(
                "failed to register keyboards with Karabiner DriverKit \
                 (is the VirtualHIDDevice daemon running?)"
                    .into(),
            );
        }
        let hashes: HashSet<u64> = matched.iter().map(|d| d.hash).collect();
        let names: Vec<String> = matched.iter().map(|d| d.product_key.clone()).collect();
        let label = if names.is_empty() {
            vec!["(all keyboards)".into()]
        } else {
            names
        };
        return Ok((label, hashes));
    }

    let mut seized_names = Vec::new();
    let mut hashes = HashSet::new();
    for device in &matched {
        let hash_id = format!("0x{:X}", device.hash);
        let ok = register_device(&hash_id) || register_device(&device.product_key);
        if ok {
            hashes.insert(device.hash);
            if !seized_names.iter().any(|s| s == &device.product_key) {
                seized_names.push(device.product_key.clone());
            }
        }
    }

    if seized_names.is_empty() {
        return Err(format!(
            "failed to register devices matching {patterns:?}"
        ));
    }

    Ok((seized_names, hashes))
}

/// Watch for newly connected keyboards and reseize without restarting the process.
fn start_hotplug_watcher(
    devices: Arc<Mutex<Vec<String>>>,
    f_row_media_devices: Arc<Mutex<Vec<String>>>,
    f_row_media_hashes: Arc<Mutex<HashSet<u64>>>,
    seized_hashes: Arc<Mutex<HashSet<u64>>>,
) {
    thread::spawn(move || {
        // Track by product name so hash churn / VirtualHID rebuilds do not loop.
        let mut known_names: HashSet<String> = {
            let matched = {
                let patterns = devices.lock().expect("devices lock").clone();
                matching_devices(&patterns)
            };
            matched.into_iter().map(|d| d.product_key).collect()
        };

        loop {
            thread::sleep(Duration::from_secs(2));

            let patterns = devices.lock().expect("devices lock").clone();
            let matched = matching_devices(&patterns);

            let newcomers: Vec<(u64, String)> = matched
                .iter()
                .filter(|d| !known_names.contains(&d.product_key))
                .map(|d| (d.hash, d.product_key.clone()))
                .collect();
            if newcomers.is_empty() {
                // Keep hash set in sync for devices we already know by name.
                let mut known = seized_hashes.lock().expect("seized lock");
                for d in &matched {
                    known.insert(d.hash);
                }
                continue;
            }

            let mut known = seized_hashes.lock().expect("seized lock");
            let mut new_names = Vec::new();

            if patterns.is_empty() {
                if !register_device("") {
                    eprintln!("hotplug: register_device(all) failed");
                    continue;
                }
                for (hash, name) in &newcomers {
                    known.insert(*hash);
                    known_names.insert(name.clone());
                    new_names.push(name.clone());
                }
            } else {
                for (hash, name) in &newcomers {
                    let hash_id = format!("0x{:X}", hash);
                    let ok = register_device(&hash_id) || register_device(name);
                    if ok {
                        known.insert(*hash);
                        known_names.insert(name.clone());
                        new_names.push(name.clone());
                    }
                }
            }
            drop(known);

            if new_names.is_empty() {
                continue;
            }

            new_names.sort();
            new_names.dedup();
            eprintln!(
                "hotplug: new keyboard(s) — {} (reseizing…)",
                new_names.join(", ")
            );

            let media_patterns = f_row_media_devices.lock().expect("media devices").clone();
            *f_row_media_hashes.lock().expect("media hashes") =
                device_hashes_matching(&media_patterns);

            HOTPLUG_RESEIZE.store(true, Ordering::SeqCst);
            release_input_only();
            // Cooldown so a flaky regrab cannot spin.
            thread::sleep(Duration::from_secs(3));
        }
    });
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

pub(super) fn emit_outputs(outputs: &[OutputEvent]) {
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

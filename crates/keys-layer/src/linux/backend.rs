//! Linux backend — exclusive evdev grab + uinput virtual keyboard.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use evdev::uinput::VirtualDevice;
use evdev::{
    AttributeSet, Device, EventSummary, EventType, InputEvent, KeyCode, LedCode,
};
use keys_layer_core::{load_config, Engine, InputEvent as KlInput, OutputEvent};

use super::keymap::{all_output_keycodes, keycode_to_name, name_to_keycode};
use super::media::{self, is_function_row, media_keycode_for_f, want_media_for_f_row};
use super::reload;

const VIRTUAL_NAME: &str = "keys-layer virtual keyboard";

enum PhysEvent {
    Key {
        code: KeyCode,
        value: i32,
        f_row_media: bool,
    },
    Disconnected {
        name: String,
        path: String,
    },
}

struct Shared {
    device_patterns: Arc<Mutex<Vec<String>>>,
    f_row_media_devices: Arc<Mutex<Vec<String>>>,
    known_paths: Arc<Mutex<HashSet<String>>>,
    caps_on: Arc<AtomicBool>,
    event_tx: mpsc::Sender<PhysEvent>,
}

/// Run the Linux remapper until the process exits.
pub fn run(config_path: &Path) -> Result<(), String> {
    let config = load_config(config_path).map_err(|e| e.to_string())?;
    let device_patterns = config.settings.devices.clone();
    let f_row_media_devices = config.settings.f_row_media_devices.clone();
    let engine = Arc::new(Mutex::new(Engine::new(config)));
    let started = Instant::now();
    let caps_on = Arc::new(AtomicBool::new(false));

    let mut virtual_keys = AttributeSet::<KeyCode>::new();
    for code in all_output_keycodes() {
        virtual_keys.insert(code);
    }

    let sink = VirtualDevice::builder()
        .map_err(|e| {
            format!(
                "open /dev/uinput failed: {e}\n\
                 Need write access to /dev/uinput (root, or udev rule)."
            )
        })?
        .name(VIRTUAL_NAME)
        .with_keys(&virtual_keys)
        .map_err(|e| format!("uinput with_keys failed: {e}"))?
        .build()
        .map_err(|e| format!("create uinput device failed: {e}"))?;

    let sink = Arc::new(Mutex::new(sink));
    thread::sleep(Duration::from_millis(200));

    let (tx, rx) = mpsc::channel::<PhysEvent>();
    let known_paths = Arc::new(Mutex::new(HashSet::new()));
    let shared = Arc::new(Shared {
        device_patterns: Arc::new(Mutex::new(device_patterns)),
        f_row_media_devices: Arc::new(Mutex::new(f_row_media_devices)),
        known_paths: Arc::clone(&known_paths),
        caps_on: Arc::clone(&caps_on),
        event_tx: tx,
    });

    // Initial grab (fail if nothing found at start).
    let initial = scan_and_grab(&shared, true)?;
    eprintln!("seized keyboards: {}", initial.join(", "));

    start_hotplug_watcher(Arc::clone(&shared));

    reload::start(
        config_path.to_path_buf(),
        Arc::clone(&engine),
        Arc::clone(&sink),
        Arc::clone(&caps_on),
        Arc::clone(&shared.device_patterns),
        Arc::clone(&shared.f_row_media_devices),
    );

    eprintln!(
        "keys-layer (Linux/evdev) running — {}\n\
         Grabbed keyboards → engine → uinput. Hot-plug + config hot-reload enabled.\n\
         Ctrl-C to quit.",
        config_path.display()
    );

    loop {
        match rx.recv_timeout(Duration::from_millis(10)) {
            Ok(PhysEvent::Key {
                code,
                value,
                f_row_media,
            }) => {
                let now_ms = started.elapsed().as_millis() as u64;
                handle_key(
                    &engine,
                    &sink,
                    &caps_on,
                    code,
                    value,
                    f_row_media,
                    now_ms,
                )?;
            }
            Ok(PhysEvent::Disconnected { name, path }) => {
                eprintln!("input device lost: {name} ({path})");
                shared.known_paths.lock().expect("paths").remove(&path);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                let now_ms = started.elapsed().as_millis() as u64;
                let outputs = {
                    let mut eng = engine.lock().expect("engine lock");
                    eng.tick(now_ms)
                };
                emit_to_sink(&sink, &caps_on, &outputs)?;
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err("event channel disconnected".into());
            }
        }
    }
}

fn handle_key(
    engine: &Arc<Mutex<Engine>>,
    sink: &Arc<Mutex<VirtualDevice>>,
    caps_on: &Arc<AtomicBool>,
    code: KeyCode,
    value: i32,
    f_row_media: bool,
    now_ms: u64,
) -> Result<(), String> {
    if !(0..=2).contains(&value) {
        return Ok(());
    }

    if media::note_fn_key(code, value) {
        // Fn is a mode switch only — do not inject into the OS.
        return Ok(());
    }

    if f_row_media && is_function_row(code) && want_media_for_f_row() {
        if let Some(media_code) = media_keycode_for_f(code) {
            emit_raw(sink, media_code, value)?;
            return Ok(());
        }
    }

    let Some(key) = keycode_to_name(code) else {
        emit_raw(sink, code, value)?;
        return Ok(());
    };

    let input = match value {
        0 => KlInput::KeyUp(key.clone()),
        _ => KlInput::KeyDown(key.clone()),
    };

    let outputs = {
        let mut eng = engine.lock().expect("engine lock");
        if !(eng.should_intercept(&key) || eng.is_native_disabled(&key)) {
            None
        } else {
            Some(eng.handle(input, now_ms))
        }
    };

    match outputs {
        None => emit_raw(sink, code, value)?,
        Some(outputs) => emit_to_sink(sink, caps_on, &outputs)?,
    }
    Ok(())
}

fn emit_raw(
    sink: &Arc<Mutex<VirtualDevice>>,
    code: KeyCode,
    value: i32,
) -> Result<(), String> {
    let ev = InputEvent::new(EventType::KEY.0, code.0, value);
    sink.lock()
        .expect("sink lock")
        .emit(&[ev])
        .map_err(|e| format!("uinput emit failed: {e}"))
}

pub(super) fn emit_to_sink(
    sink: &Arc<Mutex<VirtualDevice>>,
    caps_on: &Arc<AtomicBool>,
    outputs: &[OutputEvent],
) -> Result<(), String> {
    for out in outputs {
        let (name, value) = match out {
            OutputEvent::KeyDown(k) | OutputEvent::KeyRepeat(k) => (k, 1),
            OutputEvent::KeyUp(k) => (k, 0),
        };
        let Some(code) = name_to_keycode(name) else {
            eprintln!("warning: unknown output key: {name}");
            continue;
        };

        // Toggle Caps Lock LED state on press (matches typical OS behavior).
        if code == KeyCode::KEY_CAPSLOCK && value == 1 {
            let next = !caps_on.load(Ordering::Relaxed);
            caps_on.store(next, Ordering::Relaxed);
        }

        emit_raw(sink, code, value)?;
    }
    Ok(())
}

/// Used by the reload thread.
pub(super) fn emit_to_sink_reload(
    sink: &Arc<Mutex<VirtualDevice>>,
    caps_on: &Arc<AtomicBool>,
    outputs: &[OutputEvent],
) -> Result<(), String> {
    emit_to_sink(sink, caps_on, outputs)
}

fn name_matches_patterns(name: &str, patterns: &[String]) -> bool {
    if patterns.is_empty() {
        return true;
    }
    let lower = name.to_ascii_lowercase();
    patterns
        .iter()
        .any(|p| lower.contains(&p.to_ascii_lowercase()))
}

fn is_f_row_media_device(name: &str, patterns: &[String]) -> bool {
    if patterns.is_empty() {
        return false;
    }
    let lower = name.to_ascii_lowercase();
    patterns
        .iter()
        .any(|p| lower.contains(&p.to_ascii_lowercase()))
}

fn looks_like_keyboard(device: &Device) -> bool {
    let Some(keys) = device.supported_keys() else {
        return false;
    };
    keys.contains(KeyCode::KEY_ENTER)
        && (keys.contains(KeyCode::KEY_A) || keys.contains(KeyCode::KEY_SPACE))
}

fn should_skip_device(name: &str) -> bool {
    name == VIRTUAL_NAME || name.to_ascii_lowercase().contains("keys-layer")
}

fn scan_and_grab(shared: &Shared, require_some: bool) -> Result<Vec<String>, String> {
    let patterns = shared.device_patterns.lock().expect("patterns").clone();
    let f_row_patterns = shared
        .f_row_media_devices
        .lock()
        .expect("f_row")
        .clone();
    let mut known = shared.known_paths.lock().expect("paths");
    let mut newly = Vec::new();

    for (path, device) in evdev::enumerate() {
        let path_key = path.display().to_string();
        let name = device.name().unwrap_or("unknown").to_string();
        if should_skip_device(&name) || !looks_like_keyboard(&device) {
            continue;
        }
        if !name_matches_patterns(&name, &patterns) {
            continue;
        }
        if known.contains(&path_key) {
            continue;
        }

        let f_row_media = is_f_row_media_device(&name, &f_row_patterns);
        match spawn_device_reader(
            path,
            device,
            name.clone(),
            path_key.clone(),
            f_row_media,
            Arc::clone(&shared.caps_on),
            shared.event_tx.clone(),
        ) {
            Ok(()) => {
                known.insert(path_key);
                newly.push(name);
            }
            Err(err) => eprintln!("warning: {err}"),
        }
    }

    if require_some && known.is_empty() {
        return Err(format!(
            "no keyboards grabbed.\n\
             Patterns: {patterns:?}\n\
             Need read access to /dev/input/event* and EVIOCGRAB.\n\
             Tip: add your user to the `input` group, or run as root.\n\
             Tip: devices = [\"Moonlander\"] to match a product-name substring."
        ));
    }

    Ok(newly)
}

fn spawn_device_reader(
    path: PathBuf,
    mut device: Device,
    name: String,
    path_key: String,
    f_row_media: bool,
    caps_on: Arc<AtomicBool>,
    tx: mpsc::Sender<PhysEvent>,
) -> Result<(), String> {
    device
        .grab()
        .map_err(|e| format!("could not grab {name} ({path:?}): {e}"))?;
    device
        .set_nonblocking(true)
        .map_err(|e| format!("set_nonblocking {name}: {e}"))?;

    eprintln!(
        "grabbed: {name} ({}){}",
        path.display(),
        if f_row_media {
            " [F-row media]"
        } else {
            ""
        }
    );

    thread::spawn(move || {
        let mut led_on = false;
        loop {
            let want = caps_on.load(Ordering::Relaxed);
            if want != led_on {
                let led = InputEvent::new(EventType::LED.0, LedCode::LED_CAPSL.0, i32::from(want));
                if device.send_events(&[led]).is_ok() {
                    led_on = want;
                }
            }

            match device.fetch_events() {
                Ok(events) => {
                    for ev in events {
                        if let EventSummary::Key(_, code, value) = ev.destructure() {
                            if tx
                                .send(PhysEvent::Key {
                                    code,
                                    value,
                                    f_row_media,
                                })
                                .is_err()
                            {
                                return;
                            }
                        }
                    }
                }
                Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(5));
                }
                Err(err) => {
                    eprintln!("input device lost ({name}): {err}");
                    let _ = tx.send(PhysEvent::Disconnected {
                        name,
                        path: path_key,
                    });
                    return;
                }
            }
        }
    });

    Ok(())
}

fn start_hotplug_watcher(shared: Arc<Shared>) {
    thread::spawn(move || loop {
        thread::sleep(Duration::from_secs(2));
        match scan_and_grab(&shared, false) {
            Ok(new) if !new.is_empty() => {
                eprintln!("hotplug: new keyboard(s) — {}", new.join(", "));
            }
            Ok(_) => {}
            Err(err) => eprintln!("hotplug: {err}"),
        }
    });
}

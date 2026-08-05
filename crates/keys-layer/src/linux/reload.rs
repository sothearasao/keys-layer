//! Config hot-reload for Linux (mtime watch + SIGHUP).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime};

use evdev::uinput::VirtualDevice;
use keys_layer_core::{load_config, Engine};

use super::backend::emit_to_sink_reload;

static RELOAD_REQUESTED: AtomicBool = AtomicBool::new(false);

extern "C" fn on_sighup(_: libc::c_int) {
    RELOAD_REQUESTED.store(true, Ordering::SeqCst);
}

/// Install SIGHUP handler and spawn a watcher that reloads on file change or signal.
pub fn start(
    config_path: PathBuf,
    engine: Arc<Mutex<Engine>>,
    sink: Arc<Mutex<VirtualDevice>>,
    caps_on: Arc<AtomicBool>,
    device_patterns: Arc<Mutex<Vec<String>>>,
    f_row_media_devices: Arc<Mutex<Vec<String>>>,
) {
    unsafe {
        libc::signal(
            libc::SIGHUP,
            on_sighup as *const () as libc::sighandler_t,
        );
    }

    thread::spawn(move || {
        let mut last_mtime = file_mtime(&config_path);
        loop {
            thread::sleep(Duration::from_millis(500));

            let signal = RELOAD_REQUESTED.swap(false, Ordering::SeqCst);
            let mtime = file_mtime(&config_path);
            let file_changed = match (last_mtime, mtime) {
                (Some(prev), Some(next)) if next != prev => true,
                (None, Some(_)) => true,
                _ => false,
            };

            if !signal && !file_changed {
                continue;
            }

            if file_changed {
                thread::sleep(Duration::from_millis(300));
                let stable = file_mtime(&config_path);
                if stable != mtime && stable.is_some() {
                    last_mtime = stable;
                    continue;
                }
                last_mtime = stable.or(mtime);
            }

            try_reload(
                &config_path,
                &engine,
                &sink,
                &caps_on,
                &device_patterns,
                &f_row_media_devices,
            );
        }
    });
}

fn file_mtime(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

fn try_reload(
    config_path: &Path,
    engine: &Arc<Mutex<Engine>>,
    sink: &Arc<Mutex<VirtualDevice>>,
    caps_on: &Arc<AtomicBool>,
    device_patterns: &Arc<Mutex<Vec<String>>>,
    f_row_media_devices: &Arc<Mutex<Vec<String>>>,
) {
    eprintln!("reloading config: {}", config_path.display());

    let config = match load_config(config_path) {
        Ok(c) => c,
        Err(err) => {
            eprintln!(
                "config reload FAILED — keeping previous config\n\
                   error: {err}\n\
                   file:  {}",
                config_path.display()
            );
            return;
        }
    };

    let new_devices = config.settings.devices.clone();
    let new_media = config.settings.f_row_media_devices.clone();

    let releases = {
        let mut eng = engine.lock().expect("engine lock");
        eng.reload(config)
    };

    if let Err(err) = emit_to_sink_reload(sink, caps_on, &releases) {
        eprintln!("reload: failed to emit key releases: {err}");
    }

    *device_patterns.lock().expect("devices") = new_devices;
    *f_row_media_devices.lock().expect("f_row") = new_media;

    eprintln!("config reloaded OK — {}", config_path.display());
    eprintln!(
        "note: newly matching keyboards are seized by hot-plug within ~2s; \
         F-row media list applies to newly grabbed boards"
    );
}

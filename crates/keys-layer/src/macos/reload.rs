//! Config hot-reload: file mtime watch + SIGHUP.
//!
//! On success → `Engine::reload` and update runtime settings.
//! On failure → log a clear error and keep the previous config.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime};

use keys_layer_core::{load_config, Engine, KeyName};

use super::caps_lock;
use super::driverkit::{device_hashes_matching, emit_outputs};

static RELOAD_REQUESTED: AtomicBool = AtomicBool::new(false);

extern "C" fn on_sighup(_: libc::c_int) {
    RELOAD_REQUESTED.store(true, Ordering::SeqCst);
}

pub struct ReloadHandles {
    pub f_row_media_hashes: Arc<Mutex<std::collections::HashSet<u64>>>,
    pub devices: Arc<Mutex<Vec<String>>>,
}

/// Install SIGHUP handler and spawn a watcher that reloads on file change or signal.
pub fn start(
    config_path: PathBuf,
    engine: Arc<Mutex<Engine>>,
    handles: ReloadHandles,
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
                // Debounce editors that rewrite the file in multiple steps.
                thread::sleep(Duration::from_millis(300));
                let stable = file_mtime(&config_path);
                if stable != mtime && stable.is_some() {
                    // Still changing; pick up on next loop.
                    last_mtime = stable;
                    continue;
                }
                last_mtime = stable.or(mtime);
            }

            try_reload(&config_path, &engine, &handles);
        }
    });
}

fn file_mtime(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

fn try_reload(
    config_path: &Path,
    engine: &Arc<Mutex<Engine>>,
    handles: &ReloadHandles,
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
    let new_media = device_hashes_matching(&config.settings.f_row_media_devices);
    let suppress_caps = config.is_native_disabled(&KeyName::new("caps_lock"));

    let devices_changed = {
        let old = handles.devices.lock().expect("devices lock");
        *old != new_devices
    };

    let releases = {
        let mut eng = engine.lock().expect("engine lock");
        eng.reload(config)
    };
    emit_outputs(&releases);

    *handles.f_row_media_hashes.lock().expect("media lock") = new_media;
    *handles.devices.lock().expect("devices lock") = new_devices;

    if suppress_caps {
        caps_lock::force_caps_lock_off();
    }

    eprintln!("config reloaded OK — {}", config_path.display());
    if devices_changed {
        eprintln!(
            "warning: settings.devices changed — restart keys-layer to reseize keyboards \
             (hot-reload does not change which devices are grabbed)"
        );
    }
}

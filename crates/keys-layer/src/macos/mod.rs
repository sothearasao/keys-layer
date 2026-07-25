//! macOS CGEvent tap backend.

mod caps_lock;
mod hid_caps;
mod keycode;
mod tap;

use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use keys_layer_core::{load_config, Engine};

use self::hid_caps::CapsHidMonitor;
use self::tap::{CapsPhysical, EventTap};

/// Load config and run the event tap until the process exits.
pub fn run(config_path: &Path) -> Result<(), String> {
    let config = load_config(config_path).map_err(|e| e.to_string())?;
    let engine = Arc::new(Mutex::new(Engine::new(config)));
    let started = Instant::now();
    let caps = Arc::new(CapsPhysical::new());

    caps_lock::force_caps_lock_off();

    let engine_for_timer = Arc::clone(&engine);
    let started_for_timer = started;
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_millis(10));
        let now_ms = started_for_timer.elapsed().as_millis() as u64;
        let outputs = {
            let mut eng = engine_for_timer.lock().expect("engine lock");
            eng.tick(now_ms)
        };
        if !outputs.is_empty() {
            tap::emit_outputs(&outputs);
        }
    });

    // HID must be scheduled on this thread's run loop (same as the event tap).
    let hid = match CapsHidMonitor::start(Arc::clone(&engine), started, Arc::clone(&caps)) {
        Ok(monitor) => {
            eprintln!(
                "keys-layer running with config {}\n\
                 Hold F ≥200ms, then J → Delete.\n\
                 Hold Caps ≥200ms, then hjkl → arrows; release Caps to leave.\n\
                 Requires Accessibility + Input Monitoring permissions.\n\
                 Ctrl-C to quit.",
                config_path.display()
            );
            Some(monitor)
        }
        Err(err) => {
            eprintln!("warning: {err}");
            eprintln!(
                "keys-layer running (Caps release may be unreliable without HID) — {}\n\
                 Ctrl-C to quit.",
                config_path.display()
            );
            None
        }
    };

    let hid_drives_caps = hid.is_some();
    let tap = EventTap::new(engine, started, caps, hid_drives_caps)?;
    // Keep HID monitor alive for the run loop lifetime.
    let _hid = hid;
    tap.run()
}

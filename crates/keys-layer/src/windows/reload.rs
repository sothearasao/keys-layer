//! Config hot-reload for Windows (mtime watch).

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, SystemTime};

use keys_layer_core::load_config;

use super::backend::{self, WinState};

/// Spawn a watcher that reloads on file change.
pub fn start(config_path: PathBuf, _state: Arc<WinState>) {
    thread::spawn(move || {
        let mut last_mtime = file_mtime(&config_path);
        loop {
            thread::sleep(Duration::from_millis(500));
            let mtime = file_mtime(&config_path);
            let file_changed = match (last_mtime, mtime) {
                (Some(prev), Some(next)) if next != prev => true,
                (None, Some(_)) => true,
                _ => false,
            };
            if !file_changed {
                continue;
            }
            thread::sleep(Duration::from_millis(300));
            let stable = file_mtime(&config_path);
            if stable != mtime && stable.is_some() {
                last_mtime = stable;
                continue;
            }
            last_mtime = stable.or(mtime);
            try_reload(&config_path);
        }
    });
}

fn file_mtime(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

fn try_reload(config_path: &Path) {
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
    if !config.settings.devices.is_empty() {
        eprintln!("warning: settings.devices is ignored on Windows LLHOOK backend");
    }
    let releases = backend::reload_engine(config);
    backend::emit_outputs_pub(&releases);
    eprintln!("config reloaded OK — {}", config_path.display());
}

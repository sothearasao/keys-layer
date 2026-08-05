//! Windows backend — `WH_KEYBOARD_LL` + `SendInput`.

mod backend;
mod keymap;
mod reload;
mod stdio;

use std::path::Path;

/// Attach parent console or redirect logs to a file (no console flash on autostart).
pub fn init_process_io() {
    stdio::init();
}

/// Load config and run the Windows remapper until exit.
pub fn run(config_path: &Path) -> Result<(), String> {
    backend::run(config_path)
}

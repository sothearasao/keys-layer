//! Windows backend — `WH_KEYBOARD_LL` + `SendInput`.

mod backend;
mod keymap;
mod reload;

use std::path::Path;

/// Load config and run the Windows remapper until exit.
pub fn run(config_path: &Path) -> Result<(), String> {
    backend::run(config_path)
}

//! Linux backend — evdev grab + uinput virtual keyboard.

mod backend;
mod keymap;
mod media;
mod reload;

use std::path::Path;

/// Load config and run the Linux remapper until exit.
pub fn run(config_path: &Path) -> Result<(), String> {
    backend::run(config_path)
}

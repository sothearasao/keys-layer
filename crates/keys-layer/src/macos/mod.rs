//! macOS backend — Karabiner DriverKit VirtualHIDDevice.

mod caps_lock;
mod driverkit;
mod hid_usage;
mod media_keys;
mod reload;

use std::path::Path;

/// Load config and run the DriverKit remapper until exit.
pub fn run(config_path: &Path) -> Result<(), String> {
    driverkit::run(config_path)
}

/// Print connected keyboard-class HID product names (for `settings.devices`).
pub fn list_devices() -> Result<(), String> {
    driverkit::list_devices()
}

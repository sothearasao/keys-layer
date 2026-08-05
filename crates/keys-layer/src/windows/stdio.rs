//! Attach to parent console when launched from a terminal; otherwise log to a file.
//! Used with `#![windows_subsystem = "windows"]` so Scheduled Task / autostart
//! does not flash a console window.

use std::env;
use std::fs::{self, OpenOptions};
use std::os::windows::io::AsRawHandle;
use std::path::PathBuf;

use windows_sys::Win32::Foundation::HANDLE;
use windows_sys::Win32::System::Console::{
    AttachConsole, SetStdHandle, ATTACH_PARENT_PROCESS, STD_ERROR_HANDLE, STD_OUTPUT_HANDLE,
};

fn default_log_path() -> PathBuf {
    let base = env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .or_else(|| env::var_os("APPDATA").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("keys-layer").join("keys-layer.log")
}

fn set_stdio_handles(handle: HANDLE) {
    unsafe {
        SetStdHandle(STD_OUTPUT_HANDLE, handle);
        SetStdHandle(STD_ERROR_HANDLE, handle);
    }
}

/// Call once at process start, before any `eprintln!`.
pub fn init() {
    unsafe {
        if AttachConsole(ATTACH_PARENT_PROCESS) != 0 {
            if let Ok(out) = OpenOptions::new().write(true).open("CONOUT$") {
                set_stdio_handles(out.as_raw_handle() as HANDLE);
                std::mem::forget(out);
            }
            return;
        }
    }

    let log_path = default_log_path();
    if let Some(parent) = log_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
    {
        set_stdio_handles(file.as_raw_handle() as HANDLE);
        std::mem::forget(file);
    }
}

pub fn log_path_display() -> String {
    default_log_path().display().to_string()
}

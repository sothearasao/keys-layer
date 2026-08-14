//! keys-layer CLI — hold-to-layer keyboard remapper.

#![cfg_attr(windows, windows_subsystem = "windows")]

use std::env;
use std::path::PathBuf;
use std::process;

fn main() {
    #[cfg(windows)]
    keys_layer::windows::init_process_io();

    let mut args = env::args().skip(1);
    let first = args.next();

    match first.as_deref() {
        Some("-h") | Some("--help") => {
            print_usage();
            process::exit(0);
        }
        Some("--list-devices") => {
            #[cfg(target_os = "macos")]
            {
                if let Err(err) = keys_layer::macos::list_devices() {
                    eprintln!("error: {err}");
                    process::exit(1);
                }
                return;
            }
            #[cfg(target_os = "linux")]
            {
                if let Err(err) = keys_layer::linux::list_devices() {
                    eprintln!("error: {err}");
                    process::exit(1);
                }
                return;
            }
            #[cfg(target_os = "windows")]
            {
                eprintln!(
                    "settings.devices is ignored on Windows (system-wide LLHOOK).\n\
                     No per-device list is available."
                );
                process::exit(0);
            }
            #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
            {
                eprintln!("unsupported platform");
                process::exit(1);
            }
        }
        _ => {}
    }

    let config_path = match first {
        Some(p) => PathBuf::from(p),
        None => default_config_path().unwrap_or_else(|| {
            print_usage();
            eprintln!(
                "\nNo config given and no default found.\n\
                 Put a config at:\n\
                   macOS/Linux: ~/.config/keys-layer/config.toml\n\
                   Windows:     %APPDATA%\\keys-layer\\config.toml\n\
                 (copy from config.example.toml in the repo)."
            );
            process::exit(2);
        }),
    };

    if !config_path.is_file() {
        eprintln!("error: config not found: {}", config_path.display());
        process::exit(2);
    }

    #[cfg(target_os = "macos")]
    {
        if let Err(err) = keys_layer::macos::run(&config_path) {
            eprintln!("error: {err}");
            process::exit(1);
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let Err(err) = keys_layer::linux::run(&config_path) {
            eprintln!("error: {err}");
            process::exit(1);
        }
    }

    #[cfg(target_os = "windows")]
    {
        if let Err(err) = keys_layer::windows::run(&config_path) {
            eprintln!("error: {err}");
            process::exit(1);
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        let _ = config_path;
        eprintln!("keys-layer currently supports macOS, Linux, and Windows.");
        process::exit(1);
    }
}

fn print_usage() {
    eprintln!(
        "usage: keys-layer [config.toml]\n\
         \n\
         options:\n\
           --list-devices   print connected keyboard names for settings.devices\n\
                            (macOS: sudo; Linux: input group or root)\n\
           -h, --help       show this help"
    );
}

fn default_config_path() -> Option<PathBuf> {
    let mut candidates = Vec::new();

    #[cfg(windows)]
    {
        if let Some(appdata) = env::var_os("APPDATA") {
            candidates.push(PathBuf::from(appdata).join("keys-layer").join("config.toml"));
        }
        if let Some(home) = env::var_os("USERPROFILE") {
            candidates.push(
                PathBuf::from(home)
                    .join(".config")
                    .join("keys-layer")
                    .join("config.toml"),
            );
        }
    }

    #[cfg(not(windows))]
    {
        if let Some(xdg) = env::var_os("XDG_CONFIG_HOME") {
            candidates.push(PathBuf::from(xdg).join("keys-layer").join("config.toml"));
        }
        if let Some(home) = env::var_os("HOME") {
            candidates.push(
                PathBuf::from(home)
                    .join(".config")
                    .join("keys-layer")
                    .join("config.toml"),
            );
        }
    }

    candidates.into_iter().find(|p| p.is_file())
}

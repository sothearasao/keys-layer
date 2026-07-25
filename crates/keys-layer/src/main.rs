//! keys-layer CLI — hold-to-layer keyboard remapper.

use std::env;
use std::path::PathBuf;
use std::process;

fn main() {
    let mut args = env::args().skip(1);
    let config_path = match args.next() {
        Some(p) => PathBuf::from(p),
        None => default_config_path().unwrap_or_else(|| {
            eprintln!(
                "usage: keys-layer [config.toml]\n\n\
                 No config given and no default found.\n\
                 Put a config at ~/.config/keys-layer/config.toml\n\
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

    #[cfg(not(target_os = "macos"))]
    {
        let _ = config_path;
        eprintln!("keys-layer currently supports macOS only (DriverKit).");
        process::exit(1);
    }
}

fn default_config_path() -> Option<PathBuf> {
    let home = env::var_os("HOME")?;
    let path = PathBuf::from(home)
        .join(".config")
        .join("keys-layer")
        .join("config.toml");
    if path.is_file() {
        Some(path)
    } else {
        None
    }
}

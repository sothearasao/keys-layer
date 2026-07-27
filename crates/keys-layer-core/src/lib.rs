//! Platform-agnostic keyboard layer engine.
//!
//! Handles config parsing and the tap / hold-to-layer state machine.
//! Platform backends feed key events in and apply the emitted outputs.

mod config;
mod engine;
mod key;

pub use config::{
    load_config, Config, ConfigError, KeyBinding, Layer, NativeMode, OutputKeys, Settings,
};
pub use engine::{Engine, InputEvent, OutputEvent};
pub use key::KeyName;

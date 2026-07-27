//! TOML configuration loading.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::Deserialize;
use thiserror::Error;

use crate::key::KeyName;

pub const DEFAULT_HOLD_MS: u64 = 200;
pub const DEFAULT_BASE_LAYER: &str = "base";

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to read config: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to parse config: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("{0}")]
    Validation(String),
}

#[derive(Debug, Clone)]
pub struct Config {
    pub settings: Settings,
    pub layers: HashMap<String, Layer>,
}

#[derive(Debug, Clone)]
pub struct Settings {
    /// Global default hold delay; used when a layer/key does not set `hold_ms`.
    pub hold_ms: u64,
    pub base_layer: String,
    /// Product-name substrings of keyboards to seize. Empty = all keyboards.
    pub devices: Vec<String>,
    /// Product-name substrings that get Mac-style F1–F12 ↔ media (Fn/Globe).
    /// Default: `["Apple Internal"]`. Empty disables the feature.
    pub f_row_media_devices: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Layer {
    /// Optional default hold delay for hold-keys on this layer.
    pub hold_ms: Option<u64>,
    pub keys: HashMap<KeyName, KeyBinding>,
}

/// Whether the physical/native key action is allowed to fire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NativeMode {
    /// Physical key may still be used as a tap output (e.g. `tap = "f"`).
    #[default]
    Enable,
    /// Never emit the physical key; OS behavior is suppressed (e.g. Caps Lock).
    Disable,
}

/// What a remap emits when the physical key is pressed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutputKeys {
    /// One key (holdable / repeats).
    Single(KeyName),
    /// All keys down in order; up in reverse on release (e.g. Option+Delete).
    Chord(Vec<KeyName>),
    /// Full tap each key on press only (Down+Up); physical KeyUp is ignored.
    Sequence(Vec<KeyName>),
}

impl OutputKeys {
    pub fn keys(&self) -> &[KeyName] {
        match self {
            Self::Single(k) => std::slice::from_ref(k),
            Self::Chord(keys) | Self::Sequence(keys) => keys.as_slice(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum KeyBinding {
    /// Remap to one key, a chord, or a one-shot sequence.
    Remap(OutputKeys),
    /// After `hold_ms`, momentarily activate `hold` layer.
    /// Optional `tap` fires on quick release; with `native = "disable"` the
    /// physical key itself never fires.
    Hold {
        tap: Option<KeyName>,
        hold: String,
        /// Per-key override (wins over layer and global).
        hold_ms: Option<u64>,
        native: NativeMode,
    },
}

#[derive(Debug, Deserialize)]
struct RawConfig {
    #[serde(default)]
    settings: RawSettings,
    #[serde(default)]
    layer: HashMap<String, RawLayer>,
}

#[derive(Debug, Deserialize)]
struct RawSettings {
    #[serde(default = "default_hold_ms")]
    hold_ms: u64,
    #[serde(default = "default_base_layer")]
    base_layer: String,
    /// Keyboards to seize (product-name substrings). Empty = all.
    #[serde(default)]
    devices: Vec<String>,
    /// Devices that get Fn-aware F-row media (default: Apple Internal).
    #[serde(default = "default_f_row_media_devices")]
    f_row_media_devices: Vec<String>,
}

impl Default for RawSettings {
    fn default() -> Self {
        Self {
            hold_ms: DEFAULT_HOLD_MS,
            base_layer: DEFAULT_BASE_LAYER.to_string(),
            devices: Vec::new(),
            f_row_media_devices: default_f_row_media_devices(),
        }
    }
}

fn default_hold_ms() -> u64 {
    DEFAULT_HOLD_MS
}

fn default_base_layer() -> String {
    DEFAULT_BASE_LAYER.to_string()
}

fn default_f_row_media_devices() -> Vec<String> {
    vec!["Apple Internal".into()]
}

/// One layer table: optional `hold_ms`, plus key bindings flattened in.
#[derive(Debug, Deserialize)]
struct RawLayer {
    #[serde(default)]
    hold_ms: Option<u64>,
    #[serde(flatten)]
    keys: HashMap<String, RawBinding>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RawBinding {
    Simple(String),
    /// Chord: `k = ["left_alt", "delete"]`
    ChordList(Vec<String>),
    Table(RawBindingTable),
}

#[derive(Debug, Deserialize)]
struct RawBindingTable {
    /// Remap target (holdable). Alias of a bare string binding.
    #[serde(default)]
    key: Option<String>,
    #[serde(default)]
    tap: Option<String>,
    /// Chord: `{ chord = ["left_alt", "delete"] }`
    #[serde(default)]
    chord: Option<Vec<String>>,
    /// Sequence of taps: `{ sequence = ["a", "b"] }`
    #[serde(default)]
    sequence: Option<Vec<String>>,
    /// Hold-to-layer name (momentary layer).
    #[serde(default)]
    hold: Option<String>,
    #[serde(default)]
    hold_ms: Option<u64>,
    /// `"disable"` suppresses the physical/native key (recommended for Caps Lock).
    #[serde(default)]
    native: Option<String>,
}

impl Config {
    pub fn from_toml_str(text: &str) -> Result<Self, ConfigError> {
        let raw: RawConfig = toml::from_str(text)?;
        Self::from_raw(raw)
    }

    fn from_raw(raw: RawConfig) -> Result<Self, ConfigError> {
        if raw.layer.is_empty() {
            return Err(ConfigError::Validation(
                "config must define at least one [layer.*] table".into(),
            ));
        }

        if !raw.layer.contains_key(&raw.settings.base_layer) {
            return Err(ConfigError::Validation(format!(
                "base layer {:?} not found; defined layers: {:?}",
                raw.settings.base_layer,
                raw.layer.keys().collect::<Vec<_>>()
            )));
        }

        let mut layers = HashMap::new();
        for (layer_name, raw_layer) in raw.layer {
            let mut keys = HashMap::new();
            for (key_str, binding) in raw_layer.keys {
                if key_str == "hold_ms" {
                    // Should have been captured by RawLayer::hold_ms; skip if stray.
                    continue;
                }
                let key = KeyName::new(&key_str);
                let binding = match binding {
                    RawBinding::Simple(target) => {
                        KeyBinding::Remap(OutputKeys::Single(KeyName::new(target)))
                    }
                    RawBinding::ChordList(keys) => {
                        KeyBinding::Remap(parse_key_list(&layer_name, &key_str, "chord", keys)?)
                    }
                    RawBinding::Table(table) => {
                        parse_table_binding(&layer_name, &key_str, table)?
                    }
                };
                keys.insert(key, binding);
            }
            layers.insert(
                layer_name,
                Layer {
                    hold_ms: raw_layer.hold_ms,
                    keys,
                },
            );
        }

        // Validate hold targets exist.
        for (layer_name, layer) in &layers {
            for (key, binding) in &layer.keys {
                if let KeyBinding::Hold { hold, .. } = binding {
                    if !layers.contains_key(hold) {
                        return Err(ConfigError::Validation(format!(
                            "layer {layer_name:?} key {key}: hold target layer {hold:?} does not exist"
                        )));
                    }
                }
            }
        }

        Ok(Self {
            settings: Settings {
                hold_ms: raw.settings.hold_ms,
                base_layer: raw.settings.base_layer,
                devices: raw.settings.devices,
                f_row_media_devices: raw.settings.f_row_media_devices,
            },
            layers,
        })
    }

    pub fn layer(&self, name: &str) -> Option<&Layer> {
        self.layers.get(name)
    }

    pub fn binding(&self, layer: &str, key: &KeyName) -> Option<&KeyBinding> {
        self.layers.get(layer)?.keys.get(key)
    }

    /// True if this physical key is configured with `native = "disable"` on any layer.
    pub fn is_native_disabled(&self, key: &KeyName) -> bool {
        self.layers.values().any(|layer| {
            matches!(
                layer.keys.get(key),
                Some(KeyBinding::Hold {
                    native: NativeMode::Disable,
                    ..
                })
            )
        })
    }

    /// Resolve hold delay: per-key → per-layer → global settings.
    pub fn hold_ms_for(&self, layer_name: &str, binding: &KeyBinding) -> u64 {
        let key_ms = match binding {
            KeyBinding::Hold { hold_ms, .. } => *hold_ms,
            KeyBinding::Remap(_) => None,
        };
        self.resolve_hold_ms(layer_name, key_ms)
    }

    pub fn resolve_hold_ms(&self, layer_name: &str, key_hold_ms: Option<u64>) -> u64 {
        key_hold_ms
            .or_else(|| self.layers.get(layer_name).and_then(|l| l.hold_ms))
            .unwrap_or(self.settings.hold_ms)
    }
}

fn parse_key_list(
    layer_name: &str,
    key: &str,
    kind: &str,
    keys: Vec<String>,
) -> Result<OutputKeys, ConfigError> {
    if keys.is_empty() {
        return Err(ConfigError::Validation(format!(
            "layer {layer_name:?} key {key}: {kind} list must not be empty"
        )));
    }
    let names: Vec<KeyName> = keys.into_iter().map(KeyName::new).collect();
    match kind {
        "sequence" => Ok(OutputKeys::Sequence(names)),
        _ => Ok(OutputKeys::Chord(names)),
    }
}

fn parse_table_binding(
    layer_name: &str,
    key: &str,
    table: RawBindingTable,
) -> Result<KeyBinding, ConfigError> {
    let native = parse_native(layer_name, key, table.native.as_deref())?;

    // Hold-to-layer (tap optional).
    if let Some(hold) = table.hold {
        if table.chord.is_some() || table.sequence.is_some() {
            return Err(ConfigError::Validation(format!(
                "layer {layer_name:?} key {key}: chord/sequence cannot combine with hold"
            )));
        }
        let physical = KeyName::new(key);
        let tap = table.tap.map(KeyName::new).and_then(|t| {
            if native == NativeMode::Disable && t == physical {
                None
            } else {
                Some(t)
            }
        });
        return Ok(KeyBinding::Hold {
            tap,
            hold,
            hold_ms: table.hold_ms,
            native,
        });
    }

    if table.hold_ms.is_some() {
        return Err(ConfigError::Validation(format!(
            "layer {layer_name:?} key {key}: hold_ms only applies with hold = \"layer\""
        )));
    }
    if native == NativeMode::Disable {
        return Err(ConfigError::Validation(format!(
            "layer {layer_name:?} key {key}: native = \"disable\" only applies with hold = \"layer\""
        )));
    }

    let has_chord = table.chord.is_some();
    let has_seq = table.sequence.is_some();
    let has_single = table.key.is_some() || table.tap.is_some();
    if (has_chord as u8) + (has_seq as u8) + (has_single as u8) > 1 {
        return Err(ConfigError::Validation(format!(
            "layer {layer_name:?} key {key}: use only one of key/tap, chord, or sequence"
        )));
    }

    if let Some(keys) = table.chord {
        return Ok(KeyBinding::Remap(parse_key_list(
            layer_name, key, "chord", keys,
        )?));
    }
    if let Some(keys) = table.sequence {
        return Ok(KeyBinding::Remap(parse_key_list(
            layer_name, key, "sequence", keys,
        )?));
    }

    // Holdable remap: `j = { key = "delete" }` or `j = { tap = "delete" }`.
    if let Some(target) = table.key.or(table.tap) {
        return Ok(KeyBinding::Remap(OutputKeys::Single(KeyName::new(target))));
    }

    Err(ConfigError::Validation(format!(
        "layer {layer_name:?} key {key}: need `key`/`tap`, `chord = [...]`, `sequence = [...]`, \
         or `hold = \"layer\"`"
    )))
}

fn parse_native(
    layer_name: &str,
    key: &str,
    raw: Option<&str>,
) -> Result<NativeMode, ConfigError> {
    match raw {
        None | Some("enable") => Ok(NativeMode::Enable),
        Some("disable") => Ok(NativeMode::Disable),
        Some(other) => Err(ConfigError::Validation(format!(
            "layer {layer_name:?} key {key}: native must be \"enable\" or \"disable\", got {other:?}"
        ))),
    }
}

pub fn load_config(path: &Path) -> Result<Config, ConfigError> {
    let text = fs::read_to_string(path)?;
    Config::from_toml_str(&text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_demo_config() {
        let cfg = Config::from_toml_str(
            r#"
            [settings]
            hold_ms = 200

            [layer.base]
            f = { tap = "f", hold = "mod_f" }

            [layer.mod_f]
            j = "delete"
            "#,
        )
        .unwrap();

        assert_eq!(cfg.settings.hold_ms, 200);
        let f = cfg
            .binding("base", &KeyName::new("f"))
            .expect("f binding");
        match f {
            KeyBinding::Hold {
                tap,
                hold,
                native,
                ..
            } => {
                assert_eq!(tap.as_ref().map(|t| t.as_str()), Some("f"));
                assert_eq!(hold, "mod_f");
                assert_eq!(*native, NativeMode::Enable);
            }
            _ => panic!("expected hold"),
        }
        match cfg.binding("mod_f", &KeyName::new("j")).unwrap() {
            KeyBinding::Remap(OutputKeys::Single(k)) => assert_eq!(k.as_str(), "delete"),
            _ => panic!("expected remap"),
        }
    }

    #[test]
    fn parses_native_disable_and_optional_tap() {
        let cfg = Config::from_toml_str(
            r#"
            [layer.base]
            caps = { tap = "escape", hold = "mod_caps", native = "disable" }

            [layer.mod_caps]
            h = "left"
            "#,
        )
        .unwrap();

        match cfg.binding("base", &KeyName::new("caps")).unwrap() {
            KeyBinding::Hold {
                tap,
                hold,
                native,
                ..
            } => {
                assert_eq!(tap.as_ref().map(|t| t.as_str()), Some("escape"));
                assert_eq!(hold, "mod_caps");
                assert_eq!(*native, NativeMode::Disable);
            }
            _ => panic!("expected hold"),
        }
    }

    #[test]
    fn hold_only_with_native_disable() {
        let cfg = Config::from_toml_str(
            r#"
            [layer.base]
            caps = { hold = "mod_caps", native = "disable" }

            [layer.mod_caps]
            h = "left"
            "#,
        )
        .unwrap();

        match cfg.binding("base", &KeyName::new("caps")).unwrap() {
            KeyBinding::Hold { tap, native, .. } => {
                assert!(tap.is_none());
                assert_eq!(*native, NativeMode::Disable);
            }
            _ => panic!("expected hold"),
        }
    }

    #[test]
    fn layer_hold_ms_overrides_global() {
        let cfg = Config::from_toml_str(
            r#"
            [settings]
            hold_ms = 200

            [layer.base]
            hold_ms = 150
            f = { tap = "f", hold = "mod_f" }
            caps = { hold = "mod_caps", native = "disable", hold_ms = 80 }

            [layer.mod_f]
            j = "delete"

            [layer.mod_caps]
            hold_ms = 100
            h = "left"
            "#,
        )
        .unwrap();

        assert_eq!(cfg.layer("base").unwrap().hold_ms, Some(150));
        assert_eq!(cfg.layer("mod_caps").unwrap().hold_ms, Some(100));

        let f = cfg.binding("base", &KeyName::new("f")).unwrap();
        assert_eq!(cfg.hold_ms_for("base", f), 150);

        let caps = cfg.binding("base", &KeyName::new("caps")).unwrap();
        assert_eq!(cfg.hold_ms_for("base", caps), 80);
    }

    #[test]
    fn parses_holdable_layer_key() {
        let cfg = Config::from_toml_str(
            r#"
            [layer.base]
            f = { tap = "f", hold = "mod_f" }

            [layer.mod_f]
            j = { key = "delete" }
            k = { tap = "escape" }
            "#,
        )
        .unwrap();
        match cfg.binding("mod_f", &KeyName::new("j")).unwrap() {
            KeyBinding::Remap(OutputKeys::Single(k)) => assert_eq!(k.as_str(), "delete"),
            _ => panic!("expected remap"),
        }
        match cfg.binding("mod_f", &KeyName::new("k")).unwrap() {
            KeyBinding::Remap(OutputKeys::Single(k)) => assert_eq!(k.as_str(), "escape"),
            _ => panic!("expected remap"),
        }
    }

    #[test]
    fn parses_chord_and_sequence() {
        let cfg = Config::from_toml_str(
            r#"
            [layer.base]
            f = { tap = "f", hold = "mod_f" }

            [layer.mod_f]
            k = ["left_alt", "delete"]
            l = { chord = ["option", "delete"] }
            m = { sequence = ["a", "b"] }
            "#,
        )
        .unwrap();

        match cfg.binding("mod_f", &KeyName::new("k")).unwrap() {
            KeyBinding::Remap(OutputKeys::Chord(keys)) => {
                assert_eq!(
                    keys.iter().map(|k| k.as_str()).collect::<Vec<_>>(),
                    vec!["left_alt", "delete"]
                );
            }
            _ => panic!("expected chord"),
        }
        match cfg.binding("mod_f", &KeyName::new("l")).unwrap() {
            KeyBinding::Remap(OutputKeys::Chord(keys)) => {
                assert_eq!(keys[0].as_str(), "left_alt"); // option → left_alt
                assert_eq!(keys[1].as_str(), "delete");
            }
            _ => panic!("expected chord"),
        }
        match cfg.binding("mod_f", &KeyName::new("m")).unwrap() {
            KeyBinding::Remap(OutputKeys::Sequence(keys)) => {
                assert_eq!(
                    keys.iter().map(|k| k.as_str()).collect::<Vec<_>>(),
                    vec!["a", "b"]
                );
            }
            _ => panic!("expected sequence"),
        }
    }

    #[test]
    fn rejects_empty_chord() {
        let err = Config::from_toml_str(
            r#"
            [layer.base]
            k = []
            "#,
        )
        .unwrap_err();
        assert!(err.to_string().contains("empty"));
    }

    #[test]
    fn rejects_missing_hold_layer() {
        let err = Config::from_toml_str(
            r#"
            [layer.base]
            f = { tap = "f", hold = "missing" }
            "#,
        )
        .unwrap_err();
        assert!(err.to_string().contains("missing"));
    }
}

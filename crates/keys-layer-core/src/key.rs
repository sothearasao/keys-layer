//! Canonical key names used in config and across the engine.

use std::fmt;

/// A keyboard key identifier (config-facing name).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct KeyName(String);

impl KeyName {
    pub fn new(name: impl Into<String>) -> Self {
        let raw = name.into();
        Self(normalize(&raw))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Ctrl / Alt / Shift / Meta (either side).
    pub fn is_modifier(&self) -> bool {
        matches!(
            self.0.as_str(),
            "left_control"
                | "right_control"
                | "left_alt"
                | "right_alt"
                | "left_shift"
                | "right_shift"
                | "left_meta"
                | "right_meta"
        )
    }
}

impl fmt::Display for KeyName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for KeyName {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for KeyName {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

fn normalize(name: &str) -> String {
    let lower = name.trim().to_ascii_lowercase().replace('-', "_");
    match lower.as_str() {
        "backspace" | "bksp" => "delete".to_string(),
        "return" => "enter".to_string(),
        "esc" => "escape".to_string(),
        "ctrl" => "left_control".to_string(),
        "lctrl" | "left_ctrl" => "left_control".to_string(),
        "rctrl" | "right_ctrl" => "right_control".to_string(),
        "alt" | "option" | "lalt" | "left_alt" | "left_option" => "left_alt".to_string(),
        "ralt" | "right_alt" | "right_option" | "opt" => "right_alt".to_string(),
        "cmd" | "command" | "meta" | "win" | "lcmd" | "left_cmd" | "left_command"
        | "left_meta" => "left_meta".to_string(),
        "rcmd" | "right_cmd" | "right_command" | "right_meta" => "right_meta".to_string(),
        "shift" | "lshift" | "left_shift" => "left_shift".to_string(),
        "rshift" | "right_shift" => "right_shift".to_string(),
        "fwd_delete" | "forward_del" | "del" => "forward_delete".to_string(),
        "caps" | "capslock" | "cap_lock" | "caps_lock" => "caps_lock".to_string(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_aliases() {
        assert_eq!(KeyName::new("Backspace").as_str(), "delete");
        assert_eq!(KeyName::new("ESC").as_str(), "escape");
        assert_eq!(KeyName::new("caps").as_str(), "caps_lock");
        assert_eq!(KeyName::new("CapsLock").as_str(), "caps_lock");
        assert_eq!(KeyName::new("command").as_str(), "left_meta");
        assert!(KeyName::new("left_meta").is_modifier());
        assert!(!KeyName::new("f").is_modifier());
    }
}

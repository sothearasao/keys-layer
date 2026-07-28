//! Tap / hold-to-layer state machine.

use std::collections::HashSet;

use crate::config::{Config, KeyBinding, NativeMode, OutputKeys};
use crate::key::KeyName;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputEvent {
    KeyDown(KeyName),
    KeyUp(KeyName),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutputEvent {
    KeyDown(KeyName),
    KeyUp(KeyName),
    /// OS key-repeat while a remapped key is held (e.g. hold J → repeating Delete).
    KeyRepeat(KeyName),
}

#[derive(Debug)]
struct PendingHold {
    physical: KeyName,
    tap: Option<KeyName>,
    layer: String,
    deadline_ms: u64,
}

#[derive(Debug)]
struct ActiveHold {
    physical: KeyName,
    layer: String,
}

/// Keyboard layer engine.
///
/// Call [`Engine::handle`] for each physical key event and [`Engine::tick`]
/// periodically (or whenever you have a new timestamp) so hold delays can fire.
pub struct Engine {
    config: Config,
    /// Active momentary layers (stack). Top is the current lookup layer.
    layer_stack: Vec<String>,
    pending: Option<PendingHold>,
    active_holds: Vec<ActiveHold>,
    /// Physical keys currently down (for debugging / future use).
    physical_down: HashSet<KeyName>,
    /// Hold keys already resolved as tap while still physically held (ignore their up).
    resolved_while_held: HashSet<KeyName>,
    /// Sequence remaps already fired on KeyDown (ignore their KeyUp).
    sequence_done: HashSet<KeyName>,
    /// Output keys we have pressed and not yet released (for remap tracking).
    output_down: HashSet<KeyName>,
    /// Maps physical key → outputs currently held for that physical key.
    physical_to_output: std::collections::HashMap<KeyName, OutputKeys>,
    /// Modifiers KeyUp'd while a hold-layer is active so remaps are not
    /// chorded with Cmd/Shift/etc. Restored when the layer is left (if still held).
    suspended_modifiers: HashSet<KeyName>,
}

impl Engine {
    pub fn new(config: Config) -> Self {
        let base = config.settings.base_layer.clone();
        Self {
            config,
            layer_stack: vec![base],
            pending: None,
            active_holds: Vec::new(),
            physical_down: HashSet::new(),
            resolved_while_held: HashSet::new(),
            sequence_done: HashSet::new(),
            output_down: HashSet::new(),
            physical_to_output: std::collections::HashMap::new(),
            suspended_modifiers: HashSet::new(),
        }
    }

    /// Replace config and reset layer state.
    ///
    /// Returns KeyUp events for any outputs that were still held so the OS
    /// does not keep keys stuck across the reload.
    pub fn reload(&mut self, config: Config) -> Vec<OutputEvent> {
        let mut out = Vec::new();
        for key in self.output_down.drain() {
            out.push(OutputEvent::KeyUp(key));
        }
        let base = config.settings.base_layer.clone();
        self.config = config;
        self.layer_stack = vec![base];
        self.pending = None;
        self.active_holds.clear();
        self.physical_down.clear();
        self.resolved_while_held.clear();
        self.sequence_done.clear();
        self.physical_to_output.clear();
        self.suspended_modifiers.clear();
        out
    }

    pub fn current_layer(&self) -> &str {
        self.layer_stack
            .last()
            .map(String::as_str)
            .unwrap_or("base")
    }

    /// Whether the backend should swallow this physical key and route it through the engine.
    pub fn should_intercept(&self, key: &KeyName) -> bool {
        // Modifiers must always go through the engine. If Cmd/Shift/etc. were
        // passthrough on KeyDown and only intercepted after a hold starts, the
        // KeyUp path desyncs and the modifier sticks (Cmd+F with F as hold-key).
        if key.is_modifier() {
            return true;
        }
        // While a hold is undecided *or* a momentary layer is active, intercept
        // *all* keys. Mixing passthrough + engine for the same physical key
        // (down via passthrough, up while pending) swallows the KeyUp and the
        // OS keeps the key stuck — later presses look like "dropped" keys.
        if self.pending.is_some() || !self.active_holds.is_empty() {
            return true;
        }
        if self.resolved_while_held.contains(key) || self.sequence_done.contains(key) {
            return true;
        }
        // If we already mapped this physical key down, we must see the up.
        if self.physical_to_output.contains_key(key) {
            return true;
        }
        if self.config.is_native_disabled(key) {
            return true;
        }
        self.config
            .binding(self.current_layer(), key)
            .is_some()
    }

    fn modifier_physically_held(&self) -> bool {
        self.physical_down.iter().any(|k| k.is_modifier())
    }

    /// Temporarily release held modifiers so layer remaps are not Cmd/Shift-chorded.
    fn suspend_modifiers(&mut self) -> Vec<OutputEvent> {
        let mods: Vec<KeyName> = self
            .physical_down
            .iter()
            .filter(|k| k.is_modifier())
            .cloned()
            .collect();
        let mut out = Vec::new();
        for m in mods {
            if self.output_down.remove(&m) {
                out.push(OutputEvent::KeyUp(m.clone()));
                self.suspended_modifiers.insert(m);
            }
        }
        out
    }

    /// Re-press modifiers that are still physically held after leaving hold layers.
    fn restore_modifiers(&mut self) -> Vec<OutputEvent> {
        let mut out = Vec::new();
        let suspended: Vec<KeyName> = self.suspended_modifiers.drain().collect();
        for m in suspended {
            if !self.physical_down.contains(&m) {
                // Physical key already released while suspended; drop tracking.
                self.physical_to_output.remove(&m);
                continue;
            }
            if self.output_down.insert(m.clone()) {
                out.push(OutputEvent::KeyDown(m.clone()));
            }
            self.physical_to_output
                .entry(m.clone())
                .or_insert_with(|| OutputKeys::Single(m));
        }
        out
    }

    /// `native = "disable"` for this physical key (any layer).
    pub fn is_native_disabled(&self, key: &KeyName) -> bool {
        self.config.is_native_disabled(key)
    }

    /// Advance timers. `now_ms` is a monotonic millisecond clock.
    pub fn tick(&mut self, now_ms: u64) -> Vec<OutputEvent> {
        let Some(pending) = &self.pending else {
            return Vec::new();
        };
        if now_ms < pending.deadline_ms {
            return Vec::new();
        }

        let pending = self.pending.take().expect("pending checked");
        let mut out = Vec::new();
        // First hold-layer: drop physical modifiers so e.g. Cmd+F hold → layer
        // then J is Delete, not Cmd+Delete / stuck typing F.
        if self.active_holds.is_empty() {
            out.extend(self.suspend_modifiers());
        }
        self.layer_stack.push(pending.layer.clone());
        self.active_holds.push(ActiveHold {
            physical: pending.physical,
            layer: pending.layer,
        });
        out
    }

    pub fn handle(&mut self, event: InputEvent, now_ms: u64) -> Vec<OutputEvent> {
        // Always tick first so a hold can activate before handling a same-tick key.
        let mut out = self.tick(now_ms);

        match event {
            InputEvent::KeyDown(key) => {
                out.extend(self.on_key_down(key, now_ms));
            }
            InputEvent::KeyUp(key) => {
                out.extend(self.on_key_up(key, now_ms));
            }
        }
        out
    }

    fn on_key_down(&mut self, key: KeyName, now_ms: u64) -> Vec<OutputEvent> {
        if self.physical_down.contains(&key) {
            // OS key-repeat while held: re-fire remapped output.
            if let Some(output) = self.physical_to_output.get(&key) {
                return match output {
                    OutputKeys::Single(k) => vec![OutputEvent::KeyRepeat(k.clone())],
                    OutputKeys::Chord(keys) => keys
                        .last()
                        .map(|k| vec![OutputEvent::KeyRepeat(k.clone())])
                        .unwrap_or_default(),
                    OutputKeys::Sequence(_) => Vec::new(),
                };
            }
            return Vec::new();
        }
        self.physical_down.insert(key.clone());

        // If this key is the physical key of a pending hold, ignore (shouldn't happen).
        if self
            .pending
            .as_ref()
            .is_some_and(|p| p.physical == key)
        {
            return Vec::new();
        }

        // Another key while hold is undecided → treat hold key as a tap first.
        // This keeps fast rolls like "fe" in order instead of "ef".
        let mut out = Vec::new();
        if self
            .pending
            .as_ref()
            .is_some_and(|p| p.physical != key)
        {
            out.extend(self.flush_pending_as_tap());
        }

        let layer = self.current_layer().to_string();
        let binding = self.config.binding(&layer, &key).cloned();

        match binding {
            Some(KeyBinding::Hold {
                tap,
                hold,
                hold_ms,
                native,
            }) => {
                let mut ms = self.config.resolve_hold_ms(&layer, hold_ms);
                // native = "enable" (default) + no tap → fire the physical key on
                // quick release. native = "disable" → silence unless tap is set.
                let tap = match (tap, native) {
                    (Some(t), _) => Some(t),
                    (None, NativeMode::Enable) => Some(key.clone()),
                    (None, NativeMode::Disable) => None,
                };
                // Prefer tap for Cmd+F / Ctrl+C (slightly longer deadline), but still
                // allow hold-to-layer if the key is held long enough.
                if self.modifier_physically_held() {
                    ms = ms.saturating_mul(2).max(ms.saturating_add(120));
                }
                self.pending = Some(PendingHold {
                    physical: key,
                    tap,
                    layer: hold,
                    deadline_ms: now_ms.saturating_add(ms),
                });
            }
            Some(KeyBinding::Remap(target)) => {
                out.extend(self.press_output(key, target));
            }
            None => {
                out.extend(self.press_output(key.clone(), OutputKeys::Single(key)));
            }
        }
        out
    }

    fn on_key_up(&mut self, key: KeyName, now_ms: u64) -> Vec<OutputEvent> {
        self.physical_down.remove(&key);

        if self.resolved_while_held.remove(&key) {
            // Already emitted as tap when a later key arrived; swallow the up.
            return Vec::new();
        }
        if self.sequence_done.remove(&key) {
            return Vec::new();
        }

        // Modifier was already KeyUp'd when the hold-layer activated.
        if self.suspended_modifiers.remove(&key) {
            self.physical_to_output.remove(&key);
            return Vec::new();
        }

        // Cancel / resolve pending hold on this physical key.
        if let Some(pending) = &self.pending {
            if pending.physical == key {
                let pending = self.pending.take().expect("pending");
                // Released before deadline → tap (if configured).
                if now_ms < pending.deadline_ms {
                    return tap_events(pending.tap);
                }
                // Edge case: released exactly when deadline passed but tick hasn't run.
                // Treat as hold activate then immediately leave.
                let mut out = Vec::new();
                if self.active_holds.is_empty() {
                    out.extend(self.suspend_modifiers());
                }
                self.layer_stack.push(pending.layer.clone());
                self.active_holds.push(ActiveHold {
                    physical: pending.physical.clone(),
                    layer: pending.layer,
                });
                out.extend(self.leave_hold_for(&key));
                return out;
            }
        }

        // Leaving an active hold layer.
        if self.active_holds.iter().any(|h| h.physical == key) {
            return self.leave_hold_for(&key);
        }

        // Normal release of a remapped / engine-passthrough key.
        if self.physical_to_output.contains_key(&key) {
            return self.release_output(&key);
        }

        // Never saw KeyDown through the engine (key was already held when we
        // started intercepting, e.g. rolled into a pending hold). Still emit
        // KeyUp so the OS does not keep the key stuck.
        vec![OutputEvent::KeyUp(key)]
    }

    /// Resolve undecided hold as a tap (key still physically held).
    fn flush_pending_as_tap(&mut self) -> Vec<OutputEvent> {
        let Some(pending) = self.pending.take() else {
            return Vec::new();
        };
        self.resolved_while_held.insert(pending.physical);
        tap_events(pending.tap)
    }

    fn leave_hold_for(&mut self, physical: &KeyName) -> Vec<OutputEvent> {
        let base = self.config.settings.base_layer.clone();
        let layers_to_pop: Vec<String> = self
            .active_holds
            .iter()
            .filter(|h| &h.physical == physical)
            .map(|h| h.layer.clone())
            .collect();

        self.active_holds.retain(|h| &h.physical != physical);

        for layer in layers_to_pop {
            if let Some(pos) = self
                .layer_stack
                .iter()
                .rposition(|l| l == &layer && l.as_str() != base)
            {
                self.layer_stack.remove(pos);
            }
        }

        if self.layer_stack.is_empty() {
            self.layer_stack.push(base);
        }

        if self.active_holds.is_empty() {
            return self.restore_modifiers();
        }
        Vec::new()
    }

    fn press_output(&mut self, physical: KeyName, output: OutputKeys) -> Vec<OutputEvent> {
        match &output {
            OutputKeys::Sequence(keys) => {
                self.sequence_done.insert(physical);
                let mut out = Vec::new();
                for k in keys {
                    out.push(OutputEvent::KeyDown(k.clone()));
                    out.push(OutputEvent::KeyUp(k.clone()));
                }
                out
            }
            OutputKeys::Single(k) => {
                self.physical_to_output
                    .insert(physical, output.clone());
                if self.output_down.insert(k.clone()) {
                    vec![OutputEvent::KeyDown(k.clone())]
                } else {
                    Vec::new()
                }
            }
            OutputKeys::Chord(keys) => {
                self.physical_to_output
                    .insert(physical, output.clone());
                let mut out = Vec::new();
                for k in keys {
                    if self.output_down.insert(k.clone()) {
                        out.push(OutputEvent::KeyDown(k.clone()));
                    }
                }
                out
            }
        }
    }

    fn release_output(&mut self, physical: &KeyName) -> Vec<OutputEvent> {
        let Some(output) = self.physical_to_output.remove(physical) else {
            return Vec::new();
        };
        match output {
            OutputKeys::Single(k) => {
                if self.output_down.remove(&k) {
                    vec![OutputEvent::KeyUp(k)]
                } else {
                    Vec::new()
                }
            }
            OutputKeys::Chord(keys) => {
                let mut out = Vec::new();
                for k in keys.into_iter().rev() {
                    if self.output_down.remove(&k) {
                        out.push(OutputEvent::KeyUp(k));
                    }
                }
                out
            }
            OutputKeys::Sequence(_) => Vec::new(),
        }
    }
}

fn tap_events(tap: Option<KeyName>) -> Vec<OutputEvent> {
    match tap {
        Some(key) => vec![OutputEvent::KeyDown(key.clone()), OutputEvent::KeyUp(key)],
        None => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    fn demo_engine() -> Engine {
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
        Engine::new(cfg)
    }

    fn chord_engine() -> Engine {
        let cfg = Config::from_toml_str(
            r#"
            [settings]
            hold_ms = 200

            [layer.base]
            f = { tap = "f", hold = "mod_f" }

            [layer.mod_f]
            j = "delete"
            k = ["left_alt", "delete"]
            m = { sequence = ["a", "b"] }
            "#,
        )
        .unwrap();
        Engine::new(cfg)
    }

    #[test]
    fn tap_f_emits_f() {
        let mut eng = demo_engine();
        let out = eng.handle(InputEvent::KeyDown(KeyName::new("f")), 0);
        assert!(out.is_empty());
        let out = eng.handle(InputEvent::KeyUp(KeyName::new("f")), 100);
        assert_eq!(
            out,
            vec![
                OutputEvent::KeyDown(KeyName::new("f")),
                OutputEvent::KeyUp(KeyName::new("f")),
            ]
        );
        assert_eq!(eng.current_layer(), "base");
    }

    #[test]
    fn hold_f_then_j_becomes_delete() {
        let mut eng = demo_engine();

        assert!(eng
            .handle(InputEvent::KeyDown(KeyName::new("f")), 0)
            .is_empty());

        // Cross hold threshold with no other keys.
        assert!(eng.tick(200).is_empty());
        assert_eq!(eng.current_layer(), "mod_f");

        let out = eng.handle(InputEvent::KeyDown(KeyName::new("j")), 250);
        assert_eq!(out, vec![OutputEvent::KeyDown(KeyName::new("delete"))]);
        let out = eng.handle(InputEvent::KeyUp(KeyName::new("j")), 260);
        assert_eq!(out, vec![OutputEvent::KeyUp(KeyName::new("delete"))]);

        let out = eng.handle(InputEvent::KeyUp(KeyName::new("f")), 300);
        assert!(out.is_empty());
        assert_eq!(eng.current_layer(), "base");

        let out = eng.handle(InputEvent::KeyDown(KeyName::new("j")), 350);
        assert_eq!(out, vec![OutputEvent::KeyDown(KeyName::new("j"))]);
    }

    #[test]
    fn hold_activates_via_handle_tick() {
        let mut eng = demo_engine();
        eng.handle(InputEvent::KeyDown(KeyName::new("f")), 0);
        let out = eng.handle(InputEvent::KeyDown(KeyName::new("j")), 200);
        assert_eq!(eng.current_layer(), "mod_f");
        assert_eq!(out, vec![OutputEvent::KeyDown(KeyName::new("delete"))]);
    }

    #[test]
    fn fast_fe_keeps_order() {
        let mut eng = demo_engine();

        // Roll F then E before hold timeout — must type "fe", not "ef".
        assert!(eng
            .handle(InputEvent::KeyDown(KeyName::new("f")), 0)
            .is_empty());

        let out = eng.handle(InputEvent::KeyDown(KeyName::new("e")), 30);
        assert_eq!(
            out,
            vec![
                OutputEvent::KeyDown(KeyName::new("f")),
                OutputEvent::KeyUp(KeyName::new("f")),
                OutputEvent::KeyDown(KeyName::new("e")),
            ]
        );
        assert_eq!(eng.current_layer(), "base");

        let out = eng.handle(InputEvent::KeyUp(KeyName::new("e")), 40);
        assert_eq!(out, vec![OutputEvent::KeyUp(KeyName::new("e"))]);

        // Physical F still down; release must not emit another f.
        let out = eng.handle(InputEvent::KeyUp(KeyName::new("f")), 50);
        assert!(out.is_empty());
    }

    #[test]
    fn other_key_before_hold_timeout_is_not_layer() {
        let mut eng = demo_engine();
        eng.handle(InputEvent::KeyDown(KeyName::new("f")), 0);
        // J before 200ms means "I'm typing", not layer — flush F as tap.
        let out = eng.handle(InputEvent::KeyDown(KeyName::new("j")), 50);
        assert_eq!(
            out,
            vec![
                OutputEvent::KeyDown(KeyName::new("f")),
                OutputEvent::KeyUp(KeyName::new("f")),
                OutputEvent::KeyDown(KeyName::new("j")),
            ]
        );
        assert_eq!(eng.current_layer(), "base");
    }

    #[test]
    fn native_disable_hold_only_emits_nothing_on_tap() {
        let cfg = Config::from_toml_str(
            r#"
            [layer.base]
            caps = { hold = "mod_caps", native = "disable" }

            [layer.mod_caps]
            h = "left"
            "#,
        )
        .unwrap();
        let mut eng = Engine::new(cfg);

        eng.handle(InputEvent::KeyDown(KeyName::new("caps")), 0);
        // Quick release: no tap key configured → silence (native caps never fires).
        let out = eng.handle(InputEvent::KeyUp(KeyName::new("caps")), 50);
        assert!(out.is_empty());

        eng.handle(InputEvent::KeyDown(KeyName::new("caps")), 100);
        assert!(eng.tick(300).is_empty());
        assert_eq!(eng.current_layer(), "mod_caps");
        let out = eng.handle(InputEvent::KeyDown(KeyName::new("h")), 350);
        assert_eq!(out, vec![OutputEvent::KeyDown(KeyName::new("left"))]);
    }

    #[test]
    fn native_enable_hold_only_taps_physical_key() {
        let cfg = Config::from_toml_str(
            r#"
            [layer.base]
            caps = { hold = "mod_caps", hold_ms = 100 }

            [layer.mod_caps]
            h = "left"
            "#,
        )
        .unwrap();
        let mut eng = Engine::new(cfg);

        eng.handle(InputEvent::KeyDown(KeyName::new("caps")), 0);
        // native defaults to enable → quick release fires caps_lock.
        let out = eng.handle(InputEvent::KeyUp(KeyName::new("caps")), 40);
        assert_eq!(
            out,
            vec![
                OutputEvent::KeyDown(KeyName::new("caps_lock")),
                OutputEvent::KeyUp(KeyName::new("caps_lock")),
            ]
        );
        assert_eq!(eng.current_layer(), "base");
    }

    #[test]
    fn layer_remap_hold_repeats_output() {
        let mut eng = demo_engine();
        eng.handle(InputEvent::KeyDown(KeyName::new("f")), 0);
        assert!(eng.tick(200).is_empty());

        let out = eng.handle(InputEvent::KeyDown(KeyName::new("j")), 250);
        assert_eq!(out, vec![OutputEvent::KeyDown(KeyName::new("delete"))]);

        // OS key-repeat while J still held → repeating Delete.
        let out = eng.handle(InputEvent::KeyDown(KeyName::new("j")), 300);
        assert_eq!(out, vec![OutputEvent::KeyRepeat(KeyName::new("delete"))]);
        let out = eng.handle(InputEvent::KeyDown(KeyName::new("j")), 350);
        assert_eq!(out, vec![OutputEvent::KeyRepeat(KeyName::new("delete"))]);

        let out = eng.handle(InputEvent::KeyUp(KeyName::new("j")), 400);
        assert_eq!(out, vec![OutputEvent::KeyUp(KeyName::new("delete"))]);
    }

    #[test]
    fn key_up_without_engine_down_still_emits_up() {
        // Rolled into a pending hold: key was already down via passthrough,
        // then KeyUp arrives while we intercept — must not swallow it.
        let mut eng = demo_engine();
        eng.handle(InputEvent::KeyDown(KeyName::new("f")), 0);
        let out = eng.handle(InputEvent::KeyUp(KeyName::new("e")), 10);
        assert_eq!(out, vec![OutputEvent::KeyUp(KeyName::new("e"))]);
    }

    #[test]
    fn active_hold_passes_unbound_keys_through_engine() {
        let mut eng = demo_engine();
        eng.handle(InputEvent::KeyDown(KeyName::new("f")), 0);
        assert!(eng.tick(200).is_empty());
        assert!(eng.should_intercept(&KeyName::new("a")));

        let out = eng.handle(InputEvent::KeyDown(KeyName::new("a")), 250);
        assert_eq!(out, vec![OutputEvent::KeyDown(KeyName::new("a"))]);
        let out = eng.handle(InputEvent::KeyUp(KeyName::new("a")), 260);
        assert_eq!(out, vec![OutputEvent::KeyUp(KeyName::new("a"))]);
    }

    #[test]
    fn reload_releases_held_outputs() {
        let mut eng = demo_engine();
        eng.handle(InputEvent::KeyDown(KeyName::new("f")), 0);
        assert!(eng.tick(200).is_empty());
        eng.handle(InputEvent::KeyDown(KeyName::new("j")), 250);

        let cfg = Config::from_toml_str(
            r#"
            [layer.base]
            f = { tap = "f", hold = "mod_f" }

            [layer.mod_f]
            j = "delete"
            "#,
        )
        .unwrap();
        let out = eng.reload(cfg);
        assert_eq!(out, vec![OutputEvent::KeyUp(KeyName::new("delete"))]);
        assert_eq!(eng.current_layer(), "base");
    }

    #[test]
    fn chord_option_delete_press_repeat_release() {
        let mut eng = chord_engine();
        eng.handle(InputEvent::KeyDown(KeyName::new("f")), 0);
        assert!(eng.tick(200).is_empty());

        let out = eng.handle(InputEvent::KeyDown(KeyName::new("k")), 250);
        assert_eq!(
            out,
            vec![
                OutputEvent::KeyDown(KeyName::new("left_alt")),
                OutputEvent::KeyDown(KeyName::new("delete")),
            ]
        );

        let out = eng.handle(InputEvent::KeyDown(KeyName::new("k")), 300);
        assert_eq!(out, vec![OutputEvent::KeyRepeat(KeyName::new("delete"))]);

        let out = eng.handle(InputEvent::KeyUp(KeyName::new("k")), 350);
        assert_eq!(
            out,
            vec![
                OutputEvent::KeyUp(KeyName::new("delete")),
                OutputEvent::KeyUp(KeyName::new("left_alt")),
            ]
        );
    }

    #[test]
    fn sequence_fires_on_press_swallows_up() {
        let mut eng = chord_engine();
        eng.handle(InputEvent::KeyDown(KeyName::new("f")), 0);
        assert!(eng.tick(200).is_empty());

        let out = eng.handle(InputEvent::KeyDown(KeyName::new("m")), 250);
        assert_eq!(
            out,
            vec![
                OutputEvent::KeyDown(KeyName::new("a")),
                OutputEvent::KeyUp(KeyName::new("a")),
                OutputEvent::KeyDown(KeyName::new("b")),
                OutputEvent::KeyUp(KeyName::new("b")),
            ]
        );

        let out = eng.handle(InputEvent::KeyUp(KeyName::new("m")), 260);
        assert!(out.is_empty());
    }

    #[test]
    fn cmd_plus_hold_key_quick_tap_is_find() {
        let mut eng = demo_engine();
        eng.handle(InputEvent::KeyDown(KeyName::new("left_meta")), 0);
        eng.handle(InputEvent::KeyDown(KeyName::new("f")), 10);
        // Quick release before (extended) hold deadline → tap f while Cmd held.
        let out = eng.handle(InputEvent::KeyUp(KeyName::new("f")), 50);
        assert_eq!(
            out,
            vec![
                OutputEvent::KeyDown(KeyName::new("f")),
                OutputEvent::KeyUp(KeyName::new("f")),
            ]
        );
        assert_eq!(eng.current_layer(), "base");
        let out = eng.handle(InputEvent::KeyUp(KeyName::new("left_meta")), 60);
        assert_eq!(out, vec![OutputEvent::KeyUp(KeyName::new("left_meta"))]);
    }

    #[test]
    fn cmd_then_hold_f_then_j_is_delete_not_f() {
        let mut eng = demo_engine();
        eng.handle(InputEvent::KeyDown(KeyName::new("left_meta")), 0);
        eng.handle(InputEvent::KeyDown(KeyName::new("f")), 10);

        // Hold past extended deadline (mods double hold_ms: 200 → 400).
        let out = eng.tick(500);
        assert_eq!(out, vec![OutputEvent::KeyUp(KeyName::new("left_meta"))]);
        assert_eq!(eng.current_layer(), "mod_f");

        let out = eng.handle(InputEvent::KeyDown(KeyName::new("j")), 510);
        assert_eq!(out, vec![OutputEvent::KeyDown(KeyName::new("delete"))]);

        let out = eng.handle(InputEvent::KeyUp(KeyName::new("j")), 520);
        assert_eq!(out, vec![OutputEvent::KeyUp(KeyName::new("delete"))]);

        // Release F → leave layer and restore Cmd if still held.
        let out = eng.handle(InputEvent::KeyUp(KeyName::new("f")), 530);
        assert_eq!(out, vec![OutputEvent::KeyDown(KeyName::new("left_meta"))]);
        assert_eq!(eng.current_layer(), "base");
    }

    #[test]
    fn ctrl_then_hold_f_then_j_suspends_ctrl_too() {
        let mut eng = demo_engine();
        eng.handle(InputEvent::KeyDown(KeyName::new("left_control")), 0);
        eng.handle(InputEvent::KeyDown(KeyName::new("f")), 10);
        let out = eng.tick(500);
        assert_eq!(out, vec![OutputEvent::KeyUp(KeyName::new("left_control"))]);
        let out = eng.handle(InputEvent::KeyDown(KeyName::new("j")), 510);
        assert_eq!(out, vec![OutputEvent::KeyDown(KeyName::new("delete"))]);
    }

    #[test]
    fn modifiers_always_intercepted() {
        let eng = demo_engine();
        assert!(eng.should_intercept(&KeyName::new("left_meta")));
        assert!(eng.should_intercept(&KeyName::new("left_shift")));
        assert!(!eng.should_intercept(&KeyName::new("a")));
    }
}

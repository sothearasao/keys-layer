//! Tap / hold-to-layer state machine.

use std::collections::HashSet;

use crate::config::{Config, KeyBinding};
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
    /// Output keys we have pressed and not yet released (for remap tracking).
    output_down: HashSet<KeyName>,
    /// Maps physical key → the output key currently held for that physical key.
    physical_to_output: std::collections::HashMap<KeyName, KeyName>,
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
            output_down: HashSet::new(),
            physical_to_output: std::collections::HashMap::new(),
        }
    }

    pub fn reload(&mut self, config: Config) {
        let base = config.settings.base_layer.clone();
        self.config = config;
        self.layer_stack = vec![base];
        self.pending = None;
        self.active_holds.clear();
        self.physical_down.clear();
        self.resolved_while_held.clear();
        self.output_down.clear();
        self.physical_to_output.clear();
    }

    pub fn current_layer(&self) -> &str {
        self.layer_stack
            .last()
            .map(String::as_str)
            .unwrap_or("base")
    }

    /// Whether the backend should swallow this physical key and route it through the engine.
    pub fn should_intercept(&self, key: &KeyName) -> bool {
        // While a hold is undecided *or* a momentary layer is active, intercept
        // *all* keys. Mixing passthrough + engine for the same physical key
        // (down via passthrough, up while pending) swallows the KeyUp and the
        // OS keeps the key stuck — later presses look like "dropped" keys.
        if self.pending.is_some() || !self.active_holds.is_empty() {
            return true;
        }
        if self.resolved_while_held.contains(key) {
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
        self.layer_stack.push(pending.layer.clone());
        self.active_holds.push(ActiveHold {
            physical: pending.physical,
            layer: pending.layer,
        });
        // Hold activated: no tap output.
        Vec::new()
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
            // OS key-repeat while held: re-fire the remapped output so hold
            // works (e.g. hold J on mod_f → repeating Delete).
            if let Some(output) = self.physical_to_output.get(&key) {
                return vec![OutputEvent::KeyRepeat(output.clone())];
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
                native: _,
            }) => {
                let ms = self.config.resolve_hold_ms(&layer, hold_ms);
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
                out.extend(self.press_output(key.clone(), key));
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
                self.layer_stack.push(pending.layer.clone());
                self.active_holds.push(ActiveHold {
                    physical: pending.physical.clone(),
                    layer: pending.layer,
                });
                return self.leave_hold_for(&key);
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

        Vec::new()
    }

    fn press_output(&mut self, physical: KeyName, output: KeyName) -> Vec<OutputEvent> {
        self.physical_to_output
            .insert(physical, output.clone());
        if self.output_down.insert(output.clone()) {
            vec![OutputEvent::KeyDown(output)]
        } else {
            // Already down (unlikely); ignore.
            Vec::new()
        }
    }

    fn release_output(&mut self, physical: &KeyName) -> Vec<OutputEvent> {
        if let Some(output) = self.physical_to_output.remove(physical) {
            if self.output_down.remove(&output) {
                return vec![OutputEvent::KeyUp(output)];
            }
        }
        Vec::new()
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
}

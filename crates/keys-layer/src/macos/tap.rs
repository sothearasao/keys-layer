//! CGEvent tap: intercept keys, run engine, emit remapped events.
//!
//! Caps Lock press/release is driven by [`super::hid_caps`] (IOHID). This tap
//! only swallows Caps CGEvents and forces native Caps Lock off.

use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use core_foundation::base::CFRelease;
use core_foundation::runloop::{
    kCFRunLoopCommonModes, CFRunLoopAddSource, CFRunLoopGetCurrent, CFRunLoopRun,
    CFRunLoopSourceRef,
};
use keys_layer_core::{Engine, InputEvent, KeyName, OutputEvent};

use super::caps_lock;
use super::keycode::{code_to_key_name, key_name_to_code};

// --- Quartz / CoreGraphics FFI ---

type CGEventRef = *mut c_void;
type CGEventTapProxy = *mut c_void;
type CFMachPortRef = *mut c_void;

type CGEventTapCallBack = Option<
    unsafe extern "C" fn(
        proxy: CGEventTapProxy,
        event_type: u32,
        event: CGEventRef,
        user_info: *mut c_void,
    ) -> CGEventRef,
>;

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGEventTapCreate(
        tap: u32,
        place: u32,
        options: u32,
        events_of_interest: u64,
        callback: CGEventTapCallBack,
        user_info: *mut c_void,
    ) -> CFMachPortRef;

    fn CGEventTapEnable(tap: CFMachPortRef, enable: bool);
    fn CGEventGetIntegerValueField(event: CGEventRef, field: u32) -> i64;
    fn CGEventSetIntegerValueField(event: CGEventRef, field: u32, value: i64);
    fn CGEventGetFlags(event: CGEventRef) -> u64;
    fn CGEventSetFlags(event: CGEventRef, flags: u64);
    fn CGEventCreateKeyboardEvent(
        source: CGEventRef,
        virtual_key: u16,
        key_down: bool,
    ) -> CGEventRef;
    fn CGEventPost(tap: u32, event: CGEventRef);
    fn CGEventSourceCreate(state_id: i32) -> CGEventRef;

    fn CFMachPortCreateRunLoopSource(
        allocator: *const c_void,
        port: CFMachPortRef,
        order: i64,
    ) -> CFRunLoopSourceRef;
}

const KEY_DOWN: u32 = 10;
const KEY_UP: u32 = 11;
const FLAGS_CHANGED: u32 = 12;
const TAP_DISABLED_BY_TIMEOUT: u32 = 0xFFFF_FFFE;
const TAP_DISABLED_BY_USER_INPUT: u32 = 0xFFFF_FFFF;

const HID: u32 = 0;
const HEAD_INSERT: u32 = 0;
const DEFAULT_TAP: u32 = 0;
const CG_SESSION_EVENT_TAP: u32 = 1;

const KEYCODE_FIELD: u32 = 9;
const AUTOREPEAT_FIELD: u32 = 8;
const OUR_MARKER_FIELD: u32 = 42;

const OUR_MARKER: i64 = 0x4B4C_5952;
const CAPS_LOCK_KEYCODE: u16 = 0x39;
const ALPHA_SHIFT: u64 = 0x0001_0000;

static EMITTING: AtomicBool = AtomicBool::new(false);

/// Shared Caps Lock physical tracking (updated by HID monitor).
pub struct CapsPhysical {
    pub down: AtomicBool,
    pub ignore_until_ms: AtomicU64,
}

impl CapsPhysical {
    pub fn new() -> Self {
        Self {
            down: AtomicBool::new(false),
            ignore_until_ms: AtomicU64::new(0),
        }
    }
}

struct TapState {
    engine: Arc<Mutex<Engine>>,
    started: Instant,
    port: CFMachPortRef,
    caps: Arc<CapsPhysical>,
    /// When true, Caps Lock down/up comes from HID — CGEvent only swallows.
    hid_drives_caps: bool,
}

pub struct EventTap {
    _state: Box<TapState>,
}

impl EventTap {
    pub fn new(
        engine: Arc<Mutex<Engine>>,
        started: Instant,
        caps: Arc<CapsPhysical>,
        hid_drives_caps: bool,
    ) -> Result<Self, String> {
        let mut state = Box::new(TapState {
            engine,
            started,
            port: std::ptr::null_mut(),
            caps,
            hid_drives_caps,
        });

        let mask: u64 = (1 << KEY_DOWN) | (1 << KEY_UP) | (1 << FLAGS_CHANGED);

        let port = unsafe {
            CGEventTapCreate(
                HID,
                HEAD_INSERT,
                DEFAULT_TAP,
                mask,
                Some(tap_callback),
                state.as_mut() as *mut TapState as *mut c_void,
            )
        };

        if port.is_null() {
            return Err(
                "failed to create CGEvent tap — grant Accessibility permission \
                 (System Settings → Privacy & Security → Accessibility) and retry"
                    .into(),
            );
        }

        state.port = port;

        unsafe {
            let source = CFMachPortCreateRunLoopSource(std::ptr::null(), port, 0);
            if source.is_null() {
                CFRelease(port as *const _);
                return Err("failed to create run loop source".into());
            }
            CFRunLoopAddSource(CFRunLoopGetCurrent(), source, kCFRunLoopCommonModes);
            CGEventTapEnable(port, true);
            CFRelease(source as *const _);
        }

        Ok(Self { _state: state })
    }

    pub fn run(self) -> Result<(), String> {
        unsafe { CFRunLoopRun() };
        Ok(())
    }
}

unsafe extern "C" fn tap_callback(
    _proxy: CGEventTapProxy,
    event_type: u32,
    event: CGEventRef,
    user_info: *mut c_void,
) -> CGEventRef {
    let state = &*(user_info as *const TapState);

    if event_type == TAP_DISABLED_BY_TIMEOUT || event_type == TAP_DISABLED_BY_USER_INPUT {
        CGEventTapEnable(state.port, true);
        return event;
    }

    if EMITTING.load(Ordering::SeqCst) {
        return event;
    }
    let marker = CGEventGetIntegerValueField(event, OUR_MARKER_FIELD);
    if marker == OUR_MARKER {
        return event;
    }

    let keycode = CGEventGetIntegerValueField(event, KEYCODE_FIELD) as u16;
    let now_ms = state.started.elapsed().as_millis() as u64;

    // Caps Lock CGEvents: swallow + force native off. HID owns press/release.
    if keycode == CAPS_LOCK_KEYCODE
        && (event_type == FLAGS_CHANGED || event_type == KEY_DOWN || event_type == KEY_UP)
    {
        return handle_caps_cgevent(state, event, event_type, now_ms);
    }

    if event_type == FLAGS_CHANGED {
        return event;
    }

    if event_type != KEY_DOWN && event_type != KEY_UP {
        return event;
    }

    let Some(key) = code_to_key_name(keycode) else {
        return strip_alpha_shift_if_needed(state, event);
    };

    let is_repeat =
        event_type == KEY_DOWN && CGEventGetIntegerValueField(event, AUTOREPEAT_FIELD) != 0;

    let (intercept, outputs, strip_caps) = {
        let mut eng = state.engine.lock().expect("engine lock");
        let _ = eng.tick(now_ms);
        let strip_caps = eng.is_native_disabled(&KeyName::new("caps_lock"));
        if !eng.should_intercept(&key) {
            (false, Vec::new(), strip_caps)
        } else if is_repeat {
            // Forward OS repeats into the engine (holdable remaps / repeating Delete).
            (
                true,
                eng.handle(InputEvent::KeyDown(key), now_ms),
                strip_caps,
            )
        } else {
            let input = if event_type == KEY_DOWN {
                InputEvent::KeyDown(key)
            } else {
                InputEvent::KeyUp(key)
            };
            (true, eng.handle(input, now_ms), strip_caps)
        }
    };

    if !intercept {
        if strip_caps {
            clear_alpha_shift(event);
        }
        return event;
    }

    emit_outputs(&outputs);
    std::ptr::null_mut()
}

unsafe fn handle_caps_cgevent(
    state: &TapState,
    event: CGEventRef,
    event_type: u32,
    now_ms: u64,
) -> CGEventRef {
    // Swallow driver echo from IOHIDSetModifierLockState(false).
    if now_ms < state.caps.ignore_until_ms.load(Ordering::SeqCst) {
        return std::ptr::null_mut();
    }

    let native_disabled = {
        let eng = state.engine.lock().expect("engine lock");
        eng.is_native_disabled(&KeyName::new("caps_lock"))
    };

    if native_disabled {
        caps_lock::force_caps_lock_off();
        state
            .caps
            .ignore_until_ms
            .store(now_ms.saturating_add(50), Ordering::SeqCst);
    }

    // HID monitor owns engine KeyDown/KeyUp for Caps.
    if state.hid_drives_caps {
        if native_disabled
            || {
                let eng = state.engine.lock().expect("engine lock");
                eng.should_intercept(&KeyName::new("caps_lock"))
            }
        {
            return std::ptr::null_mut();
        }
        return event;
    }

    // Fallback without HID: flagsChanged / key edges (release may be sticky).
    let _ = event_type;
    let key = KeyName::new("caps_lock");
    let already_down = state.caps.down.load(Ordering::SeqCst);
    let want_down = match event_type {
        KEY_DOWN => true,
        KEY_UP => false,
        FLAGS_CHANGED => !already_down,
        _ => return event,
    };
    if want_down == already_down {
        if native_disabled {
            return std::ptr::null_mut();
        }
        return event;
    }

    let outputs = {
        let mut eng = state.engine.lock().expect("engine lock");
        if !eng.should_intercept(&key) && !native_disabled {
            return event;
        }
        state.caps.down.store(want_down, Ordering::SeqCst);
        let input = if want_down {
            InputEvent::KeyDown(key)
        } else {
            InputEvent::KeyUp(key)
        };
        eng.handle(input, now_ms)
    };
    emit_outputs(&outputs);
    std::ptr::null_mut()
}

unsafe fn strip_alpha_shift_if_needed(state: &TapState, event: CGEventRef) -> CGEventRef {
    let strip = {
        let eng = state.engine.lock().expect("engine lock");
        eng.is_native_disabled(&KeyName::new("caps_lock"))
    };
    if strip {
        clear_alpha_shift(event);
    }
    event
}

unsafe fn clear_alpha_shift(event: CGEventRef) {
    let flags = CGEventGetFlags(event);
    if flags & ALPHA_SHIFT != 0 {
        CGEventSetFlags(event, flags & !ALPHA_SHIFT);
    }
}

pub fn emit_outputs(outputs: &[OutputEvent]) {
    if outputs.is_empty() {
        return;
    }

    EMITTING.store(true, Ordering::SeqCst);
    for out in outputs {
        let (name, down, repeat) = match out {
            OutputEvent::KeyDown(k) => (k, true, false),
            OutputEvent::KeyUp(k) => (k, false, false),
            OutputEvent::KeyRepeat(k) => (k, true, true),
        };
        if let Some(code) = key_name_to_code(name) {
            unsafe {
                post_key(code, down, repeat);
            }
        } else {
            eprintln!("warning: unknown key in output: {name}");
        }
    }
    EMITTING.store(false, Ordering::SeqCst);
}

unsafe fn post_key(keycode: u16, key_down: bool, repeat: bool) {
    let source = CGEventSourceCreate(0);
    let event = CGEventCreateKeyboardEvent(source, keycode, key_down);
    if !event.is_null() {
        CGEventSetIntegerValueField(event, OUR_MARKER_FIELD, OUR_MARKER);
        if repeat {
            CGEventSetIntegerValueField(event, AUTOREPEAT_FIELD, 1);
        }
        CGEventPost(CG_SESSION_EVENT_TAP, event);
        CFRelease(event as *const _);
    }
    if !source.is_null() {
        CFRelease(source as *const _);
    }
}

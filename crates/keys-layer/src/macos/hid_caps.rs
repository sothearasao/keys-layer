//! Caps Lock press/release via IOHIDManager.
//!
//! After we force native Caps Lock off, CGEvent often never delivers a key-up,
//! and `CGEventSourceKeyState` does not report Caps as held. HID element
//! values (usage page 0x07 / usage 0x39) are the reliable press+release signal.

use std::ffi::c_void;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use core_foundation::base::{CFRelease, TCFType};
use core_foundation::dictionary::CFDictionary;
use core_foundation::number::CFNumber;
use core_foundation::runloop::{kCFRunLoopCommonModes, CFRunLoopGetCurrent};
use core_foundation::string::CFString;
use keys_layer_core::{Engine, InputEvent, KeyName};

use super::caps_lock;
use super::tap::{emit_outputs, CapsPhysical};

type IOHIDManagerRef = *mut c_void;
type IOHIDValueRef = *mut c_void;
type IOHIDElementRef = *mut c_void;
type IOReturn = i32;

const K_IO_RETURN_SUCCESS: IOReturn = 0;
const K_IOHID_OPTIONS_NONE: u32 = 0;

const USAGE_PAGE_GENERIC_DESKTOP: i32 = 0x01;
const USAGE_KEYBOARD: i32 = 0x06;
const USAGE_PAGE_KEYBOARD: u32 = 0x07;
const USAGE_CAPS_LOCK: u32 = 0x39;

#[link(name = "IOKit", kind = "framework")]
extern "C" {
    fn IOHIDManagerCreate(allocator: *const c_void, options: u32) -> IOHIDManagerRef;
    fn IOHIDManagerSetDeviceMatching(manager: IOHIDManagerRef, matching: *const c_void);
    fn IOHIDManagerSetInputValueMatching(manager: IOHIDManagerRef, matching: *const c_void);
    fn IOHIDManagerRegisterInputValueCallback(
        manager: IOHIDManagerRef,
        callback: Option<
            unsafe extern "C" fn(
                context: *mut c_void,
                result: IOReturn,
                sender: *mut c_void,
                value: IOHIDValueRef,
            ),
        >,
        context: *mut c_void,
    );
    fn IOHIDManagerScheduleWithRunLoop(
        manager: IOHIDManagerRef,
        run_loop: *mut c_void,
        run_loop_mode: *const c_void,
    );
    fn IOHIDManagerOpen(manager: IOHIDManagerRef, options: u32) -> IOReturn;
    fn IOHIDManagerClose(manager: IOHIDManagerRef, options: u32) -> IOReturn;

    fn IOHIDValueGetElement(value: IOHIDValueRef) -> IOHIDElementRef;
    fn IOHIDValueGetIntegerValue(value: IOHIDValueRef) -> i64;
    fn IOHIDElementGetUsagePage(element: IOHIDElementRef) -> u32;
    fn IOHIDElementGetUsage(element: IOHIDElementRef) -> u32;
}

pub struct CapsHidMonitor {
    manager: IOHIDManagerRef,
    /// Kept alive for the HID callback.
    _ctx: Box<CapsHidContext>,
}

struct CapsHidContext {
    engine: Arc<Mutex<Engine>>,
    started: Instant,
    caps: Arc<CapsPhysical>,
}

impl CapsHidMonitor {
    /// Open HID keyboard monitor and schedule it on the current CFRunLoop.
    pub fn start(
        engine: Arc<Mutex<Engine>>,
        started: Instant,
        caps: Arc<CapsPhysical>,
    ) -> Result<Self, String> {
        let manager = unsafe { IOHIDManagerCreate(std::ptr::null(), K_IOHID_OPTIONS_NONE) };
        if manager.is_null() {
            return Err("IOHIDManagerCreate failed".into());
        }

        let device_matching = CFDictionary::from_CFType_pairs(&[
            (
                CFString::new("DeviceUsagePage"),
                CFNumber::from(USAGE_PAGE_GENERIC_DESKTOP),
            ),
            (
                CFString::new("DeviceUsage"),
                CFNumber::from(USAGE_KEYBOARD),
            ),
        ]);
        let input_matching = CFDictionary::from_CFType_pairs(&[
            (
                CFString::new("UsagePage"),
                CFNumber::from(USAGE_PAGE_KEYBOARD as i32),
            ),
            (
                CFString::new("Usage"),
                CFNumber::from(USAGE_CAPS_LOCK as i32),
            ),
        ]);

        unsafe {
            IOHIDManagerSetDeviceMatching(manager, device_matching.as_CFTypeRef() as *const c_void);
            IOHIDManagerSetInputValueMatching(
                manager,
                input_matching.as_CFTypeRef() as *const c_void,
            );
        }

        let mut ctx = Box::new(CapsHidContext {
            engine,
            started,
            caps,
        });

        unsafe {
            IOHIDManagerRegisterInputValueCallback(
                manager,
                Some(hid_caps_callback),
                ctx.as_mut() as *mut CapsHidContext as *mut c_void,
            );
            IOHIDManagerScheduleWithRunLoop(
                manager,
                CFRunLoopGetCurrent() as *mut c_void,
                kCFRunLoopCommonModes as *const c_void,
            );
            let rc = IOHIDManagerOpen(manager, K_IOHID_OPTIONS_NONE);
            if rc != K_IO_RETURN_SUCCESS {
                IOHIDManagerClose(manager, K_IOHID_OPTIONS_NONE);
                CFRelease(manager as *const _);
                return Err(format!(
                    "IOHIDManagerOpen failed (rc={rc}). Grant Input Monitoring \
                     (System Settings → Privacy & Security → Input Monitoring) and retry."
                ));
            }
        }

        eprintln!("caps: HID press/release monitor active");
        Ok(Self {
            manager,
            _ctx: ctx,
        })
    }
}

impl Drop for CapsHidMonitor {
    fn drop(&mut self) {
        if !self.manager.is_null() {
            unsafe {
                IOHIDManagerClose(self.manager, K_IOHID_OPTIONS_NONE);
                CFRelease(self.manager as *const _);
            }
            self.manager = std::ptr::null_mut();
        }
    }
}

unsafe extern "C" fn hid_caps_callback(
    context: *mut c_void,
    _result: IOReturn,
    _sender: *mut c_void,
    value: IOHIDValueRef,
) {
    if context.is_null() || value.is_null() {
        return;
    }
    let ctx = &*(context as *const CapsHidContext);

    let element = IOHIDValueGetElement(value);
    if element.is_null() {
        return;
    }
    if IOHIDElementGetUsagePage(element) != USAGE_PAGE_KEYBOARD {
        return;
    }
    if IOHIDElementGetUsage(element) != USAGE_CAPS_LOCK {
        return;
    }

    let pressed = IOHIDValueGetIntegerValue(value) != 0;
    let already = ctx.caps.down.load(Ordering::SeqCst);
    if pressed == already {
        return;
    }

    let now_ms = ctx.started.elapsed().as_millis() as u64;
    let key = KeyName::new("caps_lock");

    let (native_disabled, outputs) = {
        let mut eng = ctx.engine.lock().expect("engine lock");
        let native_disabled = eng.is_native_disabled(&key);
        if !eng.should_intercept(&key) && !native_disabled {
            return;
        }
        ctx.caps.down.store(pressed, Ordering::SeqCst);
        let input = if pressed {
            InputEvent::KeyDown(key)
        } else {
            InputEvent::KeyUp(key)
        };
        (native_disabled, eng.handle(input, now_ms))
    };

    if native_disabled {
        caps_lock::force_caps_lock_off();
        ctx.caps
            .ignore_until_ms
            .store(now_ms.saturating_add(50), Ordering::SeqCst);
    }

    emit_outputs(&outputs);
}

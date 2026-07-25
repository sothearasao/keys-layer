//! Force Caps Lock LED / modifier state via IOKit.
//!
//! CGEvent taps cannot block Caps Lock: the driver toggles it before the tap
//! sees the event. `IOHIDSetModifierLockState` is the supported way to clear it
//! (same approach Kanata uses on macOS).

use std::ffi::c_void;
use std::sync::OnceLock;

type IoService = u32;
type IoConnect = u32;

const K_IOHID_PARAM_CONNECT_TYPE: u32 = 1;
const K_IOHID_CAPS_LOCK_STATE: i32 = 1;

#[link(name = "IOKit", kind = "framework")]
extern "C" {
    fn IOServiceMatching(name: *const libc::c_char) -> *mut c_void;
    fn IOServiceGetMatchingService(main_port: u32, matching: *mut c_void) -> IoService;
    fn IOServiceOpen(
        service: IoService,
        owning_task: u32,
        type_: u32,
        connect: *mut IoConnect,
    ) -> i32;
    fn IOObjectRelease(object: u32) -> i32;
    fn IOHIDGetModifierLockState(handle: IoConnect, selector: i32, state: *mut bool) -> i32;
    fn IOHIDSetModifierLockState(handle: IoConnect, selector: i32, state: bool) -> i32;
}

extern "C" {
    static mach_task_self_: u32;
}

fn hid_connect() -> Option<IoConnect> {
    static CONNECT: OnceLock<Option<IoConnect>> = OnceLock::new();
    *CONNECT.get_or_init(|| {
        let name = c"IOHIDSystem";
        let matching = unsafe { IOServiceMatching(name.as_ptr()) };
        if matching.is_null() {
            eprintln!("warning: IOServiceMatching(IOHIDSystem) failed");
            return None;
        }
        let service = unsafe { IOServiceGetMatchingService(0, matching) };
        if service == 0 {
            eprintln!("warning: IOHIDSystem service not found");
            return None;
        }
        let mut connect = 0u32;
        let rc = unsafe {
            IOServiceOpen(
                service,
                mach_task_self_,
                K_IOHID_PARAM_CONNECT_TYPE,
                &mut connect,
            )
        };
        unsafe {
            IOObjectRelease(service);
        }
        if rc == 0 && connect != 0 {
            Some(connect)
        } else {
            eprintln!("warning: IOServiceOpen(IOHIDSystem) failed rc={rc}");
            None
        }
    })
}

/// Read current Caps Lock lock state (LED / modifier).
pub fn get_caps_lock_state() -> Option<bool> {
    let connect = hid_connect()?;
    let mut state = false;
    let rc = unsafe { IOHIDGetModifierLockState(connect, K_IOHID_CAPS_LOCK_STATE, &mut state) };
    if rc == 0 {
        Some(state)
    } else {
        None
    }
}

/// Set Caps Lock lock state. `false` turns native Caps Lock off.
pub fn set_caps_lock_state(on: bool) -> bool {
    let Some(connect) = hid_connect() else {
        return false;
    };
    let rc = unsafe { IOHIDSetModifierLockState(connect, K_IOHID_CAPS_LOCK_STATE, on) };
    rc == 0
}

/// Ensure native Caps Lock is off (LED + shift-lock behavior).
pub fn force_caps_lock_off() {
    if get_caps_lock_state() == Some(false) {
        return;
    }
    if !set_caps_lock_state(false) {
        eprintln!("warning: failed to force Caps Lock off via IOHID");
    }
}

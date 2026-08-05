//! Windows backend — low-level keyboard hook (`WH_KEYBOARD_LL`) + `SendInput`.
//!
//! No third-party driver required. Works like Kanata's default Windows path.
//! Injected events are ignored via `LLKHF_INJECTED` to avoid feedback loops.
//!
//! Limits vs Interception/DriverKit:
//! - `settings.devices` cannot filter hardware (hook is system-wide)
//! - Some elevated / anti-cheat apps may not see remaps
//! - F-row media covers F7–F12 only (no brightness VKs)

use std::path::Path;
use std::ptr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

use keys_layer_core::{load_config, Engine, InputEvent, OutputEvent};
use windows_sys::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP,
};
use windows_sys::Win32::System::Console::{SetConsoleCtrlHandler, CTRL_C_EVENT};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, PostQuitMessage, SetWindowsHookExW,
    TranslateMessage, UnhookWindowsHookEx, KBDLLHOOKSTRUCT, MSG, WH_KEYBOARD_LL, WM_KEYDOWN,
    WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
};

use super::keymap::{
    is_function_row_vk, media_vk_for_f_row, name_to_winkey, vk_to_name, WinKey,
};
use super::reload;

const LLKHF_INJECTED: u32 = 0x10;

pub(super) struct WinState {
    engine: Mutex<Engine>,
    started: Instant,
    f_row_media: AtomicBool,
}

static STATE: OnceLock<Arc<WinState>> = OnceLock::new();
static HOOK: Mutex<Option<isize>> = Mutex::new(None);

/// Run the Windows remapper until the process exits.
pub fn run(config_path: &Path) -> Result<(), String> {
    let config = load_config(config_path).map_err(|e| e.to_string())?;
    if !config.settings.devices.is_empty() {
        eprintln!(
            "warning: settings.devices is ignored on Windows (LLHOOK is system-wide).\n\
             For per-device filtering, a future Interception backend is required."
        );
    }
    let f_row_media = !config.settings.f_row_media_devices.is_empty();
    if f_row_media {
        eprintln!(
            "F-row media enabled (F7–F12 → media/volume; F1–F6 stay F-keys).\n\
             Set f_row_media_devices = [] to disable."
        );
    }

    let state = Arc::new(WinState {
        engine: Mutex::new(Engine::new(config)),
        started: Instant::now(),
        f_row_media: AtomicBool::new(f_row_media),
    });
    STATE.set(Arc::clone(&state)).map_err(|_| {
        "windows backend already started in this process".to_string()
    })?;

    unsafe {
        SetConsoleCtrlHandler(Some(console_ctrl_handler), 1);
    }

    let hook = unsafe {
        SetWindowsHookExW(
            WH_KEYBOARD_LL,
            Some(low_level_keyboard_proc),
            ptr::null_mut(),
            0,
        )
    };
    if hook.is_null() {
        return Err(
            "SetWindowsHookExW(WH_KEYBOARD_LL) failed.\n\
             Try running from a normal user session (not Session 0)."
                .into(),
        );
    }
    *HOOK.lock().expect("hook lock") = Some(hook as isize);

    let tick_state = Arc::clone(&state);
    thread::spawn(move || loop {
        thread::sleep(Duration::from_millis(10));
        let now_ms = tick_state.started.elapsed().as_millis() as u64;
        let outputs = {
            let mut eng = tick_state.engine.lock().expect("engine lock");
            eng.tick(now_ms)
        };
        emit_outputs(&outputs);
    });

    reload::start(
        config_path.to_path_buf(),
        Arc::clone(&state),
    );

    eprintln!(
        "keys-layer (Windows/LLHOOK) running — {}\n\
         Physical keys → engine → SendInput. Config hot-reloads on save.\n\
         Ctrl-C to quit.",
        config_path.display()
    );

    // Message pump required for WH_KEYBOARD_LL.
    unsafe {
        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }

    if let Some(h) = HOOK.lock().expect("hook lock").take() {
        unsafe {
            UnhookWindowsHookEx(h as _);
        }
    }
    Ok(())
}

unsafe extern "system" fn console_ctrl_handler(ctrl: u32) -> i32 {
    if ctrl == CTRL_C_EVENT {
        PostQuitMessage(0);
        1
    } else {
        0
    }
}

pub(super) fn reload_engine(config: keys_layer_core::Config) -> Vec<OutputEvent> {
    let Some(st) = STATE.get() else {
        return Vec::new();
    };
    let f_row = !config.settings.f_row_media_devices.is_empty();
    st.f_row_media.store(f_row, Ordering::Relaxed);
    let mut eng = st.engine.lock().expect("engine lock");
    eng.reload(config)
}

unsafe extern "system" fn low_level_keyboard_proc(
    code: i32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if code < 0 {
        return CallNextHookEx(ptr::null_mut(), code, wparam, lparam);
    }

    let kb = &*(lparam as *const KBDLLHOOKSTRUCT);
    // Ignore our own SendInput (and other injected) events.
    if kb.flags & LLKHF_INJECTED != 0 {
        return CallNextHookEx(ptr::null_mut(), code, wparam, lparam);
    }

    let Some(st) = STATE.get() else {
        return CallNextHookEx(ptr::null_mut(), code, wparam, lparam);
    };

    let is_down = matches!(wparam as u32, WM_KEYDOWN | WM_SYSKEYDOWN);
    let is_up = matches!(wparam as u32, WM_KEYUP | WM_SYSKEYUP);
    if !is_down && !is_up {
        return CallNextHookEx(ptr::null_mut(), code, wparam, lparam);
    }

    let vk = kb.vkCode as u16;

    // Optional F-row media (system-wide when enabled).
    if st.f_row_media.load(Ordering::Relaxed) && is_function_row_vk(vk) {
        if let Some(media) = media_vk_for_f_row(vk) {
            send_key(media, is_down);
            return 1; // swallow physical F-key
        }
    }

    let Some(key) = vk_to_name(vk) else {
        return CallNextHookEx(ptr::null_mut(), code, wparam, lparam);
    };

    let now_ms = st.started.elapsed().as_millis() as u64;
    let input = if is_up {
        InputEvent::KeyUp(key.clone())
    } else {
        InputEvent::KeyDown(key.clone())
    };

    let outputs = {
        let mut eng = st.engine.lock().expect("engine lock");
        if !(eng.should_intercept(&key) || eng.is_native_disabled(&key)) {
            None
        } else {
            Some(eng.handle(input, now_ms))
        }
    };

    match outputs {
            None => CallNextHookEx(ptr::null_mut(), code, wparam, lparam),
        Some(outputs) => {
            emit_outputs(&outputs);
            1 // swallow
        }
    }
}

fn emit_outputs(outputs: &[OutputEvent]) {
    for out in outputs {
        let (name, down) = match out {
            OutputEvent::KeyDown(k) | OutputEvent::KeyRepeat(k) => (k, true),
            OutputEvent::KeyUp(k) => (k, false),
        };
        let Some(wk) = name_to_winkey(name) else {
            eprintln!("warning: unknown output key: {name}");
            continue;
        };
        send_key(wk, down);
    }
}

pub(super) fn emit_outputs_pub(outputs: &[OutputEvent]) {
    emit_outputs(outputs);
}

fn send_key(key: WinKey, down: bool) {
    let mut flags = 0u32;
    if !down {
        flags |= KEYEVENTF_KEYUP;
    }
    if key.extended {
        flags |= KEYEVENTF_EXTENDEDKEY;
    }

    let mut input = INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: unsafe { std::mem::zeroed() },
    };
    unsafe {
        input.Anonymous.ki = KEYBDINPUT {
            wVk: key.vk,
            wScan: 0,
            dwFlags: flags,
            time: 0,
            dwExtraInfo: 0,
        };
        SendInput(1, &input, std::mem::size_of::<INPUT>() as i32);
    }
}

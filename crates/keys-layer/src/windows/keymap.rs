//! Windows virtual-key codes ↔ [`KeyName`].

use keys_layer_core::KeyName;

// From winuser.h — keep numeric so we don't need the full windows crate at map time.
pub const VK_BACK: u16 = 0x08;
pub const VK_TAB: u16 = 0x09;
pub const VK_RETURN: u16 = 0x0D;
pub const VK_SHIFT: u16 = 0x10;
pub const VK_CONTROL: u16 = 0x11;
pub const VK_MENU: u16 = 0x12; // Alt
pub const VK_CAPITAL: u16 = 0x14;
pub const VK_ESCAPE: u16 = 0x1B;
pub const VK_SPACE: u16 = 0x20;
pub const VK_PRIOR: u16 = 0x21; // Page Up
pub const VK_NEXT: u16 = 0x22; // Page Down
pub const VK_END: u16 = 0x23;
pub const VK_HOME: u16 = 0x24;
pub const VK_LEFT: u16 = 0x25;
pub const VK_UP: u16 = 0x26;
pub const VK_RIGHT: u16 = 0x27;
pub const VK_DOWN: u16 = 0x28;
pub const VK_INSERT: u16 = 0x2D;
pub const VK_DELETE: u16 = 0x2E;
pub const VK_LWIN: u16 = 0x5B;
pub const VK_RWIN: u16 = 0x5C;
pub const VK_OEM_1: u16 = 0xBA; // ;
pub const VK_OEM_PLUS: u16 = 0xBB;
pub const VK_OEM_COMMA: u16 = 0xBC;
pub const VK_OEM_MINUS: u16 = 0xBD;
pub const VK_OEM_PERIOD: u16 = 0xBE;
pub const VK_OEM_2: u16 = 0xBF; // /
pub const VK_OEM_3: u16 = 0xC0; // `
pub const VK_OEM_4: u16 = 0xDB; // [
pub const VK_OEM_5: u16 = 0xDC; // \
pub const VK_OEM_6: u16 = 0xDD; // ]
pub const VK_OEM_7: u16 = 0xDE; // '
pub const VK_OEM_102: u16 = 0xE2; // non-US \\
pub const VK_F1: u16 = 0x70;
pub const VK_F12: u16 = 0x7B;
pub const VK_LSHIFT: u16 = 0xA0;
pub const VK_RSHIFT: u16 = 0xA1;
pub const VK_LCONTROL: u16 = 0xA2;
pub const VK_RCONTROL: u16 = 0xA3;
pub const VK_LMENU: u16 = 0xA4;
pub const VK_RMENU: u16 = 0xA5;
pub const VK_VOLUME_MUTE: u16 = 0xAD;
pub const VK_VOLUME_DOWN: u16 = 0xAE;
pub const VK_VOLUME_UP: u16 = 0xAF;
pub const VK_MEDIA_NEXT_TRACK: u16 = 0xB0;
pub const VK_MEDIA_PREV_TRACK: u16 = 0xB1;
pub const VK_MEDIA_PLAY_PAUSE: u16 = 0xB3;
/// `(vk, extended)` — extended keys need `KEYEVENTF_EXTENDEDKEY` on SendInput.
#[derive(Clone, Copy, Debug)]
pub struct WinKey {
    pub vk: u16,
    pub extended: bool,
}

pub fn vk_to_name(vk: u16) -> Option<KeyName> {
    let name = match vk {
        0x41 => "a",
        0x42 => "b",
        0x43 => "c",
        0x44 => "d",
        0x45 => "e",
        0x46 => "f",
        0x47 => "g",
        0x48 => "h",
        0x49 => "i",
        0x4A => "j",
        0x4B => "k",
        0x4C => "l",
        0x4D => "m",
        0x4E => "n",
        0x4F => "o",
        0x50 => "p",
        0x51 => "q",
        0x52 => "r",
        0x53 => "s",
        0x54 => "t",
        0x55 => "u",
        0x56 => "v",
        0x57 => "w",
        0x58 => "x",
        0x59 => "y",
        0x5A => "z",
        0x30 => "0",
        0x31 => "1",
        0x32 => "2",
        0x33 => "3",
        0x34 => "4",
        0x35 => "5",
        0x36 => "6",
        0x37 => "7",
        0x38 => "8",
        0x39 => "9",
        VK_RETURN => "enter",
        VK_ESCAPE => "escape",
        VK_BACK => "delete",
        VK_TAB => "tab",
        VK_SPACE => "space",
        VK_OEM_MINUS => "minus",
        VK_OEM_PLUS => "equal",
        VK_OEM_4 => "left_bracket",
        VK_OEM_6 => "right_bracket",
        VK_OEM_5 => "backslash",
        VK_OEM_1 => "semicolon",
        VK_OEM_7 => "quote",
        VK_OEM_3 => "grave",
        VK_OEM_COMMA => "comma",
        VK_OEM_PERIOD => "period",
        VK_OEM_2 => "slash",
        VK_CAPITAL => "caps_lock",
        0x70 => "f1",
        0x71 => "f2",
        0x72 => "f3",
        0x73 => "f4",
        0x74 => "f5",
        0x75 => "f6",
        0x76 => "f7",
        0x77 => "f8",
        0x78 => "f9",
        0x79 => "f10",
        0x7A => "f11",
        0x7B => "f12",
        VK_INSERT => "insert",
        VK_HOME => "home",
        VK_PRIOR => "page_up",
        VK_DELETE => "forward_delete",
        VK_END => "end",
        VK_NEXT => "page_down",
        VK_RIGHT => "right",
        VK_LEFT => "left",
        VK_DOWN => "down",
        VK_UP => "up",
        VK_LCONTROL => "left_control",
        VK_RCONTROL => "right_control",
        VK_LSHIFT => "left_shift",
        VK_RSHIFT => "right_shift",
        VK_LMENU => "left_alt",
        VK_RMENU => "right_alt",
        VK_LWIN => "left_meta",
        VK_RWIN => "right_meta",
        // Generic modifiers (some hooks report these instead of L/R)
        VK_CONTROL => "left_control",
        VK_SHIFT => "left_shift",
        VK_MENU => "left_alt",
        VK_OEM_102 => "non_us_backslash",
        _ => return None,
    };
    Some(KeyName::new(name))
}

pub fn name_to_winkey(name: &KeyName) -> Option<WinKey> {
    let (vk, extended) = match name.as_str() {
        "a" => (0x41, false),
        "b" => (0x42, false),
        "c" => (0x43, false),
        "d" => (0x44, false),
        "e" => (0x45, false),
        "f" => (0x46, false),
        "g" => (0x47, false),
        "h" => (0x48, false),
        "i" => (0x49, false),
        "j" => (0x4A, false),
        "k" => (0x4B, false),
        "l" => (0x4C, false),
        "m" => (0x4D, false),
        "n" => (0x4E, false),
        "o" => (0x4F, false),
        "p" => (0x50, false),
        "q" => (0x51, false),
        "r" => (0x52, false),
        "s" => (0x53, false),
        "t" => (0x54, false),
        "u" => (0x55, false),
        "v" => (0x56, false),
        "w" => (0x57, false),
        "x" => (0x58, false),
        "y" => (0x59, false),
        "z" => (0x5A, false),
        "0" => (0x30, false),
        "1" => (0x31, false),
        "2" => (0x32, false),
        "3" => (0x33, false),
        "4" => (0x34, false),
        "5" => (0x35, false),
        "6" => (0x36, false),
        "7" => (0x37, false),
        "8" => (0x38, false),
        "9" => (0x39, false),
        "enter" | "return" => (VK_RETURN, false),
        "escape" => (VK_ESCAPE, false),
        "delete" | "backspace" => (VK_BACK, false),
        "tab" => (VK_TAB, false),
        "space" => (VK_SPACE, false),
        "minus" | "-" => (VK_OEM_MINUS, false),
        "equal" | "=" => (VK_OEM_PLUS, false),
        "left_bracket" | "[" => (VK_OEM_4, false),
        "right_bracket" | "]" => (VK_OEM_6, false),
        "backslash" | "\\" => (VK_OEM_5, false),
        "semicolon" | ";" => (VK_OEM_1, false),
        "quote" | "'" => (VK_OEM_7, false),
        "grave" | "`" => (VK_OEM_3, false),
        "comma" | "," => (VK_OEM_COMMA, false),
        "period" | "." => (VK_OEM_PERIOD, false),
        "slash" | "/" => (VK_OEM_2, false),
        "caps_lock" | "caps" => (VK_CAPITAL, false),
        "f1" => (0x70, false),
        "f2" => (0x71, false),
        "f3" => (0x72, false),
        "f4" => (0x73, false),
        "f5" => (0x74, false),
        "f6" => (0x75, false),
        "f7" => (0x76, false),
        "f8" => (0x77, false),
        "f9" => (0x78, false),
        "f10" => (0x79, false),
        "f11" => (0x7A, false),
        "f12" => (0x7B, false),
        "insert" => (VK_INSERT, true),
        "home" => (VK_HOME, true),
        "page_up" => (VK_PRIOR, true),
        "forward_delete" => (VK_DELETE, true),
        "end" => (VK_END, true),
        "page_down" => (VK_NEXT, true),
        "right" => (VK_RIGHT, true),
        "left" => (VK_LEFT, true),
        "down" => (VK_DOWN, true),
        "up" => (VK_UP, true),
        "left_control" => (VK_LCONTROL, false),
        "right_control" => (VK_RCONTROL, true),
        "left_shift" => (VK_LSHIFT, false),
        "right_shift" => (VK_RSHIFT, false),
        "left_alt" => (VK_LMENU, false),
        "right_alt" => (VK_RMENU, true),
        "left_meta" => (VK_LWIN, true),
        "right_meta" => (VK_RWIN, true),
        "non_us_backslash" | "nubs" => (VK_OEM_102, false),
        _ => return None,
    };
    Some(WinKey { vk, extended })
}

pub fn is_function_row_vk(vk: u16) -> bool {
    (VK_F1..=VK_F12).contains(&vk)
}

/// F-row → multimedia VK where Win32 has equivalents.
/// F1–F6 / F3–F4 stay as F-keys (no stable brightness/illum VKs via SendInput).
pub fn media_vk_for_f_row(vk: u16) -> Option<WinKey> {
    let (media_vk, extended) = match vk {
        0x76 => (VK_MEDIA_PREV_TRACK, true), // F7
        0x77 => (VK_MEDIA_PLAY_PAUSE, true), // F8
        0x78 => (VK_MEDIA_NEXT_TRACK, true), // F9
        0x79 => (VK_VOLUME_MUTE, true),      // F10
        0x7A => (VK_VOLUME_DOWN, true),      // F11
        0x7B => (VK_VOLUME_UP, true),        // F12
        _ => return None,
    };
    Some(WinKey {
        vk: media_vk,
        extended,
    })
}
